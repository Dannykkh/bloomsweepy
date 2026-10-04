import { ArrowLeft, ChartNoAxesColumnDecreasing, File, FolderOpen, Map, RefreshCw, Search, ShieldCheck, Trash2 } from "lucide-react";
import { useEffect, useState, type FormEvent } from "react";
import { useLanguage } from "../i18n";
import { formatBytes, formatCount, formatDate } from "../lib/format";
import type { AssistantFileAction, AssistantFileWorkspace } from "../types";
import { FileInspectionStatus, FileOpenActions, useFileInspectionActions } from "./FileOpenActions";
import "./AssistantFileCard.css";

export function AssistantFileCard({ workspace, busy, onAction, onSelect, onPrepare, onConfirm, onShowMap, onOpenCleanupTree }: {
  workspace: AssistantFileWorkspace; busy: boolean;
  onAction: (action: AssistantFileAction) => void;
  onSelect: (ids: string[]) => void; onPrepare: () => void; onConfirm: (nestedAck: boolean) => void;
  onShowMap: () => void;
  onOpenCleanupTree?: () => void;
}) {
  const { t } = useLanguage();
  const [query, setQuery] = useState(workspace.query ?? "");
  const [nestedAck, setNestedAck] = useState(false);
  const [now, setNow] = useState(Date.now());
  const inspection = useFileInspectionActions({ scopeKey: workspace.revision });
  const plan = workspace.plan;
  useEffect(() => { setQuery(workspace.query ?? ""); }, [workspace.revision]);
  useEffect(() => {
    setNestedAck(false); setNow(Date.now());
    if (!plan) return;
    const timer = window.setInterval(() => setNow(Date.now()), 1_000);
    return () => window.clearInterval(timer);
  }, [plan?.id]);
  const expired = Boolean(plan && now >= plan.expiresAtUnixMs);
  const selected = new Set(workspace.selectedIds);
  const rows = plan?.entries ?? workspace.entries;
  function search(event: FormEvent) { event.preventDefault(); if (query.trim() && !busy) onAction({ kind: "search", query: query.trim() }); }
  return <section className="assistant-empty-review assistant-file-review" aria-label={t("파일·폴더 관리")} aria-busy={busy}>
    <header>
      {plan ? <ShieldCheck size={22} aria-hidden="true" /> : <FolderOpen size={22} aria-hidden="true" />}
      <div><strong>{plan ? t("휴지통 이동 최종 확인") : workspace.sizeRanked ? t("용량이 큰 항목") : t("파일·폴더 관리")}</strong>
        <p>{plan ? t("선택 {{count}}개 · 논리 용량 {{size}}", { count: formatCount(plan.entries.length), size: formatBytes(plan.logicalBytes) })
          : t("{{folder}} · 결과 {{count}}개 · 선택 {{selected}}개", { folder: workspace.currentName, count: formatCount(workspace.totalEntries), selected: formatCount(selected.size) })}</p>
      </div>
    </header>
    <small dir="auto">{workspace.currentPath}</small>
    {!plan ? <>
      <form className="assistant-file-review__search" onSubmit={search}>
        <input type="search" name="assistant-file-search" autoComplete="off" spellCheck={false} aria-label={t("파일·폴더 이름 검색")} placeholder={t("이름 일부로 하위 항목까지 검색")} value={query} maxLength={240} disabled={busy} onChange={(event) => setQuery(event.currentTarget.value)} />
        <button type="submit" className="secondary-button" disabled={busy || !query.trim()}><Search size={16} aria-hidden="true" />{t("검색")}</button>
      </form>
      <div className="assistant-file-review__toolbar">
        {onOpenCleanupTree && workspace.mapGeneration !== null ? <button type="button" className="text-button" disabled={busy} onClick={onOpenCleanupTree}><Trash2 size={16} aria-hidden="true" />{t("정리 후보에서 검토")}</button> : null}
        <button type="button" className="text-button" disabled={busy} onClick={() => onAction({ kind: "largest" })}><ChartNoAxesColumnDecreasing size={16} aria-hidden="true" />{t("큰 항목 찾기")}</button>
        {workspace.mapGeneration !== null ? <button type="button" className="text-button" disabled={busy} onClick={onShowMap}><Map size={16} aria-hidden="true" />{t("같은 결과 용량지도 보기")}</button> : null}
        {workspace.canGoUp ? <button type="button" className="text-button" disabled={busy} onClick={() => onAction({ kind: "parent", revision: workspace.revision })}><ArrowLeft size={16} aria-hidden="true" />{t("상위 폴더")}</button> : null}
        <button type="button" className="text-button" disabled={busy} onClick={() => onAction({ kind: "scan" })}><RefreshCw size={16} aria-hidden="true" />{workspace.query ? t("현재 폴더 목록 보기") : t("다시 검사")}</button>
        <button type="button" className="text-button" disabled={busy || !rows.length} onClick={() => onSelect([...new Set([...workspace.selectedIds, ...rows.map((entry) => entry.id)])])}>{t("현재 페이지 선택")}</button>
        <button type="button" className="text-button" disabled={busy || !selected.size} onClick={() => onSelect([])}>{t("선택 해제")}</button>
      </div>
    </> : null}
    {!plan && workspace.sizeRanked ? <p>{t("현재 폴더의 직계 항목을 용량순으로 검사했습니다. 폴더 용량은 하위 항목의 합계입니다.")} {t("용량만으로 삭제 안전성을 판단할 수 없습니다. 하위 항목·필요 여부·백업을 먼저 확인하세요.")}</p> : null}
    {workspace.truncated ? <p role="status">{t("검사 상한에 도달한 부분 결과입니다. 더 구체적인 이름이나 작은 하위 폴더로 다시 검색하세요.")}</p> : null}
    {workspace.unreadableEntries > 0 ? <p role="status">{t("읽지 못한 항목이 있어 검사 결과가 완전하지 않을 수 있습니다.")}</p> : null}
    <ul className="assistant-empty-review__list assistant-file-review__list">
      {rows.map((entry) => <li key={entry.id}>
        <div className="assistant-file-review__identity"><label>
          {!plan ? <input type="checkbox" checked={selected.has(entry.id)} disabled={busy} onChange={(event) => onSelect(event.currentTarget.checked ? [...workspace.selectedIds, entry.id] : workspace.selectedIds.filter((id) => id !== entry.id))} /> : null}
          {entry.isDirectory ? <FolderOpen size={17} aria-hidden="true" /> : <File size={17} aria-hidden="true" />}
          <span><strong>{entry.number}. {entry.name}</strong><small dir="auto">{entry.path}</small></span>
        </label>
        <span className="assistant-file-review__metric">{entry.logicalBytes === null ? t("폴더 용량 미측정") : formatBytes(entry.logicalBytes)}
          <small>{entry.modifiedAtUnixMs ? formatDate(entry.modifiedAtUnixMs) : t("날짜 확인 불가")}</small>
        </span></div>
        {entry.isDirectory && entry.fileCount !== null ? <small>{t("하위 파일 {{files}}개 · 폴더 {{folders}}개 포함", { files: formatCount(entry.fileCount), folders: formatCount(entry.directoryCount ?? 0) })}</small> : null}
        {plan && (entry.linkCount ?? 0) > 0 ? <small>{t("링크 {{count}}개는 링크 자체만 이동하며 원본은 건드리지 않습니다.", { count: formatCount(entry.linkCount!) })}</small> : null}
        {!plan ? <div className="assistant-file-review__actions">
          {entry.isDirectory ? <button type="button" className="text-button" disabled={busy} onClick={() => onAction({ kind: "browse", revision: workspace.revision, entryId: entry.id })}>{t("하위 항목 확인")}</button> : null}
          <FileOpenActions name={entry.name} directory={entry.isDirectory} disabled={busy || inspection.busy} onOpen={() => void inspection.run(entry.path, entry.name)} onReveal={() => void inspection.run(entry.path, entry.name, "reveal")} />
        </div> : null}
      </li>)}
    </ul>
    {!rows.length ? <p>{t("일치하는 항목이 없습니다. 이름을 바꾸거나 현재 폴더를 다시 검사하세요.")}</p> : null}
    <FileInspectionStatus message={inspection.message} error={inspection.error} />
    {!plan && workspace.totalEntries > 24 ? <div className="assistant-file-review__toolbar">
      <button type="button" className="secondary-button" disabled={busy || workspace.offset === 0} onClick={() => onAction({ kind: "page", revision: workspace.revision, offset: Math.max(0, workspace.offset - 24) })}>{t("이전")}</button>
      <span>{Math.floor(workspace.offset / 24) + 1} / {Math.ceil(workspace.totalEntries / 24)}</span>
      <button type="button" className="secondary-button" disabled={busy || workspace.nextOffset === null} onClick={() => onAction({ kind: "page", revision: workspace.revision, offset: workspace.nextOffset! })}>{t("다음")}</button>
    </div> : null}
    {plan?.requiresNestedAck ? <label className="assistant-file-review__ack"><input type="checkbox" checked={nestedAck} disabled={busy || expired} onChange={(event) => setNestedAck(event.currentTarget.checked)} />{t("선택한 폴더와 그 안의 모든 항목이 함께 휴지통으로 이동함을 확인했습니다.")}</label> : null}
    {plan ? <p role="status">{expired ? t("확인 시간이 만료됐습니다. 선택을 다시 검토하세요.") : t("이 목록만 휴지통으로 이동합니다. 실행 직전 다시 검사하며, 변경된 항목이 있으면 중단합니다.")}</p> : null}
    <footer><small>{t("휴지통 이동 후 운영체제에서 복원을 시도할 수 있습니다. 즉시 여유 공간이 늘어나는 것은 아닙니다.")}</small>
      {plan ? <>
        <button type="button" className="secondary-button" disabled={busy} onClick={() => onSelect(workspace.selectedIds)}>{t("선택 다시 검토")}</button>
        <button type="button" className="trash-confirm-button" disabled={busy || expired || (plan.requiresNestedAck && !nestedAck)} onClick={() => onConfirm(nestedAck)}><Trash2 size={16} aria-hidden="true" />{t("확인한 {{count}}개 휴지통으로 이동", { count: formatCount(plan.entries.length) })}</button>
      </> : <button type="button" className="secondary-button" disabled={busy || !selected.size} onClick={onPrepare}>{t("선택한 항목 최종 검토")}</button>}
    </footer>
  </section>;
}
