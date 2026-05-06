use crate::app::{RootCtx, RootState, View};
use crate::gestures::{ClickLocation, Interaction, InteractionType, ScrollDelta};
use crate::image::Image;

use crate::shape::PathData;
use crate::svg::Svg;
use crate::text::Text;
use crate::{Binding, ClickState, DragState, GestureHandler, Key};
use backer::{Area, Layout, nodes::*};
use parley::Layout as TextLayout;
use std::rc::Rc;
use vello_svg::vello::kurbo::{Affine, BezPath};
use vello_svg::vello::peniko::{self, Brush};

// A simple const FNV-1a hash for our purposes
const FNV_OFFSET: u64 = 1469598103934665603;
const FNV_PRIME: u64 = 1099511628211;

pub const fn const_hash(s: &str, line: u32, col: u32) -> u64 {
    let mut hash = FNV_OFFSET;
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        i += 1;
    }
    // Incorporate the line and column numbers into the hash.
    hash ^= line as u64;
    hash = hash.wrapping_mul(FNV_PRIME);
    hash ^= col as u64;
    hash = hash.wrapping_mul(FNV_PRIME);
    hash
}

/// This macro computes a compile-time ID from the file, line, and column
/// where it's invoked, and at runtime combines it (via XOR) with another id.
#[macro_export]
macro_rules! id {
    // It would be good to explore using a TypeId for uniqueness instead of
    // the caller location. Currently we can't hash TypeId values at
    // compile time / in const contexts so the ids throughout the crate would
    // have to be changed to some Hashable struct with the unique token.
    () => {{
        const ID: u64 = $crate::const_hash(file!(), line!(), column!());
        ID
    }};
    ($other:expr) => {{
        const ID: u64 = $crate::const_hash(file!(), line!(), column!());
        ID ^ ($other)
    }};
    ($other:expr, $other2:expr) => {{
        const ID: u64 = $crate::const_hash(file!(), line!(), column!());
        ID ^ ($other) ^ (($other2).wrapping_mul(1099511628211))
    }};
}

#[macro_export]
macro_rules! binding {
    ($state_var:ident, $State:ty, $field:ident) => {
        (
            $state_var.$field.clone(),
            Binding::new(
                |s: &$State| s.$field.clone(),
                |s: &mut $State, value| s.$field = value,
            ),
        )
    };
}

#[macro_export]
macro_rules! scope {
    ($state_var:ident, $Root:ty, { $($field:ident),+ $(,)? } => $Sub:ident, $layout:expr) => {{
        let sub_state = $Sub {
            $( $field: $state_var.$field.clone(), )+
        };
        let binding = Binding::new(
            |s: &$Root| $Sub {
                $( $field: s.$field.clone(), )+
            },
            |s: &mut $Root, v: $Sub| {
                $( s.$field = v.$field; )+
            },
        );
        $crate::scope($layout(&sub_state), binding)
    }};
}

pub fn rect_path(area: Area) -> BezPath {
    use vello_svg::vello::kurbo::{Rect, Shape};
    Rect::new(
        area.x as f64,
        area.y as f64,
        (area.x + area.width) as f64,
        (area.y + area.height) as f64,
    )
    .to_path(0.1)
}

pub fn rounded_rect_path(area: Area, radius: f32) -> BezPath {
    use vello_svg::vello::kurbo::{Rect, RoundedRect, Shape};
    RoundedRect::from_rect(
        Rect::new(
            area.x as f64,
            area.y as f64,
            (area.x + area.width) as f64,
            (area.y + area.height) as f64,
        ),
        radius as f64,
    )
    .to_path(0.1)
}

#[derive(Clone, Copy, Debug)]
pub enum BlendMode {
    Normal,
    Additive,
    Screen,
    Multiply,
}

impl BlendMode {
    fn to_peniko(self) -> peniko::BlendMode {
        use vello_svg::vello::peniko::{Compose, Mix};
        match self {
            BlendMode::Normal => peniko::BlendMode::default(),
            BlendMode::Additive => peniko::BlendMode {
                mix: Mix::Normal,
                compose: Compose::Plus,
            },
            BlendMode::Screen => peniko::BlendMode {
                mix: Mix::Screen,
                compose: Compose::SrcOver,
            },
            BlendMode::Multiply => peniko::BlendMode {
                mix: Mix::Multiply,
                compose: Compose::SrcOver,
            },
        }
    }
}

pub trait Compositing<'a, State> {
    fn clipped(self, path: impl Fn(Area) -> BezPath + 'static) -> Self;
    fn blend(self, mode: BlendMode) -> Self;
    fn opacity(self, alpha: f32) -> Self;
}

impl<'a, State: 'static> Compositing<'a, State> for Layout<'a, View<State>, RootCtx> {
    fn clipped(self, path: impl Fn(Area) -> BezPath + 'static) -> Self {
        wrap_layer(self, path, peniko::BlendMode::default(), 1.0)
    }
    fn blend(self, mode: BlendMode) -> Self {
        wrap_layer(self, rect_path, mode.to_peniko(), 1.0)
    }
    fn opacity(self, alpha: f32) -> Self {
        wrap_layer(
            self,
            rect_path,
            peniko::BlendMode::default(),
            alpha.clamp(0., 1.),
        )
    }
}

fn wrap_layer<'a, State>(
    content: Layout<'a, View<State>, RootCtx>,
    path: impl Fn(Area) -> BezPath + 'static,
    blend: peniko::BlendMode,
    alpha: f32,
) -> Layout<'a, View<State>, RootCtx> {
    stack(vec![
        draw(move |area, _| {
            vec![View::PushLayer {
                path: path(area),
                blend,
                alpha,
            }]
        }),
        content,
        draw(|_, _| vec![View::PopLayer]),
    ])
}

pub struct Drawable<State> {
    pub(crate) view_type: DrawableType,
    pub(crate) gesture_handlers: Vec<GestureHandler<State, RootState>>,
}

pub enum DrawableType {
    Text(Text),
    Layout(Box<(TextLayout<Brush>, Affine)>),
    Path(Box<PathData>),
    Svg(Svg),
    Image(Image),
}

impl Clone for DrawableType {
    fn clone(&self) -> Self {
        match self {
            DrawableType::Text(text) => DrawableType::Text(text.clone()),
            DrawableType::Layout(boxed) => DrawableType::Layout(boxed.clone()),
            DrawableType::Path(path) => DrawableType::Path(path.clone()),
            DrawableType::Svg(svg) => DrawableType::Svg(svg.clone()),
            DrawableType::Image(image) => DrawableType::Image(image.clone()),
        }
    }
}

impl<State> Drawable<State> {
    pub fn on_click(
        mut self,
        f: impl Fn(&mut State, &mut RootState, ClickState, ClickLocation) + 'static,
    ) -> Self {
        self.gesture_handlers.push(GestureHandler {
            interaction_type: InteractionType {
                click: true,
                ..Default::default()
            },
            interaction_handler: Some(Rc::new(move |state, app_state, interaction| {
                let Interaction::Click(click, location) = interaction else {
                    return;
                };
                (f)(state, app_state, click, location);
            })),
        });
        self
    }
    pub fn on_click_outside(
        mut self,
        f: impl Fn(&mut State, &mut RootState, ClickState, ClickLocation) + 'static,
    ) -> Self {
        self.gesture_handlers.push(GestureHandler {
            interaction_type: InteractionType {
                click_outside: true,
                ..Default::default()
            },
            interaction_handler: Some(Rc::new(move |state, app_state, interaction| {
                let Interaction::ClickOutside(click, location) = interaction else {
                    return;
                };
                (f)(state, app_state, click, location);
            })),
        });
        self
    }
    pub fn on_drag(mut self, f: impl Fn(&mut State, &mut RootState, DragState) + 'static) -> Self {
        self.gesture_handlers.push(GestureHandler {
            interaction_type: InteractionType {
                drag: true,
                ..Default::default()
            },
            interaction_handler: Some(Rc::new(move |state, app_state, interaction| {
                let Interaction::Drag(drag) = interaction else {
                    return;
                };
                (f)(state, app_state, drag);
            })),
        });
        self
    }
    pub fn on_hover(mut self, f: impl Fn(&mut State, &mut RootState, bool) + 'static) -> Self {
        self.gesture_handlers.push(GestureHandler {
            interaction_type: InteractionType {
                hover: true,
                ..Default::default()
            },
            interaction_handler: Some(Rc::new(move |state, app_state, interaction| {
                let Interaction::Hover(hovered) = interaction else {
                    return;
                };
                (f)(state, app_state, hovered);
            })),
        });
        self
    }
    pub fn on_key(mut self, f: impl Fn(&mut State, &mut RootState, Key) + 'static) -> Self {
        self.gesture_handlers.push(GestureHandler {
            interaction_type: InteractionType {
                key: true,
                ..Default::default()
            },
            interaction_handler: Some(Rc::new(move |state, app_state, interaction| {
                let Interaction::Key(key) = interaction else {
                    return;
                };
                (f)(state, app_state, key);
            })),
        });
        self
    }
    pub fn on_scroll(
        mut self,
        f: impl Fn(&mut State, &mut RootState, ScrollDelta) + 'static,
    ) -> Self {
        self.gesture_handlers.push(GestureHandler {
            interaction_type: InteractionType {
                scroll: true,
                ..Default::default()
            },
            interaction_handler: Some(Rc::new(move |state, app_state, interaction| {
                let Interaction::Scroll(scroll) = interaction else {
                    return;
                };
                (f)(state, app_state, scroll);
            })),
        });
        self
    }
}

impl DrawableType {
    pub(crate) fn id(&self) -> u64 {
        match self {
            DrawableType::Text(view) => view.id,
            DrawableType::Layout(_) => 0,
            DrawableType::Path(view) => view.id,
            DrawableType::Svg(view) => view.id,
            DrawableType::Image(view) => view.id,
        }
    }
}

impl<State: 'static> Drawable<State> {
    pub fn finish<'a>(self, ctx: &mut RootCtx) -> Layout<'a, View<State>, RootCtx> {
        let text_clone = if let DrawableType::Text(t) = &self.view_type {
            Some(t.clone())
        } else {
            None
        };

        let node = draw(move |area, _| {
            vec![View::Draw {
                view: Box::new(self.view_type),
                gesture_handlers: self.gesture_handlers,
                area,
            }]
        });

        if let Some(text_view) = text_clone {
            text_view.with_text_constraints(ctx, node)
        } else {
            node
        }
    }
}

pub fn scope<'a, Root: 'static, Sub: 'static>(
    layout: Layout<'a, View<Sub>, RootCtx>,
    binding: Binding<Root, Sub>,
) -> Layout<'a, View<Root>, RootCtx> {
    let binding = Rc::new(binding);
    layout.map(move |view| match view {
        View::Draw {
            view,
            gesture_handlers,
            area,
        } => View::Draw {
            view,
            gesture_handlers: gesture_handlers
                .into_iter()
                .map(|gh| {
                    let handler = gh.interaction_handler.map(|h| {
                        let binding = binding.clone();
                        Rc::new(
                            move |root: &mut Root, app: &mut RootState, interaction: Interaction| {
                                let mut sub = binding.get(root);
                                h(&mut sub, app, interaction);
                                binding.set(root, sub);
                            },
                        )
                            as Rc<dyn Fn(&mut Root, &mut RootState, Interaction)>
                    });
                    GestureHandler {
                        interaction_type: gh.interaction_type,
                        interaction_handler: handler,
                    }
                })
                .collect(),
            area,
        },
        View::PushLayer { path, blend, alpha } => View::PushLayer { path, blend, alpha },
        View::PopLayer => View::PopLayer,
        View::EditorArea(id, area) => View::EditorArea(id, area),
        View::Empty => View::Empty,
    })
}
