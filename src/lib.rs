#![allow(clippy::type_complexity, clippy::too_many_arguments)]

mod brush_source;
#[cfg(feature = "platform-winit")]
mod draw_layout;
mod editor;
mod gestures;
mod models;
mod pane;
mod platforms;
mod prebuilts;
mod primitives;
pub mod render;
mod renderers;
mod utils;
mod view;

#[cfg(test)]
mod tests;

#[cfg(feature = "platform-winit")]
pub use platforms::winit;

pub use backer::{Area, Layout, nodes::*};
pub use brush_source::BrushSource;
pub use gestures::{
    ButtonPredicate, ClickEvent, ClickLocation, ClickPhase, DragPhase, EditInteraction, Gesture,
    GestureId, KeyEvent, KeyPhase, KeyPredicate, ModifierPredicate, MouseButton, ScrollDelta,
    gesture,
};
pub use pane::{Pane, PaneBuilder, PaneEffect, PaneState, PaneWaker, View};
pub use parley::{Alignment, FontWeight, StyleProperty};
use peniko::color::AlphaColor;
use peniko::color::Srgb;
pub use prebuilts::*;
pub use primitives::{
    Circle, Image, ImageSource, Path, Rect, Shadow, Span, Svg, Text, circle, image,
    image_from_bytes, image_from_path, path, rect, rich_text, shadow, span, svg, text,
};
pub use view::{
    BlendMode, Compositing, Drawable, combine_id, const_hash, owned_scope, rect_path,
    rounded_rect_path, scope,
};

pub use kurbo::{BezPath, Cap, Join, Point, Stroke};
pub use peniko::{Brush, Gradient};

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
