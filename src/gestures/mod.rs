use backer::Area;

use crate::{Key, Modifiers, PaneState, Point};
pub use predicates::{ButtonPredicate, KeyPredicate, ModifierPredicate};
use std::{
    fmt::{self, Debug, Formatter},
    rc::Rc,
};

mod predicates;

#[derive(Debug, Clone, Copy)]
pub(crate) struct CapturedGesture {
    pub(crate) id: GestureId,
    pub(crate) area: Area,
}

#[derive(Debug, Clone)]
pub(crate) struct GestureCapturer {
    pub(crate) clicks: Vec<CapturedGesture>,
    pub(crate) drags: Vec<CapturedGesture>,
}

#[derive(Debug, Clone)]
pub(crate) enum GestureState {
    None,
    Pressing {
        start: Point,
        capturer: GestureCapturer,
        button: MouseButton,
        click_started: bool,
    },
    Dragging {
        start: Point,
        last_position: Point,
        capturer: GestureCapturer,
        button: MouseButton,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum DragPhase {
    Began {
        start: Point,
        start_global: Point,
    },
    Updated {
        start: Point,
        current: Point,
        start_global: Point,
        current_global: Point,
        delta: Point,
        distance: f32,
    },
    Completed {
        start: Point,
        current: Point,
        start_global: Point,
        current_global: Point,
        delta: Point,
        distance: f32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickPhase {
    Started,
    Cancelled,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
}

#[derive(Debug, Clone, Copy)]
pub struct ClickLocation {
    global: Point,
    area: Area,
}

impl ClickLocation {
    pub(crate) fn new(global: Point, area: Area) -> Self {
        ClickLocation { global, area }
    }
    pub fn global(&self) -> Point {
        self.global
    }
    pub fn local(&self) -> Point {
        Point {
            x: self.global.x - self.area.x as f64,
            y: self.global.y - self.area.y as f64,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ClickEvent {
    pub state: ClickPhase,
    pub button: MouseButton,
    pub location: ClickLocation,
}

#[derive(Debug, Clone)]
pub struct KeyEvent {
    pub key: Key,
    pub phase: KeyPhase,
    pub modifiers: Modifiers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyPhase {
    Pressed,
    Released,
}

#[derive(Clone)]
pub(crate) enum Interaction {
    Click(ClickEvent),
    Drag(DragPhase),
    Hover(bool),
    Key(Key, KeyPhase),
    Scroll(ScrollDelta),
}

#[derive(Debug, Clone)]
pub enum EditInteraction {
    Start,
    Update(String),
    End,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollDelta {
    pub x: f32,
    pub y: f32,
}
pub(crate) type InteractionHandler<T, U> = Rc<dyn Fn(&mut T, &mut U, Interaction)>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GestureId(u64);

impl GestureId {
    pub(crate) fn new(id: u64) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GesturePropagation {
    Stop,
    Continue,
}

pub(crate) struct GestureHandler<State: ?Sized> {
    pub(crate) modifiers: ModifierPredicate,
    pub(crate) propagation: GesturePropagation,
    pub(crate) kind: GestureKind,
    pub(crate) interaction_handler: InteractionHandler<State, PaneState>,
}

impl<State: ?Sized> Clone for GestureHandler<State> {
    fn clone(&self) -> Self {
        Self {
            modifiers: self.modifiers.clone(),
            propagation: self.propagation,
            kind: self.kind.clone(),
            interaction_handler: self.interaction_handler.clone(),
        }
    }
}

impl<State: ?Sized> Debug for GestureHandler<State> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("GestureHandler")
            .field("modifiers", &self.modifiers)
            .field("propagation", &self.propagation)
            .field("kind", &self.kind)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub(crate) enum GestureKind {
    Click { buttons: ButtonPredicate },
    Drag { button: ButtonPredicate },
    Hover,
    Scroll,
    Key { keys: KeyPredicate },
}

pub struct Gesture<State: ?Sized> {
    pub(crate) id: GestureId,
    pub(crate) handler: GestureHandler<State>,
}

impl<State: ?Sized> Clone for Gesture<State> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            handler: self.handler.clone(),
        }
    }
}

impl<State> Gesture<State> {
    pub(crate) fn map<Parent>(
        &self,
        map_handler: impl FnOnce(GestureHandler<State>) -> GestureHandler<Parent>,
    ) -> Gesture<Parent> {
        Gesture {
            id: self.id,
            handler: map_handler(self.handler.clone()),
        }
    }
}

impl<State: ?Sized> Gesture<State> {
    pub fn id(&self) -> GestureId {
        self.id
    }

    pub(crate) fn handler(&self) -> &GestureHandler<State> {
        &self.handler
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GestureHitRegion {
    Include,
    Exclude,
}

pub(crate) struct GestureRegion<State: ?Sized> {
    pub(crate) hit_region: GestureHitRegion,
    pub(crate) gesture: Gesture<State>,
}

impl<State: ?Sized> Clone for GestureRegion<State> {
    fn clone(&self) -> Self {
        Self {
            hit_region: self.hit_region,
            gesture: self.gesture.clone(),
        }
    }
}

pub mod gesture {
    use super::*;

    pub fn click(id: u64) -> ClickGesture {
        ClickGesture {
            id: GestureId::new(id),
            buttons: ButtonPredicate::any(),
            modifiers: ModifierPredicate::any(),
            propagation: GesturePropagation::Stop,
        }
    }

    pub fn drag(id: u64) -> DragGesture {
        DragGesture {
            id: GestureId::new(id),
            button: ButtonPredicate::any(),
            modifiers: ModifierPredicate::any(),
            propagation: GesturePropagation::Stop,
        }
    }

    pub fn scroll(id: u64) -> ScrollGesture {
        ScrollGesture {
            id: GestureId::new(id),
            modifiers: ModifierPredicate::any(),
            propagation: GesturePropagation::Stop,
        }
    }

    pub fn hover(id: u64) -> HoverGesture {
        HoverGesture {
            id: GestureId::new(id),
            modifiers: ModifierPredicate::any(),
            propagation: GesturePropagation::Stop,
        }
    }

    pub fn key(id: u64) -> KeyGesture {
        KeyGesture {
            id: GestureId::new(id),
            keys: KeyPredicate::any(),
            modifiers: ModifierPredicate::any(),
            propagation: GesturePropagation::Stop,
        }
    }
}

pub struct ClickGesture {
    id: GestureId,
    buttons: ButtonPredicate,
    modifiers: ModifierPredicate,
    propagation: GesturePropagation,
}

impl ClickGesture {
    pub fn button(mut self, button: impl Into<ButtonPredicate>) -> Self {
        self.buttons = button.into();
        self
    }

    pub fn modifiers(mut self, modifiers: impl Into<ModifierPredicate>) -> Self {
        self.modifiers = modifiers.into();
        self
    }

    pub fn capture(mut self) -> Self {
        self.propagation = GesturePropagation::Stop;
        self
    }

    pub fn observe(mut self) -> Self {
        self.propagation = GesturePropagation::Continue;
        self
    }

    pub fn run<State: 'static>(
        self,
        f: impl Fn(&mut State, &mut PaneState, ClickEvent) + 'static,
    ) -> Gesture<State> {
        Gesture {
            id: self.id,
            handler: GestureHandler {
                modifiers: self.modifiers,
                propagation: self.propagation,
                kind: GestureKind::Click {
                    buttons: self.buttons,
                },
                interaction_handler: Rc::new(move |state, app, interaction| {
                    let Interaction::Click(event) = interaction else {
                        return;
                    };
                    f(state, app, event);
                }),
            },
        }
    }
}

pub struct DragGesture {
    id: GestureId,
    button: ButtonPredicate,
    modifiers: ModifierPredicate,
    propagation: GesturePropagation,
}

impl DragGesture {
    pub fn button(mut self, button: impl Into<ButtonPredicate>) -> Self {
        self.button = button.into();
        self
    }

    pub fn modifiers(mut self, modifiers: impl Into<ModifierPredicate>) -> Self {
        self.modifiers = modifiers.into();
        self
    }

    pub fn capture(mut self) -> Self {
        self.propagation = GesturePropagation::Stop;
        self
    }

    pub fn observe(mut self) -> Self {
        self.propagation = GesturePropagation::Continue;
        self
    }

    pub fn run<State: 'static>(
        self,
        f: impl Fn(&mut State, &mut PaneState, DragPhase) + 'static,
    ) -> Gesture<State> {
        Gesture {
            id: self.id,
            handler: GestureHandler {
                modifiers: self.modifiers,
                propagation: self.propagation,
                kind: GestureKind::Drag {
                    button: self.button,
                },
                interaction_handler: Rc::new(move |state, app, interaction| {
                    let Interaction::Drag(event) = interaction else {
                        return;
                    };
                    f(state, app, event);
                }),
            },
        }
    }
}

pub struct ScrollGesture {
    id: GestureId,
    modifiers: ModifierPredicate,
    propagation: GesturePropagation,
}

impl ScrollGesture {
    pub fn modifiers(mut self, modifiers: impl Into<ModifierPredicate>) -> Self {
        self.modifiers = modifiers.into();
        self
    }

    pub fn capture(mut self) -> Self {
        self.propagation = GesturePropagation::Stop;
        self
    }

    pub fn observe(mut self) -> Self {
        self.propagation = GesturePropagation::Continue;
        self
    }

    pub fn run<State: 'static>(
        self,
        f: impl Fn(&mut State, &mut PaneState, ScrollDelta) + 'static,
    ) -> Gesture<State> {
        Gesture {
            id: self.id,
            handler: GestureHandler {
                modifiers: self.modifiers,
                propagation: self.propagation,
                kind: GestureKind::Scroll,
                interaction_handler: Rc::new(move |state, app, interaction| {
                    let Interaction::Scroll(delta) = interaction else {
                        return;
                    };
                    f(state, app, delta);
                }),
            },
        }
    }
}

pub struct HoverGesture {
    id: GestureId,
    modifiers: ModifierPredicate,
    propagation: GesturePropagation,
}

impl HoverGesture {
    pub fn modifiers(mut self, modifiers: impl Into<ModifierPredicate>) -> Self {
        self.modifiers = modifiers.into();
        self
    }

    pub fn capture(mut self) -> Self {
        self.propagation = GesturePropagation::Stop;
        self
    }

    pub fn observe(mut self) -> Self {
        self.propagation = GesturePropagation::Continue;
        self
    }

    pub fn run<State: 'static>(
        self,
        f: impl Fn(&mut State, &mut PaneState, bool) + 'static,
    ) -> Gesture<State> {
        Gesture {
            id: self.id,
            handler: GestureHandler {
                modifiers: self.modifiers,
                propagation: self.propagation,
                kind: GestureKind::Hover,
                interaction_handler: Rc::new(move |state, app, interaction| {
                    let Interaction::Hover(hovered) = interaction else {
                        return;
                    };
                    f(state, app, hovered);
                }),
            },
        }
    }
}

pub struct KeyGesture {
    id: GestureId,
    keys: KeyPredicate,
    modifiers: ModifierPredicate,
    propagation: GesturePropagation,
}

impl KeyGesture {
    pub fn key(mut self, key: impl Into<KeyPredicate>) -> Self {
        self.keys = key.into();
        self
    }

    pub fn modifiers(mut self, modifiers: impl Into<ModifierPredicate>) -> Self {
        self.modifiers = modifiers.into();
        self
    }

    pub fn capture(mut self) -> Self {
        self.propagation = GesturePropagation::Stop;
        self
    }

    pub fn observe(mut self) -> Self {
        self.propagation = GesturePropagation::Continue;
        self
    }

    pub fn run<State: 'static>(
        self,
        f: impl Fn(&mut State, &mut PaneState, KeyEvent) + 'static,
    ) -> Gesture<State> {
        Gesture {
            id: self.id,
            handler: GestureHandler {
                modifiers: self.modifiers,
                propagation: self.propagation,
                kind: GestureKind::Key { keys: self.keys },
                interaction_handler: Rc::new(move |state, app, interaction| {
                    let Interaction::Key(key, phase) = interaction else {
                        return;
                    };
                    let modifiers = app.modifiers.unwrap_or_default();
                    f(
                        state,
                        app,
                        KeyEvent {
                            key,
                            phase,
                            modifiers,
                        },
                    );
                }),
            },
        }
    }
}
