import {
  AlertTriangle,
  CheckCircle2,
  Files,
  FolderOpen,
  Search,
  ShieldCheck,
  X,
} from "lucide-react";
import { FileTable } from "../components/FileTable";
import { DriveStoragePanel } from "../components/DriveStoragePanel";
import { StorageTreemapPanel } from "../components/StorageTreemapPanel";
import { StorageRing } from "../components/StorageRing";
import { formatBytes, formatCount, formatDuration } from "../lib/format";
import type {
  DirectoryBreadcrumb,
  DirectoryScanProgress,
  DirectoryScanReport,
  DriveScanProgress,
  DriveScanReport,
  ScanProgress,
  ScanReport,
  ScanUiState,
  VolumeInfo,
} from "../types";

interface OverviewViewProps {
  platform: string | null;
  root: string | null;
  volume: VolumeInfo | null;
  report: ScanReport | null;
  progress: ScanProgress | null;
  state: ScanUiState;
  error: string | null;
  driveReport: DriveScanReport | null;
  driveProgress: DriveScanProgress | null;
  driveState: ScanUiState;
  driveError: string | null;
  directoryRoot: string | null;
  directoryReport: DirectoryScanReport | null;
  directoryProgress: DirectoryScanProgress | null;
  directoryState: ScanUiState;
  directoryError: string | null;
  directoryBreadcrumbs: DirectoryBreadcrumb[];
  blocked: boolean;
  onPickFolder: () => void;
  onStartScan: () => void;
  onCancelScan: () => void;
  onStartDriveScan: () => void;
  onCancelDriveScan: () => void;
  onStartDirectoryScan: (path: string, breadcrumbs?: DirectoryBreadcrumb[]) => void;
  onCancelDirectoryScan: () => void;
}

export function OverviewView({
  platform,
  root,
  volume,
  report,
  progress,
  state,
  error,
  driveReport,
  driveProgress,
  driveState,
  driveError,
  directoryRoot,
  directoryReport,
  directoryProgress,
  directoryState,
  directoryError,
  directoryBreadcrumbs,
  blocked,
  onPickFolder,
  onStartScan,
  onCancelScan,
  onStartDriveScan,
  onCancelDriveScan,
  onStartDirectoryScan,
  onCancelDirectoryScan,
}: OverviewViewProps) {
  const scanning = state === "scanning";
  const cancelled = state === "cancelled";
  const largeBytes = report?.largeFiles.reduce(
    (total, file) => total + file.logicalBytes,
    0,
  );

  return (
    <div className="view-stack">
      <section className={`state-banner ${error ? "is-error" : ""}`} aria-live="polite">
        <span className="state-banner__icon" aria-hidden="true">
          {error ? (
            <AlertTriangle size={20} />
          ) : scanning ? (
            <Search size={20} />
          ) : cancelled ? (
            <X size={20} />
          ) : report ? (
            <CheckCircle2 size={20} />
          ) : (
            <ShieldCheck size={20} />
          )}
        </span>
        <div>
          <strong>
            {error
              ? "스캔을 완료하지 못했습니다"
              : scanning
                ? "저장공간을 분석하고 있습니다"
                : cancelled
                  ? "스캔을 취소했습니다"
                : report
                  ? "분석이 완료됐습니다"
                  : "검사 후 선택하는 안전 모드입니다"}
          </strong>
          <p>
            {error
              ? error
              : scanning
                ? progress?.message ?? "스캔 작업을 준비하고 있습니다"
                : cancelled
                  ? "취소된 스캔 결과는 삭제 판단에 사용하지 않습니다."
                : report
                  ? `${formatDuration(report.durationMs)} 동안 ${formatCount(report.totalFiles)}개 파일을 확인했습니다.`
                  : "파일을 이동하거나 삭제하지 않고 크기와 중복 여부만 확인합니다."}
          </p>
        </div>
      </section>

      <DriveStoragePanel
        platform={platform}
        volume={volume}
        report={driveReport}
        progress={driveProgress}
        state={driveState}
        error={driveError}
        blocked={blocked || state === "scanning" || directoryState === "scanning"}
        onStart={onStartDriveScan}
        onCancel={onCancelDriveScan}
        onExplorePath={(location) =>
          onStartDirectoryScan(location.path, [
            { name: location.name, path: location.path },
          ])
        }
      />

      <StorageTreemapPanel
        root={directoryRoot}
        report={directoryReport}
        progress={directoryProgress}
        state={directoryState}
        error={directoryError}
        breadcrumbs={directoryBreadcrumbs}
        blocked={blocked || state === "scanning" || driveState === "scanning"}
        onStart={onStartDirectoryScan}
        onCancel={onCancelDirectoryScan}
      />

      <section className="scan-grid" aria-label="저장공간 스캔">
        <div className="scan-stage glass-panel">
          <StorageRing volume={volume} report={report} scanning={scanning} />
          <div className="scan-stage__copy">
            <p className="eyebrow">파일 내용까지 확인</p>
            <h2>{root ? "선택한 범위를 정밀하게 확인합니다" : "먼저 분석할 폴더를 선택하세요"}</h2>
            <p>
              큰 파일은 크기순으로 정렬하고, 중복 후보는 일부 내용을 먼저 확인한 뒤 전체 내용을 끝까지 비교합니다.
            </p>
          </div>

          <div className="scan-actions">
            {scanning ? (
              <button className="secondary-button danger-outline" type="button" onClick={onCancelScan}>
                <X size={17} aria-hidden="true" />
                스캔 취소
              </button>
            ) : (
              <button
                className="primary-button"
                type="button"
                disabled={blocked}
                onClick={root ? onStartScan : onPickFolder}
              >
                {root ? <Search size={17} aria-hidden="true" /> : <FolderOpen size={17} aria-hidden="true" />}
                {root ? "스캔 시작" : "폴더 선택"}
              </button>
            )}
            {root && !scanning ? (
              <button className="secondary-button" type="button" disabled={blocked} onClick={onPickFolder}>
                범위 변경
              </button>
            ) : null}
          </div>

          {scanning ? (
            <div className="progress-block" role="status" aria-live="polite">
              <div className="progress-block__meta">
                <span>{progress?.message ?? "스캔 준비 중"}</span>
                <strong>{formatCount(progress?.processedFiles ?? 0)}개</strong>
              </div>
              <div
                className={`progress-track ${progress?.fraction == null ? "is-indeterminate" : ""}`}
                role="progressbar"
                aria-label="스캔 진행률"
                aria-valuemin={0}
                aria-valuemax={100}
                aria-valuenow={
                  progress?.fraction == null
                    ? undefined
                    : Math.round(progress.fraction * 100)
                }
              >
                <span
                  style={
                    progress?.fraction == null
                      ? undefined
                      : { transform: `scaleX(${progress.fraction})` }
                  }
                />
              </div>
              <small>{formatBytes(progress?.processedBytes ?? 0)} 확인</small>
            </div>
          ) : null}
        </div>

        <aside className="evidence-rail" aria-label="스캔 요약">
          <SummaryMetric
            label="확인한 파일"
            value={report ? formatCount(report.totalFiles) : "—"}
            detail={report ? formatBytes(report.totalLogicalBytes) : "스캔 후 표시"}
            icon={<Files size={18} />}
          />
          <SummaryMetric
            label="큰 파일"
            value={report ? formatCount(report.largeFiles.length) : "—"}
            detail={report ? formatBytes(largeBytes ?? 0) : "설정 기준 이상"}
            icon={<Search size={18} />}
          />
          <SummaryMetric
            label="중복 낭비"
            value={report ? formatBytes(report.duplicateWasteBytes) : "—"}
            detail={report ? `${formatCount(report.duplicateGroups.length)}개 그룹` : "전체 내용 검증"}
            icon={<ShieldCheck size={18} />}
          />
          <SummaryMetric
            label="읽지 못한 항목"
            value={report ? formatCount(report.unreadableEntries) : "—"}
            detail={report?.hardLinksSkipped ? `하드링크 ${formatCount(report.hardLinksSkipped)}개 제외` : "권한 오류를 별도 기록"}
            icon={<AlertTriangle size={18} />}
            tone={report?.unreadableEntries ? "warning" : "neutral"}
          />
        </aside>
      </section>

      <section className="results-section">
        <div className="section-heading">
          <div>
            <p className="eyebrow">가장 큰 파일</p>
            <h2>가장 큰 파일</h2>
          </div>
          {report ? (
            <span>
              {report.largeFiles.length === 0
                ? "조건에 맞는 파일 없음"
                : `${report.largeFiles.length}개 중 상위 ${Math.min(6, report.largeFiles.length)}개`}
            </span>
          ) : null}
        </div>
        {report ? (
          <FileTable
            files={report.largeFiles.slice(0, 6)}
            emptyMessage="설정한 기준보다 큰 파일이 없습니다."
          />
        ) : (
          <div className="empty-panel">
            <Search size={24} aria-hidden="true" />
            <strong>아직 스캔 결과가 없습니다</strong>
            <p>폴더를 선택하고 스캔하면 실제 파일 경로와 크기가 여기에 표시됩니다.</p>
          </div>
        )}
      </section>

      {report?.candidateLimitReached ? (
        <div className="notice-panel" role="alert">
          <AlertTriangle size={18} aria-hidden="true" />
          <p>중복 후보 안전 한도에 도달했습니다. 설정에서 후보 한도를 높인 뒤 다시 스캔하세요.</p>
        </div>
      ) : null}
    </div>
  );
}

interface SummaryMetricProps {
  label: string;
  value: string;
  detail: string;
  icon: React.ReactNode;
  tone?: "neutral" | "warning";
}

function SummaryMetric({
  label,
  value,
  detail,
  icon,
  tone = "neutral",
}: SummaryMetricProps) {
  return (
    <div className={`summary-metric ${tone === "warning" ? "is-warning" : ""}`}>
      <span className="summary-metric__icon" aria-hidden="true">
        {icon}
      </span>
      <div>
        <span>{label}</span>
        <strong>{value}</strong>
        <small>{detail}</small>
      </div>
    </div>
  );
}
