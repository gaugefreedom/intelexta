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
  created_at: string;
  kind: string;
}

export interface CheckpointSummary {
  id: string;
  timestamp: string;
  kind: string;
  inputs_sha256?: string | null;
  outputs_sha256?: string | null;
  usage_tokens: number;
}

export interface Policy {
  allowNetwork: boolean;
  budgetTokens: number;
  budgetUsd: number;
  budgetGCo2e: number;
}

export async function listProjects(): Promise<Project[]> {
  return await invoke<Project[]>('list_projects');
}

export async function createProject(name: string): Promise<Project> {
  return await invoke<Project>('create_project', { name });
}

export async function listRuns(projectId: string): Promise<RunSummary[]> {
  return await invoke<RunSummary[]>('list_runs', { projectId, project_id: projectId });
}

export async function listCheckpoints(runId: string): Promise<CheckpointSummary[]> {
  return await invoke<CheckpointSummary[]>('list_checkpoints', { runId, run_id: runId });
}

export async function getPolicy(projectId: string): Promise<Policy> {
  return await invoke<Policy>('get_policy', { projectId, project_id: projectId });
}

export async function updatePolicy(projectId: string, policy: Policy): Promise<void> {
  await invoke('update_policy', { projectId, project_id: projectId, policy });
}

export async function emitCar(runId: string): Promise<string> {
  return await invoke<string>('emit_car', { runId, run_id: runId });
}
