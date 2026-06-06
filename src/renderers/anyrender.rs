use crate::Area;
use crate::draw_layout::draw_layout;
use crate::primitives::{ImageSource, PathData};
use crate::render::{Frame, RenderItem, TextRenderLayout};
use anyrender::{PaintScene, Scene, WindowRenderer};
use image::{DynamicImage, ImageBuffer, Rgba};
use kurbo::{Affine, Point, Rect, RoundedRect, Size, Vec2};
use peniko::{self, Brush, BrushRef, Compose, Fill, Mix};
use std::collections::HashMap;
use std::sync::Arc;

pub struct Renderer<R: WindowRenderer> {
    window_renderer: R,
    svg_scenes: HashMap<String, (Scene, f32, f32)>,
    image_data: HashMap<u64, (peniko::ImageData, f32, f32)>,
}

impl<R: WindowRenderer> Renderer<R> {
    pub fn new(
        mut window_renderer: R,
        window: Arc<winit::window::Window>,
        width: u32,
        height: u32,
    ) -> Self {
        window_renderer.resume(window, width, height, || {});
        window_renderer.complete_resume();
        Self {
            window_renderer,
            svg_scenes: HashMap::new(),
            image_data: HashMap::new(),
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.window_renderer.complete_resume();
        self.window_renderer.set_size(width, height);
    }

    pub(crate) fn render(&mut self, frame: &Frame, pre_present_notify: impl FnOnce()) {
        if !self.window_renderer.complete_resume() {
            return;
        }

        let svg_scenes = &mut self.svg_scenes;
        let image_data = &mut self.image_data;
        self.window_renderer.render(|scene| {
            scene.reset();
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                frame.base_color,
                None,
                &Rect::new(0., 0., frame.width as f64, frame.height as f64),
            );
            render_frame(svg_scenes, image_data, frame, scene);
            pre_present_notify();
        });
    }
}

fn render_frame<S: PaintScene>(
    svg_scenes: &mut HashMap<String, (Scene, f32, f32)>,
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
            RenderItem::Path { path, area } => draw_path(scene, path, *area, frame.scale_factor),
            RenderItem::Svg { svg, area } => draw_svg(
                scene,
                svg_scenes,
                frame.scale_factor,
                &svg.content,
                *area,
                svg.unlocked_aspect_ratio,
                svg.fill.as_ref(),
            ),
            RenderItem::Image { image, area } => draw_image(
                scene,
                image_data,
                frame.scale_factor,
                image.cache_key(),
                &image.source,
                *area,
                image.unlocked_aspect_ratio,
                image.corner_rounding,
            ),
            RenderItem::Shadow { shadow, area } => {
                let rect = shadow.rect(*area, frame.scale_factor);
                scene.draw_box_shadow(
                    Affine::IDENTITY,
                    rect,
                    shadow.color,
                    shadow.corner_rounding * frame.scale_factor,
                    shadow.blur * frame.scale_factor,
                );
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
    svg_scenes: &mut HashMap<String, (Scene, f32, f32)>,
    scale_factor: f64,
    content: &str,
    area: Area,
    unlocked_aspect_ratio: bool,
    fill: Option<&Brush>,
) {
    if !svg_scenes.contains_key(content) {
        match anyrender_svg::usvg::Tree::from_data(
            content.as_bytes(),
            &anyrender_svg::usvg::Options::default(),
        ) {
            Err(err) => {
                eprintln!("Loading svg failed: {err}");
                svg_scenes.insert(content.to_string(), (Scene::new(), 0., 0.));
            }
            Ok(svg) => {
                let mut svg_scene = Scene::new();
                anyrender_svg::render_svg_tree(&mut svg_scene, &svg, Affine::IDENTITY);
                let size = svg.size();
                svg_scenes.insert(
                    content.to_string(),
                    (svg_scene, size.width(), size.height()),
                );
            }
        }
    }
    if let Some((svg_scene, width, height)) = svg_scenes.get(content) {
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
}

fn draw_path<S: PaintScene>(scene: &mut S, path: &PathData, area: Area, scale_factor: f64) {
    let user_path = (path.builder)(area);
    let scale = Affine::scale(scale_factor);
    let scaled_path = scale * &user_path;

    if path.fill.is_none() && path.stroke.is_none() {
        scene.fill(
            Fill::EvenOdd,
            Affine::IDENTITY,
            peniko::Color::BLACK,
            None,
            &scaled_path,
        )
    } else {
        if let Some(ref brush_source) = path.fill {
            let brush = brush_source.resolve(area, &());
            scene.fill(
                Fill::EvenOdd,
                scale,
                BrushRef::from(&brush),
                None,
                &user_path,
            )
        }
        if let Some((ref brush_source, ref stroke_style)) = path.stroke {
            let brush = brush_source.resolve(area, &());
            scene.stroke(
                stroke_style,
                scale,
                BrushRef::from(&brush),
                None,
                &user_path,
            );
        }
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
