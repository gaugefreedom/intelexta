-- V13__add_full_output_hash.sql
-- Add full_output_hash column to checkpoint_payloads for attachment store reference
ALTER TABLE checkpoint_payloads ADD COLUMN full_output_hash TEXT;

-- Create index for faster lookups by hash
CREATE INDEX IF NOT EXISTS idx_checkpoint_payloads_hash ON checkpoint_payloads(full_output_hash);
