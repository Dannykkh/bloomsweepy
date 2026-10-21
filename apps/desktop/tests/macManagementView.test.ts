import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import test from "node:test";
import { isValidElement, type ReactElement } from "react";
import ts from "typescript";
import * as settings from "../src/lib/macSettings.ts";
import type { MacSetting, MacSettingChange, MacSettingsSnapshot } from "../src/lib/macSettings.ts";

const ids = ["show_extensions", "show_hidden_files", "show_path_bar", "show_status_bar", "folders_first"];
const first = "파일 확장자 표시";
const saved = "설정값을 저장하고 다시 확인했습니다. Finder 화면에는 다시 열 때 반영될 수 있습니다.";
const restored = "원래 사용자 설정으로 복원하고 다시 확인했습니다.";

function snapshot(value: boolean | null = null, history: MacSettingChange[] = []): MacSettingsSnapshot {
  return {
    supported: true, platform: "macos", warnings: [], history,
    settings: ids.map((id) => ({ id, title: id, description: "Synthetic preference", domain: "synthetic", key: id,
      value, state: value === null ? "unset" : "value", managed: false, canModify: true })),
  };
}
const change: MacSettingChange = { id: "synthetic-change-1", settingId: ids[0], previousValue: null,
  appliedValue: true, status: "applied", createdAtUnixMs: 1, canRestore: true };

const require = createRequire(import.meta.url);
const compiled = ts.transpileModule(readFileSync(new URL("../src/views/MacManagementView.tsx", import.meta.url), "utf8"), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX, target: ts.ScriptTarget.ES2022 },
}).outputText;
type Element = ReactElement<Record<string, unknown>>;
function elements(node: unknown): Element[] {
  if (Array.isArray(node)) return node.flatMap(elements);
  if (!isValidElement<Record<string, unknown>>(node)) return [];
  return [node, ...elements(node.props.children)];
}
function text(node: unknown): string {
  if (Array.isArray(node)) return node.map(text).join("");
  if (isValidElement<Record<string, unknown>>(node)) return text(node.props.children);
  return typeof node === "string" || typeof node === "number" ? String(node) : "";
}
function button(nodes: Element[], name: string): Element {
  const found = nodes.find((node) => node.type === "button"
    && (node.props["aria-label"] === name || text(node) === name));
  assert.ok(found, `Expected button: ${name}`);
  return found;
}
function click(node: Element) {
  assert.notEqual(node.props.disabled, true, `Cannot click a disabled button: ${text(node)}`);
  assert.equal(typeof node.props.onClick, "function");
  (node.props.onClick as () => void)();
}
function deferred<Value>() {
  let resolve!: (value: Value) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<Value>((accept, fail) => { resolve = accept; reject = fail; });
  return { promise, resolve, reject };
}
const settle = () => new Promise<void>((resolve) => setImmediate(resolve));
type ApplyRequest = { settingId: string; enabled: boolean; expectedValue: boolean | null };

// Match the Windows view harness: execute the real component/handlers, replacing
// hooks and native functions only. No browser, window IPC, or OS settings are used.
function createViewHarness(options: {
  read?: () => Promise<MacSettingsSnapshot>;
  apply?: (request: ApplyRequest) => Promise<MacSettingsSnapshot>;
  restore?: (changeId: string) => Promise<MacSettingsSnapshot>;
} = {}) {
  const slots: unknown[] = [];
  let cursor = 0;
  let updates = 0;
  let effect: (() => void | (() => void)) | undefined;
  let cleanup: void | (() => void);
  const calls = { reads: 0, applies: [] as ApplyRequest[], restores: [] as string[] };
  const module = { exports: {} as { MacManagementView?: () => unknown } };
  new Function("require", "module", "exports", compiled)((id: string) => {
    if (id === "react") return {
      useState: (initial: unknown) => {
        const index = cursor++;
        if (!(index in slots)) slots[index] = typeof initial === "function" ? initial() : initial;
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
    if (id === "../i18n") return { useLanguage: () => ({ t: (key: string) => key, appText: (text: string) => text }) };
    if (id === "../lib/format") return { formatDate: String };
    if (id === "../lib/macSettings") return {
      ...settings,
      getMacSettings: () => { calls.reads += 1; return options.read?.() ?? Promise.resolve(snapshot()); },
      applyMacSetting: (settingId: string, enabled: boolean, expectedValue: boolean | null) => {
        const request = { settingId, enabled, expectedValue };
        calls.applies.push(request);
        assert.ok(options.apply, "An unexpected native apply must never fall through to IPC");
        return options.apply(request);
      },
      restoreMacSetting: (changeId: string) => {
        calls.restores.push(changeId);
        assert.ok(options.restore, "An unexpected native restore must never fall through to IPC");
        return options.restore(changeId);
      },
    };
    if (id === "./MacManagementView.css") return {};
    assert.fail(`Unexpected runtime dependency: ${id}`);
  }, module, module.exports);
  assert.ok(module.exports.MacManagementView);
  const render = () => { cursor = 0; return elements(module.exports.MacManagementView!()); };
  return {
    calls, render,
    mount: () => { const nodes = render(); assert.ok(effect); cleanup = effect(); return nodes; },
    reconnect: () => { if (cleanup) cleanup(); assert.ok(effect); cleanup = effect(); },
    unmount: () => { if (cleanup) cleanup(); },
    updateCount: () => updates,
  };
}

test("real Mac view keeps an absent value unselected and applies only an explicit draft with expectedValue:null", async () => {
  const harness = createViewHarness({ apply: async () => snapshot(false, [{ ...change, appliedValue: false }]) });
  harness.mount(); await settle();
  const initial = harness.render();
  for (const label of ["끄기", "켜기"]) assert.equal(button(initial, `${first} · ${label}`).props["aria-pressed"], false);
  assert.equal(button(initial, `${first} · 적용`).props.disabled, true);
  click(button(initial, `${first} · 끄기`));
  const staged = harness.render();
  assert.equal(button(staged, `${first} · 끄기`).props["aria-pressed"], true);
  assert.equal(button(staged, `${first} · 적용`).props.disabled, false);
  assert.deepEqual(harness.calls.applies, [], "Choosing a value is not a native write");
  assert.ok(staged.some((node) => node.props.children === "아직 적용 안 됨"));
  click(button(staged, `${first} · 적용`)); await settle();
  assert.deepEqual(harness.calls.applies, [{ settingId: ids[0], enabled: false, expectedValue: null }]);
  assert.ok(harness.render().some((node) => node.props.children === saved));
  harness.unmount();
});

test("a draft can be cancelled and an unchanged explicit value never enables apply", async () => {
  const harness = createViewHarness({ read: async () => snapshot(true) });
  harness.mount(); await settle();
  click(button(harness.render(), `${first} · 끄기`));
  assert.equal(button(harness.render(), `${first} · 적용`).props.disabled, false);
  click(button(harness.render(), `${first} · 끄기`));
  assert.equal(button(harness.render(), `${first} · 켜기`).props["aria-pressed"], true);
  assert.equal(button(harness.render(), `${first} · 적용`).props.disabled, true);
  click(button(harness.render(), `${first} · 켜기`));
  assert.equal(button(harness.render(), `${first} · 적용`).props.disabled, true);
  assert.deepEqual(harness.calls.applies, []);
  harness.unmount();
});

test("same-render apply clicks issue one write and the ref lock blocks staging and refresh while pending", async () => {
  const pending = deferred<MacSettingsSnapshot>();
  const harness = createViewHarness({ apply: () => pending.promise });
  harness.mount(); await settle();
  click(button(harness.render(), `${first} · 켜기`));
  const beforeCommit = harness.render();
  const apply = button(beforeCommit, `${first} · 적용`);
  click(apply); click(apply);
  click(button(beforeCommit, `${first} · 끄기`));
  click(button(beforeCommit, "상태 새로 확인"));
  assert.equal(harness.calls.applies.length, 1);
  assert.equal(harness.calls.reads, 1);
  const pendingNodes = harness.render();
  assert.equal(button(pendingNodes, `${first} · 끄기`).props.disabled, true);
  assert.equal(button(pendingNodes, `${first} · 켜기`).props["aria-pressed"], true);
  assert.equal(button(pendingNodes, `${first} · 적용`).props.disabled, true);
  pending.resolve(snapshot(true, [{ ...change }])); await settle();
  assert.equal(button(harness.render(), `${first} · 적용`).props.disabled, true);
  assert.equal(button(harness.render(), `${first} · 켜기`).props.disabled, false);
  harness.unmount();
});

test("managed, wrong-type and failed native rows remain visible but cannot stage or write", async () => {
  const value = snapshot();
  const blocked: Partial<MacSetting>[] = [{ state: "managed", managed: true }, { state: "typeMismatch" }, { state: "error" }];
  value.settings = value.settings.map((row, index) => ({ ...row, ...blocked[index], canModify: index >= blocked.length }));
  const names = [first, "숨김 파일 표시", "경로 막대 표시"];
  const harness = createViewHarness({ read: async () => value });
  harness.mount(); await settle();
  const nodes = harness.render();
  for (const name of names) {
    for (const label of ["끄기", "켜기", "적용"]) assert.equal(button(nodes, `${name} · ${label}`).props.disabled, true);
    (button(nodes, `${name} · 켜기`).props.onClick as () => void)();
  }
  for (const state of ["조직에서 관리", "설정 형식 불일치", "조회 실패"]) {
    assert.ok(nodes.some((node) => node.props.className?.toString().includes("mac-management-value") && text(node).includes(state)));
  }
  assert.deepEqual(harness.calls.applies, []);
  assert.equal(button(harness.render(), `${first} · 켜기`).props["aria-pressed"], false);
  harness.unmount();
});

test("failed apply stays stale without retrying until a successful explicit refresh supplies the next expected value", async () => {
  let reads = 0;
  const harness = createViewHarness({ read: async () => snapshot(++reads === 1 ? null : false),
    apply: async () => { throw new Error("Synthetic preference changed externally"); } });
  harness.mount(); await settle();
  click(button(harness.render(), `${first} · 켜기`));
  click(button(harness.render(), `${first} · 적용`)); await settle();
  const failed = harness.render();
  assert.ok(failed.some((node) => node.props.role === "alert" && text(node).includes("Synthetic preference changed externally")));
  assert.equal(button(failed, `${first} · 켜기`).props.disabled, true);
  assert.equal(button(failed, `${first} · 적용`).props.disabled, true);
  assert.equal(failed.some((node) => node.props.children === saved), false);
  assert.equal(harness.calls.reads, 1);
  assert.equal(harness.calls.applies.length, 1);
  click(button(failed, "상태 새로 확인")); await settle();
  assert.equal(button(harness.render(), `${first} · 끄기`).props["aria-pressed"], true);
  assert.equal(button(harness.render(), `${first} · 켜기`).props.disabled, false);
  click(button(harness.render(), `${first} · 켜기`));
  click(button(harness.render(), `${first} · 적용`)); await settle();
  assert.deepEqual(harness.calls.applies.map((request) => request.expectedValue), [null, false]);
  harness.unmount();
});

test("restore delegates only the journal ID once and a native conflict stays failed until refreshed", async () => {
  let reads = 0;
  const pending = deferred<MacSettingsSnapshot>();
  const harness = createViewHarness({ read: async () => ++reads === 1 ? snapshot(true, [{ ...change }])
    : snapshot(false, [{ ...change, status: "conflict", canRestore: false }]), restore: () => pending.promise });
  harness.mount(); await settle();
  const restore = button(harness.render(), `${first} · 복원`);
  click(restore); click(restore);
  assert.deepEqual(harness.calls.restores, [change.id]);
  pending.reject(new Error("Synthetic current value differs from the applied value")); await settle();
  const failed = harness.render();
  assert.ok(failed.some((node) => node.props.role === "alert" && text(node).includes("Synthetic current value differs")));
  assert.equal(button(failed, `${first} · 복원`).props.disabled, true);
  assert.equal(failed.some((node) => node.props.children === restored), false);
  assert.equal(harness.calls.reads, 1);
  click(button(failed, "상태 새로 확인")); await settle();
  const refreshed = harness.render();
  assert.equal(refreshed.some((node) => node.props["aria-label"] === `${first} · 복원`), false);
  assert.ok(refreshed.some((node) => node.props.children === "기록만 표시"));
  assert.ok(refreshed.some((node) => node.type === "p" && text(node).includes("현재 값이 달라 복원 불가")));
  assert.deepEqual(harness.calls.restores, [change.id]);
  harness.unmount();
});

test("a successful native restore preserves original absence rather than inventing an OFF value", async () => {
  const harness = createViewHarness({ read: async () => snapshot(true, [{ ...change }]),
    restore: async () => snapshot(null, [{ ...change, status: "restored", canRestore: false }]) });
  harness.mount(); await settle();
  click(button(harness.render(), `${first} · 복원`)); await settle();
  const nodes = harness.render();
  for (const label of ["끄기", "켜기"]) assert.equal(button(nodes, `${first} · ${label}`).props["aria-pressed"], false);
  assert.ok(nodes.some((node) => node.props.children === restored));
  assert.deepEqual(harness.calls.restores, [change.id]);
  harness.unmount();
});

for (const operation of ["read", "apply", "restore"] as const) {
  test(`late ${operation} results after unmount do not update state or publish success`, async () => {
    const pending = deferred<MacSettingsSnapshot>();
    const harness = createViewHarness({
      read: () => operation === "read" ? pending.promise : Promise.resolve(snapshot(operation === "restore" ? true : null,
        operation === "restore" ? [{ ...change }] : [])),
      apply: () => pending.promise, restore: () => pending.promise,
    });
    harness.mount(); await settle();
    if (operation === "apply") {
      click(button(harness.render(), `${first} · 켜기`));
      click(button(harness.render(), `${first} · 적용`));
    } else if (operation === "restore") click(button(harness.render(), `${first} · 복원`));
    harness.unmount();
    const before = harness.updateCount();
    pending.resolve(snapshot(true)); await settle();
    assert.equal(harness.updateCount(), before);
  });
}

test("generation checks ignore an older read settling after effect reconnection and a newer snapshot", async () => {
  const old = deferred<MacSettingsSnapshot>();
  const current = deferred<MacSettingsSnapshot>();
  let reads = 0;
  const harness = createViewHarness({ read: () => ++reads === 1 ? old.promise : current.promise });
  harness.mount(); harness.reconnect();
  current.resolve(snapshot(true)); await settle();
  assert.equal(button(harness.render(), `${first} · 켜기`).props["aria-pressed"], true);
  const before = harness.updateCount();
  old.resolve(snapshot(false)); await settle();
  assert.equal(harness.updateCount(), before);
  assert.equal(button(harness.render(), `${first} · 켜기`).props["aria-pressed"], true);
  assert.equal(harness.calls.reads, 2);
  harness.unmount();
});

test("a failed refresh preserves known values but locks writes without automatic retries", async () => {
  let reads = 0;
  const harness = createViewHarness({ read: async () => {
    if (++reads > 1) throw new Error("Synthetic read failed");
    return snapshot(true);
  } });
  harness.mount(); await settle();
  click(button(harness.render(), "상태 새로 확인")); await settle();
  const nodes = harness.render();
  assert.ok(nodes.some((node) => node.props.role === "alert"));
  assert.equal(button(nodes, `${first} · 켜기`).props["aria-pressed"], true);
  assert.equal(button(nodes, `${first} · 켜기`).props.disabled, true);
  assert.equal(harness.calls.reads, 2);
  assert.deepEqual(harness.calls.applies, []);
  harness.unmount();
});

for (const unavailable of ["read-error", "unsupported"] as const) {
  test(`${unavailable} never presents editable or guessed preference values`, async () => {
    const harness = createViewHarness({ read: async () => {
      if (unavailable === "read-error") throw new Error("Synthetic initial read denied");
      return { ...snapshot(), supported: false, platform: "windows", settings: [] };
    } });
    harness.mount(); await settle();
    const nodes = harness.render();
    assert.equal(nodes.some((node) => node.props["aria-label"] === `${first} · 켜기`), false);
    assert.equal(nodes.some((node) => node.props["aria-label"] === `${first} · 적용`), false);
    assert.equal(nodes.some((node) => node.props.role === "alert"), unavailable === "read-error");
    assert.deepEqual(harness.calls.applies, []);
    assert.deepEqual(harness.calls.restores, []);
    assert.equal(harness.calls.reads, 1);
    harness.unmount();
  });
}
