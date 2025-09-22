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
  startRun,
  openInteractiveCheckpointSession,
  submitInteractiveCheckpointTurn,
  finalizeInteractiveCheckpoint,
} from "../lib/api";
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
}

function InteractiveConversationView({
  runId,
  checkpointId,
  onExit,
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
    const session = await openInteractiveCheckpointSession(runId, checkpointId);
    return session;
  }, [runId, checkpointId]);

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
      await submitInteractiveCheckpointTurn(runId, checkpointId, trimmed);
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
      await finalizeInteractiveCheckpoint(runId, checkpointId);
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
  return {
    model,
    prompt,
    tokenBudget,
    checkpointType,
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
}

export default function EditorPanel({
  projectId,
  selectedRunId,
  onSelectRun,
  refreshToken,
  onRunExecuted,
}: EditorPanelProps) {
  const [runs, setRuns] = React.useState<RunSummary[]>([]);
  const [runsLoading, setRunsLoading] = React.useState(false);
  const [runsError, setRunsError] = React.useState<string | null>(null);

  const [checkpointConfigs, setCheckpointConfigs] = React.useState<RunCheckpointConfig[]>([]);
  const [configsLoading, setConfigsLoading] = React.useState(false);
  const [configsError, setConfigsError] = React.useState<string | null>(null);
  const [configsRefreshToken, setConfigsRefreshToken] = React.useState(0);

  const [availableModels, setAvailableModels] = React.useState<string[]>(["stub-model"]);
  const [modelsLoading, setModelsLoading] = React.useState(false);
  const [modelsError, setModelsError] = React.useState<string | null>(null);

  const [activeEditor, setActiveEditor] = React.useState<EditorState | null>(null);
  const [editorSubmitting, setEditorSubmitting] = React.useState(false);

  const [statusMessage, setStatusMessage] = React.useState<string | null>(null);
  const [errorMessage, setErrorMessage] = React.useState<string | null>(null);
  const [executingRun, setExecutingRun] = React.useState(false);

  const [conversationContext, setConversationContext] = React.useState<
    { runId: string; checkpointId: string } | null
  >(null);

  const combinedModelOptions = React.useMemo(() => {
    const set = new Set<string>(availableModels);
    for (const config of checkpointConfigs) {
      set.add(config.model);
    }
    return Array.from(set).sort();
  }, [availableModels, checkpointConfigs]);

  const selectedRun = React.useMemo(() => {
    if (!selectedRunId) {
      return null;
    }
    return runs.find((run) => run.id === selectedRunId) ?? null;
  }, [runs, selectedRunId]);

  const hasInteractiveCheckpoint = React.useMemo(() => {
    return checkpointConfigs.some(
      (config) => config.checkpointType.trim().toLowerCase() === 'interactivechat'.toLowerCase(),
    );
  }, [checkpointConfigs]);

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
        } else {
          const updated = await updateCheckpointConfig(activeEditor.checkpoint.id, payload);
          setCheckpointConfigs((previous) => {
            const next = previous.map((item) => (item.id === updated.id ? updated : item));
            next.sort((a, b) => a.orderIndex - b.orderIndex);
            return next;
          });
          setStatusMessage("Checkpoint updated.");
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
  }, [selectedRunId, onRunExecuted]);

  const handleOpenInteractiveCheckpoint = React.useCallback(
    (config: RunCheckpointConfig) => {
      if (!selectedRunId) {
        return;
      }
      setStatusMessage(null);
      setErrorMessage(null);
      setConversationContext({ runId: selectedRunId, checkpointId: config.id });
    },
    [selectedRunId],
  );

  const handleConversationExit = React.useCallback(() => {
    setConversationContext(null);
  }, []);

  const disableExecute =
    !selectedRun || executingRun || checkpointConfigs.length === 0;

  return (
    <div>
      <h2>Workflow Builder</h2>
      <div style={{ fontSize: "0.85rem", marginBottom: "0.75rem", color: "#9cdcfe" }}>
        Project: {projectId}
      </div>
      {conversationContext ? (
        <InteractiveConversationView
          runId={conversationContext.runId}
          checkpointId={conversationContext.checkpointId}
          onExit={handleConversationExit}
        />
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: "16px" }}>
          <section style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
            <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
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
                    <div>{proofModeLabel(selectedRun.kind)}</div>
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
                </div>
                {hasInteractiveCheckpoint && (
                  <span style={{ fontSize: "0.8rem", color: "#c586c0" }}>
                    This run contains interactive checkpoints. Manage them via the chat controls below.
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
                        onOpenInteractive={handleOpenInteractiveCheckpoint}
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
                        }
                      : undefined
                  }
                  mode={activeEditor.mode}
                  onSubmit={handleEditorSubmit}
                  onCancel={handleCancelEditor}
                  submitting={editorSubmitting}
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
