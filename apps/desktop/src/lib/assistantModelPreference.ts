import type { AssistantProviderKind, AssistantProviderStatus } from "../types.ts";

export const ASSISTANT_MODEL_PREFERENCE_KEY = "bloomsweepy.assistant-models.v1";
export const LEGACY_ASSISTANT_PROVIDER_KEY = "bloomsweepy.assistant-provider";
export const LEGACY_OLLAMA_MODEL_KEY = "bloomsweepy.ollama-model";
export const ASSISTANT_PREFERENCE_MAX_BYTES = 4_096;
export const ASSISTANT_MODEL_ID_MAX_LENGTH = 160;
export const ASSISTANT_REASONING_CHOICE_LIMIT = 16;
export const ASSISTANT_REASONING_EFFORTS = ["none", "minimal", "low", "medium", "high", "xhigh", "max", "ultra"] as const;
export type AssistantReasoningEffort = typeof ASSISTANT_REASONING_EFFORTS[number];
const providerKinds: readonly AssistantProviderKind[] = ["codex", "claudeCode", "grok", "antigravity", "ollama"];

export interface AssistantModelPreference {
  version: 1;
  provider: AssistantProviderKind;
  models: Partial<Record<AssistantProviderKind, string>>;
  reasoningEfforts?: Partial<Record<AssistantProviderKind, Record<string, AssistantReasoningEffort>>>;
}

export interface AssistantModelPreferenceSnapshot {
  preference: AssistantModelPreference;
  storageError: boolean;
  hasSavedProvider: boolean;
}

type PreferenceStorage = Pick<Storage, "getItem" | "setItem">;

export function isAssistantProviderKind(value: unknown): value is AssistantProviderKind {
  return providerKinds.some(kind => kind === value);
}

export function normalizeAssistantModelId(value: unknown): string | null {
  if (value === "") return "";
  return typeof value === "string" && value.length <= ASSISTANT_MODEL_ID_MAX_LENGTH
    && /^[A-Za-z0-9][A-Za-z0-9._:/-]*$/.test(value) ? value : null;
}

export function normalizeAssistantReasoningEffort(value: unknown): AssistantReasoningEffort | "" | null {
  if (value === "") return "";
  return ASSISTANT_REASONING_EFFORTS.find(effort => effort === value) ?? null;
}

export function emptyAssistantModelPreference(): AssistantModelPreference {
  return { version: 1, provider: "codex", models: {} };
}

export function parseAssistantModelPreference(raw: string | null): AssistantModelPreference | null {
  if (!raw || raw.length > ASSISTANT_PREFERENCE_MAX_BYTES) return null;
  try {
    const value: unknown = JSON.parse(raw);
    if (!value || typeof value !== "object" || Array.isArray(value)) return null;
    const record = value as Record<string, unknown>;
    if (record.version !== 1 || !isAssistantProviderKind(record.provider)
      || !record.models || typeof record.models !== "object" || Array.isArray(record.models)) return null;
    const models: AssistantModelPreference["models"] = {};
    const storedModels = record.models as Record<string, unknown>;
    for (const kind of providerKinds) {
      const id = normalizeAssistantModelId(storedModels[kind]);
      if (id !== null) models[kind] = id;
    }
    const reasoningEfforts: NonNullable<AssistantModelPreference["reasoningEfforts"]> = {};
    let choiceCount = 0;
    if (record.reasoningEfforts && typeof record.reasoningEfforts === "object" && !Array.isArray(record.reasoningEfforts)) {
      const saved = record.reasoningEfforts as Record<string, unknown>;
      const activeModel = models[record.provider];
      const activeValues = saved[record.provider];
      if (activeModel && activeValues && typeof activeValues === "object" && !Array.isArray(activeValues)
        && Object.prototype.hasOwnProperty.call(activeValues, activeModel)) {
        const activeEffort = normalizeAssistantReasoningEffort((activeValues as Record<string, unknown>)[activeModel]);
        if (activeEffort) { reasoningEfforts[record.provider] = { [activeModel]: activeEffort }; choiceCount = 1; }
      }
      for (const kind of providerKinds) {
        const values = saved[kind];
        if (!values || typeof values !== "object" || Array.isArray(values)) continue;
        const choices: Record<string, AssistantReasoningEffort> = reasoningEfforts[kind] ?? {};
        for (const [model, rawEffort] of Object.entries(values)) {
          const id = normalizeAssistantModelId(model);
          const effort = normalizeAssistantReasoningEffort(rawEffort);
          if (id && effort && !Object.prototype.hasOwnProperty.call(choices, id) && choiceCount < ASSISTANT_REASONING_CHOICE_LIMIT) {
            choices[id] = effort; choiceCount++;
          }
        }
        if (Object.keys(choices).length) reasoningEfforts[kind] = choices;
      }
    }
    return { version: 1, provider: record.provider, models,
      ...(choiceCount ? { reasoningEfforts } : {}) };
  } catch { return null; }
}

export function readAssistantModelPreference(storage: Pick<PreferenceStorage, "getItem">): AssistantModelPreferenceSnapshot {
  const preference = emptyAssistantModelPreference();
  try {
    const saved = parseAssistantModelPreference(storage.getItem(ASSISTANT_MODEL_PREFERENCE_KEY));
    if (saved) return { preference: saved, storageError: false, hasSavedProvider: true };
    const legacyProvider = storage.getItem(LEGACY_ASSISTANT_PROVIDER_KEY);
    const hasSavedProvider = isAssistantProviderKind(legacyProvider);
    if (hasSavedProvider) preference.provider = legacyProvider;
    const legacyModel = normalizeAssistantModelId(storage.getItem(LEGACY_OLLAMA_MODEL_KEY));
    if (legacyModel !== null) preference.models.ollama = legacyModel;
    return { preference, storageError: false, hasSavedProvider };
  } catch { return { preference, storageError: true, hasSavedProvider: false }; }
}

export function withAssistantProvider(preference: AssistantModelPreference, provider: AssistantProviderKind): AssistantModelPreference {
  return isAssistantProviderKind(provider) ? { ...preference, provider } : preference;
}

export function withAssistantModel(preference: AssistantModelPreference, provider: AssistantProviderKind, model: string): AssistantModelPreference {
  const id = normalizeAssistantModelId(model);
  return isAssistantProviderKind(provider) && id !== null
    ? { ...preference, models: { ...preference.models, [provider]: id } } : preference;
}

export function assistantReasoningEffortFor(preference: AssistantModelPreference, provider: AssistantProviderKind, model: string): AssistantReasoningEffort | "" {
  if (!isAssistantProviderKind(provider) || !normalizeAssistantModelId(model)) return "";
  const choices = preference.reasoningEfforts?.[provider];
  if (!choices || !Object.prototype.hasOwnProperty.call(choices, model)) return "";
  return normalizeAssistantReasoningEffort(choices[model]) || "";
}

export function withAssistantReasoningEffort(preference: AssistantModelPreference, provider: AssistantProviderKind, model: string, value: string): AssistantModelPreference {
  const id = normalizeAssistantModelId(model);
  const effort = normalizeAssistantReasoningEffort(value);
  if (!isAssistantProviderKind(provider) || !id || effort === null) return preference;
  const choices: NonNullable<AssistantModelPreference["reasoningEfforts"]> = {};
  for (const kind of providerKinds) {
    const saved = preference.reasoningEfforts?.[kind];
    if (saved) choices[kind] = { ...saved };
  }
  const current = choices[provider] ?? {};
  delete current[id];
  if (effort) current[id] = effort;
  if (Object.keys(current).length) choices[provider] = current;
  else delete choices[provider];
  // minimal: 16개 모델의 추론 선호만 보관 — 16개를 넘는 모델 선호 보존이 실제로 필요하면 확장 검토.
  const entries = providerKinds.flatMap(kind => Object.keys(choices[kind] ?? {}).map(modelId => ({ kind, modelId })));
  while (entries.length > ASSISTANT_REASONING_CHOICE_LIMIT) {
    const activeModel = preference.models[preference.provider];
    const index = entries.findIndex(entry => (entry.kind !== provider || entry.modelId !== id)
      && (entry.kind !== preference.provider || entry.modelId !== activeModel));
    const removed = entries.splice(index, 1)[0];
    delete choices[removed.kind]![removed.modelId];
    if (!Object.keys(choices[removed.kind]!).length) delete choices[removed.kind];
  }
  return { version: 1, provider: preference.provider, models: preference.models,
    ...(entries.length ? { reasoningEfforts: choices } : {}) };
}

export function writeAssistantModelPreference(storage: Pick<PreferenceStorage, "setItem">, preference: AssistantModelPreference): boolean {
  const raw = JSON.stringify(preference);
  const normalized = parseAssistantModelPreference(raw);
  if (!normalized) return false;
  try { storage.setItem(ASSISTANT_MODEL_PREFERENCE_KEY, JSON.stringify(normalized)); return true; }
  catch { return false; }
}

/** Older hosts only offered an explicit model for Ollama. */
export function assistantModelSelection(provider: AssistantProviderStatus | null): "optional" | "required" | "unsupported" {
  return provider?.modelSelection ?? (provider?.provider === "ollama" ? "required" : "unsupported");
}

export function assistantModelRequestValue(provider: AssistantProviderStatus | null, model: string): string | null {
  return assistantModelSelection(provider) === "unsupported" ? null : normalizeAssistantModelId(model) || null;
}

export function assistantReasoningStatus(provider: AssistantProviderStatus | null, model: string, value: string) {
  const selected = provider?.models.find(item => item.id === model);
  const supplied = selected?.supportedReasoningEfforts;
  const supportedEfforts = Array.isArray(supplied)
    ? [...new Set(supplied.map(normalizeAssistantReasoningEffort).filter((effort): effort is AssistantReasoningEffort => Boolean(effort)))] : [];
  const mode = !provider ? "unavailable"
    : provider.provider !== "codex" || assistantModelSelection(provider) === "unsupported" ? "unsupported"
    : !model ? "modelDefault"
    : provider.modelCatalogSource === "unavailable" || !selected ? "unavailable"
    : !supportedEfforts.length ? "unsupported" : "supported";
  const effort = normalizeAssistantReasoningEffort(value);
  const stale = Boolean(value) && (mode !== "supported" || !effort || !supportedEfforts.includes(effort));
  const catalogDefault = normalizeAssistantReasoningEffort(selected?.defaultReasoningEffort);
  const defaultEffort = catalogDefault && supportedEfforts.includes(catalogDefault) ? catalogDefault : null;
  return { mode, supportedEfforts, defaultEffort, stale,
    requestValue: !stale && mode === "supported" && effort ? effort : null } as const;
}
