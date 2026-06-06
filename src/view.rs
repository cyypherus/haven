use crate::gestures::{
    EditInteraction, Gesture, GestureAreaComponent, GestureAreaOperation, GestureHandler,
    Interaction,
    regions::{area_rect, intersect},
};
use crate::pane::{EditHandler, PaneElement, PaneElementKind, PaneState, View};
use crate::primitives::{Image, PathData, Shadow, Svg, Text};
use crate::{Binding, OwnedBinding};
use backer::{Area, nodes::*};
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
    ($state_var:ident.$($field:ident).+) => {{
        fn bind<'a, State, T>(
            state: &'a State,
            get: impl for<'b> Fn(&'b State) -> &'b T + 'static,
            get_mut: impl for<'b> Fn(&'b mut State) -> &'b mut T + 'static,
        ) -> (&'a T, $crate::Binding<State, T>) {
            (get(state), $crate::Binding::new(get, get_mut))
        }
        bind(
            $state_var,
            |s| &s.$($field).+,
            |s| &mut s.$($field).+,
        )
    }};
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

pub trait Compositing<'a, State> {
    fn clipped(self, path: impl Fn(Area) -> BezPath + 'static) -> Self;
    fn blend(self, mode: BlendMode) -> Self;
    fn opacity(self, alpha: f32) -> Self;
}

impl<'a, State: 'static> Compositing<'a, State> for View<'a, State> {
    fn clipped(self, path: impl Fn(Area) -> BezPath + 'static) -> Self {
        wrap_layer(self, path, peniko::BlendMode::default(), 1.0, true)
    }
    fn blend(self, mode: BlendMode) -> Self {
        use peniko::{Compose, Mix};

        let mode = match mode {
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
        };
        wrap_layer(self, rect_path, mode, 1.0, false)
    }
    fn opacity(self, alpha: f32) -> Self {
        wrap_layer(
            self,
            rect_path,
            peniko::BlendMode::default(),
            alpha.clamp(0., 1.),
            false,
        )
    }
}

fn wrap_layer<'a, State: 'static>(
    mut content: View<'a, State>,
    path: impl Fn(Area) -> BezPath + 'static,
    blend: peniko::BlendMode,
    alpha: f32,
    clip_gestures: bool,
) -> View<'a, State> {
    draw(move |area, ctx| {
        let mut views = Vec::new();
        views.extend(
            Drawable {
                view_type: DrawableType::PushLayer {
                    path: path(area),
                    blend,
                    alpha,
                },
                gestures: Vec::new(),
            }
            .build(ctx)
            .draw(area, ctx),
        );
        let child_views = content.draw(area, ctx);
        if clip_gestures {
            let clip_rect = area_rect(area);
            views.extend(child_views.into_iter().map(move |view| {
                match view.into_kind() {
                    PaneElementKind::Draw {
                        view,
                        area,
                        gestures,
                    } => PaneElement(PaneElementKind::Draw {
                        view,
                        area,
                        gestures: gestures
                            .into_iter()
                            .filter_map(|component| {
                                let rect = component.rect.unwrap_or_else(|| area_rect(area));
                                intersect(rect, clip_rect).map(|rect| GestureAreaComponent {
                                    operation: component.operation,
                                    gesture: component.gesture,
                                    rect: Some(rect),
                                })
                            })
                            .collect(),
                    }),
                    PaneElementKind::EditorArea {
                        id,
                        area,
                        edit_handler,
                    } => PaneElement(PaneElementKind::EditorArea {
                        id,
                        area,
                        edit_handler,
                    }),
                    PaneElementKind::Empty => PaneElement::empty(),
                }
            }));
        } else {
            views.extend(child_views);
        }
        views.extend(
            Drawable {
                view_type: DrawableType::PopLayer,
                gestures: Vec::new(),
            }
            .build(ctx)
            .draw(area, ctx),
        );
        views
    })
}

pub struct Drawable<State> {
    pub(crate) view_type: DrawableType,
    gestures: Vec<GestureAreaComponent<State>>,
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

impl<State: 'static> Drawable<State> {
    pub(crate) fn new(view_type: DrawableType) -> Self {
        Self {
            view_type,
            gestures: Vec::new(),
        }
    }

    pub fn build<'a>(self, ctx: &mut PaneState) -> View<'a, State> {
        let text_clone = if let DrawableType::Text(t) = &self.view_type {
            Some(t.clone())
        } else {
            None
        };

        let node = draw(move |area, _| {
            vec![PaneElement(PaneElementKind::Draw {
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

    pub fn include(mut self, gesture: &Gesture<State>) -> Self {
        self.gestures.push(GestureAreaComponent {
            operation: GestureAreaOperation::Include,
            gesture: gesture.clone(),
            rect: None,
        });
        self
    }

    pub fn occlude(mut self, gesture: &Gesture<State>) -> Self {
        self.gestures.push(GestureAreaComponent {
            operation: GestureAreaOperation::Occlude,
            gesture: gesture.clone(),
            rect: None,
        });
        self
    }

    pub fn gesture(mut self, gesture: Gesture<State>) -> Self {
        self.gestures.push(GestureAreaComponent {
            operation: GestureAreaOperation::Include,
            gesture,
            rect: None,
        });
        self
    }
}

pub fn scope<'a, Parent: 'static, Sub: 'static>(
    layout: View<'a, Sub>,
    binding: Binding<Parent, Sub>,
) -> View<'a, Parent> {
    let binding = Rc::new(binding);
    let gesture_binding = binding.clone();
    map_scope(
        layout,
        move |parent, app, interaction, h| {
            h(gesture_binding.get_mut(parent), app, interaction);
        },
        move |parent, app, event, h| {
            h(binding.get_mut(parent), app, event);
        },
    )
}

pub fn owned_scope<'a, Parent: 'static, Sub: 'static>(
    layout: View<'a, Sub>,
    binding: OwnedBinding<Parent, Sub>,
) -> View<'a, Parent> {
    let gesture_binding = binding.clone();
    map_scope(
        layout,
        move |parent, app, interaction, h| {
            gesture_binding.update(parent, |sub| h(sub, app, interaction));
        },
        move |parent, app, event, h| {
            binding.update(parent, |sub| h(sub, app, event));
        },
    )
}

fn map_scope<'a, Parent: 'static, Sub: 'static>(
    layout: View<'a, Sub>,
    f: impl Fn(
        &mut Parent,
        &mut PaneState,
        Interaction,
        &Rc<dyn Fn(&mut Sub, &mut PaneState, Interaction)>,
    ) + Clone
    + 'static,
    callback_f: impl Fn(&mut Parent, &mut PaneState, EditInteraction, &EditHandler<Sub>)
    + Clone
    + 'static,
) -> View<'a, Parent> {
    layout.map(move |view| match view.into_kind() {
        PaneElementKind::Draw {
            view,
            area,
            gestures,
        } => PaneElement(PaneElementKind::Draw {
            view,
            area,
            gestures: gestures
                .into_iter()
                .map(|component| GestureAreaComponent {
                    operation: component.operation,
                    rect: component.rect,
                    gesture: component.gesture.map({
                        let f = f.clone();
                        move |gesture| {
                            let handler = gesture.interaction_handler.clone();
                            GestureHandler {
                                modifiers: gesture.modifiers,
                                propagation: gesture.propagation,
                                positive_by_default: gesture.positive_by_default,
                                kind: gesture.kind,
                                interaction_handler: Rc::new(
                                    move |parent: &mut Parent,
                                          app: &mut PaneState,
                                          interaction: Interaction| {
                                        f(parent, app, interaction, &handler);
                                    },
                                ),
                            }
                        }
                    }),
                })
                .collect(),
        }),
        PaneElementKind::EditorArea {
            id,
            area,
            edit_handler,
        } => PaneElement(PaneElementKind::EditorArea {
            id,
            area,
            edit_handler: if let Some(edit_handler) = edit_handler {
                Some(Rc::new({
                    let callback_f = callback_f.clone();
                    move |parent, app, edit| {
                        callback_f(parent, app, edit, &edit_handler);
                    }
                }))
            } else {
                None
            },
        }),
        PaneElementKind::Empty => PaneElement::empty(),
    })
}
