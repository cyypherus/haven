use crate::pane::{PaneState, View};
use crate::utils::adjust_brush;
use crate::{
    Binding, ClickPhase, DEFAULT_CORNER_ROUNDING, DEFAULT_GRAY, MouseButton, gesture, rect,
};
use crate::{Color, TRANSPARENT};
use backer::{Align, nodes::*};
use kurbo::Stroke;
use peniko::Brush;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct DropdownState<T> {
    pub selected: T,
    pub hovered: Option<usize>,
    pub expanded: bool,
    pub depressed: bool,
}

impl<T: Default> Default for DropdownState<T> {
    fn default() -> Self {
        Self {
            selected: T::default(),
            hovered: None,
            expanded: false,
            depressed: false,
        }
    }
}

pub struct DropdownItemCtx<'a, T> {
    pub index: usize,
    pub value: &'a T,
    pub selected: bool,
    pub hovered: bool,
    pub expanded: bool,
}

pub struct DropDown<'a, State, T> {
    id: u64,
    state: &'a DropdownState<T>,
    binding: Binding<State, DropdownState<T>>,
    options: Vec<T>,
    view_fn: Rc<dyn Fn(DropdownItemCtx<T>, &mut PaneState) -> View<'a, State> + 'a>,
    background: Option<Rc<dyn Fn(&DropdownState<T>, &mut PaneState) -> View<'a, State> + 'a>>,
    on_select: Option<Rc<dyn Fn(&mut State, &mut PaneState, &T)>>,
}

pub fn dropdown<'a, State, T: Clone + PartialEq + 'static>(
    id: u64,
    state: (&'a DropdownState<T>, Binding<State, DropdownState<T>>),
    options: Vec<T>,
    view_fn: impl Fn(DropdownItemCtx<T>, &mut PaneState) -> View<'a, State> + 'a,
) -> DropDown<'a, State, T> {
    DropDown {
        id,
        state: state.0,
        binding: state.1,
        options,
        view_fn: Rc::new(view_fn),
        background: None,
        on_select: None,
    }
}

impl<'a, State, T: Clone + PartialEq + 'static> DropDown<'a, State, T> {
    pub fn background(
        mut self,
        f: impl Fn(&DropdownState<T>, &mut PaneState) -> View<'a, State> + 'a,
    ) -> Self {
        self.background = Some(Rc::new(f));
        self
    }

    pub fn on_select(
        mut self,
        on_select: impl Fn(&mut State, &mut PaneState, &T) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(on_select));
        self
    }

    pub fn build(self, ctx: &mut PaneState) -> View<'a, State>
    where
        State: 'static,
    {
        let expanded = self.state.expanded;
        let selected_index = self
            .options
            .iter()
            .position(|o| *o == self.state.selected)
            .unwrap_or(0);
        let hovered = self.state.hovered;
        let depressed = self.state.depressed;
        let root_hovered = hovered == Some(selected_index) && !expanded;
        let id = self.id;
        let binding = self.binding.clone();
        let on_select = self.on_select.clone();
        let background_fn = self.background;
        let dd_state = self.state;
        let options = self.options.clone();
        let view_fn = self.view_fn.clone();
        let row_binding = binding.clone();

        let row = move |index: usize, option: &T, ctx: &mut PaneState| -> View<'a, State> {
            let item_ctx = DropdownItemCtx {
                index,
                value: option,
                selected: selected_index == index,
                hovered: hovered == Some(index),
                expanded,
            };
            let content = (view_fn)(item_ctx, ctx);
            let hovered = hovered == Some(index);
            let depressed = depressed && hovered;
            let background = if expanded {
                rect(crate::id!(id, index as u64))
                    .fill(adjust_brush(
                        &Brush::Solid(DEFAULT_GRAY),
                        depressed,
                        hovered,
                    ))
                    .corner_rounding(DEFAULT_CORNER_ROUNDING)
                    .build(ctx)
            } else {
                empty()
            };
            let row_id = crate::id!(id, index as u64);

            stack(vec![
                background.inert(),
                {
                    rect(row_id)
                        .fill(TRANSPARENT)
                        .view()
                        .gesture(
                            gesture::click(crate::id!(row_id, 1u64))
                                .button(MouseButton::Left)
                                .run({
                                    let binding = row_binding.clone();
                                    let on_select = on_select.clone();
                                    let option = option.clone();
                                    move |state: &mut State, app, event| match event.state {
                                        ClickPhase::Started => {
                                            binding.update(state, |s| s.depressed = true)
                                        }
                                        ClickPhase::Cancelled => {
                                            binding.update(state, |s| s.depressed = false)
                                        }
                                        ClickPhase::Completed => {
                                            if expanded {
                                                if let Some(ref on_select) = on_select {
                                                    on_select(state, app, &option);
                                                }
                                                binding.update(state, {
                                                    let option = option.clone();
                                                    move |s| {
                                                        s.selected = option.clone();
                                                        s.expanded = false;
                                                        s.depressed = false;
                                                    }
                                                });
                                            } else {
                                                binding.update(state, |s| {
                                                    s.expanded = true;
                                                    s.depressed = false;
                                                });
                                            }
                                        }
                                    }
                                }),
                        )
                        .gesture(gesture::hover(crate::id!(row_id, 2u64)).run({
                            let binding = row_binding.clone();
                            move |state: &mut State, _app, hovered| {
                                binding.update(state, move |s| {
                                    if hovered {
                                        s.hovered = Some(index)
                                    } else if s.hovered == Some(index) {
                                        s.hovered = None
                                    }
                                });
                            }
                        }))
                        .build(ctx)
                        .inert()
                },
                content,
            ])
            .expand_x()
        };

        let surface = |ctx: &mut PaneState| -> View<'a, State> {
            if let Some(ref f) = background_fn {
                f(dd_state, ctx)
            } else {
                rect(crate::id!(id))
                    .fill(adjust_brush(
                        &Brush::Solid(DEFAULT_GRAY),
                        depressed && !expanded,
                        root_hovered,
                    ))
                    .stroke(Color::from_rgb8(60, 60, 60), Stroke::new(1.))
                    .corner_rounding(DEFAULT_CORNER_ROUNDING)
                    .build(ctx)
            }
        };

        let visible_layer = stack(vec![
            surface(ctx).inert(),
            row(selected_index, &options[selected_index], ctx),
        ]);

        if !expanded {
            return visible_layer;
        }

        let close_click = gesture::click(crate::id!(id, 1u64))
            .button(MouseButton::Left)
            .anywhere()
            .run({
                let binding = binding.clone();
                move |state: &mut State, _app, event| {
                    let ClickPhase::Completed = event.state else {
                        return;
                    };
                    binding.update(state, |s| {
                        s.expanded = false;
                        s.depressed = false;
                    });
                }
            });
        let outside_hover = gesture::hover(crate::id!(id, 2u64)).anywhere().run({
            let binding = binding.clone();
            move |state: &mut State, _app, hovered| {
                if hovered {
                    binding.update(state, |s| s.hovered = None);
                }
            }
        });
        let interactive_bg = rect(crate::id!(id, 1u64))
            .fill(TRANSPARENT)
            .view()
            .occlude(&close_click)
            .occlude(&outside_hover)
            .build(ctx);

        let all_rows: Vec<_> = options
            .iter()
            .enumerate()
            .map(|(index, option)| row(index, option, ctx))
            .collect();

        let popup = stack(vec![
            surface(ctx).inert(),
            interactive_bg.inert(),
            column(all_rows).align(Align::Top),
        ])
        .align(Align::Top)
        .inert_y()
        .layer(1);

        stack(vec![visible_layer, popup])
    }
}
