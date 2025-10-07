# Intelexta MVP Analysis & Roadmap

**Date**: 2025-10-07
**Target**: First investor/user-ready version
**Vision**: "Proof, not vibes" - Verifiable Human+AI workflows

---

## Executive Summary

Intelexta is positioned as a **local-first control plane for verifiable AI workflows**. The typed steps system is now functional, enabling document ingestion and multi-step AI processing with full provenance tracking.

**Current State**: ~70% MVP-ready
**Key Gaps**: Governance robustness, CAR/replay completeness, verification tools
**Target Timeline**: 4-6 weeks to investor-ready MVP

---

## What We Have: Working Features

### ✅ Core Workflow Engine

1. **Typed Step System** (COMPLETE)
   - Ingest Document: PDF, LaTeX, TXT, DOCX processing
   - Summarize: AI-powered summarization with chaining
   - Prompt: Custom LLM queries with optional context
   - **Status**: Fully functional, tested, production-ready

2. **Step Chaining** (COMPLETE)
   - Outputs flow from step N to step N+1
   - Dynamic step selection with smart filtering
   - Multiple model support (stub, mock, real LLM)
   - **Status**: Working end-to-end

3. **Workflow Builder UI** (COMPLETE)
   - Create/edit runs as sequences of steps
   - Visual step list with reordering
   - Clear configuration for each step type
   - **Status**: User-friendly, functional

4. **Execution System** (COMPLETE)
   - Execute full runs with multiple steps
   - Prior output tracking for chaining
   - Error handling and incident creation
   - **Status**: Reliable execution

### ⚠️ Governance System (PARTIAL)

**What Exists**:
- Token budget enforcement (`governance.rs`)
- USD cost estimation (basic)
- Nature Cost estimation (placeholder)
- Network policy enforcement
- Policy validation during execution

**What's Missing**:
- **Model cost configuration** (currently hardcoded)
- **Per-model pricing** (different models have different costs)
- **Nature Cost algorithms** (user-configurable)
- **Budget tracking UI** (show remaining budget)
- **Cost projections** (estimate before execution)

**Current Implementation** (`governance.rs`):
```rust
// USD: ~$0.01 per 1000 tokens (hardcoded)
pub fn estimate_usd_cost(tokens: u64) -> f64 {
    const COST_PER_1K_TOKENS: f64 = 0.01;
    (tokens as f64 / 1000.0) * COST_PER_1K_TOKENS
}

// Nature Cost: 1.0 unit per 1000 tokens (placeholder)
pub fn estimate_nature_cost(tokens: u64) -> f64 {
    const BASE_NATURE_COST_PER_1K_TOKENS: f64 = 1.0;
    (tokens as f64 / 1000.0) * BASE_NATURE_COST_PER_1K_TOKENS
}
```

### ⚠️ CAR Export & Replay (PARTIAL)

**What Exists**:
- CAR structure defined (`car.rs`)
- Checkpoint signatures
- Provenance claims
- S-Grade calculation
- Replay infrastructure (`replay.rs`)

**What's Missing**:
- **Step outputs not saved** (only hashes stored)
- **CAR export UI** (no button to export)
- **Replay grading** (pass/fail → similarity scoring)
- **Concordant mode validation** (epsilon checking)
- **Interactive checkpoint replay** (not for V1)

**Critical Gap**: Without saved outputs, CAR replay cannot work for verification!

### ⚠️ Inspector & Verification (PARTIAL)

**What Exists**:
- Execution history display
- Checkpoint details view
- Signature verification
- Hash chain validation

**What's Missing**:
- **Output display** (can't see what steps produced)
- **Replay results comparison** (side-by-side view)
- **Similarity metrics** (for concordant mode)
- **Export/import buttons** (CAR and IXP)
- **Verification report** (automated checks)

---

## Critical Gaps for MVP

### Priority 1: MUST HAVE (Blockers)

#### 1.1 Save Step Outputs ⚠️ **CRITICAL**

**Problem**: Currently only hashes are stored, not the actual output text.

**Impact**:
- CAR export incomplete (no outputs to verify)
- Replay cannot compare results
- Users can't see what was produced
- Verification is impossible

**Solution**:
```sql
-- Already exists in schema:
CREATE TABLE checkpoints (
    ...
    output_payload TEXT,  -- ✅ Column exists
    ...
);
```

Just need to ensure `output_payload` is:
1. Populated during execution
2. Included in CAR export
3. Displayed in Inspector

**Estimated Work**: 1-2 days

#### 1.2 Model Cost Configuration 💰

**Problem**: Hardcoded $0.01/1K tokens doesn't reflect real pricing.

**Impact**:
- Budget estimates are wrong
- Users can't control costs accurately
- "Selling point" of cost control is undermined

**Solution**: Config file with per-model pricing

```json
// models.json
{
  "models": [
    {
      "id": "llama3.2:1b",
      "provider": "ollama",
      "costPerMillionTokens": 0.0,  // Local = free
      "natureCostPerMillionTokens": 2.5,  // Based on energy
      "description": "Local Llama 3.2 1B"
    },
    {
      "id": "gpt-4",
      "provider": "openai",
      "costPerMillionTokens": 30.0,
      "natureCostPerMillionTokens": 15.0,
      "description": "OpenAI GPT-4"
    }
  ],
  "natureCostAlgorithms": {
    "simple": "tokens * model.natureCostPerMillionTokens / 1000000",
    "detailed": "... custom formula ..."
  }
}
```

**Estimated Work**: 2-3 days

#### 1.3 CAR Export UI 📦

**Problem**: No way to export CARs from the UI.

**Impact**:
- Core use case (sharing proofs) is blocked
- Can't demonstrate verification to investors

**Solution**: Add "Export CAR" button to Inspector

```typescript
// In Inspector view
<button onClick={() => exportCAR(executionId)}>
  📦 Export CAR
</button>
```

**Backend** already has CAR building logic, just need:
1. Export button in UI
2. File save dialog
3. Call existing `build_car()` function

**Estimated Work**: 1 day

#### 1.4 Replay Grading System 🎯

**Problem**: Pass/fail doesn't work for stochastic LLM outputs.

**Impact**:
- Concordant mode verification is incomplete
- Can't meaningfully verify AI workflows

**Solution**: Similarity scoring instead of binary pass/fail

**Current** (`replay.rs`):
```rust
pub match_status: bool,  // ❌ Binary
```

**Needed**:
```rust
pub struct ReplayReport {
    pub match_status: MatchStatus,
    pub similarity_score: f64,  // 0.0 - 1.0
    pub grade: ReplayGrade,     // A+ to F
}

pub enum MatchStatus {
    ExactMatch,              // 100% match
    WithinEpsilon(f64),      // Concordant, within threshold
    BelowEpsilon(f64),       // Concordant, failed threshold
    Different,               // Significantly different
}

pub enum ReplayGrade {
    APla's,  // 95-100% similarity
    A,       // 90-95%
    B,       // 80-90%
    C,       // 70-80%
    D,       // 60-70%
    F,       // < 60%
}
```

**Estimated Work**: 3-4 days

### Priority 2: SHOULD HAVE (Polish)

#### 2.1 Budget Tracking UI 📊

**Problem**: Users can't see remaining budgets.

**Solution**: Dashboard showing:
```
Project Budgets:
  Token Budget: 5,234 / 10,000 used (52%)
  USD Budget: $2.15 / $10.00 used (21%)
  Nature Cost: 12.3 / 50.0 units used (24%)
```

**Estimated Work**: 2 days

#### 2.2 Output Preview in Inspector 👁️

**Problem**: Can't see what steps produced.

**Solution**: Display `output_payload` in checkpoint details:
- Syntax highlighting for JSON
- Truncation for long outputs
- Copy button

**Estimated Work**: 1 day

#### 2.3 Cost Projection 💡

**Problem**: Don't know cost until after execution.

**Solution**: Show estimated cost before clicking "Execute":
```
Estimated Run Cost:
  Tokens: ~2,500 tokens
  USD: ~$0.75
  Nature Cost: ~6.2 units

  Within budget? ✅
```

**Estimated Work**: 2 days

#### 2.4 IXP Export/Import 📁

**Problem**: Can't share full projects.

**Solution**:
- Export button: Saves `.ixp` with runs, policies, history
- Import button: Loads project from `.ixp`
- Already have `portability.rs` infrastructure

**Estimated Work**: 2-3 days

### Priority 3: NICE TO HAVE (Future)

#### 3.1 Visual Workflow Builder 🎨

Drag-and-drop step creation, visual flow diagram.

**Estimated Work**: 1-2 weeks

#### 3.2 Step Templates 📑

Save/reuse common workflows.

**Estimated Work**: 3-5 days

#### 3.3 Batch Processing 🔄

Run same workflow on multiple files.

**Estimated Work**: 5-7 days

#### 3.4 intelexta-verify CLI 🔧

Standalone verification tool.

**Estimated Work**: 1-2 weeks

---

## MVP Roadmap: 4-6 Week Plan

### Week 1: Critical Blockers
**Goal**: Make verification actually work

- [ ] Day 1-2: Save step outputs in checkpoints ⚠️ **CRITICAL**
- [ ] Day 3-4: Model cost configuration file 💰
- [ ] Day 5: CAR export UI button 📦

**Deliverable**: Can export CARs with full outputs

### Week 2: Governance & Replay
**Goal**: Robust cost control and verification

- [ ] Day 1-2: Nature Cost algorithms (user-configurable)
- [ ] Day 3-5: Replay grading system (similarity scores) 🎯

**Deliverable**: Meaningful replay reports with grades

### Week 3: UI Polish
**Goal**: Professional, investor-ready interface

- [ ] Day 1-2: Budget tracking UI 📊
- [ ] Day 3-4: Output preview in Inspector 👁️
- [ ] Day 5: Cost projection before execution 💡

**Deliverable**: Users can see budgets, costs, and outputs clearly

### Week 4: Export/Import & Testing
**Goal**: Complete the sharing workflow

- [ ] Day 1-2: IXP export/import 📁
- [ ] Day 3-4: End-to-end testing
- [ ] Day 5: Bug fixes and polish

**Deliverable**: Can share projects and CARs

### Weeks 5-6: Documentation & Demo
**Goal**: Investor-ready materials

- [ ] Use case documentation
- [ ] Demo workflows
- [ ] User guide
- [ ] Verification walkthrough
- [ ] Pitch materials

**Deliverable**: Complete demo package

---

## Use Cases for Investors/Users

### Use Case 1: Academic Research 🎓

**Persona**: Dr. Sarah Chen, Climate Researcher

**Workflow**:
1. **Ingest**: Load 50-page climate research paper (PDF)
2. **Summarize**: Generate academic summary (methodology + findings)
3. **Prompt**: "What are the policy implications?"
4. **Export**: Share CAR with journal for peer review

**Value Propositions**:
- ✅ **Provenance**: Every AI-generated claim is traceable
- ✅ **Reproducibility**: Reviewers can verify the analysis
- ✅ **Cost Control**: Budget limits prevent runaway LLM costs
- ✅ **Energy Transparency**: Nature Cost shows environmental impact

### Use Case 2: Legal Document Analysis ⚖️

**Persona**: Maria Rodriguez, Legal Researcher

**Workflow**:
1. **Ingest**: Load contract (DOCX)
2. **Summarize**: Extract key terms and obligations
3. **Prompt**: "Identify potential risks in section 4"
4. **Export**: IXP to client showing complete analysis chain

**Value Propositions**:
- ✅ **Audit Trail**: Complete history of analysis steps
- ✅ **Policy Enforcement**: No unauthorized network access
- ✅ **Verifiable**: Signatures prove no tampering
- ✅ **Portable**: Client can replay analysis

### Use Case 3: Content Verification 📰

**Persona**: James Miller, Fact-Checker

**Workflow**:
1. **Ingest**: Load article with claims
2. **Prompt**: "Extract factual claims"
3. **Prompt** (with context): "Assess credibility of each claim"
4. **Export**: CAR showing AI-assisted fact-check with proof

**Value Propositions**:
- ✅ **Transparency**: Shows exactly how AI was used
- ✅ **Accountability**: Budget and policy compliance proven
- ✅ **Trust**: Cryptographic signatures prevent manipulation
- ✅ **Shareable**: Readers can verify the process

---

## Competitive Advantages

### 1. Cost Control (Unique Selling Point)

**Problem**: ChatGPT/Claude bills surprise users with $500 charges.

**Intelexta Solution**:
- Hard budget limits (tokens, USD, nature cost)
- Per-model cost configuration
- Real-time cost projection
- Execution blocked if budget exceeded

**Investor Pitch**: "We give users *control* over AI costs, turning it from an uncertain expense into a managed tool."

### 2. Verifiable Provenance (Unique Selling Point)

**Problem**: "I used AI to help write this" is not verifiable.

**Intelexta Solution**:
- Cryptographic signatures on every step
- Tamper-evident hash chains
- Exportable CAR proofs
- Third-party verification via CAR replay

**Investor Pitch**: "We turn AI work from 'trust me' to mathematically proven provenance."

### 3. Energy Accountability (Unique Selling Point)

**Problem**: AI's environmental cost is hidden.

**Intelexta Solution**:
- Nature Cost tracking
- User-configurable algorithms
- Budget enforcement
- Transparent reporting

**Investor Pitch**: "We make AI's environmental impact a first-class metric, not an externality."

### 4. Local-First Privacy

**Problem**: Cloud AI services store all your data.

**Intelexta Solution**:
- Everything runs locally by default
- Network access disabled unless explicitly allowed
- Keys in OS keychain, not database
- Export when YOU decide

**Investor Pitch**: "Your data never leaves your machine unless you explicitly export it."

---

## Demo Script for Investors

### 5-Minute Live Demo

**Setup**: Research paper analysis workflow

1. **Show the Problem** (30 seconds)
   - "How do you prove you used AI responsibly?"
   - "How do you control AI costs?"
   - "How do you verify someone else's AI work?"

2. **Create Workflow** (2 minutes)
   - Add Ingest step (research paper)
   - Add Summarize step (academic summary)
   - Add Prompt step (policy implications)
   - Show budget limits being set

3. **Execute** (1 minute)
   - Click "Execute Full Run"
   - Show real-time execution
   - Point out budget tracking
   - Highlight nature cost

4. **Verify** (1.5 minutes)
   - Open Inspector
   - Show cryptographic signatures
   - Export CAR
   - Open CAR in text editor (show JSON)
   - Explain: "This is verifiable proof"

5. **The Ask** (30 seconds)
   - "This is the foundation for verifiable AI work"
   - "We're ready for beta users in 6 weeks"
   - "Looking for $X to complete MVP and launch"

---

## Technical Debt to Address

### Code Quality
- [ ] Remove debug `eprintln!` statements (add feature flag)
- [ ] Add comprehensive error messages
- [ ] Improve type safety (fewer `Option<String>`)
- [ ] Add unit tests for chaining logic

### Documentation
- [ ] API documentation
- [ ] Architecture diagrams
- [ ] Developer setup guide
- [ ] Contribution guidelines

### Performance
- [ ] Optimize large document processing
- [ ] Cache repeated LLM calls
- [ ] Database indexing review
- [ ] Memory usage profiling

---

## Success Metrics for MVP

### Functional Metrics
- [ ] Can execute 3+ step workflows reliably
- [ ] CAR export/import works end-to-end
- [ ] Replay grading produces meaningful scores
- [ ] Budget enforcement never fails

### UX Metrics
- [ ] User can create workflow in < 5 minutes
- [ ] Cost projection is accurate within 10%
- [ ] Inspector shows all relevant information
- [ ] No confusing error messages

### Business Metrics
- [ ] Demo runs without crashes
- [ ] Investor questions answerable
- [ ] Use cases are compelling
- [ ] Differentiation is clear

---

## Recommended Action Plan

### Immediate (This Week)
1. **Fix output storage**: Ensure all step outputs are saved
2. **Add CAR export button**: Make sharing possible
3. **Test end-to-end**: Verify complete workflows

### Next 2 Weeks
4. **Model cost config**: Implement per-model pricing
5. **Replay grading**: Add similarity scoring
6. **Budget UI**: Show remaining budgets

### Weeks 3-4
7. **Polish Inspector**: Output preview, better UX
8. **IXP export/import**: Complete sharing workflow
9. **Comprehensive testing**: Fix all edge cases

### Weeks 5-6
10. **Documentation**: User guide, use cases
11. **Demo preparation**: Practice pitch
12. **Beta readiness**: Final polish

---

## Summary

**Current State**: Strong foundation with working typed steps system

**Key Gaps**: Output storage, model costs, replay grading

**Timeline to MVP**: 4-6 weeks with focused effort

**Investor Readiness**: After Week 4, can confidently demo

**Competitive Edge**: Cost control + verifiable provenance + energy accountability = unique position in market

**Next Steps**: Start with Week 1 critical blockers (output storage, CAR export, model costs)

---

**Document Version**: 1.0
**Last Updated**: 2025-10-07
**Status**: Strategic Planning Document
