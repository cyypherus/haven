use crate::primitives::{Image, PathData, Shadow, Svg};
use crate::{Area, Color};
use kurbo::{Affine, BezPath, Rect};
use parley::Layout as TextLayout;
use peniko::{self, Brush};

pub struct Frame {
    pub base_color: Color,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
    pub items: Vec<RenderItem>,
}

pub enum RenderItem {
    PushLayer {
        path: BezPath,
        blend: peniko::BlendMode,
        alpha: f32,
    },
    PopLayer,
    Text(TextRenderLayout),
    Layout {
        layout: TextLayout<Brush>,
        transform: Affine,
    },
    Path {
        path: Box<PathData>,
        area: Area,
    },
    Svg {
        svg: Svg,
        area: Area,
    },
    Image {
        image: Image,
        area: Area,
    },
    Shadow {
        shadow: Shadow,
        area: Area,
    },
}

pub struct TextRenderLayout {
    pub transform: Affine,
    pub layout: TextLayout<Brush>,
    pub backgrounds: Vec<(Rect, Brush)>,
}
