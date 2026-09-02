use std::error::Error;

use bloomsweepy_control::{
    CleanupCandidatesRequest, CleanupPlanReference, CleanupSource, ControlCommand,
    CreateCleanupPlanRequest, DEFAULT_CLEANUP_RESULTS, DEFAULT_TIMEOUT, DocumentSearchRequest,
    FileEntryKind, FileSearchRequest, FileSearchSort, OperationReference, ProtocolError, call,
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

    #[tool(
        name = "cleanup_candidates",
        description = "Read a bounded summary of anonymous cleanup candidates from a currently completed BroomSweepy report. BroomSweepy accepts no raw file identity here, and this tool performs no cleanup."
    )]
    async fn cleanup_candidates(
        &self,
        Parameters(arguments): Parameters<CleanupCandidatesArguments>,
    ) -> Result<Json<Value>, Json<McpToolError>> {
        let request = CleanupCandidatesRequest {
            source: arguments.source.into(),
            expected_generation: arguments.expected_generation,
            offset: arguments.offset,
            max_results: arguments.max_results,
        };
        request
            .validate()
            .map_err(|error| Json(McpToolError::from_protocol(&error)))?;
        bridge(ControlCommand::CleanupCandidates(request)).await
    }

    #[tool(
        name = "create_cleanup_plan",
        description = "Ask BroomSweepy to prepare a review plan from candidate IDs in its current completed report. Nothing is moved until the user approves the exact plan inside the BroomSweepy app."
    )]
    async fn create_cleanup_plan(
        &self,
        Parameters(arguments): Parameters<CreateCleanupPlanArguments>,
    ) -> Result<Json<Value>, Json<McpToolError>> {
        let request = CreateCleanupPlanRequest {
            source: arguments.source.into(),
            source_generation: arguments.source_generation,
            candidate_ids: arguments.candidate_ids,
        };
        request
            .validate()
            .map_err(|error| Json(McpToolError::from_protocol(&error)))?;
        bridge(ControlCommand::CreateCleanupPlan(request)).await
    }

    #[tool(
        name = "cleanup_plan_status",
        description = "Read the bounded summary status of a BroomSweepy cleanup review plan. Any file operation happens only after final user approval inside the BroomSweepy app."
    )]
    async fn cleanup_plan_status(
        &self,
        Parameters(arguments): Parameters<CleanupPlanStatusArguments>,
    ) -> Result<Json<Value>, Json<McpToolError>> {
        let reference = CleanupPlanReference::new(arguments.plan_id)
            .map_err(|error| Json(McpToolError::from_protocol(&error)))?;
        bridge(ControlCommand::CleanupPlanStatus(reference)).await
    }
}

#[tool_handler(
    name = "bloomsweepy",
    version = "1.5.0",
    instructions = "Bridge to the running BroomSweepy app. Cleanup tools return bounded summaries from currently completed reports and can prepare a review plan, but file operations happen only after final user approval inside the app. No approval or direct cleanup tool is exposed."
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

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CleanupCandidatesArguments {
    source: CleanupSourceArgument,
    #[serde(default)]
    expected_generation: Option<u64>,
    #[serde(default)]
    offset: usize,
    #[serde(default = "default_cleanup_results")]
    #[schemars(description = "Maximum candidate summaries from 1 through 50.")]
    max_results: usize,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CreateCleanupPlanArguments {
    source: CleanupSourceArgument,
    source_generation: u64,
    #[schemars(description = "One through 50 opaque 32-character candidate IDs.")]
    candidate_ids: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CleanupPlanStatusArguments {
    #[schemars(description = "The exact opaque 32-character plan ID returned by BroomSweepy.")]
    plan_id: String,
}

const fn default_max_results() -> usize {
    100
}

const fn default_cleanup_results() -> usize {
    DEFAULT_CLEANUP_RESULTS
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum CleanupSourceArgument {
    DuplicateFiles,
    SystemCleanup,
}

impl From<CleanupSourceArgument> for CleanupSource {
    fn from(value: CleanupSourceArgument) -> Self {
        match value {
            CleanupSourceArgument::DuplicateFiles => Self::DuplicateFiles,
            CleanupSourceArgument::SystemCleanup => Self::SystemCleanup,
        }
    }
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
    fn exposes_exactly_ten_tools_without_direct_cleanup_actions() {
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
                "cleanup_candidates",
                "cleanup_plan_status",
                "create_cleanup_plan",
                "operation_status",
                "search_documents",
                "search_files",
                "start_storage_scan",
                "status",
                "system_overview"
            ]
        );
        let forbidden_actions = ["approve", "execute", "delete", "trash"];
        assert!(names.iter().all(|name| {
            forbidden_actions
                .iter()
                .all(|forbidden| !name.contains(forbidden))
        }));
    }

    #[test]
    fn cleanup_tool_schemas_expose_only_opaque_bounded_inputs() {
        let cleanup_tools = BroomSweepyMcp::tool_router()
            .list_all()
            .into_iter()
            .filter(|tool| {
                matches!(
                    tool.name.as_ref(),
                    "cleanup_candidates" | "create_cleanup_plan" | "cleanup_plan_status"
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(cleanup_tools.len(), 3);

        for tool in cleanup_tools {
            let schema = Value::Object((*tool.input_schema).clone());
            let mut property_names = Vec::new();
            collect_property_names(&schema, &mut property_names);
            for forbidden in ["path", "paths", "name", "names", "hash", "hashes"] {
                assert!(
                    !property_names.iter().any(|name| name == forbidden),
                    "{} unexpectedly exposes {forbidden}",
                    tool.name
                );
            }
            assert_eq!(schema["additionalProperties"], Value::Bool(false));
        }
    }

    #[test]
    fn cleanup_tool_arguments_reject_unknown_identity_fields_and_invalid_bounds() {
        let unknown = serde_json::from_value::<CleanupCandidatesArguments>(serde_json::json!({
            "source": "system_cleanup",
            "path": "C:\\untrusted"
        }));
        assert!(unknown.is_err());

        let bounded = CleanupCandidatesRequest {
            source: CleanupSource::SystemCleanup,
            expected_generation: None,
            offset: 0,
            max_results: bloomsweepy_control::MAX_CLEANUP_RESULTS + 1,
        };
        assert!(bounded.validate().is_err());

        let invalid_id = CreateCleanupPlanRequest {
            source: CleanupSource::DuplicateFiles,
            source_generation: 1,
            candidate_ids: vec!["not-an-id".to_owned()],
        };
        assert!(invalid_id.validate().is_err());
    }

    #[test]
    fn app_not_running_tool_error_is_structured_and_safe() {
        let error = McpToolError::from_protocol(&ProtocolError::AppNotRunning);
        let output = serde_json::to_string(&error).expect("serialize tool error");
        assert!(output.contains("app_not_running"));
        assert!(!output.contains("control-v1.json"));
        assert!(!output.contains("token"));
    }

    fn collect_property_names(value: &Value, output: &mut Vec<String>) {
        match value {
            Value::Object(object) => {
                if let Some(Value::Object(properties)) = object.get("properties") {
                    output.extend(properties.keys().cloned());
                }
                for value in object.values() {
                    collect_property_names(value, output);
                }
            }
            Value::Array(values) => {
                for value in values {
                    collect_property_names(value, output);
                }
            }
            _ => {}
        }
    }
}
