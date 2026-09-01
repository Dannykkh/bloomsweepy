use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Manager, WebviewWindow, WindowEvent, Wry};

const OPEN_MENU_ID: &str = "tray-open";
const STATUS_MENU_ID: &str = "tray-status";
const QUIT_MENU_ID: &str = "tray-quit";
const IDLE_LABEL: &str = "대기";
const BUSY_LABEL: &str = "검사 또는 검색 진행 중";
const MAX_SETUP_ERROR_CHARS: usize = 240;

struct WindowsTrayState {
    status_item: MenuItem<Wry>,
    _tray: TrayIcon<Wry>,
}

pub(crate) fn setup(app: &mut App<Wry>) -> tauri::Result<()> {
    let main_window = app
        .get_webview_window("main")
        .ok_or_else(|| startup_error("main 창을 찾지 못해 트레이를 준비할 수 없습니다"))?;
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| startup_error("트레이 아이콘을 찾지 못했습니다"))?;

    let open_item = MenuItem::with_id(app, OPEN_MENU_ID, "BroomSweepy 열기", true, None::<&str>)?;
    let status_item = MenuItem::with_id(app, STATUS_MENU_ID, IDLE_LABEL, false, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, QUIT_MENU_ID, "종료", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_item, &status_item, &separator, &quit_item])?;

    let tray = TrayIconBuilder::new()
        .icon(icon)
        .tooltip("BroomSweepy")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            OPEN_MENU_ID => show_main_window(app),
            QUIT_MENU_ID => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    if !app.manage(WindowsTrayState {
        status_item,
        _tray: tray,
    }) {
        return Err(startup_error("트레이 상태가 이미 등록되어 있습니다"));
    }

    hide_main_window_on_close(main_window);
    Ok(())
}

pub(crate) fn set_busy(app: &AppHandle<Wry>, busy: bool) {
    if let Some(tray) = app.try_state::<WindowsTrayState>() {
        let _ = tray
            .status_item
            .set_text(if busy { BUSY_LABEL } else { IDLE_LABEL });
    }
}

pub(crate) fn log_setup_failure(error: &tauri::Error) {
    let message = error.to_string().replace(['\r', '\n'], " ");
    let mut characters = message.chars();
    let summary: String = characters.by_ref().take(MAX_SETUP_ERROR_CHARS).collect();
    let suffix = if characters.next().is_some() {
        "..."
    } else {
        ""
    };
    eprintln!("BroomSweepy 트레이를 준비하지 못해 일반 창으로 실행합니다: {summary}{suffix}");
}

fn show_main_window(app: &AppHandle<Wry>) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    if window.is_minimized().unwrap_or(false) {
        let _ = window.unminimize();
    }
    let _ = window.show();
    let _ = window.set_focus();
}

fn hide_main_window_on_close(window: WebviewWindow<Wry>) {
    let window_to_hide = window.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event
            && window_to_hide.hide().is_ok()
        {
            api.prevent_close();
        }
    });
}

fn startup_error(message: &str) -> tauri::Error {
    std::io::Error::other(message).into()
}
