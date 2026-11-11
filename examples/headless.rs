use haven::headless::{HeadlessApp, Transport, VelloCpuImageRenderer};
use haven::*;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

type UiCallback = Box<dyn FnOnce(&mut State) + Send>;

struct State {
    tx: Sender<UiCallback>,
    rx: Receiver<UiCallback>,
    count: i32,
    loaded: Option<String>,
    button: ButtonState,
    name: TextState,
}

fn on_start(state: &mut State, app: &mut PaneState) {
    let tx = state.tx.clone();
    let wake = app.waker();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(300)).await;
        let _ = tx.send(Box::new(|state: &mut State| {
            state.loaded = Some("Loaded".into())
        }));
        wake.wake();
    });
}

fn on_wake(state: &mut State, app: &mut PaneState) {
    while let Ok(callback) = state.rx.try_recv() {
        callback(state);
    }
    app.redraw();
}

fn view<'a>(state: &'a State, app: &mut PaneState) -> View<'a, State> {
    column_spaced(
        16.,
        vec![
            text(id!(), format!("Count: {}", state.count))
                .fill(Color::WHITE)
                .font_size(24)
                .build(app),
            text(id!(), state.loaded.as_deref().unwrap_or("Loading…"))
                .fill(Color::WHITE)
                .build(app),
            button(id!(), binding!(state.button))
                .text_label("Increment")
                .on_click(|state, _| state.count += 1)
                .build(app)
                .width(160.)
                .height(44.),
            text_field(id!(), binding!(state.name))
                .hint_text("Name")
                .align(Alignment::Start)
                .build(app)
                .width(240.)
                .height(36.),
        ],
    )
    .pad(24.)
}

#[tokio::main]
async fn main() {
    let (tx, rx) = channel();
    HeadlessApp::new(State {
        tx,
        rx,
        count: 0,
        loaded: None,
        button: ButtonState::default(),
        name: TextState::new(""),
    })
    .pane(
        PaneBuilder::new("main", view)
            .on_start(on_start)
            .on_wake(on_wake)
            .inner_size(420, 320),
    )
    .run::<VelloCpuImageRenderer>(Transport::from_env().unwrap_or(Transport::Stdio));
}
