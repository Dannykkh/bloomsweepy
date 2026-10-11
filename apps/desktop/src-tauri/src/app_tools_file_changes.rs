//! Same local review list for native chat and MCP, without disclosing paths.
use bloomsweepy_control::{AppToolResult, AppToolStatus, FileChangeFilter};
use serde_json::{Value, json};
use std::path::Path;
use tauri::AppHandle;

use crate::{app_tools::ToolScope, control_server, file_change_review};

const ID: &str = "files.changes";

pub(crate) fn read(
    app: &AppHandle,
    scope: &ToolScope,
    offset: usize,
    filter: FileChangeFilter,
) -> Result<AppToolResult, String> {
    let external_binding = if scope.is_external() {
        match control_server::tool_file_workspace_binding(app) {
            Ok(binding) => Some(binding),
            Err(_) => return Ok(permission()),
        }
    } else {
        None
    };
    let authorized = external_binding
        .as_ref()
        .map(|binding| binding.root.as_path())
        .or_else(|| scope.root());
    let local = serde_json::to_value(file_change_review::page(
        app,
        Some(offset),
        Some(filter.as_str()),
    )?)
    .map_err(|error| error.to_string())?;
    if !same_root(authorized, local["status"]["config"]["root"].as_str()) {
        return Ok(permission());
    }
    // A monitor preference is not disclosure authority. The selected external
    // scope/epoch is rechecked after copying the bounded page.
    if external_binding
        .as_ref()
        .is_some_and(|binding| !control_server::file_workspace_binding_current(app, binding, false))
    {
        return Ok(permission());
    }
    let current = file_change_review::status(app)?;
    if local["status"]["revision"].as_u64() != Some(current.revision)
        || !same_root(authorized, current.config.root.as_deref())
    {
        return Ok(AppToolResult::with_status(
            ID,
            AppToolStatus::Failed,
            json!({"message":"검토 설정이 변경됐습니다. 현재 목록을 다시 조회해 주세요.","performed":false}),
        ));
    }
    let data = model_page(&local);
    let failed = matches!(data["phase"].as_str(), Some("incomplete" | "error"));
    let mut result = AppToolResult::with_status(
        ID,
        if failed {
            AppToolStatus::Failed
        } else {
            AppToolStatus::Completed
        },
        data,
    )
    .with_presentation(json!({"view":"settings","fileChangeReview":local}));
    result.truncated = result.data["truncated"].as_bool().unwrap_or(false);
    Ok(result)
}

fn same_root(authorized: Option<&Path>, selected: Option<&str>) -> bool {
    matches!((authorized, selected), (Some(root), Some(selected)) if root == Path::new(selected))
}

fn permission() -> AppToolResult {
    AppToolResult::with_status(
        ID,
        AppToolStatus::PermissionRequired,
        json!({"message":"변경 검토 폴더와 대화/앱에서 선택한 검사 폴더가 정확히 같아야 합니다. 감시 옵션은 AI 공개 권한을 추가하지 않습니다.","performed":false}),
    )
}

fn model_page(local: &Value) -> Value {
    let status = &local["status"];
    let items = local["entries"].as_array().into_iter().flatten().take(20).enumerate()
        .map(|(index, entry)| json!({
            "number":index + 1,
            "name":entry["name"].as_str().unwrap_or_default().chars().take(240).collect::<String>(),
            "kind":entry["kind"], "reviewState":entry["reviewState"],
            "decisionSource":entry["decisionSource"], "reasons":entry["reasons"],
            "isDirectory":entry["isDirectory"],
            "logicalBytes":if entry["isDirectory"].as_bool() == Some(true) { Value::Null } else { entry["logicalBytes"].clone() },
            "modifiedAtUnixMs":entry["modifiedAtUnixMs"]
        })).collect::<Vec<_>>();
    json!({"enabled":status["config"]["enabled"],"intervalMinutes":status["config"]["intervalMinutes"],
        "phase":status["phase"],"baselineEntries":status["baselineEntries"],
        "lastCheckedAtUnixMs":status["lastCheckedAtUnixMs"],"nextCheckAtUnixMs":status["nextCheckAtUnixMs"],
        "reviewNeededCount":status["reviewNeededCount"],"skippedCount":status["skippedCount"],
        "offset":local["offset"],"total":local["total"],"items":items,
        "truncated":status["historyTruncated"].as_bool().unwrap_or(false),
        "limitations":"Periodic metadata comparison only, not antivirus, real-time monitoring, or proof of safety. No file contents were read or transmitted. Skipped is a rule/user disposition. Restart and changed configuration require a new baseline. Incomplete results preserve the previous baseline; choose a smaller local folder or retry in Settings.",
        "scanStarted":false,"fileContentsRead":false})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn monitor_preference_never_expands_selected_scope() {
        assert!(!same_root(None, Some("/private/folder")));
        assert!(!same_root(
            Some(Path::new("/private")),
            Some("/private/folder")
        ));
        assert!(same_root(
            Some(Path::new("/private/folder")),
            Some("/private/folder")
        ));
    }
    #[test]
    fn projection_omits_paths_ids_and_local_errors_and_bounds_names() {
        let local = json!({"status":{"config":{"root":"/private/folder","enabled":true,"intervalMinutes":5},
            "phase":"incomplete","error":"failed /private/secret","historyTruncated":true},
            "entries":[{"id":"private-id","name":"가".repeat(300),"path":"/private/secret","previousPath":"/private/old","kind":"renamed","reviewState":"review_needed","reasons":["executable"]}],"offset":0,"total":1});
        let data = model_page(&local);
        let encoded = data.to_string();
        assert!(!encoded.contains("/private"));
        assert!(!encoded.contains("private-id"));
        assert_eq!(
            data["items"][0]["name"].as_str().unwrap().chars().count(),
            240
        );
        assert_eq!(data["phase"], "incomplete");
        assert_eq!(data["truncated"], true);
    }
    #[test]
    fn directory_sizes_are_unmeasured_not_zero_bytes() {
        let data = model_page(&json!({"entries":[
            {"name":"folder","isDirectory":true,"logicalBytes":0},
            {"name":"empty.txt","isDirectory":false,"logicalBytes":0}
        ]}));
        assert_eq!(data["items"][0]["isDirectory"], true);
        assert!(data["items"][0]["logicalBytes"].is_null());
        assert_eq!(data["items"][1]["logicalBytes"], 0);
    }
}
