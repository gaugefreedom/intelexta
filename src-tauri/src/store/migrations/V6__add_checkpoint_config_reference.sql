-- src-tauri/src/store/migrations/V6__add_checkpoint_config_reference.sql
-- Add optional linkage from checkpoints to configured run checkpoints.

ALTER TABLE checkpoints ADD COLUMN checkpoint_config_id TEXT;
CREATE INDEX IF NOT EXISTS idx_checkpoints_config_id
    ON checkpoints(checkpoint_config_id);
