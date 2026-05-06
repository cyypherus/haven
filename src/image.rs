use crate::app::{RootCtx, RootState, View};

use crate::DEFAULT_CORNER_ROUNDING;
use crate::view::{Drawable, DrawableType};

use backer::{Area, Layout};
use image::{DynamicImage, ImageBuffer, Rgba};
use std::sync::Arc;
use vello_svg::vello::kurbo::{Affine, Point, RoundedRect, Size, Vec2};
use vello_svg::vello::peniko::{Fill, Mix};
use vello_svg::vello::{Scene, peniko};

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

    pub fn finish<State: 'static>(self, ctx: &mut RootCtx) -> Layout<'static, View<State>, RootCtx> {
        self.view().finish(ctx)
    }
}

impl Image {
    pub(crate) fn draw(&mut self, area: Area, scene: &mut Scene, app: &mut RootState) {
        let cache_key = if let Some(ref image_id) = self.image_id {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            self.id.hash(&mut hasher);
            image_id.hash(&mut hasher);
            hasher.finish()
        } else {
            self.id
        };

        if !app.image_scenes.contains_key(&cache_key) {
            let peniko_image = match self.load_image() {
                Ok(img) => img,
                Err(err) => {
                    eprintln!("Loading image failed: {err}");
                    app.image_scenes.insert(cache_key, (Scene::new(), 0., 0.));
                    return;
                }
            };

            let width = peniko_image.width as f32;
            let height = peniko_image.height as f32;

            let mut image_scene = Scene::new();
            image_scene.draw_image(&peniko_image, Affine::IDENTITY);

            app.image_scenes
                .insert(cache_key, (image_scene, width, height));
        }

        let RootState { image_scenes, .. } = app;

        if let Some((image_scene, width, height)) = image_scenes.get(&cache_key) {
            let width = *width as f64;
            let height = *height as f64;
            let area_x = area.x as f64 * app.app_context.scale_factor;
            let area_y = area.y as f64 * app.app_context.scale_factor;
            let area_width = area.width as f64 * app.app_context.scale_factor;
            let area_height = area.height as f64 * app.app_context.scale_factor;
            let mut scale = 1.;

            let transform = if self.unlocked_aspect_ratio {
                Affine::IDENTITY
                    .then_scale_non_uniform(area_width / width, area_height / height)
                    .then_translate(Vec2::new(area_x, area_y))
            } else {
                scale = (area_width / width).min(area_height / height);
                let dx = area_x + (area_width - width * scale) / 2.0;
                let dy = area_y + (area_height - height * scale) / 2.0;
                Affine::IDENTITY
                    .then_scale(scale)
                    .then_translate(Vec2::new(dx, dy))
            };

            scene.push_layer(
                Fill::NonZero,
                Mix::Normal,
                1.,
                transform,
                &RoundedRect::from_origin_size(
                    Point::ZERO,
                    Size::new(width, height),
                    self.corner_rounding as f64 / scale,
                ),
            );
            scene.append(image_scene, Some(transform));
            scene.pop_layer();
        }
    }

    fn load_image(&self) -> Result<peniko::ImageData, Box<dyn std::error::Error>> {
        #[derive(Debug)]
        pub enum ImageError {
            InvalidBuffer(String),
        }

        impl std::fmt::Display for ImageError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    ImageError::InvalidBuffer(msg) => write!(f, "Invalid image buffer: {}", msg),
                }
            }
        }

        impl std::error::Error for ImageError {}

        let img = match &self.source {
            ImageSource::Path(path) => image::load_from_memory(&std::fs::read(path)?)?,
            ImageSource::Bytes(bytes) => image::load_from_memory(&bytes.as_ref().clone())?,
            ImageSource::Buffer(width, height, container) => DynamicImage::ImageRgba8(
                ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(
                    *width,
                    *height,
                    container.as_ref().clone(),
                )
                .ok_or_else(|| {
                    ImageError::InvalidBuffer(format!(
                        "Buffer size mismatch for {}x{} image",
                        width, height
                    ))
                })?,
            ),
        };

        let rgba_img = img.to_rgba8();
        let (width, height) = rgba_img.dimensions();

        let blob = peniko::Blob::new(Arc::new(rgba_img.into_raw()));

        Ok(peniko::ImageData {
            data: blob,
            format: peniko::ImageFormat::Rgba8,
            alpha_type: peniko::ImageAlphaType::Alpha,
            width,
            height,
        })
    }
}
