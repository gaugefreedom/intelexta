// In app/src/components/ProjectTree.tsx
import React, { useState, useEffect } from 'react';
import { invoke } from "@tauri-apps/api/core";

// Define a type for our project object, matching the Rust struct
interface Project {
  id: string;
  name: string;
  created_at: string;
  pubkey: string;
}

interface ProjectTreeProps {
  onSelectProject: (projectId: string | null) => void;
}

export default function ProjectTree({ onSelectProject }: ProjectTreeProps) {
  const [projects, setProjects] = useState<Project[]>([]);
  const [error, setError] = useState<string | null>(null);

  // Function to fetch projects from the backend
  const fetchProjects = async () => {
    try {
      const projectList = await invoke<Project[]>('list_projects');
      setProjects(projectList);
      setError(null);
    } catch (err) {
      console.error('Error fetching projects:', err);
      setError('Could not load projects. Is the backend running?');
    }
  };

  // Fetch projects when the component first loads
  useEffect(() => {
    fetchProjects();
  }, []);

  const handleNewProject = async () => {
    const projectName = prompt('Enter new project name:');
    if (projectName && projectName.trim()) {
      try {
        // This is the call that is currently failing
        await invoke('create_project', { name: projectName.trim() });
        // If it succeeds, refresh the project list
        await fetchProjects();
      } catch (err) {
        console.error('Error creating project:', err);
        // The detailed error from Rust will be in 'err'
        alert(`Error creating project: ${err}`);
      }
    }
  };

  return (
    <div>
      <h2>Projects</h2>
      <button onClick={handleNewProject}>+ New Project</button>
      {error && <div style={{ color: 'red', marginTop: '10px' }}>{error}</div>}
      <ul style={{ listStyle: 'none', padding: 0, marginTop: '10px' }}>
        {projects.map((p) => (
          <li key={p.id} onClick={() => onSelectProject(p.id)} style={{ cursor: 'pointer', padding: '4px', background: 'transparent' }}>
            {p.name}
          </li>
        ))}
      </ul>
    </div>
  );
}