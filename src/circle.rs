use crate::app::{PaneState, View};
use crate::background_style::BrushSource;
use crate::shape::{PathData, circle_path};
use crate::view::{Drawable, DrawableType};

use backer::Layout;
use vello_svg::vello::kurbo::Stroke;

pub struct Circle {
    id: u64,
    fill: Option<BrushSource<()>>,
    stroke: Option<(BrushSource<()>, Stroke)>,
}

pub fn circle(id: u64) -> Circle {
    Circle {
        id,
        fill: None,
        stroke: None,
    }
}

impl Circle {
    pub fn fill(mut self, brush: impl Into<BrushSource<()>>) -> Self {
        self.fill = Some(brush.into());
        self
    }
    pub fn stroke(mut self, brush: impl Into<BrushSource<()>>, style: Stroke) -> Self {
        self.stroke = Some((brush.into(), style));
        self
    }
    pub(crate) fn into_path_data(self) -> PathData {
        PathData {
            id: self.id,
            builder: circle_path(),
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
    pub fn finish<State: 'static>(
        self,
        ctx: &mut PaneState,
    ) -> Layout<'static, View<State>, PaneState> {
        self.view().finish(ctx)
    }
}
