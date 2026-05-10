use crate::primitives::{Image, PathData, Svg};
use crate::{Area, Color};
use kurbo::{Affine, BezPath};
use parley::Layout as TextLayout;
use peniko::{self, Brush};
use std::ops::Range;

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
}

pub struct TextRenderLayout {
    pub transform: Affine,
    pub layout: TextLayout<Brush>,
    pub backgrounds: Vec<(Range<usize>, Brush)>,
}
