// In src-tauri/src/lib.rs

use chrono::{DateTime, Utc};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use serde::{Deserialize, Serialize};

// The shared database pool type
pub type DbPool = Pool<SqliteConnectionManager>;

// Your main API error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Db(#[from] rusqlite::Error),
    #[error(transparent)]
    Pool(#[from] r2d2::Error),
    #[error(transparent)]
    Keyring(#[from] keyring::Error),
    #[error(transparent)]
    Migration(#[from] rusqlite_migration::Error),
    #[error("API Error: {0}")]
    Api(String),
}

impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

// Re-export modules to be accessible from main.rs
pub mod api;
pub mod car;
pub mod chunk;
pub mod governance;
pub mod ingest;
pub mod keychain;
pub mod orchestrator;
pub mod provenance;
pub mod store;

// === Core Data Structures for Sprint 0 ===

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub pubkey: String, // base64 encoded
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum CheckpointKind {
    Step,
    Incident,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Incident {
    pub kind: String,     // e.g., "budget_exceeded", "policy_change"
    pub severity: String, // "info" | "warn" | "error"
    pub details: String,
    pub related_checkpoint_id: Option<String>,
}

#[cfg(test)]
mod tests;
