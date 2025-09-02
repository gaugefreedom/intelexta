import React from "react";

export default function ContextPanel({ projectId }: { projectId: string }) {
  return (
    <div>
      <h2>Context</h2>
      <div>Project: {projectId}</div>
      <ul>
        <li>Policies / Budgets</li>
        <li>Recent Checkpoints</li>
        <li>Participants</li>
      </ul>
    </div>
  );
}
