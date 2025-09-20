// In src-tauri/src/replay.rs
use crate::{
    orchestrator::{self, RunProofMode, RunSpec},
    provenance, DbPool,
};
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
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

pub fn replay_exact_run(run_id: String, pool: &DbPool) -> Result<ReplayReport> {
    let conn = pool.get()?;

    let spec_json_opt: Option<String> = conn
        .query_row(
            "SELECT spec_json FROM runs WHERE id = ?1",
            params![&run_id],
            |row| row.get(0),
        )
        .optional()?;

    let spec_json = match spec_json_opt {
        Some(value) => value,
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

    let spec: RunSpec = serde_json::from_str(&spec_json)
        .map_err(|err| anyhow!("failed to parse stored run spec: {err}"))?;

    let mut replay_input = b"hello".to_vec();
    replay_input.extend_from_slice(&spec.seed.to_le_bytes());
    let replay_digest = provenance::sha256_hex(&replay_input);

    let final_digest: Option<String> = conn
        .query_row(
            "SELECT outputs_sha256 FROM checkpoints WHERE run_id = ?1 ORDER BY timestamp DESC LIMIT 1",
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

pub fn replay_concordant_run(run_id: String, pool: &DbPool) -> Result<ReplayReport> {
    let conn = pool.get()?;

    let spec_json_opt: Option<String> = conn
        .query_row(
            "SELECT spec_json FROM runs WHERE id = ?1",
            params![&run_id],
            |row| row.get(0),
        )
        .optional()?;

    let spec_json = match spec_json_opt {
        Some(value) => value,
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

    let spec: RunSpec = serde_json::from_str(&spec_json)
        .map_err(|err| anyhow!("failed to parse stored run spec: {err}"))?;

    if !matches!(spec.proof_mode, RunProofMode::Concordant) {
        return Ok(ReplayReport {
            run_id,
            match_status: false,
            original_digest: String::new(),
            replay_digest: String::new(),
            error_message: Some("run is not a concordant replay".to_string()),
            semantic_original_digest: None,
            semantic_replay_digest: None,
            semantic_distance: None,
            epsilon: None,
        });
    }

    let epsilon = spec
        .epsilon
        .ok_or_else(|| anyhow!("concordant run missing epsilon"))?;

    let (replay_digest, replay_semantic_digest) = if spec.model == "stub-model" {
        let mut replay_bytes = b"hello".to_vec();
        replay_bytes.extend_from_slice(&spec.seed.to_le_bytes());
        let replay_digest = provenance::sha256_hex(&replay_bytes);
        let semantic_source = hex::encode(&replay_bytes);
        let replay_semantic_digest = provenance::semantic_digest(&semantic_source);
        (replay_digest, replay_semantic_digest)
    } else {
        let replay_generation = orchestrator::replay_llm_generation(&spec)?;
        let replay_digest = provenance::sha256_hex(replay_generation.response.as_bytes());
        let replay_semantic_digest = provenance::semantic_digest(&replay_generation.response);
        (replay_digest, replay_semantic_digest)
    };

    let (original_digest_opt, semantic_digest_opt): (Option<String>, Option<String>) = conn
        .query_row(
            "SELECT outputs_sha256, semantic_digest FROM checkpoints WHERE run_id = ?1 ORDER BY timestamp DESC LIMIT 1",
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

struct InteractiveCheckpointRow {
    id: String,
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
        "SELECT id, parent_checkpoint_id, turn_index, kind, timestamp, inputs_sha256, outputs_sha256, incident_json, prev_chain, curr_chain, signature, usage_tokens, prompt_tokens, completion_tokens
         FROM checkpoints WHERE run_id = ?1 ORDER BY turn_index ASC, timestamp ASC",
    )?;

    let rows = stmt.query_map(params![&run_id], |row| {
        let incident_json: Option<String> = row.get(7)?;
        let incident = incident_json
            .map(|payload| serde_json::from_str::<Value>(&payload))
            .transpose()
            .map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(
                    7,
                    rusqlite::types::Type::Text,
                    Box::new(err),
                )
            })?;
        let turn_index = row
            .get::<_, Option<i64>>(2)?
            .map(|value| value.max(0) as u32);
        let usage_tokens: i64 = row.get(11)?;
        let prompt_tokens: i64 = row.get(12)?;
        let completion_tokens: i64 = row.get(13)?;
        Ok(InteractiveCheckpointRow {
            id: row.get(0)?,
            parent_checkpoint_id: row.get(1)?,
            turn_index,
            kind: row.get(3)?,
            timestamp: row.get(4)?,
            inputs_sha256: row.get(5)?,
            outputs_sha256: row.get(6)?,
            incident,
            prev_chain: row.get(8)?,
            curr_chain: row.get(9)?,
            signature: row.get(10)?,
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

    let mut expected_turn_index = 0_u32;
    let mut expected_prev_chain = String::new();
    let mut previous_checkpoint_id: Option<String> = None;
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

        if turn_index != expected_turn_index {
            failure = Some(format!(
                "checkpoint {} turn_index {} out of sequence (expected {})",
                ck.id, turn_index, expected_turn_index
            ));
            break;
        }

        if turn_index == 0 {
            if ck.parent_checkpoint_id.is_some() {
                failure = Some(format!(
                    "first checkpoint {} unexpectedly has a parent",
                    ck.id
                ));
                break;
            }
        } else if ck.parent_checkpoint_id.as_deref() != previous_checkpoint_id.as_deref() {
            failure = Some(format!(
                "checkpoint {} parent mismatch (expected {:?}, found {:?})",
                ck.id,
                previous_checkpoint_id.as_deref(),
                ck.parent_checkpoint_id.as_deref()
            ));
            break;
        }

        if ck.prev_chain != expected_prev_chain {
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

        expected_prev_chain = ck.curr_chain.clone();
        previous_checkpoint_id = Some(ck.id.clone());
        expected_turn_index += 1;
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
