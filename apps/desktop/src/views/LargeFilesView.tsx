import { Search } from "lucide-react";
import { useMemo, useState } from "react";
import { FileTable } from "../components/FileTable";
import { formatBytes, formatCount } from "../lib/format";
import type { ScanReport } from "../types";

interface LargeFilesViewProps {
  report: ScanReport | null;
  scanning: boolean;
  onStartScan: () => void;
}

export function LargeFilesView({ report, scanning, onStartScan }: LargeFilesViewProps) {
  const [query, setQuery] = useState("");
  const filtered = useMemo(() => {
    const normalized = query.trim().toLocaleLowerCase("ko-KR");
    if (!report || !normalized) return report?.largeFiles ?? [];
    return report.largeFiles.filter(
      (file) =>
        file.name.toLocaleLowerCase("ko-KR").includes(normalized) ||
        file.path.toLocaleLowerCase("ko-KR").includes(normalized),
    );
  }, [query, report]);
  const totalBytes = filtered.reduce((total, file) => total + file.logicalBytes, 0);

  if (!report) {
    return (
      <div className="empty-panel empty-panel--page">
        <Search size={28} aria-hidden="true" />
        <strong>큰 파일과 중복 파일을 한 번에 검사하세요</strong>
        <p>한 번의 자세한 검사로 큰 파일 목록과 실제 중복 결과를 함께 만듭니다.</p>
        <button className="primary-button" type="button" disabled={scanning} onClick={onStartScan}>
          <Search size={17} aria-hidden="true" />
          큰 파일·중복 찾기
        </button>
      </div>
    );
  }

  return (
    <div className="view-stack">
      <section className="results-toolbar">
        <div className="result-kpi">
          <span>현재 결과</span>
          <strong>{formatCount(filtered.length)}개</strong>
          <small>{formatBytes(totalBytes)}</small>
        </div>
        <label className="search-field">
          <Search size={17} aria-hidden="true" />
          <span className="sr-only">파일 이름 또는 경로 검색</span>
          <input
            type="search"
            name="file-search"
            value={query}
            autoComplete="off"
            spellCheck={false}
            placeholder="파일 이름 또는 경로 검색…"
            onChange={(event) => setQuery(event.currentTarget.value)}
          />
        </label>
      </section>

      <section className="results-section results-section--page">
        <div className="section-heading">
          <div>
            <p className="eyebrow">큰 파일부터 보기</p>
            <h2>크기순 파일 목록</h2>
          </div>
          <span>삭제 기능 없이 분석만 수행합니다</span>
        </div>
        <FileTable files={filtered} emptyMessage="검색 조건과 일치하는 큰 파일이 없습니다." />
      </section>
    </div>
  );
}
