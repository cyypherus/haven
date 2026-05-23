use crate::pane::{PaneState, View};
use crate::view::{Drawable, DrawableType};
use crate::{Area, Color, DEFAULT_CORNER_ROUNDING};
use backer::Layout;

#[derive(Clone)]
pub struct Shadow {
    pub(crate) id: u64,
    pub(crate) color: Color,
    pub(crate) blur: f64,
    pub(crate) spread: f64,
    pub(crate) corner_rounding: f64,
}

pub fn shadow(id: u64) -> Shadow {
    Shadow {
        id,
        color: Color::BLACK.with_alpha(0.25),
        blur: 8.,
        spread: 0.,
        corner_rounding: DEFAULT_CORNER_ROUNDING as f64,
    }
}

impl Shadow {
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn blur(mut self, blur: f64) -> Self {
        self.blur = blur.max(0.);
        self
    }

    pub fn spread(mut self, spread: f64) -> Self {
        self.spread = spread;
        self
    }

    pub fn corner_rounding(mut self, radius: f32) -> Self {
        self.corner_rounding = radius.max(0.) as f64;
        self
    }

    pub(crate) fn rect(&self, area: Area, scale_factor: f64) -> kurbo::Rect {
        let x = area.x as f64 - self.spread;
        let y = area.y as f64 - self.spread;
        let width = area.width as f64 + self.spread * 2.;
        let height = area.height as f64 + self.spread * 2.;
        kurbo::Rect::new(
            x * scale_factor,
            y * scale_factor,
            (x + width) * scale_factor,
            (y + height) * scale_factor,
        )
    }

    pub fn view<State: 'static>(self) -> Drawable<State> {
        Drawable::new(DrawableType::Shadow(self))
    }

    pub fn build<State: 'static>(
        self,
        ctx: &mut PaneState,
    ) -> Layout<'static, View<State>, PaneState> {
        self.view().build(ctx)
    }
}
