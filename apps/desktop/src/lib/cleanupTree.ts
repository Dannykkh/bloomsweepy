import type { CleanupTreeNode, CleanupTreeView } from "../types/cleanupTree.ts";

export const MAX_VISIBLE_CLEANUP_ROWS = 200;
export const MAX_CLEANUP_DEPTH = 32;

export interface CleanupDisplayRow {
  node: CleanupTreeNode;
  depth: number;
}

// Display only server-published branch pages. Never infer execution targets from paths.
export function visibleCleanupRows(
  tree: CleanupTreeView,
  expanded: ReadonlySet<string>,
  limit = MAX_VISIBLE_CLEANUP_ROWS,
): { rows: CleanupDisplayRow[]; limited: boolean } {
  const nodes = new Map(tree.nodes.map((node) => [node.id, node]));
  const branches = new Map(tree.branches.map((branch) => [branch.parentId, branch]));
  const pending: Array<{ id: string; parentId: string | null; depth: number }> = [];
  const addBranch = (parentId: string | null, depth: number) => {
    const ids = branches.get(parentId)?.childIds ?? [];
    for (let i = ids.length - 1; i >= 0; i--) pending.push({ id: ids[i], parentId, depth });
  };
  addBranch(null, 0);
  const seen = new Set<string>();
  const rows: CleanupDisplayRow[] = [];
  let limited = false;
  const boundedLimit = Number.isFinite(limit)
    ? Math.min(MAX_VISIBLE_CLEANUP_ROWS, Math.max(0, Math.floor(limit)))
    : MAX_VISIBLE_CLEANUP_ROWS;
  while (pending.length) {
    const item = pending.pop()!;
    const node = nodes.get(item.id);
    if (!node || seen.has(item.id) || node.parentId !== item.parentId) continue;
    if (rows.length >= boundedLimit) { limited = true; break; }
    seen.add(item.id);
    rows.push({ node, depth: item.depth });
    if (node.isDirectory && expanded.has(node.id)) {
      if (item.depth >= MAX_CLEANUP_DEPTH) limited = true;
      else addBranch(node.id, item.depth + 1);
    }
  }
  return { rows, limited };
}

export function knownCleanupRootIds(tree: CleanupTreeView): string[] {
  return tree.nodes.filter((node) => node.parentId === null && node.eligible).map((node) => node.id);
}

export function cleanupPlanIsLive(tree: CleanupTreeView, now: number): boolean {
  return Boolean(tree.plan
    && tree.plan.selectionRevision === tree.selectionRevision
    && tree.plan.expiresAtUnixMs > now
    && tree.expiresAtUnixMs > now);
}
