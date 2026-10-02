use backer::Area;
use kurbo::{Affine, Point, Rect};

pub(crate) fn area_rect(area: Area) -> Rect {
    Rect::new(
        area.x as f64,
        area.y as f64,
        (area.x + area.width) as f64,
        (area.y + area.height) as f64,
    )
}

#[derive(Debug, Clone)]
pub(crate) struct HitRegion {
    rect: Rect,
    pub(crate) transform: Affine,
    pub(crate) clips: Vec<HitRegion>,
    pub(crate) exclusions: Vec<HitRegion>,
}

impl HitRegion {
    pub(crate) fn new(rect: Rect) -> Self {
        let rect = rect.abs();
        Self {
            rect: if rect.is_finite() { rect } else { Rect::ZERO },
            transform: Affine::IDENTITY,
            clips: Vec::new(),
            exclusions: Vec::new(),
        }
    }

    pub(crate) fn contains(&self, point: Point) -> bool {
        self.rect.contains(self.transform.inverse() * point)
            && self.clips.iter().all(|clip| clip.contains(point))
            && !self.exclusions.iter().any(|cut| cut.contains(point))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotated_regions_preserve_clips_and_exclusions() {
        let mut region = HitRegion::new(Rect::new(0., 0., 100., 20.));
        region
            .clips
            .push(HitRegion::new(Rect::new(0., 0., 60., 20.)));
        region
            .exclusions
            .push(HitRegion::new(Rect::new(10., 0., 20., 20.)));
        let rotation = Affine::rotate(std::f64::consts::FRAC_PI_4);
        region.transform = rotation;
        region.clips[0].transform = rotation;
        region.exclusions[0].transform = rotation;
        assert!(region.contains(rotation * Point::new(5., 10.)));
        assert!(!region.contains(rotation * Point::new(15., 10.)));
        assert!(!region.contains(rotation * Point::new(80., 10.)));
        assert!(!region.contains(Point::new(1., 30.)));
    }
}
