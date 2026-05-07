use crate::app::{PaneState, View};
use crate::{Binding, ClickState, adjust_brush, id, rect};
use crate::{DEFAULT_FG, DEFAULT_GRAY, DEFAULT_LIGHT_GRAY, TRANSPARENT, circle};
use backer::{
    Area, Layout,
    nodes::{draw, stack},
};
use std::rc::Rc;
use vello_svg::vello::peniko::Brush;

#[derive(Default, Debug, Clone, Copy)]
pub struct ToggleState {
    pub hovered: bool,
    pub depressed: bool,
    pub on: bool,
}

impl ToggleState {
    pub fn on() -> Self {
        ToggleState {
            hovered: false,
            depressed: false,
            on: true,
        }
    }

    pub fn off() -> Self {
        ToggleState {
            hovered: false,
            depressed: false,
            on: false,
        }
    }
}

type ViewFn<'a, State> =
    Rc<dyn Fn(ToggleState, Area, &mut PaneState) -> Layout<'a, View<State>, PaneState> + 'a>;

pub struct Toggle<'a, State> {
    id: u64,
    on_toggle: Option<fn(&mut State, &mut PaneState, bool)>,
    state: ToggleState,
    binding: Binding<State, ToggleState>,
    knob: Option<ViewFn<'a, State>>,
    track: Option<ViewFn<'a, State>>,
}

pub fn toggle<'a, State>(
    id: u64,
    state: (ToggleState, Binding<State, ToggleState>),
) -> Toggle<'a, State> {
    Toggle {
        id,
        on_toggle: None,
        state: state.0,
        binding: state.1,
        knob: None,
        track: None,
    }
}

impl<'a, State> Toggle<'a, State> {
    pub fn on_toggle(mut self, on_toggle: fn(&mut State, &mut PaneState, bool)) -> Self {
        self.on_toggle = Some(on_toggle);
        self
    }

    pub fn knob(
        mut self,
        f: impl Fn(ToggleState, Area, &mut PaneState) -> Layout<'a, View<State>, PaneState> + 'a,
    ) -> Self {
        self.knob = Some(Rc::new(f));
        self
    }

    pub fn track(
        mut self,
        f: impl Fn(ToggleState, Area, &mut PaneState) -> Layout<'a, View<State>, PaneState> + 'a,
    ) -> Self {
        self.track = Some(Rc::new(f));
        self
    }
    pub fn build(self, _ctx: &mut PaneState) -> Layout<'a, View<State>, PaneState>
    where
        State: 'static,
    {
        let state = self.state;
        let knob_fn = self.knob;
        let track_fn = self.track;
        let id = self.id;
        draw(move |area, ctx: &mut PaneState| {
            let width = area.width;
            let height = area.height;

            let track = if let Some(ref f) = track_fn {
                f(state, area, ctx)
            } else {
                rect(id!(id))
                    .fill(if state.on {
                        Brush::Solid(DEFAULT_LIGHT_GRAY)
                    } else {
                        Brush::Solid(DEFAULT_GRAY)
                    })
                    .corner_rounding(height * 0.5)
                    .build(ctx)
                    .height(height)
                    .width(width)
            };

            let knob_inner = if let Some(ref f) = knob_fn {
                f(state, area, ctx)
            } else {
                let knob_brush =
                    adjust_brush(&Brush::Solid(DEFAULT_FG), state.depressed, state.hovered);
                circle(id!(id)).fill(knob_brush).finish(ctx)
            };
            let knob = knob_inner
                .pad(height * 0.1)
                .height(height)
                .width(height)
                .offset(
                    {
                        let button_padding = height * 0.5;
                        if state.on {
                            (width * 0.5) - button_padding
                        } else {
                            (-width * 0.5) + button_padding
                        }
                    },
                    0.,
                );

            stack(vec![
                track,
                knob,
                rect(crate::id!(id))
                    .fill(TRANSPARENT)
                    .view()
                    .on_hover({
                        let binding = self.binding.clone();
                        move |state: &mut State, _app: &mut PaneState, h| {
                            binding.update(state, |s| s.hovered = h)
                        }
                    })
                    .on_click({
                        let binding = self.binding.clone();
                        move |state: &mut State, app: &mut PaneState, click_state, _| {
                            match click_state {
                                ClickState::Started => {
                                    binding.update(state, |s| s.depressed = true)
                                }
                                ClickState::Cancelled => {
                                    binding.update(state, |s| s.depressed = false)
                                }
                                ClickState::Completed => {
                                    if let Some(f) = self.on_toggle {
                                        f(state, app, !binding.get(state).on);
                                    }
                                    binding.update(state, |s| {
                                        s.on = !s.on;
                                        s.depressed = false
                                    })
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
