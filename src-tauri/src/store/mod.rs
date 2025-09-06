// In src-tauri/src/store/mod.rs

// This file makes the `store` directory a Rust module.
// Now we can declare sub-modules.

pub mod projects;

// We'll also put the database migration logic here.
use crate::Error;

pub fn migrate_db(conn: &r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>) -> Result<(), Error> {
    // This is a simple, one-time migration. For a real app,
    // you'd use a more robust migration library to apply `migrations.version`.
    conn.execute_batch(include_str!("./schema.sql"))?;
    Ok(())
}