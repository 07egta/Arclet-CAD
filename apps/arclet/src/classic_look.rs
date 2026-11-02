//! Arclet's classic look: a neutral light-grey interface with a mid-grey viewport gradient, thin
//! dark-grey grid lines, blue selection edges and a yellow selected-body colour.
//!
//! The palette is the built-in `classic` theme (`assets/styles/ui/themes/classic.json`), an
//! ordinary option in the Theme list for every app. Only Arclet makes it the default, and only
//! for a fresh profile: `apply_default` runs before the saved preferences are loaded, so a theme
//! the user already chose (or the saved `theme` value) still wins. Other apps keep `dark`.

use solidcraft_ui_egui::prefs::Prefs;

/// The built-in theme id Arclet starts with.
pub(crate) const THEME_ID: &str = "classic";

/// Make the classic theme Arclet's starting theme. Call before loading saved preferences.
pub(crate) fn apply_default(prefs: &mut Prefs) {
    prefs.theme = THEME_ID.into();
}


/// Desktop chrome measurements in egui **logical points**. The operating system's
/// pixels-per-point scaling is applied by egui, so multiplying these by DPI again
/// would make 150% and 200% displays unusable. This module belongs only to Arclet.
pub(crate) mod chrome {
    use egui::{Pos2, Rect, Vec2, pos2};

    pub const STANDARD_BUTTON: f32 = 32.0;
    pub const MAIN_BUTTON: f32 = 36.0;
    pub const MAIN_WIDTH: f32 = 46.0;
    pub const ICON_SIZE: f32 = 20.0;
    pub const ITEM_GAP: f32 = 2.0;
    pub const SECTION_GAP: f32 = 4.0;
    pub const PANEL_ROW_GAP: f32 = 4.0;
    pub const PROPERTY_COLUMN_GAP: f32 = 10.0;
    pub const FLOATING_MARGIN: f32 = 8.0;
    pub const FLYOUT_WIDTH: f32 = 248.0;
    pub const FLYOUT_ROW_PADDING: f32 = 4.0;
    pub const STATUS_COMPACT_BELOW: f32 = 1380.0;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Density {
        Full,
        Compact,
    }

    /// Screen widths are already in egui points, *not* physical pixels.
    pub fn status_density(logical_width: f32) -> Density {
        if !logical_width.is_finite() || logical_width < STATUS_COMPACT_BELOW {
            Density::Compact
        } else {
            Density::Full
        }
    }

    /// Prefer the right of a tool, flip to the left near the right edge, and
    /// slide up when a bottom-row tool would otherwise open off-screen.
    /// A known popup size from the last frame is used after the first paint.
    pub fn flyout_origin(viewport: Rect, anchor: Rect, desired: Vec2) -> Pos2 {
        let left = viewport.left() + FLOATING_MARGIN;
        let top = viewport.top() + FLOATING_MARGIN;
        let right = (viewport.right() - desired.x.max(0.0) - FLOATING_MARGIN).max(left);
        let bottom = (viewport.bottom() - desired.y.max(0.0) - FLOATING_MARGIN).max(top);
        let preferred = anchor.right() + SECTION_GAP;
        let opposite = anchor.left() - SECTION_GAP - desired.x;
        let x = if preferred <= right {
            preferred
        } else if opposite >= left {
            opposite
        } else {
            right
        };
        pos2(x.clamp(left, right), anchor.top().clamp(top, bottom))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Color32;
    use solidcraft_ui_egui::theme_files;

    #[test]
    fn classic_theme_ships_as_a_builtin_option() {
        let file = theme_files::find(THEME_ID, &[]).expect("the classic theme is built in");
        assert_eq!(file.name, "Classic");
        assert!(theme_files::builtin().iter().any(|t| t.id == THEME_ID));
    }

    #[test]
    fn chrome_flyouts_stay_on_screen_at_laptop_sizes() {
        use super::chrome::{self, Density};
        use egui::{Rect, pos2, vec2};

        for (width, height, scale) in [(1280.0, 720.0, 1.0), (1366.0, 768.0, 1.25), (1920.0, 1080.0, 1.5), (2560.0, 1600.0, 2.0)] {
            let bounds = Rect::from_min_size(pos2(0.0, 0.0), vec2(width / scale, height / scale));
            let size = vec2(248.0, 208.0);
            for anchor in [
                Rect::from_min_size(bounds.left_top() + vec2(8.0, 80.0), vec2(36.0, 36.0)),
                Rect::from_min_size(bounds.right_bottom() - vec2(44.0, 44.0), vec2(36.0, 36.0)),
            ] {
                let at = chrome::flyout_origin(bounds, anchor, size);
                let popup = Rect::from_min_size(at, size);
                assert!(bounds.contains(popup.min) && bounds.contains(popup.max), "{width}x{height} at {scale}x: {popup:?}");
            }
        }
        assert_eq!(chrome::status_density(1920.0), Density::Full);
        assert_eq!(chrome::status_density(1280.0), Density::Compact);
        assert_eq!(chrome::status_density(f32::NAN), Density::Compact);
    }

    #[test]
    fn classic_chrome_text_passes_normal_text_contrast() {
        fn linear(channel: u8) -> f64 {
            let value = f64::from(channel) / 255.0;
            if value <= 0.04045 { value / 12.92 } else { ((value + 0.055) / 1.055).powf(2.4) }
        }
        fn luminance(color: Color32) -> f64 {
            0.2126 * linear(color.r()) + 0.7152 * linear(color.g()) + 0.0722 * linear(color.b())
        }
        fn contrast(a: Color32, b: Color32) -> f64 {
            let (bright, dark) = (luminance(a).max(luminance(b)), luminance(a).min(luminance(b)));
            (bright + 0.05) / (dark + 0.05)
        }
        let t = theme_files::resolve(theme_files::find(THEME_ID, &[]).expect("classic theme shipped"));
        for (name, ink, surface) in [
            ("text/panel", t.text, t.panel),
            ("muted/panel", t.text_dim, t.panel),
            ("text/app bar", t.app_bar_text, t.app_bar),
            ("selection", t.selected_ink, t.selected_bg),
            ("primary action", t.on_accent, t.accent),
        ] {
            assert!(contrast(ink, surface) >= 4.5, "{name} has insufficient contrast");
        }
    }

    #[test]
    fn classic_theme_has_grey_viewport_and_yellow_selected_body() {
        let file = theme_files::find(THEME_ID, &[]).expect("the classic theme is built in");
        let t = theme_files::resolve(file);
        let grey = |c: Color32| c.r().abs_diff(c.g()) <= 8 && c.g().abs_diff(c.b()) <= 12;
        assert!(grey(t.viewport_top) && grey(t.viewport_bottom), "viewport is neutral grey");
        assert!((0x60..=0xb0).contains(&t.viewport_top.r()), "viewport is mid grey");
        assert!(t.body_selected.r() > t.body_selected.b() + 60 && t.body_selected.g() > t.body_selected.b(), "selected body is yellowish");
        assert!(t.icon_palette.is_some(), "the classic theme carries an icon palette");
    }
}
