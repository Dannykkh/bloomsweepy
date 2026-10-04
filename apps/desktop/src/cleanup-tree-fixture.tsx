// Synthetic review workspace. No filesystem, native deletion, or AI requests run.
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { useState } from "react";
import { createRoot } from "react-dom/client";
import { AppShell } from "./components/AppShell";
import { CleanupTreePanel } from "./components/CleanupTreePanel";
import { LanguageProvider } from "./i18n";
import { LANGUAGE_STORAGE_KEY } from "./i18n/preference";
import type { TrashOperationResult } from "./types";
import type { CleanupTreeBranch, CleanupTreeNode, CleanupTreeView } from "./types/cleanupTree";
import "./App.css";

const params = new URLSearchParams(window.location.search);
window.localStorage.setItem(LANGUAGE_STORAGE_KEY, params.get("language") ?? "ko");
mockWindows("main");
mockIPC((command) => {
  if (command === "set_application_language") return null;
  throw new Error(`Synthetic fixture rejects native command: ${command}`);
});

const fixtureRoot = "/Synthetic/Downloads";
const node = (id: string, parentId: string | null, name: string, bytes: number | null,
  isDirectory = false, eligible = true): CleanupTreeNode => ({
  id, parentId, name, path: `${fixtureRoot}/${name}`, isDirectory,
  logicalBytes: bytes, fileCount: isDirectory ? null : 1, directoryCount: isDirectory ? null : 0,
  selectionState: "unchecked", eligible, blockedReason: eligible ? null : "Synthetic protected item — never selected",
  childrenLoaded: false,
});
const originals = [
  node("project", null, "Old project", 600, true),
  node("docs", "project", "Documents", 100, true),
  node("draft", "docs", "draft.txt", 60),
  node("keep", "docs", "KEEP THIS DOCUMENT.txt", 40),
  node("archive", "project", "archive.zip", 500),
  node("protected", null, "Protected app.app", 900, true, false),
  node("unmeasured", null, "Unmeasured folder", null, true),
  node("long", null, "정리 후보 long folder name ".repeat(10) + ".txt", 30),
  ...Array.from({ length: params.has("wide") ? 70 : 0 }, (_, index) => node(`wide-${index}`, null, `Paged file ${index}.txt`, index)),
];
// Paths here are synthetic presentation only; selection is by server-like IDs.
originals[1].path = `${fixtureRoot}/Old project/Documents`;
originals[2].path = `${originals[1].path}/draft.txt`;
originals[3].path = `${originals[1].path}/KEEP THIS DOCUMENT.txt`;
originals[4].path = `${fixtureRoot}/Old project/archive.zip`;

function branch(parentId: string | null, offset = 0): CleanupTreeBranch {
  const children = originals.filter((entry) => entry.parentId === parentId);
  return { parentId, offset, limit: 50, totalEntries: children.length, hasMore: offset + 50 < children.length,
    truncated: false, unreadableEntries: 0, complete: true, childIds: children.slice(offset, offset + 50).map((entry) => entry.id) };
}
function initial(): CleanupTreeView {
  const roots = branch(null);
  const capturedAtUnixMs = Date.now();
  return { treeId: `synthetic-${Date.now()}`, selectionRevision: 0, source: "directory", rootName: "Synthetic Downloads",
    rootPath: fixtureRoot, capturedAtUnixMs, expiresAtUnixMs: params.has("expired") ? capturedAtUnixMs - 1 : capturedAtUnixMs + 300_000,
    nodes: originals.filter((entry) => roots.childIds.includes(entry.id)).map((entry) => ({ ...entry,
      selectionState: params.has("mixed-only") && entry.id === "project" ? "mixed" as const : entry.selectionState })), branches: [roots],
    selection: { selectedNodeCount: 0, targetCount: 0, knownLogicalBytes: 0, unknownTargets: params.has("mixed-only") ? 1 : 0, partial: params.has("mixed-only") }, plan: null };
}

function Fixture() {
  const [tree, setTree] = useState<CleanupTreeView>(initial);
  const [rules, setRules] = useState<Map<string, boolean>>(new Map());
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<TrashOperationResult | null>(null);
  const [executions, setExecutions] = useState(0);
  const [survivors, setSurvivors] = useState(originals.map((entry) => entry.id));
  const [frontierText, setFrontierText] = useState("none");
  const [mobile, setMobile] = useState(false);
  const [activity, setActivity] = useState("Synthetic adapter ready");
  const delay = () => new Promise((resolve) => window.setTimeout(resolve, params.has("slow") ? 1500 : 50));

  function selected(entry: CleanupTreeNode, nextRules = rules): boolean {
    if (!entry.eligible) return false;
    let current: CleanupTreeNode | undefined = entry;
    while (current) {
      if (nextRules.has(current.id)) return Boolean(nextRules.get(current.id));
      current = originals.find((candidate) => candidate.id === current!.parentId);
    }
    return false;
  }
  function descendant(candidate: CleanupTreeNode, ancestor: CleanupTreeNode): boolean {
    let parent = candidate.parentId;
    while (parent) {
      if (parent === ancestor.id) return true;
      parent = originals.find((entry) => entry.id === parent)?.parentId ?? null;
    }
    return false;
  }
  function state(entry: CleanupTreeNode, nextRules = rules): CleanupTreeNode["selectionState"] {
    if (!entry.eligible) return "unchecked";
    const subtree = originals.filter((candidate) => descendant(candidate, entry));
    const all = selected(entry, nextRules) && subtree.every((candidate) => selected(candidate, nextRules));
    return all ? "checked" : subtree.some((candidate) => selected(candidate, nextRules)) ? "mixed" : "unchecked";
  }
  function apply(view: CleanupTreeView, nextRules = rules): CleanupTreeView {
    const updated = { ...view, nodes: view.nodes.map((entry) => ({ ...entry, selectionState: state(entry, nextRules) })) };
    const frontier = updated.nodes.filter((entry) => entry.selectionState === "checked"
      && !updated.nodes.some((parent) => parent.selectionState === "checked" && descendant(entry, parent)));
    setFrontierText(frontier.map((entry) => entry.name).join(" | ") || "none");
    return { ...updated, selection: {
      selectedNodeCount: updated.nodes.filter((entry) => entry.selectionState !== "unchecked").length,
      targetCount: frontier.length,
      knownLogicalBytes: frontier.reduce((total, entry) => total + (entry.logicalBytes ?? 0), 0),
      unknownTargets: frontier.filter((entry) => entry.logicalBytes === null).length,
      partial: updated.nodes.some((entry) => entry.selectionState === "mixed"),
    } };
  }
  async function load(parentId: string | null, offset: number) {
    setBusy(true);
    try {
      await delay();
      if (params.has("error")) throw new Error("Synthetic child lookup failure — retry remains available");
      const next = branch(parentId, offset);
      if (params.has("partial") && parentId === "docs") { next.truncated = true; next.complete = false; next.unreadableEntries = 1; }
      const incoming = originals.filter((entry) => next.childIds.includes(entry.id));
      const nodes = [...tree.nodes.filter((entry) => !incoming.some((item) => item.id === entry.id)), ...incoming.map((entry) => ({ ...entry }))]
        .map((entry) => entry.id === parentId ? { ...entry, childrenLoaded: true } : entry);
      setTree(apply({ ...tree, nodes, branches: [...tree.branches.filter((entry) => entry.parentId !== parentId), next] }));
    } finally { setBusy(false); }
  }
  async function change(includeIds: string[], excludeIds: string[]) {
    const nextRules = new Map(rules);
    for (const id of includeIds) {
      const target = originals.find((entry) => entry.id === id)!;
      for (const entry of originals) if (descendant(entry, target)) nextRules.delete(entry.id);
      if (target.eligible) nextRules.set(id, true);
    }
    for (const id of excludeIds) {
      const target = originals.find((entry) => entry.id === id)!;
      for (const entry of originals) if (descendant(entry, target)) nextRules.delete(entry.id);
      nextRules.set(id, false);
    }
    setRules(nextRules);
    setTree(apply({ ...tree, selectionRevision: tree.selectionRevision + 1, plan: null }, nextRules));
  }
  async function prepare() {
    const entries = tree.nodes.filter((entry) => entry.selectionState === "checked"
      && !tree.nodes.some((parent) => parent.selectionState === "checked" && descendant(entry, parent)))
      .map((entry) => entry.logicalBytes === null ? { ...entry, logicalBytes: 0, fileCount: 0, directoryCount: 0 } : entry);
    if (!entries.length) throw new Error("Synthetic selection is empty; load the selected branch before review");
    if (entries.some((entry) => entry.isDirectory && tree.branches.some((page) => page.parentId === entry.id && !page.complete))) throw new Error("Synthetic incomplete folder review is blocked");
    setTree({ ...tree, plan: { id: `synthetic-plan-${Date.now()}`, selectionRevision: tree.selectionRevision,
      expiresAtUnixMs: params.has("expired-plan") ? Date.now() - 1 : Date.now() + 300_000,
      entries, logicalBytes: entries.reduce((sum, entry) => sum + (entry.logicalBytes ?? 0), 0), requiresNestedAck: entries.some((entry) => entry.isDirectory) } });
  }
  async function confirm(acknowledged: boolean) {
    const plan = tree.plan;
    if (!plan || plan.selectionRevision !== tree.selectionRevision || plan.expiresAtUnixMs <= Date.now() || tree.expiresAtUnixMs <= Date.now()
      || (plan.requiresNestedAck && !acknowledged)) throw new Error("Synthetic invalid or unconfirmed plan");
    const moved = originals.filter((entry) => plan.entries.some((target) => target.id === entry.id || descendant(entry, target)));
    setExecutions((value) => value + 1);
    setSurvivors(originals.filter((entry) => !moved.some((item) => item.id === entry.id)).map((entry) => entry.id));
    setResult({ operationId: "synthetic-only", requestedCount: plan.entries.length, movedCount: plan.entries.length,
      movedBytes: plan.logicalBytes, cancelled: false, stoppedEarly: false, journalComplete: true, journalPath: "/Synthetic/mock-journal",
      items: plan.entries.map((entry) => ({ path: entry.path, logicalBytes: entry.logicalBytes ?? 0, status: "moved", message: null })) });
    setTree({ ...tree, plan: null });
  }

  return <AppShell activeView="cleanup" root={fixtureRoot} report={null} volume={null} mobileNavigationOpen={mobile}
    selectionBlocked={false} dockerEnabled={false} onMobileNavigationChange={setMobile} onNavigate={() => {}} onPickFolder={() => {}}>
    <section className="panel">
      <h2>Synthetic cleanup tree QA</h2>
      <p>실제 OS 호출: 0 · 실제 파일 검사/이동: 0 · AI 전송: 0</p>
      <output data-testid="fixture-executions">Mock executions: {executions}</output>
      <p data-testid="fixture-frontier">Frontier: {frontierText}</p>
      <p data-testid="fixture-kept">KEEP document survives: {String(survivors.includes("keep"))}</p>
      <p role="status">{activity}</p>
      <p>Query modes: ?partial, ?expired, ?expired-plan, ?error, ?slow, ?wide, ?language=en</p>
    </section>
    <CleanupTreePanel tree={tree} busy={busy} error={null} result={result} onLoadChildren={load}
      onSelectionChange={change} onPrepare={prepare} onConfirm={confirm}
      onDismissPlan={() => setTree({ ...tree, plan: null })}
      onRefresh={() => { setRules(new Map()); setResult(null); setTree(initial()); setFrontierText("none"); }}
      onOpen={async (path) => setActivity(`Synthetic open request: ${path} · actual OS calls: 0`)}
      onReveal={async (path) => setActivity(`Synthetic reveal request: ${path} · actual OS calls: 0`)} />
  </AppShell>;
}

createRoot(document.getElementById("root")!).render(<LanguageProvider><Fixture /></LanguageProvider>);
