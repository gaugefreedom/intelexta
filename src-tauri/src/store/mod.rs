// In src-tauri/src/store/mod.rs

// This file makes the `store` directory a Rust module.
// Now we can declare sub-modules.

pub mod migrations;
pub mod policies;
pub mod projects;

// We'll also put the database migration logic here.
use crate::Error;
use rusqlite::params;

// This helper function seems redundant if rusqlite_migration handles its own tracking.
// We can re-evaluate if we need this later, but for now, it's fine.
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

// The main migration function.
// This is the resolved version that uses the automated runner.
pub fn migrate_db(
    conn: &mut r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>,
) -> Result<(), Error> {
    // Get the migration runner.
    let runner = migrations::runner();

    // Apply migrations to the latest version.
    // FIX: Pass a mutable reference to the dereferenced connection,
    // as required by the rusqlite_migration library.
    runner.to_latest(conn)?;

    // Record the version that was applied.
    let latest_version = migrations::latest_version();
    if latest_version > 0 {
        record_migration_versions(conn, latest_version as i64)?;
    }

    Ok(())
}
