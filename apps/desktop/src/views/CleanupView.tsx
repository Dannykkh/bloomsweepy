import {
  AlertTriangle,
  CheckCircle2,
  Clock3,
  Database,
  FolderSearch,
  HardDrive,
  Search,
  ShieldAlert,
  Trash2,
  X,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { SafetyActionDialog } from "../components/SafetyActionDialog";
import { TrashResultPanel } from "../components/TrashResultPanel";
import { revealPath } from "../lib/bridge";
import {
  formatBytes,
  formatCount,
  formatDate,
  formatDuration,
} from "../lib/format";
import type {
  CleanupCandidate,
  CleanupCandidateKind,
  CleanupScanProgress,
  CleanupScanReport,
  CleanupTrashRequest,
  ScanUiState,
  TrashOperationResult,
  TrashProgress,
} from "../types";
import { useLanguage, type MessageKey } from "../i18n";

interface CleanupViewProps {
  platform: string | null;
  report: CleanupScanReport | null;
  progress: CleanupScanProgress | null;
  state: ScanUiState;
  error: string | null;
  blocked: boolean;
  actionRunning: boolean;
  actionProgress: TrashProgress | null;
  actionResult: TrashOperationResult | null;
  actionError: string | null;
  onStart: () => void;
  onCancel: () => void;
  onMoveToTrash: (request: CleanupTrashRequest) => Promise<TrashOperationResult>;
  onCancelAction: () => void;
}

type CleanupFilter = "all" | CleanupCandidateKind;

const cleanupPresentation: Record<
  CleanupCandidateKind,
  { label: MessageKey; description: MessageKey }
> = {
  temporaryEntry: {
    label: "오래된 임시 파일",
    description: "최근 사용 흔적이 없는 사용자 임시 항목",
  },
  appDataDirectory: {
    label: "프로그램 설정 폴더",
    description: "설치 앱과 이름이 맞지 않는 오래된 데이터",
  },
  cacheDirectory: {
    label: "오래된 캐시",
    description: "운영체제가 지정한 캐시 위치",
  },
};

export function CleanupView({
  platform,
  report,
  progress,
  state,
  error,
  blocked,
  actionRunning,
  actionProgress,
  actionResult,
  actionError,
  onStart,
  onCancel,
  onMoveToTrash,
  onCancelAction,
}: CleanupViewProps) {
  const { t } = useLanguage();
  const [filter, setFilter] = useState<CleanupFilter>("all");
  const [revealMessage, setRevealMessage] = useState<string | null>(null);
  const [selectedPaths, setSelectedPaths] = useState<Set<string>>(new Set());
  const [dialogOpen, setDialogOpen] = useState(false);
  const [dialogError, setDialogError] = useState<string | null>(null);
  const scanning = state === "scanning";

  useEffect(() => {
    setSelectedPaths(new Set());
    setDialogOpen(false);
    setDialogError(null);
  }, [report?.completedAtUnixMs]);
  const visibleCandidates = useMemo(
    () =>
      report?.candidates.filter(
        (candidate) => filter === "all" || candidate.kind === filter,
      ) ?? [],
    [filter, report],
  );
  const likelySafeCount =
    report?.candidates.filter((candidate) => candidate.confidence === "likelySafe")
      .length ?? 0;
  const reviewCount =
    (report?.candidates.length ?? 0) - likelySafeCount +
    (report?.registryResidues.candidates.length ?? 0);
  const selectedCandidates = useMemo(
    () => report?.candidates.filter((candidate) => selectedPaths.has(candidate.path)) ?? [],
    [report, selectedPaths],
  );
  const selectedBytes = selectedCandidates.reduce(
    (total, candidate) => total + candidate.logicalBytes,
    0,
  );
  const selectedReviewCount = selectedCandidates.filter(
    (candidate) => candidate.confidence === "review",
  ).length;

  async function revealCandidate(candidate: CleanupCandidate) {
    try {
      await revealPath(candidate.path);
      setRevealMessage(t("{{name}} 위치를 파일 탐색기에서 표시했습니다.", { name: candidate.name }));
    } catch (reason) {
      const message = reason instanceof Error ? reason.message : String(reason);
      setRevealMessage(t("{{name}} 위치를 표시하지 못했습니다: {{detail}}", {
        name: candidate.name,
        detail: message,
      }));
    }
  }

  function updateSelection(candidate: CleanupCandidate, selected: boolean) {
    setSelectedPaths((current) => {
      const next = new Set(current);
      if (selected) next.add(candidate.path);
      else next.delete(candidate.path);
      return next;
    });
  }

  async function confirmTrashMove(reviewAcknowledged: boolean) {
    if (selectedPaths.size === 0) return;
    setDialogError(null);
    try {
      await onMoveToTrash({
        paths: [...selectedPaths],
        allowReviewCandidates: selectedReviewCount > 0 && reviewAcknowledged,
      });
      setDialogOpen(false);
    } catch (reason) {
      setDialogError(normalizeError(reason, t("휴지통 이동을 완료하지 못했습니다")));
    }
  }

  if (!report && !scanning && actionResult) {
    return <TrashResultPanel result={actionResult} onRescan={onStart} />;
  }

  return (
    <div className="view-stack cleanup-view">
      <section
        className={`cleanup-state ${error ? "is-error" : ""}`}
        aria-live="polite"
      >
        <span className="cleanup-state__icon" aria-hidden="true">
          {error ? (
            <AlertTriangle size={21} />
          ) : scanning ? (
            <Search size={21} />
          ) : report ? (
            <CheckCircle2 size={21} />
          ) : (
            <FolderSearch size={21} />
          )}
        </span>
        <div>
          <strong>
            {error
              ? t("정리 후보 스캔을 완료하지 못했습니다")
              : scanning
                ? t("남은 파일과 제거 정보를 대조하고 있습니다")
                : report
                  ? t("정리 후보 분석이 완료됐습니다")
                  : t("삭제 후 남은 흔적을 근거별로 찾습니다")}
          </strong>
          <p>
            {error
              ? error
              : scanning
                ? t("정리 후보 위치를 준비하고 있습니다")
                : report
                  ? t("{{duration}} 동안 {{count}}개 항목을 확인했습니다.", {
                      duration: formatDuration(report.durationMs),
                      count: formatCount(report.processedEntries),
                    })
                  : platform === "windows"
                    ? t("Windows 임시 폴더, 프로그램 설정 폴더, 설치 기록을 바꾸지 않고 서로 비교합니다.")
                    : t("임시 파일과 오래된 캐시를 읽기 전용으로 확인합니다.")}
          </p>
        </div>
        <div className="cleanup-state__actions">
          {scanning ? (
            <button className="secondary-button danger-outline" type="button" onClick={onCancel}>
              <X size={16} aria-hidden="true" />
              {t("스캔 취소")}
            </button>
          ) : (
            <button className="primary-button" type="button" disabled={blocked} onClick={onStart}>
              <Search size={16} aria-hidden="true" />
              {report ? t("다시 스캔") : t("정리 후보 스캔")}
            </button>
          )}
        </div>
      </section>

      {scanning ? (
        <section className="cleanup-progress" role="status">
          <div>
            <span>{t("확인한 항목")}</span>
            <strong>{formatCount(progress?.processedEntries ?? 0)}</strong>
          </div>
          <div>
            <span>{t("확인한 용량")}</span>
            <strong>{formatBytes(progress?.processedBytes ?? 0)}</strong>
          </div>
          <div>
            <span>{t("발견 후보")}</span>
            <strong>{formatCount(progress?.candidatesFound ?? 0)}</strong>
          </div>
          <div>
            <span>{t("위치 진행")}</span>
            <strong>
              {formatCount(progress?.processedRoots ?? 0)} / {formatCount(progress?.totalRoots ?? 0)}
            </strong>
          </div>
        </section>
      ) : null}

      {report ? (
        <>
          <section className="cleanup-summary" aria-label={t("정리 후보 요약")}>
            <CleanupMetric
              label={t("정리 가능성 높음")}
              value={t("{{count}}개", { count: formatCount(likelySafeCount) })}
              detail={t("오래된 임시 파일·캐시")}
              icon={<Clock3 size={18} />}
            />
            <CleanupMetric
              label={t("검토 필요")}
              value={t("{{count}}개", { count: formatCount(reviewCount) })}
              detail={t("프로그램 설정 폴더·설치 기록")}
              icon={<ShieldAlert size={18} />}
            />
            <CleanupMetric
              label={t("후보 용량")}
              value={formatBytes(report.candidateBytes)}
              detail={t("Windows 설치 기록 제외")}
              icon={<HardDrive size={18} />}
            />
          </section>

          <section className="cleanup-safety-bar">
            <ShieldAlert size={19} aria-hidden="true" />
            <div>
              <strong>{t("선택한 파일 후보만 운영체제 휴지통으로 이동합니다")}</strong>
              <p>{t("옮기기 직전에 파일이 바뀌지 않았는지 다시 확인하고 작업 기록을 남깁니다. 프로그램 설정 폴더는 한 번 더 확인해야 하며 Windows 설치 정보는 바꾸지 않습니다.")}</p>
            </div>
          </section>

          <section className="cleanup-toolbar" aria-label={t("정리 후보 필터")}>
            <div className="segmented-control" role="group" aria-label={t("정리 후보 종류")}>
              {(
                [
                  ["all", t("전체")],
                  ["temporaryEntry", t("임시 파일")],
                  ["appDataDirectory", t("프로그램 설정")],
                  ["cacheDirectory", t("캐시")],
                ] as const
              ).map(([value, label]) => (
                <button
                  type="button"
                  className={filter === value ? "is-active" : ""}
                  aria-pressed={filter === value}
                  key={value}
                  onClick={() => setFilter(value)}
                >
                  {label}
                </button>
              ))}
            </div>
            <span>{t("{{count}}개 표시", { count: formatCount(visibleCandidates.length) })}</span>
          </section>

          {selectedPaths.size > 0 ? (
            <section className="trash-action-bar" aria-label={t("휴지통 이동 선택 요약")}>
              <div>
                <span>{t("선택")}</span>
                <strong>{t("{{count}}개 · {{size}}", {
                  count: formatCount(selectedPaths.size),
                  size: formatBytes(selectedBytes),
                })}</strong>
                <small>
                  {selectedReviewCount > 0
                    ? t("한 번 더 확인할 프로그램 설정 {{count}}개 포함", { count: formatCount(selectedReviewCount) })
                    : t("이동 직전 모든 항목을 다시 검사합니다.")}
                </small>
              </div>
              <button
                className="trash-action-button"
                type="button"
                disabled={actionRunning || blocked}
                onClick={() => {
                  setDialogError(null);
                  setDialogOpen(true);
                }}
              >
                <Trash2 size={16} aria-hidden="true" />
                {t("휴지통으로 이동")}
              </button>
            </section>
          ) : null}

          <section className="cleanup-candidate-list" aria-label={t("파일 정리 후보")}>
            {visibleCandidates.length === 0 ? (
              <div className="table-empty">{t("선택한 종류의 정리 후보가 없습니다.")}</div>
            ) : (
              visibleCandidates.map((candidate) => {
                const presentation = cleanupPresentation[candidate.kind];
                return (
                  <article
                    className={`cleanup-candidate ${selectedPaths.has(candidate.path) ? "is-selected" : ""}`}
                    tabIndex={0}
                    aria-selected={selectedPaths.has(candidate.path)}
                    key={candidate.path}
                    title={t("더블클릭하여 파일 탐색기에서 위치 표시")}
                    onDoubleClick={() => void revealCandidate(candidate)}
                    onKeyDown={(event) => {
                      if (event.target === event.currentTarget && event.key === "Enter") {
                        void revealCandidate(candidate);
                      }
                    }}
                  >
                    <label
                      className="file-selection cleanup-candidate__selection"
                      title={t("휴지통으로 이동할 후보 선택")}
                      onClick={(event) => event.stopPropagation()}
                      onDoubleClick={(event) => event.stopPropagation()}
                    >
                      <input
                        type="checkbox"
                        checked={selectedPaths.has(candidate.path)}
                        disabled={actionRunning || blocked}
                        aria-label={t("{{name}} 휴지통 이동 대상으로 선택", { name: candidate.name })}
                        onChange={(event) => updateSelection(candidate, event.currentTarget.checked)}
                      />
                    </label>
                    <span
                      className={`cleanup-candidate__confidence is-${candidate.confidence}`}
                    >
                      {candidate.confidence === "likelySafe" ? t("정리 가능성 높음") : t("검토 필요")}
                    </span>
                    <div className="cleanup-candidate__identity">
                      <strong>{candidate.name}</strong>
                      <span title={candidate.path}>{candidate.path}</span>
                      <small>{t(presentation.label)} · {t(presentation.description)}</small>
                    </div>
                    <div className="cleanup-candidate__evidence">
                      {candidate.evidence.map((evidence) => (
                        <span key={evidence}>{evidence}</span>
                      ))}
                    </div>
                    <div className="cleanup-candidate__metric">
                      <strong>{formatBytes(candidate.logicalBytes)}</strong>
                      <span>{t("{{count}}개 항목", { count: formatCount(candidate.entryCount) })}</span>
                      <time>{formatDate(candidate.modifiedAtUnixMs)}</time>
                    </div>
                  </article>
                );
              })
            )}
            {revealMessage ? <p className="cleanup-reveal-status" role="status">{revealMessage}</p> : null}
          </section>

          {platform === "windows" ? (
            <section className="registry-residue-panel">
              <div className="section-heading">
                <div>
                  <p className="eyebrow">{t("삭제 후 남은 흔적")}</p>
                  <h2>{t("깨진 제거 프로그램 정보")}</h2>
                </div>
                <span>{t("{{count}}개 검토 대상", { count: formatCount(report.registryResidues.candidates.length) })}</span>
              </div>
              {report.registryResidues.candidates.length === 0 ? (
                <div className="table-empty">{t("서로 다른 경로 증거가 두 개 이상 끊긴 제거 정보가 없습니다.")}</div>
              ) : (
                <div className="registry-residue-list">
                  {report.registryResidues.candidates.map((candidate) => (
                    <article key={candidate.registryPath}>
                      <Database size={17} aria-hidden="true" />
                      <div>
                        <strong>{candidate.displayName}</strong>
                        <span title={candidate.registryPath}>{candidate.registryPath}</span>
                        {candidate.evidence.map((evidence) => (
                          <small key={evidence}>{evidence}</small>
                        ))}
                      </div>
                      <span>{candidate.registryScope === "machine" ? t("컴퓨터") : t("사용자")}</span>
                    </article>
                  ))}
                </div>
              )}
            </section>
          ) : null}

          {report.limitReached ? (
            <div className="notice-panel" role="alert">
              <AlertTriangle size={18} aria-hidden="true" />
              <p>{t("스캔 안전 한도에 도달해 일부 항목은 생략했습니다. 현재 결과를 삭제 판단의 전체 목록으로 사용하면 안 됩니다.")}</p>
            </div>
          ) : null}
        </>
      ) : !scanning ? (
        <>
          {actionError ? (
            <div className="notice-panel trash-action-error" role="alert">
              <AlertTriangle size={18} aria-hidden="true" />
              <p>{actionError} {t("기존 결과는 안전을 위해 폐기했습니다. 다시 스캔하세요.")}</p>
            </div>
          ) : null}
          <div className="empty-panel empty-panel--page">
            <FolderSearch size={28} aria-hidden="true" />
            <strong>{t("아직 정리 후보를 확인하지 않았습니다")}</strong>
            <p>{t("최근 사용 시각, 설치 앱 인벤토리, 경로 존재 여부를 함께 대조해 단순 파일명보다 보수적으로 분류합니다.")}</p>
            <button className="primary-button" type="button" disabled={blocked} onClick={onStart}>
              <Search size={17} aria-hidden="true" />
              {t("정리 후보 스캔")}
            </button>
          </div>
        </>
      ) : null}

      <SafetyActionDialog
        open={dialogOpen}
        title={t("선택한 정리 후보를 휴지통으로 이동할까요?")}
        itemCount={selectedPaths.size}
        logicalBytes={selectedBytes}
        reviewCount={selectedReviewCount}
        busy={actionRunning}
        progress={actionProgress}
        error={dialogError ?? actionError}
        onConfirm={(reviewAcknowledged) => void confirmTrashMove(reviewAcknowledged)}
        onCancel={onCancelAction}
        onClose={() => setDialogOpen(false)}
      />
    </div>
  );
}

interface CleanupMetricProps {
  label: string;
  value: string;
  detail: string;
  icon: React.ReactNode;
}

function CleanupMetric({ label, value, detail, icon }: CleanupMetricProps) {
  return (
    <div className="cleanup-metric">
      <span aria-hidden="true">{icon}</span>
      <div>
        <small>{label}</small>
        <strong>{value}</strong>
        <span>{detail}</span>
      </div>
    </div>
  );
}

function normalizeError(reason: unknown, fallback: string): string {
  if (reason instanceof Error) return reason.message;
  return typeof reason === "string" ? reason : fallback;
}
