use haven::winit::WinitApp;
use haven::*;

#[derive(Debug, Clone)]
struct State {
    tasks: Vec<Task>,
    panel: Option<PanelState>,
    selected_task: Option<usize>,
    row_buttons: Vec<ButtonState>,
    compose_button: ButtonState,
    detail_edit_button: ButtonState,
    panel_cancel_button: ButtonState,
    panel_save_button: ButtonState,
}

impl Default for State {
    fn default() -> Self {
        Self {
            tasks: vec![
                Task::new(
                    "Tune the aquaponic moon-pump",
                    Project::Canopy,
                    Priority::High,
                ),
                Task::new(
                    "Map shade patterns for rooftop citrus",
                    Project::Garden,
                    Priority::Medium,
                ),
                Task::new(
                    "Host balcony compost clinic",
                    Project::People,
                    Priority::Low,
                ),
                Task::new(
                    "Patch the neighborhood battery mural",
                    Project::Commons,
                    Priority::Medium,
                ),
                Task::new(
                    "Invite elders to the seed-swap brunch",
                    Project::People,
                    Priority::Low,
                ),
                Task::new(
                    "Prototype algae lantern wayfinding",
                    Project::Commons,
                    Priority::High,
                ),
                Task::new(
                    "Train bees away from the tram sensors",
                    Project::Garden,
                    Priority::Medium,
                ),
                Task::new(
                    "Print bamboo repair clips for rain barrels",
                    Project::Canopy,
                    Priority::Low,
                ),
                Task::new(
                    "Record oral histories for the orchard walk",
                    Project::People,
                    Priority::Medium,
                ),
                Task::new(
                    "Audit mycelium filters under block C",
                    Project::Canopy,
                    Priority::Medium,
                ),
                Task::new(
                    "Tune wind chimes for storm warnings",
                    Project::Commons,
                    Priority::Low,
                ),
                Task::new(
                    "Graft pear cuttings beside the library",
                    Project::Garden,
                    Priority::High,
                ),
                Task::new(
                    "Repaint the solar oven plaza",
                    Project::Commons,
                    Priority::Medium,
                ),
            ],
            panel: None,
            selected_task: None,
            row_buttons: vec![ButtonState::default(); 13],
            compose_button: ButtonState::default(),
            detail_edit_button: ButtonState::default(),
            panel_cancel_button: ButtonState::default(),
            panel_save_button: ButtonState::default(),
        }
    }
}

#[derive(Debug, Clone)]
struct PanelState {
    mode: PanelMode,
    draft: TextState,
    project: DropdownState<Project>,
    priority: DropdownState<Priority>,
}

impl PanelState {
    fn create() -> Self {
        Self {
            mode: PanelMode::Create,
            draft: TextState::new(""),
            project: DropdownState::default(),
            priority: DropdownState::default(),
        }
    }

    fn edit(index: usize, task: &Task) -> Self {
        Self {
            mode: PanelMode::Edit(index),
            draft: TextState::new(&task.title),
            project: DropdownState {
                selected: task.project,
                ..DropdownState::default()
            },
            priority: DropdownState {
                selected: task.priority,
                ..DropdownState::default()
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum PanelMode {
    Create,
    Edit(usize),
}

#[derive(Debug, Clone)]
struct PanelActionsState {
    panel: Option<PanelState>,
    tasks: Vec<Task>,
    selected_task: Option<usize>,
    row_buttons: Vec<ButtonState>,
    cancel_button: ButtonState,
    save_button: ButtonState,
}

#[derive(Debug, Clone)]
struct Task {
    title: String,
    project: Project,
    priority: Priority,
}

fn panel_actions_scope() -> OwnedBinding<State, PanelActionsState> {
    OwnedBinding::new(
        |state: &State| {
            state.panel.as_ref().map(|panel| PanelActionsState {
                panel: Some(panel.clone()),
                tasks: state.tasks.clone(),
                selected_task: state.selected_task,
                row_buttons: state.row_buttons.clone(),
                cancel_button: state.panel_cancel_button,
                save_button: state.panel_save_button,
            })
        },
        |state, actions| {
            state.panel = actions.panel;
            state.tasks = actions.tasks;
            state.selected_task = actions.selected_task;
            state.row_buttons = actions.row_buttons;
            state.panel_cancel_button = actions.cancel_button;
            state.panel_save_button = actions.save_button;
        },
    )
}

impl Task {
    fn new(title: &str, project: Project, priority: Priority) -> Self {
        Self {
            title: title.to_string(),
            project,
            priority,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Project {
    #[default]
    Canopy,
    Garden,
    Commons,
    People,
}

impl Project {
    const ALL: [Project; 4] = [
        Project::Canopy,
        Project::Garden,
        Project::Commons,
        Project::People,
    ];

    fn label(self) -> &'static str {
        match self {
            Project::Canopy => "Canopy",
            Project::Garden => "Garden",
            Project::Commons => "Commons",
            Project::People => "People",
        }
    }

    fn color(self) -> Color {
        match self {
            Project::Canopy => Color::from_rgb8(88, 196, 126),
            Project::Garden => Color::from_rgb8(230, 178, 74),
            Project::Commons => Color::from_rgb8(70, 190, 220),
            Project::People => Color::from_rgb8(222, 108, 164),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Priority {
    Low,
    #[default]
    Medium,
    High,
}

impl Priority {
    const ALL: [Priority; 3] = [Priority::Low, Priority::Medium, Priority::High];

    fn label(self) -> &'static str {
        match self {
            Priority::Low => "Low",
            Priority::Medium => "Medium",
            Priority::High => "High",
        }
    }

    fn color(self) -> Color {
        match self {
            Priority::Low => Color::from_rgb8(120, 200, 120),
            Priority::Medium => Color::from_rgb8(230, 180, 80),
            Priority::High => Color::from_rgb8(240, 90, 90),
        }
    }

    fn index(self) -> usize {
        match self {
            Priority::Low => 0,
            Priority::Medium => 1,
            Priority::High => 2,
        }
    }
}

fn main() {
    WinitApp::new(State::default())
        .pane(PaneBuilder::new("main", main_view).inner_size(980, 680))
        .run();
}

fn main_view<'a>(state: &'a State, app: &mut PaneState) -> View<'a, State> {
    stack(vec![
        rect(id!()).fill(Color::from_rgb8(18, 18, 24)).build(app),
        column_spaced(
            18.,
            vec![
                header(state, app).expand_x(),
                row_spaced(
                    18.,
                    vec![
                        task_list(state, app).expand(),
                        panel_slot(state, app).expand_y(),
                    ],
                )
                .expand(),
                summary_chart(state, app).expand_x().height(200.),
            ],
        )
        .pad(24.)
        .align(Align::Top),
    ])
}

fn header<'a>(state: &'a State, app: &mut PaneState) -> View<'a, State> {
    row_spaced(
        14.,
        vec![
            column(vec![
                text(id!(), "Solarpunk cockpit")
                    .font_size(32)
                    .font_weight(FontWeight::BOLD)
                    .align(Alignment::Start)
                    .build(app),
                rich_text(
                    id!(),
                    [
                        span(state.tasks.len().to_string())
                            .bold()
                            .color(DEFAULT_PURP),
                        " tending city systems under the afternoon glass".into(),
                    ],
                )
                .align(Alignment::Start)
                .build(app),
            ])
            .expand_x(),
            if state.panel.is_none() {
                button(id!(), binding!(state.compose_button))
                    .text_label("New task")
                    .on_click(|state, _| {
                        if state.panel.is_none() {
                            state.panel = Some(PanelState::create());
                        } else {
                            state.panel = None;
                        }
                        state.selected_task = None;
                    })
                    .build(app)
                    .height(38.)
            } else {
                empty()
            },
        ],
    )
}

fn task_list<'a>(state: &'a State, app: &mut PaneState) -> View<'a, State> {
    let tasks = state.tasks.clone();
    let row_buttons = state.row_buttons.clone();
    let selected_task = state.selected_task;
    stack(vec![
        card(id!(), app),
        column_spaced(
            12.,
            vec![
                text(id!(), "Work queue")
                    .font_weight(FontWeight::BOLD)
                    .font_size(18)
                    .align(Alignment::Start)
                    .build(app),
                scroller(
                    id!(),
                    None,
                    move |index, _, ctx| {
                        tasks.get(index).map(|task| {
                            let button_state = row_buttons.get(index).copied().unwrap_or_default();
                            task_row(index, task, button_state, selected_task == Some(index), ctx)
                        })
                    },
                    app,
                )
                .expand(),
            ],
        )
        .pad(16.),
    ])
}

fn task_row<'a>(
    index: usize,
    task: &Task,
    button_state: ButtonState,
    selected: bool,
    app: &mut PaneState,
) -> View<'a, State> {
    button(
        id!(index as u64),
        (
            &button_state,
            Binding::new(
                move |state: &State| &state.row_buttons[index],
                move |state: &mut State| &mut state.row_buttons[index],
            ),
        ),
    )
    .surface(move |button_state, app| {
        rect(id!(index as u64))
            .fill(if selected {
                Color::from_rgb8(43, 39, 62)
            } else if button_state.hovered {
                Color::from_rgb8(38, 38, 50)
            } else {
                Color::from_rgb8(32, 32, 42)
            })
            .stroke(
                if selected {
                    DEFAULT_PURP
                } else {
                    Color::from_rgb8(42, 42, 54)
                },
                Stroke::new(1.),
            )
            .corner_rounding(10.)
            .build(app)
    })
    .label({
        let task = task.clone();
        move |_, app| {
            row_spaced(
                10.,
                vec![
                    rect(id!(index as u64))
                        .fill(task.project.color())
                        .corner_rounding(4.)
                        .build(app)
                        .width(5.)
                        .expand_y(),
                    column(vec![
                        text(id!(index as u64), &task.title)
                            .align(Alignment::Start)
                            .font_weight(FontWeight::BOLD)
                            .build(app)
                            .expand_x(),
                        text(id!(index as u64), task.project.label())
                            .align(Alignment::Start)
                            .fill(Color::from_rgb8(170, 170, 185))
                            .build(app)
                            .expand_x(),
                    ])
                    .expand_x(),
                    priority_pill(id!(index as u64), task.priority, app),
                ],
            )
            .expand_x()
            .pad(12.)
        }
    })
    .on_click(move |state, _| {
        state.selected_task = Some(index);
        state.panel = None;
    })
    .build(app)
    .align(Align::Leading)
    .pad(4.)
}

fn priority_pill<'a, State: 'static>(
    id: u64,
    priority: Priority,
    app: &mut PaneState,
) -> View<'a, State> {
    stack(vec![
        rect(id!(id))
            .fill(priority.color().with_alpha(0.2))
            .corner_rounding(999.)
            .build(app),
        text(id!(id), priority.label())
            .fill(priority.color())
            .font_weight(FontWeight::BOLD)
            .build(app)
            .pad_x(12.)
            .pad_y(5.),
    ])
    .height(28.)
}

fn project_pill<'a, State: 'static>(
    id: u64,
    project: Project,
    app: &mut PaneState,
) -> View<'a, State> {
    stack(vec![
        rect(id!(id))
            .fill(project.color().with_alpha(0.2))
            .corner_rounding(999.)
            .build(app),
        text(id!(id), project.label())
            .fill(project.color())
            .font_weight(FontWeight::BOLD)
            .build(app)
            .pad_x(12.)
            .pad_y(5.),
    ])
    .height(28.)
}

fn panel_slot<'a>(state: &'a State, app: &mut PaneState) -> View<'a, State> {
    if let Some(panel) = &state.panel {
        let title = match panel.mode {
            PanelMode::Create => "Create a task",
            PanelMode::Edit(_) => "Edit task",
        };
        return stack(vec![
            card(id!(), app),
            column_spaced(
                12.,
                vec![
                    text(id!(), title)
                        .font_size(20)
                        .font_weight(FontWeight::BOLD)
                        .align(Alignment::Start)
                        .build(app),
                    owned_scope(
                        panel_form(panel, app),
                        OwnedBinding::new(
                            |state: &State| state.panel.clone(),
                            |state, panel| state.panel = Some(panel),
                        ),
                    ),
                    owned_scope(
                        panel_actions(
                            PanelActionsState {
                                panel: Some(panel.clone()),
                                tasks: state.tasks.clone(),
                                selected_task: state.selected_task,
                                row_buttons: state.row_buttons.clone(),
                                cancel_button: state.panel_cancel_button,
                                save_button: state.panel_save_button,
                            },
                            app,
                        ),
                        panel_actions_scope(),
                    ),
                ],
            )
            .pad(16.),
        ]);
    }

    if let Some(index) = state.selected_task
        && let Some(task) = state.tasks.get(index)
    {
        return selected_task_panel(index, task, state, app);
    }

    empty_panel(app)
}

fn empty_panel<'a>(app: &mut PaneState) -> View<'a, State> {
    stack(vec![
        card(id!(), app),
        column_spaced(
            12.,
            vec![
                text(id!(), "No route selected")
                    .font_size(20)
                    .font_weight(FontWeight::BOLD)
                    .build(app)
                    .expand_x(),
                text(id!(), "Pick a work item or plant a new one.")
                    .wrap()
                    .fill(Color::from_rgb8(170, 170, 185))
                    .build(app)
                    .expand_x(),
            ],
        )
        .pad(22.),
    ])
}

fn selected_task_panel<'a>(
    index: usize,
    task: &Task,
    state: &'a State,
    app: &mut PaneState,
) -> View<'a, State> {
    stack(vec![
        card(id!(index as u64), app),
        column_spaced(
            12.,
            vec![
                text(id!(), "Work Item")
                    .font_size(20)
                    .font_weight(FontWeight::BOLD)
                    .build(app)
                    .expand_x(),
                task_card_summary(index as u64, &task.title, task.project, task.priority, app),
                text(id!(), format!("Canopy route stop {}", index + 1))
                    .align(Alignment::Start)
                    .fill(Color::from_rgb8(150, 150, 165))
                    .build(app)
                    .expand_x(),
                button(id!(index as u64), binding!(state.detail_edit_button))
                    .text_label("Edit")
                    .on_click(move |state, _| {
                        if let Some(task) = state.tasks.get(index) {
                            state.panel = Some(PanelState::edit(index, task));
                            state.selected_task = Some(index);
                        }
                    })
                    .build(app)
                    .height(38.)
                    .expand_x(),
            ],
        )
        .pad(16.)
        .align(Align::Top),
    ])
}

fn panel_actions<'a>(state: PanelActionsState, app: &mut PaneState) -> View<'a, PanelActionsState> {
    let state = &state;
    row_spaced(
        10.,
        vec![
            button(id!(), binding!(state.cancel_button))
                .text_label("Cancel")
                .on_click(|state, _| state.panel = None)
                .build(app)
                .expand_x()
                .height(38.),
            button(id!(), binding!(state.save_button))
                .text_label(match state.panel.as_ref().map(|panel| panel.mode) {
                    Some(PanelMode::Edit(_)) => "Save",
                    _ => "Create",
                })
                .on_click(|state, _| {
                    let Some(panel) = &state.panel else { return };
                    let title = panel.draft.text.trim();
                    if title.is_empty() {
                        return;
                    }
                    let task = Task {
                        title: title.to_string(),
                        project: panel.project.selected,
                        priority: panel.priority.selected,
                    };
                    match panel.mode {
                        PanelMode::Create => {
                            state.tasks.insert(0, task);
                            state.row_buttons.insert(0, ButtonState::default());
                            state.selected_task = None;
                        }
                        PanelMode::Edit(index) => {
                            if let Some(existing) = state.tasks.get_mut(index) {
                                *existing = task;
                                state.selected_task = Some(index);
                            }
                        }
                    }
                    state.panel = None;
                })
                .build(app)
                .expand_x()
                .height(38.),
        ],
    )
}

fn panel_form<'a>(state: &'a PanelState, app: &mut PaneState) -> View<'a, PanelState> {
    let preview_title = state.draft.text.trim();
    let preview_title = if preview_title.is_empty() {
        "Untitled task"
    } else {
        preview_title
    };
    column_spaced(
        12.,
        vec![
            task_card_summary(
                id!(),
                preview_title,
                state.project.selected,
                state.priority.selected,
                app,
            ),
            text_field(id!(), binding!(state.draft))
                .hint_text("Enter a task title...")
                .align(Alignment::Start)
                .enter_end_editing()
                .build(app),
            row_spaced(
                12.,
                vec![
                    dropdown(
                        id!(),
                        binding!(state.project),
                        Project::ALL.to_vec(),
                        |ctx, app| {
                            dropdown_label(
                                id!(ctx.index as u64),
                                ctx.value.label(),
                                ctx.selected,
                                ctx.hovered,
                                app,
                            )
                        },
                    )
                    .build(app),
                    dropdown(
                        id!(),
                        binding!(state.priority),
                        Priority::ALL.to_vec(),
                        |ctx, app| {
                            dropdown_label(
                                id!(ctx.index as u64),
                                ctx.value.label(),
                                ctx.selected,
                                ctx.hovered,
                                app,
                            )
                        },
                    )
                    .build(app),
                ],
            ),
        ],
    )
}

fn task_card_summary<'a, State: 'static>(
    id: u64,
    title: &str,
    project: Project,
    priority: Priority,
    app: &mut PaneState,
) -> View<'a, State> {
    column_spaced(
        10.,
        vec![
            text(id!(id), title)
                .font_weight(FontWeight::BOLD)
                .wrap()
                .align(Alignment::Start)
                .build(app)
                .expand_x(),
            row_spaced(
                10.,
                vec![
                    project_pill(id!(id, 1u64), project, app),
                    priority_pill(id!(id, 2u64), priority, app),
                ],
            ),
        ],
    )
}

fn dropdown_label<'a, State: 'static>(
    id: u64,
    label: &str,
    selected: bool,
    hovered: bool,
    app: &mut PaneState,
) -> View<'a, State> {
    text(id!(id), label)
        .fill(if selected || hovered {
            DEFAULT_FG
        } else {
            Color::from_rgb8(180, 180, 195)
        })
        .build(app)
        .pad_x(12.)
        .pad_y(8.)
        .expand_x()
}

fn summary_chart<'a>(state: &'a State, app: &mut PaneState) -> View<'a, State> {
    let data = project_priority_counts(&state.tasks);
    let totals = data.map(|counts| counts.iter().sum::<usize>());
    let max_count = totals.iter().copied().max().unwrap_or(1).max(1);
    stack(vec![
        card(id!(), app),
        column_spaced(
            10.,
            vec![
                row_spaced(
                    12.,
                    vec![
                        text(id!(), "Workload by project")
                            .align(Alignment::Start)
                            .font_weight(FontWeight::BOLD)
                            .build(app)
                            .expand_x(),
                        priority_legend(app),
                    ],
                ),
                row_spaced(
                    8.,
                    vec![
                        column(vec![
                            chart_y_axis(max_count, app).expand(),
                            space().height(32.),
                        ])
                        .width(28.),
                        column(vec![
                            draw(move |area, ctx| {
                                let max = max_count as f32;
                                let bar_width = area.width / data.len() as f32;
                                let mut views = Vec::new();
                                for (tick, ratio) in [0., 0.5, 1.].into_iter().enumerate() {
                                    let y = area.y + area.height - area.height * ratio;
                                    views.extend(
                                        rect(id!(tick as u64))
                                            .fill(Color::from_rgb8(58, 58, 72))
                                            .build(ctx)
                                            .width(area.width)
                                            .height(if tick == 0 { 2. } else { 1. })
                                            .offset(0., y - area.y - (area.height * 0.5))
                                            .draw(area, ctx),
                                    );
                                }
                                for (project_index, counts) in data.iter().enumerate() {
                                    let mut bottom = area.y + area.height;
                                    let x = area.x + project_index as f32 * bar_width;
                                    for priority in Priority::ALL {
                                        let value = counts[priority.index()];
                                        if value == 0 {
                                            continue;
                                        }
                                        let height = area.height * (value as f32 / max);
                                        bottom -= height;
                                        views.extend(
                                            rect(id!(
                                                project_index as u64,
                                                priority.index() as u64
                                            ))
                                            .fill(priority.color())
                                            .corner_rounding(4.)
                                            .build(ctx)
                                            .width((bar_width - 16.).max(6.))
                                            .height(height)
                                            .offset(
                                                x + (bar_width * 0.5) - area.x - (area.width * 0.5),
                                                bottom + (height * 0.5)
                                                    - area.y
                                                    - (area.height * 0.5),
                                            )
                                            .draw(area, ctx),
                                        );
                                    }
                                }
                                views
                            })
                            .expand(),
                            row(Project::ALL
                                .iter()
                                .enumerate()
                                .map(|(index, project)| {
                                    chart_project_label(index, *project, totals[index], app)
                                        .expand_x()
                                })
                                .collect())
                            .expand_x()
                            .height(32.),
                        ])
                        .expand(),
                    ],
                )
                .expand(),
            ],
        )
        .pad(14.),
    ])
}

fn chart_y_axis<'a, State: 'static>(max: usize, app: &mut PaneState) -> View<'a, State> {
    column(vec![
        chart_y_tick(max, app),
        space().expand(),
        chart_y_tick(max.div_ceil(2), app),
        space().expand(),
        chart_y_tick(0, app),
    ])
}

fn chart_y_tick<'a, State: 'static>(value: usize, app: &mut PaneState) -> View<'a, State> {
    text(id!(), value.to_string())
        .fill(Color::from_rgb8(150, 150, 165))
        .font_size(11)
        .align(Alignment::End)
        .build(app)
        .expand_x()
}

fn priority_legend<'a, State: 'static>(app: &mut PaneState) -> View<'a, State> {
    row_spaced(
        10.,
        Priority::ALL
            .iter()
            .map(|priority| {
                row_spaced(
                    5.,
                    vec![
                        rect(id!())
                            .fill(priority.color())
                            .corner_rounding(999.)
                            .build(app)
                            .width(8.)
                            .height(8.),
                        text(id!(), priority.label())
                            .fill(Color::from_rgb8(170, 170, 185))
                            .font_size(12)
                            .build(app),
                    ],
                )
            })
            .collect(),
    )
}

fn chart_project_label<'a, State: 'static>(
    index: usize,
    project: Project,
    count: usize,
    app: &mut PaneState,
) -> View<'a, State> {
    column(vec![
        text(id!(index as u64), project.label())
            .fill(project.color())
            .font_size(12)
            .font_weight(FontWeight::BOLD)
            .build(app)
            .expand_x(),
        text(id!(index as u64), count.to_string())
            .fill(Color::from_rgb8(170, 170, 185))
            .font_size(12)
            .build(app)
            .expand_x(),
    ])
}

fn card<'a, State: 'static>(id: u64, app: &mut PaneState) -> View<'a, State> {
    rect(id)
        .fill(Color::from_rgb8(26, 26, 36))
        .stroke(Color::from_rgb8(48, 48, 62), Stroke::new(1.))
        .corner_rounding(14.)
        .build(app)
}

fn project_priority_counts(tasks: &[Task]) -> [[usize; 3]; 4] {
    let mut counts = [[0; 3]; 4];
    for task in tasks {
        let project_index = match task.project {
            Project::Canopy => 0,
            Project::Garden => 1,
            Project::Commons => 2,
            Project::People => 3,
        };
        counts[project_index][task.priority.index()] += 1;
    }
    counts
}
