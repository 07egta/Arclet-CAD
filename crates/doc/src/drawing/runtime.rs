//! Runtime views and dimensions associated with a persisted drawing.
//!
//! Versioned drawing files currently store sheets and source references but
//! not view/annotation records. These derived records are intentionally NOT
//! persisted until the drawing-format migration has been designed and tested.
//! The caller supplies newly evaluated B-rep definitions after each model edit.

use solidcraft_geom::drawing_view::dimensions::{DimensionStyle, LengthUnit};
use solidcraft_geom::drawing_view::technical::{
    LineDisplay, PaperProjection, ProjectionConvention, ThreeViews, project_brep, three_view_layout,
};
use solidcraft_geom::drawing_view::ViewFrame;
use solidcraft_geom::hlr::{self, Placement, Prepared};
use solidcraft_geom::mesh::Aabb3;

use super::{Drawing, Units};
use crate::{DocError, Result};

/// One unsaved, linked drawing-view request. The source id identifies the
/// design file; the caller must resolve/load and evaluate that design into
/// `defs` and `scene` before generating curves.
#[derive(Clone, Copy, Debug)]
pub struct LinkedView {
    pub sheet_id: u64,
    pub source_id: u64,
    pub frame: ViewFrame,
    pub display: LineDisplay,
}

#[derive(Debug, thiserror::Error)]
pub enum DrawingRenderError {
    #[error(transparent)]
    Document(#[from] DocError),
    #[error(transparent)]
    Geometry(#[from] hlr::HlrError),
}

impl Drawing {
    /// Generate a fresh view from the supplied current model, without
    /// writing derived data into the versioned drawing document.
    pub fn render_orthographic_view(
        &self,
        request: &LinkedView,
        definitions: &[Prepared],
        placements: &[Placement],
        options: &hlr::Options,
        cancelled: &dyn Fn() -> bool,
    ) -> std::result::Result<PaperProjection, DrawingRenderError> {
        self.validate()?;
        if !self.sheets.iter().any(|s| s.id == request.sheet_id) {
            return Err(DocError::Unknown("drawing sheet".into()).into());
        }
        if !self.sources.iter().any(|s| s.id == request.source_id) {
            return Err(DocError::Unknown("drawing source".into()).into());
        }
        project_brep(definitions, placements, &request.frame, request.display, options, cancelled).map_err(Into::into)
    }

    /// Arrange three principal views on an existing drawing sheet, respecting
    /// that sheet's actual size, orientation and copied border profile.
    pub fn three_view_frames(
        &self,
        sheet_id: u64,
        model_bounds: Aabb3,
        convention: ProjectionConvention,
        gap_mm: f64,
    ) -> Result<ThreeViews> {
        self.validate()?;
        let sheet = self.sheets.iter().find(|s| s.id == sheet_id).ok_or_else(|| DocError::Unknown("drawing sheet".into()))?;
        three_view_layout(model_bounds, &sheet.sheet, sheet.border.as_ref(), convention, gap_mm)
            .map_err(|e| DocError::Invalid(e.to_string()))
    }

    /// Resolve display units for dimension labels. `FollowModel` must use the
    /// specific source design's unit supplied by its reader, never paper mm;
    /// `Fixed` takes the drawing's declared unit. Geometry stays in mm.
    pub fn dimension_style_for(&self, source_unit: LengthUnit, mut style: DimensionStyle) -> Result<DimensionStyle> {
        self.validate()?;
        style.unit = match &self.units {
            Units::FollowModel => source_unit,
            Units::Fixed { unit } => LengthUnit::from_symbol(unit)
                .ok_or_else(|| DocError::Invalid("unsupported drawing length unit".into()))?,
        };
        Ok(style)
    }
}
