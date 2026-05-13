use crate::*;

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

    let mut pane = PaneBuilder::new("test", view).build(State::default());

    let (_, effects) = pane.redraw(300, 200, 1.0);
    assert!(effects.is_empty(), "unexpected effects: {effects:?}");
    assert!(!pane.state.dropdown.expanded);

    assert!(pane.click(DROPDOWN).expect("dropdown present").is_empty());
    assert!(pane.state.dropdown.expanded);

    let (_, effects) = pane.redraw(300, 200, 1.0);
    assert!(effects.is_empty(), "unexpected effects: {effects:?}");

    assert!(pane.click(OPTIONS[1].0).expect("option present").is_empty());
    assert_eq!(pane.state.dropdown.selected, "two");
    assert_eq!(pane.state.selected, Some("two"));
    assert!(!pane.state.dropdown.expanded);
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

    let mut pane = PaneBuilder::new("test", view).build(State::default());
    let (_, effects) = pane.redraw(300, 200, 1.0);
    assert!(effects.is_empty(), "unexpected effects: {effects:?}");

    let location = pane.location(TOGGLE).expect("toggle present");
    assert!(pane.move_to(location).is_empty());
    assert!(pane.state.toggle.hovered);
    assert!(pane.press().is_empty());
    assert!(pane.state.toggle.depressed);
    assert!(pane.release().is_empty());

    assert!(pane.state.toggle.on);
    assert!(!pane.state.toggle.depressed);
    assert_eq!(pane.state.toggled, Some(true));
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

    let mut pane = PaneBuilder::new("test", view).build(State::default());
    let (_, effects) = pane.redraw(300, 200, 1.0);
    assert!(effects.is_empty(), "unexpected effects: {effects:?}");

    let location = pane.location(SLIDER).expect("slider present");
    assert!(pane.move_to(location).is_empty());
    assert!(pane.state.slider.hovered);
    assert!(pane.press().is_empty());
    assert!(pane.state.slider.dragging);
    assert!(pane.move_to(Point::new(190., 100.)).is_empty());
    assert!(pane.release().is_empty());

    assert!((pane.state.slider.value - 1.0).abs() < 0.001);
    assert!(!pane.state.slider.dragging);
    assert!((pane.state.changed.unwrap() - 1.0).abs() < 0.001);
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

    let mut pane = PaneBuilder::new("test", view).build(State::default());
    let (_, effects) = pane.redraw(300, 200, 1.0);
    assert!(effects.is_empty(), "unexpected effects: {effects:?}");

    assert!(pane.click(FIELD).expect("field present").is_empty());
    assert!(pane.state.text.editing);

    let (_, effects) = pane.redraw(300, 200, 1.0);
    assert!(effects.is_empty(), "unexpected effects: {effects:?}");

    assert!(pane.key_pressed("a").is_empty());

    assert_eq!(pane.state.text.text, "a");
    assert!(matches!(
        pane.state.edits.last(),
        Some(EditInteraction::Update(text)) if text == "a"
    ));
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

    let mut pane = PaneBuilder::new("test", view).build(State);
    let (_, effects) = pane.redraw(300, 200, 1.0);
    assert!(effects.is_empty(), "unexpected effects: {effects:?}");

    assert!(pane.elements.contains_key(&cell_id(0)));

    let location = pane.location(SCROLLER).expect("scroller present");
    assert!(pane.move_to(location).is_empty());
    assert!(
        pane.scroll(ScrollDelta { x: 0., y: -200. })
            .is_empty()
    );
    let (_, effects) = pane.redraw(300, 200, 1.0);
    assert!(effects.is_empty(), "unexpected effects: {effects:?}");

    assert!(
        !pane.elements.contains_key(&cell_id(0)),
        "cell 0 should have scrolled out of view"
    );
}
