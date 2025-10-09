# Testing Guide for intelexta-verify

Quick reference for testing the verification tool.

---

## Build

```bash
# From intelexta/src-tauri
cargo build --release --package intelexta-verify

# Binary location
./target/release/intelexta-verify
```

---

## Basic Usage

```bash
# Verify a CAR file
intelexta-verify path/to/proof.car.zip

# JSON output (for automation)
intelexta-verify path/to/proof.car.json --format json

# Check exit code
intelexta-verify proof.car.zip && echo "PASSED" || echo "FAILED"
```

---

## Test Scenarios

### ✅ Test 1: Valid CAR (Should PASS)

**Setup**: Export a fresh CAR from Intelexta app

```bash
# In Intelexta app:
# 1. Run a workflow
# 2. Export as CAR (.car.zip)

# Verify
intelexta-verify valid_proof.car.zip

# Expected output:
#   ✓ File Integrity
#   ✓ Hash Chain (X/X checkpoints)
#   ✓ Signatures (X checkpoints)
#   ✓ Content Integrity (X/X provenance claims)
#   ✓ VERIFIED: This CAR is cryptographically valid
```

**Exit code**: 0

---

### ❌ Test 2: Tampered Prompt (Should FAIL)

**Setup**: Manually modify workflow prompt in CAR file

```bash
# Extract CAR
unzip valid_proof.car.zip -d test_tamper/

# Edit prompt in car.json
# Change: "prompt": "Original text"
# To:     "prompt": "TAMPERED TEXT"

# Re-zip
cd test_tamper && zip -r ../tampered_prompt.car.zip .

# Verify
intelexta-verify tampered_prompt.car.zip

# Expected output:
#   ✓ File Integrity
#   ✓ Hash Chain (X/X checkpoints)
#   ✓ Signatures (X checkpoints)
#   ✗ Content Integrity (0/X provenance claims)
#   ✗ FAILED: Config hash mismatch at provenance claim #0
```

**Exit code**: 1

**Why it fails**: The config hash (SHA256 of run.steps) no longer matches the hash in provenance claims.

---

### ❌ Test 3: Tampered Attachment (Should FAIL)

**Setup**: Modify an output file in attachments/

```bash
# Extract CAR
unzip valid_proof.car.zip -d test_tamper/

# Find attachment file
ls test_tamper/attachments/
# Example: 703e1a1e...txt

# Modify content
echo "TAMPERED OUTPUT" > test_tamper/attachments/703e1a1e*.txt

# Re-zip
cd test_tamper && zip -r ../tampered_attachment.car.zip .

# Verify
intelexta-verify tampered_attachment.car.zip

# Expected output:
#   ✓ File Integrity
#   ✓ Hash Chain (X/X checkpoints)
#   ✓ Signatures (X checkpoints)
#   ✗ Content Integrity (X/Y provenance claims)
#   ✗ FAILED: Attachment content mismatch
#   File: attachments/703e1a1e...txt
#   Expected hash (from filename): 703e1a1e...
#   Computed hash (from content): <different hash>
```

**Exit code**: 1

**Why it fails**: The attachment filename is the hash of its content. Any modification breaks this self-verifying property.

---

### ❌ Test 4: Tampered Checkpoint Hash (Should FAIL)

**Setup**: Modify a checkpoint's `curr_chain` value

```bash
# Extract CAR
unzip valid_proof.car.zip -d test_tamper/

# Edit checkpoint hash in car.json
# In proof.process.sequential_checkpoints[0]:
# Change: "curr_chain": "3e523aa6..."
# To:     "curr_chain": "FFFFFFFF..."

# Re-zip
cd test_tamper && zip -r ../tampered_hash.car.zip .

# Verify
intelexta-verify tampered_hash.car.zip

# Expected output:
#   ✓ File Integrity
#   ✗ Hash Chain (0/X checkpoints)
#   ✗ FAILED: Hash chain broken at checkpoint #0
#   Expected: 3e523aa6...
#   Found: FFFFFFFF...
```

**Exit code**: 1

**Why it fails**: The hash chain is computed from the checkpoint body. Manually changing it breaks the chain.

---

### ❌ Test 5: Invalid Signature (Should FAIL)

**Setup**: Modify a checkpoint's signature

```bash
# Extract CAR
unzip valid_proof.car.zip -d test_tamper/

# Edit signature in car.json
# In proof.process.sequential_checkpoints[0]:
# Change: "signature": "cG/gvH52..."
# To:     "signature": "AAAAAAAAA..."

# Re-zip
cd test_tamper && zip -r ../tampered_signature.car.zip .

# Verify
intelexta-verify tampered_signature.car.zip

# Expected output:
#   ✓ File Integrity
#   ✓ Hash Chain (X/X checkpoints)
#   ✗ Signatures (X checkpoints)
#   ✗ FAILED: Signature verification failed at checkpoint #0
```

**Exit code**: 1

**Why it fails**: The signature is an Ed25519 signature of the `curr_chain` hash. Invalid signatures fail cryptographic verification.

---

### ❌ Test 6: Old CAR Format (Should WARN)

**Setup**: Try to verify a CAR exported before v0.2

```bash
# Verify old CAR
intelexta-verify old_format.car.zip

# Expected output:
#   ✓ File Integrity
#   ✗ Hash Chain (not verified)
#   ✗ Signatures (not verified)
#   ✗ Content Integrity (not verified)
#   ✗ FAILED: CAR has no process proof (match_kind: semantic).
#   This CAR was likely exported with an older version of Intelexta.
#   Please re-export the CAR to include cryptographic signatures for verification.
```

**Exit code**: 1

**Why it fails**: Old CARs don't include `proof.process.sequential_checkpoints` field. Need to re-export from updated Intelexta.

---

## Test Matrix

| Test Case | File Integrity | Hash Chain | Signatures | Content Integrity | Overall |
|-----------|----------------|------------|------------|-------------------|---------|
| Valid CAR | ✅ | ✅ | ✅ | ✅ | ✅ PASS |
| Tampered Prompt | ✅ | ✅ | ✅ | ❌ | ❌ FAIL |
| Tampered Attachment | ✅ | ✅ | ✅ | ❌ | ❌ FAIL |
| Tampered Hash | ✅ | ❌ | N/A | N/A | ❌ FAIL |
| Invalid Signature | ✅ | ✅ | ❌ | N/A | ❌ FAIL |
| Old CAR Format | ✅ | ❌ | ❌ | ❌ | ❌ FAIL |

---

## Automated Testing Script

```bash
#!/bin/bash
# test_verification.sh

echo "Building intelexta-verify..."
cd /path/to/intelexta/src-tauri
cargo build --release --package intelexta-verify

VERIFY="./target/release/intelexta-verify"

echo ""
echo "=== Test 1: Valid CAR ==="
if $VERIFY test_data/valid.car.zip; then
    echo "✅ PASS: Valid CAR verified"
else
    echo "❌ FAIL: Valid CAR should pass"
fi

echo ""
echo "=== Test 2: Tampered Prompt ==="
if ! $VERIFY test_data/tampered_prompt.car.zip; then
    echo "✅ PASS: Tampered prompt detected"
else
    echo "❌ FAIL: Tampered prompt should fail"
fi

echo ""
echo "=== Test 3: Tampered Attachment ==="
if ! $VERIFY test_data/tampered_attachment.car.zip; then
    echo "✅ PASS: Tampered attachment detected"
else
    echo "❌ FAIL: Tampered attachment should fail"
fi

echo ""
echo "=== All tests complete ==="
```

---

## JSON Output for CI/CD

```bash
# Run verification in JSON mode
intelexta-verify proof.car.zip --format json > result.json

# Example output
cat result.json
{
  "car_id": "car:abc123...",
  "file_integrity": true,
  "hash_chain_valid": true,
  "signatures_valid": true,
  "content_integrity_valid": true,
  "checkpoints_verified": 3,
  "checkpoints_total": 3,
  "provenance_claims_verified": 4,
  "provenance_claims_total": 4,
  "overall_result": true
}

# Use in CI pipeline
if jq -e '.overall_result == true' result.json > /dev/null; then
    echo "Verification passed"
    exit 0
else
    echo "Verification failed"
    jq '.error' result.json
    exit 1
fi
```

---

## Debugging Tips

### Verbose Inspection

```bash
# Extract and inspect CAR structure
unzip -l proof.car.zip

# View car.json
unzip -p proof.car.zip car.json | jq .

# Check specific fields
unzip -p proof.car.zip car.json | jq '.proof.process.sequential_checkpoints[0]'
unzip -p proof.car.zip car.json | jq '.provenance'
unzip -p proof.car.zip car.json | jq '.signer_public_key'
```

### Manual Hash Verification

```bash
# Compute attachment hash
sha256sum attachments/703e1a1e*.txt
# Should match the filename (minus .txt extension)

# Compute config hash (requires jq and canonical JSON)
unzip -p proof.car.zip car.json | jq '.run.steps' | \
  # Note: This is simplified - actual verification uses JCS
  sha256sum
```

### Compare Two CARs

```bash
# Extract both
unzip -q car1.car.zip -d car1/
unzip -q car2.car.zip -d car2/

# Diff the JSON
diff <(jq -S . car1/car.json) <(jq -S . car2/car.json)

# Compare attachments
diff -r car1/attachments/ car2/attachments/
```

---

## Common Issues

### Issue: "CAR has no process proof"
**Cause**: CAR exported before v0.2
**Fix**: Re-export from updated Intelexta app

### Issue: "Invalid public key base64"
**Cause**: Corrupted or manually edited public key field
**Fix**: Use original CAR file, don't edit manually

### Issue: "Attachment content mismatch"
**Cause**: Attachment file was modified or corrupted
**Fix**: Tamper detected! Verify with original CAR

### Issue: "Config hash mismatch"
**Cause**: Workflow specification (prompts/models) was modified
**Fix**: Tamper detected! Verify with original CAR

---

## Creating Test Data

### Generate Valid CAR

1. Open Intelexta app
2. Create simple workflow (e.g., "Hello, world!" prompt)
3. Run workflow
4. Export as CAR
5. Save as `test_data/valid.car.zip`

### Generate Tampered CARs

```bash
# Script to create test suite
mkdir -p test_data

# 1. Valid CAR
cp ~/exports/original.car.zip test_data/valid.car.zip

# 2. Tampered prompt
unzip -q test_data/valid.car.zip -d tmp/
jq '.run.steps[0].prompt = "TAMPERED"' tmp/car.json > tmp/car_new.json
mv tmp/car_new.json tmp/car.json
cd tmp && zip -rq ../test_data/tampered_prompt.car.zip . && cd ..
rm -rf tmp/

# 3. Tampered attachment
unzip -q test_data/valid.car.zip -d tmp/
echo "TAMPERED" > tmp/attachments/*.txt
cd tmp && zip -rq ../test_data/tampered_attachment.car.zip . && cd ..
rm -rf tmp/

echo "Test data created!"
```

---

## Performance Benchmarks

```bash
# Time verification
time intelexta-verify large_workflow.car.zip

# Typical results:
# - Small workflow (1-3 steps): <100ms
# - Medium workflow (10-20 steps): <500ms
# - Large workflow (100+ steps): <2s

# Profile with valgrind (Linux)
valgrind --tool=callgrind ./target/release/intelexta-verify proof.car.zip
kcachegrind callgrind.out.*
```

---

## Next Steps

After testing Phase 1:
- See [ROADMAP.md](../../../../ROADMAP.md) for Phase 2 (Graded Replay)
- See [README.md](README.md) for usage documentation
- See [PHASE1_COMPLETE.md](../../../../PHASE1_COMPLETE.md) for project status
