use crate::{
    DEFAULT_CORNER_ROUNDING, TRANSPARENT,
    app::{RootCtx, RootState, View},
    rect,
    view::{BlendMode, Compositing, rounded_rect_path},
};
use backer::{
    Align, Area, Layout,
    nodes::{column, draw, empty, stack, stack_aligned},
};
use lilt::{Animated, Easing};
use std::time::Instant;
use vello_svg::vello::peniko::{Brush, Gradient};

#[derive(Debug, Clone)]
pub(crate) struct ScrollerState {
    engine: ScrollEngine,
    dt: f32,
    area: Area,
    top_pulse: Animated<bool, Instant>,
    bottom_pulse: Animated<bool, Instant>,
}

impl Default for ScrollerState {
    fn default() -> Self {
        let pulse = || {
            Animated::new(false)
                .duration(EDGE_PULSE_MS)
                .easing(Easing::EaseOut)
        };
        Self {
            engine: ScrollEngine::default(),
            dt: 0.,
            area: Area::default(),
            top_pulse: pulse(),
            bottom_pulse: pulse(),
        }
    }
}

const EDGE_PULSE_MS: f32 = 200.;

impl ScrollerState {
    fn update<'a, State>(
        &mut self,
        available_area: Area,
        ctx: &mut RootCtx,
        id: u64,
        cell: &dyn Fn(usize, u64, &mut RootCtx) -> Option<Layout<'a, View<State>, RootCtx>>,
    ) -> bool {
        let area_changed = self.area != available_area;
        let mut height_of =
            |index: usize| cell_height::<State>(ctx, index, id, available_area, cell);
        let dt = std::mem::replace(&mut self.dt, 0.);
        let edge_hit = self
            .engine
            .update(available_area.height, area_changed, dt, &mut height_of);
        self.area = available_area;
        let now = Instant::now();
        self.top_pulse.transition(edge_hit == EdgeHit::Top, now);
        self.bottom_pulse
            .transition(edge_hit == EdgeHit::Bottom, now);
        self.top_pulse.in_progress(now) || self.bottom_pulse.in_progress(now)
    }

    fn offset_total(&self) -> f32 {
        self.engine.offset + self.engine.compensated
    }

    fn visible_window(&self) -> &[Element] {
        &self.engine.visible_window
    }

    fn top_pulse_value(&self, now: Instant) -> f32 {
        self.top_pulse.animate(|v| if v { 1. } else { 0. }, now)
    }

    fn bottom_pulse_value(&self, now: Instant) -> f32 {
        self.bottom_pulse.animate(|v| if v { 1. } else { 0. }, now)
    }
}

fn cell_height<'a, State>(
    ctx: &mut RootCtx,
    index: usize,
    id: u64,
    available_area: Area,
    cell: &dyn Fn(usize, u64, &mut RootCtx) -> Option<Layout<'a, View<State>, RootCtx>>,
) -> Option<f32> {
    cell(index, id, ctx).map(|mut layout| {
        layout
            .min_height(available_area, ctx)
            .unwrap_or(available_area.height)
    })
}

fn edge_glow<'a, State: 'static>(
    ctx: &mut RootCtx,
    top: bool,
    alpha: f32,
) -> Layout<'a, View<State>, RootCtx> {
    if alpha <= 0.01 {
        return empty();
    }
    let alpha = alpha.clamp(0., 1.) * EDGE_GLOW_MAX_ALPHA;
    let align = if top { Align::Top } else { Align::Bottom };
    let r = rect(crate::id!())
        .fill(move |area: Area, _: &()| {
            let y0 = area.y as f64;
            let y1 = y0 + area.height as f64;
            let (p1, p2) = if top {
                ((0., y0), (0., y1))
            } else {
                ((0., y1), (0., y0))
            };
            Brush::Gradient(Gradient::new_linear(p1, p2).with_stops([
                EDGE_GLOW_COLOR.with_alpha(alpha),
                EDGE_GLOW_COLOR.with_alpha(0.),
            ]))
        })
        .corner_rounding(0.)
        .build(ctx)
        .height(EDGE_GLOW_HEIGHT);
    stack_aligned(align, vec![r]).expand_y()
}

const EDGE_GLOW_HEIGHT: f32 = 24.;
const EDGE_GLOW_MAX_ALPHA: f32 = 0.35;
const EDGE_GLOW_COLOR: crate::Color = crate::Color::WHITE;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Element {
    height: f32,
    index: usize,
}

#[derive(Debug, Clone, Default)]
struct ScrollEngine {
    visible_window: Vec<Element>,
    compensated: f32,
    offset: f32,
    available_height: f32,
}

impl ScrollEngine {
    fn sum(&self) -> f32 {
        self.visible_window.iter().fold(0., |acc, e| acc + e.height)
    }

    fn fill_forwards(
        &mut self,
        available_height: f32,
        cell_height: &mut dyn FnMut(usize) -> Option<f32>,
    ) {
        let mut current = self.sum();
        let mut index = self.visible_window.last().map(|l| l.index + 1).unwrap_or(0);
        while current + self.compensated < available_height {
            let Some(h) = cell_height(index) else { break };
            current += h;
            self.visible_window.push(Element { height: h, index });
            index += 1;
        }
    }

    fn update(
        &mut self,
        available_height: f32,
        area_changed: bool,
        dt: f32,
        cell_height: &mut dyn FnMut(usize) -> Option<f32>,
    ) -> EdgeHit {
        let mut edge_hit = EdgeHit::None;

        // Always re-measure cached cell heights: content may have changed between frames.
        // Also handles area_changed (resize) by re-running layout at new width.
        if !self.visible_window.is_empty() {
            for i in 0..self.visible_window.len() {
                let idx = self.visible_window[i].index;
                if let Some(h) = cell_height(idx) {
                    self.visible_window[i].height = h;
                }
            }
            self.fill_forwards(available_height, cell_height);
        }
        if area_changed && !self.visible_window.is_empty() {
            self.visible_window.drain(1..);
            self.fill_forwards(available_height, cell_height);
        }
        if self.visible_window.is_empty() {
            self.fill_forwards(available_height, cell_height);
        }

        self.compensated += dt;

        if dt > 0. {
            // Scrolling up: prepend earlier cells as they come into view.
            loop {
                let Some(first) = self.visible_window.first().copied() else {
                    break;
                };
                if first.index == 0 || self.compensated < 0. {
                    break;
                }
                let Some(h) = cell_height(first.index - 1) else {
                    break;
                };
                self.visible_window.insert(
                    0,
                    Element {
                        height: h,
                        index: first.index - 1,
                    },
                );
                self.compensated -= h;
            }
        } else if dt < 0. {
            // Scrolling down: fetch cells at the end as they come into view.
            self.fill_forwards(available_height, cell_height);
        }

        // Clamp at top: can't scroll past first cell.
        if self
            .visible_window
            .first()
            .map(|f| f.index == 0)
            .unwrap_or(false)
            && self.compensated > 0.
        {
            if dt > 0. {
                edge_hit = EdgeHit::Top;
            }
            self.compensated = 0.;
        }

        // Clamp at bottom: can't scroll past last cell.
        let at_end = self
            .visible_window
            .last()
            .map(|l| cell_height(l.index + 1).is_none())
            .unwrap_or(true);
        if at_end {
            let lower_bound = (available_height - self.sum()).min(0.);
            if self.compensated < lower_bound {
                if dt < 0. {
                    edge_hit = EdgeHit::Bottom;
                }
                self.compensated = lower_bound;
            }
        }

        // Pop cells that ended up fully off the top after clamping.
        while let Some(first) = self.visible_window.first()
            && first.height <= -self.compensated
            && self.visible_window.len() > 1
        {
            let removed = self.visible_window.remove(0);
            self.compensated += removed.height;
        }

        // Pop cells that ended up fully off the bottom after clamping.
        while self.visible_window.len() > 1
            && self.sum() - self.visible_window.last().map(|l| l.height).unwrap_or(0.)
                + self.compensated
                >= available_height
        {
            self.visible_window.pop();
        }

        self.offset = -(available_height - self.sum()) * 0.5;
        self.available_height = available_height;

        edge_hit
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EdgeHit {
    None,
    Top,
    Bottom,
}

pub fn scroller<'a, State: 'static>(
    id: u64,
    backing: Option<Layout<'a, View<State>, RootCtx>>,
    cell: impl Fn(usize, u64, &mut RootCtx) -> Option<Layout<'a, View<State>, RootCtx>> + 'a,
    ctx: &mut RootCtx,
) -> Layout<'a, View<State>, RootCtx> {
    stack(vec![
        backing.unwrap_or(empty()),
        draw(move |area, ctx: &mut RootCtx| {
            let mut s = ctx.scrollers.remove(&id).unwrap_or_default();
            let animating = s.update::<State>(area, ctx, id, &cell);
            if animating {
                ctx.needs_redraw = true;
            }
            let offset_total = s.offset_total();
            let visible: Vec<Element> = s.visible_window().to_vec();
            let now = Instant::now();
            let top_alpha = s.top_pulse_value(now);
            let bottom_alpha = s.bottom_pulse_value(now);
            ctx.scrollers.insert(id, s);
            let mut cells = Vec::new();
            for element in &visible {
                if let Some(c) = cell(element.index, id, ctx) {
                    cells.push(c.height(element.height));
                }
            }
            let column_layout = column(cells).offset_y(offset_total).inert_y();
            stack(vec![
                column_layout,
                stack(vec![
                    edge_glow::<State>(ctx, true, top_alpha),
                    edge_glow::<State>(ctx, false, bottom_alpha),
                ])
                .expand_y()
                .blend(BlendMode::Screen),
            ])
            .expand_y()
            .draw(area, ctx)
        })
        .expand()
        .clipped(|a| rounded_rect_path(a, DEFAULT_CORNER_ROUNDING)),
        rect(crate::id!(id))
            .corner_rounding(DEFAULT_CORNER_ROUNDING)
            .fill(TRANSPARENT)
            .view()
            .on_scroll(move |_s: &mut State, app: &mut RootState, dt| {
                let entry = app.app_context.scrollers.entry(id).or_default();
                entry.dt += dt.y * 0.5;
            })
            .finish(ctx),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn heights(hs: Vec<f32>) -> impl FnMut(usize) -> Option<f32> {
        move |i| hs.get(i).copied()
    }

    fn step(engine: &mut ScrollEngine, available: f32, dt: f32, hs: &[f32]) {
        let hs = hs.to_vec();
        let mut height_of = heights(hs);
        engine.update(available, false, dt, &mut height_of);
    }

    fn content_top(engine: &ScrollEngine, available: f32) -> f32 {
        // In the scroller, cells are drawn via column(cells).offset_y(offset + compensated).
        // column is center-aligned by default, so content y = area.y + (area.h - sum)/2 + offset_total.
        // With offset = -(area.h - sum)/2, content y = area.y + compensated.
        let _ = available;
        engine.compensated
    }

    #[test]
    fn short_content_does_not_scroll_down() {
        let mut engine = ScrollEngine::default();
        let hs = vec![20., 20.];
        step(&mut engine, 100., 0., &hs);
        assert_eq!(engine.visible_window.len(), 2);
        // scroll down
        step(&mut engine, 100., -50., &hs);
        assert_eq!(engine.compensated, 0., "short content must not scroll down");
        assert_eq!(content_top(&engine, 100.), 0.);
    }

    #[test]
    fn short_content_does_not_scroll_up() {
        let mut engine = ScrollEngine::default();
        let hs = vec![20., 20.];
        step(&mut engine, 100., 0., &hs);
        step(&mut engine, 100., 50., &hs);
        assert_eq!(engine.compensated, 0., "short content must not scroll up");
    }

    #[test]
    fn exactly_fit_content_does_not_scroll() {
        let mut engine = ScrollEngine::default();
        let hs = vec![50., 50.];
        step(&mut engine, 100., 0., &hs);
        step(&mut engine, 100., -30., &hs);
        assert_eq!(engine.compensated, 0.);
        step(&mut engine, 100., 30., &hs);
        assert_eq!(engine.compensated, 0.);
    }

    #[test]
    fn tall_content_scrolls_down_and_clamps_at_bottom() {
        let mut engine = ScrollEngine::default();
        let hs = vec![60., 60., 60.]; // total 180, area 100
        step(&mut engine, 100., 0., &hs);
        // scroll way down
        step(&mut engine, 100., -500., &hs);
        // at bottom, last cell's bottom == area bottom.
        // visible_window should end with the final cell, and compensated bounded.
        let last_idx = engine.visible_window.last().unwrap().index;
        assert_eq!(last_idx, 2);
        // Content visible covers [compensated, compensated + sum(visible)]
        // At bottom, compensated + sum_visible == area.height
        let total = engine.sum();
        assert!(
            (engine.compensated + total - 100.).abs() < 0.01,
            "at bottom, content bottom should equal area bottom; comp={} sum={}",
            engine.compensated,
            total
        );
    }

    #[test]
    fn tall_content_scrolls_up_and_clamps_at_top() {
        let mut engine = ScrollEngine::default();
        let hs = vec![60., 60., 60.];
        step(&mut engine, 100., 0., &hs);
        step(&mut engine, 100., -500., &hs); // go to bottom
        step(&mut engine, 100., 500., &hs); // back to top
        assert_eq!(engine.visible_window.first().unwrap().index, 0);
        assert_eq!(engine.compensated, 0.);
    }

    #[test]
    fn empty_content_stays_empty() {
        let mut engine = ScrollEngine::default();
        let hs: Vec<f32> = vec![];
        step(&mut engine, 100., 0., &hs);
        assert!(engine.visible_window.is_empty());
        step(&mut engine, 100., -50., &hs);
        assert_eq!(engine.compensated, 0.);
    }

    #[test]
    fn resize_rebuilds_visible_window() {
        let mut engine = ScrollEngine::default();
        let hs = vec![30., 30., 30., 30., 30.];
        step(&mut engine, 100., 0., &hs);
        let initial_count = engine.visible_window.len();
        // simulate area change
        let mut height_of = heights(hs.clone());
        engine.update(200., true, 0., &mut height_of);
        assert!(engine.visible_window.len() >= initial_count);
    }

    #[test]
    fn scroll_then_return_is_stable() {
        let mut engine = ScrollEngine::default();
        let hs = vec![40., 40., 40., 40.]; // 160 total, area 100
        step(&mut engine, 100., 0., &hs);
        step(&mut engine, 100., -30., &hs);
        step(&mut engine, 100., 30., &hs);
        assert_eq!(engine.visible_window.first().unwrap().index, 0);
        assert!(engine.compensated.abs() < 0.01);
    }
}
