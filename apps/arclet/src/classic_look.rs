//! Arclet's Industrial look: quiet neutral surfaces, high-contrast annotations, a restrained
//! blue interaction accent, and the original CAD convention of yellow selected bodies.
//!
//! New Arclet profiles default to the built-in `arclet-industrial` palette; existing users
//! keep their saved theme. The original `classic` theme is left unchanged and selectable.
//! Other applications retain their independent defaults.

use solidcraft_ui_egui::prefs::Prefs;

/// The built-in theme id Arclet starts with.
pub(crate) const THEME_ID: &str = "arclet-industrial";
/// The look Arclet starts with: the Melt3D palette (dark neutral greys, green accent), so the two
/// apps share one colour language. Arclet Industrial and Classic stay selectable.
pub(crate) const DEFAULT_THEME_ID: &str = "melt3d";

/// Apply Arclet's new-user theme before loading saved preferences; saved choices win.
pub(crate) fn apply_default(prefs: &mut Prefs) {
    prefs.theme = DEFAULT_THEME_ID.into();
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
    fn arclet_theme_ships_as_a_builtin_option_without_removing_classic() {
        let file = theme_files::find(THEME_ID, &[]).expect("the classic theme is built in");
        assert_eq!(file.name, "Arclet Industrial");
        assert!(theme_files::builtin().iter().any(|t| t.id == THEME_ID));
        assert!(theme_files::find("classic", &[]).is_some(), "existing Classic theme stays selectable");
    }

    #[test]
    fn chrome_flyouts_stay_on_screen_at_laptop_sizes() {
        use super::chrome::{self, Density};
        use egui::{Rect, pos2, vec2};

        for (width, height) in [(1280.0, 720.0), (1366.0, 768.0), (1920.0, 1080.0), (2560.0, 1600.0)] {
            for scale in [1.0, 1.25, 1.5, 2.0] {
                let bounds = Rect::from_min_size(pos2(0.0, 0.0), vec2(width / scale, height / scale));
                let size = vec2(chrome::FLYOUT_WIDTH, 208.0);
                for anchor in [
                    Rect::from_min_size(bounds.left_top() + vec2(8.0, 80.0), vec2(36.0, 36.0)),
                    Rect::from_min_size(bounds.right_bottom() - vec2(44.0, 44.0), vec2(36.0, 36.0)),
                ] {
                    let at = chrome::flyout_origin(bounds, anchor, size);
                    let popup = Rect::from_min_size(at, size);
                    assert!(bounds.contains(popup.min) && bounds.contains(popup.max), "{width}x{height} at {scale}x: {popup:?}");
                }
            }
        }
        assert_eq!(chrome::status_density(1920.0), Density::Full);
        assert_eq!(chrome::status_density(1280.0), Density::Compact);
        assert_eq!(chrome::status_density(f32::NAN), Density::Compact);
    }

    #[test]
    fn arclet_chrome_text_passes_normal_text_contrast() {
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
            ("error", t.error, t.panel),
            ("warning", t.warning, t.panel),
        ] {
            assert!(contrast(ink, surface) >= 4.5, "{name} has insufficient contrast");
        }
    }

    #[test]
    fn arclet_theme_has_grey_viewport_and_yellow_selected_body() {
        let file = theme_files::find(THEME_ID, &[]).expect("the classic theme is built in");
        let t = theme_files::resolve(file);
        let grey = |c: Color32| c.r().abs_diff(c.g()) <= 8 && c.g().abs_diff(c.b()) <= 12;
        assert!(grey(t.viewport_top) && grey(t.viewport_bottom), "viewport is neutral grey");
        assert!((0x60..=0xb0).contains(&t.viewport_top.r()), "viewport is mid grey");
        assert!(t.body_selected.r() > t.body_selected.b() + 60 && t.body_selected.g() > t.body_selected.b(), "selected body is yellowish");
        assert!(t.icon_palette.is_some(), "the classic theme carries an icon palette");
    }

    #[test]
    fn arclet_starts_with_the_melt3d_palette_and_it_stays_readable() {
        assert_eq!(DEFAULT_THEME_ID, "melt3d");
        let file = theme_files::find(DEFAULT_THEME_ID, &[]).expect("the Melt3D theme is built in");
        let t = theme_files::resolve(file);
        assert_eq!((t.accent.r(), t.accent.g(), t.accent.b()), (52, 178, 120), "Melt3D green accent");
        assert!(t.viewport_bottom.r() < 0x30 && t.panel.r() < 0x30, "dark neutral surfaces");
        let mut prefs = Prefs::default();
        apply_default(&mut prefs);
        assert_eq!(prefs.theme, DEFAULT_THEME_ID);
    }
}
