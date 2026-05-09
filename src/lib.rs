#![allow(clippy::type_complexity, clippy::too_many_arguments)]

mod app;
mod brush_source;
mod draw_layout;
mod editor;
mod gestures;
mod models;
pub mod platform;
mod prebuilts;
mod primitives;
mod utils;
mod view;

pub use platform::winit;

pub(crate) use app::Pane;
pub use app::{PaneConfig, PaneEffect, PaneState, Redraw, RedrawTrigger, View};
pub use backer::{Area, Layout, nodes::*};
pub use brush_source::BrushSource;
pub use bytemuck;
pub use gestures::{
    ClickState, DragState, EditInteraction, GestureHandler, GestureState, ScrollDelta,
};
pub use parley::{Alignment, FontWeight, StyleProperty};
pub use prebuilts::*;
pub use primitives::{
    ImageSource, Span, Text, circle, image, image_from_bytes, image_from_path, path, rect,
    rich_text, span, svg, text,
};
use vello_svg::vello::peniko::color::AlphaColor;
use vello_svg::vello::peniko::color::Srgb;
pub use view::{
    BlendMode, Compositing, const_hash, owned_scope, rect_path, rounded_rect_path, scope,
};

pub use vello_svg::vello::kurbo::{BezPath, Cap, Join, Point, Stroke};
pub use vello_svg::vello::peniko::{Brush, Gradient};

pub use models::*;

pub type Color = AlphaColor<Srgb>;

const RUBIK_FONT: &[u8] = include_bytes!("../assets/Rubik-VariableFont_wght.ttf");
const DEFAULT_FONT_FAMILY: &str = "Rubik";
pub const DEFAULT_STROKE_WIDTH: f32 = 1.;
pub const DEFAULT_PADDING: f32 = 5.;
pub const DEFAULT_CORNER_ROUNDING: f32 = 6.;
pub const DEFAULT_FONT_SIZE: u32 = 14;
pub const DEFAULT_FG_COLOR: Color = AlphaColor::WHITE;
pub const DEFAULT_PURP: Color = AlphaColor::from_rgb8(113, 70, 232);
pub const DEFAULT_DARK_GRAY: Color = AlphaColor::from_rgb8(30, 30, 30);
pub const DEFAULT_GRAY: Color = AlphaColor::from_rgb8(50, 50, 50);
pub const DEFAULT_LIGHT_GRAY: Color = AlphaColor::from_rgb8(113, 70, 232);
pub const DEFAULT_FG: Color = Color::from_rgb8(230, 230, 230);
pub const TRANSPARENT: Color = Color::TRANSPARENT;
