import type { AssistantPermissionMode } from "../types";

export const ASSISTANT_PERMISSION_MODE_OPTIONS = [
  {
    mode: "read_only",
    label: "조회만",
    description: "로컬 파일을 조회하고 검사합니다. 변경이나 휴지통 이동은 실행하지 않습니다.",
  },
  {
    mode: "confirm",
    label: "확인 후 실행",
    description: "선택한 폴더에서 검토하고, 휴지통 이동은 실행 전에 확인합니다.",
  },
  {
    mode: "full_access",
    label: "전체 접근",
    description: "로컬 드라이브를 검색하고, 명확한 휴지통 요청은 추가 확인 없이 실행합니다.",
  },
] as const satisfies readonly {
  mode: AssistantPermissionMode;
  label: string;
  description: string;
}[];

export function isAssistantPermissionMode(value: unknown): value is AssistantPermissionMode {
  return value === "read_only" || value === "confirm" || value === "full_access";
}

export function normalizeAssistantPermissionMode(value: unknown): AssistantPermissionMode {
  return isAssistantPermissionMode(value) ? value : "confirm";
}

export function canReviewInMode(mode: unknown): boolean {
  // Older hosts omit this field and retain the existing review-and-confirm policy.
  return mode === undefined || mode === "confirm" || mode === "full_access";
}

export function canAutomaticallyTrashMode(mode: unknown, legacyFlag = false): boolean {
  return mode === "full_access" || (mode === undefined && legacyFlag === true);
}
