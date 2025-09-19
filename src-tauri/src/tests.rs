// In src-tauri/src/tests.rs
use std::convert::TryInto;
use std::sync::Once;
use uuid::Uuid;

use crate::{
    api, car, keychain, orchestrator, provenance, replay,
    store::{
        self,
        policies::{self, Policy},
    },
    DbPool,
};
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;

fn setup_pool() -> Result<DbPool> {
    let manager = SqliteConnectionManager::memory();
    let pool = r2d2::Pool::builder().max_size(1).build(manager)?;
    {
        let mut conn = pool.get()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        store::migrate_db(&mut conn)?;
        let latest_version = store::migrations::latest_version();
        let recorded: Option<i64> =
            conn.query_row("SELECT MAX(version) FROM migrations", [], |row| row.get(0))?;
        assert_eq!(recorded.unwrap_or_default(), latest_version);
    }
    Ok(pool)
}

fn init_keyring_mock() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let base_dir =
            std::env::temp_dir().join(format!("intelexta-keychain-tests-{}", std::process::id()));
        std::fs::create_dir_all(&base_dir).expect("create keychain test dir");
        std::env::set_var("INTELEXTA_KEYCHAIN_DIR", &base_dir);
    });
    keychain::force_fallback_for_tests();
}

#[test]
fn create_project_stores_secret_for_later_use() -> Result<()> {
    init_keyring_mock();
    let pool = setup_pool()?;
    let project = api::create_project_with_pool("Test Project".into(), &pool)?;

    let sk = provenance::load_secret_key(&project.id)?;
    let derived_pub = provenance::public_key_from_secret(&sk);
    assert_eq!(derived_pub, project.pubkey);
    Ok(())
}

#[test]
fn orchestrator_writes_incident_checkpoint_when_budget_fails() -> Result<()> {
    init_keyring_mock();
    let pool = setup_pool()?;
    let project = api::create_project_with_pool("Budget".into(), &pool)?;

    let run_id = orchestrator::start_hello_run(
        &pool,
        orchestrator::RunSpec {
            project_id: project.id.clone(),
            name: "low-budget".into(),
            seed: 1,
            dag_json: "{}".into(),
            token_budget: 5,
        },
    )?;

    let conn = pool.get()?;
    let (kind, incident_json): (String, Option<String>) = conn.query_row(
        "SELECT kind, incident_json FROM checkpoints WHERE run_id = ?1",
        params![run_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    assert_eq!(kind, "Incident");
    let incident_json = incident_json.expect("incident details");
    let incident: serde_json::Value = serde_json::from_str(&incident_json)?;
    assert_eq!(incident["kind"], "budget_exceeded");
    assert_eq!(incident["severity"], "error");
    Ok(())
}

#[test]
fn list_checkpoints_includes_incident_payload() -> Result<()> {
    init_keyring_mock();
    let pool = setup_pool()?;
    let project = api::create_project_with_pool("API Budget".into(), &pool)?;

    let run_id = orchestrator::start_hello_run(
        &pool,
        orchestrator::RunSpec {
            project_id: project.id.clone(),
            name: "budget-api".into(),
            seed: 7,
            dag_json: "{}".into(),
            token_budget: 5,
        },
    )?;

    let checkpoints = api::list_checkpoints_with_pool(run_id, &pool)?;
    assert_eq!(checkpoints.len(), 1);

    let incident_ckpt = &checkpoints[0];
    assert_eq!(incident_ckpt.kind, "Incident");
    let incident = incident_ckpt.incident.as_ref().expect("incident payload");
    assert_eq!(incident.kind, "budget_exceeded");
    assert_eq!(incident.severity, "error");
    assert_eq!(incident.details, "usage=10 > budget=5");
    Ok(())
}

#[test]
fn orchestrator_emits_signed_step_checkpoint_on_success() -> Result<()> {
    init_keyring_mock();
    let pool = setup_pool()?;
    let project = api::create_project_with_pool("Happy".into(), &pool)?;
    let seed = 42_u64;

    let run_id = orchestrator::start_hello_run(
        &pool,
        orchestrator::RunSpec {
            project_id: project.id.clone(),
            name: "happy-path".into(),
            seed,
            dag_json: "{\"hello\":true}".into(),
            token_budget: 50,
        },
    )?;

    let conn = pool.get()?;
    let (
        kind,
        incident_json,
        signature_b64,
        curr_chain,
        prev_chain,
        timestamp,
        inputs_sha,
        outputs_sha,
        semantic_digest,
        usage_tokens,
    ): (
        String,
        Option<String>,
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        i64,
    ) =
        conn.query_row(
            "SELECT kind, incident_json, signature, curr_chain, prev_chain, timestamp, inputs_sha256, outputs_sha256, semantic_digest, usage_tokens FROM checkpoints WHERE run_id = ?1",
            params![run_id.clone()],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                ))
            },
        )?;

    assert_eq!(kind, "Step");
    assert!(incident_json.is_none());
    assert!(semantic_digest.is_none());
    assert_eq!(prev_chain, "");
    assert_eq!(usage_tokens, 10);

    let inputs_sha = inputs_sha.expect("inputs sha");
    let outputs_sha = outputs_sha.expect("outputs sha");
    assert_eq!(inputs_sha, provenance::sha256_hex(b"hello"));
    let mut expected_output_input = b"hello".to_vec();
    expected_output_input.extend_from_slice(&seed.to_le_bytes());
    assert_eq!(outputs_sha, provenance::sha256_hex(&expected_output_input));

    let checkpoint_json = serde_json::json!({
        "run_id": run_id,
        "kind": "Step",
        "timestamp": timestamp,
        "inputs_sha256": inputs_sha,
        "outputs_sha256": outputs_sha,
        "incident": serde_json::Value::Null,
        "usage_tokens": usage_tokens as u64,
    });
    let canon = provenance::canonical_json(&checkpoint_json);
    let expected_curr_chain = provenance::sha256_hex(&canon);
    assert_eq!(expected_curr_chain, curr_chain);

    let sig_bytes = STANDARD.decode(signature_b64)?;
    let sig_array: [u8; ed25519_dalek::SIGNATURE_LENGTH] = sig_bytes
        .try_into()
        .map_err(|_| anyhow!("signature has wrong length"))?;
    let signature = Signature::from_bytes(&sig_array);

    let signing_key = provenance::load_secret_key(&project.id)?;
    let verifying_key = signing_key.verifying_key();
    verifying_key.verify(curr_chain.as_bytes(), &signature)?;
    Ok(())
}

#[test]
fn build_car_is_deterministic_and_signed() -> Result<()> {
    init_keyring_mock();
    let pool = setup_pool()?;
    let project = api::create_project_with_pool("CAR Builder".into(), &pool)?;

    let custom_policy = Policy {
        allow_network: true,
        budget_tokens: 2_048,
        budget_usd: 12.5,
        budget_g_co2e: 3.2,
    };

    {
        let conn = pool.get()?;
        policies::upsert(&conn, &project.id, &custom_policy)?;
    }

    let run_spec = orchestrator::RunSpec {
        project_id: project.id.clone(),
        name: "car-builder-run".into(),
        seed: 31415,
        dag_json: "{\"nodes\":[]}".into(),
        token_budget: 5_000,
    };

    let run_id = orchestrator::start_hello_run(&pool, run_spec.clone())?;

    let first_car = {
        let conn = pool.get()?;
        car::build_car(&conn, &run_id)?
    };

    let second_car = {
        let conn = pool.get()?;
        car::build_car(&conn, &run_id)?
    };

    assert_eq!(first_car.id, second_car.id, "CAR id should be stable");
    assert_eq!(first_car.signatures, second_car.signatures);

    let mut body_value_first = serde_json::to_value(&first_car)?;
    if let serde_json::Value::Object(ref mut obj) = body_value_first {
        obj.remove("id");
        obj.remove("signatures");
    }
    let canonical_first = provenance::canonical_json(&body_value_first);
    let expected_id = format!("car:{}", provenance::sha256_hex(&canonical_first));
    assert_eq!(first_car.id, expected_id);

    let mut body_value_second = serde_json::to_value(&second_car)?;
    if let serde_json::Value::Object(ref mut obj) = body_value_second {
        obj.remove("id");
        obj.remove("signatures");
    }
    let canonical_second = provenance::canonical_json(&body_value_second);
    assert_eq!(canonical_first, canonical_second);

    let expected_version = provenance::sha256_hex(run_spec.dag_json.as_bytes());
    assert_eq!(first_car.run.kind, "exact");
    assert_eq!(first_car.run.seed, run_spec.seed);
    assert_eq!(first_car.run.version, expected_version);
    assert!(first_car.run.model.contains(&run_spec.name));

    let expected_policy_hash = format!(
        "sha256:{}",
        provenance::sha256_hex(&provenance::canonical_json(&custom_policy))
    );
    assert_eq!(first_car.policy_ref.hash, expected_policy_hash);
    assert_eq!(first_car.policy_ref.egress, custom_policy.allow_network);

    let conn = pool.get()?;
    let checkpoint_ids: Vec<String> = {
        let mut stmt =
            conn.prepare("SELECT id FROM checkpoints WHERE run_id = ?1 ORDER BY timestamp ASC")?;
        let rows = stmt.query_map(params![&run_id], |row| row.get(0))?;
        let mut ids = Vec::new();
        for row in rows {
            ids.push(row?);
        }
        ids
    };
    assert_eq!(first_car.checkpoints, checkpoint_ids);

    let total_usage: u64 = conn
        .query_row(
            "SELECT COALESCE(SUM(usage_tokens), 0) FROM checkpoints WHERE run_id = ?1",
            params![&run_id],
            |row| row.get::<_, i64>(0),
        )?
        .max(0) as u64;
    assert_eq!(first_car.budgets.tokens, total_usage);

    let expected_input_sha = provenance::sha256_hex(b"hello");
    assert!(first_car.provenance.iter().any(|claim| {
        claim.claim_type == "input" && claim.sha256 == format!("sha256:{expected_input_sha}")
    }));

    let mut expected_output_input = b"hello".to_vec();
    expected_output_input.extend_from_slice(&run_spec.seed.to_le_bytes());
    let expected_output_sha = provenance::sha256_hex(&expected_output_input);
    assert!(first_car.provenance.iter().any(|claim| {
        claim.claim_type == "output" && claim.sha256 == format!("sha256:{expected_output_sha}")
    }));

    let spec_hash = format!(
        "sha256:{}",
        provenance::sha256_hex(&provenance::canonical_json(&run_spec))
    );
    assert!(first_car
        .provenance
        .iter()
        .any(|claim| { claim.claim_type == "config" && claim.sha256 == spec_hash }));

    assert_eq!(first_car.signatures.len(), 1);
    let signature_entry = &first_car.signatures[0];
    assert!(signature_entry.starts_with("ed25519:"));
    let signature_b64 = &signature_entry[8..];
    let signature_bytes = STANDARD.decode(signature_b64)?;
    let sig_array: [u8; ed25519_dalek::SIGNATURE_LENGTH] = signature_bytes
        .try_into()
        .map_err(|_| anyhow!("signature length mismatch"))?;
    let signature = Signature::from_bytes(&sig_array);

    let pubkey_bytes = STANDARD.decode(&project.pubkey)?;
    let pubkey_array: [u8; ed25519_dalek::PUBLIC_KEY_LENGTH] = pubkey_bytes
        .try_into()
        .map_err(|_| anyhow!("pubkey length mismatch"))?;
    let verifying_key = VerifyingKey::from_bytes(&pubkey_array)?;
    verifying_key.verify(first_car.id.as_bytes(), &signature)?;

    Ok(())
}

#[test]
fn replay_exact_run_successfully_matches_digest() -> Result<()> {
    init_keyring_mock();
    let pool = setup_pool()?;
    let project = api::create_project_with_pool("Replay Happy".into(), &pool)?;
    let seed = 2024_u64;

    let run_id = orchestrator::start_hello_run(
        &pool,
        orchestrator::RunSpec {
            project_id: project.id.clone(),
            name: "replay-happy".into(),
            seed,
            dag_json: "{}".into(),
            token_budget: 50,
        },
    )?;

    let report = replay::replay_exact_run(run_id.clone(), &pool)?;

    assert!(report.match_status);
    assert!(report.error_message.is_none());
    assert!(!report.original_digest.is_empty());
    assert_eq!(report.original_digest, report.replay_digest);

    let mut expected_input = b"hello".to_vec();
    expected_input.extend_from_slice(&seed.to_le_bytes());
    let expected_digest = provenance::sha256_hex(&expected_input);
    assert_eq!(report.original_digest, expected_digest);

    Ok(())
}

#[test]
fn replay_exact_run_reports_mismatched_digest() -> Result<()> {
    init_keyring_mock();
    let pool = setup_pool()?;
    let project = api::create_project_with_pool("Replay Tamper".into(), &pool)?;

    let run_id = orchestrator::start_hello_run(
        &pool,
        orchestrator::RunSpec {
            project_id: project.id.clone(),
            name: "replay-tamper".into(),
            seed: 7,
            dag_json: "{}".into(),
            token_budget: 50,
        },
    )?;

    {
        let conn = pool.get()?;
        conn.execute(
            "UPDATE checkpoints SET outputs_sha256 = ?1 WHERE run_id = ?2",
            params!["bad-digest", &run_id],
        )?;
    }

    let report = replay::replay_exact_run(run_id, &pool)?;

    assert!(!report.match_status);
    assert_eq!(
        report.error_message.as_deref(),
        Some("outputs digest mismatch")
    );
    assert_eq!(report.original_digest, "bad-digest");
    assert_ne!(report.original_digest, report.replay_digest);

    Ok(())
}

#[test]
fn emit_car_command_writes_receipt_and_file() -> Result<()> {
    init_keyring_mock();
    let pool = setup_pool()?;
    let project = api::create_project_with_pool("Emit CAR".into(), &pool)?;

    let run_id = orchestrator::start_hello_run(
        &pool,
        orchestrator::RunSpec {
            project_id: project.id.clone(),
            name: "emit-car-run".into(),
            seed: 7,
            dag_json: "{}".into(),
            token_budget: 100,
        },
    )?;

    let base_dir = std::env::temp_dir().join(format!("intelexta-tests-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&base_dir)?;

    let emitted_path = api::emit_car_to_base_dir(&run_id, &pool, &base_dir)?;
    assert!(emitted_path.exists(), "CAR file should exist on disk");

    let persisted_car: car::Car = serde_json::from_str(&std::fs::read_to_string(&emitted_path)?)?;
    let expected_filename = format!("{}.car.json", persisted_car.id);
    let actual_filename = emitted_path
        .file_name()
        .and_then(|name| name.to_str())
        .expect("CAR filename");
    assert_eq!(actual_filename, expected_filename);

    let conn = pool.get()?;
    let (receipt_id, file_path_db, match_kind, epsilon, s_grade): (
        String,
        String,
        Option<String>,
        Option<f64>,
        i64,
    ) = conn.query_row(
        "SELECT id, file_path, match_kind, epsilon, s_grade FROM receipts WHERE run_id = ?1",
        params![&run_id],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        },
    )?;

    assert_eq!(receipt_id, persisted_car.id);
    assert_eq!(file_path_db, emitted_path.to_string_lossy().to_string());
    assert_eq!(
        match_kind.as_deref(),
        Some(persisted_car.proof.match_kind.as_str())
    );
    assert_eq!(epsilon, persisted_car.proof.epsilon);
    assert_eq!(s_grade, i64::from(persisted_car.sgrade.score));

    Ok(())
}

#[test]
fn get_policy_returns_default_for_new_project() -> Result<()> {
    init_keyring_mock();
    let pool = setup_pool()?;
    let project = api::create_project_with_pool("Policy Defaults".into(), &pool)?;

    let conn = pool.get()?;
    let policy = policies::get(&conn, &project.id)?;

    assert_eq!(policy, Policy::default());
    Ok(())
}

#[test]
fn update_policy_persists_values() -> Result<()> {
    init_keyring_mock();
    let pool = setup_pool()?;
    let project = api::create_project_with_pool("Policy Persist".into(), &pool)?;

    let desired = Policy {
        allow_network: true,
        budget_tokens: 512,
        budget_usd: 4.25,
        budget_g_co2e: 0.75,
    };

    {
        let conn = pool.get()?;
        policies::upsert(&conn, &project.id, &desired)?;
    }

    let conn = pool.get()?;
    let fetched = policies::get(&conn, &project.id)?;
    assert_eq!(fetched, desired);
    Ok(())
}
