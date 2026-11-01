//! Bounded command names for Arclet's command bar.
//!
//! This only resolves names to the existing engine registry. The host starts the matching
//! tool; text is never treated as a script, parameters, a file path, or a shell command.

use std::sync::OnceLock;

use solidcraft_ui_egui::i18n::{format as message, tr};

const MAX_QUERY_BYTES: usize = 256;
const MAX_SUGGESTIONS: usize = 8;

/// A registered command that the command bar can show and start.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Suggestion {
    pub id: &'static str,
    pub label: &'static str,
}

struct Entry {
    suggestion: Suggestion,
    id_lower: String,
    label_lower: String,
}

// These are this app's explicit shortcuts, not a copied command vocabulary. Qualified IDs
// remain available for choosing a different sketch/solid/surface operation. Rotate uses the
// existing Move/Copy transform tool, whose parameters include an axis and an angle.
const ALIASES: &[(&str, &str)] = &[
    ("blend", "surface.blend"),
    ("blendsrf", "surface.blend"),
    ("boolean", "solid.combine"),
    ("boundary", "surface.boundary"),
    ("box", "solid.box"),
    ("bx", "solid.box"),
    ("c", "sketch.circle.center"),
    ("ci", "sketch.circle.center"),
    ("circle", "sketch.circle.center"),
    ("comb", "inspect.curvature_comb"),
    ("continuity", "inspect.curve_continuity"),
    ("e", "solid.extrude"),
    ("ex", "solid.extrude"),
    ("extrude", "solid.extrude"),
    ("f", "view.fit"),
    ("fair", "surface.fair"),
    ("fit", "view.fit"),
    ("front", "view.front"),
    ("gcon", "inspect.curve_continuity"),
    ("iso", "view.isometric"),
    ("isometric", "view.isometric"),
    ("l", "sketch.line"),
    ("lf", "surface.loft"),
    ("line", "sketch.line"),
    ("ln", "sketch.line"),
    ("loft", "surface.loft"),
    ("m", "solid.move"),
    ("match", "surface.match"),
    ("matchsrf", "surface.match"),
    ("move", "solid.move"),
    ("mv", "solid.move"),
    ("o", "sketch.offset"),
    ("of", "sketch.offset"),
    ("offset", "sketch.offset"),
    ("patch", "surface.patch"),
    ("rd", "edit.redo"),
    ("rebuild", "sketch.spline.rebuild"),
    ("rect", "sketch.rectangle.two_point"),
    ("rectangle", "sketch.rectangle.two_point"),
    ("redo", "edit.redo"),
    ("right", "view.right"),
    ("rot", "solid.move"),
    ("rotate", "solid.move"),
    ("sc", "solid.scale"),
    ("scale", "solid.scale"),
    ("sph", "solid.sphere"),
    ("sphere", "solid.sphere"),
    ("spl", "sketch.spline.control_point"),
    ("spline", "sketch.spline.control_point"),
    ("sw", "surface.sweep"),
    ("sweep", "surface.sweep"),
    ("top", "view.top"),
    ("tr", "sketch.trim"),
    ("trim", "sketch.trim"),
    ("un", "edit.undo"),
    ("undo", "edit.undo"),
    ("vf", "view.front"),
    ("vi", "view.isometric"),
    ("vr", "view.right"),
    ("vt", "view.top"),
    ("zebra", "inspect.zebra"),
];

const COMMON_IDS: &[&str] = &[
    "sketch.line",
    "sketch.circle.center",
    "sketch.spline.control_point",
    "sketch.rectangle.two_point",
    "solid.extrude",
    "solid.box",
    "solid.move",
    "view.fit",
];

fn normalized(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ").to_ascii_lowercase()
}

fn query(text: &str) -> Result<String, String> {
    if text.len() > MAX_QUERY_BYTES {
        return Err(tr("arclet.command.too_long").into());
    }
    if text.chars().any(char::is_control) {
        return Err(tr("arclet.command.single_line").into());
    }
    Ok(normalized(text))
}

fn entries() -> &'static [Entry] {
    static ENTRIES: OnceLock<Vec<Entry>> = OnceLock::new();
    ENTRIES.get_or_init(|| {
        let mut entries: Vec<_> = solidcraft_engine::command_specs()
            .into_iter()
            .map(|command| Entry {
                suggestion: Suggestion { id: command.id, label: command.label },
                id_lower: normalized(command.id),
                label_lower: normalized(command.label),
            })
            .collect();
        entries.sort_by_key(|entry| entry.suggestion.id);
        entries
    })
}

/// Resolve an explicit alias, a full command ID, or an unambiguous registry label.
/// Prefixes are suggestions only: pressing Enter never guesses which command was intended.
pub fn resolve(text: &str) -> Result<&'static str, String> {
    let query = query(text)?;
    if query.is_empty() {
        return Err(tr("arclet.command.empty").into());
    }
    if let Some((_, id)) = ALIASES.iter().find(|(alias, _)| *alias == query) {
        return entries()
            .iter()
            .find(|entry| entry.suggestion.id == *id)
            .map(|entry| entry.suggestion.id)
            .ok_or_else(|| tr("arclet.command.unavailable").into());
    }
    if let Some(entry) = entries().iter().find(|entry| entry.id_lower == query) {
        return Ok(entry.suggestion.id);
    }
    let matching: Vec<_> = entries().iter().filter(|entry| entry.label_lower == query).collect();
    match matching.as_slice() {
        [entry] => Ok(entry.suggestion.id),
        [] => {
            let candidates = ranked_suggestions(&query);
            if candidates.is_empty() {
                Err(tr("arclet.command.unknown").into())
            } else {
                let ids = candidates.iter().take(3).map(|candidate| candidate.id).collect::<Vec<_>>().join(", ");
                Err(message("arclet.command.suggest", &[("ids", &ids)]))
            }
        }
        _ => {
            let ids = matching.iter().take(MAX_SUGGESTIONS).map(|entry| entry.suggestion.id).collect::<Vec<_>>().join(", ");
            Err(message("arclet.command.ambiguous", &[("ids", &ids)]))
        }
    }
}

/// At most eight stable, distinct registry suggestions. Invalid input yields no suggestions.
/// Exact aliases and names precede prefixes, which precede substring matches.
pub fn suggestions(text: &str) -> Vec<Suggestion> {
    match query(text) {
        Ok(query) => ranked_suggestions(&query),
        Err(_) => Vec::new(),
    }
}

fn ranked_suggestions(query: &str) -> Vec<Suggestion> {
    if query.is_empty() {
        return COMMON_IDS
            .iter()
            .filter_map(|id| entries().iter().find(|entry| entry.suggestion.id == *id).map(|entry| entry.suggestion))
            .take(MAX_SUGGESTIONS)
            .collect();
    }
    let mut matches = Vec::new();
    for entry in entries() {
        let alias_exact = ALIASES.iter().any(|(alias, id)| *id == entry.suggestion.id && *alias == query);
        let alias_prefix = ALIASES.iter().any(|(alias, id)| *id == entry.suggestion.id && alias.starts_with(query));
        let rank = if alias_exact {
            0
        } else if entry.id_lower == query || entry.label_lower == query {
            1
        } else if alias_prefix {
            2
        } else if entry.label_lower.starts_with(query) || entry.id_lower.starts_with(query) {
            3
        } else if entry.label_lower.contains(query) || entry.id_lower.contains(query) {
            4
        } else {
            continue;
        };
        matches.push((rank, entry.suggestion));
    }
    matches.sort_by(|(a_rank, a), (b_rank, b)| a_rank.cmp(b_rank).then_with(|| a.id.cmp(b.id)));
    matches.into_iter().take(MAX_SUGGESTIONS).map(|(_, suggestion)| suggestion).collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn every_alias_resolves_to_a_real_registered_command() {
        let mut names = BTreeSet::new();
        for (name, id) in ALIASES {
            assert!(names.insert(*name), "duplicate alias {name}");
            assert!(solidcraft_engine::find_command(id).is_some(), "unregistered alias {name}: {id}");
            assert_eq!(resolve(name), Ok(*id));
            assert_eq!(resolve(&name.to_ascii_uppercase()), Ok(*id));
        }
        for name in [
            "Line",
            "Circle",
            "Spline",
            "Rectangle",
            "Extrude",
            "Loft",
            "Sweep",
            "Offset",
            "Trim",
            "Move",
            "Rotate",
            "Scale",
            "Box",
            "Sphere",
            "Front",
            "Top",
            "Right",
            "Iso",
            "Fit",
            "Undo",
            "Redo",
        ] {
            assert!(resolve(name).is_ok(), "missing requested alias {name}");
        }
    }

    #[test]
    fn names_and_full_ids_ignore_case_and_surrounding_spaces() {
        assert_eq!(resolve("  cIrClE   "), Ok("sketch.circle.center"));
        assert_eq!(resolve("\u{2003}SOLID.EXTRUDE\u{2002}"), Ok("solid.extrude"));
        assert_eq!(resolve("  2-Point   Rectangle  "), Ok("sketch.rectangle.two_point"));
        assert_eq!(resolve("Surface   Extrude"), Ok("surface.extrude"));
    }

    #[test]
    fn ambiguous_labels_require_a_qualified_id() {
        let error = resolve("Fillet").unwrap_err();
        assert!(error.contains("Ambiguous"));
        assert!(error.contains("sketch.fillet"));
        assert!(error.contains("solid.fillet"));
        assert_eq!(resolve("sketch.fillet"), Ok("sketch.fillet"));
        assert_eq!(resolve("solid.fillet"), Ok("solid.fillet"));
        assert!(resolve("Extend").is_err());
    }

    #[test]
    fn prefixes_and_script_like_text_are_never_executed_or_guessed() {
        for text in ["", "   ", "extr", "does-not-exist", "line; box", "line && box", "$(line)", "line {}", "NaN", "Infinity", "∞"] {
            assert!(resolve(text).is_err(), "unexpected resolution of {text:?}");
        }
        for text in ["line\nbox", "line\r", "line\0", "\tline", "\u{7f}line"] {
            assert!(resolve(text).is_err());
            assert!(suggestions(text).is_empty());
        }
        let too_long = "x".repeat(MAX_QUERY_BYTES + 1);
        assert!(resolve(&too_long).is_err());
        assert!(suggestions(&too_long).is_empty());
        let too_many_utf8_bytes = "線".repeat(86);
        assert!(resolve(&too_many_utf8_bytes).is_err());
        assert!(suggestions(&too_many_utf8_bytes).is_empty());
    }

    #[test]
    fn suggestions_are_bounded_stable_unique_and_registered() {
        for text in ["", "s", "sketch.", "surface", "point", "Fillet", "line", "unmatched-nonsense"] {
            let first = suggestions(text);
            assert!(first.len() <= MAX_SUGGESTIONS);
            assert_eq!(first, suggestions(text));
            let ids: BTreeSet<_> = first.iter().map(|item| item.id).collect();
            assert_eq!(ids.len(), first.len());
            for item in first {
                let command = solidcraft_engine::find_command(item.id).unwrap();
                assert_eq!(item.label, command.label);
            }
        }
        assert!(suggestions("unmatched-nonsense").is_empty());
        assert!(!suggestions("sketch.").is_empty());
    }

    #[test]
    fn exact_aliases_and_ids_lead_the_suggestions() {
        assert_eq!(suggestions(" EX ").first().map(|item| item.id), Some("solid.extrude"));
        assert_eq!(suggestions("c").first().map(|item| item.id), Some("sketch.circle.center"));
        assert_eq!(suggestions("solid.sweep").first().map(|item| item.id), Some("solid.sweep"));
        assert_eq!(suggestions("extr").first().map(|item| item.id), Some("solid.extrude"));
        let ambiguous = suggestions("Fillet");
        assert!(ambiguous.iter().any(|item| item.id == "sketch.fillet"));
        assert!(ambiguous.iter().any(|item| item.id == "solid.fillet"));
    }
}
