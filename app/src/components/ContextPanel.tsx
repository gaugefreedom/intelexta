import React from "react";
import {
  getPolicy,
  updatePolicy,
  Policy,
  estimateRunCost,
  type RunCostEstimates,
  exportProject,
  importProject,
  importCar,
  getCheckpointDetails,
  type ProjectImportSummary,
  type ReplayReport,
  type ImportedCarSnapshot,
  type CheckpointDetails,
  ProofBadgeKind,
} from "../lib/api.js";
import {
  buttonPrimary,
  buttonSecondary,
  buttonDisabled,
  combineButtonStyles,
} from "../styles/common.js";
import CheckpointDetailsPanel from "./CheckpointDetailsPanel.js";

export interface CarCheckpointRow {
  id: string;
  order: number;
  turnIndex?: number | null;
  currChain?: string | null;
  prevChain?: string | null;
  signature?: string | null;
}

export function buildCarCheckpointRows(
  snapshot: ImportedCarSnapshot,
): CarCheckpointRow[] {
  const seen = new Set<string>();
  const rows: CarCheckpointRow[] = [];
  for (const checkpoint of snapshot.checkpoints) {
    if (seen.has(checkpoint.id)) {
      continue;
    }
    seen.add(checkpoint.id);
    rows.push({
      id: checkpoint.id,
      order: rows.length + 1,
      turnIndex: checkpoint.turnIndex ?? null,
      currChain: checkpoint.currChain ?? null,
      prevChain: checkpoint.prevChain ?? null,
      signature: checkpoint.signature ?? null,
    });
  }
  return rows;
}

interface ContextPanelProps {
  projectId: string;
  selectedRunId: string | null;
  onPolicyUpdated?: () => void;
}

export default function ContextPanel({
  projectId,
  selectedRunId,
  onPolicyUpdated,
}: ContextPanelProps) {
  const [policy, setPolicy] = React.useState<Policy | null>(null);
  const [loading, setLoading] = React.useState<boolean>(true);
  const [saving, setSaving] = React.useState<boolean>(false);
  const [status, setStatus] = React.useState<string | null>(null);
  const [error, setError] = React.useState<string | null>(null);

  const [costEstimates, setCostEstimates] = React.useState<RunCostEstimates | null>(null);
  const [costLoading, setCostLoading] = React.useState<boolean>(false);
  const [costError, setCostError] = React.useState<string | null>(null);
  const [costRefreshToken, setCostRefreshToken] = React.useState(0);

  const [exportingProject, setExportingProject] = React.useState<boolean>(false);
  const [exportStatus, setExportStatus] = React.useState<string | null>(null);
  const [exportError, setExportError] = React.useState<string | null>(null);
  const [importingProjectArchive, setImportingProjectArchive] = React.useState<boolean>(false);
  const [projectImportStatus, setProjectImportStatus] = React.useState<string | null>(null);
  const [projectImportError, setProjectImportError] = React.useState<string | null>(null);
  const [lastImportSummary, setLastImportSummary] = React.useState<ProjectImportSummary | null>(null);
  const [importingCarReceipt, setImportingCarReceipt] = React.useState<boolean>(false);
  const [carImportError, setCarImportError] = React.useState<string | null>(null);
  const [carReplayReport, setCarReplayReport] = React.useState<ReplayReport | null>(null);
  const [carImportStatus, setCarImportStatus] = React.useState<string | null>(null);
  const [carSnapshot, setCarSnapshot] = React.useState<ImportedCarSnapshot | null>(null);
  const [carCheckpointDetailsOpen, setCarCheckpointDetailsOpen] = React.useState<boolean>(false);
  const [carCheckpointDetailsId, setCarCheckpointDetailsId] = React.useState<string | null>(null);
  const [carCheckpointDetails, setCarCheckpointDetails] = React.useState<CheckpointDetails | null>(null);
  const [carCheckpointDetailsLoading, setCarCheckpointDetailsLoading] = React.useState<boolean>(false);
  const [carCheckpointDetailsError, setCarCheckpointDetailsError] = React.useState<string | null>(null);

  const projectFileInputRef = React.useRef<HTMLInputElement | null>(null);
  const carFileInputRef = React.useRef<HTMLInputElement | null>(null);

  const costOverrunMessages = React.useMemo(() => {
    if (!costEstimates) {
      return [] as string[];
    }
    const messages: string[] = [];
    if (costEstimates.exceedsTokens) {
      messages.push(
        `Tokens: ${costEstimates.estimatedTokens.toLocaleString()} / ${costEstimates.budgetTokens.toLocaleString()}`,
      );
    }
    if (costEstimates.exceedsUsd) {
      messages.push(
        `USD: ${costEstimates.estimatedUsd.toFixed(2)} / ${costEstimates.budgetUsd.toFixed(2)}`,
      );
    }
    if (costEstimates.exceedsNatureCost) {
      messages.push(
        `Nature Cost: ${costEstimates.estimatedNatureCost.toFixed(2)} / ${costEstimates.budgetNatureCost.toFixed(2)}`,
      );
    }
    return messages;
  }, [costEstimates]);

  React.useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setStatus(null);
    setError(null);
    getPolicy(projectId)
      .then((policyData) => {
        if (!cancelled) {
          setPolicy(policyData);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          console.error("Failed to load policy", err);
          setError("Could not load policy settings.");
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [projectId]);

  React.useEffect(() => {
    setExportStatus(null);
    setExportError(null);
    setProjectImportStatus(null);
    setProjectImportError(null);
    setLastImportSummary(null);
    setCarImportError(null);
    setCarReplayReport(null);
    setCarImportStatus(null);
    setCarSnapshot(null);
  }, [projectId]);

  React.useEffect(() => {
    if (!selectedRunId) {
      setCostEstimates(null);
      setCostError(null);
      setCostLoading(false);
      return;
    }

    let cancelled = false;
    setCostLoading(true);
    setCostError(null);

    estimateRunCost(selectedRunId)
      .then((estimates) => {
        if (!cancelled) {
          setCostEstimates(estimates);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          console.error("Failed to estimate run cost", err);
          const message =
            err instanceof Error ? err.message : "Unable to estimate projected run cost.";
          setCostError(message);
          setCostEstimates(null);
        }
      })
      .finally(() => {
        if (!cancelled) {
          setCostLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [selectedRunId, projectId, costRefreshToken, policy]);

  const handleToggle = (event: React.ChangeEvent<HTMLInputElement>) => {
    setPolicy((prev) => (prev ? { ...prev, allowNetwork: event.target.checked } : prev));
    setStatus(null);
  };

  const handleNumberChange = <K extends keyof Policy>(field: K) => (
    event: React.ChangeEvent<HTMLInputElement>
  ) => {
    const value = event.target.valueAsNumber;
    setPolicy((prev) => {
      if (!prev) return prev;
      if (Number.isNaN(value)) {
        return prev;
      }
      return { ...prev, [field]: value };
    });
    setStatus(null);
  };

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!policy) return;

    setSaving(true);
    setStatus(null);
    setError(null);

    try {
      await updatePolicy(projectId, policy);
      const refreshed = await getPolicy(projectId);
      setPolicy(refreshed);
      setStatus("Policy saved successfully.");
      setCostRefreshToken((token) => token + 1);
      onPolicyUpdated?.();
    } catch (err) {
      console.error("Failed to save policy", err);
      setError(`Could not save policy: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setSaving(false);
    }
  };

  const handleExportProject = React.useCallback(async () => {
    setExportError(null);
    setExportStatus(null);
    setExportingProject(true);
    try {
      const path = await exportProject(projectId);
      setExportStatus(`Export saved to ${path}`);
    } catch (err) {
      console.error("Failed to export project", err);
      setExportError(
        `Failed to export project: ${err instanceof Error ? err.message : String(err)}`,
      );
    } finally {
      setExportingProject(false);
    }
  }, [projectId]);

  const handleImportProjectArchive = React.useCallback(() => {
    setProjectImportError(null);
    setProjectImportStatus(null);
    setLastImportSummary(null);

    const input = projectFileInputRef.current;
    if (input) {
      input.value = "";
      input.click();
    } else {
      setProjectImportError("File picker not available.");
    }
  }, []);

  const handleProjectFileSelected = React.useCallback(
    async (event: React.ChangeEvent<HTMLInputElement>) => {
      const file = event.target.files?.[0] ?? null;
      event.target.value = "";

      if (!file) {
        return;
      }

      setProjectImportError(null);
      setProjectImportStatus(null);
      setLastImportSummary(null);
      setImportingProjectArchive(true);

      try {
        const buffer = await file.arrayBuffer();
        const bytes = Array.from(new Uint8Array(buffer));
        const summary = await importProject({
          fileName: file.name,
          bytes,
        });
        setLastImportSummary(summary);
        setProjectImportStatus(
          `Imported project ${summary.project.name} (${summary.project.id}).`,
        );
        onPolicyUpdated?.();
      } catch (err) {
        console.error("Failed to import project", err);
        setProjectImportError(
          `Failed to import project archive: ${err instanceof Error ? err.message : String(err)}`,
        );
      } finally {
        setImportingProjectArchive(false);
      }
    },
    [onPolicyUpdated],
  );

  const resetCarCheckpointDetails = React.useCallback(() => {
    setCarCheckpointDetailsOpen(false);
    setCarCheckpointDetailsId(null);
    setCarCheckpointDetails(null);
    setCarCheckpointDetailsError(null);
    setCarCheckpointDetailsLoading(false);
  }, []);

  const handleImportCarReceipt = React.useCallback(() => {
    setCarImportError(null);
    setCarReplayReport(null);
    setCarImportStatus(null);
    setCarSnapshot(null);
    resetCarCheckpointDetails();

    const input = carFileInputRef.current;
    if (input) {
      input.value = "";
      input.click();
    } else {
      setCarImportError("File picker not available.");
    }
  }, [resetCarCheckpointDetails]);

  const handleCarFileSelected = React.useCallback(
    async (event: React.ChangeEvent<HTMLInputElement>) => {
      const file = event.target.files?.[0] ?? null;
      event.target.value = "";

      if (!file) {
        return;
      }

      setCarImportError(null);
      setCarReplayReport(null);
      setCarImportStatus(null);
      setImportingCarReceipt(true);
      resetCarCheckpointDetails();

      try {
        const buffer = await file.arrayBuffer();
        const bytes = Array.from(new Uint8Array(buffer));
        const result = await importCar({
          fileName: file.name,
          bytes,
        });
        setCarReplayReport(result.replayReport);
        setCarSnapshot({
          ...result.snapshot,
          checkpoints: [...result.snapshot.checkpoints],
        });
        setCarImportStatus(
          `Imported CAR ${result.snapshot.carId} for run ${result.snapshot.runId}.`,
        );
        onPolicyUpdated?.();
      } catch (err) {
        console.error("Failed to import CAR", err);
        setCarImportError(
          `Failed to import CAR: ${err instanceof Error ? err.message : String(err)}`,
        );
      } finally {
        setImportingCarReceipt(false);
      }
    },
    [onPolicyUpdated, resetCarCheckpointDetails],
  );

  const carReplayFeedback = React.useMemo(() => {
    if (!carReplayReport) {
      return null;
    }

    const checkpoints = carReplayReport.checkpointReports ?? [];

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
      const configuredMode: ProofBadgeKind | undefined = entry.proofMode ?? entry.mode ?? undefined;
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
        const modeLabel = configuredMode
          ? configuredMode.charAt(0).toUpperCase() + configuredMode.slice(1)
          : "Checkpoint";
        message = `${modeLabel} ${label}: FAIL — ${details} (expected ${expected}, replayed ${replayed})`;
      }
      return {
        key: entry.checkpointConfigId ?? `checkpoint-${index}`,
        tone,
        message,
      };
    };

    if (checkpoints.length === 0) {
      const base = carReplayReport.matchStatus ? "Replay PASS" : "Replay FAIL";
      const overallMessage = carReplayReport.errorMessage
        ? `${base} — ${carReplayReport.errorMessage}`
        : base;
      return {
        overallTone: carReplayReport.matchStatus ? "success" : "error",
        overallMessage,
        checkpoints: [] as { key: string; tone: "success" | "error"; message: string }[],
      };
    }

    const checkpointMessages = checkpoints.map((entry, index) =>
      buildMessageForCheckpoint(entry, index),
    );

    const summaryBase = carReplayReport.matchStatus ? "Replay PASS" : "Replay FAIL";
    const overallMessage = carReplayReport.matchStatus
      ? summaryBase
      : carReplayReport.errorMessage
      ? `${summaryBase} — ${carReplayReport.errorMessage}`
      : summaryBase;

    return {
      overallTone: carReplayReport.matchStatus ? "success" : "error",
      overallMessage,
      checkpoints: checkpointMessages,
    };
  }, [carReplayReport]);

  const carCheckpointRows = React.useMemo(() => {
    if (!carSnapshot) {
      return [] as CarCheckpointRow[];
    }
    return buildCarCheckpointRows(carSnapshot);
  }, [carSnapshot]);

  React.useEffect(() => {
    if (!carCheckpointDetailsOpen) {
      return;
    }
    if (!carCheckpointDetailsId) {
      setCarCheckpointDetailsLoading(false);
      setCarCheckpointDetails(null);
      setCarCheckpointDetailsError(null);
      return;
    }

    let cancelled = false;
    setCarCheckpointDetailsLoading(true);
    setCarCheckpointDetailsError(null);
    setCarCheckpointDetails(null);

    getCheckpointDetails(carCheckpointDetailsId)
      .then((details) => {
        if (cancelled) {
          return;
        }
        setCarCheckpointDetails(details);
      })
      .catch((err) => {
        if (cancelled) {
          return;
        }
        console.error("Failed to load checkpoint from CAR", err);
        setCarCheckpointDetailsError(
          err instanceof Error ? err.message : "Could not load checkpoint details.",
        );
      })
      .finally(() => {
        if (!cancelled) {
          setCarCheckpointDetailsLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [carCheckpointDetailsOpen, carCheckpointDetailsId]);

  React.useEffect(() => {
    resetCarCheckpointDetails();
  }, [carSnapshot?.carId, resetCarCheckpointDetails]);

  const handleOpenCarCheckpointDetails = React.useCallback((checkpointId: string) => {
    setCarCheckpointDetailsId(checkpointId);
    setCarCheckpointDetailsOpen(true);
  }, []);

  const handleCloseCarCheckpointDetails = React.useCallback(() => {
    setCarCheckpointDetailsOpen(false);
    setCarCheckpointDetailsId(null);
  }, []);

  return (
    <div>
      <h2>Context</h2>

      {/* --- Project ID with Truncation --- */}
      <div style={{ marginBottom: '16px' }}>
        <div style={{ fontSize: '0.7rem', color: '#808080' }}>Project</div>
        <div
          style={{
            fontFamily: 'monospace',
            fontSize: '0.8rem',
            color: '#9cdcfe',
            whiteSpace: 'nowrap',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
          }}
          title={projectId} // Show full ID on hover
        >
          {projectId}
        </div>
      </div>

      {/* --- Cost Overrun Warning (no changes needed here) --- */}
      {selectedRunId && (
        <div style={{ marginBottom: "0.75rem" }}>
          {costLoading && <div style={{ color: "#9cdcfe" }}>Estimating projected run costs…</div>}
          {costError && <div style={{ color: "#f48771" }}>{costError}</div>}
          {costOverrunMessages.length > 0 && costEstimates && (
            <div
              style={{
                backgroundColor: "#402020",
                border: "1px solid #7f1d1d",
                color: "#f8caca",
                padding: "10px",
                borderRadius: "6px",
                fontSize: "0.85rem",
                display: "flex", flexDirection: "column", gap: "6px",
              }}
            >
              <div style={{ fontWeight: 600, color: "#f48771" }}>
                Selected run exceeds policy budgets.
              </div>
              <ul style={{ margin: 0, paddingLeft: "18px" }}>
                {costOverrunMessages.map((message) => (
                  <li key={message} style={{ marginBottom: "2px" }}>{message}</li>
                ))}
              </ul>
            </div>
          )}
        </div>
      )}

      {/* --- Policy Form (stacked vertically for a narrow layout) --- */}
      {loading ? (
        <p>Loading policy…</p>
      ) : policy ? (
        <form onSubmit={handleSubmit} style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
          <label style={{ display: "flex", alignItems: "center", gap: "8px" }}>
            <input type="checkbox" checked={policy.allowNetwork} onChange={handleToggle} />
            Allow network access
          </label>

          <div style={{ display: "flex", flexDirection: 'column', gap: "8px" }}>
            <label style={{ display: "flex", flexDirection: 'column', gap: '4px', fontSize: '0.8rem' }}>
              Token Budget
              <input
                type="number"
                min={0} step={1}
                value={policy.budgetTokens}
                onChange={handleNumberChange("budgetTokens")}
              />
            </label>
            <label style={{ display: "flex", flexDirection: 'column', gap: '4px', fontSize: '0.8rem' }}>
              USD Budget
              <input
                type="number"
                min={0} step={0.01}
                value={policy.budgetUsd}
                onChange={handleNumberChange("budgetUsd")}
              />
            </label>
            <label style={{ display: "flex", flexDirection: 'column', gap: '4px', fontSize: '0.8rem' }}>
              Nature Cost
              <input
                type="number"
                min={0} step={0.01}
                value={policy.budgetNatureCost}
                onChange={handleNumberChange("budgetNatureCost")}
              />
            </label>
          </div>

          <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
            <button
              type="submit"
              disabled={saving}
              style={combineButtonStyles(buttonPrimary, saving && buttonDisabled)}
            >
              {saving ? "Saving…" : "Save Policy"}
            </button>
            {status && <span style={{ color: "#6A9955", fontSize: '0.8rem' }}>{status}</span>}
            {error && <span style={{ color: "#f48771", fontSize: '0.8rem' }}>{error}</span>}
          </div>
        </form>
      ) : (
        <p>No policy found for this project.</p>
      )}

      {/* --- Portability (buttons stacked vertically) --- */}
      <div
        style={{
          marginTop: "1.5rem",
          paddingTop: "1rem",
          borderTop: "1px solid #333",
        }}
      >
        <h3 style={{ margin: 0, marginBottom: '12px' }}>Portability</h3>
        <input ref={projectFileInputRef} type="file" accept=".ixp" style={{ display: "none" }} onChange={handleProjectFileSelected} />
        <input ref={carFileInputRef} type="file" accept=".car.json,.json" style={{ display: "none" }} onChange={handleCarFileSelected} />

        {/* This div stacks the buttons vertically */}
        <div style={{ display: "flex", flexDirection: "column", gap: "8px", alignItems: 'stretch' }}>
          <button
            type="button"
            onClick={handleExportProject}
            disabled={exportingProject}
            style={combineButtonStyles(buttonSecondary, exportingProject && buttonDisabled)}
          >
            {exportingProject ? "Exporting…" : "Export Project"}
          </button>
          <button
            type="button"
            onClick={handleImportProjectArchive}
            disabled={importingProjectArchive}
            style={combineButtonStyles(buttonSecondary, importingProjectArchive && buttonDisabled)}
          >
            {importingProjectArchive ? "Importing…" : "Import .ixp"}
          </button>
          <button
            type="button"
            onClick={handleImportCarReceipt}
            disabled={importingCarReceipt}
            style={combineButtonStyles(buttonSecondary, importingCarReceipt && buttonDisabled)}
          >
            {importingCarReceipt ? "Verifying…" : "Import CAR"}
          </button>
        </div>

        {/* All the status messages below will now format nicely in the vertical layout */}
        {exportStatus && (<span style={{ color: "#a5d6a7", fontSize: "0.85rem" }}>{exportStatus}</span>)}
        {exportError && (<span style={{ color: "#f48771", fontSize: "0.85rem" }}>{exportError}</span>)}
        {projectImportStatus && (<span style={{ color: "#a5d6a7", fontSize: "0.85rem" }}>{projectImportStatus}</span>)}
        {projectImportError && (<span style={{ color: "#f48771", fontSize: "0.85rem" }}>{projectImportError}</span>)}
        {lastImportSummary && (
          <ul style={{ margin: 0, paddingLeft: "18px", fontSize: "0.8rem", color: "#cbd5f5" }}>
            <li>Runs imported: {lastImportSummary.runsImported}</li>
            <li>Checkpoints imported: {lastImportSummary.checkpointsImported}</li>
            <li>Receipts imported: {lastImportSummary.receiptsImported}</li>
            <li>Incidents generated: {lastImportSummary.incidentsGenerated}</li>
          </ul>
        )}
        {carImportStatus && (<span style={{ color: "#a5d6a7", fontSize: "0.85rem" }}>{carImportStatus}</span>)}
        {carReplayFeedback && (
          <div style={{ fontSize: "0.85rem", display: "flex", flexDirection: "column", gap: "4px" }}>
            <span style={{ color: carReplayFeedback.overallTone === "success" ? "#a5d6a7" : "#f48771" }}>
              {carReplayFeedback.overallMessage}
            </span>
            {carReplayFeedback.checkpoints.length > 0 && (
              <ul style={{ margin: 0, paddingLeft: "1.2rem", listStyleType: "disc" }}>
                {carReplayFeedback.checkpoints.map((entry) => (
                  <li key={entry.key} style={{ color: entry.tone === "success" ? "#a5d6a7" : "#f48771" }}>
                    {entry.message}
                  </li>
                ))}
              </ul>
            )}
          </div>
        )}
        {carImportError && (<span style={{ color: "#f48771", fontSize: "0.85rem" }}>{carImportError}</span>)}
        {carReplayReport && !carReplayFeedback && (
          <span style={{ fontSize: "0.85rem", color: "#cbd5f5" }}>
            Imported CAR for run {carReplayReport.runId}.
          </span>
        )}
        {carReplayReport && carSnapshot && (
          <div style={{ marginTop: "1rem", display: "flex", flexDirection: "column", gap: "8px" }}>
            <div style={{ fontSize: "0.85rem", color: "#9cdcfe", display: "flex", flexWrap: "wrap", gap: "4px" }}>
              <span>Run {carSnapshot.run.name} ({carSnapshot.runId})</span>
              <span>· CAR {carSnapshot.carId}</span>
            </div>
            <div style={{ fontSize: "0.75rem", color: "#cbd5f5" }}>
              Generated {new Date(carSnapshot.createdAt).toLocaleString()} · Proof mode {carSnapshot.proof.matchKind}
            </div>
            {carCheckpointRows.length === 0 ? (
              <span style={{ fontSize: "0.8rem", color: "#808080" }}>
                CAR snapshot did not include checkpoint signatures.
              </span>
            ) : (
              <div
                style={{
                  maxHeight: "45vh",
                  overflowY: "auto",
                  border: "1px solid #1f2937",
                  borderRadius: "8px",
                  backgroundColor: "#111827",
                }}
              >
                <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "0.85rem" }}>
                  <thead>
                    <tr style={{ borderBottom: "1px solid #333" }}>
                      <th style={{ textAlign: "left", padding: "8px" }}>Checkpoint</th>
                    </tr>
                  </thead>
                  <tbody>
                    {carCheckpointRows.map((row) => {
                      const isSelected = carCheckpointDetailsOpen && carCheckpointDetailsId === row.id;
                      return (
                        <tr key={`${row.order}-${row.id}`} style={{ borderBottom: "1px solid #1f2937" }}>
                          <td style={{ padding: 0 }}>
                            <button
                              type="button"
                              onClick={() => handleOpenCarCheckpointDetails(row.id)}
                              style={{
                                width: "100%",
                                display: "flex",
                                alignItems: "center",
                                gap: "12px",
                                padding: "10px 12px",
                                backgroundColor: isSelected ? "#1f2937" : "transparent",
                                border: "none",
                                color: "inherit",
                                textAlign: "left",
                                cursor: "pointer",
                              }}
                            >
                              <span style={{ fontSize: "0.75rem", color: "#9ca3af" }}>#{row.order}</span>
                              <span
                                style={{
                                  fontFamily: "monospace",
                                  fontSize: "0.8rem",
                                  wordBreak: "break-all",
                                }}
                              >
                                {row.id}
                              </span>
                            </button>
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        )}
        <CheckpointDetailsPanel
          open={carCheckpointDetailsOpen}
          onClose={handleCloseCarCheckpointDetails}
          checkpointDetails={carCheckpointDetails}
          loading={carCheckpointDetailsLoading}
          error={carCheckpointDetailsError}
          title="Checkpoint from CAR"
          subtitle={carCheckpointDetailsId ?? undefined}
        />
      </div>
    </div>
  );
}
