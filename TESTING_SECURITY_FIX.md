# Testing the Top-Level Signature Security Fix

This document explains how to test the security fix that prevents tampering with top-level CAR fields like `created_at`, `budgets`, and `sgrade`.

## What Was Fixed

**Problem**: Only checkpoint chain hashes were signed, allowing attackers to modify top-level fields without detection.

**Solution**: Dual signature system:
- `ed25519-body:<sig>` - Covers entire CAR body (all fields)
- `ed25519-checkpoint:<sig>` - Covers checkpoint chain hash

## Prerequisites

### 1. Build All Components

```bash
# Navigate to IntelexTA root
cd /home/marcelo/Documents/codes/gaugefreedom/intelexta

# Build verifiable-summary server
cd apps/verifiable-summary/server
npm run build
npm test  # Should show 2 tests passing

# Build WASM verifier
cd ../../web-verifier/wasm-verify
wasm-pack build --target web

# Build CLI verifier
cd ../../../src-tauri/crates/intelexta-verify
cargo build --release

# Verify the binary was built
ls -lh ../../../target/release/intelexta-verify
```

### 2. Start Verifiable-Summary Server

```bash
cd /home/marcelo/Documents/codes/gaugefreedom/intelexta/apps/verifiable-summary/server
npm run dev
# Server starts on http://localhost:3000
```

### 3. Restart ChatGPT Custom GPT

**Important**: You must refresh/restart the GPT to pick up the new code!

1. Go to your ChatGPT GPT configuration
2. Either:
   - Restart the entire GPT conversation, OR
   - Tell it to reconnect to the MCP server

## Test 1: Generate CAR from Verifiable-Summary

### Step 1: Generate a Signed CAR

In ChatGPT with the Verifiable Summary GPT:

```
Please create a verifiable summary of this article:
https://example.com/some-article

Make sure to sign it with the test key.
```

Download the `.car.zip` file to `~/Documents/teste/`

### Step 2: Verify Original CAR

```bash
cd ~/Documents/teste

# Extract and check structure
unzip -l verifiable.car.zip

# Should show:
# - car.json
# - attachments/[hash1].txt
# - attachments/[hash2].txt

# Verify with CLI
/home/marcelo/Documents/codes/gaugefreedom/intelexta/target/release/intelexta-verify \
  verifiable.car.zip

# Expected output:
# ✓ File Integrity
# ✓ Hash Chain (1/1 checkpoints)
# ✓ Signatures (1 checkpoints)
# ✓ Content Integrity (3/3 provenance claims)
# ✓ VERIFIED: This CAR is cryptographically valid
```

### Step 3: Check Signature Format

```bash
unzip verifiable.car.zip -d test-car/
cat test-car/car.json | grep -A 3 '"signatures"'

# Expected:
# "signatures": [
#   "ed25519-body:...",
#   "ed25519-checkpoint:..."
# ]
```

### Step 4: Test Tamper Detection

```bash
cd ~/Documents/teste

# Create tampered version
cp -r test-car test-car-tampered

# Use Node.js to tamper with created_at
node -e "
const fs = require('fs');
const car = JSON.parse(fs.readFileSync('test-car-tampered/car.json'));
console.log('Original created_at:', car.created_at);
car.created_at = '1970-01-01T00:00:00Z';
console.log('Tampered created_at:', car.created_at);
fs.writeFileSync('test-car-tampered/car.json', JSON.stringify(car, null, 2));
"

# Verify tampered CAR (should FAIL)
/home/marcelo/Documents/codes/gaugefreedom/intelexta/target/release/intelexta-verify \
  test-car-tampered/car.json

# Expected output:
# ✓ File Integrity
# ✓ Hash Chain (1/1 checkpoints)
# ✗ Signatures (1 checkpoints)
# ✗ Content Integrity (0/0 provenance claims)
# ✗ FAILED: Top-level body signature verification failed
```

### Step 5: Test Other Field Tampering

```bash
# Test tampering with budgets
node -e "
const fs = require('fs');
const car = JSON.parse(fs.readFileSync('test-car/car.json'));
car.budgets.usd = 999999;
fs.writeFileSync('test-car-tampered/car.json', JSON.stringify(car, null, 2));
"

/home/marcelo/Documents/codes/gaugefreedom/intelexta/target/release/intelexta-verify \
  test-car-tampered/car.json
# Should FAIL

# Test tampering with sgrade
node -e "
const fs = require('fs');
const car = JSON.parse(fs.readFileSync('test-car/car.json'));
car.sgrade.score = 100;
fs.writeFileSync('test-car-tampered/car.json', JSON.stringify(car, null, 2));
"

/home/marcelo/Documents/codes/gaugefreedom/intelexta/target/release/intelexta-verify \
  test-car-tampered/car.json
# Should FAIL
```

## Test 2: Web Verifier

### Step 1: Start Web Verifier

```bash
cd /home/marcelo/Documents/codes/gaugefreedom/intelexta/apps/web-verifier
npm run dev
# Opens at http://localhost:5173
```

### Step 2: Upload Original CAR

1. Drag `verifiable.car.zip` into the web verifier
2. Should show: ✅ **VERIFIED**
3. Check that it shows:
   - Hash Chain: 1/1 verified
   - Signatures: 1 verified
   - Provenance: 3/3 verified

### Step 3: Upload Tampered CAR

```bash
# Create ZIP of tampered CAR
cd ~/Documents/teste/test-car-tampered
zip -r ../tampered.car.zip *
```

1. Drag `tampered.car.zip` into the web verifier
2. Should show: ❌ **FAILED**
3. Error message: "Top-level body signature verification failed"

## Test 3: IntelexTA Desktop (Requires Code Update)

**Note**: IntelexTA Desktop needs to be updated to generate dual signatures.

### What Needs to Be Done

1. Locate signature generation in `src-tauri/src/orchestrator.rs`
2. Update to generate both signatures (body + checkpoint)
3. Rebuild IntelexTA Desktop
4. Generate a new CAR from IntelexTA
5. Verify it with the updated verifier

### After IntelexTA Update

```bash
# Generate CAR from IntelexTA Desktop
# (Use the UI to create a workflow and export CAR)

# Verify
/home/marcelo/Documents/codes/gaugefreedom/intelexta/target/release/intelexta-verify \
  ~/Documents/teste/intelexta-export.car.zip

# Test tampering
unzip intelexta-export.car.zip -d intel-car/
node -e "
const fs = require('fs');
const car = JSON.parse(fs.readFileSync('intel-car/car.json'));
car.created_at = '2000-01-01T00:00:00Z';
fs.writeFileSync('intel-car/car.json', JSON.stringify(car, null, 2));
"

/home/marcelo/Documents/codes/gaugefreedom/intelexta/target/release/intelexta-verify \
  intel-car/car.json
# Should FAIL
```

## Troubleshooting

### "Tests still failing"
```bash
cd apps/verifiable-summary/server
npm run build
npm test
```

### "Tamper detection not working"
- Make sure you built the latest CLI verifier: `cargo build --release`
- Check binary timestamp: `stat target/release/intelexta-verify | grep Modify`
- Should be from today (2025-10-29)

### "GPT not using new code"
- Restart the GPT conversation completely
- Check server logs to see if requests are coming in
- The server MUST be restarted to pick up new code

### "Signature format wrong"
Expected format in `car.json`:
```json
"signatures": [
  "ed25519-body:Lzear1/d65TSo...",
  "ed25519-checkpoint:15HoJHpr0uj..."
]
```

If you see:
```json
"signatures": ["ed25519:..."]  // OLD FORMAT - regenerate!
```

Then the old code is still running.

## Summary of Required Steps

### Before Testing
1. ✅ `npm run build` in verifiable-summary/server
2. ✅ `wasm-pack build --target web` in web-verifier/wasm-verify
3. ✅ `cargo build --release` in src-tauri/crates/intelexta-verify
4. ✅ Restart verifiable-summary server
5. ✅ Restart/refresh ChatGPT GPT

### Testing Checklist
- [ ] Generate new CAR from verifiable-summary
- [ ] Verify original CAR passes
- [ ] Check signatures array has 2 entries with correct prefixes
- [ ] Tamper with `created_at` - should FAIL
- [ ] Tamper with `budgets` - should FAIL
- [ ] Tamper with `sgrade` - should FAIL
- [ ] Test in web verifier - original should pass, tampered should fail

## Success Criteria

✅ **Original CAR**: All checks pass
❌ **Tampered CAR**: Verification fails with "Top-level body signature verification failed"

This proves the security fix is working correctly!
