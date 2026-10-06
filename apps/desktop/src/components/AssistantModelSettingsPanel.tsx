import { Bot, RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";
import { getAssistantProviderStatus } from "../lib/bridge";
import { assistantProviderStatusKey, assistantFailureMessage } from "../lib/assistantProviderStatus";
import { useAssistantModelPreference } from "../hooks/useAssistantModelPreference";
import { useLanguage } from "../i18n";
import { AssistantModelPicker } from "./AssistantModelPicker";
import type { AssistantProviderKind, AssistantProviderStatus } from "../types";
import "./AssistantModelSettingsPanel.css";

export function AssistantModelSettingsPanel() {
  const { t } = useLanguage();
  const preference = useAssistantModelPreference();
  const [providers, setProviders] = useState<AssistantProviderStatus[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [retry, setRetry] = useState(0);
  useEffect(() => {
    let disposed = false;
    setLoading(true);
    setError(null);
    void getAssistantProviderStatus().then(next => {
      if (!disposed) setProviders(next);
    }).catch(reason => {
      if (!disposed) setError(assistantFailureMessage(reason) ?? t("AI CLI 응답을 받지 못했습니다"));
    }).finally(() => { if (!disposed) setLoading(false); });
    return () => { disposed = true; };
  }, [retry, t]);

  const provider = providers.find(item => item.provider === preference.provider) ?? null;
  return <section className="settings-panel assistant-model-settings">
    <div className="settings-panel__heading">
      <Bot size={20} aria-hidden="true" />
      <div><h2>{t("AI 모델")}</h2><p>{t("대화에 사용할 CLI와 모델을 선택합니다.")}</p></div>
      <button type="button" className="text-button" aria-label={t("AI CLI 상태 다시 확인")}
        disabled={loading} onClick={() => setRetry(value => value + 1)}>
        <RefreshCw size={18} aria-hidden="true" />
      </button>
    </div>
    <div className="assistant-model-settings__controls">
      <label className="assistant-model-settings__provider">
        <span>{t("대화 상대 선택")}</span>
        <select name="assistantDefaultProvider" className="assistant-model-select" value={preference.provider}
          disabled={loading || Boolean(provider?.busy) || providers.length === 0}
          onChange={event => preference.setProvider(event.currentTarget.value as AssistantProviderKind)}>
          {!providers.some(item => item.provider === preference.provider)
            ? <option value={preference.provider}>{loading ? t("AI CLI 확인 중") : preference.provider}</option> : null}
          {providers.map(item => <option key={item.provider} value={item.provider}>
            {t(assistantProviderStatusKey(item), { provider: item.label, count: item.models.length })}
          </option>)}
        </select>
      </label>
      <AssistantModelPicker provider={provider} value={preference.modelFor(preference.provider)}
        onChange={value => preference.setModel(preference.provider, value)}
        reasoningEffort={preference.reasoningEffortFor(preference.provider, preference.modelFor(preference.provider))}
        onReasoningEffortChange={value => preference.setReasoningEffort(preference.provider, preference.modelFor(preference.provider), value)}
        busy={loading || Boolean(provider?.busy)} />
    </div>
    <p className="settings-inline-state">{t("선택은 이 컴퓨터에만 저장됩니다. CLI의 전역 설정은 바꾸지 않습니다.")}</p>
    {loading ? <p className="settings-inline-state" role="status">{t("AI CLI 확인 중")}</p> : null}
    {error ? <p className="settings-inline-error" role="alert">{error}</p> : null}
    {preference.storageError ? <p className="settings-inline-error" role="alert">
      {t("모델 선택을 저장하지 못했습니다. 현재 실행 중에는 선택한 모델을 사용합니다.")}
    </p> : null}
  </section>;
}
