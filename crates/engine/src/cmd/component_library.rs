//! Reusable components are pinned by content hash and immutable version.
//! Uses the existing design insertion path; no separate geometry or ID allocator.
use super::CommandSpec;
use crate::{Result, Session, params::bad};
use serde::Deserialize;
use serde_json::{Value, json};
use solidcraft_io::component_library::{ComponentLibrary, Category, MAX_MANIFEST_BYTES, sha256_hex};
use std::path::{Path, PathBuf};

pub static COMMANDS: &[CommandSpec] = &[
    CommandSpec::new("components.catalog", "Browse Component Library", catalog)
        .icon("library").noundo().doc_only()
        .params("path: component manifest JSON, query?, category?, tags?: [lowercase slugs]. Returns pinned versions and previews"),
    CommandSpec::new("components.insert", "Insert Library Component", insert)
        .icon("component").enabled(super::not_in_sketch)
        .params("path: component manifest JSON, entry: ID, version: positive integer, sha256: exact revision; translate?: [x,y,z] in mm, name?, parent?, reuse?: bool. Pins source bytes before insertion"),
];
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Browse { path: String, #[serde(default)] query: String, category: Option<Category>, #[serde(default)] tags: Vec<String> }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Insert { path: String, entry: String, version: u32, sha256: String, #[serde(default)] translate: Option<[f64;3]>, name: Option<String>, parent: Option<Value>, #[serde(default)] reuse: bool }

fn load(path: &str, cmd: &str) -> Result<ComponentLibrary> {
    if path.len() > 4096 || path.is_empty() || !path.ends_with(".json") || path.chars().any(char::is_control) {
        return Err(bad(cmd, "path must name a local JSON manifest"));
    }
    solidcraft_io::vfs::check_design_path(path, None).map_err(|e| bad(cmd, e.to_string()))?;
    let bytes = solidcraft_io::vfs::read(path, MAX_MANIFEST_BYTES).map_err(|e| bad(cmd, e.to_string()))?;
    ComponentLibrary::decode(&bytes).map_err(|e| bad(cmd, e.to_string()))
}

fn catalog(_s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "components.catalog";
    let req: Browse = serde_json::from_value(p.clone()).map_err(|e| bad(cmd, e.to_string()))?;
    let library = load(&req.path, cmd)?;
    let found = library.search(&req.query, req.category.as_ref(), &req.tags).map_err(|e| bad(cmd, e.to_string()))?;
    Ok(json!({"name":library.name,"format":library.format,"entries":found}))
}

fn insert(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "components.insert";
    let req: Insert = serde_json::from_value(p.clone()).map_err(|e| bad(cmd, e.to_string()))?;
    if req.version == 0 || req.sha256.len() != 64 || !req.sha256.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(bad(cmd, "an exact version and SHA-256 must be specified"));
    }
    if req.translate.is_some_and(|t| t.iter().any(|v| !v.is_finite() || v.abs() > 1e9)) {
        return Err(bad(cmd, "invalid translation"));
    }
    let library = load(&req.path, cmd)?;
    let entry = library.entries.iter().find(|e| e.id == req.entry && e.version == req.version)
        .ok_or_else(|| bad(cmd, "component entry or version not found"))?;
    if entry.sha256 != req.sha256 {
        return Err(bad(cmd, "component revision has changed; browse again and choose its exact hash"));
    }
    // The geometry importer currently consumes millimetres, not arbitrary source units.
    if entry.unit != "mm" { return Err(bad(cmd, "only millimetre-native component designs can be inserted safely")); }
    let root = Path::new(&req.path).parent().unwrap_or_else(|| Path::new("."));
    let source: PathBuf = root.join(&entry.source);
    let shown = source.to_string_lossy().into_owned();
    // Check resolved path as well: symlinks and redirected parent paths must remain within scope.
    solidcraft_io::vfs::check_design_path(&shown, s.path.as_deref()).map_err(|e| bad(cmd, e.to_string()))?;
    let bytes = solidcraft_io::vfs::read_design_path(&shown, s.path.as_deref(), solidcraft_io::limits::max_design_bytes())
        .map_err(|e| bad(cmd, e.to_string()))?;
    if sha256_hex(&bytes) != entry.sha256 {
        return Err(bad(cmd, "component source changed since the entry was registered"));
    }
    // Delegate feature/occurrence IDs, link inheritance and undo to the existing insertion command.
    // The source may still be changed by another process after verification; this is a known
    // check/use race until insert_design accepts prevalidated in-memory bytes.
    let mut params = json!({"path":shown,"reuse":req.reuse});
    if let Some(name) = req.name { params["name"] = json!(name); }
    if let Some(parent) = req.parent { params["parent"] = parent; }
    if let Some(translate) = req.translate { params["translate"] = json!(translate); }
    let out = crate::find_command("component.insert_design")
        .ok_or_else(|| bad(cmd, "design insertion is unavailable"))?
        .run_nested(s, &params)?;
    Ok(json!({"entry":entry.id,"version":entry.version,"sha256":entry.sha256,"inserted":out,
        "warning":"linked geometry is a frozen B-rep snapshot; source parametric history remains in its .sc design"}))
}

#[cfg(test)]
#[path = "component_library_tests.rs"]
mod tests;
