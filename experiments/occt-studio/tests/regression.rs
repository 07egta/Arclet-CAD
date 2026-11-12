use arclet_occt_studio::{
    command::{tokenize, HELP},
    kernel::{Mesh, Shape},
    model::{Session, MAX_HISTORY},
};
use std::{
    f64::consts::PI,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
fn near(a: f64, b: f64, t: f64) {
    assert!((a - b).abs() <= t, "{a} != {b} (tolerance {t})");
}
fn cube() -> Shape {
    Shape::cuboid([0.0; 3], [10.0; 3]).unwrap()
}
fn one() -> Session {
    let mut s = Session::default();
    s.execute("Box 0,0,0 10 10 10").unwrap();
    s
}
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "arclet-occt-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn mesh_ok(m: &Mesh) {
    assert_eq!(m.positions.len(), m.normals.len());
    assert_eq!(m.indices.len() % 3, 0);
    assert!(m.positions.iter().flatten().all(|x| x.is_finite()));
    assert!(m.normals.iter().flatten().all(|x| x.is_finite()));
    assert!(m.indices.iter().all(|&i| (i as usize) < m.positions.len()));
    for n in &m.normals {
        let norm = n.iter().map(|x| x * x).sum::<f64>().sqrt();
        assert!(norm < 1e-12 || (norm - 1.0).abs() < 1e-9);
    }
}
#[test]
fn box_volume_bounds_and_topology() {
    let s = cube().stats().unwrap();
    near(s.volume, 1000.0, 1e-8);
    near(s.area, 600.0, 1e-8);
    assert_eq!((s.faces, s.edges, s.solids), (6, 12, 1));
    for i in 0..3 {
        near(s.bounds[i], 0.0, 1e-7);
        near(s.bounds[i + 3], 10.0, 1e-7);
    }
}
#[test]
fn sphere_analytic_volume() {
    near(
        Shape::sphere([1.0, 2.0, 3.0], 7.0)
            .unwrap()
            .stats()
            .unwrap()
            .volume,
        4.0 / 3.0 * PI * 343.0,
        1e-5,
    );
}
#[test]
fn cylinder_analytic_volume() {
    near(
        Shape::cylinder([0.0; 3], 3.0, 12.0)
            .unwrap()
            .stats()
            .unwrap()
            .volume,
        PI * 9.0 * 12.0,
        1e-6,
    );
}
#[test]
fn cone_analytic_volume() {
    near(
        Shape::cone([0.0; 3], 5.0, 2.0, 9.0)
            .unwrap()
            .stats()
            .unwrap()
            .volume,
        PI * 9.0 / 3.0 * (25.0 + 10.0 + 4.0),
        1e-5,
    );
}
#[test]
fn torus_analytic_volume() {
    near(
        Shape::torus([0.0; 3], 10.0, 2.0)
            .unwrap()
            .stats()
            .unwrap()
            .volume,
        2.0 * PI * PI * 10.0 * 4.0,
        1e-4,
    );
}
#[test]
fn overlapping_boxes_boolean_volumes() {
    let a = cube();
    let b = cube().translate([5.0, 0.0, 0.0]).unwrap();
    near(a.union(&b).unwrap().stats().unwrap().volume, 1500.0, 1e-7);
    near(
        a.difference(&b).unwrap().stats().unwrap().volume,
        500.0,
        1e-7,
    );
    near(
        a.intersection(&b).unwrap().stats().unwrap().volume,
        500.0,
        1e-7,
    );
    near(a.stats().unwrap().volume, 1000.0, 1e-7);
}
#[test]
fn empty_boolean_is_explicit_error() {
    let a = cube();
    let b = a.translate([100.0, 0.0, 0.0]).unwrap();
    assert!(a.intersection(&b).is_err());
    assert!(a.difference(&a).is_err());
}
#[test]
fn cylindrical_hole_matches_analytic_volume() {
    let a = cube();
    let hole = Shape::cylinder([5.0, 5.0, -1.0], 2.0, 12.0).unwrap();
    near(
        a.difference(&hole).unwrap().stats().unwrap().volume,
        1000.0 - PI * 4.0 * 10.0,
        1e-6,
    );
}
#[test]
fn transforms_preserve_or_scale_volume() {
    let a = cube();
    near(
        a.translate([2.0, 3.0, 4.0])
            .unwrap()
            .stats()
            .unwrap()
            .volume,
        1000.0,
        1e-6,
    );
    near(
        a.rotate([0.0; 3], [1.0, 1.0, 1.0], 73.0)
            .unwrap()
            .stats()
            .unwrap()
            .volume,
        1000.0,
        1e-6,
    );
    near(
        a.mirror([0.0; 3], [1.0, 0.0, 0.0])
            .unwrap()
            .stats()
            .unwrap()
            .volume,
        1000.0,
        1e-6,
    );
    near(
        a.scale([0.0; 3], 2.0).unwrap().stats().unwrap().volume,
        8000.0,
        1e-6,
    );
}
#[test]
fn fillet_and_chamfer_make_valid_smaller_solids() {
    for s in [
        cube().fillet(1.0, &[]).unwrap(),
        cube().chamfer(1.0, &[]).unwrap(),
    ] {
        let p = s.stats().unwrap();
        assert!(p.volume > 0.0 && p.volume < 1000.0);
        assert_eq!(p.solids, 1);
    }
}
#[test]
fn selected_edges_validate_indices() {
    assert!(cube().fillet(1.0, &[1]).is_ok());
    assert!(cube().fillet(1.0, &[0]).is_err());
    assert!(cube().fillet(1.0, &[99999]).is_err());
    assert!(cube().fillet(10000.0, &[]).is_err());
}
#[test]
fn polygon_extrudes_to_exact_solid() {
    let w = Shape::polyline(
        &[
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [10.0, 5.0, 0.0],
            [0.0, 5.0, 0.0],
        ],
        true,
    )
    .unwrap();
    let s = w.extrude([0.0, 0.0, 7.0]).unwrap().stats().unwrap();
    near(s.volume, 350.0, 1e-7);
    assert_eq!(s.solids, 1);
}
#[test]
fn circle_extrudes_to_cylinder() {
    let c = Shape::circle([0.0; 3], 4.0).unwrap();
    near(
        c.extrude([0.0, 0.0, 9.0]).unwrap().stats().unwrap().volume,
        PI * 16.0 * 9.0,
        1e-6,
    );
}
#[test]
fn revolution_of_rectangle_is_annular_solid() {
    let p = Shape::polyline(
        &[
            [2.0, 0.0, 0.0],
            [4.0, 0.0, 0.0],
            [4.0, 0.0, 5.0],
            [2.0, 0.0, 5.0],
        ],
        true,
    )
    .unwrap();
    let s = p.revolve([0.0; 3], [0.0, 0.0, 1.0], 360.0).unwrap();
    near(s.stats().unwrap().volume, PI * (16.0 - 4.0) * 5.0, 1e-6);
}
#[test]
fn loft_round_sections_is_cylinder() {
    let a = Shape::circle([0.0; 3], 4.0).unwrap();
    let b = Shape::circle([0.0, 0.0, 10.0], 4.0).unwrap();
    let s = Shape::loft(&[a, b], true).unwrap().stats().unwrap();
    near(s.volume, PI * 16.0 * 10.0, 1e-4);
    assert_eq!(s.solids, 1);
}
#[test]
fn rational_quarter_circle_is_not_polyline() {
    let w = Shape::nurbs(
        2,
        &[
            [1.0, 0.0, 0.0, 1.0],
            [1.0, 1.0, 0.0, 0.5_f64.sqrt()],
            [0.0, 1.0, 0.0, 1.0],
        ],
    )
    .unwrap();
    near(w.stats().unwrap().length, PI / 2.0, 1e-6);
    let m = w.mesh(0.001).unwrap();
    assert!(m.lines.len() > 4);
    for p in m.lines.iter().flatten() {
        near(p[0] * p[0] + p[1] * p[1], 1.0, 1e-6);
    }
}
#[test]
fn interpolated_curve_is_meshable() {
    let c = Shape::interpolate(&[[0.0, 0.0, 0.0], [5.0, 6.0, 0.0], [10.0, 0.0, 0.0]]).unwrap();
    assert!(c.stats().unwrap().length > 10.0);
    assert!(c.mesh(0.01).unwrap().lines.len() > 2);
}
#[test]
fn invalid_and_nonfinite_inputs_are_rejected() {
    for x in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -1.0,
        0.0,
        1e-12,
        1e100,
    ] {
        assert!(Shape::sphere([0.0; 3], x).is_err());
        assert!(Shape::cuboid([0.0; 3], [1.0, x, 1.0]).is_err());
    }
    assert!(Shape::circle([f64::NAN, 0.0, 0.0], 1.0).is_err());
    assert!(Shape::torus([0.0; 3], 2.0, 3.0).is_err());
    assert!(cube().rotate([0.0; 3], [0.0; 3], 45.0).is_err());
    assert!(cube().scale([0.0; 3], -1.0).is_err());
}
#[test]
fn malformed_nurbs_are_rejected() {
    assert!(Shape::nurbs(0, &[[0.0; 4]; 3]).is_err());
    assert!(Shape::nurbs(2, &[[0.0, 0.0, 0.0, 1.0]; 2]).is_err());
    assert!(Shape::nurbs(2, &[[0.0, 0.0, 0.0, -1.0]; 3]).is_err());
}
#[test]
fn tessellation_is_finite_and_indexed() {
    for s in [
        cube(),
        Shape::sphere([0.0; 3], 10.0).unwrap(),
        Shape::torus([0.0; 3], 10.0, 2.0).unwrap(),
    ] {
        let m = s.mesh(0.1).unwrap();
        mesh_ok(&m);
        assert!(!m.indices.is_empty());
        assert!(!m.lines.is_empty());
    }
}
#[test]
fn meshing_does_not_mutate_serialized_shape() {
    let s = cube();
    let before = s.to_brep().unwrap();
    s.mesh(0.1).unwrap();
    assert_eq!(before, s.to_brep().unwrap());
}
#[test]
fn brep_round_trip_preserves_measurements() {
    let s = cube().fillet(1.0, &[]).unwrap();
    let t = Shape::from_brep(&s.to_brep().unwrap()).unwrap();
    near(s.stats().unwrap().volume, t.stats().unwrap().volume, 1e-8);
}
#[test]
fn step_round_trip_preserves_volume() {
    let dir = Temp::new();
    let p = dir.0.join("part.step");
    let s = cube()
        .difference(&Shape::cylinder([5.0, 5.0, -1.0], 2.0, 12.0).unwrap())
        .unwrap();
    s.write_step(p.to_str().unwrap()).unwrap();
    let t = Shape::import_step(p.to_str().unwrap()).unwrap();
    near(s.stats().unwrap().volume, t.stats().unwrap().volume, 1e-5);
}
#[test]
fn failed_command_preserves_state_and_history() {
    let mut s = one();
    let before = s.project_json().unwrap();
    let revision = s.revision();
    let undo = s.undo_len();
    assert!(s.execute("FilletEdge 1 100000").is_err());
    assert_eq!(s.project_json().unwrap(), before);
    assert_eq!(s.revision(), revision);
    assert_eq!(s.undo_len(), undo);
}
#[test]
fn undo_redo_and_monotonic_ids() {
    let mut s = one();
    s.execute("Move 1 5,0,0").unwrap();
    s.undo().unwrap();
    near(
        s.state().object(1).unwrap().shape.stats().unwrap().bounds[0],
        0.0,
        1e-7,
    );
    s.redo().unwrap();
    near(
        s.state().object(1).unwrap().shape.stats().unwrap().bounds[0],
        5.0,
        1e-7,
    );
    s.undo().unwrap();
    s.undo().unwrap();
    let id = s.execute("Sphere 0,0,0 2").unwrap().created[0];
    assert!(id > 1);
    assert!(s.redo().is_err());
}
#[test]
fn boolean_is_one_transaction_and_undo_restores_inputs() {
    let mut s = one();
    s.execute("Box 5,0,0 10 10 10").unwrap();
    s.execute("BooleanDifference 1 2").unwrap();
    assert_eq!(s.state().objects.len(), 1);
    near(
        s.state().object(1).unwrap().shape.stats().unwrap().volume,
        500.0,
        1e-6,
    );
    s.undo().unwrap();
    assert_eq!(s.state().objects.len(), 2);
    near(
        s.state().object(1).unwrap().shape.stats().unwrap().volume,
        1000.0,
        1e-6,
    );
}
#[test]
fn keep_inputs_creates_a_new_boolean_object() {
    let mut s = one();
    s.execute("Box 5,0,0 10 10 10").unwrap();
    let id = s.execute("BooleanUnion 1 2 --keep").unwrap().created[0];
    assert_eq!(s.state().objects.len(), 3);
    near(
        s.state().object(id).unwrap().shape.stats().unwrap().volume,
        1500.0,
        1e-6,
    );
}
#[test]
fn locks_prevent_geometry_changes() {
    let mut s = one();
    s.execute("Lock 1").unwrap();
    assert!(s.execute("Move 1 1,0,0").is_err());
    assert!(s.execute("Delete 1").is_err());
    s.execute("Unlock 1").unwrap();
    s.execute("LayerLock 0 on").unwrap();
    assert!(s.execute("Scale 1 0,0,0 2").is_err());
    assert!(s.execute("Sphere 0,0,0 2").is_err());
}
#[test]
fn atomic_batch_rolls_back_all_steps() {
    let mut s = one();
    let before = s.project_json().unwrap();
    let history = s.undo_len();
    assert!(s
        .batch(&["Move 1 10,0,0".into(), "Sphere 0,0,0 -4".into()])
        .is_err());
    assert_eq!(s.project_json().unwrap(), before);
    assert_eq!(s.undo_len(), history);
    s.batch(&["Move 1 10,0,0".into(), "Move 1 0,10,0".into()])
        .unwrap();
    assert_eq!(s.undo_len(), history + 1);
    s.undo().unwrap();
    assert_eq!(s.project_json().unwrap(), before);
}
#[test]
fn batch_disallows_file_side_effects() {
    let mut s = one();
    assert!(s.batch(&["Save should-not-exist.arcocct".into()]).is_err());
    assert!(s.batch(&["Undo".into()]).is_err());
}
#[test]
fn multi_delete_validates_every_id_first() {
    let mut s = one();
    let before = s.project_json().unwrap();
    assert!(s.execute("Delete 1 99999").is_err());
    assert_eq!(before, s.project_json().unwrap());
}
#[test]
fn array_is_one_undo_step() {
    let mut s = one();
    let n = s.undo_len();
    s.execute("ArrayLinear 1 5 20,0,0").unwrap();
    assert_eq!(s.state().objects.len(), 5);
    assert_eq!(s.undo_len(), n + 1);
    s.undo().unwrap();
    assert_eq!(s.state().objects.len(), 1);
}
#[test]
fn history_is_bounded() {
    let mut s = one();
    for i in 0..150 {
        s.execute(&format!("Rename 1 part{i}")).unwrap();
    }
    assert_eq!(s.undo_len(), MAX_HISTORY);
}
#[test]
fn dirty_state_tracks_save_undo_and_redo() {
    let dir = Temp::new();
    let mut s = one();
    assert!(s.dirty());
    s.save(&dir.0.join("p.arcocct")).unwrap();
    assert!(!s.dirty());
    s.execute("Move 1 1,0,0").unwrap();
    assert!(s.dirty());
    s.undo().unwrap();
    assert!(!s.dirty());
    s.redo().unwrap();
    assert!(s.dirty());
}
#[test]
fn failed_save_keeps_dirty_and_existing_file() {
    let dir = Temp::new();
    let mut s = one();
    let p = dir.0.join("p.arcocct");
    s.save(&p).unwrap();
    let bytes = std::fs::read(&p).unwrap();
    s.execute("Move 1 1,0,0").unwrap();
    assert!(s.save(&dir.0.join("missing/no.arcocct")).is_err());
    assert!(s.dirty());
    assert_eq!(std::fs::read(&p).unwrap(), bytes);
}
#[test]
fn project_round_trip_preserves_layers_and_geometry() {
    let mut s = one();
    s.execute("Layer Metal").unwrap();
    s.execute("Sphere 20,0,0 3").unwrap();
    let data = s.project_json().unwrap();
    let mut t = Session::default();
    t.load_json(&data).unwrap(); // OCCT read/write normalizes topology check/cache flags. Compare semantic state.
    assert_eq!(s.state().current_layer, t.state().current_layer);
    assert_eq!(
        serde_json::to_value(&s.summary().unwrap().layers).unwrap(),
        serde_json::to_value(&t.summary().unwrap().layers).unwrap()
    );
    assert_eq!(s.state().objects.len(), t.state().objects.len());
    for (id, a) in &s.state().objects {
        let b = t.state().object(*id).unwrap();
        assert_eq!(
            (&a.name, a.layer, a.visible, a.locked),
            (&b.name, b.layer, b.visible, b.locked)
        );
        let x = a.shape.stats().unwrap();
        let y = b.shape.stats().unwrap();
        near(x.volume, y.volume, 1e-8);
        near(x.area, y.area, 1e-8);
        near(x.length, y.length, 1e-8);
        assert_eq!((x.faces, x.edges, x.solids), (y.faces, y.edges, y.solids));
        for i in 0..6 {
            near(x.bounds[i], y.bounds[i], 1e-8);
        }
        let am = a.shape.mesh(0.1).unwrap();
        let bm = b.shape.mesh(0.1).unwrap();
        // Triangulation order is not a persistence contract. Validate both sampled
        // boundaries independently against the exact analytic primitives instead.
        mesh_ok(&am);
        mesh_ok(&bm);
        for p in am.positions.iter().chain(&bm.positions) {
            if *id == 1 {
                assert!(p.iter().all(|v| *v >= -1e-8 && *v <= 10.0 + 1e-8));
                assert!(p.iter().any(|v| v.abs() < 1e-8 || (*v - 10.0).abs() < 1e-8));
            } else {
                near((p[0] - 20.0).powi(2) + p[1] * p[1] + p[2] * p[2], 9.0, 1e-7);
            }
        }
    }
    assert!(!t.dirty());
}
#[test]
fn malformed_project_does_not_replace_document() {
    let mut s = one();
    let before = s.project_json().unwrap();
    for bad in [b"{broken".as_slice(), b"{}", b"null"] {
        assert!(s.load_json(bad).is_err());
        assert_eq!(s.project_json().unwrap(), before);
    }
    let mut v: serde_json::Value = serde_json::from_slice(&before).unwrap();
    v["objects"][0]["brep"] = "invalid BREP".into();
    assert!(s.load_json(&serde_json::to_vec(&v).unwrap()).is_err());
    assert_eq!(s.project_json().unwrap(), before);
}
#[test]
fn duplicate_ids_and_wrong_units_are_rejected() {
    let mut s = one();
    let bytes = s.project_json().unwrap();
    let mut v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let item = v["objects"][0].clone();
    v["objects"].as_array_mut().unwrap().push(item);
    assert!(s.load_json(&serde_json::to_vec(&v).unwrap()).is_err());
    let mut v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    v["units"] = "inch".into();
    assert!(s.load_json(&serde_json::to_vec(&v).unwrap()).is_err());
}
#[test]
fn quoted_names_and_windows_paths_are_preserved() {
    assert_eq!(
        tokenize(r#"Open "C:\models\part.arcocct""#).unwrap(),
        vec!["Open", r"C:\models\part.arcocct"]
    );
    assert_eq!(
        tokenize("Rename 1 'precision plate'").unwrap(),
        vec!["Rename", "1", "precision plate"]
    );
    assert!(tokenize("Rename 1 \"unfinished").is_err());
}
#[test]
fn hostile_command_corpus_never_panics() {
    let mut s = one();
    for command in [
        "",
        " ",
        "Box",
        "Box 0,0,0 1 2",
        "Box NaN,0,0 1 1 1",
        "Box 0,0,0 NaN 1 1",
        "Sphere 0,0,0 inf",
        "Move 1 1,2",
        "Loft",
        "NurbsCurve",
        "FilletEdge 1",
        "LayerColor 0 256 0 0",
        "Delete",
        "ArrayLinear 1 999999999 1,0,0",
        "Rotate 1 0,0,0 0,0,0 20",
        "Revolve 1 0,0,0 0,0,1 400",
        "BooleanUnion 1 1",
        "DoNotExist",
        "Polyline",
        "Open \"bad",
    ] {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| s.execute(command)));
        assert!(result.is_ok(), "panicked on {command}");
        assert!(result.unwrap().is_err(), "accepted {command}");
    }
}
#[test]
fn command_catalog_has_unique_names() {
    let mut names = std::collections::BTreeSet::new();
    for h in HELP {
        assert!(names.insert(h.0.to_ascii_lowercase()));
        assert!(!h.1.is_empty());
    }
}
#[test]
fn showcase_has_three_valid_solids() {
    let mut s = Session::default();
    s.execute("Demo").unwrap();
    assert_eq!(s.state().objects.len(), 3);
    for o in s.state().objects.values() {
        let stats = o.shape.stats().unwrap();
        assert_eq!(stats.solids, 1);
        assert!(stats.volume > 0.0);
        mesh_ok(&o.shape.mesh(0.2).unwrap());
    }
    assert_eq!(s.undo_len(), 1);
    s.undo().unwrap();
    assert!(s.state().objects.is_empty());
}

#[test]
fn rational_curve_project_round_trip_retains_analytic_arc() {
    let curve = Shape::nurbs(
        2,
        &[
            [1.0, 0.0, 0.0, 1.0],
            [1.0, 1.0, 0.0, 0.5_f64.sqrt()],
            [0.0, 1.0, 0.0, 1.0],
        ],
    )
    .unwrap();
    let mut s = Session::default();
    let id = s.insert("Rational arc", curve).unwrap();
    let data = s.project_json().unwrap();
    let mut t = Session::default();
    t.load_json(&data).unwrap();
    let shape = &t.state().object(id).unwrap().shape;
    near(shape.stats().unwrap().length, PI / 2.0, 1e-6);
    assert_eq!(shape.stats().unwrap().edges, 1);
    for p in shape.mesh(0.001).unwrap().lines.iter().flatten() {
        near(p[0] * p[0] + p[1] * p[1], 1.0, 1e-6);
        near(p[2], 0.0, 1e-9);
    }
}

#[test]
fn saved_project_can_atomically_replace_existing_file() {
    let dir = Temp::new();
    let path = dir.0.join("replace-existing.arcocct");
    let mut s = one();
    s.save(&path).unwrap();
    s.execute("Move 1 12,0,0").unwrap();
    s.save(&path).unwrap();
    let mut t = Session::default();
    t.open(&path).unwrap();
    near(
        t.state().object(1).unwrap().shape.stats().unwrap().bounds[0],
        12.0,
        1e-7,
    );
    assert!(!s.dirty());
    assert_eq!(std::fs::read_dir(&dir.0).unwrap().count(), 1);
}
#[test]
fn unicode_project_path_round_trip() {
    let dir = Temp::new();
    let path = dir.0.join("\u{8a2d}\u{8a08} \u{8a66}\u{9a13}.arcocct");
    let mut s = one();
    s.save(&path).unwrap();
    let mut t = Session::default();
    t.open(&path).unwrap();
    near(
        t.state().object(1).unwrap().shape.stats().unwrap().volume,
        1000.0,
        1e-7,
    );
}
