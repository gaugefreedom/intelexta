import React from "react";
import {
  CheckpointSummary,
  listLocalModels,
  RunSummary,
  listRuns,
  listRunCheckpointConfigs,
  RunCheckpointConfig,
  createCheckpointConfig,
  updateCheckpointConfig,
  deleteCheckpointConfig,
  reorderCheckpointConfigs,
  CheckpointConfigRequest,
  createRun,
  startRun,
  reopenRun,
  cloneRun,
  estimateRunCost,
  updateRunSettings,
  openInteractiveCheckpointSession,
  submitInteractiveCheckpointTurn,
  finalizeInteractiveCheckpoint,
  type OpenInteractiveCheckpointSession,
  type SubmitInteractiveCheckpointTurn,
  type FinalizeInteractiveCheckpoint,
  type RunCostEstimates,
  type RunProofMode,
} from "../lib/api";
import { interactiveFeatureEnabled } from "../lib/featureFlags";
import CheckpointEditor, { CheckpointFormValue } from "./CheckpointEditor";
import CheckpointListItem from "./CheckpointListItem";

type ConversationRoleCategory = "human" | "ai" | "other";

interface ConversationMessage {
  id: string;
  role: string;
  body: string;
  timestamp: string;
  pending?: boolean;
}

function classifyRole(role: string): ConversationRoleCategory {
  const normalized = role.trim().toLowerCase();
  if (normalized === "human" || normalized === "user") {
    return "human";
  }
  if (normalized === "ai" || normalized === "assistant" || normalized === "model") {
    return "ai";
  }
  return "other";
}

function conversationRoleLabel(role: string): string {
  switch (classifyRole(role)) {
    case "human":
      return "You";
    case "ai":
      return "AI";
    default:
      return role;
  }
}

function formatTimestampLabel(value: string): string {
  if (!value) {
    return "";
  }
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }
  return parsed.toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

const MAX_RUN_NAME_LENGTH = 120;

function sanitizeRunName(value: string): string {
  return value.replace(/\u0000/g, "").replace(/\s+/g, " ").trim();
}

function evaluateNormalizedEpsilonInput(value: string): {
  value: number | null;
  error: string | null;
} {
  const trimmed = value.trim();
  if (trimmed.length === 0) {
    return { value: null, error: null };
  }
  const parsed = Number.parseFloat(trimmed);
  if (!Number.isFinite(parsed)) {
    return { value: null, error: "Epsilon must be a number between 0 and 1." };
  }
  if (parsed < 0 || parsed > 1) {
    return { value: null, error: "Epsilon must be between 0 and 1." };
  }
  return { value: parsed, error: null };
}

function formatNormalizedEpsilon(value: number): string {
  if (!Number.isFinite(value)) {
    return "";
  }
  return Number(value.toFixed(6)).toString();
}

function epsilonValuesEqual(a: number | null, b: number | null): boolean {
  if (a === null && b === null) {
    return true;
  }
  if (a === null || b === null) {
    return false;
  }
  return Math.abs(a - b) < 1e-9;
}

function extractMessagesFromCheckpoints(
  checkpoints: CheckpointSummary[],
): ConversationMessage[] {
  const messages: ConversationMessage[] = [];
  for (const checkpoint of checkpoints) {
    const checkpointMessage = checkpoint.message;
    if (!checkpointMessage) {
      continue;
    }
    const role = checkpointMessage.role ? checkpointMessage.role.trim() : "";
    messages.push({
      id: checkpoint.id,
      role: role.length > 0 ? role : "assistant",
      body: checkpointMessage.body,
      timestamp: checkpointMessage.createdAt ?? checkpoint.timestamp,
    });
  }
  return messages;
}

interface InteractiveConversationViewProps {
  runId: string;
  checkpointId: string;
  onExit: () => void;
  openSession: OpenInteractiveCheckpointSession;
  submitTurn: SubmitInteractiveCheckpointTurn;
  finalizeSession: FinalizeInteractiveCheckpoint;
}

function InteractiveConversationView({
  runId,
  checkpointId,
  onExit,
  openSession,
  submitTurn,
  finalizeSession,
}: InteractiveConversationViewProps) {
  const [checkpointConfig, setCheckpointConfig] = React.useState<RunCheckpointConfig | null>(
    null,
  );
  const [messages, setMessages] = React.useState<ConversationMessage[]>([]);
  const [messagesLoading, setMessagesLoading] = React.useState<boolean>(false);
  const [messagesError, setMessagesError] = React.useState<string | null>(null);
  const [composerValue, setComposerValue] = React.useState<string>("");
  const [composerError, setComposerError] = React.useState<string | null>(null);
  const [isSending, setIsSending] = React.useState<boolean>(false);
  const [finalizeMessage, setFinalizeMessage] = React.useState<string | null>(null);
  const [finalizeError, setFinalizeError] = React.useState<string | null>(null);
  const scrollContainerRef = React.useRef<HTMLDivElement | null>(null);

  const refreshSession = React.useCallback(async () => {
    const session = await openSession(runId, checkpointId);
    return session;
  }, [runId, checkpointId, openSession]);

  React.useEffect(() => {
    setMessages([]);
    setCheckpointConfig(null);
    setComposerValue("");
    setComposerError(null);
    setMessagesError(null);
    setFinalizeMessage(null);
    setFinalizeError(null);
  }, [runId, checkpointId]);

  React.useEffect(() => {
    let cancelled = false;
    setMessagesLoading(true);
    setMessagesError(null);

    refreshSession()
      .then((session) => {
        if (cancelled) return;
        setCheckpointConfig(session.checkpoint);
        setMessages(extractMessagesFromCheckpoints(session.messages));
      })
      .catch((err) => {
        if (cancelled) return;
        console.error("Failed to load conversation history", err);
        const message =
          err instanceof Error
            ? err.message
            : "Failed to load conversation history.";
        setMessagesError(message);
        setCheckpointConfig(null);
        setMessages([]);
      })
      .finally(() => {
        if (!cancelled) {
          setMessagesLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [refreshSession]);

  React.useEffect(() => {
    const container = scrollContainerRef.current;
    if (container) {
      container.scrollTop = container.scrollHeight;
    }
  }, [messages]);

  const handleComposerSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const trimmed = composerValue.trim();
    if (!trimmed || messagesLoading) {
      return;
    }

    const timestamp = new Date().toISOString();
    const nonce = Date.now();
    const humanPlaceholderId = `temp-human-${nonce}`;
    const aiPlaceholderId = `temp-ai-${nonce}`;

    setComposerError(null);
    setComposerValue("");
    setIsSending(true);

    setMessages((previous) => [
      ...previous,
      {
        id: humanPlaceholderId,
        role: "human",
        body: trimmed,
        timestamp,
      },
      {
        id: aiPlaceholderId,
        role: "ai",
        body: "Awaiting response…",
        timestamp,
        pending: true,
      },
    ]);

    try {
      await submitTurn(runId, checkpointId, trimmed);
      const session = await refreshSession();
      setCheckpointConfig(session.checkpoint);
      setMessages(extractMessagesFromCheckpoints(session.messages));
      setMessagesError(null);
    } catch (err) {
      console.error("Failed to submit interactive turn", err);
      const message =
        err instanceof Error
          ? err.message
          : "Unable to send your message. Please try again.";
      setComposerError(message);
      setMessages((previous) =>
        previous.filter(
          (entry) => entry.id !== humanPlaceholderId && entry.id !== aiPlaceholderId,
        ),
      );
    } finally {
      setIsSending(false);
    }
  };

  const handleFinalize = async () => {
    setFinalizeMessage(null);
    setFinalizeError(null);
    try {
      await finalizeSession(runId, checkpointId);
      setFinalizeMessage("Transcript finalized.");
    } catch (err) {
      console.error("Failed to finalize interactive checkpoint", err);
      const message =
        err instanceof Error
          ? err.message
          : "Unable to finalize the interactive transcript.";
      setFinalizeError(message);
    }
  };

  const disableSend =
    isSending || messagesLoading || composerValue.trim().length === 0;

  const checkpointLabel = checkpointConfig?.checkpointType ?? "Interactive Chat";

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "16px", maxWidth: "720px" }}>
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          gap: "12px",
          flexWrap: "wrap",
        }}
      >
        <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
          <div style={{ fontSize: "1rem", fontWeight: 600 }}>Interactive conversation</div>
          <div style={{ fontSize: "0.8rem", color: "#9cdcfe" }}>Run ID: {runId}</div>
          <div style={{ fontSize: "0.8rem", color: "#c8c8c8" }}>
            Checkpoint: {checkpointLabel}
          </div>
        </div>
        <div style={{ display: "flex", gap: "8px", flexWrap: "wrap" }}>
          <button type="button" onClick={handleFinalize} disabled={messagesLoading || isSending}>
            Finalize transcript
          </button>
          <button type="button" onClick={onExit}>
            Return to configuration
          </button>
        </div>
      </div>

      {finalizeMessage && (
        <div style={{ color: "#b5cea8", fontSize: "0.85rem" }}>{finalizeMessage}</div>
      )}
      {finalizeError && (
        <div style={{ color: "#f48771", fontSize: "0.85rem" }}>{finalizeError}</div>
      )}

      {checkpointConfig?.prompt && (
        <div
          style={{
            border: "1px solid #333",
            borderRadius: "6px",
            padding: "10px",
            backgroundColor: "#202020",
            fontSize: "0.85rem",
            color: "#c8c8c8",
            whiteSpace: "pre-wrap",
          }}
        >
          <div style={{ fontSize: "0.75rem", color: "#9cdcfe", marginBottom: "6px" }}>
            Prompt instructions
          </div>
          {checkpointConfig.prompt}
        </div>
      )}

      <div
        ref={scrollContainerRef}
        style={{
          border: "1px solid #333",
          borderRadius: "8px",
          padding: "12px",
          minHeight: "280px",
          maxHeight: "420px",
          overflowY: "auto",
          backgroundColor: "#1e1e1e",
          display: "flex",
          flexDirection: "column",
          gap: "12px",
        }}
      >
        {messagesLoading && messages.length === 0 ? (
          <div style={{ color: "#9cdcfe", fontSize: "0.9rem" }}>Loading conversation…</div>
        ) : messages.length === 0 ? (
          <div style={{ color: "#9cdcfe", fontSize: "0.9rem" }}>
            No messages yet. Start the conversation by sending a prompt.
          </div>
        ) : (
          messages.map((message) => {
            const roleCategory = classifyRole(message.role);
            const isHuman = roleCategory === "human";
            const borderColor =
              roleCategory === "ai" ? "#c586c0" : isHuman ? "#4ec9b0" : "#dcdcaa";
            const backgroundColor = isHuman ? "#1f2933" : "#252526";
            const timestampLabel = formatTimestampLabel(message.timestamp);
            return (
              <div
                key={message.id}
                style={{
                  alignSelf: isHuman ? "flex-end" : "flex-start",
                  backgroundColor,
                  border: `1px solid ${borderColor}`,
                  borderRadius: "10px",
                  padding: "10px 12px",
                  maxWidth: "75%",
                  boxShadow: "0 1px 3px rgba(0, 0, 0, 0.25)",
                }}
              >
                <div
                  style={{
                    display: "flex",
                    justifyContent: "space-between",
                    alignItems: "baseline",
                    gap: "12px",
                    fontSize: "0.75rem",
                    marginBottom: "6px",
                    color: borderColor,
                  }}
                >
                  <span>{conversationRoleLabel(message.role)}</span>
                  <span style={{ color: "#d4d4d4" }}>{timestampLabel}</span>
                </div>
                <div
                  style={{
                    fontSize: "0.95rem",
                    color: "#d4d4d4",
                    whiteSpace: "pre-wrap",
                    lineHeight: 1.45,
                  }}
                >
                  {message.pending ? <em>Awaiting response…</em> : message.body}
                </div>
              </div>
            );
          })
        )}
      </div>

      {messagesError && (
        <div style={{ color: "#f48771", fontSize: "0.85rem" }}>{messagesError}</div>
      )}

      <form
        onSubmit={handleComposerSubmit}
        style={{ display: "flex", flexDirection: "column", gap: "10px" }}
      >
        <label style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
          <span style={{ fontSize: "0.9rem", color: "#9cdcfe" }}>Your message</span>
          <textarea
            value={composerValue}
            onChange={(event) => setComposerValue(event.target.value)}
            rows={4}
            placeholder="Ask the assistant a question or provide guidance"
            disabled={isSending || messagesLoading}
            style={{
              resize: "vertical",
              minHeight: "96px",
              padding: "8px",
              fontSize: "0.95rem",
              borderRadius: "6px",
            }}
          />
        </label>
        <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
          <button type="submit" disabled={disableSend}>
            {isSending ? "Sending…" : "Send"}
          </button>
          {composerError && (
            <span style={{ color: "#f48771", fontSize: "0.85rem" }}>{composerError}</span>
          )}
        </div>
      </form>
    </div>
  );
}

function proofModeLabel(kind: string): string {
  switch (kind) {
    case "concordant":
      return "Concordant";
    case "exact":
    default:
      return "Exact";
  }
}

function sanitizeLabelForRequest(value: string, fallback: string): string {
  const cleaned = value.replace(/\u0000/g, "").replace(/\s+/g, " " ).trim();
  return cleaned.length > 0 ? cleaned : fallback;
}

function normalizeModelSelection(value: string, options: string[], fallback: string): string {
  const sanitized = sanitizeLabelForRequest(value, fallback);
  const normalized = sanitized.toLowerCase();
  const match = options.find((option) => option.toLowerCase() === normalized);
  return match ?? sanitized;
}

function sanitizePromptForRequest(value: string): string {
  return value.replace(/\u0000/g, "").replace(/\r\n/g, "\n").trim();
}

function clampTokenBudget(value: number): number {
  if (!Number.isFinite(value)) {
    return 0;
  }
  const clamped = Math.max(0, Math.floor(value));
  return Math.min(clamped, Number.MAX_SAFE_INTEGER);
}

function sanitizeCheckpointFormValue(
  value: CheckpointFormValue,
  modelOptions: string[],
): CheckpointConfigRequest {
  const fallbackModel = modelOptions[0] ?? "stub-model";
  const model = normalizeModelSelection(value.model, modelOptions, fallbackModel);
  const checkpointType = sanitizeLabelForRequest(value.checkpointType, "Step");
  const prompt = sanitizePromptForRequest(value.prompt);
  const tokenBudget = clampTokenBudget(value.tokenBudget);
  const proofMode = value.proofMode === "concordant" ? "concordant" : "exact";
  return {
    model,
    prompt,
    tokenBudget,
    checkpointType,
    proofMode,
  };
}

function formatRunTimestamp(value?: string): string {
  if (!value) {
    return "—";
  }
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }
  return parsed.toLocaleString();
}

type EditorState =
  | { mode: "create" }
  | { mode: "edit"; checkpoint: RunCheckpointConfig };

interface EditorPanelProps {
  projectId: string;
  selectedRunId: string | null;
  onSelectRun: (runId: string | null) => void;
  refreshToken: number;
  onRunExecuted?: (runId: string) => void;
  onRunsMutated?: () => void;
}

export default function EditorPanel({
  projectId,
  selectedRunId,
  onSelectRun,
  refreshToken,
  onRunExecuted,
  onRunsMutated,
}: EditorPanelProps) {
  const [runs, setRuns] = React.useState<RunSummary[]>([]);
  const [runsLoading, setRunsLoading] = React.useState(false);
  const [runsError, setRunsError] = React.useState<string | null>(null);

  const runEpsilonInputRef = React.useRef<HTMLInputElement | null>(null);
  const [runProofMode, setRunProofMode] = React.useState<RunProofMode>("exact");
  const [runEpsilonInput, setRunEpsilonInput] = React.useState<string>("");
  const [runSettingsSaving, setRunSettingsSaving] = React.useState(false);
  const lastSavedSettings = React.useRef<{ proofMode: RunProofMode; epsilon: number | null }>({
    proofMode: "exact",
    epsilon: null,
  });

  const [checkpointConfigs, setCheckpointConfigs] = React.useState<RunCheckpointConfig[]>([]);
  const [configsLoading, setConfigsLoading] = React.useState(false);
  const [configsError, setConfigsError] = React.useState<string | null>(null);
  const [configsRefreshToken, setConfigsRefreshToken] = React.useState(0);

  const [costEstimates, setCostEstimates] = React.useState<RunCostEstimates | null>(null);
  const [costEstimateLoading, setCostEstimateLoading] = React.useState(false);
  const [costEstimateError, setCostEstimateError] = React.useState<string | null>(null);
  const [costsRefreshToken, setCostsRefreshToken] = React.useState(0);

  const [availableModels, setAvailableModels] = React.useState<string[]>(["stub-model"]);
  const [modelsLoading, setModelsLoading] = React.useState(false);
  const [modelsError, setModelsError] = React.useState<string | null>(null);

  const [activeEditor, setActiveEditor] = React.useState<EditorState | null>(null);
  const [editorSubmitting, setEditorSubmitting] = React.useState(false);

  const [statusMessage, setStatusMessage] = React.useState<string | null>(null);
  const [errorMessage, setErrorMessage] = React.useState<string | null>(null);
  const [executingRun, setExecutingRun] = React.useState(false);
  const [creatingRun, setCreatingRun] = React.useState(false);
  const [reopeningRun, setReopeningRun] = React.useState(false);
  const [cloningRun, setCloningRun] = React.useState(false);
  const [newRunName, setNewRunName] = React.useState("");
  const [newRunNameError, setNewRunNameError] = React.useState<string | null>(null);
  const [newRunProofMode, setNewRunProofMode] = React.useState<RunProofMode>("exact");
  const [newRunEpsilon, setNewRunEpsilon] = React.useState<string>("");
  const [newRunEpsilonError, setNewRunEpsilonError] = React.useState<string | null>(null);

  const [conversationContext, setConversationContext] = React.useState<
    { runId: string; checkpointId: string } | null
  >(null);

  const interactiveSupport:
    | {
        openSession: OpenInteractiveCheckpointSession;
        submitTurn: SubmitInteractiveCheckpointTurn;
        finalizeSession: FinalizeInteractiveCheckpoint;
      }
    | null =
    interactiveFeatureEnabled &&
    openInteractiveCheckpointSession &&
    submitInteractiveCheckpointTurn &&
    finalizeInteractiveCheckpoint
      ? {
          openSession: openInteractiveCheckpointSession as OpenInteractiveCheckpointSession,
          submitTurn: submitInteractiveCheckpointTurn as SubmitInteractiveCheckpointTurn,
          finalizeSession: finalizeInteractiveCheckpoint as FinalizeInteractiveCheckpoint,
        }
      : null;

  React.useEffect(() => {
    if (!interactiveSupport) {
      setConversationContext(null);
    }
  }, [interactiveSupport]);

  React.useEffect(() => {
    if (!selectedRun) {
      setRunProofMode("exact");
      setRunEpsilonInput("");
      lastSavedSettings.current = { proofMode: "exact", epsilon: null };
      return;
    }

    const normalizedMode: RunProofMode =
      selectedRun.kind === "concordant" ? "concordant" : "exact";
    const epsilonFromRun =
      typeof selectedRun.epsilon === "number" && Number.isFinite(selectedRun.epsilon)
        ? selectedRun.epsilon
        : null;

    setRunProofMode(normalizedMode);
    setRunEpsilonInput(
      epsilonFromRun !== null ? formatNormalizedEpsilon(epsilonFromRun) : "",
    );
    lastSavedSettings.current = {
      proofMode: normalizedMode,
      epsilon: epsilonFromRun,
    };
  }, [selectedRun?.id, selectedRun?.kind, selectedRun?.epsilon]);

  const combinedModelOptions = React.useMemo(() => {
    const set = new Set<string>(availableModels);
    for (const config of checkpointConfigs) {
      set.add(config.model);
    }
    return Array.from(set).sort();
  }, [availableModels, checkpointConfigs]);

  const runEpsilonEvaluation = React.useMemo(
    () => evaluateNormalizedEpsilonInput(runEpsilonInput),
    [runEpsilonInput],
  );
  const runEpsilonValue =
    runEpsilonEvaluation.error === null ? runEpsilonEvaluation.value : null;
  const runEpsilonInputError = runEpsilonEvaluation.error;

  const newRunEpsilonEvaluation = React.useMemo(
    () => evaluateNormalizedEpsilonInput(newRunEpsilon),
    [newRunEpsilon],
  );
  const newRunEpsilonValue =
    newRunEpsilonEvaluation.error === null ? newRunEpsilonEvaluation.value : null;

  const selectedRun = React.useMemo(() => {
    if (!selectedRunId) {
      return null;
    }
    return runs.find((run) => run.id === selectedRunId) ?? null;
  }, [runs, selectedRunId]);

  const hasConcordantCheckpoint = React.useMemo(() => {
    return checkpointConfigs.some((config) => config.proofMode === "concordant");
  }, [checkpointConfigs]);

  const hasInteractiveCheckpoint = React.useMemo(() => {
    return checkpointConfigs.some(
      (config) => config.checkpointType.trim().toLowerCase() === 'interactivechat'.toLowerCase(),
    );
  }, [checkpointConfigs]);

  const epsilonRequired = runProofMode === "concordant" || hasConcordantCheckpoint;
  const epsilonRequirementUnmet =
    epsilonRequired && (runEpsilonInputError !== null || runEpsilonValue === null);

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
    if (costEstimates.exceedsGCo2e) {
      messages.push(
        `Carbon: ${costEstimates.estimatedGCo2e.toFixed(2)} g / ${costEstimates.budgetGCo2e.toFixed(2)} g`,
      );
    }
    return messages;
  }, [costEstimates]);

  React.useEffect(() => {
    let cancelled = false;
    setModelsLoading(true);
    setModelsError(null);
    listLocalModels()
      .then((models) => {
        if (cancelled) return;
        const filtered = models.map((entry) => entry.trim()).filter((entry) => entry.length > 0);
        if (filtered.length === 0) {
          setAvailableModels(["stub-model"]);
        } else {
          setAvailableModels(filtered);
        }
      })
      .catch((err) => {
        if (cancelled) return;
        console.error("Failed to load local models", err);
        setModelsError("Unable to load local models. Falling back to defaults.");
        setAvailableModels(["stub-model"]);
      })
      .finally(() => {
        if (!cancelled) {
          setModelsLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, []);

  React.useEffect(() => {
    let cancelled = false;
    setRunsLoading(true);
    setRunsError(null);
    listRuns(projectId)
      .then((runList) => {
        if (cancelled) return;
        setRuns(runList);
        if (runList.length === 0) {
          if (selectedRunId !== null) {
            onSelectRun(null);
          }
          return;
        }
        if (!selectedRunId || !runList.some((run) => run.id === selectedRunId)) {
          onSelectRun(runList[0].id);
        }
      })
      .catch((err) => {
        if (cancelled) return;
        console.error("Failed to load runs", err);
        setRunsError("Could not load runs for this project.");
        setRuns([]);
        onSelectRun(null);
      })
      .finally(() => {
        if (!cancelled) {
          setRunsLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [projectId, refreshToken, onSelectRun, selectedRunId]);

  React.useEffect(() => {
    if (!selectedRunId) {
      setCheckpointConfigs([]);
      setConfigsError(null);
      setConfigsLoading(false);
      return;
    }
    let cancelled = false;
    setConfigsLoading(true);
    setConfigsError(null);
    listRunCheckpointConfigs(selectedRunId)
      .then((items) => {
        if (!cancelled) {
          setCheckpointConfigs(items);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          console.error("Failed to load checkpoint configurations", err);
          const message =
            err instanceof Error
              ? err.message
              : "Could not load checkpoint configurations for the selected run.";
          setConfigsError(message);
          setCheckpointConfigs([]);
        }
      })
      .finally(() => {
        if (!cancelled) {
          setConfigsLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [selectedRunId, configsRefreshToken]);

  React.useEffect(() => {
    if (!selectedRunId) {
      setCostEstimates(null);
      setCostEstimateError(null);
      setCostEstimateLoading(false);
      return;
    }

    let cancelled = false;
    setCostEstimateLoading(true);
    setCostEstimateError(null);

    estimateRunCost(selectedRunId)
      .then((estimates) => {
        if (cancelled) {
          return;
        }
        setCostEstimates(estimates);
      })
      .catch((err) => {
        if (cancelled) {
          return;
        }
        console.error("Failed to estimate run cost", err);
        const message =
          err instanceof Error ? err.message : "Unable to estimate projected run cost.";
        setCostEstimateError(message);
        setCostEstimates(null);
      })
      .finally(() => {
        if (!cancelled) {
          setCostEstimateLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [selectedRunId, configsRefreshToken, costsRefreshToken, refreshToken]);

  React.useEffect(() => {
    setActiveEditor(null);
    setEditorSubmitting(false);
    setStatusMessage(null);
    setErrorMessage(null);
    setConversationContext((current) => {
      if (!selectedRunId) {
        return null;
      }
      if (current && current.runId !== selectedRunId) {
        return null;
      }
      return current;
    });
  }, [selectedRunId]);

  React.useEffect(() => {
    setConversationContext((current) => {
      if (!current) {
        return current;
      }
      const exists = checkpointConfigs.some((cfg) => cfg.id === current.checkpointId);
      return exists ? current : null;
    });
  }, [checkpointConfigs]);

  const persistRunSettings = React.useCallback(
    async (nextProofMode: RunProofMode, epsilonValue: number | null) => {
      if (!selectedRunId) {
        return;
      }

      const saved = lastSavedSettings.current;
      if (
        saved.proofMode === nextProofMode &&
        epsilonValuesEqual(saved.epsilon, epsilonValue)
      ) {
        return;
      }

      setStatusMessage(null);
      setErrorMessage(null);
      setRunSettingsSaving(true);
      try {
        const updated = await updateRunSettings({
          runId: selectedRunId,
          proofMode: nextProofMode,
          epsilon: epsilonValue,
        });
        setRuns((previous) =>
          previous.map((run) => (run.id === updated.id ? updated : run)),
        );

        const updatedProofMode: RunProofMode =
          updated.kind === "concordant" ? "concordant" : "exact";
        const updatedEpsilon =
          typeof updated.epsilon === "number" && Number.isFinite(updated.epsilon)
            ? updated.epsilon
            : null;

        lastSavedSettings.current = {
          proofMode: updatedProofMode,
          epsilon: updatedEpsilon,
        };

        setRunProofMode(updatedProofMode);
        setRunEpsilonInput(
          updatedEpsilon !== null ? formatNormalizedEpsilon(updatedEpsilon) : "",
        );
        setStatusMessage("Run proof settings updated.");
        onRunsMutated?.();
      } catch (err) {
        console.error("Failed to update run settings", err);
        const message =
          err instanceof Error ? err.message : "Unable to update run settings.";
        setErrorMessage(message);
        const savedProofMode = lastSavedSettings.current.proofMode;
        const savedEpsilon = lastSavedSettings.current.epsilon;
        setRunProofMode(savedProofMode);
        setRunEpsilonInput(
          savedEpsilon !== null ? formatNormalizedEpsilon(savedEpsilon) : "",
        );
      } finally {
        setRunSettingsSaving(false);
      }
    },
    [selectedRunId, onRunsMutated],
  );

  const handleCreateRun = React.useCallback(async () => {
    if (creatingRun) {
      return;
    }

    const cleanedName = sanitizeRunName(newRunName);
    if (cleanedName.length > MAX_RUN_NAME_LENGTH) {
      setNewRunNameError(`Run name must be ${MAX_RUN_NAME_LENGTH} characters or fewer.`);
      return;
    }

    setStatusMessage(null);
    setErrorMessage(null);
    setNewRunNameError(null);
    setNewRunEpsilonError(null);

    if (
      newRunProofMode === "concordant" &&
      (newRunEpsilonEvaluation.error !== null || newRunEpsilonValue === null)
    ) {
      const message =
        newRunEpsilonEvaluation.error ??
        "Concordant runs require an epsilon between 0 and 1.";
      setNewRunEpsilonError(message);
      return;
    }

    setCreatingRun(true);
    try {
      const fallbackModel = availableModels[0] ?? "stub-model";
      const timestampLabel = new Date().toLocaleString();
      const generatedName = `New run ${timestampLabel}`;
      const chosenName = cleanedName.length > 0 ? cleanedName : generatedName;
      const randomSeed = Math.floor(Math.random() * 1_000_000_000);
      const runId = await createRun({
        projectId,
        name: chosenName,
        proofMode: newRunProofMode,
        seed: randomSeed,
        tokenBudget: 1_000,
        defaultModel: fallbackModel,
        epsilon: newRunEpsilonValue ?? null,
      });

      let runList: RunSummary[] | null = null;
      try {
        runList = await listRuns(projectId);
      } catch (refreshErr) {
        console.error("Failed to refresh run list after creation", refreshErr);
        setRunsError(
          "Run was created, but the run list could not be refreshed automatically.",
        );
      }

      if (runList) {
        setRuns(runList);
        setRunsError(null);
      } else {
        const fallbackCreatedAt = new Date().toISOString();
        setRuns((previous) => {
          const filtered = previous.filter((item) => item.id !== runId);
          return [
            {
              id: runId,
              name: chosenName,
              createdAt: fallbackCreatedAt,
              kind: newRunProofMode,
              epsilon: newRunEpsilonValue ?? null,
              hasPersistedCheckpoint: false,
            },
            ...filtered,
          ];
        });
      }

      setNewRunName("");
      setNewRunProofMode("exact");
      setNewRunEpsilon("");
      setNewRunEpsilonError(null);
      onSelectRun(runId);
      setStatusMessage("Run created. Configure checkpoints before execution.");
      onRunsMutated?.();
    } catch (err) {
      console.error("Failed to create run", err);
      const message = err instanceof Error ? err.message : "Unable to create run.";
      setErrorMessage(message);
    } finally {
      setCreatingRun(false);
    }
  }, [
    creatingRun,
    availableModels,
    projectId,
    onSelectRun,
    onRunsMutated,
    newRunName,
    newRunProofMode,
    newRunEpsilonEvaluation.error,
    newRunEpsilonValue,
  ]);

  const handleRunProofModeChange = React.useCallback(
    (event: React.ChangeEvent<HTMLSelectElement>) => {
      const nextValue: RunProofMode =
        event.target.value === "concordant" ? "concordant" : "exact";
      setRunProofMode(nextValue);
      setStatusMessage(null);
      setErrorMessage(null);

      if (
        nextValue === "concordant" &&
        (runEpsilonInputError !== null || runEpsilonValue === null)
      ) {
        runEpsilonInputRef.current?.focus();
        return;
      }

      void persistRunSettings(nextValue, runEpsilonValue);
    },
    [runEpsilonInputError, runEpsilonValue, persistRunSettings],
  );

  const handleRunEpsilonChange = React.useCallback(
    (event: React.ChangeEvent<HTMLInputElement>) => {
      setRunEpsilonInput(event.target.value);
    },
    [],
  );

  const handleRunEpsilonBlur = React.useCallback(() => {
    if (runEpsilonInputError !== null) {
      return;
    }
    if (runProofMode === "concordant" && runEpsilonValue === null) {
      runEpsilonInputRef.current?.focus();
      return;
    }
    void persistRunSettings(runProofMode, runEpsilonValue);
  }, [runEpsilonInputError, runProofMode, runEpsilonValue, persistRunSettings]);

  const handleAddCheckpoint = React.useCallback(() => {
    if (!selectedRunId) {
      return;
    }
    setStatusMessage(null);
    setErrorMessage(null);
    setActiveEditor({ mode: "create" });
  }, [selectedRunId]);

  const handleEditCheckpoint = React.useCallback((config: RunCheckpointConfig) => {
    setStatusMessage(null);
    setErrorMessage(null);
    setActiveEditor({ mode: "edit", checkpoint: config });
  }, []);

  const handleCancelEditor = React.useCallback(() => {
    setActiveEditor(null);
    setEditorSubmitting(false);
  }, []);

  const handleEditorSubmit = React.useCallback(
    async (formValue: CheckpointFormValue) => {
      if (!selectedRunId || !activeEditor) {
        return;
      }
      setEditorSubmitting(true);
      setStatusMessage(null);
      setErrorMessage(null);
      const payload = sanitizeCheckpointFormValue(formValue, combinedModelOptions);
      try {
        if (activeEditor.mode === "create") {
          const created = await createCheckpointConfig(selectedRunId, payload);
          setCheckpointConfigs((previous) => {
            const next = [...previous, created];
            next.sort((a, b) => a.orderIndex - b.orderIndex);
            return next;
          });
          setStatusMessage("Checkpoint added.");
          setCostsRefreshToken((token) => token + 1);
        } else {
          const updated = await updateCheckpointConfig(activeEditor.checkpoint.id, payload);
          setCheckpointConfigs((previous) => {
            const next = previous.map((item) => (item.id === updated.id ? updated : item));
            next.sort((a, b) => a.orderIndex - b.orderIndex);
            return next;
          });
          setStatusMessage("Checkpoint updated.");
          setCostsRefreshToken((token) => token + 1);
        }
        setActiveEditor(null);
      } catch (err) {
        console.error("Failed to save checkpoint configuration", err);
        const message =
          err instanceof Error ? err.message : "Unable to save checkpoint configuration.";
        setErrorMessage(message);
      } finally {
        setEditorSubmitting(false);
      }
    },
    [activeEditor, combinedModelOptions, selectedRunId],
  );

  const handleDeleteCheckpoint = React.useCallback(
    async (config: RunCheckpointConfig) => {
      if (!selectedRunId) {
        return;
      }
      const confirmed = window.confirm(
        `Delete checkpoint "${config.checkpointType}" from this run?`,
      );
      if (!confirmed) {
        return;
      }
      setStatusMessage(null);
      setErrorMessage(null);
      const previous = checkpointConfigs.map((item) => ({ ...item }));
      const optimistic = checkpointConfigs
        .filter((item) => item.id !== config.id)
        .map((item, index) => ({ ...item, orderIndex: index }));
      setCheckpointConfigs(optimistic);
      try {
        await deleteCheckpointConfig(config.id);
        setStatusMessage("Checkpoint deleted.");
        setActiveEditor(null);
        setConfigsRefreshToken((token) => token + 1);
        setCostsRefreshToken((token) => token + 1);
      } catch (err) {
        console.error("Failed to delete checkpoint configuration", err);
        const message =
          err instanceof Error ? err.message : "Unable to delete checkpoint configuration.";
        setErrorMessage(message);
        setCheckpointConfigs(previous);
      }
    },
    [checkpointConfigs, selectedRunId],
  );

  const handleReorderCheckpoint = React.useCallback(
    async (index: number, offset: number) => {
      if (!selectedRunId) {
        return;
      }
      const targetIndex = index + offset;
      if (targetIndex < 0 || targetIndex >= checkpointConfigs.length) {
        return;
      }
      setStatusMessage(null);
      setErrorMessage(null);
      const original = checkpointConfigs.map((item) => ({ ...item }));
      const reordered = checkpointConfigs.map((item) => ({ ...item }));
      const [moved] = reordered.splice(index, 1);
      reordered.splice(targetIndex, 0, moved);
      const optimistic = reordered.map((item, position) => ({ ...item, orderIndex: position }));
      setCheckpointConfigs(optimistic);
      try {
        const updated = await reorderCheckpointConfigs(
          selectedRunId,
          optimistic.map((item) => item.id),
        );
        setCheckpointConfigs(updated);
        setStatusMessage("Checkpoint order updated.");
        setCostsRefreshToken((token) => token + 1);
      } catch (err) {
        console.error("Failed to reorder checkpoints", err);
        const message = err instanceof Error ? err.message : "Unable to reorder checkpoints.";
        setErrorMessage(message);
        setCheckpointConfigs(original);
      }
    },
    [checkpointConfigs, selectedRunId],
  );

  const handleExecuteRun = React.useCallback(async () => {
    if (!selectedRunId) {
      return;
    }
    if (epsilonRequirementUnmet) {
      setErrorMessage(
        "Provide a run epsilon between 0 and 1 before executing a concordant run.",
      );
      runEpsilonInputRef.current?.focus();
      return;
    }
    setStatusMessage(null);
    setErrorMessage(null);
    setExecutingRun(true);
    try {
      await startRun(selectedRunId);
      setStatusMessage("Run executed successfully.");
      onRunExecuted?.(selectedRunId);
    } catch (err) {
      console.error("Failed to execute run", err);
      const message = err instanceof Error ? err.message : "Unable to execute run.";
      setErrorMessage(message);
    } finally {
      setExecutingRun(false);
    }
  }, [selectedRunId, onRunExecuted, epsilonRequirementUnmet]);

  const handleReopenRun = React.useCallback(async () => {
    if (!selectedRunId) {
      return;
    }
    setStatusMessage(null);
    setErrorMessage(null);
    setActiveEditor(null);
    setEditorSubmitting(false);
    setConversationContext(null);
    setReopeningRun(true);
    try {
      await reopenRun(selectedRunId);
      setStatusMessage("Run reopened—ready to execute again.");
      setConfigsRefreshToken((token) => token + 1);
      setCostsRefreshToken((token) => token + 1);
      onRunsMutated?.();
    } catch (err) {
      console.error("Failed to reopen run", err);
      const message = err instanceof Error ? err.message : "Unable to reopen run.";
      setErrorMessage(message);
    } finally {
      setReopeningRun(false);
    }
  }, [selectedRunId, onRunsMutated]);

  const handleCloneRun = React.useCallback(async () => {
    if (!selectedRunId) {
      return;
    }
    setStatusMessage(null);
    setErrorMessage(null);
    setCloningRun(true);
    try {
      const clonedRunId = await cloneRun(selectedRunId);

      let runList: RunSummary[] | null = null;
      try {
        runList = await listRuns(projectId);
      } catch (refreshErr) {
        console.error("Failed to refresh run list after clone", refreshErr);
        setRunsError(
          "Run was cloned, but the run list could not be refreshed automatically.",
        );
      }

      if (runList) {
        setRuns(runList);
        setRunsError(null);
      } else {
        const cloneName = selectedRun
          ? `${selectedRun.name} (clone)`
          : "Cloned run";
        const fallbackKind = selectedRun?.kind ?? "exact";
        const fallbackCreatedAt = new Date().toISOString();
        setRuns((previous) => {
          const filtered = previous.filter((item) => item.id !== clonedRunId);
          return [
            {
              id: clonedRunId,
              name: cloneName,
              createdAt: fallbackCreatedAt,
              kind: fallbackKind,
              epsilon: selectedRun?.epsilon ?? null,
              hasPersistedCheckpoint: false,
            },
            ...filtered,
          ];
        });
      }

      setStatusMessage("Run cloned. Switched to the duplicate for editing.");
      onRunExecuted?.(clonedRunId);
      onRunsMutated?.();
    } catch (err) {
      console.error("Failed to clone run", err);
      const message = err instanceof Error ? err.message : "Unable to clone run.";
      setErrorMessage(message);
    } finally {
      setCloningRun(false);
    }
  }, [
    selectedRunId,
    projectId,
    onRunExecuted,
    onRunsMutated,
    selectedRun,
  ]);

  const handleOpenInteractiveCheckpoint = React.useCallback(
    (config: RunCheckpointConfig) => {
      if (!selectedRunId || !interactiveSupport) {
        return;
      }
      setStatusMessage(null);
      setErrorMessage(null);
      setConversationContext({ runId: selectedRunId, checkpointId: config.id });
    },
    [interactiveSupport, selectedRunId],
  );

  const handleConversationExit = React.useCallback(() => {
    setConversationContext(null);
  }, []);

  const runActionPending =
    executingRun || reopeningRun || cloningRun || runSettingsSaving;
  const disableExecute =
    !selectedRun ||
    runActionPending ||
    checkpointConfigs.length === 0 ||
    epsilonRequirementUnmet;

  return (
    <div>
      <h2>Workflow Builder</h2>
      <div style={{ fontSize: "0.85rem", marginBottom: "0.75rem", color: "#9cdcfe" }}>
        Project: {projectId}
      </div>
      {conversationContext && interactiveSupport ? (
        <InteractiveConversationView
          runId={conversationContext.runId}
          checkpointId={conversationContext.checkpointId}
          onExit={handleConversationExit}
          openSession={interactiveSupport.openSession}
          submitTurn={interactiveSupport.submitTurn}
          finalizeSession={interactiveSupport.finalizeSession}
        />
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: "16px" }}>
          <section style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                gap: "8px",
                alignItems: "flex-end",
              }}
            >
              <label style={{ display: "flex", flexDirection: "column", gap: "4px", flex: 1 }}>
                Run
                <select
                  value={selectedRunId ?? ""}
                  onChange={(event) => onSelectRun(event.target.value || null)}
                  disabled={runsLoading || runs.length === 0}
                >
                  <option value="" disabled>
                    {runsLoading ? "Loading…" : "Select a run"}
                  </option>
                  {runs.map((run) => (
                    <option key={run.id} value={run.id}>
                      {`${run.name} · ${formatRunTimestamp(run.createdAt)}`}
                    </option>
                  ))}
                </select>
              </label>
              <label style={{ display: "flex", flexDirection: "column", gap: "4px", flex: 1 }}>
                New run name
                <input
                  type="text"
                  value={newRunName}
                  onChange={(event) => {
                    setNewRunName(event.target.value);
                    if (newRunNameError) {
                      setNewRunNameError(null);
                    }
                  }}
                  placeholder="Optional"
                  maxLength={MAX_RUN_NAME_LENGTH}
                  disabled={creatingRun}
                />
              </label>
              <button
                type="button"
                onClick={handleCreateRun}
                disabled={creatingRun || runsLoading}
              >
                {creatingRun ? "Creating…" : "+ New run"}
              </button>
            </div>
            <div style={{ display: "flex", gap: "8px", flexWrap: "wrap" }}>
              <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
                New run proof mode
                <select
                  value={newRunProofMode}
                  onChange={(event) => {
                    const nextValue =
                      event.target.value === "concordant" ? "concordant" : "exact";
                    setNewRunProofMode(nextValue);
                    setNewRunEpsilonError(null);
                  }}
                  disabled={creatingRun}
                >
                  <option value="exact">Exact</option>
                  <option value="concordant">Concordant</option>
                </select>
              </label>
              <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
                New run ε (0–1)
                <input
                  type="number"
                  min={0}
                  max={1}
                  step={0.01}
                  value={newRunEpsilon}
                  onChange={(event) => {
                    setNewRunEpsilon(event.target.value);
                    setNewRunEpsilonError(null);
                  }}
                  disabled={creatingRun}
                />
              </label>
            </div>
            {newRunNameError && (
              <span style={{ color: "#f48771", fontSize: "0.85rem" }}>{newRunNameError}</span>
            )}
            {(newRunEpsilonError || newRunEpsilonEvaluation.error) && (
              <span style={{ color: "#f48771", fontSize: "0.85rem" }}>
                {newRunEpsilonError ?? newRunEpsilonEvaluation.error}
              </span>
            )}
            {runsError && <span style={{ color: "#f48771" }}>{runsError}</span>}
            {!runsLoading && runs.length === 0 ? (
              <span style={{ fontSize: "0.85rem", color: "#808080" }}>
                No runs found for this project.
              </span>
            ) : null}
          </section>

          {selectedRun ? (
            <>
              <section
                style={{
                  border: "1px solid #333",
                  borderRadius: "8px",
                  padding: "12px",
                  display: "flex",
                  flexDirection: "column",
                  gap: "8px",
                }}
              >
                <div style={{ fontSize: "1rem", fontWeight: 600 }}>Run metadata</div>
                <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))", gap: "8px" }}>
                  <div>
                    <div style={{ fontSize: "0.75rem", color: "#9cdcfe" }}>Name</div>
                    <div>{selectedRun.name}</div>
                  </div>
                  <div>
                    <div style={{ fontSize: "0.75rem", color: "#9cdcfe" }}>Proof mode</div>
                    <select
                      value={runProofMode}
                      onChange={handleRunProofModeChange}
                      disabled={runSettingsSaving}
                    >
                      <option value="exact">Exact</option>
                      <option value="concordant">Concordant</option>
                    </select>
                  </div>
                  <div>
                    <div style={{ fontSize: "0.75rem", color: "#9cdcfe" }}>Run ε (0–1)</div>
                    <input
                      ref={runEpsilonInputRef}
                      type="number"
                      min={0}
                      max={1}
                      step={0.01}
                      value={runEpsilonInput}
                      onChange={handleRunEpsilonChange}
                      onBlur={handleRunEpsilonBlur}
                      disabled={runSettingsSaving}
                    />
                    {runEpsilonInputError && (
                      <div style={{ color: "#f48771", fontSize: "0.75rem" }}>
                        {runEpsilonInputError}
                      </div>
                    )}
                    {!runEpsilonInputError && epsilonRequirementUnmet && (
                      <div style={{ color: "#f48771", fontSize: "0.75rem" }}>
                        Set a run epsilon between 0 and 1 for concordant proof.
                      </div>
                    )}
                  </div>
                  <div>
                    <div style={{ fontSize: "0.75rem", color: "#9cdcfe" }}>Created</div>
                    <div>{formatRunTimestamp(selectedRun.createdAt)}</div>
                  </div>
                  <div>
                    <div style={{ fontSize: "0.75rem", color: "#9cdcfe" }}>Run ID</div>
                    <div style={{ fontFamily: "monospace", fontSize: "0.8rem" }}>{selectedRun.id}</div>
                  </div>
                </div>
                <div style={{ display: "flex", gap: "8px", flexWrap: "wrap" }}>
                  <button type="button" onClick={handleExecuteRun} disabled={disableExecute}>
                    {executingRun ? "Executing…" : "Execute Full Run"}
                  </button>
                  <button
                    type="button"
                    onClick={handleReopenRun}
                    disabled={!selectedRunId || runActionPending}
                  >
                    {reopeningRun ? "Reopening…" : "Reopen run"}
                  </button>
                  <button
                    type="button"
                    onClick={handleCloneRun}
                    disabled={!selectedRunId || runActionPending}
                  >
                    {cloningRun ? "Cloning…" : "Clone run"}
                  </button>
                </div>
                {hasInteractiveCheckpoint && interactiveSupport && (
                  <span style={{ fontSize: "0.8rem", color: "#c586c0" }}>
                    This run contains interactive checkpoints. Manage them via the chat controls below.
                  </span>
                )}
                {hasInteractiveCheckpoint && !interactiveSupport && (
                  <span style={{ fontSize: "0.8rem", color: "#c586c0" }}>
                    This run includes interactive checkpoints, but chat controls are disabled in this build.
                  </span>
                )}
              </section>

              {statusMessage && (
                <div style={{ color: "#b5cea8" }}>{statusMessage}</div>
              )}
              {errorMessage && <div style={{ color: "#f48771" }}>{errorMessage}</div>}

              <section style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <div style={{ fontSize: "1rem", fontWeight: 600 }}>Checkpoint sequence</div>
                  <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
                    {modelsLoading && (
                      <span style={{ fontSize: "0.75rem", color: "#9cdcfe" }}>Loading models…</span>
                    )}
                    {modelsError && (
                      <span style={{ fontSize: "0.75rem", color: "#f48771" }}>{modelsError}</span>
                    )}
                    <button
                      type="button"
                      onClick={handleAddCheckpoint}
                      disabled={!selectedRunId || activeEditor !== null || editorSubmitting}
                    >
                      + Add checkpoint
                    </button>
                  </div>
                </div>
                {costEstimateLoading && (
                  <div style={{ color: "#9cdcfe", fontSize: "0.8rem" }}>
                    Estimating projected run costs…
                  </div>
                )}
                {costEstimateError && <div style={{ color: "#f48771" }}>{costEstimateError}</div>}
                {costOverrunMessages.length > 0 && costEstimates && (
                  <div
                    style={{
                      backgroundColor: "#402020",
                      border: "1px solid #7f1d1d",
                      color: "#f8caca",
                      padding: "10px",
                      borderRadius: "6px",
                      fontSize: "0.85rem",
                      display: "flex",
                      flexDirection: "column",
                      gap: "6px",
                    }}
                  >
                    <div style={{ fontWeight: 600, color: "#f48771" }}>
                      Projected costs exceed policy budgets.
                    </div>
                    <ul style={{ margin: 0, paddingLeft: "18px" }}>
                      {costOverrunMessages.map((message) => (
                        <li key={message} style={{ marginBottom: "2px" }}>
                          {message}
                        </li>
                      ))}
                    </ul>
                    <div>
                      Estimated usage totals {costEstimates.estimatedTokens.toLocaleString()} tokens (~
                      ${costEstimates.estimatedUsd.toFixed(2)}, {costEstimates.estimatedGCo2e.toFixed(2)} gCO₂e).
                      Adjust checkpoint token budgets or update the project policy before launching this run.
                    </div>
                  </div>
                )}
                {configsError && <div style={{ color: "#f48771" }}>{configsError}</div>}
                {configsLoading ? (
                  <div style={{ color: "#9cdcfe" }}>Loading checkpoint configurations…</div>
                ) : checkpointConfigs.length > 0 ? (
                  <div style={{ display: "flex", flexDirection: "column", gap: "10px" }}>
                    {checkpointConfigs.map((config, index) => (
                      <CheckpointListItem
                        key={config.id}
                        config={config}
                        onEdit={handleEditCheckpoint}
                        onDelete={handleDeleteCheckpoint}
                        onMoveUp={() => handleReorderCheckpoint(index, -1)}
                        onMoveDown={() => handleReorderCheckpoint(index, 1)}
                        isFirst={index === 0}
                        isLast={index === checkpointConfigs.length - 1}
                        onOpenInteractive={
                          interactiveSupport ? handleOpenInteractiveCheckpoint : undefined
                        }
                      />
                    ))}
                  </div>
                ) : (
                  <div style={{ fontSize: "0.85rem", color: "#808080" }}>
                    No checkpoints configured for this run yet.
                  </div>
                )}
              </section>

              {activeEditor && (
                <CheckpointEditor
                  availableModels={combinedModelOptions}
                  initialValue={
                    activeEditor.mode === "edit"
                      ? {
                          checkpointType: activeEditor.checkpoint.checkpointType,
                          model: activeEditor.checkpoint.model,
                          tokenBudget: activeEditor.checkpoint.tokenBudget,
                          prompt: activeEditor.checkpoint.prompt,
                          proofMode: activeEditor.checkpoint.proofMode,
                        }
                      : undefined
                  }
                  mode={activeEditor.mode}
                  onSubmit={handleEditorSubmit}
                  onCancel={handleCancelEditor}
                  submitting={editorSubmitting}
                  runEpsilon={runEpsilonValue}
                  epsilonRequired={epsilonRequired}
                  onRequestRunEpsilon={() => runEpsilonInputRef.current?.focus()}
                />
              )}
            </>
          ) : (
            <div style={{ fontSize: "0.85rem", color: "#808080" }}>
              Select a run to manage its checkpoint sequence.
            </div>
          )}
        </div>
      )}
    </div>
  );
}
