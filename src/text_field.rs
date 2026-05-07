use crate::app::{EditState, PaneState, View};
use crate::background_style::BrushSource;
use crate::shape::{PathData, rect_path};
use crate::view::DrawableType;
use crate::{
    Binding, DEFAULT_CORNER_ROUNDING, DEFAULT_FG_COLOR, DEFAULT_FONT_FAMILY, DEFAULT_FONT_SIZE,
    DEFAULT_PADDING, DEFAULT_PURP, DEFAULT_STROKE_WIDTH, EditInteraction, Key, NamedKey, Text,
    rect,
};
use backer::{Area, Layout, nodes::*};
use parley::{Alignment, FontWeight};
use std::fmt::Debug;
use std::rc::Rc;
use vello_svg::vello::kurbo::{Affine, Rect as KRect, Stroke};
use vello_svg::vello::peniko::color::palette::css::TRANSPARENT;
use vello_svg::vello::peniko::{Brush, Color};

#[derive(Debug, Clone, Default)]
pub struct TextState {
    pub text: String,
    pub editing: bool,
}

impl TextState {
    pub fn new(text: impl AsRef<str>) -> Self {
        Self {
            text: text.as_ref().to_string(),
            editing: false,
        }
    }
}

pub fn text_field<'a, State>(
    id: u64,
    state: (TextState, Binding<State, TextState>),
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
        cursor_fill: BrushSource::Static(Brush::Solid(DEFAULT_PURP)),
        highlight_fill: BrushSource::Static(Brush::Solid(DEFAULT_PURP)),
        on_edit: None,
        esc_end_editing: false,
        enter_end_editing: false,
    }
}

type BgViewFn<'a, State> =
    Rc<dyn Fn(&TextState, Area, &mut PaneState) -> Layout<'a, View<State>, PaneState> + 'a>;

pub struct TextField<'a, State> {
    pub(crate) id: u64,
    pub(crate) state: TextState,
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
            .field("on_edit", &self.on_edit.is_some())
            .finish()
    }
}

impl<State> Clone for TextField<'_, State> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            state: self.state.clone(),
            binding: self.binding.clone(),
            text_fill: self.text_fill.clone(),
            font_size: self.font_size,
            font_weight: self.font_weight,
            font_family: self.font_family.clone(),
            alignment: self.alignment,
            editable: self.editable,
            line_height: self.line_height,
            background: self.background.clone(),
            padding: self.padding,
            wrap: self.wrap,
            cursor_fill: self.cursor_fill.clone(),
            highlight_fill: self.highlight_fill.clone(),
            on_edit: self.on_edit.clone(),
            esc_end_editing: false,
            enter_end_editing: false,
        }
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
        let text_state = self.state.clone();
        let fill = self.text_fill.clone();
        let cursor_fill = self.cursor_fill.clone();
        let highlight_fill = self.highlight_fill.clone();
        let alignment = self.alignment;
        let line_height = self.line_height;
        let wrap = self.wrap;
        let root_id = crate::id!(id);
        let text_id = crate::id!(id);
        let text_content = if self.state.editing
            && let Some(ref mut edit_state) = ctx.editor
        {
            let cursor_width = 2f64;
            let half_cursor_width = 1f64;
            let selection_rects: Vec<vello_svg::vello::kurbo::Rect> = edit_state
                .editor
                .editor
                .selection_geometry()
                .iter()
                .map(|(bb, _i)| KRect::new(bb.x0, bb.y0, bb.x1, bb.y1))
                .collect();
            let is_empty = edit_state.editor.text().to_string().is_empty();
            let cursor = edit_state
                .editor
                .editor
                .cursor_geometry(cursor_width as f32)
                .map(|bb| KRect::new(bb.x0, bb.y0, bb.x1, bb.y1));
            let layout = edit_state
                .editor
                .editor
                .layout(&mut ctx.font_cx, &mut ctx.layout_cx)
                .clone();
            let width = layout.width();
            let height = layout.height();
            let scale_factor = ctx.scale_factor;

            let mut selection_drawables = Vec::new();
            for rect in selection_rects.clone() {
                let highlight = highlight_fill.clone();
                let ts = text_state.clone();
                selection_drawables.push(draw(move |area, _| {
                    let resolved_area = Area {
                        x: area.x + rect.x0 as f32,
                        y: area.y + rect.y0 as f32,
                        width: rect.width() as f32,
                        height: rect.height() as f32,
                    };
                    vec![View::Draw {
                        view: Box::new(DrawableType::Path(Box::new(PathData {
                            id,
                            builder: rect_path((2., 2., 2., 2.)),
                            fill: Some(highlight.resolve(resolved_area, &ts).into()),
                            stroke: None,
                        }))),
                        gesture_handlers: Vec::new(),
                        area: resolved_area,
                    }]
                }));
            }

            let has_selection = !selection_rects.is_empty();

            let mut cursor_drawables = Vec::new();
            if !has_selection
                && let Some(cursor) = if is_empty {
                    Some(vello_svg::vello::kurbo::Rect::new(
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
                    vec![View::<State>::Draw {
                        view: Box::new(DrawableType::Path(Box::new(PathData {
                            id,
                            builder: rect_path((rounding, rounding, rounding, rounding)),
                            fill: Some(cursor_fill.resolve(resolved_area, &ts).into()),
                            stroke: None,
                        }))),
                        gesture_handlers: Vec::new(),
                        area: resolved_area,
                    }]
                }));
            }

            let mut text_drawables = Vec::new();

            text_drawables.push(draw(move |area, _| {
                let transform =
                    Affine::translate((area.x as f64, area.y as f64)).then_scale(scale_factor);
                vec![View::<State>::Draw {
                    view: Box::new(DrawableType::Layout(Box::new((layout.clone(), transform)))),
                    gesture_handlers: Vec::new(),
                    area: Area {
                        x: area.x,
                        y: area.y,
                        width: area.width,
                        height: area.height,
                    },
                }]
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
            Text {
                id: text_id,
                string: self.state.text.clone(),
                font_size,
                font_weight,
                font_family: font_family.clone(),
                fill: if self.state.editing {
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
            .finish(ctx)
        };
        let content = stack(vec![
            draw(move |area, _| vec![View::EditorArea(root_id, area)]).inert(),
            text_content,
        ])
        .pad(if editable { self.padding } else { 0. });
        let sensor = {
            let binding = binding.clone();
            let font_family = self.font_family.clone();
            let on_edit = self.on_edit.clone();
            stack(vec![
                rect(root_id)
                    .fill(TRANSPARENT)
                    .view()
                    .on_key({
                        let on_edit = on_edit.clone();
                        let binding = binding.clone();
                        move |state, app, key| {
                            if (self.enter_end_editing && key == Key::Named(NamedKey::Enter))
                                || (self.esc_end_editing && key == Key::Named(NamedKey::Escape))
                            {
                                app.end_editing();
                                binding.update(state, |s| s.editing = false);
                                if let Some(ref on_edit) = on_edit {
                                    (on_edit)(state, app, EditInteraction::End);
                                }
                                return;
                            };
                            if let PaneState {
                                editor: Some(EditState { editor, id, .. }),
                                layout_cx,
                                font_cx,
                                modifiers,
                                ..
                            } = app
                                && *id == root_id
                            {
                                editor.handle_key(key.clone(), layout_cx, font_cx, *modifiers);
                            }
                            let edit_text =
                                app.editor.as_ref().map(|e| e.editor.text().to_string());

                            if let Some(edit_text) = edit_text
                                && app.editor.as_ref().map(|e| e.id) == Some(root_id)
                            {
                                if let Some(ref on_edit) = on_edit {
                                    on_edit(state, app, EditInteraction::Update(edit_text.clone()));
                                }
                                binding.update(state, |s| {
                                    s.text = edit_text.clone();
                                });
                            }
                        }
                    })
                    .on_click_outside({
                        let binding = binding.clone();
                        let on_edit = on_edit.clone();
                        move |state: &mut State, app, _, _| {
                            if let PaneState {
                                editor: Some(EditState { id, .. }),
                                ..
                            } = app
                                && *id == root_id
                            {
                                app.end_editing();
                                binding.update(state, |s| s.editing = false);
                                if let Some(ref on_edit) = on_edit {
                                    (on_edit)(state, app, EditInteraction::End);
                                }
                            }
                        }
                    })
                    .on_click({
                        let binding = binding.clone();
                        let font_family = font_family.clone();
                        move |state: &mut State, app, _, _| {
                            let editing = binding.get(state).editing;
                            if !editing && app.editor.is_none() {
                                binding.update(state, |s| s.editing = true);
                                let editor_area =
                                    app.editor_areas.get(&root_id).copied().unwrap_or(Area {
                                        x: 0.,
                                        y: 0.,
                                        width: 0.,
                                        height: 0.,
                                    });
                                let ts = binding.get(state);
                                let text = ts.text.clone();
                                app.begin_editing(
                                    root_id,
                                    text,
                                    self.text_fill.resolve(editor_area, &ts),
                                    font_family
                                        .clone()
                                        .unwrap_or(DEFAULT_FONT_FAMILY.to_string()),
                                    self.font_weight,
                                    self.line_height,
                                    self.font_size as f32,
                                    parley::OverflowWrap::Anywhere,
                                    self.alignment,
                                    self.cursor_fill.resolve(editor_area, &ts),
                                    self.highlight_fill.resolve(editor_area, &ts),
                                    self.wrap,
                                );
                            }
                        }
                    })
                    .finish(ctx),
            ])
        }
        .inert();
        let background_fn = self.background;
        let ts = self.state.clone();
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
