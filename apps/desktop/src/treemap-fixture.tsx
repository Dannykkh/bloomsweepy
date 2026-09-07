import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { useState } from "react";
import { createRoot } from "react-dom/client";
import "./App.css";
import { AppShell } from "./components/AppShell";
import { StorageSectionNav } from "./components/StorageSectionNav";
import { StorageTreemapPanel } from "./components/StorageTreemapPanel";
import { LanguageProvider } from "./i18n";
import { LANGUAGE_STORAGE_KEY } from "./i18n/preference";
import type { DirectoryBreadcrumb, DirectoryNode, DirectoryScanReport, FolderReviewPlan } from "./types";

const params = new URLSearchParams(window.location.search);
window.localStorage.setItem(LANGUAGE_STORAGE_KEY, params.get("language") ?? "ko");
mockWindows("main");
mockIPC(() => undefined);
const fixtureRoot = "/Fixture/Treemap";
let generation = 0;
function fixtureReport(root: string): DirectoryScanReport {
  const names = root === fixtureRoot ? ["Archives", "Documents", "sample-video.mp4", "report.pdf", ...Array.from({ length: 16 }, (_, i) => `sample-${i + 1}.dat`)] : ["inside-file.txt"];
  const children: DirectoryNode[] = names.map((name, index) => ({
    name, path: `${root}/${name}`, logicalBytes: (index === 0 ? 600 : index === 1 ? 300 : index === 2 ? 200 : 40 - index) * 1024 ** 2,
    isDirectory: root === fixtureRoot && index < 2, fileCount: index < 2 ? 12 : 1,
    directoryCount: index < 2 ? 2 : 0, modifiedAtUnixMs: Date.now(),
  }));
  return {
    generation: ++generation, root, name: root.split("/").pop()!, parent: "/Fixture",
    completedAtUnixMs: Date.now(), durationMs: 18, totalLogicalBytes: children.reduce((sum, node) => sum + node.logicalBytes, 0),
    totalFiles: 42, totalDirectories: 2, directChildCount: children.length, childrenTruncated: false,
    trackingLimitReached: false, omittedChildCount: 0, omittedLogicalBytes: 0,
    emptyDirectoryCount: 0, emptyDirectoriesTruncated: false, unreadableEntries: 0,
    children, emptyDirectories: [], issues: [],
  };
}

function Fixture() {
  const [report, setReport] = useState(() => fixtureReport(fixtureRoot));
  const [breadcrumbs, setBreadcrumbs] = useState<DirectoryBreadcrumb[]>([{ name: "Treemap", path: fixtureRoot }]);
  const [activity, setActivity] = useState("테스트 화면 · 실제 파일 작업 없음");
  const [plan, setPlan] = useState<FolderReviewPlan | null>(null);
  const [mobileOpen, setMobileOpen] = useState(false);
  async function moveFixture(path: string) {
    if (params.has("failure")) throw new Error("Fixture: item changed after scan");
    setActivity(`trash:${path}`);
    const file = report.children.find((node) => node.path === path)!;
    setReport({ ...report, generation: ++generation,
      totalLogicalBytes: report.totalLogicalBytes - file.logicalBytes,
      children: report.children.filter((node) => node.path !== path), directChildCount: report.directChildCount - 1 });
    return { operationId: "fixture", requestedCount: 1, movedCount: 1, movedBytes: file.logicalBytes,
      cancelled: false, stoppedEarly: false, journalComplete: true, journalPath: "fixture-only",
      items: [{ path, logicalBytes: file.logicalBytes, status: "moved" as const, message: null }] };
  }
  return <AppShell activeView="overview" root={fixtureRoot} report={null} volume={null}
    mobileNavigationOpen={mobileOpen} selectionBlocked={false} dockerEnabled={false}
    onMobileNavigationChange={setMobileOpen} onNavigate={() => {}} onPickFolder={() => {}}>
    <StorageSectionNav activeView="overview" onNavigate={() => {}} />
    <div className="view-stack"><StorageTreemapPanel root={fixtureRoot} report={report}
      progress={null} state="success" error={null} breadcrumbs={breadcrumbs} blocked={false}
      onPickFolder={() => {}} onCancel={() => {}} onCancelTrash={() => {}}
      onStart={(root, next) => { setReport(fixtureReport(root)); setBreadcrumbs(next ?? [{ name: "Treemap", path: root }]); }}
      onReveal={async (path) => { setActivity(`reveal:${path}`); }}
      onTrash={moveFixture}
      onPrepareFolder={async (path, generation) => {
        if (params.has("prepare-failure")) throw new Error("Fixture: folder review exceeded its resource limit");
        const node = report.children.find((node) => node.path === path)!;
        const next = { id: "fixture-folder-plan", generation, path, logicalBytes: node.logicalBytes,
          fileCount: node.fileCount, directoryCount: node.directoryCount,
          expiresAtUnixMs: Date.now() + (params.has("expired") ? -1 : 300_000) };
        setPlan(next);
        setActivity(`review:${path}`);
        return next;
      }}
      onConfirmFolder={async (planId, generation, acknowledged) => {
        if (!plan || plan.id !== planId || plan.generation !== generation || plan.expiresAtUnixMs <= Date.now() || !acknowledged) {
          throw new Error("Fixture: invalid, expired, or unacknowledged plan");
        }
        setPlan(null);
        return moveFixture(plan.path);
      }}
      onDismissFolder={async (planId) => { setPlan((current) => current?.id === planId ? null : current); }} />
      <output aria-label="Fixture activity">{activity}</output>
    </div>
  </AppShell>;
}
createRoot(document.getElementById("root")!).render(<LanguageProvider><Fixture /></LanguageProvider>);
