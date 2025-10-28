# Intelexta Web Verifier (WASM)

This crate exposes the Intelexta CAR verification pipeline as a WebAssembly module that can be
consumed by the `apps/web-verifier` frontend. It reuses the core verification logic from the
CLI tool while operating entirely in-memory, making it safe to run inside the browser.

## Building

Install [`wasm-pack`](https://rustwasm.github.io/wasm-pack/installer/) and run:

```bash
wasm-pack build apps/web-verifier/wasm-verify \
  --target web \
  --out-dir public/pkg \
  --release
```

This command emits the generated JavaScript bindings and `.wasm` binary to
`apps/web-verifier/public/pkg`. The Vite dev server serves assets from this directory at runtime.

## Exposed API

The crate exports two entry points that return structured verification reports via
`serde_wasm_bindgen`:

- `verify_car_bytes(bytes: &[u8])` – detects `.car.json` vs `.car.zip`, verifies proofs, and returns
  a `JsValue` that can be deserialized in TypeScript.
- `verify_car_json(json: &str)` – optimized path when the frontend already has the JSON contents.

Both functions emit rich error information through `JsError` when validation fails.

## Testing

The core logic is covered by integration-style tests that load fixture data with `include_bytes!`.
Run them with:

```bash
cargo test -p intelexta-wasm-verify
```

## Packaging Notes

- The crate is compiled as both an `rlib` and a `cdylib` so it can be unit-tested natively while
  still producing WebAssembly binaries via `wasm-pack`.
- All verification work happens in memory. CAR archives are decompressed with `zip` using an
  in-memory cursor, so no filesystem access is required inside the browser sandbox.
