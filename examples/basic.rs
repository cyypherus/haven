use haven::winit::WinitApp;
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
    WinitApp::new(State::default())
        .pane(
            PaneConfig::new("main", |state: &State, app: &mut PaneState| {
                column_spaced(
                    10.,
                    vec![
                        space().height(30.),
                        text(
                            id!(),
                            "Mycelial Networks Harmonize with Quantum-Grown Algae Towers",
                        )
                        .font_weight(FontWeight::BOLD)
                        .font_size(30)
                        .wrap()
                        .build(app),
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
                        .build(app),
                        empty(),
                        stack(vec![
                            rect(id!())
                                .fill(DEFAULT_DARK_GRAY)
                                .corner_rounding(8.)
                                .build(app),
                            draw(|area, ctx: &mut PaneState| {
                                path(id!(), |area| chart_fill(area, CHART_DATA))
                                    .fill(
                                        Gradient::new_linear(
                                            (0., area.y as f64),
                                            (0., area.y as f64 + area.height as f64),
                                        )
                                        .with_stops([
                                            DEFAULT_PURP.with_alpha(0.4),
                                            DEFAULT_PURP.with_alpha(0.0),
                                        ]),
                                    )
                                    .build(ctx)
                                    .draw(area, ctx)
                            }),
                            path(id!(), |area| chart_line(area, CHART_DATA))
                                .stroke(
                                    DEFAULT_PURP,
                                    Stroke::new(2.0)
                                        .with_caps(Cap::Round)
                                        .with_join(Join::Round),
                                )
                                .build(app),
                        ])
                        .height(120.),
                        row_spaced(
                            10.,
                            vec![
                                toggle(id!(), binding!(state, State, toggle))
                                    .build(app)
                                    .height(25.)
                                    .width(50.),
                                slider(id!(), binding!(state, State, slider))
                                    .build(app)
                                    .height(25.),
                            ],
                        ),
                        button(id!(), binding!(state, State, button))
                            .text_label("Engage thrusters")
                            .on_click(|_state, app| {
                                app.open("thrusters");
                            })
                            .surface(|_state, ctx| {
                                rect(id!())
                                    .fill(
                                        Gradient::new_linear((0., 0.), (200., 0.)).with_stops([
                                            DEFAULT_PURP,
                                            Color::from_rgb8(200, 50, 180),
                                        ]),
                                    )
                                    .corner_rounding(DEFAULT_CORNER_ROUNDING)
                                    .build(ctx)
                            })
                            .build(app)
                            .height(30.),
                    ],
                )
                .pad(20.)
                .align(Align::Top)
            })
            .inner_size(800, 600),
        )
        .pane(
            PaneConfig::new("thrusters", thrusters_view)
                .open_at_start(false)
                .title("Thrusters")
                .inner_size(400, 300)
                .transparent(true),
        )
        .run();
}

fn thrusters_view<'a>(
    _state: &'a State,
    app: &mut PaneState,
) -> Layout<'a, View<State>, PaneState> {
    stack(vec![
        rect(id!())
            .fill(Color::from_rgb8(30, 30, 40).with_alpha(0.75))
            .corner_rounding(16.)
            .stroke(
                Color::from_rgb8(120, 120, 160).with_alpha(0.5),
                Stroke::new(1.),
            )
            .build(app),
        column_spaced(
            10.,
            vec![
                text(id!(), "Thrusters Engaged")
                    .font_weight(FontWeight::BOLD)
                    .font_size(24)
                    .build(app),
                text(id!(), "All systems nominal. Quantum drive is spooling up.")
                    .wrap()
                    .build(app),
            ],
        )
        .pad(20.),
    ])
    .pad(20.)
}

fn dropdown_and_text(
    _state: Binding<State, DDTextState>,
    _app: &mut PaneState,
) -> Vec<Layout<'static, View<DDTextState>, PaneState>> {
    vec![empty()]
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum Biome {
    #[default]
    Canopy,
    Mycelial,
    Algae,
    Crystal,
    Lagoon,
}

impl Biome {
    fn label(&self) -> &'static str {
        match self {
            Biome::Canopy => "Canopy",
            Biome::Mycelial => "Mycelial",
            Biome::Algae => "Algae",
            Biome::Crystal => "Crystal",
            Biome::Lagoon => "Lagoon",
        }
    }
}

#[derive(Debug, Clone, Default)]
struct DDTextState {
    dropdown: DropdownState<Biome>,
    text: TextState,
}

const CHART_DATA: [f64; 16] = [
    0.18, 0.22, 0.31, 0.28, 0.46, 0.41, 0.58, 0.52, 0.63, 0.71, 0.68, 0.79, 0.73, 0.88, 0.84, 0.93,
];

fn chart_fill(area: Area, data: [f64; 16]) -> BezPath {
    let mut path = chart_line(area, data);
    path.line_to(Point::new(
        area.x as f64 + area.width as f64,
        area.y as f64 + area.height as f64,
    ));
    path.line_to(Point::new(
        area.x as f64,
        area.y as f64 + area.height as f64,
    ));
    path.close_path();
    path
}

fn chart_line(area: Area, data: [f64; 16]) -> BezPath {
    let mut path = BezPath::new();
    let max = data
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max)
        .max(1.0);
    let min = data.iter().copied().fold(f64::INFINITY, f64::min).min(0.0);
    let span = (max - min).max(1e-6);

    for (idx, value) in data.iter().enumerate() {
        let x = area.x as f64 + (idx as f64 / (data.len() - 1) as f64) * area.width as f64;
        let normalized = (*value - min) / span;
        let y = area.y as f64 + (1.0 - normalized) * area.height as f64;
        if idx == 0 {
            path.move_to(Point::new(x, y));
        } else {
            path.line_to(Point::new(x, y));
        }
    }

    path
}
