import React from "react";

export default function EditorPanel({ projectId }: { projectId: string }) {
  return (
    <div>
      <h2>Editor</h2>
      <div>Project: {projectId}</div>
      <p>Write or paste content here…</p>
    </div>
  );
}
