//! Clipper2 backend, compiled only with the `clipper2` feature. The unsafe code in this module is
//! the FFI to cpp/shim.cpp and is the only unsafe code in the crate.
#![allow(unsafe_code)]

use std::ffi::c_int;
use std::ptr::NonNull;

use crate::{BooleanOp, Error, Point, Ring, check_delta, check_ring};

/// Fixed-point scale: input coordinates are rounded to 1e-6 before Clipper2 sees them.
const SCALE: f64 = 1.0e6;
/// Miter spike limit passed to Clipper2, as a multiple of the offset distance. The pure backend
/// has no limit; 1000 keeps the int64 arithmetic inside range for inputs up to MAX_COORD.
const MITER_LIMIT: f64 = 1000.0;

/// Opaque result owned by the C++ side.
#[repr(C)]
pub struct PcResult {
    _private: [u8; 0],
}

unsafe extern "C" {
    fn pc2_offset(xy: *const i64, n: usize, delta: f64, miter_limit: f64, status: *mut c_int) -> *mut PcResult;
    fn pc2_boolean(a: *const i64, na: usize, b: *const i64, nb: usize, op: c_int, status: *mut c_int) -> *mut PcResult;
    fn pc2_ring_count(r: *const PcResult) -> usize;
    fn pc2_ring_len(r: *const PcResult, i: usize) -> usize;
    fn pc2_coords(r: *const PcResult) -> *const i64;
    fn pc2_free(r: *mut PcResult);
}

/// Offsets a polygon with Clipper2 (miter joins). Handles any simple polygon.
pub fn offset_polygon(points: &[Point], delta: f64) -> Result<Vec<Ring>, Error> {
    check_ring(points)?;
    check_delta(delta)?;
    let xy = to_fixed(points);
    let mut status: c_int = 0;
    // SAFETY: `xy` holds 2 * points.len() valid i64 values for the duration of the call, and
    // `status` is a valid out-pointer. The shim returns null or an owned result.
    let raw = unsafe { pc2_offset(xy.as_ptr(), points.len(), delta * SCALE, MITER_LIMIT, &mut status) };
    Handle::new(raw, status)?.rings()
}

/// Boolean operation with Clipper2, for any two simple polygons.
pub fn boolean(a: &[Point], b: &[Point], op: BooleanOp) -> Result<Vec<Ring>, Error> {
    check_ring(a)?;
    check_ring(b)?;
    let code = match op {
        BooleanOp::Union => 0,
        BooleanOp::Intersection => 1,
        BooleanOp::Difference => 2,
        BooleanOp::Xor => 3,
    };
    let (fa, fb) = (to_fixed(a), to_fixed(b));
    let mut status: c_int = 0;
    // SAFETY: as in `offset_polygon`, each slice holds 2 * len valid i64 values during the call.
    let raw = unsafe { pc2_boolean(fa.as_ptr(), a.len(), fb.as_ptr(), b.len(), code, &mut status) };
    Handle::new(raw, status)?.rings()
}

/// Rounds points to the fixed-point grid, interleaved as x, y. Inputs are already range-checked
/// to MAX_COORD, so the values fit comfortably in i64.
fn to_fixed(points: &[Point]) -> Vec<i64> {
    points.iter().flat_map(|p| [(p[0] * SCALE).round() as i64, (p[1] * SCALE).round() as i64]).collect()
}

/// Owns a result from the shim and frees it on drop.
struct Handle(NonNull<PcResult>);

impl Handle {
    fn new(raw: *mut PcResult, status: c_int) -> Result<Self, Error> {
        match (NonNull::new(raw), status) {
            (Some(p), 0) => Ok(Handle(p)),
            (_, 2) => Err(Error::Backend("out of memory")),
            (_, 3) => Err(Error::Backend("unknown boolean operation")),
            (_, 1) => Err(Error::Backend("Clipper2 raised an exception")),
            _ => Err(Error::Backend("no result")),
        }
    }

    fn rings(&self) -> Result<Vec<Ring>, Error> {
        let p = self.0.as_ptr().cast_const();
        // SAFETY: `p` is a live result owned by this handle; the accessors only read it.
        let (count, coords) = unsafe { (pc2_ring_count(p), pc2_coords(p)) };
        let mut rings = Vec::with_capacity(count);
        let mut offset = 0usize;
        for i in 0..count {
            // SAFETY: `i` is below the ring count reported for the same result.
            let len = unsafe { pc2_ring_len(p, i) };
            if len == 0 {
                rings.push(Vec::new());
                continue;
            }
            if coords.is_null() {
                return Err(Error::Backend("result has points but no coordinates"));
            }
            // SAFETY: the shim stores 2 * len coordinates per ring, back to back, so this range
            // lies inside the buffer that `coords` points to.
            let flat = unsafe { std::slice::from_raw_parts(coords.add(2 * offset), 2 * len) };
            offset += len;
            rings.push(
                flat.chunks_exact(2)
                    .map(|c| match *c {
                        [x, y] => [x as f64 / SCALE, y as f64 / SCALE],
                        _ => [0.0, 0.0],
                    })
                    .collect(),
            );
        }
        Ok(rings)
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: the handle is the only owner of this result, and it is freed exactly once.
        unsafe { pc2_free(self.0.as_ptr()) }
    }
}
