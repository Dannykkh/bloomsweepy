//! One reusable status window. Local and global mouse-down monitors exist only
//! while it is shown; global event payloads are never read or retained.
use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadOnly, Message, define_class, msg_send};
use objc2_app_kit::{
    NSApplication, NSBackingStoreType, NSColor, NSEvent, NSEventMask, NSFloatingWindowLevel,
    NSPanel, NSScreen, NSView, NSWindowStyleMask,
};
use objc2_foundation::{MainThreadMarker, NSObjectProtocol, NSPoint, NSRect, NSSize, ns_string};
use std::cell::{Cell, RefCell};
use std::ptr::NonNull;

const PANEL_SIZE: NSSize = NSSize::new(320.0, 384.0);
const SCREEN_PADDING: f64 = 8.0;
const ANCHOR_GAP: f64 = 8.0;

#[derive(Default)]
pub(super) struct PanelState {
    local_monitor: RefCell<Option<Retained<AnyObject>>>,
    global_monitor: RefCell<Option<Retained<AnyObject>>>,
    generation: Cell<u64>,
}

define_class!(
    // SAFETY: NSPanel's lifecycle remains main-thread-only. The owner calls
    // close before releasing this reusable window; releasedWhenClosed is false.
    #[unsafe(super = NSPanel)]
    #[thread_kind = MainThreadOnly]
    #[name = "BroomSweepyStatusPanel"]
    #[ivars = PanelState]
    pub(super) struct StatusPanel;

    unsafe impl NSObjectProtocol for StatusPanel {}

    impl StatusPanel {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key_window(&self) -> bool {
            true
        }

        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main_window(&self) -> bool {
            false
        }

        #[unsafe(method(cancelOperation:))]
        fn cancel_operation(&self, _sender: Option<&AnyObject>) {
            self.close();
        }

        #[unsafe(method(close))]
        fn close_panel(&self) {
            self.close();
        }
    }
);

impl StatusPanel {
    pub(super) fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let allocated = Self::alloc(mtm).set_ivars(PanelState::default());
        // SAFETY: initialize the allocated NSPanel subclass exactly once using
        // its designated initializer with a fixed, finite content rectangle.
        let panel: Retained<Self> = unsafe {
            msg_send![
                super(allocated),
                initWithContentRect: NSRect::new(NSPoint::new(0.0, 0.0), PANEL_SIZE),
                styleMask: NSWindowStyleMask::Borderless,
                backing: NSBackingStoreType::Buffered,
                defer: false,
            ]
        };
        // SAFETY: Rust owns the window's retain count; closing only orders it out.
        unsafe { panel.setReleasedWhenClosed(false) };
        panel.setTitle(ns_string!("BroomSweepy Status"));
        panel.setBackgroundColor(Some(&NSColor::clearColor()));
        panel.setOpaque(false);
        panel.setHasShadow(true);
        panel.setFloatingPanel(true);
        panel.setLevel(NSFloatingWindowLevel);
        panel.setBecomesKeyOnlyIfNeeded(false);
        // The owner explicitly closes on resignation, also removing our monitor.
        panel.setHidesOnDeactivate(false);
        panel
    }

    // Preserve the existing native status UI's small compatibility surface.
    #[allow(non_snake_case)]
    pub(super) fn isShown(&self) -> bool {
        self.isVisible()
    }

    pub(super) fn close(&self) {
        self.remove_event_monitors();
        self.orderOut(None);
    }

    pub(super) fn show_relative(
        &self,
        anchor: &NSView,
        position_beak: impl FnOnce(f64),
    ) -> Result<(), String> {
        // Invalidate callbacks from any previous opening before reusing the window.
        self.close();
        let Some(anchor_window) = anchor.window() else {
            return Err("Status panel anchor is unavailable".into());
        };
        let mtm = self.mtm();
        let Some(screen) = anchor_window.screen().or_else(|| NSScreen::mainScreen(mtm)) else {
            return Err("Status panel display is unavailable".into());
        };
        let anchor_rect =
            anchor_window.convertRectToScreen(anchor.convertRect_toView(anchor.bounds(), None));
        let origin = panel_origin(anchor_rect, self.frame().size, screen.visibleFrame());
        self.setFrameOrigin(origin);
        position_beak(anchor_rect.origin.x + anchor_rect.size.width / 2.0 - origin.x);

        let application = NSApplication::sharedApplication(mtm);
        if objc2::available!(macos = 14.0) {
            application.activate();
        } else {
            #[allow(deprecated)]
            application.activateIgnoringOtherApps(true);
        }
        self.makeKeyAndOrderFront(None);
        if !self.isShown() {
            self.close();
            return Err("Status panel could not be shown".into());
        }
        if let Err(error) = self.install_event_monitors(anchor_window.windowNumber()) {
            self.close();
            return Err(error);
        }
        Ok(())
    }

    fn remove_event_monitors(&self) {
        let state = self.ivars();
        state.generation.set(state.generation.get().wrapping_add(1));
        // Take both tokens and release every borrow before native unregistering
        // can synchronously re-enter. Repeated close is safe and invalidates any
        // asynchronously delivered callback from this generation.
        let local = state.local_monitor.borrow_mut().take();
        let global = state.global_monitor.borrow_mut().take();
        for monitor in [local, global].into_iter().flatten() {
            // SAFETY: these are tokens returned by the matching NSEvent APIs.
            unsafe { NSEvent::removeMonitor(&monitor) };
        }
    }

    fn install_event_monitors(&self, anchor_window_number: isize) -> Result<(), String> {
        self.remove_event_monitors();
        let generation = self.ivars().generation.get();
        let mask =
            NSEventMask::LeftMouseDown | NSEventMask::RightMouseDown | NSEventMask::OtherMouseDown;
        let panel = self.retain();
        let handler = RcBlock::new(move |incoming: NonNull<NSEvent>| -> *mut NSEvent {
            // SAFETY: AppKit provides a live event on the main thread for the
            // duration of this callback. The event is never stored or modified.
            let event = unsafe { incoming.as_ref() };
            let window_number = event.windowNumber();
            if should_dismiss_callback(panel.isShown(), panel.ivars().generation.get(), generation)
                && window_number != panel.windowNumber()
                && window_number != anchor_window_number
            {
                panel.close();
            }
            // Always deliver the original click, including clicks that dismiss.
            incoming.as_ptr()
        });
        // SAFETY: the block always returns its original, still-live event pointer.
        // The token's retained block owns the panel only until close removes it.
        let monitor =
            unsafe { NSEvent::addLocalMonitorForEventsMatchingMask_handler(mask, &handler) }
                .ok_or_else(|| "Status panel local click dismissal is unavailable".to_string())?;
        *self.ivars().local_monitor.borrow_mut() = Some(monitor);

        let panel = self.retain();
        let handler = RcBlock::new(move |_incoming: NonNull<NSEvent>| {
            // The mask alone identifies an outside-app mouse-down. Do not read
            // its position, target app, timestamp, contents, or any other payload.
            if should_dismiss_callback(panel.isShown(), panel.ivars().generation.get(), generation)
            {
                panel.close();
            }
        });
        let monitor = NSEvent::addGlobalMonitorForEventsMatchingMask_handler(mask, &handler)
            .ok_or_else(|| "Status panel outside click dismissal is unavailable".to_string())?;
        *self.ivars().global_monitor.borrow_mut() = Some(monitor);
        Ok(())
    }
}

fn should_dismiss_callback(shown: bool, current_generation: u64, captured_generation: u64) -> bool {
    shown && current_generation == captured_generation
}

/// Place a fixed-size panel below its anchor in macOS's bottom-left coordinates.
/// If a dimension cannot accommodate the requested padding, reduce that padding
/// before moving any content offscreen. A screen smaller than the panel itself
/// cannot contain it without resizing its fixed controls; expose its leading edge.
fn panel_origin(anchor: NSRect, panel: NSSize, visible: NSRect) -> NSPoint {
    fn clamp_axis(desired: f64, start: f64, available: f64, length: f64) -> f64 {
        let room = (available - length).max(0.0);
        let padding = SCREEN_PADDING.min(room / 2.0);
        desired.clamp(start + padding, start + room - padding)
    }

    NSPoint::new(
        clamp_axis(
            anchor.origin.x + anchor.size.width / 2.0 - panel.width / 2.0,
            visible.origin.x,
            visible.size.width,
            panel.width,
        ),
        clamp_axis(
            anchor.origin.y - panel.height - ANCHOR_GAP,
            visible.origin.y,
            visible.size.height,
            panel.height,
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f64, y: f64, width: f64, height: f64) -> NSRect {
        NSRect::new(NSPoint::new(x, y), NSSize::new(width, height))
    }

    #[test]
    fn late_callback_cannot_dismiss_a_reopened_panel() {
        assert!(!should_dismiss_callback(true, 3, 1));
        assert!(should_dismiss_callback(true, 3, 3));
    }

    #[test]
    fn hidden_panel_does_not_respond_to_queued_callbacks() {
        assert!(!should_dismiss_callback(false, 3, 3));
    }

    #[test]
    fn centers_below_anchor_with_a_gap() {
        assert_eq!(
            panel_origin(
                rect(700.0, 878.0, 40.0, 22.0),
                PANEL_SIZE,
                rect(0.0, 0.0, 1440.0, 878.0)
            ),
            NSPoint::new(560.0, 486.0)
        );
    }

    #[test]
    fn clamps_at_left_screen_padding() {
        assert_eq!(
            panel_origin(
                rect(0.0, 878.0, 40.0, 22.0),
                PANEL_SIZE,
                rect(0.0, 0.0, 1440.0, 878.0)
            )
            .x,
            8.0
        );
    }

    #[test]
    fn clamps_at_right_screen_padding() {
        assert_eq!(
            panel_origin(
                rect(1400.0, 878.0, 40.0, 22.0),
                PANEL_SIZE,
                rect(0.0, 0.0, 1440.0, 878.0)
            )
            .x,
            1112.0
        );
    }

    #[test]
    fn reduces_padding_to_fit_a_small_screen() {
        let origin = panel_origin(
            rect(300.0, 390.0, 30.0, 22.0),
            PANEL_SIZE,
            rect(0.0, 0.0, 330.0, 390.0),
        );
        assert_eq!(origin, NSPoint::new(5.0, 3.0));
        assert!(origin.x + PANEL_SIZE.width <= 330.0);
        assert!(origin.y + PANEL_SIZE.height <= 390.0);
    }

    #[test]
    fn clamps_vertical_bounds_and_handles_negative_screen_origins() {
        let screen = rect(-1440.0, -900.0, 1440.0, 878.0);
        let low = panel_origin(rect(-740.0, -1000.0, 40.0, 22.0), PANEL_SIZE, screen);
        let high = panel_origin(rect(-740.0, 2000.0, 40.0, 22.0), PANEL_SIZE, screen);
        assert_eq!(low, NSPoint::new(-880.0, -892.0));
        assert_eq!(high, NSPoint::new(-880.0, -414.0));
    }

    #[test]
    fn undersized_screen_exposes_origin_without_panicking_or_resizing() {
        assert_eq!(
            panel_origin(
                rect(500.0, 500.0, 40.0, 22.0),
                PANEL_SIZE,
                rect(20.0, 30.0, 200.0, 250.0)
            ),
            NSPoint::new(20.0, 30.0)
        );
    }
}
