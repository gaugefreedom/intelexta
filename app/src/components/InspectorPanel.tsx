import React from "react";

export default function InspectorPanel({ projectId }: { projectId: string }) {
  return (
    <div>
      <h2>Inspector</h2>
      <div>Project: {projectId}</div>
      <p>Properties / Provenance / Replay tools.</p>
    </div>
  );
}
