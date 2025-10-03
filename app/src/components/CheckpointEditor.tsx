import React from "react";
import type { RunProofMode } from "../lib/api";
import {
  buttonPrimary,
  buttonSecondary,
  buttonDisabled,
  combineButtonStyles,
} from "../styles/common.js";
import { open } from '@tauri-apps/plugin-dialog';

export interface CheckpointFormValue {
  stepType: string; // "llm" or "document_ingestion"
  checkpointType: string;
  // LLM fields
  model?: string;
  tokenBudget?: number;
  prompt?: string;
  proofMode?: RunProofMode;
  epsilon?: number | null;
  // Document ingestion fields
  sourcePath?: string;
  format?: string;
  privacyStatus?: string;
  configJson?: string;
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

export function isValidNormalizedEpsilon(value: number | null | undefined): boolean {
  if (typeof value !== "number") {
    return false;
  }
  if (!Number.isFinite(value)) {
    return false;
  }
  return value >= 0 && value <= 1;
}

export function concordantSubmissionAllowed(
  proofMode: RunProofMode,
  epsilon: number | null | undefined,
): boolean {
  if (proofMode !== "concordant") {
    return true;
  }
  return isValidNormalizedEpsilon(epsilon);
}

const DEFAULT_CONCORDANT_EPSILON = 0.1;

function clampNormalizedEpsilon(value: number): number {
  if (!Number.isFinite(value)) {
    return DEFAULT_CONCORDANT_EPSILON;
  }
  if (value < 0) {
    return 0;
  }
  if (value > 1) {
    return 1;
  }
  return value;
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

  const [stepType, setStepType] = React.useState(
    initialValue?.stepType ?? "llm",
  );
  const [checkpointType, setCheckpointType] = React.useState(
    initialValue?.checkpointType ?? "Step",
  );
  const [model, setModel] = React.useState(initialValue?.model ?? defaultModel);
  const [tokenBudget, setTokenBudget] = React.useState(
    initialValue && initialValue.tokenBudget ? String(initialValue.tokenBudget) : "1000",
  );
  const [prompt, setPrompt] = React.useState(initialValue?.prompt ?? "");
  const [proofMode, setProofMode] = React.useState<RunProofMode>(
    initialValue?.proofMode ?? "exact",
  );
  const [epsilon, setEpsilon] = React.useState<number | null>(() => {
    if (typeof initialValue?.epsilon === "number") {
      return clampNormalizedEpsilon(initialValue.epsilon);
    }
    if (initialValue?.proofMode === "concordant") {
      return DEFAULT_CONCORDANT_EPSILON;
    }
    return null;
  });

  // Document ingestion fields
  const [sourcePath, setSourcePath] = React.useState(initialValue?.sourcePath ?? "");
  const [format, setFormat] = React.useState(initialValue?.format ?? "pdf");
  const [privacyStatus, setPrivacyStatus] = React.useState(
    initialValue?.privacyStatus ?? "public",
  );

  const [error, setError] = React.useState<string | null>(null);
  const proofModeFieldName = React.useId();

  const canSubmitCurrent = React.useMemo(() => {
    return concordantSubmissionAllowed(proofMode, epsilon);
  }, [proofMode, epsilon]);

  React.useEffect(() => {
    setStepType(initialValue?.stepType ?? "llm");
    setCheckpointType(initialValue?.checkpointType ?? "Step");
    setModel(initialValue?.model ?? defaultModel);
    setTokenBudget(initialValue ? String(initialValue.tokenBudget) : "1000");
    setPrompt(initialValue?.prompt ?? "");
    setProofMode(initialValue?.proofMode ?? "exact");
    if (typeof initialValue?.epsilon === "number") {
      setEpsilon(clampNormalizedEpsilon(initialValue.epsilon));
    } else if (initialValue?.proofMode === "concordant") {
      setEpsilon(DEFAULT_CONCORDANT_EPSILON);
    } else {
      setEpsilon(null);
    }

    // Parse document ingestion fields if present
    if (initialValue?.stepType === "document_ingestion" && initialValue?.configJson) {
      try {
        const docConfig = JSON.parse(initialValue.configJson);
        setSourcePath(docConfig.sourcePath ?? "");
        setFormat(docConfig.format ?? "pdf");
        setPrivacyStatus(docConfig.privacyStatus ?? "public");
      } catch {
        // If parsing fails, use defaults
        setSourcePath("");
        setFormat("pdf");
        setPrivacyStatus("public");
      }
    }

    setError(null);
  }, [initialValue, defaultModel]);

  const handleBrowseDocument = React.useCallback(async () => {
    console.log('Browse button clicked');
    try {
      console.log('Opening file dialog...');
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [
          { name: 'Documents', extensions: ['pdf', 'tex', 'latex', 'docx', 'txt'] },
          { name: 'PDF', extensions: ['pdf'] },
          { name: 'LaTeX', extensions: ['tex', 'latex'] },
          { name: 'All Files', extensions: ['*'] },
        ],
      });
      console.log('Dialog result:', selected);
      if (selected) {
        setSourcePath(selected);
      }
    } catch (err) {
      console.error('Failed to open file picker:', err);
    }
  }, []);

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (submitting) {
      return;
    }

    const cleanedType = sanitizeLabel(checkpointType || "Step");
    if (!cleanedType) {
      setError("Checkpoint name is required.");
      return;
    }

    setError(null);

    if (stepType === "document_ingestion") {
      // Validate document ingestion fields
      const cleanedPath = sourcePath.trim();
      if (!cleanedPath) {
        setError("Document path is required.");
        return;
      }

      await onSubmit({
        stepType: "document_ingestion",
        checkpointType: cleanedType,
        sourcePath: cleanedPath,
        format,
        privacyStatus,
      });
      return;
    }

    // LLM step validation
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

    let epsilonValue: number | null = null;
    if (proofMode === "concordant") {
      if (!isValidNormalizedEpsilon(epsilon)) {
        setError("Concordant checkpoints require an epsilon between 0 and 1.");
        return;
      }
      epsilonValue = clampNormalizedEpsilon(epsilon as number);
    }

    await onSubmit({
      stepType: "llm",
      checkpointType: cleanedType,
      model: cleanedModel,
      tokenBudget: parsedBudget,
      prompt: cleanedPrompt,
      proofMode,
      epsilon: epsilonValue,
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
        Step Type
        <select
          value={stepType}
          onChange={(event) => setStepType(event.target.value as "llm" | "document_ingestion")}
        >
          <option value="llm">LLM Prompt</option>
          <option value="document_ingestion">Document Ingestion</option>
        </select>
      </label>

      <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
        Checkpoint Name
        <input
          type="text"
          value={checkpointType}
          onChange={(event) => setCheckpointType(event.target.value)}
          placeholder="Enter checkpoint name"
        />
      </label>

      {stepType === "llm" ? (
        <>
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
        </>
      ) : (
        <>
          <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
            Document Path
            <div style={{ display: "flex", gap: "8px" }}>
              <input
                type="text"
                value={sourcePath}
                onChange={(event) => setSourcePath(event.target.value)}
                placeholder="/path/to/document.pdf"
                style={{ flex: 1 }}
              />
              <button
                type="button"
                onClick={handleBrowseDocument}
                style={buttonSecondary}
              >
                Browse
              </button>
            </div>
          </label>
          <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
            Format
            <select value={format} onChange={(event) => setFormat(event.target.value)}>
              <option value="pdf">PDF</option>
              <option value="latex">LaTeX</option>
              <option value="txt">TXT</option>
              <option value="docx">DOCX</option>
            </select>
          </label>
          <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
            Privacy Status
            <select
              value={privacyStatus}
              onChange={(event) => setPrivacyStatus(event.target.value)}
            >
              <option value="public">Public</option>
              <option value="consent_obtained_anonymized">Consent Obtained (Anonymized)</option>
              <option value="internal">Internal</option>
            </select>
          </label>
        </>
      )}

      {stepType === "llm" && (
        <>
          <fieldset
            style={{
              display: "flex",
              flexDirection: "column",
              gap: "8px",
              padding: 0,
              border: "none",
              margin: 0,
            }}
          >
            <legend style={{ marginBottom: "4px" }}>Proof configuration</legend>
            <div style={{ display: "flex", gap: "12px" }}>
              <label style={{ display: "flex", alignItems: "center", gap: "4px" }}>
                <input
                  type="radio"
                  name={proofModeFieldName}
                  value="exact"
                  checked={proofMode === "exact"}
                  onChange={() => {
                    setProofMode("exact");
                    setError(null);
                  }}
                />
                Exact
              </label>
              <label style={{ display: "flex", alignItems: "center", gap: "4px" }}>
                <input
                  type="radio"
                  name={proofModeFieldName}
                  value="concordant"
                  checked={proofMode === "concordant"}
                  onChange={() => {
                    setProofMode("concordant");
                    setError(null);
                    setEpsilon((current) =>
                      isValidNormalizedEpsilon(current)
                        ? clampNormalizedEpsilon(current as number)
                        : DEFAULT_CONCORDANT_EPSILON,
                    );
                  }}
                />
                Concordant
              </label>
            </div>
            {proofMode === "concordant" && (
              <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
                <input
                  type="range"
                  min={0}
                  max={1}
                  step={0.01}
                  value={clampNormalizedEpsilon(
                    isValidNormalizedEpsilon(epsilon)
                      ? (epsilon as number)
                      : DEFAULT_CONCORDANT_EPSILON,
                  )}
                  onChange={(event) => {
                    const nextValue = Number(event.target.value);
                    setEpsilon(clampNormalizedEpsilon(nextValue));
                    setError(null);
                  }}
                />
                <span style={{ fontVariantNumeric: "tabular-nums" }}>
                  ε ={" "}
                  {clampNormalizedEpsilon(
                    isValidNormalizedEpsilon(epsilon)
                      ? (epsilon as number)
                      : DEFAULT_CONCORDANT_EPSILON,
                  ).toFixed(2)}
                </span>
              </div>
            )}
          </fieldset>
          {proofMode === "concordant" && !canSubmitCurrent ? (
            <div style={{ color: "#f48771", fontSize: "0.85rem" }}>
              Adjust the epsilon slider to a value between 0 and 1 before saving this concordant
              checkpoint.
            </div>
          ) : null}
          <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
            Prompt
            <textarea
              value={prompt}
              onChange={(event) => setPrompt(event.target.value)}
              rows={6}
              style={{ fontFamily: "monospace" }}
            />
          </label>
        </>
      )}
      {error && <div style={{ color: "#f48771", fontSize: "0.85rem" }}>{error}</div>}
      <div style={{ display: "flex", gap: "8px" }}>
        <button
          type="submit"
          disabled={submitting}
          style={combineButtonStyles(buttonPrimary, submitting && buttonDisabled)}
        >
          {submitting ? "Saving…" : submitLabel}
        </button>
        <button
          type="button"
          onClick={onCancel}
          disabled={submitting}
          style={combineButtonStyles(buttonSecondary, submitting && buttonDisabled)}
        >
          Cancel
        </button>
      </div>
    </form>
  );
}
