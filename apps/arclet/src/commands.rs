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

// These are Arclet's own short names, mapped onto Arclet engine commands. They are a user
// convenience for people who know the common CAD vocabulary: they claim no compatibility with
// another product's command set, and nothing typed here is executed as a script, shell command,
// path or parameter list. Qualified IDs remain available for choosing a different
// sketch/solid/surface operation. Rotate uses the existing Move/Copy transform tool, whose
// parameters include an axis and an angle.
const ALIASES: &[(&str, &str)] = &[
    ("accessibility", "inspect.accessibility"),
    ("align", "solid.align"),
    ("angle", "inspect.measure"),
    ("arc", "sketch.arc.three_point"),
    ("arc3pt", "sketch.arc.three_point"),
    ("arccenter", "sketch.arc.center_point"),
    ("arctangent", "sketch.arc.tangent"),
    ("area", "inspect.measure"),
    ("array", "sketch.pattern.rectangular"),
    ("arraycircular", "sketch.pattern.circular"),
    ("arraylinear", "sketch.pattern.rectangular"),
    ("arraypath", "solid.pattern.path"),
    ("arraypolar", "sketch.pattern.circular"),
    ("arrayrect", "sketch.pattern.rectangular"),
    ("back", "view.back"),
    ("blend", "surface.blend"),
    ("blendcrv", "sketch.blend_curve"),
    ("blendsrf", "surface.blend"),
    ("boolean", "solid.combine"),
    ("booleandifference", "solid.combine"),
    ("booleanintersection", "solid.combine"),
    ("booleanunion", "solid.combine"),
    ("bottom", "view.bottom"),
    ("boundary", "surface.boundary"),
    ("boundaryfill", "solid.boundary_fill"),
    ("boundingsolid", "solid.bounding_solid"),
    ("box", "solid.box"),
    ("break", "sketch.break"),
    ("bx", "solid.box"),
    ("c", "sketch.circle.center"),
    ("centerline", "sketch.centerline"),
    ("centerofmass", "inspect.center_of_mass"),
    ("chamfer", "sketch.chamfer.equal_distance"),
    ("chamferedge", "solid.chamfer"),
    ("checkpoint", "edit.checkpoint"),
    ("ci", "sketch.circle.center"),
    ("circle", "sketch.circle.center"),
    ("circle2pt", "sketch.circle.two_point"),
    ("circle2tangent", "sketch.circle.two_tangent"),
    ("circle3pt", "sketch.circle.three_point"),
    ("circle3tangent", "sketch.circle.three_tangent"),
    ("clearance", "inspect.clearance"),
    ("coil", "solid.coil"),
    ("comb", "inspect.curvature_comb"),
    ("compare", "inspect.compare"),
    ("computeall", "timeline.compute_all"),
    ("cone", "solid.cone"),
    ("conic", "sketch.conic"),
    ("continuity", "inspect.curve_continuity"),
    ("contour", "inspect.section"),
    ("controlpointcurve", "sketch.spline.control_point"),
    ("copy", "solid.move"),
    ("curvatureanalysis", "inspect.curvature_map"),
    ("curvaturegraph", "inspect.curvature_comb"),
    ("curve", "sketch.spline.fit_point"),
    ("cut", "solid.split_body"),
    ("cyl", "solid.cylinder"),
    ("cylinder", "solid.cylinder"),
    ("delete", "selection.delete"),
    ("deleteface", "face.delete"),
    ("dimlinear", "sketch.dimension"),
    ("distance", "inspect.measure"),
    ("draftanalysis", "inspect.draft"),
    ("draftangleanalysis", "inspect.draft"),
    ("draftface", "solid.draft"),
    ("e", "solid.extrude"),
    ("edgesrf", "surface.boundary"),
    ("ellipse", "sketch.ellipse"),
    ("emap", "inspect.environment_map"),
    ("emboss", "solid.emboss"),
    ("environmentmap", "inspect.environment_map"),
    ("ex", "solid.extrude"),
    ("explode", "surface.unstitch"),
    ("export", "file.export"),
    ("extend", "sketch.extend"),
    ("extendsrf", "surface.extend"),
    ("extrude", "solid.extrude"),
    ("extrudecrv", "solid.extrude"),
    ("extrudesrf", "surface.extrude"),
    ("f", "view.fit"),
    ("fair", "surface.fair"),
    ("fillet", "sketch.fillet"),
    ("filletedge", "solid.fillet"),
    ("fit", "view.fit"),
    ("flip", "surface.reverse_normal"),
    ("front", "view.front"),
    ("gcon", "inspect.curve_continuity"),
    ("grid", "view.grid"),
    ("group", "browser.group"),
    ("hide", "browser.visibility"),
    ("hole", "solid.hole"),
    ("home", "view.home"),
    ("infinite", "sketch.infinite"),
    ("insertmesh", "file.insert_mesh"),
    ("insertstep", "file.insert_step"),
    ("interference", "inspect.interference"),
    ("interpcrv", "sketch.spline.fit_point"),
    ("intersect", "sketch.intersect"),
    ("iso", "view.isometric"),
    ("isocurve", "inspect.isocurve"),
    ("isometric", "view.isometric"),
    ("join", "surface.stitch"),
    ("l", "sketch.line"),
    ("left", "view.left"),
    ("length", "inspect.measure"),
    ("lf", "surface.loft"),
    ("line", "sketch.line"),
    ("line3d", "sketch.line3d"),
    ("ln", "sketch.line"),
    ("loft", "surface.loft"),
    ("loftsolid", "solid.loft"),
    ("m", "solid.move"),
    ("massprops", "component.properties"),
    ("match", "surface.match"),
    ("matchsrf", "surface.match"),
    ("measure", "inspect.measure"),
    ("meshconvert", "mesh.convert"),
    ("minimumradius", "inspect.minimum_radius"),
    ("mirror", "sketch.mirror"),
    ("mirrorsolid", "solid.mirror"),
    ("move", "solid.move"),
    ("movesketch", "sketch.move"),
    ("mv", "solid.move"),
    ("new", "file.new"),
    ("newfile", "file.new"),
    ("normaltosel", "view.normal_to"),
    ("o", "sketch.offset"),
    ("of", "sketch.offset"),
    ("offset", "sketch.offset"),
    ("offsetface", "solid.offset_face"),
    ("offsetsrf", "surface.offset"),
    ("open", "doc.open"),
    ("patch", "surface.patch"),
    ("perspective", "view.perspective"),
    ("pipe", "solid.pipe"),
    ("pl", "sketch.chain"),
    ("point", "sketch.point"),
    ("point3d", "sketch.point3d"),
    ("polygon", "sketch.polygon.inscribed"),
    ("polygoncircumscribed", "sketch.polygon.circumscribed"),
    ("polygonedge", "sketch.polygon.edge"),
    ("polyline", "sketch.chain"),
    ("presspull", "solid.press_pull"),
    ("project", "sketch.project"),
    ("projecttosurface", "sketch.project_to_surface"),
    ("pt", "sketch.point"),
    ("rd", "edit.redo"),
    ("rebuild", "sketch.spline.rebuild"),
    ("rect", "sketch.rectangle.two_point"),
    ("rect3pt", "sketch.rectangle.three_point"),
    ("rectangle", "sketch.rectangle.two_point"),
    ("rectcenter", "sketch.rectangle.center"),
    ("rectrounded", "sketch.rectangle.rounded"),
    ("redo", "edit.redo"),
    ("removebody", "solid.remove"),
    ("renamebody", "body.rename"),
    ("repeat", "edit.repeat"),
    ("replaceface", "solid.replace_face"),
    ("restorecheckpoint", "edit.restore_checkpoint"),
    ("reversecurve", "sketch.spline.reverse"),
    ("revolve", "solid.revolve"),
    ("revolvesrf", "surface.revolve"),
    ("rib", "solid.rib"),
    ("right", "view.right"),
    ("rot", "solid.move"),
    ("rotate", "solid.move"),
    ("ruled", "surface.ruled"),
    ("save", "file.save"),
    ("saveas", "file.save_as"),
    ("saveasmesh", "file.save_mesh"),
    ("sc", "solid.scale"),
    ("scale", "solid.scale"),
    ("scalesketch", "sketch.scale"),
    ("section", "inspect.section"),
    ("select", "select.set"),
    ("selectionfilter", "select.filter"),
    ("selnone", "select.clear"),
    ("sendtoslicer", "file.send_to_slicer"),
    ("shell", "solid.shell"),
    ("show", "browser.visibility"),
    ("sph", "solid.sphere"),
    ("sphere", "solid.sphere"),
    ("spl", "sketch.spline.control_point"),
    ("spline", "sketch.spline.control_point"),
    ("splinecv", "sketch.spline.control_point"),
    ("splinefit", "sketch.spline.fit_point"),
    ("split", "solid.split_body"),
    ("splitface", "solid.split_face"),
    ("splitsolid", "solid.split_body"),
    ("stitch", "surface.stitch"),
    ("sw", "surface.sweep"),
    ("sweep", "surface.sweep"),
    ("sweep1", "surface.sweep"),
    ("sweep2", "surface.sweep"),
    ("sweepsolid", "solid.sweep"),
    ("text", "sketch.text"),
    ("thicken", "surface.thicken"),
    ("thread", "solid.thread"),
    ("top", "view.top"),
    ("torus", "solid.torus"),
    ("tr", "sketch.trim"),
    ("trim", "sketch.trim"),
    ("trimpath", "sketch.trim.path"),
    ("trimsrf", "surface.trim"),
    ("un", "edit.undo"),
    ("undo", "edit.undo"),
    ("ungroup", "browser.ungroup"),
    ("unjoin", "surface.unstitch"),
    ("untrim", "surface.untrim"),
    ("vf", "view.front"),
    ("vi", "view.isometric"),
    ("volume", "inspect.measure"),
    ("vr", "view.right"),
    ("vt", "view.top"),
    ("web", "solid.web"),
    ("ze", "view.fit"),
    ("zebra", "inspect.zebra"),
    ("zoomextents", "view.fit"),
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
    fn rhino_style_names_resolve() {
        for (name, id) in [
            ("arc", "sketch.arc.three_point"),
            ("polyline", "sketch.chain"),
            ("ellipse", "sketch.ellipse"),
            ("polygon", "sketch.polygon.inscribed"),
            ("point", "sketch.point"),
            ("interpcrv", "sketch.spline.fit_point"),
            ("controlpointcurve", "sketch.spline.control_point"),
            ("filletedge", "solid.fillet"),
            ("extrudesrf", "surface.extrude"),
            ("edgesrf", "surface.boundary"),
            ("offsetsrf", "surface.offset"),
            ("explode", "surface.unstitch"),
            ("booleanunion", "solid.combine"),
            ("curvatureanalysis", "inspect.curvature_map"),
            ("zoomextents", "view.fit"),
            ("group", "browser.group"),
            ("hide", "browser.visibility"),
            ("dimlinear", "sketch.dimension"),
            ("sweep2", "surface.sweep"),
        ] {
            assert_eq!(resolve(name), Ok(id), "alias {name}");
        }
    }

    #[test]
    fn aliases_are_sorted_lowercase_ascii_words() {
        for (name, _) in ALIASES {
            assert!(name.is_ascii(), "alias {name} is not ASCII");
            assert!(!name.contains(' '), "alias {name} contains a space");
            assert!(name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()), "alias {name} is not a lowercase word");
        }
        assert!(ALIASES.windows(2).all(|pair| pair[0].0 < pair[1].0), "ALIASES is not sorted and unique");
    }

    #[test]
    fn common_short_names_reach_their_engine_commands() {
        for (name, id) in [
            ("line3d", "sketch.line3d"),
            ("arc3pt", "sketch.arc.three_point"),
            ("arccenter", "sketch.arc.center_point"),
            ("arctangent", "sketch.arc.tangent"),
            ("circle2pt", "sketch.circle.two_point"),
            ("circle3pt", "sketch.circle.three_point"),
            ("circle2tangent", "sketch.circle.two_tangent"),
            ("circle3tangent", "sketch.circle.three_tangent"),
            ("polygoncircumscribed", "sketch.polygon.circumscribed"),
            ("polygonedge", "sketch.polygon.edge"),
            ("rect3pt", "sketch.rectangle.three_point"),
            ("rectcenter", "sketch.rectangle.center"),
            ("rectrounded", "sketch.rectangle.rounded"),
            ("point3d", "sketch.point3d"),
            ("conic", "sketch.conic"),
            ("centerline", "sketch.centerline"),
            ("infinite", "sketch.infinite"),
            ("trimpath", "sketch.trim.path"),
            ("break", "sketch.break"),
            ("blendcrv", "sketch.blend_curve"),
            ("reversecurve", "sketch.spline.reverse"),
            ("splinecv", "sketch.spline.control_point"),
            ("splinefit", "sketch.spline.fit_point"),
            ("loftsolid", "solid.loft"),
            ("sweepsolid", "solid.sweep"),
            ("revolvesrf", "surface.revolve"),
            ("projecttosurface", "sketch.project_to_surface"),
            ("draftface", "solid.draft"),
            ("offsetface", "solid.offset_face"),
            ("replaceface", "solid.replace_face"),
            ("deleteface", "face.delete"),
            ("splitface", "solid.split_face"),
            ("splitsolid", "solid.split_body"),
            ("presspull", "solid.press_pull"),
            ("boundaryfill", "solid.boundary_fill"),
            ("boundingsolid", "solid.bounding_solid"),
            ("emboss", "solid.emboss"),
            ("web", "solid.web"),
            ("removebody", "solid.remove"),
            ("copy", "solid.move"),
            ("mirrorsolid", "solid.mirror"),
            ("movesketch", "sketch.move"),
            ("scalesketch", "sketch.scale"),
            ("arrayrect", "sketch.pattern.rectangular"),
            ("arraycircular", "sketch.pattern.circular"),
            ("arraypath", "solid.pattern.path"),
            ("angle", "inspect.measure"),
            ("area", "inspect.measure"),
            ("volume", "inspect.measure"),
            ("massprops", "component.properties"),
            ("centerofmass", "inspect.center_of_mass"),
            ("minimumradius", "inspect.minimum_radius"),
            ("isocurve", "inspect.isocurve"),
            ("interference", "inspect.interference"),
            ("clearance", "inspect.clearance"),
            ("compare", "inspect.compare"),
            ("draftanalysis", "inspect.draft"),
            ("accessibility", "inspect.accessibility"),
            ("emap", "inspect.environment_map"),
            ("home", "view.home"),
            ("grid", "view.grid"),
            ("normaltosel", "view.normal_to"),
            ("select", "select.set"),
            ("selectionfilter", "select.filter"),
            ("checkpoint", "edit.checkpoint"),
            ("restorecheckpoint", "edit.restore_checkpoint"),
            ("computeall", "timeline.compute_all"),
            ("renamebody", "body.rename"),
            ("insertstep", "file.insert_step"),
            ("insertmesh", "file.insert_mesh"),
            ("saveasmesh", "file.save_mesh"),
            ("sendtoslicer", "file.send_to_slicer"),
            ("meshconvert", "mesh.convert"),
        ] {
            assert_eq!(resolve(name), Ok(id), "alias {name}");
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
        let error = resolve("Circular Pattern").unwrap_err();
        assert!(error.contains("Ambiguous"));
        assert!(error.contains("sketch.pattern.circular"));
        assert!(error.contains("solid.pattern.circular"));
        assert_eq!(resolve("Fillet"), Ok("sketch.fillet"));
        assert_eq!(resolve("sketch.fillet"), Ok("sketch.fillet"));
        assert_eq!(resolve("solid.fillet"), Ok("solid.fillet"));
        assert_eq!(resolve("Extend"), Ok("sketch.extend"));
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
        for text in ["", "s", "sketch.", "surface", "point", "Circular Pattern", "line", "unmatched-nonsense"] {
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
        let ambiguous = suggestions("Circular Pattern");
        assert!(ambiguous.iter().any(|item| item.id == "sketch.pattern.circular"));
        assert!(ambiguous.iter().any(|item| item.id == "solid.pattern.circular"));
    }
}
