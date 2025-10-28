# Testing guide for `intelexta-wasm-verify`

## Rust unit tests

```bash
cargo test -p intelexta-wasm-verify
```

These tests execute the verification pipeline against the fixtures stored in
`tests/fixtures`. They assert that both the standalone `.car.json` and the
`.car.zip` archive validate successfully and that the generated report matches
the expectations used by the browser UI.

## WebAssembly build smoke test

```bash
wasm-pack build apps/web-verifier/wasm-verify \
  --target web \
  --out-dir public/pkg \
  --release
```

This command produces a production-ready WASM bundle. The output directory can
be served locally with `npm run dev` from `apps/web-verifier` to exercise the
end-to-end integration.
