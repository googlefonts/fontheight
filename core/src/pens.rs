use skrifa::outline::OutlinePen;

// Taken from https://github.com/googlefonts/fontations/blob/57715f39/skrifa/src/outline/mod.rs#L1159-L1184 (same license)
#[derive(Debug, Default)]
pub(crate) struct BezierPen {
    pub(crate) path: kurbo::BezPath,
}

fn kurbo_point(x: f32, y: f32) -> kurbo::Point {
    (x as f64, y as f64).into()
}

impl OutlinePen for BezierPen {
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
