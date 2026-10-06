//! App-owned file workspace. Models can browse and request review, never approve.
use crate::assistant_provider::{AssistantFolderSummary, AssistantScopeKind};
use crate::{
    ScanCompletionGuard, ScanRuntime, StoredReports, assistant_sessions, assistant_tools,
    trash_actions,
};
use bloomsweepy_control::{
    AppToolResult, AppToolStatus, ControlOperationState, FileWorkspaceAction,
};
use bloomsweepy_core::{
    DirectoryNode, DirectoryScanConfig, VerifiedTrashItem, scan_directory_level,
    search_local_entries, validate_directory_trash_file, validate_directory_trash_folder,
    validate_local_directory_path,
};
use serde::Serialize;
use serde_json::json;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};

const PAGE_SIZE: usize = 24;
const MAX_WORKSPACES: usize = 16;
const MAX_SELECTION: usize = 100;

pub(crate) type FileAction = FileWorkspaceAction;
const EXTERNAL_WORKSPACE_KEY: &str = "external-files.workspace";

#[derive(Default)]
struct ExternalWorkspaceState {
    // minimal: one shared external workspace and last outcome — add per-client isolation only with authenticated transport identities.
    binding: Option<crate::control_server::FileWorkspaceBinding>,
    operation: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileEntryView {
    id: String,
    number: usize,
    name: String,
    path: String,
    is_directory: bool,
    logical_bytes: Option<u64>,
    file_count: Option<u64>,
    directory_count: Option<u64>,
    link_count: Option<u64>,
    modified_at_unix_ms: Option<u128>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileReviewPlan {
    id: String,
    entries: Vec<FileEntryView>,
    logical_bytes: u64,
    requires_nested_ack: bool,
    expires_at_unix_ms: Option<u64>,
    #[serde(skip)]
    verified: Vec<VerifiedTrashItem>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileWorkspaceView {
    revision: String,
    current_path: String,
    current_name: String,
    can_go_up: bool,
    query: Option<String>,
    size_ranked: bool,
    map_generation: Option<u64>,
    summary: AssistantFolderSummary,
    total_entries: usize,
    truncated: bool,
    unreadable_entries: u64,
    offset: usize,
    next_offset: Option<usize>,
    entries: Vec<FileEntryView>,
    selected_ids: Vec<String>,
    plan: Option<FileReviewPlan>,
}

struct Workspace {
    revision: String,
    scope: PathBuf,
    current: PathBuf,
    nodes: Vec<DirectoryNode>,
    summary: AssistantFolderSummary,
    query: Option<String>,
    size_ranked: bool,
    map_generation: Option<u64>,
    truncated: bool,
    unreadable: u64,
    offset: usize,
    selected: Vec<String>,
    plan: Option<FileReviewPlan>,
}

impl Workspace {
    fn entry(&self, index: usize) -> FileEntryView {
        let node = &self.nodes[index];
        let measured = self.query.is_none() || !node.is_directory;
        FileEntryView {
            id: format!("{}-{}", self.revision, index + 1),
            number: index + 1,
            name: node.name.clone(),
            path: node.path.clone(),
            is_directory: node.is_directory,
            logical_bytes: measured.then_some(node.logical_bytes),
            file_count: measured.then_some(node.file_count),
            directory_count: measured.then_some(node.directory_count),
            link_count: None,
            modified_at_unix_ms: node.modified_at_unix_ms,
        }
    }
    fn index(&self, id: &str) -> Result<usize, String> {
        id.strip_prefix(&format!("{}-", self.revision))
            .and_then(|number| number.parse::<usize>().ok())
            .filter(|&number| {
                number > 0
                    && number <= self.nodes.len()
                    && id == format!("{}-{number}", self.revision)
            })
            .map(|number| number - 1)
            .ok_or_else(|| "현재 목록에 없는 항목입니다. 다시 검사하세요".into())
    }
    fn view(&self) -> FileWorkspaceView {
        FileWorkspaceView {
            revision: self.revision.clone(),
            current_path: self.current.display().to_string(),
            current_name: self
                .current
                .file_name()
                .unwrap_or(self.current.as_os_str())
                .to_string_lossy()
                .into_owned(),
            can_go_up: self.current != self.scope,
            query: self.query.clone(),
            size_ranked: self.size_ranked,
            map_generation: self.map_generation,
            summary: self.summary.clone(),
            total_entries: self.nodes.len(),
            truncated: self.truncated,
            unreadable_entries: self.unreadable,
            offset: self.offset,
            next_offset: (self.offset + PAGE_SIZE < self.nodes.len())
                .then_some(self.offset + PAGE_SIZE),
            entries: (self.offset..self.nodes.len())
                .take(PAGE_SIZE)
                .map(|i| self.entry(i))
                .collect(),
            selected_ids: self.selected.clone(),
            plan: self.plan.clone(),
        }
    }
    fn check_page_ids(&self, ids: &[String]) -> Result<(), String> {
        validate_ids(ids)?;
        for id in ids {
            let index = self.index(id)?;
            if index < self.offset || index >= self.offset + PAGE_SIZE {
                return Err("AI에 전달된 현재 페이지에서만 항목을 지정할 수 있습니다".into());
            }
        }
        Ok(())
    }
}

#[derive(Default)]
pub(crate) struct AssistantFilesState(
    Mutex<HashMap<String, Workspace>>,
    Mutex<ExternalWorkspaceState>,
);
impl AssistantFilesState {
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, HashMap<String, Workspace>>, String> {
        self.0
            .lock()
            .map_err(|_| "파일 대화 상태를 읽지 못했습니다".into())
    }
    pub(crate) fn forget(&self, session_id: &str) -> Result<(), String> {
        self.lock()?.remove(session_id);
        Ok(())
    }
    fn view(&self, session_id: &str) -> Result<Option<FileWorkspaceView>, String> {
        Ok(self.lock()?.get(session_id).map(Workspace::view))
    }
    pub(crate) fn prompt_context(&self, session_id: &str) -> Result<String, String> {
        let workspaces = self.lock()?;
        let Some(workspace) = workspaces.get(session_id) else {
            return Ok(json!({"freshScan":false}).to_string());
        };
        Ok(model_workspace_context(&workspace.view()).to_string())
    }
    fn insert(&self, session_id: &str, workspace: Workspace) -> Result<FileWorkspaceView, String> {
        let view = workspace.view();
        let mut workspaces = self.lock()?;
        if !workspaces.contains_key(session_id)
            && workspaces.len() >= MAX_WORKSPACES
            && let Some(key) = workspaces.keys().next().cloned()
        {
            workspaces.remove(&key);
        }
        workspaces.insert(session_id.into(), workspace);
        Ok(view)
    }
    fn select(
        &self,
        session_id: &str,
        revision: &str,
        ids: Vec<String>,
    ) -> Result<FileWorkspaceView, String> {
        validate_ids(&ids)?;
        let mut workspaces = self.lock()?;
        let workspace = workspace_mut(&mut workspaces, session_id, revision)?;
        for id in &ids {
            workspace.index(id)?;
        }
        workspace.selected = ids;
        workspace.plan = None;
        Ok(workspace.view())
    }
    fn claim(
        &self,
        session_id: &str,
        revision: &str,
        plan_id: &str,
        nested_ack: bool,
    ) -> Result<Vec<VerifiedTrashItem>, String> {
        let mut workspaces = self.lock()?;
        let workspace = workspace_mut(&mut workspaces, session_id, revision)?;
        let plan = workspace
            .plan
            .as_ref()
            .ok_or("확인할 계획이 없습니다. 다시 검토하세요")?;
        if plan.id != plan_id {
            return Err("확인 대상이 변경됐습니다. 다시 검토하세요".into());
        }
        if plan.requires_nested_ack && !nested_ack {
            return Err("폴더의 하위 항목 전체가 이동함을 확인해 주세요".into());
        }
        let plan = workspace.plan.take().expect("checked plan");
        // Consume before I/O. Failure cannot make a destructive operation replayable.
        workspace.nodes.clear();
        workspace.selected.clear();
        Ok(plan.verified)
    }
}

fn model_workspace_context(view: &FileWorkspaceView) -> serde_json::Value {
    let entries: Vec<_> = view.entries.iter().map(|entry| json!({"id":entry.id,"number":entry.number,
        "name":entry.name.chars().take(240).collect::<String>(),"kind":if entry.is_directory {"directory"} else {"file"},
        "logicalBytes":entry.logical_bytes,"files":entry.file_count,"directories":entry.directory_count,
        "modifiedAtUnixMs":entry.modified_at_unix_ms,
        "selected":view.selected_ids.contains(&entry.id)})).collect();
    json!({"freshScan":true,"revision":view.revision,"currentFolder":view.current_name,"canGoUp":view.can_go_up,
        "query":view.query,"sizeRanked":view.size_ranked,"scanCompletedAtUnixMs":view.summary.completed_at_unix_ms,
        "rankingScope":"direct children; folder sizes include descendants; logical bytes, not reclaimable space",
        "deletionSafety":"unknown; names, sizes and modification times cannot establish backup, necessity or reproducibility",
        "totalEntries":view.total_entries,"truncated":view.truncated,"unreadable":view.unreadable_entries,
        "offset":view.offset,"nextOffset":view.next_offset,"entries":entries,"selectedCount":view.selected_ids.len(),
        "reviewReady":view.plan.is_some(),"approval":"main app human decision or native opt-in exact-named removal; model cannot approve"})
}

fn validate_ids(ids: &[String]) -> Result<(), String> {
    if ids.len() > MAX_SELECTION
        || ids.iter().any(|id| id.len() > 80)
        || ids.iter().collect::<HashSet<_>>().len() != ids.len()
    {
        return Err("중복 없이 최대 100개 항목을 선택하세요".into());
    }
    Ok(())
}
fn workspace_mut<'a>(
    map: &'a mut HashMap<String, Workspace>,
    session_id: &str,
    revision: &str,
) -> Result<&'a mut Workspace, String> {
    map.get_mut(session_id)
        .filter(|workspace| workspace.revision == revision)
        .ok_or_else(|| "파일 목록이 변경됐습니다. 다시 검사하세요".into())
}
fn confined(scope: &Path, path: &Path) -> Result<PathBuf, String> {
    if !path.starts_with(scope) {
        return Err("선택한 대화 폴더 밖으로 이동할 수 없습니다".into());
    }
    let scope_live = validate_local_directory_path(scope).map_err(|error| error.to_string())?;
    let path_live = validate_local_directory_path(path).map_err(|error| error.to_string())?;
    if scope_live != scope || path_live != path || !path_live.starts_with(&scope_live) {
        return Err("폴더 경계가 변경됐습니다. 새 대화에서 다시 선택하세요".into());
    }
    Ok(path_live)
}

fn scan_workspace(
    scope: PathBuf,
    current: PathBuf,
    reports: &StoredReports,
    cancellation: &impl Fn() -> bool,
) -> Result<Workspace, String> {
    let current = confined(&scope, &current)?;
    let report = scan_directory_level(
        &current,
        DirectoryScanConfig::default(),
        |_| {},
        cancellation,
    )
    .map_err(|error| error.to_string())?;
    let summary = assistant_tools::folder_summary(&report);
    // One bounded snapshot is shared with the treemap; workspaces retain only its generation.
    let result = reports.replace_directory(report)?;
    let report = result.report;
    Ok(Workspace {
        revision: assistant_tools::new_id()?,
        scope,
        current,
        nodes: report.children,
        summary,
        query: None,
        size_ranked: false,
        map_generation: Some(result.generation),
        truncated: report.children_truncated || report.tracking_limit_reached,
        unreadable: report.unreadable_entries,
        offset: 0,
        selected: Vec::new(),
        plan: None,
    })
}

fn prepare(
    state: &AssistantFilesState,
    session_id: &str,
    revision: &str,
    cancelled: &impl Fn() -> bool,
) -> Result<FileWorkspaceView, String> {
    let started = Instant::now();
    let cancelled = || cancelled() || started.elapsed() > Duration::from_secs(60);
    let (scope, selected) = {
        let mut workspaces = state.lock()?;
        let workspace = workspace_mut(&mut workspaces, session_id, revision)?;
        // Re-preparation invalidates the previous plan even if validation fails.
        workspace.plan = None;
        if workspace.selected.is_empty() {
            return Err("휴지통으로 보낼 항목을 먼저 선택하세요".into());
        }
        let nodes: Result<Vec<_>, String> = workspace
            .selected
            .iter()
            .map(|id| {
                workspace
                    .index(id)
                    .map(|index| (workspace.entry(index), workspace.nodes[index].clone()))
            })
            .collect();
        (workspace.scope.clone(), nodes?)
    };
    for (_, node) in &selected {
        if selected.iter().any(|(_, parent)| {
            parent.is_directory
                && parent.path != node.path
                && Path::new(&node.path).starts_with(&parent.path)
        }) {
            return Err("폴더와 그 안의 항목을 함께 선택할 수 없습니다. 상위 폴더 또는 하위 항목만 선택하세요".into());
        }
    }
    let mut verified = Vec::new();
    let mut entries = Vec::new();
    // One parent report at a time; no retained per-parent scan cache.
    let mut parent_report: Option<bloomsweepy_core::DirectoryScanReport> = None;
    for (mut entry, old) in selected {
        if cancelled() {
            return Err("파일 검토를 취소했습니다".into());
        }
        let parent = Path::new(&old.path)
            .parent()
            .ok_or("부모 폴더가 없습니다")?;
        confined(&scope, parent)?;
        if parent_report
            .as_ref()
            .is_none_or(|report| Path::new(&report.root) != parent)
        {
            parent_report = Some(
                scan_directory_level(parent, DirectoryScanConfig::default(), |_| {}, cancelled)
                    .map_err(|error| error.to_string())?,
            );
        }
        let report = parent_report.as_ref().expect("parent report loaded");
        let live = report
            .children
            .iter()
            .find(|node| node.path == old.path)
            .ok_or("대상이 현재 검사 상한 밖이거나 없어졌습니다. 더 작은 폴더를 탐색하세요")?;
        if !old.same_entry_as(live) {
            return Err("검색 후 대상이 변경됐습니다. 다시 검사하고 검토하세요".into());
        }
        let item = if live.is_directory {
            validate_directory_trash_folder(report, &live.path, cancelled)
        } else {
            validate_directory_trash_file(report, &live.path, cancelled)
        }
        .map_err(|error| error.to_string())?;
        entry.logical_bytes = Some(item.logical_bytes());
        entry.file_count = Some(live.file_count);
        entry.directory_count = Some(live.directory_count);
        entry.link_count = Some(item.directory_link_count());
        entries.push(entry);
        verified.push(item);
    }
    if cancelled() {
        return Err("파일 검토를 취소했습니다".into());
    }
    let plan = FileReviewPlan {
        id: assistant_tools::new_id()?,
        logical_bytes: verified.iter().fold(0_u64, |total, item| {
            total.saturating_add(item.logical_bytes())
        }),
        requires_nested_ack: entries.iter().any(|entry| entry.is_directory),
        entries,
        expires_at_unix_ms: None,
        verified,
    };
    let mut workspaces = state.lock()?;
    let workspace = workspace_mut(&mut workspaces, session_id, revision)?;
    if workspace.selected
        != plan
            .entries
            .iter()
            .map(|entry| entry.id.clone())
            .collect::<Vec<_>>()
    {
        return Err("검토 중 선택이 변경됐습니다".into());
    }
    workspace.plan = Some(plan);
    Ok(workspace.view())
}

pub(crate) async fn dispatch(
    app: AppHandle,
    session_id: String,
    operation: FileAction,
    request_cancel: Arc<AtomicBool>,
) -> Result<FileWorkspaceView, String> {
    let session =
        assistant_sessions::get_assistant_session(app.clone(), session_id.clone()).await?;
    if session.session.scope_kind != AssistantScopeKind::Folder {
        return Err("폴더 대화에서만 사용할 수 있습니다".into());
    }
    let scope = PathBuf::from(session.session.scope_root);
    app.state::<assistant_tools::AssistantToolsState>()
        .forget(&session_id)?;
    let view = dispatch_root(
        app.clone(),
        FileDispatchRoot {
            key: session_id.clone(),
            scope: scope.clone(),
            summary: session.folder_summary,
            binding: None,
        },
        operation,
        request_cancel,
        None,
    )
    .await?;
    if view.query.is_none() && Path::new(&view.current_path) == scope {
        assistant_sessions::update_folder_summary(app, session_id, view.summary.clone()).await?;
    }
    Ok(view)
}

struct FileDispatchRoot {
    key: String,
    scope: PathBuf,
    summary: AssistantFolderSummary,
    binding: Option<(crate::control_server::FileWorkspaceBinding, bool)>,
}

async fn dispatch_root(
    app: AppHandle,
    context: FileDispatchRoot,
    operation: FileAction,
    request_cancel: Arc<AtomicBool>,
    reserved_cancellation: Option<Arc<AtomicBool>>,
) -> Result<FileWorkspaceView, String> {
    operation.validate().map_err(|error| error.to_string())?;
    let (cancellation, completion) = if let Some(cancellation) = reserved_cancellation {
        (cancellation, None)
    } else {
        (
            app.state::<ScanRuntime>().begin()?,
            Some(ScanCompletionGuard::new(app.clone())),
        )
    };
    let worker_app = app.clone();
    let worker_completion = completion.clone();
    tauri::async_runtime::spawn_blocking(move || {
        // Dropping the IPC future must not release the native scan lease while blocking work remains.
        let _worker_completion = worker_completion;
        let cancelled = || {
            cancellation.load(Ordering::Acquire)
                || request_cancel.load(Ordering::Acquire)
                || context.binding.as_ref().is_some_and(|(binding, review)| {
                    !crate::control_server::file_workspace_epochs_current(
                        &worker_app,
                        binding,
                        *review,
                    )
                })
        };
        apply_workspace_action(
            &worker_app.state::<AssistantFilesState>(),
            &worker_app.state::<StoredReports>(),
            &context.key,
            context.scope.clone(),
            context.summary.clone(),
            operation,
            &cancelled,
        )
    })
    .await
    .map_err(|error| format!("파일 대화 작업이 중단됐습니다: {error}"))?
}

fn apply_workspace_action(
    state: &AssistantFilesState,
    reports: &StoredReports,
    worker_session: &str,
    scope: PathBuf,
    initial_summary: AssistantFolderSummary,
    operation: FileAction,
    cancelled: &impl Fn() -> bool,
) -> Result<FileWorkspaceView, String> {
    operation.validate().map_err(|error| error.to_string())?;
    if cancelled() {
        return Err("대화 작업을 취소했습니다".into());
    }
    let named = match &operation {
        FileAction::ReviewNamed { name } => Some(name.clone()),
        _ => None,
    };
    let operation = if let Some(name) = &named {
        FileAction::Search {
            query: name.clone(),
        }
    } else {
        operation
    };
    let view = match operation {
        FileAction::Scan {} | FileAction::Largest {} => {
            let size_ranked = matches!(operation, FileAction::Largest {});
            let current = state
                .lock()?
                .get(worker_session)
                .map(|workspace| workspace.current.clone())
                .unwrap_or(scope.clone());
            state.forget(worker_session)?;
            let mut workspace = scan_workspace(scope, current, reports, cancelled)?;
            workspace.size_ranked = size_ranked;
            state.insert(worker_session, workspace)
        }
        FileAction::Search { query } => {
            let current = state
                .lock()?
                .get(worker_session)
                .map(|workspace| workspace.current.clone())
                .unwrap_or(scope.clone());
            state.forget(worker_session)?;
            let current = confined(&scope, &current)?;
            let report = search_local_entries(&current, &query, cancelled)
                .map_err(|error| error.to_string())?;
            state.insert(
                worker_session,
                Workspace {
                    revision: assistant_tools::new_id()?,
                    scope,
                    current,
                    nodes: report.entries,
                    summary: initial_summary,
                    query: Some(query),
                    size_ranked: false,
                    map_generation: None,
                    truncated: report.truncated,
                    unreadable: report.unreadable_entries,
                    offset: 0,
                    selected: Vec::new(),
                    plan: None,
                },
            )
        }
        FileAction::Browse { revision, entry_id } => {
            let target = {
                let workspaces = state.lock()?;
                let workspace = workspaces
                    .get(worker_session)
                    .filter(|workspace| workspace.revision == revision)
                    .ok_or("파일 목록이 변경됐습니다")?;
                workspace.check_page_ids(std::slice::from_ref(&entry_id))?;
                let node = &workspace.nodes[workspace.index(&entry_id)?];
                if !node.is_directory {
                    return Err("폴더만 하위 탐색할 수 있습니다".into());
                }
                PathBuf::from(&node.path)
            };
            state.forget(worker_session)?;
            state.insert(
                worker_session,
                scan_workspace(scope, target, reports, cancelled)?,
            )
        }
        FileAction::Parent { revision } => {
            let target = {
                let workspaces = state.lock()?;
                let workspace = workspaces
                    .get(worker_session)
                    .filter(|workspace| workspace.revision == revision)
                    .ok_or("파일 목록이 변경됐습니다")?;
                if workspace.current == scope {
                    return Err("대화 폴더 밖으로 이동할 수 없습니다".into());
                }
                workspace
                    .current
                    .parent()
                    .ok_or("부모 폴더가 없습니다")?
                    .to_owned()
            };
            state.forget(worker_session)?;
            state.insert(
                worker_session,
                scan_workspace(scope, target, reports, cancelled)?,
            )
        }
        FileAction::Page { revision, offset } => {
            let mut workspaces = state.lock()?;
            let workspace = workspace_mut(&mut workspaces, worker_session, &revision)?;
            if offset != 0 && offset >= workspace.nodes.len() {
                return Err("목록 페이지 범위를 벗어났습니다".into());
            }
            workspace.offset = offset;
            Ok(workspace.view())
        }
        FileAction::Select {
            revision,
            include_ids,
            exclude_ids,
        } => {
            let selected = {
                let workspaces = state.lock()?;
                let workspace = workspaces
                    .get(worker_session)
                    .filter(|workspace| workspace.revision == revision)
                    .ok_or("파일 목록이 변경됐습니다")?;
                workspace.check_page_ids(&include_ids)?;
                workspace.check_page_ids(&exclude_ids)?;
                if include_ids.iter().any(|id| exclude_ids.contains(id)) {
                    return Err("같은 항목을 포함하고 제외할 수 없습니다".into());
                }
                let mut selected = workspace.selected.clone();
                selected.retain(|id| !exclude_ids.contains(id));
                for id in include_ids {
                    if !selected.contains(&id) {
                        selected.push(id);
                    }
                }
                selected
            };
            state.select(worker_session, &revision, selected)
        }
        FileAction::Review { revision, ids } => {
            state
                .lock()?
                .get(worker_session)
                .filter(|workspace| workspace.revision == revision)
                .ok_or("파일 목록이 변경됐습니다")?
                .check_page_ids(&ids)?;
            state.select(worker_session, &revision, ids)?;
            prepare(state, worker_session, &revision, cancelled)
        }
        FileAction::Status {} => state
            .view(worker_session)?
            .ok_or_else(|| "현재 파일 검사 결과가 없습니다".into()),
        FileAction::ReviewNamed { .. } => unreachable!("normalized to exact name search"),
    }?;
    if let Some(name) = named {
        prepare_unique_name(state, worker_session, view, &name, cancelled)
    } else {
        Ok(view)
    }
}

fn prepare_unique_name(
    state: &AssistantFilesState,
    key: &str,
    view: FileWorkspaceView,
    name: &str,
    cancelled: &impl Fn() -> bool,
) -> Result<FileWorkspaceView, String> {
    let id = {
        let mut workspaces = state.lock()?;
        let workspace = workspace_mut(&mut workspaces, key, &view.revision)?;
        let Some(id) = unique_exact_id(workspace, name) else {
            return Ok(view);
        };
        workspace.offset = workspace.index(&id)? / PAGE_SIZE * PAGE_SIZE;
        id
    };
    state.select(key, &view.revision, vec![id])?;
    prepare(state, key, &view.revision, cancelled)
}

pub(crate) async fn review_named(
    app: AppHandle,
    session_id: String,
    name: String,
    cancelled: Arc<AtomicBool>,
) -> Result<FileWorkspaceView, String> {
    dispatch(app, session_id, FileAction::ReviewNamed { name }, cancelled).await
}

fn unique_exact_id(workspace: &Workspace, name: &str) -> Option<String> {
    if workspace.truncated || workspace.unreadable > 0 {
        return None;
    }
    let name = name.trim().to_lowercase();
    let mut matches = workspace
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.name.to_lowercase() == name);
    let (index, _) = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    Some(workspace.entry(index).id)
}

fn external_permission() -> AppToolResult {
    AppToolResult::with_status(
        "files.workspace",
        AppToolStatus::PermissionRequired,
        json!({"workspace":{"freshScan":false},"reviewPrepared":false,"deleted":false,
            "reason":"앱에서 검사 폴더와 정리 검토 허용을 확인해 주세요","permissionChanged":false}),
    )
}

fn needs_review_access(action: &FileAction) -> bool {
    matches!(
        action,
        FileAction::Select { .. } | FileAction::Review { .. } | FileAction::ReviewNamed { .. }
    )
}
fn heavy_action(action: &FileAction) -> bool {
    matches!(
        action,
        FileAction::Scan {}
            | FileAction::Largest {}
            | FileAction::Search { .. }
            | FileAction::Browse { .. }
            | FileAction::Parent { .. }
            | FileAction::Review { .. }
            | FileAction::ReviewNamed { .. }
    )
}

impl AssistantFilesState {
    fn external_lock(&self) -> Result<std::sync::MutexGuard<'_, ExternalWorkspaceState>, String> {
        self.1
            .lock()
            .map_err(|_| "외부 파일 조회 상태를 확인하지 못했습니다".into())
    }
    fn bind_external(
        &self,
        binding: &crate::control_server::FileWorkspaceBinding,
    ) -> Result<(), String> {
        let mut external = self.external_lock()?;
        let mut workspaces = self.lock()?;
        if external
            .binding
            .as_ref()
            .is_none_or(|old| !old.same_scope(binding))
        {
            workspaces.remove(EXTERNAL_WORKSPACE_KEY);
            external.operation = None;
        } else if external
            .binding
            .as_ref()
            .is_some_and(|old| old.cleanup_epoch != binding.cleanup_epoch)
            && let Some(workspace) = workspaces.get_mut(EXTERNAL_WORKSPACE_KEY)
        {
            workspace.selected.clear();
            workspace.plan = None;
        }
        external.binding = Some(binding.clone());
        Ok(())
    }
    fn invalidate_external(&self) -> Result<(), String> {
        let mut external = self.external_lock()?;
        self.lock()?.remove(EXTERNAL_WORKSPACE_KEY);
        external.binding = None;
        external.operation = None;
        Ok(())
    }
    fn tool_result(
        &self,
        key: &str,
        status: Option<AppToolStatus>,
    ) -> Result<AppToolResult, String> {
        let view = self.view(key)?;
        let mut context = view
            .as_ref()
            .map(model_workspace_context)
            .unwrap_or_else(|| json!({"freshScan":false}));
        let mut last_operation = None;
        let mut inferred = if view.as_ref().is_some_and(|view| view.plan.is_some()) {
            AppToolStatus::ReviewRequired
        } else {
            AppToolStatus::Completed
        };
        if key == EXTERNAL_WORKSPACE_KEY {
            context["approval"] = json!(
                "local main-app final confirmation only; external models cannot approve or execute"
            );
            last_operation = self.external_lock()?.operation.clone();
            match last_operation
                .as_ref()
                .and_then(|value| value["state"].as_str())
            {
                Some("running" | "executing") => {
                    inferred = AppToolStatus::Running;
                    context = json!({"freshScan":false,"scanInProgress":true});
                }
                Some("failed") => inferred = AppToolStatus::Failed,
                _ => {}
            }
        }
        let review = view.as_ref().is_some_and(|view| view.plan.is_some());
        let mut result = AppToolResult::with_status(
            "files.workspace",
            status.unwrap_or(inferred),
            json!({"workspace":context,"reviewPrepared":review,"deleted":false,"lastOperation":last_operation}),
        );
        result.truncated = view
            .as_ref()
            .is_some_and(|view| view.truncated || view.unreadable_entries > 0);
        if let Some(view) = view {
            result.presentation = Some(
                json!({"view":"overview","reviewKind":"files","workspaceKey":key,"workspace":view}),
            );
        }
        Ok(result)
    }
}

fn unknown_summary(root: &Path) -> AssistantFolderSummary {
    AssistantFolderSummary {
        scope_name: root
            .file_name()
            .unwrap_or(root.as_os_str())
            .to_string_lossy()
            .into_owned(),
        completed_at_unix_ms: 0,
        total_logical_bytes: 0,
        total_files: 0,
        total_directories: 0,
        unreadable_entries: 0,
        empty_directory_count: 0,
        children_truncated: true,
        children: Vec::new(),
    }
}

/// The canonical adapter never accepts a session/path/approval from an external caller.
pub(crate) async fn execute_tool(
    app: &AppHandle,
    scope: &crate::app_tools::ToolScope,
    operation: &FileWorkspaceAction,
    cancellation: Arc<AtomicBool>,
) -> Result<AppToolResult, String> {
    operation.validate().map_err(|error| error.to_string())?;
    if cancellation.load(Ordering::Acquire) {
        return Err("파일 조회를 취소했습니다".into());
    }
    let files = app.state::<AssistantFilesState>();
    if let crate::app_tools::ToolScope::Native { session_id, .. } = scope {
        let session =
            assistant_sessions::get_assistant_session(app.clone(), session_id.clone()).await?;
        if session.session.scope_kind != AssistantScopeKind::Folder {
            return Err("폴더 대화에서만 파일을 조회할 수 있습니다".into());
        }
        if !matches!(operation, FileAction::Status {}) {
            dispatch(
                app.clone(),
                session_id.clone(),
                operation.clone(),
                cancellation,
            )
            .await?;
        }
        return files.tool_result(session_id, None);
    }
    let binding = match crate::control_server::tool_file_workspace_binding(app) {
        Ok(binding) => binding,
        Err(_) => {
            files.invalidate_external()?;
            return Ok(external_permission());
        }
    };
    files.bind_external(&binding)?;
    let review = needs_review_access(operation);
    if review && !binding.cleanup_allowed {
        return Ok(external_permission());
    }
    if matches!(operation, FileAction::Status {}) {
        let result = files.tool_result(EXTERNAL_WORKSPACE_KEY, None)?;
        if !crate::control_server::file_workspace_binding_current(
            app,
            &binding,
            result.status == AppToolStatus::ReviewRequired,
        ) {
            files.invalidate_external()?;
            return Ok(external_permission());
        }
        return Ok(result);
    }
    if files
        .external_lock()?
        .operation
        .as_ref()
        .is_some_and(|operation| {
            matches!(operation["state"].as_str(), Some("running" | "executing"))
        })
    {
        return files.tool_result(EXTERNAL_WORKSPACE_KEY, Some(AppToolStatus::Running));
    }
    let summary = files
        .view(EXTERNAL_WORKSPACE_KEY)?
        .map(|view| view.summary)
        .unwrap_or_else(|| unknown_summary(&binding.root));
    if heavy_action(operation) {
        let reservation =
            crate::control_server::reserve_file_workspace_operation(app, &binding, review)?;
        let id = reservation.operation.operation_id.clone();
        files.external_lock()?.operation =
            Some(json!({"operationId":id,"state":"running","outcome":"pending","performed":false}));
        let task_app = app.clone();
        let action = operation.clone();
        tauri::async_runtime::spawn(async move {
            let _completion = reservation.completion;
            let result = dispatch_root(
                task_app.clone(),
                FileDispatchRoot {
                    key: EXTERNAL_WORKSPACE_KEY.into(),
                    scope: binding.root.clone(),
                    summary,
                    binding: Some((binding.clone(), review)),
                },
                action,
                cancellation,
                Some(reservation.cancellation.clone()),
            )
            .await;
            let files = task_app.state::<AssistantFilesState>();
            let current =
                crate::control_server::file_workspace_binding_current(&task_app, &binding, review);
            let (state, outcome, message) = if !current {
                let _ = files.invalidate_external();
                (
                    ControlOperationState::Cancelled,
                    "permission_changed",
                    "파일 조회 허용 범위가 바뀌어 결과를 전달하지 않았습니다",
                )
            } else if result.is_ok() {
                (
                    ControlOperationState::Completed,
                    "inspected",
                    "앱의 파일 조회·검토를 완료했습니다",
                )
            } else if reservation.cancellation.load(Ordering::Acquire) {
                let _ = files.forget(EXTERNAL_WORKSPACE_KEY);
                (
                    ControlOperationState::Cancelled,
                    "cancelled",
                    "파일 조회를 취소했습니다",
                )
            } else {
                let _ = files.forget(EXTERNAL_WORKSPACE_KEY);
                (
                    ControlOperationState::Failed,
                    "failed",
                    "파일 조회를 완료하지 못했습니다. 최신 목록과 범위를 확인해 주세요",
                )
            };
            if current {
                if let Ok(mut external) = files.external_lock() {
                    external.operation = Some(
                        json!({"operationId":id,"state":state,"outcome":outcome,"performed":result.is_ok()}),
                    );
                }
                if let Ok(result) = files.tool_result(EXTERNAL_WORKSPACE_KEY, None)
                    && result.status == AppToolStatus::ReviewRequired
                    && crate::control_server::file_workspace_binding_current(
                        &task_app, &binding, true,
                    )
                {
                    let _ = task_app.emit("app-tool-review", result);
                }
            }
            crate::control_server::finish_tool_operation(
                &task_app,
                &id,
                state,
                message.into(),
                None,
                None,
            );
        });
        return files.tool_result(EXTERNAL_WORKSPACE_KEY, Some(AppToolStatus::Running));
    }
    dispatch_root(
        app.clone(),
        FileDispatchRoot {
            key: EXTERNAL_WORKSPACE_KEY.into(),
            scope: binding.root.clone(),
            summary,
            binding: Some((binding.clone(), review)),
        },
        operation.clone(),
        cancellation,
        None,
    )
    .await
    .map_err(|_| {
        "파일 목록을 변경하지 못했습니다. 최신 목록과 허용 범위를 확인해 주세요".to_owned()
    })?;
    if !crate::control_server::file_workspace_binding_current(app, &binding, review) {
        files.invalidate_external()?;
        return Ok(external_permission());
    }
    files.external_lock()?.operation = None;
    files.tool_result(EXTERNAL_WORKSPACE_KEY, None)
}

#[tauri::command]
pub(crate) async fn confirm_external_file_plan(
    app: AppHandle,
    window: WebviewWindow,
    revision: String,
    plan_id: String,
    nested_contents_acknowledged: bool,
) -> Result<trash_actions::TrashOperationResult, String> {
    if window.label() != "main" {
        return Err("기본 앱 화면에서만 최종 확인할 수 있습니다".into());
    }
    let files = app.state::<AssistantFilesState>();
    let binding = files
        .external_lock()?
        .binding
        .clone()
        .ok_or("파일 검토 범위가 변경됐습니다")?;
    if !crate::control_server::file_workspace_binding_current(&app, &binding, true) {
        files.invalidate_external()?;
        return Err("파일 검사 또는 정리 검토 허용이 변경됐습니다. 다시 검토하세요".into());
    }
    let cancellation = app.state::<ScanRuntime>().begin()?;
    let completion = ScanCompletionGuard::new(app.clone());
    let items = crate::control_server::with_file_workspace_authority(&app, &binding, || {
        let items = files.claim(
            EXTERNAL_WORKSPACE_KEY,
            &revision,
            &plan_id,
            nested_contents_acknowledged,
        )?;
        files.external_lock()?.operation =
            Some(json!({"state":"executing","outcome":"pending","performed":false}));
        Ok(items)
    })?;
    let result = trash_actions::trash_verified_cleanup_tree_files(
        app.clone(),
        items,
        cancellation,
        completion.clone(),
    )
    .await;
    files.forget(EXTERNAL_WORKSPACE_KEY)?;
    app.state::<StoredReports>().clear_all()?;
    let outcome = match &result {
        Ok(actual) => trash_outcome(actual),
        Err(_) => {
            json!({"state":"failed","outcome":"unconfirmed","performed":true,"automaticRetry":false})
        }
    };
    if crate::control_server::file_workspace_binding_current(&app, &binding, false) {
        files.external_lock()?.operation = Some(outcome);
    } else {
        files.invalidate_external()?;
    }
    result
}

fn trash_outcome(actual: &trash_actions::TrashOperationResult) -> serde_json::Value {
    json!({"state":"completed","outcome":if actual.cancelled && actual.moved_count == 0 {"cancelled"} else if actual.moved_count == actual.requested_count {"moved"} else {"partial"},
        "requestedCount":actual.requested_count,"movedCount":actual.moved_count,"movedBytes":actual.moved_bytes,
        "cancelled":actual.cancelled,"stoppedEarly":actual.stopped_early,"journalComplete":actual.journal_complete,
        "failedCount":actual.items.iter().filter(|item| item.status == trash_actions::TrashItemStatus::Failed).count(),
        "skippedCount":actual.items.iter().filter(|item| item.status == trash_actions::TrashItemStatus::Skipped).count(),
        "performed":true,"automaticRetry":false})
}

#[tauri::command]
pub(crate) fn cancel_external_file_plan(
    app: AppHandle,
    window: WebviewWindow,
    revision: String,
    plan_id: String,
) -> Result<(), String> {
    if window.label() != "main" {
        return Err("기본 앱 화면에서만 최종 확인할 수 있습니다".into());
    }
    let files = app.state::<AssistantFilesState>();
    cancel_external_plan(&files, revision, plan_id)
}

fn cancel_external_plan(
    files: &AssistantFilesState,
    revision: String,
    plan_id: String,
) -> Result<(), String> {
    if [&revision, &plan_id]
        .iter()
        .any(|id| id.len() != 32 || !id.bytes().all(|byte| byte.is_ascii_hexdigit()))
    {
        return Err("앱이 발급한 검토 계획 번호가 필요합니다".into());
    }
    let mut external = files.external_lock()?;
    let mut workspaces = files.lock()?;
    let current = workspaces
        .get_mut(EXTERNAL_WORKSPACE_KEY)
        .filter(|workspace| {
            workspace.revision == revision
                && workspace
                    .plan
                    .as_ref()
                    .is_some_and(|plan| plan.id == plan_id)
        });
    let Some(workspace) = current else {
        // Stale dismissal is harmless: it never changes a newer plan or restores execution authority.
        // No history or TTL is needed, even after arbitrarily many intervening workspace changes.
        return Ok(());
    };
    if external
        .operation
        .as_ref()
        .is_some_and(|operation| operation["state"] == "executing")
    {
        return Err("이미 휴지통 작업이 시작됐습니다".into());
    }
    workspace.plan = None;
    workspace.selected.clear();
    external.operation =
        Some(json!({"state":"completed","outcome":"cancelled","performed":false,"movedCount":0}));
    Ok(())
}

#[tauri::command]
pub(crate) fn get_assistant_file_workspace(
    state: State<'_, AssistantFilesState>,
    session_id: String,
) -> Result<Option<FileWorkspaceView>, String> {
    state.view(&session_id)
}

#[tauri::command]
pub(crate) fn get_assistant_directory_report(
    state: State<'_, AssistantFilesState>,
    reports: State<'_, StoredReports>,
    session_id: String,
    revision: String,
) -> Result<crate::DirectoryScanResult, String> {
    directory_snapshot(&state, &reports, &session_id, &revision)
}

fn directory_snapshot(
    state: &AssistantFilesState,
    reports: &StoredReports,
    session_id: &str,
    revision: &str,
) -> Result<crate::DirectoryScanResult, String> {
    let (generation, current) = {
        let mut workspaces = state.lock()?;
        let workspace = workspace_mut(&mut workspaces, session_id, revision)?;
        (
            workspace
                .map_generation
                .ok_or("이름 검색만으로는 용량지도를 만들 수 없습니다. 폴더를 검사하세요")?,
            workspace.current.clone(),
        )
    };
    let report = reports.directory_report(generation)?;
    if Path::new(&report.root) != current {
        return Err("폴더 지도 경계가 변경됐습니다. 다시 검사하세요".into());
    }
    Ok(crate::DirectoryScanResult { generation, report })
}

/// Native candidate trees pin a trusted measured snapshot without changing the
/// chat workspace or the shared treemap generation.
pub(crate) fn cleanup_tree_seed(
    state: &AssistantFilesState,
    reports: &StoredReports,
    session_id: &str,
    revision: &str,
) -> Result<bloomsweepy_core::DirectoryScanReport, String> {
    Ok(directory_snapshot(state, reports, session_id, revision)?.report)
}
#[tauri::command]
pub(crate) async fn assistant_file_action(
    app: AppHandle,
    session_id: String,
    operation: FileAction,
) -> Result<FileWorkspaceView, String> {
    let cancelled = Arc::new(AtomicBool::new(false));
    if let FileAction::ReviewNamed { name } = operation {
        review_named(app, session_id, name, cancelled).await
    } else {
        dispatch(app, session_id, operation, cancelled).await
    }
}
#[tauri::command]
pub(crate) fn select_assistant_files(
    state: State<'_, AssistantFilesState>,
    session_id: String,
    revision: String,
    ids: Vec<String>,
) -> Result<FileWorkspaceView, String> {
    state.select(&session_id, &revision, ids)
}
#[tauri::command]
pub(crate) async fn prepare_assistant_file_plan(
    app: AppHandle,
    session_id: String,
    revision: String,
) -> Result<FileWorkspaceView, String> {
    let cancellation = app.state::<ScanRuntime>().begin()?;
    let _completion = ScanCompletionGuard::new(app.clone());
    tauri::async_runtime::spawn_blocking(move || {
        prepare(
            &app.state::<AssistantFilesState>(),
            &session_id,
            &revision,
            &|| cancellation.load(Ordering::Acquire),
        )
    })
    .await
    .map_err(|error| error.to_string())?
}
#[tauri::command]
pub(crate) async fn confirm_assistant_file_plan(
    app: AppHandle,
    window: WebviewWindow,
    session_id: String,
    revision: String,
    plan_id: String,
    nested_contents_acknowledged: bool,
    automatic: Option<bool>,
) -> Result<trash_actions::TrashOperationResult, String> {
    if window.label() != "main" {
        return Err("기본 앱 화면에서만 최종 확인할 수 있습니다".into());
    }
    crate::control_server::require_automatic_trash_access(&app, automatic.unwrap_or(false))?;
    let session =
        assistant_sessions::get_assistant_session(app.clone(), session_id.clone()).await?;
    if session.session.scope_kind != AssistantScopeKind::Folder {
        return Err("폴더 대화가 아닙니다".into());
    }
    let cancellation = app.state::<ScanRuntime>().begin()?;
    let _completion = ScanCompletionGuard::new(app.clone());
    let items = app.state::<AssistantFilesState>().claim(
        &session_id,
        &revision,
        &plan_id,
        nested_contents_acknowledged,
    )?;
    let result =
        trash_actions::trash_verified_assistant_files(app.clone(), items, cancellation).await;
    app.state::<AssistantFilesState>().forget(&session_id)?;
    app.state::<assistant_tools::AssistantToolsState>()
        .forget(&session_id)?;
    app.state::<StoredReports>().clear_all()?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, Workspace) {
        #[cfg(windows)]
        let temp = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
        #[cfg(not(windows))]
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        std::fs::create_dir(root.join("promo-video")).unwrap();
        std::fs::write(root.join("promo-video/clip.mp4"), "synthetic").unwrap();
        std::fs::write(root.join("notes.txt"), "notes").unwrap();
        let workspace =
            scan_workspace(root.clone(), root, &StoredReports::default(), &|| false).unwrap();
        (temp, workspace)
    }

    fn external_binding(root: PathBuf) -> crate::control_server::FileWorkspaceBinding {
        crate::control_server::FileWorkspaceBinding {
            root,
            scope_epoch: 1,
            cleanup_epoch: 1,
            cleanup_allowed: true,
        }
    }

    #[test]
    fn file_dispatch_worker_keeps_its_lease_after_the_request_guard_drops() {
        let runtime = Arc::new(ScanRuntime::default());
        let cancellation = runtime.begin().unwrap();
        let completion = ScanCompletionGuard::for_runtime(runtime.clone(), None);
        let worker_completion = Some(completion.clone());
        let (started_tx, started_rx) = std::sync::mpsc::sync_channel(0);
        let (finish_tx, finish_rx) = std::sync::mpsc::sync_channel(0);
        let worker = tauri::async_runtime::spawn_blocking(move || {
            let _worker_completion = worker_completion;
            started_tx.send(()).unwrap();
            finish_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            assert!(cancellation.load(Ordering::Acquire));
        });
        started_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        drop(completion); // The awaiting IPC request no longer owns the lease.
        assert!(runtime.is_running());
        assert!(runtime.begin().is_err());
        assert!(runtime.cancel().unwrap());
        finish_tx.send(()).unwrap();
        tauri::async_runtime::block_on(worker).unwrap();
        assert!(!runtime.is_running());
    }

    #[test]
    fn shared_root_engine_scans_ranks_browses_and_searches_only_its_fixture() {
        let (_temp, fixture_workspace) = fixture();
        let root = fixture_workspace.scope.clone();
        let state = AssistantFilesState::default();
        let reports = StoredReports::default();
        let scan = apply_workspace_action(
            &state,
            &reports,
            EXTERNAL_WORKSPACE_KEY,
            root.clone(),
            unknown_summary(&root),
            FileAction::Largest {},
            &|| false,
        )
        .unwrap();
        assert!(scan.size_ranked);
        assert_eq!(scan.entries[0].name, "promo-video");
        let browse = apply_workspace_action(
            &state,
            &reports,
            EXTERNAL_WORKSPACE_KEY,
            root.clone(),
            scan.summary.clone(),
            FileAction::Browse {
                revision: scan.revision,
                entry_id: scan.entries[0].id.clone(),
            },
            &|| false,
        )
        .unwrap();
        assert!(browse.can_go_up);
        assert_eq!(browse.entries[0].name, "clip.mp4");
        let parent = apply_workspace_action(
            &state,
            &reports,
            EXTERNAL_WORKSPACE_KEY,
            root.clone(),
            browse.summary,
            FileAction::Parent {
                revision: browse.revision,
            },
            &|| false,
        )
        .unwrap();
        assert_eq!(Path::new(&parent.current_path), root);
        assert!(
            apply_workspace_action(
                &state,
                &reports,
                EXTERNAL_WORKSPACE_KEY,
                root.clone(),
                parent.summary.clone(),
                FileAction::Parent {
                    revision: parent.revision.clone()
                },
                &|| false
            )
            .is_err()
        );
        let search = apply_workspace_action(
            &state,
            &reports,
            EXTERNAL_WORKSPACE_KEY,
            root.clone(),
            parent.summary,
            FileAction::Search {
                query: "clip".into(),
            },
            &|| false,
        )
        .unwrap();
        assert_eq!(search.entries[0].name, "clip.mp4");
        assert_eq!(search.map_generation, None);
        assert!(
            apply_workspace_action(
                &state,
                &reports,
                EXTERNAL_WORKSPACE_KEY,
                root.clone(),
                search.summary.clone(),
                FileAction::Search {
                    query: "../private".into()
                },
                &|| false
            )
            .is_err()
        );
        assert!(
            apply_workspace_action(
                &state,
                &reports,
                EXTERNAL_WORKSPACE_KEY,
                root,
                search.summary,
                FileAction::Scan {},
                &|| true
            )
            .is_err()
        );
    }

    #[test]
    fn external_reserved_workspace_never_reuses_native_selection_or_identity() {
        let (_temp, workspace) = fixture();
        let root = workspace.scope.clone();
        let native_key = assistant_tools::new_id().unwrap();
        let state = AssistantFilesState::default();
        let native = state.insert(&native_key, workspace).unwrap();
        state
            .bind_external(&external_binding(root.clone()))
            .unwrap();
        let reports = StoredReports::default();
        let external = apply_workspace_action(
            &state,
            &reports,
            EXTERNAL_WORKSPACE_KEY,
            root.clone(),
            native.summary.clone(),
            FileAction::Scan {},
            &|| false,
        )
        .unwrap();
        assert_ne!(external.revision, native.revision);
        assert!(
            apply_workspace_action(
                &state,
                &reports,
                EXTERNAL_WORKSPACE_KEY,
                root.clone(),
                native.summary.clone(),
                FileAction::Select {
                    revision: external.revision.clone(),
                    include_ids: vec![native.entries[0].id.clone()],
                    exclude_ids: Vec::new()
                },
                &|| false
            )
            .is_err()
        );
        apply_workspace_action(
            &state,
            &reports,
            EXTERNAL_WORKSPACE_KEY,
            root,
            native.summary,
            FileAction::Select {
                revision: external.revision,
                include_ids: vec![external.entries[0].id.clone()],
                exclude_ids: Vec::new(),
            },
            &|| false,
        )
        .unwrap();
        assert!(
            state
                .view(&native_key)
                .unwrap()
                .unwrap()
                .selected_ids
                .is_empty()
        );
        assert_eq!(
            state
                .view(EXTERNAL_WORKSPACE_KEY)
                .unwrap()
                .unwrap()
                .selected_ids
                .len(),
            1
        );
    }

    #[test]
    fn shared_model_actions_reject_unknown_ids_and_only_select_current_page() {
        let (_temp, workspace) = fixture();
        let root = workspace.scope.clone();
        for index in 0..30 {
            std::fs::write(root.join(format!("page-{index:02}.txt")), "x").unwrap();
        }
        let state = AssistantFilesState::default();
        let reports = StoredReports::default();
        let scan = apply_workspace_action(
            &state,
            &reports,
            EXTERNAL_WORKSPACE_KEY,
            root.clone(),
            workspace.summary,
            FileAction::Scan {},
            &|| false,
        )
        .unwrap();
        let outside = state
            .lock()
            .unwrap()
            .get(EXTERNAL_WORKSPACE_KEY)
            .unwrap()
            .entry(PAGE_SIZE)
            .id;
        let select = FileAction::Select {
            revision: scan.revision.clone(),
            include_ids: vec![outside.clone()],
            exclude_ids: Vec::new(),
        };
        assert!(
            apply_workspace_action(
                &state,
                &reports,
                EXTERNAL_WORKSPACE_KEY,
                root.clone(),
                scan.summary.clone(),
                select.clone(),
                &|| false
            )
            .is_err()
        );
        assert!(
            apply_workspace_action(
                &state,
                &reports,
                EXTERNAL_WORKSPACE_KEY,
                root.clone(),
                scan.summary.clone(),
                FileAction::Review {
                    revision: scan.revision.clone(),
                    ids: vec![outside.clone()]
                },
                &|| false
            )
            .is_err()
        );
        apply_workspace_action(
            &state,
            &reports,
            EXTERNAL_WORKSPACE_KEY,
            root.clone(),
            scan.summary.clone(),
            FileAction::Page {
                revision: scan.revision,
                offset: PAGE_SIZE,
            },
            &|| false,
        )
        .unwrap();
        let selected = apply_workspace_action(
            &state,
            &reports,
            EXTERNAL_WORKSPACE_KEY,
            root,
            scan.summary,
            select,
            &|| false,
        )
        .unwrap();
        assert_eq!(selected.selected_ids, vec![outside]);
        for raw in [
            r#"{"kind":"scan","path":"/"}"#,
            r#"{"kind":"status","nativeSession":"stolen"}"#,
            r#"{"kind":"confirm","planId":"stolen"}"#,
            r#"{"kind":"review_named","name":"x","approved":true}"#,
        ] {
            assert!(serde_json::from_str::<FileAction>(raw).is_err(), "{raw}");
        }
    }

    #[test]
    fn external_review_is_local_only_one_shot_and_exact_cancel_is_idempotent() {
        let (_temp, workspace) = fixture();
        let root = workspace.scope.clone();
        let state = AssistantFilesState::default();
        state
            .bind_external(&external_binding(root.clone()))
            .unwrap();
        let reports = StoredReports::default();
        let reviewed = apply_workspace_action(
            &state,
            &reports,
            EXTERNAL_WORKSPACE_KEY,
            root,
            workspace.summary,
            FileAction::ReviewNamed {
                name: "notes.txt".into(),
            },
            &|| false,
        )
        .unwrap();
        let plan = reviewed.plan.unwrap();
        assert_eq!(plan.expires_at_unix_ms, None);
        let result = state.tool_result(EXTERNAL_WORKSPACE_KEY, None).unwrap();
        assert_eq!(result.status, AppToolStatus::ReviewRequired);
        assert_eq!(
            result.data["workspace"]["revision"],
            result.presentation.as_ref().unwrap()["workspace"]["revision"]
        );
        assert_eq!(result.data["workspace"]["reviewReady"], true);
        let public = crate::app_tools::model_context(&result).unwrap();
        assert!(!public.contains(&plan.id));
        assert!(!public.contains(&plan.entries[0].path));
        assert!(!public.contains("presentation"));
        assert!(!public.contains("opt-in"));
        assert!(cancel_external_plan(&state, reviewed.revision.clone(), "wrong".into()).is_err());
        assert_eq!(
            state
                .claim(EXTERNAL_WORKSPACE_KEY, &reviewed.revision, &plan.id, true)
                .unwrap()
                .len(),
            1
        );
        assert!(
            state
                .claim(EXTERNAL_WORKSPACE_KEY, &reviewed.revision, &plan.id, true)
                .is_err()
        );
        cancel_external_plan(&state, reviewed.revision.clone(), plan.id.clone()).unwrap();
        assert!(cancel_external_plan(&state, reviewed.revision, "other".into()).is_err());
    }

    #[test]
    fn stale_external_dismissal_leaves_newer_plans_unchanged_after_every_invalidation() {
        for invalidation in [
            "scan",
            "largest",
            "search",
            "browse",
            "parent",
            "review_named",
            "select",
            "review",
            "prepare",
            "replace",
            "forget",
            "cleanup_revoke",
            "scope_revoke",
            "invalidate",
            "claim",
        ] {
            let (_temp, workspace) = fixture();
            let root = workspace.scope.clone();
            let summary = workspace.summary.clone();
            let mut binding = external_binding(root.clone());
            let state = AssistantFilesState::default();
            let reports = StoredReports::default();
            state.bind_external(&binding).unwrap();
            let mut current = state.insert(EXTERNAL_WORKSPACE_KEY, workspace).unwrap();
            let run = |operation| {
                apply_workspace_action(
                    &state,
                    &reports,
                    EXTERNAL_WORKSPACE_KEY,
                    root.clone(),
                    summary.clone(),
                    operation,
                    &|| false,
                )
                .unwrap()
            };
            if invalidation == "parent" {
                let folder = current
                    .entries
                    .iter()
                    .find(|entry| entry.is_directory)
                    .unwrap();
                current = run(FileAction::Browse {
                    revision: current.revision.clone(),
                    entry_id: folder.id.clone(),
                });
            }
            let target = current
                .entries
                .iter()
                .find(|entry| !entry.is_directory)
                .unwrap()
                .id
                .clone();
            let reviewed = run(FileAction::Review {
                revision: current.revision.clone(),
                ids: vec![target.clone()],
            });
            let old_revision = reviewed.revision;
            let old_plan_id = reviewed.plan.unwrap().id;
            match invalidation {
                "scan" => {
                    run(FileAction::Scan {});
                }
                "largest" => {
                    run(FileAction::Largest {});
                }
                "search" => {
                    run(FileAction::Search {
                        query: "notes".into(),
                    });
                }
                "browse" => {
                    let folder = current
                        .entries
                        .iter()
                        .find(|entry| entry.is_directory)
                        .unwrap();
                    run(FileAction::Browse {
                        revision: old_revision.clone(),
                        entry_id: folder.id.clone(),
                    });
                }
                "parent" => {
                    run(FileAction::Parent {
                        revision: old_revision.clone(),
                    });
                }
                "review_named" => {
                    run(FileAction::ReviewNamed {
                        name: "notes.txt".into(),
                    });
                }
                "select" => {
                    run(FileAction::Select {
                        revision: old_revision.clone(),
                        include_ids: Vec::new(),
                        exclude_ids: vec![target.clone()],
                    });
                }
                "review" => {
                    run(FileAction::Review {
                        revision: old_revision.clone(),
                        ids: vec![target],
                    });
                }
                "prepare" => {
                    prepare(&state, EXTERNAL_WORKSPACE_KEY, &old_revision, &|| false).unwrap();
                }
                "replace" => {
                    state
                        .insert(
                            EXTERNAL_WORKSPACE_KEY,
                            scan_workspace(root.clone(), root.clone(), &reports, &|| false)
                                .unwrap(),
                        )
                        .unwrap();
                }
                "forget" => {
                    state.forget(EXTERNAL_WORKSPACE_KEY).unwrap();
                }
                "cleanup_revoke" => {
                    binding.cleanup_epoch += 1;
                    binding.cleanup_allowed = false;
                    state.bind_external(&binding).unwrap();
                }
                "scope_revoke" => {
                    binding.scope_epoch += 1;
                    state.bind_external(&binding).unwrap();
                }
                "invalidate" => {
                    state.invalidate_external().unwrap();
                }
                "claim" => {
                    state
                        .claim(EXTERNAL_WORKSPACE_KEY, &old_revision, &old_plan_id, true)
                        .unwrap();
                }
                _ => unreachable!(),
            }
            let before = serde_json::to_value(state.view(EXTERNAL_WORKSPACE_KEY).unwrap()).unwrap();
            let before_operation = state.external_lock().unwrap().operation.clone();
            cancel_external_plan(&state, old_revision.clone(), old_plan_id.clone()).unwrap();
            // Unknown but valid-shaped dismissals are equally harmless; no nonce grants execution.
            cancel_external_plan(&state, "f".repeat(32), "e".repeat(32)).unwrap();
            assert_eq!(
                serde_json::to_value(state.view(EXTERNAL_WORKSPACE_KEY).unwrap()).unwrap(),
                before,
                "{invalidation}"
            );
            assert_eq!(
                state.external_lock().unwrap().operation,
                before_operation,
                "{invalidation}"
            );
            assert!(
                state
                    .claim(EXTERNAL_WORKSPACE_KEY, &old_revision, &old_plan_id, true)
                    .is_err(),
                "{invalidation}"
            );
            assert!(root.join("notes.txt").exists() && root.join("promo-video/clip.mp4").exists());
        }
    }

    #[test]
    fn old_external_dismissal_has_no_history_limit_and_active_cancel_still_clears_only_its_plan() {
        let (_temp, workspace) = fixture();
        let state = AssistantFilesState::default();
        state
            .bind_external(&external_binding(workspace.scope.clone()))
            .unwrap();
        let view = state.insert(EXTERNAL_WORKSPACE_KEY, workspace).unwrap();
        let target = view
            .entries
            .iter()
            .find(|entry| !entry.is_directory)
            .unwrap()
            .id
            .clone();
        state
            .select(EXTERNAL_WORKSPACE_KEY, &view.revision, vec![target])
            .unwrap();
        let first = prepare(&state, EXTERNAL_WORKSPACE_KEY, &view.revision, &|| false)
            .unwrap()
            .plan
            .unwrap();
        for _ in 0..20 {
            prepare(&state, EXTERNAL_WORKSPACE_KEY, &view.revision, &|| false).unwrap();
        }
        let current = state.view(EXTERNAL_WORKSPACE_KEY).unwrap().unwrap();
        let new_id = current.plan.as_ref().unwrap().id.clone();
        assert_ne!(new_id, first.id);
        let before = serde_json::to_value(&current).unwrap();
        state.external_lock().unwrap().operation =
            Some(json!({"state":"completed","outcome":"inspected","performed":false}));
        let before_operation = state.external_lock().unwrap().operation.clone();
        cancel_external_plan(&state, current.revision.clone(), first.id.clone()).unwrap();
        cancel_external_plan(&state, "f".repeat(32), "e".repeat(32)).unwrap();
        assert_eq!(
            serde_json::to_value(state.view(EXTERNAL_WORKSPACE_KEY).unwrap().unwrap()).unwrap(),
            before
        );
        assert_eq!(state.external_lock().unwrap().operation, before_operation);
        assert!(
            state
                .claim(EXTERNAL_WORKSPACE_KEY, &current.revision, &first.id, true)
                .is_err()
        );
        for (revision, plan_id) in [
            ("short".into(), new_id.clone()),
            (current.revision.clone(), "x".repeat(32)),
            ("a".repeat(33), new_id.clone()),
        ] {
            assert!(cancel_external_plan(&state, revision, plan_id).is_err());
        }
        assert_eq!(
            state
                .view(EXTERNAL_WORKSPACE_KEY)
                .unwrap()
                .unwrap()
                .plan
                .unwrap()
                .id,
            new_id
        );
        cancel_external_plan(&state, current.revision.clone(), new_id.clone()).unwrap();
        let cancelled = state.view(EXTERNAL_WORKSPACE_KEY).unwrap().unwrap();
        assert!(cancelled.plan.is_none() && cancelled.selected_ids.is_empty());
        assert_eq!(
            state.external_lock().unwrap().operation.as_ref().unwrap()["outcome"],
            "cancelled"
        );
        assert!(
            state
                .claim(EXTERNAL_WORKSPACE_KEY, &current.revision, &new_id, true)
                .is_err()
        );
        cancel_external_plan(&state, current.revision, new_id).unwrap();
    }

    #[test]
    fn external_scope_aba_and_cleanup_regrant_do_not_restore_previous_plans() {
        let (_temp, workspace) = fixture();
        let mut binding = external_binding(workspace.scope.clone());
        let state = AssistantFilesState::default();
        state.bind_external(&binding).unwrap();
        let view = state.insert(EXTERNAL_WORKSPACE_KEY, workspace).unwrap();
        state
            .select(
                EXTERNAL_WORKSPACE_KEY,
                &view.revision,
                vec![view.entries[0].id.clone()],
            )
            .unwrap();
        let plan = prepare(&state, EXTERNAL_WORKSPACE_KEY, &view.revision, &|| false)
            .unwrap()
            .plan
            .unwrap();
        binding.cleanup_epoch += 1;
        binding.cleanup_allowed = false;
        state.bind_external(&binding).unwrap();
        let current = state.view(EXTERNAL_WORKSPACE_KEY).unwrap().unwrap();
        assert!(current.plan.is_none() && current.selected_ids.is_empty());
        assert!(
            state
                .claim(EXTERNAL_WORKSPACE_KEY, &view.revision, &plan.id, true)
                .is_err()
        );
        cancel_external_plan(&state, view.revision, plan.id).unwrap();
        binding.cleanup_epoch += 1;
        binding.cleanup_allowed = true;
        state.bind_external(&binding).unwrap();
        assert!(
            state
                .view(EXTERNAL_WORKSPACE_KEY)
                .unwrap()
                .unwrap()
                .plan
                .is_none()
        );
        binding.scope_epoch += 2; // A -> B -> A has the same path but a different authority.
        state.bind_external(&binding).unwrap();
        assert!(state.view(EXTERNAL_WORKSPACE_KEY).unwrap().is_none());
        assert_eq!(
            state
                .tool_result(EXTERNAL_WORKSPACE_KEY, None)
                .unwrap()
                .data["workspace"]["freshScan"],
            false
        );
    }

    #[test]
    fn external_status_distinguishes_running_failure_cancel_and_partial_moves_without_paths() {
        let state = AssistantFilesState::default();
        state.external_lock().unwrap().operation =
            Some(json!({"state":"running","operationId":"opaque","outcome":"pending"}));
        let running = state.tool_result(EXTERNAL_WORKSPACE_KEY, None).unwrap();
        assert_eq!(running.status, AppToolStatus::Running);
        assert_eq!(running.data["workspace"]["freshScan"], false);
        state.external_lock().unwrap().operation =
            Some(json!({"state":"failed","outcome":"failed","performed":false}));
        assert_eq!(
            state
                .tool_result(EXTERNAL_WORKSPACE_KEY, None)
                .unwrap()
                .status,
            AppToolStatus::Failed
        );
        let actual = trash_actions::TrashOperationResult {
            operation_id: "local-only-operation".into(),
            requested_count: 2,
            moved_count: 1,
            moved_bytes: 8,
            cancelled: true,
            stopped_early: true,
            journal_complete: false,
            journal_path: "/private/journal".into(),
            items: vec![trash_actions::TrashItemResult {
                path: "/private/target".into(),
                logical_bytes: 8,
                status: trash_actions::TrashItemStatus::Skipped,
                message: Some("private detail".into()),
            }],
        };
        let outcome = trash_outcome(&actual);
        assert_eq!(outcome["outcome"], "partial");
        assert_eq!(outcome["movedCount"], 1);
        assert_eq!(outcome["skippedCount"], 1);
        state.external_lock().unwrap().operation = Some(outcome);
        let completed = state.tool_result(EXTERNAL_WORKSPACE_KEY, None).unwrap();
        assert_eq!(completed.status, AppToolStatus::Completed);
        assert_eq!(completed.data["workspace"]["freshScan"], false);
        assert!(
            !crate::app_tools::model_context(&completed)
                .unwrap()
                .contains("private")
        );
    }
    #[test]
    fn cleanup_tree_seed_rejects_old_revision_missing_workspace_and_replaced_map() {
        let (_temp, fixture_workspace) = fixture();
        let reports = StoredReports::default();
        let files = AssistantFilesState::default();
        let workspace = scan_workspace(
            fixture_workspace.scope.clone(),
            fixture_workspace.current.clone(),
            &reports,
            &|| false,
        )
        .unwrap();
        let view = files.insert("test-tree-source", workspace).unwrap();
        assert!(cleanup_tree_seed(&files, &reports, "test-tree-source", &view.revision).is_ok());
        assert!(cleanup_tree_seed(&files, &reports, "test-tree-source", "old-revision").is_err());
        reports
            .replace_directory(
                scan_directory_level(
                    &fixture_workspace.current,
                    Default::default(),
                    |_| {},
                    || false,
                )
                .unwrap(),
            )
            .unwrap();
        assert!(cleanup_tree_seed(&files, &reports, "test-tree-source", &view.revision).is_err());
        files.forget("test-tree-source").unwrap();
        assert!(cleanup_tree_seed(&files, &reports, "test-tree-source", &view.revision).is_err());
    }
    #[test]
    fn review_nonempty_folder_requires_ack_and_consumes_once() {
        let (_temp, workspace) = fixture();
        let state = AssistantFilesState::default();
        let view = state.insert("test", workspace).unwrap();
        let id = view
            .entries
            .iter()
            .find(|entry| entry.name == "promo-video")
            .unwrap()
            .id
            .clone();
        state.select("test", &view.revision, vec![id]).unwrap();
        let reviewed = prepare(&state, "test", &view.revision, &|| false).unwrap();
        let plan = reviewed.plan.unwrap();
        assert!(plan.requires_nested_ack);
        assert_eq!(plan.logical_bytes, 9);
        assert!(
            state
                .claim("test", &view.revision, &plan.id, false)
                .is_err()
        );
        assert_eq!(
            state
                .claim("test", &view.revision, &plan.id, true)
                .unwrap()
                .len(),
            1
        );
        assert!(state.claim("test", &view.revision, &plan.id, true).is_err());
    }
    #[test]
    fn changed_files_stale_ids_cancel_and_selection_invalidate_plans() {
        let (_temp, workspace) = fixture();
        let root = workspace.scope.clone();
        let state = AssistantFilesState::default();
        let view = state.insert("test", workspace).unwrap();
        let id = view
            .entries
            .iter()
            .find(|entry| entry.name == "notes.txt")
            .unwrap()
            .id
            .clone();
        assert!(state.select("test", "old", vec![id.clone()]).is_err());
        assert!(
            state
                .select("test", &view.revision, vec![id.clone(), id.clone()])
                .is_err()
        );
        state
            .select("test", &view.revision, vec![id.clone()])
            .unwrap();
        assert!(prepare(&state, "test", &view.revision, &|| true).is_err());
        let plan = prepare(&state, "test", &view.revision, &|| false)
            .unwrap()
            .plan
            .unwrap();
        state.select("test", &view.revision, vec![id]).unwrap();
        assert!(state.claim("test", &view.revision, &plan.id, true).is_err());
        std::fs::write(root.join("notes.txt"), "changed content").unwrap();
        assert!(prepare(&state, "test", &view.revision, &|| false).is_err());
        assert!(confined(&root, root.parent().unwrap()).is_err());
    }
    #[test]
    fn prompt_omits_paths_and_protocol_cannot_approve_or_run_shell() {
        let (_temp, workspace) = fixture();
        let root = workspace.scope.display().to_string();
        let state = AssistantFilesState::default();
        state.insert("test", workspace).unwrap();
        let context = state.prompt_context("test").unwrap();
        assert!(!context.contains(&root));
        for raw in [
            r#"{"kind":"confirm","planId":"x"}"#,
            r#"{"kind":"search","query":"x","path":"/"}"#,
            r#"{"kind":"shell","command":"rm -rf"}"#,
        ] {
            assert!(serde_json::from_str::<FileAction>(raw).is_err());
        }
    }
    #[test]
    fn exact_names_are_unique_not_partial_and_plans_are_nonexpiring_one_shot() {
        let (_temp, mut workspace) = fixture();
        let id = unique_exact_id(&workspace, "PROMO-VIDEO").unwrap();
        assert!(unique_exact_id(&workspace, "promo").is_none());
        workspace.truncated = true;
        assert!(unique_exact_id(&workspace, "promo-video").is_none());
        workspace.truncated = false;
        workspace.unreadable = 1;
        assert!(unique_exact_id(&workspace, "promo-video").is_none());
        workspace.unreadable = 0;
        workspace.nodes.push(
            workspace
                .nodes
                .iter()
                .find(|node| node.name == "promo-video")
                .unwrap()
                .clone(),
        );
        assert!(unique_exact_id(&workspace, "promo-video").is_none());
        workspace.nodes.pop();
        let state = AssistantFilesState::default();
        let view = state.insert("test", workspace).unwrap();
        state.select("test", &view.revision, vec![id]).unwrap();
        let plan = prepare(&state, "test", &view.revision, &|| false)
            .unwrap()
            .plan
            .unwrap();
        assert_eq!(plan.expires_at_unix_ms, None);
        assert!(state.claim("test", &view.revision, &plan.id, true).is_ok());
        assert!(state.claim("test", &view.revision, &plan.id, true).is_err());
        assert!(state.view("test").unwrap().unwrap().plan.is_none());
    }
    #[test]
    fn mixed_files_and_folders_and_page_scope() {
        let (_temp, workspace) = fixture();
        let state = AssistantFilesState::default();
        let view = state.insert("test", workspace).unwrap();
        let ids: Vec<_> = view.entries.iter().map(|entry| entry.id.clone()).collect();
        state.select("test", &view.revision, ids.clone()).unwrap();
        let plan = prepare(&state, "test", &view.revision, &|| false)
            .unwrap()
            .plan
            .unwrap();
        assert_eq!(plan.entries.len(), 2);
        assert_eq!(plan.logical_bytes, 14);
        let mut workspaces = state.lock().unwrap();
        let workspace = workspaces.get_mut("test").unwrap();
        workspace.offset = 1;
        assert!(workspace.check_page_ids(&[ids[0].clone()]).is_err());
        assert!(workspace.check_page_ids(&[ids[1].clone()]).is_ok());
    }
    #[test]
    fn discovery_and_map_share_bytes_without_selection_or_plan() {
        let (_temp, seed) = fixture();
        let reports = StoredReports::default();
        let mut workspace =
            scan_workspace(seed.scope.clone(), seed.current, &reports, &|| false).unwrap();
        workspace.size_ranked = true;
        let state = AssistantFilesState::default();
        let view = state.insert("test", workspace).unwrap();
        assert!(view.size_ranked);
        assert!(view.selected_ids.is_empty());
        assert!(view.plan.is_none());
        let map = directory_snapshot(&state, &reports, "test", &view.revision).unwrap();
        assert_eq!(Some(map.generation), view.map_generation);
        assert_eq!(map.report.root, view.current_path);
        assert_eq!(
            map.report.total_logical_bytes,
            view.summary.total_logical_bytes
        );
        assert_eq!(view.entries[0].name, "promo-video");
        for (entry, node) in view.entries.iter().zip(map.report.children.iter()) {
            assert_eq!(entry.path, node.path);
            assert_eq!(entry.logical_bytes, Some(node.logical_bytes));
        }
        let context = state.prompt_context("test").unwrap();
        assert!(context.contains("deletionSafety"));
        assert!(context.contains("scanCompletedAtUnixMs"));
        assert!(!context.contains(&view.current_path));
        assert!(directory_snapshot(&state, &reports, "other-session", &view.revision).is_err());
        assert!(directory_snapshot(&state, &reports, "test", "old-revision").is_err());
        reports.replace_directory(map.report).unwrap();
        assert!(directory_snapshot(&state, &reports, "test", &view.revision).is_err());
    }
    #[test]
    fn name_only_search_cannot_masquerade_as_a_measured_map() {
        let (_temp, mut workspace) = fixture();
        workspace.query = Some("promo".into());
        workspace.map_generation = None;
        let state = AssistantFilesState::default();
        let view = state.insert("test", workspace).unwrap();
        assert!(
            view.entries
                .iter()
                .filter(|entry| entry.is_directory)
                .all(|entry| entry.logical_bytes.is_none())
        );
        assert!(
            directory_snapshot(&state, &StoredReports::default(), "test", &view.revision).is_err()
        );
        assert!(serde_json::from_str::<FileAction>(r#"{"kind":"largest","delete":true}"#).is_err());
        assert!(matches!(
            serde_json::from_str::<FileAction>(r#"{"kind":"largest"}"#).unwrap(),
            FileAction::Largest {}
        ));
    }
}
