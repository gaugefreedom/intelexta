use crate::{orchestrator::RunSpec, provenance, DbPool};
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
