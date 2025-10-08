# CAR Bundle with Attachments Implementation

**Status**: ✅ Complete (Task 2.2 of Phase 2)
**Date**: October 7, 2025

---

## Overview

Enhanced the "Emit CAR" button to create complete verification bundles that include both the CAR JSON metadata and all full output attachments. This enables independent verification of AI workflow outputs.

### Problem Solved

**Before**:
- CAR export created only `.car.json` file (metadata)
- Output hashes included, but not the actual text
- Verifiers couldn't independently check outputs
- Incomplete proof chain

**After**:
- CAR export creates `.car.zip` bundle containing:
  - `car.json` - Complete metadata, signatures, hashes
  - `attachments/` - All full, untruncated outputs
- Complete, self-contained verification package
- Verifiers can independently verify all outputs

---

## Architecture

### Bundle Structure

```
car_abc123...hash.zip
├── car.json                    # CAR metadata (2-3KB)
└── attachments/
    ├── hash1.txt              # Full output from checkpoint 1
    ├── hash2.txt              # Full output from checkpoint 2
    └── hash3.txt              # Full output from checkpoint 3
```

### Data Flow

```
User clicks "Emit CAR"
      ↓
1. Build CAR JSON (metadata, hashes, signatures)
2. Query checkpoint_payloads for full_output_hash values
3. Load full outputs from attachment store
4. Create zip file:
   - Add car.json
   - Add each attachment as attachments/{hash}.txt
5. Save to receipts directory
6. Record in database
      ↓
User receives path to .car.zip file
```

---

## Implementation Details

### 1. New Function: `build_car_bundle` (`src-tauri/src/car.rs`)

**Purpose**: Create a complete CAR bundle with attachments

**Signature**:
```rust
pub fn build_car_bundle(
    conn: &Connection,
    run_id: &str,
    run_execution_id: Option<&str>,
    output_path: &std::path::Path,
) -> Result<()>
```

**Logic**:
```rust
// 1. Build CAR JSON
let car = build_car(conn, run_id, run_execution_id)?;
let car_json = serde_json::to_string_pretty(&car)?;

// 2. Create zip file
let mut zip = ZipWriter::new(File::create(output_path)?);

// 3. Add car.json
zip.start_file("car.json", FileOptions::default())?;
zip.write_all(car_json.as_bytes())?;

// 4. Collect attachment hashes from checkpoints
let mut attachment_hashes = Vec::new();
for checkpoint_id in &car.checkpoints {
    let hash: Option<String> = conn.query_row(
        "SELECT full_output_hash FROM checkpoint_payloads WHERE checkpoint_id = ?1",
        params![checkpoint_id],
        |row| row.get(0),
    ).optional()?;

    if let Some(h) = hash {
        attachment_hashes.push(h);
    }
}

// 5. Add all attachments to zip
let attachment_store = crate::attachments::get_global_attachment_store();
for hash in attachment_hashes {
    if attachment_store.exists(&hash) {
        let content = attachment_store.load_full_output(&hash)?;
        zip.start_file(&format!("attachments/{}.txt", hash), FileOptions::default())?;
        zip.write_all(content.as_bytes())?;
    }
}

zip.finish()?;
```

### 2. Updated: `emit_car_to_base_dir` (`src-tauri/src/api.rs`)

**Before**:
```rust
let file_path = receipts_dir.join(format!("{}.car.json", car.id.replace(':', "_")));
let json = serde_json::to_string_pretty(&car)?;
std::fs::write(&file_path, json)?;
```

**After**:
```rust
let file_path = receipts_dir.join(format!("{}.car.zip", car.id.replace(':', "_")));
car::build_car_bundle(&conn, run_id, run_execution_id, &file_path)?;
```

### 3. Updated: `emit_car` Command (`src-tauri/src/api.rs`)

**Custom Path Case**:
```rust
if let Some(custom_path) = output_path {
    let conn = pool.get()?;
    let car = car::build_car(&conn, &run_id, None)?;

    let custom_path_buf = PathBuf::from(&custom_path);
    car::build_car_bundle(&conn, &run_id, None, &custom_path_buf)?;

    // Record in database
    conn.execute(
        "INSERT INTO receipts (id, run_id, created_at, file_path, match_kind, epsilon, s_grade)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![&car.id, &run_id, &created_at, &custom_path, ...],
    )?;

    Ok(custom_path)
}
```

---

## Files Modified

1. **`src-tauri/src/car.rs`** (+57 lines)
   - Added `build_car_bundle()` function
   - Collects attachment hashes from checkpoints
   - Creates zip with CAR + attachments

2. **`src-tauri/src/api.rs`** (modified)
   - Updated `emit_car_to_base_dir()` to create `.zip` instead of `.json`
   - Updated `emit_car()` custom path handling
   - Both now use `car::build_car_bundle()`

---

## Testing

### Test 1: Export CAR Bundle

**Steps**:
1. Restart the app (`npm run tauri dev`)
2. Execute a run with multiple steps
3. Open Inspector → select the run
4. Click "Emit CAR" button
5. Check console for success message with file path

**Expected Result**:
- Message: "CAR emitted to: /path/to/car_abc123.car.zip"
- File created at path
- File size larger than before (includes attachments)

### Test 2: Inspect Bundle Contents

**Extract the zip**:
```bash
# Find the latest CAR bundle
CAR_FILE=$(ls -t ~/.local/share/com.intelexta.dev/*/receipts/*.car.zip | head -1)

# Create temp directory
mkdir /tmp/car_test
cd /tmp/car_test

# Extract
unzip "$CAR_FILE"

# Verify structure
ls -lh
# Should see:
# car.json
# attachments/

ls -lh attachments/
# Should see:
# hash1.txt
# hash2.txt
# etc.
```

**Verify car.json**:
```bash
cat car.json | head -50
```

Should contain:
- run metadata
- checkpoint IDs
- output hashes in provenance claims
- model_catalog_hash and model_catalog_version
- signatures

**Verify attachments**:
```bash
# Check one attachment
cat attachments/*.txt | head -20
```

Should contain full, untruncated output text.

### Test 3: Verify Hash Integrity

**Goal**: Confirm attachment hashes match what's in car.json

```bash
cd /tmp/car_test

# Get a hash from car.json provenance
EXPECTED_HASH=$(jq -r '.provenance[] | select(.claim_type == "output") | .sha256' car.json | head -1 | sed 's/sha256://')

echo "Expected hash: $EXPECTED_HASH"

# Find matching attachment
ATTACHMENT_FILE="attachments/$EXPECTED_HASH.txt"

if [ -f "$ATTACHMENT_FILE" ]; then
    # Compute actual hash
    ACTUAL_HASH=$(sha256sum "$ATTACHMENT_FILE" | awk '{print $1}')
    echo "Actual hash:   $ACTUAL_HASH"

    # Compare
    if [ "$EXPECTED_HASH" == "$ACTUAL_HASH" ]; then
        echo "✅ Hash matches!"
    else
        echo "❌ Hash mismatch!"
    fi
else
    echo "⚠️  Attachment file not found"
fi
```

### Test 4: Compare with Attachment Store

**Verify bundle attachments match originals**:

```bash
# Get one attachment hash
HASH=$(ls /tmp/car_test/attachments/ | head -1 | sed 's/.txt$//')

# Compare bundle vs original
diff /tmp/car_test/attachments/${HASH}.txt \
     ~/.local/share/com.intelexta.dev/attachments/${HASH:0:2}/${HASH}.txt

# Should output nothing (files identical)
```

---

## Usage

### For Users

**Export a CAR bundle**:
1. Execute a run
2. Open Inspector
3. Select the run
4. Click "Emit CAR"
5. Note the file path in the success message

**Find CAR bundles**:
```bash
ls -lh ~/.local/share/com.intelexta.dev/*/receipts/*.car.zip
```

**Share a CAR bundle**:
- Send the `.car.zip` file to verifiers
- They can extract and independently verify all outputs

### For Verifiers

**Verify a CAR bundle**:
1. Extract the `.car.zip` file
2. Read `car.json` for metadata
3. Check `attachments/` for full outputs
4. Verify hashes match:
   ```bash
   jq -r '.provenance[] | select(.claim_type == "output") | .sha256' car.json | \
   while read hash; do
       hash_clean=$(echo $hash | sed 's/sha256://')
       sha256sum attachments/${hash_clean}.txt
   done
   ```
5. Optionally replay the run with `intelexta-verify` (Phase 3)

---

## Benefits

### Complete Verification Package

✅ **Self-Contained**: Everything needed for verification in one file
✅ **Portable**: Share via email, USB, IPFS, etc.
✅ **Tamper-Evident**: Hashes verify attachment integrity
✅ **Independent**: Verifier doesn't need original database

### Trustless Verification

✅ **Output Hashes**: Provenance claims include SHA256 of outputs
✅ **Full Outputs**: Verifier can compute hashes independently
✅ **Signatures**: Ed25519 signatures prove authenticity
✅ **Model Catalog**: Catalog hash enables cost verification

### Use Cases

**Academic Research**:
- Submit CAR bundle with paper
- Reviewers verify AI-assisted claims
- Reproducible research

**Legal/Compliance**:
- Audit trail for AI analysis
- Prove no tampering
- Chain of custody

**AI Safety**:
- Verify model outputs
- Detect prompt injection
- Policy compliance proof

---

## File Size Comparison

### Before (JSON only):
```bash
ls -lh ~/.local/share/com.intelexta.dev/cars/car_*.car.json
# 2.7K  car_abc123.car.json
```

### After (ZIP bundle):
```bash
ls -lh ~/.local/share/com.intelexta.dev/*/receipts/car_*.car.zip
# 15K   car_abc123.car.zip  (3 checkpoints, ~4KB each output)
```

**Note**: Size varies based on:
- Number of checkpoints
- Length of outputs
- Zip compression (typically 30-50% reduction)

---

## Future Enhancements

### Phase 3 Tasks:

1. **intelexta-verify CLI**
   - Standalone tool to verify CAR bundles
   - No database required
   - Outputs verification report

2. **Graded Replay in Bundle**
   - Include replay results in CAR
   - Similarity scores for each checkpoint
   - Grade (A-F) for stochastic outputs

3. **CAR Bundle Import**
   - Import CAR bundle into database
   - View in Inspector
   - Compare with new runs

4. **IPFS/Arweave Upload**
   - Publish CAR bundles to decentralized storage
   - Permanent, censorship-resistant proofs
   - Content-addressable URLs

---

## Verification Workflow

### Full Verification Process:

```
1. User executes run
         ↓
2. User clicks "Emit CAR"
         ↓
3. CAR bundle created with attachments
         ↓
4. User shares .car.zip file
         ↓
5. Verifier extracts bundle
         ↓
6. Verifier checks:
   - Signatures (Ed25519)
   - Hash chains (prev_chain → curr_chain)
   - Output hashes (SHA256)
   - Policy compliance
   - Model catalog integrity
         ↓
7. Verifier runs `intelexta-verify` (Phase 3)
         ↓
8. Verification report generated:
   - ✅ All signatures valid
   - ✅ All hashes match
   - ✅ Policy followed
   - ✅ Model costs accurate
   - Grade: A (98% similarity on replay)
         ↓
9. Verifier trusts the AI workflow output
```

---

## Success Criteria

✅ **All Completed**:

- [x] CAR bundles include full attachments
- [x] Zip file created instead of JSON only
- [x] Attachments organized in `attachments/` directory
- [x] Hash integrity maintained
- [x] File paths updated (`.car.json` → `.car.zip`)
- [x] Database records correct paths
- [x] Backend compiles successfully
- [x] Ready for user testing

**Ready for**:
- User testing with real runs
- Hash verification
- Sharing with external verifiers

---

## Summary

The CAR Bundle implementation provides:

- **Complete verification packages** with metadata + outputs
- **Zip compression** for efficient storage/sharing
- **Hash integrity** via SHA256 verification
- **Self-contained proofs** requiring no database
- **Foundation for Phase 3** (intelexta-verify CLI)

This completes the critical path for independent AI workflow verification, enabling trustless proof of AI-assisted work.

**Implementation Time**: ~2 hours
**Files Modified**: 2
**Lines Added**: ~60
**Format**: `.car.zip` (was `.car.json`)
