//! App-only application management. Never expose destructive commands to chat/MCP.
//! OS uninstall UI owns Windows removal. macOS bundle/data policies are independent
//! of general folder cleanup and never execute a discovered uninstaller command.
#[cfg(target_os = "macos")]
use crate::StoredReports;
use crate::{ScanCompletionGuard, ScanRuntime, system_inventory, trash_actions};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "macos")]
use tauri::Manager;
use tauri::{AppHandle, State, WebviewWindow};

#[cfg(any(windows, test))]
const WINDOWS_UNINSTALL_SETTINGS_URI: &str = "ms-settings:appsfeatures";

#[derive(Default)]
pub(crate) struct ApplicationActionsState {
    #[cfg(target_os = "macos")]
    inner: std::sync::Mutex<mac::ActionsInner>,
    /// Refreshing an inventory must not permit retries after an uncertain OS reply.
    #[cfg(target_os = "macos")]
    needs_inspection: AtomicBool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ApplicationInventory {
    platform: &'static str,
    inventory_id: String,
    applications: Vec<ApplicationEntry>,
    issues: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApplicationEntry {
    id: String,
    display_name: String,
    display_version: Option<String>,
    publisher: Option<String>,
    install_location: Option<String>,
    estimated_bytes: Option<u64>,
    removal_mode: &'static str,
    protection_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ApplicationPlan {
    plan_id: String,
    display_name: String,
    path: String,
    expires_at_unix_ms: u64,
    related_data: Vec<DataCandidate>,
    warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DataCandidate {
    id: String,
    path: String,
    kind: &'static str,
    evidence: String,
    estimated_bytes: Option<u64>,
}

#[derive(Deserialize)]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PrepareApplicationRequest {
    inventory_id: String,
    application_id: String,
}

#[derive(Deserialize)]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PrepareApplicationDataRequest {
    inventory_id: String,
    application_id: String,
    candidate_ids: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ConfirmApplicationRequest {
    plan_id: String,
    bundle_only_acknowledged: bool,
    no_uninstaller_acknowledged: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ConfirmApplicationDataRequest {
    plan_id: String,
    related_data_acknowledged: bool,
}

fn opaque_id() -> Result<String, String> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| "앱 확인 번호를 만들지 못했습니다".to_owned())?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn require_main(window: &WebviewWindow) -> Result<(), String> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err("앱의 기본 창에서만 사용할 수 있습니다".to_owned())
    }
}

fn cancelled(cancellation: &AtomicBool) -> Result<(), String> {
    if cancellation.load(Ordering::Acquire) {
        Err("앱 관리 작업이 취소되었습니다".to_owned())
    } else {
        Ok(())
    }
}

#[tauri::command]
pub(crate) async fn get_application_inventory(
    window: WebviewWindow,
    app: AppHandle,
    runtime: State<'_, ScanRuntime>,
) -> Result<ApplicationInventory, String> {
    require_main(&window)?;
    let cancellation = runtime.begin()?;
    let _completion = ScanCompletionGuard::new(app.clone());
    let worker_completion = _completion.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _completion = worker_completion;
        #[cfg(target_os = "macos")]
        {
            *app.state::<ApplicationActionsState>()
                .inner
                .lock()
                .map_err(|_| "앱 목록 상태를 잠그지 못했습니다")? = mac::ActionsInner::default();
        }
        let inventory = system_inventory::installed_app_inventory_with_cancellation(|| {
            cancellation.load(Ordering::Acquire)
        })
        .map_err(|_| "앱 목록 조회가 취소되었습니다".to_owned())?;
        cancelled(&cancellation)?;
        let inventory_id = opaque_id()?;
        #[cfg(target_os = "macos")]
        {
            mac::publish_inventory(&app, inventory, inventory_id, &cancellation)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = app;
            let applications = inventory
                .applications
                .into_iter()
                .map(|entry| {
                    Ok(ApplicationEntry {
                        id: opaque_id()?,
                        display_name: entry.display_name,
                        display_version: entry.display_version,
                        publisher: entry.publisher,
                        install_location: entry.install_location,
                        estimated_bytes: entry.estimated_bytes,
                        removal_mode: if cfg!(windows) {
                            "systemSettings"
                        } else {
                            "protected"
                        },
                        protection_reason: if cfg!(windows) {
                            None
                        } else {
                            Some("이 운영체제의 앱 제거는 지원하지 않습니다".to_owned())
                        },
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            Ok(ApplicationInventory {
                platform: if cfg!(windows) {
                    "windows"
                } else {
                    "unsupported"
                },
                inventory_id,
                applications,
                issues: inventory.issues,
            })
        }
    })
    .await
    .map_err(|error| format!("앱 목록 조회가 중단됐습니다: {error}"))?
}

#[tauri::command]
pub(crate) async fn prepare_application_trash(
    window: WebviewWindow,
    app: AppHandle,
    runtime: State<'_, ScanRuntime>,
    request: PrepareApplicationRequest,
) -> Result<ApplicationPlan, String> {
    require_main(&window)?;
    prepare(app, runtime.inner(), request, None).await
}

#[tauri::command]
pub(crate) async fn prepare_application_data_trash(
    window: WebviewWindow,
    app: AppHandle,
    runtime: State<'_, ScanRuntime>,
    request: PrepareApplicationDataRequest,
) -> Result<ApplicationPlan, String> {
    require_main(&window)?;
    prepare(
        app,
        runtime.inner(),
        PrepareApplicationRequest {
            inventory_id: request.inventory_id,
            application_id: request.application_id,
        },
        Some(request.candidate_ids),
    )
    .await
}

async fn prepare(
    app: AppHandle,
    runtime: &ScanRuntime,
    request: PrepareApplicationRequest,
    selected: Option<Vec<String>>,
) -> Result<ApplicationPlan, String> {
    if !cfg!(target_os = "macos") {
        return Err("이 운영체제에서는 정식 앱 제거 화면을 사용하세요".to_owned());
    }
    #[cfg(target_os = "macos")]
    mac::require_confirmed_previous_operation(&app)?;
    let cancellation = runtime.begin()?;
    let _completion = ScanCompletionGuard::new(app.clone());
    let worker_completion = _completion.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _completion = worker_completion;
        #[cfg(target_os = "macos")]
        {
            mac::require_confirmed_previous_operation(&app)?;
            mac::prepare(&app, request, selected, &cancellation)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (app, request, selected, cancellation);
            Err("지원하지 않는 앱 정리 작업입니다".to_owned())
        }
    })
    .await
    .map_err(|error| format!("앱 정리 검토가 중단됐습니다: {error}"))?
}

#[tauri::command]
pub(crate) async fn confirm_application_trash(
    window: WebviewWindow,
    app: AppHandle,
    runtime: State<'_, ScanRuntime>,
    request: ConfirmApplicationRequest,
) -> Result<trash_actions::TrashOperationResult, String> {
    require_main(&window)?;
    if !request.bundle_only_acknowledged || !request.no_uninstaller_acknowledged {
        return Err("앱 본체만 이동하며 전용 제거기가 없음을 확인해야 합니다".to_owned());
    }
    confirm(app, runtime.inner(), request.plan_id, false).await
}

#[tauri::command]
pub(crate) async fn confirm_application_data_trash(
    window: WebviewWindow,
    app: AppHandle,
    runtime: State<'_, ScanRuntime>,
    request: ConfirmApplicationDataRequest,
) -> Result<trash_actions::TrashOperationResult, String> {
    require_main(&window)?;
    if !request.related_data_acknowledged {
        return Err("선택한 캐시·환경설정 이동을 별도로 확인해야 합니다".to_owned());
    }
    confirm(app, runtime.inner(), request.plan_id, true).await
}

async fn confirm(
    app: AppHandle,
    runtime: &ScanRuntime,
    plan_id: String,
    data_only: bool,
) -> Result<trash_actions::TrashOperationResult, String> {
    if !cfg!(target_os = "macos") {
        return Err("이 운영체제에서는 정식 앱 제거 화면을 사용하세요".to_owned());
    }
    #[cfg(target_os = "macos")]
    mac::require_confirmed_previous_operation(&app)?;
    let cancellation = runtime.begin()?;
    let _completion = ScanCompletionGuard::new(app.clone());
    let worker_completion = _completion.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _completion = worker_completion;
        #[cfg(target_os = "macos")]
        {
            mac::require_confirmed_previous_operation(&app)?;
            let plan = app
                .state::<ApplicationActionsState>()
                .inner
                .lock()
                .map_err(|_| "앱 확인 상태를 잠그지 못했습니다")?
                .claim(&plan_id, data_only)?;
            cancelled(&cancellation)?;
            if !app.state::<ScanRuntime>().begin_commit()? {
                return Err("앱 정리가 취소되었습니다".to_owned());
            }
            // Arm before entering native code: a panicked/dropped IPC worker must
            // not silently permit another native request whose result is unknown.
            app.state::<ApplicationActionsState>()
                .needs_inspection
                .store(true, Ordering::Release);
            let result =
                trash_actions::trash_application_items(&app, plan.items, data_only, &cancellation);
            if result.as_ref().is_ok_and(|result| result.journal_complete) {
                app.state::<ApplicationActionsState>()
                    .needs_inspection
                    .store(false, Ordering::Release);
            }
            // Persist stale guards and invalidate scan maps even if the IPC caller disappears.
            if let Ok(result) = &result {
                app.state::<ApplicationActionsState>()
                    .inner
                    .lock()
                    .map_err(|_| "앱 결과 상태를 잠그지 못했습니다")?
                    .record_result(&plan.inventory_id, &plan.application_id, data_only, result);
            }
            let _ = app.state::<StoredReports>().clear_all();
            result
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (app, plan_id, data_only, cancellation);
            Err("지원하지 않는 앱 정리 작업입니다".to_owned())
        }
    })
    .await
    .map_err(|error| {
        format!("앱 정리 결과를 확인하지 못했습니다. 작업 기록을 확인하세요: {error}")
    })?
}

#[tauri::command]
pub(crate) fn dismiss_application_plan(
    window: WebviewWindow,
    state: State<'_, ApplicationActionsState>,
    plan_id: String,
) -> Result<(), String> {
    require_main(&window)?;
    #[cfg(target_os = "macos")]
    {
        let mut inner = state
            .inner
            .lock()
            .map_err(|_| "앱 확인 상태를 잠그지 못했습니다")?;
        if inner
            .pending
            .as_ref()
            .is_some_and(|plan| plan.view.plan_id == plan_id)
        {
            inner.pending = None;
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (state, plan_id);
    Ok(())
}

#[tauri::command]
pub(crate) fn open_application_uninstall_settings(
    window: WebviewWindow,
    app: AppHandle,
) -> Result<(), String> {
    require_main(&window)?;
    #[cfg(windows)]
    {
        use tauri_plugin_opener::OpenerExt;
        // A fixed OS URI, never a registry UninstallString, shell, or app path.
        app.opener()
            .open_url(WINDOWS_UNINSTALL_SETTINGS_URI, None::<&str>)
            .map_err(|error| format!("Windows 앱 제거 화면을 열지 못했습니다: {error}"))
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        Err("Windows에서만 사용할 수 있는 앱 제거 화면입니다".to_owned())
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use super::*;
    use std::collections::{BTreeMap, HashMap, HashSet};
    use std::fs::{self, Metadata, OpenOptions};
    use std::io::Read;
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    use std::path::{Component, Path, PathBuf};
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    const PLAN_TTL: Duration = Duration::from_secs(120);
    const MAX_TREE_ENTRIES: usize = 20_000;
    const MAX_TREE_DEPTH: usize = 64;
    const MAX_TREE_PATH_BYTES: usize = 8 * 1024 * 1024;
    const MAX_TREE_TIME: Duration = Duration::from_secs(15);
    const MAX_PLIST_BYTES: u64 = 1024 * 1024;
    const NEEDS_INSPECTION_MESSAGE: &str = "이전 앱 정리 결과가 확정되지 않았습니다. 원본과 휴지통에서 운영체제 작업이 끝났는지 확인한 뒤 앱을 다시 시작하세요";

    pub(super) fn require_confirmed_previous_operation(app: &AppHandle) -> Result<(), String> {
        if app
            .state::<ApplicationActionsState>()
            .needs_inspection
            .load(Ordering::Acquire)
        {
            Err(NEEDS_INSPECTION_MESSAGE.to_owned())
        } else {
            Ok(())
        }
    }

    #[derive(Default)]
    pub(super) struct ActionsInner {
        inventory_id: String,
        entries: BTreeMap<String, Record>,
        pub(super) pending: Option<StoredPlan>,
    }

    #[derive(Clone)]
    struct Record {
        view: ApplicationEntry,
        bundle: Option<Bundle>,
        candidates: Vec<StoredCandidate>,
        removed: bool,
        shared_id: bool,
    }

    #[derive(Clone)]
    struct StoredCandidate {
        view: DataCandidate,
        item: ApplicationItem,
    }

    pub(super) struct StoredPlan {
        pub(super) view: ApplicationPlan,
        pub(super) inventory_id: String,
        pub(super) application_id: String,
        data_only: bool,
        deadline: Instant,
        pub(super) items: Vec<ApplicationItem>,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Identity {
        dev: u64,
        ino: u64,
        mode: u32,
        uid: u32,
        len: u64,
        modified: (i64, i64),
        changed: (i64, i64),
    }

    impl Identity {
        fn of(metadata: &Metadata) -> Self {
            Self {
                dev: metadata.dev(),
                ino: metadata.ino(),
                mode: metadata.mode(),
                uid: metadata.uid(),
                len: metadata.len(),
                modified: (metadata.mtime(), metadata.mtime_nsec()),
                changed: (metadata.ctime(), metadata.ctime_nsec()),
            }
        }
        fn same_node(&self, other: &Self) -> bool {
            self.dev == other.dev
                && self.ino == other.ino
                && self.mode == other.mode
                && self.uid == other.uid
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Bundle {
        path: PathBuf,
        bundle_id: String,
        root: Identity,
        parent: Identity,
        contents: Identity,
        plist: Identity,
        plist_hash: [u8; 32],
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct TreeSnapshot {
        root: Identity,
        parent: Identity,
        fingerprint: [u8; 32],
        logical_bytes: u64,
    }

    #[derive(Clone)]
    pub(super) struct ApplicationItem {
        path: PathBuf,
        snapshot: TreeSnapshot,
        bundle: Bundle,
        data_only: bool,
        bundle_removed: bool,
    }

    impl trash_actions::JournalTrashItem for ApplicationItem {
        fn path(&self) -> &Path {
            &self.path
        }
        fn recovery_path(&self) -> &Path {
            &self.path
        }
        fn logical_bytes(&self) -> u64 {
            self.snapshot.logical_bytes
        }
        fn revalidate(&self, cancellation: &AtomicBool) -> Result<(), String> {
            verify_app_current(&self.bundle, self.bundle_removed)?;
            if self.data_only {
                validate_data_path(&self.bundle.bundle_id, &self.path)?;
                ensure_unique_bundle_owner(&self.bundle, self.bundle_removed, cancellation)?;
            }
            let current = snapshot(&self.path, !self.data_only, cancellation)?;
            if current != self.snapshot {
                return Err("검토 후 항목이 변경됐습니다. 다시 검토하세요".to_owned());
            }
            // Running state is checked again after potentially long metadata walking.
            ensure_not_running(&self.bundle)?;
            cancelled(cancellation)
        }
    }

    impl ActionsInner {
        fn record(&self, request: &PrepareApplicationRequest) -> Result<Record, String> {
            if self.inventory_id != request.inventory_id {
                return Err("앱 목록이 변경됐습니다. 새 목록에서 다시 선택하세요".to_owned());
            }
            self.entries
                .get(&request.application_id)
                .cloned()
                .ok_or_else(|| "앱 목록에서 대상을 다시 선택하세요".to_owned())
        }
        pub(super) fn claim(&mut self, id: &str, data_only: bool) -> Result<StoredPlan, String> {
            let plan = self.pending.as_ref().ok_or_else(|| {
                "확인 계획이 없거나 이미 사용했습니다. 다시 검토하세요".to_owned()
            })?;
            if plan.view.plan_id != id
                || plan.inventory_id != self.inventory_id
                || plan.data_only != data_only
            {
                return Err("앱 정리 확인 대상 또는 종류가 다릅니다".to_owned());
            }
            if plan.deadline <= Instant::now() {
                self.pending = None;
                return Err("앱 정리 확인 시간이 만료됐습니다. 다시 검토하세요".to_owned());
            }
            Ok(self.pending.take().expect("validated application plan"))
        }
        pub(super) fn record_result(
            &mut self,
            inventory_id: &str,
            app_id: &str,
            data_only: bool,
            result: &trash_actions::TrashOperationResult,
        ) {
            if self.inventory_id != inventory_id {
                return;
            }
            if let Some(record) = self.entries.get_mut(app_id) {
                if !result.journal_complete {
                    record.candidates.clear();
                    record.view.removal_mode = "protected";
                    record.view.protection_reason = Some(NEEDS_INSPECTION_MESSAGE.to_owned());
                    return;
                }
                if !data_only && result.moved_count == 1 {
                    record.removed = true;
                }
                if data_only {
                    record.candidates.retain(|candidate| {
                        !result.items.iter().any(|item| {
                            item.path == candidate.view.path
                                && item.status == trash_actions::TrashItemStatus::Moved
                        })
                    });
                }
            }
        }
    }

    pub(super) fn publish_inventory(
        app: &AppHandle,
        inventory: system_inventory::InstalledAppInventory,
        inventory_id: String,
        cancellation: &AtomicBool,
    ) -> Result<ApplicationInventory, String> {
        let mut entries = BTreeMap::new();
        let mut applications = Vec::with_capacity(inventory.applications.len());
        let mut bundle_counts = HashMap::<String, usize>::new();
        for entry in &inventory.applications {
            for id in &entry.cleanup_identity_tokens {
                *bundle_counts.entry(id.to_ascii_lowercase()).or_default() += 1;
            }
        }
        let running = running_applications()?;
        for entry in inventory.applications {
            cancelled(cancellation)?;
            let inspected = entry
                .install_location
                .as_deref()
                .ok_or_else(|| "앱 위치를 확인할 수 없습니다".to_owned())
                .and_then(|path| read_bundle(Path::new(path)));
            let (bundle, mut reason) = match inspected {
                Ok(bundle) => {
                    let reason = running.check(&bundle).err();
                    (Some(bundle), reason)
                }
                Err(reason) => (None, Some(reason)),
            };
            if app
                .state::<ApplicationActionsState>()
                .needs_inspection
                .load(Ordering::Acquire)
            {
                reason = Some(NEEDS_INSPECTION_MESSAGE.to_owned());
            }
            let view = ApplicationEntry {
                id: opaque_id()?,
                display_name: entry.display_name,
                display_version: entry.display_version,
                publisher: entry.publisher,
                install_location: entry.install_location,
                estimated_bytes: entry.estimated_bytes,
                removal_mode: if reason.is_some() {
                    "protected"
                } else {
                    "trashBundle"
                },
                protection_reason: reason,
            };
            let shared_id = bundle.as_ref().is_some_and(|bundle| {
                bundle_counts
                    .get(&bundle.bundle_id.to_ascii_lowercase())
                    .copied()
                    .unwrap_or_default()
                    > 1
            });
            applications.push(view.clone());
            entries.insert(
                view.id.clone(),
                Record {
                    view,
                    bundle,
                    candidates: Vec::new(),
                    removed: false,
                    shared_id,
                },
            );
        }
        cancelled(cancellation)?;
        let result = ApplicationInventory {
            platform: "macos",
            inventory_id: inventory_id.clone(),
            applications,
            issues: inventory.issues,
        };
        *app.state::<ApplicationActionsState>()
            .inner
            .lock()
            .map_err(|_| "앱 목록 상태를 잠그지 못했습니다")? = ActionsInner {
            inventory_id,
            entries,
            pending: None,
        };
        Ok(result)
    }

    pub(super) fn prepare(
        app: &AppHandle,
        request: PrepareApplicationRequest,
        selected: Option<Vec<String>>,
        cancellation: &AtomicBool,
    ) -> Result<ApplicationPlan, String> {
        let state = app.state::<ApplicationActionsState>();
        let record = {
            let mut inner = state
                .inner
                .lock()
                .map_err(|_| "앱 확인 상태를 잠그지 못했습니다")?;
            inner.pending = None;
            inner.record(&request)?
        };
        if let Some(reason) = &record.view.protection_reason {
            return Err(reason.clone());
        }
        let bundle = record
            .bundle
            .clone()
            .ok_or_else(|| "안전한 앱 본체를 확인하지 못했습니다".to_owned())?;
        verify_app_current(&bundle, record.removed)?;
        let data_only = selected.is_some();
        let (items, candidates) = if let Some(selected) = selected {
            if selected.is_empty()
                || selected.len() > 2
                || selected.iter().collect::<HashSet<_>>().len() != selected.len()
            {
                return Err("관련 데이터 후보를 중복 없이 1~2개 선택하세요".to_owned());
            }
            let mut candidates = Vec::new();
            for id in selected {
                let mut candidate = record
                    .candidates
                    .iter()
                    .find(|candidate| candidate.view.id == id)
                    .cloned()
                    .ok_or_else(|| "관련 데이터 후보가 변경됐습니다. 다시 검토하세요".to_owned())?;
                candidate.item.bundle_removed = record.removed;
                trash_actions::JournalTrashItem::revalidate(&candidate.item, cancellation)?;
                candidates.push(candidate);
            }
            (
                candidates
                    .iter()
                    .map(|candidate| candidate.item.clone())
                    .collect(),
                candidates,
            )
        } else {
            if record.removed {
                return Err(
                    "이 앱 본체는 이미 휴지통으로 이동했습니다. 목록을 새로 고치세요".to_owned(),
                );
            }
            let snapshot = snapshot(&bundle.path, true, cancellation)?;
            let item = ApplicationItem {
                path: bundle.path.clone(),
                snapshot,
                bundle: bundle.clone(),
                data_only: false,
                bundle_removed: false,
            };
            let candidates = if record.shared_id {
                Vec::new()
            } else {
                collect_candidates(&bundle, cancellation)?
            };
            (vec![item], candidates)
        };
        cancelled(cancellation)?;
        verify_app_current(&bundle, record.removed)?;
        let mut view = ApplicationPlan {
            plan_id: opaque_id()?,
            display_name: record.view.display_name.clone(),
            path: bundle.path.to_string_lossy().into_owned(),
            expires_at_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64
                + PLAN_TTL.as_millis() as u64,
            related_data: candidates
                .iter()
                .map(|candidate| candidate.view.clone())
                .collect(),
            warnings: if data_only {
                vec!["선택한 캐시·환경설정만 이동합니다. 환경설정이 초기화될 수 있으며 macOS가 다시 만들 수도 있습니다".to_owned(), "앱 식별자의 정확한 일치는 연관 근거일 뿐 데이터가 불필요하다는 보장은 아닙니다".to_owned()]
            } else {
                vec!["전용 제거 프로그램이 있다면 먼저 사용하세요. 앱 본체 이동은 완전 제거가 아닙니다".to_owned(), "설정·문서·관련 데이터는 자동으로 삭제하지 않습니다".to_owned()]
            },
        };
        if record.shared_id {
            view.warnings.push(
                "같은 앱 식별자를 사용하는 다른 앱이 있어 관련 데이터는 보호됩니다".to_owned(),
            );
        }
        let mut inner = state
            .inner
            .lock()
            .map_err(|_| "앱 확인 상태를 잠그지 못했습니다")?;
        inner.record(&request)?;
        if !data_only {
            inner
                .entries
                .get_mut(&request.application_id)
                .expect("checked record")
                .candidates = candidates;
        }
        inner.pending = Some(StoredPlan {
            view: view.clone(),
            inventory_id: request.inventory_id,
            application_id: request.application_id,
            data_only,
            deadline: Instant::now() + PLAN_TTL,
            items,
        });
        Ok(view)
    }

    fn safe_chain(path: &Path) -> Result<(), String> {
        if bloomsweepy_core::is_cloud_storage_path(path) {
            return Err("클라우드 동기화 폴더는 정리하지 않습니다".to_owned());
        }
        if !path.is_absolute()
            || path.as_os_str().len() > 4096
            || path
                .components()
                .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
        {
            return Err("안전한 절대 경로가 아닙니다".to_owned());
        }
        let mut current = PathBuf::new();
        for component in path.components() {
            current.push(component);
            let metadata = fs::symlink_metadata(&current)
                .map_err(|error| format!("경로를 확인하지 못했습니다: {error}"))?;
            if metadata.file_type().is_symlink()
                || bloomsweepy_core::is_online_only_metadata(&metadata)
            {
                return Err("링크 또는 온라인 전용 항목은 정리하지 않습니다".to_owned());
            }
            if !metadata.is_dir() && current != path {
                return Err("상위 경로가 일반 폴더가 아닙니다".to_owned());
            }
        }
        Ok(())
    }

    fn valid_bundle_id(id: &str) -> bool {
        id.len() <= 256
            && id.split('.').count() >= 2
            && id.split('.').all(|part| {
                !part.is_empty()
                    && part
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            })
    }

    fn allowed_bundle_path(path: &Path) -> Result<(), String> {
        let home_root = dirs::home_dir().map(|home| home.join("Applications"));
        if path.parent() != Some(Path::new("/Applications"))
            && path.parent() != home_root.as_deref()
        {
            return Err("시스템 앱 또는 관리 대상 밖의 앱은 보호됩니다".to_owned());
        }
        if !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("app"))
        {
            return Err("macOS 앱 본체가 아닙니다".to_owned());
        }
        safe_chain(path)
    }

    fn read_bundle(path: &Path) -> Result<Bundle, String> {
        allowed_bundle_path(path)?;
        read_bundle_at(path)
    }

    fn read_bundle_at(path: &Path) -> Result<Bundle, String> {
        safe_chain(path)?;
        let root = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
        if !root.is_dir() {
            return Err("앱 본체가 일반 폴더가 아닙니다".to_owned());
        }
        let parent = fs::symlink_metadata(path.parent().ok_or("앱 상위 경로가 없습니다")?)
            .map_err(|error| error.to_string())?;
        let contents_path = path.join("Contents");
        let plist_path = contents_path.join("Info.plist");
        safe_chain(&plist_path)?;
        let contents = fs::symlink_metadata(&contents_path).map_err(|error| error.to_string())?;
        let before = fs::symlink_metadata(&plist_path).map_err(|error| error.to_string())?;
        if !before.is_file() || before.len() > MAX_PLIST_BYTES {
            return Err("앱 정보를 안전한 크기로 읽을 수 없습니다".to_owned());
        }
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(0x100 | 0x4)
            .open(&plist_path)
            .map_err(|error| error.to_string())?;
        if Identity::of(&file.metadata().map_err(|error| error.to_string())?)
            != Identity::of(&before)
        {
            return Err("앱 정보가 변경됐습니다".to_owned());
        }
        let mut bytes = Vec::with_capacity(before.len() as usize);
        file.take(MAX_PLIST_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| error.to_string())?;
        if bytes.len() as u64 > MAX_PLIST_BYTES {
            return Err("앱 정보 크기 한도를 넘었습니다".to_owned());
        }
        let value = plist::Value::from_reader(std::io::Cursor::new(&bytes))
            .map_err(|_| "앱 정보를 해석할 수 없습니다")?;
        let dict = value
            .as_dictionary()
            .ok_or("앱 정보 형식이 올바르지 않습니다")?;
        let id = dict
            .get("CFBundleIdentifier")
            .and_then(plist::Value::as_string)
            .filter(|id| valid_bundle_id(id))
            .ok_or("정확한 앱 식별자를 확인할 수 없습니다")?
            .to_owned();
        let lower_id = id.to_ascii_lowercase();
        if lower_id.starts_with("com.apple.")
            || lower_id == "com.broomsweepy.desktop"
            || lower_id == "com.bloomsweepy.desktop"
            || std::env::current_exe().is_ok_and(|exe| exe.starts_with(path))
        {
            return Err("시스템 앱 또는 현재 BroomSweepy 앱은 보호됩니다".to_owned());
        }
        if dict.contains_key("SMPrivilegedExecutables")
            || dict.contains_key("NSExtension")
            || dict
                .get("CFBundlePackageType")
                .and_then(plist::Value::as_string)
                != Some("APPL")
        {
            return Err(
                "확장 기능 또는 보조 서비스를 포함한 앱은 전용 제거 프로그램을 사용하세요"
                    .to_owned(),
            );
        }
        if let Ok(metadata) = fs::symlink_metadata(contents_path.join("Library"))
            && (metadata.file_type().is_symlink() || !metadata.is_dir())
        {
            return Err("앱 보조 서비스 경로가 링크 또는 특수 파일입니다".to_owned());
        }
        for marker in [
            "Library/LaunchServices",
            "Library/LaunchDaemons",
            "Library/LaunchAgents",
            "Library/SystemExtensions",
            "Library/LoginItems",
            "Library/HelperTools",
            "PlugIns",
            "Extensions",
        ] {
            match fs::symlink_metadata(contents_path.join(marker)) {
                Ok(_) => {
                    return Err(
                        "확장 기능 또는 보조 서비스를 포함한 앱은 전용 제거 프로그램을 사용하세요"
                            .to_owned(),
                    );
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => return Err("앱의 보조 서비스 유무를 확인하지 못했습니다".to_owned()),
            }
        }
        let bundle = Bundle {
            path: path.to_owned(),
            bundle_id: id,
            root: Identity::of(&root),
            parent: Identity::of(&parent),
            contents: Identity::of(&contents),
            plist: Identity::of(&before),
            plist_hash: *blake3::hash(&bytes).as_bytes(),
        };
        if Identity::of(&fs::symlink_metadata(path).map_err(|error| error.to_string())?)
            != bundle.root
            || Identity::of(&fs::symlink_metadata(&plist_path).map_err(|error| error.to_string())?)
                != bundle.plist
        {
            return Err("앱이 확인 중 변경됐습니다".to_owned());
        }
        Ok(bundle)
    }

    fn verify_app_current(bundle: &Bundle, removed: bool) -> Result<(), String> {
        if removed {
            safe_chain(bundle.path.parent().ok_or("앱 상위 경로가 없습니다")?)?;
            let parent = Identity::of(
                &fs::symlink_metadata(bundle.path.parent().expect("checked parent"))
                    .map_err(|error| error.to_string())?,
            );
            if !parent.same_node(&bundle.parent) {
                return Err("앱 상위 경로가 변경됐습니다".to_owned());
            }
            match fs::symlink_metadata(&bundle.path) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                _ => {
                    return Err(
                        "앱이 복구 또는 재설치됐거나 위치를 확인할 수 없습니다. 새로 고침하세요"
                            .to_owned(),
                    );
                }
            }
        } else {
            let current = read_bundle(&bundle.path)?;
            // Parent timestamps may change when unrelated applications update.
            if current.root != bundle.root
                || current.contents != bundle.contents
                || current.plist != bundle.plist
                || current.plist_hash != bundle.plist_hash
                || current.bundle_id != bundle.bundle_id
                || !current.parent.same_node(&bundle.parent)
            {
                return Err("목록 조회 후 앱이 변경됐습니다. 새로 고침하세요".to_owned());
            }
        }
        ensure_not_running(bundle)
    }

    struct RunningApplications {
        ids: HashSet<String>,
        paths: Vec<PathBuf>,
    }

    impl RunningApplications {
        fn check(&self, bundle: &Bundle) -> Result<(), String> {
            if self.ids.contains(&bundle.bundle_id)
                || self.paths.iter().any(|path| path.starts_with(&bundle.path))
            {
                return Err("백그라운드 실행을 포함해 사용 중인 앱입니다. 앱을 완전히 종료한 뒤 목록을 새로 고치세요".to_owned());
            }
            Ok(())
        }
    }

    fn running_applications() -> Result<RunningApplications, String> {
        let mut running = objc2::rc::autoreleasepool(|_| {
            let mut running = RunningApplications {
                ids: HashSet::new(),
                paths: Vec::new(),
            };
            let workspace = objc2_app_kit::NSWorkspace::sharedWorkspace();
            let applications = workspace.runningApplications();
            if applications.len() > 20_000 {
                return Err("실행 앱 수가 안전 확인 한도를 넘었습니다".to_owned());
            }
            // Deliberately no activationPolicy filter: Background apps count too.
            for app in applications.iter().filter(|app| !app.isTerminated()) {
                if let Some(id) = app.bundleIdentifier() {
                    running.ids.insert(id.to_string());
                }
                if let Some(path) = app.bundleURL().and_then(|url| url.path()) {
                    running.paths.push(PathBuf::from(path.to_string()));
                }
                if let Some(path) = app.executableURL().and_then(|url| url.path()) {
                    running.paths.push(PathBuf::from(path.to_string()));
                }
            }
            Ok(running)
        })?;
        // Include CLI/helper processes launched from a bundle but absent from NSWorkspace.
        let mut system = sysinfo::System::new();
        system.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::All,
            true,
            sysinfo::ProcessRefreshKind::nothing()
                .with_exe(sysinfo::UpdateKind::OnlyIfNotSet)
                .without_tasks(),
        );
        if system.processes().len() > 20_000 {
            return Err("실행 프로세스 수가 안전 확인 한도를 넘었습니다".to_owned());
        }
        running.paths.extend(
            system
                .processes()
                .values()
                .filter_map(|process| process.exe().map(Path::to_owned)),
        );
        Ok(running)
    }

    fn ensure_not_running(bundle: &Bundle) -> Result<(), String> {
        running_applications()?.check(bundle)
    }

    fn data_paths(bundle_id: &str) -> Result<[(PathBuf, &'static str); 2], String> {
        if !valid_bundle_id(bundle_id) || bundle_id.to_ascii_lowercase().starts_with("com.apple.") {
            return Err("관련 데이터 식별자가 올바르지 않습니다".to_owned());
        }
        let library = dirs::home_dir()
            .ok_or("사용자 폴더를 찾지 못했습니다")?
            .join("Library");
        Ok([
            (library.join("Caches").join(bundle_id), "cache"),
            (
                library
                    .join("Preferences")
                    .join(format!("{bundle_id}.plist")),
                "preferences",
            ),
        ])
    }

    fn validate_data_path(bundle_id: &str, path: &Path) -> Result<(), String> {
        let paths = data_paths(bundle_id)?;
        let (_, kind) = paths
            .iter()
            .find(|(allowed, _)| allowed == path)
            .ok_or("한정된 앱 캐시·환경설정 경로가 아닙니다")?;
        safe_chain(path)?;
        let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
        let home = dirs::home_dir().ok_or("사용자 폴더를 찾지 못했습니다")?;
        let owner = fs::symlink_metadata(home)
            .map_err(|error| error.to_string())?
            .uid();
        if metadata.uid() != owner {
            return Err("다른 사용자가 소유한 관련 데이터는 보호됩니다".to_owned());
        }
        if (*kind == "cache" && !metadata.is_dir())
            || (*kind == "preferences" && (!metadata.is_file() || metadata.nlink() != 1))
        {
            return Err("공유 또는 비정상 캐시·환경설정 항목은 보호됩니다".to_owned());
        }
        Ok(())
    }

    fn ensure_unique_bundle_owner(
        bundle: &Bundle,
        removed: bool,
        cancellation: &AtomicBool,
    ) -> Result<(), String> {
        let inventory =
            system_inventory::installed_app_ownership_inventory_with_cancellation(|| {
                cancellation.load(Ordering::Acquire)
            })
            .map_err(|_| "앱 소유 관계 확인이 취소됐습니다")?;
        if !inventory.supported || !inventory.issues.is_empty() {
            return Err(
                "앱 목록 전체의 소유 관계를 확인하지 못해 관련 데이터를 보호합니다".to_owned(),
            );
        }
        let matching: Vec<_> = inventory
            .applications
            .iter()
            .filter(|app| {
                app.cleanup_identity_tokens
                    .iter()
                    .any(|id| id.eq_ignore_ascii_case(&bundle.bundle_id))
            })
            .collect();
        let same_owner = matching.len() == 1
            && matching[0]
                .install_location
                .as_deref()
                .is_some_and(|path| Path::new(path) == bundle.path);
        if (removed && !matching.is_empty()) || (!removed && !same_owner) {
            return Err(
                "같은 식별자의 앱이 다른 위치에 있거나 소유 관계가 변경돼 관련 데이터를 보호합니다"
                    .to_owned(),
            );
        }
        Ok(())
    }

    fn collect_candidates(
        bundle: &Bundle,
        cancellation: &AtomicBool,
    ) -> Result<Vec<StoredCandidate>, String> {
        let mut candidates = Vec::new();
        if ensure_unique_bundle_owner(bundle, false, cancellation).is_err() {
            cancelled(cancellation)?;
            return Ok(candidates);
        }
        for (path, kind) in data_paths(&bundle.bundle_id)? {
            cancelled(cancellation)?;
            if validate_data_path(&bundle.bundle_id, &path).is_err() {
                continue;
            }
            let Ok(snapshot) = snapshot(&path, false, cancellation) else {
                cancelled(cancellation)?;
                continue;
            };
            candidates.push(StoredCandidate {
                view: DataCandidate {
                    id: opaque_id()?,
                    path: path.to_string_lossy().into_owned(),
                    kind,
                    evidence: format!(
                        "앱 식별자 {}와 정확히 일치하는 사용자 전용 경로",
                        bundle.bundle_id
                    ),
                    estimated_bytes: Some(snapshot.logical_bytes),
                },
                item: ApplicationItem {
                    path,
                    snapshot,
                    bundle: bundle.clone(),
                    data_only: true,
                    bundle_removed: false,
                },
            });
        }
        Ok(candidates)
    }

    fn snapshot(
        path: &Path,
        allow_internal_links: bool,
        cancellation: &AtomicBool,
    ) -> Result<TreeSnapshot, String> {
        safe_chain(path)?;
        let root = Identity::of(&fs::symlink_metadata(path).map_err(|error| error.to_string())?);
        let parent_path = path.parent().ok_or("상위 폴더를 확인할 수 없습니다")?;
        let parent =
            Identity::of(&fs::symlink_metadata(parent_path).map_err(|error| error.to_string())?);
        if parent.dev != root.dev {
            return Err("마운트된 볼륨의 루트는 정리하지 않습니다".to_owned());
        }
        let started = Instant::now();
        let mut hasher = blake3::Hasher::new();
        let mut stack = vec![(path.to_owned(), 0_usize)];
        let mut entries = 0_usize;
        let mut path_bytes = 0_usize;
        let mut logical_bytes = 0_u64;
        while let Some((current, depth)) = stack.pop() {
            cancelled(cancellation)?;
            if entries >= MAX_TREE_ENTRIES
                || path_bytes > MAX_TREE_PATH_BYTES
                || depth > MAX_TREE_DEPTH
                || started.elapsed() > MAX_TREE_TIME
            {
                return Err("안전 검토의 항목·경로·시간 한도를 넘었습니다. Finder 또는 전용 제거 프로그램을 사용하세요".to_owned());
            }
            entries += 1;
            let metadata = fs::symlink_metadata(&current).map_err(|error| error.to_string())?;
            let identity = Identity::of(&metadata);
            if metadata.dev() != root.dev || bloomsweepy_core::is_online_only_metadata(&metadata) {
                return Err("다른 볼륨 또는 온라인 전용 데이터는 정리하지 않습니다".to_owned());
            }
            if metadata.is_file() && !allow_internal_links && metadata.nlink() != 1 {
                return Err("다른 위치와 공유된 파일은 정리하지 않습니다".to_owned());
            }
            if !allow_internal_links && metadata.uid() != root.uid {
                return Err("다른 사용자의 데이터를 포함한 폴더는 보호됩니다".to_owned());
            }
            let relative = current
                .strip_prefix(path)
                .map_err(|error| error.to_string())?;
            hasher.update(relative.as_os_str().as_encoded_bytes());
            hasher.update(format!("{identity:?}").as_bytes());
            if metadata.file_type().is_symlink() {
                if !allow_internal_links || current == path {
                    return Err("관련 데이터의 링크는 정리하지 않습니다".to_owned());
                }
                // Preserve the link object within the bundle. Never follow its target.
                hasher.update(
                    fs::read_link(&current)
                        .map_err(|error| error.to_string())?
                        .as_os_str()
                        .as_encoded_bytes(),
                );
            } else if metadata.is_dir() {
                safe_chain(&current)?;
                let mut children = Vec::new();
                for child in fs::read_dir(&current).map_err(|error| error.to_string())? {
                    cancelled(cancellation)?;
                    if entries + stack.len() + children.len() >= MAX_TREE_ENTRIES
                        || path_bytes > MAX_TREE_PATH_BYTES
                        || started.elapsed() > MAX_TREE_TIME
                    {
                        return Err("안전 검토의 항목·경로·시간 한도를 넘었습니다".to_owned());
                    }
                    let child = child.map_err(|error| error.to_string())?.path();
                    path_bytes = path_bytes.saturating_add(child.as_os_str().len());
                    children.push(child);
                }
                if Identity::of(&fs::symlink_metadata(&current).map_err(|error| error.to_string())?)
                    != identity
                {
                    return Err("검토 중 폴더가 변경됐습니다".to_owned());
                }
                children.sort_unstable();
                stack.extend(children.into_iter().rev().map(|child| (child, depth + 1)));
            } else if metadata.is_file() {
                logical_bytes = logical_bytes.saturating_add(metadata.len());
            } else {
                return Err("특수 파일을 포함한 항목은 보호됩니다".to_owned());
            }
        }
        if Identity::of(&fs::symlink_metadata(path).map_err(|error| error.to_string())?) != root {
            return Err("검토 중 대상이 변경됐습니다".to_owned());
        }
        // Ignore parent timestamps: moving the first of two candidates can update it.
        let parent = Identity {
            len: 0,
            modified: (0, 0),
            changed: (0, 0),
            ..parent
        };
        Ok(TreeSnapshot {
            root,
            parent,
            fingerprint: *hasher.finalize().as_bytes(),
            logical_bytes,
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::os::unix::fs::symlink;

        fn fixture_app(id: &str) -> (tempfile::TempDir, PathBuf) {
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().canonicalize().unwrap().join("Fixture.app");
            fs::create_dir_all(path.join("Contents")).unwrap();
            fs::write(path.join("Contents/Info.plist"), format!(r#"<?xml version="1.0"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>{id}</string><key>CFBundlePackageType</key><string>APPL</string></dict></plist>"#)).unwrap();
            (temp, path)
        }

        fn plan_fixture(data_only: bool) -> ActionsInner {
            ActionsInner {
                inventory_id: "inventory".to_owned(),
                entries: BTreeMap::new(),
                pending: Some(StoredPlan {
                    view: ApplicationPlan {
                        plan_id: "plan".to_owned(),
                        display_name: "Fixture".to_owned(),
                        path: "/Applications/Fixture.app".to_owned(),
                        expires_at_unix_ms: 1,
                        related_data: Vec::new(),
                        warnings: Vec::new(),
                    },
                    inventory_id: "inventory".to_owned(),
                    application_id: "app".to_owned(),
                    data_only,
                    deadline: Instant::now() + PLAN_TTL,
                    items: Vec::new(),
                }),
            }
        }

        #[test]
        fn application_plans_are_kind_bound_one_shot_expiring_and_refresh_invalidated() {
            let mut state = plan_fixture(false);
            assert!(state.claim("plan", true).is_err());
            assert!(state.claim("foreign", false).is_err());
            assert!(state.claim("plan", false).is_ok());
            assert!(state.claim("plan", false).is_err());
            let mut state = plan_fixture(true);
            state.pending.as_mut().unwrap().deadline = Instant::now();
            assert!(state.claim("plan", true).is_err());
            assert!(state.pending.is_none());
            let mut state = plan_fixture(false);
            state.inventory_id = "new inventory".to_owned();
            assert!(state.claim("plan", false).is_err());
            assert!(ActionsInner::default().claim("plan", false).is_err());
        }

        #[test]
        fn concurrent_application_claim_can_only_succeed_once() {
            let state = std::sync::Mutex::new(plan_fixture(false));
            std::thread::scope(|scope| {
                let handles: Vec<_> = (0..4)
                    .map(|_| scope.spawn(|| state.lock().unwrap().claim("plan", false).is_ok()))
                    .collect();
                assert_eq!(
                    handles
                        .into_iter()
                        .map(|handle| handle.join().unwrap())
                        .filter(|claimed| *claimed)
                        .count(),
                    1
                );
            });
        }

        #[test]
        fn bundle_snapshot_keeps_internal_links_but_does_not_walk_the_target() {
            let (_temp, path) = fixture_app("org.example.fixture");
            let outside = path.parent().unwrap().join("outside");
            fs::create_dir(&outside).unwrap();
            fs::write(outside.join("secret"), vec![0_u8; 1000]).unwrap();
            symlink(&outside, path.join("Contents/Frameworks")).unwrap();
            let snapshot = snapshot(&path, true, &AtomicBool::new(false)).unwrap();
            assert!(snapshot.logical_bytes < 1000);
            assert!(super::snapshot(&path, false, &AtomicBool::new(false)).is_err());
            assert!(
                super::snapshot(
                    &path.join("Contents/Frameworks"),
                    true,
                    &AtomicBool::new(false)
                )
                .is_err()
            );
        }

        #[test]
        fn snapshots_detect_changed_data_and_replaced_roots() {
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().canonicalize().unwrap().join("cache");
            fs::create_dir(&path).unwrap();
            fs::write(path.join("data"), b"first").unwrap();
            let before = snapshot(&path, false, &AtomicBool::new(false)).unwrap();
            fs::write(path.join("data"), b"changed content").unwrap();
            assert_ne!(
                snapshot(&path, false, &AtomicBool::new(false)).unwrap(),
                before
            );
            fs::rename(&path, path.with_extension("old")).unwrap();
            fs::create_dir(&path).unwrap();
            fs::write(path.join("data"), b"first").unwrap();
            assert_ne!(
                snapshot(&path, false, &AtomicBool::new(false)).unwrap(),
                before
            );
        }

        #[test]
        fn snapshots_reject_shared_files_depth_and_cancellation() {
            let (_temp, path) = fixture_app("org.example.fixture");
            assert!(snapshot(&path, true, &AtomicBool::new(true)).is_err());
            fs::hard_link(
                path.join("Contents/Info.plist"),
                path.join("Contents/shared.plist"),
            )
            .unwrap();
            assert!(snapshot(&path, false, &AtomicBool::new(false)).is_err());
            let mut nested = path.clone();
            for _ in 0..=MAX_TREE_DEPTH {
                nested.push("nested");
                fs::create_dir(&nested).unwrap();
            }
            assert!(snapshot(&path, true, &AtomicBool::new(false)).is_err());
        }

        #[test]
        fn bundle_review_rejects_apple_self_helpers_bad_plist_and_links() {
            let (_temp, path) = fixture_app("org.example.fixture");
            assert!(read_bundle_at(&path).is_ok());
            assert!(
                read_bundle(&path).is_err(),
                "temporary paths are not authorized application roots"
            );
            fs::create_dir_all(path.join("Contents/Library/LoginItems")).unwrap();
            assert!(read_bundle_at(&path).is_err());
            let (_temp, path) = fixture_app("com.apple.fixture");
            assert!(read_bundle_at(&path).is_err());
            let (_temp, path) = fixture_app("com.broomsweepy.desktop");
            assert!(read_bundle_at(&path).is_err());
            let (_temp, path) = fixture_app("org.example.fixture");
            let real = path.join("Contents/real.plist");
            fs::rename(path.join("Contents/Info.plist"), &real).unwrap();
            symlink(real, path.join("Contents/Info.plist")).unwrap();
            assert!(read_bundle_at(&path).is_err());
        }

        #[test]
        fn bundle_data_ids_cannot_escape_or_select_support_and_documents() {
            for id in [
                "../escape",
                "com.example/escape",
                "com..example",
                "com.example\\escape",
                "org",
                "com.apple.fixture",
            ] {
                assert!(data_paths(id).is_err(), "{id}");
            }
            let paths = data_paths("org.example.fixture").unwrap();
            assert!(paths[0].0.ends_with("Library/Caches/org.example.fixture"));
            assert!(
                paths[1]
                    .0
                    .ends_with("Library/Preferences/org.example.fixture.plist")
            );
            assert!(
                validate_data_path(
                    "org.example.fixture",
                    Path::new("/Applications/Fixture.app")
                )
                .is_err()
            );
            assert!(
                safe_chain(Path::new(
                    "/Users/fixture/Library/CloudStorage/provider/file"
                ))
                .is_err()
            );
        }

        #[test]
        fn background_and_bundle_helper_identity_both_block_removal() {
            let (_temp, path) = fixture_app("org.example.fixture");
            let bundle = read_bundle_at(&path).unwrap();
            let running = RunningApplications {
                ids: HashSet::from([bundle.bundle_id.clone()]),
                paths: Vec::new(),
            };
            assert!(running.check(&bundle).is_err());
            let running = RunningApplications {
                ids: HashSet::new(),
                paths: vec![path.join("Contents/MacOS/helper")],
            };
            assert!(running.check(&bundle).is_err());
            let running = RunningApplications {
                ids: HashSet::new(),
                paths: vec![path.with_extension("app-other")],
            };
            assert!(running.check(&bundle).is_ok());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_uninstall_target_is_fixed_os_settings_only() {
        assert_eq!(WINDOWS_UNINSTALL_SETTINGS_URI, "ms-settings:appsfeatures");
        assert!(!WINDOWS_UNINSTALL_SETTINGS_URI.contains(' '));
    }

    #[test]
    fn app_requests_reject_raw_paths_unknown_fields_and_missing_acknowledgment() {
        assert!(serde_json::from_value::<PrepareApplicationRequest>(serde_json::json!({"inventoryId":"i", "applicationId":"a", "path":"/Applications/Other.app"})).is_err());
        assert!(
            serde_json::from_value::<ConfirmApplicationRequest>(
                serde_json::json!({"planId":"p", "bundleOnlyAcknowledged":true})
            )
            .is_err()
        );
        assert!(serde_json::from_value::<ConfirmApplicationDataRequest>(serde_json::json!({"planId":"p", "relatedDataAcknowledged":true, "paths":["/tmp/anything"]})).is_err());
        let first = opaque_id().unwrap();
        assert_eq!(first.len(), 32);
        assert_ne!(first, opaque_id().unwrap());
    }
}
