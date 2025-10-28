export interface WorkflowStep {
  id?: string;
  label?: string;
  status?: string;
  started_at?: string;
  finished_at?: string;
  prompt?: string;
  output?: string;
}

export interface VerificationMetadata {
  runId?: string;
  signer?: string;
  model?: string;
  createdAt?: string;
  dataset?: string;
}

export interface VerificationResult {
  metadata?: VerificationMetadata;
  workflow?: WorkflowStep[];
  raw?: unknown;
}

type VerifierModule = {
  default?: (input?: RequestInfo | URL, init?: RequestInit) => Promise<unknown>;
  init_verifier?: () => Promise<void>;
  verify_car_bytes?: (bytes: Uint8Array) => Promise<unknown>;
  verify_car_json?: (json: string) => Promise<unknown>;
};

let modulePromise: Promise<VerifierModule> | null = null;

async function loadModule(): Promise<VerifierModule> {
  if (!modulePromise) {
    modulePromise = import(/* @vite-ignore */ '/pkg/web_verifier.js') as Promise<VerifierModule>;
    const mod = await modulePromise;
    if (typeof mod.default === 'function') {
      await mod.default('/pkg/web_verifier_bg.wasm');
    }
    return mod;
  }

  return modulePromise;
}

function normalizeResult(value: unknown): VerificationResult {
  if (!value) {
    return {};
  }

  if (typeof value === 'string') {
    try {
      return JSON.parse(value) as VerificationResult;
    } catch (error) {
      return { raw: value };
    }
  }

  if (typeof value === 'object') {
    return value as VerificationResult;
  }

  return { raw: value };
}

export async function initVerifier(): Promise<void> {
  const mod = await loadModule();
  if (typeof mod.init_verifier === 'function') {
    await mod.init_verifier();
  }
}

export async function verifyCarBytes(bytes: Uint8Array): Promise<VerificationResult> {
  const mod = await loadModule();
  if (!mod.verify_car_bytes) {
    throw new Error('verify_car_bytes is not exported by the WASM bundle');
  }

  const result = await mod.verify_car_bytes(bytes);
  return normalizeResult(result);
}

export async function verifyCarJson(json: string): Promise<VerificationResult> {
  const mod = await loadModule();
  if (!mod.verify_car_json) {
    throw new Error('verify_car_json is not exported by the WASM bundle');
  }

  const result = await mod.verify_car_json(json);
  return normalizeResult(result);
}
