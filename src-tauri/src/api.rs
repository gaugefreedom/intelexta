// In src-tauri/src/api.rs
use crate::{
    car, orchestrator, provenance,
    store::{self, policies::Policy},
    DbPool, Error, Project,
};
use chrono::Utc;
use rusqlite::{params, types::Type};
use serde::{Deserialize, Serialize};
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
pub fn create_project(name: String, pool: State<DbPool>) -> Result<Project, Error> {
    create_project_with_pool(name, pool.inner())
}

pub(crate) fn create_project_with_pool(name: String, pool: &DbPool) -> Result<Project, Error> {
    let project_id = Uuid::new_v4().to_string();
    let kp = provenance::generate_keypair();
    provenance::store_secret_key(&project_id, &kp.secret_key_b64)
        .map_err(|e| Error::Api(e.to_string()))?;
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
}

#[tauri::command]
pub fn start_hello_run(spec: HelloRunSpec, pool: State<DbPool>) -> Result<String, Error> {
    let rs = orchestrator::RunSpec {
        project_id: spec.project_id,
        name: spec.name,
        seed: spec.seed,
        dag_json: spec.dag_json,
        token_budget: spec.token_budget,
    };
    orchestrator::start_hello_run(&pool, rs).map_err(|e| Error::Api(e.to_string()))
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

#[tauri::command]
pub fn list_checkpoints(
    run_id: String,
    pool: State<DbPool>,
) -> Result<Vec<CheckpointSummary>, Error> {
    list_checkpoints_with_pool(run_id, pool.inner())
}

pub(crate) fn list_checkpoints_with_pool(
    run_id: String,
    pool: &DbPool,
) -> Result<Vec<CheckpointSummary>, Error> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT id, timestamp, kind, incident_json, inputs_sha256, outputs_sha256, semantic_digest, usage_tokens FROM checkpoints WHERE run_id = ?1 ORDER BY timestamp ASC",
    )?;
    let rows = stmt.query_map(params![run_id], |row| {
        let incident_json: Option<String> = row.get(3)?;
        let incident = incident_json
            .map(|payload| serde_json::from_str::<IncidentSummary>(&payload))
            .transpose()
            .map_err(|err| rusqlite::Error::FromSqlConversionFailure(3, Type::Text, Box::new(err)))?;
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
        })
    })?;
    let mut checkpoints = Vec::new();
    for row in rows {
        checkpoints.push(row?);
    }
    Ok(checkpoints)
}

#[tauri::command]
pub fn get_policy(project_id: String, pool: State<DbPool>) -> Result<Policy, Error> {
    let conn = pool.get()?;
    store::policies::get(&conn, &project_id)
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
    let project_id: String = conn.query_row(
        "SELECT project_id FROM runs WHERE id = ?1",
        params![run_id],
        |row| row.get(0),
    ).map_err(|err| match err {
        rusqlite::Error::QueryReturnedNoRows => Error::Api(format!("run {run_id} not found")),
        other => Error::from(other),
    })?;

    // NOTE: This is still a placeholder builder until Sprint 1B
    let car = car::build_car(run_id).map_err(|err| Error::Api(err.to_string()))?;
    
    // **FIX FOR [P1]**: Generate a unique ID for the receipt to prevent DB constraint errors.
    let receipt_id = Uuid::new_v4().to_string();

    let receipts_dir = base_dir.join(&project_id).join("receipts");
    std::fs::create_dir_all(&receipts_dir)
        .map_err(|err| Error::Api(format!("failed to create receipts dir: {err}")))?;

    let file_path = receipts_dir.join(format!("{receipt_id}.car.json"));
    let json = serde_json::to_string_pretty(&car)
        .map_err(|err| Error::Api(format!("failed to serialize CAR: {err}")))?;
    std::fs::write(&file_path, json)
        .map_err(|err| Error::Api(format!("failed to write CAR file: {err}")))?;

    let created_at = Utc::now().to_rfc3339();
    let file_path_str = file_path.to_string_lossy().to_string();
    
    conn.execute(
        "INSERT INTO receipts (id, run_id, created_at, file_path, match_kind, epsilon, s_grade) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            &receipt_id, // Use the unique ID here
            run_id,
            &created_at,
            &file_path_str,
            "pending", // Placeholder
            0.0,       // Placeholder
            car.sgrade.score as i64,
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