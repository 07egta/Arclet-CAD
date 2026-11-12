//! Safe, thread-confined ownership of OCCT shapes. The only application FFI module.
//! A Shape is intentionally !Send and !Sync; construct and use it on the kernel worker.
use std::{ffi::{c_char, c_int, c_void, CStr, CString}, ptr::NonNull, rc::Rc};
use serde::{Deserialize, Serialize};
use crate::{Error, Result};
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, Serialize, Deserialize)]
pub struct Stats {
    pub volume: f64, pub area: f64, pub length: f64, pub bounds: [f64; 6],
    pub faces: u64, pub edges: u64, pub solids: u64,
}
#[repr(C)]
struct MeshView { positions: *const f64, normals: *const f64, lines: *const f64, indices: *const u32, vertices: usize, triangles: usize, segments: usize }
unsafe extern "C" {
    fn ac_version() -> *const c_char;
    fn ac_shape_free(p: *mut c_void);
    fn ac_op(op: c_int, a: *const c_void, b: *const c_void, values: *const f64, n: usize, group: *const *const c_void, count: usize, err: *mut c_char, cap: usize) -> *mut c_void;
    fn ac_stats(p: *const c_void, out: *mut Stats, err: *mut c_char, cap: usize) -> c_int;
    fn ac_brep(p: *const c_void, err: *mut c_char, cap: usize) -> *mut c_void;
    fn ac_bytes_free(p: *mut c_void);
    fn ac_bytes_data(p: *const c_void) -> *const c_char;
    fn ac_bytes_size(p: *const c_void) -> usize;
    fn ac_read(data: *const c_char, len: usize, format: c_int, err: *mut c_char, cap: usize) -> *mut c_void;
    fn ac_write(p: *const c_void, path: *const c_char, format: c_int, err: *mut c_char, cap: usize) -> c_int;
    fn ac_mesh(p: *const c_void, deflection: f64, err: *mut c_char, cap: usize) -> *mut c_void;
    fn ac_mesh_free(p: *mut c_void);
    fn ac_mesh_view(p: *const c_void) -> MeshView;
}
struct OwnedShape(NonNull<c_void>);
impl Drop for OwnedShape { fn drop(&mut self) {
    // SAFETY: this unique native allocation is released once, after the last Rc owner.
    unsafe { ac_shape_free(self.0.as_ptr()) };
} }
#[derive(Clone)]
pub struct Shape(Rc<OwnedShape>);
impl std::fmt::Debug for Shape { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("Shape(OCCT)") } }
struct OwnedBytes(NonNull<c_void>);
impl Drop for OwnedBytes { fn drop(&mut self) { unsafe { ac_bytes_free(self.0.as_ptr()) } } }
struct OwnedMesh(NonNull<c_void>);
impl Drop for OwnedMesh { fn drop(&mut self) { unsafe { ac_mesh_free(self.0.as_ptr()) } } }
#[derive(Clone, Debug, Default)]
pub struct Mesh { pub positions: Vec<[f64;3]>, pub normals: Vec<[f64;3]>, pub indices: Vec<u32>, pub lines: Vec<[[f64;3];2]> }
fn native_error(buf: &[c_char;1024]) -> Error {
    // The native bridge always writes a terminating NUL, including on exceptions.
    let s = unsafe { CStr::from_ptr(buf.as_ptr()) }.to_string_lossy();
    Error::new(if s.is_empty() { "Native operation failed without an error message".into() } else { s.into_owned() })
}
pub fn version() -> String { unsafe { CStr::from_ptr(ac_version()) }.to_string_lossy().into_owned() }
pub fn number(x: f64) -> Result<f64> {
    if x.is_finite() && x.abs() <= 1e8 { Ok(x) } else { Err(Error::new("Numbers must be finite and within +/-100,000,000 mm")) }
}
fn positive(x: f64) -> Result<f64> { number(x)?; if x > 1e-7 { Ok(x) } else { Err(Error::new("Dimension must be greater than 0.0000001 mm")) } }
impl Shape {
    fn ptr(&self) -> *const c_void { self.0.0.as_ptr() }
    pub fn shares_geometry(&self, other: &Self) -> bool { Rc::ptr_eq(&self.0, &other.0) }
    fn from_raw(raw: *mut c_void, err: &[c_char;1024]) -> Result<Self> {
        let p = NonNull::new(raw).ok_or_else(|| native_error(err))?; Ok(Self(Rc::new(OwnedShape(p))))
    }
    fn op(op: i32, a: Option<&Self>, b: Option<&Self>, values: &[f64], group: &[Self]) -> Result<Self> {
        if values.len()>2048 || group.len()>2048 { return Err(Error::new("Operation exceeds input limits")); }
        for x in values { number(*x)?; }
        let ptrs: Vec<_> = group.iter().map(Self::ptr).collect(); let mut err = [0;1024];
        // SAFETY: borrowed arrays and handles outlive this synchronous call; native exceptions are caught.
        let raw = unsafe { ac_op(op, a.map_or(std::ptr::null(),Self::ptr), b.map_or(std::ptr::null(),Self::ptr), values.as_ptr(),values.len(),ptrs.as_ptr(),ptrs.len(),err.as_mut_ptr(),err.len()) };
        Self::from_raw(raw,&err)
    }
    pub fn cuboid(origin: [f64;3], dims: [f64;3]) -> Result<Self> { for d in dims { positive(d)?; } Self::op(1,None,None,&[origin[0],origin[1],origin[2],dims[0],dims[1],dims[2]],&[]) }
    pub fn sphere(center: [f64;3], radius: f64) -> Result<Self> { positive(radius)?;Self::op(2,None,None,&[center[0],center[1],center[2],radius],&[]) }
    pub fn cylinder(center: [f64;3], radius: f64, height: f64) -> Result<Self> { positive(radius)?;positive(height)?;Self::op(3,None,None,&[center[0],center[1],center[2],radius,height],&[]) }
    pub fn cone(center: [f64;3], bottom: f64, top: f64, height: f64) -> Result<Self> { positive(height)?;Self::op(4,None,None,&[center[0],center[1],center[2],bottom,top,height],&[]) }
    pub fn torus(center: [f64;3], major: f64, minor: f64) -> Result<Self> { positive(minor)?;if major<=minor {return Err(Error::new("Major radius must exceed minor radius"));} Self::op(5,None,None,&[center[0],center[1],center[2],major,minor],&[]) }
    pub fn circle(center: [f64;3], radius: f64) -> Result<Self> { positive(radius)?;Self::op(6,None,None,&[center[0],center[1],center[2],radius],&[]) }
    pub fn polyline(points: &[[f64;3]], closed: bool) -> Result<Self> {
        if !(2..=256).contains(&points.len()) || (closed && points.len()<3) {return Err(Error::new("Polyline needs 2..256 points (3 if closed)"));}
        let mut v=vec![if closed {1.0} else {0.0}];v.extend(points.iter().flatten());Self::op(7,None,None,&v,&[])
    }
    /// Open-uniform rational B-spline. Controls are [x,y,z,positive_weight].
    pub fn nurbs(degree: u32, controls: &[[f64;4]]) -> Result<Self> {
        if !(1..=12).contains(&degree) || controls.len()<=degree as usize || controls.len()>256 {return Err(Error::new("NURBS requires degree 1..12 and degree+1..256 controls"));}
        for p in controls {if !(p[3]>1e-9 && p[3]<=1e6) {return Err(Error::new("NURBS weights must be in (1e-9, 1e6]"));}}
        let mut v=vec![degree as f64];v.extend(controls.iter().flatten());Self::op(8,None,None,&v,&[])
    }
    pub fn interpolate(points: &[[f64;3]]) -> Result<Self> { if !(2..=256).contains(&points.len()) {return Err(Error::new("Curve requires 2..256 points"));} Self::op(9,None,None,&points.iter().flatten().copied().collect::<Vec<_>>(),&[]) }
    pub fn extrude(&self, vector: [f64;3]) -> Result<Self> {Self::op(10,Some(self),None,&vector,&[])}
    pub fn revolve(&self, origin: [f64;3], axis: [f64;3], degrees: f64) -> Result<Self> { Self::op(11,Some(self),None,&[origin[0],origin[1],origin[2],axis[0],axis[1],axis[2],degrees],&[]) }
    pub fn loft(sections: &[Self], solid: bool) -> Result<Self> {Self::op(12,None,None,&[if solid {1.0}else{0.0}],sections)}
    pub fn union(&self, other: &Self) -> Result<Self> {Self::op(20,Some(self),Some(other),&[],&[])}
    pub fn difference(&self, other: &Self) -> Result<Self> {Self::op(21,Some(self),Some(other),&[],&[])}
    pub fn intersection(&self, other: &Self) -> Result<Self> {Self::op(22,Some(self),Some(other),&[],&[])}
    pub fn fillet(&self, radius: f64, edges: &[u32]) -> Result<Self> { positive(radius)?;let mut v=vec![radius];v.extend(edges.iter().map(|&x|x as f64));Self::op(23,Some(self),None,&v,&[]) }
    pub fn chamfer(&self, amount: f64, edges: &[u32]) -> Result<Self> { positive(amount)?;let mut v=vec![amount];v.extend(edges.iter().map(|&x|x as f64));Self::op(24,Some(self),None,&v,&[]) }
    pub fn translate(&self, vector: [f64;3]) -> Result<Self> {Self::op(25,Some(self),None,&vector,&[])}
    pub fn rotate(&self, origin: [f64;3], axis: [f64;3], degrees: f64) -> Result<Self> { Self::op(26,Some(self),None,&[origin[0],origin[1],origin[2],axis[0],axis[1],axis[2],degrees],&[]) }
    pub fn scale(&self, center: [f64;3], factor: f64) -> Result<Self> {Self::op(27,Some(self),None,&[center[0],center[1],center[2],factor],&[])}
    pub fn mirror(&self, origin: [f64;3], normal: [f64;3]) -> Result<Self> {Self::op(28,Some(self),None,&[origin[0],origin[1],origin[2],normal[0],normal[1],normal[2]],&[])}
    pub fn compound(shapes: &[Self]) -> Result<Self> {Self::op(30,None,None,&[],shapes)}
    pub fn section(&self, other: &Self) -> Result<Self> {Self::op(31,Some(self),Some(other),&[],&[])}
    pub fn stats(&self) -> Result<Stats> {
        let mut out=Stats::default();let mut err=[0;1024];
        if unsafe {ac_stats(self.ptr(),&mut out,err.as_mut_ptr(),err.len())}==0 {return Err(native_error(&err));}
        if [out.volume,out.area,out.length].iter().chain(out.bounds.iter()).any(|x|!x.is_finite()) {return Err(Error::new("Kernel returned non-finite measurements"));}
        if out.bounds.iter().any(|x|x.abs()>1e10) {return Err(Error::new("Result exceeds supported coordinate range"));} Ok(out)
    }
    pub fn to_brep(&self) -> Result<String> {
        let mut err=[0;1024];let raw=unsafe{ac_brep(self.ptr(),err.as_mut_ptr(),err.len())}; let owned=OwnedBytes(NonNull::new(raw).ok_or_else(||native_error(&err))?);
        let len=unsafe{ac_bytes_size(owned.0.as_ptr())};if len==0 || len>64*1024*1024 {return Err(Error::new("BREP exceeds serialization limit"));}
        let data=unsafe{ac_bytes_data(owned.0.as_ptr())};if data.is_null(){return Err(Error::new("Null BREP buffer"));}
        let bytes=unsafe{std::slice::from_raw_parts(data.cast::<u8>(),len)}; String::from_utf8(bytes.to_vec()).map_err(|_|Error::new("BREP is not UTF-8"))
    }
    pub fn from_brep(brep: &str) -> Result<Self> {Self::read(brep,1)}
    pub fn import_step(path: &str) -> Result<Self> { CString::new(path).map_err(|_|Error::new("Path contains a NUL"))?; Self::read(path,2) }
    fn read(data: &str, format: i32) -> Result<Self> {
        if data.is_empty() || data.len()>64*1024*1024 {return Err(Error::new("Input exceeds allowed size"));}
        let mut err=[0;1024];let raw=unsafe{ac_read(data.as_ptr().cast(),data.len(),format,err.as_mut_ptr(),err.len())};Self::from_raw(raw,&err)
    }
    pub fn write_step(&self, path: &str) -> Result<()> {self.write(path,1)}
    pub fn write_stl(&self, path: &str) -> Result<()> {self.write(path,2)}
    fn write(&self, path: &str, format: i32) -> Result<()> {
        let path=CString::new(path).map_err(|_|Error::new("Path contains a NUL"))?; let mut err=[0;1024];
        if unsafe{ac_write(self.ptr(),path.as_ptr(),format,err.as_mut_ptr(),err.len())}==0 {Err(native_error(&err))}else{Ok(())}
    }
    pub fn mesh(&self, deflection: f64) -> Result<Mesh> {
        number(deflection)?;let mut err=[0;1024];let raw=unsafe{ac_mesh(self.ptr(),deflection,err.as_mut_ptr(),err.len())};
        let owned=OwnedMesh(NonNull::new(raw).ok_or_else(||native_error(&err))?);let v=unsafe{ac_mesh_view(owned.0.as_ptr())};
        if v.vertices>2_000_000 || v.triangles>2_000_000 || v.segments>1_000_000 {return Err(Error::new("Native mesh exceeded limits"));}
        // SAFETY: MeshView points into the owned native vectors. Copy before the guard is dropped.
        // Zero-length vectors are handled without constructing a slice from a null pointer.
        unsafe fn copy<T:Copy>(p:*const T,len:usize)->Result<Vec<T>>{ if len==0{return Ok(Vec::new());}if p.is_null(){return Err(Error::new("Null native array"));} Ok(unsafe{std::slice::from_raw_parts(p,len)}.to_vec()) }
        let positions=unsafe{copy(v.positions,v.vertices*3)}?;let normals=unsafe{copy(v.normals,v.vertices*3)}?;
        let lines=unsafe{copy(v.lines,v.segments*6)}?;let indices=unsafe{copy(v.indices,v.triangles*3)}?;
        if positions.iter().chain(normals.iter()).chain(lines.iter()).any(|x|!x.is_finite()) || indices.iter().any(|&x|x as usize>=v.vertices) {return Err(Error::new("Invalid native tessellation"));}
        Ok(Mesh{positions:positions.chunks_exact(3).map(|p|[p[0],p[1],p[2]]).collect(),normals:normals.chunks_exact(3).map(|p|[p[0],p[1],p[2]]).collect(),indices,lines:lines.chunks_exact(6).map(|p|[[p[0],p[1],p[2]],[p[3],p[4],p[5]]]).collect()})
    }
}
