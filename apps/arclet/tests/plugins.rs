//! Black-box integration tests for the real sample executable and Arclet's
//! plugin broker. No system-wide plugin installation or environment overrides.
#![forbid(unsafe_code)]

#[path = "../src/plugins.rs"]
mod plugins;

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use plugins::{Permission, PluginManager};
use serde_json::{Value, json};
use solidcraft_engine::Session;

static NEXT: AtomicU64 = AtomicU64::new(0);
const SAMPLE: &str = env!("CARGO_BIN_EXE_arclet-sample-plugin");
const ID: &str = "org.arclet.sample";

struct Fixture {
    root: PathBuf,
    manifest: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let token = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("arclet-plugin-test-{}-{token}", std::process::id()));
        let directory = root.join(ID);
        fs::create_dir_all(&directory).expect("create temporary plugin dir");
        let source = PathBuf::from(SAMPLE);
        let filename = if cfg!(windows) { "arclet-sample-plugin.exe" } else { "arclet-sample-plugin" };
        fs::copy(source, directory.join(filename)).expect("copy real compiled sample executable");
        let manifest = directory.join("plugin.json");
        fs::write(&manifest, include_str!("../../../examples/arclet-plugin-sample/plugin.json")).expect("sample manifest");
        Self { root, manifest }
    }

    fn edit_manifest(&self, f: impl FnOnce(&mut Value)) {
        let mut value: Value = serde_json::from_slice(&fs::read(&self.manifest).expect("read manifest")).expect("parse manifest");
        f(&mut value);
        fs::write(&self.manifest, serde_json::to_vec(&value).expect("encode manifest")).expect("write manifest");
    }

    fn registered(&self) -> PluginManager {
        PluginManager::discover(&self.root)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn discovery_registration_and_opt_in_activation() {
    let fixture = Fixture::new();
    let mut manager = fixture.registered();
    assert!(manager.diagnostics().is_empty(), "{:?}", manager.diagnostics());
    let plugins = manager.list();
    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0].id, ID);
    assert_eq!(plugins[0].version, "1.0.0");
    assert!(!plugins[0].enabled);
    assert!(plugins[0].permissions.contains(&Permission::ModelRead));
    assert!(plugins[0].permissions.contains(&Permission::ModelEdit));
    assert_eq!(plugins[0].commands, vec!["plugin.org.arclet.sample.box", "plugin.org.arclet.sample.report"]);
    manager.set_enabled(ID, true).expect("explicitly enable plugin");
    assert!(manager.list()[0].enabled);
    manager.set_enabled(ID, false).expect("disable plugin");
    assert!(!manager.list()[0].enabled);
}

#[test]
fn actual_worker_runs_read_only_and_creates_a_real_box() {
    let fixture = Fixture::new();
    let mut manager = fixture.registered();
    let mut session = Session::default();
    assert!(manager.execute(&mut session, "plugin.org.arclet.sample.report", &json!({})).is_err());
    manager.set_enabled(ID, true).expect("enable");
    let first = manager.execute(&mut session, "plugin.org.arclet.sample.report", &json!({})).expect("read-only report");
    assert_eq!(first["output"]["body_count"], 0);
    assert_eq!(first["applied"].as_array().map(Vec::len), Some(0));

    assert!(manager.execute(&mut session, "plugin.org.arclet.sample.box", &json!({"size_mm":0})).is_err());
    let built = manager.execute(&mut session, "plugin.org.arclet.sample.box", &json!({"size_mm":10})).expect("real native worker + engine");
    assert_eq!(built["applied"][0]["command"], "solid.box");
    let measured = session.execute("inspect.measure", &json!({})).expect("measure resulting design");
    assert_eq!(measured["body_count"], 1);
    let volume = measured["total"]["volume_mm3"].as_f64().expect("volume");
    assert!((volume - 1000.0).abs() < 0.01, "volume {volume}");
    let second = manager.execute(&mut session, "plugin.org.arclet.sample.report", &json!({})).expect("report new geometry");
    assert_eq!(second["output"]["body_count"], 1);

    manager.set_enabled(ID, false).expect("revoke");
    assert!(manager.execute(&mut session, "plugin.org.arclet.sample.box", &json!({"size_mm":5})).is_err());
    let still = session.execute("inspect.measure", &json!({})).expect("original host still operates");
    assert_eq!(still["body_count"], 1);
}

#[test]
fn invalid_manifest_and_api_drift_are_rejected_without_execution() {
    let fixture = Fixture::new();
    fixture.edit_manifest(|v| v["api_version"] = json!("2.0"));
    let registry = fixture.registered();
    assert!(registry.list().is_empty());
    assert!(registry.diagnostics().iter().any(|message| message.contains("Incompatible")));

    fixture.edit_manifest(|v| v["api_version"] = json!("1.0"));
    fixture.edit_manifest(|v| v["commands"][0]["name"] = json!("../../escape"));
    let registry = fixture.registered();
    assert!(registry.list().is_empty());
    assert!(!registry.diagnostics().is_empty());

    fs::write(&fixture.manifest, b"{ invalid json").expect("corrupt manifest");
    assert!(fixture.registered().list().is_empty());
}

#[test]
fn worker_failures_are_reported_and_cannot_modify_the_host() {
    let fixture = Fixture::new();
    let mut manager = fixture.registered();
    manager.set_enabled(ID, true).expect("enable");
    let mut session = Session::default();
    let error = manager
        .execute(&mut session, "plugin.org.arclet.sample.report", &json!({"fail":true}))
        .expect_err("worker should report failure");
    assert!(error.contains("Deliberate sample plugin error"), "{error}");
    assert_eq!(session.execute("inspect.measure", &json!({})).expect("measure")["body_count"], 0);
}

#[test]
fn declared_capabilities_do_not_grant_undeclared_edits() {
    let fixture = Fixture::new();
    fixture.edit_manifest(|v| v["permissions"] = json!(["model_read"]));
    let mut manager = fixture.registered();
    manager.set_enabled(ID, true).expect("read-only plugin enabled");
    let mut session = Session::default();
    let error = manager
        .execute(&mut session, "plugin.org.arclet.sample.box", &json!({"size_mm":10}))
        .expect_err("unpermitted edit refused");
    assert!(error.contains("Permission denied"), "{error}");
    assert_eq!(session.execute("inspect.measure", &json!({})).expect("measure")["body_count"], 0);
}

#[test]
fn invalid_commands_and_missing_directories_are_safe() {
    let fixture = Fixture::new();
    let mut manager = fixture.registered();
    manager.set_enabled(ID, true).expect("enable");
    let mut session = Session::default();
    assert!(manager.execute(&mut session, "plugin.org.arclet.sample.unknown", &json!({})).is_err());
    assert!(PluginManager::discover(fixture.root.join("does-not-exist")).list().is_empty());
    let unrelated = session.execute("solid.box", &json!({"length":1,"width":1,"height":1}));
    assert!(unrelated.is_ok(), "Arclet is independent of plugin discovery");
}
