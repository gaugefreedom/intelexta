import type {
  ModelSummary,
  SignerSummary,
  StepDetail,
  StepStatus,
  SummaryMetrics,
  VerificationReport,
  VerificationStatus,
  VerificationWorkflow,
  WasmVerificationReport,
  WorkflowStep
} from '../types/verifier';

// Shape of the wasm-bindgen JS glue
type VerifierModule = {
  default?: (input?: RequestInfo | URL, init?: RequestInit) => Promise<unknown>;
  init_verifier?: () => Promise<void>;
  verify_car_bytes?: (bytes: Uint8Array) => Promise<unknown>;
  verify_car_json?: (json: string) => Promise<unknown>;
};

let modulePromise: Promise<VerifierModule> | null = null;

async function loadModule(): Promise<VerifierModule> {
  if (!modulePromise) {
    // IMPORTANT: load via URL string, not a static import
    const jsUrl = new URL('/pkg/intelexta_wasm_verify.js', window.location.origin).toString();
    modulePromise = import(/* @vite-ignore */ jsUrl) as Promise<VerifierModule>;

    const mod = await modulePromise;

    // Let wasm-bindgen locate the .wasm relative to the JS file
    // If your glue expects a URL explicitly, pass the correct filename:
    if (typeof mod.default === 'function') {
      await mod.default(new URL('/pkg/intelexta_wasm_verify_bg.wasm', window.location.origin));
      // or simply: await mod.default();  // works for most wasm-pack builds
    }
    return mod;
  }
  return modulePromise;
}

const defaultSummary: SummaryMetrics = {
  checkpoints_verified: 0,
  checkpoints_total: 0,
  provenance_verified: 0,
  provenance_total: 0,
  attachments_verified: 0,
  attachments_total: 0,
  hash_chain_valid: false,
  signatures_valid: false,
  content_integrity_valid: false
};

const defaultModel: ModelSummary = {
  name: 'Unknown model',
  version: '—',
  kind: '—'
};

const defaultWorkflow: VerificationWorkflow = { steps: [] };

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function coerceStatus(value: unknown): VerificationStatus {
  if (typeof value === 'string') {
    const normalized = value.toLowerCase();
    if (normalized === 'verified') {
      return 'verified';
    }
  }
  return 'failed';
}

function coerceStepStatus(value: unknown): StepStatus {
  if (typeof value === 'string') {
    const normalized = value.toLowerCase();
    if (normalized === 'passed' || normalized === 'failed' || normalized === 'skipped') {
      return normalized;
    }
  }
  return 'skipped';
}

function sanitizeDetail(detail: unknown): StepDetail | null {
  if (!isRecord(detail)) return null;
  const label = typeof detail.label === 'string' ? detail.label : null;
  const value = typeof detail.value === 'string' ? detail.value : null;
  if (!label || value === null) return null;
  return { label, value };
}

function sanitizeStep(step: unknown, index: number): WorkflowStep {
  if (!isRecord(step)) {
    return {
      key: `step-${index}`,
      label: `Step ${index + 1}`,
      status: 'skipped'
    };
  }

  const detailsSource = Array.isArray(step.details) ? step.details : [];
  const details = detailsSource
    .map((detail) => sanitizeDetail(detail) ?? null)
    .filter((detail): detail is StepDetail => detail !== null);

  return {
    key: typeof step.key === 'string' ? step.key : `step-${index}`,
    label: typeof step.label === 'string' ? step.label : `Step ${index + 1}`,
    status: coerceStepStatus(step.status),
    details: details.length ? details : undefined,
    error: typeof step.error === 'string' ? step.error : undefined
  };
}

function sanitizeSummary(summary: unknown): SummaryMetrics {
  if (!isRecord(summary)) {
    return { ...defaultSummary };
  }

  const numberOrZero = (value: unknown) => (typeof value === 'number' && Number.isFinite(value) ? value : 0);

  return {
    checkpoints_verified: numberOrZero(summary.checkpoints_verified),
    checkpoints_total: numberOrZero(summary.checkpoints_total),
    provenance_verified: numberOrZero(summary.provenance_verified),
    provenance_total: numberOrZero(summary.provenance_total),
    attachments_verified: numberOrZero(summary.attachments_verified),
    attachments_total: numberOrZero(summary.attachments_total),
    hash_chain_valid: Boolean(summary.hash_chain_valid),
    signatures_valid: Boolean(summary.signatures_valid),
    content_integrity_valid: Boolean(summary.content_integrity_valid)
  };
}

function sanitizeModel(model: unknown): ModelSummary {
  if (!isRecord(model)) return { ...defaultModel };
  return {
    name: typeof model.name === 'string' && model.name.trim() ? model.name : defaultModel.name,
    version: typeof model.version === 'string' && model.version.trim() ? model.version : defaultModel.version,
    kind: typeof model.kind === 'string' && model.kind.trim() ? model.kind : defaultModel.kind
  };
}

function sanitizeSigner(signer: unknown): SignerSummary | undefined {
  if (!isRecord(signer)) return undefined;
  if (typeof signer.public_key !== 'string' || !signer.public_key.trim()) {
    return undefined;
  }
  return { public_key: signer.public_key };
}

function fromPartialReport(value: unknown): VerificationReport {
  if (!isRecord(value)) {
    return {
      status: 'failed',
      car_id: '',
      run_id: '',
      created_at: '',
      signer: undefined,
      model: { ...defaultModel },
      summary: { ...defaultSummary },
      workflow: { ...defaultWorkflow },
      error: undefined
    };
  }

  const partial = value as Partial<WasmVerificationReport> & Record<string, unknown>;

  const stepsSource = Array.isArray(partial.steps)
    ? partial.steps
    : isRecord(partial.workflow) && Array.isArray((partial.workflow as Record<string, unknown>).steps)
      ? ((partial.workflow as Record<string, unknown>).steps as unknown[])
      : [];
  const workflow: VerificationWorkflow = {
    steps: stepsSource.map((step, index) => sanitizeStep(step, index))
  };

  return {
    status: coerceStatus(partial.status),
    car_id: typeof partial.car_id === 'string' ? partial.car_id : '',
    run_id: typeof partial.run_id === 'string' ? partial.run_id : '',
    created_at: typeof partial.created_at === 'string' ? partial.created_at : '',
    signer: sanitizeSigner(partial.signer),
    model: sanitizeModel(partial.model),
    summary: sanitizeSummary(partial.summary),
    workflow,
    error: typeof partial.error === 'string' ? partial.error : undefined
  };
}

function normalizeResult(value: unknown): VerificationReport {
  if (typeof value === 'string') {
    try {
      const parsed = JSON.parse(value) as unknown;
      return fromPartialReport(parsed);
    } catch {
      return fromPartialReport(undefined);
    }
  }

  return fromPartialReport(value);
}

export async function initVerifier(): Promise<void> {
  const mod = await loadModule();
  if (typeof mod.init_verifier === 'function') {
    await mod.init_verifier();
  }
}

export async function verifyCarBytes(bytes: Uint8Array): Promise<VerificationReport> {
  const mod = await loadModule();
  if (!mod.verify_car_bytes) throw new Error('verify_car_bytes is not exported by the WASM bundle');
  const result = await mod.verify_car_bytes(bytes);
  return normalizeResult(result);
}

export async function verifyCarJson(json: string): Promise<VerificationReport> {
  const mod = await loadModule();
  if (!mod.verify_car_json) throw new Error('verify_car_json is not exported by the WASM bundle');
  const result = await mod.verify_car_json(json);
  return normalizeResult(result);
}
