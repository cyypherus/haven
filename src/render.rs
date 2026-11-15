#[cfg(feature = "renderer")]
pub use crate::renderers::{HybridImageRenderer, HybridTextureRenderer};

use crate::primitives::{Image, Svg};
use crate::{Area, Color};
use kurbo::{Affine, BezPath, Rect, Stroke};
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
        effect: Option<LayerEffect>,
    },
    PopLayer,
    Text(TextRenderLayout),
    Layout {
        layout: TextLayout<Brush>,
        transform: Affine,
    },
    Path {
        id: u64,
        path: BezPath,
        fill: Option<Brush>,
        stroke: Option<(Brush, Stroke)>,
        area: Area,
    },
    Svg {
        svg: Svg,
        cache_key: u64,
        area: Area,
    },
    Image {
        image: Image,
        cache_key: u64,
        area: Area,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LayerEffect {
    Blur {
        radius: f32,
    },
    DropShadow {
        dx: f32,
        dy: f32,
        blur: f32,
        color: Color,
    },
}

pub struct TextRenderLayout {
    pub transform: Affine,
    pub layout: TextLayout<Brush>,
    pub backgrounds: Vec<(Rect, Brush)>,
}
