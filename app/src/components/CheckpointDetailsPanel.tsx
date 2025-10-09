import React from "react";
import { invoke } from "@tauri-apps/api/core";
import { CheckpointDetails, IncidentSummary } from "../lib/api";
import {
  buttonGhost,
  buttonSecondary,
  buttonDisabled,
  combineButtonStyles,
} from "../styles/common.js";

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
  displayOverride?: string | null;
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
  displayOverride,
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
  let contentForCopyDownload: string;

  switch (viewMode) {
    case "raw":
      // Use displayOverride for preview, but raw for copy/download
      if (displayOverride !== undefined && displayOverride !== null && hasRawValue) {
        displayContent = displayOverride;
        contentForCopyDownload = raw as string;
      } else {
        displayContent = hasRawValue ? (raw as string) : "No raw content stored for this checkpoint.";
        contentForCopyDownload = displayContent;
      }
      break;
    case "canonical":
      displayContent = hasCanonicalValue
        ? (canonical as string)
        : "Canonical JSON view is only available for valid JSON payloads.";
      contentForCopyDownload = displayContent;
      break;
    case "digest":
    default:
      displayContent = digestContent;
      contentForCopyDownload = digestContent;
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
    return combineButtonStyles(buttonGhost, {
      fontSize: "0.7rem",
      padding: "2px 8px",
      borderColor: isActive ? "#9cdcfe" : "#333",
      backgroundColor: isActive ? "#1f2937" : "#111",
      color: enabled ? (isActive ? "#9cdcfe" : "#ccc") : "#666",
      cursor: enabled ? "pointer" : "not-allowed",
      opacity: enabled ? 1 : 0.5,
    });
  };

  const handleCopy = React.useCallback(() => {
    if (copyDisabled) {
      return;
    }
    try {
      if (typeof navigator !== "undefined" && navigator.clipboard) {
        void navigator.clipboard.writeText(contentForCopyDownload);
      } else {
        throw new Error("Clipboard API not available");
      }
    } catch (error) {
      console.error(`Failed to copy ${label} ${viewMode}`, error);
    }
  }, [copyDisabled, contentForCopyDownload, label, viewMode]);

  const handleDownload = React.useCallback(() => {
    if (copyDisabled) {
      return;
    }
    try {
      const fileName = `${safeFileName(`${downloadBaseName}-${label.toLowerCase()}-${viewMode}`)}.txt`;
      const blob = new Blob([contentForCopyDownload], {
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
  }, [copyDisabled, contentForCopyDownload, downloadBaseName, label, viewMode]);

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
        <button
          type="button"
          onClick={handleCopy}
          disabled={copyDisabled}
          style={combineButtonStyles(buttonSecondary, copyDisabled && buttonDisabled)}
        >
          Copy
        </button>
        <button
          type="button"
          onClick={handleDownload}
          disabled={copyDisabled}
          style={combineButtonStyles(buttonSecondary, copyDisabled && buttonDisabled)}
        >
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
  const [documentCleanedText, setDocumentCleanedText] = React.useState<string | null>(null);

  const promptCanonical = React.useMemo(
    () => canonicalizeJsonText(checkpointDetails?.promptPayload ?? null),
    [checkpointDetails?.promptPayload],
  );
  const outputCanonical = React.useMemo(
    () => canonicalizeJsonText(checkpointDetails?.outputPayload ?? null),
    [checkpointDetails?.outputPayload],
  );

  // Fetch and extract cleaned text for document ingestion outputs
  React.useEffect(() => {
    // Reset when changing checkpoints
    setDocumentCleanedText(null);

    if (!open || !checkpointDetails?.id) {
      return;
    }

    // Check if this is an ingest step by looking at the prompt payload
    let isIngestStep = false;
    if (checkpointDetails.promptPayload) {
      try {
        const promptParsed = JSON.parse(checkpointDetails.promptPayload);
        isIngestStep = promptParsed && typeof promptParsed === "object" && promptParsed.stepType === "ingest";
      } catch (error) {
        // Ignore parse errors
      }
    }

    // If not clearly an ingest step, skip the fetch (optimization)
    if (!isIngestStep && !checkpointDetails.outputPayload?.includes('"document_id"')) {
      return;
    }

    // Fetch the full output to get cleaned_text_with_markdown_structure
    let cancelled = false;

    invoke<string>("download_checkpoint_full_output", {
      checkpointId: checkpointDetails.id,
    })
      .then((fullOutput) => {
        if (cancelled) return;

        try {
          const parsed = JSON.parse(fullOutput);
          if (parsed && typeof parsed === "object" && "cleaned_text_with_markdown_structure" in parsed) {
            const cleanedText = parsed.cleaned_text_with_markdown_structure;
            if (typeof cleanedText === "string" && cleanedText.length > 0) {
              setDocumentCleanedText(cleanedText);
            }
          }
        } catch (error) {
          console.error('Failed to parse full output for document:', error);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          console.error('Failed to fetch full output for document:', err);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [open, checkpointDetails?.id, checkpointDetails?.promptPayload, checkpointDetails?.outputPayload]);

  // Truncate the cleaned text for display (keep full for copy/download)
  const documentCleanedTextTruncated = React.useMemo(() => {
    if (!documentCleanedText) return null;
    const MAX_DISPLAY_LENGTH = 5000;
    if (documentCleanedText.length <= MAX_DISPLAY_LENGTH) {
      return documentCleanedText;
    }
    return documentCleanedText.substring(0, MAX_DISPLAY_LENGTH) + '\n\n[... Document preview truncated. Use "Download" to get the full content ...]';
  }, [documentCleanedText]);

  React.useEffect(() => {
    if (!open) {
      setPromptViewMode("raw");
      setOutputViewMode("raw");
      setDocumentCleanedText(null);
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
          <button type="button" onClick={onClose} style={buttonSecondary}>
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
                {checkpointDetails.runExecutionId}
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
              raw={documentCleanedText ?? checkpointDetails.outputPayload ?? null}
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
              displayOverride={documentCleanedTextTruncated}
            />
            <div style={{ marginTop: "12px" }}>
              <button
                type="button"
                onClick={async () => {
                  console.log("Download Full Output clicked, checkpoint ID:", checkpointDetails.id);
                  try {
                    console.log("Calling download_checkpoint_full_output...");
                    const fullOutput = await invoke<string>("download_checkpoint_full_output", {
                      checkpointId: checkpointDetails.id,
                    });

                    console.log("Received output, length:", fullOutput.length);

                    // Trigger browser download
                    const blob = new Blob([fullOutput], { type: "text/plain" });
                    const url = URL.createObjectURL(blob);
                    const a = document.createElement("a");
                    a.href = url;
                    a.download = `checkpoint-${checkpointDetails.id}-full-output.txt`;
                    document.body.appendChild(a);
                    a.click();
                    document.body.removeChild(a);
                    URL.revokeObjectURL(url);
                    console.log("Download triggered successfully");
                  } catch (err) {
                    console.error("Download Full Output error:", err);
                    alert(`Failed to download full output: ${err}`);
                  }
                }}
                style={combineButtonStyles(buttonSecondary)}
              >
                Download Full Output
              </button>
              <div style={{ fontSize: "0.75rem", color: "#888", marginTop: "4px" }}>
                The output above shows a preview (first 1000 chars). Click to download the complete, untruncated output.
              </div>
            </div>
          </div>
        )}
        {children && (
          <div style={{ marginTop: 24 }}>{children}</div>
        )}
      </aside>
    </>
  );
}
