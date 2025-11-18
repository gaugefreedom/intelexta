# CAR-Lite Implementation Plan

## Overview

Implement **CAR-Lite** profile for the verifiable-summary MCP server to enable easy adoption by community plugins while maintaining 100% compliance with Intelexta's CAR v0.2 schema.

## Goals

1. ✅ **Schema Compliance**: Generate car.json that validates against `schemas/car-v0.2.schema.json`
2. ✅ **Verifier Compatible**: Works with both `apps/web-verifier` and `target/release/intelexta-verify`
3. ✅ **Minimal Barrier**: Simple integration for OSS plugins (Ollama, LangChain, LlamaIndex, etc.)
4. ✅ **Deterministic IDs**: Proper JCS canonicalization for `car:` ID computation
5. ✅ **Optional Signing**: Support both signed and unsigned workflows

## CAR v0.2 Schema Requirements

Based on `schemas/car-v0.2.schema.json`, ALL of these fields are **required**:

### Top-Level Required Fields

```typescript
{
  id: string,                    // "car:<64-hex>" - computed from JCS canonical body
  run_id: string,                // Primary run identifier
  created_at: string,            // ISO 8601 date-time
  run: RunObject,                // Workflow definition
  proof: ProofObject,            // Verification strategy
  policy_ref: PolicyRefObject,   // Policy reference
  budgets: BudgetsObject,        // Resource consumption
  provenance: ProvenanceClaim[], // Asset anchors (minItems: 1)
  checkpoints: string[],         // Checkpoint IDs (minItems: 1, pattern: "ckpt:*")
  sgrade: SgradeObject,          // Stewardship grade
  signer_public_key: string,     // Base64 public key
  signatures: string[]           // Detached signatures (minItems: 1, pattern: "<alg>:<sig>")
}
```

### run Object (Required)

```typescript
{
  kind: "exact" | "concordant" | "interactive",
  name: string,           // Human-readable name
  model: string,          // Model identifier
  version: string,        // Version/revision
  seed: number,           // Integer >= 0
  steps: RunStep[],       // Array of steps
  sampler?: SamplerObject // Optional
}
```

### run.steps[*] Object (Required per step)

```typescript
{
  id: string,
  run_id: string,
  order_index: number,
  checkpoint_type: string,
  token_budget: number,        // >= 0
  proof_mode: "exact" | "concordant",

  // Optional but recommended:
  step_type?: string,          // Default: "llm"
  model?: string | null,
  prompt?: string | null,
  epsilon?: number,
  config_json?: string | null
}
```

### proof Object (Required)

```typescript
{
  match_kind: "exact" | "semantic" | "process"

  // If match_kind === "semantic", also required:
  // epsilon: number,
  // distance_metric: string,
  // original_semantic_digest: string
}
```

### policy_ref Object (Required)

```typescript
{
  hash: string,        // Pattern: "<alg>:<hash>"
  egress: boolean,
  estimator: string    // Formula/description
}
```

### budgets Object (Required)

```typescript
{
  usd: number,         // >= 0
  tokens: number,      // Integer >= 0
  nature_cost: number  // >= 0
}
```

### provenance Array (Required, minItems: 1)

```typescript
[
  {
    claim_type: string,    // e.g., "config", "input", "output"
    sha256: string         // Pattern: "sha256:<64-hex>"
  }
]
```

### checkpoints Array (Required, minItems: 1)

```typescript
["ckpt:<identifier>", ...]  // Pattern: "ckpt:[A-Za-z0-9:_-]+"
```

### sgrade Object (Required)

```typescript
{
  score: number,        // 0-100
  components: {
    provenance: number,  // 0-1
    energy: number,      // 0-1
    replay: number,      // 0-1
    consent: number,     // 0-1
    incidents: number    // 0-1
  }
}
```

### signer_public_key (Required)

```typescript
string  // Base64 encoded, pattern: "^[A-Za-z0-9+/]+={0,2}$"
```

### signatures Array (Required, minItems: 1)

```typescript
["<algorithm>:<base64-signature>", ...]  // Pattern: "^[a-z0-9_-]+:[A-Za-z0-9+/=._-]+$"
```

## CAR-Lite Profile Mapping

### Minimal Values for Unknown Data

When integrating plugins don't have full Intelexta capabilities, use these **neutral defaults**:

| Field | CAR-Lite Default | Notes |
|-------|------------------|-------|
| `proof.match_kind` | `"process"` | Minimal verification strategy |
| `budgets.usd` | `0` | Set to 0 if unknown |
| `budgets.tokens` | `0` | Set to 0 or estimated token count |
| `budgets.nature_cost` | `0` | Set to 0 if unknown |
| `policy_ref.hash` | `"sha256:<hash-of-static-policy>"` | Ship a static policy doc |
| `policy_ref.egress` | `true` | Conservative default for network access |
| `policy_ref.estimator` | `"usage_tokens * 0.010000 nature_cost/token"` | Generic formula |
| `sgrade.score` | `70-97` | Baseline score for basic compliance |
| `sgrade.components` | `{ provenance: 1.0, energy: 1.0, replay: 1.0, consent: 0.8, incidents: 1.0 }` | Neutral values |
| `run.kind` | `"concordant"` | Standard proof mode |
| `run.seed` | `0` or random | Use 0 if determinism not available |
| `run.steps[*].step_type` | `"prompt"`, `"ingest"`, `"summarize"` | Based on operation |
| `run.steps[*].epsilon` | `0.5` | Default tolerance |
| `checkpoints` | `["ckpt:<run_id>"]` | At least one synthetic checkpoint |

### ID Computation (Deterministic)

1. Build the full car.json **without** `id` and `signatures` fields
2. Canonicalize using **JCS** (RFC 8785)
3. Compute SHA-256 of canonical bytes
4. Set `id = "car:" + hex(sha256)`
5. Sign the canonical body (without signatures)
6. Add `signatures` array with `"ed25519:<base64-sig>"`

### Signing Strategy

**Signed (recommended for production)**:
- Generate Ed25519 keypair: `npm run keygen`
- Include `signer_public_key` with base64 public key
- Sign canonical body and add to `signatures` array
- Pattern: `["ed25519:<base64-signature>"]`

**Unsigned (dev/testing only)**:
- Set `signer_public_key` to empty string `""`
- Set `signatures` to `["unsigned:"]` or `["none:"]`
- Add note in README/docs that bundle is unsigned

## Implementation Steps

### 1. Install JCS Library

```bash
cd apps/verifiable-summary/server
npm install json-canonicalize
```

### 2. Update `provenance.ts`

Create helper functions:

```typescript
// JCS canonicalization
import canonicalize from 'json-canonicalize';

export function computeCarId(carBodyWithoutIdAndSigs: any): string {
  const canonical = canonicalize(carBodyWithoutIdAndSigs);
  const hash = sha256(canonical);
  return `car:${hash}`;
}

export function signCanonicalBody(carBodyWithoutSigs: any, secretKeyB64: string): string {
  const canonical = canonicalize(carBodyWithoutSigs);
  return signEd25519(canonical, secretKeyB64);
}
```

### 3. Generate CAR-Lite Compliant car.json

```typescript
export async function generateProofBundle(
  source: Source,
  summary: string,
  model: string,
  secretKeyB64?: string
): Promise<ProofBundleResult> {
  const runId = `vs-${Date.now()}-${Math.random().toString(36).slice(2, 11)}`;
  const createdAt = new Date().toISOString();

  // 1. Compute hashes for attachments
  const summaryContent = `# Verifiable Summary\n\n${summary}\n`;
  const summaryHash = `sha256:${sha256(summaryContent)}`;

  const sourcesContent = `Source: ${source.url}\n\n${source.content}\n`;
  const sourcesHash = `sha256:${sha256(sourcesContent)}`;

  // 2. Build policy
  const policyDoc = `Verifiable Summary Policy v1.0\nAllows: summarization, content ingestion\nEgress: permitted\n`;
  const policyHash = `sha256:${sha256(policyDoc)}`;

  // 3. Build checkpoint ID
  const checkpointId = `ckpt:${runId}`;

  // 4. Build car.json body (without id and signatures)
  const carBody = {
    run_id: runId,
    created_at: createdAt,
    run: {
      kind: "concordant",
      name: "verifiable summary",
      model: `workflow:${model}`,
      version: sha256(model + createdAt),
      seed: Math.floor(Math.random() * 100000000),
      steps: [
        {
          id: runId,
          run_id: runId,
          order_index: 0,
          checkpoint_type: "Summary",
          step_type: "summarize",
          model: model,
          prompt: `Summarize content from: ${source.url}`,
          token_budget: 4000,
          proof_mode: "concordant",
          epsilon: 0.5,
          config_json: JSON.stringify({
            source_url: source.url,
            style: "concise"
          })
        }
      ]
    },
    proof: {
      match_kind: "process"
    },
    policy_ref: {
      hash: policyHash,
      egress: true,
      estimator: "usage_tokens * 0.010000 nature_cost/token"
    },
    budgets: {
      usd: 0,
      tokens: 0,
      nature_cost: 0
    },
    provenance: [
      { claim_type: "config", sha256: `sha256:${sha256(policyDoc)}` },
      { claim_type: "input", sha256: sourcesHash },
      { claim_type: "output", sha256: summaryHash }
    ],
    checkpoints: [checkpointId],
    sgrade: {
      score: 85,
      components: {
        provenance: 1.0,
        energy: 1.0,
        replay: 0.8,
        consent: 0.8,
        incidents: 1.0
      }
    },
    signer_public_key: secretKeyB64 ? getPublicKey(secretKeyB64) : "",
  };

  // 5. Compute ID
  const carId = computeCarId(carBody);

  // 6. Sign if key provided
  let signatures: string[];
  if (secretKeyB64) {
    const signature = signCanonicalBody({ id: carId, ...carBody }, secretKeyB64);
    signatures = [`ed25519:${signature}`];
  } else {
    signatures = ["unsigned:"];
  }

  // 7. Build final car.json
  const carJson = {
    id: carId,
    ...carBody,
    signatures
  };

  // 8. Return bundle
  const bundle: ProofBundle = {
    'car.json': JSON.stringify(carJson, null, 2),
    [`attachments/${summaryHash.replace('sha256:', '')}.txt`]: summaryContent,
    [`attachments/${sourcesHash.replace('sha256:', '')}.txt`]: sourcesContent
  };

  return {
    bundle,
    isSigned: Boolean(secretKeyB64)
  };
}
```

### 4. Update ZIP Filename

In `index.ts`, change download filename:

```typescript
res.setHeader('Content-Disposition', 'attachment; filename="verifiable.car.zip"');
```

### 5. Testing Checklist

- [ ] Generate unsigned bundle → verify with `intelexta-verify`
- [ ] Generate signed bundle → verify with `intelexta-verify`
- [ ] Upload to web-verifier → should show all metadata
- [ ] Validate against `schemas/car-v0.2.schema.json` using ajv or similar
- [ ] Inspect `car.json` manually for schema compliance

### 6. Documentation Updates

Update `CAR_FORMAT.md` to include:
- CAR-Lite vs CAR-Full comparison table
- Field-by-field mapping guidance
- Example minimal car.json
- Integration guide for plugin authors

## Testing Commands

```bash
# 1. Build and start server
cd apps/verifiable-summary/server
npm run build
npm run dev

# 2. Generate a bundle via ChatGPT → download verifiable.car.zip

# 3. Verify with CLI
cd /home/marcelo/Documents/codes/gaugefreedom/intelexta
./src-tauri/target/release/intelexta-verify ~/Documents/teste/verifiable.car.zip

# 4. Verify with web UI
cd apps/web-verifier
npm run dev
# Open http://localhost:5173 and drop the ZIP

# 5. Schema validation (optional)
npm install -g ajv-cli
ajv validate -s schemas/car-v0.2.schema.json -d verifiable.car/car.json
```

## Success Criteria

✅ **Schema Valid**: `car.json` passes v0.2 schema validation
✅ **CLI Verified**: `intelexta-verify` reports "Verified" status
✅ **Web Verified**: Web verifier shows green checkmarks
✅ **Deterministic**: Same inputs → same `car:` ID
✅ **Signature Valid**: Signed bundles validate with public key
✅ **Documented**: CAR-Lite profile clearly explained for plugin authors

## Next Steps (Future SDKs)

Once CAR-Lite is proven in verifiable-summary:

1. **Extract to SDK**: Create `@intelexta/car-lite` npm package
2. **Python SDK**: Port to `intelexta-car` pip package
3. **Go/Rust**: Minimal builders + schema validation
4. **CLI Tools**: `car build`, `car sign`, `car verify`, `car zip`
5. **Conformance Tests**: Golden test vectors for plugins to validate against

## References

- Schema: `/home/marcelo/Documents/codes/gaugefreedom/intelexta/schemas/car-v0.2.schema.json`
- Web Verifier: `/home/marcelo/Documents/codes/gaugefreedom/intelexta/apps/web-verifier/`
- CLI Verifier: `/home/marcelo/Documents/codes/gaugefreedom/intelexta/src-tauri/target/release/intelexta-verify`
- Current Implementation: `/home/marcelo/Documents/codes/gaugefreedom/intelexta/apps/verifiable-summary/server/src/provenance.ts`
