use crate::brush_source::BrushSource;
use crate::editor::Editor;
use crate::pane::{PaneElement, PaneState, View};
use crate::primitives::shape::{PathData, rect_path};
use crate::view::DrawableType;
use crate::view::rect_path as clip_rect_path;
use crate::{
    Binding, ClickPhase, Compositing, DEFAULT_CORNER_ROUNDING, DEFAULT_FG_COLOR,
    DEFAULT_FONT_FAMILY, DEFAULT_FONT_SIZE, DEFAULT_PADDING, DEFAULT_PURP, DEFAULT_STROKE_WIDTH,
    DragPhase, EditInteraction, Gesture, Key, KeyPhase, Modifier, MouseButton, NamedKey, gesture,
    rect,
};
use backer::{Area, nodes::*};
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
    pub editing: bool,
    pub(crate) editor: Editor,
    pub(crate) edit_command: Option<TextEditCommand>,
    pub(crate) viewport: TextFieldViewport,
    pub(crate) edge_feedback: ScrollEdgeFeedback,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TextEditCommand {
    Focus(u64),
    End(u64),
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TextFieldLineMode {
    Single,
    Multi,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextFieldVerticalAlignment {
    Top,
    Center,
    Bottom,
}

fn without_line_breaks(text: &str) -> String {
    text.chars()
        .filter(|character| !matches!(character, '\n' | '\r'))
        .collect()
}

impl Debug for TextState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextState")
            .field("text", &self.text)
            .field("editing", &self.editing)
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
            edit_command: None,
            editing: false,
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
        self.editor
            .editor
            .driver(&mut app.font_cx, &mut app.layout_cx)
            .select_all();
        app.request_redraw();
    }

    pub fn begin_editing(&mut self, app: &mut PaneState) {
        self.begin_editing_with(app, InitialSelection::End);
    }

    pub fn begin_editing_with(&mut self, app: &mut PaneState, target: InitialSelection) {
        let mut driver = self
            .editor
            .editor
            .driver(&mut app.font_cx, &mut app.layout_cx);
        match target {
            InitialSelection::Start => driver.move_to_text_start(),
            InitialSelection::End => driver.move_to_text_end(),
            InitialSelection::All => driver.select_all(),
        }
        self.editor
            .focus_without_pointer_selection(&mut app.layout_cx, &mut app.font_cx);
        self.editing = true;
        self.edit_command = Some(TextEditCommand::Focus(
            app.next_text_edit_command_revision(),
        ));
        app.request_redraw();
    }

    pub fn end_editing(&mut self, app: &mut PaneState) {
        self.editing = false;
        self.edit_command = Some(TextEditCommand::End(app.next_text_edit_command_revision()));
        app.request_redraw();
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

fn style_bound_text_field_editor<State>(
    state: &mut State,
    binding: &Binding<State, TextState>,
    area: Area,
    fill: &BrushSource<TextState>,
    font_family: &Option<String>,
    font_weight: FontWeight,
    line_height: f32,
    font_size: f32,
    alignment: Alignment,
    wrap: bool,
) {
    let text_fill = fill.resolve(area, binding.get(state));
    let ts = binding.get_mut(state);
    style_text_field_editor(
        &mut ts.editor,
        &ts.text,
        text_fill,
        font_family
            .clone()
            .unwrap_or_else(|| DEFAULT_FONT_FAMILY.to_string()),
        font_weight,
        line_height,
        font_size,
        alignment,
        if wrap { Some(area.width) } else { None },
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
    vertical_alignment: TextFieldVerticalAlignment,
    wrap: bool,
    app: &mut PaneState,
) -> crate::Point {
    let cursor_width = 1.5;
    let layout = editor
        .editor
        .layout(&mut app.font_cx, &mut app.layout_cx)
        .clone();
    let cursor = text_field_cursor(editor, cursor_width);
    let (scroll_width, scroll_height) =
        text_field_scroll_size(&layout, cursor, wrap.then_some(area.width));
    let placement = text_field_placement(
        area,
        viewport,
        layout.height(),
        scroll_width,
        scroll_height,
        alignment,
        vertical_alignment,
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

fn text_field_scroll_size(
    layout: &parley::Layout<Brush>,
    cursor: Option<KRect>,
    wrap_width: Option<f32>,
) -> (f32, f32) {
    let mut width = wrap_width.unwrap_or_else(|| layout.full_width());
    let mut height = layout.height();
    if let Some(cursor) = cursor {
        if wrap_width.is_none() {
            width = width.max(cursor.x1 as f32);
        }
        height = height.max(cursor.y1 as f32);
    }
    (width, height)
}

fn bound_text_field_scroll_size(
    editor: &mut Editor,
    app: &mut PaneState,
    cursor_width: f32,
    wrap_width: Option<f32>,
) -> (f32, f32) {
    let layout = editor
        .editor
        .layout(&mut app.font_cx, &mut app.layout_cx)
        .clone();
    let cursor = text_field_cursor(editor, cursor_width);
    text_field_scroll_size(&layout, cursor, wrap_width)
}

#[derive(Clone, Copy)]
enum TextFieldScrollAxis {
    Horizontal,
    Vertical,
}

fn text_field_scroll_gesture<State: 'static>(
    id: u64,
    root_id: u64,
    axis: TextFieldScrollAxis,
    binding: Binding<State, TextState>,
    font_family: Option<String>,
    edit_fill: BrushSource<TextState>,
    font_weight: FontWeight,
    line_height: f32,
    font_size: f32,
    alignment: Alignment,
    wrap: bool,
) -> Gesture<State> {
    let scroll = match axis {
        TextFieldScrollAxis::Horizontal => gesture::scroll(id).horizontal(),
        TextFieldScrollAxis::Vertical => gesture::scroll(id).vertical(),
    };
    scroll.run(move |state: &mut State, app, delta| {
        let editor_area = text_field_editor_area(app, root_id);
        style_bound_text_field_editor(
            state,
            &binding,
            editor_area,
            &edit_fill,
            &font_family,
            font_weight,
            line_height,
            font_size,
            alignment,
            wrap,
        );
        let ts = binding.get_mut(state);
        let (content_width, content_height) = bound_text_field_scroll_size(
            &mut ts.editor,
            app,
            1.5,
            wrap.then_some(editor_area.width),
        );
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
    })
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

fn text_field_placement(
    area: Area,
    viewport: TextFieldViewport,
    layout_height: f32,
    scroll_width: f32,
    scroll_height: f32,
    alignment: Alignment,
    vertical_alignment: TextFieldVerticalAlignment,
) -> TextFieldPlacement {
    let overflow_x = scroll_width > area.width;
    let overflow_y = scroll_height > area.height;
    let viewport = clamp_text_field_viewport(viewport, area, scroll_width, scroll_height);
    let origin_x = if overflow_x {
        area.x - viewport.x
    } else {
        let extra = (area.width - scroll_width).max(0.);
        area.x
            + match alignment {
                Alignment::Center => extra * 0.5,
                Alignment::End | Alignment::Right => extra,
                _ => 0.,
            }
    };
    let origin_y = if overflow_y {
        area.y - viewport.y
    } else {
        let extra = (area.height - layout_height).max(0.);
        area.y
            + match vertical_alignment {
                TextFieldVerticalAlignment::Top => 0.,
                TextFieldVerticalAlignment::Center => extra * 0.5,
                TextFieldVerticalAlignment::Bottom => extra,
            }
    };
    TextFieldPlacement {
        origin_x,
        origin_y,
        overflow_x,
        overflow_y,
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
        vertical_alignment: TextFieldVerticalAlignment::Center,
        editable: true,
        line_height: 1.,
        background: None,
        padding: DEFAULT_PADDING,
        wrap: false,
        line_mode: TextFieldLineMode::Single,
        cursor_fill: BrushSource::Static(Brush::Solid(DEFAULT_FG_COLOR)),
        highlight_fill: BrushSource::Static(Brush::Solid(DEFAULT_PURP)),
        hint_text: None,
        hint_fill: BrushSource::Static(Brush::Solid(DEFAULT_FG_COLOR.with_alpha(0.55))),
        display: None,
        on_edit: None,
        esc_end_editing: false,
        enter_end_editing: false,
    }
}

type BgViewFn<'a, State> = Rc<dyn Fn(&TextState, Area, &mut PaneState) -> View<'a, State> + 'a>;

pub struct TextField<'a, State> {
    pub(crate) id: u64,
    pub(crate) state: &'a TextState,
    pub(crate) binding: Binding<State, TextState>,
    pub(crate) text_fill: BrushSource<TextState>,
    pub(crate) font_size: u32,
    pub(crate) font_weight: FontWeight,
    pub(crate) font_family: Option<String>,
    pub(crate) alignment: Alignment,
    pub(crate) vertical_alignment: TextFieldVerticalAlignment,
    pub(crate) editable: bool,
    pub(crate) line_height: f32,
    pub(crate) background: Option<BgViewFn<'a, State>>,
    pub(crate) padding: f32,
    pub(crate) wrap: bool,
    line_mode: TextFieldLineMode,
    pub(crate) esc_end_editing: bool,
    pub(crate) enter_end_editing: bool,
    pub(crate) cursor_fill: BrushSource<TextState>,
    pub(crate) highlight_fill: BrushSource<TextState>,
    pub(crate) hint_text: Option<String>,
    pub(crate) hint_fill: BrushSource<TextState>,
    display: Option<Rc<dyn Fn(&TextState) -> String>>,
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
            .field("vertical_alignment", &self.vertical_alignment)
            .field("editable", &self.editable)
            .field("line_height", &self.line_height)
            .field("background", &self.background.is_some())
            .field("wrap", &self.wrap)
            .field("line_mode", &self.line_mode)
            .field("cursor_fill", &self.cursor_fill)
            .field("highlight_fill", &self.highlight_fill)
            .field("hint_text", &self.hint_text)
            .field("hint_fill", &self.hint_fill)
            .field("display", &self.display.is_some())
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
    pub fn display(mut self, display: impl Fn(&TextState) -> String + 'static) -> Self {
        self.display = Some(Rc::new(display));
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
    pub fn vertical_align(mut self, align: TextFieldVerticalAlignment) -> Self {
        self.vertical_alignment = align;
        self
    }
    pub fn background(
        mut self,
        f: impl Fn(&TextState, Area, &mut PaneState) -> View<'a, State> + 'a,
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
    pub fn singleline(mut self) -> Self {
        self.line_mode = TextFieldLineMode::Single;
        self
    }
    pub fn multiline(mut self) -> Self {
        self.line_mode = TextFieldLineMode::Multi;
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
    pub fn build(self, ctx: &mut PaneState) -> View<'a, State>
    where
        State: 'static,
    {
        let id = self.id;
        let editable = self.editable;
        let padding = self.padding;
        let binding = self.binding.clone();
        let font_size = self.font_size;
        let font_weight = self.font_weight;
        let font_family = self.font_family.clone();
        let text_state = (*self.state).clone();
        let fill = self.text_fill.clone();
        let hint_text = self.hint_text.clone();
        let hint_fill = self.hint_fill.clone();
        let display = self.display.clone();
        let uses_display = display.is_some();
        let cursor_fill = self.cursor_fill.clone();
        let highlight_fill = self.highlight_fill.clone();
        let alignment = self.alignment;
        let vertical_alignment = self.vertical_alignment;
        let line_height = self.line_height;
        let wrap = self.wrap;
        let line_mode = self.line_mode;
        let root_id = id;
        ctx.apply_text_edit_command(root_id, self.state.edit_command);
        let show_hint = self.state.text.trim().is_empty() && hint_text.is_some();
        let render_text = if show_hint {
            hint_text.unwrap()
        } else if let Some(display) = display {
            display(self.state)
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
        let horizontal_scroll_gesture = text_field_scroll_gesture(
            crate::id!(root_id, 4u64),
            root_id,
            TextFieldScrollAxis::Horizontal,
            binding.clone(),
            font_family.clone(),
            self.text_fill.clone(),
            font_weight,
            line_height,
            font_size as f32,
            alignment,
            wrap,
        );
        let vertical_scroll_gesture = text_field_scroll_gesture(
            crate::id!(root_id, 5u64),
            root_id,
            TextFieldScrollAxis::Vertical,
            binding.clone(),
            font_family.clone(),
            self.text_fill.clone(),
            font_weight,
            line_height,
            font_size as f32,
            alignment,
            wrap,
        );
        let scroll_text_state = text_state.clone();
        let scroll_text = render_text.clone();
        let scroll_fill = render_fill.clone();
        let scroll_editor = editor.clone();
        let text_content = draw(move |area, ctx: &mut PaneState| {
            let is_focused_editor = ctx.text_field_is_focused(root_id);
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
            let selection_rects: Vec<kurbo::Rect> = if is_focused_editor {
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
            let cursor = if is_focused_editor {
                text_field_cursor(&mut editor, cursor_width as f32)
            } else {
                None
            };
            let height = layout.height();
            let empty_cursor_height = height.max(font_size as f32 * line_height);
            let (content_width, content_height) =
                text_field_scroll_size(&layout, cursor, wrap.then_some(area.width));
            let placement = text_field_placement(
                area,
                viewport,
                height,
                content_width,
                content_height,
                alignment,
                vertical_alignment,
            );
            let mut selection_views = Vec::new();
            for (selection_index, rect) in selection_rects.clone().into_iter().enumerate() {
                let resolved_area = Area {
                    x: placement.origin_x + rect.x0 as f32,
                    y: placement.origin_y + rect.y0 as f32,
                    width: rect.width() as f32,
                    height: rect.height() as f32,
                };
                selection_views.push(PaneElement::draw(
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
                cursor_views.push(PaneElement::<State>::draw(
                    Box::new(DrawableType::Path(Box::new(PathData {
                        id: crate::id!(id, 2u64),
                        builder: rect_path((rounding, rounding, rounding, rounding)),
                        fill: Some(cursor_fill.resolve(resolved_area, &text_state).into()),
                        stroke: None,
                    }))),
                    resolved_area,
                ));
            }

            let now = Instant::now();
            let needs_clip =
                placement.overflow_x || placement.overflow_y || edge_feedback.is_animating(now);
            let mut views = Vec::new();
            views.extend(selection_views);
            let transform =
                Affine::translate((placement.origin_x as f64, placement.origin_y as f64))
                    .then_scale(scale_factor);
            views.push(PaneElement::<State>::draw(
                Box::new(DrawableType::Layout(Box::new((layout, transform)))),
                area,
            ));
            views.extend(cursor_views);
            if needs_clip {
                stack(vec![
                    draw(move |_, _| views),
                    scroll_edge_glows::<State>(ctx, &edge_feedback, now),
                ])
                .clipped(clip_rect_path)
                .draw(area, ctx)
            } else {
                views
            }
        })
        .expand();
        let edit_callback = {
            let binding = binding.clone();
            let on_edit = self.on_edit.clone();
            Some(
                Rc::new(move |state: &mut State, app: &mut PaneState, edit| {
                    match edit {
                        EditInteraction::Start => binding.get_mut(state).editing = true,
                        EditInteraction::End => binding.get_mut(state).editing = false,
                        EditInteraction::Update(_) => {}
                    }
                    if let Some(on_edit) = &on_edit {
                        on_edit(state, app, edit);
                    }
                }) as crate::pane::EditHandler<State>,
            )
        };
        let content = stack(vec![
            draw(move |area, _| {
                vec![PaneElement::editor_area(
                    root_id,
                    area,
                    edit_callback.clone(),
                )]
            })
            .inert(),
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
            let horizontal_scroll_sensor_id = crate::id!(root_id, 6u64);
            let vertical_scroll_sensor_id = crate::id!(root_id, 7u64);
            let scroll_font_family = font_family.clone();
            let scroll_receiver = draw(move |area, ctx: &mut PaneState| {
                if show_hint {
                    return Vec::new();
                }
                let inset = if editable { padding } else { 0. };
                let editor_area = Area {
                    x: area.x + inset,
                    y: area.y + inset,
                    width: (area.width - inset * 2.).max(0.),
                    height: (area.height - inset * 2.).max(0.),
                };
                if editor_area.width <= 0. || editor_area.height <= 0. {
                    return Vec::new();
                }
                let mut editor = scroll_editor.clone();
                let text_fill = scroll_fill.resolve(editor_area, &scroll_text_state);
                style_text_field_editor(
                    &mut editor,
                    &scroll_text,
                    text_fill,
                    scroll_font_family
                        .clone()
                        .unwrap_or_else(|| DEFAULT_FONT_FAMILY.to_string()),
                    font_weight,
                    line_height,
                    font_size as f32,
                    alignment,
                    if wrap { Some(editor_area.width) } else { None },
                );
                let (content_width, content_height) = bound_text_field_scroll_size(
                    &mut editor,
                    ctx,
                    1.5,
                    wrap.then_some(editor_area.width),
                );
                let mut views = Vec::new();
                if content_width > editor_area.width {
                    views.extend(
                        rect(horizontal_scroll_sensor_id)
                            .fill(TRANSPARENT)
                            .view()
                            .gesture(horizontal_scroll_gesture.clone())
                            .build(ctx)
                            .draw(area, ctx),
                    );
                }
                if content_height > editor_area.height {
                    views.extend(
                        rect(vertical_scroll_sensor_id)
                            .fill(TRANSPARENT)
                            .view()
                            .gesture(vertical_scroll_gesture.clone())
                            .build(ctx)
                            .draw(area, ctx),
                    );
                }
                views
            });
            stack(vec![
                scroll_receiver,
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
                            if !app.text_field_is_focused(root_id) {
                                return;
                            }
                            if (self.enter_end_editing && event.key == Key::Named(NamedKey::Enter))
                                || (self.esc_end_editing
                                    && event.key == Key::Named(NamedKey::Escape))
                            {
                                binding.update(state, |s| s.end_editing(app));
                                app.end_editing();
                                return;
                            };
                            let editor_area = text_field_editor_area(app, root_id);
                            style_bound_text_field_editor(
                                state,
                                &binding,
                                editor_area,
                                &edit_fill,
                                &key_font_family,
                                font_weight,
                                line_height,
                                font_size as f32,
                                alignment,
                                wrap,
                            );
                            let ts = binding.get_mut(state);
                            let previous_text = ts.text.clone();
                            let mut handled = false;
                            let action_mod = app.modifiers.unwrap_or_default().contains(
                                if cfg!(target_os = "macos") {
                                    Modifier::Super
                                } else {
                                    Modifier::Control
                                },
                            );
                            if uses_display
                                && let Key::Character(text) = &event.key
                                && action_mod
                                && (text.eq_ignore_ascii_case("c")
                                    || text.eq_ignore_ascii_case("x"))
                            {
                                handled = true;
                            }
                            if line_mode == TextFieldLineMode::Single {
                                match &event.key {
                                    Key::Named(NamedKey::Enter) => handled = true,
                                    Key::Character(text)
                                        if action_mod && text.eq_ignore_ascii_case("v") =>
                                    {
                                        use clipboard_rs::{Clipboard, ClipboardContext};

                                        let cb = ClipboardContext::new().unwrap();
                                        let text =
                                            without_line_breaks(&cb.get_text().unwrap_or_default());
                                        ts.editor
                                            .editor
                                            .driver(&mut app.font_cx, &mut app.layout_cx)
                                            .insert_or_replace_selection(&text);
                                        handled = true;
                                    }
                                    Key::Character(text) if text.contains(['\n', '\r']) => {
                                        let text = without_line_breaks(text);
                                        ts.editor
                                            .editor
                                            .driver(&mut app.font_cx, &mut app.layout_cx)
                                            .insert_or_replace_selection(&text);
                                        handled = true;
                                    }
                                    _ => {}
                                }
                            }
                            if !handled {
                                ts.editor.handle_key(
                                    event.key.clone(),
                                    &mut app.layout_cx,
                                    &mut app.font_cx,
                                    app.modifiers,
                                );
                            }
                            let edit_text = ts.editor.text().to_string();
                            ts.text = edit_text.clone();
                            let layout = ts
                                .editor
                                .editor
                                .layout(&mut app.font_cx, &mut app.layout_cx)
                                .clone();
                            let cursor = text_field_cursor(&mut ts.editor, 1.5);
                            let (content_width, content_height) = text_field_scroll_size(
                                &layout,
                                cursor,
                                wrap.then_some(editor_area.width),
                            );
                            let mut visible = ts.viewport;
                            let mut content_width = content_width;
                            let mut content_height = content_height;
                            if let Some(cursor) = cursor {
                                if !wrap {
                                    content_width = content_width.max(cursor.x1 as f32);
                                }
                                content_height = content_height.max(cursor.y1 as f32);

                                if !wrap {
                                    let cursor_x0 = cursor.x0 as f32;
                                    let cursor_x1 = cursor.x1 as f32;
                                    let inset = editor_area.width / 3.;
                                    let min = cursor_x1 - editor_area.width + inset;
                                    let max = cursor_x0 - inset;
                                    visible.x = if min <= max {
                                        ts.viewport.x.clamp(min, max)
                                    } else {
                                        let min = cursor_x1 - editor_area.width;
                                        let max = cursor_x0;
                                        if min <= max {
                                            ts.viewport.x.clamp(min, max)
                                        } else {
                                            ts.viewport
                                                .x
                                                .clamp(cursor_x0 - editor_area.width, cursor_x0)
                                        }
                                    };
                                }

                                let cursor_y0 = cursor.y0 as f32;
                                let cursor_y1 = cursor.y1 as f32;
                                let inset = editor_area.height / 3.;
                                let min = cursor_y1 - editor_area.height + inset;
                                let max = cursor_y0 - inset;
                                visible.y = if min <= max {
                                    ts.viewport.y.clamp(min, max)
                                } else {
                                    let min = cursor_y1 - editor_area.height;
                                    let max = cursor_y0;
                                    if min <= max {
                                        ts.viewport.y.clamp(min, max)
                                    } else {
                                        ts.viewport
                                            .y
                                            .clamp(cursor_y0 - editor_area.height, cursor_y0)
                                    }
                                };
                            }
                            ts.viewport = clamp_text_field_viewport(
                                visible,
                                editor_area,
                                content_width,
                                content_height,
                            );
                            if edit_text != previous_text
                                && let Some(ref on_edit) = on_edit
                            {
                                on_edit(state, app, EditInteraction::Update(edit_text.clone()));
                                if app.text_field_is_focused(root_id) && !binding.get(state).editing
                                {
                                    let ts = binding.get_mut(state);
                                    ts.editor
                                        .editor
                                        .driver(&mut app.font_cx, &mut app.layout_cx)
                                        .move_to_text_end();
                                    ts.editor.focus_without_pointer_selection(
                                        &mut app.layout_cx,
                                        &mut app.font_cx,
                                    );
                                    ts.editing = true;
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
                                let edit_fill = self.text_fill.clone();
                                move |state: &mut State, app, drag| {
                                    if matches!(drag, DragPhase::Began { .. })
                                        && app.begin_editing(root_id)
                                    {
                                        app.request_redraw();
                                    }
                                    if !app.text_field_is_focused(root_id) {
                                        return;
                                    }

                                    let editor_area = text_field_editor_area(app, root_id);
                                    style_bound_text_field_editor(
                                        state,
                                        &binding,
                                        editor_area,
                                        &edit_fill,
                                        &font_family,
                                        font_weight,
                                        line_height,
                                        font_size as f32,
                                        alignment,
                                        wrap,
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
                                                vertical_alignment,
                                                wrap,
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
                                            let mut viewport = ts.viewport;
                                            let right = editor_area.x + editor_area.width;
                                            let bottom = editor_area.y + editor_area.height;
                                            if current_global.x < editor_area.x as f64 {
                                                viewport.x -=
                                                    editor_area.x - current_global.x as f32;
                                            } else if current_global.x > right as f64 {
                                                viewport.x += current_global.x as f32 - right;
                                            }
                                            if current_global.y < editor_area.y as f64 {
                                                viewport.y -=
                                                    editor_area.y - current_global.y as f32;
                                            } else if current_global.y > bottom as f64 {
                                                viewport.y += current_global.y as f32 - bottom;
                                            }

                                            let (content_width, content_height) =
                                                bound_text_field_scroll_size(
                                                    &mut ts.editor,
                                                    app,
                                                    1.5,
                                                    wrap.then_some(editor_area.width),
                                                );
                                            update_text_field_viewport(
                                                ts,
                                                viewport,
                                                editor_area,
                                                content_width,
                                                content_height,
                                                app,
                                            );
                                            let point = text_field_local_point(
                                                &mut ts.editor,
                                                editor_area,
                                                ts.viewport,
                                                current_global,
                                                alignment,
                                                vertical_alignment,
                                                wrap,
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
                                                    vertical_alignment,
                                                    wrap,
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
                                }
                            }),
                    )
                    .gesture(
                        gesture::click(crate::id!(root_id, 3u64))
                            .button(MouseButton::Left | MouseButton::Right)
                            .run({
                                let binding = binding.clone();
                                let font_family = font_family.clone();
                                let edit_fill = self.text_fill.clone();
                                move |state: &mut State, app, event| {
                                    let editor_area = text_field_editor_area(app, root_id);
                                    let style_for_edit = |state: &mut State| {
                                        style_bound_text_field_editor(
                                            state,
                                            &binding,
                                            editor_area,
                                            &edit_fill,
                                            &font_family,
                                            font_weight,
                                            line_height,
                                            font_size as f32,
                                            alignment,
                                            wrap,
                                        );
                                    };

                                    match (event.button, event.state) {
                                        (MouseButton::Left, ClickPhase::Started) => {}
                                        (MouseButton::Left, ClickPhase::Completed) => {
                                            if app.begin_editing(root_id) {
                                                app.request_redraw();
                                            }
                                            style_for_edit(state);
                                            let ts = binding.get_mut(state);
                                            let point = text_field_local_point(
                                                &mut ts.editor,
                                                editor_area,
                                                ts.viewport,
                                                event.location.global(),
                                                alignment,
                                                vertical_alignment,
                                                wrap,
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
                                        }
                                        (MouseButton::Right, ClickPhase::Started) => {
                                            if app.begin_editing(root_id) {
                                                app.request_redraw();
                                            }
                                            style_for_edit(state);
                                            let ts = binding.get_mut(state);
                                            if let Some(pos) = app.cursor_position {
                                                let point = text_field_local_point(
                                                    &mut ts.editor,
                                                    editor_area,
                                                    ts.viewport,
                                                    pos,
                                                    alignment,
                                                    vertical_alignment,
                                                    wrap,
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
                                                vertical_alignment,
                                                wrap,
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
                                        (_, ClickPhase::Completed | ClickPhase::Cancelled) => {
                                            binding.get_mut(state).editor.mouse_released();
                                        }
                                        _ => {}
                                    }
                                }
                            }),
                    )
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{Frame, RenderItem};
    use crate::*;

    fn test_pane<State: 'static>(builder: PaneBuilder<State>) -> Pane<State> {
        builder.build()
    }

    fn action_modifier() -> Modifier {
        if cfg!(target_os = "macos") {
            Modifier::Super
        } else {
            Modifier::Control
        }
    }

    fn text_origin_x(frame: &Frame) -> f64 {
        frame
            .items
            .iter()
            .find_map(|item| match item {
                RenderItem::Layout { transform, .. } => Some(transform.as_coeffs()[4]),
                _ => None,
            })
            .expect("text layout rendered")
    }

    fn text_origin_y(frame: &Frame) -> f64 {
        frame
            .items
            .iter()
            .find_map(|item| match item {
                RenderItem::Layout { transform, .. } => Some(transform.as_coeffs()[5]),
                _ => None,
            })
            .expect("text layout rendered")
    }

    #[test]
    fn supports_vertical_alignment() {
        struct State {
            text: TextState,
            vertical_alignment: TextFieldVerticalAlignment,
        }

        const FIELD: u64 = 9010;

        fn view<'a>(state: &'a State, app: &mut PaneState) -> View<'a, State> {
            text_field(FIELD, binding!(state.text))
                .vertical_align(state.vertical_alignment)
                .build(app)
                .width(160.)
                .height(90.)
        }

        let mut state = State {
            text: TextState::new("text"),
            vertical_alignment: TextFieldVerticalAlignment::Top,
        };
        let mut pane = test_pane(PaneBuilder::new("test", view));

        let (frame, _) = pane.redraw(&mut state, 300, 200, 1.0);
        let top = text_origin_y(&frame);

        state.vertical_alignment = TextFieldVerticalAlignment::Center;
        let (frame, _) = pane.redraw(&mut state, 300, 200, 1.0);
        let center = text_origin_y(&frame);

        state.vertical_alignment = TextFieldVerticalAlignment::Bottom;
        let (frame, _) = pane.redraw(&mut state, 300, 200, 1.0);
        let bottom = text_origin_y(&frame);

        assert!(top < center);
        assert!(center < bottom);
    }

    #[test]
    fn supports_single_and_multi_line_configurations() {
        struct State {
            single: TextState,
            multi: TextState,
        }

        const SINGLE: u64 = 9001;
        const MULTI: u64 = 9002;

        fn view<'a>(state: &'a State, app: &mut PaneState) -> View<'a, State> {
            column(vec![
                text_field(SINGLE, binding!(state.single))
                    .build(app)
                    .width(140.)
                    .height(40.),
                text_field(MULTI, binding!(state.multi))
                    .multiline()
                    .build(app)
                    .width(140.)
                    .height(40.),
            ])
        }

        let mut state = State {
            single: TextState::new(""),
            multi: TextState::new(""),
        };
        let mut pane = test_pane(PaneBuilder::new("test", view));
        pane.redraw(&mut state, 300, 120, 1.0);

        pane.click(&mut state, pane.location(SINGLE).expect("single present"));
        pane.redraw(&mut state, 300, 120, 1.0);
        pane.key_pressed(&mut state, NamedKey::Enter);
        pane.key_pressed(&mut state, "a\nb");
        assert_eq!(state.single.text, "ab");

        pane.click(&mut state, pane.location(MULTI).expect("multi present"));
        pane.redraw(&mut state, 300, 120, 1.0);
        pane.key_pressed(&mut state, NamedKey::Enter);
        pane.key_pressed(&mut state, "a\nb");
        assert_eq!(state.multi.text, "\na\nb");
    }

    #[test]
    fn wrapped_multiline_does_not_scroll_horizontally_for_cursor_width() {
        struct State {
            text: TextState,
        }

        const FIELD: u64 = 9011;

        fn view<'a>(state: &'a State, app: &mut PaneState) -> View<'a, State> {
            text_field(FIELD, binding!(state.text))
                .multiline()
                .wrap()
                .build(app)
                .width(80.)
                .height(70.)
        }

        let mut state = State {
            text: TextState::new(""),
        };
        let mut pane = test_pane(PaneBuilder::new("test", view));
        pane.redraw(&mut state, 200, 120, 1.0);

        let location = pane.location(FIELD).expect("field present");
        pane.click(&mut state, location);
        pane.redraw(&mut state, 200, 120, 1.0);
        for _ in 0..30 {
            pane.key_pressed(&mut state, "w");
        }
        pane.redraw(&mut state, 200, 120, 1.0);

        pane.move_to(&mut state, location);
        pane.scroll(&mut state, ScrollDelta { x: -30., y: 0. });

        assert_eq!(state.text.viewport.x, 0.);
    }

    #[test]
    fn enter_end_editing_only_applies_to_focused_field() {
        #[derive(Default)]
        struct State {
            multiline: TextState,
            enter_done: TextState,
        }

        const MULTI: u64 = 9007;
        const ENTER_DONE: u64 = 9008;

        fn view<'a>(state: &'a State, app: &mut PaneState) -> View<'a, State> {
            column(vec![
                text_field(MULTI, binding!(state.multiline))
                    .multiline()
                    .build(app)
                    .width(140.)
                    .height(40.),
                text_field(ENTER_DONE, binding!(state.enter_done))
                    .enter_end_editing()
                    .build(app)
                    .width(140.)
                    .height(40.),
            ])
        }

        let mut state = State::default();
        let mut pane = test_pane(PaneBuilder::new("test", view));
        pane.redraw(&mut state, 300, 120, 1.0);

        pane.click(&mut state, pane.location(MULTI).expect("multiline present"));
        pane.redraw(&mut state, 300, 120, 1.0);
        pane.key_pressed(&mut state, NamedKey::Enter);

        assert!(state.multiline.editing);
        assert!(!state.enter_done.editing);
        assert_eq!(state.multiline.text, "\n");
    }

    #[test]
    fn scrolls_on_text_overflow() {
        struct State {
            text: TextState,
        }

        const FIELD: u64 = 9001;

        fn view<'a>(state: &'a State, app: &mut PaneState) -> View<'a, State> {
            text_field(FIELD, binding!(state.text))
                .build(app)
                .width(60.)
                .height(32.)
        }

        let mut state = State {
            text: TextState::new("wwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwww"),
        };
        let mut pane = test_pane(PaneBuilder::new("test", view));
        let (frame, _) = pane.redraw(&mut state, 300, 200, 1.0);
        let before = text_origin_x(&frame);

        pane.move_to(&mut state, pane.location(FIELD).expect("field present"));
        pane.scroll(&mut state, ScrollDelta { x: -80., y: 0. });
        let (frame, _) = pane.redraw(&mut state, 300, 200, 1.0);
        let after = text_origin_x(&frame);

        assert!(after < before);
    }

    #[test]
    fn supports_programmatic_focus_changes() {
        struct State {
            text: TextState,
        }

        const FIELD: u64 = 9002;
        const START: u64 = 9003;
        const END: u64 = 9004;

        fn view<'a>(state: &'a State, app: &mut PaneState) -> View<'a, State> {
            column(vec![
                text_field(FIELD, binding!(state.text))
                    .build(app)
                    .width(140.)
                    .height(40.),
                rect(START)
                    .fill(crate::TRANSPARENT)
                    .view()
                    .gesture(gesture::click(id!(START, 1u64)).run(
                        |state: &mut State, app, event| {
                            if event.state == ClickPhase::Completed {
                                state.text.begin_editing(app);
                            }
                        },
                    ))
                    .build(app)
                    .width(140.)
                    .height(40.),
                rect(END)
                    .fill(crate::TRANSPARENT)
                    .view()
                    .gesture(
                        gesture::click(id!(END, 1u64)).run(|state: &mut State, app, event| {
                            if event.state == ClickPhase::Completed {
                                state.text.end_editing(app);
                            }
                        }),
                    )
                    .build(app)
                    .width(140.)
                    .height(40.),
            ])
        }

        let mut state = State {
            text: TextState::new("hello"),
        };
        let mut pane = test_pane(PaneBuilder::new("test", view));
        pane.redraw(&mut state, 300, 200, 1.0);

        pane.click(&mut state, pane.location(START).expect("start present"));
        pane.redraw(&mut state, 300, 200, 1.0);
        assert!(state.text.editing);

        pane.click(&mut state, pane.location(END).expect("end present"));
        pane.redraw(&mut state, 300, 200, 1.0);
        assert!(!state.text.editing);
    }

    #[test]
    fn escape_does_not_end_editing_by_default() {
        #[derive(Default)]
        struct State {
            text: TextState,
        }

        const FIELD: u64 = 9005;

        fn view<'a>(state: &'a State, app: &mut PaneState) -> View<'a, State> {
            text_field(FIELD, binding!(state.text))
                .build(app)
                .width(140.)
                .height(40.)
        }

        let mut state = State::default();
        let mut pane = test_pane(PaneBuilder::new("test", view));
        pane.redraw(&mut state, 300, 200, 1.0);

        pane.click(&mut state, pane.location(FIELD).expect("field present"));
        pane.redraw(&mut state, 300, 200, 1.0);
        pane.key_pressed(&mut state, NamedKey::Escape);

        assert!(state.text.editing);
    }

    #[test]
    fn exposes_focus_state_per_text_field() {
        #[derive(Default)]
        struct State {
            a: TextState,
            b: TextState,
        }

        const A: u64 = 9003;
        const B: u64 = 9004;

        fn view<'a>(state: &'a State, app: &mut PaneState) -> View<'a, State> {
            column(vec![
                text_field(A, binding!(state.a))
                    .build(app)
                    .width(140.)
                    .height(40.),
                text_field(B, binding!(state.b))
                    .build(app)
                    .width(140.)
                    .height(40.),
            ])
        }

        let mut state = State::default();
        let mut pane = test_pane(PaneBuilder::new("test", view));
        pane.redraw(&mut state, 300, 200, 1.0);

        pane.click(&mut state, pane.location(A).expect("field a present"));
        assert!(state.a.editing);
        assert!(!state.b.editing);

        pane.click(&mut state, pane.location(B).expect("field b present"));
        assert!(!state.a.editing);
        assert!(state.b.editing);
    }

    #[test]
    fn enables_redacted_input() {
        struct State {
            text: TextState,
            edits: Vec<EditInteraction>,
        }

        const FIELD: u64 = 9005;

        fn view<'a>(state: &'a State, app: &mut PaneState) -> View<'a, State> {
            text_field(FIELD, binding!(state.text))
                .display(|state| "*".repeat(state.text.chars().count()))
                .on_edit(|state, _, edit| state.edits.push(edit))
                .build(app)
                .width(140.)
                .height(40.)
        }

        let mut state = State {
            text: TextState::new(""),
            edits: Vec::new(),
        };
        let mut pane = test_pane(PaneBuilder::new("test", view));
        pane.redraw(&mut state, 300, 200, 1.0);
        pane.click(&mut state, pane.location(FIELD).expect("field present"));
        pane.redraw(&mut state, 300, 200, 1.0);

        pane.key_pressed(&mut state, "a");
        let (frame, _) = pane.redraw(&mut state, 300, 200, 1.0);
        let (text_x, text_width) = frame
            .items
            .iter()
            .find_map(|item| match item {
                RenderItem::Layout { layout, transform } => {
                    Some((transform.as_coeffs()[4] as f32, layout.full_width()))
                }
                _ => None,
            })
            .expect("text layout rendered");
        let cursor_x = frame
            .items
            .iter()
            .find_map(|item| match item {
                RenderItem::Path { area, .. } if area.width <= 2. && area.height > 1. => {
                    Some(area.x)
                }
                _ => None,
            })
            .expect("cursor rendered");
        pane.key_pressed(&mut state, "b");
        pane.redraw(&mut state, 300, 200, 1.0);
        pane.key_pressed(&mut state, NamedKey::Backspace);
        pane.key_pressed(&mut state, NamedKey::ArrowLeft);
        pane.key_pressed(&mut state, "z");

        assert!(cursor_x > text_x + text_width * 0.5);
        assert_eq!(state.text.text, "za");
        assert!(matches!(
            state.edits.last(),
            Some(EditInteraction::Update(text)) if text == "za"
        ));
    }

    #[test]
    fn displayed_text_copy_does_not_expose_backing_text() {
        struct State {
            text: TextState,
        }

        const FIELD: u64 = 9012;

        fn view<'a>(state: &'a State, app: &mut PaneState) -> View<'a, State> {
            text_field(FIELD, binding!(state.text))
                .display(|state| "*".repeat(state.text.chars().count()))
                .build(app)
                .width(140.)
                .height(40.)
        }

        let mut state = State {
            text: TextState::new("secret"),
        };
        let mut pane = test_pane(PaneBuilder::new("test", view));
        pane.redraw(&mut state, 300, 200, 1.0);
        state
            .text
            .begin_editing_with(&mut pane.pane_state, InitialSelection::All);
        pane.redraw(&mut state, 300, 200, 1.0);

        pane.modifiers_changed(Modifiers::from_pressed([action_modifier()]));
        pane.key_pressed(&mut state, "c");

        assert_eq!(state.text.text, "secret");
    }

    #[test]
    fn displayed_text_cut_does_not_expose_or_delete_backing_text() {
        struct State {
            text: TextState,
        }

        const FIELD: u64 = 9013;

        fn view<'a>(state: &'a State, app: &mut PaneState) -> View<'a, State> {
            text_field(FIELD, binding!(state.text))
                .display(|state| "*".repeat(state.text.chars().count()))
                .build(app)
                .width(140.)
                .height(40.)
        }

        let mut state = State {
            text: TextState::new("secret"),
        };
        let mut pane = test_pane(PaneBuilder::new("test", view));
        pane.redraw(&mut state, 300, 200, 1.0);
        state
            .text
            .begin_editing_with(&mut pane.pane_state, InitialSelection::All);
        pane.redraw(&mut state, 300, 200, 1.0);

        pane.modifiers_changed(Modifiers::from_pressed([action_modifier()]));
        pane.key_pressed(&mut state, "x");

        assert_eq!(state.text.text, "secret");
    }

    #[test]
    fn enables_filtered_input() {
        struct State {
            text: TextState,
        }

        const FIELD: u64 = 9006;

        fn view<'a>(state: &'a State, app: &mut PaneState) -> View<'a, State> {
            text_field(FIELD, binding!(state.text))
                .on_edit(|state, _, edit| {
                    if let EditInteraction::Update(text) = edit {
                        state.text = TextState::new(
                            text.chars()
                                .filter(|character| character.is_ascii_digit())
                                .collect::<String>(),
                        );
                    }
                })
                .build(app)
                .width(140.)
                .height(40.)
        }

        let mut state = State {
            text: TextState::new(""),
        };
        let mut pane = test_pane(PaneBuilder::new("test", view));
        pane.redraw(&mut state, 300, 200, 1.0);
        pane.click(&mut state, pane.location(FIELD).expect("field present"));
        pane.redraw(&mut state, 300, 200, 1.0);

        pane.key_pressed(&mut state, "a");
        pane.redraw(&mut state, 300, 200, 1.0);
        pane.key_pressed(&mut state, "1");

        assert_eq!(state.text.text, "1");
    }
}
