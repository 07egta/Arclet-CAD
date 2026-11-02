//! A shallow circular cut after a pocket (#1765): repeated empty numerical answers must not
//! stop the later geometric strategies. Inject the numerical failure on a real small part;
//! the exact rounding and ray classification of the original design are not available.

use super::*;
use crate::{box_solid, cylinder, fillet, measure, sphere, step_export, step_import};
use std::cell::Cell;
use std::f64::consts::PI;

/// A box `[c, c + s]`, as in the issue_616 tests.
fn bx(c: [f64; 3], s: [f64; 3]) -> Body {
    box_solid(Vec3::new(c[0], c[1], c[2]), Vec3::new(c[0] + s[0], c[1] + s[1], c[2] + s[2])).unwrap()
}

/// Exact volume through `measure`, as in the issue_616 tests.
fn volume(b: &Body) -> f64 {
    measure(b).unwrap().volume
}

#[test]
fn shallow_cut_after_a_pocket_recovers_from_empty_numerical_attempts() {
    let plate = box_solid(Vec3::ZERO, Vec3::new(40.0, 30.0, 10.0)).unwrap();
    let plate = fillet(&plate, &[Vec3::new(20.0, 0.0, 10.0)], 2.0).unwrap();
    let pocket = box_solid(Vec3::new(5.0, 15.0, 9.25), Vec3::new(35.0, 25.0, 10.25)).unwrap();
    let target = boolean(&plate, &pocket, BoolOp::Cut).unwrap().unwrap();
    let tool = cylinder(Vec3::new(12.0, 15.0, 8.5), Vec3::Z, 5.0, 1.75).unwrap();
    let attempts = Cell::new(0);
    let empty = |_: &Body, _: &Body, _: BoolOp, _: Option<mt::Vector3>, _: f64| {
        attempts.set(attempts.get() + 1);
        Ok((Solid::new_unchecked(Vec::new()), None))
    };
    let (result, diagnostics) =
        capture_boolean_diagnostics(|| boolean_via(&target, &tool, BoolOp::Cut, &|a, b, op| boolean_whole_with(a, b, op, false, &empty)));
    assert!(attempts.get() >= 2, "the first two numerical tries must really have failed");
    let cut = result.unwrap().expect("a shallow cut leaves the plate");
    assert!(cut.validity().is_empty(), "{:?}", cut.validity());
    assert_eq!(cut.lumps().unwrap().len(), 1);
    // Half of the circle overlaps the existing 0.75 mm pocket.
    let expected = 40.0 * 30.0 * 10.0 - (4.0 - PI) * 40.0 - 30.0 * 10.0 * 0.75 - PI * 25.0 * (1.5 - 0.75 / 2.0);
    let actual = measure(&cut).unwrap().volume;
    assert!((actual - expected).abs() < 0.02, "{actual} vs {expected}");
    assert!(diagnostics.events.iter().all(|e| e.strategy != BooleanStrategy::RefinedTolerance));
}

#[test]
fn step_seam_cut_recovers_from_empty_numerical_attempts() {
    let pin = cylinder(Vec3::new(-5.0, 0.0, 0.0), Vec3::X, 4.5, 10.0).unwrap();
    let target = step_import(&step_export(&[&pin], "seam regression").unwrap()).unwrap().bodies.remove(0).body;
    let tool = sphere(Vec3::new(-5.0, 0.0, 0.0), 6.0).unwrap();
    assert!(crate::seam_faces::needed(&target, &tool));
    let attempts = Cell::new(0);
    let shifted_attempts = Cell::new(0);
    let empty = |_: &Body, _: &Body, _: BoolOp, shift: Option<mt::Vector3>, _: f64| {
        attempts.set(attempts.get() + 1);
        if shift.is_some() {
            shifted_attempts.set(shifted_attempts.get() + 1);
        }
        Ok((Solid::new_unchecked(Vec::new()), None))
    };
    let (result, diagnostics) =
        capture_boolean_diagnostics(|| boolean_via(&target, &tool, BoolOp::Cut, &|a, b, op| boolean_whole_with(a, b, op, false, &empty)));
    assert!(attempts.get() >= 2, "the initial numerical tries must really have failed");
    assert_eq!(shifted_attempts.get(), 0, "seam recovery must precede displaced retries");
    let cut = result.unwrap().expect("the spherical cut leaves the end of the pin");
    assert!(cut.validity().is_empty(), "{:?}", cut.validity());
    assert_eq!(cut.lumps().unwrap().len(), 1);
    assert!(cut.history().is_some());
    // Along the common axis, integrate min(pi*4.5^2, pi*(6^2-x^2)) from 0 to 6.
    let full_radius_end = (6.0_f64.powi(2) - 4.5_f64.powi(2)).sqrt();
    let removed = PI * (4.5_f64.powi(2) * full_radius_end + 36.0 * (6.0 - full_radius_end) - (216.0 - full_radius_end.powi(3)) / 3.0);
    let expected = PI * 4.5_f64.powi(2) * 10.0 - removed;
    let actual = measure(&cut).unwrap().volume;
    assert!((actual - expected).abs() < expected * 1e-3, "{actual} vs {expected}");
    assert!(diagnostics.warning().is_none(), "{:?}", diagnostics.warning());
}

#[test]
fn unrecovered_empty_attempts_keep_refusal_without_displacing_tool() {
    // These spheres keep fine_tols empty, isolating retries after the exact local strategies.
    let target = sphere(Vec3::ZERO, 10.0).unwrap();
    let tool = sphere(Vec3::new(8.0, 0.0, 0.0), 5.0).unwrap();
    let shifted_attempts = Cell::new(0);
    let empty = |_: &Body, _: &Body, _: BoolOp, shift: Option<mt::Vector3>, _: f64| {
        if shift.is_some() {
            shifted_attempts.set(shifted_attempts.get() + 1);
        }
        Ok((Solid::new_unchecked(Vec::new()), None))
    };
    let result = boolean_via(&target, &tool, BoolOp::Cut, &|a, b, op| boolean_whole_with(a, b, op, false, &empty));
    assert_eq!(shifted_attempts.get(), 0, "unrecovered empty attempts must not enable displaced retries");
    let error = result.unwrap_err();
    assert!(error.to_string().contains("it came out empty"), "{error}");
}

/// A bounding box is not a solid: the sphere's bounds enclose the cube, but the
/// cube corners remain outside the sphere. A failed numerical cut cannot return empty.
#[test]
fn empty_retry_rejects_ball_whose_box_covers_the_target() {
    let target = bx([0., 0., 0.], [2., 2., 2.]);
    let tool = sphere(Vec3::new(1., 1., 1.), 1.5).unwrap();
    let target_volume = volume(&target);
    let target_box = target.tessellate(0.01).unwrap().bounds();
    let tool_box = tool.tessellate(0.01).unwrap().bounds();
    assert!(target_box.min.x >= tool_box.min.x && target_box.min.y >= tool_box.min.y && target_box.min.z >= tool_box.min.z);
    assert!(target_box.max.x <= tool_box.max.x && target_box.max.y <= tool_box.max.y && target_box.max.z <= tool_box.max.z);
    let error = crate::boolean_guard::check_retry(&target, &tool, BoolOp::Cut, None).unwrap_err();
    assert!(error.to_string().contains("refused"), "{error}");
    // The failed retry took the target by shared reference and did not mutate it.
    assert!((volume(&target) - target_volume).abs() < 1e-10);
    assert!(target.validity().is_empty(), "{:?}", target.validity());
}

/// A cavity may lie entirely inside the tool's bounds. Refuse an empty cut of a
/// target inside that cavity: no material was removed, even though both boxes overlap.
#[test]
fn empty_retry_rejects_hollow_tool_enclosing_the_target() {
    let target = bx([0., 0., 0.], [2., 2., 2.]);
    let outer = bx([-3., -3., -3.], [8., 8., 8.]);
    let cavity = bx([-1., -1., -1.], [4., 4., 4.]);
    let tool = boolean(&outer, &cavity, BoolOp::Cut).unwrap().unwrap();
    assert!(tool.validity().is_empty(), "{:?}", tool.validity());
    let error = crate::boolean_guard::check_retry(&target, &tool, BoolOp::Cut, None).unwrap_err();
    assert!(error.to_string().contains("outside the tool"), "{error}");
    assert!((volume(&target) - 8.0).abs() < 1e-8);
}

/// The cutter spans metres but its void is only millimetres from a tiny target.
/// Using the cutter's diagonal as a proximity tolerance used to treat the
/// target as touching the inner wall and incorrectly accept an empty result.
#[test]
fn empty_retry_rejects_small_part_in_a_huge_cutters_void() {
    let target = bx([0., 0., 0.], [2., 2., 2.]);
    let outer = bx([-1000., -1000., -1000.], [2000., 2000., 2000.]);
    let cavity = bx([-5., -5., -5.], [12., 12., 12.]);
    let tool = boolean(&outer, &cavity, BoolOp::Cut).unwrap().unwrap();
    assert!(tool.validity().is_empty(), "{:?}", tool.validity());
    let error = crate::boolean_guard::check_retry(&target, &tool, BoolOp::Cut, None).unwrap_err();
    assert!(error.to_string().contains("refused"), "{error}");
    assert!((volume(&target) - 8.0).abs() < 1e-8);
}

/// A true complete cut is valid: a larger ball contains the target, not merely
/// its bounding box. Preserve this important success path for recovered retries.
#[test]
fn empty_retry_accepts_a_ball_that_contains_the_entire_target() {
    let target = bx([0., 0., 0.], [2., 2., 2.]);
    let tool = sphere(Vec3::new(1., 1., 1.), 2.1).unwrap();
    assert!(crate::boolean_guard::check_retry(&target, &tool, BoolOp::Cut, None).is_ok());
}

/// A coplanar enclosing box is a valid full cut. It should not be refused just
/// because every sampled target surface lies on (or inside) the cutter's boundary.
#[test]
fn empty_retry_accepts_exactly_coincident_boxes() {
    let target = bx([0., 0., 0.], [2., 2., 2.]);
    let tool = bx([0., 0., 0.], [2., 2., 2.]);
    assert!(crate::boolean_guard::check_retry(&target, &tool, BoolOp::Cut, None).is_ok());
}
