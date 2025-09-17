// In src-tauri/src/store/policies.rs
use crate::Error;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Policy {
    pub allow_network: bool,
    pub budget_tokens: u64,
    pub budget_usd: f64,
    pub budget_g_co2e: f64,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            allow_network: false,
            budget_tokens: 1_000,
            budget_usd: 10.0,
            budget_g_co2e: 1.0,
        }
    }
}

pub fn get(conn: &Connection, project_id: &str) -> Result<Policy, Error> {
    let policy_json: Option<String> = conn
        .query_row(
            "SELECT policy_json FROM policies WHERE project_id = ?1",
            params![project_id],
            |row| row.get(0),
        )
        .optional()?;

    match policy_json {
        Some(json) => serde_json::from_str(&json)
            .map_err(|e| Error::Api(format!("failed to parse policy JSON: {e}"))),
        None => Ok(Policy::default()),
    }
}

pub fn upsert(conn: &Connection, project_id: &str, policy: &Policy) -> Result<(), Error> {
    let policy_json = serde_json::to_string(policy)
        .map_err(|e| Error::Api(format!("failed to serialize policy: {e}")))?;

    conn.execute(
        "INSERT INTO policies (project_id, policy_json) VALUES (?1, ?2)
         ON CONFLICT(project_id) DO UPDATE SET policy_json = excluded.policy_json",
        params![project_id, policy_json],
    )?;

    Ok(())
}
