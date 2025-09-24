-- V8__add_proof_mode_to_run_checkpoints.sql
ALTER TABLE run_checkpoints ADD COLUMN proof_mode TEXT NOT NULL DEFAULT 'exact';
