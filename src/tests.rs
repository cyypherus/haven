use crate::pane::PaneEffect;
use crate::*;

fn test_pane<State: 'static>(builder: PaneBuilder<State>) -> crate::pane::Pane<State> {
    builder.build()
}

#[test]
fn dropdown_expands_and_selects_an_option() {
    struct State {
        dropdown: DropdownState<&'static str>,
        selected: Option<&'static str>,
    }

    impl Default for State {
        fn default() -> Self {
            Self {
                dropdown: DropdownState {
                    selected: "one",
                    hovered: None,
                    expanded: false,
                    depressed: false,
                },
                selected: None,
            }
        }
    }

    const DROPDOWN: u64 = 10;
    const OPTIONS: [(u64, &str); 3] = [(11, "one"), (12, "two"), (13, "three")];

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        dropdown(
            DROPDOWN,
            binding!(state, State, dropdown),
            OPTIONS.iter().map(|(_, value)| *value).collect(),
            |item, app| text(OPTIONS[item.index].0, *item.value).build(app),
        )
        .on_select(|state, _, selected| state.selected = Some(*selected))
        .build(app)
        .width(120.)
        .height(30.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));

    let (_, effects) = pane.redraw(&mut state, 300, 200, 1.0);
    assert!(effects.is_empty(), "unexpected effects: {effects:?}");
    assert!(!state.dropdown.expanded);

    let location = pane.location(OPTIONS[0].0).expect("dropdown present");
    assert!(pane.click(&mut state, location).is_empty());
    assert!(state.dropdown.expanded);

    let (_, effects) = pane.redraw(&mut state, 300, 200, 1.0);
    assert!(effects.is_empty(), "unexpected effects: {effects:?}");

    let location = pane.location(OPTIONS[1].0).expect("option present");
    assert!(pane.click(&mut state, location).is_empty());
    assert_eq!(state.dropdown.selected, "two");
    assert_eq!(state.selected, Some("two"));
    assert!(!state.dropdown.expanded);
}

#[test]
fn dropdown_hover_captures_overlapping_button() {
    struct State {
        dropdown: DropdownState<&'static str>,
        button: ButtonState,
    }

    impl Default for State {
        fn default() -> Self {
            Self {
                dropdown: DropdownState {
                    selected: "one",
                    hovered: None,
                    expanded: true,
                    depressed: false,
                },
                button: ButtonState::default(),
            }
        }
    }

    const BUTTON: u64 = 14;
    const DROPDOWN: u64 = 15;
    const OPTIONS: [(u64, &str); 3] = [(16, "one"), (17, "two"), (18, "three")];

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        stack(vec![
            button(BUTTON, binding!(state, State, button))
                .text_label("behind")
                .build(app)
                .width(120.)
                .height(90.),
            dropdown(
                DROPDOWN,
                binding!(state, State, dropdown),
                OPTIONS.iter().map(|(_, value)| *value).collect(),
                |item, app| text(OPTIONS[item.index].0, *item.value).build(app),
            )
            .build(app)
            .width(120.)
            .height(30.),
        ])
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    let (_, effects) = pane.redraw(&mut state, 300, 200, 1.0);
    assert!(effects.is_empty(), "unexpected effects: {effects:?}");

    let location = pane.location(OPTIONS[1].0).expect("option present");
    assert!(pane.move_to(&mut state, location).is_empty());
    let (_, effects) = pane.redraw(&mut state, 300, 200, 1.0);
    assert!(effects.is_empty(), "unexpected effects: {effects:?}");

    assert_eq!(state.dropdown.hovered, Some(1));
    assert!(!state.button.hovered);
}

#[test]
fn toggle_click_updates_state() {
    #[derive(Default)]
    struct State {
        toggle: ToggleState,
        toggled: Option<bool>,
    }

    const TOGGLE: u64 = 20;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        toggle(TOGGLE, binding!(state, State, toggle))
            .on_toggle(|state, _, on| state.toggled = Some(on))
            .build(app)
            .width(60.)
            .height(30.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    let (_, effects) = pane.redraw(&mut state, 300, 200, 1.0);
    assert!(effects.is_empty(), "unexpected effects: {effects:?}");

    let location = pane.location(TOGGLE).expect("toggle present");
    assert!(pane.move_to(&mut state, location).is_empty());
    let (_, effects) = pane.redraw(&mut state, 300, 200, 1.0);
    assert!(effects.is_empty(), "unexpected effects: {effects:?}");
    assert!(state.toggle.hovered);
    assert!(pane.press(&mut state).is_empty());
    assert!(state.toggle.depressed);
    assert!(pane.release(&mut state).is_empty());

    assert!(state.toggle.on);
    assert!(!state.toggle.depressed);
    assert_eq!(state.toggled, Some(true));
}

#[test]
fn toggle_drag_updates_state_from_position() {
    #[derive(Default)]
    struct State {
        toggle: ToggleState,
        toggled: Option<bool>,
    }

    const TOGGLE: u64 = 21;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        toggle(TOGGLE, binding!(state, State, toggle))
            .on_toggle(|state, _, on| state.toggled = Some(on))
            .build(app)
            .width(60.)
            .height(30.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    let (_, effects) = pane.redraw(&mut state, 300, 200, 1.0);
    assert!(effects.is_empty(), "unexpected effects: {effects:?}");

    let center = pane.location(TOGGLE).expect("toggle present");
    assert!(
        pane.drag(
            &mut state,
            Point::new(center.x - 20., center.y),
            Point::new(center.x + 20., center.y),
        )
        .is_empty()
    );
    assert!(state.toggle.on);
    assert!(!state.toggle.depressed);
    assert_eq!(state.toggled, Some(true));

    state.toggled = None;
    assert!(
        pane.drag(
            &mut state,
            Point::new(center.x + 20., center.y),
            Point::new(center.x - 20., center.y),
        )
        .is_empty()
    );
    assert!(!state.toggle.on);
    assert!(!state.toggle.depressed);
    assert_eq!(state.toggled, Some(false));
}

#[test]
fn drawable_click_event_reports_mouse_button_and_location() {
    #[derive(Default)]
    struct State {
        events: Vec<(ClickPhase, Point)>,
    }

    const TARGET: u64 = 25;

    fn view<'a>(_: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        rect(TARGET)
            .fill(TRANSPARENT)
            .view()
            .gesture(
                gesture::click(id!(TARGET, 1u64))
                    .button(MouseButton::Right)
                    .run(|state: &mut State, _, event| {
                        state.events.push((event.state, event.location.local()));
                    }),
            )
            .build(app)
            .width(80.)
            .height(40.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    let (_, effects) = pane.redraw(&mut state, 300, 200, 1.0);
    assert!(effects.is_empty(), "unexpected effects: {effects:?}");

    let location = pane.location(TARGET).expect("target present");
    assert!(pane.move_to(&mut state, location).is_empty());
    assert!(pane.press_button(&mut state, MouseButton::Right).is_empty());
    assert!(
        pane.release_button(&mut state, MouseButton::Right)
            .is_empty()
    );

    assert_eq!(state.events.len(), 2);
    assert_eq!(state.events[0].0, ClickPhase::Started);
    assert_eq!(state.events[1].0, ClickPhase::Completed);
    assert_eq!(state.events[0].1, state.events[1].1);
}

#[test]
fn mouse_predicates_cover_all_button_variants() {
    #[derive(Default)]
    struct State {
        completions: usize,
    }

    const TARGET: u64 = 125;

    fn view<'a>(_: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        rect(TARGET)
            .fill(TRANSPARENT)
            .view()
            .gesture(
                gesture::click(id!(TARGET, 1u64))
                    .button(
                        MouseButton::Left
                            | MouseButton::Right
                            | MouseButton::Middle
                            | MouseButton::Back
                            | MouseButton::Forward
                            | MouseButton::Other(9)
                            | MouseButton::Other(10),
                    )
                    .run(|state: &mut State, _, event| {
                        if event.state == ClickPhase::Completed {
                            state.completions += 1;
                        }
                    }),
            )
            .build(app)
            .width(80.)
            .height(40.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    let (_, effects) = pane.redraw(&mut state, 300, 200, 1.0);
    assert!(effects.is_empty(), "unexpected effects: {effects:?}");

    let location = pane.location(TARGET).expect("target present");
    assert!(pane.move_to(&mut state, location).is_empty());

    for button in [
        MouseButton::Left,
        MouseButton::Right,
        MouseButton::Middle,
        MouseButton::Back,
        MouseButton::Forward,
        MouseButton::Other(9),
        MouseButton::Other(10),
    ] {
        assert!(pane.press_button(&mut state, button).is_empty());
        assert!(pane.release_button(&mut state, button).is_empty());
    }

    assert_eq!(state.completions, 7);
}

#[test]
fn programmatic_click_reports_logical_location_when_scaled() {
    #[derive(Default)]
    struct State {
        events: Vec<Point>,
    }

    const TARGET: u64 = 26;

    fn view<'a>(_: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        rect(TARGET)
            .fill(TRANSPARENT)
            .view()
            .gesture(
                gesture::click(id!(TARGET, 1u64))
                    .button(MouseButton::Left)
                    .run(|state: &mut State, _, event| {
                        state.events.push(event.location.local());
                    }),
            )
            .build(app)
            .width(80.)
            .height(40.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    let (_, effects) = pane.redraw(&mut state, 300, 200, 2.0);
    assert!(effects.is_empty(), "unexpected effects: {effects:?}");

    let location = pane.location(TARGET).expect("target present");
    assert!(pane.click(&mut state, location).is_empty());

    assert_eq!(
        state.events,
        vec![Point::new(40., 20.), Point::new(40., 20.)]
    );
}

#[test]
fn click_capture_ignores_handlers_for_other_buttons() {
    #[derive(Default)]
    struct State {
        left_events: Vec<ClickPhase>,
        right_events: Vec<ClickPhase>,
    }

    const LEFT_TARGET: u64 = 26;
    const RIGHT_TARGET: u64 = 27;

    fn view<'a>(_: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        let left_click = gesture::click(id!(LEFT_TARGET, 1u64))
            .button(MouseButton::Left)
            .run(|state: &mut State, _, event| {
                state.left_events.push(event.state);
            });
        let right_click = gesture::click(id!(RIGHT_TARGET, 1u64))
            .button(MouseButton::Right)
            .run(|state: &mut State, _, event| {
                state.right_events.push(event.state);
            });
        stack(vec![
            rect(LEFT_TARGET)
                .fill(TRANSPARENT)
                .view()
                .include(&left_click)
                .build(app)
                .width(80.)
                .height(40.),
            rect(RIGHT_TARGET)
                .fill(TRANSPARENT)
                .view()
                .include(&right_click)
                .build(app)
                .width(80.)
                .height(40.),
        ])
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    let (_, effects) = pane.redraw(&mut state, 300, 200, 1.0);
    assert!(effects.is_empty(), "unexpected effects: {effects:?}");

    let location = pane.location(RIGHT_TARGET).expect("target present");
    assert!(pane.move_to(&mut state, location).is_empty());
    assert!(pane.press_button(&mut state, MouseButton::Left).is_empty());
    assert!(
        pane.release_button(&mut state, MouseButton::Left)
            .is_empty()
    );

    assert_eq!(
        state.left_events,
        vec![ClickPhase::Started, ClickPhase::Completed]
    );
    assert!(state.right_events.is_empty());
}

#[test]
fn click_capture_waits_for_matching_release_button() {
    #[derive(Default)]
    struct State {
        events: Vec<ClickPhase>,
    }

    const TARGET: u64 = 28;

    fn view<'a>(_: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        rect(TARGET)
            .fill(TRANSPARENT)
            .view()
            .gesture(
                gesture::click(id!(TARGET, 1u64))
                    .button(MouseButton::Right)
                    .run(|state: &mut State, _, event| {
                        state.events.push(event.state);
                    }),
            )
            .build(app)
            .width(80.)
            .height(40.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);

    let location = pane.location(TARGET).expect("target present");
    pane.move_to(&mut state, location);
    pane.press_button(&mut state, MouseButton::Right);
    pane.release_button(&mut state, MouseButton::Left);
    assert_eq!(state.events, vec![ClickPhase::Started]);

    pane.release_button(&mut state, MouseButton::Right);
    assert_eq!(
        state.events,
        vec![ClickPhase::Started, ClickPhase::Completed]
    );
}

#[test]
fn scoped_gesture_click_uses_region_and_button_predicates() {
    #[derive(Default)]
    struct State {
        events: Vec<&'static str>,
    }

    const ROOT: u64 = 2001;
    const A: u64 = 2002;
    const B: u64 = 2003;

    fn view<'a>(_: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        let target = gesture::click(id!(A, 1u64))
            .button(MouseButton::Left | MouseButton::Right)
            .run(|state: &mut State, _, event| {
                if event.state == ClickPhase::Completed {
                    state.events.push("target");
                }
            });
        let empty = gesture::click(id!(ROOT, 1u64))
            .button(MouseButton::Left)
            .anywhere()
            .run(|state: &mut State, _, event| {
                if event.state == ClickPhase::Completed {
                    state.events.push("empty");
                }
            });
        stack(vec![
            rect(ROOT)
                .fill(TRANSPARENT)
                .build(app)
                .width(300.)
                .height(100.),
            rect(A)
                .fill(TRANSPARENT)
                .view()
                .include(&target)
                .occlude(&empty)
                .build(app)
                .width(50.)
                .height(50.)
                .offset(-100., 0.),
            rect(B)
                .fill(TRANSPARENT)
                .view()
                .include(&target)
                .occlude(&empty)
                .build(app)
                .width(50.)
                .height(50.)
                .offset(100., 0.),
        ])
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 400, 200, 1.0);

    let a = pane.location(A).expect("a present");
    pane.move_to(&mut state, a);
    pane.press_button(&mut state, MouseButton::Left);
    pane.release_button(&mut state, MouseButton::Left);

    let b = pane.location(B).expect("b present");
    pane.move_to(&mut state, b);
    pane.press_button(&mut state, MouseButton::Right);
    pane.release_button(&mut state, MouseButton::Right);

    let root = pane.location(ROOT).expect("root present");
    pane.move_to(&mut state, root);
    pane.press_button(&mut state, MouseButton::Left);
    pane.release_button(&mut state, MouseButton::Left);

    assert_eq!(state.events, vec!["target", "target", "empty"]);
}

#[test]
fn scoped_gesture_drag_can_use_right_button() {
    #[derive(Default)]
    struct State {
        phases: Vec<&'static str>,
    }

    const SURFACE: u64 = 2011;

    fn view<'a>(_: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        rect(SURFACE)
            .fill(TRANSPARENT)
            .view()
            .gesture(
                gesture::drag(id!(SURFACE, 1u64))
                    .button(MouseButton::Right)
                    .run(|state: &mut State, _, event| match event {
                        DragPhase::Began { .. } => state.phases.push("began"),
                        DragPhase::Updated { .. } => state.phases.push("updated"),
                        DragPhase::Completed { .. } => state.phases.push("completed"),
                    }),
            )
            .build(app)
            .width(100.)
            .height(100.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);

    let location = pane.location(SURFACE).expect("surface present");
    pane.move_to(&mut state, location);
    pane.press_button(&mut state, MouseButton::Right);
    pane.move_to(&mut state, Point::new(location.x + 10., location.y));
    pane.release_button(&mut state, MouseButton::Right);

    assert_eq!(state.phases, vec!["began", "updated", "completed"]);
}

#[test]
fn scoped_gesture_drag_can_use_button_predicate() {
    #[derive(Default)]
    struct State {
        completed: usize,
    }

    const SURFACE: u64 = 2012;

    fn view<'a>(_: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        rect(SURFACE)
            .fill(TRANSPARENT)
            .view()
            .gesture(
                gesture::drag(id!(SURFACE, 1u64))
                    .button(MouseButton::Left | MouseButton::Right)
                    .run(|state: &mut State, _, event| {
                        if matches!(event, DragPhase::Completed { .. }) {
                            state.completed += 1;
                        }
                    }),
            )
            .build(app)
            .width(100.)
            .height(100.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);

    let location = pane.location(SURFACE).expect("surface present");
    for button in [MouseButton::Left, MouseButton::Right] {
        pane.move_to(&mut state, location);
        pane.press_button(&mut state, button);
        pane.move_to(&mut state, Point::new(location.x + 10., location.y));
        pane.release_button(&mut state, button);
    }
    pane.move_to(&mut state, location);
    pane.press_button(&mut state, MouseButton::Middle);
    pane.move_to(&mut state, Point::new(location.x + 10., location.y));
    pane.release_button(&mut state, MouseButton::Middle);

    assert_eq!(state.completed, 2);
}

#[test]
fn scoped_gesture_click_can_use_modifier_predicate() {
    let predicate = (Modifier::Shift | Modifier::Control) & !Modifier::Alt;
    assert!(predicate.matches(Modifiers::from_pressed([Modifier::Shift])));
    assert!(predicate.matches(Modifiers::from_pressed([Modifier::Control])));
    assert!(!predicate.matches(Modifiers::from_pressed([Modifier::Shift, Modifier::Alt,])));
    assert!(!predicate.matches(Modifiers::empty()));

    #[derive(Default)]
    struct State {
        clicks: usize,
    }

    const SURFACE: u64 = 2013;

    fn view<'a>(_: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        rect(SURFACE)
            .fill(TRANSPARENT)
            .view()
            .gesture(
                gesture::click(id!(SURFACE, 1u64))
                    .button(MouseButton::Left)
                    .modifiers((Modifier::Shift | Modifier::Control) & !Modifier::Alt)
                    .run(|state: &mut State, _, event| {
                        if event.state == ClickPhase::Completed {
                            state.clicks += 1;
                        }
                    }),
            )
            .build(app)
            .width(100.)
            .height(100.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);
    let location = pane.location(SURFACE).expect("surface present");
    pane.move_to(&mut state, location);

    pane.modifiers_changed(Modifiers::from_pressed([Modifier::Shift]));
    pane.press_button(&mut state, MouseButton::Left);
    pane.release_button(&mut state, MouseButton::Left);
    assert_eq!(state.clicks, 1);

    pane.modifiers_changed(Modifiers::from_pressed([Modifier::Control]));
    pane.press_button(&mut state, MouseButton::Left);
    pane.release_button(&mut state, MouseButton::Left);
    assert_eq!(state.clicks, 2);

    pane.modifiers_changed(Modifiers::from_pressed([Modifier::Shift, Modifier::Alt]));
    pane.press_button(&mut state, MouseButton::Left);
    pane.release_button(&mut state, MouseButton::Left);
    assert_eq!(state.clicks, 2);

    pane.modifiers_changed(Modifiers::empty());
    pane.press_button(&mut state, MouseButton::Left);
    pane.release_button(&mut state, MouseButton::Left);

    assert_eq!(state.clicks, 2);
}

#[test]
fn scoped_scroll_is_region_gated_but_key_is_not() {
    #[derive(Default)]
    struct State {
        scrolls: usize,
        keys: usize,
    }

    const A: u64 = 2021;
    const B: u64 = 2022;

    fn view<'a>(_: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        let scroll =
            gesture::scroll(id!(A, 1u64)).run(|state: &mut State, _, _| state.scrolls += 1);
        let key = gesture::key(id!(A, 2u64))
            .key(NamedKey::Enter)
            .modifiers(Modifier::Shift)
            .run(|state: &mut State, _, _| state.keys += 1);
        row(vec![
            rect(A)
                .fill(TRANSPARENT)
                .view()
                .include(&scroll)
                .include(&key)
                .build(app)
                .width(100.)
                .height(100.),
            rect(B)
                .fill(TRANSPARENT)
                .build(app)
                .width(100.)
                .height(100.),
        ])
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);
    pane.modifiers_changed(Modifiers::from_pressed([Modifier::Shift]));
    pane.key_pressed(&mut state, NamedKey::Enter);

    let a = pane.location(A).expect("a present");
    pane.move_to(&mut state, a);
    pane.scroll(&mut state, ScrollDelta { x: 0., y: 1. });
    pane.key_pressed(&mut state, NamedKey::Enter);

    let b = pane.location(B).expect("b present");
    pane.move_to(&mut state, b);
    pane.scroll(&mut state, ScrollDelta { x: 0., y: 1. });
    pane.key_pressed(&mut state, NamedKey::Enter);

    assert_eq!(state.scrolls, 1);
    assert_eq!(state.keys, 3);
}

#[test]
fn clipping_limits_pointer_gestures() {
    #[derive(Default)]
    struct State {
        clicks: usize,
        scrolls: usize,
        hovered: bool,
    }

    const CLIP: u64 = 2023;
    const TARGET: u64 = 2024;

    fn view<'a>(_: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        stack_aligned(
            Align::TopLeading,
            vec![
                rect(CLIP)
                    .fill(TRANSPARENT)
                    .build(app)
                    .width(100.)
                    .height(50.),
                rect(TARGET)
                    .fill(TRANSPARENT)
                    .view()
                    .gesture(
                        gesture::hover(id!(TARGET, 1u64))
                            .run(|state: &mut State, _, hovered| state.hovered = hovered),
                    )
                    .gesture(
                        gesture::scroll(id!(TARGET, 2u64))
                            .run(|state: &mut State, _, _| state.scrolls += 1),
                    )
                    .gesture(gesture::click(id!(TARGET, 3u64)).run(
                        |state: &mut State, _, event| {
                            if event.state == ClickPhase::Completed {
                                state.clicks += 1;
                            }
                        },
                    ))
                    .build(app)
                    .width(100.)
                    .height(100.)
                    .offset_y(40.)
                    .inert(),
            ],
        )
        .clipped(rect_path)
        .width(100.)
        .height(50.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);

    let target = pane.elements[&TARGET];
    let inside_clip = Point::new(
        target.x as f64 + target.width as f64 * 0.5,
        target.y as f64 + 5.,
    );
    let outside_clip = Point::new(
        target.x as f64 + target.width as f64 * 0.5,
        target.y as f64 + 20.,
    );

    pane.move_to(&mut state, inside_clip);
    pane.redraw(&mut state, 300, 200, 1.0);
    pane.scroll(&mut state, ScrollDelta { x: 0., y: 1. });
    pane.click(&mut state, inside_clip);
    assert!(state.hovered);
    assert_eq!(state.scrolls, 1);
    assert_eq!(state.clicks, 1);

    pane.move_to(&mut state, outside_clip);
    pane.redraw(&mut state, 300, 200, 1.0);
    pane.scroll(&mut state, ScrollDelta { x: 0., y: 1. });
    pane.click(&mut state, outside_clip);
    assert!(!state.hovered);
    assert_eq!(state.scrolls, 1);
    assert_eq!(state.clicks, 1);
}

#[test]
fn blend_layer_does_not_clip_pointer_gestures() {
    #[derive(Default)]
    struct State {
        clicks: usize,
    }

    const LAYER: u64 = 2035;
    const TARGET: u64 = 2036;

    fn view<'a>(_: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        stack_aligned(
            Align::TopLeading,
            vec![
                rect(LAYER)
                    .fill(TRANSPARENT)
                    .build(app)
                    .width(100.)
                    .height(50.),
                rect(TARGET)
                    .fill(TRANSPARENT)
                    .view()
                    .gesture(gesture::click(id!(TARGET, 1u64)).run(
                        |state: &mut State, _, event| {
                            if event.state == ClickPhase::Completed {
                                state.clicks += 1;
                            }
                        },
                    ))
                    .build(app)
                    .width(100.)
                    .height(100.),
            ],
        )
        .blend(BlendMode::Screen)
        .width(100.)
        .height(50.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);

    let target = pane.elements[&TARGET];
    let outside_layer = Point::new(
        target.x as f64 + target.width as f64 * 0.5,
        target.y as f64 + 75.,
    );
    pane.click(&mut state, outside_layer);

    assert_eq!(state.clicks, 1);
}

#[test]
fn clipping_limits_layered_pointer_gestures() {
    #[derive(Default)]
    struct State {
        clicks: usize,
    }

    const CLIP: u64 = 2037;
    const TARGET: u64 = 2038;

    fn view<'a>(_: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        stack_aligned(
            Align::TopLeading,
            vec![
                rect(CLIP)
                    .fill(TRANSPARENT)
                    .build(app)
                    .width(100.)
                    .height(50.),
                rect(TARGET)
                    .fill(TRANSPARENT)
                    .view()
                    .gesture(gesture::click(id!(TARGET, 1u64)).run(
                        |state: &mut State, _, event| {
                            if event.state == ClickPhase::Completed {
                                state.clicks += 1;
                            }
                        },
                    ))
                    .build(app)
                    .width(100.)
                    .height(100.)
                    .layer(1),
            ],
        )
        .clipped(rect_path)
        .width(100.)
        .height(50.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);

    let target = pane.elements[&TARGET];
    let outside_clip = Point::new(
        target.x as f64 + target.width as f64 * 0.5,
        target.y as f64 + 75.,
    );
    pane.click(&mut state, outside_clip);

    assert_eq!(state.clicks, 0);
}

#[test]
fn key_gestures_default_to_capture() {
    #[derive(Default)]
    struct State {
        bottom: usize,
        top: usize,
    }

    const BOTTOM: u64 = 2031;
    const TOP: u64 = 2032;

    fn view<'a>(_: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        stack(vec![
            rect(BOTTOM)
                .fill(TRANSPARENT)
                .view()
                .gesture(
                    gesture::key(id!(BOTTOM, 1u64))
                        .key(NamedKey::Enter)
                        .run(|state: &mut State, _, _| state.bottom += 1),
                )
                .build(app)
                .width(100.)
                .height(100.),
            rect(TOP)
                .fill(TRANSPARENT)
                .view()
                .gesture(
                    gesture::key(id!(TOP, 1u64))
                        .key(NamedKey::Enter)
                        .run(|state: &mut State, _, _| state.top += 1),
                )
                .build(app)
                .width(100.)
                .height(100.),
        ])
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);
    pane.key_pressed(&mut state, NamedKey::Enter);

    assert_eq!(state.bottom, 0);
    assert_eq!(state.top, 1);
}

#[test]
fn scoped_shared_token_can_mark_multiple_regions() {
    #[derive(Default)]
    struct Child {
        clicks: usize,
    }

    #[derive(Default)]
    struct State {
        child: Child,
    }

    const A: u64 = 2031;
    const B: u64 = 2032;
    const CLICK: u64 = 2033;

    fn child_view<'a>(_: &'a Child, app: &mut PaneState) -> Layout<'a, View<Child>, PaneState> {
        let click =
            gesture::click(CLICK)
                .button(MouseButton::Left)
                .run(|state: &mut Child, _, event| {
                    if event.state == ClickPhase::Completed {
                        state.clicks += 1;
                    }
                });

        row(vec![
            rect(A)
                .fill(TRANSPARENT)
                .view()
                .include(&click)
                .build(app)
                .width(80.)
                .height(40.),
            rect(B)
                .fill(TRANSPARENT)
                .view()
                .include(&click)
                .build(app)
                .width(80.)
                .height(40.),
        ])
    }

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        let (_, child) = binding!(state, State, child);
        scope(child_view(&state.child, app), child)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);

    let a = pane.location(A).expect("a present");
    pane.click(&mut state, a);
    let b = pane.location(B).expect("b present");
    pane.click(&mut state, b);

    assert_eq!(state.child.clicks, 2);
}

#[test]
fn scoped_text_field_lifecycle_callback_uses_child_state() {
    #[derive(Default)]
    struct Child {
        text: TextState,
        starts: usize,
    }

    #[derive(Default)]
    struct State {
        child: Child,
    }

    const FIELD: u64 = 2034;

    fn child_view<'a>(state: &'a Child, app: &mut PaneState) -> Layout<'a, View<Child>, PaneState> {
        text_field(FIELD, binding!(state, Child, text))
            .on_edit(|state, _, edit| {
                if matches!(edit, EditInteraction::Start) {
                    state.starts += 1;
                }
            })
            .build(app)
            .width(140.)
            .height(40.)
    }

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        let (_, child) = binding!(state, State, child);
        scope(child_view(&state.child, app), child)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);

    state.child.text.begin_editing(&mut pane.pane_state);
    pane.redraw(&mut state, 300, 200, 1.0);

    assert_eq!(state.child.starts, 1);
}

#[test]
fn click_release_outside_cancels_captured_click() {
    #[derive(Default)]
    struct State {
        events: Vec<ClickPhase>,
    }

    const TARGET: u64 = 29;

    fn view<'a>(_: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        rect(TARGET)
            .fill(TRANSPARENT)
            .view()
            .gesture(
                gesture::click(id!(TARGET, 1u64))
                    .button(MouseButton::Left)
                    .run(|state: &mut State, _, event| {
                        state.events.push(event.state);
                    }),
            )
            .build(app)
            .width(80.)
            .height(40.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);

    let location = pane.location(TARGET).expect("target present");
    pane.move_to(&mut state, location);
    pane.press(&mut state);
    pane.move_to(&mut state, Point::new(location.x + 200., location.y));
    pane.release(&mut state);

    assert_eq!(
        state.events,
        vec![ClickPhase::Started, ClickPhase::Cancelled]
    );
}

#[test]
fn drag_starts_after_threshold_and_cancels_click() {
    #[derive(Default)]
    struct State {
        events: Vec<&'static str>,
    }

    const TARGET: u64 = 30;

    fn view<'a>(_: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        rect(TARGET)
            .fill(TRANSPARENT)
            .view()
            .gesture(
                gesture::drag(id!(TARGET, 1u64))
                    .button(MouseButton::Left)
                    .run(|state: &mut State, _, event| match event {
                        DragPhase::Began { .. } => state.events.push("drag-began"),
                        DragPhase::Updated { .. } => state.events.push("drag-updated"),
                        DragPhase::Completed { .. } => state.events.push("drag-completed"),
                    }),
            )
            .gesture(
                gesture::click(id!(TARGET, 2u64))
                    .button(MouseButton::Left)
                    .run(|state: &mut State, _, event| {
                        state.events.push(match event.state {
                            ClickPhase::Started => "click-started",
                            ClickPhase::Cancelled => "click-cancelled",
                            ClickPhase::Completed => "click-completed",
                        });
                    }),
            )
            .build(app)
            .width(80.)
            .height(40.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);

    let location = pane.location(TARGET).expect("target present");
    pane.move_to(&mut state, location);
    pane.press(&mut state);
    pane.move_to(&mut state, Point::new(location.x + 2., location.y));
    pane.release(&mut state);
    assert_eq!(state.events, vec!["click-started", "click-completed"]);

    state.events.clear();
    pane.move_to(&mut state, location);
    pane.press(&mut state);
    pane.move_to(&mut state, Point::new(location.x + 4., location.y));
    pane.release(&mut state);
    assert_eq!(
        state.events,
        vec![
            "click-started",
            "click-cancelled",
            "drag-began",
            "drag-updated",
            "drag-completed"
        ]
    );
}

#[test]
fn click_outside_is_button_specific() {
    #[derive(Default)]
    struct State {
        outside: Vec<ClickPhase>,
    }

    const LISTENER: u64 = 30;
    const TARGET: u64 = 31;

    fn view<'a>(_: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        let outside = gesture::click(id!(LISTENER, 1u64))
            .button(MouseButton::Right)
            .anywhere()
            .run(|state: &mut State, _, event| {
                state.outside.push(event.state);
            });
        row_spaced(
            20.,
            vec![
                rect(LISTENER)
                    .fill(TRANSPARENT)
                    .view()
                    .occlude(&outside)
                    .build(app)
                    .width(60.)
                    .height(40.),
                rect(TARGET)
                    .fill(TRANSPARENT)
                    .build(app)
                    .width(60.)
                    .height(40.),
            ],
        )
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);

    let location = pane.location(TARGET).expect("target present");
    pane.move_to(&mut state, location);
    pane.press(&mut state);
    pane.release(&mut state);
    assert!(state.outside.is_empty());

    pane.press_button(&mut state, MouseButton::Right);
    pane.release_button(&mut state, MouseButton::Right);
    assert_eq!(
        state.outside,
        vec![ClickPhase::Started, ClickPhase::Completed]
    );
}

#[test]
fn occluded_gesture_without_anywhere_has_no_default_region() {
    #[derive(Default)]
    struct State {
        outside: usize,
    }

    const OCCLUDER: u64 = 33;
    const TARGET: u64 = 34;

    fn view<'a>(_: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        let outside = gesture::click(id!(TARGET, 1u64)).run(|state: &mut State, _, event| {
            if event.state == ClickPhase::Completed {
                state.outside += 1;
            }
        });
        row_spaced(
            20.,
            vec![
                rect(OCCLUDER)
                    .fill(TRANSPARENT)
                    .view()
                    .occlude(&outside)
                    .build(app)
                    .width(60.)
                    .height(40.),
                rect(TARGET)
                    .fill(TRANSPARENT)
                    .build(app)
                    .width(60.)
                    .height(40.),
            ],
        )
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);

    let location = pane.location(TARGET).expect("target present");
    pane.click(&mut state, location);

    assert_eq!(state.outside, 0);
}

#[test]
fn right_click_does_not_start_left_drag_handler() {
    #[derive(Default)]
    struct State {
        slider: SliderState,
    }

    const SLIDER: u64 = 32;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        slider(SLIDER, binding!(state, State, slider))
            .build(app)
            .width(100.)
            .height(20.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);

    let location = pane.location(SLIDER).expect("slider present");
    pane.move_to(&mut state, location);
    pane.press_button(&mut state, MouseButton::Right);
    pane.move_to(&mut state, Point::new(location.x + 50., location.y));
    pane.release_button(&mut state, MouseButton::Right);

    assert!(!state.slider.dragging);
    assert_eq!(state.slider.value, 0.);
}

#[test]
fn slider_click_updates_value() {
    #[derive(Default)]
    struct State {
        slider: SliderState,
        changed: Option<f32>,
    }

    const SLIDER: u64 = 31;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        slider(SLIDER, binding!(state, State, slider))
            .on_change(|state, _, value| state.changed = Some(value))
            .build(app)
            .width(100.)
            .height(20.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);

    let location = pane.location(SLIDER).expect("slider present");
    assert!(pane.click(&mut state, location).is_empty());

    assert!((state.slider.value - 0.5).abs() < 0.001);
    assert!((state.changed.unwrap() - 0.5).abs() < 0.001);
}

#[test]
fn slider_drag_updates_value() {
    #[derive(Default)]
    struct State {
        slider: SliderState,
        changed: Option<f32>,
    }

    const SLIDER: u64 = 30;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        slider(SLIDER, binding!(state, State, slider))
            .on_change(|state, _, value| state.changed = Some(value))
            .build(app)
            .width(100.)
            .height(20.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    let (_, effects) = pane.redraw(&mut state, 300, 200, 1.0);
    assert!(effects.is_empty(), "unexpected effects: {effects:?}");

    let location = pane.location(SLIDER).expect("slider present");
    assert!(pane.move_to(&mut state, location).is_empty());
    let (_, effects) = pane.redraw(&mut state, 300, 200, 1.0);
    assert!(effects.is_empty(), "unexpected effects: {effects:?}");
    assert!(state.slider.hovered);
    assert!(pane.press(&mut state).is_empty());
    assert!(!state.slider.dragging);
    assert!(
        pane.move_to(&mut state, Point::new(location.x + 10., location.y))
            .is_empty()
    );
    assert!(state.slider.dragging);
    assert!(pane.move_to(&mut state, Point::new(190., 100.)).is_empty());
    assert!(pane.release(&mut state).is_empty());

    assert!((state.slider.value - 1.0).abs() < 0.001);
    assert!(!state.slider.dragging);
    assert!((state.changed.unwrap() - 1.0).abs() < 0.001);
}

#[test]
fn text_field_click_and_key_update_state() {
    struct State {
        text: TextState,
        edits: Vec<EditInteraction>,
    }

    impl Default for State {
        fn default() -> Self {
            Self {
                text: TextState::new(""),
                edits: Vec::new(),
            }
        }
    }

    const FIELD: u64 = 40;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .on_edit(|state, _, edit| state.edits.push(edit))
            .build(app)
            .width(140.)
            .height(40.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    let (_, effects) = pane.redraw(&mut state, 300, 200, 1.0);
    assert!(effects.is_empty(), "unexpected effects: {effects:?}");

    let location = pane.location(FIELD).expect("field present");
    assert!(pane.click(&mut state, location).is_empty());
    assert!(pane.pane_state.text_field_is_focused(FIELD));
    assert!(matches!(state.edits.last(), Some(EditInteraction::Start)));

    let (_, effects) = pane.redraw(&mut state, 300, 200, 1.0);
    assert!(effects.is_empty(), "unexpected effects: {effects:?}");

    assert!(pane.key_pressed(&mut state, "a").is_empty());

    assert_eq!(state.text.text, "a");
    assert!(matches!(
        state.edits.last(),
        Some(EditInteraction::Update(text)) if text == "a"
    ));
}

#[test]
fn text_field_on_edit_observes_applied_text_state() {
    #[derive(Debug, PartialEq, Eq)]
    enum Seen {
        Start {
            text: String,
        },
        Update {
            state_text: String,
            edit_text: String,
        },
        End {
            text: String,
        },
    }

    struct State {
        text: TextState,
        seen: Vec<Seen>,
    }

    impl Default for State {
        fn default() -> Self {
            Self {
                text: TextState::new(""),
                seen: Vec::new(),
            }
        }
    }

    const FIELD: u64 = 76;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .on_edit(|state, _, edit| match edit {
                EditInteraction::Start => state.seen.push(Seen::Start {
                    text: state.text.text.clone(),
                }),
                EditInteraction::Update(edit_text) => state.seen.push(Seen::Update {
                    state_text: state.text.text.clone(),
                    edit_text,
                }),
                EditInteraction::End => state.seen.push(Seen::End {
                    text: state.text.text.clone(),
                }),
            })
            .build(app)
            .width(140.)
            .height(40.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);

    let location = pane.location(FIELD).expect("field present");
    pane.click(&mut state, location);
    pane.redraw(&mut state, 300, 200, 1.0);
    pane.key_pressed(&mut state, "a");
    state.text.end_editing(&mut pane.pane_state);
    pane.redraw(&mut state, 300, 200, 1.0);

    assert_eq!(
        state.seen,
        vec![
            Seen::Start {
                text: String::new(),
            },
            Seen::Update {
                state_text: "a".to_string(),
                edit_text: "a".to_string(),
            },
            Seen::End {
                text: "a".to_string(),
            },
        ]
    );
}

#[test]
fn text_field_left_mouse_down_does_not_start_editing() {
    #[derive(Default)]
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 69;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .build(app)
            .width(140.)
            .height(40.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);

    let location = pane.location(FIELD).expect("field present");
    pane.move_to(&mut state, location);
    pane.press(&mut state);

    assert!(!pane.pane_state.text_field_is_focused(FIELD));

    pane.release(&mut state);
    assert!(pane.pane_state.text_field_is_focused(FIELD));
}

#[test]
fn text_field_types_multiple_characters() {
    #[derive(Default)]
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 41;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .build(app)
            .width(140.)
            .height(40.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);
    let location = pane.location(FIELD).expect("field present");
    pane.click(&mut state, location);
    pane.redraw(&mut state, 300, 200, 1.0);

    for ch in ["h", "e", "l", "l", "o"] {
        pane.key_pressed(&mut state, ch);
        pane.redraw(&mut state, 300, 200, 1.0);
    }

    assert_eq!(state.text.text, "hello");
}

#[test]
fn text_field_keeps_horizontal_cursor_in_view_after_edit() {
    #[derive(Default)]
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 70;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .build(app)
            .width(60.)
            .height(32.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);
    let location = pane.location(FIELD).expect("field present");
    pane.click(&mut state, location);
    pane.redraw(&mut state, 300, 200, 1.0);

    for _ in 0..30 {
        pane.key_pressed(&mut state, "w");
    }

    let (frame, _) = pane.redraw(&mut state, 300, 200, 1.0);

    assert!(state.text.viewport.x > 0.);
    assert!(
        frame
            .items
            .iter()
            .any(|item| matches!(item, crate::render::RenderItem::PushLayer { .. }))
    );
}

#[test]
fn text_field_arrow_navigation_updates_viewport() {
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 77;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .build(app)
            .width(60.)
            .height(32.)
    }

    let mut state = State {
        text: TextState::new("wwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwww"),
    };
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);
    state.text.begin_editing(&mut pane.pane_state);
    pane.redraw(&mut state, 300, 200, 1.0);

    pane.key_pressed(&mut state, NamedKey::End);

    assert!(state.text.viewport.x > 0.);
}

#[test]
fn wrapped_text_field_keeps_vertical_cursor_in_view_after_edit() {
    #[derive(Default)]
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 71;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .wrap()
            .build(app)
            .width(60.)
            .height(32.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);
    let location = pane.location(FIELD).expect("field present");
    pane.click(&mut state, location);
    pane.redraw(&mut state, 300, 200, 1.0);

    for _ in 0..80 {
        pane.key_pressed(&mut state, "w");
    }

    pane.redraw(&mut state, 300, 200, 1.0);

    assert!(state.text.viewport.y > 0.);
}

#[test]
fn text_field_horizontal_arrows_do_not_jitter_vertical_viewport() {
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 93;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .build(app)
            .width(160.)
            .height(28.)
    }

    let mut state = State {
        text: TextState::new("one\ntwo\nthree"),
    };
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);
    state.text.begin_editing(&mut pane.pane_state);
    pane.redraw(&mut state, 300, 200, 1.0);

    pane.key_pressed(&mut state, NamedKey::ArrowLeft);
    let y = state.text.viewport.y;

    for key in [
        NamedKey::ArrowRight,
        NamedKey::ArrowLeft,
        NamedKey::ArrowRight,
        NamedKey::ArrowLeft,
    ] {
        pane.key_pressed(&mut state, key);
        assert!(
            (state.text.viewport.y - y).abs() < 0.01,
            "viewport y changed from {y} to {}",
            state.text.viewport.y
        );
    }
}

#[test]
fn inactive_text_field_scrolls_viewport_without_editing() {
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 72;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .build(app)
            .width(60.)
            .height(32.)
    }

    let mut state = State {
        text: TextState::new("wwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwww"),
    };
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);
    let location = pane.location(FIELD).expect("field present");
    pane.move_to(&mut state, location);

    pane.scroll(&mut state, ScrollDelta { x: -80., y: 0. });

    assert!(!pane.pane_state.text_field_is_focused(FIELD));
    assert!(state.text.viewport.x > 0.);
}

#[test]
fn inactive_text_field_keeps_non_overflow_text_in_field_width() {
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 80;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .build(app)
            .width(160.)
            .height(40.)
    }

    let mut state = State {
        text: TextState::new("idle hue"),
    };
    let mut pane = test_pane(PaneBuilder::new("test", view));
    let (frame, _) = pane.redraw(&mut state, 300, 200, 1.0);
    let editor_area = pane.pane_state.editor_areas[&FIELD];
    let layout_width = frame
        .items
        .iter()
        .find_map(|item| match item {
            crate::render::RenderItem::Layout { layout, .. } => Some(layout.width()),
            _ => None,
        })
        .expect("text layout rendered");

    assert!(
        layout_width <= editor_area.width,
        "layout_width={layout_width} editor_width={}",
        editor_area.width
    );
}

#[test]
fn text_field_without_overflow_does_not_scroll_or_pulse_edges() {
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 81;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .build(app)
            .width(160.)
            .height(40.)
    }

    let mut state = State {
        text: TextState::new("idle hue"),
    };
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);
    let location = pane.location(FIELD).expect("field present");
    pane.move_to(&mut state, location);

    pane.scroll(&mut state, ScrollDelta { x: -80., y: -80. });

    assert_eq!(state.text.viewport, Default::default());
    assert!(
        !state
            .text
            .edge_feedback
            .is_animating(std::time::Instant::now())
    );
}

#[test]
fn text_field_without_overflow_does_not_capture_scroller_scroll() {
    struct State {
        text: TextState,
    }

    const SCROLLER: u64 = 82;
    const FIELD: u64 = 83;

    fn row_id(index: usize) -> u64 {
        id!(SCROLLER, index as u64)
    }

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        scroller(
            SCROLLER,
            None,
            |index, _, app| {
                if index == 0 {
                    return Some(
                        text_field(FIELD, binding!(state, State, text))
                            .build(app)
                            .height(40.),
                    );
                }
                if index >= 10 {
                    return None;
                }
                Some(
                    text(row_id(index), format!("row {index}"))
                        .build(app)
                        .height(60.),
                )
            },
            app,
        )
        .height(90.)
    }

    let mut state = State {
        text: TextState::new("idle hue"),
    };
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);
    assert!(pane.elements.contains_key(&FIELD));
    let location = pane.location(FIELD).expect("field present");
    pane.move_to(&mut state, location);

    pane.scroll(&mut state, ScrollDelta { x: 0., y: -200. });
    pane.redraw(&mut state, 300, 200, 1.0);

    assert_eq!(state.text.viewport, Default::default());
    assert!(!pane.elements.contains_key(&FIELD));
}

#[test]
fn text_field_horizontal_overflow_captures_horizontal_not_vertical_scroll() {
    struct State {
        text: TextState,
    }

    const SCROLLER: u64 = 84;
    const FIELD: u64 = 85;

    fn row_id(index: usize) -> u64 {
        id!(SCROLLER, index as u64)
    }

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        scroller(
            SCROLLER,
            None,
            |index, _, app| {
                if index == 0 {
                    return Some(
                        text_field(FIELD, binding!(state, State, text))
                            .build(app)
                            .width(60.)
                            .height(40.),
                    );
                }
                if index >= 10 {
                    return None;
                }
                Some(
                    text(row_id(index), format!("row {index}"))
                        .build(app)
                        .height(60.),
                )
            },
            app,
        )
        .height(90.)
    }

    let mut state = State {
        text: TextState::new("wwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwww"),
    };
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);
    let location = pane.location(FIELD).expect("field present");
    pane.move_to(&mut state, location);

    pane.scroll(&mut state, ScrollDelta { x: -80., y: 0. });
    pane.redraw(&mut state, 300, 200, 1.0);

    assert!(state.text.viewport.x > 0.);
    assert!(pane.elements.contains_key(&FIELD));

    pane.scroll(&mut state, ScrollDelta { x: 0., y: -200. });
    pane.redraw(&mut state, 300, 200, 1.0);

    assert!(!pane.elements.contains_key(&FIELD));
}

#[test]
fn text_field_focus_does_not_move_text_layout() {
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 82;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .build(app)
            .width(160.)
            .height(40.)
    }

    fn text_origin(frame: &crate::render::Frame) -> (f64, f64) {
        frame
            .items
            .iter()
            .find_map(|item| match item {
                crate::render::RenderItem::Layout { transform, .. } => {
                    let coeffs = transform.as_coeffs();
                    Some((coeffs[4], coeffs[5]))
                }
                _ => None,
            })
            .expect("text layout rendered")
    }

    let mut state = State {
        text: TextState::new("idle hue"),
    };
    let mut pane = test_pane(PaneBuilder::new("test", view));
    let (frame, _) = pane.redraw(&mut state, 300, 200, 1.0);
    let before = text_origin(&frame);
    let location = pane.location(FIELD).expect("field present");

    pane.click(&mut state, location);
    let (frame, _) = pane.redraw(&mut state, 300, 200, 1.0);
    let after = text_origin(&frame);

    assert!((before.0 - after.0).abs() < 0.01);
    assert!((before.1 - after.1).abs() < 0.01);
}

#[test]
fn text_field_click_uses_rendered_text_origin() {
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 83;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .build(app)
            .width(180.)
            .height(44.)
    }

    let mut state = State {
        text: TextState::new("idle hue"),
    };
    let mut pane = test_pane(PaneBuilder::new("test", view));
    let (frame, _) = pane.redraw(&mut state, 300, 200, 1.0);
    let editor_area = pane.pane_state.editor_areas[&FIELD];
    let origin = frame
        .items
        .iter()
        .find_map(|item| match item {
            crate::render::RenderItem::Layout { transform, .. } => {
                let coeffs = transform.as_coeffs();
                Some((coeffs[4], coeffs[5]))
            }
            _ => None,
        })
        .expect("text layout rendered");
    let click_x = (origin.0 - 5.).max(editor_area.x as f64 + 1.);
    let click_y = origin.1 + 5.;

    pane.click(&mut state, Point::new(click_x, click_y));
    pane.key_pressed(&mut state, "x");

    assert_eq!(state.text.text, "xidle hue");
}

#[test]
fn text_field_scroll_edge_feedback_pulses_at_limits() {
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 78;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .build(app)
            .width(60.)
            .height(32.)
    }

    let mut state = State {
        text: TextState::new("wwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwww"),
    };
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);
    let location = pane.location(FIELD).expect("field present");
    pane.move_to(&mut state, location);

    pane.scroll(&mut state, ScrollDelta { x: 80., y: 0. });

    assert!(
        state
            .text
            .edge_feedback
            .is_animating(std::time::Instant::now())
    );
}

#[test]
fn empty_focused_text_field_cursor_uses_text_metrics() {
    #[derive(Default)]
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 79;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .build(app)
            .width(120.)
            .height(100.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);
    let location = pane.location(FIELD).expect("field present");
    pane.click(&mut state, location);
    let (frame, _) = pane.redraw(&mut state, 300, 200, 1.0);
    let editor_area = pane.pane_state.editor_areas[&FIELD];
    let cursor_area = frame
        .items
        .iter()
        .filter_map(|item| match item {
            crate::render::RenderItem::Path { area, .. }
                if area.width <= 2. && area.height > 1. =>
            {
                Some(*area)
            }
            _ => None,
        })
        .next()
        .expect("cursor rendered");

    assert!(cursor_area.height < editor_area.height * 0.5);
}

#[test]
fn text_field_clicking_cursor_position_does_not_scroll_viewport() {
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 73;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .build(app)
            .width(60.)
            .height(32.)
    }

    let mut state = State {
        text: TextState::new("wwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwww"),
    };
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);
    let location = pane.location(FIELD).expect("field present");
    pane.move_to(&mut state, location);
    pane.scroll(&mut state, ScrollDelta { x: -80., y: 0. });
    let scrolled_x = state.text.viewport.x;

    pane.click(&mut state, location);

    assert_eq!(state.text.viewport.x, scrolled_x);
}

#[test]
fn text_field_viewport_allows_end_cursor_width() {
    #[derive(Default)]
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 74;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .build(app)
            .width(60.)
            .height(32.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);
    let location = pane.location(FIELD).expect("field present");
    pane.click(&mut state, location);
    pane.redraw(&mut state, 300, 200, 1.0);

    for _ in 0..30 {
        pane.key_pressed(&mut state, "w");
    }

    pane.redraw(&mut state, 300, 200, 1.0);
    let editor_area = pane.pane_state.editor_areas[&FIELD];
    let layout = state
        .text
        .editor
        .editor
        .layout(&mut pane.pane_state.font_cx, &mut pane.pane_state.layout_cx)
        .clone();

    assert!(state.text.viewport.x > (layout.width() - editor_area.width).max(0.));
}

#[test]
fn text_field_trailing_spaces_keep_cursor_in_view() {
    #[derive(Default)]
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 84;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .build(app)
            .width(60.)
            .height(32.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);
    let location = pane.location(FIELD).expect("field present");
    pane.click(&mut state, location);
    pane.redraw(&mut state, 300, 200, 1.0);

    for ch in ["w", "w", "w", "w", " ", " ", " ", " ", " ", " ", " ", " "] {
        pane.key_pressed(&mut state, ch);
    }

    let (frame, _) = pane.redraw(&mut state, 300, 200, 1.0);
    let editor_area = pane.pane_state.editor_areas[&FIELD];
    let cursor_area = frame
        .items
        .iter()
        .filter_map(|item| match item {
            crate::render::RenderItem::Path { area, .. }
                if area.width <= 2. && area.height > 1. =>
            {
                Some(*area)
            }
            _ => None,
        })
        .next()
        .expect("cursor rendered");

    assert!(
        cursor_area.x + cursor_area.width <= editor_area.x + editor_area.width + 0.01,
        "cursor right={} editor right={} viewport={:?}",
        cursor_area.x + cursor_area.width,
        editor_area.x + editor_area.width,
        state.text.viewport
    );
}

#[test]
fn inactive_centered_text_field_accounts_for_trailing_spaces() {
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 92;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .build(app)
            .width(160.)
            .height(40.)
    }

    fn text_metrics(text: &str) -> (f64, f32, f32) {
        let mut state = State {
            text: TextState::new(text),
        };
        let mut pane = test_pane(PaneBuilder::new("test", view));
        let (frame, _) = pane.redraw(&mut state, 300, 200, 1.0);
        frame
            .items
            .iter()
            .find_map(|item| match item {
                crate::render::RenderItem::Layout { layout, transform } => Some((
                    transform.as_coeffs()[4],
                    layout.width(),
                    layout.full_width(),
                )),
                _ => None,
            })
            .expect("text layout rendered")
    }

    let (plain_x, plain_width, plain_full_width) = text_metrics("w");
    let (padded_x, padded_width, padded_full_width) = text_metrics("w            ");

    assert!(
        padded_x < plain_x - 1.,
        "padded_x={padded_x} plain_x={plain_x} padded_width={padded_width} plain_width={plain_width} padded_full_width={padded_full_width} plain_full_width={plain_full_width}"
    );
}

#[test]
fn text_field_drag_select_inside_viewport_does_not_scroll() {
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 75;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .build(app)
            .width(80.)
            .height(32.)
    }

    let mut state = State {
        text: TextState::new("wwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwww"),
    };
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);
    let location = pane.location(FIELD).expect("field present");
    pane.move_to(&mut state, location);
    pane.scroll(&mut state, ScrollDelta { x: -80., y: 0. });
    let scrolled_x = state.text.viewport.x;

    pane.drag(
        &mut state,
        location,
        Point::new(location.x + 20., location.y),
    );

    assert_eq!(state.text.viewport.x, scrolled_x);
}

#[test]
fn text_field_drag_select_past_viewport_edge_scrolls() {
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 76;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .build(app)
            .width(80.)
            .height(32.)
    }

    let mut state = State {
        text: TextState::new("wwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwww"),
    };
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);
    let location = pane.location(FIELD).expect("field present");

    pane.drag(
        &mut state,
        location,
        Point::new(location.x + 100., location.y),
    );

    assert!(state.text.viewport.x > 0.);
}

#[test]
fn text_field_right_click_starts_editing() {
    struct State {
        text: TextState,
        edits: Vec<EditInteraction>,
    }

    impl Default for State {
        fn default() -> Self {
            Self {
                text: TextState::new(""),
                edits: Vec::new(),
            }
        }
    }

    const FIELD: u64 = 67;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .on_edit(|state, _, edit| state.edits.push(edit))
            .build(app)
            .width(140.)
            .height(40.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);
    let location = pane.location(FIELD).expect("field present");

    pane.move_to(&mut state, location);
    pane.press_button(&mut state, MouseButton::Right);
    pane.release_button(&mut state, MouseButton::Right);
    pane.redraw(&mut state, 300, 200, 1.0);

    assert!(pane.pane_state.text_field_is_focused(FIELD));
    assert!(matches!(state.edits.last(), Some(EditInteraction::Start)));

    pane.key_pressed(&mut state, "a");

    assert_eq!(state.text.text, "a");
}

#[test]
fn text_field_backspace_removes_character() {
    #[derive(Default)]
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 42;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .build(app)
            .width(140.)
            .height(40.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);
    let location = pane.location(FIELD).expect("field present");
    pane.click(&mut state, location);
    pane.redraw(&mut state, 300, 200, 1.0);

    for ch in ["a", "b", "c"] {
        pane.key_pressed(&mut state, ch);
        pane.redraw(&mut state, 300, 200, 1.0);
    }
    assert_eq!(state.text.text, "abc");

    pane.key_pressed(&mut state, NamedKey::Backspace);
    assert_eq!(state.text.text, "ab");

    pane.key_pressed(&mut state, NamedKey::Backspace);
    pane.key_pressed(&mut state, NamedKey::Backspace);
    assert_eq!(state.text.text, "");
}

#[test]
fn text_field_arrow_then_insert_places_caret() {
    #[derive(Default)]
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 43;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .build(app)
            .width(140.)
            .height(40.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);
    let location = pane.location(FIELD).expect("field present");
    pane.click(&mut state, location);
    pane.redraw(&mut state, 300, 200, 1.0);

    for ch in ["a", "c"] {
        pane.key_pressed(&mut state, ch);
        pane.redraw(&mut state, 300, 200, 1.0);
    }
    pane.key_pressed(&mut state, NamedKey::ArrowLeft);
    pane.key_pressed(&mut state, "b");

    assert_eq!(state.text.text, "abc");
}

#[test]
fn text_state_select_all_replaces_with_next_key() {
    #[derive(Default)]
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 62;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .build(app)
            .width(140.)
            .height(40.)
    }

    let mut state = State {
        text: TextState::new("hello"),
    };
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);
    let location = pane.location(FIELD).expect("field present");
    pane.click(&mut state, location);
    pane.redraw(&mut state, 300, 200, 1.0);

    state.text.select_all_text(&mut pane.pane_state);
    pane.key_pressed(&mut state, "x");

    assert_eq!(state.text.text, "x");
}

#[test]
fn text_field_start_edit_can_select_all() {
    struct State {
        text: TextState,
    }

    impl Default for State {
        fn default() -> Self {
            Self {
                text: TextState::new("hello"),
            }
        }
    }

    const FIELD: u64 = 66;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .on_edit(|state, app, edit| {
                if matches!(edit, EditInteraction::Start) {
                    state.text.select_all_text(app);
                }
            })
            .build(app)
            .width(140.)
            .height(40.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);
    let location = pane.location(FIELD).expect("field present");
    pane.click(&mut state, location);
    pane.redraw(&mut state, 300, 200, 1.0);

    pane.key_pressed(&mut state, "x");

    assert_eq!(state.text.text, "x");
}

#[test]
fn text_field_enter_ends_editing_when_configured() {
    #[derive(Default)]
    struct State {
        text: TextState,
        edits: Vec<EditInteraction>,
    }

    const FIELD: u64 = 44;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .enter_end_editing()
            .on_edit(|state, _, edit| state.edits.push(edit))
            .build(app)
            .width(140.)
            .height(40.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);
    let location = pane.location(FIELD).expect("field present");
    pane.click(&mut state, location);
    pane.redraw(&mut state, 300, 200, 1.0);
    assert!(pane.pane_state.text_field_is_focused(FIELD));

    pane.key_pressed(&mut state, NamedKey::Enter);

    assert!(!pane.pane_state.text_field_is_focused(FIELD));
    assert!(matches!(state.edits.last(), Some(EditInteraction::End)));
}

#[test]
fn text_field_escape_ends_editing_when_configured() {
    #[derive(Default)]
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 45;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .esc_end_editing()
            .build(app)
            .width(140.)
            .height(40.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);
    let location = pane.location(FIELD).expect("field present");
    pane.click(&mut state, location);
    pane.redraw(&mut state, 300, 200, 1.0);
    assert!(pane.pane_state.text_field_is_focused(FIELD));

    pane.key_pressed(&mut state, NamedKey::Escape);

    assert!(!pane.pane_state.text_field_is_focused(FIELD));
}

#[test]
fn text_field_enter_does_not_end_editing_by_default() {
    #[derive(Default)]
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 46;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .build(app)
            .width(140.)
            .height(40.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);
    let location = pane.location(FIELD).expect("field present");
    pane.click(&mut state, location);
    pane.redraw(&mut state, 300, 200, 1.0);

    pane.key_pressed(&mut state, NamedKey::Enter);

    assert!(pane.pane_state.text_field_is_focused(FIELD));
}

#[test]
fn text_field_click_outside_can_be_user_handled() {
    #[derive(Default)]
    struct State {
        text: TextState,
        edits: Vec<EditInteraction>,
        outside: bool,
    }

    const FIELD: u64 = 47;
    const OUTSIDE: u64 = 64;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        column_spaced(
            10.,
            vec![
                text_field(FIELD, binding!(state, State, text))
                    .on_edit(|state, _, edit| state.edits.push(edit))
                    .build(app)
                    .width(140.)
                    .height(40.),
                rect(OUTSIDE)
                    .fill(TRANSPARENT)
                    .view()
                    .gesture(
                        gesture::click(id!(OUTSIDE, 1u64))
                            .button(MouseButton::Left)
                            .run(|state: &mut State, app, event| {
                                if event.state == ClickPhase::Started {
                                    state.text.end_editing(app);
                                    state.outside = true;
                                }
                            }),
                    )
                    .build(app)
                    .width(140.)
                    .height(40.),
            ],
        )
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);
    let location = pane.location(FIELD).expect("field present");
    pane.click(&mut state, location);
    pane.redraw(&mut state, 300, 200, 1.0);
    assert!(pane.pane_state.text_field_is_focused(FIELD));

    let outside = pane.location(OUTSIDE).expect("outside target present");
    pane.click(&mut state, outside);
    pane.redraw(&mut state, 300, 200, 1.0);

    assert!(state.outside);
    assert!(!pane.pane_state.text_field_is_focused(FIELD));
    assert!(matches!(state.edits.last(), Some(EditInteraction::End)));
}

#[test]
fn text_field_right_click_does_not_end_editing() {
    #[derive(Default)]
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 63;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .build(app)
            .width(140.)
            .height(40.)
    }

    let mut state = State {
        text: TextState::new("hello"),
    };
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);
    let location = pane.location(FIELD).expect("field present");
    pane.click(&mut state, location);
    pane.redraw(&mut state, 300, 200, 1.0);
    assert!(pane.pane_state.text_field_is_focused(FIELD));

    pane.move_to(&mut state, location);
    pane.press_button(&mut state, MouseButton::Right);
    pane.release_button(&mut state, MouseButton::Right);
    pane.redraw(&mut state, 300, 200, 1.0);

    assert!(pane.pane_state.text_field_is_focused(FIELD));
}

#[test]
fn text_state_end_editing_clears_active_editor() {
    #[derive(Default)]
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 65;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .build(app)
            .width(140.)
            .height(40.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);
    let location = pane.location(FIELD).expect("field present");
    pane.click(&mut state, location);
    pane.redraw(&mut state, 300, 200, 1.0);
    assert!(pane.pane_state.text_field_is_focused(FIELD));

    state.text.end_editing(&mut pane.pane_state);
    pane.redraw(&mut state, 300, 200, 1.0);

    assert!(!pane.pane_state.text_field_is_focused(FIELD));
}

#[test]
fn text_state_begin_editing_focuses_matching_text_field() {
    #[derive(Default)]
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 67;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .build(app)
            .width(140.)
            .height(40.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);

    state.text.begin_editing(&mut pane.pane_state);
    pane.redraw(&mut state, 300, 200, 1.0);

    assert!(pane.pane_state.text_field_is_focused(FIELD));
}

#[test]
fn text_state_begin_editing_places_cursor_at_end_by_default() {
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 86;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .build(app)
            .width(140.)
            .height(40.)
    }

    let mut state = State {
        text: TextState::new("hello"),
    };
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);

    state.text.begin_editing(&mut pane.pane_state);
    pane.redraw(&mut state, 300, 200, 1.0);
    pane.key_pressed(&mut state, "x");

    assert_eq!(state.text.text, "hellox");
}

#[test]
fn text_state_begin_editing_renders_caret_without_click() {
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 89;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .build(app)
            .width(140.)
            .height(40.)
    }

    let mut state = State {
        text: TextState::new("hello"),
    };
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);

    state.text.begin_editing(&mut pane.pane_state);
    let (frame, _) = pane.redraw(&mut state, 300, 200, 1.0);

    assert!(
        frame.items.iter().any(|item| matches!(
            item,
            crate::render::RenderItem::Path { area, .. }
                if area.width <= 2. && area.height > 1.
        )),
        "programmatic focus should render a caret"
    );
}

#[test]
fn text_state_begin_editing_with_cursor_start_inserts_at_start() {
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 87;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .build(app)
            .width(140.)
            .height(40.)
    }

    let mut state = State {
        text: TextState::new("hello"),
    };
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);

    state
        .text
        .begin_editing_with(&mut pane.pane_state, InitialSelection::Start);
    pane.redraw(&mut state, 300, 200, 1.0);
    pane.key_pressed(&mut state, "x");

    assert_eq!(state.text.text, "xhello");
}

#[test]
fn text_state_begin_editing_with_select_all_replaces_text() {
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 88;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .build(app)
            .width(140.)
            .height(40.)
    }

    let mut state = State {
        text: TextState::new("hello"),
    };
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);

    state
        .text
        .begin_editing_with(&mut pane.pane_state, InitialSelection::All);
    pane.redraw(&mut state, 300, 200, 1.0);
    pane.key_pressed(&mut state, "x");

    assert_eq!(state.text.text, "x");
}

#[test]
fn text_state_begin_editing_refocuses_after_another_field_takes_focus() {
    #[derive(Default)]
    struct State {
        a: TextState,
        b: TextState,
    }

    const FIELD_A: u64 = 84;
    const FIELD_B: u64 = 85;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        column(vec![
            text_field(FIELD_A, binding!(state, State, a))
                .build(app)
                .width(140.)
                .height(40.),
            text_field(FIELD_B, binding!(state, State, b))
                .build(app)
                .width(140.)
                .height(40.),
        ])
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);

    state.b.begin_editing(&mut pane.pane_state);
    pane.redraw(&mut state, 300, 200, 1.0);
    assert!(pane.pane_state.text_field_is_focused(FIELD_B));

    state.a.begin_editing(&mut pane.pane_state);
    pane.redraw(&mut state, 300, 200, 1.0);
    assert!(pane.pane_state.text_field_is_focused(FIELD_A));

    state.b.begin_editing(&mut pane.pane_state);
    pane.redraw(&mut state, 300, 200, 1.0);
    assert!(pane.pane_state.text_field_is_focused(FIELD_B));
}

#[test]
fn text_state_begin_and_end_editing_send_lifecycle_callbacks() {
    #[derive(Default)]
    struct State {
        text: TextState,
        edits: Vec<EditInteraction>,
    }

    const FIELD: u64 = 93;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .on_edit(|state, _, edit| state.edits.push(edit))
            .build(app)
            .width(140.)
            .height(40.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);

    state.text.begin_editing(&mut pane.pane_state);
    pane.redraw(&mut state, 300, 200, 1.0);
    state.text.end_editing(&mut pane.pane_state);
    pane.redraw(&mut state, 300, 200, 1.0);

    assert!(matches!(state.edits.first(), Some(EditInteraction::Start)));
    assert!(matches!(state.edits.last(), Some(EditInteraction::End)));
}

#[test]
fn text_state_begin_editing_defocuses_previous_field_in_same_redraw() {
    #[derive(Default)]
    struct State {
        a: TextState,
        b: TextState,
    }

    const FIELD_A: u64 = 90;
    const FIELD_B: u64 = 91;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        column(vec![
            text_field(FIELD_A, binding!(state, State, a))
                .build(app)
                .width(140.)
                .height(40.),
            text_field(FIELD_B, binding!(state, State, b))
                .build(app)
                .width(140.)
                .height(40.),
        ])
    }

    let mut state = State {
        a: TextState::new("a"),
        b: TextState::new("b"),
    };
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);

    state.a.begin_editing(&mut pane.pane_state);
    pane.redraw(&mut state, 300, 200, 1.0);
    state.b.begin_editing(&mut pane.pane_state);
    let (frame, _) = pane.redraw(&mut state, 300, 200, 1.0);
    let b_area = pane.pane_state.editor_areas[&FIELD_B];
    let cursors: Vec<_> = frame
        .items
        .iter()
        .filter_map(|item| match item {
            crate::render::RenderItem::Path { area, .. }
                if area.width <= 2. && area.height > 1. =>
            {
                Some(*area)
            }
            _ => None,
        })
        .collect();

    assert_eq!(cursors.len(), 1);
    assert!(cursors[0].y >= b_area.y);
    assert!(cursors[0].y <= b_area.y + b_area.height);
}

#[test]
fn text_state_end_editing_cancels_unapplied_focus_request() {
    #[derive(Default)]
    struct State {
        text: TextState,
    }

    const FIELD: u64 = 68;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        text_field(FIELD, binding!(state, State, text))
            .build(app)
            .width(140.)
            .height(40.)
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 300, 200, 1.0);

    state.text.begin_editing(&mut pane.pane_state);
    state.text.end_editing(&mut pane.pane_state);
    pane.redraw(&mut state, 300, 200, 1.0);

    assert!(!pane.pane_state.text_field_is_focused(FIELD));
}

#[test]
fn text_field_focus_switches_between_two_fields() {
    #[derive(Default)]
    struct State {
        a: TextState,
        b: TextState,
    }

    const FIELD_A: u64 = 48;
    const FIELD_B: u64 = 49;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        column_spaced(
            10.,
            vec![
                text_field(FIELD_A, binding!(state, State, a))
                    .build(app)
                    .width(140.)
                    .height(40.),
                text_field(FIELD_B, binding!(state, State, b))
                    .build(app)
                    .width(140.)
                    .height(40.),
            ],
        )
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 400, 400, 1.0);

    let location = pane.location(FIELD_A).expect("field a present");
    pane.click(&mut state, location);
    pane.redraw(&mut state, 400, 400, 1.0);
    pane.key_pressed(&mut state, "x");
    pane.redraw(&mut state, 400, 400, 1.0);
    assert_eq!(state.a.text, "x");
    assert!(pane.pane_state.text_field_is_focused(FIELD_A));

    let location = pane.location(FIELD_B).expect("field b present");
    pane.click(&mut state, location);
    pane.redraw(&mut state, 400, 400, 1.0);
    assert!(pane.pane_state.text_field_is_focused(FIELD_B));

    pane.key_pressed(&mut state, "y");
    pane.redraw(&mut state, 400, 400, 1.0);
    assert_eq!(state.a.text, "x");
    assert_eq!(state.b.text, "y");
}

#[test]
fn text_field_focus_switch_sends_end_to_previous_field() {
    #[derive(Default)]
    struct State {
        a: TextState,
        b: TextState,
        edits: Vec<(&'static str, EditInteraction)>,
    }

    const FIELD_A: u64 = 70;
    const FIELD_B: u64 = 71;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        column_spaced(
            10.,
            vec![
                text_field(FIELD_A, binding!(state, State, a))
                    .on_edit(|state, _, edit| state.edits.push(("a", edit)))
                    .build(app)
                    .width(140.)
                    .height(40.),
                text_field(FIELD_B, binding!(state, State, b))
                    .on_edit(|state, _, edit| state.edits.push(("b", edit)))
                    .build(app)
                    .width(140.)
                    .height(40.),
            ],
        )
    }

    let mut state = State::default();
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 400, 400, 1.0);

    let a_location = pane.location(FIELD_A).expect("field a present");
    pane.click(&mut state, a_location);
    let b_location = pane.location(FIELD_B).expect("field b present");
    pane.click(&mut state, b_location);

    assert!(
        state
            .edits
            .iter()
            .any(|(field, edit)| *field == "a" && matches!(edit, EditInteraction::End))
    );
    assert!(
        state
            .edits
            .iter()
            .any(|(field, edit)| *field == "b" && matches!(edit, EditInteraction::Start))
    );
}

#[test]
fn text_field_focus_switch_sets_new_editor_before_previous_end_callback() {
    #[derive(Default)]
    struct State {
        a: TextState,
        b: TextState,
    }

    const FIELD_A: u64 = 72;
    const FIELD_B: u64 = 73;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        column_spaced(
            10.,
            vec![
                text_field(FIELD_A, binding!(state, State, a))
                    .on_edit(|state, app, edit| {
                        if matches!(edit, EditInteraction::End) {
                            if !app.text_field_is_focused(FIELD_A) {
                                state.a = TextState::new("synced a");
                            }
                            if !app.text_field_is_focused(FIELD_B) {
                                state.b = TextState::new("synced b");
                            }
                        }
                    })
                    .build(app)
                    .width(140.)
                    .height(40.),
                text_field(FIELD_B, binding!(state, State, b))
                    .build(app)
                    .width(140.)
                    .height(40.),
            ],
        )
    }

    let mut state = State {
        a: TextState::new("a"),
        b: TextState::new("b"),
    };
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 400, 400, 1.0);

    let a_location = pane.location(FIELD_A).expect("field a present");
    pane.click(&mut state, a_location);
    let b_location = pane.location(FIELD_B).expect("field b present");
    pane.click(&mut state, b_location);

    assert_eq!(state.a.text, "synced a");
    assert_eq!(state.b.text, "b");
    assert!(pane.pane_state.text_field_is_focused(FIELD_B));
}

#[test]
fn text_state_recreated_field_can_focus_again_with_same_view_id() {
    #[derive(Default)]
    struct State {
        a: TextState,
        b: TextState,
    }

    const FIELD_A: u64 = 74;
    const FIELD_B: u64 = 75;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        column_spaced(
            10.,
            vec![
                text_field(FIELD_A, binding!(state, State, a))
                    .on_edit(|state, _, edit| {
                        if matches!(edit, EditInteraction::End) {
                            state.a = TextState::new("synced a");
                        }
                    })
                    .build(app)
                    .width(140.)
                    .height(40.),
                text_field(FIELD_B, binding!(state, State, b))
                    .build(app)
                    .width(140.)
                    .height(40.),
            ],
        )
    }

    let mut state = State {
        a: TextState::new("a"),
        b: TextState::new("b"),
    };
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 400, 400, 1.0);

    state.a.begin_editing(&mut pane.pane_state);
    pane.redraw(&mut state, 400, 400, 1.0);
    assert!(pane.pane_state.text_field_is_focused(FIELD_A));

    state.b.begin_editing(&mut pane.pane_state);
    pane.redraw(&mut state, 400, 400, 1.0);
    assert!(pane.pane_state.text_field_is_focused(FIELD_B));
    assert_eq!(state.a.text, "synced a");

    state.a.begin_editing(&mut pane.pane_state);
    pane.redraw(&mut state, 400, 400, 1.0);

    assert!(pane.pane_state.text_field_is_focused(FIELD_A));
}

#[test]
fn text_field_drag_in_second_field_switches_focus() {
    #[derive(Default)]
    struct State {
        a: TextState,
        b: TextState,
    }

    const FIELD_A: u64 = 60;
    const FIELD_B: u64 = 61;

    fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        column_spaced(
            10.,
            vec![
                text_field(FIELD_A, binding!(state, State, a))
                    .build(app)
                    .width(140.)
                    .height(40.),
                text_field(FIELD_B, binding!(state, State, b))
                    .build(app)
                    .width(140.)
                    .height(40.),
            ],
        )
    }

    let mut state = State {
        a: TextState::new("hello"),
        b: TextState::new("world"),
    };
    let mut pane = test_pane(PaneBuilder::new("test", view));
    pane.redraw(&mut state, 400, 400, 1.0);

    let a_location = pane.location(FIELD_A).expect("field a present");
    pane.move_to(&mut state, a_location);
    pane.press(&mut state);
    pane.move_to(&mut state, Point::new(a_location.x + 30., a_location.y));
    pane.release(&mut state);
    pane.redraw(&mut state, 400, 400, 1.0);
    assert!(pane.pane_state.text_field_is_focused(FIELD_A));

    let b_location = pane.location(FIELD_B).expect("field b present");
    pane.move_to(&mut state, b_location);
    pane.press(&mut state);
    pane.move_to(&mut state, Point::new(b_location.x + 30., b_location.y));
    pane.release(&mut state);
    pane.redraw(&mut state, 400, 400, 1.0);

    assert!(pane.pane_state.text_field_is_focused(FIELD_B));
}

#[test]
fn scroller_scroll_updates_state() {
    #[derive(Default)]
    struct State;

    const SCROLLER: u64 = 50;

    fn cell_id(index: usize) -> u64 {
        id!(index as u64)
    }

    fn view<'a>(_state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        scroller(
            SCROLLER,
            None,
            |index, _, app| {
                if index >= 10 {
                    return None;
                }
                Some(
                    text(cell_id(index), format!("row {index}"))
                        .build(app)
                        .height(60.),
                )
            },
            app,
        )
        .height(90.)
    }

    let mut state = State;
    let mut pane = test_pane(PaneBuilder::new("test", view));
    let (_, effects) = pane.redraw(&mut state, 300, 200, 1.0);
    assert!(effects.is_empty(), "unexpected effects: {effects:?}");

    assert!(pane.elements.contains_key(&cell_id(0)));

    let location = pane.location(SCROLLER).expect("scroller present");
    assert!(pane.move_to(&mut state, location).is_empty());
    assert!(
        pane.scroll(&mut state, ScrollDelta { x: 0., y: -200. })
            .is_empty()
    );
    let (_, effects) = pane.redraw(&mut state, 300, 200, 1.0);
    assert!(effects.is_empty(), "unexpected effects: {effects:?}");

    assert!(
        !pane.elements.contains_key(&cell_id(0)),
        "cell 0 should have scrolled out of view"
    );
}

#[test]
fn nested_scroller_receives_scroll_over_its_area() {
    #[derive(Default)]
    struct State;

    const OUTER: u64 = 51;
    const INNER: u64 = 52;

    fn view<'a>(_state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        scroller(
            OUTER,
            None,
            |index, _, app| {
                if index == 0 {
                    return Some(
                        scroller(
                            INNER,
                            None,
                            |index, _, app| {
                                if index >= 10 {
                                    return None;
                                }
                                Some(
                                    text(id!(INNER, index as u64), format!("inner {index}"))
                                        .build(app)
                                        .height(60.),
                                )
                            },
                            app,
                        )
                        .height(100.),
                    );
                }
                if index >= 10 {
                    return None;
                }
                Some(
                    text(id!(OUTER, index as u64), format!("outer {index}"))
                        .build(app)
                        .height(60.),
                )
            },
            app,
        )
        .height(120.)
    }

    let mut state = State;
    let mut pane = test_pane(PaneBuilder::new("test", view));
    let (_, effects) = pane.redraw(&mut state, 300, 200, 1.0);
    assert!(effects.is_empty(), "unexpected effects: {effects:?}");
    let outer_before = pane.pane_state.scrollers[&OUTER].engine.compensated;
    let inner_before = pane.pane_state.scrollers[&INNER].engine.compensated;

    let location = pane.location(INNER).expect("inner scroller present");
    pane.move_to(&mut state, location);
    assert!(
        pane.scroll(&mut state, ScrollDelta { x: 0., y: -200. })
            .is_empty()
    );
    let (_, effects) = pane.redraw(&mut state, 300, 200, 1.0);
    assert!(effects.is_empty(), "unexpected effects: {effects:?}");

    let outer = &pane.pane_state.scrollers[&OUTER].engine;
    let inner = &pane.pane_state.scrollers[&INNER].engine;
    assert_eq!(outer.compensated, outer_before);
    assert_ne!(inner.compensated, inner_before);
}

#[test]
fn wake_runs_on_wake_and_returns_pane_effects() {
    #[derive(Default)]
    struct State {
        value: u32,
    }

    fn view<'a>(_state: &'a State, _app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        empty()
    }

    fn on_wake(state: &mut State, app: &mut PaneState) {
        state.value = 42;
        app.redraw();
    }

    let mut state = State::default();
    let mut pane = PaneBuilder::new("test", view).on_wake(on_wake).build();

    let effects = pane.wake(&mut state);

    assert_eq!(state.value, 42);
    assert!(
        effects
            .into_iter()
            .any(|effect| matches!(effect, PaneEffect::Redraw))
    );
}

#[test]
fn shadow_primitive_emits_render_item() {
    struct State;

    fn view<'a>(_state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        shadow(70)
            .color(Color::BLACK.with_alpha(0.4))
            .blur(12.)
            .spread(4.)
            .corner_rounding(8.)
            .build(app)
            .width(100.)
            .height(40.)
    }

    let mut state = State;
    let mut pane = test_pane(PaneBuilder::new("test", view));
    let (frame, effects) = pane.redraw(&mut state, 300, 200, 1.0);

    assert!(effects.is_empty(), "unexpected effects: {effects:?}");
    assert!(
        frame
            .items
            .iter()
            .any(|item| matches!(item, crate::render::RenderItem::Shadow { .. }))
    );
}
