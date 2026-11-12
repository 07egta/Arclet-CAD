//! Optional exact triangle-chunk indexing. No third-party dependencies.
//!
//! The algorithm uses bounded cell storage and stable counting scatter rather
//! than materialising corner/bounds cubes and sorting their full copies.
//! It does not change vertices, triangle order within cells, or tolerances.
#![deny(unsafe_op_in_unsafe_fn)]

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::slice;

const BAD_ARGUMENT: i32 = 1;
const BAD_INDEX: i32 = 2;
const NONFINITE: i32 = 3;
const ALLOCATION: i32 = 4;
const PANIC: i32 = 5;

type Bounds = ([f32; 3], [f32; 3]);

/// Owns all memory behind a successful C-ABI result.
pub struct Index {
    order: Vec<i32>,
    starts: Vec<i64>,
    mins: Vec<[f64; 3]>,
    maxs: Vec<[f64; 3]>,
}

/// Borrowed result, valid only until `serp_pick_free` is called.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct IndexView {
    order: *const i32,
    starts: *const i64,
    mins: *const f64,
    maxs: *const f64,
    primitives: usize,
    chunks: usize,
}

fn filled<T: Clone>(n: usize, value: T) -> Result<Vec<T>, i32> {
    let mut v = Vec::new();
    v.try_reserve_exact(n).map_err(|_| ALLOCATION)?;
    v.resize(n, value);
    Ok(v)
}

fn lower(a: f32, b: f32) -> f32 {
    if a < b {
        a
    } else {
        b
    }
}
fn upper(a: f32, b: f32) -> f32 {
    if a > b {
        a
    } else {
        b
    }
}

fn triangle_bounds(vertices: &[[f32; 3]], triangle: &[u32; 3]) -> Result<Bounds, i32> {
    let mut lo = [f32::INFINITY; 3];
    let mut hi = [f32::NEG_INFINITY; 3];
    for index in triangle {
        let point = vertices.get(*index as usize).ok_or(BAD_INDEX)?;
        for axis in 0..3 {
            let value = point[axis];
            if !value.is_finite() {
                return Err(NONFINITE);
            }
            lo[axis] = lower(lo[axis], value);
            hi[axis] = upper(hi[axis], value);
        }
    }
    Ok((lo, hi))
}

fn cell_key(bounds: Bounds, origin: [f64; 3], span: [f64; 3], grid: usize) -> Result<usize, i32> {
    let (lo, hi) = bounds;
    let mut key = 0;
    for axis in 0..3 {
        // Match NumPy's float32 midpoint then float64 normalisation. Do not
        // enable fast-math or replace this with a different midpoint formula.
        let midpoint = (lo[axis] + hi[axis]) * 0.5_f32;
        if !midpoint.is_finite() {
            return Err(NONFINITE);
        }
        let coordinate = ((f64::from(midpoint) - origin[axis]) / span[axis]) * grid as f64;
        let cell = coordinate.clamp(0.0, (grid - 1) as f64) as usize;
        key = key * grid + cell;
    }
    Ok(key)
}

fn build(vertices: &[[f32; 3]], triangles: &[[u32; 3]], grid: usize) -> Result<Index, i32> {
    if !(1..=64).contains(&grid) || triangles.len() > i32::MAX as usize {
        return Err(BAD_ARGUMENT);
    }
    if triangles.is_empty() {
        return Ok(Index {
            order: Vec::new(),
            starts: vec![0],
            mins: Vec::new(),
            maxs: Vec::new(),
        });
    }
    // Bounds must use referenced vertices only, as the NumPy implementation
    // does. Unused distant vertices cannot alter the grid or candidate order.
    let mut global_lo = [f32::INFINITY; 3];
    let mut global_hi = [f32::NEG_INFINITY; 3];
    for triangle in triangles {
        let (lo, hi) = triangle_bounds(vertices, triangle)?;
        for axis in 0..3 {
            global_lo[axis] = lower(global_lo[axis], lo[axis]);
            global_hi[axis] = upper(global_hi[axis], hi[axis]);
        }
    }
    let origin = global_lo.map(f64::from);
    let mut span = [0.0; 3];
    for axis in 0..3 {
        span[axis] = f64::from(global_hi[axis]) - origin[axis];
        if span[axis] < 1e-12 {
            span[axis] = 1.0;
        }
    }
    let cell_count = grid * grid * grid;
    let mut counts = filled(cell_count, 0_usize)?;
    let mut cell_lo = filled(cell_count, [f32::INFINITY; 3])?;
    let mut cell_hi = filled(cell_count, [f32::NEG_INFINITY; 3])?;
    let mut keys = filled(triangles.len(), 0_u32)?;
    for (i, triangle) in triangles.iter().enumerate() {
        let bounds = triangle_bounds(vertices, triangle)?;
        let key = cell_key(bounds, origin, span, grid)?;
        keys[i] = key as u32;
        counts[key] += 1;
        for axis in 0..3 {
            cell_lo[key][axis] = lower(cell_lo[key][axis], bounds.0[axis]);
            cell_hi[key][axis] = upper(cell_hi[key][axis], bounds.1[axis]);
        }
    }
    let occupied = counts.iter().filter(|&&count| count != 0).count();
    let mut starts = filled(occupied + 1, 0_i64)?;
    let mut mins = filled(occupied, [0.0; 3])?;
    let mut maxs = filled(occupied, [0.0; 3])?;
    let mut chunk = 0;
    let mut offset = 0_usize;
    for key in 0..cell_count {
        let count = counts[key];
        // Reuse counts as scatter cursors; no second cell-sized prefix array.
        counts[key] = offset;
        if count != 0 {
            starts[chunk] = offset as i64;
            mins[chunk] = cell_lo[key].map(f64::from);
            maxs[chunk] = cell_hi[key].map(f64::from);
            offset += count;
            chunk += 1;
        }
    }
    starts[occupied] = triangles.len() as i64;
    drop(cell_lo);
    drop(cell_hi);
    let mut order = filled(triangles.len(), 0_i32)?;
    for (i, &key) in keys.iter().enumerate() {
        let position = &mut counts[key as usize];
        order[*position] = i as i32;
        *position += 1;
    }
    Ok(Index {
        order,
        starts,
        mins,
        maxs,
    })
}

#[no_mangle]
pub extern "C" fn serp_pick_abi_version() -> u32 {
    1
}

/// Construct an owned index, writing null on every failure.
///
/// # Safety
/// `vertices` and `triangles` must be aligned pointers to the stated numbers
/// of live packed triples, readable and unmodified for this call. `out` must
/// point to a writable pointer slot. Non-null, valid pointers are a caller
/// obligation; indices, lengths, grid size and finite values are validated.
#[no_mangle]
pub unsafe extern "C" fn serp_pick_build_f32(
    vertices: *const f32,
    vertex_count: usize,
    triangles: *const u32,
    triangle_count: usize,
    grid: u32,
    out: *mut *mut Index,
) -> i32 {
    if out.is_null() {
        return BAD_ARGUMENT;
    }
    // SAFETY: the caller guarantees `out` points to a writable pointer slot.
    unsafe { *out = ptr::null_mut() };
    let max_triples = isize::MAX as usize / std::mem::size_of::<[f32; 3]>();
    if vertices.is_null()
        || triangles.is_null()
        || vertex_count > max_triples
        || triangle_count > max_triples
        || !vertices.is_aligned()
        || !triangles.is_aligned()
    {
        return BAD_ARGUMENT;
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: lengths/alignment were checked, allocation validity and
        // immutability are part of the documented foreign caller contract.
        let vertices = unsafe { slice::from_raw_parts(vertices.cast::<[f32; 3]>(), vertex_count) };
        let triangles =
            unsafe { slice::from_raw_parts(triangles.cast::<[u32; 3]>(), triangle_count) };
        build(vertices, triangles, grid as usize)
    }));
    match result {
        Ok(Ok(index)) => {
            // SAFETY: as above, `out` is writable; ownership passes to caller.
            unsafe { *out = Box::into_raw(Box::new(index)) };
            0
        }
        Ok(Err(code)) => code,
        Err(_) => PANIC,
    }
}

/// Borrow result arrays without transferring ownership.
///
/// # Safety
/// `index` must be a live pointer returned by `serp_pick_build_f32`; `out`
/// must point to a writable IndexView. Do not read arrays after freeing it.
#[no_mangle]
pub unsafe extern "C" fn serp_pick_view(index: *const Index, out: *mut IndexView) -> i32 {
    if index.is_null() || out.is_null() {
        return BAD_ARGUMENT;
    }
    // SAFETY: the caller guarantees a live handle and writable view slot.
    let index = unsafe { &*index };
    unsafe {
        *out = IndexView {
            order: index.order.as_ptr(),
            starts: index.starts.as_ptr(),
            mins: index.mins.as_ptr().cast(),
            maxs: index.maxs.as_ptr().cast(),
            primitives: index.order.len(),
            chunks: index.mins.len(),
        };
    }
    0
}

/// Release an index. Null is accepted. No arrays may be used afterwards.
///
/// # Safety
/// Non-null must be a live handle returned by `serp_pick_build_f32`, freed
/// exactly once. This function must not race with reads of its views.
#[no_mangle]
pub unsafe extern "C" fn serp_pick_free(index: *mut Index) {
    if !index.is_null() {
        // SAFETY: caller transfers a live, uniquely owned Box allocation.
        drop(unsafe { Box::from_raw(index) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stable_grouping_and_conservative_bounds() {
        let v = [[0., 0., 0.], [1., 1., 1.], [10., 10., 10.], [11., 11., 11.]];
        let t = [[2, 3, 2], [0, 1, 0], [2, 2, 3], [1, 0, 1]];
        let index = build(&v, &t, 4).unwrap();
        assert_eq!(index.order, [1, 3, 0, 2]);
        assert_eq!(index.starts, [0, 2, 4]);
        assert_eq!(index.mins, [[0., 0., 0.], [10., 10., 10.]]);
        assert_eq!(index.maxs, [[1., 1., 1.], [11., 11., 11.]]);
    }
    #[test]
    fn unused_vertices_do_not_change_cells() {
        let v = [[0., 0., 0.], [1., 1., 1.], [f32::NAN, 1e20, -1e20]];
        let index = build(&v, &[[0, 1, 0]], 2).unwrap();
        assert_eq!(index.mins, [[0., 0., 0.]]);
    }
    #[test]
    fn invalid_indices_and_nonfinite_are_errors() {
        assert!(matches!(build(&[[0.; 3]], &[[0, 1, 0]], 4), Err(BAD_INDEX)));
        assert!(matches!(
            build(&[[f32::NAN; 3]], &[[0; 3]], 4),
            Err(NONFINITE)
        ));
        assert!(matches!(
            build(&[[f32::MAX; 3]], &[[0; 3]], 4),
            Err(NONFINITE)
        ));
    }
    #[test]
    fn grid_limits_and_empty_input() {
        for grid in [0, 65, usize::MAX] {
            assert!(matches!(build(&[], &[], grid), Err(BAD_ARGUMENT)));
        }
        assert_eq!(build(&[], &[], 1).unwrap().starts, [0]);
    }
    #[test]
    fn all_points_in_one_cell_are_stable() {
        let index = build(&[[3., 4., 5.]], &[[0; 3]; 100], 64).unwrap();
        assert_eq!(index.order, (0..100).collect::<Vec<i32>>());
        assert_eq!(index.starts, [0, 100]);
    }
    #[test]
    fn abi_lifecycle_and_failure_nulling() {
        let v = [[0_f32; 3], [1.; 3]];
        let t = [[0_u32, 1, 0]];
        let mut ptr = ptr::null_mut();
        unsafe {
            assert_eq!(
                serp_pick_build_f32(v.as_ptr().cast(), 2, t.as_ptr().cast(), 1, 4, &mut ptr),
                0
            );
            assert!(!ptr.is_null());
            let mut view = std::mem::MaybeUninit::<IndexView>::uninit();
            assert_eq!(serp_pick_view(ptr, view.as_mut_ptr()), 0);
            let view = view.assume_init();
            assert_eq!((view.primitives, view.chunks), (1, 1));
            assert_eq!(*view.order, 0);
            serp_pick_free(ptr);
            ptr = ptr::dangling_mut::<Index>();
            assert_eq!(
                serp_pick_build_f32(ptr::null(), 0, t.as_ptr().cast(), 1, 4, &mut ptr),
                BAD_ARGUMENT
            );
            assert!(ptr.is_null());
            serp_pick_free(ptr);
        }
    }
}
