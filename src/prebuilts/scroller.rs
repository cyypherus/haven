use crate::{
    DEFAULT_CORNER_ROUNDING, TRANSPARENT, gesture,
    pane::{PaneState, View},
    rect,
    view::{Compositing, rounded_rect_path},
};
use backer::{
    Align, Area,
    nodes::{column, draw, empty, row, stack},
};
use std::time::Instant;

use super::scroll_feedback::{ScrollEdge, ScrollEdgeFeedback, scroll_edge_glows};

#[derive(Debug, Clone)]
pub(crate) struct ScrollerState {
    pub(crate) engine: ScrollEngine,
    dt: f32,
    area: Area,
    edge_feedback: ScrollEdgeFeedback,
}

impl Default for ScrollerState {
    fn default() -> Self {
        Self {
            engine: ScrollEngine::default(),
            dt: 0.,
            area: Area::default(),
            edge_feedback: ScrollEdgeFeedback::default(),
        }
    }
}

impl ScrollerState {
    fn update<'a, State>(
        &mut self,
        available_area: Area,
        ctx: &mut PaneState,
        id: u64,
        horizontal: bool,
        cell: &dyn Fn(usize, u64, &mut PaneState) -> Option<View<'a, State>>,
    ) -> bool {
        let area_changed = self.area != available_area;
        let extent = if horizontal {
            available_area.width
        } else {
            available_area.height
        };
        let mut measure = |index| {
            cell(index, id, ctx).map(|mut layout| {
                if horizontal {
                    layout.min_width(available_area, ctx)
                } else {
                    layout.min_height(available_area, ctx)
                }
                .unwrap_or(extent)
            })
        };
        let dt = std::mem::take(&mut self.dt);
        let edge_hit = self.engine.update(extent, area_changed, dt, &mut measure);
        self.area = available_area;
        let now = Instant::now();
        let edge = match (edge_hit, horizontal) {
            (EdgeHit::Start, true) => Some(ScrollEdge::Left),
            (EdgeHit::End, true) => Some(ScrollEdge::Right),
            (EdgeHit::Start, false) => Some(ScrollEdge::Top),
            (EdgeHit::End, false) => Some(ScrollEdge::Bottom),
            (EdgeHit::None, _) => None,
        };
        if let Some(edge) = edge {
            self.edge_feedback.pulse(edge, now);
        }
        self.edge_feedback.is_animating(now)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Element {
    extent: f32,
    index: usize,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ScrollEngine {
    visible_window: Vec<Element>,
    pub(crate) compensated: f32,
    pub(crate) offset: f32,
    viewport_extent: f32,
}

impl ScrollEngine {
    fn sum(&self) -> f32 {
        self.visible_window.iter().fold(0., |acc, e| acc + e.extent)
    }

    fn fill_forwards(
        &mut self,
        viewport_extent: f32,
        cell_extent: &mut dyn FnMut(usize) -> Option<f32>,
    ) {
        let mut current = self.sum();
        let mut index = self.visible_window.last().map(|l| l.index + 1).unwrap_or(0);
        while current + self.compensated < viewport_extent {
            let Some(h) = cell_extent(index) else { break };
            current += h;
            self.visible_window.push(Element { extent: h, index });
            index += 1;
        }
    }

    fn update(
        &mut self,
        viewport_extent: f32,
        area_changed: bool,
        dt: f32,
        cell_extent: &mut dyn FnMut(usize) -> Option<f32>,
    ) -> EdgeHit {
        let mut edge_hit = EdgeHit::None;

        // Always re-measure cached cell heights: content may have changed between frames.
        // Also handles area_changed (resize) by re-running layout at new width.
        if !self.visible_window.is_empty() {
            for i in 0..self.visible_window.len() {
                let idx = self.visible_window[i].index;
                let Some(h) = cell_extent(idx) else {
                    self.visible_window.truncate(i);
                    if i == 0 {
                        self.compensated = 0.;
                    }
                    break;
                };
                self.visible_window[i].extent = h;
            }
            self.fill_forwards(viewport_extent, cell_extent);
        }
        if area_changed && !self.visible_window.is_empty() {
            self.visible_window.drain(1..);
            self.fill_forwards(viewport_extent, cell_extent);
        }
        if self.visible_window.is_empty() {
            self.fill_forwards(viewport_extent, cell_extent);
        }

        self.compensated += dt;

        if dt < 0. {
            // Scrolling down: fetch cells at the end as they come into view.
            self.fill_forwards(viewport_extent, cell_extent);
        }

        let at_end = self
            .visible_window
            .last()
            .map(|l| cell_extent(l.index + 1).is_none())
            .unwrap_or(true);

        if dt > 0. || at_end && self.sum() < viewport_extent {
            // Scrolling up: prepend earlier cells as they come into view.
            while let Some(first) = self.visible_window.first().copied() {
                if first.index == 0
                    || self.compensated < 0. && (!at_end || self.sum() >= viewport_extent)
                {
                    break;
                }
                let Some(h) = cell_extent(first.index - 1) else {
                    break;
                };
                self.visible_window.insert(
                    0,
                    Element {
                        extent: h,
                        index: first.index - 1,
                    },
                );
                self.compensated -= h;
            }
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
                edge_hit = EdgeHit::Start;
            }
            self.compensated = 0.;
        }

        // Clamp at bottom: can't scroll past last cell.
        if at_end {
            let lower_bound = (viewport_extent - self.sum()).min(0.);
            if self.compensated < lower_bound {
                if dt < 0. {
                    edge_hit = EdgeHit::End;
                }
                self.compensated = lower_bound;
            }
        }

        // Pop cells that ended up fully off the top after clamping.
        while let Some(first) = self.visible_window.first()
            && first.extent <= -self.compensated
            && self.visible_window.len() > 1
        {
            let removed = self.visible_window.remove(0);
            self.compensated += removed.extent;
        }

        // Pop cells that ended up fully off the bottom after clamping.
        while self.visible_window.len() > 1
            && self.sum() - self.visible_window.last().map(|l| l.extent).unwrap_or(0.)
                + self.compensated
                >= viewport_extent
        {
            self.visible_window.pop();
        }

        self.offset = -(viewport_extent - self.sum()) * 0.5;
        self.viewport_extent = viewport_extent;

        edge_hit
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EdgeHit {
    None,
    Start,
    End,
}

pub enum Cells<F: Fn(usize, u64, &mut PaneState) -> R, R> {
    Row(F),
    Column(F),
}

pub fn scroller<'a, State: 'static>(
    id: u64,
    backing: Option<View<'a, State>>,
    cells: Cells<
        impl Fn(usize, u64, &mut PaneState) -> Option<View<'a, State>> + 'a,
        Option<View<'a, State>>,
    >,
    ctx: &mut PaneState,
) -> View<'a, State> {
    let scroll = gesture::scroll(crate::id!(id, 1u64));
    let (cell, horizontal, scroll) = match cells {
        Cells::Row(cell) => (cell, true, scroll.horizontal()),
        Cells::Column(cell) => (cell, false, scroll.vertical()),
    };
    stack(vec![
        backing.unwrap_or(empty()),
        rect(id)
            .corner_rounding(DEFAULT_CORNER_ROUNDING)
            .fill(TRANSPARENT)
            .view()
            .gesture(scroll.run(move |_s: &mut State, app: &mut PaneState, dt| {
                let entry = app.scrollers.entry(id).or_default();
                entry.dt += (if horizontal { dt.x } else { dt.y }) * 0.5;
            }))
            .build(ctx),
        draw(move |area, ctx: &mut PaneState| {
            let mut s = ctx.scrollers.remove(&id).unwrap_or_default();
            let animating = s.update::<State>(area, ctx, id, horizontal, &cell);
            if animating {
                ctx.needs_redraw = true;
            }
            let offset_total = s.engine.offset + s.engine.compensated;
            let visible: Vec<Element> = s.engine.visible_window.to_vec();
            let now = Instant::now();
            let edge_feedback = s.edge_feedback.clone();
            ctx.scrollers.insert(id, s);
            let mut cells = Vec::new();
            for element in &visible {
                if let Some(c) = cell(element.index, id, ctx) {
                    cells.push(if horizontal {
                        c.width(element.extent)
                    } else {
                        c.height(element.extent)
                    });
                }
            }
            let strip = if horizontal {
                row(cells).offset_x(offset_total).inert_x()
            } else {
                column(cells).offset_y(offset_total).inert_y()
            };
            let layout = stack(vec![
                strip,
                scroll_edge_glows::<State>(ctx, &edge_feedback, now),
            ])
            .align(Align::TopLeading);
            (if horizontal {
                layout.expand_x()
            } else {
                layout.expand_y()
            })
            .draw(area, ctx)
        })
        .expand()
        .clipped(|a| rounded_rect_path(a, DEFAULT_CORNER_ROUNDING)),
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

    #[test]
    fn short_content_does_not_scroll_down() {
        let mut engine = ScrollEngine::default();
        let hs = vec![20., 20.];
        step(&mut engine, 100., 0., &hs);
        assert_eq!(engine.visible_window.len(), 2);
        // scroll down
        step(&mut engine, 100., -50., &hs);
        assert_eq!(engine.compensated, 0., "short content must not scroll down");
        assert_eq!(engine.compensated, 0.);
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
