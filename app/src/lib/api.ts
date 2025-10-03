// In app/src/lib/api.ts
import { invoke } from '@tauri-apps/api/core';
import { interactiveFeatureEnabled } from './featureFlags.js';

export interface Project {
  id: string;
  name: string;
  created_at: string; // Dates are serialized as strings
  pubkey: string;
}

export interface ExecutionStepProofSummary {
  checkpointConfigId: string;
  checkpointType: string;
  orderIndex: number;
  proofMode: RunProofMode;
  epsilon?: number | null;
}

export interface RunExecutionSummary {
  id: string;
  createdAt: string;
  stepProofs: ExecutionStepProofSummary[];
}

export interface RunSummary {
  id: string;
  name: string;
  createdAt: string;
  kind: string;
  epsilon?: number | null;
  hasPersistedCheckpoint: boolean;
  executions: RunExecutionSummary[];
  stepProofs: ExecutionStepProofSummary[];
}

export type RunProofMode = 'exact' | 'concordant';

export type ProofBadgeKind = RunProofMode | 'interactive' | 'unknown';

export interface CheckpointSummary {
  id: string;
  runExecutionId: string;
  timestamp: string;
  kind: string;
  incident?: IncidentSummary | null;
  inputsSha256?: string | null;
  outputsSha256?: string | null;
  semanticDigest?: string | null;
  usageTokens: number;
  promptTokens: number;
  completionTokens: number;
  parentCheckpointId?: string | null;
  turnIndex?: number | null;
  checkpointConfigId?: string | null;
  message?: CheckpointMessage | null;
}

export interface CheckpointDetails {
  id: string;
  runId: string;
  runExecutionId: string;
  timestamp: string;
  kind: string;
  incident?: IncidentSummary | null;
  inputsSha256?: string | null;
  outputsSha256?: string | null;
  semanticDigest?: string | null;
  usageTokens: number;
  promptTokens: number;
  completionTokens: number;
  parentCheckpointId?: string | null;
  turnIndex?: number | null;
  checkpointConfigId?: string | null;
  promptPayload?: string | null;
  outputPayload?: string | null;
  message?: CheckpointMessage | null;
}

export interface IncidentSummary {
  kind: string;
  severity: string;
  details: string;
  relatedCheckpointId?: string | null;
}

export interface CheckpointMessage {
  role: string;
  body: string;
  createdAt: string;
  updatedAt?: string | null;
}

export interface TokenUsage {
  promptTokens: number;
  completionTokens: number;
}

export interface SubmitTurnResult {
  humanCheckpointId: string;
  aiCheckpointId: string;
  aiResponse: string;
  usage: TokenUsage;
}

export type CheckpointReplayMode = 'exact' | 'concordant' | 'interactive';

export interface CheckpointReplayReport {
  checkpointConfigId?: string | null;
  checkpointType?: string | null;
  orderIndex?: number | null;
  mode: CheckpointReplayMode;
  matchStatus: boolean;
  originalDigest: string;
  replayDigest: string;
  errorMessage?: string | null;
  proofMode?: RunProofMode | null;
  semanticOriginalDigest?: string | null;
  semanticReplayDigest?: string | null;
  semanticDistance?: number | null;
  epsilon?: number | null;
  configuredEpsilon?: number | null;
}

export interface ReplayReport {
  runId: string;
  matchStatus: boolean;
  originalDigest: string;
  replayDigest: string;
  errorMessage?: string | null;
  semanticOriginalDigest?: string | null;
  semanticReplayDigest?: string | null;
  semanticDistance?: number | null;
  epsilon?: number | null;
  checkpointReports: CheckpointReplayReport[];
}

export interface CarSampler {
  temp: number;
  topP: number;
  rng: string;
}

export interface CarProcessCheckpointProof {
  id: string;
  parentCheckpointId?: string | null;
  turnIndex?: number | null;
  prevChain: string;
  currChain: string;
  signature: string;
}

export interface CarProcessProof {
  sequentialCheckpoints: CarProcessCheckpointProof[];
}

export interface CarProof {
  matchKind: string;
  epsilon?: number | null;
  distanceMetric?: string | null;
  originalSemanticDigest?: string | null;
  replaySemanticDigest?: string | null;
  process?: CarProcessProof | null;
}

export interface CarPolicyRef {
  hash: string;
  egress: boolean;
  estimator: string;
}

export interface CarBudgets {
  usd: number;
  tokens: number;
  natureCost: number;
}

export interface CarProvenanceClaim {
  claimType: string;
  sha256: string;
}

export interface CarSGradeComponents {
  provenance: number;
  energy: number;
  replay: number;
  consent: number;
  incidents: number;
}

export interface CarSGrade {
  score: number;
  components: CarSGradeComponents;
}

export interface ImportedCarCheckpointSnapshot {
  id: string;
  parentCheckpointId?: string | null;
  turnIndex?: number | null;
  prevChain?: string | null;
  currChain?: string | null;
  signature?: string | null;
}

export interface CarRunInfo {
  kind: string;
  name: string;
  model: string;
  version: string;
  seed: number;
  steps: RunStepConfig[];
  sampler?: CarSampler | null;
}

export interface ImportedCarSnapshot {
  carId: string;
  runId: string;
  createdAt: string;
  run: CarRunInfo;
  proof: CarProof;
  policyRef: CarPolicyRef;
  budgets: CarBudgets;
  provenance: CarProvenanceClaim[];
  checkpoints: ImportedCarCheckpointSnapshot[];
  sgrade: CarSGrade;
  signerPublicKey: string;
}

export interface CarImportResult {
  replayReport: ReplayReport;
  snapshot: ImportedCarSnapshot;
}

export interface ProjectImportSummary {
  project: Project;
  runsImported: number;
  checkpointsImported: number;
  receiptsImported: number;
  incidentsGenerated: number;
}

export interface FileImportPayload {
  fileName?: string;
  bytes?: number[];
  archivePath?: string | null;
  carPath?: string | null;
  [key: string]: unknown;
}

export interface Policy {
  allowNetwork: boolean;
  budgetTokens: number;
  budgetUsd: number;
  budgetNatureCost: number;
}

export interface RunCostEstimates {
  estimatedTokens: number;
  estimatedUsd: number;
  estimatedNatureCost: number;
  budgetTokens: number;
  budgetUsd: number;
  budgetNatureCost: number;
  exceedsTokens: boolean;
  exceedsUsd: boolean;
  exceedsNatureCost: boolean;
}

export interface RunStepConfig {
  id: string;
  runId: string;
  orderIndex: number;
  checkpointType: string;
  stepType: string; // "llm" or "document_ingestion"
  model?: string | null;
  prompt?: string | null;
  tokenBudget: number;
  proofMode: RunProofMode;
  epsilon?: number | null;
  configJson?: string | null;
}

export interface InteractiveCheckpointSession {
  checkpoint: RunStepConfig;
  messages: CheckpointSummary[];
}

export type OpenInteractiveCheckpointSession = (
  runId: string,
  checkpointId: string,
) => Promise<InteractiveCheckpointSession>;

export type SubmitInteractiveCheckpointTurn = (
  runId: string,
  checkpointId: string,
  promptText: string,
) => Promise<SubmitTurnResult>;

export type FinalizeInteractiveCheckpoint = (
  runId: string,
  checkpointId: string,
) => Promise<void>;

export interface DocumentIngestionConfig {
  sourcePath: string;
  format: string; // "pdf", "latex", "docx", "txt"
  privacyStatus: string;
  outputStorage?: string;
}

export interface RunStepRequest {
  stepType?: string; // "llm" or "document_ingestion"
  // LLM fields
  model?: string;
  prompt?: string;
  tokenBudget?: number;
  proofMode?: RunProofMode;
  epsilon?: number | null;
  // Document ingestion fields
  configJson?: string;
  // Common fields
  checkpointType?: string;
  orderIndex?: number;
}

export interface HelloRunSpec {
  projectId: string;
  name: string;
  seed: number;
  dagJson: string;
  tokenBudget: number;
  model: string;
  proofMode?: RunProofMode;
  epsilon?: number | null;
}

export interface CreateRunParams {
  projectId: string;
  name: string;
  proofMode: RunProofMode;
  seed: number;
  tokenBudget: number;
  defaultModel: string;
  epsilon?: number | null;
}

export interface UpdateRunSettingsParams {
  runId: string;
  proofMode: RunProofMode;
  epsilon?: number | null;
}

export async function listLocalModels(): Promise<string[]> {
  return await invoke<string[]>("list_local_models");
}

export async function listProjects(): Promise<Project[]> {
  return await invoke<Project[]>('list_projects');
}

export async function createProject(name: string): Promise<Project> {
  return await invoke<Project>('create_project', { name });
}

export async function renameProject(projectId: string, name: string): Promise<Project> {
  return await invoke<Project>('rename_project', { projectId, name });
}

export async function deleteProject(projectId: string): Promise<void> {
  await invoke('delete_project', { projectId });
}

export async function createRun(params: CreateRunParams): Promise<string> {
  return await invoke<string>('create_run', {
    projectId: params.projectId,
    name: params.name,
    proofMode: params.proofMode,
    seed: params.seed,
    tokenBudget: params.tokenBudget,
    defaultModel: params.defaultModel,
    epsilon: params.epsilon ?? null,
  });
}

export async function listRuns(projectId: string): Promise<RunSummary[]> {
  return await invoke<RunSummary[]>('list_runs', { projectId });
}

export async function renameRun(runId: string, name: string): Promise<void> {
  await invoke('rename_run', { runId, name });
}

export async function deleteRun(runId: string): Promise<void> {
  await invoke('delete_run', { runId });
}

export async function listCheckpoints(
  runExecutionId?: string | null,
): Promise<CheckpointSummary[]> {
  return await invoke<CheckpointSummary[]>('list_checkpoints', {
    args: { runExecutionId: runExecutionId ?? null },
  });
}

export async function getCheckpointDetails(
  checkpointId: string,
): Promise<CheckpointDetails> {
  return await invoke<CheckpointDetails>('get_checkpoint_details', {
    checkpointId,
  });
}

export async function listRunSteps(
  runId: string,
): Promise<RunStepConfig[]> {
  return await invoke<RunStepConfig[]>('list_run_steps', { runId });
}

export async function createRunStep(
  runId: string,
  config: RunStepRequest,
): Promise<RunStepConfig> {
  const normalizedConfig: RunStepRequest = {
    ...config,
    epsilon: config.epsilon ?? null,
  };
  return await invoke<RunStepConfig>('create_run_step', {
    runId,
    config: normalizedConfig,
  });
}

export async function updateRunStep(
  checkpointId: string,
  updates: Partial<RunStepRequest> & { checkpointType?: string },
): Promise<RunStepConfig> {
  const normalizedUpdates = { ...updates } as typeof updates & {
    epsilon?: number | null;
  };
  if ('epsilon' in normalizedUpdates) {
    normalizedUpdates.epsilon = normalizedUpdates.epsilon ?? null;
  }
  return await invoke<RunStepConfig>('update_run_step', {
    checkpointId,
    updates: normalizedUpdates,
  });
}

export async function deleteRunStep(checkpointId: string): Promise<void> {
  await invoke('delete_run_step', { checkpointId });
}

export async function reorderRunSteps(
  runId: string,
  checkpointIds: string[],
): Promise<RunStepConfig[]> {
  return await invoke<RunStepConfig[]>('reorder_run_steps', {
    runId,
    checkpointIds,
  });
}

export async function startRun(runId: string): Promise<RunExecutionSummary> {
  return await invoke<RunExecutionSummary>('start_run', { runId });
}

export async function cloneRun(runId: string): Promise<string> {
  return await invoke<string>('clone_run', { runId });
}

export async function estimateRunCost(runId: string): Promise<RunCostEstimates> {
  return await invoke<RunCostEstimates>('estimate_run_cost', { runId });
}

export const openInteractiveCheckpointSession: OpenInteractiveCheckpointSession | undefined =
  interactiveFeatureEnabled
    ? async (runId: string, checkpointId: string) =>
        await invoke<InteractiveCheckpointSession>('open_interactive_checkpoint_session', {
          runId,
          checkpointId,
        })
    : undefined;

export const submitInteractiveCheckpointTurn: SubmitInteractiveCheckpointTurn | undefined =
  interactiveFeatureEnabled
    ? async (runId: string, checkpointId: string, promptText: string) =>
        await invoke<SubmitTurnResult>('submit_interactive_checkpoint_turn', {
          runId,
          checkpointId,
          promptText,
        })
    : undefined;

export const finalizeInteractiveCheckpoint: FinalizeInteractiveCheckpoint | undefined =
  interactiveFeatureEnabled
    ? async (runId: string, checkpointId: string) => {
        await invoke('finalize_interactive_checkpoint', { runId, checkpointId });
      }
    : undefined;

export async function getPolicy(projectId: string): Promise<Policy> {
  return await invoke<Policy>('get_policy', { projectId });
}

export async function updatePolicy(projectId: string, policy: Policy): Promise<void> {
  await invoke('update_policy', { projectId, policy });
}


export async function emitCar(runId: string, outputPath?: string): Promise<string> {
  return await invoke<string>('emit_car', { runId, outputPath: outputPath ?? null });
}

export async function replayRun(runId: string): Promise<ReplayReport> {
  return await invoke<ReplayReport>('replay_run', { runId });
}

export async function exportProject(projectId: string, outputPath?: string): Promise<string> {
  return await invoke<string>('export_project', { projectId, outputPath: outputPath ?? null });
}

export async function importProject(payload: FileImportPayload): Promise<ProjectImportSummary> {
  return await invoke<ProjectImportSummary>('import_project', { args: payload });
}

export async function importCar(payload: FileImportPayload): Promise<CarImportResult> {
  return await invoke<CarImportResult>('import_car', { args: payload });
}
