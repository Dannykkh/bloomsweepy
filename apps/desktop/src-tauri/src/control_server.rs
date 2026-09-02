use bloomsweepy_control::{
    CleanupCandidatesRequest, CleanupPlanReference, CleanupSource, ControlCommand,
    ControlInstanceLock, ControlOperationSource, ControlOperationState, ControlOperationStatus,
    ControlRequest, ControlResponse, CreateCleanupPlanRequest, DocumentSearchRequest,
    EndpointDescriptor, FileSearchRequest, MAX_REQUEST_BYTES, MAX_RESPONSE_BYTES,
    OPERATION_ID_BYTES, OperationReference, PROTOCOL_VERSION, StorageScanSummary,
    acquire_instance_lock, default_descriptor_path, default_lock_path, random_operation_id,
    random_token, read_json_frame, remove_descriptor_if_owned, write_descriptor, write_json_frame,
};
use bloomsweepy_core::{
    DocumentSearchError, DocumentSearchRequest as CoreDocumentSearchRequest, FileCatalogError,
    FileCatalogSearchRequest as CoreFileSearchRequest, ScanConfig, ScanProgress,
    document_index_status, file_catalog_status, search_document_index_with_cancellation,
    search_file_catalog_with_cancellation,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};

const CONTROL_STATUS_EVENT: &str = "control-status-changed";
const MAX_STATUS_ERROR_CHARS: usize = 1_000;
const MAX_CONNECTIONS: usize = 8;
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);
const SEARCH_TIMEOUT: Duration = Duration::from_secs(8);
const ACCEPT_RETRY_DELAY: Duration = Duration::from_millis(25);
const RESPONSE_ENVELOPE_RESERVE_BYTES: usize = 64 * 1024;
const MAX_RESULT_VALUE_BYTES: usize = MAX_RESPONSE_BYTES - RESPONSE_ENVELOPE_RESERVE_BYTES;
const MAX_COMPLETED_OPERATIONS: usize = 16;
const MAX_START_REQUESTS: usize = MAX_COMPLETED_OPERATIONS;
const CLEANUP_PLAN_TTL: Duration = Duration::from_secs(5 * 60);
const MAX_COMPLETED_CLEANUP_PLANS: usize = 16;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ControlStatus {
    revision: u64,
    bridge_available: bool,
    connected_clients: u32,
    last_connected_at_unix_ms: Option<u64>,
    active_operation: Option<ControlOperationStatus>,
    last_operation: Option<ControlOperationStatus>,
    pending_review: Option<PendingReviewStatus>,
    last_error: Option<String>,
    protocol_version: u32,
    search_access: ControlSearchAccess,
    scan_access: ControlScanAccess,
    cleanup_access: ControlCleanupAccess,
}

impl Default for ControlStatus {
    fn default() -> Self {
        Self {
            revision: 0,
            bridge_available: false,
            connected_clients: 0,
            last_connected_at_unix_ms: None,
            active_operation: None,
            last_operation: None,
            pending_review: None,
            last_error: None,
            protocol_version: u32::from(PROTOCOL_VERSION),
            search_access: ControlSearchAccess::default(),
            scan_access: ControlScanAccess::default(),
            cleanup_access: ControlCleanupAccess::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ControlSearchAccess {
    files: bool,
    documents: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ControlSearchAccessRequest {
    file_root: Option<String>,
    document_root: Option<String>,
}

#[derive(Default)]
struct ControlSearchScopes {
    file_root: Option<PathBuf>,
    document_root: Option<PathBuf>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ControlScanAccess {
    enabled: bool,
    root: Option<String>,
    approved_at_unix_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ControlScanAccessRequest {
    root: Option<String>,
    config: Option<ScanConfig>,
}

#[derive(Clone, Copy, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ControlCleanupAccess {
    enabled: bool,
    approved_at_unix_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ControlCleanupAccessRequest {
    enabled: bool,
}

#[derive(Clone)]
struct ControlScanPlan {
    root: String,
    canonical_root: PathBuf,
    config: ScanConfig,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PendingReviewStatus {
    id: String,
    item_count: u64,
    total_bytes: u64,
    expires_at_unix_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
enum CleanupPlanState {
    AwaitingApproval,
    Executing,
    Completed,
    Rejected,
    Expired,
    Stale,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CleanupPlanResultSummary {
    moved_count: u64,
    moved_bytes: u64,
    skipped_count: u64,
    failed_count: u64,
    cancelled: bool,
    journal_complete: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CleanupPlanStatusResponse {
    plan_id: String,
    state: CleanupPlanState,
    source: CleanupSource,
    source_generation: u64,
    item_count: u64,
    total_bytes: u64,
    review_count: u64,
    created_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    result: Option<CleanupPlanResultSummary>,
    message: Option<String>,
}

#[derive(Clone)]
struct CleanupReviewPlan {
    status: CleanupPlanStatusResponse,
    candidate_ids: Vec<String>,
}

#[derive(Default)]
struct CleanupReviewState {
    pending: Option<CleanupReviewPlan>,
    completed: VecDeque<CleanupPlanStatusResponse>,
}

#[derive(Clone)]
pub(crate) struct CleanupPlanExecution {
    pub(crate) plan_id: String,
    pub(crate) source: CleanupSource,
    pub(crate) source_generation: u64,
    pub(crate) candidate_ids: Vec<String>,
    pub(crate) expires_at_unix_ms: u64,
}

pub(crate) struct ControlStatusStore {
    status: Mutex<ControlStatus>,
    search_scopes: Mutex<ControlSearchScopes>,
    search_active: AtomicBool,
    scan_plan: Mutex<Option<ControlScanPlan>>,
    completed_operations: Mutex<VecDeque<ControlOperationStatus>>,
    start_requests: Mutex<VecDeque<(String, String)>>,
    cleanup_review: Mutex<CleanupReviewState>,
}

impl Default for ControlStatusStore {
    fn default() -> Self {
        Self {
            status: Mutex::new(ControlStatus::default()),
            search_scopes: Mutex::new(ControlSearchScopes::default()),
            search_active: AtomicBool::new(false),
            scan_plan: Mutex::new(None),
            completed_operations: Mutex::new(VecDeque::new()),
            start_requests: Mutex::new(VecDeque::new()),
            cleanup_review: Mutex::new(CleanupReviewState::default()),
        }
    }
}

impl ControlStatusStore {
    fn snapshot(&self) -> Result<ControlStatus, String> {
        self.status
            .lock()
            .map(|status| status.clone())
            .map_err(|_| "채팅 CLI 연결 상태를 잠글 수 없습니다".to_owned())
    }

    fn update(
        &self,
        app: &AppHandle,
        update: impl FnOnce(&mut ControlStatus),
    ) -> Result<ControlStatus, String> {
        let snapshot = {
            let mut status = self
                .status
                .lock()
                .map_err(|_| "채팅 CLI 연결 상태를 잠글 수 없습니다".to_owned())?;
            update(&mut status);
            status.revision = status.revision.saturating_add(1);
            status.clone()
        };
        let _ = app.emit(CONTROL_STATUS_EVENT, snapshot.clone());
        Ok(snapshot)
    }

    fn set_available(&self, app: &AppHandle) {
        let _ = self.update(app, |status| {
            status.bridge_available = true;
            status.last_error = None;
        });
    }

    fn set_unavailable_error(&self, app: &AppHandle, error: String) {
        let error = bounded_error(error);
        let _ = self.update(app, |status| {
            status.bridge_available = false;
            status.connected_clients = 0;
            status.last_error = Some(error);
        });
    }

    fn connected(&self, app: &AppHandle) {
        let connected_at = unix_time_ms();
        let _ = self.update(app, |status| {
            status.connected_clients = status.connected_clients.saturating_add(1);
            status.last_connected_at_unix_ms = Some(connected_at);
        });
    }

    fn disconnected(&self, app: &AppHandle) {
        let _ = self.update(app, |status| {
            status.connected_clients = status.connected_clients.saturating_sub(1);
        });
    }

    fn request_succeeded(&self, app: &AppHandle) {
        let _ = self.update(app, |status| {
            status.last_error = None;
        });
    }

    fn request_failed(&self, app: &AppHandle, error: String) {
        let error = bounded_error(error);
        let _ = self.update(app, |status| {
            status.last_error = Some(error);
        });
    }

    fn configure_search_access(
        &self,
        app: &AppHandle,
        request: ControlSearchAccessRequest,
    ) -> Result<ControlStatus, String> {
        let file_root = request
            .file_root
            .as_deref()
            .map(canonical_directory)
            .transpose()?;
        let document_root = request
            .document_root
            .as_deref()
            .map(canonical_directory)
            .transpose()?;
        let access = ControlSearchAccess {
            files: file_root.is_some(),
            documents: document_root.is_some(),
        };
        {
            let mut scopes = self
                .search_scopes
                .lock()
                .map_err(|_| "채팅 검색 허용 범위를 잠글 수 없습니다".to_owned())?;
            scopes.file_root = file_root;
            scopes.document_root = document_root;
        }
        self.update(app, |status| {
            status.search_access = access;
            status.last_error = None;
        })?;
        self.snapshot()
    }

    fn configure_scan_access(
        &self,
        app: &AppHandle,
        request: ControlScanAccessRequest,
    ) -> Result<ControlStatus, String> {
        let (plan, access) = match (request.root, request.config) {
            (None, None) => (None, ControlScanAccess::default()),
            (Some(root), Some(config)) => {
                validate_scan_config(&config)?;
                let canonical_root = canonical_directory(&root)?;
                let display_root = root.clone();
                (
                    Some(ControlScanPlan {
                        root,
                        canonical_root,
                        config,
                    }),
                    ControlScanAccess {
                        enabled: true,
                        root: Some(display_root),
                        approved_at_unix_ms: Some(unix_time_ms()),
                    },
                )
            }
            _ => {
                return Err("검사 허용을 켤 때는 폴더와 현재 설정이 모두 필요합니다".to_owned());
            }
        };
        *self
            .scan_plan
            .lock()
            .map_err(|_| "채팅 검사 허용 범위를 잠글 수 없습니다".to_owned())? = plan;
        self.update(app, |status| {
            status.scan_access = access;
            status.last_error = None;
        })
    }

    fn configure_cleanup_access(
        &self,
        app: &AppHandle,
        request: ControlCleanupAccessRequest,
    ) -> Result<ControlStatus, String> {
        if !request.enabled {
            let mut review = self
                .cleanup_review
                .lock()
                .map_err(|_| "정리 계획 상태를 잠글 수 없습니다".to_owned())?;
            if review
                .pending
                .as_ref()
                .is_some_and(|plan| plan.status.state == CleanupPlanState::Executing)
            {
                return Err(
                    "진행 중인 휴지통 작업이 끝난 뒤 정리 검토 허용을 끌 수 있습니다".to_owned(),
                );
            }
            if let Some(mut plan) = review.pending.take() {
                plan.status.state = CleanupPlanState::Rejected;
                plan.status.message = Some("앱에서 외부 정리 검토 허용을 껐습니다".to_owned());
                remember_cleanup_plan(&mut review.completed, plan.status);
            }
        }
        self.update(app, |status| {
            status.cleanup_access = if request.enabled {
                ControlCleanupAccess {
                    enabled: true,
                    approved_at_unix_ms: Some(unix_time_ms()),
                }
            } else {
                ControlCleanupAccess::default()
            };
            if !request.enabled {
                status.pending_review = None;
            }
            status.last_error = None;
        })
    }

    fn ensure_cleanup_access(&self) -> Result<(), String> {
        let enabled = self
            .status
            .lock()
            .map_err(|_| "채팅 CLI 연결 상태를 잠글 수 없습니다".to_owned())?
            .cleanup_access
            .enabled;
        if enabled {
            Ok(())
        } else {
            Err(
                "앱의 대화 > 연결과 권한에서 이번 실행의 정리 계획 검토를 먼저 허용해 주세요"
                    .to_owned(),
            )
        }
    }

    fn create_cleanup_plan(
        &self,
        app: &AppHandle,
        request: &CreateCleanupPlanRequest,
        selection: &super::CleanupPlanSelection,
    ) -> Result<CleanupPlanStatusResponse, String> {
        self.ensure_cleanup_access()?;
        let now = unix_time_ms();
        let mut candidate_ids = request.candidate_ids.clone();
        candidate_ids.sort_unstable_by_key(|value| value.to_ascii_lowercase());
        let mut review = self
            .cleanup_review
            .lock()
            .map_err(|_| "정리 계획 상태를 잠글 수 없습니다".to_owned())?;

        expire_pending_cleanup_plan(&mut review, now);
        if let Some(existing) = review.pending.as_ref() {
            if existing.status.source == request.source
                && existing.status.source_generation == request.source_generation
                && existing.candidate_ids == candidate_ids
            {
                return Ok(existing.status.clone());
            }
            return Err("다른 정리 계획이 앱의 최종 확인을 기다리고 있습니다".to_owned());
        }

        let plan_id =
            random_operation_id().map_err(|_| "정리 계획 번호를 만들지 못했습니다".to_owned())?;
        let expires_at_unix_ms =
            now.saturating_add(u64::try_from(CLEANUP_PLAN_TTL.as_millis()).unwrap_or(u64::MAX));
        let status = CleanupPlanStatusResponse {
            plan_id: plan_id.clone(),
            state: CleanupPlanState::AwaitingApproval,
            source: request.source,
            source_generation: request.source_generation,
            item_count: selection.item_count,
            total_bytes: selection.total_bytes,
            review_count: selection.review_count,
            created_at_unix_ms: now,
            expires_at_unix_ms,
            result: None,
            message: Some("BroomSweepy 앱에서 최종 확인을 기다리고 있습니다".to_owned()),
        };
        review.pending = Some(CleanupReviewPlan {
            status: status.clone(),
            candidate_ids,
        });
        drop(review);

        self.update(app, |control| {
            control.pending_review = Some(PendingReviewStatus {
                id: plan_id,
                item_count: status.item_count,
                total_bytes: status.total_bytes,
                expires_at_unix_ms,
            });
        })?;
        Ok(status)
    }

    fn cleanup_plan_status(
        &self,
        app: &AppHandle,
        plan_id: &str,
    ) -> Result<CleanupPlanStatusResponse, String> {
        let now = unix_time_ms();
        let (status, expired) = {
            let mut review = self
                .cleanup_review
                .lock()
                .map_err(|_| "정리 계획 상태를 잠글 수 없습니다".to_owned())?;
            let expired = expire_pending_cleanup_plan(&mut review, now);
            let status = review
                .pending
                .as_ref()
                .filter(|plan| plan.status.plan_id.eq_ignore_ascii_case(plan_id))
                .map(|plan| plan.status.clone())
                .or_else(|| {
                    review
                        .completed
                        .iter()
                        .find(|plan| plan.plan_id.eq_ignore_ascii_case(plan_id))
                        .cloned()
                })
                .ok_or_else(|| "정리 계획을 찾을 수 없습니다".to_owned())?;
            (status, expired)
        };
        if expired {
            let _ = self.update(app, |control| control.pending_review = None);
        }
        Ok(status)
    }

    pub(crate) fn pending_cleanup_plan(
        &self,
        app: &AppHandle,
    ) -> Result<Option<CleanupPlanExecution>, String> {
        let now = unix_time_ms();
        let (plan, expired) = {
            let mut review = self
                .cleanup_review
                .lock()
                .map_err(|_| "정리 계획 상태를 잠글 수 없습니다".to_owned())?;
            let expired = expire_pending_cleanup_plan(&mut review, now);
            let plan = review.pending.as_ref().and_then(|plan| {
                (plan.status.state == CleanupPlanState::AwaitingApproval).then(|| {
                    CleanupPlanExecution {
                        plan_id: plan.status.plan_id.clone(),
                        source: plan.status.source,
                        source_generation: plan.status.source_generation,
                        candidate_ids: plan.candidate_ids.clone(),
                        expires_at_unix_ms: plan.status.expires_at_unix_ms,
                    }
                })
            });
            (plan, expired)
        };
        if expired {
            let _ = self.update(app, |control| control.pending_review = None);
        }
        Ok(plan)
    }

    pub(crate) fn begin_cleanup_plan(
        &self,
        app: &AppHandle,
        plan_id: &str,
    ) -> Result<CleanupPlanExecution, String> {
        let now = unix_time_ms();
        let execution = {
            let mut review = self
                .cleanup_review
                .lock()
                .map_err(|_| "정리 계획 상태를 잠글 수 없습니다".to_owned())?;
            expire_pending_cleanup_plan(&mut review, now);
            let plan = review
                .pending
                .as_mut()
                .filter(|plan| plan.status.plan_id.eq_ignore_ascii_case(plan_id))
                .ok_or_else(|| "정리 계획을 찾을 수 없거나 확인 시간이 지났습니다".to_owned())?;
            if plan.status.state != CleanupPlanState::AwaitingApproval {
                return Err("이미 처리 중이거나 끝난 정리 계획입니다".to_owned());
            }
            plan.status.state = CleanupPlanState::Executing;
            plan.status.message = Some("앱이 선택 항목을 다시 확인하고 있습니다".to_owned());
            CleanupPlanExecution {
                plan_id: plan.status.plan_id.clone(),
                source: plan.status.source,
                source_generation: plan.status.source_generation,
                candidate_ids: plan.candidate_ids.clone(),
                expires_at_unix_ms: plan.status.expires_at_unix_ms,
            }
        };
        self.update(app, |control| control.pending_review = None)?;
        Ok(execution)
    }

    pub(crate) fn reject_cleanup_plan(
        &self,
        app: &AppHandle,
        plan_id: &str,
    ) -> Result<bool, String> {
        let mut review = self
            .cleanup_review
            .lock()
            .map_err(|_| "정리 계획 상태를 잠글 수 없습니다".to_owned())?;
        expire_pending_cleanup_plan(&mut review, unix_time_ms());
        let Some(mut plan) = review.pending.take() else {
            return Ok(false);
        };
        if !plan.status.plan_id.eq_ignore_ascii_case(plan_id) {
            review.pending = Some(plan);
            return Err("다른 정리 계획 번호입니다".to_owned());
        }
        if plan.status.state != CleanupPlanState::AwaitingApproval {
            review.pending = Some(plan);
            return Err("처리 중인 정리 계획은 거부할 수 없습니다".to_owned());
        }
        plan.status.state = CleanupPlanState::Rejected;
        plan.status.message = Some("사용자가 앱에서 정리 계획을 거부했습니다".to_owned());
        remember_cleanup_plan(&mut review.completed, plan.status);
        drop(review);
        self.update(app, |control| control.pending_review = None)?;
        Ok(true)
    }

    fn finish_cleanup_plan(
        &self,
        app: &AppHandle,
        plan_id: &str,
        state: CleanupPlanState,
        result: Option<CleanupPlanResultSummary>,
        message: String,
    ) -> Result<(), String> {
        let mut review = self
            .cleanup_review
            .lock()
            .map_err(|_| "정리 계획 상태를 잠글 수 없습니다".to_owned())?;
        let Some(mut plan) = review.pending.take() else {
            return Err("마무리할 정리 계획이 없습니다".to_owned());
        };
        if !plan.status.plan_id.eq_ignore_ascii_case(plan_id) {
            review.pending = Some(plan);
            return Err("마무리할 정리 계획 번호가 다릅니다".to_owned());
        }
        plan.status.state = state;
        plan.status.result = result;
        plan.status.message = Some(message);
        remember_cleanup_plan(&mut review.completed, plan.status);
        drop(review);
        self.update(app, |control| control.pending_review = None)?;
        Ok(())
    }

    fn scan_plan(&self) -> Result<ControlScanPlan, String> {
        let plan = self
            .scan_plan
            .lock()
            .map_err(|_| "채팅 검사 허용 범위를 잠글 수 없습니다".to_owned())?
            .clone()
            .ok_or_else(|| "대시보드에서 이 실행의 채팅 검사를 먼저 허용해 주세요".to_owned())?;
        let current_root = canonical_directory(&plan.root)
            .map_err(|_| "허용한 폴더를 다시 확인할 수 없습니다".to_owned())?;
        if current_root != plan.canonical_root {
            return Err("허용한 폴더가 바뀌었습니다. 대시보드에서 다시 허용해 주세요".to_owned());
        }
        Ok(plan)
    }

    fn start_operation(
        &self,
        app: &AppHandle,
        operation: ControlOperationStatus,
    ) -> Result<ControlOperationStatus, String> {
        let snapshot = {
            let mut status = self
                .status
                .lock()
                .map_err(|_| "채팅 CLI 연결 상태를 잠글 수 없습니다".to_owned())?;
            if status.active_operation.is_some() {
                return Err("다른 채팅 작업이 진행 중입니다".to_owned());
            }
            status.active_operation = Some(operation.clone());
            status.revision = status.revision.saturating_add(1);
            status.clone()
        };
        let _ = app.emit(CONTROL_STATUS_EVENT, snapshot);
        Ok(operation)
    }

    fn update_operation_progress(
        &self,
        app: &AppHandle,
        operation_id: &str,
        progress: &ScanProgress,
    ) -> Result<ControlStatus, String> {
        let snapshot = {
            let mut status = self
                .status
                .lock()
                .map_err(|_| "채팅 CLI 연결 상태를 잠글 수 없습니다".to_owned())?;
            let operation = status
                .active_operation
                .as_mut()
                .filter(|operation| operation.operation_id == operation_id)
                .ok_or_else(|| "현재 작업 번호가 바뀌었습니다".to_owned())?;
            operation.state = ControlOperationState::Running;
            operation.message = Some(progress.message.clone());
            operation.processed_items = Some(progress.processed_files);
            operation.processed_bytes = Some(progress.processed_bytes);
            status.revision = status.revision.saturating_add(1);
            status.clone()
        };
        let _ = app.emit(CONTROL_STATUS_EVENT, snapshot.clone());
        Ok(snapshot)
    }

    fn request_operation_cancellation(
        &self,
        app: &AppHandle,
        operation_id: &str,
    ) -> Result<ControlOperationStatus, String> {
        let (snapshot, operation) = {
            let mut status = self
                .status
                .lock()
                .map_err(|_| "채팅 CLI 연결 상태를 잠글 수 없습니다".to_owned())?;
            let operation = status
                .active_operation
                .as_mut()
                .filter(|operation| operation.operation_id == operation_id)
                .ok_or_else(|| "진행 중인 작업 번호를 찾을 수 없습니다".to_owned())?;
            operation.cancellation_requested = true;
            operation.message = Some("취소 요청을 확인하고 있습니다".to_owned());
            let operation = operation.clone();
            status.revision = status.revision.saturating_add(1);
            (status.clone(), operation)
        };
        let _ = app.emit(CONTROL_STATUS_EVENT, snapshot);
        Ok(operation)
    }

    fn finish_operation(
        &self,
        app: &AppHandle,
        operation_id: &str,
        state: ControlOperationState,
        message: String,
        scan_generation: Option<u64>,
        summary: Option<StorageScanSummary>,
    ) -> Result<(ControlOperationStatus, u64), String> {
        let (snapshot, operation) = {
            let mut status = self
                .status
                .lock()
                .map_err(|_| "채팅 CLI 연결 상태를 잠글 수 없습니다".to_owned())?;
            let mut history = self
                .completed_operations
                .lock()
                .map_err(|_| "완료된 채팅 작업을 잠글 수 없습니다".to_owned())?;
            if status
                .active_operation
                .as_ref()
                .is_none_or(|operation| operation.operation_id != operation_id)
            {
                return Err("완료할 작업 번호가 현재 작업과 일치하지 않습니다".to_owned());
            }
            let mut operation = status
                .active_operation
                .take()
                .ok_or_else(|| "완료할 작업을 찾을 수 없습니다".to_owned())?;
            operation.state = state;
            operation.message = Some(bounded_error(message));
            operation.finished_at_unix_ms = Some(unix_time_ms());
            operation.scan_generation = scan_generation;
            operation.summary = summary;
            history.push_front(operation.clone());
            history.truncate(MAX_COMPLETED_OPERATIONS);
            status.last_operation = Some(operation.clone());
            status.revision = status.revision.saturating_add(1);
            (status.clone(), operation)
        };
        let revision = snapshot.revision;
        let _ = app.emit(CONTROL_STATUS_EVENT, snapshot);
        Ok((operation, revision))
    }

    fn operation(&self, operation_id: &str) -> Result<ControlOperationStatus, String> {
        if let Some(operation) = self
            .status
            .lock()
            .map_err(|_| "채팅 CLI 연결 상태를 잠글 수 없습니다".to_owned())?
            .active_operation
            .as_ref()
            .filter(|operation| operation.operation_id == operation_id)
            .cloned()
        {
            return Ok(operation);
        }
        self.completed_operations
            .lock()
            .map_err(|_| "완료된 채팅 작업을 잠글 수 없습니다".to_owned())?
            .iter()
            .find(|operation| operation.operation_id == operation_id)
            .cloned()
            .ok_or_else(|| "이 앱 실행에서 작업 번호를 찾을 수 없습니다".to_owned())
    }

    fn operation_for_start_request(
        &self,
        request_id: &str,
    ) -> Result<Option<ControlOperationStatus>, String> {
        let operation_id = self
            .start_requests
            .lock()
            .map_err(|_| "검사 시작 요청 기록을 잠글 수 없습니다".to_owned())?
            .iter()
            .find(|(stored_request_id, _)| stored_request_id == request_id)
            .map(|(_, operation_id)| operation_id.clone());
        operation_id
            .map(|operation_id| self.operation(&operation_id))
            .transpose()
    }

    fn remember_start_request(
        &self,
        request_id: String,
        operation_id: String,
    ) -> Result<(), String> {
        let mut requests = self
            .start_requests
            .lock()
            .map_err(|_| "검사 시작 요청 기록을 잠글 수 없습니다".to_owned())?;
        requests.push_front((request_id, operation_id));
        requests.truncate(MAX_START_REQUESTS);
        Ok(())
    }

    fn allowed_root(&self, kind: SearchScopeKind) -> Result<PathBuf, String> {
        let scopes = self
            .search_scopes
            .lock()
            .map_err(|_| "채팅 검색 허용 범위를 잠글 수 없습니다".to_owned())?;
        let root = match kind {
            SearchScopeKind::Files => scopes.file_root.as_ref(),
            SearchScopeKind::Documents => scopes.document_root.as_ref(),
        };
        root.cloned()
            .ok_or_else(|| "대시보드에서 이 채팅 검색을 먼저 허용해 주세요".to_owned())
    }

    fn begin_search(&self) -> Result<ExternalSearchPermit<'_>, ()> {
        self.search_active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| ExternalSearchPermit(&self.search_active))
            .map_err(|_| ())
    }
}

fn remember_cleanup_plan(
    completed: &mut VecDeque<CleanupPlanStatusResponse>,
    status: CleanupPlanStatusResponse,
) {
    completed.push_front(status);
    completed.truncate(MAX_COMPLETED_CLEANUP_PLANS);
}

fn expire_pending_cleanup_plan(review: &mut CleanupReviewState, now: u64) -> bool {
    let should_expire = review.pending.as_ref().is_some_and(|plan| {
        plan.status.state == CleanupPlanState::AwaitingApproval
            && now >= plan.status.expires_at_unix_ms
    });
    if !should_expire {
        return false;
    }
    let Some(mut plan) = review.pending.take() else {
        return false;
    };
    plan.status.state = CleanupPlanState::Expired;
    plan.status.message = Some("앱 확인 시간이 지나 파일을 변경하지 않았습니다".to_owned());
    remember_cleanup_plan(&mut review.completed, plan.status);
    true
}

#[derive(Clone, Copy)]
enum SearchScopeKind {
    Files,
    Documents,
}

struct ExternalSearchPermit<'a>(&'a AtomicBool);

impl Drop for ExternalSearchPermit<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

#[tauri::command]
pub(crate) fn get_control_status(
    state: State<'_, ControlStatusStore>,
) -> Result<ControlStatus, String> {
    state.snapshot()
}

#[tauri::command]
pub(crate) fn configure_control_search_access(
    app: AppHandle,
    request: ControlSearchAccessRequest,
    state: State<'_, ControlStatusStore>,
) -> Result<ControlStatus, String> {
    state.configure_search_access(&app, request)
}

#[tauri::command]
pub(crate) fn configure_control_scan_access(
    app: AppHandle,
    request: ControlScanAccessRequest,
    state: State<'_, ControlStatusStore>,
) -> Result<ControlStatus, String> {
    state.configure_scan_access(&app, request)
}

#[tauri::command]
pub(crate) fn configure_control_cleanup_access(
    app: AppHandle,
    request: ControlCleanupAccessRequest,
    state: State<'_, ControlStatusStore>,
) -> Result<ControlStatus, String> {
    state.configure_cleanup_access(&app, request)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ApproveCleanupPlanRequest {
    plan_id: String,
    allow_review_candidates: bool,
}

#[tauri::command]
pub(crate) fn get_pending_cleanup_plan(
    app: AppHandle,
    state: State<'_, ControlStatusStore>,
    reports: State<'_, super::StoredReports>,
) -> Result<Option<super::PendingCleanupPlanDetail>, String> {
    let Some(execution) = state.pending_cleanup_plan(&app)? else {
        return Ok(None);
    };
    match reports.pending_cleanup_plan_detail(&execution) {
        Ok(detail) => Ok(Some(detail)),
        Err(error) => {
            let _ = state.finish_cleanup_plan(
                &app,
                &execution.plan_id,
                CleanupPlanState::Stale,
                None,
                "검사 결과가 바뀌어 정리 계획을 실행하지 않았습니다".to_owned(),
            );
            Err(error)
        }
    }
}

#[tauri::command]
pub(crate) fn reject_cleanup_plan(
    app: AppHandle,
    plan_id: String,
    state: State<'_, ControlStatusStore>,
) -> Result<bool, String> {
    CleanupPlanReference::new(plan_id.clone()).map_err(|error| error.to_string())?;
    state.reject_cleanup_plan(&app, &plan_id)
}

#[tauri::command]
pub(crate) async fn approve_cleanup_plan(
    app: AppHandle,
    request: ApproveCleanupPlanRequest,
    state: State<'_, ControlStatusStore>,
    runtime: State<'_, super::ScanRuntime>,
    reports: State<'_, super::StoredReports>,
) -> Result<super::trash_actions::TrashOperationResult, String> {
    CleanupPlanReference::new(request.plan_id.clone()).map_err(|error| error.to_string())?;
    let execution = state.begin_cleanup_plan(&app, &request.plan_id)?;
    let action = match reports.cleanup_plan_action(&execution, request.allow_review_candidates) {
        Ok(action) => action,
        Err(error) => {
            let _ = state.finish_cleanup_plan(
                &app,
                &execution.plan_id,
                CleanupPlanState::Stale,
                None,
                "검사 결과가 바뀌어 파일을 이동하지 않았습니다".to_owned(),
            );
            return Err(error);
        }
    };

    let result = match action {
        super::CleanupPlanAction::Duplicate(request) => {
            super::trash_actions::trash_duplicate_files_internal(
                app.clone(),
                runtime.inner(),
                reports.inner(),
                request,
            )
            .await
        }
        super::CleanupPlanAction::System(request) => {
            super::trash_actions::trash_cleanup_candidates_internal(
                app.clone(),
                runtime.inner(),
                reports.inner(),
                request,
            )
            .await
        }
    };

    match result {
        Ok(result) => {
            let failed_count = result
                .items
                .iter()
                .filter(|item| item.status == super::trash_actions::TrashItemStatus::Failed)
                .count()
                .try_into()
                .unwrap_or(u64::MAX);
            let skipped_count = result
                .items
                .iter()
                .filter(|item| item.status == super::trash_actions::TrashItemStatus::Skipped)
                .count()
                .try_into()
                .unwrap_or(u64::MAX);
            let summary = CleanupPlanResultSummary {
                moved_count: result.moved_count.try_into().unwrap_or(u64::MAX),
                moved_bytes: result.moved_bytes,
                skipped_count,
                failed_count,
                cancelled: result.cancelled,
                journal_complete: result.journal_complete,
            };
            let plan_state = if result.cancelled {
                CleanupPlanState::Cancelled
            } else {
                CleanupPlanState::Completed
            };
            let message = if result.cancelled {
                "사용자가 정리 작업 중단을 요청했습니다"
            } else if failed_count > 0 || skipped_count > 0 {
                "정리 작업을 마쳤지만 일부 항목은 이동하지 못했습니다"
            } else {
                "앱에서 확인한 항목을 운영체제 휴지통으로 이동했습니다"
            };
            state.finish_cleanup_plan(
                &app,
                &execution.plan_id,
                plan_state,
                Some(summary),
                message.to_owned(),
            )?;
            Ok(result)
        }
        Err(error) => {
            let _ = state.finish_cleanup_plan(
                &app,
                &execution.plan_id,
                CleanupPlanState::Failed,
                None,
                "안전 재검사를 통과하지 못해 파일을 이동하지 않았습니다".to_owned(),
            );
            Err(error)
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ControlScanProgressEvent {
    operation_id: String,
    revision: u64,
    progress: ScanProgress,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ControlScanCompletedEvent {
    operation_id: String,
    revision: u64,
    state: ControlOperationState,
    scan_generation: Option<u64>,
    message: String,
}

pub(crate) fn record_scan_progress(app: &AppHandle, operation_id: &str, progress: ScanProgress) {
    let Ok(status) =
        app.state::<ControlStatusStore>()
            .update_operation_progress(app, operation_id, &progress)
    else {
        return;
    };
    let _ = app.emit(
        "control-scan-progress",
        ControlScanProgressEvent {
            operation_id: operation_id.to_owned(),
            revision: status.revision,
            progress,
        },
    );
}

pub(crate) fn record_start_failure(app: &AppHandle, error: String) {
    app.state::<ControlStatusStore>()
        .set_unavailable_error(app, error);
}

#[derive(Default)]
struct ConnectionRegistry {
    active: AtomicUsize,
    next_id: AtomicUsize,
    threads: Mutex<Vec<JoinHandle<()>>>,
    streams: Mutex<HashMap<usize, TcpStream>>,
}

pub(crate) struct ControlServer {
    _instance_lock: ControlInstanceLock,
    shutdown: Arc<AtomicBool>,
    wake_address: SocketAddr,
    descriptor: EndpointDescriptor,
    descriptor_path: PathBuf,
    accept_thread: Mutex<Option<JoinHandle<()>>>,
    connections: Arc<ConnectionRegistry>,
}

pub(crate) fn start(app: AppHandle) -> Result<ControlServer, String> {
    let lock_path = default_lock_path().map_err(|error| error.to_string())?;
    let instance_lock = acquire_instance_lock(&lock_path).map_err(|error| error.to_string())?;
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .map_err(|error| format!("채팅 CLI 연결 통로를 열지 못했습니다: {error}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|error| format!("채팅 CLI 연결 통로를 준비하지 못했습니다: {error}"))?;
    let wake_address = listener
        .local_addr()
        .map_err(|error| format!("채팅 CLI 연결 주소를 확인하지 못했습니다: {error}"))?;
    if !wake_address.ip().is_loopback() {
        return Err("채팅 CLI 연결 통로가 로컬 주소에 열리지 않았습니다".to_owned());
    }

    let descriptor = EndpointDescriptor::new_loopback(
        wake_address,
        random_token().map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let descriptor_path = default_descriptor_path().map_err(|error| error.to_string())?;
    write_descriptor(&descriptor_path, &descriptor).map_err(|error| error.to_string())?;

    let shutdown = Arc::new(AtomicBool::new(false));
    let connections = Arc::new(ConnectionRegistry::default());
    let accept_shutdown = Arc::clone(&shutdown);
    let accept_connections = Arc::clone(&connections);
    let expected_token = descriptor.token.clone();
    let app_for_accept = app.clone();
    let accept_thread = thread::Builder::new()
        .name("bloomsweepy-control-listener".to_owned())
        .spawn(move || {
            accept_loop(
                listener,
                app_for_accept,
                expected_token,
                accept_shutdown,
                accept_connections,
            );
        })
        .map_err(|error| {
            let _ = remove_descriptor_if_owned(&descriptor_path, &descriptor);
            format!("채팅 CLI 연결 대기 작업을 시작하지 못했습니다: {error}")
        })?;

    app.state::<ControlStatusStore>().set_available(&app);
    Ok(ControlServer {
        _instance_lock: instance_lock,
        shutdown,
        wake_address,
        descriptor,
        descriptor_path,
        accept_thread: Mutex::new(Some(accept_thread)),
        connections,
    })
}

impl Drop for ControlServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl ControlServer {
    pub(crate) fn shutdown(&self) {
        if self.shutdown.swap(true, Ordering::AcqRel) {
            return;
        }
        let _ = TcpStream::connect_timeout(&self.wake_address, Duration::from_millis(100))
            .and_then(|stream| stream.shutdown(Shutdown::Both));
        if let Ok(streams) = self.connections.streams.lock() {
            for stream in streams.values() {
                let _ = stream.shutdown(Shutdown::Both);
            }
        }
        if let Ok(mut thread) = self.accept_thread.lock()
            && let Some(thread) = thread.take()
        {
            let _ = thread.join();
        }
        if let Ok(mut workers) = self.connections.threads.lock() {
            for worker in workers.drain(..) {
                let _ = worker.join();
            }
        }
        let _ = remove_descriptor_if_owned(&self.descriptor_path, &self.descriptor);
    }
}

fn accept_loop(
    listener: TcpListener,
    app: AppHandle,
    expected_token: String,
    shutdown: Arc<AtomicBool>,
    connections: Arc<ConnectionRegistry>,
) {
    while !shutdown.load(Ordering::Acquire) {
        reap_finished_threads(&connections.threads);
        match listener.accept() {
            Ok((stream, peer)) => {
                if shutdown.load(Ordering::Acquire)
                    || stream.set_nonblocking(false).is_err()
                    || !peer.ip().is_loopback()
                    || connections.active.load(Ordering::Acquire) >= MAX_CONNECTIONS
                {
                    let _ = stream.shutdown(Shutdown::Both);
                    continue;
                }
                let connection_id = connections.next_id.fetch_add(1, Ordering::AcqRel);
                let shutdown_stream = match stream.try_clone() {
                    Ok(stream) => stream,
                    Err(_) => {
                        let _ = stream.shutdown(Shutdown::Both);
                        continue;
                    }
                };
                if connections
                    .streams
                    .lock()
                    .map(|mut streams| streams.insert(connection_id, shutdown_stream))
                    .is_err()
                {
                    let _ = stream.shutdown(Shutdown::Both);
                    continue;
                }
                connections.active.fetch_add(1, Ordering::AcqRel);
                let worker_app = app.clone();
                let worker_token = expected_token.clone();
                let worker_connections = Arc::clone(&connections);
                let worker_shutdown = Arc::clone(&shutdown);
                match thread::Builder::new()
                    .name("bloomsweepy-control-connection".to_owned())
                    .spawn(move || {
                        let _active_guard = ActiveConnectionGuard {
                            connections: worker_connections,
                            connection_id,
                        };
                        handle_connection(stream, worker_app, &worker_token, worker_shutdown);
                    }) {
                    Ok(worker) => {
                        if let Ok(mut workers) = connections.threads.lock() {
                            workers.push(worker);
                        } else {
                            let _ = worker.join();
                        }
                    }
                    Err(_) => {
                        connections.active.fetch_sub(1, Ordering::AcqRel);
                        if let Ok(mut streams) = connections.streams.lock() {
                            streams.remove(&connection_id);
                        }
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(ACCEPT_RETRY_DELAY);
            }
            Err(error) => {
                app.state::<ControlStatusStore>().set_unavailable_error(
                    &app,
                    format!("채팅 CLI 연결을 받지 못했습니다: {error}"),
                );
                break;
            }
        }
    }
}

fn reap_finished_threads(threads: &Mutex<Vec<JoinHandle<()>>>) {
    let finished = if let Ok(mut workers) = threads.lock() {
        let mut finished = Vec::new();
        let mut index = 0;
        while index < workers.len() {
            if workers[index].is_finished() {
                finished.push(workers.swap_remove(index));
            } else {
                index += 1;
            }
        }
        finished
    } else {
        return;
    };
    for worker in finished {
        let _ = worker.join();
    }
}

struct ActiveConnectionGuard {
    connections: Arc<ConnectionRegistry>,
    connection_id: usize,
}

impl Drop for ActiveConnectionGuard {
    fn drop(&mut self) {
        if let Ok(mut streams) = self.connections.streams.lock() {
            streams.remove(&self.connection_id);
        }
        self.connections.active.fetch_sub(1, Ordering::AcqRel);
    }
}

struct DeadlineReader<'a> {
    stream: &'a mut TcpStream,
    deadline: Instant,
    shutdown: &'a AtomicBool,
}

impl Read for DeadlineReader<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "server is shutting down",
            ));
        }
        let remaining = self
            .deadline
            .checked_duration_since(Instant::now())
            .filter(|remaining| !remaining.is_zero())
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "frame deadline elapsed"))?;
        self.stream.set_read_timeout(Some(remaining))?;
        self.stream.read(buffer)
    }
}

struct DeadlineWriter<'a> {
    stream: &'a mut TcpStream,
    deadline: Instant,
    shutdown: &'a AtomicBool,
}

impl DeadlineWriter<'_> {
    fn remaining(&self) -> io::Result<Duration> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "server is shutting down",
            ));
        }
        self.deadline
            .checked_duration_since(Instant::now())
            .filter(|remaining| !remaining.is_zero())
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "frame deadline elapsed"))
    }
}

impl Write for DeadlineWriter<'_> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let remaining = self.remaining()?;
        self.stream.set_write_timeout(Some(remaining))?;
        self.stream.write(buffer)
    }

    fn flush(&mut self) -> io::Result<()> {
        let remaining = self.remaining()?;
        self.stream.set_write_timeout(Some(remaining))?;
        self.stream.flush()
    }
}

fn handle_connection(
    mut stream: TcpStream,
    app: AppHandle,
    expected_token: &str,
    shutdown: Arc<AtomicBool>,
) {
    if shutdown.load(Ordering::Acquire) {
        return;
    }
    if stream
        .peer_addr()
        .map(|peer| !is_loopback(peer.ip()))
        .unwrap_or(true)
    {
        return;
    }
    let shutdown_ref = shutdown.as_ref();
    let request: ControlRequest = {
        let mut reader = DeadlineReader {
            stream: &mut stream,
            deadline: Instant::now() + CONNECTION_TIMEOUT,
            shutdown: shutdown_ref,
        };
        match read_json_frame(&mut reader, MAX_REQUEST_BYTES) {
            Ok(request) => request,
            Err(_) => return,
        }
    };
    if shutdown.load(Ordering::Acquire) {
        return;
    }
    let response = if request.protocol_version != PROTOCOL_VERSION {
        ControlResponse::error(
            request.request_id,
            "version_mismatch",
            format!(
                "제어 규격 버전이 다릅니다: 예상 {PROTOCOL_VERSION}, 실제 {}",
                request.protocol_version
            ),
            false,
        )
    } else if !tokens_match(&request.token, expected_token) {
        ControlResponse::error(
            request.request_id,
            "authentication_failed",
            "채팅 CLI 연결을 확인할 수 없습니다",
            false,
        )
    } else if !valid_request_id(&request.request_id) {
        ControlResponse::error(
            request.request_id,
            "invalid_request",
            "요청 번호가 올바르지 않습니다",
            false,
        )
    } else {
        let _connection_status = ConnectionStatusGuard::new(app.clone());
        handle_authenticated_request(&app, request, Arc::clone(&shutdown))
    };
    if !shutdown.load(Ordering::Acquire) {
        let mut writer = DeadlineWriter {
            stream: &mut stream,
            deadline: Instant::now() + CONNECTION_TIMEOUT,
            shutdown: shutdown_ref,
        };
        let _ = write_json_frame(&mut writer, &response, MAX_RESPONSE_BYTES);
    }
}

fn handle_authenticated_request(
    app: &AppHandle,
    request: ControlRequest,
    shutdown: Arc<AtomicBool>,
) -> ControlResponse {
    let request_id = request.request_id;
    let result = match request.command {
        ControlCommand::AppStatus => app
            .state::<ControlStatusStore>()
            .snapshot()
            .map_err(|message| RequestFailure {
                code: "status_unavailable",
                message,
                retryable: true,
            })
            .and_then(to_value),
        ControlCommand::SystemOverview => to_value(super::collect_system_overview()),
        ControlCommand::SearchFiles(request) => search_files(app, request, shutdown),
        ControlCommand::SearchDocuments(request) => search_documents(app, request, shutdown),
        ControlCommand::StartStorageScan => start_storage_scan(app, &request_id),
        ControlCommand::OperationStatus(reference) => operation_status(app, reference),
        ControlCommand::CancelOperation(reference) => cancel_operation(app, reference),
        ControlCommand::CleanupCandidates(request) => cleanup_candidates(app, request),
        ControlCommand::CreateCleanupPlan(request) => create_cleanup_plan(app, request),
        ControlCommand::CleanupPlanStatus(reference) => cleanup_plan_status(app, reference),
    };

    match result {
        Ok(result) => {
            app.state::<ControlStatusStore>().request_succeeded(app);
            ControlResponse::ok(request_id, result)
        }
        Err(error) => {
            app.state::<ControlStatusStore>()
                .request_failed(app, error.message.clone());
            ControlResponse::error(request_id, error.code, error.message, error.retryable)
        }
    }
}

struct RequestFailure {
    code: &'static str,
    message: String,
    retryable: bool,
}

fn start_storage_scan(app: &AppHandle, request_id: &str) -> Result<Value, RequestFailure> {
    let status_store = app.state::<ControlStatusStore>();
    if let Some(operation) = status_store
        .operation_for_start_request(request_id)
        .map_err(|message| RequestFailure {
            code: "operation_status_failed",
            message,
            retryable: true,
        })?
    {
        return to_value(operation);
    }
    let plan = status_store.scan_plan().map_err(|message| RequestFailure {
        code: "scan_approval_required",
        message,
        retryable: false,
    })?;
    let operation_id = random_operation_id().map_err(|_| RequestFailure {
        code: "operation_id_failed",
        message: "검사 작업 번호를 만들지 못했습니다".to_owned(),
        retryable: true,
    })?;
    let runtime = app.state::<super::ScanRuntime>();
    let cancellation = runtime
        .begin_control(operation_id.clone())
        .map_err(|message| RequestFailure {
            code: "busy",
            message,
            retryable: true,
        })?;
    let completion = super::ScanCompletionGuard::for_control(app.clone(), operation_id.clone());
    if let Err(message) = app.state::<super::StoredReports>().clear_scan() {
        drop(completion);
        return Err(RequestFailure {
            code: "scan_start_failed",
            message,
            retryable: true,
        });
    }

    let operation = ControlOperationStatus {
        operation_id: operation_id.clone(),
        kind: "storageScan".to_owned(),
        source: ControlOperationSource::ChatCli,
        state: ControlOperationState::Running,
        cancellation_requested: false,
        message: Some("허용한 폴더 검사를 시작했습니다".to_owned()),
        processed_items: Some(0),
        processed_bytes: Some(0),
        started_at_unix_ms: unix_time_ms(),
        finished_at_unix_ms: None,
        scan_generation: None,
        summary: None,
    };
    let operation = match status_store.start_operation(app, operation) {
        Ok(operation) => operation,
        Err(message) => {
            drop(completion);
            return Err(RequestFailure {
                code: "busy",
                message,
                retryable: true,
            });
        }
    };
    let _ = status_store.remember_start_request(request_id.to_owned(), operation_id.clone());

    let task_app = app.clone();
    let task_operation_id = operation_id.clone();
    tauri::async_runtime::spawn(async move {
        let result = super::execute_reserved_scan(
            task_app.clone(),
            plan.canonical_root,
            plan.config,
            cancellation,
            completion.clone(),
            super::ScanProgressTarget::Control {
                operation_id: task_operation_id.clone(),
            },
        )
        .await;

        let (state, message, scan_generation, summary) = match result {
            Ok(snapshot) => (
                ControlOperationState::Completed,
                "허용한 폴더 검사를 완료했습니다".to_owned(),
                Some(snapshot.generation),
                Some(storage_scan_summary(&snapshot)),
            ),
            Err(error) if error.is_cancelled() => (
                ControlOperationState::Cancelled,
                "검사를 취소했습니다".to_owned(),
                None,
                None,
            ),
            Err(error) => (ControlOperationState::Failed, error.message(), None, None),
        };
        let Ok((_, revision)) = task_app.state::<ControlStatusStore>().finish_operation(
            &task_app,
            &task_operation_id,
            state,
            message.clone(),
            scan_generation,
            summary,
        ) else {
            return;
        };
        let _ = task_app.emit(
            "control-scan-completed",
            ControlScanCompletedEvent {
                operation_id: task_operation_id,
                revision,
                state,
                scan_generation,
                message,
            },
        );
        drop(completion);
    });

    to_value(operation)
}

fn operation_status(
    app: &AppHandle,
    reference: OperationReference,
) -> Result<Value, RequestFailure> {
    reference.validate().map_err(invalid_request)?;
    let operation = app
        .state::<ControlStatusStore>()
        .operation(&reference.operation_id)
        .map_err(operation_not_found)?;
    to_value(operation)
}

fn cancel_operation(
    app: &AppHandle,
    reference: OperationReference,
) -> Result<Value, RequestFailure> {
    reference.validate().map_err(invalid_request)?;
    let status_store = app.state::<ControlStatusStore>();
    match app
        .state::<super::ScanRuntime>()
        .cancel_control(&reference.operation_id)
        .map_err(|message| RequestFailure {
            code: "cancel_failed",
            message,
            retryable: true,
        })? {
        super::ControlCancelOutcome::Requested => {
            let operation = match status_store
                .request_operation_cancellation(app, &reference.operation_id)
            {
                Ok(operation) => operation,
                Err(message) => status_store
                    .operation(&reference.operation_id)
                    .map_err(|_| RequestFailure {
                        code: "cancel_failed",
                        message,
                        retryable: true,
                    })?,
            };
            to_value(operation)
        }
        super::ControlCancelOutcome::TooLate | super::ControlCancelOutcome::NotActive => {
            status_store
                .operation(&reference.operation_id)
                .map_err(operation_not_found)
                .and_then(to_value)
        }
    }
}

fn storage_scan_summary(snapshot: &super::StoredScanSnapshot) -> StorageScanSummary {
    let report = &snapshot.report;
    StorageScanSummary {
        root: report.root.clone(),
        completed_at_unix_ms: saturating_u64(report.completed_at_unix_ms),
        duration_ms: saturating_u64(report.duration_ms),
        total_files: report.total_files,
        total_logical_bytes: report.total_logical_bytes,
        large_file_count: report.large_files.len().try_into().unwrap_or(u64::MAX),
        duplicate_group_count: report.duplicate_groups.len().try_into().unwrap_or(u64::MAX),
        duplicate_waste_bytes: report.duplicate_waste_bytes,
        unreadable_entries: report.unreadable_entries,
        issue_count: report.issues.len().try_into().unwrap_or(u64::MAX),
        candidate_limit_reached: report.candidate_limit_reached,
        hard_link_identity_limit_reached: report.hard_link_identity_limit_reached,
    }
}

fn cleanup_candidates(
    app: &AppHandle,
    request: CleanupCandidatesRequest,
) -> Result<Value, RequestFailure> {
    request.validate().map_err(invalid_request)?;
    app.state::<ControlStatusStore>()
        .ensure_cleanup_access()
        .map_err(|message| RequestFailure {
            code: "cleanup_approval_required",
            message,
            retryable: false,
        })?;
    let page = app
        .state::<super::StoredReports>()
        .cleanup_candidates_page(
            request.source,
            request.expected_generation,
            request.offset,
            request.max_results,
        )
        .map_err(cleanup_report_failure)?;
    to_value(page)
}

fn create_cleanup_plan(
    app: &AppHandle,
    request: CreateCleanupPlanRequest,
) -> Result<Value, RequestFailure> {
    request.validate().map_err(invalid_request)?;
    let status_store = app.state::<ControlStatusStore>();
    status_store
        .ensure_cleanup_access()
        .map_err(|message| RequestFailure {
            code: "cleanup_approval_required",
            message,
            retryable: false,
        })?;
    let selection = app
        .state::<super::StoredReports>()
        .cleanup_plan_selection(
            request.source,
            request.source_generation,
            &request.candidate_ids,
        )
        .map_err(cleanup_report_failure)?;
    let status = status_store
        .create_cleanup_plan(app, &request, &selection)
        .map_err(|message| RequestFailure {
            code: if message.contains("다른 정리 계획") {
                "review_pending"
            } else {
                "cleanup_plan_failed"
            },
            message,
            retryable: false,
        })?;
    to_value(status)
}

fn cleanup_plan_status(
    app: &AppHandle,
    reference: CleanupPlanReference,
) -> Result<Value, RequestFailure> {
    reference.validate().map_err(invalid_request)?;
    let status = app
        .state::<ControlStatusStore>()
        .cleanup_plan_status(app, &reference.plan_id)
        .map_err(|message| RequestFailure {
            code: "cleanup_plan_not_found",
            message,
            retryable: false,
        })?;
    to_value(status)
}

fn cleanup_report_failure(message: String) -> RequestFailure {
    let code = if message.contains("바뀌") || message.contains("세대") {
        "stale_generation"
    } else if message.contains("없습니다") {
        "cleanup_report_missing"
    } else if message.contains("후보") || message.contains("그룹") {
        "invalid_candidate"
    } else {
        "cleanup_report_failed"
    };
    RequestFailure {
        code,
        message,
        retryable: false,
    }
}

fn search_files(
    app: &AppHandle,
    request: FileSearchRequest,
    shutdown: Arc<AtomicBool>,
) -> Result<Value, RequestFailure> {
    request.validate().map_err(invalid_request)?;
    let status_store = app.state::<ControlStatusStore>();
    let allowed_root = status_store
        .allowed_root(SearchScopeKind::Files)
        .map_err(scope_required)?;
    let _search_permit = status_store.begin_search().map_err(|_| RequestFailure {
        code: "busy",
        message: "다른 채팅 검색이 진행 중입니다. 잠시 후 다시 시도해 주세요".to_owned(),
        retryable: true,
    })?;
    let core_request: CoreFileSearchRequest = convert_request(request)?;
    let runtime = app.state::<super::ScanRuntime>();
    super::validate_search_result_limit(core_request.max_results).map_err(busy_or_invalid)?;
    let runtime_cancellation = runtime.begin().map_err(busy_or_invalid)?;
    let _runtime_completion = super::ScanCompletionGuard::new(app.clone());
    let database_path = super::file_catalog_path(app)
        .map_err(|_| search_failure("파일 검색 목록 위치를 확인하지 못했습니다"))?;
    let index_status = file_catalog_status(&database_path)
        .map_err(|_| search_failure("파일 검색 목록을 확인하지 못했습니다"))?
        .ok_or_else(|| search_failure("앱에서 파일 목록을 먼저 만들어 주세요"))?;
    ensure_allowed_root(&index_status.root, &allowed_root)?;
    let deadline = Instant::now() + SEARCH_TIMEOUT;
    let shutdown_for_search = Arc::clone(&shutdown);
    let runtime_cancel_for_search = Arc::clone(&runtime_cancellation);
    let report = search_file_catalog_with_cancellation(database_path, core_request, move || {
        shutdown_for_search.load(Ordering::Acquire)
            || runtime_cancel_for_search.load(Ordering::Acquire)
            || Instant::now() >= deadline
    })
    .map_err(|error| {
        map_file_search_error(
            error,
            Instant::now() >= deadline,
            shutdown.load(Ordering::Acquire) || runtime_cancellation.load(Ordering::Acquire),
        )
    })?;
    ensure_allowed_root(&report.root, &allowed_root)?;
    if report
        .results
        .iter()
        .any(|result| !path_is_within_root(&result.path, &report.root))
    {
        return Err(scope_mismatch());
    }
    to_value(report)
}

fn search_documents(
    app: &AppHandle,
    request: DocumentSearchRequest,
    shutdown: Arc<AtomicBool>,
) -> Result<Value, RequestFailure> {
    request.validate().map_err(invalid_request)?;
    let status_store = app.state::<ControlStatusStore>();
    let allowed_root = status_store
        .allowed_root(SearchScopeKind::Documents)
        .map_err(scope_required)?;
    let _search_permit = status_store.begin_search().map_err(|_| RequestFailure {
        code: "busy",
        message: "다른 채팅 검색이 진행 중입니다. 잠시 후 다시 시도해 주세요".to_owned(),
        retryable: true,
    })?;
    let core_request: CoreDocumentSearchRequest = convert_request(request)?;
    let runtime = app.state::<super::ScanRuntime>();
    super::validate_search_result_limit(core_request.max_results).map_err(busy_or_invalid)?;
    let runtime_cancellation = runtime.begin().map_err(busy_or_invalid)?;
    let _runtime_completion = super::ScanCompletionGuard::new(app.clone());
    let database_path = super::document_index_path(app)
        .map_err(|_| search_failure("문서 검색 목록 위치를 확인하지 못했습니다"))?;
    let index_status = document_index_status(&database_path)
        .map_err(|_| search_failure("문서 검색 목록을 확인하지 못했습니다"))?
        .ok_or_else(|| search_failure("앱에서 문서 목록을 먼저 만들어 주세요"))?;
    ensure_allowed_root(&index_status.root, &allowed_root)?;
    let deadline = Instant::now() + SEARCH_TIMEOUT;
    let shutdown_for_search = Arc::clone(&shutdown);
    let runtime_cancel_for_search = Arc::clone(&runtime_cancellation);
    let report = search_document_index_with_cancellation(database_path, core_request, move || {
        shutdown_for_search.load(Ordering::Acquire)
            || runtime_cancel_for_search.load(Ordering::Acquire)
            || Instant::now() >= deadline
    })
    .map_err(|error| {
        map_document_search_error(
            error,
            Instant::now() >= deadline,
            shutdown.load(Ordering::Acquire) || runtime_cancellation.load(Ordering::Acquire),
        )
    })?;
    ensure_allowed_root(&report.root, &allowed_root)?;
    if report
        .results
        .iter()
        .any(|result| !path_is_within_root(&result.path, &report.root))
    {
        return Err(scope_mismatch());
    }
    to_value(report)
}

fn convert_request<T: Serialize, U: serde::de::DeserializeOwned>(
    request: T,
) -> Result<U, RequestFailure> {
    serde_json::to_value(request)
        .and_then(serde_json::from_value)
        .map_err(|error| RequestFailure {
            code: "invalid_request",
            message: format!("검색 조건을 읽지 못했습니다: {error}"),
            retryable: false,
        })
}

fn to_value(value: impl Serialize) -> Result<Value, RequestFailure> {
    let encoded = serde_json::to_vec(&value).map_err(|error| RequestFailure {
        code: "serialization_failed",
        message: format!("결과 크기를 확인하지 못했습니다: {error}"),
        retryable: false,
    })?;
    if encoded.len() > MAX_RESULT_VALUE_BYTES {
        return Err(RequestFailure {
            code: "response_too_large",
            message: "결과가 너무 큽니다. 검색어를 좁히거나 결과 개수를 줄여 주세요".to_owned(),
            retryable: false,
        });
    }
    serde_json::to_value(value).map_err(|error| RequestFailure {
        code: "serialization_failed",
        message: format!("결과를 전달 형식으로 바꾸지 못했습니다: {error}"),
        retryable: false,
    })
}

fn invalid_request(error: impl ToString) -> RequestFailure {
    RequestFailure {
        code: "invalid_request",
        message: error.to_string(),
        retryable: false,
    }
}

fn busy_or_invalid(message: String) -> RequestFailure {
    let busy = message.contains("진행 중");
    RequestFailure {
        code: if busy { "busy" } else { "invalid_request" },
        message,
        retryable: busy,
    }
}

fn search_failure(message: impl Into<String>) -> RequestFailure {
    RequestFailure {
        code: "search_failed",
        message: message.into(),
        retryable: true,
    }
}

fn map_file_search_error(
    error: FileCatalogError,
    timed_out: bool,
    cancelled: bool,
) -> RequestFailure {
    match error {
        error @ FileCatalogError::EmptyQuery
        | error @ FileCatalogError::QueryTooLong
        | error @ FileCatalogError::InvalidQuery(_) => invalid_request(error),
        _ if timed_out => RequestFailure {
            code: "search_timeout",
            message:
                "파일 검색을 제한 시간 안에 마치지 못했습니다. 검색어를 좁혀 다시 시도해 주세요"
                    .to_owned(),
            retryable: true,
        },
        _ if cancelled => RequestFailure {
            code: "search_cancelled",
            message: "파일 검색을 취소했습니다".to_owned(),
            retryable: true,
        },
        error => search_failure(format!("파일 검색을 완료하지 못했습니다: {error}")),
    }
}

fn map_document_search_error(
    error: DocumentSearchError,
    timed_out: bool,
    cancelled: bool,
) -> RequestFailure {
    match error {
        error @ DocumentSearchError::EmptyQuery | error @ DocumentSearchError::QueryTooLong => {
            invalid_request(error)
        }
        _ if timed_out => RequestFailure {
            code: "search_timeout",
            message:
                "문서 검색을 제한 시간 안에 마치지 못했습니다. 검색어를 좁혀 다시 시도해 주세요"
                    .to_owned(),
            retryable: true,
        },
        _ if cancelled => RequestFailure {
            code: "search_cancelled",
            message: "문서 검색을 취소했습니다".to_owned(),
            retryable: true,
        },
        error => search_failure(format!("문서 검색을 완료하지 못했습니다: {error}")),
    }
}

fn scope_required(message: String) -> RequestFailure {
    RequestFailure {
        code: "scope_required",
        message,
        retryable: false,
    }
}

fn scope_mismatch() -> RequestFailure {
    RequestFailure {
        code: "scope_changed",
        message: "허용한 검색 위치가 바뀌었습니다. 대시보드에서 다시 허용해 주세요".to_owned(),
        retryable: false,
    }
}

fn operation_not_found(message: String) -> RequestFailure {
    RequestFailure {
        code: "operation_not_found",
        message,
        retryable: false,
    }
}

fn validate_scan_config(config: &ScanConfig) -> Result<(), String> {
    if config.min_large_file_bytes == 0 || config.min_duplicate_file_bytes == 0 {
        return Err("파일 크기 기준은 0보다 커야 합니다".to_owned());
    }
    if !(1..=10_000).contains(&config.max_large_files)
        || !(1..=10_000).contains(&config.max_duplicate_groups)
        || !(1..=250_000).contains(&config.max_duplicate_candidates)
        || !(1..=1_000).contains(&config.max_issues)
    {
        return Err("검사 결과 한도가 허용 범위를 벗어났습니다".to_owned());
    }
    Ok(())
}

fn canonical_directory(root: &str) -> Result<PathBuf, String> {
    let path = Path::new(root);
    if !path.is_dir() {
        return Err("검색을 허용할 폴더를 찾을 수 없습니다".to_owned());
    }
    path.canonicalize()
        .map_err(|_| "검색을 허용할 폴더를 확인하지 못했습니다".to_owned())
}

fn ensure_allowed_root(index_root: &str, allowed_root: &Path) -> Result<(), RequestFailure> {
    let current_root = canonical_directory(index_root).map_err(|_| scope_mismatch())?;
    if current_root == allowed_root {
        Ok(())
    } else {
        Err(scope_mismatch())
    }
}

fn path_is_within_root(candidate: &str, root: &str) -> bool {
    let candidate = Path::new(candidate);
    let root = Path::new(root);
    if candidate
        .components()
        .chain(root.components())
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return false;
    }
    candidate == root || candidate.starts_with(root)
}

fn tokens_match(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    let mut difference = left.len() ^ right.len();
    for index in 0..left.len().max(right.len()) {
        difference |= usize::from(
            left.get(index).copied().unwrap_or_default()
                ^ right.get(index).copied().unwrap_or_default(),
        );
    }
    difference == 0
}

fn valid_request_id(request_id: &str) -> bool {
    request_id.len() == OPERATION_ID_BYTES * 2
        && request_id.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_loopback(address: IpAddr) -> bool {
    address.is_loopback()
}

struct ConnectionStatusGuard {
    app: AppHandle,
}

impl ConnectionStatusGuard {
    fn new(app: AppHandle) -> Self {
        app.state::<ControlStatusStore>().connected(&app);
        Self { app }
    }
}

impl Drop for ConnectionStatusGuard {
    fn drop(&mut self) {
        self.app
            .state::<ControlStatusStore>()
            .disconnected(&self.app);
    }
}

fn bounded_error(error: String) -> String {
    error.chars().take(MAX_STATUS_ERROR_CHARS).collect()
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn saturating_u64(value: u128) -> u64 {
    value.try_into().unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn error_text_is_bounded_by_characters() {
        let long = "가".repeat(MAX_STATUS_ERROR_CHARS + 5);
        let bounded = bounded_error(long);

        assert_eq!(bounded.chars().count(), MAX_STATUS_ERROR_CHARS);
    }

    #[test]
    fn oversized_result_is_replaced_with_a_small_structured_error() {
        let error = to_value("x".repeat(MAX_RESULT_VALUE_BYTES + 1))
            .expect_err("oversized result must be rejected");

        assert_eq!(error.code, "response_too_large");
        assert!(error.message.len() < 200);
    }

    #[test]
    fn search_errors_keep_validation_timeout_and_cancellation_distinct() {
        let invalid = map_file_search_error(FileCatalogError::QueryTooLong, false, false);
        let timeout = map_document_search_error(
            DocumentSearchError::Index("interrupted".to_owned()),
            true,
            false,
        );
        let cancelled = map_file_search_error(
            FileCatalogError::Index("interrupted".to_owned()),
            false,
            true,
        );

        assert_eq!(invalid.code, "invalid_request");
        assert_eq!(timeout.code, "search_timeout");
        assert_eq!(cancelled.code, "search_cancelled");
    }

    #[test]
    fn disconnected_count_saturates_at_zero() {
        let mut status = ControlStatus::default();
        status.connected_clients = status.connected_clients.saturating_sub(1);

        assert_eq!(status.connected_clients, 0);
    }

    #[test]
    fn token_comparison_accepts_only_the_exact_token() {
        assert!(tokens_match("same-token", "same-token"));
        assert!(!tokens_match("same-token", "different-token"));
        assert!(!tokens_match("short", "short-plus-suffix"));
    }

    #[test]
    fn external_search_gate_allows_only_one_search() {
        let store = ControlStatusStore::default();
        let first = store.begin_search().expect("first search permit");

        assert!(store.begin_search().is_err());
        drop(first);
        store
            .begin_search()
            .expect("search after first permit ends");
    }

    #[test]
    fn default_search_access_is_closed() {
        let status = ControlStatus::default();

        assert!(!status.search_access.files);
        assert!(!status.search_access.documents);
        assert!(!status.scan_access.enabled);
        assert!(status.scan_access.root.is_none());
        assert!(!status.cleanup_access.enabled);
        assert!(status.pending_review.is_none());
        assert_eq!(status.revision, 0);
    }

    #[test]
    fn expired_cleanup_plan_is_removed_and_remains_queryable_as_terminal_history() {
        let mut review = CleanupReviewState {
            pending: Some(CleanupReviewPlan {
                status: CleanupPlanStatusResponse {
                    plan_id: "ab".repeat(16),
                    state: CleanupPlanState::AwaitingApproval,
                    source: CleanupSource::SystemCleanup,
                    source_generation: 1,
                    item_count: 1,
                    total_bytes: 4_096,
                    review_count: 0,
                    created_at_unix_ms: 100,
                    expires_at_unix_ms: 200,
                    result: None,
                    message: None,
                },
                candidate_ids: vec!["cd".repeat(16)],
            }),
            completed: VecDeque::new(),
        };

        assert!(expire_pending_cleanup_plan(&mut review, 200));
        assert!(review.pending.is_none());
        assert_eq!(review.completed.len(), 1);
        assert_eq!(review.completed[0].state, CleanupPlanState::Expired);
        assert_eq!(
            review.completed[0].message.as_deref(),
            Some("앱 확인 시간이 지나 파일을 변경하지 않았습니다")
        );
        assert!(!expire_pending_cleanup_plan(&mut review, 201));
    }

    #[test]
    fn request_ids_are_fixed_length_hex_values() {
        assert!(valid_request_id(&"ab".repeat(OPERATION_ID_BYTES)));
        assert!(!valid_request_id("short"));
        assert!(!valid_request_id(&"zz".repeat(OPERATION_ID_BYTES)));
    }

    #[test]
    fn accepted_stream_can_wait_for_a_delayed_frame() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("listener");
        listener
            .set_nonblocking(true)
            .expect("nonblocking listener");
        let address = listener.local_addr().expect("listener address");
        let sender = thread::spawn(move || {
            let mut stream = TcpStream::connect(address).expect("connect");
            thread::sleep(Duration::from_millis(50));
            let request =
                ControlRequest::new("a".repeat(64), ControlCommand::AppStatus).expect("request");
            write_json_frame(&mut stream, &request, MAX_REQUEST_BYTES).expect("write frame");
        });

        let started = Instant::now();
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    assert!(started.elapsed() < Duration::from_secs(1));
                    thread::sleep(Duration::from_millis(5));
                }
                Err(error) => panic!("accept failed: {error}"),
            }
        };
        stream
            .set_nonblocking(false)
            .expect("blocking client stream");
        let shutdown = AtomicBool::new(false);
        let mut reader = DeadlineReader {
            stream: &mut stream,
            deadline: Instant::now() + Duration::from_secs(1),
            shutdown: &shutdown,
        };
        let request: ControlRequest =
            read_json_frame(&mut reader, MAX_REQUEST_BYTES).expect("delayed request");
        assert!(matches!(request.command, ControlCommand::AppStatus));
        sender.join().expect("sender");
    }

    #[test]
    fn frame_deadline_rejects_a_trickling_client() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("listener");
        let address = listener.local_addr().expect("listener address");
        let sender = thread::spawn(move || {
            let mut stream = TcpStream::connect(address).expect("connect");
            let request =
                ControlRequest::new("a".repeat(64), ControlCommand::AppStatus).expect("request");
            let payload = serde_json::to_vec(&request).expect("serialize request");
            let mut frame = (payload.len() as u32).to_be_bytes().to_vec();
            frame.extend(payload);
            for byte in frame {
                if stream.write_all(&[byte]).is_err() {
                    break;
                }
                thread::sleep(Duration::from_millis(40));
            }
        });
        let (mut stream, _) = listener.accept().expect("accept");
        let shutdown = AtomicBool::new(false);
        let started = Instant::now();
        let result: Result<ControlRequest, _> = {
            let mut reader = DeadlineReader {
                stream: &mut stream,
                deadline: started + Duration::from_millis(150),
                shutdown: &shutdown,
            };
            read_json_frame(&mut reader, MAX_REQUEST_BYTES)
        };

        assert!(result.is_err());
        assert!(started.elapsed() < Duration::from_secs(1));
        let _ = stream.shutdown(Shutdown::Both);
        drop(stream);
        sender.join().expect("sender");
    }

    #[test]
    fn scope_paths_reject_sibling_entries() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let root = directory.path().join("allowed");
        let sibling = directory.path().join("other");
        std::fs::create_dir_all(&root).expect("allowed directory");
        std::fs::create_dir_all(&sibling).expect("sibling directory");

        assert!(path_is_within_root(
            root.join("inside.txt").to_string_lossy().as_ref(),
            root.to_string_lossy().as_ref(),
        ));
        assert!(!path_is_within_root(
            sibling.join("outside.txt").to_string_lossy().as_ref(),
            root.to_string_lossy().as_ref(),
        ));
        assert!(!path_is_within_root(
            root.join("..")
                .join("other")
                .join("outside.txt")
                .to_string_lossy()
                .as_ref(),
            root.to_string_lossy().as_ref(),
        ));
    }
}
