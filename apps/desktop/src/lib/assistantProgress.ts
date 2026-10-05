import type { AssistantProgress } from "../types.ts";

export function matchesAssistantProgress(value: unknown, progressId: string | null, sessionId: string): value is AssistantProgress {
  if (!value || typeof value !== "object" || !progressId) return false;
  const progress = value as Record<string, unknown>;
  return progress.progressId === progressId && progress.sessionId === sessionId
    && typeof progress.phase === "string" && ["preparing", "analyzing", "querying"].includes(progress.phase)
    && typeof progress.round === "number" && Number.isInteger(progress.round) && progress.round >= 0 && progress.round <= 4
    && (progress.capability === null || typeof progress.capability === "string" && progress.capability.length <= 64);
}

export function elapsedLabel(start: number, now: number): string {
  const seconds = Math.max(0, Math.floor((now - start) / 1000));
  return seconds < 60 ? `${seconds}s` : `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
}
