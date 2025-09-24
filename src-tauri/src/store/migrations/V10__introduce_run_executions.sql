-- V10__introduce_run_executions.sql
PRAGMA foreign_keys=OFF;

CREATE TABLE run_executions (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (run_id) REFERENCES runs(id)
);

CREATE INDEX idx_run_executions_run_id ON run_executions(run_id);

ALTER TABLE checkpoints ADD COLUMN run_execution_id TEXT;

UPDATE checkpoints
SET run_execution_id = run_id || '-legacy'
WHERE run_execution_id IS NULL;

INSERT INTO run_executions (id, run_id, created_at)
SELECT DISTINCT
    run_id || '-legacy' AS id,
    run_id,
    MIN(timestamp) AS created_at
FROM checkpoints
GROUP BY run_id;

CREATE TABLE checkpoints_new (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    run_execution_id TEXT NOT NULL,
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
    FOREIGN KEY (run_execution_id) REFERENCES run_executions(id),
    FOREIGN KEY (parent_checkpoint_id) REFERENCES checkpoints(id),
    FOREIGN KEY (checkpoint_config_id) REFERENCES run_steps(id)
);

INSERT INTO checkpoints_new (
    id,
    run_id,
    run_execution_id,
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
    run_execution_id,
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

CREATE INDEX idx_checkpoints_config_id ON checkpoints(checkpoint_config_id);
CREATE INDEX idx_checkpoints_execution ON checkpoints(run_execution_id);
CREATE INDEX idx_ckpt_run ON checkpoints(run_id);

PRAGMA foreign_keys=ON;
