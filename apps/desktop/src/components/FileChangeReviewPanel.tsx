import { invoke, isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { AlertTriangle, Clock3, FileClock, FolderOpen, RefreshCw } from "lucide-react";
import { useEffect, useId, useRef, useState } from "react";
import { useLanguage, type MessageKey } from "../i18n";
import { formatBytes, formatCount, formatDate } from "../lib/format";
import {
  createFileChangeReviewPageLoader,
  FILE_CHANGE_REVIEW_PAGE_SIZE,
  fileChangeReviewDisposition,
  fileChangeReviewKind,
  fileChangeReviewPagination,
  fileChangeReviewPhase,
  fileChangeReviewReason,
  fileChangeReviewRelativePath,
  type FileChangeReviewApi,
  type FileChangeReviewConfig,
  type FileChangeReviewFilter,
  type FileChangeReviewPage,
  type FileChangeReviewQuery,
  type FileChangeReviewState,
} from "../lib/fileChangeReview";
import "./FileChangeReviewPanel.css";

const nativeApi: FileChangeReviewApi = {
  getPage: (query) => invoke("get_file_change_review", { ...query }),
  setConfig: (config) => invoke("set_file_change_review_config", { config }),
  checkNow: () => invoke("request_file_change_review_check"),
  setDisposition: (entryId, reviewState) => invoke("set_file_change_review_disposition", { entryId, reviewState }),
  pickFolder: async () => {
    const path = await open({ directory: true, multiple: false });
    return typeof path === "string" ? path : null;
  },
};

export interface FileChangeReviewPanelProps {
  /** Synthetic QA bridge; omitted by the product so native availability remains authoritative. */
  api?: FileChangeReviewApi;
  isAvailable?: boolean;
}

export function FileChangeReviewPanel({ api = nativeApi, isAvailable }: FileChangeReviewPanelProps = {}) {
  const { t } = useLanguage();
  const available = isAvailable ?? isTauri();
  const id = useId();
  const [page, setPage] = useState<FileChangeReviewPage | null>(null);
  const [query, setQuery] = useState<FileChangeReviewQuery>({ offset: 0, filter: "review_needed" });
  const [loading, setLoading] = useState(available);
  const [working, setWorking] = useState<string | null>(null);
  const [showWorking, setShowWorking] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);
  const [announcement, setAnnouncement] = useState("");
  const [retry, setRetry] = useState(0);
  const mounted = useRef(false);
  const operation = useRef(false);
  const queryRef = useRef(query);
  const pageRef = useRef(page);
  const loaderRef = useRef<ReturnType<typeof createFileChangeReviewPageLoader> | null>(null);
  const tRef = useRef(t);
  queryRef.current = query;
  pageRef.current = page;
  tRef.current = t;

  function accept(next: FileChangeReviewPage) {
    if (!mounted.current || (pageRef.current && next.status.revision < pageRef.current.status.revision)) return;
    const pagination = fileChangeReviewPagination(next.offset, next.total, next.entries.length);
    setPage(next);
    setFailure(null);
    if (pagination.offset !== next.offset) setQuery((current) => ({ ...current, offset: pagination.offset }));
  }

  useEffect(() => {
    mounted.current = true;
    const loader = createFileChangeReviewPageLoader(api.getPage);
    loaderRef.current = loader;
    return () => {
      mounted.current = false;
      loader.dispose();
      loaderRef.current = null;
    };
  }, [api]);

  useEffect(() => {
    if (!available) { setLoading(false); return; }
    let disposed = false;
    let timer: number | undefined;
    setLoading(true);
    const load = async () => {
      if (disposed) return;
      if (!operation.current) {
        try {
          const next = await loaderRef.current?.load(queryRef.current);
          if (!disposed && next) accept(next);
        } catch {
          if (!disposed) setFailure(tRef.current("변경 파일 검토 상태를 불러오지 못했습니다. 다시 확인하세요."));
        } finally {
          if (!disposed) setLoading(false);
        }
      }
      if (!disposed) timer = window.setTimeout(() => void load(), 5_000);
    };
    void load();
    return () => { disposed = true; if (timer !== undefined) window.clearTimeout(timer); };
  }, [api, available, query.filter, query.offset, retry]);

  useEffect(() => {
    if (!loading && !working) { setShowWorking(false); return; }
    const timer = window.setTimeout(() => setShowWorking(true), 300);
    return () => window.clearTimeout(timer);
  }, [loading, working]);

  async function perform(label: string, action: () => Promise<unknown>, complete: string | (() => string)) {
    if (!available || operation.current || !loaderRef.current) return;
    operation.current = true;
    const loader = loaderRef.current;
    loader.invalidate();
    setWorking(label);
    setFailure(null);
    setAnnouncement(label);
    try {
      await action();
      const next = await loader.load(queryRef.current);
      if (!mounted.current || loader !== loaderRef.current) return;
      if (next) {
        accept(next);
        setAnnouncement(next.status.error ? t("설정과 검사 상태를 확인하세요. {{detail}}", { detail: next.status.error })
          : typeof complete === "function" ? complete() : complete);
      }
    } catch (reason) {
      if (!mounted.current || loader !== loaderRef.current) return;
      const detail = typeof reason === "string" ? reason : reason instanceof Error ? reason.message : "";
      const message = t("변경 파일 검토 작업을 완료하지 못했습니다. 다시 확인하세요. {{detail}}", { detail: detail.slice(0, 400) });
      setFailure(message);
      setAnnouncement(message);
      // A rejected response does not establish the actual native configuration.
      try { const actual = await loader.load(queryRef.current); if (actual) { accept(actual); setFailure(message); } } catch { /* retry remains available */ }
    } finally {
      if (mounted.current && loader === loaderRef.current) { setWorking(null); setLoading(false); }
      if (loader === loaderRef.current) operation.current = false;
    }
  }

  const status = page?.status;
  const config = status?.config;
  const disabled = !available || !config || loading || Boolean(working);
  const checking = status?.phase === "baseline" || status?.phase === "checking";
  const pagination = fileChangeReviewPagination(page?.offset ?? query.offset, page?.total ?? 0, page?.entries.length ?? 0);
  const rows = page?.entries.slice(0, FILE_CHANGE_REVIEW_PAGE_SIZE) ?? [];
  const translateLabel = (message: string) => t(message as MessageKey);

  function changeConfig(next: FileChangeReviewConfig) {
    void perform(t("검토 설정 저장 중…"), () => api.setConfig(next), t("변경 파일 검토 설정을 저장했습니다."));
  }

  function chooseFolder() {
    let selected = false;
    void perform(t("폴더 선택 중…"), async () => {
      const root = await api.pickFolder();
      if (root && config) {
        queryRef.current = { ...queryRef.current, offset: 0 };
        setQuery(queryRef.current);
        await api.setConfig({ ...config, root });
        selected = true;
      }
    }, () => selected ? t("대상 폴더를 저장했습니다.") : t("폴더 선택을 취소했습니다."));
  }

  function changeDisposition(entryId: string, reviewState: FileChangeReviewState) {
    void perform(t("검토 상태 저장 중…"), () => api.setDisposition(entryId, reviewState),
      reviewState === "skipped" ? t("이 변경 기록을 넘겼습니다. 안전 판정은 아닙니다.") : t("이 변경 기록을 다시 검토 목록에 넣었습니다."));
  }

  function changeFilter(filter: FileChangeReviewFilter) {
    if (operation.current) return;
    loaderRef.current?.invalidate();
    setQuery({ offset: 0, filter });
  }

  return <section className="settings-panel file-change-review" aria-labelledby={`${id}-title`} aria-busy={loading || Boolean(working) || checking}>
    <div className="settings-panel__heading file-change-review__heading">
      <FileClock size={20} aria-hidden="true" />
      <div><h2 id={`${id}-title`}>{t("변경 파일 검토")}</h2>
        <p id={`${id}-scope`}>{t("선택한 폴더를 주기적으로 비교합니다. 백신이나 실시간 감시는 아닙니다.")}</p>
        <p>{t("파일 내용의 자동 전송과 자동 삭제는 없습니다.")}</p>
      </div>
    </div>
    <label className="setting-row developer-tool-toggle file-change-review__toggle">
      <span><strong>{t("주기적으로 변경 확인")}</strong><small id={`${id}-toggle-description`}>{t("기본값은 꺼짐입니다. 대상 폴더를 선택한 뒤 켜세요.")}</small></span>
      <span className="developer-tool-toggle__control">
        <input type="checkbox" role="switch" name="fileChangeReviewEnabled" checked={config?.enabled ?? false}
          aria-label={t("변경 파일 검토 사용")} aria-describedby={`${id}-scope ${id}-toggle-description`}
          disabled={disabled || (!config?.enabled && !config?.root)} onChange={(event) => config && changeConfig({ ...config, enabled: event.currentTarget.checked })} />
        <strong>{!available ? t("확인 불가") : !config ? t("확인 중…") : config.enabled ? t("켜짐") : t("꺼짐")}</strong>
      </span>
    </label>
    <div className="file-change-review__scope">
      <div className="file-change-review__folder"><strong>{t("대상 폴더")}</strong><span>{config?.root ?? t("선택한 폴더 없음")}</span></div>
      <button type="button" className={config?.root ? "secondary-button" : "primary-button"} disabled={disabled} onClick={chooseFolder}>
        <FolderOpen size={16} aria-hidden="true" />{t("폴더 선택")}
      </button>
    </div>
    <div className="file-change-review__schedule">
      <label htmlFor={`${id}-interval`}>{t("확인 주기")}</label>
      <select id={`${id}-interval`} value={config?.intervalMinutes ?? 15} disabled={disabled}
        onChange={(event) => config && changeConfig({ ...config, intervalMinutes: Number(event.currentTarget.value) as 5 | 15 | 30 })}>
        {[5, 15, 30].map((minutes) => <option key={minutes} value={minutes}>{t("{{minutes}}분", { minutes })}</option>)}
      </select>
      <button type="button" className="primary-button" disabled={disabled || !config?.enabled || !config.root || checking || status?.phase === "busy"}
        onClick={() => void perform(t("변경 확인 요청 중…"), api.checkNow, t("변경 확인을 요청했습니다."))}>
        <RefreshCw size={16} aria-hidden="true" />{t("지금 확인")}
      </button>
    </div>
    {!available ? <p className="file-change-review__notice">{t("변경 파일 검토는 설치한 데스크톱 앱에서 사용할 수 있습니다.")}</p> : null}
    {showWorking ? <p className="file-change-review__notice">{working ?? t("검토 상태 확인 중…")}</p> : null}
    {failure ? <div className="file-change-review__error"><AlertTriangle size={18} aria-hidden="true" /><div><p>{failure}</p>
      <button type="button" className="secondary-button" disabled={Boolean(working)} onClick={() => { setAnnouncement(t("검토 상태를 다시 확인합니다.")); setRetry((value) => value + 1); }}>{t("다시 확인")}</button>
    </div></div> : null}
    {status ? <>
      <dl className="file-change-review__status">
        <div><dt>{t("검사 상태")}</dt><dd><Clock3 size={16} aria-hidden="true" />{translateLabel(fileChangeReviewPhase(status.phase))}</dd></div>
        <div><dt>{t("비교 기준")}</dt><dd>{t("{{count}}개 항목", { count: formatCount(status.baselineEntries) })}</dd></div>
        <div><dt>{t("마지막 완전 검사")}</dt><dd>{status.lastCheckedAtUnixMs === null ? t("아직 완료된 검사 없음") : formatDate(status.lastCheckedAtUnixMs)}</dd></div>
        <div><dt>{t("다음 확인")}</dt><dd>{status.nextCheckAtUnixMs === null ? "—" : formatDate(status.nextCheckAtUnixMs)}</dd></div>
      </dl>
      {status.phase === "baseline" ? <p className="file-change-review__notice">{t("첫 목록은 비교 기준입니다. 이 검사에서는 변경 기록을 만들지 않습니다.")}</p> : null}
      {status.phase === "off" ? <p className="file-change-review__notice">{t("주기 확인이 꺼져 있습니다. 폴더를 선택하고 켜면 비교 기준부터 만듭니다.")}</p> : null}
      {status.phase === "busy" ? <p className="file-change-review__notice">{t("다른 검사가 진행 중입니다. 자원을 나눠 쓰지 않고 다음 주기에 다시 시도합니다.")}</p> : null}
      {status.phase === "incomplete" || status.phase === "error" || status.error ? <div className="file-change-review__error"><AlertTriangle size={18} aria-hidden="true" /><div>
        <strong>{t("검사 미완료")}</strong><p>{t("이전 완전 검사 결과를 유지합니다. 더 작은 로컬 폴더를 선택하거나 다시 확인하세요.")}</p>
        {status.error ? <p className="file-change-review__detail">{status.error}</p> : null}
      </div></div> : null}
      <div className="file-change-review__toolbar">
        <div className="file-change-review__filters" role="group" aria-label={t("변경 기록 필터")}>
          {(["review_needed", "skipped", "all"] as const).map((filter) => <button type="button" key={filter}
            aria-pressed={query.filter === filter} disabled={Boolean(working)} onClick={() => changeFilter(filter)}>
            {filter === "review_needed" ? t("검토 필요") : filter === "skipped" ? t("넘김") : t("전체")}
            <span>{formatCount(filter === "review_needed" ? status.reviewNeededCount : filter === "skipped" ? status.skippedCount : status.historyCount)}</span>
          </button>)}
        </div>
        <span>{t("{{count}}개 기록", { count: formatCount(pagination.total) })}</span>
      </div>
      <p className="file-change-review__review-note">{t("넘김은 안전 판정이 아닙니다. 같은 파일이 다시 바뀌면 새 기록으로 검토합니다.")}</p>
      {rows.length ? <ul className="file-change-review__list">{rows.map((entry) => <li key={entry.id} className="file-change-review__row">
        <div className="file-change-review__identity"><strong>{entry.name}</strong><span>{fileChangeReviewRelativePath(entry.path, config?.root ?? null)}</span>
          {entry.previousPath ? <span>{t("이전 경로: {{path}}", { path: fileChangeReviewRelativePath(entry.previousPath, config?.root ?? null) })}</span> : null}
        </div>
        <div className="file-change-review__evidence"><div className="file-change-review__labels"><span>{translateLabel(fileChangeReviewKind(entry.kind))}</span>
          <span className={entry.reviewState === "review_needed" ? "is-review-needed" : ""}>{translateLabel(fileChangeReviewDisposition(entry))}</span></div>
          <span>{entry.reasons.length ? [...new Set(entry.reasons)].map((reason) => translateLabel(fileChangeReviewReason(reason))).join(" · ") : t("검토 사유 없음")}</span>
          <span className="file-change-review__metadata">{entry.isDirectory || entry.logicalBytes === null ? t("미측정 용량") : formatBytes(entry.logicalBytes)} · {formatDate(entry.modifiedAtUnixMs)}</span>
        </div>
        <button type="button" className="secondary-button" disabled={disabled || checking}
          aria-label={entry.reviewState === "review_needed" ? t("{{name}} 변경 기록 넘김", { name: entry.name }) : t("{{name}} 변경 기록 다시 검토", { name: entry.name })}
          onClick={() => changeDisposition(entry.id, entry.reviewState === "review_needed" ? "skipped" : "review_needed")}>
          {entry.reviewState === "review_needed" ? t("넘김") : t("다시 검토")}
        </button>
      </li>)}</ul> : <div className="file-change-review__empty"><strong>{query.filter === "skipped" ? t("넘긴 기록이 없습니다.") : query.filter === "all" ? t("변경 기록이 없습니다.") : t("검토할 변경 기록이 없습니다.")}</strong>
        <p>{checking ? t("현재 검사 결과를 기다리고 있습니다.") : config?.enabled ? t("다음 완전 검사에서 변경된 항목이 있으면 여기에 표시합니다.") : t("이 옵션을 켜면 선택한 폴더의 변경을 확인할 수 있습니다.")}</p>
      </div>}
      {pagination.total > 0 ? <nav className="file-change-review__pagination" aria-label={t("변경 기록 페이지")}>
        <span>{t("{{start}}–{{end}} / {{total}}", { start: formatCount(pagination.start), end: formatCount(pagination.end), total: formatCount(pagination.total) })}</span>
        <button type="button" className="secondary-button" disabled={Boolean(working) || loading || pagination.previousOffset === null} onClick={() => setQuery((current) => ({ ...current, offset: pagination.previousOffset ?? 0 }))}>{t("이전")}</button>
        <button type="button" className="secondary-button" disabled={Boolean(working) || loading || pagination.nextOffset === null} onClick={() => setQuery((current) => ({ ...current, offset: pagination.nextOffset ?? 0 }))}>{t("다음")}</button>
      </nav> : null}
      {status.historyTruncated ? <p className="file-change-review__notice is-warning">{t("보관·표시 한도로 일부 기록을 생략했습니다. 전체 변경 목록은 아닙니다.")}</p> : null}
    </> : null}
    <details className="file-change-review__limits"><summary>{t("비교 기준과 자원 한도")}</summary>
      <p>{t("폴더 변경과 앱 재시작 시 비교 기준과 기록을 새로 만듭니다. 주기 변경·끄기·다시 켜기는 비교 기준을 새로 만듭니다.")}</p>
      <p>{t("검사당 10,000개 항목·경로 4 MiB·8초, 최근 기록 250개·1 MiB로 제한합니다. 한 번에 20개 기록을 표시합니다.")}</p>
      <p>{t("클라우드 전용 항목이나 읽을 수 없는 경로가 포함되면 검사가 미완료될 수 있습니다.")}</p>
    </details>
    <span className="sr-only" role="status" aria-live="polite" aria-atomic="true">{announcement}</span>
  </section>;
}
