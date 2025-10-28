# IntelexTA Web Verifier

A Vite + React + TypeScript frontend for verifying IntelexTA workflow proofs in the browser. The
application lazily loads the WebAssembly bindings produced by `wasm-pack`, reads CAR archives or JSON
transcripts dropped by the user, and renders a friendly timeline and metadata viewer.

## Prerequisites

- Node.js 20+
- npm 9+
- IntelexTA verifier WASM artifacts generated via `wasm-pack build`.

## Getting started

```bash
cd apps/web-verifier
npm install
```

During development Vite will serve files from the `public` directory directly. Make sure the
`wasm-pack` output has been copied to `public/pkg` (for example `public/pkg/web_verifier.js` and
`public/pkg/web_verifier_bg.wasm`).

### Available scripts

| Script        | Description |
| ------------- | ----------- |
| `npm run dev` | Start the Vite development server with hot module reloading. |
| `npm run build` | Create a production build in `dist`. |
| `npm run preview` | Preview the production build locally. |
| `npm run build:wasm` | Build the app and copy the WASM artifacts into `dist/pkg` for deployment. |

### Building the WASM package

1. Build the verifier crate using `wasm-pack`. For example:
   ```bash
   wasm-pack build --target web --out-dir pkg
   ```
2. Copy the resulting `pkg` directory into `apps/web-verifier/public/pkg`:
   ```bash
   cp -r pkg apps/web-verifier/public/
   ```

### Deployment notes

- When deploying, run `npm run build:wasm` to ensure the WebAssembly files are bundled into the
  `dist/pkg` directory alongside the Vite output.
- If hosting behind a CDN make sure `application/wasm` is served with the correct MIME type.
- The app is static and can be deployed to any static host (e.g. Cloudflare Pages, Vercel, GitHub
  Pages) as long as the `/pkg` directory is present.

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
