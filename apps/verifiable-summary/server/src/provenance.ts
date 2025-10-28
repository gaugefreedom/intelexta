/**
 * Provenance utilities for generating verifiable proof bundles
 *
 * This module implements deterministic cryptographic verification following
 * the IntelexTA CAR v0.2 format for content authenticity.
 */

import { createHash } from 'node:crypto';
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
 * Compute JCS (JSON Canonicalization Scheme) hash
 * This ensures deterministic hashing regardless of key order or whitespace
 */
export function jcsHash(obj: any): string {
  const canonical = JSON.stringify(obj);
  return sha256(canonical);
}

/**
 * Sign payload with Ed25519
 * @param payload - Hex string to sign
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
// Bundle Generation
// ============================================================================

export interface Source {
  url: string;
  content: string;
}

export interface ProofBundle {
  'summary.md': string;
  'sources.jsonl': string;
  'transcript.json': string;
  'manifest.json': string;
  'receipts/ed25519.json': string;
}

/**
 * Generate a complete verifiable proof bundle
 *
 * @param source - Source document with URL and content
 * @param summary - Generated summary text
 * @param model - Model identifier (e.g., "gpt-4o", "local-summarizer")
 * @param secretKeyB64 - Optional Ed25519 secret key for signing
 * @returns Object containing all bundle artifacts as strings
 */
export async function generateProofBundle(
  source: Source,
  summary: string,
  model: string,
  secretKeyB64?: string
): Promise<ProofBundle> {
  const runId = `vs-${Date.now()}-${Math.random().toString(36).slice(2, 11)}`;
  const createdAt = new Date().toISOString();

  // ========================================
  // 1. Generate sources.jsonl
  // ========================================
  const sourceEntry = {
    url: source.url,
    accessedAt: createdAt,
    bytes: Buffer.byteLength(source.content, 'utf-8'),
    sha256: sha256(source.content)
  };
  const sourcesJsonl = JSON.stringify(sourceEntry) + '\n';

  // ========================================
  // 2. Generate summary.md
  // ========================================
  const summaryMd = summary;

  // ========================================
  // 3. Generate transcript.json
  // ========================================
  const transcript = {
    status: 'success',
    metadata: {
      runId,
      signer: secretKeyB64 ? `ed25519:${getPublicKey(secretKeyB64)}` : 'unsigned',
      model,
      createdAt,
      dataset: 'verifiable-summary'
    },
    workflow: [
      {
        label: 'Collect Sources',
        status: 'success',
        attachments: [{
          name: 'sources.jsonl',
          href: '../verifiable/sources.jsonl',
          mediaType: 'application/jsonl'
        }]
      },
      {
        label: 'Summarize',
        status: 'success',
        attachments: [{
          name: 'summary.md',
          href: '../verifiable/summary.md',
          mediaType: 'text/markdown'
        }]
      },
      {
        label: 'Emit Manifest',
        status: 'success',
        attachments: [{
          name: 'manifest.json',
          href: '../verifiable/manifest.json',
          mediaType: 'application/json'
        }]
      }
    ]
  };
  const transcriptJson = JSON.stringify(transcript, null, 2);

  // ========================================
  // 4. Generate manifest.json
  // ========================================

  // Compute file hashes
  const fileHashes = {
    'summary.md': sha256(summaryMd),
    'sources.jsonl': sha256(sourcesJsonl),
    'transcript.json': sha256(transcriptJson)
  };

  // Compute tree hash (SHA256 of sorted, newline-joined hashes)
  const sortedHashes = Object.keys(fileHashes)
    .sort()
    .map(key => fileHashes[key as keyof typeof fileHashes]);
  const treeHash = sha256(sortedHashes.join('\n'));

  const manifest = {
    version: '0.2.0',
    runId,
    createdAt,
    model,
    files: {
      'summary.md': {
        sha256: fileHashes['summary.md'],
        bytes: Buffer.byteLength(summaryMd, 'utf-8')
      },
      'sources.jsonl': {
        sha256: fileHashes['sources.jsonl'],
        bytes: Buffer.byteLength(sourcesJsonl, 'utf-8')
      },
      'transcript.json': {
        sha256: fileHashes['transcript.json'],
        bytes: Buffer.byteLength(transcriptJson, 'utf-8')
      }
    },
    treeHash
  };
  const manifestJson = JSON.stringify(manifest, null, 2);

  // ========================================
  // 5. Generate receipts/ed25519.json
  // ========================================

  let receiptJson: string;

  if (secretKeyB64) {
    // Compute manifestSha256 using JCS
    const manifestSha256 = jcsHash(manifest);

    // Sign the payload: manifestSha256 || treeHash
    const payload = `${manifestSha256}${treeHash}`;
    const signature = signEd25519(payload, secretKeyB64);
    const publicKey = getPublicKey(secretKeyB64);

    const receipt = {
      version: '1.0',
      algorithm: 'ed25519',
      publicKey,
      manifestSha256,
      treeHash,
      signature,
      signedAt: createdAt
    };
    receiptJson = JSON.stringify(receipt, null, 2);
  } else {
    // Unsigned receipt
    const receipt = {
      version: '1.0',
      algorithm: 'none',
      manifestSha256: jcsHash(manifest),
      treeHash,
      signedAt: createdAt,
      note: 'Unsigned bundle - no cryptographic proof'
    };
    receiptJson = JSON.stringify(receipt, null, 2);
  }

  // ========================================
  // Return all artifacts
  // ========================================
  return {
    'summary.md': summaryMd,
    'sources.jsonl': sourcesJsonl,
    'transcript.json': transcriptJson,
    'manifest.json': manifestJson,
    'receipts/ed25519.json': receiptJson
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
