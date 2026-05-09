use crate::{Area, Brush, Point};

pub(crate) fn area_contains(area: &Area, point: Point) -> bool {
    let x = point.x;
    let y = point.y;
    x > area.x as f64
        && y > area.y as f64
        && y < area.y as f64 + area.height as f64
        && x < area.x as f64 + area.width as f64
}

pub(crate) fn adjust_brush(brush: &Brush, depressed: bool, hovered: bool) -> Brush {
    match brush {
        Brush::Solid(color) => {
            let adjusted = match (depressed, hovered) {
                (true, _) => color.map_lightness(|l| l - 0.1),
                (false, true) => color.map_lightness(|l| l + 0.1),
                (false, false) => *color,
            };
            Brush::Solid(adjusted)
        }
        other => other.clone(),
    }
}
