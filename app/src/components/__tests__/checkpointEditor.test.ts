import assert from "node:assert";
import test from "node:test";
import { concordantSubmissionAllowed } from "../CheckpointEditor.js";

test("concordant checkpoints require epsilon", () => {
  assert.strictEqual(concordantSubmissionAllowed("concordant", null), false);
  assert.strictEqual(concordantSubmissionAllowed("concordant", -0.1), false);
});

test("concordant checkpoints accept normalized epsilon", () => {
  assert.strictEqual(concordantSubmissionAllowed("concordant", 0.25), true);
  assert.strictEqual(concordantSubmissionAllowed("exact", null), true);
});
