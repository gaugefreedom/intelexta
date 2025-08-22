import React, { useState, useEffect } from 'react';
import { Project, Document, listDocuments, addDocument } from '../lib/api';

interface ProjectPanelProps {
  project: Project;
}

export function ProjectPanel({ project }: ProjectPanelProps) {
  const [documents, setDocuments] = useState<Document[]>([]);

  const loadDocuments = async () => {
    setDocuments(await listDocuments(project.id));
  };

  useEffect(() => {
    loadDocuments();
  }, [project]);

  const handleAddFile = async () => {
    try {
      await addDocument(project.id);
      await loadDocuments(); // Refresh the list
    } catch (error) {
      console.error("Failed to add document:", error);
    }
  };

  return (
    <div>
      <h1>{project.name}</h1>
      <button onClick={handleAddFile}>Add File</button>
      <h3>Documents</h3>
      <ul>
        {documents.map(doc => (
          <li key={doc.id}>{doc.source_path.split(/[\\/]/).pop()}</li>
        ))}
      </ul>
      <p>Next steps: Context Panel, Chat Panel...</p>
    </div>
  );
}
