# intelexta
**Intelexta — Intelligence with Extra Context.**

Local‑first desktop app that maximizes a large LLM context window (~200k tokens) and persists distilled state (checkpoints) so any project — book, paper, course, or problem‑solving workspace — can be resumed with minimal friction.

## Features (MVP)
- Large‑context chat with explicit **Context Preview**
- Pinned items and priority‑based context assembly
- Checkpoints with summary, decisions, TODOs, keypoints + citations
- Hybrid retrieval (BM25 + vectors) over local SQLite
- Templates: **Book**, **Paper**, **Course**, **Problem Solver**

## Stack
- Desktop: **Tauri** (Rust) + **React (Vite)**
- DB: SQLite + FTS5; vectors via `sqlite-vss` (or LanceDB fallback)
- LLM: provider with ≥200k ctx + function calls; embeddings API

## Quick start
```bash
# Frontend
cd app
npm i
npm run dev

# Backend
cd ../src-tauri
cargo tauri dev
```

## 1) Repository structure

```text
intelexta/
├─ LICENSE
├─ README.md
├─ .gitignore
├─ .editorconfig
├─ CONTRIBUTING.md
├─ CODE_OF_CONDUCT.md
├─ scripts/
│  ├─ dev.sh
│  └─ build-release.sh
├─ app/                      # React (Vite) frontend
│  ├─ index.html
│  ├─ package.json
│  ├─ tsconfig.json
│  ├─ vite.config.ts
│  └─ src/
│     ├─ main.tsx
│     ├─ App.tsx
│     ├─ components/
│     │  ├─ ProjectPanel.tsx
│     │  ├─ ContextPanel.tsx
│     │  ├─ ChatPanel.tsx
│     │  └─ TimelinePanel.tsx
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
      ├─ assemble/mod.rs
      ├─ retrieve/mod.rs
      ├─ distill/mod.rs
      ├─ ingest/mod.rs
      └─ util/tokens.rs
```
