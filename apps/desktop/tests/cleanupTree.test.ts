import assert from "node:assert/strict";
import test from "node:test";
import { cleanupPlanIsLive, knownCleanupRootIds, MAX_VISIBLE_CLEANUP_ROWS, visibleCleanupRows } from "../src/lib/cleanupTree.ts";
import type { CleanupTreeNode, CleanupTreeView } from "../src/types/cleanupTree.ts";

function entry(id: string, parentId: string | null, isDirectory = false): CleanupTreeNode {
  return { id, parentId, name: id, path: `/Synthetic/${id}`, isDirectory, logicalBytes: null,
    fileCount: null, directoryCount: null, selectionState: "unchecked", eligible: true, blockedReason: null, childrenLoaded: false };
}
function snapshot(): CleanupTreeView {
  return { treeId: "tree", selectionRevision: 1, source: "directory", rootName: "Demo", rootPath: "/Synthetic", capturedAtUnixMs: 0,
    expiresAtUnixMs: 10_000, nodes: [entry("folder", null, true), entry("keep", "folder"), entry("file", null)],
    branches: [{ parentId: null, offset: 0, limit: 50, totalEntries: 2, hasMore: false, truncated: false, unreadableEntries: 0, complete: true, childIds: ["folder", "file"] },
      { parentId: "folder", offset: 0, limit: 50, totalEntries: 1, hasMore: false, truncated: false, unreadableEntries: 0, complete: true, childIds: ["keep"] }],
    selection: { selectedNodeCount: 0, targetCount: 0, knownLogicalBytes: 0, unknownTargets: 0, partial: false }, plan: null };
}

test("only published pages render; opening a branch preserves server checkbox states", () => {
  const tree = snapshot();
  tree.nodes[0].selectionState = "mixed";
  tree.nodes[1].selectionState = "unchecked";
  tree.nodes[2].selectionState = "checked";
  assert.deepEqual(visibleCleanupRows(tree, new Set()).rows.map(({ node }) => node.id), ["folder", "file"]);
  const rows = visibleCleanupRows(tree, new Set(["folder"])).rows;
  assert.deepEqual(rows.map(({ node, depth }) => [node.id, depth, node.selectionState]), [["folder", 0, "mixed"], ["keep", 1, "unchecked"], ["file", 0, "checked"]]);
  assert.equal(tree.nodes[0].selectionState, "mixed", "display never promotes a partial folder into a whole-folder target");
});

test("whole selection only includes known eligible roots, including retained older pages", () => {
  const tree = snapshot();
  tree.nodes.push({ ...entry("protected", null), eligible: false }, entry("retained-page", null));
  assert.deepEqual(knownCleanupRootIds(tree), ["folder", "file", "retained-page"]);
  assert.equal(visibleCleanupRows(tree, new Set()).rows.some(({ node }) => node.id === "retained-page"), false);
});

test("bad branch IDs, duplicates and cross-parent entries cannot grow the visible tree", () => {
  const tree = snapshot();
  tree.branches[0].childIds.push("missing", "folder", "keep");
  tree.branches[1].childIds.push("folder", "keep");
  assert.deepEqual(visibleCleanupRows(tree, new Set(["folder"])).rows.map(({ node }) => node.id), ["folder", "keep", "file"]);
});

test("rendering uses a strict row budget instead of rendering all cached nodes", () => {
  const tree = snapshot();
  tree.nodes = Array.from({ length: 300 }, (_, index) => entry(`file-${index}`, null));
  tree.branches[0].childIds = tree.nodes.map((node) => node.id);
  const display = visibleCleanupRows(tree, new Set(), 10_000);
  assert.equal(display.rows.length, MAX_VISIBLE_CLEANUP_ROWS);
  assert.equal(display.limited, true);
});

test("a new branch page replaces display rows without clearing cached selection", () => {
  const tree = snapshot();
  tree.nodes.push({ ...entry("page-two", null), selectionState: "checked" });
  tree.branches[0].childIds = ["page-two"];
  tree.branches[0].offset = 50;
  assert.deepEqual(visibleCleanupRows(tree, new Set()).rows.map(({ node }) => node.id), ["page-two"]);
  assert.equal(tree.nodes[0].id, "folder");
});

test("confirmation is stale after selection changes or either expiry", () => {
  const tree = snapshot();
  tree.plan = { id: "plan", selectionRevision: 1, expiresAtUnixMs: 5_000, entries: [tree.nodes[2]], logicalBytes: 0, requiresNestedAck: false };
  assert.equal(cleanupPlanIsLive(tree, 4_999), true);
  assert.equal(cleanupPlanIsLive(tree, 5_000), false);
  tree.selectionRevision = 2;
  assert.equal(cleanupPlanIsLive(tree, 1_000), false);
  tree.selectionRevision = 1;
  tree.expiresAtUnixMs = 900;
  assert.equal(cleanupPlanIsLive(tree, 1_000), false);
});
