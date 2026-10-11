import assert from "node:assert/strict";
import test from "node:test";
import { appendAssistantEvidence, assistantTurnKey, boundedAssistantTurns, boundedAssistantFailureDetail, canRetryAssistantQuestion,
  MAX_ASSISTANT_DISPLAY_TURNS, MAX_ASSISTANT_EVIDENCE_PAYLOADS, MAX_ASSISTANT_PENDING_RESULTS, mergeAssistantToolResults } from "../src/lib/assistantTimeline.ts";
import type { AssistantQuestionRetry, AssistantTurnEvidence } from "../src/lib/assistantTimeline.ts";
import type { AppToolResult } from "../src/types.ts";

const result = (status: AppToolResult["status"], n = 1): AppToolResult => ({ source: "broomsweepy",
  capability: "applications.list", status, capturedAtUnixMs: n, truncated: false, data: {} });
const evidence = (status: AppToolResult["status"], n: number): AssistantTurnEvidence => ({
  id: `e-${n}`, turnKey: `a-${n}`, result: result(status, n),
});

test("stable turn keys survive later questions and bounded display eviction", () => {
  let key = 0;
  const turns = boundedAssistantTurns([{ role: "assistant", content: "answer" }, { role: "user", content: "next" }], () => `local-${++key}`);
  const first = assistantTurnKey(turns[0]);
  assert.equal(assistantTurnKey(boundedAssistantTurns([...turns, { role: "user", content: "later" }], () => `local-${++key}`)[0]), first);
  const many = Array.from({ length: MAX_ASSISTANT_DISPLAY_TURNS + 1 }, (_, sequence) => ({ role: "user" as const, content: "q", sequence }));
  const bounded = boundedAssistantTurns(many, () => "unused");
  assert.equal(bounded.length, MAX_ASSISTANT_DISPLAY_TURNS);
  assert.equal(assistantTurnKey(bounded[0]), "saved-1");
});

test("new answers preserve pending review authority but replace completed live results", () => {
  const pending = result("review_required");
  const oldRead = result("completed");
  const fresh = result("completed", 2);
  assert.deepEqual(mergeAssistantToolResults([pending, oldRead], [fresh]), [pending, fresh]);
  assert.deepEqual(mergeAssistantToolResults([pending], [pending]), [pending]);
  assert.equal(mergeAssistantToolResults(Array.from({ length: 30 }, (_, i) => result("review_required", i)), [fresh]).length, MAX_ASSISTANT_PENDING_RESULTS);
});

test("old evidence retains status without retaining native plan tokens or unbounded payloads", () => {
  const pending = evidence("review_required", 0);
  pending.result.presentation = { plan: { planId: "secret-token", path: "/private/local/path" } };
  pending.result.data = { items: [{ snippet: "private body", path: "/private/local/path" }], displayName: "Target" };
  const entries = appendAssistantEvidence([pending], Array.from({ length: 20 }, (_, i) => evidence("completed", i + 1)));
  assert.equal(entries[0].result.status, "review_required");
  assert.equal(entries[0].result.data.displayName, "Target");
  assert.equal(entries[0].result.presentation, null);
  assert.equal(entries[0].result.data.items, undefined);
  assert.equal(JSON.stringify(entries[0]).includes("secret-token"), false);
});

test("failure details are plain text and bounded before display and persistence", () => {
  const detail = boundedAssistantFailureDetail("**Failed**\n# More\n" + "x".repeat(10_000));
  assert.equal(detail.includes("**"), false);
  assert.ok(detail.length <= 2_000);
});

test("evidence stays attached to its answer and completed payloads have a fixed cap", () => {
  const pending = evidence("review_required", 0);
  const entries = Array.from({ length: MAX_ASSISTANT_EVIDENCE_PAYLOADS + 5 }, (_, i) => evidence("completed", i + 1));
  const next = appendAssistantEvidence([pending], entries);
  assert.equal(next.filter(entry => !entry.archived).length, MAX_ASSISTANT_EVIDENCE_PAYLOADS);
  assert.ok(next.some(entry => entry.id === pending.id && entry.result.status === "review_required"));
  assert.equal(next[0].result.presentation, null);
  assert.equal(next.at(-1)?.turnKey, entries.at(-1)?.turnKey);
  const permission = evidence("permission_required", 50);
  assert.ok(appendAssistantEvidence(next, [permission]).includes(permission));
});

test("retry is explicit and bound to an existing question, error, session and permission revision", () => {
  const retry: AssistantQuestionRetry = { errorTurnKey: "error", userTurnKey: "question", sessionId: "s1",
    permissionRevision: 2, workspaceRevision: 4, message: "q", history: [], cancelled: false };
  const turns = [{ id: "question", role: "user" as const, content: "q" }, { id: "error", role: "assistant" as const, content: "failed", failed: true }];
  assert.equal(canRetryAssistantQuestion(retry, "s1", 2, 4, turns), true);
  assert.equal(canRetryAssistantQuestion(retry, "s2", 2, 4, turns), false);
  assert.equal(canRetryAssistantQuestion(retry, "s1", 3, 4, turns), false);
  assert.equal(canRetryAssistantQuestion(retry, "s1", 2, 4, turns.slice(1)), false);
  assert.equal(canRetryAssistantQuestion(retry, "s1", 2, 4, turns.slice(0, 1)), false);
  assert.equal(canRetryAssistantQuestion(null, "s1", 2, 4, turns), false);
  assert.equal(canRetryAssistantQuestion({ ...retry, cancelled: true }, "s1", 2, 4, turns), false);
  assert.equal(canRetryAssistantQuestion(retry, "s1", 2, 4, [...turns, { id: "later", role: "user", content: "different question" }]), false);
  // Manual volume/browse/search flows add only assistant turns, not a new human question.
  assert.equal(canRetryAssistantQuestion(retry, "s1", 2, 5, [...turns, { id: "browse", role: "assistant", content: "different workspace" }]), false);
});
