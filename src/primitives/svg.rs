use crate::app::{PaneState, View};

use crate::view::{Drawable, DrawableType};

use backer::Layout;
use peniko::Brush;

#[derive(Debug, Clone)]
pub struct Svg {
    pub(crate) id: u64,
    pub(crate) content: String,
    pub(crate) unlocked_aspect_ratio: bool,
    pub(crate) fill: Option<Brush>,
}

pub fn svg(id: u64, content: impl AsRef<str>) -> Svg {
    Svg {
        id,
        content: content.as_ref().to_string(),
        unlocked_aspect_ratio: false,
        fill: None,
    }
}

impl Svg {
    pub fn unlock_aspect_ratio(mut self) -> Self {
        self.unlocked_aspect_ratio = true;
        self
    }
    pub fn fill(mut self, fill: impl Into<Brush>) -> Self {
        self.fill = Some(fill.into());
        self
    }
    pub fn view<State>(self) -> Drawable<State> {
        Drawable {
            view_type: DrawableType::Svg(self),
            gesture_handlers: Vec::new(),
        }
    }
    pub fn finish<State: 'static>(
        self,
        ctx: &mut PaneState,
    ) -> Layout<'static, View<State>, PaneState> {
        self.view().finish(ctx)
    }
}
