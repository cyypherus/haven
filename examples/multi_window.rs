use haven::winit::WinitApp;
use haven::*;

#[derive(Clone, Default, Debug)]
struct State {
    count: i32,
    main_increment: ButtonState,
    open_detail: ButtonState,
    detail_increment: ButtonState,
    close_detail: ButtonState,
}

fn main() {
    WinitApp::new(State::default())
        .pane(
            PaneBuilder::new("main", main_view)
                .title("Counter")
                .inner_size(360, 220)
                .initial_position(120, 120),
        )
        .pane(
            PaneBuilder::new("detail", detail_view)
                .title("Counter Detail")
                .inner_size(360, 220)
                .initial_position(520, 160)
                .open_at_start(false),
        )
        .run();
}

fn main_view<'a>(state: &'a State, app: &mut PaneState) -> View<'a, State> {
    stack(vec![
        rect(id!())
            .fill(Color::from_rgb8(24, 26, 28))
            .build(app)
            .expand(),
        column_spaced(
            14.,
            vec![
                text(id!(), "Main window")
                    .fill(Color::WHITE)
                    .build(app)
                    .height(32.),
                text(id!(), format!("Shared count: {}", state.count))
                    .fill(Color::from_rgb8(220, 220, 220))
                    .build(app)
                    .height(28.),
                button(id!(), binding!(state.main_increment))
                    .text_label("Increment")
                    .on_click(|state, _| state.count += 1)
                    .build(app)
                    .height(42.),
                button(id!(), binding!(state.open_detail))
                    .text_label("Open detail")
                    .on_click(|_, app| app.open("detail"))
                    .build(app)
                    .height(42.),
            ],
        )
        .pad(20.),
    ])
}

fn detail_view<'a>(state: &'a State, app: &mut PaneState) -> View<'a, State> {
    stack(vec![
        rect(id!())
            .fill(Color::from_rgb8(28, 24, 32))
            .build(app)
            .expand(),
        column_spaced(
            14.,
            vec![
                text(id!(), "Detail window")
                    .fill(Color::WHITE)
                    .build(app)
                    .height(32.),
                text(id!(), format!("Shared count: {}", state.count))
                    .fill(Color::from_rgb8(220, 220, 220))
                    .build(app)
                    .height(28.),
                button(id!(), binding!(state.detail_increment))
                    .text_label("Increment")
                    .on_click(|state, _| state.count += 1)
                    .build(app)
                    .height(42.),
                button(id!(), binding!(state.close_detail))
                    .text_label("Close")
                    .on_click(|_, app| app.close())
                    .build(app)
                    .height(42.),
            ],
        )
        .pad(20.),
    ])
}
