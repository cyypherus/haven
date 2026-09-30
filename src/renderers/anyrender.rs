use crate::Area;
use crate::draw_layout::draw_layout;
use crate::primitives::ImageSource;
#[cfg(feature = "platform-winit")]
use crate::render::FrameOutput;
use crate::render::{Frame, RenderItem, TextRenderLayout};
#[cfg(feature = "platform-winit")]
use anyrender::WindowRenderer;
use anyrender::{PaintScene, Scene};
use image::{DynamicImage, ImageBuffer, Rgba};
use kurbo::{Affine, Point, Rect, RoundedRect, Size, Vec2};
use peniko::{self, Brush, BrushRef, Compose, Fill, Mix};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Default)]
pub struct FramePainter {
    svg_scenes: HashMap<u64, (String, Scene, f32, f32)>,
    image_data: HashMap<u64, (peniko::ImageData, f32, f32)>,
}

impl FramePainter {
    pub fn paint<S: PaintScene>(&mut self, frame: &Frame, scene: &mut S) {
        scene.reset();
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            frame.base_color,
            None,
            &Rect::new(0., 0., frame.width as f64, frame.height as f64),
        );
        render_frame(&mut self.svg_scenes, &mut self.image_data, frame, scene);
    }
}

#[cfg(feature = "platform-winit")]
pub struct Renderer<R: WindowRenderer> {
    window_renderer: R,
    window: Arc<winit::window::Window>,
    painter: FramePainter,
}

#[cfg(feature = "platform-winit")]
impl<R: WindowRenderer> Renderer<R> {
    pub fn new(
        mut window_renderer: R,
        window: Arc<winit::window::Window>,
        width: u32,
        height: u32,
    ) -> Self {
        window_renderer.resume(window.clone(), width, height, || {});
        window_renderer.complete_resume();
        Self {
            window_renderer,
            window,
            painter: FramePainter::default(),
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.window_renderer.complete_resume();
        self.window_renderer.set_size(width, height);
    }
}

#[cfg(feature = "platform-winit")]
impl<R: WindowRenderer> FrameOutput for Renderer<R> {
    type Output = ();

    fn render(&mut self, frame: &Frame) {
        if !self.window_renderer.complete_resume() {
            return;
        }

        let painter = &mut self.painter;
        let window = &self.window;
        self.window_renderer.render(|scene| {
            painter.paint(frame, scene);
            window.pre_present_notify();
        });
    }
}

fn render_frame<S: PaintScene>(
    svg_scenes: &mut HashMap<u64, (String, Scene, f32, f32)>,
    image_data: &mut HashMap<u64, (peniko::ImageData, f32, f32)>,
    frame: &Frame,
    scene: &mut S,
) {
    for item in &frame.items {
        match item {
            RenderItem::PushLayer { path, blend, alpha } => {
                scene.push_layer(
                    *blend,
                    *alpha,
                    Affine::scale(frame.scale_factor),
                    path,
                    None,
                    None,
                );
            }
            RenderItem::PopLayer => scene.pop_layer(),
            RenderItem::Text(text) => draw_text(scene, text),
            RenderItem::Layout { layout, transform } => draw_layout(*transform, layout, scene),
            RenderItem::Path {
                path, fill, stroke, ..
            } => draw_path(
                scene,
                path,
                fill.as_ref(),
                stroke.as_ref(),
                frame.scale_factor,
            ),
            RenderItem::Svg {
                cache_key,
                content,
                area,
                unlocked_aspect_ratio,
                fill,
            } => draw_svg(
                scene,
                svg_scenes,
                frame.scale_factor,
                *cache_key,
                content,
                *area,
                *unlocked_aspect_ratio,
                fill.as_ref(),
            ),
            RenderItem::Image {
                cache_key,
                source,
                area,
                unlocked_aspect_ratio,
                corner_rounding,
            } => draw_image(
                scene,
                image_data,
                frame.scale_factor,
                *cache_key,
                source,
                *area,
                *unlocked_aspect_ratio,
                *corner_rounding,
            ),
            RenderItem::Shadow {
                rect,
                color,
                blur,
                corner_rounding,
            } => {
                scene.draw_box_shadow(Affine::IDENTITY, *rect, *color, *corner_rounding, *blur);
            }
        }
    }
}

fn draw_text<S: PaintScene>(scene: &mut S, text: &TextRenderLayout) {
    for (rect, brush) in &text.backgrounds {
        scene.fill(
            Fill::NonZero,
            text.transform,
            BrushRef::from(brush),
            None,
            rect,
        );
    }
    draw_layout(text.transform, &text.layout, scene);
}

fn draw_image<S: PaintScene>(
    scene: &mut S,
    image_data: &mut HashMap<u64, (peniko::ImageData, f32, f32)>,
    scale_factor: f64,
    cache_key: u64,
    source: &ImageSource,
    area: Area,
    unlocked_aspect_ratio: bool,
    corner_rounding: f32,
) {
    if !image_data.contains_key(&cache_key) {
        let image = match load_image(source) {
            Ok(img) => img,
            Err(err) => {
                eprintln!("Loading image failed: {err}");
                image_data.insert(
                    cache_key,
                    (
                        peniko::ImageData {
                            data: peniko::Blob::new(Arc::new(vec![0; 4])),
                            format: peniko::ImageFormat::Rgba8,
                            alpha_type: peniko::ImageAlphaType::Alpha,
                            width: 1,
                            height: 1,
                        },
                        0.,
                        0.,
                    ),
                );
                return;
            }
        };

        let width = image.width as f32;
        let height = image.height as f32;
        image_data.insert(cache_key, (image, width, height));
    }

    if let Some((image, width, height)) = image_data.get(&cache_key) {
        let width = *width as f64;
        let height = *height as f64;
        let area_x = area.x as f64 * scale_factor;
        let area_y = area.y as f64 * scale_factor;
        let area_width = area.width as f64 * scale_factor;
        let area_height = area.height as f64 * scale_factor;
        let mut scale = 1.;

        let transform = if unlocked_aspect_ratio {
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
            Mix::Normal,
            1.,
            transform,
            &RoundedRect::from_origin_size(
                Point::ZERO,
                Size::new(width, height),
                corner_rounding as f64 / scale,
            ),
            None,
            None,
        );
        scene.draw_image(image.into(), transform);
        scene.pop_layer();
    }
}

fn draw_svg<S: PaintScene>(
    scene: &mut S,
    svg_scenes: &mut HashMap<u64, (String, Scene, f32, f32)>,
    scale_factor: f64,
    cache_key: u64,
    content: &str,
    area: Area,
    unlocked_aspect_ratio: bool,
    fill: Option<&Brush>,
) {
    let (_, svg_scene, width, height) = cached_svg_scene(svg_scenes, cache_key, content);
    let width = *width as f64;
    let height = *height as f64;
    let area_x = area.x as f64 * scale_factor;
    let area_y = area.y as f64 * scale_factor;
    let area_width = area.width as f64 * scale_factor;
    let area_height = area.height as f64 * scale_factor;
    if fill.is_some() {
        scene.push_layer(
            peniko::BlendMode {
                mix: Mix::Normal,
                compose: Compose::SrcOver,
            },
            1.0,
            Affine::IDENTITY,
            &Rect::from_origin_size(
                Point::new(area_x, area_y),
                Size::new(area_width, area_height),
            ),
            None,
            None,
        );
    }
    scene.append_scene(
        svg_scene.clone(),
        if unlocked_aspect_ratio {
            Affine::IDENTITY
                .then_scale_non_uniform(area_width / width, area_height / height)
                .then_translate(Vec2::new(area_x, area_y))
        } else {
            let scale = (area_width / width).min(area_height / height);
            let dx = area_x + (area_width - width * scale) / 2.0;
            let dy = area_y + (area_height - height * scale) / 2.0;
            Affine::IDENTITY
                .then_scale(scale)
                .then_translate(Vec2::new(dx, dy))
        },
    );
    if let Some(fill) = fill {
        scene.push_layer(
            peniko::BlendMode {
                mix: Mix::Normal,
                compose: Compose::SrcIn,
            },
            1.0,
            Affine::IDENTITY,
            &Rect::from_origin_size(
                Point::new(area_x, area_y),
                Size::new(area_width, area_height),
            ),
            None,
            None,
        );

        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            BrushRef::from(fill),
            None,
            &Rect::from_origin_size(
                Point::new(area_x, area_y),
                Size::new(area_width, area_height),
            ),
        );
        scene.pop_layer();
        scene.pop_layer();
    }
}

fn cached_svg_scene<'a>(
    svg_scenes: &'a mut HashMap<u64, (String, Scene, f32, f32)>,
    cache_key: u64,
    content: &str,
) -> &'a (String, Scene, f32, f32) {
    let needs_update = svg_scenes
        .get(&cache_key)
        .is_none_or(|(cached_content, _, _, _)| cached_content != content);

    if needs_update {
        let cached_svg = match anyrender_svg::usvg::Tree::from_data(
            content.as_bytes(),
            &anyrender_svg::usvg::Options::default(),
        ) {
            Err(err) => {
                eprintln!("Loading svg failed: {err}");
                (content.to_string(), Scene::new(), 0., 0.)
            }
            Ok(svg) => {
                let mut svg_scene = Scene::new();
                anyrender_svg::render_svg_tree(&mut svg_scene, &svg, Affine::IDENTITY);
                let size = svg.size();
                (content.to_string(), svg_scene, size.width(), size.height())
            }
        };

        match svg_scenes.entry(cache_key) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                *entry.get_mut() = cached_svg;
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(cached_svg);
            }
        }
    }

    svg_scenes.get(&cache_key).expect("cached svg")
}

fn draw_path<S: PaintScene>(
    scene: &mut S,
    user_path: &kurbo::BezPath,
    fill: Option<&Brush>,
    stroke: Option<&(Brush, kurbo::Stroke)>,
    scale_factor: f64,
) {
    let scale = Affine::scale(scale_factor);
    if let Some(brush) = fill {
        scene.fill(Fill::EvenOdd, scale, BrushRef::from(brush), None, user_path)
    }
    if let Some((brush, stroke_style)) = stroke {
        scene.stroke(stroke_style, scale, BrushRef::from(brush), None, user_path);
    }
}

fn load_image(source: &ImageSource) -> Result<peniko::ImageData, Box<dyn std::error::Error>> {
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

    let img = match source {
        ImageSource::Path(path) => image::load_from_memory(&std::fs::read(path)?)?,
        ImageSource::Bytes(bytes) => image::load_from_memory(bytes.as_ref())?,
        ImageSource::Buffer(width, height, container) => DynamicImage::ImageRgba8(
            ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(*width, *height, container.as_ref().clone())
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

#[cfg(test)]
mod tests {
    use super::cached_svg_scene;
    use anyrender::Scene;
    use std::collections::HashMap;

    const FIRST: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10"/></svg>"#;
    const SECOND: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="5"><rect width="20" height="5"/></svg>"#;

    #[test]
    fn svg_cache_replaces_content_for_existing_key() {
        let mut cache: HashMap<u64, (String, Scene, f32, f32)> = HashMap::new();

        {
            let (content, _, width, height) = cached_svg_scene(&mut cache, 1, FIRST);
            assert_eq!(content, FIRST);
            assert_eq!(*width, 10.);
            assert_eq!(*height, 10.);
        }
        assert_eq!(cache.len(), 1);

        {
            let (content, _, width, height) = cached_svg_scene(&mut cache, 1, FIRST);
            assert_eq!(content, FIRST);
            assert_eq!(*width, 10.);
            assert_eq!(*height, 10.);
        }
        assert_eq!(cache.len(), 1);

        {
            let (content, _, width, height) = cached_svg_scene(&mut cache, 1, SECOND);
            assert_eq!(content, SECOND);
            assert_eq!(*width, 20.);
            assert_eq!(*height, 5.);
        }
        assert_eq!(cache.len(), 1);

        cached_svg_scene(&mut cache, 2, FIRST);
        assert_eq!(cache.len(), 2);
    }
}
