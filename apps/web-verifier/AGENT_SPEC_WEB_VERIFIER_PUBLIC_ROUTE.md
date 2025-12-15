# AGENT SPEC — Web Verifier Public Receipt Route

Owner: Marcelo
Scope: apps/web-verifier
Assumption: Repo already uses IXIR + Report naming in UI and code.

Goal:
Implement a zero-login “Auditor View” route:
  /r/:receiptId

This route fetches a sanitized public snapshot from Validator:
  GET {VALIDATOR_API_BASE}/public/r/{receiptId}

Then:
1) Runs WASM verification on the CAR payload.
2) Renders a human-friendly auditor view of the IXIR summary.

This is the landing page linked from:
- IXIR UI watermark
- IXIR PDF watermark
- Shared proof links

---

## Environment

Add:
- `VITE_VALIDATOR_API_BASE`
Default:
- `https://validator.intelexta.com/api`

---

## Route & page

### 1) Add React Router route
- `/r/:receiptId`

### 2) New page component
Create:
- `src/pages/PublicReceiptPage.tsx`

Behavior:
- Read param `receiptId`
- Fetch:
  `${VITE_VALIDATOR_API_BASE}/public/r/${receiptId}`

States:
- Loading
- Not found (404)
- Error (network)

---

## Verification flow

### Inputs
The public endpoint returns:
- `report` (sanitized)
- `receipt` (sanitized CAR)
- `meta`

The page should verify the CAR using existing WASM logic.

### Required behavior
- If WASM verification succeeds:
  - show green “Receipt verified”
- If it fails:
  - show red “Receipt mismatch or tamper detected”
  - still show sanitized metadata for transparency

---

## UI requirements

PublicReceiptPage layout:

1) Header
- “Intelexta Integrity Report”
- small subtitle:
  - “Auditor view — no login required”

2) Verification status banner
- success / failure

3) Metadata card
- receipt id
- run id (if present)
- tier
- engine name
- privacy mode
- created at

4) IXIR summary cards
Render what you have available in your existing “Visualize Content” UI components:
- Factual reliability score + explanation
- Novelty score + interpretation
- Key claims list (statement + support label)
- Fragile sections list
- Risks & suggested improvements (if included in public snapshot)

5) CTA footer
Two buttons:
- “Verify your own work”
  -> https://validator.intelexta.com
- “Learn about the protocol”
  -> https://intelexta.com

---

## Non-goals
- Do NOT accept file drops in this route as the primary flow
  (dropzone stays in the main verifier home).
- Do NOT require auth.
- Do NOT show raw artifact text.

---

## Acceptance criteria

- Visiting `/r/{id}`:
  - fetches public snapshot
  - verifies CAR via WASM
  - renders auditor-friendly IXIR content
- Works for:
  - anonymous community runs
  - authenticated runs that were explicitly shared
- 404 when public snapshot does not exist
- No raw user text ever rendered.

---

## Files likely touched

- `apps/web-verifier/src/App.tsx` (router)
- `apps/web-verifier/src/pages/PublicReceiptPage.tsx` (new)
- Reuse existing:
  - `components/Verifier.tsx` verification logic
  - `components/ContentView.tsx` or overview cards (as applicable)
- `apps/web-verifier/src/wasm/loader.ts` (if needed)

Prefer minimal new UI components by reusing existing Overview/Steps/Metadata cards. Only create new components when necessary.