use haven::winit::WinitApp;
use haven::*;
use vello_svg::vello::peniko::color::AlphaColor;

#[derive(Clone, Default, Debug)]
struct State {
    count: i32,
    b1: ButtonState,
    b2: ButtonState,
    b3: ButtonState,
}

fn main() {
    WinitApp::new(State::default())
        .root(
            Root::new("main", |state: &State, app: &mut RootState| {
                row_spaced(
                    20.,
                    vec![
                        column_spaced(
                            20.,
                            vec![text(id!(), "Label only")
                                .fill(Color::WHITE)
                                .build(app.ctx())],
                        )
                        .height(150.)
                        .width(200.),
                        column_spaced(
                            20.,
                            vec![
                                text(id!(), "Text button")
                                    .fill(Color::WHITE)
                                    .build(app.ctx()),
                                button(id!(), binding!(state, State, b1))
                                    .text_label(format!("Click count {}", state.count))
                                    .on_click(|s, _| s.count += 1)
                                    .build(app.ctx()),
                            ],
                        )
                        .height(150.)
                        .width(200.),
                        column_spaced(
                            20.,
                            vec![
                                text(id!(), "Custom body")
                                    .fill(Color::WHITE)
                                    .build(app.ctx()),
                                button(id!(), binding!(state, State, b2))
                                    .on_click(|s, _| s.count += 1)
                                    .surface(|state, ctx| {
                                        rect(id!())
                                            .fill(match (state.hovered, state.depressed) {
                                                (_, true) => AlphaColor::from_rgb8(100, 30, 30),
                                                (true, false) => AlphaColor::from_rgb8(230, 30, 30),
                                                (false, false) => AlphaColor::from_rgb8(200, 30, 30),
                                            })
                                            .corner_rounding(40.)
                                            .build(ctx)
                                    })
                                    .build(app.ctx()),
                            ],
                        )
                        .height(150.)
                        .width(200.),
                        column_spaced(
                            20.,
                            vec![
                                text(id!(), "Svg label").fill(Color::WHITE).build(app.ctx()),
                                button(id!(), binding!(state, State, b3))
                                    .on_click(|s, _| s.count += 1)
                                    .label(|state, ctx| {
                                        svg(id!(), include_str!("../assets/download.svg"))
                                            .fill(match (state.depressed, state.hovered) {
                                                (true, _) => AlphaColor::from_rgb8(190, 190, 190),
                                                (false, true) => AlphaColor::from_rgb8(250, 250, 250),
                                                (false, false) => AlphaColor::from_rgb8(240, 240, 240),
                                            })
                                            .finish(ctx)
                                            .pad(15.)
                                    })
                                    .build(app.ctx()),
                            ],
                        )
                        .height(150.)
                        .width(200.),
                    ],
                )
                .pad(20.)
            })
            .title("Buttons")
            .inner_size(920, 240),
        )
        .run();
}
