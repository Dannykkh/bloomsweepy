import { AlertTriangle, ExternalLink, LoaderCircle, Trash2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useLanguage, type MessageKey } from "../i18n";
import { confirmEmptySystemTrash, dismissEmptySystemTrash, openSystemTrash, prepareEmptySystemTrash } from "../lib/bridge";
import { canConfirmEmptyTrash, emptyTrashErrorMessage, emptyTrashOutcomeMessages } from "../lib/emptyTrashPolicy";
import type { EmptyTrashOutcome, EmptyTrashPlan } from "../types";

interface Props {
  visible: boolean;
  platform: string | null;
  blocked: boolean;
  onBusyChange: (busy: boolean) => void;
  onSettled: () => void;
}

export function EmptyTrashControl({ visible, platform, blocked, onBusyChange, onSettled }: Props) {
  const { t } = useLanguage();
  const dialogRef = useRef<HTMLDialogElement>(null);
  const cancelRef = useRef<HTMLButtonElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const openRef = useRef<HTMLButtonElement>(null);
  const submitting = useRef(false);
  const [reviewing, setReviewing] = useState(false);
  const [plan, setPlan] = useState<EmptyTrashPlan | null>(null);
  const [acknowledged, setAcknowledged] = useState(false);
  const [busy, setBusy] = useState(false);
  const [consumed, setConsumed] = useState(false);
  const [now, setNow] = useState(Date.now());
  const [result, setResult] = useState<EmptyTrashOutcome | null>(null);
  const [error, setError] = useState<MessageKey | null>(null);
  const [needsInspection, setNeedsInspection] = useState(false);
  const [opening, setOpening] = useState(false);
  const supported = platform === "macos" || platform === "windows";

  useEffect(() => {
    // Wait for React to enable Close before focusing it after native completion.
    if (reviewing && consumed && !busy) cancelRef.current?.focus();
  }, [reviewing, consumed, busy]);

  useEffect(() => {
    if (!reviewing) return;
    let disposed = false;
    let prepared: EmptyTrashPlan | null = null;
    setPlan(null);
    setAcknowledged(false);
    setConsumed(false);
    setResult(null);
    setError(null);
    setNow(Date.now());
    // Native modal makes the background inert and traps keyboard focus.
    dialogRef.current?.showModal();
    cancelRef.current?.focus();
    void prepareEmptySystemTrash().then((next) => {
      prepared = next;
      if (disposed) void dismissEmptySystemTrash(next.id).catch(() => {});
      else {
        setPlan(next);
        setNow(Date.now());
      }
    }).catch((reason: unknown) => {
      if (!disposed) {
        setError(emptyTrashErrorMessage(reason));
        if (reason === "needsInspection") setNeedsInspection(true);
      }
    });
    const timer = window.setInterval(() => setNow(Date.now()), 500);
    return () => {
      disposed = true;
      window.clearInterval(timer);
      if (prepared) void dismissEmptySystemTrash(prepared.id).catch(() => {});
      dialogRef.current?.close();
      if (triggerRef.current?.disabled) openRef.current?.focus();
      else triggerRef.current?.focus();
    };
  }, [reviewing]);

  async function showTrash() {
    if (opening) return;
    setOpening(true);
    try {
      await openSystemTrash();
    } catch {
      setError("휴지통을 열지 못했습니다. 운영체제에서 직접 열어 확인하세요.");
    } finally {
      setOpening(false);
    }
  }

  async function confirm() {
    if (blocked || submitting.current || !canConfirmEmptyTrash(plan, acknowledged, consumed, Date.now())) return;
    submitting.current = true;
    setConsumed(true);
    setBusy(true);
    setError(null);
    onBusyChange(true);
    try {
      const outcome = await confirmEmptySystemTrash(plan!.id, acknowledged);
      setResult(outcome);
      if (outcome === "unconfirmed") setNeedsInspection(true);
    } catch (reason) {
      // Unknown transport failures may occur after dispatch. Do not invite a retry.
      const known = typeof reason === "string" && ["expired", "invalidPlan", "needsInspection", "unsupported", "unavailable", "acknowledgmentRequired"].includes(reason);
      setError(known ? emptyTrashErrorMessage(reason) : emptyTrashOutcomeMessages.unconfirmed);
      if (!known || reason === "needsInspection") setNeedsInspection(true);
    } finally {
      setBusy(false);
      submitting.current = false;
      onBusyChange(false);
      onSettled();
    }
  }

  const expired = !!plan && plan.expiresAtUnixMs <= now;
  return <>
    <div className="empty-trash-tools" hidden={!visible}>
      <span>{t("휴지통 이동만으로는 디스크 공간이 확보되지 않을 수 있습니다.")}</span>
      <button ref={openRef} type="button" className="secondary-button" disabled={opening || busy} onClick={() => void showTrash()}>
        <ExternalLink size={15} aria-hidden="true" />{t("휴지통 열기")}
      </button>
      <button ref={triggerRef} type="button" className="secondary-button empty-trash-trigger" disabled={blocked || !supported || needsInspection}
        title={needsInspection ? t(emptyTrashOutcomeMessages.unconfirmed) : blocked ? t(emptyTrashErrorMessage("unavailable")) : supported ? t("별도 확인 후 영구 삭제합니다.") : t("휴지통 비우기는 macOS와 Windows에서 지원합니다.")}
        onClick={() => setReviewing(true)}>
        <Trash2 size={16} aria-hidden="true" />{t("운영체제 휴지통 비우기")}
      </button>
      {!reviewing && error ? <p role="alert">{t(error)}</p> : null}
      {!reviewing && result && !error ? <p role="status">{t(emptyTrashOutcomeMessages[result])}</p> : null}
    </div>
    <dialog ref={dialogRef} className="safety-dialog empty-trash-dialog" aria-labelledby="empty-trash-title" aria-describedby="empty-trash-scope"
      onCancel={(event) => { event.preventDefault(); if (!submitting.current) setReviewing(false); }}>
      <header><span><AlertTriangle size={21} aria-hidden="true" /></span><div>
        <small>{t("복원할 수 없는 작업")}</small>
        <h2 id="empty-trash-title">{t("운영체제 휴지통 비우기")}</h2>
      </div></header>
      <div className="safety-dialog__warning" id="empty-trash-scope"><AlertTriangle size={18} aria-hidden="true" /><div>
        <p>{t("현재 사용자의 연결된 드라이브 휴지통 전체가 대상입니다. 다른 앱에서 버린 항목도 포함되며, 선택한 폴더나 BroomSweepy 작업 기록으로 범위를 제한하지 않습니다.")}</p>
        <p>{t("실행 시점에 휴지통에 있는 항목을 영구 삭제합니다. 이 앱에서 되돌릴 수 없으며, 작업 기록으로도 복원할 수 없습니다.")}</p>
      </div></div>
      <p className="safety-dialog__intro">{t("먼저 휴지통을 열어 보관할 항목을 복원하세요. 내용과 용량을 미리 전부 스캔하지 않습니다. 확인은 2분 동안 한 번만 사용할 수 있습니다.")}</p>
      <label className="review-acknowledgement">
        <input type="checkbox" checked={acknowledged} disabled={busy || consumed || !plan || expired}
          onChange={(event) => setAcknowledged(event.target.checked)} />
        <span>{t("다른 앱의 항목까지 영구 삭제되며 되돌릴 수 없음을 확인했습니다.")}</span>
      </label>
      {busy ? <p className="safety-dialog__intro" role="status"><LoaderCircle className="spin" size={16} aria-hidden="true" /> {t("운영체제 응답을 기다리는 중입니다. OS 확인창을 확인하세요. 시작된 삭제는 이 앱에서 취소할 수 없습니다.")}</p> : null}
      {expired && !consumed ? <p className="safety-dialog__error" role="alert">{t("확인 시간이 만료됐습니다. 창을 닫고 다시 검토하세요.")}</p> : null}
      {result ? <p className="safety-dialog__intro" role="status">{t(emptyTrashOutcomeMessages[result])}</p> : null}
      {error ? <p className="safety-dialog__error" role="alert">{t(error)}</p> : null}
      <footer>
        <button type="button" className="secondary-button" disabled={busy || opening} onClick={() => void showTrash()}>{t("휴지통 열기")}</button>
        <button ref={cancelRef} type="button" className="secondary-button" disabled={busy} onClick={() => setReviewing(false)}>{t(consumed ? "닫기" : "취소")}</button>
        {!consumed ? <button type="button" className="trash-confirm-button" disabled={blocked || !canConfirmEmptyTrash(plan, acknowledged, consumed, now)} onClick={() => void confirm()}>
          <Trash2 size={16} aria-hidden="true" />{t("휴지통 전체 영구 삭제")}
        </button> : null}
      </footer>
    </dialog>
  </>;
}
