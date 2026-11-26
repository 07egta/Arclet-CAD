//! Arclet document commands: edit, save, reopen, edit, save, reopen.
//! This tests the real data path, but is not an interactive GUI smoke test.
#![cfg(not(target_arch = "wasm32"))]

use serde_json::json;
use solidcraft_engine::Session;

fn volume(session: &mut Session) -> f64 {
    session.execute("inspect.measure", &json!({})).unwrap()["total"]["volume_mm3"].as_f64().unwrap()
}

#[test]
fn edit_save_reopen_edit_save_reopen_sc() {
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let dir = std::env::temp_dir().join(format!("arclet-release-{}-{stamp}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("roundtrip.sc");
    let path_json = json!({"path": path.to_string_lossy()});

    let mut first = Session::default();
    first
        .execute("solid.box", &json!({"length": 12, "width": 8, "height": 6, "body_name": "Arclet release smoke"}))
        .unwrap();
    let initial_volume = volume(&mut first);
    assert!(initial_volume > 0.0 && first.is_dirty());
    first.execute("file.save_as", &path_json).unwrap();
    assert!(!first.is_dirty());
    assert!(std::fs::metadata(&path).unwrap().len() > 128);

    let mut reopened = Session::default();
    reopened.execute("doc.open", &path_json).unwrap();
    assert!(!reopened.is_dirty());
    assert!((volume(&mut reopened) - initial_volume).abs() < 1e-7);
    assert_eq!(reopened.doc.features.len(), 1);
    reopened
        .execute("solid.sphere", &json!({"diameter": 4, "operation": "new", "body_name": "Arclet second feature"}))
        .unwrap();
    assert!(reopened.is_dirty());
    assert_eq!(reopened.doc.features.len(), 2);
    let edited_volume = volume(&mut reopened);
    assert!(edited_volume > initial_volume);
    reopened.execute("file.save", &json!({})).unwrap();
    assert!(!reopened.is_dirty());

    let mut last = Session::default();
    last.execute("doc.open", &path_json).unwrap();
    assert_eq!(last.doc.features.len(), 2);
    assert!((volume(&mut last) - edited_volume).abs() < 1e-7);
    assert!(!last.is_dirty());

    // A failing Save As must not damage the most recent valid document.
    let invalid = dir.join("missing-folder").join("bad.sc");
    assert!(last.execute("file.save_as", &json!({"path": invalid.to_string_lossy()})).is_err());
    assert!(path.is_file());
    std::fs::remove_dir_all(&dir).unwrap();
}
