# Week 1 Action Plan: Critical MVP Blockers

**Start Date**: 2025-10-07
**Goal**: Enable complete verification workflow
**Duration**: 5 working days

---

## Day 1-2: Save Step Outputs ⚠️ **HIGHEST PRIORITY**

### Problem
Checkpoints store hashes but not actual output text, making verification impossible.

### Current State
```rust
// In orchestrator.rs, checkpoints are created with:
output_payload: execution.output_payload.as_deref()  // ✅ Actually works!
```

### Action Items

**Task 1.1**: Verify outputs are being saved ✅ (Quick check)
```bash
# Run a test workflow
# Check database:
sqlite3 intelexta.db "SELECT id, kind, LENGTH(output_payload) FROM checkpoints ORDER BY id DESC LIMIT 5;"
```

**Expected**: Should see output_payload lengths > 0

**If not working**:
- Check `execute_document_ingestion_checkpoint()` returns `output_payload`
- Check `execute_llm_checkpoint()` returns `output_payload`
- Check `execute_stub_checkpoint()` returns `output_payload`

**Task 1.2**: Add output display to Inspector UI

**File**: `app/src/components/CheckpointDetailsPanel.tsx` (or create new component)

```typescript
// Add to checkpoint details view:
{checkpoint.outputPayload && (
  <div>
    <h4>Output</h4>
    <pre style={{
      background: '#1a1a1a',
      padding: '12px',
      borderRadius: '4px',
      overflow: 'auto',
      maxHeight: '400px'
    }}>
      {checkpoint.outputPayload}
    </pre>
    <button onClick={() => navigator.clipboard.writeText(checkpoint.outputPayload)}>
      Copy Output
    </button>
  </div>
)}
```

**Task 1.3**: Test output display
- Create workflow: Ingest → Summarize → Prompt
- Execute
- Open Inspector
- Verify each step shows its output

**Success Criteria**:
- ✅ Outputs visible in Inspector
- ✅ Can copy outputs
- ✅ Long outputs are scrollable

**Estimated Time**: 1-2 days

---

## Day 3: Add CAR Export Button

### Problem
No UI to export CARs, blocking the sharing use case.

### Action Items

**Task 3.1**: Add export button to Inspector

**File**: `app/src/components/Inspector.tsx` (or similar)

```typescript
import { exportCAR } from '../lib/api';

// In execution details view:
<button
  onClick={async () => {
    try {
      const carJson = await exportCAR(executionId);
      const blob = new Blob([JSON.stringify(carJson, null, 2)], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `execution-${executionId}.car.json`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (err) {
      console.error('Failed to export CAR:', err);
      alert('Failed to export CAR: ' + (err instanceof Error ? err.message : String(err)));
    }
  }}
>
  📦 Export CAR
</button>
```

**Task 3.2**: Verify CAR export function exists

**Check**: `src-tauri/src/car.rs` and `src-tauri/src/api.rs`

If missing, add:
```rust
// In api.rs
#[tauri::command]
pub fn export_car(
    execution_id: String,
    pool: State<'_, DbPool>,
) -> Result<car::Car, Error> {
    car::build_car(pool.inner(), &execution_id)
        .map_err(|err| Error::Api(err.to_string()))
}
```

**Task 3.3**: Test export
- Execute workflow
- Click "Export CAR"
- Verify JSON file downloads
- Open in text editor
- Check contains: signatures, checkpoints, policy, budgets

**Success Criteria**:
- ✅ Export button works
- ✅ CAR JSON is well-formed
- ✅ Contains all required fields
- ✅ File saves with correct name

**Estimated Time**: 1 day

---

## Day 4: Model Cost Configuration

### Problem
Hardcoded $0.01/1K tokens doesn't reflect real model costs.

### Action Items

**Task 4.1**: Create model configuration file

**File**: Create `models.json` in project root or config directory

```json
{
  "models": [
    {
      "id": "stub-model",
      "provider": "internal",
      "displayName": "Stub Model (Testing)",
      "costPerMillionTokens": 0.0,
      "natureCostPerMillionTokens": 0.0,
      "description": "Deterministic testing model"
    },
    {
      "id": "llama3.2:1b",
      "provider": "ollama",
      "displayName": "Llama 3.2 1B (Local)",
      "costPerMillionTokens": 0.0,
      "natureCostPerMillionTokens": 2.5,
      "energyPerMillionTokens": 0.05,
      "description": "Local Llama 3.2 1B model"
    },
    {
      "id": "llama3.2:3b",
      "provider": "ollama",
      "displayName": "Llama 3.2 3B (Local)",
      "costPerMillionTokens": 0.0,
      "natureCostPerMillionTokens": 7.5,
      "energyPerMillionTokens": 0.15,
      "description": "Local Llama 3.2 3B model"
    },
    {
      "id": "gpt-4",
      "provider": "openai",
      "displayName": "GPT-4",
      "costPerMillionTokens": 30000.0,
      "natureCostPerMillionTokens": 15.0,
      "description": "OpenAI GPT-4"
    },
    {
      "id": "gpt-3.5-turbo",
      "provider": "openai",
      "displayName": "GPT-3.5 Turbo",
      "costPerMillionTokens": 500.0,
      "natureCostPerMillionTokens": 5.0,
      "description": "OpenAI GPT-3.5 Turbo"
    }
  ],
  "defaultNatureCostAlgorithm": "simple",
  "natureCostAlgorithms": {
    "simple": {
      "formula": "tokens * model.natureCostPerMillionTokens / 1000000",
      "description": "Basic calculation: tokens × model nature cost factor"
    },
    "energy_based": {
      "formula": "(tokens * model.energyPerMillionTokens / 1000000) * grid_carbon_intensity",
      "description": "Energy consumption × carbon intensity of grid",
      "parameters": {
        "grid_carbon_intensity": 0.5
      }
    }
  }
}
```

**Task 4.2**: Load configuration in Rust

**File**: `src-tauri/src/governance.rs`

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Clone)]
pub struct ModelConfig {
    pub id: String,
    pub provider: String,
    pub display_name: String,
    pub cost_per_million_tokens: f64,
    pub nature_cost_per_million_tokens: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub energy_per_million_tokens: Option<f64>,
    pub description: String,
}

#[derive(Deserialize, Serialize)]
pub struct ModelsConfig {
    pub models: Vec<ModelConfig>,
    pub default_nature_cost_algorithm: String,
    pub nature_cost_algorithms: HashMap<String, serde_json::Value>,
}

pub fn load_models_config() -> anyhow::Result<ModelsConfig> {
    let config_path = "models.json"; // Or use app config dir
    let config_str = std::fs::read_to_string(config_path)?;
    let config: ModelsConfig = serde_json::from_str(&config_str)?;
    Ok(config)
}

pub fn get_model_cost(model_id: &str, config: &ModelsConfig) -> Option<f64> {
    config.models
        .iter()
        .find(|m| m.id == model_id)
        .map(|m| m.cost_per_million_tokens)
}

pub fn estimate_usd_cost_with_config(tokens: u64, model_id: &str, config: &ModelsConfig) -> f64 {
    let cost_per_million = get_model_cost(model_id, config).unwrap_or(10.0); // Default fallback
    (tokens as f64 / 1_000_000.0) * cost_per_million
}
```

**Task 4.3**: Update cost estimation calls

Find all calls to `estimate_usd_cost()` and replace with config-aware version.

**Task 4.4**: Test with different models
- Execute workflow with llama3.2 (should show $0.00)
- Execute workflow with gpt-4 simulation (should show correct cost)
- Verify budget enforcement uses correct pricing

**Success Criteria**:
- ✅ Config file loads successfully
- ✅ Per-model costs are correct
- ✅ Budget projection is accurate
- ✅ Nature cost varies by model

**Estimated Time**: 1 day

---

## Day 5: Integration Testing & Bug Fixes

### Action Items

**Task 5.1**: End-to-End Workflow Test
```
1. Create new project
2. Set budgets (tokens, USD, nature cost)
3. Add 3-step workflow:
   - Ingest document
   - Summarize
   - Prompt with context
4. Execute workflow
5. Check Inspector shows all outputs
6. Export CAR
7. Verify CAR contains outputs
```

**Task 5.2**: Budget Enforcement Test
```
1. Set token budget to 100
2. Try to execute step requiring 500 tokens
3. Verify execution is blocked
4. Check incident checkpoint is created
```

**Task 5.3**: Model Cost Test
```
1. Execute with llama3.2 → verify $0 cost
2. Simulate gpt-4 → verify correct cost calculation
3. Check budget remaining is accurate
```

**Task 5.4**: CAR Export/Import Test
```
1. Export CAR from execution
2. Verify all fields present:
   - Signatures
   - Checkpoints
   - Policy
   - Budgets
   - Step outputs
3. (Future) Import and replay
```

**Task 5.5**: Bug Fixes
- Fix any issues found during testing
- Address edge cases
- Improve error messages

**Success Criteria**:
- ✅ All workflows execute successfully
- ✅ Budgets are enforced correctly
- ✅ Outputs are saved and displayed
- ✅ CAR export works
- ✅ Costs are calculated accurately

**Estimated Time**: 1 day

---

## Week 1 Deliverables

By end of Week 1, you should have:

1. ✅ **Output Display**: Can see what each step produced
2. ✅ **CAR Export**: Can export verifiable proofs
3. ✅ **Model Costs**: Accurate per-model pricing
4. ✅ **Budget Tracking**: Correct cost calculations
5. ✅ **End-to-End Tested**: All workflows work reliably

**What This Enables**:
- Complete verification workflow
- Meaningful cost control
- Shareable proofs (CAR files)
- Accurate budget projections

**Unblocks**:
- Week 2: Replay grading
- Week 3: Budget UI
- Week 4: IXP export/import
- Demo preparation

---

## Daily Checklist

### Day 1
- [ ] Verify output storage works
- [ ] Add output display to Inspector UI
- [ ] Test output viewing with real workflow

### Day 2
- [ ] Improve output display (copy, scroll, formatting)
- [ ] Test with various output types (JSON, text, long outputs)
- [ ] Polish UI

### Day 3
- [ ] Add CAR export button
- [ ] Verify CAR export function works
- [ ] Test export with complete workflow
- [ ] Validate exported JSON structure

### Day 4
- [ ] Create models.json configuration
- [ ] Implement config loading in Rust
- [ ] Update cost calculation functions
- [ ] Test with multiple models

### Day 5
- [ ] End-to-end workflow tests
- [ ] Budget enforcement tests
- [ ] Model cost tests
- [ ] CAR export/validation tests
- [ ] Fix discovered bugs

---

## Success Metrics

### Functional
- All steps save outputs: **Pass/Fail**
- Inspector displays outputs: **Pass/Fail**
- CAR export works: **Pass/Fail**
- Model costs are accurate: **Within 10%**

### UX
- Output display is readable: **Yes/No**
- Export is one-click: **Yes/No**
- No confusing errors: **Yes/No**

### Technical
- No crashes during testing: **Pass/Fail**
- Performance is acceptable: **< 5s per step**
- Database operations succeed: **100% success rate**

---

## Risks & Mitigation

### Risk 1: Output payload not populating
**Mitigation**: Already checked - code looks correct. If issue found, check each executor function.

### Risk 2: CAR export API missing
**Mitigation**: CAR building logic exists in `car.rs`. If missing API endpoint, add as shown above.

### Risk 3: Model config breaks existing workflows
**Mitigation**: Use fallback pricing for unknown models. Keep backwards compatibility.

### Risk 4: Time overrun
**Mitigation**: Focus on core functionality first. Polish can come in Week 2.

---

## After Week 1

**Next Steps**: Proceed to Week 2 (Governance & Replay)
- Implement replay grading system
- Add Nature Cost algorithms
- Budget tracking UI

**Current Capability**: Can create, execute, and export verifiable workflows with accurate cost tracking.

**Market Readiness**: ~80% of core MVP functionality complete.

---

**Document Version**: 1.0
**Last Updated**: 2025-10-07
**Owner**: Development Team
**Review Date**: End of Week 1 (2025-10-14)
