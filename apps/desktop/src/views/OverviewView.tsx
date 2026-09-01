import {
  AlertTriangle,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Copy,
  FolderOpen,
  HardDrive,
  ListChecks,
  Map,
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

  function runNextStep() {
    if (!root) {
      onPickFolder();
      return;
    }
    if (!mapReady) {
      onStartDirectoryScan(root);
      return;
    }
    if (!detailReady) {
      onStartScan();
      return;
    }
    onOpenLargeFiles();
  }

  const nextLabel = !root
    ? "폴더 선택"
    : !mapReady
      ? "폴더 용량 지도 만들기"
      : !detailReady
        ? "큰 파일·중복 찾기"
        : "큰 파일 보기";

  return (
    <div className="view-stack storage-overview">
      <section
        className={`storage-guide ${mapReady && !mapScanning ? "is-compact" : ""}`}
        aria-labelledby="storage-guide-title"
      >
        <div className="storage-guide__copy">
          <p className="eyebrow">{mapReady ? "용량 지도 준비됨" : "처음이라면 여기부터"}</p>
          <h2 id="storage-guide-title">
            {mapReady ? "큰 사각형부터 눌러 안쪽 폴더를 보세요" : "폴더를 고르고 용량 지도를 만드세요"}
          </h2>
          <p>
            {mapReady
              ? "사각형이 클수록 더 많은 용량을 사용합니다."
              : "지도에서 큰 사각형을 누르면 하위 폴더로 이동합니다."}
          </p>
        </div>

        {!mapReady || mapScanning ? (
          <ol className="storage-guide__steps" aria-label="용량 관리 순서">
            <GuideStep
              number="1"
              label="폴더 선택"
              detail={root ? "선택됨" : "먼저 선택"}
              state={root ? "complete" : "active"}
            />
            <GuideStep
              number="2"
              label="용량 지도"
              detail={mapScanning ? "분석 중" : root ? "다음 단계" : "대기"}
              state={root ? "active" : "pending"}
            />
            <GuideStep
              number="3"
              label="큰 파일·중복"
              detail={scanning ? "검사 중" : "대기"}
              state="pending"
            />
          </ol>
        ) : null}

        <div className="storage-guide__actions">
          {mapScanning ? (
            <button className="secondary-button danger-outline" type="button" onClick={onCancelDirectoryScan}>
              <X size={17} aria-hidden="true" />
              지도 만들기 취소
            </button>
          ) : scanning ? (
            <button className="secondary-button danger-outline" type="button" onClick={onCancelScan}>
              <X size={17} aria-hidden="true" />
              자세한 검사 취소
            </button>
          ) : (
            <button className="primary-button" type="button" disabled={actionBlocked} onClick={runNextStep}>
              {!root ? (
                <FolderOpen size={17} aria-hidden="true" />
              ) : !mapReady ? (
                <Map size={17} aria-hidden="true" />
              ) : !detailReady ? (
                <Search size={17} aria-hidden="true" />
              ) : (
                <ChevronRight size={17} aria-hidden="true" />
              )}
              {nextLabel}
            </button>
          )}
          {root && !mapScanning && !scanning ? (
            <button className="storage-guide__change" type="button" disabled={actionBlocked} onClick={onPickFolder}>
              폴더 바꾸기
            </button>
          ) : null}
        </div>

        {scanning ? (
          <div className="storage-guide__progress" role="status">
            <span>{progress?.message ?? "큰 파일과 중복을 확인하고 있습니다"}</span>
            <strong>{formatCount(progress?.processedFiles ?? 0)}개 · {formatBytes(progress?.processedBytes ?? 0)}</strong>
          </div>
        ) : null}

        {error ? (
          <p className="storage-guide__error" role="alert">
            <AlertTriangle size={16} aria-hidden="true" />
            {error}
          </p>
        ) : state === "cancelled" ? (
          <p className="storage-guide__notice" role="status">자세한 검사를 취소했습니다. 이전 완료 결과가 있으면 그대로 유지됩니다.</p>
        ) : null}
      </section>

      <StorageTreemapPanel
        root={root}
        report={directoryReport}
        progress={directoryProgress}
        state={directoryState}
        error={directoryError}
        breadcrumbs={directoryBreadcrumbs}
        blocked={blocked || scanning || driveScanning}
        showAction={false}
        onPickFolder={onPickFolder}
        onStart={onStartDirectoryScan}
        onCancel={onCancelDirectoryScan}
      />

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

interface GuideStepProps {
  number: string;
  label: string;
  detail: string;
  state: "pending" | "active" | "complete";
}

function GuideStep({ number, label, detail, state }: GuideStepProps) {
  return (
    <li className={`is-${state}`} aria-current={state === "active" ? "step" : undefined}>
      <span>{state === "complete" ? <CheckCircle2 size={15} aria-hidden="true" /> : number}</span>
      <span>
        <strong>{label}</strong>
        <small>{detail}</small>
      </span>
    </li>
  );
}
