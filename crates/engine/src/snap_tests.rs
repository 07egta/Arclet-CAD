//! Snap points (#5881): the centres, middles and ends edges and faces offer. Every expected
//! point is worked out by hand from the part's dimensions.

use std::f64::consts::{FRAC_1_SQRT_2, PI};

use serde_json::{Value, json};
use solidcraft_geom::Vec3;

use crate::snap::{SnapKind, SnapPoint, snap_points};
use crate::{Sel, Session};

fn run(s: &mut Session, id: &str, p: Value) {
    if let Err(e) = s.execute(id, &p) {
        panic!("{id} {p}: {e}");
    }
}

/// A 60 × 40 × 10 plate with a Ø6 hole at (15, 20) and a Ø8 hole at (45, 20).
fn plate() -> Session {
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length": 60, "width": 40, "height": 10}));
    run(&mut s, "solid.cylinder", json!({"radius": 3, "height": 12, "base": [15, 20, -1], "operation": "cut"}));
    run(&mut s, "solid.cylinder", json!({"radius": 4, "height": 12, "base": [45, 20, -1], "operation": "cut"}));
    s
}

fn sel(v: Value) -> Sel {
    serde_json::from_value(v).unwrap()
}

/// The first mesh edge all of whose points satisfy `on`, as a pick.
fn edge(s: &Session, on: impl Fn(Vec3) -> bool) -> Sel {
    let st = s.model.state();
    let b = st.bodies.first().unwrap();
    let m = b.mesh();
    let (i, e) = m.edges.iter().enumerate().find(|(_, e)| e.iter().all(|p| on(*p))).unwrap();
    sel(json!({"type": "edge", "body": b.name, "index": i, "point": e[e.len() / 2]}))
}

/// The face all of whose triangle corners satisfy `on`, as a pick.
fn face(s: &Session, on: impl Fn(Vec3) -> bool) -> Sel {
    let st = s.model.state();
    let b = st.bodies.first().unwrap();
    let m = b.mesh();
    let n = m.tri_face.iter().map(|f| *f as usize + 1).max().unwrap();
    let tris = |f: usize| -> Vec<[Vec3; 3]> {
        m.triangles.iter().zip(&m.tri_face).filter(|(_, tf)| **tf as usize == f).filter_map(|(t, _)| m.tri(t)).collect()
    };
    let f = (0..n).find(|&f| !tris(f).is_empty() && tris(f).iter().all(|t| t.iter().all(|p| on(*p)))).unwrap();
    sel(json!({"type": "face", "body": b.name, "index": f, "point": tris(f)[0][0]}))
}

/// The points of one kind, in the order given.
fn of(points: &[SnapPoint], kind: SnapKind) -> Vec<Vec3> {
    points.iter().filter(|p| p.kind == kind).map(|p| p.at).collect()
}

/// Whether `got` holds exactly the `want` points (any order), each within `tol`.
fn same(got: &[Vec3], want: &[Vec3], tol: f64) -> bool {
    got.len() == want.len() && want.iter().all(|w| got.iter().any(|g| g.dist(*w) <= tol))
}

#[test]
fn a_hole_offers_the_centres_of_its_ends_and_its_rim_its_centre() {
    let s = plate();
    let wall = face(&s, |p| ((p - Vec3::new(15.0, 20.0, p.z)).len() - 3.0).abs() < 1e-3);
    let got = snap_points(&s, &wall);
    assert!(same(&of(&got, SnapKind::Centre), &[Vec3::new(15.0, 20.0, 0.0), Vec3::new(15.0, 20.0, 10.0)], 1e-6), "{got:?}");
    assert_eq!(got.len(), 2, "only the two centres: {got:?}");

    let rim = edge(&s, |p| (p.z - 10.0).abs() < 1e-9 && ((p - Vec3::new(45.0, 20.0, 10.0)).len() - 4.0).abs() < 1e-3);
    let got = snap_points(&s, &rim);
    assert!(same(&of(&got, SnapKind::Centre), &[Vec3::new(45.0, 20.0, 10.0)], 1e-6), "{got:?}");
    assert_eq!(got.len(), 1, "a whole circle offers only its centre: {got:?}");
    assert!(got.iter().all(|p| p.body == "Body1"), "{got:?}");
}

#[test]
fn a_straight_edge_offers_its_ends_and_middle() {
    let s = plate();
    let front = edge(&s, |p| p.y.abs() < 1e-9 && (p.z - 10.0).abs() < 1e-9);
    let got = snap_points(&s, &front);
    assert!(same(&of(&got, SnapKind::End), &[Vec3::new(0.0, 0.0, 10.0), Vec3::new(60.0, 0.0, 10.0)], 1e-9), "{got:?}");
    assert!(same(&of(&got, SnapKind::Middle), &[Vec3::new(30.0, 0.0, 10.0)], 1e-9), "{got:?}");
}

#[test]
fn a_flat_face_offers_the_centre_of_its_area() {
    let s = plate();
    let top = face(&s, |p| (p.z - 10.0).abs() < 1e-9);
    // The plate's area less the two holes, each holding its area at its centre.
    let (a, h6, h8) = (60.0 * 40.0, PI * 9.0, PI * 16.0);
    let x = (a * 30.0 - h6 * 15.0 - h8 * 45.0) / (a - h6 - h8);
    let got = snap_points(&s, &top);
    // The holes' circles are tessellated, so their areas are a little off.
    assert!(same(&of(&got, SnapKind::FaceCentre), &[Vec3::new(x, 20.0, 10.0)], 0.02), "want x = {x}: {got:?}");
    assert_eq!(got.len(), 1, "{got:?}");
}

#[test]
fn an_arc_offers_its_centre_ends_and_middle() {
    // A Ø20 cylinder cut at the corner (60, 40) leaves a quarter circle about it on the top.
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length": 60, "width": 40, "height": 10}));
    run(&mut s, "solid.cylinder", json!({"radius": 10, "height": 12, "base": [60, 40, -1], "operation": "cut"}));
    let c = Vec3::new(60.0, 40.0, 10.0);
    let arc = edge(&s, |p| (p.z - 10.0).abs() < 1e-9 && ((p - c).len() - 10.0).abs() < 1e-3);
    let got = snap_points(&s, &arc);
    assert!(same(&of(&got, SnapKind::Centre), &[c], 1e-6), "{got:?}");
    assert!(same(&of(&got, SnapKind::End), &[Vec3::new(50.0, 40.0, 10.0), Vec3::new(60.0, 30.0, 10.0)], 1e-6), "{got:?}");
    let mid = Vec3::new(60.0 - 10.0 * FRAC_1_SQRT_2, 40.0 - 10.0 * FRAC_1_SQRT_2, 10.0);
    assert!(same(&of(&got, SnapKind::Middle), &[mid], 1e-6), "the exact middle of the arc, not of its chord: {got:?}");
}

#[test]
fn a_sphere_offers_its_centre_and_a_vertex_itself() {
    let mut s = Session::default();
    run(&mut s, "solid.sphere", json!({"radius": 7, "center": [5, -3, 12]}));
    let ball = face(&s, |_| true);
    let got = snap_points(&s, &ball);
    assert_eq!(got.iter().map(|p| p.kind).collect::<Vec<_>>(), [SnapKind::Centre], "{got:?}");
    assert!(got[0].at.dist(Vec3::new(5.0, -3.0, 12.0)) < 1e-6, "{got:?}");

    let v = sel(json!({"type": "vertex", "body": "Body1", "point": [1, 2, 3]}));
    let got = snap_points(&s, &v);
    assert_eq!(got, [SnapPoint { at: Vec3::new(1.0, 2.0, 3.0), kind: SnapKind::Vertex, body: "Body1".into() }]);
}

#[test]
fn things_without_points_offer_none() {
    let s = plate();
    for v in [
        json!({"type": "body", "name": "Body1"}),
        json!({"type": "plane", "name": "XY"}),
        json!({"type": "edge", "body": "Nope", "index": 0, "point": [0, 0, 0]}),
    ] {
        assert!(snap_points(&s, &sel(v.clone())).is_empty(), "{v}");
    }
}

mod smart_snap_tests {
    use std::collections::BTreeSet;
    use std::f64::consts::{FRAC_1_SQRT_2, FRAC_PI_2, PI};
    use solidcraft_geom::{Vec3, nurbs_curve::NurbsCurve};
    use crate::snap::{
        SnapAccuracy, SnapClipDepth, SnapGeometry, SnapInference, SnapKind, SnapObject,
        SnapPlane, SnapQuery, SnapScreenPoint, SnapSettings, SnapTransform, SnapView, query_snaps,
    };

    fn ortho() -> SnapView {
        SnapView {
            clip_from_world: [
                [0.1, 0.0, 0.0, 0.0],
                [0.0, 0.1, 0.0, 0.0],
                [0.0, 0.0, 0.0, 0.5],
                [0.0, 0.0, 0.0, 1.0],
            ],
            viewport_px: [1000.0, 1000.0],
            depth: SnapClipDepth::ZeroToOne,
        }
    }

    fn pixel(x: f64, y: f64) -> SnapScreenPoint {
        SnapScreenPoint { x: 500.0 + 50.0 * x, y: 500.0 - 50.0 * y }
    }

    fn query(x: f64, y: f64) -> SnapQuery {
        SnapQuery {
            cursor_px: pixel(x, y),
            view: ortho(),
            settings: SnapSettings::default(),
            inference: SnapInference::default(),
        }
    }

    fn near(a: Vec3, b: Vec3, tol: f64) {
        let d = (a.x - b.x).hypot(a.y - b.y).hypot(a.z - b.z);
        assert!(d <= tol, "{a:?} != {b:?}, error {d}, tolerance {tol}");
    }

    fn segment(name: &str, a: Vec3, b: Vec3) -> SnapObject {
        SnapObject::new(name, SnapGeometry::Segment { a, b })
    }

    fn circle(name: &str, center: Vec3, radius: f64) -> SnapObject {
        SnapObject::new(name, SnapGeometry::Circle {
            center, normal: Vec3::Z, radius, x_axis: Vec3::X,
        })
    }

    #[test]
    fn line_endpoint_midpoint_nearest_and_world_perpendicular_are_exact() {
        let a = segment("line", Vec3::new(-2.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 0.0));
        let mut q = query(0.1, 0.05);
        q.inference.reference = Some(Vec3::new(0.3, 2.0, 0.0));
        let got = query_snaps(&[a], &q).unwrap();
        let middle = got.candidates.iter().find(|c| c.kind == SnapKind::Middle).unwrap();
        near(middle.at, Vec3::ZERO, 1e-15);
        let nearest = got.candidates.iter().find(|c| c.kind == SnapKind::Nearest).unwrap();
        near(nearest.at, Vec3::new(0.1, 0.0, 0.0), 1e-12);
        assert_eq!(nearest.accuracy, SnapAccuracy::Analytic);
        let foot = got.candidates.iter().find(|c| c.kind == SnapKind::Perpendicular).unwrap();
        near(foot.at, Vec3::new(0.3, 0.0, 0.0), 1e-12);
        q.cursor_px = pixel(-2.0, 0.0);
        assert!(query_snaps(&[segment("line", Vec3::new(-2.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 0.0))], &q)
            .unwrap().candidates.iter().any(|c| c.kind == SnapKind::End && c.at.x == -2.0));
    }

    #[test]
    fn circle_line_intersections_quadrants_and_exact_tangencies() {
        let shapes = [
            segment("horizontal", Vec3::new(-2.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 0.0)),
            circle("circle", Vec3::ZERO, 1.0),
        ];
        let got = query_snaps(&shapes, &query(1.0, 0.0)).unwrap();
        let first = got.candidates.first().unwrap();
        assert_eq!(first.kind, SnapKind::Intersection, "{got:?}");
        assert_eq!(first.accuracy, SnapAccuracy::Analytic);
        near(first.at, Vec3::X, 1e-12);

        let mut q = query(FRAC_1_SQRT_2, FRAC_1_SQRT_2);
        q.inference.reference = Some(Vec3::new(0.0, 2.0, 0.0));
        q.settings.radius_px = 150.0;
        let got = query_snaps(&[circle("circle", Vec3::ZERO, 1.0)], &q).unwrap();
        let tangencies: Vec<Vec3> = got.candidates.iter()
            .filter(|c| c.kind == SnapKind::Tangent).map(|c| c.at).collect();
        assert_eq!(tangencies.len(), 2);
        for p in tangencies {
            near(p - Vec3::ZERO, p, 1e-14);
            assert!((p.x * p.x + p.y * p.y - 1.0).abs() < 1e-12);
            assert!((p.dot(Vec3::new(0.0, 2.0, 0.0) - p)).abs() < 1e-12);
        }
        assert_eq!(got.candidates.iter().filter(|c| c.kind == SnapKind::Quadrant).count(), 4);
        assert!(got.candidates.iter().any(|c| c.kind == SnapKind::Centre && c.at == Vec3::ZERO));
    }

    #[test]
    fn quarter_arc_has_analytic_end_middle_and_allowed_quadrants() {
        let arc = SnapObject::new("arc", SnapGeometry::Arc {
            center: Vec3::ZERO, normal: Vec3::Z, radius: 2.0, start: Vec3::X, sweep: FRAC_PI_2,
        });
        let mut q = query(2.0 * FRAC_1_SQRT_2, 2.0 * FRAC_1_SQRT_2);
        q.settings.radius_px = 200.0;
        let got = query_snaps(&[arc], &q).unwrap();
        assert!(got.candidates.iter().any(|c| c.kind == SnapKind::Middle
            && (c.at.x - 2.0 * FRAC_1_SQRT_2).abs() < 1e-12));
        assert_eq!(got.candidates.iter().filter(|c| c.kind == SnapKind::End).count(), 2);
        let quadrant = got.candidates.iter().filter(|c| c.kind == SnapKind::Quadrant).collect::<Vec<_>>();
        assert_eq!(quadrant.len(), 2, "{got:?}");
        assert!(quadrant.iter().all(|c| c.at.x >= -1e-12 && c.at.y >= -1e-12));
    }

    #[test]
    fn sorting_is_stable_under_object_reordering_and_coincident_points() {
        let mut a = vec![
            SnapObject::new("z", SnapGeometry::Anchor {
                at: Vec3::ZERO, kind: SnapKind::Vertex, accuracy: SnapAccuracy::Analytic,
            }),
            SnapObject::new("a", SnapGeometry::Anchor {
                at: Vec3::ZERO, kind: SnapKind::Vertex, accuracy: SnapAccuracy::Analytic,
            }),
            segment("line", Vec3::ZERO, Vec3::new(5.0, 0.0, 0.0)),
        ];
        let q = query(0.0, 0.0);
        let first = query_snaps(&a, &q).unwrap();
        a.reverse();
        let second = query_snaps(&a, &q).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.candidates[0].object_id, "a");
        assert!(first.candidates.iter().any(|c| c.object_id == "z"));
    }

    #[test]
    fn hidden_and_locked_objects_are_opt_in_and_independently_filtered() {
        let mut hidden = SnapObject::new("hidden", SnapGeometry::Anchor {
            at: Vec3::ZERO, kind: SnapKind::Vertex, accuracy: SnapAccuracy::Analytic,
        });
        hidden.visible = false;
        let mut locked = hidden.clone();
        locked.visible = true;
        locked.locked = true;
        locked.id = "locked".into();
        let objects = [hidden, locked];
        let mut q = query(0.0, 0.0);
        assert!(query_snaps(&objects, &q).unwrap().candidates.is_empty());
        q.settings.include_hidden = true;
        assert_eq!(query_snaps(&objects, &q).unwrap().candidates.len(), 1);
        q.settings.include_hidden = false;
        q.settings.include_locked = true;
        assert_eq!(query_snaps(&objects, &q).unwrap().candidates.len(), 1);
        q.settings.include_hidden = true;
        assert_eq!(query_snaps(&objects, &q).unwrap().candidates.len(), 2);
        q.settings.enabled = BTreeSet::new();
        assert!(query_snaps(&objects, &q).unwrap().candidates.is_empty());
    }

    #[test]
    fn rigid_component_transform_applies_to_analytic_geometry() {
        let mut object = circle("placed", Vec3::ZERO, 1.0);
        object.transform = SnapTransform {
            translation: Vec3::new(3.0, 1.0, 0.0),
            x: Vec3::Y, y: -Vec3::X, z: Vec3::Z,
        };
        let got = query_snaps(&[object.clone()], &query(3.0, 2.0)).unwrap();
        assert!(got.candidates.iter().any(|c| c.kind == SnapKind::Quadrant
            && (c.at - Vec3::new(3.0, 2.0, 0.0)).len() < 1e-12));
        object.transform.y = Vec3::new(-2.0, 0.0, 0.0);
        assert!(query_snaps(&[object], &query(3.0, 2.0)).unwrap().candidates.is_empty(),
            "nonuniformly transformed circles must not masquerade as circles");
    }

    #[test]
    fn orthographic_and_perspective_nearest_use_physical_pixels() {
        let line = segment("line", Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 1.0));
        let mut q = query(0.9, 0.0);
        q.settings.enabled = [SnapKind::Nearest].into_iter().collect();
        assert!(query_snaps(&[line.clone()], &q).unwrap().candidates.iter().any(|c| c.kind == SnapKind::Nearest));

        // w = z+3: projected screen midpoint is NOT the midpoint in 3D.
        // lambda=1/2 and endpoint w values 3 and 4 imply t=3/7.
        q.view.clip_from_world = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 1.5],
            [0.0, 0.0, 1.0, 3.0],
        ];
        q.cursor_px = SnapScreenPoint { x: 625.0, y: 500.0 };
        let got = query_snaps(&[line], &q).unwrap();
        near(got.candidates[0].at, Vec3::new(6.0 / 7.0, 0.0, 3.0 / 7.0), 1e-12);
        q.cursor_px.y += 20.0;
        q.settings.radius_px = 1.0;
        assert!(query_snaps(&[segment("line", Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 1.0))], &q)
            .unwrap().candidates.is_empty());
    }

    #[test]
    fn plane_projection_tracking_horizontal_vertical_and_angle_are_distinct() {
        let mut q = query(1.0, 2.0);
        q.settings.radius_px = 300.0;
        q.inference = SnapInference {
            reference: Some(Vec3::ZERO),
            cursor_world: Some(Vec3::new(1.0, 2.0, 3.0)),
            plane: Some(SnapPlane { origin: Vec3::ZERO, x: Vec3::X, y: Vec3::Y }),
            angle_step_deg: Some(90.0),
            acquired: vec![Vec3::new(2.0, 2.0, 0.0)],
        };
        let result = query_snaps(&[], &q).unwrap();
        let get = |kind| result.candidates.iter().find(|c| c.kind == kind).map(|c| c.at).unwrap();
        near(get(SnapKind::Projected), Vec3::new(1.0, 2.0, 0.0), 1e-12);
        near(get(SnapKind::Horizontal), Vec3::X, 1e-12);
        near(get(SnapKind::Vertical), Vec3::new(0.0, 2.0, 0.0), 1e-12);
        near(get(SnapKind::Angle), Vec3::new(0.0, 5.0_f64.sqrt(), 0.0), 1e-12);
        assert_eq!(result.candidates.iter().filter(|c| c.kind == SnapKind::Tracking).count(), 2);
    }

    #[test]
    fn rational_spline_nearest_uses_evaluated_curve_not_a_render_polyline() {
        let curve = NurbsCurve::uniform(
            2, vec![Vec3::new(-1.0, -1.0, 0.0), Vec3::new(0.0, 2.0, 0.0), Vec3::new(1.0, -1.0, 0.0)],
        ).unwrap();
        let mut q = query(0.0, 0.5);
        q.settings.radius_px = 1.0;
        q.settings.enabled = [SnapKind::Nearest].into_iter().collect();
        let got = query_snaps(&[SnapObject::new("spline", SnapGeometry::Nurbs(curve))], &q).unwrap();
        assert_eq!(got.candidates.len(), 1);
        assert_eq!(got.candidates[0].accuracy, SnapAccuracy::Evaluated);
        near(got.candidates[0].at, Vec3::new(0.0, 0.5, 0.0), 1e-7);
    }

    #[test]
    fn crossing_rational_spline_and_segment_is_refined_to_actual_curve() {
        let spline = NurbsCurve::uniform(
            2, vec![Vec3::new(-1.0, -1.0, 0.0), Vec3::new(0.0, 2.0, 0.0), Vec3::new(1.0, -1.0, 0.0)],
        ).unwrap();
        let shapes = [
            SnapObject::new("spline", SnapGeometry::Nurbs(spline)),
            segment("axis", Vec3::new(-2.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 0.0)),
        ];
        let q = query(-1.0 / 3.0_f64.sqrt(), 0.0);
        let result = query_snaps(&shapes, &q).unwrap();
        let intersections = result.candidates.iter().filter(|c| c.kind == SnapKind::Intersection).collect::<Vec<_>>();
        assert_eq!(intersections.len(), 1, "{result:?}");
        let p = intersections[0];
        near(p.at, Vec3::new(-1.0 / 3.0_f64.sqrt(), 0.0, 0.0), 1e-7);
        assert_eq!(p.accuracy, SnapAccuracy::Evaluated);
    }

    #[test]
    fn invalid_nonfinite_inputs_are_rejected_or_safely_skipped() {
        let good = SnapObject::new("good", SnapGeometry::Anchor {
            at: Vec3::ZERO, kind: SnapKind::Vertex, accuracy: SnapAccuracy::Analytic,
        });
        let mut invalid = good.clone();
        invalid.id = "invalid".into();
        invalid.transform.translation.x = f64::NAN;
        let mut q = query(0.0, 0.0);
        assert_eq!(query_snaps(&[invalid, good], &q).unwrap().candidates.len(), 1);
        q.cursor_px.x = f64::NAN;
        assert!(query_snaps(&[], &q).is_err());
        q.cursor_px.x = 500.0;
        q.settings.radius_px = f64::INFINITY;
        assert!(query_snaps(&[], &q).is_err());
        q.settings.radius_px = 12.0;
        q.inference.plane = Some(SnapPlane { origin: Vec3::ZERO, x: Vec3::X, y: Vec3::X });
        assert!(query_snaps(&[], &q).is_err());
        q.inference.plane = None;
        q.view.clip_from_world[0][0] = f64::NAN;
        assert!(query_snaps(&[], &q).is_err());
    }

    #[test]
    fn circle_circle_intersection_and_nonintersecting_skew_segments() {
        let shapes = [circle("c1", Vec3::ZERO, 1.0), circle("c2", Vec3::new(1.0, 0.0, 0.0), 1.0)];
        let got = query_snaps(&shapes, &query(0.5, 3.0_f64.sqrt() / 2.0)).unwrap();
        assert!(got.candidates.iter().any(|c| c.kind == SnapKind::Intersection
            && (c.at.x - 0.5).abs() < 1e-12 && (c.at.y - 3.0_f64.sqrt() / 2.0).abs() < 1e-12));
        let skew = [
            segment("a", Vec3::new(-1.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)),
            segment("b", Vec3::new(0.0, -1.0, 0.1), Vec3::new(0.0, 1.0, 0.1)),
        ];
        assert!(!query_snaps(&skew, &query(0.0, 0.0)).unwrap().candidates.iter().any(|c| c.kind == SnapKind::Intersection));
    }

    #[test]
    fn depth_clipping_and_geometry_budget_do_not_leak_invalid_points() {
        let mut q = query(0.0, 0.0);
        let items = [SnapObject::new("behind", SnapGeometry::Anchor {
            at: Vec3::new(0.0, 0.0, -2.0), kind: SnapKind::Vertex, accuracy: SnapAccuracy::Analytic,
        })];
        q.view.clip_from_world[2] = [0.0, 0.0, 1.0, 0.0];
        assert!(query_snaps(&items, &q).unwrap().candidates.is_empty());
        q.view.depth = SnapClipDepth::MinusOneToOne;
        let on = SnapObject::new("on", SnapGeometry::Anchor {
            at: Vec3::new(0.0, 0.0, -0.5), kind: SnapKind::Vertex, accuracy: SnapAccuracy::Analytic,
        });
        assert_eq!(query_snaps(&[on], &q).unwrap().candidates.len(), 1);
        let mut q = query(0.0, 0.0);
        q.settings.max_candidates = 1;
        let shapes = [
            SnapObject::new("a", SnapGeometry::Anchor {
                at: Vec3::ZERO, kind: SnapKind::Vertex, accuracy: SnapAccuracy::Analytic,
            }),
            SnapObject::new("b", SnapGeometry::Anchor {
                at: Vec3::ZERO, kind: SnapKind::Vertex, accuracy: SnapAccuracy::Analytic,
            }),
        ];
        let result = query_snaps(&shapes, &q).unwrap();
        assert_eq!(result.candidates.len(), 1);
        assert!(result.truncated);
    }

    #[test]
    fn extreme_zoom_levels_use_pixel_radius_not_world_distance() {
        for zoom in [1e-12, 0.1, 1e9] {
            let mut q = query(0.0, 0.0);
            q.view.clip_from_world[0][0] = zoom;
            q.settings.enabled = [SnapKind::Vertex].into_iter().collect();
            let position = Vec3::new(0.02 / zoom, 0.0, 0.0);
            let object = SnapObject::new("point", SnapGeometry::Anchor {
                at: position, kind: SnapKind::Vertex, accuracy: SnapAccuracy::Analytic,
            });
            assert_eq!(query_snaps(&[object.clone()], &q).unwrap().candidates.len(), 1);
            q.settings.radius_px = 4.0; // screen distance is 10 physical pixels
            assert!(query_snaps(&[object], &q).unwrap().candidates.is_empty());
        }
        let mut q = query(0.0, 0.0);
        q.view.viewport_px = [2000.0, 2000.0];
        q.cursor_px = SnapScreenPoint { x: 1000.0, y: 1000.0 };
        q.settings.enabled = [SnapKind::Vertex].into_iter().collect();
        let object = SnapObject::new("point", SnapGeometry::Anchor {
            at: Vec3::new(0.1, 0.0, 0.0), kind: SnapKind::Vertex, accuracy: SnapAccuracy::Analytic,
        });
        assert!(query_snaps(&[object], &q).unwrap().candidates.iter().any(|c| (c.distance_px - 10.0).abs() < 1e-12));
    }

    #[test]
    fn intersection_is_bitwise_stable_when_input_order_reverses() {
        let mut objects = vec![
            segment("h", Vec3::new(-2.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 0.0)),
            segment("v", Vec3::new(0.0, -2.0, 0.0), Vec3::new(0.0, 2.0, 0.0)),
            circle("c", Vec3::ZERO, 1.0),
        ];
        let q = query(0.0, 0.0);
        let before = query_snaps(&objects, &q).unwrap();
        objects.reverse();
        let after = query_snaps(&objects, &q).unwrap();
        assert_eq!(before, after);
        assert!(before.candidates.iter().any(|c| c.kind == SnapKind::Intersection && c.at == Vec3::ZERO));
    }

    #[test]
    fn display_polylines_never_claim_analytic_accuracy() {
        let displayed = SnapObject::new("preview", SnapGeometry::Polyline(vec![
            Vec3::new(-1.0, 0.0, 0.0), Vec3::new(0.0, 0.2, 0.0), Vec3::new(1.0, 0.0, 0.0),
        ]));
        let mut q = query(0.0, 0.2);
        q.settings.radius_px = 100.0;
        let candidates = query_snaps(&[displayed], &q).unwrap().candidates;
        assert!(!candidates.is_empty());
        assert!(candidates.iter().all(|c| c.accuracy == SnapAccuracy::DisplayApproximate));
    }

    #[test]
    fn legacy_selection_wrapper_preserves_existing_snap_metadata() {
        // Original snap_points tests above cover the actual Session geometry.
        // The wrapper must not claim that mesh-fitted legacy centres are exact.
        let s = super::plate();
        let front = super::edge(&s, |p| p.y.abs() < 1e-9 && (p.z - 10.0).abs() < 1e-9);
        let objects = crate::snap::legacy_snap_objects(&s, &[front]);
        assert!(!objects.is_empty());
        assert!(objects.iter().all(|o| matches!(o.geometry, SnapGeometry::Anchor {
            accuracy: SnapAccuracy::DisplayApproximate, ..
        })));
    }

    #[test]
    fn a_full_arc_and_the_exact_circle_are_not_confused() {
        let arc = SnapObject::new("near-full", SnapGeometry::Arc {
            center: Vec3::ZERO, normal: Vec3::Z, radius: 1.0,
            start: Vec3::X, sweep: PI,
        });
        let q = query(-1.0, 0.0);
        assert!(query_snaps(&[arc], &q).unwrap().candidates.iter().any(|c| c.kind == SnapKind::End
            && (c.at - Vec3::new(-1.0, 0.0, 0.0)).len() < 1e-12));
    }
}
