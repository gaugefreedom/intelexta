# Error Analysis: "LLM step missing prompt"

## Problem Summary

When creating new typed steps (Ingest Document, Summarize, Prompt with optional context) and executing a full run, the system throws:
```
[Error] Failed to execute run – "API Error: LLM step missing prompt"
```

## Root Cause Analysis

### The Error Location
The error originates from `src-tauri/src/orchestrator.rs:2093`:

```rust
fn execute_checkpoint(
    config: &RunStep,
    run_seed: u64,
    llm_client: &dyn LlmClient,
) -> anyhow::Result<NodeExecution> {
    // Check if this is a document ingestion step
    if config.is_document_ingestion() {
        let config_json = config.config_json.as_ref()
            .ok_or_else(|| anyhow!("Document ingestion step missing config_json"))?;
        return execute_document_ingestion_checkpoint(config_json);
    }

    // For LLM steps, model and prompt must be present
    let model = config.model.as_ref()
        .ok_or_else(|| anyhow!("LLM step missing model"))?;
    let prompt = config.prompt.as_ref()
        .ok_or_else(|| anyhow!("LLM step missing prompt"))?;  // ❌ LINE 2093

    // ... rest of function
}
```

### Why This Happens

The error occurs because **typed steps are falling back to legacy execution**. Here's the execution flow:

1. **Frontend** creates a step with `config_json` containing all data:
   ```json
   {
     "stepType": "prompt",
     "model": "stub",
     "prompt": "Analyze this",
     "useOutputFrom": 0,
     "tokenBudget": 1500,
     "proofMode": "exact"
   }
   ```

2. **Backend** receives the step and stores it in database with:
   - `step_type = "prompt"`
   - `config_json = "{\"stepType\":\"prompt\", ...}"`
   - `model = NULL` (for typed steps, model is in config_json, not as separate field)
   - `prompt = NULL` (for typed steps, prompt is in config_json, not as separate field)

3. **During execution** (`src-tauri/src/orchestrator.rs:1740-1827`):
   - System tries to parse `config_json` as `StepConfig` enum
   - **If parsing fails**, it falls back to legacy `execute_checkpoint()`
   - Legacy function expects `config.model` and `config.prompt` to be populated
   - Since they're NULL for typed steps, it throws "LLM step missing prompt"

### Why Parsing Might Fail

The parsing can fail for several reasons:

#### 1. **JSON Structure Mismatch** (Most Likely)
The Rust `StepConfig` enum uses:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "stepType", rename_all = "camelCase")]
pub enum StepConfig {
    #[serde(rename = "ingest")]
    Ingest {
        source_path: String,        // expects "sourcePath" in JSON
        format: String,
        privacy_status: String,     // expects "privacyStatus" in JSON
    },
    // ...
}
```

The frontend sends camelCase, which should work. However, **there might be extra fields** that the Rust deserializer doesn't expect.

#### 2. **Undefined Fields in JSON**
JavaScript can send `undefined` values which become `null` or missing in JSON:
```typescript
const configJson = JSON.stringify({
  stepType: "prompt",
  model: cleanedModel,
  prompt: cleanedPrompt,
  useOutputFrom: useOutputFrom,  // ⚠️ Could be null or undefined
  tokenBudget: parsedBudget,
  proofMode: proofMode,
  epsilon: proofMode === "concordant" ? epsilon : undefined,  // ⚠️ undefined
});
```

When `epsilon: undefined` is stringified, it's omitted from JSON. This should be fine with `#[serde(skip_serializing_if = "Option::is_none")]`, but there might be edge cases.

#### 3. **Type Mismatches**
- Frontend sends `sourceStep: number` but Rust expects `source_step: Option<usize>`
- Frontend sends `tokenBudget: number` but Rust expects `token_budget: Option<i32>`

If the number is too large or has decimals, parsing could fail.

## Debug Logging Added

I've added debug logging to help identify the issue:

```rust
// Line 1742-1745
eprintln!("🔍 Attempting to parse config_json: {}", config_json_str);
match serde_json::from_str::<StepConfig>(config_json_str) {
    Ok(step_config) => {
        eprintln!("✅ Successfully parsed typed step: {:?}", step_config);
        // ... typed execution
    }
    Err(parse_err) => {
        eprintln!("❌ Failed to parse as typed step: {}", parse_err);
        eprintln!("   Falling back to legacy execution");
        // ... legacy execution
    }
}
```

## How to Reproduce and Debug

### Step 1: Run the application with console output visible

```bash
cd /home/marcelo/Documents/codes/gaugefreedom/intelexta
cargo tauri dev
```

Watch the terminal for the debug messages when you execute a run.

### Step 2: Create a test workflow

1. Create new run
2. Add Step 1: **Ingest Document** type
   - Select a PDF file
   - Format: PDF
   - Privacy: Public
3. Add Step 2: **Prompt** type
   - Model: stub
   - Prompt: "What is this document about?"
   - Use Output From: Step 1
4. Execute run

### Step 3: Check console output

Look for:
```
🔍 Attempting to parse config_json: {...}
✅ Successfully parsed typed step: ...
```

or

```
🔍 Attempting to parse config_json: {...}
❌ Failed to parse as typed step: ...
   Falling back to legacy execution
```

The error message will tell you **exactly why** the parsing failed.

## Potential Issues Found

### Issue 1: Frontend might not be setting prompt for non-prompt steps

Looking at `CheckpointEditor.tsx` (lines 226-236), for "ingest" steps:
```typescript
await onSubmit({
  stepType: "ingest",
  checkpointType: cleanedType,
  sourcePath: cleanedPath,
  format,
  privacyStatus,
  configJson,
  tokenBudget: 1000,
  proofMode: "exact",
  // ❌ No 'prompt' or 'model' field
});
```

When this goes through the API layer, the RunStep struct will have:
- `model: None`
- `prompt: None`

This is correct for typed steps. But if parsing fails and it falls back to legacy execution, the legacy path will fail.

### Issue 2: Inconsistent field naming

Frontend sends (line 259):
```typescript
sourceStep: sourceStep,  // JavaScript variable
```

Which becomes:
```json
{
  "sourceStep": 0  // JSON key
}
```

Rust expects (line 56):
```rust
source_step: Option<usize>,  // Rust field
```

With `#[serde(rename_all = "camelCase")]`, Rust will look for `"sourceStep"` in JSON, which matches. ✅

## Action Items

### 1. Test with Debug Logging ✅ DONE
- Build completed with debug logging
- Ready to test

### 2. Capture Actual JSON Being Sent
When testing, capture the exact JSON string that fails to parse.

### 3. Fix Based on Debug Output
Common fixes:
- **If JSON has extra fields**: Add `#[serde(deny_unknown_fields)]` to catch this early OR allow extra fields
- **If field names don't match**: Adjust frontend or add `#[serde(rename = "...")]` attributes
- **If types don't match**: Convert types in frontend before stringifying

### 4. Add Validation at API Layer
In `create_run_step()` and `update_run_step()`, validate that typed steps have valid `config_json`:

```rust
// If step_type is a typed step, try to parse config_json NOW
if step_type == "ingest" || step_type == "summarize" || step_type == "prompt" {
    if let Some(ref config_str) = config_json {
        // Try to parse - fail fast with clear error
        serde_json::from_str::<StepConfig>(config_str)
            .map_err(|e| anyhow!("Invalid config_json for typed step: {}", e))?;
    } else {
        return Err(anyhow!("Typed step {} requires config_json", step_type));
    }
}
```

This will catch issues at creation time instead of execution time.

### 5. Consider Robustness Improvements

#### A. Populate legacy fields for typed steps
When creating a typed step, also populate `model` and `prompt` fields:

```typescript
// For prompt type
await onSubmit({
  stepType: "prompt",
  model: cleanedModel,     // ✅ Also set at top level
  prompt: cleanedPrompt,   // ✅ Also set at top level
  configJson,
  // ...
});
```

This makes the system more resilient - if typed execution fails, legacy can work as backup.

#### B. Make legacy execution smarter
Update `execute_checkpoint()` to check `config_json` first:

```rust
fn execute_checkpoint(
    config: &RunStep,
    run_seed: u64,
    llm_client: &dyn LlmClient,
) -> anyhow::Result<NodeExecution> {
    // Try to extract model/prompt from config_json if available
    if let Some(ref config_json_str) = config.config_json {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(config_json_str) {
            let model = value.get("model")
                .and_then(|v| v.as_str())
                .or(config.model.as_deref());
            let prompt = value.get("prompt")
                .and_then(|v| v.as_str())
                .or(config.prompt.as_deref());

            // Now use extracted values...
        }
    }

    // ... rest of function
}
```

## Summary

The error "LLM step missing prompt" occurs because:
1. Typed steps store all data in `config_json`
2. They don't populate legacy `model` and `prompt` fields
3. When typed execution fails (JSON parsing fails), system falls back to legacy
4. Legacy execution expects `model` and `prompt` to be populated
5. Since they're NULL, it throws the error

**Solution**: Find out why JSON parsing is failing using the debug logs, then fix the JSON structure or deserialization logic.

**Next Steps**:
1. Run app with debug logging
2. Execute a test workflow
3. Capture the exact parse error
4. Fix based on the specific error message
