import { X } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import "./App.css";
import { AppShell } from "./components/AppShell";
import { RecoveryCheckNotice, RecoveryNotice } from "./components/RecoveryNotice";
import {
  cancelScan,
  clearFileCatalog,
  getActionRecoveryStatus,
  getDocumentIndexStatus,
  getFileCatalogStatus,
  getSystemOverview,
  listenToCleanupScanProgress,
  listenToDirectoryScanProgress,
  listenToDriveScanProgress,
  listenToDocumentIndexProgress,
  listenToFileCatalogProgress,
  listenToScanProgress,
  listenToTrashProgress,
  openSystemTrash,
  selectDirectory,
  startDirectoryScan,
  startCleanupScan,
  startDriveScan,
  startDocumentIndex,
  startFileCatalogBuild,
  startScan,
  trashCleanupCandidates,
  trashDuplicateFiles,
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
  DocumentIndexProgress,
  DocumentIndexReport,
  DocumentIndexStatus,
  FileCatalogProgress,
  FileCatalogReport,
  FileCatalogStatus,
} from "./types";
import { DEFAULT_SCAN_CONFIG } from "./types";
import { DuplicatesView } from "./views/DuplicatesView";
import { CleanupView } from "./views/CleanupView";
import { LargeFilesView } from "./views/LargeFilesView";
import { OverviewView } from "./views/OverviewView";
import { SettingsView } from "./views/SettingsView";
import { DocumentSearchView } from "./views/DocumentSearchView";
import { FastFileSearchView } from "./views/FastFileSearchView";

function App() {
  const [activeView, setActiveView] = useState<ViewId>("overview");
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
  const [cleanupReport, setCleanupReport] = useState<CleanupScanReport | null>(null);
  const [cleanupProgress, setCleanupProgress] = useState<CleanupScanProgress | null>(null);
  const [cleanupScanState, setCleanupScanState] = useState<ScanUiState>("idle");
  const [cleanupError, setCleanupError] = useState<string | null>(null);
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
  const [trashRunning, setTrashRunning] = useState(false);
  const [trashProgress, setTrashProgress] = useState<TrashProgress | null>(null);
  const [trashResult, setTrashResult] = useState<TrashOperationResult | null>(null);
  const [trashResultSource, setTrashResultSource] = useState<"duplicates" | "cleanup" | null>(null);
  const [trashError, setTrashError] = useState<string | null>(null);
  const [recoveryReport, setRecoveryReport] = useState<ActionRecoveryReport | null>(null);
  const [recoveryChecking, setRecoveryChecking] = useState(true);
  const [recoveryCheckSlow, setRecoveryCheckSlow] = useState(false);
  const [recoveryCheckError, setRecoveryCheckError] = useState<string | null>(null);
  const [recoveryDismissed, setRecoveryDismissed] = useState(false);
  const [recoveryErrorDismissed, setRecoveryErrorDismissed] = useState(false);
  const [openingSystemTrash, setOpeningSystemTrash] = useState(false);
  const [recoveryActionError, setRecoveryActionError] = useState<string | null>(null);
  const startupRecoveryPromise = useRef<Promise<ActionRecoveryReport> | null>(null);
  const [config, setConfig] = useState<ScanConfig>(DEFAULT_SCAN_CONFIG);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    let unlistenDrive: (() => void) | undefined;
    let unlistenDirectory: (() => void) | undefined;
    let unlistenCleanup: (() => void) | undefined;
    let unlistenTrash: (() => void) | undefined;
    let unlistenDocuments: (() => void) | undefined;
    let unlistenFileCatalog: (() => void) | undefined;
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

    getSystemOverview()
      .then((overview) => {
        if (!disposed) setSystem(overview);
      })
      .catch((reason: unknown) => {
        if (!disposed) setError(normalizeError(reason));
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
    };
  }, []);

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

  const volume = useMemo(
    () => findVolumeForPath(system?.volumes ?? [], root ?? report?.root ?? null),
    [report?.root, root, system?.volumes],
  );
  const selectionBlocked =
    recoveryChecking ||
    trashRunning ||
    scanState === "scanning" ||
    driveScanState === "scanning" ||
    directoryScanState === "scanning" ||
    cleanupScanState === "scanning" ||
    documentIndexState === "scanning" ||
    fileCatalogState === "scanning" ||
    fileCatalogClearing;

  function navigate(view: ViewId) {
    const transition = document.startViewTransition?.(() => setActiveView(view));
    if (!transition) setActiveView(view);
  }

  async function pickFolder(): Promise<string | null> {
    if (selectionBlocked) return null;
    try {
      const selected = await selectDirectory();
      if (!selected) return null;

      if (selected !== root) {
        setRoot(selected);
        setReport(null);
        setProgress(null);
        setScanState("idle");
        setError(null);
        setDirectoryReport(null);
        setDirectoryProgress(null);
        setDirectoryScanState("idle");
        setDirectoryError(null);
        setDirectoryBreadcrumbs([]);
        setTrashResult(null);
        setTrashResultSource(null);
        setTrashError(null);
      }
      return selected;
    } catch (reason) {
      setError(normalizeError(reason));
      setScanState("error");
      return null;
    }
  }

  async function runScan() {
    const scanRoot = root ?? (await pickFolder());
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

    setActiveView("overview");
    setTrashResult(null);
    setTrashResultSource(null);
    setTrashError(null);
    setReport(null);
    setScanState("scanning");
    setError(null);
    setProgress({
      phase: "discovering",
      message: "스캔 작업을 준비하고 있습니다",
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
      message: "드라이브 분석을 준비하고 있습니다",
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
          message: "안전하게 스캔을 중단하고 있습니다",
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
          message: "안전하게 드라이브 분석을 중단하고 있습니다",
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
  ) {
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
      return;

    setActiveView("overview");
    setDirectoryScanState("scanning");
    setDirectoryError(null);
    setDirectoryProgress({
      message: "폴더 구조 분석을 준비하고 있습니다",
      processedEntries: 0,
      processedFiles: 0,
      processedBytes: 0,
      unreadableEntries: 0,
    });

    try {
      const nextReport = await startDirectoryScan(scanRoot);
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
      requestAnimationFrame(() => {
        document.getElementById("storage-map")?.scrollIntoView({
          behavior: "smooth",
          block: "start",
        });
      });
    } catch (reason) {
      const message = normalizeError(reason);
      if (message.toLocaleLowerCase("en-US").includes("cancel")) {
        setDirectoryScanState("cancelled");
        setDirectoryError(null);
      } else {
        setDirectoryScanState("error");
        setDirectoryError(message);
      }
      setDirectoryProgress(null);
    }
  }

  async function stopDirectoryScan() {
    try {
      const cancellationRequested = await cancelScan();
      if (cancellationRequested) {
        setDirectoryProgress((current) => ({
          message: "안전하게 폴더 지도 분석을 중단하고 있습니다",
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

  async function runCleanupScan() {
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

    setActiveView("cleanup");
    if (trashResultSource === "cleanup") {
      setTrashResult(null);
      setTrashResultSource(null);
      setTrashError(null);
    }
    setCleanupReport(null);
    setCleanupScanState("scanning");
    setCleanupError(null);
    setCleanupProgress({
      message: "정리 후보 위치를 준비하고 있습니다",
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
          message: "안전하게 정리 후보 분석을 중단하고 있습니다",
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
      message: "문서 검색 목록을 준비하고 있습니다…",
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
          message: "현재 문서 확인을 마친 뒤 안전하게 멈추고 있습니다…",
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

  async function runFileCatalogBuild() {
    const catalogRoot = root ?? fileCatalog?.root ?? volume?.mountPoint ?? (await pickFolder());
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

    setActiveView("files");
    setFileCatalogState("scanning");
    setFileCatalogError(null);
    setFileCatalogProgress({
      phase: "discovering",
      message: "파일 검색 목록을 준비하고 있습니다…",
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
    }
  }

  async function stopFileCatalogBuild() {
    try {
      const cancellationRequested = await cancelScan();
      if (cancellationRequested) {
        setFileCatalogProgress((current) => ({
          phase: current?.phase ?? "discovering",
          message: "현재 파일 확인을 마친 뒤 목록 만들기를 안전하게 멈추고 있습니다…",
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
      setFileCatalogState("idle");
    } catch (reason) {
      setFileCatalogError(normalizeError(reason));
    } finally {
      setFileCatalogClearing(false);
    }
  }

  function invalidateAnalysisReports() {
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
    source: "duplicates" | "cleanup",
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
      throw new Error("다른 스캔 또는 정리 작업이 진행 중입니다");
    }

    setTrashRunning(true);
    setTrashProgress({
      phase: "preflight",
      message: "서버에 보관된 스캔 결과와 선택 항목을 대조하고 있습니다",
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
    }
  }

  function moveDuplicateFiles(request: DuplicateTrashRequest) {
    return runTrashAction("duplicates", () => trashDuplicateFiles(request));
  }

  function moveCleanupCandidates(request: CleanupTrashRequest) {
    return runTrashAction("cleanup", () => trashCleanupCandidates(request));
  }

  async function stopTrashAction() {
    try {
      const cancellationRequested = await cancelScan();
      if (cancellationRequested) {
        setTrashProgress((current) => ({
          phase: current?.phase ?? "preflight",
          message: "현재 항목을 마친 뒤 안전하게 작업을 중단하고 있습니다",
          processedItems: current?.processedItems ?? 0,
          totalItems: current?.totalItems ?? 0,
        }));
      }
    } catch (reason) {
      setTrashError(normalizeError(reason));
    }
  }

  return (
    <AppShell
      activeView={activeView}
      root={
        activeView === "documents"
          ? root ?? documentIndex?.root ?? null
          : activeView === "files"
            ? root ?? fileCatalog?.root ?? volume?.mountPoint ?? null
            : root
      }
      report={report}
      cleanupReport={cleanupReport}
      documentIndex={documentIndex}
      fileCatalog={fileCatalog}
      volume={volume}
      mobileNavigationOpen={mobileNavigationOpen}
      selectionBlocked={selectionBlocked}
      onMobileNavigationChange={setMobileNavigationOpen}
      onNavigate={navigate}
      onPickFolder={() => void pickFolder()}
    >
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
      {activeView === "overview" ? (
        <OverviewView
          platform={system?.platform ?? null}
          root={root}
          volume={volume}
          report={report}
          progress={progress}
          state={scanState}
          error={error}
          driveReport={driveReport}
          driveProgress={driveProgress}
          driveState={driveScanState}
          driveError={driveError}
          directoryRoot={root ?? volume?.mountPoint ?? null}
          directoryReport={directoryReport}
          directoryProgress={directoryProgress}
          directoryState={directoryScanState}
          directoryError={directoryError}
          directoryBreadcrumbs={directoryBreadcrumbs}
          blocked={
            recoveryChecking ||
            cleanupScanState === "scanning" ||
            documentIndexState === "scanning" ||
            fileCatalogState === "scanning" ||
            fileCatalogClearing ||
            trashRunning
          }
          onPickFolder={() => void pickFolder()}
          onStartScan={() => void runScan()}
          onCancelScan={() => void stopScan()}
          onStartDriveScan={() => void runDriveScan()}
          onCancelDriveScan={() => void stopDriveScan()}
          onStartDirectoryScan={(path, breadcrumbs) =>
            void runDirectoryScan(path, breadcrumbs)
          }
          onCancelDirectoryScan={() => void stopDirectoryScan()}
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
      {activeView === "cleanup" ? (
        <CleanupView
          platform={system?.platform ?? null}
          report={cleanupReport}
          progress={cleanupProgress}
          state={cleanupScanState}
          error={cleanupError}
          blocked={
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
        />
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
        <SettingsView config={config} onConfigChange={setConfig} />
      ) : null}

      {(trashRunning && activeView !== trashResultSource) ||
      (!trashRunning &&
        (scanState === "scanning" ||
          driveScanState === "scanning" ||
          directoryScanState === "scanning" ||
          cleanupScanState === "scanning" ||
          documentIndexState === "scanning" ||
          fileCatalogState === "scanning") &&
        activeView !== "overview" &&
        activeView !== "cleanup" &&
        !(activeView === "documents" && documentIndexState === "scanning") &&
        !(activeView === "files" && fileCatalogState === "scanning")) ? (
        <div className="scan-status-dock" role="status" aria-live="polite">
          <span>
            <strong>
              {trashRunning
                ? "휴지통 이동 중"
                : directoryScanState === "scanning"
                ? "저장공간 맵 분석 중"
                : cleanupScanState === "scanning"
                  ? "정리 후보 분석 중"
                : documentIndexState === "scanning"
                  ? "문서 검색 준비 중"
                : fileCatalogState === "scanning"
                  ? "파일 목록 만드는 중"
                : driveScanState === "scanning"
                  ? "드라이브 분석 중"
                  : "스캔 진행 중"}
            </strong>
            <small>
              {trashRunning
                ? trashProgress?.message ?? "선택 항목을 안전하게 다시 확인하고 있습니다"
                : directoryScanState === "scanning"
                ? directoryProgress?.message ?? "폴더 구조를 확인하고 있습니다"
                : cleanupScanState === "scanning"
                  ? cleanupProgress?.message ?? "남은 파일과 제거 정보를 대조하고 있습니다"
                : documentIndexState === "scanning"
                  ? documentProgress?.message ?? "문서 내용을 검색할 수 있게 정리하고 있습니다…"
                : fileCatalogState === "scanning"
                  ? fileCatalogProgress?.message ?? "파일 이름과 경로를 수집하고 있습니다"
                : driveScanState === "scanning"
                  ? driveProgress?.message ?? "저장공간을 분류하고 있습니다"
                  : progress?.message ?? "파일을 확인하고 있습니다"}
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
            취소
          </button>
        </div>
      ) : null}
    </AppShell>
  );
}

function findVolumeForPath(volumes: VolumeInfo[], path: string | null): VolumeInfo | null {
  if (volumes.length === 0) return null;
  if (!path) return volumes.find((volume) => volume.isSystem) ?? volumes[0];

  const normalizedPath = path.toLocaleLowerCase("en-US");
  return (
    [...volumes]
      .sort((left, right) => right.mountPoint.length - left.mountPoint.length)
      .find((volume) =>
        normalizedPath.startsWith(volume.mountPoint.toLocaleLowerCase("en-US")),
      ) ?? volumes[0]
  );
}

function normalizeError(reason: unknown): string {
  if (reason instanceof Error) return reason.message;
  if (typeof reason === "string") return reason;
  return "알 수 없는 오류가 발생했습니다";
}

export default App;
