//! The single unsafe module for the native C ABI. Ownership stays in a RAII Handle.
#![allow(unsafe_code)]

use std::ffi::c_int;
use std::ptr::NonNull;

use crate::{BooleanOp, Error, MAX_RESULT_TRIANGLES, MAX_RESULT_VERTICES, Mesh};

#[repr(C)]
struct NativeResult {
    _opaque: [u8; 0],
}

unsafe extern "C" {
    fn ac_manifold_boolean(
        a_xyz: *const f64, a_vertices: usize, a_tri: *const u32, a_triangles: usize,
        b_xyz: *const f64, b_vertices: usize, b_tri: *const u32, b_triangles: usize,
        operation: c_int, out: *mut *mut NativeResult,
    ) -> c_int;
    fn ac_manifold_vertex_count(result: *const NativeResult) -> usize;
    fn ac_manifold_triangle_count(result: *const NativeResult) -> usize;
    fn ac_manifold_positions(result: *const NativeResult) -> *const f64;
    fn ac_manifold_indices(result: *const NativeResult) -> *const u32;
    fn ac_manifold_free(result: *mut NativeResult);
}

struct Handle(NonNull<NativeResult>);

impl Handle {
    fn new(raw: *mut NativeResult, status: c_int) -> Result<Self, Error> {
        if status != 0 {
            if !raw.is_null() {
                // SAFETY: on error, a non-null pointer is still a transferred allocation.
                unsafe { ac_manifold_free(raw) };
            }
            return Err(match status {
                1 => Error::InvalidInput("C ABI rejected the input"),
                2 => Error::Backend("out of memory"),
                3 => Error::Backend("Manifold rejected the mesh or the operation failed"),
                4 => Error::TooLarge("native result exceeded its budget"),
                _ => Error::Backend("unknown native error code"),
            });
        }
        NonNull::new(raw).map(Self).ok_or(Error::Backend("null result on success"))
    }

    fn read(&self) -> Result<Mesh, Error> {
        let p = self.0.as_ptr().cast_const();
        // SAFETY: accessors are noexcept and the Handle holds a live result.
        let (n, t, xyz, faces) = unsafe {
            (ac_manifold_vertex_count(p), ac_manifold_triangle_count(p),
             ac_manifold_positions(p), ac_manifold_indices(p))
        };
        if n > MAX_RESULT_VERTICES || t > MAX_RESULT_TRIANGLES {
            return Err(Error::TooLarge("native result too big to copy"));
        }
        let ncoords = n.checked_mul(3).ok_or(Error::TooLarge("vertex stride overflow"))?;
        let nindices = t.checked_mul(3).ok_or(Error::TooLarge("triangle stride overflow"))?;
        if (ncoords != 0 && xyz.is_null()) || (nindices != 0 && faces.is_null()) {
            return Err(Error::Backend("null native output buffer"));
        }
        let vertices = if n == 0 {
            Vec::new()
        } else {
            // SAFETY: shim allocates exactly 3*n contiguous double values.
            unsafe { std::slice::from_raw_parts(xyz, ncoords) }
                .chunks_exact(3).map(|p| [p[0], p[1], p[2]]).collect()
        };
        let triangles = if t == 0 {
            Vec::new()
        } else {
            // SAFETY: shim allocates exactly 3*t contiguous u32 indices.
            unsafe { std::slice::from_raw_parts(faces, nindices) }
                .chunks_exact(3).map(|p| [p[0], p[1], p[2]]).collect()
        };
        let result = Mesh { vertices, triangles };
        // Empty intersection or full subtraction is a valid empty mesh.
        if result.vertices.is_empty() && result.triangles.is_empty() {
            return Ok(result);
        }
        if result.vertices.len() < 4 || result.triangles.len() < 4 ||
            result.vertices.iter().flat_map(|v| v.iter()).any(|v| !v.is_finite()) ||
            result.triangles.iter().flatten().any(|&i| (i as usize) >= result.vertices.len()) {
            return Err(Error::Backend("invalid native result"));
        }
        Ok(result)
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: unique non-null owner, freed once via the allocating C++ library.
        unsafe { ac_manifold_free(self.0.as_ptr()) };
    }
}

pub(super) fn boolean(a: &Mesh, b: &Mesh, op: BooleanOp) -> Result<Mesh, Error> {
    let code = match op {
        BooleanOp::Union => 0,
        BooleanOp::Difference => 1,
        BooleanOp::Intersection => 2,
    };
    let mut raw = std::ptr::null_mut();
    // SAFETY: Rust arrays of three numbers are contiguous. The C++ shim only reads them
    // during this call; lengths are checked, raw is a writable out pointer, and C++
    // catches exceptions. The vectors are not moved or modified during the call.
    let status = unsafe {
        ac_manifold_boolean(
            a.vertices.as_ptr().cast::<f64>(), a.vertices.len(),
            a.triangles.as_ptr().cast::<u32>(), a.triangles.len(),
            b.vertices.as_ptr().cast::<f64>(), b.vertices.len(),
            b.triangles.as_ptr().cast::<u32>(), b.triangles.len(),
            code, &mut raw,
        )
    };
    Handle::new(raw, status)?.read()
}
