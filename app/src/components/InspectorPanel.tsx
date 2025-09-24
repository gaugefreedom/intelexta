// In app/src/components/InspectorPanel.tsx

import React from "react";
import {
  listCheckpoints,
  listRuns,
  getCheckpointDetails,
  CheckpointSummary,
  CheckpointDetails,
  IncidentSummary,
  RunSummary,
  emitCar,
  replayRun,
  ReplayReport,
  ExecutionStepProofSummary,
  RunProofMode,
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

type ProofBadgeKind = RunProofMode | "interactive" | "unknown";

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

type PayloadViewMode = "raw" | "canonical" | "digest";

interface DigestItem {
  label: string;
  value?: string | null;
}

function canonicalizeValue(value: unknown): unknown {
  if (Array.isArray(value)) {
    return value.map((entry) => canonicalizeValue(entry));
  }
  if (value && typeof value === "object") {
    const entries = Object.entries(value as Record<string, unknown>)
      .map(([key, entryValue]) => [key, canonicalizeValue(entryValue)] as const)
      .sort(([keyA], [keyB]) => keyA.localeCompare(keyB));
    return entries.reduce<Record<string, unknown>>((acc, [key, entryValue]) => {
      acc[key] = entryValue;
      return acc;
    }, {});
  }
  return value;
}

function canonicalizeJsonText(input?: string | null): string | null {
  if (input === undefined || input === null) {
    return null;
  }
  const trimmed = input.trim();
  if (!trimmed) {
    return null;
  }
  try {
    const parsed = JSON.parse(trimmed);
    const canonical = canonicalizeValue(parsed);
    return JSON.stringify(canonical, null, 2);
  } catch (_error) {
    return null;
  }
}

function safeFileName(base: string): string {
  return base.replace(/[^a-z0-9._-]+/gi, "_");
}

interface PayloadViewerProps {
  label: string;
  raw?: string | null;
  canonical?: string | null;
  digestItems: DigestItem[];
  viewMode: PayloadViewMode;
  onChangeMode: (mode: PayloadViewMode) => void;
  downloadBaseName: string;
}

function PayloadViewer({
  label,
  raw,
  canonical,
  digestItems,
  viewMode,
  onChangeMode,
  downloadBaseName,
}: PayloadViewerProps) {
  const hasRawValue = raw !== undefined && raw !== null;
  const hasCanonicalValue = canonical !== undefined && canonical !== null;
  const digestLines = digestItems
    .map((item) => ({ ...item, value: item.value ?? null }))
    .filter((item) => item.value !== null && item.value !== undefined && item.value !== "");
  const digestContent =
    digestLines.length > 0
      ? digestLines.map((item) => `${item.label}: ${item.value}`).join("\n")
      : "No digest information recorded.";

  let displayContent: string;
  switch (viewMode) {
    case "raw":
      displayContent = hasRawValue ? (raw as string) : "No raw content stored for this checkpoint.";
      break;
    case "canonical":
      displayContent = hasCanonicalValue
        ? (canonical as string)
        : "Canonical JSON view is only available for valid JSON payloads.";
      break;
    case "digest":
    default:
      displayContent = digestContent;
      break;
  }

  const copyDisabled =
    viewMode === "raw"
      ? !hasRawValue
      : viewMode === "canonical"
      ? !hasCanonicalValue
      : digestContent.length === 0;

  const toggleStyle = (mode: PayloadViewMode, enabled: boolean): React.CSSProperties => {
    const isActive = viewMode === mode;
    return {
      fontSize: "0.7rem",
      padding: "2px 8px",
      borderRadius: "4px",
      border: `1px solid ${isActive ? "#9cdcfe" : "#333"}`,
      backgroundColor: isActive ? "#1f2937" : "#111",
      color: enabled ? (isActive ? "#9cdcfe" : "#ccc") : "#666",
      cursor: enabled ? "pointer" : "not-allowed",
      opacity: enabled ? 1 : 0.5,
    };
  };

  const handleCopy = React.useCallback(() => {
    if (copyDisabled) {
      return;
    }
    try {
      if (typeof navigator !== "undefined" && navigator.clipboard) {
        void navigator.clipboard.writeText(displayContent);
      } else {
        throw new Error("Clipboard API not available");
      }
    } catch (error) {
      console.error(`Failed to copy ${label} ${viewMode}`, error);
    }
  }, [copyDisabled, displayContent, label, viewMode]);

  const handleDownload = React.useCallback(() => {
    if (copyDisabled) {
      return;
    }
    try {
      const fileName = `${safeFileName(`${downloadBaseName}-${label.toLowerCase()}-${viewMode}`)}.txt`;
      const blob = new Blob([displayContent], {
        type: "text/plain;charset=utf-8",
      });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = fileName;
      document.body.appendChild(anchor);
      anchor.click();
      document.body.removeChild(anchor);
      URL.revokeObjectURL(url);
    } catch (error) {
      console.error(`Failed to download ${label} ${viewMode}`, error);
    }
  }, [copyDisabled, displayContent, downloadBaseName, label, viewMode]);

  return (
    <div style={{ marginTop: "16px" }}>
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          gap: "8px",
        }}
      >
        <h4 style={{ margin: 0 }}>{label}</h4>
        <div style={{ display: "flex", gap: "6px" }}>
          <button
            type="button"
            onClick={() => onChangeMode("raw")}
            disabled={!hasRawValue}
            style={toggleStyle("raw", hasRawValue)}
            title={hasRawValue ? "Show raw text" : "No raw payload stored"}
          >
            Raw
          </button>
          <button
            type="button"
            onClick={() => onChangeMode("canonical")}
            disabled={!hasCanonicalValue}
            style={toggleStyle("canonical", hasCanonicalValue)}
            title="View canonical JSON"
          >
            Canonical JSON
          </button>
          <button
            type="button"
            onClick={() => onChangeMode("digest")}
            style={toggleStyle("digest", true)}
            title="View recorded digests"
          >
            Digest
          </button>
        </div>
      </div>
      <pre
        style={{
          marginTop: "8px",
          border: "1px solid #222",
          borderRadius: "6px",
          padding: "8px",
          backgroundColor: "#111",
          fontFamily: "monospace",
          fontSize: "0.8rem",
          whiteSpace: "pre-wrap",
          wordBreak: "break-word",
          maxHeight: "35vh",
          overflowY: "auto",
        }}
      >
        {displayContent}
      </pre>
      <div style={{ marginTop: "8px", display: "flex", gap: "8px" }}>
        <button type="button" onClick={handleCopy} disabled={copyDisabled}>
          Copy
        </button>
        <button type="button" onClick={handleDownload} disabled={copyDisabled}>
          Download
        </button>
      </div>
    </div>
  );
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
  const [promptViewMode, setPromptViewMode] = React.useState<PayloadViewMode>("raw");
  const [outputViewMode, setOutputViewMode] = React.useState<PayloadViewMode>("raw");

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

  const activeExecutionId = React.useMemo(() => {
    if (availableExecutions.length === 0) {
      return null;
    }
    if (selectedExecutionId) {
      const exists = availableExecutions.some((entry) => entry.id === selectedExecutionId);
      if (exists) {
        return selectedExecutionId;
      }
    }
    return availableExecutions[0]?.id ?? null;
  }, [availableExecutions, selectedExecutionId]);

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
    const nextExecutionId =
      executions.length === 0
        ? null
        : selectedExecutionId && executions.some((entry) => entry.id === selectedExecutionId)
        ? selectedExecutionId
        : executions[0].id;

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
    listCheckpoints(selectedRunIdWithCheckpoint, activeExecutionId)
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
    setPromptViewMode("raw");
    setOutputViewMode("raw");
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
      setPromptViewMode("raw");
      setOutputViewMode("raw");
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
        setPromptViewMode(details.promptPayload != null ? "raw" : "digest");
        setOutputViewMode(details.outputPayload != null ? "raw" : "digest");
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

  const handleEmitCar = React.useCallback(() => {
    if (!selectedRunIdWithCheckpoint) {
      return;
    }
    setEmittingCar(true);
    setEmitSuccess(null);
    setEmitError(null);
    setReplayReport(null);
    setReplayError(null);
    emitCar(selectedRunIdWithCheckpoint)
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
    setPromptViewMode("raw");
    setOutputViewMode("raw");
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
        message = `Concordant ${label}: ${entry.matchStatus ? "PASS" : "FAIL"} (distance ${normalized.toFixed(
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
      };
    };

    if (checkpoints.length === 0) {
      const base = replayReport.matchStatus ? "Replay PASS" : "Replay FAIL";
      const overallMessage = replayReport.errorMessage ? `${base} — ${replayReport.errorMessage}` : base;
      return {
        overallTone: replayReport.matchStatus ? "success" : "error",
        overallMessage,
        checkpoints: [] as { key: string; tone: "success" | "error"; message: string }[],
      };
    }

    const checkpointMessages = checkpoints.map((entry, index) =>
      buildMessageForCheckpoint(entry, index),
    );

    const summaryBase = replayReport.matchStatus ? "Replay PASS" : "Replay FAIL";
    const overallMessage = replayReport.matchStatus
      ? summaryBase
      : replayReport.errorMessage
      ? `${summaryBase} — ${replayReport.errorMessage}`
      : summaryBase;

    return {
      overallTone: replayReport.matchStatus ? "success" : "error",
      overallMessage,
      checkpoints: checkpointMessages,
    };
  }, [replayReport]);

  const promptCanonical = React.useMemo(
    () => canonicalizeJsonText(checkpointDetails?.promptPayload ?? null),
    [checkpointDetails?.promptPayload],
  );
  const outputCanonical = React.useMemo(
    () => canonicalizeJsonText(checkpointDetails?.outputPayload ?? null),
    [checkpointDetails?.outputPayload],
  );

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
              style={{ flex: 1 }}
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
          <div style={{ fontSize: "0.8rem", display: "flex", flexDirection: "column", gap: "4px" }}>
            <span
              style={{
                color: replayFeedback.overallTone === "success" ? "#a5d6a7" : "#f48771",
              }}
            >
              {replayFeedback.overallMessage}
            </span>
            {replayFeedback.checkpoints.length > 0 && (
              <ul style={{ margin: 0, paddingLeft: "1.2rem", listStyleType: "disc" }}>
                {replayFeedback.checkpoints.map((entry) => (
                  <li
                    key={entry.key}
                    style={{
                      color: entry.tone === "success" ? "#a5d6a7" : "#f48771",
                    }}
                  >
                    {entry.message}
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
      {detailsOpen && (
        <>
          <div
            role="presentation"
            onClick={handleCloseDetails}
            style={{
              position: "fixed",
              inset: 0,
              backgroundColor: "rgba(0, 0, 0, 0.45)",
              zIndex: 40,
            }}
          />
          <aside
            role="dialog"
            aria-modal="true"
            aria-label="Checkpoint details"
            style={{
              position: "fixed",
              top: 0,
              right: 0,
              width: "min(480px, 90vw)",
              height: "100%",
              backgroundColor: "#0f111a",
              borderLeft: "1px solid #222",
              boxShadow: "-4px 0 12px rgba(0, 0, 0, 0.4)",
              padding: "16px",
              overflowY: "auto",
              zIndex: 41,
            }}
            onClick={(event) => event.stopPropagation()}
          >
            <div
              style={{
                display: "flex",
                alignItems: "flex-start",
                justifyContent: "space-between",
                gap: "12px",
                marginBottom: "12px",
              }}
            >
              <div>
                <h3 style={{ margin: 0 }}>Checkpoint Details</h3>
                {checkpointDetails?.id && (
                  <div
                    style={{
                      fontSize: "0.75rem",
                      color: "#9cdcfe",
                      marginTop: "4px",
                      fontFamily: "monospace",
                      wordBreak: "break-all",
                    }}
                  >
                    {checkpointDetails.id}
                  </div>
                )}
              </div>
              <button type="button" onClick={handleCloseDetails}>
                Close
              </button>
            </div>
            {detailsLoading && <p>Loading details…</p>}
            {detailsError && !detailsLoading && (
              <p style={{ color: "#f48771" }}>{detailsError}</p>
            )}
            {!detailsLoading && !detailsError && checkpointDetails && (
              <div>
                <dl
                  style={{
                    display: "grid",
                    gridTemplateColumns: "max-content 1fr",
                    gap: "6px 12px",
                    fontSize: "0.85rem",
                    margin: 0,
                  }}
                >
                  <dt>Run</dt>
                  <dd
                    style={{
                      margin: 0,
                      fontFamily: "monospace",
                      wordBreak: "break-all",
                    }}
                  >
                    {checkpointDetails.runId}
                  </dd>
                  <dt>Timestamp</dt>
                  <dd style={{ margin: 0 }}>
                    {new Date(checkpointDetails.timestamp).toLocaleString()}
                  </dd>
                  <dt>Kind</dt>
                  <dd style={{ margin: 0 }}>{checkpointDetails.kind}</dd>
                  {typeof checkpointDetails.turnIndex === "number" && (
                    <>
                      <dt>Turn</dt>
                      <dd style={{ margin: 0 }}>{checkpointDetails.turnIndex}</dd>
                    </>
                  )}
                  {checkpointDetails.parentCheckpointId && (
                    <>
                      <dt>Parent</dt>
                      <dd
                        style={{
                          margin: 0,
                          fontFamily: "monospace",
                          wordBreak: "break-all",
                        }}
                      >
                        {checkpointDetails.parentCheckpointId}
                      </dd>
                    </>
                  )}
                  {checkpointDetails.checkpointConfigId && (
                    <>
                      <dt>Config</dt>
                      <dd
                        style={{
                          margin: 0,
                          fontFamily: "monospace",
                          wordBreak: "break-all",
                        }}
                      >
                        {checkpointDetails.checkpointConfigId}
                      </dd>
                    </>
                  )}
                  <dt>Usage</dt>
                  <dd style={{ margin: 0 }}>
                    {`${checkpointDetails.usageTokens} tokens (prompt ${checkpointDetails.promptTokens} · completion ${checkpointDetails.completionTokens})`}
                  </dd>
                </dl>
                {checkpointDetails.incident && (
                  <div
                    style={{
                      marginTop: "12px",
                      padding: "8px",
                      borderRadius: "6px",
                      border: `1px solid ${incidentSeverityColor(checkpointDetails.incident)}`,
                      backgroundColor: "#211112",
                    }}
                  >
                    <div
                      style={{
                        fontWeight: 700,
                        color: incidentSeverityColor(checkpointDetails.incident),
                      }}
                    >
                      {formatIncidentMessage(checkpointDetails.incident)}
                    </div>
                    <div style={{ fontSize: "0.8rem", marginTop: "4px", color: "#ccc" }}>
                      Severity: {checkpointDetails.incident.severity.toUpperCase()}
                    </div>
                    <div style={{ fontSize: "0.8rem", marginTop: "4px", color: "#ccc" }}>
                      Details: {checkpointDetails.incident.details}
                    </div>
                  </div>
                )}
                {checkpointDetails.message && (
                  <div
                    style={{
                      marginTop: "12px",
                      padding: "8px",
                      borderRadius: "6px",
                      border: "1px solid #333",
                      backgroundColor: "#151515",
                    }}
                  >
                    <div style={{ fontSize: "0.75rem", color: "#a6a6a6" }}>
                      Conversation ({checkpointDetails.message.role}) ·
                      {" "}
                      {new Date(checkpointDetails.message.createdAt).toLocaleString()}
                    </div>
                    <div
                      style={{
                        marginTop: "4px",
                        whiteSpace: "pre-wrap",
                        wordBreak: "break-word",
                        lineHeight: 1.4,
                      }}
                    >
                      {checkpointDetails.message.body}
                    </div>
                  </div>
                )}
                <PayloadViewer
                  label="Prompt"
                  raw={checkpointDetails.promptPayload ?? null}
                  canonical={promptCanonical}
                  digestItems={[
                    { label: "SHA-256", value: checkpointDetails.inputsSha256 ?? null },
                  ]}
                  viewMode={promptViewMode}
                  onChangeMode={setPromptViewMode}
                  downloadBaseName={checkpointDetails.id}
                />
                <PayloadViewer
                  label="Output"
                  raw={checkpointDetails.outputPayload ?? null}
                  canonical={outputCanonical}
                  digestItems={[
                    { label: "SHA-256", value: checkpointDetails.outputsSha256 ?? null },
                    {
                      label: "Semantic Digest",
                      value: checkpointDetails.semanticDigest ?? null,
                    },
                  ]}
                  viewMode={outputViewMode}
                  onChangeMode={setOutputViewMode}
                  downloadBaseName={checkpointDetails.id}
                />
              </div>
            )}
          </aside>
        </>
      )}
    </div>
  );
}