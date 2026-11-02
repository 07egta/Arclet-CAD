//! Dimension geometry in paper millimetres, driven by model-space measurements.
//!
//! Dimensions deliberately keep the measured value separate from graphics scale:
//! 1:2 makes lines half as long on paper, not the physical measurement.
//! These are geometry/label primitives for a drawing editor to render, not a
//! claim of complete ISO 129-1/ASME Y14.5 annotation compliance.

use crate::{Seg2, Vec2, Vec3};

use super::{Invalid, ViewFrame};

/// Explicit display units; all source length measurements are model millimetres.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LengthUnit {
    Millimetre,
    Centimetre,
    Metre,
    Inch,
    Foot,
}

impl LengthUnit {
    pub fn mm_per_unit(self) -> f64 {
        match self {
            LengthUnit::Millimetre => 1.0,
            LengthUnit::Centimetre => 10.0,
            LengthUnit::Metre => 1000.0,
            LengthUnit::Inch => 25.4,
            LengthUnit::Foot => 304.8,
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            LengthUnit::Millimetre => "mm",
            LengthUnit::Centimetre => "cm",
            LengthUnit::Metre => "m",
            LengthUnit::Inch => "in",
            LengthUnit::Foot => "ft",
        }
    }
}

/// Sizes and rounding on a printed sheet (except the selected display unit).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DimensionStyle {
    pub unit: LengthUnit,
    pub precision: u8,
    pub text_height_mm: f64,
    pub arrow_length_mm: f64,
    pub extension_gap_mm: f64,
    pub extension_overhang_mm: f64,
}

impl Default for DimensionStyle {
    fn default() -> Self {
        DimensionStyle {
            unit: LengthUnit::Millimetre,
            precision: 2,
            text_height_mm: 3.5,
            arrow_length_mm: 3.0,
            extension_gap_mm: 1.0,
            extension_overhang_mm: 1.5,
        }
    }
}

impl DimensionStyle {
    fn validate(self) -> Result<(), Invalid> {
        let valid = [self.text_height_mm, self.arrow_length_mm, self.extension_gap_mm, self.extension_overhang_mm]
            .iter()
            .all(|v| v.is_finite() && (0.0..=100.0).contains(v));
        if !valid || self.text_height_mm <= 0.0 || self.arrow_length_mm <= 0.0 || self.precision > 6 {
            return Err(Invalid("invalid dimension style: finite sheet sizes and precision 0..6 required"));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DimensionKind {
    Linear,
    Radius,
    Diameter,
    Angle,
}

/// A ready-to-draw dimension: lines/arcs, triangular arrowheads and label anchor.
/// Text outlines and collision avoidance are the renderer's responsibility.
#[derive(Clone, Debug, PartialEq)]
pub struct DimensionGraphics {
    pub kind: DimensionKind,
    /// Model length in mm for linear/radial/diameter, degrees for angles.
    pub measured_value: f64,
    pub label: String,
    pub strokes: Vec<Seg2>,
    pub arrowheads: Vec<[Vec2; 3]>,
    pub text_position_mm: Vec2,
    pub text_angle_rad: f64,
}

fn formatted(value_mm: f64, style: DimensionStyle, prefix: &str) -> String {
    let display = value_mm / style.unit.mm_per_unit();
    let display = if display == 0.0 { 0.0 } else { display };
    format!("{prefix}{display:.precision$} {}", style.unit.symbol(), precision = usize::from(style.precision))
}

fn line(a: Vec2, b: Vec2) -> Seg2 {
    Seg2::Line { a, b }
}

fn arrow(tip: Vec2, toward: Vec2, size: f64) -> [Vec2; 3] {
    let back = toward * size;
    [tip, tip + back + toward.perp() * (size * 0.25), tip + back - toward.perp() * (size * 0.25)]
}

fn finite_paper(points: &[Vec2]) -> bool {
    points.iter().all(|p| p.is_finite() && p.x.abs() <= 1e12 && p.y.abs() <= 1e12)
}

/// Aligned linear dimension of two points in the view plane. Length is
/// computed from the projected *model* distance, never from measured paper
/// units. A signed offset selects the side for dimension/extension lines.
pub fn linear_dimension(
    frame: &ViewFrame,
    a: Vec3,
    b: Vec3,
    offset_mm: f64,
    style: DimensionStyle,
) -> Result<DimensionGraphics, Invalid> {
    style.validate()?;
    if !a.is_finite() || !b.is_finite() || !offset_mm.is_finite() || offset_mm.abs() > 5000.0 || offset_mm.abs() < 1e-9 {
        return Err(Invalid("linear dimension needs finite endpoints and a nonzero sheet offset"));
    }
    let (p, q) = (frame.to_paper(a), frame.to_paper(b));
    let d = q - p;
    let length_mm = d.len() / frame.scale().factor();
    let u = d.normalized().ok_or(Invalid("dimension endpoints coincide in this projection"))?;
    if !length_mm.is_finite() || length_mm > 1e12 {
        return Err(Invalid("linear dimension length is outside model limits"));
    }
    let normal = u.perp() * offset_mm.signum();
    let base = normal * offset_mm.abs();
    let (p1, q1) = (p + base, q + base);
    let first = p + normal * style.extension_gap_mm;
    let second = q + normal * style.extension_gap_mm;
    let end1 = p1 + normal * style.extension_overhang_mm;
    let end2 = q1 + normal * style.extension_overhang_mm;
    let label_position = (p1 + q1) * 0.5 + normal * (style.text_height_mm * 0.75);
    if !finite_paper(&[p, q, p1, q1, first, second, end1, end2, label_position]) {
        return Err(Invalid("dimension does not fit within finite paper coordinates"));
    }
    Ok(DimensionGraphics {
        kind: DimensionKind::Linear,
        measured_value: length_mm,
        label: formatted(length_mm, style, ""),
        strokes: vec![line(first, end1), line(second, end2), line(p1, q1)],
        arrowheads: vec![arrow(p1, u, style.arrow_length_mm), arrow(q1, -u, style.arrow_length_mm)],
        text_position_mm: label_position,
        text_angle_rad: u.angle(),
    })
}

/// Radius or diameter of a genuinely circular model feature. The caller
/// supplies centre/rim and circle-plane normal from its exact model geometry.
/// An obliquely viewed circle is an ellipse and is rejected, not mislabelled.
pub fn circular_dimension(
    frame: &ViewFrame,
    centre: Vec3,
    rim: Vec3,
    normal: Vec3,
    diameter: bool,
    style: DimensionStyle,
) -> Result<DimensionGraphics, Invalid> {
    style.validate()?;
    let n = normal.normalized().ok_or(Invalid("circle normal is invalid"))?;
    let v = rim - centre;
    let radius = v.len();
    if !centre.is_finite() || !rim.is_finite() || !radius.is_finite() || !(1e-9..=1e12).contains(&radius) {
        return Err(Invalid("circle radius must be finite and positive"));
    }
    if (n.dot(frame.basis().normal()).abs() - 1.0).abs() > 1e-8 || v.dot(n).abs() > radius * 1e-8 {
        return Err(Invalid("circular dimension requires a face-on circle"));
    }
    let c = frame.to_paper(centre);
    let p = frame.to_paper(rim);
    let u = (p - c).normalized().ok_or(Invalid("circle projects to a point"))?;
    let extent = if diameter { radius * 2.0 } else { radius };
    let start = if diameter { c - (p - c) } else { c };
    let text = p + u * (style.text_height_mm + style.arrow_length_mm);
    if !finite_paper(&[c, p, start, text]) {
        return Err(Invalid("circular dimension lies outside numeric paper limits"));
    }
    let prefix = if diameter { "⌀" } else { "R" };
    Ok(DimensionGraphics {
        kind: if diameter { DimensionKind::Diameter } else { DimensionKind::Radius },
        measured_value: extent,
        label: formatted(extent, style, prefix),
        strokes: vec![line(start, p)],
        arrowheads: vec![arrow(p, -u, style.arrow_length_mm)],
        text_position_mm: text,
        text_angle_rad: u.angle(),
    })
}

/// Angular dimension between two rays, measured in the view plane. A radius
/// is a *paper* radius; the angle is independent of drawing scale.
/// Parallel/coincident rays are rejected; opposite rays define 180 degrees.
pub fn angular_dimension(
    frame: &ViewFrame,
    vertex: Vec3,
    end_a: Vec3,
    end_b: Vec3,
    radius_mm: f64,
    style: DimensionStyle,
) -> Result<DimensionGraphics, Invalid> {
    style.validate()?;
    if !vertex.is_finite() || !end_a.is_finite() || !end_b.is_finite()
        || !radius_mm.is_finite() || !(1e-6..=5000.0).contains(&radius_mm)
    {
        return Err(Invalid("invalid angular dimension points or paper radius"));
    }
    let (c, a, b) = (frame.to_paper(vertex), frame.to_paper(end_a), frame.to_paper(end_b));
    let (u, v) = (
        (a - c).normalized().ok_or(Invalid("first angular ray collapses in this view"))?,
        (b - c).normalized().ok_or(Invalid("second angular ray collapses in this view"))?,
    );
    let sweep = u.cross(v).atan2(u.dot(v));
    if sweep.abs() < 1e-9 {
        return Err(Invalid("angular dimension has zero angle"));
    }
    let start = u.angle();
    let p = c + u * radius_mm;
    let q = c + v * radius_mm;
    let mid = Vec2::from_angle(start + sweep * 0.5);
    let label = c + mid * (radius_mm + style.text_height_mm);
    if !finite_paper(&[c, p, q, label]) {
        return Err(Invalid("angular dimension lies outside numeric paper limits"));
    }
    let value = sweep.abs().to_degrees();
    Ok(DimensionGraphics {
        kind: DimensionKind::Angle,
        measured_value: value,
        label: format!("{value:.precision$}°", precision = usize::from(style.precision)),
        strokes: vec![
            line(c + u * style.extension_gap_mm, c + u * (radius_mm + style.extension_overhang_mm)),
            line(c + v * style.extension_gap_mm, c + v * (radius_mm + style.extension_overhang_mm)),
            Seg2::Arc { center: c, radius: radius_mm, start, sweep },
        ],
        arrowheads: vec![
            arrow(p, u.perp() * sweep.signum(), style.arrow_length_mm),
            arrow(q, -v.perp() * sweep.signum(), style.arrow_length_mm),
        ],
        text_position_mm: label,
        text_angle_rad: 0.0,
    })
}
