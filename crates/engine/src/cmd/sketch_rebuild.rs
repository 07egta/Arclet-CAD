//! Curve rebuilding and end-continuity inspection backed by the shared rational-curve maths.
//! Fits retain their original cubic pieces exactly during measurement; only the explicitly
//! requested rebuilt copy is approximated. No source point or constraint is removed or reused.

use serde::Deserialize;
use serde_json::Value;
use solidcraft_geom::nurbs_curve::{self, GeometricContinuity, JoinTolerances, MAX_CURVE_CONTROLS, NurbsCurve, RebuildOptions};
use solidcraft_geom::{Vec2, Vec3};
use solidcraft_sketch::{CurveKind, Sketch};

use super::{CommandSpec, sketch, sketch_store};
use crate::params::bad;
use crate::schema::sketch_rebuild::{
    CurveContinuityOrder, CurveContinuityParams, CurveContinuityResult, CurveContinuityTolerances, SampledDeviationResult, SplineRebuildParams,
    SplineRebuildResult,
};
use crate::schema::{TypedContract, output_schema_for};
use crate::{EngineError, Result, Session};

const REBUILD_CONTRACT: TypedContract<SplineRebuildParams> = TypedContract::new();
const CONTINUITY_CONTRACT: TypedContract<CurveContinuityParams> = TypedContract::new();

pub static COMMANDS: &[CommandSpec] = &[
    CommandSpec::new("sketch.spline.rebuild", "Rebuild Curve", crate::checked!("sketch.spline.rebuild", rebuild))
        .at("SKETCH", "MODIFY")
        .icon("spline_cv")
        .contract(&REBUILD_CONTRACT)
        .output(output_schema_for::<SplineRebuildResult>)
        .params(concat!("curve: non-periodic spline id; sketch?: id|name (default active); degree?: 1..7 (default 3); control_points?: 2..64, greater than degree (default 8); fit_samples?: at least control_points, at most 2049 (default 129); check_samples?: 2..1025 (default 257); construction?: bool (default true). Adds a new uniform-clamped control-point spline with fixed endpoints; retains the source and its constraints. Returns parameter RMS and both directed sampled geometric deviations, not a certified error bound", created_doc!())),
    CommandSpec::new("inspect.curve_continuity", "Curve Continuity", crate::checked!("inspect.curve_continuity", continuity))
        .at("SKETCH", "INSPECT")
        .icon("continuity")
        .noundo()
        .contract(&CONTINUITY_CONTRACT)
        .output(output_schema_for::<CurveContinuityResult>)
        .params("before, after: line or non-periodic spline ids in one sketch; sketch?: id|name (default active); reverse_before?, reverse_after?: bool (default false); gap_tolerance_mm?: 0.000001, angle_tolerance_deg?: 0.01, curvature_tolerance_per_mm?: 0.000001, curvature_change_tolerance_per_mm2?: 0.000001. Measures the first curve's end against the second curve's start, using arc-length derivatives through G3. No geometry is changed; zero-speed endpoints are unmeasurable, and this is not a surface-quality certificate"),
];

fn rebuild(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "sketch.spline.rebuild";
    let params = SplineRebuildParams::deserialize(p).map_err(|e| bad(cmd, e.to_string()))?;
    let options = RebuildOptions {
        degree: params.degree,
        control_points: params.control_points,
        fit_samples: params.fit_samples,
        check_samples: params.check_samples,
    };
    options.validate().map_err(|e| bad(cmd, e.to_string()))?;
    let watch = solidcraft_kernel::watch::current();
    let cancelled = || watch.as_ref().is_some_and(|watch| watch.cancelled());
    let degree = u8::try_from(params.degree).map_err(|e| bad(cmd, e.to_string()))?;
    let ((curve, report), info) = sketch::edit(s, p, cmd, false, |sk, _| {
        if cancelled() {
            return Err(bad(cmd, "curve operation cancelled"));
        }
        if sk.points.len().saturating_add(options.control_points) > solidcraft_sketch::MAX_POINTS || sk.curves.len() >= solidcraft_sketch::MAX_CURVES
        {
            return Err(bad(cmd, "the sketch has no room for another rebuilt curve"));
        }
        let index = sk.curve_index(&params.curve).ok_or_else(|| bad(cmd, format!("unknown curve `{}`", params.curve)))?;
        if !sk.curves.get(index).is_some_and(|curve| matches!(curve.kind, CurveKind::Spline { .. })) {
            return Err(bad(cmd, "rebuild needs a non-periodic sketch spline"));
        }
        let source = curve_geometry(sk, &params.curve, cmd)?;
        let report = nurbs_curve::rebuild(&source, options, cancelled).map_err(|e| bad(cmd, e.to_string()))?;
        if cancelled() {
            return Err(bad(cmd, "curve operation cancelled"));
        }
        let mut points = Vec::with_capacity(report.curve.control_points().len());
        for point in report.curve.control_points() {
            points.push(sk.add_point(Vec2::new(point.x, point.y), None)?);
        }
        let index = sk.add_curve(CurveKind::Spline { pts: points, control: true, degree, periodic: false }, None)?;
        let made = sk.curves.get_mut(index).ok_or_else(|| bad(cmd, "the rebuilt curve is missing"))?;
        made.construction = params.construction;
        Ok((made.id.clone(), report))
    })?;
    let deviation = report.deviation;
    let result = SplineRebuildResult {
        source: params.curve,
        curve,
        degree: params.degree,
        control_points: params.control_points,
        construction: params.construction,
        parameter_rms_mm: report.parameter_rms_mm,
        deviation: SampledDeviationResult {
            source_to_rebuilt_mm: deviation.source_to_rebuilt_mm,
            rebuilt_to_source_mm: deviation.rebuilt_to_source_mm,
            samples_per_curve: deviation.samples_per_curve,
            max_mm: deviation.max_mm(),
        },
        estimate_note: "Deviation uses finite point-to-polyline samples in both directions; features between samples may be missed. It is not a certified Hausdorff bound. Endpoint positions are fixed; endpoint tangents are not constrained.".into(),
        sketch: info,
        created: None,
        param_warnings: Vec::new(),
    };
    serde_json::to_value(result).map_err(|e| EngineError::Other(e.to_string()))
}

fn continuity(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "inspect.curve_continuity";
    let params = CurveContinuityParams::deserialize(p).map_err(|e| bad(cmd, e.to_string()))?;
    let tolerances = JoinTolerances {
        gap_mm: params.gap_tolerance_mm,
        tangent_angle_deg: params.angle_tolerance_deg,
        curvature_per_mm: params.curvature_tolerance_per_mm,
        curvature_change_per_mm2: params.curvature_change_tolerance_per_mm2,
    };
    tolerances.validate().map_err(|e| bad(cmd, e.to_string()))?;
    let watch = solidcraft_kernel::watch::current();
    if watch.as_ref().is_some_and(|watch| watch.cancelled()) {
        return Err(bad(cmd, "curve operation cancelled"));
    }
    let target = sketch_store::target(s, p, cmd)?;
    let sk = sketch_store::read(&s.doc, target)?;
    let mut before = curve_geometry(sk, &params.before, cmd)?;
    let mut after = curve_geometry(sk, &params.after, cmd)?;
    if params.reverse_before {
        before = before.reversed().map_err(|e| bad(cmd, e.to_string()))?;
    }
    if params.reverse_after {
        after = after.reversed().map_err(|e| bad(cmd, e.to_string()))?;
    }
    let report = nurbs_curve::measure_join(&before, &after, tolerances).map_err(|e| bad(cmd, e.to_string()))?;
    let measured_order = report.measured_order.map(|order| match order {
        GeometricContinuity::G0 => CurveContinuityOrder::G0,
        GeometricContinuity::G1 => CurveContinuityOrder::G1,
        GeometricContinuity::G2 => CurveContinuityOrder::G2,
        GeometricContinuity::G3 => CurveContinuityOrder::G3,
    });
    let result = CurveContinuityResult {
        before: params.before,
        after: params.after,
        reverse_before: params.reverse_before,
        reverse_after: params.reverse_after,
        gap_mm: report.gap_mm,
        tangent_angle_deg: report.tangent_angle_deg,
        curvature_jump_per_mm: report.curvature_jump_per_mm,
        curvature_change_jump_per_mm2: report.curvature_change_jump_per_mm2,
        measured_order,
        tolerances: CurveContinuityTolerances {
            gap_mm: tolerances.gap_mm,
            tangent_angle_deg: tolerances.tangent_angle_deg,
            curvature_per_mm: tolerances.curvature_per_mm,
            curvature_change_per_mm2: tolerances.curvature_change_per_mm2,
        },
        scope_note: "Local curve-end measurements with explicit tolerances and directions; not a surface-continuity or Class A certificate.".into(),
        param_warnings: Vec::new(),
    };
    serde_json::to_value(result).map_err(|e| EngineError::Other(e.to_string()))
}

/// Exact conversion of supported sketch geometry, in sketch millimetres. A fit spline has
/// equal parameter spans in `curves::spline_point`, so repeated cubic knots preserve that same
/// parameterization while retaining every original Hermite/Bézier segment.
fn curve_geometry(sk: &Sketch, id: &str, cmd: &str) -> Result<NurbsCurve> {
    let index = sk.curve_index(id).ok_or_else(|| bad(cmd, format!("unknown curve `{id}`")))?;
    let curve = sk.curves.get(index).ok_or_else(|| bad(cmd, "the source curve is missing"))?;
    let point = |index| {
        sk.point(index)
            .filter(|point| point.is_finite())
            .map(|point| point.extend(0.0))
            .ok_or_else(|| bad(cmd, "the curve has a missing or non-finite point"))
    };
    match &curve.kind {
        CurveKind::Line { a, b } if !curve.infinite => NurbsCurve::uniform(1, vec![point(*a)?, point(*b)?]).map_err(|e| bad(cmd, e.to_string())),
        CurveKind::Spline { periodic: true, .. } => Err(bad(cmd, "periodic splines are not supported by this operation yet")),
        CurveKind::Spline { pts, control, degree, periodic: false } => {
            if !(2..=MAX_CURVE_CONTROLS).contains(&pts.len()) {
                return Err(bad(cmd, "the spline has too few or too many control points"));
            }
            if !*control && 3 * (pts.len() - 1) + 1 > MAX_CURVE_CONTROLS {
                return Err(bad(cmd, "the fit spline's exact cubic conversion exceeds 1024 controls"));
            }
            let points = pts.iter().map(|index| point(*index)).collect::<Result<Vec<_>>>()?;
            if *control {
                let degree = usize::from(*degree).clamp(1, solidcraft_geom::MAX_BEZIER_DEGREE).min(points.len() - 1);
                return NurbsCurve::uniform(degree, points).map_err(|e| bad(cmd, e.to_string()));
            }
            let fit_points: Vec<Vec2> = points.iter().map(|point| Vec2::new(point.x, point.y)).collect();
            let pieces = solidcraft_sketch::fit_beziers(&fit_points);
            let Some(first) = pieces.first() else { return Err(bad(cmd, "the fit spline has no cubic pieces")) };
            let count = pieces.len();
            let mut controls: Vec<Vec3> = first.iter().map(|point| point.extend(0.0)).collect();
            for piece in pieces.iter().skip(1) {
                controls.extend(piece.iter().skip(1).map(|point| point.extend(0.0)));
            }
            let mut knots = vec![0.0; 4];
            for index in 1..count {
                knots.extend(std::iter::repeat_n(index as f64 / count as f64, 3));
            }
            knots.extend(std::iter::repeat_n(1.0, 4));
            let weights = vec![1.0; controls.len()];
            NurbsCurve::new(3, knots, controls, weights).map_err(|e| bad(cmd, e.to_string()))
        }
        _ => Err(bad(cmd, "this operation accepts finite lines and non-periodic fit/control-point splines")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use solidcraft_doc::Document;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn run(s: &mut Session, cmd: &str, params: Value) -> Value {
        s.execute(cmd, &params).unwrap_or_else(|error| panic!("{cmd}: {error}"))
    }

    fn sketch_session() -> Session {
        let mut session = Session::default();
        run(&mut session, "sketch.create", json!({"plane":"XY", "name":"Curves"}));
        session
    }

    fn spline(s: &mut Session, points: Value, control: bool) -> String {
        let command = if control { "sketch.spline.control_point" } else { "sketch.spline.fit_point" };
        run(s, command, json!({"points": points}))["curves"][0].as_str().unwrap().to_string()
    }

    #[test]
    fn rebuild_adds_an_editable_copy_preserves_source_and_survives_undo_and_cold_load() {
        let mut s = sketch_session();
        let source = spline(&mut s, json!([[0, 0], [10, 20], [20, -10], [30, 0]]), true);
        let target = sketch_store::target(&s, &json!({}), "test").unwrap();
        let fixed_point = {
            let sk = sketch_store::read(&s.doc, target).unwrap();
            let CurveKind::Spline { pts, .. } = &sk.curves[sk.curve_index(&source).unwrap()].kind else { panic!("spline") };
            sk.points[pts[1]].id.clone()
        };
        run(&mut s, "sketch.constraint.fix", json!({"entity":fixed_point,"fixed":true}));
        let before = sketch_store::read(&s.doc, target).unwrap().clone();
        assert!(!before.constraints.is_empty());
        let before_json = s.doc.to_json();
        let reply = run(&mut s, "sketch.spline.rebuild", json!({"curve": source, "degree":3,"control_points":4,"fit_samples":33,"check_samples":65}));
        let made = reply["curve"].as_str().unwrap();
        assert_ne!(made, source);
        assert_eq!(reply["source"], source);
        assert_eq!(reply["construction"], true);
        assert!(reply["parameter_rms_mm"].as_f64().unwrap() < 1e-10);
        assert!(reply["deviation"]["max_mm"].as_f64().unwrap() < 1e-10);
        let after = sketch_store::read(&s.doc, target).unwrap();
        assert_eq!(&after.points[..before.points.len()], before.points.as_slice());
        assert_eq!(&after.curves[..before.curves.len()], before.curves.as_slice());
        assert_eq!(after.constraints, before.constraints);
        let rebuilt = &after.curves[after.curve_index(made).unwrap()];
        assert!(rebuilt.construction);
        let CurveKind::Spline { pts, control, degree, periodic } = &rebuilt.kind else { panic!("spline") };
        assert!(*control && !*periodic);
        assert_eq!((*degree, pts.len()), (3, 4));
        assert!(pts.iter().all(|index| *index >= before.points.len()));
        assert_eq!(reply["created"]["points"].as_array().unwrap().len(), 4);
        assert_eq!(reply["created"]["constraints"], json!([]));
        let after_json = s.doc.to_json();
        run(&mut s, "edit.undo", json!({}));
        assert_eq!(s.doc.to_json(), before_json);
        run(&mut s, "edit.redo", json!({}));
        assert_eq!(s.doc.to_json(), after_json);
        let reopened = Session::new(Document::from_json(&after_json).unwrap());
        let cold = sketch_store::read(&reopened.doc, target).unwrap();
        assert_eq!(curve_geometry(cold, made, "test").unwrap(), curve_geometry(sketch_store::read(&s.doc, target).unwrap(), made, "test").unwrap());
    }

    #[test]
    fn fit_spline_adapter_retains_the_exact_piecewise_curve_and_rebuilds_it() {
        let mut s = sketch_session();
        let points = [Vec2::new(0.0, 0.0), Vec2::new(2.0, 5.0), Vec2::new(9.0, -3.0), Vec2::new(20.0, 1.0)];
        let source = spline(&mut s, json!(points), false);
        let target = sketch_store::target(&s, &json!({}), "test").unwrap();
        let curve = curve_geometry(sketch_store::read(&s.doc, target).unwrap(), &source, "test").unwrap();
        for i in 0..=100 {
            let t = f64::from(i) / 100.0;
            let expected = solidcraft_sketch::spline_point(&points, false, 3, t).extend(0.0);
            assert!(curve.eval(t).unwrap().dist(expected) < 1e-11);
        }
        let result =
            run(&mut s, "sketch.spline.rebuild", json!({"curve":source,"control_points":8,"construction":false,"fit_samples":65,"check_samples":65}));
        assert_eq!(result["construction"], false);
        let made = result["curve"].as_str().unwrap();
        let after = curve_geometry(sketch_store::read(&s.doc, target).unwrap(), made, "test").unwrap();
        assert_eq!(after.eval(0.0).unwrap(), curve.eval(0.0).unwrap());
        assert_eq!(after.eval(1.0).unwrap(), curve.eval(1.0).unwrap());
    }

    #[test]
    fn continuity_uses_explicit_direction_and_does_not_mutate_geometry() {
        let mut s = sketch_session();
        let before = run(&mut s, "sketch.line", json!({"points":[[-10,0],[0,0]]}))["curves"][0].clone();
        let after = spline(&mut s, json!([[0, 0], [10, 0]]), true);
        let saved = s.doc.to_json();
        let undo_count = s.undo.len();
        let report = run(&mut s, "inspect.curve_continuity", json!({"before":before,"after":after}));
        assert_eq!(report["measured_order"], "G3");
        assert_eq!(report["gap_mm"], 0.0);
        assert_eq!(s.doc.to_json(), saved);
        assert_eq!(s.undo.len(), undo_count);
        let report = run(&mut s, "inspect.curve_continuity", json!({"before":before,"after":after,"reverse_after":true}));
        assert!(report["tangent_angle_deg"].as_f64().unwrap() > 179.9);
        assert!(report["measured_order"].is_null());
        assert_eq!(s.doc.to_json(), saved);
    }

    #[test]
    fn invalid_counts_unsupported_sources_and_cancellation_are_atomic() {
        let mut s = sketch_session();
        let source = spline(&mut s, json!([[0, 0], [10, 5], [20, 0]]), false);
        let periodic = run(&mut s, "sketch.spline.fit_point", json!({"points":[[0,0],[4,0],[2,3]],"periodic":true}))["curves"][0].clone();
        let circle = run(&mut s, "sketch.circle.center", json!({"center":[30,0],"radius":2}))["curves"][0].clone();
        let saved = s.doc.to_json();
        let undo_count = s.undo.len();
        for params in [
            json!({"curve":source,"control_points":3,"degree":3}),
            json!({"curve":source,"degree":8}),
            json!({"curve":source,"fit_samples":2}),
            json!({"curve":source,"check_samples":1026}),
            json!({"curve":source,"degree":3.5}),
            json!({"curve":source,"construction":"yes"}),
            json!({"curve":periodic}),
            json!({"curve":circle}),
            json!({"curve":"missing"}),
            json!({"curve":"x".repeat(257)}),
            json!({"curve":"bad\u{0000}id"}),
            json!({"curve":source,"sketch":null}),
        ] {
            assert!(s.execute("sketch.spline.rebuild", &params).is_err(), "{params}");
            assert_eq!(s.doc.to_json(), saved);
            assert_eq!(s.undo.len(), undo_count);
        }
        assert!(s.execute("inspect.curve_continuity", &json!({"before":source,"after":source,"gap_tolerance_mm":-1})).is_err());
        struct Stop;
        impl solidcraft_kernel::watch::Watch for Stop {
            fn cancelled(&self) -> bool {
                true
            }
            fn progress(&self, _: &'static str, _: usize, _: usize) {}
        }
        let result = solidcraft_kernel::watch::watching(Arc::new(Stop), || s.execute("sketch.spline.rebuild", &json!({"curve":source})));
        assert!(result.is_err());
        assert_eq!(s.doc.to_json(), saved);
        assert_eq!(s.undo.len(), undo_count);
    }

    #[test]
    fn cancellation_during_fit_leaves_points_constraints_and_history_untouched() {
        struct StopLater(AtomicUsize);
        impl solidcraft_kernel::watch::Watch for StopLater {
            fn cancelled(&self) -> bool {
                self.0.fetch_add(1, Ordering::Relaxed) >= 40
            }
            fn progress(&self, _: &'static str, _: usize, _: usize) {}
        }
        let mut s = sketch_session();
        let source = spline(&mut s, json!([[0, 0], [10, 20], [20, -10], [30, 0]]), true);
        let before = s.doc.to_json();
        let undo_count = s.undo.len();
        let watch = Arc::new(StopLater(AtomicUsize::new(0)));
        let result = solidcraft_kernel::watch::watching(watch.clone(), || s.execute("sketch.spline.rebuild", &json!({"curve":source})));
        assert!(result.is_err());
        assert!(watch.0.load(Ordering::Relaxed) > 40);
        assert_eq!(s.doc.to_json(), before);
        assert_eq!(s.undo.len(), undo_count);
    }

    #[test]
    fn exact_source_conversion_refuses_expansion_beyond_its_control_budget() {
        let mut sketch = Sketch::new();
        let points = (0..343).map(|i| sketch.add_point(Vec2::new(f64::from(i), 0.0), None).unwrap()).collect();
        let index = sketch.add_curve(CurveKind::Spline { pts: points, control: false, degree: 3, periodic: false }, None).unwrap();
        assert!(curve_geometry(&sketch, &sketch.curves[index].id, "test").is_err());
    }
}
