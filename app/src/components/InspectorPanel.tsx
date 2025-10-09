// In app/src/components/InspectorPanel.tsx

import React from "react";
import {
  listCheckpoints,
  listRuns,
  getCheckpointDetails,
  CheckpointSummary,
  CheckpointDetails,
  RunSummary,
  emitCar,
  replayRun,
  ReplayReport,
  ExecutionStepProofSummary,
  RunProofMode,
  ProofBadgeKind,
  ReplayGrade,
} from "../lib/api";
import CheckpointDetailsPanel, {
  formatIncidentMessage,
  incidentSeverityColor,
} from "./CheckpointDetailsPanel";
import {
  buttonPrimary,
  buttonSecondary,
  buttonDisabled,
  combineButtonStyles,
} from "../styles/common.js";

function gradeToDisplay(grade: ReplayGrade): string {
  switch (grade) {
    case 'A_PLUS': return 'A+';
    case 'A': return 'A';
    case 'B': return 'B';
    case 'C': return 'C';
    case 'D': return 'D';
    case 'F': return 'F';
  }
}

function gradeToColor(grade: ReplayGrade): string {
  switch (grade) {
    case 'A_PLUS': return '#4ade80'; // green-400
    case 'A': return '#86efac'; // green-300
    case 'B': return '#fbbf24'; // yellow-400
    case 'C': return '#fb923c'; // orange-400
    case 'D': return '#f87171'; // red-400
    case 'F': return '#ef4444'; // red-500
  }
}

function proofBadgeFor(mode: ProofBadgeKind): {
  label: string;
  color: string;
  title: string;
} {
  switch (mode) {
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

function collectProofBadges(
  stepProofs: ExecutionStepProofSummary[] | undefined,
  fallbackKind?: string,
): ReturnType<typeof proofBadgeFor>[] {
  const uniqueModes: ProofBadgeKind[] = [];
  const seen = new Set<ProofBadgeKind>();
  for (const entry of stepProofs ?? []) {
    const checkpointType = entry.checkpointType?.toLowerCase() ?? "";
    const mode: ProofBadgeKind = checkpointType === "interactivechat"
      ? "interactive"
      : entry.proofMode;
    if (!seen.has(mode)) {
      seen.add(mode);
      uniqueModes.push(mode);
    }
  }
  if (uniqueModes.length === 0 && fallbackKind) {
    const fallback = (fallbackKind as ProofBadgeKind) ?? "unknown";
    if (!seen.has(fallback)) {
      uniqueModes.push(fallback);
    }
  }
  if (uniqueModes.length === 0) {
    uniqueModes.push("unknown");
  }
  const order = new Map<ProofBadgeKind, number>([
    ["concordant", 0],
    ["exact", 1],
    ["interactive", 2],
    ["unknown", 3],
  ]);
  uniqueModes.sort((a, b) => (order.get(a) ?? 10) - (order.get(b) ?? 10));
  return uniqueModes.map((mode) => proofBadgeFor(mode));
}

function formatExecutionTimestamp(value?: string | null): string {
  if (!value) {
    return "Unknown time";
  }
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }
  return parsed.toLocaleString();
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
  selectedExecutionId,
  onSelectRun,
}: {
  projectId: string;
  refreshToken: number;
  selectedRunId: string | null;
  selectedExecutionId: string | null;
  onSelectRun: (runId: string | null, executionId?: string | null) => void;
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
  const [detailsOpen, setDetailsOpen] = React.useState<boolean>(false);
  const [selectedCheckpointId, setSelectedCheckpointId] = React.useState<string | null>(null);
  const [checkpointDetails, setCheckpointDetails] = React.useState<CheckpointDetails | null>(
    null,
  );
  const [detailsLoading, setDetailsLoading] = React.useState<boolean>(false);
  const [detailsError, setDetailsError] = React.useState<string | null>(null);

  const runsWithExecutions = React.useMemo(
    () => runs.filter((run) => (run.executions?.length ?? 0) > 0),
    [runs],
  );

  const selectedRun = React.useMemo(() => {
    if (!selectedRunId) {
      return null;
    }
    return runs.find((run) => run.id === selectedRunId) ?? null;
  }, [runs, selectedRunId]);

  const availableExecutions = selectedRun?.executions ?? [];

  const pendingExecutionIdRef = React.useRef<string | null>(null);

  if (!selectedRunId) {
    pendingExecutionIdRef.current = null;
  }

  const selectedExecutionExists = React.useMemo(() => {
    if (!selectedExecutionId) {
      return false;
    }
    return availableExecutions.some((entry) => entry.id === selectedExecutionId);
  }, [availableExecutions, selectedExecutionId]);

  if (selectedExecutionId && !selectedExecutionExists) {
    pendingExecutionIdRef.current = selectedExecutionId;
  } else if (
    pendingExecutionIdRef.current &&
    availableExecutions.some((entry) => entry.id === pendingExecutionIdRef.current)
  ) {
    pendingExecutionIdRef.current = null;
  } else if (!selectedExecutionId) {
    pendingExecutionIdRef.current = null;
  }

  const activeExecutionId = React.useMemo(() => {
    const pendingExecutionId = pendingExecutionIdRef.current;

    if (selectedExecutionId) {
      if (selectedExecutionExists || pendingExecutionId === selectedExecutionId) {
        return selectedExecutionId;
      }
    }

    if (
      pendingExecutionId &&
      !availableExecutions.some((entry) => entry.id === pendingExecutionId)
    ) {
      return pendingExecutionId;
    }

    return availableExecutions[0]?.id ?? null;
  }, [availableExecutions, selectedExecutionExists, selectedExecutionId]);

  const selectedExecution = activeExecutionId
    ? availableExecutions.find((entry) => entry.id === activeExecutionId) ?? null
    : null;

  const selectedRunIdWithCheckpoint = selectedRun?.id ?? null;

  React.useEffect(() => {
    if (!projectId) return;
    let cancelled = false;
    setLoadingRuns(true);
    setRunsError(null);
    listRuns(projectId)
      .then((runList) => {
        if (cancelled) return;
        setRuns(runList);
      })
      .catch((err) => {
        if (cancelled) return;
        console.error("Failed to load runs", err);
        setRunsError("Could not load runs for this project.");
        setRuns([]);
        onSelectRun(null, null);
      })
      .finally(() => {
        if (!cancelled) {
          setLoadingRuns(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [projectId, refreshToken, onSelectRun]);

  React.useEffect(() => {
    if (runs.length === 0) {
      if (selectedRunId !== null || selectedExecutionId !== null) {
        onSelectRun(null, null);
      }
      return;
    }

    if (runsWithExecutions.length === 0) {
      if (selectedRunId && runs.some((run) => run.id === selectedRunId)) {
        if (selectedExecutionId !== null) {
          onSelectRun(selectedRunId, null);
        }
      } else {
        const fallbackRunId = runs[0].id;
        if (selectedRunId !== fallbackRunId || selectedExecutionId !== null) {
          onSelectRun(fallbackRunId, null);
        }
      }
      return;
    }

    let nextRunId = selectedRunId;
    if (!nextRunId || !runs.some((run) => run.id === nextRunId)) {
      nextRunId = runsWithExecutions[0].id;
    }

    const run = runs.find((item) => item.id === nextRunId) ?? runsWithExecutions[0];
    const executions = run.executions ?? [];
    const pendingExecutionId = pendingExecutionIdRef.current;
    const selectedExecutionIsAvailable =
      selectedExecutionId !== null &&
      executions.some((entry) => entry.id === selectedExecutionId);
    const pendingExecutionIsMissing =
      pendingExecutionId !== null &&
      !executions.some((entry) => entry.id === pendingExecutionId);

    if (pendingExecutionId !== null && !pendingExecutionIsMissing) {
      pendingExecutionIdRef.current = null;
    }

    let nextExecutionId: string | null = null;
    if (executions.length === 0) {
      nextExecutionId = null;
    } else if (selectedExecutionIsAvailable) {
      nextExecutionId = selectedExecutionId;
    } else if (pendingExecutionIsMissing) {
      nextExecutionId = pendingExecutionId;
    } else {
      nextExecutionId = executions[0].id;
    }

    if (nextRunId !== selectedRunId || nextExecutionId !== selectedExecutionId) {
      onSelectRun(nextRunId, nextExecutionId ?? null);
    }
  }, [runs, runsWithExecutions, selectedRunId, selectedExecutionId, onSelectRun]);

  React.useEffect(() => {
    if (!selectedRunIdWithCheckpoint || !activeExecutionId) {
      setCheckpoints([]);
      setCheckpointError(null);
      return;
    }
    let cancelled = false;
    setLoadingCheckpoints(true);
    setCheckpointError(null);
    listCheckpoints(activeExecutionId)
      .then((items) => {
        if (!cancelled) {
          setCheckpoints(items);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          console.error("Failed to load steps", err);
          // This now displays the EXACT error message from the backend
          setCheckpointError(`Could not load steps: ${err as string}`);
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
  }, [selectedRunIdWithCheckpoint, activeExecutionId, refreshToken]);

  // MERGED LOGIC: Using the useEffect and useCallback from the `emit_car` branch.
  React.useEffect(() => {
    setEmitSuccess(null);
    setEmitError(null);
    setReplayReport(null);
    setReplayError(null);
    setDetailsOpen(false);
    setSelectedCheckpointId(null);
    setCheckpointDetails(null);
    setDetailsError(null);
    setDetailsLoading(false);
  }, [selectedRunId, activeExecutionId]);

  React.useEffect(() => {
    if (
      selectedCheckpointId &&
      !checkpoints.some((entry) => entry.id === selectedCheckpointId)
    ) {
      setDetailsOpen(false);
      setSelectedCheckpointId(null);
      setCheckpointDetails(null);
      setDetailsError(null);
      setDetailsLoading(false);
        }
  }, [checkpoints, selectedCheckpointId]);

  React.useEffect(() => {
    if (!detailsOpen || !selectedCheckpointId) {
      return;
    }
    let cancelled = false;
    setDetailsLoading(true);
    setDetailsError(null);
    setCheckpointDetails(null);
    getCheckpointDetails(selectedCheckpointId)
      .then((details) => {
        if (cancelled) {
          return;
        }
        setCheckpointDetails(details);
      })
      .catch((err) => {
        if (cancelled) {
          return;
        }
        console.error("Failed to load checkpoint details", err);
        const message = err instanceof Error ? err.message : String(err);
        setDetailsError(`Could not load checkpoint details: ${message}`);
      })
      .finally(() => {
        if (!cancelled) {
          setDetailsLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [detailsOpen, selectedCheckpointId]);

  const handleEmitCar = React.useCallback(async () => {
    console.log('Emit CAR button clicked');
    if (!selectedRunIdWithCheckpoint) {
      console.log('No run selected');
      return;
    }

    console.log('Importing dialog plugin...');
    // Import save dialog
    const { save } = await import('@tauri-apps/plugin-dialog');

    try {
      console.log('Opening save dialog...');
      const savePath = await save({
        defaultPath: `${selectedRunIdWithCheckpoint.replace(/:/g, '_')}.car.zip`,
        filters: [
          { name: 'CAR Bundle', extensions: ['car.zip', 'zip'] },
          { name: 'All Files', extensions: ['*'] },
        ],
      });
      console.log('Save dialog result:', savePath);

      if (!savePath) {
        // User cancelled
        console.log('User cancelled save dialog');
        return;
      }

      setEmittingCar(true);
      setEmitSuccess(null);
      setEmitError(null);
      setReplayReport(null);
      setReplayError(null);

      emitCar(selectedRunIdWithCheckpoint, savePath)
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
    } catch (err) {
      console.error('Failed to show save dialog:', err);
    }
  }, [selectedRunIdWithCheckpoint]);

  const handleReplayRun = React.useCallback(() => {
    if (!selectedRunIdWithCheckpoint) {
      return;
    }
    setReplayingRun(true);
    setReplayError(null);
    setReplayReport(null);
    replayRun(selectedRunIdWithCheckpoint)
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
  }, [selectedRunIdWithCheckpoint]);

  const handleOpenDetails = React.useCallback((checkpointId: string) => {
    setSelectedCheckpointId(checkpointId);
    setDetailsOpen(true);
  }, []);

  const handleCloseDetails = React.useCallback(() => {
    setDetailsOpen(false);
    setSelectedCheckpointId(null);
    setCheckpointDetails(null);
    setDetailsError(null);
    setDetailsLoading(false);
  }, []);

  const actionDisabled =
    !selectedRunIdWithCheckpoint || !activeExecutionId || emittingCar || replayingRun;
  const executionHasConcordant = React.useMemo(() => {
    if (selectedExecution?.stepProofs) {
      return selectedExecution.stepProofs.some((entry) => entry.proofMode === "concordant");
    }
    if (selectedRun?.stepProofs) {
      return selectedRun.stepProofs.some((entry) => entry.proofMode === "concordant");
    }
    return selectedRun?.kind === "concordant";
  }, [selectedExecution?.stepProofs, selectedRun?.stepProofs, selectedRun?.kind]);
  const replayButtonLabel = executionHasConcordant ? "Replay (Concordant)" : "Replay Run";
  const selectedRunBadges = React.useMemo(
    () => collectProofBadges(selectedRun?.stepProofs, selectedRun?.kind),
    [selectedRun?.stepProofs, selectedRun?.kind],
  );
  const selectedExecutionBadges = React.useMemo(
    () => collectProofBadges(selectedExecution?.stepProofs ?? selectedRun?.stepProofs, selectedRun?.kind),
    [selectedExecution?.stepProofs, selectedRun?.stepProofs, selectedRun?.kind],
  );

  const replayFeedback = React.useMemo(() => {
    if (!replayReport) {
      return null;
    }

    const checkpoints = replayReport.checkpointReports ?? [];

    const buildMessageForCheckpoint = (
      entry: ReplayReport["checkpointReports"][number],
      index: number,
    ) => {
      const tone = entry.matchStatus ? "success" : "error";
      const parts: string[] = [];
      if (typeof entry.orderIndex === "number") {
        parts.push(`#${entry.orderIndex}`);
      }
      if (entry.checkpointType) {
        parts.push(entry.checkpointType);
      }
      const label = parts.length > 0 ? parts.join(" ") : "Checkpoint";
      let message: string;
      const configuredMode: ProofBadgeKind = entry.proofMode ?? entry.mode ?? "unknown";
      if (
        configuredMode === "concordant" &&
        typeof entry.semanticDistance === "number" &&
        typeof entry.epsilon === "number"
      ) {
        const normalized = entry.semanticDistance / 64;
        const comparison = normalized <= entry.epsilon ? "<=" : ">";
        const configuredEpsilon = entry.configuredEpsilon ?? entry.epsilon;
        const epsilonText =
          typeof configuredEpsilon === "number" ? configuredEpsilon.toFixed(2) : "∅";

        // Build message with grade and similarity
        const gradeText = entry.grade ? ` [Grade: ${gradeToDisplay(entry.grade)}]` : '';
        const similarityText = typeof entry.similarityScore === "number"
          ? ` ${(entry.similarityScore * 100).toFixed(1)}% similar`
          : '';

        message = `Concordant ${label}: ${entry.matchStatus ? "PASS" : "FAIL"}${gradeText}${similarityText} (distance ${normalized.toFixed(
          2,
        )} ${comparison} ε=${epsilonText})`;
        if (!entry.matchStatus && entry.errorMessage) {
          message += ` — ${entry.errorMessage}`;
        }
      } else if (configuredMode === "interactive") {
        message = `Interactive ${label}: ${entry.matchStatus ? "PASS" : "FAIL"}`;
        if (entry.errorMessage) {
          message += ` — ${entry.errorMessage}`;
        }
      } else if (configuredMode === "exact" && entry.matchStatus) {
        message = `Exact ${label}: PASS (digest ${entry.replayDigest || "∅"})`;
      } else {
        const expected = entry.originalDigest || "∅";
        const replayed = entry.replayDigest || "∅";
        const details = entry.errorMessage ?? "digests differ";
        const modeLabel =
          configuredMode === "unknown"
            ? "Checkpoint"
            : configuredMode.charAt(0).toUpperCase() + configuredMode.slice(1);
        message = `${modeLabel} ${label}: FAIL — ${details} (expected ${expected}, replayed ${replayed})`;
      }
      return {
        key: entry.checkpointConfigId ?? `checkpoint-${index}`,
        tone,
        message,
        grade: entry.grade,
      };
    };

    if (checkpoints.length === 0) {
      const base = replayReport.matchStatus ? "Replay PASS" : "Replay FAIL";
      const overallMessage = replayReport.errorMessage ? `${base} — ${replayReport.errorMessage}` : base;
      return {
        overallTone: replayReport.matchStatus ? "success" : "error",
        overallMessage,
        checkpoints: [] as { key: string; tone: "success" | "error"; message: string; grade?: ReplayGrade | null }[],
        overallGrade: null,
        overallSimilarity: null,
      };
    }

    const checkpointMessages = checkpoints.map((entry, index) =>
      buildMessageForCheckpoint(entry, index),
    );

    const summaryBase = replayReport.matchStatus ? "Replay PASS" : "Replay FAIL";
    const gradeText = replayReport.grade ? ` [Overall Grade: ${gradeToDisplay(replayReport.grade)}]` : '';
    const similarityText = typeof replayReport.similarityScore === "number"
      ? ` ${(replayReport.similarityScore * 100).toFixed(1)}% similar`
      : '';

    const overallMessage = replayReport.matchStatus
      ? `${summaryBase}${gradeText}${similarityText}`
      : replayReport.errorMessage
      ? `${summaryBase} — ${replayReport.errorMessage}`
      : summaryBase;

    return {
      overallTone: replayReport.matchStatus ? "success" : "error",
      overallMessage,
      checkpoints: checkpointMessages,
      overallGrade: replayReport.grade,
      overallSimilarity: replayReport.similarityScore,
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
            <div style={{ display: "flex", gap: "4px" }}>
              {selectedRun
                ? selectedRunBadges.map((badge) => (
                    <span
                      key={`${badge.label}-${badge.title}`}
                      title={badge.title}
                      style={{
                        border: `1px solid ${badge.color}`,
                        color: badge.color,
                        borderRadius: "999px",
                        fontSize: "0.7rem",
                        fontWeight: 700,
                        letterSpacing: "0.08em",
                        padding: "1px 6px",
                        minWidth: "3ch",
                        textAlign: "center",
                      }}
                    >
                      {badge.label}
                    </span>
                  ))
                : null}
            </div>
            <select
              value={selectedRunIdWithCheckpoint ?? ""}
              onChange={(event) => onSelectRun(event.target.value || null, undefined)}
              disabled={loadingRuns || runsWithExecutions.length === 0}
              style={{ width: '100%' }}
            >
              <option value="" disabled>
                {loadingRuns
                  ? "Loading…"
                  : runsWithExecutions.length === 0
                  ? "No executed runs"
                  : "Select a run"}
              </option>
              {runsWithExecutions.map((run) => {
                const badges = collectProofBadges(run.stepProofs, run.kind);
                const badgeText = badges.map((badge) => badge.label).join(" ");
                const timestamp = formatExecutionTimestamp(run.createdAt);
                return (
                  <option key={run.id} value={run.id}>
                    {`${badgeText} ${run.name} · ${timestamp}`}
                  </option>
                );
              })}
            </select>
          </div>
        </label>
        <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
          Run Execution
          <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
            <div style={{ display: "flex", gap: "4px" }}>
              {selectedRun
                ? selectedExecutionBadges.map((badge) => (
                    <span
                      key={`execution-${badge.label}-${badge.title}`}
                      title={badge.title}
                      style={{
                        border: `1px solid ${badge.color}`,
                        color: badge.color,
                        borderRadius: "999px",
                        fontSize: "0.7rem",
                        fontWeight: 700,
                        letterSpacing: "0.08em",
                        padding: "1px 6px",
                        minWidth: "3ch",
                        textAlign: "center",
                      }}
                    >
                      {badge.label}
                    </span>
                  ))
                : null}
            </div>
          <select
            value={activeExecutionId ?? ""}
            onChange={(event) =>
              onSelectRun(
                selectedRunIdWithCheckpoint,
                event.target.value ? event.target.value : null,
              )
            }
            disabled={!selectedRunIdWithCheckpoint || availableExecutions.length === 0}
            style={{ width: "100%" }}
          >
            <option value="" disabled>
              {availableExecutions.length === 0 ? "No executions" : "Select an execution"}
            </option>
            {availableExecutions.map((execution) => {
              const badges = collectProofBadges(execution.stepProofs, selectedRun?.kind);
              const badgeText = badges.map((badge) => badge.label).join(" ");
              const timestamp = formatExecutionTimestamp(execution.createdAt);
              const executionLabel = abbreviateId(execution.id) ?? execution.id;
              return (
                <option key={execution.id} value={execution.id}>
                  {`${badgeText} ${timestamp} · ${executionLabel}`}
                </option>
              );
            })}
          </select>
          </div>
        </label>
        {selectedExecution && (
          <div style={{ fontSize: "0.75rem", color: "#9cdcfe" }}>
            Viewing execution {abbreviateId(selectedExecution.id) ?? selectedExecution.id} from{' '}
            {formatExecutionTimestamp(selectedExecution.createdAt)}
          </div>
        )}
        {runsError && <span style={{ color: "#f48771" }}>{runsError}</span>}
        {!runsError && !loadingRuns && runsWithExecutions.length === 0 && (
          <span style={{ fontSize: "0.8rem", color: "#808080" }}>
            No executed runs are available. Launch a workflow to populate the history.
          </span>
        )}

        {/* MERGED JSX: Using the more detailed button and feedback from the `emit_car` branch. */}
        <div style={{ display: "flex", gap: "8px" }}>
          <button
            type="button"
            onClick={handleEmitCar}
            disabled={actionDisabled}
            style={combineButtonStyles(
              buttonPrimary,
              actionDisabled && buttonDisabled,
              { alignSelf: "flex-start" },
            )}
          >
            {emittingCar ? "Emitting…" : "Emit CAR"}
          </button>
          <button
            type="button"
            onClick={handleReplayRun}
            disabled={actionDisabled}
            style={combineButtonStyles(
              buttonSecondary,
              actionDisabled && buttonDisabled,
              { alignSelf: "flex-start" },
            )}
          >
            {replayingRun ? "Replaying…" : replayButtonLabel}
          </button>
        </div>
        {emitSuccess && (
          <span style={{ fontSize: "0.8rem", color: "#a5d6a7" }}>{emitSuccess}</span>
        )}
        {emitError && <span style={{ fontSize: "0.8rem", color: "#f48771" }}>{emitError}</span>}
        {replayFeedback && (
          <div style={{ fontSize: "0.8rem", display: "flex", flexDirection: "column", gap: "4px" }}>
            <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
              {replayFeedback.overallGrade && (
                <span
                  style={{
                    display: "inline-block",
                    padding: "2px 8px",
                    borderRadius: "4px",
                    fontWeight: "bold",
                    fontSize: "0.75rem",
                    backgroundColor: gradeToColor(replayFeedback.overallGrade),
                    color: "#000",
                  }}
                >
                  {gradeToDisplay(replayFeedback.overallGrade)}
                </span>
              )}
              <span
                style={{
                  color: replayFeedback.overallTone === "success" ? "#a5d6a7" : "#f48771",
                }}
              >
                {replayFeedback.overallMessage}
              </span>
            </div>
            {replayFeedback.checkpoints.length > 0 && (
              <ul style={{ margin: 0, paddingLeft: "1.2rem", listStyleType: "disc" }}>
                {replayFeedback.checkpoints.map((entry) => (
                  <li
                    key={entry.key}
                    style={{
                      color: entry.tone === "success" ? "#a5d6a7" : "#f48771",
                      display: "flex",
                      alignItems: "center",
                      gap: "8px",
                    }}
                  >
                    {entry.grade && (
                      <span
                        style={{
                          display: "inline-block",
                          padding: "1px 6px",
                          borderRadius: "3px",
                          fontWeight: "bold",
                          fontSize: "0.7rem",
                          backgroundColor: gradeToColor(entry.grade),
                          color: "#000",
                          flexShrink: 0,
                        }}
                      >
                        {gradeToDisplay(entry.grade)}
                      </span>
                    )}
                    <span>{entry.message}</span>
                  </li>
                ))}
              </ul>
            )}
          </div>
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
                  {/* --- Simplified Headers --- */}
                  <th style={{ textAlign: "left", padding: "4px", width: '180px' }}>Timestamp</th>
                  <th style={{ textAlign: "left", padding: "4px", width: '80px' }}>Kind</th>
                  <th style={{ textAlign: "left", padding: "4px" }}>Summary</th>
                  <th style={{ textAlign: "right", padding: "4px", width: '80px' }}>Usage</th>
                </tr>
              </thead>
              <tbody>
                {checkpoints.map((ckpt) => {
                  const isIncident = ckpt.kind === "Incident";
                  const incidentMessage = isIncident ? formatIncidentMessage(ckpt.incident) : null;
                  const isSelected = selectedCheckpointId === ckpt.id;
                  const baseBackground = isIncident ? "#2d1616" : undefined;
                  const rowStyle: React.CSSProperties = {
                    borderBottom: "1px solid #222",
                    backgroundColor: isSelected ? "#1f2937" : baseBackground,
                    cursor: "pointer",
                  };
                  if (isSelected) {
                    rowStyle.boxShadow = "inset 3px 0 0 #9cdcfe";
                  }

                  // Logic for the new Summary column
                  const summaryText = isIncident
                    ? incidentMessage
                    : ckpt.message
                    ? ckpt.message.body
                    : '—';

                  return (
                    <tr
                      key={ckpt.id}
                      onClick={() => handleOpenDetails(ckpt.id)}
                      onKeyDown={(event) => {
                        if (event.key === "Enter" || event.key === " ") {
                          event.preventDefault();
                          handleOpenDetails(ckpt.id);
                        }
                      }}
                      role="button"
                      tabIndex={0}
                      aria-pressed={isSelected}
                      style={rowStyle}
                      title="Inspect checkpoint details"
                    >
                      {/* --- Simplified Cells --- */}
                      <td style={{ padding: "4px", verticalAlign: "top" }}>
                        {new Date(ckpt.timestamp).toLocaleString()}
                      </td>
                      <td style={{ padding: "4px", verticalAlign: "top" }}>
                        <div style={{ fontWeight: 600 }}>{ckpt.kind}</div>
                      </td>
                      <td style={{ 
                        padding: "4px", 
                        verticalAlign: "top",
                        whiteSpace: 'nowrap',
                        overflow: 'hidden',
                        textOverflow: 'ellipsis',
                        maxWidth: '200px' // Adjust as needed
                      }}>
                        {summaryText}
                      </td>
                      <td style={{ padding: "4px", textAlign: "right", verticalAlign: "top" }}>
                        {ckpt.usageTokens}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
            <div style={{ fontSize: "0.75rem", color: "#a6a6a6", marginTop: "8px" }}>
              Click a checkpoint row to inspect prompts, responses, and digests.
            </div>
          </div>
        ) : (
          <p>No checkpoints recorded for this run yet.</p>
        )
      ) : (
        <p>Select a run to inspect its checkpoints.</p>
      )}
      {checkpointError && <div style={{ color: "#f48771", marginTop: "8px" }}>{checkpointError}</div>}
      <CheckpointDetailsPanel
        open={detailsOpen}
        onClose={handleCloseDetails}
        checkpointDetails={checkpointDetails}
        loading={detailsLoading}
        error={detailsError}
      />
    </div>
  );
}