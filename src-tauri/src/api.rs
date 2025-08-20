use crate::DbPool;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tauri::State;

// Custom error type for our API
#[derive(Debug, thiserror::Error)]
enum ApiError {
    #[error(transparent)]
    DbError(#[from] rusqlite::Error),
    #[error(transparent)]
    PoolError(#[from] r2d2::Error),
}

// Convert our custom error into a user-facing string for the frontend
impl serde::Serialize for ApiError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

// Update the Project struct to match our schema more closely
#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // settings_json can be added later if needed
}

#[tauri::command]
pub fn create_project(name: String, pool: State<DbPool>) -> Result<Project, ApiError> {
    let conn = pool.get()?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now();

    conn.execute(
        "INSERT INTO project (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![id, name, now, now],
    )?;

    let project = Project {
        id,
        name,
        created_at: now,
        updated_at: now,
    };

    Ok(project)
}

#[tauri::command]
pub fn list_projects(pool: State<DbPool>) -> Result<Vec<Project>, ApiError> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare("SELECT id, name, created_at, updated_at FROM project ORDER BY updated_at DESC")?;
    
    let project_iter = stmt.query_map([], |row| {
        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            created_at: row.get(2)?,
            updated_at: row.get(3)?,
        })
    })?;

    let projects = project_iter.collect::<Result<Vec<Project>, _>>()?;
    
    Ok(projects)
}