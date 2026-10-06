import { useId } from "react";
import { useLanguage } from "../i18n";
import { assistantModelSelection, assistantReasoningStatus, type AssistantReasoningEffort } from "../lib/assistantModelPreference";
import type { AssistantProviderStatus } from "../types";
import "./AssistantModelPicker.css";

interface AssistantModelPickerProps {
  provider: AssistantProviderStatus | null;
  value: string;
  onChange: (value: string) => void;
  reasoningEffort?: string;
  onReasoningEffortChange?: (value: string) => void;
  busy?: boolean;
}

export function AssistantModelPicker({ provider, value, onChange, reasoningEffort = "", onReasoningEffortChange, busy = false }: AssistantModelPickerProps) {
  const { t } = useLanguage();
  const id = useId();
  const reasoningId = useId();
  const selection = assistantModelSelection(provider);
  const unsupported = selection === "unsupported";
  const models = unsupported ? [] : provider?.models ?? [];
  const selected = unsupported ? "" : value;
  const missing = Boolean(selected) && !models.some(model => model.id === selected);
  const source = provider?.modelCatalogSource
    ?? (provider?.provider === "ollama" ? "installed" : "unsupported");
  const detail = source === "cli" ? "CLI가 제공한 모델 목록입니다. 계정별 사용 가능 여부는 응답할 때 확인됩니다."
    : source === "bundled" ? "CLI 내장 목록입니다. 계정별 사용 가능 여부는 확인하지 않았습니다."
    : source === "aliases" ? "CLI 모델 별칭입니다. 실제 모델 버전은 공급자가 결정합니다."
    : source === "installed" ? "로컬에 설치된 모델만 선택할 수 있습니다."
    : source === "unavailable" ? "모델 목록을 확인하지 못했습니다. CLI 기본값으로 사용하거나 다시 확인하세요."
    : "현재 앱 연동에서는 CLI 기본 모델을 사용합니다.";
  const reasoning = assistantReasoningStatus(provider, value, reasoningEffort);
  const reasoningDetail = reasoning.mode === "supported" ? "선택한 모델이 지원하는 추론 강도만 표시합니다. 기본값은 CLI가 결정합니다."
    : reasoning.mode === "modelDefault" ? "추론 강도를 지정하려면 Codex 모델을 직접 선택하세요. CLI 기본 모델에서는 강도를 지정하지 않습니다."
    : reasoning.mode === "unavailable" ? "추론 지원 목록을 확인하지 못했습니다. 저장한 선택은 유지하며, 기본값으로 바꾸거나 목록을 다시 확인하세요."
    : "이 CLI 또는 모델에서는 앱의 추론 강도 선택을 지원하지 않습니다. 기본값을 사용합니다.";
  const effortLabels: Record<AssistantReasoningEffort, Parameters<typeof t>[0]> = {
    none: "추론 없음", minimal: "최소", low: "낮음", medium: "중간", high: "높음", xhigh: "매우 높음", max: "최대", ultra: "울트라",
  };
  const effortLabel = (effort: string) => Object.prototype.hasOwnProperty.call(effortLabels, effort)
    ? t(effortLabels[effort as AssistantReasoningEffort]) : effort;
  return <div className="assistant-model-picker">
    <div className="assistant-model-picker__fields">
    <div className="assistant-model-picker__control">
      <label htmlFor={id}>{t("AI 모델")}</label>
      <select id={id} name="assistantModel" className="assistant-model-select" aria-label={provider ? t("{{provider}} 모델 선택", { provider: provider.label }) : t("모델 선택")}
        aria-describedby={`${id}-detail`} value={selected}
        disabled={busy || !provider || unsupported || selection === "required" && models.length === 0}
        title={selected || t("CLI 기본값")} onChange={event => onChange(event.currentTarget.value)}>
        {selection === "required" ? <option value="" disabled>{t(models.length ? "모델 선택" : "설치된 모델 없음")}</option>
          : <option value="">{t("CLI 기본값")}</option>}
        {missing ? <option value={selected}>{t("{{model}} · 목록에서 확인되지 않음", { model: selected })}</option> : null}
        {models.map(model => <option key={model.id} value={model.id}>{model.label}</option>)}
      </select>
    </div>
    {onReasoningEffortChange ? <div className="assistant-model-picker__control assistant-model-picker__reasoning">
      <label htmlFor={reasoningId}>{t("추론 강도")}</label>
      <select id={reasoningId} name="assistantReasoningEffort" className="assistant-model-select"
        value={reasoningEffort} aria-label={t("추론 강도 선택")}
        aria-describedby={`${reasoningId}-detail${reasoning.stale ? ` ${reasoningId}-warning` : ""}`}
        aria-invalid={reasoning.stale || undefined}
        disabled={busy || reasoning.mode !== "supported"}
        onChange={event => onReasoningEffortChange(event.currentTarget.value)}>
        <option value="">{reasoning.defaultEffort ? t("CLI 기본값 · {{effort}}", { effort: effortLabel(reasoning.defaultEffort) }) : t("CLI 기본값")}</option>
        {reasoning.stale ? <option value={reasoningEffort}>{t("{{effort}} · 지원 목록에서 확인되지 않음", { effort: effortLabel(reasoningEffort) })}</option> : null}
        {reasoning.mode === "supported" ? reasoning.supportedEfforts.map(effort => <option key={effort} value={effort}>{effortLabel(effort)}</option>) : null}
      </select>
    </div> : null}
    </div>
    {selected && (missing || selected.length > 32) ? <span className="assistant-model-picker__value">{selected}</span> : null}
    <p id={`${id}-detail`} className="assistant-model-picker__detail">{t(detail)}</p>
    {onReasoningEffortChange ? <p id={`${reasoningId}-detail`} className="assistant-model-picker__detail">{t(reasoningDetail)}</p> : null}
    {onReasoningEffortChange && reasoning.stale ? <div id={`${reasoningId}-warning`} className="assistant-model-picker__warning" role="alert">
      <span>{t("저장한 추론 강도를 현재 모델에서 확인하지 못했습니다. 기본값 또는 지원되는 강도를 선택해야 질문을 보낼 수 있습니다.")}</span>
      <button type="button" className="text-button" disabled={busy} onClick={() => onReasoningEffortChange("")}>{t("추론 강도 기본값으로 변경")}</button>
    </div> : null}
  </div>;
}
