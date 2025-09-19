-- V3__add_token_breakdown_to_checkpoints.sql
ALTER TABLE checkpoints ADD COLUMN prompt_tokens INTEGER NOT NULL DEFAULT 0;
ALTER TABLE checkpoints ADD COLUMN completion_tokens INTEGER NOT NULL DEFAULT 0;
