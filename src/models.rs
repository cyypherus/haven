use std::{fmt::Debug, rc::Rc};
use vello_svg::vello::kurbo::Point;

pub use backer::{Align, Area};

pub(crate) fn area_contains(area: &Area, point: Point) -> bool {
    let x = point.x;
    let y = point.y;
    if x > area.x as f64
        && y > area.y as f64
        && y < area.y as f64 + area.height as f64
        && x < area.x as f64 + area.width as f64
    {
        return true;
    }
    false
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Key {
    Named(NamedKey),
    Character(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NamedKey {
    Enter,
    Escape,
    Space,
    Backspace,
    Delete,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    Home,
    End,
    Tab,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub super_key: bool,
}

impl Key {
    pub fn character(value: impl Into<String>) -> Self {
        Self::Character(value.into())
    }
}

type Getter<State, T> = Rc<dyn Fn(&State) -> T>;
type Setter<State, T> = Rc<dyn Fn(&mut State, T)>;

pub struct Binding<State, T> {
    pub get: Getter<State, T>,
    pub set: Setter<State, T>,
}

impl<State, T> Debug for Binding<State, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Binding")
            .field("get", &"<function>")
            .field("set", &"<function>")
            .finish()
    }
}

impl<State, T> Binding<State, T> {
    pub fn new(get: impl Fn(&State) -> T + 'static, set: impl Fn(&mut State, T) + 'static) -> Self {
        Self {
            get: Rc::new(get),
            set: Rc::new(set),
        }
    }
    pub fn constant(value: T) -> Self
    where
        T: Clone + 'static,
    {
        Self {
            get: Rc::new(move |_| value.clone()),
            set: Rc::new(move |_, _| {}),
        }
    }
}

impl<State, T> Clone for Binding<State, T> {
    fn clone(&self) -> Self {
        Self {
            get: self.get.clone(),
            set: self.set.clone(),
        }
    }
}

impl<State, T> Binding<State, T> {
    pub fn get(&self, state: &State) -> T {
        (self.get)(state)
    }
    pub fn set(&self, state: &mut State, value: T) {
        (self.set)(state, value)
    }
    pub fn update(&self, state: &mut State, f: impl Fn(&mut T)) {
        let mut temp = (self.get)(state);
        f(&mut temp);
        (self.set)(state, temp)
    }
}
