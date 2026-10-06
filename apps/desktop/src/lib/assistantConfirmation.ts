import type { AppToolResult, AssistantFileWorkspace } from "../types.ts";
import type { ApplicationInventory, ApplicationTrashPlan } from "./applicationTypes.ts";

/** Only call for a fresh human composer event, never a provider/tool message. */
export function humanTrashDecision(message: string): boolean | null {
  const reply = message.trim().toLocaleLowerCase().replace(/[.!。！]+$/u, "").trim();
  if (["예", "네", "응", "그래", "예 삭제", "삭제하자", "지워", "휴지통으로 이동", "yes", "yes delete", "delete", "はい", "是", "确认"].includes(reply)) return true;
  if (["아니오", "아니요", "아니", "취소", "삭제하지 마", "no", "cancel", "いいえ", "否", "取消"].includes(reply)) return false;
  return null;
}

export function applicationTrashQuestion(result: AppToolResult) {
  if (result.status !== "review_required") return null;
  const presentation = result.presentation;
  const kind = presentation?.reviewKind;
  if (kind !== "applicationBundle" && kind !== "applicationData") return null;
  const plan = presentation?.plan as ApplicationTrashPlan | undefined;
  const inventory = presentation?.inventory as ApplicationInventory | undefined;
  const application = inventory?.applications?.find(item => item.id === presentation?.applicationId);
  if (!application || !plan?.planId || inventory?.inventoryId !== presentation?.inventoryId
    || (kind === "applicationBundle" && plan.path !== application.installLocation)
    || (kind === "applicationData" && !plan.relatedData?.length)) return null;
  return { plan, application, bundle: kind === "applicationBundle" };
}

export function solePendingTrash<T>(pending: readonly T[]): T | null {
  return pending.length === 1 ? pending[0] : null;
}

/** Native workspace evidence describes the inline plan, not a second approval. */
export function workspaceReviewMatches(result: AppToolResult, capability: "files.workspace" | "empty.workspace", revision: string): boolean {
  const workspace = result.data.workspace;
  return Boolean(revision) && result.source === "broomsweepy" && result.capability === capability
    && result.status === "review_required" && result.data.reviewPrepared === true
    && workspace !== null && typeof workspace === "object" && !Array.isArray(workspace)
    && (workspace as Record<string, unknown>).freshScan === true
    && (workspace as Record<string, unknown>).revision === revision;
}

export function canAutomaticallyTrashFiles(message: string, workspace: AssistantFileWorkspace | null, results: readonly AppToolResult[]): boolean {
  if (!workspace?.plan || workspace.truncated || workspace.unreadableEntries
    || !namedTrashRequest(message, workspace.plan.entries.map(entry => entry.name))) return false;
  const reviews = results.filter(result => result.status === "review_required");
  const ownReview = solePendingTrash(reviews);
  return Boolean(ownReview && !ownReview.truncated && workspaceReviewMatches(ownReview, "files.workspace", workspace.revision)
    && results.every(result => result === ownReview || result.status === "completed"));
}

/** Explicit named removal only. Permission is checked again by native execution. */
export function namedTrashRequest(message: string, targetNames: readonly string[]): boolean {
  // minimal: one 2,000-character command and at most 100 literal names — expand only after a concrete unsupported direct-command case.
  const forbiddenLiteral = /[\u0000-\u001f\u007f\u0085\u2028\u2029\ue000\ue001]/u;
  if (message.length > 2_000 || forbiddenLiteral.test(message) || !targetNames.length || targetNames.length > 100) return false;
  const text = message.trim().normalize("NFC").toLocaleLowerCase();
  const names = targetNames.map(name => name.normalize("NFC").toLocaleLowerCase());
  if (names.some(name => !name.length || name.length > 255 || name.trim() !== name
    || /[/\\]/u.test(name) || forbiddenLiteral.test(name)) || new Set(names).size !== names.length) return false;

  // Replace trusted native names before checking command syntax: a filename may
  // itself contain "if", a condition, punctuation, or a removal verb.
  const ids = new Map(names.map((name, index) => [name, index]));
  const occurrences = names.map(() => 0);
  const alternatives = [...names].sort((a, b) => b.length - a.length)
    .map(name => name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")).join("|");
  const literals = new RegExp(`(?<![a-z0-9_.-])(?:${alternatives})(?![a-z0-9_-]|\\.[a-z0-9_.-])`, "gu");
  const masked = text.replace(literals, name => {
    const index = ids.get(name)!;
    occurrences[index]++;
    return `\ue000${index}\ue001`;
  });
  if (occurrences.some(count => count !== 1)) return false;

  // Only one complete direct command is accepted. Unknown text, conditions,
  // quotations, advice and additional targets/actions all fall back to review.
  const target = "\\ue000[0-9]+\\ue001";
  const targets = `${target}(?:\\s*(?:[,，、]|and|와|과|및|と|和)\\s*${target})*`;
  const end = "\\s*[.!。！]*";
  const commands = [
    `${targets}(?:[을를])?\\s+(?:(?:앱|파일|폴더)(?:[을를])?\\s+)?(?:삭제(?:\\s*(?:하자|해(?:\\s*(?:줘|주세요))?))?|지워(?:\\s*(?:줘|주세요))?|휴지통으로\\s*(?:이동|옮겨)(?:\\s*(?:줘|주세요))?)`,
    `(?:please\\s+)?(?:delete|remove|trash)\\s+${targets}(?:\\s+please)?`,
    `${targets}(?:\\s*を)?\\s*(?:削除してください|削除して下さい|削除して|削除しよう|削除)`,
    `(?:请\\s*)?(?:删除|移到回收站)\\s*${targets}`,
    `(?:请\\s*)?把\\s*${targets}\\s*(?:删除|移到回收站)`,
    `${targets}\\s*(?:删除|移到回收站)`,
  ];
  return commands.some(command => new RegExp(`^(?:${command})${end}$`, "u").test(masked));
}
