use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;

pub const PROTOCOL_VERSION: u16 = 3;
pub const MAX_REQUEST_BYTES: usize = 1024 * 1024;
pub const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_SEARCH_RESULTS: usize = 250;
pub const MAX_SEARCH_QUERY_CHARS: usize = 256;
pub const DEFAULT_CLEANUP_RESULTS: usize = 20;
pub const MAX_CLEANUP_RESULTS: usize = 50;
pub const CLEANUP_ID_BYTES: usize = 16;
pub const OPERATION_ID_BYTES: usize = 16;
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_DESCRIPTOR_BYTES: u64 = 64 * 1024;
const DESCRIPTOR_FILE_NAME: &str = "control-v1.json";
const LOCK_FILE_NAME: &str = "control-v1.lock";

pub struct ControlInstanceLock {
    _file: File,
}

impl fmt::Debug for ControlInstanceLock {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ControlInstanceLock")
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EndpointDescriptor {
    pub protocol_version: u16,
    pub transport: ControlTransport,
    pub address: String,
    pub token: String,
    pub process_id: u32,
    pub started_at_unix_ms: u64,
}

impl fmt::Debug for EndpointDescriptor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EndpointDescriptor")
            .field("protocol_version", &self.protocol_version)
            .field("transport", &self.transport)
            .field("address", &self.address)
            .field("token", &"[redacted]")
            .field("process_id", &self.process_id)
            .field("started_at_unix_ms", &self.started_at_unix_ms)
            .finish()
    }
}

impl EndpointDescriptor {
    pub fn new_loopback(address: SocketAddr, token: String) -> Result<Self, ProtocolError> {
        if !address.ip().is_loopback() {
            return Err(ProtocolError::UnsafeEndpoint);
        }
        if token.len() < 32 {
            return Err(ProtocolError::InvalidToken);
        }

        Ok(Self {
            protocol_version: PROTOCOL_VERSION,
            transport: ControlTransport::LoopbackTcp,
            address: address.to_string(),
            token,
            process_id: std::process::id(),
            started_at_unix_ms: unix_time_ms(),
        })
    }

    pub fn socket_addr(&self) -> Result<SocketAddr, ProtocolError> {
        if self.protocol_version != PROTOCOL_VERSION {
            return Err(ProtocolError::VersionMismatch {
                expected: PROTOCOL_VERSION,
                actual: self.protocol_version,
            });
        }
        if self.transport != ControlTransport::LoopbackTcp {
            return Err(ProtocolError::UnsupportedTransport);
        }
        if self.token.len() < 32 {
            return Err(ProtocolError::InvalidToken);
        }
        let address: SocketAddr = self
            .address
            .parse()
            .map_err(|_| ProtocolError::InvalidEndpoint)?;
        if !address.ip().is_loopback() {
            return Err(ProtocolError::UnsafeEndpoint);
        }
        Ok(address)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ControlTransport {
    LoopbackTcp,
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ControlRequest {
    pub protocol_version: u16,
    pub request_id: String,
    pub token: String,
    pub command: ControlCommand,
}

impl fmt::Debug for ControlRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ControlRequest")
            .field("protocol_version", &self.protocol_version)
            .field("request_id", &self.request_id)
            .field("token", &"[redacted]")
            .field("command", &self.command)
            .finish()
    }
}

impl ControlRequest {
    pub fn new(token: String, command: ControlCommand) -> Result<Self, ProtocolError> {
        Ok(Self {
            protocol_version: PROTOCOL_VERSION,
            request_id: random_hex(16)?,
            token,
            command,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "method", content = "params", rename_all = "snake_case")]
pub enum ControlCommand {
    AppStatus,
    SystemOverview,
    SearchFiles(FileSearchRequest),
    SearchDocuments(DocumentSearchRequest),
    StartStorageScan,
    OperationStatus(OperationReference),
    CancelOperation(OperationReference),
    CleanupCandidates(CleanupCandidatesRequest),
    CreateCleanupPlan(CreateCleanupPlanRequest),
    CleanupPlanStatus(CleanupPlanReference),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CleanupSource {
    DuplicateFiles,
    SystemCleanup,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CleanupCandidatesRequest {
    pub source: CleanupSource,
    #[serde(default)]
    pub expected_generation: Option<u64>,
    #[serde(default)]
    pub offset: usize,
    #[serde(default = "default_cleanup_results")]
    pub max_results: usize,
}

impl CleanupCandidatesRequest {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.expected_generation == Some(0) {
            return Err(ProtocolError::InvalidRequest(
                "검사 세대 번호는 1 이상이어야 합니다".to_owned(),
            ));
        }
        validate_cleanup_result_count(self.max_results)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateCleanupPlanRequest {
    pub source: CleanupSource,
    pub source_generation: u64,
    pub candidate_ids: Vec<String>,
}

impl CreateCleanupPlanRequest {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.source_generation == 0 {
            return Err(ProtocolError::InvalidRequest(
                "검사 세대 번호는 1 이상이어야 합니다".to_owned(),
            ));
        }
        if self.candidate_ids.is_empty() || self.candidate_ids.len() > MAX_CLEANUP_RESULTS {
            return Err(ProtocolError::InvalidRequest(format!(
                "정리 후보 수는 1에서 {MAX_CLEANUP_RESULTS} 사이여야 합니다"
            )));
        }
        for (index, candidate_id) in self.candidate_ids.iter().enumerate() {
            validate_cleanup_id(candidate_id, "정리 후보 번호")?;
            if self.candidate_ids[..index]
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(candidate_id))
            {
                return Err(ProtocolError::InvalidRequest(
                    "정리 후보 번호는 중복될 수 없습니다".to_owned(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CleanupPlanReference {
    pub plan_id: String,
}

impl CleanupPlanReference {
    pub fn new(plan_id: impl Into<String>) -> Result<Self, ProtocolError> {
        let reference = Self {
            plan_id: plan_id.into(),
        };
        reference.validate()?;
        Ok(reference)
    }

    pub fn validate(&self) -> Result<(), ProtocolError> {
        validate_cleanup_id(&self.plan_id, "정리 계획 번호")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OperationReference {
    pub operation_id: String,
}

impl OperationReference {
    pub fn new(operation_id: impl Into<String>) -> Result<Self, ProtocolError> {
        let reference = Self {
            operation_id: operation_id.into(),
        };
        reference.validate()?;
        Ok(reference)
    }

    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.operation_id.len() != OPERATION_ID_BYTES * 2
            || !self
                .operation_id
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(ProtocolError::InvalidRequest(
                "작업 번호가 올바르지 않습니다".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ControlOperationSource {
    App,
    ChatCli,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ControlOperationState {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StorageScanSummary {
    pub root: String,
    pub completed_at_unix_ms: u64,
    pub duration_ms: u64,
    pub total_files: u64,
    pub total_logical_bytes: u64,
    pub large_file_count: u64,
    pub duplicate_group_count: u64,
    pub duplicate_waste_bytes: u64,
    pub unreadable_entries: u64,
    pub issue_count: u64,
    pub candidate_limit_reached: bool,
    pub hard_link_identity_limit_reached: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ControlOperationStatus {
    pub operation_id: String,
    pub kind: String,
    pub source: ControlOperationSource,
    pub state: ControlOperationState,
    pub cancellation_requested: bool,
    pub message: Option<String>,
    pub processed_items: Option<u64>,
    pub processed_bytes: Option<u64>,
    pub started_at_unix_ms: u64,
    pub finished_at_unix_ms: Option<u64>,
    pub scan_generation: Option<u64>,
    pub summary: Option<StorageScanSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileSearchRequest {
    pub query: String,
    #[serde(default)]
    pub kind: Option<FileEntryKind>,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub min_bytes: Option<u64>,
    #[serde(default)]
    pub max_bytes: Option<u64>,
    #[serde(default)]
    pub timezone_offset_minutes: i32,
    #[serde(default)]
    pub sort: FileSearchSort,
    #[serde(default = "default_search_results")]
    pub max_results: usize,
}

impl FileSearchRequest {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        validate_query(&self.query, self.max_results)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FileEntryKind {
    File,
    Directory,
    Symlink,
    Other,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FileSearchSort {
    #[default]
    Relevance,
    Name,
    Largest,
    Modified,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSearchRequest {
    pub query: String,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default = "default_search_results")]
    pub max_results: usize,
}

impl DocumentSearchRequest {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        validate_query(&self.query, self.max_results)
    }
}

fn default_search_results() -> usize {
    100
}

const fn default_cleanup_results() -> usize {
    DEFAULT_CLEANUP_RESULTS
}

fn validate_cleanup_result_count(max_results: usize) -> Result<(), ProtocolError> {
    if max_results == 0 || max_results > MAX_CLEANUP_RESULTS {
        return Err(ProtocolError::InvalidRequest(format!(
            "정리 후보 수는 1에서 {MAX_CLEANUP_RESULTS} 사이여야 합니다"
        )));
    }
    Ok(())
}

fn validate_cleanup_id(value: &str, label: &str) -> Result<(), ProtocolError> {
    if value.len() != CLEANUP_ID_BYTES * 2 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ProtocolError::InvalidRequest(format!(
            "{label}가 올바르지 않습니다"
        )));
    }
    Ok(())
}

fn validate_query(query: &str, max_results: usize) -> Result<(), ProtocolError> {
    if query.trim().is_empty() {
        return Err(ProtocolError::InvalidRequest(
            "검색어를 입력해야 합니다".to_owned(),
        ));
    }
    if query.chars().count() > MAX_SEARCH_QUERY_CHARS {
        return Err(ProtocolError::InvalidRequest(
            "검색어가 너무 깁니다".to_owned(),
        ));
    }
    if max_results == 0 || max_results > MAX_SEARCH_RESULTS {
        return Err(ProtocolError::InvalidRequest(format!(
            "검색 결과 수는 1에서 {MAX_SEARCH_RESULTS} 사이여야 합니다"
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ControlResponse {
    Ok {
        protocol_version: u16,
        request_id: String,
        result: Value,
    },
    Error {
        protocol_version: u16,
        request_id: String,
        error: ControlErrorBody,
    },
}

impl ControlResponse {
    pub fn ok(request_id: impl Into<String>, result: Value) -> Self {
        Self::Ok {
            protocol_version: PROTOCOL_VERSION,
            request_id: request_id.into(),
            result,
        }
    }

    pub fn error(
        request_id: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
        retryable: bool,
    ) -> Self {
        Self::Error {
            protocol_version: PROTOCOL_VERSION,
            request_id: request_id.into(),
            error: ControlErrorBody {
                code: code.into(),
                message: message.into(),
                retryable,
            },
        }
    }

    pub fn request_id(&self) -> &str {
        match self {
            Self::Ok { request_id, .. } | Self::Error { request_id, .. } => request_id,
        }
    }

    pub fn into_result(self) -> Result<Value, RemoteError> {
        match self {
            Self::Ok {
                protocol_version,
                result,
                ..
            } => {
                ensure_version(protocol_version)?;
                Ok(result)
            }
            Self::Error {
                protocol_version,
                error,
                ..
            } => {
                ensure_version(protocol_version)?;
                Err(RemoteError(error))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ControlErrorBody {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

impl fmt::Display for ControlErrorBody {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("{0}")]
pub struct RemoteError(pub ControlErrorBody);

impl RemoteError {
    pub fn code(&self) -> &str {
        &self.0.code
    }
}

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("BroomSweepy 앱이 실행 중이지 않거나 연결 정보를 찾을 수 없습니다")]
    AppNotRunning,
    #[error("로컬 연결 파일을 읽을 수 없습니다: {0}")]
    DescriptorIo(#[source] io::Error),
    #[error("로컬 연결 파일이 올바르지 않습니다: {0}")]
    DescriptorFormat(#[from] serde_json::Error),
    #[error("지원하지 않는 연결 방식입니다")]
    UnsupportedTransport,
    #[error("로컬 연결 주소가 올바르지 않습니다")]
    InvalidEndpoint,
    #[error("로컬이 아닌 연결 주소는 사용할 수 없습니다")]
    UnsafeEndpoint,
    #[error("연결 토큰이 올바르지 않습니다")]
    InvalidToken,
    #[error("BroomSweepy가 이미 실행 중입니다")]
    AlreadyRunning,
    #[error("제어 규격 버전이 다릅니다: 예상 {expected}, 실제 {actual}")]
    VersionMismatch { expected: u16, actual: u16 },
    #[error("요청이 올바르지 않습니다: {0}")]
    InvalidRequest(String),
    #[error("메시지가 허용 크기 {limit}바이트를 초과했습니다")]
    FrameTooLarge { limit: usize },
    #[error("로컬 연결에 실패했습니다: {0}")]
    Connection(#[source] io::Error),
    #[error(transparent)]
    Remote(#[from] RemoteError),
}

impl ProtocolError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::AppNotRunning => "app_not_running",
            Self::DescriptorIo(_) | Self::DescriptorFormat(_) => "invalid_descriptor",
            Self::UnsupportedTransport | Self::InvalidEndpoint | Self::UnsafeEndpoint => {
                "invalid_endpoint"
            }
            Self::InvalidToken => "authentication_failed",
            Self::AlreadyRunning => "app_already_running",
            Self::VersionMismatch { .. } => "version_mismatch",
            Self::InvalidRequest(_) => "invalid_request",
            Self::FrameTooLarge { .. } => "frame_too_large",
            Self::Connection(_) => "connection_failed",
            Self::Remote(_) => "remote_error",
        }
    }
}

fn ensure_version(actual: u16) -> Result<(), RemoteError> {
    if actual == PROTOCOL_VERSION {
        Ok(())
    } else {
        Err(RemoteError(ControlErrorBody {
            code: "version_mismatch".to_owned(),
            message: format!("제어 규격 버전이 다릅니다: 예상 {PROTOCOL_VERSION}, 실제 {actual}"),
            retryable: false,
        }))
    }
}

pub fn default_descriptor_path() -> Result<PathBuf, ProtocolError> {
    default_control_directory().map(|root| root.join(DESCRIPTOR_FILE_NAME))
}

pub fn default_lock_path() -> Result<PathBuf, ProtocolError> {
    default_control_directory().map(|root| root.join(LOCK_FILE_NAME))
}

fn default_control_directory() -> Result<PathBuf, ProtocolError> {
    dirs::data_local_dir()
        .map(|root| root.join("BroomSweepy"))
        .ok_or_else(|| {
            ProtocolError::DescriptorIo(io::Error::new(
                io::ErrorKind::NotFound,
                "사용자 데이터 폴더를 찾지 못했습니다",
            ))
        })
}

pub fn acquire_instance_lock(path: &Path) -> Result<ControlInstanceLock, ProtocolError> {
    let parent = path.parent().ok_or_else(|| {
        ProtocolError::DescriptorIo(io::Error::new(
            io::ErrorKind::InvalidInput,
            "잠금 파일의 상위 폴더가 없습니다",
        ))
    })?;
    fs::create_dir_all(parent).map_err(ProtocolError::DescriptorIo)?;
    set_private_directory_permissions(parent)?;
    reject_unsafe_existing_path(path)?;

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(ProtocolError::DescriptorIo)?;
    set_private_file_permissions(path)?;
    match file.try_lock() {
        Ok(()) => Ok(ControlInstanceLock { _file: file }),
        Err(TryLockError::WouldBlock) => Err(ProtocolError::AlreadyRunning),
        Err(TryLockError::Error(error)) => Err(ProtocolError::DescriptorIo(error)),
    }
}

pub fn write_descriptor(path: &Path, descriptor: &EndpointDescriptor) -> Result<(), ProtocolError> {
    descriptor.socket_addr()?;
    let parent = path.parent().ok_or_else(|| {
        ProtocolError::DescriptorIo(io::Error::new(
            io::ErrorKind::InvalidInput,
            "연결 파일의 상위 폴더가 없습니다",
        ))
    })?;
    fs::create_dir_all(parent).map_err(ProtocolError::DescriptorIo)?;
    set_private_directory_permissions(parent)?;
    reject_unsafe_existing_path(path)?;

    let temporary_path = parent.join(format!(
        ".{DESCRIPTOR_FILE_NAME}.{}.{}.tmp",
        std::process::id(),
        unix_time_ms()
    ));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    let mut file = options
        .open(&temporary_path)
        .map_err(ProtocolError::DescriptorIo)?;
    set_private_file_permissions(&temporary_path)?;
    let payload = serde_json::to_vec(descriptor)?;
    file.write_all(&payload)
        .and_then(|_| file.sync_all())
        .map_err(ProtocolError::DescriptorIo)?;
    drop(file);

    if path.exists() {
        fs::remove_file(path).map_err(ProtocolError::DescriptorIo)?;
    }
    if let Err(error) = fs::rename(&temporary_path, path) {
        let _ = fs::remove_file(&temporary_path);
        return Err(ProtocolError::DescriptorIo(error));
    }
    set_private_file_permissions(path)
}

pub fn read_descriptor(path: &Path) -> Result<EndpointDescriptor, ProtocolError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            ProtocolError::AppNotRunning
        } else {
            ProtocolError::DescriptorIo(error)
        }
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(ProtocolError::DescriptorIo(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "연결 파일은 일반 파일이어야 합니다",
        )));
    }
    if metadata.len() > MAX_DESCRIPTOR_BYTES {
        return Err(ProtocolError::FrameTooLarge {
            limit: MAX_DESCRIPTOR_BYTES as usize,
        });
    }
    let descriptor: EndpointDescriptor =
        serde_json::from_reader(File::open(path).map_err(ProtocolError::DescriptorIo)?)?;
    descriptor.socket_addr()?;
    Ok(descriptor)
}

pub fn remove_descriptor_if_owned(
    path: &Path,
    descriptor: &EndpointDescriptor,
) -> Result<bool, ProtocolError> {
    let current = match read_descriptor(path) {
        Ok(current) => current,
        Err(ProtocolError::AppNotRunning) => return Ok(false),
        Err(error) => return Err(error),
    };
    if current.process_id != descriptor.process_id || current.token != descriptor.token {
        return Ok(false);
    }
    fs::remove_file(path).map_err(ProtocolError::DescriptorIo)?;
    Ok(true)
}

pub fn random_token() -> Result<String, ProtocolError> {
    random_hex(32)
}

pub fn random_operation_id() -> Result<String, ProtocolError> {
    random_hex(OPERATION_ID_BYTES)
}

fn random_hex(bytes: usize) -> Result<String, ProtocolError> {
    let mut random = vec![0_u8; bytes];
    getrandom::fill(&mut random).map_err(|error| {
        ProtocolError::Connection(io::Error::other(format!(
            "안전한 난수를 만들지 못했습니다: {error}"
        )))
    })?;
    let mut output = String::with_capacity(bytes * 2);
    for byte in random {
        use fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    Ok(output)
}

pub fn call(command: ControlCommand, timeout: Duration) -> Result<Value, ProtocolError> {
    let path = default_descriptor_path()?;
    let descriptor = read_descriptor(&path)?;
    let address = descriptor.socket_addr()?;
    let mut stream = TcpStream::connect_timeout(&address, timeout)
        .map_err(|error| map_connection_error(error, &path))?;
    stream
        .set_read_timeout(Some(timeout))
        .and_then(|_| stream.set_write_timeout(Some(timeout)))
        .map_err(ProtocolError::Connection)?;
    let request = ControlRequest::new(descriptor.token, command)?;
    write_json_frame(&mut stream, &request, MAX_REQUEST_BYTES)?;
    let response: ControlResponse = read_json_frame(&mut stream, MAX_RESPONSE_BYTES)?;
    if response.request_id() != request.request_id {
        return Err(ProtocolError::Connection(io::Error::new(
            io::ErrorKind::InvalidData,
            "응답의 요청 번호가 일치하지 않습니다",
        )));
    }
    response.into_result().map_err(ProtocolError::Remote)
}

fn map_connection_error(error: io::Error, descriptor_path: &Path) -> ProtocolError {
    if matches!(
        error.kind(),
        io::ErrorKind::ConnectionRefused
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionReset
            | io::ErrorKind::NotFound
            | io::ErrorKind::TimedOut
    ) {
        let _ = reject_unsafe_existing_path(descriptor_path);
        ProtocolError::AppNotRunning
    } else {
        ProtocolError::Connection(error)
    }
}

pub fn write_json_frame<W: Write, T: Serialize>(
    writer: &mut W,
    value: &T,
    limit: usize,
) -> Result<(), ProtocolError> {
    let payload = serde_json::to_vec(value)?;
    if payload.len() > limit || payload.len() > u32::MAX as usize {
        return Err(ProtocolError::FrameTooLarge { limit });
    }
    writer
        .write_all(&(payload.len() as u32).to_be_bytes())
        .and_then(|_| writer.write_all(&payload))
        .and_then(|_| writer.flush())
        .map_err(ProtocolError::Connection)
}

pub fn read_json_frame<R: Read, T: DeserializeOwned>(
    reader: &mut R,
    limit: usize,
) -> Result<T, ProtocolError> {
    let mut length_bytes = [0_u8; 4];
    reader
        .read_exact(&mut length_bytes)
        .map_err(ProtocolError::Connection)?;
    let length = u32::from_be_bytes(length_bytes) as usize;
    if length > limit {
        return Err(ProtocolError::FrameTooLarge { limit });
    }
    let mut payload = vec![0_u8; length];
    reader
        .read_exact(&mut payload)
        .map_err(ProtocolError::Connection)?;
    serde_json::from_slice(&payload).map_err(ProtocolError::DescriptorFormat)
}

fn reject_unsafe_existing_path(path: &Path) -> Result<(), ProtocolError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(ProtocolError::DescriptorIo(error)),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(ProtocolError::DescriptorIo(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "안전하지 않은 연결 파일 경로입니다",
        )));
    }
    Ok(())
}

#[cfg(unix)]
fn set_private_directory_permissions(path: &Path) -> Result<(), ProtocolError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(ProtocolError::DescriptorIo)
}

#[cfg(not(unix))]
fn set_private_directory_permissions(_path: &Path) -> Result<(), ProtocolError> {
    Ok(())
}

#[cfg(unix)]
fn set_private_file_permissions(path: &Path) -> Result<(), ProtocolError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(ProtocolError::DescriptorIo)
}

#[cfg(not(unix))]
fn set_private_file_permissions(_path: &Path) -> Result<(), ProtocolError> {
    Ok(())
}

pub fn is_loopback_address(address: IpAddr) -> bool {
    address.is_loopback()
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn frame_round_trip_preserves_request() {
        let request = ControlRequest {
            protocol_version: PROTOCOL_VERSION,
            request_id: "request-1".to_owned(),
            token: "a".repeat(64),
            command: ControlCommand::SearchDocuments(DocumentSearchRequest {
                query: "계약서".to_owned(),
                extensions: vec!["pdf".to_owned()],
                max_results: 20,
            }),
        };
        let mut frame = Vec::new();
        write_json_frame(&mut frame, &request, MAX_REQUEST_BYTES).expect("write request frame");
        let decoded: ControlRequest = read_json_frame(&mut Cursor::new(frame), MAX_REQUEST_BYTES)
            .expect("read request frame");
        assert_eq!(decoded, request);
    }

    #[test]
    fn oversized_frame_is_rejected_before_payload_allocation() {
        let mut frame = Cursor::new(((MAX_REQUEST_BYTES as u32) + 1).to_be_bytes());
        let error = read_json_frame::<_, ControlRequest>(&mut frame, MAX_REQUEST_BYTES)
            .expect_err("oversized frame must fail");
        assert!(matches!(error, ProtocolError::FrameTooLarge { .. }));
    }

    #[test]
    fn descriptor_rejects_non_loopback_address() {
        let error = EndpointDescriptor::new_loopback(
            "192.0.2.10:1234".parse().expect("test address"),
            "b".repeat(64),
        )
        .expect_err("non-loopback address must fail");
        assert!(matches!(error, ProtocolError::UnsafeEndpoint));
    }

    #[test]
    fn descriptor_debug_redacts_token() {
        let descriptor = EndpointDescriptor::new_loopback(
            "127.0.0.1:1234".parse().expect("loopback address"),
            "secret-token-that-must-not-be-logged-000000".to_owned(),
        )
        .expect("descriptor");
        let debug = format!("{descriptor:?}");
        assert!(debug.contains("[redacted]"));
        assert!(!debug.contains("secret-token"));
    }

    #[test]
    fn request_debug_redacts_token() {
        let request = ControlRequest {
            protocol_version: PROTOCOL_VERSION,
            request_id: "request-2".to_owned(),
            token: "request-secret-token-that-must-not-be-logged".to_owned(),
            command: ControlCommand::AppStatus,
        };
        let debug = format!("{request:?}");
        assert!(debug.contains("[redacted]"));
        assert!(!debug.contains("request-secret-token"));
    }

    #[test]
    fn search_request_enforces_result_limit() {
        let request = DocumentSearchRequest {
            query: "report".to_owned(),
            extensions: Vec::new(),
            max_results: MAX_SEARCH_RESULTS + 1,
        };
        assert!(matches!(
            request.validate(),
            Err(ProtocolError::InvalidRequest(_))
        ));
    }

    #[test]
    fn search_request_uses_the_same_query_limit_as_the_app_index() {
        let request = DocumentSearchRequest {
            query: "가".repeat(MAX_SEARCH_QUERY_CHARS + 1),
            extensions: Vec::new(),
            max_results: 10,
        };

        assert!(matches!(
            request.validate(),
            Err(ProtocolError::InvalidRequest(_))
        ));
    }

    #[test]
    fn storage_scan_commands_have_stable_wire_names() {
        let start =
            serde_json::to_value(ControlCommand::StartStorageScan).expect("serialize start");
        assert_eq!(start, serde_json::json!({ "method": "start_storage_scan" }));

        let reference =
            OperationReference::new("ab".repeat(OPERATION_ID_BYTES)).expect("operation reference");
        let status = serde_json::to_value(ControlCommand::OperationStatus(reference.clone()))
            .expect("serialize status");
        let cancel = serde_json::to_value(ControlCommand::CancelOperation(reference))
            .expect("serialize cancel");
        assert_eq!(
            status,
            serde_json::json!({
                "method": "operation_status",
                "params": { "operationId": "ab".repeat(OPERATION_ID_BYTES) }
            })
        );
        assert_eq!(
            cancel,
            serde_json::json!({
                "method": "cancel_operation",
                "params": { "operationId": "ab".repeat(OPERATION_ID_BYTES) }
            })
        );
    }

    #[test]
    fn cleanup_commands_have_stable_v3_wire_contract() {
        assert_eq!(PROTOCOL_VERSION, 3);

        let candidates = serde_json::to_value(ControlCommand::CleanupCandidates(
            CleanupCandidatesRequest {
                source: CleanupSource::SystemCleanup,
                expected_generation: Some(7),
                offset: 20,
                max_results: DEFAULT_CLEANUP_RESULTS,
            },
        ))
        .expect("serialize cleanup candidates");
        assert_eq!(
            candidates,
            serde_json::json!({
                "method": "cleanup_candidates",
                "params": {
                    "source": "system_cleanup",
                    "expectedGeneration": 7,
                    "offset": 20,
                    "maxResults": 20
                }
            })
        );

        let candidate_id = "ab".repeat(CLEANUP_ID_BYTES);
        let create = serde_json::to_value(ControlCommand::CreateCleanupPlan(
            CreateCleanupPlanRequest {
                source: CleanupSource::DuplicateFiles,
                source_generation: 9,
                candidate_ids: vec![candidate_id.clone()],
            },
        ))
        .expect("serialize cleanup plan");
        assert_eq!(
            create,
            serde_json::json!({
                "method": "create_cleanup_plan",
                "params": {
                    "source": "duplicate_files",
                    "sourceGeneration": 9,
                    "candidateIds": [candidate_id]
                }
            })
        );

        let plan_id = "cd".repeat(CLEANUP_ID_BYTES);
        let status = serde_json::to_value(ControlCommand::CleanupPlanStatus(
            CleanupPlanReference::new(plan_id.clone()).expect("plan reference"),
        ))
        .expect("serialize cleanup plan status");
        assert_eq!(
            status,
            serde_json::json!({
                "method": "cleanup_plan_status",
                "params": { "planId": plan_id }
            })
        );

        let request = ControlRequest::new("f".repeat(64), ControlCommand::AppStatus)
            .expect("control request");
        let request_json = serde_json::to_value(request).expect("serialize control request");
        assert_eq!(request_json["protocolVersion"], 3);
    }

    #[test]
    fn cleanup_candidate_request_defaults_and_limits_are_bounded() {
        let command = serde_json::from_value::<ControlCommand>(serde_json::json!({
            "method": "cleanup_candidates",
            "params": { "source": "system_cleanup" }
        }))
        .expect("default cleanup request");
        let ControlCommand::CleanupCandidates(request) = command else {
            panic!("cleanup candidates command expected");
        };
        assert_eq!(request.max_results, DEFAULT_CLEANUP_RESULTS);
        assert_eq!(request.offset, 0);
        assert_eq!(request.expected_generation, None);
        request.validate().expect("default request is valid");

        for max_results in [0, MAX_CLEANUP_RESULTS + 1] {
            let request = CleanupCandidatesRequest {
                source: CleanupSource::SystemCleanup,
                expected_generation: None,
                offset: 0,
                max_results,
            };
            assert!(matches!(
                request.validate(),
                Err(ProtocolError::InvalidRequest(_))
            ));
        }
    }

    #[test]
    fn cleanup_plan_requests_enforce_fixed_hex_ids_and_candidate_bounds() {
        let valid_id = "ab".repeat(CLEANUP_ID_BYTES);
        let valid = CreateCleanupPlanRequest {
            source: CleanupSource::DuplicateFiles,
            source_generation: 1,
            candidate_ids: vec![valid_id.clone()],
        };
        valid.validate().expect("valid cleanup plan");
        CleanupPlanReference::new(valid_id.clone()).expect("valid plan reference");

        for invalid_id in ["short".to_owned(), "zz".repeat(CLEANUP_ID_BYTES)] {
            let request = CreateCleanupPlanRequest {
                source: CleanupSource::SystemCleanup,
                source_generation: 1,
                candidate_ids: vec![invalid_id.clone()],
            };
            assert!(request.validate().is_err());
            assert!(CleanupPlanReference::new(invalid_id).is_err());
        }

        let empty = CreateCleanupPlanRequest {
            source: CleanupSource::SystemCleanup,
            source_generation: 1,
            candidate_ids: Vec::new(),
        };
        assert!(empty.validate().is_err());

        let too_many = CreateCleanupPlanRequest {
            source: CleanupSource::SystemCleanup,
            source_generation: 1,
            candidate_ids: (0..=MAX_CLEANUP_RESULTS)
                .map(|index| format!("{index:032x}"))
                .collect(),
        };
        assert!(too_many.validate().is_err());

        let duplicate = CreateCleanupPlanRequest {
            source: CleanupSource::SystemCleanup,
            source_generation: 1,
            candidate_ids: vec![valid_id.to_ascii_lowercase(), valid_id.to_ascii_uppercase()],
        };
        assert!(duplicate.validate().is_err());
    }

    #[test]
    fn cleanup_request_dtos_reject_unknown_raw_identity_fields() {
        let candidates = serde_json::from_value::<ControlCommand>(serde_json::json!({
            "method": "cleanup_candidates",
            "params": {
                "source": "system_cleanup",
                "path": "C:\\untrusted"
            }
        }));
        assert!(candidates.is_err());

        let create = serde_json::from_value::<ControlCommand>(serde_json::json!({
            "method": "create_cleanup_plan",
            "params": {
                "source": "duplicate_files",
                "sourceGeneration": 1,
                "candidateIds": ["ab".repeat(CLEANUP_ID_BYTES)],
                "name": "untrusted"
            }
        }));
        assert!(create.is_err());

        let status = serde_json::from_value::<ControlCommand>(serde_json::json!({
            "method": "cleanup_plan_status",
            "params": {
                "planId": "cd".repeat(CLEANUP_ID_BYTES),
                "hash": "untrusted"
            }
        }));
        assert!(status.is_err());
    }

    #[test]
    fn operation_reference_rejects_untrusted_identifiers() {
        assert!(OperationReference::new("short").is_err());
        assert!(OperationReference::new("zz".repeat(OPERATION_ID_BYTES)).is_err());
        assert!(OperationReference::new("ab".repeat(OPERATION_ID_BYTES)).is_ok());
    }

    #[test]
    fn storage_scan_rejects_a_caller_supplied_path() {
        let command = serde_json::from_value::<ControlCommand>(serde_json::json!({
            "method": "start_storage_scan",
            "params": { "root": "C:\\not-approved" }
        }));
        assert!(command.is_err());
    }

    #[test]
    fn operation_status_uses_operation_id_and_camel_case_values() {
        let operation = ControlOperationStatus {
            operation_id: "ab".repeat(OPERATION_ID_BYTES),
            kind: "storageScan".to_owned(),
            source: ControlOperationSource::ChatCli,
            state: ControlOperationState::Running,
            cancellation_requested: false,
            message: Some("검사 중".to_owned()),
            processed_items: Some(12),
            processed_bytes: Some(34),
            started_at_unix_ms: 56,
            finished_at_unix_ms: None,
            scan_generation: None,
            summary: None,
        };
        let json = serde_json::to_value(operation).expect("serialize operation");
        assert_eq!(json["operationId"], "ab".repeat(OPERATION_ID_BYTES));
        assert_eq!(json["source"], "chatCli");
        assert_eq!(json["state"], "running");
        assert!(json.get("id").is_none());
    }

    #[test]
    fn descriptor_file_round_trip_and_owned_removal() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("control.json");
        let descriptor = EndpointDescriptor::new_loopback(
            "127.0.0.1:32123".parse().expect("loopback address"),
            "c".repeat(64),
        )
        .expect("descriptor");
        write_descriptor(&path, &descriptor).expect("write descriptor");
        assert_eq!(read_descriptor(&path).expect("read descriptor"), descriptor);
        assert!(remove_descriptor_if_owned(&path, &descriptor).expect("remove descriptor"));
        assert!(!path.exists());
    }

    #[test]
    fn instance_lock_allows_only_one_owner() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("control.lock");
        let first = acquire_instance_lock(&path).expect("first lock");

        assert!(matches!(
            acquire_instance_lock(&path),
            Err(ProtocolError::AlreadyRunning)
        ));

        drop(first);
        acquire_instance_lock(&path).expect("lock after first owner exits");
    }
}
