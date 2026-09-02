use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Manager, WebviewWindow, WindowEvent, Wry};

const OPEN_MENU_ID: &str = "tray-open";
const STATUS_MENU_ID: &str = "tray-status";
const QUIT_MENU_ID: &str = "tray-quit";
const MAX_SETUP_ERROR_CHARS: usize = 240;

struct WindowsTrayState {
    open_item: MenuItem<Wry>,
    status_item: MenuItem<Wry>,
    quit_item: MenuItem<Wry>,
    _tray: TrayIcon<Wry>,
    display: Mutex<TrayDisplay>,
    updates: Mutex<()>,
}

#[derive(Clone, Copy)]
enum TrayLanguage {
    English,
    Korean,
    Japanese,
    SimplifiedChinese,
}

#[derive(Clone, Copy)]
struct TrayDisplay {
    language: TrayLanguage,
    busy: bool,
}

#[derive(Clone, Copy)]
struct TrayLabels {
    open: &'static str,
    idle: &'static str,
    busy: &'static str,
    quit: &'static str,
}

impl TrayLanguage {
    fn from_code(code: &str) -> Option<Self> {
        match code {
            "en" => Some(Self::English),
            "ko" => Some(Self::Korean),
            "ja" => Some(Self::Japanese),
            "zh-CN" => Some(Self::SimplifiedChinese),
            _ => None,
        }
    }

    fn labels(self) -> TrayLabels {
        match self {
            Self::English => TrayLabels {
                open: "Open BroomSweepy",
                idle: "Idle",
                busy: "Scan or search in progress",
                quit: "Quit",
            },
            Self::Korean => TrayLabels {
                open: "BroomSweepy 열기",
                idle: "대기",
                busy: "검사 또는 검색 진행 중",
                quit: "종료",
            },
            Self::Japanese => TrayLabels {
                open: "BroomSweepy を開く",
                idle: "待機中",
                busy: "スキャンまたは検索を実行中",
                quit: "終了",
            },
            Self::SimplifiedChinese => TrayLabels {
                open: "打开 BroomSweepy",
                idle: "空闲",
                busy: "正在扫描或搜索",
                quit: "退出",
            },
        }
    }
}

pub(crate) fn setup(app: &mut App<Wry>) -> tauri::Result<()> {
    let main_window = app
        .get_webview_window("main")
        .ok_or_else(|| startup_error("main 창을 찾지 못해 트레이를 준비할 수 없습니다"))?;
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| startup_error("트레이 아이콘을 찾지 못했습니다"))?;

    let labels = TrayLanguage::English.labels();
    let open_item = MenuItem::with_id(app, OPEN_MENU_ID, labels.open, true, None::<&str>)?;
    let status_item = MenuItem::with_id(app, STATUS_MENU_ID, labels.idle, false, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, QUIT_MENU_ID, labels.quit, true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_item, &status_item, &separator, &quit_item])?;

    let tray = TrayIconBuilder::new()
        .icon(icon)
        .tooltip(format!("BroomSweepy {}", env!("CARGO_PKG_VERSION")))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            OPEN_MENU_ID => queue_show_main_window(app.clone()),
            QUIT_MENU_ID => queue_app_exit(app.clone()),
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
                queue_show_main_window(tray.app_handle().clone());
            }
        })
        .build(app)?;

    if !app.manage(WindowsTrayState {
        open_item,
        status_item,
        quit_item,
        _tray: tray,
        display: Mutex::new(TrayDisplay {
            language: TrayLanguage::English,
            busy: false,
        }),
        updates: Mutex::new(()),
    }) {
        return Err(startup_error("트레이 상태가 이미 등록되어 있습니다"));
    }

    hide_main_window_on_close(main_window);
    Ok(())
}

pub(crate) fn set_busy(app: &AppHandle<Wry>, busy: bool) {
    let Some(tray) = app.try_state::<WindowsTrayState>() else {
        return;
    };
    let Ok(mut display) = tray.display.lock() else {
        return;
    };
    display.busy = busy;
    drop(display);

    let app = app.clone();
    queue_tray_worker("broomsweepy-tray-status", move || {
        refresh_busy_label_from_worker(&app);
    });
}

fn refresh_busy_label_from_worker(app: &AppHandle<Wry>) {
    if let Some(tray) = app.try_state::<WindowsTrayState>() {
        let Ok(_update) = tray.updates.lock() else {
            return;
        };
        let Ok(display) = tray.display.lock() else {
            return;
        };
        let labels = display.language.labels();
        let status = if display.busy {
            labels.busy
        } else {
            labels.idle
        };
        drop(display);
        let _ = tray.status_item.set_text(status);
    }
}

pub(crate) fn set_language(app: &AppHandle<Wry>, code: &str) -> Result<(), String> {
    let language = TrayLanguage::from_code(code)
        .ok_or_else(|| "unsupported application language".to_owned())?;
    let Some(tray) = app.try_state::<WindowsTrayState>() else {
        return Ok(());
    };
    {
        let mut display = tray
            .display
            .lock()
            .map_err(|_| "Windows tray language state is unavailable".to_owned())?;
        display.language = language;
    }
    let _update = tray
        .updates
        .lock()
        .map_err(|_| "Windows tray update state is unavailable".to_owned())?;
    let display = tray
        .display
        .lock()
        .map_err(|_| "Windows tray language state is unavailable".to_owned())?;
    let labels = display.language.labels();
    let status = if display.busy {
        labels.busy
    } else {
        labels.idle
    };
    drop(display);
    tray.open_item
        .set_text(labels.open)
        .map_err(|error| error.to_string())?;
    tray.status_item
        .set_text(status)
        .map_err(|error| error.to_string())?;
    tray.quit_item
        .set_text(labels.quit)
        .map_err(|error| error.to_string())?;
    Ok(())
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

fn queue_show_main_window(app: AppHandle<Wry>) {
    queue_tray_worker("broomsweepy-tray-window-show", move || {
        show_main_window_from_worker(&app);
    });
}

fn show_main_window_from_worker(app: &AppHandle<Wry>) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

fn queue_app_exit(app: AppHandle<Wry>) {
    queue_tray_worker("broomsweepy-tray-exit", move || {
        app.exit(0);
    });
}

fn hide_main_window_on_close(window: WebviewWindow<Wry>) {
    let window_to_hide = window.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let window = window_to_hide.clone();
            queue_tray_worker("broomsweepy-tray-window-hide", move || {
                if let Err(error) = window.hide() {
                    eprintln!("트레이로 창을 숨기지 못했습니다: {error}");
                }
            });
        }
    });
}

fn queue_tray_worker<F>(name: &str, task: F)
where
    F: FnOnce() + Send + 'static,
{
    if let Err(error) = std::thread::Builder::new()
        .name(name.to_owned())
        .spawn(task)
    {
        eprintln!("Windows 트레이 작업자를 시작하지 못했습니다: {error}");
    }
}

fn startup_error(message: &str) -> tauri::Error {
    std::io::Error::other(message).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_language_codes_are_explicit_and_default_labels_are_english() {
        assert!(TrayLanguage::from_code("en").is_some());
        assert!(TrayLanguage::from_code("ko").is_some());
        assert!(TrayLanguage::from_code("ja").is_some());
        assert!(TrayLanguage::from_code("zh-CN").is_some());
        assert!(TrayLanguage::from_code("system").is_none());
        let labels = TrayLanguage::English.labels();
        assert_eq!(labels.open, "Open BroomSweepy");
        assert_eq!(labels.idle, "Idle");
        assert_eq!(labels.quit, "Quit");
    }
}
