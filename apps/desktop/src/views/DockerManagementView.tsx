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
import type { DockerCleanupPreview, DockerManagementStatus } from "../types";

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
      setReviewError(normalizeDockerError(reason));
    } finally {
      setPreparingReview(false);
    }
  }

  if (!status?.enabled && !loading) {
    return (
      <section className="empty-panel empty-panel--page">
        <Boxes size={28} aria-hidden="true" />
        <strong>Docker 용량 관리가 꺼져 있습니다</strong>
        <p>설정에서 기능을 켜면 이 메뉴와 Docker 사용량이 표시됩니다.</p>
      </section>
    );
  }

  return (
    <div className="docker-management-view">
      <section className="docker-overview-strip" aria-label="Docker 용량 요약">
        <div>
          <small>Docker 범주 합계</small>
          <strong>{status ? formatDockerBytes(status.totalSizeBytes) : "확인 중"}</strong>
          <span>Docker CLI가 보고한 논리 합계</span>
        </div>
        <div>
          <small>정리 가능 최대</small>
          <strong>{status ? formatDockerBytes(status.reclaimableBytes) : "확인 중"}</strong>
          <span>볼륨을 제외한 참고 상한</span>
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
              {loading ? "Docker 사용량 확인 중" : ready ? "Docker 연결됨" : "Docker를 사용할 수 없음"}
            </strong>
            <small>{status?.detail ?? "Docker 설정을 확인하고 있습니다."}</small>
            {status?.clientVersion || status?.serverVersion ? (
              <small>
                CLI {status.clientVersion ?? "확인 안 됨"} · Engine {status.serverVersion ?? "확인 안 됨"}
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
            다시 확인
          </button>
        </div>

        {ready ? (
          <div className="docker-usage-list" aria-label="Docker 사용량">
            {status.categories.map((category) => (
              <div className="docker-usage-row" key={category.kind}>
                <span>
                  <strong>{category.label}</strong>
                  <small>
                    전체 {formatCount(category.totalCount)}개 · 사용 중 {formatCount(category.activeCount)}개
                  </small>
                </span>
                <span>
                  <small>사용량</small>
                  <strong>{formatDockerBytes(category.sizeBytes)}</strong>
                </span>
                <span>
                  <small>{category.cleanupSupported ? "정리 가능 최대" : "보호"}</small>
                  <strong>
                    {category.cleanupSupported
                      ? formatDockerBytes(category.reclaimableBytes)
                      : "자동 정리 안 함"}
                  </strong>
                </span>
              </div>
            ))}
            <div className="docker-usage-footer">
              <span>
                {status.capturedAtUnixMs
                  ? `${formatDate(status.capturedAtUnixMs)}에 Docker CLI로 확인`
                  : "Docker 사용량 확인 시각 없음"}
              </span>
            </div>
          </div>
        ) : null}

        {status?.lastCleanup ? (
          <p className="docker-last-cleanup">
            최근 Docker 정리 · {formatDate(status.lastCleanup.finishedAtUnixMs)} · {status.lastCleanup.message}
          </p>
        ) : null}

        {error ? <p className="docker-tool-error" role="alert">{error}</p> : null}
        {reviewError ? <p className="docker-tool-error" role="alert">{reviewError}</p> : null}
      </section>

      <section className="docker-page-actions" aria-label="Docker 다음 작업">
        <div>
          <strong>무엇을 정리할지 먼저 판단하세요</strong>
          <p>대화는 현재 Docker 요약만 전달합니다. 실제 정리는 이 앱의 최종 확인 뒤에만 실행됩니다.</p>
        </div>
        <div>
          <button
            className="secondary-button"
            type="button"
            disabled={!status?.enabled}
            onClick={onAskInChat}
          >
            <MessageSquare size={16} aria-hidden="true" />
            Docker 대화 시작
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
            {status?.reclaimableBytes === 0 ? "정리할 항목 없음" : "Docker 정리 검토"}
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

function normalizeDockerError(reason: unknown): string {
  if (reason instanceof Error) return reason.message;
  if (typeof reason === "string") return reason;
  return "Docker 상태를 확인하지 못했습니다";
}
