import type { AppToolResult, AssistantFileEntry, AssistantFileWorkspace, TrashOperationResult } from "../types.ts";

function record(value: unknown): Record<string, unknown> | null {
  return value !== null && typeof value === "object" && !Array.isArray(value) ? value as Record<string, unknown> : null;
}
function count(value: unknown): value is number { return typeof value === "number" && Number.isSafeInteger(value) && value >= 0; }
function string(value: unknown, max = 32768): value is string { return typeof value === "string" && value.length > 0 && value.length <= max && !value.includes("\0"); }
function nullableCount(value: unknown): boolean { return value === null || count(value); }
function nonce(value: unknown): value is string { return typeof value === "string" && /^[a-f0-9]{32}$/.test(value); }
function entry(value: unknown, revision: string, totalEntries: number): value is AssistantFileEntry {
  const row = record(value);
  return Boolean(row && count(row.number) && row.number > 0 && row.number <= totalEntries
    && row.id === `${revision}-${row.number}` && string(row.name, 1024) && string(row.path)
    && typeof row.isDirectory === "boolean" && nullableCount(row.logicalBytes)
    && nullableCount(row.fileCount) && nullableCount(row.directoryCount)
    && (row.linkCount === undefined || nullableCount(row.linkCount)) && nullableCount(row.modifiedAtUnixMs));
}

/** Local prepared native presentation only. Model-safe data cannot supply a plan. */
export function preparedExternalFileWorkspace(result: AppToolResult): AssistantFileWorkspace | null {
  if (result.source !== "broomsweepy" || result.capability !== "files.workspace" || result.status !== "review_required") return null;
  const presentation = record(result.presentation);
  const workspace = record(presentation?.workspace);
  const evidence = record(result.data.workspace);
  const plan = record(workspace?.plan);
  if (presentation?.view !== "overview" || presentation.reviewKind !== "files" || presentation.workspaceKey !== "external-files.workspace"
    || !workspace || !plan || !nonce(workspace.revision) || !nonce(plan.id)
    || evidence?.freshScan !== true || evidence.revision !== workspace.revision || evidence.reviewReady !== true || result.data.reviewPrepared !== true
    || !string(workspace.currentPath) || !string(workspace.currentName, 1024)
    || typeof workspace.canGoUp !== "boolean" || typeof workspace.sizeRanked !== "boolean"
    || (workspace.query !== null && typeof workspace.query !== "string") || !nullableCount(workspace.mapGeneration)
    || typeof workspace.truncated !== "boolean" || !count(workspace.unreadableEntries)
    || !count(workspace.totalEntries) || !count(workspace.offset) || !nullableCount(workspace.nextOffset) || !record(workspace.summary)
    || !Array.isArray(workspace.entries) || workspace.entries.length > 24 || !workspace.entries.every(row => entry(row, workspace.revision as string, workspace.totalEntries as number))
    || !Array.isArray(plan.entries) || plan.entries.length < 1 || plan.entries.length > 100
    || !plan.entries.every(row => entry(row, workspace.revision as string, workspace.totalEntries as number))
    || !count(plan.logicalBytes) || typeof plan.requiresNestedAck !== "boolean" || !nullableCount(plan.expiresAtUnixMs)
    || !Array.isArray(workspace.selectedIds) || workspace.selectedIds.length !== plan.entries.length) return null;
  const entries = plan.entries as AssistantFileEntry[];
  const ids = new Set(entries.map(row => row.id));
  if (ids.size !== entries.length || new Set(workspace.selectedIds).size !== ids.size
    || !workspace.selectedIds.every(id => typeof id === "string" && ids.has(id))
    || plan.requiresNestedAck !== entries.some(row => row.isDirectory)
    || entries.some(row => row.logicalBytes === null)) return null;
  return structuredClone(workspace) as unknown as AssistantFileWorkspace;
}

export interface ExternalFileReviewBridge {
  confirm: (revision: string, planId: string, nestedContentsAcknowledged: boolean) => Promise<TrashOperationResult>;
  cancel: (revision: string, planId: string) => Promise<void>;
}

/** The backend is the authority; this gate only prevents a double UI dispatch. */
export function externalFileReviewDecision(workspace: AssistantFileWorkspace, bridge: ExternalFileReviewBridge) {
  const revision = workspace.revision;
  const planId = workspace.plan!.id;
  let running = false;
  let confirmationAttempted = false;
  let closed = false;
  return {
    get running() { return running; },
    get confirmationAttempted() { return confirmationAttempted; },
    async confirm(nestedContentsAcknowledged: boolean): Promise<TrashOperationResult | null> {
      if (running || confirmationAttempted || closed) return null;
      running = true; confirmationAttempted = true;
      try { return await bridge.confirm(revision, planId, nestedContentsAcknowledged); }
      finally { running = false; }
    },
    async cancel(): Promise<boolean> {
      if (running || closed) return false;
      running = true;
      try { await bridge.cancel(revision, planId); closed = true; return true; }
      finally { running = false; }
    },
  };
}
