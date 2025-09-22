-- src-tauri/src/store/migrations/V5__create_run_checkpoint_configs.sql
-- Introduce run-level checkpoint configuration storage and supporting metadata.

ALTER TABLE runs ADD COLUMN seed INTEGER NOT NULL DEFAULT 0;
ALTER TABLE runs ADD COLUMN epsilon REAL;
ALTER TABLE runs ADD COLUMN token_budget INTEGER NOT NULL DEFAULT 0;
ALTER TABLE runs ADD COLUMN default_model TEXT NOT NULL DEFAULT '';

CREATE TABLE IF NOT EXISTS run_checkpoints (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    order_index INTEGER NOT NULL,
    checkpoint_type TEXT NOT NULL DEFAULT 'Step',
    model TEXT NOT NULL,
    prompt TEXT NOT NULL,
    token_budget INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (run_id) REFERENCES runs(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_run_checkpoints_order
    ON run_checkpoints(run_id, order_index);

CREATE INDEX IF NOT EXISTS idx_run_checkpoints_run_id
    ON run_checkpoints(run_id);
