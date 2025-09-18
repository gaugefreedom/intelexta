// In src-tauri/src/api.rs
use crate::{
    orchestrator, provenance,
    store::{self, policies::Policy},
    DbPool, Error, Project,
};
use rusqlite::{params, types::Type};
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn list_projects(pool: State<DbPool>) -> Result<Vec<Project>, Error> {
    // Get the connection from the pool
    let conn = pool.get()?;
    // Pass a reference to the connection
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

    // Get the connection from the pool
    let conn = pool.get()?;
    // Pass a reference to the connection
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
        "SELECT id, timestamp, kind, incident_json, inputs_sha256, outputs_sha256, usage_tokens FROM checkpoints WHERE run_id = ?1 ORDER BY timestamp ASC",
    )?;

    let rows = stmt.query_map(params![run_id], |row| {
        let incident_json: Option<String> = row.get(3)?;
        let incident = incident_json
            .map(|payload| serde_json::from_str::<IncidentSummary>(&payload))
            .transpose()
            .map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(3, Type::Text, Box::new(err))
            })?;

        Ok(CheckpointSummary {
            id: row.get(0)?,
            timestamp: row.get(1)?,
            kind: row.get(2)?,
            incident,
            inputs_sha256: row.get(4)?,
            outputs_sha256: row.get(5)?,
            usage_tokens: {
                let value: i64 = row.get(6)?;
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
