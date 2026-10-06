use crate::Area;
use crate::draw_layout::draw_layout;
use crate::render::{Effect, Frame, RenderItem, TextRenderLayout};
use anyrender::{PaintScene, Scene};
use kurbo::{Affine, Rect, Size, Vec2};
use peniko::{self, Brush, BrushRef, Compose, Fill, Mix};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Default)]
pub(crate) struct Painter {
    svg_scenes: HashMap<u64, (String, Option<(Scene, Size)>)>,
}

impl Painter {
    pub(crate) fn paint(&mut self, frame: &Frame, scene: &mut impl PaintScene) {
        let svg_scenes = &mut self.svg_scenes;
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
                RenderItem::Text(text) => draw_text(scene, text),
                RenderItem::Layout { layout, transform } => draw_layout(*transform, layout, scene),
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
                    scene,
                    svg_scenes,
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
                    scene.draw_box_shadow(Affine::IDENTITY, *rect, *color, *corner_rounding, *blur);
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use anyrender::Scene;
    use anyrender::recording::RenderCommand;
    use kurbo::Shape;
    use std::collections::HashMap;

    const FIRST: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10"/></svg>"#;
    const SECOND: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="5"><rect width="20" height="5"/></svg>"#;

    #[test]
    fn paint_appends_a_frame_at_physical_scale() {
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
        let mut painter = Painter::default();
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
            "stabs: paint_commands={}, svg_cache_entries={}",
            scene.commands.len(),
            painter.svg_scenes.len()
        );
    }

    #[test]
    fn frame_freezes_path_and_brush_callbacks() {
        use std::{cell::Cell, rc::Rc};
        let mut calls = Rc::new(Cell::new((0, 0)));
        let mut pane = crate::PaneBuilder::new("test", |calls: &Rc<Cell<(u32, u32)>>, ctx| {
            let path_calls = calls.clone();
            let brush_calls = calls.clone();
            crate::path(1, move |area| {
                let (paths, brushes) = path_calls.get();
                path_calls.set((paths + 1, brushes));
                Rect::new(
                    area.x as f64,
                    area.y as f64,
                    (area.x + area.width) as f64,
                    (area.y + area.height) as f64,
                )
                .to_path(0.1)
            })
            .fill(move |_: Area, _: &()| {
                let (paths, brushes) = brush_calls.get();
                brush_calls.set((paths, brushes + 1));
                Brush::Solid(crate::Color::WHITE)
            })
            .build(ctx)
        })
        .build();
        let (frame, effects) = pane.redraw(&mut calls, 80, 40, 2.);
        assert!(effects.is_empty());
        assert_eq!(calls.get(), (1, 1));
        let mut painter = Painter::default();
        let mut scene = Scene::new();
        painter.paint(&frame, &mut scene);
        painter.paint(&frame, &mut scene);
        assert_eq!(calls.get(), (1, 1));
        assert_eq!(scene.commands.len(), 4);
        eprintln!("stabs: path_callbacks=1, brush_callbacks=1, paints=2, commands=4");
    }

    #[test]
    fn svg_cache_replaces_content_for_existing_key() {
        let mut cache = HashMap::new();
        for (content, size) in [
            (FIRST, (10., 10.)),
            (FIRST, (10., 10.)),
            (SECOND, (20., 5.)),
        ] {
            let (_, dimensions) = cached_svg_scene(&mut cache, 1, content).expect("valid SVG");
            assert_eq!((dimensions.width, dimensions.height), size);
            assert_eq!(cache.len(), 1);
            assert_eq!(cache[&1].0, content);
        }
        assert!(cached_svg_scene(&mut cache, 2, "invalid").is_none());
        assert_eq!(cache.len(), 2);
        assert!(cached_svg_scene(&mut cache, 2, FIRST).is_some());
        assert_eq!(cache.len(), 2);
        eprintln!(
            "stabs: svg_cache_entries=2, failed_svg_entries=0, invalid_dimension_sentinels=0"
        );
    }
}
