import { invoke } from "@tauri-apps/api/core";

export type MacSettingState = "value" | "unset" | "managed" | "typeMismatch" | "error" | "unsupported";

export interface MacSetting {
  id: string;
  title: string;
  description: string;
  domain: string;
  key: string;
  value: boolean | null;
  state: MacSettingState;
  managed: boolean;
  canModify: boolean;
}

export interface MacSettingChange {
  id: string;
  settingId: string;
  previousValue: boolean | null;
  appliedValue: boolean;
  status: string;
  createdAtUnixMs: number;
  canRestore: boolean;
}

export interface MacSettingsSnapshot {
  supported: boolean;
  platform: string;
  settings: MacSetting[];
  history: MacSettingChange[];
  warnings: string[];
}

export function getMacSettings(): Promise<MacSettingsSnapshot> {
  return invoke("get_mac_settings");
}

export function applyMacSetting(settingId: string, enabled: boolean, expectedValue: boolean | null): Promise<MacSettingsSnapshot> {
  return invoke("apply_mac_setting", { request: { settingId, enabled, expectedValue } });
}

export function restoreMacSetting(changeId: string): Promise<MacSettingsSnapshot> {
  return invoke("restore_mac_setting", { request: { changeId } });
}

// Absence and failed/managed reads never become a guessed false value.
export function canStageMacSetting(setting: MacSetting): boolean {
  return setting.canModify && !setting.managed && (setting.state === "value" || setting.state === "unset");
}

export function hasMacSettingChange(setting: MacSetting, draft: boolean | undefined): boolean {
  return typeof draft === "boolean" && canStageMacSetting(setting) && (setting.state === "unset" || draft !== setting.value);
}
