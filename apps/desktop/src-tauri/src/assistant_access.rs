//! Native chat authority and app-issued local volume IDs. Never an MCP grant.
use crate::permission_settings::AssistantPermissionMode;
use crate::{assistant_files, assistant_tools, control_server};
use bloomsweepy_control::{AppToolRequest, AppToolResult, AppToolStatus, FileWorkspaceAction};
use bloomsweepy_core::DirectoryNode;
use serde_json::json;
use std::{
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager, WebviewWindow};

const MAX_SESSIONS: usize = 16;
const MAX_VOLUMES: usize = 16;
const MAX_RESULTS: usize = 200;
const MAX_VISITED: u64 = 250_000;
const MAX_PATH_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone)]
struct Volume {
    id: String,
    root: PathBuf,
    name: String,
    file_system: String,
    total_bytes: u64,
    available_bytes: u64,
    read_only: bool,
    identity: (u64, u64),
}

#[derive(Clone)]
struct SearchEntry {
    node: DirectoryNode,
    volume: Volume,
}

struct SessionAccess {
    binding: control_server::NativeChatBinding,
    effective_root: Option<PathBuf>,
    effective_identity: Option<(u64, u64)>,
    automatic_eligible: bool,
    revision: String,
    volumes: Vec<Volume>,
    entries: Vec<SearchEntry>,
    complete: bool,
}

#[derive(Default)]
pub(crate) struct AssistantAccessState {
    sessions: Mutex<HashMap<String, SessionAccess>>,
    plans: Mutex<VecDeque<(String, control_server::NativeChatBinding, String)>>,
    cancellations: Mutex<Vec<std::sync::Weak<AtomicBool>>>,
}

pub(crate) fn invalidate(app: &AppHandle) -> Result<(), String> {
    for cancellation in app
        .state::<AssistantAccessState>()
        .cancellations
        .lock()
        .map_err(|_| "채팅 작업을 잠글 수 없습니다")?
        .drain(..)
    {
        if let Some(cancellation) = cancellation.upgrade() {
            cancellation.store(true, Ordering::Release);
        }
    }
    app.state::<AssistantAccessState>()
        .sessions
        .lock()
        .map_err(|_| "채팅 범위를 잠글 수 없습니다")?
        .clear();
    app.state::<assistant_files::AssistantFilesState>()
        .invalidate_native()?;
    app.state::<assistant_tools::AssistantToolsState>()
        .invalidate_native()?;
    Ok(())
}

pub(crate) fn register_cancellation(
    app: &AppHandle,
    cancellation: &Arc<AtomicBool>,
) -> Result<(), String> {
    let state = app.state::<AssistantAccessState>();
    let mut cancellations = state
        .cancellations
        .lock()
        .map_err(|_| "채팅 작업을 잠글 수 없습니다")?;
    cancellations.retain(|item| item.strong_count() > 0);
    if !cancellations
        .iter()
        .any(|item| item.ptr_eq(&Arc::downgrade(cancellation)))
    {
        cancellations.push(Arc::downgrade(cancellation));
    }
    Ok(())
}

pub(crate) fn effective_root(
    app: &AppHandle,
    session_id: &str,
    anchor: &Path,
) -> Result<(PathBuf, bool), String> {
    let binding = control_server::native_chat_binding(app)?;
    let state = app.state::<AssistantAccessState>();
    let sessions = state
        .sessions
        .lock()
        .map_err(|_| "채팅 범위를 잠글 수 없습니다")?;
    if let Some(access) = sessions
        .get(session_id)
        .filter(|access| access.binding == binding)
    {
        if let Some(root) = &access.effective_root {
            if binding.mode != AssistantPermissionMode::FullAccess {
                return Err("전체 접근 권한이 변경됐습니다".into());
            }
            let live = bloomsweepy_core::validate_local_volume_directory(root)
                .map_err(|error| error.to_string())?;
            if &live != root
                || Some(
                    bloomsweepy_core::cleanup_tree_scope_identity(&live)
                        .map_err(|error| error.to_string())?,
                ) != access.effective_identity
            {
                return Err("드라이브 범위가 변경됐습니다".into());
            }
            return Ok((live, access.automatic_eligible));
        }
    }
    Ok((anchor.to_path_buf(), true))
}

pub(crate) fn register_review(
    app: &AppHandle,
    binding: control_server::NativeChatBinding,
    result: &mut AppToolResult,
) -> Result<(), String> {
    if result.status != AppToolStatus::ReviewRequired {
        return Ok(());
    }
    let Some(presentation) = result.presentation.as_mut() else {
        return Ok(());
    };
    presentation["assistantPermissionEpoch"] = json!(binding.epoch);
    let id = presentation["plan"]["planId"]
        .as_str()
        .or_else(|| presentation["preview"]["previewId"].as_str())
        .or_else(|| presentation["planId"].as_str())
        .map(str::to_owned);
    if let Some(id) = id {
        let state = app.state::<AssistantAccessState>();
        let mut plans = state
            .plans
            .lock()
            .map_err(|_| "채팅 검토 권한을 잠글 수 없습니다")?;
        record_plan(&mut plans, id, binding, result.capability.clone());
    }
    Ok(())
}

fn record_plan(
    plans: &mut VecDeque<(String, control_server::NativeChatBinding, String)>,
    id: String,
    binding: control_server::NativeChatBinding,
    family: String,
) {
    plans.retain(|(old, _, _)| old != &id);
    plans.push_back((id, binding, family.clone()));
    // Match the underlying engine, never evict a live plan from another engine.
    let limit = if family == "processes.review" { 8 } else { 1 };
    while plans.iter().filter(|(_, _, kind)| *kind == family).count() > limit {
        if let Some(index) = plans.iter().position(|(_, _, kind)| *kind == family) {
            plans.remove(index);
        }
    }
}

pub(crate) fn require_plan_current(app: &AppHandle, id: &str) -> Result<(), String> {
    with_plan_authority(app, id, || Ok(()))
}

pub(crate) fn plan_binding(
    app: &AppHandle,
    id: &str,
) -> Result<Option<control_server::NativeChatBinding>, String> {
    Ok(app
        .state::<AssistantAccessState>()
        .plans
        .lock()
        .map_err(|_| "채팅 검토 권한을 잠글 수 없습니다")?
        .iter()
        .find(|(plan, _, _)| plan.eq_ignore_ascii_case(id))
        .map(|(_, binding, _)| *binding))
}

pub(crate) fn register_plan_cancellation(
    app: &AppHandle,
    id: &str,
    cancellation: &Arc<AtomicBool>,
) -> Result<(), String> {
    if let Some(binding) = plan_binding(app, id)? {
        control_server::with_native_chat_authority(app, binding, true, || {
            register_cancellation(app, cancellation)
        })?;
    }
    Ok(())
}

pub(crate) fn with_plan_authority<T>(
    app: &AppHandle,
    id: &str,
    action: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let state = app.state::<AssistantAccessState>();
    let binding = state
        .plans
        .lock()
        .map_err(|_| "채팅 검토 권한을 잠글 수 없습니다")?
        .iter()
        .find(|(plan, _, _)| plan.eq_ignore_ascii_case(id))
        .map(|(_, binding, _)| *binding);
    if let Some(binding) = binding {
        control_server::with_native_chat_authority(app, binding, true, action)
    } else {
        action()
    }
}

pub(crate) fn is_access_action(action: &FileWorkspaceAction) -> bool {
    matches!(
        action,
        FileWorkspaceAction::Volumes {}
            | FileWorkspaceAction::OpenVolume { .. }
            | FileWorkspaceAction::SearchAll { .. }
            | FileWorkspaceAction::OpenSearchResult { .. }
    )
}

pub(crate) fn excluded_mounts(root: &Path) -> Vec<PathBuf> {
    crate::collect_system_overview()
        .volumes
        .into_iter()
        .map(|volume| mount_boundary(Path::new(&volume.mount_point)))
        .filter(|mount| mount != root && mount.starts_with(root))
        .collect()
}

fn mount_boundary(path: &Path) -> PathBuf {
    if let Ok(canonical) = bloomsweepy_core::validate_local_volume_directory(path) {
        return canonical;
    }
    #[cfg(windows)]
    {
        let raw = path.to_string_lossy().replace('/', "\\");
        if raw.as_bytes().get(1) == Some(&b':') {
            return PathBuf::from(format!("\\\\?\\{}", raw));
        }
    }
    path.to_path_buf()
}

fn native_volume_candidate(platform: &str, name: &str, mount: &str, disk_image: bool) -> bool {
    if disk_image {
        return false;
    }
    let identity = format!("{name} {mount}").to_lowercase();
    let compact: String = identity
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect();
    if [
        "googledrive",
        "googledrivefs",
        "drivefs",
        "onedrive",
        "dropbox",
        "iclouddrive",
        "pcloud",
        "boxsync",
    ]
    .iter()
    .any(|provider| compact.contains(provider))
        || (identity.contains('@')
            && identity.contains(" - google")
            && (identity.contains("...") || identity.contains('…')))
    {
        return false;
    }
    platform != "macos"
        || mount == "/"
        || mount == "/System/Volumes/Data"
        || mount.starts_with("/Volumes/")
}

pub(crate) fn confine_volume(scope: &Path, current: &Path) -> Result<(), String> {
    let root = bloomsweepy_core::validate_local_volume_directory(scope)
        .map_err(|error| error.to_string())?;
    let live = bloomsweepy_core::validate_local_volume_directory(current)
        .map_err(|error| error.to_string())?;
    if root != scope
        || live != current
        || !live.starts_with(&root)
        || bloomsweepy_core::cleanup_tree_scope_identity(&root)
            .map_err(|error| error.to_string())?
            .0
            != bloomsweepy_core::cleanup_tree_scope_identity(&live)
                .map_err(|error| error.to_string())?
                .0
        || excluded_mounts(&root)
            .iter()
            .any(|mount| live.starts_with(mount))
    {
        return Err("선택한 로컬 드라이브 경계가 변경됐습니다".into());
    }
    Ok(())
}

fn permission() -> AppToolResult {
    AppToolResult::with_status(
        "files.workspace",
        AppToolStatus::PermissionRequired,
        json!({"nativeOnly":true,"performed":false,"message":"로컬 드라이브 탐색은 전체 접근 모드에서 사용할 수 있습니다"}),
    )
}

pub(crate) async fn execute(
    app: &AppHandle,
    session_id: &str,
    action: &FileWorkspaceAction,
    cancellation: Arc<AtomicBool>,
) -> Result<AppToolResult, String> {
    let binding = control_server::native_chat_binding(app)?;
    if binding.mode != AssistantPermissionMode::FullAccess {
        return Ok(permission());
    }
    let session =
        crate::assistant_sessions::get_assistant_session(app.clone(), session_id.into()).await?;
    if session.session.scope_kind != crate::assistant_provider::AssistantScopeKind::Folder {
        return Ok(permission());
    }
    if let FileWorkspaceAction::OpenVolume {
        revision,
        volume_id,
    }
    | FileWorkspaceAction::OpenSearchResult {
        revision,
        entry_id: volume_id,
    } = action
    {
        let (root, current, eligible, expected, identity) = {
            let state = app.state::<AssistantAccessState>();
            let sessions = state
                .sessions
                .lock()
                .map_err(|_| "드라이브 목록을 잠글 수 없습니다")?;
            let access = sessions
                .get(session_id)
                .filter(|access| access.binding == binding && access.revision == *revision)
                .ok_or("드라이브 또는 검색 목록이 변경됐습니다")?;
            if matches!(action, FileWorkspaceAction::OpenVolume { .. }) {
                let volume = access
                    .volumes
                    .iter()
                    .find(|volume| volume.id == *volume_id)
                    .ok_or("현재 드라이브 목록에 없는 번호입니다")?;
                (
                    volume.root.clone(),
                    volume.root.clone(),
                    access.automatic_eligible && access.complete,
                    None,
                    volume.identity,
                )
            } else {
                let index = volume_id
                    .strip_prefix(&format!("{}-", access.revision))
                    .and_then(|number| number.parse::<usize>().ok())
                    .filter(|index| *index > 0 && *index <= access.entries.len())
                    .ok_or("현재 검색 목록에 없는 번호입니다")?
                    - 1;
                if *volume_id != format!("{}-{}", access.revision, index + 1) {
                    return Err("검색 번호가 올바르지 않습니다".into());
                }
                let entry = &access.entries[index];
                let current = Path::new(&entry.node.path)
                    .parent()
                    .ok_or("검색 대상의 부모 폴더가 없습니다")?
                    .to_path_buf();
                let exact_count = access
                    .entries
                    .iter()
                    .filter(|candidate| {
                        candidate.node.name.to_lowercase() == entry.node.name.to_lowercase()
                    })
                    .count();
                (
                    entry.volume.root.clone(),
                    current,
                    access.complete && exact_count == 1,
                    Some(entry.node.clone()),
                    entry.volume.identity,
                )
            }
        };
        let live = bloomsweepy_core::validate_local_volume_directory(&root)
            .map_err(|error| error.to_string())?;
        if live != root
            || !current.starts_with(&root)
            || bloomsweepy_core::cleanup_tree_scope_identity(&root)
                .map_err(|error| error.to_string())?
                != identity
        {
            return Err("드라이브 경계가 변경됐습니다".into());
        }
        control_server::with_native_chat_authority(app, binding, false, || {
            let state = app.state::<AssistantAccessState>();
            let mut sessions = state
                .sessions
                .lock()
                .map_err(|_| "채팅 범위를 잠글 수 없습니다")?;
            let access = sessions
                .get_mut(session_id)
                .ok_or("드라이브 목록이 변경됐습니다")?;
            access.effective_root = Some(root.clone());
            access.effective_identity = Some(identity);
            access.automatic_eligible = eligible;
            app.state::<assistant_files::AssistantFilesState>()
                .forget(session_id)?;
            app.state::<assistant_tools::AssistantToolsState>()
                .forget(session_id)
        })?;
        return assistant_files::open_native_root(
            app.clone(),
            session_id.into(),
            root,
            current,
            eligible,
            expected,
            binding,
            cancellation,
        )
        .await;
    }
    let task_app = app.clone();
    let key = session_id.to_owned();
    let action = action.clone();
    let runtime_cancel = app.state::<crate::ScanRuntime>().begin()?;
    register_cancellation(app, &runtime_cancel)?;
    let completion = crate::ScanCompletionGuard::new(app.clone());
    tauri::async_runtime::spawn_blocking(move || {
        let _completion = completion;
        let cancelled = || cancellation.load(Ordering::Acquire) || runtime_cancel.load(Ordering::Acquire) || !control_server::native_chat_binding_current(&task_app, binding);
        let overview = crate::collect_system_overview();
        let all_mounts: Vec<_> = overview.volumes.iter().map(|volume| mount_boundary(Path::new(&volume.mount_point))).collect();
        let mut volumes = Vec::new();
        let mut omitted = 0usize;
        for volume in overview.volumes {
            if cancelled() { return Err("드라이브 조회가 취소됐습니다".into()); }
            if !native_volume_candidate(std::env::consts::OS,&volume.name,&volume.mount_point,volume.is_disk_image) { continue; }
            let Ok(root) = bloomsweepy_core::validate_local_volume_directory(Path::new(&volume.mount_point)) else { omitted += 1; continue; };
            if volumes.iter().any(|prior: &Volume| prior.root == root) { continue; }
            if volumes.len() >= MAX_VOLUMES { omitted += 1; continue; }
            let Ok(identity)=bloomsweepy_core::cleanup_tree_scope_identity(&root) else { omitted+=1; continue; };
            volumes.push(Volume { id: assistant_tools::new_id()?, root, name: volume.name.chars().take(240).collect(), file_system: volume.file_system.chars().take(64).collect(), total_bytes: volume.total_bytes, available_bytes: volume.available_bytes, read_only: volume.read_only, identity });
        }
        let revision = assistant_tools::new_id()?;
        let mut entries = Vec::new();
        let mut visited = 0u64;
        let mut unreadable = 0u64;
        let mut scanned = 0usize;
        let mut truncated = omitted > 0;
        let mut path_bytes = 0usize;
        let query = if let FileWorkspaceAction::SearchAll { query } = &action { Some(query.clone()) } else { None };
        if let Some(query) = &query {
            let deadline = Instant::now() + Duration::from_secs(30);
            for volume in &volumes {
                if cancelled() { return Err("전체 이름 검색을 취소했습니다".into()); }
                if Instant::now() >= deadline || visited >= MAX_VISITED || entries.len() >= MAX_RESULTS || path_bytes >= MAX_PATH_BYTES { truncated = true; break; }
                let excluded: Vec<_> = all_mounts.iter().filter(|root| **root != volume.root && root.starts_with(&volume.root)).cloned().collect();
                match bloomsweepy_core::search_local_volume_entries(&volume.root, query, &excluded, MAX_VISITED - visited, MAX_RESULTS - entries.len(), MAX_PATH_BYTES - path_bytes, deadline, cancelled) {
                    Ok(report) => {
                        visited = visited.saturating_add(report.visited_entries);
                        unreadable = unreadable.saturating_add(report.unreadable_entries);
                        truncated |= report.truncated;
                        scanned += 1;
                        for node in report.entries { path_bytes = path_bytes.saturating_add(node.path.len().saturating_mul(6)); entries.push(SearchEntry { node, volume: volume.clone() }); }
                    }
                    Err(bloomsweepy_core::ScanError::Cancelled) => return Err("전체 이름 검색을 취소했습니다".into()),
                    Err(_) => { unreadable += 1; truncated = true; }
                }
            }
            truncated |= scanned != volumes.len();
        }
        let volume_data: Vec<_> = volumes.iter().map(|volume| json!({"id":volume.id,"name":volume.name,"fileSystem":volume.file_system,"totalBytes":volume.total_bytes,"availableBytes":volume.available_bytes,"readOnly":volume.read_only})).collect();
        let entry_data: Vec<_> = entries.iter().enumerate().map(|(index, entry)| json!({"id":format!("{}-{}",revision,index+1),"name":entry.node.name.chars().take(240).collect::<String>(),"volumeId":entry.volume.id,"volumeName":entry.volume.name,"isDirectory":entry.node.is_directory,"logicalBytes":if entry.node.is_directory {None} else {Some(entry.node.logical_bytes)}})).collect();
        // The model sees one bounded page; native IDs retain the bounded local matches.
        let data = json!({"nativeOnly":true,"accessKind":if query.is_some(){"search"}else{"volumes"},"revision":revision,"query":query,"volumes":volume_data,"entries":entry_data.iter().take(24).collect::<Vec<_>>(),"totalEntries":entries.len(),"truncated":truncated || entries.len()>24,"partial":truncated || unreadable>0,"unreadableEntries":unreadable,"visitedEntries":visited,"scannedVolumes":scanned,"totalVolumes":volumes.len()+omitted,"deleted":false});
        let local_entries: Vec<_> = entries.iter().enumerate().map(|(index,entry)| json!({"id":format!("{}-{}",revision,index+1),"name":entry.node.name,"path":entry.node.path,"volumeId":entry.volume.id,"volumeName":entry.volume.name,"isDirectory":entry.node.is_directory})).collect();
        let presentation = json!({"view":"overview","reviewKind":"fileAccess","workspaceKey":key,"access":data,"entries":local_entries});
        control_server::with_native_chat_authority(&task_app, binding, false, || {
            let state = task_app.state::<AssistantAccessState>();
            let mut sessions = state.sessions.lock().map_err(|_| "채팅 범위를 잠글 수 없습니다")?;
            if !sessions.contains_key(&key) && sessions.len() >= MAX_SESSIONS { if let Some(old)=sessions.keys().next().cloned() { sessions.remove(&old); } }
            let previous = sessions.remove(&key).filter(|old| old.binding == binding);
            sessions.insert(key, SessionAccess { binding, effective_root:previous.as_ref().and_then(|old|old.effective_root.clone()), effective_identity:previous.as_ref().and_then(|old|old.effective_identity), automatic_eligible:previous.as_ref().is_none_or(|old|old.automatic_eligible) && !truncated && unreadable==0, revision, volumes, entries, complete:!truncated && unreadable==0 });
            let mut result=AppToolResult::completed("files.workspace",data).with_presentation(presentation);
            result.truncated=truncated || unreadable>0;
            Ok(result)
        })
    }).await.map_err(|error|format!("드라이브 조회가 중단됐습니다: {error}"))?
}

#[tauri::command]
pub(crate) async fn assistant_file_access_action(
    app: AppHandle,
    window: WebviewWindow,
    session_id: String,
    operation: FileWorkspaceAction,
) -> Result<AppToolResult, String> {
    if window.label() != "main" {
        return Err("앱의 기본 창에서만 드라이브를 탐색할 수 있습니다".into());
    }
    let session =
        crate::assistant_sessions::get_assistant_session(app.clone(), session_id.clone()).await?;
    let root = (session.session.scope_kind
        == crate::assistant_provider::AssistantScopeKind::Folder)
        .then(|| PathBuf::from(session.session.scope_root));
    crate::app_tools::execute(
        &app,
        &crate::app_tools::ToolScope::Native { session_id, root },
        &AppToolRequest::FileWorkspace { operation },
        Arc::new(AtomicBool::new(false)),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_volume_policy_excludes_cloud_images_and_internal_os_mounts() {
        for mount in [
            "/System/Volumes/Preboot",
            "/System/Volumes/VM",
            "/System/Volumes/Update",
            "/System/Volumes/Recovery",
            "/dev",
        ] {
            assert!(!native_volume_candidate("macos", "System", mount, false));
        }
        for mount in ["/", "/System/Volumes/Data", "/Volumes/Archive"] {
            assert!(native_volume_candidate("macos", "Local", mount, false));
        }
        assert!(!native_volume_candidate(
            "macos",
            "Local",
            "/Volumes/Image",
            true
        ));
        for name in [
            "GoogleDriveFS",
            "OneDrive",
            "Dropbox",
            "iCloud Drive",
            "pCloud",
            "account@gmail.com - Google...",
        ] {
            assert!(!native_volume_candidate("windows", name, "G:\\", false));
        }
        assert!(native_volume_candidate(
            "windows",
            "Google archive",
            "D:\\",
            false
        ));
        assert!(native_volume_candidate("windows", "Local", "C:\\", false));
    }
    #[test]
    fn plan_tombstones_are_bounded_per_engine_not_global_review_traffic() {
        let mut plans = VecDeque::new();
        let binding = control_server::NativeChatBinding {
            epoch: 3,
            mode: AssistantPermissionMode::Confirm,
        };
        record_plan(
            &mut plans,
            "app".into(),
            binding,
            "applications.review".into(),
        );
        record_plan(&mut plans, "docker".into(), binding, "docker.review".into());
        for index in 0..300 {
            record_plan(
                &mut plans,
                format!("process-{index}"),
                binding,
                "processes.review".into(),
            );
        }
        assert_eq!(plans.len(), 10);
        assert!(plans.iter().any(|(id, _, _)| id == "app"));
        assert!(plans.iter().any(|(id, _, _)| id == "docker"));
        record_plan(
            &mut plans,
            "app-new".into(),
            binding,
            "applications.review".into(),
        );
        assert!(!plans.iter().any(|(id, _, _)| id == "app"));
        assert!(plans.iter().any(|(id, _, _)| id == "app-new"));
    }
}
