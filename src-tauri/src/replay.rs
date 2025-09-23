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
use std::collections::HashMap;
#[cfg(feature = "interactive")]
use std::convert::TryInto;

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
}

fn simulate_stub_checkpoint(
    run_seed: u64,
    config: &orchestrator::RunCheckpointConfig,
) -> (String, String) {
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
            });
        }
    };

    let has_concordant = stored_run
        .checkpoints
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
        });
    }

    if stored_run.checkpoints.is_empty() {
        return Ok(ReplayReport {
            run_id,
            match_status: false,
            original_digest: String::new(),
            replay_digest: String::new(),
            error_message: Some("run has no configured checkpoints".to_string()),
            semantic_original_digest: None,
            semantic_replay_digest: None,
            semantic_distance: None,
            epsilon: None,
        });
    }

    let mut replay_digest = String::new();
    for config in &stored_run.checkpoints {
        if config.is_interactive_chat() {
            continue;
        }
        if config.model == "stub-model" {
            let (outputs_hex, _) = simulate_stub_checkpoint(stored_run.seed, config);
            replay_digest = outputs_hex;
        } else {
            let generation = orchestrator::replay_llm_generation(&config.model, &config.prompt)?;
            replay_digest = provenance::sha256_hex(generation.response.as_bytes());
        }
    }

    let final_digest: Option<String> = conn
        .query_row(
            "SELECT outputs_sha256 FROM checkpoints WHERE run_id = ?1 AND kind = 'Step' ORDER BY timestamp DESC LIMIT 1",
            params![&run_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten();

    let mut report = ReplayReport {
        run_id,
        match_status: false,
        original_digest: final_digest.clone().unwrap_or_default(),
        replay_digest,
        error_message: None,
        semantic_original_digest: None,
        semantic_replay_digest: None,
        semantic_distance: None,
        epsilon: None,
    };

    if final_digest.is_none() || report.original_digest.is_empty() {
        report.error_message = Some("no outputs digest recorded for run".to_string());
    } else if report.original_digest != report.replay_digest {
        report.error_message = Some("outputs digest mismatch".to_string());
    } else {
        report.match_status = true;
    }

    Ok(report)
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
        let spec = RunSpec {
            project_id: project.id.clone(),
            name: "interactive-replay".into(),
            seed: 0,
            token_budget: 10_000,
            model: run_model.clone(),
            checkpoints: vec![orchestrator::RunCheckpointTemplate {
                model: run_model.clone(),
                prompt: chat_prompt.clone(),
                token_budget: 10_000,
                order_index: Some(0),
                checkpoint_type: "InteractiveChat".to_string(),
            }],
            proof_mode: RunProofMode::Exact,
            epsilon: None,
        };

        let panic_client = PanicLlmClient;
        let run_id = orchestrator::start_hello_run_with_client(&pool, spec.clone(), &panic_client)?;

        let config_id: String = {
            let conn = pool.get()?;
            conn.query_row(
                "SELECT id FROM run_checkpoints WHERE run_id = ?1",
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
            });
        }
    };

    let has_concordant = stored_run
        .checkpoints
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
        });
    }

    let epsilon = stored_run
        .epsilon
        .ok_or_else(|| anyhow!("concordant run missing epsilon"))?;

    if stored_run.checkpoints.is_empty() {
        return Ok(ReplayReport {
            run_id,
            match_status: false,
            original_digest: String::new(),
            replay_digest: String::new(),
            error_message: Some("run has no configured checkpoints".to_string()),
            semantic_original_digest: None,
            semantic_replay_digest: None,
            semantic_distance: None,
            epsilon: Some(epsilon),
        });
    }

    let mut replay_digest = String::new();
    let mut replay_semantic_digest = String::new();
    for config in &stored_run.checkpoints {
        if config.is_interactive_chat() {
            continue;
        }
        if config.model == "stub-model" {
            let (digest, semantic) = simulate_stub_checkpoint(stored_run.seed, config);
            replay_digest = digest;
            if matches!(config.proof_mode, RunProofMode::Concordant) {
                replay_semantic_digest = semantic;
            }
        } else {
            let generation = orchestrator::replay_llm_generation(&config.model, &config.prompt)?;
            replay_digest = provenance::sha256_hex(generation.response.as_bytes());
            if matches!(config.proof_mode, RunProofMode::Concordant) {
                replay_semantic_digest = provenance::semantic_digest(&generation.response);
            }
        }
    }

    let (original_digest_opt, semantic_digest_opt): (Option<String>, Option<String>) = conn
        .query_row(
            "SELECT outputs_sha256, semantic_digest FROM checkpoints WHERE run_id = ?1 AND kind = 'Step' ORDER BY timestamp DESC LIMIT 1",
            params![&run_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .unwrap_or((None, None));

    let mut report = ReplayReport {
        run_id,
        match_status: false,
        original_digest: original_digest_opt.clone().unwrap_or_default(),
        replay_digest,
        error_message: None,
        semantic_original_digest: semantic_digest_opt.clone(),
        semantic_replay_digest: Some(replay_semantic_digest.clone()),
        semantic_distance: None,
        epsilon: Some(epsilon),
    };

    if semantic_digest_opt.is_none() {
        report.error_message = Some("no semantic digest recorded for run".to_string());
        return Ok(report);
    }

    let original_semantic = semantic_digest_opt.unwrap();
    let distance = provenance::semantic_distance(&original_semantic, &replay_semantic_digest)
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
            });
        }
    };

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

    let mut report = ReplayReport {
        run_id: run_id.clone(),
        match_status: false,
        original_digest: String::new(),
        replay_digest: String::new(),
        error_message: None,
        semantic_original_digest: None,
        semantic_replay_digest: None,
        semantic_distance: None,
        epsilon: None,
    };

    if checkpoints.is_empty() {
        report.error_message = Some("no checkpoints recorded for run".to_string());
        return Ok(report);
    }

    let mut conversation_states: HashMap<Option<String>, ConversationState> = HashMap::new();
    let mut last_stored_curr = String::new();
    let mut last_computed_curr = String::new();
    let mut failure: Option<String> = None;

    for ck in &checkpoints {
        let turn_index = match ck.turn_index {
            Some(value) => value,
            None => {
                failure = Some(format!("checkpoint {} missing turn_index", ck.id));
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
            break;
        }

        if ck.prev_chain != state.expected_prev_chain {
            failure = Some(format!("checkpoint {} prev_chain mismatch", ck.id));
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
        last_computed_curr = computed_curr.clone();
        last_stored_curr = ck.curr_chain.clone();

        if computed_curr != ck.curr_chain {
            failure = Some(format!("checkpoint {} curr_chain mismatch", ck.id));
            break;
        }

        let signature_bytes = match STANDARD.decode(ck.signature.as_bytes()) {
            Ok(bytes) => bytes,
            Err(_) => {
                failure = Some(format!("checkpoint {} signature decoding failed", ck.id));
                break;
            }
        };

        let signature_array: [u8; ed25519_dalek::SIGNATURE_LENGTH] =
            match signature_bytes.try_into() {
                Ok(arr) => arr,
                Err(_) => {
                    failure = Some(format!("checkpoint {} signature length invalid", ck.id));
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
            break;
        }

        state.previous_checkpoint_id = Some(ck.id.clone());
        state.expected_prev_chain = ck.curr_chain.clone();
        state.expected_turn_index += 1;
    }

    report.original_digest = last_stored_curr;
    report.replay_digest = last_computed_curr;

    if let Some(reason) = failure {
        report.error_message = Some(reason);
    } else {
        report.match_status = true;
    }

    Ok(report)
}
