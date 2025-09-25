import React from "react";
import { CheckpointDetails, IncidentSummary } from "../lib/api";

type PayloadViewMode = "raw" | "canonical" | "digest";

interface DigestItem {
  label: string;
  value?: string | null;
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

function safeFileName(base: string): string {
  return base.replace(/[^a-z0-9._-]+/gi, "_");
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

export function formatIncidentMessage(incident?: IncidentSummary | null): string {
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

export function incidentSeverityColor(incident?: IncidentSummary | null): string {
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

export interface CheckpointDetailsPanelProps {
  open: boolean;
  onClose: () => void;
  checkpointDetails: CheckpointDetails | null;
  loading: boolean;
  error: string | null;
  title?: string;
  subtitle?: React.ReactNode;
  children?: React.ReactNode;
}

export default function CheckpointDetailsPanel({
  open,
  onClose,
  checkpointDetails,
  loading,
  error,
  title = "Checkpoint Details",
  subtitle,
  children,
}: CheckpointDetailsPanelProps) {
  const [promptViewMode, setPromptViewMode] = React.useState<PayloadViewMode>("raw");
  const [outputViewMode, setOutputViewMode] = React.useState<PayloadViewMode>("raw");

  const promptCanonical = React.useMemo(
    () => canonicalizeJsonText(checkpointDetails?.promptPayload ?? null),
    [checkpointDetails?.promptPayload],
  );
  const outputCanonical = React.useMemo(
    () => canonicalizeJsonText(checkpointDetails?.outputPayload ?? null),
    [checkpointDetails?.outputPayload],
  );

  React.useEffect(() => {
    if (!open) {
      setPromptViewMode("raw");
      setOutputViewMode("raw");
      return;
    }
    setPromptViewMode(checkpointDetails?.promptPayload != null ? "raw" : "digest");
    setOutputViewMode(checkpointDetails?.outputPayload != null ? "raw" : "digest");
  }, [checkpointDetails?.promptPayload, checkpointDetails?.outputPayload, open]);

  const headerSubtitle = subtitle ?? checkpointDetails?.id ?? null;

  if (!open) {
    return null;
  }

  return (
    <>
      <div
        role="presentation"
        onClick={onClose}
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
        aria-label={title}
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
            <h3 style={{ margin: 0 }}>{title}</h3>
            {headerSubtitle && (
              <div
                style={{
                  fontSize: "0.75rem",
                  color: "#9cdcfe",
                  marginTop: "4px",
                  fontFamily: "monospace",
                  wordBreak: "break-all",
                }}
              >
                {headerSubtitle}
              </div>
            )}
          </div>
          <button type="button" onClick={onClose}>
            Close
          </button>
        </div>
        {loading && <p>Loading details…</p>}
        {error && !loading && <p style={{ color: "#f48771" }}>{error}</p>}
        {!loading && !error && checkpointDetails && (
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
              <dt>Execution</dt>
              <dd
                style={{
                  margin: 0,
                  fontFamily: "monospace",
                  wordBreak: "break-all",
                }}
              >
                {checkpointDetails.executionId}
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
                  Conversation ({checkpointDetails.message.role}) · {" "}
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
        {children && (
          <div style={{ marginTop: 24 }}>{children}</div>
        )}
      </aside>
    </>
  );
}
