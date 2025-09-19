import React from "react";
import { startHelloRun } from "../lib/api";

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
  const [formError, setFormError] = React.useState<string | null>(null);
  const [successMessage, setSuccessMessage] = React.useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = React.useState(false);

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

    setIsSubmitting(true);
    try {
      const runId = await startHelloRun({
        projectId,
        name: trimmedName,
        seed: parsedSeed,
        dagJson: dagJsonInput,
        tokenBudget: parsedTokenBudget,
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
