use bloomsweepy_control::{
    ControlCommand, ControlErrorBody, DEFAULT_TIMEOUT, DocumentSearchRequest, FileEntryKind,
    FileSearchRequest, FileSearchSort, OperationReference, ProtocolError, call,
};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use serde_json::Value;

pub mod mcp;

#[derive(Debug, Parser)]
#[command(
    name = "bloomsweepy-mcp",
    version,
    about = "실행 중인 BroomSweepy 앱에 검사와 검색을 요청합니다"
)]
pub struct Arguments {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// MCP 표준 입출력 서버로 실행합니다.
    Mcp,
    /// 앱 연결 상태와 현재 작업을 확인합니다.
    Status,
    /// 드라이브 사용량 요약을 확인합니다.
    SystemOverview,
    /// 앱이 만들어 둔 파일 목록에서 이름과 경로를 검색합니다.
    SearchFiles {
        query: String,
        #[arg(long, default_value_t = 100)]
        max_results: usize,
        #[arg(long, value_enum)]
        kind: Option<EntryKindArgument>,
        #[arg(long = "extension")]
        extensions: Vec<String>,
        #[arg(long)]
        min_bytes: Option<u64>,
        #[arg(long)]
        max_bytes: Option<u64>,
        #[arg(long, default_value_t = 0)]
        timezone_offset_minutes: i32,
        #[arg(long, value_enum, default_value_t = SortArgument::Relevance)]
        sort: SortArgument,
    },
    /// 앱이 만들어 둔 문서 내용 목록에서 검색합니다.
    SearchDocuments {
        query: String,
        #[arg(long, default_value_t = 100)]
        max_results: usize,
        #[arg(long = "extension")]
        extensions: Vec<String>,
    },
    /// 앱에서 이번 실행에 허용한 폴더 검사를 시작합니다.
    StartScan,
    /// 작업 번호로 검사 진행 상태와 제한된 결과 요약을 확인합니다.
    OperationStatus { operation_id: String },
    /// 작업 번호가 정확히 일치하는 진행 중 검사의 취소를 요청합니다.
    CancelOperation { operation_id: String },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum EntryKindArgument {
    File,
    Directory,
    Symlink,
    Other,
}

impl From<EntryKindArgument> for FileEntryKind {
    fn from(value: EntryKindArgument) -> Self {
        match value {
            EntryKindArgument::File => Self::File,
            EntryKindArgument::Directory => Self::Directory,
            EntryKindArgument::Symlink => Self::Symlink,
            EntryKindArgument::Other => Self::Other,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum SortArgument {
    #[default]
    Relevance,
    Name,
    Largest,
    Modified,
}

impl From<SortArgument> for FileSearchSort {
    fn from(value: SortArgument) -> Self {
        match value {
            SortArgument::Relevance => Self::Relevance,
            SortArgument::Name => Self::Name,
            SortArgument::Largest => Self::Largest,
            SortArgument::Modified => Self::Modified,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum JsonOutput {
    Ok { result: Value },
    Error { error: ControlErrorBody },
}

impl JsonOutput {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Ok { .. } => 0,
            Self::Error { .. } => 2,
        }
    }
}

pub fn execute(command: Command) -> JsonOutput {
    let command = match control_command(command) {
        Ok(command) => command,
        Err(error) => {
            return JsonOutput::Error {
                error: public_error(&error),
            };
        }
    };

    match call(command, DEFAULT_TIMEOUT) {
        Ok(result) => JsonOutput::Ok { result },
        Err(error) => JsonOutput::Error {
            error: public_error(&error),
        },
    }
}

pub fn control_command(command: Command) -> Result<ControlCommand, ProtocolError> {
    match command {
        Command::Mcp => Err(ProtocolError::InvalidRequest(
            "mcp 명령은 표준 입출력 서버로 실행해야 합니다".to_owned(),
        )),
        Command::Status => Ok(ControlCommand::AppStatus),
        Command::SystemOverview => Ok(ControlCommand::SystemOverview),
        Command::SearchFiles {
            query,
            max_results,
            kind,
            extensions,
            min_bytes,
            max_bytes,
            timezone_offset_minutes,
            sort,
        } => {
            let request = FileSearchRequest {
                query,
                kind: kind.map(Into::into),
                extensions,
                min_bytes,
                max_bytes,
                timezone_offset_minutes,
                sort: sort.into(),
                max_results,
            };
            request.validate()?;
            if matches!((request.min_bytes, request.max_bytes), (Some(min), Some(max)) if min > max)
            {
                return Err(ProtocolError::InvalidRequest(
                    "최소 크기는 최대 크기보다 클 수 없습니다".to_owned(),
                ));
            }
            Ok(ControlCommand::SearchFiles(request))
        }
        Command::SearchDocuments {
            query,
            max_results,
            extensions,
        } => {
            let request = DocumentSearchRequest {
                query,
                extensions,
                max_results,
            };
            request.validate()?;
            Ok(ControlCommand::SearchDocuments(request))
        }
        Command::StartScan => Ok(ControlCommand::StartStorageScan),
        Command::OperationStatus { operation_id } => Ok(ControlCommand::OperationStatus(
            OperationReference::new(operation_id)?,
        )),
        Command::CancelOperation { operation_id } => Ok(ControlCommand::CancelOperation(
            OperationReference::new(operation_id)?,
        )),
    }
}

pub fn public_error(error: &ProtocolError) -> ControlErrorBody {
    if let ProtocolError::Remote(remote) = error {
        return remote.0.clone();
    }

    let (message, retryable) = match error {
        ProtocolError::AppNotRunning => ("BroomSweepy를 실행한 뒤 다시 시도하세요.", true),
        ProtocolError::DescriptorIo(_) | ProtocolError::DescriptorFormat(_) => (
            "BroomSweepy 연결 정보를 읽을 수 없습니다. 앱을 다시 실행하세요.",
            true,
        ),
        ProtocolError::UnsupportedTransport
        | ProtocolError::InvalidEndpoint
        | ProtocolError::UnsafeEndpoint
        | ProtocolError::InvalidToken => (
            "BroomSweepy 연결 정보가 올바르지 않습니다. 앱을 다시 실행하세요.",
            true,
        ),
        ProtocolError::VersionMismatch { .. } => {
            ("BroomSweepy 앱과 제어 도구의 버전이 맞지 않습니다.", false)
        }
        ProtocolError::AlreadyRunning => ("BroomSweepy가 이미 실행 중입니다.", false),
        ProtocolError::InvalidRequest(message) => (message.as_str(), false),
        ProtocolError::FrameTooLarge { .. } => ("요청 또는 응답이 허용 크기를 넘었습니다.", false),
        ProtocolError::Connection(_) => ("BroomSweepy와 로컬 통신 중 오류가 발생했습니다.", true),
        ProtocolError::Remote(_) => unreachable!("remote errors returned above"),
    };
    ControlErrorBody {
        code: error.code().to_owned(),
        message: message.to_owned(),
        retryable,
    }
}

#[cfg(test)]
mod tests {
    use bloomsweepy_control::{ControlCommand, MAX_SEARCH_RESULTS};

    use super::*;

    #[test]
    fn every_normal_subcommand_maps_to_an_explicit_control_command() {
        assert!(matches!(
            control_command(Command::Status).expect("status"),
            ControlCommand::AppStatus
        ));
        assert!(matches!(
            control_command(Command::SystemOverview).expect("overview"),
            ControlCommand::SystemOverview
        ));
        assert!(matches!(
            control_command(Command::SearchFiles {
                query: "invoice".to_owned(),
                max_results: 25,
                kind: Some(EntryKindArgument::File),
                extensions: vec!["pdf".to_owned()],
                min_bytes: None,
                max_bytes: None,
                timezone_offset_minutes: 540,
                sort: SortArgument::Relevance,
            })
            .expect("file search"),
            ControlCommand::SearchFiles(_)
        ));
        assert!(matches!(
            control_command(Command::SearchDocuments {
                query: "계약서".to_owned(),
                max_results: 25,
                extensions: vec!["hwpx".to_owned()],
            })
            .expect("document search"),
            ControlCommand::SearchDocuments(_)
        ));
        assert!(matches!(
            control_command(Command::StartScan).expect("start scan"),
            ControlCommand::StartStorageScan
        ));
        let operation_id = "ab".repeat(bloomsweepy_control::OPERATION_ID_BYTES);
        assert!(matches!(
            control_command(Command::OperationStatus {
                operation_id: operation_id.clone(),
            })
            .expect("operation status"),
            ControlCommand::OperationStatus(_)
        ));
        assert!(matches!(
            control_command(Command::CancelOperation { operation_id }).expect("cancel operation"),
            ControlCommand::CancelOperation(_)
        ));
    }

    #[test]
    fn invalid_search_limit_becomes_structured_json_error() {
        let error = control_command(Command::SearchDocuments {
            query: "report".to_owned(),
            max_results: MAX_SEARCH_RESULTS + 1,
            extensions: Vec::new(),
        })
        .expect_err("limit must fail");
        let output = JsonOutput::Error {
            error: public_error(&error),
        };
        let json = serde_json::to_value(output).expect("serialize output");
        assert_eq!(json["status"], "error");
        assert_eq!(json["error"]["code"], "invalid_request");
    }

    #[test]
    fn app_not_running_is_stable_and_retryable() {
        let error = public_error(&ProtocolError::AppNotRunning);
        assert_eq!(error.code, "app_not_running");
        assert!(error.retryable);
    }

    #[test]
    fn public_errors_never_include_endpoint_or_token_details() {
        let endpoint_error = ProtocolError::InvalidToken;
        let body = public_error(&endpoint_error);
        let output = serde_json::to_string(&body).expect("serialize error");
        assert!(!output.contains("token-value"));
        assert!(!output.contains("control-v1.json"));
    }
}
