use haven::*;

#[derive(Default)]
struct State {
    button: ButtonState,
    slider: SliderState,
    clicked: bool,
    changed: Option<f32>,
}

const BUTTON: u64 = 1;
const SLIDER: u64 = 2;

fn button_view<'a>(state: &'a State, app: &mut PaneState) -> View<'a, State> {
    button(BUTTON, binding!(state.button))
        .text_label("Run")
        .on_click(|state, _| state.clicked = true)
        .build(app)
        .width(120.)
        .height(40.)
}

fn slider_view<'a>(state: &'a State, app: &mut PaneState) -> View<'a, State> {
    slider(SLIDER, binding!(state.slider))
        .on_change(|state, _, value| state.changed = Some(value))
        .build(app)
        .width(100.)
        .height(20.)
}

fn main() {
    let mut state = State::default();
    let mut button_pane = PaneBuilder::new("button", button_view).build();

    let (_frame, effects) = button_pane.redraw(&mut state, 300, 200, 1.0);
    assert!(effects.is_empty());

    let button_location = button_pane.location(BUTTON).expect("button exists");
    assert!(button_pane.click(&mut state, button_location).is_empty());
    assert!(state.clicked);

    let mut slider_pane = PaneBuilder::new("slider", slider_view).build();
    slider_pane.redraw(&mut state, 300, 200, 1.0);
    slider_pane.move_to(&mut state, Point::new(150., 100.));
    slider_pane.press_button(&mut state, MouseButton::Left);
    slider_pane.move_to(&mut state, Point::new(190., 100.));
    slider_pane.release_button(&mut state, MouseButton::Left);

    assert!((state.slider.value - 1.0).abs() < 0.001);
    assert!((state.changed.unwrap() - 1.0).abs() < 0.001);
}
