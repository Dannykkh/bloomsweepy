import { ShieldAlert, Trash2, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { formatBytes, formatCount } from "../lib/format";
import type { TrashProgress } from "../types";

interface SafetyActionDialogProps {
  open: boolean;
  title: string;
  itemCount: number;
  logicalBytes: number;
  reviewCount?: number;
  busy: boolean;
  progress: TrashProgress | null;
  error: string | null;
  intro?: string;
  items?: Array<{
    path: string;
    logicalBytes: number;
    detail?: string;
  }>;
  confirmLabel?: string;
  onConfirm: (reviewAcknowledged: boolean) => void;
  onCancel: () => void;
  onClose: () => void;
}

export function SafetyActionDialog({
  open,
  title,
  itemCount,
  logicalBytes,
  reviewCount = 0,
  busy,
  progress,
  error,
  intro,
  items = [],
  confirmLabel = "휴지통으로 이동",
  onConfirm,
  onCancel,
  onClose,
}: SafetyActionDialogProps) {
  const dialogRef = useRef<HTMLDivElement>(null);
  const cancelButtonRef = useRef<HTMLButtonElement>(null);
  const [reviewAcknowledged, setReviewAcknowledged] = useState(false);

  useEffect(() => {
    if (!open) return;
    setReviewAcknowledged(false);
    cancelButtonRef.current?.focus();
  }, [open]);

  useEffect(() => {
    if (!open) return;
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        event.preventDefault();
        if (busy) onCancel();
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
  }, [busy, onCancel, onClose, open]);

  if (!open) return null;

  const fraction = progress?.totalItems
    ? Math.min(1, progress.processedItems / progress.totalItems)
    : 0;

  return (
    <div
      className="safety-dialog-backdrop"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget && !busy) onClose();
      }}
    >
      <div
        className="safety-dialog"
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="safety-dialog-title"
        aria-describedby="safety-dialog-description"
      >
        <header>
          <span aria-hidden="true"><Trash2 size={20} /></span>
          <div>
            <p className="eyebrow">되돌릴 수 있는 작업</p>
            <h2 id="safety-dialog-title">{title}</h2>
          </div>
          <button
            className="icon-button"
            type="button"
            aria-label="확인 창 닫기"
            disabled={busy}
            onClick={onClose}
          >
            <X size={17} aria-hidden="true" />
          </button>
        </header>

        <div className="safety-dialog__summary" id="safety-dialog-description">
          <div><span>대상</span><strong>{formatCount(itemCount)}개</strong></div>
          <div><span>선택한 파일 크기</span><strong>{formatBytes(logicalBytes)}</strong></div>
          <div><span>복구 위치</span><strong>운영체제 휴지통</strong></div>
        </div>

        {intro ? <p className="safety-dialog__intro">{intro}</p> : null}

        {items.length > 0 ? (
          <div className="safety-dialog__items" aria-label="휴지통으로 이동할 항목">
            {items.map((item) => (
              <div className="safety-dialog__item" key={item.path}>
                <span>
                  <strong title={item.path}>{item.path}</strong>
                  {item.detail ? <small>{item.detail}</small> : null}
                </span>
                <b>{formatBytes(item.logicalBytes)}</b>
              </div>
            ))}
          </div>
        ) : null}

        <div className="safety-dialog__warning">
          <ShieldAlert size={18} aria-hidden="true" />
          <p>
            이동 직전에 파일 신원과 변경 여부를 다시 검사합니다. 휴지통에서 복원할 수 있지만,
            실제 여유 공간은 휴지통을 비운 뒤에 늘어납니다.
          </p>
        </div>

        {reviewCount > 0 ? (
          <label className="review-acknowledgement">
            <input
              type="checkbox"
              checked={reviewAcknowledged}
              disabled={busy}
              onChange={(event) => setReviewAcknowledged(event.currentTarget.checked)}
            />
            <span>
              한 번 더 확인할 프로그램 설정 {formatCount(reviewCount)}개에는 계정이나 설정 데이터가 포함될 수 있음을 확인했습니다.
            </span>
          </label>
        ) : null}

        {busy ? (
          <div className="safety-dialog__progress" role="status" aria-live="polite">
            <div><span>{progress?.message ?? "안전 검사를 준비하고 있습니다"}</span><strong>{Math.round(fraction * 100)}%</strong></div>
            <progress max={1} value={fraction} />
          </div>
        ) : null}

        {error ? <p className="safety-dialog__error" role="alert">{error}</p> : null}

        <footer>
          <button
            className="secondary-button"
            ref={cancelButtonRef}
            type="button"
            onClick={busy ? onCancel : onClose}
          >
            {busy ? "작업 중단 요청" : "취소"}
          </button>
          <button
            className="trash-confirm-button"
            type="button"
            disabled={busy || (reviewCount > 0 && !reviewAcknowledged)}
            onClick={() => onConfirm(reviewAcknowledged)}
          >
            <Trash2 size={16} aria-hidden="true" />
            {confirmLabel}
          </button>
        </footer>
      </div>
    </div>
  );
}
