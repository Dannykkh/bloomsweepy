import {
  AppWindow,
  Archive,
  ChevronRight,
  Code2,
  Database,
  Download,
  FileText,
  Folder,
  HardDrive,
  Image,
  Monitor,
  Music,
  RefreshCw,
  Search,
  Trash2,
  Users,
  Video,
  X,
  type LucideIcon,
} from "lucide-react";
import { formatBytes, formatCount, formatDuration } from "../lib/format";
import type {
  DriveScanProgress,
  DriveScanReport,
  ScanUiState,
  StorageCategory,
  StorageCategoryKind,
  StorageLocation,
  VolumeInfo,
} from "../types";

interface DriveStoragePanelProps {
  platform: string | null;
  volume: VolumeInfo | null;
  report: DriveScanReport | null;
  progress: DriveScanProgress | null;
  state: ScanUiState;
  error: string | null;
  blocked: boolean;
  onStart: () => void;
  onCancel: () => void;
  onExplorePath: (location: StorageLocation) => void;
}

interface CategoryPresentation {
  label: string;
  description: string;
  icon: LucideIcon;
}

const categoryPresentation: Record<StorageCategoryKind, CategoryPresentation> = {
  applications: {
    label: "설치된 앱",
    description: "앱 본체와 로컬 애플리케이션 데이터",
    icon: AppWindow,
  },
  system: {
    label: "시스템 사용 및 예약",
    description: "운영체제와 보호된 시스템 영역",
    icon: Database,
  },
  temporaryFiles: {
    label: "임시 파일",
    description: "캐시, 로그, 빌드 및 작업 중간 파일",
    icon: Trash2,
  },
  recycleBin: {
    label: "휴지통",
    description: "복원할 수 있도록 보관 중인 파일",
    icon: Trash2,
  },
  desktop: {
    label: "데스크톱",
    description: "현재 사용자의 바탕 화면",
    icon: Monitor,
  },
  documents: {
    label: "문서",
    description: "문서 폴더와 일반 문서 형식",
    icon: FileText,
  },
  downloads: {
    label: "다운로드",
    description: "브라우저와 앱에서 내려받은 파일",
    icon: Download,
  },
  photos: {
    label: "사진",
    description: "사진 폴더와 이미지 형식",
    icon: Image,
  },
  videos: {
    label: "동영상",
    description: "비디오 폴더와 영상 형식",
    icon: Video,
  },
  audio: {
    label: "음악 및 오디오",
    description: "음악 폴더와 오디오 형식",
    icon: Music,
  },
  archives: {
    label: "압축 및 디스크 이미지",
    description: "압축 파일과 ISO 이미지",
    icon: Archive,
  },
  developer: {
    label: "개발 파일",
    description: "소스, 의존성, 빌드 및 테스트 산출물",
    icon: Code2,
  },
  otherUsers: {
    label: "다른 사용자",
    description: "현재 계정 외 사용자 프로필",
    icon: Users,
  },
  other: {
    label: "기타",
    description: "아직 명확한 범주로 분류되지 않은 파일",
    icon: Folder,
  },
};

const categoryOrder = Object.keys(categoryPresentation) as StorageCategoryKind[];

export function DriveStoragePanel({
  platform,
  volume,
  report,
  progress,
  state,
  error,
  blocked,
  onStart,
  onCancel,
  onExplorePath,
}: DriveStoragePanelProps) {
  const scanning = state === "scanning";
  const categories = mergeCategories(
    scanning ? progress?.categories : report?.categories,
  );
  const volumeUsedBytes = volume
    ? Math.max(0, volume.totalBytes - volume.availableBytes)
    : Math.max(1, progress?.processedBytes ?? report?.totalLogicalBytes ?? 1);
  const installedApps = report?.installedApps;
  const platformLabel = platform === "windows" ? "Windows" : platform === "macos" ? "macOS" : "현재 OS";

  return (
    <section className="drive-inventory" aria-labelledby="drive-inventory-title">
      <header className="drive-inventory__header">
        <div>
          <p className="eyebrow">DRIVE INVENTORY</p>
          <h2 id="drive-inventory-title">드라이브 사용량</h2>
          <p>
            {volume
              ? `${volume.mountPoint}의 실제 파일을 읽기 전용으로 분류합니다.`
              : "분석할 수 있는 드라이브를 확인하고 있습니다."}
          </p>
        </div>
        <div className="drive-inventory__actions">
          {scanning ? (
            <button className="secondary-button danger-outline" type="button" onClick={onCancel}>
              <X size={16} aria-hidden="true" />
              분석 취소
            </button>
          ) : (
            <button
              className="primary-button"
              type="button"
              disabled={!volume || blocked}
              onClick={onStart}
            >
              {report ? <RefreshCw size={16} aria-hidden="true" /> : <Search size={16} aria-hidden="true" />}
              {report ? "다시 분석" : "드라이브 분석"}
            </button>
          )}
        </div>
      </header>

      {scanning ? (
        <div className="drive-progress" role="status" aria-live="polite">
          <span className="drive-progress__spinner" aria-hidden="true" />
          <div>
            <strong>{progress?.message ?? "저장공간 범주를 준비하고 있습니다"}</strong>
            <small>
              {formatCount(progress?.processedFiles ?? 0)}개 파일 · {formatBytes(progress?.processedBytes ?? 0)} 확인
            </small>
          </div>
        </div>
      ) : null}

      {error ? (
        <div className="drive-inventory__error" role="alert">
          {error}
        </div>
      ) : null}

      <div className="drive-category-list" aria-label="저장공간 범주">
        {categories.map((category) => {
          const presentation = categoryPresentation[category.kind];
          const Icon = presentation.icon;
          const percent = volumeUsedBytes
            ? Math.min(100, (category.logicalBytes / volumeUsedBytes) * 100)
            : 0;
          const appRegistryDetail =
            category.kind === "applications" && installedApps?.supported
              ? `${platformLabel} 레지스트리 ${formatCount(installedApps.applications.length)}개 앱 대조`
              : null;

          return (
            <div className="drive-category-row" key={category.kind}>
              <span className="drive-category-row__icon" aria-hidden="true">
                <Icon size={17} />
              </span>
              <div className="drive-category-row__body">
                <div className="drive-category-row__line">
                  <span>
                    <strong>{presentation.label}</strong>
                    <small>{appRegistryDetail ?? presentation.description}</small>
                  </span>
                  <span className="drive-category-row__metric">
                    <strong>
                      {report || scanning ? formatBytes(category.logicalBytes) : "—"}
                    </strong>
                    <small>
                      {report || scanning
                        ? `${formatCount(category.fileCount)}개 파일`
                        : "분석 전"}
                    </small>
                  </span>
                </div>
                <div className="drive-category-row__track" aria-hidden="true">
                  <span style={{ transform: `scaleX(${percent / 100})` }} />
                </div>
              </div>
            </div>
          );
        })}
      </div>

      <footer className="drive-inventory__footer">
        <span>
          <HardDrive size={15} aria-hidden="true" />
          {report
            ? `${formatDuration(report.durationMs)} · 접근 가능한 논리 용량 ${formatBytes(report.totalLogicalBytes)}`
            : "드라이브 분류 단계는 파일을 변경하지 않습니다"}
        </span>
        {report ? (
          <span>
            {formatCount(report.unreadableEntries)}개 접근 제한 · {report.hardLinkDeduplication
              ? `하드링크 ${formatCount(report.hardLinksSkipped)}개 제외`
              : "논리 크기 기준"}
            {report.locationTrackingLimitReached ? " · 위치 목록 안전 상한 도달" : ""}
            {report.hardLinkIdentityLimitReached ? " · 하드링크 집계 상한 도달" : ""}
          </span>
        ) : null}
      </footer>

      {report?.largestLocations.length ? (
        <div className="drive-locations">
          <div className="drive-locations__heading">
            <strong>용량이 큰 위치</strong>
            <small>상위 {Math.min(8, report.largestLocations.length)}개</small>
          </div>
          {report.largestLocations.slice(0, 8).map((location) => (
            <button
              className="drive-location-row"
              key={location.path}
              type="button"
              disabled={blocked}
              onClick={() => onExplorePath(location)}
              title={`${location.path}의 저장공간 맵 열기`}
            >
              <span>
                <strong>{location.name}</strong>
                <small title={location.path}>{location.path}</small>
              </span>
              <span>
                <strong>{formatBytes(location.logicalBytes)}</strong>
                <small>{categoryPresentation[location.dominantCategory].label}</small>
              </span>
              <ChevronRight size={15} aria-hidden="true" />
            </button>
          ))}
        </div>
      ) : null}
    </section>
  );
}

function mergeCategories(
  categories: StorageCategory[] | null | undefined,
): StorageCategory[] {
  const byKind = new Map(categories?.map((category) => [category.kind, category]));
  return categoryOrder.map(
    (kind) => byKind.get(kind) ?? { kind, logicalBytes: 0, fileCount: 0 },
  );
}
