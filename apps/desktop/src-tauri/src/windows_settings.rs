//! App-owned Windows stored-value inspection and fixed local settings links.
//! File/trash grants never become OS-write authority; no write path exists.
mod model;
#[cfg(windows)]
mod native;

#[cfg(not(windows))]
use model::{RegistryRead, RegistryReader, SettingDefinition, WindowsMetadata};
pub(crate) use model::{WindowsSettingsOpenResult, WindowsSettingsSnapshot};
use tauri::{AppHandle, WebviewWindow};

#[cfg(not(windows))]
struct UnsupportedReader;
#[cfg(not(windows))]
impl RegistryReader for UnsupportedReader {
    fn metadata(&self) -> WindowsMetadata {
        unreachable!("unsupported platform must not inspect Windows metadata")
    }
    fn read(&self, _: &SettingDefinition) -> RegistryRead {
        unreachable!("unsupported platform must not inspect Windows registry")
    }
}

fn collect_snapshot() -> WindowsSettingsSnapshot {
    #[cfg(windows)]
    {
        model::build_snapshot(true, "windows", &native::NativeReader)
    }
    #[cfg(not(windows))]
    {
        model::build_snapshot(false, std::env::consts::OS, &UnsupportedReader)
    }
}

pub(crate) async fn inspect() -> Result<WindowsSettingsSnapshot, String> {
    tauri::async_runtime::spawn_blocking(collect_snapshot)
        .await
        .map_err(|error| format!("Windows 설정 조회 작업이 중단되었습니다: {error}"))
}

#[tauri::command]
pub(crate) async fn get_windows_settings() -> Result<WindowsSettingsSnapshot, String> {
    inspect().await
}

#[tauri::command]
pub(crate) async fn open_windows_settings_target(
    window: WebviewWindow,
    app: AppHandle,
    target_id: String,
) -> Result<WindowsSettingsOpenResult, String> {
    let url = window
        .url()
        .map_err(|_| "앱 창의 출처를 확인하지 못했습니다")?;
    if !model::trusted_main_caller(
        window.label(),
        url.scheme(),
        url.host_str(),
        url.port(),
        !url.username().is_empty() || url.password().is_some(),
        cfg!(debug_assertions),
    ) {
        return Err("Windows 설정 연결은 앱의 로컬 기본 창에서만 요청할 수 있습니다".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        model::open_target_with(cfg!(windows), &target_id, |target| {
            #[cfg(windows)]
            {
                use tauri_plugin_opener::OpenerExt;
                if target == model::Target::FolderOptions {
                    return native::launch_folder_options();
                }
                let uri = target
                    .uri()
                    .ok_or("고정 Windows 설정 연결을 찾지 못했습니다")?;
                app.opener().open_url(uri, None::<&str>).map_err(|error| {
                    format!("Windows 설정 연결 요청을 전달하지 못했습니다: {error}")
                })
            }
            #[cfg(not(windows))]
            {
                let _ = (&app, target);
                Err("이 운영체제에서는 Windows 설정을 실행하지 않습니다".into())
            }
        })
    })
    .await
    .map_err(|error| format!("Windows 설정 연결 작업이 중단되었습니다: {error}"))?
}

#[cfg(test)]
mod tests {
    #[cfg(not(windows))]
    #[test]
    fn actual_non_windows_adapter_never_calls_registry_and_has_no_targets() {
        let snapshot = super::collect_snapshot();
        assert!(!snapshot.supported);
        assert!(snapshot.settings.is_empty());
        assert!(snapshot.targets.is_empty());
        assert!(snapshot.os_build.is_none());
    }
}
