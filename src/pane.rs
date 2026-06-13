use crate::gestures::{
    ClickEvent, ClickLocation, EditInteraction, Gesture, GestureAreaComponent,
    GestureAreaOperation, GestureId, GestureKind, GesturePropagation, Interaction, ScrollDelta,
    regions::{contains, subtract, valid_area},
};
use crate::prebuilts::TextEditCommand;
use crate::render::{Frame, RenderItem};

use crate::primitives::TextLayout;
use crate::view::DrawableType;
use crate::{ClickPhase, DragPhase, Key, KeyPhase, Modifiers, MouseButton, Point, RUBIK_FONT};
use backer::{Area, Layout};
use parley::fontique::Blob;
use parley::fontique::FontInfoOverride;
use parley::{FontContext, LayoutContext};
use peniko::{self, Brush, Color};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

type FontEntry = (Arc<Vec<u8>>, Option<String>);
pub(crate) type EditHandler<State> = Rc<dyn Fn(&mut State, &mut PaneState, EditInteraction)>;

type ViewFn<State> = for<'a> fn(&'a State, &mut PaneState) -> View<'a, State>;

const DRAG_START_DISTANCE: f32 = 3.0;

pub struct PaneBuilder<State> {
    pub(crate) name: &'static str,
    view: ViewFn<State>,
    pub(crate) inner_size: Option<(u32, u32)>,
    pub(crate) initial_bounds: Option<(i32, i32, u32, u32)>,
    pub(crate) resizable: Option<bool>,
    pub(crate) title: Option<String>,
    pub(crate) window_level: Option<WindowLevel>,
    pub(crate) transparent: Option<bool>,
    background: Option<Color>,
    pub(crate) decorations: Option<bool>,
    pub(crate) initially_active: Option<bool>,
    pub(crate) skip_taskbar: Option<bool>,
    pub(crate) cursor_visible: Option<bool>,
    pub(crate) open_at_start: bool,
    on_frame: fn(&mut State, &mut PaneState) -> (),
    on_start: fn(&mut State, &mut PaneState) -> (),
    on_wake: fn(&mut State, &mut PaneState) -> (),
    on_exit: fn(&mut State, &mut PaneState) -> (),
    custom_fonts: Vec<FontEntry>,
}

impl<State> Clone for PaneBuilder<State> {
    fn clone(&self) -> Self {
        Self {
            name: self.name,
            view: self.view,
            inner_size: self.inner_size,
            initial_bounds: self.initial_bounds,
            resizable: self.resizable,
            title: self.title.clone(),
            window_level: self.window_level,
            transparent: self.transparent,
            background: self.background,
            decorations: self.decorations,
            initially_active: self.initially_active,
            skip_taskbar: self.skip_taskbar,
            cursor_visible: self.cursor_visible,
            open_at_start: self.open_at_start,
            on_frame: self.on_frame,
            on_start: self.on_start,
            on_wake: self.on_wake,
            on_exit: self.on_exit,
            custom_fonts: self.custom_fonts.clone(),
        }
    }
}

pub struct Pane<State> {
    name: &'static str,
    base_color: Color,
    view: ViewFn<State>,
    gestures: Vec<ActiveGesture<State>>,
    pressed_buttons: Vec<MouseButton>,
    pub(crate) elements: HashMap<u64, Area>,
    edit_handlers: HashMap<u64, EditHandler<State>>,
    hovered: HashSet<GestureId>,
    cursor_position: Option<Point>,
    gesture_state: GestureState,
    pub(crate) pane_state: PaneState,
    on_frame: fn(&mut State, &mut PaneState) -> (),
    on_start: fn(&mut State, &mut PaneState) -> (),
    on_wake: fn(&mut State, &mut PaneState) -> (),
    on_exit: fn(&mut State, &mut PaneState) -> (),
    started: bool,
}

struct ActiveGesture<State> {
    gesture: Gesture<State>,
    hit_rect: Area,
    local_area: Area,
}

#[derive(Debug, Clone, Copy)]
struct CapturedGesture {
    id: GestureId,
    local_area: Area,
    hit_rect: Area,
}

#[derive(Debug, Clone)]
struct GestureCapturer {
    clicks: Vec<CapturedGesture>,
    drags: Vec<CapturedGesture>,
}

#[derive(Debug, Clone)]
enum GestureState {
    None,
    Pressing {
        start: Point,
        capturer: GestureCapturer,
        button: MouseButton,
        click_started: bool,
    },
    Dragging {
        start: Point,
        last_position: Point,
        capturer: GestureCapturer,
        button: MouseButton,
    },
}

impl<State> Clone for ActiveGesture<State> {
    fn clone(&self) -> Self {
        Self {
            gesture: self.gesture.clone(),
            hit_rect: self.hit_rect,
            local_area: self.local_area,
        }
    }
}

impl<State> PaneBuilder<State> {
    pub fn new(name: &'static str, view: ViewFn<State>) -> Self {
        Self {
            name,
            view,
            inner_size: None,
            initial_bounds: None,
            resizable: None,
            title: None,
            window_level: None,
            transparent: None,
            background: None,
            decorations: None,
            initially_active: None,
            skip_taskbar: None,
            cursor_visible: None,
            open_at_start: true,
            on_frame: |_, _| {},
            on_start: |_, _| {},
            on_wake: |_, _| {},
            on_exit: |_, _| {},
            custom_fonts: Vec::new(),
        }
    }

    pub fn inner_size(mut self, width: u32, height: u32) -> Self {
        self.inner_size = Some((width, height));
        self
    }

    pub fn initial_bounds(mut self, x: i32, y: i32, width: u32, height: u32) -> Self {
        self.initial_bounds = Some((x, y, width, height));
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

    pub fn window_level(mut self, level: WindowLevel) -> Self {
        self.window_level = Some(level);
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

    pub fn initially_active(mut self, active: bool) -> Self {
        self.initially_active = Some(active);
        self
    }

    pub fn skip_taskbar(mut self, skip: bool) -> Self {
        self.skip_taskbar = Some(skip);
        self
    }

    pub fn cursor_visible(mut self, visible: bool) -> Self {
        self.cursor_visible = Some(visible);
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

    pub fn on_wake(mut self, on_wake: fn(&mut State, &mut PaneState) -> ()) -> Self {
        self.on_wake = on_wake;
        self
    }

    pub fn on_exit(mut self, on_exit: fn(&mut State, &mut PaneState) -> ()) -> Self {
        self.on_exit = on_exit;
        self
    }

    pub fn build(self) -> Pane<State>
    where
        State: 'static,
    {
        Pane::new(self)
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
        Brush,
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum WindowLevel {
    AlwaysOnBottom,
    #[default]
    Normal,
    AlwaysOnTop,
}

#[derive(Clone)]
pub struct PaneWaker {
    wake: Arc<dyn Fn() + Send + Sync>,
}

impl PaneWaker {
    pub fn wake(&self) {
        (self.wake)();
    }
}

impl std::fmt::Debug for PaneWaker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PaneWaker").finish()
    }
}

pub struct PaneState {
    pub(crate) text_layout: TextLayout,
    pub(crate) font_cx: FontContext,
    pub(crate) layout_cx: LayoutContext<Brush>,
    pub(crate) scale_factor: f64,
    text_editing: TextEditing,
    pub(crate) editor_areas: HashMap<u64, Area>,
    pub(crate) scrollers: HashMap<u64, crate::prebuilts::ScrollerState>,
    pub(crate) needs_redraw: bool,
    pub(crate) modifiers: Option<Modifiers>,
    pub(crate) effects: Vec<PaneEffect>,
    pub(crate) cursor_position: Option<Point>,
    wake: Arc<dyn Fn() + Send + Sync>,
}

pub type View<'a, State> = Layout<'a, PaneElement<State>, PaneState>;

pub struct PaneElement<State: ?Sized>(pub(crate) PaneElementKind<State>);

pub(crate) enum PaneElementKind<State: ?Sized> {
    Draw {
        view: Box<DrawableType>,
        area: Area,
        gestures: Vec<GestureAreaComponent<State>>,
    },
    EditorArea {
        id: u64,
        area: Area,
        edit_handler: Option<EditHandler<State>>,
    },
    Empty,
}

impl<State: ?Sized> PaneElement<State> {
    pub(crate) fn draw(view: Box<DrawableType>, area: Area) -> Self {
        Self(PaneElementKind::Draw {
            view,
            area,
            gestures: Vec::new(),
        })
    }

    pub(crate) fn editor_area(
        id: u64,
        area: Area,
        edit_handler: Option<EditHandler<State>>,
    ) -> Self {
        Self(PaneElementKind::EditorArea {
            id,
            area,
            edit_handler,
        })
    }

    pub(crate) fn empty() -> Self {
        Self(PaneElementKind::Empty)
    }

    pub(crate) fn into_kind(self) -> PaneElementKind<State> {
        self.0
    }
}

#[derive(Default)]
struct TextEditing {
    focused_field: Option<u64>,
    text_edit_command_revision: u64,
    applied_text_edit_command_revision: u64,
    lifecycle_events: Vec<(u64, EditInteraction)>,
}

impl TextEditing {
    fn focus_field(&mut self, field: Option<u64>) -> bool {
        if self.focused_field == field {
            return false;
        }
        if let Some(focused_field) = self.focused_field {
            self.lifecycle_events
                .push((focused_field, EditInteraction::End));
        }
        self.focused_field = field;
        if let Some(field) = field {
            self.lifecycle_events.push((field, EditInteraction::Start));
        }
        true
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
        self.text_editing.focus_field(None);
    }

    pub(crate) fn begin_editing(&mut self, id: u64) -> bool {
        self.text_editing.focus_field(Some(id))
    }

    pub(crate) fn next_text_edit_command_revision(&mut self) -> u64 {
        self.text_editing.text_edit_command_revision =
            self.text_editing.text_edit_command_revision.wrapping_add(1);
        if self.text_editing.text_edit_command_revision == 0 {
            self.text_editing.text_edit_command_revision = 1;
        }
        self.text_editing.text_edit_command_revision
    }

    pub(crate) fn apply_text_edit_command(&mut self, id: u64, command: Option<TextEditCommand>) {
        let Some(command) = command else {
            return;
        };
        let revision = match command {
            TextEditCommand::Focus(revision) | TextEditCommand::End(revision) => revision,
        };
        if revision != self.text_editing.text_edit_command_revision
            || revision == self.text_editing.applied_text_edit_command_revision
        {
            return;
        }
        match command {
            TextEditCommand::Focus(_) => {
                self.text_editing.focus_field(Some(id));
            }
            TextEditCommand::End(_) => {
                if self.text_editing.focused_field == Some(id) {
                    self.text_editing.focus_field(None);
                }
            }
        }
        self.text_editing.applied_text_edit_command_revision = revision;
    }

    pub(crate) fn text_field_is_focused(&self, id: u64) -> bool {
        self.text_editing.focused_field == Some(id)
    }

    pub fn redraw(&mut self) {
        self.effects.push(PaneEffect::Redraw);
    }

    pub fn waker(&self) -> PaneWaker {
        PaneWaker {
            wake: self.wake.clone(),
        }
    }

    pub(crate) fn request_redraw(&mut self) {
        self.waker().wake();
    }
}

impl<State: 'static> Pane<State> {
    fn new(config: PaneBuilder<State>) -> Self {
        let layout_cache = HashMap::new();
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

        let base_color = if config.transparent.unwrap_or(false) {
            Color::TRANSPARENT
        } else {
            config.background.unwrap_or(Color::BLACK)
        };

        Self {
            name: config.name,
            base_color,
            view: config.view,
            gestures: Vec::new(),
            pressed_buttons: Vec::new(),
            elements: HashMap::new(),
            edit_handlers: HashMap::new(),
            hovered: HashSet::new(),
            cursor_position: None,
            gesture_state: GestureState::None,
            pane_state: PaneState {
                text_layout: TextLayout::new(layout_cache),
                font_cx,
                layout_cx: LayoutContext::new(),
                scale_factor: 1.,
                text_editing: TextEditing::default(),
                editor_areas: HashMap::new(),
                scrollers: HashMap::new(),
                needs_redraw: false,
                modifiers: None,
                effects: Vec::new(),
                cursor_position: None,
                wake: Arc::new(|| {}),
            },
            on_frame: config.on_frame,
            on_start: config.on_start,
            on_wake: config.on_wake,
            on_exit: config.on_exit,
            started: false,
        }
    }

    pub(crate) fn name(&self) -> &'static str {
        self.name
    }

    pub(crate) fn set_wake_handler(&mut self, wake: Arc<dyn Fn() + Send + Sync>) {
        self.pane_state.wake = wake;
    }

    pub fn location(&self, id: u64) -> Option<Point> {
        let area = *self.elements.get(&id)?;
        Some(Point::new(
            area.x + area.width * 0.5,
            area.y + area.height * 0.5,
        ))
    }

    pub fn click(&mut self, state: &mut State, location: Point) -> Vec<PaneEffect> {
        let mut effects = self.move_to(state, location);
        effects.extend(self.press(state));
        effects.extend(self.release(state));
        effects
    }

    pub fn drag(&mut self, state: &mut State, from: Point, to: Point) -> Vec<PaneEffect> {
        let mut effects = self.move_to(state, from);
        effects.extend(self.press(state));
        effects.extend(self.move_to(state, to));
        effects.extend(self.release(state));
        effects
    }

    pub(crate) fn close(mut self, state: &mut State) {
        (self.on_exit)(state, &mut self.pane_state);
    }

    pub(crate) fn wake(&mut self, state: &mut State) -> Vec<PaneEffect> {
        (self.on_wake)(state, &mut self.pane_state);
        self.dispatch_text_edit_lifecycle_events(state);
        self.take_effects()
    }

    fn dispatch_text_edit_lifecycle_events(&mut self, state: &mut State) {
        loop {
            let edits = std::mem::take(&mut self.pane_state.text_editing.lifecycle_events);
            if edits.is_empty() {
                return;
            }
            for (id, edit) in edits {
                if let Some(handler) = self.edit_handlers.get(&id).cloned() {
                    handler(state, &mut self.pane_state, edit);
                }
            }
        }
    }

    fn take_effects(&mut self) -> Vec<PaneEffect> {
        std::mem::take(&mut self.pane_state.effects)
    }

    pub fn redraw(
        &mut self,
        state: &mut State,
        width: u32,
        height: u32,
        scale_factor: f64,
    ) -> (Frame, Vec<PaneEffect>) {
        if !self.started {
            self.started = true;
            (self.on_start)(state, &mut self.pane_state);
        }

        let updated_active_drag = if matches!(self.gesture_state, GestureState::Dragging { .. }) {
            if let Some(current) = self.cursor_position {
                let effects = self.move_to(state, current);
                self.pane_state.effects.extend(effects);
                true
            } else {
                false
            }
        } else {
            false
        };
        self.update_hover(state);

        self.gestures.clear();
        self.elements.clear();
        self.edit_handlers.clear();
        self.pane_state.scale_factor = scale_factor;

        let view = self.view;
        let pane_area = Area {
            x: 0.,
            y: 0.,
            width: ((width as f64) / self.pane_state.scale_factor) as f32,
            height: ((height as f64) / self.pane_state.scale_factor) as f32,
        };
        let draw_items = {
            let mut layout = view(state, &mut self.pane_state);
            layout.draw(pane_area, &mut self.pane_state)
        };

        let continue_animating =
            updated_active_drag || std::mem::take(&mut self.pane_state.needs_redraw);

        let mut items = Vec::new();
        let mut gesture_area_components = Vec::new();

        for item in draw_items {
            match item.into_kind() {
                PaneElementKind::EditorArea {
                    id,
                    area,
                    edit_handler,
                } => {
                    self.elements.insert(id, area);
                    self.pane_state.editor_areas.insert(id, area);
                    if let Some(edit_handler) = edit_handler {
                        self.edit_handlers.insert(id, edit_handler);
                    }
                }
                PaneElementKind::Draw {
                    view,
                    area,
                    gestures,
                } => {
                    let id = match &*view {
                        DrawableType::Text(view) => Some(view.id),
                        DrawableType::Layout(_) => None,
                        DrawableType::Path(view) => Some(view.id),
                        DrawableType::Svg(view) => Some(view.id),
                        DrawableType::Image(view) => Some(view.id),
                        DrawableType::Shadow(view) => Some(view.id),
                        DrawableType::PushLayer { .. } | DrawableType::PopLayer => None,
                    };
                    let draw_area = area;
                    if let Some(id) = id {
                        self.elements.insert(id, draw_area);
                    }
                    gesture_area_components
                        .extend(gestures.into_iter().map(|gesture| (draw_area, gesture)));

                    let render_item = match *view {
                        DrawableType::Text(text) => text.render_item(
                            self.pane_state.scale_factor,
                            draw_area,
                            &mut self.pane_state,
                        ),
                        DrawableType::Layout(boxed) => {
                            let (layout, transform) = *boxed;
                            RenderItem::Layout { layout, transform }
                        }
                        DrawableType::Path(path) => RenderItem::Path {
                            path,
                            area: draw_area,
                        },
                        DrawableType::Svg(svg) => RenderItem::Svg {
                            svg,
                            area: draw_area,
                        },
                        DrawableType::Image(image) => RenderItem::Image {
                            image,
                            area: draw_area,
                        },
                        DrawableType::Shadow(shadow) => RenderItem::Shadow {
                            shadow,
                            area: draw_area,
                        },
                        DrawableType::PushLayer { path, blend, alpha } => {
                            RenderItem::PushLayer { path, blend, alpha }
                        }
                        DrawableType::PopLayer => RenderItem::PopLayer,
                    };
                    items.push(render_item);
                }
                PaneElementKind::Empty => (),
            }
        }
        let mut seen_gestures = HashSet::new();
        for (area, component) in gesture_area_components {
            let rect = component.rect.unwrap_or(area);
            let Some(rect) = valid_area(rect) else {
                continue;
            };
            let gesture = component.gesture;
            if gesture.handler().positive_by_default && seen_gestures.insert(gesture.id()) {
                self.gestures.push(ActiveGesture {
                    gesture: gesture.clone(),
                    hit_rect: pane_area,
                    local_area: pane_area,
                });
            }
            let id = gesture.id();
            match component.operation {
                GestureAreaOperation::Include => {
                    self.gestures.push(ActiveGesture {
                        gesture,
                        hit_rect: rect,
                        local_area: area,
                    });
                }
                GestureAreaOperation::Occlude => {
                    self.gestures = std::mem::take(&mut self.gestures)
                        .into_iter()
                        .flat_map(|active| {
                            if active.gesture.id() == id {
                                subtract(active.hit_rect, rect)
                                    .into_iter()
                                    .map(|hit_rect| ActiveGesture {
                                        gesture: active.gesture.clone(),
                                        hit_rect,
                                        local_area: active.local_area,
                                    })
                                    .collect()
                            } else {
                                vec![active]
                            }
                        })
                        .collect();
                }
            }
        }

        if self
            .pane_state
            .text_editing
            .focused_field
            .is_some_and(|id| !self.elements.contains_key(&id))
        {
            self.pane_state.end_editing();
        }

        let frame = Frame {
            base_color: self.base_color,
            width,
            height,
            scale_factor: self.pane_state.scale_factor,
            items,
        };

        (self.on_frame)(state, &mut self.pane_state);

        if continue_animating {
            self.pane_state.request_redraw();
        }
        self.dispatch_text_edit_lifecycle_events(state);
        (frame, self.take_effects())
    }
}

impl<State: 'static> Pane<State> {
    fn hit_area(&self, active: &ActiveGesture<State>, position: Point) -> Option<Area> {
        if !active
            .gesture
            .handler()
            .modifiers
            .matches(self.pane_state.modifiers.unwrap_or_default())
        {
            return None;
        }
        contains(active.hit_rect, position).then_some(active.local_area)
    }

    fn pointer_gestures_at(
        &self,
        position: Point,
        predicate: impl Fn(&GestureKind) -> bool,
    ) -> Vec<(ActiveGesture<State>, Area)> {
        let mut matched = Vec::new();
        let mut seen = HashSet::new();
        for active in self.gestures.iter().rev() {
            let id = active.gesture.id();
            if seen.contains(&id) {
                continue;
            }
            if !predicate(&active.gesture.handler().kind) {
                continue;
            }
            let Some(area) = self.hit_area(active, position) else {
                continue;
            };
            seen.insert(id);
            let stops_propagation =
                active.gesture.handler().propagation == GesturePropagation::Stop;
            matched.push((active.clone(), area));
            if stops_propagation {
                break;
            }
        }
        matched.reverse();
        matched
    }

    fn point_in_area(area: Area, point: Point) -> Point {
        Point {
            x: point.x - area.x,
            y: point.y - area.y,
        }
    }

    fn update_hover(&mut self, state: &mut State) -> bool {
        let Some(pos) = self.cursor_position else {
            return false;
        };
        let mut needs_redraw = false;
        let hovered_ids: HashSet<GestureId> = self
            .pointer_gestures_at(pos, |kind| matches!(kind, GestureKind::Hover))
            .into_iter()
            .map(|(active, _)| active.gesture.id())
            .collect();
        let mut hoverable_ids = HashSet::new();
        for active in self.gestures.iter() {
            if !matches!(active.gesture.handler().kind, GestureKind::Hover) {
                continue;
            }
            let id = active.gesture.id();
            if hoverable_ids.contains(&id) {
                continue;
            }
            hoverable_ids.insert(id);
            let hovered = hovered_ids.contains(&id);
            if self.hovered.contains(&id) == hovered {
                continue;
            }
            needs_redraw = true;
            if hovered {
                self.hovered.insert(id);
            } else {
                self.hovered.remove(&id);
            }
            (active.gesture.handler().interaction_handler)(
                state,
                &mut self.pane_state,
                Interaction::Hover(hovered),
            );
        }
        self.hovered.retain(|id| hoverable_ids.contains(id));
        needs_redraw
    }

    pub fn key_pressed(&mut self, state: &mut State, key: impl Into<Key>) -> Vec<PaneEffect> {
        self.dispatch_key(state, key.into(), KeyPhase::Pressed)
    }

    pub fn key_released(&mut self, state: &mut State, key: impl Into<Key>) -> Vec<PaneEffect> {
        self.dispatch_key(state, key.into(), KeyPhase::Released)
    }

    fn dispatch_key(
        &mut self,
        state: &mut State,
        key: Key,
        key_state: KeyPhase,
    ) -> Vec<PaneEffect> {
        let mut needs_redraw = false;
        let mut seen = HashSet::new();
        for active in self.gestures.iter().rev() {
            let id = active.gesture.id();
            if seen.contains(&id) {
                continue;
            }
            let GestureKind::Key { keys } = &active.gesture.handler().kind else {
                continue;
            };
            if !keys.matches(&key)
                || !active
                    .gesture
                    .handler()
                    .modifiers
                    .matches(self.pane_state.modifiers.unwrap_or_default())
            {
                continue;
            }
            seen.insert(id);
            needs_redraw = true;
            let stops_propagation =
                active.gesture.handler().propagation == GesturePropagation::Stop;
            (active.gesture.handler().interaction_handler)(
                state,
                &mut self.pane_state,
                Interaction::Key(key.clone(), key_state),
            );
            if stops_propagation {
                break;
            }
        }
        if needs_redraw {
            self.pane_state.request_redraw();
        }
        self.dispatch_text_edit_lifecycle_events(state);
        self.take_effects()
    }

    pub(crate) fn modifiers_changed(&mut self, modifiers: Modifiers) -> Vec<PaneEffect> {
        self.pane_state.modifiers = Some(modifiers);
        self.take_effects()
    }

    pub(crate) fn scale_factor_changed(&mut self, scale_factor: f64) -> Vec<PaneEffect> {
        self.pane_state.scale_factor = scale_factor;
        self.pane_state.text_layout.layout_cache.clear();
        self.pane_state.request_redraw();
        self.take_effects()
    }

    pub(crate) fn exit(&mut self, state: &mut State) -> Vec<PaneEffect> {
        self.cursor_position = None;
        let mut needs_redraw = false;
        let mut seen = HashSet::new();
        for active in self.gestures.iter() {
            let id = active.gesture.id();
            if seen.contains(&id) {
                continue;
            }
            seen.insert(id);
            if matches!(active.gesture.handler().kind, GestureKind::Hover) {
                needs_redraw = true;
                (active.gesture.handler().interaction_handler)(
                    state,
                    &mut self.pane_state,
                    Interaction::Hover(false),
                );
            }
        }
        self.hovered.clear();
        if needs_redraw {
            self.pane_state.request_redraw();
        }
        self.dispatch_text_edit_lifecycle_events(state);
        self.take_effects()
    }

    pub fn move_to(&mut self, state: &mut State, pos: Point) -> Vec<PaneEffect> {
        self.cursor_position = Some(pos);
        self.pane_state.cursor_position = Some(pos);
        let gesture_state = self.gesture_state.clone();
        match gesture_state {
            GestureState::Pressing {
                start,
                capturer,
                button,
                click_started,
                ..
            } => {
                let distance = start.distance(pos);
                let can_drag = !capturer.drags.is_empty();
                if can_drag && distance >= DRAG_START_DISTANCE {
                    let delta = Point {
                        x: pos.x - start.x,
                        y: pos.y - start.y,
                    };
                    if click_started {
                        for captured in &capturer.clicks {
                            let Some(active) = self
                                .gestures
                                .iter()
                                .find(|active| active.gesture.id() == captured.id)
                                .cloned()
                            else {
                                continue;
                            };
                            if !matches!(active.gesture.handler().kind, GestureKind::Click { .. }) {
                                continue;
                            }
                            (active.gesture.handler().interaction_handler)(
                                state,
                                &mut self.pane_state,
                                Interaction::Click(ClickEvent {
                                    state: ClickPhase::Cancelled,
                                    button,
                                    location: ClickLocation::new(pos, captured.local_area),
                                }),
                            );
                        }
                    }
                    for captured in &capturer.drags {
                        let Some(active) = self
                            .gestures
                            .iter()
                            .find(|active| active.gesture.id() == captured.id)
                            .cloned()
                        else {
                            continue;
                        };
                        if !matches!(active.gesture.handler().kind, GestureKind::Drag { .. }) {
                            continue;
                        }
                        (active.gesture.handler().interaction_handler)(
                            state,
                            &mut self.pane_state,
                            Interaction::Drag(DragPhase::Began {
                                start: Self::point_in_area(captured.local_area, start),
                                start_global: start,
                            }),
                        );
                        (active.gesture.handler().interaction_handler)(
                            state,
                            &mut self.pane_state,
                            Interaction::Drag(DragPhase::Updated {
                                start: Self::point_in_area(captured.local_area, start),
                                current: Self::point_in_area(captured.local_area, pos),
                                start_global: start,
                                current_global: pos,
                                delta,
                                distance,
                            }),
                        );
                    }
                    self.gesture_state = GestureState::Dragging {
                        start,
                        last_position: pos,
                        capturer,
                        button,
                    };
                } else {
                    self.gesture_state = GestureState::Pressing {
                        start,
                        capturer,
                        button,
                        click_started,
                    };
                }
            }
            GestureState::Dragging {
                start,
                last_position,
                capturer,
                button,
            } => {
                let distance = start.distance(pos);
                let delta = Point {
                    x: pos.x - last_position.x,
                    y: pos.y - last_position.y,
                };
                for captured in &capturer.drags {
                    let Some(active) = self
                        .gestures
                        .iter()
                        .find(|active| active.gesture.id() == captured.id)
                        .cloned()
                    else {
                        continue;
                    };
                    if !matches!(active.gesture.handler().kind, GestureKind::Drag { .. }) {
                        continue;
                    }
                    (active.gesture.handler().interaction_handler)(
                        state,
                        &mut self.pane_state,
                        Interaction::Drag(DragPhase::Updated {
                            start: Self::point_in_area(captured.local_area, start),
                            current: Self::point_in_area(captured.local_area, pos),
                            start_global: start,
                            current_global: pos,
                            delta,
                            distance,
                        }),
                    );
                }
                self.gesture_state = GestureState::Dragging {
                    start,
                    last_position: pos,
                    capturer,
                    button,
                };
            }
            GestureState::None => {}
        }
        self.dispatch_text_edit_lifecycle_events(state);
        self.take_effects()
    }

    pub(crate) fn press(&mut self, state: &mut State) -> Vec<PaneEffect> {
        self.press_button(state, MouseButton::Left)
    }

    pub fn press_button(&mut self, state: &mut State, button: MouseButton) -> Vec<PaneEffect> {
        let mut needs_redraw = false;
        if !self.pressed_buttons.contains(&button) {
            self.pressed_buttons.push(button);
        }
        let cursor_position = self.cursor_position;
        if let Some(location) = cursor_position {
            let click_matches = self.pointer_gestures_at(location, |kind| {
                matches!(kind, GestureKind::Click { buttons } if buttons.matches(&self.pressed_buttons))
            });
            let drag_matches = self.pointer_gestures_at(location, |kind| {
                matches!(kind, GestureKind::Drag { button } if button.matches(&self.pressed_buttons))
            });
            if !click_matches.is_empty() || !drag_matches.is_empty() {
                needs_redraw = true;
                let click_started = !click_matches.is_empty();
                for (active, area) in &click_matches {
                    (active.gesture.handler().interaction_handler)(
                        state,
                        &mut self.pane_state,
                        Interaction::Click(ClickEvent {
                            state: ClickPhase::Started,
                            button,
                            location: ClickLocation::new(location, *area),
                        }),
                    );
                }
                self.gesture_state = GestureState::Pressing {
                    start: location,
                    capturer: GestureCapturer {
                        clicks: click_matches
                            .into_iter()
                            .map(|(active, area)| CapturedGesture {
                                id: active.gesture.id(),
                                local_area: area,
                                hit_rect: active.hit_rect,
                            })
                            .collect(),
                        drags: drag_matches
                            .into_iter()
                            .map(|(active, area)| CapturedGesture {
                                id: active.gesture.id(),
                                local_area: area,
                                hit_rect: active.hit_rect,
                            })
                            .collect(),
                    },
                    button,
                    click_started,
                };
            }
        }

        if needs_redraw {
            self.pane_state.request_redraw();
        }
        self.dispatch_text_edit_lifecycle_events(state);
        self.take_effects()
    }

    pub(crate) fn release(&mut self, state: &mut State) -> Vec<PaneEffect> {
        self.release_button(state, MouseButton::Left)
    }

    pub fn release_button(&mut self, state: &mut State, button: MouseButton) -> Vec<PaneEffect> {
        let mut needs_redraw = false;
        let cursor_position = self.cursor_position;
        let gesture_state = self.gesture_state.clone();
        if let Some(current) = cursor_position {
            match gesture_state {
                GestureState::Pressing {
                    capturer,
                    button: press_button,
                    click_started,
                    ..
                } => {
                    if button != press_button {
                        self.pressed_buttons.retain(|pressed| *pressed != button);
                        self.dispatch_text_edit_lifecycle_events(state);
                        return self.take_effects();
                    }
                    if click_started {
                        for captured in &capturer.clicks {
                            let Some(active) = self
                                .gestures
                                .iter()
                                .find(|active| active.gesture.id() == captured.id)
                                .cloned()
                            else {
                                continue;
                            };
                            if !matches!(active.gesture.handler().kind, GestureKind::Click { .. }) {
                                continue;
                            }
                            let phase = if contains(captured.hit_rect, current) {
                                ClickPhase::Completed
                            } else {
                                ClickPhase::Cancelled
                            };
                            needs_redraw = true;
                            (active.gesture.handler().interaction_handler)(
                                state,
                                &mut self.pane_state,
                                Interaction::Click(ClickEvent {
                                    state: phase,
                                    button: press_button,
                                    location: ClickLocation::new(current, captured.local_area),
                                }),
                            );
                        }
                    }
                }
                GestureState::Dragging {
                    start,
                    last_position,
                    capturer,
                    button: press_button,
                } => {
                    if button != press_button {
                        self.pressed_buttons.retain(|pressed| *pressed != button);
                        self.dispatch_text_edit_lifecycle_events(state);
                        return self.take_effects();
                    }
                    let distance = start.distance(current);
                    let delta = Point {
                        x: current.x - last_position.x,
                        y: current.y - last_position.y,
                    };
                    for captured in &capturer.drags {
                        let Some(active) = self
                            .gestures
                            .iter()
                            .find(|active| active.gesture.id() == captured.id)
                            .cloned()
                        else {
                            continue;
                        };
                        if !matches!(active.gesture.handler().kind, GestureKind::Drag { .. }) {
                            continue;
                        }
                        needs_redraw = true;
                        (active.gesture.handler().interaction_handler)(
                            state,
                            &mut self.pane_state,
                            Interaction::Drag(DragPhase::Completed {
                                start: Self::point_in_area(captured.local_area, start),
                                current: Self::point_in_area(captured.local_area, current),
                                start_global: start,
                                current_global: current,
                                delta,
                                distance,
                            }),
                        );
                    }
                }
                GestureState::None => {}
            }
        }
        self.pressed_buttons.retain(|pressed| *pressed != button);
        self.gesture_state = GestureState::None;
        if needs_redraw {
            self.pane_state.request_redraw();
        }
        self.dispatch_text_edit_lifecycle_events(state);
        self.take_effects()
    }

    pub(crate) fn scroll(&mut self, state: &mut State, delta: ScrollDelta) -> Vec<PaneEffect> {
        let mut needs_redraw = false;
        let cursor_position = self.cursor_position;
        if let Some(current) = cursor_position {
            let mut remaining_x = delta.x != 0.;
            let mut remaining_y = delta.y != 0.;
            let mut matches = Vec::new();
            let mut seen = HashSet::new();
            for active in self.gestures.iter().rev() {
                let id = active.gesture.id();
                if seen.contains(&id) {
                    continue;
                }
                let GestureKind::Scroll { axes } = &active.gesture.handler().kind else {
                    continue;
                };
                if (!remaining_x || !axes.x) && (!remaining_y || !axes.y) {
                    continue;
                }
                if self.hit_area(active, current).is_none() {
                    continue;
                }
                seen.insert(id);
                matches.push((
                    active.clone(),
                    ScrollDelta {
                        x: if remaining_x && axes.x { delta.x } else { 0. },
                        y: if remaining_y && axes.y { delta.y } else { 0. },
                    },
                ));
                if active.gesture.handler().propagation == GesturePropagation::Stop {
                    if axes.x {
                        remaining_x = false;
                    }
                    if axes.y {
                        remaining_y = false;
                    }
                    if !remaining_x && !remaining_y {
                        break;
                    }
                }
            }
            if !matches.is_empty() {
                needs_redraw = true;
                matches.reverse();
                for (active, delta) in matches {
                    (active.gesture.handler().interaction_handler)(
                        state,
                        &mut self.pane_state,
                        Interaction::Scroll(delta),
                    );
                }
            }
        }
        if needs_redraw {
            self.pane_state.request_redraw();
        }
        self.dispatch_text_edit_lifecycle_events(state);
        self.take_effects()
    }
}
