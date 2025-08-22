use crate::DbPool;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tauri::State;
use crate::{ingest, chunk};
use std::path::Path;
use tiktoken_rs::cl100k_base;

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

#[derive(Debug, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub project_id: String,
    pub source_path: String,
}

#[tauri::command]
pub fn add_document(project_id: String, file_path: String, pool: State<DbPool>) -> Result<Document, ApiError> {
    let conn = pool.get()?;
    let tx = conn.transaction()?; // Use a transaction

    // 1. Extract text from file
    let text = ingest::extract_text(Path::new(&file_path))
        .map_err(|e| ApiError::DbError(rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))))?;

    // 2. Create the document record
    let doc_id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now();
    tx.execute(
        "INSERT INTO document (id, project_id, source_path, imported_at) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![&doc_id, &project_id, &file_path, now],
    )?;

    // 3. Chunk the text
    let chunks = chunk::chunk_text(&text)
        .map_err(|e| ApiError::DbError(rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))))?;
    let bpe = cl100k_base().unwrap();

    // 4. Insert each chunk
    for (i, chunk_content) in chunks.iter().enumerate() {
        let chunk_id = uuid::Uuid::new_v4().to_string();
        let token_count = bpe.encode_with_special_tokens(chunk_content).len();
        tx.execute(
            "INSERT INTO chunk (id, document_id, chunk_index, content, token_count) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![chunk_id, &doc_id, i, chunk_content, token_count],
        )?;
    }
    
    tx.commit()?;

    Ok(Document { id: doc_id, project_id, source_path: file_path })
}

// And a command to view the documents
#[tauri::command]
pub fn list_documents(project_id: String, pool: State<DbPool>) -> Result<Vec<Document>, ApiError> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare("SELECT id, project_id, source_path FROM document WHERE project_id = ?1 ORDER BY imported_at DESC")?;
    
    let doc_iter = stmt.query_map(rusqlite::params![project_id], |row| {
        Ok(Document {
            id: row.get(0)?,
            project_id: row.get(1)?,
            source_path: row.get(2)?,
        })
    })?;

    let documents = doc_iter.collect::<Result<Vec<_>, _>>()?;
    Ok(documents)
}