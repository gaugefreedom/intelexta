// In app/src/lib/api.ts
import { invoke } from '@tauri-apps/api/core';

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

export interface CheckpointSummary {
  id: string;
  timestamp: string;
  kind: string;
  incident?: IncidentSummary | null;
  inputsSha256?: string | null;
  outputsSha256?: string | null;
  semanticDigest?: string | null;
  usageTokens: number;
}

export interface IncidentSummary {
  kind: string;
  severity: string;
  details: string;
  relatedCheckpointId?: string | null;
}

export interface ReplayReport {
  runId: string;
  matchStatus: boolean;
  originalDigest: string;
  replayDigest: string;
  errorMessage?: string | null;
}

export interface Policy {
  allowNetwork: boolean;
  budgetTokens: number;
  budgetUsd: number;
  budgetGCo2e: number;
}

export interface HelloRunSpec {
  projectId: string;
  name: string;
  seed: number;
  dagJson: string;
  tokenBudget: number;
}

export async function listProjects(): Promise<Project[]> {
  return await invoke<Project[]>('list_projects');
}

export async function createProject(name: string): Promise<Project> {
  return await invoke<Project>('create_project', { name });
}

export async function listRuns(projectId: string): Promise<RunSummary[]> {
  return await invoke<RunSummary[]>('list_runs', { projectId });
}

export async function listCheckpoints(runId: string): Promise<CheckpointSummary[]> {
  return await invoke<CheckpointSummary[]>('list_checkpoints', { runId });
}

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
