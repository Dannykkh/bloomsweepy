import { OctagonAlert, Power, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { formatDate } from "../lib/format";
import type { TerminationPreview } from "../types";
import { useLanguage } from "../i18n";

interface ProcessTerminationDialogProps {
  preview: TerminationPreview | null;
  busy: boolean;
  error: string | null;
  onConfirm: () => void;
  onClose: () => void;
}

export function ProcessTerminationDialog({
  preview,
  busy,
  error,
  onConfirm,
  onClose,
}: ProcessTerminationDialogProps) {
  const { t } = useLanguage();
  const dialogRef = useRef<HTMLDivElement>(null);
  const cancelButtonRef = useRef<HTMLButtonElement>(null);
  const [acknowledged, setAcknowledged] = useState(false);

  useEffect(() => {
    if (!preview) return;
    setAcknowledged(false);
    requestAnimationFrame(() => cancelButtonRef.current?.focus());
  }, [preview]);

  useEffect(() => {
    if (!preview) return;
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        event.preventDefault();
        if (!busy) onClose();
        return;
      }
      if (event.key !== "Tab") return;
      const focusable = dialogRef.current?.querySelectorAll<HTMLElement>(
        'button:not([disabled]), input:not([disabled]), [tabindex]:not([tabindex="-1"])',
      );
      if (!focusable?.length) return;
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

  if (!preview) return null;

  return (
    <div
      className="safety-dialog-backdrop"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget && !busy) onClose();
      }}
    >
      <div
        className="safety-dialog process-termination-dialog"
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="process-termination-title"
        aria-describedby="process-termination-description"
      >
        <header>
          <span aria-hidden="true"><Power size={20} /></span>
          <div>
            <p className="eyebrow">{t("정상 종료 요청")}</p>
            <h2 id="process-termination-title">
              {t("{{name}} 종료 전 확인", { name: preview.displayName })}
            </h2>
          </div>
          <button
            className="icon-button"
            type="button"
            aria-label={t("종료 확인 창 닫기")}
            disabled={busy}
            onClick={onClose}
          >
            <X size={17} aria-hidden="true" />
          </button>
        </header>

        <div className="safety-dialog__summary" id="process-termination-description">
          <div><span>{t("대상 앱")}</span><strong>{preview.displayName}</strong></div>
          <div><span>{t("프로세스 ID")}</span><strong>{preview.pid}</strong></div>
          <div><span>{t("마지막 확인")}</span><strong>{formatDate(preview.capturedAtUnixMs)}</strong></div>
        </div>

        <div className="safety-dialog__warning">
          <OctagonAlert size={18} aria-hidden="true" />
          <p>{t("저장하지 않은 작업이 있으면 사라질 수 있습니다. macOS가 앱에 정상 종료를 요청하며 강제 종료는 하지 않습니다.")}</p>
        </div>

        <label className="review-acknowledgement">
          <input
            type="checkbox"
            checked={acknowledged}
            disabled={busy}
            onChange={(event) => setAcknowledged(event.currentTarget.checked)}
          />
          <span>{t("대상 앱과 저장하지 않은 작업 위험을 확인했습니다.")}</span>
        </label>

        {busy ? (
          <div className="process-termination-dialog__busy" role="status">
            <span className="process-termination-dialog__spinner" aria-hidden="true" />
            {t("종료 요청 중…")}
          </div>
        ) : null}
        {error ? <p className="safety-dialog__error" role="alert">{error}</p> : null}

        <footer>
          <button
            className="secondary-button"
            ref={cancelButtonRef}
            type="button"
            disabled={busy}
            onClick={onClose}
          >
            {t("취소")}
          </button>
          <button
            className="process-termination-dialog__confirm"
            type="button"
            disabled={busy || !acknowledged}
            onClick={onConfirm}
          >
            <Power size={16} aria-hidden="true" />
            {t("{{name}}에 종료 요청 보내기", { name: preview.displayName })}
          </button>
        </footer>
      </div>
    </div>
  );
}
