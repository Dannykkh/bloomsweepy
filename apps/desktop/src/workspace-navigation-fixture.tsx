import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { useState } from "react";
import { createRoot } from "react-dom/client";
import "@fontsource/jetbrains-mono/latin-500.css";
import App from "./App";
import { LanguageProvider } from "./i18n";
import { LANGUAGE_STORAGE_KEY } from "./i18n/preference";

// Isolated entry: real App composition; no command can reach the native bridge.
const params = new URLSearchParams(window.location.search);
const platform = params.get("platform") === "windows" ? "windows" : "macos";
const windowsCase = params.get("windows-case") ?? "default";
window.localStorage.setItem(LANGUAGE_STORAGE_KEY, params.get("language") ?? "ko");
document.documentElement.dataset.windowMaterial = "opaque";
Object.defineProperty(window, "isTauri", { value: true, configurable: true });
mockWindows("main");
const gb = 1024 ** 3;
const volumes = [
  { name: platform === "macos" ? "Macintosh HD" : "Local Disk", mountPoint: platform === "macos" ? "/" : "C:\\", fileSystem: platform === "macos" ? "APFS" : "NTFS", totalBytes: 256 * gb, availableBytes: 24 * gb, removable: false, readOnly: false, isDiskImage: false, isSystem: true },
  ...(params.get("drives") === "3" ? [
    { name: "QA Archive", mountPoint: platform === "macos" ? "/Volumes/QA-Archive" : "D:\\", fileSystem: "exFAT", totalBytes: 512 * gb, availableBytes: 320 * gb, removable: true, readOnly: false, isDiskImage: false, isSystem: false },
    { name: "QA Work", mountPoint: platform === "macos" ? "/Volumes/QA-Work" : "E:\\", fileSystem: "APFS", totalBytes: 128 * gb, availableBytes: 64 * gb, removable: false, readOnly: false, isDiskImage: false, isSystem: false },
  ] : []),
];
const reads = new Map<string, number>();
let rejected = 0;
let settingsOpenRequests = 0;
let publish = (_value: string) => {};
const control = { revision: 1, bridgeAvailable: true, connectedClients: 0, lastConnectedAtUnixMs: null, activeOperation: null, lastOperation: null, pendingReview: null, lastError: null, protocolVersion: 3, searchAccess: { files: false, documents: false }, scanAccess: { enabled: false, root: null, approvedAtUnixMs: null }, cleanupAccess: { enabled: false, approvedAtUnixMs: null }, inspectionAllowed: false, assistantPermissionMode: "confirm", permissionLifetime: "session", chatTrashWithoutConfirmation: false };
const preferenceRows = ["show_extensions", "show_hidden_files", "show_path_bar", "show_status_bar", "folders_first"];
const windowsTargetIds = ["folder_options", "storage", "startup", "apps", "taskbar", "privacy", "about", "optional_features"];
const windowsRows = ["explorer_hide_extensions", "explorer_hidden_files", "explorer_protected_files", "explorer_full_path", "legacy_copilot", "recall_snapshots", "recall_snapshots_user", "recall_availability", "notepad_ai", "paint_image_creator"];
function windowsSnapshot() {
  const supported = platform === "windows" && windowsCase !== "unsupported";
  return {
    supported, platform, osBuild: supported ? "26100" : null, osRevision: supported ? 3915 : null, edition: supported ? "Synthetic QA" : null,
    settings: !supported ? [] : windowsRows.map((id, index) => {
      const state = windowsCase === "mixed" ? ["value", "unset", "typeMismatch", "accessDenied", "unsupportedValue", "error"][index % 6] : index === 3 ? "unset" : "value";
      return { id, title: id, description: "Synthetic stored value", category: index < 4 ? "explorer" : "aiPolicy", scope: index < 5 || id === "recall_snapshots_user" ? "currentUser" : "machine", state, value: state === "value" ? index % 2 : state === "unsupportedValue" ? 9 : null, valueType: state === "typeMismatch" ? "REG_SZ" : state === "value" || state === "unsupportedValue" ? "REG_DWORD" : null, support: index < 4 ? "storedValueOnly" : id === "legacy_copilot" ? "deprecated" : "requiresVerification", targetId: index < 4 ? "folder_options" : "privacy" };
    }),
    targets: !supported ? [] : windowsTargetIds.map(id => ({ id, title: id, available: true })),
    warnings: supported ? [] : ["Synthetic unsupported host; no Windows native calls."],
  };
}
const statusText = () => `합성 QA · 실제 파일 작업/AI 전송 없음 · 거부 ${rejected} · 모델 조회 ${reads.get("get_assistant_provider_status") ?? 0} · 설정 진입 요청 ${settingsOpenRequests} · ${platform}`;
mockIPC((command, payload) => {
  reads.set(command, (reads.get(command) ?? 0) + 1);
  publish(statusText());
  switch (command) {
    case "set_application_language": return;
    case "get_action_recovery_status": return { checkedAtUnixMs: Date.now(), journalPath: "/synthetic/journal", trashLookupSupported: true, trashLookupPerformed: false, incompleteOperations: [], issues: [] };
    case "get_system_overview": return { platform, volumes };
    case "get_action_history": return { checkedAtUnixMs: Date.now(), journalPath: "/synthetic/journal", entries: [], issues: [] };
    case "get_recent_file_catalog_entries":
    case "get_file_catalog_status":
    case "get_document_index_status":
    case "get_pending_cleanup_plan": return null;
    case "get_control_status": return control;
    case "get_docker_management_status": return { enabled: false, cliInstalled: null, daemonRunning: null, busy: false, detail: "Synthetic QA", clientVersion: null, serverVersion: null, capturedAtUnixMs: null, totalSizeBytes: 0, reclaimableBytes: 0, categories: [], lastCleanup: null };
    case "get_performance_snapshot": return { snapshotId: "synthetic", capturedAtUnixMs: Date.now(), sampleWindowMs: 2000, refreshAfterMs: 2000, platform, logicalCpuCount: 8, cpuUsagePercent: 16, memory: { totalBytes: 8 * gb, availableBytes: 4 * gb, usedBytes: 4 * gb, totalSwapBytes: 0, usedSwapBytes: 0 }, processes: [], processesTruncated: false, capabilities: { processMetricsAvailable: true, gracefulTerminationAvailable: platform === "macos", appMemoryCleanupAvailable: platform === "macos", processScope: platform === "macos" ? "guiApplications" : "topProcesses" } };
    case "plugin:autostart|is_enabled": return false;
    case "get_menu_bar_settings": return { supported: platform === "macos", available: platform === "macos", showMemoryPercent: false };
    case "get_assistant_provider_status": return [{ provider: "codex", label: "Codex", installed: true, authentication: "authenticated", available: true, busy: false, detail: "Synthetic QA; sending disabled", models: [{ id: "synthetic-model", label: "Synthetic model", supportedReasoningEfforts: ["low", "medium", "high"], defaultReasoningEffort: "medium" }], modelSelection: "optional", modelCatalogSource: "cli", state: "ready", executablePath: null, version: "QA" }];
    case "get_mcp_registration_statuses":
    case "list_assistant_sessions": return [];
    case "get_file_change_review": return { status: { config: { enabled: false, root: null, intervalMinutes: 15 }, phase: "off", baselineEntries: 0, lastCheckedAtUnixMs: null, nextCheckAtUnixMs: null, historyCount: 0, reviewNeededCount: 0, skippedCount: 0, historyTruncated: false, error: null, revision: 1 }, entries: [], offset: 0, total: 0 };
    case "get_application_inventory": return { platform, inventoryId: "synthetic", applications: [], issues: [] };
    case "get_mac_settings": return { supported: platform === "macos", platform, settings: platform !== "macos" ? [] : preferenceRows.map(id => ({ id, title: id, description: "Synthetic preference", domain: "synthetic", key: id, value: null, state: "unset", managed: false, canModify: true })), warnings: [], history: [] };
    case "get_windows_settings":
      if (windowsCase === "error") throw new Error("Synthetic Windows read error");
      if (windowsCase === "slow") return new Promise(resolve => setTimeout(() => resolve(windowsSnapshot()), 1600));
      return windowsSnapshot();
    case "open_windows_settings_target": {
      const targetId = (payload as { targetId?: unknown } | undefined)?.targetId;
      if (platform !== "windows" || typeof targetId !== "string" || !windowsTargetIds.includes(targetId)) throw new Error("Synthetic launch rejected");
      if (windowsCase === "open-error") throw new Error("Synthetic settings launch error");
      settingsOpenRequests += 1;
      publish(statusText());
      return { targetId, requested: true, changed: false };
    }
    default:
      rejected += 1;
      publish(statusText());
      throw new Error(`QA blocked command: ${command}`);
  }
}, { shouldMockEvents: true });

function Fixture() {
  const [status, setStatus] = useState(statusText());
  publish = setStatus;
  return <><App /><output data-testid="fixture-status" style={{ position: "fixed", zIndex: 1000, right: 8, bottom: 2, maxWidth: "95vw", padding: "2px 6px", fontSize: 12, color: "#c8cbd2", background: "#202329", pointerEvents: "none" }}>{status}</output></>;
}
const fixtureRoot = createRoot(document.getElementById("root")!);
fixtureRoot.render(<LanguageProvider><Fixture /></LanguageProvider>);
if (import.meta.hot) import.meta.hot.dispose(() => { publish = () => {}; fixtureRoot.unmount(); });
