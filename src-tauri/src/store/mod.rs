// In src-tauri/src/store/mod.rs

// This file makes the `store` directory a Rust module.
// Now we can declare sub-modules.

pub mod migrations;
pub mod policies;
pub mod projects;

// We'll also put the database migration logic here.
use crate::Error;
use rusqlite::params;

fn record_migration_versions(
    conn: &r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>,
    latest_version: i64,
) -> Result<(), Error> {
    for version in 1..=latest_version {
        conn.execute(
            "INSERT INTO migrations (version, applied_at)
             VALUES (?1, CURRENT_TIMESTAMP)
             ON CONFLICT(version) DO UPDATE SET applied_at = CURRENT_TIMESTAMP",
            params![version],
        )?;
    }
    Ok(())
}

pub fn migrate_db(
    conn: &r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>,
) -> Result<(), Error> {
    let mut runner = migrations::runner();
    runner.to_latest(conn)?;

    let latest_version = migrations::latest_version();
    if latest_version > 0 {
        record_migration_versions(conn, latest_version)?;
    }

    Ok(())
}
