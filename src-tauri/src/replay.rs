// In src-tauri/src/replay.rs
use crate::{
    orchestrator::{self, RunProofMode},
    provenance, DbPool,
};
#[cfg(feature = "interactive")]
use anyhow::Context;
use anyhow::{anyhow, Result};
#[cfg(feature = "interactive")]
use base64::{engine::general_purpose::STANDARD, Engine as _};
#[cfg(feature = "interactive")]
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
#[cfg(feature = "interactive")]
use serde_json::Value;
#[cfg(feature = "interactive")]
#[cfg(feature = "interactive")]
use std::collections::HashMap;
#[cfg(feature = "interactive")]
use std::convert::TryInto;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CheckpointReplayMode {
    Exact,
    Concordant,
    Interactive,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointReplayReport {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkpoint_config_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkpoint_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_index: Option<i64>,
    pub mode: CheckpointReplayMode,
    pub match_status: bool,
    pub original_digest: String,
    pub replay_digest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof_mode: Option<RunProofMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_original_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_replay_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_distance: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epsilon: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configured_epsilon: Option<f64>,
}

impl CheckpointReplayReport {
    fn new(config: &orchestrator::RunStep, mode: CheckpointReplayMode) -> Self {
        Self {
            checkpoint_config_id: Some(config.id.clone()),
            checkpoint_type: Some(config.checkpoint_type.clone()),
            order_index: Some(config.order_index),
            mode,
            match_status: false,
            original_digest: String::new(),
            replay_digest: String::new(),
            error_message: None,
            proof_mode: Some(config.proof_mode),
            semantic_original_digest: None,
            semantic_replay_digest: None,
            semantic_distance: None,
            epsilon: None,
            configured_epsilon: config.epsilon,
        }
    }

    fn for_interactive_config(config: &orchestrator::RunStep) -> Self {
        Self {
            checkpoint_config_id: Some(config.id.clone()),
            checkpoint_type: Some(config.checkpoint_type.clone()),
            order_index: Some(config.order_index),
            mode: CheckpointReplayMode::Interactive,
            match_status: false,
            original_digest: String::new(),
            replay_digest: String::new(),
            error_message: None,
            proof_mode: Some(config.proof_mode),
            semantic_original_digest: None,
            semantic_replay_digest: None,
            semantic_distance: None,
            epsilon: None,
            configured_epsilon: config.epsilon,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReplayReport {
    pub run_id: String,
    pub match_status: bool,
    pub original_digest: String,
    pub replay_digest: String,
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_original_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_replay_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_distance: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epsilon: Option<f64>,
    #[serde(default)]
    pub checkpoint_reports: Vec<CheckpointReplayReport>,
}

impl ReplayReport {
    pub(crate) fn from_checkpoint_reports(
        run_id: String,
        checkpoint_reports: Vec<CheckpointReplayReport>,
        default_error: Option<String>,
    ) -> Self {
        if checkpoint_reports.is_empty() {
            return ReplayReport {
                run_id,
                match_status: false,
                original_digest: String::new(),
                replay_digest: String::new(),
                error_message: default_error
                    .or_else(|| Some("run has no configured checkpoints".to_string())),
                semantic_original_digest: None,
                semantic_replay_digest: None,
                semantic_distance: None,
                epsilon: None,
                checkpoint_reports,
            };
        }

        let match_status = checkpoint_reports.iter().all(|entry| entry.match_status);
        let error_message = if match_status {
            None
        } else {
            checkpoint_reports
                .iter()
                .find(|entry| !entry.match_status)
                .and_then(|entry| entry.error_message.clone())
                .or(default_error)
        };

        let original_digest = checkpoint_reports
            .last()
            .map(|entry| entry.original_digest.clone())
            .unwrap_or_default();
        let replay_digest = checkpoint_reports
            .last()
            .map(|entry| entry.replay_digest.clone())
            .unwrap_or_default();
        let semantic_original_digest = checkpoint_reports
            .iter()
            .rev()
            .find_map(|entry| entry.semantic_original_digest.clone());
        let semantic_replay_digest = checkpoint_reports
            .iter()
            .rev()
            .find_map(|entry| entry.semantic_replay_digest.clone());
        let semantic_distance = checkpoint_reports
            .iter()
            .rev()
            .find_map(|entry| entry.semantic_distance);
        let epsilon = checkpoint_reports
            .iter()
            .rev()
            .find_map(|entry| entry.epsilon);

        ReplayReport {
            run_id,
            match_status,
            original_digest,
            replay_digest,
            error_message,
            semantic_original_digest,
            semantic_replay_digest,
            semantic_distance,
            epsilon,
            checkpoint_reports,
        }
    }
}

fn simulate_stub_checkpoint(run_seed: u64, config: &orchestrator::RunStep) -> (String, String) {
    let mut output = b"hello".to_vec();
    output.extend_from_slice(&run_seed.to_le_bytes());
    output.extend_from_slice(&config.order_index.to_le_bytes());
    let prompt_hash = provenance::sha256_hex(config.prompt.as_bytes());
    output.extend_from_slice(prompt_hash.as_bytes());
    let outputs_hex = provenance::sha256_hex(&output);
    let semantic_source = hex::encode(&output);
    let semantic_digest = provenance::semantic_digest(&semantic_source);
    (outputs_hex, semantic_digest)
}

fn load_checkpoint_digests(
    conn: &rusqlite::Connection,
    run_id: &str,
    config_id: &str,
) -> Result<Option<(Option<String>, Option<String>)>> {
    let row = conn
        .query_row(
            "SELECT outputs_sha256, semantic_digest FROM checkpoints WHERE run_id = ?1 AND checkpoint_config_id = ?2 AND kind = 'Step' ORDER BY timestamp DESC, id DESC LIMIT 1",
            params![run_id, config_id],
            |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .optional()?;
    Ok(row)
}

pub(crate) fn replay_exact_checkpoint(
    run: &orchestrator::StoredRun,
    conn: &rusqlite::Connection,
    config: &orchestrator::RunStep,
) -> Result<CheckpointReplayReport> {
    let mut report = CheckpointReplayReport::new(config, CheckpointReplayMode::Exact);

    let digests = load_checkpoint_digests(conn, &run.id, &config.id)?;
    let Some((original_digest_opt, _semantic_opt)) = digests else {
        report.error_message = Some("no outputs digest recorded for checkpoint".to_string());
        return Ok(report);
    };

    let original_digest = original_digest_opt.unwrap_or_default();
    if original_digest.is_empty() {
        report.error_message = Some("no outputs digest recorded for checkpoint".to_string());
        return Ok(report);
    }
    report.original_digest = original_digest.clone();

    let replay_digest = if config.model == "stub-model" {
        let (outputs_hex, _) = simulate_stub_checkpoint(run.seed, config);
        outputs_hex
    } else {
        let generation = orchestrator::replay_llm_generation(&config.model, &config.prompt)?;
        provenance::sha256_hex(generation.response.as_bytes())
    };

    report.replay_digest = replay_digest.clone();
    if replay_digest == original_digest {
        report.match_status = true;
    } else {
        report.error_message = Some("outputs digest mismatch".to_string());
    }

    Ok(report)
}

pub(crate) fn replay_concordant_checkpoint(
    run: &orchestrator::StoredRun,
    conn: &rusqlite::Connection,
    config: &orchestrator::RunStep,
) -> Result<CheckpointReplayReport> {
    let epsilon = config
        .epsilon
        .or(run.epsilon)
        .ok_or_else(|| anyhow!("concordant step missing epsilon"))?;

    let mut report = CheckpointReplayReport::new(config, CheckpointReplayMode::Concordant);
    report.epsilon = Some(epsilon);

    let digests = load_checkpoint_digests(conn, &run.id, &config.id)?;
    let Some((original_digest_opt, semantic_digest_opt)) = digests else {
        report.error_message = Some("no outputs digest recorded for checkpoint".to_string());
        return Ok(report);
    };

    let original_digest = original_digest_opt.unwrap_or_default();
    if original_digest.is_empty() {
        report.error_message = Some("no outputs digest recorded for checkpoint".to_string());
        return Ok(report);
    }
    report.original_digest = original_digest.clone();

    let original_semantic = match semantic_digest_opt {
        Some(value) if !value.is_empty() => value,
        _ => {
            report.error_message = Some("no semantic digest recorded for checkpoint".to_string());
            return Ok(report);
        }
    };
    report.semantic_original_digest = Some(original_semantic.clone());

    let (replay_digest, replay_semantic) = if config.model == "stub-model" {
        simulate_stub_checkpoint(run.seed, config)
    } else {
        let generation = orchestrator::replay_llm_generation(&config.model, &config.prompt)?;
        let outputs_hex = provenance::sha256_hex(generation.response.as_bytes());
        let semantic = provenance::semantic_digest(&generation.response);
        (outputs_hex, semantic)
    };

    report.replay_digest = replay_digest.clone();
    report.semantic_replay_digest = Some(replay_semantic.clone());

    if replay_digest != original_digest {
        report.error_message = Some("outputs digest mismatch".to_string());
        return Ok(report);
    }

    let distance = provenance::semantic_distance(&original_semantic, &replay_semantic)
        .ok_or_else(|| anyhow!("invalid semantic digest encoding"))?;
    report.semantic_distance = Some(distance);

    let normalized_distance = distance as f64 / 64.0;
    if normalized_distance <= epsilon {
        report.match_status = true;
    } else {
        report.error_message = Some(format!(
            "semantic distance {:.2} exceeded epsilon {:.2}",
            normalized_distance, epsilon
        ));
    }

    Ok(report)
}

pub fn replay_exact_run(run_id: String, pool: &DbPool) -> Result<ReplayReport> {
    let conn = pool.get()?;
    let stored_run = match orchestrator::load_stored_run(&conn, &run_id) {
        Ok(run) => run,
        Err(_) => {
            return Ok(ReplayReport {
                run_id,
                match_status: false,
                original_digest: String::new(),
                replay_digest: String::new(),
                error_message: Some("run not found".to_string()),
                semantic_original_digest: None,
                semantic_replay_digest: None,
                semantic_distance: None,
                epsilon: None,
                checkpoint_reports: Vec::new(),
            });
        }
    };

    let has_concordant = stored_run
        .steps
        .iter()
        .filter(|cfg| !cfg.is_interactive_chat())
        .any(|cfg| matches!(cfg.proof_mode, RunProofMode::Concordant));

    if has_concordant {
        return Ok(ReplayReport {
            run_id,
            match_status: false,
            original_digest: String::new(),
            replay_digest: String::new(),
            error_message: Some("run includes concordant checkpoints".to_string()),
            semantic_original_digest: None,
            semantic_replay_digest: None,
            semantic_distance: None,
            epsilon: None,
            checkpoint_reports: Vec::new(),
        });
    }

    let mut checkpoint_reports = Vec::new();
    for config in &stored_run.steps {
        if config.is_interactive_chat() {
            continue;
        }
        let report = replay_exact_checkpoint(&stored_run, &conn, config)?;
        checkpoint_reports.push(report);
    }

    Ok(ReplayReport::from_checkpoint_reports(
        run_id,
        checkpoint_reports,
        None,
    ))
}

#[cfg(test)]
#[cfg(feature = "interactive")]
mod tests {
    use super::*;
    use crate::{api, keychain, orchestrator, store};
    use anyhow::Result;
    use r2d2::Pool;
    use r2d2_sqlite::SqliteConnectionManager;
    use rusqlite::params;
    use std::sync::{Mutex, Once};

    struct PanicLlmClient;

    impl orchestrator::LlmClient for PanicLlmClient {
        fn stream_generate(
            &self,
            _model: &str,
            _prompt: &str,
        ) -> anyhow::Result<orchestrator::LlmGeneration> {
            panic!("interactive start should not call LLM");
        }
    }

    struct FixedLlmClient {
        expected_model: String,
        expected_prompt: String,
        response: String,
        usage: orchestrator::TokenUsage,
        calls: Mutex<usize>,
    }

    impl FixedLlmClient {
        fn new(
            expected_model: String,
            expected_prompt: String,
            response: String,
            usage: orchestrator::TokenUsage,
        ) -> Self {
            Self {
                expected_model,
                expected_prompt,
                response,
                usage,
                calls: Mutex::new(0),
            }
        }
    }

    impl orchestrator::LlmClient for FixedLlmClient {
        fn stream_generate(
            &self,
            model: &str,
            prompt: &str,
        ) -> anyhow::Result<orchestrator::LlmGeneration> {
            assert_eq!(model, self.expected_model);
            assert_eq!(prompt, self.expected_prompt);
            let mut calls = self.calls.lock().expect("lock call count");
            *calls += 1;
            Ok(orchestrator::LlmGeneration {
                response: self.response.clone(),
                usage: self.usage,
            })
        }
    }

    fn init_keychain_backend() {
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            let base_dir =
                std::env::temp_dir().join(format!("intelexta-replay-tests-{}", std::process::id()));
            std::fs::create_dir_all(&base_dir).expect("create replay keychain dir");
            std::env::set_var("INTELEXTA_KEYCHAIN_DIR", &base_dir);
        });
        keychain::force_fallback_for_tests();
    }

    #[test]
    fn replay_interactive_run_succeeds_after_first_turn() -> Result<()> {
        init_keychain_backend();

        let manager = SqliteConnectionManager::memory();
        let pool: Pool<SqliteConnectionManager> = Pool::builder().max_size(1).build(manager)?;
        {
            let mut conn = pool.get()?;
            conn.execute_batch("PRAGMA foreign_keys = ON;")?;
            store::migrate_db(&mut conn)?;
        }

        let project = api::create_project_with_pool("Replay Interactive".into(), &pool)?;
        let chat_prompt = "Keep the conversation brief.".to_string();
        let run_model = "stub-model".to_string();
        let spec = orchestrator::RunSpec {
            project_id: project.id.clone(),
            name: "interactive-replay".into(),
            seed: 0,
            token_budget: 10_000,
            model: run_model.clone(),
            steps: vec![orchestrator::RunStepTemplate {
                model: run_model.clone(),
                prompt: chat_prompt.clone(),
                token_budget: 10_000,
                order_index: Some(0),
                checkpoint_type: "InteractiveChat".to_string(),
                proof_mode: RunProofMode::Exact,
                epsilon: None,
            }],
            proof_mode: RunProofMode::Exact,
            epsilon: None,
        };

        let panic_client = PanicLlmClient;
        let run_id = orchestrator::start_hello_run_with_client(&pool, spec.clone(), &panic_client)?;

        let config_id: String = {
            let conn = pool.get()?;
            conn.query_row(
                "SELECT id FROM run_steps WHERE run_id = ?1",
                params![&run_id],
                |row| row.get(0),
            )?
        };

        {
            let conn = pool.get()?;
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM checkpoints WHERE run_id = ?1",
                params![&run_id],
                |row| row.get(0),
            )?;
            assert_eq!(count, 0);
        }

        let prompt_text = "Hello interactive".to_string();
        let response_text = "Hi human".to_string();
        let usage = orchestrator::TokenUsage {
            prompt_tokens: 3,
            completion_tokens: 5,
        };
        let turn_client = FixedLlmClient::new(
            run_model.clone(),
            format!("{}\n\nHuman: {}\nAI:", chat_prompt, prompt_text.trim()),
            response_text.clone(),
            usage,
        );
        let outcome = orchestrator::submit_interactive_checkpoint_turn_with_client(
            &pool,
            &run_id,
            &config_id,
            &prompt_text,
            &turn_client,
        )?;
        assert_eq!(outcome.ai_response, response_text);
        assert_eq!(*turn_client.calls.lock().unwrap(), 1);

        let report = replay_interactive_run(run_id.clone(), &pool)?;
        assert!(report.match_status);
        assert!(report.error_message.is_none());
        assert_eq!(report.original_digest, report.replay_digest);
        assert!(!report.original_digest.is_empty());

        Ok(())
    }
}

pub fn replay_concordant_run(run_id: String, pool: &DbPool) -> Result<ReplayReport> {
    let conn = pool.get()?;

    let stored_run = match orchestrator::load_stored_run(&conn, &run_id) {
        Ok(run) => run,
        Err(_) => {
            return Ok(ReplayReport {
                run_id,
                match_status: false,
                original_digest: String::new(),
                replay_digest: String::new(),
                error_message: Some("run not found".to_string()),
                semantic_original_digest: None,
                semantic_replay_digest: None,
                semantic_distance: None,
                epsilon: None,
                checkpoint_reports: Vec::new(),
            });
        }
    };

    let has_concordant = stored_run
        .steps
        .iter()
        .filter(|cfg| !cfg.is_interactive_chat())
        .any(|cfg| matches!(cfg.proof_mode, RunProofMode::Concordant));

    if !has_concordant {
        return Ok(ReplayReport {
            run_id,
            match_status: false,
            original_digest: String::new(),
            replay_digest: String::new(),
            error_message: Some("run has no concordant checkpoints".to_string()),
            semantic_original_digest: None,
            semantic_replay_digest: None,
            semantic_distance: None,
            epsilon: None,
            checkpoint_reports: Vec::new(),
        });
    }

    let mut checkpoint_reports = Vec::new();
    for config in &stored_run.steps {
        if config.is_interactive_chat() {
            continue;
        }
        let entry = if matches!(config.proof_mode, RunProofMode::Concordant) {
            replay_concordant_checkpoint(&stored_run, &conn, config)?
        } else {
            replay_exact_checkpoint(&stored_run, &conn, config)?
        };
        checkpoint_reports.push(entry);
    }

    Ok(ReplayReport::from_checkpoint_reports(
        run_id,
        checkpoint_reports,
        None,
    ))
}

#[cfg(feature = "interactive")]
#[derive(Serialize)]
struct ReplayCheckpointBody<'a> {
    run_id: &'a str,
    kind: &'a str,
    timestamp: String,
    inputs_sha256: Option<&'a str>,
    outputs_sha256: Option<&'a str>,
    incident: Option<&'a Value>,
    usage_tokens: u64,
    prompt_tokens: u64,
    completion_tokens: u64,
}

#[cfg(feature = "interactive")]
struct InteractiveCheckpointRow {
    id: String,
    checkpoint_config_id: Option<String>,
    parent_checkpoint_id: Option<String>,
    turn_index: Option<u32>,
    kind: String,
    timestamp: String,
    inputs_sha256: Option<String>,
    outputs_sha256: Option<String>,
    incident: Option<Value>,
    prev_chain: String,
    curr_chain: String,
    signature: String,
    usage_tokens: u64,
    prompt_tokens: u64,
    completion_tokens: u64,
}

#[cfg(feature = "interactive")]
#[derive(Default)]
struct ConversationState {
    expected_turn_index: u32,
    previous_checkpoint_id: Option<String>,
    expected_prev_chain: String,
    last_stored_curr: Option<String>,
    last_computed_curr: Option<String>,
}

#[cfg(feature = "interactive")]
pub fn replay_interactive_run(run_id: String, pool: &DbPool) -> Result<ReplayReport> {
    let conn = pool.get()?;

    let project_and_pubkey: Option<(String, String)> = conn
        .query_row(
            "SELECT r.project_id, p.pubkey FROM runs r JOIN projects p ON p.id = r.project_id WHERE r.id = ?1",
            params![&run_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;

    let (_project_id, pubkey_b64) = match project_and_pubkey {
        Some(tuple) => tuple,
        None => {
            return Ok(ReplayReport {
                run_id,
                match_status: false,
                original_digest: String::new(),
                replay_digest: String::new(),
                error_message: Some("run not found".to_string()),
                semantic_original_digest: None,
                semantic_replay_digest: None,
                semantic_distance: None,
                epsilon: None,
                checkpoint_reports: Vec::new(),
            });
        }
    };

    let stored_run = match orchestrator::load_stored_run(&conn, &run_id) {
        Ok(run) => run,
        Err(err) => {
            return Ok(ReplayReport {
                run_id,
                match_status: false,
                original_digest: String::new(),
                replay_digest: String::new(),
                error_message: Some(err.to_string()),
                semantic_original_digest: None,
                semantic_replay_digest: None,
                semantic_distance: None,
                epsilon: None,
                checkpoint_reports: Vec::new(),
            });
        }
    };
    let config_map: HashMap<String, orchestrator::RunStep> = stored_run
        .steps
        .iter()
        .map(|cfg| (cfg.id.clone(), cfg.clone()))
        .collect();

    let pubkey_bytes = STANDARD
        .decode(pubkey_b64.as_bytes())
        .context("invalid project pubkey encoding")?;
    let pubkey_array: [u8; ed25519_dalek::PUBLIC_KEY_LENGTH] = pubkey_bytes
        .try_into()
        .map_err(|_| anyhow!("invalid project pubkey length"))?;
    let verifying_key = VerifyingKey::from_bytes(&pubkey_array)?;

    let mut stmt = conn.prepare(
        "SELECT id, checkpoint_config_id, parent_checkpoint_id, turn_index, kind, timestamp, inputs_sha256, outputs_sha256, incident_json, prev_chain, curr_chain, signature, usage_tokens, prompt_tokens, completion_tokens
         FROM checkpoints WHERE run_id = ?1 AND turn_index IS NOT NULL ORDER BY timestamp ASC, id ASC",
    )?;

    let rows = stmt.query_map(params![&run_id], |row| {
        let incident_json: Option<String> = row.get(8)?;
        let incident = incident_json
            .map(|payload| serde_json::from_str::<Value>(&payload))
            .transpose()
            .map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(
                    8,
                    rusqlite::types::Type::Text,
                    Box::new(err),
                )
            })?;
        let turn_index = row
            .get::<_, Option<i64>>(3)?
            .map(|value| value.max(0) as u32);
        let usage_tokens: i64 = row.get(12)?;
        let prompt_tokens: i64 = row.get(13)?;
        let completion_tokens: i64 = row.get(14)?;
        Ok(InteractiveCheckpointRow {
            id: row.get(0)?,
            checkpoint_config_id: row.get(1)?,
            parent_checkpoint_id: row.get(2)?,
            turn_index,
            kind: row.get(4)?,
            timestamp: row.get(5)?,
            inputs_sha256: row.get(6)?,
            outputs_sha256: row.get(7)?,
            incident,
            prev_chain: row.get(9)?,
            curr_chain: row.get(10)?,
            signature: row.get(11)?,
            usage_tokens: usage_tokens.max(0) as u64,
            prompt_tokens: prompt_tokens.max(0) as u64,
            completion_tokens: completion_tokens.max(0) as u64,
        })
    })?;

    let mut checkpoints = Vec::new();
    for row in rows {
        checkpoints.push(row?);
    }

    if checkpoints.is_empty() {
        return Ok(ReplayReport::from_checkpoint_reports(
            run_id,
            Vec::new(),
            Some("no checkpoints recorded for run".to_string()),
        ));
    }

    let mut conversation_states: HashMap<Option<String>, ConversationState> = HashMap::new();
    let mut failure: Option<String> = None;
    let mut failure_config: Option<Option<String>> = None;

    for ck in &checkpoints {
        let turn_index = match ck.turn_index {
            Some(value) => value,
            None => {
                failure = Some(format!("checkpoint {} missing turn_index", ck.id));
                failure_config = Some(ck.checkpoint_config_id.clone());
                break;
            }
        };

        let config_key = ck.checkpoint_config_id.clone();

        let mut reset_prev_chain: Option<String> = None;
        if turn_index == 0 {
            let expected_prev_chain = if let Some(parent_id) = ck.parent_checkpoint_id.as_ref() {
                match conn
                    .query_row(
                        "SELECT curr_chain FROM checkpoints WHERE id = ?1",
                        params![parent_id],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()?
                {
                    Some(chain) => chain,
                    None => {
                        failure = Some(format!(
                            "checkpoint {} references missing parent {}",
                            ck.id, parent_id
                        ));
                        break;
                    }
                }
            } else {
                String::new()
            };
            reset_prev_chain = Some(expected_prev_chain);
        }

        let state = conversation_states
            .entry(config_key.clone())
            .or_insert_with(ConversationState::default);

        if let Some(expected_prev_chain) = reset_prev_chain {
            state.expected_turn_index = 0;
            state.previous_checkpoint_id = None;
            state.expected_prev_chain = expected_prev_chain;
        }

        if turn_index != state.expected_turn_index {
            failure = Some(format!(
                "checkpoint {} turn_index {} out of sequence for config {:?} (expected {})",
                ck.id,
                turn_index,
                ck.checkpoint_config_id.as_deref(),
                state.expected_turn_index
            ));
            failure_config = Some(config_key.clone());
            break;
        }

        if state.expected_turn_index > 0
            && ck.parent_checkpoint_id.as_deref() != state.previous_checkpoint_id.as_deref()
        {
            failure = Some(format!(
                "checkpoint {} parent mismatch (expected {:?}, found {:?})",
                ck.id,
                state.previous_checkpoint_id.as_deref(),
                ck.parent_checkpoint_id.as_deref()
            ));
            failure_config = Some(config_key.clone());
            break;
        }

        if ck.prev_chain != state.expected_prev_chain {
            failure = Some(format!("checkpoint {} prev_chain mismatch", ck.id));
            failure_config = Some(config_key.clone());
            break;
        }

        let body = ReplayCheckpointBody {
            run_id: &run_id,
            kind: ck.kind.as_str(),
            timestamp: ck.timestamp.clone(),
            inputs_sha256: ck.inputs_sha256.as_deref(),
            outputs_sha256: ck.outputs_sha256.as_deref(),
            incident: ck.incident.as_ref(),
            usage_tokens: ck.usage_tokens,
            prompt_tokens: ck.prompt_tokens,
            completion_tokens: ck.completion_tokens,
        };

        let canonical = provenance::canonical_json(&body);
        let computed_curr =
            provenance::sha256_hex(&[ck.prev_chain.as_bytes(), &canonical].concat());

        if computed_curr != ck.curr_chain {
            failure = Some(format!("checkpoint {} curr_chain mismatch", ck.id));
            failure_config = Some(config_key.clone());
            break;
        }

        let signature_bytes = match STANDARD.decode(ck.signature.as_bytes()) {
            Ok(bytes) => bytes,
            Err(_) => {
                failure = Some(format!("checkpoint {} signature decoding failed", ck.id));
                failure_config = Some(config_key.clone());
                break;
            }
        };

        let signature_array: [u8; ed25519_dalek::SIGNATURE_LENGTH] =
            match signature_bytes.try_into() {
                Ok(arr) => arr,
                Err(_) => {
                    failure = Some(format!("checkpoint {} signature length invalid", ck.id));
                    failure_config = Some(config_key.clone());
                    break;
                }
            };

        let signature = Signature::from_bytes(&signature_array);
        if verifying_key
            .verify(ck.curr_chain.as_bytes(), &signature)
            .is_err()
        {
            failure = Some(format!(
                "checkpoint {} signature verification failed",
                ck.id
            ));
            failure_config = Some(config_key.clone());
            break;
        }

        state.previous_checkpoint_id = Some(ck.id.clone());
        state.expected_prev_chain = ck.curr_chain.clone();
        state.expected_turn_index += 1;
        state.last_stored_curr = Some(ck.curr_chain.clone());
        state.last_computed_curr = Some(computed_curr);
    }

    let mut checkpoint_reports: Vec<CheckpointReplayReport> = Vec::new();
    for (config_key, state) in conversation_states.into_iter() {
        let mut entry = if let Some(config_id) = config_key.as_ref() {
            if let Some(config) = config_map.get(config_id) {
                CheckpointReplayReport::for_interactive_config(config)
            } else {
                CheckpointReplayReport {
                    checkpoint_config_id: Some(config_id.clone()),
                    checkpoint_type: None,
                    order_index: None,
                    mode: CheckpointReplayMode::Interactive,
                    match_status: false,
                    original_digest: String::new(),
                    replay_digest: String::new(),
                    error_message: None,
                    proof_mode: None,
                    semantic_original_digest: None,
                    semantic_replay_digest: None,
                    semantic_distance: None,
                    epsilon: None,
                    configured_epsilon: None,
                }
            }
        } else {
            CheckpointReplayReport {
                checkpoint_config_id: None,
                checkpoint_type: None,
                order_index: None,
                mode: CheckpointReplayMode::Interactive,
                match_status: false,
                original_digest: String::new(),
                replay_digest: String::new(),
                error_message: None,
                proof_mode: None,
                semantic_original_digest: None,
                semantic_replay_digest: None,
                semantic_distance: None,
                epsilon: None,
                configured_epsilon: None,
            }
        };

        if let Some(value) = state.last_stored_curr {
            entry.original_digest = value;
        }
        if let Some(value) = state.last_computed_curr {
            entry.replay_digest = value;
        }

        if let Some(reason) = failure.as_ref() {
            let failure_matches = failure_config.as_ref().map_or(true, |fc| fc == &config_key);
            if failure_matches {
                entry.match_status = false;
                entry.error_message = Some(reason.clone());
            } else {
                entry.match_status = true;
            }
        } else if entry.original_digest.is_empty() || entry.replay_digest.is_empty() {
            entry.match_status = false;
            entry.error_message = Some("no interactive digest recorded".to_string());
        } else if entry.original_digest == entry.replay_digest {
            entry.match_status = true;
        } else {
            entry.match_status = false;
            entry.error_message = Some("interactive digest mismatch".to_string());
        }

        checkpoint_reports.push(entry);
    }

    checkpoint_reports.sort_by(|a, b| {
        let left = a.order_index.unwrap_or(i64::MAX);
        let right = b.order_index.unwrap_or(i64::MAX);
        left.cmp(&right)
            .then_with(|| a.checkpoint_config_id.cmp(&b.checkpoint_config_id))
    });

    Ok(ReplayReport::from_checkpoint_reports(
        run_id,
        checkpoint_reports,
        failure,
    ))
}
