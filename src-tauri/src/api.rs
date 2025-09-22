// In src-tauri/src/api.rs
use crate::{
    car, orchestrator, provenance, replay,
    store::{self, policies::Policy},
    DbPool, Error, Project,
};
use rusqlite::{params, types::Type, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckpointConfigRequest {
    pub model: Option<String>,
    pub prompt: Option<String>,
    pub token_budget: Option<u64>,
    pub checkpoint_type: Option<String>,
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
}

#[tauri::command]
pub fn list_runs(project_id: String, pool: State<DbPool>) -> Result<Vec<RunSummary>, Error> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT id, name, created_at, kind FROM runs WHERE project_id = ?1 ORDER BY created_at DESC",
    )?;
    let runs_iter = stmt.query_map(params![project_id], |row| {
        Ok(RunSummary {
            id: row.get(0)?,
            name: row.get(1)?,
            created_at: row.get(2)?,
            kind: row.get(3)?,
        })
    })?;
    let mut runs = Vec::new();
    for run in runs_iter {
        runs.push(run?);
    }
    Ok(runs)
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
    let row: Option<(String, i64, String, String, String, i64)> = conn
        .query_row(
            "SELECT run_id, order_index, checkpoint_type, model, prompt, token_budget FROM run_checkpoints WHERE id = ?1",
            params![checkpoint_id],
            |row| Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            )),
        )
        .optional()?;

    let (run_id, order_index, checkpoint_type, model, prompt, token_budget_raw) =
        row.ok_or_else(|| Error::Api(format!("checkpoint config {checkpoint_id} not found")))?;

    Ok(orchestrator::RunCheckpointConfig {
        id: checkpoint_id.to_string(),
        run_id,
        order_index,
        checkpoint_type,
        model,
        prompt,
        token_budget: token_budget_raw.max(0) as u64,
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
    let kind_opt: Option<String> = conn
        .query_row(
            "SELECT kind FROM runs WHERE id = ?1",
            params![&run_id],
            |row| row.get(0),
        )
        .optional()?;

    let has_interactive_config: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM run_checkpoints WHERE run_id = ?1 AND LOWER(checkpoint_type) = 'interactivechat')",
        params![&run_id],
        |row| {
            let value: i64 = row.get(0)?;
            Ok(value != 0)
        },
    )?;

    let has_interactive_turns: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM checkpoints WHERE run_id = ?1 AND turn_index IS NOT NULL)",
        params![&run_id],
        |row| {
            let value: i64 = row.get(0)?;
            Ok(value != 0)
        },
    )?;

    let has_interactive = has_interactive_config || has_interactive_turns;

    let report = if has_interactive {
        replay::replay_interactive_run(run_id.clone(), pool)
    } else {
        match kind_opt.as_deref() {
            Some("exact") => replay::replay_exact_run(run_id.clone(), pool),
            Some("concordant") => replay::replay_concordant_run(run_id.clone(), pool),
            Some(other) => Err(anyhow::anyhow!(
                "Replay not implemented for run kind: '{}'",
                other
            )),
            None => Ok(replay::ReplayReport {
                run_id: run_id.clone(),
                match_status: false,
                original_digest: String::new(),
                replay_digest: String::new(),
                error_message: Some("run not found".to_string()),
                semantic_original_digest: None,
                semantic_replay_digest: None,
                semantic_distance: None,
                epsilon: None,
            }),
        }
    }
    .map_err(|err| Error::Api(err.to_string()))?;

    Ok(report)
}

#[tauri::command]
pub fn list_run_checkpoint_configs(
    run_id: String,
    pool: State<DbPool>,
) -> Result<Vec<orchestrator::RunCheckpointConfig>, Error> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT id, run_id, order_index, checkpoint_type, model, prompt, token_budget FROM run_checkpoints WHERE run_id = ?1 ORDER BY order_index ASC",
    )?;
    let rows = stmt.query_map(params![&run_id], |row| {
        let token_budget: i64 = row.get(6)?;
        Ok(orchestrator::RunCheckpointConfig {
            id: row.get(0)?,
            run_id: row.get(1)?,
            order_index: row.get(2)?,
            checkpoint_type: row.get(3)?,
            model: row.get(4)?,
            prompt: row.get(5)?,
            token_budget: token_budget.max(0) as u64,
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
        ..
    } = config;
    tx.execute(
        "INSERT INTO run_checkpoints (id, run_id, order_index, checkpoint_type, model, prompt, token_budget) VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![
            &checkpoint_id,
            &run_id,
            order_index,
            &checkpoint_type,
            &model,
            &prompt,
            (token_budget as i64),
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

    tx.execute(
        "UPDATE run_checkpoints SET model = ?1, prompt = ?2, token_budget = ?3, checkpoint_type = ?4, updated_at = CURRENT_TIMESTAMP WHERE id = ?5",
        params![
            &config.model,
            &config.prompt,
            (config.token_budget as i64),
            &config.checkpoint_type,
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
    let mut conn = pool.get()?;
    let tx = conn.transaction()?;

    let existing: Vec<String> = {
        let mut existing_stmt = tx
            .prepare("SELECT id FROM run_checkpoints WHERE run_id = ?1 ORDER BY order_index ASC")?;
        let existing_rows =
            existing_stmt.query_map(params![&run_id], |row| row.get::<_, String>(0))?;
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

    let existing_set: HashSet<_> = existing.iter().cloned().collect();
    let provided_set: HashSet<_> = checkpoint_ids.iter().cloned().collect();
    if existing_set != provided_set {
        return Err(Error::Api(
            "reorder list does not match stored checkpoint identifiers".to_string(),
        ));
    }

    for (index, checkpoint_id) in checkpoint_ids.iter().enumerate() {
        tx.execute(
            "UPDATE run_checkpoints SET order_index = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params![index as i64, checkpoint_id],
        )?;
    }

    tx.commit()?;

    list_run_checkpoint_configs(run_id, pool)
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
