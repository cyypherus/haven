use crate::{
    pane::{PaneState, View},
    rect,
    view::{BlendMode, Compositing},
};
use backer::{
    Align, Area,
    nodes::{empty, stack, stack_aligned},
};
use peniko::{Brush, Gradient};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScrollEdge {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ScrollEdgeFeedback {
    top: Option<Instant>,
    bottom: Option<Instant>,
    left: Option<Instant>,
    right: Option<Instant>,
}

impl ScrollEdgeFeedback {
    pub(crate) fn pulse(&mut self, edge: ScrollEdge, now: Instant) {
        *self.hit_mut(edge) = Some(now);
    }

    pub(crate) fn pulse_all(&mut self, edges: &[ScrollEdge], now: Instant) {
        for edge in edges {
            self.pulse(*edge, now);
        }
    }

    pub(crate) fn alpha(&self, edge: ScrollEdge, now: Instant) -> f32 {
        let Some(hit) = self.hit(edge) else {
            return 0.;
        };
        let progress = now.saturating_duration_since(hit).as_secs_f32() * 1000. / EDGE_PULSE_MS;
        (1. - progress).clamp(0., 1.)
    }

    pub(crate) fn is_animating(&self, now: Instant) -> bool {
        [
            ScrollEdge::Top,
            ScrollEdge::Bottom,
            ScrollEdge::Left,
            ScrollEdge::Right,
        ]
        .into_iter()
        .any(|edge| self.alpha(edge, now) > 0.01)
    }

    fn hit(&self, edge: ScrollEdge) -> Option<Instant> {
        match edge {
            ScrollEdge::Top => self.top,
            ScrollEdge::Bottom => self.bottom,
            ScrollEdge::Left => self.left,
            ScrollEdge::Right => self.right,
        }
    }

    fn hit_mut(&mut self, edge: ScrollEdge) -> &mut Option<Instant> {
        match edge {
            ScrollEdge::Top => &mut self.top,
            ScrollEdge::Bottom => &mut self.bottom,
            ScrollEdge::Left => &mut self.left,
            ScrollEdge::Right => &mut self.right,
        }
    }
}

pub(crate) fn scroll_edge_glows<'a, State: 'static>(
    ctx: &mut PaneState,
    feedback: &ScrollEdgeFeedback,
    now: Instant,
) -> View<'a, State> {
    if feedback.is_animating(now) {
        ctx.needs_redraw = true;
    }
    stack(vec![
        scroll_edge_glow::<State>(ctx, ScrollEdge::Top, feedback.alpha(ScrollEdge::Top, now)),
        scroll_edge_glow::<State>(
            ctx,
            ScrollEdge::Bottom,
            feedback.alpha(ScrollEdge::Bottom, now),
        ),
        scroll_edge_glow::<State>(ctx, ScrollEdge::Left, feedback.alpha(ScrollEdge::Left, now)),
        scroll_edge_glow::<State>(
            ctx,
            ScrollEdge::Right,
            feedback.alpha(ScrollEdge::Right, now),
        ),
    ])
    .expand()
    .blend(BlendMode::Screen)
}

fn scroll_edge_glow<'a, State: 'static>(
    ctx: &mut PaneState,
    edge: ScrollEdge,
    alpha: f32,
) -> View<'a, State> {
    if alpha <= 0.01 {
        return empty();
    }
    let alpha = alpha.clamp(0., 1.) * EDGE_GLOW_MAX_ALPHA;
    let r = rect(crate::id!())
        .fill(move |area: Area, _: &()| {
            let x0 = area.x as f64;
            let y0 = area.y as f64;
            let x1 = x0 + area.width as f64;
            let y1 = y0 + area.height as f64;
            let (p1, p2) = match edge {
                ScrollEdge::Top => ((0., y0), (0., y1)),
                ScrollEdge::Bottom => ((0., y1), (0., y0)),
                ScrollEdge::Left => ((x0, 0.), (x1, 0.)),
                ScrollEdge::Right => ((x1, 0.), (x0, 0.)),
            };
            Brush::Gradient(Gradient::new_linear(p1, p2).with_stops([
                EDGE_GLOW_COLOR.with_alpha(alpha),
                EDGE_GLOW_COLOR.with_alpha(0.),
            ]))
        })
        .corner_rounding(0.)
        .build(ctx);
    match edge {
        ScrollEdge::Top => stack_aligned(Align::Top, vec![r.height(EDGE_GLOW_SIZE)]).expand(),
        ScrollEdge::Bottom => stack_aligned(Align::Bottom, vec![r.height(EDGE_GLOW_SIZE)]).expand(),
        ScrollEdge::Left => stack_aligned(Align::Leading, vec![r.width(EDGE_GLOW_SIZE)]).expand(),
        ScrollEdge::Right => stack_aligned(Align::Trailing, vec![r.width(EDGE_GLOW_SIZE)]).expand(),
    }
}

const EDGE_PULSE_MS: f32 = 200.;
const EDGE_GLOW_SIZE: f32 = 24.;
const EDGE_GLOW_MAX_ALPHA: f32 = 0.35;
const EDGE_GLOW_COLOR: crate::Color = crate::Color::WHITE;
