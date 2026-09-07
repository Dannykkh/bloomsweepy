import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { useState } from "react";
import { createRoot } from "react-dom/client";
import "./App.css";
import { AppShell } from "./components/AppShell";
import { FileTable } from "./components/FileTable";
import { StorageTreemapPanel } from "./components/StorageTreemapPanel";
import { CleanupView } from "./views/CleanupView";
import { LanguageProvider } from "./i18n";
import { LANGUAGE_STORAGE_KEY } from "./i18n/preference";
import type { CleanupScanReport, DirectoryScanReport, FileEntry } from "./types";

const params = new URLSearchParams(window.location.search);
window.localStorage.setItem(LANGUAGE_STORAGE_KEY, params.get("language") ?? "ko");
mockWindows("main");
let calls = 0;
let notify = (_message: string) => {};
mockIPC(async (command, payload) => {
  if (command === "set_application_language") return;
  const args = payload as { path?: string; paths?: string[] } | undefined;
  if (command === "inspect_local_path" || command === "reveal_local_path") {
    calls += 1;
    const path = args?.path ?? args?.paths?.[0] ?? "unknown";
    notify(`mock dispatches: ${calls} · ${command} · ${path} · actual OS calls: 0`);
    await new Promise((resolve) => window.setTimeout(resolve, params.has("slow") ? 2000 : 120));
    if (params.has("failure")) throw new Error("Fixture: item missing or inaccessible");
    return command === "inspect_local_path" ? path.endsWith(".command") ? "revealed" : "opened" : undefined;
  }
  throw new Error(`Unexpected fixture command: ${command}`);
});

const root = "/Fixture/Downloads";
const timestamp = 1_783_000_000_000;
const files: FileEntry[] = [
  { name: "report.pdf", path: `${root}/report.pdf`, logicalBytes: 8_000_000, modifiedAtUnixMs: timestamp },
  { name: "run.command", path: `${root}/run.command`, logicalBytes: 1024, modifiedAtUnixMs: timestamp },
];
const cleanup: CleanupScanReport = {
  completedAtUnixMs: timestamp, durationMs: 20, scannedRoots: 1, processedEntries: 3,
  processedBytes: 10_000, unreadableEntries: 0, candidateBytes: 10_000, limitReached: false, issues: [],
  registryResidues: { supported: false, candidates: [], issues: [] },
  candidates: [
    { kind: "cacheDirectory", confidence: "likelySafe", name: "BroomSweepy downloads", path: `${root}/BroomSweepy downloads`,
      sourceLabel: "Fixture", logicalBytes: 10_000, entryCount: 2, modifiedAtUnixMs: timestamp,
      inactiveDays: 100, evidence: ["Synthetic folder · no filesystem access"] },
  ],
};
const treemap: DirectoryScanReport = {
  generation: 1, root, name: "Downloads", parent: "/Fixture", completedAtUnixMs: timestamp,
  durationMs: 20, totalLogicalBytes: 8_010_000, totalFiles: 3, totalDirectories: 2,
  directChildCount: 2, childrenTruncated: false, trackingLimitReached: false,
  omittedChildCount: 0, omittedLogicalBytes: 0, emptyDirectoryCount: 1,
  emptyDirectoriesTruncated: false, unreadableEntries: 0, issues: [],
  emptyDirectories: [{ name: "Empty folder", path: `${root}/Empty folder`, modifiedAtUnixMs: timestamp }],
  children: [
    { ...files[0], isDirectory: false, fileCount: 1, directoryCount: 0 },
    { name: "Documents", path: `${root}/Documents`, isDirectory: true, fileCount: 2,
      directoryCount: 1, logicalBytes: 10_000, modifiedAtUnixMs: timestamp },
  ],
};

function Fixture() {
  const [activity, setActivity] = useState("mock dispatches: 0 · actual OS calls: 0");
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [mode, setMode] = useState(params.get("view") ?? "files");
  const [mobile, setMobile] = useState(false);
  notify = setActivity;
  return <AppShell activeView="cleanup" root={root} report={null} volume={null}
    mobileNavigationOpen={mobile} selectionBlocked={false} dockerEnabled={false}
    onMobileNavigationChange={setMobile} onNavigate={() => {}} onPickFolder={() => {}}>
    <section className="panel">
      <h2>파일·폴더 열기 테스트</h2>
      <p role="status" aria-label="Fixture dispatches">{activity}</p>
      <p>실제 OS 호출 없음 · 파일 열기 없음 · 삭제 없음</p>
      <div className="segmented-control" aria-label="Fixture view">
        {[["files", "파일 목록"], ["cleanup", "정리 후보"], ["treemap", "용량 지도"]].map(([value, label]) =>
          <button type="button" key={value} aria-pressed={mode === value} onClick={() => setMode(value)}>{label}</button>)}
      </div>
      <output aria-label="Fixture selection">selected: {selected.size}</output>
    </section>
    {mode === "files" ? <FileTable files={files} emptyMessage="No fixture files" selectedPaths={selected}
      onSelectionChange={(file, checked) => setSelected((previous) => {
        const next = new Set(previous);
        if (checked) next.add(file.path); else next.delete(file.path);
        return next;
      })} /> : mode === "cleanup" ? <CleanupView platform="macos" report={cleanup} progress={null}
        state="success" error={null} blocked={false} actionRunning={false} actionProgress={null}
        actionResult={null} actionError={null} onStart={() => {}} onCancel={() => {}}
        onMoveToTrash={async () => { throw new Error("Fixture does not support deletion"); }} onCancelAction={() => {}} />
      : <StorageTreemapPanel root={root} report={treemap} progress={null} state="success" error={null}
        breadcrumbs={[{ name: "Downloads", path: root }]} blocked={false} onPickFolder={() => {}}
        onStart={(path) => setActivity(`drilldown:${path} · mock dispatches: ${calls} · actual OS calls: 0`)}
        onCancel={() => {}} />}
  </AppShell>;
}

createRoot(document.getElementById("root")!).render(<LanguageProvider><Fixture /></LanguageProvider>);
