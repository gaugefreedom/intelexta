// In app/src/components/InspectorPanel.tsx

import React from "react";
import {
  listCheckpoints,
  listRuns,
  CheckpointSummary,
  IncidentSummary,
  RunSummary,
  emitCar,
  replayRun,
  ReplayReport,
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

function proofBadgeFor(kind: string): { label: string; color: string; title: string } {
  switch (kind) {
    case "concordant":
      return { label: "[C]", color: "#c586c0", title: "Concordant proof mode" };
    case "exact":
      return { label: "[E]", color: "#9cdcfe", title: "Exact proof mode" };
    case "interactive":
      return { label: "[I]", color: "#4ec9b0", title: "Interactive proof mode" };
    default:
      return { label: "[?]", color: "#dcdcaa", title: "Unknown proof mode" };
  }
}

function messageRoleBadge(role: string): { label: string; color: string } {
  const normalized = role.trim().toLowerCase();
  switch (normalized) {
    case "human":
    case "user":
      return { label: "Human", color: "#9cdcfe" };
    case "assistant":
    case "ai":
    case "model":
      return { label: "AI", color: "#c586c0" };
    default:
      return { label: role.toUpperCase(), color: "#dcdcaa" };
  }
}

function abbreviateId(value?: string | null): string | null {
  if (!value) {
    return null;
  }
  if (value.length <= 8) {
    return value;
  }
  return `${value.slice(0, 8)}…`;
}

export default function InspectorPanel({
  projectId,
  refreshToken,
  selectedRunId,
  onSelectRun,
}: {
  projectId: string;
  refreshToken: number;
  selectedRunId: string | null;
  onSelectRun: (runId: string | null) => void;
}) {
  const [runs, setRuns] = React.useState<RunSummary[]>([]);
  const [checkpoints, setCheckpoints] = React.useState<CheckpointSummary[]>([]);
  const [loadingRuns, setLoadingRuns] = React.useState<boolean>(false);
  const [loadingCheckpoints, setLoadingCheckpoints] = React.useState<boolean>(false);
  const [runsError, setRunsError] = React.useState<string | null>(null);
  const [checkpointError, setCheckpointError] = React.useState<string | null>(null);
  
  // MERGED STATE: Using the more explicit state management from the `emit_car` branch.
  const [emittingCar, setEmittingCar] = React.useState<boolean>(false);
  const [emitSuccess, setEmitSuccess] = React.useState<string | null>(null);
  const [emitError, setEmitError] = React.useState<string | null>(null);
  const [replayingRun, setReplayingRun] = React.useState<boolean>(false);
  const [replayReport, setReplayReport] = React.useState<ReplayReport | null>(null);
  const [replayError, setReplayError] = React.useState<string | null>(null);

  const selectedRun = React.useMemo(() => {
    if (!selectedRunId) {
      return null;
    }
    return runs.find((run) => run.id === selectedRunId) ?? null;
  }, [runs, selectedRunId]);

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
          const nextId = runList.length > 0 ? runList[0].id : null;
          if (nextId !== selectedRunId) {
            onSelectRun(nextId);
          }
        }
      })
      .catch((err) => {
        if (cancelled) return;
        console.error("Failed to load runs", err);
        setRunsError("Could not load runs for this project.");
        setRuns([]);
        if (selectedRunId !== null) {
          onSelectRun(null);
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoadingRuns(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [projectId, refreshToken, onSelectRun, selectedRunId]);

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
    setReplayReport(null);
    setReplayError(null);
  }, [selectedRunId]);

  const handleEmitCar = React.useCallback(() => {
    if (!selectedRunId) {
      return;
    }
    setEmittingCar(true);
    setEmitSuccess(null);
    setEmitError(null);
    setReplayReport(null);
    setReplayError(null);
    emitCar(selectedRunId)
      .then((path) => {
        setEmitSuccess(`CAR file saved to ${path}`);
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

  const handleReplayRun = React.useCallback(() => {
    if (!selectedRunId) {
      return;
    }
    setReplayingRun(true);
    setReplayError(null);
    setReplayReport(null);
    replayRun(selectedRunId)
      .then((report) => {
        setReplayReport(report);
      })
      .catch((err) => {
        console.error("Failed to replay run", err);
        const message = err instanceof Error ? err.message : String(err);
        setReplayError(`Failed to replay run: ${message}`);
      })
      .finally(() => {
        setReplayingRun(false);
      });
  }, [selectedRunId]);

  const actionDisabled = !selectedRunId || emittingCar || replayingRun;
  const replayButtonLabel = selectedRun?.kind === "concordant" ? "Replay (Concordant)" : "Replay Run";
  const selectedRunBadge = selectedRun ? proofBadgeFor(selectedRun.kind) : null;

  const replayFeedback = React.useMemo(() => {
    if (!replayReport) {
      return null;
    }
    if (
      typeof replayReport.epsilon === "number" &&
      typeof replayReport.semanticDistance === "number"
    ) {
      const rawDistance = replayReport.semanticDistance;
      const normalizedDistance = rawDistance / 64;
      const epsilon = replayReport.epsilon;
      const comparison = normalizedDistance <= epsilon ? "<=" : ">";
      const statusLabel = replayReport.matchStatus ? "PASS" : "FAIL";
      const tone = replayReport.matchStatus ? "success" : "error";
      const distanceDisplay = normalizedDistance.toFixed(2);
      const epsilonDisplay = epsilon.toFixed(2);
      const messageBase = `Concordant Proof: ${statusLabel} (Normalized Distance: ${distanceDisplay} ${comparison} ε: ${epsilonDisplay})`;
      const suffix =
        !replayReport.matchStatus && replayReport.errorMessage
          ? ` — ${replayReport.errorMessage}`
          : "";
      return {
        tone,
        message: `${messageBase}${suffix}`,
      };
    }

    const expectedDigest = replayReport.originalDigest || "∅";
    const replayedDigest = replayReport.replayDigest || "∅";
    if (replayReport.matchStatus) {
      return {
        tone: "success" as const,
        message: `Exact Proof: PASS (Digest: ${replayedDigest})`,
      };
    }
    const details = replayReport.errorMessage ?? "digests differ";
    return {
      tone: "error" as const,
      message: `Exact Proof: FAIL — ${details} (expected ${expectedDigest}, replayed ${replayedDigest})`,
    };
  }, [replayReport]);

  return (
    <div>
      <h2>Inspector</h2>
      <div style={{ fontSize: "0.85rem", marginBottom: "0.5rem", color: "#9cdcfe" }}>
        Project: {projectId}
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: "8px", marginBottom: "12px" }}>
        <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
          Run
          <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
            {selectedRunBadge && (
              <span
                title={selectedRunBadge.title}
                style={{
                  border: `1px solid ${selectedRunBadge.color}`,
                  color: selectedRunBadge.color,
                  borderRadius: "999px",
                  fontSize: "0.7rem",
                  fontWeight: 700,
                  letterSpacing: "0.08em",
                  padding: "1px 6px",
                  minWidth: "3ch",
                  textAlign: "center",
                }}
              >
                {selectedRunBadge.label}
              </span>
            )}
            <select
              value={selectedRunId ?? ""}
              onChange={(event) => onSelectRun(event.target.value || null)}
              disabled={loadingRuns || runs.length === 0}
              style={{ flex: 1 }}
            >
              <option value="" disabled>
                {loadingRuns ? "Loading…" : "Select a run"}
              </option>
              {runs.map((run) => {
                const badge = proofBadgeFor(run.kind);
                const timestamp = new Date(run.createdAt).toLocaleString();
                return (
                  <option key={run.id} value={run.id}>
                    {`${badge.label} ${run.name} · ${timestamp}`}
                  </option>
                );
              })}
            </select>
          </div>
        </label>
        {runsError && <span style={{ color: "#f48771" }}>{runsError}</span>}
        
        {/* MERGED JSX: Using the more detailed button and feedback from the `emit_car` branch. */}
        <div style={{ display: "flex", gap: "8px" }}>
          <button
            type="button"
            onClick={handleEmitCar}
            disabled={actionDisabled}
            style={{ alignSelf: "flex-start" }}
          >
            {emittingCar ? "Emitting…" : "Emit CAR"}
          </button>
          <button
            type="button"
            onClick={handleReplayRun}
            disabled={actionDisabled}
            style={{ alignSelf: "flex-start" }}
          >
            {replayingRun ? "Replaying…" : replayButtonLabel}
          </button>
        </div>
        {emitSuccess && (
          <span style={{ fontSize: "0.8rem", color: "#a5d6a7" }}>{emitSuccess}</span>
        )}
        {emitError && <span style={{ fontSize: "0.8rem", color: "#f48771" }}>{emitError}</span>}
        {replayFeedback && (
          <span
            style={{
              fontSize: "0.8rem",
              color: replayFeedback.tone === "success" ? "#a5d6a7" : "#f48771",
            }}
          >
            {replayFeedback.message}
          </span>
        )}
        {replayError && (
          <span style={{ fontSize: "0.8rem", color: "#f48771" }}>{replayError}</span>
        )}
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
                  <th style={{ textAlign: "left", padding: "4px" }}>Turn</th>
                  <th style={{ textAlign: "left", padding: "4px" }}>Kind</th>
                  <th style={{ textAlign: "left", padding: "4px" }}>Message</th>
                  <th style={{ textAlign: "left", padding: "4px" }}>Inputs SHA</th>
                  <th style={{ textAlign: "left", padding: "4px" }}>Outputs SHA</th>
                  <th style={{ textAlign: "right", padding: "4px" }}>Usage</th>
                </tr>
              </thead>
              <tbody>
                {checkpoints.map((ckpt) => {
                  const isIncident = ckpt.kind === "Incident";
                  const incidentMessage = isIncident
                    ? formatIncidentMessage(ckpt.incident)
                    : null;
                  const severityColor = incidentSeverityColor(ckpt.incident);
                  const turnLabel = ckpt.turnIndex ?? null;
                  const parentLabel = abbreviateId(ckpt.parentCheckpointId);
                  const messageBadge = ckpt.message
                    ? messageRoleBadge(ckpt.message.role)
                    : null;
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
                      <td style={{ padding: "4px", verticalAlign: "top" }}>
                        {turnLabel !== null ? (
                          <div style={{ fontWeight: 600 }}>{turnLabel}</div>
                        ) : (
                          "—"
                        )}
                        {parentLabel && (
                          <div
                            style={{
                              marginTop: "4px",
                              fontSize: "0.7rem",
                              color: "#a6a6a6",
                              fontFamily: "monospace",
                            }}
                          >
                            Parent · {parentLabel}
                          </div>
                        )}
                      </td>
                      <td style={{ padding: "4px" }}>
                        <div style={{ fontWeight: 600 }}>{ckpt.kind}</div>
                        {isIncident && ckpt.incident?.severity && (
                          <span
                            style={{
                              display: "inline-block",
                              marginTop: "4px",
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
                      </td>
                      <td style={{ padding: "4px", verticalAlign: "top" }}>
                        {ckpt.message ? (
                          <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
                            {messageBadge && (
                              <span
                                style={{
                                  alignSelf: "flex-start",
                                  fontSize: "0.7rem",
                                  letterSpacing: "0.08em",
                                  fontWeight: 700,
                                  padding: "2px 6px",
                                  borderRadius: "999px",
                                  border: `1px solid ${messageBadge.color}`,
                                  color: messageBadge.color,
                                  textTransform: "uppercase",
                                }}
                              >
                                {messageBadge.label}
                              </span>
                            )}
                            <div
                              style={{
                                whiteSpace: "pre-wrap",
                                wordBreak: "break-word",
                                lineHeight: 1.4,
                              }}
                            >
                              {ckpt.message.body}
                            </div>
                          </div>
                        ) : isIncident ? (
                          <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
                            <div style={{ fontWeight: 700, color: severityColor }}>{incidentMessage}</div>
                          </div>
                        ) : (
                          <span>—</span>
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