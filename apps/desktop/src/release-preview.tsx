// Documentation-only entry point. Not included in the packaged application.
// Render the real views with synthetic data; never invoke a native tool or AI.
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { useState } from "react";
import { createRoot } from "react-dom/client";
import "./App.css";
import { AppShell } from "./components/AppShell";
import { StorageSectionNav } from "./components/StorageSectionNav";
import { StorageTreemapPanel } from "./components/StorageTreemapPanel";
import { LanguageProvider } from "./i18n";
import { LANGUAGE_STORAGE_KEY } from "./i18n/preference";
import { DEFAULT_SCAN_CONFIG } from "./types";
import type { AssistantSessionDetail, ControlStatus, DirectoryScanReport, PerformanceSnapshot, ViewId, VolumeInfo } from "./types";
import { AssistantView } from "./views/AssistantView";
import { DashboardView } from "./views/DashboardView";
import { PerformanceView } from "./views/PerformanceView";
import { SettingsView } from "./views/SettingsView";

const params = new URLSearchParams(window.location.search);
const language = params.get("language") ?? "en";
window.localStorage.setItem(LANGUAGE_STORAGE_KEY, language);
window.localStorage.setItem("bloomsweepy.assistant-provider", "codex");
mockWindows("main");
const GiB = 1024 ** 3;
const MiB = 1024 ** 2;
const now = Date.now();
const root = "/Demo/Workspace";
const noop = () => undefined;
const volume: VolumeInfo = {
  name: "Macintosh HD", mountPoint: "/", fileSystem: "apfs", totalBytes: 494 * GiB,
  availableBytes: 142 * GiB, removable: false, readOnly: false, isDiskImage: false, isSystem: true,
};
const volumes: VolumeInfo[] = [volume, {
  ...volume, name: "Studio SSD", mountPoint: "/Volumes/Studio SSD", totalBytes: 1_000 * GiB,
  availableBytes: 680 * GiB, removable: true, isSystem: false,
}, {
  ...volume, name: "Archive", mountPoint: "/Volumes/Archive", totalBytes: 2_000 * GiB,
  availableBytes: 860 * GiB, removable: true, isSystem: false,
}];
const children = [
  ["Video projects", 28, true], ["Design assets", 16, true], ["Documents", 9, true],
  ["recording.mov", 6, false], ["project-backup.zip", 4, false], ["Photos", 3, true],
  ["presentation.pdf", 0.8, false], ["notes.md", 0.02, false],
].map(([name, size, directory], index) => ({
  name: String(name), path: `${root}/${name}`, logicalBytes: Number(size) * GiB,
  isDirectory: Boolean(directory), fileCount: directory ? 120 + index * 12 : 1,
  directoryCount: directory ? 4 : 0, modifiedAtUnixMs: now - 3_600_000,
}));
const report: DirectoryScanReport = {
  generation: 1, root, name: "Workspace", parent: "/Demo", completedAtUnixMs: now,
  durationMs: 780, totalLogicalBytes: children.reduce((sum, item) => sum + item.logicalBytes, 0),
  totalFiles: 628, totalDirectories: 16, directChildCount: children.length,
  childrenTruncated: false, trackingLimitReached: false, omittedChildCount: 0,
  omittedLogicalBytes: 0, emptyDirectoryCount: 0, emptyDirectoriesTruncated: false,
  unreadableEntries: 0, children, emptyDirectories: [], issues: [],
};
const snapshot = (): PerformanceSnapshot => ({
  snapshotId: "documentation-sample", capturedAtUnixMs: Date.now(), sampleWindowMs: 2_000,
  refreshAfterMs: 2_000, platform: "macos", logicalCpuCount: 10, cpuUsagePercent: 37.4,
  memory: { totalBytes: 24 * GiB, availableBytes: 8.2 * GiB, usedBytes: 15.8 * GiB,
    totalSwapBytes: 4 * GiB, usedSwapBytes: 620 * MiB },
  processes: [
    ["Figma", 14.2, 2.86, 9], ["Google Chrome", 8.8, 3.42, 18],
    ["Music", 2.6, 0.78, 5], ["BroomSweepy", 0.4, 0.176, 1],
  ].map(([name, cpu, memory, count], index) => ({
    targetId: index < 3 ? `demo-${index}` : null, displayName: String(name),
    bundleIdentifier: null, pid: 12000 + index, kind: "guiApp", cpuCorePercent: Number(cpu) * 10,
    cpuMachinePercent: Number(cpu), residentBytes: Number(memory) * GiB, processCount: Number(count),
    canRequestTermination: index < 3, terminationEligibility: index < 3 ? "eligible" : "selfApp",
  })),
  processesTruncated: false, capabilities: { processMetricsAvailable: true,
    gracefulTerminationAvailable: true, appMemoryCleanupAvailable: true, processScope: "guiApplications" },
});
const chatText: Record<string, [string, string]> = {
  en: ["Where should I start reviewing this folder?", "Start with Video projects (28 GB), then Design assets (16 GB). Together they account for most of this folder.\n\nOpen the storage map to review the largest items. Check whether project-backup.zip is still needed before selecting it. I have not moved or deleted any files."],
  ko: ["이 폴더는 어디부터 확인하면 좋을까?", "Video projects(28 GB)와 Design assets(16 GB)부터 살펴보세요. 두 폴더가 대부분의 공간을 사용합니다.\n\n용량 지도에서 큰 항목부터 확인하고, project-backup.zip이 아직 필요한 백업인지 검토해 주세요. 파일을 이동하거나 삭제하지 않았습니다."],
  ja: ["このフォルダはどこから確認すればよいですか？", "まずVideo projects（28 GB）、次にDesign assets（16 GB）を確認してください。この2つが容量の大部分を占めています。\n\nストレージマップで大きい項目を確認し、project-backup.zipが必要なバックアップかどうか検討してください。ファイルの移動や削除は行っていません。"],
  "zh-CN": ["这个文件夹应该从哪里开始检查？", "建议先查看Video projects（28 GB），再查看Design assets（16 GB）。这两个文件夹占用了大部分空间。\n\n在存储空间地图中检查较大的项目，并确认project-backup.zip是否仍是需要保留的备份。我没有移动或删除任何文件。"],
};
const dialogue = chatText[language] ?? chatText.en;
const session: AssistantSessionDetail = {
  session: { id: "demo-session", scopeKind: "folder", scopeRoot: root, scopeName: "Workspace",
    createdAtUnixMs: now, updatedAtUnixMs: now, messageCount: 2, lastProvider: "codex", lastModel: null },
  folderSummary: { scopeName: "Workspace", completedAtUnixMs: now,
    totalLogicalBytes: report.totalLogicalBytes, totalFiles: report.totalFiles, totalDirectories: 16,
    unreadableEntries: 0, emptyDirectoryCount: 0, childrenTruncated: false,
    children: children.map(item => ({ name: item.name, kind: item.isDirectory ? "directory" : "file",
      logicalBytes: item.logicalBytes, fileCount: item.fileCount, directoryCount: item.directoryCount })) },
  messages: dialogue.map((content, index) => ({ sequence: index + 1,
    role: index === 0 ? "user" : "assistant", content, provider: index === 0 ? null : "codex",
    providerLabel: index === 0 ? null : "Codex", model: null, createdAtUnixMs: now })),
};
const control: ControlStatus = {
  revision: 1, bridgeAvailable: true, connectedClients: 0, lastConnectedAtUnixMs: null,
  activeOperation: null, lastOperation: null, pendingReview: null, lastError: null, protocolVersion: 3,
  searchAccess: { files: false, documents: false },
  scanAccess: { enabled: false, root: null, approvedAtUnixMs: null },
  cleanupAccess: { enabled: false, approvedAtUnixMs: null },
};
mockIPC((command) => {
  if (command === "get_performance_snapshot") return snapshot();
  if (command === "get_assistant_provider_status") return [{ provider: "codex", label: "Codex",
    installed: true, authentication: "authenticated", available: true, busy: false,
    detail: "Documentation sample", models: [], state: "ready", executablePath: null, version: null }];
  if (command === "list_assistant_sessions") return [session.session];
  if (command === "get_assistant_session") return session;
  if (command === "get_mcp_registration_statuses") return [];
  if (command === "plugin:autostart|is_enabled") return false;
  if (command === "set_application_language") return null;
  // Reject all unimplemented actions. This page cannot execute native operations.
  throw new Error(`Documentation preview: ${command} is not available`);
}, { shouldMockEvents: true });

function Preview() {
  const [view, setView] = useState<ViewId>((params.get("view") ?? "dashboard") as ViewId);
  const [mobileOpen, setMobileOpen] = useState(false);
  const [config, setConfig] = useState(DEFAULT_SCAN_CONFIG);
  return <AppShell activeView={view} root={view === "dashboard" || view === "performance" || view === "settings" ? null : root}
    report={null} volume={volume} mobileNavigationOpen={mobileOpen} selectionBlocked={false}
    dockerEnabled={false} onMobileNavigationChange={setMobileOpen} onNavigate={setView} onPickFolder={noop}>
    {view === "dashboard" ? <DashboardView system={{ platform: "macos", volumes }} report={null}
      progress={null} scanState="idle" scanError={null} cleanupReport={null} cleanupProgress={null}
      cleanupState="idle" cleanupError={null} actionHistory={null} recentFiles={null} fileCatalog={null}
      fileCatalogStale={false} loading={false} error={null} blocked={false} onRefresh={noop}
      onStartScan={noop} onCancelScan={noop} onStartCleanupScan={noop} onCancelCleanupScan={noop}
      onOpenStorage={() => setView("overview")} onOpenLargeFiles={noop} onOpenDuplicates={noop}
      onOpenCleanup={noop} onOpenPerformance={() => setView("performance")} onRefreshFileCatalog={noop}
      onOpenFileSearch={noop} onRevealFile={noop} /> : null}
    {view === "overview" ? <><StorageSectionNav activeView="overview" onNavigate={noop} />
      <div className="view-stack"><StorageTreemapPanel root={root} report={report} progress={null}
        state="success" error={null} breadcrumbs={[{ name: "Workspace", path: root }]} blocked={false}
        onPickFolder={noop} onCancel={noop} onCancelTrash={noop} onStart={noop}
        onReveal={async () => { throw new Error("Documentation preview only"); }}
        onTrash={async () => { throw new Error("Documentation preview only"); }} /></div></> : null}
    {view === "performance" ? <PerformanceView /> : null}
    {view === "assistant" ? <AssistantView status={control} canEnableSearch={false}
      updatingSearchAccess={false} searchAccessError={null} onToggleSearchAccess={noop}
      scanRoot={root} scanConfig={config} canEnableScan={false} updatingScanAccess={false}
      scanAccessError={null} onToggleScanAccess={noop} canEnableCleanup={false} cleanupAccessLocked={false}
      updatingCleanupAccess={false} cleanupAccessError={null} onToggleCleanupAccess={noop} onReviewPending={noop}
      directoryProgress={null} directoryState="success" volumes={volumes} dockerStatus={null}
      launchRequest={null} onLaunchRequestHandled={noop} onPickFolder={async () => null} /> : null}
    {view === "settings" ? <SettingsView config={config} onConfigChange={setConfig} dockerStatus={null}
      dockerLoading={false} dockerChanging={false} dockerError={null} onDockerEnabledChange={async () => {}}
      onOpenDocker={noop} /> : null}
  </AppShell>;
}
createRoot(document.getElementById("root")!).render(<LanguageProvider><Preview /></LanguageProvider>);
