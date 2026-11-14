use crate::Area;
use crate::draw_layout::draw_layout;
use crate::primitives::{ImageSource, Svg};
use crate::render::{Filter, Frame, RenderItem};
use image::{DynamicImage, ImageBuffer, Rgba};
use kurbo::{Affine, BezPath, Point, Rect, RoundedRect, Shape, Size, Stroke, Vec2};
use peniko::{self, BlendMode, Brush, Compose, Fill, Mix};
use std::collections::HashMap;
use std::sync::Arc;
use vello_common::filter_effects::{EdgeMode, FilterPrimitive};
use vello_common::paint::{ImageId, PaintType};
use vello_hybrid::{RenderSize, RenderTargetConfig, Resources, TextureBindings};

const TOLERANCE: f64 = 0.1;

pub struct TextureRenderer {
    scene: Scene,
    images: HashMap<u64, Option<peniko::ImageData>>,
    svgs: HashMap<u64, Option<vello_svg::usvg::Tree>>,
}

pub(crate) struct Scene {
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: vello_hybrid::Renderer,
    resources: Resources,
    hybrid: vello_hybrid::Scene,
    uploaded_images: HashMap<u64, ImageId>,
}

impl TextureRenderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        Self {
            scene: Scene {
                device: device.clone(),
                queue: queue.clone(),
                renderer: vello_hybrid::Renderer::new(
                    device,
                    &RenderTargetConfig {
                        format,
                        width: 1,
                        height: 1,
                    },
                ),
                resources: Resources::new(),
                hybrid: vello_hybrid::Scene::new(1, 1),
                uploaded_images: HashMap::new(),
            },
            images: HashMap::new(),
            svgs: HashMap::new(),
        }
    }

    pub fn render(&mut self, frame: &Frame, target: &wgpu::TextureView) {
        let (width, height) = (frame.width as u16, frame.height as u16);
        let scene = &mut self.scene;
        if scene.hybrid.width() != width || scene.hybrid.height() != height {
            scene.hybrid = vello_hybrid::Scene::new(width, height);
        } else {
            scene.hybrid.reset();
        }
        let scale = Affine::scale(frame.scale_factor);
        let bounds = Rect::new(0., 0., frame.width as f64, frame.height as f64);
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            &Brush::Solid(frame.base_color),
            Affine::IDENTITY,
            &bounds.to_path(TOLERANCE),
        );
        for item in &frame.items {
            match item {
                RenderItem::PushLayer {
                    path,
                    blend,
                    alpha,
                    filter,
                } => {
                    let unclipped = filter.map(|_| scale.inverse() * bounds.to_path(TOLERANCE));
                    scene.push_layer(
                        *blend,
                        *alpha,
                        scale,
                        unclipped.as_ref().unwrap_or(path),
                        filter.map(|filter| {
                            vello_common::filter_effects::Filter::from_primitive(match filter {
                                Filter::Blur { radius } => FilterPrimitive::GaussianBlur {
                                    std_deviation: radius,
                                    edge_mode: EdgeMode::None,
                                },
                                Filter::DropShadow {
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
                RenderItem::PopLayer => scene.pop_layer(),
                RenderItem::Text(text) => {
                    for (rect, brush) in &text.backgrounds {
                        scene.fill(
                            Fill::NonZero,
                            text.transform,
                            brush,
                            Affine::IDENTITY,
                            &rect.to_path(TOLERANCE),
                        );
                    }
                    draw_layout(scene, text.transform, &text.layout);
                }
                RenderItem::Layout { layout, transform } => draw_layout(scene, *transform, layout),
                RenderItem::Path {
                    path, fill, stroke, ..
                } => {
                    if fill.is_none() && stroke.is_none() {
                        scene.fill(
                            Fill::EvenOdd,
                            scale,
                            &Brush::Solid(peniko::Color::BLACK),
                            Affine::IDENTITY,
                            path,
                        );
                    }
                    if let Some(brush) = fill {
                        scene.fill(Fill::EvenOdd, scale, brush, Affine::IDENTITY, path);
                    }
                    if let Some((brush, stroke)) = stroke {
                        scene.stroke(stroke, scale, brush, Affine::IDENTITY, path);
                    }
                }
                RenderItem::Svg {
                    svg,
                    cache_key,
                    area,
                } => {
                    let tree = self.svgs.entry(*cache_key).or_insert_with(|| {
                        vello_svg::usvg::Tree::from_data(
                            svg.content.as_bytes(),
                            &Default::default(),
                        )
                        .inspect_err(|err| eprintln!("Loading svg failed: {err}"))
                        .ok()
                    });
                    if let Some(tree) = tree {
                        scene.draw_svg(tree, svg, *area, frame.scale_factor);
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
                    scene.fill(
                        Fill::NonZero,
                        transform,
                        &Brush::Image(peniko::ImageBrush::new(data)),
                        Affine::IDENTITY,
                        &RoundedRect::from_origin_size(
                            Point::ZERO,
                            size,
                            image.corner_rounding as f64 / scale,
                        )
                        .to_path(TOLERANCE),
                    );
                }
                RenderItem::Shadow { shadow, area } => scene.blurred_rounded_rect(
                    shadow.rect(*area, frame.scale_factor),
                    shadow.color,
                    shadow.corner_rounding * frame.scale_factor,
                    shadow.blur * frame.scale_factor,
                ),
            }
        }
        let mut encoder = scene
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        scene
            .renderer
            .render(
                &scene.hybrid,
                &mut scene.resources,
                &scene.device,
                &scene.queue,
                &mut encoder,
                &RenderSize {
                    width: frame.width,
                    height: frame.height,
                },
                target,
                &TextureBindings::new(),
            )
            .expect("render frame");
        scene.queue.submit([encoder.finish()]);
    }
}

impl Scene {
    fn draw_svg(&mut self, tree: &vello_svg::usvg::Tree, svg: &Svg, area: Area, scale_factor: f64) {
        let size = Size::new(tree.size().width() as f64, tree.size().height() as f64);
        let (transform, _) = fit(size, area, scale_factor, svg.unlocked_aspect_ratio);
        let target = scaled_rect(area, scale_factor).to_path(TOLERANCE);
        if svg.fill.is_some() {
            self.push_layer(BlendMode::default(), 1., Affine::IDENTITY, &target, None);
        }
        vello_svg::append_tree_with_transform(self, tree, transform);
        if let Some(fill) = &svg.fill {
            self.push_layer(
                BlendMode::new(Mix::Normal, Compose::SrcIn),
                1.,
                Affine::IDENTITY,
                &target,
                None,
            );
            self.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                fill,
                Affine::IDENTITY,
                &target,
            );
            self.pop_layer();
            self.pop_layer();
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
                        let id = self.renderer.upload_image(
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
        self.hybrid.set_paint(paint);
        self.hybrid.set_paint_transform(brush_transform);
    }

    pub(crate) fn fill(
        &mut self,
        rule: Fill,
        transform: Affine,
        brush: &Brush,
        brush_transform: Affine,
        path: &BezPath,
    ) {
        self.hybrid.set_transform(transform);
        self.hybrid.set_fill_rule(rule);
        self.set_brush(brush, brush_transform);
        self.hybrid.fill_path(path);
    }

    pub(crate) fn stroke(
        &mut self,
        stroke: &Stroke,
        transform: Affine,
        brush: &Brush,
        brush_transform: Affine,
        path: &BezPath,
    ) {
        self.hybrid.set_transform(transform);
        self.hybrid.set_stroke(stroke.clone());
        self.set_brush(brush, brush_transform);
        self.hybrid.stroke_path(path);
    }

    pub(crate) fn glyphs(
        &mut self,
        font: &peniko::FontData,
        font_size: f32,
        normalized_coords: &[i16],
        brush: &Brush,
        transform: Affine,
        glyph_transform: Affine,
        glyphs: impl Iterator<Item = glifo::Glyph> + Clone,
    ) {
        self.hybrid.set_transform(transform);
        self.hybrid.set_fill_rule(Fill::NonZero);
        self.set_brush(brush, Affine::IDENTITY);
        self.hybrid
            .glyph_run(&mut self.resources, font)
            .font_size(font_size)
            .hint(true)
            .normalized_coords(normalized_coords)
            .glyph_transform(glyph_transform)
            .fill_glyphs(glyphs);
    }

    pub(crate) fn push_layer(
        &mut self,
        blend: BlendMode,
        alpha: f32,
        transform: Affine,
        clip: &BezPath,
        filter: Option<vello_common::filter_effects::Filter>,
    ) {
        self.hybrid.set_transform(transform);
        self.hybrid.push_clip_path(clip);
        self.hybrid
            .push_layer(None, Some(blend), Some(alpha), None, filter);
    }

    pub(crate) fn pop_layer(&mut self) {
        self.hybrid.pop_layer();
        self.hybrid.pop_clip_path();
    }

    fn blurred_rounded_rect(&mut self, rect: Rect, color: peniko::Color, radius: f64, blur: f64) {
        self.hybrid.set_transform(Affine::IDENTITY);
        self.set_brush(&Brush::Solid(color), Affine::IDENTITY);
        self.hybrid
            .fill_blurred_rounded_rect(&rect, radius as f32, blur as f32, false);
    }
}

impl vello_svg::RenderSink for Scene {
    fn push_layer(
        &mut self,
        blend: impl Into<BlendMode>,
        alpha: f32,
        transform: Affine,
        shape: &impl Shape,
    ) {
        Self::push_layer(
            self,
            blend.into(),
            alpha,
            transform,
            &shape.to_path(TOLERANCE),
            None,
        );
    }

    fn pop_layer(&mut self) {
        Self::pop_layer(self);
    }

    fn fill(
        &mut self,
        fill: Fill,
        transform: Affine,
        brush: &Brush,
        brush_transform: Affine,
        shape: &impl Shape,
    ) {
        Self::fill(
            self,
            fill,
            transform,
            brush,
            brush_transform,
            &shape.to_path(TOLERANCE),
        );
    }

    fn stroke(
        &mut self,
        stroke: &Stroke,
        transform: Affine,
        brush: &Brush,
        brush_transform: Affine,
        shape: &impl Shape,
    ) {
        Self::stroke(
            self,
            stroke,
            transform,
            brush,
            brush_transform,
            &shape.to_path(TOLERANCE),
        );
    }

    fn draw_image(&mut self, image: &vello_svg::usvg::Image, transform: Affine) {
        let (vello_svg::usvg::ImageKind::JPEG(data)
        | vello_svg::usvg::ImageKind::PNG(data)
        | vello_svg::usvg::ImageKind::GIF(data)
        | vello_svg::usvg::ImageKind::WEBP(data)) = image.kind()
        else {
            return;
        };
        let Ok(decoded) = image::load_from_memory(data) else {
            return;
        };
        let decoded = decoded.into_rgba8();
        let (width, height) = decoded.dimensions();
        Self::fill(
            self,
            Fill::NonZero,
            transform,
            &Brush::Image(peniko::ImageBrush::new(peniko::ImageData {
                data: peniko::Blob::new(Arc::new(decoded.into_raw())),
                format: peniko::ImageFormat::Rgba8,
                alpha_type: peniko::ImageAlphaType::Alpha,
                width,
                height,
            })),
            Affine::IDENTITY,
            &Rect::new(0., 0., width as f64, height as f64).to_path(TOLERANCE),
        );
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
