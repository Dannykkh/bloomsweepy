use bloomsweepy_control::MAX_SEARCH_RESULTS;
use bloomsweepy_core::{
    CleanupCandidate, CleanupCandidateKind, CleanupRootSpec, CleanupScanConfig,
    CleanupScanProgress, CleanupScanReport, DirectoryScanConfig, DirectoryScanProgress,
    DirectoryScanReport, DocumentIndexConfig, DocumentIndexProgress, DocumentIndexReport,
    DocumentIndexStatus, DocumentSearchReport, DocumentSearchRequest, DriveScanConfig,
    DriveScanProgress, DriveScanReport, DuplicateGroup, FileCatalogConfig, FileCatalogProgress,
    FileCatalogReport, FileCatalogSearchReport, FileCatalogSearchRequest, FileCatalogStatus,
    ScanConfig, ScanError, ScanProgress, ScanReport, build_document_index, build_file_catalog,
    clear_file_catalog, document_index_status, file_catalog_status, scan_cleanup_candidates,
    scan_directory_level, scan_drive, scan_path, search_document_index_with_cancellation,
    search_file_catalog_with_cancellation,
};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use sysinfo::Disks;
use tauri::{AppHandle, Emitter, Manager, State};

mod action_recovery;
mod control_server;
mod system_inventory;
mod trash_actions;
#[cfg(windows)]
mod windows_tray;

use system_inventory::{
    InstalledAppInventory, RegistryResidueInventory, installed_app_inventory_with_cancellation,
    registry_residue_inventory_with_cancellation,
};

#[derive(Default)]
pub(crate) struct ScanRuntime {
    active: Mutex<Option<ActiveRuntimeWork>>,
    closed: AtomicBool,
}

struct ActiveRuntimeWork {
    operation_id: Option<String>,
    cancellation: Arc<AtomicBool>,
    phase: RuntimePhase,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RuntimePhase {
    Running,
    Committing,
}

pub(crate) enum ControlCancelOutcome {
    Requested,
    TooLate,
    NotActive,
}

impl ScanRuntime {
    pub(crate) fn begin(&self) -> Result<Arc<AtomicBool>, String> {
        self.begin_inner(None)
    }

    pub(crate) fn begin_control(&self, operation_id: String) -> Result<Arc<AtomicBool>, String> {
        self.begin_inner(Some(operation_id))
    }

    fn begin_inner(&self, operation_id: Option<String>) -> Result<Arc<AtomicBool>, String> {
        if self.closed.load(Ordering::Acquire) {
            return Err("앱을 종료하고 있어 새 작업을 시작할 수 없습니다".to_owned());
        }
        let mut active = self
            .active
            .lock()
            .map_err(|_| "작업 상태를 잠글 수 없습니다".to_owned())?;
        if self.closed.load(Ordering::Acquire) {
            return Err("앱을 종료하고 있어 새 작업을 시작할 수 없습니다".to_owned());
        }
        if active.is_some() {
            return Err("이미 스캔 또는 정리 작업이 진행 중입니다".to_owned());
        }

        let cancellation = Arc::new(AtomicBool::new(false));
        *active = Some(ActiveRuntimeWork {
            operation_id,
            cancellation: Arc::clone(&cancellation),
            phase: RuntimePhase::Running,
        });
        Ok(cancellation)
    }

    pub(crate) fn finish(&self) {
        if let Ok(mut active) = self.active.lock() {
            *active = None;
        }
    }

    fn cancel(&self) -> Result<bool, String> {
        let active = self
            .active
            .lock()
            .map_err(|_| "작업 상태를 잠글 수 없습니다".to_owned())?;
        if let Some(work) = active.as_ref()
            && work.phase == RuntimePhase::Running
        {
            work.cancellation.store(true, Ordering::Release);
            return Ok(true);
        }
        Ok(false)
    }

    fn close_and_cancel(&self) {
        self.closed.store(true, Ordering::Release);
        if let Ok(active) = self.active.lock()
            && let Some(work) = active.as_ref()
        {
            work.cancellation.store(true, Ordering::Release);
        }
    }

    pub(crate) fn cancel_control(
        &self,
        operation_id: &str,
    ) -> Result<ControlCancelOutcome, String> {
        let active = self
            .active
            .lock()
            .map_err(|_| "작업 상태를 잠글 수 없습니다".to_owned())?;
        let Some(work) = active.as_ref() else {
            return Ok(ControlCancelOutcome::NotActive);
        };
        if work.operation_id.as_deref() != Some(operation_id) {
            return Ok(ControlCancelOutcome::NotActive);
        }
        if work.phase == RuntimePhase::Committing {
            return Ok(ControlCancelOutcome::TooLate);
        }
        work.cancellation.store(true, Ordering::Release);
        Ok(ControlCancelOutcome::Requested)
    }

    fn begin_commit(&self) -> Result<bool, String> {
        self.begin_commit_inner(None)
    }

    pub(crate) fn begin_control_commit(&self, operation_id: &str) -> Result<bool, String> {
        self.begin_commit_inner(Some(operation_id))
    }

    fn begin_commit_inner(&self, operation_id: Option<&str>) -> Result<bool, String> {
        let mut active = self
            .active
            .lock()
            .map_err(|_| "작업 상태를 잠글 수 없습니다".to_owned())?;
        let work = active
            .as_mut()
            .ok_or_else(|| "진행 중인 작업을 찾을 수 없습니다".to_owned())?;
        if work.operation_id.as_deref() != operation_id {
            return Err("작업 번호가 현재 검사와 일치하지 않습니다".to_owned());
        }
        if work.cancellation.load(Ordering::Acquire) {
            return Ok(false);
        }
        work.phase = RuntimePhase::Committing;
        Ok(true)
    }

    pub(crate) fn finish_control(&self, operation_id: &str) {
        if let Ok(mut active) = self.active.lock()
            && active
                .as_ref()
                .is_some_and(|work| work.operation_id.as_deref() == Some(operation_id))
        {
            *active = None;
        }
    }

    fn active_work_marker(&self, operation_id: Option<&str>) -> Option<Arc<AtomicBool>> {
        self.active.lock().ok().and_then(|active| {
            active.as_ref().and_then(|work| {
                (work.operation_id.as_deref() == operation_id)
                    .then(|| Arc::clone(&work.cancellation))
            })
        })
    }

    fn owns_active_work(
        &self,
        operation_id: Option<&str>,
        work_marker: Option<&Arc<AtomicBool>>,
    ) -> bool {
        let Some(work_marker) = work_marker else {
            return false;
        };
        self.active
            .lock()
            .map(|active| {
                active.as_ref().is_some_and(|work| {
                    work.operation_id.as_deref() == operation_id
                        && Arc::ptr_eq(&work.cancellation, work_marker)
                })
            })
            .unwrap_or(false)
    }

    fn is_running(&self) -> bool {
        self.active
            .lock()
            .map(|active| active.is_some())
            .unwrap_or(false)
    }
}

#[derive(Clone)]
pub(crate) struct ScanCompletionGuard {
    _lease: Arc<ScanCompletionLease>,
}

struct ScanCompletionLease {
    target: ScanCompletionTarget,
}

enum ScanCompletionTarget {
    App {
        app: AppHandle,
        operation_id: Option<String>,
        work_marker: Option<Arc<AtomicBool>>,
    },
    #[cfg(test)]
    Runtime {
        runtime: Arc<ScanRuntime>,
        operation_id: Option<String>,
        work_marker: Option<Arc<AtomicBool>>,
    },
}

impl ScanCompletionGuard {
    pub(crate) fn new(app: AppHandle) -> Self {
        let work_marker = app.state::<ScanRuntime>().active_work_marker(None);
        #[cfg(windows)]
        if work_marker.is_some() {
            windows_tray::set_busy(&app, true);
        }
        Self {
            _lease: Arc::new(ScanCompletionLease {
                target: ScanCompletionTarget::App {
                    app,
                    operation_id: None,
                    work_marker,
                },
            }),
        }
    }

    pub(crate) fn for_control(app: AppHandle, operation_id: String) -> Self {
        let work_marker = app
            .state::<ScanRuntime>()
            .active_work_marker(Some(&operation_id));
        #[cfg(windows)]
        if work_marker.is_some() {
            windows_tray::set_busy(&app, true);
        }
        Self {
            _lease: Arc::new(ScanCompletionLease {
                target: ScanCompletionTarget::App {
                    app,
                    operation_id: Some(operation_id),
                    work_marker,
                },
            }),
        }
    }

    #[cfg(test)]
    fn for_runtime(runtime: Arc<ScanRuntime>, operation_id: Option<String>) -> Self {
        let work_marker = runtime.active_work_marker(operation_id.as_deref());
        Self {
            _lease: Arc::new(ScanCompletionLease {
                target: ScanCompletionTarget::Runtime {
                    runtime,
                    operation_id,
                    work_marker,
                },
            }),
        }
    }
}

#[derive(Default)]
pub(crate) struct StoredReports {
    scan: Mutex<Option<StoredScanSnapshot>>,
    next_scan_generation: AtomicU64,
    cleanup: Mutex<Option<CleanupActionReport>>,
}

#[derive(Clone)]
pub(crate) struct StoredScanSnapshot {
    pub(crate) generation: u64,
    pub(crate) report: Arc<ScanReport>,
}

#[derive(Clone)]
pub(crate) struct DuplicateActionReport {
    pub(crate) root: String,
    pub(crate) duplicate_groups: Vec<DuplicateGroup>,
}

#[derive(Clone)]
pub(crate) struct CleanupActionReport {
    pub(crate) candidates: Vec<CleanupCandidate>,
}

impl StoredReports {
    fn replace_scan(&self, report: ScanReport) -> Result<StoredScanSnapshot, String> {
        let mut stored = self
            .scan
            .lock()
            .map_err(|_| "중복 스캔 결과를 잠글 수 없습니다".to_owned())?;
        let generation = self
            .next_scan_generation
            .fetch_add(1, Ordering::AcqRel)
            .saturating_add(1);
        let snapshot = StoredScanSnapshot {
            generation,
            report: Arc::new(report),
        };
        *stored = Some(snapshot.clone());
        Ok(snapshot)
    }

    fn replace_cleanup(&self, report: &CleanupScanReport) -> Result<(), String> {
        *self
            .cleanup
            .lock()
            .map_err(|_| "정리 후보 결과를 잠글 수 없습니다".to_owned())? =
            Some(CleanupActionReport {
                candidates: report.candidates.clone(),
            });
        Ok(())
    }

    pub(crate) fn scan_report(&self) -> Result<DuplicateActionReport, String> {
        let snapshot = self
            .scan
            .lock()
            .map_err(|_| "중복 스캔 결과를 잠글 수 없습니다".to_owned())?
            .clone()
            .ok_or_else(|| "서버에 보관된 중복 스캔 결과가 없습니다. 다시 스캔하세요".to_owned())?;
        Ok(DuplicateActionReport {
            root: snapshot.report.root.clone(),
            duplicate_groups: snapshot.report.duplicate_groups.clone(),
        })
    }

    fn scan_snapshot(&self, generation: u64) -> Result<StoredScanSnapshot, String> {
        let snapshot = self
            .scan
            .lock()
            .map_err(|_| "스캔 결과를 잠글 수 없습니다".to_owned())?
            .clone()
            .ok_or_else(|| "표시할 스캔 결과가 없습니다".to_owned())?;
        if snapshot.generation != generation {
            return Err("스캔 결과가 새 검사로 바뀌었습니다".to_owned());
        }
        Ok(snapshot)
    }

    pub(crate) fn cleanup_report(&self) -> Result<CleanupActionReport, String> {
        self.cleanup
            .lock()
            .map_err(|_| "정리 후보 결과를 잠글 수 없습니다".to_owned())?
            .clone()
            .ok_or_else(|| "서버에 보관된 정리 후보 결과가 없습니다. 다시 스캔하세요".to_owned())
    }

    pub(crate) fn clear_scan(&self) -> Result<(), String> {
        *self
            .scan
            .lock()
            .map_err(|_| "중복 스캔 결과를 잠글 수 없습니다".to_owned())? = None;
        Ok(())
    }

    fn clear_cleanup(&self) -> Result<(), String> {
        *self
            .cleanup
            .lock()
            .map_err(|_| "정리 후보 결과를 잠글 수 없습니다".to_owned())? = None;
        Ok(())
    }

    pub(crate) fn clear_all(&self) -> Result<(), String> {
        self.clear_scan()?;
        self.clear_cleanup()
    }
}

impl Drop for ScanCompletionLease {
    fn drop(&mut self) {
        match &self.target {
            ScanCompletionTarget::App {
                app,
                operation_id,
                work_marker,
            } => {
                let runtime = app.state::<ScanRuntime>();
                if runtime.owns_active_work(operation_id.as_deref(), work_marker.as_ref()) {
                    #[cfg(windows)]
                    windows_tray::set_busy(app, false);
                    if let Some(operation_id) = operation_id.as_deref() {
                        runtime.finish_control(operation_id);
                    } else {
                        runtime.finish();
                    }
                }
            }
            #[cfg(test)]
            ScanCompletionTarget::Runtime {
                runtime,
                operation_id,
                work_marker,
            } => {
                if runtime.owns_active_work(operation_id.as_deref(), work_marker.as_ref()) {
                    if let Some(operation_id) = operation_id.as_deref() {
                        runtime.finish_control(operation_id);
                    } else {
                        runtime.finish();
                    }
                }
            }
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SystemOverview {
    platform: &'static str,
    volumes: Vec<VolumeInfo>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VolumeInfo {
    name: String,
    mount_point: String,
    file_system: String,
    total_bytes: u64,
    available_bytes: u64,
    removable: bool,
    read_only: bool,
    is_system: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DriveScanResult {
    #[serde(flatten)]
    report: DriveScanReport,
    installed_apps: InstalledAppInventory,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanReportSnapshot {
    scan_generation: u64,
    report: ScanReport,
}

#[derive(Clone)]
pub(crate) enum ScanProgressTarget {
    App,
    Control { operation_id: String },
}

#[derive(Debug)]
pub(crate) enum ScanExecutionError {
    Cancelled,
    Failed(String),
}

impl ScanExecutionError {
    pub(crate) fn message(&self) -> String {
        match self {
            Self::Cancelled => "scan was cancelled".to_owned(),
            Self::Failed(message) => message.clone(),
        }
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        matches!(self, Self::Cancelled)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SystemCleanupResult {
    #[serde(flatten)]
    report: CleanupScanReport,
    registry_residues: RegistryResidueInventory,
}

#[tauri::command]
async fn get_system_overview() -> Result<SystemOverview, String> {
    tauri::async_runtime::spawn_blocking(collect_system_overview)
        .await
        .map_err(|error| format!("드라이브 정보를 조회하지 못했습니다: {error}"))
}

fn collect_system_overview() -> SystemOverview {
    let disks = Disks::new_with_refreshed_list();
    let system_root = system_volume_root();
    let mut volumes: Vec<VolumeInfo> = disks
        .list()
        .iter()
        .map(|disk| VolumeInfo {
            name: disk.name().to_string_lossy().into_owned(),
            mount_point: disk.mount_point().to_string_lossy().into_owned(),
            file_system: disk.file_system().to_string_lossy().into_owned(),
            total_bytes: disk.total_space(),
            available_bytes: disk.available_space(),
            removable: disk.is_removable(),
            read_only: disk.is_read_only(),
            is_system: system_root
                .as_deref()
                .is_some_and(|root| same_mount_point(disk.mount_point(), root)),
        })
        .collect();
    volumes.sort_unstable_by(|left, right| {
        right
            .is_system
            .cmp(&left.is_system)
            .then_with(|| right.total_bytes.cmp(&left.total_bytes))
            .then_with(|| left.mount_point.cmp(&right.mount_point))
    });

    SystemOverview {
        platform: std::env::consts::OS,
        volumes,
    }
}

#[cfg(windows)]
fn same_mount_point(left: &Path, right: &Path) -> bool {
    let normalize = |path: &Path| {
        path.to_string_lossy()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_lowercase()
    };
    normalize(left) == normalize(right)
}

#[cfg(not(windows))]
fn same_mount_point(left: &Path, right: &Path) -> bool {
    left.components().eq(right.components())
}

#[cfg(windows)]
fn system_volume_root() -> Option<PathBuf> {
    std::env::var_os("SystemRoot")
        .and_then(|value| PathBuf::from(value).parent().map(Path::to_path_buf))
}

#[cfg(not(windows))]
fn system_volume_root() -> Option<PathBuf> {
    Some(PathBuf::from("/"))
}

#[tauri::command]
fn is_scan_running(state: State<'_, ScanRuntime>) -> bool {
    state.is_running()
}

#[tauri::command]
fn cancel_scan(state: State<'_, ScanRuntime>) -> Result<bool, String> {
    state.cancel()
}

#[tauri::command]
async fn start_scan(
    app: AppHandle,
    state: State<'_, ScanRuntime>,
    reports: State<'_, StoredReports>,
    root: String,
    config: Option<ScanConfig>,
) -> Result<ScanReport, String> {
    let cancellation = state.begin()?;
    let completion = ScanCompletionGuard::new(app.clone());
    reports.clear_scan()?;
    let result = execute_reserved_scan(
        app,
        PathBuf::from(root),
        config.unwrap_or_default(),
        cancellation,
        completion.clone(),
        ScanProgressTarget::App,
    )
    .await;
    drop(completion);
    let snapshot = result.map_err(|error| error.message())?;
    Ok((*snapshot.report).clone())
}

pub(crate) async fn execute_reserved_scan(
    app: AppHandle,
    root: PathBuf,
    config: ScanConfig,
    cancellation: Arc<AtomicBool>,
    completion: ScanCompletionGuard,
    progress_target: ScanProgressTarget,
) -> Result<StoredScanSnapshot, ScanExecutionError> {
    let app_for_worker = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _completion = completion;
        let cancellation_for_worker = Arc::clone(&cancellation);
        let progress_app = app_for_worker.clone();
        let progress_target_for_worker = progress_target.clone();
        let report = scan_path(
            root,
            config,
            move |progress: ScanProgress| match &progress_target_for_worker {
                ScanProgressTarget::App => {
                    let _ = progress_app.emit("scan-progress", progress);
                }
                ScanProgressTarget::Control { operation_id } => {
                    control_server::record_scan_progress(&progress_app, operation_id, progress);
                }
            },
            move || cancellation_for_worker.load(Ordering::Acquire),
        )
        .map_err(|error| match error {
            ScanError::Cancelled => ScanExecutionError::Cancelled,
            error => ScanExecutionError::Failed(error.to_string()),
        })?;

        let can_commit = match &progress_target {
            ScanProgressTarget::App => app_for_worker.state::<ScanRuntime>().begin_commit(),
            ScanProgressTarget::Control { operation_id } => app_for_worker
                .state::<ScanRuntime>()
                .begin_control_commit(operation_id),
        }
        .map_err(ScanExecutionError::Failed)?;
        if !can_commit {
            return Err(ScanExecutionError::Cancelled);
        }

        app_for_worker
            .state::<StoredReports>()
            .replace_scan(report)
            .map_err(ScanExecutionError::Failed)
    })
    .await
    .map_err(|error| {
        ScanExecutionError::Failed(format!("스캔 작업을 실행하지 못했습니다: {error}"))
    })?
}

#[tauri::command]
fn get_scan_report_snapshot(
    reports: State<'_, StoredReports>,
    scan_generation: u64,
) -> Result<ScanReportSnapshot, String> {
    let snapshot = reports.scan_snapshot(scan_generation)?;
    Ok(ScanReportSnapshot {
        scan_generation: snapshot.generation,
        report: (*snapshot.report).clone(),
    })
}

#[tauri::command]
async fn start_drive_scan(
    app: AppHandle,
    state: State<'_, ScanRuntime>,
    root: String,
    config: Option<DriveScanConfig>,
) -> Result<DriveScanResult, String> {
    let cancellation = state.begin()?;
    let _completion = ScanCompletionGuard::new(app.clone());
    let cancellation_for_scan = Arc::clone(&cancellation);
    let app_for_worker = app.clone();

    let worker_result = tauri::async_runtime::spawn_blocking(move || {
        let report = scan_drive(
            root,
            config.unwrap_or_default(),
            move |progress: DriveScanProgress| {
                let _ = app_for_worker.emit("drive-scan-progress", progress);
            },
            move || cancellation_for_scan.load(Ordering::Acquire),
        )?;
        let installed_apps =
            installed_app_inventory_with_cancellation(|| cancellation.load(Ordering::Acquire))
                .map_err(|_| ScanError::Cancelled)?;
        Ok::<DriveScanResult, bloomsweepy_core::ScanError>(DriveScanResult {
            report,
            installed_apps,
        })
    })
    .await;

    let result = worker_result
        .map_err(|error| format!("드라이브 스캔 작업을 실행하지 못했습니다: {error}"))?
        .map_err(|error| error.to_string())?;
    Ok(result)
}

#[tauri::command]
async fn start_directory_scan(
    app: AppHandle,
    state: State<'_, ScanRuntime>,
    root: String,
    config: Option<DirectoryScanConfig>,
) -> Result<DirectoryScanReport, String> {
    let cancellation = state.begin()?;
    let _completion = ScanCompletionGuard::new(app.clone());
    let cancellation_for_worker = Arc::clone(&cancellation);
    let app_for_worker = app.clone();

    let worker_result = tauri::async_runtime::spawn_blocking(move || {
        scan_directory_level(
            root,
            config.unwrap_or_default(),
            move |progress: DirectoryScanProgress| {
                let _ = app_for_worker.emit("directory-scan-progress", progress);
            },
            move || cancellation_for_worker.load(Ordering::Acquire),
        )
    })
    .await;

    let result =
        worker_result.map_err(|error| format!("폴더 지도 스캔을 실행하지 못했습니다: {error}"))?;
    result.map_err(|error| error.to_string())
}

#[tauri::command]
async fn start_cleanup_scan(
    app: AppHandle,
    state: State<'_, ScanRuntime>,
    reports: State<'_, StoredReports>,
) -> Result<SystemCleanupResult, String> {
    let cancellation = state.begin()?;
    let _completion = ScanCompletionGuard::new(app.clone());
    reports.clear_cleanup()?;
    let cancellation_for_scan = Arc::clone(&cancellation);
    let app_for_worker = app.clone();

    let worker_result = tauri::async_runtime::spawn_blocking(move || {
        let installed_apps =
            installed_app_inventory_with_cancellation(|| cancellation.load(Ordering::Acquire))
                .map_err(|_| ScanError::Cancelled)?;
        let config = cleanup_scan_config(&installed_apps);
        let report = scan_cleanup_candidates(
            config,
            move |progress: CleanupScanProgress| {
                let _ = app_for_worker.emit("cleanup-scan-progress", progress);
            },
            move || cancellation_for_scan.load(Ordering::Acquire),
        )?;
        let registry_residues =
            registry_residue_inventory_with_cancellation(|| cancellation.load(Ordering::Acquire))
                .map_err(|_| ScanError::Cancelled)?;
        Ok::<SystemCleanupResult, bloomsweepy_core::ScanError>(SystemCleanupResult {
            report,
            registry_residues,
        })
    })
    .await;

    let result = worker_result
        .map_err(|error| format!("정리 후보 스캔을 실행하지 못했습니다: {error}"))?
        .map_err(|error| error.to_string())?;
    reports.replace_cleanup(&result.report)?;
    Ok(result)
}

#[tauri::command]
async fn get_document_index_status(app: AppHandle) -> Result<Option<DocumentIndexStatus>, String> {
    let database_path = document_index_path(&app)?;
    tauri::async_runtime::spawn_blocking(move || document_index_status(database_path))
        .await
        .map_err(|error| format!("문서 색인 상태를 조회하지 못했습니다: {error}"))?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn start_document_index(
    app: AppHandle,
    state: State<'_, ScanRuntime>,
    root: String,
    config: Option<DocumentIndexConfig>,
) -> Result<DocumentIndexReport, String> {
    let cancellation = state.begin()?;
    let _completion = ScanCompletionGuard::new(app.clone());
    let database_path = document_index_path(&app)?;
    let cancellation_for_worker = Arc::clone(&cancellation);
    let app_for_worker = app.clone();

    let worker_result = tauri::async_runtime::spawn_blocking(move || {
        build_document_index(
            root,
            database_path,
            config.unwrap_or_default(),
            move |progress: DocumentIndexProgress| {
                let _ = app_for_worker.emit("document-index-progress", progress);
            },
            move || cancellation_for_worker.load(Ordering::Acquire),
        )
    })
    .await;

    worker_result
        .map_err(|error| format!("문서 색인 작업을 실행하지 못했습니다: {error}"))?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn search_documents(
    app: AppHandle,
    state: State<'_, ScanRuntime>,
    request: DocumentSearchRequest,
) -> Result<DocumentSearchReport, String> {
    validate_search_result_limit(request.max_results)?;
    let cancellation = state.begin()?;
    let _completion = ScanCompletionGuard::new(app.clone());
    let database_path = document_index_path(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        search_document_index_with_cancellation(database_path, request, move || {
            cancellation.load(Ordering::Acquire)
        })
    })
    .await
    .map_err(|error| format!("문서 검색을 실행하지 못했습니다: {error}"))?
    .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_file_catalog_status(app: AppHandle) -> Result<Option<FileCatalogStatus>, String> {
    let database_path = file_catalog_path(&app)?;
    tauri::async_runtime::spawn_blocking(move || file_catalog_status(database_path))
        .await
        .map_err(|error| format!("파일 카탈로그 상태를 조회하지 못했습니다: {error}"))?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn start_file_catalog_build(
    app: AppHandle,
    state: State<'_, ScanRuntime>,
    root: String,
    config: Option<FileCatalogConfig>,
) -> Result<FileCatalogReport, String> {
    let cancellation = state.begin()?;
    let _completion = ScanCompletionGuard::new(app.clone());
    let database_path = file_catalog_path(&app)?;
    let cancellation_for_worker = Arc::clone(&cancellation);
    let app_for_worker = app.clone();

    let worker_result = tauri::async_runtime::spawn_blocking(move || {
        build_file_catalog(
            root,
            database_path,
            config.unwrap_or_default(),
            move |progress: FileCatalogProgress| {
                let _ = app_for_worker.emit("file-catalog-progress", progress);
            },
            move || cancellation_for_worker.load(Ordering::Acquire),
        )
    })
    .await;

    worker_result
        .map_err(|error| format!("파일 카탈로그 생성을 실행하지 못했습니다: {error}"))?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn search_file_catalog_entries(
    app: AppHandle,
    state: State<'_, ScanRuntime>,
    request: FileCatalogSearchRequest,
) -> Result<FileCatalogSearchReport, String> {
    validate_search_result_limit(request.max_results)?;
    let cancellation = state.begin()?;
    let _completion = ScanCompletionGuard::new(app.clone());
    let database_path = file_catalog_path(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        search_file_catalog_with_cancellation(database_path, request, move || {
            cancellation.load(Ordering::Acquire)
        })
    })
    .await
    .map_err(|error| format!("파일 검색을 실행하지 못했습니다: {error}"))?
    .map_err(|error| error.to_string())
}

pub(crate) fn validate_search_result_limit(max_results: usize) -> Result<(), String> {
    if !(1..=MAX_SEARCH_RESULTS).contains(&max_results) {
        return Err(format!(
            "검색 결과 수는 1개 이상 {MAX_SEARCH_RESULTS}개 이하여야 합니다"
        ));
    }
    Ok(())
}

#[tauri::command]
async fn clear_file_catalog_index(
    app: AppHandle,
    state: State<'_, ScanRuntime>,
) -> Result<bool, String> {
    state.begin()?;
    let _completion = ScanCompletionGuard::new(app.clone());
    let database_path = file_catalog_path(&app)?;
    tauri::async_runtime::spawn_blocking(move || clear_file_catalog(database_path))
        .await
        .map_err(|error| format!("파일 카탈로그를 비우지 못했습니다: {error}"))?
        .map_err(|error| error.to_string())
}

fn document_index_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_cache_dir()
        .map(|directory| directory.join("document-search-v1.sqlite3"))
        .map_err(|error| format!("문서 색인 저장 위치를 찾지 못했습니다: {error}"))
}

fn file_catalog_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_cache_dir()
        .map(|directory| directory.join("file-catalog-v1.sqlite3"))
        .map_err(|error| format!("파일 카탈로그 저장 위치를 찾지 못했습니다: {error}"))
}

fn cleanup_scan_config(installed_apps: &InstalledAppInventory) -> CleanupScanConfig {
    let installed_identity_tokens = installed_apps
        .applications
        .iter()
        .flat_map(|application| {
            let mut tokens = vec![application.display_name.clone()];
            if let Some(publisher) = application.publisher.as_ref() {
                tokens.push(publisher.clone());
            }
            if let Some(location) = application.install_location.as_ref()
                && let Some(name) = Path::new(location).file_name()
            {
                tokens.push(name.to_string_lossy().into_owned());
            }
            tokens.extend(application.cleanup_identity_tokens.iter().cloned());
            tokens
        })
        .collect();

    let mut roots = Vec::new();
    roots.push(CleanupRootSpec::new(
        std::env::temp_dir(),
        "사용자 임시 폴더",
        CleanupCandidateKind::TemporaryEntry,
        Duration::from_secs(7 * 86_400),
    ));

    #[cfg(windows)]
    {
        const WINDOWS_APPDATA_PROTECTED: [&str; 14] = [
            "ConnectedDevicesPlatform",
            "CrashDumps",
            "D3DSCache",
            "ElevatedDiagnostics",
            "Microsoft",
            "Packages",
            "PeerDistRepub",
            "PlaceholderTileLogoFolder",
            "Programs",
            "Publishers",
            "Temp",
            "TileDataLayer",
            "VirtualStore",
            "Windows",
        ];
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            roots.push(
                CleanupRootSpec::new(
                    PathBuf::from(local_app_data),
                    "Local AppData",
                    CleanupCandidateKind::AppDataDirectory,
                    Duration::from_secs(90 * 86_400),
                )
                .with_protected_names(WINDOWS_APPDATA_PROTECTED),
            );
        }
        if let Some(roaming_app_data) = std::env::var_os("APPDATA") {
            roots.push(
                CleanupRootSpec::new(
                    PathBuf::from(roaming_app_data),
                    "Roaming AppData",
                    CleanupCandidateKind::AppDataDirectory,
                    Duration::from_secs(90 * 86_400),
                )
                .with_protected_names(["Microsoft", "Windows"]),
            );
        }
    }

    #[cfg(not(windows))]
    if let Some(cache_dir) = dirs::cache_dir() {
        roots.push(CleanupRootSpec::new(
            cache_dir,
            "사용자 캐시",
            CleanupCandidateKind::CacheDirectory,
            Duration::from_secs(30 * 86_400),
        ));
    }

    CleanupScanConfig {
        roots,
        installed_identity_tokens,
        ..CleanupScanConfig::default()
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .manage(ScanRuntime::default())
        .manage(StoredReports::default())
        .manage(control_server::ControlStatusStore::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            #[cfg(windows)]
            if let Err(error) = windows_tray::setup(app) {
                windows_tray::log_setup_failure(&error);
            }
            let app_handle = app.handle().clone();
            match control_server::start(app_handle.clone()) {
                Ok(server) => {
                    app.manage(server);
                }
                Err(error) => {
                    control_server::record_start_failure(&app_handle, error);
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            control_server::get_control_status,
            control_server::configure_control_search_access,
            control_server::configure_control_scan_access,
            get_system_overview,
            is_scan_running,
            start_scan,
            get_scan_report_snapshot,
            start_drive_scan,
            start_directory_scan,
            start_cleanup_scan,
            get_document_index_status,
            start_document_index,
            search_documents,
            get_file_catalog_status,
            start_file_catalog_build,
            search_file_catalog_entries,
            clear_file_catalog_index,
            action_recovery::get_action_recovery_status,
            action_recovery::open_system_trash,
            trash_actions::trash_duplicate_files,
            trash_actions::trash_cleanup_candidates,
            cancel_scan
        ])
        .build(tauri::generate_context!())
        .expect("failed to build BroomSweepy");

    app.run(|app_handle, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            if let Some(runtime) = app_handle.try_state::<ScanRuntime>() {
                runtime.close_and_cancel();
            }
            if let Some(server) = app_handle.try_state::<control_server::ControlServer>() {
                server.shutdown();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::thread;
    use std::time::{Duration, Instant};

    #[cfg(windows)]
    #[test]
    fn windows_mount_points_ignore_case_and_slash_direction() {
        assert!(same_mount_point(Path::new(r"C:\"), Path::new("c:/")));
        assert!(!same_mount_point(Path::new(r"C:\"), Path::new(r"D:\")));
    }

    #[cfg(not(windows))]
    #[test]
    fn unix_mount_points_preserve_case_and_ignore_trailing_separators() {
        assert!(same_mount_point(
            Path::new("/Volumes/Data/"),
            Path::new("/Volumes/Data")
        ));
        assert!(!same_mount_point(
            Path::new("/Volumes/Data"),
            Path::new("/Volumes/data")
        ));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn mac_cleanup_defaults_exclude_sensitive_application_data_roots() {
        let inventory = InstalledAppInventory {
            supported: true,
            source: "macApplicationBundles",
            estimated_total_bytes: 0,
            applications: vec![system_inventory::InstalledApplication {
                display_name: "Sample App".to_owned(),
                display_version: Some("1.0".to_owned()),
                publisher: None,
                install_location: Some("/Applications/Sample.app".to_owned()),
                estimated_bytes: None,
                registry_scope: "machine",
                cleanup_identity_tokens: vec!["com.example.sample".to_owned()],
            }],
            issues: Vec::new(),
        };

        let config = cleanup_scan_config(&inventory);

        assert!(
            config
                .roots
                .iter()
                .all(|root| root.kind != CleanupCandidateKind::AppDataDirectory)
        );
        assert!(config.roots.iter().all(|root| {
            root.label != "Application Support" && root.label != "앱 컨테이너"
        }));
    }

    #[test]
    fn scan_runtime_rejects_overlap_and_recovers_after_finish() {
        let runtime = ScanRuntime::default();
        let first = runtime.begin().expect("begin first scan");

        assert!(runtime.is_running());
        assert!(runtime.begin().is_err());

        runtime.finish();
        assert!(!runtime.is_running());
        assert!(runtime.begin().is_ok());
        drop(first);
    }

    #[test]
    fn scan_runtime_cancel_sets_the_active_token() {
        let runtime = ScanRuntime::default();
        let cancellation = runtime.begin().expect("begin scan");

        assert!(runtime.cancel().expect("request cancellation"));
        assert!(cancellation.load(Ordering::Acquire));

        runtime.finish();
        assert!(!runtime.cancel().expect("cancel with no active scan"));
    }

    #[test]
    fn closing_runtime_cancels_active_work_and_rejects_new_work() {
        let runtime = ScanRuntime::default();
        let cancellation = runtime.begin().expect("begin scan");

        runtime.close_and_cancel();

        assert!(cancellation.load(Ordering::Acquire));
        assert!(runtime.begin().is_err());
        runtime.finish();
        assert!(runtime.begin().is_err());
    }

    #[test]
    fn blocking_worker_keeps_status_overlap_and_cancel_responsive() {
        const CONTROL_PLANE_LIMIT: Duration = Duration::from_millis(250);

        let runtime = Arc::new(ScanRuntime::default());
        let cancellation = runtime.begin().expect("begin blocking work");
        let (started_tx, started_rx) = mpsc::sync_channel(0);
        let worker = tauri::async_runtime::spawn_blocking(move || {
            started_tx.send(()).expect("signal worker start");
            let deadline = Instant::now() + Duration::from_secs(2);
            while !cancellation.load(Ordering::Acquire) {
                assert!(Instant::now() < deadline, "worker was not cancelled");
                thread::sleep(Duration::from_millis(1));
            }
        });
        started_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("blocking worker start");

        let started = Instant::now();
        assert!(runtime.is_running());
        assert!(runtime.begin().is_err());
        assert!(runtime.cancel().expect("cancel blocking work"));
        let control_plane_elapsed = started.elapsed();
        println!("runtime control-plane latency: {control_plane_elapsed:?}");
        assert!(
            control_plane_elapsed < CONTROL_PLANE_LIMIT,
            "status, overlap rejection, and cancellation took {:?}",
            control_plane_elapsed
        );

        tauri::async_runtime::block_on(worker).expect("join blocking worker");
        runtime.finish();
        assert!(!runtime.is_running());
        assert!(runtime.begin().is_ok());
    }

    #[test]
    fn search_request_limit_and_runtime_lease_share_one_gate() {
        let runtime = ScanRuntime::default();

        assert!(validate_search_result_limit(1).is_ok());
        assert!(validate_search_result_limit(MAX_SEARCH_RESULTS).is_ok());
        assert!(validate_search_result_limit(0).is_err());
        assert!(validate_search_result_limit(MAX_SEARCH_RESULTS + 1).is_err());

        let _cancellation = runtime.begin().expect("begin work");
        assert!(runtime.begin().is_err());
    }

    #[test]
    fn control_cancel_requires_the_exact_operation_id() {
        let runtime = ScanRuntime::default();
        let cancellation = runtime
            .begin_control("operation-a".to_owned())
            .expect("begin control scan");

        assert!(matches!(
            runtime.cancel_control("operation-b").expect("wrong cancel"),
            ControlCancelOutcome::NotActive
        ));
        assert!(!cancellation.load(Ordering::Acquire));
        assert!(matches!(
            runtime
                .cancel_control("operation-a")
                .expect("matching cancel"),
            ControlCancelOutcome::Requested
        ));
        assert!(cancellation.load(Ordering::Acquire));
    }

    #[test]
    fn control_cancel_cannot_interrupt_the_commit_phase() {
        let runtime = ScanRuntime::default();
        runtime
            .begin_control("operation-a".to_owned())
            .expect("begin control scan");
        assert!(
            runtime
                .begin_control_commit("operation-a")
                .expect("begin commit")
        );
        assert!(matches!(
            runtime.cancel_control("operation-a").expect("late cancel"),
            ControlCancelOutcome::TooLate
        ));
    }

    #[test]
    fn completion_guard_releases_runtime_only_after_the_last_owner_drops() {
        let runtime = Arc::new(ScanRuntime::default());
        runtime
            .begin_control("operation-a".to_owned())
            .expect("begin control scan");
        let terminal_owner =
            ScanCompletionGuard::for_runtime(Arc::clone(&runtime), Some("operation-a".to_owned()));
        let worker_owner = terminal_owner.clone();

        drop(worker_owner);
        assert!(
            runtime.is_running(),
            "the worker finishing must not release the slot before terminal state is recorded"
        );

        drop(terminal_owner);
        assert!(!runtime.is_running());
    }

    #[test]
    fn stale_control_completion_guard_does_not_release_reused_operation_id() {
        let runtime = Arc::new(ScanRuntime::default());
        runtime
            .begin_control("operation-a".to_owned())
            .expect("begin first control scan");
        let stale_guard =
            ScanCompletionGuard::for_runtime(Arc::clone(&runtime), Some("operation-a".to_owned()));
        runtime.finish_control("operation-a");
        runtime
            .begin_control("operation-a".to_owned())
            .expect("begin replacement control scan");
        let replacement_marker = runtime
            .active_work_marker(Some("operation-a"))
            .expect("replacement work marker");

        drop(stale_guard);

        assert!(runtime.is_running());
        assert!(runtime.owns_active_work(Some("operation-a"), Some(&replacement_marker)));
        runtime.finish_control("operation-a");
    }

    #[test]
    fn stored_scan_generation_advances_only_when_a_report_is_saved() {
        let reports = StoredReports::default();
        let report = || ScanReport {
            root: "test-root".to_owned(),
            completed_at_unix_ms: 1,
            duration_ms: 2,
            total_files: 0,
            total_logical_bytes: 0,
            hard_links_skipped: 0,
            hard_link_identity_limit_reached: false,
            unreadable_entries: 0,
            candidate_limit_reached: false,
            large_files: Vec::new(),
            duplicate_groups: Vec::new(),
            duplicate_waste_bytes: 0,
            issues: Vec::new(),
        };

        let first = reports.replace_scan(report()).expect("store first report");
        assert_eq!(first.generation, 1);
        reports.clear_scan().expect("clear report");
        assert!(reports.scan_snapshot(1).is_err());
        let second = reports.replace_scan(report()).expect("store second report");
        assert_eq!(second.generation, 2);
    }
}
