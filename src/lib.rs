#![allow(clippy::type_complexity, clippy::too_many_arguments)]

mod app;
mod background_style;
mod button;
mod circle;
mod draw_layout;
mod dropdown;
mod editor;
mod event;
mod gestures;
mod image;
mod models;
mod path;
mod rect;
mod scroller;
mod shape;
mod slider;
mod svg;
mod text;
mod text_field;
mod toggle;
mod view;

pub use app::{App, AppBuilder, AppCtx, AppState, Callback, RedrawTrigger, View, Window};
pub use backer::{Area, Layout, nodes::*};
pub use background_style::BrushSource;
pub use button::*;
pub use bytemuck;
pub use circle::circle;
pub use dropdown::*;
pub use gestures::{ClickState, DragState, EditInteraction, GestureHandler, GestureState};
pub use image::{ImageSource, image, image_from_bytes, image_from_path};
pub use parley::{Alignment, FontWeight};
pub use path::path;
pub use rect::rect;
pub use scroller::*;
pub use slider::*;
pub use svg::svg;
pub use text::*;
pub use text_field::*;
pub use toggle::*;
use vello_svg::vello::peniko::color::AlphaColor;
use vello_svg::vello::peniko::color::Srgb;
pub use view::{BlendMode, Compositing, const_hash, rect_path, rounded_rect_path, scope};

pub mod keyboard {
    pub use winit::keyboard::NamedKey;
}
pub mod window {
    pub use winit::window::WindowId;
}

pub use vello_svg::vello::kurbo::{BezPath, Cap, Join, Point, Stroke};
pub use vello_svg::vello::peniko::{Brush, Gradient};

pub use models::*;

pub type Color = AlphaColor<Srgb>;

pub(crate) fn adjust_brush(brush: &Brush, depressed: bool, hovered: bool) -> Brush {
    match brush {
        Brush::Solid(color) => {
            let adjusted = match (depressed, hovered) {
                (true, _) => color.map_lightness(|l| l - 0.1),
                (false, true) => color.map_lightness(|l| l + 0.1),
                (false, false) => *color,
            };
            Brush::Solid(adjusted)
        }
        other => other.clone(),
    }
}

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
