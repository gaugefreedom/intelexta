import React from "react";
import type { RunProofMode, CatalogModel } from "../lib/api";
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
  catalogModels: CatalogModel[];
  existingSteps?: Array<{ orderIndex: number; checkpointType: string; stepType: string }>;
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

const DEFAULT_CONCORDANT_EPSILON = 0.5;

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
  catalogModels,
  existingSteps = [],
  initialValue,
  mode,
  onSubmit,
  onCancel,
  submitting = false,
}: CheckpointEditorProps) {
  const mergedModels = React.useMemo(() => {
    const modelIdSet = new Set<string>();

    console.log('[CheckpointEditor] Building merged models list');
    console.log('[CheckpointEditor] catalogModels:', catalogModels.length);
    console.log('[CheckpointEditor] availableModels:', availableModels);

    catalogModels.forEach(model => {
      console.log(`[CheckpointEditor] Checking model ${model.id}:`, {
        requires_network: model.requires_network,
        requires_api_key: model.requires_api_key,
        is_api_key_configured: model.is_api_key_configured,
      });

      // Add local models (like Ollama) ONLY if they are detected as available on the system.
      if (!model.requires_network) {
        if (availableModels.includes(model.id)) {
          console.log(`[CheckpointEditor] Adding local model: ${model.id}`);
          modelIdSet.add(model.id);
        }
      }
      // Add remote models ONLY if their API key is configured.
      else if (model.requires_api_key && model.is_api_key_configured) {
        console.log(`[CheckpointEditor] Adding remote model with API key: ${model.id}`);
        modelIdSet.add(model.id);
      }
      // Handle models that don't need a key (like the stub-model).
      else if (!model.requires_api_key) {
        console.log(`[CheckpointEditor] Adding model without API key requirement: ${model.id}`);
        modelIdSet.add(model.id);
      }
    });

    // Ensure the initial value is in the list if we are editing an existing step.
    if (initialValue?.model) {
      modelIdSet.add(initialValue.model);
    }

    // If after all checks the list is empty, add the stub model as a last resort.
    if (modelIdSet.size === 0) {
      modelIdSet.add("stub-model");
    }

    console.log('[CheckpointEditor] Final merged models:', Array.from(modelIdSet));
    return Array.from(modelIdSet);
  }, [catalogModels, availableModels, initialValue?.model]);

  // Filter steps to only show previous steps (for chaining)
  // When creating a new step, show all existing steps
  // When editing, show only steps before the current one
  const availablePreviousSteps = React.useMemo(() => {
    // For create mode, all existing steps can be referenced
    if (mode === "create") {
      return existingSteps;
    }

    // For edit mode, find current step's order index and filter
    // This prevents circular references and forward references
    const currentOrderIndex = existingSteps.findIndex(
      step => step.checkpointType === initialValue?.checkpointType
    );

    if (currentOrderIndex === -1) {
      return existingSteps; // Fallback: show all if we can't find current step
    }

    // Only show steps that come before the current step
    return existingSteps.filter(step => step.orderIndex < currentOrderIndex);
  }, [existingSteps, mode, initialValue?.checkpointType]);

  // Find first local model (prioritize Ollama, then any non-network model)
  const defaultModel = React.useMemo(() => {
    // 1. If an initial model is provided, use it.
    if (initialValue?.model && catalogModels.some(m => m.id === initialValue.model)) {
      return initialValue.model;
    }

    // 2. Prioritize the first available and configured Ollama model.
    const ollamaModel = catalogModels.find(m => m.provider === "ollama" && m.is_api_key_configured);
    if (ollamaModel) {
      return ollamaModel.id;
    }

    // 3. Next, prioritize the first available and configured remote model.
    const remoteModel = catalogModels.find(m => m.requires_network && m.is_api_key_configured);
    if (remoteModel) {
      return remoteModel.id;
    }
    
    // 4. Fallback to the first enabled model in the catalog, or finally the stub model.
    const firstEnabled = catalogModels.find(m => m.enabled);
    return firstEnabled?.id ?? "stub-model";
  }, [catalogModels, initialValue?.model]);

  const [stepType, setStepType] = React.useState(
    initialValue?.stepType ?? "prompt", // Default to new prompt type instead of legacy llm
  );
  const [checkpointType, setCheckpointType] = React.useState(
    initialValue?.checkpointType ?? "Step",
  );
  const [model, setModel] = React.useState(initialValue?.model ?? defaultModel);
  const [tokenBudget, setTokenBudget] = React.useState(
    initialValue && initialValue.tokenBudget ? String(initialValue.tokenBudget) : "1000",
  );
  const [prompt, setPrompt] = React.useState(initialValue?.prompt ?? "");

  // Helper to get catalog info for a model
  const getModelInfo = React.useCallback((modelId: string) => {
    return catalogModels.find(m => m.id === modelId);
  }, [catalogModels]);

  // Get info for currently selected model
  const selectedModelInfo = React.useMemo(() => {
    return getModelInfo(model);
  }, [model, getModelInfo]);

  // Default proof mode based on step type
  const getDefaultProofMode = (type: string): RunProofMode => {
    if (type === "summarize" || type === "prompt" || type === "llm") {
      return "concordant"; // LLM-based steps default to concordant
    }
    return "exact"; // Only ingest defaults to exact
  };

  const [proofMode, setProofMode] = React.useState<RunProofMode>(
    initialValue?.proofMode ?? getDefaultProofMode(initialValue?.stepType ?? "llm"),
  );
  const [epsilon, setEpsilon] = React.useState<number | null>(() => {
    if (typeof initialValue?.epsilon === "number") {
      return clampNormalizedEpsilon(initialValue.epsilon);
    }
    if (initialValue?.proofMode === "concordant") {
      return 0.5; // Default epsilon for concordant mode
    }
    // For LLM-based steps (summarize/prompt/llm) without initial value, set default epsilon
    const type = initialValue?.stepType ?? "prompt";
    if (type === "summarize" || type === "prompt" || type === "llm") {
      return 0.5;
    }
    return null;
  });

  // Document ingestion fields
  const [sourcePath, setSourcePath] = React.useState(initialValue?.sourcePath ?? "");
  const [format, setFormat] = React.useState(initialValue?.format ?? "pdf");
  const [privacyStatus, setPrivacyStatus] = React.useState(
    initialValue?.privacyStatus ?? "public",
  );

  // Typed step fields (new system)
  const [sourceStep, setSourceStep] = React.useState<number | null>(null);
  const [useOutputFrom, setUseOutputFrom] = React.useState<number | null>(null);
  const [summaryType, setSummaryType] = React.useState("brief");
  const [customInstructions, setCustomInstructions] = React.useState("");

  const [error, setError] = React.useState<string | null>(null);
  const proofModeFieldName = React.useId();

  const canSubmitCurrent = React.useMemo(() => {
    return concordantSubmissionAllowed(proofMode, epsilon);
  }, [proofMode, epsilon]);

  // When step type changes, update proof mode defaults
  React.useEffect(() => {
    if (!initialValue) {
      // Only apply defaults for new steps (not editing)
      const defaultMode = getDefaultProofMode(stepType);
      setProofMode(defaultMode);
      if (defaultMode === "concordant") {
        setEpsilon(0.5);
      } else {
        setEpsilon(null);
      }
    }
  }, [stepType, initialValue]);

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

    // Handle typed steps (new system)
    if (stepType === "ingest") {
      const cleanedPath = sourcePath.trim();
      if (!cleanedPath) {
        setError("Document path is required.");
        return;
      }

      const configJson = JSON.stringify({
        stepType: "ingest",
        sourcePath: cleanedPath,
        format,
        privacyStatus,
      });

      await onSubmit({
        stepType: "ingest",
        checkpointType: cleanedType,
        sourcePath: cleanedPath,
        format,
        privacyStatus,
        configJson,
        tokenBudget: 1000, // Default budget for ingest steps
        proofMode: "exact",
      });
      return;
    }

    if (stepType === "summarize") {
      if (sourceStep === null) {
        setError("Source step is required for summarize.");
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

      const configJson = JSON.stringify({
        stepType: "summarize",
        sourceStep: sourceStep,
        model: cleanedModel,
        summaryType: summaryType,
        customInstructions: summaryType === "custom" ? customInstructions : undefined,
        tokenBudget: parsedBudget,
        proofMode: proofMode,
        epsilon: proofMode === "concordant" ? epsilon : undefined,
      });

      await onSubmit({
        stepType: "summarize",
        checkpointType: cleanedType,
        model: cleanedModel,
        prompt: `Summarize the output from step ${sourceStep + 1}`, // Fallback for legacy execution
        tokenBudget: parsedBudget,
        proofMode,
        epsilon: proofMode === "concordant" ? epsilon : null,
        configJson,
      });
      return;
    }

    if (stepType === "prompt") {
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

      const configJson = JSON.stringify({
        stepType: "prompt",
        model: cleanedModel,
        prompt: cleanedPrompt,
        useOutputFrom: useOutputFrom === null ? undefined : useOutputFrom,
        tokenBudget: parsedBudget,
        proofMode: proofMode,
        epsilon: proofMode === "concordant" ? epsilon : undefined,
      });

      await onSubmit({
        stepType: "prompt",
        checkpointType: cleanedType,
        model: cleanedModel,
        tokenBudget: parsedBudget,
        prompt: cleanedPrompt,
        proofMode,
        epsilon: proofMode === "concordant" ? epsilon : null,
        configJson,
      });
      return;
    }

    // Legacy step types
    if (stepType === "document_ingestion") {
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

    if (stepType === "llm") {
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
    }
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
          onChange={(event) => setStepType(event.target.value as "llm" | "document_ingestion" | "ingest" | "summarize" | "prompt")}
        >
          <option key="ingest" value="ingest">Ingest Document</option>
          <option key="summarize" value="summarize">Summarize</option>
          <option key="prompt" value="prompt">Prompt (with optional context)</option>
          <option key="document_ingestion" value="document_ingestion">Document Ingestion (legacy)</option>
          <option key="llm" value="llm">LLM Prompt (legacy)</option>
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

      {/* Render fields based on step type */}
      {(stepType === "ingest" || stepType === "document_ingestion") && (
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
              <option key="pdf" value="pdf">PDF</option>
              <option key="latex" value="latex">LaTeX</option>
              <option key="txt" value="txt">TXT</option>
              <option key="docx" value="docx">DOCX</option>
            </select>
          </label>
          <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
            Privacy Status
            <select
              value={privacyStatus}
              onChange={(event) => setPrivacyStatus(event.target.value)}
            >
              <option key="public" value="public">Public</option>
              <option key="consent_obtained_anonymized" value="consent_obtained_anonymized">Consent Obtained (Anonymized)</option>
              <option key="internal" value="internal">Internal</option>
            </select>
          </label>
        </>
      )}

      {stepType === "summarize" && (
        <>
          <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
            Source Step
            <select
              value={sourceStep ?? ""}
              onChange={(event) => setSourceStep(event.target.value ? parseInt(event.target.value) : null)}
            >
              <option value="">Select a previous step...</option>
              {availablePreviousSteps.map((step) => (
                <option key={step.orderIndex} value={step.orderIndex}>
                  Step {step.orderIndex + 1}: {step.checkpointType}
                </option>
              ))}
            </select>
          </label>
          <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
            Model
            <select value={model} onChange={(event) => setModel(event.target.value)}>
              {mergedModels.map((modelId) => {
                const info = getModelInfo(modelId);
                const badge = info ? (info.requires_network ? "[R]" : "[L]") : "";
                const displayText = info ? `${badge} ${info.display_name}` : modelId;
                return (
                  <option key={modelId} value={modelId}>
                    {displayText}
                  </option>
                );
              })}
            </select>
          </label>
          <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
            Summary Type
            <select value={summaryType} onChange={(event) => setSummaryType(event.target.value)}>
              <option key="brief" value="brief">Brief (2-3 sentences)</option>
              <option key="detailed" value="detailed">Detailed (comprehensive)</option>
              <option key="academic" value="academic">Academic (methodology + findings)</option>
              <option key="custom" value="custom">Custom instructions</option>
            </select>
          </label>
          {summaryType === "custom" && (
            <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
              Custom Instructions
              <textarea
                value={customInstructions}
                onChange={(event) => setCustomInstructions(event.target.value)}
                placeholder="Enter your custom summary instructions..."
                rows={3}
                style={{
                  fontFamily: "inherit",
                  fontSize: "inherit",
                  padding: "6px 8px",
                  backgroundColor: "#1e1e1e",
                  color: "#d4d4d4",
                  border: "1px solid #3c3c3c",
                  borderRadius: "4px",
                }}
              />
            </label>
          )}
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
          {/* Proof configuration for summarize */}
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
                        : 0.5, // Default for summarize
                    );
                  }}
                />
                Concordant (recommended)
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
                      : 0.5,
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
                      : 0.5,
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
        </>
      )}

      {(stepType === "prompt" || stepType === "llm") && (
        <>
          {stepType === "prompt" && (
            <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
              Use Output From (optional)
              <select
                value={useOutputFrom ?? ""}
                onChange={(event) => setUseOutputFrom(event.target.value ? parseInt(event.target.value) : null)}
              >
                <option value="">None (standalone prompt)</option>
                {availablePreviousSteps.map((step) => (
                  <option key={step.orderIndex} value={step.orderIndex}>
                    Step {step.orderIndex + 1}: {step.checkpointType}
                  </option>
                ))}
              </select>
            </label>
          )}
          <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
            Model
            <select value={model} onChange={(event) => setModel(event.target.value)}>
              {mergedModels.map((modelId) => {
                const info = getModelInfo(modelId);
                const badge = info ? (info.requires_network ? "[R]" : "[L]") : "";
                const displayText = info ? `${badge} ${info.display_name}` : modelId;
                return (
                  <option key={modelId} value={modelId}>
                    {displayText}
                  </option>
                );
              })}
            </select>
          </label>
          <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
            Prompt
            <textarea
              value={prompt}
              onChange={(event) => setPrompt(event.target.value)}
              placeholder="Enter your prompt..."
              rows={4}
              style={{
                fontFamily: "inherit",
                fontSize: "inherit",
                padding: "6px 8px",
                backgroundColor: "#1e1e1e",
                color: "#d4d4d4",
                border: "1px solid #3c3c3c",
                borderRadius: "4px",
              }}
            />
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
      )}

      {stepType === "prompt" && (
        <>
          {/* Proof configuration for prompt */}
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
                        : 0.5,
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
                      : 0.5,
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
                      : 0.5,
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
