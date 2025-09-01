# Intelexta

**Local‑first, auditable control plane for research‑grade AI work. Reproducible pipelines. Signed provenance. Policy‑governed cost and carbon.**

Intelexta is a desktop application for researchers, writers, and developers who need to produce verifiable and reproducible results from AI workflows. It is not another chat interface; it is a high-integrity control plane built on three core principles:

1.  **Signed Provenance:** Every output is cryptographically linked to its exact inputs, models, and prompts via hash-chained checkpoints.
2.  **Reproducible Pipelines:** Workflows are captured as deterministic execution graphs (DAGs) that can be replayed to verify results.
3.  **Local-First Governance:** Your data and cryptographic keys live on your machine. You set per-project policies for cost, emissions, and data egress.

## Core Features (MVP)

  - **Signed Checkpoints:** Cryptographically sign and chain every step of a workflow, creating an immutable audit trail.
  - **Deterministic Replay:** Re-execute any workflow from any checkpoint to validate its output.
  - **Local-First Governance:** Enforce per-project budgets for cost ($), tokens, and carbon emissions (gCO₂e).
  - **Full Export:** Package and export an entire project—including data, policies, and the full provenance chain—in an open format.

## Stack

  - **Desktop:** Tauri (Rust) + React (Vite)
  - **Database:** Local SQLite
  - **Provenance:** Ed25519 Signatures + SHA-256 Hash Chains

## Quick Start

```bash
# 1. Run the Frontend
cd app
npm install
npm run dev

# 2. In a separate terminal, run the Backend
cd src-tauri
cargo tauri dev
```

## Repository Structure (Post-Refactor)

```text
intelexta/
├─ scripts/
│  ├─ dev.sh
│  └─ build-release.sh
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

## Status

Actively developing the MVP as per `Strategic Spec v0.1`. The current codebase is being refactored to align with this new mission.
