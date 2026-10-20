import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import test from "node:test";
import { isValidElement, type ReactElement } from "react";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import ts from "typescript";
import * as settings from "../src/lib/windowsSettings.ts";
import type { WindowsSetting, WindowsSettingsOpenResult, WindowsSettingsSnapshot } from "../src/lib/windowsSettings.ts";

const ids = ["explorer_hide_extensions", "explorer_hidden_files", "explorer_protected_files", "explorer_full_path", "legacy_copilot", "recall_snapshots", "recall_snapshots_user", "recall_availability", "notepad_ai", "paint_image_creator"];
const targets = ["folder_options", "storage", "startup", "apps", "taskbar", "privacy", "about", "optional_features"];
const row: WindowsSetting = { id: ids[0], title: "Native title", description: "Native description", category: "explorer", scope: "currentUser", state: "value", value: 0, valueType: "REG_DWORD", support: "storedValueOnly", targetId: "folder_options" };

function snapshot(): WindowsSettingsSnapshot {
  return {
    supported: true, platform: "windows", osBuild: "26100", osRevision: 0, edition: "Test edition",
    settings: ids.map((id, index) => ({ ...row, id, category: index < 4 ? "explorer" : "aiPolicy", scope: id === "recall_snapshots_user" || index < 4 ? "currentUser" : "machine", support: index < 4 ? "storedValueOnly" : index === 4 ? "deprecated" : "requiresVerification", targetId: index < 4 ? "folder_options" : "privacy" })),
    targets: targets.map((id) => ({ id, title: `Native ${id}`, available: id !== "startup" })), warnings: [],
  };
}

test("DWORD values remain literal numbers rather than feature ON/OFF claims", () => {
  for (const value of [0, 1, 2, 0xffff_ffff]) assert.equal(settings.windowsStoredValue({ ...row, value }), String(value));
  for (const value of [null, -1, 1.5, Number.NaN, Number.POSITIVE_INFINITY, 0x1_0000_0000]) {
    assert.equal(settings.windowsStoredValue({ ...row, value }), null);
    assert.equal(settings.windowsSettingStateMessage({ ...row, value }), "조회 실패");
  }
  assert.equal(settings.windowsStoredValue({ ...row, state: "unsupportedValue", value: 77 }), "77");
});

test("absent, denied, wrong-type and unknown states do not become a zero or false value", () => {
  for (const [state, label] of [
    ["unset", "저장값 없음"], ["accessDenied", "접근 거부"], ["typeMismatch", "설정 형식 불일치"], ["error", "조회 실패"],
  ] as const) {
    assert.equal(settings.windowsStoredValue({ ...row, state, value: 0 }), null);
    assert.equal(settings.windowsSettingStateMessage({ ...row, state }), label);
  }
  for (const value of ["unexpected", null, undefined]) assert.equal(settings.windowsSettingStateKey(value), "상태 확인 필요");
});

test("all ten fixed setting IDs have localized titles, including separate Recall scopes", () => {
  for (const id of ids) assert.ok(settings.windowsSettingTitle(id));
  assert.notEqual(settings.windowsSettingTitle("recall_snapshots"), settings.windowsSettingTitle("recall_snapshots_user"));
  assert.equal(settings.windowsSettingTitle("unknown"), undefined);
  assert.equal(settings.windowsSettingTitle("toString"), undefined);
});

test("only a supported platform and an available fixed target enable an official link", () => {
  const value = snapshot();
  assert.equal(settings.canOpenWindowsSettingsTarget(value, "storage"), true);
  assert.equal(settings.canOpenWindowsSettingsTarget(value, "startup"), false);
  assert.equal(settings.canOpenWindowsSettingsTarget({ ...value, supported: false }, "storage"), false);
  assert.equal(settings.canOpenWindowsSettingsTarget({ ...value, targets: [] }, "storage"), false);
  for (const id of ["ms-settings:privacy", "file:///test", "unknown", "__proto__"]) {
    assert.equal(settings.canOpenWindowsSettingsTarget({ ...value, targets: [...value.targets, { id, title: "Injected", available: true }] }, id), false);
  }
});

test("Windows IPC sends only read and fixed-target requests and preserves changed:false semantics", async () => {
  const previous = Object.getOwnPropertyDescriptor(globalThis, "window");
  Object.defineProperty(globalThis, "window", { configurable: true, value: {} });
  const calls: unknown[] = [];
  mockIPC((command, payload) => {
    calls.push({ command, payload });
    return command === "get_windows_settings" ? snapshot() : { targetId: "storage", requested: true, changed: false };
  });
  try {
    assert.equal((await settings.getWindowsSettings()).supported, true);
    assert.deepEqual(await settings.openWindowsSettingsTarget("storage"), { targetId: "storage", requested: true, changed: false });
    assert.deepEqual(calls, [
      { command: "get_windows_settings", payload: {} },
      { command: "open_windows_settings_target", payload: { targetId: "storage" } },
    ]);
  } finally {
    clearMocks();
    if (previous) Object.defineProperty(globalThis, "window", previous);
    else Reflect.deleteProperty(globalThis, "window");
  }
});

const require = createRequire(import.meta.url);
const compiled = ts.transpileModule(readFileSync(new URL("../src/views/WindowsManagementView.tsx", import.meta.url), "utf8"), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX, target: ts.ScriptTarget.ES2022 },
}).outputText;
type Element = ReactElement<Record<string, unknown>>;
function elements(node: unknown): Element[] {
  if (Array.isArray(node)) return node.flatMap(elements);
  if (!isValidElement<Record<string, unknown>>(node)) return [];
  return [node, ...elements(node.props.children)];
}
function deferred<Value>() {
  let resolve!: (value: Value) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<Value>((accept, fail) => { resolve = accept; reject = fail; });
  return { promise, resolve, reject };
}
const settle = () => new Promise<void>((resolve) => setImmediate(resolve));

// Real view and handlers; only hooks, IPC, icons, formatting, and translation are replaced.
function createViewHarness(read = () => Promise.resolve(snapshot()), open = (id: string): Promise<WindowsSettingsOpenResult> => Promise.resolve({ targetId: id, requested: true, changed: false })) {
  const slots: unknown[] = [];
  let cursor = 0;
  let updates = 0;
  let effect: (() => void | (() => void)) | undefined;
  let cleanup: void | (() => void);
  const calls = { reads: 0, opens: [] as string[] };
  const module = { exports: {} as { WindowsManagementView?: (props: unknown) => unknown } };
  new Function("require", "module", "exports", compiled)((id: string) => {
    if (id === "react") return {
      useState: (initial: unknown) => {
        const index = cursor++;
        if (!(index in slots)) slots[index] = initial;
        return [slots[index], (next: unknown) => { updates += 1; slots[index] = typeof next === "function" ? next(slots[index]) : next; }];
      },
      useRef: (initial: unknown) => {
        const index = cursor++;
        if (!(index in slots)) slots[index] = { current: initial };
        return slots[index];
      },
      useEffect: (callback: () => void | (() => void)) => { effect = callback; },
    };
    if (id === "react/jsx-runtime") return require(id);
    if (id === "lucide-react") return new Proxy({}, { get: () => () => null });
    if (id === "../i18n") return { useLanguage: () => ({ t: (key: string, params?: Record<string, unknown>) => key.replace(/\{\{(\w+)\}\}/g, (_match, name) => String(params?.[name] ?? name)) }) };
    if (id === "../lib/format") return { formatDate: String };
    if (id === "../lib/windowsSettings") return {
      ...settings,
      getWindowsSettings: () => { calls.reads += 1; return read(); },
      openWindowsSettingsTarget: (targetId: string) => { calls.opens.push(targetId); return open(targetId); },
    };
    if (id === "./WindowsManagementView.css") return {};
    assert.fail(`Unexpected runtime dependency: ${id}`);
  }, module, module.exports);
  assert.ok(module.exports.WindowsManagementView);
  const render = () => { cursor = 0; return elements(module.exports.WindowsManagementView!({})); };
  const reconnect = () => { if (cleanup) cleanup(); cleanup = effect!(); };
  return {
    calls, render, reconnect,
    mount: () => { const nodes = render(); cleanup = effect!(); return nodes; },
    unmount: () => { if (cleanup) cleanup(); },
    updateCount: () => updates,
  };
}

test("real view keeps official targets first and AI policies collapsed with separate Recall rows", async () => {
  const harness = createViewHarness();
  harness.mount(); await settle();
  const nodes = harness.render();
  const targetsIndex = nodes.findIndex((node) => node.props["aria-labelledby"] === "windows-targets-title");
  const explorerIndex = nodes.findIndex((node) => node.props["aria-labelledby"] === "windows-explorer-title");
  const policiesIndex = nodes.findIndex((node) => node.props.className === "windows-management-policies");
  assert.ok(targetsIndex >= 0 && targetsIndex < explorerIndex && explorerIndex < policiesIndex);
  assert.equal(nodes[policiesIndex].type, "details");
  assert.equal(nodes[policiesIndex].props.open, undefined);
  const rows = nodes.filter((node) => node.props.className === "windows-management-setting");
  assert.equal(rows.length, 10);
  assert.equal(nodes.filter((node) => node.type === "select" || node.props.role === "switch").length, 0);
  assert.ok(nodes.some((node) => node.type === "strong" && node.props.children === "Windows AI 정책 저장값 6개"));
  const recall = rows.filter((node) => node.key === "recall_snapshots" || node.key === "recall_snapshots_user");
  assert.equal(recall.length, 2);
  assert.ok(elements(recall[0]).some((node) => node.props.children === "컴퓨터 전체 정책"));
  assert.ok(elements(recall[1]).some((node) => node.props.children === "현재 사용자"));
  harness.unmount();
});

test("read request is single-flight across immediate clicks and effect reconnection", async () => {
  const pending = deferred<WindowsSettingsSnapshot>();
  const harness = createViewHarness(() => pending.promise);
  const nodes = harness.mount();
  const refresh = nodes.find((node) => node.type === "button" && node.props.className === "secondary-button")!;
  (refresh.props.onClick as () => void)();
  harness.reconnect();
  await settle();
  assert.equal(harness.calls.reads, 1);
  pending.resolve(snapshot()); await settle();
  assert.equal(harness.render().find((node) => node.props.className === "windows-management-status")?.props.children !== null, true);
  assert.equal(harness.render().find((node) => node.type === "button")?.props.disabled, false);
  harness.unmount();
});

test("official opening is single-flight and reports a request, not a changed setting", async () => {
  const pending = deferred<WindowsSettingsOpenResult>();
  const harness = createViewHarness(undefined, () => pending.promise);
  harness.mount(); await settle();
  const nodes = harness.render();
  const storage = nodes.find((node) => node.props["aria-label"] === "저장소 설정 · 설정 화면 열기")!;
  (storage.props.onClick as () => void)();
  (storage.props.onClick as () => void)();
  assert.deepEqual(harness.calls.opens, ["storage"]);
  pending.resolve({ targetId: "storage", requested: true, changed: false }); await settle();
  assert.ok(harness.render().some((node) => node.type === "span" && node.props.children === "요청을 전달했습니다. 설정 변경 완료를 의미하지 않습니다."));
  assert.equal(harness.calls.reads, 1);
  harness.unmount();
});

test("Mac unsupported result never presents Explorer rows or official-opening controls", async () => {
  const value = { ...snapshot(), supported: false, platform: "macos", settings: [], targets: [] };
  const harness = createViewHarness(() => Promise.resolve(value));
  harness.mount(); await settle();
  const nodes = harness.render();
  assert.ok(nodes.some((node) => node.type === "h3" && node.props.children === "이 운영체제에서는 Windows 설정을 지원하지 않습니다."));
  assert.equal(nodes.some((node) => node.props.className === "windows-management-setting"), false);
  assert.equal(nodes.some((node) => node.props["aria-labelledby"] === "windows-targets-title"), false);
  assert.deepEqual(harness.calls.opens, []);
  harness.unmount();
});

test("missing or unknown OS metadata stays unverified without guessing a Windows version", async () => {
  const value = { ...snapshot(), osBuild: null, osRevision: null, edition: " Unknown " };
  const harness = createViewHarness(() => Promise.resolve(value));
  harness.mount(); await settle();
  const nodes = harness.render();
  const data = nodes.filter((node) => node.type === "dd").map((node) => node.props.children);
  assert.deepEqual(data, ["windows", "미확인", "미확인", "미확인"]);
  harness.unmount();
});

test("late read results after unmount do not update the view", async () => {
  const pending = deferred<WindowsSettingsSnapshot>();
  const harness = createViewHarness(() => pending.promise);
  harness.mount(); await settle(); harness.unmount();
  const before = harness.updateCount();
  pending.resolve(snapshot()); await settle();
  assert.equal(harness.updateCount(), before);
});

test("a failed refresh keeps previous values visibly stale rather than replacing them with defaults", async () => {
  let reads = 0;
  const harness = createViewHarness(() => ++reads === 1 ? Promise.resolve(snapshot()) : Promise.reject(new Error("Read denied")));
  harness.mount(); await settle();
  const refresh = harness.render().find((node) => node.type === "button" && node.props.className === "secondary-button")!;
  (refresh.props.onClick as () => void)(); await settle();
  const nodes = harness.render();
  assert.ok(nodes.some((node) => node.props.role === "alert"));
  assert.ok(nodes.some((node) => node.props.className === "windows-management-settings is-stale"));
  assert.equal(nodes.filter((node) => node.props.className === "windows-management-setting").length, 10);
  assert.ok(nodes.some((node) => node.type === "p" && node.props.children === "표시된 값은 이전 조회 결과입니다. 상태를 새로 확인하세요."));
  assert.equal(harness.calls.reads, 2, "Read failure must not trigger an automatic retry");
  harness.unmount();
});

test("an initial read failure reports the error without claiming that earlier values are displayed", async () => {
  const harness = createViewHarness(() => Promise.reject(new Error("Initial read denied")));
  harness.mount(); await settle();
  const nodes = harness.render();
  assert.ok(nodes.some((node) => node.props.role === "alert"));
  assert.ok(nodes.some((node) => node.type === "p" && node.props.children === "Initial read denied"));
  assert.equal(nodes.some((node) => node.props.children === "표시된 값은 이전 조회 결과입니다. 상태를 새로 확인하세요."), false);
  assert.equal(nodes.some((node) => node.props.className === "windows-management-settings is-stale"), false);
  assert.equal(nodes.some((node) => node.props.className === "windows-management-setting"), false);
  assert.equal(harness.calls.reads, 1);
  harness.unmount();
});

test("unavailable links stay disabled and an unconfirmed open request is not a success", async () => {
  const harness = createViewHarness(undefined, (targetId) => Promise.resolve({ targetId, requested: false, changed: false }));
  harness.mount(); await settle();
  const nodes = harness.render();
  const startup = nodes.find((node) => node.props["aria-label"] === "시작 프로그램 설정 · 설정 화면 열기")!;
  assert.equal(startup.props.disabled, true);
  (startup.props.onClick as () => void)();
  assert.deepEqual(harness.calls.opens, []);
  const storage = nodes.find((node) => node.props["aria-label"] === "저장소 설정 · 설정 화면 열기")!;
  (storage.props.onClick as () => void)(); await settle();
  const after = harness.render();
  assert.ok(after.some((node) => node.props.role === "alert"));
  assert.ok(after.some((node) => node.type === "p" && node.props.children === "설정 화면 열기 요청을 확인하지 못했습니다."));
  assert.equal(after.some((node) => node.props.children === "요청을 전달했습니다. 설정 변경 완료를 의미하지 않습니다."), false);
  harness.unmount();
});

test("late official-opening acknowledgements after unmount are ignored", async () => {
  const pending = deferred<WindowsSettingsOpenResult>();
  const harness = createViewHarness(undefined, () => pending.promise);
  harness.mount(); await settle();
  const storage = harness.render().find((node) => node.props["aria-label"] === "저장소 설정 · 설정 화면 열기")!;
  (storage.props.onClick as () => void)(); harness.unmount();
  const before = harness.updateCount();
  pending.resolve({ targetId: "storage", requested: true, changed: false }); await settle();
  assert.equal(harness.updateCount(), before);
});
