# Prompt → Prompt Chaining Analysis

## Date: 2025-10-07

## Question

Does chaining Prompt → Prompt work? User tested chaining two Prompt steps together (e.g., one model asks a question, the next model answers based on the first model's output), but the second step seemed to give a general answer, not using the previous context.

## Implementation Review

### Code Analysis

The implementation **should work** based on the code:

**File**: `src-tauri/src/orchestrator.rs` (lines 1809-1831)

```rust
StepConfig::Prompt { model, prompt, use_output_from, ... } => {
    // Optionally use output from previous step
    let final_prompt = if let Some(source_idx) = use_output_from {
        let source = prior_outputs.get(&source_idx).ok_or_else(|| {
            anyhow!("Step {} references non-existent source step {}",
                    config.order_index, source_idx)
        })?;
        build_prompt_with_context(&prompt, source)  // ✅ Appends context
    } else {
        prompt.clone()
    };

    // Execute based on model type
    if model == STUB_MODEL_ID {
        execute_stub_checkpoint(stored_run.seed, config.order_index, &final_prompt)
    } else if model.starts_with(CLAUDE_MODEL_PREFIX) {
        execute_claude_mock_checkpoint(&model, &final_prompt)?
    } else {
        execute_llm_checkpoint(&model, &final_prompt, llm_client)?
    }
}
```

**Context Building** (lines 2088-2094):

```rust
fn build_prompt_with_context(prompt: &str, source: &StepOutput) -> String {
    format!(
        "{}\n\n--- Context from previous step ---\n{}",
        prompt,
        source.output_text
    )
}
```

This correctly:
1. ✅ Gets the previous step's output from `prior_outputs` HashMap
2. ✅ Appends it to the current prompt with a separator
3. ✅ Sends the combined prompt to the LLM

### Potential Issues

#### Issue 1: Stub Model Output is Not Human-Readable

**Problem**: If using `stub-model`, the output is a hex-encoded hash, not actual text.

**Stub Model Output Generation** (lines 2127-2143):
```rust
fn stub_output_bytes(seed: u64, order_index: i64, prompt: &str) -> Vec<u8> {
    let mut output = b"hello".to_vec();
    output.extend_from_slice(&seed.to_le_bytes());
    output.extend_from_slice(&order_index.to_le_bytes());
    let prompt_hash = provenance::sha256_hex(prompt.as_bytes());
    output.extend_from_slice(prompt_hash.as_bytes());
    output
}

fn execute_stub_checkpoint(run_seed: u64, order_index: i64, prompt: &str) -> NodeExecution {
    let output_bytes = stub_output_bytes(run_seed, order_index, prompt);
    let semantic_source = hex::encode(&output_bytes);  // ← Hex encoding!
    let output_payload = sanitize_payload(&semantic_source);
    // ...
}
```

**Result**: Output looks like:
```
68656c6c6f4f0000000000000001000000000000003a7bd3...
```

This is **intentional** for stub-model - it's for deterministic testing, not readable output.

#### Issue 2: Real LLM Not Seeing Context

If using a real LLM (Ollama), the context **should** be included. To verify, we need to check:
1. Is the prompt actually being built with context?
2. Is the LLM receiving the full prompt?

## Debug Logging Added

Added comprehensive logging to track context chaining (lines 1817-1830):

```rust
if let Some(source_idx) = use_output_from {
    eprintln!("🔗 Prompt step {} using output from step {}",
              config.order_index, source_idx);
    eprintln!("   Source output length: {} chars", source.output_text.len());
    eprintln!("   Source output preview: {}",
        if source.output_text.len() > 200 {
            format!("{}...", &source.output_text[..200])
        } else {
            source.output_text.clone()
        });
    let context_prompt = build_prompt_with_context(&prompt, source);
    eprintln!("   Final prompt length: {} chars", context_prompt.len());
    context_prompt
} else {
    eprintln!("🔗 Prompt step {} running standalone (no context)",
              config.order_index);
    prompt.clone()
}
```

## Testing Instructions

### Test 1: Verify Context is Being Added

1. Create a 2-step Prompt chain:
   - **Step 1**: Prompt (stub-model)
     - Prompt: "Generate a random topic"
     - Use Output From: None

   - **Step 2**: Prompt (stub-model)
     - Prompt: "Analyze the topic mentioned above"
     - Use Output From: Step 1

2. Execute the run and check the terminal output:

**Expected Output**:
```
🔗 Prompt step 0 running standalone (no context)
✅ Successfully parsed typed step: Prompt { ... }

🔗 Prompt step 1 using output from step 0
   Source output length: 142 chars
   Source output preview: 68656c6c6f4f00000000000000010000000000000...
   Final prompt length: 198 chars
✅ Successfully parsed typed step: Prompt { ... }
```

**Interpretation**:
- ✅ If you see "using output from step X" → chaining is working
- ❌ If you see "running standalone" → chaining is NOT working

### Test 2: With Real LLM (Ollama)

To see actual readable context chaining:

1. Make sure Ollama is running with a model (e.g., llama2)

2. Create a 2-step Prompt chain with **real model**:
   - **Step 1**: Prompt (llama2)
     - Prompt: "Tell me a short joke about programming"
     - Use Output From: None

   - **Step 2**: Prompt (llama2)
     - Prompt: "Explain why that joke is funny"
     - Use Output From: Step 1

3. Execute and check:
   - Terminal should show Step 2 receiving Step 1's joke as context
   - Step 2's output should reference the specific joke from Step 1

**Expected Context Format**:
```
Explain why that joke is funny

--- Context from previous step ---
Why do programmers prefer dark mode?
Because light attracts bugs! 🐛
```

## Expected Behavior

### Prompt → Prompt Chaining SHOULD Work

**When it works**:
- Step 1 generates output (either stub hash or real LLM response)
- Step 2 receives that output as context
- Step 2's prompt = original prompt + separator + Step 1's output
- Step 2's LLM sees the full combined prompt

**When it might not seem to work**:
1. **Using stub-model**: Output is hex hash, looks like gibberish
   - This is expected behavior
   - Chaining still works technically, just not human-readable

2. **LLM ignoring context**: Some models might not follow instructions well
   - Try more explicit prompts: "Based on the text below, ..."
   - Try different models (some are better at following context)

3. **Context not being added**: Check debug logs
   - Should see "using output from step X"
   - Should see "Source output length: X chars"

## Troubleshooting

### Issue: "General answer, not using context"

**Possible Causes**:

1. **Stub model** → Output is hex hash, not useful for chaining
   - **Solution**: Use real LLM model (Ollama)

2. **LLM not following instructions** → Model doesn't understand task
   - **Solution**: Make prompt more explicit:
     ```
     Based on the previous response shown below, explain why it's funny.

     Previous response:
     {context will be appended here}
     ```

3. **Context not being added** → Bug in chaining logic
   - **Solution**: Check debug logs for "🔗 using output from"
   - If missing, report with logs

### Issue: "Hex gibberish in output"

**Cause**: Using stub-model for testing

**Solution**:
- This is expected for stub-model
- Switch to real LLM (Ollama) for readable chaining
- Or accept that stub-model is for testing structure, not content

## Summary

**Does Prompt → Prompt chaining work?**

✅ **YES** - The implementation is correct and should work.

**Why might it seem like it's not working?**

1. **Stub-model output is hex** → Not human-readable but technically chained
2. **LLM ignoring context** → Model behavior, not code issue
3. **Need clearer prompts** → Explicitly reference "the text below" or "previous response"

**How to verify it's working?**

1. ✅ Check debug logs for "🔗 using output from step X"
2. ✅ Check "Source output length" is > 0
3. ✅ Use real LLM model to see actual text chaining

**Next Steps**:

1. Run the test with debug logging
2. Share the console output
3. Try with a real Ollama model to see readable chaining
4. If still not working, we can add more debugging

## Files Modified

- `src-tauri/src/orchestrator.rs` (lines 1817-1830): Added debug logging for prompt chaining

## Status

✅ Code is correct, chaining should work
⏳ Waiting for debug logs to confirm behavior
📝 Documentation provided for troubleshooting
