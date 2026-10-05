import { X } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import "./App.css";
import { AppShell } from "./components/AppShell";
import type { ControlStatusPanelProps } from "./components/ControlStatusPanel";
import { bindSelectedScanScope } from "./lib/selectedScanScope";
import { SafetyActionDialog } from "./components/SafetyActionDialog";
import { RecoveryCheckNotice, RecoveryNotice } from "./components/RecoveryNotice";
import { FileSectionNav } from "./components/FileSectionNav";
import { StorageSectionNav } from "./components/StorageSectionNav";
import { EmptyTrashControl } from "./components/EmptyTrashControl";
import { CleanupTreePanel } from "./components/CleanupTreePanel";
import type { CleanupTreeSource, CleanupTreeView } from "./types/cleanupTree";
import { ApplicationsView } from "./views/ApplicationsView";
import { AssistantAppToolReview, appToolView, type AppToolReviewCompletion } from "./components/AssistantAppToolCard";
import {
  cancelScan,
  confirmAssistantEmptyPlan,
  confirmAssistantFilePlan,
  clearFileCatalog,
  configureControlScanAccess,
  configureControlSearchAccess,
  configureControlCleanupAccess,
  configureControlInspectionAccess,
  configureControlPermissionLifetime,
  listenToAppToolReview,
  listenToAppToolCompleted,
  getActionHistory,
  getActionRecoveryStatus,
  getControlStatus,
  getDocumentIndexStatus,
  getDockerManagementStatus,
  getFileCatalogStatus,
  getRecentFileCatalogEntries,
  getScanReportSnapshot,
  getSystemOverview,
  getPendingCleanupPlan,
  listenToCleanupScanProgress,
  listenToControlStatus,
  listenToControlScanCompleted,
  listenToControlScanProgress,
  listenToDirectoryScanProgress,
  listenToDriveScanProgress,
  listenToDocumentIndexProgress,
  listenToFileCatalogProgress,
  listenToScanProgress,
  listenToTrashProgress,
  openSystemTrash,
  revealPath,
  setDockerManagementEnabled,
  selectDirectory,
  startDirectoryScan,
  startCleanupScan,
  startDriveScan,
  startDocumentIndex,
  startFileCatalogBuild,
  startScan,
  trashCleanupCandidates,
  trashDuplicateFiles,
  trashDirectoryFile,
  prepareDirectoryFolderPlan,
  confirmDirectoryFolderPlan,
  dismissDirectoryFolderPlan,
  approveCleanupPlan,
  rejectCleanupPlan,
  openCleanupTree,
  loadCleanupTreeChildren,
  updateCleanupTreeSelection,
  prepareCleanupTreePlan,
  confirmCleanupTreePlan,
  dismissCleanupTreePlan,
  inspectFile,
} from "./lib/bridge";
import type {
  CleanupTrashRequest,
  CleanupScanProgress,
  CleanupScanReport,
  DirectoryBreadcrumb,
  DirectoryScanProgress,
  DirectoryScanReport,
  DriveScanProgress,
  DriveScanReport,
  ScanConfig,
  PermissionLifetime,
  ScanProgress,
  ScanReport,
  ScanUiState,
  SystemOverview,
  DuplicateTrashRequest,
  TrashOperationResult,
  TrashProgress,
  ViewId,
  VolumeInfo,
  ActionRecoveryReport,
  ActionHistoryReport,
  DocumentIndexProgress,
  DocumentIndexReport,
  DocumentIndexStatus,
  FileCatalogProgress,
  FileCatalogReport,
  FileCatalogRecentReport,
  FileCatalogStatus,
  ControlStatus,
  DockerManagementStatus,
  PendingCleanupPlanDetail,
  AppToolResult,
  AppToolLocalCompletion,
} from "./types";
import { DEFAULT_SCAN_CONFIG } from "./types";
import { DuplicatesView } from "./views/DuplicatesView";
import { CleanupView } from "./views/CleanupView";
import { LargeFilesView } from "./views/LargeFilesView";
import { OverviewView } from "./views/OverviewView";
import { SettingsView } from "./views/SettingsView";
import { DocumentSearchView } from "./views/DocumentSearchView";
import { FastFileSearchView } from "./views/FastFileSearchView";
import { AssistantView } from "./views/AssistantView";
import { findVolumeForPath } from "./lib/volumePath";
import { DashboardView } from "./views/DashboardView";
import { DockerManagementView } from "./views/DockerManagementView";
import { PerformanceView } from "./views/PerformanceView";
import { useLanguage } from "./i18n";

interface AssistantLaunchRequest {
  id: number;
  target: "docker";
}

const storageViews = new Set<ViewId>([
  "overview",
  "large-files",
  "duplicates",
  "cleanup",
]);

const fileViews = new Set<ViewId>(["files", "documents"]);

const unavailableControlStatus: ControlStatus = {
  revision: 0,
  bridgeAvailable: false,
  connectedClients: 0,
  lastConnectedAtUnixMs: null,
  activeOperation: null,
  lastOperation: null,
  pendingReview: null,
  lastError: null,
  protocolVersion: 3,
  searchAccess: { files: false, documents: false },
  scanAccess: { enabled: false, root: null, approvedAtUnixMs: null },
  cleanupAccess: { enabled: false, approvedAtUnixMs: null },
  inspectionAllowed: false,
};

function App() {
  const { t } = useLanguage();
  const [activeView, setActiveView] = useState<ViewId>("dashboard");
  const [mobileNavigationOpen, setMobileNavigationOpen] = useState(false);
  const [system, setSystem] = useState<SystemOverview | null>(null);
  const [root, setRoot] = useState<string | null>(null);
  const [report, setReport] = useState<ScanReport | null>(null);
  const [progress, setProgress] = useState<ScanProgress | null>(null);
  const [scanState, setScanState] = useState<ScanUiState>("idle");
  const [error, setError] = useState<string | null>(null);
  const [driveReport, setDriveReport] = useState<DriveScanReport | null>(null);
  const [driveProgress, setDriveProgress] = useState<DriveScanProgress | null>(null);
  const [driveScanState, setDriveScanState] = useState<ScanUiState>("idle");
  const [driveError, setDriveError] = useState<string | null>(null);
  const [directoryReport, setDirectoryReport] = useState<DirectoryScanReport | null>(null);
  const [directoryProgress, setDirectoryProgress] = useState<DirectoryScanProgress | null>(null);
  const [directoryScanState, setDirectoryScanState] = useState<ScanUiState>("idle");
  const [directoryError, setDirectoryError] = useState<string | null>(null);
  const [directoryBreadcrumbs, setDirectoryBreadcrumbs] = useState<DirectoryBreadcrumb[]>([]);
  const latestDirectoryGeneration = useRef(0);
  const acceptAssistantDirectoryReport = useCallback((nextReport: DirectoryScanReport, open: boolean) => {
    if (nextReport.generation < latestDirectoryGeneration.current) return;
    latestDirectoryGeneration.current = nextReport.generation;
    setDirectoryReport(nextReport);
    setDirectoryScanState("success");
    setDirectoryProgress(null);
    setDirectoryError(null);
    setDirectoryBreadcrumbs([{ name: nextReport.name, path: nextReport.root }]);
    // Sharing a result must not change the global scan scope or navigate away from the chat.
    if (open) setActiveView("overview");
  }, []);
  const [cleanupReport, setCleanupReport] = useState<CleanupScanReport | null>(null);
  const [cleanupProgress, setCleanupProgress] = useState<CleanupScanProgress | null>(null);
  const [cleanupScanState, setCleanupScanState] = useState<ScanUiState>("idle");
  const [cleanupError, setCleanupError] = useState<string | null>(null);
  const [cleanupMode, setCleanupMode] = useState<"tree" | "system">("tree");
  const [cleanupTree, setCleanupTree] = useState<CleanupTreeView | null>(null);
  const [cleanupTreeSource, setCleanupTreeSource] = useState<CleanupTreeSource | null>(null);
  const [cleanupTreeBusy, setCleanupTreeBusy] = useState(false);
  const [cleanupTreeError, setCleanupTreeError] = useState<string | null>(null);
  const cleanupTreeEpoch = useRef(0);
  const cleanupTreeInFlight = useRef(false);
  const cleanupTreeRoot = useRef<string | null>(null);
  const [documentIndex, setDocumentIndex] = useState<DocumentIndexStatus | null>(null);
  const [documentBuild, setDocumentBuild] = useState<DocumentIndexReport | null>(null);
  const [documentProgress, setDocumentProgress] = useState<DocumentIndexProgress | null>(null);
  const [documentIndexState, setDocumentIndexState] = useState<ScanUiState>("idle");
  const [documentError, setDocumentError] = useState<string | null>(null);
  const [fileCatalog, setFileCatalog] = useState<FileCatalogStatus | null>(null);
  const [fileCatalogBuild, setFileCatalogBuild] = useState<FileCatalogReport | null>(null);
  const [fileCatalogProgress, setFileCatalogProgress] = useState<FileCatalogProgress | null>(null);
  const [fileCatalogState, setFileCatalogState] = useState<ScanUiState>("idle");
  const [fileCatalogError, setFileCatalogError] = useState<string | null>(null);
  const [fileCatalogClearing, setFileCatalogClearing] = useState(false);
  const [fileCatalogStale, setFileCatalogStale] = useState(false);
  const [actionHistory, setActionHistory] = useState<ActionHistoryReport | null>(null);
  const [recentFiles, setRecentFiles] = useState<FileCatalogRecentReport | null>(null);
  const [dashboardLoading, setDashboardLoading] = useState(true);
  const [dashboardError, setDashboardError] = useState<string | null>(null);
  const [trashRunning, setTrashRunning] = useState(false);
  const [emptyTrashRunning, setEmptyTrashRunning] = useState(false);
  const [applicationRunning, setApplicationRunning] = useState(false);
  const [applicationStatus, setApplicationStatus] = useState("");
  const [applicationsVisited, setApplicationsVisited] = useState(false);
  const [trashProgress, setTrashProgress] = useState<TrashProgress | null>(null);
  const [trashResult, setTrashResult] = useState<TrashOperationResult | null>(null);
  const [trashResultSource, setTrashResultSource] = useState<"duplicates" | "cleanup" | "cleanup-tree" | "overview" | "assistant" | null>(null);
  const [trashError, setTrashError] = useState<string | null>(null);
  const [recoveryReport, setRecoveryReport] = useState<ActionRecoveryReport | null>(null);
  const [recoveryChecking, setRecoveryChecking] = useState(true);
  const [recoveryCheckSlow, setRecoveryCheckSlow] = useState(false);
  const [recoveryCheckError, setRecoveryCheckError] = useState<string | null>(null);
  const [recoveryDismissed, setRecoveryDismissed] = useState(false);
  const [recoveryErrorDismissed, setRecoveryErrorDismissed] = useState(false);
  const [openingSystemTrash, setOpeningSystemTrash] = useState(false);
  const [recoveryActionError, setRecoveryActionError] = useState<string | null>(null);
  const [controlStatus, setControlStatus] = useState<ControlStatus>(
    unavailableControlStatus,
  );
  const [controlAccessUpdating, setControlAccessUpdating] = useState(false);
  const [controlAccessError, setControlAccessError] = useState<string | null>(null);
  const [controlScanAccessUpdating, setControlScanAccessUpdating] = useState(false);
  const [controlScanAccessError, setControlScanAccessError] = useState<string | null>(null);
  const [controlCleanupAccessUpdating, setControlCleanupAccessUpdating] = useState(false);
  const [controlCleanupAccessError, setControlCleanupAccessError] = useState<string | null>(null);
  const [inspectionUpdating, setInspectionUpdating] = useState(false);
  const [inspectionError, setInspectionError] = useState<string | null>(null);
  const [permissionLifetimeUpdating, setPermissionLifetimeUpdating] = useState(false);
  const [permissionLifetimeError, setPermissionLifetimeError] = useState<string | null>(null);
  const restoredPermissionScope = useRef(false);
  const [appToolReview, setAppToolReview] = useState<{result: AppToolResult; sessionId: string | null} | null>(null);
  const appToolReviewRef = useRef<{result: AppToolResult; sessionId: string | null} | null>(null);
  const appToolReviewTrigger = useRef<HTMLElement | null>(null);
  const [appToolCompletion, setAppToolCompletion] = useState<AppToolLocalCompletion | null>(null);
  const [appToolNotice, setAppToolNotice] = useState<string | null>(null);
  const [pendingCleanupPlan, setPendingCleanupPlan] =
    useState<PendingCleanupPlanDetail | null>(null);
  const [pendingCleanupPlanLoading, setPendingCleanupPlanLoading] = useState(false);
  const [pendingCleanupPlanError, setPendingCleanupPlanError] = useState<string | null>(null);
  const [backgroundScanTerminalAnnouncement, setBackgroundScanTerminalAnnouncement] =
    useState("");
  const [backgroundScanErrorAnnouncement, setBackgroundScanErrorAnnouncement] = useState("");
  const controlStatusRevision = useRef(0);
  const controlScanOperation = useRef<string | null>(null);
  const loadedScanGeneration = useRef(0);
  const handledControlCompletion = useRef<string | null>(null);
  const announcedScanState = useRef<ScanUiState>("idle");
  const startupRecoveryPromise = useRef<Promise<ActionRecoveryReport> | null>(null);
  const [config, setConfig] = useState<ScanConfig>(DEFAULT_SCAN_CONFIG);
  const [dockerStatus, setDockerStatus] = useState<DockerManagementStatus | null>(null);
  const [dockerStatusLoading, setDockerStatusLoading] = useState(true);
  const [dockerStatusChanging, setDockerStatusChanging] = useState(false);
  const [dockerStatusError, setDockerStatusError] = useState<string | null>(null);
  const [assistantLaunchRequest, setAssistantLaunchRequest] =
    useState<AssistantLaunchRequest | null>(null);

  useEffect(() => {
    let disposed = false;
    void getDockerManagementStatus()
      .then((status) => {
        if (!disposed) setDockerStatus(status);
      })
      .catch((reason: unknown) => {
        if (!disposed) setDockerStatusError(normalizeError(reason));
      })
      .finally(() => {
        if (!disposed) setDockerStatusLoading(false);
      });
    return () => {
      disposed = true;
    };
  }, []);

  useEffect(() => {
    if (activeView === "overview") {
      announcedScanState.current = scanState;
      setBackgroundScanTerminalAnnouncement("");
      setBackgroundScanErrorAnnouncement("");
      return;
    }

    if (announcedScanState.current === scanState && scanState !== "error") return;
    announcedScanState.current = scanState;
    setBackgroundScanTerminalAnnouncement(
      scanState === "success"
        ? t("파일 검사가 완료됐습니다.")
        : scanState === "cancelled"
          ? t("파일 검사를 취소했습니다.")
          : "",
    );
    setBackgroundScanErrorAnnouncement(
      scanState === "error"
        ? t("파일 검사를 완료하지 못했습니다. {{detail}}", {
            detail: error ?? t("오류 내용을 확인해 주세요."),
          })
        : "",
    );
  }, [activeView, error, scanState]);

  async function loadControlScanResult(
    operationId: string,
    scanGeneration: number,
  ) {
    if (scanGeneration <= loadedScanGeneration.current) return;
    try {
      const snapshot = await getScanReportSnapshot(scanGeneration);
      if (
        snapshot.scanGeneration !== scanGeneration ||
        scanGeneration <= loadedScanGeneration.current
      )
        return;
      loadedScanGeneration.current = scanGeneration;
      controlScanOperation.current = null;
      setRoot((current) => current ?? displayPath(snapshot.report.root));
      setReport(snapshot.report);
      setScanState("success");
      setProgress(null);
      setError(null);
      setTrashResult(null);
      setTrashResultSource(null);
      setTrashError(null);
      void getSystemOverview()
        .then(setSystem)
        .catch((reason: unknown) => {
          console.warn("디스크 사용량을 새로 고치지 못했습니다.", reason);
        });
    } catch (reason) {
      if (handledControlCompletion.current !== operationId) return;
      controlScanOperation.current = null;
      setScanState("error");
      setProgress(null);
      setError(normalizeError(reason));
    }
  }

  function handleControlScanCompletion(
    operationId: string,
    state: "completed" | "failed" | "cancelled",
    scanGeneration: number | null,
    message: string,
  ) {
    if (handledControlCompletion.current === operationId) return;
    handledControlCompletion.current = operationId;
    if (state === "completed" && scanGeneration !== null) {
      void loadControlScanResult(operationId, scanGeneration);
      return;
    }
    controlScanOperation.current = null;
    setProgress(null);
    if (state === "cancelled") {
      setScanState("cancelled");
      setError(null);
    } else {
      setScanState("error");
      setError(message);
    }
  }

  function applyControlStatus(status: ControlStatus) {
    if (status.revision < controlStatusRevision.current) return;
    controlStatusRevision.current = status.revision;
    setControlStatus(status);

    const operation = status.activeOperation;
    if (
      operation?.source === "chatCli" &&
      operation.kind === "storageScan" &&
      (operation.state === "queued" || operation.state === "running")
    ) {
      if (controlScanOperation.current !== operation.operationId) {
        controlScanOperation.current = operation.operationId;
        handledControlCompletion.current = null;
        if (status.scanAccess.root)
          setRoot((current) => current ?? displayPath(status.scanAccess.root ?? ""));
        setReport(null);
        setTrashResult(null);
        setTrashResultSource(null);
        setTrashError(null);
        setScanState("scanning");
        setProgress({
          phase: "discovering",
          message: t("채팅에서 요청한 검사를 준비하고 있습니다"),
          processedFiles: operation.processedItems ?? 0,
          processedBytes: operation.processedBytes ?? 0,
          fraction: null,
        });
        setError(null);
      } else {
        setProgress((current) => ({
          phase: current?.phase ?? "discovering",
          message: t("채팅에서 요청한 검사를 진행하고 있습니다"),
          processedFiles: operation.processedItems ?? current?.processedFiles ?? 0,
          processedBytes: operation.processedBytes ?? current?.processedBytes ?? 0,
          fraction: current?.fraction ?? null,
        }));
      }
      return;
    }

    const completed = status.lastOperation;
    if (
      completed?.source === "chatCli" &&
      completed.kind === "storageScan" &&
      (completed.state === "completed" ||
        completed.state === "failed" ||
        completed.state === "cancelled")
    ) {
      handleControlScanCompletion(
        completed.operationId,
        completed.state,
        completed.scanGeneration,
        t("채팅에서 요청한 검사를 완료하지 못했습니다"),
      );
    }
  }

  function acceptAppToolPresentation(presentation: Record<string, unknown> | null | undefined) {
    if (!presentation) return;
    if (presentation.kind === "indexStatus") {
      if (presentation.source === "files") {
        setFileCatalog((presentation.index as FileCatalogStatus | null) ?? null);
        setFileCatalogState(presentation.index ? "success" : "idle");
        setFileCatalogStale(false);
      } else if (presentation.source === "documents") {
        setDocumentIndex((presentation.index as DocumentIndexStatus | null) ?? null);
        setDocumentIndexState(presentation.index ? "success" : "idle");
      }
    } else if (presentation.kind === "cleanupScan" && presentation.result) {
      setCleanupReport(presentation.result as CleanupScanReport);
      setCleanupScanState("success"); setCleanupProgress(null); setCleanupError(null);
    } else if (presentation.overview) setSystem(presentation.overview as SystemOverview);
  }

  function openAppToolReview(result: AppToolResult, sessionId: string | null = null) {
    if (result.status !== "review_required" || !result.presentation) return;
    if (result.presentation.kind === "cleanupReview") { void openPendingCleanupReview(); return; }
    if (appToolReviewRef.current) return;
    appToolReviewTrigger.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const review = { result, sessionId };
    appToolReviewRef.current = review; setAppToolReview(review); setAppToolNotice(null);
  }
  function closeAppToolReview() {
    appToolReviewRef.current = null; setAppToolReview(null);
    window.requestAnimationFrame(() => { if (appToolReviewTrigger.current?.isConnected) appToolReviewTrigger.current.focus({ preventScroll: true }); });
  }
  function completedAppToolReview(completion: AppToolReviewCompletion) {
    setAppToolNotice(completion.message);
    setAppToolCompletion({ sequence: Date.now(), sessionId: appToolReviewRef.current?.sessionId ?? null, message: completion.message, trashResult: completion.trashResult });
    if (completion.dockerStatus) setDockerStatus(completion.dockerStatus);
    if (completion.mutated) {
      invalidateAnalysisReports(); setFileCatalogStale(true);
      if (completion.trashResult) { setTrashResult(completion.trashResult); setTrashResultSource("assistant"); }
      void getSystemOverview().then(setSystem).catch(() => {});
      void getActionHistory().then(setActionHistory).catch(() => {});
    }
  }
  function showAppToolView(result: AppToolResult) {
    acceptAppToolPresentation(result.presentation);
    const view = appToolView(result);
    if (view === "cleanup") setCleanupMode("system");
    if (view) navigate(view);
  }

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    let unlistenDrive: (() => void) | undefined;
    let unlistenDirectory: (() => void) | undefined;
    let unlistenCleanup: (() => void) | undefined;
    let unlistenTrash: (() => void) | undefined;
    let unlistenDocuments: (() => void) | undefined;
    let unlistenFileCatalog: (() => void) | undefined;
    let unlistenControlStatus: (() => void) | undefined;
    let unlistenControlScanProgress: (() => void) | undefined;
    let unlistenControlScanCompleted: (() => void) | undefined;
    const recoverySlowTimer = window.setTimeout(() => {
      if (!disposed) setRecoveryCheckSlow(true);
    }, 300);

    const recoveryPromise =
      startupRecoveryPromise.current ?? getActionRecoveryStatus();
    startupRecoveryPromise.current = recoveryPromise;
    recoveryPromise
      .then((nextReport) => {
        if (disposed) return;
        setRecoveryReport(nextReport);
        setRecoveryCheckError(null);
      })
      .catch((reason: unknown) => {
        if (!disposed) setRecoveryCheckError(normalizeError(reason));
      })
      .finally(() => {
        if (disposed) return;
        window.clearTimeout(recoverySlowTimer);
        setRecoveryCheckSlow(false);
        setRecoveryChecking(false);
      });

    Promise.allSettled([
      getSystemOverview(),
      getActionHistory(),
      getRecentFileCatalogEntries(),
    ]).then(([systemResult, historyResult, recentResult]) => {
      if (disposed) return;

      const failures: string[] = [];
      if (systemResult.status === "fulfilled") {
        setSystem(systemResult.value);
      } else {
        failures.push(t("드라이브: {{detail}}", { detail: normalizeError(systemResult.reason) }));
      }
      if (historyResult.status === "fulfilled") {
        setActionHistory(historyResult.value);
      } else {
        failures.push(t("최근 정리: {{detail}}", { detail: normalizeError(historyResult.reason) }));
      }
      if (recentResult.status === "fulfilled") {
        setRecentFiles(recentResult.value);
      } else {
        failures.push(t("최근 파일: {{detail}}", { detail: normalizeError(recentResult.reason) }));
      }

      setDashboardError(failures.length > 0 ? failures.join(" · ") : null);
      setDashboardLoading(false);
    });

    getDocumentIndexStatus()
      .then((status) => {
        if (disposed || !status) return;
        setDocumentIndex(status);
        setDocumentIndexState("success");
      })
      .catch((reason: unknown) => {
        if (!disposed) setDocumentError(normalizeError(reason));
      });

    getFileCatalogStatus()
      .then((status) => {
        if (disposed || !status) return;
        setFileCatalog(status);
        setFileCatalogState("success");
      })
      .catch((reason: unknown) => {
        if (!disposed) setFileCatalogError(normalizeError(reason));
      });

    listenToScanProgress((nextProgress) => {
      if (!disposed) setProgress(nextProgress);
    })
      .then((cleanup) => {
        if (disposed) cleanup();
        else unlisten = cleanup;
      })
      .catch((reason: unknown) => {
        if (!disposed) setError(normalizeError(reason));
      });

    listenToDriveScanProgress((nextProgress) => {
      if (!disposed) setDriveProgress(nextProgress);
    })
      .then((cleanup) => {
        if (disposed) cleanup();
        else unlistenDrive = cleanup;
      })
      .catch((reason: unknown) => {
        if (!disposed) setDriveError(normalizeError(reason));
      });

    listenToDirectoryScanProgress((nextProgress) => {
      if (!disposed) setDirectoryProgress(nextProgress);
    })
      .then((cleanup) => {
        if (disposed) cleanup();
        else unlistenDirectory = cleanup;
      })
      .catch((reason: unknown) => {
        if (!disposed) setDirectoryError(normalizeError(reason));
      });

    listenToCleanupScanProgress((nextProgress) => {
      if (!disposed) setCleanupProgress(nextProgress);
    })
      .then((cleanup) => {
        if (disposed) cleanup();
        else unlistenCleanup = cleanup;
      })
      .catch((reason: unknown) => {
        if (!disposed) setCleanupError(normalizeError(reason));
      });

    listenToTrashProgress((nextProgress) => {
      if (!disposed) setTrashProgress(nextProgress);
    })
      .then((cleanup) => {
        if (disposed) cleanup();
        else unlistenTrash = cleanup;
      })
      .catch((reason: unknown) => {
        if (!disposed) setTrashError(normalizeError(reason));
      });

    listenToDocumentIndexProgress((nextProgress) => {
      if (!disposed) setDocumentProgress(nextProgress);
    })
      .then((cleanup) => {
        if (disposed) cleanup();
        else unlistenDocuments = cleanup;
      })
      .catch((reason: unknown) => {
        if (!disposed) setDocumentError(normalizeError(reason));
      });

    listenToFileCatalogProgress((nextProgress) => {
      if (!disposed) setFileCatalogProgress(nextProgress);
    })
      .then((cleanup) => {
        if (disposed) cleanup();
        else unlistenFileCatalog = cleanup;
      })
      .catch((reason: unknown) => {
        if (!disposed) setFileCatalogError(normalizeError(reason));
      });

    async function connectControlBridge() {
      try {
        const cleanup = await listenToControlStatus((status) => {
          if (!disposed) applyControlStatus(status);
        });
        if (disposed) {
          cleanup();
          return;
        }
        unlistenControlStatus = cleanup;
      } catch {
        if (!disposed && controlStatusRevision.current === 0)
          setControlStatus(unavailableControlStatus);
      }

      try {
        const cleanup = await listenToControlScanProgress((event) => {
          if (
            disposed ||
            event.revision < controlStatusRevision.current ||
            controlScanOperation.current !== event.operationId
          )
            return;
          controlStatusRevision.current = event.revision;
          setProgress(event.progress);
        });
        if (disposed) {
          cleanup();
          return;
        }
        unlistenControlScanProgress = cleanup;
      } catch (reason) {
        if (!disposed) setError(normalizeError(reason));
      }

      try {
        const cleanup = await listenToControlScanCompleted((event) => {
          if (disposed || event.revision < controlStatusRevision.current) return;
          if (event.kind && event.kind !== "storageScan") return;
          controlStatusRevision.current = event.revision;
          handleControlScanCompletion(
            event.operationId,
            event.state,
            event.scanGeneration,
            event.message,
          );
        });
        if (disposed) {
          cleanup();
          return;
        }
        unlistenControlScanCompleted = cleanup;
      } catch (reason) {
        if (!disposed) setError(normalizeError(reason));
      }

      try {
        const status = await getControlStatus();
        if (!disposed) {
          applyControlStatus(status);
          if (!restoredPermissionScope.current) {
            restoredPermissionScope.current = true;
            if (status.permissionLifetime === "remember" && status.scanAccess.enabled && status.scanAccess.root && status.scanAccess.config) {
              const approvedRoot = status.scanAccess.root;
              const approvedConfig = status.scanAccess.config;
              setRoot(current => current ?? approvedRoot);
              setConfig(current => current === DEFAULT_SCAN_CONFIG ? approvedConfig : current);
            }
          }
        }
      } catch {
        if (!disposed && controlStatusRevision.current === 0)
          setControlStatus(unavailableControlStatus);
      }
    }

    void connectControlBridge();

    return () => {
      disposed = true;
      window.clearTimeout(recoverySlowTimer);
      unlisten?.();
      unlistenDrive?.();
      unlistenDirectory?.();
      unlistenCleanup?.();
      unlistenTrash?.();
      unlistenDocuments?.();
      unlistenFileCatalog?.();
      unlistenControlStatus?.();
      unlistenControlScanProgress?.();
      unlistenControlScanCompleted?.();
    };
  }, []);

  useEffect(() => {
    let disposed = false;
    const listeners: (() => void)[] = [];
    void listenToAppToolReview((result) => { if (!disposed) openAppToolReview(result); }).then((stop) => disposed ? stop() : listeners.push(stop)).catch(() => {});
    void listenToAppToolCompleted((event) => { if (!disposed) acceptAppToolPresentation(event.presentation); }).then((stop) => disposed ? stop() : listeners.push(stop)).catch(() => {});
    return () => { disposed = true; listeners.forEach((stop) => stop()); };
  }, []);

  async function refreshDashboard() {
    if (dashboardLoading || selectionBlocked) return;
    setDashboardLoading(true);
    setDashboardError(null);

    const [systemResult, historyResult, recentResult] = await Promise.allSettled([
      getSystemOverview(),
      getActionHistory(),
      getRecentFileCatalogEntries(),
    ]);
    const failures: string[] = [];

    if (systemResult.status === "fulfilled") {
      setSystem(systemResult.value);
    } else {
      failures.push(t("드라이브: {{detail}}", { detail: normalizeError(systemResult.reason) }));
    }
    if (historyResult.status === "fulfilled") {
      setActionHistory(historyResult.value);
    } else {
      failures.push(t("최근 정리: {{detail}}", { detail: normalizeError(historyResult.reason) }));
    }
    if (recentResult.status === "fulfilled") {
      setRecentFiles(recentResult.value);
    } else {
      failures.push(t("최근 파일: {{detail}}", { detail: normalizeError(recentResult.reason) }));
    }

    setDashboardError(failures.length > 0 ? failures.join(" · ") : null);
    setDashboardLoading(false);
  }

  async function retryActionRecovery() {
    if (recoveryChecking) return;
    setRecoveryChecking(true);
    setRecoveryCheckSlow(false);
    setRecoveryCheckError(null);
    setRecoveryReport(null);
    setRecoveryErrorDismissed(false);
    setRecoveryDismissed(false);
    const slowTimer = window.setTimeout(() => setRecoveryCheckSlow(true), 300);
    try {
      const nextReport = await getActionRecoveryStatus();
      setRecoveryReport(nextReport);
    } catch (reason) {
      setRecoveryCheckError(normalizeError(reason));
    } finally {
      window.clearTimeout(slowTimer);
      setRecoveryCheckSlow(false);
      setRecoveryChecking(false);
    }
  }

  async function showSystemTrash() {
    if (openingSystemTrash) return;
    setOpeningSystemTrash(true);
    setRecoveryActionError(null);
    try {
      await openSystemTrash();
    } catch (reason) {
      setRecoveryActionError(normalizeError(reason));
    } finally {
      setOpeningSystemTrash(false);
    }
  }

  async function toggleControlSearchAccess() {
    if (permissionLifetimeUpdating) return;
    if (controlAccessUpdating || !controlStatus.bridgeAvailable) return;
    const enabled =
      controlStatus.searchAccess.files || controlStatus.searchAccess.documents;
    if (!enabled && !fileCatalog && !documentIndex) return;

    setControlAccessUpdating(true);
    setControlAccessError(null);
    try {
      const status = await configureControlSearchAccess({
        fileRoot: enabled ? null : (fileCatalog?.root ?? null),
        documentRoot: enabled ? null : (documentIndex?.root ?? null),
      });
      applyControlStatus(status);
    } catch (reason) {
      setControlAccessError(normalizeError(reason));
    } finally {
      setControlAccessUpdating(false);
    }
  }

  async function updateControlScanAccess(
    nextRoot: string | null,
    nextConfig: ScanConfig | null,
  ): Promise<boolean> {
    if (controlScanAccessUpdating || !controlStatus.bridgeAvailable) return false;
    setControlScanAccessUpdating(true);
    setControlScanAccessError(null);
    try {
      const status = await configureControlScanAccess({
        root: nextRoot,
        config: nextConfig,
      });
      applyControlStatus(status);
      return true;
    } catch (reason) {
      setControlScanAccessError(normalizeError(reason));
      return false;
    } finally {
      setControlScanAccessUpdating(false);
    }
  }

  async function toggleControlCleanupAccess() {
    if (permissionLifetimeUpdating) return;
    if (controlCleanupAccessUpdating || !controlStatus.bridgeAvailable) return;
    const enabled = controlStatus.cleanupAccess.enabled;
    if (selectionBlocked) return;

    setControlCleanupAccessUpdating(true);
    setControlCleanupAccessError(null);
    try {
      const status = await configureControlCleanupAccess({ enabled: !enabled });
      applyControlStatus(status);
      if (enabled) {
        setPendingCleanupPlan(null);
        setPendingCleanupPlanError(null);
      }
    } catch (reason) {
      setControlCleanupAccessError(normalizeError(reason));
    } finally {
      setControlCleanupAccessUpdating(false);
    }
  }

  async function toggleInspectionAccess() {
    if (permissionLifetimeUpdating) return;
    if (inspectionUpdating || !controlStatus.bridgeAvailable) return;
    setInspectionUpdating(true); setInspectionError(null);
    try { applyControlStatus(await configureControlInspectionAccess(controlStatus.inspectionAllowed !== true)); }
    catch (reason) { setInspectionError(normalizeError(reason)); }
    finally { setInspectionUpdating(false); }
  }

  async function changePermissionLifetime(lifetime: PermissionLifetime) {
    if (permissionLifetimeUpdating || selectionBlocked || controlAccessUpdating || controlScanAccessUpdating || controlCleanupAccessUpdating || inspectionUpdating || !controlStatus.bridgeAvailable) return;
    setPermissionLifetimeUpdating(true);
    setPermissionLifetimeError(null);
    try {
      applyControlStatus(await configureControlPermissionLifetime(lifetime));
    } catch (reason) {
      setPermissionLifetimeError(normalizeError(reason));
    } finally {
      setPermissionLifetimeUpdating(false);
    }
  }

  async function openPendingCleanupReview() {
    if (pendingCleanupPlanLoading || trashRunning) return;
    setPendingCleanupPlanLoading(true);
    setPendingCleanupPlanError(null);
    try {
      const plan = await getPendingCleanupPlan();
      if (!plan) {
        setPendingCleanupPlan(null);
        setPendingCleanupPlanError(t("확인할 정리 계획이 없거나 확인 시간이 지났습니다."));
        return;
      }
      setPendingCleanupPlan(plan);
    } catch (reason) {
      setPendingCleanupPlanError(normalizeError(reason));
    } finally {
      setPendingCleanupPlanLoading(false);
    }
  }

  async function closePendingCleanupReview() {
    const plan = pendingCleanupPlan;
    if (!plan || trashRunning) return;
    setPendingCleanupPlanError(null);
    try {
      await rejectCleanupPlan(plan.planId);
      setPendingCleanupPlan(null);
    } catch (reason) {
      setPendingCleanupPlanError(normalizeError(reason));
    }
  }

  async function confirmPendingCleanupReview(reviewAcknowledged: boolean) {
    const plan = pendingCleanupPlan;
    if (!plan || trashRunning) return;
    setPendingCleanupPlanError(null);
    try {
      await runTrashAction(
        plan.source === "duplicateFiles" ? "duplicates" : "cleanup",
        () =>
          approveCleanupPlan({
            planId: plan.planId,
            allowReviewCandidates: reviewAcknowledged,
          }),
      );
      setPendingCleanupPlan(null);
    } catch (reason) {
      setPendingCleanupPlanError(normalizeError(reason));
    }
  }

  async function updateScanConfig(nextConfig: ScanConfig) {
    if (root && controlStatus.bridgeAvailable) {
      const updated = await updateControlScanAccess(root, nextConfig);
      if (!updated) return;
    }
    setConfig(nextConfig);
  }

  const volume = useMemo(
    () => findVolumeForPath(system?.volumes ?? [], root ?? report?.root ?? null),
    [report?.root, root, system?.volumes],
  );
  const selectionBlocked =
    cleanupTreeBusy ||
    recoveryChecking ||
    applicationRunning ||
    emptyTrashRunning ||
    trashRunning ||
    scanState === "scanning" ||
    driveScanState === "scanning" ||
    directoryScanState === "scanning" ||
    cleanupScanState === "scanning" ||
    documentIndexState === "scanning" ||
    fileCatalogState === "scanning" ||
    fileCatalogClearing;

  function navigate(view: ViewId) {
    if (view === "applications") setApplicationsVisited(true);
    const reducedMotion = window.matchMedia?.(
      "(prefers-reduced-motion: reduce)",
    ).matches;
    if (reducedMotion || !document.startViewTransition) {
      setActiveView(view);
      return;
    }
    document.startViewTransition(() => setActiveView(view));
  }

  async function refreshDockerStatus() {
    if (dockerStatusLoading || dockerStatus?.busy) return;
    setDockerStatusLoading(true);
    setDockerStatusError(null);
    try {
      setDockerStatus(await getDockerManagementStatus());
    } catch (reason) {
      setDockerStatusError(normalizeError(reason));
    } finally {
      setDockerStatusLoading(false);
    }
  }

  async function updateDockerEnabled(enabled: boolean) {
    if (dockerStatusChanging || dockerStatus?.busy) return;
    setDockerStatusChanging(true);
    setDockerStatusError(null);
    try {
      setDockerStatus(await setDockerManagementEnabled(enabled));
    } catch (reason) {
      setDockerStatusError(normalizeError(reason));
    } finally {
      setDockerStatusChanging(false);
    }
  }

  function openDockerConversation() {
    setAssistantLaunchRequest({ id: Date.now(), target: "docker" });
    navigate("assistant");
  }

  async function useSelectedRoot(selected: string): Promise<boolean> {
    if (!await bindSelectedScanScope(root, selected, config,
      controlStatus.scanAccess.enabled, controlStatus.bridgeAvailable, updateControlScanAccess)) return false;
    if (selected === root) return true;

    setRoot(selected);
    cleanupTreeEpoch.current += 1;
    setCleanupTree(null);
    setCleanupTreeSource(null);
    cleanupTreeRoot.current = null;
    setCleanupTreeError(null);
    setReport(null);
    setProgress(null);
    setScanState("idle");
    setError(null);
    setDriveReport(null);
    setDriveProgress(null);
    setDriveScanState("idle");
    setDriveError(null);
    setDirectoryReport(null);
    setDirectoryProgress(null);
    setDirectoryScanState("idle");
    setDirectoryError(null);
    setDirectoryBreadcrumbs([]);
    setTrashResult(null);
    setTrashResultSource(null);
    setTrashError(null);
    return true;
  }

  async function pickFolder(onError?: (message: string) => void): Promise<string | null> {
    if (selectionBlocked) return null;
    try {
      const selected = await selectDirectory(t("스캔할 폴더 선택"));
      if (!selected || !(await useSelectedRoot(selected))) return null;
      return selected;
    } catch (reason) {
      setError(normalizeError(reason));
      onError?.(normalizeError(reason));
      setScanState("error");
      return null;
    }
  }

  async function pickStorageFolder(
    options: { stayOnView?: boolean; onError?: (message: string) => void } = {},
  ): Promise<DirectoryScanReport | null> {
    const selected = await pickFolder(options.onError);
    if (!selected) return null;
    return runDirectoryScan(selected, undefined, options);
  }

  async function openCandidateTree(source: CleanupTreeSource): Promise<CleanupTreeView | null> {
    if (selectionBlocked || cleanupTreeInFlight.current) return null;
    cleanupTreeInFlight.current = true;
    const epoch = ++cleanupTreeEpoch.current;
    const sameSource = cleanupTreeSource?.kind === source.kind && (
      source.kind === "directory"
        ? cleanupTreeSource.kind === "directory" && cleanupTreeSource.generation === source.generation
        : cleanupTreeSource.kind === "assistant" && cleanupTreeSource.sessionId === source.sessionId && cleanupTreeSource.revision === source.revision
    );
    if (!sameSource) cleanupTreeRoot.current = null;
    setCleanupMode("tree");
    setCleanupTreeSource(source);
    setCleanupTree(null);
    setCleanupTreeError(null);
    setCleanupTreeBusy(true);
    setTrashResult(null);
    setTrashError(null);
    navigate("cleanup");
    try {
      const next = await openCleanupTree(source);
      if (epoch !== cleanupTreeEpoch.current) return null;
      cleanupTreeRoot.current = next.rootPath;
      setCleanupTree(next);
      return next;
    } catch (reason) {
      if (epoch === cleanupTreeEpoch.current) setCleanupTreeError(normalizeError(reason));
      return null;
    } finally {
      cleanupTreeInFlight.current = false;
      setCleanupTreeBusy(false);
    }
  }

  async function changeCandidateTree(action: (tree: CleanupTreeView) => Promise<CleanupTreeView>) {
    if (!cleanupTree || selectionBlocked || cleanupTreeInFlight.current) return;
    cleanupTreeInFlight.current = true;
    const epoch = cleanupTreeEpoch.current;
    const treeId = cleanupTree.treeId;
    setCleanupTreeBusy(true);
    setCleanupTreeError(null);
    try {
      const next = await action(cleanupTree);
      if (epoch === cleanupTreeEpoch.current && next.treeId === treeId) setCleanupTree(next);
    } catch (reason) {
      if (epoch === cleanupTreeEpoch.current) setCleanupTreeError(normalizeError(reason));
      throw reason;
    } finally {
      cleanupTreeInFlight.current = false;
      setCleanupTreeBusy(false);
    }
  }

  async function inspectCandidateFolder() {
    if (selectionBlocked || cleanupTreeInFlight.current) return;
    setCleanupTreeError(null);
    const next = await pickStorageFolder({ stayOnView: true, onError: setCleanupTreeError });
    if (next) await openCandidateTree({ kind: "directory", generation: next.generation });
  }

  async function refreshCandidateTree() {
    if (selectionBlocked || cleanupTreeInFlight.current) return;
    setCleanupTreeError(null);
    if (cleanupTreeSource?.kind === "assistant") {
      // Trash execution invalidates the assistant revision. Refresh the same
      // locally reviewed folder rather than replaying an obsolete session ID.
      if (cleanupTreeRoot.current) {
        const next = await runDirectoryScan(cleanupTreeRoot.current, undefined, { stayOnView: true, onError: setCleanupTreeError });
        if (next) await openCandidateTree({ kind: "directory", generation: next.generation });
      } else {
        await openCandidateTree(cleanupTreeSource);
      }
    } else if (directoryReport) {
      await openCandidateTree({ kind: "directory", generation: directoryReport.generation });
    } else if (cleanupTreeRoot.current) {
      const next = await runDirectoryScan(cleanupTreeRoot.current, undefined, { stayOnView: true, onError: setCleanupTreeError });
      if (next) await openCandidateTree({ kind: "directory", generation: next.generation });
    } else {
      await inspectCandidateFolder();
    }
  }

  async function moveCandidateTree(nestedContentsAcknowledged: boolean) {
    if (!cleanupTree?.plan || selectionBlocked || cleanupTreeInFlight.current) return;
    const { treeId, selectionRevision, plan } = cleanupTree;
    cleanupTreeInFlight.current = true;
    setCleanupTreeBusy(true);
    setCleanupTreeError(null);
    try {
      return await runTrashAction("cleanup-tree", () => confirmCleanupTreePlan(treeId, selectionRevision, plan.id, nestedContentsAcknowledged));
    } finally {
      cleanupTreeInFlight.current = false;
      setCleanupTreeBusy(false);
    }
  }

  useEffect(() => {
    if (activeView === "cleanup" && cleanupMode === "tree") return;
    if (!cleanupTree?.plan || cleanupTreeBusy || trashRunning || cleanupTreeInFlight.current) return;
    // Leaving review never preserves an executable approval for later navigation.
    void changeCandidateTree((tree) => dismissCleanupTreePlan(tree.treeId, tree.plan!.id)).catch(() => {});
  }, [activeView, cleanupMode, cleanupTree?.plan?.id, cleanupTreeBusy, trashRunning]);

  async function runDashboardScan(nextVolume: VolumeInfo) {
    if (selectionBlocked || !(await useSelectedRoot(nextVolume.mountPoint))) return;
    await runScan({
      stayOnView: true,
      rootOverride: nextVolume.mountPoint,
    });
  }

  async function revealDashboardFile(path: string) {
    try {
      await revealPath(path);
    } catch (reason) {
      setDashboardError(
        t("파일 위치를 열지 못했습니다. {{detail}}", { detail: normalizeError(reason) }),
      );
    }
  }

  async function runScan(
    options: { stayOnView?: boolean; rootOverride?: string } = {},
  ) {
    const scanRoot = options.rootOverride ?? root ?? (await pickFolder());
    if (
      !scanRoot ||
      scanState === "scanning" ||
      driveScanState === "scanning" ||
      directoryScanState === "scanning" ||
      cleanupScanState === "scanning" ||
      documentIndexState === "scanning" ||
      fileCatalogState === "scanning" ||
      trashRunning ||
      recoveryChecking
    )
      return;

    if (!options.stayOnView) setActiveView("overview");
    setTrashResult(null);
    setTrashResultSource(null);
    setTrashError(null);
    setReport(null);
    setScanState("scanning");
    setError(null);
    setProgress({
      phase: "discovering",
      message: t("스캔 작업을 준비하고 있습니다"),
      processedFiles: 0,
      processedBytes: 0,
      fraction: null,
    });

    try {
      const nextReport = await startScan(scanRoot, config);
      setReport(nextReport);
      setScanState("success");
      setProgress(null);
      void getSystemOverview()
        .then(setSystem)
        .catch((reason: unknown) => {
          console.warn("디스크 사용량을 새로 고치지 못했습니다.", reason);
        });
    } catch (reason) {
      const message = normalizeError(reason);
      if (message.toLocaleLowerCase("en-US").includes("cancel")) {
        setScanState("cancelled");
        setError(null);
      } else {
        setScanState("error");
        setError(message);
      }
      setProgress(null);
    }
  }

  async function runDriveScan() {
    const driveRoot = volume?.mountPoint;
    if (
      !driveRoot ||
      driveScanState === "scanning" ||
      scanState === "scanning" ||
      directoryScanState === "scanning" ||
      cleanupScanState === "scanning" ||
      documentIndexState === "scanning" ||
      fileCatalogState === "scanning" ||
      trashRunning ||
      recoveryChecking
    )
      return;

    setActiveView("overview");
    setDriveScanState("scanning");
    setDriveError(null);
    setDriveProgress({
      phase: "discovering",
      message: t("드라이브 분석을 준비하고 있습니다"),
      processedFiles: 0,
      processedBytes: 0,
      unreadableEntries: 0,
      categories: [],
    });

    try {
      const nextReport = await startDriveScan(driveRoot);
      setDriveReport(nextReport);
      setDriveScanState("success");
      setDriveProgress(null);
      void getSystemOverview()
        .then(setSystem)
        .catch((reason: unknown) => {
          console.warn("디스크 사용량을 새로 고치지 못했습니다.", reason);
        });
    } catch (reason) {
      const message = normalizeError(reason);
      if (message.toLocaleLowerCase("en-US").includes("cancel")) {
        setDriveScanState("cancelled");
        setDriveError(null);
      } else {
        setDriveScanState("error");
        setDriveError(message);
      }
      setDriveProgress(null);
    }
  }

  async function stopScan() {
    try {
      const cancellationRequested = await cancelScan();
      if (cancellationRequested) {
        setProgress((current) => ({
          phase: current?.phase ?? "discovering",
          message: t("안전하게 스캔을 중단하고 있습니다"),
          processedFiles: current?.processedFiles ?? 0,
          processedBytes: current?.processedBytes ?? 0,
          fraction: current?.fraction ?? null,
        }));
      }
    } catch (reason) {
      setError(normalizeError(reason));
    }
  }

  async function stopDriveScan() {
    try {
      const cancellationRequested = await cancelScan();
      if (cancellationRequested) {
        setDriveProgress((current) => ({
          phase: current?.phase ?? "discovering",
          message: t("안전하게 드라이브 분석을 중단하고 있습니다"),
          processedFiles: current?.processedFiles ?? 0,
          processedBytes: current?.processedBytes ?? 0,
          unreadableEntries: current?.unreadableEntries ?? 0,
          categories: current?.categories ?? [],
        }));
      }
    } catch (reason) {
      setDriveError(normalizeError(reason));
    }
  }

  async function runDirectoryScan(
    scanRoot: string,
    nextBreadcrumbs?: DirectoryBreadcrumb[],
    options: { stayOnView?: boolean; onError?: (message: string) => void } = {},
  ): Promise<DirectoryScanReport | null> {
    if (
      !scanRoot ||
      directoryScanState === "scanning" ||
      driveScanState === "scanning" ||
      scanState === "scanning" ||
      cleanupScanState === "scanning" ||
      documentIndexState === "scanning" ||
      fileCatalogState === "scanning" ||
      trashRunning ||
      recoveryChecking
    )
      return null;

    if (!options.stayOnView) setActiveView("overview");
    setDirectoryScanState("scanning");
    setDirectoryError(null);
    setDirectoryProgress({
      message: t("폴더 구조 분석을 준비하고 있습니다"),
      processedEntries: 0,
      processedFiles: 0,
      processedBytes: 0,
      unreadableEntries: 0,
    });

    try {
      const nextReport = await startDirectoryScan(scanRoot);
      latestDirectoryGeneration.current = nextReport.generation;
      setDirectoryReport(nextReport);
      setDirectoryScanState("success");
      setDirectoryProgress(null);
      const breadcrumbs = nextBreadcrumbs?.length
        ? [...nextBreadcrumbs]
        : [{ name: nextReport.name, path: nextReport.root }];
      const lastIndex = breadcrumbs.length - 1;
      breadcrumbs[lastIndex] = {
        name: breadcrumbs[lastIndex]?.name || nextReport.name,
        path: nextReport.root,
      };
      setDirectoryBreadcrumbs(breadcrumbs);
      return nextReport;
    } catch (reason) {
      const message = normalizeError(reason);
      options.onError?.(message);
      if (message.toLocaleLowerCase("en-US").includes("cancel")) {
        setDirectoryScanState("cancelled");
        setDirectoryError(null);
      } else {
        setDirectoryScanState("error");
        setDirectoryError(message);
      }
      setDirectoryProgress(null);
      return null;
    }
  }

  async function stopDirectoryScan() {
    try {
      const cancellationRequested = await cancelScan();
      if (cancellationRequested) {
        setDirectoryProgress((current) => ({
          message: t("안전하게 폴더 지도 분석을 중단하고 있습니다"),
          processedEntries: current?.processedEntries ?? 0,
          processedFiles: current?.processedFiles ?? 0,
          processedBytes: current?.processedBytes ?? 0,
          unreadableEntries: current?.unreadableEntries ?? 0,
        }));
      }
    } catch (reason) {
      setDirectoryError(normalizeError(reason));
    }
  }

  async function runCleanupScan(
    options: { stayOnView?: boolean } = {},
  ) {
    if (
      cleanupScanState === "scanning" ||
      scanState === "scanning" ||
      driveScanState === "scanning" ||
      directoryScanState === "scanning" ||
      documentIndexState === "scanning" ||
      fileCatalogState === "scanning" ||
      trashRunning ||
      recoveryChecking
    )
      return;

    if (!options.stayOnView) setActiveView("cleanup");
    if (trashResultSource === "cleanup") {
      setTrashResult(null);
      setTrashResultSource(null);
      setTrashError(null);
    }
    setCleanupReport(null);
    setCleanupScanState("scanning");
    setCleanupError(null);
    setCleanupProgress({
      message: t("정리 후보 위치를 준비하고 있습니다"),
      processedRoots: 0,
      totalRoots: 0,
      processedEntries: 0,
      processedBytes: 0,
      candidatesFound: 0,
    });

    try {
      const nextReport = await startCleanupScan();
      setCleanupReport(nextReport);
      setCleanupScanState("success");
      setCleanupProgress(null);
    } catch (reason) {
      const message = normalizeError(reason);
      if (message.toLocaleLowerCase("en-US").includes("cancel")) {
        setCleanupScanState("cancelled");
        setCleanupError(null);
      } else {
        setCleanupScanState("error");
        setCleanupError(message);
      }
      setCleanupProgress(null);
    }
  }

  async function stopCleanupScan() {
    try {
      const cancellationRequested = await cancelScan();
      if (cancellationRequested) {
        setCleanupProgress((current) => ({
          message: t("안전하게 정리 후보 분석을 중단하고 있습니다"),
          processedRoots: current?.processedRoots ?? 0,
          totalRoots: current?.totalRoots ?? 0,
          processedEntries: current?.processedEntries ?? 0,
          processedBytes: current?.processedBytes ?? 0,
          candidatesFound: current?.candidatesFound ?? 0,
        }));
      }
    } catch (reason) {
      setCleanupError(normalizeError(reason));
    }
  }

  async function runDocumentIndex() {
    const indexRoot = root ?? documentIndex?.root ?? (await pickFolder());
    if (
      !indexRoot ||
      documentIndexState === "scanning" ||
      scanState === "scanning" ||
      driveScanState === "scanning" ||
      directoryScanState === "scanning" ||
      cleanupScanState === "scanning" ||
      fileCatalogState === "scanning" ||
      trashRunning ||
      recoveryChecking
    )
      return;

    setActiveView("documents");
    setDocumentIndexState("scanning");
    setDocumentError(null);
    setDocumentProgress({
      phase: "discovering",
      message: t("문서 검색 목록을 준비하고 있습니다…"),
      scannedFiles: 0,
      candidateDocuments: 0,
      indexedDocuments: 0,
      reusedDocuments: 0,
      processedBytes: 0,
      skippedDocuments: 0,
      unreadableEntries: 0,
    });

    try {
      const nextReport = await startDocumentIndex(indexRoot);
      setDocumentIndex(nextReport);
      setDocumentBuild(nextReport);
      setDocumentIndexState("success");
      setDocumentProgress(null);
    } catch (reason) {
      const message = normalizeError(reason);
      if (message.toLocaleLowerCase("en-US").includes("cancel")) {
        setDocumentIndexState("cancelled");
        setDocumentError(null);
      } else {
        setDocumentIndexState("error");
        setDocumentError(message);
      }
      setDocumentProgress(null);
    }
  }

  async function stopDocumentIndex() {
    try {
      const cancellationRequested = await cancelScan();
      if (cancellationRequested) {
        setDocumentProgress((current) => ({
          phase: current?.phase ?? "discovering",
          message: t("현재 문서 확인을 마친 뒤 안전하게 멈추고 있습니다…"),
          scannedFiles: current?.scannedFiles ?? 0,
          candidateDocuments: current?.candidateDocuments ?? 0,
          indexedDocuments: current?.indexedDocuments ?? 0,
          reusedDocuments: current?.reusedDocuments ?? 0,
          processedBytes: current?.processedBytes ?? 0,
          skippedDocuments: current?.skippedDocuments ?? 0,
          unreadableEntries: current?.unreadableEntries ?? 0,
        }));
      }
    } catch (reason) {
      setDocumentError(normalizeError(reason));
    }
  }

  async function runFileCatalogBuild(
    options: { stayOnView?: boolean; rootOverride?: string } = {},
  ) {
    const catalogRoot =
      options.rootOverride ??
      root ??
      fileCatalog?.root ??
      volume?.mountPoint ??
      (await pickFolder());
    if (
      !catalogRoot ||
      fileCatalogState === "scanning" ||
      fileCatalogClearing ||
      scanState === "scanning" ||
      driveScanState === "scanning" ||
      directoryScanState === "scanning" ||
      cleanupScanState === "scanning" ||
      documentIndexState === "scanning" ||
      trashRunning ||
      recoveryChecking
    )
      return;

    if (!options.stayOnView) setActiveView("files");
    setFileCatalogState("scanning");
    setFileCatalogError(null);
    setFileCatalogProgress({
      phase: "discovering",
      message: t("파일 검색 목록을 준비하고 있습니다…"),
      scannedEntries: 0,
      indexedEntries: 0,
      indexedFiles: 0,
      indexedDirectories: 0,
      processedBytes: 0,
      unreadableEntries: 0,
    });

    try {
      const nextReport = await startFileCatalogBuild(catalogRoot);
      setFileCatalog(nextReport);
      setFileCatalogBuild(nextReport);
      setFileCatalogStale(false);
      setFileCatalogState("success");
      setFileCatalogProgress(null);
      try {
        setRecentFiles(await getRecentFileCatalogEntries());
        if (options.stayOnView) setDashboardError(null);
      } catch (reason) {
        if (options.stayOnView) {
          setDashboardError(
            t("최근 파일을 읽지 못했습니다. {{detail}}", { detail: normalizeError(reason) }),
          );
        }
      }
    } catch (reason) {
      const message = normalizeError(reason);
      if (message.toLocaleLowerCase("en-US").includes("cancel")) {
        setFileCatalogState(fileCatalog ? "success" : "cancelled");
        setFileCatalogError(null);
      } else {
        setFileCatalogState("error");
        setFileCatalogError(message);
      }
      setFileCatalogProgress(null);
      if (options.stayOnView) setDashboardError(message);
    }
  }

  async function stopFileCatalogBuild() {
    try {
      const cancellationRequested = await cancelScan();
      if (cancellationRequested) {
        setFileCatalogProgress((current) => ({
          phase: current?.phase ?? "discovering",
          message: t("현재 파일 확인을 마친 뒤 목록 만들기를 안전하게 멈추고 있습니다…"),
          scannedEntries: current?.scannedEntries ?? 0,
          indexedEntries: current?.indexedEntries ?? 0,
          indexedFiles: current?.indexedFiles ?? 0,
          indexedDirectories: current?.indexedDirectories ?? 0,
          processedBytes: current?.processedBytes ?? 0,
          unreadableEntries: current?.unreadableEntries ?? 0,
        }));
      }
    } catch (reason) {
      setFileCatalogError(normalizeError(reason));
    }
  }

  async function clearFileCatalogIndex() {
    if (!fileCatalog || selectionBlocked) return;
    setFileCatalogClearing(true);
    setFileCatalogError(null);
    try {
      await clearFileCatalog();
      setFileCatalog(null);
      setFileCatalogBuild(null);
      setFileCatalogProgress(null);
      setFileCatalogStale(false);
      setRecentFiles(null);
      setFileCatalogState("idle");
    } catch (reason) {
      setFileCatalogError(normalizeError(reason));
    } finally {
      setFileCatalogClearing(false);
    }
  }

  function invalidateAnalysisReports() {
    cleanupTreeEpoch.current += 1;
    setCleanupTree(null);
    setReport(null);
    setProgress(null);
    setScanState("idle");
    setDriveReport(null);
    setDriveProgress(null);
    setDriveScanState("idle");
    setDirectoryReport(null);
    setDirectoryProgress(null);
    setDirectoryScanState("idle");
    setDirectoryBreadcrumbs([]);
    setCleanupReport(null);
    setCleanupProgress(null);
    setCleanupScanState("idle");
  }

  async function runTrashAction(
    source: "duplicates" | "cleanup" | "cleanup-tree" | "overview" | "assistant",
    action: () => Promise<TrashOperationResult>,
  ): Promise<TrashOperationResult> {
    if (
      trashRunning ||
      scanState === "scanning" ||
      driveScanState === "scanning" ||
      directoryScanState === "scanning" ||
      cleanupScanState === "scanning" ||
      documentIndexState === "scanning" ||
      fileCatalogState === "scanning" ||
      fileCatalogClearing ||
      recoveryChecking
    ) {
      throw new Error(t("다른 스캔 또는 정리 작업이 진행 중입니다"));
    }

    setTrashRunning(true);
    setTrashProgress({
      phase: "preflight",
      message: t("서버에 보관된 스캔 결과와 선택 항목을 대조하고 있습니다"),
      processedItems: 0,
      totalItems: 0,
    });
    setTrashResult(null);
    setTrashResultSource(source);
    setTrashError(null);

    try {
      const result = await action();
      setTrashResult(result);
      if (result.movedCount > 0 && fileCatalog) setFileCatalogStale(true);
      return result;
    } catch (reason) {
      const message = normalizeError(reason);
      setTrashError(message);
      // An IPC/journal failure may happen after a move. Require a refresh conservatively.
      if (fileCatalog) setFileCatalogStale(true);
      throw new Error(message);
    } finally {
      setTrashRunning(false);
      setTrashProgress(null);
      invalidateAnalysisReports();
      void getSystemOverview()
        .then(setSystem)
        .catch((reason: unknown) => {
          console.warn("휴지통 이동 후 디스크 사용량을 새로 고치지 못했습니다.", reason);
        });
      void getActionHistory()
        .then(setActionHistory)
        .catch((reason: unknown) => {
          console.warn("최근 정리 이력을 새로 고치지 못했습니다.", reason);
        });
    }
  }

  function moveDuplicateFiles(request: DuplicateTrashRequest) {
    return runTrashAction("duplicates", () => trashDuplicateFiles(request));
  }

  function moveCleanupCandidates(request: CleanupTrashRequest) {
    return runTrashAction("cleanup", () => trashCleanupCandidates(request));
  }

  async function moveDirectoryFile(path: string, generation: number) {
    const scanRoot = directoryReport?.root;
    const breadcrumbs = directoryBreadcrumbs;
    const result = await runTrashAction("overview", () => trashDirectoryFile(path, generation));
    if (result.movedCount > 0 && scanRoot) {
      await runDirectoryScan(scanRoot, breadcrumbs);
    }
    return result;
  }

  async function prepareDirectoryFolder(path: string, generation: number) {
    if (selectionBlocked) throw new Error(t("다른 스캔 또는 정리 작업이 진행 중입니다"));
    setTrashRunning(true);
    setTrashResultSource("overview");
    setTrashProgress({ phase: "preflight", message: t("포함 항목과 변경 여부를 확인하고 있습니다. 검토가 끝나기 전에는 이동할 수 없습니다."), processedItems: 0, totalItems: 1 });
    try {
      return await prepareDirectoryFolderPlan(path, generation);
    } finally {
      setTrashRunning(false);
      setTrashProgress(null);
    }
  }

  async function moveDirectoryFolder(planId: string, generation: number, acknowledged: boolean) {
    const scanRoot = directoryReport?.root;
    const breadcrumbs = directoryBreadcrumbs;
    const result = await runTrashAction("overview", () => confirmDirectoryFolderPlan(planId, generation, acknowledged));
    if (result.movedCount > 0 && scanRoot) await runDirectoryScan(scanRoot, breadcrumbs);
    return result;
  }

  async function stopTrashAction() {
    try {
      const cancellationRequested = await cancelScan();
      if (cancellationRequested) {
        setTrashProgress((current) => ({
          phase: current?.phase ?? "preflight",
          message: t("현재 항목을 마친 뒤 안전하게 작업을 중단하고 있습니다"),
          processedItems: current?.processedItems ?? 0,
          totalItems: current?.totalItems ?? 0,
        }));
      }
    } catch (reason) {
      setTrashError(normalizeError(reason));
    }
  }

  const controlSettings: ControlStatusPanelProps = {
    status: controlStatus,
    updatingPermissionLifetime: permissionLifetimeUpdating,
    permissionLifetimeError,
    permissionControlsLocked: permissionLifetimeUpdating,
    permissionLifetimeLocked: selectionBlocked || controlAccessUpdating || controlScanAccessUpdating || controlCleanupAccessUpdating || inspectionUpdating,
    onPermissionLifetimeChange: lifetime => void changePermissionLifetime(lifetime),
    canEnableSearch: Boolean(fileCatalog || documentIndex),
    updatingSearchAccess: controlAccessUpdating,
    searchAccessError: controlAccessError,
    onToggleSearchAccess: () => void toggleControlSearchAccess(),
    scanAccessError: controlScanAccessError,
    canEnableCleanup: controlStatus.bridgeAvailable && !selectionBlocked,
    cleanupAccessLocked: selectionBlocked,
    updatingCleanupAccess: controlCleanupAccessUpdating,
    cleanupAccessError: controlCleanupAccessError,
    onToggleCleanupAccess: () => void toggleControlCleanupAccess(),
    onReviewPending: () => void openPendingCleanupReview(),
    updatingInspectionAccess: inspectionUpdating,
    inspectionAccessError: inspectionError,
    onToggleInspectionAccess: () => void toggleInspectionAccess(),
  };

  return <>
    <div style={{ display: "contents" }} inert={appToolReview !== null}>
    <AppShell
      activeView={activeView}
      root={
        activeView === "overview"
          ? directoryReport?.root ?? root
          : activeView === "large-files" || activeView === "duplicates"
            ? report?.root ?? root
          : activeView === "documents"
          ? root ?? documentIndex?.root ?? null
          : activeView === "files"
            ? root ?? fileCatalog?.root ?? volume?.mountPoint ?? null
            : root
      }
      report={report}
      volume={volume}
      mobileNavigationOpen={mobileNavigationOpen}
      selectionBlocked={selectionBlocked}
      dockerEnabled={dockerStatus?.enabled === true}
      onMobileNavigationChange={setMobileNavigationOpen}
      onNavigate={navigate}
      onPickFolder={() => {
        if (activeView === "overview") void pickStorageFolder();
        else void pickFolder();
      }}
    >
      <span
        className="sr-only background-scan-announcement"
        role="status"
        aria-live="polite"
        aria-atomic="true"
      >
        {backgroundScanTerminalAnnouncement}
      </span>
      <span className="sr-only background-scan-error-announcement" role="alert">
        {backgroundScanErrorAnnouncement}
      </span>
      {recoveryChecking && recoveryCheckSlow ? (
        <RecoveryCheckNotice
          error={null}
          onRetry={() => void retryActionRecovery()}
          onDismiss={() => setRecoveryErrorDismissed(true)}
        />
      ) : null}
      {!recoveryChecking && recoveryCheckError && !recoveryErrorDismissed ? (
        <RecoveryCheckNotice
          error={recoveryCheckError}
          onRetry={() => void retryActionRecovery()}
          onDismiss={() => setRecoveryErrorDismissed(true)}
        />
      ) : null}
      {!recoveryChecking &&
      recoveryReport &&
      !recoveryDismissed &&
      (recoveryReport.incompleteOperations.length > 0 ||
        recoveryReport.issues.length > 0) ? (
        <RecoveryNotice
          report={recoveryReport}
          openingTrash={openingSystemTrash}
          actionError={recoveryActionError}
          onOpenTrash={() => void showSystemTrash()}
          onDismiss={() => setRecoveryDismissed(true)}
        />
      ) : null}
      {activeView === "dashboard" ? (
        <DashboardView
          system={system}
          report={report}
          progress={progress}
          scanState={scanState}
          scanError={error}
          cleanupReport={cleanupReport}
          cleanupProgress={cleanupProgress}
          cleanupState={cleanupScanState}
          cleanupError={cleanupError}
          actionHistory={actionHistory}
          recentFiles={recentFiles}
          fileCatalog={fileCatalog}
          fileCatalogStale={fileCatalogStale}
          loading={dashboardLoading}
          error={dashboardError}
          blocked={selectionBlocked}
          onRefresh={() => void refreshDashboard()}
          onStartScan={(nextVolume) => void runDashboardScan(nextVolume)}
          onCancelScan={() => void stopScan()}
          onStartCleanupScan={() => void runCleanupScan({ stayOnView: true })}
          onCancelCleanupScan={() => void stopCleanupScan()}
          onOpenStorage={() => navigate("overview")}
          onOpenLargeFiles={() => navigate("large-files")}
          onOpenDuplicates={() => navigate("duplicates")}
          onOpenCleanup={() => navigate("cleanup")}
          onOpenPerformance={() => navigate("performance")}
          onRefreshFileCatalog={() =>
            void runFileCatalogBuild({ stayOnView: true })
          }
          onOpenFileSearch={() => navigate("files")}
          onRevealFile={(path) => void revealDashboardFile(path)}
        />
      ) : null}
      {storageViews.has(activeView) ? (
        <StorageSectionNav activeView={activeView} onNavigate={navigate} />
      ) : null}
      <EmptyTrashControl visible={storageViews.has(activeView) || activeView === "applications"} platform={system?.platform ?? null}
        blocked={selectionBlocked || !!recoveryCheckError || !!recoveryReport?.issues.length || !!recoveryReport?.incompleteOperations.some((operation) => !operation.resolved || operation.attentionCount > 0)}
        onBusyChange={setEmptyTrashRunning}
        onSettled={() => {
          invalidateAnalysisReports();
          setTrashResult(null);
          setFileCatalogStale(true);
          void getSystemOverview().then(setSystem).catch(() => {});
        }} />
      {fileViews.has(activeView) ? (
        <FileSectionNav activeView={activeView} onNavigate={navigate} />
      ) : null}
      {applicationsVisited || activeView === "applications" ? (
        <div hidden={activeView !== "applications"}>
        <ApplicationsView
          active={activeView === "applications"}
          busy={selectionBlocked || !!recoveryCheckError || !!recoveryReport?.issues.length || !!recoveryReport?.incompleteOperations.some((operation) => !operation.resolved || operation.attentionCount > 0)}
          onStatus={setApplicationStatus}
          onBusyChange={setApplicationRunning}
          onMutated={() => {
            invalidateAnalysisReports();
            setTrashResult(null);
            setFileCatalogStale(true);
            void getSystemOverview().then(setSystem).catch(() => {});
            void getActionHistory().then(setActionHistory).catch(() => {});
            setRecoveryChecking(true);
            void getActionRecoveryStatus().then((status) => {
              setRecoveryReport(status);
              setRecoveryDismissed(false);
              setRecoveryCheckError(null);
            }).catch((reason: unknown) => {
              setRecoveryCheckError(normalizeError(reason));
              setRecoveryErrorDismissed(false);
            }).finally(() => setRecoveryChecking(false));
          }}
        />
        </div>
      ) : null}
      <span className="sr-only" role="status" aria-live="polite">{applicationStatus}</span>
      {activeView === "overview" ? (
        <OverviewView
          platform={system?.platform ?? null}
          root={directoryBreadcrumbs[0]?.path ?? root}
          volume={volume}
          report={report}
          progress={progress}
          state={scanState}
          error={error}
          driveReport={driveReport}
          driveProgress={driveProgress}
          driveState={driveScanState}
          driveError={driveError}
          directoryReport={directoryReport}
          directoryProgress={directoryProgress}
          directoryState={directoryScanState}
          directoryError={directoryError}
          directoryBreadcrumbs={directoryBreadcrumbs}
          blocked={
            cleanupTreeBusy ||
            recoveryChecking ||
            cleanupScanState === "scanning" ||
            documentIndexState === "scanning" ||
            fileCatalogState === "scanning" ||
            fileCatalogClearing ||
            trashRunning
          }
          onPickFolder={() => void pickStorageFolder()}
          onStartScan={() => void runScan({ rootOverride: directoryReport?.root })}
          onCancelScan={() => void stopScan()}
          onStartDriveScan={() => void runDriveScan()}
          onCancelDriveScan={() => void stopDriveScan()}
          onStartDirectoryScan={(path, breadcrumbs) =>
            void runDirectoryScan(path, breadcrumbs)
          }
          onCancelDirectoryScan={() => void stopDirectoryScan()}
          onRevealDirectoryItem={revealPath}
          onTrashDirectoryFile={moveDirectoryFile}
          onPrepareDirectoryFolder={prepareDirectoryFolder}
          onConfirmDirectoryFolder={moveDirectoryFolder}
          onDismissDirectoryFolder={dismissDirectoryFolderPlan}
          onCancelTrash={() => void stopTrashAction()}
          trashProgress={trashResultSource === "overview" ? trashProgress : null}
          onOpenLargeFiles={() => navigate("large-files")}
          onOpenDuplicates={() => navigate("duplicates")}
          onOpenCleanup={() => { setCleanupMode("system"); navigate("cleanup"); }}
          onOpenCleanupTree={(generation) => void openCandidateTree({ kind: "directory", generation })}
        />
      ) : null}
      {activeView === "large-files" ? (
        <LargeFilesView
          report={report}
          scanning={selectionBlocked}
          onStartScan={() => void runScan()}
        />
      ) : null}
      {activeView === "files" ? (
        <FastFileSearchView
          selectedRoot={root}
          defaultRoot={volume?.mountPoint ?? null}
          catalog={fileCatalog}
          lastBuild={fileCatalogBuild}
          progress={fileCatalogProgress}
          state={fileCatalogState}
          error={fileCatalogError}
          stale={fileCatalogStale}
          blocked={
            recoveryChecking ||
            trashRunning ||
            scanState === "scanning" ||
            driveScanState === "scanning" ||
            directoryScanState === "scanning" ||
            cleanupScanState === "scanning" ||
            documentIndexState === "scanning" ||
            fileCatalogClearing
          }
          onPickFolder={() => void pickFolder()}
          onStartCatalog={() => void runFileCatalogBuild()}
          onCancelCatalog={() => void stopFileCatalogBuild()}
          onClearCatalog={clearFileCatalogIndex}
        />
      ) : null}
      {activeView === "documents" ? (
        <DocumentSearchView
          selectedRoot={root}
          index={documentIndex}
          lastBuild={documentBuild}
          progress={documentProgress}
          state={documentIndexState}
          error={documentError}
          blocked={
            recoveryChecking ||
            trashRunning ||
            scanState === "scanning" ||
            driveScanState === "scanning" ||
            directoryScanState === "scanning" ||
            cleanupScanState === "scanning" ||
            fileCatalogState === "scanning" ||
            fileCatalogClearing
          }
          onPickFolder={() => void pickFolder()}
          onStartIndex={() => void runDocumentIndex()}
          onCancelIndex={() => void stopDocumentIndex()}
        />
      ) : null}
      {activeView === "docker" ? (
        <DockerManagementView
          status={dockerStatus}
          loading={dockerStatusLoading}
          error={dockerStatusError}
          onRefresh={refreshDockerStatus}
          onStatusChange={setDockerStatus}
          onAskInChat={openDockerConversation}
        />
      ) : null}
      {activeView === "assistant" ? (
        <AssistantView
          controlSettings={controlSettings}
          directoryProgress={directoryProgress}
          directoryState={directoryScanState}
          volumes={system?.volumes ?? []}
          dockerStatus={dockerStatus}
          launchRequest={assistantLaunchRequest}
          onLaunchRequestHandled={() => setAssistantLaunchRequest(null)}
          onPickFolder={() => pickStorageFolder({ stayOnView: true })}
          onConfirmEmptyPlan={(sessionId, revision, planId) => runTrashAction("assistant", () => confirmAssistantEmptyPlan(sessionId, revision, planId))}
          onConfirmFilePlan={(sessionId, revision, planId, nestedAck) => runTrashAction("assistant", () => confirmAssistantFilePlan(sessionId, revision, planId, nestedAck))}
          onDirectoryReport={acceptAssistantDirectoryReport}
          onOpenCleanupTree={(sessionId, revision) => void openCandidateTree({ kind: "assistant", sessionId, revision })}
          onAppToolView={showAppToolView}
          onAppToolReview={openAppToolReview}
          appToolCompletion={appToolCompletion}
        />
      ) : null}
      {activeView === "performance" ? <PerformanceView /> : null}
      {activeView === "cleanup" ? (
        <div className="view-stack">
          <section className="cleanup-source-toolbar" aria-label={t("정리 후보")}>
            <div className="cleanup-source-toolbar__modes">
              <button type="button" className={cleanupMode === "tree" ? "secondary-button is-active" : "text-button"} aria-pressed={cleanupMode === "tree"} disabled={cleanupTreeBusy || trashRunning} onClick={() => setCleanupMode("tree")}>{t("폴더 트리")}</button>
              <button type="button" className={cleanupMode === "system" ? "secondary-button is-active" : "text-button"} aria-pressed={cleanupMode === "system"} disabled={cleanupTreeBusy || trashRunning} onClick={() => setCleanupMode("system")}>{t("시스템 후보")}</button>
            </div>
            {cleanupMode === "tree" ? <div className="cleanup-source-toolbar__actions">
              {directoryReport ? <button type="button" className="secondary-button" disabled={selectionBlocked} onClick={() => void openCandidateTree({ kind: "directory", generation: directoryReport.generation })}>{t("검사 결과에서 후보 열기")}</button> : null}
              <button type="button" className="secondary-button" disabled={selectionBlocked} onClick={() => void inspectCandidateFolder()}>{t("폴더를 선택해 후보 검사")}</button>
            </div> : null}
            <p>{t("폴더별로 확인하고 이동할 항목만 선택합니다. 시스템 캐시 후보는 별도 목록에서 검토하세요.")}</p>
          </section>
          {cleanupMode === "tree" ? <CleanupTreePanel
            tree={cleanupTree}
            busy={selectionBlocked}
            error={cleanupTreeError ?? (trashResultSource === "cleanup-tree" ? trashError : null)}
            result={trashResultSource === "cleanup-tree" ? trashResult : null}
            progress={trashResultSource === "cleanup-tree" ? trashProgress : null}
            onLoadChildren={(parentId, offset) => changeCandidateTree((tree) => loadCleanupTreeChildren(tree.treeId, parentId, offset))}
            onSelectionChange={(includeIds, excludeIds) => changeCandidateTree((tree) => updateCleanupTreeSelection(tree.treeId, tree.selectionRevision, includeIds, excludeIds))}
            onPrepare={() => changeCandidateTree((tree) => prepareCleanupTreePlan(tree.treeId, tree.selectionRevision))}
            onConfirm={moveCandidateTree}
            onDismissPlan={() => changeCandidateTree((tree) => tree.plan ? dismissCleanupTreePlan(tree.treeId, tree.plan.id) : Promise.resolve(tree))}
            onRefresh={refreshCandidateTree}
            onCancel={() => { void cancelScan().catch((reason) => setCleanupTreeError(normalizeError(reason))); }}
            onOpen={(path) => inspectFile(path)}
            onReveal={revealPath}
          /> : <CleanupView
          platform={system?.platform ?? null}
          report={cleanupReport}
          progress={cleanupProgress}
          state={cleanupScanState}
          error={cleanupError}
          blocked={
            cleanupTreeBusy ||
            scanState === "scanning" ||
            driveScanState === "scanning" ||
            directoryScanState === "scanning" ||
            documentIndexState === "scanning" ||
            fileCatalogState === "scanning" ||
            fileCatalogClearing ||
            trashRunning ||
            recoveryChecking
          }
          actionRunning={trashRunning && trashResultSource === "cleanup"}
          actionProgress={trashResultSource === "cleanup" ? trashProgress : null}
          actionResult={trashResultSource === "cleanup" ? trashResult : null}
          actionError={trashResultSource === "cleanup" ? trashError : null}
          onStart={() => void runCleanupScan()}
          onCancel={() => void stopCleanupScan()}
          onMoveToTrash={moveCleanupCandidates}
          onCancelAction={() => void stopTrashAction()}
        />}
        </div>
      ) : null}
      {activeView === "duplicates" ? (
        <DuplicatesView
          report={report}
          scanning={selectionBlocked}
          actionRunning={trashRunning && trashResultSource === "duplicates"}
          actionProgress={trashResultSource === "duplicates" ? trashProgress : null}
          actionResult={trashResultSource === "duplicates" ? trashResult : null}
          actionError={trashResultSource === "duplicates" ? trashError : null}
          onStartScan={() => void runScan()}
          onMoveToTrash={moveDuplicateFiles}
          onCancelAction={() => void stopTrashAction()}
        />
      ) : null}
      {activeView === "settings" ? (
        <SettingsView
          controlSettings={controlSettings}
          config={config}
          dockerStatus={dockerStatus}
          dockerLoading={dockerStatusLoading}
          dockerChanging={dockerStatusChanging}
          dockerError={dockerStatusError}
          onConfigChange={(nextConfig) => void updateScanConfig(nextConfig)}
          onDockerEnabledChange={updateDockerEnabled}
          onOpenDocker={() => navigate("docker")}
        />
      ) : null}

      <SafetyActionDialog
        open={Boolean(pendingCleanupPlan)}
        title={
          t("휴지통 이동 최종 확인")
        }
        itemCount={pendingCleanupPlan?.itemCount ?? 0}
        logicalBytes={pendingCleanupPlan?.totalBytes ?? 0}
        reviewCount={pendingCleanupPlan?.reviewCount ?? 0}
        busy={trashRunning}
        progress={trashProgress}
        error={pendingCleanupPlanError ?? trashError}
        intro={t("AI에는 종류와 용량 요약만 전달하고, 파일 이동은 앱에서 다시 확인합니다.")}
        items={(pendingCleanupPlan?.items ?? []).map((item) => ({
          path: displayPath(item.path),
          logicalBytes: item.logicalBytes,
          detail: item.detail,
        }))}
        confirmLabel={t("확인하고 휴지통으로 이동")}
        onConfirm={(reviewAcknowledged) =>
          void confirmPendingCleanupReview(reviewAcknowledged)
        }
        onCancel={() =>
          void (trashRunning ? stopTrashAction() : closePendingCleanupReview())
        }
        onClose={() => void closePendingCleanupReview()}
      />

      {controlStatus.pendingReview && !pendingCleanupPlan && !selectionBlocked ? (
        <div className="scan-status-dock mcp-review-dock">
          <span>
            <strong>{t("최종 확인 대기")}</strong>
            <small>
              {pendingCleanupPlanError ??
                t("{{count}}개 · 앱에서 정확한 경로를 확인해야 실행됩니다.", {
                  count: controlStatus.pendingReview.itemCount,
                })}
            </small>
          </span>
          <button
            type="button"
            disabled={pendingCleanupPlanLoading}
            onClick={() => void openPendingCleanupReview()}
          >
            {pendingCleanupPlanLoading ? t("불러오는 중…") : t("검토하기")}
          </button>
        </div>
      ) : null}

      {(trashRunning && activeView !== (trashResultSource === "cleanup-tree" ? "cleanup" : trashResultSource)) ||
      (!trashRunning &&
        (scanState === "scanning" ||
          driveScanState === "scanning" ||
          directoryScanState === "scanning" ||
          cleanupScanState === "scanning" ||
          documentIndexState === "scanning" ||
          fileCatalogState === "scanning") &&
        activeView !== "overview" &&
        activeView !== "cleanup" &&
        !(
          activeView === "dashboard" &&
          (scanState === "scanning" || cleanupScanState === "scanning")
        ) &&
        !(activeView === "documents" && documentIndexState === "scanning") &&
        !(activeView === "files" && fileCatalogState === "scanning")) ? (
        <div className="scan-status-dock">
          <span>
            <strong role="status" aria-live="polite" aria-atomic="true">
              {trashRunning
                ? t("휴지통 이동 중")
                : directoryScanState === "scanning"
                ? t("저장공간 맵 분석 중")
                : cleanupScanState === "scanning"
                  ? t("정리 후보 분석 중")
                : documentIndexState === "scanning"
                  ? t("문서 검색 준비 중")
                : fileCatalogState === "scanning"
                  ? t("파일 목록 만드는 중")
                : driveScanState === "scanning"
                  ? t("드라이브 분석 중")
                : controlStatus.activeOperation?.source === "chatCli" &&
                    controlStatus.activeOperation.kind === "storageScan"
                  ? t("채팅에서 요청한 파일 검사 중")
                  : t("스캔 진행 중")}
            </strong>
            <small>
              {trashRunning
                ? t("선택 항목을 안전하게 다시 확인하고 있습니다")
                : directoryScanState === "scanning"
                ? t("폴더 구조를 확인하고 있습니다")
                : cleanupScanState === "scanning"
                  ? t("남은 파일과 제거 정보를 대조하고 있습니다")
                : documentIndexState === "scanning"
                  ? t("문서 내용을 검색할 수 있게 정리하고 있습니다…")
                : fileCatalogState === "scanning"
                  ? t("파일 이름과 경로를 수집하고 있습니다")
                : driveScanState === "scanning"
                  ? t("저장공간을 분류하고 있습니다")
                  : t("파일을 확인하고 있습니다")}
            </small>
          </span>
          <button
            type="button"
            onClick={() =>
              void (trashRunning
                ? stopTrashAction()
                : directoryScanState === "scanning"
                ? stopDirectoryScan()
                : cleanupScanState === "scanning"
                  ? stopCleanupScan()
                : documentIndexState === "scanning"
                  ? stopDocumentIndex()
                : fileCatalogState === "scanning"
                  ? stopFileCatalogBuild()
                : driveScanState === "scanning"
                  ? stopDriveScan()
                  : stopScan())
            }
          >
            <X size={16} aria-hidden="true" />
            {t("취소")}
          </button>
        </div>
      ) : null}
    </AppShell>
    </div>
    {appToolReview ? <AssistantAppToolReview key={`${appToolReview.result.capability}-${appToolReview.result.capturedAtUnixMs}`} result={appToolReview.result} busy={selectionBlocked} onBusyChange={setApplicationRunning} onClose={closeAppToolReview} onCompleted={completedAppToolReview} /> : null}
    {appToolNotice && !appToolReview ? <div className="scan-status-dock"><p role="status">{appToolNotice}</p><button type="button" onClick={() => setAppToolNotice(null)}>{t("닫기")}</button></div> : null}
  </>;
}

function displayPath(path: string): string {
  if (path.startsWith("\\\\?\\UNC\\")) return `\\\\${path.slice(8)}`;
  if (path.startsWith("\\\\?\\")) return path.slice(4);
  return path;
}

function normalizeError(reason: unknown): string {
  if (reason instanceof Error) return reason.message;
  if (typeof reason === "string") return reason;
  switch (document.documentElement.lang) {
    case "ko":
      return "알 수 없는 오류가 발생했습니다";
    case "ja":
      return "不明なエラーが発生しました";
    case "zh-CN":
      return "发生未知错误";
    default:
      return "An unknown error occurred";
  }
}

export default App;
