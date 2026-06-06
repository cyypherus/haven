use haven::winit::WinitApp;
use haven::*;

#[derive(Debug, Clone)]
struct State {
    default_single: TextState,
    explicit_single: TextState,
    wrapped_single: TextState,
    multiline: TextState,
    wrapped_multiline: TextState,
    enter_done: TextState,
    escape_done: TextState,
    password: TextState,
    digits: TextState,
    styled: TextState,
    aligned_start: TextState,
    aligned_center: TextState,
    aligned_end: TextState,
    vertical_top: TextState,
    vertical_center: TextState,
    vertical_bottom: TextState,
    focus_target: TextState,
    focus_button: ButtonState,
    end_button: ButtonState,
    select_button: ButtonState,
}

impl Default for State {
    fn default() -> Self {
        Self {
            default_single: TextState::new("Default single-line"),
            explicit_single: TextState::new("Explicit single-line"),
            wrapped_single: TextState::new(
                "Single-line filtering is separate from layout wrapping. Press Enter here.",
            ),
            multiline: TextState::new("Unwrapped multiline field"),
            wrapped_multiline: TextState::new(
                "Wrapped multiline field. Press Enter or paste text with line breaks.",
            ),
            enter_done: TextState::new("Enter ends editing"),
            escape_done: TextState::new("Escape ends editing"),
            password: TextState::new(""),
            digits: TextState::new(""),
            styled: TextState::new("Styled text field"),
            aligned_start: TextState::new("Start aligned"),
            aligned_center: TextState::new("Center aligned"),
            aligned_end: TextState::new("End aligned"),
            vertical_top: TextState::new("Top"),
            vertical_center: TextState::new("Center"),
            vertical_bottom: TextState::new("Bottom"),
            focus_target: TextState::new("Programmatic focus target"),
            focus_button: ButtonState::default(),
            end_button: ButtonState::default(),
            select_button: ButtonState::default(),
        }
    }
}

fn main() {
    WinitApp::new(State::default())
        .pane(
            PaneBuilder::new("main", view)
                .title("Text Fields")
                .inner_size(960, 960),
        )
        .run();
}

fn view<'a>(state: &'a State, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
    stack(vec![
        rect(id!()).fill(background()).build(app).expand(),
        column_spaced(
            14.,
            vec![
                heading("Text fields", app),
                row_spaced(
                    14.,
                    vec![
                        field_panel(
                            "Default single-line",
                            state.default_single.editing,
                            text_field(id!(), binding!(state, State, default_single))
                                .hint_text("Enter one line")
                                .align(Alignment::Start)
                                .build(app)
                                .height(42.),
                            app,
                        ),
                        field_panel(
                            "Explicit single-line",
                            state.explicit_single.editing,
                            text_field(id!(), binding!(state, State, explicit_single))
                                .singleline()
                                .hint_text("Newlines are filtered")
                                .align(Alignment::Start)
                                .build(app)
                                .height(42.),
                            app,
                        ),
                        field_panel(
                            "Wrapped single-line",
                            state.wrapped_single.editing,
                            text_field(id!(), binding!(state, State, wrapped_single))
                                .singleline()
                                .wrap()
                                .align(Alignment::Start)
                                .build(app)
                                .height(76.),
                            app,
                        ),
                    ],
                ),
                row_spaced(
                    14.,
                    vec![
                        field_panel(
                            "Multiline",
                            state.multiline.editing,
                            text_field(id!(), binding!(state, State, multiline))
                                .multiline()
                                .hint_text("Enter preserves newlines")
                                .align(Alignment::Start)
                                .build(app)
                                .height(76.),
                            app,
                        ),
                        field_panel(
                            "Wrapped multiline",
                            state.wrapped_multiline.editing,
                            text_field(id!(), binding!(state, State, wrapped_multiline))
                                .multiline()
                                .wrap()
                                .align(Alignment::Start)
                                .build(app)
                                .height(96.),
                            app,
                        ),
                        field_panel(
                            "Custom styling",
                            state.styled.editing,
                            text_field(id!(), binding!(state, State, styled))
                                .text_fill(Color::from_rgb8(255, 245, 210))
                                .cursor_fill(Color::from_rgb8(255, 215, 90))
                                .highlight_fill(Color::from_rgb8(80, 130, 220).with_alpha(0.55))
                                .hint_text("Styled hint")
                                .hint_fill(Color::from_rgb8(170, 185, 210))
                                .font_size(18)
                                .font_weight(FontWeight::BOLD)
                                .align(Alignment::Start)
                                .padding(10.)
                                .background(|state, _, app| {
                                    rect(id!())
                                        .fill(if state.editing {
                                            Color::from_rgb8(42, 48, 62)
                                        } else {
                                            Color::from_rgb8(34, 38, 48)
                                        })
                                        .stroke(
                                            if state.editing {
                                                Color::from_rgb8(255, 215, 90)
                                            } else {
                                                border()
                                            },
                                            Stroke::new(1.),
                                        )
                                        .corner_rounding(6.)
                                        .build(app)
                                })
                                .build(app)
                                .height(58.),
                            app,
                        ),
                    ],
                ),
                row_spaced(
                    14.,
                    vec![
                        field_panel(
                            "Enter end editing",
                            state.enter_done.editing,
                            text_field(id!(), binding!(state, State, enter_done))
                                .enter_end_editing()
                                .align(Alignment::Start)
                                .build(app)
                                .height(42.),
                            app,
                        ),
                        field_panel(
                            "Escape end editing",
                            state.escape_done.editing,
                            text_field(id!(), binding!(state, State, escape_done))
                                .esc_end_editing()
                                .align(Alignment::Start)
                                .build(app)
                                .height(42.),
                            app,
                        ),
                        field_panel(
                            "Redacted",
                            state.password.editing,
                            text_field(id!(), binding!(state, State, password))
                                .hint_text("Type a secret")
                                .align(Alignment::Start)
                                .display(|state| "*".repeat(state.text.chars().count()))
                                .build(app)
                                .height(42.),
                            app,
                        ),
                    ],
                ),
                row_spaced(
                    14.,
                    vec![
                        field_panel(
                            "Filtered digits",
                            state.digits.editing,
                            text_field(id!(), binding!(state, State, digits))
                                .hint_text("Only digits remain")
                                .align(Alignment::Start)
                                .on_edit(|state, _, edit| {
                                    if let EditInteraction::Update(text) = edit {
                                        state.digits = TextState::new(
                                            text.chars()
                                                .filter(|character| character.is_ascii_digit())
                                                .collect::<String>(),
                                        );
                                    }
                                })
                                .build(app)
                                .height(42.),
                            app,
                        ),
                        field_panel(
                            "Programmatic focus",
                            state.focus_target.editing,
                            column_spaced(
                                10.,
                                vec![
                                    text_field(id!(), binding!(state, State, focus_target))
                                        .align(Alignment::Start)
                                        .build(app)
                                        .height(42.),
                                    row_spaced(
                                        10.,
                                        vec![
                                            button(id!(), binding!(state, State, focus_button))
                                                .text_label("Focus")
                                                .on_click(|state, app| {
                                                    state.focus_target.begin_editing(app);
                                                })
                                                .build(app)
                                                .height(34.)
                                                .width(68.),
                                            button(id!(), binding!(state, State, end_button))
                                                .text_label("End")
                                                .on_click(|state, app| {
                                                    state.focus_target.end_editing(app);
                                                })
                                                .build(app)
                                                .height(34.)
                                                .width(58.),
                                            button(id!(), binding!(state, State, select_button))
                                                .text_label("Select all")
                                                .on_click(|state, app| {
                                                    state.focus_target.begin_editing_with(
                                                        app,
                                                        InitialSelection::All,
                                                    );
                                                })
                                                .build(app)
                                                .height(34.)
                                                .width(94.),
                                        ],
                                    )
                                    .height(34.),
                                ],
                            ),
                            app,
                        ),
                        field_panel(
                            "Horizontal align",
                            state.aligned_start.editing
                                || state.aligned_center.editing
                                || state.aligned_end.editing,
                            column_spaced(
                                8.,
                                vec![
                                    text_field(id!(), binding!(state, State, aligned_start))
                                        .align(Alignment::Start)
                                        .build(app)
                                        .height(30.),
                                    text_field(id!(), binding!(state, State, aligned_center))
                                        .align(Alignment::Center)
                                        .build(app)
                                        .height(30.),
                                    text_field(id!(), binding!(state, State, aligned_end))
                                        .align(Alignment::End)
                                        .build(app)
                                        .height(30.),
                                ],
                            ),
                            app,
                        ),
                    ],
                ),
                row_spaced(
                    14.,
                    vec![
                        field_panel(
                            "Vertical align",
                            state.vertical_top.editing
                                || state.vertical_center.editing
                                || state.vertical_bottom.editing,
                            row_spaced(
                                8.,
                                vec![
                                    text_field(id!(), binding!(state, State, vertical_top))
                                        .align(Alignment::Center)
                                        .vertical_align(TextFieldVerticalAlignment::Top)
                                        .build(app)
                                        .height(92.),
                                    text_field(id!(), binding!(state, State, vertical_center))
                                        .align(Alignment::Center)
                                        .vertical_align(TextFieldVerticalAlignment::Center)
                                        .build(app)
                                        .height(92.),
                                    text_field(id!(), binding!(state, State, vertical_bottom))
                                        .align(Alignment::Center)
                                        .vertical_align(TextFieldVerticalAlignment::Bottom)
                                        .build(app)
                                        .height(92.),
                                ],
                            ),
                            app,
                        ),
                        empty(),
                        empty(),
                    ],
                ),
            ],
        )
        .pad(24.)
        .align(Align::Top),
    ])
}

fn heading<'a>(value: &'static str, app: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
    text(id!(), value)
        .font_size(30)
        .font_weight(FontWeight::BOLD)
        .align(Alignment::Start)
        .build(app)
        .height(40.)
}

fn field_panel<'a>(
    label: &'static str,
    editing: bool,
    field: Layout<'a, View<State>, PaneState>,
    app: &mut PaneState,
) -> Layout<'a, View<State>, PaneState> {
    stack(vec![
        rect(id!())
            .fill(panel_fill(editing))
            .stroke(
                if editing { active_border() } else { border() },
                Stroke::new(1.),
            )
            .corner_rounding(8.)
            .build(app),
        column_spaced(
            8.,
            vec![
                row_spaced(
                    8.,
                    vec![
                        text(id!(), label)
                            .font_size(13)
                            .font_weight(FontWeight::BOLD)
                            .fill(label_color())
                            .align(Alignment::Start)
                            .build(app),
                        text(id!(), if editing { "editing" } else { "idle" })
                            .font_size(12)
                            .fill(if editing { active_border() } else { quiet() })
                            .align(Alignment::End)
                            .build(app)
                            .width(70.),
                    ],
                )
                .height(18.),
                field,
            ],
        )
        .pad(12.),
    ])
    .height(150.)
}

fn background() -> Color {
    Color::from_rgb8(18, 20, 24)
}

fn panel_fill(editing: bool) -> Color {
    if editing {
        Color::from_rgb8(32, 39, 51)
    } else {
        Color::from_rgb8(27, 30, 36)
    }
}

fn border() -> Color {
    Color::from_rgb8(65, 72, 84)
}

fn active_border() -> Color {
    Color::from_rgb8(112, 164, 240)
}

fn label_color() -> Color {
    Color::from_rgb8(232, 236, 242)
}

fn quiet() -> Color {
    Color::from_rgb8(146, 154, 168)
}
