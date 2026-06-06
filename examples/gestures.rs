use haven::winit::WinitApp;
use haven::*;

#[derive(Clone, Debug)]
struct State {
    shared_active: bool,
    ignored_active: bool,
    pass_clicked: bool,
    pass_observed: bool,
    predicate_latched: bool,
    gated_hovered: bool,
    space_down: bool,
    gated_dragging: bool,
    puck: Point,
}

impl Default for State {
    fn default() -> Self {
        Self {
            shared_active: false,
            ignored_active: false,
            pass_clicked: false,
            pass_observed: false,
            predicate_latched: false,
            gated_hovered: false,
            space_down: false,
            gated_dragging: false,
            puck: Point::new(centered_puck(), centered_puck()),
        }
    }
}

fn main() {
    WinitApp::new(State::default())
        .pane(
            PaneBuilder::new("main", view)
                .title("Gestures")
                .inner_size(940, 520),
        )
        .run();
}

fn view<'a>(state: &'a State, app: &mut PaneState) -> View<'a, State> {
    stack(vec![
        rect(id!())
            .fill(background())
            .view()
            .gesture(gesture::key(id!()).key(NamedKey::Space).run(
                |state: &mut State, _app, event| {
                    state.space_down = event.phase == KeyPhase::Pressed;
                    if !state.space_down {
                        state.gated_dragging = false;
                    }
                },
            ))
            .build(app)
            .expand(),
        row_spaced(
            18.,
            vec![
                shared_regions(state, app).width(210.).height(260.),
                pass_through(state, app).width(210.).height(260.),
                predicate_buttons(state, app).width(210.).height(260.),
                gated_drag(state, app).width(210.).height(260.),
            ],
        )
        .pad(24.)
        .align(Align::CenterCenter),
    ])
}

fn shared_regions<'a>(state: &'a State, app: &mut PaneState) -> View<'a, State> {
    let shared = gesture::click(id!())
        .button(MouseButton::Left | MouseButton::Right)
        .run(|state: &mut State, _app, event| {
            if event.state == ClickPhase::Completed {
                state.shared_active = !state.shared_active;
            }
        });
    panel(
        stack(vec![
            rect(id!())
                .fill(fill_for(state.shared_active))
                .stroke(border(), Stroke::new(3.))
                .corner_rounding(ROUNDING)
                .view()
                .include(&shared)
                .build(app)
                .width(128.)
                .height(128.)
                .offset(-28., -22.),
            rect(id!())
                .fill(fill_for(state.shared_active))
                .stroke(border(), Stroke::new(3.))
                .corner_rounding(ROUNDING)
                .view()
                .include(&shared)
                .build(app)
                .width(128.)
                .height(128.)
                .offset(28., 22.),
            rect(id!())
                .fill(fill_for(state.ignored_active))
                .stroke(border(), Stroke::new(3.))
                .corner_rounding(ROUNDING)
                .view()
                .occlude(&shared)
                .gesture(gesture::click(id!()).button(MouseButton::Left).run(
                    |state: &mut State, _app, event| {
                        if event.state == ClickPhase::Completed {
                            state.ignored_active = !state.ignored_active;
                        }
                    },
                ))
                .build(app)
                .width(72.)
                .height(72.),
        ]),
        hint(id!(), "Large: Click\nCenter: Click", app),
    )
}

fn pass_through<'a>(state: &'a State, app: &mut PaneState) -> View<'a, State> {
    panel(
        stack(vec![
            rect(id!())
                .fill(fill_for(state.pass_clicked))
                .stroke(border(), Stroke::new(3.))
                .corner_rounding(ROUNDING)
                .view()
                .gesture(gesture::click(id!()).button(MouseButton::Left).run(
                    |state: &mut State, _app, event| {
                        if event.state == ClickPhase::Completed {
                            state.pass_clicked = !state.pass_clicked;
                        }
                    },
                ))
                .build(app)
                .width(176.)
                .height(176.),
            rect(id!())
                .fill(fill_for(state.pass_observed))
                .stroke(border(), Stroke::new(3.))
                .corner_rounding(ROUNDING)
                .view()
                .gesture(
                    gesture::click(id!())
                        .observe()
                        .button(MouseButton::Left)
                        .run(|state: &mut State, _app, event| {
                            if event.state == ClickPhase::Completed {
                                state.pass_observed = !state.pass_observed;
                            }
                        }),
                )
                .build(app)
                .width(96.)
                .height(96.)
                .offset(30., -30.),
        ]),
        hint(id!(), "Small: Click\nLarge: Click", app),
    )
}

fn predicate_buttons<'a>(state: &'a State, app: &mut PaneState) -> View<'a, State> {
    panel(
        stack(vec![
            rect(id!())
                .fill(fill_for(state.predicate_latched))
                .stroke(border(), Stroke::new(3.))
                .corner_rounding(ROUNDING)
                .view()
                .gesture(
                    gesture::click(id!())
                        .button((MouseButton::Left | MouseButton::Right) & !MouseButton::Middle)
                        .modifiers((Modifier::Shift | Modifier::Control) & !Modifier::Alt)
                        .run(|state: &mut State, _app, event| match event.state {
                            ClickPhase::Started | ClickPhase::Cancelled => {}
                            ClickPhase::Completed => {
                                state.predicate_latched = !state.predicate_latched;
                            }
                        }),
                )
                .build(app)
                .width(150.)
                .height(150.),
        ]),
        hint(id!(), "Shift / Ctrl + Click", app),
    )
}

fn gated_drag<'a>(state: &'a State, app: &mut PaneState) -> View<'a, State> {
    let puck_x = state.puck.x as f32;
    let puck_y = state.puck.y as f32;
    panel(
        stack(vec![
            rect(id!())
                .fill(fill_for(state.space_down && state.gated_hovered))
                .stroke(border(), Stroke::new(3.))
                .corner_rounding(ROUNDING)
                .view()
                .gesture(
                    gesture::hover(id!())
                        .observe()
                        .run(|state: &mut State, _app, hovered| state.gated_hovered = hovered),
                )
                .gesture(
                    gesture::drag(id!())
                        .button(MouseButton::Left | MouseButton::Right)
                        .run(|state: &mut State, _app, drag| {
                            if !state.space_down || !state.gated_hovered {
                                return;
                            }
                            match drag {
                                DragPhase::Began { start, .. } => {
                                    state.gated_dragging = true;
                                    state.puck = puck_position(start);
                                }
                                DragPhase::Updated { current, .. } => {
                                    state.puck = puck_position(current);
                                }
                                DragPhase::Completed { current, .. } => {
                                    state.puck = puck_position(current);
                                    state.gated_dragging = false;
                                }
                            }
                        }),
                )
                .build(app)
                .width(PLAYFIELD)
                .height(PLAYFIELD),
            column(vec![
                space().height(puck_y),
                row(vec![
                    space().width(puck_x),
                    rect(id!())
                        .fill(fill_for(state.space_down && state.gated_hovered))
                        .stroke(border(), Stroke::new(3.))
                        .corner_rounding(ROUNDING)
                        .build(app)
                        .width(PUCK)
                        .height(PUCK),
                    space(),
                ])
                .height(PUCK),
                space(),
            ])
            .width(PLAYFIELD)
            .height(PLAYFIELD),
        ]),
        hint(id!(), "Space + Hover + Drag", app),
    )
}

fn panel<'a>(visual: View<'a, State>, label: View<'a, State>) -> View<'a, State> {
    column_spaced(20., vec![visual.height(190.), label]).align(Align::CenterCenter)
}

fn hint<'a>(id: u64, copy: &str, app: &mut PaneState) -> View<'a, State> {
    text(id, copy)
        .font_size(12)
        .fill(hint_color())
        .wrap()
        .build(app)
        .width(200.)
        .height(44.)
}

fn background() -> Color {
    Color::from_rgb8(18, 18, 20)
}

const ROUNDING: f32 = 8.;
const PLAYFIELD: f32 = 170.;
const PUCK: f32 = 32.;

fn centered_puck() -> f64 {
    ((PLAYFIELD - PUCK) * 0.5) as f64
}

fn puck_position(point: Point) -> Point {
    let max = (PLAYFIELD - PUCK) as f64;
    Point::new(
        (point.x - (PUCK * 0.5) as f64).clamp(0., max),
        (point.y - (PUCK * 0.5) as f64).clamp(0., max),
    )
}

fn fill_for(active_state: bool) -> Color {
    if active_state {
        Color::from_rgb8(76, 132, 230)
    } else {
        inactive()
    }
}

fn inactive() -> Color {
    Color::from_rgb8(58, 62, 70)
}

fn border() -> Color {
    Color::from_rgb8(120, 126, 138)
}

fn hint_color() -> Color {
    Color::from_rgb8(185, 190, 200)
}
