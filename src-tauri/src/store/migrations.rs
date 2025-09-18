use rusqlite_migration::{Migrations, M};

const MIGRATION_SCRIPTS: &[&str] = &[include_str!("migrations/V1__initial_schema.sql")];

pub fn runner() -> Migrations<'static> {
    let steps = MIGRATION_SCRIPTS
        .iter()
        .map(|sql| M::up(*sql))
        .collect::<Vec<_>>();
    Migrations::new(steps)
}

pub fn latest_version() -> i64 {
    MIGRATION_SCRIPTS.len() as i64
}
