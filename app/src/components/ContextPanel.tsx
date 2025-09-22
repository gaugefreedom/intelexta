import React from "react";
import {
  getPolicy,
  updatePolicy,
  Policy,
  estimateRunCost,
  type RunCostEstimates,
} from "../lib/api";

interface ContextPanelProps {
  projectId: string;
  selectedRunId: string | null;
  onPolicyUpdated?: () => void;
}

export default function ContextPanel({
  projectId,
  selectedRunId,
  onPolicyUpdated,
}: ContextPanelProps) {
  const [policy, setPolicy] = React.useState<Policy | null>(null);
  const [loading, setLoading] = React.useState<boolean>(true);
  const [saving, setSaving] = React.useState<boolean>(false);
  const [status, setStatus] = React.useState<string | null>(null);
  const [error, setError] = React.useState<string | null>(null);

  const [costEstimates, setCostEstimates] = React.useState<RunCostEstimates | null>(null);
  const [costLoading, setCostLoading] = React.useState<boolean>(false);
  const [costError, setCostError] = React.useState<string | null>(null);
  const [costRefreshToken, setCostRefreshToken] = React.useState(0);

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
    setLoading(true);
    setStatus(null);
    setError(null);
    getPolicy(projectId)
      .then((policyData) => {
        if (!cancelled) {
          setPolicy(policyData);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          console.error("Failed to load policy", err);
          setError("Could not load policy settings.");
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [projectId]);

  React.useEffect(() => {
    if (!selectedRunId) {
      setCostEstimates(null);
      setCostError(null);
      setCostLoading(false);
      return;
    }

    let cancelled = false;
    setCostLoading(true);
    setCostError(null);

    estimateRunCost(selectedRunId)
      .then((estimates) => {
        if (!cancelled) {
          setCostEstimates(estimates);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          console.error("Failed to estimate run cost", err);
          const message =
            err instanceof Error ? err.message : "Unable to estimate projected run cost.";
          setCostError(message);
          setCostEstimates(null);
        }
      })
      .finally(() => {
        if (!cancelled) {
          setCostLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [selectedRunId, projectId, costRefreshToken, policy]);

  const handleToggle = (event: React.ChangeEvent<HTMLInputElement>) => {
    setPolicy((prev) => (prev ? { ...prev, allowNetwork: event.target.checked } : prev));
    setStatus(null);
  };

  const handleNumberChange = <K extends keyof Policy>(field: K) => (
    event: React.ChangeEvent<HTMLInputElement>
  ) => {
    const value = event.target.valueAsNumber;
    setPolicy((prev) => {
      if (!prev) return prev;
      if (Number.isNaN(value)) {
        return prev;
      }
      return { ...prev, [field]: value };
    });
    setStatus(null);
  };

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!policy) return;

    setSaving(true);
    setStatus(null);
    setError(null);

    try {
      await updatePolicy(projectId, policy);
      const refreshed = await getPolicy(projectId);
      setPolicy(refreshed);
      setStatus("Policy saved successfully.");
      setCostRefreshToken((token) => token + 1);
      onPolicyUpdated?.();
    } catch (err) {
      console.error("Failed to save policy", err);
      setError(`Could not save policy: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div>
      <h2>Context</h2>
      <div style={{ fontSize: "0.85rem", marginBottom: "0.5rem", color: "#9cdcfe" }}>
        Project: {projectId}
      </div>
      {selectedRunId ? (
        <div style={{ marginBottom: "0.75rem" }}>
          {costLoading && <div style={{ color: "#9cdcfe" }}>Estimating projected run costs…</div>}
          {costError && <div style={{ color: "#f48771" }}>{costError}</div>}
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
                Selected run exceeds policy budgets.
              </div>
              <ul style={{ margin: 0, paddingLeft: "18px" }}>
                {costOverrunMessages.map((message) => (
                  <li key={message} style={{ marginBottom: "2px" }}>
                    {message}
                  </li>
                ))}
              </ul>
              <div>
                Estimated totals: {costEstimates.estimatedTokens.toLocaleString()} tokens (~$
                {costEstimates.estimatedUsd.toFixed(2)}, {costEstimates.estimatedGCo2e.toFixed(2)} gCO₂e).
                Increase policy budgets or lower checkpoint token budgets before running.
              </div>
            </div>
          )}
        </div>
      ) : (
        <div style={{ fontSize: "0.8rem", color: "#808080", marginBottom: "0.75rem" }}>
          Select a run to review policy cost projections.
        </div>
      )}
      {loading ? (
        <p>Loading policy…</p>
      ) : policy ? (
        <form onSubmit={handleSubmit} style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
          <label style={{ display: "flex", alignItems: "center", gap: "8px" }}>
            <input type="checkbox" checked={policy.allowNetwork} onChange={handleToggle} />
            Allow outbound network access
          </label>

          <div style={{ display: "grid", gridTemplateColumns: "1fr", gap: "8px" }}>
            <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
              Token Budget
              <input
                type="number"
                min={0}
                step={1}
                value={policy.budgetTokens}
                onChange={handleNumberChange("budgetTokens")}
              />
            </label>
            <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
              USD Budget
              <input
                type="number"
                min={0}
                step={0.01}
                value={policy.budgetUsd}
                onChange={handleNumberChange("budgetUsd")}
              />
            </label>
            <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
              Carbon Budget (gCO₂e)
              <input
                type="number"
                min={0}
                step={0.01}
                value={policy.budgetGCo2e}
                onChange={handleNumberChange("budgetGCo2e")}
              />
            </label>
          </div>

          <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
            <button type="submit" disabled={saving}>
              {saving ? "Saving…" : "Save Policy"}
            </button>
            {status && <span style={{ color: "#6A9955" }}>{status}</span>}
            {error && <span style={{ color: "#f48771" }}>{error}</span>}
          </div>
        </form>
      ) : (
        <p>No policy found for this project.</p>
      )}
    </div>
  );
}
