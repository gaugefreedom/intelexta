// In src-tauri/src/store/mod.rs

// This file makes the `store` directory a Rust module.
// Now we can declare sub-modules.

pub mod policies;
pub mod projects;

// We'll also put the database migration logic here.
use crate::Error;

fn ensure_semantic_digest_column(
    conn: &r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>,
) -> Result<(), Error> {
    let mut stmt = conn.prepare(
        "SELECT 1 FROM pragma_table_info('checkpoints') WHERE name = 'semantic_digest' LIMIT 1",
    )?;
    let exists = stmt.exists([])?;
    drop(stmt);

    if !exists {
        conn.execute(
            "ALTER TABLE checkpoints ADD COLUMN semantic_digest TEXT",
            [],
        )?;
    }

    Ok(())
}

pub fn migrate_db(
    conn: &r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>,
) -> Result<(), Error> {
    // This is a simple, one-time migration. For a real app,
    // you'd use a more robust migration library to apply `migrations.version`.
    conn.execute_batch(include_str!("./schema.sql"))?;
    ensure_semantic_digest_column(conn)?;
    Ok(())
}
