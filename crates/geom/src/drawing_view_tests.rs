use super::*;
use crate::section::validate_plane;

const EPS: f64 = 1e-12;

fn close(a: Vec3, b: Vec3) -> bool {
    (a - b).len() < EPS
}

/// A 50 mm cube's corners, from the origin.
fn cube() -> Vec<Vec3> {
    let s = 50.0;
    (0..8).map(|i| Vec3::new(f64::from(i & 1) * s, f64::from((i >> 1) & 1) * s, f64::from((i >> 2) & 1) * s)).collect()
}

/// The paper extents (width, height) of points seen through a frame.
fn extents(frame: &ViewFrame, points: &[Vec3]) -> Vec2 {
    let paper: Vec<Vec2> = points.iter().map(|p| frame.to_paper(*p)).collect();
    let span = |f: fn(&Vec2) -> f64| {
        let values = paper.iter().map(f);
        values.clone().fold(f64::NEG_INFINITY, f64::max) - values.fold(f64::INFINITY, f64::min)
    };
    Vec2::new(span(|p| p.x), span(|p| p.y))
}

fn full_size(view: NamedView) -> ViewFrame {
    ViewFrame::new(view.basis(), Scale::new(1.0, 1.0).unwrap(), Vec2::ZERO, Vec2::ZERO).unwrap()
}

/// Every named view is an exact, right-handed, orthonormal basis whose normal points back at the
/// viewer: the principal views from where their names say, the isometric views from their
/// upper corners with paper x horizontal.
#[test]
fn each_named_view_looks_from_where_its_name_says() {
    let viewer = [
        (NamedView::Front, Vec3::new(0.0, -1.0, 0.0)),
        (NamedView::Back, Vec3::Y),
        (NamedView::Top, Vec3::Z),
        (NamedView::Bottom, Vec3::new(0.0, 0.0, -1.0)),
        (NamedView::Left, Vec3::new(-1.0, 0.0, 0.0)),
        (NamedView::Right, Vec3::X),
        (NamedView::IsoFrontRight, Vec3::new(1.0, -1.0, 1.0) * (1.0 / 3f64.sqrt())),
        (NamedView::IsoFrontLeft, Vec3::new(-1.0, -1.0, 1.0) * (1.0 / 3f64.sqrt())),
        (NamedView::IsoBackRight, Vec3::new(1.0, 1.0, 1.0) * (1.0 / 3f64.sqrt())),
        (NamedView::IsoBackLeft, Vec3::new(-1.0, 1.0, 1.0) * (1.0 / 3f64.sqrt())),
    ];
    assert_eq!(viewer.len(), NamedView::ALL.len());
    for (view, toward_viewer) in viewer {
        let b = view.basis();
        validate_plane(&b).unwrap();
        assert!(close(b.normal(), toward_viewer), "{view:?}: normal {:?}", b.normal());
        assert_eq!(b.origin, Vec3::ZERO);
        assert!(b.y.z >= 0.0, "{view:?}: paper y never points down");
        if matches!(view, NamedView::IsoFrontRight | NamedView::IsoFrontLeft | NamedView::IsoBackRight | NamedView::IsoBackLeft) {
            assert!(b.x.z.abs() < EPS, "{view:?}: paper x is horizontal");
        }
    }
    // The principal views' axes are exact, no rounding from a camera, and each lies on its paper
    // as the front view folded about the edge they share: the top and bottom views keep x along
    // +X, the back and side views keep y along +Z.
    let minus = |v: Vec3| Vec3::ZERO - v;
    let principal = [
        (NamedView::Front, Vec3::X, Vec3::Z),
        (NamedView::Back, minus(Vec3::X), Vec3::Z),
        (NamedView::Top, Vec3::X, Vec3::Y),
        (NamedView::Bottom, Vec3::X, minus(Vec3::Y)),
        (NamedView::Left, minus(Vec3::Y), Vec3::Z),
        (NamedView::Right, Vec3::Y, Vec3::Z),
    ];
    for (view, x, y) in principal {
        assert_eq!((view.basis().x, view.basis().y), (x, y), "{view:?}");
    }
    // The sketch plane XZ is not the front view: its y is -Z, a front view's is +Z.
    assert_eq!(Plane::XZ.y, Vec3::new(0.0, 0.0, -1.0));
    assert_eq!(NamedView::Front.basis().y, Vec3::Z);
    // No axis has a -0, so a view writes out as `[-1.0, 0.0, 0.0]`, never `[-1.0, -0.0, -0.0]`.
    let negative_zero = |v: Vec3| [v.x, v.y, v.z].iter().any(|c| *c == 0.0 && c.is_sign_negative());
    assert!(NamedView::ALL.iter().all(|v| !negative_zero(v.basis().x) && !negative_zero(v.basis().y)));
}

/// A view from a direction and an up gives the named views' bases (the principal ones exactly),
/// keeps paper y in the plane of the up, and refuses a direction or up it can't make a view from.
#[test]
fn a_view_from_a_direction_and_up_matches_the_named_views() {
    for view in NamedView::ALL {
        let named = view.basis();
        // Looking down, up is +Y (the back at the top); looking up, -Y (the front at the top).
        let up = match view {
            NamedView::Top => Vec3::Y,
            NamedView::Bottom => Vec3::new(0.0, -1.0, 0.0),
            _ => Vec3::Z,
        };
        let built = basis_looking(-named.normal(), up, Vec3::ZERO).unwrap();
        assert!(close(built.x, named.x) && close(built.y, named.y), "{view:?}: {built:?}");
    }
    assert_eq!(basis_looking(Vec3::Y, Vec3::Z, Vec3::ZERO).unwrap(), NamedView::Front.basis(), "exact");
    let leaning = basis_looking(Vec3::Y, Vec3::new(0.0, 0.1, 1.0), Vec3::new(1.0, 2.0, 3.0)).unwrap();
    assert_eq!((leaning.x, leaning.y, leaning.origin), (Vec3::X, Vec3::Z, Vec3::new(1.0, 2.0, 3.0)));
    let refused = [
        (Vec3::ZERO, Vec3::Z, Vec3::ZERO),
        (Vec3::new(f64::NAN, 1.0, 0.0), Vec3::Z, Vec3::ZERO),
        (Vec3::Y, Vec3::ZERO, Vec3::ZERO),
        (Vec3::Y, Vec3::new(0.0, f64::INFINITY, 0.0), Vec3::ZERO),
        (Vec3::new(0.0, 0.0, -1.0), Vec3::Z, Vec3::ZERO),
        (Vec3::Z, Vec3::new(1e-12, 0.0, 1.0), Vec3::ZERO),
        (Vec3::Y, Vec3::Z, Vec3::new(f64::NAN, 0.0, 0.0)),
        (Vec3::Y, Vec3::Z, Vec3::new(0.0, 1e13, 0.0)),
    ];
    for (direction, up, origin) in refused {
        assert!(basis_looking(direction, up, origin).is_err(), "{direction:?} {up:?} {origin:?}");
    }
    // A bad origin is named as such, not as a bad basis.
    for origin in [Vec3::new(f64::NAN, 0.0, 0.0), Vec3::new(0.0, 1e13, 0.0)] {
        assert_eq!(basis_looking(Vec3::Y, Vec3::Z, origin), Err(Invalid("a view's origin is finite and within 1e12 mm of the world's")));
    }
    assert_eq!(serde_json::from_value::<NamedView>(serde_json::json!("iso")).unwrap(), NamedView::IsoFrontRight);
    assert_eq!(serde_json::to_value(NamedView::IsoFrontRight).unwrap(), serde_json::json!("iso_front_right"));
}

/// An up just off the view direction still gives a square basis: its normal is the direction back
/// at the viewer to rounding, where the cross product alone would tilt it by up to 9e-9 rad.
#[test]
fn an_up_just_off_the_direction_still_looks_along_it() {
    for (direction, off) in [(Vec3::new(0.3, -0.7, 0.2), 1.0001e-9), (Vec3::new(1.0, 2.0, 3.0), 1.2e-9), (Vec3::Y, 1.0001e-9)] {
        let d = direction.normalized().unwrap();
        let aside = d.cross(Vec3::Z).normalized().unwrap();
        let basis = basis_looking(direction, d + aside * off, Vec3::ZERO).unwrap();
        let tilt = (basis.normal() + d).len();
        assert!(tilt < 1e-14 && basis.x.dot(d).abs() < 1e-15, "{direction:?}: tilted {tilt:e}");
    }
}

/// A 50 mm cube is a 50 mm square in every principal view at 1:1, and half that at 1:2. In an
/// orthographic isometric view each edge reads 50·√(2/3), and the outline is 50·√2 wide.
#[test]
fn a_50_mm_cube_measures_as_the_views_say() {
    for view in [NamedView::Front, NamedView::Back, NamedView::Top, NamedView::Bottom, NamedView::Left, NamedView::Right] {
        let size = extents(&full_size(view), &cube());
        assert!((size - Vec2::new(50.0, 50.0)).len() < EPS, "{view:?}: {size:?}");
        let half = ViewFrame::new(view.basis(), "1:2".parse().unwrap(), Vec2::ZERO, Vec2::ZERO).unwrap();
        assert!((extents(&half, &cube()) - Vec2::new(25.0, 25.0)).len() < EPS, "{view:?} at 1:2");
    }
    // Orthographic isometric: a vertical edge reads 50·√(2/3), and the outline is 50·√2 wide.
    let iso = full_size(NamedView::IsoFrontRight);
    let edge = iso.to_paper(Vec3::new(0.0, 0.0, 50.0)) - iso.to_paper(Vec3::ZERO);
    assert!((edge.len() - 50.0 * (2.0f64 / 3.0).sqrt()).abs() < EPS, "{edge:?}");
    assert!(edge.x.abs() < EPS, "a vertical edge stays vertical on paper");
    let size = extents(&iso, &cube());
    assert!((size.x - 50.0 * 2f64.sqrt()).abs() < EPS, "{size:?}");
}

/// The view's centre lands on its position, and the map scales about it.
#[test]
fn the_views_centre_lands_on_its_position() {
    let front = NamedView::Front.basis();
    let frame = ViewFrame::new(front, "2:1".parse().unwrap(), Vec2::new(50.0, 10.0), Vec2::new(210.0, 148.5)).unwrap();
    assert!((frame.to_paper(Vec3::new(50.0, 0.0, 10.0)) - Vec2::new(210.0, 148.5)).len() < EPS);
    assert!((frame.to_paper(Vec3::new(0.0, 7.0, 0.0)) - Vec2::new(110.0, 128.5)).len() < EPS, "depth doesn't move a point");
    // W07's example: a 100 x 20 front view on A3 at 2:1, centred: boundary 110..310 x 128.5..168.5.
    assert!((frame.to_paper(Vec3::new(100.0, 0.0, 20.0)) - Vec2::new(310.0, 168.5)).len() < EPS);
    let boundary = frame.boundary(&Aabb2 { min: Vec2::ZERO, max: Vec2::new(100.0, 20.0) }).unwrap();
    assert!(boundary.min.dist(Vec2::new(110.0, 128.5)) < EPS && boundary.max.dist(Vec2::new(310.0, 168.5)) < EPS, "{boundary:?}");
    // A box that is inside out, or not finite on paper, has no boundary: never one that drops a
    // corner. 1e308 is finite in the plane, but not at 2:1.
    let box_of = |x0: f64, y0: f64, x1: f64, y1: f64| Aabb2 { min: Vec2::new(x0, y0), max: Vec2::new(x1, y1) };
    let refused = [
        Aabb2::EMPTY,
        box_of(10.0, 0.0, 0.0, 10.0),
        box_of(f64::NAN, 0.0, 10.0, 10.0),
        box_of(0.0, 0.0, f64::INFINITY, 10.0),
        box_of(0.0, 0.0, 1e308, 10.0),
    ];
    for model in refused {
        assert_eq!(frame.boundary(&model), Err(Invalid("a view's box is not inside out and is finite on paper")), "{model:?}");
    }
    assert!(frame.boundary(&box_of(5.0, 5.0, 5.0, 5.0)).is_ok_and(|b| b.min == b.max), "a point's box is a point");
    let line = Seg2::Line { a: Vec2::ZERO, b: Vec2::new(100.0, 20.0) };
    assert_eq!(frame.seg_to_paper(&line), Seg2::Line { a: Vec2::new(110.0, 128.5), b: Vec2::new(310.0, 168.5) });
    let bounded = CurveAccuracy::Bounded { max_deviation_mm: 0.01 };
    assert_eq!(frame.accuracy_on_paper(bounded), CurveAccuracy::Bounded { max_deviation_mm: 0.02 });
    assert_eq!(frame.accuracy_on_paper(CurveAccuracy::Analytic), CurveAccuracy::Analytic);
}

/// A scale is read and written as paper:model, kept as written, within 1:100000 to 1000:1.
#[test]
fn a_scale_reads_and_writes_as_paper_to_model() {
    for (text, factor) in [("1:1", 1.0), ("1:2", 0.5), ("2:1", 2.0), ("1:2.5", 0.4), ("1:100000", 1e-5), ("1000:1", 1000.0), ("3:7", 3.0 / 7.0)] {
        let s: Scale = text.parse().unwrap();
        assert_eq!((s.to_string(), s.factor()), (text.to_string(), factor), "{text}");
        assert_eq!(serde_json::to_value(s).unwrap(), serde_json::json!(text));
        assert_eq!(serde_json::from_value::<Scale>(serde_json::json!(text)).unwrap(), s);
    }
    let not_scales = [
        "", "1", "1:", ":2", "1:2:3", "0:1", "1:0", "-1:2", "1:-2", "1:100001", "1001:1", "1/2", " 1:2", "1:2 ", "1e3:1", "NaN:1", "inf:1", "1..5:1",
        "1:2.", "١:٢",
    ];
    for text in not_scales {
        assert!(text.parse::<Scale>().is_err(), "{text:?}");
    }
    assert!(serde_json::from_value::<Scale>(serde_json::json!(0.5)).is_err(), "a bare number is not a scale");
    for (paper, model) in [(0.0, 1.0), (1.0, 0.0), (f64::NAN, 1.0), (1.0, f64::INFINITY), (1.0, 1e300), (1e-300, 1.0)] {
        assert!(Scale::new(paper, model).is_err(), "{paper}:{model}");
    }
    // Each side is capped too, or 1e300:1e300 (1:1) would write 603 characters.
    for (paper, model) in [(1e300, 1e300), (1e-300, 1e-300), (1.000001e6, 1.000001e6), (0.9e-6, 0.9e-6)] {
        assert!(Scale::new(paper, model).is_err(), "{paper}:{model}");
    }
    assert!("1000001:1000001".parse::<Scale>().is_err() && "0.0000009:0.0000009".parse::<Scale>().is_err());
    // The longest a scale writes: its widest sides, about 50 characters.
    for (paper, model) in [(1e6, 1e6), (MIN_PART, MIN_PART), (1.2345678901234567e-6, 1.2345678901234567e-6), (999999.9999999999, 999999.9999999999)] {
        let text = Scale::new(paper, model).unwrap().to_string();
        assert!(text.len() <= 50, "{text}");
        assert_eq!(text.parse::<Scale>().unwrap(), Scale::new(paper, model).unwrap(), "{text}");
    }
}

/// The fitted scale is the largest ISO 5455 preset at which the view fits: W07's 100 x 20 front
/// view fits an A3 drawing area at 2:1 (5:1 would be 500 mm wide), exactly fitting counts, and a
/// view too large for every preset gets none.
#[test]
fn the_fitted_scale_is_the_largest_preset_that_fits() {
    let a3 = Vec2::new(400.0, 277.0);
    assert_eq!(Scale::fit(Vec2::new(100.0, 20.0), a3).map(|s| s.to_string()), Some("2:1".into()));
    assert_eq!(Scale::fit(Vec2::new(400.0, 10.0), a3).map(|s| s.to_string()), Some("1:1".into()), "an exact fit counts");
    assert_eq!(Scale::fit(Vec2::new(400.1, 10.0), a3).map(|s| s.to_string()), Some("1:2".into()));
    assert_eq!(Scale::fit(Vec2::new(1.0, 1.0), a3).map(|s| s.to_string()), Some("50:1".into()));
    assert_eq!(Scale::fit(Vec2::new(4e6, 1.0), a3).map(|s| s.to_string()), Some("1:10000".into()), "ISO 5455's smallest");
    assert_eq!(Scale::fit(Vec2::new(5e6, 1.0), a3), None);
    // An exact fit at a reduction counts though its factor rounds: 1675.75 * 0.2 is 335.15000000000003.
    let room = Vec2::new(335.15, 1000.0);
    assert_eq!(Scale::fit(Vec2::new(1675.75, 1.0), room).map(|s| s.to_string()), Some("1:5".into()));
    // The presets run largest first, each a 1, 2 or 5 step.
    assert!(PRESETS.windows(2).all(|w| w[0].factor() > w[1].factor()));
    // A size of 0 sets no limit that way: a line fits by its length, a point at the largest preset.
    assert_eq!(Scale::fit(Vec2::new(0.0, 20.0), a3).map(|s| s.to_string()), Some("10:1".into()));
    assert_eq!(Scale::fit(Vec2::ZERO, a3).map(|s| s.to_string()), Some("50:1".into()));
    // A size or room that is negative or not finite fits nothing.
    for (size, room) in [
        (Vec2::new(-1.0, 1.0), a3),
        (Vec2::new(1.0, -0.0001), a3),
        (Vec2::new(f64::NAN, 1.0), a3),
        (Vec2::new(1.0, f64::INFINITY), a3),
        (Vec2::new(1.0, 1.0), Vec2::new(-400.0, 277.0)),
        (Vec2::new(1.0, 1.0), Vec2::new(f64::INFINITY, 277.0)),
        (Vec2::new(1.0, 1.0), Vec2::new(400.0, f64::NAN)),
    ] {
        assert_eq!(Scale::fit(size, room), None, "{size:?} in {room:?}");
    }
}

/// What a frame refuses: a basis that isn't orthonormal or finite, a centre that isn't finite, and
/// a position past the paper's limit.
#[test]
fn a_frame_refuses_hostile_input() {
    let one = Scale::new(1.0, 1.0).unwrap();
    let front = NamedView::Front.basis();
    let bases = [
        Plane { x: Vec3::new(2.0, 0.0, 0.0), ..front },
        Plane { x: Vec3::Z, ..front },
        Plane { origin: Vec3::new(f64::NAN, 0.0, 0.0), ..front },
        Plane { y: Vec3::new(0.0, 0.0, f64::INFINITY), ..front },
    ];
    for basis in bases {
        assert!(ViewFrame::new(basis, one, Vec2::ZERO, Vec2::ZERO).is_err(), "{basis:?}");
    }
    for centre in [Vec2::new(f64::NAN, 0.0), Vec2::new(0.0, f64::INFINITY), Vec2::new(1e13, 0.0)] {
        assert!(ViewFrame::new(front, one, centre, Vec2::ZERO).is_err(), "{centre:?}");
    }
    for position in [Vec2::new(5000.001, 0.0), Vec2::new(0.0, -5000.001), Vec2::new(f64::NAN, 0.0)] {
        assert!(ViewFrame::new(front, one, Vec2::ZERO, position).is_err(), "{position:?}");
    }
    assert!(ViewFrame::new(front, one, Vec2::ZERO, Vec2::new(5000.0, -5000.0)).is_ok());
}


/// A 100 x 50 x 20 mm solid fits orthographic front/top/right views in an
/// A3 border with a common 2:1 scale; diagram positions follow first/third angle.
#[test]
fn a3_three_view_layout_preserves_model_dimensions_and_sheet_scale() {
    use super::technical::{ProjectionConvention, three_view_layout};
    use crate::drawing_sheet::{DrawingBorderProfile, Orientation, PaperSize, Sheet};
    use crate::mesh::Aabb3;
    let bounds = Aabb3 { min: Vec3::ZERO, max: Vec3::new(100.0, 50.0, 20.0) };
    let sheet = Sheet::standard(PaperSize::A3, Orientation::Landscape);
    let border = DrawingBorderProfile::iso().layout(&sheet).unwrap().drawing_border;
    for convention in [ProjectionConvention::FirstAngle, ProjectionConvention::ThirdAngle] {
        let result = three_view_layout(bounds, &sheet, Some(&DrawingBorderProfile::iso()), convention, 12.0).unwrap();
        assert_eq!(result.front.scale(), result.top.scale());
        assert_eq!(result.front.scale(), result.right.scale());
        assert_eq!(result.front.scale().to_string(), "2:1");
        assert!((extents(&result.front, &[bounds.min, bounds.max]) - Vec2::new(200.0, 40.0)).len() < 1e-10);
        assert!((extents(&result.top, &[bounds.min, bounds.max]) - Vec2::new(200.0, 100.0)).len() < 1e-10);
        assert!((extents(&result.right, &[bounds.min, bounds.max]) - Vec2::new(100.0, 40.0)).len() < 1e-10);
        let paper_bounds = |f: &ViewFrame| -> Aabb2 {
            (0..8).map(|i| f.to_paper(Vec3::new(
                if i & 1 == 0 { 0.0 } else { 100.0 },
                if i & 2 == 0 { 0.0 } else { 50.0 },
                if i & 4 == 0 { 0.0 } else { 20.0 },
            ))).collect()
        };
        let (front, top, right) = (paper_bounds(&result.front), paper_bounds(&result.top), paper_bounds(&result.right));
        assert!(border.contains(front.min) && border.contains(front.max));
        assert!(border.contains(top.min) && border.contains(top.max));
        assert!(border.contains(right.min) && border.contains(right.max));
        assert!((front.min.x - top.min.x).abs() < 1e-10, "top aligned with front");
        assert!((front.min.y - right.min.y).abs() < 1e-10, "right aligned with front");
        match convention {
            ProjectionConvention::FirstAngle => {
                assert!(top.max.y + 11.99 < front.min.y, "first-angle top below front");
                assert!(right.max.x + 11.99 < front.min.x, "first-angle right on left");
            }
            ProjectionConvention::ThirdAngle => {
                assert!(front.max.y + 11.99 < top.min.y, "third-angle top above front");
                assert!(front.max.x + 11.99 < right.min.x, "third-angle right on right");
            }
        }
        let again = three_view_layout(bounds, &sheet, Some(&DrawingBorderProfile::iso()), convention, 12.0).unwrap();
        assert_eq!(again, result, "layout is repeatable without camera state");
    }
    assert!(three_view_layout(Aabb3::EMPTY, &sheet, None, ProjectionConvention::FirstAngle, 10.0).is_err());
    assert!(three_view_layout(bounds, &sheet, None, ProjectionConvention::FirstAngle, f64::NAN).is_err());
}

/// The focal plane is a perspective illustration, not a scaled technical view.
#[test]
fn perspective_divides_by_depth_and_rejects_behind_eye() {
    use super::technical::PerspectiveFrame;
    let eye = Vec3::new(0.0, -200.0, 0.0);
    let perspective = PerspectiveFrame::new(eye, Vec3::ZERO, Vec3::Z, 100.0, Vec2::new(100.0, 100.0)).unwrap();
    let near = perspective.to_paper(Vec3::new(100.0, 0.0, 0.0)).unwrap();
    let far = perspective.to_paper(Vec3::new(100.0, 200.0, 0.0)).unwrap();
    assert!((near - Vec2::new(150.0, 100.0)).len() < 1e-10);
    assert!((far - Vec2::new(125.0, 100.0)).len() < 1e-10);
    assert!(perspective.to_paper(Vec3::new(1.0, -200.0, 0.0)).is_none());
    assert!(perspective.to_paper(Vec3::new(1.0, -300.0, 0.0)).is_none());
    assert!(PerspectiveFrame::new(eye, Vec3::ZERO, Vec3::Z, 0.0, Vec2::ZERO).is_err());
    assert!(PerspectiveFrame::new(eye, eye, Vec3::Z, 100.0, Vec2::ZERO).is_err());
    let orthographic = full_size(NamedView::Front);
    assert!((orthographic.to_paper(Vec3::new(100.0, 0.0, 0.0))
        - orthographic.to_paper(Vec3::new(100.0, 200.0, 0.0))).len() < EPS);
}

/// A detail view is a true scaled copy with a new centre, not a camera zoom.
#[test]
fn detail_magnification_repositions_without_modifying_model_measurements() {
    use super::technical::detail_frame;
    let original = ViewFrame::new(NamedView::Front.basis(), "1:5".parse().unwrap(), Vec2::ZERO, Vec2::new(40.0, 40.0)).unwrap();
    let detail = detail_frame(&original, Vec2::new(20.0, 10.0), Vec2::new(120.0, 80.0), 4.0).unwrap();
    assert!((detail.scale().factor() - 0.8).abs() < EPS);
    assert!((detail.to_paper(Vec3::new(20.0, 0.0, 10.0)) - Vec2::new(120.0, 80.0)).len() < EPS);
    assert!((detail.to_paper(Vec3::new(30.0, 0.0, 10.0)) - Vec2::new(128.0, 80.0)).len() < EPS);
    assert!(detail_frame(&original, Vec2::ZERO, Vec2::ZERO, 0.0).is_err());
    assert!(detail_frame(&original, Vec2::ZERO, Vec2::ZERO, f64::INFINITY).is_err());
}

/// A section producer's geometry is mapped into sheet coordinates without
/// losing its open-wire classification or declared curve accuracy.
#[test]
fn a_provided_section_maps_to_paper_and_rejects_wrong_orientation() {
    use super::technical::project_section;
    use crate::section::{SectionGeometry, SectionCurve, SectionWire, CurveUse};
    let frame = ViewFrame::new(NamedView::Front.basis(), "1:2".parse().unwrap(), Vec2::ZERO, Vec2::new(100.0, 100.0)).unwrap();
    let mut section = SectionGeometry {
        plane: Plane { origin: Vec3::new(10.0, 0.0, 30.0), ..NamedView::Front.basis() },
        curves: vec![SectionCurve {
            geometry: Seg2::Line { a: Vec2::ZERO, b: Vec2::new(20.0, 0.0) },
            accuracy: CurveAccuracy::Bounded { max_deviation_mm: 0.02 },
        }],
        wires: vec![SectionWire { curves: vec![CurveUse { curve: 0, forward: true }], closed: false }],
        regions: Vec::new(),
        contacts: vec![Vec2::new(5.0, 0.0)],
    };
    let result = project_section(&section, &frame, 1e-6).unwrap();
    assert_eq!(result.curves[0].geometry, Seg2::Line { a: Vec2::new(105.0, 115.0), b: Vec2::new(115.0, 115.0) });
    assert_eq!(result.curves[0].accuracy, CurveAccuracy::Bounded { max_deviation_mm: 0.01 });
    assert_eq!(result.contacts, vec![Vec2::new(107.5, 115.0)]);
    assert_eq!(result.wires, section.wires);
    section.plane.y = -section.plane.y;
    assert!(project_section(&section, &frame, 1e-6).is_err(), "do not mirror a section silently");
}

/// Drawing dimensions measure model space; the paper 1:2 ratio must not turn a
/// 100 mm dimension into a 50 mm label. Units affect labels, never strokes.
#[test]
fn dimension_graphics_keep_physical_values_distinct_from_paper_lengths() {
    use super::dimensions::{LengthUnit, DimensionKind, DimensionStyle, angular_dimension, circular_dimension, linear_dimension};
    let frame = ViewFrame::new(NamedView::Front.basis(), "1:2".parse().unwrap(), Vec2::ZERO, Vec2::new(100.0, 100.0)).unwrap();
    let mm = DimensionStyle::default();
    let linear = linear_dimension(&frame, Vec3::ZERO, Vec3::new(100.0, 25.0, 0.0), 12.0, mm).unwrap();
    assert_eq!(linear.kind, DimensionKind::Linear);
    assert_eq!(linear.measured_value, 100.0, "the view ignores depth");
    assert_eq!(linear.label, "100.00 mm");
    assert_eq!(linear.strokes.len(), 3);
    assert_eq!(linear.arrowheads.len(), 2);
    let inches = DimensionStyle { unit: LengthUnit::Inch, ..mm };
    let inch_dim = linear_dimension(&frame, Vec3::ZERO, Vec3::new(100.0, 0.0, 0.0), 12.0, inches).unwrap();
    assert_eq!(inch_dim.label, "3.94 in");
    assert_eq!(linear.strokes, inch_dim.strokes);
    let radius = circular_dimension(&frame, Vec3::ZERO, Vec3::new(0.0, 0.0, 10.0), -Vec3::Y, false, mm).unwrap();
    let diameter = circular_dimension(&frame, Vec3::ZERO, Vec3::new(0.0, 0.0, 10.0), -Vec3::Y, true, mm).unwrap();
    assert_eq!(radius.label, "R10.00 mm");
    assert_eq!(diameter.label, "⌀20.00 mm");
    assert_eq!(diameter.measured_value, 20.0);
    assert!(circular_dimension(&frame, Vec3::ZERO, Vec3::new(0.0, 0.0, 10.0), Vec3::Z, true, mm).is_err());
    let angle = angular_dimension(&frame, Vec3::ZERO, Vec3::X, Vec3::Z, 15.0, mm).unwrap();
    assert_eq!(angle.kind, DimensionKind::Angle);
    assert!((angle.measured_value - 90.0).abs() < EPS);
    assert_eq!(angle.label, "90.00°");
    assert!(matches!(angle.strokes[2], Seg2::Arc { .. }));
    assert!(linear_dimension(&frame, Vec3::ZERO, Vec3::ZERO, 10.0, mm).is_err());
    assert!(angular_dimension(&frame, Vec3::ZERO, Vec3::X, Vec3::X, 15.0, mm).is_err());
    assert!(linear_dimension(&frame, Vec3::ZERO, Vec3::X, 10.0,
        DimensionStyle { precision: 50, ..mm }).is_err());
}
