// src-tauri/src/store.rs

use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;

// The SQL commands from our schema file, embedded directly into the binary
const MIGRATION: &str = include_str!("store/schema.sql");

// Custom error type for store operations
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error(transparent)]
    PoolError(#[from] r2d2::Error),
    #[error(transparent)]
    DbError(#[from] rusqlite::Error),
}

// The migration function that sets up the database
pub fn migrate_db(conn: &PooledConnection<SqliteConnectionManager>) -> Result<(), StoreError> {
    conn.execute_batch(MIGRATION)?;
    println!("Database migration completed successfully.");
    Ok(())
}

// We'll also define our `ApiError` mapping here to keep things tidy.
// In api.rs, you can then change `ApiError` to use `store::StoreError`.

impl serde::Serialize for StoreError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
