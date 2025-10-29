# Security Issue: Missing Top-Level Signature Verification

**Severity**: HIGH
**Status**: Identified, Fix In Progress
**Affects**: web-verifier (WASM), intelexta-verify (CLI)

## Issue Description

Currently, the verifiers only validate signatures on checkpoint chain hashes (`checkpoint.curr_chain`) but do NOT verify a signature covering the entire CAR body. This means an attacker can modify top-level fields like `created_at`, `run_id`, `budgets`, `sgrade`, etc. without invalidating the verification.

### Proof of Concept

1. Take a valid signed CAR bundle
2. Modify `car.json` field `created_at` from `"2025-10-29T05:00:00Z"` to `"1970-01-01T00:00:00Z"`
3. Run verification: ✅ **PASSES** (incorrectly!)

The checkpoint signatures still verify because they only cover the chain hash, not the full document.

## Current Behavior

### What IS Verified
- ✅ Checkpoint chain integrity (`prev_chain` → `curr_chain`)
- ✅ Checkpoint signatures (`sign(curr_chain)`)
- ✅ Provenance claim hashes
- ✅ Attachment file integrity

### What IS NOT Verified
- ❌ Top-level CAR body signature
- ❌ Tamper-proof `created_at` timestamp
- ❌ Tamper-proof `run_id`
- ❌ Tamper-proof `budgets`
- ❌ Tamper-proof `sgrade`
- ❌ Tamper-proof `policy_ref`

## Expected Behavior

The `signatures` array should contain:
1. **Checkpoint signatures** (already done) - Sign each `curr_chain`
2. **Top-level body signature** (missing) - Sign canonical CAR body

### Correct Signing Flow

```typescript
// 1. Build CAR body without id and signatures
const carBody = {
  run_id,
  created_at,
  run,
  proof,
  policy_ref,
  budgets,
  provenance,
  checkpoints,
  sgrade,
  signer_public_key
};

// 2. Compute deterministic ID
const canonical = canonicalize(carBody);
const carId = `car:${sha256(canonical)}`;

// 3. Add ID to body
const bodyWithId = { id: carId, ...carBody };

// 4. Sign the full body (with ID, without signatures)
const bodyCanonical = canonicalize(bodyWithId);
const bodySignature = signEd25519(bodyCanonical, secretKey);

// 5. Add checkpoint signatures (curr_chain for each)
const checkpointSigs = checkpoints.map(cp => signEd25519(cp.curr_chain, secretKey));

// 6. Build final signatures array
const signatures = [
  `ed25519:${bodySignature}`,        // Top-level body
  ...checkpointSigs.map(s => `ed25519:${s}`)  // Each checkpoint
];

// 7. Final CAR
const car = { ...bodyWithId, signatures };
```

### Correct Verification Flow

```rust
// 1. Load car.json
let car: Car = serde_json::from_str(&car_json)?;

// 2. Extract signatures
let signatures = car.signatures.clone();
let public_key = car.signer_public_key.clone();

// 3. Verify top-level body signature (first signature)
let body_signature = signatures.get(0)
    .ok_or(anyhow!("Missing top-level body signature"))?;

// 4. Rebuild body without signatures
let mut car_without_sigs = car.clone();
car_without_sigs.signatures = vec![];

// 5. Canonicalize and verify
let canonical = canonical_json(&car_without_sigs)?;
verify_signature(&public_key, &canonical, body_signature)?;

// 6. Verify checkpoint signatures (remaining signatures)
for (index, checkpoint) in car.proof.process.sequential_checkpoints.iter().enumerate() {
    let checkpoint_sig = signatures.get(index + 1)
        .ok_or(anyhow!("Missing checkpoint signature #{index}"))?;

    verify_signature(&public_key, checkpoint.curr_chain.as_bytes(), checkpoint_sig)?;
}
```

## Impact

### Security Implications

1. **Timestamp Manipulation**: Attacker can claim a CAR was created at any time
2. **Budget Manipulation**: Attacker can falsify resource usage costs
3. **Grade Manipulation**: Attacker can inflate `sgrade` scores
4. **Policy Bypass**: Attacker can change `policy_ref` to claim different governance

### Real-World Attack Scenario

```
Original CAR:
{
  "id": "car:abc123...",
  "created_at": "2025-10-29T10:00:00Z",
  "budgets": { "usd": 5.00, "tokens": 5000, "nature_cost": 0.05 },
  "sgrade": { "score": 75, ... },
  ...
}

Tampered CAR (still verifies!):
{
  "id": "car:abc123...",  // Same ID
  "created_at": "2020-01-01T00:00:00Z",  // ← Backdated by 5 years!
  "budgets": { "usd": 0.50, "tokens": 500, "nature_cost": 0.005 },  // ← 10x cheaper!
  "sgrade": { "score": 99, ... },  // ← Inflated grade!
  ...
}
```

This would pass current verification ✓ but should FAIL ✗

## Proposed Fix

### Phase 1: Update Signature Generation (provenance.ts)

**Current (INSECURE)**:
```typescript
// Only signs checkpoint chain
if (secretKeyB64) {
  const signature = signEd25519(currChain, secretKeyB64);
  signatures = [`ed25519:${signature}`];
}
```

**Fixed (SECURE)**:
```typescript
// 1. Sign full body
const bodyWithId = { id: carId, ...carBody };
const bodyCanonical = canonicalize(bodyWithId);
const bodySignature = signEd25519(bodyCanonical, secretKeyB64);

// 2. Sign checkpoint chain
const checkpointSignature = signEd25519(currChain, secretKeyB64);

// 3. Store both
signatures = [
  `ed25519-body:${bodySignature}`,           // Top-level
  `ed25519-checkpoint:${checkpointSignature}` // Checkpoint
];
```

### Phase 2: Update Verifiers

**WASM Verifier** (`apps/web-verifier/wasm-verify/src/lib.rs`):
```rust
// Add top-level signature verification step
fn verify_top_level_signature(car: &Car) -> Result<()> {
    if car.signatures.is_empty() {
        return Err(anyhow!("No signatures found"));
    }

    let body_sig = &car.signatures[0];
    if !body_sig.starts_with("ed25519-body:") {
        return Err(anyhow!("First signature must be top-level body signature"));
    }

    // Rebuild body without signatures
    let mut car_without_sigs = car.clone();
    car_without_sigs.signatures = vec![];

    // Canonicalize
    let car_json = serde_json::to_value(&car_without_sigs)?;
    let canonical = canonical_json(&car_json)?;

    // Verify signature
    let sig_b64 = body_sig.strip_prefix("ed25519-body:").unwrap();
    verify_ed25519(&car.signer_public_key, &canonical, sig_b64)
        .context("Top-level body signature verification failed")?;

    Ok(())
}
```

**CLI Verifier** (similar changes in Rust CLI code)

### Phase 3: Migration Strategy

To avoid breaking existing CARs:

1. **Detection**: Check signature format
   - If `signatures[0].starts_with("ed25519-body:")` → new format (verify both)
   - Otherwise → legacy format (verify checkpoints only, WARN user)

2. **Deprecation Period**:
   - Accept both formats for 6 months
   - Log warnings for legacy format
   - After 6 months: reject legacy format

## Testing Plan

### Test Cases

1. **Valid signed CAR**: ✅ PASS
2. **Modify `created_at`**: ✗ FAIL (top-level sig invalid)
3. **Modify `budgets`**: ✗ FAIL (top-level sig invalid)
4. **Modify `sgrade`**: ✗ FAIL (top-level sig invalid)
5. **Modify checkpoint data**: ✗ FAIL (checkpoint sig invalid)
6. **Unsigned CAR**: ⚠️ PASS (with warning)

### Manual Verification

```bash
# 1. Generate valid signed CAR
npm run dev  # Start server
# Generate summary in ChatGPT → download verifiable.car.zip

# 2. Verify original (should PASS)
./intelexta-verify verifiable.car.zip

# 3. Tamper with created_at
unzip verifiable.car.zip -d test/
jq '.created_at = "1970-01-01T00:00:00Z"' test/car.json > test/car_tampered.json
mv test/car_tampered.json test/car.json
cd test && zip -r ../tampered.car.zip * && cd ..

# 4. Verify tampered (should FAIL with new code)
./intelexta-verify tampered.car.zip
# Expected: "Top-level body signature verification failed"
```

## References

- **JCS Spec**: RFC 8785 (JSON Canonicalization Scheme)
- **Ed25519**: RFC 8032
- **Provenance Code**: `apps/verifiable-summary/server/src/provenance.ts`
- **WASM Verifier**: `apps/web-verifier/wasm-verify/src/lib.rs`

## Timeline

- **Day 1**: Document issue (this file) ✅
- **Day 2**: Fix provenance.ts signature generation
- **Day 3**: Update WASM verifier
- **Day 4**: Update CLI verifier
- **Day 5**: Testing and validation
- **Week 2**: Deploy with backward compatibility

## Contact

Report related issues: https://github.com/gaugefreedom/intelexta/issues
