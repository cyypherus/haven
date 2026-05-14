use crate::Area;
use crate::draw_layout::draw_layout;
use crate::primitives::{ImageSource, PathData};
use crate::render::{Frame, RenderItem, TextRenderLayout};
use image::{DynamicImage, ImageBuffer, Rgba};
use kurbo::{Affine, Point, Rect, RoundedRect, Size, Vec2};
use peniko::{self, Brush, Compose, Fill, Mix};
use std::collections::HashMap;
use std::sync::Arc;
use vello_svg::vello::util::{RenderContext, RenderSurface};
use vello_svg::vello::{Renderer, RendererOptions, Scene};

pub type VelloSurface = RenderSurface<'static>;

pub struct VelloRenderer {
    render_context: RenderContext,
    renderers: Vec<Option<Renderer>>,
    svg_scenes: HashMap<String, (Scene, f32, f32)>,
    image_scenes: HashMap<u64, (Scene, f32, f32)>,
}

impl Default for VelloRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl VelloRenderer {
    pub fn new() -> Self {
        Self {
            render_context: RenderContext::new(),
            renderers: Vec::new(),
            svg_scenes: HashMap::new(),
            image_scenes: HashMap::new(),
        }
    }

    pub fn create_surface(
        &mut self,
        window: Arc<winit::window::Window>,
        width: u32,
        height: u32,
        transparent: bool,
    ) -> RenderSurface<'static> {
        let mut surface = pollster::block_on(self.render_context.create_surface(
            window,
            width,
            height,
            vello_svg::vello::wgpu::PresentMode::AutoNoVsync,
        ))
        .expect("Error creating surface");
        if transparent {
            self.configure_transparency(&mut surface);
        }
        surface
    }

    pub fn configure_transparency(&mut self, surface: &mut RenderSurface<'static>) {
        let device = &self.render_context.devices[surface.dev_id].device;
        let capabilities = surface
            .surface
            .get_capabilities(self.render_context.devices[surface.dev_id].adapter());
        if capabilities
            .alpha_modes
            .contains(&wgpu::CompositeAlphaMode::PostMultiplied)
        {
            surface.config.alpha_mode = wgpu::CompositeAlphaMode::PostMultiplied;
        }
        surface.surface.configure(device, &surface.config);
    }

    pub fn resize(
        &mut self,
        surface: &mut RenderSurface<'static>,
        width: u32,
        height: u32,
    ) {
        self.render_context.resize_surface(surface, width, height);
    }

    pub fn render(&mut self, surface: &mut RenderSurface<'static>, frame: &Frame) {
        self.renderers
            .resize_with(self.render_context.devices.len(), || None);
        let dev_id = surface.dev_id;
        self.renderers[dev_id].get_or_insert_with(|| {
            Renderer::new(
                &self.render_context.devices[dev_id].device,
                RendererOptions::default(),
            )
            .expect("Failed to create renderer")
        });

        let mut scene = Scene::new();
        self.render_frame(frame, &mut scene);

        let device_handle = &self.render_context.devices[surface.dev_id];
        let render_params = vello_svg::vello::RenderParams {
            base_color: frame.base_color,
            width: frame.width,
            height: frame.height,
            antialiasing_method: vello_svg::vello::AaConfig::Msaa8,
        };

        self.renderers[surface.dev_id]
            .as_mut()
            .unwrap()
            .render_to_texture(
                &device_handle.device,
                &device_handle.queue,
                &scene,
                &surface.target_view,
                &render_params,
            )
            .expect("failed to render to texture");

        let surface_texture = surface
            .surface
            .get_current_texture()
            .expect("failed to get surface texture");

        let mut encoder =
            device_handle
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Surface Blit"),
                });
        surface.blitter.copy(
            &device_handle.device,
            &mut encoder,
            &surface.target_view,
            &surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );
        device_handle.queue.submit([encoder.finish()]);
        surface_texture.present();
    }

    fn render_frame(&mut self, frame: &Frame, scene: &mut Scene) {
        for item in &frame.items {
            match item {
                RenderItem::PushLayer { path, blend, alpha } => {
                    scene.push_layer(
                        Fill::NonZero,
                        *blend,
                        *alpha,
                        Affine::scale(frame.scale_factor),
                        path,
                    );
                }
                RenderItem::PopLayer => scene.pop_layer(),
                RenderItem::Text(text) => self.draw_text(scene, text),
                RenderItem::Layout { layout, transform } => draw_layout(*transform, layout, scene),
                RenderItem::Path { path, area } => {
                    draw_path(scene, path, *area, frame.scale_factor)
                }
                RenderItem::Svg { svg, area } => self.draw_svg(
                    scene,
                    frame.scale_factor,
                    &svg.content,
                    *area,
                    svg.unlocked_aspect_ratio,
                    svg.fill.as_ref(),
                ),
                RenderItem::Image { image, area } => self.draw_image(
                    scene,
                    frame.scale_factor,
                    image.cache_key(),
                    &image.source,
                    *area,
                    image.unlocked_aspect_ratio,
                    image.corner_rounding,
                ),
            }
        }
    }

    fn draw_text(&mut self, scene: &mut Scene, text: &TextRenderLayout) {
        for (rect, brush) in &text.backgrounds {
            scene.fill(Fill::NonZero, text.transform, brush, None, rect);
        }
        draw_layout(text.transform, &text.layout, scene);
    }

    fn draw_image(
        &mut self,
        scene: &mut Scene,
        scale_factor: f64,
        cache_key: u64,
        source: &ImageSource,
        area: Area,
        unlocked_aspect_ratio: bool,
        corner_rounding: f32,
    ) {
        if !self.image_scenes.contains_key(&cache_key) {
            let peniko_image = match load_image(source) {
                Ok(img) => img,
                Err(err) => {
                    eprintln!("Loading image failed: {err}");
                    self.image_scenes.insert(cache_key, (Scene::new(), 0., 0.));
                    return;
                }
            };

            let width = peniko_image.width as f32;
            let height = peniko_image.height as f32;

            let mut image_scene = Scene::new();
            image_scene.draw_image(&peniko_image, Affine::IDENTITY);

            self.image_scenes
                .insert(cache_key, (image_scene, width, height));
        }

        if let Some((image_scene, width, height)) = self.image_scenes.get(&cache_key) {
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
                Fill::NonZero,
                Mix::Normal,
                1.,
                transform,
                &RoundedRect::from_origin_size(
                    Point::ZERO,
                    Size::new(width, height),
                    corner_rounding as f64 / scale,
                ),
            );
            scene.append(image_scene, Some(transform));
            scene.pop_layer();
        }
    }

    fn draw_svg(
        &mut self,
        scene: &mut Scene,
        scale_factor: f64,
        content: &str,
        area: Area,
        unlocked_aspect_ratio: bool,
        fill: Option<&Brush>,
    ) {
        if !self.svg_scenes.contains_key(content) {
            match vello_svg::usvg::Tree::from_data(
                content.as_bytes(),
                &vello_svg::usvg::Options::default(),
            ) {
                Err(err) => {
                    eprintln!("Loading svg failed: {err}");
                    self.svg_scenes
                        .insert(content.to_string(), (Scene::new(), 0., 0.));
                }
                Ok(svg) => {
                    let svg_scene = vello_svg::render_tree(&svg);
                    let size = svg.size();
                    self.svg_scenes.insert(
                        content.to_string(),
                        (svg_scene, size.width(), size.height()),
                    );
                }
            }
        }
        if let Some((svg_scene, width, height)) = self.svg_scenes.get(content) {
            let width = *width as f64;
            let height = *height as f64;
            let area_x = area.x as f64 * scale_factor;
            let area_y = area.y as f64 * scale_factor;
            let area_width = area.width as f64 * scale_factor;
            let area_height = area.height as f64 * scale_factor;
            if fill.is_some() {
                scene.push_layer(
                    Fill::NonZero,
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
                );
            }
            scene.append(
                svg_scene,
                Some(if unlocked_aspect_ratio {
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
                }),
            );
            if let Some(fill) = fill {
                scene.push_layer(
                    Fill::NonZero,
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
                );

                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    fill,
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
}

fn draw_path(scene: &mut Scene, path: &PathData, area: Area, scale_factor: f64) {
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
            scene.fill(Fill::EvenOdd, scale, &brush, None, &user_path)
        }
        if let Some((ref brush_source, ref stroke_style)) = path.stroke {
            let brush = brush_source.resolve(area, &());
            scene.stroke(stroke_style, scale, &brush, None, &user_path);
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
