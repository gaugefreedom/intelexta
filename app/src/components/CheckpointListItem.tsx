import React from "react";
import { RunCheckpointConfig } from "../lib/api";

interface CheckpointListItemProps {
  config: RunCheckpointConfig;
  onEdit: (config: RunCheckpointConfig) => void;
  onDelete: (config: RunCheckpointConfig) => void;
  onMoveUp: () => void;
  onMoveDown: () => void;
  isFirst: boolean;
  isLast: boolean;
  onOpenInteractive?: (config: RunCheckpointConfig) => void;
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
  const promptPreview = truncatePrompt(config.prompt);
  const isInteractive = config.checkpointType.trim().toLowerCase() === 'interactivechat'.toLowerCase();

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
          <button type="button" onClick={() => onMoveUp()} disabled={isFirst} title="Move up">
            ↑
          </button>
          <button type="button" onClick={() => onMoveDown()} disabled={isLast} title="Move down">
            ↓
          </button>
        </div>
      </div>
      <div style={{ display: "flex", flexWrap: "wrap", gap: "12px", fontSize: "0.9rem" }}>
        <span>
          <strong>Model:</strong> {config.model}
        </span>
        <span>
          <strong>Token Budget:</strong> {config.tokenBudget.toLocaleString()}
        </span>
      </div>
      <div style={{ fontSize: "0.85rem", color: "#c8c8c8" }}>{promptPreview}</div>
      <div style={{ display: "flex", gap: "8px" }}>
        <button type="button" onClick={() => onEdit(config)}>
          Edit
        </button>
        <button
          type="button"
          onClick={() => onDelete(config)}
          style={{ color: "#f48771" }}
        >
          Delete
        </button>
        {isInteractive && onOpenInteractive && (
          <button type="button" onClick={() => onOpenInteractive(config)}>
            Open Chat
          </button>
        )}
      </div>
    </div>
  );
}
