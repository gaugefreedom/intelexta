// In app/src/components/InspectorPanel.tsx

import React from "react";
import {
  listCheckpoints,
  listRuns,
  CheckpointSummary,
  IncidentSummary,
  RunSummary,
  emitCar,
} from "../lib/api";

function formatIncidentMessage(incident?: IncidentSummary | null): string {
  if (!incident) {
    return "Policy incident reported";
  }
  switch (incident.kind) {
    case "budget_exceeded":
      return `Budget exceeded: ${incident.details}`;
    default: {
      const readableKind = incident.kind
        .replace(/_/g, " ")
        .replace(/(^|\s)\S/g, (match) => match.toUpperCase());
      return `${readableKind}: ${incident.details}`;
    }
  }
}

function incidentSeverityColor(incident?: IncidentSummary | null): string {
  if (!incident) {
    return "#f48771";
  }
  switch (incident.severity) {
    case "warn":
      return "#dcdcaa";
    case "info":
      return "#9cdcfe";
    case "error":
    default:
      return "#f48771";
  }
}

export default function InspectorPanel({
  projectId,
  refreshToken,
}: {
  projectId: string;
  refreshToken: number;
}) {
  const [runs, setRuns] = React.useState<RunSummary[]>([]);
  const [selectedRunId, setSelectedRunId] = React.useState<string | null>(null);
  const [checkpoints, setCheckpoints] = React.useState<CheckpointSummary[]>([]);
  const [loadingRuns, setLoadingRuns] = React.useState<boolean>(false);
  const [loadingCheckpoints, setLoadingCheckpoints] = React.useState<boolean>(false);
  const [runsError, setRunsError] = React.useState<string | null>(null);
  const [checkpointError, setCheckpointError] = React.useState<string | null>(null);
  
  // MERGED STATE: Using the more explicit state management from the `emit_car` branch.
  const [emittingCar, setEmittingCar] = React.useState<boolean>(false);
  const [emitSuccess, setEmitSuccess] = React.useState<string | null>(null);
  const [emitError, setEmitError] = React.useState<string | null>(null);

  React.useEffect(() => {
    if (!projectId) return;
    let cancelled = false;
    setLoadingRuns(true);
    setRunsError(null);
    listRuns(projectId)
      .then((runList) => {
        if (cancelled) return;
        setRuns(runList);
        if (!selectedRunId || !runList.find((r) => r.id === selectedRunId)) {
          setSelectedRunId(runList.length > 0 ? runList[0].id : null);
        }
      })
      .catch((err) => {
        if (cancelled) return;
        console.error("Failed to load runs", err);
        setRunsError("Could not load runs for this project.");
        setRuns([]);
        setSelectedRunId(null);
      })
      .finally(() => {
        if (!cancelled) {
          setLoadingRuns(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [projectId, refreshToken]);

  React.useEffect(() => {
    if (!selectedRunId) {
      setCheckpoints([]);
      setCheckpointError(null);
      return;
    }
    let cancelled = false;
    setLoadingCheckpoints(true);
    setCheckpointError(null);
    listCheckpoints(selectedRunId)
      .then((items) => {
        if (!cancelled) {
          setCheckpoints(items);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          console.error("Failed to load checkpoints", err);
          setCheckpointError("Could not load checkpoints for the selected run.");
          setCheckpoints([]);
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoadingCheckpoints(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [selectedRunId, refreshToken]);

  // MERGED LOGIC: Using the useEffect and useCallback from the `emit_car` branch.
  React.useEffect(() => {
    setEmitSuccess(null);
    setEmitError(null);
  }, [selectedRunId]);

  const handleEmitCar = React.useCallback(() => {
    if (!selectedRunId) {
      return;
    }
    setEmittingCar(true);
    setEmitSuccess(null);
    setEmitError(null);
    emitCar(selectedRunId)
      .then((path) => {
        setEmitSuccess(`Receipt saved to ${path}`);
      })
      .catch((err) => {
        console.error("Failed to emit CAR", err);
        const message = err instanceof Error ? err.message : String(err);
        setEmitError(`Failed to emit CAR: ${message}`);
      })
      .finally(() => {
        setEmittingCar(false);
      });
  }, [selectedRunId]);

  return (
    <div>
      <h2>Inspector</h2>
      <div style={{ fontSize: "0.85rem", marginBottom: "0.5rem", color: "#9cdcfe" }}>
        Project: {projectId}
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: "8px", marginBottom: "12px" }}>
        <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
          Run
          <select
            value={selectedRunId ?? ""}
            onChange={(event) => setSelectedRunId(event.target.value || null)}
            disabled={loadingRuns || runs.length === 0}
          >
            <option value="" disabled>
              {loadingRuns ? "Loading…" : "Select a run"}
            </option>
            {runs.map((run) => (
              <option key={run.id} value={run.id}>
                {run.name} · {new Date(run.createdAt).toLocaleString()}
              </option>
            ))}
          </select>
        </label>
        {runsError && <span style={{ color: "#f48771" }}>{runsError}</span>}
        
        {/* MERGED JSX: Using the more detailed button and feedback from the `emit_car` branch. */}
        <button
          type="button"
          onClick={handleEmitCar}
          disabled={!selectedRunId || emittingCar}
          style={{ alignSelf: "flex-start" }}
        >
          {emittingCar ? "Emitting…" : "Emit CAR"}
        </button>
        {emitSuccess && (
          <span style={{ fontSize: '0.8rem', color: "#a5d6a7" }}>{emitSuccess}</span>
        )}
        {emitError && <span style={{ fontSize: '0.8rem', color: "#f48771" }}>{emitError}</span>}
      </div>

      {loadingCheckpoints ? (
        <p>Loading checkpoints…</p>
      ) : selectedRunId ? (
        checkpoints.length > 0 ? (
          <div style={{ maxHeight: "60vh", overflowY: "auto" }}>
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "0.85rem" }}>
              <thead>
                <tr style={{ borderBottom: "1px solid #333" }}>
                  <th style={{ textAlign: "left", padding: "4px" }}>Timestamp</th>
                  <th style={{ textAlign: "left", padding: "4px" }}>Kind</th>
                  <th style={{ textAlign: "left", padding: "4px" }}>Inputs SHA</th>
                  <th style={{ textAlign: "left", padding: "4px" }}>Outputs SHA</th>
                  <th style={{ textAlign: "right", padding: "4px" }}>Usage</th>
                </tr>
              </thead>
              <tbody>
                {checkpoints.map((ckpt) => {
                  const isIncident = ckpt.kind === "Incident";
                  const message = isIncident ? formatIncidentMessage(ckpt.incident) : null;
                  const severityColor = incidentSeverityColor(ckpt.incident);
                  return (
                    <tr
                      key={ckpt.id}
                      style={{
                        borderBottom: "1px solid #222",
                        backgroundColor: isIncident ? "#2d1616" : undefined,
                      }}
                    >
                      <td style={{ padding: "4px", verticalAlign: "top" }}>
                        {new Date(ckpt.timestamp).toLocaleString()}
                      </td>
                      <td style={{ padding: "4px" }}>
                        <div style={{ fontWeight: 600 }}>{ckpt.kind}</div>
                        {isIncident && (
                          <div style={{ marginTop: "4px", display: "flex", flexDirection: "column", gap: "4px" }}>
                            {ckpt.incident?.severity && (
                              <span
                                style={{
                                  alignSelf: "flex-start",
                                  fontSize: "0.7rem",
                                  letterSpacing: "0.08em",
                                  fontWeight: 700,
                                  padding: "2px 6px",
                                  borderRadius: "999px",
                                  border: `1px solid ${severityColor}`,
                                  color: severityColor,
                                  textTransform: "uppercase",
                                }}
                              >
                                {ckpt.incident.severity.toUpperCase()}
                              </span>
                            )}
                            <span style={{ fontWeight: 700, color: severityColor }}>{message}</span>
                          </div>
                        )}
                      </td>
                      <td style={{ padding: "4px", fontFamily: "monospace", wordBreak: "break-all" }}>
                        {ckpt.inputsSha256 ?? "—"}
                      </td>
                      <td style={{ padding: "4px", fontFamily: "monospace", wordBreak: "break-all" }}>
                        {ckpt.outputsSha256 ?? "—"}
                      </td>
                      <td style={{ padding: "4px", textAlign: "right", verticalAlign: "top" }}>
                        {ckpt.usageTokens}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        ) : (
          <p>No checkpoints recorded for this run yet.</p>
        )
      ) : (
        <p>Select a run to inspect its checkpoints.</p>
      )}
      {checkpointError && <div style={{ color: "#f48771", marginTop: "8px" }}>{checkpointError}</div>}
    </div>
  );
}