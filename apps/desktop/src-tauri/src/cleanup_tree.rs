//! Ephemeral native candidate tree: server-owned snapshots and selection rules,
//! never client paths. Partial parents never enter the trash frontier.
use crate::{
    ScanCompletionGuard, ScanRuntime, StoredReports, assistant_files, assistant_sessions,
    assistant_tools, trash_actions,
};
use bloomsweepy_core::{
    DirectoryNode, DirectoryScanConfig, DirectoryScanReport, VerifiedTrashItem,
    cleanup_tree_scope_identity, scan_directory_level, validate_cleanup_tree_node_identity,
    validate_cleanup_tree_path, validate_cleanup_tree_trash_folder, validate_directory_trash_file,
    validate_local_directory_path,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{Mutex, atomic::Ordering},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager, State, WebviewWindow};

const PAGE_SIZE: usize = 50;
const MAX_BRANCHES: usize = 16;
const MAX_NODES: usize = 2_048;
const MAX_PATH_BYTES: usize = 8 * 1024 * 1024;
const MAX_TARGETS: usize = 100;
const TREE_TTL: Duration = Duration::from_secs(300);
const REVIEW_BUDGET: Duration = Duration::from_secs(60);

#[derive(Default)]
pub(crate) struct CleanupTreeState(Mutex<Option<Tree>>);

#[derive(Clone)]
struct Tree {
    id: String,
    source: TreeSource,
    session_id: Option<String>,
    authority: SourceAuthority,
    scope: PathBuf,
    scope_identity: (u64, u64),
    captured_at_unix_ms: u128,
    expires_at_unix_ms: u64,
    deadline: Instant,
    selection_revision: u64,
    nodes: BTreeMap<String, Node>,
    branches: HashMap<Option<String>, Branch>,
    rules: BTreeMap<String, bool>,
    path_bytes: usize,
    plan: Option<StoredPlan>,
}

#[derive(Clone)]
struct Node {
    parent_id: Option<String>,
    snapshot: DirectoryNode,
    blocked_reason: Option<String>,
    measured: bool,
}

#[derive(Clone)]
struct Branch {
    child_ids: Vec<String>,
    offset: usize,
    truncated: bool,
    unreadable: u64,
    complete: bool,
}

#[derive(Clone)]
struct StoredPlan {
    view: CleanupTreePlan,
    deadline: Instant,
    verified: Vec<VerifiedTrashItem>,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum TreeSource {
    Directory,
    Assistant,
}

#[derive(Clone)]
enum SourceAuthority {
    Directory {
        generation: u64,
    },
    Assistant {
        session_id: String,
        revision: String,
        scope_root: PathBuf,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum SelectionState {
    Unchecked,
    Checked,
    Mixed,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CleanupTreeNode {
    id: String,
    parent_id: Option<String>,
    name: String,
    path: String,
    is_directory: bool,
    logical_bytes: Option<u64>,
    file_count: Option<u64>,
    directory_count: Option<u64>,
    selection_state: SelectionState,
    eligible: bool,
    blocked_reason: Option<String>,
    children_loaded: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CleanupTreeBranch {
    parent_id: Option<String>,
    offset: usize,
    limit: usize,
    total_entries: usize,
    has_more: bool,
    truncated: bool,
    unreadable_entries: u64,
    complete: bool,
    child_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CleanupTreeSelection {
    selected_node_count: usize,
    target_count: usize,
    known_logical_bytes: u64,
    unknown_targets: usize,
    partial: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CleanupTreePlan {
    id: String,
    selection_revision: u64,
    expires_at_unix_ms: u64,
    entries: Vec<CleanupTreeNode>,
    logical_bytes: u64,
    requires_nested_ack: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CleanupTreeView {
    tree_id: String,
    selection_revision: u64,
    source: TreeSource,
    root_name: String,
    root_path: String,
    captured_at_unix_ms: u128,
    expires_at_unix_ms: u64,
    nodes: Vec<CleanupTreeNode>,
    branches: Vec<CleanupTreeBranch>,
    selection: CleanupTreeSelection,
    plan: Option<CleanupTreePlan>,
}

#[derive(Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum CleanupTreeSourceRequest {
    Directory {
        generation: u64,
    },
    Assistant {
        session_id: String,
        revision: String,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OpenCleanupTreeRequest {
    source: CleanupTreeSourceRequest,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LoadCleanupTreeChildrenRequest {
    tree_id: String,
    parent_id: Option<String>,
    offset: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct UpdateCleanupTreeSelectionRequest {
    tree_id: String,
    selection_revision: u64,
    include_ids: Vec<String>,
    exclude_ids: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PrepareCleanupTreePlanRequest {
    tree_id: String,
    selection_revision: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ConfirmCleanupTreePlanRequest {
    tree_id: String,
    selection_revision: u64,
    plan_id: String,
    nested_contents_acknowledged: bool,
}

impl CleanupTreeState {
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Option<Tree>>, String> {
        self.0
            .lock()
            .map_err(|_| "정리 트리 상태를 잠그지 못했습니다".into())
    }
    fn snapshot(&self, id: &str) -> Result<Tree, String> {
        let mut state = self.lock()?;
        Ok(live_tree(&mut state, id)?.clone())
    }
    fn claim(
        &self,
        request: &ConfirmCleanupTreePlanRequest,
    ) -> Result<(Vec<VerifiedTrashItem>, Option<String>), String> {
        let mut state = self.lock()?;
        let tree = live_tree(&mut state, &request.tree_id)?;
        if tree.selection_revision != request.selection_revision {
            return Err("검토 후 선택이 변경됐습니다. 다시 검토하세요".into());
        }
        let plan = tree
            .plan
            .as_ref()
            .ok_or("검토 계획이 없거나 이미 사용했습니다")?;
        if plan.view.id != request.plan_id {
            return Err("현재 검토 계획이 아닙니다. 다시 확인하세요".into());
        }
        if plan.deadline <= Instant::now() {
            tree.plan = None;
            return Err("검토 계획이 변경됐거나 만료됐습니다. 다시 검토하세요".into());
        }
        if plan.view.requires_nested_ack && !request.nested_contents_acknowledged {
            return Err("선택한 폴더의 하위 항목 전체 이동을 확인하세요".into());
        }
        tree.check_scope()?;
        let plan = tree.plan.take().expect("validated plan exists");
        let session_id = tree.session_id.clone();
        // Consume the complete authority before I/O; failed execution cannot be replayed.
        *state = None;
        Ok((plan.verified, session_id))
    }
}

fn live_tree<'a>(state: &'a mut Option<Tree>, id: &str) -> Result<&'a mut Tree, String> {
    if id.len() != 32 || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("정리 트리 번호가 올바르지 않습니다".into());
    }
    if state
        .as_ref()
        .is_some_and(|tree| tree.deadline <= Instant::now())
    {
        *state = None;
    }
    state
        .as_mut()
        .filter(|tree| tree.id == id)
        .ok_or_else(|| "정리 트리가 변경됐거나 만료됐습니다. 다시 여세요".into())
}

impl Tree {
    fn from_report(
        report: DirectoryScanReport,
        source: TreeSource,
        session_id: Option<String>,
        authority: SourceAuthority,
        cancelled: &impl Fn() -> bool,
    ) -> Result<Self, String> {
        let scope = validate_local_directory_path(Path::new(&report.root))
            .map_err(|error| error.to_string())?;
        if scope != Path::new(&report.root) {
            return Err("검사 폴더 경계가 변경됐습니다".into());
        }
        let mut tree = Self {
            id: assistant_tools::new_id()?,
            source,
            session_id,
            authority,
            scope_identity: cleanup_tree_scope_identity(&scope)
                .map_err(|error| error.to_string())?,
            scope,
            captured_at_unix_ms: report.completed_at_unix_ms,
            expires_at_unix_ms: assistant_tools::unix_ms() + TREE_TTL.as_millis() as u64,
            deadline: Instant::now() + TREE_TTL,
            selection_revision: 0,
            nodes: BTreeMap::new(),
            branches: HashMap::new(),
            rules: BTreeMap::new(),
            path_bytes: 0,
            plan: None,
        };
        tree.add_branch(None, report, cancelled)?;
        Ok(tree)
    }
    fn check_scope(&self) -> Result<(), String> {
        if cleanup_tree_scope_identity(&self.scope).map_err(|error| error.to_string())?
            != self.scope_identity
        {
            return Err("정리 트리의 폴더 신원이 변경됐습니다. 다시 검사하세요".into());
        }
        Ok(())
    }
    fn check_source_snapshot(&self, app: &AppHandle) -> Result<(), String> {
        self.check_source_reports(
            &app.state::<StoredReports>(),
            &app.state::<assistant_files::AssistantFilesState>(),
        )
    }
    fn check_source_reports(
        &self,
        reports: &StoredReports,
        files: &assistant_files::AssistantFilesState,
    ) -> Result<(), String> {
        let report = match &self.authority {
            SourceAuthority::Directory { generation } => reports.directory_report(*generation)?,
            SourceAuthority::Assistant {
                session_id,
                revision,
                ..
            } => assistant_files::cleanup_tree_seed(files, reports, session_id, revision)?,
        };
        if Path::new(&report.root) != self.scope
            || report.completed_at_unix_ms != self.captured_at_unix_ms
        {
            return Err("정리 트리의 원본 검사 결과가 변경됐습니다. 다시 여세요".into());
        }
        self.check_scope()
    }
    async fn check_source(&self, app: &AppHandle) -> Result<(), String> {
        if let SourceAuthority::Assistant {
            session_id,
            scope_root,
            ..
        } = &self.authority
        {
            let session =
                assistant_sessions::get_assistant_session(app.clone(), session_id.clone()).await?;
            if session.session.scope_kind != crate::assistant_provider::AssistantScopeKind::Folder
                || Path::new(&session.session.scope_root) != scope_root
                || !self.scope.starts_with(scope_root)
            {
                return Err("정리 트리의 대화 폴더가 변경됐습니다".into());
            }
        }
        self.check_source_snapshot(app)
    }
    fn add_branch(
        &mut self,
        parent_id: Option<String>,
        report: DirectoryScanReport,
        cancelled: &impl Fn() -> bool,
    ) -> Result<(), String> {
        if self.branches.contains_key(&parent_id) {
            return Err("이미 검사한 트리 가지입니다".into());
        }
        if self.branches.len() >= MAX_BRANCHES
            || self.nodes.len() + report.children.len() > MAX_NODES
        {
            return Err(
                "정리 트리 메모리 상한에 도달했습니다. 더 작은 폴더에서 다시 여세요".into(),
            );
        }
        let expected = parent_id
            .as_ref()
            .map(|id| {
                self.nodes
                    .get(id)
                    .map(|node| Path::new(&node.snapshot.path))
            })
            .unwrap_or(Some(self.scope.as_path()))
            .ok_or("현재 트리에 없는 부모입니다")?;
        if Path::new(&report.root) != expected {
            return Err("트리 검사 범위가 일치하지 않습니다".into());
        }
        let added_bytes = report.children.iter().try_fold(0_usize, |total, node| {
            total
                .checked_add(node.path.len() + node.name.len())
                .ok_or("트리 경로 상한을 초과했습니다")
        })?;
        if self.path_bytes.saturating_add(added_bytes) > MAX_PATH_BYTES {
            return Err(
                "정리 트리 경로 메모리 상한에 도달했습니다. 더 작은 폴더를 선택하세요".into(),
            );
        }
        let complete = !report.children_truncated
            && !report.tracking_limit_reached
            && report.unreadable_entries == 0;
        let folder_sizes_known = report.unreadable_entries == 0;
        let mut staged = Vec::with_capacity(report.children.len());
        for snapshot in report.children {
            if cancelled() {
                return Err("정리 트리 검사가 취소됐습니다".into());
            }
            let path = Path::new(&snapshot.path);
            if path.parent() != Some(expected) || !path.starts_with(&self.scope) {
                return Err("검사 결과에 범위 밖 항목이 포함됐습니다".into());
            }
            let blocked_reason = validate_cleanup_tree_path(&self.scope, path)
                .and_then(|()| validate_cleanup_tree_node_identity(&snapshot))
                .err()
                .map(|error| error.to_string());
            staged.push((
                assistant_tools::new_id()?,
                Node {
                    parent_id: parent_id.clone(),
                    measured: !snapshot.is_directory || folder_sizes_known,
                    snapshot,
                    blocked_reason,
                },
            ));
        }
        let child_ids = staged.iter().map(|(id, _)| id.clone()).collect();
        self.nodes.extend(staged);
        self.path_bytes += added_bytes;
        self.branches.insert(
            parent_id,
            Branch {
                child_ids,
                offset: 0,
                truncated: report.children_truncated || report.tracking_limit_reached,
                unreadable: report.unreadable_entries,
                complete,
            },
        );
        Ok(())
    }
    fn included(&self, id: &str) -> bool {
        let mut current = Some(id);
        for _ in 0..=MAX_BRANCHES {
            let Some(id) = current else {
                return false;
            };
            if let Some(value) = self.rules.get(id) {
                return *value;
            }
            current = self
                .nodes
                .get(id)
                .and_then(|node| node.parent_id.as_deref());
        }
        false
    }
    fn selection_states(&self) -> BTreeMap<String, SelectionState> {
        let mut states: BTreeMap<_, _> = self
            .nodes
            .iter()
            .map(|(id, node)| {
                (
                    id.clone(),
                    if node.blocked_reason.is_none() && self.included(id) {
                        SelectionState::Checked
                    } else {
                        SelectionState::Unchecked
                    },
                )
            })
            .collect();
        let mut by_depth: Vec<_> = self
            .nodes
            .keys()
            .map(|id| {
                let mut depth = 0;
                let mut current = self.nodes[id].parent_id.as_deref();
                while let Some(parent) = current {
                    depth += 1;
                    if depth > MAX_BRANCHES {
                        break;
                    }
                    current = self.nodes[parent].parent_id.as_deref();
                }
                (depth, id)
            })
            .collect();
        by_depth.sort_unstable_by_key(|entry| std::cmp::Reverse(entry.0));
        for (_, id) in by_depth {
            let Some(parent) = &self.nodes[id].parent_id else {
                continue;
            };
            if self.nodes[parent].blocked_reason.is_some() {
                continue;
            }
            let child_state = states[id];
            let parent_state = states[parent];
            if child_state == SelectionState::Mixed
                || (parent_state == SelectionState::Checked
                    && child_state == SelectionState::Unchecked)
                || (parent_state == SelectionState::Unchecked
                    && child_state == SelectionState::Checked)
            {
                states.insert(parent.clone(), SelectionState::Mixed);
            }
        }
        states
    }
    #[cfg(test)]
    fn selection_state(&self, id: &str) -> SelectionState {
        self.selection_states()
            .get(id)
            .copied()
            .unwrap_or(SelectionState::Unchecked)
    }
    fn node_view(&self, id: &str, selection_state: SelectionState) -> CleanupTreeNode {
        let node = &self.nodes[id];
        CleanupTreeNode {
            id: id.into(),
            parent_id: node.parent_id.clone(),
            name: node.snapshot.name.clone(),
            path: node.snapshot.path.clone(),
            is_directory: node.snapshot.is_directory,
            logical_bytes: node.measured.then_some(node.snapshot.logical_bytes),
            file_count: node.measured.then_some(node.snapshot.file_count),
            directory_count: node.measured.then_some(node.snapshot.directory_count),
            selection_state,
            eligible: node.blocked_reason.is_none(),
            blocked_reason: node.blocked_reason.clone(),
            children_loaded: self.branches.contains_key(&Some(id.into())),
        }
    }
    fn frontier(&self, strict: bool) -> Result<(Vec<String>, usize), String> {
        self.frontier_with_states(strict, &self.selection_states())
    }
    fn frontier_with_states(
        &self,
        strict: bool,
        states: &BTreeMap<String, SelectionState>,
    ) -> Result<(Vec<String>, usize), String> {
        let mut result = Vec::new();
        let mut unknown = 0;
        let mut pending = self
            .branches
            .get(&None)
            .ok_or("트리 루트가 없습니다")?
            .child_ids
            .clone();
        while let Some(id) = pending.pop() {
            match states[&id] {
                SelectionState::Unchecked => {}
                SelectionState::Checked => result.push(id),
                SelectionState::Mixed => {
                    let branch = self.branches.get(&Some(id));
                    if branch.is_none_or(|branch| !branch.complete) {
                        if strict {
                            return Err("부분 선택한 폴더의 하위 목록이 완전하지 않습니다. 폴더를 펼쳐 검사하거나 더 작은 항목을 선택하세요".into());
                        }
                        unknown += 1;
                    } else {
                        pending.extend(branch.expect("complete branch").child_ids.iter().cloned());
                    }
                }
            }
        }
        result.sort();
        if strict && result.len() > MAX_TARGETS {
            return Err(
                "한 번에 검토할 실제 항목은 최대 100개입니다. 더 작은 범위를 선택하세요".into(),
            );
        }
        Ok((result, unknown))
    }
    fn view(&self) -> CleanupTreeView {
        // Compute selection once, then reuse it across rows, frontier and totals.
        let states = self.selection_states();
        let (frontier, unresolved) = self
            .frontier_with_states(false, &states)
            .unwrap_or_default();
        let unknown = unresolved
            + frontier
                .iter()
                .filter(|id| !self.nodes[*id].measured)
                .count();
        let mut branches: Vec<_> = self
            .branches
            .iter()
            .map(|(parent_id, branch)| CleanupTreeBranch {
                parent_id: parent_id.clone(),
                offset: branch.offset,
                limit: PAGE_SIZE,
                total_entries: branch.child_ids.len(),
                has_more: branch.offset + PAGE_SIZE < branch.child_ids.len(),
                truncated: branch.truncated,
                unreadable_entries: branch.unreadable,
                complete: branch.complete,
                child_ids: branch
                    .child_ids
                    .iter()
                    .skip(branch.offset)
                    .take(PAGE_SIZE)
                    .cloned()
                    .collect(),
            })
            .collect();
        branches.sort_by(|left, right| left.parent_id.cmp(&right.parent_id));
        CleanupTreeView {
            tree_id: self.id.clone(),
            selection_revision: self.selection_revision,
            source: self.source,
            root_name: self
                .scope
                .file_name()
                .unwrap_or(self.scope.as_os_str())
                .to_string_lossy()
                .into_owned(),
            root_path: self.scope.to_string_lossy().into_owned(),
            captured_at_unix_ms: self.captured_at_unix_ms,
            expires_at_unix_ms: self.expires_at_unix_ms,
            nodes: self
                .nodes
                .keys()
                .map(|id| self.node_view(id, states[id]))
                .collect(),
            branches,
            selection: CleanupTreeSelection {
                selected_node_count: states
                    .values()
                    .filter(|state| **state == SelectionState::Checked)
                    .count(),
                target_count: frontier.len(),
                known_logical_bytes: frontier.iter().fold(0_u64, |total, id| {
                    total.saturating_add(if self.nodes[id].measured {
                        self.nodes[id].snapshot.logical_bytes
                    } else {
                        0
                    })
                }),
                unknown_targets: unknown,
                partial: unknown > 0
                    || states.values().any(|state| *state == SelectionState::Mixed),
            },
            plan: self
                .plan
                .as_ref()
                .filter(|plan| plan.deadline > Instant::now())
                .map(|plan| plan.view.clone()),
        }
    }
    fn select(&mut self, request: &UpdateCleanupTreeSelectionRequest) -> Result<(), String> {
        if self.selection_revision != request.selection_revision {
            return Err("선택 상태가 변경됐습니다. 현재 트리를 다시 확인하세요".into());
        }
        if request.include_ids.len() + request.exclude_ids.len() > MAX_NODES {
            return Err("선택 규칙 상한을 초과했습니다".into());
        }
        let mut seen = HashSet::new();
        for id in request.include_ids.iter().chain(&request.exclude_ids) {
            if id.len() != 32 || !seen.insert(id) || !self.nodes.contains_key(id) {
                return Err("현재 트리 항목을 중복 없이 선택하세요".into());
            }
        }
        for id in &request.include_ids {
            if self.nodes[id].blocked_reason.is_some() {
                return Err("보호 항목은 선택할 수 없습니다".into());
            }
        }
        for (ids, included) in [(&request.include_ids, true), (&request.exclude_ids, false)] {
            let roots: HashSet<_> = ids.iter().map(String::as_str).collect();
            let descendants: Vec<_> = self
                .rules
                .keys()
                .filter(|id| {
                    let mut parent = self.nodes[*id].parent_id.as_deref();
                    for _ in 0..=MAX_BRANCHES {
                        let Some(id) = parent else {
                            return false;
                        };
                        if roots.contains(id) {
                            return true;
                        }
                        parent = self.nodes[id].parent_id.as_deref();
                    }
                    false
                })
                .cloned()
                .collect();
            for child in descendants {
                self.rules.remove(&child);
            }
            for id in ids {
                self.rules.insert(id.clone(), included);
            }
        }
        self.selection_revision = self
            .selection_revision
            .checked_add(1)
            .ok_or("선택 버전 상한에 도달했습니다")?;
        self.plan = None;
        Ok(())
    }
}

#[tauri::command]
pub(crate) async fn open_cleanup_tree(
    app: AppHandle,
    runtime: State<'_, ScanRuntime>,
    reports: State<'_, StoredReports>,
    _state: State<'_, CleanupTreeState>,
    request: OpenCleanupTreeRequest,
) -> Result<CleanupTreeView, String> {
    let (report, source, session_id, authority) = match request.source {
        CleanupTreeSourceRequest::Directory { generation } => (
            reports.directory_report(generation)?,
            TreeSource::Directory,
            None,
            SourceAuthority::Directory { generation },
        ),
        CleanupTreeSourceRequest::Assistant {
            session_id,
            revision,
        } => {
            let session =
                assistant_sessions::get_assistant_session(app.clone(), session_id.clone()).await?;
            if session.session.scope_kind != crate::assistant_provider::AssistantScopeKind::Folder {
                return Err("폴더 대화에서만 정리 트리를 열 수 있습니다".into());
            }
            let report = assistant_files::cleanup_tree_seed(
                &app.state::<assistant_files::AssistantFilesState>(),
                &reports,
                &session_id,
                &revision,
            )?;
            if !Path::new(&report.root).starts_with(&session.session.scope_root) {
                return Err("대화 폴더 밖의 정리 트리입니다".into());
            }
            let authority = SourceAuthority::Assistant {
                session_id: session_id.clone(),
                revision,
                scope_root: PathBuf::from(session.session.scope_root),
            };
            (report, TreeSource::Assistant, Some(session_id), authority)
        }
    };
    let cancellation = runtime.begin()?;
    let completion = ScanCompletionGuard::new(app.clone());
    let worker_completion = completion.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _worker_completion = worker_completion;
        let tree = Tree::from_report(report, source, session_id, authority, &|| {
            cancellation.load(Ordering::Acquire)
        })?;
        if cancellation.load(Ordering::Acquire) {
            return Err("정리 트리 검사가 취소됐습니다".into());
        }
        tree.check_source_snapshot(&app)?;
        let view = tree.view();
        *app.state::<CleanupTreeState>().lock()? = Some(tree);
        Ok(view)
    })
    .await
    .map_err(|error| format!("정리 트리를 열지 못했습니다: {error}"))?
}

#[tauri::command]
pub(crate) async fn get_cleanup_tree(
    app: AppHandle,
    state: State<'_, CleanupTreeState>,
    tree_id: String,
) -> Result<CleanupTreeView, String> {
    let snapshot = state.snapshot(&tree_id)?;
    snapshot.check_source(&app).await?;
    Ok(snapshot.view())
}

#[tauri::command]
pub(crate) async fn load_cleanup_tree_children(
    app: AppHandle,
    runtime: State<'_, ScanRuntime>,
    state: State<'_, CleanupTreeState>,
    request: LoadCleanupTreeChildrenRequest,
) -> Result<CleanupTreeView, String> {
    let snapshot = state.snapshot(&request.tree_id)?;
    snapshot.check_source(&app).await?;
    if let Some(branch) = snapshot.branches.get(&request.parent_id) {
        if request.offset != 0
            && (request.offset >= branch.child_ids.len()
                || !request.offset.is_multiple_of(PAGE_SIZE))
        {
            return Err("트리 페이지 범위가 올바르지 않습니다".into());
        }
        let mut stored = state.lock()?;
        let tree = live_tree(&mut stored, &request.tree_id)?;
        tree.check_source_snapshot(&app)?;
        tree.branches
            .get_mut(&request.parent_id)
            .ok_or("트리 가지가 변경됐습니다")?
            .offset = request.offset;
        return Ok(tree.view());
    }
    if request.offset != 0 {
        return Err("먼저 폴더의 첫 페이지를 검사하세요".into());
    }
    let parent_id = request.parent_id.ok_or("트리 루트가 만료됐습니다")?;
    let parent = snapshot
        .nodes
        .get(&parent_id)
        .ok_or("현재 트리에 없는 폴더입니다")?
        .clone();
    if !parent.snapshot.is_directory || parent.blocked_reason.is_some() {
        return Err("보호되지 않은 일반 폴더만 펼칠 수 있습니다".into());
    }
    if snapshot.branches.len() >= MAX_BRANCHES {
        return Err("트리 가지 상한에 도달했습니다. 더 작은 폴더에서 다시 여세요".into());
    }
    let cancellation = runtime.begin()?;
    let completion = ScanCompletionGuard::new(app.clone());
    let worker_completion = completion.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _worker_completion = worker_completion;
        let started = Instant::now();
        let cancelled =
            || cancellation.load(Ordering::Acquire) || started.elapsed() > REVIEW_BUDGET;
        snapshot.check_source_snapshot(&app)?;
        validate_cleanup_tree_node_identity(&parent.snapshot).map_err(|error| error.to_string())?;
        let report = scan_directory_level(
            &parent.snapshot.path,
            DirectoryScanConfig {
                max_children: 512,
                max_tracked_children: 2_048,
                max_empty_directories: 0,
                max_issues: 20,
            },
            |_| {},
            cancelled,
        )
        .map_err(|error| error.to_string())?;
        validate_cleanup_tree_node_identity(&parent.snapshot).map_err(|error| error.to_string())?;
        if cancelled() {
            return Err("정리 트리 검사가 취소됐거나 시간 상한에 도달했습니다".into());
        }
        let state = app.state::<CleanupTreeState>();
        let mut stored = state.lock()?;
        let tree = live_tree(&mut stored, &snapshot.id)?;
        tree.check_source_snapshot(&app)?;
        tree.add_branch(Some(parent_id), report, &cancelled)?;
        Ok(tree.view())
    })
    .await
    .map_err(|error| format!("하위 폴더 검사를 완료하지 못했습니다: {error}"))?
}

#[tauri::command]
pub(crate) async fn update_cleanup_tree_selection(
    app: AppHandle,
    state: State<'_, CleanupTreeState>,
    request: UpdateCleanupTreeSelectionRequest,
) -> Result<CleanupTreeView, String> {
    let snapshot = state.snapshot(&request.tree_id)?;
    snapshot.check_source(&app).await?;
    let mut stored = state.lock()?;
    let tree = live_tree(&mut stored, &request.tree_id)?;
    tree.check_source_snapshot(&app)?;
    tree.select(&request)?;
    Ok(tree.view())
}

fn prepare(tree: &Tree, cancelled: &impl Fn() -> bool) -> Result<StoredPlan, String> {
    tree.check_scope()?;
    let (ids, _) = tree.frontier(true)?;
    if ids.is_empty() {
        return Err("휴지통으로 이동할 항목을 먼저 선택하세요".into());
    }
    let started = Instant::now();
    let cancelled = || cancelled() || started.elapsed() > REVIEW_BUDGET;
    let mut verified = Vec::with_capacity(ids.len());
    let mut entries = Vec::with_capacity(ids.len());
    let mut parent_report: Option<DirectoryScanReport> = None;
    let mut ordered = ids;
    ordered.sort_by(|left, right| {
        tree.nodes[left]
            .snapshot
            .path
            .cmp(&tree.nodes[right].snapshot.path)
    });
    for id in ordered {
        if cancelled() {
            return Err("정리 검토가 취소됐거나 시간 상한에 도달했습니다".into());
        }
        let old = &tree.nodes[&id].snapshot;
        let path = Path::new(&old.path);
        validate_cleanup_tree_path(&tree.scope, path).map_err(|error| error.to_string())?;
        let parent = path.parent().ok_or("항목의 부모 폴더가 없습니다")?;
        if parent_report
            .as_ref()
            .is_none_or(|report| Path::new(&report.root) != parent)
        {
            parent_report = Some(
                scan_directory_level(
                    parent,
                    DirectoryScanConfig {
                        max_children: 2_048,
                        max_tracked_children: 2_048,
                        max_empty_directories: 0,
                        max_issues: 20,
                    },
                    |_| {},
                    cancelled,
                )
                .map_err(|error| error.to_string())?,
            );
        }
        let report = parent_report.as_ref().expect("parent report loaded");
        let live = report
            .children
            .iter()
            .find(|node| node.path == old.path)
            .ok_or("대상이 없어졌거나 현재 검사 상한 밖입니다. 다시 검사하세요")?;
        if !old.same_entry_as(live)
            || old.logical_bytes != live.logical_bytes
            || old.file_count != live.file_count
            || old.directory_count != live.directory_count
        {
            return Err("검사 후 대상이나 용량이 변경됐습니다. 다시 검사하고 선택하세요".into());
        }
        let item = if live.is_directory {
            validate_cleanup_tree_trash_folder(report, &live.path, cancelled)
        } else {
            validate_directory_trash_file(report, &live.path, cancelled)
        }
        .map_err(|error| error.to_string())?;
        let mut entry = tree.node_view(&id, SelectionState::Checked);
        entry.logical_bytes = Some(item.logical_bytes());
        entry.file_count = Some(live.file_count);
        entry.directory_count = Some(live.directory_count);
        entries.push(entry);
        verified.push(item);
    }
    tree.check_scope()?;
    if cancelled() {
        return Err("정리 검토가 취소됐거나 시간 상한에 도달했습니다".into());
    }
    Ok(StoredPlan {
        view: CleanupTreePlan {
            id: assistant_tools::new_id()?,
            selection_revision: tree.selection_revision,
            expires_at_unix_ms: tree.expires_at_unix_ms,
            logical_bytes: verified.iter().fold(0_u64, |total, item| {
                total.saturating_add(item.logical_bytes())
            }),
            requires_nested_ack: entries.iter().any(|entry| entry.is_directory),
            entries,
        },
        deadline: tree.deadline,
        verified,
    })
}

#[tauri::command]
pub(crate) async fn prepare_cleanup_tree_plan(
    app: AppHandle,
    runtime: State<'_, ScanRuntime>,
    state: State<'_, CleanupTreeState>,
    request: PrepareCleanupTreePlanRequest,
) -> Result<CleanupTreeView, String> {
    let snapshot = {
        let mut stored = state.lock()?;
        let tree = live_tree(&mut stored, &request.tree_id)?;
        if tree.selection_revision != request.selection_revision {
            return Err("선택 상태가 변경됐습니다. 다시 확인하세요".into());
        }
        tree.plan = None;
        tree.clone()
    };
    snapshot.check_source(&app).await?;
    let cancellation = runtime.begin()?;
    let completion = ScanCompletionGuard::new(app.clone());
    let worker_completion = completion.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _worker_completion = worker_completion;
        let plan = prepare(&snapshot, &|| cancellation.load(Ordering::Acquire))?;
        let state = app.state::<CleanupTreeState>();
        let mut stored = state.lock()?;
        let tree = live_tree(&mut stored, &snapshot.id)?;
        if tree.selection_revision != snapshot.selection_revision {
            return Err("검토 중 선택이 변경됐습니다".into());
        }
        if cancellation.load(Ordering::Acquire) {
            return Err("정리 검토가 취소됐습니다".into());
        }
        tree.check_source_snapshot(&app)?;
        tree.plan = Some(plan);
        Ok(tree.view())
    })
    .await
    .map_err(|error| format!("정리 검토를 완료하지 못했습니다: {error}"))?
}

#[tauri::command]
pub(crate) async fn confirm_cleanup_tree_plan(
    app: AppHandle,
    window: WebviewWindow,
    runtime: State<'_, ScanRuntime>,
    state: State<'_, CleanupTreeState>,
    request: ConfirmCleanupTreePlanRequest,
) -> Result<trash_actions::TrashOperationResult, String> {
    if window.label() != "main" {
        return Err("기본 앱 화면에서만 최종 확인할 수 있습니다".into());
    }
    let snapshot = state.snapshot(&request.tree_id)?;
    snapshot.check_source(&app).await?;
    let cancellation = runtime.begin()?;
    let completion = ScanCompletionGuard::new(app.clone());
    snapshot.check_source_snapshot(&app)?;
    let (items, session_id) = state.claim(&request)?;
    let result = trash_actions::trash_verified_cleanup_tree_files(
        app.clone(),
        items,
        cancellation,
        completion.clone(),
    )
    .await;
    if let Some(session_id) = session_id {
        app.state::<assistant_files::AssistantFilesState>()
            .forget(&session_id)?;
        app.state::<assistant_tools::AssistantToolsState>()
            .forget(&session_id)?;
    }
    app.state::<StoredReports>().clear_all()?;
    result
}

#[tauri::command]
pub(crate) fn dismiss_cleanup_tree_plan(
    state: State<'_, CleanupTreeState>,
    tree_id: String,
    plan_id: String,
) -> Result<CleanupTreeView, String> {
    let mut stored = state.lock()?;
    let tree = live_tree(&mut stored, &tree_id)?;
    if tree
        .plan
        .as_ref()
        .is_some_and(|plan| plan.view.id == plan_id)
    {
        tree.plan = None;
    }
    Ok(tree.view())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (tempfile::TempDir, Tree) {
        #[cfg(windows)]
        let temp = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
        #[cfg(not(windows))]
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(temp.path().join("A/B")).unwrap();
        std::fs::create_dir_all(temp.path().join("A/C")).unwrap();
        std::fs::write(temp.path().join("A/B/keep.txt"), b"keep").unwrap();
        std::fs::write(temp.path().join("A/B/remove.txt"), b"remove").unwrap();
        std::fs::write(temp.path().join("A/C/data.txt"), b"data").unwrap();
        std::fs::write(temp.path().join("top.txt"), b"top").unwrap();
        let report =
            scan_directory_level(temp.path(), Default::default(), |_| {}, || false).unwrap();
        let tree = Tree::from_report(
            report,
            TreeSource::Directory,
            None,
            SourceAuthority::Directory { generation: 1 },
            &|| false,
        )
        .unwrap();
        (temp, tree)
    }
    fn id(tree: &Tree, name: &str) -> String {
        tree.nodes
            .iter()
            .find(|(_, node)| node.snapshot.name == name)
            .unwrap()
            .0
            .clone()
    }
    fn expand(tree: &mut Tree, parent: &str) {
        let report = scan_directory_level(
            &tree.nodes[parent].snapshot.path,
            Default::default(),
            |_| {},
            || false,
        )
        .unwrap();
        tree.add_branch(Some(parent.into()), report, &|| false)
            .unwrap();
    }
    fn select(
        tree: &mut Tree,
        include_ids: Vec<String>,
        exclude_ids: Vec<String>,
    ) -> Result<(), String> {
        tree.select(&UpdateCleanupTreeSelectionRequest {
            tree_id: tree.id.clone(),
            selection_revision: tree.selection_revision,
            include_ids,
            exclude_ids,
        })
    }
    #[test]
    fn nested_exclusion_never_targets_partial_ancestors_or_keeper() {
        let (_temp, mut tree) = fixture();
        assert_eq!(tree.view().selection.target_count, 0);
        let a = id(&tree, "A");
        select(&mut tree, vec![a.clone()], vec![]).unwrap();
        expand(&mut tree, &a);
        let b = id(&tree, "B");
        assert_eq!(tree.selection_state(&b), SelectionState::Checked);
        expand(&mut tree, &b);
        let keep = id(&tree, "keep.txt");
        select(&mut tree, vec![], vec![keep.clone()]).unwrap();
        assert_eq!(tree.selection_state(&a), SelectionState::Mixed);
        assert_eq!(tree.selection_state(&b), SelectionState::Mixed);
        let (frontier, unknown) = tree.frontier(true).unwrap();
        assert_eq!(unknown, 0);
        assert!(!frontier.contains(&a) && !frontier.contains(&b) && !frontier.contains(&keep));
        assert!(frontier.contains(&id(&tree, "remove.txt")) && frontier.contains(&id(&tree, "C")));
        let plan = prepare(&tree, &|| false).unwrap();
        assert_eq!(plan.view.logical_bytes, 10);
        assert_eq!(plan.verified.len(), 2);
        assert!(
            plan.verified
                .iter()
                .all(|item| !item.path().ends_with("keep.txt")
                    && !item.path().ends_with("A")
                    && !item.path().ends_with("B"))
        );
        // Review does not execute or modify any fixture item.
        assert!(tree.scope.join("A/B/keep.txt").exists());
        assert!(tree.scope.join("A/B/remove.txt").exists());
    }
    #[test]
    fn parent_and_child_selection_are_deduplicated_and_page_changes_preserve_rules() {
        let (_temp, mut tree) = fixture();
        let a = id(&tree, "A");
        expand(&mut tree, &a);
        let b = id(&tree, "B");
        select(&mut tree, vec![a.clone(), b], vec![]).unwrap();
        assert_eq!(tree.frontier(true).unwrap().0, vec![a.clone()]);
        let plan = prepare(&tree, &|| false).unwrap();
        assert_eq!(plan.view.logical_bytes, 14);
        assert_eq!(plan.view.entries.len(), 1);
        assert_eq!(plan.view.entries[0].logical_bytes, Some(14));
        tree.plan = Some(plan);
        select(&mut tree, vec![], vec![]).unwrap();
        assert!(tree.plan.is_none());
        assert_eq!(tree.frontier(true).unwrap().0, vec![a]);
    }
    #[test]
    fn incomplete_partial_branch_and_unloaded_protected_descendants_fail_closed() {
        let (temp, mut tree) = fixture();
        let a = id(&tree, "A");
        expand(&mut tree, &a);
        let b = id(&tree, "B");
        select(&mut tree, vec![a.clone()], vec![b]).unwrap();
        tree.branches.get_mut(&Some(a.clone())).unwrap().complete = false;
        assert!(tree.frontier(true).is_err());
        assert_eq!(tree.view().selection.unknown_targets, 1);
        std::fs::create_dir_all(temp.path().join("A/Unsafe.app/Contents")).unwrap();
        let report =
            scan_directory_level(temp.path(), Default::default(), |_| {}, || false).unwrap();
        let mut fresh = Tree::from_report(
            report,
            TreeSource::Directory,
            None,
            SourceAuthority::Directory { generation: 2 },
            &|| false,
        )
        .unwrap();
        let a = id(&fresh, "A");
        select(&mut fresh, vec![a], vec![]).unwrap();
        assert!(prepare(&fresh, &|| false).is_err());
        assert!(temp.path().join("A/Unsafe.app/Contents").exists());
    }
    #[test]
    fn stale_ids_revisions_cross_scope_and_retention_limits_are_rejected_without_mutation() {
        let (_temp, mut tree) = fixture();
        let a = id(&tree, "A");
        let stale = UpdateCleanupTreeSelectionRequest {
            tree_id: tree.id.clone(),
            selection_revision: 99,
            include_ids: vec![a.clone()],
            exclude_ids: vec![],
        };
        assert!(tree.select(&stale).is_err());
        assert!(select(&mut tree, vec!["0".repeat(32)], vec![]).is_err());
        assert!(select(&mut tree, vec![a.clone(), a.clone()], vec![]).is_err());
        assert_eq!(tree.selection_revision, 0);
        let mut wrong =
            scan_directory_level(&tree.scope, Default::default(), |_| {}, || false).unwrap();
        wrong.root = tree.scope.join("elsewhere").to_string_lossy().into_owned();
        assert!(tree.add_branch(Some(a.clone()), wrong, &|| false).is_err());
        tree.path_bytes = MAX_PATH_BYTES;
        let report = scan_directory_level(
            &tree.nodes[&a].snapshot.path,
            Default::default(),
            |_| {},
            || false,
        )
        .unwrap();
        assert!(tree.add_branch(Some(a), report, &|| false).is_err());
        assert_eq!(tree.branches.len(), 1);
        assert_eq!(tree.nodes.len(), 2);
    }
    #[test]
    fn plan_expiry_acknowledgment_and_one_shot_claim_are_enforced() {
        let (_temp, mut tree) = fixture();
        let a = id(&tree, "A");
        select(&mut tree, vec![a], vec![]).unwrap();
        let plan = prepare(&tree, &|| false).unwrap();
        let mut request = ConfirmCleanupTreePlanRequest {
            tree_id: tree.id.clone(),
            selection_revision: tree.selection_revision,
            plan_id: plan.view.id.clone(),
            nested_contents_acknowledged: false,
        };
        tree.plan = Some(plan);
        let state = CleanupTreeState(Mutex::new(Some(tree.clone())));
        assert!(state.claim(&request).is_err());
        request.nested_contents_acknowledged = true;
        request.plan_id = "foreign".into();
        assert!(state.claim(&request).is_err());
        request.plan_id = tree.plan.as_ref().unwrap().view.id.clone();
        assert_eq!(state.claim(&request).unwrap().0.len(), 1);
        assert!(state.claim(&request).is_err());
        tree.deadline = Instant::now();
        let expired = CleanupTreeState(Mutex::new(Some(tree)));
        assert!(expired.snapshot(&request.tree_id).is_err());
        assert!(expired.lock().unwrap().is_none());
    }
    #[test]
    fn changed_nodes_and_scope_replacement_cancel_review_without_trashing() {
        let (temp, mut tree) = fixture();
        let top = id(&tree, "top.txt");
        select(&mut tree, vec![top.clone()], vec![]).unwrap();
        std::fs::write(temp.path().join("top.txt"), b"changed").unwrap();
        assert!(prepare(&tree, &|| false).is_err());
        assert!(temp.path().join("top.txt").exists());
        assert!(prepare(&tree, &|| true).is_err());
        #[cfg(unix)]
        {
            let moved = temp.path().with_extension("original-tree");
            std::fs::rename(temp.path(), &moved).unwrap();
            std::fs::create_dir(temp.path()).unwrap();
            assert!(tree.check_scope().is_err());
            std::fs::remove_dir(temp.path()).unwrap();
            std::fs::rename(moved, temp.path()).unwrap();
        }
    }
    #[test]
    fn protocol_cannot_supply_paths_shell_or_confirmation_side_channels() {
        assert!(
            serde_json::from_str::<OpenCleanupTreeRequest>(
                r#"{"source":{"kind":"directory","generation":1,"path":"/"}}"#
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<LoadCleanupTreeChildrenRequest>(
                r#"{"treeId":"x","parentId":null,"offset":0,"path":"/"}"#
            )
            .is_err()
        );
        assert!(serde_json::from_str::<ConfirmCleanupTreePlanRequest>(r#"{"treeId":"x","selectionRevision":0,"planId":"x","nestedContentsAcknowledged":true,"command":"rm"}"#).is_err());
    }
    #[test]
    fn unreadable_folder_sizes_remain_unknown_not_zero() {
        let (_temp, original) = fixture();
        let mut report =
            scan_directory_level(&original.scope, Default::default(), |_| {}, || false).unwrap();
        report.unreadable_entries = 1;
        let mut tree = Tree::from_report(
            report,
            TreeSource::Directory,
            None,
            SourceAuthority::Directory { generation: 1 },
            &|| false,
        )
        .unwrap();
        let a = id(&tree, "A");
        select(&mut tree, vec![a.clone()], vec![]).unwrap();
        let view = tree.view();
        assert_eq!(
            view.nodes
                .iter()
                .find(|node| node.id == a)
                .unwrap()
                .logical_bytes,
            None
        );
        assert_eq!(view.selection.known_logical_bytes, 0);
        assert_eq!(view.selection.unknown_targets, 1);
        assert!(view.selection.partial);
    }
    #[test]
    fn dense_view_records_bounded_cost_and_preserves_off_page_selection() {
        let (_temp, mut tree) = fixture();
        let template = tree.nodes[&id(&tree, "top.txt")].clone();
        tree.nodes.clear();
        tree.rules.clear();
        let mut child_ids = Vec::with_capacity(MAX_NODES);
        // Synthetic metadata only: no filesystem files or deletion authority are used.
        for index in 0..MAX_NODES {
            let id = format!("{index:032x}");
            let mut node = template.clone();
            node.snapshot.name = format!("synthetic-{index}.txt");
            node.snapshot.path = tree
                .scope
                .join(&node.snapshot.name)
                .to_string_lossy()
                .into_owned();
            tree.nodes.insert(id.clone(), node);
            if index % 2 == 0 {
                tree.rules.insert(id.clone(), true);
            }
            child_ids.push(id);
        }
        tree.branches.get_mut(&None).unwrap().child_ids = child_ids;
        let started = Instant::now();
        let view = tree.view();
        let elapsed = started.elapsed();
        let payload_bytes = serde_json::to_vec(&view).unwrap().len();
        println!(
            "cleanup-tree dense view: nodes={}, page={}, elapsed={elapsed:?}, payloadBytes={payload_bytes}",
            view.nodes.len(),
            view.branches[0].child_ids.len()
        );
        assert_eq!(view.nodes.len(), MAX_NODES);
        assert_eq!(view.branches[0].child_ids.len(), PAGE_SIZE);
        assert_eq!(view.selection.target_count, MAX_NODES / 2);
        assert_eq!(view.selection.selected_node_count, MAX_NODES / 2);
        assert!(tree.frontier(true).is_err());
    }
    #[test]
    fn replaced_source_generation_or_cleared_source_cannot_authorize_tree() {
        let (_temp, fixture_tree) = fixture();
        let reports = StoredReports::default();
        let files = assistant_files::AssistantFilesState::default();
        let source = reports
            .replace_directory(
                scan_directory_level(&fixture_tree.scope, Default::default(), |_| {}, || false)
                    .unwrap(),
            )
            .unwrap();
        let tree = Tree::from_report(
            source.report,
            TreeSource::Directory,
            None,
            SourceAuthority::Directory {
                generation: source.generation,
            },
            &|| false,
        )
        .unwrap();
        tree.check_source_reports(&reports, &files).unwrap();
        reports
            .replace_directory(
                scan_directory_level(&tree.scope, Default::default(), |_| {}, || false).unwrap(),
            )
            .unwrap();
        assert!(tree.check_source_reports(&reports, &files).is_err());
        reports.clear_all().unwrap();
        assert!(tree.check_source_reports(&reports, &files).is_err());
    }
    #[test]
    fn changed_scan_objects_are_blocked_at_open_before_any_selection() {
        let (_temp, fixture_tree) = fixture();
        let report =
            scan_directory_level(&fixture_tree.scope, Default::default(), |_| {}, || false)
                .unwrap();
        std::fs::write(fixture_tree.scope.join("top.txt"), b"replaced contents").unwrap();
        let tree = Tree::from_report(
            report,
            TreeSource::Directory,
            None,
            SourceAuthority::Directory { generation: 1 },
            &|| false,
        )
        .unwrap();
        let top = id(&tree, "top.txt");
        assert!(tree.nodes[&top].blocked_reason.is_some());
        assert_eq!(tree.view().selection.selected_node_count, 0);
        assert!(
            !tree
                .view()
                .nodes
                .iter()
                .find(|node| node.id == top)
                .unwrap()
                .eligible
        );
    }
}
