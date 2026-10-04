import type { TrashOperationResult, TrashProgress } from "../types";

export type CleanupTreeSource =
  | { kind: "directory"; generation: number }
  | { kind: "assistant"; sessionId: string; revision: string };

export interface CleanupTreeNode {
  id: string;
  parentId: string | null;
  name: string;
  path: string;
  isDirectory: boolean;
  logicalBytes: number | null;
  fileCount: number | null;
  directoryCount: number | null;
  selectionState: "unchecked" | "checked" | "mixed";
  eligible: boolean;
  blockedReason: string | null;
  childrenLoaded: boolean;
}

export interface CleanupTreeBranch {
  parentId: string | null;
  offset: number;
  limit: number;
  totalEntries: number;
  hasMore: boolean;
  truncated: boolean;
  unreadableEntries: number;
  complete: boolean;
  childIds: string[];
}

export interface CleanupTreeSelection {
  selectedNodeCount: number;
  targetCount: number;
  knownLogicalBytes: number;
  unknownTargets: number;
  partial: boolean;
}

export interface CleanupTreePlan {
  id: string;
  selectionRevision: number;
  expiresAtUnixMs: number;
  entries: CleanupTreeNode[];
  logicalBytes: number;
  requiresNestedAck: boolean;
}

export interface CleanupTreeView {
  treeId: string;
  selectionRevision: number;
  source: "directory" | "assistant";
  rootName: string;
  rootPath: string;
  capturedAtUnixMs: number;
  expiresAtUnixMs: number;
  nodes: CleanupTreeNode[];
  branches: CleanupTreeBranch[];
  selection: CleanupTreeSelection;
  plan: CleanupTreePlan | null;
}

export interface CleanupTreePanelProps {
  tree: CleanupTreeView | null;
  busy: boolean;
  error: string | null;
  result?: TrashOperationResult | null;
  progress?: TrashProgress | null;
  onLoadChildren: (parentId: string | null, offset: number) => Promise<unknown> | void;
  onSelectionChange: (includeIds: string[], excludeIds: string[]) => Promise<unknown> | void;
  onPrepare: () => Promise<unknown> | void;
  onConfirm: (nestedContentsAcknowledged: boolean) => Promise<unknown> | void;
  onDismissPlan: () => Promise<unknown> | void;
  onRefresh: () => Promise<unknown> | void;
  onCancel?: () => void;
  onOpen?: (path: string) => Promise<unknown> | void;
  onReveal?: (path: string) => Promise<unknown> | void;
}
