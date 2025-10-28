export type VerificationStatus = 'verified' | 'failed';

export type StepStatus = 'passed' | 'failed' | 'skipped';

export interface SignerSummary {
  public_key: string;
}

export interface ModelSummary {
  name: string;
  version: string;
  kind: string;
}

export interface SummaryMetrics {
  checkpoints_verified: number;
  checkpoints_total: number;
  provenance_verified: number;
  provenance_total: number;
  attachments_verified: number;
  attachments_total: number;
  hash_chain_valid: boolean;
  signatures_valid: boolean;
  content_integrity_valid: boolean;
}

export interface StepDetail {
  label: string;
  value: string;
}

export interface WorkflowStep {
  key: string;
  label: string;
  status: StepStatus;
  details?: StepDetail[];
  error?: string;
}

export interface WasmVerificationReport {
  status: VerificationStatus;
  car_id: string;
  run_id: string;
  created_at: string;
  signer?: SignerSummary;
  model: ModelSummary;
  steps: WorkflowStep[];
  summary: SummaryMetrics;
  error?: string;
}

export interface VerificationWorkflow {
  steps: WorkflowStep[];
}

export interface VerificationReport extends Omit<WasmVerificationReport, 'steps'> {
  workflow: VerificationWorkflow;
}
