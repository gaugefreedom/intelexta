import React from "react";
import {
  CheckpointSummary,
  listCheckpoints,
  listLocalModels,
  RunProofMode,
  startHelloRun,
  submitTurn,
} from "../lib/api";

function generateRandomSeed(): number {
  if (typeof crypto !== "undefined" && typeof crypto.getRandomValues === "function") {
    const array = new Uint32Array(1);
    crypto.getRandomValues(array);
    return array[0];
  }

  return Math.floor(Math.random() * 1_000_000_000);
}

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
  onExit: () => void;
}

function InteractiveConversationView({ runId, onExit }: InteractiveConversationViewProps) {
  const [messages, setMessages] = React.useState<ConversationMessage[]>([]);
  const [messagesLoading, setMessagesLoading] = React.useState<boolean>(false);
  const [messagesError, setMessagesError] = React.useState<string | null>(null);
  const [composerValue, setComposerValue] = React.useState<string>("");
  const [composerError, setComposerError] = React.useState<string | null>(null);
  const [isSending, setIsSending] = React.useState<boolean>(false);
  const scrollContainerRef = React.useRef<HTMLDivElement | null>(null);

  const refreshMessages = React.useCallback(async () => {
    const checkpoints = await listCheckpoints(runId);
    return extractMessagesFromCheckpoints(checkpoints);
  }, [runId]);

  React.useEffect(() => {
    setMessages([]);
    setComposerValue("");
    setComposerError(null);
    setMessagesError(null);
  }, [runId]);

  React.useEffect(() => {
    let cancelled = false;
    setMessagesLoading(true);
    setMessagesError(null);

    refreshMessages()
      .then((items) => {
        if (!cancelled) {
          setMessages(items);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          console.error("Failed to load conversation history", err);
          const message =
            err instanceof Error
              ? err.message
              : "Failed to load conversation history.";
          setMessagesError(message);
        }
      })
      .finally(() => {
        if (!cancelled) {
          setMessagesLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [refreshMessages]);

  React.useEffect(() => {
    const container = scrollContainerRef.current;
    if (container) {
      container.scrollTop = container.scrollHeight;
    }
  }, [messages]);

  const handleComposerSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const trimmed = composerValue.trim();
    if (!trimmed) {
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
      await submitTurn(runId, trimmed);
      const updatedMessages = await refreshMessages();
      setMessages(updatedMessages);
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

  const disableSend = isSending || composerValue.trim().length === 0;

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
        <div>
          <div style={{ fontSize: "1rem", fontWeight: 600 }}>Interactive conversation</div>
          <div style={{ fontSize: "0.8rem", color: "#9cdcfe" }}>Run ID: {runId}</div>
        </div>
        <button type="button" onClick={onExit}>
          Return to configuration
        </button>
      </div>

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
            disabled={isSending}
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

interface EditorPanelProps {
  projectId: string;
  onRunStarted?: (runId: string) => void;
}

export default function EditorPanel({ projectId, onRunStarted }: EditorPanelProps) {
  const [name, setName] = React.useState("");
  const [seed, setSeed] = React.useState(() => String(generateRandomSeed()));
  const [dagJson, setDagJson] = React.useState("");
  const [tokenBudget, setTokenBudget] = React.useState("");
  const [model, setModel] = React.useState("stub-model");
  const [availableModels, setAvailableModels] = React.useState<string[]>(["stub-model"]);
  const [modelsLoading, setModelsLoading] = React.useState(false);
  const [modelsError, setModelsError] = React.useState<string | null>(null);
  const [proofMode, setProofMode] = React.useState<RunProofMode>("exact");
  const [uiState, setUiState] = React.useState<"configure" | "conversation">("configure");
  const [activeRunId, setActiveRunId] = React.useState<string | null>(null);
  const [isInteractiveSession, setIsInteractiveSession] = React.useState(false);
  const [epsilon, setEpsilon] = React.useState(0.15);
  const [formError, setFormError] = React.useState<string | null>(null);
  const [successMessage, setSuccessMessage] = React.useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = React.useState(false);

  const handleConversationExit = React.useCallback(() => {
    setActiveRunId(null);
    setIsInteractiveSession(false);
    setUiState("configure");
    setSuccessMessage(null);
    setFormError(null);
  }, []);

  React.useEffect(() => {
    let cancelled = false;
    setModelsLoading(true);
    setModelsError(null);
    listLocalModels()
      .then((models) => {
        if (cancelled) return;
        const filtered = models.filter((entry) => entry.trim().length > 0);
        if (filtered.length === 0) {
          setAvailableModels(["stub-model"]);
          setModel("stub-model");
          return;
        }
        setAvailableModels(filtered);
        setModel((current) => (filtered.includes(current) ? current : filtered[0]));
      })
      .catch((err) => {
        if (cancelled) return;
        console.error("Failed to load local models", err);
        setModelsError("Unable to load local models. Falling back to defaults.");
        setAvailableModels(["stub-model"]);
        setModel("stub-model");
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

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setFormError(null);
    setSuccessMessage(null);

    const trimmedName = name.trim();
    if (!trimmedName) {
      setFormError("Run name is required.");
      return;
    }

    const seedInput = seed.trim();
    if (!seedInput) {
      setFormError("Seed is required.");
      return;
    }
    const parsedSeed = Number(seedInput);
    if (
      !Number.isFinite(parsedSeed) ||
      !Number.isInteger(parsedSeed) ||
      parsedSeed < 0 ||
      parsedSeed > Number.MAX_SAFE_INTEGER
    ) {
      setFormError("Seed must be a non-negative integer within JavaScript's safe range.");
      return;
    }

    const dagJsonInput = dagJson.trim();
    if (!dagJsonInput) {
      setFormError("DAG JSON is required.");
      return;
    }
    try {
      JSON.parse(dagJsonInput);
    } catch (err) {
      console.error("Invalid DAG JSON", err);
      setFormError("DAG JSON must be valid JSON.");
      return;
    }

    const modelInput = model.trim();
    if (!modelInput) {
      setFormError("Model identifier is required.");
      return;
    }

    const tokenBudgetInput = tokenBudget.trim();
    if (!tokenBudgetInput) {
      setFormError("Token budget is required.");
      return;
    }
    const parsedTokenBudget = Number(tokenBudgetInput);
    if (
      !Number.isFinite(parsedTokenBudget) ||
      !Number.isInteger(parsedTokenBudget) ||
      parsedTokenBudget < 0 ||
      parsedTokenBudget > Number.MAX_SAFE_INTEGER
    ) {
      setFormError("Token budget must be a non-negative integer within JavaScript's safe range.");
      return;
    }

    let epsilonValue: number | null = null;
    if (proofMode === "concordant") {
      epsilonValue = Number(epsilon);
      if (!Number.isFinite(epsilonValue) || epsilonValue < 0) {
        setFormError("Epsilon must be a finite, non-negative number.");
        return;
      }
    }

    setIsSubmitting(true);
    try {
      const runId = await startHelloRun({
        projectId,
        name: trimmedName,
        seed: parsedSeed,
        dagJson: dagJsonInput,
        tokenBudget: parsedTokenBudget,
        model: modelInput,
        proofMode,
        epsilon: epsilonValue,
      });
      if (proofMode === "interactive") {
        setActiveRunId(runId);
        setIsInteractiveSession(true);
        setUiState("conversation");
        setSuccessMessage(null);
      } else {
        setActiveRunId(null);
        setIsInteractiveSession(false);
        setUiState("configure");
        setSuccessMessage(`Run started successfully. ID: ${runId}`);
      }
      onRunStarted?.(runId);
    } catch (err) {
      console.error("Failed to start run", err);
      const message = err instanceof Error ? err.message : "Failed to start run.";
      setFormError(message);
    } finally {
      setIsSubmitting(false);
    }
  };

  const showConversation = uiState === "conversation" && isInteractiveSession && activeRunId;

  return (
    <div>
      <h2>Editor</h2>
      <div style={{ fontSize: "0.85rem", marginBottom: "0.75rem", color: "#9cdcfe" }}>
        Project: {projectId}
      </div>
      {showConversation ? (
        <InteractiveConversationView runId={activeRunId!} onExit={handleConversationExit} />
      ) : (
        <form
          onSubmit={handleSubmit}
          style={{
            display: "flex",
            flexDirection: "column",
            gap: "12px",
            maxWidth: "600px",
          }}
        >
          <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
            Run name
            <input
              type="text"
              value={name}
              onChange={(event) => setName(event.target.value)}
              placeholder="e.g. Hello world"
            />
          </label>

          <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
            Seed
            <input
              type="number"
              inputMode="numeric"
              value={seed}
              onChange={(event) => setSeed(event.target.value)}
              placeholder="0"
              min={0}
            />
          </label>

          <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
            Model
            <select
              value={model}
              onChange={(event) => setModel(event.target.value)}
              disabled={modelsLoading || availableModels.length === 0}
            >
              {availableModels.map((modelId) => (
                <option key={modelId} value={modelId}>
                  {modelId}
                </option>
              ))}
            </select>
            {modelsLoading ? (
              <span style={{ fontSize: "0.75rem", color: "#9cdcfe" }}>Loading local models…</span>
            ) : modelsError ? (
              <span style={{ fontSize: "0.75rem", color: "#f48771" }}>{modelsError}</span>
            ) : null}
          </label>

          <fieldset
            style={{
              border: "1px solid #333",
              borderRadius: "6px",
              padding: "8px 12px",
              display: "flex",
              flexDirection: "column",
              gap: "8px",
            }}
          >
            <legend style={{ padding: "0 4px" }}>Proof mode</legend>
            <label style={{ display: "flex", alignItems: "center", gap: "8px" }}>
              <input
                type="radio"
                name="proof-mode"
                value="exact"
                checked={proofMode === "exact"}
                onChange={() => setProofMode("exact")}
              />
              Exact
            </label>
            <label style={{ display: "flex", alignItems: "center", gap: "8px" }}>
              <input
                type="radio"
                name="proof-mode"
                value="concordant"
                checked={proofMode === "concordant"}
                onChange={() => setProofMode("concordant")}
              />
              Concordant
            </label>
            <label style={{ display: "flex", alignItems: "center", gap: "8px" }}>
              <input
                type="radio"
                name="proof-mode"
                value="interactive"
                checked={proofMode === "interactive"}
                onChange={() => setProofMode("interactive")}
              />
              Interactive
            </label>
            {proofMode === "concordant" && (
              <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
                <label style={{ fontSize: "0.85rem", color: "#9cdcfe" }}>
                  Semantic tolerance (ε)
                </label>
                <input
                  type="range"
                  min={0}
                  max={1}
                  step={0.01}
                  value={epsilon}
                  onChange={(event) => setEpsilon(Number(event.target.value))}
                />
                <div style={{ display: "flex", justifyContent: "space-between", fontSize: "0.8rem" }}>
                  <span>0.00</span>
                  <span>
                    ε = {epsilon.toFixed(2)}
                  </span>
                  <span>1.00</span>
                </div>
              </div>
            )}
            {proofMode === "interactive" && (
              <div style={{ fontSize: "0.8rem", color: "#4ec9b0" }}>
                Interactive runs open the conversational workspace after launch.
              </div>
            )}
          </fieldset>

          <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
            DAG JSON
            <textarea
              value={dagJson}
              onChange={(event) => setDagJson(event.target.value)}
              rows={8}
              placeholder='{ "nodes": [] }'
              style={{ fontFamily: "monospace" }}
            />
          </label>

          <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
            Token budget
            <input
              type="number"
              inputMode="numeric"
              value={tokenBudget}
              onChange={(event) => setTokenBudget(event.target.value)}
              placeholder="1000"
              min={0}
            />
          </label>

          <button type="submit" disabled={isSubmitting} style={{ alignSelf: "flex-start" }}>
            {isSubmitting ? "Starting…" : "Start run"}
          </button>

          {formError && <div style={{ color: "#f48771" }}>{formError}</div>}
          {successMessage && <div style={{ color: "#b5cea8" }}>{successMessage}</div>}
        </form>
      )}
    </div>
  );
}
