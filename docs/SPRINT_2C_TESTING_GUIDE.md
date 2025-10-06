# Sprint 2C Testing Guide - Backend Typed Steps

## Overview

This guide provides manual testing procedures for the typed step system (Phase 1 - Backend).

## Prerequisites

1. Ensure migrations are applied (V12 should be active)
2. Have a test project created
3. Have test documents ready (PDF, TXT, DOCX, LaTeX)
4. Database can be reset between tests if needed

## Test 1: Single Ingest Step

**Objective**: Verify that document ingestion works with new typed config

### Steps:

1. **Create a run with one ingest step:**

```bash
# Using API or UI, create a run step with:
step_type: "ingest"
config_json: {
  "stepType": "ingest",
  "source_path": "/path/to/test.pdf",
  "format": "pdf",
  "privacy_status": "unrestricted"
}
```

2. **Execute the run**
3. **Verify checkpoint:**
   - Should create a "Step" checkpoint (not Incident)
   - Should have output_payload with CanonicalDocument JSON
   - Should have outputs_sha256
   - Should have semantic_digest (document_id)

### Expected Result:
✅ Checkpoint created successfully with document content

---

## Test 2: Single Prompt Step (Standalone)

**Objective**: Verify that prompt steps work without chaining

### Steps:

1. **Create a run with one prompt step:**

```bash
step_type: "prompt"
config_json: {
  "stepType": "prompt",
  "model": "stub",
  "prompt": "What is the capital of France?",
  "token_budget": 1000
}
```

2. **Execute the run**
3. **Verify checkpoint:**
   - Should create a "Step" checkpoint
   - Should have output_payload with LLM response
   - Should NOT reference any previous step

### Expected Result:
✅ Checkpoint created with prompt-only execution

---

## Test 3: Ingest → Summarize Chain

**Objective**: Verify that summarize step can read from ingest step

### Steps:

1. **Create a run with two steps:**

**Step 0 (Ingest):**
```json
{
  "stepType": "ingest",
  "source_path": "/path/to/document.pdf",
  "format": "pdf",
  "privacy_status": "unrestricted"
}
```

**Step 1 (Summarize):**
```json
{
  "stepType": "summarize",
  "source_step": 0,
  "model": "stub",
  "summary_type": "brief",
  "token_budget": 2000
}
```

2. **Execute the run**
3. **Verify both checkpoints:**
   - Step 0: Document ingestion succeeds
   - Step 1: Summarize uses document content
   - Check that Step 1's prompt_payload contains text from Step 0

### Expected Result:
✅ Two checkpoints created
✅ Step 1 prompt includes "Provide a brief 2-3 sentence summary" + document text

---

## Test 4: Ingest → Prompt Chain

**Objective**: Verify that prompt step can use previous output as context

### Steps:

1. **Create a run with two steps:**

**Step 0 (Ingest):**
```json
{
  "stepType": "ingest",
  "source_path": "/path/to/research_paper.pdf",
  "format": "pdf",
  "privacy_status": "unrestricted"
}
```

**Step 1 (Prompt):**
```json
{
  "stepType": "prompt",
  "model": "stub",
  "prompt": "What are the main findings in this paper?",
  "use_output_from": 0,
  "token_budget": 3000
}
```

2. **Execute the run**
3. **Verify Step 1 checkpoint:**
   - Check prompt_payload contains: "What are the main findings..." + "--- Context from previous step ---" + document text

### Expected Result:
✅ Step 1 includes both user prompt and document context

---

## Test 5: Three-Step Chain (Ingest → Summarize → Prompt)

**Objective**: Verify multi-step chaining works

### Steps:

1. **Create a run with three steps:**

**Step 0 (Ingest):**
```json
{
  "stepType": "ingest",
  "source_path": "/path/to/long_document.pdf",
  "format": "pdf",
  "privacy_status": "unrestricted"
}
```

**Step 1 (Summarize):**
```json
{
  "stepType": "summarize",
  "source_step": 0,
  "model": "stub",
  "summary_type": "detailed",
  "token_budget": 2000
}
```

**Step 2 (Prompt):**
```json
{
  "stepType": "prompt",
  "model": "stub",
  "prompt": "Based on this summary, what should we investigate next?",
  "use_output_from": 1,
  "token_budget": 1500
}
```

2. **Execute the run**
3. **Verify all three checkpoints:**
   - Step 0: Document ingestion
   - Step 1: Summary of document (uses step 0)
   - Step 2: Analysis of summary (uses step 1)

### Expected Result:
✅ Three checkpoints created in sequence
✅ Each step correctly references previous output

---

## Test 6: Error - Invalid source_step Reference

**Objective**: Verify error handling for invalid references

### Steps:

1. **Create a run with invalid reference:**

**Step 0 (Summarize with invalid reference):**
```json
{
  "stepType": "summarize",
  "source_step": 5,  // Step 5 doesn't exist!
  "model": "stub",
  "summary_type": "brief"
}
```

2. **Execute the run**
3. **Verify error:**
   - Should create an Incident checkpoint
   - Error message should be: "Step 0 references non-existent source step 5"

### Expected Result:
✅ Incident checkpoint created with clear error message
✅ Run stops at first step

---

## Test 7: Error - Forward Reference

**Objective**: Verify that steps can't reference future steps

### Steps:

1. **Create a run with forward reference:**

**Step 0 (Prompt):**
```json
{
  "stepType": "prompt",
  "model": "stub",
  "prompt": "Test prompt",
  "use_output_from": 1  // References step 1, which comes AFTER step 0
}
```

**Step 1 (Ingest):**
```json
{
  "stepType": "ingest",
  "source_path": "/path/to/test.pdf",
  "format": "pdf",
  "privacy_status": "unrestricted"
}
```

2. **Execute the run**
3. **Verify error:**
   - Step 0 execution should fail
   - Error: "Step 0 references non-existent source step 1"

### Expected Result:
✅ Incident checkpoint at step 0
✅ Step 1 never executes

---

## Test 8: Error - Summarize Without source_step

**Objective**: Verify that summarize requires a source

### Steps:

1. **Create a summarize step with no source:**

```json
{
  "stepType": "summarize",
  "model": "stub",
  "summary_type": "brief"
  // source_step is missing!
}
```

2. **Execute the run**
3. **Verify error:**
   - Error message: "Summarize step 0 requires a source_step"

### Expected Result:
✅ Incident checkpoint with clear error message

---

## Test 9: Backward Compatibility - Legacy Step

**Objective**: Verify that old-style steps still work

### Steps:

1. **Create a run with legacy LLM step (no config_json):**

```bash
step_type: "prompt"  # or "llm"
model: "stub"
prompt: "Hello world"
token_budget: 500
# No config_json field
```

2. **Execute the run**
3. **Verify:**
   - Should execute via legacy `execute_checkpoint()` path
   - Should create successful checkpoint

### Expected Result:
✅ Legacy steps continue to work

---

## Test 10: Budget Enforcement with Chaining

**Objective**: Verify governance still works with chained steps

### Steps:

1. **Create a chain with budget that exceeds policy:**

**Step 0 (Ingest):**
```json
{
  "stepType": "ingest",
  "source_path": "/path/to/test.pdf",
  "format": "pdf",
  "privacy_status": "unrestricted"
}
```

**Step 1 (Summarize with large budget):**
```json
{
  "stepType": "summarize",
  "source_step": 0,
  "model": "stub",
  "summary_type": "detailed",
  "token_budget": 999999999  // Exceeds policy!
}
```

2. **Execute the run**
3. **Verify:**
   - Step 0 should succeed
   - Before Step 1, should create budget_projection_exceeded Incident
   - Step 1 should NOT execute

### Expected Result:
✅ Budget enforcement still works
✅ Chain stops at budget violation

---

## Test 11: Different Summary Types

**Objective**: Verify all summary_type options work

### Test Variations:

**a) Brief Summary:**
```json
{
  "stepType": "summarize",
  "source_step": 0,
  "model": "stub",
  "summary_type": "brief"
}
```
Expected prompt: "Provide a brief 2-3 sentence summary..."

**b) Detailed Summary:**
```json
{
  "stepType": "summarize",
  "source_step": 0,
  "model": "stub",
  "summary_type": "detailed"
}
```
Expected prompt: "Provide a comprehensive summary..."

**c) Academic Summary:**
```json
{
  "stepType": "summarize",
  "source_step": 0,
  "model": "stub",
  "summary_type": "academic"
}
```
Expected prompt: "Provide an academic summary including methodology..."

**d) Custom Summary:**
```json
{
  "stepType": "summarize",
  "source_step": 0,
  "model": "stub",
  "summary_type": "custom",
  "custom_instructions": "Focus only on the data analysis sections"
}
```
Expected prompt: "Focus only on the data analysis sections..."

### Expected Result:
✅ All four summary types generate correct prompts

---

## Test 12: Multiple Document Formats

**Objective**: Verify ingestion works for all formats

### Test Variations:

**a) PDF:**
```json
{"stepType": "ingest", "source_path": "/path/to/test.pdf", "format": "pdf", "privacy_status": "unrestricted"}
```

**b) LaTeX:**
```json
{"stepType": "ingest", "source_path": "/path/to/paper.tex", "format": "latex", "privacy_status": "unrestricted"}
```

**c) TXT:**
```json
{"stepType": "ingest", "source_path": "/path/to/notes.txt", "format": "txt", "privacy_status": "unrestricted"}
```

**d) DOCX:**
```json
{"stepType": "ingest", "source_path": "/path/to/report.docx", "format": "docx", "privacy_status": "unrestricted"}
```

### Expected Result:
✅ All formats successfully ingested
✅ All produce CanonicalDocument JSON

---

## Verification Checklist

After running all tests, verify:

- [ ] Single steps work (ingest, prompt)
- [ ] Chained steps work (summarize uses source_step)
- [ ] Optional chaining works (prompt with/without use_output_from)
- [ ] Multi-step chains work (3+ steps)
- [ ] Error handling works (invalid references, missing source)
- [ ] Forward references are blocked
- [ ] Budget enforcement still works
- [ ] Network policy still works
- [ ] Legacy steps still work
- [ ] All summary types work
- [ ] All document formats work
- [ ] Signature chain is maintained (prev_chain/curr_chain)
- [ ] Token usage is tracked correctly

---

## Database Inspection Queries

Useful queries for verification:

```sql
-- Check step configurations
SELECT id, run_id, order_index, step_type, config_json
FROM run_steps
ORDER BY run_id, order_index;

-- Check checkpoints created
SELECT id, run_id, checkpoint_config_id, kind,
       inputs_sha256, outputs_sha256, usage_tokens,
       SUBSTR(output_payload, 1, 100) as output_preview
FROM checkpoints
ORDER BY created_at DESC
LIMIT 10;

-- Check for incidents
SELECT id, run_id, kind, incident
FROM checkpoints
WHERE kind = 'Incident'
ORDER BY created_at DESC;

-- Check signature chain
SELECT id, prev_chain, curr_chain
FROM checkpoints
ORDER BY created_at DESC
LIMIT 10;
```

---

## Success Criteria

**Phase 1 (Backend) is complete when:**

✅ All 12 tests pass
✅ Code compiles without errors
✅ Governance checks still work
✅ Legacy steps still work
✅ Error messages are clear and helpful

**Next Phase:**

After backend testing is complete, move to Phase 2 (Frontend UI) to create the workflow builder interface.
