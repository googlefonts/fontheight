use ::kurbo::Shape;
use skrifa::outline::OutlinePen;

use crate::kurbo;

/// Pen to calculate the bounds of a shape.
///
/// Has the functionality of both Python's [`BoundsPen`](https://fonttools.readthedocs.io/en/latest/pens/boundsPen.html#fontTools.pens.boundsPen.BoundsPen)
/// and [`ControlBoundsPen`](https://fonttools.readthedocs.io/en/latest/pens/boundsPen.html#fontTools.pens.boundsPen.ControlBoundsPen),
/// but is powered by [`kurbo`].
// Adapted from https://github.com/googlefonts/fontations/blob/57715f39/skrifa/src/outline/mod.rs#L1159-L1184 (same license)
#[derive(Debug, Default)]
pub struct BoundsPen {
    path: kurbo::BezPath,
}

impl BoundsPen {
    /// Create a new `BoundsPen`.
    #[must_use]
    pub fn new() -> Self {
        Default::default()
    }

    /// Get out the drawn [`kurbo::BezPath`]
    #[must_use]
    pub const fn path(&self) -> &kurbo::BezPath {
        &self.path
    }

    /// Calculate the bounds of a shape.
    ///
    /// It calculates the correct bounds even when the shape contains curves
    /// that don’t have points on their extremes.
    ///
    /// This is somewhat slower to compute than the
    /// [`BoundsPen::control_bounds`].
    #[must_use]
    pub fn bounds(&self) -> kurbo::Rect {
        self.path.bounding_box()
    }

    /// Calculate the "control bounds" of a shape.
    ///
    /// This is the bounding box of all control points, so may be larger than
    /// the actual bounding box if there are curves that don’t have points on
    /// their extremes.
    ///
    /// Faster to compute than [`BoundsPen::bounds`], but not always what you
    /// want.
    #[must_use]
    pub fn control_bounds(&self) -> kurbo::Rect {
        self.path.control_box()
    }
}

impl OutlinePen for BoundsPen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.path.move_to(kurbo_point(x, y));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.path.line_to(kurbo_point(x, y));
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.path.quad_to(kurbo_point(cx0, cy0), kurbo_point(x, y));
    }

    fn curve_to(
        &mut self,
        cx0: f32,
        cy0: f32,
        cx1: f32,
        cy1: f32,
        x: f32,
        y: f32,
    ) {
        self.path.curve_to(
            kurbo_point(cx0, cy0),
            kurbo_point(cx1, cy1),
            kurbo_point(x, y),
        );
    }

    fn close(&mut self) {
        self.path.close_path();
    }
}

fn kurbo_point(x: f32, y: f32) -> kurbo::Point {
    (x as f64, y as f64).into()
}
