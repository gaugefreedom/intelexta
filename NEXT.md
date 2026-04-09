# NEXT — Short-Term Roadmap

_Ordered by priority. Prune completed items on each agent turn._

---

## Immediate (commit / ship)

- [ ] **Commit web-verifier i18n** — PT-BR localization is working, tests pass, uncommitted
- [ ] **Commit verifiable-summary refactor** — index.ts + provenance.ts + storage.ts changes uncommitted
- [ ] **Commit CAR v0.4 schema draft** — untracked file, should be versioned even as draft

---

## Short-Term (next sessions)

### web-verifier
- [ ] **i18n: proofFiles.ts error messages** — still hardcoded English, excluded from this pass
- [ ] **i18n: WASM-originated error strings** — come from Rust, need separate strategy (map known codes)

### verifiable-summary
- [ ] **CAR v0.4 upgrade** — adopt new schema fields (reliability score, extensions) once v0.4 stabilises
- [ ] **Widget production hardening** — review iframe Skybridge UX, handle expired download links gracefully in UI

### schemas
- [ ] **Ratify CAR v0.4** — finalize + document migration path from v0.3; update `schemas/README.md`
- [ ] **Validator for CAR v0.4** — update `schemas/validate-car.js` to support v0.4

### Desktop Node / CLI
- [ ] **Audit `replay.rs`** — determine what Phase 2 is actually implemented vs scaffolded
- [ ] **Wire `--replay` flag to CLI** — `intelexta-verify proof.car.zip --replay --api-keys-from-env`

---

## Medium-Term (Phase 2 completion)

- [ ] `src-tauri/crates/intelexta-verify/src/similarity.rs` — semantic similarity scoring
- [ ] `src-tauri/crates/intelexta-verify/src/grading.rs` — A+→F grade output
- [ ] Graded report JSON output from CLI
- [ ] Web Verifier: display replay/grade results when available in CAR

---

## Backlog (Phase 3+)
- Batch verification dashboard
- CAR diff visualizer
- GitHub Actions `intelexta/verify-action@v1`
- IPFS / public CAR registry
- RBAC + policy-as-code
