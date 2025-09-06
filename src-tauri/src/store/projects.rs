// In src-tauri/src/store/projects.rs

use crate::{Error, Project};
use chrono::Utc;
use rusqlite::{params, Connection};

// === Queries for the 'projects' table ===

/// Creates a new project in the database.
pub fn create(
    conn: &Connection,
    id: &str,
    name: &str,
    pubkey: &str,
) -> Result<Project, Error> {
    let now = Utc::now();

    conn.execute(
        "INSERT INTO projects (id, name, created_at, pubkey) VALUES (?1, ?2, ?3, ?4)",
        params![id, name, &now, pubkey],
    )?;

    Ok(Project {
        id: id.to_string(),
        name: name.to_string(),
        created_at: now,
        pubkey: pubkey.to_string(),
    })
}

/// Lists all projects, ordered by most recently created.
pub fn list(conn: &Connection) -> Result<Vec<Project>, Error> {
    let mut stmt =
        conn.prepare("SELECT id, name, created_at, pubkey FROM projects ORDER BY created_at DESC")?;

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