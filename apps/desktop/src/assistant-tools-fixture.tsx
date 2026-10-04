// Synthetic UI integration fixture. No provider, filesystem, or native operation runs.
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { createRoot } from "react-dom/client";
import { useCallback, useState } from "react";
import { AssistantView } from "./views/AssistantView";
import { StorageTreemapPanel } from "./components/StorageTreemapPanel";
import { AssistantAppToolReview } from "./components/AssistantAppToolCard";
import { LanguageProvider } from "./i18n";
import { LANGUAGE_STORAGE_KEY } from "./i18n/preference";
import { confirmAssistantEmptyPlan, confirmAssistantFilePlan } from "./lib/bridge";
import { DEFAULT_SCAN_CONFIG } from "./types";
import type { AppToolResult, AssistantEmptyWorkspace, AssistantFileWorkspace, AssistantFileAction, AssistantSessionDetail, ControlStatus, DirectoryScanReport, TrashOperationResult } from "./types";
import "./App.css";

const params = new URLSearchParams(window.location.search);
window.localStorage.setItem(LANGUAGE_STORAGE_KEY, params.get("language") ?? "ko");
window.localStorage.setItem("bloomsweepy.assistant-provider", "codex");
mockWindows("main");
const now = Date.now();
const summary = { scopeName: "QA Workspace", completedAtUnixMs: now, totalLogicalBytes: 100,
  totalFiles: 1, totalDirectories: 3, unreadableEntries: 0, emptyDirectoryCount: 3, childrenTruncated: false, children: [] };
const session: AssistantSessionDetail = {
  session: { id: "aabbccdd", scopeKind: "folder", scopeRoot: "/Demo/QA Workspace", scopeName: "QA Workspace",
    createdAtUnixMs: now, updatedAtUnixMs: now, messageCount: 0, lastProvider: "codex", lastModel: null },
  folderSummary: summary, messages: [],
};
const seed: AssistantEmptyWorkspace = {
  revision: "fixture-revision", summary, totalFound: 3, omittedCount: 0,
  candidates: ["Empty draft", "Keep this folder", "Long folder name ".repeat(12)].map((name, index) => ({
    id: `candidate-${index + 1}`, number: index + 1, name, path: `/Demo/QA Workspace/${name}`,
  })), selectedIds: ["candidate-1", "candidate-2", "candidate-3"], plan: null,
};
let workspace: AssistantEmptyWorkspace | null = null;
const fileMode = params.has("files");
const fileSeed: AssistantFileWorkspace = {
  revision: "fixture-files", currentPath: session.session.scopeRoot, currentName: "QA Workspace", canGoUp: false,
  query: null, summary, totalEntries: 2, truncated: false, unreadableEntries: 0, offset: 0, nextOffset: null,
  sizeRanked: false, mapGeneration: 1,
  entries: [
    { id: "fixture-files-1", number: 1, name: "promo-video", path: "/Demo/QA Workspace/promo-video", isDirectory: true, logicalBytes: 1_420_000_000, fileCount: 15, directoryCount: 3, modifiedAtUnixMs: now },
    { id: "fixture-files-2", number: 2, name: "Keep this document.txt", path: "/Demo/QA Workspace/Keep this document.txt", isDirectory: false, logicalBytes: 2400, fileCount: 1, directoryCount: 0, modifiedAtUnixMs: now },
  ], selectedIds: [], plan: null,
};
let fileWorkspace: AssistantFileWorkspace | null = null;
let executions = 0;
let preparations = 0;
const appToolsMode = params.has("app-tools");
const clone = <T,>(value: T): T => structuredClone(value);
const control: ControlStatus = { revision: 1, bridgeAvailable: true, connectedClients: 0,
  lastConnectedAtUnixMs: null, activeOperation: null, lastOperation: null, pendingReview: null, lastError: null,
  protocolVersion: 3, searchAccess: { files: false, documents: false },
  scanAccess: { enabled: false, root: null, approvedAtUnixMs: null }, cleanupAccess: { enabled: false, approvedAtUnixMs: null } };

mockIPC((command, raw) => {
  const args = raw as Record<string, unknown>;
  if (command === "set_application_language") return null;
  if (command === "get_mcp_registration_statuses") return [];
  if (command === "get_assistant_provider_status") return [{ provider: "codex", label: "Codex · QA mock",
    installed: true, authentication: "authenticated", available: true, busy: false, detail: "Synthetic test adapter — no AI requests",
    models: [], state: "ready", executablePath: null, version: "fixture" }];
  if (command === "list_assistant_sessions") return [session.session];
  if (command === "get_assistant_session") return clone(session);
  if (command === "get_assistant_empty_workspace") return clone(workspace);
  if (command === "get_assistant_file_workspace") return clone(fileWorkspace);
  if (command === "get_assistant_directory_report" && fileWorkspace?.mapGeneration) {
    if (params.has("stale-map")) throw new Error("폴더 지도 결과가 만료되었습니다. 다시 검사하세요");
    if (args.revision !== fileWorkspace.revision) throw new Error("Stale workspace");
    return { generation: fileWorkspace.mapGeneration, root: fileWorkspace.currentPath, name: fileWorkspace.currentName,
      parent: null, completedAtUnixMs: now, durationMs: 12,
      totalLogicalBytes: fileWorkspace.entries.reduce((sum, entry) => sum + (entry.logicalBytes ?? 0), 0),
      totalFiles: 16, totalDirectories: 4, directChildCount: fileWorkspace.totalEntries,
      childrenTruncated: false, trackingLimitReached: false, omittedChildCount: 0, omittedLogicalBytes: 0,
      emptyDirectoryCount: 0, emptyDirectoriesTruncated: false, unreadableEntries: 0,
      children: fileWorkspace.entries.map(entry => ({ ...entry, logicalBytes: entry.logicalBytes ?? 0, fileCount: entry.fileCount ?? 0, directoryCount: entry.directoryCount ?? 0 })),
      emptyDirectories: [], issues: [] } satisfies DirectoryScanReport;
  }
  if (command === "append_assistant_message") {
    const request = args.request as { role: "user" | "assistant"; content: string; provider: "codex" | null; model: null };
    const message = { sequence: session.messages.length + 1, ...request,
      providerLabel: request.role === "user" ? null : request.provider ? "Codex · QA mock" : "BroomSweepy", createdAtUnixMs: Date.now() };
    session.messages.push(message);
    session.session.messageCount = session.messages.length;
    return clone({ session: session.session, message });
  }
  if (command === "ask_assistant") {
    const request = args.request as { message: string };
    if (appToolsMode) {
      const common = { source: "broomsweepy" as const, status: "completed" as const, capturedAtUnixMs: now, truncated: false };
      const performance: AppToolResult = { ...common, capability: "performance.inspect", data: { cpuUsagePercent: 12.5, memory: { usedBytes: 6_000_000_000, totalBytes: 8_000_000_000, availableBytes: 2_000_000_000 }, items: [{ displayName: "Editor <script>noop</script>", residentBytes: 2_000_000_000, cpuMachinePercent: 5.2 }], sourceProcessesTruncated: true }, truncated: true, presentation: { view: "performance" } };
      const applications: AppToolResult = { ...common, capability: "applications.list", data: { matchedCount: 2, items: [{ displayName: "Editor", displayVersion: "1.2", estimatedBytes: 150_000_000 }, { displayName: "Unmeasured app", estimatedBytes: null }] }, presentation: { view: "applications" } };
      let appToolResults: AppToolResult[] = [performance, applications];
      if (/문서/.test(request.message)) appToolResults = [{ ...common, capability: "documents.search", data: { query: "예산", contentExcerptsShared: true, returnedCount: 1, items: [{ name: "Budget.txt", logicalBytes: 2400, snippet: "허용한 일치 본문 <img src=x> — 실행되는 HTML이 아닙니다." }] }, presentation: { kind: "documentSearch" } }];
      if (/메모리 정리/.test(request.message)) appToolResults = [{ ...common, capability: "memory.review", status: "review_required", data: { executed: false }, presentation: { view: "performance", reviewKind: "memory", scope: "broomSweepyHostAllocator" } }];
      if (/종료/.test(request.message)) appToolResults = [{ ...common, capability: "processes.review", status: "review_required", data: { displayName: "Synthetic Editor", terminationRequested: false }, presentation: { view: "performance", reviewKind: "process", preview: { previewId: "mock-preview", displayName: "Synthetic Editor", pid: 123, capturedAtUnixMs: now, expiresAtUnixMs: now + 300_000 } } }];
      if (/앱 삭제/.test(request.message)) appToolResults = [{ ...common, capability: "applications.review", status: "review_required", data: { displayName: "Synthetic Editor", removed: false }, presentation: { view: "applications", reviewKind: "applicationBundle", inventoryId: "mock-inventory", applicationId: "mock-app", candidateIds: [], inventory: { platform: "macos", inventoryId: "mock-inventory", issues: [], applications: [{ id: "mock-app", displayName: "Synthetic Editor", displayVersion: "1.2", publisher: null, installLocation: "/Demo/Synthetic Editor.app", estimatedBytes: null, removalMode: "trashBundle", protectionReason: null }] }, plan: { planId: "mock-prepared-plan", displayName: "Synthetic Editor", path: "/Demo/Synthetic Editor.app", expiresAtUnixMs: now + 300_000, relatedData: [], warnings: [] } } }];
      fileWorkspace = clone(fileSeed);
      return { provider: "codex", label: "Codex · QA mock", model: null, message: "실제 앱 목록을 두 번 받아 분석했습니다. Editor가 가장 큰 메모리 사용 항목이며 Unmeasured app의 크기는 아직 모릅니다. 파일 카드가 있어도 이 최종 분석을 보존합니다.", analysisComplete: true, appToolResults, dockerContext: null, emptyWorkspace: null, fileWorkspace: clone(fileWorkspace), toolAction: "app" };
    }
    if (fileMode) {
      fileWorkspace ??= clone(fileSeed);
      if (/가장|큰|largest|biggest/.test(request.message)) {
        fileWorkspace.sizeRanked = true;
        fileWorkspace.selectedIds = []; fileWorkspace.plan = null;
      } else if (/지워|삭제|delete/.test(request.message) && !/삭제해도|삭제 해도|지워도|can .*delete/i.test(request.message)) {
        fileWorkspace.selectedIds = [fileWorkspace.entries[0].id];
        prepareFiles();
      }
      return { provider: "codex", label: "Codex · QA mock", model: null, message: "Synthetic file tool result",
        dockerContext: null, emptyWorkspace: null, fileWorkspace: clone(fileWorkspace), toolAction: "files" };
    }
    let action: "scan" | "selection" | null = null;
    if (request.message.includes("검사") || request.message.includes("scan")) { workspace = clone(seed); action = "scan"; }
    else if (workspace && request.message.includes("2")) { workspace.selectedIds = workspace.selectedIds.filter(id => id !== "candidate-2"); workspace.plan = null; action = "selection"; }
    return { provider: "codex", label: "Codex · QA mock", model: null, message: "이 메시지는 테스트 응답입니다. 최종 확인 버튼을 사용하세요.",
      dockerContext: null, emptyWorkspace: action ? clone(workspace) : null, toolAction: action };
  }
  if (command === "execute_graceful_process_termination") { executions++; return { outcome: "requestSent", displayName: "Synthetic Editor", requestedAtUnixMs: Date.now() }; }
  if (command === "clean_app_memory") { executions++; return { outcome: "completed", allocatorReleasedBytes: 0, appResidentBeforeBytes: 1000, appResidentAfterBytes: 1000, systemAvailableBeforeBytes: 2000, systemAvailableAfterBytes: 2000, requestedAtUnixMs: now, completedAtUnixMs: Date.now() }; }
  if (command === "prepare_application_trash" || command === "prepare_application_data_trash") { preparations++; throw new Error("Prepared tool reviews must not be prepared again"); }
  if (command === "dismiss_application_plan") return null;
  if (command === "confirm_application_trash" || command === "confirm_application_data_trash") {
    executions++; return { requestedCount: 1, movedCount: 1, movedBytes: 0, cancelled: false, journalComplete: true, items: [{ path: "/Demo/Synthetic Editor.app", logicalBytes: 0, status: "moved", message: null }] };
  }
  if (command === "assistant_file_action") {
    const operation = args.operation as AssistantFileAction;
    fileWorkspace = clone(fileSeed);
    if (operation.kind === "largest") fileWorkspace.sizeRanked = true;
    if (operation.kind === "search") {
      fileWorkspace.query = operation.query;
      fileWorkspace.mapGeneration = null;
      fileWorkspace.entries = fileWorkspace.entries.filter(entry => entry.name.includes(operation.query));
      fileWorkspace.entries = fileWorkspace.entries.map(entry => entry.isDirectory ? { ...entry, logicalBytes: null, fileCount: null, directoryCount: null } : entry);
      fileWorkspace.totalEntries = fileWorkspace.entries.length;
    }
    return clone(fileWorkspace);
  }
  if (command === "select_assistant_files" && fileWorkspace) {
    fileWorkspace.selectedIds = args.ids as string[]; fileWorkspace.plan = null; return clone(fileWorkspace);
  }
  if (command === "prepare_assistant_file_plan" && fileWorkspace) { prepareFiles(); return clone(fileWorkspace); }
  if (command === "confirm_assistant_file_plan" && fileWorkspace?.plan) {
    const plan = fileWorkspace.plan;
    if (args.planId !== plan.id || (plan.requiresNestedAck && !args.nestedContentsAcknowledged) || plan.expiresAtUnixMs <= Date.now()) throw new Error("Unconfirmed or expired plan");
    fileWorkspace = null; executions += 1;
    document.getElementById("fixture-executions")!.textContent = `Mock executions: ${executions}`;
    return { operationId: "fixture-files-only", requestedCount: plan.entries.length, movedCount: plan.entries.length, movedBytes: plan.logicalBytes,
      cancelled: false, stoppedEarly: false, journalComplete: true, journalPath: "/Demo/mock-journal",
      items: plan.entries.map(entry => ({ path: entry.path, logicalBytes: entry.logicalBytes ?? 0, status: "moved", message: null })) } satisfies TrashOperationResult;
  }
  if (command === "select_assistant_empty_candidates" && workspace) {
    workspace.selectedIds = args.candidateIds as string[]; workspace.plan = null; return clone(workspace);
  }
  if (command === "prepare_assistant_empty_plan" && workspace) {
    workspace.plan = { id: `plan-${Date.now()}`, candidateIds: [...workspace.selectedIds],
      expiresAtUnixMs: params.has("expired") ? Date.now() - 1 : Date.now() + 300_000 };
    return clone(workspace);
  }
  if (command === "confirm_assistant_empty_plan" && workspace?.plan && workspace.plan.id === args.planId) {
    const items = workspace.candidates.filter(candidate => workspace!.plan!.candidateIds.includes(candidate.id));
    workspace = null; executions += 1;
    document.getElementById("fixture-executions")!.textContent = `Mock executions: ${executions}`;
    return { operationId: "fixture-only", requestedCount: items.length, movedCount: items.length, movedBytes: 0,
      cancelled: false, stoppedEarly: false, journalComplete: true, journalPath: "/Demo/mock-journal",
      items: items.map(item => ({ path: item.path, logicalBytes: 0, status: "moved", message: null })) } satisfies TrashOperationResult;
  }
  throw new Error(`Synthetic fixture rejects command: ${command}`);
}, { shouldMockEvents: true });
function prepareFiles() {
  if (!fileWorkspace) throw new Error("No file workspace");
  const entries = fileWorkspace.entries.filter(entry => fileWorkspace!.selectedIds.includes(entry.id));
  fileWorkspace.plan = { id: `file-plan-${Date.now()}`, entries, logicalBytes: entries.reduce((sum, entry) => sum + (entry.logicalBytes ?? 0), 0),
    requiresNestedAck: entries.some(entry => entry.isDirectory), expiresAtUnixMs: params.has("expired") ? Date.now() - 1 : Date.now() + 300_000 };
}
const noop = () => undefined;
function Fixture() {
  const [appReview, setAppReview] = useState<AppToolResult | null>(null);
  const [localNotice, setLocalNotice] = useState("");
  const [map, setMap] = useState<DirectoryScanReport | null>(null);
  const [showMap, setShowMap] = useState(false);
  const acceptMap = useCallback((report: DirectoryScanReport, open: boolean) => { setMap(report); if (open) setShowMap(true); }, []);
  return <main style={{ maxWidth: 1100, margin: "0 auto", padding: 20 }}>
    <div style={{ display: "contents" }} inert={appReview !== null}>
    <p role="note">합성 QA · 실제 파일 검사/이동 및 AI 전송 없음 · “검사”, “2번 보관”을 사용</p>
    <output id="fixture-executions">Mock executions: 0</output>
    <output>Shared map: {map?.root ?? "none"} · {map?.totalLogicalBytes ?? 0} B</output>
    {showMap ? <><button type="button" className="secondary-button" onClick={() => setShowMap(false)}>대화로 돌아가기</button>
      <StorageTreemapPanel root={map?.root ?? null} report={map} progress={null} state="success" error={null}
        breadcrumbs={map ? [{ path: map.root, name: map.name }] : []} blocked={false} showAction={false}
        onPickFolder={noop} onStart={noop} onCancel={noop} onReveal={async () => undefined} />
    </> : <AssistantView status={control} canEnableSearch={false} updatingSearchAccess={false} searchAccessError={null}
      onToggleSearchAccess={noop} scanRoot={session.session.scopeRoot} scanConfig={DEFAULT_SCAN_CONFIG} canEnableScan={false}
      updatingScanAccess={false} scanAccessError={null} onToggleScanAccess={noop} canEnableCleanup={false}
      cleanupAccessLocked={false} updatingCleanupAccess={false} cleanupAccessError={null} onToggleCleanupAccess={noop}
      onReviewPending={noop} directoryProgress={null} directoryState="success" volumes={[]} dockerStatus={null}
      launchRequest={null} onLaunchRequestHandled={noop} onPickFolder={async () => null} onConfirmEmptyPlan={confirmAssistantEmptyPlan} onConfirmFilePlan={confirmAssistantFilePlan} onDirectoryReport={acceptMap}
      onAppToolReview={(prepared) => setAppReview(prepared)} onAppToolView={(result) => setLocalNotice(`Explicit navigation: ${result.capability}`)} />}
    </div>
    {appReview ? <AssistantAppToolReview result={appReview} onClose={() => setAppReview(null)} onCompleted={(result) => setLocalNotice(result.message)} /> : null}
    <output data-testid="app-tool-fixture-result">{localNotice} · Mock preparations: {preparations} · Mock executions: {executions}</output>
  </main>;
}
createRoot(document.getElementById("root")!).render(<LanguageProvider><Fixture /></LanguageProvider>);
