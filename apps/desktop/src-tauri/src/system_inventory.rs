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
    #[serde(skip)]
    pub cleanup_identity_tokens: Vec<String>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InventoryCancelled;

fn cancel_if_requested<C>(should_cancel: &C) -> Result<(), InventoryCancelled>
where
    C: Fn() -> bool,
{
    if should_cancel() {
        Err(InventoryCancelled)
    } else {
        Ok(())
    }
}

#[cfg(windows)]
pub fn installed_app_inventory_with_cancellation<C>(
    should_cancel: C,
) -> Result<InstalledAppInventory, InventoryCancelled>
where
    C: Fn() -> bool,
{
    use std::collections::HashMap;
    use winreg::enums::{
        HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
    };
    use winreg::{HKEY, RegKey};

    const UNINSTALL_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
    const MAX_SUBKEYS: usize = 50_000;

    let sources: [(HKEY, &'static str, u32); 4] = [
        (HKEY_LOCAL_MACHINE, "machine", KEY_WOW64_64KEY),
        (HKEY_LOCAL_MACHINE, "machine", KEY_WOW64_32KEY),
        (HKEY_CURRENT_USER, "user", KEY_WOW64_64KEY),
        (HKEY_CURRENT_USER, "user", KEY_WOW64_32KEY),
    ];
    let mut applications = HashMap::new();
    let mut issues = Vec::new();
    let mut opened_sources = 0_usize;
    let mut inspected_subkeys = 0_usize;

    cancel_if_requested(&should_cancel)?;
    'sources: for (hive, scope, view) in sources {
        cancel_if_requested(&should_cancel)?;
        let root = RegKey::predef(hive);
        let Ok(uninstall) = root.open_subkey_with_flags(UNINSTALL_KEY, KEY_READ | view) else {
            continue;
        };
        opened_sources += 1;
        cancel_if_requested(&should_cancel)?;

        for subkey_name in uninstall.enum_keys().flatten() {
            cancel_if_requested(&should_cancel)?;
            if inspected_subkeys >= MAX_SUBKEYS {
                issues.push(format!(
                    "설치 앱 레지스트리 항목이 {MAX_SUBKEYS}개를 넘어 나머지는 생략했습니다"
                ));
                break 'sources;
            }
            inspected_subkeys = inspected_subkeys.saturating_add(1);
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
                cleanup_identity_tokens: Vec::new(),
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
        cancel_if_requested(&should_cancel)?;
    }
    cancel_if_requested(&should_cancel)?;

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
    cancel_if_requested(&should_cancel)?;
    if opened_sources == 0 {
        issues.push("설치된 앱 레지스트리에 접근하지 못했습니다".to_owned());
    }

    Ok(InstalledAppInventory {
        supported: opened_sources > 0,
        source: "windowsRegistry",
        estimated_total_bytes,
        applications,
        issues,
    })
}

#[cfg(windows)]
pub fn registry_residue_inventory_with_cancellation<C>(
    should_cancel: C,
) -> Result<RegistryResidueInventory, InventoryCancelled>
where
    C: Fn() -> bool,
{
    use std::path::Path;
    use winreg::enums::{
        HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
    };
    use winreg::{HKEY, RegKey};

    const UNINSTALL_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
    const MAX_CANDIDATES: usize = 100;
    const MAX_SUBKEYS: usize = 50_000;

    let sources: [(HKEY, &'static str, &'static str, u32); 4] = [
        (HKEY_LOCAL_MACHINE, "machine", "HKLM-64", KEY_WOW64_64KEY),
        (HKEY_LOCAL_MACHINE, "machine", "HKLM-32", KEY_WOW64_32KEY),
        (HKEY_CURRENT_USER, "user", "HKCU-64", KEY_WOW64_64KEY),
        (HKEY_CURRENT_USER, "user", "HKCU-32", KEY_WOW64_32KEY),
    ];
    let mut candidates = Vec::new();
    let mut issues = Vec::new();
    let mut opened_sources = 0_usize;
    let mut inspected_subkeys = 0_usize;

    cancel_if_requested(&should_cancel)?;
    'sources: for (hive, scope, hive_label, view) in sources {
        cancel_if_requested(&should_cancel)?;
        let root = RegKey::predef(hive);
        let Ok(uninstall) = root.open_subkey_with_flags(UNINSTALL_KEY, KEY_READ | view) else {
            continue;
        };
        opened_sources = opened_sources.saturating_add(1);
        cancel_if_requested(&should_cancel)?;

        for subkey_name in uninstall.enum_keys().flatten() {
            cancel_if_requested(&should_cancel)?;
            if candidates.len() >= MAX_CANDIDATES {
                issues.push(format!(
                    "깨진 제거 정보 후보가 {MAX_CANDIDATES}개를 넘어 나머지는 생략했습니다"
                ));
                break 'sources;
            }
            if inspected_subkeys >= MAX_SUBKEYS {
                issues.push(format!(
                    "제거 프로그램 레지스트리 항목이 {MAX_SUBKEYS}개를 넘어 나머지는 생략했습니다"
                ));
                break 'sources;
            }
            inspected_subkeys = inspected_subkeys.saturating_add(1);
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
        cancel_if_requested(&should_cancel)?;
    }
    cancel_if_requested(&should_cancel)?;

    candidates.sort_unstable_by(|left, right| {
        left.display_name
            .to_lowercase()
            .cmp(&right.display_name.to_lowercase())
            .then_with(|| left.registry_path.cmp(&right.registry_path))
    });
    cancel_if_requested(&should_cancel)?;
    if opened_sources == 0 {
        issues.push("제거 프로그램 레지스트리에 접근하지 못했습니다".to_owned());
    }

    Ok(RegistryResidueInventory {
        supported: opened_sources > 0,
        candidates,
        issues,
    })
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

#[cfg(target_os = "macos")]
const MAX_MACOS_APP_ENTRIES: usize = 10_000;
#[cfg(target_os = "macos")]
const MAX_MACOS_PLIST_BYTES: u64 = 4 * 1024 * 1024;
#[cfg(target_os = "macos")]
const MAX_MACOS_ISSUES: usize = 100;
#[cfg(target_os = "macos")]
const MACOS_PLIST_READ_CHUNK_BYTES: usize = 16 * 1024;

#[cfg(target_os = "macos")]
#[derive(Debug)]
enum MacApplicationReadError {
    Cancelled,
    Issue(String),
}

#[cfg(target_os = "macos")]
pub fn installed_app_inventory_with_cancellation<C>(
    should_cancel: C,
) -> Result<InstalledAppInventory, InventoryCancelled>
where
    C: Fn() -> bool,
{
    use std::path::PathBuf;

    let mut roots = vec![
        (PathBuf::from("/Applications"), "machine"),
        (PathBuf::from("/System/Applications"), "machine"),
    ];
    if let Some(home) = dirs::home_dir() {
        roots.push((home.join("Applications"), "user"));
    }
    scan_macos_application_roots(roots, MAX_MACOS_APP_ENTRIES, &should_cancel)
}

#[cfg(target_os = "macos")]
fn scan_macos_application_roots<C>(
    roots: Vec<(std::path::PathBuf, &'static str)>,
    max_entries: usize,
    should_cancel: &C,
) -> Result<InstalledAppInventory, InventoryCancelled>
where
    C: Fn() -> bool,
{
    use std::collections::HashMap;
    use std::fs;

    let mut applications = HashMap::new();
    let mut issues = Vec::new();
    let mut opened_roots = 0_usize;
    let mut inspected_entries = 0_usize;

    cancel_if_requested(should_cancel)?;
    'roots: for (root, scope) in roots {
        cancel_if_requested(should_cancel)?;
        if !root.exists() {
            continue;
        }
        let mut entries = match fs::read_dir(&root) {
            Ok(entries) => {
                opened_roots = opened_roots.saturating_add(1);
                entries
            }
            Err(error) => {
                push_macos_issue(
                    &mut issues,
                    MAX_MACOS_ISSUES,
                    format!("{} 앱 폴더를 읽지 못했습니다: {error}", root.display()),
                );
                continue;
            }
        };
        cancel_if_requested(should_cancel)?;

        loop {
            cancel_if_requested(should_cancel)?;
            let Some(entry) = entries.next() else {
                break;
            };
            cancel_if_requested(should_cancel)?;
            if inspected_entries >= max_entries {
                push_macos_issue(
                    &mut issues,
                    MAX_MACOS_ISSUES,
                    format!(
                        "macOS 앱 폴더 직접 항목이 {max_entries}개를 넘어 나머지는 생략했습니다"
                    ),
                );
                break 'roots;
            }
            inspected_entries = inspected_entries.saturating_add(1);

            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    push_macos_issue(
                        &mut issues,
                        MAX_MACOS_ISSUES,
                        format!("{} 앱 항목을 읽지 못했습니다: {error}", root.display()),
                    );
                    continue;
                }
            };
            let path = entry.path();
            if !has_app_extension(&path) {
                continue;
            }
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(error) => {
                    push_macos_issue(
                        &mut issues,
                        MAX_MACOS_ISSUES,
                        format!("{} 앱 종류를 확인하지 못했습니다: {error}", path.display()),
                    );
                    continue;
                }
            };
            if file_type.is_symlink() {
                push_macos_issue(
                    &mut issues,
                    MAX_MACOS_ISSUES,
                    format!(
                        "{} 심볼릭 링크 앱은 따라가지 않고 건너뜁니다",
                        path.display()
                    ),
                );
                continue;
            }
            if !file_type.is_dir() {
                push_macos_issue(
                    &mut issues,
                    MAX_MACOS_ISSUES,
                    format!("{} 항목은 앱 폴더가 아니어서 건너뜁니다", path.display()),
                );
                continue;
            }

            match read_macos_application(&path, scope, should_cancel) {
                Ok((identity, application)) => {
                    applications.entry(identity).or_insert(application);
                }
                Err(MacApplicationReadError::Cancelled) => return Err(InventoryCancelled),
                Err(MacApplicationReadError::Issue(error)) => {
                    push_macos_issue(&mut issues, MAX_MACOS_ISSUES, error);
                }
            }
            cancel_if_requested(should_cancel)?;
        }
        cancel_if_requested(should_cancel)?;
    }

    cancel_if_requested(should_cancel)?;
    let mut applications: Vec<InstalledApplication> = applications.into_values().collect();
    applications.sort_unstable_by(|left, right| {
        left.display_name
            .to_lowercase()
            .cmp(&right.display_name.to_lowercase())
            .then_with(|| left.display_version.cmp(&right.display_version))
            .then_with(|| left.install_location.cmp(&right.install_location))
    });
    cancel_if_requested(should_cancel)?;

    if opened_roots == 0 {
        push_macos_issue(
            &mut issues,
            MAX_MACOS_ISSUES,
            "macOS 앱 폴더에 접근하지 못했습니다".to_owned(),
        );
    }
    cancel_if_requested(should_cancel)?;

    Ok(InstalledAppInventory {
        supported: opened_roots > 0,
        source: "macApplicationBundles",
        estimated_total_bytes: 0,
        applications,
        issues,
    })
}

#[cfg(target_os = "macos")]
fn read_macos_application<C>(
    application_path: &std::path::Path,
    scope: &'static str,
    should_cancel: &C,
) -> Result<(String, InstalledApplication), MacApplicationReadError>
where
    C: Fn() -> bool,
{
    use std::fs::{self, File};
    use std::io::{Cursor, Read};

    let check_cancel =
        || cancel_if_requested(should_cancel).map_err(|_| MacApplicationReadError::Cancelled);
    check_cancel()?;
    let application_metadata = fs::symlink_metadata(application_path).map_err(|error| {
        MacApplicationReadError::Issue(format!(
            "{} 앱 종류를 확인하지 못했습니다: {error}",
            application_path.display()
        ))
    })?;
    if application_metadata.file_type().is_symlink() {
        return Err(MacApplicationReadError::Issue(format!(
            "{} 심볼릭 링크 앱은 따라가지 않고 건너뜁니다",
            application_path.display()
        )));
    }
    if !application_metadata.is_dir() {
        return Err(MacApplicationReadError::Issue(format!(
            "{} 항목은 앱 폴더가 아니어서 건너뜁니다",
            application_path.display()
        )));
    }
    check_cancel()?;

    let plist_path = application_path.join("Contents").join("Info.plist");
    let file = File::open(&plist_path).map_err(|error| {
        MacApplicationReadError::Issue(format!(
            "{} 앱 정보를 읽지 못했습니다: {error}",
            application_path.display()
        ))
    })?;
    check_cancel()?;
    let metadata = file.metadata().map_err(|error| {
        MacApplicationReadError::Issue(format!(
            "{} 앱 정보 크기를 확인하지 못했습니다: {error}",
            application_path.display()
        ))
    })?;
    if !metadata.is_file() {
        return Err(MacApplicationReadError::Issue(format!(
            "{} 앱 정보가 일반 파일이 아니어서 건너뜁니다",
            application_path.display()
        )));
    }
    if metadata.len() > MAX_MACOS_PLIST_BYTES {
        return Err(MacApplicationReadError::Issue(format!(
            "{} 앱 정보가 {} MiB 제한을 넘어 건너뜁니다",
            application_path.display(),
            MAX_MACOS_PLIST_BYTES / (1024 * 1024)
        )));
    }
    check_cancel()?;

    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    let mut bounded_reader = file.take(MAX_MACOS_PLIST_BYTES.saturating_add(1));
    let mut chunk = [0_u8; MACOS_PLIST_READ_CHUNK_BYTES];
    loop {
        check_cancel()?;
        let read = bounded_reader.read(&mut chunk).map_err(|error| {
            MacApplicationReadError::Issue(format!(
                "{} 앱 정보를 읽지 못했습니다: {error}",
                application_path.display()
            ))
        })?;
        check_cancel()?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..read]);
        if bytes.len() as u64 > MAX_MACOS_PLIST_BYTES {
            return Err(MacApplicationReadError::Issue(format!(
                "{} 앱 정보가 읽는 동안 {} MiB 제한을 넘어 건너뜁니다",
                application_path.display(),
                MAX_MACOS_PLIST_BYTES / (1024 * 1024)
            )));
        }
    }
    check_cancel()?;
    let plist = plist::Value::from_reader(Cursor::new(bytes)).map_err(|error| {
        MacApplicationReadError::Issue(format!(
            "{} 앱 정보를 해석하지 못했습니다: {error}",
            application_path.display()
        ))
    })?;
    check_cancel()?;

    let application_metadata_after = fs::symlink_metadata(application_path).map_err(|error| {
        MacApplicationReadError::Issue(format!(
            "{} 앱을 다시 확인하지 못했습니다: {error}",
            application_path.display()
        ))
    })?;
    if application_metadata_after.file_type().is_symlink() || !application_metadata_after.is_dir() {
        return Err(MacApplicationReadError::Issue(format!(
            "{} 앱 종류가 확인 중 바뀌어 결과에서 제외했습니다",
            application_path.display()
        )));
    }

    let dictionary = plist.as_dictionary().ok_or_else(|| {
        MacApplicationReadError::Issue(format!(
            "{} 앱 정보 형식이 올바르지 않습니다",
            application_path.display()
        ))
    })?;
    let plist_string = |key: &str| {
        dictionary
            .get(key)
            .and_then(plist::Value::as_string)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    };

    let fallback_name = application_path
        .file_stem()
        .and_then(|name| name.to_str())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .ok_or_else(|| {
            MacApplicationReadError::Issue(format!(
                "{} 앱 이름을 확인하지 못했습니다",
                application_path.display()
            ))
        })?
        .to_owned();
    let display_name = plist_string("CFBundleDisplayName")
        .or_else(|| plist_string("CFBundleName"))
        .unwrap_or(fallback_name);
    let display_version =
        plist_string("CFBundleShortVersionString").or_else(|| plist_string("CFBundleVersion"));
    let bundle_id = plist_string("CFBundleIdentifier").filter(|value| valid_bundle_id(value));
    let identity = macos_application_identity(
        &display_name,
        display_version.as_deref(),
        bundle_id.as_deref(),
        application_path,
    );
    check_cancel()?;

    Ok((
        identity,
        InstalledApplication {
            display_name,
            display_version,
            publisher: None,
            install_location: Some(application_path.to_string_lossy().into_owned()),
            estimated_bytes: None,
            registry_scope: scope,
            cleanup_identity_tokens: bundle_id.into_iter().collect(),
        },
    ))
}

#[cfg(target_os = "macos")]
fn has_app_extension(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("app"))
}

#[cfg(target_os = "macos")]
fn valid_bundle_id(value: &str) -> bool {
    let mut components = value.split('.');
    let first = components.next().unwrap_or_default();
    let mut component_count = 1_usize;
    if first.is_empty()
        || !first
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return false;
    }
    for component in components {
        component_count = component_count.saturating_add(1);
        if component.is_empty()
            || !component
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '-')
        {
            return false;
        }
    }
    component_count >= 2
}

#[cfg(target_os = "macos")]
fn macos_application_identity(
    display_name: &str,
    display_version: Option<&str>,
    bundle_id: Option<&str>,
    path: &std::path::Path,
) -> String {
    format!(
        "{}|{}|{}|{}",
        bundle_id.unwrap_or_default().to_lowercase(),
        display_name.to_lowercase(),
        display_version.unwrap_or_default().to_lowercase(),
        path.to_string_lossy(),
    )
}

#[cfg(target_os = "macos")]
fn push_macos_issue(issues: &mut Vec<String>, max_issues: usize, message: String) {
    if issues.len() < max_issues {
        issues.push(message);
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn installed_app_inventory_with_cancellation<C>(
    should_cancel: C,
) -> Result<InstalledAppInventory, InventoryCancelled>
where
    C: Fn() -> bool,
{
    cancel_if_requested(&should_cancel)?;
    Ok(InstalledAppInventory {
        supported: false,
        source: "notAvailable",
        estimated_total_bytes: 0,
        applications: Vec::new(),
        issues: Vec::new(),
    })
}

#[cfg(not(windows))]
pub fn registry_residue_inventory_with_cancellation<C>(
    should_cancel: C,
) -> Result<RegistryResidueInventory, InventoryCancelled>
where
    C: Fn() -> bool,
{
    cancel_if_requested(&should_cancel)?;
    Ok(RegistryResidueInventory {
        supported: false,
        candidates: Vec::new(),
        issues: Vec::new(),
    })
}

#[cfg(test)]
mod cancellation_tests {
    use super::*;

    #[test]
    fn inventory_entry_points_honor_immediate_cancellation() {
        assert!(installed_app_inventory_with_cancellation(|| true).is_err());
        assert!(registry_residue_inventory_with_cancellation(|| true).is_err());
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
            cleanup_identity_tokens: Vec::new(),
        };
        let lower = InstalledApplication {
            display_name: "example app".to_owned(),
            display_version: Some("1.0".to_owned()),
            publisher: Some("publisher".to_owned()),
            install_location: Some(r"c:\program files\example".to_owned()),
            estimated_bytes: Some(10),
            registry_scope: "machine",
            cleanup_identity_tokens: Vec::new(),
        };

        assert_eq!(application_identity(&upper), application_identity(&lower));
    }

    #[test]
    fn reads_the_current_windows_app_inventory() {
        let inventory = installed_app_inventory_with_cancellation(|| false)
            .expect("read installed app inventory");

        assert!(inventory.supported);
        assert_eq!(inventory.source, "windowsRegistry");
        assert!(!inventory.applications.is_empty());
    }

    #[test]
    fn windows_inventory_rechecks_cancellation_during_registry_walk() {
        use std::cell::Cell;

        let checks = Cell::new(0_usize);
        let should_cancel = || {
            let current = checks.get();
            checks.set(current.saturating_add(1));
            current >= 3
        };

        assert!(installed_app_inventory_with_cancellation(should_cancel).is_err());
        assert!(checks.get() >= 4);
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

#[cfg(all(test, target_os = "macos"))]
mod macos_tests {
    use super::*;
    use std::fs;

    #[test]
    fn reads_binary_application_plist_and_keeps_bundle_id_internal() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let application = temp.path().join("Sample.app");
        let contents = application.join("Contents");
        fs::create_dir_all(&contents).expect("create application bundle");

        let mut dictionary = plist::Dictionary::new();
        dictionary.insert(
            "CFBundleDisplayName".to_owned(),
            plist::Value::String("Sample App".to_owned()),
        );
        dictionary.insert(
            "CFBundleShortVersionString".to_owned(),
            plist::Value::String("1.2.3".to_owned()),
        );
        dictionary.insert(
            "CFBundleIdentifier".to_owned(),
            plist::Value::String("com.example.sample".to_owned()),
        );
        plist::Value::Dictionary(dictionary)
            .to_file_binary(contents.join("Info.plist"))
            .expect("write binary plist");

        let (identity, parsed) = read_macos_application(&application, "user", &|| false)
            .expect("read application bundle");

        assert_eq!(parsed.display_name, "Sample App");
        assert_eq!(parsed.display_version.as_deref(), Some("1.2.3"));
        assert_eq!(parsed.registry_scope, "user");
        assert_eq!(
            parsed.cleanup_identity_tokens,
            vec!["com.example.sample".to_owned()]
        );
        assert!(identity.starts_with("com.example.sample|sample app|1.2.3"));
        let serialized = serde_json::to_value(parsed).expect("serialize application");
        assert!(serialized.get("cleanupIdentityTokens").is_none());
    }

    #[test]
    fn validates_reverse_dns_bundle_identifiers() {
        assert!(valid_bundle_id("com.example.Sample-App"));
        assert!(!valid_bundle_id("SampleApp"));
        assert!(!valid_bundle_id("com..sample"));
        assert!(!valid_bundle_id("com.example.sample_app"));
    }

    #[test]
    fn application_identity_keeps_distinct_case_sensitive_install_paths() {
        let system_copy = macos_application_identity(
            "Sample App",
            Some("1.2.3"),
            Some("com.example.sample"),
            std::path::Path::new("/Applications/Sample.app"),
        );
        let user_copy = macos_application_identity(
            "Sample App",
            Some("1.2.3"),
            Some("com.example.sample"),
            std::path::Path::new("/Users/test/Applications/Sample.app"),
        );
        let differently_cased_copy = macos_application_identity(
            "Sample App",
            Some("1.2.3"),
            Some("com.example.sample"),
            std::path::Path::new("/Applications/sample.app"),
        );

        assert_ne!(system_copy, user_copy);
        assert_ne!(system_copy, differently_cased_copy);
        assert!(system_copy.ends_with("|/Applications/Sample.app"));
    }

    #[test]
    fn oversized_plist_is_rejected_before_parsing() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let application = temp.path().join("Oversized.app");
        let contents = application.join("Contents");
        fs::create_dir_all(&contents).expect("create application bundle");
        fs::write(
            contents.join("Info.plist"),
            vec![0_u8; MAX_MACOS_PLIST_BYTES as usize + 1],
        )
        .expect("write oversized plist");

        let error = read_macos_application(&application, "user", &|| false)
            .expect_err("oversized plist must be rejected");

        assert!(
            matches!(error, MacApplicationReadError::Issue(message) if message.contains("제한"))
        );
    }

    #[test]
    fn app_symlink_is_reported_and_never_followed() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().expect("create temp directory");
        let scan_root = temp.path().join("scan");
        let real_application = temp.path().join("outside").join("Real.app");
        fs::create_dir_all(real_application.join("Contents"))
            .expect("create real application bundle");
        fs::create_dir_all(&scan_root).expect("create scan root");
        symlink(&real_application, scan_root.join("Linked.app")).expect("create application link");

        let inventory =
            scan_macos_application_roots(vec![(scan_root, "user")], MAX_MACOS_APP_ENTRIES, &|| {
                false
            })
            .expect("scan application root");

        assert!(inventory.applications.is_empty());
        assert!(
            inventory
                .issues
                .iter()
                .any(|issue| issue.contains("심볼릭 링크"))
        );
    }

    #[test]
    fn direct_entry_limit_stops_oversized_application_root() {
        let temp = tempfile::tempdir().expect("create temp directory");
        fs::write(temp.path().join("first"), b"one").expect("write first entry");
        fs::write(temp.path().join("second"), b"two").expect("write second entry");

        let inventory =
            scan_macos_application_roots(vec![(temp.path().to_path_buf(), "user")], 1, &|| false)
                .expect("scan bounded application root");

        assert!(inventory.issues.iter().any(|issue| issue.contains("1개")));
    }

    #[test]
    fn plist_pipeline_checks_cancellation_around_bounded_reads() {
        use std::cell::Cell;

        let temp = tempfile::tempdir().expect("create temp directory");
        let application = temp.path().join("Cancelled.app");
        let contents = application.join("Contents");
        fs::create_dir_all(&contents).expect("create application bundle");
        fs::write(
            contents.join("Info.plist"),
            vec![0_u8; MACOS_PLIST_READ_CHUNK_BYTES * 2],
        )
        .expect("write plist fixture");
        let checks = Cell::new(0_usize);
        let should_cancel = || {
            let current = checks.get();
            checks.set(current.saturating_add(1));
            current >= 5
        };

        let result = read_macos_application(&application, "user", &should_cancel);

        assert!(matches!(result, Err(MacApplicationReadError::Cancelled)));
        assert!(checks.get() >= 6);
    }
}
