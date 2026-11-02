//! Optional polygon offset and boolean spike.
//!
//! The default build is pure Rust and handles convex polygons only: offsetting, and intersection
//! of two convex polygons. Other inputs and operations return [`Error::Unsupported`].
//!
//! With the `clipper2` feature the same functions call Clipper2 (BSL-1.0) through a small C++
//! shim, and every polygon and operation is supported. That path is off by default and never in
//! wasm builds. See docs/cpp-optional.md.
//!
//! Units are whatever the caller uses (millimetres in CAD). Inputs are limited to
//! [`MAX_POINTS`] points per ring and coordinates or offsets of magnitude [`MAX_COORD`].

#![deny(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

#[cfg(feature = "clipper2")]
mod cpp;
pub mod pure;

#[cfg(feature = "clipper2")]
pub use cpp::{boolean, offset_polygon};
#[cfg(not(feature = "clipper2"))]
pub use pure::{boolean, offset_polygon};

/// A point in the plane.
pub type Point = [f64; 2];
/// A closed ring of points; the closing edge is implied.
pub type Ring = Vec<Point>;

/// Most points accepted in one ring.
pub const MAX_POINTS: usize = 1_000_000;
/// Largest accepted absolute coordinate or offset distance.
pub const MAX_COORD: f64 = 1.0e9;

/// Boolean operation between two polygons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanOp {
    Union,
    Intersection,
    Difference,
    Xor,
}

/// Errors from this crate. None of them is a panic.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The input is too small, degenerate, or contains NaN or infinity.
    #[error("invalid input: {0}")]
    InvalidInput(&'static str),
    /// The input exceeds [`MAX_POINTS`] or [`MAX_COORD`].
    #[error("input too large: {0}")]
    TooLarge(&'static str),
    /// The active backend does not implement this case.
    #[error("not supported by the {backend} backend: {what}")]
    Unsupported { backend: &'static str, what: &'static str },
    /// The C++ backend failed.
    #[error("clipper2 backend failed: {0}")]
    Backend(&'static str),
}

/// Name of the backend that [`offset_polygon`] and [`boolean`] use in this build.
pub fn backend() -> &'static str {
    if cfg!(feature = "clipper2") { "clipper2" } else { "pure-rust" }
}

/// Checks one ring: at least three points, at most [`MAX_POINTS`], finite, within [`MAX_COORD`].
pub(crate) fn check_ring(points: &[Point]) -> Result<(), Error> {
    if points.len() > MAX_POINTS {
        return Err(Error::TooLarge("more points than MAX_POINTS"));
    }
    if points.len() < 3 {
        return Err(Error::InvalidInput("fewer than three points"));
    }
    for p in points {
        if !(p[0].is_finite() && p[1].is_finite()) {
            return Err(Error::InvalidInput("non-finite coordinate"));
        }
        if !(p[0].abs() <= MAX_COORD && p[1].abs() <= MAX_COORD) {
            return Err(Error::TooLarge("coordinate magnitude above MAX_COORD"));
        }
    }
    let n = points.len();
    let twice_area: f64 = (0..n)
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % n]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum();
    if twice_area == 0.0 {
        return Err(Error::InvalidInput("zero-area ring"));
    }
    Ok(())
}

/// Checks an offset distance: finite and within [`MAX_COORD`].
pub(crate) fn check_delta(delta: f64) -> Result<(), Error> {
    if !delta.is_finite() {
        return Err(Error::InvalidInput("non-finite offset distance"));
    }
    if !(delta.abs() <= MAX_COORD) {
        return Err(Error::TooLarge("offset distance above MAX_COORD"));
    }
    Ok(())
}
