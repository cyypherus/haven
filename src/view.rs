use crate::gestures::{
    Gesture, GestureHandler, GestureHitRegion, GestureRegion, Interaction,
};
use crate::pane::{PaneState, View, ViewKind};
use crate::primitives::{Image, PathData, Shadow, Svg, Text};
use crate::{Binding, OwnedBinding};
use backer::{Area, Layout, nodes::*};
use kurbo::{Affine, BezPath};
use parley::Layout as TextLayout;
use peniko::{self, Brush};
use std::rc::Rc;

// A simple const hash for our purposes.
const FNV_OFFSET: u64 = 1469598103934665603;

pub const fn const_hash(s: &str, line: u32, col: u32) -> u64 {
    let mut hash = FNV_OFFSET;
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        hash = combine_id(hash, bytes[i] as u64);
        i += 1;
    }
    // Incorporate the line and column numbers into the hash.
    hash = combine_id(hash, line as u64);
    hash = combine_id(hash, col as u64);
    hash
}

pub const fn combine_id(seed: u64, part: u64) -> u64 {
    seed.wrapping_add(part)
        .wrapping_add(0x9e3779b97f4a7c15)
        .wrapping_mul(0xbf58476d1ce4e5b9)
        .rotate_left(31)
}

/// This macro computes a compile-time ID from the file, line, and column
/// where it's invoked, and at runtime combines it with caller-provided parts.
#[macro_export]
macro_rules! id {
    // It would be good to explore using a TypeId for uniqueness instead of
    // the caller location. Currently we can't hash TypeId values at
    // compile time / in const contexts so the ids throughout the crate would
    // have to be changed to some Hashable struct with the unique gesture.
    () => {{
        const ID: u64 = $crate::const_hash(file!(), line!(), column!());
        ID
    }};
    ($other:expr) => {{
        const ID: u64 = $crate::const_hash(file!(), line!(), column!());
        $crate::combine_id(ID, ($other) as u64)
    }};
    ($first:expr, $($rest:expr),+ $(,)?) => {{
        const ID: u64 = $crate::const_hash(file!(), line!(), column!());
        let id = $crate::combine_id(ID, ($first) as u64);
        $(
            let id = $crate::combine_id(id, ($rest) as u64);
        )+
        id
    }};
}

#[macro_export]
macro_rules! binding {
    ($state_var:ident, $State:ty, $field:ident) => {
        (
            &$state_var.$field,
            Binding::new(|s: &$State| &s.$field, |s: &mut $State| &mut s.$field),
        )
    };
}

pub fn rect_path(area: Area) -> BezPath {
    use kurbo::{Rect, Shape};
    Rect::new(
        area.x as f64,
        area.y as f64,
        (area.x + area.width) as f64,
        (area.y + area.height) as f64,
    )
    .to_path(0.1)
}

pub fn rounded_rect_path(area: Area, radius: f32) -> BezPath {
    use kurbo::{Rect, RoundedRect, Shape};
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
        use peniko::{Compose, Mix};
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

impl<'a, State: 'static> Compositing<'a, State> for Layout<'a, View<State>, PaneState> {
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
    content: Layout<'a, View<State>, PaneState>,
    path: impl Fn(Area) -> BezPath + 'static,
    blend: peniko::BlendMode,
    alpha: f32,
) -> Layout<'a, View<State>, PaneState> {
    stack(vec![
        draw(move |area, _| {
            vec![View::draw(
                Box::new(DrawableType::PushLayer {
                    path: path(area),
                    blend,
                    alpha,
                }),
                area,
            )]
        }),
        content,
        draw(|area, _| vec![View::draw(Box::new(DrawableType::PopLayer), area)]),
    ])
}

pub struct Drawable<State> {
    pub(crate) view_type: DrawableType,
    gestures: Vec<GestureRegion<State>>,
}

pub(crate) enum DrawableType {
    Text(Text),
    Layout(Box<(TextLayout<Brush>, Affine)>),
    Path(Box<PathData>),
    Svg(Svg),
    Image(Image),
    Shadow(Shadow),
    PushLayer {
        path: BezPath,
        blend: peniko::BlendMode,
        alpha: f32,
    },
    PopLayer,
}

impl Clone for DrawableType {
    fn clone(&self) -> Self {
        match self {
            DrawableType::Text(text) => DrawableType::Text(text.clone()),
            DrawableType::Layout(boxed) => DrawableType::Layout(boxed.clone()),
            DrawableType::Path(path) => DrawableType::Path(path.clone()),
            DrawableType::Svg(svg) => DrawableType::Svg(svg.clone()),
            DrawableType::Image(image) => DrawableType::Image(image.clone()),
            DrawableType::Shadow(shadow) => DrawableType::Shadow(shadow.clone()),
            DrawableType::PushLayer { path, blend, alpha } => DrawableType::PushLayer {
                path: path.clone(),
                blend: *blend,
                alpha: *alpha,
            },
            DrawableType::PopLayer => DrawableType::PopLayer,
        }
    }
}

impl DrawableType {
    pub(crate) fn id(&self) -> Option<u64> {
        match self {
            DrawableType::Text(view) => Some(view.id),
            DrawableType::Layout(_) => None,
            DrawableType::Path(view) => Some(view.id),
            DrawableType::Svg(view) => Some(view.id),
            DrawableType::Image(view) => Some(view.id),
            DrawableType::Shadow(view) => Some(view.id),
            DrawableType::PushLayer { .. } | DrawableType::PopLayer => None,
        }
    }
}

impl<State: 'static> Drawable<State> {
    pub(crate) fn new(view_type: DrawableType) -> Self {
        Self {
            view_type,
            gestures: Vec::new(),
        }
    }

    pub fn build<'a>(self, ctx: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
        let text_clone = if let DrawableType::Text(t) = &self.view_type {
            Some(t.clone())
        } else {
            None
        };

        let node = draw(move |area, _| {
            vec![View(ViewKind::Draw {
                view: Box::new(self.view_type.clone()),
                area,
                gestures: self.gestures.clone(),
            })]
        });

        if let Some(text_view) = text_clone {
            text_view.with_text_constraints(ctx, node)
        } else {
            node
        }
    }

    pub fn capture(mut self, gesture: &Gesture<State>) -> Self {
        self.gestures.push(GestureRegion {
            hit_region: GestureHitRegion::Include,
            gesture: gesture.clone(),
        });
        self
    }

    pub fn ignore(mut self, gesture: &Gesture<State>) -> Self {
        self.gestures.push(GestureRegion {
            hit_region: GestureHitRegion::Exclude,
            gesture: gesture.clone(),
        });
        self
    }

    pub fn gesture(mut self, gesture: Gesture<State>) -> Self {
        self.gestures.push(GestureRegion {
            hit_region: GestureHitRegion::Include,
            gesture,
        });
        self
    }
}

pub fn scope<'a, Parent: 'static, Sub: 'static>(
    layout: Layout<'a, View<Sub>, PaneState>,
    binding: Binding<Parent, Sub>,
) -> Layout<'a, View<Parent>, PaneState> {
    let binding = Rc::new(binding);
    map_scope(layout, move |parent, app, interaction, h| {
        h(binding.get_mut(parent), app, interaction);
    })
}

pub fn owned_scope<'a, Parent: 'static, Sub: 'static>(
    layout: Layout<'a, View<Sub>, PaneState>,
    binding: OwnedBinding<Parent, Sub>,
) -> Layout<'a, View<Parent>, PaneState> {
    map_scope(layout, move |parent, app, interaction, h| {
        binding.update(parent, |sub| h(sub, app, interaction));
    })
}

fn map_scope<'a, Parent: 'static, Sub: 'static>(
    layout: Layout<'a, View<Sub>, PaneState>,
    f: impl Fn(
        &mut Parent,
        &mut PaneState,
        Interaction,
        &Rc<dyn Fn(&mut Sub, &mut PaneState, Interaction)>,
    ) + Clone
    + 'static,
) -> Layout<'a, View<Parent>, PaneState> {
    layout.map(move |view| match view.into_kind() {
        ViewKind::Draw {
            view,
            area,
            gestures,
        } => View(ViewKind::Draw {
            view,
            area,
            gestures: gestures
                .into_iter()
                .map(|region| map_gesture_region(region, f.clone()))
                .collect(),
        }),
        ViewKind::EditorArea(id, area, gestures) => View(ViewKind::EditorArea(
            id,
            area,
            gestures
                .into_iter()
                .map(|region| map_gesture_region(region, f.clone()))
                .collect(),
        )),
        ViewKind::Empty => View::empty(),
    })
}

fn map_gesture_region<Parent: 'static, Sub: 'static>(
    region: GestureRegion<Sub>,
    f: impl Fn(
        &mut Parent,
        &mut PaneState,
        Interaction,
        &Rc<dyn Fn(&mut Sub, &mut PaneState, Interaction)>,
    ) + Clone
    + 'static,
) -> GestureRegion<Parent> {
    GestureRegion {
        hit_region: region.hit_region,
        gesture: region
            .gesture
            .map(|handler| map_gesture_handler(handler, f)),
    }
}

fn map_gesture_handler<Parent: 'static, Sub: 'static>(
    gesture: GestureHandler<Sub>,
    f: impl Fn(
        &mut Parent,
        &mut PaneState,
        Interaction,
        &Rc<dyn Fn(&mut Sub, &mut PaneState, Interaction)>,
    ) + Clone
    + 'static,
) -> GestureHandler<Parent> {
    let handler = gesture.interaction_handler.clone();
    GestureHandler {
        modifiers: gesture.modifiers,
        propagation: gesture.propagation,
        kind: gesture.kind,
        interaction_handler: Rc::new(
            move |parent: &mut Parent, app: &mut PaneState, interaction: Interaction| {
                f(parent, app, interaction, &handler);
            },
        ),
    }
}
