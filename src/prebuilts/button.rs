use crate::DEFAULT_FG;
use crate::utils::adjust_brush;
use crate::{
    Binding, ClickPhase, DEFAULT_CORNER_ROUNDING, DEFAULT_FONT_SIZE, DEFAULT_PURP, MouseButton,
    gesture,
    pane::{PaneState, View},
    rect,
};
use backer::{Layout, nodes::stack};
use peniko::Brush;
use peniko::color::palette::css::TRANSPARENT;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, Default)]
pub struct ButtonState {
    pub hovered: bool,
    pub depressed: bool,
}

type ViewFn<'a, State> =
    Rc<dyn Fn(ButtonState, &mut PaneState) -> Layout<'a, View<State>, PaneState> + 'a>;

pub struct Button<'a, State> {
    id: u64,
    surface: Option<ViewFn<'a, State>>,
    label: Option<ViewFn<'a, State>>,
    text_label: Option<String>,
    on_click: Option<Rc<dyn Fn(&mut State, &mut PaneState)>>,
    state: ButtonState,
    binding: Binding<State, ButtonState>,
}

pub fn button<'a, State>(
    id: u64,
    state: (&ButtonState, Binding<State, ButtonState>),
) -> Button<'a, State> {
    Button {
        id,
        surface: None,
        label: None,
        text_label: None,
        on_click: None,
        state: *state.0,
        binding: state.1,
    }
}

impl<'a, State> Button<'a, State> {
    pub fn surface(
        mut self,
        f: impl Fn(ButtonState, &mut PaneState) -> Layout<'a, View<State>, PaneState> + 'a,
    ) -> Self {
        self.surface = Some(Rc::new(f));
        self
    }
    pub fn label(
        mut self,
        f: impl Fn(ButtonState, &mut PaneState) -> Layout<'a, View<State>, PaneState> + 'a,
    ) -> Self {
        self.label = Some(Rc::new(f));
        self
    }
    pub fn text_label(mut self, text_label: impl AsRef<str>) -> Self {
        self.text_label = Some(text_label.as_ref().to_string());
        self
    }
    pub fn on_click(mut self, on_click: impl Fn(&mut State, &mut PaneState) + 'static) -> Self {
        self.on_click = Some(Rc::new(on_click));
        self
    }
    pub fn build(self, ctx: &mut PaneState) -> Layout<'a, View<State>, PaneState>
    where
        State: 'static,
    {
        let btn_state = self.state;
        let surface_fn = self.surface;
        let label_fn = self.label;
        let text_label = self.text_label.unwrap_or_default();
        let id = self.id;

        let surface = if let Some(ref f) = surface_fn {
            f(btn_state, ctx)
        } else {
            rect(crate::id!(id))
                .fill(adjust_brush(
                    &Brush::Solid(DEFAULT_PURP),
                    btn_state.depressed,
                    btn_state.hovered,
                ))
                .corner_rounding(DEFAULT_CORNER_ROUNDING)
                .build(ctx)
        };

        let label = if let Some(ref f) = label_fn {
            f(btn_state, ctx)
        } else {
            crate::text(crate::id!(id), text_label.clone())
                .fill(adjust_brush(
                    &Brush::Solid(DEFAULT_FG),
                    btn_state.depressed,
                    btn_state.hovered,
                ))
                .font_size(DEFAULT_FONT_SIZE)
                .view()
                .build(ctx)
        };

        stack(vec![
            surface,
            label,
            rect(id)
                .fill(TRANSPARENT)
                .view()
                .gesture(gesture::hover(crate::id!(id, 1u64)).run({
                    let binding = self.binding.clone();
                    move |state, _app: &mut PaneState, h| binding.update(state, |s| s.hovered = h)
                }))
                .gesture(
                    gesture::click(crate::id!(id, 2u64))
                        .button(MouseButton::Left)
                        .run({
                            let binding = self.binding.clone();
                            let on_click = self.on_click.clone();
                            move |state: &mut State, app: &mut PaneState, event| match event.state {
                                ClickPhase::Started => {
                                    binding.update(state, |s| s.depressed = true)
                                }
                                ClickPhase::Cancelled => {
                                    binding.update(state, |s| s.depressed = false)
                                }
                                ClickPhase::Completed => {
                                    binding.update(state, |s| s.depressed = false);
                                    if let Some(f) = &on_click {
                                        f(state, app);
                                    }
                                }
                            }
                        }),
                )
                .build(ctx)
                .inert(),
        ])
    }
}
