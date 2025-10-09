# intelexta-verify

Standalone command-line tool for verifying Intelexta CAR (Content-Addressed Receipt) files.

## What it does

`intelexta-verify` provides **trustless verification** of AI workflow proofs without requiring the full Intelexta application or database. It verifies:

- ✅ **File Integrity**: CAR file is properly formatted
- ✅ **Hash Chain**: Tamper-evident chain linking all checkpoints
- ✅ **Cryptographic Signatures**: Ed25519 signatures on every checkpoint
- ✅ **Content Integrity**: Workflow config and attachment files match their hashes
  - Config hash: Verifies prompts/models in `run.steps` haven't been tampered with
  - Attachment hashes: Verifies all output files match their checkpoint hashes

## Installation

### Build from source

```bash
# From the intelexta repository root
cd src-tauri
cargo build --release --package intelexta-verify

# The binary will be at:
# target/release/intelexta-verify
```

### Copy to PATH (optional)

```bash
# Linux/macOS
sudo cp target/release/intelexta-verify /usr/local/bin/

# Or add to your shell config:
export PATH="$PATH:/path/to/intelexta/src-tauri/target/release"
```

## Usage

### Basic verification

```bash
intelexta-verify my_proof.car.json
```

Or with a ZIP archive:

```bash
intelexta-verify my_proof.car.zip
```

### Example output

```
Intelexta CAR Verification
==================================================

CAR ID: car:abc123...

  ✓ File Integrity
  ✓ Hash Chain (3/3 checkpoints)
  ✓ Signatures (3 checkpoints)
  ✓ Content Integrity (4/4 provenance claims)

--------------------------------------------------
✓ VERIFIED: This CAR is cryptographically valid and has not been tampered with.
```

### JSON output (for automation)

```bash
intelexta-verify my_proof.car.json --format json
```

Output:
```json
{
  "car_id": "car:abc123...",
  "file_integrity": true,
  "hash_chain_valid": true,
  "signatures_valid": true,
  "checkpoints_verified": 3,
  "checkpoints_total": 3,
  "overall_result": true
}
```

### Exit codes

- `0`: Verification passed
- `1`: Verification failed

This makes it easy to use in scripts:

```bash
if intelexta-verify proof.car.zip; then
    echo "Valid proof!"
else
    echo "Invalid proof!"
fi
```

## How it works

### Phase 1: Integrity Verification (Current)

1. **Parse CAR file**: Reads JSON or extracts from ZIP
2. **Verify hash chain**: Each checkpoint's `curr_chain` must equal `SHA256(prev_chain || canonical_json(checkpoint_body))`
3. **Verify signatures**: Each checkpoint's Ed25519 signature must be valid for the `curr_chain` hash
4. **Verify content integrity**:
   - **Config hash**: Ensures workflow specification (prompts, models) matches the hash in provenance claims
   - **Attachment files**: For each checkpoint with an `outputs_sha256`, verifies the file `attachments/{hash}.txt` exists and matches the hash
   - **Tamper detection**: Any modification to prompts, attachments, or checkpoint hashes will cause verification to fail

### Future: Phase 2 - Graded Replay

The next version will support **reproducibility verification** by re-running the workflow and comparing outputs:

- Parse workflow specification from CAR
- Re-execute each step (requires API keys via env vars)
- Compare outputs with similarity scoring
- Generate graded verification report (A+, A, B, etc.)

## CAR File Format

CAR files contain:
- **Workflow specification** (steps, models, prompts)
- **Execution metadata** (timestamps, costs)
- **Cryptographic proofs** (hash chains, signatures)
- **Checkpoints** (inputs/outputs at each step)

They can be exported from Intelexta as:
- `.car.json` - Plain JSON
- `.car.zip` - Compressed archive with attachments

**Note**: CARs exported before v0.2 may not include the `proof.process` field required for signature verification. If you encounter an error about missing process proof, re-export the CAR from Intelexta to include the latest cryptographic evidence.

## Why verification matters

> "I used AI to help write this" is not verifiable.

Intelexta CARs provide **cryptographic proof** that:
1. Workflow steps were executed in sequence
2. Outputs have not been tampered with
3. Results are reproducible (Phase 2)

This enables:
- **Auditable AI workflows** for compliance
- **Third-party verification** without trusting the creator
- **Immutable proof** for critical applications

## Development

### Run tests

```bash
cargo test --package intelexta-verify
```

### Run with sample data

```bash
cargo run --package intelexta-verify -- tests/data/sample.json
```

## License

Part of the Intelexta project. See main repository for license details.
