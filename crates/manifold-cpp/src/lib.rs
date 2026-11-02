//! Independent, optional mesh-only Boolean backend for Arclet CAD.
//! The output is never a NURBS or exact B-rep. Without the native feature the API
//! returns a clear Unavailable error; callers choose their own fallback.
//! No dependency on the Arclet B-rep kernel.

#![deny(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

#[cfg(feature = "manifold")]
mod ffi;

/// Guard against accidentally allocating enormous native buffers.
pub const MAX_VERTICES: usize = 1_000_000;
pub const MAX_TRIANGLES: usize = 2_000_000;
#[cfg(feature = "manifold")]
pub(crate) const MAX_RESULT_VERTICES: usize = 4_000_000;
#[cfg(feature = "manifold")]
pub(crate) const MAX_RESULT_TRIANGLES: usize = 8_000_000;
pub const MAX_COORD: f64 = 1.0e9;

/// Outward-oriented closed triangle mesh with zero-based indices.
#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    pub vertices: Vec<[f64; 3]>,
    pub triangles: Vec<[u32; 3]>,
}

impl Mesh {
    /// Check types, lengths, finiteness, and indices before calling native code.
    pub fn validate(&self) -> Result<(), Error> {
        if self.vertices.len() > MAX_VERTICES || self.triangles.len() > MAX_TRIANGLES {
            return Err(Error::TooLarge("mesh exceeds input vertex/triangle budget"));
        }
        if self.vertices.len() < 4 || self.triangles.len() < 4 {
            return Err(Error::InvalidInput("need at least four vertices and four triangles"));
        }
        for v in &self.vertices {
            if !v.iter().all(|c| c.is_finite()) {
                return Err(Error::InvalidInput("NaN or infinity in vertex"));
            }
            if !v.iter().all(|c| c.abs() <= MAX_COORD) {
                return Err(Error::TooLarge("vertex outside coordinate budget"));
            }
        }
        for t in &self.triangles {
            let [a, b, c] = *t;
            if a == b || b == c || a == c {
                return Err(Error::InvalidInput("repeated index in triangle"));
            }
            if [a, b, c].iter().any(|&i| (i as usize) >= self.vertices.len()) {
                return Err(Error::InvalidInput("triangle index out of bounds"));
            }
            let u = self.vertices[b as usize];
            let v = self.vertices[c as usize];
            let origin = self.vertices[a as usize];
            let ab = [u[0] - origin[0], u[1] - origin[1], u[2] - origin[2]];
            let ac = [v[0] - origin[0], v[1] - origin[1], v[2] - origin[2]];
            let normal = [
                ab[1] * ac[2] - ab[2] * ac[1],
                ab[2] * ac[0] - ab[0] * ac[2],
                ab[0] * ac[1] - ab[1] * ac[0],
            ];
            if normal.iter().all(|n| *n == 0.0) {
                return Err(Error::InvalidInput("zero-area triangle"));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanOp {
    Union,
    Difference,
    Intersection,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("invalid mesh: {0}")]
    InvalidInput(&'static str),
    #[error("mesh too large: {0}")]
    TooLarge(&'static str),
    #[error("Manifold C++ backend is not enabled; enable feature manifold with MANIFOLD_SRC")]
    Unavailable,
    #[error("Manifold backend failed: {0}")]
    Backend(&'static str),
}

pub fn backend() -> &'static str {
    if cfg!(feature = "manifold") { "manifold-cpp" } else { "unavailable" }
}

/// Boolean mesh operation. No silent conversion to exact B-rep or automatic fallback.
pub fn boolean(a: &Mesh, b: &Mesh, op: BooleanOp) -> Result<Mesh, Error> {
    a.validate()?;
    b.validate()?;
    #[cfg(feature = "manifold")]
    {
        ffi::boolean(a, b, op)
    }
    #[cfg(not(feature = "manifold"))]
    {
        let _ = (a, b, op);
        Err(Error::Unavailable)
    }
}
