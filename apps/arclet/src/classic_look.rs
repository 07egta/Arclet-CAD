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
