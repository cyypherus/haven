use crate::Brush;

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
