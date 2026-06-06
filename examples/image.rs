use haven::winit::WinitApp;
use haven::*;
use parley::FontWeight;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

#[derive(Clone)]
enum DownloadState {
    Idle,
    Downloading,
    Success(String, Arc<Vec<u8>>),
    Error(String),
}

type UiCallback = Box<dyn FnOnce(&mut State, &mut PaneState) + Send>;

struct State {
    tx: Sender<UiCallback>,
    rx: Receiver<UiCallback>,
    input: TextState,
    load_button: ButtonState,
    download_state: DownloadState,
}

impl State {
    fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            tx,
            rx,
            input: TextState::new(""),
            load_button: ButtonState::default(),
            download_state: DownloadState::Idle,
        }
    }

    fn load_image(&mut self, app: &mut PaneState) {
        let input = self.input.text.trim().to_string();
        if input.is_empty() {
            return;
        }

        self.download_state = DownloadState::Downloading;
        app.redraw();

        let tx = self.tx.clone();
        let wake = app.waker();

        if input.starts_with("http://") || input.starts_with("https://") {
            tokio::spawn(async move {
                let result = reqwest::get(&input).await;

                let download_state = match result {
                    Ok(response) => {
                        if response.status().is_success() {
                            match response.bytes().await {
                                Ok(bytes) => decoded_image_state(input, bytes.into()),
                                Err(e) => {
                                    DownloadState::Error(format!("Failed to read response: {e}"))
                                }
                            }
                        } else {
                            DownloadState::Error(format!(
                                "HTTP {}: {}",
                                response.status().as_u16(),
                                response
                                    .status()
                                    .canonical_reason()
                                    .unwrap_or("Unknown error")
                            ))
                        }
                    }
                    Err(e) => DownloadState::Error(format!("Request failed: {e}")),
                };

                tx.send(Box::new(move |state: &mut State, app: &mut PaneState| {
                    state.download_state = download_state;
                    app.redraw();
                }))
                .ok();

                wake.wake();
            });
        } else {
            tokio::spawn(async move {
                let download_state = match tokio::fs::read(&input).await {
                    Ok(bytes) => decoded_image_state(input, bytes),
                    Err(e) => DownloadState::Error(format!("Failed to read file: {e}")),
                };

                tx.send(Box::new(move |state: &mut State, app: &mut PaneState| {
                    state.download_state = download_state;
                    app.redraw();
                }))
                .ok();

                wake.wake();
            });
        }
    }
}

fn decoded_image_state(input: String, bytes: Vec<u8>) -> DownloadState {
    match image::load_from_memory(&bytes) {
        Ok(_) => DownloadState::Success(input, Arc::new(bytes)),
        Err(e) => DownloadState::Error(format!("Loading image failed: {e}")),
    }
}

fn on_wake(state: &mut State, app: &mut PaneState) {
    while let Ok(callback) = state.rx.try_recv() {
        callback(state, app);
    }
}

fn view<'a>(state: &'a State, app: &mut PaneState) -> View<'a, State> {
    let download_state = state.download_state.clone();
    let download_state_for_button = download_state.clone();

    column_spaced(
        20.,
        vec![
            text(id!(), "Image Loader")
                .font_size(32)
                .font_weight(FontWeight::BOLD)
                .build(app)
                .pad(10.),
            text_field(id!(), binding!(state.input))
                .hint_text("URL or file path")
                .singleline()
                .align(Alignment::Start)
                .build(app)
                .height(40.)
                .width_range(360.0..),
            button(id!(), binding!(state.load_button))
                .label(move |_state, ctx| {
                    text(
                        id!(),
                        match download_state_for_button {
                            DownloadState::Downloading => "Loading...",
                            _ => "Load Image",
                        },
                    )
                    .build(ctx)
                })
                .on_click(|s, app| {
                    if matches!(s.download_state, DownloadState::Downloading) {
                        return;
                    }
                    s.load_image(app);
                })
                .build(app)
                .height(50.)
                .width(200.),
            match download_state {
                DownloadState::Idle => text(id!(), "Paste a URL or file path, then click Load")
                    .font_size(14)
                    .build(app),
                DownloadState::Downloading => {
                    text(id!(), "Loading image...").font_size(14).build(app)
                }
                DownloadState::Success(ref image_id, ref bytes) => column_spaced(
                    10.,
                    vec![
                        text(id!(), format!("Loaded {} bytes", bytes.len()))
                            .font_size(14)
                            .build(app),
                        image_from_bytes(id!(), bytes.clone())
                            .image_id(image_id)
                            .view()
                            .build(app)
                            .height_range(100.0..)
                            .width_range(100.0..),
                    ],
                ),
                DownloadState::Error(ref error) => text(id!(), format!("Error: {error}"))
                    .font_size(14)
                    .fill(Color::from_rgb8(255, 0, 0))
                    .build(app),
            },
        ],
    )
    .pad(20.)
    .pad_top(20.)
}

#[tokio::main]
async fn main() {
    WinitApp::new(State::new())
        .pane(
            PaneBuilder::new("main", view)
                .on_wake(on_wake)
                .title("Image Loader")
                .inner_size(720, 720),
        )
        .run();
}
