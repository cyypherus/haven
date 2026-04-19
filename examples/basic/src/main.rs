use haven::*;

#[derive(Debug, Clone, Default)]
struct State {
    text: TextState,
    toggle: ToggleState,
    slider: SliderState,
    button: ButtonState,
    style_dropdown: DropdownState<Biome>,
}

fn main() {
    App::builder(
        State {
            text: TextState {
                text: "Bio-luminescenct moss carpets power floating gardens while crystal-infused mycelium networks whisper data through the canopy above. With reverent whispers the fauna lift their gaze to the shafts of light piercing the deep green"
                    .to_string(),
                editing: false,
            },
            toggle: ToggleState::default(),
            slider: SliderState::default(),
            button: ButtonState::default(),
            style_dropdown: DropdownState::default(),
        },
        Window::new("main", |state: &State, app: &mut AppState| {
                column_spaced(
                    10.,
                    vec![
                        space()
                            .height(30.),
                        text(
                            id!(),
                            "Mycelial Networks Harmonize with Quantum-Grown Algae Towers",
                        )
                        .font_weight(FontWeight::BOLD)
                        .font_size(30)
                        .wrap()
                        .build(app.ctx()),
                        rich_text(
                            id!(),
                            [
                                span("Harvest").bold(),
                                " yields are up ".into(),
                                span("12%").bold().color(Color::from_rgb8(120, 230, 140)),
                                " across all ".into(),
                                span("biomes").background(Color::from_rgb8(60, 40, 80)),
                                " - ".into(),
                                span("see notes").italic().underline().color(DEFAULT_PURP),
                                ".".into(),
                            ],
                        )
                        .wrap()
                        .align(parley::Alignment::Start)
                        .build(app.ctx()),
                        scope!(state, State, { style_dropdown, text } => DDTextState,
                            |sub_state| row_spaced(10., dropdown_and_text(sub_state, app))
                        ),
                        stack(vec![
                            rect(id!()).fill(DEFAULT_DARK_GRAY).corner_rounding(8.).build(app.ctx()),
                            draw(|area, ctx: &mut AppCtx| {
                                path(id!(), |area| chart_fill(area, CHART_DATA))
                                    .fill(
                                        Gradient::new_linear(
                                            (0., area.y as f64),
                                            (0., area.y as f64 + area.height as f64),
                                        )
                                        .with_stops([DEFAULT_PURP.with_alpha(0.4), DEFAULT_PURP.with_alpha(0.0)])
                                    )
                                    .build(ctx)
                                    .draw(area, ctx)
                            }),
                            path(id!(), |area| chart_line(area, CHART_DATA))
                                .stroke(DEFAULT_PURP, Stroke::new(2.0).with_caps(Cap::Round).with_join(Join::Round))
                                .build(app.ctx()),
                        ])
                        .height(120.),
                        row_spaced(
                            10.,
                            vec![
                                toggle(id!(), binding!(state, State, toggle)).build(app.ctx()).height(25.).width(50.),
                                slider(id!(), binding!(state, State, slider)).build(app.ctx()).height(25.),
                            ]
                        ),
                        button(id!(), binding!(state, State, button))
                            .text_label("Engage thrusters")
                            .on_click(|_state, app| {
                                app.open_window("thrusters");
                            })
                            .surface(|_state, ctx| {
                                rect(id!())
                                    .fill(
                                        Gradient::new_linear((0., 0.), (200., 0.))
                                            .with_stops([DEFAULT_PURP, Color::from_rgb8(200, 50, 180)])
                                    )
                                    .corner_rounding(DEFAULT_CORNER_ROUNDING)
                                    .build(ctx)
                            })
                            .build(app.ctx()).height(30.),
                    ],
                )
                .pad(20.)
                .align(Align::Top)
        }).inner_size(800, 600),
    )
    .window(
        Window::new("thrusters", thrusters_view)
            .open_at_start(false)
            .title("Thrusters")
            .inner_size(400, 300)
            .transparent(true),
    )
    .start()
}

fn thrusters_view<'a>(_state: &'a State, app: &mut AppState) -> Layout<'a, View<State>, AppCtx> {
    stack(vec![
        rect(id!())
            .fill(Color::from_rgb8(30, 30, 40).with_alpha(0.75))
            .corner_rounding(16.)
            .stroke(Color::from_rgb8(120, 120, 160).with_alpha(0.5), Stroke::new(1.))
            .build(app.ctx()),
        column_spaced(
            10.,
            vec![
                text(id!(), "Thrusters Engaged")
                    .font_weight(FontWeight::BOLD)
                    .font_size(24)
                    .build(app.ctx()),
                text(id!(), "All systems nominal. Quantum drive is spooling up.")
                    .wrap()
                    .build(app.ctx()),
            ],
        )
        .pad(20.),
    ])
    .pad(20.)
}

#[derive(Clone)]
struct DDTextState {
    style_dropdown: DropdownState<Biome>,
    text: TextState,
}

fn dropdown_and_text(
    state: &DDTextState,
    app: &mut AppState,
) -> Vec<Layout<'static, View<DDTextState>, AppCtx>> {
    vec![
        dropdown(
            id!(),
            binding!(state, DDTextState, style_dropdown),
            Biome::ALL.to_vec(),
            |item, ctx| text(id!(), item.value.label()).build(ctx),
        )
        .build(app.ctx())
        .width(140.)
        .align(Align::Top),
        {
            let tf = text_field(id!(), binding!(state, DDTextState, text)).wrap();
            match state.style_dropdown.selected {
                Biome::LuminescentMoss | Biome::FloatingGardens => tf
                    .background(|ts, area, ctx| {
                        let mut r = rect(id!())
                            .fill(
                                Gradient::new_linear(
                                    (area.x as f64, area.y as f64),
                                    (area.x as f64 + area.width as f64, area.y as f64),
                                )
                                .with_stops([
                                    Color::from_rgb8(10, 30, 60),
                                    Color::from_rgb8(20, 80, 120),
                                ]),
                            )
                            .corner_rounding(12.);
                        r = if ts.editing {
                            r.stroke(Color::from_rgb8(0, 0, 255), Stroke::new(4.))
                        } else {
                            r.stroke(Color::from_rgb8(0, 0, 100), Stroke::new(4.))
                        };
                        r.build(ctx)
                    })
                    .text_fill(Color::from_rgb8(140, 210, 255))
                    .cursor_fill(Color::from_rgb8(100, 180, 255))
                    .highlight_fill(|area: Area, _: &TextState| {
                        Gradient::new_linear(
                            (area.x as f64, area.y as f64),
                            (area.x as f64 + area.width as f64, area.y as f64),
                        )
                        .with_stops([
                            Color::from_rgb8(30, 90, 140),
                            Color::from_rgb8(50, 120, 160),
                        ])
                        .into()
                    }),
                Biome::CrystalMycelium | Biome::CerebralForests => tf
                    .background(|_ts, area, ctx| {
                        rect(id!())
                            .fill(
                                Gradient::new_linear(
                                    (area.x as f64, area.y as f64),
                                    (
                                        area.x as f64 + area.width as f64,
                                        area.y as f64 + area.height as f64,
                                    ),
                                )
                                .with_stops([
                                    Color::from_rgb8(80, 20, 10),
                                    Color::from_rgb8(140, 80, 10),
                                ]),
                            )
                            .corner_rounding(2.)
                            .build(ctx)
                    })
                    .text_fill(Color::from_rgb8(255, 200, 120))
                    .cursor_fill(Color::from_rgb8(255, 160, 60))
                    .highlight_fill(Color::from_rgb8(160, 80, 20)),
                Biome::QuantumAlgae | Biome::GlassMarrow => tf
                    .background(|_ts, area, ctx| {
                        rect(id!())
                            .fill(
                                Gradient::new_linear(
                                    (area.x as f64, area.y as f64),
                                    (area.x as f64, area.y as f64 + area.height as f64),
                                )
                                .with_stops([
                                    Color::from_rgb8(25, 5, 50),
                                    Color::from_rgb8(5, 15, 35),
                                ]),
                            )
                            .corner_rounding(16.)
                            .build(ctx)
                    })
                    .padding(12.)
                    .text_fill(|area: Area, _: &TextState| {
                        Gradient::new_linear(
                            (area.x as f64, 0.),
                            (area.x as f64 + area.width as f64, 0.),
                        )
                        .with_stops([
                            Color::from_rgb8(255, 50, 200),
                            Color::from_rgb8(50, 200, 255),
                        ])
                        .into()
                    })
                    .cursor_fill(|area: Area, _: &TextState| {
                        Gradient::new_linear(
                            (0., area.y as f64),
                            (0., area.y as f64 + area.height as f64),
                        )
                        .with_stops([
                            Color::from_rgb8(255, 50, 200),
                            Color::from_rgb8(50, 200, 255),
                        ])
                        .into()
                    })
                    .highlight_fill(|area: Area, _: &TextState| {
                        Gradient::new_linear(
                            (area.x as f64, 0.),
                            (area.x as f64 + area.width as f64, 0.),
                        )
                        .with_stops([
                            Color::from_rgb8(80, 0, 120).with_alpha(0.5),
                            Color::from_rgb8(0, 60, 120).with_alpha(0.5),
                        ])
                        .into()
                    }),
            }
            .build(app.ctx())
            .align(Align::Top)
        },
    ]
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
enum Biome {
    #[default]
    LuminescentMoss,
    CrystalMycelium,
    QuantumAlgae,
    FloatingGardens,
    CerebralForests,
    GlassMarrow,
}

impl Biome {
    const ALL: &[Biome] = &[
        Biome::LuminescentMoss,
        Biome::CrystalMycelium,
        Biome::QuantumAlgae,
        Biome::FloatingGardens,
        Biome::CerebralForests,
        Biome::GlassMarrow,
    ];

    fn label(&self) -> &'static str {
        match self {
            Biome::LuminescentMoss => "Luminescent Moss",
            Biome::CrystalMycelium => "Crystal Mycelium",
            Biome::QuantumAlgae => "Quantum Algae",
            Biome::FloatingGardens => "Floating Gardens",
            Biome::CerebralForests => "Cerebral Forests",
            Biome::GlassMarrow => "Glass Marrow",
        }
    }
}

const CHART_DATA: &[f64] = &[0.1, 0.4, 0.25, 0.8, 0.6, 0.5, 0.9, 0.7, 0.75];

fn chart_line(area: Area, points: &[f64]) -> BezPath {
    let w = area.width as f64;
    let h = area.height as f64;
    let x0 = area.x as f64;
    let y0 = area.y as f64;
    let step = w / (points.len() - 1) as f64;

    let px = |i: usize| x0 + step * i as f64;
    let py = |v: f64| y0 + h * (1.0 - v);

    let mut line = BezPath::new();
    line.move_to((px(0), py(points[0])));
    for i in 0..points.len() - 1 {
        let mid_x = (px(i) + px(i + 1)) / 2.0;
        line.curve_to(
            (mid_x, py(points[i])),
            (mid_x, py(points[i + 1])),
            (px(i + 1), py(points[i + 1])),
        );
    }
    line
}

fn chart_fill(area: Area, points: &[f64]) -> BezPath {
    let mut p = chart_line(area, points);
    let x0 = area.x as f64;
    let y0 = area.y as f64;
    p.line_to((x0 + area.width as f64, y0 + area.height as f64));
    p.line_to((x0, y0 + area.height as f64));
    p.close_path();
    p
}
