use solidcraft_manifold_cpp::{BooleanOp, Error, Mesh, backend, boolean};

fn cube(x: f64, y: f64, z: f64, s: f64) -> Mesh {
    Mesh {
        vertices: vec![
            [x, y, z], [x+s, y, z], [x+s, y+s, z], [x, y+s, z],
            [x, y, z+s], [x+s, y, z+s], [x+s, y+s, z+s], [x, y+s, z+s],
        ],
        triangles: vec![
            [0, 2, 1], [0, 3, 2], [4, 5, 6], [4, 6, 7],
            [0, 1, 5], [0, 5, 4], [3, 7, 6], [3, 6, 2],
            [0, 4, 7], [0, 7, 3], [1, 2, 6], [1, 6, 5],
        ],
    }
}

// Independent Rust signed-tetrahedra integration, not a second CSG backend.
#[cfg(feature = "manifold")]
fn volume(mesh: &Mesh) -> f64 {
    mesh.triangles.iter().map(|&[a, b, c]| {
        let p = mesh.vertices[a as usize];
        let q = mesh.vertices[b as usize];
        let r = mesh.vertices[c as usize];
        (p[0] * (q[1] * r[2] - q[2] * r[1]) +
         p[1] * (q[2] * r[0] - q[0] * r[2]) +
         p[2] * (q[0] * r[1] - q[1] * r[0])) / 6.0
    }).sum::<f64>().abs()
}

#[test]
fn invalid_inputs_are_errors_not_panics() {
    let good = cube(0.0, 0.0, 0.0, 1.0);
    assert!(good.validate().is_ok());
    let mut bad = good.clone();
    bad.vertices[0][0] = f64::NAN;
    assert!(matches!(boolean(&bad, &good, BooleanOp::Union), Err(Error::InvalidInput(_))));
    bad.vertices[0][0] = f64::INFINITY;
    assert!(matches!(boolean(&bad, &good, BooleanOp::Union), Err(Error::InvalidInput(_))));
    bad.vertices[0][0] = 1e200;
    assert!(matches!(boolean(&bad, &good, BooleanOp::Union), Err(Error::TooLarge(_))));
    bad.vertices[0][0] = 0.0;
    bad.triangles[0] = [0, 0, 1];
    assert!(matches!(boolean(&bad, &good, BooleanOp::Union), Err(Error::InvalidInput(_))));
    bad.triangles[0] = [0, 1, u32::MAX];
    assert!(matches!(boolean(&bad, &good, BooleanOp::Union), Err(Error::InvalidInput(_))));
    bad.triangles.clear();
    assert!(matches!(boolean(&bad, &good, BooleanOp::Union), Err(Error::InvalidInput(_))));
}

#[cfg(not(feature = "manifold"))]
#[test]
fn missing_backend_is_explicit_not_fake_success() {
    assert_eq!(backend(), "unavailable");
    let a = cube(0.0, 0.0, 0.0, 1.0);
    assert_eq!(boolean(&a, &a, BooleanOp::Union), Err(Error::Unavailable));
}

#[cfg(feature = "manifold")]
#[test]
fn union_difference_intersection_have_analytic_cube_volumes() {
    assert_eq!(backend(), "manifold-cpp");
    let a = cube(0.0, 0.0, 0.0, 1.0);
    let b = cube(0.5, 0.5, 0.5, 1.0);
    for (op, expected) in [
        (BooleanOp::Union, 1.875),
        (BooleanOp::Difference, 0.875),
        (BooleanOp::Intersection, 0.125),
    ] {
        let result = boolean(&a, &b, op).expect("Manifold C++ Boolean");
        assert!(!result.triangles.is_empty(), "{op:?}");
        let actual = volume(&result);
        assert!((actual - expected).abs() < 1e-7,
                "{op:?}: volume {actual}, expected {expected}");
    }
}

#[cfg(feature = "manifold")]
#[test]
fn disjoint_intersection_is_empty() {
    let a = cube(0.0, 0.0, 0.0, 1.0);
    let b = cube(10.0, 10.0, 10.0, 1.0);
    let out = boolean(&a, &b, BooleanOp::Intersection).expect("empty intersection");
    assert!(out.vertices.is_empty() && out.triangles.is_empty());
}

#[cfg(feature = "manifold")]
#[test]
fn repeated_calls_release_owned_outputs() {
    let a = cube(0.0, 0.0, 0.0, 1.0);
    let b = cube(0.5, 0.5, 0.5, 1.0);
    for _ in 0..100 {
        let output = boolean(&a, &b, BooleanOp::Intersection).expect("repeated intersection");
        assert!((volume(&output) - 0.125).abs() < 1e-7);
    }
}

#[cfg(feature = "manifold")]
#[test]
fn open_non_manifold_soup_errors() {
    let mut invalid = cube(0.0, 0.0, 0.0, 1.0);
    invalid.triangles.pop();
    let valid = cube(0.5, 0.5, 0.5, 1.0);
    assert!(matches!(boolean(&invalid, &valid, BooleanOp::Union), Err(Error::Backend(_))));
}
