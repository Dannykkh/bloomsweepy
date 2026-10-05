import { ExternalLink, ShieldCheck } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { useLanguage, type MessageKey } from "../i18n";
import { cleanAppMemory, executeGracefulProcessTermination } from "../lib/bridge";
import { formatBytes, formatCount, formatDate } from "../lib/format";
import type { ApplicationInventory, ApplicationTrashPlan } from "../lib/applicationTypes";
import type { AppToolResult, DockerCleanupPreview, DockerManagementStatus, TerminationPreview, TrashOperationResult, ViewId } from "../types";
import { ApplicationReview, type ReviewTarget } from "../views/ApplicationsView";
import { DockerCleanupDialog } from "./DockerCleanupDialog";
import { ProcessTerminationDialog } from "./ProcessTerminationDialog";
import "./AssistantAppToolCard.css";

export const appToolTitles: Record<string, MessageKey> = {
  "files.workspace": "파일 검사", "empty.workspace": "빈 폴더",
  "storage.overview": "드라이브 사용량", "performance.inspect": "시스템 성능",
  "applications.list": "설치된 앱", "applications.inspect": "관련 데이터 검토",
  "applications.review": "앱 본체 정리 검토", "processes.review": "정상 종료 요청",
  "memory.review": "앱 메모리 정리", "files.search": "파일 찾기", "documents.search": "문서 검색",
  "index.status": "검색 목록", "index.build": "검색 준비", "storage.scan": "파일 검사",
  "cleanup.scan": "정리 후보 검사", "cleanup.candidates": "정리 후보", "cleanup.review": "정리 후보",
  "cleanup.plan_status": "정리 후보", "operations.status": "요청한 작업", "operations.cancel": "요청한 작업",
  "docker.status": "Docker 용량", "docker.review": "Docker 정리 검토", "ui.view": "앱 화면",
};
export const appToolStatusKeys: Record<AppToolResult["status"], MessageKey> = {
  completed: "완료", running: "진행 중", review_required: "최종 확인 대기",
  permission_required: "허용 안 됨", unsupported: "지원하지 않음", failed: "완료하지 못함",
};

function object(value: unknown): Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value) ? value as Record<string, unknown> : {};
}
function text(value: unknown): string { return typeof value === "string" ? value : ""; }
function number(value: unknown): number | null { return typeof value === "number" && Number.isFinite(value) ? value : null; }
function rows(value: unknown): Record<string, unknown>[] { return Array.isArray(value) ? value.slice(0, 24).map(object) : []; }

export function appToolView(result: AppToolResult): ViewId | null {
  const raw = text(result.presentation?.view);
  const aliases: Record<string, ViewId> = { large_files: "large-files", system_cleanup: "cleanup", fast_search: "files", document_search: "documents" };
  const views = ["dashboard", "performance", "applications", "overview", "large-files", "duplicates", "cleanup", "files", "documents", "docker", "settings"];
  if (aliases[raw]) return aliases[raw];
  if (views.includes(raw)) return raw as ViewId;
  if (result.capability === "files.search" || result.capability === "index.status" && result.data.source === "files") return "files";
  if (result.capability === "documents.search" || result.capability === "index.status" && result.data.source === "documents") return "documents";
  if (result.capability.startsWith("cleanup.")) return "cleanup";
  if (result.capability === "storage.scan") return "large-files";
  return null;
}

export function AssistantAppToolCard({ result, busy = false, onView, onReview }: {
  result: AppToolResult; busy?: boolean;
  onView?: (result: AppToolResult) => void;
  onReview?: (result: AppToolResult) => void;
}) {
  const { t } = useLanguage();
  const data = object(result.data);
  const memory = object(data.memory);
  const items = rows(data.items ?? data.results ?? data.relatedData ?? data.categories ?? data.volumes);
  const application = object(data.application);
  const index = object(data.index);
  const incomplete = result.truncated || data.truncated === true || data.sourceMayBeIncomplete === true;
  const view = appToolView(result);
  const measuredAt = number(data.capturedAtUnixMs) ?? result.capturedAtUnixMs;
  return <section className="assistant-app-tool-card" aria-label={t("앱에서 확인한 결과")}>
    <header><div><p className="eyebrow">BroomSweepy</p><h3>{t(appToolTitles[result.capability] ?? "앱에서 확인한 결과")}</h3></div><span className="assistant-app-tool-state">{t(appToolStatusKeys[result.status])}</span></header>
    <p className="assistant-app-tool-meta">{t("실제 앱 조회 · {{date}}", { date: formatDate(measuredAt) })}</p>
    {result.capability === "performance.inspect" ? <dl className="assistant-app-tool-metrics">
      <div><dt>CPU</dt><dd>{number(data.cpuUsagePercent)?.toFixed(1) ?? "—"}%</dd></div>
      <div><dt>{t("메모리")}</dt><dd>{number(memory.usedBytes) === null ? "—" : formatBytes(number(memory.usedBytes)!)} / {number(memory.totalBytes) === null ? "—" : formatBytes(number(memory.totalBytes)!)}</dd></div>
      <div><dt>{t("사용 가능")}</dt><dd>{number(memory.availableBytes) === null ? "—" : formatBytes(number(memory.availableBytes)!)}</dd></div>
    </dl> : null}
    {typeof data.query === "string" && data.query ? <p>{t("검색어")}: {data.query}</p> : null}
    {text(application.displayName) ? <p><strong>{text(application.displayName)}</strong> · {number(application.estimatedBytes) === null ? t("용량 미측정") : formatBytes(number(application.estimatedBytes)!)}</p> : null}
    {text(data.displayName) ? <p><strong>{text(data.displayName)}</strong></p> : null}
    {typeof index.available === "boolean" && !index.available || data.available === false ? <p>{t("검색 목록을 먼저 만들어 주세요.")}</p> : null}
    {number(index.completedAtUnixMs) !== null ? <p>{t("색인 기준 · {{date}}", { date: formatDate(number(index.completedAtUnixMs)!) })}</p> : null}
    {items.length ? <ol className="assistant-app-tool-list">{items.map((item, i) => {
      const size = number(item.residentBytes ?? item.estimatedBytes ?? item.logicalBytes ?? item.sizeBytes ?? item.totalBytes);
      const cpu = number(item.cpuMachinePercent);
      return <li key={`${text(item.id ?? item.targetId)}-${i}`}>
        <div><strong>{text(item.displayName ?? item.name ?? item.label ?? item.kind) || t("요청한 작업")}</strong>
          {text(item.displayVersion) ? <span>{t("버전 {{version}}", { version: text(item.displayVersion) })}</span> : null}
          {text(item.snippet) ? <p>{text(item.snippet)}</p> : null}
          {text(item.protectionReason) ? <span>{text(item.protectionReason)}</span> : null}
        </div><div className="assistant-app-tool-values">
          {cpu !== null ? <span>CPU {cpu.toFixed(1)}%</span> : null}
          <span>{size === null ? t("용량 미측정") : formatBytes(size)}</span>
          {number(item.modifiedAtUnixMs) !== null ? <span>{formatDate(number(item.modifiedAtUnixMs)!)}</span> : null}
        </div>
      </li>;
    })}</ol> : null}
    {number(data.returnedCount ?? data.matchedCount) !== null ? <p>{t("{{count}}개", { count: formatCount(number(data.returnedCount ?? data.matchedCount)!) })}</p> : null}
    {typeof data.message === "string" ? <p>{data.message}</p> : null}
    {typeof data.reason === "string" ? <p>{data.reason}</p> : null}
    {typeof data.state === "string" ? <p>{data.state}</p> : null}
    {incomplete ? <p className="assistant-app-tool-warning">{t("부분 결과입니다. 생략된 항목이나 읽지 못한 자료가 있을 수 있습니다.")}</p> : null}
    {data.contentExcerptsShared === true ? <p>{t("허용된 문서 이름과 일치 본문 일부를 AI에 전달했습니다.")}</p> : null}
    {result.status === "review_required" ? <p><ShieldCheck size={16} aria-hidden="true" /> {t("검토만 준비됐습니다. 실제 실행은 앱에서 최종 확인해야 합니다.")}</p> : null}
    <footer>
      {result.status === "review_required" && result.presentation && onReview ? <button type="button" className="secondary-button" disabled={busy} onClick={() => onReview(result)}>{t("검토 열기")}</button> : null}
      {view && onView ? <button type="button" className="text-button" disabled={busy} onClick={() => onView(result)}><ExternalLink size={16} aria-hidden="true" />{t("이 결과 화면 보기")}</button> : null}
    </footer>
  </section>;
}

export interface AppToolReviewCompletion { message: string; trashResult?: TrashOperationResult; dockerStatus?: DockerManagementStatus; mutated?: boolean }

/// Consume only the prepared app presentation; never call a prepare endpoint again.
export function AssistantAppToolReview({ result, busy = false, onBusyChange, onClose, onCompleted }: {
  result: AppToolResult; busy?: boolean;
  onBusyChange?: (busy: boolean) => void;
  onClose: () => void;
  onCompleted: (completion: AppToolReviewCompletion) => void;
}) {
  const { t } = useLanguage();
  const presentation = object(result.presentation);
  const reviewKind = text(presentation.reviewKind);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const consumed = useRef(false);
  const memoryDialog = useRef<HTMLDialogElement>(null);
  const cancelRef = useRef<HTMLButtonElement>(null);
  const preparedApplication = useMemo(() => {
    if (reviewKind !== "applicationBundle" && reviewKind !== "applicationData") return null;
    const inventory = presentation.inventory as ApplicationInventory | undefined;
    const plan = presentation.plan as ApplicationTrashPlan | undefined;
    const application = inventory?.applications?.find((app) => app.id === presentation.applicationId);
    if (!application || !plan?.planId) return null;
    const target: ReviewTarget = { application, inventoryId: text(presentation.inventoryId), kind: reviewKind === "applicationBundle" ? "bundle" : "data", candidateIds: Array.isArray(presentation.candidateIds) ? presentation.candidateIds.filter((id): id is string => typeof id === "string") : [] };
    return { target, plan };
  }, [result]);
  const dockerPreview = useMemo(() => {
    if (reviewKind !== "docker") return null;
    const preview = presentation.preview as DockerCleanupPreview | undefined;
    const actions = Array.isArray(presentation.actions) ? presentation.actions : [];
    const categories: Record<string, string> = { build_cache: "buildCache", dangling_images: "danglingImages", stopped_containers: "stoppedContainers" };
    const allowed = new Set(actions.map((action) => categories[String(action)]));
    return preview?.previewId ? { ...preview, items: preview.items.filter((item) => allowed.has(item.kind)).map((item) => ({ ...item, defaultSelected: true })) } : null;
  }, [result]);

  useEffect(() => {
    if (reviewKind !== "memory") return;
    const dialog = memoryDialog.current;
    dialog?.showModal(); cancelRef.current?.focus();
    return () => dialog?.close();
  }, [reviewKind]);

  async function confirmProcess() {
    const preview = presentation.preview as TerminationPreview | undefined;
    if (!preview?.previewId || running || busy || consumed.current) return;
    consumed.current = true; setRunning(true); onBusyChange?.(true);
    try {
      const actual = await executeGracefulProcessTermination({ previewId: preview.previewId, unsavedWorkAcknowledged: true });
      const name = actual.displayName ?? preview.displayName;
      const message = actual.outcome === "terminated" ? t("{{name}}이 종료됐습니다.", { name })
        : actual.outcome === "requestSent" ? t("{{name}}에 정상 종료 요청을 보냈습니다. 저장 확인 창이 열려 있을 수 있습니다.", { name })
        : actual.outcome === "alreadyExited" ? t("앱이 이미 종료됐습니다.")
        : t("종료 요청 결과 · {{outcome}}", { outcome: actual.outcome });
      setNotice(message); onCompleted({ message });
    } catch (reason) {
      const message = t("성능 작업을 완료하지 못했습니다. {{detail}}", { detail: String(reason) });
      setError(message); onCompleted({ message });
    } finally { setRunning(false); onBusyChange?.(false); }
  }
  async function confirmMemory() {
    if (running || busy || consumed.current) return;
    consumed.current = true; setRunning(true); onBusyChange?.(true);
    try {
      const actual = await cleanAppMemory();
      const message = actual.outcome !== "completed" ? t("메모리 정리를 완료하지 못했습니다. {{detail}}", { detail: actual.outcome })
        : actual.allocatorReleasedBytes > 0 ? t("BroomSweepy가 {{amount}}의 자체 메모리를 macOS에 반환했습니다.", { amount: formatBytes(actual.allocatorReleasedBytes) })
        : t("메모리 정리를 완료했습니다. 현재 반환 가능한 자체 메모리는 없습니다.");
      setNotice(message); onCompleted({ message });
    } catch (reason) {
      const message = t("메모리 정리를 완료하지 못했습니다. {{detail}}", { detail: String(reason) });
      setError(message); onCompleted({ message });
    } finally { setRunning(false); onBusyChange?.(false); }
  }

  if (preparedApplication) return <ApplicationReview target={preparedApplication.target} preparedPlan={preparedApplication.plan} busy={busy} onBusyChange={onBusyChange} onClose={onClose}
    onCompleted={(_target, _plan, actual) => onCompleted(actual ? { message: t("요청 {{requested}}개 중 {{moved}}개를 휴지통으로 이동했습니다.", { requested: formatCount(actual.requestedCount), moved: formatCount(actual.movedCount) }), trashResult: actual, mutated: true } : { message: t("결과를 확인하지 못했습니다. 휴지통과 작업 기록을 먼저 확인한 뒤 목록을 새로 고치세요. 같은 요청을 자동으로 재시도하지 않습니다."), mutated: true })} />;
  if (dockerPreview?.items.length) return <DockerCleanupDialog preview={dockerPreview} onClose={onClose} onCompleted={(status) => onCompleted({ message: status.lastCleanup?.message ?? t("현재 상태를 확인하고 있습니다."), dockerStatus: status })} />;
  if (reviewKind === "process" && presentation.preview) return consumed.current && !running
    ? <AppToolNoticeDialog title={t("정상 종료 요청")} message={notice ?? error ?? ""} onClose={onClose} />
    : <ProcessTerminationDialog preview={presentation.preview as TerminationPreview} busy={running || busy} error={error} onClose={onClose} onConfirm={() => void confirmProcess()} />;
  if (reviewKind === "memory") return <dialog ref={memoryDialog} className="safety-dialog" aria-labelledby="app-tool-memory-title" onCancel={(event) => { event.preventDefault(); if (!running) onClose(); }}>
    <h2 id="app-tool-memory-title">{t("앱 메모리 정리")}</h2><p>{t("BroomSweepy 본체의 사용하지 않는 메모리만 반환합니다. 시스템 전체·다른 앱·WebKit·CLI 메모리나 CPU를 정리하지 않습니다.")}</p>
    {notice ? <p role="status">{notice}</p> : null}{error ? <p role="alert">{error}</p> : null}
    <footer><button ref={cancelRef} type="button" className="secondary-button" disabled={running} onClick={onClose}>{t(consumed.current ? "닫기" : "취소")}</button>{!consumed.current ? <button type="button" className="primary-button" disabled={running || busy} onClick={() => void confirmMemory()}>{t("앱 메모리 정리")}</button> : null}</footer>
  </dialog>;
  return <AppToolNoticeDialog title={t("검토 열기")} message={t("검토 정보가 만료됐거나 지원되지 않습니다. 앱에서 다시 조회해 주세요.")} onClose={onClose} />;
}

function AppToolNoticeDialog({ title, message, onClose }: { title: string; message: string; onClose: () => void }) {
  const { t } = useLanguage();
  const dialog = useRef<HTMLDialogElement>(null);
  const close = useRef<HTMLButtonElement>(null);
  useEffect(() => { const node = dialog.current; node?.showModal(); close.current?.focus(); return () => node?.close(); }, []);
  return <dialog ref={dialog} className="safety-dialog" aria-labelledby="app-tool-notice-title" onCancel={(event) => { event.preventDefault(); onClose(); }}><h2 id="app-tool-notice-title">{title}</h2><p role="status">{message}</p><button ref={close} type="button" className="secondary-button" onClick={onClose}>{t("닫기")}</button></dialog>;
}
