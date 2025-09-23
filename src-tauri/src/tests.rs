// In src-tauri/src/tests.rs
use std::convert::TryInto;
use std::sync::Once;
use uuid::Uuid;

use chrono::{Duration, Utc};

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
use serde::Serialize;

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
            token_budget: 5,
            model: "stub-model".into(),
            checkpoints: vec![orchestrator::RunCheckpointTemplate {
                model: "stub-model".into(),
                prompt: "{}".into(),
                token_budget: 5,
                order_index: Some(0),
                checkpoint_type: "Step".to_string(),
                proof_mode: orchestrator::RunProofMode::Exact,
            }],
            proof_mode: orchestrator::RunProofMode::Exact,
            epsilon: None,
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
            token_budget: 5,
            model: "stub-model".into(),
            checkpoints: vec![orchestrator::RunCheckpointTemplate {
                model: "stub-model".into(),
                prompt: "{}".into(),
                token_budget: 5,
                order_index: Some(0),
                checkpoint_type: "Step".to_string(),
                proof_mode: orchestrator::RunProofMode::Exact,
            }],
            proof_mode: orchestrator::RunProofMode::Exact,
            epsilon: None,
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
    assert!(incident_ckpt.parent_checkpoint_id.is_none());
    assert!(incident_ckpt.turn_index.is_none());
    assert!(incident_ckpt.message.is_none());
    Ok(())
}

#[test]
fn list_checkpoints_includes_message_payload() -> Result<()> {
    init_keyring_mock();
    let pool = setup_pool()?;
    let project = api::create_project_with_pool("Message".into(), &pool)?;

    let run_id = orchestrator::start_hello_run(
        &pool,
        orchestrator::RunSpec {
            project_id: project.id.clone(),
            name: "message-run".into(),
            seed: 3,
            token_budget: 50,
            model: "stub-model".into(),
            checkpoints: vec![orchestrator::RunCheckpointTemplate {
                model: "stub-model".into(),
                prompt: "{}".into(),
                token_budget: 50,
                order_index: Some(0),
                checkpoint_type: "Step".to_string(),
                proof_mode: orchestrator::RunProofMode::Exact,
            }],
            proof_mode: orchestrator::RunProofMode::Exact,
            epsilon: None,
        },
    )?;

    let checkpoint_id: String = {
        let conn = pool.get()?;
        conn.query_row(
            "SELECT id FROM checkpoints WHERE run_id = ?1",
            params![&run_id],
            |row| row.get(0),
        )?
    };

    {
        let conn = pool.get()?;
        conn.execute(
            "UPDATE checkpoints SET turn_index = 0, parent_checkpoint_id = ?2 WHERE id = ?1",
            params![&checkpoint_id, &checkpoint_id],
        )?;
        conn.execute(
            "INSERT INTO checkpoint_messages (checkpoint_id, role, body, created_at, updated_at) VALUES (?1, 'human', 'Hello, agent.', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            params![&checkpoint_id],
        )?;
    }

    let checkpoints = api::list_checkpoints_with_pool(run_id, &pool)?;
    assert_eq!(checkpoints.len(), 1);
    let checkpoint = &checkpoints[0];
    assert_eq!(checkpoint.turn_index, Some(0));
    assert_eq!(
        checkpoint.parent_checkpoint_id.as_deref(),
        Some(checkpoint.id.as_str())
    );
    let message = checkpoint.message.as_ref().expect("stored message");
    assert_eq!(message.role, "human");
    assert_eq!(message.body, "Hello, agent.");
    assert!(!message.created_at.is_empty());
    Ok(())
}

#[test]
fn get_checkpoint_details_includes_payloads() -> Result<()> {
    init_keyring_mock();
    let pool = setup_pool()?;
    let project = api::create_project_with_pool("Details".into(), &pool)?;

    let run_id = orchestrator::start_hello_run(
        &pool,
        orchestrator::RunSpec {
            project_id: project.id.clone(),
            name: "details-run".into(),
            seed: 11,
            token_budget: 100,
            model: "stub-model".into(),
            checkpoints: vec![orchestrator::RunCheckpointTemplate {
                model: "stub-model".into(),
                prompt: "{}".into(),
                token_budget: 100,
                order_index: Some(0),
                checkpoint_type: "Step".to_string(),
                proof_mode: orchestrator::RunProofMode::Exact,
            }],
            proof_mode: orchestrator::RunProofMode::Exact,
            epsilon: None,
        },
    )?;

    let checkpoint_id: String = {
        let conn = pool.get()?;
        conn.query_row(
            "SELECT id FROM checkpoints WHERE run_id = ?1",
            params![&run_id],
            |row| row.get(0),
        )?
    };

    let details = api::get_checkpoint_details_with_pool(checkpoint_id.clone(), &pool)?;

    assert_eq!(details.id, checkpoint_id);
    assert_eq!(details.run_id, run_id);
    assert_eq!(details.kind, "Step");
    assert_eq!(details.prompt_payload.as_deref(), Some("{}"));
    assert!(details.output_payload.as_ref().is_some());
    assert!(details.inputs_sha256.as_ref().is_some());
    assert!(details.outputs_sha256.as_ref().is_some());
    assert_eq!(details.prompt_tokens, 0);
    assert!(details.completion_tokens > 0);

    Ok(())
}

#[test]
fn reopen_run_returns_to_draft_without_execution() -> Result<()> {
    init_keyring_mock();
    let pool = setup_pool()?;
    let project = api::create_project_with_pool("Reopen Draft".into(), &pool)?;

    let spec = orchestrator::RunSpec {
        project_id: project.id.clone(),
        name: "reopen-me".into(),
        seed: 11,
        token_budget: 1_000,
        model: "stub-model".into(),
        checkpoints: vec![orchestrator::RunCheckpointTemplate {
            model: "stub-model".into(),
            prompt: "{\"prompt\":\"hello\"}".into(),
            token_budget: 1_000,
            order_index: Some(0),
            checkpoint_type: "Step".to_string(),
            proof_mode: orchestrator::RunProofMode::Exact,
        }],
        proof_mode: orchestrator::RunProofMode::Exact,
        epsilon: None,
    };

    let run_id = orchestrator::create_run(&pool, spec)?;

    struct FixedClient;

    impl orchestrator::LlmClient for FixedClient {
        fn stream_generate(
            &self,
            _model: &str,
            _prompt: &str,
        ) -> anyhow::Result<orchestrator::LlmGeneration> {
            Ok(orchestrator::LlmGeneration {
                response: "stub-response".to_string(),
                usage: orchestrator::TokenUsage {
                    prompt_tokens: 3,
                    completion_tokens: 5,
                },
            })
        }
    }

    let client = FixedClient;
    orchestrator::start_run_with_client(&pool, &run_id, &client)?;

    {
        let conn = pool.get()?;
        let existing: i64 = conn.query_row(
            "SELECT COUNT(*) FROM checkpoints WHERE run_id = ?1",
            params![&run_id],
            |row| row.get(0),
        )?;
        assert_eq!(existing, 1);
    }

    orchestrator::reopen_run(&pool, &run_id)?;

    {
        let conn = pool.get()?;
        let remaining: i64 = conn.query_row(
            "SELECT COUNT(*) FROM checkpoints WHERE run_id = ?1",
            params![&run_id],
            |row| row.get(0),
        )?;
        assert_eq!(remaining, 0);
    }

    orchestrator::start_run_with_client(&pool, &run_id, &client)?;

    {
        let conn = pool.get()?;
        let rerun: i64 = conn.query_row(
            "SELECT COUNT(*) FROM checkpoints WHERE run_id = ?1",
            params![&run_id],
            |row| row.get(0),
        )?;
        assert_eq!(rerun, 1);
    }

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
            token_budget: 50,
            model: "stub-model".into(),
            checkpoints: vec![orchestrator::RunCheckpointTemplate {
                model: "stub-model".into(),
                prompt: "{\"hello\":true}".into(),
                token_budget: 50,
                order_index: Some(0),
                checkpoint_type: "Step".to_string(),
                proof_mode: orchestrator::RunProofMode::Exact,
            }],
            proof_mode: orchestrator::RunProofMode::Exact,
            epsilon: None,
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
        token_budget: 5_000,
        model: "stub-model".into(),
        checkpoints: vec![orchestrator::RunCheckpointTemplate {
            model: "stub-model".into(),
            prompt: "{\"nodes\":[]}".into(),
            token_budget: 5_000,
            order_index: Some(0),
            checkpoint_type: "Step".to_string(),
            proof_mode: orchestrator::RunProofMode::Exact,
        }],
        proof_mode: orchestrator::RunProofMode::Exact,
        epsilon: None,
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

    let expected_version =
        provenance::sha256_hex(&provenance::canonical_json(&run_spec.checkpoints));
    assert_eq!(first_car.run.kind, "exact");
    assert_eq!(first_car.run.seed, run_spec.seed);
    assert_eq!(first_car.run.version, expected_version);
    assert!(first_car.run.model.contains(&run_spec.name));
    assert!(first_car.proof.process.is_none());

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

    let expected_input_sha = provenance::sha256_hex(run_spec.checkpoints[0].prompt.as_bytes());
    assert!(first_car.provenance.iter().any(|claim| {
        claim.claim_type == "input" && claim.sha256 == format!("sha256:{expected_input_sha}")
    }));

    let mut expected_output_input = b"hello".to_vec();
    expected_output_input.extend_from_slice(&run_spec.seed.to_le_bytes());
    expected_output_input.extend_from_slice(&0_i64.to_le_bytes());
    let prompt_hash = provenance::sha256_hex(run_spec.checkpoints[0].prompt.as_bytes());
    expected_output_input.extend_from_slice(prompt_hash.as_bytes());
    let expected_output_sha = provenance::sha256_hex(&expected_output_input);
    assert!(first_car.provenance.iter().any(|claim| {
        claim.claim_type == "output" && claim.sha256 == format!("sha256:{expected_output_sha}")
    }));

    let spec_hash = format!(
        "sha256:{}",
        provenance::sha256_hex(&provenance::canonical_json(&run_spec.checkpoints))
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
            token_budget: 50,
            model: "stub-model".into(),
            checkpoints: vec![orchestrator::RunCheckpointTemplate {
                model: "stub-model".into(),
                prompt: "{}".into(),
                token_budget: 50,
                order_index: Some(0),
                checkpoint_type: "Step".to_string(),
                proof_mode: orchestrator::RunProofMode::Exact,
            }],
            proof_mode: orchestrator::RunProofMode::Exact,
            epsilon: None,
        },
    )?;

    let report = replay::replay_exact_run(run_id.clone(), &pool)?;

    assert!(report.match_status);
    assert!(report.error_message.is_none());
    assert!(!report.original_digest.is_empty());
    assert_eq!(report.original_digest, report.replay_digest);
    assert!(report.semantic_original_digest.is_none());
    assert!(report.semantic_replay_digest.is_none());
    assert!(report.semantic_distance.is_none());
    assert!(report.epsilon.is_none());
    assert_eq!(report.checkpoint_reports.len(), 1);
    let checkpoint = report
        .checkpoint_reports
        .first()
        .expect("checkpoint report present");
    assert_eq!(checkpoint.mode, replay::CheckpointReplayMode::Exact);
    assert!(checkpoint.match_status);
    assert_eq!(checkpoint.original_digest, report.original_digest);
    assert_eq!(checkpoint.replay_digest, report.replay_digest);

    let mut expected_input = b"hello".to_vec();
    expected_input.extend_from_slice(&seed.to_le_bytes());
    expected_input.extend_from_slice(&0_i64.to_le_bytes());
    let prompt_hash = provenance::sha256_hex(b"{}");
    expected_input.extend_from_slice(prompt_hash.as_bytes());
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
            token_budget: 50,
            model: "stub-model".into(),
            checkpoints: vec![orchestrator::RunCheckpointTemplate {
                model: "stub-model".into(),
                prompt: "{}".into(),
                token_budget: 50,
                order_index: Some(0),
                checkpoint_type: "Step".to_string(),
                proof_mode: orchestrator::RunProofMode::Concordant,
            }],
            proof_mode: orchestrator::RunProofMode::Exact,
            epsilon: None,
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
    assert!(report.semantic_original_digest.is_none());
    assert!(report.semantic_replay_digest.is_none());
    assert!(report.semantic_distance.is_none());
    assert!(report.epsilon.is_none());
    assert_eq!(report.checkpoint_reports.len(), 1);
    let checkpoint = report
        .checkpoint_reports
        .first()
        .expect("checkpoint report present");
    assert_eq!(checkpoint.mode, replay::CheckpointReplayMode::Exact);
    assert!(!checkpoint.match_status);
    assert_eq!(checkpoint.original_digest, "bad-digest");
    assert_eq!(
        checkpoint.error_message.as_deref(),
        Some("outputs digest mismatch")
    );

    Ok(())
}

#[test]
fn replay_concordant_run_successfully_matches_semantics() -> Result<()> {
    init_keyring_mock();
    let pool = setup_pool()?;
    let project = api::create_project_with_pool("Replay Concordant".into(), &pool)?;
    let epsilon = 5.0;

    let run_id = orchestrator::start_hello_run(
        &pool,
        orchestrator::RunSpec {
            project_id: project.id.clone(),
            name: "replay-concordant".into(),
            seed: 11,
            token_budget: 50,
            model: "stub-model".into(),
            checkpoints: vec![orchestrator::RunCheckpointTemplate {
                model: "stub-model".into(),
                prompt: "{}".into(),
                token_budget: 50,
                order_index: Some(0),
                checkpoint_type: "Step".to_string(),
                proof_mode: orchestrator::RunProofMode::Concordant,
            }],
            proof_mode: orchestrator::RunProofMode::Concordant,
            epsilon: Some(epsilon),
        },
    )?;

    let report = replay::replay_concordant_run(run_id.clone(), &pool)?;
    assert!(report.match_status);
    assert!(report.error_message.is_none());
    assert_eq!(report.epsilon, Some(epsilon));
    assert_eq!(report.semantic_distance, Some(0));
    assert_eq!(
        report.semantic_original_digest,
        report.semantic_replay_digest
    );
    assert_eq!(report.checkpoint_reports.len(), 1);
    let checkpoint = report
        .checkpoint_reports
        .first()
        .expect("checkpoint report present");
    assert_eq!(checkpoint.mode, replay::CheckpointReplayMode::Concordant);
    assert!(checkpoint.match_status);
    assert_eq!(checkpoint.epsilon, Some(epsilon));

    let api_report = api::replay_run_with_pool(run_id.clone(), &pool)?;
    assert_eq!(api_report, report);

    Ok(())
}

#[test]
fn replay_concordant_run_detects_semantic_mismatch() -> Result<()> {
    init_keyring_mock();
    let pool = setup_pool()?;
    let project = api::create_project_with_pool("Replay Concordant Fail".into(), &pool)?;
    let epsilon = 1.0;

    let run_id = orchestrator::start_hello_run(
        &pool,
        orchestrator::RunSpec {
            project_id: project.id.clone(),
            name: "replay-concordant-fail".into(),
            seed: 12,
            token_budget: 50,
            model: "stub-model".into(),
            checkpoints: vec![orchestrator::RunCheckpointTemplate {
                model: "stub-model".into(),
                prompt: "{}".into(),
                token_budget: 50,
                order_index: Some(0),
                checkpoint_type: "Step".to_string(),
            }],
            proof_mode: orchestrator::RunProofMode::Concordant,
            epsilon: Some(epsilon),
        },
    )?;

    {
        let conn = pool.get()?;
        conn.execute(
            "UPDATE checkpoints SET semantic_digest = ?1 WHERE run_id = ?2",
            params!["ffffffffffffffff", &run_id],
        )?;
    }

    let report = replay::replay_concordant_run(run_id.clone(), &pool)?;
    assert!(!report.match_status);
    assert!(report
        .error_message
        .as_deref()
        .unwrap_or_default()
        .contains("semantic distance"));
    assert_eq!(report.epsilon, Some(epsilon));
    let distance = report.semantic_distance.expect("distance recorded");
    assert!(f64::from(distance) > epsilon);
    assert_eq!(report.checkpoint_reports.len(), 1);
    let checkpoint = report
        .checkpoint_reports
        .first()
        .expect("checkpoint report present");
    assert_eq!(checkpoint.mode, replay::CheckpointReplayMode::Concordant);
    assert!(!checkpoint.match_status);
    assert_eq!(checkpoint.epsilon, Some(epsilon));

    let api_report = api::replay_run_with_pool(run_id.clone(), &pool)?;
    assert_eq!(api_report, report);

    Ok(())
}

#[test]
fn replay_mixed_modes_reports_per_checkpoint() -> Result<()> {
    init_keyring_mock();
    let pool = setup_pool()?;
    let project = api::create_project_with_pool("Replay Mixed".into(), &pool)?;
    let epsilon = 2.5;

    let run_id = orchestrator::start_hello_run(
        &pool,
        orchestrator::RunSpec {
            project_id: project.id.clone(),
            name: "replay-mixed".into(),
            seed: 42,
            token_budget: 100,
            model: "stub-model".into(),
            checkpoints: vec![
                orchestrator::RunCheckpointTemplate {
                    model: "stub-model".into(),
                    prompt: "{}".into(),
                    token_budget: 50,
                    order_index: Some(0),
                    checkpoint_type: "Step".to_string(),
                    proof_mode: orchestrator::RunProofMode::Exact,
                },
                orchestrator::RunCheckpointTemplate {
                    model: "stub-model".into(),
                    prompt: "{\"value\":1}".into(),
                    token_budget: 50,
                    order_index: Some(1),
                    checkpoint_type: "Step".to_string(),
                    proof_mode: orchestrator::RunProofMode::Concordant,
                },
            ],
            proof_mode: orchestrator::RunProofMode::Concordant,
            epsilon: Some(epsilon),
        },
    )?;

    let report = api::replay_run_with_pool(run_id.clone(), &pool)?;
    assert!(report.match_status);
    assert_eq!(report.checkpoint_reports.len(), 2);

    let exact_entry = &report.checkpoint_reports[0];
    assert_eq!(exact_entry.mode, replay::CheckpointReplayMode::Exact);
    assert!(exact_entry.match_status);

    let concordant_entry = &report.checkpoint_reports[1];
    assert_eq!(
        concordant_entry.mode,
        replay::CheckpointReplayMode::Concordant
    );
    assert!(concordant_entry.match_status);
    assert_eq!(concordant_entry.epsilon, Some(epsilon));
    assert!(concordant_entry.semantic_distance.is_some());

    // Ensure the overall report still exposes semantic metrics from the concordant checkpoint.
    assert_eq!(report.epsilon, Some(epsilon));
    assert_eq!(report.semantic_distance, concordant_entry.semantic_distance);

    Ok(())
}

#[cfg(feature = "interactive")]
#[test]
fn interactive_run_emits_process_proof_and_replays() -> Result<()> {
    init_keyring_mock();
    let pool = setup_pool()?;
    let project = api::create_project_with_pool("Interactive Proof".into(), &pool)?;

    let run_spec = orchestrator::RunSpec {
        project_id: project.id.clone(),
        name: "interactive-sequential".into(),
        seed: 123,
        token_budget: 10_000,
        model: "stub-model".into(),
        checkpoints: Vec::new(),
        proof_mode: orchestrator::RunProofMode::Exact,
        epsilon: None,
    };

    let run_id = Uuid::new_v4().to_string();
    let created_at = Utc::now();
    let spec_json = serde_json::to_string(&run_spec)?;

    {
        let conn = pool.get()?;
        conn.execute(
            "INSERT INTO runs (id, project_id, name, created_at, kind, spec_json, sampler_json, seed, epsilon, token_budget, default_model) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8, ?9, ?10)",
            params![
                &run_id,
                &run_spec.project_id,
                &run_spec.name,
                &created_at.to_rfc3339(),
                run_spec.proof_mode.as_str(),
                &spec_json,
                (run_spec.seed as i64),
                run_spec.epsilon,
                (run_spec.token_budget as i64),
                &run_spec.model,
            ],
        )?;
    }

    #[derive(Serialize)]
    struct TestCheckpointBody<'a> {
        run_id: &'a str,
        kind: &'a str,
        timestamp: String,
        inputs_sha256: Option<&'a str>,
        outputs_sha256: Option<&'a str>,
        incident: Option<&'a serde_json::Value>,
        usage_tokens: u64,
        prompt_tokens: u64,
        completion_tokens: u64,
    }

    let signing_key = provenance::load_secret_key(&project.id)?;
    let mut prev_chain = String::new();
    let mut parent_id: Option<String> = None;
    let mut inserted_ids = Vec::new();
    let mut expected_metadata = Vec::new();
    let mut final_curr_chain = String::new();

    for turn in 0..3_u32 {
        let timestamp = (created_at + Duration::seconds(i64::from(turn))).to_rfc3339();
        let checkpoint_id = Uuid::new_v4().to_string();
        let inputs_sha = provenance::sha256_hex(format!("interactive-input-{turn}").as_bytes());
        let outputs_sha = provenance::sha256_hex(format!("interactive-output-{turn}").as_bytes());
        let usage_tokens = 7 + u64::from(turn);
        let prompt_tokens = u64::from(turn);
        let completion_tokens = 3;

        let body = TestCheckpointBody {
            run_id: &run_id,
            kind: "Step",
            timestamp: timestamp.clone(),
            inputs_sha256: Some(inputs_sha.as_str()),
            outputs_sha256: Some(outputs_sha.as_str()),
            incident: None,
            usage_tokens,
            prompt_tokens,
            completion_tokens,
        };

        let canonical = provenance::canonical_json(&body);
        let curr_chain = provenance::sha256_hex(&[prev_chain.as_bytes(), &canonical].concat());
        let signature = provenance::sign_bytes(&signing_key, curr_chain.as_bytes());

        {
            let conn = pool.get()?;
            conn.execute(
                "INSERT INTO checkpoints (id, run_id, parent_checkpoint_id, turn_index, kind, incident_json, timestamp, inputs_sha256, outputs_sha256, prev_chain, curr_chain, signature, usage_tokens, semantic_digest, prompt_tokens, completion_tokens) VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7, ?8, ?9, ?10, ?11, ?12, NULL, ?13, ?14)",
                params![
                    &checkpoint_id,
                    &run_id,
                    parent_id.as_deref(),
                    i64::from(turn),
                    "Step",
                    &timestamp,
                    Some(inputs_sha.as_str()),
                    Some(outputs_sha.as_str()),
                    &prev_chain,
                    &curr_chain,
                    &signature,
                    (usage_tokens as i64),
                    (prompt_tokens as i64),
                    (completion_tokens as i64),
                ],
            )?;
        }

        expected_metadata.push((
            checkpoint_id.clone(),
            parent_id.clone(),
            turn,
            prev_chain.clone(),
            curr_chain.clone(),
            signature.clone(),
        ));
        inserted_ids.push(checkpoint_id.clone());
        parent_id = Some(checkpoint_id);
        prev_chain = curr_chain.clone();
        final_curr_chain = curr_chain;
    }

    let car = {
        let conn = pool.get()?;
        car::build_car(&conn, &run_id)?
    };

    assert_eq!(car.proof.match_kind, "process");
    let process = car.proof.process.as_ref().expect("process proof metadata");
    assert_eq!(
        process.sequential_checkpoints.len(),
        expected_metadata.len()
    );
    for (entry, expected) in process
        .sequential_checkpoints
        .iter()
        .zip(expected_metadata.iter())
    {
        assert_eq!(entry.id, expected.0);
        assert_eq!(entry.parent_checkpoint_id, expected.1.clone());
        assert_eq!(entry.turn_index, Some(expected.2));
        assert_eq!(entry.prev_chain, expected.3);
        assert_eq!(entry.curr_chain, expected.4);
        assert_eq!(entry.signature, expected.5);
    }
    assert_eq!(car.checkpoints, inserted_ids);

    let base_dir = std::env::temp_dir().join(format!("intelexta-process-tests-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&base_dir)?;
    let emitted_path = api::emit_car_to_base_dir(&run_id, &pool, &base_dir)?;
    assert!(emitted_path.exists());
    let persisted: car::Car = serde_json::from_str(&std::fs::read_to_string(&emitted_path)?)?;
    assert_eq!(persisted.proof.match_kind, "process");

    let report = replay::replay_interactive_run(run_id.clone(), &pool)?;
    assert!(report.match_status);
    assert!(report.error_message.is_none());
    assert_eq!(report.original_digest, final_curr_chain);
    assert_eq!(report.replay_digest, final_curr_chain);

    let api_report = api::replay_run_with_pool(run_id.clone(), &pool)?;
    assert_eq!(api_report, report);

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
            token_budget: 100,
            model: "stub-model".into(),
            checkpoints: vec![orchestrator::RunCheckpointTemplate {
                model: "stub-model".into(),
                prompt: "{}".into(),
                token_budget: 100,
                order_index: Some(0),
                checkpoint_type: "Step".to_string(),
                proof_mode: orchestrator::RunProofMode::Exact,
            }],
            proof_mode: orchestrator::RunProofMode::Exact,
            epsilon: None,
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
