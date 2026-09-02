import {
  CheckCircle2,
  ShieldAlert,
  Square,
  Trash2,
  X,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import {
  cancelDockerCleanup,
  executeDockerCleanup,
  listenToDockerCleanupProgress,
} from "../lib/bridge";
import { formatDate, formatDockerBytes } from "../lib/format";
import type {
  DockerCleanupKind,
  DockerCleanupPreview,
  DockerCleanupProgress,
  DockerCleanupResult,
  DockerManagementStatus,
} from "../types";
import { useLanguage, type MessageKey, type Translate } from "../i18n";

interface DockerCleanupDialogProps {
  preview: DockerCleanupPreview | null;
  onClose: () => void;
  onCompleted: (status: DockerManagementStatus) => void;
}

export function DockerCleanupDialog({
  preview,
  onClose,
  onCompleted,
}: DockerCleanupDialogProps) {
  const { t } = useLanguage();
  const dialogRef = useRef<HTMLDivElement>(null);
  const cancelButtonRef = useRef<HTMLButtonElement>(null);
  const [selectedKinds, setSelectedKinds] = useState<DockerCleanupKind[]>([]);
  const [acknowledged, setAcknowledged] = useState(false);
  const [busy, setBusy] = useState(false);
  const [cancelRequested, setCancelRequested] = useState(false);
  const [progress, setProgress] = useState<DockerCleanupProgress | null>(null);
  const [result, setResult] = useState<DockerCleanupResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!preview) return;
    const previouslyFocused = document.activeElement instanceof HTMLElement
      ? document.activeElement
      : null;
    const defaults = preview.items
      .filter((item) => item.defaultSelected)
      .map((item) => item.kind);
    setSelectedKinds(defaults);
    setAcknowledged(false);
    setBusy(false);
    setCancelRequested(false);
    setProgress(null);
    setResult(null);
    setError(null);
    cancelButtonRef.current?.focus();
    return () => {
      if (previouslyFocused?.isConnected) previouslyFocused.focus();
    };
  }, [preview]);

  useEffect(() => {
    if (!preview) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listenToDockerCleanupProgress((nextProgress) => {
      if (!disposed) setProgress(nextProgress);
    }).then((stopListening) => {
      if (disposed) stopListening();
      else unlisten = stopListening;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [preview]);

  useEffect(() => {
    if (!preview) return;
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        event.preventDefault();
        if (busy) void requestCancellation();
        else onClose();
        return;
      }
      if (event.key !== "Tab") return;
      const focusable = dialogRef.current?.querySelectorAll<HTMLElement>(
        'button:not([disabled]), input:not([disabled]), [tabindex]:not([tabindex="-1"])',
      );
      if (!focusable || focusable.length === 0) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    }
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [busy, onClose, preview]);

  const estimatedBytes = useMemo(() => {
    if (!preview) return 0;
    return preview.items
      .filter((item) => selectedKinds.includes(item.kind))
      .reduce((total, item) => total + item.estimatedReclaimableBytes, 0);
  }, [preview, selectedKinds]);

  if (!preview) return null;

  function toggleKind(kind: DockerCleanupKind, checked: boolean) {
    setSelectedKinds((current) => checked
      ? [...current.filter((candidate) => candidate !== kind), kind]
      : current.filter((candidate) => candidate !== kind));
    setError(null);
  }

  async function executeCleanup() {
    if (!preview || busy || result || selectedKinds.length === 0 || !acknowledged) return;
    setBusy(true);
    setCancelRequested(false);
    setError(null);
    setProgress({
      message: t("Docker 정리를 준비하고 있습니다"),
      completedSteps: 0,
      totalSteps: selectedKinds.length,
    });
    try {
      const nextResult = await executeDockerCleanup({
        previewId: preview.previewId,
        selectedKinds,
        irreversibleAcknowledged: true,
      });
      setResult(nextResult);
      onCompleted(nextResult.statusAfter);
    } catch (reason) {
      setError(normalizeDockerError(reason, t("Docker 작업을 완료하지 못했습니다")));
    } finally {
      setBusy(false);
      setCancelRequested(false);
    }
  }

  async function requestCancellation() {
    if (!busy || cancelRequested) return;
    setCancelRequested(true);
    try {
      const requested = await cancelDockerCleanup();
      if (!requested) setCancelRequested(false);
    } catch (reason) {
      setError(normalizeDockerError(reason, t("Docker 작업을 완료하지 못했습니다")));
      setCancelRequested(false);
    }
  }

  const progressFraction = progress?.totalSteps
    ? Math.min(1, progress.completedSteps / progress.totalSteps)
    : 0;

  return (
    <div
      className="safety-dialog-backdrop"
    >
      <div
        className="safety-dialog docker-cleanup-dialog"
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-busy={busy}
        aria-labelledby="docker-cleanup-title"
        aria-describedby="docker-cleanup-description"
      >
        <header>
          <span aria-hidden="true"><Trash2 size={20} /></span>
          <div>
            <p className="eyebrow">{t("Docker가 관리하는 데이터")}</p>
            <h2 id="docker-cleanup-title">{t("Docker 정리 전 확인")}</h2>
          </div>
          <button
            className="icon-button"
            type="button"
            aria-label={t("Docker 정리 확인 창 닫기")}
            disabled={busy}
            onClick={onClose}
          >
            <X size={17} aria-hidden="true" />
          </button>
        </header>

        {result ? (
          <DockerCleanupResultView result={result} />
        ) : (
          <>
            <p className="docker-cleanup-dialog__intro" id="docker-cleanup-description">
              {t("아래 명령은 Docker CLI에 직접 전달됩니다. 운영체제 휴지통을 거치지 않으며, 이미 완료된 단계는 취소해도 되돌릴 수 없습니다.")}
            </p>

            <div className="docker-cleanup-options">
              {preview.items.map((item) => (
                <label className="docker-cleanup-option" key={item.kind}>
                  <input
                    type="checkbox"
                    name="dockerCleanupKinds"
                    value={item.kind}
                    checked={selectedKinds.includes(item.kind)}
                    disabled={busy}
                    onChange={(event) => toggleKind(item.kind, event.currentTarget.checked)}
                  />
                  <span>
                    <strong>{t(dockerCleanupLabel(item.kind))}</strong>
                    <small>{t(dockerCleanupDescription(item.kind))}</small>
                    <code translate="no">{item.commandDisplay}</code>
                  </span>
                  <span>
                    <small>{t("정리 가능 최대")}</small>
                    <strong>{formatDockerBytes(item.estimatedReclaimableBytes)}</strong>
                  </span>
                </label>
              ))}
              <div className="docker-cleanup-option is-excluded">
                <span aria-hidden="true"><ShieldAlert size={18} /></span>
                <span>
                  <strong>{t("Docker 볼륨은 정리하지 않습니다")}</strong>
                  <small>{t("데이터베이스와 사용자 파일이 들어 있을 수 있어 사용량만 표시합니다.")}</small>
                </span>
              </div>
            </div>

            <div className="docker-cleanup-summary">
              <span>{t("선택한 항목의 정리 가능 최대")}</span>
              <strong>{formatDockerBytes(estimatedBytes)}</strong>
              <small>{t("실제 확보량은 Docker의 공유 계층과 실행 시점 상태에 따라 더 작을 수 있습니다.")}</small>
            </div>

            <label className="review-acknowledgement docker-cleanup-acknowledgement">
              <input
                type="checkbox"
                name="dockerIrreversibleAcknowledgement"
                checked={acknowledged}
                disabled={busy}
                onChange={(event) => setAcknowledged(event.currentTarget.checked)}
              />
              <span>
                {t("선택한 Docker 데이터는 휴지통으로 가지 않으며 복원할 수 없음을 확인했습니다.")}
              </span>
            </label>

            {busy ? (
              <div className="safety-dialog__progress docker-cleanup-progress">
                <div>
                  <span>
                    {cancelRequested
                      ? t("현재 Docker 단계를 중단하고 있습니다")
                      : dockerProgressMessage(progress, selectedKinds, t)}
                  </span>
                  <strong>{Math.round(progressFraction * 100)}%</strong>
                </div>
                <progress
                  max={1}
                  value={progressFraction}
                  aria-label={t("Docker 정리 진행률")}
                />
              </div>
            ) : null}
          </>
        )}

        {error ? <p className="safety-dialog__error" role="alert">{error}</p> : null}

        <footer>
          {result ? (
            <button className="primary-button" ref={cancelButtonRef} type="button" onClick={onClose}>
              {t("닫기")}
            </button>
          ) : (
            <>
              <button
                className="secondary-button"
                ref={cancelButtonRef}
                type="button"
                disabled={cancelRequested}
                onClick={busy ? () => void requestCancellation() : onClose}
              >
                {busy ? <Square size={15} aria-hidden="true" /> : null}
                {busy ? t("중단 요청") : t("취소")}
              </button>
              <button
                className="trash-confirm-button"
                type="button"
                disabled={busy || selectedKinds.length === 0 || !acknowledged}
                onClick={() => void executeCleanup()}
              >
                <Trash2 size={16} aria-hidden="true" />
                {t("선택 항목 정리")}
              </button>
            </>
          )}
        </footer>
      </div>
    </div>
  );
}

function DockerCleanupResultView({ result }: { result: DockerCleanupResult }) {
  const { t } = useLanguage();
  const completed = result.outcome === "completed";
  return (
    <div className={`docker-cleanup-result is-${result.outcome}`} role="status">
      <div className="docker-cleanup-result__heading">
        {completed
          ? <CheckCircle2 size={20} aria-hidden="true" />
          : <ShieldAlert size={20} aria-hidden="true" />}
        <span>
          <strong>{t(dockerOutcomeMessage(result.outcome))}</strong>
          <small>{formatDate(result.finishedAtUnixMs)}</small>
        </span>
      </div>
      <div className="docker-cleanup-result__steps">
        {result.steps.map((step) => (
          <div key={step.kind}>
            <span>
              <strong>{t(dockerCleanupLabel(step.kind))}</strong>
              <small>{t(step.completed ? "Docker가 이 정리 단계를 완료했습니다" : "이 단계는 시작하지 않았습니다")}</small>
            </span>
            <strong>{step.completed ? formatDockerBytes(step.reportedReclaimedBytes) : t("완료 안 됨")}</strong>
          </div>
        ))}
      </div>
      <p>
        {t("Docker가 보고한 정리량 {{size}}. 볼륨은 변경하지 않았습니다.", {
          size: formatDockerBytes(result.reportedReclaimedBytes),
        })}
        {!result.historyRecorded ? ` ${t("정리 이력 저장은 완료하지 못했습니다.")}` : ""}
      </p>
    </div>
  );
}

function dockerCleanupLabel(kind: DockerCleanupKind): MessageKey {
  if (kind === "buildCache") return "7일 이상 사용하지 않은 빌드 캐시";
  if (kind === "danglingImages") return "7일 이상 된 매달린 이미지";
  return "7일 이상 된 중지 컨테이너";
}

function dockerCleanupDescription(kind: DockerCleanupKind): MessageKey {
  if (kind === "buildCache") return "다음 빌드 때 다시 만들어질 수 있습니다.";
  if (kind === "danglingImages") {
    return "태그가 없는 이미지 계층이며 필요하면 다시 내려받거나 빌드해야 합니다.";
  }
  return "중지된 컨테이너의 쓰기 계층은 복원할 수 없습니다.";
}

function dockerOutcomeMessage(outcome: DockerCleanupResult["outcome"]): MessageKey {
  if (outcome === "completed") return "Docker 정리를 완료했습니다";
  if (outcome === "partial") return "Docker 정리를 일부만 완료했습니다";
  if (outcome === "cancelled") return "Docker 정리를 취소했습니다";
  return "Docker 정리에 실패했습니다";
}

function dockerProgressMessage(
  progress: DockerCleanupProgress | null,
  selectedKinds: DockerCleanupKind[],
  t: Translate,
): string {
  if (!progress) return t("Docker 정리를 준비하고 있습니다");
  if (progress.completedSteps >= progress.totalSteps) {
    return t("Docker 사용량 다시 확인 중");
  }
  const kind = selectedKinds[progress.completedSteps];
  return kind
    ? t("{{kind}} 정리 중", { kind: t(dockerCleanupLabel(kind)) })
    : t("Docker 정리를 진행하고 있습니다");
}

function normalizeDockerError(reason: unknown, fallback: string): string {
  if (reason instanceof Error) return reason.message;
  if (typeof reason === "string") return reason;
  return fallback;
}
