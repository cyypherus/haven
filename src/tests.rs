use crate::app::PaneEffect;
use crate::*;

fn test_pane<State: 'static>(builder: PaneBuilder<State>) -> crate::app::Pane<State> {
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
    assert!(state.toggle.hovered);
    assert!(pane.press(&mut state).is_empty());
    assert!(state.toggle.depressed);
    assert!(pane.release(&mut state).is_empty());

    assert!(state.toggle.on);
    assert!(!state.toggle.depressed);
    assert_eq!(state.toggled, Some(true));
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
            .on_click(MouseButton::Right, |state: &mut State, _, event| {
                state.events.push((event.state, event.location.local()));
            })
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
            .on_click(MouseButton::Left, |state: &mut State, _, event| {
                state.events.push(event.location.local());
            })
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
        stack(vec![
            rect(LEFT_TARGET)
                .fill(TRANSPARENT)
                .view()
                .on_click(MouseButton::Left, |state: &mut State, _, event| {
                    state.left_events.push(event.state);
                })
                .build(app)
                .width(80.)
                .height(40.),
            rect(RIGHT_TARGET)
                .fill(TRANSPARENT)
                .view()
                .on_click(MouseButton::Right, |state: &mut State, _, event| {
                    state.right_events.push(event.state);
                })
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
            .on_click(MouseButton::Right, |state: &mut State, _, event| {
                state.events.push(event.state);
            })
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
            .on_click(MouseButton::Left, |state: &mut State, _, event| {
                state.events.push(event.state);
            })
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
fn click_outside_is_button_specific() {
    #[derive(Default)]
    struct State {
        outside: Vec<ClickPhase>,
    }

    const LISTENER: u64 = 30;
    const TARGET: u64 = 31;

    fn view<'a>(_: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        row_spaced(
            20.,
            vec![
                rect(LISTENER)
                    .fill(TRANSPARENT)
                    .view()
                    .on_click_outside(MouseButton::Right, |state: &mut State, _, event| {
                        state.outside.push(event.state);
                    })
                    .build(app)
                    .width(60.)
                    .height(40.),
                rect(TARGET)
                    .fill(TRANSPARENT)
                    .view()
                    .on_click(MouseButton::Right, |_, _, _| {})
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
    assert!(state.slider.hovered);
    assert!(pane.press(&mut state).is_empty());
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
    assert!(state.text.editing);

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
    assert!(state.text.editing);

    pane.key_pressed(&mut state, NamedKey::Enter);

    assert!(!state.text.editing);
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
    assert!(state.text.editing);

    pane.key_pressed(&mut state, NamedKey::Escape);

    assert!(!state.text.editing);
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

    assert!(state.text.editing);
}

#[test]
fn text_field_click_outside_ends_editing() {
    #[derive(Default)]
    struct State {
        text: TextState,
        edits: Vec<EditInteraction>,
    }

    const FIELD: u64 = 47;

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
    pane.click(&mut state, location);
    pane.redraw(&mut state, 300, 200, 1.0);
    assert!(state.text.editing);

    let field_location = pane.location(FIELD).expect("field present");
    let outside = Point::new(field_location.x, field_location.y + 200.);
    pane.move_to(&mut state, outside);
    pane.press(&mut state);
    pane.release(&mut state);

    assert!(!state.text.editing);
    assert!(matches!(state.edits.last(), Some(EditInteraction::End)));
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
    assert!(state.a.editing);

    let location = pane.location(FIELD_B).expect("field b present");
    pane.click(&mut state, location);
    pane.redraw(&mut state, 400, 400, 1.0);
    assert!(!state.a.editing);
    assert!(state.b.editing);

    pane.key_pressed(&mut state, "y");
    pane.redraw(&mut state, 400, 400, 1.0);
    assert_eq!(state.a.text, "x");
    assert_eq!(state.b.text, "y");
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
    assert!(state.a.editing);
    assert!(!state.b.editing);

    let b_location = pane.location(FIELD_B).expect("field b present");
    pane.move_to(&mut state, b_location);
    pane.press(&mut state);
    pane.move_to(&mut state, Point::new(b_location.x + 30., b_location.y));
    pane.release(&mut state);
    pane.redraw(&mut state, 400, 400, 1.0);

    assert!(!state.a.editing);
    assert!(state.b.editing);
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
