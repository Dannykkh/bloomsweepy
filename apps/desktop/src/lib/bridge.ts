import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
import { decideFileInspection } from "./fileInspectionPolicy";
import type {
  AssistantChatRequest,
  AssistantChatResponse,
  AssistantSessionSummary,
  AssistantSessionDetail,
  CreateAssistantSessionRequest,
  AppendAssistantMessageRequest,
  AssistantMessageMutation,
  AssistantProviderStatus,
  DirectoryScanProgress,
  DirectoryScanReport,
  CleanupScanProgress,
  CleanupScanReport,
  DriveScanProgress,
  DriveScanReport,
  ScanConfig,
  ScanProgress,
  ScanReport,
  SystemOverview,
  CleanupTrashRequest,
  DuplicateTrashRequest,
  TrashOperationResult,
  TrashProgress,
  ActionRecoveryReport,
  ActionHistoryReport,
  DocumentIndexProgress,
  DocumentIndexReport,
  DocumentIndexStatus,
  DocumentSearchReport,
  DocumentSearchRequest,
  FileCatalogProgress,
  FileCatalogReport,
  FileCatalogSearchReport,
  FileCatalogSearchRequest,
  FileCatalogStatus,
  FileCatalogRecentReport,
  FileCatalogEntryKind,
  ControlStatus,
  ControlSearchAccessRequest,
  ControlScanAccessRequest,
  ControlScanProgressEvent,
  ControlScanCompletedEvent,
  ScanReportSnapshot,
  DockerManagementStatus,
  DockerCleanupPreview,
  ExecuteDockerCleanupRequest,
  DockerCleanupResult,
  DockerCleanupProgress,
  ControlCleanupAccessRequest,
  PendingCleanupPlanDetail,
  ApproveCleanupPlanRequest,
  McpClientKind,
  McpClientRegistrationStatus,
} from "../types";

export function getAssistantProviderStatus(): Promise<AssistantProviderStatus[]> {
  return invoke<AssistantProviderStatus[]>("get_assistant_provider_status");
}

export function askAssistant(
  request: AssistantChatRequest,
): Promise<AssistantChatResponse> {
  return invoke<AssistantChatResponse>("ask_assistant", { request });
}

export function cancelAssistant(): Promise<boolean> {
  return invoke<boolean>("cancel_assistant");
}

export function listAssistantSessions(): Promise<AssistantSessionSummary[]> {
  return invoke<AssistantSessionSummary[]>("list_assistant_sessions");
}

export function createAssistantSession(
  request: CreateAssistantSessionRequest,
): Promise<AssistantSessionDetail> {
  return invoke<AssistantSessionDetail>("create_assistant_session", { request });
}

export function getAssistantSession(sessionId: string): Promise<AssistantSessionDetail> {
  return invoke<AssistantSessionDetail>("get_assistant_session", { sessionId });
}

export function appendAssistantMessage(
  request: AppendAssistantMessageRequest,
): Promise<AssistantMessageMutation> {
  return invoke<AssistantMessageMutation>("append_assistant_message", { request });
}

export function deleteAssistantSession(sessionId: string): Promise<boolean> {
  return invoke<boolean>("delete_assistant_session", { sessionId });
}

export function getDockerManagementStatus(): Promise<DockerManagementStatus> {
  return invoke<DockerManagementStatus>("get_docker_management_status");
}

export function setDockerManagementEnabled(
  enabled: boolean,
): Promise<DockerManagementStatus> {
  return invoke<DockerManagementStatus>("set_docker_management_enabled", { enabled });
}

export function createDockerCleanupPreview(): Promise<DockerCleanupPreview> {
  return invoke<DockerCleanupPreview>("create_docker_cleanup_preview");
}

export function executeDockerCleanup(
  request: ExecuteDockerCleanupRequest,
): Promise<DockerCleanupResult> {
  return invoke<DockerCleanupResult>("execute_docker_cleanup", { request });
}

export function cancelDockerCleanup(): Promise<boolean> {
  return invoke<boolean>("cancel_docker_cleanup");
}

export function getControlStatus(): Promise<ControlStatus> {
  return invoke<ControlStatus>("get_control_status");
}

export function configureControlSearchAccess(
  request: ControlSearchAccessRequest,
): Promise<ControlStatus> {
  return invoke<ControlStatus>("configure_control_search_access", { request });
}

export function configureControlScanAccess(
  request: ControlScanAccessRequest,
): Promise<ControlStatus> {
  return invoke<ControlStatus>("configure_control_scan_access", { request });
}

export function configureControlCleanupAccess(
  request: ControlCleanupAccessRequest,
): Promise<ControlStatus> {
  return invoke<ControlStatus>("configure_control_cleanup_access", { request });
}

export function getPendingCleanupPlan(): Promise<PendingCleanupPlanDetail | null> {
  return invoke<PendingCleanupPlanDetail | null>("get_pending_cleanup_plan");
}

export function approveCleanupPlan(
  request: ApproveCleanupPlanRequest,
): Promise<TrashOperationResult> {
  return invoke<TrashOperationResult>("approve_cleanup_plan", { request });
}

export function rejectCleanupPlan(planId: string): Promise<boolean> {
  return invoke<boolean>("reject_cleanup_plan", { planId });
}

export function getMcpRegistrationStatuses(): Promise<McpClientRegistrationStatus[]> {
  return invoke<McpClientRegistrationStatus[]>("get_mcp_registration_statuses");
}

export function registerMcpClient(
  client: McpClientKind,
): Promise<McpClientRegistrationStatus> {
  return invoke<McpClientRegistrationStatus>("register_mcp_client", { client });
}

export function unregisterMcpClient(
  client: McpClientKind,
): Promise<McpClientRegistrationStatus> {
  return invoke<McpClientRegistrationStatus>("unregister_mcp_client", { client });
}

export function getSystemOverview(): Promise<SystemOverview> {
  return invoke<SystemOverview>("get_system_overview");
}

export function startScan(root: string, config: ScanConfig): Promise<ScanReport> {
  return invoke<ScanReport>("start_scan", { root, config });
}

export function getScanReportSnapshot(
  scanGeneration: number,
): Promise<ScanReportSnapshot> {
  return invoke<ScanReportSnapshot>("get_scan_report_snapshot", { scanGeneration });
}

export function startDriveScan(root: string): Promise<DriveScanReport> {
  return invoke<DriveScanReport>("start_drive_scan", { root, config: null });
}

export function startDirectoryScan(root: string): Promise<DirectoryScanReport> {
  return invoke<DirectoryScanReport>("start_directory_scan", { root, config: null });
}

export function startCleanupScan(): Promise<CleanupScanReport> {
  return invoke<CleanupScanReport>("start_cleanup_scan");
}

export function cancelScan(): Promise<boolean> {
  return invoke<boolean>("cancel_scan");
}

export function getActionRecoveryStatus(): Promise<ActionRecoveryReport> {
  return invoke<ActionRecoveryReport>("get_action_recovery_status");
}

export function getActionHistory(): Promise<ActionHistoryReport> {
  return invoke<ActionHistoryReport>("get_action_history");
}

export function openSystemTrash(): Promise<void> {
  return invoke<void>("open_system_trash");
}

export function getDocumentIndexStatus(): Promise<DocumentIndexStatus | null> {
  return invoke<DocumentIndexStatus | null>("get_document_index_status");
}

export function startDocumentIndex(root: string): Promise<DocumentIndexReport> {
  return invoke<DocumentIndexReport>("start_document_index", { root, config: null });
}

export function searchDocuments(
  request: DocumentSearchRequest,
): Promise<DocumentSearchReport> {
  return invoke<DocumentSearchReport>("search_documents", { request });
}

export function getFileCatalogStatus(): Promise<FileCatalogStatus | null> {
  return invoke<FileCatalogStatus | null>("get_file_catalog_status");
}

export function getRecentFileCatalogEntries(): Promise<FileCatalogRecentReport | null> {
  return invoke<FileCatalogRecentReport | null>("get_recent_file_catalog_entries");
}

export function startFileCatalogBuild(root: string): Promise<FileCatalogReport> {
  return invoke<FileCatalogReport>("start_file_catalog_build", { root, config: null });
}

export function searchFileCatalog(
  request: FileCatalogSearchRequest,
): Promise<FileCatalogSearchReport> {
  return invoke<FileCatalogSearchReport>("search_file_catalog_entries", { request });
}

export function clearFileCatalog(): Promise<boolean> {
  return invoke<boolean>("clear_file_catalog_index");
}

export function trashDuplicateFiles(
  request: DuplicateTrashRequest,
): Promise<TrashOperationResult> {
  return invoke<TrashOperationResult>("trash_duplicate_files", { request });
}

export function trashCleanupCandidates(
  request: CleanupTrashRequest,
): Promise<TrashOperationResult> {
  return invoke<TrashOperationResult>("trash_cleanup_candidates", { request });
}

export function listenToScanProgress(
  handler: (progress: ScanProgress) => void,
): Promise<UnlistenFn> {
  return listen<ScanProgress>("scan-progress", (event) => handler(event.payload));
}

export function listenToDriveScanProgress(
  handler: (progress: DriveScanProgress) => void,
): Promise<UnlistenFn> {
  return listen<DriveScanProgress>("drive-scan-progress", (event) =>
    handler(event.payload),
  );
}

export function listenToDirectoryScanProgress(
  handler: (progress: DirectoryScanProgress) => void,
): Promise<UnlistenFn> {
  return listen<DirectoryScanProgress>("directory-scan-progress", (event) =>
    handler(event.payload),
  );
}

export function listenToCleanupScanProgress(
  handler: (progress: CleanupScanProgress) => void,
): Promise<UnlistenFn> {
  return listen<CleanupScanProgress>("cleanup-scan-progress", (event) =>
    handler(event.payload),
  );
}

export function listenToTrashProgress(
  handler: (progress: TrashProgress) => void,
): Promise<UnlistenFn> {
  return listen<TrashProgress>("trash-progress", (event) => handler(event.payload));
}

export function listenToDockerCleanupProgress(
  handler: (progress: DockerCleanupProgress) => void,
): Promise<UnlistenFn> {
  return listen<DockerCleanupProgress>("docker-cleanup-progress", (event) =>
    handler(event.payload),
  );
}

export function listenToDocumentIndexProgress(
  handler: (progress: DocumentIndexProgress) => void,
): Promise<UnlistenFn> {
  return listen<DocumentIndexProgress>("document-index-progress", (event) =>
    handler(event.payload),
  );
}

export function listenToFileCatalogProgress(
  handler: (progress: FileCatalogProgress) => void,
): Promise<UnlistenFn> {
  return listen<FileCatalogProgress>("file-catalog-progress", (event) =>
    handler(event.payload),
  );
}

export function listenToControlStatus(
  handler: (status: ControlStatus) => void,
): Promise<UnlistenFn> {
  return listen<ControlStatus>("control-status-changed", (event) =>
    handler(event.payload),
  );
}

export function listenToControlScanProgress(
  handler: (event: ControlScanProgressEvent) => void,
): Promise<UnlistenFn> {
  return listen<ControlScanProgressEvent>("control-scan-progress", (event) =>
    handler(event.payload),
  );
}

export function listenToControlScanCompleted(
  handler: (event: ControlScanCompletedEvent) => void,
): Promise<UnlistenFn> {
  return listen<ControlScanCompletedEvent>("control-scan-completed", (event) =>
    handler(event.payload),
  );
}

export async function selectDirectory(): Promise<string | null> {
  const selection = await open({
    directory: true,
    multiple: false,
    title: "스캔할 폴더 선택",
  });

  return typeof selection === "string" ? selection : null;
}

export type FileInspectionOutcome = "opened" | "revealed";

export async function inspectFile(
  path: string,
  kind: FileCatalogEntryKind = "file",
): Promise<FileInspectionOutcome> {
  if (decideFileInspection(path, kind) === "reveal") {
    await revealItemInDir(path);
    return "revealed";
  }

  await openPath(path);
  return "opened";
}

export function revealPath(path: string): Promise<void> {
  return revealItemInDir(path);
}
