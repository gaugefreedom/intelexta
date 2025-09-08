// In src-tauri/src/api.rs
use crate::{orchestrator, provenance, store, DbPool, Error, Project};
use serde::Deserialize;
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