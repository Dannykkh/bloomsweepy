//! All AppKit objects live only on the main thread. The worker owns aggregate
//! counters, never a WebView, process list, file list, or growing sample history.
use super::{MenuBarSettings, stale, title_percent, usage_percent, valid_cpu};
use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSAccessibility, NSButton, NSButtonType, NSCellImagePosition, NSColor, NSFont, NSImage,
    NSPopover, NSPopoverBehavior, NSProgressIndicator, NSProgressIndicatorStyle, NSStatusBar,
    NSStatusItem, NSTextAlignment, NSTextField, NSVariableStatusItemLength, NSView,
    NSViewController,
};
use objc2_foundation::{
    MainThreadMarker, NSObject, NSObjectProtocol, NSPoint, NSRect, NSRectEdge, NSSize, NSString,
    NSUserDefaults, ns_string,
};
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use sysinfo::{CpuRefreshKind, System};
use tauri::{App, AppHandle, Emitter, Manager, WindowEvent};

const PERCENT_KEY: &str = "BroomSweepy.menuBar.showMemoryPercent";
const LANGUAGE_KEY: &str = "BroomSweepy.menuBar.language";
const CHANGED_EVENT: &str = "menu-bar-settings-changed";

thread_local! { static UI: RefCell<Option<NativeMenuBar>> = const { RefCell::new(None) }; }

struct Runtime {
    stopped: Arc<AtomicBool>,
    stop: Mutex<mpsc::Sender<()>>,
}

#[derive(Clone, Default)]
struct Metrics {
    cpu: Option<f64>,
    memory: Option<f64>,
    used: u64,
    total: u64,
    disk: Option<(u64, u64)>,
    captured_at: u64,
}

struct ActionContext {
    app: AppHandle,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements; all actions use the main thread.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = ActionContext]
    struct MenuBarActions;
    unsafe impl NSObjectProtocol for MenuBarActions {}
    impl MenuBarActions {
        #[unsafe(method(togglePanel:))]
        fn toggle_panel(&self, _sender: Option<&AnyObject>) {
            UI.with_borrow_mut(|slot| {
                if let Some(ui) = slot {
                    if ui.popover.isShown() { ui.popover.close(); }
                    else {
                        ui.show();
                    }
                }
            });
        }
        #[unsafe(method(togglePercent:))]
        fn toggle_percent(&self, _sender: Option<&AnyObject>) {
            UI.with_borrow_mut(|slot| {
                if let Some(ui) = slot { ui.set_percent(!ui.show_percent); }
            });
        }
        #[unsafe(method(openMain:))]
        fn open_main(&self, _sender: Option<&AnyObject>) {
            UI.with_borrow(|slot| { if let Some(ui) = slot { ui.popover.close(); } });
            crate::queue_main_window_restore(self.ivars().app.clone());
        }
        #[unsafe(method(quitApp:))]
        fn quit_app(&self, _sender: Option<&AnyObject>) {
            UI.with_borrow(|slot| { if let Some(ui) = slot { ui.popover.close(); } });
            let app = self.ivars().app.clone();
            // Tauri exit dispatch must not wait on its own main-thread event loop.
            let _ = std::thread::Builder::new().name("menu-bar-exit".into()).spawn(move || app.exit(0));
        }
    }
);

impl MenuBarActions {
    fn new(mtm: MainThreadMarker, app: AppHandle) -> Retained<Self> {
        let object = Self::alloc(mtm).set_ivars(ActionContext { app });
        // SAFETY: initialize the allocated NSObject subclass exactly once.
        unsafe { msg_send![super(object), init] }
    }
}

struct MetricRow {
    label: Retained<NSTextField>,
    value: Retained<NSTextField>,
    detail: Retained<NSTextField>,
    bar: Retained<NSProgressIndicator>,
}

struct NativeMenuBar {
    status: Retained<NSStatusItem>,
    popover: Retained<NSPopover>,
    actions: Retained<MenuBarActions>,
    rows: [MetricRow; 3],
    freshness: Retained<NSTextField>,
    toggle: Retained<NSButton>,
    open: Retained<NSButton>,
    quit: Retained<NSButton>,
    show_percent: bool,
    language: String,
    metrics: Metrics,
}

fn frame(x: f64, y: f64, width: f64, height: f64) -> NSRect {
    NSRect::new(NSPoint::new(x, y), NSSize::new(width, height))
}

fn label(
    mtm: MainThreadMarker,
    view: &NSView,
    text: &str,
    rect: NSRect,
    size: f64,
) -> Retained<NSTextField> {
    let field = NSTextField::labelWithString(&NSString::from_str(text), mtm);
    field.setFrame(rect);
    field.setFont(Some(&NSFont::systemFontOfSize(size)));
    // SAFETY: Both views are owned and main-thread confined.
    view.addSubview(&field);
    field
}

fn button(
    mtm: MainThreadMarker,
    view: &NSView,
    actions: &MenuBarActions,
    action: objc2::runtime::Sel,
    rect: NSRect,
) -> Retained<NSButton> {
    // SAFETY: The target implements each supplied one-argument action selector and
    // is retained for longer than the controls (AppKit targets are weak).
    let result = unsafe {
        NSButton::buttonWithTitle_target_action(ns_string!(""), Some(actions), Some(action), mtm)
    };
    result.setFrame(rect);
    result.setFont(Some(&NSFont::systemFontOfSize(14.0)));
    view.addSubview(&result);
    result
}

impl NativeMenuBar {
    fn new(mtm: MainThreadMarker, app: AppHandle) -> Result<Self, String> {
        let actions = MenuBarActions::new(mtm, app);
        let status =
            NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
        let Some(status_button) = status.button(mtm) else {
            NSStatusBar::systemStatusBar().removeStatusItem(&status);
            return Err("Menu bar button unavailable".into());
        };
        if objc2::available!(macos = 11.0)
            && let Some(image) = NSImage::imageWithSystemSymbolName_accessibilityDescription(
                ns_string!("sparkles"),
                Some(ns_string!("BroomSweepy")),
            )
        {
            image.setTemplate(true);
            image.setSize(NSSize::new(18.0, 18.0));
            status_button.setImage(Some(&image));
            status_button.setImagePosition(NSCellImagePosition::ImageLeft);
        }
        status_button.setAccessibilityLabel(Some(ns_string!("BroomSweepy")));
        status_button.setFont(Some(&NSFont::monospacedDigitSystemFontOfSize_weight(
            12.0, 0.0,
        )));
        unsafe {
            status_button.setTarget(Some(&actions));
            status_button.setAction(Some(sel!(togglePanel:)));
        }
        let view = NSView::initWithFrame(NSView::alloc(mtm), frame(0.0, 0.0, 320.0, 370.0));
        let heading = label(
            mtm,
            &view,
            "BroomSweepy",
            frame(16.0, 334.0, 206.0, 24.0),
            17.0,
        );
        heading.setFont(Some(&NSFont::boldSystemFontOfSize(17.0)));
        let version = label(
            mtm,
            &view,
            &format!("v{}", env!("CARGO_PKG_VERSION")),
            frame(228.0, 337.0, 76.0, 20.0),
            14.0,
        );
        version.setTextColor(Some(&NSColor::secondaryLabelColor()));
        version.setAlignment(NSTextAlignment::Right);
        let rows = [278.0, 214.0, 150.0].map(|y| {
            let name = label(mtm, &view, "", frame(16.0, y + 20.0, 148.0, 22.0), 14.0);
            let value = label(mtm, &view, "—", frame(165.0, y + 20.0, 139.0, 22.0), 15.0);
            value.setAlignment(NSTextAlignment::Right);
            value.setFont(Some(&NSFont::monospacedDigitSystemFontOfSize_weight(
                15.0, 0.3,
            )));
            let detail = label(mtm, &view, "", frame(16.0, y - 10.0, 288.0, 20.0), 14.0);
            detail.setTextColor(Some(&NSColor::secondaryLabelColor()));
            let bar = NSProgressIndicator::initWithFrame(
                NSProgressIndicator::alloc(mtm),
                frame(16.0, y + 11.0, 288.0, 6.0),
            );
            bar.setStyle(NSProgressIndicatorStyle::Bar);
            bar.setIndeterminate(false);
            bar.setMinValue(0.0);
            bar.setMaxValue(100.0);
            view.addSubview(&bar);
            MetricRow {
                label: name,
                value,
                detail,
                bar,
            }
        });
        let freshness = label(mtm, &view, "", frame(16.0, 110.0, 288.0, 22.0), 14.0);
        freshness.setTextColor(Some(&NSColor::secondaryLabelColor()));
        let toggle = button(
            mtm,
            &view,
            &actions,
            sel!(togglePercent:),
            frame(16.0, 76.0, 288.0, 28.0),
        );
        toggle.setButtonType(NSButtonType::Switch);
        let open = button(
            mtm,
            &view,
            &actions,
            sel!(openMain:),
            frame(12.0, 24.0, 210.0, 40.0),
        );
        let quit = button(
            mtm,
            &view,
            &actions,
            sel!(quitApp:),
            frame(226.0, 24.0, 82.0, 40.0),
        );
        let controller = NSViewController::new(mtm);
        controller.setView(&view);
        let popover = NSPopover::new(mtm);
        popover.setContentViewController(Some(&controller));
        popover.setContentSize(NSSize::new(320.0, 370.0));
        popover.setBehavior(NSPopoverBehavior::Transient);
        popover.setAnimates(false);
        let defaults = NSUserDefaults::standardUserDefaults();
        let key = NSString::from_str(PERCENT_KEY);
        let show_percent = defaults.objectForKey(&key).is_none() || defaults.boolForKey(&key);
        let language = defaults
            .stringForKey(&NSString::from_str(LANGUAGE_KEY))
            .map(|s| s.to_string())
            .unwrap_or_else(|| "en".into());
        let ui = Self {
            status,
            popover,
            actions,
            rows,
            freshness,
            toggle,
            open,
            quit,
            show_percent,
            language,
            metrics: Metrics::default(),
        };
        ui.render();
        Ok(ui)
    }

    fn settings(&self) -> MenuBarSettings {
        MenuBarSettings {
            supported: true,
            available: true,
            show_memory_percent: self.show_percent,
        }
    }

    fn show(&self) {
        self.render();
        if let Some(button) = self.status.button(self.actions.mtm()) {
            self.popover.showRelativeToRect_ofView_preferredEdge(
                button.bounds(),
                &button,
                NSRectEdge::MinY,
            );
        }
    }

    fn set_percent(&mut self, enabled: bool) {
        self.show_percent = enabled;
        NSUserDefaults::standardUserDefaults()
            .setBool_forKey(enabled, &NSString::from_str(PERCENT_KEY));
        self.render();
        let _ = self
            .actions
            .ivars()
            .app
            .emit(CHANGED_EVENT, self.settings());
    }

    fn render(&self) {
        let mtm = self.actions.mtm();
        let old = stale(self.metrics.captured_at, now_ms());
        let words = match self.language.as_str() {
            "ko" => [
                "메모리",
                "CPU",
                "시스템 디스크",
                "10초 간격으로 갱신",
                "측정 준비 중…",
                "이전 측정 · 갱신 대기",
                "메모리 사용량 표시",
                "BroomSweepy 열기",
                "종료",
                "사용 가능",
            ],
            "ja" => [
                "メモリ",
                "CPU",
                "システムディスク",
                "10秒ごとに更新",
                "測定を準備中…",
                "前回の測定 · 更新待ち",
                "メモリ使用率を表示",
                "BroomSweepy を開く",
                "終了",
                "空き",
            ],
            "zh-CN" => [
                "内存",
                "CPU",
                "系统磁盘",
                "每10秒更新",
                "正在准备测量…",
                "上次测量 · 等待更新",
                "显示内存使用率",
                "打开 BroomSweepy",
                "退出",
                "可用",
            ],
            _ => [
                "Memory",
                "CPU",
                "System disk",
                "Updates every 10 seconds",
                "Preparing measurement…",
                "Older reading · awaiting update",
                "Show memory usage",
                "Open BroomSweepy",
                "Quit",
                "available",
            ],
        };
        if let Some(button) = self.status.button(mtm) {
            let mut title = title_percent(self.show_percent, self.metrics.memory, old);
            if button.image().is_none() {
                title.insert(0, 'B');
            }
            button.setTitle(&NSString::from_str(&title));
            button.setToolTip(Some(&NSString::from_str(&format!(
                "BroomSweepy {}",
                env!("CARGO_PKG_VERSION")
            ))));
        }
        let disk_percent = self
            .metrics
            .disk
            .and_then(|(free, total)| usage_percent(total.saturating_sub(free), total));
        for (index, percent) in [self.metrics.memory, self.metrics.cpu, disk_percent]
            .into_iter()
            .enumerate()
        {
            let row = &self.rows[index];
            row.label.setStringValue(&NSString::from_str(words[index]));
            let text = percent
                .map(|p| format!("{p:.0}%"))
                .unwrap_or_else(|| "—".into());
            row.value.setStringValue(&NSString::from_str(&text));
            row.bar.setDoubleValue(percent.unwrap_or(0.0));
            row.bar.setHidden(percent.is_none());
            row.bar
                .setAccessibilityLabel(Some(&NSString::from_str(words[index])));
        }
        self.rows[0]
            .detail
            .setStringValue(&NSString::from_str(&if self.metrics.total > 0 {
                format!(
                    "{} / {}",
                    bytes(self.metrics.used),
                    bytes(self.metrics.total)
                )
            } else {
                "—".into()
            }));
        self.rows[1].detail.setStringValue(ns_string!(""));
        self.rows[2].detail.setStringValue(&NSString::from_str(
            &self
                .metrics
                .disk
                .map(|(free, total)| format!("{} {} / {}", bytes(free), words[9], bytes(total)))
                .unwrap_or_else(|| "—".into()),
        ));
        self.freshness
            .setStringValue(&NSString::from_str(if self.metrics.captured_at == 0 {
                words[4]
            } else if old {
                words[5]
            } else {
                words[3]
            }));
        self.toggle.setTitle(&NSString::from_str(words[6]));
        self.toggle.setState(if self.show_percent { 1 } else { 0 });
        self.open.setTitle(&NSString::from_str(words[7]));
        self.quit.setTitle(&NSString::from_str(words[8]));
    }
}

impl Drop for NativeMenuBar {
    fn drop(&mut self) {
        self.popover.close();
        NSStatusBar::systemStatusBar().removeStatusItem(&self.status);
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn bytes(value: u64) -> String {
    format!("{:.1} GiB", value as f64 / 1_073_741_824.0)
}

fn disk_space() -> Option<(u64, u64)> {
    let mut stat = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: fixed NUL-terminated root path and writable struct. No traversal.
    if unsafe { libc::statvfs(c"/".as_ptr(), stat.as_mut_ptr()) } != 0 {
        return None;
    }
    let stat = unsafe { stat.assume_init() };
    let unit = stat.f_frsize;
    let total = u64::from(stat.f_blocks).checked_mul(unit)?;
    let free = u64::from(stat.f_bavail).checked_mul(unit)?.min(total);
    (total > 0).then_some((free, total))
}

fn collect_metrics(system: &mut System) -> Metrics {
    system.refresh_cpu_specifics(CpuRefreshKind::nothing().with_cpu_usage());
    system.refresh_memory();
    Metrics {
        cpu: valid_cpu(system.global_cpu_usage()),
        memory: usage_percent(system.used_memory(), system.total_memory()),
        used: system.used_memory(),
        total: system.total_memory(),
        disk: disk_space(),
        captured_at: now_ms(),
    }
}

fn should_stop(receiver: &mpsc::Receiver<()>, timeout: Duration) -> bool {
    // Disconnect is terminal too; never spin if the app releases its sender.
    !matches!(
        receiver.recv_timeout(timeout),
        Err(mpsc::RecvTimeoutError::Timeout)
    )
}

pub(crate) fn setup(app: &mut App) -> Result<(), String> {
    let mtm = MainThreadMarker::new().ok_or("Menu bar setup requires main thread")?;
    let ui = NativeMenuBar::new(mtm, app.handle().clone())?;
    UI.with_borrow_mut(|slot| *slot = Some(ui));
    let (sender, receiver) = mpsc::channel();
    let stopped = Arc::new(AtomicBool::new(false));
    let worker_stopped = Arc::clone(&stopped);
    let handle = app.handle().clone();
    let worker = std::thread::Builder::new()
        .name("macos-menu-bar-sampler".into())
        .spawn(move || {
            let mut system = System::new();
            system.refresh_cpu_specifics(CpuRefreshKind::nothing().with_cpu_usage());
            if should_stop(&receiver, Duration::from_millis(250)) {
                return;
            }
            let pending_ui = Arc::new(AtomicBool::new(false));
            while !worker_stopped.load(Ordering::Acquire) {
                let metrics = autoreleasepool(|_| collect_metrics(&mut system));
                if !pending_ui.swap(true, Ordering::AcqRel) {
                    let stopped = Arc::clone(&worker_stopped);
                    let pending = Arc::clone(&pending_ui);
                    if handle
                        .run_on_main_thread(move || {
                            if !stopped.load(Ordering::Acquire) {
                                UI.with_borrow_mut(|slot| {
                                    if let Some(ui) = slot {
                                        ui.metrics = metrics;
                                        ui.render();
                                    }
                                });
                            }
                            pending.store(false, Ordering::Release);
                        })
                        .is_err()
                    {
                        break;
                    }
                }
                if should_stop(&receiver, Duration::from_secs(10)) {
                    break;
                }
            }
        });
    if let Err(error) = worker {
        UI.with_borrow_mut(|slot| *slot = None);
        return Err(error.to_string());
    }
    app.manage(Runtime {
        stopped,
        stop: Mutex::new(sender),
    });
    if let Some(window) = app.get_webview_window("main") {
        let hidden = window.clone();
        window.on_window_event(move |event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let window = hidden.clone();
                // Same asynchronous dispatch pattern as the Windows tray.
                let _ = std::thread::Builder::new()
                    .name("menu-bar-hide-main".into())
                    .spawn(move || {
                        if let Err(error) = window.hide() {
                            eprintln!("Could not hide main window: {error}");
                        }
                    });
            }
        });
    }
    Ok(())
}

pub(crate) fn shutdown(app: &AppHandle) {
    if let Some(runtime) = app.try_state::<Runtime>() {
        runtime.stopped.store(true, Ordering::Release);
        if let Ok(sender) = runtime.stop.lock() {
            let _ = sender.send(());
        }
    }
    if MainThreadMarker::new().is_some() {
        UI.with_borrow_mut(|slot| *slot = None);
    }
}

pub(crate) fn set_language(app: &AppHandle, language: String) -> Result<(), String> {
    app.run_on_main_thread(move || {
        let defaults = NSUserDefaults::standardUserDefaults();
        // SAFETY: NSString is a valid NSUserDefaults property-list value.
        unsafe {
            defaults.setObject_forKey(
                Some(&NSString::from_str(&language)),
                &NSString::from_str(LANGUAGE_KEY),
            );
        }
        UI.with_borrow_mut(|slot| {
            if let Some(ui) = slot {
                ui.language = language;
                ui.render();
            }
        });
    })
    .map_err(|error| error.to_string())
}

pub(super) fn show_panel(app: &AppHandle) -> Result<(), String> {
    app.run_on_main_thread(|| {
        UI.with_borrow(|slot| {
            if let Some(ui) = slot {
                ui.show();
            }
        });
    })
    .map_err(|error| error.to_string())
}

pub(super) async fn settings(
    app: AppHandle,
    change: Option<bool>,
) -> Result<MenuBarSettings, String> {
    let (sender, receiver) = mpsc::channel();
    app.run_on_main_thread(move || {
        let result = UI.with_borrow_mut(|slot| {
            if let Some(ui) = slot {
                if let Some(enabled) = change {
                    ui.set_percent(enabled);
                }
                Ok(ui.settings())
            } else if change.is_some() {
                Err("Menu bar status is unavailable; restart the app".into())
            } else {
                Ok(MenuBarSettings {
                    supported: true,
                    available: false,
                    show_memory_percent: false,
                })
            }
        });
        let _ = sender.send(result);
    })
    .map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        receiver
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "Menu bar settings timed out".to_string())
    })
    .await
    .map_err(|error| error.to_string())??
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sampler_stops_on_signal_or_disconnect() {
        let (sender, receiver) = mpsc::channel();
        assert!(!should_stop(&receiver, Duration::ZERO));
        sender.send(()).unwrap();
        assert!(should_stop(&receiver, Duration::ZERO));
        drop(sender);
        assert!(should_stop(&receiver, Duration::ZERO));
    }

    #[test]
    fn repeated_aggregate_samples_do_not_collect_processes() {
        let mut system = System::new();
        for _ in 0..100 {
            let metrics = autoreleasepool(|_| collect_metrics(&mut system));
            assert!(system.processes().is_empty());
            assert!(metrics.total > 0);
            assert!(
                metrics
                    .memory
                    .is_some_and(|value| (0.0..=100.0).contains(&value))
            );
            assert!(metrics.captured_at > 0);
            let (free, total) = metrics.disk.expect("macOS root filesystem capacity");
            assert!(total > 0 && free <= total);
        }
    }
}
