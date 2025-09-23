import React from "react";
import type { RunProofMode } from "../lib/api";

export interface CheckpointFormValue {
  checkpointType: string;
  model: string;
  tokenBudget: number;
  prompt: string;
  proofMode: RunProofMode;
}

interface CheckpointEditorProps {
  availableModels: string[];
  initialValue?: CheckpointFormValue;
  mode: "create" | "edit";
  onSubmit: (value: CheckpointFormValue) => Promise<void> | void;
  onCancel: () => void;
  submitting?: boolean;
}

function sanitizeLabel(value: string): string {
  return value.replace(/\u0000/g, "").replace(/\s+/g, " ").trim();
}

function sanitizePrompt(value: string): string {
  return value.replace(/\u0000/g, "").replace(/\r\n/g, "\n").trim();
}

export default function CheckpointEditor({
  availableModels,
  initialValue,
  mode,
  onSubmit,
  onCancel,
  submitting = false,
}: CheckpointEditorProps) {
  const mergedModels = React.useMemo(() => {
    const set = new Set<string>(availableModels);
    if (initialValue?.model) {
      set.add(initialValue.model);
    }
    return Array.from(set);
  }, [availableModels, initialValue?.model]);

  const defaultModel = mergedModels[0] ?? "stub-model";

  const [checkpointType, setCheckpointType] = React.useState(
    initialValue?.checkpointType ?? "Step",
  );
  const [model, setModel] = React.useState(initialValue?.model ?? defaultModel);
  const [tokenBudget, setTokenBudget] = React.useState(
    initialValue ? String(initialValue.tokenBudget) : "1000",
  );
  const [prompt, setPrompt] = React.useState(initialValue?.prompt ?? "");
  const [proofMode, setProofMode] = React.useState<RunProofMode>(
    initialValue?.proofMode ?? "exact",
  );
  const [error, setError] = React.useState<string | null>(null);

  React.useEffect(() => {
    setCheckpointType(initialValue?.checkpointType ?? "Step");
    setModel(initialValue?.model ?? defaultModel);
    setTokenBudget(initialValue ? String(initialValue.tokenBudget) : "1000");
    setPrompt(initialValue?.prompt ?? "");
    setProofMode(initialValue?.proofMode ?? "exact");
    setError(null);
  }, [initialValue, defaultModel]);

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (submitting) {
      return;
    }

    const cleanedType = sanitizeLabel(checkpointType || "Step");
    if (!cleanedType) {
      setError("Checkpoint type is required.");
      return;
    }

    const cleanedModel = sanitizeLabel(model || defaultModel);
    if (!cleanedModel) {
      setError("Model is required.");
      return;
    }

    const parsedBudget = Number.parseInt(tokenBudget, 10);
    if (!Number.isFinite(parsedBudget) || parsedBudget < 0) {
      setError("Token budget must be a non-negative integer.");
      return;
    }

    const cleanedPrompt = sanitizePrompt(prompt);
    if (!cleanedPrompt) {
      setError("Prompt text is required.");
      return;
    }

    if (!proofMode || (proofMode !== "exact" && proofMode !== "concordant")) {
      setError("Proof mode selection is required.");
      return;
    }

    setError(null);
    await onSubmit({
      checkpointType: cleanedType,
      model: cleanedModel,
      tokenBudget: parsedBudget,
      prompt: cleanedPrompt,
      proofMode,
    });
  };

  const headerLabel = mode === "create" ? "Add Checkpoint" : "Edit Checkpoint";
  const submitLabel = mode === "create" ? "Create" : "Save";

  return (
    <form
      onSubmit={handleSubmit}
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "12px",
        padding: "12px",
        border: "1px solid #333",
        borderRadius: "8px",
        backgroundColor: "#202020",
      }}
    >
      <div style={{ fontSize: "1rem", fontWeight: 600 }}>{headerLabel}</div>
      <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
        Checkpoint Type
        <input
          type="text"
          value={checkpointType}
          onChange={(event) => setCheckpointType(event.target.value)}
          placeholder="Step"
        />
      </label>
      <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
        Model
        <select value={model} onChange={(event) => setModel(event.target.value)}>
          {mergedModels.map((modelId) => (
            <option key={modelId} value={modelId}>
              {modelId}
            </option>
          ))}
        </select>
      </label>
      <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
        Token Budget
        <input
          type="number"
          min={0}
          step={1}
          value={tokenBudget}
          onChange={(event) => setTokenBudget(event.target.value)}
        />
      </label>
      <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
        Proof Mode
        <select
          value={proofMode}
          onChange={(event) => {
            const nextValue = event.target.value === "concordant" ? "concordant" : "exact";
            setProofMode(nextValue);
          }}
        >
          <option value="exact">Exact</option>
          <option value="concordant">Concordant</option>
        </select>
      </label>
      <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
        Prompt
        <textarea
          value={prompt}
          onChange={(event) => setPrompt(event.target.value)}
          rows={6}
          style={{ fontFamily: "monospace" }}
        />
      </label>
      {error && <div style={{ color: "#f48771", fontSize: "0.85rem" }}>{error}</div>}
      <div style={{ display: "flex", gap: "8px" }}>
        <button type="submit" disabled={submitting}>
          {submitting ? "Saving…" : submitLabel}
        </button>
        <button type="button" onClick={onCancel} disabled={submitting}>
          Cancel
        </button>
      </div>
    </form>
  );
}
