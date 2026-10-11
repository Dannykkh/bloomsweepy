export const FILE_CHANGE_REVIEW_PAGE_SIZE = 20;

export type FileChangeReviewFilter = "review_needed" | "skipped" | "all";
export type FileChangeReviewState = "review_needed" | "skipped";
export type FileChangeReviewPhase = "off" | "baseline" | "checking" | "idle" | "incomplete" | "error" | "busy";

export interface FileChangeReviewConfig {
  enabled: boolean;
  root: string | null;
  intervalMinutes: 5 | 15 | 30;
}

export interface FileChangeReviewStatus {
  config: FileChangeReviewConfig;
  phase: FileChangeReviewPhase;
  baselineEntries: number;
  lastCheckedAtUnixMs: number | null;
  nextCheckAtUnixMs: number | null;
  historyCount: number;
  reviewNeededCount: number;
  skippedCount: number;
  historyTruncated: boolean;
  error: string | null;
  revision: number;
}

export interface FileChangeReviewEntry {
  id: string;
  path: string;
  name: string;
  previousPath?: string | null;
  kind: "created" | "modified" | "renamed" | "removed";
  reviewState: FileChangeReviewState;
  reasons: string[];
  isDirectory?: boolean;
  logicalBytes: number | null;
  modifiedAtUnixMs: number | null;
  decisionSource: "rule" | "manual";
}

export interface FileChangeReviewPage {
  status: FileChangeReviewStatus;
  entries: FileChangeReviewEntry[];
  offset: number;
  total: number;
}

export interface FileChangeReviewQuery {
  offset: number;
  filter: FileChangeReviewFilter;
}

export interface FileChangeReviewApi {
  getPage(query: FileChangeReviewQuery): Promise<FileChangeReviewPage>;
  setConfig(config: FileChangeReviewConfig): Promise<unknown>;
  checkNow(): Promise<unknown>;
  setDisposition(entryId: string, reviewState: FileChangeReviewState): Promise<unknown>;
  pickFolder(): Promise<string | null>;
}

function displayPath(path: string): string {
  if (path.startsWith("\\\\?\\UNC\\")) return `//${path.slice(8).replace(/\\/g, "/")}`;
  if (path.startsWith("\\\\?\\")) return path.slice(4).replace(/\\/g, "/");
  return /^[a-z]:[\\/]/i.test(path) || path.startsWith("\\\\") ? path.replace(/\\/g, "/") : path;
}

/** Presentation only: never makes an outside path appear to belong to the selected root. */
export function fileChangeReviewRelativePath(path: string, root: string | null): string {
  if (!root) return path;
  const displayed = displayPath(path);
  const scope = displayPath(root).replace(/\/+$/, "") || "/";
  if ([displayed, scope].some((value) => value.split("/").some((part) => part === "." || part === ".."))) return path;
  const windowsPath = /^[a-z]:\//i.test(scope) || scope.startsWith("//");
  const compare = (value: string) => windowsPath ? value.toLocaleLowerCase("en-US") : value;
  if (compare(displayed.replace(/\/+$/, "")) === compare(scope)) return ".";
  const boundary = scope === "/" ? "/" : `${scope}/`;
  return compare(displayed).startsWith(compare(boundary)) ? displayed.slice(boundary.length) : path;
}

const reasonMessages = {
  executable: "실행형 파일",
  double_extension: "이중 확장자",
  bulk_change: "한 번에 여러 항목 변경",
  metadata_changed: "크기 또는 수정 시각 변경",
  removed: "이전 검사 이후 사라짐",
  no_rule_match: "검토 규칙에 해당하지 않음",
  directory_changed: "폴더 메타데이터 변경",
} as const;

export function fileChangeReviewReason(reason: string): string {
  return reasonMessages[reason as keyof typeof reasonMessages] ?? "알 수 없는 검토 사유";
}

export function fileChangeReviewDisposition(entry: Pick<FileChangeReviewEntry, "reviewState" | "decisionSource">): string {
  if (entry.reviewState === "review_needed") return "검토 필요";
  return entry.decisionSource === "manual" ? "직접 넘김" : "규칙으로 넘김";
}

export function fileChangeReviewKind(kind: FileChangeReviewEntry["kind"]): string {
  return { created: "새로 발견", modified: "변경됨", renamed: "이름 변경", removed: "사라짐" }[kind];
}

export function fileChangeReviewPhase(phase: FileChangeReviewPhase): string {
  return {
    off: "꺼짐", baseline: "비교 기준 만드는 중", checking: "변경 확인 중", idle: "다음 확인 대기",
    incomplete: "검사 미완료", error: "검사 오류", busy: "다른 검사 완료 대기",
  }[phase];
}

export function fileChangeReviewPagination(offset: number, total: number, entries: number) {
  const count = Number.isFinite(total) ? Math.max(0, Math.floor(total)) : 0;
  const requestedOffset = Number.isFinite(offset) ? Math.max(0, Math.floor(offset)) : 0;
  const lastOffset = Math.max(0, Math.ceil(count / FILE_CHANGE_REVIEW_PAGE_SIZE) - 1) * FILE_CHANGE_REVIEW_PAGE_SIZE;
  const effectiveOffset = Math.min(Math.floor(requestedOffset / FILE_CHANGE_REVIEW_PAGE_SIZE) * FILE_CHANGE_REVIEW_PAGE_SIZE, lastOffset);
  const visible = Math.min(FILE_CHANGE_REVIEW_PAGE_SIZE, Math.max(0, entries), Math.max(0, count - effectiveOffset));
  return {
    offset: effectiveOffset,
    total: count,
    start: visible ? effectiveOffset + 1 : 0,
    end: visible ? effectiveOffset + visible : 0,
    previousOffset: effectiveOffset ? Math.max(0, effectiveOffset - FILE_CHANGE_REVIEW_PAGE_SIZE) : null,
    nextOffset: effectiveOffset < lastOffset ? effectiveOffset + FILE_CHANGE_REVIEW_PAGE_SIZE : null,
  };
}

/** Serializes page reads and drops responses superseded by a filter, scope, or action. */
export function createFileChangeReviewPageLoader(fetchPage: FileChangeReviewApi["getPage"]) {
  let generation = 0;
  let disposed = false;
  let tail: Promise<unknown> = Promise.resolve();
  return {
    load(query: FileChangeReviewQuery): Promise<FileChangeReviewPage | null> {
      const requestGeneration = ++generation;
      const request = { ...query };
      const work = tail.catch(() => undefined).then(async () => {
        if (disposed || requestGeneration !== generation) return null;
        const result = await fetchPage(request);
        return !disposed && requestGeneration === generation ? result : null;
      });
      tail = work;
      return work;
    },
    invalidate() { generation += 1; },
    dispose() { disposed = true; generation += 1; },
  };
}
