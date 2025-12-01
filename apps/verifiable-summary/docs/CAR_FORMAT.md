# CAR Format Specification

This document describes the Content-Addressable Receipt (CAR) format used by Intelexta and compatible tools.

## Overview

A CAR bundle is a ZIP archive containing cryptographically verifiable proof of workflow execution. It consists of:

1. **car.json** - Main verification manifest (REQUIRED)
2. **attachments/** - Directory containing input/output files referenced by SHA-256 hashes

## File Structure

```
bundle.car.zip
├── car.json                        # Main manifest (REQUIRED)
└── attachments/
    ├── <sha256-hash-1>.txt        # Input/output files
    ├── <sha256-hash-2>.txt
    └── ...
```

## car.json Schema

The `car.json` file must contain the following structure:

```typescript
{
  // Unique identifier for this CAR bundle
  "id": "car:<sha256-hash>",

  // Workflow run identifier
  "run_id": "string (UUID or custom format)",

  // ISO 8601 timestamp
  "created_at": "2025-10-28T06:09:36.822396593Z",

  // Workflow metadata
  "run": {
    "kind": "concordant",              // Proof mode
    "name": "workflow name",           // Human-readable name
    "model": "workflow:<model-name>",  // Model identifier with workflow: prefix
    "version": "<sha256-hash>",        // Workflow version hash
    "seed": 12345678,                  // Random seed
    "steps": [                         // Workflow steps
      {
        "id": "string (UUID)",
        "runId": "string (matches run_id)",
        "orderIndex": 0,
        "checkpointType": "StepName",
        "stepType": "step_type",
        "model": "model-name",
        "prompt": "prompt text",
        "tokenBudget": 4000,
        "proofMode": "concordant",
        "epsilon": 0.5,
        "configJson": "{\"key\":\"value\"}"
      }
    ]
  },

  // Cryptographic proof chain
  "proof": {
    "match_kind": "semantic",
    "process": {
      "sequential_checkpoints": [
        {
          "id": "string (UUID)",
          "prev_chain": "sha256-hash (empty string for first checkpoint)",
          "curr_chain": "sha256-hash of checkpoint",
          "signature": "base64-encoded Ed25519 signature (empty if unsigned)",
          "run_id": "string (matches run_id)",
          "kind": "Step",
          "timestamp": "ISO 8601 timestamp",
          "inputs_sha256": "sha256-hash of inputs",
          "outputs_sha256": "sha256-hash of outputs",
          "usage_tokens": 0,
          "prompt_tokens": 0,
          "completion_tokens": 0
        }
      ]
    }
  },

  // Policy reference
  "policy_ref": {
    "hash": "sha256:<hash>",
    "egress": true,
    "ingress": false
  },

  // Ed25519 public key (null if unsigned)
  "public_key": "base64-encoded-public-key or null",

  // Attachment references
  "attachments": [
    {
      "checkpoint_id": "string (UUID)",
      "sha256": "sha256-hash matching filename",
      "role": "output or input",
      "name": "friendly-name.txt"
    }
  ]
}
```

## Verification Process

The web-verifier performs these checks:

1. **File Existence**: Ensures `car.json` exists
2. **Attachment Integrity**: Verifies each attachment file matches its SHA-256 hash
3. **Chain Validation**: Validates `prev_chain` → `curr_chain` hash progression
4. **Signature Verification**: If `public_key` is present, verifies Ed25519 signatures
5. **Content Integrity**: Ensures inputs/outputs match checkpoint hashes

## Creating Compatible CAR Bundles

### Required Fields

- `id`: Must start with `car:` prefix
- `run_id`: Unique identifier for the workflow run
- `created_at`: ISO 8601 timestamp
- `run.model`: Must start with `workflow:` prefix
- `proof.process.sequential_checkpoints`: At least one checkpoint
- `attachments`: Array of attachment references

### Attachment Files

- Must be placed in `attachments/` directory
- Filename must be `<sha256-hash>.txt` where hash matches file content
- Each attachment must be referenced in `car.json` attachments array

### Signing (Optional)

To create a signed bundle:

1. Generate Ed25519 keypair
2. Compute checkpoint chain hashes
3. Sign each checkpoint's `curr_chain` with private key
4. Include `signature` in checkpoint and `public_key` in car.json

### Unsigned Bundles

Unsigned bundles are valid but provide no cryptographic proof. Set:
- `public_key`: `null`
- `signature`: `""` (empty string)

## Example Minimal car.json

```json
{
  "id": "car:abc123...",
  "run_id": "test-run-001",
  "created_at": "2025-10-28T10:00:00Z",
  "run": {
    "kind": "concordant",
    "name": "test workflow",
    "model": "workflow:test",
    "version": "def456...",
    "seed": 12345,
    "steps": [
      {
        "id": "step-001",
        "runId": "test-run-001",
        "orderIndex": 0,
        "checkpointType": "Test",
        "stepType": "test",
        "model": "test-model",
        "prompt": "test prompt",
        "tokenBudget": 1000,
        "proofMode": "concordant",
        "epsilon": 0.5,
        "configJson": "{}"
      }
    ]
  },
  "proof": {
    "match_kind": "semantic",
    "process": {
      "sequential_checkpoints": [
        {
          "id": "checkpoint-001",
          "prev_chain": "",
          "curr_chain": "abc123...",
          "signature": "",
          "run_id": "test-run-001",
          "kind": "Step",
          "timestamp": "2025-10-28T10:00:00Z",
          "inputs_sha256": "input-hash",
          "outputs_sha256": "output-hash",
          "usage_tokens": 0,
          "prompt_tokens": 0,
          "completion_tokens": 0
        }
      ]
    }
  },
  "policy_ref": {
    "hash": "sha256:policy-hash",
    "egress": true,
    "ingress": false
  },
  "public_key": null,
  "attachments": [
    {
      "checkpoint_id": "checkpoint-001",
      "sha256": "output-hash",
      "role": "output",
      "name": "output.txt"
    }
  ]
}
```

## Tools

- **Intelexta Desktop**: Native CAR generation with full workflow tracking
- **Verifiable Summary MCP**: Generates CAR bundles for content summarization
- **Web Verifier**: Browser-based CAR verification at `apps/web-verifier`
- **CLI Verifier**: Command-line verification tool `intelexta-verify`

## References

- Web Verifier Types: `apps/web-verifier/src/types/verifier.ts`
- WASM Verifier: `apps/web-verifier/wasm-verify/`
- MCP Server Implementation: `apps/verifiable-summary/server/src/provenance.ts`
