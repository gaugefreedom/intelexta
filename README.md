# Intelexta

**Local-first control plane for verifiable Human+AI workflows.**  
_Intelexta = Git + Notary for AI workflows._

_Mottos: "Proof, not vibes." · "Exact where possible, accountable where not."_

---

Intelexta is a desktop application for researchers, writers, students, and developers who need to produce **verifiable and reproducible** results from AI workflows. It is not another chat interface; it is a high-integrity control plane built on the principles of:

- **Signed Provenance**
- **Reproducible Pipelines**
- **Local-First Governance**

## What this repository includes

This monorepo contains the reference implementation of the Intelexta Protocol:

- A local-first **Desktop Node** (Rust/Tauri) to run AI workflows under policy
- A **CLI verifier** and **Web Verifier** so third parties can validate CAR receipts
- Core **CAI/CAR schemas** and example apps that make AI outputs verifiable, not just plausible

## Repository Structure

This monorepo contains the reference implementation of the Intelexta Protocol:

- **`/src-tauri`**: The **Desktop Node** (Rust/Tauri). A local-first verified workspace.
- **`/apps/web-verifier`**: The **Public Verifier** (React/Vite). The source code for [verify.intelexta.com](https://verify.intelexta.com).
- **`/apps/verifiable-summary`**: The **AI Agent Integration**. Middleware for generating CARs from LLM outputs.
- **`/schemas`**: Core data structures for **CAI** (certificates) and **CAR** (receipts).  
  _Historical note_: earlier schema work lives in [`intelexta-schema`](https://github.com/…).  
  This monorepo is now the **canonical home** for CAR v0.3 and future protocol evolution.

## Mission Alignment

- **Human+AI Symbiosis:** The human is the strategic director; the AI is a powerful cognitive partner operating within a provable envelope of human-defined policy.
- **Integrity of Knowledge:** Creates a permanent, tamper-evident record of how information is generated, preserving the context and integrity of knowledge.
- **Accountable Efficiency:** Makes energy and carbon consumption a first-class, manageable metric in every workflow.

## Stack

- **Desktop:** Tauri (Rust) + React (Vite)
- **Database:** Local SQLite
- **Provenance:** Ed25519 signatures + SHA-256 hash chains

## Intelexta Apps & Live Deployments

This repository is the **reference implementation** of the Intelexta Protocol (Desktop Node + schemas + verifier tools).  
On top of it, I’m developing several apps and integrations that show how the protocol is used in practice:

### Desktop Node (this repo)

- **Intelexta Desktop (Local-First Control Plane)**  
  The Tauri + Rust application that runs workflows locally, signs provenance, and exports **CAR** (Content-Addressable Receipt) bundles.
- **`intelexta-verify` CLI**  
  Standalone verifier (also in this repo) for trustless checking of CAR files.  
  This is the cryptographic “truth engine” behind the other apps.

### Web Verifier (this repo)

- **Intelexta Web Verifier** – `apps/web-verifier`  
  A Vite + React frontend that loads the verifier as WebAssembly in the browser.  
  It powers the public site:

  - 🌐 **Live instance**: `https://verify.intelexta.com`  
    Drop a `*.car.json` or `*.car.zip` to:
    - Verify signatures and hash chains
    - Inspect workflows and proof metadata in a human-friendly UI

### Verifiable Summary (this repo)

- **Verifiable Summary MCP Server** – `apps/verifiable-summary`  
  An OpenAI Apps SDK / MCP integration that:
  - Accepts content (text or file) and a summary style (TL;DR, bullets, outline)
  - Produces a summary **plus** a signed CAR bundle
  - Exposes a widget inside ChatGPT to download and verify proofs

  This shows how Intelexta’s **CAI/CAR schemas** can be embedded directly into agentic workflows.

### Hosted Validator (separate app)

- **Intelexta Validator** – `https://validator.intelexta.com`  
  A hosted application that uses some of the same ideas (signed receipts, verifiable runs, structured reports) but **lives in a separate codebase** from this monorepo.



## Getting Started

Clone the repository and follow these steps:

```bash
# 1. Run the Frontend
cd app
npm install
npm run dev

# 2. In a separate terminal, run the Backend
cd src-tauri
# NOTE: On Wayland-based Linux systems, you may need to prefix this command.
# See CONTRIBUTING.md for details on graphics driver workarounds.
cargo tauri dev
```

## Detailed Architecture

```text
intelexta/
├─ scripts/
│  ├─ dev.sh
│  └─ build-release.sh
├─ schemas/
│  ├─ car-v0.3.schema.json   # Current JSON Schema for Content-Addressable Receipts
│  └─ car-v0.2.schema.json   # Legacy schema (deprecated)
├─ app/                      # React (Vite) frontend
│  └─ src/
│     ├─ main.tsx
│     ├─ App.tsx
│     ├─ components/
│     │  ├─ ProjectTree.tsx
│     │  ├─ ContextPanel.tsx
│     │  ├─ EditorPanel.tsx
│     │  └─ InspectorPanel.tsx
│     └─ lib/api.ts
└─ src-tauri/                # Tauri + Rust backend
   ├─ Cargo.toml
   ├─ tauri.conf.json
   └─ src/
      ├─ main.rs
      ├─ api.rs
      ├─ store/
      │  ├─ mod.rs
      │  └─ schema.sql
   ├─ governance.rs       # Policy router logic
   ├─ provenance.rs       # Signing and hash-chaining
   └─ orchestrator.rs     # DAG execution engine
```

## Schemas

The canonical JSON Schema for Content-Addressable Receipts (CAR v0.3) lives in [`schemas/car-v0.3.schema.json`](schemas/car-v0.3.schema.json).

**Version History**:
- **v0.3** (current): Added `proof.process` structure with sequential checkpoints, updated `run.steps` field names to camelCase, relaxed checkpoint ID patterns
- **v0.2** (legacy): Original schema - now deprecated, maintained for backward compatibility

See [`schemas/README.md`](schemas/README.md) for detailed schema documentation and migration guide.

## Status

**Phase 1 MVP Complete** ✅ - Full cryptographic integrity verification system operational.

See [ROADMAP.md](ROADMAP.md) for detailed development plan and upcoming features.

## Key Features

### Standalone Verification Tool (v0.2+)
- **`intelexta-verify`**: CLI tool for trustless verification of CAR (Content-Addressed Receipt) files
- **Cryptographic Integrity**: Verifies hash chains, signatures, and content integrity
- **Tamper Detection**: Detects modifications to prompts, models, outputs, or execution metadata
- **No Dependencies**: Works offline without database or network access
- **See**: [src-tauri/crates/intelexta-verify/README.md](src-tauri/crates/intelexta-verify/README.md)

### Document Processing (v0.2+)
- **Multi-format Support**: Process PDF, LaTeX, plain text (TXT), and DOCX files into verifiable workflow steps
- **Canonical Schema**: Standardized document representation with full metadata extraction
- **Workflow Integration**: Add document ingestion as workflow steps with complete provenance tracking
- **Supported Formats**:
  - PDF (via pdf-extract)
  - LaTeX (.tex) with Markdown conversion
  - Plain text (.txt)
  - Microsoft Word (.docx) with Office Open XML parsing

### Native File Dialogs
- **Document Selection**: Browse for documents when creating workflow steps using native OS file pickers
- **Export Locations**: Choose save locations for CAR exports and project archives
- **User-Friendly**: Standard OS file picker integration via Tauri dialog plugin v2

### Enhanced Portability
- **Simplified Exports**: Project exports (.ixp) save directly to user-chosen folders without nested directory structures
- **Backward Compatibility**: Import system handles both new and legacy project formats seamlessly
- **Run Execution Tracking**: Proper foreign key relationships ensure reproducible execution history across imports

## Security

If you believe you’ve found a security or cryptography-related issue in Intelexta,
please email `root@gaugefreedom.com` with details.
Please do not open public GitHub issues for sensitive reports.


## ⚖️ License

This project is licensed under the **GNU Affero General Public License v3.0 (AGPLv3)**. See the `LICENSE` file for details.

For **commercial licensing inquiries** (e.g., to use this IP in a closed-source product), please contact Gauge Freedom, Inc.