import assert from "node:assert/strict";
import test from "node:test";
import { externalFileReviewDecision, preparedExternalFileWorkspace } from "../src/lib/externalFileReview.ts";
import type { AppToolResult, AssistantFileWorkspace, TrashOperationResult } from "../src/types.ts";

const revision = "a".repeat(32);
const planId = "b".repeat(32);
const row = { id: `${revision}-1`, number: 1, name: "Synthetic folder", path: "/Demo/Synthetic folder", isDirectory: true,
  logicalBytes: 8, fileCount: 1, directoryCount: 0, linkCount: 0, modifiedAtUnixMs: 1 };
const workspace: AssistantFileWorkspace = {
  revision, currentPath: "/Demo", currentName: "Demo", canGoUp: false, query: null, sizeRanked: false, mapGeneration: null,
  summary: { scopeName: "Demo", completedAtUnixMs: 1, totalLogicalBytes: 8, totalFiles: 1, totalDirectories: 1,
    unreadableEntries: 0, emptyDirectoryCount: 0, childrenTruncated: false, children: [] },
  totalEntries: 1, truncated: false, unreadableEntries: 0, offset: 0, nextOffset: null, entries: [row], selectedIds: [row.id],
  plan: { id: planId, entries: [row], logicalBytes: 8, requiresNestedAck: true, expiresAtUnixMs: null },
};
const result: AppToolResult = {
  source: "broomsweepy", capability: "files.workspace", status: "review_required", capturedAtUnixMs: 1, truncated: false,
  data: { workspace: { freshScan: true, revision, reviewReady: true }, reviewPrepared: true, deleted: false },
  presentation: { view: "overview", reviewKind: "files", workspaceKey: "external-files.workspace", workspace },
};
const actual: TrashOperationResult = { operationId: "mock", requestedCount: 1, movedCount: 1, movedBytes: 8,
  cancelled: false, stoppedEarly: false, journalComplete: true, journalPath: "/Demo/mock-journal", items: [] };

test("only the exact prepared native external review is consumed without prepare or inspection", () => {
  const prepared = preparedExternalFileWorkspace(result);
  assert.deepEqual(prepared, workspace);
  assert.notEqual(prepared, workspace);
  for (const invalid of [
    { ...result, source: "other" }, { ...result, capability: "files.search" }, { ...result, status: "completed" },
    { ...result, presentation: { ...result.presentation, workspaceKey: "native-session" } },
    { ...result, presentation: { ...result.presentation, reviewKind: "memory" } },
    { ...result, presentation: { ...result.presentation, view: "files" } },
    { ...result, data: { ...result.data, reviewPrepared: false } },
    { ...result, data: { workspace: { freshScan: true, revision: "c".repeat(32), reviewReady: true }, reviewPrepared: true } },
    { ...result, data: { workspace: { freshScan: true, revision, reviewReady: false }, reviewPrepared: true } },
    { ...result, presentation: { ...result.presentation, workspace: { ...workspace, plan: null } } },
  ]) assert.equal(preparedExternalFileWorkspace(invalid as AppToolResult), null);
});

test("prepared entries must be bounded, unique and belong to the current revision and exact selection", () => {
  for (const change of [
    { revision: "unknown" }, { selectedIds: [] }, { selectedIds: ["c".repeat(32) + "-1"] },
    { plan: { ...workspace.plan!, id: "some-plan" } },
    { plan: { ...workspace.plan!, entries: [] } },
    { plan: { ...workspace.plan!, requiresNestedAck: false } },
    { plan: { ...workspace.plan!, entries: [{ ...row, id: "c".repeat(32) + "-1" }] } },
    { plan: { ...workspace.plan!, entries: [{ ...row, path: "" }] } },
    { plan: { ...workspace.plan!, entries: [{ ...row, logicalBytes: null }] } },
    { plan: { ...workspace.plan!, entries: [row, row] }, selectedIds: [row.id, row.id] },
    { plan: { ...workspace.plan!, entries: Array.from({ length: 101 }, () => row) } },
  ]) assert.equal(preparedExternalFileWorkspace({ ...result, presentation: { ...result.presentation, workspace: { ...workspace, ...change } } }), null);
});

test("mock bridge confirms once even for a double click, with immutable nonce identity and no prepare", async () => {
  const requests: unknown[] = [];
  let finish!: (value: TrashOperationResult) => void;
  const prepared = preparedExternalFileWorkspace(result)!;
  const bridge = {
    confirm: async (...args: unknown[]) => { requests.push({ command: "confirm", args }); return new Promise<TrashOperationResult>(resolve => { finish = resolve; }); },
    cancel: async (...args: unknown[]) => { requests.push({ command: "cancel", args }); },
    prepare: async () => { throw new Error("Do not prepare again"); },
  };
  const decision = externalFileReviewDecision(prepared, bridge);
  prepared.revision = "c".repeat(32); prepared.plan!.id = "d".repeat(32);
  const first = decision.confirm(true);
  assert.equal(decision.running, true);
  assert.equal(await decision.confirm(true), null);
  assert.equal(await decision.cancel(), false);
  assert.deepEqual(requests, [{ command: "confirm", args: [revision, planId, true] }]);
  finish(actual);
  assert.deepEqual(await first, actual);
  assert.equal(decision.running, false);
  assert.equal(await decision.confirm(true), null);
  assert.equal(await decision.cancel(), true);
  assert.deepEqual(requests[1], { command: "cancel", args: [revision, planId] });
});

test("no and Escape use the same exact cancellation, await completion and never confirm", async () => {
  const calls: string[] = [];
  let finish!: () => void;
  const decision = externalFileReviewDecision(workspace, {
    confirm: async () => { calls.push("confirm"); return actual; },
    cancel: async () => { calls.push("cancel"); return new Promise<void>(resolve => { finish = resolve; }); },
  });
  const cancelling = decision.cancel();
  assert.equal(decision.running, true);
  assert.equal(await decision.cancel(), false);
  assert.equal(await decision.confirm(true), null);
  assert.deepEqual(calls, ["cancel"]);
  finish(); assert.equal(await cancelling, true);
  assert.equal(await decision.confirm(true), null);
  assert.equal(decision.confirmationAttempted, false);
});

test("confirmation failure cannot replay the nonce, while cancellation failure remains retryable", async () => {
  let confirms = 0; let cancels = 0;
  const decision = externalFileReviewDecision(workspace, {
    confirm: async () => { confirms++; throw new Error("Native target changed"); },
    cancel: async () => { cancels++; if (cancels === 1) throw new Error("Native cancellation failed"); },
  });
  await assert.rejects(decision.confirm(true), /Native target changed/);
  assert.equal(await decision.confirm(true), null);
  assert.equal(confirms, 1);
  await assert.rejects(decision.cancel(), /Native cancellation failed/);
  assert.equal(decision.running, false);
  assert.equal(await decision.cancel(), true);
  assert.equal(cancels, 2);
});
