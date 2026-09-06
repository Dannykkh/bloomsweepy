import {
  AlertTriangle,
  ArrowRight,
  Clock3,
  Copy,
  FilePlus2,
  FolderSearch,
  HardDrive,
  ListChecks,
  RefreshCw,
  Search,
  ShieldCheck,
  X,
} from "lucide-react";
import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type ReactNode,
} from "react";
import { flushSync } from "react-dom";
import { StorageRing } from "../components/StorageRing";
import { PerformancePulse } from "../components/PerformancePulse";
import {
  dashboardVolumeKey,
  resolveDashboardVolume,
  visibleDashboardVolumes,
} from "../lib/cloudVolumePolicy";
import {
  formatBytes,
  formatCount,
  formatDate,
  formatDateTimeAttribute,
} from "../lib/format";
import { findContainingVolumeForPath } from "../lib/volumePath";
import type {
  ActionHistoryEntry,
  ActionHistoryReport,
  CleanupScanProgress,
  CleanupScanReport,
  FileCatalogRecentReport,
  FileCatalogStatus,
  ScanProgress,
  ScanReport,
  ScanUiState,
  SystemOverview,
  VolumeInfo,
} from "../types";
import { useLanguage } from "../i18n";

interface DashboardViewProps {
  system: SystemOverview | null;
  report: ScanReport | null;
  progress: ScanProgress | null;
  scanState: ScanUiState;
  scanError: string | null;
  cleanupReport: CleanupScanReport | null;
  cleanupProgress: CleanupScanProgress | null;
  cleanupState: ScanUiState;
  cleanupError: string | null;
  actionHistory: ActionHistoryReport | null;
  recentFiles: FileCatalogRecentReport | null;
  fileCatalog: FileCatalogStatus | null;
  fileCatalogStale: boolean;
  loading: boolean;
  error: string | null;
  blocked: boolean;
  onRefresh: () => void;
  onStartScan: (volume: VolumeInfo) => void;
  onCancelScan: () => void;
  onStartCleanupScan: () => void;
  onCancelCleanupScan: () => void;
  onOpenStorage: () => void;
  onOpenLargeFiles: () => void;
  onOpenDuplicates: () => void;
  onOpenCleanup: () => void;
  onOpenPerformance: () => void;
  onRefreshFileCatalog: () => void;
  onOpenFileSearch: () => void;
  onRevealFile: (path: string) => void;
}

export function DashboardView({
  system,
  report,
  progress,
  scanState,
  scanError,
  cleanupReport,
  cleanupProgress,
  cleanupState,
  cleanupError,
  actionHistory,
  recentFiles,
  fileCatalog,
  fileCatalogStale,
  loading,
  error,
  blocked,
  onRefresh,
  onStartScan,
  onCancelScan,
  onStartCleanupScan,
  onCancelCleanupScan,
  onOpenStorage,
  onOpenLargeFiles,
  onOpenDuplicates,
  onOpenCleanup,
  onOpenPerformance,
  onRefreshFileCatalog,
  onOpenFileSearch,
  onRevealFile,
}: DashboardViewProps) {
  const { t } = useLanguage();
  const [selectedMountPoint, setSelectedMountPoint] = useState<string | null>(null);
  const driveDeckRef = useRef<HTMLDivElement>(null);
  const activeDriveRef = useRef<HTMLDivElement>(null);
  const driveAnimationsRef = useRef<Animation[]>([]);
  const shouldRestoreDriveFocus = useRef(false);
  const volumes = useMemo(
    () => visibleDashboardVolumes(system?.volumes ?? [], system?.platform),
    [system?.platform, system?.volumes],
  );
  const primaryVolume = useMemo(
    () => resolveDashboardVolume(volumes, selectedMountPoint),
    [selectedMountPoint, volumes],
  );
  const primaryVolumeKey = primaryVolume ? dashboardVolumeKey(primaryVolume) : null;
  const otherVolumes = volumes.filter(
    (volume) => dashboardVolumeKey(volume) !== primaryVolumeKey,
  );
  const reportVolume = report
    ? findContainingVolumeForPath(volumes, report.root)
    : null;
  const activeReport = reportVolume && primaryVolumeKey
    && dashboardVolumeKey(reportVolume) === primaryVolumeKey
    ? report
    : null;
  const scanning = scanState === "scanning";
  const cleanupScanning = cleanupState === "scanning";
  const usedBytes = primaryVolume
    ? Math.max(0, primaryVolume.totalBytes - primaryVolume.availableBytes)
    : 0;
  const usedPercent = primaryVolume?.totalBytes
    ? Math.min(100, (usedBytes / primaryVolume.totalBytes) * 100)
    : 0;
  const largeBytes = report?.largeFiles.reduce(
    (total, file) => total + file.logicalBytes,
    0,
  ) ?? 0;
  const likelySafeCandidates = cleanupReport?.candidates.filter(
    (candidate) => candidate.confidence === "likelySafe",
  ) ?? [];
  const reviewCandidates = cleanupReport?.candidates.filter(
    (candidate) => candidate.confidence === "review",
  ) ?? [];
  const likelySafeBytes = likelySafeCandidates.reduce(
    (total, candidate) => total + candidate.logicalBytes,
    0,
  );
  const reviewBytes = reviewCandidates.reduce(
    (total, candidate) => total + candidate.logicalBytes,
    0,
  );
  const scanFraction = scanning ? progress?.fraction ?? null : null;
  const cleanupFraction = cleanupScanning && cleanupProgress?.totalRoots
    ? Math.min(1, cleanupProgress.processedRoots / cleanupProgress.totalRoots)
    : null;
  const activeFraction = scanFraction ?? cleanupFraction;
  const activeMessage = scanning
    ? progress?.message ?? t("파일을 확인하고 있습니다")
    : cleanupScanning
      ? cleanupProgress?.message ?? t("남은 파일과 제거 정보를 대조하고 있습니다")
      : null;
  const activeCount = scanning
    ? progress?.processedFiles ?? 0
    : cleanupProgress?.processedEntries ?? 0;
  const activeBytes = scanning
    ? progress?.processedBytes ?? 0
    : cleanupProgress?.processedBytes ?? 0;
  const ringStatus = (scanning ? activeMessage : null)
    ?? (loading && !primaryVolume
      ? `${t("디스크 확인 중")}…`
      : activeReport
        ? t("{{count}}개 파일 확인", { count: activeReport.totalFiles.toLocaleString() })
        : t("분석 준비 완료"));

  useEffect(() => {
    if (!shouldRestoreDriveFocus.current) return;
    shouldRestoreDriveFocus.current = false;
    activeDriveRef.current?.focus({ preventScroll: true });
  }, [primaryVolumeKey]);

  useEffect(() => () => {
    driveAnimationsRef.current.forEach((animation) => animation.cancel());
    driveAnimationsRef.current = [];
  }, []);

  function selectDashboardVolume(volume: VolumeInfo) {
    if (blocked || dashboardVolumeKey(volume) === primaryVolumeKey) return;
    const previousAnimations = driveAnimationsRef.current;
    driveAnimationsRef.current = [];
    previousAnimations.forEach((animation) => animation.cancel());
    driveDeckRef.current?.classList.remove("is-swapping");
    driveDeckRef.current?.querySelectorAll<HTMLElement>(
      "[data-dashboard-drive-key]",
    ).forEach((element) => element.style.removeProperty("z-index"));

    shouldRestoreDriveFocus.current = true;
    const reducedMotion = window.matchMedia?.(
      "(prefers-reduced-motion: reduce)",
    ).matches;
    if (reducedMotion) {
      flushSync(() => setSelectedMountPoint(volume.mountPoint));
      return;
    }

    const previousRects = captureDriveRects(driveDeckRef.current);
    flushSync(() => setSelectedMountPoint(volume.mountPoint));
    const nextDriveKey = dashboardVolumeKey(volume);
    const deck = driveDeckRef.current;
    const animations = [...(deck?.querySelectorAll<HTMLElement>(
      "[data-dashboard-drive-key]",
    ) ?? [])].flatMap((element) => {
      const key = element.dataset.dashboardDriveKey;
      const previousRect = key ? previousRects.get(key) : null;
      if (!key || !previousRect) return [];

      const nextRect = element.getBoundingClientRect();
      if (nextRect.width === 0 || nextRect.height === 0) return [];
      const deltaX = previousRect.left - nextRect.left;
      const deltaY = previousRect.top - nextRect.top;
      const scaleX = previousRect.width / nextRect.width;
      const scaleY = previousRect.height / nextRect.height;
      if (
        Math.abs(deltaX) < 0.5
        && Math.abs(deltaY) < 0.5
        && Math.abs(scaleX - 1) < 0.005
        && Math.abs(scaleY - 1) < 0.005
      ) return [];

      element.style.zIndex = key === nextDriveKey ? "3" : "2";
      return [element.animate(
        [
          {
            transform: `translate(${deltaX}px, ${deltaY}px) scale(${scaleX}, ${scaleY})`,
            transformOrigin: "top left",
          },
          {
            transform: "translate(0, 0) scale(1, 1)",
            transformOrigin: "top left",
          },
        ],
        {
          duration: 460,
          easing: "cubic-bezier(0.22, 1, 0.36, 1)",
          fill: "both",
        },
      )];
    });

    if (animations.length === 0) return;
    deck?.classList.add("is-swapping");
    driveAnimationsRef.current = animations;
    void Promise.allSettled(animations.map((animation) => animation.finished)).then(() => {
      if (driveAnimationsRef.current !== animations) return;
      animations.forEach((animation) => animation.cancel());
      driveAnimationsRef.current = [];
      deck?.classList.remove("is-swapping");
      deck?.querySelectorAll<HTMLElement>("[data-dashboard-drive-key]").forEach(
        (element) => element.style.removeProperty("z-index"),
      );
    });
  }

  return (
    <div className="view-stack dashboard-view">
      <header className="dashboard-briefing" aria-labelledby="dashboard-title">
        <div>
          <p className="eyebrow">{t("오늘의 저장공간")}</p>
          <h1 id="dashboard-title">{t("저장공간 상태")}</h1>
          <p>{t("한눈에 보고, 한 번 눌러 원인을 찾습니다.")}</p>
        </div>
        <button
          className="secondary-button dashboard-refresh"
          type="button"
          disabled={loading || blocked}
          onClick={onRefresh}
        >
          <RefreshCw size={17} aria-hidden="true" />
          {loading ? `${t("확인 중")}…` : t("새로 고침")}
        </button>
      </header>

      {error ? (
        <div className="dashboard-inline-error" role="alert">
          <AlertTriangle size={18} aria-hidden="true" />
          <span>{error}</span>
          <button
            className="secondary-button dashboard-inline-error__action"
            type="button"
            disabled={loading || blocked}
            onClick={onRefresh}
          >
            {t("다시 시도")}
          </button>
        </div>
      ) : null}

      <PerformancePulse onOpen={onOpenPerformance} />

      <section
        className="dashboard-hero glass-panel"
        aria-labelledby="dashboard-hero-title"
      >
        <div className="dashboard-hero__copy">
          <p className="eyebrow">
            {primaryVolume
              ? primaryVolume.isSystem
                ? t("시스템 드라이브")
                : t("선택한 드라이브")
              : t("기본 디스크 상태")}
          </p>
          <h2 id="dashboard-hero-title">
            {primaryVolume
              ? t("{{size}} 여유", { size: formatBytes(primaryVolume.availableBytes) })
              : loading
                ? `${t("디스크 확인 중")}…`
                : t("드라이브 정보를 읽지 못했습니다")}
          </h2>
          {primaryVolume ? (
            <dl className="dashboard-hero__metrics">
              <div>
                <dt>{t("사용 중")}</dt>
                <dd>{formatBytes(usedBytes)}</dd>
              </div>
              <div>
                <dt>{t("남음")}</dt>
                <dd>{formatBytes(primaryVolume.availableBytes)}</dd>
              </div>
              <div>
                <dt>{t("사용")}</dt>
                <dd>{Math.round(usedPercent)}%</dd>
              </div>
            </dl>
          ) : (
            <p>{t("새로 고침한 뒤에도 보이지 않으면 운영체제 권한을 확인해 주세요.")}</p>
          )}
        </div>

        <div className="dashboard-hero__visual">
          <div
            ref={driveDeckRef}
            className={`dashboard-drive-deck ${otherVolumes.length === 0 ? "is-single" : ""}`}
            role="group"
            aria-label={t("드라이브 선택")}
          >
            <div
              key={primaryVolumeKey ?? "empty-drive"}
              ref={activeDriveRef}
              className="dashboard-drive-feature"
              tabIndex={primaryVolume ? -1 : undefined}
              data-dashboard-drive-key={primaryVolumeKey ?? undefined}
            >
              {primaryVolume ? (
                <div className="dashboard-drive-feature__identity">
                  <span className="dashboard-drive-feature__icon" aria-hidden="true">
                    <HardDrive size={18} />
                  </span>
                  <span>
                    <strong>{volumeLabel(primaryVolume)}</strong>
                    <small>
                      {primaryVolume.mountPoint}
                      {primaryVolume.fileSystem ? ` · ${primaryVolume.fileSystem}` : ""}
                    </small>
                  </span>
                  <span className="dashboard-drive-feature__badges">
                    {primaryVolume.isSystem ? <b>{t("시스템")}</b> : null}
                    {primaryVolume.removable ? <b>{t("이동식")}</b> : null}
                  </span>
                </div>
              ) : null}
              <StorageRing
                volume={primaryVolume}
                report={activeReport}
                scanning={scanning}
                loading={loading}
                status={ringStatus}
              />
            </div>

            {otherVolumes.length > 0 ? (
              <div className="dashboard-drive-rail" aria-label={t("다른 드라이브")}>
                {otherVolumes.map((volume) => (
                  <DriveSelectorCard
                    key={dashboardVolumeKey(volume)}
                    volume={volume}
                    disabled={blocked}
                    onSelect={() => selectDashboardVolume(volume)}
                  />
                ))}
              </div>
            ) : null}
          </div>
          <span className="sr-only" role="status" aria-live="polite">
            {primaryVolume
              ? t("{{name}} 드라이브를 크게 표시합니다", {
                  name: volumeLabel(primaryVolume),
                })
              : ""}
          </span>
        </div>

        <div className="dashboard-hero__actions">
          <div className="dashboard-hero__action-copy">
            <strong>{t("큰 파일과 중복 파일을 한 번에 검사하세요")}</strong>
            <p>{t("한 번의 자세한 검사로 큰 파일 목록과 실제 중복 결과를 함께 만듭니다.")}</p>
          </div>
          <div className="dashboard-hero__buttons">
            {scanning ? (
              <button
                className="secondary-button danger-outline"
                type="button"
                onClick={onCancelScan}
              >
                <X size={17} aria-hidden="true" />
                {t("검사 중단")}
              </button>
            ) : (
              <button
                className="primary-button"
                type="button"
                disabled={blocked || loading || !primaryVolume}
                onClick={() => {
                  if (primaryVolume) onStartScan(primaryVolume);
                }}
              >
                <Search size={18} aria-hidden="true" />
                {activeReport ? t("다시 스캔") : t("이 드라이브 검사")}
              </button>
            )}
            {cleanupScanning ? (
              <button
                className="secondary-button danger-outline"
                type="button"
                onClick={onCancelCleanupScan}
              >
                <X size={17} aria-hidden="true" />
                {t("정리 후보 검사 중단")}
              </button>
            ) : (
              <button
                className="secondary-button"
                type="button"
                disabled={blocked || loading}
                onClick={onStartCleanupScan}
              >
                <ListChecks size={18} aria-hidden="true" />
                {cleanupReport ? t("정리 후보 다시 찾기") : t("정리 후보 찾기")}
              </button>
            )}
          </div>
          <p className="dashboard-hero__trust">
            <ShieldCheck size={16} aria-hidden="true" />
                    {t("스캔은 파일을 수정하거나 이동하지 않습니다.")}
                    {" "}{t("클라우드 동기화 폴더와 온라인 전용 항목은 검사에서 제외합니다.")}
          </p>
        </div>

        {activeMessage ? (
          <div className="dashboard-hero__progress">
            <div>
              <strong role="status" aria-live="polite">{activeMessage}</strong>
              <span>
                {t("{{count}}개 · {{size}} 확인", {
                  count: formatCount(activeCount),
                  size: formatBytes(activeBytes),
                })}
              </span>
            </div>
            <progress aria-label={activeMessage} max={1} value={activeFraction ?? undefined} />
          </div>
        ) : scanState === "cancelled" || cleanupState === "cancelled" ? (
          <p className="dashboard-hero__notice" role="status">
            {t("검사를 취소했습니다.")}
          </p>
        ) : null}

        {scanError || cleanupError ? (
          <div className="dashboard-action-errors">
            {scanError ? (
              <div className="dashboard-action-error" role="alert">
                <AlertTriangle size={17} aria-hidden="true" />
                <span>
                  <strong>{t("전체 검사를 완료하지 못했습니다")}</strong>
                  <small>{scanError}</small>
                </span>
                <button
                  type="button"
                  disabled={blocked || !primaryVolume}
                  onClick={() => {
                    if (primaryVolume) onStartScan(primaryVolume);
                  }}
                >
                  {t("다시 시도")}
                </button>
              </div>
            ) : null}
            {cleanupError ? (
              <div className="dashboard-action-error" role="alert">
                <AlertTriangle size={17} aria-hidden="true" />
                <span>
                  <strong>{t("정리 후보 스캔을 완료하지 못했습니다")}</strong>
                  <small>{cleanupError}</small>
                </span>
                <button type="button" disabled={blocked} onClick={onStartCleanupScan}>
                  {t("다시 시도")}
                </button>
              </div>
            ) : null}
          </div>
        ) : null}
      </section>

      <section className="dashboard-quick-section" aria-labelledby="dashboard-quick-title">
        <div className="dashboard-section-heading">
          <div>
            <p className="eyebrow">{t("빠른 실행")}</p>
            <h2 id="dashboard-quick-title">{t("필요한 결과로 바로 이동하세요")}</h2>
          </div>
        </div>
        <div className="dashboard-quick-grid">
          <QuickAction
            modifier="cleanup"
            icon={<ListChecks size={25} aria-hidden="true" />}
            title={t("정리 후보")}
            detail={cleanupReport
              ? t("{{count}}개 · {{size}}", {
                  count: formatCount(likelySafeCandidates.length),
                  size: formatBytes(likelySafeBytes),
                })
              : t("임시 파일과 삭제 후 남은 흔적")}
            onClick={onOpenCleanup}
          />
          <QuickAction
            modifier="large-files"
            icon={<HardDrive size={25} aria-hidden="true" />}
            title={t("큰 파일")}
            detail={report
              ? t("{{count}}개 · {{size}}", {
                  count: formatCount(report.largeFiles.length),
                  size: formatBytes(largeBytes),
                })
              : t("크기와 위치를 나란히 비교합니다.")}
            onClick={onOpenLargeFiles}
          />
          <QuickAction
            modifier="duplicates"
            icon={<Copy size={25} aria-hidden="true" />}
            title={t("중복 파일")}
            detail={report
              ? t("{{count}}그룹 · {{size}}", {
                  count: formatCount(report.duplicateGroups.length),
                  size: formatBytes(report.duplicateWasteBytes),
                })
              : t("파일 내용을 끝까지 비교해 실제로 같은 결과만 표시합니다.")}
            onClick={onOpenDuplicates}
          />
          <QuickAction
            modifier="files"
            icon={<Search size={25} aria-hidden="true" />}
            title={t("파일 이름 찾기")}
            detail={fileCatalog
              ? t("{{count}}개 · {{size}}", {
                  count: formatCount(fileCatalog.indexedFiles),
                  size: formatBytes(fileCatalog.indexedBytes),
                })
              : t("이름과 위치로 찾기")}
            onClick={onOpenFileSearch}
          />
        </div>
      </section>

      {report || cleanupReport ? (
        <section className="dashboard-evidence" aria-labelledby="dashboard-evidence-title">
          <div className="dashboard-section-heading">
            <div>
              <p className="eyebrow">{t("검사 결과")}</p>
              <h2 id="dashboard-evidence-title">{t("실제 보고서 요약")}</h2>
            </div>
            <span>{t("결과 용량은 서로 합산하지 않습니다")}</span>
          </div>
          <div className="dashboard-evidence-grid">
            {report ? (
              <>
                <ResultMetric
                  icon={<HardDrive size={19} aria-hidden="true" />}
                  label={t("큰 파일")}
                  value={formatBytes(largeBytes)}
                  detail={t("{{count}}개", { count: formatCount(report.largeFiles.length) })}
                  completedAt={report.completedAtUnixMs}
                  onClick={onOpenLargeFiles}
                />
                <ResultMetric
                  icon={<Copy size={19} aria-hidden="true" />}
                  label={t("검증된 중복")}
                  value={formatBytes(report.duplicateWasteBytes)}
                  detail={t("{{count}}개", { count: formatCount(report.duplicateGroups.length) })}
                  completedAt={report.completedAtUnixMs}
                  onClick={onOpenDuplicates}
                />
              </>
            ) : null}
            {cleanupReport ? (
              <>
                <ResultMetric
                  icon={<ShieldCheck size={19} aria-hidden="true" />}
                  label={t("정리 가능성 높음")}
                  value={formatBytes(likelySafeBytes)}
                  detail={t("{{count}}개", { count: formatCount(likelySafeCandidates.length) })}
                  completedAt={cleanupReport.completedAtUnixMs}
                  onClick={onOpenCleanup}
                />
                <ResultMetric
                  icon={<AlertTriangle size={19} aria-hidden="true" />}
                  label={t("검토 필요")}
                  value={formatBytes(reviewBytes)}
                  detail={t("{{count}}개", { count: formatCount(reviewCandidates.length) })}
                  completedAt={cleanupReport.completedAtUnixMs}
                  onClick={onOpenCleanup}
                />
              </>
            ) : null}
          </div>
        </section>
      ) : null}

      <div className="dashboard-activity-grid">
        <section className="dashboard-panel dashboard-history" aria-labelledby="dashboard-history-title">
          <DashboardHeading
            title={t("최근 정리")}
            detail={t("운영체제 휴지통으로 이동한 논리 용량")}
            icon={<Clock3 size={18} aria-hidden="true" />}
            id="dashboard-history-title"
          />
          {actionHistory?.entries.length ? (
            <div className="dashboard-history-list">
              {actionHistory.entries.map((entry) => (
                <HistoryRow key={entry.operationId} entry={entry} />
              ))}
            </div>
          ) : (
            <DashboardEmpty
              title={t("아직 정리 기록이 없습니다")}
              detail={t("파일을 선택하고 최종 확인한 뒤 휴지통으로 옮긴 결과만 여기에 남습니다.")}
              actionLabel={t("용량 관리 열기")}
              onAction={onOpenStorage}
            />
          )}
          {actionHistory?.issues.length ? (
            <p className="dashboard-panel-note has-warning">
              {t("최근 기록 일부를 읽지 못했습니다. 중단된 작업은 위쪽 복구 안내에서 확인하세요.")}
            </p>
          ) : null}
        </section>

        <section className="dashboard-panel dashboard-recent" aria-labelledby="dashboard-recent-title">
          <DashboardHeading
            title={t("최근 추가된 파일")}
            detail={t("BroomSweepy가 이전 목록 이후 새로 발견")}
            icon={<FilePlus2 size={18} aria-hidden="true" />}
            id="dashboard-recent-title"
          />
          {fileCatalogStale ? (
            <p className="dashboard-panel-note has-warning">
              {t("파일을 휴지통으로 옮긴 뒤 목록이 오래됐습니다. 새로 고쳐야 최근 파일이 정확합니다.")}
            </p>
          ) : null}
          {!fileCatalog || !recentFiles ? (
            <DashboardEmpty
              title={t("먼저 파일 목록을 만들어 주세요")}
              detail={t("첫 목록은 비교 기준으로만 저장하며 기존 파일을 모두 새 파일로 표시하지 않습니다.")}
              actionLabel={t("파일 목록 만들기")}
              onAction={onRefreshFileCatalog}
              disabled={blocked}
            />
          ) : !recentFiles.comparisonReady ? (
            <DashboardEmpty
              title={t("비교 기준 목록이 준비됐습니다")}
              detail={t("마지막 목록 {{date}}. 다음 갱신부터 새로 발견한 파일을 표시합니다.", {
                date: formatDate(recentFiles.completedAtUnixMs),
              })}
              actionLabel={t("목록 새로 고침")}
              onAction={onRefreshFileCatalog}
              disabled={blocked}
            />
          ) : recentFiles.results.length > 0 ? (
            <>
              <div className="dashboard-file-list">
                {recentFiles.results.map((file) => (
                  <button
                    type="button"
                    className="dashboard-file-row"
                    key={`${file.path}-${file.firstSeenAtUnixMs}`}
                    title={`${file.path}\n${t("선택하면 파일 위치를 엽니다")}`}
                    onClick={() => onRevealFile(file.path)}
                  >
                    <span>
                      <strong>{file.name}</strong>
                      <small title={file.parent}>
                        {t("{{parent}} · {{date}} 발견", {
                          parent: file.parent,
                          date: formatDate(file.firstSeenAtUnixMs),
                        })}
                      </small>
                    </span>
                    <b>{formatBytes(file.logicalBytes)}</b>
                  </button>
                ))}
              </div>
              <div className="dashboard-list-footer">
                <span>
                  {t("새 파일 {{count}}개", { count: formatCount(recentFiles.totalNewFiles) })}
                  {recentFiles.resultsTruncated ? ` · ${t("최근 항목만 표시")}` : ""}
                </span>
                <button type="button" onClick={onOpenFileSearch}>
                  {t("파일 찾기")}
                  <ArrowRight size={15} aria-hidden="true" />
                </button>
              </div>
            </>
          ) : (
            <DashboardEmpty
              title={t("이전 목록 이후 새 파일이 없습니다")}
              detail={t("마지막 비교 {{date}}", { date: formatDate(recentFiles.completedAtUnixMs) })}
              actionLabel={t("목록 새로 고침")}
              onAction={onRefreshFileCatalog}
              disabled={blocked}
            />
          )}
        </section>
      </div>
    </div>
  );
}

function QuickAction({
  modifier,
  icon,
  title,
  detail,
  onClick,
}: {
  modifier: string;
  icon: ReactNode;
  title: string;
  detail: string;
  onClick: () => void;
}) {
  return (
    <button className={`dashboard-quick-action is-${modifier}`} type="button" onClick={onClick}>
      <span className="dashboard-quick-action__icon" aria-hidden="true">{icon}</span>
      <span>
        <strong>{title}</strong>
        <small>{detail}</small>
      </span>
      <ArrowRight size={17} aria-hidden="true" />
    </button>
  );
}

function ResultMetric({
  icon,
  label,
  value,
  detail,
  completedAt,
  onClick,
}: {
  icon: ReactNode;
  label: string;
  value: string;
  detail: string;
  completedAt: number;
  onClick: () => void;
}) {
  const { t } = useLanguage();
  return (
    <button className="dashboard-result-metric" type="button" onClick={onClick}>
      <span className="dashboard-result-metric__icon" aria-hidden="true">{icon}</span>
      <span>
        <small>{label}</small>
        <strong>{value}</strong>
        <span>{detail}</span>
      </span>
      <time dateTime={formatDateTimeAttribute(completedAt)}>
        {formatDate(completedAt)} · {t("완료")}
      </time>
      <ArrowRight size={17} aria-hidden="true" />
    </button>
  );
}

interface DashboardHeadingProps {
  id: string;
  title: string;
  detail: string;
  icon: ReactNode;
}

function DashboardHeading({ id, title, detail, icon }: DashboardHeadingProps) {
  return (
    <div className="dashboard-panel-heading">
      <span aria-hidden="true">{icon}</span>
      <div>
        <h2 id={id}>{title}</h2>
        <p>{detail}</p>
      </div>
    </div>
  );
}

function DriveSelectorCard({
  volume,
  disabled,
  onSelect,
}: {
  volume: VolumeInfo;
  disabled: boolean;
  onSelect: () => void;
}) {
  const { t } = useLanguage();
  const usedBytes = Math.max(0, volume.totalBytes - volume.availableBytes);
  const usedPercent = volume.totalBytes > 0
    ? Math.min(100, (usedBytes / volume.totalBytes) * 100)
    : 0;
  const warning = usedPercent >= 85;
  const label = volumeLabel(volume);
  const roundedPercent = Math.round(usedPercent);

  return (
    <button
      className={`dashboard-drive-card ${warning ? "has-warning" : ""}`}
      type="button"
      disabled={disabled}
      data-dashboard-drive-key={dashboardVolumeKey(volume)}
      aria-label={`${t("{{name}} 선택", { name: label })}. ${t("{{percent}}% 사용", {
        percent: roundedPercent,
      })}`}
      title={`${label}\n${volume.mountPoint}`}
      style={{
        "--used-angle": `${usedPercent * 3.6}deg`,
      } as CSSProperties}
      onClick={onSelect}
    >
      <span className="dashboard-drive-card__ring" aria-hidden="true">
        <strong>{roundedPercent}%</strong>
      </span>
      <span className="dashboard-drive-card__copy">
        <strong>{label}</strong>
        <small>
          {t("{{size}} 여유", { size: formatBytes(volume.availableBytes) })}
        </small>
      </span>
      {volume.isSystem || volume.removable ? (
        <span className="dashboard-drive-card__badge" aria-hidden="true">
          {volume.isSystem ? t("시스템") : t("이동식")}
        </span>
      ) : null}
    </button>
  );
}

function volumeLabel(volume: VolumeInfo): string {
  return volume.name && volume.name !== volume.mountPoint
    ? volume.name
    : volume.mountPoint;
}

function captureDriveRects(deck: HTMLDivElement | null): Map<string, DOMRect> {
  const rects = new Map<string, DOMRect>();
  deck?.querySelectorAll<HTMLElement>("[data-dashboard-drive-key]").forEach((element) => {
    const key = element.dataset.dashboardDriveKey;
    if (key) rects.set(key, element.getBoundingClientRect());
  });
  return rects;
}

function HistoryRow({ entry }: { entry: ActionHistoryEntry }) {
  const { t } = useLanguage();
  const title = entry.actionKind === "duplicateFiles"
    ? t("중복 파일 정리")
    : entry.actionKind === "cleanupCandidates"
      ? t("정리 후보 이동")
      : t("파일 정리");
  const status = entry.cancelled
    ? t("사용자 취소")
    : entry.stoppedEarly
      ? t("일부만 완료")
      : t("완료");

  return (
    <div className="dashboard-history-row">
      <time dateTime={formatDateTimeAttribute(entry.completedAtUnixMs)}>
        {formatDate(entry.completedAtUnixMs)}
      </time>
      <span>
        <strong>{title}</strong>
        <small>
          {t("요청 {{requested}}개 중 {{moved}}개 이동 · {{status}}", {
            requested: formatCount(entry.requestedCount),
            moved: formatCount(entry.movedCount),
            status,
          })}
        </small>
      </span>
      <b>{formatBytes(entry.movedBytes)}</b>
    </div>
  );
}

function DashboardEmpty({
  title,
  detail,
  actionLabel,
  onAction,
  disabled = false,
}: {
  title: string;
  detail: string;
  actionLabel: string;
  onAction: () => void;
  disabled?: boolean;
}) {
  return (
    <div className="dashboard-empty">
      <FolderSearch size={20} aria-hidden="true" />
      <span>
        <strong>{title}</strong>
        <small>{detail}</small>
      </span>
      <button type="button" disabled={disabled} onClick={onAction}>{actionLabel}</button>
    </div>
  );
}
