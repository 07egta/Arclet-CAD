//! Arclet's opt-in desktop plugin broker (protocol 1.0).
//!
//! Only metadata is read during discovery. Untrusted code is not loaded into Arclet:
//! an explicitly enabled executable runs in a child process with a bounded JSON
//! request and can propose one host-validated engine command. Process separation
//! is crash isolation, NOT an OS security sandbox; enable only trusted binaries.
#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::{Value, json};
use solidcraft_engine::Session;

const PROTOCOL: &str = "arclet.plugins/1";
const API_VERSION: &str = "1.0";
const MANIFEST_LIMIT: u64 = 64 * 1024;
const REQUEST_LIMIT: usize = 64 * 1024;
const RESPONSE_LIMIT: usize = 64 * 1024;
const MAX_PLUGINS: usize = 64;
const MAX_COMMANDS: usize = 32;
const TIMEOUT: Duration = Duration::from_secs(5);

// This list is deliberately much smaller than the complete engine registry.
// In particular plugins may not save/open files, execute scripts, change
// preferences, run other plugins, or access the control channel.
const EDIT_COMMANDS: &[&str] = &[
    "sketch.arc.three_point",
    "sketch.circle.center",
    "sketch.line",
    "sketch.spline.control_point",
    "solid.box",
    "solid.cone",
    "solid.cylinder",
    "solid.sphere",
    "solid.torus",
];

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    ModelRead,
    ModelEdit,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginCommand {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub output_schema: Value,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub id: String,
    pub version: String,
    pub api_version: String,
    pub executable: String,
    pub permissions: Vec<Permission>,
    pub commands: Vec<PluginCommand>,
}

#[derive(Clone, Debug)]
pub struct PluginInfo {
    pub id: String,
    pub version: String,
    pub permissions: Vec<Permission>,
    pub enabled: bool,
    pub commands: Vec<String>,
}

struct Registered {
    manifest: Manifest,
    directory: PathBuf,
    executable: PathBuf,
    enabled: bool,
}

/// Opt-in plugin registry owned by the Arclet desktop shell, not by SolidCraft.
/// Enablement is intentionally per-session: changing a binary or restarting the
/// app can never silently reuse a prior trust decision.
pub struct PluginManager {
    root: PathBuf,
    registered: BTreeMap<String, Registered>,
    diagnostics: Vec<String>,
}

impl PluginManager {
    /// A registry that performs no file-system access, also used by Arclet tests.
    pub fn empty() -> Self {
        Self { root: PathBuf::new(), registered: BTreeMap::new(), diagnostics: Vec::new() }
    }

    /// Discover manifests in <root>/<plugin-id>/plugin.json. Nothing is executed.
    /// Missing directories are normal: Arclet launches without plugins.
    pub fn discover(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        let mut manager = Self { root: root.clone(), registered: BTreeMap::new(), diagnostics: Vec::new() };
        let entries = match fs::read_dir(&root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return manager,
            Err(error) => {
                manager.diagnostics.push(format!("Cannot scan plugins: {error}"));
                return manager;
            }
        };
        let mut dirs = Vec::new();
        for entry in entries {
            match entry {
                Ok(entry) => {
                    // Symlinked plugin directories are not a discovery source.
                    if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                        dirs.push(entry.path());
                    }
                }
                Err(error) => manager.diagnostics.push(format!("Plugin directory entry: {error}")),
            }
        }
        dirs.sort();
        if dirs.len() > MAX_PLUGINS {
            manager.diagnostics.push(format!("Only the first {MAX_PLUGINS} plugin directories are scanned"));
            dirs.truncate(MAX_PLUGINS);
        }
        for dir in dirs {
            match load_plugin(&dir) {
                Ok(plugin) => {
                    let id = plugin.manifest.id.clone();
                    if manager.registered.insert(id.clone(), plugin).is_some() {
                        manager.registered.remove(&id);
                        manager.diagnostics.push(format!("Duplicate plugin ID: {id}"));
                    }
                }
                Err(error) => manager.diagnostics.push(format!("{}: {error}", dir.display())),
            }
        }
        manager
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn diagnostics(&self) -> &[String] {
        &self.diagnostics
    }

    pub fn list(&self) -> Vec<PluginInfo> {
        self.registered
            .values()
            .map(|registered| PluginInfo {
                id: registered.manifest.id.clone(),
                version: registered.manifest.version.clone(),
                permissions: registered.manifest.permissions.clone(),
                enabled: registered.enabled,
                commands: registered
                    .manifest
                    .commands
                    .iter()
                    .map(|command| format!("plugin.{}.{}", registered.manifest.id, command.name))
                    .collect(),
            })
            .collect()
    }

    /// User action only. Enabling opts into ALL capabilities requested by the
    /// manifest; the UI displays them before the user can enable it.
    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), String> {
        let plugin = self.registered.get_mut(id).ok_or_else(|| format!("Unknown plugin: {id}"))?;
        if enabled {
            validate_executable(&plugin.directory, &plugin.executable)?;
        }
        plugin.enabled = enabled;
        Ok(())
    }

    /// Run a namespaced plugin command, e.g. plugin.org.arclet.sample.report.
    /// Parameters and responses use the deliberately restricted JSON schema
    /// subset documented in docs/arclet-plugins.md.
    pub fn execute(&self, session: &mut Session, id: &str, params: &Value) -> Result<Value, String> {
        let (plugin, command) = self
            .registered
            .values()
            .find_map(|plugin| {
                plugin.manifest.commands.iter().find_map(|command| {
                    (id == format!("plugin.{}.{}", plugin.manifest.id, command.name)).then_some((plugin, command))
                })
            })
            .ok_or_else(|| format!("Unknown plugin command: {id}"))?;
        if !plugin.enabled {
            return Err(format!("Plugin {} is disabled", plugin.manifest.id));
        }
        validate_value(&command.input_schema, params, "parameters", 0)?;
        validate_executable(&plugin.directory, &plugin.executable)?;

        let model = if plugin.manifest.permissions.contains(&Permission::ModelRead) {
            model_snapshot(session)?
        } else {
            Value::Null
        };
        let request = json!({
            "protocol": PROTOCOL,
            "api_version": API_VERSION,
            "plugin_id": plugin.manifest.id,
            "command": command.name,
            "params": params,
            "model": model
        });
        let reply = run_worker(&plugin.executable, &plugin.directory, &request, TIMEOUT)?;
        if !reply.ok {
            return Err(format!("Plugin {} failed: {}", plugin.manifest.id, reply.error.unwrap_or_else(|| "unspecified failure".into())));
        }
        validate_value(&command.output_schema, &reply.output, "output", 0)?;
        if reply.actions.len() > 1 {
            return Err("Plugins may propose at most one editing command per invocation".into());
        }
        // Validate the entire proposal BEFORE mutating any part of the model.
        for action in &reply.actions {
            if !plugin.manifest.permissions.contains(&Permission::ModelEdit) {
                return Err("Permission denied: model_edit was not declared".into());
            }
            if !EDIT_COMMANDS.contains(&action.command.as_str()) {
                return Err(format!("Permission denied: engine command {} is not on the plugin allowlist", action.command));
            }
            if !action.params.is_object() {
                return Err("Engine command parameters must be a JSON object".into());
            }
        }
        let mut applied = Vec::new();
        for action in &reply.actions {
            let value = session.execute(&action.command, &action.params).map_err(|error| format!("Engine rejected plugin edit: {error}"))?;
            applied.push(json!({"command": action.command, "result": value}));
        }
        Ok(json!({"output": reply.output, "applied": applied}))
    }
}

/// Explicit override for development; otherwise a user-specific data directory.
/// Never scan the working directory, PATH, or the shared SolidCraft plugin folder.
pub fn default_directory() -> PathBuf {
    if let Some(path) = std::env::var_os("ARCLET_PLUGIN_DIR") {
        return PathBuf::from(path);
    }
    #[cfg(target_os = "windows")]
    {
        return std::env::var_os("APPDATA")
            .map(|home| PathBuf::from(home).join("Arclet").join("plugins"))
            .unwrap_or_default();
    }
    #[cfg(target_os = "macos")]
    {
        return std::env::var_os("HOME")
            .map(|home| PathBuf::from(home).join("Library/Application Support/Arclet/plugins"))
            .unwrap_or_default();
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Some(home) = std::env::var_os("XDG_DATA_HOME") {
            PathBuf::from(home).join("arclet/plugins")
        } else {
            std::env::var_os("HOME")
                .map(|home| PathBuf::from(home).join(".local/share/arclet/plugins"))
                .unwrap_or_default()
        }
    }
}

fn load_plugin(dir: &Path) -> Result<Registered, String> {
    let canonical = dir.canonicalize().map_err(|error| error.to_string())?;
    let path = canonical.join("plugin.json");
    let file = File::open(&path).map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    file.take(MANIFEST_LIMIT + 1).read_to_end(&mut bytes).map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MANIFEST_LIMIT {
        return Err("Manifest is too large".into());
    }
    let manifest: Manifest = serde_json::from_slice(&bytes).map_err(|error| format!("Invalid manifest: {error}"))?;
    validate_manifest(&manifest)?;
    if dir.file_name().and_then(|name| name.to_str()) != Some(manifest.id.as_str()) {
        return Err("Plugin folder must match its manifest ID".into());
    }
    let name = if cfg!(windows) && !manifest.executable.ends_with(".exe") {
        format!("{}.exe", manifest.executable)
    } else {
        manifest.executable.clone()
    };
    let executable = canonical.join(name);
    validate_executable(&canonical, &executable)?;
    Ok(Registered { manifest, directory: canonical, executable, enabled: false })
}

fn validate_manifest(manifest: &Manifest) -> Result<(), String> {
    let id = manifest.id.as_str();
    if id.is_empty()
        || id.len() > 80
        || !id.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'.' || c == b'_' || c == b'-')
        || id.starts_with(&['.', '-', '_'][..])
        || id.ends_with(&['.', '-', '_'][..])
        || id.contains("..")
    {
        return Err("Invalid plugin ID".into());
    }
    if !valid_semver(&manifest.version) {
        return Err("Plugin version must be major.minor.patch".into());
    }
    if manifest.api_version != API_VERSION {
        return Err(format!("Incompatible plugin API: {} (host supports {API_VERSION})", manifest.api_version));
    }
    let exe = manifest.executable.as_str();
    if exe.is_empty() || exe.starts_with('.') || exe.contains(&['/', '\\', ':'][..]) || exe.len() > 128 {
        return Err("Executable must be a filename within the plugin directory".into());
    }
    if manifest.commands.is_empty() || manifest.commands.len() > MAX_COMMANDS {
        return Err("Plugin must register 1 to 32 commands".into());
    }
    let mut ids = BTreeSet::new();
    for command in &manifest.commands {
        if command.name.is_empty()
            || command.name.len() > 48
            || !command.name.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
            || !command.name.as_bytes()[0].is_ascii_lowercase()
            || !ids.insert(&command.name)
        {
            return Err("Invalid or duplicate plugin command".into());
        }
        if command.description.len() > 256 {
            return Err("Plugin command description is too long".into());
        }
        validate_schema(&command.input_schema, 0)?;
        validate_schema(&command.output_schema, 0)?;
        if command.input_schema["type"] != "object" || command.output_schema["type"] != "object" {
            return Err("Command inputs and outputs must be JSON objects".into());
        }
    }
    if manifest.permissions.len() > 2
        || manifest.permissions.iter().collect::<BTreeSet<_>>().len() != manifest.permissions.len()
    {
        return Err("Duplicate plugin permissions".into());
    }
    Ok(())
}

fn valid_semver(version: &str) -> bool {
    let parts: Vec<_> = version.split('.').collect();
    parts.len() == 3
        && parts.iter().all(|part| !part.is_empty() && (part.len() == 1 || !part.starts_with('0'))
            && part.bytes().all(|byte| byte.is_ascii_digit()) && part.parse::<u64>().is_ok())
}

fn validate_executable(dir: &Path, executable: &Path) -> Result<(), String> {
    let resolved = executable.canonicalize().map_err(|error| format!("Executable not available: {error}"))?;
    if !resolved.starts_with(dir) || !resolved.is_file() {
        return Err("Executable must be a regular file inside its plugin folder".into());
    }
    Ok(())
}

fn validate_schema(schema: &Value, depth: usize) -> Result<(), String> {
    if depth > 8 {
        return Err("Schema nesting limit exceeded".into());
    }
    let fields = schema.as_object().ok_or("Schema must be an object")?;
    let kind = schema.get("type").and_then(Value::as_str).ok_or("Schema needs a type")?;
    if !["object", "array", "string", "number", "integer", "boolean", "null"].contains(&kind) {
        return Err("Unsupported schema type".into());
    }
    for field in fields.keys() {
        if !["type", "properties", "required", "items", "additionalProperties", "minimum", "maximum"].contains(&field.as_str()) {
            return Err(format!("Unsupported schema keyword: {field}"));
        }
    }
    if let Some(properties) = schema.get("properties") {
        if kind != "object" {
            return Err("properties requires object type".into());
        }
        for child in properties.as_object().ok_or("properties must be an object")?.values() {
            validate_schema(child, depth + 1)?;
        }
    }
    if let Some(required) = schema.get("required") {
        if kind != "object" || !required.as_array().is_some_and(|items| items.iter().all(Value::is_string)) {
            return Err("required must be an array of property names".into());
        }
    }
    if let Some(additional) = schema.get("additionalProperties") {
        if kind != "object" || !additional.is_boolean() {
            return Err("additionalProperties must be a boolean for objects".into());
        }
    }
    if let Some(items) = schema.get("items") {
        if kind != "array" {
            return Err("items requires array type".into());
        }
        validate_schema(items, depth + 1)?;
    }
    for limit in ["minimum", "maximum"] {
        if let Some(bound) = schema.get(limit)
            && (!matches!(kind, "number" | "integer") || !bound.is_number())
        {
            return Err(format!("{limit} requires a numeric type and value"));
        }
    }
    Ok(())
}

fn validate_value(schema: &Value, value: &Value, path: &str, depth: usize) -> Result<(), String> {
    if depth > 8 {
        return Err("Value nesting limit exceeded".into());
    }
    let matches = match schema["type"].as_str().unwrap_or("") {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "number" => value.is_number(),
        "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        _ => false,
    };
    if !matches {
        return Err(format!("{path} must be {}", schema["type"].as_str().unwrap_or("a supported type")));
    }
    if let Some(number) = value.as_f64() {
        if schema["minimum"].as_f64().is_some_and(|min| number < min)
            || schema["maximum"].as_f64().is_some_and(|max| number > max)
        {
            return Err(format!("{path} is outside the permitted numeric range"));
        }
    }
    if let Some(object) = value.as_object() {
        if let Some(required) = schema["required"].as_array() {
            for property in required.iter().filter_map(Value::as_str) {
                if !object.contains_key(property) {
                    return Err(format!("{path}.{property} is required"));
                }
            }
        }
        let props = schema["properties"].as_object();
        for (key, field) in object {
            if let Some(child) = props.and_then(|p| p.get(key)) {
                validate_value(child, field, &format!("{path}.{key}"), depth + 1)?;
            } else if schema["additionalProperties"] == false {
                return Err(format!("{path}.{key} is not allowed"));
            }
        }
    }
    if let Some(values) = value.as_array()
        && let Some(item_schema) = schema.get("items")
    {
        for (i, item) in values.iter().enumerate() {
            validate_value(item_schema, item, &format!("{path}[{i}]"), depth + 1)?;
        }
    }
    Ok(())
}

fn model_snapshot(session: &mut Session) -> Result<Value, String> {
    let measured = session.execute("inspect.measure", &json!({})).map_err(|error| format!("Could not read model: {error}"))?;
    let bodies: Vec<Value> = measured["bodies"]
        .as_array()
        .into_iter()
        .flatten()
        .take(32)
        .map(|body| json!({
            "name": body["name"],
            "closed": body["closed"],
            "bbox": body["bbox"],
            "volume_mm3": body["volume_mm3"],
            "area_mm2": body["area_mm2"]
        }))
        .collect();
    Ok(json!({
        "revision": session.revision,
        "body_count": measured["body_count"],
        "total": measured["total"],
        "bodies": bodies,
        "truncated": measured["body_count"].as_u64().is_some_and(|count| count > 32)
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkerAction {
    command: String,
    params: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkerReply {
    protocol: String,
    ok: bool,
    #[serde(default)]
    output: Value,
    #[serde(default)]
    actions: Vec<WorkerAction>,
    error: Option<String>,
}

fn run_worker(executable: &Path, directory: &Path, request: &Value, timeout: Duration) -> Result<WorkerReply, String> {
    let bytes = serde_json::to_vec(request).map_err(|error| error.to_string())?;
    if bytes.len() > REQUEST_LIMIT {
        return Err("Model summary and parameters exceed the plugin request limit".into());
    }
    let mut command = Command::new(executable);
    command.current_dir(directory).env_clear().stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
    // Some native Windows programs need SystemRoot for DLL resolution. Do not
    // inherit arbitrary environment variables or credentials from the host.
    #[cfg(windows)]
    if let Some(system_root) = std::env::var_os("SystemRoot") {
        command.env("SystemRoot", system_root);
    }
    let mut child = command.spawn().map_err(|error| format!("Cannot start plugin process: {error}"))?;
    let Some(mut stdin) = child.stdin.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return Err("Plugin stdin unavailable".into());
    };
    // Write on another thread: a process that never reads stdin cannot hang
    // the Arclet thread before the timeout loop begins.
    let writer = std::thread::spawn(move || stdin.write_all(&bytes));
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let output = child.wait_with_output().map_err(|error| format!("Cannot read plugin output: {error}"))?;
                writer.join().map_err(|_| "Plugin input writer failed".to_string())?
                    .map_err(|error| format!("Plugin did not accept request: {error}"))?;
                if !status.success() {
                    return Err(format!("Plugin process exited with {status}"));
                }
                if output.stdout.len() > RESPONSE_LIMIT {
                    return Err("Plugin response exceeds 64 KiB".into());
                }
                let reply: WorkerReply =
                    serde_json::from_slice(&output.stdout).map_err(|error| format!("Invalid plugin response: {error}"))?;
                if reply.protocol != PROTOCOL {
                    return Err("Plugin response protocol does not match the host".into());
                }
                return Ok(reply);
            }
            Ok(None) if started.elapsed() < timeout => std::thread::sleep(Duration::from_millis(10)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = writer.join();
                return Err("Plugin process timed out and was terminated".into());
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = writer.join();
                return Err(format!("Could not check plugin process: {error}"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_and_schema_are_strict() {
        assert!(valid_semver("1.2.3"));
        assert!(!valid_semver("1.02.3"));
        assert!(!valid_semver("1.2"));
        let schema = json!({"type":"object","properties":{"width":{"type":"number","minimum":1}},"required":["width"],"additionalProperties":false});
        assert!(validate_schema(&schema, 0).is_ok());
        assert!(validate_value(&schema, &json!({"width":2}), "input", 0).is_ok());
        assert!(validate_value(&schema, &json!({"width":0}), "input", 0).is_err());
        assert!(validate_value(&schema, &json!({"width":2,"script":"bad"}), "input", 0).is_err());
        assert!(validate_schema(&json!({"type":"object","$ref":"anything"}), 0).is_err());
    }

    #[test]
    fn no_plugin_directory_is_not_an_error() {
        let missing = std::env::temp_dir().join(format!("arclet-missing-plugins-{}", std::process::id()));
        let manager = PluginManager::discover(missing);
        assert!(manager.list().is_empty());
    }
}
