import type { AssistantReasoningEffort } from "./assistantModelPreference.ts";
import type { AssistantProviderStatus } from "../types.ts";

// minimal: official CLI examples verified 2026-10-07, not account catalogs —
// replace only this reference copy when the vendor changes its published selectors.
const referenceModels: Partial<Record<AssistantProviderStatus["provider"], readonly { id: string; label: string }[]>> = {
  grok: [
    { id: "grok-build", label: "Grok Build" },
    { id: "grok-4.7", label: "Grok 4.7" },
  ],
  antigravity: [
    { id: "gemini-3.8-flash-high", label: "Gemini 3.8 Flash (High)" },
    { id: "gemini-3.8-flash-medium", label: "Gemini 3.8 Flash (Medium)" },
    { id: "gemini-3.7-flash-high", label: "Gemini 3.7 Flash (High)" },
    { id: "gemini-3.7-flash-medium", label: "Gemini 3.7 Flash (Medium)" },
    { id: "gemini-3.6-flash-high", label: "Gemini 3.6 Flash (High)" },
    { id: "gemini-3.6-flash-medium", label: "Gemini 3.6 Flash (Medium)" },
    { id: "gemini-3.1-pro-high", label: "Gemini 3.1 Pro (High)" },
  ],
};

/** An empty catalog is not proof that the provider only supports its default model. */
export function assistantModelCatalogPresentation(provider: AssistantProviderStatus | null) {
  const documentationUrl = provider?.provider === "grok" ? "https://docs.x.ai/build/overview"
    : provider?.provider === "antigravity" ? "https://antigravity.google/docs/cli/install/" : null;
  const result = <P extends string, D extends string>(placeholder: P, detail: D) => ({
    unavailable: true, placeholder, detail, documentationUrl,
    referenceModels: provider ? referenceModels[provider.provider] ?? [] : [],
    referenceUrl: provider?.provider === "grok" ? "https://docs.x.ai/build/settings"
      : provider?.provider === "antigravity" ? "https://antigravity.google/docs/cli/headless/" : null,
  });
  if (provider?.state === "notInstalled") return result("CLI 설치 필요", "CLI를 설치한 뒤 다시 확인하면 모델 목록을 불러옵니다.");
  if (provider?.state === "broken" || provider?.state === "incompatible") {
    return result("CLI 확인 필요", "CLI 실행과 호환성을 확인해야 모델 목록을 불러올 수 있습니다.");
  }
  // A signed-out healthy CLI can still supply genuine metadata (e.g. Claude).
  if (provider?.models.length || provider?.modelSelection === "unsupported") {
    return { unavailable: false, placeholder: null, detail: null, documentationUrl: null, referenceModels: [], referenceUrl: null };
  }
  if (provider?.state === "loginRequired") return result("로그인 후 목록 확인", "로그인한 뒤 모델 목록을 다시 확인하세요.");
  if (provider?.state === "serviceUnavailable") return result("서비스 연결 필요", "서비스에 연결한 뒤 모델 목록을 다시 확인하세요.");
  if (provider?.state === "noModels") return result("설치된 모델 없음", "모델을 설치한 뒤 목록을 다시 확인하세요.");
  if (provider?.state === "checkFailed") return result("CLI 확인 필요", "CLI 실행과 호환성을 확인해야 모델 목록을 불러올 수 있습니다.");
  return result("모델 목록 확인 필요", "모델 목록을 다시 확인하세요. CLI 기본값 사용은 연결 상태와 별개입니다.");
}

/** A display position is not an override: an empty selection remains CLI-owned. */
export function reasoningSliderIndex(
  supportedEfforts: readonly AssistantReasoningEffort[],
  selectedEffort: string,
  defaultEffort: AssistantReasoningEffort | null,
): number | null {
  const shownEffort = selectedEffort || defaultEffort;
  if (!shownEffort) return null;
  const index = supportedEfforts.findIndex(effort => effort === shownEffort);
  return index < 0 ? null : index;
}

/** Reject rather than clamp an invalid index into an unintended effort. */
export function reasoningEffortAtIndex(
  supportedEfforts: readonly AssistantReasoningEffort[],
  index: number,
): AssistantReasoningEffort | null {
  return Number.isInteger(index) && index >= 0 && index < supportedEfforts.length
    ? supportedEfforts[index] : null;
}
