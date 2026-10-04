//! Bounded projections of app-owned system/application services. These tools
//! can query or prepare a review, never approve removal/termination/cleanup.
use crate::{application_actions, system_performance};
use bloomsweepy_control::{
    AppToolRequest, AppToolResult, AppToolStatus, ApplicationReviewKind, ApplicationSort, UsageSort,
};
use serde_json::{Value, json};
use std::cmp::Ordering as SortOrdering;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager};

const MAX_RESULTS: usize = 24;
const MAX_TEXT_CHARS: usize = 240;
const MAX_QUERY_CHARS: usize = 256;

pub(crate) async fn execute(
    app: &AppHandle,
    _scope: &crate::app_tools::ToolScope,
    request: &AppToolRequest,
    cancellation: Arc<AtomicBool>,
) -> Result<Option<AppToolResult>, String> {
    check_cancelled(&cancellation)?;
    let result = match request {
        AppToolRequest::Performance {
            query,
            sort,
            offset,
            max_results,
        } => {
            validate_page(query, *max_results)?;
            let state = Arc::clone(
                app.state::<Arc<system_performance::PerformanceMonitorState>>()
                    .inner(),
            );
            let snapshot = tauri::async_runtime::spawn_blocking(move || state.collect_snapshot())
                .await
                .map_err(|_| "성능 상태 조회 작업을 완료하지 못했습니다".to_owned())??;
            check_cancelled(&cancellation)?;
            performance_result(
                serde_json::to_value(snapshot).map_err(serialization_error)?,
                query,
                sort,
                *offset,
                *max_results,
            )
        }
        AppToolRequest::Applications {
            query,
            sort,
            offset,
            max_results,
        } => {
            validate_page(query, *max_results)?;
            let cached = application_actions::inventory_for_tools(app).await?;
            check_cancelled(&cancellation)?;
            applications_result(
                serde_json::to_value(cached.inventory).map_err(serialization_error)?,
                cached.captured_at_unix_ms,
                query,
                sort,
                *offset,
                *max_results,
            )
        }
        AppToolRequest::ApplicationInspect {
            inventory_id,
            application_id,
        } => {
            match application_actions::inspect_for_tools(app, inventory_id, application_id).await {
                Ok(inspection) => {
                    check_cancelled(&cancellation)?;
                    let local = serde_json::to_value(inspection).map_err(serialization_error)?;
                    let candidates = local["relatedData"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|item| {
                            json!({
                                "id": item["id"], "kind": item["kind"],
                                "estimatedBytes": item["estimatedBytes"],
                                "evidence": bounded_text(&item["evidence"]),
                            })
                        })
                        .collect::<Vec<_>>();
                    AppToolResult::completed(
                        "applications.inspect",
                        json!({
                            "inventoryId": inventory_id, "applicationId": application_id,
                            "inventoryCapturedAtUnixMs": local["capturedAtUnixMs"],
                            "application": application_projection(&local["application"]),
                            "relatedDataSupported": local["relatedDataSupported"],
                            "relatedData": candidates,
                            "warnings": bounded_messages(&local["warnings"]),
                            "deletionSafety": "unknown", "removed": false,
                        }),
                    )
                    .with_presentation(json!({
                        "view": "applications", "inventoryId": inventory_id,
                        "applicationId": application_id, "inspection": local,
                    }))
                }
                Err(_) => {
                    check_cancelled(&cancellation)?;
                    unavailable("applications.inspect", "inspectionUnavailable")
                }
            }
        }
        AppToolRequest::ApplicationReview {
            inventory_id,
            application_id,
            kind,
            candidate_ids,
        } => {
            if matches!(kind, ApplicationReviewKind::Data)
                && (candidate_ids.is_empty() || candidate_ids.len() > 2)
            {
                return Err("관련 데이터 후보를 1~2개 선택하세요".to_owned());
            }
            if !cfg!(target_os = "macos") {
                AppToolResult::with_status("applications.review", AppToolStatus::Unsupported, json!({
                    "platform": std::env::consts::OS,
                    "outcome": "useOperatingSystemUninstallSettings",
                    "removed": false,
                    "message": "Windows에서는 앱 화면의 정식 운영체제 제거 절차를 사용하세요. 제거 완료를 확인한 것이 아닙니다.",
                })).with_presentation(json!({ "view": "applications" }))
            } else {
                if matches!(kind, ApplicationReviewKind::Bundle) && !candidate_ids.is_empty() {
                    return Err("앱 본체 검토에 관련 데이터 선택을 포함할 수 없습니다".to_owned());
                }
                let selected = match kind {
                    ApplicationReviewKind::Bundle => None,
                    ApplicationReviewKind::Data => Some(candidate_ids.clone()),
                };
                match application_actions::prepare_for_tools(
                    app,
                    inventory_id,
                    application_id,
                    selected,
                )
                .await
                {
                    Ok(plan) => {
                        check_cancelled(&cancellation)?;
                        let local_plan = serde_json::to_value(plan).map_err(serialization_error)?;
                        let cached = application_actions::inventory_for_tools(app).await?;
                        let review_kind = match kind {
                            ApplicationReviewKind::Bundle => "applicationBundle",
                            ApplicationReviewKind::Data => "applicationData",
                        };
                        AppToolResult::with_status(
                            "applications.review",
                            AppToolStatus::ReviewRequired,
                            json!({
                                "outcome": "preparedAwaitingUserConfirmation",
                                "inventoryId": inventory_id, "applicationId": application_id,
                                "displayName": bounded_text(&local_plan["displayName"]),
                                "reviewKind": review_kind,
                                "expiresAtUnixMs": local_plan["expiresAtUnixMs"],
                                "warnings": bounded_messages(&local_plan["warnings"]),
                                "removed": false, "relatedDataAutomaticallyRemoved": false,
                            }),
                        )
                        .with_presentation(json!({
                            "view": "applications", "reviewKind": review_kind,
                            "plan": local_plan, "inventory": cached.inventory,
                            "inventoryId": inventory_id, "applicationId": application_id,
                            "candidateIds": candidate_ids,
                        }))
                    }
                    Err(_) => {
                        check_cancelled(&cancellation)?;
                        unavailable("applications.review", "reviewUnavailable")
                    }
                }
            }
        }
        AppToolRequest::ProcessReview {
            snapshot_id,
            target_id,
        } => {
            let state = Arc::clone(
                app.state::<Arc<system_performance::PerformanceMonitorState>>()
                    .inner(),
            );
            let snapshot_id = snapshot_id.clone();
            let target_id = target_id.clone();
            let response = tauri::async_runtime::spawn_blocking(move || {
                state.prepare_termination(&snapshot_id, &target_id)
            })
            .await
            .map_err(|_| "앱 종료 확인을 준비하지 못했습니다".to_owned())??;
            check_cancelled(&cancellation)?;
            let local = serde_json::to_value(response).map_err(serialization_error)?;
            match local["outcome"].as_str() {
                Some("ready") => AppToolResult::with_status(
                    "processes.review",
                    AppToolStatus::ReviewRequired,
                    json!({
                        "outcome": "preparedAwaitingUserConfirmation",
                        "displayName": bounded_text(&local["preview"]["displayName"]),
                        "expiresAtUnixMs": local["preview"]["expiresAtUnixMs"],
                        "terminationMode": "gracefulRequestOnly", "terminationRequested": false,
                        "unsavedWorkRisk": true,
                    }),
                )
                .with_presentation(json!({
                    "view": "performance", "reviewKind": "process", "preview": local["preview"],
                })),
                Some("unsupported") => AppToolResult::with_status(
                    "processes.review",
                    AppToolStatus::Unsupported,
                    json!({
                        "outcome": "unsupported", "terminationRequested": false,
                        "platform": std::env::consts::OS,
                    }),
                ),
                _ => AppToolResult::completed(
                    "processes.review",
                    json!({
                        "outcome": local["outcome"], "terminationRequested": false,
                        "refreshRequired": true,
                    }),
                ),
            }
        }
        AppToolRequest::MemoryReview {} => {
            let status = if cfg!(target_os = "macos") {
                AppToolStatus::ReviewRequired
            } else {
                AppToolStatus::Unsupported
            };
            AppToolResult::with_status("memory.review", status, json!({
                "scope": "broomSweepyHostAllocator", "platform": std::env::consts::OS,
                "executed": false,
                "message": "BroomSweepy Rust 호스트의 사용하지 않는 allocator 영역만 반환 요청합니다. 시스템 전체나 다른 앱·WebKit·CLI의 메모리를 정리하지 않으며 CPU 청소도 아닙니다.",
                "systemReclaimGuaranteed": false,
            })).with_presentation(json!({
                "view": "performance", "reviewKind": "memory", "scope": "broomSweepyHostAllocator",
            }))
        }
        _ => return Ok(None),
    };
    check_cancelled(&cancellation)?;
    Ok(Some(result))
}

fn performance_result(
    snapshot: Value,
    query: &str,
    sort: &UsageSort,
    offset: usize,
    max_results: usize,
) -> AppToolResult {
    let mut matching = snapshot["processes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|row| matches_query(row, query, &["displayName", "bundleIdentifier"]))
        .collect::<Vec<_>>();
    matching.sort_by(|left, right| {
        let primary = match sort {
            UsageSort::Memory => right["residentBytes"]
                .as_u64()
                .cmp(&left["residentBytes"].as_u64()),
            UsageSort::Cpu => right["cpuMachinePercent"]
                .as_f64()
                .partial_cmp(&left["cpuMachinePercent"].as_f64())
                .unwrap_or(SortOrdering::Equal),
            UsageSort::Name => SortOrdering::Equal,
        };
        primary.then_with(|| {
            text_value(&left["displayName"])
                .to_lowercase()
                .cmp(&text_value(&right["displayName"]).to_lowercase())
        })
    });
    let matched = matching.len();
    let items = matching.into_iter().skip(offset).take(max_results.min(MAX_RESULTS)).map(|row| json!({
        "targetId": row["targetId"], "displayName": bounded_text(&row["displayName"]),
        "bundleIdentifier": bounded_text(&row["bundleIdentifier"]), "kind": row["kind"],
        "cpuCorePercent": row["cpuCorePercent"], "cpuMachinePercent": row["cpuMachinePercent"],
        "residentBytes": row["residentBytes"], "processCount": row["processCount"],
        "canRequestTermination": row["canRequestTermination"],
        "terminationEligibility": row["terminationEligibility"],
    })).collect::<Vec<_>>();
    let has_more = offset.saturating_add(items.len()) < matched;
    let truncated = has_more || snapshot["processesTruncated"].as_bool().unwrap_or(false);
    let mut result = AppToolResult::completed("performance.inspect", json!({
        "snapshotId": snapshot["snapshotId"], "capturedAtUnixMs": snapshot["capturedAtUnixMs"],
        "sampleWindowMs": snapshot["sampleWindowMs"], "refreshAfterMs": snapshot["refreshAfterMs"],
        "platform": snapshot["platform"], "logicalCpuCount": snapshot["logicalCpuCount"],
        "cpuUsagePercent": snapshot["cpuUsagePercent"], "memory": snapshot["memory"],
        "capabilities": snapshot["capabilities"], "processScope": snapshot["capabilities"]["processScope"],
        "sourceProcessesTruncated": snapshot["processesTruncated"],
        "query": query, "sort": sort, "offset": offset, "maxResults": max_results.min(MAX_RESULTS),
        "matchedCount": matched, "hasMore": has_more, "items": items,
        "snapshotLifetimeMs": 60000, "snapshotMayBeEvictedByNewMeasurements": true,
        "cpuCleaningSupported": false, "deletionSafety": "notApplicable",
    })).with_presentation(json!({ "view": "performance", "snapshot": snapshot }));
    result.truncated = truncated;
    result
}

fn applications_result(
    inventory: Value,
    captured_at: u64,
    query: &str,
    sort: &ApplicationSort,
    offset: usize,
    max_results: usize,
) -> AppToolResult {
    let mut matching = inventory["applications"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|row| matches_query(row, query, &["displayName", "publisher"]))
        .collect::<Vec<_>>();
    matching.sort_by(|left, right| {
        let primary = match sort {
            ApplicationSort::Name => SortOrdering::Equal,
            ApplicationSort::Size => match (
                left["estimatedBytes"].as_u64(),
                right["estimatedBytes"].as_u64(),
            ) {
                (Some(left), Some(right)) => right.cmp(&left),
                (Some(_), None) => SortOrdering::Less,
                (None, Some(_)) => SortOrdering::Greater,
                (None, None) => SortOrdering::Equal,
            },
        };
        primary.then_with(|| {
            text_value(&left["displayName"])
                .to_lowercase()
                .cmp(&text_value(&right["displayName"]).to_lowercase())
        })
    });
    let matched = matching.len();
    let items = matching
        .into_iter()
        .skip(offset)
        .take(max_results.min(MAX_RESULTS))
        .map(application_projection)
        .collect::<Vec<_>>();
    let has_more = offset.saturating_add(items.len()) < matched;
    let issues_count = inventory["issues"].as_array().map_or(0, Vec::len);
    let mut result = AppToolResult::completed(
        "applications.list",
        json!({
            "inventoryId": inventory["inventoryId"], "capturedAtUnixMs": captured_at,
            "platform": inventory["platform"], "query": query, "sort": sort,
            "offset": offset, "maxResults": max_results.min(MAX_RESULTS),
            "matchedCount": matched, "hasMore": has_more, "items": items,
            "sourceIssueCount": issues_count, "sourceMayBeIncomplete": issues_count > 0,
            "installDateAvailable": false,
            "sizeSemantics": "optionalOperatingSystemEstimateNotMeasuredBundleContents",
            "inventoryRefreshInvalidatesIds": true,
        }),
    )
    .with_presentation(json!({
        "view": "applications", "inventory": inventory, "query": query, "sort": sort,
        "offset": offset, "maxResults": max_results.min(MAX_RESULTS),
    }));
    result.truncated = has_more || issues_count > 0;
    result
}

fn application_projection(row: &Value) -> Value {
    json!({
        "id": row["id"], "displayName": bounded_text(&row["displayName"]),
        "displayVersion": bounded_text(&row["displayVersion"]), "publisher": bounded_text(&row["publisher"]),
        "estimatedBytes": row["estimatedBytes"], "removalMode": row["removalMode"],
        "protectionReason": bounded_message(&row["protectionReason"]),
    })
}

fn matches_query(row: &Value, query: &str, fields: &[&str]) -> bool {
    let needle = query.trim().to_lowercase();
    needle.is_empty()
        || fields
            .iter()
            .any(|field| text_value(&row[*field]).to_lowercase().contains(&needle))
}

fn bounded_text(value: &Value) -> Value {
    value
        .as_str()
        .map(|value| Value::String(value.chars().take(MAX_TEXT_CHARS).collect()))
        .unwrap_or(Value::Null)
}

fn text_value(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}

fn bounded_messages(value: &Value) -> Vec<Value> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .take(8)
        .map(bounded_message)
        .collect()
}

fn bounded_message(value: &Value) -> Value {
    if value
        .as_str()
        .is_some_and(|text| text.contains('/') || text.contains('\\'))
    {
        Value::String("앱이 보호되었거나 현재 위치·상태를 확인할 수 없습니다. 자세한 경로는 앱에서 확인하세요.".to_owned())
    } else {
        bounded_text(value)
    }
}

fn validate_page(query: &str, max_results: usize) -> Result<(), String> {
    if query.chars().count() > MAX_QUERY_CHARS || !(1..=MAX_RESULTS).contains(&max_results) {
        return Err("조회 검색어 또는 결과 개수 제한을 초과했습니다".to_owned());
    }
    Ok(())
}

fn check_cancelled(cancellation: &AtomicBool) -> Result<(), String> {
    if cancellation.load(Ordering::Acquire) {
        Err("앱 도구 조회가 취소되었습니다".to_owned())
    } else {
        Ok(())
    }
}

fn serialization_error(_: serde_json::Error) -> String {
    "앱 조회 결과를 준비하지 못했습니다".to_owned()
}

fn unavailable(capability: &str, outcome: &str) -> AppToolResult {
    AppToolResult::completed(
        capability,
        json!({
            "outcome": outcome, "refreshRequired": true, "removed": false,
            "message": "현재 앱 목록/대상을 확인할 수 없거나 보호된 대상입니다. 앱 목록을 다시 확인하세요. 실행된 제거 작업은 없습니다.",
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_projection_sorts_known_sizes_first_and_preserves_unknown_without_paths() {
        let result = applications_result(
            json!({
                "inventoryId": "real-id", "platform": "macos", "issues": [],
                "applications": [
                { "id": "unknown", "displayName": "Alpha", "estimatedBytes": null, "installLocation": "/Users/private/Alpha.app", "protectionReason": "cannot read /Users/private/Alpha.app" },
                    { "id": "small", "displayName": "Gamma", "estimatedBytes": 10 },
                    { "id": "large", "displayName": "Beta", "estimatedBytes": 100 },
                ]
            }),
            123,
            "",
            &ApplicationSort::Size,
            0,
            24,
        );
        assert_eq!(result.data["items"][0]["id"], "large");
        assert_eq!(result.data["items"][2]["estimatedBytes"], Value::Null);
        assert!(result.data["items"][2].get("installLocation").is_none());
        assert!(!result.data.to_string().contains("/Users/private"));
        assert_eq!(result.data["capturedAtUnixMs"], 123);
        assert!(result.presentation.is_some());
    }

    #[test]
    fn app_projection_searches_actual_names_and_bounds_page_and_text() {
        let rows = (0..40).map(|index| json!({
            "id": format!("id-{index}"), "displayName": format!("Match {index}{}", "x".repeat(500)),
            "estimatedBytes": index,
        })).collect::<Vec<_>>();
        let result = applications_result(
            json!({ "applications": rows, "issues": ["private path"], "inventoryId": "actual" }),
            100,
            "match",
            &ApplicationSort::Size,
            0,
            24,
        );
        assert_eq!(result.data["items"].as_array().unwrap().len(), 24);
        assert_eq!(
            result.data["items"][0]["displayName"]
                .as_str()
                .unwrap()
                .chars()
                .count(),
            MAX_TEXT_CHARS
        );
        assert!(result.truncated);
        assert!(!result.data.to_string().contains("private path"));
        assert!(validate_page("", 25).is_err());
        assert!(validate_page(&"한".repeat(257), 24).is_err());
    }

    #[test]
    fn performance_projection_uses_actual_measures_scope_and_identity_handles() {
        let result = performance_result(
            json!({
                "snapshotId": "actual", "processesTruncated": true,
                "capabilities": {"processScope": "guiApplications"},
                "processes": [
                    { "displayName": "App", "targetId": "eligible-handle", "residentBytes": 1024, "cpuMachinePercent": 2.0, "pid": 100, "command": "private" },
                    { "displayName": "Helper", "targetId": null, "residentBytes": 2048, "cpuMachinePercent": 1.0 },
                ]
            }),
            "",
            &UsageSort::Memory,
            0,
            24,
        );
        assert_eq!(result.data["items"][0]["residentBytes"], 2048);
        assert_eq!(result.data["items"][1]["targetId"], "eligible-handle");
        assert!(result.data["items"][1].get("command").is_none());
        assert!(result.data["items"][1].get("pid").is_none());
        assert_eq!(result.data["processScope"], "guiApplications");
        assert!(result.truncated);
    }
}
