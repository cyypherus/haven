use crate::DEFAULT_CORNER_ROUNDING;
use crate::app::{RootCtx, View};
use crate::background_style::BrushSource;
use crate::shape::{PathData, rect_path};
use crate::view::{Drawable, DrawableType};
use backer::Layout;
use vello_svg::vello::kurbo::Stroke;
use vello_svg::vello::peniko::Brush;
use vello_svg::vello::peniko::color::palette::css::BLACK;

pub struct Rect {
    id: u64,
    fill: Option<BrushSource<()>>,
    stroke: Option<(BrushSource<()>, Stroke)>,
    corner_rounding: (f32, f32, f32, f32),
}

pub fn rect(id: u64) -> Rect {
    Rect {
        id,
        fill: Some(BrushSource::Static(Brush::Solid(BLACK))),
        stroke: None,
        corner_rounding: (
            DEFAULT_CORNER_ROUNDING,
            DEFAULT_CORNER_ROUNDING,
            DEFAULT_CORNER_ROUNDING,
            DEFAULT_CORNER_ROUNDING,
        ),
    }
}

impl Rect {
    pub fn fill(mut self, brush: impl Into<BrushSource<()>>) -> Self {
        self.fill = Some(brush.into());
        self
    }
    pub fn corner_rounding(mut self, radius: f32) -> Self {
        self.corner_rounding = (radius, radius, radius, radius);
        self
    }
    pub fn corner_rounding_individual(
        mut self,
        top_left: f32,
        top_right: f32,
        bottom_right: f32,
        bottom_left: f32,
    ) -> Self {
        self.corner_rounding = (top_left, top_right, bottom_right, bottom_left);
        self
    }
    pub fn stroke(mut self, brush: impl Into<BrushSource<()>>, style: impl Into<Stroke>) -> Self {
        self.stroke = Some((brush.into(), style.into()));
        self
    }
    pub(crate) fn into_path_data(self) -> PathData {
        PathData {
            id: self.id,
            builder: rect_path(self.corner_rounding),
            fill: self.fill,
            stroke: self.stroke,
        }
    }
    pub fn view<State>(self) -> Drawable<State> {
        Drawable {
            view_type: DrawableType::Path(Box::new(self.into_path_data())),
            gesture_handlers: Vec::new(),
        }
    }
    pub fn build<State: 'static>(self, ctx: &mut RootCtx) -> Layout<'static, View<State>, RootCtx> {
        self.view().finish(ctx)
    }
}
