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
  { value: "relevance", label: "관련도순" },
  { value: "name", label: "이름순" },
  { value: "largest", label: "큰 용량순" },
  { value: "modified", label: "최근 수정순" },
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
            <p className="eyebrow">LOCAL FILE CATALOG</p>
            <h2 id="file-search-title">이름·경로·조건을 한 줄에서 찾으세요</h2>
            <p>일반 단어는 모두 포함하며 OR로 이름·경로·파일명 패턴을 나눠 찾습니다. 따옴표는 문구, 앞의 -는 제외 조건입니다.</p>
          </div>
          <span className="document-local-badge">
            <ShieldCheck size={14} aria-hidden="true" />
            로컬 전용
          </span>
        </div>

        <div className="document-query file-query" role="search">
          <Search size={20} aria-hidden="true" />
          <label htmlFor="file-query-input" className="sr-only">
            파일명 또는 경로 검색어
          </label>
          <input
            id="file-query-input"
            type="search"
            value={query}
            disabled={!canSearch}
            maxLength={256}
            autoComplete="off"
            aria-describedby="file-query-syntax"
            placeholder={
              scopeChanged
                ? "선택한 폴더의 카탈로그를 먼저 업데이트하세요"
                : stale
                  ? "휴지통 이동 결과를 반영하도록 카탈로그를 업데이트하세요"
                : catalog
                  ? "예: invoice OR receipt ext:pdf -draft"
                  : "먼저 드라이브 또는 폴더 카탈로그를 만드세요"
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
              ? "검색 중"
              : searchReport
                ? `${formatCount(searchReport.results.length)}개`
                : "즉시 검색"}
          </span>
        </div>

        <div className="file-query-syntax" id="file-query-syntax" role="note">
          <span>한 줄 조건</span>
          <code>OR</code>
          <code>glob:report-*.pdf</code>
          <code>ext:pdf,jpg</code>
          <code>type:file</code>
          <code>path:&quot;보고서&quot;</code>
          <code>size:&gt;100mb</code>
          <code>after:2026-01-01</code>
          <code>before:2027-01-01</code>
          <code>-draft</code>
        </div>

        <div className="document-filter-row file-filter-row">
          <div className="segmented-control" role="group" aria-label="항목 종류 필터">
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
            <span>확장자</span>
            <input
              type="text"
              value={extensions}
              disabled={!catalog}
              placeholder="pdf, jpg, rs"
              aria-label="확장자 필터"
              onChange={(event) => setExtensions(event.currentTarget.value)}
            />
          </label>
          <label className="file-filter-field">
            <span>크기</span>
            <select
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
            <span>정렬</span>
            <select
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
            <strong>선택한 폴더와 현재 파일 카탈로그의 범위가 다릅니다</strong>
            <p>이전 범위의 결과를 섞지 않습니다. 새 범위로 카탈로그를 갱신하세요.</p>
          </div>
          <button type="button" disabled={blocked || state === "scanning"} onClick={onStartCatalog}>
            새 범위 색인
          </button>
        </section>
      ) : null}

      {stale && !scopeChanged ? (
        <section className="document-scope-warning" role="status">
          <AlertTriangle size={18} aria-hidden="true" />
          <div>
            <strong>앱에서 이동한 파일이 현재 카탈로그에 남아 있습니다</strong>
            <p>오래된 경로를 결과로 보여주지 않도록 검색을 잠갔습니다. 카탈로그를 한 번 업데이트하세요.</p>
          </div>
          <button type="button" disabled={blocked || state === "scanning"} onClick={onStartCatalog}>
            지금 업데이트
          </button>
        </section>
      ) : null}

      <section className="document-index-console" aria-label="파일 카탈로그 상태">
        <div className="document-index-console__status">
          <span aria-hidden="true"><Database size={19} /></span>
          <div>
            <strong>
              {state === "scanning"
                ? "파일 이름과 경로를 수집하고 있습니다"
                : catalog
                  ? `${formatCount(catalog.indexedEntries)}개 항목을 바로 검색할 수 있습니다`
                  : "아직 만들어진 파일 카탈로그가 없습니다"}
            </strong>
            <p title={catalog?.root ?? effectiveRoot ?? undefined}>
              {state === "scanning"
                ? progress?.message ?? "폴더 항목을 확인하고 있습니다"
                : catalog
                  ? `${formatDate(catalog.completedAtUnixMs)} 갱신 · ${providerLabel(catalog.provider)} · ${refreshModeLabel(catalog.refreshMode)} · 파일 ${formatCount(catalog.indexedFiles)}개 · 폴더 ${formatCount(catalog.indexedDirectories)}개 · ${formatDuration(catalog.durationMs)}`
                  : `${effectiveRoot ?? "검색 범위 미선택"} · 최초 한 번 이름과 경로를 수집합니다.`}
            </p>
          </div>
        </div>

        {state === "scanning" ? (
          <button className="document-cancel-button" type="button" onClick={onCancelCatalog}>
            <X size={15} aria-hidden="true" />
            수집 취소
          </button>
        ) : (
          <div className="document-index-console__actions file-catalog-actions">
            <button type="button" disabled={blocked} onClick={onPickFolder}>
              <FolderOpen size={15} aria-hidden="true" />
              범위 선택
            </button>
            <button
              className={!catalog ? "primary-button" : ""}
              type="button"
              disabled={blocked || !effectiveRoot}
              onClick={onStartCatalog}
            >
              <RefreshCw size={15} aria-hidden="true" />
              {catalog && !scopeChanged ? "카탈로그 업데이트" : "카탈로그 만들기"}
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
                {confirmClear ? "한 번 더 눌러 비우기" : "카탈로그 비우기"}
              </button>
            ) : null}
          </div>
        )}

        {state === "scanning" && showBuildProgress ? (
          <div className="document-index-progress" role="status" aria-live="polite">
            <IndexMetric label="확인한 항목" value={`${formatCount(progress?.scannedEntries ?? 0)}개`} />
            <IndexMetric label="수집 완료" value={`${formatCount(progress?.indexedEntries ?? 0)}개`} />
            <IndexMetric label="파일" value={`${formatCount(progress?.indexedFiles ?? 0)}개`} />
            <IndexMetric label="폴더" value={`${formatCount(progress?.indexedDirectories ?? 0)}개`} />
          </div>
        ) : null}
      </section>

      {error ? (
        <section className="document-error" role="alert">
          <AlertTriangle size={18} aria-hidden="true" />
          <div><strong>파일 카탈로그 작업을 완료하지 못했습니다</strong><p>{error}</p></div>
        </section>
      ) : null}

      {lastBuild && catalog?.completedAtUnixMs === lastBuild.completedAtUnixMs ? (
        <>
          <section className="document-build-evidence" aria-label="최근 파일 카탈로그 근거">
            <span><strong>{formatCount(lastBuild.indexedFiles)}</strong>파일</span>
            <span><strong>{formatCount(lastBuild.indexedDirectories)}</strong>폴더</span>
            <span><strong>{formatCount(lastBuild.removedEntries)}</strong>삭제 반영</span>
            <span><strong>{formatCount(lastBuild.unreadableEntries)}</strong>읽기 실패</span>
            <p>본문은 읽지 않으며 파일명·경로·크기·수정 시각만 저장합니다. 검색 결과는 삭제 추천이 아닙니다.</p>
          </section>
          {lastBuild.entryLimitReached ? (
            <section className="document-scope-warning" role="status">
              <AlertTriangle size={18} aria-hidden="true" />
              <div><strong>카탈로그 항목 상한에 도달했습니다</strong><p>검색 결과가 완전하지 않을 수 있습니다. 더 작은 폴더 범위를 선택하세요.</p></div>
            </section>
          ) : null}
          {lastBuild.issues.length > 0 ? (
            <details className="document-index-issues">
              <summary>수집 알림 {formatCount(lastBuild.issues.length)}개</summary>
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
          <div><strong>파일을 검색하지 못했습니다</strong><p>{searchError}</p></div>
        </section>
      ) : null}

      {searchReport ? (
        <section className="document-results file-results" aria-labelledby="file-results-title">
          <header className="document-results__header">
            <div>
              <p className="eyebrow">MATCHED LOCALLY</p>
              <h2 id="file-results-title">“{searchReport.query}” 검색 결과</h2>
              <p>
                {formatCount(searchReport.indexedEntries)}개 항목을 {formatDuration(searchReport.searchDurationMs)}에 조회했습니다.
                {searchReport.resultsTruncated ? " 상위 100개만 표시합니다." : ""}
              </p>
            </div>
            <span>{sortOptions.find((option) => option.value === sort)?.label}</span>
          </header>

          {searchReport.results.length === 0 ? (
            <div className="document-results-empty">
              <Search size={24} aria-hidden="true" />
              <strong>일치하는 파일이나 폴더가 없습니다</strong>
              <p>검색어 또는 한 줄 조건을 줄이거나 아래 필터를 바꿔보세요.</p>
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
                        {result.matchSource === "path" ? <span>경로 일치</span> : null}
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
          <p>Windows의 NTFS 드라이브 전체 범위는 읽기 권한이 있으면 MFT와 USN 변경분을 사용합니다. 권한이 없거나 선택 폴더·비NTFS 범위이면 기존의 안전한 공용 순회로 자동 전환합니다.</p>
        </section>
      ) : null}
    </div>
  );
}

function IndexMetric({ label, value }: { label: string; value: string }) {
  return <span><small>{label}</small><strong>{value}</strong></span>;
}

function kindLabel(kind: FileCatalogEntryKind): string {
  if (kind === "directory") return "FOLDER";
  if (kind === "symlink") return "LINK";
  if (kind === "other") return "OTHER";
  return "FILE";
}

function providerLabel(provider: FileCatalogStatus["provider"]): string {
  return provider === "windowsNtfs" ? "NTFS MFT" : "공용 순회";
}

function refreshModeLabel(mode: FileCatalogStatus["refreshMode"]): string {
  return mode === "incremental" ? "USN 변경분" : "전체 갱신";
}

function pathsEqual(left: string, right: string): boolean {
  const normalize = (value: string) =>
    value.replace(/\//g, "\\").replace(/\\+$/, "").toLocaleLowerCase("en-US");
  return normalize(left) === normalize(right);
}

function normalizeError(reason: unknown): string {
  if (reason instanceof Error) return reason.message;
  if (typeof reason === "string") return reason;
  return "알 수 없는 오류가 발생했습니다";
}
