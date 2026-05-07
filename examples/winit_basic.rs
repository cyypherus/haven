use haven::winit::WinitApp;
use haven::*;

#[derive(Debug, Clone, Default)]
struct State {
    button: ButtonState,
}

fn main() {
    WinitApp::new(State::default())
        .pane(
            PaneConfig::new("main", |state: &State, app: &mut PaneState| {
                button(id!(), binding!(state, State, button))
                    .text_label("Hello from Haven")
                    .build(app)
                    .height(50.)
                    .width(200.)
            })
            .inner_size(400, 240),
        )
        .run();
}
