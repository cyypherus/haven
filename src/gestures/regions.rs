use backer::Area;
use kurbo::Rect;

fn area_rect(area: Area) -> Rect {
    Rect::new(
        area.x as f64,
        area.y as f64,
        (area.x + area.width) as f64,
        (area.y + area.height) as f64,
    )
}

fn rect_area(rect: Rect) -> Area {
    Area {
        x: rect.x0 as f32,
        y: rect.y0 as f32,
        width: rect.width() as f32,
        height: rect.height() as f32,
    }
}

pub(crate) fn valid_area(area: Area) -> Option<Area> {
    valid_rect(area_rect(area)).map(rect_area)
}

fn valid_rect(rect: Rect) -> Option<Rect> {
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

pub(crate) fn contains(area: Area, point: crate::Point) -> bool {
    point.x >= area.x
        && point.y >= area.y
        && point.x < area.x + area.width
        && point.y < area.y + area.height
}

pub(crate) fn intersect(area: Area, other: Area) -> Option<Area> {
    valid_rect(area_rect(valid_area(area)?).intersect(area_rect(valid_area(other)?))).map(rect_area)
}

pub(crate) fn subtract(area: Area, other: Area) -> Vec<Area> {
    let Some(rect) = valid_area(area).map(area_rect) else {
        return Vec::new();
    };
    let Some(other) = valid_area(other).map(area_rect) else {
        return vec![rect_area(rect)];
    };
    let cut = rect.intersect(other);
    if valid_rect(cut).is_none() {
        return vec![rect_area(rect)];
    }

    [
        Rect::new(rect.x0, rect.y0, rect.x1, cut.y0),
        Rect::new(rect.x0, cut.y1, rect.x1, rect.y1),
        Rect::new(rect.x0, cut.y0, cut.x0, cut.y1),
        Rect::new(cut.x1, cut.y0, rect.x1, cut.y1),
    ]
    .into_iter()
    .filter_map(valid_rect)
    .map(rect_area)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(left: f32, top: f32, right: f32, bottom: f32) -> Area {
        Area {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        }
    }

    #[test]
    fn subtracts_inner_rect() {
        assert_eq!(
            subtract(area(0., 0., 10., 10.), area(2., 3., 7., 8.)),
            vec![
                area(0., 0., 10., 3.),
                area(0., 8., 10., 10.),
                area(0., 3., 2., 8.),
                area(7., 3., 10., 8.),
            ]
        );
    }

    #[test]
    fn subtracting_non_overlapping_rect_keeps_source() {
        assert_eq!(
            subtract(area(0., 0., 10., 10.), area(20., 20., 30., 30.)),
            vec![area(0., 0., 10., 10.)]
        );
    }

    #[test]
    fn intersect_rejects_empty_rects() {
        assert_eq!(
            intersect(area(0., 0., 10., 10.), area(10., 0., 20., 10.)),
            None
        );
    }
}
