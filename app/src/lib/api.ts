// In app/src/lib/api.ts
import { invoke } from '@tauri-apps/api/core';
import { interactiveFeatureEnabled } from './featureFlags';

export interface Project {
  id: string;
  name: string;
  created_at: string; // Dates are serialized as strings
  pubkey: string;
}

export interface RunSummary {
  id: string;
  name: string;
  createdAt: string;
  kind: string;
}

export type RunProofMode = 'exact' | 'concordant';

export interface CheckpointSummary {
  id: string;
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
  semanticOriginalDigest?: string | null;
  semanticReplayDigest?: string | null;
  semanticDistance?: number | null;
  epsilon?: number | null;
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

export interface ProjectImportSummary {
  project: Project;
  runsImported: number;
  checkpointsImported: number;
  receiptsImported: number;
  incidentsGenerated: number;
}

export interface FileImportPayload {
  fileName: string;
  bytes: number[];
}

export interface Policy {
  allowNetwork: boolean;
  budgetTokens: number;
  budgetUsd: number;
  budgetGCo2e: number;
}

export interface RunCostEstimates {
  estimatedTokens: number;
  estimatedUsd: number;
  estimatedGCo2e: number;
  budgetTokens: number;
  budgetUsd: number;
  budgetGCo2e: number;
  exceedsTokens: boolean;
  exceedsUsd: boolean;
  exceedsGCo2e: boolean;
}

export interface RunCheckpointConfig {
  id: string;
  runId: string;
  orderIndex: number;
  checkpointType: string;
  model: string;
  prompt: string;
  tokenBudget: number;
  proofMode: RunProofMode;
}

export interface InteractiveCheckpointSession {
  checkpoint: RunCheckpointConfig;
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

export interface CheckpointConfigRequest {
  model: string;
  prompt: string;
  tokenBudget: number;
  checkpointType?: string;
  orderIndex?: number;
  proofMode?: RunProofMode;
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

export async function listLocalModels(): Promise<string[]> {
  return await invoke<string[]>("list_local_models");
}

export async function listProjects(): Promise<Project[]> {
  return await invoke<Project[]>('list_projects');
}

export async function createProject(name: string): Promise<Project> {
  return await invoke<Project>('create_project', { name });
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

export async function listCheckpoints(runId: string): Promise<CheckpointSummary[]> {
  return await invoke<CheckpointSummary[]>('list_checkpoints', { runId });
}

export async function getCheckpointDetails(
  checkpointId: string,
): Promise<CheckpointDetails> {
  return await invoke<CheckpointDetails>('get_checkpoint_details', {
    checkpointId,
  });
}

export async function listRunCheckpointConfigs(
  runId: string,
): Promise<RunCheckpointConfig[]> {
  return await invoke<RunCheckpointConfig[]>('list_run_checkpoint_configs', { runId });
}

export async function createCheckpointConfig(
  runId: string,
  config: CheckpointConfigRequest,
): Promise<RunCheckpointConfig> {
  return await invoke<RunCheckpointConfig>('create_checkpoint_config', {
    runId,
    config,
  });
}

export async function updateCheckpointConfig(
  checkpointId: string,
  updates: Partial<CheckpointConfigRequest> & { checkpointType?: string },
): Promise<RunCheckpointConfig> {
  return await invoke<RunCheckpointConfig>('update_checkpoint_config', {
    checkpointId,
    updates,
  });
}

export async function deleteCheckpointConfig(checkpointId: string): Promise<void> {
  await invoke('delete_checkpoint_config', { checkpointId });
}

export async function reorderCheckpointConfigs(
  runId: string,
  checkpointIds: string[],
): Promise<RunCheckpointConfig[]> {
  return await invoke<RunCheckpointConfig[]>('reorder_checkpoint_configs', {
    runId,
    checkpointIds,
  });
}

export async function startRun(runId: string): Promise<void> {
  await invoke('start_run', { runId });
}

export async function reopenRun(runId: string): Promise<void> {
  await invoke('reopen_run', { runId });
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

export async function startHelloRun(spec: HelloRunSpec): Promise<string> {
  return await invoke<string>('start_hello_run', { spec });
}

export async function emitCar(runId: string): Promise<string> {
  return await invoke<string>('emit_car', { runId });
}

export async function replayRun(runId: string): Promise<ReplayReport> {
  return await invoke<ReplayReport>('replay_run', { runId });
}

export async function exportProject(projectId: string): Promise<string> {
  return await invoke<string>('export_project', { projectId });
}

export async function importProject(payload: FileImportPayload): Promise<ProjectImportSummary> {
  return await invoke<ProjectImportSummary>('import_project', payload);
}

export async function importCar(payload: FileImportPayload): Promise<ReplayReport> {
  return await invoke<ReplayReport>('import_car', payload);
}
