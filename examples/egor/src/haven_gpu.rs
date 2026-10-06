#![allow(clippy::type_complexity)]

use anyrender::{Glyph, PaintScene, Scene};
use anyrender_vello_hybrid::{ImageManager, VelloHybridScenePainter};
use haven::render::{Effect, Frame, FramePainter, RenderItem, TextRenderLayout};
use haven::*;
use kurbo::{Affine, Line, Rect, Size, Vec2};
use parley::{Layout, PositionedLayoutItem};
use peniko::{BrushRef, Compose, Fill, Mix};
use rustc_hash::FxHashMap;
use std::{collections::HashMap, sync::Arc};

pub(super) struct Painter {
    gpu: wgpu_context::DeviceHandle,
    svg_scenes: HashMap<u64, (String, Option<(Scene, Size)>)>,
    image_atlas: FxHashMap<u64, vello_common::paint::ImageId>,
    vello: vello_hybrid::Renderer,
    resources: vello_hybrid::Resources,
    scene: vello_hybrid::Scene,
    target: Option<wgpu::TextureView>,
}

impl Painter {
    pub(super) fn new(
        instance: &wgpu::Instance,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Self {
        let vello = vello_hybrid::Renderer::new(
            device,
            &vello_hybrid::RenderTargetConfig {
                format: wgpu::TextureFormat::Rgba8Unorm,
                width: 1,
                height: 1,
            },
        );
        Self {
            gpu: wgpu_context::DeviceHandle {
                instance: instance.clone(),
                adapter: adapter.clone(),
                device: device.clone(),
                queue: queue.clone(),
            },
            svg_scenes: HashMap::new(),
            image_atlas: FxHashMap::default(),
            vello,
            resources: vello_hybrid::Resources::new(),
            scene: vello_hybrid::Scene::new(0, 0),
            target: None,
        }
    }
}

impl FramePainter for Painter {
    type Output = wgpu::TextureView;

    fn paint(&mut self, frame: &Frame) -> Self::Output {
        if self.scene.width() as u32 != frame.width || self.scene.height() as u32 != frame.height {
            self.target = Some(
                self.gpu
                    .device
                    .create_texture(&wgpu::TextureDescriptor {
                        label: Some("Haven target"),
                        size: wgpu::Extent3d {
                            width: frame.width,
                            height: frame.height,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                            | wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    })
                    .create_view(&Default::default()),
            );
            self.scene = vello_hybrid::Scene::new(
                frame.width.try_into().expect("Haven width"),
                frame.height.try_into().expect("Haven height"),
            );
        }
        self.scene.reset();
        let mut encoder = self.gpu.device.create_command_encoder(&Default::default());
        let mut texture_bindings = FxHashMap::default();
        {
            let images = ImageManager::new(
                &mut self.vello,
                &mut self.resources,
                &self.gpu.device,
                &self.gpu.queue,
                &mut encoder,
                &mut self.image_atlas,
            );
            let mut scene = VelloHybridScenePainter::new(
                &mut self.scene,
                images,
                &mut texture_bindings,
                &self.gpu,
            );
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                frame.base_color,
                None,
                &Rect::new(0., 0., frame.width as f64, frame.height as f64),
            );
            for item in &frame.items {
                match item {
                    RenderItem::PushLayer {
                        path,
                        blend,
                        alpha,
                        effect,
                    } => {
                        scene.push_layer(
                            *blend,
                            *alpha,
                            Affine::scale(frame.scale_factor),
                            path,
                            effect.map(|effect| {
                                Arc::new(anyrender::Filter::single(match effect {
                                    Effect::Blur { radius } => {
                                        anyrender::filters::FilterEffect::blur(radius)
                                    }
                                    Effect::DropShadow {
                                        offset,
                                        blur,
                                        color,
                                    } => anyrender::filters::FilterEffect::drop_shadow(
                                        offset.x as f32,
                                        offset.y as f32,
                                        blur,
                                        color,
                                    ),
                                }))
                            }),
                            None,
                        );
                    }
                    RenderItem::PopLayer => scene.pop_layer(),
                    RenderItem::Text(text) => draw_text(&mut scene, text),
                    RenderItem::Layout { layout, transform } => {
                        draw_layout(*transform, layout, &mut scene)
                    }
                    RenderItem::Path {
                        path, fill, stroke, ..
                    } => {
                        let scale = Affine::scale(frame.scale_factor);
                        if let Some(brush) = fill {
                            scene.fill(Fill::EvenOdd, scale, BrushRef::from(brush), None, path);
                        }
                        if let Some((brush, style)) = stroke {
                            scene.stroke(style, scale, BrushRef::from(brush), None, path);
                        }
                    }
                    RenderItem::Svg {
                        resource_id,
                        content,
                        area,
                        fill,
                    } => draw_svg(
                        &mut scene,
                        &mut self.svg_scenes,
                        frame.scale_factor,
                        *resource_id,
                        content,
                        *area,
                        fill.as_ref(),
                    ),
                    RenderItem::Image { image, area } => scene.draw_image(
                        image.into(),
                        Affine::scale_non_uniform(
                            area.width as f64 / image.width as f64,
                            area.height as f64 / image.height as f64,
                        )
                        .then_translate(Vec2::new(area.x as f64, area.y as f64))
                        .then_scale(frame.scale_factor),
                    ),
                    RenderItem::Shadow {
                        rect,
                        color,
                        blur,
                        corner_rounding,
                    } => {
                        scene.draw_box_shadow(
                            Affine::IDENTITY,
                            *rect,
                            *color,
                            *corner_rounding,
                            *blur,
                        );
                    }
                }
            }
        }
        assert!(texture_bindings.is_empty());
        self.vello
            .render(
                &self.scene,
                &mut self.resources,
                &self.gpu.device,
                &self.gpu.queue,
                &mut encoder,
                &vello_hybrid::RenderSize {
                    width: frame.width,
                    height: frame.height,
                },
                self.target.as_ref().expect("Haven target"),
                &vello_hybrid::TextureBindings::new(),
            )
            .expect("paint Haven frame");
        self.gpu.queue.submit([encoder.finish()]);
        self.target.as_ref().expect("painted Haven target").clone()
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

fn draw_svg<S: PaintScene>(
    scene: &mut S,
    svg_scenes: &mut HashMap<u64, (String, Option<(Scene, Size)>)>,
    scale_factor: f64,
    cache_key: u64,
    content: &str,
    area: Area,
    fill: Option<&Brush>,
) {
    let Some((svg_scene, size)) = cached_svg_scene(svg_scenes, cache_key, content) else {
        return;
    };
    let width = size.width;
    let height = size.height;
    let rect = Rect::new(
        area.x as f64 * scale_factor,
        area.y as f64 * scale_factor,
        (area.x + area.width) as f64 * scale_factor,
        (area.y + area.height) as f64 * scale_factor,
    );
    if fill.is_some() {
        scene.push_layer(Mix::Normal, 1., Affine::IDENTITY, &rect, None, None);
    }
    scene.append_scene(
        svg_scene.clone(),
        Affine::scale_non_uniform(rect.width() / width, rect.height() / height)
            .then_translate(rect.origin().to_vec2()),
    );
    if let Some(fill) = fill {
        scene.push_layer(
            peniko::BlendMode {
                mix: Mix::Normal,
                compose: Compose::SrcIn,
            },
            1.,
            Affine::IDENTITY,
            &rect,
            None,
            None,
        );
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            BrushRef::from(fill),
            None,
            &rect,
        );
        scene.pop_layer();
        scene.pop_layer();
    }
}

fn cached_svg_scene<'a>(
    svg_scenes: &'a mut HashMap<u64, (String, Option<(Scene, Size)>)>,
    cache_key: u64,
    content: &str,
) -> Option<&'a (Scene, Size)> {
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
                Some((
                    svg_scene,
                    Size::new(size.width() as f64, size.height() as f64),
                ))
            }
        };

        match svg_scenes.entry(cache_key) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                *entry.get_mut() = (content.to_string(), cached_svg);
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert((content.to_string(), cached_svg));
            }
        }
    }

    svg_scenes.get(&cache_key).expect("cached svg").1.as_ref()
}

fn draw_layout<S: PaintScene>(transform: Affine, layout: &Layout<Brush>, scene: &mut S) {
    for line in layout.lines() {
        for item in line.items() {
            let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                continue;
            };
            let style = glyph_run.style();
            if let Some(underline) = &style.underline {
                let underline_brush = &style.brush;
                let run_metrics = glyph_run.run().metrics();
                let offset = match underline.offset {
                    Some(offset) => offset,
                    None => run_metrics.underline_offset,
                };
                let width = match underline.size {
                    Some(size) => size,
                    None => run_metrics.underline_size,
                };
                let y = glyph_run.baseline() - offset + width / 2.;

                let line = Line::new(
                    (glyph_run.offset() as f64, y as f64),
                    ((glyph_run.offset() + glyph_run.advance()) as f64, y as f64),
                );
                scene.stroke(
                    &Stroke::new(width.into()),
                    transform,
                    BrushRef::from(underline_brush),
                    None,
                    &line,
                );
            }
            let mut x = glyph_run.offset();
            let y = glyph_run.baseline();
            let run = glyph_run.run();
            let font = run.font();
            let font_size = run.font_size();
            let synthesis = run.synthesis();
            let glyph_xform = synthesis
                .skew()
                .map(|angle| Affine::skew(angle.to_radians().tan() as f64, 0.0));

            let brush = &style.brush;

            let glyphs: Vec<_> = glyph_run
                .glyphs()
                .map(|glyph| {
                    let gx = x + glyph.x;
                    let gy = y - glyph.y;
                    x += glyph.advance;
                    Glyph {
                        id: glyph.id as _,
                        x: gx,
                        y: gy,
                    }
                })
                .collect();
            scene.draw_glyphs(
                font,
                font_size,
                true,
                run.normalized_coords(),
                kurbo::Vec2::ZERO,
                Fill::NonZero,
                BrushRef::from(brush),
                1.0,
                transform,
                glyph_xform,
                glyphs.into_iter(),
            );
            if let Some(strikethrough) = &style.strikethrough {
                let strikethrough_brush = &style.brush;
                let run_metrics = glyph_run.run().metrics();
                let offset = match strikethrough.offset {
                    Some(offset) => offset,
                    None => run_metrics.strikethrough_offset,
                };
                let width = match strikethrough.size {
                    Some(size) => size,
                    None => run_metrics.strikethrough_size,
                };
                let y = glyph_run.baseline() - offset + run_metrics.strikethrough_size / 2.;

                let line = Line::new(
                    (glyph_run.offset() as f64, y as f64),
                    ((glyph_run.offset() + glyph_run.advance()) as f64, y as f64),
                );
                scene.stroke(
                    &Stroke::new(width.into()),
                    transform,
                    BrushRef::from(strikethrough_brush),
                    None,
                    &line,
                );
            }
        }
    }
}
