import type { EmptyTrashOutcome, EmptyTrashPlan } from "../types.ts";

export function canConfirmEmptyTrash(plan: EmptyTrashPlan | null, acknowledged: boolean, consumed: boolean, now: number): boolean {
  return !!plan && acknowledged && !consumed && Number.isFinite(plan.expiresAtUnixMs) && plan.expiresAtUnixMs > now;
}

export const emptyTrashOutcomeMessages = {
  requested: "운영체제에 비우기를 요청했습니다. 휴지통에서 완료 여부와 남은 항목을 확인하세요. 확보된 용량은 보장하지 않습니다.",
  cancelled: "운영체제에서 취소되었습니다. 일부 항목은 이미 삭제됐을 수 있으니 휴지통을 확인하세요.",
  permissionDenied: "운영체제가 권한을 거부했습니다. macOS에서는 시스템 설정 → 개인정보 보호 및 보안 → 자동화에서 Finder 권한을 확인하세요. 권한을 자동 변경하지 않습니다.",
  launchFailed: "운영체제 휴지통 기능을 시작하지 못했습니다. 휴지통을 직접 열어 확인하세요.",
  unconfirmed: "완료 여부를 확인하지 못했습니다. 운영체제 작업은 계속될 수 있습니다. 휴지통을 직접 확인하세요. 중복 삭제 요청을 막기 위해 앱을 다시 시작하기 전에는 재요청할 수 없습니다.",
} as const satisfies Record<EmptyTrashOutcome, string>;

export function emptyTrashErrorMessage(error: unknown) {
  if (error === "expired" || error === "invalidPlan") return "확인 시간이 만료됐습니다. 창을 닫고 다시 검토하세요.";
  if (error === "needsInspection") return emptyTrashOutcomeMessages.unconfirmed;
  if (error === "unsupported") return "휴지통 비우기는 macOS와 Windows에서 지원합니다.";
  return "작업을 시작할 수 없습니다. 진행 중인 작업과 시스템 자원을 확인한 뒤 다시 검토하세요.";
}
