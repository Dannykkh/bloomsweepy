//! A small, read-only usage gauge. Drawing never depends on window activation,
//! so native progress-indicator dimming cannot turn its accent into gray.
use objc2::rc::Retained;
use objc2::runtime::Sel;
use objc2::{AnyThread, DefinedClass, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSAccessibility, NSAccessibilityProgressIndicatorRole, NSBezierPath, NSColor, NSGradient,
    NSView,
};
use objc2_foundation::{MainThreadMarker, NSNumber, NSObjectProtocol, NSRect, NSString};
use std::cell::Cell;

pub(super) struct GaugeState {
    value: Cell<f64>,
    track_color: Retained<NSColor>,
    fallback_color: Retained<NSColor>,
    gradient: Option<Retained<NSGradient>>,
}

define_class!(
    // SAFETY: NSView drawing and all state changes are confined to the main
    // thread; this subclass owns no observer, timer, or external resources.
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    #[name = "BroomSweepyNativeGauge"]
    #[ivars = GaugeState]
    pub(super) struct NativeGauge;

    unsafe impl NSObjectProtocol for NativeGauge {}

    impl NativeGauge {
        #[unsafe(method(isOpaque))]
        fn is_opaque(&self) -> bool {
            false
        }

        #[unsafe(method(allowsVibrancy))]
        fn allows_vibrancy(&self) -> bool {
            false
        }

        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty_rect: NSRect) {
            let bounds = self.bounds();
            if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
                return;
            }
            let radius = (bounds.size.height / 2.0).min(bounds.size.width / 2.0);
            let track = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(
                bounds, radius, radius,
            );
            self.ivars().track_color.setFill();
            track.fill();

            let mut fill_rect = bounds;
            fill_rect.size.width *= self.ivars().value.get() / 100.0;
            if fill_rect.size.width <= 0.0 {
                return;
            }
            let radius = radius.min(fill_rect.size.width / 2.0);
            let fill = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(
                fill_rect, radius, radius,
            );
            if let Some(gradient) = &self.ivars().gradient {
                // AppKit clips this draw to the supplied rounded path.
                gradient.drawInBezierPath_angle(&fill, 0.0);
            } else {
                self.ivars().fallback_color.setFill();
                fill.fill();
            }
        }

        #[unsafe(method_id(accessibilityValue))]
        fn accessibility_value(&self) -> Retained<NSNumber> {
            NSNumber::numberWithDouble(self.ivars().value.get())
        }

        #[unsafe(method_id(accessibilityValueDescription))]
        fn accessibility_value_description(&self) -> Retained<NSString> {
            NSString::from_str(&format!("{:.0}%", self.ivars().value.get()))
        }

        #[unsafe(method(isAccessibilitySelectorAllowed:))]
        fn accessibility_selector_allowed(&self, selector: Sel) -> bool {
            if selector == sel!(setAccessibilityValue:) {
                false
            } else {
                // SAFETY: delegate other capability queries to NSView; this
                // custom value is display-only, not a slider action.
                unsafe { msg_send![super(self), isAccessibilitySelectorAllowed: selector] }
            }
        }
    }
);

impl NativeGauge {
    pub(super) fn new(
        mtm: MainThreadMarker,
        frame: NSRect,
        colors: [(f64, f64, f64); 2],
    ) -> Retained<Self> {
        let [start, end] = colors.map(|(red, green, blue)| {
            NSColor::colorWithSRGBRed_green_blue_alpha(
                color_component(red),
                color_component(green),
                color_component(blue),
                1.0,
            )
        });
        let gradient =
            NSGradient::initWithStartingColor_endingColor(NSGradient::alloc(), &start, &end);
        let allocated = Self::alloc(mtm).set_ivars(GaugeState {
            value: Cell::new(0.0),
            track_color: NSColor::colorWithSRGBRed_green_blue_alpha(1.0, 1.0, 1.0, 0.10),
            fallback_color: start,
            gradient,
        });
        // SAFETY: initialize this NSView subclass exactly once; its frame is
        // supplied by the existing native status layout.
        let gauge: Retained<Self> = unsafe { msg_send![super(allocated), initWithFrame: frame] };
        gauge.setAccessibilityElement(true);
        // SAFETY: AppKit's immutable accessibility role name has process lifetime.
        gauge.setAccessibilityRole(Some(unsafe { NSAccessibilityProgressIndicatorRole }));
        // SAFETY: progress-indicator bounds use numeric accessibility values.
        unsafe {
            gauge.setAccessibilityMinValue(Some(&NSNumber::numberWithDouble(0.0)));
            gauge.setAccessibilityMaxValue(Some(&NSNumber::numberWithDouble(100.0)));
        }
        gauge.setNeedsDisplay(true);
        gauge
    }

    pub(super) fn set_value(&self, value: f64) {
        self.ivars().value.set(clamped_percent(value));
        self.setNeedsDisplay(true);
        // Accessibility getters expose the same value without repeated live
        // announcements on every sample; label/visibility belong to the owner.
    }
}

fn clamped_percent(value: f64) -> f64 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, 100.0)
    }
}

fn color_component(value: f64) -> f64 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_values_are_bounded_and_nan_is_empty() {
        assert_eq!(clamped_percent(f64::NAN), 0.0);
        assert_eq!(clamped_percent(f64::NEG_INFINITY), 0.0);
        assert_eq!(clamped_percent(-1.0), 0.0);
        assert_eq!(clamped_percent(37.5), 37.5);
        assert_eq!(clamped_percent(100.0), 100.0);
        assert_eq!(clamped_percent(101.0), 100.0);
        assert_eq!(clamped_percent(f64::INFINITY), 100.0);
    }

    #[test]
    fn color_components_remain_valid_srgb_values() {
        assert_eq!(color_component(f64::NAN), 0.0);
        assert_eq!(color_component(-0.1), 0.0);
        assert_eq!(color_component(0.5), 0.5);
        assert_eq!(color_component(1.1), 1.0);
    }
}
