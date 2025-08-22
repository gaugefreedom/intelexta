// app/src/lib/api.ts
import { invoke } from '@tauri-apps/api/tauri'
import { open } from '@tauri-apps/api/dialog';

export interface Project {
  id: string;
  name: string;
  created_at: string;
  updated_at: string;
}

export interface Document {
  id: string;
  project_id: string;
  source_path: string;
}

export async function createProject(name: string): Promise<Project> {
  return await invoke('create_project', { name });
}

export async function listProjects(): Promise<Project[]> {
  return await invoke('list_projects');
}

export async function listDocuments(projectId: string): Promise<Document[]> {
  return await invoke('list_documents', { projectId });
}

export async function addDocument(projectId: string): Promise<Document> {
  const selectedPath = await open({
    multiple: false,
    filters: [
      { name: 'Text Files', extensions: ['pdf', 'md', 'txt'] }
    ]
  });

  if (typeof selectedPath === 'string') {
    return await invoke('add_document', { projectId, filePath: selectedPath });
  }
  throw new Error("No file selected");
}