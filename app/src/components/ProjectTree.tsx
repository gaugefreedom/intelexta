// In app/src/components/ProjectTree.tsx
import React, { useEffect, useState } from 'react';
import { createProject, listProjects, Project } from '../lib/api';

interface Props {
  onSelectProject: (projectId: string) => void;
}

export default function ProjectTree({ onSelectProject }: Props) {
  const [projects, setProjects] = useState<Project[]>([]);

  const refreshProjects = async () => {
    try {
      const fetchedProjects = await listProjects();
      setProjects(fetchedProjects);
    } catch (e) {
      console.error("Failed to fetch projects:", e);
      alert("Error fetching projects. Is the backend running?");
    }
  };

  useEffect(() => {
    refreshProjects();
  }, []);

  const handleCreateProject = async () => {
    const name = prompt("Enter new project name:");
    if (name && name.trim()) {
      try {
        await createProject(name.trim());
        refreshProjects();
      } catch (e) {
        console.error("Failed to create project:", e);
        alert("Error creating project.");
      }
    }
  };

  return (
    <div>
      <h2>Projects</h2>
      <button onClick={handleCreateProject} style={{ width: '100%', marginBottom: '12px' }}>
        + New Project
      </button>
      <ul style={{ listStyle: 'none', padding: 0 }}>
        {projects.map(p => (
          <li key={p.id} onClick={() => onSelectProject(p.id)} style={{ padding: '4px', cursor: 'pointer' }}>
            {p.name}
          </li>
        ))}
      </ul>
    </div>
  );
}