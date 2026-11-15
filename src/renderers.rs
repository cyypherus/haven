mod image;

pub use image::HybridImageRenderer;

use crate::Area;
use crate::primitives::ImageSource;
use crate::render::{Frame, LayerEffect, RenderItem};
use ::image::{DynamicImage, ImageBuffer, Rgba};
use kurbo::{Affine, Line, Point, Rect, RoundedRect, Shape, Size, Stroke, Vec2};
use parley::{Layout, PositionedLayoutItem};
use peniko::{self, BlendMode, Brush, Compose, Fill, Mix};
use std::collections::HashMap;
use std::sync::Arc;
use vello_common::filter_effects::{EdgeMode, FilterPrimitive};
use vello_common::paint::{ImageId, PaintType};
use vello_hybrid::{RenderSize, RenderTargetConfig, Resources, TextureBindings};
use vello_svg::usvg;

const TOLERANCE: f64 = 0.1;

pub struct HybridTextureRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    vello: vello_hybrid::Renderer,
    resources: Resources,
    scene: vello_hybrid::Scene,
    images: HashMap<u64, Option<peniko::ImageData>>,
    embedded_images: HashMap<*const Vec<u8>, Option<peniko::ImageData>>,
    svgs: HashMap<u64, Option<Arc<usvg::Tree>>>,
    uploaded_images: HashMap<u64, ImageId>,
}

impl HybridTextureRenderer {
    pub fn with_device(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Option<Self> {
        if !matches!(
            format,
            wgpu::TextureFormat::Rgba8Unorm
                | wgpu::TextureFormat::Rgba8UnormSrgb
                | wgpu::TextureFormat::Bgra8Unorm
                | wgpu::TextureFormat::Bgra8UnormSrgb
        ) {
            return None;
        }
        Some(Self {
            device: device.clone(),
            queue: queue.clone(),
            vello: vello_hybrid::Renderer::new(
                device,
                &RenderTargetConfig {
                    format,
                    width: 1,
                    height: 1,
                },
            ),
            resources: Resources::new(),
            scene: vello_hybrid::Scene::new(1, 1),
            images: HashMap::new(),
            embedded_images: HashMap::new(),
            svgs: HashMap::new(),
            uploaded_images: HashMap::new(),
        })
    }

    pub fn render(&mut self, frame: &Frame, target: &wgpu::TextureView) {
        let (width, height) = (frame.width as u16, frame.height as u16);
        if self.scene.width() != width || self.scene.height() != height {
            self.scene = vello_hybrid::Scene::new(width, height);
        } else {
            self.scene.reset();
        }
        let scale = Affine::scale(frame.scale_factor);
        let bounds = Rect::new(0., 0., frame.width as f64, frame.height as f64);
        self.scene.set_transform(Affine::IDENTITY);
        self.scene.set_fill_rule(Fill::NonZero);
        self.set_brush(&Brush::Solid(frame.base_color), Affine::IDENTITY);
        self.scene.fill_rect(&bounds);
        for item in &frame.items {
            match item {
                RenderItem::PushLayer {
                    path,
                    blend,
                    alpha,
                    effect,
                } => {
                    let unclipped = effect.map(|_| scale.inverse() * bounds.to_path(TOLERANCE));
                    self.scene.set_transform(scale);
                    self.scene
                        .push_clip_path(unclipped.as_ref().unwrap_or(path));
                    self.scene.push_layer(
                        None,
                        Some(*blend),
                        Some(*alpha),
                        None,
                        effect.map(|effect| {
                            vello_common::filter_effects::Filter::from_primitive(match effect {
                                LayerEffect::Blur { radius } => FilterPrimitive::GaussianBlur {
                                    std_deviation: radius,
                                    edge_mode: EdgeMode::None,
                                },
                                LayerEffect::DropShadow {
                                    dx,
                                    dy,
                                    blur,
                                    color,
                                } => FilterPrimitive::DropShadow {
                                    dx,
                                    dy,
                                    std_deviation: blur,
                                    color,
                                    edge_mode: EdgeMode::None,
                                },
                            })
                        }),
                    );
                }
                RenderItem::PopLayer => {
                    self.scene.pop_layer();
                    self.scene.pop_clip_path();
                }
                RenderItem::Text(text) => {
                    self.scene.set_transform(text.transform);
                    self.scene.set_fill_rule(Fill::NonZero);
                    for (rect, brush) in &text.backgrounds {
                        self.set_brush(brush, Affine::IDENTITY);
                        self.scene.fill_rect(rect);
                    }
                    self.draw_layout(text.transform, &text.layout);
                }
                RenderItem::Layout { layout, transform } => self.draw_layout(*transform, layout),
                RenderItem::Path {
                    path, fill, stroke, ..
                } => {
                    self.scene.set_transform(scale);
                    self.scene.set_fill_rule(Fill::EvenOdd);
                    if fill.is_none() && stroke.is_none() {
                        self.set_brush(&Brush::Solid(peniko::Color::BLACK), Affine::IDENTITY);
                        self.scene.fill_path(path);
                    }
                    if let Some(brush) = fill {
                        self.set_brush(brush, Affine::IDENTITY);
                        self.scene.fill_path(path);
                    }
                    if let Some((brush, stroke)) = stroke {
                        self.scene.set_stroke(stroke.clone());
                        self.set_brush(brush, Affine::IDENTITY);
                        self.scene.stroke_path(path);
                    }
                }
                RenderItem::Svg {
                    svg,
                    cache_key,
                    area,
                } => {
                    let tree = self
                        .svgs
                        .entry(*cache_key)
                        .or_insert_with(|| {
                            usvg::Tree::from_data(svg.content.as_bytes(), &Default::default())
                                .inspect_err(|err| eprintln!("Loading svg failed: {err}"))
                                .ok()
                                .map(Arc::new)
                        })
                        .clone();
                    if let Some(tree) = tree {
                        let size =
                            Size::new(tree.size().width() as f64, tree.size().height() as f64);
                        let (transform, _) =
                            fit(size, *area, frame.scale_factor, svg.unlocked_aspect_ratio);
                        let target = scaled_rect(*area, frame.scale_factor).to_path(TOLERANCE);
                        if svg.fill.is_some() {
                            self.scene.set_transform(Affine::IDENTITY);
                            self.scene.push_clip_path(&target);
                            self.scene.push_layer(None, None, None, None, None);
                        }
                        vello_svg::append_tree_with_transform(
                            &mut SvgPainter(self),
                            &tree,
                            transform,
                        );
                        if let Some(fill) = &svg.fill {
                            self.scene.set_transform(Affine::IDENTITY);
                            self.scene.push_clip_path(&target);
                            self.scene.push_layer(
                                None,
                                Some(BlendMode::new(Mix::Normal, Compose::SrcIn)),
                                None,
                                None,
                                None,
                            );
                            self.scene.set_fill_rule(Fill::NonZero);
                            self.set_brush(fill, Affine::IDENTITY);
                            self.scene.fill_path(&target);
                            for _ in 0..2 {
                                self.scene.pop_layer();
                                self.scene.pop_clip_path();
                            }
                        }
                    }
                }
                RenderItem::Image {
                    image,
                    cache_key,
                    area,
                } => {
                    let Some(data) = self
                        .images
                        .entry(*cache_key)
                        .or_insert_with(|| {
                            load_image(&image.source)
                                .inspect_err(|err| eprintln!("Loading image failed: {err}"))
                                .ok()
                        })
                        .clone()
                    else {
                        continue;
                    };
                    let size = Size::new(data.width as f64, data.height as f64);
                    let (transform, scale) =
                        fit(size, *area, frame.scale_factor, image.unlocked_aspect_ratio);
                    self.scene.set_transform(transform);
                    self.scene.set_fill_rule(Fill::NonZero);
                    self.set_brush(
                        &Brush::Image(peniko::ImageBrush::new(data)),
                        Affine::IDENTITY,
                    );
                    self.scene.fill_path(
                        &RoundedRect::from_origin_size(
                            Point::ZERO,
                            size,
                            image.corner_rounding as f64 / scale,
                        )
                        .to_path(TOLERANCE),
                    );
                }
            }
        }
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.vello
            .render(
                &self.scene,
                &mut self.resources,
                &self.device,
                &self.queue,
                &mut encoder,
                &RenderSize {
                    width: frame.width,
                    height: frame.height,
                },
                target,
                &TextureBindings::new(),
            )
            .expect("render frame");
        self.queue.submit([encoder.finish()]);
    }

    fn draw_layout(&mut self, transform: Affine, layout: &Layout<Brush>) {
        self.scene.set_transform(transform);
        self.scene.set_fill_rule(Fill::NonZero);
        for line in layout.lines() {
            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                    continue;
                };
                let style = glyph_run.style();
                let run = glyph_run.run();
                let metrics = run.metrics();
                let decoration = |offset: f32, size: f32| {
                    let y = (glyph_run.baseline() - offset + size / 2.) as f64;
                    Line::new(
                        (glyph_run.offset() as f64, y),
                        ((glyph_run.offset() + glyph_run.advance()) as f64, y),
                    )
                };
                self.set_brush(&style.brush, Affine::IDENTITY);
                if let Some(underline) = &style.underline {
                    let width = underline.size.unwrap_or(metrics.underline_size);
                    let offset = underline.offset.unwrap_or(metrics.underline_offset);
                    self.scene.set_stroke(Stroke::new(width.into()));
                    self.scene
                        .stroke_path(&decoration(offset, width).to_path(TOLERANCE));
                }
                let mut x = glyph_run.offset();
                let y = glyph_run.baseline();
                let glyphs: Vec<_> = glyph_run
                    .glyphs()
                    .map(|glyph| {
                        let gx = x + glyph.x;
                        x += glyph.advance;
                        glifo::Glyph {
                            id: glyph.id as _,
                            x: gx,
                            y: y - glyph.y,
                        }
                    })
                    .collect();
                self.scene
                    .glyph_run(&mut self.resources, run.font())
                    .font_size(run.font_size())
                    .hint(true)
                    .normalized_coords(run.normalized_coords())
                    .glyph_transform(
                        run.synthesis()
                            .skew()
                            .map(|angle| Affine::skew(angle.to_radians().tan() as f64, 0.))
                            .unwrap_or_default(),
                    )
                    .fill_glyphs(glyphs.into_iter());
                if let Some(strikethrough) = &style.strikethrough {
                    let width = strikethrough.size.unwrap_or(metrics.strikethrough_size);
                    let offset = strikethrough.offset.unwrap_or(metrics.strikethrough_offset);
                    self.scene.set_stroke(Stroke::new(width.into()));
                    self.scene.stroke_path(
                        &decoration(offset, metrics.strikethrough_size).to_path(TOLERANCE),
                    );
                }
            }
        }
    }

    fn set_brush(&mut self, brush: &Brush, brush_transform: Affine) {
        let paint = match brush {
            Brush::Solid(color) => PaintType::Solid(*color),
            Brush::Gradient(gradient) => PaintType::Gradient(gradient.clone()),
            Brush::Image(brush) => {
                let id = *self
                    .uploaded_images
                    .entry(brush.image.data.id())
                    .or_insert_with(|| {
                        let vello_common::paint::ImageSource::Pixmap(pixmap) =
                            vello_common::paint::ImageSource::from_peniko_image_data(&brush.image)
                        else {
                            unreachable!()
                        };
                        let mut encoder = self
                            .device
                            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                        let id = self.vello.upload_image(
                            &mut self.resources,
                            &self.device,
                            &self.queue,
                            &mut encoder,
                            &pixmap,
                        );
                        self.queue.submit([encoder.finish()]);
                        id
                    });
                PaintType::Image(peniko::ImageBrush {
                    image: vello_common::paint::ImageSource::OpaqueId {
                        id,
                        may_have_transparency: true,
                    },
                    sampler: brush.sampler,
                })
            }
        };
        self.scene.set_paint(paint);
        self.scene.set_paint_transform(brush_transform);
    }
}

struct SvgPainter<'a>(&'a mut HybridTextureRenderer);

impl vello_svg::RenderSink for SvgPainter<'_> {
    fn push_layer(
        &mut self,
        blend: impl Into<BlendMode>,
        alpha: f32,
        transform: Affine,
        shape: &impl Shape,
    ) {
        self.0.scene.set_transform(transform);
        self.0.scene.push_clip_path(&shape.to_path(TOLERANCE));
        self.0
            .scene
            .push_layer(None, Some(blend.into()), Some(alpha), None, None);
    }

    fn pop_layer(&mut self) {
        self.0.scene.pop_layer();
        self.0.scene.pop_clip_path();
    }

    fn fill(
        &mut self,
        fill: Fill,
        transform: Affine,
        brush: &Brush,
        brush_transform: Affine,
        shape: &impl Shape,
    ) {
        self.0.scene.set_transform(transform);
        self.0.scene.set_fill_rule(fill);
        self.0.set_brush(brush, brush_transform);
        self.0.scene.fill_path(&shape.to_path(TOLERANCE));
    }

    fn stroke(
        &mut self,
        stroke: &Stroke,
        transform: Affine,
        brush: &Brush,
        brush_transform: Affine,
        shape: &impl Shape,
    ) {
        self.0.scene.set_transform(transform);
        self.0.scene.set_stroke(stroke.clone());
        self.0.set_brush(brush, brush_transform);
        self.0.scene.stroke_path(&shape.to_path(TOLERANCE));
    }

    fn draw_image(&mut self, image: &usvg::Image, transform: Affine) {
        let (usvg::ImageKind::JPEG(data)
        | usvg::ImageKind::PNG(data)
        | usvg::ImageKind::GIF(data)
        | usvg::ImageKind::WEBP(data)) = image.kind()
        else {
            return;
        };
        let Some(data) = self
            .0
            .embedded_images
            .entry(Arc::as_ptr(data))
            .or_insert_with(|| {
                load_image(&ImageSource::Bytes(data.clone()))
                    .inspect_err(|err| eprintln!("Loading image failed: {err}"))
                    .ok()
            })
            .clone()
        else {
            return;
        };
        let rect = Rect::new(0., 0., data.width as f64, data.height as f64);
        self.0.scene.set_transform(transform);
        self.0.scene.set_fill_rule(Fill::NonZero);
        self.0.set_brush(
            &Brush::Image(peniko::ImageBrush::new(data)),
            Affine::IDENTITY,
        );
        self.0.scene.fill_rect(&rect);
    }
}

fn scaled_rect(area: Area, scale_factor: f64) -> Rect {
    Rect::from_origin_size(
        Point::new(area.x as f64 * scale_factor, area.y as f64 * scale_factor),
        Size::new(
            area.width as f64 * scale_factor,
            area.height as f64 * scale_factor,
        ),
    )
}

fn fit(size: Size, area: Area, scale_factor: f64, unlocked_aspect_ratio: bool) -> (Affine, f64) {
    let target = scaled_rect(area, scale_factor);
    if unlocked_aspect_ratio {
        let transform = Affine::IDENTITY
            .then_scale_non_uniform(target.width() / size.width, target.height() / size.height)
            .then_translate(target.origin().to_vec2());
        (transform, 1.)
    } else {
        let scale = (target.width() / size.width).min(target.height() / size.height);
        let transform = Affine::IDENTITY.then_scale(scale).then_translate(Vec2::new(
            target.x0 + (target.width() - size.width * scale) / 2.,
            target.y0 + (target.height() - size.height * scale) / 2.,
        ));
        (transform, scale)
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
        ImageSource::Path(path) => ::image::load_from_memory(&std::fs::read(path)?)?,
        ImageSource::Bytes(bytes) => ::image::load_from_memory(bytes.as_ref())?,
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
    use super::*;

    #[test]
    #[ignore = "requires a graphics adapter"]
    fn svg_drawing_preserves_layers_and_paint_order() {
        let mut renderer = HybridImageRenderer::new();
        for scale in [1, 2] {
            let mut frame = Frame {
                base_color: peniko::Color::BLACK,
                width: 128 * scale,
                height: 96 * scale,
                scale_factor: scale as f64,
                items: vec![RenderItem::Svg {
                    svg: crate::svg(
                        1,
                        r##"<svg xmlns="http://www.w3.org/2000/svg" width="96" height="64">
                            <defs>
                                <clipPath id="clip"><rect width="8" height="16"/></clipPath>
                                <linearGradient id="linear">
                                    <stop offset=".5" stop-color="blue"/>
                                    <stop offset=".5" stop-color="red"/>
                                </linearGradient>
                                <radialGradient id="radial">
                                    <stop offset=".5" stop-color="white"/>
                                    <stop offset=".5" stop-color="black"/>
                                </radialGradient>
                                <pattern id="pattern" width="4" height="4" patternUnits="userSpaceOnUse">
                                    <rect width="4" height="4" fill="blue"/>
                                </pattern>
                            </defs>
                            <g transform="translate(8 8)">
                                <g transform="translate(4 0)" opacity=".5" clip-path="url(#clip)">
                                    <rect width="16" height="16" fill="red"/>
                                </g>
                            </g>
                            <rect x="32" y="8" width="16" height="16" fill="blue" stroke="red" stroke-width="8"/>
                            <rect x="64" y="8" width="16" height="16" fill="blue" stroke="red" stroke-width="8" paint-order="stroke fill"/>
                            <path d="M8 32H24V48H8Z M12 36H20V44H12Z" fill="white" fill-rule="evenodd"/>
                            <rect x="32" y="32" width="16" height="16" fill="url(#linear)"/>
                            <rect x="64" y="32" width="16" height="16" fill="url(#radial)"/>
                            <rect x="80" width="8" height="8" fill="url(#pattern)"/>
                            <image x="80" y="48" width="8" height="8" href="data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSI4IiBoZWlnaHQ9IjgiPjxyZWN0IHdpZHRoPSI4IiBoZWlnaHQ9IjgiIGZpbGw9Im9yYW5nZSIvPjwvc3ZnPg=="/>
                        </svg>"##,
                    ),
                    cache_key: 1,
                    area: Area {
                        x: 16.,
                        y: 16.,
                        width: 96.,
                        height: 64.,
                    },
                }],
            };
            let image = renderer.render(&frame);
            for (x, y, expected) in [
                (14, 16, [128, 0, 0, 255]),
                (24, 16, [0, 0, 0, 255]),
                (33, 16, [255, 0, 0, 255]),
                (40, 16, [0, 0, 255, 255]),
                (65, 16, [0, 0, 255, 255]),
                (61, 16, [255, 0, 0, 255]),
                (10, 40, [255, 255, 255, 255]),
                (16, 40, [0, 0, 0, 255]),
                (34, 40, [0, 0, 255, 255]),
                (46, 40, [255, 0, 0, 255]),
                (72, 40, [255, 255, 255, 255]),
                (65, 33, [0, 0, 0, 255]),
                (84, 4, [128, 0, 0, 255]),
                (84, 52, [255, 165, 0, 255]),
            ] {
                assert_eq!(
                    image.get_pixel((16 + x) * scale, (16 + y) * scale).0,
                    expected,
                    "SVG pixel ({x}, {y}) at {scale}x"
                );
            }
            if let RenderItem::Svg { svg, .. } = &mut frame.items[0] {
                svg.fill = Some(Brush::Solid(peniko::Color::from_rgb8(0, 255, 0)));
            }
            let image = renderer.render(&frame);
            assert_eq!(image.get_pixel(56 * scale, 32 * scale).0, [0, 255, 0, 255]);
            assert_eq!(image.get_pixel(32 * scale, 56 * scale).0, [0, 0, 0, 255]);
        }
    }

    #[test]
    #[ignore = "requires a graphics adapter"]
    fn embedded_svg_image_reuses_uploads() {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let frame = Frame {
            base_color: peniko::Color::BLACK,
            width: 16,
            height: 16,
            scale_factor: 1.,
            items: vec![RenderItem::Svg {
                svg: crate::svg(
                    1,
                    r#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><image width="1" height="1" href="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg=="/></svg>"#,
                ),
                cache_key: 1,
                area: Area {
                    x: 0.,
                    y: 0.,
                    width: 16.,
                    height: 16.,
                },
            }],
        };
        for format in [
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Bgra8Unorm,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::TextureFormat::Bgra8UnormSrgb,
        ] {
            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: 16,
                    height: 16,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let view = target.create_view(&Default::default());
            let mut renderer = HybridTextureRenderer::with_device(&device, &queue, format).unwrap();
            for _ in 0..32 {
                renderer.render(&frame, &view);
            }
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            assert_eq!(renderer.uploaded_images.len(), 1);
        }
        let image = HybridImageRenderer::new().render(&frame);
        assert_eq!(image.get_pixel(8, 8).0, [255, 0, 0, 255]);
    }

    #[test]
    #[ignore = "requires a graphics adapter"]
    fn unsupported_render_target_formats_are_rejected() {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        for format in [
            wgpu::TextureFormat::Rgba16Float,
            wgpu::TextureFormat::Rgba8Uint,
            wgpu::TextureFormat::Depth32Float,
        ] {
            assert!(HybridTextureRenderer::with_device(&device, &queue, format).is_none());
        }
    }
}
