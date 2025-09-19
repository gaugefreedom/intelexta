-- V4__create_checkpoint_messages.sql
CREATE TABLE IF NOT EXISTS checkpoint_messages (
    checkpoint_id TEXT PRIMARY KEY,
    role TEXT NOT NULL,
    body TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (checkpoint_id) REFERENCES checkpoints(id) ON DELETE CASCADE
);
