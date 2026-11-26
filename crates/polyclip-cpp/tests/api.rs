use solidcraft_polyclip_cpp::{BooleanOp, Error, MAX_COORD, MAX_POINTS, Point, Ring, boolean, offset_polygon, pure};

fn square(x0: f64, y0: f64, side: f64) -> Vec<Point> {
    vec![[x0, y0], [x0 + side, y0], [x0 + side, y0 + side], [x0, y0 + side]]
}

fn area(ring: &Ring) -> f64 {
    let n = ring.len();
    let twice: f64 = (0..n)
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum();
    (twice / 2.0).abs()
}

fn assert_close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-5, "{a} != {b}");
}

fn assert_has_point(ring: &Ring, p: Point) {
    let hit = ring.iter().any(|q| (q[0] - p[0]).abs() < 1e-5 && (q[1] - p[1]).abs() < 1e-5);
    assert!(hit, "{p:?} not in {ring:?}");
}

#[test]
fn outward_offset_of_square_moves_each_corner_diagonally() {
    let rings = offset_polygon(&square(0.0, 0.0, 2.0), 1.0).expect("offset");
    assert_eq!(rings.len(), 1);
    let ring = &rings[0];
    assert_eq!(ring.len(), 4);
    for corner in [[-1.0, -1.0], [3.0, -1.0], [3.0, 3.0], [-1.0, 3.0]] {
        assert_has_point(ring, corner);
    }
    assert_close(area(ring), 16.0);
}

#[test]
fn clockwise_input_gives_the_same_offset() {
    let mut cw = square(0.0, 0.0, 2.0);
    cw.reverse();
    let ring = &offset_polygon(&cw, 1.0).expect("offset")[0];
    assert_close(area(ring), 16.0);
}

#[test]
fn inward_offset_shrinks_the_square() {
    let ring = &offset_polygon(&square(0.0, 0.0, 2.0), -0.5).expect("offset")[0];
    assert_close(area(ring), 1.0);
    assert_has_point(ring, [0.5, 0.5]);
}

#[test]
fn inward_offset_past_the_inradius_collapses_to_nothing() {
    let rings = offset_polygon(&square(0.0, 0.0, 2.0), -1.5).expect("offset");
    assert!(rings.is_empty());
}

#[test]
fn zero_offset_returns_the_cleaned_ring() {
    let mut pts = square(0.0, 0.0, 2.0);
    pts.push([0.0, 0.0]);
    let rings = offset_polygon(&pts, 0.0).expect("offset");
    // Clipper2 may start at another vertex or reverse the winding, so compare shape, not order.
    assert_eq!(rings.len(), 1);
    assert_close(area(&rings[0]), 4.0);
    for corner in [[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0]] {
        assert_has_point(&rings[0], corner);
    }
}

#[test]
fn triangle_offset_keeps_three_vertices() {
    let tri = vec![[0.0, 0.0], [4.0, 0.0], [0.0, 3.0]];
    let ring = &offset_polygon(&tri, 0.5).expect("offset")[0];
    assert_eq!(ring.len(), 3);
}

#[test]
fn intersection_of_overlapping_squares() {
    let rings = boolean(&square(0.0, 0.0, 2.0), &square(1.0, 1.0, 2.0), BooleanOp::Intersection).expect("boolean");
    assert_eq!(rings.len(), 1);
    assert_close(area(&rings[0]), 1.0);
}

#[test]
fn disjoint_intersection_is_empty() {
    let rings = boolean(&square(0.0, 0.0, 1.0), &square(5.0, 5.0, 1.0), BooleanOp::Intersection).expect("boolean");
    assert!(rings.is_empty());
}

#[test]
fn pure_backend_reports_unsupported_for_non_convex_inputs() {
    let l_shape = vec![[0.0, 0.0], [2.0, 0.0], [2.0, 1.0], [1.0, 1.0], [1.0, 2.0], [0.0, 2.0]];
    let err = pure::offset_polygon(&l_shape, 0.1).expect_err("non-convex");
    assert!(matches!(err, Error::Unsupported { .. }), "{err:?}");
}

#[test]
fn pure_backend_reports_unsupported_for_union_and_difference() {
    let a = square(0.0, 0.0, 2.0);
    let b = square(1.0, 1.0, 2.0);
    for op in [BooleanOp::Union, BooleanOp::Difference, BooleanOp::Xor] {
        let err = pure::boolean(&a, &b, op).expect_err("unsupported");
        assert!(matches!(err, Error::Unsupported { .. }), "{op:?}: {err:?}");
    }
}

#[test]
fn self_intersecting_pentagram_is_not_treated_as_convex() {
    let star: Vec<Point> = (0..5)
        .map(|k| {
            let a = std::f64::consts::TAU * (2 * k) as f64 / 5.0;
            [a.cos(), a.sin()]
        })
        .collect();
    let err = pure::offset_polygon(&star, 0.1).expect_err("self-intersecting");
    assert!(matches!(err, Error::Unsupported { .. }), "{err:?}");
}

#[test]
fn invalid_inputs_are_errors_not_panics() {
    let sq = square(0.0, 0.0, 1.0);
    assert!(matches!(offset_polygon(&[], 1.0), Err(Error::InvalidInput(_))));
    assert!(matches!(offset_polygon(&sq[..2], 1.0), Err(Error::InvalidInput(_))));
    assert!(matches!(offset_polygon(&[[0.0, 0.0], [1.0, 1.0], [2.0, 2.0]], 1.0), Err(Error::InvalidInput(_))));
    assert!(matches!(offset_polygon(&sq, f64::NAN), Err(Error::InvalidInput(_))));
    assert!(matches!(offset_polygon(&sq, f64::INFINITY), Err(Error::InvalidInput(_))));
    assert!(matches!(offset_polygon(&sq, 1e300), Err(Error::TooLarge(_))));
    let nan_pt = vec![[f64::NAN, 0.0], [1.0, 0.0], [0.0, 1.0]];
    assert!(matches!(offset_polygon(&nan_pt, 1.0), Err(Error::InvalidInput(_))));
    let huge = vec![[1e300, 0.0], [1.0, 0.0], [0.0, 1.0]];
    assert!(matches!(offset_polygon(&huge, 1.0), Err(Error::TooLarge(_))));
    assert!(matches!(boolean(&sq, &[], BooleanOp::Intersection), Err(Error::InvalidInput(_))));
}

#[test]
fn oversized_input_is_rejected_before_any_work() {
    let many = vec![[0.0, 0.0]; MAX_POINTS + 1];
    assert!(matches!(offset_polygon(&many, 1.0), Err(Error::TooLarge(_))));
    let edge = vec![[MAX_COORD, 0.0], [0.0, MAX_COORD], [-MAX_COORD, 0.0]];
    assert!(offset_polygon(&edge, MAX_COORD * 2.0).is_err());
}

#[test]
fn hostile_values_never_panic() {
    // Deterministic pseudo-random mix of NaN, infinities, subnormals, huge and tiny values.
    let specials = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -0.0, 1e-310, 1e300, -1e9, 1e9, 3.0];
    let mut seed: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut next = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (seed >> 33) as usize
    };
    for _ in 0..2000 {
        let n = next() % 6;
        let pts: Vec<Point> = (0..n)
            .map(|_| {
                let x = if next() % 2 == 0 { specials[next() % specials.len()] } else { (next() % 100) as f64 };
                let y = if next() % 2 == 0 { specials[next() % specials.len()] } else { (next() % 100) as f64 };
                [x, y]
            })
            .collect();
        let delta = specials[next() % specials.len()];
        let _ = offset_polygon(&pts, delta);
        let _ = pure::offset_polygon(&pts, delta);
        let _ = boolean(&pts, &square(0.0, 0.0, 10.0), BooleanOp::Intersection);
    }
}

#[cfg(feature = "clipper2")]
mod with_clipper2 {
    use super::*;

    #[test]
    fn backend_is_clipper2() {
        assert_eq!(solidcraft_polyclip_cpp::backend(), "clipper2");
    }

    #[test]
    fn clipper2_matches_pure_backend_for_convex_offset() {
        let tri = vec![[0.0, 0.0], [4.0, 0.0], [1.0, 3.0]];
        for delta in [0.5, -0.3] {
            let a = area(&pure::offset_polygon(&tri, delta).expect("pure")[0]);
            let b = area(&offset_polygon(&tri, delta).expect("clipper2")[0]);
            assert!((a - b).abs() < 1e-3, "delta {delta}: {a} vs {b}");
        }
    }

    #[test]
    fn clipper2_handles_non_convex_offset_and_all_boolean_ops() {
        let l_shape = vec![[0.0, 0.0], [2.0, 0.0], [2.0, 1.0], [1.0, 1.0], [1.0, 2.0], [0.0, 2.0]];
        assert_eq!(offset_polygon(&l_shape, 0.1).expect("offset").len(), 1);
        let a = square(0.0, 0.0, 2.0);
        let b = square(1.0, 1.0, 2.0);
        assert_close(area(&boolean(&a, &b, BooleanOp::Union).expect("union")[0]), 7.0);
        assert_close(area(&boolean(&a, &b, BooleanOp::Difference).expect("difference")[0]), 3.0);
        assert_close(area(&boolean(&a, &b, BooleanOp::Intersection).expect("intersection")[0]), 1.0);
    }

    #[test]
    fn clipper2_empty_intersection() {
        let rings = boolean(&square(0.0, 0.0, 1.0), &square(5.0, 5.0, 1.0), BooleanOp::Intersection).expect("boolean");
        assert!(rings.is_empty());
    }
}


#[cfg(feature = "clipper2")]
#[test]
fn repeated_clipper_calls_free_native_results() {
    let a = square(0.0, 0.0, 2.0);
    let b = square(1.0, 1.0, 2.0);
    for _ in 0..100 {
        let out = boolean(&a, &b, BooleanOp::Intersection).expect("native boolean");
        assert_close(area(&out[0]), 1.0);
    }
}

/// Run explicitly with -- --ignored --nocapture for a per-run comparison.
/// This compares equivalent convex intersection algorithms, not B-rep performance.
#[cfg(feature = "clipper2")]
#[test]
#[ignore = "manual measurement; output includes hardware and iterations"]
fn time_native_and_rust_convex_intersection() {
    use std::hint::black_box;
    use std::time::Instant;
    let a = square(0.0, 0.0, 2.0);
    let b = square(1.0, 1.0, 2.0);
    let loops = 2_000;
    let pure_start = Instant::now();
    for _ in 0..loops {
        black_box(pure::boolean(black_box(&a), black_box(&b), BooleanOp::Intersection).expect("pure"));
    }
    let pure_time = pure_start.elapsed();
    let native_start = Instant::now();
    for _ in 0..loops {
        black_box(boolean(black_box(&a), black_box(&b), BooleanOp::Intersection).expect("native"));
    }
    let native_time = native_start.elapsed();
    println!("convex square intersection: runs={loops}, pure_rust_ns_total={}, clipper2_ns_total={}",
             pure_time.as_nanos(), native_time.as_nanos());
}
