use haven::winit::WinitApp;
use haven::*;

#[derive(Debug, Clone, Default)]
struct State {
    main_open: ButtonState,
    main_pulse: ButtonState,
    tools_close: ButtonState,
    tools_pulse: ButtonState,
    main_count: u32,
    tools_count: u32,
}

fn main() {
    WinitApp::new(State::default())
        .pane(
            PaneConfig::new("main", |state: &State, app: &mut PaneState| {
                column_spaced(
                    16.,
                    vec![
                        text(id!(), "Main root")
                            .font_size(30)
                            .font_weight(FontWeight::BOLD)
                            .build(app),
                        text(id!(), format!("Main clicks: {}", state.main_count))
                            .font_size(18)
                            .build(app),
                        button(id!(), binding!(state, State, main_pulse))
                            .text_label("Increment main")
                            .on_click(|state, _| {
                                state.main_count += 1;
                            })
                            .build(app)
                            .height(44.)
                            .width(180.),
                        button(id!(), binding!(state, State, main_open))
                            .text_label("Open tools window")
                            .on_click(|_, app| {
                                app.open("tools");
                            })
                            .build(app)
                            .height(44.)
                            .width(180.),
                    ],
                )
                .pad(24.)
            })
            .title("Haven main")
            .inner_size(420, 300),
        )
        .pane(
            PaneConfig::new("tools", |state: &State, app: &mut PaneState| {
                column_spaced(
                    16.,
                    vec![
                        text(id!(), "Tools root")
                            .font_size(30)
                            .font_weight(FontWeight::BOLD)
                            .build(app),
                        text(id!(), format!("Tools clicks: {}", state.tools_count))
                            .font_size(18)
                            .build(app),
                        button(id!(), binding!(state, State, tools_pulse))
                            .text_label("Increment tools")
                            .on_click(|state, _| {
                                state.tools_count += 1;
                            })
                            .build(app)
                            .height(44.)
                            .width(180.),
                        button(id!(), binding!(state, State, tools_close))
                            .text_label("Close this window")
                            .on_click(|_, app| {
                                app.close();
                            })
                            .build(app)
                            .height(44.)
                            .width(180.),
                    ],
                )
                .pad(24.)
            })
            .title("Haven tools")
            .inner_size(420, 300)
            .open_at_start(false),
        )
        .run();
}
