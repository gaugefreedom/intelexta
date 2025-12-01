# Intelexta Development Roadmap

**Last Updated**: 2025-10-09
**Current Status**: Phase 1 MVP Complete

---

## Mission

Build a local-first control plane for verifiable Human+AI workflows with cryptographic integrity, reproducibility, and governance.

**Core Principles**:
- **"Proof, not vibes"** - Everything is cryptographically verifiable
- **"Exact where possible, accountable where not"** - Determinism when feasible, traceable when not

---

## Phase 1: Cryptographic Integrity ✅ COMPLETE

**Goal**: Establish tamper-evident proof that workflows executed as claimed.

### Completed Features

#### 1.1 Provenance System ✅
- [x] Ed25519 signature generation for all checkpoints
- [x] SHA-256 hash chains linking workflow execution
- [x] Canonical JSON (JCS) for deterministic hashing
- [x] Base64-encoded keys and signatures
- [x] Content-addressed storage for outputs

**Implementation**:
- `src-tauri/src/provenance.rs` - Signing and hash chain logic
- `src-tauri/src/orchestrator.rs` - Checkpoint creation during workflow execution

#### 1.2 CAR Export System ✅
- [x] JSON export (`.car.json`) for standalone receipts
- [x] ZIP export (`.car.zip`) with full output attachments
- [x] Provenance claims (config, inputs, outputs)
- [x] Policy versioning and model catalog snapshots
- [x] S-Grade scoring (sustainability metrics)

**Implementation**:
- `src-tauri/src/car.rs` - CAR structure and export logic
- `src-tauri/src/api.rs` - Export API endpoints
- `schemas/car-v0.3.schema.json` - Current CAR schema (v0.3)
- `schemas/car-v0.2.schema.json` - Legacy CAR schema (deprecated)

#### 1.3 Standalone Verification Tool ✅
- [x] `intelexta-verify` CLI for trustless verification
- [x] Hash chain verification
- [x] Signature verification
- [x] Config integrity (prompts, models)
- [x] Attachment integrity (outputs)
- [x] Human-readable and JSON output formats

**Implementation**:
- `src-tauri/crates/intelexta-verify/` - Standalone verification crate

**Key Achievement**: Third parties can now verify AI workflow proofs without trusting the creator or running the full application.

---

## Phase 2: Graded Replay

**Goal**: Enable reproducibility verification by re-executing workflows and comparing outputs.

### Planned Features

#### 2.1 Workflow Parser
- [x] Parse CAR files to extract workflow specification
- [x] Reconstruct step-by-step execution plan
- [x] Handle multi-step dependencies and data flow

**Design Notes**:
- Extract from `car.run.steps` array
- Support all step types: `llm`, `tool`, `decision`, `loop`
- Preserve original prompts, models, and parameters

#### 2.2 Model Adapter Integration
- [x] Use existing model adapter system from main app
- [x] Support API key injection via environment variables
- [x] Handle rate limiting and API errors gracefully

**Environment Variables**:
```bash
OPENAI_API_KEY=sk-...
ANTHROPIC_API_KEY=sk-ant-...
OLLAMA_HOST=http://localhost:11434
```

#### 2.3 Similarity Scoring
- [x] Exact match detection (deterministic models)
- [x] Semantic similarity for non-deterministic outputs
- [ ] Token-level diff for structured outputs (JSON, code)

**Scoring Rubric**:
- **A+** (100%): Byte-for-byte exact match
- **A** (95-99%): High semantic similarity
- **B** (80-94%): Good similarity with minor variations
- **C** (60-79%): Partial match, significant drift
- **F** (<60%): Failed to reproduce

**Implementation Options**:
- Embedding-based similarity (sentence-transformers)
- BLEU/ROUGE scores for text generation
- JSON structural comparison for structured outputs

#### 2.4 Graded Report Generation
- [x] Per-step replay results
- [x] Aggregate reproducibility score
- [ ] Diff visualization for failed reproductions
- [ ] Export graded report to JSON

**Output Format**:
```json
{
  "car_id": "car:abc123...",
  "replay_timestamp": "2025-10-09T12:00:00Z",
  "overall_grade": "A",
  "overall_score": 97.5,
  "steps": [
    {
      "step_id": "e111d25f-...",
      "grade": "A+",
      "score": 100,
      "exact_match": true
    },
    {
      "step_id": "a222b33c-...",
      "grade": "B",
      "score": 85.3,
      "similarity_details": {
        "semantic_similarity": 0.853,
        "length_ratio": 0.95,
        "diff": "..."
      }
    }
  ]
}
```

#### 2.5 Verification CLI Extension
- [ ] Add `--replay` flag to `intelexta-verify`
- [ ] Network access for model API calls
- [ ] Progress indicators for long-running replays

**Usage**:
```bash
# Phase 1: Integrity verification (current)
intelexta-verify proof.car.zip

# Phase 2: Integrity + reproducibility verification
intelexta-verify proof.car.zip --replay --api-keys-from-env
```

**Files to Modify**:
- `src-tauri/crates/intelexta-verify/src/main.rs` - Add replay subcommand
- `src-tauri/crates/intelexta-verify/Cargo.toml` - Add model adapter dependencies
- Create `src-tauri/crates/intelexta-verify/src/replay.rs` - Replay logic

---

## Phase 3: Visualization & Insights 📊 FUTURE

**Goal**: Make verification results understandable and actionable through rich visualizations.

### Planned Features

#### 3.1 Web-Based Verification Viewer
- [ ] Static HTML/JS viewer for CAR files
- [ ] Interactive workflow graph visualization
- [ ] Step-by-step execution timeline
- [ ] Embedded in exported CAR archives

**Tech Stack**:
- D3.js or Cytoscape.js for graph visualization
- React or vanilla JS for UI
- No backend required (client-side only)

**Deliverable**: `viewer.html` embedded in `.car.zip` archives

#### 3.2 Diff Visualization
- [ ] Side-by-side comparison of expected vs actual outputs
- [ ] Syntax highlighting for code/JSON diffs
- [ ] Semantic similarity heatmaps

**Use Cases**:
- Debugging failed reproductions
- Understanding model drift over time
- Comparing outputs across different models

#### 3.3 Provenance Graph Explorer
- [ ] Visualize hash chain as directed graph
- [ ] Click to inspect checkpoint details
- [ ] Highlight signature verification status

**Example**:
```
[Genesis] → [Step 1: LLM] → [Step 2: Tool] → [Step 3: LLM] → [Final]
   ✅          ✅               ✅              ✅             ✅
```

#### 3.4 S-Grade Dashboard
- [ ] Energy consumption breakdown by step
- [ ] Cost analysis (USD, tokens, nature cost)
- [ ] Model efficiency comparison
- [ ] Export sustainability report

**Metrics to Visualize**:
- Total tokens vs nature cost
- Energy per output token
- Cost-effectiveness by model
- Consent score breakdown

#### 3.5 Batch Verification Dashboard
- [ ] Verify multiple CARs in parallel
- [ ] Generate compliance reports for audits
- [ ] Export aggregated statistics (CSV, JSON)

**Use Cases**:
- Regulatory compliance (verify all AI-generated outputs)
- Quality assurance in production pipelines
- Research reproducibility audits

---

## Phase 4: Advanced Governance 🛡️ FUTURE

**Goal**: Fine-grained policy control over model usage, costs, and consent.

### Planned Features

#### 4.1 Policy-as-Code
- [ ] YAML/TOML policy definitions
- [x] Per-project policies
- [ ] Per-workflow policies
- [x] Policy versioning 
- [  ] Policy migration

**Example Policy**:
```toml
[policy]
version = "1.0"
name = "Research Project Alpha"

[limits]
max_usd_per_run = 5.00
max_tokens_per_step = 4000
max_nature_cost = 0.5

[restrictions]
allowed_models = ["claude-3-5-sonnet", "gpt-4o"]
require_consent = true
require_signatures = true
```

#### 4.2 Dynamic Model Selection
- [ ] Cost-aware model routing (use cheaper models when sufficient)
- [ ] Quality threshold enforcement (fallback to better models if needed)
- [ ] Load balancing across providers

#### 4.3 Consent Management
- [ ] User confirmation for high-cost operations
- [ ] Budget approval workflows
- [ ] Audit logs for policy violations

#### 4.4 Multi-User Governance
- [ ] Role-based access control (RBAC)
- [ ] Project sharing with permission scopes
- [ ] Collaborative workflow approval

---

## Phase 5: Ecosystem Integration 🌐 FUTURE

**Goal**: Make Intelexta the standard for verifiable AI workflows across tools and platforms.

### Planned Features

#### 5.1 CAR Publishing & Discovery
- [ ] Public CAR registry (opt-in)
- [ ] IPFS integration for decentralized storage
- [ ] Verification badges for published CARs

#### 5.2 IDE Integrations
- [ ] VS Code extension for CAR inspection
- [ ] JetBrains plugin for workflow verification
- [ ] CLI hooks for git pre-commit verification

#### 5.3 CI/CD Pipelines
- [ ] GitHub Actions workflow for CAR verification
- [ ] GitLab CI template
- [ ] Pre-built Docker images for verification

**Example GitHub Action**:
```yaml
- name: Verify AI Workflow Proof
  uses: intelexta/verify-action@v1
  with:
    car-file: outputs/proof.car.zip
    fail-on-invalid: true
```

#### 5.4 Third-Party Auditor Tools
- [ ] API for automated verification services
- [ ] Bulk verification endpoints
- [ ] Verification certificate generation

---

## Technical Debt & Maintenance 🔧

### Ongoing Tasks

#### Code Quality
- [ ] Expand test coverage for `intelexta-verify`
- [ ] Add integration tests for CAR export/import
- [ ] Performance benchmarks for large workflows

#### Documentation
- [x] Comprehensive README for `intelexta-verify` ✅
- [ ] API documentation for main app
- [ ] Video tutorials for end users
- [ ] Developer onboarding guide

#### Security
- [ ] Security audit of cryptographic implementation
- [ ] Penetration testing of CAR verification logic
- [ ] Threat model documentation

#### Performance
- [ ] Optimize large CAR file parsing
- [ ] Parallel signature verification
- [ ] Streaming attachment verification for huge files

---

## Research Questions 🔬

### Open Problems

1. **Reproducibility Guarantees**
   - How do we handle non-deterministic models (temperature > 0)?
   - What similarity threshold constitutes "reproduced"?
   - How to detect intentional vs unintentional drift?

2. **Energy Accounting**
   - How accurate are our nature cost estimates?
   - Can we integrate with actual datacenter energy metrics?
   - How to account for embodied carbon in hardware?
   - Connect user to Carbon Market for pay back?

3. **Governance Trade-offs**
   - How to balance control with flexibility?
   - When should policies be enforced vs advisory?
   - How to handle policy conflicts in collaborative settings?

4. **Ecosystem Adoption**
   - What incentives drive users to adopt verifiable workflows?
   - How to make verification valuable without being burdensome?
   - Can we create network effects around verified AI outputs?

---

## How to Contribute

### For Phase 2 (Graded Replay)

**Start Here**:
1. Read `src-tauri/crates/intelexta-verify/README.md` to understand current implementation
2. Examine `src-tauri/src/orchestrator.rs` to see how workflows are executed in the main app
3. Explore `src-tauri/src/model_adapters.rs` for model API integration patterns

**Key Files to Create**:
- `src-tauri/crates/intelexta-verify/src/replay.rs` - Replay orchestration logic
- `src-tauri/crates/intelexta-verify/src/similarity.rs` - Output comparison algorithms
- `src-tauri/crates/intelexta-verify/src/grading.rs` - Scoring and report generation

**Testing Strategy**:
1. Create test CARs with known outputs
2. Run replay with same models/prompts
3. Verify similarity scores are reasonable
4. Handle edge cases (API failures, timeouts, invalid keys)

### For Phase 3 (Visualization)

**Start Here**:
1. Study example CAR files to understand data structure
2. Sketch mockups for workflow graph and diff views
3. Choose visualization libraries (D3.js, Cytoscape, etc.)

**Deliverable**:
- Single-file HTML viewer that can be embedded in CAR archives
- No external dependencies (bundle everything)
- Works offline (no API calls)


## Intelexta Apps & Live Deployments

This repository is the **reference implementation** of the Intelexta Protocol (Desktop Node + schemas + verifier tools).  
On top of it, I am developing several apps and integrations that demonstrate how the protocol can be used in practice:

### Desktop Node (this repo)

- **Intelexta Desktop (Local-First Control Plane)**  
  The Tauri + Rust application that runs workflows locally, signs provenance, and exports **CAR** (Content-Addressable Receipt) bundles.
- **`intelexta-verify` CLI**  
  Standalone verifier (also in this repo) for trustless checking of CAR files.  
  This is the cryptographic “truth engine” behind the other apps.

### Web Verifier (same repo)

- **Intelexta Web Verifier** – `apps/web-verifier`  
  A Vite + React frontend that loads the verifier as WebAssembly in the browser.  
  It powers the public site:

  - 🌐 **Live instance**: `https://verify.intelexta.com`  
    Drop a `*.car.json` or `*.car.zip` to:
    - Verify signatures and hash chains
    - Inspect workflows and proof metadata in a human-friendly UI

### Verifiable Summary (same repo)

- **Verifiable Summary MCP Server** – `apps/verifiable-summary`  
  An OpenAI Apps SDK / MCP integration that:
  - Accepts content (text or file) and a summary style (TL;DR, bullets, outline)
  - Produces a summary **plus** a signed CAR bundle
  - Exposes a widget inside ChatGPT to download and verify proofs

  This shows how Intelexta’s **CAI/CAR schemas** can be embedded directly into agentic workflows.

### Hosted Validator (separate app)

- **Intelexta Validator** – `https://validator.intelexta.com`  
  A hosted application that uses part of the same ideas (signed receipts, verifiable runs, structured reports) but **lives in a separate codebase** from this monorepo.

  It predates the current Desktop Node and serves as:
  - An early **productized validator** for external users
  - A proving ground for UX patterns and reporting formats that later informed the Intelexta Protocol

---

### Where to Start

- If you want to **inspect the protocol and code**: start in this repo (`src-tauri`, `schemas`, `intelexta-verify`).
- If you want to **see it in action** as a user:
  - Visit **`https://verify.intelexta.com`** and drop an example CAR.
  - Use **`verifiable-summary`** inside ChatGPT to generate summaries with signed receipts.
  - Explore **`https://validator.intelexta.com`** as an early hosted validator built on the same concepts.
