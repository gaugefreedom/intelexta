import assert from "node:assert";
import test from "node:test";
import { buildCarCheckpointRows } from "../ContextPanel.js";
import type { ImportedCarSnapshot } from "../../lib/api.js";

test("buildCarCheckpointRows preserves checkpoint order and metadata", () => {
  const snapshot: ImportedCarSnapshot = {
    carId: "car:abc123",
    runId: "run-xyz",
    createdAt: "2024-01-01T00:00:00Z",
    run: {
      kind: "exact",
      name: "example",
      model: "stub-model",
      version: "v1",
      seed: 42,
      steps: [
        {
          id: "step-1",
          runId: "run-xyz",
          orderIndex: 0,
          checkpointType: "Step",
          model: "stub-model",
          prompt: "hello",
          tokenBudget: 128,
          proofMode: "exact",
          epsilon: null,
        },
      ],
      sampler: null,
    },
    proof: {
      matchKind: "exact",
      epsilon: null,
      distanceMetric: null,
      originalSemanticDigest: null,
      replaySemanticDigest: null,
      process: {
        sequentialCheckpoints: [
          {
            id: "ckpt:1",
            parentCheckpointId: null,
            turnIndex: null,
            prevChain: "prev-1",
            currChain: "curr-1",
            signature: "ed25519:sig-1",
          },
          {
            id: "ckpt:2",
            parentCheckpointId: "ckpt:1",
            turnIndex: 1,
            prevChain: "curr-1",
            currChain: "curr-2",
            signature: "ed25519:sig-2",
          },
        ],
      },
    },
    policyRef: {
      hash: "sha256:abc",
      egress: false,
      estimator: "usage_tokens * 0",
    },
    budgets: {
      usd: 0,
      tokens: 256,
      natureCost: 0,
    },
    provenance: [],
    checkpoints: [
      {
        id: "ckpt:1",
        parentCheckpointId: null,
        turnIndex: null,
        prevChain: "prev-1",
        currChain: "curr-1",
        signature: "ed25519:sig-1",
      },
      {
        id: "ckpt:2",
        parentCheckpointId: "ckpt:1",
        turnIndex: 1,
        prevChain: "curr-1",
        currChain: "curr-2",
        signature: "ed25519:sig-2",
      },
    ],
    sgrade: {
      score: 100,
      components: {
        provenance: 1,
        energy: 1,
        replay: 1,
        consent: 1,
        incidents: 1,
      },
    },
    signerPublicKey: "ZmFrZV9rZXk=",
  };

  const rows = buildCarCheckpointRows(snapshot);
  assert.strictEqual(rows.length, 2);
  assert.deepStrictEqual(rows[0], {
    id: "ckpt:1",
    order: 1,
    turnIndex: null,
    currChain: "curr-1",
    prevChain: "prev-1",
    signature: "ed25519:sig-1",
  });
  assert.deepStrictEqual(rows[1], {
    id: "ckpt:2",
    order: 2,
    turnIndex: 1,
    currChain: "curr-2",
    prevChain: "curr-1",
    signature: "ed25519:sig-2",
  });
});
