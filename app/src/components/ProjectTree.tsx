import React from "react";
import {
  createProject,
  listProjects,
  listRuns,
  Project,
  RunSummary,
} from "../lib/api";

interface ProjectTreeProps {
  onSelectProject: (projectId: string | null) => void;
  refreshToken?: number;
}

interface ProjectRunState {
  runs: RunSummary[];
  loading: boolean;
  error: string | null;
}

type RunsByProject = Record<string, ProjectRunState>;

function proofBadgeFor(kind: string): { label: string; color: string; title: string } {
  switch (kind) {
    case "exact":
      return { label: "[E]", color: "#9cdcfe", title: "Exact proof mode" };
    case "concordant":
      return { label: "[C]", color: "#c586c0", title: "Concordant proof mode" };
    default:
      return { label: "[I]", color: "#dcdcaa", title: "Indeterminate proof mode" };
  }
}

export default function ProjectTree({
  onSelectProject,
  refreshToken = 0,
}: ProjectTreeProps) {
  const [projects, setProjects] = React.useState<Project[]>([]);
  const [loadingProjects, setLoadingProjects] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const [selectedProjectId, setSelectedProjectId] = React.useState<string | null>(null);
  const [expandedProjects, setExpandedProjects] = React.useState<Set<string>>(new Set());
  const [runsByProject, setRunsByProject] = React.useState<RunsByProject>({});

  const fetchProjects = React.useCallback(async () => {
    setLoadingProjects(true);
    setError(null);
    try {
      const projectList = await listProjects();
      setProjects(projectList);
      setSelectedProjectId((current) => {
        if (!current) {
          return current;
        }
        if (projectList.some((p) => p.id === current)) {
          return current;
        }
        onSelectProject(null);
        return null;
      });
      setExpandedProjects((prev) => {
        if (prev.size === 0) {
          return prev;
        }
        const next = new Set<string>();
        for (const project of projectList) {
          if (prev.has(project.id)) {
            next.add(project.id);
          }
        }
        return next;
      });
      setRunsByProject((prev) => {
        if (Object.keys(prev).length === 0) {
          return prev;
        }
        const next: RunsByProject = {};
        for (const project of projectList) {
          if (prev[project.id]) {
            next[project.id] = prev[project.id];
          }
        }
        return next;
      });
    } catch (err) {
      console.error("Error fetching projects:", err);
      setError("Could not load projects. Is the backend running?");
      setProjects([]);
      setSelectedProjectId((current) => {
        if (current !== null) {
          onSelectProject(null);
        }
        return null;
      });
      setExpandedProjects(new Set());
      setRunsByProject({});
    } finally {
      setLoadingProjects(false);
    }
  }, [onSelectProject]);

  React.useEffect(() => {
    fetchProjects();
  }, [fetchProjects]);

  const loadRunsForProject = React.useCallback((projectId: string) => {
    setRunsByProject((prev) => ({
      ...prev,
      [projectId]: {
        runs: prev[projectId]?.runs ?? [],
        loading: true,
        error: null,
      },
    }));
    listRuns(projectId)
      .then((runList) => {
        setRunsByProject((prev) => ({
          ...prev,
          [projectId]: {
            runs: runList,
            loading: false,
            error: null,
          },
        }));
      })
      .catch((err) => {
        console.error("Failed to load runs", err);
        setRunsByProject((prev) => ({
          ...prev,
          [projectId]: {
            runs: prev[projectId]?.runs ?? [],
            loading: false,
            error: "Could not load runs for this project.",
          },
        }));
      });
  }, []);

  const handleProjectSelect = React.useCallback(
    (projectId: string) => {
      setSelectedProjectId(projectId);
      onSelectProject(projectId);
      setExpandedProjects((prev) => {
        if (prev.has(projectId)) {
          return prev;
        }
        const next = new Set(prev);
        next.add(projectId);
        loadRunsForProject(projectId);
        return next;
      });
    },
    [loadRunsForProject, onSelectProject],
  );

  const handleProjectToggle = React.useCallback(
    (projectId: string) => {
      setExpandedProjects((prev) => {
        const next = new Set(prev);
        if (next.has(projectId)) {
          next.delete(projectId);
        } else {
          next.add(projectId);
          loadRunsForProject(projectId);
        }
        return next;
      });
    },
    [loadRunsForProject],
  );

  const handleNewProject = React.useCallback(async () => {
    const projectName = prompt("Enter new project name:");
    if (!projectName || !projectName.trim()) {
      return;
    }
    try {
      const project = await createProject(projectName.trim());
      await fetchProjects();
      setSelectedProjectId(project.id);
      onSelectProject(project.id);
      setExpandedProjects((prev) => {
        const next = new Set(prev);
        next.add(project.id);
        return next;
      });
      setRunsByProject((prev) => ({
        ...prev,
        [project.id]: { runs: [], loading: false, error: null },
      }));
    } catch (err) {
      console.error("Error creating project:", err);
      alert(`Error creating project: ${err instanceof Error ? err.message : String(err)}`);
    }
  }, [fetchProjects, onSelectProject]);

  React.useEffect(() => {
    if (expandedProjects.size === 0) {
      return;
    }
    expandedProjects.forEach((projectId) => {
      loadRunsForProject(projectId);
    });
  }, [refreshToken, expandedProjects, loadRunsForProject]);

  return (
    <div>
      <h2>Projects</h2>
      <button onClick={handleNewProject} style={{ marginBottom: "8px" }}>
        + New Project
      </button>
      {loadingProjects && <div>Loading projects…</div>}
      {error && <div style={{ color: "#f48771", marginTop: "8px" }}>{error}</div>}
      {!loadingProjects && projects.length === 0 ? (
        <p style={{ fontSize: "0.9rem", color: "#808080" }}>No projects yet. Create one to get started.</p>
      ) : null}
      <ul style={{ listStyle: "none", padding: 0, margin: 0, display: "flex", flexDirection: "column", gap: "4px" }}>
        {projects.map((project) => {
          const isExpanded = expandedProjects.has(project.id);
          const runState = runsByProject[project.id];
          const isSelected = selectedProjectId === project.id;
          return (
            <li key={project.id}>
              <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                <button
                  type="button"
                  onClick={() => handleProjectToggle(project.id)}
                  style={{
                    background: "none",
                    border: "none",
                    color: "#d4d4d4",
                    cursor: "pointer",
                    fontSize: "1rem",
                    lineHeight: 1,
                    padding: 0,
                    width: "18px",
                  }}
                  aria-label={isExpanded ? "Collapse project" : "Expand project"}
                >
                  {isExpanded ? "▾" : "▸"}
                </button>
                <button
                  type="button"
                  onClick={() => handleProjectSelect(project.id)}
                  style={{
                    flex: 1,
                    background: "none",
                    border: "none",
                    color: isSelected ? "#ffffff" : "#d4d4d4",
                    cursor: "pointer",
                    fontWeight: isSelected ? 600 : 500,
                    textAlign: "left",
                    padding: "2px 0",
                  }}
                >
                  {project.name}
                </button>
              </div>
              {isExpanded && (
                <div style={{ marginLeft: "24px", marginTop: "4px" }}>
                  {runState?.loading ? (
                    <div style={{ fontSize: "0.85rem", color: "#9cdcfe" }}>Loading runs…</div>
                  ) : runState?.error ? (
                    <div style={{ fontSize: "0.85rem", color: "#f48771" }}>{runState.error}</div>
                  ) : runState && runState.runs.length > 0 ? (
                    <ul style={{ listStyle: "none", padding: 0, margin: 0, display: "flex", flexDirection: "column", gap: "2px" }}>
                      {runState.runs.map((run) => {
                        const badge = proofBadgeFor(run.kind);
                        return (
                          <li key={run.id} style={{ display: "flex", alignItems: "center", gap: "6px", fontSize: "0.85rem" }}>
                            <span
                              title={badge.title}
                              style={{
                                border: `1px solid ${badge.color}`,
                                color: badge.color,
                                borderRadius: "999px",
                                fontSize: "0.7rem",
                                fontWeight: 700,
                                letterSpacing: "0.08em",
                                padding: "1px 6px",
                              }}
                            >
                              {badge.label}
                            </span>
                            <span>{run.name}</span>
                          </li>
                        );
                      })}
                    </ul>
                  ) : (
                    <div style={{ fontSize: "0.8rem", color: "#808080" }}>No runs yet.</div>
                  )}
                </div>
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
}
