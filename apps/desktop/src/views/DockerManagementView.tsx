import {
  Boxes,
  LoaderCircle,
  MessageSquare,
  RefreshCw,
  ShieldCheck,
} from "lucide-react";
import { useState } from "react";
import { DockerCleanupDialog } from "../components/DockerCleanupDialog";
import { createDockerCleanupPreview } from "../lib/bridge";
import { formatCount, formatDate, formatDockerBytes } from "../lib/format";
import type { DockerCleanupPreview, DockerManagementStatus, DockerUsageKind } from "../types";
import { useLanguage, type MessageKey } from "../i18n";

interface DockerManagementViewProps {
  status: DockerManagementStatus | null;
  loading: boolean;
  error: string | null;
  onRefresh: () => Promise<void>;
  onStatusChange: (status: DockerManagementStatus) => void;
  onAskInChat: () => void;
}

export function DockerManagementView({
  status,
  loading,
  error,
  onRefresh,
  onStatusChange,
  onAskInChat,
}: DockerManagementViewProps) {
  const { t } = useLanguage();
  const [preparingReview, setPreparingReview] = useState(false);
  const [preview, setPreview] = useState<DockerCleanupPreview | null>(null);
  const [reviewError, setReviewError] = useState<string | null>(null);
  const ready = status?.enabled
    && status.cliInstalled === true
    && status.daemonRunning === true;

  async function prepareCleanupReview() {
    if (preparingReview || !ready || status?.busy) return;
    setPreparingReview(true);
    setReviewError(null);
    try {
      setPreview(await createDockerCleanupPreview());
    } catch (reason) {
      setReviewError(normalizeDockerError(reason, t("Docker 상태를 확인하지 못했습니다")));
    } finally {
      setPreparingReview(false);
    }
  }

  if (!status?.enabled && !loading) {
    return (
      <section className="empty-panel empty-panel--page">
        <Boxes size={28} aria-hidden="true" />
        <strong>{t("Docker 용량 관리가 꺼져 있습니다")}</strong>
        <p>{t("설정에서 기능을 켜면 이 메뉴와 Docker 사용량이 표시됩니다.")}</p>
      </section>
    );
  }

  return (
    <div className="docker-management-view">
      <section className="docker-overview-strip" aria-label={t("Docker 용량 요약")}>
        <div>
          <small>{t("Docker 범주 합계")}</small>
          <strong>{status ? formatDockerBytes(status.totalSizeBytes) : t("확인 중")}</strong>
          <span>{t("Docker CLI가 보고한 논리 합계")}</span>
        </div>
        <div>
          <small>{t("정리 가능 최대")}</small>
          <strong>{status ? formatDockerBytes(status.reclaimableBytes) : t("확인 중")}</strong>
          <span>{t("볼륨을 제외한 참고 상한")}</span>
        </div>
      </section>

      <section className="docker-instrument" aria-labelledby="docker-status-title">
        <div className={`docker-tool-state ${ready ? "is-ready" : "is-unavailable"}`}>
          <span aria-hidden="true">
            {loading
              ? <LoaderCircle className="is-spinning" size={18} />
              : ready
                ? <ShieldCheck size={18} />
                : <Boxes size={18} />}
          </span>
          <span role="status">
            <strong id="docker-status-title">
              {loading ? t("Docker 사용량 확인 중") : ready ? t("Docker 연결됨") : t("Docker를 사용할 수 없음")}
            </strong>
            <small>{status?.detail ?? t("Docker 설정을 확인하고 있습니다.")}</small>
            {status?.clientVersion || status?.serverVersion ? (
              <small>
                CLI {status.clientVersion ?? t("확인 안 됨")} · Engine {status.serverVersion ?? t("확인 안 됨")}
              </small>
            ) : null}
          </span>
          <button
            className="secondary-button"
            type="button"
            disabled={loading || status?.busy}
            onClick={() => void onRefresh()}
          >
            <RefreshCw className={loading ? "is-spinning" : ""} size={15} aria-hidden="true" />
            {t("다시 확인")}
          </button>
        </div>

        {ready ? (
          <div className="docker-usage-list" aria-label={t("Docker 사용량")}>
            {status.categories.map((category) => (
              <div className="docker-usage-row" key={category.kind}>
                <span>
                  <strong>{t(dockerUsageLabel(category.kind))}</strong>
                  <small>
                    {t("전체 {{total}}개 · 사용 중 {{active}}개", {
                      total: formatCount(category.totalCount),
                      active: formatCount(category.activeCount),
                    })}
                  </small>
                </span>
                <span>
                  <small>{t("사용량")}</small>
                  <strong>{formatDockerBytes(category.sizeBytes)}</strong>
                </span>
                <span>
                  <small>{category.cleanupSupported ? t("정리 가능 최대") : t("보호")}</small>
                  <strong>
                    {category.cleanupSupported
                      ? formatDockerBytes(category.reclaimableBytes)
                      : t("자동 정리 안 함")}
                  </strong>
                </span>
              </div>
            ))}
            <div className="docker-usage-footer">
              <span>
                {status.capturedAtUnixMs
                  ? t("{{date}}에 Docker CLI로 확인", { date: formatDate(status.capturedAtUnixMs) })
                  : t("Docker 사용량 확인 시각 없음")}
              </span>
            </div>
          </div>
        ) : null}

        {status?.lastCleanup ? (
          <p className="docker-last-cleanup">
            {t("최근 Docker 정리 · {{date}} · {{message}}", {
              date: formatDate(status.lastCleanup.finishedAtUnixMs),
              message: status.lastCleanup.message,
            })}
          </p>
        ) : null}

        {error ? <p className="docker-tool-error" role="alert">{error}</p> : null}
        {reviewError ? <p className="docker-tool-error" role="alert">{reviewError}</p> : null}
      </section>

      <section className="docker-page-actions" aria-label={t("Docker 다음 작업")}>
        <div>
          <strong>{t("무엇을 정리할지 먼저 판단하세요")}</strong>
          <p>{t("대화는 현재 Docker 요약만 전달합니다. 실제 정리는 이 앱의 최종 확인 뒤에만 실행됩니다.")}</p>
        </div>
        <div>
          <button
            className="secondary-button"
            type="button"
            disabled={!status?.enabled}
            onClick={onAskInChat}
          >
            <MessageSquare size={16} aria-hidden="true" />
            {t("Docker 대화 시작")}
          </button>
          <button
            className="primary-button"
            type="button"
            disabled={preparingReview || !ready || status.busy || status.reclaimableBytes === 0}
            onClick={() => void prepareCleanupReview()}
          >
            {preparingReview
              ? <LoaderCircle className="is-spinning" size={16} aria-hidden="true" />
              : <Boxes size={16} aria-hidden="true" />}
            {status?.reclaimableBytes === 0 ? t("정리할 항목 없음") : t("Docker 정리 검토")}
          </button>
        </div>
      </section>

      <DockerCleanupDialog
        preview={preview}
        onClose={() => setPreview(null)}
        onCompleted={onStatusChange}
      />
    </div>
  );
}

function dockerUsageLabel(kind: DockerUsageKind): MessageKey {
  if (kind === "images") return "이미지";
  if (kind === "containers") return "컨테이너";
  if (kind === "volumes") return "볼륨";
  return "빌드 캐시";
}

function normalizeDockerError(reason: unknown, fallback: string): string {
  if (reason instanceof Error) return reason.message;
  if (typeof reason === "string") return reason;
  return fallback;
}
