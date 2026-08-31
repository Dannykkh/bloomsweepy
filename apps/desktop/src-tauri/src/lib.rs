use bloomsweepy_core::{
    CleanupCandidate, CleanupCandidateKind, CleanupRootSpec, CleanupScanConfig,
    CleanupScanProgress, CleanupScanReport, DirectoryScanConfig, DirectoryScanProgress,
    DirectoryScanReport, DocumentIndexConfig, DocumentIndexProgress, DocumentIndexReport,
    DocumentIndexStatus, DocumentSearchReport, DocumentSearchRequest, DriveScanConfig,
    DriveScanProgress, DriveScanReport, DuplicateGroup, FileCatalogConfig, FileCatalogProgress,
    FileCatalogReport, FileCatalogSearchReport, FileCatalogSearchRequest, FileCatalogStatus,
    ScanConfig, ScanProgress, ScanReport, build_document_index, build_file_catalog,
    clear_file_catalog, document_index_status, file_catalog_status, scan_cleanup_candidates,
    scan_directory_level, scan_drive, scan_path, search_document_index, search_file_catalog,
};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use sysinfo::Disks;
use tauri::{AppHandle, Emitter, Manager, State};

mod action_recovery;
mod system_inventory;
mod trash_actions;

use system_inventory::{
    InstalledAppInventory, RegistryResidueInventory, installed_app_inventory,
    registry_residue_inventory,
};

#[derive(Default)]
pub(crate) struct ScanRuntime {
    active: Mutex<Option<Arc<AtomicBool>>>,
}

impl ScanRuntime {
    pub(crate) fn begin(&self) -> Result<Arc<AtomicBool>, String> {
        let mut active = self
            .active
            .lock()
            .map_err(|_| "작업 상태를 잠글 수 없습니다".to_owned())?;
        if active.is_some() {
            return Err("이미 스캔 또는 정리 작업이 진행 중입니다".to_owned());
        }

        let cancellation = Arc::new(AtomicBool::new(false));
        *active = Some(Arc::clone(&cancellation));
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
        if let Some(cancellation) = active.as_ref() {
            cancellation.store(true, Ordering::Release);
            return Ok(true);
        }
        Ok(false)
    }

    fn is_running(&self) -> bool {
        self.active
            .lock()
            .map(|active| active.is_some())
            .unwrap_or(false)
    }
}

pub(crate) struct ScanCompletionGuard {
    app: AppHandle,
}

impl ScanCompletionGuard {
    pub(crate) fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

#[derive(Default)]
pub(crate) struct StoredReports {
    scan: Mutex<Option<DuplicateActionReport>>,
    cleanup: Mutex<Option<CleanupActionReport>>,
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
    fn replace_scan(&self, report: &ScanReport) -> Result<(), String> {
        *self
            .scan
            .lock()
            .map_err(|_| "중복 스캔 결과를 잠글 수 없습니다".to_owned())? =
            Some(DuplicateActionReport {
                root: report.root.clone(),
                duplicate_groups: report.duplicate_groups.clone(),
            });
        Ok(())
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
        self.scan
            .lock()
            .map_err(|_| "중복 스캔 결과를 잠글 수 없습니다".to_owned())?
            .clone()
            .ok_or_else(|| "서버에 보관된 중복 스캔 결과가 없습니다. 다시 스캔하세요".to_owned())
    }

    pub(crate) fn cleanup_report(&self) -> Result<CleanupActionReport, String> {
        self.cleanup
            .lock()
            .map_err(|_| "정리 후보 결과를 잠글 수 없습니다".to_owned())?
            .clone()
            .ok_or_else(|| "서버에 보관된 정리 후보 결과가 없습니다. 다시 스캔하세요".to_owned())
    }

    fn clear_scan(&self) -> Result<(), String> {
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

impl Drop for ScanCompletionGuard {
    fn drop(&mut self) {
        self.app.state::<ScanRuntime>().finish();
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

fn same_mount_point(left: &Path, right: &Path) -> bool {
    let normalize = |path: &Path| {
        path.to_string_lossy()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_lowercase()
    };
    normalize(left) == normalize(right)
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
    let _completion = ScanCompletionGuard::new(app.clone());
    reports.clear_scan()?;
    let cancellation_for_worker = Arc::clone(&cancellation);
    let app_for_worker = app.clone();

    let worker_result = tauri::async_runtime::spawn_blocking(move || {
        scan_path(
            root,
            config.unwrap_or_default(),
            move |progress: ScanProgress| {
                let _ = app_for_worker.emit("scan-progress", progress);
            },
            move || cancellation_for_worker.load(Ordering::Acquire),
        )
    })
    .await;

    let result = worker_result
        .map_err(|error| format!("스캔 작업을 실행하지 못했습니다: {error}"))?
        .map_err(|error| error.to_string())?;
    reports.replace_scan(&result)?;
    Ok(result)
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
    let cancellation_for_worker = Arc::clone(&cancellation);
    let app_for_worker = app.clone();

    let worker_result = tauri::async_runtime::spawn_blocking(move || {
        let report = scan_drive(
            root,
            config.unwrap_or_default(),
            move |progress: DriveScanProgress| {
                let _ = app_for_worker.emit("drive-scan-progress", progress);
            },
            move || cancellation_for_worker.load(Ordering::Acquire),
        )?;
        Ok::<DriveScanResult, bloomsweepy_core::ScanError>(DriveScanResult {
            report,
            installed_apps: installed_app_inventory(),
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
    let cancellation_for_worker = Arc::clone(&cancellation);
    let app_for_worker = app.clone();

    let worker_result = tauri::async_runtime::spawn_blocking(move || {
        let installed_apps = installed_app_inventory();
        let config = cleanup_scan_config(&installed_apps);
        let report = scan_cleanup_candidates(
            config,
            move |progress: CleanupScanProgress| {
                let _ = app_for_worker.emit("cleanup-scan-progress", progress);
            },
            move || cancellation_for_worker.load(Ordering::Acquire),
        )?;
        Ok::<SystemCleanupResult, bloomsweepy_core::ScanError>(SystemCleanupResult {
            report,
            registry_residues: registry_residue_inventory(),
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
    if state.is_running() {
        return Err("다른 스캔 또는 문서 색인 작업이 진행 중입니다".to_owned());
    }
    let database_path = document_index_path(&app)?;
    tauri::async_runtime::spawn_blocking(move || search_document_index(database_path, request))
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
    if state.is_running() {
        return Err("다른 스캔 또는 색인 작업이 진행 중입니다".to_owned());
    }
    let database_path = file_catalog_path(&app)?;
    tauri::async_runtime::spawn_blocking(move || search_file_catalog(database_path, request))
        .await
        .map_err(|error| format!("파일 검색을 실행하지 못했습니다: {error}"))?
        .map_err(|error| error.to_string())
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
            tokens
        })
        .collect();

    CleanupScanConfig {
        roots,
        installed_identity_tokens,
        ..CleanupScanConfig::default()
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(ScanRuntime::default())
        .manage(StoredReports::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_system_overview,
            is_scan_running,
            start_scan,
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
        .run(tauri::generate_context!())
        .expect("failed to run BroomSweepy");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::thread;
    use std::time::{Duration, Instant};

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
}
