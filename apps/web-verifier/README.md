# Intelexta Web Verifier

A Vite + React + TypeScript frontend for verifying Intelexta workflow proofs in the browser. The
application lazily loads the WebAssembly bindings produced by `wasm-pack`, reads CAR archives or JSON
transcripts dropped by the user, and renders:

* A **Verification** view (cryptographic integrity, signatures, hash chains)
* A **Visualize Content** view (human-friendly overview of workflows, steps, and provenance)

## Prerequisites

* Node.js 20+
* npm 9+
* [`wasm-pack`](https://rustwasm.github.io/wasm-pack/installer/)
* Intelexta verifier WASM artifacts generated via the provided helper scripts.
* (For deployment) [Firebase CLI](https://firebase.google.com/docs/cli) installed and authenticated

## Getting started

```bash
cd apps/web-verifier
npm install
```

During development Vite will serve files from the `public` directory directly. Run `npm run build:wasm`
whenever you change the Rust crate so the latest artifacts are written to `public/pkg`.

### Available scripts

| Script               | Description                                                                            |
| -------------------- | -------------------------------------------------------------------------------------- |
| `npm run dev`        | Start the Vite development server with hot module reloading.                           |
| `npm run build`      | Create a production build in `dist`.                                                   |
| `npm run preview`    | Preview the production build locally.                                                  |
| `npm run build:wasm` | Compile the Rust verifier crate with `wasm-pack` and place the output in `public/pkg`. |
| `npm run typecheck`  | Run TypeScript in no-emit mode to verify the frontend types.                           |

### Building the WASM package

The repository provides a reusable script at `scripts/build-wasm.sh` (from the monorepo root) that targets
the web build of the Rust crate and copies the generated glue code and `.wasm` binaries into `public/pkg`:

```bash
cd apps/web-verifier
npm run build:wasm
```

The script checks for `wasm-pack` and fails fast with a helpful error message when the binary is not
installed.

---

## Local development

### 1. Run the dev server

```bash
cd apps/web-verifier

# Make sure WASM bundle exists at least once
npm run build:wasm

# Start Vite dev server
npm run dev
```

Vite will print a local URL (typically `http://localhost:5173`). Open it in a browser and:

1. Drag a valid `*.car.json` or `*.car.zip` into the dropzone.
2. Wait for verification to complete (success/failure banner at the top).
3. Use the **Verification / Visualize Content** toggle to switch between:

   * Cryptographic verification details
   * Human-friendly workflow, steps, and provenance preview

---

## Deployment: Firebase Hosting

The Web Verifier is designed to be deployed as a static site using **Firebase Hosting** (e.g. at
`https://verify.intelexta.com`).

### One-time Firebase setup

From the monorepo root or from `apps/web-verifier`, ensure you’re logged in:

```bash
firebase login
```

If you haven’t initialized hosting for this project yet (do once):

```bash
cd apps/web-verifier
firebase init hosting
# public directory: dist
# single-page app: yes
# overwrite index.html: no
```

This will create/update a `firebase.json` and `.firebaserc` in this directory.

### Build for production

From `apps/web-verifier`:

```bash
# Build WASM + frontend
npm run build:wasm
npm run build

# Ensure dist/ contains the bundled site
ls dist
```

### Preview locally with Firebase emulator (optional)

```bash
cd apps/web-verifier
firebase serve --only hosting
# or:
firebase emulators:start --only hosting
```

Then open `http://localhost:5000` and exercise the verifier as you would in production.

### Deploy to Firebase Hosting

```bash
cd apps/web-verifier
firebase deploy --only hosting
# or, if you have multiple projects:
firebase deploy --only hosting --project your-project-id
```

After deploy, Firebase will show your Hosting URL, e.g.:

```
Hosting URL: https://your-project-id.web.app
```

Your Web Verifier is now live 🎉

### Hosting configuration

`apps/web-verifier/firebase.json` is configured to:

* Serve the built app from `dist/`
* Treat it as a single-page app (all routes rewrite to `/index.html`)
* Apply long-lived caching headers to static assets:

  * `Cache-Control: public, max-age=31536000, immutable` for `.wasm`, `.js`, `.css`, images, fonts, etc.
* Serve WASM with the correct content type and COEP/COOP headers:

```jsonc
{
  "source": "**/*.wasm",
  "headers": [
    { "key": "Content-Type", "value": "application/wasm" },
    { "key": "Cross-Origin-Embedder-Policy", "value": "require-corp" },
    { "key": "Cross-Origin-Opener-Policy", "value": "same-origin" }
  ]
}
```

These settings ensure the verifier’s WebAssembly module loads correctly and is cached efficiently in
production.

---

## Project structure

```text
apps/web-verifier
├── index.html
├── package.json
├── firebase.json           # Firebase Hosting config for this app
├── public
│   └── pkg                 # wasm-pack output (glue JS + .wasm)
├── src
│   ├── App.tsx
│   ├── components
│   │   ├── MetadataCard.tsx
│   │   ├── Verifier.tsx
│   │   ├── WorkflowTimeline.tsx
│   │   ├── WorkflowOverviewCard.tsx   # high-level run summary
│   │   ├── WorkflowStepsCard.tsx     # step-by-step content view
│   │   └── ContentView.tsx           # "Visualize Content" container
│   ├── types
│   │   └── car.ts                    # CAR v0.3 TypeScript types
│   ├── utils
│   │   └── textHelpers.ts            # truncation & formatting helpers
│   ├── index.css
│   ├── main.tsx
│   └── wasm
│       └── loader.ts                 # dynamic WASM loader
├── tailwind.config.js
├── tsconfig.json
├── tsconfig.node.json
└── vite.config.ts
```

---

## Manual QA

1. Build the WASM bundle so the verifier glue and `.wasm` files exist in `public/pkg`:

   ```bash
   cd apps/web-verifier
   npm run build:wasm
   ```
2. Start the development server:

   ```bash
   npm run dev
   ```
3. Visit the printed local URL and drag a valid `*.car.json` or `*.car.zip` into the dropzone. Confirm the
   loading skeleton appears until verification completes and the success banner references the file name.
4. Toggle to **Visualize Content** and verify that:

   * Workflow overview shows run name, created_at, kind, model, budgets, and S-grade.
   * Budget values display tiny USD amounts as `< $0.01` with a tooltip showing the full value.
   * Nature Cost is displayed in kWh with automatic unit scaling (Wh/mWh) for small values.
   * Steps list shows step type, checkpoint type, model, proof mode, and truncated prompts.
   * Provenance entries (config, input, output) and checkpoint token usage are visible.
5. Drop a file with an unsupported extension (for example `notes.txt`). Verify the dropzone rejects the
   file, shows a red alert with the descriptive error, and no stale results remain.
6. Drop a failing proof (e.g. one with a tampered CAR). Ensure the status banner turns red, the
   alert surfaces the WASM error message, and the raw JSON payload is visible for debugging.

**Note**: `budgets.nature_cost` is treated as kWh when `policy_ref.estimator` is
`intelexta-validator-local-kwh-v1`. We plan to adapt this when `gauge-index` is the
source of cost + kWh estimates for most providers (including local OSS models).

---

## Troubleshooting

* **Missing WASM exports**
  Ensure the expected functions (`init_verifier`, `verify_car_bytes`, `verify_car_json`) are exported by the
  generated JS glue code in `public/pkg/*.js`. Rebuild with:

  ```bash
  npm run build:wasm
  ```

* **CORS / COEP / COOP errors**
  When deploying to Firebase, verify that:

  * `firebase.json` in `apps/web-verifier` is the configuration being used.
  * WASM responses include `Content-Type: application/wasm`.
  * COEP/COOP headers are present as configured.

* **Type errors**
  Re-run dependency installation:

  ```bash
  npm install
  npm run typecheck
  ```

* **Stale WASM bundle**
  If the UI seems to use old verifier behavior:

  ```bash
  npm run build:wasm
  npm run dev
  ```

  and hard-refresh the page (Shift+Reload) or clear cache.

  ## License

This component is part of the Intelexta Protocol and is licensed under the **GNU Affero General Public License v3.0 (AGPLv3)**.

* **See Root License:** [../../LICENSE](../../LICENSE)
* **Commercial Use:** For proprietary use cases (embedding without open-sourcing your code), commercial licenses are available. Contact `root@gaugefreedom.com`.
