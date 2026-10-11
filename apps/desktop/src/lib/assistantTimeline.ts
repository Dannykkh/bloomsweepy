import type { AppToolResult, AssistantChatTurn } from "../types.ts";
import { formatAssistantPlainText } from "./assistantText.ts";

export const MAX_ASSISTANT_DISPLAY_TURNS = 160;
export const MAX_ASSISTANT_EVIDENCE_PAYLOADS = 16;
export const MAX_ASSISTANT_PENDING_RESULTS = 16;

export interface AssistantTimelineTurn extends AssistantChatTurn {
  id?: string;
  sequence?: number;
  providerLabel?: string;
  failed?: boolean;
}

export interface AssistantTurnEvidence {
  id: string;
  turnKey: string;
  result: AppToolResult;
  archived?: boolean;
}

export interface AssistantQuestionRetry {
  errorTurnKey: string;
  userTurnKey: string;
  sessionId: string;
  permissionRevision: number;
  workspaceRevision: number;
  message: string;
  history: AssistantChatTurn[];
  cancelled: boolean;
}

export function assistantTurnKey(turn: AssistantTimelineTurn): string {
  return turn.id ?? `saved-${turn.sequence}`;
}

export function boundedAssistantTurns(turns: AssistantTimelineTurn[], newKey: () => string): AssistantTimelineTurn[] {
  return turns.slice(-MAX_ASSISTANT_DISPLAY_TURNS).map(turn => turn.id || turn.sequence !== undefined
    ? turn : { ...turn, id: newKey() });
}

export function mergeAssistantToolResults(current: AppToolResult[], incoming: AppToolResult[]): AppToolResult[] {
  // A new answer is not a decision about an earlier unconfirmed plan.
  return [...current.filter(result => result.status === "review_required" && !incoming.includes(result)), ...incoming]
    .slice(-MAX_ASSISTANT_PENDING_RESULTS);
}

export function appendAssistantEvidence(current: AssistantTurnEvidence[], incoming: AssistantTurnEvidence[]): AssistantTurnEvidence[] {
  const entries = [...current, ...incoming];
  // Old cards keep their position and status, not unbounded native payloads or plan tokens.
  return entries.map((entry, index) => {
    if (entry.archived || index >= entries.length - MAX_ASSISTANT_EVIDENCE_PAYLOADS) return entry;
    const data: Record<string, unknown> = {};
    for (const key of ["displayName", "message", "reason", "returnedCount", "matchedCount"]) {
      const value = entry.result.data[key];
      if (typeof value === "string") data[key] = value.slice(0, 500);
      else if (typeof value === "number") data[key] = value;
    }
    return { ...entry, archived: true, result: { ...entry.result, data, presentation: null } };
  });
}

export function canRetryAssistantQuestion(retry: AssistantQuestionRetry | null, sessionId: string | undefined,
  permissionRevision: number, workspaceRevision: number, turns: AssistantTimelineTurn[]): boolean {
  const userTurns = turns.filter(turn => turn.role === "user");
  const lastUser = userTurns[userTurns.length - 1];
  return Boolean(retry && !retry.cancelled && retry.sessionId === sessionId && retry.permissionRevision === permissionRevision
    && retry.workspaceRevision === workspaceRevision
    && lastUser && assistantTurnKey(lastUser) === retry.userTurnKey
    && turns.some(turn => assistantTurnKey(turn) === retry.userTurnKey && turn.role === "user")
    && turns.some(turn => assistantTurnKey(turn) === retry.errorTurnKey && turn.failed));
}

export function boundedAssistantFailureDetail(detail: string): string {
  return formatAssistantPlainText(detail).slice(0, 2_000);
}
