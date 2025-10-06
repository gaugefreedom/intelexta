# Sprint 2C Implementation Guide: Typed Step System & Optional Chaining

## Overview

This sprint introduces three explicit step types (ingest, summarize, prompt) with **optional** step chaining. The key design principle is that steps can work both independently AND in chains, allowing:

1. **Current behavior continues**: Single-step or multi-step workflows without linking
2. **New chained workflows**: Steps can reference previous step outputs
3. **Incremental adoption**: Users can show/demo before chaining is complete

## Design Principles

### 1. Optional, Not Mandatory
```
✅ Allowed: Single ingest step (standalone)
✅ Allowed: Ingest + Prompt (both standalone, no linking)
✅ Allowed: Ingest → Summarize (linked via source_step)
```

### 2. Backward Compatible
- Existing `document_ingestion` → becomes `ingest`
- Existing `llm_prompt` → becomes `prompt`
- No data migration needed (starting with clean database)

### 3. Clear Step Types
- `ingest`: Extract document from filesystem → CanonicalDocument JSON
- `summarize`: Take document/text → LLM summary → summary text
- `prompt`: Custom LLM prompt → response text

## Phase 1: Backend Implementation (Week 1-2)

### Step 1: Define Step Configuration Types

**File**: `src-tauri/src/orchestrator.rs`

Add these type definitions near the top of the file:

```rust
use serde::{Deserialize, Serialize};

/// Step configuration variants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "stepType")]
pub enum StepConfig {
    #[serde(rename = "ingest")]
    Ingest {
        source_path: String,
        format: String,  // "pdf", "latex", "txt", "docx"
        privacy_status: String,
    },

    #[serde(rename = "summarize")]
    Summarize {
        /// Optional: which step to summarize (None = error for now)
        source_step: Option<usize>,

        model: String,
        summary_type: String,  // "brief", "detailed", "academic", "custom"
        custom_instructions: Option<String>,

        // LLM execution options
        token_budget: Option<i32>,
        proof_mode: Option<String>,
        epsilon: Option<f64>,
    },

    #[serde(rename = "prompt")]
    Prompt {
        model: String,
        prompt: String,

        /// Optional: include output from this step in context
        use_output_from: Option<usize>,

        // LLM execution options
        token_budget: Option<i32>,
        proof_mode: Option<String>,
        epsilon: Option<f64>,
    },
}

/// Output from a step execution
#[derive(Debug, Clone)]
pub struct StepOutput {
    pub order_index: usize,
    pub step_type: String,
    pub output_text: String,
    pub output_json: Option<serde_json::Value>,
    pub outputs_sha256: String,
}
```

### Step 2: Update Database Schema

**File**: `src-tauri/src/store/schema.sql`

The schema already has `step_type` column. Just verify it exists:

```sql
-- run_steps table should have:
CREATE TABLE IF NOT EXISTS run_steps (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    order_index INTEGER NOT NULL,
    checkpoint_type TEXT NOT NULL,
    step_type TEXT NOT NULL,  -- 'ingest', 'summarize', 'prompt'
    config_json TEXT NOT NULL,  -- StepConfig serialized
    created_at TEXT NOT NULL,
    FOREIGN KEY (run_id) REFERENCES runs(id) ON DELETE CASCADE
);
```

No changes needed if this already exists!

### Step 3: Add Helper Functions

**File**: `src-tauri/src/orchestrator.rs`

Add these helper functions:

```rust
/// Extract text content from step output
fn extract_text_from_output(output: &StepOutput) -> Result<String> {
    // If output is CanonicalDocument JSON, extract cleaned text
    if let Some(json) = &output.output_json {
        if let Some(cleaned_text) = json.get("cleaned_text_with_markdown_structure") {
            if let Some(text) = cleaned_text.as_str() {
                return Ok(text.to_string());
            }
        }
    }

    // Otherwise just use the text output
    Ok(output.output_text.clone())
}

/// Build prompt for summarization
fn build_summary_prompt(
    source: &StepOutput,
    config: &StepConfig,
) -> Result<String> {
    let (summary_type, custom_instructions) = match config {
        StepConfig::Summarize { summary_type, custom_instructions, .. } => {
            (summary_type.as_str(), custom_instructions.as_deref())
        }
        _ => return Err(anyhow!("Expected Summarize config")),
    };

    let base_prompt = match summary_type {
        "brief" => "Provide a brief 2-3 sentence summary of the following:\n\n",
        "detailed" => "Provide a comprehensive summary covering all main points of:\n\n",
        "academic" => "Provide an academic summary including methodology, findings, and conclusions of:\n\n",
        "custom" => custom_instructions.unwrap_or("Summarize the following:\n\n"),
        _ => "Summarize the following:\n\n",
    };

    let source_text = extract_text_from_output(source)?;

    Ok(format!("{}{}", base_prompt, source_text))
}

/// Build prompt with context from previous step
fn build_prompt_with_context(
    prompt: &str,
    source: &StepOutput,
) -> String {
    format!(
        "{}\n\n--- Context from previous step ---\n{}",
        prompt,
        source.output_text
    )
}
```

### Step 4: Update Orchestrator Execution

**File**: `src-tauri/src/orchestrator.rs`

Modify the `start_run_with_client` function:

```rust
pub async fn start_run_with_client(
    conn: &Connection,
    run_id: &str,
    client: &dyn OllamaClient,
) -> Result<()> {
    // Load run steps
    let steps = load_run_steps(conn, run_id)?;

    // Track outputs for chaining
    let mut prior_outputs: HashMap<usize, StepOutput> = HashMap::new();

    for step in steps {
        // Parse config
        let config: StepConfig = serde_json::from_str(&step.config_json)
            .context("Failed to parse step config")?;

        // Execute based on type
        let output = match config {
            StepConfig::Ingest { source_path, format, privacy_status } => {
                // Execute document ingestion (existing code)
                execute_ingest_step(
                    conn,
                    run_id,
                    &step,
                    &source_path,
                    &format,
                    &privacy_status,
                ).await?
            }

            StepConfig::Summarize { source_step, model, token_budget, proof_mode, epsilon, .. } => {
                // Check if source is specified
                let source_idx = source_step
                    .ok_or_else(|| anyhow!("Summarize step requires a source_step"))?;

                // Get source output
                let source = prior_outputs
                    .get(&source_idx)
                    .ok_or_else(|| anyhow!("Source step {} not found", source_idx))?;

                // Build summarization prompt
                let prompt = build_summary_prompt(source, &config)?;

                // Execute LLM
                execute_llm_step(
                    conn,
                    run_id,
                    &step,
                    &model,
                    &prompt,
                    token_budget,
                    proof_mode.as_deref(),
                    epsilon,
                    client,
                ).await?
            }

            StepConfig::Prompt { model, prompt, use_output_from, token_budget, proof_mode, epsilon } => {
                // Build final prompt (with or without context)
                let final_prompt = if let Some(source_idx) = use_output_from {
                    let source = prior_outputs
                        .get(&source_idx)
                        .ok_or_else(|| anyhow!("Source step {} not found", source_idx))?;

                    build_prompt_with_context(&prompt, source)
                } else {
                    prompt.clone()
                };

                // Execute LLM
                execute_llm_step(
                    conn,
                    run_id,
                    &step,
                    &model,
                    &final_prompt,
                    token_budget,
                    proof_mode.as_deref(),
                    epsilon,
                    client,
                ).await?
            }
        };

        // Store output for potential use by next steps
        prior_outputs.insert(step.order_index, output);
    }

    Ok(())
}
```

### Step 5: Implement Execution Functions

**File**: `src-tauri/src/orchestrator.rs`

Add these new execution functions:

```rust
/// Execute document ingestion step
async fn execute_ingest_step(
    conn: &Connection,
    run_id: &str,
    step: &RunStepTemplate,
    source_path: &str,
    format: &str,
    privacy_status: &str,
) -> Result<StepOutput> {
    // This is the existing execute_document_ingestion_checkpoint logic
    // Just refactored to return StepOutput

    let canonical_doc = match format {
        "pdf" => document_processing::process_pdf_to_canonical(source_path, Some(privacy_status.to_string()))?,
        "latex" | "tex" => document_processing::process_latex_to_canonical(source_path, Some(privacy_status.to_string()))?,
        "txt" => document_processing::process_txt_to_canonical(source_path, Some(privacy_status.to_string()))?,
        "docx" | "doc" => document_processing::process_docx_to_canonical(source_path, Some(privacy_status.to_string()))?,
        _ => return Err(anyhow!("Unsupported format: {}", format)),
    };

    let output_json = serde_json::to_string_pretty(&canonical_doc)?;
    let outputs_sha256 = hash_output(&output_json);

    // Create checkpoint (existing logic)
    create_checkpoint(conn, run_id, step, &output_json, &outputs_sha256)?;

    Ok(StepOutput {
        order_index: step.order_index,
        step_type: "ingest".to_string(),
        output_text: canonical_doc.cleaned_text_with_markdown_structure.clone(),
        output_json: Some(serde_json::to_value(canonical_doc)?),
        outputs_sha256,
    })
}

/// Execute LLM step (prompt or summarize)
async fn execute_llm_step(
    conn: &Connection,
    run_id: &str,
    step: &RunStepTemplate,
    model: &str,
    prompt: &str,
    token_budget: Option<i32>,
    proof_mode: Option<&str>,
    epsilon: Option<f64>,
    client: &dyn OllamaClient,
) -> Result<StepOutput> {
    // This is the existing LLM execution logic
    // Just refactored to accept rendered prompt and return StepOutput

    let response = client.generate(model, prompt).await?;
    let output_text = response.response;
    let outputs_sha256 = hash_output(&output_text);

    // Create checkpoint with usage tracking
    let usage = Usage {
        prompt_tokens: response.prompt_eval_count.unwrap_or(0),
        completion_tokens: response.eval_count.unwrap_or(0),
        total_tokens: response.prompt_eval_count.unwrap_or(0) + response.eval_count.unwrap_or(0),
    };

    create_checkpoint_with_usage(
        conn,
        run_id,
        step,
        prompt,
        &output_text,
        &outputs_sha256,
        &usage,
    )?;

    Ok(StepOutput {
        order_index: step.order_index,
        step_type: step.step_type.clone(),
        output_text: output_text.clone(),
        output_json: None,
        outputs_sha256,
    })
}
```

### Step 6: Update API Handlers

**File**: `src-tauri/src/api.rs`

Update the `create_run_step` and `update_run_step` commands to handle new step types:

```rust
#[tauri::command]
pub fn create_run_step(
    run_id: String,
    checkpoint_type: String,
    step_type: String,  // 'ingest', 'summarize', 'prompt'
    config_json: String,
    pool: State<'_, DbPool>,
) -> Result<String, Error> {
    let conn = pool.get()?;

    // Validate config_json matches step_type
    validate_step_config(&step_type, &config_json)?;

    // Get next order_index
    let order_index = get_next_order_index(&conn, &run_id)?;

    let step_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO run_steps (id, run_id, order_index, checkpoint_type, step_type, config_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![&step_id, &run_id, order_index, &checkpoint_type, &step_type, &config_json, &now],
    )?;

    Ok(step_id)
}

/// Validate that config_json is valid for step_type
fn validate_step_config(step_type: &str, config_json: &str) -> Result<(), Error> {
    let config: StepConfig = serde_json::from_str(config_json)
        .map_err(|e| Error::InvalidConfig(format!("Invalid config JSON: {}", e)))?;

    // Verify tag matches
    let expected_type = match config {
        StepConfig::Ingest { .. } => "ingest",
        StepConfig::Summarize { .. } => "summarize",
        StepConfig::Prompt { .. } => "prompt",
    };

    if step_type != expected_type {
        return Err(Error::InvalidConfig(format!(
            "Step type '{}' doesn't match config type '{}'",
            step_type, expected_type
        )));
    }

    Ok(())
}
```

## Phase 2: Frontend Implementation (Week 3-4)

### Step 1: Update TypeScript Types

**File**: `app/src/lib/api.ts`

Add these type definitions:

```typescript
export type StepConfig =
  | {
      stepType: 'ingest';
      sourcePath: string;
      format: 'pdf' | 'latex' | 'txt' | 'docx';
      privacyStatus: string;
    }
  | {
      stepType: 'summarize';
      sourceStep?: number;
      model: string;
      summaryType: 'brief' | 'detailed' | 'academic' | 'custom';
      customInstructions?: string;
      tokenBudget?: number;
      proofMode?: 'exact' | 'concordant';
      epsilon?: number;
    }
  | {
      stepType: 'prompt';
      model: string;
      prompt: string;
      useOutputFrom?: number;
      tokenBudget?: number;
      proofMode?: 'exact' | 'concordant';
      epsilon?: number;
    };

export interface RunStepTemplate {
  id: string;
  runId: string;
  orderIndex: number;
  checkpointType: string;
  stepType: 'ingest' | 'summarize' | 'prompt';
  configJson: string;  // Serialized StepConfig
  createdAt: string;
}

// Helper to parse config
export function parseStepConfig(configJson: string): StepConfig {
  return JSON.parse(configJson) as StepConfig;
}
```

### Step 2: Refactor CheckpointEditor

**File**: `app/src/components/CheckpointEditor.tsx`

Major refactor to support step types:

```tsx
import React from 'react';
import type { StepConfig } from '../lib/api';

export function CheckpointEditor({ ... }) {
  const [stepType, setStepType] = React.useState<'ingest' | 'summarize' | 'prompt'>('ingest');

  // Type-specific state
  const [ingestConfig, setIngestConfig] = React.useState<StepConfig>({
    stepType: 'ingest',
    sourcePath: '',
    format: 'pdf',
    privacyStatus: 'public',
  });

  const [summarizeConfig, setSummarizeConfig] = React.useState<StepConfig>({
    stepType: 'summarize',
    sourceStep: undefined,
    model: defaultModel,
    summaryType: 'brief',
    tokenBudget: 4000,
    proofMode: 'exact',
  });

  const [promptConfig, setPromptConfig] = React.useState<StepConfig>({
    stepType: 'prompt',
    model: defaultModel,
    prompt: '',
    useOutputFrom: undefined,
    tokenBudget: 4000,
    proofMode: 'exact',
  });

  const handleStepTypeChange = (newType: 'ingest' | 'summarize' | 'prompt') => {
    setStepType(newType);
  };

  const handleSubmit = async () => {
    const config = stepType === 'ingest' ? ingestConfig
                 : stepType === 'summarize' ? summarizeConfig
                 : promptConfig;

    const configJson = JSON.stringify(config);

    await createRunStep(runId, 'checkpoint', stepType, configJson);
  };

  return (
    <div className="checkpoint-editor">
      <label>Step Type</label>
      <select value={stepType} onChange={(e) => handleStepTypeChange(e.target.value as any)}>
        <option value="ingest">📄 Ingest Document</option>
        <option value="summarize">📝 Summarize</option>
        <option value="prompt">💬 Custom Prompt</option>
      </select>

      {stepType === 'ingest' && (
        <IngestStepForm
          config={ingestConfig}
          onChange={setIngestConfig}
        />
      )}

      {stepType === 'summarize' && (
        <SummarizeStepForm
          config={summarizeConfig}
          availableSteps={getAvailableSourceSteps()}
          onChange={setSummarizeConfig}
        />
      )}

      {stepType === 'prompt' && (
        <PromptStepForm
          config={promptConfig}
          availableSteps={getAllPriorSteps()}
          onChange={setPromptConfig}
        />
      )}

      <button onClick={handleSubmit}>Save Step</button>
    </div>
  );
}
```

### Step 3: Create IngestStepForm

**File**: `app/src/components/IngestStepForm.tsx`

```tsx
import React from 'react';
import { open } from '@tauri-apps/plugin-dialog';

interface IngestStepFormProps {
  config: {
    stepType: 'ingest';
    sourcePath: string;
    format: string;
    privacyStatus: string;
  };
  onChange: (config: any) => void;
}

export function IngestStepForm({ config, onChange }: IngestStepFormProps) {
  const handleBrowse = async () => {
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [
        { name: 'Documents', extensions: ['pdf', 'tex', 'latex', 'docx', 'txt'] },
      ],
    });

    if (selected) {
      onChange({ ...config, sourcePath: selected });
    }
  };

  return (
    <div className="ingest-form">
      <label>Document Path</label>
      <div style={{ display: 'flex', gap: '8px' }}>
        <input
          type="text"
          value={config.sourcePath}
          onChange={(e) => onChange({ ...config, sourcePath: e.target.value })}
        />
        <button onClick={handleBrowse}>Browse</button>
      </div>

      <label>Format</label>
      <select
        value={config.format}
        onChange={(e) => onChange({ ...config, format: e.target.value })}
      >
        <option value="pdf">PDF</option>
        <option value="latex">LaTeX</option>
        <option value="txt">Plain Text</option>
        <option value="docx">DOCX</option>
      </select>

      <label>Privacy Status</label>
      <select
        value={config.privacyStatus}
        onChange={(e) => onChange({ ...config, privacyStatus: e.target.value })}
      >
        <option value="public">Public</option>
        <option value="private">Private</option>
      </select>
    </div>
  );
}
```

### Step 4: Create SummarizeStepForm

**File**: `app/src/components/SummarizeStepForm.tsx`

```tsx
import React from 'react';

interface SummarizeStepFormProps {
  config: {
    stepType: 'summarize';
    sourceStep?: number;
    model: string;
    summaryType: string;
    customInstructions?: string;
    tokenBudget?: number;
    proofMode?: string;
    epsilon?: number;
  };
  availableSteps: Array<{ orderIndex: number; name: string }>;
  onChange: (config: any) => void;
}

export function SummarizeStepForm({ config, availableSteps, onChange }: SummarizeStepFormProps) {
  return (
    <div className="summarize-form">
      <label>Source Document Step</label>
      <select
        value={config.sourceStep ?? ''}
        onChange={(e) => onChange({
          ...config,
          sourceStep: e.target.value ? Number(e.target.value) : undefined
        })}
      >
        <option value="">Select source step...</option>
        {availableSteps.map((step) => (
          <option key={step.orderIndex} value={step.orderIndex}>
            Step {step.orderIndex + 1}: {step.name}
          </option>
        ))}
      </select>

      {config.sourceStep === undefined && (
        <p className="help-text warning">
          ⚠️ Summarize requires a source step. Select an ingest step above.
        </p>
      )}

      <label>Model</label>
      <select
        value={config.model}
        onChange={(e) => onChange({ ...config, model: e.target.value })}
      >
        <option value="llama3.2">Llama 3.2</option>
        <option value="claude-3-5-sonnet">Claude 3.5 Sonnet</option>
        {/* Add more models */}
      </select>

      <label>Summary Type</label>
      <select
        value={config.summaryType}
        onChange={(e) => onChange({ ...config, summaryType: e.target.value })}
      >
        <option value="brief">Brief (2-3 sentences)</option>
        <option value="detailed">Detailed</option>
        <option value="academic">Academic</option>
        <option value="custom">Custom Instructions</option>
      </select>

      {config.summaryType === 'custom' && (
        <>
          <label>Custom Instructions</label>
          <textarea
            value={config.customInstructions ?? ''}
            onChange={(e) => onChange({ ...config, customInstructions: e.target.value })}
            placeholder="E.g., Focus on methodology and key findings"
            rows={3}
          />
        </>
      )}

      {/* Token budget, proof mode, epsilon... */}
    </div>
  );
}
```

### Step 5: Update PromptStepForm

**File**: `app/src/components/PromptStepForm.tsx`

```tsx
import React from 'react';

interface PromptStepFormProps {
  config: {
    stepType: 'prompt';
    model: string;
    prompt: string;
    useOutputFrom?: number;
    tokenBudget?: number;
    proofMode?: string;
    epsilon?: number;
  };
  availableSteps: Array<{ orderIndex: number; name: string; stepType: string }>;
  onChange: (config: any) => void;
}

export function PromptStepForm({ config, availableSteps, onChange }: PromptStepFormProps) {
  return (
    <div className="prompt-form">
      <label>Prompt</label>
      <textarea
        value={config.prompt}
        onChange={(e) => onChange({ ...config, prompt: e.target.value })}
        rows={8}
        placeholder="Enter your prompt..."
      />

      <label>
        Use Output From (Optional)
        <span className="help-text">Include context from a previous step</span>
      </label>
      <select
        value={config.useOutputFrom ?? ''}
        onChange={(e) => onChange({
          ...config,
          useOutputFrom: e.target.value ? Number(e.target.value) : undefined
        })}
      >
        <option value="">None (standalone prompt)</option>
        {availableSteps.map((step) => (
          <option key={step.orderIndex} value={step.orderIndex}>
            Step {step.orderIndex + 1}: {step.stepType} ({step.name})
          </option>
        ))}
      </select>

      <label>Model</label>
      <select
        value={config.model}
        onChange={(e) => onChange({ ...config, model: e.target.value })}
      >
        <option value="llama3.2">Llama 3.2</option>
        <option value="claude-3-5-sonnet">Claude 3.5 Sonnet</option>
      </select>

      {/* Token budget, proof mode, epsilon... */}
    </div>
  );
}
```

### Step 6: Add Visual Indicators

**File**: `app/src/components/EditorPanel.tsx`

Update the step list to show type icons and chains:

```tsx
function StepListItem({ step, index }: { step: RunStepTemplate; index: number }) {
  const config = parseStepConfig(step.configJson);

  const icon = step.stepType === 'ingest' ? '📄'
             : step.stepType === 'summarize' ? '📝'
             : '💬';

  const sourceStep = step.stepType === 'summarize' && config.sourceStep !== undefined
                   ? config.sourceStep
                   : step.stepType === 'prompt' && config.useOutputFrom !== undefined
                   ? config.useOutputFrom
                   : null;

  return (
    <div className="step-item">
      {sourceStep !== null && (
        <span className="chain-arrow">↳</span>
      )}
      <span className="step-icon">{icon}</span>
      <span className="step-name">
        Step {index + 1}: {step.stepType}
        {sourceStep !== null && (
          <span className="source-ref"> (from Step {sourceStep + 1})</span>
        )}
      </span>
    </div>
  );
}
```

## Phase 3: Testing (Week 5)

### Test Plan

#### 1. Standalone Workflows

```bash
# Test 1: Single ingest step
1. Create run
2. Add ingest step (PDF file)
3. Execute
4. Verify checkpoint created with CanonicalDocument JSON
5. Verify can view in Inspector

# Test 2: Single prompt step
1. Create run
2. Add prompt step (no source)
3. Execute
4. Verify LLM response in checkpoint

# Test 3: Multiple independent steps
1. Create run
2. Add prompt step 1 (standalone)
3. Add prompt step 2 (standalone)
4. Execute
5. Verify both execute independently
```

#### 2. Chained Workflows

```bash
# Test 4: Ingest → Summarize
1. Create run
2. Add ingest step (test PDF)
3. Add summarize step (source_step = 0, type = brief)
4. Execute
5. Verify summary contains content from PDF
6. Check Inspector shows both checkpoints

# Test 5: Ingest → Prompt
1. Create run
2. Add ingest step
3. Add prompt step (use_output_from = 0)
4. Execute
5. Verify prompt received document context

# Test 6: Full pipeline
1. Create run
2. Add ingest step (document)
3. Add summarize step (source = step 0)
4. Add prompt step (use_output_from = 1, ask question about summary)
5. Execute
6. Verify all 3 steps complete with correct data flow
```

#### 3. Error Cases

```bash
# Test 7: Summarize without source
1. Create run
2. Add summarize step (no source_step)
3. Try to execute
4. Expect error: "Summarize step requires a source_step"

# Test 8: Invalid source reference
1. Create run
2. Add ingest step (index 0)
3. Add prompt step (use_output_from = 5)
4. Try to execute
5. Expect error: "Source step 5 not found"
```

### Manual Testing Checklist

- [ ] Can create ingest step with file picker
- [ ] Can create summarize step with source dropdown
- [ ] Can create prompt step with/without source
- [ ] Step icons display correctly (📄 📝 💬)
- [ ] Chain arrows show for linked steps
- [ ] Standalone steps work (no source refs)
- [ ] Chained steps work (with source refs)
- [ ] Error messages clear for invalid configs
- [ ] Can edit existing steps
- [ ] Can reorder steps (breaks references? show warning?)
- [ ] Can delete steps
- [ ] Export CAR works
- [ ] Replay works for chained workflows

## Summary

This implementation:

✅ **Maintains current functionality**: Standalone steps work exactly as before
✅ **Adds new capabilities**: Optional chaining for multi-step workflows
✅ **Clear types**: Three explicit step types with clear purposes
✅ **Incremental**: Can demo/use before chaining is fully polished
✅ **Extensible**: Easy to add more step types (compare, classify, etc.)

The key insight is making chaining **optional** via `Option<usize>` fields, so steps can work both ways without breaking existing workflows.
