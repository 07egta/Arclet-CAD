//! Derived technical drawing geometry, in paper millimetres.
//!
//! The authoritative model is a B-rep (prepared by hlr), not a saved picture. Re-run
//! project_brep after model evaluation to refresh a view. Hidden/uncertain edges retain
//! their classifications; an unsupported surface is never silently claimed visible.
//! This runtime representation does not change the versioned drawing-file format.

use crate::drawing_sheet::{DrawingBorderProfile, Sheet};
use crate::hlr::{self, HlrError, Placement, Prepared, Visibility};
use crate::mesh::Aabb3;
use crate::section::{CurveAccuracy, SectionCurve, SectionGeometry, SectionRegion, SectionWire};
use crate::{Aabb2, Seg2, Similarity2, Vec2, Vec3};

use super::{Invalid, NamedView, Scale, ViewFrame, basis_looking};

/// Projection convention. First angle places the top below and right to the left;
/// third angle places the top above and right to the right of the front.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionConvention {
    FirstAngle,
    ThirdAngle,
}

/// Paper frames for the three principal orthographic views, arranged on one sheet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThreeViews {
    pub front: ViewFrame,
    pub top: ViewFrame,
    pub right: ViewFrame,
    pub convention: ProjectionConvention,
}

/// Produce true orthographic views of a model-space box (millimetres), fitted
/// inside the sheet's drawing border at one shared, standard drafting scale.
/// No camera pitch, perspective, or pixel-DPI participates in the scale.
pub fn three_view_layout(
    bounds: Aabb3,
    paper: &Sheet,
    border: Option<&DrawingBorderProfile>,
    convention: ProjectionConvention,
    gap_mm: f64,
) -> Result<ThreeViews, Invalid> {
    let finite = [bounds.min.x, bounds.min.y, bounds.min.z, bounds.max.x, bounds.max.y, bounds.max.z]
        .into_iter()
        .all(|n| n.is_finite() && n.abs() <= 1e12);
    if !finite || bounds.is_empty() {
        return Err(Invalid("finite, nonempty model bounds are required"));
    }
    if !gap_mm.is_finite() || !(0.0..=5000.0).contains(&gap_mm) {
        return Err(Invalid("three-view gap must be between 0 and 5000 mm"));
    }
    let sheet_box = match border {
        Some(profile) => profile.layout(paper).map_err(|e| Invalid(e.0))?.drawing_border,
        None => Aabb2 { min: Vec2::ZERO, max: paper.size_mm().map_err(|e| Invalid(e.0))? },
    };
    let size = bounds.size();
    let total_model = Vec2::new(size.x + size.y, size.z + size.y);
    let room = sheet_box.size() - Vec2::new(gap_mm, gap_mm);
    let scale = Scale::fit(total_model, room).ok_or(Invalid("three-view layout does not fit the sheet at a supported scale"))?;
    let factor = scale.factor();
    let occupied = total_model * factor + Vec2::new(gap_mm, gap_mm);
    let free = sheet_box.size() - occupied;
    if free.x < -1e-8 || free.y < -1e-8 {
        return Err(Invalid("three-view layout exceeds the drawing border"));
    }
    let origin = sheet_box.min + Vec2::new(free.x.max(0.0), free.y.max(0.0)) * 0.5;
    let front_left = match convention {
        ProjectionConvention::FirstAngle => origin.x + size.y * factor + gap_mm,
        ProjectionConvention::ThirdAngle => origin.x,
    };
    let front_bottom = match convention {
        ProjectionConvention::FirstAngle => origin.y + size.y * factor + gap_mm,
        ProjectionConvention::ThirdAngle => origin.y,
    };
    let front_corner = Vec2::new(front_left, front_bottom);
    let top_corner = Vec2::new(
        front_left,
        match convention {
            ProjectionConvention::FirstAngle => origin.y,
            ProjectionConvention::ThirdAngle => front_bottom + size.z * factor + gap_mm,
        },
    );
    let right_corner = Vec2::new(
        match convention {
            ProjectionConvention::FirstAngle => origin.x,
            ProjectionConvention::ThirdAngle => front_left + size.x * factor + gap_mm,
        },
        front_bottom,
    );
    fn placed(bounds: Aabb3, named: NamedView, scale: Scale, low_on_paper: Vec2) -> Result<ViewFrame, Invalid> {
        let basis = named.basis();
        let mut box2 = Aabb2::EMPTY;
        for i in 0..8 {
            let p = Vec3::new(
                if i & 1 == 0 { bounds.min.x } else { bounds.max.x },
                if i & 2 == 0 { bounds.min.y } else { bounds.max.y },
                if i & 4 == 0 { bounds.min.z } else { bounds.max.z },
            );
            box2.add(basis.to_local(p));
        }
        let centre = (box2.min + box2.max) * 0.5;
        let paper_centre = low_on_paper + box2.size() * scale.factor() * 0.5;
        ViewFrame::new(basis, scale, centre, paper_centre)
    }
    Ok(ThreeViews {
        front: placed(bounds, NamedView::Front, scale, front_corner)?,
        top: placed(bounds, NamedView::Top, scale, top_corner)?,
        right: placed(bounds, NamedView::Right, scale, right_corner)?,
        convention,
    })
}

/// Reframe an existing orthographic view around a local (model-mm) detail
/// centre. Magnification changes drawing scale rather than measured geometry.
pub fn detail_frame(parent: &ViewFrame, centre: Vec2, position_mm: Vec2, magnification: f64) -> Result<ViewFrame, Invalid> {
    if !magnification.is_finite() || magnification <= 0.0 {
        return Err(Invalid("detail magnification must be finite and positive"));
    }
    let scale = Scale::new(parent.scale().factor() * magnification, 1.0)?;
    ViewFrame::new(parent.basis(), scale, centre, position_mm)
}

/// Perspective for illustrations only: there is intentionally no drawing scale
/// or dimension API here because perspective distances are depth-dependent.
/// Focal length and coordinates are both in millimetres.
#[derive(Clone, Copy, Debug)]
pub struct PerspectiveFrame {
    eye: Vec3,
    x: Vec3,
    y: Vec3,
    forward: Vec3,
    focal_mm: f64,
    centre_mm: Vec2,
}

impl PerspectiveFrame {
    pub fn new(eye: Vec3, target: Vec3, up: Vec3, focal_mm: f64, centre_mm: Vec2) -> Result<Self, Invalid> {
        if !(0.0..=5000.0).contains(&focal_mm) || !focal_mm.is_finite() {
            return Err(Invalid("focal length must be positive and at most 5000 mm"));
        }
        if !centre_mm.is_finite() || centre_mm.x.abs() > 5000.0 || centre_mm.y.abs() > 5000.0 {
            return Err(Invalid("perspective sheet centre is not finite or outside drawing limits"));
        }
        let basis = basis_looking(target - eye, up, eye)?;
        Ok(Self { eye, x: basis.x, y: basis.y, forward: -basis.normal(), focal_mm, centre_mm })
    }

    /// None for a point on/behind the eye plane, or unrepresentable coordinates.
    pub fn to_paper(&self, point: Vec3) -> Option<Vec2> {
        let ray = point - self.eye;
        let depth = ray.dot(self.forward);
        if !depth.is_finite() || depth <= 1e-9 || !point.is_finite() {
            return None;
        }
        let result = self.centre_mm + Vec2::new(ray.dot(self.x), ray.dot(self.y)) * (self.focal_mm / depth);
        result.is_finite().then_some(result)
    }
}

/// Which classified B-rep lines to include in the derived paper curves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineDisplay {
    Visible,
    VisibleAndHidden,
    AllClassified,
}

/// One exact or accuracy-labelled HLR curve on paper.
#[derive(Clone, Debug)]
pub struct PaperCurve {
    pub geometry: Seg2,
    pub accuracy: CurveAccuracy,
    pub visibility: Visibility,
    pub placement: usize,
    pub source: hlr::Source,
}

/// A derived view with HLR warnings retained for the consuming drawing editor.
#[derive(Debug)]
pub struct PaperProjection {
    pub curves: Vec<PaperCurve>,
    pub uncertain_pieces: usize,
    pub unsupported: Vec<hlr::Unsupported>,
    pub stats: hlr::Stats,
}

/// Recompute an orthographic view from the latest evaluated B-reps and placements.
/// A new call, rather than reusing an old raster, refreshes a view after edits.
/// HLR returns global x/y dot products; an arbitrary frame origin must be
/// subtracted before mapping to the sheet.
pub fn project_brep(
    defs: &[Prepared],
    scene: &[Placement],
    frame: &ViewFrame,
    display: LineDisplay,
    opts: &hlr::Options,
    cancel: &dyn Fn() -> bool,
) -> Result<PaperProjection, HlrError> {
    let basis = frame.basis();
    let view = hlr::View { x: basis.x, y: basis.y, d: -basis.normal() };
    let drawing = hlr::hidden_lines(defs, scene, &view, opts, cancel)?;
    // The source coordinates are p.dot(basis.x/y), while ViewFrame
    // expects (p - basis.origin).dot(basis.x/y).
    let offset = Vec2::new(basis.origin.dot(basis.x), basis.origin.dot(basis.y));
    let map = Similarity2::new(frame.scale().factor(), 0.0, frame.paper_map().apply(-offset))
        .ok_or(HlrError::Invalid("view-to-paper mapping"))?;
    let mut curves = Vec::new();
    let mut uncertain_pieces = 0;
    for line in drawing.lines {
        for piece in line.pieces {
            if piece.covered {
                continue;
            }
            if piece.visibility == Visibility::Uncertain {
                uncertain_pieces += 1;
            }
            let include = match display {
                LineDisplay::Visible => piece.visibility == Visibility::Visible,
                LineDisplay::VisibleAndHidden => piece.visibility != Visibility::Uncertain,
                LineDisplay::AllClassified => true,
            };
            if include {
                for segment in piece.geometry {
                    curves.push(PaperCurve {
                        geometry: segment.similar(&map),
                        accuracy: frame.accuracy_on_paper(piece.accuracy),
                        visibility: piece.visibility,
                        placement: line.placement,
                        source: line.source,
                    });
                }
            }
        }
    }
    Ok(PaperProjection { curves, uncertain_pieces, unsupported: drawing.unsupported, stats: drawing.stats })
}

/// A validated section producer's curves, wires and material regions, on paper.
/// This consumes section geometry from the model kernel; it does not infer
/// solid material from unconnected edges, or mislabel sampled outlines as exact.
#[derive(Clone, Debug)]
pub struct PaperSection {
    pub curves: Vec<SectionCurve>,
    pub wires: Vec<SectionWire>,
    pub regions: Vec<SectionRegion>,
    pub contacts: Vec<Vec2>,
}

/// Place section curves on an orthographic view of the same oriented plane.
/// Reflected/reversed normal and oblique views require a general affine
/// conic transform and are rejected rather than distorting arcs.
pub fn project_section(section: &SectionGeometry, frame: &ViewFrame, closure_tol_mm: f64) -> Result<PaperSection, Invalid> {
    section.validate_structure(closure_tol_mm).map_err(|_| Invalid("invalid section topology or geometry"))?;
    let source = section.plane;
    let target = frame.basis();
    if source.normal().dot(target.normal()) < 1.0 - 1e-8 {
        return Err(Invalid("section plane must face the orthographic view"));
    }
    let angle = source.x.dot(target.y).atan2(source.x.dot(target.x));
    let shift = frame.to_paper(source.origin);
    let map = Similarity2::new(frame.scale().factor(), angle, shift).ok_or(Invalid("section-to-paper mapping"))?;
    Ok(PaperSection {
        curves: section.curves.iter().map(|c| SectionCurve {
            geometry: c.geometry.similar(&map),
            accuracy: frame.accuracy_on_paper(c.accuracy),
        }).collect(),
        wires: section.wires.clone(),
        regions: section.regions.clone(),
        contacts: section.contacts.iter().map(|&p| map.apply(p)).collect(),
    })
}
