use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledAppInventory {
    pub supported: bool,
    pub source: &'static str,
    pub estimated_total_bytes: u64,
    pub applications: Vec<InstalledApplication>,
    pub issues: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledApplication {
    pub display_name: String,
    pub display_version: Option<String>,
    pub publisher: Option<String>,
    pub install_location: Option<String>,
    pub estimated_bytes: Option<u64>,
    pub registry_scope: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryResidueInventory {
    pub supported: bool,
    pub candidates: Vec<RegistryResidueCandidate>,
    pub issues: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryResidueCandidate {
    pub display_name: String,
    pub registry_path: String,
    pub registry_scope: &'static str,
    pub evidence: Vec<String>,
}

#[cfg(windows)]
pub fn installed_app_inventory() -> InstalledAppInventory {
    use std::collections::HashMap;
    use winreg::enums::{
        HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
    };
    use winreg::{HKEY, RegKey};

    const UNINSTALL_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";

    let sources: [(HKEY, &'static str, u32); 4] = [
        (HKEY_LOCAL_MACHINE, "machine", KEY_WOW64_64KEY),
        (HKEY_LOCAL_MACHINE, "machine", KEY_WOW64_32KEY),
        (HKEY_CURRENT_USER, "user", KEY_WOW64_64KEY),
        (HKEY_CURRENT_USER, "user", KEY_WOW64_32KEY),
    ];
    let mut applications = HashMap::new();
    let mut opened_sources = 0_usize;

    for (hive, scope, view) in sources {
        let root = RegKey::predef(hive);
        let Ok(uninstall) = root.open_subkey_with_flags(UNINSTALL_KEY, KEY_READ | view) else {
            continue;
        };
        opened_sources += 1;

        for subkey_name in uninstall.enum_keys().flatten() {
            let Ok(subkey) = uninstall.open_subkey_with_flags(&subkey_name, KEY_READ | view) else {
                continue;
            };
            if subkey
                .get_value::<u32, _>("SystemComponent")
                .is_ok_and(|value| value == 1)
                || subkey.get_raw_value("ParentKeyName").is_ok()
                || subkey
                    .get_value::<String, _>("ReleaseType")
                    .is_ok_and(|value| !value.trim().is_empty())
            {
                continue;
            }

            let Ok(display_name) = subkey.get_value::<String, _>("DisplayName") else {
                continue;
            };
            let display_name = display_name.trim().to_owned();
            if display_name.is_empty() {
                continue;
            }

            let application = InstalledApplication {
                display_name,
                display_version: optional_string(&subkey, "DisplayVersion"),
                publisher: optional_string(&subkey, "Publisher"),
                install_location: optional_string(&subkey, "InstallLocation"),
                estimated_bytes: estimated_size_bytes(&subkey),
                registry_scope: scope,
            };
            let identity = application_identity(&application);
            match applications.entry(identity) {
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(application);
                }
                std::collections::hash_map::Entry::Occupied(mut entry) => {
                    let current_size = entry.get().estimated_bytes.unwrap_or_default();
                    if application.estimated_bytes.unwrap_or_default() > current_size {
                        entry.insert(application);
                    }
                }
            }
        }
    }

    let mut applications: Vec<InstalledApplication> = applications.into_values().collect();
    applications.sort_unstable_by(|left, right| {
        left.display_name
            .to_lowercase()
            .cmp(&right.display_name.to_lowercase())
            .then_with(|| left.display_version.cmp(&right.display_version))
    });
    let estimated_total_bytes = applications.iter().fold(0_u64, |total, application| {
        total.saturating_add(application.estimated_bytes.unwrap_or_default())
    });
    let issues = if opened_sources == 0 {
        vec!["설치된 앱 레지스트리에 접근하지 못했습니다".to_owned()]
    } else {
        Vec::new()
    };

    InstalledAppInventory {
        supported: opened_sources > 0,
        source: "windowsRegistry",
        estimated_total_bytes,
        applications,
        issues,
    }
}

#[cfg(windows)]
pub fn registry_residue_inventory() -> RegistryResidueInventory {
    use std::path::Path;
    use winreg::enums::{
        HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
    };
    use winreg::{HKEY, RegKey};

    const UNINSTALL_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
    const MAX_CANDIDATES: usize = 100;

    let sources: [(HKEY, &'static str, &'static str, u32); 4] = [
        (HKEY_LOCAL_MACHINE, "machine", "HKLM-64", KEY_WOW64_64KEY),
        (HKEY_LOCAL_MACHINE, "machine", "HKLM-32", KEY_WOW64_32KEY),
        (HKEY_CURRENT_USER, "user", "HKCU-64", KEY_WOW64_64KEY),
        (HKEY_CURRENT_USER, "user", "HKCU-32", KEY_WOW64_32KEY),
    ];
    let mut candidates = Vec::new();
    let mut issues = Vec::new();
    let mut opened_sources = 0_usize;

    'sources: for (hive, scope, hive_label, view) in sources {
        let root = RegKey::predef(hive);
        let Ok(uninstall) = root.open_subkey_with_flags(UNINSTALL_KEY, KEY_READ | view) else {
            continue;
        };
        opened_sources = opened_sources.saturating_add(1);

        for subkey_name in uninstall.enum_keys().flatten() {
            if candidates.len() >= MAX_CANDIDATES {
                issues.push(format!(
                    "깨진 제거 정보 후보가 {MAX_CANDIDATES}개를 넘어 나머지는 생략했습니다"
                ));
                break 'sources;
            }
            let Ok(subkey) = uninstall.open_subkey_with_flags(&subkey_name, KEY_READ | view) else {
                continue;
            };
            if subkey
                .get_value::<u32, _>("SystemComponent")
                .is_ok_and(|value| value == 1)
                || subkey.get_raw_value("ParentKeyName").is_ok()
                || subkey
                    .get_value::<String, _>("ReleaseType")
                    .is_ok_and(|value| !value.trim().is_empty())
            {
                continue;
            }
            let Ok(display_name) = subkey.get_value::<String, _>("DisplayName") else {
                continue;
            };
            let display_name = display_name.trim().to_owned();
            if display_name.is_empty() {
                continue;
            }

            let mut evidence = Vec::new();
            if let Some(path) = optional_string(&subkey, "InstallLocation")
                .filter(|path| looks_like_absolute_path(path))
                && !Path::new(&path).exists()
            {
                evidence.push(format!("설치 위치가 존재하지 않음: {path}"));
            }
            if let Some(path) = optional_string(&subkey, "DisplayIcon")
                .and_then(|value| executable_path_from_registry_value(&value))
                && !path.exists()
            {
                evidence.push(format!(
                    "표시 아이콘 대상이 존재하지 않음: {}",
                    path.display()
                ));
            }
            if let Some(path) = optional_string(&subkey, "UninstallString")
                .and_then(|value| executable_path_from_registry_value(&value))
                && !is_shared_windows_command(&path)
                && !path.exists()
            {
                evidence.push(format!("제거 프로그램이 존재하지 않음: {}", path.display()));
            }

            if evidence.len() >= 2 {
                candidates.push(RegistryResidueCandidate {
                    display_name,
                    registry_path: format!(r"{hive_label}\{UNINSTALL_KEY}\{subkey_name}"),
                    registry_scope: scope,
                    evidence,
                });
            }
        }
    }

    candidates.sort_unstable_by(|left, right| {
        left.display_name
            .to_lowercase()
            .cmp(&right.display_name.to_lowercase())
            .then_with(|| left.registry_path.cmp(&right.registry_path))
    });
    if opened_sources == 0 {
        issues.push("제거 프로그램 레지스트리에 접근하지 못했습니다".to_owned());
    }

    RegistryResidueInventory {
        supported: opened_sources > 0,
        candidates,
        issues,
    }
}

#[cfg(windows)]
fn optional_string(key: &winreg::RegKey, name: &str) -> Option<String> {
    key.get_value::<String, _>(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

#[cfg(windows)]
fn estimated_size_bytes(key: &winreg::RegKey) -> Option<u64> {
    key.get_value::<u32, _>("EstimatedSize")
        .ok()
        .map(|value| u64::from(value).saturating_mul(1_024))
        .or_else(|| {
            key.get_value::<u64, _>("EstimatedSize")
                .ok()
                .map(|value| value.saturating_mul(1_024))
        })
}

#[cfg(windows)]
fn application_identity(application: &InstalledApplication) -> String {
    format!(
        "{}|{}|{}|{}",
        application.display_name.to_lowercase(),
        application
            .display_version
            .as_deref()
            .unwrap_or_default()
            .to_lowercase(),
        application
            .publisher
            .as_deref()
            .unwrap_or_default()
            .to_lowercase(),
        application
            .install_location
            .as_deref()
            .unwrap_or_default()
            .to_lowercase(),
    )
}

#[cfg(windows)]
fn looks_like_absolute_path(value: &str) -> bool {
    std::path::Path::new(value.trim_matches('"')).is_absolute()
}

#[cfg(windows)]
fn executable_path_from_registry_value(value: &str) -> Option<std::path::PathBuf> {
    use std::path::PathBuf;

    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.contains('%') {
        return None;
    }
    let candidate = if let Some(rest) = trimmed.strip_prefix('"') {
        let end = rest.find('"')?;
        &rest[..end]
    } else {
        let lower = trimmed.to_ascii_lowercase();
        let extension_end = [".exe", ".ico", ".dll"]
            .iter()
            .filter_map(|extension| lower.find(extension).map(|index| index + extension.len()))
            .min()?;
        &trimmed[..extension_end]
    };
    let candidate = candidate
        .trim()
        .trim_end_matches(|character: char| character == ',' || character.is_ascii_digit());
    let path = PathBuf::from(candidate);
    path.is_absolute().then_some(path)
}

#[cfg(windows)]
fn is_shared_windows_command(path: &std::path::Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            matches!(
                name.to_ascii_lowercase().as_str(),
                "msiexec.exe" | "rundll32.exe" | "regsvr32.exe"
            )
        })
}

#[cfg(not(windows))]
pub fn installed_app_inventory() -> InstalledAppInventory {
    InstalledAppInventory {
        supported: false,
        source: "notAvailable",
        estimated_total_bytes: 0,
        applications: Vec::new(),
        issues: Vec::new(),
    }
}

#[cfg(not(windows))]
pub fn registry_residue_inventory() -> RegistryResidueInventory {
    RegistryResidueInventory {
        supported: false,
        candidates: Vec::new(),
        issues: Vec::new(),
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn application_identity_is_case_insensitive() {
        let upper = InstalledApplication {
            display_name: "Example App".to_owned(),
            display_version: Some("1.0".to_owned()),
            publisher: Some("Publisher".to_owned()),
            install_location: Some(r"C:\Program Files\Example".to_owned()),
            estimated_bytes: Some(10),
            registry_scope: "machine",
        };
        let lower = InstalledApplication {
            display_name: "example app".to_owned(),
            display_version: Some("1.0".to_owned()),
            publisher: Some("publisher".to_owned()),
            install_location: Some(r"c:\program files\example".to_owned()),
            estimated_bytes: Some(10),
            registry_scope: "machine",
        };

        assert_eq!(application_identity(&upper), application_identity(&lower));
    }

    #[test]
    fn reads_the_current_windows_app_inventory() {
        let inventory = installed_app_inventory();

        assert!(inventory.supported);
        assert_eq!(inventory.source, "windowsRegistry");
        assert!(!inventory.applications.is_empty());
    }

    #[test]
    fn extracts_absolute_executable_paths_from_registry_values() {
        let quoted = executable_path_from_registry_value(
            r#""C:\Program Files\Example\uninstall.exe" /quiet"#,
        )
        .expect("quoted path");
        let icon = executable_path_from_registry_value(r"C:\Program Files\Example\example.exe,0")
            .expect("icon path");

        assert_eq!(
            quoted,
            std::path::PathBuf::from(r"C:\Program Files\Example\uninstall.exe")
        );
        assert_eq!(
            icon,
            std::path::PathBuf::from(r"C:\Program Files\Example\example.exe")
        );
        assert!(executable_path_from_registry_value(r"%ProgramFiles%\Example\app.exe").is_none());
    }
}
