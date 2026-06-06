use backer::Area;
use kurbo::Rect;

pub(crate) fn area_rect(area: Area) -> Rect {
    Rect::new(
        area.x as f64,
        area.y as f64,
        (area.x + area.width) as f64,
        (area.y + area.height) as f64,
    )
}

pub(crate) fn valid_rect(rect: Rect) -> Option<Rect> {
    let rect = rect.abs();
    if rect.x0.is_finite()
        && rect.y0.is_finite()
        && rect.x1.is_finite()
        && rect.y1.is_finite()
        && rect.width() > 0.
        && rect.height() > 0.
    {
        Some(rect)
    } else {
        None
    }
}

pub(crate) fn intersect(rect: Rect, other: Rect) -> Option<Rect> {
    valid_rect(valid_rect(rect)?.intersect(valid_rect(other)?))
}

pub(crate) fn subtract(rect: Rect, other: Rect) -> Vec<Rect> {
    let Some(rect) = valid_rect(rect) else {
        return Vec::new();
    };
    let Some(other) = valid_rect(other) else {
        return vec![rect];
    };
    let cut = rect.intersect(other);
    if valid_rect(cut).is_none() {
        return vec![rect];
    }

    [
        Rect::new(rect.x0, rect.y0, rect.x1, cut.y0),
        Rect::new(rect.x0, cut.y1, rect.x1, rect.y1),
        Rect::new(rect.x0, cut.y0, cut.x0, cut.y1),
        Rect::new(cut.x1, cut.y0, rect.x1, cut.y1),
    ]
    .into_iter()
    .filter_map(valid_rect)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(left: f64, top: f64, right: f64, bottom: f64) -> Rect {
        Rect::new(left, top, right, bottom)
    }

    #[test]
    fn subtracts_inner_rect() {
        assert_eq!(
            subtract(rect(0., 0., 10., 10.), rect(2., 3., 7., 8.)),
            vec![
                rect(0., 0., 10., 3.),
                rect(0., 8., 10., 10.),
                rect(0., 3., 2., 8.),
                rect(7., 3., 10., 8.),
            ]
        );
    }

    #[test]
    fn subtracting_non_overlapping_rect_keeps_source() {
        assert_eq!(
            subtract(rect(0., 0., 10., 10.), rect(20., 20., 30., 30.)),
            vec![rect(0., 0., 10., 10.)]
        );
    }

    #[test]
    fn intersect_rejects_empty_rects() {
        assert_eq!(
            intersect(rect(0., 0., 10., 10.), rect(10., 0., 20., 10.)),
            None
        );
    }
}
