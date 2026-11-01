//! A small FDM slicer: triangles to layers, walls, rectilinear infill and G-code text.
//!
//! Everything is plain numbers in millimetres, so it is tested without a printer. Inputs are
//! hostile: every count is capped and bad numbers are errors, never panics.
//!
//! Limits (stated, not hidden): walls use a mitre offset, so they suit shapes without very sharp
//! inner corners; solids that overlap each other are sliced with the even-odd rule, so overlapping
//! parts cancel; there are no supports or bridges. Solid top/bottom skins are detected from
//! neighbouring cross-sections, including exposed shelves inside the model's height range.

use std::collections::HashMap;
use std::fmt::Write as _;

use solidcraft_geom::{Vec2, Vec3};

mod fill;

use fill::{FillKind, FillPlanner, WorkBudget};

/// One triangle.
pub type Tri = [Vec3; 3];

/// Most layers one slice may make.
pub const MAX_LAYERS: usize = 20_000;
/// Most triangles one slice may read.
pub const MAX_TRIANGLES: usize = 2_000_000;
/// Most perimeter walls.
pub const MAX_WALLS: u32 = 8;
/// Most solid layers requested on either side of a surface.
pub const MAX_SKIN_LAYERS: u32 = 64;
/// Most vertices accepted across an already sliced model.
pub const MAX_SLICE_POINTS: usize = 2_000_000;
/// Largest absolute coordinate accepted by the G-code planner, in millimetres.
pub const MAX_COORDINATE: f64 = 1_000_000.0;
/// Maximum generated text size; exceeding it is an error, never a partial print file.
pub const MAX_GCODE_BYTES: usize = 128 * 1024 * 1024;
const GCODE_Z_STEP: f64 = 0.001;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum SliceError {
    #[error("{0} must be a number greater than zero")]
    BadNumber(&'static str),
    #[error("there is nothing to slice (no triangles, or the shape is flat)")]
    Empty,
    #[error("too many triangles (up to {MAX_TRIANGLES})")]
    TooManyTriangles,
    #[error("too many layers (up to {MAX_LAYERS}); use a thicker layer height")]
    TooManyLayers,
    #[error("the slice exceeds the geometry planning limit")]
    TooComplex,
    #[error("a slice contour branches at an edge; repair the non-manifold mesh")]
    BranchingContour,
    #[error("stopped")]
    Cancelled,
}

/// One layer: the outlines of the material at its middle height.
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    /// Height of the layer's top, measured from the bottom of the shape.
    pub top: f64,
    /// Closed outlines. Outer loops run counter-clockwise, holes clockwise.
    pub loops: Vec<Vec<Vec2>>,
    /// Outlines that did not close: the shape has holes in its surface here.
    pub open: u32,
}

/// The result of slicing a shape.
#[derive(Clone, Debug, PartialEq)]
pub struct Sliced {
    pub layer_height: f64,
    pub layers: Vec<Layer>,
}

impl Sliced {
    /// How many outlines failed to close, over all layers. More than zero means the shape is not
    /// watertight and will print with gaps.
    pub fn open_outlines(&self) -> u32 {
        self.layers.iter().map(|l| l.open).fold(0, u32::saturating_add)
    }
}

fn finite3(v: Vec3) -> bool {
    v.x.is_finite() && v.y.is_finite() && v.z.is_finite()
}

/// Cuts `tris` into layers `layer_height` thick, the shape's lowest point sitting at height 0.
pub fn slice(tris: &[Tri], layer_height: f64) -> Result<Sliced, SliceError> {
    slice_until(tris, layer_height, || false)
}

/// Slices triangles while checking `stop` between layers and batches of geometry work.
pub fn slice_until(tris: &[Tri], layer_height: f64, mut stop: impl FnMut() -> bool) -> Result<Sliced, SliceError> {
    slice_checkpoint(&mut stop)?;
    if !(layer_height > 1e-3) || !layer_height.is_finite() {
        return Err(SliceError::BadNumber("layer height"));
    }
    if tris.len() > MAX_TRIANGLES {
        return Err(SliceError::TooManyTriangles);
    }
    let mut zmin = f64::INFINITY;
    let mut zmax = f64::NEG_INFINITY;
    for (i, t) in tris.iter().enumerate() {
        if i % 256 == 0 {
            slice_checkpoint(&mut stop)?;
        }
        for v in t {
            if !finite3(*v) || v.x.abs() > MAX_COORDINATE || v.y.abs() > MAX_COORDINATE || v.z.abs() > MAX_COORDINATE {
                return Err(SliceError::BadNumber("coordinate"));
            }
            zmin = zmin.min(v.z);
            zmax = zmax.max(v.z);
        }
    }
    let height = zmax - zmin;
    if !(height > 1e-9) || !height.is_finite() {
        return Err(SliceError::Empty);
    }
    let mut count = (height / layer_height).ceil();
    // A floating-point quotient can round just above an integer. Also merge a genuine
    // remainder below the writer's Z quantum: it must not become an extra extrusion at
    // the same serialized height. Preserve the actual model top in the final layer.
    if count > 1.0 && height - (count - 1.0) * layer_height < GCODE_Z_STEP {
        count -= 1.0;
    }
    if count > MAX_LAYERS as f64 {
        return Err(SliceError::TooManyLayers);
    }
    let count = count as usize;
    let mut layers = Vec::with_capacity(count);
    let mut work = 200_000_000usize;
    let mut points = 0usize;
    for i in 0..count {
        slice_checkpoint(&mut stop)?;
        work = work.checked_sub(tris.len()).ok_or(SliceError::TooComplex)?;
        let bottom = i as f64 * layer_height;
        let top = if i + 1 == count { height } else { ((i as f64 + 1.0) * layer_height).min(height) };
        let z = zmin + (bottom + top) * 0.5;
        let (loops, open) = loops_at(tris, z, &mut stop, &mut work)?;
        for contour in &loops {
            points = points.checked_add(contour.len()).ok_or(SliceError::TooComplex)?;
            if points > MAX_SLICE_POINTS {
                return Err(SliceError::TooComplex);
            }
        }
        layers.push(Layer { top, loops, open });
    }
    slice_checkpoint(&mut stop)?;
    Ok(Sliced { layer_height, layers })
}

fn slice_checkpoint(stop: &mut impl FnMut() -> bool) -> Result<(), SliceError> {
    if stop() { Err(SliceError::Cancelled) } else { Ok(()) }
}

type Key = (i64, i64);

fn key(p: Vec2) -> Key {
    ((p.x * 1e4).round() as i64, (p.y * 1e4).round() as i64)
}

/// The point where the edge `a`-`b` meets height `z`; the same for either order of `a` and `b`.
fn cross_edge(a: Vec3, b: Vec3, z: f64) -> Vec2 {
    let (a, b) = if (a.x, a.y, a.z).partial_cmp(&(b.x, b.y, b.z)) == Some(std::cmp::Ordering::Greater) { (b, a) } else { (a, b) };
    let dz = b.z - a.z;
    let t = if dz.abs() < 1e-12 { 0.0 } else { ((z - a.z) / dz).clamp(0.0, 1.0) };
    Vec2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

fn loops_at(tris: &[Tri], z: f64, stop: &mut impl FnMut() -> bool, work: &mut usize) -> Result<(Vec<Vec<Vec2>>, u32), SliceError> {
    let mut segs: Vec<(Vec2, Vec2)> = Vec::new();
    for (index, t) in tris.iter().enumerate() {
        if index % 256 == 0 {
            slice_checkpoint(stop)?;
        }
        // A vertex exactly on the cut counts as above it, so no triangle is cut twice at a corner.
        let above = [t[0].z > z - 1e-9, t[1].z > z - 1e-9, t[2].z > z - 1e-9];
        if above[0] == above[1] && above[1] == above[2] {
            continue;
        }
        let mut pts: Vec<Vec2> = Vec::with_capacity(2);
        for (i, j) in [(0usize, 1usize), (1, 2), (2, 0)] {
            if above[i] != above[j] {
                pts.push(cross_edge(t[i], t[j], z));
            }
        }
        if let [a, b] = pts[..]
            && key(a) != key(b)
        {
            segs.push((a, b));
        }
    }
    let mut by_end: HashMap<Key, Vec<usize>> = HashMap::new();
    for (i, (a, b)) in segs.iter().enumerate() {
        if i % 256 == 0 {
            slice_checkpoint(stop)?;
        }
        for endpoint in [key(*a), key(*b)] {
            let incident = by_end.entry(endpoint).or_default();
            if incident.len() >= 2 {
                return Err(SliceError::BranchingContour);
            }
            incident.push(i);
        }
    }
    let mut used = vec![false; segs.len()];
    let mut loops: Vec<Vec<Vec2>> = Vec::new();
    let mut open = 0u32;
    for start in 0..segs.len() {
        if start % 256 == 0 {
            slice_checkpoint(stop)?;
        }
        if used[start] {
            continue;
        }
        used[start] = true;
        let (first, mut at) = segs[start];
        let mut pts = vec![first];
        let mut closed = false;
        for step in 0..=segs.len() {
            if step % 256 == 0 {
                slice_checkpoint(stop)?;
            }
            if key(at) == key(first) {
                closed = true;
                break;
            }
            pts.push(at);
            let next = by_end.get(&key(at)).and_then(|c| c.iter().copied().find(|k| !used[*k]));
            let Some(k) = next else { break };
            used[k] = true;
            let (a, b) = segs[k];
            at = if key(a) == key(at) { b } else { a };
        }
        if closed && pts.len() >= 3 {
            loops.push(pts);
        } else {
            open = open.saturating_add(1);
        }
    }
    Ok((orient(loops, stop, work)?, open))
}

/// Signed area: positive when the loop runs counter-clockwise.
pub fn area(l: &[Vec2]) -> f64 {
    let mut s = 0.0;
    for (i, a) in l.iter().enumerate() {
        let b = l[(i + 1) % l.len()];
        s += a.x * b.y - b.x * a.y;
    }
    s * 0.5
}

fn contains(l: &[Vec2], p: Vec2, stop: &mut impl FnMut() -> bool) -> Result<bool, SliceError> {
    let mut inside = false;
    let mut j = l.len().wrapping_sub(1);
    for (i, a) in l.iter().enumerate() {
        if i % 256 == 0 {
            slice_checkpoint(stop)?;
        }
        let b = l[j];
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    Ok(inside)
}

/// Outer loops counter-clockwise, loops inside an odd number of others (holes) clockwise.
fn orient(mut loops: Vec<Vec<Vec2>>, stop: &mut impl FnMut() -> bool, work: &mut usize) -> Result<Vec<Vec<Vec2>>, SliceError> {
    let mut depths = Vec::with_capacity(loops.len());
    for (i, contour) in loops.iter().enumerate() {
        slice_checkpoint(stop)?;
        let p = contour.first().copied().unwrap_or(Vec2::ZERO);
        let mut depth = 0;
        for (j, other) in loops.iter().enumerate() {
            if i != j {
                *work = work.checked_sub(other.len()).ok_or(SliceError::TooComplex)?;
                if contains(other, p, stop)? {
                    depth += 1;
                }
            }
        }
        depths.push(depth);
    }
    for (l, d) in loops.iter_mut().zip(depths) {
        let ccw = area(l) > 0.0;
        if ccw != (d % 2 == 0) {
            l.reverse();
        }
    }
    Ok(loops)
}

/// Moves a loop sideways by `d` into the material (positive) or out of it (negative).
/// `None` when the loop collapses.
pub fn offset(l: &[Vec2], d: f64) -> Option<Vec<Vec2>> {
    let n = l.len();
    if n < 3 {
        return None;
    }
    let mut out: Vec<Vec2> = Vec::with_capacity(n);
    for i in 0..n {
        let p = l[i];
        let prev = l[(i + n - 1) % n];
        let next = l[(i + 1) % n];
        let n1 = left_normal(prev, p)?;
        let n2 = left_normal(p, next)?;
        let denom = (1.0 + n1.dot(n2)).max(0.25);
        let m = Vec2::new((n1.x + n2.x) / denom, (n1.y + n2.y) / denom);
        out.push(Vec2::new(p.x + m.x * d, p.y + m.y * d));
    }
    // A loop that was pushed past its own middle comes out turned inside out, which the sign of
    // its area cannot show (a square turned half way round is still counter-clockwise). It shows
    // in the edges: one that now points the opposite way means the loop has collapsed.
    for i in 0..n {
        let (a, b) = (l[i], l[(i + 1) % n]);
        let (c, e) = (out[i], out[(i + 1) % n]);
        if (b.x - a.x) * (e.x - c.x) + (b.y - a.y) * (e.y - c.y) <= 0.0 {
            return None;
        }
    }
    if area(&out).abs() < 1e-6 {
        return None;
    }
    Some(out)
}

fn left_normal(a: Vec2, b: Vec2) -> Option<Vec2> {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len = dx.hypot(dy);
    if len < 1e-9 { None } else { Some(Vec2::new(-dy / len, dx / len)) }
}

/// Parallel lines `spacing` apart at `angle` (radians), cut to the inside of `loops` (even-odd).
pub fn infill(loops: &[Vec<Vec2>], spacing: f64, angle: f64) -> Vec<(Vec2, Vec2)> {
    let mut out = Vec::new();
    if !(spacing > 1e-3) || loops.is_empty() {
        return out;
    }
    let (s, c) = angle.sin_cos();
    let rot = |p: Vec2, a: f64, b: f64| Vec2::new(p.x * a - p.y * b, p.x * b + p.y * a);
    let turned: Vec<Vec<Vec2>> = loops.iter().map(|l| l.iter().map(|p| rot(*p, c, -s)).collect()).collect();
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for p in turned.iter().flatten() {
        lo = lo.min(p.y);
        hi = hi.max(p.y);
    }
    if !(hi > lo) || (hi - lo) / spacing > 20_000.0 {
        return out;
    }
    let mut y = (lo / spacing).ceil() * spacing + spacing * 0.5;
    while y < hi {
        let mut xs: Vec<f64> = Vec::new();
        for l in &turned {
            for (i, a) in l.iter().enumerate() {
                let b = l[(i + 1) % l.len()];
                if (a.y <= y) != (b.y <= y) {
                    xs.push(a.x + (y - a.y) / (b.y - a.y) * (b.x - a.x));
                }
            }
        }
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        for pair in xs.as_chunks::<2>().0 {
            if pair[1] - pair[0] > 0.05 {
                out.push((rot(Vec2::new(pair[0], y), c, s), rot(Vec2::new(pair[1], y), c, s)));
            }
        }
        y += spacing;
    }
    out
}

/// Printer and filament settings.
#[derive(Clone, Debug, PartialEq)]
pub struct PrintConfig {
    pub line_width: f64,
    pub filament_diameter: f64,
    pub nozzle_temp: f64,
    pub bed_temp: f64,
    pub walls: u32,
    /// Distance between infill lines; zero turns infill off.
    /// Used when this layer has no density override in [`Self::layer_infill`].
    pub infill_spacing: f64,
    /// Solid layers below an exposed top surface. Zero disables top skins.
    pub top_solid_layers: u32,
    /// Solid layers above an exposed bottom surface. Zero disables bottom skins.
    pub bottom_solid_layers: u32,
    /// Optional density overrides, indexed from zero. Unlisted layers use `infill_spacing`.
    /// Skin regions stay solid even when a layer's sparse infill density is zero.
    pub layer_infill: Vec<LayerInfill>,
    pub print_speed: f64,
    pub travel_speed: f64,
    pub retract: f64,
    /// Where the middle of the shape goes on the bed.
    pub bed_center: (f64, f64),
    /// Bed size; the shape must fit.
    pub bed_size: (f64, f64),
    pub start_gcode: String,
    pub end_gcode: String,
}

impl Default for PrintConfig {
    /// A generic Marlin-style printer with a 180 x 180 mm bed (Bambu A1 mini size).
    /// The start and end G-code are plain placeholders: Bambu machines need their own.
    fn default() -> Self {
        PrintConfig {
            line_width: 0.42,
            filament_diameter: 1.75,
            nozzle_temp: 220.0,
            bed_temp: 60.0,
            walls: 2,
            infill_spacing: 3.0,
            top_solid_layers: 3,
            bottom_solid_layers: 3,
            layer_infill: Vec::new(),
            print_speed: 60.0,
            travel_speed: 150.0,
            retract: 0.8,
            bed_center: (90.0, 90.0),
            bed_size: (180.0, 180.0),
            start_gcode: "G28 ; home all axes".to_string(),
            end_gcode: "M84 ; motors off".to_string(),
        }
    }
}

/// Sparse infill density for one layer, expressed as a fraction from zero to one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayerInfill {
    pub layer: usize,
    pub density: f64,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum GcodeError {
    #[error("{0} must be a number greater than zero")]
    BadNumber(&'static str),
    #[error("nothing to print")]
    Empty,
    #[error("the shape does not fit on the bed ({0:.0} x {1:.0} mm needed)")]
    TooBig(f64, f64),
    #[error("invalid print setting: {0}")]
    BadSetting(&'static str),
    #[error("the sliced model has open contours; repair the mesh before printing")]
    OpenContours,
    #[error("the sliced model or toolpath exceeds the planning limit: {0}")]
    TooComplex(&'static str),
    #[error("stopped")]
    Cancelled,
}

struct Writer<'a> {
    cfg: &'a PrintConfig,
    centre: Vec2,
    text: String,
    at: Vec2,
    e_per_mm: f64,
    layer_h: f64,
    retracted: bool,
}

impl Writer<'_> {
    fn line(&mut self, s: &str) {
        self.text.push_str(s);
        self.text.push('\n');
    }
    fn place(&self, p: Vec2) -> Vec2 {
        Vec2::new(p.x - self.centre.x + self.cfg.bed_center.0, p.y - self.centre.y + self.cfg.bed_center.1)
    }
    fn extrusion_position(&self, point: Vec2) -> Result<Vec2, GcodeError> {
        let to = self.place(point);
        let margin = self.cfg.line_width * 0.5;
        let tolerance = GCODE_Z_STEP * 0.5;
        if !to.x.is_finite()
            || !to.y.is_finite()
            || to.x < margin - tolerance
            || to.x > self.cfg.bed_size.0 - margin + tolerance
            || to.y < margin - tolerance
            || to.y > self.cfg.bed_size.1 - margin + tolerance
        {
            return Err(GcodeError::BadSetting("planned extrusion lies outside the bed"));
        }
        Ok(to)
    }
    fn travel(&mut self, to: Vec2) -> Result<(), GcodeError> {
        let to = self.extrusion_position(to)?;
        if !self.retracted {
            let _ = writeln!(self.text, "G1 E-{:.3} F2400", self.cfg.retract);
            self.retracted = true;
        }
        let _ = writeln!(self.text, "G0 X{:.3} Y{:.3} F{:.0}", to.x, to.y, self.cfg.travel_speed * 60.0);
        self.at = to;
        self.check_size()
    }
    fn print_to(&mut self, to: Vec2) -> Result<(), GcodeError> {
        let to = self.extrusion_position(to)?;
        if self.retracted {
            let _ = writeln!(self.text, "G1 E{:.3} F2400", self.cfg.retract);
            self.retracted = false;
        }
        let len = (to.x - self.at.x).hypot(to.y - self.at.y);
        let e = len * self.cfg.line_width * self.layer_h * self.e_per_mm;
        if !e.is_finite() {
            return Err(GcodeError::BadSetting("extrusion is not finite"));
        }
        let _ = writeln!(self.text, "G1 X{:.3} Y{:.3} E{:.5} F{:.0}", to.x, to.y, e, self.cfg.print_speed * 60.0);
        self.at = to;
        self.check_size()
    }
    fn check_size(&self) -> Result<(), GcodeError> {
        if self.text.len() > MAX_GCODE_BYTES { Err(GcodeError::TooComplex("G-code bytes")) } else { Ok(()) }
    }
    fn path(&mut self, pts: &[Vec2], closed: bool, budget: &mut WorkBudget<'_>) -> Result<(), GcodeError> {
        let Some(first) = pts.first().copied() else { return Ok(()) };
        budget.checkpoint()?;
        self.travel(first)?;
        for (i, p) in pts.iter().skip(1).enumerate() {
            if i % 256 == 0 {
                budget.checkpoint()?;
            }
            budget.segment()?;
            self.print_to(*p)?;
        }
        if closed {
            budget.segment()?;
            self.print_to(first)?;
        }
        self.check_size()
    }
}

/// Turns sliced layers into G-code text.
pub fn gcode(sliced: &Sliced, cfg: &PrintConfig) -> Result<String, GcodeError> {
    gcode_until(sliced, cfg, || false)
}

/// Turns layers into G-code, checking `stop` during validation, layers and scanline planning.
/// A cancellation or planning limit returns an error; incomplete G-code is never returned.
pub fn gcode_until(sliced: &Sliced, cfg: &PrintConfig, mut stop: impl FnMut() -> bool) -> Result<String, GcodeError> {
    let mut budget = WorkBudget::new(&mut stop);
    let (centre, densities) = validate_print(sliced, cfg, &mut budget)?;
    let area_filament = std::f64::consts::PI * (cfg.filament_diameter * 0.5).powi(2);
    let mut w_ =
        Writer { cfg, centre, text: String::new(), at: Vec2::ZERO, e_per_mm: 1.0 / area_filament, layer_h: sliced.layer_height, retracted: false };
    w_.line("; generated by SolidCraft slice");
    w_.line("; Generic printer output; Bambu hardware compatibility is unverified");
    w_.line("G21 ; millimetres");
    w_.line("G90 ; absolute moves");
    w_.line("M83 ; relative extrusion");
    let _ = writeln!(w_.text, "M140 S{:.0}", cfg.bed_temp);
    let _ = writeln!(w_.text, "M104 S{:.0}", cfg.nozzle_temp);
    let _ = writeln!(w_.text, "M190 S{:.0}", cfg.bed_temp);
    let _ = writeln!(w_.text, "M109 S{:.0}", cfg.nozzle_temp);
    w_.line(&cfg.start_gcode);
    let mut previous_top = 0.0;
    for (n, layer) in sliced.layers.iter().enumerate() {
        budget.checkpoint()?;
        let print_top = quantized_z(layer.top);
        w_.layer_h = print_top - previous_top;
        previous_top = print_top;
        let _ = writeln!(w_.text, ";LAYER:{n}");
        let _ = writeln!(w_.text, "G1 Z{print_top:.3} F600");
        let mut inner: Vec<Vec<Vec2>> = Vec::new();
        w_.line(";TYPE:WALL");
        for contour in &layer.loops {
            budget.spend(contour.len())?;
            let mut cur = contour.clone();
            for k in 0..cfg.walls {
                budget.spend(cur.len())?;
                let Some(wall) = offset(&cur, if k == 0 { cfg.line_width * 0.5 } else { cfg.line_width }) else { break };
                w_.path(&wall, true, &mut budget)?;
                cur = wall;
            }
            if let Some(core) = offset(&cur, cfg.line_width * 0.5) {
                inner.push(core);
            }
        }
        let (full_skin, neighbours) = skin_neighbours(sliced, cfg, n);
        let angle = if n % 2 == 0 { std::f64::consts::FRAC_PI_4 } else { -std::f64::consts::FRAC_PI_4 };
        let planner = FillPlanner::new(&inner, &neighbours, full_skin, angle, &mut budget)?;
        let density = densities.get(n).copied().flatten();
        let spacing = match density {
            Some(0.0) => 0.0,
            Some(value) => cfg.line_width / value,
            None => cfg.infill_spacing,
        };
        for (kind, distance) in [(FillKind::Skin, cfg.line_width), (FillKind::Infill, spacing)] {
            let paths = planner.paths(distance, kind, &mut budget)?;
            if !paths.is_empty() {
                w_.line(match kind {
                    FillKind::Skin => ";TYPE:SKIN",
                    FillKind::Infill => ";TYPE:INFILL",
                });
            }
            for (a, b) in paths {
                w_.path(&[a, b], false, &mut budget)?;
            }
        }
    }
    budget.checkpoint()?;
    w_.line("M104 S0");
    w_.line("M140 S0");
    w_.line(&cfg.end_gcode);
    w_.check_size()?;
    Ok(w_.text)
}

fn skin_neighbours<'a>(sliced: &'a Sliced, cfg: &PrintConfig, index: usize) -> (bool, Vec<&'a [Vec<Vec2>]>) {
    let below = cfg.bottom_solid_layers as usize;
    let above = cfg.top_solid_layers as usize;
    let after = sliced.layers.len().saturating_sub(index.saturating_add(1));
    if index < below || after < above {
        return (true, Vec::new());
    }
    let mut neighbours = Vec::with_capacity(below + above);
    for distance in 1..=below {
        if let Some(layer) = sliced.layers.get(index.saturating_sub(distance)) {
            neighbours.push(layer.loops.as_slice());
        }
    }
    for distance in 1..=above {
        if let Some(layer) = sliced.layers.get(index.saturating_add(distance)) {
            neighbours.push(layer.loops.as_slice());
        }
    }
    (false, neighbours)
}

fn quantized_z(top: f64) -> f64 {
    (top / GCODE_Z_STEP).round() * GCODE_Z_STEP
}

fn validate_print(sliced: &Sliced, cfg: &PrintConfig, budget: &mut WorkBudget<'_>) -> Result<(Vec2, Vec<Option<f64>>), GcodeError> {
    budget.checkpoint()?;
    if sliced.layers.is_empty() {
        return Err(GcodeError::Empty);
    }
    if sliced.layers.len() > MAX_LAYERS {
        return Err(GcodeError::TooComplex("layer count"));
    }
    for (name, v) in [
        ("line width", cfg.line_width),
        ("filament diameter", cfg.filament_diameter),
        ("print speed", cfg.print_speed),
        ("travel speed", cfg.travel_speed),
        ("layer height", sliced.layer_height),
        ("bed width", cfg.bed_size.0),
        ("bed depth", cfg.bed_size.1),
    ] {
        if !(v > 0.0) || !v.is_finite() || v > MAX_COORDINATE {
            return Err(GcodeError::BadNumber(name));
        }
    }
    for (name, v) in [
        ("nozzle temperature", cfg.nozzle_temp),
        ("bed temperature", cfg.bed_temp),
        ("retraction", cfg.retract),
        ("infill spacing", cfg.infill_spacing),
    ] {
        if !v.is_finite() || !(0.0..=MAX_COORDINATE).contains(&v) {
            return Err(GcodeError::BadSetting(name));
        }
    }
    if cfg.line_width <= 1e-3 || cfg.filament_diameter <= 1e-3 || (cfg.infill_spacing > 0.0 && cfg.infill_spacing <= 1e-3) {
        return Err(GcodeError::BadSetting("line width, filament diameter and nonzero spacing must exceed 0.001 mm"));
    }
    if cfg.walls > MAX_WALLS || cfg.top_solid_layers > MAX_SKIN_LAYERS || cfg.bottom_solid_layers > MAX_SKIN_LAYERS {
        return Err(GcodeError::BadSetting("wall or skin layer count"));
    }
    if cfg.start_gcode.len() > 1024 * 1024 || cfg.end_gcode.len() > 1024 * 1024 {
        return Err(GcodeError::TooComplex("start/end G-code bytes"));
    }
    if cfg.layer_infill.len() > sliced.layers.len() {
        return Err(GcodeError::BadSetting("too many layer density overrides"));
    }
    let mut densities = vec![None; sliced.layers.len()];
    for override_ in &cfg.layer_infill {
        budget.checkpoint()?;
        if !override_.density.is_finite() || !(0.0..=1.0).contains(&override_.density) {
            return Err(GcodeError::BadSetting("infill density must be in 0..=1"));
        }
        let entry = densities.get_mut(override_.layer).ok_or(GcodeError::BadSetting("density layer does not exist"))?;
        if entry.is_some() {
            return Err(GcodeError::BadSetting("duplicate layer density override"));
        }
        *entry = Some(override_.density);
    }
    let (mut lo, mut hi) = (Vec2::new(f64::INFINITY, f64::INFINITY), Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY));
    let mut points = 0usize;
    let mut previous_top = 0.0;
    let mut previous_print_top = 0.0;
    for layer in &sliced.layers {
        budget.checkpoint()?;
        if !layer.top.is_finite() || layer.top <= previous_top || layer.top > MAX_COORDINATE {
            return Err(GcodeError::BadSetting("layer heights must be finite and strictly increasing"));
        }
        previous_top = layer.top;
        let print_top = quantized_z(layer.top);
        if print_top <= previous_print_top {
            return Err(GcodeError::BadSetting("layer heights collapse at 0.001 mm G-code precision"));
        }
        previous_print_top = print_top;
        if layer.open != 0 {
            return Err(GcodeError::OpenContours);
        }
        for contour in &layer.loops {
            points = points.checked_add(contour.len()).ok_or(GcodeError::TooComplex("contour vertices"))?;
            if points > MAX_SLICE_POINTS {
                return Err(GcodeError::TooComplex("contour vertices"));
            }
            if contour.len() < 3 {
                return Err(GcodeError::BadSetting("a contour needs at least three vertices"));
            }
            for chunk in contour.chunks(256) {
                budget.spend(chunk.len())?;
                for p in chunk {
                    if !p.x.is_finite() || !p.y.is_finite() || p.x.abs() > MAX_COORDINATE || p.y.abs() > MAX_COORDINATE {
                        return Err(GcodeError::BadSetting("contour coordinate"));
                    }
                    lo = Vec2::new(lo.x.min(p.x), lo.y.min(p.y));
                    hi = Vec2::new(hi.x.max(p.x), hi.y.max(p.y));
                }
            }
        }
    }
    if !(hi.x >= lo.x) {
        return Err(GcodeError::Empty);
    }
    let (w, d) = (hi.x - lo.x, hi.y - lo.y);
    if w > cfg.bed_size.0 || d > cfg.bed_size.1 {
        return Err(GcodeError::TooBig(w, d));
    }
    if !cfg.bed_center.0.is_finite()
        || !cfg.bed_center.1.is_finite()
        || cfg.bed_center.0 - w * 0.5 < 0.0
        || cfg.bed_center.0 + w * 0.5 > cfg.bed_size.0
        || cfg.bed_center.1 - d * 0.5 < 0.0
        || cfg.bed_center.1 + d * 0.5 > cfg.bed_size.1
    {
        return Err(GcodeError::BadSetting("centred shape lies outside the bed"));
    }
    let centre = Vec2::new((lo.x + hi.x) * 0.5, (lo.y + hi.y) * 0.5);
    Ok((centre, densities))
}

/// Slices `tris` and writes G-code in one step.
pub fn print_triangles(tris: &[Tri], layer_height: f64, cfg: &PrintConfig) -> Result<String, String> {
    let sliced = slice(tris, layer_height).map_err(|e| e.to_string())?;
    gcode(&sliced, cfg).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An axis-aligned box from (0,0,0) to (w,d,h), 12 triangles.
    pub fn cuboid(w: f64, d: f64, h: f64, ox: f64) -> Vec<Tri> {
        let v = |x: f64, y: f64, z: f64| Vec3::new(ox + x * w, y * d, z * h);
        let q = |a: Vec3, b: Vec3, c: Vec3, e: Vec3| vec![[a, b, c], [a, c, e]];
        let mut t = Vec::new();
        t.extend(q(v(0., 0., 0.), v(1., 0., 0.), v(1., 1., 0.), v(0., 1., 0.)));
        t.extend(q(v(0., 0., 1.), v(1., 0., 1.), v(1., 1., 1.), v(0., 1., 1.)));
        t.extend(q(v(0., 0., 0.), v(1., 0., 0.), v(1., 0., 1.), v(0., 0., 1.)));
        t.extend(q(v(1., 0., 0.), v(1., 1., 0.), v(1., 1., 1.), v(1., 0., 1.)));
        t.extend(q(v(1., 1., 0.), v(0., 1., 0.), v(0., 1., 1.), v(1., 1., 1.)));
        t.extend(q(v(0., 1., 0.), v(0., 0., 0.), v(0., 0., 1.), v(0., 1., 1.)));
        t
    }

    #[test]
    fn box_gives_one_ccw_square_per_layer() {
        let s = slice(&cuboid(10.0, 10.0, 2.0, 0.0), 0.2).unwrap();
        assert_eq!(s.layers.len(), 10);
        for l in &s.layers {
            assert_eq!(l.loops.len(), 1);
            assert!((area(&l.loops[0]) - 100.0).abs() < 1e-6);
        }
    }

    #[test]
    fn a_box_with_a_missing_side_is_reported_as_open() {
        let mut t = cuboid(10.0, 10.0, 2.0, 0.0);
        t.truncate(t.len() - 2);
        let s = slice(&t, 0.2).unwrap();
        assert_eq!(slice(&cuboid(10.0, 10.0, 2.0, 0.0), 0.2).unwrap().open_outlines(), 0);
        assert!(s.open_outlines() > 0);
    }

    #[test]
    fn two_separate_boxes_give_two_loops() {
        let mut t = cuboid(10.0, 10.0, 2.0, 0.0);
        t.extend(cuboid(10.0, 10.0, 2.0, 30.0));
        let s = slice(&t, 0.2).unwrap();
        assert!(s.layers.iter().all(|l| l.loops.len() == 2));
    }

    #[test]
    fn bad_input_is_an_error_not_a_panic() {
        assert_eq!(slice(&[], 0.2), Err(SliceError::Empty));
        assert!(slice(&cuboid(1.0, 1.0, 1.0, 0.0), f64::NAN).is_err());
        assert!(slice(&cuboid(1.0, 1.0, 1.0, 0.0), 0.0).is_err());
        let nan = [[Vec3::new(f64::NAN, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 1.0)]];
        assert!(slice(&nan, 0.2).is_err());
        assert!(slice(&cuboid(1.0, 1.0, 1000.0, 0.0), 1e-3 * 1.5).is_err());
    }

    #[test]
    fn offset_shrinks_a_square() {
        let sq = vec![Vec2::new(0., 0.), Vec2::new(10., 0.), Vec2::new(10., 10.), Vec2::new(0., 10.)];
        let o = offset(&sq, 1.0).unwrap();
        assert!((area(&o) - 64.0).abs() < 1e-6);
        assert!(offset(&sq, 6.0).is_none());
    }

    #[test]
    fn infill_covers_a_square() {
        let sq = vec![Vec2::new(0., 0.), Vec2::new(10., 0.), Vec2::new(10., 10.), Vec2::new(0., 10.)];
        let lines = infill(&[sq], 2.0, 0.0);
        assert_eq!(lines.len(), 5);
        assert!(lines.iter().all(|(a, b)| (b.x - a.x - 10.0).abs() < 1e-9));
    }

    #[test]
    fn gcode_has_layers_and_positive_extrusion() {
        let g = print_triangles(&cuboid(20.0, 20.0, 2.0, 0.0), 0.2, &PrintConfig::default()).unwrap();
        assert_eq!(g.matches(";LAYER:").count(), 10);
        assert!(g.contains("G1 X"));
        assert!(g.contains("M109 S220"));
        for l in g.lines().filter(|l| l.starts_with("G1 X")) {
            let e: f64 = l.split(" E").nth(1).and_then(|s| s.split(' ').next()).and_then(|s| s.parse().ok()).unwrap();
            assert!(e > 0.0);
        }
    }

    #[test]
    fn a_shape_bigger_than_the_bed_is_refused() {
        let r = print_triangles(&cuboid(500.0, 20.0, 2.0, 0.0), 0.2, &PrintConfig::default());
        assert!(r.unwrap_err().contains("does not fit"));
    }

    fn rectangle(x0: f64, x1: f64, y0: f64, y1: f64) -> Vec<Vec2> {
        vec![Vec2::new(x0, y0), Vec2::new(x1, y0), Vec2::new(x1, y1), Vec2::new(x0, y1)]
    }

    fn path_length(paths: &[(Vec2, Vec2)]) -> f64 {
        paths.iter().map(|(a, b)| (b.x - a.x).hypot(b.y - a.y)).sum()
    }

    fn planned_fill(sliced: &Sliced, cfg: &PrintConfig, layer: usize, spacing: f64, kind: FillKind) -> Vec<(Vec2, Vec2)> {
        let mut stop = || false;
        let mut budget = WorkBudget::new(&mut stop);
        let (full, neighbours) = skin_neighbours(sliced, cfg, layer);
        let planner = FillPlanner::new(&sliced.layers[layer].loops, &neighbours, full, 0.0, &mut budget).unwrap();
        planner.paths(spacing, kind, &mut budget).unwrap()
    }

    fn sections(widths: &[f64]) -> Sliced {
        Sliced {
            layer_height: 0.2,
            layers: widths
                .iter()
                .enumerate()
                .map(|(i, width)| Layer { top: (i + 1) as f64 * 0.2, loops: vec![rectangle(0.0, *width, 0.0, 10.0)], open: 0 })
                .collect(),
        }
    }

    #[test]
    fn skins_close_requested_terminal_layers_and_can_be_disabled() {
        let sliced = slice(&cuboid(20.0, 20.0, 2.0, 0.0), 0.2).unwrap();
        let mut cfg = PrintConfig { top_solid_layers: 2, bottom_solid_layers: 1, ..PrintConfig::default() };
        let g = gcode(&sliced, &cfg).unwrap();
        let layers: Vec<_> = g.split(";LAYER:").skip(1).collect();
        assert_eq!(layers.len(), 10);
        for (i, layer) in layers.iter().enumerate() {
            assert_eq!(layer.contains(";TYPE:SKIN"), matches!(i, 0 | 8 | 9));
            assert_eq!(layer.contains(";TYPE:INFILL"), (1..8).contains(&i));
        }
        cfg.top_solid_layers = 0;
        cfg.bottom_solid_layers = 0;
        let old_style = gcode(&sliced, &cfg).unwrap();
        assert!(!old_style.contains(";TYPE:SKIN"));
        assert_eq!(old_style.matches(";TYPE:INFILL").count(), 10);
    }

    #[test]
    fn partial_top_roofs_get_skins_through_the_requested_depth() {
        let sliced = sections(&[10.0, 10.0, 10.0, 5.0, 5.0, 5.0]);
        let cfg = PrintConfig { top_solid_layers: 2, bottom_solid_layers: 0, ..PrintConfig::default() };
        let skin = planned_fill(&sliced, &cfg, 1, 1.0, FillKind::Skin);
        assert!((path_length(&skin) - 50.0).abs() < 1e-9);
        assert!(skin.iter().all(|(a, b)| a.x >= 5.0 && b.x <= 10.0));
        let core = planned_fill(&sliced, &cfg, 1, 2.0, FillKind::Infill);
        assert!((path_length(&core) - 25.0).abs() < 1e-9);
        assert!(core.iter().all(|(a, b)| a.x >= 0.0 && b.x <= 5.0));
        assert!(planned_fill(&sliced, &cfg, 0, 1.0, FillKind::Skin).is_empty());
    }

    #[test]
    fn local_bottom_surfaces_get_skins_without_filling_the_whole_layer() {
        let sliced = sections(&[5.0, 10.0, 10.0, 10.0]);
        let cfg = PrintConfig { top_solid_layers: 0, bottom_solid_layers: 2, ..PrintConfig::default() };
        let skin = planned_fill(&sliced, &cfg, 2, 1.0, FillKind::Skin);
        assert!((path_length(&skin) - 50.0).abs() < 1e-9);
        assert!(skin.iter().all(|(a, b)| a.x >= 5.0 && b.x <= 10.0));
        assert!(planned_fill(&sliced, &cfg, 3, 1.0, FillKind::Skin).is_empty());
    }

    #[test]
    fn skins_leave_through_holes_open_and_do_not_double_fill() {
        let mut hole = rectangle(4.0, 6.0, 4.0, 6.0);
        hole.reverse();
        let sliced = Sliced { layer_height: 0.2, layers: vec![Layer { top: 0.2, loops: vec![rectangle(0.0, 10.0, 0.0, 10.0), hole], open: 0 }] };
        let cfg = PrintConfig::default();
        let skin = planned_fill(&sliced, &cfg, 0, 1.0, FillKind::Skin);
        assert!((path_length(&skin) - 96.0).abs() < 1e-9);
        assert!(skin.iter().all(|(a, b)| !(a.y > 4.0 && a.y < 6.0) || b.x <= 4.0 || a.x >= 6.0));
        assert!(planned_fill(&sliced, &cfg, 0, 1.0, FillKind::Infill).is_empty());
        let g = gcode(&sliced, &cfg).unwrap();
        assert_eq!(g.matches(";TYPE:SKIN").count(), 1);
        assert!(!g.contains(";TYPE:INFILL"));
    }

    #[test]
    fn solid_infill_and_partial_skin_form_a_single_coverage() {
        let sliced = sections(&[10.0, 5.0]);
        let cfg = PrintConfig { top_solid_layers: 1, bottom_solid_layers: 0, ..PrintConfig::default() };
        let skin = planned_fill(&sliced, &cfg, 0, 1.0, FillKind::Skin);
        let core = planned_fill(&sliced, &cfg, 0, 1.0, FillKind::Infill);
        assert!((path_length(&skin) + path_length(&core) - 100.0).abs() < 1e-9);
        for (a, b) in &skin {
            for (c, d) in &core {
                if (a.y - c.y).abs() < 1e-9 {
                    assert!(a.x >= d.x || c.x >= b.x);
                }
            }
        }
    }

    #[test]
    fn density_overrides_are_per_layer_and_do_not_turn_off_skins() {
        let sliced = sections(&[20.0; 7]);
        let cfg = PrintConfig {
            walls: 0,
            top_solid_layers: 1,
            bottom_solid_layers: 1,
            layer_infill: vec![
                LayerInfill { layer: 0, density: 0.0 },
                LayerInfill { layer: 1, density: 0.0 },
                LayerInfill { layer: 2, density: 0.25 },
                LayerInfill { layer: 4, density: 1.0 },
            ],
            ..PrintConfig::default()
        };
        let g = gcode(&sliced, &cfg).unwrap();
        let layers: Vec<_> = g.split(";LAYER:").skip(1).collect();
        assert!(layers[0].contains(";TYPE:SKIN"));
        assert!(!layers[1].contains("G1 X"));
        let quarter = layers[2].matches("G1 X").count();
        let solid = layers[4].matches("G1 X").count();
        assert!(quarter > 0);
        assert!(solid > quarter * 3, "quarter={quarter}, solid={solid}");
        assert!(layers[3].contains(";TYPE:INFILL"));
    }

    #[test]
    fn final_partial_layer_is_kept_and_uses_its_actual_thickness() {
        let sliced = slice(&cuboid(10.0, 10.0, 0.45, 0.0), 0.2).unwrap();
        assert_eq!(sliced.layers.len(), 3);
        assert!((sliced.layers[2].top - 0.45).abs() < 1e-9);
        let cfg = PrintConfig { top_solid_layers: 0, bottom_solid_layers: 0, infill_spacing: 0.0, walls: 1, ..PrintConfig::default() };
        let g = gcode(&sliced, &cfg).unwrap();
        let totals: Vec<f64> = g
            .split(";LAYER:")
            .skip(1)
            .map(|layer| {
                layer
                    .lines()
                    .filter(|line| line.starts_with("G1 X"))
                    .map(|line| line.split(" E").nth(1).unwrap().split(' ').next().unwrap().parse::<f64>().unwrap())
                    .sum()
            })
            .collect();
        assert!((totals[2] / totals[0] - 0.25).abs() < 1e-4);
        let thin = slice(&cuboid(10.0, 10.0, 0.05, 0.0), 0.2).unwrap();
        assert_eq!(thin.layers.len(), 1);
        assert!((thin.layers[0].top - 0.05).abs() < 1e-9);
    }

    #[test]
    fn rounding_tails_do_not_create_duplicate_print_heights() {
        for (height, offset, expected) in [(0.6000000000000001, 0.0, 3), (0.6, 100_000.0, 3), (0.4004, 0.0, 2)] {
            let mut mesh = cuboid(10.0, 10.0, height, 0.0);
            for vertex in mesh.iter_mut().flatten() {
                vertex.z += offset;
            }
            let sliced = slice(&mesh, 0.2).unwrap();
            assert_eq!(sliced.layers.len(), expected);
            assert!((sliced.layers.last().unwrap().top - height).abs() < 1e-9);
            let g = gcode(&sliced, &PrintConfig::default()).unwrap();
            let heights: Vec<f64> =
                g.lines().filter_map(|line| line.strip_prefix("G1 Z").and_then(|v| v.split(' ').next()).and_then(|v| v.parse().ok())).collect();
            assert_eq!(heights.len(), expected);
            assert!(heights.windows(2).all(|pair| pair[1] > pair[0]));
        }
        let mut sliced = sections(&[10.0, 10.0]);
        sliced.layers[1].top = 0.2004;
        assert!(matches!(gcode(&sliced, &PrintConfig::default()), Err(GcodeError::BadSetting(_))));
    }

    #[test]
    fn invalid_densities_counts_and_non_finite_fields_are_errors() {
        let sliced = sections(&[10.0; 8]);
        for density in [f64::NAN, f64::INFINITY, -0.1, 1.01] {
            let cfg = PrintConfig { layer_infill: vec![LayerInfill { layer: 2, density }], ..PrintConfig::default() };
            assert!(matches!(gcode(&sliced, &cfg), Err(GcodeError::BadSetting(_))));
        }
        for overrides in [vec![LayerInfill { layer: 8, density: 0.5 }], vec![LayerInfill { layer: 2, density: 0.5 }; 2]] {
            assert!(gcode(&sliced, &PrintConfig { layer_infill: overrides, ..PrintConfig::default() }).is_err());
        }
        assert!(gcode(&sliced, &PrintConfig { top_solid_layers: MAX_SKIN_LAYERS + 1, ..PrintConfig::default() }).is_err());
        for value in [f64::NAN, f64::INFINITY, -1.0] {
            assert!(gcode(&sliced, &PrintConfig { infill_spacing: value, ..PrintConfig::default() }).is_err());
            assert!(gcode(&sliced, &PrintConfig { nozzle_temp: value, ..PrintConfig::default() }).is_err());
        }
        assert!(gcode(&sliced, &PrintConfig { bed_center: (0.0, 90.0), ..PrintConfig::default() }).is_err());
        let mut malformed = sliced.clone();
        malformed.layers[0].loops[0][0].x = f64::NAN;
        assert!(gcode(&malformed, &PrintConfig::default()).is_err());
        malformed = sliced;
        malformed.layers[2].top = 0.1;
        assert!(gcode(&malformed, &PrintConfig::default()).is_err());
    }

    #[test]
    fn open_meshes_and_excessive_fill_are_refused_instead_of_partly_printed() {
        let mut mesh = cuboid(10.0, 10.0, 2.0, 0.0);
        mesh.truncate(mesh.len() - 2);
        let sliced = slice(&mesh, 0.2).unwrap();
        assert_eq!(gcode(&sliced, &PrintConfig::default()), Err(GcodeError::OpenContours));
        let large = sections(&[100.0]);
        let cfg = PrintConfig { walls: 0, line_width: 0.002, ..PrintConfig::default() };
        assert!(matches!(gcode(&large, &cfg), Err(GcodeError::TooComplex(_))));
    }

    #[test]
    fn planned_extrusion_is_checked_against_the_bed_after_offsets() {
        let mut clockwise = rectangle(0.0, 180.0, 0.0, 180.0);
        clockwise.reverse();
        let malformed = Sliced { layer_height: 0.2, layers: vec![Layer { top: 0.2, loops: vec![clockwise], open: 0 }] };
        assert_eq!(gcode(&malformed, &PrintConfig::default()), Err(GcodeError::BadSetting("planned extrusion lies outside the bed")));
        let valid = slice(&cuboid(180.0, 180.0, 0.2, 0.0), 0.2).unwrap();
        assert!(gcode(&valid, &PrintConfig::default()).is_ok());
    }

    #[test]
    fn slicing_and_gcode_can_stop_inside_geometry_work() {
        let mesh = cuboid(20.0, 20.0, 2.0, 0.0);
        assert_eq!(slice_until(&mesh, 0.2, || true), Err(SliceError::Cancelled));
        let mut calls = 0;
        let cancelled = slice_until(&mesh, 0.2, || {
            calls += 1;
            calls > 12
        });
        assert_eq!(cancelled, Err(SliceError::Cancelled));
        let sliced = slice(&mesh, 0.2).unwrap();
        assert_eq!(gcode_until(&sliced, &PrintConfig::default(), || true), Err(GcodeError::Cancelled));
        calls = 0;
        let cancelled = gcode_until(&sliced, &PrintConfig::default(), || {
            calls += 1;
            calls > 100
        });
        assert_eq!(cancelled, Err(GcodeError::Cancelled));
        assert_eq!(gcode_until(&sliced, &PrintConfig::default(), || false).unwrap(), gcode(&sliced, &PrintConfig::default()).unwrap());
    }

    #[test]
    fn branching_contours_are_refused_before_adjacency_search_can_grow_quadratically() {
        let mesh: Vec<Tri> = (1..=4).map(|i| [Vec3::new(2.0 * f64::from(i), 2.0, 1.0), Vec3::ZERO, Vec3::new(0.0, 0.0, 1.0)]).collect();
        assert_eq!(slice(&mesh, 1.0), Err(SliceError::BranchingContour));
    }
}
