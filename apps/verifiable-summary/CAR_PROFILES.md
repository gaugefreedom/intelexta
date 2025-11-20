# CAR Profiles: Lite vs Full

This document explains the two profiles built on top of Intelexta CAR v0.3 schema:

- **CAR-Lite**: Simplified profile for community plugins and easy adoption
- **CAR-Full**: Rich evidence profile used by Intelexta Desktop

Both profiles are 100% compliant with `schemas/car-v0.3.schema.json` and can be verified by the same tools (web-verifier, intelexta-verify CLI).

## Why Two Profiles?

**Problem**: Community plugins (Ollama, LangChain, LlamaIndex, Airflow, Prefect) don't have access to all the runtime metrics that Intelexta Desktop tracks.

**Solution**: Define minimal defaults for unknown fields while maintaining full schema compliance.

**Benefit**: Plugins can generate verifiable proofs immediately without complex instrumentation, then upgrade to richer evidence over time.

## Schema Compliance

Both profiles implement the same schema (`car-v0.3.schema.json`) with identical **required fields**:

```typescript
{
  id: string,                    // "car:<64-hex>"
  run_id: string,
  created_at: string,            // ISO 8601
  run: RunObject,
  proof: ProofObject,
  policy_ref: PolicyRefObject,
  budgets: BudgetsObject,
  provenance: ProvenanceClaim[], // minItems: 1
  checkpoints: string[],         // minItems: 1
  sgrade: SgradeObject,
  signer_public_key: string,
  signatures: string[]           // minItems: 1
}
```

## CAR-Lite Profile

### Purpose
- **Fast adoption** for community plugins
- **Minimal instrumentation** required
- **Neutral defaults** for unknown metrics
- **Development-friendly** (unsigned bundles allowed)

### Field Mapping

| Field | CAR-Lite Value | Rationale |
|-------|----------------|-----------|
| `proof.match_kind` | `"process"` | Minimal verification (no semantic/exact matching required) |
| `budgets.usd` | `0` | Unknown cost = 0 (can be updated later) |
| `budgets.tokens` | `0` | Unknown tokens = 0 (or estimated if available) |
| `budgets.nature_cost` | `0` | Unknown environmental cost = 0 |
| `policy_ref.hash` | `"sha256:<hash>"` | Hash of static policy document |
| `policy_ref.egress` | `true` | Conservative default (network allowed) |
| `policy_ref.estimator` | `"usage_tokens * 0.010000 nature_cost/token"` | Generic formula |
| `provenance` | `[{config}, {input}, {output}]` | Minimum 3 claims |
| `checkpoints` | `["ckpt:<run_id>"]` | Single synthetic checkpoint |
| `sgrade.score` | `70-97` | Baseline compliance score |
| `sgrade.components` | `{provenance:1.0, energy:1.0, replay:0.8, consent:0.8, incidents:1.0}` | Neutral component scores |
| `run.kind` | `"concordant"` | Standard proof mode |
| `run.seed` | `0` or random | Use 0 if determinism unavailable |
| `run.steps[*].epsilon` | `0.5` | Default concordant tolerance |
| `signer_public_key` | `""` (empty) or base64 | Empty for unsigned |
| `signatures` | `["unsigned:"]` or `["ed25519:<sig>"]` | Mark unsigned explicitly |

### Example CAR-Lite Bundle

```json
{
  "id": "car:8f3a2c1b9e4d5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b",
  "run_id": "vs-1730174400000-abc123def",
  "created_at": "2025-10-28T21:00:00Z",
  "run": {
    "kind": "concordant",
    "name": "verifiable summary",
    "model": "workflow:gpt-4o-mini",
    "version": "a1b2c3d4e5f6...",
    "seed": 42,
    "steps": [{
      "id": "vs-1730174400000-abc123def",
      "run_id": "vs-1730174400000-abc123def",
      "order_index": 0,
      "checkpoint_type": "Summary",
      "step_type": "summarize",
      "model": "gpt-4o-mini",
      "prompt": "Summarize content from: https://example.com/article",
      "token_budget": 4000,
      "proof_mode": "concordant",
      "epsilon": 0.5,
      "config_json": "{\"source_url\":\"https://example.com/article\"}"
    }]
  },
  "proof": {
    "match_kind": "process"
  },
  "policy_ref": {
    "hash": "sha256:1234567890abcdef...",
    "egress": true,
    "estimator": "usage_tokens * 0.010000 nature_cost/token"
  },
  "budgets": {
    "usd": 0,
    "tokens": 0,
    "nature_cost": 0
  },
  "provenance": [
    { "claim_type": "config", "sha256": "sha256:policy_hash..." },
    { "claim_type": "input", "sha256": "sha256:input_hash..." },
    { "claim_type": "output", "sha256": "sha256:output_hash..." }
  ],
  "checkpoints": ["ckpt:vs-1730174400000-abc123def"],
  "sgrade": {
    "score": 85,
    "components": {
      "provenance": 1.0,
      "energy": 1.0,
      "replay": 0.8,
      "consent": 0.8,
      "incidents": 1.0
    }
  },
  "signer_public_key": "",
  "signatures": ["unsigned:"]
}
```

### When to Use CAR-Lite

✅ **Use CAR-Lite when:**
- Building a community plugin or integration
- You don't have full runtime metrics
- You want to ship quickly and iterate
- You're prototyping or in development

❌ **Don't use CAR-Lite when:**
- You have access to full runtime data (use CAR-Full)
- You need rich provenance for audit/compliance
- You're building production Intelexta workflows

## CAR-Full Profile

### Purpose
- **Rich evidence** for audit and compliance
- **Full provenance** tracking with multiple checkpoints
- **Precise metrics** (tokens, cost, nature impact)
- **Production-ready** Intelexta workflows

### Additional Fields (vs CAR-Lite)

| Field | CAR-Full Enhancement |
|-------|----------------------|
| `proof.match_kind` | `"semantic"` or `"exact"` with full digest tracking |
| `proof.epsilon` | Actual semantic similarity threshold |
| `proof.distance_metric` | Metric used (e.g., `"cosine"`, `"euclidean"`) |
| `proof.original_semantic_digest` | Digest from original run |
| `proof.replay_semantic_digest` | Digest from replay verification |
| `budgets.usd` | Actual USD cost from provider |
| `budgets.tokens` | Real token count (prompt + completion) |
| `budgets.nature_cost` | Calculated environmental impact |
| `policy_ref.estimator` | Precise formula tied to internal accounting |
| `provenance` | Additional claims (datasets, model cards, tool I/O) |
| `checkpoints` | Multiple checkpoints for full step chain |
| `run.sampler` | `{temp, top_p, rng}` for reproducibility |
| `run.steps[*]` | Richer metadata per step (actual usage, timestamps) |
| `sgrade.score` | Calculated from actual metrics |

### Example CAR-Full Bundle

See Intelexta Desktop exports at `~/Documents/teste/llmquestion.car/car.json` for full examples.

## Migration Path

### Phase 1: Start with CAR-Lite
```bash
npm install @intelexta/car-lite  # (future SDK)

import { CarLiteBuilder } from '@intelexta/car-lite';

const car = new CarLiteBuilder()
  .setRunId('my-plugin-run-123')
  .addInput(sourceText)
  .addOutput(summary)
  .sign(secretKey)
  .build();
```

### Phase 2: Add Metrics (CAR-Lite+)
```typescript
const car = new CarLiteBuilder()
  .setRunId('my-plugin-run-123')
  .addInput(sourceText)
  .addOutput(summary)
  // Add metrics as you instrument your plugin
  .setBudgets({ usd: 0.005, tokens: 1234, nature_cost: 0.012 })
  .setProofMode('semantic', { epsilon: 0.3, metric: 'cosine' })
  .sign(secretKey)
  .build();
```

### Phase 3: Upgrade to CAR-Full
```typescript
const car = new CarFullBuilder()
  .setRunId('my-plugin-run-123')
  .addCheckpoint(step1Checkpoint)
  .addCheckpoint(step2Checkpoint)
  .setProvenance([...richProvenanceClaims])
  .setSampler({ temp: 0.7, top_p: 0.9, rng: 'pcg64' })
  .setBudgets(actualBudgets)
  .sign(secretKey)
  .build();
```

## Verification

Both profiles verify identically:

### Web Verifier
```bash
cd apps/web-verifier
npm run dev
# Upload .car.zip at http://localhost:5173
```

### CLI Verifier
```bash
./src-tauri/target/release/intelexta-verify ~/path/to/bundle.car.zip
```

### Schema Validation
```bash
npm install -g ajv-cli
ajv validate -s schemas/car-v0.3.schema.json -d bundle.car/car.json
```

## Deterministic ID Computation

Both profiles use identical canonicalization:

1. Build car.json **without** `id` and `signatures`
2. Canonicalize using **JCS** (RFC 8785)
3. Compute `id = "car:" + SHA256(canonical_bytes)`
4. Add `id` to car.json
5. Canonicalize again (with `id`, without `signatures`)
6. Sign canonical bytes
7. Add `signatures` array

```typescript
import { canonicalize } from 'json-canonicalize';
import { sha256 } from 'crypto';

const carBody = { run_id, created_at, run, proof, ... }; // no id, no signatures
const canonical = canonicalize(carBody);
const carId = `car:${sha256(canonical)}`;

const bodyWithId = { id: carId, ...carBody };
const canonicalWithId = canonicalize(bodyWithId);
const signature = sign(canonicalWithId, secretKey);

const finalCar = { ...bodyWithId, signatures: [`ed25519:${signature}`] };
```

## Signing Strategy

### Signed (Production)
```typescript
// Generate keypair once
const { publicKey, secretKey } = generateKeypair();

// Store secret key securely (environment variable, key vault)
process.env.ED25519_SECRET_KEY = secretKey;

// Generate signed bundle
const { bundle } = await generateProofBundle(source, summary, model, secretKey);

// Result:
// - signer_public_key: "<base64>"
// - signatures: ["ed25519:<base64-signature>"]
```

### Unsigned (Development)
```typescript
// Generate without secret key
const { bundle } = await generateProofBundle(source, summary, model);

// Result:
// - signer_public_key: ""
// - signatures: ["unsigned:"]
```

## Implementation Reference

### CAR-Lite Implementation
- **Code**: `apps/verifiable-summary/server/src/provenance.ts`
- **Tests**: `apps/verifiable-summary/server/src/provenance.test.ts`
- **Schema**: `schemas/car-v0.3.schema.json`

### CAR-Full Implementation
- **Code**: Intelexta Desktop (Rust)
- **Examples**: `~/Documents/teste/*.car/car.json`

## Future SDKs

### Node.js (`@intelexta/car-lite`)
```bash
npm install @intelexta/car-lite
```

Features:
- JCS canonicalization
- SHA-256 hashing
- Ed25519 signing
- Schema validation
- CAR-Lite builder API

### Python (`intelexta-car`)
```bash
pip install intelexta-car
```

Features:
- Same helpers as Node.js SDK
- Pythonic builder API
- Compatible with LangChain, LlamaIndex

### CLI (`car` command)
```bash
npm install -g @intelexta/car-cli

car build < workflow.jsonl > car.json
car sign --key secret.key car.json > signed.car.json
car verify bundle.car.zip
car zip car.json attachments/ -o bundle.car.zip
```

## Conformance Testing

### Golden Test Vectors

We provide reference CAR bundles for plugin authors to test against:

```bash
# Download test vectors
curl -O https://intelexta.org/test-vectors/car-lite-unsigned.zip
curl -O https://intelexta.org/test-vectors/car-lite-signed.zip

# Verify your implementation matches
diff <(cat your-bundle.car/car.json | jq -S .) \
     <(cat test-vector.car/car.json | jq -S .)
```

### Conformance Test Suite

```bash
npm install --save-dev @intelexta/car-conformance

# Run conformance tests
car-conformance test your-bundle.car.zip
```

Checks:
- ✅ Schema validation against v0.2
- ✅ JCS canonicalization correctness
- ✅ ID computation determinism
- ✅ Signature verification
- ✅ Attachment hash integrity

## Summary Table

| Feature | CAR-Lite | CAR-Full |
|---------|----------|----------|
| **Schema Compliance** | ✅ 100% v0.2 | ✅ 100% v0.2 |
| **Required Instrumentation** | Minimal | Full runtime tracking |
| **Proof Mode** | `process` | `semantic` or `exact` |
| **Budgets** | Neutral (0) | Actual metrics |
| **Provenance** | 3 claims (config, input, output) | Rich claims (datasets, model cards) |
| **Checkpoints** | 1 synthetic | Multiple real checkpoints |
| **Signing** | Optional (unsigned allowed) | Required for production |
| **Verification** | ✅ web-verifier, CLI | ✅ web-verifier, CLI |
| **Use Case** | Community plugins, prototypes | Production Intelexta workflows |
| **Time to Integrate** | < 1 day | Weeks (full instrumentation) |

## Getting Started

1. **Review Examples**: Check `apps/verifiable-summary/server/src/provenance.ts`
2. **Read Schema**: Understand `schemas/car-v0.3.schema.json`
3. **Generate Bundle**: Use `generateProofBundle()` from provenance.ts
4. **Verify**: Test with web-verifier and `intelexta-verify` CLI
5. **Iterate**: Add richer metrics over time

## Questions?

- **Schema Questions**: See `schemas/car-v0.3.schema.json` comments
- **Implementation**: See `apps/verifiable-summary/CAR_LITE_PLAN.md`
- **Issues**: https://github.com/gaugefreedom/intelexta/issues
