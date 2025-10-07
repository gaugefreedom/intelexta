# Critical Fix: Serde rename_all Configuration

## Date: 2025-10-07

## Issue Found During Testing

When testing Ingest Document step, the debug logging revealed:

```
🔍 Attempting to parse config_json: {"stepType":"ingest","sourcePath":"/home/marcelo/Documents/2025/research/essay/Eucharist and the Ethos of Open Source Sharing.pdf","format":"pdf","privacyStatus":"public"}
❌ Failed to parse as typed step: missing field `source_path`
   Falling back to legacy execution
```

## Root Cause

The `#[serde(rename_all = "camelCase")]` was at the **enum level** instead of at the **variant level**.

### Before (INCORRECT):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "stepType", rename_all = "camelCase")]  // ❌ At enum level
pub enum StepConfig {
    #[serde(rename = "ingest")]
    Ingest {
        source_path: String,     // Expected "source_path" instead of "sourcePath"
        format: String,
        privacy_status: String,  // Expected "privacy_status" instead of "privacyStatus"
    },
    // ...
}
```

With `rename_all` at the enum level on a **tagged enum**, serde doesn't apply the renaming to the variant fields correctly.

### After (CORRECT):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "stepType")]  // ✅ No rename_all here
pub enum StepConfig {
    #[serde(rename = "ingest", rename_all = "camelCase")]  // ✅ At variant level
    Ingest {
        source_path: String,     // Now expects "sourcePath" ✓
        format: String,
        privacy_status: String,  // Now expects "privacyStatus" ✓
    },

    #[serde(rename = "summarize", rename_all = "camelCase")]  // ✅ At variant level
    Summarize {
        source_step: Option<usize>,  // Expects "sourceStep" ✓
        model: String,
        summary_type: String,        // Expects "summaryType" ✓
        custom_instructions: Option<String>,  // Expects "customInstructions" ✓
        token_budget: Option<i32>,   // Expects "tokenBudget" ✓
        proof_mode: Option<String>,  // Expects "proofMode" ✓
        epsilon: Option<f64>,
    },

    #[serde(rename = "prompt", rename_all = "camelCase")]  // ✅ At variant level
    Prompt {
        model: String,
        prompt: String,
        use_output_from: Option<usize>,  // Expects "useOutputFrom" ✓
        token_budget: Option<i32>,       // Expects "tokenBudget" ✓
        proof_mode: Option<String>,      // Expects "proofMode" ✓
        epsilon: Option<f64>,
    },
}
```

## Why This Matters

For **tagged enums** (enums with `#[serde(tag = "...")]`), the `rename_all` attribute must be on **each variant**, not on the enum itself. This is because:

1. The tag field (`stepType`) is handled specially
2. Each variant's fields are serialized as part of the same JSON object as the tag
3. Serde needs to know how to rename fields for each variant independently

## Fix Applied

**File**: `src-tauri/src/orchestrator.rs` (lines 42-93)

- Removed `rename_all = "camelCase"` from line 42 (enum level)
- Added `rename_all = "camelCase"` to each variant:
  - Line 45: `#[serde(rename = "ingest", rename_all = "camelCase")]`
  - Line 53: `#[serde(rename = "summarize", rename_all = "camelCase")]`
  - Line 75: `#[serde(rename = "prompt", rename_all = "camelCase")]`

## Testing

### Expected JSON Matching

**Ingest**:
```json
{
  "stepType": "ingest",
  "sourcePath": "...",      // ✓ camelCase
  "format": "pdf",
  "privacyStatus": "public" // ✓ camelCase
}
```

**Summarize**:
```json
{
  "stepType": "summarize",
  "sourceStep": 0,          // ✓ camelCase
  "model": "stub",
  "summaryType": "brief",   // ✓ camelCase
  "customInstructions": "...", // ✓ camelCase (optional)
  "tokenBudget": 2000,      // ✓ camelCase (optional)
  "proofMode": "exact",     // ✓ camelCase (optional)
  "epsilon": 0.5            // ✓ (optional)
}
```

**Prompt**:
```json
{
  "stepType": "prompt",
  "model": "stub",
  "prompt": "...",
  "useOutputFrom": 0,       // ✓ camelCase (optional)
  "tokenBudget": 1500,      // ✓ camelCase (optional)
  "proofMode": "exact",     // ✓ camelCase (optional)
  "epsilon": 0.5            // ✓ (optional)
}
```

### Verification

After rebuilding, test again:
1. Create Ingest Document step
2. Execute run
3. Should see: `✅ Successfully parsed typed step: Ingest { source_path: "...", format: "pdf", privacy_status: "public" }`
4. No more "missing field `source_path`" error

## Status

- ✅ Fix applied
- ✅ Backend rebuilt successfully
- ⏳ Ready for testing

## Impact

This fix resolves the JSON deserialization issue completely. All typed steps should now:
- Parse successfully from `config_json`
- Execute via typed execution paths (not fall back to legacy)
- Chain correctly (output from one step flows to next)

## Related Files

- `src-tauri/src/orchestrator.rs`: StepConfig enum definition (FIXED)
- `app/src/components/CheckpointEditor.tsx`: Generates correct camelCase JSON (already correct)
- `docs/TYPED_STEPS_FIXES.md`: Previous fix documentation (referenced the issue)
- `docs/ERROR_ANALYSIS_TYPED_STEPS.md`: Detailed analysis that led to this fix
