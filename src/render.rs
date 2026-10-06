use crate::{Area, Color};
use kurbo::{Affine, BezPath, Rect, Stroke, Vec2};
use parley::Layout as TextLayout;
use peniko::{self, Brush};

pub struct Frame {
    pub base_color: Color,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
    pub items: Vec<RenderItem>,
}

pub trait FramePainter {
    type Output;
    fn paint(&mut self, frame: &Frame) -> Self::Output;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Effect {
    Blur {
        radius: f32,
    },
    DropShadow {
        offset: Vec2,
        blur: f32,
        color: Color,
    },
}

pub enum RenderItem {
    PushLayer {
        path: BezPath,
        blend: peniko::BlendMode,
        alpha: f32,
        effect: Option<Effect>,
    },
    PopLayer,
    Text(TextRenderLayout),
    Layout {
        layout: TextLayout<Brush>,
        transform: Affine,
    },
    Path {
        path: BezPath,
        fill: Option<Brush>,
        stroke: Option<(Brush, Stroke)>,
        area: Area,
    },
    Svg {
        resource_id: u64,
        content: String,
        fill: Option<Brush>,
        area: Area,
    },
    Image {
        image: peniko::ImageData,
        area: Area,
    },
    Shadow {
        rect: Rect,
        color: Color,
        blur: f64,
        corner_rounding: f64,
    },
}

pub struct TextRenderLayout {
    pub transform: Affine,
    pub layout: TextLayout<Brush>,
    pub backgrounds: Vec<(Rect, Brush)>,
}
