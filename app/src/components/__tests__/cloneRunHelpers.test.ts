import assert from "node:assert";
import test from "node:test";
import { isCloneRunDisabled } from "../cloneRunHelpers.js";

test("clone run disabled when no checkpoints exist", () => {
  assert.strictEqual(isCloneRunDisabled("run-123", false, 0), true);
});

test("clone run enabled when checkpoints exist and no other blockers", () => {
  assert.strictEqual(isCloneRunDisabled("run-123", false, 2), false);
});
