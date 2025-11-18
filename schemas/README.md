# Intelexta CAR Schemas

This directory contains the official JSON Schema definitions for Intelexta Content-Addressable Receipts (CARs).

## Current Schema Version

**`car-v0.3.schema.json`** - Active schema (November 2025)

This version accurately reflects the current implementation of:
- Intelexta Desktop CAR exporter (CAR-Full)
- Verifiable Summary MCP server (CAR-Lite)
- `intelexta-verify` CLI verifier
- Web verifier

## Schema Versions

### v0.3 (Current)
**Status**: ✅ Active
**File**: `car-v0.3.schema.json`
**Release**: November 2025

**Key Features**:
- Includes `proof.process.sequential_checkpoints` structure
- Supports three `match_kind` modes: `"process"`, `"exact"`, `"semantic"`
- Uses hybrid naming convention (top-level: snake_case, run.steps: camelCase)
- Validates both Desktop CARs and MCP CAR-Lite bundles
- 100% backward compatible with existing implementations

**What's New**:
- Added `proof.process` and `process_checkpoint_proof` definitions
- Updated `run_step` field names to camelCase (matches actual CARs)
- Added optional `policy_ref.model_catalog_hash` and `model_catalog_version`
- Relaxed `checkpoints` pattern to accept both `ckpt:` prefixed and plain UUIDs
- Added conditional validation for `match_kind: "process"`

### v0.2 (Historical)
**Status**: 📚 Reference only
**File**: `car-v0.2.schema.json`
**Release**: Initial design

**Limitations**:
- Missing `proof.process` structure (implementation diverged)
- Used snake_case for `run_step` fields (actual CARs use camelCase)
- Strict `ckpt:` prefix requirement for checkpoint IDs

## Validation

### Quick Validation (No Dependencies)

```bash
node validate-car.js examples/sample.car.json
```

This performs basic structural checks without external dependencies.

### Full Schema Validation (Recommended)

Install dependencies:
```bash
npm install ajv ajv-formats
```

Validate against schema:
```bash
node -e "
const Ajv = require('ajv/dist/2020');
const addFormats = require('ajv-formats');
const fs = require('fs');

const ajv = new Ajv({ allErrors: true, strict: false });
addFormats(ajv);

const schema = JSON.parse(fs.readFileSync('car-v0.3.schema.json', 'utf8'));
const car = JSON.parse(fs.readFileSync('your-car.json', 'utf8'));

const validate = ajv.compile(schema);
if (validate(car)) {
  console.log('✓ Valid CAR');
} else {
  console.error('✗ Invalid:', validate.errors);
}
"
```

### Validate ZIP Bundles

Extract and validate:
```bash
unzip -p bundle.car.zip car.json > temp.car.json
node validate-car.js temp.car.json
```

Or use `intelexta-verify` CLI:
```bash
intelexta-verify bundle.car.zip
```

## CAR Structure Overview

```json
{
  "id": "car:<sha256>",
  "run_id": "...",
  "created_at": "2025-11-17T...",
  "run": {
    "kind": "exact" | "concordant" | "interactive",
    "steps": [
      {
        "id": "...",
        "runId": "...",           // camelCase!
        "orderIndex": 0,          // camelCase!
        "checkpointType": "...",  // camelCase!
        "tokenBudget": 1000,      // camelCase!
        "proofMode": "exact",     // camelCase!
        ...
      }
    ]
  },
  "proof": {
    "match_kind": "process" | "exact" | "semantic",
    "process": {                 // NEW in v0.3
      "sequential_checkpoints": [
        {
          "id": "...",
          "prev_chain": "",
          "curr_chain": "<sha256>",
          "signature": "<base64>",
          "run_id": "...",        // snake_case
          "kind": "Step",
          "timestamp": "...",
          "inputs_sha256": "...", // snake_case
          "outputs_sha256": "...",
          "usage_tokens": 0,
          "prompt_tokens": 0,
          "completion_tokens": 0
        }
      ]
    }
  },
  "policy_ref": { ... },
  "budgets": { ... },
  "provenance": [ ... ],
  "checkpoints": [ ... ],
  "sgrade": { ... },
  "signer_public_key": "...",
  "signatures": [ ... ]
}
```

## Naming Convention (Hybrid)

v0.3 uses a **hybrid naming convention** that matches the actual implementation:

| Location | Convention | Example |
|----------|-----------|---------|
| Top-level fields | snake_case | `run_id`, `created_at`, `signer_public_key` |
| `run.steps` fields | camelCase | `runId`, `orderIndex`, `checkpointType`, `tokenBudget` |
| `proof.process.sequential_checkpoints` | snake_case | `prev_chain`, `curr_chain`, `inputs_sha256` |

This is **intentional** and consistent across both Desktop and MCP implementations.

## match_kind Semantics

### `"process"`
- **Used by**: CAR-Lite (MCP), Desktop interactive workflows
- **Meaning**: Verify cryptographic hash chain and signatures
- **Requires**: `proof.process.sequential_checkpoints`
- **Does NOT require**: Byte-for-byte replay

### `"semantic"`
- **Used by**: Desktop concordant/semantic workflows
- **Meaning**: Verify via semantic similarity metrics
- **Current status**: Desktop emits this but doesn't yet populate semantic digest fields
- **Future**: Will require `epsilon`, `distance_metric`, `original_semantic_digest`

### `"exact"`
- **Used by**: Desktop exact replay workflows
- **Meaning**: Byte-for-byte deterministic replay
- **Current status**: Desktop may include `proof.process` even for exact mode

## Development Workflow

### Creating a New Schema Version

1. **Discovery Phase**:
   - Analyze actual CAR outputs from all implementations
   - Document field names, types, and structures
   - Identify gaps between schema and reality

2. **Schema Design**:
   - Create `car-vX.Y.schema.json` based on findings
   - Add new definitions to `$defs`
   - Update conditional validation rules

3. **Validation**:
   - Test against sample CARs from Desktop and MCP
   - Ensure backward compatibility
   - Document breaking changes

4. **Documentation**:
   - Update this README
   - Create migration guide (if needed)
   - Update downstream tools

### Testing Schema Changes

```bash
# Validate Desktop CAR
node -e "..." # (see Full Schema Validation above)

# Validate MCP CAR
unzip -p mcp-bundle.car.zip car.json | node validate-car.js /dev/stdin

# Run CLI verifier
intelexta-verify desktop-bundle.car.zip
intelexta-verify mcp-bundle.car.zip
```

## Documentation

- **`PHASE1_DISCOVERY_FINDINGS.md`**: Detailed analysis of current CAR implementation
- **`PHASE2_SCHEMA_IMPLEMENTATION.md`**: v0.3 schema creation process and validation results

## References

### Implementation Files
- Desktop exporter: `src-tauri/src/car.rs`
- MCP server: `apps/verifiable-summary/server/src/provenance.ts`
- CLI verifier: `src-tauri/crates/intelexta-verify/src/main.rs`

### Related Documentation
- `apps/verifiable-summary/CAR_FORMAT.md`: CAR format guide
- `apps/verifiable-summary/CAR_LITE_PLAN.md`: CAR-Lite profile specification

## Contributing

When proposing schema changes:

1. Ensure the change reflects **actual implementation behavior** (not aspirational)
2. Test against real CARs from both Desktop and MCP
3. Document the rationale in a Phase document
4. Update validation tests
5. Consider backward compatibility

## License

See the main Intelexta repository LICENSE file.

---

**Questions?** Check the discovery and implementation documents in this directory.
