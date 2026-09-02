use std::error::Error;

use bloomsweepy_control::{
    ControlCommand, DEFAULT_TIMEOUT, DocumentSearchRequest, FileEntryKind, FileSearchRequest,
    FileSearchSort, OperationReference, ProtocolError, call,
};
use rmcp::{
    Json, ServerHandler, ServiceExt, handler::server::wrapper::Parameters, tool, tool_handler,
    tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::public_error;

#[derive(Debug, Clone)]
pub struct BroomSweepyMcp;

#[tool_router]
impl BroomSweepyMcp {
    #[tool(
        name = "status",
        description = "Check whether the BroomSweepy app is connected and inspect its current work status."
    )]
    async fn status(&self) -> Result<Json<Value>, Json<McpToolError>> {
        bridge(ControlCommand::AppStatus).await
    }

    #[tool(
        name = "system_overview",
        description = "Read the storage overview collected by the running BroomSweepy app."
    )]
    async fn system_overview(&self) -> Result<Json<Value>, Json<McpToolError>> {
        bridge(ControlCommand::SystemOverview).await
    }

    #[tool(
        name = "search_files",
        description = "Search the file catalog that the BroomSweepy app has already built. This tool never scans or deletes files itself."
    )]
    async fn search_files(
        &self,
        Parameters(arguments): Parameters<FileSearchArguments>,
    ) -> Result<Json<Value>, Json<McpToolError>> {
        let request = FileSearchRequest {
            query: arguments.query,
            kind: arguments.kind.map(Into::into),
            extensions: arguments.extensions,
            min_bytes: arguments.min_bytes,
            max_bytes: arguments.max_bytes,
            timezone_offset_minutes: arguments.timezone_offset_minutes,
            sort: arguments.sort.into(),
            max_results: arguments.max_results,
        };
        if let Err(error) = request.validate() {
            return Err(Json(McpToolError::from_protocol(&error)));
        }
        if matches!((request.min_bytes, request.max_bytes), (Some(min), Some(max)) if min > max) {
            let error = ProtocolError::InvalidRequest(
                "최소 크기는 최대 크기보다 클 수 없습니다".to_owned(),
            );
            return Err(Json(McpToolError::from_protocol(&error)));
        }
        bridge(ControlCommand::SearchFiles(request)).await
    }

    #[tool(
        name = "search_documents",
        description = "Search document contents through the index owned by the running BroomSweepy app. This tool never reads or deletes documents itself."
    )]
    async fn search_documents(
        &self,
        Parameters(arguments): Parameters<DocumentSearchArguments>,
    ) -> Result<Json<Value>, Json<McpToolError>> {
        let request = DocumentSearchRequest {
            query: arguments.query,
            extensions: arguments.extensions,
            max_results: arguments.max_results,
        };
        if let Err(error) = request.validate() {
            return Err(Json(McpToolError::from_protocol(&error)));
        }
        bridge(ControlCommand::SearchDocuments(request)).await
    }

    #[tool(
        name = "start_storage_scan",
        description = "Ask the running BroomSweepy app to scan the folder and settings that the user explicitly approved in the dashboard for this app run. This tool accepts no path and never scans files itself."
    )]
    async fn start_storage_scan(&self) -> Result<Json<Value>, Json<McpToolError>> {
        bridge(ControlCommand::StartStorageScan).await
    }

    #[tool(
        name = "operation_status",
        description = "Read the progress or bounded result summary for a BroomSweepy operation ID."
    )]
    async fn operation_status(
        &self,
        Parameters(arguments): Parameters<OperationArguments>,
    ) -> Result<Json<Value>, Json<McpToolError>> {
        let reference = OperationReference::new(arguments.operation_id)
            .map_err(|error| Json(McpToolError::from_protocol(&error)))?;
        bridge(ControlCommand::OperationStatus(reference)).await
    }

    #[tool(
        name = "cancel_operation",
        description = "Request cancellation of the running BroomSweepy scan whose operation ID exactly matches. Cancellation is cooperative and must be confirmed with operation_status."
    )]
    async fn cancel_operation(
        &self,
        Parameters(arguments): Parameters<OperationArguments>,
    ) -> Result<Json<Value>, Json<McpToolError>> {
        let reference = OperationReference::new(arguments.operation_id)
            .map_err(|error| Json(McpToolError::from_protocol(&error)))?;
        bridge(ControlCommand::CancelOperation(reference)).await
    }
}

#[tool_handler(
    name = "bloomsweepy",
    version = "1.2.0",
    instructions = "Bridge to the running BroomSweepy app. The bridge interprets structured requests, while the app performs approved scans and searches. No delete or trash tool is exposed."
)]
impl ServerHandler for BroomSweepyMcp {}

pub async fn run_stdio() -> Result<(), Box<dyn Error + Send + Sync>> {
    let service = BroomSweepyMcp.serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;
    Ok(())
}

async fn bridge(command: ControlCommand) -> Result<Json<Value>, Json<McpToolError>> {
    match tokio::task::spawn_blocking(move || call(command, DEFAULT_TIMEOUT)).await {
        Ok(Ok(result)) => Ok(Json(result)),
        Ok(Err(error)) => Err(Json(McpToolError::from_protocol(&error))),
        Err(_) => Err(Json(McpToolError {
            code: "bridge_failed".to_owned(),
            message: "BroomSweepy 연결 작업을 완료하지 못했습니다.".to_owned(),
            retryable: true,
        })),
    }
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct McpToolError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

impl McpToolError {
    fn from_protocol(error: &ProtocolError) -> Self {
        let error = public_error(error);
        Self {
            code: error.code,
            message: error.message,
            retryable: error.retryable,
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct FileSearchArguments {
    #[schemars(description = "Text or BroomSweepy structured query to find in names and paths.")]
    query: String,
    #[serde(default = "default_max_results")]
    #[schemars(description = "Maximum results from 1 through 250.")]
    max_results: usize,
    #[serde(default)]
    kind: Option<EntryKind>,
    #[serde(default)]
    extensions: Vec<String>,
    #[serde(default)]
    min_bytes: Option<u64>,
    #[serde(default)]
    max_bytes: Option<u64>,
    #[serde(default)]
    timezone_offset_minutes: i32,
    #[serde(default)]
    sort: SearchSort,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct DocumentSearchArguments {
    #[schemars(description = "Text to find inside indexed documents.")]
    query: String,
    #[serde(default = "default_max_results")]
    #[schemars(description = "Maximum results from 1 through 250.")]
    max_results: usize,
    #[serde(default)]
    extensions: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct OperationArguments {
    #[schemars(description = "The exact operationId returned by start_storage_scan.")]
    operation_id: String,
}

const fn default_max_results() -> usize {
    100
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum EntryKind {
    File,
    Directory,
    Symlink,
    Other,
}

impl From<EntryKind> for FileEntryKind {
    fn from(value: EntryKind) -> Self {
        match value {
            EntryKind::File => Self::File,
            EntryKind::Directory => Self::Directory,
            EntryKind::Symlink => Self::Symlink,
            EntryKind::Other => Self::Other,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum SearchSort {
    #[default]
    Relevance,
    Name,
    Largest,
    Modified,
}

impl From<SearchSort> for FileSearchSort {
    fn from(value: SearchSort) -> Self {
        match value {
            SearchSort::Relevance => Self::Relevance,
            SearchSort::Name => Self::Name,
            SearchSort::Largest => Self::Largest,
            SearchSort::Modified => Self::Modified,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_the_seven_explicit_tools_without_delete_or_trash() {
        let mut names: Vec<String> = BroomSweepyMcp::tool_router()
            .list_all()
            .iter()
            .map(|tool| tool.name.to_string())
            .collect();
        names.sort();
        assert_eq!(
            names,
            [
                "cancel_operation",
                "operation_status",
                "search_documents",
                "search_files",
                "start_storage_scan",
                "status",
                "system_overview"
            ]
        );
        assert!(
            names
                .iter()
                .all(|name| !name.contains("delete") && !name.contains("trash"))
        );
    }

    #[test]
    fn app_not_running_tool_error_is_structured_and_safe() {
        let error = McpToolError::from_protocol(&ProtocolError::AppNotRunning);
        let output = serde_json::to_string(&error).expect("serialize tool error");
        assert!(output.contains("app_not_running"));
        assert!(!output.contains("control-v1.json"));
        assert!(!output.contains("token"));
    }
}
