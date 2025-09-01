// In src-tauri/src/api.rs
use crate::{provenance, DbPool};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error(transparent)]
    DbError(#[from] rusqlite::Error),
    #[error(transparent)]
    PoolError(#[from] r2d2::Error),
}

impl serde::Serialize for ApiError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

// This struct matches the `projects` table in our new schema
#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub pubkey: String,
}

#[tauri::command]
pub fn create_project(name: String, pool: State<DbPool>) -> Result<Project, ApiError> {
    let conn = pool.get()?;
    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let keypair = provenance::generate_keypair();

    conn.execute(
        "INSERT INTO projects (id, name, created_at, pubkey) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![&id, &name, &now, &keypair.public_key],
    )?;

    // IMPORTANT: In a real app, the secret key would be returned to the frontend to be
    // stored securely (e.g., in the OS keystore), not just discarded.
    // For Sprint 1, we just need to prove we can generate it and store the public key.

    let project = Project {
        id,
        name,
        created_at: now,
        pubkey: keypair.public_key,
    };

    Ok(project)
}

#[tauri::command]
pub fn list_projects(pool: State<DbPool>) -> Result<Vec<Project>, ApiError> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare("SELECT id, name, created_at, pubkey FROM projects ORDER BY created_at DESC")?;

    let project_iter = stmt.query_map([], |row| {
        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            created_at: row.get(2)?,
            pubkey: row.get(3)?,
        })
    })?;

    let projects = project_iter.collect::<Result<Vec<Project>, _>>()?;
    Ok(projects)
}