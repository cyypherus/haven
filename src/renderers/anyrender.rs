use crate::Area;
use crate::draw_layout::draw_layout;
use crate::primitives::{ImageSource, PathData};
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
    svg_scenes: HashMap<u64, (String, Option<(Scene, f32, f32)>)>,
    image_data: HashMap<u64, Option<peniko::ImageData>>,
}

impl FramePainter {
    pub fn paint(&mut self, frame: &Frame, scene: &mut impl PaintScene) {
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
        window_renderer.resume(window, width, height, || {});
        window_renderer.complete_resume();
        Self {
            window_renderer,
            painter: FramePainter::default(),
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
        let painter = &mut self.painter;
        self.window_renderer.render(|scene| {
            scene.reset();
            painter.paint(frame, scene);
            pre_present_notify();
        });
    }
}

fn render_frame<S: PaintScene>(
    svg_scenes: &mut HashMap<u64, (String, Option<(Scene, f32, f32)>)>,
    image_data: &mut HashMap<u64, Option<peniko::ImageData>>,
    frame: &Frame,
    scene: &mut S,
) {
    for item in &frame.items {
        match item {
            RenderItem::PushLayer {
                path,
                blend,
                alpha,
                filter,
            } => {
                let clip = filter.as_ref().map(|_| {
                    use kurbo::Shape;
                    Rect::new(
                        0.,
                        0.,
                        frame.width as f64 / frame.scale_factor,
                        frame.height as f64 / frame.scale_factor,
                    )
                    .to_path(0.1)
                });
                scene.push_layer(
                    *blend,
                    *alpha,
                    Affine::scale(frame.scale_factor),
                    clip.as_ref().unwrap_or(path),
                    filter.clone(),
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
                svg.cache_key(),
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
    image_data: &mut HashMap<u64, Option<peniko::ImageData>>,
    scale_factor: f64,
    cache_key: u64,
    source: &ImageSource,
    area: Area,
    unlocked_aspect_ratio: bool,
    corner_rounding: f32,
) {
    let image = image_data.entry(cache_key).or_insert_with(|| {
        load_image(source)
            .map_err(|err| eprintln!("Loading image failed: {err}"))
            .ok()
    });
    if let Some(image) = image.as_ref() {
        let width = image.width as f64;
        let height = image.height as f64;
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
    svg_scenes: &mut HashMap<u64, (String, Option<(Scene, f32, f32)>)>,
    scale_factor: f64,
    cache_key: u64,
    content: &str,
    area: Area,
    unlocked_aspect_ratio: bool,
    fill: Option<&Brush>,
) {
    let Some((svg_scene, width, height)) = cached_svg_scene(svg_scenes, cache_key, content) else {
        return;
    };
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
    svg_scenes: &'a mut HashMap<u64, (String, Option<(Scene, f32, f32)>)>,
    cache_key: u64,
    content: &str,
) -> Option<&'a (Scene, f32, f32)> {
    let needs_update = svg_scenes
        .get(&cache_key)
        .is_none_or(|(cached_content, _)| cached_content != content);

    if needs_update {
        let cached_svg = match anyrender_svg::usvg::Tree::from_data(
            content.as_bytes(),
            &anyrender_svg::usvg::Options::default(),
        ) {
            Err(err) => {
                eprintln!("Loading svg failed: {err}");
                None
            }
            Ok(svg) => {
                let mut svg_scene = Scene::new();
                anyrender_svg::render_svg_tree(&mut svg_scene, &svg, Affine::IDENTITY);
                let size = svg.size();
                Some((svg_scene, size.width(), size.height()))
            }
        };

        svg_scenes.insert(cache_key, (content.to_string(), cached_svg));
    }

    svg_scenes.get(&cache_key).expect("cached svg").1.as_ref()
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

    if width == 0 || height == 0 {
        return Err("image dimensions must be positive".into());
    }
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
    use super::*;
    use anyrender::Scene;
    use std::collections::HashMap;

    #[test]
    fn paint_appends_at_physical_scale() {
        use anyrender::recording::RenderCommand;
        use kurbo::Shape;
        let started = std::time::Instant::now();
        let mut pane = crate::PaneBuilder::new("test", |_: &(), ctx| {
            crate::rect(1)
                .fill(crate::Color::WHITE)
                .corner_rounding(0.)
                .build(ctx)
        })
        .background(crate::Color::BLACK)
        .build();
        let (frame, effects) = pane.redraw(&mut (), 80, 40, 2.);
        assert!(effects.is_empty());
        let mut scene = Scene::new();
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            crate::Color::TRANSPARENT,
            None,
            &Rect::new(0., 0., 1., 1.),
        );
        let original = scene.commands[0].clone();
        let mut painter = FramePainter::default();
        painter.paint(&frame, &mut scene);
        assert_eq!(scene.commands.len(), 3);
        assert_eq!(scene.commands[0], original);
        let RenderCommand::Fill(background) = &scene.commands[1] else {
            panic!("frame background missing");
        };
        assert_eq!(background.transform, Affine::IDENTITY);
        assert_eq!(background.shape.bounding_box(), Rect::new(0., 0., 80., 40.));
        assert_eq!(
            background.brush,
            anyrender::Paint::Solid(crate::Color::BLACK)
        );
        let RenderCommand::Fill(content) = &scene.commands[2] else {
            panic!("frame content missing");
        };
        assert_eq!(content.transform, Affine::scale(2.));
        assert_eq!(content.shape.bounding_box(), Rect::new(0., 0., 40., 20.));
        eprintln!(
            "stabs: added_paint_commands=2, svg_cache_entries={}, image_cache_entries={}, elapsed_ms={:.3}",
            painter.svg_scenes.len(),
            painter.image_data.len(),
            started.elapsed().as_secs_f64() * 1000.
        );
    }

    #[test]
    fn image_cache_reuses_pixels_and_skips_invalid_images() {
        use anyrender::recording::RenderCommand;
        let started = std::time::Instant::now();
        let mut cache = HashMap::new();
        let mut scene = Scene::new();
        let area = Area {
            x: 0.,
            y: 0.,
            width: 100.,
            height: 100.,
        };
        let source = ImageSource::Buffer(20, 10, Arc::new(vec![255; 800]));
        draw_image(&mut scene, &mut cache, 2., 1, &source, area, false, 8.);
        let blob = cache[&1].as_ref().expect("valid image").data.id();
        draw_image(&mut scene, &mut cache, 2., 1, &source, area, false, 8.);
        assert_eq!(cache[&1].as_ref().expect("cached image").data.id(), blob);
        assert_eq!(scene.commands.len(), 6);
        let RenderCommand::Fill(image) = &scene.commands[1] else {
            panic!("image missing")
        };
        assert_eq!(
            image.transform,
            Affine::scale(10.).then_translate(Vec2::new(0., 50.))
        );
        for _ in 0..2 {
            draw_image(
                &mut scene,
                &mut cache,
                2.,
                2,
                &ImageSource::Buffer(1, 1, Arc::new(vec![255; 3])),
                area,
                false,
                8.,
            );
            draw_image(
                &mut scene,
                &mut cache,
                2.,
                3,
                &ImageSource::Buffer(0, 0, Arc::new(Vec::new())),
                area,
                false,
                8.,
            );
        }
        assert_eq!(scene.commands.len(), 6);
        assert!(cache[&2].is_none());
        assert!(cache[&3].is_none());
        eprintln!(
            "stabs: image_cache_entries=3, decoded_bytes=800, image_paints=2, invalid_dimension_sentinels=0, elapsed_ms={:.3}",
            started.elapsed().as_secs_f64() * 1000.
        );
    }

    const FIRST: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10"/></svg>"#;
    const SECOND: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="5"><rect width="20" height="5"/></svg>"#;

    #[test]
    fn svg_cache_replaces_content_for_existing_key() {
        let mut cache: HashMap<u64, (String, Option<(Scene, f32, f32)>)> = HashMap::new();

        {
            let (_, width, height) = cached_svg_scene(&mut cache, 1, FIRST).expect("valid SVG");
            assert_eq!(*width, 10.);
            assert_eq!(*height, 10.);
        }
        assert_eq!(cache.len(), 1);

        {
            let (_, width, height) = cached_svg_scene(&mut cache, 1, FIRST).expect("valid SVG");
            assert_eq!(*width, 10.);
            assert_eq!(*height, 10.);
        }
        assert_eq!(cache.len(), 1);

        {
            let (_, width, height) = cached_svg_scene(&mut cache, 1, SECOND).expect("valid SVG");
            assert_eq!(*width, 20.);
            assert_eq!(*height, 5.);
        }
        assert_eq!(cache.len(), 1);

        assert!(cached_svg_scene(&mut cache, 2, "invalid").is_none());
        assert!(cached_svg_scene(&mut cache, 2, FIRST).is_some());
        assert_eq!(cache.len(), 2);
        eprintln!(
            "stabs: svg_cache_entries=2, failed_svg_entries=0, invalid_dimension_sentinels=0"
        );
    }
}
