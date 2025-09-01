// In app/src/App.tsx
import React from 'react';
import ProjectTree from './components/ProjectTree';
import ContextPanel from './components/ContextPanel';
import EditorPanel from './components/EditorPanel';
import InspectorPanel from './components/InspectorPanel';

export default function App() {
  const [selectedProject, setSelectedProject] = React.useState<string | null>(null);

  return (
    <main style={{ display: 'flex', height: '100vh', fontFamily: 'sans-serif', background: '#1e1e1e', color: '#d4d4d4' }}>
      <div style={{ width: '250px', borderRight: '1px solid #333', padding: '8px' }}>
        <ProjectTree onSelectProject={setSelectedProject} />
      </div>
      <div style={{ flex: 1, display: 'flex' }}>
        <div style={{ width: '300px', borderRight: '1px solid #333', padding: '8px' }}>
          {selectedProject ? <ContextPanel projectId={selectedProject} /> : <div>Select a project</div>}
        </div>
        <div style={{ flex: 1, padding: '8px' }}>
          {selectedProject ? <EditorPanel projectId={selectedProject} /> : null}
        </div>
        <div style={{ width: '350px', borderLeft: '1px solid #333', padding: '8px' }}>
          {selectedProject ? <InspectorPanel projectId={selectedProject} /> : null}
        </div>
      </div>
    </main>
  );
}