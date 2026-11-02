//! Pure-Rust backend (the default build).
//!
//! Offset: miter join, for convex simple polygons. Each vertex moves along its outward bisector
//! by `delta / (1 + n0 . n1)`. An inward offset that reverses an edge has collapsed the polygon,
//! and is returned as no rings.
//!
//! Boolean: intersection of two convex polygons by Sutherland-Hodgman clipping. Union, difference
//! and xor of convex polygons are not convex in general, so they return [`Error::Unsupported`].

use crate::{BooleanOp, Error, Point, Ring, check_delta, check_ring};

const BACKEND: &str = "pure-rust";

/// Offsets a convex simple polygon by `delta` (positive grows it, negative shrinks it), with
/// miter joins. Returns the offset ring, or no rings when the polygon collapses.
pub fn offset_polygon(points: &[Point], delta: f64) -> Result<Vec<Ring>, Error> {
    check_ring(points)?;
    check_delta(delta)?;
    let ring = clean(points);
    let sign = convex_orientation(&ring)?;
    if delta == 0.0 {
        return Ok(vec![ring]);
    }
    let n = ring.len();
    let mut normals = Vec::with_capacity(n);
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let len = dx.hypot(dy);
        if !(len > 0.0 && len.is_finite()) {
            return Err(Error::InvalidInput("edge length underflows"));
        }
        // Outward normal: right of the edge for a counter-clockwise ring, left for clockwise.
        normals.push([dy / len * sign, -dx / len * sign]);
    }
    let mut out: Ring = Vec::with_capacity(n);
    for i in 0..n {
        let n0 = normals[(i + n - 1) % n];
        let n1 = normals[i];
        let denom = 1.0 + (n0[0] * n1[0] + n0[1] * n1[1]);
        if !(denom > 1e-12) {
            return Err(Error::InvalidInput("corner too sharp for a miter join"));
        }
        let v = ring[i];
        let k = delta / denom;
        let p = [v[0] + k * (n0[0] + n1[0]), v[1] + k * (n0[1] + n1[1])];
        if !(p[0].is_finite() && p[1].is_finite()) {
            return Err(Error::InvalidInput("offset result out of range"));
        }
        out.push(p);
    }
    // An edge that reversed direction means the polygon has collapsed.
    for i in 0..n {
        let j = (i + 1) % n;
        let (a, b) = (ring[i], ring[j]);
        let (c, d) = (out[i], out[j]);
        if (b[0] - a[0]) * (d[0] - c[0]) + (b[1] - a[1]) * (d[1] - c[1]) <= 0.0 {
            return Ok(Vec::new());
        }
    }
    Ok(vec![out])
}

/// Boolean operation between two convex polygons. Only [`BooleanOp::Intersection`] is supported.
pub fn boolean(a: &[Point], b: &[Point], op: BooleanOp) -> Result<Vec<Ring>, Error> {
    check_ring(a)?;
    check_ring(b)?;
    match op {
        BooleanOp::Intersection => intersect_convex(a, b),
        _ => Err(Error::Unsupported { backend: BACKEND, what: "union, difference and xor (the pure backend intersects convex polygons only)" }),
    }
}

fn intersect_convex(a: &[Point], b: &[Point]) -> Result<Vec<Ring>, Error> {
    let mut poly = ccw(clean(a))?;
    let clip = ccw(clean(b))?;
    let m = clip.len();
    for i in 0..m {
        let (p, q) = (clip[i], clip[(i + 1) % m]);
        let input = std::mem::take(&mut poly);
        let k = input.len();
        for j in 0..k {
            let cur = input[j];
            let prev = input[(j + k - 1) % k];
            let cur_in = cross(p, q, cur) >= 0.0;
            let prev_in = cross(p, q, prev) >= 0.0;
            if cur_in {
                if !prev_in {
                    poly.push(cut(prev, cur, p, q));
                }
                poly.push(cur);
            } else if prev_in {
                poly.push(cut(prev, cur, p, q));
            }
        }
        if poly.is_empty() {
            break;
        }
    }
    if poly.len() < 3 { Ok(Vec::new()) } else { Ok(vec![poly]) }
}

/// Point where segment s-e crosses the line p-q. Only called when s and e lie on opposite sides.
fn cut(s: Point, e: Point, p: Point, q: Point) -> Point {
    let cs = cross(p, q, s);
    let ce = cross(p, q, e);
    let denom = cs - ce;
    let t = if denom != 0.0 { (cs / denom).clamp(0.0, 1.0) } else { 0.0 };
    [s[0] + t * (e[0] - s[0]), s[1] + t * (e[1] - s[1])]
}

/// Cross product (q - p) x (x - p): positive when x is left of the directed line p-q.
fn cross(p: Point, q: Point, x: Point) -> f64 {
    (q[0] - p[0]) * (x[1] - p[1]) - (q[1] - p[1]) * (x[0] - p[0])
}

/// Drops consecutive duplicates and a repeated closing point.
fn clean(points: &[Point]) -> Ring {
    let mut out: Ring = Vec::with_capacity(points.len());
    for &p in points {
        if out.last() != Some(&p) {
            out.push(p);
        }
    }
    while out.len() > 1 && out.first() == out.last() {
        out.pop();
    }
    out
}

fn ccw(mut ring: Ring) -> Result<Ring, Error> {
    if convex_orientation(&ring)? < 0.0 {
        ring.reverse();
    }
    Ok(ring)
}

/// Checks that a ring is convex and simple (total turning is one full turn), and returns +1.0 for
/// counter-clockwise or -1.0 for clockwise.
fn convex_orientation(ring: &[Point]) -> Result<f64, Error> {
    let n = ring.len();
    if n < 3 {
        return Err(Error::InvalidInput("fewer than three distinct points"));
    }
    let area2: f64 = (0..n)
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum();
    if !area2.is_finite() || area2 == 0.0 {
        return Err(Error::InvalidInput("zero-area ring"));
    }
    let sign = area2.signum();
    let mut turning = 0.0_f64;
    for i in 0..n {
        let (a, b, c) = (ring[i], ring[(i + 1) % n], ring[(i + 2) % n]);
        let (e1, e2) = ([b[0] - a[0], b[1] - a[1]], [c[0] - b[0], c[1] - b[1]]);
        let cr = e1[0] * e2[1] - e1[1] * e2[0];
        let dt = e1[0] * e2[0] + e1[1] * e2[1];
        let tol = 1e-12 * e1[0].hypot(e1[1]) * e2[0].hypot(e2[1]);
        if cr * sign < -tol {
            return Err(Error::Unsupported { backend: BACKEND, what: "non-convex polygon" });
        }
        turning += cr.atan2(dt);
    }
    if (turning.abs() - std::f64::consts::TAU).abs() > 1e-6 {
        return Err(Error::Unsupported { backend: BACKEND, what: "self-intersecting polygon" });
    }
    Ok(sign)
}
