import assert from "node:assert/strict";
import test from "node:test";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { applyMacSetting, canStageMacSetting, getMacSettings, hasMacSettingChange, restoreMacSetting, type MacSetting } from "../src/lib/macSettings.ts";

const editable: MacSetting = { id: "show_extensions", title: "Extensions", description: "", domain: ".GlobalPreferences", key: "AppleShowAllExtensions", state: "value", value: true, managed: false, canModify: true };

test("Mac preference drafting preserves absence and only enables an actual explicit change", () => {
  assert.equal(hasMacSettingChange(editable, undefined), false);
  assert.equal(hasMacSettingChange(editable, true), false);
  assert.equal(hasMacSettingChange(editable, false), true);
  const absent = { ...editable, state: "unset" as const, value: null };
  assert.equal(hasMacSettingChange(absent, undefined), false);
  assert.equal(hasMacSettingChange(absent, false), true);
  assert.equal(hasMacSettingChange(absent, true), true);
});

test("managed, failed, wrong-type and unsupported preferences cannot be staged", () => {
  for (const state of ["managed", "error", "typeMismatch", "unsupported"] as const) {
    const setting = { ...editable, state, value: null };
    assert.equal(canStageMacSetting(setting), false);
    assert.equal(hasMacSettingChange(setting, false), false);
  }
  assert.equal(canStageMacSetting({ ...editable, managed: true }), false);
  assert.equal(canStageMacSetting({ ...editable, canModify: false }), false);
});

test("Mac settings IPC uses allowlisted IDs and compare-and-set values, not native shell commands", async () => {
  const requests: unknown[] = [];
  const previous = Object.getOwnPropertyDescriptor(globalThis, "window");
  Object.defineProperty(globalThis, "window", { configurable: true, value: {} });
  mockIPC((command, payload) => { requests.push({ command, payload }); return null; });
  try {
    await getMacSettings();
    await applyMacSetting("show_extensions", false, null);
    await applyMacSetting("show_path_bar", true, false);
    await restoreMacSetting("change-1");
    assert.deepEqual(requests, [
      { command: "get_mac_settings", payload: {} },
      { command: "apply_mac_setting", payload: { request: { settingId: "show_extensions", enabled: false, expectedValue: null } } },
      { command: "apply_mac_setting", payload: { request: { settingId: "show_path_bar", enabled: true, expectedValue: false } } },
      { command: "restore_mac_setting", payload: { request: { changeId: "change-1" } } },
    ]);
  } finally {
    clearMocks();
    if (previous) Object.defineProperty(globalThis, "window", previous);
    else Reflect.deleteProperty(globalThis, "window");
  }
});

test("failed apply propagates without retrying or falling back to defaults", async () => {
  let calls = 0;
  const previous = Object.getOwnPropertyDescriptor(globalThis, "window");
  Object.defineProperty(globalThis, "window", { configurable: true, value: {} });
  mockIPC(() => { calls += 1; throw new Error("Preference changed"); });
  try {
    await assert.rejects(applyMacSetting("show_hidden_files", true, null), /Preference changed/);
    assert.equal(calls, 1);
  } finally {
    clearMocks();
    if (previous) Object.defineProperty(globalThis, "window", previous);
    else Reflect.deleteProperty(globalThis, "window");
  }
});
