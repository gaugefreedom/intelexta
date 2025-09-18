-- V2__add_semantic_digest_to_checkpoints.sql
-- Add the semantic_digest column to the checkpoints table for Concordant proof mode.
ALTER TABLE checkpoints ADD COLUMN semantic_digest TEXT;