use std::convert::TryFrom;

use crate::Error;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

fn normalize_policy_version(policy_version: Option<i64>) -> i64 {
    policy_version.unwrap_or(0).max(0)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectUsageLedger {
    pub project_id: String,
    pub policy_version: i64,
    pub total_tokens: u64,
    pub total_usd: f64,
    pub total_nature_cost: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

pub fn get(
    conn: &Connection,
    project_id: &str,
    policy_version: Option<i64>,
) -> Result<ProjectUsageLedger, Error> {
    let normalized_version = normalize_policy_version(policy_version);
    let row: Option<(i64, f64, f64, Option<String>)> = conn
        .query_row(
            concat!(
                "SELECT total_tokens, total_usd, total_nature_cost, updated_at ",
                "FROM project_usage_ledgers ",
                "WHERE project_id = ?1 AND policy_version = ?2"
            ),
            params![project_id, normalized_version],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?;

    if let Some((tokens_raw, usd, nature_cost, updated_at)) = row {
        let total_tokens = tokens_raw.max(0) as u64;
        Ok(ProjectUsageLedger {
            project_id: project_id.to_string(),
            policy_version: normalized_version,
            total_tokens,
            total_usd: usd,
            total_nature_cost: nature_cost,
            updated_at,
        })
    } else {
        Ok(ProjectUsageLedger {
            project_id: project_id.to_string(),
            policy_version: normalized_version,
            total_tokens: 0,
            total_usd: 0.0,
            total_nature_cost: 0.0,
            updated_at: None,
        })
    }
}

pub fn increment(
    conn: &Connection,
    project_id: &str,
    policy_version: Option<i64>,
    delta_tokens: u64,
    delta_usd: f64,
    delta_nature_cost: f64,
) -> Result<ProjectUsageLedger, Error> {
    let normalized_version = normalize_policy_version(policy_version);
    let delta_tokens_i64 = i64::try_from(delta_tokens)
        .map_err(|_| Error::Api("token delta exceeds supported range".to_string()))?;

    conn.execute(
        concat!(
            "INSERT INTO project_usage_ledgers ",
            "(project_id, policy_version, total_tokens, total_usd, total_nature_cost) ",
            "VALUES (?1, ?2, ?3, ?4, ?5) ",
            "ON CONFLICT(project_id, policy_version) DO UPDATE SET ",
            "total_tokens = total_tokens + excluded.total_tokens, ",
            "total_usd = total_usd + excluded.total_usd, ",
            "total_nature_cost = total_nature_cost + excluded.total_nature_cost, ",
            "updated_at = CURRENT_TIMESTAMP"
        ),
        params![
            project_id,
            normalized_version,
            delta_tokens_i64,
            delta_usd,
            delta_nature_cost
        ],
    )?;

    get(conn, project_id, Some(normalized_version))
}
