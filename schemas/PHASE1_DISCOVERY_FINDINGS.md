# Phase 1: Discovery Findings - CAR v0.3 Schema Alignment

**Date**: 2025-11-17
**Goal**: Document the actual CAR structure emitted by working implementations as the foundation for v0.3 schema

---

## Executive Summary

Both IntelexTA Desktop (CAR-Full) and the verifiable-summary MCP server (CAR-Lite) are **already fully aligned** and generating compatible CARs that validate correctly with `intelexta-verify`. The field naming and structure are consistent across implementations.

**Key Finding**: v0.2 schema is accurate for field names and types. The main issue is that v0.2 doesn't include `proof.process` which is **required** by the current implementation.

---

## 1. Current Schema vs. Reality

### v0.2 Schema Status
- **Location**: `schemas/car-v0.2.schema.json`
- **Field naming**: Uses `snake_case` for top-level and proof fields, `camelCase` for run.steps fields
- **Missing**: `proof.process` and `proof.process.sequential_checkpoints` structure
- **`match_kind` enum**: Already includes `["exact", "semantic", "process"]` ✅

### Critical Gap
The v0.2 schema defines `proof` with only:
```json
{
  "match_kind": "...",
  "epsilon": ...,
  "distance_metric": "...",
  "original_semantic_digest": "...",
  "replay_semantic_digest": "..."
}
```

But **both implementations emit**:
```json
{
  "match_kind": "process" | "semantic",
  "process": {
    "sequential_checkpoints": [...]
  }
}
```

---

## 2. Field Naming Conventions (Actual)

### Top-Level CAR Fields
**snake_case** (confirmed in both Desktop and MCP):
- `run_id`
- `created_at`
- `policy_ref`
- `signer_public_key`

### run.steps Fields
**camelCase** (confirmed in both):
- `runId`
- `orderIndex`
- `checkpointType`
- `stepType`
- `tokenBudget`
- `proofMode`
- `configJson`

### proof.process.sequential_checkpoints Fields
**snake_case** (confirmed):
- `prev_chain`
- `curr_chain`
- `run_id`
- `inputs_sha256`
- `outputs_sha256`
- `usage_tokens`
- `prompt_tokens`
- `completion_tokens`

**Conclusion**: The naming convention is **hybrid** and already consistent across implementations. No changes needed.

---

## 3. Proof Structure Analysis

### 3.1 Desktop CAR (CAR-Full)
**Sample**: `intelextadesctopexamplenov16.car.zip`

```json
{
  "proof": {
    "match_kind": "semantic",
    "process": {
      "sequential_checkpoints": [
        {
          "id": "a0534cb5-7e36-44a7-89ec-7e66a15d7518",
          "prev_chain": "",
          "curr_chain": "96859784cd434331d105b93464b01f0feaf230f7f0ed8f71701368827420cf9c",
          "signature": "aBQ9HzLZxSg0js2MTmFw9BS6z9xEJ5h4Fq6puTR/xAK39bM0rtfXX4eedlOETyxQ1h0M0mUFbvpaz2sdcyDcBw==",
          "run_id": "7b28935e-eb36-4fe4-8077-f0b1ca0b1b1d",
          "kind": "Step",
          "timestamp": "2025-11-17T07:58:02.457633858+00:00",
          "inputs_sha256": "8e51212e08f6bbbeab13830d1d626376309b29b01b483b69198baaf9fa5860ee",
          "outputs_sha256": "7d98feb8f3c10f89d0fdd7f68bcf1596f521ed81693fc2835c9917326e98c279",
          "usage_tokens": 206,
          "prompt_tokens": 28,
          "completion_tokens": 178
        },
        {
          "id": "02400196-4bb4-4dcc-a3ca-bc285700e0b0",
          "prev_chain": "96859784cd434331d105b93464b01f0feaf230f7f0ed8f71701368827420cf9c",
          "curr_chain": "db323ebe20bccb3e9ae3eaab5ecaaa579c3c79d5a09e458c183900ebc0329bda",
          "signature": "kqK196FRbdmUmzKpc5lIeZw/k5XWn8Gbpm54NCqK4K2JgF4uDcJvu/mG+EfkcyFVi631SX9G/URE644UufwwCw==",
          "run_id": "7b28935e-eb36-4fe4-8077-f0b1ca0b1b1d",
          "kind": "Step",
          "timestamp": "2025-11-17T07:58:38.246149900+00:00",
          "inputs_sha256": "c5312b59e7f3669049d1febc38c00518b744a3d95ed222bd0b7cd708ae4d1839",
          "outputs_sha256": "5efa3802fc27f4edba8ef4ad5f2d565eda7fe0b5a34cce3b1e168dce40ed66b3",
          "usage_tokens": 305,
          "prompt_tokens": 210,
          "completion_tokens": 95
        }
      ]
    }
  }
}
```

**Characteristics**:
- `match_kind`: `"semantic"` (for concordant/semantic replay workflows)
- Multiple checkpoints (2 in this example)
- Non-empty `prev_chain` for checkpoints after the first
- Signatures present (Ed25519)
- No `parent_checkpoint_id` or `turn_index` (not an interactive workflow)

---

### 3.2 MCP CAR-Lite
**Sample**: `verifiable.car.zip`

```json
{
  "proof": {
    "match_kind": "process",
    "process": {
      "sequential_checkpoints": [
        {
          "id": "vs-1761766730663-b2o3z01rb",
          "prev_chain": "",
          "curr_chain": "cc5928eff28b1ffe041b3fe1544c07111e949655962c35f73f4ab70b56e25f48",
          "signature": "SUEPExmxgrDtE+6jDhuF3yWm3qSmD8GohJTuxBxQdh9hoX2/2as29JCOIWOlQD3j6t5CX/6BPUXs0qphk1eXBQ==",
          "run_id": "vs-1761766730663-b2o3z01rb",
          "kind": "Step",
          "timestamp": "2025-10-29T19:38:50.663Z",
          "inputs_sha256": "ad555bc2ac448e09b5f20abb2d3a3d2e068e637b76be591f95705e9dc357dd48",
          "outputs_sha256": "11e451cbb4c789950d5577bf3dd6da3b4223411059b17a7f1f5c4c33afa5e1a1",
          "usage_tokens": 0,
          "prompt_tokens": 0,
          "completion_tokens": 0
        }
      ]
    }
  }
}
```

**Characteristics**:
- `match_kind`: `"process"` (CAR-Lite profile)
- Single checkpoint
- Empty `prev_chain` (first checkpoint)
- Signature present
- Neutral token counts (0,0,0)

---

## 4. Verification Implementation Analysis

### 4.1 `intelexta-verify` CLI (`src-tauri/crates/intelexta-verify/src/main.rs`)

**Critical code path** (lines 145-155):
```rust
let checkpoints = match &car.proof.process {
    Some(process) => &process.sequential_checkpoints,
    None => {
        report.error = Some(format!(
            "CAR has no process proof (match_kind: {}). This CAR was likely exported with an older version...",
            car.proof.match_kind
        ));
        return Ok(report);
    }
};
```

**Finding**: `proof.process` is **REQUIRED** by the CLI verifier. Without it, verification fails.

**Checkpoint verification** (lines 232-253):
- Verifies hash chain: `SHA256(prev_chain || canonical_json(checkpoint_body)) == curr_chain`
- Verifies Ed25519 signatures on `curr_chain`

**Expected checkpoint body fields** (lines 218-229):
```rust
struct CheckpointBody<'a> {
    run_id: &'a str,
    kind: &'a str,
    timestamp: &'a str,
    inputs_sha256: &'a Option<String>,
    outputs_sha256: &'a Option<String>,
    incident: Option<serde_json::Value>,  // Always None for process
    usage_tokens: u64,
    prompt_tokens: u64,
    completion_tokens: u64,
}
```

---

### 4.2 Desktop Exporter (`src-tauri/src/car.rs`)

**Process proof generation** (lines 366-392):
```rust
let process_proof = if !checkpoints.is_empty() {
    let sequential = checkpoints.iter().map(|ck| ProcessCheckpointProof {
        id: ck.id.clone(),
        parent_checkpoint_id: ck.parent_checkpoint_id.clone(),
        turn_index: ck.turn_index,
        prev_chain: ck.prev_chain.clone(),
        curr_chain: ck.curr_chain.clone(),
        signature: ck.signature.clone(),
        run_id: run_id.to_string(),
        kind: ck.kind.clone(),
        timestamp: ck.timestamp.to_rfc3339(),
        inputs_sha256: ck.inputs_sha256.clone(),
        outputs_sha256: ck.outputs_sha256.clone(),
        usage_tokens: ck.usage_tokens,
        prompt_tokens: ck.prompt_tokens,
        completion_tokens: ck.completion_tokens,
    }).collect();
    Some(ProcessProof { sequential_checkpoints: sequential })
} else {
    None
}
```

**`match_kind` logic** (lines 405-411):
```rust
let proof_match_kind = if is_interactive {
    "process".to_string()
} else if has_concordant_checkpoint {
    "semantic".to_string()
} else {
    "exact".to_string()
};
```

**Findings**:
- Desktop **always** includes `proof.process` if checkpoints exist
- `match_kind` selection:
  - Interactive workflows → `"process"`
  - Concordant steps → `"semantic"`
  - Otherwise → `"exact"`

---

### 4.3 MCP Server (`apps/verifiable-summary/server/src/provenance.ts`)

**Process proof generation** (lines 224-241):
```typescript
proof: {
  match_kind: 'process' as const,
  process: {
    sequential_checkpoints: [
      {
        id: runId,
        prev_chain: prevChain,
        curr_chain: currChain,
        signature: checkpointSignature,
        run_id: runId,
        kind: 'Step',
        timestamp: createdAt,
        inputs_sha256: inputsSha256,
        outputs_sha256: outputsSha256,
        usage_tokens: 0,
        prompt_tokens: 0,
        completion_tokens: 0
      }
    ]
  }
}
```

**Findings**:
- MCP **always** uses `match_kind: "process"`
- Always includes `proof.process.sequential_checkpoints`
- Single checkpoint for CAR-Lite

---

## 5. Struct Definitions

### 5.1 Rust (Desktop) - `src-tauri/src/car.rs`

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Proof {
    pub match_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epsilon: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_metric: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_semantic_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replay_semantic_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process: Option<ProcessProof>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProcessProof {
    pub sequential_checkpoints: Vec<ProcessCheckpointProof>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProcessCheckpointProof {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_checkpoint_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_index: Option<u32>,
    pub prev_chain: String,
    pub curr_chain: String,
    pub signature: String,
    pub run_id: String,
    pub kind: String,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs_sha256: Option<String>,
    pub usage_tokens: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}
```

### 5.2 TypeScript (MCP) - Inferred from `provenance.ts`

```typescript
interface Proof {
  match_kind: 'process' | 'exact' | 'semantic';
  epsilon?: number;
  distance_metric?: string;
  original_semantic_digest?: string;
  replay_semantic_digest?: string;
  process?: {
    sequential_checkpoints: ProcessCheckpointProof[];
  };
}

interface ProcessCheckpointProof {
  id: string;
  parent_checkpoint_id?: string;
  turn_index?: number;
  prev_chain: string;
  curr_chain: string;
  signature: string;
  run_id: string;
  kind: string;
  timestamp: string;
  inputs_sha256?: string;
  outputs_sha256?: string;
  usage_tokens: number;
  prompt_tokens: number;
  completion_tokens: number;
}
```

---

## 6. Additional Fields in Desktop vs. MCP

### Desktop-Specific Fields
In `policy_ref`:
```json
{
  "model_catalog_hash": "sha256:081c2228e00bfc84a1b32fae590cde6420190eebe1d4a6ce295f339cc3b89238",
  "model_catalog_version": "1.0.0"
}
```

These fields have defaults in Rust (lines 104-116 of `car.rs`):
```rust
#[serde(default = "default_catalog_hash")]
pub model_catalog_hash: String,
#[serde(default = "default_catalog_version")]
pub model_catalog_version: String,
```

**Conclusion**: These are optional and backward-compatible.

---

## 7. Signatures

Both implementations use **dual signatures**:

### Format
```json
"signatures": [
  "ed25519-body:<base64-sig>",
  "ed25519-checkpoint:<base64-sig>"
]
```

### Semantics
1. **`ed25519-body`**: Signs the entire canonical CAR body (with `id` included, `signatures` removed)
2. **`ed25519-checkpoint`**: Signs the final checkpoint's `curr_chain` hash

**Sources**:
- Desktop: `car.rs` lines 468-482
- MCP: `provenance.ts` lines 286-302

---

## 8. Required vs. Optional Fields

### Currently Required in Both Implementations
Top-level:
- `id`, `run_id`, `created_at`, `run`, `proof`, `policy_ref`, `budgets`, `provenance`, `checkpoints`, `sgrade`, `signer_public_key`, `signatures`

`proof.process.sequential_checkpoints[*]`:
- `id`, `prev_chain`, `curr_chain`, `signature`, `run_id`, `kind`, `timestamp`, `usage_tokens`, `prompt_tokens`, `completion_tokens`

### Optional (with `#[serde(skip_serializing_if = "Option::is_none")]`)
- `proof.epsilon`, `proof.distance_metric`, `proof.original_semantic_digest`, `proof.replay_semantic_digest`
- `proof.process` (though **functionally required** by verifier)
- `ProcessCheckpointProof.parent_checkpoint_id`, `turn_index`, `inputs_sha256`, `outputs_sha256`

---

## 9. Recommendations for v0.3 Schema

### 9.1 Add `proof.process` to Schema
```json
{
  "proof": {
    "type": "object",
    "properties": {
      "match_kind": { "enum": ["exact", "semantic", "process"] },
      "process": {
        "type": "object",
        "properties": {
          "sequential_checkpoints": {
            "type": "array",
            "items": { "$ref": "#/$defs/process_checkpoint_proof" }
          }
        },
        "required": ["sequential_checkpoints"]
      },
      ...
    }
  }
}
```

### 9.2 Define `process_checkpoint_proof`
```json
{
  "process_checkpoint_proof": {
    "type": "object",
    "required": [
      "id", "prev_chain", "curr_chain", "signature",
      "run_id", "kind", "timestamp",
      "usage_tokens", "prompt_tokens", "completion_tokens"
    ],
    "properties": {
      "id": { "type": "string" },
      "parent_checkpoint_id": { "type": "string" },
      "turn_index": { "type": "integer" },
      "prev_chain": { "type": "string" },
      "curr_chain": { "type": "string" },
      "signature": { "type": "string" },
      "run_id": { "type": "string" },
      "kind": { "type": "string" },
      "timestamp": { "type": "string", "format": "date-time" },
      "inputs_sha256": { "type": "string" },
      "outputs_sha256": { "type": "string" },
      "usage_tokens": { "type": "integer", "minimum": 0 },
      "prompt_tokens": { "type": "integer", "minimum": 0 },
      "completion_tokens": { "type": "integer", "minimum": 0 }
    }
  }
}
```

### 9.3 Add Conditional Validation for `match_kind`
```json
{
  "allOf": [
    {
      "if": { "properties": { "match_kind": { "const": "semantic" } } },
      "then": {
        "required": ["epsilon", "distance_metric", "original_semantic_digest"]
      }
    },
    {
      "if": {
        "properties": {
          "match_kind": { "enum": ["process", "semantic", "exact"] }
        }
      },
      "then": {
        "required": ["process"]
      }
    }
  ]
}
```

### 9.4 Add Optional `policy_ref` Fields
```json
{
  "policy_ref": {
    "properties": {
      "hash": { ... },
      "egress": { ... },
      "estimator": { ... },
      "model_catalog_hash": { "type": "string" },
      "model_catalog_version": { "type": "string" }
    },
    "required": ["hash", "egress", "estimator"]
  }
}
```

### 9.5 Keep Hybrid Naming Convention
**No changes needed** - the hybrid convention is consistent and intentional:
- Top-level and proof: `snake_case`
- `run.steps`: `camelCase`

---

## 10. Summary

### What's Working
✅ Field naming is consistent across Desktop and MCP
✅ Both generate `proof.process.sequential_checkpoints`
✅ Both use dual signatures (`ed25519-body` + `ed25519-checkpoint`)
✅ `intelexta-verify` correctly validates both CAR types
✅ `match_kind` enum already includes `"process"`

### What Needs Fixing in v0.3
❌ v0.2 schema is **missing** `proof.process` definition
❌ v0.2 schema doesn't define `process_checkpoint_proof` structure
❌ v0.2 schema doesn't have conditional validation for `match_kind`

### Next Steps
1. Create `car-v0.3.schema.json` with `proof.process` added
2. Define `process_checkpoint_proof` in `$defs`
3. Add conditional validation rules
4. Keep field naming as-is (no changes needed)
5. Test v0.3 against existing CARs from Desktop and MCP
