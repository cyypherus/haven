use crate::primitives::ImageSource;
use crate::{Area, Color};
use accesskit::TreeUpdate;
use kurbo::{Affine, BezPath, Rect, Stroke};
use parley::Layout as TextLayout;
use peniko::{self, Brush};

pub struct Frame {
    pub base_color: Color,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
    pub items: Vec<RenderItem>,
    pub semantics: TreeUpdate,
}

#[cfg(any(feature = "platform-winit", feature = "paint-anyrender"))]
pub use crate::renderers::anyrender::FramePainter;

pub trait FrameOutput {
    type Output;

    fn render(&mut self, frame: &Frame) -> Self::Output;
}

impl<F, T> FrameOutput for F
where
    F: FnMut(&Frame) -> T,
{
    type Output = T;

    fn render(&mut self, frame: &Frame) -> T {
        self(frame)
    }
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
        path: BezPath,
        area: Area,
        fill: Option<Brush>,
        stroke: Option<(Brush, Stroke)>,
    },
    Svg {
        cache_key: u64,
        content: String,
        area: Area,
        unlocked_aspect_ratio: bool,
        fill: Option<Brush>,
    },
    Image {
        cache_key: u64,
        source: ImageSource,
        area: Area,
        unlocked_aspect_ratio: bool,
        corner_rounding: f32,
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
