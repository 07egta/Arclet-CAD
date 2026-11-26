use super::*;
use solidcraft_io::component_library::{ComponentEntry, Parameter, Preview, FORMAT};

#[test]
fn catalog_searches_pinned_versions_and_rejects_tampering_before_insertion() {
    let dir = solidcraft_io::vfs::temp_dir().join(format!("arclet-library-{}-{}", std::process::id(), 38));
    std::fs::create_dir_all(&dir).unwrap();
    let part = dir.join("chair.sc");
    let manifest = dir.join("parts.json");
    let mut source = Session::default();
    source.execute("solid.box", &json!({"length":450,"width":420,"height":30})).unwrap();
    source.execute("file.save", &json!({"path":part})).unwrap();
    let bytes = std::fs::read(&part).unwrap();
    let library = ComponentLibrary {
        format: FORMAT.into(), name: "Parametric demo".into(), entries: vec![ComponentEntry {
            id:"chair".into(), name:"Chair test piece".into(), category:Category::Furniture,
            tags:vec!["seating".into()],source:"chair.sc".into(),sha256:sha256_hex(&bytes),version:1,unit:"mm".into(),
            preview:Preview{bounds:[450.,420.,30.],description:"Simple block fixture".into()},
            parameters:vec![Parameter{name:"width".into(),min:300.,max:600.,default:450.,unit:"mm".into()}],
        }],
    };
    std::fs::write(&manifest, library.encode().unwrap()).unwrap();
    let mut target = Session::default();
    let response = target.execute("components.catalog", &json!({"path":manifest,"query":"CHAIR","tags":["seating"]})).unwrap();
    assert_eq!(response["entries"].as_array().unwrap().len(),1);
    let revision = &library.entries[0].sha256;
    let place = json!({"path":manifest,"entry":"chair","version":1,"sha256":revision,"translate":[10,20,0]});
    let inserted = target.execute("components.insert",&place).unwrap();
    assert!(inserted["inserted"]["component"].as_u64().is_some());
    assert_eq!(target.doc.components.len(),1);
    let before = target.doc.clone();
    let bad_version = json!({"path":manifest,"entry":"chair","version":2,"sha256":revision});
    assert!(target.execute("components.insert",&bad_version).is_err());
    assert_eq!(*before,*target.doc);
    std::fs::write(&part, b"tampered source").unwrap();
    assert!(target.execute("components.insert",&place).is_err());
    assert_eq!(*before,*target.doc);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn rejects_malformed_manifest_without_mutating_design() {
    let mut target = Session::default();
    let before = target.doc.clone();
    assert!(target.execute("components.catalog", &json!({"path":"missing.json","unknown":1})).is_err());
    assert!(target.execute("components.insert", &json!({"path":"missing.json","entry":"a","version":0,"sha256":"broken"})).is_err());
    assert_eq!(*before,*target.doc);
}

#[test]
fn registration_creates_manifest_and_refuses_overwriting_an_existing_version() {
    let dir = solidcraft_io::vfs::temp_dir().join(format!("arclet-register-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let part = dir.join("part.sc");
    let manifest = dir.join("parts.json");
    let mut source = Session::default();
    source.execute("solid.box", &json!({"length":20,"width":20,"height":20})).unwrap();
    source.execute("file.save", &json!({"path":part})).unwrap();
    let mut target = Session::default();
    let input = json!({"path":manifest,"entry":"part","version":1,"name":"Part",
        "category":"primitives","source":"part.sc","unit":"mm",
        "preview":{"bounds":[20,20,20],"description":"Box"}});
    let response = target.execute("components.register", &input).unwrap();
    assert_eq!(response["entries"],1);
    let original = std::fs::read(&manifest).unwrap();
    assert!(target.execute("components.register", &input).is_err());
    assert_eq!(original, std::fs::read(&manifest).unwrap());
    assert!(target.doc.components.is_empty());
    let mut second = input.clone();
    second["version"] = json!(2);
    second["source"] = json!("part-v2.sc");
    std::fs::copy(&part, dir.join("part-v2.sc")).unwrap();
    target.execute("components.register", &second).unwrap();
    let listed = target.execute("components.catalog", &json!({"path":manifest})).unwrap();
    assert_eq!(listed["entries"].as_array().unwrap().len(),2);
    std::fs::remove_dir_all(dir).unwrap();
}
