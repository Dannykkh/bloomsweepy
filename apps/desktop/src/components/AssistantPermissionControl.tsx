import { Check, ChevronDown, Shield, X } from "lucide-react";
import { useEffect, useId, useRef, useState, type KeyboardEvent } from "react";
import { useLanguage } from "../i18n";
import {
  ASSISTANT_PERMISSION_MODE_OPTIONS,
  isAssistantPermissionMode,
  normalizeAssistantPermissionMode,
} from "../lib/assistantPermissionMode";
import type { AssistantPermissionMode } from "../types";
import "./AssistantPermissionControl.css";

interface AssistantPermissionControlProps {
  mode?: AssistantPermissionMode;
  onChange?: (mode: AssistantPermissionMode) => void;
  busy?: boolean;
  error?: string;
  available?: boolean;
}

export function AssistantPermissionControl({
  mode,
  onChange,
  busy = false,
  error,
  available = true,
}: AssistantPermissionControlProps) {
  const { t } = useLanguage();
  const id = useId();
  const rootRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const popupRef = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const current = normalizeAssistantPermissionMode(mode);
  const selected = ASSISTANT_PERMISSION_MODE_OPTIONS.find(option => option.mode === current)!;
  const locked = busy || !available || !onChange || (mode !== undefined && !isAssistantPermissionMode(mode));
  const popup = open && !locked;
  const label = t(selected.label);
  const title = `${t("채팅 권한 선택")}: ${label}`;

  useEffect(() => { setOpen(false); }, [locked, current]);

  useEffect(() => {
    if (!popup) return;
    const element = popupRef.current;
    const selectedOption = element?.querySelector<HTMLButtonElement>('[role="option"][aria-selected="true"]');
    (selectedOption ?? element)?.focus({ preventScroll: true });
    if (selectedOption) revealOption(selectedOption);

    function outside(event: Event) {
      if (event.target instanceof Node && !rootRef.current?.contains(event.target)) setOpen(false);
    }
    function escape(event: globalThis.KeyboardEvent) {
      if (event.key !== "Escape") return;
      event.preventDefault();
      event.stopPropagation();
      setOpen(false);
      triggerRef.current?.focus({ preventScroll: true });
    }
    document.addEventListener("pointerdown", outside);
    document.addEventListener("focusin", outside);
    document.addEventListener("keydown", escape);
    return () => {
      document.removeEventListener("pointerdown", outside);
      document.removeEventListener("focusin", outside);
      document.removeEventListener("keydown", escape);
    };
  }, [popup]);

  function close() {
    setOpen(false);
    triggerRef.current?.focus({ preventScroll: true });
  }

  function choose(next: AssistantPermissionMode) {
    if (locked) return;
    close();
    if (next !== current) onChange?.(next);
  }

  function moveOptionFocus(event: KeyboardEvent<HTMLDivElement>) {
    if (!["ArrowDown", "ArrowUp", "ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
    const options = Array.from(event.currentTarget.querySelectorAll<HTMLButtonElement>('[role="option"]:not(:disabled)'));
    if (!options.length) return;
    event.preventDefault();
    event.stopPropagation();
    const focused = options.indexOf(document.activeElement as HTMLButtonElement);
    const next = event.key === "Home" ? 0 : event.key === "End" ? options.length - 1
      : event.key === "ArrowDown" || event.key === "ArrowRight" ? (focused + 1) % options.length
      : focused < 0 ? options.length - 1 : (focused - 1 + options.length) % options.length;
    options[next]?.focus({ preventScroll: true });
    if (options[next]) revealOption(options[next]);
  }

  function revealOption(option: HTMLButtonElement) {
    const element = popupRef.current;
    if (!element) return;
    const bounds = element.getBoundingClientRect();
    const row = option.getBoundingClientRect();
    if (row.top < bounds.top) element.scrollTop += row.top - bounds.top - 8;
    else if (row.bottom > bounds.bottom) element.scrollTop += row.bottom - bounds.bottom + 8;
  }

  return <div ref={rootRef} className="assistant-permission-control">
    <button ref={triggerRef} type="button" className="assistant-permission-control__trigger"
      aria-label={title} title={title} aria-haspopup="dialog" aria-expanded={popup}
      aria-controls={popup ? `${id}-popup` : undefined} aria-describedby={error ? `${id}-error` : undefined}
      disabled={locked} onClick={() => { if (!locked) setOpen(previous => !previous); }}>
      <Shield size={16} aria-hidden="true" /><span>{label}</span><ChevronDown size={14} aria-hidden="true" />
    </button>
    {popup ? <div ref={popupRef} id={`${id}-popup`} className="assistant-permission-control__popup"
      role="dialog" aria-modal="false" aria-labelledby={`${id}-heading`} tabIndex={-1}>
      <div className="assistant-permission-control__heading">
        <strong id={`${id}-heading`}>{t("채팅 권한 선택")}</strong>
        <button type="button" className="assistant-permission-control__close" aria-label={t("닫기")} onClick={close}><X size={18} aria-hidden="true" /></button>
      </div>
      <div role="listbox" aria-label={t("채팅 권한 선택")} aria-orientation="vertical"
        className="assistant-permission-control__options" onKeyDown={moveOptionFocus}>
        {ASSISTANT_PERMISSION_MODE_OPTIONS.map(option => <button type="button" role="option" key={option.mode}
          aria-selected={current === option.mode} aria-labelledby={`${id}-${option.mode}-label`}
          aria-describedby={`${id}-${option.mode}-description`} tabIndex={current === option.mode ? 0 : -1}
          onClick={() => choose(option.mode)}>
          <span className="assistant-permission-control__option-copy">
            <span id={`${id}-${option.mode}-label`} className="assistant-permission-control__option-label">{t(option.label)}</span>
            <span id={`${id}-${option.mode}-description`} className="assistant-permission-control__option-description">{t(option.description)}</span>
          </span>
          {current === option.mode ? <Check size={17} aria-hidden="true" /> : null}
        </button>)}
      </div>
      <p className="assistant-permission-control__scope">{t("영구 삭제·문서 내용 공개·OS 파일 접근·외부 MCP 권한은 별도입니다.")}</p>
    </div> : null}
    {error ? <span id={`${id}-error`} className="assistant-permission-control__error" role="alert">{error}</span> : null}
  </div>;
}
