import { invoke } from "@tauri-apps/api/core";
import type { MessageKey } from "../i18n";

export type WindowsSettingState = "value" | "unset" | "accessDenied" | "typeMismatch" | "unsupportedValue" | "error";
export type WindowsSettingSupport = "storedValueOnly" | "requiresVerification" | "deprecated";
export type WindowsSettingsTargetId = "folder_options" | "storage" | "startup" | "apps" | "taskbar" | "privacy" | "about" | "optional_features";

export interface WindowsSetting {
  id: string;
  title: string;
  description: string;
  category: "explorer" | "aiPolicy";
  scope: "currentUser" | "machine";
  state: WindowsSettingState;
  value: number | null;
  valueType: string | null;
  support: WindowsSettingSupport;
  targetId: string | null;
}

export interface WindowsSettingsTarget {
  id: string;
  title: string;
  available: boolean;
}

export interface WindowsSettingsSnapshot {
  supported: boolean;
  platform: string;
  osBuild: string | null;
  osRevision: number | null;
  edition: string | null;
  settings: WindowsSetting[];
  targets: WindowsSettingsTarget[];
  warnings: string[];
}

export interface WindowsSettingsOpenResult {
  targetId: string;
  requested: boolean;
  changed: false;
}

const targetIds = new Set<string>(["folder_options", "storage", "startup", "apps", "taskbar", "privacy", "about", "optional_features"]);
const settingTitles: Record<string, MessageKey> = {
  explorer_hide_extensions: "파일 확장자 숨김 설정",
  explorer_hidden_files: "숨김 파일 표시 설정",
  explorer_protected_files: "보호된 운영체제 파일 표시 설정",
  explorer_full_path: "전체 경로 표시 설정",
  legacy_copilot: "이전 Windows Copilot 정책",
  recall_snapshots: "Recall 스냅샷 저장 정책 · 컴퓨터",
  recall_snapshots_user: "Recall 스냅샷 저장 정책 · 사용자",
  recall_availability: "Recall 사용 가능 정책",
  notepad_ai: "메모장 AI 정책",
  paint_image_creator: "그림판 이미지 생성 정책",
};

export function windowsSettingTitle(id: string): MessageKey | undefined {
  return Object.prototype.hasOwnProperty.call(settingTitles, id) ? settingTitles[id] : undefined;
}

export function isWindowsSettingsTargetId(value: string): value is WindowsSettingsTargetId {
  return targetIds.has(value);
}

export function canOpenWindowsSettingsTarget(snapshot: WindowsSettingsSnapshot, targetId: string): targetId is WindowsSettingsTargetId {
  return snapshot.supported && isWindowsSettingsTargetId(targetId)
    && snapshot.targets.some((target) => target.id === targetId && target.available);
}

export function getWindowsSettings(): Promise<WindowsSettingsSnapshot> {
  return invoke("get_windows_settings");
}

export function openWindowsSettingsTarget(targetId: WindowsSettingsTargetId): Promise<WindowsSettingsOpenResult> {
  return invoke("open_windows_settings_target", { targetId });
}

export function windowsStoredValue(setting: WindowsSetting): string | null {
  if (setting.state !== "value" && setting.state !== "unsupportedValue") return null;
  return typeof setting.value === "number" && Number.isInteger(setting.value)
    && setting.value >= 0 && setting.value <= 0xffff_ffff ? String(setting.value) : null;
}

export function windowsSettingStateKey(state: unknown): MessageKey {
  switch (state) {
    case "value": return "저장된 설정값";
    case "unset": return "저장값 없음";
    case "accessDenied": return "접근 거부";
    case "typeMismatch": return "설정 형식 불일치";
    case "unsupportedValue": return "알 수 없는 저장값";
    case "error": return "조회 실패";
    default: return "상태 확인 필요";
  }
}

// A missing, denied, or malformed registry value never becomes a guessed false/OFF.
export function windowsSettingStateMessage(setting: WindowsSetting): MessageKey {
  return setting.state === "value" && windowsStoredValue(setting) === null ? "조회 실패" : windowsSettingStateKey(setting.state);
}
