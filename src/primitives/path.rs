use std::rc::Rc;

use crate::brush_source::BrushSource;
use crate::pane::{PaneState, View};
use crate::primitives::shape::PathData;
use crate::view::{Drawable, DrawableType};
use backer::{Area, Layout};
use kurbo::{BezPath, Stroke};

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
    pub fn view<State: 'static>(self) -> Drawable<State> {
        Drawable::new(DrawableType::Path(Box::new(PathData {
            id: self.id,
            builder: self.builder,
            fill: self.fill,
            stroke: self.stroke,
        })))
    }
    pub fn build<State: 'static>(
        self,
        ctx: &mut PaneState,
    ) -> Layout<'static, View<State>, PaneState> {
        self.view().build(ctx)
    }
}
