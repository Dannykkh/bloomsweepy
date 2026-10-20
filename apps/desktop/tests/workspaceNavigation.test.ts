import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import test from "node:test";
import { isValidElement, type ReactElement } from "react";
import ts from "typescript";
import type { ViewId } from "../src/types.ts";

const require = createRequire(import.meta.url);
const source = readFileSync(new URL("../src/components/AppShell.tsx", import.meta.url), "utf8");
const compiled = ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.CommonJS,
    jsx: ts.JsxEmit.ReactJSX,
    target: ts.ScriptTarget.ES2022,
  },
}).outputText;

type Element = ReactElement<Record<string, unknown>>;

function elements(node: unknown): Element[] {
  if (Array.isArray(node)) return node.flatMap(elements);
  if (!isValidElement<Record<string, unknown>>(node)) return [];
  return [node, ...elements(node.props.children)];
}

// Use the real shell render while leaving browser effects, icons, and localization outside this unit boundary.
function createHarness() {
  const module = { exports: {} as { AppShell?: (props: unknown) => unknown } };
  new Function("require", "module", "exports", compiled)((id: string) => {
    if (id === "react") return { useEffect: () => undefined, useRef: () => ({ current: null }), useState: () => [null, () => undefined] };
    if (id === "react/jsx-runtime") return require(id);
    if (id === "lucide-react") return new Proxy({}, { get: () => () => null });
    if (id === "../i18n") return { useLanguage: () => ({ t: (value: string) => value }) };
    if (id === "../lib/format") return { formatBytes: String, formatDate: String };
    if (id === "./WorkspaceNavigation.css") return {};
    assert.fail(`Unexpected runtime dependency: ${id}`);
  }, module, module.exports);
  assert.ok(module.exports.AppShell);

  const navigations: ViewId[] = [];
  const mobileChanges: boolean[] = [];
  const render = (options: {
    activeView?: ViewId;
    dockerEnabled?: boolean;
    macosAvailable?: boolean;
    windowsAvailable?: boolean;
  } = {}) => elements(module.exports.AppShell!({
    activeView: "dashboard",
    children: null,
    root: null,
    report: null,
    volume: null,
    mobileNavigationOpen: false,
    selectionBlocked: false,
    dockerEnabled: false,
    onMobileNavigationChange: (open: boolean) => mobileChanges.push(open),
    onNavigate: (view: ViewId) => navigations.push(view),
    onPickFolder: () => assert.fail("Rendering navigation must not select or scan a folder"),
    ...options,
  }));
  const navigation = (nodes: Element[]) => nodes.filter((node) =>
    node.type === "button" && String(node.props.className).split(" ").includes("nav-item"));
  return { render, navigation, navigations, mobileChanges };
}

test("workspace navigation keeps Dashboard, Performance, and Applications first in that order", () => {
  const harness = createHarness();
  for (const macosAvailable of [false, true]) {
    const nodes = harness.render({ macosAvailable, dockerEnabled: true });
    assert.deepEqual(harness.navigation(nodes).slice(0, 3).map((node) => node.key), [
      "dashboard", "performance", "applications",
    ]);
    assert.deepEqual(nodes.filter((node) => node.props.className === "navigation-group__label")
      .map((node) => node.props.children), ["컴퓨터", "파일과 공간", "도구"]);
  }
  assert.deepEqual(harness.navigations, []);
  assert.deepEqual(harness.mobileChanges, []);
});

test("Finder settings menu is present only with an explicit macOS capability", () => {
  const harness = createHarness();
  assert.equal(harness.navigation(harness.render()).some((node) => node.key === "macos-management"), false);
  assert.equal(harness.navigation(harness.render({ macosAvailable: false }))
    .some((node) => node.key === "macos-management"), false);

  const menu = harness.navigation(harness.render({ macosAvailable: true }));
  assert.equal(menu.filter((node) => node.key === "macos-management").length, 1);
  assert.equal(menu[3].key, "macos-management");
  assert.equal(menu[3].props["aria-label"], "맥 설정: Finder 표시·변경 기록");
});

test("Windows shows its own OS settings and never substitutes Finder properties", () => {
  const harness = createHarness();
  const windows = harness.navigation(harness.render({ macosAvailable: false, windowsAvailable: true, activeView: "windows-management" }));
  assert.equal(windows.some(node => node.key === "macos-management"), false);
  assert.equal(windows[3].key, "windows-management");
  assert.equal(windows[3].props["aria-label"], "Windows 설정: 탐색기·시스템 속성");
  assert.equal(windows.filter(node => node.props["aria-current"] === "page").length, 1);
  const mac = harness.navigation(harness.render({ macosAvailable: true, windowsAvailable: false }));
  assert.equal(mac.some(node => node.key === "windows-management"), false);
  assert.equal(mac[3].key, "macos-management");
});

test("Docker menu requires opt-in and stays between the AI assistant and app settings", () => {
  const harness = createHarness();
  assert.equal(harness.navigation(harness.render()).some((node) => node.key === "docker"), false);
  assert.equal(harness.navigation(harness.render({ dockerEnabled: false }))
    .some((node) => node.key === "docker"), false);
  const enabled = harness.navigation(harness.render({ dockerEnabled: true }));
  assert.equal(enabled.filter((node) => node.key === "docker").length, 1);
  assert.deepEqual(enabled.slice(-3).map((node) => node.key), ["assistant", "docker", "settings"]);
  assert.deepEqual(harness.navigations, [], "Enabling menu visibility alone must not navigate");
});

for (const [activeView, parent] of [
  ["cleanup", "overview"],
  ["duplicates", "overview"],
  ["large-files", "overview"],
  ["documents", "files"],
] as const) {
  test(`${activeView} marks exactly one global parent menu as the current page`, () => {
    const harness = createHarness();
    const navigation = harness.navigation(harness.render({ activeView, macosAvailable: true, dockerEnabled: true }));
    const current = navigation.filter((node) => node.props["aria-current"] === "page");
    assert.equal(current.length, 1);
    assert.equal(current[0].key, parent);
    assert.ok(String(current[0].props.className).split(" ").includes("is-current"));
    (current[0].props.onClick as () => void)();
    assert.deepEqual(harness.navigations, [parent]);
    assert.deepEqual(harness.mobileChanges, [false]);
  });
}

test("settings page has a general app-settings title rather than a scan-only title", () => {
  const harness = createHarness();
  const nodes = harness.render({ activeView: "settings" });
  assert.deepEqual(nodes.filter((node) => node.type === "h1").map((node) => node.props.children), ["앱 설정"]);
  const description = nodes.find((node) => node.props.className === "utility-header__description");
  assert.equal(description?.props.children, "필요한 설정을 분류별로 확인합니다.");
  const current = harness.navigation(nodes).filter((node) => node.props["aria-current"] === "page");
  assert.equal(current.length, 1);
  assert.equal(current[0].key, "settings");
});
