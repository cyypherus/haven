use std::rc::Rc;

use crate::brush_source::BrushSource;
use backer::Area;
use vello_svg::vello::Scene;
use vello_svg::vello::kurbo::{Affine, BezPath, Point, RoundedRect, Shape as _, Stroke};
use vello_svg::vello::peniko::{Color, Fill};

pub(crate) type PathBuilder = Rc<dyn Fn(Area) -> BezPath>;

#[derive(Clone)]
pub struct PathData {
    pub(crate) id: u64,
    pub(crate) builder: PathBuilder,
    pub(crate) fill: Option<BrushSource<()>>,
    pub(crate) stroke: Option<(BrushSource<()>, Stroke)>,
}

impl PathData {
    pub(crate) fn draw(&self, scene: &mut Scene, area: Area, scale_factor: f64) {
        let user_path = (self.builder)(area);
        let scale = Affine::scale(scale_factor);
        let path = scale * &user_path;

        if self.fill.is_none() && self.stroke.is_none() {
            scene.fill(Fill::EvenOdd, Affine::IDENTITY, Color::BLACK, None, &path)
        } else {
            if let Some(ref brush_source) = self.fill {
                let brush = brush_source.resolve(area, &());
                scene.fill(Fill::EvenOdd, scale, &brush, None, &user_path)
            }
            if let Some((ref brush_source, ref stroke_style)) = self.stroke {
                let brush = brush_source.resolve(area, &());
                scene.stroke(stroke_style, scale, &brush, None, &user_path);
            }
        }
    }
}

pub(crate) fn rect_path(corner_rounding: (f32, f32, f32, f32)) -> PathBuilder {
    Rc::new(move |area| {
        let (top_left, top_right, bottom_left, bottom_right) = corner_rounding;
        RoundedRect::from_rect(
            vello_svg::vello::kurbo::Rect::from_origin_size(
                Point::new(area.x as f64, area.y as f64),
                vello_svg::vello::kurbo::Size::new(area.width as f64, area.height as f64),
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
        vello_svg::vello::kurbo::Circle::new(
            Point::new(
                (area.x + (area.width * 0.5)) as f64,
                (area.y + (area.height * 0.5)) as f64,
            ),
            radius as f64,
        )
        .to_path(0.01)
    })
}
