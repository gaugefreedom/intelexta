# Proof Mode Strategy for Graded Replay

## Overview

IntelExta supports **graded replay** - the ability to re-execute a workflow and verify that outputs are consistent with the original execution. The `proof_mode` setting determines how strictly outputs must match during replay verification.

## Proof Modes

### `exact` (Deterministic Match)
- **Definition**: Outputs must be **byte-for-byte identical** (SHA-256 hash match)
- **Use Case**: Deterministic operations where any variation indicates a problem
- **Grade**: Binary pass/fail (either matches or doesn't)

### `concordant` (Semantic Match)
- **Definition**: Outputs must be **semantically similar** within tolerance (epsilon)
- **Use Case**: LLM-generated content where wording varies but meaning is preserved
- **Grade**: A+ through F based on semantic distance (SimHash comparison)
  - **A+**: 95-100% similar (0.00-0.05 distance)
  - **A**: 90-95% similar (0.05-0.10 distance)
  - **B**: 80-90% similar (0.10-0.20 distance)
  - **C**: 70-80% similar (0.20-0.30 distance)
  - **D**: 60-70% similar (0.30-0.40 distance)
  - **F**: < 60% similar (> 0.40 distance)

---

## Step Type Recommendations

### 1. **Ingest Document** (`step_type: "ingest"`)

**Recommended Mode**: `exact` (always)

**Rationale**:
- Document ingestion is **deterministic** - same file produces same canonical representation
- Extracts structured text from PDF, LaTeX, TXT, DOCX formats
- No LLM involved, no randomness
- Any variation in output indicates:
  - File has changed
  - Parser has changed
  - Corruption occurred

**Implementation**:
- UI should **not show** proof_mode selector for ingest steps
- Backend should **enforce** `proof_mode: "exact"` for `step_type: "ingest"`
- Replay verifies SHA-256 hash of canonical document JSON

**Example**:
```json
{
  "step_type": "ingest",
  "checkpoint_type": "Step",
  "proof_mode": "exact",  // Always exact
  "config_json": "{\"source_path\":\"/path/to/doc.pdf\",\"format\":\"pdf\"}"
}
```

---

### 2. **Summarize** (`step_type: "summarize"`)

**Recommended Mode**: `concordant` (default)

**Rationale**:
- LLM-generated summaries are **non-deterministic** even with same model/prompt
- Different runs produce different wording but same meaning
- Temperature > 0 causes natural variation
- Goal is to verify semantic consistency, not exact wording

**Typical Epsilon**: `0.15` (allows B-grade or better)

**Implementation**:
- UI should show proof_mode selector: `["exact", "concordant"]`
- Default to `concordant` with `epsilon: 0.15`
- User can tighten (lower epsilon) or loosen (higher epsilon) tolerance
- User can choose `exact` if they need reproducible summaries (temp=0, seed fixed)

**Example**:
```json
{
  "step_type": "summarize",
  "checkpoint_type": "Step",
  "proof_mode": "concordant",
  "epsilon": 0.15,
  "model": "llama3.2",
  "config_json": "{\"source_step\":\"step-1\",\"summary_type\":\"executive\"}"
}
```

---

### 3. **Prompt** (with context) (`step_type: "prompt"`)

**Recommended Mode**: User choice (`exact` or `concordant`)

**Rationale**:
- Use case varies widely:
  - **Data extraction**: Needs `exact` (extracting structured fields)
  - **Creative writing**: Needs `concordant` (semantic similarity acceptable)
  - **Classification**: Needs `exact` (category labels must match)
  - **Explanation**: Needs `concordant` (wording can vary)

**Typical Epsilon (if concordant)**: `0.15`

**Implementation**:
- UI should show proof_mode selector: `["exact", "concordant"]`
- No default - **require user to choose**
- If `concordant`, show epsilon slider (0.05 - 0.40 range)
- Help text explains when to use each mode

**Example (data extraction - exact)**:
```json
{
  "step_type": "prompt",
  "checkpoint_type": "Step",
  "proof_mode": "exact",
  "model": "llama3.2",
  "prompt": "Extract author name from context",
  "config_json": "{\"context_step\":\"step-1\"}"
}
```

**Example (explanation - concordant)**:
```json
{
  "step_type": "prompt",
  "checkpoint_type": "Step",
  "proof_mode": "concordant",
  "epsilon": 0.20,
  "model": "llama3.2",
  "prompt": "Explain the main argument in simple terms",
  "config_json": "{\"context_step\":\"step-1\"}"
}
```

---

### 4. **Legacy Step Prompt** (`checkpoint_type: "Step"`, `step_type: "step"`)

**Current Implementation**: User choice (already working ✅)

**Rationale**:
- Original design - predates step chaining
- User explicitly chooses proof_mode in UI
- Works the same as new Prompt type

---

## Implementation Checklist

### Backend (Already Complete ✅)
- [x] `run_steps` table has `proof_mode` column
- [x] `RunStep` struct has `proof_mode` field
- [x] Replay logic handles both `exact` and `concordant` modes
- [x] Grading system (A+ through F) implemented
- [x] SimHash semantic comparison working

### Frontend (Needs Updates)
- [ ] **Ingest step editor**: Hide proof_mode selector (force exact)
- [ ] **Summarize step editor**: Add proof_mode selector (default: concordant)
- [ ] **Prompt step editor**: Add proof_mode selector (require choice)
- [ ] **Epsilon slider**: Show when `concordant` selected (range: 0.05-0.40)
- [ ] **Help tooltips**: Explain when to use each mode

### Validation Rules
- [ ] Backend: Force `proof_mode = "exact"` for `step_type = "ingest"`
- [ ] Frontend: Require proof_mode selection for new prompt/summarize steps
- [ ] Frontend: Show epsilon field only when `concordant` selected

---

## CAR (Content-Addressable Receipt) Integration

When a CAR is generated, it includes:

```json
{
  "proof": {
    "match_kind": "exact" | "semantic" | "process",
    "epsilon": 0.15,  // If semantic
    "distance_metric": "simhash_hamming_256",
    "original_semantic_digest": "abc123...",
    "replay_semantic_digest": "def456...",
    "semantic_distance": 12  // Hamming distance
  }
}
```

The `proof.match_kind` is determined by:
- **"exact"**: All steps used `proof_mode: "exact"`
- **"semantic"**: At least one step used `proof_mode: "concordant"`
- **"process"**: Interactive chat mode (different verification)

---

## Grading Philosophy

### Why Grade Instead of Pass/Fail?

Traditional testing is binary: pass or fail. But LLM outputs exist on a **spectrum of correctness**:

- **A+ (95-100%)**: Nearly identical, minor variation (e.g., "The document discusses..." vs "This document discusses...")
- **A (90-95%)**: Very similar, same meaning with different structure
- **B (80-90%)**: Similar core content, some stylistic differences
- **C (70-80%)**: Recognizable similarity, diverging details
- **D (60-70%)**: Loosely related, significant drift
- **F (< 60%)**: Different content, model hallucinated or prompt drifted

This allows **governance policies** to set thresholds:
- High-stakes: Require A+ grade
- Medium-stakes: Accept B or better
- Low-stakes: Accept C or better

---

## Future Enhancements

### Model-Specific Defaults
Different models have different variation patterns:
- **Llama 3.2**: Low variation (epsilon: 0.10 recommended)
- **Mixtral**: Medium variation (epsilon: 0.15 recommended)
- **GPT-4**: Low variation (epsilon: 0.08 recommended)

Could auto-suggest epsilon based on model selection.

### Epsilon Calibration Tool
Run multiple replays and show distribution of semantic distances, helping users choose appropriate epsilon for their use case.

### Custom Grading Rubrics
Allow users to define their own grading scales beyond A-F (e.g., "Excellent/Good/Fair/Poor").

---

## References

- **SimHash Algorithm**: Used for semantic similarity (Charikar 2002)
- **Hamming Distance**: Measures bit differences in 256-bit SimHash digests
- **CAR Specification**: See `docs/CAR_FORMAT.md`
- **Replay Implementation**: See `src-tauri/src/replay.rs`
