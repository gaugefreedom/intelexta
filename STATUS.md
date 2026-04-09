# STATUS — 2026-04-08

## Overall: Phase 1 Complete · Phase 2 Partial

---

## Component Status

### `apps/web-verifier` ✅ Live · Recently Active
- Deployed at [verify.intelexta.com](https://verify.intelexta.com) via Firebase Hosting
- **Just completed**: PT-BR i18n (i18next, ~100 keys, `?lang=` URL param, localStorage persistence)
- Light theme UI, Content Visualizer (WorkflowOverviewCard, WorkflowStepsCard, AttachmentsCard)
- Public Receipt page (`/r/:receiptId`) pulls from validator API + WASM verifies
- CAR v0.3 compatible; verifies `.car.json` and `.car.zip`
- **Dev**: `npm run build:wasm && npm run dev` → localhost:5173
- **Deploy**: `npm run build:wasm && npm run build && firebase deploy --only hosting`

### `apps/verifiable-summary` ✅ Live · Recently Refactored
- MCP server integrated with ChatGPT via OpenAI Apps SDK
- Generates CAR-Lite bundles (signed, schema-compliant with CAR v0.3)
- Signed download URLs with HMAC (15-min TTL), persistent bundle storage
- Security hardened (URL validation, signed URLs removed in favour of expiring tokens)
- Recent: summarizer.ts + provenance.ts + storage.ts refactored; test coverage updated

### `src-tauri` (Desktop Node) ✅ Working · Stable
- Full workflow execution: orchestrator → provenance → CAR export
- Document processing: PDF, LaTeX, TXT, DOCX ingestion
- `replay.rs` exists — Phase 2 scaffold present, completion unknown
- Governance, model catalog, S-grade scoring all operational
- Model catalog: `config/model_catalog.toml` (Ollama, OpenAI, Anthropic, internal)

### `src-tauri/crates/intelexta-verify` ✅ Working
- CLI verifier: hash chain, signatures, config + attachment integrity
- WASM-compiled into `apps/web-verifier/public/pkg/`
- Phase 2 `--replay` flag: not yet implemented

### `schemas/`
- **v0.3**: Canonical, active across all tools
- **v0.4**: Draft schema exists (`car-v0.4.schema.json`) — adds receipt lineage (parent/root), optional Reliability Score, extensions bag (Edu-Node / Patent-Node), preset/node context. **Not yet adopted** by any component.

---

## Active Uncommitted Changes (working tree)
- `apps/web-verifier/` — i18n implementation (this session), docs
- `apps/verifiable-summary/` — refactor + new docs
- `schemas/car-v0.4.schema.json` — draft, untracked
- `config/model_catalog.toml` — minor updates
- `docs/PROOF_MODE_STRATEGY.md` — updated

---

## Known Blockers / Risks
- CAR v0.4 adoption: schema drafted but no component generates/validates it yet
- Phase 2 (`replay.rs`): file exists but extent of implementation unknown — needs audit
- Desktop Node build not tested recently; Wayland workaround required on Linux (see CONTRIBUTING.md)
- `intelexta-verify` CLI: `--replay` flag missing, Phase 2 not wired to CLI
