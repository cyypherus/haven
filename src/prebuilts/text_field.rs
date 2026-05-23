use crate::brush_source::BrushSource;
use crate::editor::Editor;
use crate::pane::{PaneState, View};
use crate::primitives::{
    Text,
    shape::{PathData, rect_path},
};
use crate::view::DrawableType;
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

#[derive(Clone)]
pub struct TextState {
    pub text: String,
    pub(crate) editor: Editor,
    pub(crate) edit_request: TextEditRequest,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TextEditRequest {
    editing: Option<bool>,
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
        self.request_editing(true, app);
    }

    pub fn end_editing(&mut self, app: &mut PaneState) {
        self.request_editing(false, app);
    }

    fn request_editing(&mut self, editing: bool, app: &mut PaneState) {
        self.edit_request = TextEditRequest {
            editing: Some(editing),
        };
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

fn text_field_local_point(area: Area, point: crate::Point) -> crate::Point {
    crate::Point::new(point.x - area.x as f64, point.y - area.y as f64)
}

fn begin_text_field_editing(app: &mut PaneState, root_id: u64) -> bool {
    let started_editing = app.editor.as_ref().map(|e| e.id) != Some(root_id);
    if started_editing {
        app.begin_editing(root_id);
        app.request_redraw();
    }
    started_editing
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
        let text_id = crate::id!(id);
        let edit_request = self.state.edit_request;
        ctx.sync_text_edit_request(root_id, edit_request.editing);
        let is_active_editor = ctx.editor.as_ref().is_some_and(|edit| edit.id == root_id);
        let text_content = if is_active_editor {
            let editor_area = ctx.editor_areas.get(&root_id).copied().unwrap_or(Area {
                x: 0.,
                y: 0.,
                width: 0.,
                height: 0.,
            });
            let mut editor = self.state.editor.clone();
            style_text_field_editor(
                &mut editor,
                &self.state.text,
                fill.resolve(editor_area, &text_state),
                font_family
                    .clone()
                    .unwrap_or(DEFAULT_FONT_FAMILY.to_string()),
                font_weight,
                line_height,
                font_size as f32,
                alignment,
                if wrap { Some(editor_area.width) } else { None },
            );
            let cursor_width = 1.5f64;
            let half_cursor_width = cursor_width * 0.5;
            let selection_rects: Vec<kurbo::Rect> = editor
                .editor
                .selection_geometry()
                .iter()
                .map(|(bb, _i)| KRect::new(bb.x0, bb.y0, bb.x1, bb.y1))
                .collect();
            let is_empty = editor.text().to_string().is_empty();
            let cursor = editor
                .editor
                .cursor_geometry(cursor_width as f32)
                .map(|bb| KRect::new(bb.x0, bb.y0, bb.x1, bb.y1));
            let layout = editor
                .editor
                .layout(&mut ctx.font_cx, &mut ctx.layout_cx)
                .clone();
            let width = layout.width();
            let height = layout.height();
            let scale_factor = ctx.scale_factor;

            let mut selection_drawables = Vec::new();
            for (selection_index, rect) in selection_rects.clone().into_iter().enumerate() {
                let highlight = highlight_fill.clone();
                let ts = text_state.clone();
                let selection_id = crate::id!(id, selection_index as u64, 1u64);
                selection_drawables.push(draw(move |area, _| {
                    let resolved_area = Area {
                        x: area.x + rect.x0 as f32,
                        y: area.y + rect.y0 as f32,
                        width: rect.width() as f32,
                        height: rect.height() as f32,
                    };
                    vec![View::draw(
                        Box::new(DrawableType::Path(Box::new(PathData {
                            id: selection_id,
                            builder: rect_path((2., 2., 2., 2.)),
                            fill: Some(highlight.resolve(resolved_area, &ts).into()),
                            stroke: None,
                        }))),
                        resolved_area,
                    )]
                }));
            }

            let has_selection = !selection_rects.is_empty();

            let mut cursor_drawables = Vec::new();
            if !has_selection
                && let Some(cursor) = if is_empty {
                    Some(kurbo::Rect::new(
                        -half_cursor_width,
                        0.,
                        half_cursor_width,
                        0.,
                    ))
                } else {
                    cursor
                }
            {
                let rounding = (cursor_width * 0.5) as f32;
                let ts = text_state.clone();
                let cursor_id = crate::id!(id, 2u64);
                cursor_drawables.push(draw(move |area, _| {
                    let resolved_area = Area {
                        x: area.x + cursor.x0 as f32,
                        y: area.y + cursor.y0 as f32,
                        width: cursor.width() as f32,
                        height: if is_empty {
                            area.height
                        } else {
                            cursor.height() as f32
                        },
                    };
                    vec![View::<State>::draw(
                        Box::new(DrawableType::Path(Box::new(PathData {
                            id: cursor_id,
                            builder: rect_path((rounding, rounding, rounding, rounding)),
                            fill: Some(cursor_fill.resolve(resolved_area, &ts).into()),
                            stroke: None,
                        }))),
                        resolved_area,
                    )]
                }));
            }

            let mut text_drawables = Vec::new();

            text_drawables.push(draw(move |area, _| {
                let transform =
                    Affine::translate((area.x as f64, area.y as f64)).then_scale(scale_factor);
                vec![View::<State>::draw(
                    Box::new(DrawableType::Layout(Box::new((layout.clone(), transform)))),
                    Area {
                        x: area.x,
                        y: area.y,
                        width: area.width,
                        height: area.height,
                    },
                )]
            }));

            let stack = stack(
                [selection_drawables, text_drawables, cursor_drawables]
                    .into_iter()
                    .flatten()
                    .collect(),
            )
            .height(height);
            if wrap { stack } else { stack.width(width) }
        } else {
            let show_hint = self.state.text.trim().is_empty() && hint_text.is_some();
            Text {
                id: text_id,
                string: if show_hint {
                    hint_text.unwrap()
                } else {
                    self.state.text.clone()
                },
                font_size,
                font_weight,
                font_family: font_family.clone(),
                fill: if show_hint {
                    hint_fill.resolve_to_stateless(&text_state)
                } else if is_active_editor {
                    BrushSource::Static(Brush::Solid(TRANSPARENT))
                } else {
                    fill.resolve_to_stateless(&text_state)
                },
                alignment,
                line_height,
                wrap,
                styles: Vec::new(),
                backgrounds: Vec::new(),
            }
            .view()
            .build(ctx)
        };
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
                                    let local = |point| text_field_local_point(editor_area, point);
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
                                            ts.editor.mouse_moved(
                                                local(start_global),
                                                &mut app.layout_cx,
                                                &mut app.font_cx,
                                            );
                                            ts.editor.mouse_pressed(
                                                &mut app.layout_cx,
                                                &mut app.font_cx,
                                            );
                                        }
                                        DragPhase::Updated { current_global, .. } => {
                                            ts.editor.mouse_moved(
                                                local(current_global),
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
                                                ts.editor.mouse_moved(
                                                    local(current_global),
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
                                    let local = |point| text_field_local_point(editor_area, point);
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
                                            ts.editor.mouse_moved(
                                                local(event.location.global()),
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
                                                ts.editor.mouse_moved(
                                                    local(pos),
                                                    &mut app.layout_cx,
                                                    &mut app.font_cx,
                                                );
                                            }
                                            ts.editor.mouse_moved(
                                                local(event.location.global()),
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
