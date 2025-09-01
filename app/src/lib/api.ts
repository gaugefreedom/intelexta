// In app/src/lib/api.ts
import { invoke } from '@tauri-apps/api/tauri';

export interface Project {
  id: string;
  name: string;
  created_at: string; // Dates are serialized as strings
  pubkey: string;
}

export async function listProjects(): Promise<Project[]> {
  return await invoke('list_projects');
}

export async function createProject(name: string): Promise<Project> {
  return await invoke('create_project', { name });
}