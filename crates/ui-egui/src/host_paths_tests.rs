use serde_json::{Value, json};

use crate::{HostPaths, Services, SolveApp, host_recovery_params};

fn folder(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("arclet-host-paths-{tag}-{}", std::process::id()))
}

fn app(paths: HostPaths) -> SolveApp {
    SolveApp::new(solidcraft_engine::Session::default(), Services { paths: Some(paths), ..Default::default() })
}

#[test]
fn missing_or_empty_recovery_directory_uses_only_the_host_folder() {
    let dir = folder("defaults");
    let paths = HostPaths { recovery: Some(dir.clone()), ..Default::default() };
    for mut params in [Value::Null, json!({}), json!({"dir": null}), json!({"dir": ""}), json!({"dir": "   ", "id": "saved-entry"})] {
        host_recovery_params(&paths, &mut params).unwrap();
        assert_eq!(params["dir"], json!(dir));
    }
}

#[test]
fn an_explicit_user_directory_remains_available_without_host_recovery() {
    let mut params = json!({"dir": "selected-folder", "id": "saved-entry", "discard": true});
    let before = params.clone();
    host_recovery_params(&HostPaths::default(), &mut params).unwrap();
    assert_eq!(params, before);
}

#[test]
fn malformed_recovery_parameters_cannot_fall_back_to_a_shared_folder() {
    let paths = HostPaths { recovery: Some(folder("malformed")), ..Default::default() };
    for mut params in [json!([]), json!(false), json!(42), json!({"dir": false}), json!({"dir": "x".repeat(4096)})] {
        assert!(host_recovery_params(&paths, &mut params).is_err());
    }
    let mut missing = json!({});
    assert_eq!(host_recovery_params(&HostPaths::default(), &mut missing), Err("host.recovery_unavailable"));
}

#[test]
fn recovery_commands_refuse_a_disabled_host_folder_before_engine_execution() {
    let mut app = app(HostPaths::default());
    assert_eq!(app.recovery_folder(), None);
    for command in ["doc.recovery_list", "doc.recover", "doc.recovery_discard"] {
        let expected = app.tr("host.recovery_unavailable").to_string();
        assert_eq!(app.run(command, json!({"all": true})), Err(expected));
    }
}

#[test]
fn recovery_command_results_name_the_host_or_explicit_directory() {
    let dir = folder("host");
    let mut app = app(HostPaths { recovery: Some(dir.clone()), ..Default::default() });
    assert_eq!(app.recovery_folder(), Some(dir.clone()));
    let listed = app.run("doc.recovery_list", json!({})).unwrap();
    assert_eq!(listed["dir"], json!(dir));

    let explicit = folder("explicit");
    app.services.paths = Some(HostPaths::default());
    let listed = app.run("doc.recovery_list", json!({"dir": explicit})).unwrap();
    assert_eq!(listed["dir"], json!(explicit));
}

fn script(app: &mut SolveApp, params: Value) -> Value {
    let (request, _) = crate::ControlRequest::new("engine.script", params);
    match crate::control::handle(app, &egui::Context::default(), &request) {
        crate::control::Outcome::Done(reply) => reply,
        crate::control::Outcome::Screenshot { .. } => panic!("a script must return a command reply"),
    }
}

#[test]
fn scripts_apply_the_same_host_folder_before_entering_the_engine() {
    let dir = folder("script");
    let mut app = app(HostPaths { recovery: Some(dir.clone()), ..Default::default() });
    for params in [json!([{"command": "doc.recovery_list"}]), json!({"commands": [{"id": "doc.recovery_list", "params": {"dir": null}}]})] {
        let reply = script(&mut app, params);
        assert_eq!(reply["ok"], true, "{reply}");
        assert_eq!(reply["result"][0]["dir"], json!(dir));
    }
}

#[test]
fn scripts_refuse_disabled_recovery_before_applying_earlier_design_steps() {
    let mut app = app(HostPaths::default());
    let reply = script(
        &mut app,
        json!([
            {"command": "solid.box", "params": {"length": 2, "width": 2, "height": 2}},
            {"command": "doc.recovery_discard", "params": {"all": true}},
        ]),
    );
    assert_eq!(reply["ok"], false);
    assert_eq!(reply["error"], app.tr("host.recovery_unavailable"));
    assert!(app.session.doc.features.is_empty());
}
