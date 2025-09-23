// In src-tauri/src/api.rs
use crate::{
    car, orchestrator, portability, provenance, replay,
    store::{self, policies::Policy},
    DbPool, Error, Project,
};
use rusqlite::{params, types::Type, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
#[cfg(feature = "interactive")]
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, State};
use uuid::Uuid;

#[tauri::command]
pub fn list_projects(pool: State<DbPool>) -> Result<Vec<Project>, Error> {
    let conn = pool.get()?;
    let projects = store::projects::list(&conn)?;
    Ok(projects)
}

#[tauri::command]
pub fn list_local_models() -> Result<Vec<String>, Error> {
    orchestrator::list_local_models().map_err(|err| Error::Api(err.to_string()))
}

#[tauri::command]
pub fn create_project(name: String, pool: State<DbPool>) -> Result<Project, Error> {
    create_project_with_pool(name, pool.inner())
}

pub(crate) fn create_project_with_pool(name: String, pool: &DbPool) -> Result<Project, Error> {
    let project_id = Uuid::new_v4().to_string();
    let kp = provenance::generate_keypair();

    provenance::store_secret_key(&project_id, &kp.secret_key_b64)
        .map_err(|e| Error::Api(format!("Failed to store secret key: {}", e)))?;

    let conn = pool.get()?;
    let project = store::projects::create(&conn, &project_id, &name, &kp.public_key_b64)?;

    Ok(project)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HelloRunSpec {
    pub project_id: String,
    pub name: String,
    pub seed: u64,
    pub dag_json: String,
    pub token_budget: u64,
    pub model: String,
    #[serde(default)]
    pub proof_mode: orchestrator::RunProofMode,
    #[serde(default)]
    pub epsilon: Option<f64>,
}

#[tauri::command]
pub fn create_run(
    project_id: String,
    name: String,
    proof_mode: orchestrator::RunProofMode,
    seed: u64,
    token_budget: u64,
    default_model: String,
    epsilon: Option<f64>,
    pool: State<DbPool>,
) -> Result<String, Error> {
    let spec = orchestrator::RunSpec {
        project_id,
        name,
        seed,
        token_budget,
        model: default_model,
        checkpoints: Vec::new(),
        proof_mode,
        epsilon,
    };

    orchestrator::create_run(pool.inner(), spec).map_err(|err| Error::Api(err.to_string()))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointConfigRequest {
    pub model: String,
    pub prompt: String,
    pub token_budget: u64,
    #[serde(default)]
    pub checkpoint_type: Option<String>,
    #[serde(default)]
    pub order_index: Option<i64>,
    #[serde(default)]
    pub proof_mode: orchestrator::RunProofMode,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckpointConfigRequest {
    pub model: Option<String>,
    pub prompt: Option<String>,
    pub token_budget: Option<u64>,
    pub checkpoint_type: Option<String>,
    pub proof_mode: Option<orchestrator::RunProofMode>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProjectArgs {
    #[serde(default)]
    pub archive_path: Option<String>,
    #[serde(default)]
    pub file_name: Option<String>,
    #[serde(default)]
    pub bytes: Option<Vec<u8>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportCarArgs {
    #[serde(default)]
    pub car_path: Option<String>,
    #[serde(default)]
    pub file_name: Option<String>,
    #[serde(default)]
    pub bytes: Option<Vec<u8>>,
}

// In src-tauri/src/api.rs

#[tauri::command]
pub fn start_hello_run(spec: HelloRunSpec, pool: State<DbPool>) -> Result<String, Error> {
    let rs = orchestrator::RunSpec {
        project_id: spec.project_id,
        name: spec.name,
        seed: spec.seed,
        token_budget: spec.token_budget,
        model: spec.model.clone(),
        checkpoints: vec![orchestrator::RunCheckpointTemplate {
            model: spec.model,
            prompt: spec.dag_json,
            token_budget: spec.token_budget,
            order_index: Some(0),
            checkpoint_type: "Step".to_string(),
            proof_mode: spec.proof_mode,
        }],
        proof_mode: spec.proof_mode,
        epsilon: spec.epsilon,
    };

    // --- FIX: The debugging code now lives INSIDE the function ---
    let result = orchestrator::start_hello_run(&pool, rs);

    // If the result is an error, print the detailed reason to the terminal.
    if let Err(e) = &result {
        println!("[DEBUG] orchestrator::start_hello_run failed with: {:?}", e);
    }

    result.map_err(|e| Error::Api(e.to_string()))
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSummary {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epsilon: Option<f64>,
    pub has_persisted_checkpoint: bool,
}

fn hydrate_run_summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<RunSummary> {
    Ok(RunSummary {
        id: row.get(0)?,
        name: row.get(1)?,
        created_at: row.get(2)?,
        kind: row.get(3)?,
        epsilon: row.get(4)?,
        has_persisted_checkpoint: row.get(5)?,
    })
}

#[tauri::command]
pub fn list_runs(project_id: String, pool: State<DbPool>) -> Result<Vec<RunSummary>, Error> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT r.id, r.name, r.created_at, r.kind, r.epsilon, EXISTS (SELECT 1 FROM checkpoints c WHERE c.run_id = r.id) AS has_persisted_checkpoint FROM runs r WHERE r.project_id = ?1 ORDER BY r.created_at DESC",
    )?;
    let runs_iter = stmt.query_map(params![project_id], hydrate_run_summary)?;
    let mut runs = Vec::new();
    for run in runs_iter {
        runs.push(run?);
    }
    Ok(runs)
}

fn load_run_summary(conn: &Connection, run_id: &str) -> Result<RunSummary, Error> {
    let summary = conn
        .query_row(
            "SELECT r.id, r.name, r.created_at, r.kind, r.epsilon, EXISTS (SELECT 1 FROM checkpoints c WHERE c.run_id = r.id) AS has_persisted_checkpoint FROM runs r WHERE r.id = ?1",
            params![run_id],
            hydrate_run_summary,
        )
        .optional()?;

    summary.ok_or_else(|| Error::Api(format!("run {run_id} not found")))
}

#[tauri::command]
pub fn update_run_settings(
    run_id: String,
    proof_mode: orchestrator::RunProofMode,
    epsilon: Option<f64>,
    pool: State<DbPool>,
) -> Result<RunSummary, Error> {
    orchestrator::update_run_proof_settings(pool.inner(), &run_id, proof_mode, epsilon)
        .map_err(|err| Error::Api(err.to_string()))?;

    let conn = pool.get()?;
    load_run_summary(&conn, &run_id)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointSummary {
    pub id: String,
    pub timestamp: String,
    pub kind: String,
    pub incident: Option<IncidentSummary>,
    pub inputs_sha256: Option<String>,
    pub outputs_sha256: Option<String>,
    pub semantic_digest: Option<String>,
    pub usage_tokens: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub parent_checkpoint_id: Option<String>,
    pub turn_index: Option<u32>,
    pub checkpoint_config_id: Option<String>,
    pub message: Option<CheckpointMessageSummary>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointDetails {
    pub id: String,
    pub run_id: String,
    pub timestamp: String,
    pub kind: String,
    pub incident: Option<IncidentSummary>,
    pub inputs_sha256: Option<String>,
    pub outputs_sha256: Option<String>,
    pub semantic_digest: Option<String>,
    pub usage_tokens: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub parent_checkpoint_id: Option<String>,
    pub turn_index: Option<u32>,
    pub checkpoint_config_id: Option<String>,
    pub prompt_payload: Option<String>,
    pub output_payload: Option<String>,
    pub message: Option<CheckpointMessageSummary>,
}

#[cfg(feature = "interactive")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractiveCheckpointSession {
    pub checkpoint: orchestrator::RunCheckpointConfig,
    pub messages: Vec<CheckpointSummary>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncidentSummary {
    pub kind: String,
    pub severity: String,
    pub details: String,
    #[serde(alias = "related_checkpoint_id")]
    pub related_checkpoint_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointMessageSummary {
    pub role: String,
    pub body: String,
    pub created_at: String,
    pub updated_at: Option<String>,
}

#[tauri::command]
pub fn list_checkpoints(
    run_id: String,
    pool: State<DbPool>,
) -> Result<Vec<CheckpointSummary>, Error> {
    list_checkpoints_with_pool(run_id, pool.inner())
}

#[tauri::command]
pub fn get_checkpoint_details(
    checkpoint_id: String,
    pool: State<DbPool>,
) -> Result<CheckpointDetails, Error> {
    get_checkpoint_details_with_pool(checkpoint_id, pool.inner())
}

#[cfg(feature = "interactive")]
#[tauri::command]
pub fn open_interactive_checkpoint_session(
    run_id: String,
    checkpoint_id: String,
    pool: State<DbPool>,
) -> Result<InteractiveCheckpointSession, Error> {
    let conn = pool.get()?;
    let config = load_checkpoint_config(&conn, &checkpoint_id)?;

    if config.run_id != run_id {
        return Err(Error::Api(
            "checkpoint configuration does not belong to the specified run".to_string(),
        ));
    }

    if !config.is_interactive_chat() {
        return Err(Error::Api(
            "checkpoint is not configured for interactive chat".to_string(),
        ));
    }

    drop(conn);

    let mut messages = list_checkpoints_with_pool(run_id.clone(), pool.inner())?;
    let checkpoint_id_ref = checkpoint_id.as_str();
    messages.retain(|entry| {
        entry
            .checkpoint_config_id
            .as_deref()
            .map(|value| value == checkpoint_id_ref)
            .unwrap_or(false)
    });

    Ok(InteractiveCheckpointSession {
        checkpoint: config,
        messages,
    })
}

pub(crate) fn list_checkpoints_with_pool(
    run_id: String,
    pool: &DbPool,
) -> Result<Vec<CheckpointSummary>, Error> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT c.id, c.timestamp, c.kind, c.incident_json, c.inputs_sha256, c.outputs_sha256, c.semantic_digest, c.usage_tokens, c.prompt_tokens, c.completion_tokens, c.parent_checkpoint_id, c.turn_index, c.checkpoint_config_id, m.role, m.body, m.created_at, m.updated_at
         FROM checkpoints c
         LEFT JOIN checkpoint_messages m ON m.checkpoint_id = c.id
         WHERE c.run_id = ?1
         ORDER BY c.timestamp ASC",
    )?;
    let rows = stmt.query_map(params![run_id], |row| {
        let incident_json: Option<String> = row.get(3)?;
        let incident = incident_json
            .map(|payload| serde_json::from_str::<IncidentSummary>(&payload))
            .transpose()
            .map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(3, Type::Text, Box::new(err))
            })?;
        let parent_checkpoint_id: Option<String> = row.get(10)?;
        let turn_index = row
            .get::<_, Option<i64>>(11)?
            .map(|value| value.max(0) as u32);
        let checkpoint_config_id: Option<String> = row.get(12)?;
        let message_role: Option<String> = row.get(13)?;
        let message_body: Option<String> = row.get(14)?;
        let message_created_at: Option<String> = row.get(15)?;
        let message_updated_at: Option<String> = row.get(16)?;
        let message = match (message_role, message_body, message_created_at) {
            (Some(role), Some(body), Some(created_at)) => Some(CheckpointMessageSummary {
                role,
                body,
                created_at,
                updated_at: message_updated_at,
            }),
            _ => None,
        };
        Ok(CheckpointSummary {
            id: row.get(0)?,
            timestamp: row.get(1)?,
            kind: row.get(2)?,
            incident,
            inputs_sha256: row.get(4)?,
            outputs_sha256: row.get(5)?,
            semantic_digest: row.get(6)?,
            usage_tokens: {
                let value: i64 = row.get(7)?;
                value.max(0) as u64
            },
            prompt_tokens: {
                let value: i64 = row.get(8)?;
                value.max(0) as u64
            },
            completion_tokens: {
                let value: i64 = row.get(9)?;
                value.max(0) as u64
            },
            parent_checkpoint_id,
            turn_index,
            checkpoint_config_id,
            message,
        })
    })?;
    let mut checkpoints = Vec::new();
    for row in rows {
        checkpoints.push(row?);
    }
    Ok(checkpoints)
}

pub(crate) fn get_checkpoint_details_with_pool(
    checkpoint_id: String,
    pool: &DbPool,
) -> Result<CheckpointDetails, Error> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT c.id, c.run_id, c.timestamp, c.kind, c.incident_json, c.inputs_sha256, c.outputs_sha256, c.semantic_digest, c.usage_tokens, c.prompt_tokens, c.completion_tokens, c.parent_checkpoint_id, c.turn_index, c.checkpoint_config_id, p.prompt_payload, p.output_payload, m.role, m.body, m.created_at, m.updated_at
         FROM checkpoints c
         LEFT JOIN checkpoint_payloads p ON p.checkpoint_id = c.id
         LEFT JOIN checkpoint_messages m ON m.checkpoint_id = c.id
         WHERE c.id = ?1",
    )?;

    let result = stmt.query_row(params![checkpoint_id], |row| {
        let incident_json: Option<String> = row.get(4)?;
        let incident = incident_json
            .map(|payload| serde_json::from_str::<IncidentSummary>(&payload))
            .transpose()
            .map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(4, Type::Text, Box::new(err))
            })?;
        let parent_checkpoint_id: Option<String> = row.get(11)?;
        let turn_index = row
            .get::<_, Option<i64>>(12)?
            .map(|value| value.max(0) as u32);
        let checkpoint_config_id: Option<String> = row.get(13)?;
        let prompt_payload: Option<String> = row.get(14)?;
        let output_payload: Option<String> = row.get(15)?;
        let message_role: Option<String> = row.get(16)?;
        let message_body: Option<String> = row.get(17)?;
        let message_created_at: Option<String> = row.get(18)?;
        let message_updated_at: Option<String> = row.get(19)?;
        let message = match (message_role, message_body, message_created_at) {
            (Some(role), Some(body), Some(created_at)) => Some(CheckpointMessageSummary {
                role,
                body,
                created_at,
                updated_at: message_updated_at,
            }),
            _ => None,
        };

        Ok(CheckpointDetails {
            id: row.get(0)?,
            run_id: row.get(1)?,
            timestamp: row.get(2)?,
            kind: row.get(3)?,
            incident,
            inputs_sha256: row.get(5)?,
            outputs_sha256: row.get(6)?,
            semantic_digest: row.get(7)?,
            usage_tokens: {
                let value: i64 = row.get(8)?;
                value.max(0) as u64
            },
            prompt_tokens: {
                let value: i64 = row.get(9)?;
                value.max(0) as u64
            },
            completion_tokens: {
                let value: i64 = row.get(10)?;
                value.max(0) as u64
            },
            parent_checkpoint_id,
            turn_index,
            checkpoint_config_id,
            prompt_payload,
            output_payload,
            message,
        })
    });

    match result {
        Ok(details) => Ok(details),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            Err(Error::Api("checkpoint not found".to_string()))
        }
        Err(err) => Err(err.into()),
    }
}

#[cfg(feature = "interactive")]
#[tauri::command]
pub fn submit_interactive_checkpoint_turn(
    run_id: String,
    checkpoint_id: String,
    prompt_text: String,
    pool: State<DbPool>,
) -> Result<orchestrator::SubmitTurnOutcome, Error> {
    orchestrator::submit_interactive_checkpoint_turn(
        pool.inner(),
        &run_id,
        &checkpoint_id,
        &prompt_text,
    )
    .map_err(|err| Error::Api(err.to_string()))
}

#[cfg(feature = "interactive")]
#[tauri::command]
pub fn finalize_interactive_checkpoint(
    run_id: String,
    checkpoint_id: String,
    pool: State<DbPool>,
) -> Result<(), Error> {
    orchestrator::finalize_interactive_checkpoint(pool.inner(), &run_id, &checkpoint_id)
        .map_err(|err| Error::Api(err.to_string()))
}

fn load_checkpoint_config(
    conn: &Connection,
    checkpoint_id: &str,
) -> Result<orchestrator::RunCheckpointConfig, Error> {
    let row: Option<(String, i64, String, String, String, i64, String)> = conn
        .query_row(
            "SELECT run_id, order_index, checkpoint_type, model, prompt, token_budget, proof_mode FROM run_checkpoints WHERE id = ?1",
            params![checkpoint_id],
            |row| Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            )),
        )
        .optional()?;

    let (run_id, order_index, checkpoint_type, model, prompt, token_budget_raw, proof_mode_raw) =
        row.ok_or_else(|| Error::Api(format!("checkpoint config {checkpoint_id} not found")))?;

    let proof_mode =
        orchestrator::RunProofMode::try_from(proof_mode_raw.as_str()).map_err(|err| {
            Error::from(rusqlite::Error::FromSqlConversionFailure(
                6,
                rusqlite::types::Type::Text,
                Box::new(err),
            ))
        })?;

    Ok(orchestrator::RunCheckpointConfig {
        id: checkpoint_id.to_string(),
        run_id,
        order_index,
        checkpoint_type,
        model,
        prompt,
        token_budget: token_budget_raw.max(0) as u64,
        proof_mode,
    })
}

#[tauri::command]
pub fn get_policy(project_id: String, pool: State<DbPool>) -> Result<Policy, Error> {
    let conn = pool.get()?;
    store::policies::get(&conn, &project_id)
}

#[tauri::command]
pub fn replay_run(run_id: String, pool: State<DbPool>) -> Result<replay::ReplayReport, Error> {
    replay_run_with_pool(run_id, pool.inner())
}

pub(crate) fn replay_run_with_pool(
    run_id: String,
    pool: &DbPool,
) -> Result<replay::ReplayReport, Error> {
    let conn = pool.get()?;
    let stored_run = match orchestrator::load_stored_run(&conn, &run_id) {
        Ok(run) => run,
        Err(err) => {
            let message = err.to_string();
            if message.contains("not found") {
                return Ok(replay::ReplayReport::from_checkpoint_reports(
                    run_id,
                    Vec::new(),
                    Some("run not found".to_string()),
                ));
            }
            return Err(Error::Api(message));
        }
    };

    if stored_run.checkpoints.is_empty() {
        return Ok(replay::ReplayReport::from_checkpoint_reports(
            run_id,
            Vec::new(),
            Some("run has no configured checkpoints".to_string()),
        ));
    }

    let mut checkpoint_reports: Vec<replay::CheckpointReplayReport> = Vec::new();

    #[cfg(feature = "interactive")]
    let mut interactive_lookup: HashMap<Option<String>, replay::CheckpointReplayReport> =
        HashMap::new();
    #[cfg(feature = "interactive")]
    let mut interactive_default_error: Option<String> = None;

    #[cfg(feature = "interactive")]
    let has_interactive_configs = stored_run
        .checkpoints
        .iter()
        .any(|cfg| cfg.is_interactive_chat());

    #[cfg(feature = "interactive")]
    if has_interactive_configs {
        let interactive_report = replay::replay_interactive_run(run_id.clone(), pool)
            .map_err(|err| Error::Api(err.to_string()))?;
        interactive_default_error = interactive_report.error_message.clone();
        for entry in interactive_report.checkpoint_reports {
            interactive_lookup.insert(entry.checkpoint_config_id.clone(), entry);
        }
    }

    #[cfg(not(feature = "interactive"))]
    if stored_run
        .checkpoints
        .iter()
        .any(|cfg| cfg.is_interactive_chat())
    {
        return Err(Error::Api(
            "Interactive replays are disabled in this build.".to_string(),
        ));
    }

    for config in &stored_run.checkpoints {
        if config.is_interactive_chat() {
            #[cfg(feature = "interactive")]
            {
                let report = interactive_lookup
                    .remove(&Some(config.id.clone()))
                    .unwrap_or_else(|| replay::CheckpointReplayReport {
                        checkpoint_config_id: Some(config.id.clone()),
                        checkpoint_type: Some(config.checkpoint_type.clone()),
                        order_index: Some(config.order_index),
                        mode: replay::CheckpointReplayMode::Interactive,
                        match_status: false,
                        original_digest: String::new(),
                        replay_digest: String::new(),
                        error_message: interactive_default_error.clone().or_else(|| {
                            Some("no interactive checkpoints recorded for config".to_string())
                        }),
                        semantic_original_digest: None,
                        semantic_replay_digest: None,
                        semantic_distance: None,
                        epsilon: None,
                    });
                checkpoint_reports.push(report);
            }

            #[cfg(not(feature = "interactive"))]
            {
                let _ = config;
            }

            continue;
        }

        let report = if matches!(config.proof_mode, orchestrator::RunProofMode::Concordant) {
            replay::replay_concordant_checkpoint(&stored_run, &conn, config)
        } else {
            replay::replay_exact_checkpoint(&stored_run, &conn, config)
        }
        .map_err(|err| Error::Api(err.to_string()))?;
        checkpoint_reports.push(report);
    }

    #[cfg(feature = "interactive")]
    {
        for (_key, report) in interactive_lookup.into_iter() {
            checkpoint_reports.push(report);
        }
    }

    checkpoint_reports.sort_by(|a, b| {
        let left = a.order_index.unwrap_or(i64::MAX);
        let right = b.order_index.unwrap_or(i64::MAX);
        left.cmp(&right)
            .then_with(|| a.checkpoint_config_id.cmp(&b.checkpoint_config_id))
    });

    Ok(replay::ReplayReport::from_checkpoint_reports(
        run_id,
        checkpoint_reports,
        None,
    ))
}

#[tauri::command]
pub fn list_run_checkpoint_configs(
    run_id: String,
    pool: State<DbPool>,
) -> Result<Vec<orchestrator::RunCheckpointConfig>, Error> {
    list_run_checkpoint_configs_with_pool(run_id, pool.inner())
}

pub(crate) fn list_run_checkpoint_configs_with_pool(
    run_id: String,
    pool: &DbPool,
) -> Result<Vec<orchestrator::RunCheckpointConfig>, Error> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT id, run_id, order_index, checkpoint_type, model, prompt, token_budget, proof_mode FROM run_checkpoints WHERE run_id = ?1 ORDER BY order_index ASC",
    )?;
    let rows = stmt.query_map(params![&run_id], |row| {
        let token_budget: i64 = row.get(6)?;
        let proof_mode_raw: String = row.get(7)?;
        let proof_mode =
            orchestrator::RunProofMode::try_from(proof_mode_raw.as_str()).map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(
                    7,
                    rusqlite::types::Type::Text,
                    Box::new(err),
                )
            })?;
        Ok(orchestrator::RunCheckpointConfig {
            id: row.get(0)?,
            run_id: row.get(1)?,
            order_index: row.get(2)?,
            checkpoint_type: row.get(3)?,
            model: row.get(4)?,
            prompt: row.get(5)?,
            token_budget: token_budget.max(0) as u64,
            proof_mode,
        })
    })?;

    let mut configs = Vec::new();
    for row in rows {
        configs.push(row?);
    }

    Ok(configs)
}

#[tauri::command]
pub fn create_checkpoint_config(
    run_id: String,
    config: CheckpointConfigRequest,
    pool: State<DbPool>,
) -> Result<orchestrator::RunCheckpointConfig, Error> {
    let mut conn = pool.get()?;
    let tx = conn.transaction()?;

    let exists: Option<()> = tx
        .query_row("SELECT 1 FROM runs WHERE id = ?1", params![&run_id], |_| {
            Ok(())
        })
        .optional()?;
    if exists.is_none() {
        return Err(Error::Api(format!("run {run_id} not found")));
    }

    let checkpoint_type = config.checkpoint_type.unwrap_or_else(|| "Step".to_string());
    let order_index = if let Some(index) = config.order_index {
        tx.execute(
            "UPDATE run_checkpoints SET order_index = order_index + 1, updated_at = CURRENT_TIMESTAMP WHERE run_id = ?1 AND order_index >= ?2",
            params![&run_id, index],
        )?;
        index
    } else {
        tx.query_row(
            "SELECT COALESCE(MAX(order_index), -1) + 1 FROM run_checkpoints WHERE run_id = ?1",
            params![&run_id],
            |row| row.get::<_, i64>(0),
        )?
    };

    let checkpoint_id = Uuid::new_v4().to_string();
    let CheckpointConfigRequest {
        model,
        prompt,
        token_budget,
        proof_mode,
        ..
    } = config;
    tx.execute(
        "INSERT INTO run_checkpoints (id, run_id, order_index, checkpoint_type, model, prompt, token_budget, proof_mode) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
            &checkpoint_id,
            &run_id,
            order_index,
            &checkpoint_type,
            &model,
            &prompt,
            (token_budget as i64),
            proof_mode.as_str(),
        ],
    )?;

    tx.commit()?;

    Ok(orchestrator::RunCheckpointConfig {
        id: checkpoint_id,
        run_id,
        order_index,
        checkpoint_type,
        model,
        prompt,
        token_budget,
        proof_mode,
    })
}

#[tauri::command]
pub fn update_checkpoint_config(
    checkpoint_id: String,
    updates: UpdateCheckpointConfigRequest,
    pool: State<DbPool>,
) -> Result<orchestrator::RunCheckpointConfig, Error> {
    let mut conn = pool.get()?;
    let tx = conn.transaction()?;
    let mut config = load_checkpoint_config(&tx, &checkpoint_id)?;

    if let Some(model) = updates.model {
        config.model = model;
    }
    if let Some(prompt) = updates.prompt {
        config.prompt = prompt;
    }
    if let Some(token_budget) = updates.token_budget {
        config.token_budget = token_budget;
    }
    if let Some(kind) = updates.checkpoint_type {
        config.checkpoint_type = kind;
    }
    if let Some(mode) = updates.proof_mode {
        config.proof_mode = mode;
    }

    tx.execute(
        "UPDATE run_checkpoints SET model = ?1, prompt = ?2, token_budget = ?3, checkpoint_type = ?4, proof_mode = ?5, updated_at = CURRENT_TIMESTAMP WHERE id = ?6",
        params![
            &config.model,
            &config.prompt,
            (config.token_budget as i64),
            &config.checkpoint_type,
            config.proof_mode.as_str(),
            &checkpoint_id,
        ],
    )?;

    tx.commit()?;
    Ok(config)
}

#[tauri::command]
pub fn delete_checkpoint_config(checkpoint_id: String, pool: State<DbPool>) -> Result<(), Error> {
    let mut conn = pool.get()?;
    let tx = conn.transaction()?;

    let row: Option<(String, i64)> = tx
        .query_row(
            "SELECT run_id, order_index FROM run_checkpoints WHERE id = ?1",
            params![&checkpoint_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;

    let (run_id, order_index) =
        row.ok_or_else(|| Error::Api(format!("checkpoint config {checkpoint_id} not found")))?;

    tx.execute(
        "DELETE FROM run_checkpoints WHERE id = ?1",
        params![&checkpoint_id],
    )?;
    tx.execute(
        "UPDATE run_checkpoints SET order_index = order_index - 1, updated_at = CURRENT_TIMESTAMP WHERE run_id = ?1 AND order_index > ?2",
        params![&run_id, order_index],
    )?;

    tx.commit()?;
    Ok(())
}

#[tauri::command]
pub fn reorder_checkpoint_configs(
    run_id: String,
    checkpoint_ids: Vec<String>,
    pool: State<DbPool>,
) -> Result<Vec<orchestrator::RunCheckpointConfig>, Error> {
    reorder_checkpoint_configs_with_pool(run_id, checkpoint_ids, pool.inner())
}

pub(crate) fn reorder_checkpoint_configs_with_pool(
    run_id: String,
    checkpoint_ids: Vec<String>,
    pool: &DbPool,
) -> Result<Vec<orchestrator::RunCheckpointConfig>, Error> {
    {
        let mut conn = pool.get()?;
        let tx = conn.transaction()?;

        let existing: Vec<(String, i64)> = {
            let mut existing_stmt = tx.prepare(
                "SELECT id, order_index FROM run_checkpoints WHERE run_id = ?1 ORDER BY order_index ASC",
            )?;
            let existing_rows = existing_stmt.query_map(params![&run_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?;
            let mut existing = Vec::new();
            for row in existing_rows {
                existing.push(row?);
            }
            existing
        };

        if existing.len() != checkpoint_ids.len() {
            return Err(Error::Api(
                "reorder list must include all checkpoint ids".to_string(),
            ));
        }

        let existing_set: HashSet<_> = existing.iter().map(|(id, _)| id.clone()).collect();
        let provided_set: HashSet<_> = checkpoint_ids.iter().cloned().collect();
        if existing_set != provided_set {
            return Err(Error::Api(
                "reorder list does not match stored checkpoint identifiers".to_string(),
            ));
        }

        let temporary_offset = existing
            .iter()
            .map(|(_, order_index)| *order_index)
            .max()
            .unwrap_or(-1)
            + 1;

        for (index, checkpoint_id) in checkpoint_ids.iter().enumerate() {
            tx.execute(
                "UPDATE run_checkpoints SET order_index = ?1 WHERE id = ?2",
                params![temporary_offset + index as i64, checkpoint_id],
            )?;
        }

        for (index, checkpoint_id) in checkpoint_ids.iter().enumerate() {
            tx.execute(
                "UPDATE run_checkpoints SET order_index = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                params![index as i64, checkpoint_id],
            )?;
        }

        tx.commit()?;
    }

    list_run_checkpoint_configs_with_pool(run_id, pool)
}

#[tauri::command]
pub fn start_run(run_id: String, pool: State<DbPool>) -> Result<(), Error> {
    orchestrator::start_run(pool.inner(), &run_id).map_err(|err| Error::Api(err.to_string()))
}

#[tauri::command]
pub fn reopen_run(run_id: String, pool: State<DbPool>) -> Result<(), Error> {
    orchestrator::reopen_run(pool.inner(), &run_id).map_err(|err| Error::Api(err.to_string()))
}

#[tauri::command]
pub fn clone_run(run_id: String, pool: State<DbPool>) -> Result<String, Error> {
    orchestrator::clone_run(pool.inner(), &run_id).map_err(|err| Error::Api(err.to_string()))
}

#[tauri::command]
pub fn estimate_run_cost(
    run_id: String,
    pool: State<DbPool>,
) -> Result<orchestrator::RunCostEstimates, Error> {
    let conn = pool.get()?;
    orchestrator::estimate_run_cost(conn.deref(), &run_id)
        .map_err(|err| Error::Api(err.to_string()))
}

#[tauri::command]
pub fn update_policy(project_id: String, policy: Policy, pool: State<DbPool>) -> Result<(), Error> {
    let conn = pool.get()?;
    store::policies::upsert(&conn, &project_id, &policy)
}

// --- MERGED AND FIXED emit_car FUNCTIONALITY ---
pub(crate) fn emit_car_to_base_dir(
    run_id: &str,
    pool: &DbPool,
    base_dir: &Path,
) -> Result<PathBuf, Error> {
    let conn = pool.get()?;
    let project_id: String = conn
        .query_row(
            "SELECT project_id FROM runs WHERE id = ?1",
            params![run_id],
            |row| row.get(0),
        )
        .map_err(|err| match err {
            rusqlite::Error::QueryReturnedNoRows => Error::Api(format!("run {run_id} not found")),
            other => Error::from(other),
        })?;

    let car = car::build_car(&conn, run_id).map_err(|err| Error::Api(err.to_string()))?;

    let receipts_dir = base_dir.join(&project_id).join("receipts");
    std::fs::create_dir_all(&receipts_dir)
        .map_err(|err| Error::Api(format!("failed to create receipts dir: {err}")))?;

    let file_path = receipts_dir.join(format!("{}.car.json", car.id));
    let json = serde_json::to_string_pretty(&car)
        .map_err(|err| Error::Api(format!("failed to serialize CAR: {err}")))?;
    std::fs::write(&file_path, json)
        .map_err(|err| Error::Api(format!("failed to write CAR file: {err}")))?;

    let created_at = car.created_at.to_rfc3339();
    let file_path_str = file_path.to_string_lossy().to_string();

    conn.execute(
        "INSERT INTO receipts (id, run_id, created_at, file_path, match_kind, epsilon, s_grade) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            &car.id,
            run_id,
            &created_at,
            &file_path_str,
            &car.proof.match_kind,
            car.proof.epsilon,
            i64::from(car.sgrade.score),
        ],
    )?;

    Ok(file_path)
}

#[tauri::command]
pub fn emit_car(
    run_id: String,
    pool: State<DbPool>,
    app_handle: AppHandle,
) -> Result<String, Error> {
    let base_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|err| Error::Api(format!("failed to resolve app data dir: {err}")))?;
    let path = emit_car_to_base_dir(&run_id, pool.inner(), &base_dir)?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn export_project(
    project_id: String,
    pool: State<DbPool>,
    app_handle: AppHandle,
) -> Result<String, Error> {
    let base_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|err| Error::Api(format!("failed to resolve app data dir: {err}")))?;
    let path = portability::export_project_archive(pool.inner(), &project_id, &base_dir)?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn import_project(
    args: ImportProjectArgs,
    pool: State<DbPool>,
    app_handle: AppHandle,
) -> Result<portability::ProjectImportSummary, Error> {
    let ImportProjectArgs {
        archive_path,
        file_name,
        bytes,
    } = args;

    let base_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|err| Error::Api(format!("failed to resolve app data dir: {err}")))?;

    if let Some(path) = archive_path {
        let path = PathBuf::from(path);
        return portability::import_project_archive(pool.inner(), &path, &base_dir);
    }

    let bytes = bytes.ok_or_else(|| Error::Api("No project archive provided.".into()))?;
    let temp_path =
        persist_uploaded_bytes(&base_dir, "imports", file_name.as_deref(), &bytes, "ixp")?;

    let result = portability::import_project_archive(pool.inner(), &temp_path, &base_dir);
    if let Err(err) = fs::remove_file(&temp_path) {
        eprintln!(
            "failed to remove temporary project archive {}: {err}",
            temp_path.display()
        );
    }
    result
}

#[tauri::command]
pub fn import_car(
    args: ImportCarArgs,
    pool: State<DbPool>,
    app_handle: AppHandle,
) -> Result<replay::ReplayReport, Error> {
    let ImportCarArgs {
        car_path,
        file_name,
        bytes,
    } = args;

    let base_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|err| Error::Api(format!("failed to resolve app data dir: {err}")))?;

    if let Some(path) = car_path {
        let path = PathBuf::from(path);
        return portability::import_car_file(pool.inner(), &path, &base_dir);
    }

    let bytes = bytes.ok_or_else(|| Error::Api("No CAR data provided.".into()))?;
    let temp_path = persist_uploaded_bytes(
        &base_dir,
        "imports",
        file_name.as_deref(),
        &bytes,
        "car.json",
    )?;

    let result = portability::import_car_file(pool.inner(), &temp_path, &base_dir);
    if let Err(err) = fs::remove_file(&temp_path) {
        eprintln!(
            "failed to remove temporary CAR file {}: {err}",
            temp_path.display()
        );
    }
    result
}

fn persist_uploaded_bytes(
    base_dir: &Path,
    subdir: &str,
    suggested_name: Option<&str>,
    bytes: &[u8],
    fallback_ext: &str,
) -> Result<PathBuf, Error> {
    let import_dir = base_dir.join(subdir);
    fs::create_dir_all(&import_dir).map_err(|err| {
        Error::Api(format!(
            "failed to create {subdir} directory {}: {err}",
            import_dir.display()
        ))
    })?;

    let sanitized = suggested_name
        .map(|name| sanitize_file_name(name, fallback_ext))
        .unwrap_or_else(|| sanitize_file_name("", fallback_ext));
    let unique_name = format!("{}-{}", Uuid::new_v4(), sanitized);
    let temp_path = import_dir.join(unique_name);

    fs::write(&temp_path, bytes).map_err(|err| {
        Error::Api(format!(
            "failed to persist uploaded file {}: {err}",
            temp_path.display()
        ))
    })?;

    Ok(temp_path)
}

fn sanitize_file_name(name: &str, fallback_ext: &str) -> String {
    let mut cleaned: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        .collect();

    if cleaned.len() > 64 {
        cleaned.truncate(64);
    }

    let trimmed = cleaned.trim_matches('.');
    let mut sanitized = if trimmed.is_empty() {
        String::new()
    } else {
        trimmed.to_string()
    };

    if !sanitized.chars().any(|c| c.is_ascii_alphanumeric()) {
        sanitized.clear();
    }

    if sanitized.is_empty() {
        return fallback_file_name(fallback_ext);
    }

    if !sanitized.contains('.') {
        if fallback_ext.starts_with('.') {
            sanitized.push_str(fallback_ext);
        } else {
            sanitized.push('.');
            sanitized.push_str(fallback_ext);
        }
    }

    sanitized
}

fn fallback_file_name(fallback_ext: &str) -> String {
    if fallback_ext.starts_with('.') {
        format!("upload{}", fallback_ext)
    } else {
        format!("upload.{fallback_ext}")
    }
}
