import {
  AlertTriangle,
  ArrowRight,
  Clock3,
  FilePlus2,
  FolderSearch,
  HardDrive,
  ListRestart,
  RefreshCw,
} from "lucide-react";
import type { CSSProperties, ReactNode } from "react";
import { formatBytes, formatCount, formatDate, formatDateTimeAttribute } from "../lib/format";
import type {
  ActionHistoryEntry,
  ActionHistoryReport,
  FileCatalogRecentReport,
  FileCatalogStatus,
  SystemOverview,
  VolumeInfo,
} from "../types";
import { visibleDashboardVolumes } from "../lib/cloudVolumePolicy";
import { useLanguage } from "../i18n";

interface DashboardViewProps {
  system: SystemOverview | null;
  actionHistory: ActionHistoryReport | null;
  recentFiles: FileCatalogRecentReport | null;
  fileCatalog: FileCatalogStatus | null;
  fileCatalogStale: boolean;
  loading: boolean;
  error: string | null;
  blocked: boolean;
  onRefresh: () => void;
  onOpenVolume: (volume: VolumeInfo) => void;
  onOpenStorage: () => void;
  onRefreshFileCatalog: () => void;
  onOpenFileSearch: () => void;
  onRevealFile: (path: string) => void;
}

export function DashboardView({
  system,
  actionHistory,
  recentFiles,
  fileCatalog,
  fileCatalogStale,
  loading,
  error,
  blocked,
  onRefresh,
  onOpenVolume,
  onOpenStorage,
  onRefreshFileCatalog,
  onOpenFileSearch,
  onRevealFile,
}: DashboardViewProps) {
  const { t } = useLanguage();
  const volumes = visibleDashboardVolumes(system?.volumes ?? []);

  return (
    <div className="view-stack dashboard-view">
      <div className="dashboard-toolbar">
        <p>
          <Clock3 size={16} aria-hidden="true" />
          {t("운영체제 용량, 완료된 휴지통 기록, 마지막 파일 목록을 함께 봅니다.")}
        </p>
        <button
          className="secondary-button"
          type="button"
          disabled={loading || blocked}
          onClick={onRefresh}
        >
          <RefreshCw size={16} aria-hidden="true" />
          {loading ? t("확인 중") : t("새로 고침")}
        </button>
      </div>

      {error ? (
        <div className="dashboard-inline-error" role="alert">
          <AlertTriangle size={17} aria-hidden="true" />
          <span>{error}</span>
          <button type="button" onClick={onRefresh}>{t("다시 시도")}</button>
        </div>
      ) : null}

      <section className="dashboard-panel dashboard-volumes" aria-labelledby="dashboard-volumes-title">
        <DashboardHeading
          title={t("드라이브 용량")}
          detail={t("운영체제가 보고한 현재 값")}
          icon={<HardDrive size={18} aria-hidden="true" />}
          id="dashboard-volumes-title"
        />
        {volumes.length > 0 ? (
          <div className="dashboard-volume-list">
            {volumes.map((volume) => (
              <VolumeRow
                key={`${volume.mountPoint}-${volume.name}`}
                volume={volume}
                disabled={blocked}
                onOpen={() => onOpenVolume(volume)}
              />
            ))}
          </div>
        ) : (
          <DashboardEmpty
            title={loading ? t("드라이브를 확인하고 있습니다") : t("드라이브 정보를 읽지 못했습니다")}
            detail={t("새로 고침한 뒤에도 보이지 않으면 운영체제 권한을 확인해 주세요.")}
            actionLabel={t("다시 확인")}
            onAction={onRefresh}
          />
        )}
      </section>

      <div className="dashboard-activity-grid">
        <section className="dashboard-panel dashboard-history" aria-labelledby="dashboard-history-title">
          <DashboardHeading
            title={t("최근 정리")}
            detail={t("운영체제 휴지통으로 이동한 논리 용량")}
            icon={<ListRestart size={18} aria-hidden="true" />}
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

function VolumeRow({
  volume,
  disabled,
  onOpen,
}: {
  volume: VolumeInfo;
  disabled: boolean;
  onOpen: () => void;
}) {
  const { t } = useLanguage();
  const usedBytes = Math.max(0, volume.totalBytes - volume.availableBytes);
  const usedPercent = volume.totalBytes > 0 ? Math.min(100, (usedBytes / volume.totalBytes) * 100) : 0;
  const warning = usedPercent >= 85;
  const label = volume.name && volume.name !== volume.mountPoint
    ? `${volume.name} (${volume.mountPoint})`
    : volume.mountPoint;

  return (
    <div className={`dashboard-volume-row ${warning ? "has-warning" : ""}`}>
      <span className="dashboard-volume-name">
        <strong>{label}</strong>
        <small>
          {volume.fileSystem || t("파일 시스템 정보 없음")}
          {volume.isSystem ? ` · ${t("시스템")}` : ""}
          {volume.removable ? ` · ${t("이동식")}` : ""}
        </small>
      </span>
      <span className="dashboard-volume-metric">
        <strong>{formatBytes(usedBytes)}</strong>
        <small>{t("사용 중")}</small>
      </span>
      <span className="dashboard-volume-metric">
        <strong>{formatBytes(volume.availableBytes)}</strong>
        <small>{t("남음")}</small>
      </span>
      <span className="dashboard-volume-usage">
        <span
          className="dashboard-volume-ring"
          style={{ "--used-angle": `${usedPercent * 3.6}deg` } as CSSProperties}
        >
          <strong>{Math.round(usedPercent)}%</strong>
        </span>
        <small>{t("사용")}</small>
      </span>
      <button type="button" disabled={disabled} onClick={onOpen}>
        {t("용량 보기")}
        <ArrowRight size={15} aria-hidden="true" />
      </button>
    </div>
  );
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
