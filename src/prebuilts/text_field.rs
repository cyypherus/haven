use crate::brush_source::BrushSource;
use crate::editor::Editor;
use crate::pane::{PaneState, View};
use crate::primitives::shape::{PathData, rect_path};
use crate::view::DrawableType;
use crate::view::rect_path as clip_rect_path;
use crate::{
    Binding, ClickPhase, DEFAULT_CORNER_ROUNDING, DEFAULT_FG_COLOR, DEFAULT_FONT_FAMILY,
    DEFAULT_FONT_SIZE, DEFAULT_PADDING, DEFAULT_PURP, DEFAULT_STROKE_WIDTH, DragPhase,
    EditInteraction, Key, KeyPhase, MouseButton, NamedKey, gesture, rect,
};
use backer::{Area, Layout, nodes::*};
use kurbo::{Affine, Rect as KRect, Stroke};
use parley::{Alignment, FontWeight, LineHeight, StyleProperty};
use peniko::color::palette::css::TRANSPARENT;
use peniko::{Brush, Color};
use std::fmt::Debug;
use std::rc::Rc;
use std::time::Instant;

use super::scroll_feedback::{ScrollEdge, ScrollEdgeFeedback, scroll_edge_glows};

#[derive(Clone)]
pub struct TextState {
    pub text: String,
    pub(crate) editor: Editor,
    pub(crate) edit_request: TextEditRequest,
    pub(crate) viewport: TextFieldViewport,
    pub(crate) edge_feedback: ScrollEdgeFeedback,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TextEditRequest {
    editing: Option<bool>,
    token: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InitialSelection {
    Start,
    End,
    All,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct TextFieldViewport {
    pub(crate) x: f32,
    pub(crate) y: f32,
}

impl Debug for TextState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextState")
            .field("text", &self.text)
            .finish()
    }
}

impl Default for TextState {
    fn default() -> Self {
        Self::new("")
    }
}

impl TextState {
    pub fn new(text: impl AsRef<str>) -> Self {
        let text = text.as_ref().to_string();
        Self {
            editor: Editor::new(&text),
            edit_request: TextEditRequest::default(),
            text,
            viewport: TextFieldViewport::default(),
            edge_feedback: ScrollEdgeFeedback::default(),
        }
    }

    pub fn copy_text(&self) -> Option<String> {
        use clipboard_rs::{Clipboard, ClipboardContext};

        let text = self.editor.editor.selected_text()?.to_owned();
        let cb = ClipboardContext::new().unwrap();
        cb.set_text(text.clone()).ok();
        Some(text)
    }

    pub fn cut_text(&mut self, app: &mut PaneState) -> Option<String> {
        let text = self.copy_text()?;
        self.editor
            .editor
            .driver(&mut app.font_cx, &mut app.layout_cx)
            .delete_selection();
        self.text = self.editor.text().to_string();
        app.request_redraw();
        Some(text)
    }

    pub fn paste_text(&mut self, app: &mut PaneState) {
        use clipboard_rs::{Clipboard, ClipboardContext};

        let cb = ClipboardContext::new().unwrap();
        let text = cb.get_text().unwrap_or_default();
        self.editor
            .editor
            .driver(&mut app.font_cx, &mut app.layout_cx)
            .insert_or_replace_selection(&text);
        self.text = self.editor.text().to_string();
        app.request_redraw();
    }

    pub fn select_all_text(&mut self, app: &mut PaneState) {
        self.apply_initial_focus(InitialSelection::All, app);
        app.request_redraw();
    }

    pub fn begin_editing(&mut self, app: &mut PaneState) {
        self.begin_editing_with(app, InitialSelection::End);
    }

    pub fn begin_editing_with(&mut self, app: &mut PaneState, target: InitialSelection) {
        self.apply_initial_focus(target, app);
        self.editor
            .focus_without_pointer_selection(&mut app.layout_cx, &mut app.font_cx);
        self.request_editing(true, app);
    }

    pub fn end_editing(&mut self, app: &mut PaneState) {
        self.request_editing(false, app);
    }

    fn request_editing(&mut self, editing: bool, app: &mut PaneState) {
        self.edit_request = TextEditRequest {
            editing: Some(editing),
            token: self.edit_request.token.wrapping_add(1),
        };
        app.request_redraw();
    }

    fn apply_initial_focus(&mut self, target: InitialSelection, app: &mut PaneState) {
        let mut driver = self
            .editor
            .editor
            .driver(&mut app.font_cx, &mut app.layout_cx);
        match target {
            InitialSelection::Start => driver.move_to_text_start(),
            InitialSelection::End => driver.move_to_text_end(),
            InitialSelection::All => driver.select_all(),
        }
    }
}

fn style_text_field_editor(
    editor: &mut Editor,
    text: &str,
    fill: Brush,
    font_family: String,
    font_weight: FontWeight,
    line_height: f32,
    font_size: f32,
    alignment: Alignment,
    width: Option<f32>,
) {
    editor.editor.set_text(text);
    let styles = editor.editor.edit_styles();

    styles.insert(parley::StyleProperty::Brush(fill));
    styles.insert(parley::FontFamily::Named(font_family.into()).into());
    styles.insert(StyleProperty::FontWeight(font_weight));
    styles.insert(StyleProperty::LineHeight(LineHeight::FontSizeRelative(
        line_height,
    )));
    styles.insert(StyleProperty::FontSize(font_size));
    styles.insert(StyleProperty::OverflowWrap(parley::OverflowWrap::Anywhere));

    editor.editor.set_alignment(alignment);
    editor.editor.set_width(width);
}

struct TextFieldEditStyle<'a> {
    fill: &'a BrushSource<TextState>,
    font_family: &'a Option<String>,
    font_weight: FontWeight,
    line_height: f32,
    font_size: f32,
    alignment: Alignment,
    wrap: bool,
}

fn style_bound_text_field_editor<State>(
    state: &mut State,
    binding: &Binding<State, TextState>,
    area: Area,
    style: TextFieldEditStyle<'_>,
) {
    let text_fill = style.fill.resolve(area, binding.get(state));
    let ts = binding.get_mut(state);
    style_text_field_editor(
        &mut ts.editor,
        &ts.text,
        text_fill,
        style
            .font_family
            .clone()
            .unwrap_or_else(|| DEFAULT_FONT_FAMILY.to_string()),
        style.font_weight,
        style.line_height,
        style.font_size,
        style.alignment,
        if style.wrap { Some(area.width) } else { None },
    );
}

fn text_field_editor_area(app: &PaneState, root_id: u64) -> Area {
    app.editor_areas.get(&root_id).copied().unwrap_or(Area {
        x: 0.,
        y: 0.,
        width: 0.,
        height: 0.,
    })
}

fn text_field_local_point(
    editor: &mut Editor,
    area: Area,
    viewport: TextFieldViewport,
    point: crate::Point,
    alignment: Alignment,
    app: &mut PaneState,
) -> crate::Point {
    let cursor_width = 1.5;
    let layout = editor
        .editor
        .layout(&mut app.font_cx, &mut app.layout_cx)
        .clone();
    let cursor = text_field_cursor(editor, cursor_width);
    let (scroll_width, scroll_height) = text_field_scroll_size(&layout, cursor);
    let placement = text_field_placement(
        area,
        viewport,
        layout.height(),
        scroll_width,
        scroll_height,
        alignment,
    );
    crate::Point::new(
        point.x - placement.origin_x as f64,
        point.y - placement.origin_y as f64,
    )
}

fn text_field_cursor(editor: &mut Editor, cursor_width: f32) -> Option<KRect> {
    editor
        .editor
        .cursor_geometry(cursor_width)
        .map(|bb| KRect::new(bb.x0, bb.y0, bb.x1, bb.y1))
}

fn text_field_scroll_size(layout: &parley::Layout<Brush>, cursor: Option<KRect>) -> (f32, f32) {
    let mut width = layout.full_width();
    let mut height = layout.height();
    if let Some(cursor) = cursor {
        width = width.max(cursor.x1 as f32);
        height = height.max(cursor.y1 as f32);
    }
    (width, height)
}

fn bound_text_field_scroll_size(
    editor: &mut Editor,
    app: &mut PaneState,
    cursor_width: f32,
) -> (f32, f32) {
    let layout = editor
        .editor
        .layout(&mut app.font_cx, &mut app.layout_cx)
        .clone();
    let cursor = text_field_cursor(editor, cursor_width);
    text_field_scroll_size(&layout, cursor)
}

fn begin_text_field_editing(app: &mut PaneState, root_id: u64) -> bool {
    let started_editing = app.editor.as_ref().map(|e| e.id) != Some(root_id);
    if started_editing {
        app.begin_editing(root_id);
        app.request_redraw();
    }
    started_editing
}

fn clamp_text_field_viewport(
    viewport: TextFieldViewport,
    area: Area,
    content_width: f32,
    content_height: f32,
) -> TextFieldViewport {
    TextFieldViewport {
        x: viewport.x.clamp(0., (content_width - area.width).max(0.)),
        y: viewport.y.clamp(0., (content_height - area.height).max(0.)),
    }
}

#[derive(Clone, Copy, Debug)]
struct TextFieldPlacement {
    origin_x: f32,
    origin_y: f32,
    overflow_x: bool,
    overflow_y: bool,
}

impl TextFieldPlacement {
    fn needs_clip(&self) -> bool {
        self.overflow_x || self.overflow_y
    }
}

fn text_field_placement(
    area: Area,
    viewport: TextFieldViewport,
    layout_height: f32,
    scroll_width: f32,
    scroll_height: f32,
    alignment: Alignment,
) -> TextFieldPlacement {
    let overflow_x = scroll_width > area.width;
    let overflow_y = scroll_height > area.height;
    let viewport = clamp_text_field_viewport(viewport, area, scroll_width, scroll_height);
    let origin_x = if overflow_x {
        area.x - viewport.x
    } else {
        area.x + text_field_alignment_offset(area.width, scroll_width, alignment)
    };
    let origin_y = if overflow_y {
        area.y - viewport.y
    } else {
        area.y + ((area.height - layout_height) * 0.5).max(0.)
    };
    TextFieldPlacement {
        origin_x,
        origin_y,
        overflow_x,
        overflow_y,
    }
}

fn text_field_alignment_offset(area_width: f32, content_width: f32, alignment: Alignment) -> f32 {
    let extra = (area_width - content_width).max(0.);
    match alignment {
        Alignment::Center => extra * 0.5,
        Alignment::End | Alignment::Right => extra,
        _ => 0.,
    }
}

fn update_text_field_viewport(
    state: &mut TextState,
    requested: TextFieldViewport,
    area: Area,
    content_width: f32,
    content_height: f32,
    app: &mut PaneState,
) {
    let clamped = clamp_text_field_viewport(requested, area, content_width, content_height);
    let mut edges = Vec::new();
    if content_width > area.width {
        if requested.x < clamped.x {
            edges.push(ScrollEdge::Left);
        } else if requested.x > clamped.x {
            edges.push(ScrollEdge::Right);
        }
    }
    if content_height > area.height {
        if requested.y < clamped.y {
            edges.push(ScrollEdge::Top);
        } else if requested.y > clamped.y {
            edges.push(ScrollEdge::Bottom);
        }
    }
    if !edges.is_empty() {
        state.edge_feedback.pulse_all(&edges, Instant::now());
    }
    if state.viewport != clamped || !edges.is_empty() {
        app.request_redraw();
    }
    state.viewport = clamped;
}

fn cursor_visible_text_field_viewport(
    viewport: TextFieldViewport,
    area: Area,
    content_width: f32,
    content_height: f32,
    cursor: Option<KRect>,
) -> TextFieldViewport {
    let mut visible = viewport;
    let mut content_width = content_width;
    let mut content_height = content_height;
    if let Some(cursor) = cursor {
        content_width = content_width.max(cursor.x1 as f32);
        content_height = content_height.max(cursor.y1 as f32);

        visible.x = cursor_visible_axis_viewport(
            viewport.x,
            area.width,
            cursor.x0 as f32,
            cursor.x1 as f32,
        );
        visible.y = cursor_visible_axis_viewport(
            viewport.y,
            area.height,
            cursor.y0 as f32,
            cursor.y1 as f32,
        );
    }
    clamp_text_field_viewport(visible, area, content_width, content_height)
}

fn cursor_visible_axis_viewport(
    viewport: f32,
    area_size: f32,
    cursor_start: f32,
    cursor_end: f32,
) -> f32 {
    if let Some(viewport) = clamp_viewport_to_cursor(
        viewport,
        area_size,
        cursor_start,
        cursor_end,
        area_size / 3.,
    ) {
        return viewport;
    }
    if let Some(viewport) =
        clamp_viewport_to_cursor(viewport, area_size, cursor_start, cursor_end, 0.)
    {
        return viewport;
    }
    viewport.clamp(cursor_start - area_size, cursor_start)
}

fn clamp_viewport_to_cursor(
    viewport: f32,
    area_size: f32,
    cursor_start: f32,
    cursor_end: f32,
    inset: f32,
) -> Option<f32> {
    let min = cursor_end - area_size + inset;
    let max = cursor_start - inset;
    (min <= max).then(|| viewport.clamp(min, max))
}

fn show_text_field_cursor_after_edit(state: &mut TextState, area: Area, app: &mut PaneState) {
    let layout = state
        .editor
        .editor
        .layout(&mut app.font_cx, &mut app.layout_cx)
        .clone();
    let cursor = text_field_cursor(&mut state.editor, 1.5);
    let (content_width, content_height) = text_field_scroll_size(&layout, cursor);
    state.viewport = cursor_visible_text_field_viewport(
        state.viewport,
        area,
        content_width,
        content_height,
        cursor,
    );
}

fn scroll_text_field_viewport_for_drag(
    state: &mut TextState,
    area: Area,
    point: crate::Point,
    app: &mut PaneState,
) {
    let mut viewport = state.viewport;
    let right = area.x + area.width;
    let bottom = area.y + area.height;
    if point.x < area.x as f64 {
        viewport.x -= area.x - point.x as f32;
    } else if point.x > right as f64 {
        viewport.x += point.x as f32 - right;
    }
    if point.y < area.y as f64 {
        viewport.y -= area.y - point.y as f32;
    } else if point.y > bottom as f64 {
        viewport.y += point.y as f32 - bottom;
    }

    let (content_width, content_height) = bound_text_field_scroll_size(&mut state.editor, app, 1.5);
    update_text_field_viewport(state, viewport, area, content_width, content_height, app);
}

fn text_field_layout_views<State: 'static>(
    area: Area,
    layout: parley::Layout<Brush>,
    scale_factor: f64,
    placement: TextFieldPlacement,
    edge_feedback: &ScrollEdgeFeedback,
    ctx: &mut PaneState,
    before: impl IntoIterator<Item = View<State>>,
    after: impl IntoIterator<Item = View<State>>,
) -> Vec<View<State>> {
    let now = Instant::now();
    let needs_clip = placement.needs_clip() || edge_feedback.is_animating(now);
    let mut views = Vec::new();
    if needs_clip {
        views.push(View::<State>::draw(
            Box::new(DrawableType::PushLayer {
                path: clip_rect_path(area),
                blend: peniko::BlendMode::default(),
                alpha: 1.,
            }),
            area,
        ));
    }
    views.extend(before);
    let transform = Affine::translate((placement.origin_x as f64, placement.origin_y as f64))
        .then_scale(scale_factor);
    views.push(View::<State>::draw(
        Box::new(DrawableType::Layout(Box::new((layout, transform)))),
        area,
    ));
    views.extend(after);
    if needs_clip {
        views.extend(scroll_edge_glows::<State>(ctx, edge_feedback, now).draw(area, ctx));
        views.push(View::<State>::draw(Box::new(DrawableType::PopLayer), area));
    }
    views
}

pub fn text_field<'a, State>(
    id: u64,
    state: (&'a TextState, Binding<State, TextState>),
) -> TextField<'a, State> {
    TextField {
        id,
        state: state.0,
        binding: state.1,
        font_size: DEFAULT_FONT_SIZE,
        font_weight: FontWeight::NORMAL,
        font_family: None,
        text_fill: BrushSource::Static(Brush::Solid(DEFAULT_FG_COLOR)),
        alignment: Alignment::Center,
        editable: true,
        line_height: 1.,
        background: None,
        padding: DEFAULT_PADDING,
        wrap: false,
        cursor_fill: BrushSource::Static(Brush::Solid(DEFAULT_FG_COLOR)),
        highlight_fill: BrushSource::Static(Brush::Solid(DEFAULT_PURP)),
        hint_text: None,
        hint_fill: BrushSource::Static(Brush::Solid(DEFAULT_FG_COLOR.with_alpha(0.55))),
        on_edit: None,
        esc_end_editing: false,
        enter_end_editing: false,
    }
}

type BgViewFn<'a, State> =
    Rc<dyn Fn(&TextState, Area, &mut PaneState) -> Layout<'a, View<State>, PaneState> + 'a>;

pub struct TextField<'a, State> {
    pub(crate) id: u64,
    pub(crate) state: &'a TextState,
    pub(crate) binding: Binding<State, TextState>,
    pub(crate) text_fill: BrushSource<TextState>,
    pub(crate) font_size: u32,
    pub(crate) font_weight: FontWeight,
    pub(crate) font_family: Option<String>,
    pub(crate) alignment: Alignment,
    pub(crate) editable: bool,
    pub(crate) line_height: f32,
    pub(crate) background: Option<BgViewFn<'a, State>>,
    pub(crate) padding: f32,
    pub(crate) wrap: bool,
    pub(crate) esc_end_editing: bool,
    pub(crate) enter_end_editing: bool,
    pub(crate) cursor_fill: BrushSource<TextState>,
    pub(crate) highlight_fill: BrushSource<TextState>,
    pub(crate) hint_text: Option<String>,
    pub(crate) hint_fill: BrushSource<TextState>,
    on_edit: Option<Rc<dyn Fn(&mut State, &mut PaneState, EditInteraction)>>,
}

impl<State> Debug for TextField<'_, State> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Text")
            .field("id", &self.id)
            .field("state", &self.binding)
            .field("text_fill", &self.text_fill)
            .field("font_size", &self.font_size)
            .field("font_weight", &self.font_weight)
            .field("alignment", &self.alignment)
            .field("editable", &self.editable)
            .field("line_height", &self.line_height)
            .field("background", &self.background.is_some())
            .field("wrap", &self.wrap)
            .field("cursor_fill", &self.cursor_fill)
            .field("highlight_fill", &self.highlight_fill)
            .field("hint_text", &self.hint_text)
            .field("hint_fill", &self.hint_fill)
            .field("on_edit", &self.on_edit.is_some())
            .finish()
    }
}

impl<'a, State> TextField<'a, State> {
    pub fn cursor_fill(mut self, fill: impl Into<BrushSource<TextState>>) -> Self {
        self.cursor_fill = fill.into();
        self
    }
    pub fn highlight_fill(mut self, fill: impl Into<BrushSource<TextState>>) -> Self {
        self.highlight_fill = fill.into();
        self
    }
    pub fn hint_text(mut self, text: impl AsRef<str>) -> Self {
        self.hint_text = Some(text.as_ref().to_string());
        self
    }
    pub fn hint_fill(mut self, fill: impl Into<BrushSource<TextState>>) -> Self {
        self.hint_fill = fill.into();
        self
    }
    pub fn on_edit(
        mut self,
        on_edit: impl Fn(&mut State, &mut PaneState, EditInteraction) + 'static,
    ) -> Self {
        self.on_edit = Some(Rc::new(on_edit));
        self
    }
    pub fn text_fill(mut self, fill: impl Into<BrushSource<TextState>>) -> Self {
        self.text_fill = fill.into();
        self
    }
    pub fn font_size(mut self, size: u32) -> Self {
        self.font_size = size;
        self
    }
    pub fn font_weight(mut self, weight: FontWeight) -> Self {
        self.font_weight = weight;
        self
    }
    pub fn font_family(mut self, family: impl Into<String>) -> Self {
        self.font_family = Some(family.into());
        self
    }
    pub fn align(mut self, align: Alignment) -> Self {
        self.alignment = align;
        self
    }
    pub fn background(
        mut self,
        f: impl Fn(&TextState, Area, &mut PaneState) -> Layout<'a, View<State>, PaneState> + 'a,
    ) -> Self {
        self.background = Some(Rc::new(f));
        self
    }
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }
    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }
    pub fn esc_end_editing(mut self) -> Self {
        self.esc_end_editing = true;
        self
    }
    pub fn enter_end_editing(mut self) -> Self {
        self.enter_end_editing = true;
        self
    }
}

impl<'a, State> TextField<'a, State> {
    pub fn build(self, ctx: &mut PaneState) -> Layout<'a, View<State>, PaneState>
    where
        State: 'static,
    {
        let id = self.id;
        let editable = self.editable;
        let binding = self.binding.clone();
        let font_size = self.font_size;
        let font_weight = self.font_weight;
        let font_family = self.font_family.clone();
        let text_state = (*self.state).clone();
        let fill = self.text_fill.clone();
        let hint_text = self.hint_text.clone();
        let hint_fill = self.hint_fill.clone();
        let cursor_fill = self.cursor_fill.clone();
        let highlight_fill = self.highlight_fill.clone();
        let alignment = self.alignment;
        let line_height = self.line_height;
        let wrap = self.wrap;
        let root_id = id;
        let edit_request = self.state.edit_request;
        ctx.sync_text_edit_request(root_id, edit_request.editing, edit_request.token);
        let show_hint = self.state.text.trim().is_empty() && hint_text.is_some();
        let render_text = if show_hint {
            hint_text.unwrap()
        } else {
            self.state.text.clone()
        };
        let render_fill = if show_hint { hint_fill } else { fill.clone() };
        let viewport = if show_hint {
            TextFieldViewport::default()
        } else {
            self.state.viewport
        };
        let editor = self.state.editor.clone();
        let edge_feedback = self.state.edge_feedback.clone();
        let scale_factor = ctx.scale_factor;
        let text_content = draw(move |area, ctx: &mut PaneState| {
            let is_active_editor = ctx.editor.as_ref().is_some_and(|edit| edit.id == root_id);
            let mut editor = editor.clone();
            let text_fill = render_fill.resolve(area, &text_state);
            style_text_field_editor(
                &mut editor,
                &render_text,
                text_fill,
                font_family
                    .clone()
                    .unwrap_or(DEFAULT_FONT_FAMILY.to_string()),
                font_weight,
                line_height,
                font_size as f32,
                alignment,
                if wrap { Some(area.width) } else { None },
            );
            let cursor_width = 1.5f64;
            let half_cursor_width = cursor_width * 0.5;
            let layout = editor
                .editor
                .layout(&mut ctx.font_cx, &mut ctx.layout_cx)
                .clone();
            let selection_rects: Vec<kurbo::Rect> = if is_active_editor {
                editor
                    .editor
                    .selection_geometry()
                    .iter()
                    .map(|(bb, _i)| KRect::new(bb.x0, bb.y0, bb.x1, bb.y1))
                    .filter(|rect| rect.width() > 0. && rect.height() > 0.)
                    .collect()
            } else {
                Vec::new()
            };
            let is_empty = editor.text().to_string().is_empty();
            let cursor = if is_active_editor {
                text_field_cursor(&mut editor, cursor_width as f32)
            } else {
                None
            };
            let height = layout.height();
            let empty_cursor_height = height.max(font_size as f32 * line_height);
            let (content_width, content_height) = text_field_scroll_size(&layout, cursor);
            let placement = text_field_placement(
                area,
                viewport,
                height,
                content_width,
                content_height,
                alignment,
            );
            let mut selection_views = Vec::new();
            for (selection_index, rect) in selection_rects.clone().into_iter().enumerate() {
                let resolved_area = Area {
                    x: placement.origin_x + rect.x0 as f32,
                    y: placement.origin_y + rect.y0 as f32,
                    width: rect.width() as f32,
                    height: rect.height() as f32,
                };
                selection_views.push(View::draw(
                    Box::new(DrawableType::Path(Box::new(PathData {
                        id: crate::id!(id, selection_index as u64, 1u64),
                        builder: rect_path((2., 2., 2., 2.)),
                        fill: Some(highlight_fill.resolve(resolved_area, &text_state).into()),
                        stroke: None,
                    }))),
                    resolved_area,
                ));
            }

            let mut cursor_views = Vec::new();
            if selection_rects.is_empty()
                && let Some(cursor) = if is_empty {
                    Some(kurbo::Rect::new(
                        -half_cursor_width,
                        0.,
                        half_cursor_width,
                        empty_cursor_height as f64,
                    ))
                } else {
                    cursor
                }
            {
                let resolved_area = Area {
                    x: placement.origin_x + cursor.x0 as f32,
                    y: placement.origin_y + cursor.y0 as f32,
                    width: cursor.width() as f32,
                    height: cursor.height() as f32,
                };
                let rounding = (cursor_width * 0.5) as f32;
                cursor_views.push(View::<State>::draw(
                    Box::new(DrawableType::Path(Box::new(PathData {
                        id: crate::id!(id, 2u64),
                        builder: rect_path((rounding, rounding, rounding, rounding)),
                        fill: Some(cursor_fill.resolve(resolved_area, &text_state).into()),
                        stroke: None,
                    }))),
                    resolved_area,
                ));
            }

            text_field_layout_views::<State>(
                area,
                layout,
                scale_factor,
                placement,
                &edge_feedback,
                ctx,
                selection_views,
                cursor_views,
            )
        })
        .expand();
        let content = stack(vec![
            draw(move |area, _| vec![View::editor_area(root_id, area)]).inert(),
            text_content,
        ])
        .pad(if editable { self.padding } else { 0. });
        let sensor = {
            let binding = binding.clone();
            let font_family = self.font_family.clone();
            let on_edit = self.on_edit.clone();
            let edit_fill = fill.clone();
            let key_font_family = font_family.clone();
            let sensor_id = crate::id!(root_id);
            stack(vec![
                rect(sensor_id)
                    .fill(TRANSPARENT)
                    .view()
                    .gesture(gesture::key(crate::id!(root_id, 1u64)).observe().run({
                        let on_edit = on_edit.clone();
                        let binding = binding.clone();
                        move |state, app, event| {
                            if event.phase != KeyPhase::Pressed {
                                return;
                            }
                            if (self.enter_end_editing && event.key == Key::Named(NamedKey::Enter))
                                || (self.esc_end_editing
                                    && event.key == Key::Named(NamedKey::Escape))
                            {
                                binding.update(state, |s| s.end_editing(app));
                                app.end_editing();
                                if let Some(ref on_edit) = on_edit {
                                    (on_edit)(state, app, EditInteraction::End);
                                }
                                return;
                            };
                            if app.editor.as_ref().map(|e| e.id) == Some(root_id) {
                                let editor_area = text_field_editor_area(app, root_id);
                                style_bound_text_field_editor(
                                    state,
                                    &binding,
                                    editor_area,
                                    TextFieldEditStyle {
                                        fill: &edit_fill,
                                        font_family: &key_font_family,
                                        font_weight,
                                        line_height,
                                        font_size: font_size as f32,
                                        alignment,
                                        wrap,
                                    },
                                );
                                let ts = binding.get_mut(state);
                                ts.editor.handle_key(
                                    event.key.clone(),
                                    &mut app.layout_cx,
                                    &mut app.font_cx,
                                    app.modifiers,
                                );
                                let edit_text = {
                                    let edit_text = ts.editor.text().to_string();
                                    ts.text = edit_text.clone();
                                    edit_text
                                };
                                show_text_field_cursor_after_edit(ts, editor_area, app);
                                if let Some(ref on_edit) = on_edit {
                                    on_edit(state, app, EditInteraction::Update(edit_text.clone()));
                                }
                            }
                        }
                    }))
                    .gesture(
                        gesture::drag(crate::id!(root_id, 2u64))
                            .button(MouseButton::Left)
                            .run({
                                let binding = binding.clone();
                                let font_family = font_family.clone();
                                let on_edit = on_edit.clone();
                                let edit_fill = self.text_fill.clone();
                                move |state: &mut State, app, drag| {
                                    let started_editing = if matches!(drag, DragPhase::Began { .. })
                                    {
                                        begin_text_field_editing(app, root_id)
                                    } else {
                                        false
                                    };
                                    if app.editor.as_ref().map(|e| e.id) != Some(root_id) {
                                        return;
                                    }

                                    let editor_area = text_field_editor_area(app, root_id);
                                    style_bound_text_field_editor(
                                        state,
                                        &binding,
                                        editor_area,
                                        TextFieldEditStyle {
                                            fill: &edit_fill,
                                            font_family: &font_family,
                                            font_weight,
                                            line_height,
                                            font_size: font_size as f32,
                                            alignment,
                                            wrap,
                                        },
                                    );
                                    let ts = binding.get_mut(state);
                                    match drag {
                                        DragPhase::Began { start_global, .. } => {
                                            let point = text_field_local_point(
                                                &mut ts.editor,
                                                editor_area,
                                                ts.viewport,
                                                start_global,
                                                alignment,
                                                app,
                                            );
                                            ts.editor.mouse_moved(
                                                point,
                                                &mut app.layout_cx,
                                                &mut app.font_cx,
                                            );
                                            ts.editor.mouse_pressed(
                                                &mut app.layout_cx,
                                                &mut app.font_cx,
                                            );
                                        }
                                        DragPhase::Updated { current_global, .. } => {
                                            scroll_text_field_viewport_for_drag(
                                                ts,
                                                editor_area,
                                                current_global,
                                                app,
                                            );
                                            let point = text_field_local_point(
                                                &mut ts.editor,
                                                editor_area,
                                                ts.viewport,
                                                current_global,
                                                alignment,
                                                app,
                                            );
                                            ts.editor.mouse_moved(
                                                point,
                                                &mut app.layout_cx,
                                                &mut app.font_cx,
                                            );
                                        }
                                        DragPhase::Completed {
                                            current_global,
                                            distance,
                                            ..
                                        } => {
                                            if distance > 0.0 {
                                                let point = text_field_local_point(
                                                    &mut ts.editor,
                                                    editor_area,
                                                    ts.viewport,
                                                    current_global,
                                                    alignment,
                                                    app,
                                                );
                                                ts.editor.mouse_moved(
                                                    point,
                                                    &mut app.layout_cx,
                                                    &mut app.font_cx,
                                                );
                                            }
                                            ts.editor.mouse_released();
                                        }
                                    }
                                    if started_editing && let Some(ref on_edit) = on_edit {
                                        on_edit(state, app, EditInteraction::Start);
                                    }
                                }
                            }),
                    )
                    .gesture(
                        gesture::click(crate::id!(root_id, 3u64))
                            .button(MouseButton::Left | MouseButton::Right)
                            .run({
                                let binding = binding.clone();
                                let font_family = font_family.clone();
                                let on_edit = on_edit.clone();
                                let edit_fill = self.text_fill.clone();
                                move |state: &mut State, app, event| {
                                    let editor_area = text_field_editor_area(app, root_id);
                                    let style_for_edit = |state: &mut State| {
                                        style_bound_text_field_editor(
                                            state,
                                            &binding,
                                            editor_area,
                                            TextFieldEditStyle {
                                                fill: &edit_fill,
                                                font_family: &font_family,
                                                font_weight,
                                                line_height,
                                                font_size: font_size as f32,
                                                alignment,
                                                wrap,
                                            },
                                        );
                                    };

                                    match (event.button, event.state) {
                                        (MouseButton::Left, ClickPhase::Started) => {}
                                        (MouseButton::Left, ClickPhase::Completed) => {
                                            let started_editing =
                                                begin_text_field_editing(app, root_id);
                                            style_for_edit(state);
                                            let ts = binding.get_mut(state);
                                            let point = text_field_local_point(
                                                &mut ts.editor,
                                                editor_area,
                                                ts.viewport,
                                                event.location.global(),
                                                alignment,
                                                app,
                                            );
                                            ts.editor.mouse_moved(
                                                point,
                                                &mut app.layout_cx,
                                                &mut app.font_cx,
                                            );
                                            ts.editor.mouse_pressed(
                                                &mut app.layout_cx,
                                                &mut app.font_cx,
                                            );
                                            ts.editor.mouse_released();
                                            if started_editing && let Some(ref on_edit) = on_edit {
                                                on_edit(state, app, EditInteraction::Start);
                                            }
                                        }
                                        (MouseButton::Right, ClickPhase::Started) => {
                                            let started_editing =
                                                begin_text_field_editing(app, root_id);
                                            style_for_edit(state);
                                            let ts = binding.get_mut(state);
                                            if let Some(pos) = app.cursor_position {
                                                let point = text_field_local_point(
                                                    &mut ts.editor,
                                                    editor_area,
                                                    ts.viewport,
                                                    pos,
                                                    alignment,
                                                    app,
                                                );
                                                ts.editor.mouse_moved(
                                                    point,
                                                    &mut app.layout_cx,
                                                    &mut app.font_cx,
                                                );
                                            }
                                            let point = text_field_local_point(
                                                &mut ts.editor,
                                                editor_area,
                                                ts.viewport,
                                                event.location.global(),
                                                alignment,
                                                app,
                                            );
                                            ts.editor.mouse_moved(
                                                point,
                                                &mut app.layout_cx,
                                                &mut app.font_cx,
                                            );
                                            ts.editor.mouse_pressed(
                                                &mut app.layout_cx,
                                                &mut app.font_cx,
                                            );
                                            if started_editing && let Some(ref on_edit) = on_edit {
                                                on_edit(state, app, EditInteraction::Start);
                                            }
                                        }
                                        (_, ClickPhase::Completed | ClickPhase::Cancelled) => {
                                            binding.get_mut(state).editor.mouse_released();
                                        }
                                        _ => {}
                                    }
                                }
                            }),
                    )
                    .gesture(gesture::scroll(crate::id!(root_id, 4u64)).run({
                        let binding = binding.clone();
                        let font_family = font_family.clone();
                        let edit_fill = self.text_fill.clone();
                        move |state: &mut State, app, delta| {
                            let editor_area = text_field_editor_area(app, root_id);
                            style_bound_text_field_editor(
                                state,
                                &binding,
                                editor_area,
                                TextFieldEditStyle {
                                    fill: &edit_fill,
                                    font_family: &font_family,
                                    font_weight,
                                    line_height,
                                    font_size: font_size as f32,
                                    alignment,
                                    wrap,
                                },
                            );
                            let ts = binding.get_mut(state);
                            let (content_width, content_height) =
                                bound_text_field_scroll_size(&mut ts.editor, app, 1.5);
                            update_text_field_viewport(
                                ts,
                                TextFieldViewport {
                                    x: ts.viewport.x - delta.x,
                                    y: ts.viewport.y - delta.y,
                                },
                                editor_area,
                                content_width,
                                content_height,
                                app,
                            );
                        }
                    }))
                    .build(ctx),
            ])
        }
        .inert();
        let background_fn = self.background;
        let ts = (*self.state).clone();
        let bg = if let Some(f) = background_fn {
            draw(move |area, ctx: &mut PaneState| f(&ts, area, ctx).draw(area, ctx))
        } else if editable {
            draw(move |area, ctx: &mut PaneState| {
                rect(crate::id!(id))
                    .fill(Color::from_rgb8(50, 50, 50))
                    .stroke(
                        Color::from_rgb8(60, 60, 60),
                        Stroke::new(DEFAULT_STROKE_WIDTH as f64),
                    )
                    .corner_rounding(DEFAULT_CORNER_ROUNDING)
                    .build(ctx)
                    .draw(area, ctx)
            })
        } else {
            draw(|_, _| Vec::new())
        };
        stack(vec![stack(vec![bg.inert(), sensor]), content])
    }
}
