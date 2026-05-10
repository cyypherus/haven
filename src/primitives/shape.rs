use std::rc::Rc;

use crate::brush_source::BrushSource;
use backer::Area;
use kurbo::{BezPath, Point, RoundedRect, Shape as _, Stroke};

pub(crate) type PathBuilder = Rc<dyn Fn(Area) -> BezPath>;

#[derive(Clone)]
pub struct PathData {
    pub(crate) id: u64,
    pub(crate) builder: PathBuilder,
    pub(crate) fill: Option<BrushSource<()>>,
    pub(crate) stroke: Option<(BrushSource<()>, Stroke)>,
}

pub(crate) fn rect_path(corner_rounding: (f32, f32, f32, f32)) -> PathBuilder {
    Rc::new(move |area| {
        let (top_left, top_right, bottom_left, bottom_right) = corner_rounding;
        RoundedRect::from_rect(
            kurbo::Rect::from_origin_size(
                Point::new(area.x as f64, area.y as f64),
                kurbo::Size::new(area.width as f64, area.height as f64),
            ),
            (
                top_left as f64,
                top_right as f64,
                bottom_left as f64,
                bottom_right as f64,
            ),
        )
        .to_path(0.01)
    })
}

pub(crate) fn circle_path() -> PathBuilder {
    Rc::new(|area| {
        let radius = f32::min(area.width, area.height) * 0.5;
        kurbo::Circle::new(
            Point::new(
                (area.x + (area.width * 0.5)) as f64,
                (area.y + (area.height * 0.5)) as f64,
            ),
            radius as f64,
        )
        .to_path(0.01)
    })
}
