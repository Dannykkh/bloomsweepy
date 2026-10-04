import assert from "node:assert/strict";
import test from "node:test";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import { openCleanupTree, getCleanupTree, loadCleanupTreeChildren, updateCleanupTreeSelection,
  prepareCleanupTreePlan, confirmCleanupTreePlan, dismissCleanupTreePlan } from "../src/lib/cleanupTreeBridge.ts";

test("the production cleanup bridge matches native commands and sends IDs instead of paths", async () => {
  const requests: unknown[] = [];
  const originalWindow = Object.getOwnPropertyDescriptor(globalThis, "window");
  Object.defineProperty(globalThis, "window", { configurable: true, value: {} });
  mockIPC((command, payload) => { requests.push({ command, payload }); return null; });
  try {
    await openCleanupTree({ kind: "directory", generation: 7 });
    await openCleanupTree({ kind: "assistant", sessionId: "session", revision: "snapshot" });
    await getCleanupTree("tree");
    await loadCleanupTreeChildren("tree", null, 50);
    await loadCleanupTreeChildren("tree", "parent", 0);
    await updateCleanupTreeSelection("tree", 2, ["parent"], ["keep"]);
    await prepareCleanupTreePlan("tree", 3);
    await dismissCleanupTreePlan("tree", "plan");
    await confirmCleanupTreePlan("tree", 3, "plan", true);
    assert.deepEqual(requests, [
      { command: "open_cleanup_tree", payload: { request: { source: { kind: "directory", generation: 7 } } } },
      { command: "open_cleanup_tree", payload: { request: { source: { kind: "assistant", sessionId: "session", revision: "snapshot" } } } },
      { command: "get_cleanup_tree", payload: { treeId: "tree" } },
      { command: "load_cleanup_tree_children", payload: { request: { treeId: "tree", parentId: null, offset: 50 } } },
      { command: "load_cleanup_tree_children", payload: { request: { treeId: "tree", parentId: "parent", offset: 0 } } },
      { command: "update_cleanup_tree_selection", payload: { request: { treeId: "tree", selectionRevision: 2, includeIds: ["parent"], excludeIds: ["keep"] } } },
      { command: "prepare_cleanup_tree_plan", payload: { request: { treeId: "tree", selectionRevision: 3 } } },
      { command: "dismiss_cleanup_tree_plan", payload: { treeId: "tree", planId: "plan" } },
      { command: "confirm_cleanup_tree_plan", payload: { request: { treeId: "tree", selectionRevision: 3, planId: "plan", nestedContentsAcknowledged: true } } },
    ]);
  } finally {
    clearMocks();
    if (originalWindow) Object.defineProperty(globalThis, "window", originalWindow);
    else Reflect.deleteProperty(globalThis, "window");
  }
});

test("a rejected confirmation is propagated without an automatic retry", async () => {
  const originalWindow = Object.getOwnPropertyDescriptor(globalThis, "window");
  Object.defineProperty(globalThis, "window", { configurable: true, value: {} });
  let calls = 0;
  mockIPC(() => { calls += 1; return Promise.reject(new Error("Expired review")); });
  try {
    await assert.rejects(confirmCleanupTreePlan("tree", 4, "plan", false), /Expired review/);
    assert.equal(calls, 1);
  } finally {
    clearMocks();
    if (originalWindow) Object.defineProperty(globalThis, "window", originalWindow);
    else Reflect.deleteProperty(globalThis, "window");
  }
});
