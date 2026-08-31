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

const kindFilters: Array<{ id: KindFilter; label: string }> = [
  { id: "all", label: "전체" },
  { id: "file", label: "파일" },
  { id: "directory", label: "폴더" },
];

const sizeFilters = [
  { value: "0", label: "모든 크기", minBytes: null },
  { value: "100", label: "100 MB 이상", minBytes: 100 * 1024 * 1024 },
  { value: "1024", label: "1 GB 이상", minBytes: 1024 * 1024 * 1024 },
  { value: "10240", label: "10 GB 이상", minBytes: 10 * 1024 * 1024 * 1024 },
] as const;

const sortOptions: Array<{ value: FileCatalogSort; label: string }> = [
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
          setSearchError(normalizeError(reason));
          setSearchState("error");
        });
    }, 140);

    return () => window.clearTimeout(timer);
  }, [canSearch, kind, minBytes, normalizedExtensions, query, sort]);

  async function openEntry(path: string, name: string, entryKind: FileCatalogEntryKind) {
    setInspectionMessage(null);
    try {
      const outcome = await inspectFile(path);
      setInspectionMessage(
        entryKind === "directory"
          ? `${name} 폴더를 열었습니다.`
          : outcome === "opened"
            ? `${name} 파일을 기본 앱으로 열었습니다.`
            : `${name}은 실행 가능한 형식이라 폴더에서 위치만 표시했습니다.`,
      );
    } catch (reason) {
      setInspectionMessage(`${name}을 열지 못했습니다: ${normalizeError(reason)}`);
    }
  }

  async function showInFolder(path: string, name: string) {
    setInspectionMessage(null);
    try {
      await revealPath(path);
      setInspectionMessage(`${name} 위치를 파일 탐색기에 표시했습니다.`);
    } catch (reason) {
      setInspectionMessage(`${name} 위치를 표시하지 못했습니다: ${normalizeError(reason)}`);
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
            <p className="eyebrow">내 파일에서 찾기</p>
            <h2 id="file-search-title">파일 이름이나 폴더 위치로 찾으세요</h2>
            <p>찾고 싶은 이름을 입력하세요. 파일을 열지 않고 저장된 이름과 위치만 확인합니다.</p>
          </div>
          <span className="document-local-badge">
            <ShieldCheck size={14} aria-hidden="true" />
            이 기기 안에서만 검색
          </span>
        </div>

        <div className="document-query file-query" role="search">
          <Search size={20} aria-hidden="true" />
          <label htmlFor="file-query-input" className="sr-only">
            파일명 또는 경로 검색어
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
                ? "선택한 위치의 파일 목록을 먼저 새로고침하세요…"
                : stale
                  ? "옮긴 파일을 반영하도록 파일 목록을 새로고침하세요…"
                : catalog
                  ? "예: 8월 보고서…"
                  : "먼저 찾을 위치의 파일 목록을 만드세요…"
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
              ? "검색 중…"
              : searchReport
                ? `${formatCount(searchReport.results.length)}개`
                : "즉시 검색"}
          </span>
        </div>

        <details className="file-query-help">
          <summary id="file-query-syntax">검색을 더 정확하게 하는 법</summary>
          <div className="file-query-syntax" role="note">
            <span><small>둘 중 하나</small><code translate="no">OR</code></span>
            <span><small>이름 모양</small><code translate="no">glob:report-*.pdf</code></span>
            <span><small>파일 종류</small><code translate="no">ext:pdf,jpg</code></span>
            <span><small>파일만</small><code translate="no">type:file</code></span>
            <span><small>폴더 위치</small><code translate="no">path:&quot;보고서&quot;</code></span>
            <span><small>100 MB보다 큼</small><code translate="no">size:&gt;100mb</code></span>
            <span><small>이 날짜 뒤</small><code translate="no">after:2026-01-01</code></span>
            <span><small>이 날짜 앞</small><code translate="no">before:2027-01-01</code></span>
            <span><small>이 단어 빼기</small><code translate="no">-draft</code></span>
          </div>
        </details>

        <div className="document-filter-row file-filter-row">
          <div className="segmented-control" role="group" aria-label="찾을 대상 선택">
            {kindFilters.map((item) => (
              <button
                type="button"
                className={kind === item.id ? "is-active" : ""}
                aria-pressed={kind === item.id}
                key={item.id}
                onClick={() => setKind(item.id)}
              >
                {item.label}
              </button>
            ))}
          </div>
          <label className="file-filter-field">
            <span>파일 종류</span>
            <input
              name="file-extensions"
              type="text"
              value={extensions}
              disabled={!catalog}
              autoComplete="off"
              spellCheck={false}
              placeholder="pdf, jpg, rs…"
              aria-label="파일 종류 필터"
              onChange={(event) => setExtensions(event.currentTarget.value)}
            />
          </label>
          <label className="file-filter-field">
            <span>파일 크기</span>
            <select
              name="file-size-filter"
              value={sizeFilter}
              disabled={!catalog}
              aria-label="최소 파일 크기"
              onChange={(event) => setSizeFilter(event.currentTarget.value)}
            >
              {sizeFilters.map((option) => (
                <option value={option.value} key={option.value}>{option.label}</option>
              ))}
            </select>
          </label>
          <label className="file-filter-field">
            <span>정렬 기준</span>
            <select
              name="file-sort"
              value={sort}
              disabled={!catalog}
              aria-label="검색 결과 정렬"
              onChange={(event) => setSort(event.currentTarget.value as FileCatalogSort)}
            >
              {sortOptions.map((option) => (
                <option value={option.value} key={option.value}>{option.label}</option>
              ))}
            </select>
          </label>
        </div>
      </section>

      {scopeChanged ? (
        <section className="document-scope-warning" role="status">
          <AlertTriangle size={18} aria-hidden="true" />
          <div>
            <strong>선택한 위치가 지금 만든 파일 목록과 다릅니다</strong>
            <p>이전 위치의 결과와 섞이지 않도록 새 위치의 목록을 다시 만드세요.</p>
          </div>
          <button type="button" disabled={blocked || state === "scanning"} onClick={onStartCatalog}>
            새 위치 다시 읽기
          </button>
        </section>
      ) : null}

      {stale && !scopeChanged ? (
        <section className="document-scope-warning" role="status">
          <AlertTriangle size={18} aria-hidden="true" />
          <div>
            <strong>옮긴 파일이 검색 목록에 아직 남아 있습니다</strong>
            <p>예전 위치를 보여주지 않도록 검색을 잠시 멈췄습니다. 파일 목록을 새로고침하세요.</p>
          </div>
          <button type="button" disabled={blocked || state === "scanning"} onClick={onStartCatalog}>
            지금 새로고침
          </button>
        </section>
      ) : null}

      <section className="document-index-console" aria-label="검색용 파일 목록 상태">
        <div className="document-index-console__status">
          <span aria-hidden="true"><Database size={19} /></span>
          <div>
            <strong>
              {state === "scanning"
                ? "파일 이름과 위치를 읽고 있습니다…"
                : catalog
                  ? `파일과 폴더 ${formatCount(catalog.indexedEntries)}개를 바로 찾을 수 있습니다`
                  : "아직 검색할 파일 목록이 없습니다"}
            </strong>
            <p title={catalog?.root ?? effectiveRoot ?? undefined}>
              {state === "scanning"
                ? progress?.message ?? "폴더 안의 파일을 확인하고 있습니다…"
                : catalog
                  ? `${formatDate(catalog.completedAtUnixMs)} 새로고침 · ${providerLabel(catalog.provider)} · ${refreshModeLabel(catalog.refreshMode)} · 파일 ${formatCount(catalog.indexedFiles)}개 · 폴더 ${formatCount(catalog.indexedDirectories)}개 · ${formatDuration(catalog.durationMs)}`
                  : `${effectiveRoot ?? "찾을 위치를 선택하지 않음"} · 처음 한 번 이름과 위치를 읽습니다.`}
            </p>
          </div>
        </div>

        {state === "scanning" ? (
          <button className="document-cancel-button" type="button" onClick={onCancelCatalog}>
            <X size={15} aria-hidden="true" />
            읽기 취소
          </button>
        ) : (
          <div className="document-index-console__actions file-catalog-actions">
            <button type="button" disabled={blocked} onClick={onPickFolder}>
              <FolderOpen size={15} aria-hidden="true" />
              찾을 위치 선택
            </button>
            <button
              className={!catalog ? "primary-button" : ""}
              type="button"
              disabled={blocked || !effectiveRoot}
              onClick={onStartCatalog}
            >
              <RefreshCw size={15} aria-hidden="true" />
              {catalog && !scopeChanged ? "파일 목록 새로고침" : "파일 목록 만들기"}
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
                {confirmClear ? "한 번 더 눌러 지우기" : "파일 목록 지우기"}
              </button>
            ) : null}
          </div>
        )}

        {state === "scanning" && showBuildProgress ? (
          <div className="document-index-progress" role="status" aria-live="polite">
            <IndexMetric label="확인함" value={`${formatCount(progress?.scannedEntries ?? 0)}개`} />
            <IndexMetric label="목록에 넣음" value={`${formatCount(progress?.indexedEntries ?? 0)}개`} />
            <IndexMetric label="파일" value={`${formatCount(progress?.indexedFiles ?? 0)}개`} />
            <IndexMetric label="폴더" value={`${formatCount(progress?.indexedDirectories ?? 0)}개`} />
          </div>
        ) : null}
      </section>

      {error ? (
        <section className="document-error" role="alert">
          <AlertTriangle size={18} aria-hidden="true" />
          <div><strong>파일 목록을 만들지 못했습니다</strong><p>{friendlySearchError(error)}</p></div>
        </section>
      ) : null}

      {lastBuild && catalog?.completedAtUnixMs === lastBuild.completedAtUnixMs ? (
        <>
          <section className="document-build-evidence" aria-label="최근 파일 목록 만들기 결과">
            <span><strong>{formatCount(lastBuild.indexedFiles)}</strong>파일</span>
            <span><strong>{formatCount(lastBuild.indexedDirectories)}</strong>폴더</span>
            <span><strong>{formatCount(lastBuild.removedEntries)}</strong>삭제 반영</span>
            <span><strong>{formatCount(lastBuild.unreadableEntries)}</strong>읽기 실패</span>
            <p>파일 내용은 읽지 않고 이름·위치·크기·바뀐 시각만 저장합니다. 찾은 결과가 곧 지워도 되는 파일이라는 뜻은 아닙니다.</p>
          </section>
          {lastBuild.entryLimitReached ? (
            <section className="document-scope-warning" role="status">
              <AlertTriangle size={18} aria-hidden="true" />
              <div><strong>한 번에 담을 수 있는 파일 수를 넘었습니다</strong><p>일부 파일이 빠졌을 수 있습니다. 더 작은 폴더를 선택하세요.</p></div>
            </section>
          ) : null}
          {lastBuild.issues.length > 0 ? (
            <details className="document-index-issues">
              <summary>확인하지 못한 위치 {formatCount(lastBuild.issues.length)}개</summary>
              <ul>
                {lastBuild.issues.slice(0, 20).map((issue, issueIndex) => (
                  <li key={`${issue.path ?? "unknown"}-${issueIndex}`}>
                    <strong title={issue.path ?? undefined}>{issue.path ?? "경로 정보 없음"}</strong>
                    <span>{issue.message}</span>
                  </li>
                ))}
              </ul>
              {lastBuild.issues.length > 20 ? <p>화면에는 처음 20개 사유만 표시합니다.</p> : null}
            </details>
          ) : null}
        </>
      ) : null}

      {searchError ? (
        <section className="document-error" role="alert">
          <AlertTriangle size={18} aria-hidden="true" />
          <div><strong>검색 조건을 이해하지 못했습니다</strong><p>{friendlySearchError(searchError)}</p></div>
        </section>
      ) : null}

      {searchReport ? (
        <section className="document-results file-results" aria-labelledby="file-results-title">
          <header className="document-results__header">
            <div>
              <p className="eyebrow">이 기기에서 찾은 결과</p>
              <h2 id="file-results-title">“{searchReport.query}” 검색 결과</h2>
              <p>
                파일과 폴더 {formatCount(searchReport.indexedEntries)}개를 {formatDuration(searchReport.searchDurationMs)}에 확인했습니다.
                {searchReport.resultsTruncated ? " 처음 100개만 보여줍니다." : ""}
              </p>
            </div>
            <span>{sortOptions.find((option) => option.value === sort)?.label}</span>
          </header>

          {searchReport.results.length === 0 ? (
            <div className="document-results-empty">
              <Search size={24} aria-hidden="true" />
              <strong>일치하는 파일이나 폴더가 없습니다</strong>
              <p>검색어를 짧게 하거나 파일 종류와 크기 조건을 바꿔보세요.</p>
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
                    aria-label={`${result.name}, 더블클릭하거나 Enter 키를 눌러 열기`}
                    onDoubleClick={() => void openEntry(result.path, result.name, result.kind)}
                    onKeyDown={(event) => {
                      if (event.key === "Enter") void openEntry(result.path, result.name, result.kind);
                    }}
                  >
                    <span className="document-result__icon" aria-hidden="true"><EntryIcon size={18} /></span>
                    <div className="document-result__body">
                      <div className="document-result__identity">
                        <strong title={result.name}>{result.name}</strong>
                        <span>{kindLabel(result.kind)}</span>
                        {result.matchSource === "path" ? <span>폴더 위치 일치</span> : null}
                      </div>
                      <span className="document-result__path" title={result.path}>{result.parent}</span>
                    </div>
                    <div className="document-result__meta">
                      <strong>{result.kind === "directory" ? "폴더" : formatBytes(result.logicalBytes)}</strong>
                      <time dateTime={formatDateTimeAttribute(result.modifiedAtUnixMs)}>{formatDate(result.modifiedAtUnixMs)}</time>
                      <button
                        type="button"
                        aria-label={`${result.name} 폴더에서 표시`}
                        onClick={(event) => {
                          event.stopPropagation();
                          void showInFolder(result.path, result.name);
                        }}
                      >
                        <HardDrive size={13} aria-hidden="true" />
                        위치 표시
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
          <div><Database size={18} aria-hidden="true" /><strong>현재 검색 방식</strong></div>
          <p>Windows 드라이브 전체를 찾을 때는 가능한 경우 빠른 방식으로 파일 목록을 읽습니다. 권한이 없거나 특정 폴더를 고르면 일반 방식으로 안전하게 확인합니다.</p>
        </section>
      ) : null}
    </div>
  );
}

function IndexMetric({ label, value }: { label: string; value: string }) {
  return <span><small>{label}</small><strong>{value}</strong></span>;
}

function kindLabel(kind: FileCatalogEntryKind): string {
  if (kind === "directory") return "폴더";
  if (kind === "symlink") return "바로가기";
  if (kind === "other") return "기타";
  return "파일";
}

function providerLabel(provider: FileCatalogStatus["provider"]): string {
  return provider === "windowsNtfs" ? "Windows 빠른 읽기" : "일반 폴더 확인";
}

function refreshModeLabel(mode: FileCatalogStatus["refreshMode"]): string {
  return mode === "incremental" ? "바뀐 항목만 확인" : "전체 다시 확인";
}

function pathsEqual(left: string, right: string): boolean {
  const normalize = (value: string) =>
    value.replace(/\//g, "\\").replace(/\\+$/, "").toLocaleLowerCase("en-US");
  return normalize(left) === normalize(right);
}

function normalizeError(reason: unknown): string {
  if (reason instanceof Error) return friendlySearchError(reason.message);
  if (typeof reason === "string") return friendlySearchError(reason);
  return "알 수 없는 오류가 발생했습니다";
}

function friendlySearchError(message: string): string {
  const detail = message
    .replace(/^invalid file search query:\s*/i, "")
    .replace(/^검색 조건이 올바르지 않습니다:\s*/, "")
    .trim();
  if (detail === "glob needs a literal run of at least three characters") {
    return "이름 모양 검색에는 * 또는 ?를 제외한 글자가 3자 이상 필요합니다. 예: glob:report-*.pdf";
  }
  if (detail.includes("place OR between two")) {
    return "OR의 앞과 뒤에 각각 찾을 이름이나 위치를 입력하세요.";
  }
  if (detail.includes("OR branch needs")) {
    return "OR로 나눈 각 조건에는 3자 이상의 이름이나 위치가 필요합니다.";
  }
  if (detail.includes("parentheses are not supported")) {
    return "괄호는 사용할 수 없습니다. 두 조건 중 하나를 찾으려면 사이에 OR를 넣으세요.";
  }
  if (detail.includes("close the quoted search phrase")) {
    return "큰따옴표로 묶은 문구의 끝에 닫는 큰따옴표를 넣으세요.";
  }
  if (detail.includes("ext filter") || detail.includes("extensions may contain")) {
    return "파일 종류에는 pdf, jpg처럼 영문과 숫자만 입력하세요.";
  }
  if (detail.includes("size")) {
    return "파일 크기 조건을 확인하세요. 예: size:>100mb";
  }
  if (detail.includes("date") || detail.includes("after") || detail.includes("before")) {
    return "날짜는 2026-01-31처럼 연도-월-일 순서로 입력하세요.";
  }
  if (message !== detail) {
    return "검색 조건을 확인하세요. 자세한 예시는 ‘검색을 더 정확하게 하는 법’에서 볼 수 있습니다.";
  }
  return message;
}
