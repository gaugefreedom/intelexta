-- V7__create_checkpoint_payloads.sql
CREATE TABLE IF NOT EXISTS checkpoint_payloads (
    checkpoint_id TEXT PRIMARY KEY,
    prompt_payload TEXT,
    output_payload TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (checkpoint_id) REFERENCES checkpoints(id) ON DELETE CASCADE
);
