import {
  AlertTriangle,
  Database,
  FileSearch,
  FileText,
  FolderOpen,
  HardDrive,
  LoaderCircle,
  RefreshCw,
  Search,
  ShieldCheck,
  X,
} from "lucide-react";
import { useEffect, useMemo, useState, type FormEvent } from "react";
import { inspectFile, revealPath, searchDocuments } from "../lib/bridge";
import {
  fileParent,
  formatBytes,
  formatCount,
  formatDate,
  formatDateTimeAttribute,
  formatDuration,
} from "../lib/format";
import type {
  DocumentFormat,
  DocumentIndexProgress,
  DocumentIndexReport,
  DocumentIndexStatus,
  DocumentSearchReport,
  ScanUiState,
} from "../types";

interface DocumentSearchViewProps {
  selectedRoot: string | null;
  index: DocumentIndexStatus | null;
  lastBuild: DocumentIndexReport | null;
  progress: DocumentIndexProgress | null;
  state: ScanUiState;
  error: string | null;
  blocked: boolean;
  onPickFolder: () => void;
  onStartIndex: () => void;
  onCancelIndex: () => void;
}

type SearchState = "idle" | "searching" | "success" | "error";
type DocumentFilter = "all" | "text" | "office" | "pdf";

const textExtensions = [
  "txt",
  "md",
  "markdown",
  "rst",
  "log",
  "csv",
  "tsv",
  "json",
  "jsonl",
  "xml",
  "yaml",
  "yml",
  "toml",
  "ini",
  "cfg",
  "conf",
  "sql",
  "html",
  "htm",
  "css",
  "js",
  "jsx",
  "ts",
  "tsx",
  "py",
  "rs",
  "go",
  "java",
  "c",
  "h",
  "cpp",
  "hpp",
  "cs",
  "swift",
  "kt",
  "kts",
  "sh",
  "ps1",
  "bat",
  "cmd",
];

const extensionFilters: Array<{
  id: DocumentFilter;
  label: string;
  extensions: string[];
}> = [
  { id: "all", label: "전체", extensions: [] },
  { id: "text", label: "텍스트·코드", extensions: textExtensions },
  { id: "office", label: "Office·HWPX", extensions: ["docx", "xlsx", "pptx", "hwpx"] },
  { id: "pdf", label: "PDF", extensions: ["pdf"] },
];

const formatLabels: Record<DocumentFormat, string> = {
  plainText: "TEXT",
  pdf: "PDF",
  word: "WORD",
  spreadsheet: "SHEET",
  presentation: "SLIDE",
  hwpx: "HWPX",
};

export function DocumentSearchView({
  selectedRoot,
  index,
  lastBuild,
  progress,
  state,
  error,
  blocked,
  onPickFolder,
  onStartIndex,
  onCancelIndex,
}: DocumentSearchViewProps) {
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<DocumentFilter>("all");
  const [searchState, setSearchState] = useState<SearchState>("idle");
  const [searchReport, setSearchReport] = useState<DocumentSearchReport | null>(null);
  const [searchError, setSearchError] = useState<string | null>(null);
  const [inspectionMessage, setInspectionMessage] = useState<string | null>(null);
  const [showIndexProgress, setShowIndexProgress] = useState(false);
  const scopeChanged = Boolean(
    selectedRoot && index && !pathsEqual(selectedRoot, index.root),
  );
  const effectiveRoot = selectedRoot ?? index?.root ?? null;
  const canSearch = Boolean(index && !scopeChanged && state !== "scanning" && !blocked);
  const activeFilter = useMemo(
    () => extensionFilters.find((item) => item.id === filter) ?? extensionFilters[0],
    [filter],
  );

  useEffect(() => {
    if (state !== "scanning") {
      setShowIndexProgress(false);
      return;
    }
    const timer = window.setTimeout(() => setShowIndexProgress(true), 300);
    return () => window.clearTimeout(timer);
  }, [state]);

  useEffect(() => {
    setSearchReport(null);
    setSearchError(null);
    setSearchState("idle");
  }, [filter, index?.completedAtUnixMs, scopeChanged]);

  async function runSearch(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const normalized = query.trim();
    if (!canSearch || !normalized || searchState === "searching") return;

    setSearchState("searching");
    setSearchError(null);
    setInspectionMessage(null);
    try {
      const report = await searchDocuments({
        query: normalized,
        extensions: activeFilter.extensions,
        maxResults: 100,
      });
      setSearchReport(report);
      setSearchState("success");
    } catch (reason) {
      setSearchError(normalizeError(reason));
      setSearchState("error");
    }
  }

  async function openDocument(path: string, name: string) {
    setInspectionMessage(null);
    try {
      const outcome = await inspectFile(path);
      setInspectionMessage(
        outcome === "opened"
          ? `${name} 문서를 기본 앱으로 열었습니다.`
          : `${name}은 실행 가능한 형식이라 폴더에서 위치만 표시했습니다.`,
      );
    } catch (reason) {
      setInspectionMessage(`${name} 문서를 열지 못했습니다: ${normalizeError(reason)}`);
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

  return (
    <div className="view-stack document-search-view">
      <section className="document-search-stage" aria-labelledby="document-search-title">
        <div className="document-search-stage__heading">
          <span className="document-search-stage__seal" aria-hidden="true">
            <FileSearch size={22} />
          </span>
          <div>
            <p className="eyebrow">LOCAL FULL-TEXT INDEX</p>
            <h2 id="document-search-title">문서 안의 단어와 문장을 찾습니다</h2>
            <p>
              파일은 이 컴퓨터에서만 읽고 색인합니다. 검색 내용이나 문서 본문을 외부로 보내지 않습니다.
            </p>
          </div>
          <span className="document-local-badge">
            <ShieldCheck size={14} aria-hidden="true" />
            로컬 전용
          </span>
        </div>

        <form className="document-query" role="search" onSubmit={runSearch}>
          <Search size={20} aria-hidden="true" />
          <label htmlFor="document-query-input" className="sr-only">
            문서 내용 검색어
          </label>
          <input
            id="document-query-input"
            type="search"
            name="document-query"
            value={query}
            disabled={!canSearch}
            maxLength={256}
            autoComplete="off"
            placeholder={
              scopeChanged
                ? "선택한 폴더의 색인을 먼저 업데이트하세요"
                : index
                  ? "예: 계약 변경, 오류 코드, 회의 결정"
                  : "먼저 검색할 폴더의 색인을 만드세요"
            }
            onChange={(event) => setQuery(event.currentTarget.value)}
          />
          <button
            className="document-search-button"
            type="submit"
            disabled={!canSearch || !query.trim() || searchState === "searching"}
          >
            {searchState === "searching" ? (
              <LoaderCircle className="is-spinning" size={16} aria-hidden="true" />
            ) : (
              <Search size={16} aria-hidden="true" />
            )}
            {searchState === "searching" ? "검색 중" : "내용 검색"}
          </button>
        </form>

        <div className="document-filter-row">
          <div className="segmented-control" role="group" aria-label="문서 형식 필터">
            {extensionFilters.map((item) => (
              <button
                type="button"
                className={filter === item.id ? "is-active" : ""}
                aria-pressed={filter === item.id}
                key={item.id}
                onClick={() => setFilter(item.id)}
              >
                {item.label}
              </button>
            ))}
          </div>
          <span title={effectiveRoot ?? undefined}>
            <FolderOpen size={13} aria-hidden="true" />
            {effectiveRoot ?? "검색 범위가 선택되지 않았습니다"}
          </span>
        </div>
      </section>

      {scopeChanged ? (
        <section className="document-scope-warning" role="status">
          <AlertTriangle size={18} aria-hidden="true" />
          <div>
            <strong>선택한 폴더와 현재 색인의 범위가 다릅니다</strong>
            <p>이전 폴더의 결과를 섞지 않습니다. 새 범위를 색인한 뒤 검색할 수 있습니다.</p>
          </div>
          <button type="button" disabled={blocked || state === "scanning"} onClick={onStartIndex}>
            새 범위 색인
          </button>
        </section>
      ) : null}

      <section className="document-index-console" aria-label="문서 색인 상태">
        <div className="document-index-console__status">
          <span aria-hidden="true">
            <Database size={19} />
          </span>
          <div>
            <strong>
              {state === "scanning"
                ? "문서 색인을 갱신하고 있습니다"
                : index
                  ? `${formatCount(index.indexedDocuments)}개 문서를 검색할 수 있습니다`
                  : "아직 만들어진 문서 색인이 없습니다"}
            </strong>
            <p>
              {state === "scanning"
                ? progress?.message ?? "문서 파일을 확인하고 있습니다"
                : index
                  ? `${formatDate(index.completedAtUnixMs)} 갱신 · ${formatBytes(index.indexedBytes)} · ${formatDuration(index.durationMs)}`
                  : "최초 한 번 본문을 읽고, 다음부터 크기와 수정 시각이 바뀐 문서만 다시 처리합니다."}
            </p>
          </div>
        </div>

        {state === "scanning" ? (
          <button className="document-cancel-button" type="button" onClick={onCancelIndex}>
            <X size={15} aria-hidden="true" />
            색인 취소
          </button>
        ) : (
          <div className="document-index-console__actions">
            {!effectiveRoot ? (
              <button type="button" disabled={blocked} onClick={onPickFolder}>
                <FolderOpen size={15} aria-hidden="true" />
                폴더 선택
              </button>
            ) : null}
            <button
              className={!index ? "primary-button" : ""}
              type="button"
              disabled={blocked || !effectiveRoot}
              onClick={onStartIndex}
            >
              <RefreshCw size={15} aria-hidden="true" />
              {index && !scopeChanged ? "색인 업데이트" : "색인 만들기"}
            </button>
          </div>
        )}

        {state === "scanning" && showIndexProgress ? (
          <div className="document-index-progress" role="status" aria-live="polite">
            <IndexMetric label="확인한 파일" value={`${formatCount(progress?.scannedFiles ?? 0)}개`} />
            <IndexMetric label="문서 후보" value={`${formatCount(progress?.candidateDocuments ?? 0)}개`} />
            <IndexMetric label="색인 완료" value={`${formatCount(progress?.indexedDocuments ?? 0)}개`} />
            <IndexMetric label="캐시 재사용" value={`${formatCount(progress?.reusedDocuments ?? 0)}개`} />
          </div>
        ) : null}
      </section>

      {error ? (
        <section className="document-error" role="alert">
          <AlertTriangle size={18} aria-hidden="true" />
          <div>
            <strong>문서 색인을 완료하지 못했습니다</strong>
            <p>{error}</p>
          </div>
        </section>
      ) : null}

      {lastBuild && index?.completedAtUnixMs === lastBuild.completedAtUnixMs ? (
        <>
          <section className="document-build-evidence" aria-label="최근 색인 근거">
            <span>
              <strong>{formatCount(lastBuild.updatedDocuments)}</strong>
              새로 읽음
            </span>
            <span>
              <strong>{formatCount(lastBuild.reusedDocuments)}</strong>
              변경 없음
            </span>
            <span>
              <strong>{formatCount(lastBuild.skippedDocuments)}</strong>
              건너뜀
            </span>
            <span>
              <strong>{formatCount(lastBuild.unsupportedDocuments)}</strong>
              구형 HWP
            </span>
            <p>
              PDF는 텍스트 계층만 검색합니다. 이미지형 PDF와 암호 문서는 OCR이나 암호 해제를 시도하지 않습니다.
            </p>
          </section>
          {lastBuild.documentLimitReached ? (
            <section className="document-scope-warning" role="status">
              <AlertTriangle size={18} aria-hidden="true" />
              <div>
                <strong>문서 색인 상한에 도달했습니다</strong>
                <p>개별 문서 목록이 완전하지 않습니다. 더 작은 폴더를 선택해 다시 색인하세요.</p>
              </div>
            </section>
          ) : null}
          {lastBuild.issues.length > 0 ? (
            <details className="document-index-issues">
              <summary>
                읽지 못한 문서와 건너뜀 사유 {formatCount(lastBuild.issues.length)}개
              </summary>
              <ul>
                {lastBuild.issues.slice(0, 20).map((issue, issueIndex) => (
                  <li key={`${issue.path ?? "unknown"}-${issueIndex}`}>
                    <strong title={issue.path ?? undefined}>{issue.path ?? "경로 정보 없음"}</strong>
                    <span>{issue.message}</span>
                  </li>
                ))}
              </ul>
              {lastBuild.issues.length > 20 ? (
                <p>화면에는 처음 20개 사유만 표시합니다.</p>
              ) : null}
            </details>
          ) : null}
        </>
      ) : null}

      {searchError ? (
        <section className="document-error" role="alert">
          <AlertTriangle size={18} aria-hidden="true" />
          <div>
            <strong>문서를 검색하지 못했습니다</strong>
            <p>{searchError}</p>
          </div>
        </section>
      ) : null}

      {searchReport ? (
        <section className="document-results" aria-labelledby="document-results-title">
          <header className="document-results__header">
            <div>
              <p className="eyebrow">MATCHED LOCALLY</p>
              <h2 id="document-results-title">“{searchReport.query}” 검색 결과</h2>
              <p>
                {formatCount(searchReport.searchedDocuments)}개 문서에서 {formatCount(searchReport.totalMatches)}개를 찾았습니다.
                {searchReport.resultsTruncated ? " 상위 100개만 표시합니다." : ""}
              </p>
            </div>
            <span>{activeFilter.label}</span>
          </header>

          {searchReport.results.length === 0 ? (
            <div className="document-results-empty">
              <Search size={24} aria-hidden="true" />
              <strong>일치하는 문서가 없습니다</strong>
              <p>단어를 줄이거나 다른 문서 형식을 선택해 다시 검색해 보세요.</p>
            </div>
          ) : (
            <div className="document-result-list">
              {searchReport.results.map((result) => (
                <article
                  className="document-result"
                  tabIndex={0}
                  key={result.path}
                  aria-label={`${result.name}, 더블클릭하거나 Enter 키를 눌러 열기`}
                  onDoubleClick={() => void openDocument(result.path, result.name)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter") void openDocument(result.path, result.name);
                  }}
                >
                  <span className="document-result__icon" aria-hidden="true">
                    <FileText size={18} />
                  </span>
                  <div className="document-result__body">
                    <div className="document-result__identity">
                      <strong title={result.name}>{result.name}</strong>
                      <span>{formatLabels[result.format]}</span>
                    </div>
                    <p className="document-result__snippet">
                      {result.snippet.map((part, index) =>
                        part.highlighted ? (
                          <mark key={`${result.path}-snippet-${index}`}>{part.text}</mark>
                        ) : (
                          <span key={`${result.path}-snippet-${index}`}>{part.text}</span>
                        ),
                      )}
                    </p>
                    <span className="document-result__path" title={result.path}>
                      {fileParent(result.path)}
                    </span>
                  </div>
                  <div className="document-result__meta">
                    <strong>{formatBytes(result.logicalBytes)}</strong>
                    <time dateTime={formatDateTimeAttribute(result.modifiedAtUnixMs)}>
                      {formatDate(result.modifiedAtUnixMs)}
                    </time>
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
              ))}
            </div>
          )}

          {inspectionMessage ? (
            <p className="document-inspection-status" role="status" aria-live="polite">
              {inspectionMessage}
            </p>
          ) : null}
        </section>
      ) : null}

      {!index && state !== "scanning" ? (
        <section className="document-capability-note">
          <div>
            <FileText size={18} aria-hidden="true" />
            <strong>첫 버전 검색 범위</strong>
          </div>
          <p>
            TXT·Markdown·코드·CSV·JSON, DOCX·XLSX·PPTX·HWPX, 텍스트 PDF를 지원합니다. 구형 HWP와 스캔 PDF는 후속 전용 파서·OCR 범위입니다.
          </p>
        </section>
      ) : null}
    </div>
  );
}

function IndexMetric({ label, value }: { label: string; value: string }) {
  return (
    <span>
      <small>{label}</small>
      <strong>{value}</strong>
    </span>
  );
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
