import React from "react";
import { RunStepConfig } from "../lib/api";
import {
  buttonGhost,
  buttonSecondary,
  buttonDanger,
  buttonPrimary,
  buttonDisabled,
  combineButtonStyles,
} from "../styles/common.js";

interface CheckpointListItemProps {
  config: RunStepConfig;
  onEdit: (config: RunStepConfig) => void;
  onDelete: (config: RunStepConfig) => void;
  onMoveUp: () => void;
  onMoveDown: () => void;
  isFirst: boolean;
  isLast: boolean;
  onOpenInteractive?: (config: RunStepConfig) => void;
}

function truncatePrompt(value: string, length = 160): string {
  const normalized = value.replace(/\s+/g, " ").trim();
  if (normalized.length <= length) {
    return normalized;
  }
  return `${normalized.slice(0, length)}…`;
}

export default function CheckpointListItem({
  config,
  onEdit,
  onDelete,
  onMoveUp,
  onMoveDown,
  isFirst,
  isLast,
  onOpenInteractive,
}: CheckpointListItemProps) {
  const orderLabel = config.orderIndex + 1;
  const isDocumentIngestion = config.stepType === 'document_ingestion' || config.stepType === 'ingest';
  const isInteractive = config.checkpointType.trim().toLowerCase() === 'interactivechat'.toLowerCase();

  // Generate preview based on step type
  let promptPreview = '';
  if (isDocumentIngestion && config.configJson) {
    try {
      const docConfig = JSON.parse(config.configJson);
      promptPreview = `Document: ${docConfig.sourcePath || docConfig.source_path} (${docConfig.format})`;
    } catch {
      promptPreview = 'Document ingestion';
    }
  } else if (config.stepType === 'summarize' && config.configJson) {
    try {
      const summaryConfig = JSON.parse(config.configJson);
      const sourceStepLabel = summaryConfig.sourceStep !== undefined ? `Step ${summaryConfig.sourceStep + 1}` : 'unknown';
      promptPreview = `Summarize ${sourceStepLabel} (${summaryConfig.summaryType || 'brief'})`;
    } catch {
      promptPreview = 'Summarize previous step';
    }
  } else if (config.stepType === 'prompt' && config.configJson) {
    try {
      const promptConfig = JSON.parse(config.configJson);
      const contextLabel = promptConfig.useOutputFrom !== undefined && promptConfig.useOutputFrom !== null
        ? ` (with context from Step ${promptConfig.useOutputFrom + 1})`
        : '';
      promptPreview = truncatePrompt(promptConfig.prompt) + contextLabel;
    } catch {
      promptPreview = config.prompt ? truncatePrompt(config.prompt) : '(no prompt)';
    }
  } else if (config.prompt) {
    promptPreview = truncatePrompt(config.prompt);
  } else {
    promptPreview = '(no prompt)';
  }

  return (
    <div
      style={{
        border: "1px solid #333",
        borderRadius: "8px",
        padding: "12px",
        backgroundColor: "#1f1f1f",
        display: "flex",
        flexDirection: "column",
        gap: "8px",
      }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline" }}>
        <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
          <span style={{ fontSize: "0.75rem", color: "#9cdcfe", letterSpacing: "0.05em" }}>
            Step {orderLabel}
          </span>
          <span style={{ fontSize: "1.05rem", fontWeight: 600 }}>{config.checkpointType}</span>
        </div>
        <div style={{ display: "flex", gap: "6px" }}>
          <button
            type="button"
            onClick={() => onMoveUp()}
            disabled={isFirst}
            title="Move up"
            style={combineButtonStyles(
              buttonGhost,
              { padding: "2px 6px", fontSize: "0.85rem" },
              isFirst && buttonDisabled,
            )}
          >
            ↑
          </button>
          <button
            type="button"
            onClick={() => onMoveDown()}
            disabled={isLast}
            title="Move down"
            style={combineButtonStyles(
              buttonGhost,
              { padding: "2px 6px", fontSize: "0.85rem" },
              isLast && buttonDisabled,
            )}
          >
            ↓
          </button>
        </div>
      </div>
      <div style={{ display: "flex", flexWrap: "wrap", gap: "12px", fontSize: "0.9rem" }}>
        {isDocumentIngestion ? (
          <>
            {config.configJson && (() => {
              try {
                const docConfig = JSON.parse(config.configJson);
                return (
                  <>
                    <span>
                      <strong>Format:</strong> {docConfig.format?.toUpperCase() || 'Unknown'}
                    </span>
                    <span>
                      <strong>Privacy:</strong> {docConfig.privacyStatus || docConfig.privacy_status || 'Unknown'}
                    </span>
                  </>
                );
              } catch {
                return null;
              }
            })()}
          </>
        ) : config.stepType === 'summarize' ? (
          <>
            {config.configJson && (() => {
              try {
                const summaryConfig = JSON.parse(config.configJson);
                return (
                  <>
                    <span>
                      <strong>Model:</strong> {summaryConfig.model || config.model || 'Unknown'}
                    </span>
                    <span>
                      <strong>Type:</strong> {summaryConfig.summaryType || 'brief'}
                    </span>
                    <span>
                      <strong>Token Budget:</strong> {config.tokenBudget.toLocaleString()}
                    </span>
                  </>
                );
              } catch {
                return null;
              }
            })()}
          </>
        ) : config.stepType === 'prompt' ? (
          <>
            {config.configJson && (() => {
              try {
                const promptConfig = JSON.parse(config.configJson);
                return (
                  <>
                    <span>
                      <strong>Model:</strong> {promptConfig.model || config.model || 'Unknown'}
                    </span>
                    <span>
                      <strong>Token Budget:</strong> {config.tokenBudget.toLocaleString()}
                    </span>
                    {promptConfig.useOutputFrom !== undefined && promptConfig.useOutputFrom !== null && (
                      <span>
                        <strong>Uses:</strong> Step {promptConfig.useOutputFrom + 1}
                      </span>
                    )}
                  </>
                );
              } catch {
                return null;
              }
            })()}
          </>
        ) : (
          <>
            <span>
              <strong>Model:</strong> {config.model || 'Unknown'}
            </span>
            <span>
              <strong>Token Budget:</strong> {config.tokenBudget.toLocaleString()}
            </span>
            <span>
              <strong>Proof Mode:</strong> {config.proofMode === "concordant" ? "Concordant" : "Exact"}
            </span>
            {config.proofMode === "concordant" && (
              <span>
                <strong>Epsilon:</strong> {typeof config.epsilon === "number" ? config.epsilon.toFixed(3) : "—"}
              </span>
            )}
          </>
        )}
      </div>
      <div style={{ fontSize: "0.85rem", color: "#c8c8c8" }}>{promptPreview}</div>
      <div style={{ display: "flex", gap: "8px" }}>
        <button type="button" onClick={() => onEdit(config)} style={buttonSecondary}>
          Edit
        </button>
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onDelete(config);
          }}
          style={buttonDanger}
        >
          Delete
        </button>
        {isInteractive && onOpenInteractive && (
          <button
            type="button"
            onClick={() => onOpenInteractive(config)}
            style={buttonPrimary}
          >
            Open Chat
          </button>
        )}
      </div>
    </div>
  );
}
