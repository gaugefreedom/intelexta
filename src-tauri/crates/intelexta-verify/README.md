# intelexta-verify

Standalone command-line tool for verifying Intelexta CAR (Content-Addressed Receipt) files.

**Status**: ✅ **Phase 1 MVP Complete** (v0.2) - Full cryptographic integrity verification

## What it does

`intelexta-verify` provides **trustless verification** of AI workflow proofs without requiring the full Intelexta application or database.

### ✅ Completed Features (Phase 1 MVP)

1. **Full CAR File Support**
   - ✅ Reads `.car.json` (plain JSON)
   - ✅ Reads `.car.zip` (compressed archives with attachments)
   - ✅ Auto-detects format

2. **Cryptographic Verification**
   - ✅ Hash chain integrity (SHA-256 with JCS canonical JSON)
   - ✅ Ed25519 signature verification on every checkpoint
   - ✅ Content integrity (workflow config + attachments)
   - ✅ Tamper detection for:
     - Modified prompts or models in workflow specification
     - Changed attachment files (outputs)
     - Altered checkpoint metadata (timestamps, tokens, hashes)
     - Forged or invalid signatures

3. **User-Friendly Output**
   - ✅ Colored terminal output with clear indicators
   - ✅ JSON format for automation/CI pipelines
   - ✅ Detailed error messages showing what failed and why

4. **Production Ready**
   - ✅ Proper error handling with context
   - ✅ Exit codes (0=verified, 1=failed)
   - ✅ CLI argument parsing with help text
   - ✅ Works offline (no network or database required)

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

### ✅ Phase 1: Integrity Verification (COMPLETED - v0.2)

The verification process has 4 stages:

#### 1. File Integrity
- Parses CAR from `.car.json` (plain JSON) or `.car.zip` (compressed archive)
- Auto-detects format and extracts if needed
- Validates JSON structure against CAR schema

#### 2. Hash Chain Verification
- Each checkpoint contains a cryptographic chain: `SHA256(prev_chain || canonical_json(checkpoint_body))`
- Verifies every checkpoint's `curr_chain` matches the computed hash
- Uses JCS (JSON Canonicalization Scheme) for deterministic hashing
- **Detects**: Any modification to checkpoint metadata, timestamps, or token counts

#### 3. Signature Verification
- Each checkpoint is digitally signed with Ed25519
- Verifies signature against the checkpoint's `curr_chain` hash
- Uses the public key from `signer_public_key` field
- **Detects**: Forged checkpoints or unauthorized modifications

#### 4. Content Integrity Verification
- **Config hash**: Computes `SHA256(canonical_json(run.steps))` and verifies against provenance claim
  - **Detects**: Modified prompts, changed models, altered workflow configuration
- **Attachment verification**: For each file in `attachments/`, verifies content matches filename hash
  - Files are content-addressed: `attachments/{sha256_hash}.txt`
  - **Detects**: Modified outputs, tampered attachments, substituted files

**Result**: Any tampering with prompts, models, outputs, or execution metadata causes verification to fail.

### 🔮 Phase 2: Graded Replay (FUTURE)

The next phase will add **reproducibility verification** by re-executing workflows:

- Parse workflow specification from CAR
- Re-execute each step using the same models/prompts (requires API keys via env vars)
- Compare outputs with semantic similarity scoring
- Generate graded verification report:
  - **A+**: Exact match (deterministic outputs)
  - **A**: High similarity (>95% semantic match)
  - **B**: Good similarity (>80% semantic match)
  - **C**: Partial similarity (>60% semantic match)
  - **F**: Failed to reproduce (<60% similarity)

This will enable verification of reproducibility claims and detection of model drift over time.

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
