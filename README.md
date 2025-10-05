# Intelexta

**A local-first control plane for verifiable Human+AI workflows.**

_Mottos: "Proof, not vibes." & "Exact where possible, accountable where not."_

---

Intelexta is a desktop application for researchers, writers, students and developers who need to produce verifiable and reproducible results from AI workflows. It is not another chat interface; it is a high-integrity control plane built on the principles of **Signed Provenance**, **Reproducible Pipelines**, and **Local-First Governance**.

## Mission Alignment

-   **Human+AI Symbiosis:** The human is the strategic director; the AI is a powerful cognitive partner operating within a provable envelope of human-defined policy.
-   **Integrity of Knowledge:** Creates a permanent, tamper-evident record of how information is generated, preserving the context and integrity of knowledge.
-   **Accountable Efficiency:** Makes energy and carbon consumption a first-class, manageable metric in every workflow.

## Stack

-   **Desktop:** Tauri (Rust) + React (Vite)
-   **Database:** Local SQLite
-   **Provenance:** Ed25519 Signatures + SHA-256 Hash Chains

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

## Repository Structure (Post-Refactor)

```text
intelexta/
├─ scripts/
│  ├─ dev.sh
│  └─ build-release.sh
├─ schemas/
│  └─ car-v0.2.schema.json   # Locked JSON Schema for Content-Addressable Receipts
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

The canonical JSON Schema for Content-Addressable Receipts (CAR v0.2) lives in [`schemas/car-v0.2.schema.json`](schemas/car-v0.2.schema.json).
Builds verify that the file exists, so keep it in place when updating tooling or CI.

## Status

Actively developing the MVP as per `Strategic Spec v0.1`. The current codebase is being refactored to align with this new mission.

## Recent Features

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
