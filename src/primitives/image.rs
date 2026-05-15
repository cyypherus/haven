use crate::app::{PaneState, View};

use crate::DEFAULT_CORNER_ROUNDING;
use crate::view::{Drawable, DrawableType};

use backer::Layout;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Image {
    pub(crate) id: u64,
    pub(crate) source: ImageSource,
    pub(crate) unlocked_aspect_ratio: bool,
    pub(crate) image_id: Option<String>,
    pub(crate) corner_rounding: f32,
}

#[derive(Debug, Clone)]
pub enum ImageSource {
    Path(String),
    Bytes(Arc<Vec<u8>>),
    Buffer(u32, u32, Arc<Vec<u8>>),
}

pub fn image(id: u64, source: impl Into<ImageSource>) -> Image {
    Image {
        id,
        source: source.into(),
        unlocked_aspect_ratio: false,
        image_id: None,
        corner_rounding: DEFAULT_CORNER_ROUNDING,
    }
}

pub fn image_from_path(id: u64, path: impl AsRef<str>) -> Image {
    image(id, ImageSource::Path(path.as_ref().to_string()))
}

pub fn image_from_bytes(id: u64, bytes: Arc<Vec<u8>>) -> Image {
    image(id, ImageSource::Bytes(bytes))
}

impl From<String> for ImageSource {
    fn from(path: String) -> Self {
        ImageSource::Path(path)
    }
}

impl From<&str> for ImageSource {
    fn from(path: &str) -> Self {
        ImageSource::Path(path.to_string())
    }
}

impl From<Vec<u8>> for ImageSource {
    fn from(bytes: Vec<u8>) -> Self {
        ImageSource::Bytes(Arc::new(bytes))
    }
}

impl From<Arc<Vec<u8>>> for ImageSource {
    fn from(bytes: Arc<Vec<u8>>) -> Self {
        ImageSource::Bytes(bytes)
    }
}

impl Image {
    /// Used to differentiate images when a view with the same id() is passed different image data.
    pub fn image_id(mut self, image_id: impl Into<String>) -> Self {
        self.image_id = Some(image_id.into());
        self
    }

    pub fn corner_rounding(mut self, radius: f32) -> Self {
        self.corner_rounding = radius;
        self
    }

    pub fn view<State>(self) -> Drawable<State> {
        Drawable {
            view_type: DrawableType::Image(self),
            gesture_handlers: Vec::new(),
        }
    }

    pub fn finish<State: 'static>(
        self,
        ctx: &mut PaneState,
    ) -> Layout<'static, View<State>, PaneState> {
        self.view().build(ctx)
    }
}

impl Image {
    pub(crate) fn cache_key(&self) -> u64 {
        if let Some(ref image_id) = self.image_id {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            self.id.hash(&mut hasher);
            image_id.hash(&mut hasher);
            hasher.finish()
        } else {
            self.id
        }
    }
}
