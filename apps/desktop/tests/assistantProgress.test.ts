import assert from "node:assert/strict";
import test from "node:test";
import { elapsedLabel, matchesAssistantProgress } from "../src/lib/assistantProgress.ts";
import type { AssistantProgress } from "../src/types.ts";

const progress: AssistantProgress = { progressId: "request-1", sessionId: "session-1", phase: "querying", round: 1, capability: "applications.list" };
test("progress belongs to the active request and session, not an earlier conversation", () => {
  assert.equal(matchesAssistantProgress(progress, "request-1", "session-1"), true);
  assert.equal(matchesAssistantProgress(progress, "request-2", "session-1"), false);
  assert.equal(matchesAssistantProgress(progress, "request-1", "session-2"), false);
  assert.equal(matchesAssistantProgress(progress, null, "session-1"), false);
  assert.equal(matchesAssistantProgress({ ...progress, round: 5 }, "request-1", "session-1"), false);
  assert.equal(matchesAssistantProgress(null, "request-1", "session-1"), false);
  assert.equal(matchesAssistantProgress({ ...progress, phase: "pretend-complete" }, "request-1", "session-1"), false);
  assert.equal(matchesAssistantProgress({ ...progress, capability: {} }, "request-1", "session-1"), false);
});
test("elapsed time remains readable for slow CLI replies without a fake percentage", () => {
  assert.equal(elapsedLabel(1000, 1000), "0s");
  assert.equal(elapsedLabel(1000, 62000), "1m 1s");
  assert.equal(elapsedLabel(1000, 601000), "10m 0s");
  assert.equal(elapsedLabel(1000, 0), "0s");
});
