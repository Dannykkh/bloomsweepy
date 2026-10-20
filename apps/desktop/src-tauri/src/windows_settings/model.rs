//! Bounded Windows inspection contract. No Tauri, registry writes or OS launch.
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WindowsSettingsSnapshot {
    pub supported: bool,
    pub platform: String,
    pub os_build: Option<String>,
    pub os_revision: Option<u32>,
    pub edition: Option<String>,
    pub settings: Vec<WindowsSetting>,
    pub targets: Vec<WindowsSettingsTarget>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WindowsSetting {
    pub id: String,
    pub title: String,
    pub description: String,
    pub category: SettingCategory,
    pub scope: SettingScope,
    pub state: SettingState,
    pub value: Option<u32>,
    pub value_type: Option<String>,
    pub support: SettingSupport,
    pub target_id: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WindowsSettingsTarget {
    pub id: String,
    pub title: String,
    pub available: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WindowsSettingsOpenResult {
    pub target_id: String,
    pub requested: bool,
    pub changed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum SettingCategory {
    Explorer,
    AiPolicy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum SettingScope {
    CurrentUser,
    Machine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum SettingState {
    Value,
    Unset,
    AccessDenied,
    TypeMismatch,
    UnsupportedValue,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum SettingSupport {
    StoredValueOnly,
    RequiresVerification,
    Deprecated,
}

#[derive(Clone, Copy)]
pub(crate) struct SettingDefinition {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub category: SettingCategory,
    pub scope: SettingScope,
    #[cfg_attr(
        not(any(windows, test)),
        allow(
            dead_code,
            reason = "Fixed registry paths are consumed only by the Windows adapter and catalog tests"
        )
    )]
    pub key: &'static str,
    #[cfg_attr(
        not(any(windows, test)),
        allow(
            dead_code,
            reason = "Fixed registry names are consumed only by the Windows adapter and catalog tests"
        )
    )]
    pub name: &'static str,
    pub allowed_values: &'static [u32],
    pub support: SettingSupport,
    pub target: Target,
}

const EXPLORER_ADVANCED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced";
const WINDOWS_AI: &str = r"Software\Policies\Microsoft\Windows\WindowsAI";
const BOOL_VALUES: &[u32] = &[0, 1];

pub(crate) const DEFINITIONS: [SettingDefinition; 10] = [
    SettingDefinition {
        id: "explorer_hide_extensions",
        title: "파일 확장자 숨김 값",
        description: "HideFileExt의 원시 저장값입니다. 실제 탐색기 표시 상태나 기본값으로 해석하지 않습니다.",
        category: SettingCategory::Explorer,
        scope: SettingScope::CurrentUser,
        key: EXPLORER_ADVANCED,
        name: "HideFileExt",
        allowed_values: BOOL_VALUES,
        support: SettingSupport::StoredValueOnly,
        target: Target::FolderOptions,
    },
    SettingDefinition {
        id: "explorer_hidden_files",
        title: "숨김 파일 표시 값",
        description: "Hidden의 0·1·2 저장값만 구분합니다. 실제 탐색기 표시 여부를 추정하지 않습니다.",
        category: SettingCategory::Explorer,
        scope: SettingScope::CurrentUser,
        key: EXPLORER_ADVANCED,
        name: "Hidden",
        allowed_values: &[0, 1, 2],
        support: SettingSupport::StoredValueOnly,
        target: Target::FolderOptions,
    },
    SettingDefinition {
        id: "explorer_protected_files",
        title: "보호된 운영체제 파일 표시 값",
        description: "ShowSuperHidden의 원시 저장값입니다. 보호 파일을 표시하거나 삭제하지 않습니다.",
        category: SettingCategory::Explorer,
        scope: SettingScope::CurrentUser,
        key: EXPLORER_ADVANCED,
        name: "ShowSuperHidden",
        allowed_values: BOOL_VALUES,
        support: SettingSupport::StoredValueOnly,
        target: Target::FolderOptions,
    },
    SettingDefinition {
        id: "explorer_full_path",
        title: "탐색기 전체 경로 표시 값",
        description: "FullPath의 원시 저장값입니다. 창 표시 상태와 적용 여부는 별도 확인이 필요합니다.",
        category: SettingCategory::Explorer,
        scope: SettingScope::CurrentUser,
        key: r"Software\Microsoft\Windows\CurrentVersion\Explorer\CabinetState",
        name: "FullPath",
        allowed_values: BOOL_VALUES,
        support: SettingSupport::StoredValueOnly,
        target: Target::FolderOptions,
    },
    SettingDefinition {
        id: "legacy_copilot",
        title: "이전 Windows Copilot 정책",
        description: "폐기 예정인 이전 Copilot 정책입니다. 새 Copilot 앱의 설치·실행·차단 상태를 나타내지 않습니다.",
        category: SettingCategory::AiPolicy,
        scope: SettingScope::CurrentUser,
        key: r"Software\Policies\Microsoft\Windows\WindowsCopilot",
        name: "TurnOffWindowsCopilot",
        allowed_values: BOOL_VALUES,
        support: SettingSupport::Deprecated,
        target: Target::Apps,
    },
    SettingDefinition {
        id: "recall_snapshots",
        title: "Recall 스냅샷 저장 정책 · 컴퓨터",
        description: "컴퓨터 범위 DisableAIDataAnalysis 저장값입니다. 사용자 정책과 합성하지 않으며 실제 저장·기능 지원 상태는 미확인입니다.",
        category: SettingCategory::AiPolicy,
        scope: SettingScope::Machine,
        key: WINDOWS_AI,
        name: "DisableAIDataAnalysis",
        allowed_values: BOOL_VALUES,
        support: SettingSupport::RequiresVerification,
        target: Target::Privacy,
    },
    SettingDefinition {
        id: "recall_snapshots_user",
        title: "Recall 스냅샷 저장 정책 · 현재 사용자",
        description: "현재 사용자 범위 DisableAIDataAnalysis 저장값입니다. 컴퓨터 정책과 우선순위를 합성하지 않으며 실제 저장 상태는 미확인입니다.",
        category: SettingCategory::AiPolicy,
        scope: SettingScope::CurrentUser,
        key: WINDOWS_AI,
        name: "DisableAIDataAnalysis",
        allowed_values: BOOL_VALUES,
        support: SettingSupport::RequiresVerification,
        target: Target::Privacy,
    },
    SettingDefinition {
        id: "recall_availability",
        title: "Recall 가용성 정책",
        description: "AllowRecallEnablement 저장값입니다. 구성요소 설치·지원 여부와 다르며 제거하거나 스냅샷을 삭제하지 않습니다.",
        category: SettingCategory::AiPolicy,
        scope: SettingScope::Machine,
        key: WINDOWS_AI,
        name: "AllowRecallEnablement",
        allowed_values: BOOL_VALUES,
        support: SettingSupport::RequiresVerification,
        target: Target::OptionalFeatures,
    },
    SettingDefinition {
        id: "notepad_ai",
        title: "메모장 AI 정책",
        description: "컴퓨터 전체 DisableAIFeatures 저장값입니다. Windows 빌드·에디션·메모장 버전과 실제 정책 적용은 별도 확인이 필요합니다.",
        category: SettingCategory::AiPolicy,
        scope: SettingScope::Machine,
        key: r"Software\Policies\WindowsNotepad",
        name: "DisableAIFeatures",
        allowed_values: BOOL_VALUES,
        support: SettingSupport::RequiresVerification,
        target: Target::Apps,
    },
    SettingDefinition {
        id: "paint_image_creator",
        title: "그림판 Image Creator 정책",
        description: "DisableImageCreator 저장값입니다. Windows 빌드·에디션·앱 버전과 실제 기능 지원·정책 적용은 미확인입니다.",
        category: SettingCategory::AiPolicy,
        scope: SettingScope::Machine,
        key: r"Software\Microsoft\Windows\CurrentVersion\Policies\Paint",
        name: "DisableImageCreator",
        allowed_values: BOOL_VALUES,
        support: SettingSupport::RequiresVerification,
        target: Target::Apps,
    },
];

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    not(any(windows, test)),
    allow(
        dead_code,
        reason = "Only Windows native reads and fake-reader tests construct raw registry observations"
    )
)]
pub(crate) enum RegistryRead {
    Missing,
    AccessDenied,
    Dword(u32),
    TypeMismatch(String),
    Error,
}

/// The native adapter supplies a fixed four-byte buffer. Keep decoding shared
/// with Mac-hosted tests so a malformed/oversized Windows value cannot become 0.
#[cfg(any(windows, test))]
pub(crate) fn decode_dword_value(
    value_type: &str,
    bytes: &[u8],
    reported_length: usize,
    buffer_too_small: bool,
) -> RegistryRead {
    if value_type != "REG_DWORD" || reported_length != 4 || bytes.len() != 4 || buffer_too_small {
        return RegistryRead::TypeMismatch(value_type.into());
    }
    match bytes.try_into() {
        Ok(bytes) => RegistryRead::Dword(u32::from_le_bytes(bytes)),
        Err(_) => RegistryRead::Error,
    }
}

/// OS identity is the only string exception: fixed names, at most 256 bytes,
/// strict terminated UTF-16, and no embedded NUL or truncation recovery.
#[cfg(any(windows, test))]
pub(crate) fn decode_metadata_string(
    value_type: &str,
    bytes: &[u8],
    reported_length: usize,
) -> Option<String> {
    if value_type != "REG_SZ"
        || !(2..=256).contains(&reported_length)
        || reported_length > bytes.len()
        || !reported_length.is_multiple_of(2)
    {
        return None;
    }
    let units = bytes[..reported_length]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .collect::<Vec<_>>();
    if units.last() != Some(&0) || units[..units.len() - 1].contains(&0) {
        return None;
    }
    let value = String::from_utf16(&units[..units.len() - 1]).ok()?;
    (!value.is_empty()).then_some(value)
}

#[derive(Default)]
pub(crate) struct WindowsMetadata {
    pub os_build: Option<String>,
    pub os_revision: Option<u32>,
    pub edition: Option<String>,
    pub warnings: Vec<String>,
}

pub(crate) trait RegistryReader {
    fn metadata(&self) -> WindowsMetadata;
    fn read(&self, definition: &SettingDefinition) -> RegistryRead;
}

pub(crate) fn build_snapshot(
    supported: bool,
    platform: &str,
    reader: &impl RegistryReader,
) -> WindowsSettingsSnapshot {
    if !supported {
        return WindowsSettingsSnapshot { supported:false, platform:platform.to_owned(), os_build:None, os_revision:None, edition:None, settings:Vec::new(), targets:Vec::new(), warnings:vec!["Windows 설정 조회와 설정 화면 연결은 Windows에서만 제공됩니다. 이 시스템에서는 레지스트리를 읽거나 설정 화면을 실행하지 않았습니다.".into()] };
    }
    let metadata = reader.metadata();
    let settings = DEFINITIONS
        .iter()
        .map(|definition| {
            let (state, value, value_type) = match reader.read(definition) {
                RegistryRead::Missing => (SettingState::Unset, None, None),
                RegistryRead::AccessDenied => (SettingState::AccessDenied, None, None),
                RegistryRead::Dword(value) => (
                    if definition.allowed_values.contains(&value) {
                        SettingState::Value
                    } else {
                        SettingState::UnsupportedValue
                    },
                    Some(value),
                    Some("REG_DWORD".into()),
                ),
                RegistryRead::TypeMismatch(value_type) => {
                    (SettingState::TypeMismatch, None, Some(value_type))
                }
                RegistryRead::Error => (SettingState::Error, None, None),
            };
            WindowsSetting {
                id: definition.id.into(),
                title: definition.title.into(),
                description: definition.description.into(),
                category: definition.category,
                scope: definition.scope,
                state,
                value,
                value_type,
                support: definition.support,
                target_id: Some(definition.target.id().into()),
            }
        })
        .collect();
    let mut warnings = vec![
        "64비트 레지스트리 보기의 고정 저장값만 조회합니다. 미설정은 꺼짐이 아니며 저장값은 실제 기능 상태·정책 우선순위·OS/앱 지원 여부를 보장하지 않습니다.".into(),
        "설정 화면 연결은 실행 요청 전달만 의미합니다. Windows 버전별 페이지 가용성·실제 창 표시·설정 변경은 확인하지 않으며 앱은 값을 쓰지 않습니다.".into(),
        "폴더 옵션 연결은 기존 Windows 제어판의 고정 경로입니다. Windows 실기에서 페이지 동작은 별도 검증이 필요합니다.".into(),
    ];
    warnings.extend(metadata.warnings.into_iter().take(3));
    WindowsSettingsSnapshot {
        supported: true,
        platform: platform.into(),
        os_build: metadata.os_build,
        os_revision: metadata.os_revision,
        edition: metadata.edition,
        settings,
        targets: TARGETS
            .iter()
            .map(|target| WindowsSettingsTarget {
                id: target.id().into(),
                title: target.title().into(),
                available: true,
            })
            .collect(),
        warnings,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Target {
    FolderOptions,
    Storage,
    Startup,
    Apps,
    Taskbar,
    Privacy,
    About,
    OptionalFeatures,
}
pub(crate) const TARGETS: [Target; 8] = [
    Target::FolderOptions,
    Target::Storage,
    Target::Startup,
    Target::Apps,
    Target::Taskbar,
    Target::Privacy,
    Target::About,
    Target::OptionalFeatures,
];

impl Target {
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::FolderOptions => "folder_options",
            Self::Storage => "storage",
            Self::Startup => "startup",
            Self::Apps => "apps",
            Self::Taskbar => "taskbar",
            Self::Privacy => "privacy",
            Self::About => "about",
            Self::OptionalFeatures => "optional_features",
        }
    }
    pub(crate) fn title(self) -> &'static str {
        match self {
            Self::FolderOptions => "파일 탐색기 옵션",
            Self::Storage => "저장소 설정",
            Self::Startup => "시작 앱 설정",
            Self::Apps => "설치된 앱 설정",
            Self::Taskbar => "작업 표시줄 설정",
            Self::Privacy => "개인 정보 설정",
            Self::About => "Windows 정보",
            Self::OptionalFeatures => "선택적 기능 설정",
        }
    }
    #[cfg(any(windows, test))]
    pub(crate) fn uri(self) -> Option<&'static str> {
        match self {
            Self::FolderOptions => None,
            Self::Storage => Some("ms-settings:storagesense"),
            Self::Startup => Some("ms-settings:startupapps"),
            Self::Apps => Some("ms-settings:appsfeatures"),
            Self::Taskbar => Some("ms-settings:taskbar"),
            Self::Privacy => Some("ms-settings:privacy"),
            Self::About => Some("ms-settings:about"),
            Self::OptionalFeatures => Some("ms-settings:optionalfeatures"),
        }
    }
}

pub(crate) fn resolve_target(supported: bool, id: &str) -> Result<Target, String> {
    if !supported {
        return Err("Windows에서만 설정 화면 연결을 요청할 수 있습니다".into());
    }
    TARGETS
        .iter()
        .copied()
        .find(|target| target.id() == id)
        .ok_or_else(|| "지원하지 않는 Windows 설정 대상입니다".into())
}

pub(crate) fn open_target_with(
    supported: bool,
    id: &str,
    launch: impl FnOnce(Target) -> Result<(), String>,
) -> Result<WindowsSettingsOpenResult, String> {
    let target = resolve_target(supported, id)?;
    launch(target)?;
    Ok(WindowsSettingsOpenResult {
        target_id: target.id().into(),
        requested: true,
        changed: false,
    })
}

pub(crate) fn trusted_main_caller(
    label: &str,
    scheme: &str,
    host: Option<&str>,
    port: Option<u16>,
    credentials: bool,
    development: bool,
) -> bool {
    label == "main"
        && !credentials
        && ((matches!(
            (scheme, host),
            ("tauri", Some("localhost")) | ("http", Some("tauri.localhost"))
        ) && port.is_none())
            || (development && scheme == "http" && host == Some("localhost") && port == Some(1420)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, collections::HashSet};
    struct FakeReader {
        read: RegistryRead,
        reads: Cell<usize>,
        metadata_reads: Cell<usize>,
    }
    impl FakeReader {
        fn new(read: RegistryRead) -> Self {
            Self {
                read,
                reads: Cell::new(0),
                metadata_reads: Cell::new(0),
            }
        }
    }
    impl RegistryReader for FakeReader {
        fn metadata(&self) -> WindowsMetadata {
            self.metadata_reads.set(self.metadata_reads.get() + 1);
            WindowsMetadata {
                os_build: Some("26100".into()),
                os_revision: Some(123),
                edition: Some("Synthetic".into()),
                warnings: vec![],
            }
        }
        fn read(&self, _: &SettingDefinition) -> RegistryRead {
            self.reads.set(self.reads.get() + 1);
            self.read.clone()
        }
    }
    #[test]
    fn exactly_ten_fixed_rows_and_eight_targets_no_paths_in_projection() {
        let reader = FakeReader::new(RegistryRead::Dword(1));
        let result = build_snapshot(true, "windows", &reader);
        assert_eq!(result.settings.len(), 10);
        assert_eq!(reader.reads.get(), 10);
        assert_eq!(reader.metadata_reads.get(), 1);
        assert_eq!(result.targets.len(), 8);
        let ids = result
            .settings
            .iter()
            .map(|row| &row.id)
            .collect::<HashSet<_>>();
        assert_eq!(ids.len(), 10);
        assert_eq!(
            result
                .settings
                .iter()
                .filter(|row| row.category == SettingCategory::Explorer)
                .count(),
            4
        );
        assert_eq!(
            result
                .settings
                .iter()
                .filter(|row| row.category == SettingCategory::AiPolicy)
                .count(),
            6
        );
        let json = serde_json::to_string(&result).unwrap();
        for excluded in [
            "HKEY",
            "Software\\",
            "command",
            "canModify",
            "effectiveState",
            "ms-settings:",
        ] {
            assert!(!json.contains(excluded), "{excluded}");
        }
        assert_eq!(
            result
                .settings
                .iter()
                .find(|row| row.id == "legacy_copilot")
                .unwrap()
                .support,
            SettingSupport::Deprecated
        );
        assert!(
            result
                .settings
                .iter()
                .filter(
                    |row| row.category == SettingCategory::AiPolicy && row.id != "legacy_copilot"
                )
                .all(|row| row.support == SettingSupport::RequiresVerification)
        );
    }
    #[test]
    fn absence_errors_and_unsupported_values_are_never_guessed_false() {
        for (raw, state, value, value_type) in [
            (RegistryRead::Missing, SettingState::Unset, None, None),
            (
                RegistryRead::AccessDenied,
                SettingState::AccessDenied,
                None,
                None,
            ),
            (
                RegistryRead::TypeMismatch("REG_SZ".into()),
                SettingState::TypeMismatch,
                None,
                Some("REG_SZ"),
            ),
            (RegistryRead::Error, SettingState::Error, None, None),
            (
                RegistryRead::Dword(0),
                SettingState::Value,
                Some(0),
                Some("REG_DWORD"),
            ),
            (
                RegistryRead::Dword(9),
                SettingState::UnsupportedValue,
                Some(9),
                Some("REG_DWORD"),
            ),
        ] {
            let result = build_snapshot(true, "windows", &FakeReader::new(raw));
            let row = &result.settings[0];
            assert_eq!(row.state, state);
            assert_eq!(row.value, value);
            assert_eq!(row.value_type.as_deref(), value_type);
        }
        let result = build_snapshot(true, "windows", &FakeReader::new(RegistryRead::Dword(2)));
        assert_eq!(result.settings[0].state, SettingState::UnsupportedValue);
        assert_eq!(result.settings[1].state, SettingState::Value);
        assert_eq!(result.settings[1].value, Some(2));
    }
    #[test]
    fn native_dword_decoder_rejects_type_length_and_truncation_without_false_defaults() {
        assert_eq!(
            decode_dword_value("REG_DWORD", &[2, 0, 0, 0], 4, false),
            RegistryRead::Dword(2)
        );
        assert_eq!(
            decode_dword_value("REG_DWORD", &[0, 0, 0, 0], 4, false),
            RegistryRead::Dword(0)
        );
        for (kind, bytes, length, too_small) in [
            ("REG_SZ", &[0, 0, 0, 0][..], 4, false),
            ("REG_DWORD", &[0, 0][..], 2, false),
            ("REG_DWORD", &[0, 0, 0, 0][..], 4096, true),
            ("REG_QWORD", &[0, 0, 0, 0][..], 8, true),
            ("REG_DWORD", &[0, 0, 0, 0][..], 4, true),
        ] {
            assert_eq!(
                decode_dword_value(kind, bytes, length, too_small),
                RegistryRead::TypeMismatch(kind.into())
            );
        }
    }
    #[test]
    fn native_metadata_decoder_requires_bounded_well_formed_terminated_utf16() {
        let valid = "26100\0"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        assert_eq!(
            decode_metadata_string("REG_SZ", &valid, valid.len()).as_deref(),
            Some("26100")
        );
        assert!(decode_metadata_string("REG_DWORD", &valid, valid.len()).is_none());
        assert!(decode_metadata_string("REG_SZ", &valid, 257).is_none());
        assert!(decode_metadata_string("REG_SZ", &valid, valid.len() - 1).is_none());
        assert!(decode_metadata_string("REG_SZ", &valid, valid.len() - 2).is_none());
        assert!(decode_metadata_string("REG_SZ", &[0x00, 0xD8, 0, 0], 4).is_none());
        assert!(decode_metadata_string("REG_SZ", &[65, 0, 0, 0, 66, 0, 0, 0], 8).is_none());
        assert!(decode_metadata_string("REG_SZ", &[0, 0], 2).is_none());
    }
    #[test]
    fn recall_hives_remain_independent_and_no_effective_precedence_is_derived() {
        let machine = DEFINITIONS
            .iter()
            .find(|row| row.id == "recall_snapshots")
            .unwrap();
        let user = DEFINITIONS
            .iter()
            .find(|row| row.id == "recall_snapshots_user")
            .unwrap();
        assert_eq!(machine.key, user.key);
        assert_eq!(machine.name, user.name);
        assert_eq!(machine.scope, SettingScope::Machine);
        assert_eq!(user.scope, SettingScope::CurrentUser);
    }
    #[test]
    fn unsupported_platform_never_reads_registry_or_calls_launcher() {
        let reader = FakeReader::new(RegistryRead::Error);
        let result = build_snapshot(false, "macos", &reader);
        assert!(!result.supported);
        assert!(result.settings.is_empty());
        assert!(result.targets.is_empty());
        assert_eq!(reader.reads.get(), 0);
        assert_eq!(reader.metadata_reads.get(), 0);
        let calls = Cell::new(0);
        assert!(
            open_target_with(false, "storage", |_| {
                calls.set(calls.get() + 1);
                Ok(())
            })
            .is_err()
        );
        assert!(
            open_target_with(false, "ms-settings:privacy", |_| {
                calls.set(calls.get() + 1);
                Ok(())
            })
            .is_err()
        );
        assert_eq!(calls.get(), 0);
    }
    #[test]
    fn only_fixed_target_ids_launch_and_success_is_only_requested_not_changed() {
        for invalid in [
            "ms-settings:appsfeatures",
            "https://example.com",
            "storage; calc.exe",
            "../control.exe",
            "Storage",
            "",
            "optional_features?x=1",
        ] {
            assert!(open_target_with(true, invalid, |_| panic!("must not launch")).is_err());
        }
        for target in TARGETS {
            let calls = Cell::new(0);
            let result = open_target_with(true, target.id(), |actual| {
                assert_eq!(actual, target);
                calls.set(calls.get() + 1);
                Ok(())
            })
            .unwrap();
            assert_eq!(calls.get(), 1);
            assert!(result.requested);
            assert!(!result.changed);
            if let Some(uri) = target.uri() {
                assert!(uri.starts_with("ms-settings:"));
                assert!(!uri.contains([' ', '?', '&', ';']));
            }
        }
        assert!(
            open_target_with(true, "storage", |_| Err("synthetic launch failure".into())).is_err()
        );
    }
    #[test]
    fn settings_launcher_requires_trusted_local_main_not_remote_or_full_access() {
        assert!(trusted_main_caller(
            "main",
            "http",
            Some("tauri.localhost"),
            None,
            false,
            false
        ));
        assert!(!trusted_main_caller(
            "secondary",
            "http",
            Some("tauri.localhost"),
            None,
            false,
            false
        ));
        assert!(!trusted_main_caller(
            "main",
            "https",
            Some("example.com"),
            None,
            false,
            false
        ));
        assert!(!trusted_main_caller(
            "main",
            "http",
            Some("localhost"),
            Some(1420),
            false,
            false
        ));
        assert!(!trusted_main_caller(
            "main",
            "tauri",
            Some("localhost"),
            None,
            true,
            false
        ));
        assert!(trusted_main_caller(
            "main",
            "http",
            Some("localhost"),
            Some(1420),
            false,
            true
        ));
    }
}
