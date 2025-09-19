import React from "react";
import { listLocalModels, startHelloRun, RunProofMode } from "../lib/api";

function generateRandomSeed(): number {
  if (typeof crypto !== "undefined" && typeof crypto.getRandomValues === "function") {
    const array = new Uint32Array(1);
    crypto.getRandomValues(array);
    return array[0];
  }

  return Math.floor(Math.random() * 1_000_000_000);
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
  const [epsilon, setEpsilon] = React.useState(0.15);
  const [formError, setFormError] = React.useState<string | null>(null);
  const [successMessage, setSuccessMessage] = React.useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = React.useState(false);

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
      setSuccessMessage(`Run started successfully. ID: ${runId}`);
      onRunStarted?.(runId);
    } catch (err) {
      console.error("Failed to start run", err);
      const message = err instanceof Error ? err.message : "Failed to start run.";
      setFormError(message);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div>
      <h2>Editor</h2>
      <div style={{ fontSize: "0.85rem", marginBottom: "0.75rem", color: "#9cdcfe" }}>
        Project: {projectId}
      </div>
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
    </div>
  );
}
