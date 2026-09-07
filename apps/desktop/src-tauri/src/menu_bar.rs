use serde::Serialize;
use tauri::AppHandle;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub(crate) use macos::{set_language, setup, shutdown};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MenuBarSettings {
    supported: bool,
    available: bool,
    show_memory_percent: bool,
}

#[tauri::command]
pub(crate) async fn get_menu_bar_settings(app: AppHandle) -> Result<MenuBarSettings, String> {
    #[cfg(target_os = "macos")]
    return macos::settings(app, None).await;
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Ok(MenuBarSettings {
            supported: false,
            available: false,
            show_memory_percent: false,
        })
    }
}

#[tauri::command]
pub(crate) fn open_menu_bar_panel(app: AppHandle) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    return macos::show_panel(&app);
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Err("Menu bar status is only available on macOS".into())
    }
}

#[tauri::command]
pub(crate) async fn set_menu_bar_memory_percent(
    app: AppHandle,
    enabled: bool,
) -> Result<MenuBarSettings, String> {
    #[cfg(target_os = "macos")]
    return macos::settings(app, Some(enabled)).await;
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, enabled);
        Err("Menu bar status is only available on macOS".into())
    }
}

#[cfg(any(target_os = "macos", test))]
fn usage_percent(used: u64, total: u64) -> Option<f64> {
    (total > 0).then(|| ((used as f64 / total as f64) * 100.0).clamp(0.0, 100.0))
}

#[cfg(any(target_os = "macos", test))]
fn valid_cpu(value: f32) -> Option<f64> {
    value
        .is_finite()
        .then(|| f64::from(value.clamp(0.0, 100.0)))
}

#[cfg(any(target_os = "macos", test))]
fn stale(captured_at: u64, now: u64) -> bool {
    captured_at == 0 || now < captured_at || now.saturating_sub(captured_at) > 30_000
}

#[cfg(any(target_os = "macos", test))]
fn title_percent(show: bool, value: Option<f64>, is_stale: bool) -> String {
    if !show {
        return String::new();
    }
    if is_stale {
        return " —".into();
    }
    value
        .map(|value| format!(" {value:.0}%"))
        .unwrap_or_else(|| " —".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_metrics_are_not_zero_and_percentages_are_bounded() {
        assert_eq!(usage_percent(1, 0), None);
        assert_eq!(usage_percent(2, 1), Some(100.0));
        assert_eq!(usage_percent(1, 4), Some(25.0));
        assert_eq!(valid_cpu(f32::NAN), None);
        assert_eq!(valid_cpu(f32::INFINITY), None);
        assert_eq!(valid_cpu(-1.0), Some(0.0));
    }

    #[test]
    fn hidden_percent_keeps_icon_and_stale_data_is_not_current() {
        assert_eq!(title_percent(false, Some(50.0), false), "");
        assert_eq!(title_percent(true, Some(50.0), false), " 50%");
        assert_eq!(title_percent(true, Some(50.0), true), " —");
        assert!(stale(0, 1));
        assert!(stale(100, 99));
        assert!(!stale(100, 30_100));
        assert!(stale(100, 30_101));
    }
}
