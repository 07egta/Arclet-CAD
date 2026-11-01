//! Keep `COMPATIBILITY_MATRIX.md` honest about the two things a reader cannot check by eye:
//! every command id it names really exists in the engine registry, and every tier word is one
//! of the four the document defines. The matrix is documentation, so a typo in an id or a new
//! synonym for a tier is otherwise invisible until somebody tries the command.
//!
//! This does not claim the tiers are right: reachability changes with `app.rs`, and the menu
//! tests there own that half. It only stops the document from naming commands that do not exist.

use std::collections::BTreeSet;

const MATRIX: &str = include_str!("../../../COMPATIBILITY_MATRIX.md");

/// Ids that the matrix names but that are deliberately not engine commands.
///
/// `ui.isolate` is a viewport context-menu item built in `ui-egui`, documented as such.
const NOT_COMMANDS: &[&str] = &["ui.isolate"];

/// File names and identifiers the document also spells with a dot. They are not commands even
/// when their first segment happens to look like one.
fn is_file_or_host(token: &str) -> bool {
    const SUFFIXES: &[&str] = &[
        ".rs", ".md", ".json", ".toml", ".sc", ".stl", ".step", ".stp", ".3mf", ".obj", ".gltf", ".glb", ".ply", ".dxf", ".svg", ".csv", ".log",
        ".png", ".exe", ".yml", ".yaml", ".com", ".org", ".io", ".dev",
    ];
    SUFFIXES.iter().any(|suffix| token.ends_with(suffix))
}

/// The command namespaces the registry actually uses, so prose such as `e.g.` and file names
/// such as `app.rs` are not mistaken for a stale command.
fn namespaces() -> BTreeSet<String> {
    solidcraft_engine::command_specs().iter().filter_map(|spec| spec.id.split('.').next().map(str::to_string)).collect()
}

/// Every dotted lower-case token in the matrix whose first segment is a command namespace.
fn named_ids() -> BTreeSet<String> {
    let namespaces = namespaces();
    let mut ids = BTreeSet::new();
    let bytes = MATRIX.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let start_ok = bytes[index].is_ascii_lowercase()
            && (index == 0 || !(bytes[index - 1].is_ascii_alphanumeric() || bytes[index - 1] == b'_' || bytes[index - 1] == b'.'));
        if !start_ok {
            index += 1;
            continue;
        }
        let start = index;
        let mut cursor = index;
        while cursor < bytes.len() {
            let byte = bytes[cursor];
            if !(byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'.') {
                break;
            }
            cursor += 1;
        }
        let token = &MATRIX[start..cursor];
        let dotted = token.contains('.') && token.split('.').all(|segment| !segment.is_empty());
        if dotted && !is_file_or_host(token) {
            let first = token.split('.').next().unwrap_or_default();
            if namespaces.contains(first) || NOT_COMMANDS.contains(&token) {
                ids.insert(token.to_string());
            }
        }
        index = cursor.max(index + 1);
    }
    ids
}

#[test]
fn every_command_id_named_in_the_matrix_is_registered() {
    let mut missing = Vec::new();
    for id in named_ids() {
        if NOT_COMMANDS.contains(&id.as_str()) {
            continue;
        }
        if solidcraft_engine::find_command(&id).is_none() {
            missing.push(id);
        }
    }
    assert!(
        missing.is_empty(),
        "the coverage matrix names {} command id(s) the engine does not register, so the document describes commands a user cannot run: {missing:?}",
        missing.len()
    );
}

#[test]
fn the_matrix_only_uses_the_tiers_it_defines() {
    const TIERS: &[&str] = &["main-works", "partial", "engine-only", "missing"];
    let mut bad = Vec::new();
    for line in MATRIX.lines() {
        let Some(cells) = line.strip_prefix("| ") else { continue };
        if cells.starts_with("--") || cells.starts_with("Rhino-style feature") || cells.starts_with("Capability") {
            continue;
        }
        let mut columns = cells.split('|').map(str::trim);
        let _feature = columns.next();
        let _ids = columns.next();
        let Some(tier) = columns.next() else { continue };
        if tier.is_empty() {
            continue;
        }
        if !TIERS.contains(&tier) {
            bad.push(format!("{tier} in: {line}"));
        }
    }
    assert!(bad.is_empty(), "the matrix uses a tier it never defines: {bad:?}");
}

#[test]
fn the_sweep_reads_a_document_with_tiers_in_it() {
    // A parser that stopped matching would make both tests above vacuous.
    assert!(named_ids().len() > 100, "only found {} command id(s) in the matrix", named_ids().len());
    let rows = MATRIX.lines().filter(|line| line.starts_with("| ") && line.ends_with("|")).count();
    assert!(rows > 100, "only found {rows} table row(s) in the matrix");
}
