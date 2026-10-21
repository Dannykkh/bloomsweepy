//! One native material behind full-opacity controls, clipped to a status bubble.
use objc2::{MainThreadOnly, rc::Retained};
use objc2_app_kit::{
    NSColor, NSGlassEffectView, NSGlassEffectViewStyle, NSView, NSVisualEffectBlendingMode,
    NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView, NSWorkspace,
};
use objc2_core_graphics::CGMutablePath;
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize};
use objc2_quartz_core::CAShapeLayer;

pub(super) const BODY_HEIGHT: f64 = 370.0;
pub(super) const SIZE: NSSize = NSSize::new(320.0, BODY_HEIGHT + 14.0);
const CORNER: f64 = 16.0;
const BEAK_HALF_WIDTH: f64 = 13.0;
const BACKGROUND_ALPHA: f64 = 0.82;

pub(super) struct BubbleSurface {
    pub(super) view: Retained<NSView>,
    background: Retained<NSView>,
    mask: Retained<CAShapeLayer>,
    rim: Retained<CAShapeLayer>,
}

impl BubbleSurface {
    pub(super) fn new(mtm: MainThreadMarker, content: &NSView) -> Self {
        let bounds = NSRect::new(NSPoint::new(0.0, 0.0), SIZE);
        let view = NSView::initWithFrame(NSView::alloc(mtm), bounds);
        view.setWantsLayer(true);
        let background: Retained<NSView> = if objc2::available!(macos = 26.0) {
            let glass = NSGlassEffectView::initWithFrame(NSGlassEffectView::alloc(mtm), bounds);
            glass.setStyle(NSGlassEffectViewStyle::Regular);
            glass.setTintColor(None);
            glass.setCornerRadius(0.0);
            // Only this empty background subtree is translucent. Putting the
            // controls here would fade the text/buttons along with the material.
            let empty = NSView::initWithFrame(NSView::alloc(mtm), bounds);
            glass.setContentView(Some(&empty));
            glass.into_super()
        } else {
            let effect = NSVisualEffectView::initWithFrame(NSVisualEffectView::alloc(mtm), bounds);
            effect.setMaterial(NSVisualEffectMaterial::UnderWindowBackground);
            effect.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
            effect.setState(NSVisualEffectState::Active);
            effect.into_super()
        };
        view.addSubview(&background);
        // AppKit adds the later sibling above the background; no nested blur.
        view.addSubview(content);
        let mask = CAShapeLayer::new();
        let rim = CAShapeLayer::new();
        mask.setFrame(bounds);
        rim.setFrame(bounds);
        rim.setFillColor(None);
        rim.setStrokeColor(Some(
            &NSColor::colorWithSRGBRed_green_blue_alpha(1.0, 1.0, 1.0, 0.24).CGColor(),
        ));
        rim.setLineWidth(0.75);
        if let Some(layer) = view.layer() {
            // SAFETY: this mask is owned solely here and has no superlayer.
            unsafe { layer.setMask(Some(&mask)) };
            layer.addSublayer(&rim);
        }
        let surface = Self {
            view,
            background,
            mask,
            rim,
        };
        surface.set_arrow_x(SIZE.width / 2.0);
        surface
    }

    pub(super) fn set_arrow_x(&self, anchor_x: f64) {
        let x = arrow_x(anchor_x);
        let path = CGMutablePath::new();
        let p = Some(&*path);
        let t = std::ptr::null();
        let left = 0.5;
        let right = SIZE.width - 0.5;
        let bottom = 0.5;
        let top = BODY_HEIGHT - 0.5;
        let tip = SIZE.height - 0.5;
        let r = CORNER;
        // Cubic circular corners and a gently rounded, upward-pointing beak.
        let c = r * 0.552_284_749_8;
        // SAFETY: the path is alive throughout construction; a null transform is
        // CoreGraphics' documented identity transform. All coordinates are finite.
        unsafe {
            CGMutablePath::move_to_point(p, t, left + r, bottom);
            CGMutablePath::add_line_to_point(p, t, right - r, bottom);
            CGMutablePath::add_curve_to_point(
                p,
                t,
                right - r + c,
                bottom,
                right,
                bottom + r - c,
                right,
                bottom + r,
            );
            CGMutablePath::add_line_to_point(p, t, right, top - r);
            CGMutablePath::add_curve_to_point(
                p,
                t,
                right,
                top - r + c,
                right - r + c,
                top,
                right - r,
                top,
            );
            CGMutablePath::add_line_to_point(p, t, x + BEAK_HALF_WIDTH, top);
            CGMutablePath::add_curve_to_point(p, t, x + 9.0, top, x + 4.0, tip, x, tip);
            CGMutablePath::add_curve_to_point(
                p,
                t,
                x - 4.0,
                tip,
                x - 9.0,
                top,
                x - BEAK_HALF_WIDTH,
                top,
            );
            CGMutablePath::add_line_to_point(p, t, left + r, top);
            CGMutablePath::add_curve_to_point(
                p,
                t,
                left + r - c,
                top,
                left,
                top - r + c,
                left,
                top - r,
            );
            CGMutablePath::add_line_to_point(p, t, left, bottom + r);
            CGMutablePath::add_curve_to_point(
                p,
                t,
                left,
                bottom + r - c,
                left + r - c,
                bottom,
                left + r,
                bottom,
            );
        }
        CGMutablePath::close_subpath(p);
        self.mask.setPath(Some(&path));
        self.rim.setPath(Some(&path));
        // Recheck on every opening without adding a timer or changing OS prefs.
        self.background.setAlphaValue(background_alpha(
            NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceTransparency(),
        ));
        if let Some(window) = self.view.window() {
            window.invalidateShadow();
        }
    }
}

fn arrow_x(anchor_x: f64) -> f64 {
    let inset = CORNER + BEAK_HALF_WIDTH + 1.0;
    if anchor_x.is_finite() {
        anchor_x.clamp(inset, SIZE.width - inset)
    } else {
        SIZE.width / 2.0
    }
}

fn background_alpha(reduce_transparency: bool) -> f64 {
    if reduce_transparency {
        1.0
    } else {
        BACKGROUND_ALPHA
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beak_tracks_anchor_without_crossing_corners() {
        assert_eq!(arrow_x(160.0), 160.0);
        assert_eq!(arrow_x(70.0), 70.0);
        assert_eq!(arrow_x(-20.0), 30.0);
        assert_eq!(arrow_x(340.0), 290.0);
        assert_eq!(arrow_x(f64::NAN), 160.0);
    }

    #[test]
    fn accessibility_never_forces_translucency() {
        assert_eq!(background_alpha(true), 1.0);
        assert_eq!(background_alpha(false), 0.82);
        assert_eq!(SIZE.height, BODY_HEIGHT + 14.0);
    }
}
