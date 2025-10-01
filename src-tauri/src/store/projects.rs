// In src-tauri/src/store/projects.rs
use crate::{Error, Project};
use chrono::Utc;
use rusqlite::{params, Connection};

pub fn create(conn: &Connection, id: &str, name: &str, pubkey: &str) -> Result<Project, Error> {
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

pub fn rename(conn: &Connection, id: &str, name: &str) -> Result<Project, Error> {
    let affected = conn.execute(
        "UPDATE projects SET name = ?1 WHERE id = ?2",
        params![name, id],
    )?;
    if affected == 0 {
        return Err(Error::Api(format!("Project {id} not found")));
    }
    let mut stmt =
        conn.prepare("SELECT id, name, created_at, pubkey FROM projects WHERE id = ?1")?;
    let project = stmt.query_row(params![id], |row| {
        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            created_at: row.get(2)?,
            pubkey: row.get(3)?,
        })
    })?;
    Ok(project)
}

pub fn delete(conn: &mut Connection, id: &str) -> Result<(), Error> {
    let tx = conn.transaction()?;

    tx.execute("DELETE FROM policies WHERE project_id = ?1", params![id])?;

    tx.execute(
        "DELETE FROM checkpoint_payloads WHERE checkpoint_id IN (SELECT id FROM checkpoints WHERE run_id IN (SELECT id FROM runs WHERE project_id = ?1))",
        params![id],
    )?;

    tx.execute(
        "DELETE FROM checkpoint_messages WHERE checkpoint_id IN (SELECT id FROM checkpoints WHERE run_id IN (SELECT id FROM runs WHERE project_id = ?1))",
        params![id],
    )?;

    tx.execute(
        "DELETE FROM receipts WHERE run_id IN (SELECT id FROM runs WHERE project_id = ?1)",
        params![id],
    )?;

    tx.execute(
        "DELETE FROM checkpoints WHERE run_id IN (SELECT id FROM runs WHERE project_id = ?1)",
        params![id],
    )?;

    tx.execute(
        "DELETE FROM run_executions WHERE run_id IN (SELECT id FROM runs WHERE project_id = ?1)",
        params![id],
    )?;

    tx.execute("DELETE FROM runs WHERE project_id = ?1", params![id])?;

    let affected = tx.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
    if affected == 0 {
        return Err(Error::Api(format!("Project {id} not found")));
    }

    tx.commit()?;

    Ok(())
}
