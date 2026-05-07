use crate::draw_layout::draw_layout;
use crate::gestures::{ClickLocation, Interaction, ScrollDelta};

use crate::editor::Editor;
use crate::text::TextLayout;
use crate::view::DrawableType;
use crate::{
    ClickState, DragState, GestureHandler, GestureState, Key, Modifiers, Point, RUBIK_FONT,
    area_contains,
};
use backer::{Area, Layout};
use parley::fontique::Blob;
use parley::fontique::FontInfoOverride;
use parley::{
    Alignment, FontContext, FontWeight, LayoutContext, LineHeight, OverflowWrap, PlainEditor,
    StyleProperty,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use vello_svg::vello::Scene;
use vello_svg::vello::kurbo::{Affine, BezPath};
use vello_svg::vello::peniko::{self, Brush, Color, Fill};

type FontEntry = (Arc<Vec<u8>>, Option<String>);

type ViewFn<State> = for<'a> fn(&'a State, &mut PaneState) -> Layout<'a, View<State>, PaneState>;

pub struct PaneConfig<State> {
    name: &'static str,
    view: ViewFn<State>,
    inner_size: Option<(u32, u32)>,
    resizable: Option<bool>,
    title: Option<String>,
    transparent: Option<bool>,
    background: Option<Color>,
    decorations: Option<bool>,
    open_at_start: bool,
    on_frame: fn(&mut State, &mut PaneState) -> (),
    on_start: fn(&mut State, &mut PaneState) -> (),
    on_exit: fn(&mut State, &mut PaneState) -> (),
    custom_fonts: Vec<FontEntry>,
}

impl<State> Clone for PaneConfig<State> {
    fn clone(&self) -> Self {
        Self {
            name: self.name,
            view: self.view,
            inner_size: self.inner_size,
            resizable: self.resizable,
            title: self.title.clone(),
            transparent: self.transparent,
            background: self.background,
            decorations: self.decorations,
            open_at_start: self.open_at_start,
            on_frame: self.on_frame,
            on_start: self.on_start,
            on_exit: self.on_exit,
            custom_fonts: self.custom_fonts.clone(),
        }
    }
}

pub(crate) struct Pane<State> {
    name: &'static str,
    base_color: Color,
    view: ViewFn<State>,
    scene: Scene,
    gesture_handlers: Vec<(u64, Area, GestureHandler<State, PaneState>)>,
    cursor_position: Option<Point>,
    gesture_state: GestureState,
    pane_state: PaneState,
    state: State,
    on_frame: fn(&mut State, &mut PaneState) -> (),
    on_start: fn(&mut State, &mut PaneState) -> (),
    on_exit: fn(&mut State, &mut PaneState) -> (),
    started: bool,
}

impl<State> PaneConfig<State> {
    pub fn new(name: &'static str, view: ViewFn<State>) -> Self {
        Self {
            name,
            view,
            inner_size: None,
            resizable: None,
            title: None,
            transparent: None,
            background: None,
            decorations: None,
            open_at_start: true,
            on_frame: |_, _| {},
            on_start: |_, _| {},
            on_exit: |_, _| {},
            custom_fonts: Vec::new(),
        }
    }

    pub fn inner_size(mut self, width: u32, height: u32) -> Self {
        self.inner_size = Some((width, height));
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = Some(resizable);
        self
    }

    pub fn title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn transparent(mut self, transparent: bool) -> Self {
        self.transparent = Some(transparent);
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub fn decorations(mut self, decorations: bool) -> Self {
        self.decorations = Some(decorations);
        self
    }

    pub fn open_at_start(mut self, open: bool) -> Self {
        self.open_at_start = open;
        self
    }

    pub fn add_font_bytes(mut self, bytes: Vec<u8>, family: Option<&str>) -> Self {
        self.custom_fonts
            .push((Arc::new(bytes), family.map(|s| s.to_string())));
        self
    }

    pub fn on_frame(mut self, on_frame: fn(&mut State, &mut PaneState) -> ()) -> Self {
        self.on_frame = on_frame;
        self
    }

    pub fn on_start(mut self, on_start: fn(&mut State, &mut PaneState) -> ()) -> Self {
        self.on_start = on_start;
        self
    }

    pub fn on_exit(mut self, on_exit: fn(&mut State, &mut PaneState) -> ()) -> Self {
        self.on_exit = on_exit;
        self
    }
    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn inner_size_value(&self) -> Option<(u32, u32)> {
        self.inner_size
    }

    pub fn resizable_value(&self) -> Option<bool> {
        self.resizable
    }

    pub fn title_value(&self) -> Option<&str> {
        self.title.as_deref()
    }

    pub fn transparent_value(&self) -> Option<bool> {
        self.transparent
    }

    pub fn background_value(&self) -> Option<Color> {
        self.background
    }

    pub(crate) fn build(self, state: State, redraw: Redraw) -> Pane<State>
    where
        State: 'static,
    {
        Pane::new(state, self, redraw)
    }

    pub fn decorations_value(&self) -> Option<bool> {
        self.decorations
    }

    pub fn open_at_start_value(&self) -> bool {
        self.open_at_start
    }
}

pub(crate) type LayoutCache = HashMap<
    u64,
    Vec<(
        String,
        Vec<(
            std::ops::Range<usize>,
            parley::StyleProperty<'static, Brush>,
        )>,
        Vec<(std::ops::Range<usize>, Brush)>,
        f32,
        parley::Layout<Brush>,
    )>,
>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneEffect {
    Open(&'static str),
    Close,
    Redraw,
}

pub struct PaneState {
    pub(crate) text_layout: TextLayout,
    pub(crate) font_cx: FontContext,
    pub(crate) layout_cx: LayoutContext<Brush>,
    pub(crate) scale_factor: f64,
    pub(crate) editor: Option<EditState>,
    pub(crate) editor_areas: HashMap<u64, Area>,
    pub(crate) scrollers: HashMap<u64, crate::scroller::ScrollerState>,
    pub(crate) needs_redraw: bool,
    pub(crate) task_runtime: Runtime,
    pub(crate) cancellation_token: CancellationToken,
    pub(crate) task_tracker: TaskTracker,
    pub(crate) svg_scenes: HashMap<String, (Scene, f32, f32)>,
    pub(crate) image_scenes: HashMap<u64, (Scene, f32, f32)>,
    pub(crate) modifiers: Option<Modifiers>,
    pub(crate) redraw: Redraw,
    pub(crate) effects: Vec<PaneEffect>,
    pub(crate) cursor_position: Option<Point>,
}

pub enum View<State> {
    Draw {
        view: Box<DrawableType>,
        gesture_handlers: Vec<GestureHandler<State, PaneState>>,
        area: Area,
    },
    PushLayer {
        path: BezPath,
        blend: peniko::BlendMode,
        alpha: f32,
    },
    PopLayer,
    EditorArea(u64, Area),
    Empty,
}

pub(crate) struct EditState {
    pub(crate) id: u64,
    pub(crate) editor: Editor,
    pub(crate) editing: bool,
    pub(crate) cursor_color: Brush,
    pub(crate) highlight_color: Brush,
}

impl Clone for EditState {
    fn clone(&self) -> Self {
        EditState {
            id: self.id,
            editor: self.editor.clone(),
            editing: self.editing,
            cursor_color: self.cursor_color.clone(),
            highlight_color: self.highlight_color.clone(),
        }
    }
}

impl PaneState {
    pub fn open(&mut self, config: &'static str) {
        self.effects.push(PaneEffect::Open(config));
    }

    pub fn close(&mut self) {
        self.effects.push(PaneEffect::Close);
    }

    pub fn end_editing(&mut self) {
        if self.editor.is_some() {
            self.editor = None;
        }
    }

    pub fn begin_editing(
        &mut self,
        id: u64,
        text: String,
        fill: Brush,
        font_family: String,
        font_weight: FontWeight,
        line_height: f32,
        font_size: f32,
        overflow_wrap: OverflowWrap,
        alignment: Alignment,
        cursor_fill: Brush,
        highlight_fill: Brush,
        wrap: bool,
    ) {
        if self.editor.is_some() {
            return;
        }
        let Some(area) = self.editor_areas.get(&id) else {
            return;
        };
        let mut editor = PlainEditor::new(font_size);
        editor.set_text(&text);
        let styles = editor.edit_styles();

        styles.insert(parley::StyleProperty::Brush(fill));
        styles.insert(parley::FontFamily::Named(font_family.into()).into());
        styles.insert(StyleProperty::FontWeight(font_weight));
        styles.insert(StyleProperty::LineHeight(LineHeight::FontSizeRelative(
            line_height,
        )));
        styles.insert(StyleProperty::FontSize(font_size));
        styles.insert(StyleProperty::OverflowWrap(overflow_wrap));

        editor.set_alignment(alignment);
        if wrap {
            editor.set_width(Some(area.width));
        }
        let mut editor = Editor {
            editor,
            last_click_time: Default::default(),
            click_count: Default::default(),
            pointer_down: Default::default(),
            cursor_pos: Default::default(),
            cursor_visible: Default::default(),
            modifiers: Default::default(),
            start_time: Default::default(),
            blink_period: Default::default(),
        };

        if let Some(pos) = self.cursor_position {
            editor.mouse_moved(
                Point::new(pos.x - area.x as f64, pos.y - area.y as f64),
                &mut self.layout_cx,
                &mut self.font_cx,
            );
        }
        self.editor = Some(EditState {
            id,
            editor,
            editing: true,
            cursor_color: cursor_fill,
            highlight_color: highlight_fill,
        });
    }

    pub fn spawn(&self, task: impl std::future::Future<Output = ()> + Send + 'static) {
        self.task_tracker.spawn_on(task, self.task_runtime.handle());
    }

    pub fn redraw_trigger(&self) -> RedrawTrigger {
        RedrawTrigger::new(self.redraw.clone())
    }

    pub fn redraw(&mut self) {
        self.effects.push(PaneEffect::Redraw);
        self.redraw.request();
    }
}

pub struct Redraw {
    sender: Arc<dyn Fn() + Send + Sync>,
}

impl Redraw {
    pub fn new(sender: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            sender: Arc::new(sender),
        }
    }

    fn request(&self) {
        (self.sender)();
    }
}

impl Clone for Redraw {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

impl std::fmt::Debug for Redraw {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Redraw").finish()
    }
}

pub struct Callback<T> {
    sender: Arc<dyn Fn(T) + Send + Sync>,
}

impl<T> Clone for Callback<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

impl<T> Callback<T> {
    pub fn new(sender: impl Fn(T) + Send + Sync + 'static) -> Self {
        Self {
            sender: Arc::new(sender),
        }
    }

    pub fn send(&self, value: T) {
        (self.sender)(value);
    }
}

#[derive(Debug, Clone)]
pub struct RedrawTrigger {
    redraw: Redraw,
}

impl RedrawTrigger {
    pub fn new(redraw: Redraw) -> Self {
        Self { redraw }
    }

    pub async fn trigger(&self) {
        self.redraw.request();
    }
}

impl<State: 'static> Pane<State> {
    fn new(state: State, config: PaneConfig<State>, redraw: Redraw) -> Self {
        let mut font_cx = FontContext::new();

        font_cx
            .collection
            .register_fonts(Blob::new(Arc::new(RUBIK_FONT)), None);

        for (font_bytes, family_opt) in config.custom_fonts.iter() {
            font_cx.collection.register_fonts(
                Blob::new(font_bytes.clone()),
                Some(FontInfoOverride {
                    family_name: family_opt.as_deref(),
                    ..Default::default()
                }),
            );
        }

        let task_runtime = Runtime::new().expect("Failed to create task runtime");
        let layout_cache = HashMap::new();
        let layout_cx = LayoutContext::new();
        let font_cx_inner = FontContext::new();
        let base_color = if config.transparent.unwrap_or(false) {
            Color::TRANSPARENT
        } else {
            config.background.unwrap_or(Color::BLACK)
        };

        Self {
            scene: Scene::new(),
            name: config.name,
            base_color,
            view: config.view,
            gesture_handlers: Vec::new(),
            cursor_position: None,
            gesture_state: GestureState::None,
            pane_state: PaneState {
                task_runtime,
                cancellation_token: CancellationToken::new(),
                task_tracker: TaskTracker::new(),
                text_layout: TextLayout::new(layout_cache, font_cx_inner, layout_cx),
                font_cx: FontContext::new(),
                layout_cx: LayoutContext::new(),
                scale_factor: 1.,
                editor: None,
                editor_areas: HashMap::new(),
                scrollers: HashMap::new(),
                needs_redraw: false,
                image_scenes: HashMap::new(),
                svg_scenes: HashMap::new(),
                modifiers: None,
                redraw,
                effects: Vec::new(),
                cursor_position: None,
            },
            state,
            on_frame: config.on_frame,
            on_start: config.on_start,
            on_exit: config.on_exit,
            started: false,
        }
    }

    pub(crate) fn name(&self) -> &'static str {
        self.name
    }

    pub(crate) fn base_color(&self) -> Color {
        self.base_color
    }

    pub(crate) fn scene(&self) -> &Scene {
        &self.scene
    }

    pub(crate) fn reset_scene(&mut self) {
        self.scene.reset();
    }

    pub(crate) fn close(mut self) {
        (self.on_exit)(&mut self.state, &mut self.pane_state);
        self.pane_state.cancellation_token.cancel();
        self.pane_state.task_tracker.close();
        self.pane_state.task_runtime.block_on(async {
            tokio::time::timeout(Duration::from_secs(1), self.pane_state.task_tracker.wait())
                .await
                .ok();
        });
    }

    pub(crate) fn redraw(&mut self, width: u32, height: u32, scale_factor: f64) -> Vec<PaneEffect> {
        if !self.started {
            self.started = true;
            (self.on_start)(&mut self.state, &mut self.pane_state);
        }

        self.gesture_handlers.clear();
        self.pane_state.scale_factor = scale_factor;

        let view = self.view;
        let draw_items = {
            let mut layout = view(&self.state, &mut self.pane_state);
            layout.draw(
                Area {
                    x: 0.,
                    y: 0.,
                    width: ((width as f64) / self.pane_state.scale_factor) as f32,
                    height: ((height as f64) / self.pane_state.scale_factor) as f32,
                },
                &mut self.pane_state,
            )
        };

        let continue_animating = std::mem::take(&mut self.pane_state.needs_redraw);

        for item in draw_items {
            match item {
                View::PushLayer { path, blend, alpha } => {
                    self.scene.push_layer(
                        Fill::NonZero,
                        blend,
                        alpha,
                        Affine::scale(self.pane_state.scale_factor),
                        &path,
                    );
                }
                View::PopLayer => {
                    self.scene.pop_layer();
                }
                View::EditorArea(id, area) => {
                    self.pane_state.editor_areas.insert(id, area);
                }
                View::Draw {
                    mut view,
                    gesture_handlers,
                    area,
                } => {
                    let id = view.id();
                    let draw_area = area;

                    self.gesture_handlers.extend(
                        gesture_handlers
                            .into_iter()
                            .map(|handler| (id, draw_area, handler)),
                    );

                    match &mut *view {
                        DrawableType::Text(v) => {
                            v.draw(draw_area, area, &mut self.scene, &mut self.pane_state)
                        }
                        DrawableType::Layout(boxed) => {
                            let (layout, transform) = boxed.as_mut();
                            draw_layout(*transform, layout, &mut self.scene)
                        }
                        DrawableType::Path(v) => {
                            v.draw(&mut self.scene, draw_area, self.pane_state.scale_factor)
                        }
                        DrawableType::Svg(v) => {
                            v.draw(draw_area, &mut self.scene, &mut self.pane_state)
                        }
                        DrawableType::Image(v) => {
                            v.draw(draw_area, &mut self.scene, &mut self.pane_state)
                        }
                    }
                }
                View::Empty => (),
            }
        }

        (self.on_frame)(&mut self.state, &mut self.pane_state);

        if continue_animating {
            self.pane_state.redraw.request();
        }
        self.take_effects()
    }
}
impl<State: 'static> Pane<State> {
    fn gesture_handlers(&self) -> Vec<(u64, Area, GestureHandler<State, PaneState>)> {
        self.gesture_handlers.clone()
    }

    fn take_effects(&mut self) -> Vec<PaneEffect> {
        std::mem::take(&mut self.pane_state.effects)
    }

    pub(crate) fn key_pressed(&mut self, key: Key) -> Vec<PaneEffect> {
        let mut needs_redraw = false;
        for (_id, _area, handler) in self.gesture_handlers() {
            if let Some(ref interaction_handler) = handler.interaction_handler
                && handler.interaction_type.key
            {
                needs_redraw = true;
                interaction_handler(
                    &mut self.state,
                    &mut self.pane_state,
                    Interaction::Key(key.clone()),
                );
            }
        }
        if needs_redraw {
            self.pane_state.redraw.request();
        }
        self.take_effects()
    }

    pub(crate) fn modifiers_changed(&mut self, modifiers: Modifiers) -> Vec<PaneEffect> {
        self.pane_state.modifiers = Some(modifiers);
        self.take_effects()
    }

    pub(crate) fn scale_factor_changed(&mut self, scale_factor: f64) -> Vec<PaneEffect> {
        self.pane_state.scale_factor = scale_factor;
        self.pane_state.text_layout.layout_cache.clear();
        self.pane_state.redraw.request();
        self.take_effects()
    }

    pub(crate) fn exit(&mut self) -> Vec<PaneEffect> {
        self.cursor_position = None;
        let mut needs_redraw = false;
        for (_, _, gh) in self.gesture_handlers() {
            if gh.interaction_type.hover
                && let Some(ref on_hover) = gh.interaction_handler
            {
                needs_redraw = true;
                on_hover(
                    &mut self.state,
                    &mut self.pane_state,
                    Interaction::Hover(false),
                );
            }
        }
        if needs_redraw {
            self.pane_state.redraw.request();
        }
        self.take_effects()
    }

    pub(crate) fn move_to(&mut self, pos: Point) -> Vec<PaneEffect> {
        let pos = Point::new(
            pos.x / self.pane_state.scale_factor,
            pos.y / self.pane_state.scale_factor,
        );
        let mut needs_redraw = false;
        self.cursor_position = Some(pos);
        self.pane_state.cursor_position = Some(pos);
        if let Some(EditState { id, editor, .. }) = self.pane_state.editor.as_mut()
            && let Some(area) = self.pane_state.editor_areas.get(id).copied()
        {
            needs_redraw = true;
            editor.mouse_moved(
                Point::new(pos.x - area.x as f64, pos.y - area.y as f64),
                &mut self.pane_state.layout_cx,
                &mut self.pane_state.font_cx,
            );
        }
        self.gesture_handlers().iter().for_each(|(_, area, gh)| {
            if gh.interaction_type.hover
                && let Some(ref on_hover) = gh.interaction_handler
            {
                needs_redraw = true;
                (on_hover)(
                    &mut self.state,
                    &mut self.pane_state,
                    Interaction::Hover(area_contains(area, pos)),
                );
            }
        });
        let gesture_state = self.gesture_state;
        if let GestureState::Dragging {
            start,
            last_position,
            capturer,
        } = gesture_state
        {
            let distance = start.distance(pos);
            let delta = Point {
                x: pos.x - last_position.x,
                y: pos.y - last_position.y,
            };
            self.gesture_handlers()
                .iter()
                .filter(|(id, _, gh)| *id == capturer && gh.interaction_type.drag)
                .for_each(|(_, area, gh)| {
                    needs_redraw = true;
                    if let Some(handler) = &gh.interaction_handler {
                        (handler)(
                            &mut self.state,
                            &mut self.pane_state,
                            Interaction::Drag(DragState::Updated {
                                start: Point {
                                    x: start.x - area.x as f64,
                                    y: start.y - area.y as f64,
                                },
                                current: Point {
                                    x: pos.x - area.x as f64,
                                    y: pos.y - area.y as f64,
                                },
                                start_global: start,
                                current_global: pos,
                                delta,
                                distance: distance as f32,
                            }),
                        );
                    }
                });
            self.gesture_state = GestureState::Dragging {
                start,
                last_position: pos,
                capturer,
            };
        }
        if needs_redraw {
            self.pane_state.redraw.request();
        }
        self.take_effects()
    }
    pub(crate) fn press_current(&mut self) -> Vec<PaneEffect> {
        let mut needs_redraw = false;
        let cursor_position = self.cursor_position;
        if let Some(point) = cursor_position {
            let handlers = self.gesture_handlers();
            let winner = handlers
                .iter()
                .rev()
                .find(|(_, area, handler)| {
                    area_contains(area, point)
                        && (handler.interaction_type.click || handler.interaction_type.drag)
                })
                .or(handlers.iter().rev().find(|(_, area, handler)| {
                    area_contains(
                        &Area {
                            x: area.x - 10.,
                            y: area.y - 10.,
                            width: area.width + 20.,
                            height: area.height + 20.,
                        },
                        point,
                    ) && (handler.interaction_type.click || handler.interaction_type.drag)
                }));
            let winner_id = winner.map(|(id, _, _)| *id);
            for (id, area, handler) in handlers
                .iter()
                .rev()
                .filter(|(_, _, h)| h.interaction_type.click_outside)
            {
                if Some(*id) != winner_id
                    && let Some(ref on_click_outside) = handler.interaction_handler
                {
                    on_click_outside(
                        &mut self.state,
                        &mut self.pane_state,
                        Interaction::ClickOutside(
                            ClickState::Started,
                            ClickLocation::new(point, *area),
                        ),
                    );
                }
            }
            if let Some((capturer, area, handler)) = winner {
                needs_redraw = true;
                if handler.interaction_type.click
                    && let Some(ref on_click) = handler.interaction_handler
                {
                    on_click(
                        &mut self.state,
                        &mut self.pane_state,
                        Interaction::Click(ClickState::Started, ClickLocation::new(point, *area)),
                    );
                } else if handler.interaction_type.drag
                    && let Some(ref on_drag) = handler.interaction_handler
                {
                    on_drag(
                        &mut self.state,
                        &mut self.pane_state,
                        Interaction::Drag(DragState::Began {
                            start: Point {
                                x: point.x - area.x as f64,
                                y: point.y - area.y as f64,
                            },
                            start_global: point,
                        }),
                    );
                }
                self.gesture_state = GestureState::Dragging {
                    start: point,
                    last_position: point,
                    capturer: *capturer,
                };
            }
            if let Some(EditState { editor, .. }) = self.pane_state.editor.as_mut() {
                editor.mouse_pressed(&mut self.pane_state.layout_cx, &mut self.pane_state.font_cx);
            }
        }

        if needs_redraw {
            self.pane_state.redraw.request();
        }
        self.take_effects()
    }
    pub(crate) fn release_current(&mut self) -> Vec<PaneEffect> {
        let mut needs_redraw = false;
        let cursor_position = self.cursor_position;
        let gesture_state = self.gesture_state;
        if let Some(current) = cursor_position {
            if let Some(EditState { id, editor, .. }) = self.pane_state.editor.as_mut()
                && let Some(area) = self.pane_state.editor_areas.get(id)
            {
                editor.mouse_released();
                needs_redraw = true;
                if !area_contains(area, current)
                    && (!matches!(gesture_state, GestureState::Dragging { .. })
                        || match gesture_state {
                            GestureState::Dragging { capturer, .. } => capturer != *id,
                            _ => false,
                        })
                {
                    self.pane_state.end_editing();
                }
            }
            if let GestureState::Dragging {
                start,
                last_position,
                capturer,
            } = gesture_state
            {
                let distance = start.distance(current);
                let delta = Point {
                    x: current.x - last_position.x,
                    y: current.y - last_position.y,
                };
                self.gesture_handlers()
                    .iter()
                    .filter(|(id, _, _)| *id == capturer)
                    .for_each(|(_, area, gh)| {
                        if let (Some(on_click), true) =
                            (&gh.interaction_handler, gh.interaction_type.click)
                        {
                            needs_redraw = true;
                            if area_contains(area, current) {
                                on_click(
                                    &mut self.state,
                                    &mut self.pane_state,
                                    Interaction::Click(
                                        ClickState::Completed,
                                        ClickLocation::new(current, *area),
                                    ),
                                );
                            } else {
                                on_click(
                                    &mut self.state,
                                    &mut self.pane_state,
                                    Interaction::Click(
                                        ClickState::Cancelled,
                                        ClickLocation::new(current, *area),
                                    ),
                                );
                            }
                        }
                        if let (Some(on_drag), true) =
                            (&gh.interaction_handler, gh.interaction_type.drag)
                        {
                            needs_redraw = true;
                            on_drag(
                                &mut self.state,
                                &mut self.pane_state,
                                Interaction::Drag(DragState::Completed {
                                    start: Point {
                                        x: start.x - area.x as f64,
                                        y: start.y - area.y as f64,
                                    },
                                    current: Point {
                                        x: current.x - area.x as f64,
                                        y: current.y - area.y as f64,
                                    },
                                    start_global: start,
                                    current_global: current,
                                    delta,
                                    distance: distance as f32,
                                }),
                            );
                        }
                    });
            }
            let press_capturer = match gesture_state {
                GestureState::Dragging { capturer, .. } => Some(capturer),
                _ => None,
            };
            for (id, area, handler) in self
                .gesture_handlers()
                .iter()
                .filter(|(_, _, h)| h.interaction_type.click_outside)
            {
                if Some(*id) != press_capturer
                    && let Some(ref handler) = handler.interaction_handler
                {
                    needs_redraw = true;
                    handler(
                        &mut self.state,
                        &mut self.pane_state,
                        Interaction::ClickOutside(
                            ClickState::Completed,
                            ClickLocation::new(current, *area),
                        ),
                    );
                }
            }
        }
        self.gesture_state = GestureState::None;
        if needs_redraw {
            self.pane_state.redraw.request();
        }
        self.take_effects()
    }

    pub(crate) fn scroll(&mut self, delta: ScrollDelta) -> Vec<PaneEffect> {
        let mut needs_redraw = false;
        let cursor_position = self.cursor_position;
        if let Some(current) = cursor_position
            && let Some((_, _, handler)) =
                self.gesture_handlers()
                    .iter()
                    .rev()
                    .find(|(_, area, handler)| {
                        area_contains(area, current) && (handler.interaction_type.scroll)
                    })
            && let Some(ref on_scroll) = handler.interaction_handler
        {
            needs_redraw = true;
            on_scroll(
                &mut self.state,
                &mut self.pane_state,
                Interaction::Scroll(delta),
            );
        }
        if needs_redraw {
            self.pane_state.redraw.request();
        }
        self.take_effects()
    }
}
