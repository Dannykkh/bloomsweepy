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
import { useLanguage, type MessageKey } from "../i18n";

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
  label: MessageKey;
  extensions: string[];
}> = [
  { id: "all", label: "전체", extensions: [] },
  { id: "text", label: "텍스트·코드", extensions: textExtensions },
  { id: "office", label: "워드·엑셀·한글", extensions: ["docx", "xlsx", "pptx", "hwpx"] },
  { id: "pdf", label: "PDF", extensions: ["pdf"] },
];

const formatLabels: Record<DocumentFormat, MessageKey> = {
  plainText: "텍스트",
  pdf: "PDF",
  word: "문서",
  spreadsheet: "표",
  presentation: "발표",
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
  const { t } = useLanguage();
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
      setSearchError(normalizeError(reason, t("알 수 없는 오류가 발생했습니다")));
      setSearchState("error");
    }
  }

  async function openDocument(path: string, name: string) {
    setInspectionMessage(null);
    try {
      const outcome = await inspectFile(path);
      setInspectionMessage(
        outcome === "opened"
          ? t("{{name}} 문서를 기본 앱으로 열었습니다.", { name })
          : t("{{name}}은 직접 열도록 허용한 문서 형식이 아니라 폴더에서 위치만 표시했습니다.", { name }),
      );
    } catch (reason) {
      setInspectionMessage(t("{{name}} 문서를 열지 못했습니다: {{detail}}", {
        name,
        detail: normalizeError(reason, t("알 수 없는 오류가 발생했습니다")),
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
        detail: normalizeError(reason, t("알 수 없는 오류가 발생했습니다")),
      }));
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
            <p className="eyebrow">{t("문서 내용에서 찾기")}</p>
            <h2 id="document-search-title">{t("문서 안의 단어와 문장을 찾습니다")}</h2>
            <p>
              {t("파일은 이 기기 안에서만 읽고 검색하기 좋게 정리합니다. 문서 내용은 외부로 보내지 않습니다.")}
            </p>
          </div>
          <span className="document-local-badge">
            <ShieldCheck size={14} aria-hidden="true" />
            {t("이 기기 안에서만 처리")}
          </span>
        </div>

        <form className="document-query" role="search" onSubmit={runSearch}>
          <Search size={20} aria-hidden="true" />
          <label htmlFor="document-query-input" className="sr-only">
            {t("문서 내용 검색어")}
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
                ? t("선택한 폴더의 문서 목록을 먼저 새로고침하세요…")
                : index
                  ? t("예: 계약 변경, 오류 코드, 회의 결정…")
                  : t("먼저 검색할 폴더의 문서를 읽어 두세요…")
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
            {searchState === "searching" ? t("검색 중…") : t("내용 검색")}
          </button>
        </form>

        <div className="document-filter-row">
          <div className="segmented-control" role="group" aria-label={t("문서 형식 필터")}>
            {extensionFilters.map((item) => (
              <button
                type="button"
                className={filter === item.id ? "is-active" : ""}
                aria-pressed={filter === item.id}
                key={item.id}
                onClick={() => setFilter(item.id)}
              >
                {t(item.label)}
              </button>
            ))}
          </div>
          <span title={effectiveRoot ?? undefined}>
            <FolderOpen size={13} aria-hidden="true" />
            {effectiveRoot ?? t("검색 범위가 선택되지 않았습니다")}
          </span>
        </div>
      </section>

      {scopeChanged ? (
        <section className="document-scope-warning" role="status">
          <AlertTriangle size={18} aria-hidden="true" />
          <div>
            <strong>{t("선택한 폴더가 지금 읽어 둔 문서 목록과 다릅니다")}</strong>
            <p>{t("이전 폴더의 결과와 섞이지 않도록 새 폴더의 문서를 다시 읽어 주세요.")}</p>
          </div>
          <button type="button" disabled={blocked || state === "scanning"} onClick={onStartIndex}>
            {t("새 폴더 다시 읽기")}
          </button>
        </section>
      ) : null}

      <section className="document-index-console" aria-label={t("문서 검색 준비 상태")}>
        <div className="document-index-console__status">
          <span aria-hidden="true">
            <Database size={19} />
          </span>
          <div>
            <strong>
              {state === "scanning"
                ? t("문서를 읽고 검색 목록을 새로 만들고 있습니다…")
                : index
                  ? t("{{count}}개 문서를 검색할 수 있습니다", { count: formatCount(index.indexedDocuments) })
                  : t("아직 읽어 둔 문서가 없습니다")}
            </strong>
            <p>
              {state === "scanning"
                ? t("문서 파일을 확인하고 있습니다…")
                : index
                  ? t("{{date}} 새로고침 · 읽은 문서 {{size}} · {{duration}}", {
                      date: formatDate(index.completedAtUnixMs),
                      size: formatBytes(index.indexedBytes),
                      duration: formatDuration(index.durationMs),
                    })
                  : t("처음에는 문서 내용을 읽고, 다음부터는 바뀐 문서만 다시 읽습니다.")}
            </p>
          </div>
        </div>

        {state === "scanning" ? (
          <button className="document-cancel-button" type="button" onClick={onCancelIndex}>
            <X size={15} aria-hidden="true" />
            {t("읽기 취소")}
          </button>
        ) : (
          <div className="document-index-console__actions">
            {!effectiveRoot ? (
              <button type="button" disabled={blocked} onClick={onPickFolder}>
                <FolderOpen size={15} aria-hidden="true" />
                {t("폴더 선택")}
              </button>
            ) : null}
            <button
              className={!index ? "primary-button" : ""}
              type="button"
              disabled={blocked || !effectiveRoot}
              onClick={onStartIndex}
            >
              <RefreshCw size={15} aria-hidden="true" />
              {index && !scopeChanged ? t("문서 목록 새로고침") : t("문서 미리 읽기")}
            </button>
          </div>
        )}

        {state === "scanning" && showIndexProgress ? (
          <div className="document-index-progress" role="status" aria-live="polite">
            <IndexMetric label={t("확인한 파일")} value={t("{{count}}개", { count: formatCount(progress?.scannedFiles ?? 0) })} />
            <IndexMetric label={t("읽을 문서")} value={t("{{count}}개", { count: formatCount(progress?.candidateDocuments ?? 0) })} />
            <IndexMetric label={t("검색 준비됨")} value={t("{{count}}개", { count: formatCount(progress?.indexedDocuments ?? 0) })} />
            <IndexMetric label={t("다시 안 읽음")} value={t("{{count}}개", { count: formatCount(progress?.reusedDocuments ?? 0) })} />
          </div>
        ) : null}
      </section>

      {error ? (
        <section className="document-error" role="alert">
          <AlertTriangle size={18} aria-hidden="true" />
          <div>
            <strong>{t("문서를 검색할 수 있게 준비하지 못했습니다")}</strong>
            <p>{error}</p>
          </div>
        </section>
      ) : null}

      {lastBuild && index?.completedAtUnixMs === lastBuild.completedAtUnixMs ? (
        <>
          <section className="document-build-evidence" aria-label={t("최근 문서 읽기 결과")}>
            <span>
              <strong>{formatCount(lastBuild.updatedDocuments)}</strong>
              {t("새로 읽음")}
            </span>
            <span>
              <strong>{formatCount(lastBuild.reusedDocuments)}</strong>
              {t("변경 없음")}
            </span>
            <span>
              <strong>{formatCount(lastBuild.skippedDocuments)}</strong>
              {t("건너뜀")}
            </span>
            <span>
              <strong>{formatCount(lastBuild.unsupportedDocuments)}</strong>
              {t("구형 HWP")}
            </span>
            <p>
              {t("PDF 안에서 마우스로 선택할 수 있는 글자만 찾습니다. 사진처럼 저장된 PDF와 비밀번호가 걸린 문서는 읽지 않습니다.")}
            </p>
          </section>
          {lastBuild.documentLimitReached ? (
            <section className="document-scope-warning" role="status">
              <AlertTriangle size={18} aria-hidden="true" />
              <div>
                <strong>{t("한 번에 읽을 수 있는 문서 수를 넘었습니다")}</strong>
                <p>{t("일부 문서가 빠졌을 수 있습니다. 더 작은 폴더를 선택해 다시 읽어 주세요.")}</p>
              </div>
            </section>
          ) : null}
          {lastBuild.issues.length > 0 ? (
            <details className="document-index-issues">
              <summary>
                {t("읽지 못한 문서와 건너뜀 사유 {{count}}개", { count: formatCount(lastBuild.issues.length) })}
              </summary>
              <ul>
                {lastBuild.issues.slice(0, 20).map((issue, issueIndex) => (
                  <li key={`${issue.path ?? "unknown"}-${issueIndex}`}>
                    <strong title={issue.path ?? undefined}>{issue.path ?? t("경로 정보 없음")}</strong>
                    <span>{issue.message}</span>
                  </li>
                ))}
              </ul>
              {lastBuild.issues.length > 20 ? (
                <p>{t("화면에는 처음 20개 사유만 표시합니다.")}</p>
              ) : null}
            </details>
          ) : null}
        </>
      ) : null}

      {searchError ? (
        <section className="document-error" role="alert">
          <AlertTriangle size={18} aria-hidden="true" />
          <div>
            <strong>{t("문서를 검색하지 못했습니다")}</strong>
            <p>{searchError}</p>
          </div>
        </section>
      ) : null}

      {searchReport ? (
        <section className="document-results" aria-labelledby="document-results-title">
          <header className="document-results__header">
            <div>
              <p className="eyebrow">{t("이 기기에서 찾은 결과")}</p>
              <h2 id="document-results-title">{t("“{{query}}” 검색 결과", { query: searchReport.query })}</h2>
              <p>
                {t("{{documents}}개 문서에서 {{matches}}개를 찾았습니다.", {
                  documents: formatCount(searchReport.searchedDocuments),
                  matches: formatCount(searchReport.totalMatches),
                })}
                {searchReport.resultsTruncated ? ` ${t("상위 100개만 표시합니다.")}` : ""}
              </p>
            </div>
            <span>{t(activeFilter.label)}</span>
          </header>

          {searchReport.results.length === 0 ? (
            <div className="document-results-empty">
              <Search size={24} aria-hidden="true" />
              <strong>{t("일치하는 문서가 없습니다")}</strong>
              <p>{t("단어를 줄이거나 다른 문서 형식을 선택해 다시 검색해 보세요.")}</p>
            </div>
          ) : (
            <div className="document-result-list">
              {searchReport.results.map((result) => (
                <article
                  className="document-result"
                  tabIndex={0}
                  key={result.path}
                  aria-label={t("{{name}}, 더블클릭하거나 Enter 키를 눌러 열기", { name: result.name })}
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
                      <span>{t(formatLabels[result.format])}</span>
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
            <strong>{t("첫 버전 검색 범위")}</strong>
          </div>
          <p>
            {t("TXT·Markdown·코드·CSV·JSON, 워드·엑셀·파워포인트·HWPX, 글자를 선택할 수 있는 PDF를 지원합니다. 구형 HWP와 사진으로 된 PDF는 아직 내용 검색을 지원하지 않습니다.")}
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

function normalizeError(reason: unknown, fallback: string): string {
  if (reason instanceof Error) return reason.message;
  if (typeof reason === "string") return reason;
  return fallback;
}
