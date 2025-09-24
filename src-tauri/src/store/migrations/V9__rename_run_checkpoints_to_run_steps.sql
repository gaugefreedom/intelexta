-- V9__rename_run_checkpoints_to_run_steps.sql
PRAGMA foreign_keys=OFF;

DROP INDEX IF EXISTS idx_run_checkpoints_order;
DROP INDEX IF EXISTS idx_run_checkpoints_run_id;

ALTER TABLE run_checkpoints RENAME TO run_steps;

ALTER TABLE run_steps ADD COLUMN epsilon REAL;

UPDATE run_steps
SET epsilon = (
    SELECT epsilon FROM runs WHERE runs.id = run_steps.run_id
);

CREATE TABLE runs_new (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL,
    sampler_json TEXT,
    seed INTEGER NOT NULL DEFAULT 0,
    epsilon REAL,
    token_budget INTEGER NOT NULL DEFAULT 0,
    default_model TEXT NOT NULL DEFAULT '',
    proof_mode TEXT NOT NULL DEFAULT 'exact',
    FOREIGN KEY (project_id) REFERENCES projects(id)
);

INSERT INTO runs_new (
    id,
    project_id,
    name,
    created_at,
    sampler_json,
    seed,
    epsilon,
    token_budget,
    default_model,
    proof_mode
)
SELECT
    id,
    project_id,
    name,
    created_at,
    sampler_json,
    seed,
    epsilon,
    token_budget,
    default_model,
    COALESCE(json_extract(spec_json, '$.proofMode'), 'exact')
FROM runs;

DROP TABLE runs;

ALTER TABLE runs_new RENAME TO runs;

CREATE TABLE checkpoints_new (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    checkpoint_config_id TEXT,
    parent_checkpoint_id TEXT,
    turn_index INTEGER,
    kind TEXT NOT NULL DEFAULT 'Step',
    incident_json TEXT,
    timestamp TEXT NOT NULL,
    inputs_sha256 TEXT,
    outputs_sha256 TEXT,
    prev_chain TEXT,
    curr_chain TEXT NOT NULL UNIQUE,
    signature TEXT NOT NULL,
    usage_tokens INTEGER NOT NULL DEFAULT 0,
    prompt_tokens INTEGER NOT NULL DEFAULT 0,
    completion_tokens INTEGER NOT NULL DEFAULT 0,
    semantic_digest TEXT,
    FOREIGN KEY (run_id) REFERENCES runs(id),
    FOREIGN KEY (parent_checkpoint_id) REFERENCES checkpoints(id),
    FOREIGN KEY (checkpoint_config_id) REFERENCES run_steps(id)
);

INSERT INTO checkpoints_new (
    id,
    run_id,
    checkpoint_config_id,
    parent_checkpoint_id,
    turn_index,
    kind,
    incident_json,
    timestamp,
    inputs_sha256,
    outputs_sha256,
    prev_chain,
    curr_chain,
    signature,
    usage_tokens,
    prompt_tokens,
    completion_tokens,
    semantic_digest
)
SELECT
    id,
    run_id,
    checkpoint_config_id,
    parent_checkpoint_id,
    turn_index,
    kind,
    incident_json,
    timestamp,
    inputs_sha256,
    outputs_sha256,
    prev_chain,
    curr_chain,
    signature,
    usage_tokens,
    prompt_tokens,
    completion_tokens,
    semantic_digest
FROM checkpoints;

DROP TABLE checkpoints;

ALTER TABLE checkpoints_new RENAME TO checkpoints;

PRAGMA foreign_keys=ON;

CREATE UNIQUE INDEX IF NOT EXISTS idx_run_steps_order
    ON run_steps(run_id, order_index);

CREATE INDEX IF NOT EXISTS idx_run_steps_run_id
    ON run_steps(run_id);

CREATE INDEX IF NOT EXISTS idx_checkpoints_config_id
    ON checkpoints(checkpoint_config_id);

CREATE INDEX IF NOT EXISTS idx_ckpt_run
    ON checkpoints(run_id);

CREATE INDEX IF NOT EXISTS idx_runs_project
    ON runs(project_id);
