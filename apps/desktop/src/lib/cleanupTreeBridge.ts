import { invoke } from "@tauri-apps/api/core";
import type { TrashOperationResult } from "../types";
import type { CleanupTreeSource, CleanupTreeView } from "../types/cleanupTree";

// Native IDs/revisions, never client paths, carry selection and review authority.
export function openCleanupTree(source: CleanupTreeSource): Promise<CleanupTreeView> {
  return invoke("open_cleanup_tree", { request: { source } });
}
export function getCleanupTree(treeId: string): Promise<CleanupTreeView> {
  return invoke("get_cleanup_tree", { treeId });
}
export function loadCleanupTreeChildren(treeId: string, parentId: string | null, offset: number): Promise<CleanupTreeView> {
  return invoke("load_cleanup_tree_children", { request: { treeId, parentId, offset } });
}
export function updateCleanupTreeSelection(treeId: string, selectionRevision: number, includeIds: string[], excludeIds: string[]): Promise<CleanupTreeView> {
  return invoke("update_cleanup_tree_selection", { request: { treeId, selectionRevision, includeIds, excludeIds } });
}
export function prepareCleanupTreePlan(treeId: string, selectionRevision: number): Promise<CleanupTreeView> {
  return invoke("prepare_cleanup_tree_plan", { request: { treeId, selectionRevision } });
}
export function confirmCleanupTreePlan(treeId: string, selectionRevision: number, planId: string, nestedContentsAcknowledged: boolean): Promise<TrashOperationResult> {
  return invoke("confirm_cleanup_tree_plan", { request: { treeId, selectionRevision, planId, nestedContentsAcknowledged } });
}
export function dismissCleanupTreePlan(treeId: string, planId: string): Promise<CleanupTreeView> {
  return invoke("dismiss_cleanup_tree_plan", { treeId, planId });
}
