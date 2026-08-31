import {
  AlertTriangle,
  CheckCircle2,
  ExternalLink,
  RefreshCw,
  ShieldCheck,
  X,
} from "lucide-react";
import { formatBytes, formatCount, formatDate } from "../lib/format";
import type {
  ActionRecoveryReport,
  RecoveryItemStatus,
} from "../types";

interface RecoveryNoticeProps {
  report: ActionRecoveryReport;
  openingTrash: boolean;
  actionError: string | null;
  onOpenTrash: () => void;
  onDismiss: () => void;
}

const statusLabels: Record<RecoveryItemStatus, string> = {
  notStarted: "시작 전",
  originalPresent: "원본 유지",
  recordedMoved: "이동 기록 확인",
  foundInTrash: "휴지통 확인",
  recordedFailed: "실패·원본 확인",
  originalAndTrash: "양쪽에 존재",
  missing: "위치 불명",
  trashLookupUnavailable: "휴지통 확인 불가",
  accessUnknown: "경로 확인 불가",
};

export function RecoveryNotice({
  report,
  openingTrash,
  actionError,
  onOpenTrash,
  onDismiss,
}: RecoveryNoticeProps) {
  const operations = report.incompleteOperations;
  const totalItems = operations.reduce(
    (total, operation) => total + operation.items.length,
    0,
  );
  const attentionCount = operations.reduce(
    (total, operation) => total + operation.attentionCount,
    0,
  );
  const hasWarning = attentionCount > 0 || report.issues.length > 0;
  const visibleOperations = operations.slice(0, 8);

  return (
    <section
      className={`recovery-notice ${hasWarning ? "has-warning" : "is-resolved"}`}
      aria-live="polite"
      aria-label="이전 휴지통 작업 확인 결과"
    >
      <header className="recovery-notice__header">
        <span className="recovery-notice__icon" aria-hidden="true">
          {hasWarning ? <AlertTriangle size={22} /> : <ShieldCheck size={22} />}
        </span>
        <div className="recovery-notice__copy">
          <p className="eyebrow">작업 복구</p>
          <h2>
            {attentionCount > 0
              ? "확인이 필요한 이전 휴지통 작업이 있습니다"
              : report.issues.length > 0
                ? "이전 작업 기록을 일부만 확인했습니다"
                : "중단된 휴지통 작업을 자동으로 대조했습니다"}
          </h2>
          <p>
            {attentionCount > 0
              ? "자동 복원이나 영구 삭제는 하지 않았습니다. 원본과 운영체제 휴지통을 직접 확인하세요."
              : "원본 경로, 완료 기록, 운영체제 휴지통을 비교했으며 파일을 추가로 변경하지 않았습니다."}
          </p>
        </div>
        <button
          className="recovery-notice__dismiss icon-button"
          type="button"
          aria-label="이전 작업 알림 닫기"
          onClick={onDismiss}
        >
          <X size={17} aria-hidden="true" />
        </button>
      </header>

      <div className="recovery-notice__metrics" aria-label="복구 대조 요약">
        <span>
          <small>중단 기록</small>
          <strong>{formatCount(operations.length)}건</strong>
        </span>
        <span>
          <small>확인 항목</small>
          <strong>{formatCount(totalItems)}개</strong>
        </span>
        <span className={attentionCount > 0 ? "is-warning" : ""}>
          <small>직접 확인</small>
          <strong>{formatCount(attentionCount)}개</strong>
        </span>
        <span>
          <small>확인 시각</small>
          <strong>{formatDate(report.checkedAtUnixMs)}</strong>
        </span>
      </div>

      {visibleOperations.length > 0 ? (
        <div className="recovery-operation-list">
          {visibleOperations.map((operation, operationIndex) => {
            const visibleItems = operation.items.slice(0, 10);
            return (
              <details
                className={`recovery-operation ${operation.attentionCount > 0 ? "has-warning" : ""}`}
                key={operation.operationId}
                open={operationIndex === 0 && operation.attentionCount > 0}
              >
                <summary>
                  <span aria-hidden="true">
                    {operation.attentionCount > 0 ? (
                      <AlertTriangle size={15} />
                    ) : (
                      <CheckCircle2 size={15} />
                    )}
                  </span>
                  <span>
                    <strong>{formatDate(operation.startedAtUnixMs)} 작업</strong>
                    <small>
                      {formatCount(operation.plannedCount)}개 계획 · {operation.attentionCount > 0
                        ? `${formatCount(operation.attentionCount)}개 직접 확인`
                        : operation.auditSaved
                          ? "자동 대조 기록 저장"
                          : "자동 대조 완료"}
                    </small>
                  </span>
                </summary>
                <div className="recovery-item-list">
                  {visibleItems.map((item) => (
                    <div
                      className={`recovery-item ${item.needsAttention ? "has-warning" : ""}`}
                      key={`${operation.operationId}:${item.path}`}
                    >
                      <span>
                        <strong>{statusLabels[item.status]}</strong>
                        <small>{formatBytes(item.logicalBytes)}</small>
                      </span>
                      <span>
                        <code title={item.path}>{item.path}</code>
                        <small>{item.message}</small>
                      </span>
                    </div>
                  ))}
                  {operation.items.length > visibleItems.length ? (
                    <p className="recovery-item-list__more">
                      나머지 {formatCount(operation.items.length - visibleItems.length)}개 항목은 작업 기록에 보존되어 있습니다.
                    </p>
                  ) : null}
                </div>
              </details>
            );
          })}
          {operations.length > visibleOperations.length ? (
            <p className="recovery-operation-list__more">
              나머지 {formatCount(operations.length - visibleOperations.length)}건은 작업 기록에 보존되어 있습니다.
            </p>
          ) : null}
        </div>
      ) : null}

      {report.issues.length > 0 ? (
        <div className="recovery-notice__issues" role="alert">
          <strong>대조 중 확인하지 못한 내용</strong>
          {report.issues.slice(0, 6).map((issue) => (
            <p key={issue}>{issue}</p>
          ))}
        </div>
      ) : null}

      {actionError ? (
        <p className="recovery-notice__action-error" role="alert">
          {actionError}
        </p>
      ) : null}

      <footer className="recovery-notice__footer">
        <span>
          <small>작업 기록</small>
          <code title={report.journalPath}>{report.journalPath}</code>
        </span>
        <button
          className="secondary-button"
          type="button"
          disabled={openingTrash}
          onClick={onOpenTrash}
        >
          {openingTrash ? (
            <RefreshCw size={16} aria-hidden="true" />
          ) : (
            <ExternalLink size={16} aria-hidden="true" />
          )}
          {openingTrash ? "휴지통 여는 중" : "운영체제 휴지통 열기"}
        </button>
      </footer>
    </section>
  );
}

interface RecoveryCheckNoticeProps {
  error: string | null;
  onRetry: () => void;
  onDismiss: () => void;
}

export function RecoveryCheckNotice({
  error,
  onRetry,
  onDismiss,
}: RecoveryCheckNoticeProps) {
  return (
    <section className={`recovery-check ${error ? "has-warning" : ""}`} aria-live="polite">
      <span className="recovery-check__icon" aria-hidden="true">
        {error ? <AlertTriangle size={19} /> : <ShieldCheck size={19} />}
      </span>
      <span>
        <strong>{error ? "이전 휴지통 작업을 확인하지 못했습니다" : "이전 작업 기록을 확인하고 있습니다"}</strong>
        <small>
          {error ?? "원본 경로와 운영체제 휴지통을 대조하는 중입니다. 파일은 변경하지 않습니다."}
        </small>
      </span>
      {error ? (
        <span className="recovery-check__actions">
          <button className="secondary-button" type="button" onClick={onRetry}>
            <RefreshCw size={15} aria-hidden="true" />
            다시 확인
          </button>
          <button
            className="recovery-check__dismiss icon-button"
            type="button"
            aria-label="이전 작업 확인 오류 닫기"
            onClick={onDismiss}
          >
            <X size={16} aria-hidden="true" />
          </button>
        </span>
      ) : (
        <span className="recovery-check__track" aria-hidden="true"><i /></span>
      )}
    </section>
  );
}
