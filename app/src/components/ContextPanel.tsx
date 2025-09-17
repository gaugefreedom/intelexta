import React from "react";
import { getPolicy, updatePolicy, Policy } from "../lib/api";

export default function ContextPanel({ projectId }: { projectId: string }) {
  const [policy, setPolicy] = React.useState<Policy | null>(null);
  const [loading, setLoading] = React.useState<boolean>(true);
  const [saving, setSaving] = React.useState<boolean>(false);
  const [status, setStatus] = React.useState<string | null>(null);
  const [error, setError] = React.useState<string | null>(null);

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
