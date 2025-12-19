/**
 * Provenance utilities for generating CAR-Lite verifiable proof bundles
 *
 * This module implements CAR-Lite profile - a simplified but fully compliant
 * subset of Intelexta CAR v0.2 format designed for easy community adoption.
 */

import { createHash } from 'node:crypto';
import { canonicalize } from 'json-canonicalize';
import nacl from 'tweetnacl';
import util from 'tweetnacl-util';

const { encodeBase64, decodeBase64 } = util;

// ============================================================================
// Core Cryptographic Utilities
// ============================================================================

/**
 * Compute SHA-256 hash of data
 */
export function sha256(data: string | Buffer): string {
  const hash = createHash('sha256');
  hash.update(data);
  return hash.digest('hex');
}

/**
 * Compute JCS (JSON Canonicalization Scheme) hash per RFC 8785
 * This ensures deterministic hashing regardless of key order or whitespace
 */
export function jcsHash(obj: any): string {
  const canonical = canonicalize(obj);
  return sha256(canonical);
}

/**
 * Sign payload with Ed25519
 * @param payload - UTF-8 string to sign
 * @param secretKeyB64 - Base64-encoded Ed25519 secret key (64 bytes)
 * @returns Base64-encoded signature
 */
export function signEd25519(payload: string, secretKeyB64: string): string {
  const secretKey = decodeBase64(secretKeyB64);
  if (secretKey.length !== 64) {
    throw new Error('Ed25519 secret key must be 64 bytes');
  }

  const messageBytes = Buffer.from(payload, 'utf-8');
  const signature = nacl.sign.detached(messageBytes, secretKey);
  return encodeBase64(signature);
}

/**
 * Extract public key from secret key
 */
export function getPublicKey(secretKeyB64: string): string {
  const secretKey = decodeBase64(secretKeyB64);
  const keyPair = nacl.sign.keyPair.fromSecretKey(secretKey);
  return encodeBase64(keyPair.publicKey);
}

// ============================================================================
// CAR-Lite Bundle Generation
// ============================================================================

export interface Source {
  url: string;
  content: string;
}

export interface ProofBundle {
  'car.json': string;
  [key: `attachments/${string}`]: string;
}

export interface ProofBundleResult {
  bundle: ProofBundle;
  isSigned: boolean;
}

export interface GenerateProofBundleOptions {
  includeSource?: boolean;
  previewLength?: number;
  usage?: {
    prompt_tokens: number;
    completion_tokens: number;
  };
}

/**
 * Generate a CAR-Lite compliant verifiable proof bundle
 *
 * CAR-Lite is a simplified profile of Intelexta CAR v0.2 that:
 * - Uses neutral defaults for unknown metrics (budgets, sgrade)
 * - Supports minimal provenance tracking (config, input, output)
 * - Allows unsigned bundles for development
 * - Maintains 100% schema compliance with car-v0.2.schema.json
 *
 * @param source - Source document with URL and content
 * @param summary - Generated summary text
 * @param model - Model identifier (e.g., "gpt-4o-mini", "chatgpt-summarizer")
 * @param secretKeyB64 - Optional Ed25519 secret key for signing
 * @param options - Additional generation options
 * @returns Object containing all bundle artifacts as strings
 */
export async function generateProofBundle(
  source: Source,
  summary: string,
  model: string,
  secretKeyB64?: string,
  options: GenerateProofBundleOptions = {}
): Promise<ProofBundleResult> {
  const runId = `vs-${Date.now()}-${Math.random().toString(36).slice(2, 11)}`;
  const createdAt = new Date().toISOString();
  const includeSource = options.includeSource ?? false;
  const previewLength = options.previewLength ?? 200;
  const hasMeasuredUsage = Boolean(options.usage);
  const promptTokensMeasured = options.usage?.prompt_tokens;
  const completionTokensMeasured = options.usage?.completion_tokens;

  const estimatedPromptTokens = Math.ceil(source.content.length / 4);
  const estimatedCompletionTokens = Math.ceil(summary.length / 4);

  const promptTokens = promptTokensMeasured ?? estimatedPromptTokens;
  const completionTokens = completionTokensMeasured ?? estimatedCompletionTokens;
  const totalTokens = promptTokens + completionTokens;
  const tokenMode = hasMeasuredUsage ? 'measured' : 'estimated';

  // ========================================
  // 1. Generate attachment files
  // ========================================

  // Summary output
  const summaryContent = `# Verifiable Summary

Generated: ${createdAt}
Model: ${model}
Source: ${source.url}

## Summary

${summary}
`;
  const summaryHash = sha256(summaryContent);

  const preview = source.content.slice(0, previewLength);
  const previewWithEllipsis = source.content.length > previewLength ? `${preview}...` : preview;

  const originalInputHash = sha256(source.content);

  const sourceMetadata = {
    source_url: source.url,
    accessed_at: createdAt,
    bytes: Buffer.byteLength(source.content, 'utf-8'),
    input_sha256: `sha256:${originalInputHash}`,
    preview: previewWithEllipsis
  };
  const metadataJson = JSON.stringify(sourceMetadata, null, 2);
  const metadataHash = sha256(metadataJson);
  const inputsSha256 = includeSource ? originalInputHash : metadataHash;

  const sourceAttachmentContent = includeSource ? source.content : metadataJson;
  const sourceAttachmentName = includeSource ? `${inputsSha256}.txt` : `${metadataHash}.json`;

  // Policy document (static for CAR-Lite)
  const policyDoc = `Verifiable Summary Policy v1.0

This policy governs the verifiable-summary workflow:
- Allows inline content ingestion from caller-provided text
- Allows summarization using configured LLM
- Network egress: ${hasMeasuredUsage ? 'OpenAI API call observed' : 'none observed (local summarization)'}
- Data retention: bounded TTL persistent storage for bundle downloads
- Token accounting: ${tokenMode} via ${hasMeasuredUsage ? 'OpenAI usage' : 'chars/4 heuristic'}

Nature cost estimator: usage_tokens * 0.010000 nature_cost/token
`;
  const policyHash = sha256(policyDoc);

  // Build run.steps array first (needed for config provenance)
  const runSteps = [
    {
      id: runId,
      runId: runId,  // camelCase for WASM verifier
      orderIndex: 0,
      checkpointType: 'Summary',
      stepType: 'summarize',
      model: model,
      prompt: `Summarize content from: ${source.url}`,
      tokenBudget: totalTokens || 4000,
      proofMode: 'concordant' as const,
      epsilon: 0.5,
      configJson: JSON.stringify({
        source_url: source.url,
        content_length: source.content.length,
        summarization_style: 'concise',
        token_mode: tokenMode,
        tokens_measured: hasMeasuredUsage,
        tokens_estimated: !hasMeasuredUsage,
        estimated_prompt_tokens: promptTokens,
        estimated_completion_tokens: completionTokens,
        estimated_total_tokens: totalTokens,
        budgets_mode: tokenMode,
        sgrade_not_computed: true
      })
    }
  ];

  // Compute config hash from canonical JSON of steps (what verifier expects)
  const configHash = jcsHash(runSteps);

  // ========================================
  // 2. Build CAR body (without id and signatures)
  // ========================================

  const checkpointId = `ckpt:${runId}`;

  // ========================================
  // 2. Build checkpoint proof chain
  // ========================================

  const outputsSha256 = summaryHash;

  // For CAR-Lite, we create a single synthetic checkpoint
  const prevChain = '';

  // Build checkpoint body matching Intelexta's structure
  const checkpointBody = {
    run_id: runId,
    kind: 'Step',
    timestamp: createdAt,
    inputs_sha256: inputsSha256,
    outputs_sha256: outputsSha256,
    incident: null,
    usage_tokens: totalTokens,
    prompt_tokens: promptTokens,
    completion_tokens: completionTokens
  };

  // Compute curr_chain: SHA256(prev_chain + canonical_json(checkpoint_body))
  const checkpointCanonical = canonicalize(checkpointBody);
  const currChain = sha256(prevChain + checkpointCanonical);

  // Sign the chain hash if key provided
  let checkpointSignature = '';
  if (secretKeyB64) {
    checkpointSignature = signEd25519(currChain, secretKeyB64);
  }

  const carBody = {
    run_id: runId,
    created_at: createdAt,
    run: {
      kind: 'concordant' as const,
      name: 'verifiable summary',
      model: `workflow:${model}`,
      version: sha256(`${model}:v1.0`),
      seed: Math.floor(Math.random() * 100000000),
      steps: runSteps
    },
    proof: {
      match_kind: 'process' as const,
      process: {
        sequential_checkpoints: [
          {
            id: runId,
            prev_chain: prevChain,
            curr_chain: currChain,
            signature: checkpointSignature,
            run_id: runId,
            kind: 'Step',
            timestamp: createdAt,
            inputs_sha256: inputsSha256,
            outputs_sha256: outputsSha256,
            usage_tokens: totalTokens,
            prompt_tokens: promptTokens,
            completion_tokens: completionTokens
          }
        ]
      }
    },
    policy_ref: {
      hash: `sha256:${policyHash}`,
      egress: hasMeasuredUsage,
      estimator: hasMeasuredUsage ? 'openai_usage_tokens' : 'estimated:chars_per_token=4'
    },
    budgets: totalTokens
      ? {
          usd: estimateUsd(promptTokens, completionTokens, hasMeasuredUsage),
          tokens: totalTokens,
          nature_cost: Number((totalTokens * 0.01).toFixed(6))
        }
      : undefined,
    sgrade: {
      score: 0,
      components: {
        provenance: 0,
        energy: 0,
        replay: 0,
        consent: 0,
        incidents: 0
      }
    },
    provenance: [
      { claim_type: 'config', sha256: `sha256:${configHash}` },
      { claim_type: 'input', sha256: `sha256:${inputsSha256}` },
      { claim_type: 'input_meta', sha256: `sha256:${metadataHash}` },
      { claim_type: 'output', sha256: `sha256:${outputsSha256}` }
    ],
    checkpoints: [checkpointId],
    signer_public_key: secretKeyB64 ? getPublicKey(secretKeyB64) : ''
  };

  // ========================================
  // 4. Compute deterministic CAR ID
  // ========================================

  // Canonicalize using JCS (RFC 8785)
  const canonical = canonicalize(carBody);
  const carId = `car:${sha256(canonical)}`;

  // ========================================
  // 5. Sign the canonical body (if key provided)
  // ========================================

  let signatures: string[];

  if (secretKeyB64) {
    // Build body with ID (for signing the complete document)
    const bodyWithId = { id: carId, ...carBody };

    // Sign 1: Top-level body (covers ALL fields including created_at, budgets, sgrade)
    const bodyCanonical = canonicalize(bodyWithId);
    const bodySignature = signEd25519(bodyCanonical, secretKeyB64);

    // Sign 2: Use the checkpoint signature already computed in step 2
    // (checkpointSignature variable contains the signature of curr_chain)

    // Store both signatures
    // Format: ["ed25519-body:<sig>", "ed25519-checkpoint:<sig>"]
    signatures = [
      `ed25519-body:${bodySignature}`,
      `ed25519-checkpoint:${checkpointSignature}`
    ];
  } else {
    // Unsigned bundle (for dev/testing)
    signatures = ['unsigned:'];
  }

  // ========================================
  // 6. Build final car.json
  // ========================================

  const carJson = {
    id: carId,
    ...carBody,
    signatures
  };

  // ========================================
  // 7. Return all artifacts
  // ========================================

  const bundle: ProofBundle = {
    'car.json': JSON.stringify(carJson, null, 2),
    [`attachments/${summaryHash}.txt`]: summaryContent,
    [`attachments/${metadataHash}.json`]: metadataJson,
    [`attachments/${sourceAttachmentName}`]: sourceAttachmentContent
  };

  return {
    bundle,
    isSigned: Boolean(secretKeyB64)
  };
}

/**
 * Generate a new Ed25519 keypair
 * @returns Object with publicKey and secretKey (both base64-encoded)
 */
export function generateKeypair(): { publicKey: string; secretKey: string } {
  const keypair = nacl.sign.keyPair();
  return {
    publicKey: encodeBase64(keypair.publicKey),
    secretKey: encodeBase64(keypair.secretKey)
  };
}

function estimateUsd(promptTokens: number, completionTokens: number, usedLlm: boolean): number {
  if (!usedLlm) {
    return 0;
  }

  // gpt-4o-mini pricing (approx): $0.15 / 1M input tokens, $0.60 / 1M output tokens
  const promptCost = promptTokens * 0.15 / 1_000_000;
  const completionCost = completionTokens * 0.60 / 1_000_000;

  return Number((promptCost + completionCost).toFixed(6));
}
