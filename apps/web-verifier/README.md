# IntelexTA Web Verifier

A Vite + React + TypeScript frontend for verifying IntelexTA workflow proofs in the browser. The
application lazily loads the WebAssembly bindings produced by `wasm-pack`, reads CAR archives or JSON
transcripts dropped by the user, and renders a friendly timeline and metadata viewer.

## Prerequisites

- Node.js 20+
- npm 9+
- [`wasm-pack`](https://rustwasm.github.io/wasm-pack/installer/)
- IntelexTA verifier WASM artifacts generated via the provided helper scripts.

## Getting started

```bash
cd apps/web-verifier
npm install
```

During development Vite will serve files from the `public` directory directly. Run `npm run build:wasm`
whenever you change the Rust crate so the latest artifacts are written to `public/pkg`.

### Available scripts

| Script | Description |
| ------ | ----------- |
| `npm run dev` | Start the Vite development server with hot module reloading. |
| `npm run build` | Create a production build in `dist`. |
| `npm run preview` | Preview the production build locally. |
| `npm run build:wasm` | Compile the Rust verifier crate with `wasm-pack` and place the output in `public/pkg`. |
| `npm run typecheck` | Run TypeScript in no-emit mode to verify the frontend types. |

### Building the WASM package

The repository provides a reusable script at `scripts/build-wasm.sh` that targets the web build of the
Rust crate and copies the generated glue code and `.wasm` binaries into `public/pkg`:

```bash
npm run build:wasm
```

The script checks for `wasm-pack` and fails fast with a helpful error message when the binary is not
installed.

### Deployment workflow

1. Ensure you are in the project root (`/workspace/intelexta`).
2. Build the WebAssembly package and frontend bundle:
   ```bash
   cd apps/web-verifier
   npm run build:wasm && npm run build
   ```
3. Upload the `apps/web-verifier/dist/` directory to your static host (e.g. DreamHost). The build copies
   the `pkg` directory so the WebAssembly files and JS glue are served alongside the compiled assets.

For CI/CD or automated deployments, run the same pair of commands prior to publishing the contents of
`dist/`.

### Project structure

```
apps/web-verifier
├── index.html
├── package.json
├── postcss.config.js
├── public
│   └── pkg
├── src
│   ├── App.tsx
│   ├── components
│   │   ├── MetadataCard.tsx
│   │   ├── Verifier.tsx
│   │   └── WorkflowTimeline.tsx
│   ├── index.css
│   ├── main.tsx
│   └── wasm
│       └── loader.ts
├── tailwind.config.js
├── tsconfig.json
├── tsconfig.node.json
└── vite.config.ts
```

## Troubleshooting

- **Missing WASM exports:** Ensure the expected functions (`init_verifier`, `verify_car_bytes`,
  `verify_car_json`) are exported by the generated JS glue code.
- **CORS errors:** When developing with a remote backend double-check the `public/pkg` path is
  accessible from the dev server origin.
- **Type errors:** Re-run `npm install` to make sure TypeScript types for dependencies are present.
- **Stale WASM bundle:** Rebuild the WebAssembly artifacts with `npm run build:wasm` and restart the dev
  server.
