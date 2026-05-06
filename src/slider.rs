use crate::{
    Binding, DEFAULT_DARK_GRAY, DEFAULT_FG, DEFAULT_GRAY, DEFAULT_PURP, DragState, TRANSPARENT,
    adjust_brush,
    app::{RootCtx, RootState, View},
    circle, id, rect,
};
use backer::{
    Area, Layout,
    nodes::{draw, stack},
};
use std::rc::Rc;
use vello_svg::vello::peniko::Brush;

#[derive(Debug, Clone, Copy, Default)]
pub struct SliderState {
    pub hovered: bool,
    pub dragging: bool,
    pub value: f32,
}

type ViewFn<'a, State> =
    Rc<dyn Fn(SliderState, Area, &mut RootCtx) -> Layout<'a, View<State>, RootCtx> + 'a>;

pub struct Slider<'a, State> {
    id: u64,
    state: SliderState,
    binding: Binding<State, SliderState>,
    min: f32,
    max: f32,
    on_change: Option<Rc<dyn Fn(&mut State, &mut RootState, f32)>>,
    knob: Option<ViewFn<'a, State>>,
    track: Option<ViewFn<'a, State>>,
    traveled_track: Option<ViewFn<'a, State>>,
    background: Option<ViewFn<'a, State>>,
}

pub fn slider<'a, State>(
    id: u64,
    state: (SliderState, Binding<State, SliderState>),
) -> Slider<'a, State> {
    Slider {
        id,
        state: state.0,
        binding: state.1,
        min: 0.0,
        max: 1.0,
        on_change: None,
        knob: None,
        track: None,
        traveled_track: None,
        background: None,
    }
}

impl<'a, State> Slider<'a, State> {
    pub fn range(mut self, min: f32, max: f32) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    pub fn on_change(
        mut self,
        on_change: impl Fn(&mut State, &mut RootState, f32) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(on_change));
        self
    }

    pub fn knob(
        mut self,
        f: impl Fn(SliderState, Area, &mut RootCtx) -> Layout<'a, View<State>, RootCtx> + 'a,
    ) -> Self {
        self.knob = Some(Rc::new(f));
        self
    }

    pub fn track(
        mut self,
        f: impl Fn(SliderState, Area, &mut RootCtx) -> Layout<'a, View<State>, RootCtx> + 'a,
    ) -> Self {
        self.track = Some(Rc::new(f));
        self
    }

    pub fn traveled_track(
        mut self,
        f: impl Fn(SliderState, Area, &mut RootCtx) -> Layout<'a, View<State>, RootCtx> + 'a,
    ) -> Self {
        self.traveled_track = Some(Rc::new(f));
        self
    }

    pub fn background(
        mut self,
        f: impl Fn(SliderState, Area, &mut RootCtx) -> Layout<'a, View<State>, RootCtx> + 'a,
    ) -> Self {
        self.background = Some(Rc::new(f));
        self
    }

    pub fn build(self, _ctx: &mut RootCtx) -> Layout<'a, View<State>, RootCtx>
    where
        State: 'static,
    {
        let state = self.state;
        let knob_fn = self.knob;
        let track_fn = self.track;
        let traveled_track_fn = self.traveled_track;
        let background_fn = self.background;
        let id = self.id;
        draw(move |area, ctx: &mut RootCtx| {
            let width = area.width;
            let height = area.height;
            let normalized_value = (state.value - self.min) / (self.max - self.min);
            let slider_width = (width - height) * normalized_value + height;

            let bg = if let Some(ref f) = background_fn {
                f(state, area, ctx)
            } else {
                rect(id!(id))
                    .fill(Brush::Solid(DEFAULT_GRAY))
                    .corner_rounding(height * 0.5)
                    .build(ctx)
                    .height(height)
                    .width(width)
            };

            let track = if let Some(ref f) = track_fn {
                f(state, area, ctx)
            } else {
                rect(id!(id))
                    .fill(Brush::Solid(DEFAULT_DARK_GRAY))
                    .corner_rounding(height)
                    .build(ctx)
            };

            let traveled = if let Some(ref f) = traveled_track_fn {
                f(state, area, ctx)
            } else {
                rect(id!(id))
                    .fill(Brush::Solid(DEFAULT_PURP))
                    .corner_rounding(height)
                    .build(ctx)
            };

            let knob = if let Some(ref f) = knob_fn {
                f(state, area, ctx)
            } else {
                let knob_brush =
                    adjust_brush(&Brush::Solid(DEFAULT_FG), state.dragging, state.hovered);
                circle(id!(id)).fill(knob_brush).finish(ctx)
            };

            stack(vec![
                bg,
                track.pad(height * 0.3).height(height).width(width),
                traveled
                    .pad(height * 0.2)
                    .height(height)
                    .width(slider_width)
                    .offset((-width * 0.5) + (slider_width * 0.5), 0.),
                knob.pad(height * 0.1)
                    .height(if state.dragging { height * 1.1 } else { height })
                    .width(height)
                    .offset((-width * 0.5) + slider_width - (height * 0.5), 0.),
                rect(id!(id))
                    .fill(TRANSPARENT)
                    .view()
                    .on_hover({
                        let binding = self.binding.clone();
                        move |state: &mut State, _app: &mut RootState, h| {
                            binding.update(state, |s| s.hovered = h)
                        }
                    })
                    .on_drag({
                        let binding = self.binding.clone();
                        let min = self.min;
                        let max = self.max;
                        let on_change = self.on_change.clone();
                        move |state: &mut State, app: &mut RootState, drag_state| {
                            let gesture_padding = height / width;
                            let update_value = |x: f64| {
                                let padded_start = gesture_padding * width;
                                let padded_end = width - (gesture_padding * width);
                                let padded_width = padded_end - padded_start;
                                let normalized = ((x - padded_start as f64) / padded_width as f64)
                                    .clamp(0.0, 1.0)
                                    as f32;
                                min + normalized * (max - min)
                            };

                            match drag_state {
                                DragState::Began { start, .. } => {
                                    binding.update(state, |s| s.dragging = true);
                                    let new_value = update_value(start.x);
                                    binding.update(state, |s| s.value = new_value);
                                    if let Some(ref f) = on_change {
                                        f(state, app, new_value);
                                    }
                                }
                                DragState::Updated { current, .. } => {
                                    let new_value = update_value(current.x);
                                    binding.update(state, |s| s.value = new_value);
                                    if let Some(ref f) = on_change {
                                        f(state, app, new_value);
                                    }
                                }
                                DragState::Completed { .. } => {
                                    binding.update(state, |s| s.dragging = false);
                                }
                            }
                        }
                    })
                    .finish(ctx)
                    .height(height)
                    .width(width),
            ])
            .draw(area, ctx)
        })
    }
}
