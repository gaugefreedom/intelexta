// src-tauri/src/store/migrations.rs
use rusqlite_migration::{Migrations, M};

const MIGRATION_SCRIPTS: &[&str] = &[
    include_str!("migrations/V1__initial_schema.sql"),
    include_str!("migrations/V2__add_semantic_digest_to_checkpoints.sql"),
    include_str!("migrations/V3__add_token_breakdown_to_checkpoints.sql"),
    include_str!("migrations/V4__create_checkpoint_messages.sql"),
    include_str!("migrations/V5__create_run_checkpoint_configs.sql"),
    include_str!("migrations/V6__add_checkpoint_config_reference.sql"),
    include_str!("migrations/V7__create_checkpoint_payloads.sql"),
    include_str!("migrations/V8__add_proof_mode_to_run_checkpoints.sql"),
    include_str!("migrations/V9__rename_run_checkpoints_to_run_steps.sql"),
];

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
