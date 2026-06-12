use crate::pane::{PaneElement, PaneState};

use crate::view::{Drawable, DrawableType};

use backer::Layout;
use peniko::Brush;

#[derive(Debug, Clone)]
pub struct Svg {
    pub(crate) id: u64,
    pub(crate) content: String,
    pub(crate) unlocked_aspect_ratio: bool,
    pub(crate) svg_id: Option<String>,
    pub(crate) fill: Option<Brush>,
}

pub fn svg(id: u64, content: impl AsRef<str>) -> Svg {
    Svg {
        id,
        content: content.as_ref().to_string(),
        unlocked_aspect_ratio: false,
        svg_id: None,
        fill: None,
    }
}

impl Svg {
    pub fn svg_id(mut self, svg_id: impl Into<String>) -> Self {
        self.svg_id = Some(svg_id.into());
        self
    }
    pub fn unlock_aspect_ratio(mut self) -> Self {
        self.unlocked_aspect_ratio = true;
        self
    }
    pub fn fill(mut self, fill: impl Into<Brush>) -> Self {
        self.fill = Some(fill.into());
        self
    }
    pub fn view<State: 'static>(self) -> Drawable<State> {
        Drawable::new(DrawableType::Svg(self))
    }
    pub fn finish<State: 'static>(
        self,
        ctx: &mut PaneState,
    ) -> Layout<'static, PaneElement<State>, PaneState> {
        self.view().build(ctx)
    }
}

impl Svg {
    pub(crate) fn cache_key(&self) -> u64 {
        if let Some(ref svg_id) = self.svg_id {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            self.id.hash(&mut hasher);
            svg_id.hash(&mut hasher);
            hasher.finish()
        } else {
            self.id
        }
    }
}

#[cfg(test)]
mod tests {
    use super::svg;

    #[test]
    fn cache_key_defaults_to_view_id() {
        assert_eq!(svg(7, "<svg/>").cache_key(), 7);
        assert_eq!(svg(7, "<svg><rect/></svg>").cache_key(), 7);
    }

    #[test]
    fn cache_key_includes_svg_id_when_set() {
        let key = svg(7, "<svg/>").svg_id("one").cache_key();

        assert_eq!(key, svg(7, "<svg><rect/></svg>").svg_id("one").cache_key());
        assert_ne!(key, svg(7, "<svg/>").svg_id("two").cache_key());
        assert_ne!(key, svg(8, "<svg/>").svg_id("one").cache_key());
    }
}
