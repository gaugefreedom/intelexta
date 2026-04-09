# Intelexta

**Local-first control plane for verifiable Human+AI workflows.**
_"Proof, not vibes." · "Exact where possible, accountable where not."_

## What it is

Git + Notary for AI workflows. Cryptographically signed, reproducible, auditable.
Not a chat interface — a high-integrity protocol and toolchain.

## Monorepo Map

| Path | What | Status |
|------|------|--------|
| `src-tauri/` | Desktop Node (Rust/Tauri) — local workflow execution, signing, CAR export | ✅ Working |
| `src-tauri/crates/intelexta-verify/` | CLI verifier — trustless CAR verification | ✅ Working |
| `apps/web-verifier/` | Web Verifier (React/Vite/WASM) → [verify.intelexta.com](https://verify.intelexta.com) | ✅ Live |
| `apps/verifiable-summary/` | MCP server (TypeScript) — ChatGPT integration, CAR-Lite proofs | ✅ Live |
| `schemas/` | CAR JSON schemas (v0.2 legacy, v0.3 canonical, v0.4 draft) | v0.3 active |
| `config/model_catalog.toml` | Model registry with pricing + energy metadata | ✅ Active |

## Protocol Core

- **CAR** (Content-Addressable Receipt): the portable proof artifact
- **Signing**: Ed25519 + SHA-256 hash chains + JCS canonicalization
- **Profiles**: CAR-Full (Desktop, rich metrics) · CAR-Lite (community plugins, minimal)
- **Proof modes**: `exact` (byte-match) · `concordant` (semantic, graded A+→F)

## Key External Apps (separate repos)

- **Intelexta Validator** → [validator.intelexta.com](https://validator.intelexta.com) — hosted validator, predates Desktop Node

## Stack

- Desktop: Tauri (Rust) + React (Vite) + SQLite
- Web tools: Vite + React + WASM (compiled from Rust verifier crate)
- MCP server: TypeScript/Express + OpenAI Apps SDK
- Crypto: `ed25519-dalek`, `sha2`, `serde_json` (JCS)

## Phases

1. **Cryptographic Integrity** ✅ — sign, hash-chain, verify, export CAR
2. **Graded Replay** 🔄 — re-execute and score output similarity (`replay.rs` exists)
3. **Visualization** 📋 — batch verification, diff views, S-grade dashboards
4. **Advanced Governance** 📋 — policy-as-code, dynamic model routing, RBAC
5. **Ecosystem** 📋 — CI/CD actions, IPFS, public CAR registry
