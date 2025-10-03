-- V11__add_step_type_to_run_steps.sql
-- Add support for heterogeneous step types (LLM and Document Ingestion)
PRAGMA foreign_keys=OFF;

-- Create new table with step_type and config_json columns
-- Make model and prompt nullable for non-LLM steps
CREATE TABLE run_steps_new (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    order_index INTEGER NOT NULL,
    checkpoint_type TEXT NOT NULL DEFAULT 'Step',
    step_type TEXT NOT NULL DEFAULT 'llm',
    model TEXT,
    prompt TEXT,
    token_budget INTEGER NOT NULL,
    proof_mode TEXT NOT NULL DEFAULT 'exact',
    epsilon REAL,
    config_json TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (run_id) REFERENCES runs(id) ON DELETE CASCADE
);

-- Copy existing data (all existing steps are LLM steps)
INSERT INTO run_steps_new (
    id,
    run_id,
    order_index,
    checkpoint_type,
    step_type,
    model,
    prompt,
    token_budget,
    proof_mode,
    epsilon,
    config_json,
    created_at,
    updated_at
)
SELECT
    id,
    run_id,
    order_index,
    checkpoint_type,
    'llm' AS step_type,
    model,
    prompt,
    token_budget,
    proof_mode,
    epsilon,
    NULL AS config_json,
    created_at,
    updated_at
FROM run_steps;

-- Drop old table
DROP TABLE run_steps;

-- Rename new table
ALTER TABLE run_steps_new RENAME TO run_steps;

-- Recreate indexes
CREATE UNIQUE INDEX idx_run_steps_order ON run_steps(run_id, order_index);
CREATE INDEX idx_run_steps_run_id ON run_steps(run_id);

PRAGMA foreign_keys=ON;
