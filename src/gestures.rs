use backer::Area;

use crate::{Key, Point};
use std::{
    fmt::{self, Debug, Formatter},
    rc::Rc,
};

#[derive(Debug, Clone, Copy)]
pub(crate) enum GestureState {
    None,
    Dragging {
        start: Point,
        last_position: Point,
        capturer: u64,
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
    pub location: ClickLocation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyPhase {
    Pressed,
    Released,
}

#[derive(Clone)]
pub(crate) enum Interaction {
    Click(ClickEvent),
    ClickOutside(ClickEvent),
    Drag(DragPhase),
    Hover(bool),
    Key(Key, KeyPhase),
    Scroll(ScrollDelta),
}

#[derive(Debug, Clone)]
pub enum EditInteraction {
    Update(String),
    End,
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct InteractionType {
    pub(crate) click: Option<MouseButton>,
    pub(crate) click_outside: Option<MouseButton>,
    pub(crate) drag: bool,
    pub(crate) hover: bool,
    pub(crate) key: bool,
    pub(crate) scroll: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollDelta {
    pub x: f32,
    pub y: f32,
}
pub(crate) type InteractionHandler<T, U> = Rc<dyn Fn(&mut T, &mut U, Interaction)>;
pub(crate) struct GestureHandler<T: ?Sized, U> {
    pub(crate) interaction_type: InteractionType,
    pub(crate) interaction_handler: Option<InteractionHandler<T, U>>,
}

impl<T: ?Sized, U> Debug for GestureHandler<T, U> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("GestureHandler")
            .field("interaction_type", &self.interaction_type)
            .finish()
    }
}

impl<T: ?Sized, U> Default for GestureHandler<T, U> {
    fn default() -> Self {
        GestureHandler {
            interaction_type: InteractionType::default(),
            interaction_handler: None,
        }
    }
}

impl<T: ?Sized, U> Clone for GestureHandler<T, U> {
    fn clone(&self) -> Self {
        Self {
            interaction_type: self.interaction_type,
            interaction_handler: self.interaction_handler.clone(),
        }
    }
}
