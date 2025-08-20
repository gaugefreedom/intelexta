// src-tauri/src/store.rs
use rusqlite::Connection;

pub fn migrate_db(conn: &Connection) -> Result<(), anyhow::Error> {
    static SCHEMA_SQL: &str = include_str!("store/schema.sql");
    conn.execute_batch(SCHEMA_SQL)?;
    // enable useful pragmas
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;",
    )?;
    Ok(())
}
