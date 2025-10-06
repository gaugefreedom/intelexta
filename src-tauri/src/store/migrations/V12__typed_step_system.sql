-- V12__typed_step_system.sql
-- Transition to typed step system (ingest, summarize, prompt)
-- Make model and prompt nullable since not all step types need them
-- Ensure config_json exists and is the primary configuration storage

PRAGMA foreign_keys=OFF;

-- Create new table with proper nullable columns
CREATE TABLE run_steps_new (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    order_index INTEGER NOT NULL,
    checkpoint_type TEXT NOT NULL DEFAULT 'Step',
    step_type TEXT NOT NULL, -- 'ingest', 'summarize', 'prompt' (or legacy 'llm', 'document_ingestion')
    model TEXT,              -- Nullable: only for LLM steps (summarize, prompt)
    prompt TEXT,             -- Nullable: only for prompt steps
    token_budget INTEGER,    -- Nullable: only for LLM steps
    proof_mode TEXT DEFAULT 'exact',  -- Nullable: only for LLM steps
    epsilon REAL,            -- Nullable: only for concordant mode
    config_json TEXT,        -- Primary configuration (StepConfig enum serialized)
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (run_id) REFERENCES runs(id) ON DELETE CASCADE
);

-- Migrate existing data
-- For steps with step_type, copy as-is
-- For steps without step_type, default to 'prompt' (was 'llm')
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
    CASE
        -- Map old step types to new typed system
        WHEN step_type = 'llm' OR step_type = 'llm_prompt' THEN 'prompt'
        WHEN step_type = 'document_ingestion' THEN 'ingest'
        -- If step_type column doesn't exist yet, default to 'prompt'
        ELSE COALESCE(step_type, 'prompt')
    END AS step_type,
    model,
    prompt,
    token_budget,
    proof_mode,
    epsilon,
    config_json,
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
CREATE INDEX idx_run_steps_type ON run_steps(step_type);  -- New index for filtering by type

PRAGMA foreign_keys=ON;
