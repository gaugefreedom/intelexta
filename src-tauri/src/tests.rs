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
fn start_run_creates_new_execution_without_truncating_history() -> Result<()> {
    init_keyring_mock();
    let pool = setup_pool()?;
    let project = api::create_project_with_pool("Execution History".into(), &pool)?;

    let run_id = orchestrator::create_run(
        &pool,
        &project.id,
        "history-test",
        orchestrator::RunProofMode::Exact,
        None,
        7,
        1_000,
        "stub-model",
        vec![orchestrator::RunStepTemplate {
            model: "stub-model".into(),
            prompt: "{\"prompt\":\"hello\"}".into(),
            token_budget: 1_000,
            order_index: Some(0),
            checkpoint_type: "Step".to_string(),
            proof_mode: orchestrator::RunProofMode::Exact,
            epsilon: None,
        }],
    )?;

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
    let first_execution = orchestrator::start_run_with_client(&pool, &run_id, &client)?;

    {
        let conn = pool.get()?;
        let execution_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM run_executions WHERE run_id = ?1",
            params![&run_id],
            |row| row.get(0),
        )?;
        assert_eq!(execution_count, 1);
        let first_checkpoint_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM checkpoints WHERE run_execution_id = ?1",
            params![&first_execution.id],
            |row| row.get(0),
        )?;
        assert!(first_checkpoint_count > 0);
    }

    let second_execution = orchestrator::start_run_with_client(&pool, &run_id, &client)?;
    assert_ne!(first_execution.id, second_execution.id);

    {
        let conn = pool.get()?;
        let execution_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM run_executions WHERE run_id = ?1",
            params![&run_id],
            |row| row.get(0),
        )?;
        assert_eq!(execution_count, 2);

        let total_checkpoints: i64 = conn.query_row(
            "SELECT COUNT(*) FROM checkpoints WHERE run_id = ?1",
            params![&run_id],
            |row| row.get(0),
        )?;
        assert!(total_checkpoints >= 2);

        let first_execution_remaining: i64 = conn.query_row(
            "SELECT COUNT(*) FROM checkpoints WHERE run_execution_id = ?1",
            params![&first_execution.id],
            |row| row.get(0),
        )?;
        let second_execution_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM checkpoints WHERE run_execution_id = ?1",
            params![&second_execution.id],
            |row| row.get(0),
        )?;
        assert!(first_execution_remaining > 0);
        assert!(second_execution_count > 0);
    }

    Ok(())
}

#[test]
fn start_run_with_client_replays_concordant_with_epsilon() -> Result<()> {
    init_keyring_mock();
    let pool = setup_pool()?;
    let project = api::create_project_with_pool("Concordant Start".into(), &pool)?;

    let run_id = orchestrator::create_run(
        &pool,
        &project.id,
        "concordant-start",
        orchestrator::RunProofMode::Concordant,
        Some(0.25),
        99,
        120,
        "stub-model",
        vec![orchestrator::RunStepTemplate {
            model: "stub-model".into(),
            prompt: "{\"value\":42}".into(),
            token_budget: 120,
            order_index: Some(0),
            checkpoint_type: "Step".to_string(),
            proof_mode: orchestrator::RunProofMode::Concordant,
            epsilon: None,
        }],
    )?;

    struct NoopClient;

    impl orchestrator::LlmClient for NoopClient {
        fn stream_generate(
            &self,
            _model: &str,
            _prompt: &str,
        ) -> anyhow::Result<orchestrator::LlmGeneration> {
            Ok(orchestrator::LlmGeneration {
                response: String::new(),
                usage: orchestrator::TokenUsage {
                    prompt_tokens: 0,
                    completion_tokens: 0,
                },
            })
        }
    }

    let client = NoopClient;
    let _ = orchestrator::start_run_with_client(&pool, &run_id, &client)?;

    {
        let conn = pool.get()?;
        let (count, semantic_digest): (i64, Option<String>) = conn.query_row(
            "SELECT COUNT(*), MAX(semantic_digest) FROM checkpoints WHERE run_id = ?1",
            params![&run_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        assert_eq!(count, 1);
        assert!(semantic_digest.is_some());
    }

    let report = replay::replay_concordant_run(run_id.clone(), &pool)?;
    assert!(report.match_status);
    assert_eq!(report.epsilon, Some(0.25));
    assert_eq!(report.semantic_distance, Some(0));
    assert_eq!(report.checkpoint_reports.len(), 1);
    let checkpoint = report
        .checkpoint_reports
        .first()
        .expect("checkpoint report present");
    assert_eq!(
        checkpoint.proof_mode,
        Some(orchestrator::RunProofMode::Concordant)
    );
    assert_eq!(checkpoint.configured_epsilon, None);
    assert_eq!(checkpoint.epsilon, Some(0.25));

    Ok(())
}

#[test]
fn reorder_run_steps_swaps_entries() -> Result<()> {
    init_keyring_mock();
    let pool = setup_pool()?;
    let project = api::create_project_with_pool("Reorder Checkpoints".into(), &pool)?;

    let run_id = orchestrator::create_run(
        &pool,
        &project.id,
        "reorder-checkpoints",
        orchestrator::RunProofMode::Exact,
        None,
        5,
        100,
        "stub-model",
        vec![
            orchestrator::RunStepTemplate {
                model: "stub-model".into(),
                prompt: "{\"prompt\":\"first\"}".into(),
                token_budget: 100,
                order_index: Some(0),
                checkpoint_type: "Step".to_string(),
                proof_mode: orchestrator::RunProofMode::Exact,
                epsilon: None,
            },
            orchestrator::RunStepTemplate {
                model: "stub-model".into(),
                prompt: "{\"prompt\":\"second\"}".into(),
                token_budget: 100,
                order_index: Some(1),
                checkpoint_type: "Step".to_string(),
                proof_mode: orchestrator::RunProofMode::Exact,
                epsilon: None,
            },
        ],
    )?;

    let configs = api::list_run_steps_with_pool(run_id.clone(), &pool)?;
    assert_eq!(configs.len(), 2);
    assert_eq!(configs[0].order_index, 0);
    assert_eq!(configs[1].order_index, 1);

    let reordered = vec![configs[1].id.clone(), configs[0].id.clone()];
    let updated = api::reorder_run_steps_with_pool(run_id.clone(), reordered, &pool)?;
    assert_eq!(updated.len(), 2);
    assert_eq!(updated[0].id, configs[1].id);
    assert_eq!(updated[0].order_index, 0);
    assert_eq!(updated[1].id, configs[0].id);
    assert_eq!(updated[1].order_index, 1);

    let persisted = api::list_run_steps_with_pool(run_id, &pool)?;
    assert_eq!(persisted.len(), 2);
    assert_eq!(persisted[0].id, updated[0].id);
    assert_eq!(persisted[1].id, updated[1].id);

    Ok(())
}

#[cfg(feature = "interactive")]
#[test]
fn interactive_run_emits_process_proof_and_replays() -> Result<()> {
    init_keyring_mock();
    let pool = setup_pool()?;
    let project = api::create_project_with_pool("Interactive Proof".into(), &pool)?;

    let run_id = Uuid::new_v4().to_string();
    let created_at = Utc::now();

    {
        let conn = pool.get()?;
        conn.execute(
            "INSERT INTO runs (id, project_id, name, created_at, sampler_json, seed, epsilon, token_budget, default_model, proof_mode)
             VALUES (?1, ?2, ?3, ?4, NULL, ?5, NULL, ?6, ?7, ?8)",
            params![
                &run_id,
                &project.id,
                "interactive-sequential",
                &created_at.to_rfc3339(),
                123_i64,
                10_000_i64,
                "stub-model",
                orchestrator::RunProofMode::Exact.as_str(),
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
    assert_eq!(car.steps, inserted_ids);

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
