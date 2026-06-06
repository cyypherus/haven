use crate::pane::{PaneState, View};
use crate::utils::adjust_brush;
use crate::{Binding, ClickPhase, DragPhase, MouseButton, gesture, id, rect};
use crate::{DEFAULT_FG, DEFAULT_GRAY, DEFAULT_LIGHT_GRAY, TRANSPARENT, circle};
use backer::{
    Area,
    nodes::{draw, stack},
};
use peniko::Brush;
use std::rc::Rc;

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

type ViewFn<'a, State> = Rc<dyn Fn(ToggleState, Area, &mut PaneState) -> View<'a, State> + 'a>;
type OnToggle<State> = Rc<dyn Fn(&mut State, &mut PaneState, bool)>;

fn set_toggle_on<State>(
    state: &mut State,
    app: &mut PaneState,
    binding: &Binding<State, ToggleState>,
    on_toggle: &Option<OnToggle<State>>,
    on: bool,
) {
    if binding.get(state).on == on {
        return;
    }
    if let Some(f) = on_toggle {
        f(state, app, on);
    }
    binding.update(state, |s| s.on = on);
}

pub struct Toggle<'a, State> {
    id: u64,
    on_toggle: Option<OnToggle<State>>,
    state: ToggleState,
    binding: Binding<State, ToggleState>,
    knob: Option<ViewFn<'a, State>>,
    track: Option<ViewFn<'a, State>>,
}

pub fn toggle<'a, State>(
    id: u64,
    state: (&ToggleState, Binding<State, ToggleState>),
) -> Toggle<'a, State> {
    Toggle {
        id,
        on_toggle: None,
        state: *state.0,
        binding: state.1,
        knob: None,
        track: None,
    }
}

impl<'a, State> Toggle<'a, State> {
    pub fn on_toggle(
        mut self,
        on_toggle: impl Fn(&mut State, &mut PaneState, bool) + 'static,
    ) -> Self {
        self.on_toggle = Some(Rc::new(on_toggle));
        self
    }

    pub fn knob(
        mut self,
        f: impl Fn(ToggleState, Area, &mut PaneState) -> View<'a, State> + 'a,
    ) -> Self {
        self.knob = Some(Rc::new(f));
        self
    }

    pub fn track(
        mut self,
        f: impl Fn(ToggleState, Area, &mut PaneState) -> View<'a, State> + 'a,
    ) -> Self {
        self.track = Some(Rc::new(f));
        self
    }
    pub fn build(self, _ctx: &mut PaneState) -> View<'a, State>
    where
        State: 'static,
    {
        let state = self.state;
        let knob_fn = self.knob;
        let track_fn = self.track;
        let on_toggle = self.on_toggle;
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
                rect(id)
                    .fill(TRANSPARENT)
                    .view()
                    .gesture(gesture::hover(id!(id, 1u64)).run({
                        let binding = self.binding.clone();
                        move |state: &mut State, _app: &mut PaneState, h| {
                            binding.update(state, |s| s.hovered = h)
                        }
                    }))
                    .gesture(
                        gesture::click(id!(id, 2u64))
                            .button(MouseButton::Left | MouseButton::Right)
                            .run({
                                let binding = self.binding.clone();
                                let on_toggle = on_toggle.clone();
                                move |state: &mut State, app: &mut PaneState, event| match event
                                    .state
                                {
                                    ClickPhase::Started => {
                                        binding.update(state, |s| s.depressed = true)
                                    }
                                    ClickPhase::Cancelled => {
                                        binding.update(state, |s| s.depressed = false)
                                    }
                                    ClickPhase::Completed => {
                                        set_toggle_on(
                                            state,
                                            app,
                                            &binding,
                                            &on_toggle,
                                            !binding.get(state).on,
                                        );
                                        binding.update(state, |s| s.depressed = false)
                                    }
                                }
                            }),
                    )
                    .gesture(
                        gesture::drag(id!(id, 3u64))
                            .button(MouseButton::Left | MouseButton::Right)
                            .run({
                                let binding = self.binding.clone();
                                let on_toggle = on_toggle.clone();
                                move |state: &mut State, app: &mut PaneState, drag| {
                                    let (x, completed) = match drag {
                                        DragPhase::Began { .. } => {
                                            binding.update(state, |s| s.depressed = true);
                                            return;
                                        }
                                        DragPhase::Updated { current, .. } => (current.x, false),
                                        DragPhase::Completed { current, .. } => (current.x, true),
                                    };
                                    binding.update(state, |s| s.depressed = !completed);
                                    set_toggle_on(
                                        state,
                                        app,
                                        &binding,
                                        &on_toggle,
                                        x >= width as f64 * 0.5,
                                    );
                                }
                            }),
                    )
                    .build(ctx)
                    .height(height)
                    .width(width),
            ])
            .draw(area, ctx)
        })
    }
}
