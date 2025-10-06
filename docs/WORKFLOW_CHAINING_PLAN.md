# Workflow Step Chaining Implementation Plan

## Overview

This document outlines the plan to enable multi-step workflows where outputs from one step (document ingestion or LLM execution) can be used as inputs to subsequent steps, while maintaining Intelexta's core principles of verifiability, reproducibility, and signed provenance.

## Current State Analysis

### What Works Today

1. **Sequential Step Execution**:
   - Orchestrator executes steps in order defined by `order_index`
   - Each step produces an immutable checkpoint with signed provenance
   - Two step types: `llm_prompt` and `document_ingestion`

2. **Checkpoint Persistence**:
   - Stores: `prompt_payload`, `output_payload`, `outputs_sha256`, `signature`
   - Output payloads may be truncated for display (especially large documents)
   - Full canonical documents from ingestion stored in checkpoint output

3. **Current Isolation**:
   - Each step executes independently from static configuration
   - No data flow between steps during execution
   - Cannot reference prior step outputs in subsequent prompts

### Current Limitations

1. **No Inter-Step Data Flow**:
   - Document ingestion output cannot feed into LLM analysis
   - Multi-step reasoning requires manual copy/paste
   - Cannot build complex pipelines (extract → analyze → summarize)

2. **Replay/Verification Challenges**:
   - If users manually copy outputs between steps, replays won't match
   - CAR exports wouldn't capture the data dependencies
   - Third-party verification would fail

3. **UX Friction**:
   - Must execute Run A, inspect checkpoint, copy output, create Run B
   - No visual indication of step dependencies
   - Error-prone manual data transfer

## Design Goals

### 1. Maintain Verifiability
- All data bindings must be **explicit** in run configuration
- Replays must deterministically reproduce the same data flow
- CAR exports must be self-contained and verifiable by third parties

### 2. Two Integration Patterns

#### Pattern A: Cross-Run References (Future Enhancement)
```
Run A (Document Ingestion)
  ↓ [Execution creates immutable checkpoint]
  ↓
Run B (LLM Analysis)
  └─ References Run A's checkpoint as static input
```

**Use Cases**:
- Reference a single, well-known document across multiple analysis runs
- Build a "document library" of ingested files
- Compare multiple analysis approaches on same input

**Implementation Complexity**: Medium-High
- Requires checkpoint discovery/selection UI
- Cross-run dependency tracking
- Ensures referenced checkpoints are available during replay

#### Pattern B: Intra-Run Bindings (Primary Focus) ⭐
```
Run C (Multi-Step Pipeline)
  Step 1: Document Ingestion → produces canonical_doc
  Step 2: LLM Summary → consumes canonical_doc.cleaned_text
  Step 3: LLM Questions → consumes step2.output + canonical_doc.metadata
```

**Use Cases**:
- End-to-end document processing pipelines
- Multi-stage analysis (extract → classify → summarize)
- Progressive refinement workflows

**Implementation Complexity**: Medium
- Single execution context maintains state
- Natural sequential dependencies
- Self-contained in single CAR export

### 3. CAR Export Compatibility

Exported CARs must include:
- Binding metadata showing step dependencies
- All intermediate outputs (not truncated)
- Rendered prompts (after binding resolution)
- Deterministic replay instructions

Third-party verifiers should be able to:
1. Parse binding graph from CAR
2. Re-execute steps in order
3. Verify each checkpoint matches expected hash
4. Confirm signatures on checkpoint chain

## Proposed Architecture

### Database Schema Changes

#### Migration V12: Add Step Input Bindings

```sql
-- V12__add_step_input_bindings.sql

ALTER TABLE run_steps
ADD COLUMN input_bindings_json TEXT DEFAULT '[]';

-- Populate existing rows
UPDATE run_steps SET input_bindings_json = '[]' WHERE input_bindings_json IS NULL;

-- Add index for lookup performance
CREATE INDEX IF NOT EXISTS idx_run_steps_bindings
ON run_steps(run_id, order_index);

-- Update schema comments
COMMENT ON COLUMN run_steps.input_bindings_json IS
'JSON array of StepInputBinding: [{name, sourceStep, payloadType, jsonPointer?, fallback?}]';
```

#### New Checkpoint Fields

```sql
-- Consider adding to checkpoints table (or separate artifacts table)
ALTER TABLE checkpoints
ADD COLUMN rendered_prompt TEXT,  -- Prompt after binding resolution
ADD COLUMN artifact_path TEXT;    -- Path to full output artifact (for large payloads)
```

### Rust Data Structures

```rust
// src-tauri/src/orchestrator.rs

/// Binding to upstream step output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepInputBinding {
    /// Name of binding (used in template as {{name}})
    pub name: String,

    /// Order index of source step (must be < current step)
    pub source_step: usize,

    /// What to extract from source checkpoint
    pub payload_type: BindingPayloadType,

    /// Optional JSON pointer (e.g., "/metadata/title")
    pub json_pointer: Option<String>,

    /// Fallback value if resolution fails
    pub fallback: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingPayloadType {
    /// Full prompt text
    Prompt,

    /// Full output text/JSON
    Output,

    /// SHA-256 hash
    OutputHash,

    /// For document ingestion: full canonical document
    CanonicalDocument,

    /// Specific metadata field
    Metadata,
}

/// Resolved binding value
#[derive(Debug, Clone)]
pub enum ResolvedValue {
    String(String),
    Json(serde_json::Value),
}

/// Extended step template with bindings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunStepTemplate {
    pub order_index: usize,
    pub step_type: String,
    pub checkpoint_type: String,
    pub config_json: String,

    // NEW: Input bindings
    #[serde(default)]
    pub input_bindings: Vec<StepInputBinding>,
}
```

### Binding Resolution Algorithm

```rust
// src-tauri/src/orchestrator.rs

/// Resolve bindings for a step
fn resolve_bindings(
    bindings: &[StepInputBinding],
    prior_executions: &HashMap<usize, NodeExecution>,
) -> Result<HashMap<String, ResolvedValue>> {
    let mut resolved = HashMap::new();

    for binding in bindings {
        // Validate source step exists and is prior
        let source = prior_executions
            .get(&binding.source_step)
            .ok_or_else(|| anyhow!("Binding '{}' references non-existent step {}",
                                   binding.name, binding.source_step))?;

        // Extract value based on payload type
        let value = match &binding.payload_type {
            BindingPayloadType::Prompt => {
                ResolvedValue::String(source.prompt_payload.clone())
            }

            BindingPayloadType::Output => {
                // Load full output, not truncated preview
                if let Some(artifact_path) = &source.artifact_path {
                    let full_output = fs::read_to_string(artifact_path)?;
                    ResolvedValue::String(full_output)
                } else {
                    ResolvedValue::String(source.output_payload.clone())
                }
            }

            BindingPayloadType::OutputHash => {
                ResolvedValue::String(source.outputs_sha256.clone())
            }

            BindingPayloadType::CanonicalDocument => {
                // Parse output as CanonicalDocument
                let doc: CanonicalDocument = serde_json::from_str(&source.output_payload)?;
                ResolvedValue::Json(serde_json::to_value(doc)?)
            }

            BindingPayloadType::Metadata => {
                // Extract metadata subset
                let doc: CanonicalDocument = serde_json::from_str(&source.output_payload)?;
                ResolvedValue::Json(serde_json::to_value(doc.metadata)?)
            }
        };

        // Apply JSON pointer if specified
        let final_value = if let Some(pointer) = &binding.json_pointer {
            apply_json_pointer(value, pointer)?
        } else {
            value
        };

        resolved.insert(binding.name.clone(), final_value);
    }

    Ok(resolved)
}

/// Render template with bindings
fn render_prompt_template(
    template: &str,
    bindings: &HashMap<String, ResolvedValue>,
) -> Result<String> {
    let mut rendered = template.to_string();

    // Replace {{bindingName}} with resolved values
    for (name, value) in bindings {
        let placeholder = format!("{{{{{}}}}}", name);
        let replacement = match value {
            ResolvedValue::String(s) => s.clone(),
            ResolvedValue::Json(j) => serde_json::to_string_pretty(j)?,
        };

        rendered = rendered.replace(&placeholder, &replacement);
    }

    // Validate no unresolved bindings remain
    if rendered.contains("{{") {
        return Err(anyhow!("Template contains unresolved bindings"));
    }

    Ok(rendered)
}
```

### Orchestrator Integration

```rust
// src-tauri/src/orchestrator.rs - Modified start_run_with_client

pub async fn start_run_with_client(
    conn: &Connection,
    run_id: &str,
    client: &dyn OllamaClient,
) -> Result<()> {
    // Load run steps with bindings
    let steps: Vec<RunStepTemplate> = load_run_steps_with_bindings(conn, run_id)?;

    // Track completed executions for binding resolution
    let mut prior_executions: HashMap<usize, NodeExecution> = HashMap::new();

    for (index, step) in steps.iter().enumerate() {
        // Resolve input bindings
        let bindings = resolve_bindings(&step.input_bindings, &prior_executions)?;

        // Execute step with resolved bindings
        let execution = match step.step_type.as_str() {
            "llm_prompt" => {
                // Render prompt template
                let config: LlmPromptConfig = serde_json::from_str(&step.config_json)?;
                let rendered_prompt = render_prompt_template(&config.prompt, &bindings)?;

                // Execute with rendered prompt
                execute_llm_checkpoint_with_rendered_prompt(
                    conn,
                    run_id,
                    step,
                    &rendered_prompt,
                    client,
                ).await?
            }

            "document_ingestion" => {
                // Document ingestion doesn't use bindings (it's typically first step)
                // But could theoretically reference another document
                execute_document_ingestion_checkpoint(conn, run_id, step).await?
            }

            _ => return Err(anyhow!("Unknown step type: {}", step.step_type)),
        };

        // Store execution for subsequent bindings
        prior_executions.insert(step.order_index, execution);
    }

    Ok(())
}
```

### Frontend Data Structures

```typescript
// app/src/lib/api.ts

export interface StepInputBinding {
  name: string;
  sourceStep: number;
  payloadType: 'prompt' | 'output' | 'output_hash' | 'canonical_document' | 'metadata';
  jsonPointer?: string;
  fallback?: string;
}

export interface RunStepConfig {
  stepType: 'llm_prompt' | 'document_ingestion';
  checkpointType: string;

  // For llm_prompt
  model?: string;
  prompt?: string;
  tokenBudget?: number;
  proofMode?: 'exact' | 'concordant';
  epsilon?: number;

  // For document_ingestion
  sourcePath?: string;
  format?: string;
  privacyStatus?: string;

  // NEW: Input bindings
  inputBindings?: StepInputBinding[];
}

export interface BindingSource {
  stepIndex: number;
  stepType: string;
  label: string;  // e.g., "Step 1: Document Ingestion (paper.pdf)"
  availablePayloads: BindingPayloadType[];
}
```

### UI Components

#### 1. StepInputBindingsEditor Component

```tsx
// app/src/components/StepInputBindingsEditor.tsx

interface StepInputBindingsEditorProps {
  bindings: StepInputBinding[];
  onChange: (bindings: StepInputBinding[]) => void;
  availableSources: BindingSource[];  // Upstream steps
}

export function StepInputBindingsEditor({
  bindings,
  onChange,
  availableSources
}: StepInputBindingsEditorProps) {
  const handleAddBinding = () => {
    onChange([
      ...bindings,
      {
        name: `binding${bindings.length + 1}`,
        sourceStep: availableSources[0]?.stepIndex ?? 0,
        payloadType: 'output',
      }
    ]);
  };

  const handleRemoveBinding = (index: number) => {
    onChange(bindings.filter((_, i) => i !== index));
  };

  const handleUpdateBinding = (index: number, updated: StepInputBinding) => {
    onChange(bindings.map((b, i) => i === index ? updated : b));
  };

  return (
    <div className="bindings-editor">
      <h4>Input Bindings</h4>
      <p className="help-text">
        Connect outputs from previous steps to this step's prompt using {'{{'}{'{'}binding_name{'}}'}
      </p>

      {bindings.map((binding, index) => (
        <BindingRow
          key={index}
          binding={binding}
          availableSources={availableSources}
          onUpdate={(updated) => handleUpdateBinding(index, updated)}
          onRemove={() => handleRemoveBinding(index)}
        />
      ))}

      <button onClick={handleAddBinding} disabled={availableSources.length === 0}>
        + Add Binding
      </button>

      {availableSources.length === 0 && (
        <p className="warning">Add previous steps first to enable bindings</p>
      )}
    </div>
  );
}
```

#### 2. Prompt Template Assistant

```tsx
// app/src/components/PromptTemplateEditor.tsx

interface PromptTemplateEditorProps {
  prompt: string;
  onChange: (prompt: string) => void;
  bindings: StepInputBinding[];
}

export function PromptTemplateEditor({
  prompt,
  onChange,
  bindings
}: PromptTemplateEditorProps) {
  const insertBinding = (bindingName: string) => {
    const insertion = `{{${bindingName}}}`;
    // Insert at cursor position or append
    onChange(prompt + insertion);
  };

  // Highlight unresolved {{tokens}} in red
  const validateTemplate = (text: string): string[] => {
    const tokenRegex = /\{\{(\w+)\}\}/g;
    const tokens = [...text.matchAll(tokenRegex)].map(m => m[1]);
    const boundNames = new Set(bindings.map(b => b.name));
    return tokens.filter(t => !boundNames.has(t));
  };

  const unresolvedTokens = validateTemplate(prompt);

  return (
    <div>
      <label>Prompt Template</label>
      <textarea
        value={prompt}
        onChange={(e) => onChange(e.target.value)}
        rows={10}
      />

      {bindings.length > 0 && (
        <div className="binding-buttons">
          <span>Insert binding:</span>
          {bindings.map(b => (
            <button
              key={b.name}
              onClick={() => insertBinding(b.name)}
              title={`Insert {{${b.name}}}`}
            >
              {b.name}
            </button>
          ))}
        </div>
      )}

      {unresolvedTokens.length > 0 && (
        <div className="validation-error">
          Unresolved bindings: {unresolvedTokens.join(', ')}
        </div>
      )}
    </div>
  );
}
```

#### 3. Prefill from Execution

```tsx
// app/src/components/EditorPanel.tsx

const handlePrefillFromExecution = async (checkpointId: string) => {
  const checkpoint = await getCheckpointDetails(checkpointId);

  // Update prompt text (static copy, not binding)
  setPrompt(checkpoint.output_payload);

  // Show notification
  showToast('Prompt prefilled from checkpoint. This is static text, not a live binding.');
};
```

## Implementation Phases

### Phase 1: Foundation (Week 1-2)
**Goal**: Database schema and basic binding persistence

**Tasks**:
1. ✅ Create migration V12 for `input_bindings_json` column
2. ✅ Add Rust structs: `StepInputBinding`, `BindingPayloadType`, `ResolvedValue`
3. ✅ Update `RunStepTemplate` to include `input_bindings` field
4. ✅ Extend create/update/list API endpoints to serialize/deserialize bindings
5. ✅ Update TypeScript types in `api.ts`
6. ✅ Add Rust unit tests for binding round-trip
7. ✅ Update portability (export/import) to include bindings

**Acceptance Criteria**:
- Can create/update step with `input_bindings` JSON
- Bindings persist across app restart
- Export/import preserves binding configuration

### Phase 2: Execution Engine (Week 3-4)
**Goal**: Resolve and render bindings during execution

**Tasks**:
1. ✅ Implement `resolve_bindings()` function
2. ✅ Implement `render_prompt_template()` function
3. ✅ Implement `apply_json_pointer()` helper
4. ✅ Modify `start_run_with_client()` to track prior executions
5. ✅ Add artifact storage for large outputs (canonical documents)
6. ✅ Store `rendered_prompt` in checkpoints
7. ✅ Add validation: bindings must reference prior steps only
8. ✅ Add Rust tests for:
   - Document → LLM binding
   - LLM → LLM binding
   - JSON pointer extraction
   - Error cases (invalid step reference, missing fields)

**Acceptance Criteria**:
- Can execute run with step 2 binding to step 1 output
- Rendered prompts stored in checkpoints
- Replay produces identical bindings and outputs
- Validation prevents forward references

### Phase 3: UI - Basic Binding Editor (Week 5-6)
**Goal**: Create/edit bindings in workflow builder

**Tasks**:
1. ✅ Create `StepInputBindingsEditor` component
2. ✅ Create `BindingRow` component
3. ✅ Implement `getAvailableBindingSources()` selector
4. ✅ Integrate into `CheckpointEditor.tsx` for LLM steps
5. ✅ Add binding name/source/type dropdowns
6. ✅ Update `createRunStep()` and `updateRunStep()` to include bindings
7. ✅ Add frontend validation (no forward references)
8. ✅ Add visual indicator in step list showing dependencies

**Acceptance Criteria**:
- Can add/edit/remove bindings in UI
- Dropdowns show only valid upstream steps
- Changes persist to backend
- Visual dependency indicators in workflow builder

### Phase 4: UI - Template Assistant (Week 7)
**Goal**: Help users create prompt templates

**Tasks**:
1. ✅ Create `PromptTemplateEditor` component
2. ✅ Add "Insert Binding" buttons for each binding
3. ✅ Implement template validation (highlight unresolved tokens)
4. ✅ Add syntax highlighting for `{{tokens}}`
5. ✅ Add "Prefill from Execution" action
6. ✅ Show binding preview (what value will be substituted)

**Acceptance Criteria**:
- Can click button to insert `{{bindingName}}` into prompt
- Validation shows errors for unresolved tokens
- Prefill action works without creating bindings

### Phase 5: CAR Export & Verification (Week 8)
**Goal**: Ensure chained workflows are verifiable

**Tasks**:
1. ✅ Update CAR schema to include binding metadata
2. ✅ Include all intermediate artifacts in CAR
3. ✅ Include rendered prompts in checkpoints
4. ✅ Update `intelexta-verify` CLI to:
   - Parse binding graph
   - Resolve bindings during replay
   - Verify each checkpoint hash matches
5. ✅ Add documentation for CAR verification with bindings

**Acceptance Criteria**:
- CAR export includes complete binding metadata
- Third-party verifier can replay chained workflow
- All checkpoint hashes match original execution

### Phase 6: Advanced Features (Week 9-10)
**Goal**: JSON pointers, fallbacks, cross-run references

**Tasks**:
1. ✅ Implement JSON pointer extraction
2. ✅ Add fallback value support
3. ✅ Add UI for JSON pointer specification
4. ✅ Add UI for fallback values
5. 🔄 Cross-run checkpoint references (future)

**Acceptance Criteria**:
- Can extract nested JSON fields via pointer
- Fallback values used when extraction fails
- (Future) Can reference checkpoint from different run

## Example Use Cases

### Use Case 1: Document Analysis Pipeline

```
Step 1: Document Ingestion
  - Type: document_ingestion
  - File: research_paper.pdf
  - Format: pdf
  - Output: CanonicalDocument with full text

Step 2: Generate Summary
  - Type: llm_prompt
  - Model: llama3.2
  - Bindings:
    - name: "document_text"
      sourceStep: 0
      payloadType: "canonical_document"
      jsonPointer: "/cleaned_text_with_markdown_structure"
  - Prompt: "Summarize the following research paper:\n\n{{document_text}}"

Step 3: Extract Key Findings
  - Type: llm_prompt
  - Model: llama3.2
  - Bindings:
    - name: "summary"
      sourceStep: 1
      payloadType: "output"
    - name: "doc_title"
      sourceStep: 0
      payloadType: "canonical_document"
      jsonPointer: "/metadata/title"
  - Prompt: "Based on this summary of '{{doc_title}}':\n\n{{summary}}\n\nList the 5 key findings."
```

### Use Case 2: Multi-Stage Reasoning

```
Step 1: Initial Analysis
  - Type: llm_prompt
  - Prompt: "Analyze the following code for security issues: [code here]"

Step 2: Deeper Investigation
  - Type: llm_prompt
  - Bindings:
    - name: "initial_findings"
      sourceStep: 0
      payloadType: "output"
  - Prompt: "You identified these issues:\n{{initial_findings}}\n\nFor each issue, provide a CVE reference if applicable."

Step 3: Remediation Plan
  - Type: llm_prompt
  - Bindings:
    - name: "issues"
      sourceStep: 0
      payloadType: "output"
    - name: "cves"
      sourceStep: 1
      payloadType: "output"
  - Prompt: "Given these issues:\n{{issues}}\n\nAnd these CVE references:\n{{cves}}\n\nGenerate a prioritized remediation plan."
```

## Testing Strategy

### Unit Tests (Rust)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binding_resolution() {
        let mut prior = HashMap::new();
        prior.insert(0, NodeExecution {
            order_index: 0,
            output_payload: "Hello World".to_string(),
            // ... other fields
        });

        let bindings = vec![
            StepInputBinding {
                name: "greeting".to_string(),
                source_step: 0,
                payload_type: BindingPayloadType::Output,
                json_pointer: None,
                fallback: None,
            }
        ];

        let resolved = resolve_bindings(&bindings, &prior).unwrap();
        assert_eq!(resolved.get("greeting").unwrap(), &ResolvedValue::String("Hello World".to_string()));
    }

    #[test]
    fn test_template_rendering() {
        let mut bindings = HashMap::new();
        bindings.insert("name".to_string(), ResolvedValue::String("Alice".to_string()));

        let template = "Hello {{name}}!";
        let rendered = render_prompt_template(template, &bindings).unwrap();

        assert_eq!(rendered, "Hello Alice!");
    }

    #[test]
    fn test_forward_reference_rejected() {
        let prior = HashMap::new();  // Empty

        let bindings = vec![
            StepInputBinding {
                name: "future".to_string(),
                source_step: 1,  // Step 1 doesn't exist yet
                payload_type: BindingPayloadType::Output,
                json_pointer: None,
                fallback: None,
            }
        ];

        let result = resolve_bindings(&bindings, &prior);
        assert!(result.is_err());
    }
}
```

### Integration Tests (Manual)

1. **Document → LLM Pipeline**:
   - Create run with 2 steps
   - Step 1: Ingest test PDF
   - Step 2: Summarize with binding to step 1
   - Execute and verify summary includes document content
   - Export CAR and verify in `intelexta-verify`

2. **Three-Step Reasoning Chain**:
   - Step 1: Generate ideas
   - Step 2: Critique ideas (binding to step 1)
   - Step 3: Synthesize best idea (bindings to steps 1 and 2)
   - Verify all bindings resolve correctly

3. **Error Handling**:
   - Try to bind to non-existent step → expect error
   - Try forward reference → expect error
   - Try JSON pointer on non-JSON output → expect fallback or error
   - Delete step that has dependent steps → expect warning

## Documentation Updates

### User Guide: Creating Multi-Step Workflows

Create new file: `docs/MULTI_STEP_WORKFLOWS.md`

```markdown
# Multi-Step Workflows

## Overview

Intelexta supports chaining multiple steps together where later steps can use outputs from earlier steps as inputs. This enables complex document processing and reasoning pipelines while maintaining full verifiability.

## Creating a Chained Workflow

### Step 1: Add Steps in Order

Add your steps in the Workflow Builder. Remember:
- Steps execute in order (Step 1, then Step 2, then Step 3, etc.)
- Later steps can reference earlier steps
- You cannot reference future steps

### Step 2: Configure Input Bindings

When editing an LLM step:
1. Scroll to "Input Bindings" section
2. Click "+ Add Binding"
3. Configure:
   - **Name**: How you'll reference this in your prompt (e.g., "document")
   - **Source Step**: Which previous step to get data from
   - **Payload Type**: What data to extract
     - Output: The full text output
     - Canonical Document: For document ingestion steps
     - Metadata: Just the metadata fields
     - Hash: The SHA-256 hash
4. Optional: Specify JSON Pointer to extract nested fields

### Step 3: Use Bindings in Prompt Template

In your prompt, reference bindings using `{{name}}`:

```
Analyze this document:

Title: {{document.title}}
Content: {{document.text}}

Provide a summary.
```

The template editor will:
- Show available bindings as buttons you can click to insert
- Highlight unresolved `{{tokens}}` in red
- Validate before saving

### Step 4: Execute and Verify

Execute your run. Each checkpoint will show:
- Original template (with `{{bindings}}`)
- Rendered prompt (with actual values substituted)
- Output
- Signature

## Example Workflows

[Include examples from above]

## Best Practices

1. **Name Bindings Clearly**: Use descriptive names like `source_document` not `doc1`
2. **Keep Steps Focused**: Each step should do one thing well
3. **Test Incrementally**: Execute after adding each step to verify bindings work
4. **Document Intent**: Use step names and checkpoint types to explain workflow
5. **Consider Replay**: Remember that replays will re-execute all steps with same bindings

## Troubleshooting

**"Unresolved binding" error**: Your prompt uses `{{name}}` but no binding with that name exists
**"Invalid source step" error**: You're referencing a step that doesn't exist or comes later
**"JSON pointer failed" error**: The JSON path you specified doesn't exist in the source
```

## Migration Path

### For Existing Users

1. **No Breaking Changes**: Existing runs without bindings continue to work
2. **Opt-In**: Bindings are optional, default to empty array `[]`
3. **Gradual Adoption**: Can add bindings to new steps while keeping old steps as-is
4. **CAR Compatibility**: Old CARs remain valid, new CARs include binding metadata

### Database Migration

The V12 migration is non-destructive:
- Adds new column with default value
- Updates existing rows to `'[]'`
- No data loss
- Backward compatible

## Future Enhancements

### Cross-Run References (V2)

Allow referencing checkpoints from other runs:

```typescript
{
  name: "reference_doc",
  sourceRun: "run-abc-123",
  sourceStep: 0,
  payloadType: "canonical_document"
}
```

**Use Cases**:
- Maintain document library
- Compare analysis approaches
- Build knowledge graphs

**Challenges**:
- Dependency management across runs
- Ensure referenced runs don't get deleted
- CAR export must include referenced checkpoints
- Replay complexity

### Visual Workflow Builder (V3)

Graph-based UI showing:
- Nodes = Steps
- Edges = Bindings
- Drag-and-drop step creation
- Visual dependency validation

### Conditional Execution (V4)

Execute steps conditionally based on prior outputs:

```typescript
{
  condition: "{{step1.confidence}} > 0.8",
  thenStep: 2,
  elseStep: 3
}
```

## Security Considerations

### Prompt Injection Risks

Bindings could enable prompt injection if:
- User-controlled input flows to LLM prompts
- No sanitization of binding values

**Mitigation**:
1. Document best practices for sanitization
2. Consider adding `sanitize: true` flag to bindings
3. Warn when binding user-generated content
4. Future: Automatic prompt injection detection

### Information Disclosure

Bindings might inadvertently leak sensitive data:
- Document metadata in prompts
- Prior step outputs visible in templates

**Mitigation**:
1. Privacy status propagates through bindings
2. Warn if binding to "private" step from "public" step
3. CAR exports respect privacy settings

## Performance Considerations

### Large Documents

Document ingestion outputs can be large (100KB+):
- Store full output as artifact file, not in checkpoint table
- Load lazily during binding resolution
- Consider compression for storage

### Memory Usage

During execution, maintain map of all prior outputs:
- Could grow large for 50+ step workflows
- Consider streaming or pagination for huge workflows
- Future: Spill to disk if memory constrained

### Execution Time

Each binding resolution adds latency:
- JSON parsing
- File I/O for artifacts
- Template rendering

**Optimization**:
- Cache parsed JSON
- Parallel binding resolution where possible
- Pre-validate bindings before execution starts

## Open Questions

1. **Should bindings support transformations?**
   - Example: Lowercase, trim, truncate
   - Pro: More flexible
   - Con: Adds complexity, harder to verify

2. **How to handle binding cycles?**
   - Not possible with current design (only backward references)
   - But cross-run references could create cycles
   - Need cycle detection algorithm

3. **Maximum binding depth?**
   - Should there be a limit on chain length?
   - Long chains harder to understand and debug
   - Consider warning at 10+ steps

4. **Binding versioning?**
   - If we change binding schema, how to handle old runs?
   - Version bindings separately from run schema?

5. **Partial execution resume?**
   - If step 3 of 5 fails, can we resume at step 3?
   - Bindings complicate this (need to restore prior state)
   - Future enhancement

## Summary

This feature enables powerful multi-step workflows while maintaining Intelexta's core principles:

✅ **Verifiable**: All bindings explicit in configuration
✅ **Reproducible**: Replays deterministically resolve same bindings
✅ **Portable**: CARs include binding metadata for third-party verification
✅ **Auditable**: Rendered prompts and bindings stored in checkpoints

Implementation follows "workflow definition → deterministic execution record" philosophy, ensuring that chained workflows remain trustworthy and verifiable.
