import React from "react";
import {
  listCheckpoints,
  listRuns,
  CheckpointSummary,
  RunSummary,
} from "../lib/api";

export default function InspectorPanel({ projectId }: { projectId: string }) {
  const [runs, setRuns] = React.useState<RunSummary[]>([]);
  const [selectedRunId, setSelectedRunId] = React.useState<string | null>(null);
  const [checkpoints, setCheckpoints] = React.useState<CheckpointSummary[]>([]);
  const [loadingRuns, setLoadingRuns] = React.useState<boolean>(false);
  const [loadingCheckpoints, setLoadingCheckpoints] = React.useState<boolean>(false);
  const [runsError, setRunsError] = React.useState<string | null>(null);
  const [checkpointError, setCheckpointError] = React.useState<string | null>(null);

  React.useEffect(() => {
    let cancelled = false;
    setLoadingRuns(true);
    setRunsError(null);
    listRuns(projectId)
      .then((runList) => {
        if (cancelled) return;
        setRuns(runList);
        setSelectedRunId(runList.length > 0 ? runList[0].id : null);
      })
      .catch((err) => {
        if (cancelled) return;
        console.error("Failed to load runs", err);
        setRunsError("Could not load runs for this project.");
        setRuns([]);
        setSelectedRunId(null);
      })
      .finally(() => {
        if (!cancelled) {
          setLoadingRuns(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [projectId]);

  React.useEffect(() => {
    if (!selectedRunId) {
      setCheckpoints([]);
      return;
    }

    let cancelled = false;
    setLoadingCheckpoints(true);
    setCheckpointError(null);
    listCheckpoints(selectedRunId)
      .then((items) => {
        if (!cancelled) {
          setCheckpoints(items);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          console.error("Failed to load checkpoints", err);
          setCheckpointError("Could not load checkpoints for the selected run.");
          setCheckpoints([]);
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoadingCheckpoints(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [selectedRunId]);

  return (
    <div>
      <h2>Inspector</h2>
      <div style={{ fontSize: "0.85rem", marginBottom: "0.5rem", color: "#9cdcfe" }}>
        Project: {projectId}
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: "8px", marginBottom: "12px" }}>
        <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
          Run
          <select
            value={selectedRunId ?? ""}
            onChange={(event) => setSelectedRunId(event.target.value || null)}
            disabled={loadingRuns || runs.length === 0}
          >
            <option value="" disabled>
              {loadingRuns ? "Loading…" : "Select a run"}
            </option>
            {runs.map((run) => (
              <option key={run.id} value={run.id}>
                {run.name} · {new Date(run.created_at).toLocaleString()}
              </option>
            ))}
          </select>
        </label>
        {runsError && <span style={{ color: "#f48771" }}>{runsError}</span>}
        <button type="button" disabled style={{ alignSelf: "flex-start" }}>
          Emit CAR (coming soon)
        </button>
      </div>

      {loadingCheckpoints ? (
        <p>Loading checkpoints…</p>
      ) : selectedRunId ? (
        checkpoints.length > 0 ? (
          <div style={{ maxHeight: "60vh", overflowY: "auto" }}>
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "0.85rem" }}>
              <thead>
                <tr style={{ borderBottom: "1px solid #333" }}>
                  <th style={{ textAlign: "left", padding: "4px" }}>Timestamp</th>
                  <th style={{ textAlign: "left", padding: "4px" }}>Kind</th>
                  <th style={{ textAlign: "left", padding: "4px" }}>Inputs SHA</th>
                  <th style={{ textAlign: "left", padding: "4px" }}>Outputs SHA</th>
                  <th style={{ textAlign: "right", padding: "4px" }}>Usage</th>
                </tr>
              </thead>
              <tbody>
                {checkpoints.map((ckpt) => (
                  <tr key={ckpt.id} style={{ borderBottom: "1px solid #222" }}>
                    <td style={{ padding: "4px" }}>{new Date(ckpt.timestamp).toLocaleString()}</td>
                    <td style={{ padding: "4px" }}>{ckpt.kind}</td>
                    <td style={{ padding: "4px", fontFamily: "monospace" }}>
                      {ckpt.inputs_sha256 ?? "—"}
                    </td>
                    <td style={{ padding: "4px", fontFamily: "monospace" }}>
                      {ckpt.outputs_sha256 ?? "—"}
                    </td>
                    <td style={{ padding: "4px", textAlign: "right" }}>{ckpt.usage_tokens}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <p>No checkpoints recorded for this run yet.</p>
        )
      ) : (
        <p>Select a run to inspect its checkpoints.</p>
      )}

      {checkpointError && <div style={{ color: "#f48771", marginTop: "8px" }}>{checkpointError}</div>}
    </div>
  );
}
