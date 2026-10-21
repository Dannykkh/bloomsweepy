import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import test from "node:test";
import { isValidElement, type ReactElement } from "react";
import ts from "typescript";
import { cleanupReviewAcknowledgement } from "../src/lib/cleanupTree.ts";
import type { CleanupCandidate, CleanupScanReport, CleanupTrashRequest, TrashOperationResult } from "../src/types.ts";

const require = createRequire(import.meta.url);
const compiled = ts.transpileModule(readFileSync(new URL("../src/views/CleanupView.tsx", import.meta.url), "utf8"), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX, target: ts.ScriptTarget.ES2022 },
}).outputText;
type Element = ReactElement<Record<string, unknown>>;
const SafetyDialog = () => null;
const EmptyComponent = () => null;

function elements(node: unknown): Element[] {
  if (Array.isArray(node)) return node.flatMap(elements);
  if (!isValidElement<Record<string, unknown>>(node)) return [];
  // CleanupMetric is a pure local component. All imported components below are
  // inert markers; none of their native bridges or browser effects are loaded.
  const children = typeof node.type === "function"
    ? (node.type as (props: Record<string, unknown>) => unknown)(node.props)
    : node.props.children;
  return [node, ...elements(children)];
}
function text(node: unknown): string {
  if (Array.isArray(node)) return node.map(text).join("");
  if (isValidElement<Record<string, unknown>>(node)) return text(node.props.children);
  return typeof node === "string" || typeof node === "number" ? String(node) : "";
}
function button(nodes: Element[], label: string) {
  const found = nodes.find(node => node.type === "button" && (node.props["aria-label"] === label || text(node) === label));
  assert.ok(found, `Expected button: ${label}`);
  return found;
}
function click(node: Element) {
  assert.notEqual(node.props.disabled, true, `Disabled button: ${text(node)}`);
  (node.props.onClick as () => void)();
}
function checkbox(nodes: Element[], name: string) {
  const found = nodes.find(node => node.type === "input" && node.props["aria-label"] === `${name} 휴지통 이동 대상으로 선택`);
  assert.ok(found, `Expected checkbox: ${name}`);
  return found;
}
function select(nodes: Element[], name: string) {
  const input = checkbox(nodes, name);
  assert.notEqual(input.props.disabled, true);
  (input.props.onChange as (event: unknown) => void)({ currentTarget: { checked: true } });
}
function dialog(nodes: Element[]) {
  const found = nodes.find(node => node.type === SafetyDialog);
  assert.ok(found, "Expected existing SafetyActionDialog composition");
  return found;
}
const candidate = (name: string, kind: CleanupCandidate["kind"], logicalBytes: number, confidence: CleanupCandidate["confidence"] = "likelySafe"): CleanupCandidate => ({
  name, kind, logicalBytes, confidence, path: `/SyntheticCleanup/${name}`, sourceLabel: "synthetic only", entryCount: 1,
  modifiedAtUnixMs: 1, inactiveDays: 31, evidence: ["Synthetic evidence; no filesystem access"],
});
function report(patch: Partial<CleanupScanReport> = {}): CleanupScanReport {
  const candidates = [candidate("old-cache", "cacheDirectory", 1024), candidate("old-temp", "temporaryEntry", 64), candidate("DerivedData-synthetic", "developerCache", 2048, "review")];
  return { completedAtUnixMs: 1, durationMs: 20, scannedRoots: 2, processedEntries: 3, processedBytes: 3136,
    unreadableEntries: 0, candidateBytes: 3136, candidates, limitReached: false, issues: [],
    registryResidues: { supported: false, candidates: [], issues: [] }, ...patch };
}
const fakeResult: TrashOperationResult = { operationId: "synthetic-only", requestedCount: 1, movedCount: 0, movedBytes: 0,
  cancelled: false, stoppedEarly: false, journalComplete: true, journalPath: "", items: [] };
const translate = (key: string, params: Record<string, unknown> = {}) => key.replace(/\{\{(\w+)\}\}/g, (_, name: string) => String(params[name] ?? ""));

function createHarness(initial: Record<string, unknown> = {}) {
  const slots: unknown[] = [];
  let cursor = 0;
  let revision = 0;
  let effects: Array<() => unknown> = [];
  const calls = { starts: 0, cancels: 0, moves: [] as CleanupTrashRequest[], inspections: 0 };
  const props: Record<string, unknown> = { platform: "macos", report: null, progress: null, state: "idle", error: null,
    blocked: false, actionRunning: false, actionProgress: null, actionResult: null, actionError: null,
    onStart: () => { calls.starts += 1; }, onCancel: () => { calls.cancels += 1; }, onCancelAction: () => {},
    onMoveToTrash: async (request: CleanupTrashRequest) => { calls.moves.push(structuredClone(request)); return fakeResult; }, ...initial };
  const module = { exports: {} as { CleanupView?: (props: Record<string, unknown>) => unknown } };
  new Function("require", "module", "exports", compiled)((id: string) => {
    if (id === "react") return {
      useState: (initialValue: unknown) => {
        const index = cursor++;
        if (!(index in slots)) slots[index] = typeof initialValue === "function" ? initialValue() : initialValue;
        return [slots[index], (next: unknown) => { revision += 1; slots[index] = typeof next === "function" ? next(slots[index]) : next; }];
      },
      useMemo: (compute: () => unknown) => compute(),
      useEffect: (callback: () => unknown, dependencies: unknown[]) => {
        const index = cursor++;
        const previous = slots[index] as unknown[] | undefined;
        if (!previous || dependencies.some((value, i) => !Object.is(value, previous[i]))) effects.push(callback);
        slots[index] = dependencies;
      },
    };
    if (id === "react/jsx-runtime") return require(id);
    if (id === "lucide-react") return new Proxy({}, { get: () => EmptyComponent });
    if (id === "../i18n") return { useLanguage: () => ({ t: translate, appText: (text: string) => text }) };
    if (id === "../lib/format") return { formatBytes: (value: number) => `${value}B`, formatCount: String, formatDate: String, formatDuration: (value: number) => `${value}ms` };
    if (id === "../lib/cleanupTree") return { cleanupReviewAcknowledgement };
    if (id === "../components/SafetyActionDialog") return { SafetyActionDialog: SafetyDialog };
    if (id === "../components/TrashResultPanel") return { TrashResultPanel: EmptyComponent };
    if (id === "../components/FileOpenActions") return { FileInspectionStatus: EmptyComponent, FileOpenActions: EmptyComponent,
      useFileInspectionActions: () => ({ busy: false, message: null, error: null, run: () => { calls.inspections += 1; assert.fail("OS inspection is forbidden in the unit harness"); } }) };
    assert.fail(`Unexpected runtime dependency; no IPC fallback is allowed: ${id}`);
  }, module, module.exports);
  assert.ok(module.exports.CleanupView);
  return { calls, render: (patch: Record<string, unknown> = {}) => {
    Object.assign(props, patch);
    for (let pass = 0; pass < 4; pass += 1) {
      cursor = 0; effects = [];
      const nodes = elements(module.exports.CleanupView!(props));
      const before = revision;
      for (const effect of effects) effect();
      if (revision === before) return nodes;
    }
    assert.fail("Unexpected render/effect loop");
  } };
}

test("empty system cleanup has exactly one scan CTA and no implicit work", () => {
  const harness = createHarness();
  const nodes = harness.render();
  assert.equal(nodes.filter(node => node.type === "button").length, 1);
  assert.ok(nodes.some(node => text(node).includes("아직 정리 후보를 확인하지 않았습니다")));
  assert.equal(harness.calls.starts, 0);
  click(button(nodes, "정리 후보 스캔"));
  assert.equal(harness.calls.starts, 1);
  assert.deepEqual(harness.calls.moves, []);
});

test("scanning shows only cancellation, with read-only progress instead of a second start CTA", () => {
  const harness = createHarness({ state: "scanning", progress: { processedEntries: 7, processedBytes: 64, candidatesFound: 2, processedRoots: 1, totalRoots: 3 } });
  const nodes = harness.render();
  assert.deepEqual(nodes.filter(node => node.type === "button").map(text), ["스캔 취소"]);
  assert.ok(nodes.some(node => node.props.role === "status" && text(node).includes("확인한 항목7")));
  click(button(nodes, "스캔 취소"));
  assert.equal(harness.calls.cancels, 1);
  assert.equal(harness.calls.starts, 0);
  assert.deepEqual(harness.calls.moves, []);
});

test("report filtering preserves selection summary and opening review does not execute trash", () => {
  const harness = createHarness({ report: report(), state: "success" });
  click(button(harness.render(), "캐시"));
  const filtered = harness.render();
  assert.equal(filtered.filter(node => node.type === "article").length, 1);
  select(filtered, "old-cache");
  let nodes = harness.render();
  assert.ok(nodes.some(node => node.props["aria-label"] === "휴지통 이동 선택 요약" && text(node).includes("1개 · 1024B")));
  click(button(nodes, "임시 파일"));
  nodes = harness.render();
  assert.equal(checkbox(nodes, "old-temp").props.checked, false);
  click(button(nodes, "휴지통으로 이동"));
  const review = dialog(harness.render());
  assert.equal(review.props.open, true);
  assert.equal(review.props.itemCount, 1);
  assert.equal(review.props.logicalBytes, 1024);
  assert.deepEqual(harness.calls.moves, [], "Opening the existing dialog is not execution");
  assert.equal(harness.calls.inspections, 0);
});

test("developer-cache review keeps its rebuild warning and delegates the existing acknowledgement payload only to a fake callback", async () => {
  const harness = createHarness({ report: report(), state: "success" });
  click(button(harness.render(), "개발 캐시"));
  select(harness.render(), "DerivedData-synthetic");
  click(button(harness.render(), "휴지통으로 이동"));
  const review = dialog(harness.render());
  assert.equal(review.props.reviewCount, 1);
  assert.match(String(review.props.reviewAcknowledgementLabel), /개발 캐시는 재빌드·재다운로드가 필요/);
  assert.deepEqual(harness.calls.moves, []);
  (review.props.onConfirm as (acknowledged: boolean) => void)(true);
  await new Promise<void>(resolve => setImmediate(resolve));
  assert.deepEqual(harness.calls.moves, [{ paths: ["/SyntheticCleanup/DerivedData-synthetic"], allowReviewCandidates: true }]);
  assert.equal(dialog(harness.render()).props.open, false);
  assert.equal(harness.calls.inspections, 0);
});

test("blocked or busy cleanup disables selection and execution without changing the selected targets", () => {
  for (const patch of [{ blocked: true }, { actionRunning: true }]) {
    const harness = createHarness({ report: report(), state: "success" });
    select(harness.render(), "old-temp");
    const nodes = harness.render(patch);
    assert.equal(checkbox(nodes, "old-temp").props.disabled, true);
    assert.equal(checkbox(nodes, "old-temp").props.checked, true);
    assert.equal(checkbox(nodes, "old-cache").props.disabled, true);
    assert.equal(button(nodes, "휴지통으로 이동").props.disabled, true);
    assert.equal(dialog(nodes).props.open, false);
    assert.deepEqual(harness.calls.moves, []);
  }
});

test("a newly completed report clears previous selection and closes its stale review dialog", () => {
  const harness = createHarness({ report: report(), state: "success" });
  select(harness.render(), "old-temp");
  click(button(harness.render(), "휴지통으로 이동"));
  assert.equal(dialog(harness.render()).props.open, true);
  const nodes = harness.render({ report: report({ completedAtUnixMs: 2 }) });
  assert.equal(checkbox(nodes, "old-temp").props.checked, false);
  assert.equal(nodes.some(node => node.props["aria-label"] === "휴지통 이동 선택 요약"), false);
  assert.equal(dialog(nodes).props.open, false);
  assert.deepEqual(harness.calls.moves, []);
});

test("error, zero and partial reports stay explicit; limits and unreadable issues precede selection summaries", () => {
  const failed = createHarness({ state: "error", error: "Synthetic read denied" });
  assert.ok(failed.render().some(node => text(node).includes("Synthetic read denied")));
  const empty = createHarness({ report: report({ candidates: [], candidateBytes: 0 }), state: "success" });
  assert.ok(empty.render().some(node => text(node) === "선택한 종류의 정리 후보가 없습니다."));
  assert.equal(empty.render().some(node => node.props.role === "alert"), false);
  for (const patch of [{ limitReached: true }, { unreadableEntries: 1 }, { issues: [{ path: "/SyntheticCleanup/unreadable", message: "Synthetic denial" }] }]) {
    const harness = createHarness({ report: report(patch), state: "success" });
    const nodes = harness.render();
    const warning = nodes.findIndex(node => node.props.role === "alert");
    const summary = nodes.findIndex(node => node.props["aria-label"] === "정리 후보 요약");
    assert.ok(warning >= 0 && warning < summary, "Incomplete results must be disclosed before selecting candidates");
    assert.match(text(nodes[warning]), "limitReached" in patch ? /스캔 안전 한도에 도달/ : /부분 결과입니다/);
    assert.deepEqual(harness.calls.moves, []);
  }
});

test("Windows has no Mac developer-cache filter and registry findings remain read-only", () => {
  const value = report({ candidates: [candidate("old-temp", "temporaryEntry", 64)], candidateBytes: 64,
    registryResidues: { supported: true, issues: [], candidates: [{ displayName: "Synthetic removed app", registryPath: "HKCU\\SyntheticCleanup\\RemovedApp", registryScope: "user", evidence: ["Install path absent", "Uninstaller absent"] }] } });
  const nodes = createHarness({ platform: "windows", report: value, state: "success" }).render();
  assert.equal(nodes.some(node => node.type === "button" && text(node) === "개발 캐시"), false);
  const registry = nodes.find(node => node.props.className === "registry-residue-panel");
  assert.ok(registry);
  assert.match(text(registry), /Synthetic removed app/);
  assert.equal(elements(registry).some(node => node.type === "input" || node.type === "button" || node.props.onChange || node.props.onClick), false);
  assert.ok(nodes.some(node => text(node).includes("Windows 설치 정보는 바꾸지 않습니다.")));
});
