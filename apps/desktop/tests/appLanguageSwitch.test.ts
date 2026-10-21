import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import test from "node:test";
import { isValidElement, type ReactElement } from "react";
import ts from "typescript";

const require = createRequire(import.meta.url);
const compiled = ts.transpileModule(readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8"), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX, target: ts.ScriptTarget.ES2022 },
}).outputText;
const catalogSource = ts.createSourceFile("i18n.tsx", readFileSync(new URL("../src/i18n/index.tsx", import.meta.url), "utf8"), ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
let catalog: Record<string, string> = {};
function readCatalog(node: ts.Node) {
  if (ts.isVariableDeclaration(node) && node.name.getText(catalogSource) === "englishMessages" && node.initializer) {
    let value = node.initializer;
    while (ts.isAsExpression(value) || ts.isSatisfiesExpression(value)) value = value.expression;
    assert.ok(ts.isObjectLiteralExpression(value));
    catalog = Object.fromEntries(value.properties.map(property => {
      assert.ok(ts.isPropertyAssignment(property) && ts.isStringLiteral(property.name) && ts.isStringLiteral(property.initializer));
      return [property.name.text, property.initializer.text];
    }));
  }
  ts.forEachChild(node, readCatalog);
}
readCatalog(catalogSource);

type Element = ReactElement<Record<string, unknown>>;
function elements(node: unknown): Element[] {
  if (Array.isArray(node)) return node.flatMap(elements);
  return isValidElement<Record<string, unknown>>(node) ? [node, ...elements(node.props.children)] : [];
}
const settle = () => new Promise<void>(resolve => setImmediate(resolve));

// Execute the actual App composition, state and registered callbacks. Imported
// views are inert markers and every native operation is an explicit read-only fake.
function createHarness(systemRead: () => Promise<unknown> = async () => null) {
  const slots: unknown[] = [];
  let cursor = 0;
  let revision = 0;
  let language: "ko" | "en" = "ko";
  const doc = { documentElement: { lang: language } };
  let pending: (() => void)[] = [];
  const cleanups = new Set<() => void>();
  const calls = new Map<string, number>();
  const listeners = new Map<string, (event: unknown) => void>();
  const markers = new Map<string, () => null>();
  function marker(name: string) {
    if (!markers.has(name)) markers.set(name, () => null);
    return markers.get(name)!;
  }
  const bridge = new Proxy({}, { get: (_target, key: string) => (...args: unknown[]) => {
    calls.set(key, (calls.get(key) ?? 0) + 1);
    if (key.startsWith("listenTo")) {
      listeners.set(key, args[0] as (event: unknown) => void);
      return Promise.resolve(() => listeners.delete(key));
    }
    switch (key) {
      case "getSystemOverview": return systemRead();
      case "getActionHistory": return Promise.resolve({ entries: [], issues: [] });
      case "getRecentFileCatalogEntries": return Promise.resolve({ entries: [], issues: [] });
      case "getActionRecoveryStatus": return Promise.resolve({ incompleteOperations: [], issues: [] });
      case "getDocumentIndexStatus":
      case "getFileCatalogStatus": return Promise.resolve(null);
      case "getDockerManagementStatus": return Promise.resolve({ enabled: false });
      case "getControlStatus": return Promise.resolve({ revision: 0, bridgeAvailable: false,
        connectedClients: 0, activeOperation: null, lastOperation: null, pendingReview: null,
        searchAccess: { files: false, documents: false }, scanAccess: { enabled: false, root: null },
        cleanupAccess: { enabled: false }, inspectionAllowed: false });
      default: assert.fail(`Unexpected native action: ${key}`);
    }
  } });
  const module = { exports: {} as { default?: () => unknown } };
  new Function("require", "module", "exports", "window", "document", compiled)((id: string) => {
    if (id === "react/jsx-runtime") return require(id);
    if (id === "react") return {
      useState: (initial: unknown) => {
        const index = cursor++;
        if (!(index in slots)) slots[index] = typeof initial === "function" ? initial() : initial;
        return [slots[index], (next: unknown) => {
          const value = typeof next === "function" ? next(slots[index]) : next;
          if (!Object.is(value, slots[index])) { slots[index] = value; revision += 1; }
        }];
      },
      useRef: (initial: unknown) => {
        const index = cursor++;
        if (!(index in slots)) slots[index] = { current: initial };
        return slots[index];
      },
      useMemo: (compute: () => unknown) => compute(),
      useCallback: (callback: unknown) => callback,
      useEffect: (callback: () => void | (() => void), dependencies: unknown[]) => {
        const index = cursor++;
        const previous = slots[index] as unknown[] | undefined;
        if (!previous || dependencies.some((value, i) => !Object.is(value, previous[i]))) {
          pending.push(() => { const cleanup = callback(); if (cleanup) cleanups.add(cleanup); });
        }
        slots[index] = dependencies;
      },
    };
    if (id === "./i18n") return { useLanguage: () => {
      // Match the real provider: each render returns a language-bound function.
      // A dynamic mock would conceal stale closures in mount-only effects.
      const current = language;
      const appText = (text: string) => current === "ko" ? text : catalog[text] ?? text;
      return { language: current, appText, t: (key: string, values: Record<string, unknown> = {}) =>
        appText(key).replace(/\{\{(\w+)\}\}/g, (match, name: string) => Object.hasOwn(values, name) ? String(values[name]) : match) };
    } };
    if (id === "./lib/bridge") return bridge;
    if (id === "./types") return { DEFAULT_SCAN_CONFIG: { maxIssues: 50 } };
    if (id === "./lib/volumePath") return { findVolumeForPath: () => null };
    if (id === "./lib/selectedScanScope") return { bindSelectedScanScope: () => assert.fail("Unexpected scan") };
    if (id.endsWith(".css")) return {};
    if (id.startsWith("./components/") || id.startsWith("./views/") || id === "lucide-react") {
      return new Proxy({}, { get: (_target, key: string) => marker(key) });
    }
    assert.fail(`Unexpected runtime dependency: ${id}`);
  }, module, module.exports, { setTimeout, clearTimeout }, doc);
  assert.ok(module.exports.default);
  function render() {
    for (let pass = 0; pass < 8; pass += 1) {
      cursor = 0; pending = [];
      const nodes = elements(module.exports.default!());
      const before = revision;
      for (const effect of pending) effect();
      if (before === revision) return nodes;
    }
    assert.fail("Unexpected App render loop");
  }
  return { calls, listeners, render,
    changeLanguage: (next: "ko" | "en") => { language = next; doc.documentElement.lang = next; return render(); },
    dashboard: (nodes: Element[]) => nodes.find(node => node.type === marker("DashboardView"))!,
    unmount: () => { for (const cleanup of cleanups) cleanup(); },
  };
}

for (const delayed of [false, true]) {
  test(`startup dashboard errors follow the selected language without repeating native setup (${delayed ? "pending" : "settled"} read)`, async () => {
    let reject!: (reason: unknown) => void;
    const read = new Promise<never>((_resolve, fail) => { reject = fail; });
    const harness = createHarness(() => read);
    harness.render();
    if (delayed) harness.changeLanguage("en");
    const path = "/Users/사용자/개인자료.txt";
    reject(new Error(path)); await settle();
    const before = harness.render();
    assert.equal(harness.dashboard(before).props.error, `${delayed ? "Drives" : "드라이브"}: ${path}`);
    const counts = [...harness.calls];
    assert.equal(harness.dashboard(harness.changeLanguage("en")).props.error, `Drives: ${path}`);
    assert.equal(harness.dashboard(harness.changeLanguage("ko")).props.error, `드라이브: ${path}`);
    assert.deepEqual([...harness.calls], counts, "Language changes must not restart IPC reads or listeners");
    harness.unmount();
  });
}

test("existing background completion and error announcements retranslate without another scan", async () => {
  const harness = createHarness();
  harness.render(); await settle(); harness.render();
  const emit = harness.listeners.get("listenToControlScanCompleted");
  assert.ok(emit);
  emit({ revision: 1, operationId: "synthetic-cancelled", kind: "storageScan", state: "cancelled", scanGeneration: null, message: "" });
  const cancelled = (nodes: Element[]) => nodes.find(node => node.props.className === "sr-only background-scan-announcement")?.props.children;
  const error = (nodes: Element[]) => nodes.find(node => node.props.className === "sr-only background-scan-error-announcement")?.props.children;
  assert.equal(cancelled(harness.render()), "파일 검사를 취소했습니다.");
  const counts = [...harness.calls];
  assert.equal(cancelled(harness.changeLanguage("en")), catalog["파일 검사를 취소했습니다."]);
  harness.changeLanguage("ko");
  const detail = "확인할 정리 계획이 없거나 확인 시간이 지났습니다.";
  emit({ revision: 2, operationId: "synthetic-failed", kind: "storageScan", state: "failed", scanGeneration: null, message: detail });
  assert.equal(error(harness.render()), `파일 검사를 완료하지 못했습니다. ${detail}`);
  assert.equal(error(harness.changeLanguage("en")), catalog["파일 검사를 완료하지 못했습니다. {{detail}}"].replace("{{detail}}", catalog[detail]));
  assert.deepEqual([...harness.calls], counts);
  harness.unmount();
});

test("an unknown error created in English retains its canonical message for a later Korean switch", async () => {
  const harness = createHarness(async () => { throw null; });
  harness.changeLanguage("en"); await settle();
  assert.equal(harness.dashboard(harness.render()).props.error, "Drives: An unknown error occurred");
  const counts = [...harness.calls];
  assert.equal(harness.dashboard(harness.changeLanguage("ko")).props.error, "드라이브: 알 수 없는 오류가 발생했습니다");
  assert.deepEqual([...harness.calls], counts);
  harness.unmount();
});
