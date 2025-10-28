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

| Script    | Description |
| --------- | ----------- |
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

### Deployment target: Netlify

The project is configured to deploy to **Netlify**, which offers automated builds, SSL, and a global CDN.
The generated `apps/web-verifier/netlify.toml` file encapsulates the build command and caching policy for
the WebAssembly bundle.

#### Build command

Netlify (and the provided CI workflow) execute the combined build in one step so the generated `pkg/`
artifacts are copied into the final `dist/` output:

```bash
npm run build:wasm && npm run build
```

If you are building locally, run the command from `apps/web-verifier/` after installing dependencies with
`npm ci` or `npm install`.

#### Manual deployment via Netlify CLI

1. Install the Netlify CLI (`npm install -g netlify-cli`) and authenticate with `netlify login`.
2. Build the site:

   ```bash
   cd apps/web-verifier
   npm ci
   npm run build:wasm && npm run build
   ```
3. Deploy a preview build to confirm everything looks correct:

   ```bash
   netlify deploy --dir=dist --message "Preview build" 
   ```
4. Promote the same artifact to production when ready:

   ```bash
   netlify deploy --dir=dist --prod --message "Production release"
   ```

#### DNS and domain configuration

1. In the Netlify dashboard create or select the site and note the generated `*.netlify.app` hostname.
2. To use a custom domain, add it under **Site settings → Domain management** and follow Netlify's
   verification steps.
3. Update your DNS provider:
   - For a root (apex) domain, create `A` records pointing to Netlify's load balancer IP addresses
     (`75.2.60.5` and `99.83.190.102`).
   - For subdomains, create a `CNAME` record that targets the Netlify-provided hostname.
4. Once DNS propagates, issue a production deploy so Netlify can provision HTTPS certificates via
   Let's Encrypt automatically.

#### Caching headers for WASM

`netlify.toml` configures long-term caching for the WebAssembly artifacts produced by `wasm-pack`:

```toml
[[headers]]
  for = "/pkg/*.wasm"
  [headers.values]
    Cache-Control = "public, max-age=31536000, immutable"
    Content-Type = "application/wasm"

[[headers]]
  for = "/pkg/*.js"
  [headers.values]
    Cache-Control = "public, max-age=31536000, immutable"
```

Because the filenames are content-hashed by `wasm-pack`, these caching directives ensure clients reuse the
same binary across sessions while allowing instant rollbacks when a new hash is published.

#### Continuous deployment

- `.github/workflows/web-verifier.yml` orchestrates CI/CD. It builds the WASM package, bundles the frontend,
  and uploads the production assets as an artifact on every pull request or push affecting the web verifier.
- When commits land on `main`, the workflow downloads the artifact and publishes it with
  `netlify/actions/cli@master`. Provide the following repository secrets to enable automatic deploys:
  - `NETLIFY_AUTH_TOKEN`: Personal access token generated in the Netlify user settings.
  - `NETLIFY_SITE_ID`: The UUID of the Netlify site (found in Site settings → General → Site details).

#### Production verification checklist

1. After a deploy, visit the live site (either the Netlify preview URL or your custom domain).
2. Open the browser developer tools **Network** tab and refresh the page.
3. Confirm a request to `/pkg/*_bg.wasm` completes with HTTP 200 and the response header `content-type:
   application/wasm`.
4. Upload a known-good transcript and ensure the UI shows a green success banner once verification
   finishes.
5. Optionally, use the Netlify deploy details page to inspect build logs and verify the `npm run
   build:wasm && npm run build` command succeeded.

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

## Manual QA

1. Build the WASM bundle so the verifier glue and `.wasm` files exist in `public/pkg`:
   ```bash
   npm run build:wasm
   ```
2. Start the development server:
   ```bash
   npm run dev
   ```
3. Visit the printed local URL and drag a valid `*.car.json` transcript into the dropzone. Confirm the
   loading skeleton appears until verification completes and the success banner references the file name.
4. Drop a file with an unsupported extension (for example `notes.txt`). Verify the dropzone rejects the
   file, shows a red alert with the descriptive error, and no stale results remain.
5. Drop a failing proof (e.g. one with an invalid signature). Ensure the status banner turns red, the
   alert surfaces the WASM error message, and the raw JSON payload is visible for debugging.

## Troubleshooting

- **Missing WASM exports:** Ensure the expected functions (`init_verifier`, `verify_car_bytes`,
  `verify_car_json`) are exported by the generated JS glue code.
- **CORS errors:** When developing with a remote backend double-check the `public/pkg` path is
  accessible from the dev server origin.
- **Type errors:** Re-run `npm install` to make sure TypeScript types for dependencies are present.
- **Stale WASM bundle:** Rebuild the WebAssembly artifacts with `npm run build:wasm` and restart the dev
  server.
