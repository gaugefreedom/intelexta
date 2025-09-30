# Governance System Testing Guide

## Overview
This guide covers testing the enhanced governance system, including token budgets, USD budgets, Nature Cost tracking, policy enforcement, and incident checkpoint creation.

---

## Prerequisites

1. **Start the Application:**
   ```bash
   cd /home/marcelo/Documents/codes/gaugefreedom/intelexta/app
   npm run dev
   ```

2. **In a separate terminal, start Tauri:**
   ```bash
   cd /home/marcelo/Documents/codes/gaugefreedom/intelexta/src-tauri
   cargo tauri dev
   ```

3. **Optional - Start Ollama** (for local model testing):
   ```bash
   ollama serve
   ```

---

## Test Suite

### Test 1: Create Project and View Default Policy

**Steps:**
1. Click "New Project" in the UI
2. Enter a project name (e.g., "Governance Test")
3. Select the newly created project
4. View the **Context** panel on the left

**Expected Results:**
- ✅ Project created successfully
- ✅ Default policy values displayed:
  - `Allow network access`: **unchecked** (disabled)
  - `Token Budget`: **1000**
  - `USD Budget`: **10.0**
  - `Nature Cost`: **100.0**

---

### Test 2: Token Budget Enforcement (Blocking)

**Setup:**
1. Select your test project
2. In Context panel, set budgets:
   - **Token Budget**: `50` (very low)
   - **USD Budget**: `10.0`
   - **Nature Cost**: `100.0`
3. Click "Save Policy"

**Test Steps:**
1. Click "New Run"
2. Enter run name: "Token Budget Test"
3. Click "Add Step"
4. Configure step:
   - **Model**: `stub-model`
   - **Prompt**: `Test prompt for budget enforcement`
   - **Token Budget**: `100` (exceeds project budget of 50)
5. Click "Execute Full Run"

**Expected Results:**
- ✅ Execution **stops before running**
- ✅ **Incident checkpoint** created with:
  - `kind`: "budget_projection_exceeded"
  - `severity`: "error"
  - `details`: Shows "tokens 100 > 50"
- ✅ No output generated
- ✅ Red warning in Context panel shows budget overrun

---

### Test 3: USD Budget Enforcement (Blocking)

**Setup:**
1. In Context panel, set:
   - **Token Budget**: `10000` (high)
   - **USD Budget**: `0.05` (very low)
   - **Nature Cost**: `100.0`
2. Click "Save Policy"

**Test Steps:**
1. Create new run: "USD Budget Test"
2. Add step with:
   - **Token Budget**: `6000` (will cost ~$0.06 at $0.01/1k tokens)
3. Execute run

**Expected Results:**
- ✅ Execution **stops before running**
- ✅ **Incident checkpoint** with "budget_projection_exceeded"
- ✅ Details show "USD 0.06 > 0.05"

---

### Test 4: Nature Cost Warning (Non-Blocking)

**Setup:**
1. In Context panel, set:
   - **Token Budget**: `10000` (high)
   - **USD Budget**: `10.0` (high)
   - **Nature Cost**: `0.5` (very low - will trigger warning)
2. Click "Save Policy"

**Test Steps:**
1. Create new run: "Nature Cost Warning Test"
2. Add step with:
   - **Model**: `stub-model`
   - **Prompt**: `Testing nature cost warning`
   - **Token Budget**: `1000` (will generate ~1.0 Nature Cost)
3. Execute run

**Expected Results:**
- ✅ **Warning incident checkpoint** created with:
  - `kind`: "nature_cost_warning"
  - `severity`: "warn"
  - `details`: Shows "Nature Cost 1.0 exceeds budget 0.5 (execution allowed)"
- ✅ **Execution continues** despite warning
- ✅ **Step completes** successfully
- ✅ Both warning checkpoint AND step checkpoint visible in Inspector
- ✅ Yellow warning indicator in Inspector

---

### Test 5: Network Policy Enforcement (Blocking)

**Setup:**
1. In Context panel, **uncheck** "Allow network access"
2. Set reasonable budgets (all high enough to pass)
3. Click "Save Policy"

**Test Steps:**
1. Create new run: "Network Policy Test"
2. Add step with:
   - **Model**: `claude-3-5-sonnet-20241022` (requires network)
   - **Prompt**: `Test network blocking`
   - **Token Budget**: `100`
3. Execute run

**Expected Results:**
- ✅ Execution **stops before network call**
- ✅ **Incident checkpoint** created with:
  - `kind`: "network_denied"
  - `severity`: "error"
  - `details`: "Network access denied by project policy"
- ✅ No mock response generated

---

### Test 6: Claude Mock API (With Network Enabled)

**Setup:**
1. In Context panel, **check** "Allow network access"
2. Set high budgets (to avoid blocking)
3. Click "Save Policy"

**Test Steps:**
1. Create new run: "Claude Mock Test"
2. Add step with:
   - **Model**: `claude-3-5-haiku-20241022`
   - **Prompt**: `What is the capital of France?`
   - **Token Budget**: `1000`
3. Execute run

**Expected Results:**
- ✅ Execution succeeds
- ✅ Mock response generated: `[MOCK CLAUDE RESPONSE - Model: claude-3-5-haiku-20241022]...`
- ✅ Token usage estimated from text length
- ✅ Checkpoint shows inputs/outputs SHA256
- ✅ Full mock response visible in Inspector

---

### Test 7: Local Ollama Model (If Available)

**Prerequisites:**
- Ollama running (`ollama serve`)
- Model installed (e.g., `ollama pull llama3.2:1b`)

**Setup:**
1. Enable network access (Ollama uses localhost:11434)
2. Set high budgets

**Test Steps:**
1. Create new run: "Ollama Test"
2. Add step with:
   - **Model**: Select your Ollama model from dropdown
   - **Prompt**: `Write a haiku about programming`
   - **Token Budget**: `1000`
3. Execute run

**Expected Results:**
- ✅ Real LLM response generated
- ✅ Actual token usage reported
- ✅ Checkpoint contains real output

---

### Test 8: Multi-Step Run with Mixed Budgets

**Setup:**
1. Set budgets:
   - **Token Budget**: `250`
   - **USD Budget**: `10.0`
   - **Nature Cost**: `0.5`

**Test Steps:**
1. Create run: "Multi-Step Budget Test"
2. Add 3 steps:
   - **Step 1**: Token Budget: `80`, Model: `stub-model`
   - **Step 2**: Token Budget: `80`, Model: `stub-model`
   - **Step 3**: Token Budget: `100`, Model: `stub-model`
   (Total: 260 tokens - exceeds project budget)
3. Execute run

**Expected Results:**
- ✅ Step 1 executes successfully
- ✅ Step 2 executes successfully
- ✅ **Before Step 3**: Incident checkpoint created (cumulative budget exceeded)
- ✅ Step 3 **does not execute**
- ✅ Inspector shows 2 successful steps + 1 incident

---

### Test 9: Budget Updates Mid-Workflow

**Test Steps:**
1. Create a run with token budget: `100`
2. Execute it (should succeed)
3. Change project policy: Token Budget to `50`
4. Click "Execute Full Run" again on the same run

**Expected Results:**
- ✅ First execution succeeds
- ✅ Second execution **blocked** by new lower budget
- ✅ Separate execution records in Inspector
- ✅ Policy change takes effect immediately

---

### Test 10: Inspector Incident Visibility

**Test Steps:**
1. Trigger any budget violation (from tests above)
2. Open **Inspector** panel (right side)
3. Click on the incident checkpoint

**Expected Results:**
- ✅ Incident checkpoints have distinct visual indicator (⚠️ icon or different color)
- ✅ Incident details show:
  - `kind` field
  - `severity` level
  - Full `details` message
- ✅ No inputs/outputs SHA (incident occurred before execution)
- ✅ `usage_tokens`: 0

---

## Model Selection Guide

Available in dropdown:
- **stub-model**: Deterministic test model (no network)
- **claude-3-5-sonnet-20241022**: Mock external API (requires network policy)
- **claude-3-5-haiku-20241022**: Mock external API (requires network policy)
- **claude-3-opus-20240229**: Mock external API (requires network policy)
- **[Ollama models]**: Real local LLMs (if Ollama running)

---

## Verification Checklist

After running all tests, verify:

- [ ] Token budget violations **block** execution
- [ ] USD budget violations **block** execution
- [ ] Nature Cost violations **warn** but **allow** execution
- [ ] Network policy is enforced for Claude models
- [ ] stub-model works without network access
- [ ] Ollama models work with network access enabled
- [ ] Incident checkpoints are signed and auditable
- [ ] Cost estimates update in real-time in Context panel
- [ ] Multiple executions create separate records in Inspector
- [ ] Policy changes take effect immediately on next execution

---

## Troubleshooting

**Issue: Rust compilation errors**
- Run: `cd src-tauri && cargo clean && cargo build`

**Issue: Network policy blocks Ollama**
- Ollama is treated as network access - enable "Allow network access"

**Issue: Models not showing in dropdown**
- Ensure Ollama is running: `ollama serve`
- Check Ollama models: `ollama list`

**Issue: Nature Cost always 0**
- This is expected if using `stub-model` with completion_tokens=10
- Try larger token budgets or real models for meaningful metrics

---

## Expected Behavior Summary

| Budget Type   | Violation Behavior | Creates Incident? | Blocks Execution? |
|---------------|-------------------|-------------------|-------------------|
| **Token**     | Error             | ✅ Yes            | ✅ Yes            |
| **USD**       | Error             | ✅ Yes            | ✅ Yes            |
| **Nature Cost** | Warning         | ✅ Yes            | ❌ No             |
| **Network**   | Error             | ✅ Yes            | ✅ Yes            |

---

## Future Enhancements Tested

✅ **Placeholder Nature Cost algorithm** - Ready for user configuration
✅ **Mock Claude API** - Foundation for real external API integration
✅ **Network policy enforcement** - Distinguishes local vs. remote execution
✅ **Multi-budget governance** - Tokens, USD, and Nature Cost tracked independently

---

## Questions or Issues?

If you encounter unexpected behavior, check:
1. Console logs in browser DevTools
2. Terminal running `cargo tauri dev` for Rust errors
3. Policy values saved correctly (refresh project list)
4. Execution history in Inspector panel