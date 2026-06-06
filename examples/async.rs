use haven::winit::WinitApp;
use haven::*;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

type UiCallback = Box<dyn FnOnce(&mut State, &mut PaneState) + Send>;

struct State {
    tx: Sender<UiCallback>,
    rx: Receiver<UiCallback>,
    load_button: ButtonState,
    loading: bool,
    value: Option<String>,
}

impl State {
    fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            tx,
            rx,
            load_button: ButtonState::default(),
            loading: false,
            value: None,
        }
    }
}

fn on_wake(state: &mut State, app: &mut PaneState) {
    while let Ok(callback) = state.rx.try_recv() {
        callback(state, app);
    }
}

fn view<'a>(state: &'a State, app: &mut PaneState) -> View<'a, State> {
    column_spaced(
        16.,
        vec![
            text(id!(), "Async callback").font_size(28).build(app),
            text(
                id!(),
                if state.loading {
                    "Loading...".to_string()
                } else {
                    state
                        .value
                        .as_deref()
                        .unwrap_or("No value loaded")
                        .to_string()
                },
            )
            .font_size(16)
            .build(app),
            button(id!(), binding!(state.load_button))
                .text_label("Load value")
                .on_click(|state, app| {
                    if state.loading {
                        return;
                    }

                    state.loading = true;
                    app.redraw();

                    let tx = state.tx.clone();
                    let wake = app.waker();

                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_millis(800)).await;
                        let value = "Loaded from Tokio".to_string();

                        tx.send(Box::new(move |state: &mut State, app: &mut PaneState| {
                            state.loading = false;
                            state.value = Some(value);
                            app.redraw();
                        }))
                        .ok();

                        wake.wake();
                    });
                })
                .build(app)
                .width(160.)
                .height(44.),
        ],
    )
    .pad(24.)
}

#[tokio::main]
async fn main() {
    WinitApp::new(State::new())
        .pane(
            PaneBuilder::new("main", view)
                .on_wake(on_wake)
                .title("Async callbacks")
                .inner_size(420, 240),
        )
        .run();
}
