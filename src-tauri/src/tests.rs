// In src-tauri/src/tests.rs
use std::convert::TryInto;
use std::sync::Once;
use uuid::Uuid;

use crate::{
    api, keychain, orchestrator, provenance,
    store::{
        self,
        policies::{self, Policy},
    },
    DbPool,
};
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use ed25519_dalek::{Signature, Verifier};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;

fn setup_pool() -> Result<DbPool> {
    let manager = SqliteConnectionManager::memory();
    let pool = r2d2::Pool::builder().max_size(1).build(manager)?;
    {
        let conn = pool.get()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        store::migrate_db(&conn)?;
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
        keychain::force_in_memory_keyring();
    });
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

    let conn = pool.get()?;
    let (receipt_id, file_path_db, match_kind, epsilon): (
        String,
        String,
        Option<String>,
        Option<f64>,
    ) = conn.query_row(
        "SELECT id, file_path, match_kind, epsilon FROM receipts WHERE run_id = ?1",
        params![&run_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;

    assert!(!receipt_id.is_empty());
    assert_eq!(file_path_db, emitted_path.to_string_lossy().to_string());
    assert_eq!(match_kind.as_deref(), Some("pending"));
    assert!(epsilon.is_none());

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
