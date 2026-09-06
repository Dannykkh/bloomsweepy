import type { AssistantProviderStatus } from "../types.ts";

// The process health result takes precedence over file existence or auth hints.
export function assistantProviderStatusKey(provider: AssistantProviderStatus) {
  switch (provider.state) {
    case "notInstalled": return "{{provider}} · 설치 안 됨";
    case "broken": return "{{provider}} · CLI 실행 오류";
    case "incompatible": return "{{provider}} · 버전 호환 확인 필요";
    case "loginRequired": return "{{provider}} · 로그인 필요";
    case "serviceUnavailable": return "{{provider}} · 서비스 연결 필요";
    case "noModels": return "{{provider}} · 모델 없음";
    case "ready": return provider.authentication === "notRequired"
      ? "{{provider}} · 모델 {{count}}개"
      : "{{provider}} · CLI 준비됨";
    default: return "{{provider}} · 상태 확인 실패";
  }
}

export function assistantFailureMessage(reason: unknown): string | null {
  if (reason instanceof Error) return reason.message;
  if (typeof reason === "string") return reason;
  if (reason && typeof reason === "object" && "message" in reason && typeof reason.message === "string") return reason.message;
  return null;
}

export function isAssistantAuthenticationFailure(reason: unknown): boolean {
  return Boolean(reason && typeof reason === "object" && "kind" in reason && reason.kind === "authentication");
}
