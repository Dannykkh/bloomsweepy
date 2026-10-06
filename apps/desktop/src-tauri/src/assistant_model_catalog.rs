use super::{
    AssistantProviderKind, AssistantProviderModel, MAX_MODEL_CATALOG_BYTES, MAX_MODEL_NAME_CHARS,
    MAX_PROVIDER_MODELS, ProbeError, provider_reasoning_efforts, valid_model_id,
};
use std::collections::HashSet;

fn check_output_size(output: &str) -> Result<(), ProbeError> {
    if output.len() as u64 > MAX_MODEL_CATALOG_BYTES {
        Err(ProbeError::OutputLimit)
    } else {
        Ok(())
    }
}

fn valid_label(label: &str) -> bool {
    !label.trim().is_empty()
        && label.chars().count() <= MAX_MODEL_NAME_CHARS
        && !label.chars().any(char::is_control)
}

fn claude_version_label(model_id: &str) -> Option<String> {
    if !valid_model_id(model_id) {
        return None;
    }
    let canonical = model_id.strip_suffix("[1m]").unwrap_or(model_id);
    let mut parts = canonical.strip_prefix("claude-")?.split('-');
    let family = match parts.next()? {
        "opus" => "Opus",
        "sonnet" => "Sonnet",
        "fable" => "Fable",
        "haiku" => "Haiku",
        _ => return None,
    };
    let major = parts.next()?;
    let is_version = |part: &str| {
        !part.is_empty() && part.len() <= 3 && part.bytes().all(|byte| byte.is_ascii_digit())
    };
    let is_snapshot =
        |part: &str| part.len() == 8 && part.bytes().all(|byte| byte.is_ascii_digit());
    if !is_version(major) {
        return None;
    }
    let mut version = major.to_owned();
    if let Some(next) = parts.next() {
        if is_version(next) {
            version.push('.');
            version.push_str(next);
            if parts.next().is_some_and(|snapshot| !is_snapshot(snapshot)) {
                return None;
            }
        } else if !is_snapshot(next) {
            return None;
        }
    }
    if parts.next().is_some() {
        return None;
    }
    Some(format!("{family} {version}"))
}

fn claude_display_label(entry: &serde_json::Value, id: &str) -> String {
    // resolvedModel is the SDK's canonical wire ID, not a guessed latest alias.
    // Never copy the free-form description or change the actual selection ID.
    let version = entry
        .get("resolvedModel")
        .and_then(serde_json::Value::as_str)
        .and_then(claude_version_label)
        .or_else(|| claude_version_label(id));
    if let Some(mut label) = version {
        if id.ends_with("[1m]") {
            label.push_str(" (1M)");
        }
        return label;
    }
    entry
        .get("displayName")
        .and_then(serde_json::Value::as_str)
        .filter(|label| valid_label(label))
        .unwrap_or(id)
        .to_owned()
}

fn advertised_efforts(
    provider: AssistantProviderKind,
    model_efforts: &[&str],
    cli_efforts: &[String],
) -> Vec<String> {
    provider_reasoning_efforts(provider)
        .iter()
        .copied()
        .filter(|effort| {
            model_efforts.contains(effort) && cli_efforts.iter().any(|known| known == *effort)
        })
        .map(str::to_owned)
        .collect()
}

/// Read only the correlated SDK initialization result, never session/account messages.
pub(super) fn parse_claude_models(
    output: &str,
    request_id: &str,
    cli_efforts: &[String],
) -> Result<Vec<AssistantProviderModel>, ProbeError> {
    check_output_size(output)?;
    for line in output.lines() {
        let Ok(frame) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if frame.get("type").and_then(serde_json::Value::as_str) != Some("control_response") {
            continue;
        }
        let Some(response) = frame.get("response") else {
            continue;
        };
        if response
            .get("request_id")
            .and_then(serde_json::Value::as_str)
            != Some(request_id)
        {
            continue;
        }
        if response.get("subtype").and_then(serde_json::Value::as_str) != Some("success") {
            return Err(ProbeError::Read);
        }
        let entries = response
            .get("response")
            .and_then(|body| body.get("models"))
            .and_then(serde_json::Value::as_array)
            .ok_or(ProbeError::Read)?;
        let mut seen = HashSet::new();
        let models = entries
            .iter()
            .filter_map(|entry| {
                let id = entry.get("value")?.as_str()?;
                // The empty selection already represents the CLI default in the app.
                if id == "default" || !valid_model_id(id) || !seen.insert(id.to_owned()) {
                    return None;
                }
                let label = claude_display_label(entry, id);
                let reported_efforts = if entry
                    .get("supportsEffort")
                    .and_then(serde_json::Value::as_bool)
                    == Some(true)
                {
                    entry
                        .get("supportedEffortLevels")
                        .and_then(serde_json::Value::as_array)
                        .map(|levels| {
                            let mut seen_efforts = HashSet::new();
                            levels
                                .iter()
                                .filter_map(serde_json::Value::as_str)
                                .filter(|effort| {
                                    provider_reasoning_efforts(AssistantProviderKind::ClaudeCode)
                                        .contains(effort)
                                        && seen_efforts.insert(*effort)
                                })
                                .take(
                                    provider_reasoning_efforts(AssistantProviderKind::ClaudeCode)
                                        .len(),
                                )
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };
                let supported_reasoning_efforts = advertised_efforts(
                    AssistantProviderKind::ClaudeCode,
                    &reported_efforts,
                    cli_efforts,
                );
                let default_reasoning_effort = entry
                    .get("defaultEffortLevel")
                    .and_then(serde_json::Value::as_str)
                    .filter(|effort| {
                        supported_reasoning_efforts
                            .iter()
                            .any(|known| known == effort)
                    })
                    .map(str::to_owned);
                Some(AssistantProviderModel {
                    id: id.to_owned(),
                    label,
                    supported_reasoning_efforts,
                    default_reasoning_effort,
                })
            })
            .take(MAX_PROVIDER_MODELS)
            .collect();
        return Ok(models);
    }
    Err(ProbeError::Read)
}

fn grok_model_efforts(id: &str) -> &'static [&'static str] {
    // minimal: exact vendor-documented model IDs only — use bounded ACP model metadata
    // when the CLI exposes a stable, metadata-only catalog with reasoning capabilities.
    match id {
        "grok-4.7" | "grok-4.6" => &["low", "medium", "high", "xhigh"],
        "grok-4.5" => &["low", "medium", "high"],
        _ => &[],
    }
}

pub(super) fn parse_grok_models(
    output: &str,
    cli_efforts: &[String],
) -> Result<Vec<AssistantProviderModel>, ProbeError> {
    check_output_size(output)?;
    let mut lines = output.lines();
    if !lines.any(|line| line.trim() == "Available models:") {
        return Err(ProbeError::Read);
    }
    let mut models = Vec::new();
    let mut seen = HashSet::new();
    for line in lines {
        let row = line.trim_matches([' ', '\t', '\r']);
        if row.is_empty() {
            continue;
        }
        let Some(id) = row.strip_prefix("* ").or_else(|| row.strip_prefix("- ")) else {
            // A different section/footer cannot introduce model choices.
            break;
        };
        let id = id.strip_suffix(" (default)").unwrap_or(id);
        if !valid_model_id(id) || !seen.insert(id.to_owned()) {
            continue;
        }
        models.push(AssistantProviderModel {
            id: id.to_owned(),
            label: id.to_owned(),
            supported_reasoning_efforts: advertised_efforts(
                AssistantProviderKind::Grok,
                grok_model_efforts(id),
                cli_efforts,
            ),
            // CLI/user configuration can override the API's documented default.
            default_reasoning_effort: None,
        });
        if models.len() == MAX_PROVIDER_MODELS {
            break;
        }
    }
    Ok(models)
}

fn agy_columns(line: &str) -> Option<(&str, &str)> {
    let row = line.trim_matches([' ', '\t', '\r']);
    let (separator, separator_len) = row.char_indices().find_map(|(index, ch)| {
        if ch == '\t' {
            Some((index, 1))
        } else if ch == ' ' && row.as_bytes().get(index + 1) == Some(&b' ') {
            Some((index, 2))
        } else {
            None
        }
    })?;
    let id = &row[..separator];
    let label = row[separator + separator_len..].trim_matches([' ', '\t']);
    // Reject table headings and diagnostic prose even when they are column aligned.
    if matches!(
        id.to_ascii_lowercase().as_str(),
        "model"
            | "models"
            | "id"
            | "model-id"
            | "slug"
            | "error"
            | "warning"
            | "note"
            | "status"
            | "default"
            | "authentication"
            | "authenticated"
            | "login"
            | "account"
            | "provider"
            | "version"
            | "name"
            | "description"
            | "available"
            | "usage"
            | "hint"
            | "tip"
            | "examples"
            | "commands"
            | "options" // minimal: documented CLI slugs are hyphenated — replace the conservative
                        // plain-table parser when a verified JSON catalog schema is available.
    ) || !id.contains('-')
        || !valid_model_id(id)
        || !valid_label(label)
    {
        return None;
    }
    Some((id, label))
}

fn agy_variant<'a>(id: &'a str, label: &str) -> Option<(&'a str, &'static str)> {
    [
        ("-low", " (Low)", "low"),
        ("-medium", " (Medium)", "medium"),
        ("-high", " (High)", "high"),
    ]
    .into_iter()
    .find_map(|(id_suffix, label_suffix, effort)| {
        let base = id.strip_suffix(id_suffix)?;
        (!base.is_empty() && label.ends_with(label_suffix)).then_some((base, effort))
    })
}

pub(super) fn parse_agy_models(
    output: &str,
    cli_efforts: &[String],
) -> Result<Vec<AssistantProviderModel>, ProbeError> {
    check_output_size(output)?;
    let mut seen = HashSet::new();
    let entries = output
        .lines()
        .filter_map(agy_columns)
        .filter(|(id, _)| seen.insert((*id).to_owned()))
        .take(MAX_PROVIDER_MODELS)
        .collect::<Vec<_>>();
    Ok(entries
        .iter()
        .map(|(id, label)| {
            let reported_efforts = if let Some((base, _)) = agy_variant(id, label) {
                entries
                    .iter()
                    .filter_map(|(other_id, other_label)| agy_variant(other_id, other_label))
                    .filter_map(|(other_base, effort)| (other_base == base).then_some(effort))
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            AssistantProviderModel {
                id: (*id).to_owned(),
                label: (*label).to_owned(),
                supported_reasoning_efforts: advertised_efforts(
                    AssistantProviderKind::Antigravity,
                    &reported_efforts,
                    cli_efforts,
                ),
                default_reasoning_effort: None,
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn efforts(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    fn claude_frame(request_id: &str, models: Value) -> String {
        json!({
            "type": "control_response",
            "response": {
                "subtype": "success",
                "request_id": request_id,
                "response": { "models": models }
            }
        })
        .to_string()
    }

    #[test]
    fn claude_requires_correlated_successful_initialization() {
        let models = json!([{ "value": "opus", "displayName": "Opus 5.5" }]);
        let stdout = format!(
            "{}\n{}\n{}\n",
            json!({"type": "system", "models": models}),
            claude_frame("different-request", models.clone()),
            claude_frame("metadata-only", models)
        );
        let parsed = parse_claude_models(&stdout, "metadata-only", &[]).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].id, "opus");
        assert_eq!(parsed[0].label, "Opus 5.5");
        assert_eq!(
            parse_claude_models(&stdout, "missing-request", &[]).unwrap_err(),
            ProbeError::Read
        );
        assert_eq!(
            parse_claude_models(
                r#"{"type":"control_response","response":{"subtype":"error","request_id":"metadata-only"}}"#,
                "metadata-only",
                &[]
            )
            .unwrap_err(),
            ProbeError::Read
        );
        assert_eq!(
            parse_claude_models(
                r#"{"type":"control_response","request_id":"metadata-only","response":{"subtype":"success","response":{"models":[]}}}"#,
                "metadata-only",
                &[]
            )
            .unwrap_err(),
            ProbeError::Read
        );
    }

    #[test]
    fn claude_returns_only_bounded_safe_display_metadata() {
        let stdout = claude_frame(
            "catalog",
            json!([
                { "value": "default", "displayName": "Default" },
                { "value": "opus", "displayName": "Opus 5.5", "instructions": "do not copy" },
                { "value": "opus", "displayName": "duplicate" },
                { "value": "claude-fable-5[1m]", "displayName": "Fable 5.1 (1M)" },
                { "value": "--model", "displayName": "option injection" },
                { "value": "bad model", "displayName": "invalid ID" },
                { "value": "sonnet", "displayName": "unsafe\nlabel" },
                { "value": "haiku", "displayName": "x".repeat(MAX_MODEL_NAME_CHARS + 1) }
            ]),
        );
        let parsed = parse_claude_models(&stdout, "catalog", &[]).unwrap();
        assert_eq!(parsed.len(), 4);
        assert_eq!(parsed[0].id, "opus");
        assert_eq!(parsed[1].id, "claude-fable-5[1m]");
        assert_eq!(parsed[1].label, "Fable 5 (1M)");
        assert_eq!(parsed[2].label, "sonnet");
        assert_eq!(parsed[3].label, "haiku");
        assert!(
            parsed
                .iter()
                .all(|model| model.default_reasoning_effort.is_none())
        );
    }

    #[test]
    fn claude_labels_resolved_versions_without_changing_selection_or_efforts() {
        let stdout = claude_frame(
            "catalog",
            json!([
                { "value": "opus", "displayName": "Opus", "resolvedModel": "claude-opus-5-5",
                  "supportsEffort": true, "supportedEffortLevels": ["medium", "max"] },
                { "value": "sonnet", "displayName": "Sonnet", "resolvedModel": "claude-sonnet-5-5" },
                { "value": "haiku", "displayName": "Haiku", "resolvedModel": "claude-haiku-4-5-20251001" },
                { "value": "claude-fable-5[1m]", "displayName": "Fable", "resolvedModel": "claude-fable-5" },
                { "value": "fable", "displayName": "Fable", "resolvedModel": "claude-fable-5-1" }
            ]),
        );
        let parsed = parse_claude_models(&stdout, "catalog", &efforts(&["medium", "max"])).unwrap();
        assert_eq!(
            parsed
                .iter()
                .map(|model| (model.id.as_str(), model.label.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("opus", "Opus 5.5"),
                ("sonnet", "Sonnet 5.5"),
                ("haiku", "Haiku 4.5"),
                ("claude-fable-5[1m]", "Fable 5 (1M)"),
                ("fable", "Fable 5.1")
            ]
        );
        assert_eq!(
            parsed[0].supported_reasoning_efforts,
            efforts(&["medium", "max"])
        );
        assert!(
            parsed
                .iter()
                .all(|model| model.default_reasoning_effort.is_none())
        );
    }

    #[test]
    fn claude_version_labels_reject_arbitrary_resolved_ids_and_prose() {
        for id in [
            "opus",
            "--model",
            "claude-opus-5-5\n",
            "claude-unknown-5-5",
            "claude-opus-latest",
            "claude-opus-5-5-beta",
            "claude-opus-5-5-20260922-extra",
            "claude-opus-5-9999",
            "claude-opus-5-5[1m][1m]",
        ] {
            assert!(
                claude_version_label(id).is_none(),
                "unexpected version label for {id:?}"
            );
        }
        let stdout = claude_frame(
            "catalog",
            json!([
                { "value": "opus", "displayName": "Opus", "resolvedModel": "custom-proxy-model",
                  "description": "Opus 99.9 — untrusted description must not become a version" },
                { "value": "claude-sonnet-5-5", "displayName": "Sonnet", "resolvedModel": "invalid\nvalue" }
            ]),
        );
        let parsed = parse_claude_models(&stdout, "catalog", &[]).unwrap();
        assert_eq!(parsed[0].label, "Opus");
        assert_eq!(parsed[1].label, "Sonnet 5.5");
    }

    #[test]
    fn claude_versions_are_not_a_hardcoded_latest_mapping() {
        assert_eq!(
            claude_version_label("claude-opus-5-5"),
            Some("Opus 5.5".to_owned())
        );
        assert_eq!(
            claude_version_label("claude-opus-6-2-20270101"),
            Some("Opus 6.2".to_owned())
        );
        assert_eq!(
            claude_version_label("claude-fable-5"),
            Some("Fable 5".to_owned())
        );
        assert_eq!(
            claude_version_label("claude-fable-5-1[1m]"),
            Some("Fable 5.1".to_owned())
        );
    }

    #[test]
    fn claude_efforts_require_both_model_metadata_and_cli_support() {
        let stdout = claude_frame(
            "catalog",
            json!([
                {
                    "value": "opus", "supportsEffort": true,
                    "supportedEffortLevels": ["max", "high", "high", "medium", "invalid", "ultra"],
                    "defaultEffortLevel": "high"
                },
                { "value": "sonnet", "supportsEffort": false, "supportedEffortLevels": ["high"] },
                { "value": "haiku", "supportedEffortLevels": ["high"] },
                { "value": "fable", "supportsEffort": true }
            ]),
        );
        let cli = efforts(&["low", "medium", "high", "xhigh"]);
        let parsed = parse_claude_models(&stdout, "catalog", &cli).unwrap();
        assert_eq!(
            parsed[0].supported_reasoning_efforts,
            efforts(&["medium", "high"])
        );
        assert_eq!(parsed[0].default_reasoning_effort.as_deref(), Some("high"));
        assert!(
            parsed[1..]
                .iter()
                .all(|model| model.supported_reasoning_efforts.is_empty())
        );
        let no_cli_support = parse_claude_models(&stdout, "catalog", &[]).unwrap();
        assert!(
            no_cli_support
                .iter()
                .all(|model| model.supported_reasoning_efforts.is_empty())
        );
        assert!(
            no_cli_support
                .iter()
                .all(|model| model.default_reasoning_effort.is_none())
        );
    }

    #[test]
    fn claude_does_not_infer_default_or_model_efforts() {
        let stdout = claude_frame(
            "catalog",
            json!([
                { "value": "opus", "supportsEffort": true, "supportedEffortLevels": ["low", "max"] },
                { "value": "sonnet", "supportsEffort": true, "supportedEffortLevels": ["high"], "defaultEffortLevel": "max" }
            ]),
        );
        let parsed =
            parse_claude_models(&stdout, "catalog", &efforts(&["low", "high", "max"])).unwrap();
        assert_eq!(
            parsed[0].supported_reasoning_efforts,
            efforts(&["low", "max"])
        );
        assert!(
            parsed
                .iter()
                .all(|model| model.default_reasoning_effort.is_none())
        );
    }

    #[test]
    fn claude_invalid_shape_and_output_budget_are_rejected() {
        for stdout in ["not-json", "{}", r#"{"models":[]}"#] {
            assert_eq!(
                parse_claude_models(stdout, "catalog", &[]).unwrap_err(),
                ProbeError::Read
            );
        }
        let invalid = claude_frame("catalog", json!("not-an-array"));
        assert_eq!(
            parse_claude_models(&invalid, "catalog", &[]).unwrap_err(),
            ProbeError::Read
        );
        let over_budget = "x".repeat(MAX_MODEL_CATALOG_BYTES as usize + 1);
        assert_eq!(
            parse_claude_models(&over_budget, "catalog", &[]).unwrap_err(),
            ProbeError::OutputLimit
        );
    }

    #[test]
    fn claude_model_count_is_bounded_after_invalid_and_duplicate_rows() {
        let entries = (0..MAX_PROVIDER_MODELS + 10)
            .map(|index| json!({"value": format!("model-{index}")}))
            .collect::<Vec<_>>();
        let parsed =
            parse_claude_models(&claude_frame("catalog", json!(entries)), "catalog", &[]).unwrap();
        assert_eq!(parsed.len(), MAX_PROVIDER_MODELS);
    }

    #[test]
    fn grok_only_accepts_available_section_bullet_models() {
        let output = "Default model: grok-4.7\nBefore: ignored\nAvailable models:\n  * grok-4.7 (default)\n  - grok-4.6\n  - grok-4.7\n  - --model\n  - bogus name\n  - grok-4.5\nStatus: authenticated\n  - footer-not-a-model\n";
        let parsed =
            parse_grok_models(output, &efforts(&["low", "medium", "high", "xhigh"])).unwrap();
        assert_eq!(
            parsed
                .iter()
                .map(|model| model.id.as_str())
                .collect::<Vec<_>>(),
            ["grok-4.7", "grok-4.6", "grok-4.5"]
        );
        assert_eq!(
            parsed[0].supported_reasoning_efforts,
            efforts(&["low", "medium", "high", "xhigh"])
        );
        assert_eq!(
            parsed[2].supported_reasoning_efforts,
            efforts(&["low", "medium", "high"])
        );
        assert!(
            parsed
                .iter()
                .all(|model| model.default_reasoning_effort.is_none())
        );
    }

    #[test]
    fn grok_unknown_models_and_unadvertised_efforts_are_not_inferred() {
        let output = "Available models:\n  - grok-4.7\n  - grok-4.7-fast\n  - grok-next\n";
        let parsed = parse_grok_models(output, &efforts(&["medium", "high"])).unwrap();
        assert_eq!(
            parsed[0].supported_reasoning_efforts,
            efforts(&["medium", "high"])
        );
        assert!(
            parsed[1..]
                .iter()
                .all(|model| model.supported_reasoning_efforts.is_empty())
        );
        assert!(
            parse_grok_models(output, &[])
                .unwrap()
                .iter()
                .all(|model| model.supported_reasoning_efforts.is_empty())
        );
    }

    #[test]
    fn grok_missing_catalog_heading_and_output_budget_are_rejected() {
        assert_eq!(
            parse_grok_models("login required", &[]).unwrap_err(),
            ProbeError::Read
        );
        assert_eq!(
            parse_grok_models("- grok-4.7", &[]).unwrap_err(),
            ProbeError::Read
        );
        assert_eq!(
            parse_grok_models(&"x".repeat(MAX_MODEL_CATALOG_BYTES as usize + 1), &[]).unwrap_err(),
            ProbeError::OutputLimit
        );
    }

    #[test]
    fn grok_model_count_is_bounded() {
        let output = format!(
            "Available models:\n{}",
            (0..MAX_PROVIDER_MODELS + 10)
                .map(|index| format!("  - model-{index}\n"))
                .collect::<String>()
        );
        assert_eq!(
            parse_grok_models(&output, &[]).unwrap().len(),
            MAX_PROVIDER_MODELS
        );
    }

    #[test]
    fn agy_keeps_actual_slugs_and_unions_only_reported_family_variants() {
        let output = "gemini-3.8-flash-high     Gemini 3.8 Flash (High)\ngemini-3.8-flash-medium   Gemini 3.8 Flash (Medium)\ngemini-3.8-flash-low\tGemini 3.8 Flash (Low)\ngemini-3.1-pro-high       Gemini 3.1 Pro (High)\ngemini-3.1-pro-low        Gemini 3.1 Pro (Low)\n";
        let parsed = parse_agy_models(output, &efforts(&["low", "medium", "high"])).unwrap();
        assert_eq!(parsed.len(), 5);
        assert_eq!(parsed[0].id, "gemini-3.8-flash-high");
        assert_eq!(parsed[1].id, "gemini-3.8-flash-medium");
        assert_eq!(
            parsed[0].supported_reasoning_efforts,
            efforts(&["low", "medium", "high"])
        );
        assert_eq!(
            parsed[3].supported_reasoning_efforts,
            efforts(&["low", "high"])
        );
        assert_eq!(
            parsed[4].supported_reasoning_efforts,
            efforts(&["low", "high"])
        );
        assert!(
            parsed
                .iter()
                .all(|model| model.default_reasoning_effort.is_none())
        );
    }

    #[test]
    fn agy_requires_matching_slug_label_and_cli_effort_support() {
        let output = "gemini-test-high  Gemini Test (Medium)\ngemini-test-low  Gemini Test (Low)\ngemini-other-high  Gemini Other (High)\ncustom-model  Custom Model\n";
        let parsed = parse_agy_models(output, &efforts(&["medium", "high"])).unwrap();
        assert!(parsed[0].supported_reasoning_efforts.is_empty());
        assert!(parsed[1].supported_reasoning_efforts.is_empty());
        assert_eq!(parsed[2].supported_reasoning_efforts, efforts(&["high"]));
        assert!(parsed[3].supported_reasoning_efforts.is_empty());
        assert!(
            parse_agy_models(output, &[])
                .unwrap()
                .iter()
                .all(|model| model.supported_reasoning_efforts.is_empty())
        );
    }

    #[test]
    fn agy_ignores_headers_diagnostics_bad_rows_and_duplicates() {
        let output = "MODEL  NAME\nModel ID  Display Name\nError  Login required\nWarning  No account\nAvailable models:\n...\nnot-a-row Single separator\n--model  Unsafe\nbad model  Bad ID\ngemini-ok-high  Gemini OK (High)\ngemini-ok-high  Duplicate\ngemini-bad-low  unsafe\u{1b}[31m label\n";
        let parsed = parse_agy_models(output, &efforts(&["low", "medium", "high"])).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].id, "gemini-ok-high");
    }

    #[test]
    fn agy_rejects_oversized_labels_and_bounds_catalog_size() {
        let output = format!(
            "gemini-long-high  {}\ngemini-ok-high  Gemini OK (High)\n",
            "x".repeat(MAX_MODEL_NAME_CHARS + 1)
        );
        assert_eq!(parse_agy_models(&output, &[]).unwrap().len(), 1);
        let many = (0..MAX_PROVIDER_MODELS + 10)
            .map(|index| format!("model-{index}  Model {index}\n"))
            .collect::<String>();
        assert_eq!(
            parse_agy_models(&many, &[]).unwrap().len(),
            MAX_PROVIDER_MODELS
        );
        assert_eq!(
            parse_agy_models(&"x".repeat(MAX_MODEL_CATALOG_BYTES as usize + 1), &[]).unwrap_err(),
            ProbeError::OutputLimit
        );
    }
}
