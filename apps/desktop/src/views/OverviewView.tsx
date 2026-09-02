import {
  AlertTriangle,
  ChevronDown,
  ChevronRight,
  Copy,
  HardDrive,
  ListChecks,
  Search,
  X,
} from "lucide-react";
import { DriveStoragePanel } from "../components/DriveStoragePanel";
import { StorageTreemapPanel } from "../components/StorageTreemapPanel";
import { formatBytes, formatCount } from "../lib/format";
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
  onOpenLargeFiles: () => void;
  onOpenDuplicates: () => void;
  onOpenCleanup: () => void;
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
  onOpenLargeFiles,
  onOpenDuplicates,
  onOpenCleanup,
}: OverviewViewProps) {
  const scanning = state === "scanning";
  const mapScanning = directoryState === "scanning";
  const driveScanning = driveState === "scanning";
  const mapReady = Boolean(directoryReport);
  const detailReady = Boolean(report);
  const largeBytes = report?.largeFiles.reduce(
    (total, file) => total + file.logicalBytes,
    0,
  ) ?? 0;
  const actionBlocked = blocked || driveScanning;

  return (
    <div className="view-stack storage-overview">
      <StorageTreemapPanel
        root={root}
        report={directoryReport}
        progress={directoryProgress}
        state={directoryState}
        error={directoryError}
        breadcrumbs={directoryBreadcrumbs}
        blocked={blocked || scanning || driveScanning}
        showAction
        onPickFolder={onPickFolder}
        onStart={onStartDirectoryScan}
        onCancel={onCancelDirectoryScan}
      />

      {mapReady && !detailReady ? (
        <section className="storage-detail-action" aria-label="큰 파일과 중복 파일 검사">
          <span>
            <strong>{scanning ? "큰 파일과 중복을 확인하고 있습니다" : "더 정리할 항목 찾기"}</strong>
            <small>
              {scanning
                ? `${formatCount(progress?.processedFiles ?? 0)}개 · ${formatBytes(progress?.processedBytes ?? 0)} 확인`
                : "큰 파일을 모으고, 중복 후보만 내용을 비교합니다."}
            </small>
          </span>
          {scanning ? (
            <button className="secondary-button danger-outline" type="button" onClick={onCancelScan}>
              <X size={17} aria-hidden="true" />
              검사 취소
            </button>
          ) : (
            <button className="primary-button" type="button" disabled={actionBlocked || mapScanning} onClick={onStartScan}>
              <Search size={17} aria-hidden="true" />
              큰 파일·중복 찾기
            </button>
          )}
          {error ? (
            <p className="storage-detail-action__error" role="alert">
              <AlertTriangle size={16} aria-hidden="true" />
              {error}
            </p>
          ) : state === "cancelled" ? (
            <p className="storage-detail-action__notice" role="status">검사를 취소했습니다.</p>
          ) : null}
        </section>
      ) : null}

      {report ? (
        <section className="storage-result-links" aria-labelledby="storage-result-links-title">
          <div className="storage-result-links__heading">
            <p className="eyebrow">검사 결과</p>
            <h2 id="storage-result-links-title">확인할 항목을 고르세요</h2>
          </div>
          <button type="button" onClick={onOpenLargeFiles}>
            <HardDrive size={19} aria-hidden="true" />
            <span>
              <strong>큰 파일</strong>
              <small>{formatCount(report.largeFiles.length)}개 · {formatBytes(largeBytes)}</small>
            </span>
            <ChevronRight size={17} aria-hidden="true" />
          </button>
          <button type="button" onClick={onOpenDuplicates}>
            <Copy size={19} aria-hidden="true" />
            <span>
              <strong>중복 파일</strong>
              <small>{formatCount(report.duplicateGroups.length)}그룹 · {formatBytes(report.duplicateWasteBytes)}</small>
            </span>
            <ChevronRight size={17} aria-hidden="true" />
          </button>
          <button type="button" onClick={onOpenCleanup}>
            <ListChecks size={19} aria-hidden="true" />
            <span>
              <strong>정리 후보</strong>
              <small>임시 파일과 삭제 후 남은 흔적</small>
            </span>
            <ChevronRight size={17} aria-hidden="true" />
          </button>
        </section>
      ) : null}

      <details className="storage-advanced">
        <summary>
          <span aria-hidden="true"><HardDrive size={19} /></span>
          <span>
            <strong>컴퓨터 전체 용량을 종류별로 보기</strong>
            <small>설치된 앱, 임시 파일, 문서, 사진처럼 시스템 드라이브를 나눠 봅니다.</small>
          </span>
          <ChevronDown size={18} aria-hidden="true" />
        </summary>
        <DriveStoragePanel
          platform={platform}
          volume={volume}
          report={driveReport}
          progress={driveProgress}
          state={driveState}
          error={driveError}
          blocked={blocked || scanning || mapScanning}
          onStart={onStartDriveScan}
          onCancel={onCancelDriveScan}
          onExplorePath={(location) =>
            onStartDirectoryScan(location.path, [
              { name: location.name, path: location.path },
            ])
          }
        />
      </details>
    </div>
  );
}
