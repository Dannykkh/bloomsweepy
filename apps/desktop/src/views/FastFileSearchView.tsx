import {
  AlertTriangle,
  Database,
  File,
  Folder,
  FolderOpen,
  HardDrive,
  Link2,
  LoaderCircle,
  RefreshCw,
  Search,
  ShieldCheck,
  Trash2,
  X,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { inspectFile, revealPath, searchFileCatalog } from "../lib/bridge";
import {
  formatBytes,
  formatCount,
  formatDate,
  formatDateTimeAttribute,
  formatDuration,
} from "../lib/format";
import type {
  FileCatalogEntryKind,
  FileCatalogProgress,
  FileCatalogReport,
  FileCatalogSearchReport,
  FileCatalogSort,
  FileCatalogStatus,
  ScanUiState,
} from "../types";
import { useLanguage, type MessageKey } from "../i18n";

interface FastFileSearchViewProps {
  selectedRoot: string | null;
  defaultRoot: string | null;
  catalog: FileCatalogStatus | null;
  lastBuild: FileCatalogReport | null;
  progress: FileCatalogProgress | null;
  state: ScanUiState;
  error: string | null;
  stale: boolean;
  blocked: boolean;
  onPickFolder: () => void;
  onStartCatalog: () => void;
  onCancelCatalog: () => void;
  onClearCatalog: () => Promise<void>;
}

type KindFilter = "all" | "file" | "directory";
type SearchState = "idle" | "searching" | "success" | "error";

const kindFilters: Array<{ id: KindFilter; label: MessageKey }> = [
  { id: "all", label: "전체" },
  { id: "file", label: "파일" },
  { id: "directory", label: "폴더" },
];

const sizeFilters: ReadonlyArray<{ value: string; label: MessageKey; minBytes: number | null }> = [
  { value: "0", label: "모든 크기", minBytes: null },
  { value: "100", label: "100 MB 이상", minBytes: 100 * 1024 * 1024 },
  { value: "1024", label: "1 GB 이상", minBytes: 1024 * 1024 * 1024 },
  { value: "10240", label: "10 GB 이상", minBytes: 10 * 1024 * 1024 * 1024 },
];

const sortOptions: Array<{ value: FileCatalogSort; label: MessageKey }> = [
  { value: "relevance", label: "검색어와 가까운 순" },
  { value: "name", label: "이름 가나다순" },
  { value: "largest", label: "용량 큰 순" },
  { value: "modified", label: "최근에 바뀐 순" },
];

export function FastFileSearchView({
  selectedRoot,
  defaultRoot,
  catalog,
  lastBuild,
  progress,
  state,
  error,
  stale,
  blocked,
  onPickFolder,
  onStartCatalog,
  onCancelCatalog,
  onClearCatalog,
}: FastFileSearchViewProps) {
  const { t } = useLanguage();
  const [query, setQuery] = useState("");
  const [kind, setKind] = useState<KindFilter>("all");
  const [extensions, setExtensions] = useState("");
  const [sizeFilter, setSizeFilter] = useState("0");
  const [sort, setSort] = useState<FileCatalogSort>("relevance");
  const [searchState, setSearchState] = useState<SearchState>("idle");
  const [searchReport, setSearchReport] = useState<FileCatalogSearchReport | null>(null);
  const [searchError, setSearchError] = useState<string | null>(null);
  const [inspectionMessage, setInspectionMessage] = useState<string | null>(null);
  const [confirmClear, setConfirmClear] = useState(false);
  const [showBuildProgress, setShowBuildProgress] = useState(false);
  const requestSequence = useRef(0);

  const scopeChanged = Boolean(
    selectedRoot && catalog && !pathsEqual(selectedRoot, catalog.root),
  );
  const effectiveRoot = selectedRoot ?? catalog?.root ?? defaultRoot;
  const canSearch = Boolean(
    catalog && !scopeChanged && !stale && state !== "scanning" && !blocked,
  );
  const normalizedExtensions = useMemo(
    () =>
      extensions
        .split(/[\s,;]+/)
        .map((value) => value.trim().replace(/^\./, "").toLocaleLowerCase("en-US"))
        .filter(Boolean),
    [extensions],
  );
  const minBytes =
    sizeFilters.find((option) => option.value === sizeFilter)?.minBytes ?? null;

  useEffect(() => {
    if (state !== "scanning") {
      setShowBuildProgress(false);
      return;
    }
    const timer = window.setTimeout(() => setShowBuildProgress(true), 300);
    return () => window.clearTimeout(timer);
  }, [state]);

  useEffect(() => {
    setConfirmClear(false);
    setSearchReport(null);
    setSearchError(null);
    setSearchState("idle");
  }, [catalog?.completedAtUnixMs, scopeChanged]);

  useEffect(() => {
    const normalized = query.trim();
    const sequence = ++requestSequence.current;
    if (!canSearch || !normalized) {
      setSearchReport(null);
      setSearchError(null);
      setSearchState("idle");
      return;
    }

    setSearchState("searching");
    setSearchError(null);
    const timer = window.setTimeout(() => {
      void searchFileCatalog({
        query: normalized,
        kind: kind === "all" ? null : (kind as FileCatalogEntryKind),
        extensions: normalizedExtensions,
        minBytes,
        maxBytes: null,
        timezoneOffsetMinutes: new Date().getTimezoneOffset(),
        sort,
        maxResults: 100,
      })
        .then((report) => {
          if (requestSequence.current !== sequence) return;
          setSearchReport(report);
          setSearchState("success");
        })
        .catch((reason: unknown) => {
          if (requestSequence.current !== sequence) return;
          setSearchError(normalizeError(reason, t("알 수 없는 오류가 발생했습니다"), t));
          setSearchState("error");
        });
    }, 140);

    return () => window.clearTimeout(timer);
  }, [canSearch, kind, minBytes, normalizedExtensions, query, sort]);

  async function openEntry(path: string, name: string, entryKind: FileCatalogEntryKind) {
    setInspectionMessage(null);
    try {
      const outcome = await inspectFile(path, entryKind);
      setInspectionMessage(
        entryKind === "directory"
          ? t("{{name}} 폴더의 위치를 표시했습니다.", { name })
          : outcome === "opened"
            ? t("{{name}} 파일을 기본 앱으로 열었습니다.", { name })
            : t("{{name}}은 직접 열도록 허용한 문서·미디어 형식이 아니라 폴더에서 위치만 표시했습니다.", { name }),
      );
    } catch (reason) {
      setInspectionMessage(t("{{name}}을 열지 못했습니다: {{detail}}", {
        name,
        detail: normalizeError(reason, t("알 수 없는 오류가 발생했습니다"), t),
      }));
    }
  }

  async function showInFolder(path: string, name: string) {
    setInspectionMessage(null);
    try {
      await revealPath(path);
      setInspectionMessage(t("{{name}} 위치를 파일 탐색기에서 표시했습니다.", { name }));
    } catch (reason) {
      setInspectionMessage(t("{{name}} 위치를 표시하지 못했습니다: {{detail}}", {
        name,
        detail: normalizeError(reason, t("알 수 없는 오류가 발생했습니다"), t),
      }));
    }
  }

  async function clearCatalog() {
    if (!confirmClear) {
      setConfirmClear(true);
      return;
    }
    await onClearCatalog();
    setConfirmClear(false);
  }

  return (
    <div className="view-stack fast-file-search-view">
      <section className="document-search-stage file-search-stage" aria-labelledby="file-search-title">
        <div className="document-search-stage__heading">
          <span className="document-search-stage__seal" aria-hidden="true">
            <Search size={22} />
          </span>
          <div>
            <p className="eyebrow">{t("내 파일에서 찾기")}</p>
            <h2 id="file-search-title">{t("파일 이름이나 폴더 위치로 찾으세요")}</h2>
            <p>{t("찾고 싶은 이름을 입력하세요. 파일을 열지 않고 저장된 이름과 위치만 확인합니다.")}</p>
          </div>
          <span className="document-local-badge">
            <ShieldCheck size={14} aria-hidden="true" />
            {t("이 기기 안에서만 검색")}
          </span>
        </div>

        <div className="document-query file-query" role="search">
          <Search size={20} aria-hidden="true" />
          <label htmlFor="file-query-input" className="sr-only">
            {t("파일명 또는 경로 검색어")}
          </label>
          <input
            id="file-query-input"
            name="file-query"
            type="search"
            value={query}
            disabled={!canSearch}
            maxLength={256}
            autoComplete="off"
            spellCheck={false}
            aria-describedby="file-query-syntax"
            placeholder={
              scopeChanged
                ? t("선택한 위치의 파일 목록을 먼저 새로고침하세요…")
                : stale
                  ? t("옮긴 파일을 반영하도록 파일 목록을 새로고침하세요…")
                : catalog
                  ? t("예: 8월 보고서…")
                  : t("먼저 찾을 위치의 파일 목록을 만드세요…")
            }
            onChange={(event) => setQuery(event.currentTarget.value)}
          />
          <span className="file-query__state" aria-live="polite">
            {searchState === "searching" ? (
              <LoaderCircle className="is-spinning" size={16} aria-hidden="true" />
            ) : (
              <Search size={16} aria-hidden="true" />
            )}
            {searchState === "searching"
              ? t("검색 중…")
              : searchReport
                ? t("{{count}}개", { count: formatCount(searchReport.results.length) })
                : t("즉시 검색")}
          </span>
        </div>

        <details className="file-query-help">
          <summary id="file-query-syntax">{t("검색을 더 정확하게 하는 법")}</summary>
          <div className="file-query-syntax" role="note">
            <span><small>{t("둘 중 하나")}</small><code translate="no">OR</code></span>
            <span><small>{t("이름 모양")}</small><code translate="no">glob:report-*.pdf</code></span>
            <span><small>{t("파일 종류")}</small><code translate="no">ext:pdf,jpg</code></span>
            <span><small>{t("파일만")}</small><code translate="no">type:file</code></span>
            <span><small>{t("폴더 위치")}</small><code translate="no">path:&quot;report&quot;</code></span>
            <span><small>{t("100 MB보다 큼")}</small><code translate="no">size:&gt;100mb</code></span>
            <span><small>{t("이 날짜 뒤")}</small><code translate="no">after:2026-01-01</code></span>
            <span><small>{t("이 날짜 앞")}</small><code translate="no">before:2027-01-01</code></span>
            <span><small>{t("이 단어 빼기")}</small><code translate="no">-draft</code></span>
          </div>
        </details>

        <div className="document-filter-row file-filter-row">
          <div className="segmented-control" role="group" aria-label={t("찾을 대상 선택")}>
            {kindFilters.map((item) => (
              <button
                type="button"
                className={kind === item.id ? "is-active" : ""}
                aria-pressed={kind === item.id}
                key={item.id}
                onClick={() => setKind(item.id)}
              >
                {t(item.label)}
              </button>
            ))}
          </div>
          <label className="file-filter-field">
            <span>{t("파일 종류")}</span>
            <input
              name="file-extensions"
              type="text"
              value={extensions}
              disabled={!catalog}
              autoComplete="off"
              spellCheck={false}
              placeholder="pdf, jpg, rs…"
              aria-label={t("파일 종류 필터")}
              onChange={(event) => setExtensions(event.currentTarget.value)}
            />
          </label>
          <label className="file-filter-field">
            <span>{t("파일 크기")}</span>
            <select
              name="file-size-filter"
              value={sizeFilter}
              disabled={!catalog}
              aria-label={t("최소 파일 크기")}
              onChange={(event) => setSizeFilter(event.currentTarget.value)}
            >
              {sizeFilters.map((option) => (
                <option value={option.value} key={option.value}>{t(option.label)}</option>
              ))}
            </select>
          </label>
          <label className="file-filter-field">
            <span>{t("정렬 기준")}</span>
            <select
              name="file-sort"
              value={sort}
              disabled={!catalog}
              aria-label={t("검색 결과 정렬")}
              onChange={(event) => setSort(event.currentTarget.value as FileCatalogSort)}
            >
              {sortOptions.map((option) => (
                <option value={option.value} key={option.value}>{t(option.label)}</option>
              ))}
            </select>
          </label>
        </div>
      </section>

      {scopeChanged ? (
        <section className="document-scope-warning" role="status">
          <AlertTriangle size={18} aria-hidden="true" />
          <div>
            <strong>{t("선택한 위치가 지금 만든 파일 목록과 다릅니다")}</strong>
            <p>{t("이전 위치의 결과와 섞이지 않도록 새 위치의 목록을 다시 만드세요.")}</p>
          </div>
          <button type="button" disabled={blocked || state === "scanning"} onClick={onStartCatalog}>
            {t("새 위치 다시 읽기")}
          </button>
        </section>
      ) : null}

      {stale && !scopeChanged ? (
        <section className="document-scope-warning" role="status">
          <AlertTriangle size={18} aria-hidden="true" />
          <div>
            <strong>{t("옮긴 파일이 검색 목록에 아직 남아 있습니다")}</strong>
            <p>{t("예전 위치를 보여주지 않도록 검색을 잠시 멈췄습니다. 파일 목록을 새로고침하세요.")}</p>
          </div>
          <button type="button" disabled={blocked || state === "scanning"} onClick={onStartCatalog}>
            {t("지금 새로고침")}
          </button>
        </section>
      ) : null}

      <section className="document-index-console" aria-label={t("검색용 파일 목록 상태")}>
        <div className="document-index-console__status">
          <span aria-hidden="true"><Database size={19} /></span>
          <div>
            <strong>
              {state === "scanning"
                ? t("파일 이름과 위치를 읽고 있습니다…")
                : catalog
                  ? t("파일과 폴더 {{count}}개를 바로 찾을 수 있습니다", { count: formatCount(catalog.indexedEntries) })
                  : t("아직 검색할 파일 목록이 없습니다")}
            </strong>
            <p title={catalog?.root ?? effectiveRoot ?? undefined}>
              {state === "scanning"
                ? t("폴더 안의 파일을 확인하고 있습니다…")
                : catalog
                  ? t("{{date}} 새로고침 · {{provider}} · {{mode}} · 파일 {{files}}개 · 폴더 {{folders}}개 · {{duration}}", {
                      date: formatDate(catalog.completedAtUnixMs),
                      provider: t(providerLabel(catalog.provider)),
                      mode: t(refreshModeLabel(catalog.refreshMode)),
                      files: formatCount(catalog.indexedFiles),
                      folders: formatCount(catalog.indexedDirectories),
                      duration: formatDuration(catalog.durationMs),
                    })
                  : t("{{root}} · 처음 한 번 이름과 위치를 읽습니다.", {
                      root: effectiveRoot ?? t("찾을 위치를 선택하지 않음"),
                    })}
            </p>
          </div>
        </div>

        {state === "scanning" ? (
          <button className="document-cancel-button" type="button" onClick={onCancelCatalog}>
            <X size={15} aria-hidden="true" />
            {t("읽기 취소")}
          </button>
        ) : (
          <div className="document-index-console__actions file-catalog-actions">
            <button type="button" disabled={blocked} onClick={onPickFolder}>
              <FolderOpen size={15} aria-hidden="true" />
              {t("찾을 위치 선택")}
            </button>
            <button
              className={!catalog ? "primary-button" : ""}
              type="button"
              disabled={blocked || !effectiveRoot}
              onClick={onStartCatalog}
            >
              <RefreshCw size={15} aria-hidden="true" />
              {catalog && !scopeChanged ? t("파일 목록 새로고침") : t("파일 목록 만들기")}
            </button>
            {catalog ? (
              <button
                className={confirmClear ? "danger-button" : ""}
                type="button"
                disabled={blocked}
                onBlur={() => setConfirmClear(false)}
                onClick={() => void clearCatalog()}
              >
                <Trash2 size={15} aria-hidden="true" />
                {confirmClear ? t("한 번 더 눌러 지우기") : t("파일 목록 지우기")}
              </button>
            ) : null}
          </div>
        )}

        {state === "scanning" && showBuildProgress ? (
          <div className="document-index-progress" role="status" aria-live="polite">
            <IndexMetric label={t("확인함")} value={t("{{count}}개", { count: formatCount(progress?.scannedEntries ?? 0) })} />
            <IndexMetric label={t("목록에 넣음")} value={t("{{count}}개", { count: formatCount(progress?.indexedEntries ?? 0) })} />
            <IndexMetric label={t("파일")} value={t("{{count}}개", { count: formatCount(progress?.indexedFiles ?? 0) })} />
            <IndexMetric label={t("폴더")} value={t("{{count}}개", { count: formatCount(progress?.indexedDirectories ?? 0) })} />
          </div>
        ) : null}
      </section>

      {error ? (
        <section className="document-error" role="alert">
          <AlertTriangle size={18} aria-hidden="true" />
          <div><strong>{t("파일 목록을 만들지 못했습니다")}</strong><p>{friendlySearchError(error, t)}</p></div>
        </section>
      ) : null}

      {lastBuild && catalog?.completedAtUnixMs === lastBuild.completedAtUnixMs ? (
        <>
          <section className="document-build-evidence" aria-label={t("최근 파일 목록 만들기 결과")}>
            <span><strong>{formatCount(lastBuild.indexedFiles)}</strong>{t("파일")}</span>
            <span><strong>{formatCount(lastBuild.indexedDirectories)}</strong>{t("폴더")}</span>
            <span><strong>{formatCount(lastBuild.removedEntries)}</strong>{t("삭제 반영")}</span>
            <span><strong>{formatCount(lastBuild.unreadableEntries)}</strong>{t("읽기 실패")}</span>
            <p>{t("파일 내용은 읽지 않고 이름·위치·크기·바뀐 시각만 저장합니다. 찾은 결과가 곧 지워도 되는 파일이라는 뜻은 아닙니다.")}</p>
          </section>
          {lastBuild.entryLimitReached ? (
            <section className="document-scope-warning" role="status">
              <AlertTriangle size={18} aria-hidden="true" />
              <div><strong>{t("한 번에 담을 수 있는 파일 수를 넘었습니다")}</strong><p>{t("일부 파일이 빠졌을 수 있습니다. 더 작은 폴더를 선택하세요.")}</p></div>
            </section>
          ) : null}
          {lastBuild.issues.length > 0 ? (
            <details className="document-index-issues">
              <summary>{t("확인하지 못한 위치 {{count}}개", { count: formatCount(lastBuild.issues.length) })}</summary>
              <ul>
                {lastBuild.issues.slice(0, 20).map((issue, issueIndex) => (
                  <li key={`${issue.path ?? "unknown"}-${issueIndex}`}>
                    <strong title={issue.path ?? undefined}>{issue.path ?? t("경로 정보 없음")}</strong>
                    <span>{issue.message}</span>
                  </li>
                ))}
              </ul>
              {lastBuild.issues.length > 20 ? <p>{t("화면에는 처음 20개 사유만 표시합니다.")}</p> : null}
            </details>
          ) : null}
        </>
      ) : null}

      {searchError ? (
        <section className="document-error" role="alert">
          <AlertTriangle size={18} aria-hidden="true" />
          <div><strong>{t("검색 조건을 이해하지 못했습니다")}</strong><p>{friendlySearchError(searchError, t)}</p></div>
        </section>
      ) : null}

      {searchReport ? (
        <section className="document-results file-results" aria-labelledby="file-results-title">
          <header className="document-results__header">
            <div>
              <p className="eyebrow">{t("이 기기에서 찾은 결과")}</p>
              <h2 id="file-results-title">{t("“{{query}}” 검색 결과", { query: searchReport.query })}</h2>
              <p>
                {t("파일과 폴더 {{count}}개를 {{duration}}에 확인했습니다.", {
                  count: formatCount(searchReport.indexedEntries),
                  duration: formatDuration(searchReport.searchDurationMs),
                })}
                {searchReport.resultsTruncated ? ` ${t("처음 100개만 보여줍니다.")}` : ""}
              </p>
            </div>
            <span>{sortOptions.find((option) => option.value === sort)?.label ? t(sortOptions.find((option) => option.value === sort)!.label) : null}</span>
          </header>

          {searchReport.results.length === 0 ? (
            <div className="document-results-empty">
              <Search size={24} aria-hidden="true" />
              <strong>{t("일치하는 파일이나 폴더가 없습니다")}</strong>
              <p>{t("검색어를 짧게 하거나 파일 종류와 크기 조건을 바꿔보세요.")}</p>
            </div>
          ) : (
            <div className="document-result-list">
              {searchReport.results.map((result) => {
                const EntryIcon = result.kind === "directory" ? Folder : result.kind === "symlink" ? Link2 : File;
                return (
                  <article
                    className="document-result file-result"
                    tabIndex={0}
                    key={result.path}
                    aria-label={t("{{name}}, 더블클릭하거나 Enter 키를 눌러 열기", { name: result.name })}
                    onDoubleClick={() => void openEntry(result.path, result.name, result.kind)}
                    onKeyDown={(event) => {
                      if (event.key === "Enter") void openEntry(result.path, result.name, result.kind);
                    }}
                  >
                    <span className="document-result__icon" aria-hidden="true"><EntryIcon size={18} /></span>
                    <div className="document-result__body">
                      <div className="document-result__identity">
                        <strong title={result.name}>{result.name}</strong>
                        <span>{t(kindLabel(result.kind))}</span>
                        {result.matchSource === "path" ? <span>{t("폴더 위치 일치")}</span> : null}
                      </div>
                      <span className="document-result__path" title={result.path}>{result.parent}</span>
                    </div>
                    <div className="document-result__meta">
                      <strong>{result.kind === "directory" ? t("폴더") : formatBytes(result.logicalBytes)}</strong>
                      <time dateTime={formatDateTimeAttribute(result.modifiedAtUnixMs)}>{formatDate(result.modifiedAtUnixMs)}</time>
                      <button
                        type="button"
                        aria-label={t("{{name}} 폴더에서 표시", { name: result.name })}
                        onClick={(event) => {
                          event.stopPropagation();
                          void showInFolder(result.path, result.name);
                        }}
                      >
                        <HardDrive size={13} aria-hidden="true" />
                        {t("위치 표시")}
                      </button>
                    </div>
                  </article>
                );
              })}
            </div>
          )}

          {inspectionMessage ? <p className="document-inspection-status" role="status" aria-live="polite">{inspectionMessage}</p> : null}
        </section>
      ) : null}

      {!catalog && state !== "scanning" ? (
        <section className="document-capability-note file-capability-note">
          <div><Database size={18} aria-hidden="true" /><strong>{t("현재 검색 방식")}</strong></div>
          <p>{t("Windows 드라이브 전체를 찾을 때는 가능한 경우 빠른 방식으로 파일 목록을 읽습니다. 권한이 없거나 특정 폴더를 고르면 일반 방식으로 안전하게 확인합니다.")}</p>
        </section>
      ) : null}
    </div>
  );
}

function IndexMetric({ label, value }: { label: string; value: string }) {
  return <span><small>{label}</small><strong>{value}</strong></span>;
}

function kindLabel(kind: FileCatalogEntryKind): MessageKey {
  if (kind === "directory") return "폴더";
  if (kind === "symlink") return "바로가기";
  if (kind === "other") return "기타";
  return "파일";
}

function providerLabel(provider: FileCatalogStatus["provider"]): MessageKey {
  return provider === "windowsNtfs" ? "Windows 빠른 읽기" : "일반 폴더 확인";
}

function refreshModeLabel(mode: FileCatalogStatus["refreshMode"]): MessageKey {
  return mode === "incremental" ? "바뀐 항목만 확인" : "전체 다시 확인";
}

function pathsEqual(left: string, right: string): boolean {
  const normalize = (value: string) =>
    value.replace(/\//g, "\\").replace(/\\+$/, "").toLocaleLowerCase("en-US");
  return normalize(left) === normalize(right);
}

type Translator = ReturnType<typeof useLanguage>["t"];

function normalizeError(reason: unknown, fallback: string, t: Translator): string {
  if (reason instanceof Error) return friendlySearchError(reason.message, t);
  if (typeof reason === "string") return friendlySearchError(reason, t);
  return fallback;
}

function friendlySearchError(message: string, t: Translator): string {
  const detail = message
    .replace(/^invalid file search query:\s*/i, "")
    .replace(/^검색 조건이 올바르지 않습니다:\s*/, "")
    .trim();
  if (detail === "glob needs a literal run of at least three characters") {
    return t("이름 모양 검색에는 * 또는 ?를 제외한 글자가 3자 이상 필요합니다. 예: glob:report-*.pdf");
  }
  if (detail.includes("place OR between two")) {
    return t("OR의 앞과 뒤에 각각 찾을 이름이나 위치를 입력하세요.");
  }
  if (detail.includes("OR branch needs")) {
    return t("OR로 나눈 각 조건에는 3자 이상의 이름이나 위치가 필요합니다.");
  }
  if (detail.includes("parentheses are not supported")) {
    return t("괄호는 사용할 수 없습니다. 두 조건 중 하나를 찾으려면 사이에 OR를 넣으세요.");
  }
  if (detail.includes("close the quoted search phrase")) {
    return t("큰따옴표로 묶은 문구의 끝에 닫는 큰따옴표를 넣으세요.");
  }
  if (detail.includes("ext filter") || detail.includes("extensions may contain")) {
    return t("파일 종류에는 pdf, jpg처럼 영문과 숫자만 입력하세요.");
  }
  if (detail.includes("size")) {
    return t("파일 크기 조건을 확인하세요. 예: size:>100mb");
  }
  if (detail.includes("date") || detail.includes("after") || detail.includes("before")) {
    return t("날짜는 2026-01-31처럼 연도-월-일 순서로 입력하세요.");
  }
  if (message !== detail) {
    return t("검색 조건을 확인하세요. 자세한 예시는 ‘검색을 더 정확하게 하는 법’에서 볼 수 있습니다.");
  }
  return message;
}
