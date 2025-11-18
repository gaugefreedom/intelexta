# Phase 2: Schema Implementation Complete

**Date**: 2025-11-17
**Status**: ✅ COMPLETE

---

## Summary

Successfully created `car-v0.3.schema.json` that accurately reflects the **current implementation** of both IntelexTA Desktop and the verifiable-summary MCP server (CAR-Lite).

### Validation Results

✅ **Desktop CAR** (`intelextadesctopexamplenov16.car.zip`): **VALID**
✅ **MCP CAR-Lite** (`verifiable.car.zip`): **VALID**

Both sample CARs validate successfully against v0.3 **without any code changes**, confirming the schema now matches reality.

---

## Changes Made to v0.3

### 1. Metadata Updates
- Updated `$id`: `https://intelexta.org/schemas/car-v0.3.schema.json`
- Updated `title`: "Intelexta Content-Addressable Receipt (CAR) v0.3"
- Updated `description` to reflect process proof chains

### 2. Added `proof.process` Structure

**Before (v0.2)**:
```json
{
  "proof": {
    "match_kind": "...",
    "epsilon": ...,
    "distance_metric": "...",
    "original_semantic_digest": "...",
    "replay_semantic_digest": "..."
  }
}
```

**After (v0.3)**:
```json
{
  "proof": {
    "match_kind": "process" | "exact" | "semantic",
    "process": {
      "sequential_checkpoints": [...]
    },
    ...
  }
}
```

### 3. New Schema Definitions

Added two new `$defs`:

#### `process_proof`
```json
{
  "type": "object",
  "required": ["sequential_checkpoints"],
  "properties": {
    "sequential_checkpoints": {
      "type": "array",
      "minItems": 1,
      "items": { "$ref": "#/$defs/process_checkpoint_proof" }
    }
  }
}
```

#### `process_checkpoint_proof`
```json
{
  "type": "object",
  "required": [
    "id", "prev_chain", "curr_chain", "signature",
    "run_id", "kind", "timestamp",
    "usage_tokens", "prompt_tokens", "completion_tokens"
  ],
  "properties": {
    "id": { "type": "string" },
    "parent_checkpoint_id": { "type": "string" },  // Optional
    "turn_index": { "type": "integer" },           // Optional
    "prev_chain": { "type": "string" },
    "curr_chain": { "type": "string", "pattern": "^[0-9a-f]{64}$" },
    "signature": { "type": "string" },
    "run_id": { "type": "string" },
    "kind": { "type": "string" },
    "timestamp": { "type": "string", "format": "date-time" },
    "inputs_sha256": { "type": "string", "pattern": "^[0-9a-f]{64}$" },
    "outputs_sha256": { "type": "string", "pattern": "^[0-9a-f]{64}$" },
    "usage_tokens": { "type": "integer", "minimum": 0 },
    "prompt_tokens": { "type": "integer", "minimum": 0 },
    "completion_tokens": { "type": "integer", "minimum": 0 }
  }
}
```

### 4. Updated `run_step` Field Names (camelCase)

**Before (v0.2)**:
- `run_id` → **After (v0.3)**: `runId`
- `order_index` → `orderIndex`
- `checkpoint_type` → `checkpointType`
- `step_type` → `stepType`
- `token_budget` → `tokenBudget`
- `proof_mode` → `proofMode`
- `config_json` → `configJson`

**Required fields updated**:
```json
"required": [
  "id", "runId", "orderIndex", "checkpointType",
  "tokenBudget", "proofMode"
]
```

### 5. Relaxed `checkpoints` Pattern

**Before (v0.2)**:
```json
"pattern": "^ckpt:[A-Za-z0-9:_-]+$"
```

**After (v0.3)**:
```json
"pattern": "^(ckpt:)?[A-Za-z0-9:_-]+$"
```

**Rationale**: Desktop uses plain UUIDs, MCP uses `ckpt:` prefix.

### 6. Added Optional `policy_ref` Fields

```json
{
  "policy_ref": {
    "properties": {
      ...
      "model_catalog_hash": { "type": "string" },
      "model_catalog_version": { "type": "string" }
    }
  }
}
```

**Rationale**: Desktop-specific fields for model catalog verification. Not required, allowing MCP CARs to omit them.

### 7. Conditional Validation for `match_kind`

Added `allOf` rule in `proof`:

```json
{
  "allOf": [
    {
      "if": {
        "properties": { "match_kind": { "const": "process" } }
      },
      "then": {
        "required": ["process"]
      }
    }
  ]
}
```

**Semantics**:
- `match_kind: "process"` → `proof.process` is **REQUIRED**
- `match_kind: "semantic"` → `proof.process` is optional (Desktop includes it anyway)
- `match_kind: "exact"` → `proof.process` is optional

---

## What Was NOT Changed

### 1. Hybrid Naming Convention (Kept As-Is)
- Top-level fields: `snake_case` (e.g., `run_id`, `created_at`)
- `run.steps` fields: `camelCase` (e.g., `runId`, `orderIndex`)
- `proof.process.sequential_checkpoints` fields: `snake_case` (e.g., `prev_chain`, `curr_chain`)

This hybrid convention is **intentional and consistent** across both implementations.

### 2. Optional Semantic Fields
Left as optional in v0.3:
- `proof.epsilon`
- `proof.distance_metric`
- `proof.original_semantic_digest`
- `proof.replay_semantic_digest`

**Rationale**: Desktop CARs with `match_kind: "semantic"` currently don't emit these fields. We'll tighten this in a future version once the Desktop exporter is upgraded.

### 3. No Code Changes
Per Phase 2 constraints, **zero code changes** were made to:
- Desktop exporter (`src-tauri/src/car.rs`)
- MCP server (`apps/verifiable-summary/server/src/provenance.ts`)
- CLI verifier (`src-tauri/crates/intelexta-verify/src/main.rs`)

---

## Validation Process

### Tools Used
- **AJV 8.x** (JSON Schema 2020-12 validator)
- Node.js validation script: `/tmp/validate-car.js`

### Test Cases

#### Test 1: Desktop CAR
**File**: `intelextadesctopexamplenov16.car.zip`
**Characteristics**:
- `match_kind: "semantic"`
- 2 checkpoints
- `proof.process.sequential_checkpoints` present
- `policy_ref.model_catalog_hash` and `model_catalog_version` present

**Result**: ✅ VALID

#### Test 2: MCP CAR-Lite
**File**: `verifiable.car.zip`
**Characteristics**:
- `match_kind: "process"`
- 1 checkpoint
- `proof.process.sequential_checkpoints` present
- No `model_catalog_*` fields

**Result**: ✅ VALID

---

## Key Insights

### 1. Schema Was the Problem, Not the Code
The implementations were already aligned. v0.2 schema was simply **missing** the `proof.process` structure that both implementations have been emitting all along.

### 2. Both Implementations Use `proof.process`
- **Desktop**: Emits `proof.process` for ALL runs (even non-interactive)
- **MCP**: Always emits `proof.process` for CAR-Lite
- **CLI Verifier**: **REQUIRES** `proof.process` to verify CARs

### 3. `match_kind` Semantics Are Clear
From actual code:
- **`"process"`**: Used by MCP CAR-Lite and Desktop interactive workflows
- **`"semantic"`**: Used by Desktop for concordant/semantic replay workflows
- **`"exact"`**: Used by Desktop for exact replay workflows

All three modes can (and do) coexist with `proof.process`.

---

## Files Created/Modified

### Created
- ✅ `schemas/car-v0.3.schema.json` - New schema matching current implementation
- ✅ `schemas/PHASE1_DISCOVERY_FINDINGS.md` - Discovery documentation
- ✅ `schemas/PHASE2_SCHEMA_IMPLEMENTATION.md` - This document

### Modified
- None (per Phase 2 constraints)

---

## Next Steps (Future Phases)

### Phase 3 (Not Started): Code Alignment
If any code needs updating to match v0.3 schema (unlikely based on validation results).

### Phase 4 (Not Started): Testing
- Add schema validation tests to CI/CD
- Ensure new CARs validate against v0.3

### Phase 5 (Not Started): Documentation Updates
- Update `CAR_FORMAT.md` to reference v0.3
- Update schema repo README
- Add migration guide from v0.2 to v0.3

---

## Conclusion

**Phase 2 is complete and successful.** The v0.3 schema now accurately describes the CAR format as implemented by both IntelexTA Desktop and the verifiable-summary MCP server.

**Key Achievement**: Achieved **100% backward compatibility** - both existing implementations produce valid v0.3 CARs without any code changes.

The schema is now the **single source of truth** for the IntelexTA CAR format, ready for community adoption and third-party tooling.
