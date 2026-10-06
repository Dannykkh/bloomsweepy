import { useSyncExternalStore } from "react";
import type { AssistantProviderKind } from "../types";
import {
  ASSISTANT_MODEL_PREFERENCE_KEY,
  LEGACY_ASSISTANT_PROVIDER_KEY,
  LEGACY_OLLAMA_MODEL_KEY,
  emptyAssistantModelPreference,
  readAssistantModelPreference,
  withAssistantModel,
  withAssistantProvider,
  withAssistantReasoningEffort,
  assistantReasoningEffortFor,
  writeAssistantModelPreference,
  type AssistantModelPreference,
  type AssistantModelPreferenceSnapshot,
} from "../lib/assistantModelPreference";

const changeEvent = "bloomsweepy-assistant-model-preference-changed";
const serverSnapshot: AssistantModelPreferenceSnapshot = {
  preference: emptyAssistantModelPreference(), storageError: false, hasSavedProvider: false,
};
let snapshot: AssistantModelPreferenceSnapshot | undefined;

function readBrowserPreference(): AssistantModelPreferenceSnapshot {
  try { return readAssistantModelPreference(window.localStorage); }
  catch { return { preference: emptyAssistantModelPreference(), storageError: true, hasSavedProvider: false }; }
}

function getSnapshot(): AssistantModelPreferenceSnapshot {
  if (!snapshot) snapshot = typeof window === "undefined" ? serverSnapshot : readBrowserPreference();
  return snapshot;
}

function subscribe(listener: () => void): () => void {
  const onStorage = (event: StorageEvent) => {
    if (event.key !== null && event.key !== ASSISTANT_MODEL_PREFERENCE_KEY
      && event.key !== LEGACY_ASSISTANT_PROVIDER_KEY && event.key !== LEGACY_OLLAMA_MODEL_KEY) return;
    snapshot = readBrowserPreference();
    listener();
  };
  window.addEventListener(changeEvent, listener);
  window.addEventListener("storage", onStorage);
  return () => {
    window.removeEventListener(changeEvent, listener);
    window.removeEventListener("storage", onStorage);
  };
}

function updatePreference(next: AssistantModelPreference): void {
  if (next === getSnapshot().preference) return;
  let saved = false;
  try { saved = writeAssistantModelPreference(window.localStorage, next); } catch { /* Keep the current choice. */ }
  snapshot = { preference: next, storageError: !saved, hasSavedProvider: true };
  window.dispatchEvent(new Event(changeEvent));
}

function setProvider(provider: AssistantProviderKind): void {
  updatePreference(withAssistantProvider(getSnapshot().preference, provider));
}

function setModel(provider: AssistantProviderKind, model: string): void {
  updatePreference(withAssistantModel(getSnapshot().preference, provider, model));
}

function setReasoningEffort(provider: AssistantProviderKind, model: string, effort: string): void {
  updatePreference(withAssistantReasoningEffort(getSnapshot().preference, provider, model, effort));
}

export function useAssistantModelPreference() {
  const current = useSyncExternalStore(subscribe, getSnapshot, () => serverSnapshot);
  return {
    provider: current.preference.provider,
    modelFor: (provider: AssistantProviderKind) => current.preference.models[provider] ?? "",
    setProvider,
    setModel,
    reasoningEffortFor: (provider: AssistantProviderKind, model: string) => assistantReasoningEffortFor(current.preference, provider, model),
    setReasoningEffort,
    storageError: current.storageError,
    hasSavedProvider: current.hasSavedProvider,
  };
}
