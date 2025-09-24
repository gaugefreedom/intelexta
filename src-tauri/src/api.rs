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
    // Note: proof_mode and epsilon are no longer needed here,
    // as they are defined on a per-step basis.
    seed: u64,
    token_budget: u64,
    default_model: String,
    pool: State<DbPool>,
) -> Result<String, Error> {
    // We create an empty run. Steps will be added separately by the UI.
    let initial_steps = Vec::new();

    orchestrator::create_run(
        pool.inner(),
        &project_id,
        &name,
        // Since proof mode is per-step, we can use a default for the run itself.
        orchestrator::RunProofMode::Exact,
        None, // Epsilon is also per-step.
        seed,
        token_budget,
        &default_model,
        initial_steps,
    )
    .map_err(|err| Error::Api(err.to_string()))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunStepRequest {
    pub model: String,
    pub prompt: String,
    pub token_budget: u64,
    #[serde(default)]
    pub checkpoint_type: Option<String>,
    #[serde(default)]
    pub order_index: Option<i64>,
    #[serde(default)]
    pub proof_mode: orchestrator::RunProofMode,
    #[serde(default)]
    pub epsilon: Option<f64>,
}

#[tauri::command]
pub fn create_run_step(
    run_id: String,
    config: RunStepRequest,
    pool: State<DbPool>,
) -> Result<orchestrator::RunStep, Error> {
    orchestrator::create_run_step(pool.inner(), &run_id, config)
        .map_err(|err| Error::Api(err.to_string()))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRunStepRequest {
    pub model: Option<String>,
    pub prompt: Option<String>,
    pub token_budget: Option<u64>,
    pub checkpoint_type: Option<String>,
    pub proof_mode: Option<orchestrator::RunProofMode>,
    pub epsilon: Option<f64>,
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


#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionStepProofSummary {
    pub checkpoint_config_id: String,
    pub checkpoint_type: String,
    pub order_index: i64,
    pub proof_mode: orchestrator::RunProofMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epsilon: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunExecutionSummary {
    pub id: String,
    pub created_at: String,
    #[serde(default)]
    pub step_proofs: Vec<ExecutionStepProofSummary>,
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
    #[serde(default)]
    pub executions: Vec<RunExecutionSummary>,
    #[serde(default)]
    pub step_proofs: Vec<ExecutionStepProofSummary>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListCheckpointsArgs {
    run_execution_id: Option<String>,
}

fn hydrate_run_summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<RunSummary> {
    // This function now reads simple, individual columns directly from the database.
    // It no longer needs to parse a complex JSON blob.
    Ok(RunSummary {
        id: row.get(0)?,
        name: row.get(1)?,
        created_at: row.get(2)?,
        // The 'kind' is determined later by inspecting the steps.
        kind: String::new(), 
        // Epsilon is now per-step, so it's not needed here.
        epsilon: None,
        // has_persisted_checkpoint is the new name for the column at index 3.
        has_persisted_checkpoint: row.get(3)?,
        executions: Vec::new(),
        step_proofs: Vec::new(),
    })
}

fn load_step_proof_summaries(
    conn: &Connection,
    run_id: &str,
) -> Result<Vec<ExecutionStepProofSummary>, Error> {
    let mut stmt = conn.prepare(
        "SELECT id, checkpoint_type, order_index, proof_mode, epsilon FROM run_steps WHERE run_id = ?1 ORDER BY order_index ASC",
    )?;

    let rows = stmt.query_map(params![run_id], |row| {
        let proof_mode_raw: String = row.get(3)?;
        let proof_mode =
            orchestrator::RunProofMode::try_from(proof_mode_raw.as_str()).map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(3, Type::Text, Box::new(err))
            })?;
        Ok(ExecutionStepProofSummary {
            checkpoint_config_id: row.get(0)?,
            checkpoint_type: row.get(1)?,
            order_index: row.get(2)?,
            proof_mode,
            epsilon: row.get(4)?,
        })
    })?;

    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }

    Ok(entries)
}

#[tauri::command]
pub fn list_runs(project_id: String, pool: State<DbPool>) -> Result<Vec<RunSummary>, Error> {
    let conn = pool.get()?;
    // This SQL query is now simpler and no longer selects the obsolete spec_json.
    let mut stmt = conn.prepare(
        "SELECT r.id, r.name, r.created_at, EXISTS (SELECT 1 FROM run_executions e WHERE e.run_id = r.id) AS has_persisted_checkpoint FROM runs r WHERE r.project_id = ?1 ORDER BY r.created_at DESC",
    )?;
    
    let runs_iter = stmt.query_map(params![project_id], hydrate_run_summary)?;
    let mut runs = Vec::new();

    for run in runs_iter {
        let mut summary = run?;
        
        // Load the configured steps for this run.
        let step_proofs = load_step_proof_summaries(&conn, &summary.id)?;

        // Determine the overall 'kind' of the run by checking if any of its steps are concordant.
        let has_concordant_step = step_proofs
            .iter()
            .any(|template| template.proof_mode.is_concordant());
        summary.kind = if has_concordant_step {
            "concordant".to_string()
        } else {
            "exact".to_string()
        };
        summary.step_proofs = step_proofs.clone();

        // Load all the execution records for this run.
        let executions = orchestrator::list_run_executions(&conn, &summary.id)
            .map_err(|err| Error::Api(err.to_string()))?;
        summary.executions = executions
            .into_iter()
            .map(|record| RunExecutionSummary {
                id: record.id,
                created_at: record.created_at,
                step_proofs: step_proofs.clone(),
            })
            .collect();

        runs.push(summary);
    }
    Ok(runs)
}

fn load_run_summary(conn: &Connection, run_id: &str) -> Result<RunSummary, Error> {
    let summary = conn
        .query_row(
            "SELECT r.id, r.name, r.created_at, r.spec_json, EXISTS (SELECT 1 FROM run_executions e WHERE e.run_id = r.id) AS has_persisted_checkpoint FROM runs r WHERE r.id = ?1",
            params![run_id],
            hydrate_run_summary,
        )
        .optional()?;

    let mut summary = summary.ok_or_else(|| Error::Api(format!("run {run_id} not found")))?;
    let step_proofs = load_step_proof_summaries(conn, &summary.id)?;
    summary.step_proofs = step_proofs.clone();

    let executions = orchestrator::list_run_executions(conn, &summary.id)
        .map_err(|err| Error::Api(err.to_string()))?;
    summary.executions = executions
        .into_iter()
        .map(|record| RunExecutionSummary {
            id: record.id,
            created_at: record.created_at,
            step_proofs: step_proofs.clone(),
        })
        .collect();
    if !summary.executions.is_empty() {
        summary.has_persisted_checkpoint = true;
    }

    Ok(summary)
}


#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointSummary {
    pub id: String,
    pub run_execution_id: String,
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
    pub run_execution_id: String,
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
    pub checkpoint: orchestrator::RunStep,
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

// In src-tauri/src/api.rs

#[tauri::command]
pub fn list_checkpoints(
    args: ListCheckpointsArgs,
    pool: State<DbPool>,
) -> Result<Vec<CheckpointSummary>, Error> {
    // 1. Get the execution_id from the arguments first.
    let Some(execution_id) = args.run_execution_id else {
        // If there's no ID, we can return an empty list right away.
        return Ok(Vec::new());
    };

    // 2. Call the database and store the result.
    let result = list_checkpoints_with_pool(Some(execution_id.as_str()), pool.inner());

    // 3. Use the `match` block as the final expression to handle the result.
    match result {
        Ok(checkpoints) => Ok(checkpoints),
        Err(err) => {
            // This converts the complex Rust error into a simple string
            // that can be sent to the frontend.
            Err(Error::Api(err.to_string()))
        }
    }
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
    let config = load_run_step(&conn, &checkpoint_id)?;

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

    // First, get the latest execution record for the run.
    let latest_execution = orchestrator::load_latest_run_execution(&conn, &run_id)
    .map_err(|err| Error::Api(err.to_string()))?
    .ok_or_else(|| Error::Api(format!("Run {} has no executions", run_id)))?;

    // Now, call the helper with the specific execution ID.
    let mut messages = list_checkpoints_with_pool(Some(latest_execution.id.as_str()), pool.inner())?;
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
    run_execution_id: Option<&str>,
    pool: &DbPool,
) -> Result<Vec<CheckpointSummary>, Error> {
    // 1. The logic is simpler: if no execution ID is provided, there's nothing to load.
    let Some(execution_id) = run_execution_id else {
        return Ok(Vec::new());
    };

    let conn = pool.get()?;
    
    // 2. The SQL query is corrected to filter ONLY by run_execution_id.
    let mut stmt = conn.prepare(
        "SELECT c.id, c.run_execution_id, c.timestamp, c.kind, c.incident_json, c.inputs_sha256, c.outputs_sha256, c.semantic_digest, c.usage_tokens, c.prompt_tokens, c.completion_tokens, c.parent_checkpoint_id, c.turn_index, c.checkpoint_config_id, m.role, m.body, m.created_at, m.updated_at
         FROM checkpoints c
         LEFT JOIN checkpoint_messages m ON m.checkpoint_id = c.id
         WHERE c.run_execution_id = ?1
         ORDER BY c.timestamp ASC",
    )?;

    // 3. The `params!` macro is updated to match the simplified query.
    let rows = stmt.query_map(params![execution_id], |row| {
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
        let message_role: Option<String> = row.get(14)?;
        let message_body: Option<String> = row.get(15)?;
        let message_created_at: Option<String> = row.get(16)?;
        let message_updated_at: Option<String> = row.get(17)?;
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
            run_execution_id: row.get(1)?,
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
            message,
        })
    })?;
    
    let mut checkpoints = Vec::new();
    for row in rows {
        checkpoints.push(row?);
    }
    Ok(checkpoints)
}

// In src-tauri/src/api.rs

pub(crate) fn get_checkpoint_details_with_pool(
    checkpoint_id: String,
    pool: &DbPool,
) -> Result<CheckpointDetails, Error> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT c.id, c.run_id, c.run_execution_id, c.timestamp, c.kind, c.incident_json, c.inputs_sha256, c.outputs_sha256, c.semantic_digest, c.usage_tokens, c.prompt_tokens, c.completion_tokens, c.parent_checkpoint_id, c.turn_index, c.checkpoint_config_id, p.prompt_payload, p.output_payload, m.role, m.body, m.created_at, m.updated_at
         FROM checkpoints c
         LEFT JOIN checkpoint_payloads p ON p.checkpoint_id = c.id
         LEFT JOIN checkpoint_messages m ON m.checkpoint_id = c.id
         WHERE c.id = ?1",
    )?;

    let result = stmt.query_row(params![checkpoint_id], |row| {
        let incident_json: Option<String> = row.get(5)?; // Index 5 for incident_json
        
        // --- START OF FIX ---
        // This logic now safely handles empty or null JSON strings.
        let incident = incident_json
            .and_then(|payload| {
                if payload.is_empty() {
                    None
                } else {
                    Some(serde_json::from_str::<IncidentSummary>(&payload))
                }
            })
            .transpose()
            .map_err(|err| rusqlite::Error::FromSqlConversionFailure(5, Type::Text, Box::new(err)))?;
        // --- END OF FIX ---

        let parent_checkpoint_id: Option<String> = row.get(12)?;
        let turn_index = row.get::<_, Option<i64>>(13)?.map(|value| value.max(0) as u32);
        let checkpoint_config_id: Option<String> = row.get(14)?;
        let prompt_payload: Option<String> = row.get(15)?;
        let output_payload: Option<String> = row.get(16)?;
        let message_role: Option<String> = row.get(17)?;
        let message_body: Option<String> = row.get(18)?;
        let message_created_at: Option<String> = row.get(19)?;
        let message_updated_at: Option<String> = row.get(20)?;
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
            run_execution_id: row.get(2)?,
            timestamp: row.get(3)?,
            kind: row.get(4)?,
            incident,
            inputs_sha256: row.get(6)?,
            outputs_sha256: row.get(7)?,
            semantic_digest: row.get(8)?,
            usage_tokens: { let value: i64 = row.get(9)?; value.max(0) as u64 },
            prompt_tokens: { let value: i64 = row.get(10)?; value.max(0) as u64 },
            completion_tokens: { let value: i64 = row.get(11)?; value.max(0) as u64 },
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

fn load_run_step(conn: &Connection, checkpoint_id: &str) -> Result<orchestrator::RunStep, Error> {
    let row: Option<(String, i64, String, String, String, i64, String, Option<f64>)> = conn
        .query_row(
            "SELECT run_id, order_index, checkpoint_type, model, prompt, token_budget, proof_mode, epsilon FROM run_steps WHERE id = ?1",
            params![checkpoint_id],
            |row| Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
            )),
        )
        .optional()?;

    let (
        run_id,
        order_index,
        checkpoint_type,
        model,
        prompt,
        token_budget_raw,
        proof_mode_raw,
        epsilon,
    ) = row.ok_or_else(|| Error::Api(format!("checkpoint config {checkpoint_id} not found")))?;

    let proof_mode =
        orchestrator::RunProofMode::try_from(proof_mode_raw.as_str()).map_err(|err| {
            Error::from(rusqlite::Error::FromSqlConversionFailure(
                6,
                rusqlite::types::Type::Text,
                Box::new(err),
            ))
        })?;

    Ok(orchestrator::RunStep {
        id: checkpoint_id.to_string(),
        run_id,
        order_index,
        checkpoint_type,
        model,
        prompt,
        token_budget: token_budget_raw.max(0) as u64,
        proof_mode,
        epsilon,
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

    if stored_run.steps.is_empty() {
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
    let has_interactive_configs = stored_run.steps.iter().any(|cfg| cfg.is_interactive_chat());

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
    if stored_run.steps.iter().any(|cfg| cfg.is_interactive_chat()) {
        return Err(Error::Api(
            "Interactive replays are disabled in this build.".to_string(),
        ));
    }

    for config in &stored_run.steps {
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
                        proof_mode: Some(config.proof_mode),
                        semantic_original_digest: None,
                        semantic_replay_digest: None,
                        semantic_distance: None,
                        epsilon: None,
                        configured_epsilon: config.epsilon,
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
pub fn list_run_steps(
    run_id: String,
    pool: State<DbPool>,
) -> Result<Vec<orchestrator::RunStep>, Error> {
    list_run_steps_with_pool(run_id, pool.inner())
}

pub(crate) fn list_run_steps_with_pool(
    run_id: String,
    pool: &DbPool,
) -> Result<Vec<orchestrator::RunStep>, Error> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT id, run_id, order_index, checkpoint_type, model, prompt, token_budget, proof_mode, epsilon FROM run_steps WHERE run_id = ?1 ORDER BY order_index ASC",
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
        Ok(orchestrator::RunStep {
            id: row.get(0)?,
            run_id: row.get(1)?,
            order_index: row.get(2)?,
            checkpoint_type: row.get(3)?,
            model: row.get(4)?,
            prompt: row.get(5)?,
            token_budget: token_budget.max(0) as u64,
            proof_mode,
            epsilon: row.get(8)?,
        })
    })?;

    let mut configs = Vec::new();
    for row in rows {
        configs.push(row?);
    }

    Ok(configs)
}

#[tauri::command]
pub fn update_run_step(
    checkpoint_id: String,
    updates: UpdateRunStepRequest,
    pool: State<DbPool>,
) -> Result<orchestrator::RunStep, Error> {
    let mut conn = pool.get()?;
    let tx = conn.transaction()?;
    let mut config = load_run_step(&tx, &checkpoint_id)?;

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
    if let Some(epsilon) = updates.epsilon {
        config.epsilon = Some(epsilon);
    }
    if config.proof_mode.is_concordant() {
        let value = config
            .epsilon
            .ok_or_else(|| Error::Api("concordant steps require an epsilon".to_string()))?;
        if !value.is_finite() || value < 0.0 {
            return Err(Error::Api(
                "epsilon must be a finite, non-negative value".to_string(),
            ));
        }
        config.epsilon = Some(value);
    } else {
        config.epsilon = None;
    }

    tx.execute(
        "UPDATE run_steps SET model = ?1, prompt = ?2, token_budget = ?3, checkpoint_type = ?4, proof_mode = ?5, epsilon = ?6, updated_at = CURRENT_TIMESTAMP WHERE id = ?7",
        params![
            &config.model,
            &config.prompt,
            (config.token_budget as i64),
            &config.checkpoint_type,
            config.proof_mode.as_str(),
            config.epsilon,
            &checkpoint_id,
        ],
    )?;

    tx.commit()?;
    Ok(config)
}

#[tauri::command]
pub fn delete_run_step(checkpoint_id: String, pool: State<DbPool>) -> Result<(), Error> {
    let mut conn = pool.get()?;
    let tx = conn.transaction()?;

    let row: Option<(String, i64)> = tx
        .query_row(
            "SELECT run_id, order_index FROM run_steps WHERE id = ?1",
            params![&checkpoint_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;

    let (run_id, order_index) =
        row.ok_or_else(|| Error::Api(format!("checkpoint config {checkpoint_id} not found")))?;

    tx.execute(
        "DELETE FROM run_steps WHERE id = ?1",
        params![&checkpoint_id],
    )?;
    tx.execute(
        "UPDATE run_steps SET order_index = order_index - 1, updated_at = CURRENT_TIMESTAMP WHERE run_id = ?1 AND order_index > ?2",
        params![&run_id, order_index],
    )?;

    tx.commit()?;
    Ok(())
}

#[tauri::command]
pub fn reorder_run_steps(
    run_id: String,
    checkpoint_ids: Vec<String>,
    pool: State<DbPool>,
) -> Result<Vec<orchestrator::RunStep>, Error> {
    reorder_run_steps_with_pool(run_id, checkpoint_ids, pool.inner())
}

pub(crate) fn reorder_run_steps_with_pool(
    run_id: String,
    checkpoint_ids: Vec<String>,
    pool: &DbPool,
) -> Result<Vec<orchestrator::RunStep>, Error> {
    {
        let mut conn = pool.get()?;
        let tx = conn.transaction()?;

        let existing: Vec<(String, i64)> = {
            let mut existing_stmt = tx.prepare(
                "SELECT id, order_index FROM run_steps WHERE run_id = ?1 ORDER BY order_index ASC",
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
                "UPDATE run_steps SET order_index = ?1 WHERE id = ?2",
                params![temporary_offset + index as i64, checkpoint_id],
            )?;
        }

        for (index, checkpoint_id) in checkpoint_ids.iter().enumerate() {
            tx.execute(
                "UPDATE run_steps SET order_index = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                params![index as i64, checkpoint_id],
            )?;
        }

        tx.commit()?;
    }

    list_run_steps_with_pool(run_id, pool)
}

#[tauri::command]
pub fn start_run(run_id: String, pool: State<DbPool>) -> Result<RunExecutionSummary, Error> {
    let record = orchestrator::start_run(pool.inner(), &run_id)
        .map_err(|err| Error::Api(err.to_string()))?;

    let conn = pool.get()?;
    let step_proofs = load_step_proof_summaries(&conn, &run_id)?;

    Ok(RunExecutionSummary {
        id: record.id,
        created_at: record.created_at,
        step_proofs,
    })
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

    let file_path = receipts_dir.join(format!("{}.car.json", car.id.replace(':', "_")));
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
