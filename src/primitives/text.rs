use crate::app::{LayoutCache, PaneState, View};
use crate::brush_source::BrushSource;
use crate::render::{RenderItem, TextRenderLayout};
use crate::view::{Drawable, DrawableType};
use crate::{DEFAULT_FG_COLOR, DEFAULT_FONT_FAMILY, DEFAULT_FONT_SIZE};
use backer::{Area, Layout};
use parley::{
    Alignment, AlignmentOptions, FontContext, FontStack, FontWeight, Layout as ParleyLayout,
    LayoutContext, LineHeight, StyleProperty, TextStyle,
};
use std::fmt::Debug;
use std::ops::Range;
use kurbo::{Affine, Rect};
use parley::{Affinity, Cursor};
use peniko::Brush;

pub fn text(id: u64, text: impl AsRef<str>) -> Text {
    Text {
        id,
        string: text.as_ref().to_string(),
        font_size: DEFAULT_FONT_SIZE,
        font_weight: FontWeight::NORMAL,
        font_family: Some(DEFAULT_FONT_FAMILY.to_string()),
        fill: BrushSource::Static(Brush::Solid(DEFAULT_FG_COLOR)),
        alignment: Alignment::Center,
        line_height: 1.,
        wrap: false,
        styles: Vec::new(),
        backgrounds: Vec::new(),
    }
}

pub fn rich_text(id: u64, spans: impl IntoIterator<Item = Span>) -> Text {
    let mut string = String::new();
    let mut styles = Vec::new();
    let mut backgrounds = Vec::new();
    for span in spans {
        let start = string.len();
        string.push_str(&span.text);
        let end = string.len();
        for prop in span.styles {
            styles.push((start..end, prop));
        }
        if let Some(bg) = span.background {
            backgrounds.push((start..end, bg));
        }
    }
    let mut t = self::text(id, string);
    t.styles = styles;
    t.backgrounds = backgrounds;
    t
}

pub fn span(text: impl Into<String>) -> Span {
    Span {
        text: text.into(),
        styles: Vec::new(),
        background: None,
    }
}

pub struct Span {
    text: String,
    styles: Vec<StyleProperty<'static, Brush>>,
    background: Option<Brush>,
}

impl Span {
    pub fn style(mut self, prop: StyleProperty<'static, Brush>) -> Self {
        self.styles.push(prop);
        self
    }
    pub fn bold(self) -> Self {
        self.style(StyleProperty::FontWeight(FontWeight::BOLD))
    }
    pub fn italic(self) -> Self {
        self.style(StyleProperty::FontStyle(parley::FontStyle::Italic))
    }
    pub fn underline(self) -> Self {
        self.style(StyleProperty::Underline(true))
    }
    pub fn strikethrough(self) -> Self {
        self.style(StyleProperty::Strikethrough(true))
    }
    pub fn color(self, c: impl Into<Brush>) -> Self {
        self.style(StyleProperty::Brush(c.into()))
    }
    pub fn size(self, s: u32) -> Self {
        self.style(StyleProperty::FontSize(s as f32))
    }
    pub fn weight(self, w: FontWeight) -> Self {
        self.style(StyleProperty::FontWeight(w))
    }
    pub fn family(self, f: impl Into<String>) -> Self {
        self.style(StyleProperty::FontStack(FontStack::Single(
            parley::FontFamily::Named(f.into().into()),
        )))
    }
    pub fn background(mut self, b: impl Into<Brush>) -> Self {
        self.background = Some(b.into());
        self
    }
}

impl From<&str> for Span {
    fn from(s: &str) -> Self {
        span(s)
    }
}

impl From<String> for Span {
    fn from(s: String) -> Self {
        span(s)
    }
}

pub struct Text {
    pub(crate) id: u64,
    pub(crate) string: String,
    pub(crate) fill: BrushSource<()>,
    pub(crate) font_size: u32,
    pub(crate) font_weight: FontWeight,
    pub(crate) font_family: Option<String>,
    pub(crate) alignment: Alignment,
    pub(crate) line_height: f32,
    pub(crate) wrap: bool,
    pub(crate) styles: Vec<(Range<usize>, StyleProperty<'static, Brush>)>,
    pub(crate) backgrounds: Vec<(Range<usize>, Brush)>,
}

impl Debug for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Text")
            .field("id", &self.id)
            .field("string", &self.string)
            .field("fill", &self.fill)
            .field("font_size", &self.font_size)
            .field("font_weight", &self.font_weight)
            .field("alignment", &self.alignment)
            .field("line_height", &self.line_height)
            .field("wrap", &self.wrap)
            .field("styles", &self.styles.len())
            .field("backgrounds", &self.backgrounds.len())
            .finish()
    }
}

impl Clone for Text {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            string: self.string.clone(),
            fill: self.fill.clone(),
            font_size: self.font_size,
            font_weight: self.font_weight,
            font_family: self.font_family.clone(),
            alignment: self.alignment,
            line_height: self.line_height,
            wrap: self.wrap,
            styles: self.styles.clone(),
            backgrounds: self.backgrounds.clone(),
        }
    }
}

impl Text {
    pub fn fill(mut self, fill: impl Into<BrushSource<()>>) -> Self {
        self.fill = fill.into();
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
    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }
}

impl Text {
    pub fn view<State>(self) -> Drawable<State> {
        Drawable {
            view_type: DrawableType::Text(self),
            gesture_handlers: Vec::new(),
        }
    }
    pub fn build<State: 'static>(
        self,
        ctx: &mut PaneState,
    ) -> Layout<'static, View<State>, PaneState> {
        self.view().finish(ctx)
    }
}

pub struct TextLayout {
    pub(crate) layout_cache: LayoutCache,
    pub(crate) font_cx: FontContext,
    pub(crate) layout_cx: LayoutContext<Brush>,
}

impl TextLayout {
    pub(crate) fn new(
        layout_cache: LayoutCache,
        font_cx: FontContext,
        layout_cx: LayoutContext<Brush>,
    ) -> Self {
        Self {
            layout_cache,
            font_cx,
            layout_cx,
        }
    }

    pub(crate) fn build_layout(
        &mut self,
        text: &Text,
        current_fill: &Brush,
        available_width: f32,
        cache: bool,
    ) -> ParleyLayout<Brush> {
        let current_text = if text.string.is_empty() {
            " ".to_string()
        } else {
            text.string.clone()
        };

        if let Some((_, _, _, _, layout)) = self.layout_cache.get(&text.id).and_then(|cached| {
            cached.iter().find(|(t, styles, backgrounds, width, _)| {
                t == &current_text
                    && styles == &text.styles
                    && backgrounds == &text.backgrounds
                    && *width == available_width
            })
        }) {
            return layout.clone();
        }

        let font_family = text
            .font_family
            .clone()
            .unwrap_or_else(|| DEFAULT_FONT_FAMILY.to_string());
        let root_style = TextStyle {
            brush: current_fill.clone(),
            font_stack: FontStack::Single(parley::FontFamily::Named(font_family.into())),
            font_weight: text.font_weight,
            line_height: LineHeight::FontSizeRelative(text.line_height),
            font_size: text.font_size as f32,
            overflow_wrap: parley::OverflowWrap::Anywhere,
            ..Default::default()
        };

        let mut layout = if text.styles.is_empty() {
            let mut builder = self
                .layout_cx
                .tree_builder(&mut self.font_cx, 1., true, &root_style);
            builder.push_text(&current_text);
            builder.build().0
        } else {
            let mut builder =
                self.layout_cx
                    .ranged_builder(&mut self.font_cx, &current_text, 1., true);
            builder.push_default(StyleProperty::Brush(root_style.brush.clone()));
            builder.push_default(StyleProperty::FontStack(root_style.font_stack));
            builder.push_default(StyleProperty::FontWeight(root_style.font_weight));
            builder.push_default(StyleProperty::LineHeight(root_style.line_height));
            builder.push_default(StyleProperty::FontSize(root_style.font_size));
            builder.push_default(StyleProperty::OverflowWrap(root_style.overflow_wrap));
            for (range, prop) in &text.styles {
                builder.push(prop.clone(), range.clone());
            }
            builder.build(&current_text)
        };
        layout.break_all_lines(Some(available_width));
        layout.align(
            Some(available_width),
            text.alignment,
            AlignmentOptions {
                align_when_overflowing: true,
            },
        );

        if cache {
            let entry = self.layout_cache.entry(text.id).or_default();
            entry.push((
                current_text,
                text.styles.clone(),
                text.backgrounds.clone(),
                available_width,
                layout.clone(),
            ));
            if entry.len() > 2 {
                entry.remove(0);
            }
        }
        layout
    }
}

impl Text {
    pub(crate) fn render_layout(
        &self,
        scale_factor: f64,
        area: Area,
        app: &mut PaneState,
    ) -> TextRenderLayout {
        let fill = self.fill.resolve(area, &());
        let layout = app.text_layout.build_layout(self, &fill, area.width, true);
        let transform = Affine::translate((area.x as f64, area.y as f64)).then_scale(scale_factor);

        let backgrounds = self
            .backgrounds
            .iter()
            .flat_map(|(range, brush)| {
                let mut rects = Vec::new();
                if !range.is_empty() {
                    let anchor = Cursor::from_byte_index(&layout, range.start, Affinity::Downstream);
                    let focus = Cursor::from_byte_index(&layout, range.end, Affinity::Upstream);
                    parley::Selection::new(anchor, focus).geometry_with(&layout, |bb, _| {
                        rects.push((Rect::new(bb.x0, bb.y0, bb.x1, bb.y1), brush.clone()));
                    });
                }
                rects
            })
            .collect();

        TextRenderLayout {
            transform,
            layout,
            backgrounds,
        }
    }

    pub(crate) fn render_item(&self, scale_factor: f64, area: Area, app: &mut PaneState) -> RenderItem {
        RenderItem::Text(self.render_layout(scale_factor, area, app))
    }
}

impl Text {
    pub(crate) fn with_text_constraints<State>(
        self,
        ctx: &mut PaneState,
        node: Layout<'static, View<State>, PaneState>,
    ) -> Layout<'static, View<State>, PaneState> {
        if self.wrap {
            node.dynamic_height(move |w, ctx| {
                let default_brush = Brush::Solid(crate::DEFAULT_FG_COLOR);
                ctx.text_layout
                    .build_layout(&self, &default_brush, w, true)
                    .height()
            })
        } else {
            let default_brush = Brush::Solid(crate::DEFAULT_FG_COLOR);
            let layout = ctx
                .text_layout
                .build_layout(&self, &default_brush, 10000., true);
            node.height(layout.height()).width(layout.width().max(10.))
        }
    }
}
