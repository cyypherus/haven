use std::rc::Rc;

use crate::app::{PaneState, View};
use crate::background_style::BrushSource;
use crate::shape::PathData;
use crate::view::{Drawable, DrawableType};
use backer::{Area, Layout};
use vello_svg::vello::kurbo::{BezPath, Stroke};

pub struct Path {
    id: u64,
    builder: Rc<dyn Fn(Area) -> BezPath>,
    fill: Option<BrushSource<()>>,
    stroke: Option<(BrushSource<()>, Stroke)>,
}

pub fn path(id: u64, builder: impl Fn(Area) -> BezPath + 'static) -> Path {
    Path {
        id,
        builder: Rc::new(builder),
        fill: None,
        stroke: None,
    }
}

impl Path {
    pub fn fill(mut self, brush: impl Into<BrushSource<()>>) -> Self {
        self.fill = Some(brush.into());
        self
    }
    pub fn stroke(mut self, brush: impl Into<BrushSource<()>>, style: Stroke) -> Self {
        self.stroke = Some((brush.into(), style));
        self
    }
    pub fn view<State>(self) -> Drawable<State> {
        Drawable {
            view_type: DrawableType::Path(Box::new(PathData {
                id: self.id,
                builder: self.builder,
                fill: self.fill,
                stroke: self.stroke,
            })),
            gesture_handlers: Vec::new(),
        }
    }
    pub fn build<State: 'static>(
        self,
        ctx: &mut PaneState,
    ) -> Layout<'static, View<State>, PaneState> {
        self.view().finish(ctx)
    }
}
