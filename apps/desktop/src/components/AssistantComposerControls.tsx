import { Check, ChevronDown, ChevronRight, RotateCcw, X, Zap } from "lucide-react";
import { isTauri } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useEffect, useId, useRef, useState, type KeyboardEvent, type MouseEvent } from "react";
import { useLanguage } from "../i18n";
import { assistantModelCatalogPresentation, reasoningEffortAtIndex, reasoningSliderIndex } from "../lib/assistantComposerOptions";
import { assistantModelSelection, assistantReasoningStatus, type AssistantReasoningEffort } from "../lib/assistantModelPreference";
import type { AssistantProviderStatus } from "../types";
import "./AssistantComposerControls.css";

interface AssistantComposerControlsProps {
  provider: AssistantProviderStatus | null;
  value: string;
  onChange: (value: string) => void;
  reasoningEffort?: string;
  onReasoningEffortChange?: (value: string) => void;
  busy?: boolean;
}

type PopupKind = "models" | "reasoning";
interface OpenPopup { kind: PopupKind; provider: string | undefined; model: string }

export function AssistantComposerControls({ provider, value, onChange, reasoningEffort = "", onReasoningEffortChange, busy = false }: AssistantComposerControlsProps) {
  const { t } = useLanguage();
  const id = useId();
  const rootRef = useRef<HTMLDivElement>(null);
  const popupRef = useRef<HTMLDivElement>(null);
  const modelTriggerRef = useRef<HTMLButtonElement>(null);
  const reasoningTriggerRef = useRef<HTMLButtonElement>(null);
  const originRef = useRef<HTMLButtonElement | null>(null);
  const [open, setOpen] = useState<OpenPopup | null>(null);
  const [documentationError, setDocumentationError] = useState<string | null>(null);
  const locked = busy || Boolean(provider?.busy);
  const selection = assistantModelSelection(provider);
  const models = selection === "unsupported" ? [] : provider?.models ?? [];
  const selected = value;
  const model = models.find(item => item.id === selected);
  const missing = Boolean(selected) && !model;
  const catalog = assistantModelCatalogPresentation(provider);
  const catalogChoiceLocked = models.length === 0 && catalog.unavailable && !provider?.available;
  const reasoning = assistantReasoningStatus(provider, value, reasoningEffort);
  const popup = !locked && open?.provider === provider?.provider && open?.model === value ? open?.kind : null;
  const reasoningDetail = catalog.unavailable || reasoning.mode === "supported" ? null
    : reasoning.mode === "modelDefault" ? "추론 강도를 지정하려면 모델을 직접 선택하세요. CLI 기본 모델에서는 강도를 지정하지 않습니다."
    : reasoning.mode === "unavailable" ? "추론 지원 목록을 확인하지 못했습니다. 저장한 선택은 유지하며, 기본값으로 바꾸거나 목록을 다시 확인하세요."
    : "이 CLI 또는 모델에서는 앱의 추론 강도 선택을 지원하지 않습니다. 기본값을 사용합니다.";
  const effortLabels: Record<AssistantReasoningEffort, Parameters<typeof t>[0]> = {
    none: "추론 없음", minimal: "최소", low: "낮음", medium: "중간", high: "높음", xhigh: "매우 높음", max: "최대", ultra: "울트라",
  };
  const effortLabel = (effort: string) => Object.prototype.hasOwnProperty.call(effortLabels, effort)
    ? t(effortLabels[effort as AssistantReasoningEffort]) : effort;
  const modelLabel = model?.label || selected || t(catalog.placeholder ?? (selection === "required" ? models.length ? "모델 선택" : "설치된 모델 없음" : "CLI 기본값"));
  const modelTitle = model && model.label !== model.id ? `${model.label} · ${model.id}` : modelLabel;
  const effortTitle = catalog.placeholder ? t(catalog.placeholder) : reasoningEffort ? effortLabel(reasoningEffort)
    : reasoning.defaultEffort ? t("CLI 기본값 · {{effort}}", { effort: effortLabel(reasoning.defaultEffort) }) : t("CLI 기본값");
  const sliderIndex = reasoning.mode === "supported" && !reasoning.stale
    ? reasoningSliderIndex(reasoning.supportedEfforts, reasoningEffort, reasoning.defaultEffort) : null;
  const dotCount = Math.min(7, reasoning.supportedEfforts.length);

  useEffect(() => { setOpen(null); }, [locked, provider?.provider, value]);
  useEffect(() => { setDocumentationError(null); }, [catalog.documentationUrl, catalog.referenceUrl, provider?.provider]);

  useEffect(() => {
    if (!popup) return;
    const element = popupRef.current;
    const first = popup === "reasoning"
      ? element?.querySelector<HTMLInputElement>("input[type=range]:not(:disabled)")
      : element?.querySelector<HTMLButtonElement>('[role="option"][aria-selected="true"]');
    (first ?? element?.querySelector<HTMLButtonElement>("button:not(:disabled)") ?? element)?.focus({ preventScroll: true });
    const list = element?.querySelector<HTMLDivElement>('[role="listbox"]');
    if (popup === "models" && first && list) {
      // Reveal the selected row inside its bounded list, without moving the chat.
      list.scrollTop += first.getBoundingClientRect().top - list.getBoundingClientRect().top - (list.clientHeight - first.clientHeight) / 2;
    }
    function outside(event: Event) {
      if (event.target instanceof Node && !rootRef.current?.contains(event.target)) setOpen(null);
    }
    function escape(event: globalThis.KeyboardEvent) {
      if (event.key !== "Escape") return;
      event.preventDefault();
      event.stopPropagation();
      setOpen(null);
      originRef.current?.focus({ preventScroll: true });
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

  function toggle(kind: PopupKind, trigger: HTMLButtonElement) {
    if (locked || !provider) return;
    originRef.current = trigger;
    setOpen(popup === kind ? null : { kind, provider: provider.provider, model: value });
  }

  function close() {
    setOpen(null);
    originRef.current?.focus({ preventScroll: true });
  }

  function openDocumentation(event: MouseEvent<HTMLAnchorElement>, url: string) {
    if (!isTauri()) return;
    event.preventDefault();
    setDocumentationError(null);
    void openUrl(url).catch(() => setDocumentationError(url));
  }

  function chooseModel(next: string) {
    if (locked || !provider || (!next && selection === "required")
      || (next && (catalogChoiceLocked || selection === "unsupported" || !models.some(item => item.id === next)))) return;
    onChange(next);
    close();
  }

  function chooseEffort(next: string) {
    if (locked || !onReasoningEffortChange || (next && (reasoning.mode !== "supported" || !reasoning.supportedEfforts.some(effort => effort === next)))) return;
    onReasoningEffortChange(next);
  }

  function moveModelFocus(event: KeyboardEvent<HTMLDivElement>) {
    const keys = ["ArrowDown", "ArrowUp", "Home", "End"];
    if (!keys.includes(event.key)) return;
    const options = Array.from(event.currentTarget.querySelectorAll<HTMLButtonElement>('[role="option"]:not(:disabled)'));
    if (!options.length) return;
    event.preventDefault();
    const current = options.indexOf(document.activeElement as HTMLButtonElement);
    const index = event.key === "Home" ? 0 : event.key === "End" ? options.length - 1
      : event.key === "ArrowDown" ? (current + 1) % options.length : (current - 1 + options.length) % options.length;
    options[index]?.focus();
  }

  return <div ref={rootRef} className="assistant-composer-controls" onKeyDown={event => {
    // A range inside a form must never implicitly submit the user's draft.
    if (event.key === "Enter" && event.target instanceof HTMLInputElement && event.target.type === "range") event.preventDefault();
  }}>
    <div className="assistant-composer-controls__triggers">
      <button ref={modelTriggerRef} type="button" className="assistant-composer-controls__trigger assistant-composer-controls__model-trigger"
        aria-label={`${t("모델 선택")}: ${modelTitle}`} title={modelTitle} aria-haspopup="dialog" aria-expanded={popup === "models"}
        aria-controls={popup === "models" ? `${id}-popup` : undefined} disabled={locked || !provider}
        onClick={event => toggle("models", event.currentTarget)}>
        <span>{modelLabel}</span><ChevronDown size={14} aria-hidden="true" />
      </button>
      {onReasoningEffortChange ? <button ref={reasoningTriggerRef} type="button" className="assistant-composer-controls__trigger assistant-composer-controls__effort-trigger"
        aria-label={`${t("추론 강도 선택")}: ${effortTitle}`} title={effortTitle} aria-haspopup="dialog" aria-expanded={popup === "reasoning"}
        aria-controls={popup === "reasoning" ? `${id}-popup` : undefined} disabled={locked || !provider}
        onClick={event => toggle("reasoning", event.currentTarget)}>
        <Zap size={15} aria-hidden="true" /><span>{catalog.placeholder ? t(catalog.placeholder) : reasoningEffort ? effortLabel(reasoningEffort) : t("CLI 기본값")}</span><ChevronDown size={14} aria-hidden="true" />
      </button> : null}
    </div>
    {missing ? <div className="assistant-composer-controls__warning" role="alert">
      <span>{t("{{model}} · 목록에서 확인되지 않음", { model: selected })}</span>
      {selection !== "required" ? <button type="button" disabled={locked || !provider} onClick={() => chooseModel("")}>{t("모델 기본값으로 변경")}</button> : null}
    </div> : null}
    {onReasoningEffortChange && reasoning.stale ? <div className="assistant-composer-controls__warning" role="alert">
      <span>{t("저장한 추론 강도를 현재 모델에서 확인하지 못했습니다. 기본값 또는 지원되는 강도를 선택해야 질문을 보낼 수 있습니다.")}</span>
      <button type="button" disabled={locked} onClick={() => chooseEffort("")}>{t("추론 강도 기본값으로 변경")}</button>
    </div> : null}
    {popup ? <div ref={popupRef} id={`${id}-popup`} role="dialog" aria-modal="false" tabIndex={-1}
      aria-label={t(popup === "reasoning" ? "추론 강도 선택" : "모델 선택")} className="assistant-composer-controls__popup">
      {popup === "reasoning" ? <>
        <div className="assistant-composer-controls__heading assistant-composer-controls__heading--reasoning"><Zap size={21} aria-hidden="true" />
          <strong className="assistant-composer-controls__current" title={effortTitle}>{effortTitle}</strong>
          <button type="button" className="assistant-composer-controls__icon-button" title={t("추론 강도 기본값으로 변경")}
            aria-label={t("추론 강도 기본값으로 변경")} disabled={locked || !reasoningEffort} onClick={() => chooseEffort("")}><RotateCcw size={17} aria-hidden="true" /></button>
        </div>
        <button type="button" className="assistant-composer-controls__model-subtitle" title={modelTitle} aria-label={`${t("모델 선택")}: ${modelTitle}`}
          onClick={() => { if (!locked) setOpen({ kind: "models", provider: provider?.provider, model: value }); }}>
          <span>{modelLabel}</span><ChevronRight size={15} aria-hidden="true" />
        </button>
        {sliderIndex !== null ? <div className="assistant-composer-controls__slider">
          <div className="assistant-composer-controls__range-wrap">
            <span className="assistant-composer-controls__range-track" aria-hidden="true">
              <span>{Array.from({ length: dotCount }, (_, index) => <i key={index} style={{ left: `${dotCount === 1 ? 50 : index * 100 / (dotCount - 1)}%` }} />)}</span>
            </span>
            <input type="range" min={0} max={Math.max(0, reasoning.supportedEfforts.length - 1)} step={1} value={sliderIndex}
              aria-label={t("추론 강도 선택")} aria-valuetext={effortTitle}
              disabled={locked || reasoning.supportedEfforts.length < 2}
              onChange={event => { const effort = reasoningEffortAtIndex(reasoning.supportedEfforts, event.currentTarget.valueAsNumber); if (effort) chooseEffort(effort); }} />
          </div>
          <div className="assistant-composer-controls__range-labels" aria-hidden="true"><span>{effortLabel(reasoning.supportedEfforts[0])}</span><span>{effortLabel(reasoning.supportedEfforts[reasoning.supportedEfforts.length - 1])}</span></div>
        </div> : null}
        {reasoning.mode === "supported" && (sliderIndex === null || reasoning.supportedEfforts.length === 1) ? <div className="assistant-composer-controls__effort-options" role="group" aria-label={t("추론 강도 선택")}>
          {reasoning.supportedEfforts.map(effort => <button type="button" key={effort} aria-pressed={reasoningEffort === effort} disabled={locked} onClick={() => chooseEffort(effort)}>{effortLabel(effort)}</button>)}
        </div> : null}
        {reasoning.mode === "supported" && sliderIndex === null && !reasoning.stale ? <p className="assistant-composer-controls__detail">{t("기본 추론 단계가 확인되지 않았습니다. 사용할 강도를 직접 선택하거나 CLI 기본값을 유지하세요.")}</p> : null}
        {reasoningDetail ? <p className="assistant-composer-controls__detail">{t(reasoningDetail)}</p> : null}
      </> : <>
        <div className="assistant-composer-controls__heading"><strong>{t("AI 모델")}</strong><button type="button" className="assistant-composer-controls__icon-button" aria-label={t("닫기")} onClick={close}><X size={18} aria-hidden="true" /></button></div>
        <div role="listbox" aria-label={provider ? t("{{provider}} 모델 선택", { provider: provider.label }) : t("모델 선택")}
          className="assistant-composer-controls__model-options" onKeyDown={moveModelFocus}>
          {selection !== "required" ? <button type="button" role="option" aria-selected={!selected && !catalogChoiceLocked} disabled={catalogChoiceLocked} onClick={() => chooseModel("")}><span>{catalogChoiceLocked && catalog.placeholder ? t(catalog.placeholder) : t("CLI 기본값")}</span>{!selected && !catalogChoiceLocked ? <Check size={17} aria-hidden="true" /> : null}</button> : null}
          {models.map(item => <button type="button" role="option" key={item.id} aria-selected={selected === item.id} title={`${item.label} · ${item.id}`} aria-label={`${item.label} · ${item.id}`}
            onClick={() => chooseModel(item.id)}><span>{item.label}</span>{selected === item.id ? <Check size={17} aria-hidden="true" /> : null}</button>)}
        </div>
      </>}
      {models.length === 0 && catalog.detail ? <p className="assistant-composer-controls__detail">{t(catalog.detail)}</p> : null}
      {models.length === 0 && catalog.referenceModels.length ? <section className="assistant-composer-controls__reference" aria-label={t("공식 모델 예시")}>
        <strong>{catalog.referenceUrl ? <a className="assistant-composer-controls__documentation-link" href={catalog.referenceUrl} target="_blank" rel="noopener noreferrer"
          onClick={event => openDocumentation(event, catalog.referenceUrl!)}>{t("공식 모델 예시")}</a> : t("공식 모델 예시")}</strong>
        <p>{t("설치·계정 사용 가능 여부는 아직 확인하지 않았습니다.")}</p>
        <ul role="list">{catalog.referenceModels.map(item => <li key={item.id}><span translate="no">{item.label}</span><code translate="no">{item.id}</code></li>)}</ul>
      </section> : null}
      {models.length === 0 && catalog.documentationUrl ? <p className="assistant-composer-controls__detail">
        <a className="assistant-composer-controls__documentation-link" href={catalog.documentationUrl} target="_blank" rel="noopener noreferrer"
          onClick={event => openDocumentation(event, catalog.documentationUrl!)}>{t("공식 CLI 설치·모델 안내")}</a>
      </p> : null}
      {documentationError && (documentationError === catalog.documentationUrl || documentationError === catalog.referenceUrl)
        ? <p className="assistant-composer-controls__warning" role="alert">{t("공식 안내를 열지 못했습니다.")}</p> : null}
    </div> : null}
  </div>;
}
