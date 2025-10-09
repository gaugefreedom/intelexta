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
    pub budget_nature_cost: f64, // Renamed from budget_g_co2e
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyVersion {
    pub id: i64,
    pub project_id: String,
    pub version: i64,
    pub policy: Policy,
    pub created_at: String,
    pub created_by: Option<String>,
    pub change_notes: Option<String>,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            allow_network: false,
            budget_tokens: 1_000,
            budget_usd: 10.0,
            budget_nature_cost: 100.0, // Higher default, more flexible metric
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

pub fn get_for_policy_version(
    conn: &Connection,
    project_id: &str,
    policy_version: Option<i64>,
) -> Result<Policy, Error> {
    if let Some(version) = policy_version {
        if version > 0 {
            if let Some(policy_version) = get_version(conn, project_id, version)? {
                return Ok(policy_version.policy);
            }
        }
    }

    get(conn, project_id)
}

pub fn upsert(conn: &Connection, project_id: &str, policy: &Policy) -> Result<(), Error> {
    upsert_with_notes(conn, project_id, policy, None, None)
}

pub fn upsert_with_notes(
    conn: &Connection,
    project_id: &str,
    policy: &Policy,
    created_by: Option<&str>,
    change_notes: Option<&str>,
) -> Result<(), Error> {
    let policy_json = serde_json::to_string(policy)
        .map_err(|e| Error::Api(format!("failed to serialize policy: {e}")))?;

    // Get current version or default to 0
    let current_version: i64 = conn
        .query_row(
            "SELECT current_version FROM policies WHERE project_id = ?1",
            params![project_id],
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or(0);

    let new_version = current_version + 1;

    // Insert new version into policy_versions table
    conn.execute(
        "INSERT INTO policy_versions (project_id, version, policy_json, created_by, change_notes)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            project_id,
            new_version,
            policy_json,
            created_by,
            change_notes
        ],
    )?;

    // Update policies table with new policy and version
    conn.execute(
        "INSERT INTO policies (project_id, policy_json, current_version) VALUES (?1, ?2, ?3)
         ON CONFLICT(project_id) DO UPDATE SET
            policy_json = excluded.policy_json,
            current_version = excluded.current_version",
        params![project_id, policy_json, new_version],
    )?;

    // Migrate usage ledger from previous version to new version
    // This preserves accumulated usage while allowing new budgets to apply
    if current_version > 0 {
        conn.execute(
            "INSERT INTO project_usage_ledgers (project_id, policy_version, total_tokens, total_usd, total_nature_cost)
             SELECT project_id, ?1, total_tokens, total_usd, total_nature_cost
             FROM project_usage_ledgers
             WHERE project_id = ?2 AND policy_version = ?3
             ON CONFLICT(project_id, policy_version) DO NOTHING",
            params![new_version, project_id, current_version],
        )?;
    }

    // Automatically upgrade all runs to the new policy version
    // This ensures that existing runs immediately benefit from updated budgets
    conn.execute(
        "UPDATE runs SET policy_version = ?1 WHERE project_id = ?2",
        params![new_version, project_id],
    )?;

    Ok(())
}

/// Get all policy versions for a project
pub fn get_versions(conn: &Connection, project_id: &str) -> Result<Vec<PolicyVersion>, Error> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, version, policy_json, created_at, created_by, change_notes
         FROM policy_versions
         WHERE project_id = ?1
         ORDER BY version DESC",
    )?;

    let versions = stmt
        .query_map(params![project_id], |row| {
            let policy_json: String = row.get(3)?;
            let policy: Policy = serde_json::from_str(&policy_json).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    3,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;

            Ok(PolicyVersion {
                id: row.get(0)?,
                project_id: row.get(1)?,
                version: row.get(2)?,
                policy,
                created_at: row.get(4)?,
                created_by: row.get(5)?,
                change_notes: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(versions)
}

/// Get a specific policy version
pub fn get_version(
    conn: &Connection,
    project_id: &str,
    version: i64,
) -> Result<Option<PolicyVersion>, Error> {
    let row = conn
        .query_row(
            "SELECT id, project_id, version, policy_json, created_at, created_by, change_notes
             FROM policy_versions
             WHERE project_id = ?1 AND version = ?2",
            params![project_id, version],
            |row| {
                let policy_json: String = row.get(3)?;
                let policy: Policy = serde_json::from_str(&policy_json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;

                Ok(PolicyVersion {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    version: row.get(2)?,
                    policy,
                    created_at: row.get(4)?,
                    created_by: row.get(5)?,
                    change_notes: row.get(6)?,
                })
            },
        )
        .optional()?;

    Ok(row)
}

/// Get the current policy version number
pub fn get_current_version(conn: &Connection, project_id: &str) -> Result<i64, Error> {
    let version = conn
        .query_row(
            "SELECT current_version FROM policies WHERE project_id = ?1",
            params![project_id],
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or(0);

    Ok(version)
}
