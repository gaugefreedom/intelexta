// In app/src/App.tsx
import React from 'react';
import ProjectTree from './components/ProjectTree';
import ContextPanel from './components/ContextPanel';
import EditorPanel from './components/EditorPanel';
import InspectorPanel from './components/InspectorPanel';

export default function App() {
  const [selectedProject, setSelectedProject] = React.useState<string | null>(null);
  const [selectedRunId, setSelectedRunId] = React.useState<string | null>(null);
  const [runsRefreshToken, setRunsRefreshToken] = React.useState(0);

  const requestRunsRefresh = React.useCallback(() => {
    setRunsRefreshToken((token) => token + 1);
  }, []);

  const handleProjectSelect = React.useCallback((projectId: string | null) => {
    setSelectedProject(projectId);
    setSelectedRunId(null);
  }, []);

  const handleRunExecuted = React.useCallback((runId: string) => {
    requestRunsRefresh();
    setSelectedRunId(runId);
  }, [requestRunsRefresh]);

  const handleRunSelection = React.useCallback((runId: string | null) => {
    setSelectedRunId(runId);
  }, []);

  return (
    <main style={{ display: 'flex', height: '100vh', fontFamily: 'sans-serif', background: '#1e1e1e', color: '#d4d4d4' }}>
      <div style={{ width: '250px', borderRight: '1px solid #333', padding: '8px' }}>
        <ProjectTree onSelectProject={handleProjectSelect} refreshToken={runsRefreshToken} />
      </div>
      <div style={{ flex: 1, display: 'flex' }}>
        <div style={{ width: '300px', borderRight: '1px solid #333', padding: '8px' }}>
          {selectedProject ? <ContextPanel projectId={selectedProject} /> : <div>Select a project</div>}
        </div>
        <div style={{ flex: 1, padding: '8px' }}>
          {selectedProject ? (
            <EditorPanel
              projectId={selectedProject}
              selectedRunId={selectedRunId}
              onSelectRun={handleRunSelection}
              refreshToken={runsRefreshToken}
              onRunExecuted={handleRunExecuted}
              onRunsMutated={requestRunsRefresh}
            />
          ) : null}
        </div>
        <div style={{ width: '350px', borderLeft: '1px solid #333', padding: '8px' }}>
          {selectedProject ? (
            <InspectorPanel
              projectId={selectedProject}
              refreshToken={runsRefreshToken}
              selectedRunId={selectedRunId}
              onSelectRun={handleRunSelection}
            />
          ) : null}
        </div>
      </div>
    </main>
  );
}