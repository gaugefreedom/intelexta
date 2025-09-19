use crate::{
    orchestrator::{RunProofMode, RunSpec},
    provenance, DbPool,
};
use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

    if spec.model != "stub-model" {
        return Ok(ReplayReport {
            run_id,
            match_status: false,
            original_digest: String::new(),
            replay_digest: String::new(),
            error_message: Some("semantic replay not implemented for model".to_string()),
            semantic_original_digest: None,
            semantic_replay_digest: None,
            semantic_distance: None,
            epsilon: Some(epsilon),
        });
    }

    let mut replay_bytes = b"hello".to_vec();
    replay_bytes.extend_from_slice(&spec.seed.to_le_bytes());
    let replay_digest = provenance::sha256_hex(&replay_bytes);
    let semantic_source = hex::encode(&replay_bytes);
    let replay_semantic_digest = provenance::semantic_digest(&semantic_source);

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

    if (distance as f64) <= epsilon {
        report.match_status = true;
    } else {
        report.error_message = Some(format!(
            "semantic distance {distance} exceeded epsilon {epsilon}"
        ));
    }

    Ok(report)
}
