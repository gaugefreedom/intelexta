/**
 * TypeScript types for CAR (Content-Addressable Receipt) v0.3 structure
 * These match the actual CAR JSON format emitted by IntelexTA
 */

export interface Car {
  id: string;
  run_id: string;
  created_at: string;
  run: RunInfo;
  proof: Proof;
  policy_ref: PolicyRef;
  budgets: Budgets;
  provenance: ProvenanceClaim[];
  checkpoints: string[];
  sgrade: SGrade;
  signer_public_key: string;
  signatures: string[];
}

export interface RunInfo {
  kind: string; // 'exact' | 'concordant' | 'interactive'
  name: string;
  model: string;
  version: string;
  seed: number;
  steps: RunStep[];
  sampler?: Sampler;
}

export interface RunStep {
  id: string;
  runId: string;
  orderIndex: number;
  checkpointType: string;
  stepType?: string;
  model?: string | null;
  prompt?: string | null;
  tokenBudget: number;
  proofMode: string; // 'exact' | 'concordant'
  epsilon?: number;
  configJson?: string | null;
}

export interface Sampler {
  temp: number;
  top_p: number;
  rng: string;
}

export interface Proof {
  match_kind: string; // 'exact' | 'semantic' | 'process'
  epsilon?: number;
  distance_metric?: string;
  original_semantic_digest?: string;
  replay_semantic_digest?: string;
  process?: ProcessProof;
}

export interface ProcessProof {
  sequential_checkpoints: ProcessCheckpointProof[];
}

export interface ProcessCheckpointProof {
  id: string;
  parent_checkpoint_id?: string;
  turn_index?: number;
  prev_chain: string;
  curr_chain: string;
  signature: string;
  run_id: string;
  kind: string;
  timestamp: string;
  inputs_sha256?: string;
  outputs_sha256?: string;
  usage_tokens: number;
  prompt_tokens: number;
  completion_tokens: number;
}

export interface PolicyRef {
  hash: string;
  egress: boolean;
  estimator: string;
  model_catalog_hash?: string;
  model_catalog_version?: string;
}

export interface Budgets {
  usd: number;
  tokens: number;
  nature_cost: number;
}

export interface ProvenanceClaim {
  claim_type: string; // 'input' | 'output' | 'config'
  sha256: string;
}

export interface SGrade {
  score: number;
  components: SGradeComponents;
}

export interface SGradeComponents {
  provenance: number;
  energy: number;
  replay: number;
  consent: number;
  incidents: number;
}

/**
 * Represents a text or binary attachment extracted from a CAR bundle
 */
export interface AttachmentPreview {
  fileName: string;
  size: number;
  claimType?: string; // 'config' | 'input' | 'output' | other
  hashHex?: string; // hex hash without "sha256:" prefix
  kind: 'text' | 'binary';
  preview?: string; // truncated text content
  fullText?: string; // full text content (loaded on demand)
}
