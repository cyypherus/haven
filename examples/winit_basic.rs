use haven::*;
use haven::winit::WinitApp;

#[derive(Debug, Clone, Default)]
struct State {
    button: ButtonState,
}

fn main() {
    WinitApp::new(State::default())
        .root(
            Root::new("main", |state: &State, app: &mut RootState| {
                button(id!(), binding!(state, State, button))
                    .text_label("Hello from Haven")
                    .build(app.ctx())
                    .height(50.)
                    .width(200.)
            })
            .inner_size(400, 240),
        )
        .run();
}
