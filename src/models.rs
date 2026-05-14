use std::{fmt::Debug, rc::Rc};

pub use backer::Align;

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

impl From<&str> for Key {
    fn from(value: &str) -> Self {
        Self::Character(value.to_string())
    }
}

impl From<String> for Key {
    fn from(value: String) -> Self {
        Self::Character(value)
    }
}

impl From<NamedKey> for Key {
    fn from(value: NamedKey) -> Self {
        Self::Named(value)
    }
}

type Getter<State, T> = Rc<dyn for<'a> Fn(&'a State) -> &'a T>;
type GetterMut<State, T> = Rc<dyn for<'a> Fn(&'a mut State) -> &'a mut T>;
type OwnedGetter<State, T> = Rc<dyn Fn(&State) -> Option<T>>;
type OwnedSetter<State, T> = Rc<dyn Fn(&mut State, T)>;

enum BindingStorage<State, T> {
    Lens {
        get: Getter<State, T>,
        get_mut: GetterMut<State, T>,
    },
    Constant(Rc<T>),
}

pub struct Binding<State, T> {
    storage: BindingStorage<State, T>,
}

pub struct OwnedBinding<State, T> {
    get: OwnedGetter<State, T>,
    set: OwnedSetter<State, T>,
}

impl<State, T> OwnedBinding<State, T> {
    pub fn new(
        get: impl Fn(&State) -> Option<T> + 'static,
        set: impl Fn(&mut State, T) + 'static,
    ) -> Self {
        Self {
            get: Rc::new(get),
            set: Rc::new(set),
        }
    }

    pub fn get(&self, state: &State) -> Option<T> {
        (self.get)(state)
    }

    pub fn set(&self, state: &mut State, value: T) {
        (self.set)(state, value);
    }

    pub fn update(&self, state: &mut State, f: impl FnOnce(&mut T)) {
        if let Some(mut value) = self.get(state) {
            f(&mut value);
            self.set(state, value);
        }
    }
}

impl<State, T> Clone for OwnedBinding<State, T> {
    fn clone(&self) -> Self {
        Self {
            get: self.get.clone(),
            set: self.set.clone(),
        }
    }
}

impl<State, T> Debug for OwnedBinding<State, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OwnedBinding").finish_non_exhaustive()
    }
}

impl<State, T> Debug for Binding<State, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Binding").finish_non_exhaustive()
    }
}

impl<State, T> Binding<State, T> {
    pub fn new(
        get: impl for<'a> Fn(&'a State) -> &'a T + 'static,
        get_mut: impl for<'a> Fn(&'a mut State) -> &'a mut T + 'static,
    ) -> Self {
        Self {
            storage: BindingStorage::Lens {
                get: Rc::new(get),
                get_mut: Rc::new(get_mut),
            },
        }
    }

    pub fn constant(value: T) -> Self
    where
        T: 'static,
    {
        Self {
            storage: BindingStorage::Constant(Rc::new(value)),
        }
    }
}

impl<State, T> Clone for Binding<State, T> {
    fn clone(&self) -> Self {
        Self {
            storage: match &self.storage {
                BindingStorage::Lens { get, get_mut } => BindingStorage::Lens {
                    get: get.clone(),
                    get_mut: get_mut.clone(),
                },
                BindingStorage::Constant(value) => BindingStorage::Constant(value.clone()),
            },
        }
    }
}

impl<State, T> Binding<State, T> {
    pub fn get<'a>(&'a self, state: &'a State) -> &'a T {
        match &self.storage {
            BindingStorage::Lens { get, .. } => get(state),
            BindingStorage::Constant(value) => value.as_ref(),
        }
    }

    pub fn get_mut<'a>(&'a self, state: &'a mut State) -> &'a mut T {
        match &self.storage {
            BindingStorage::Lens { get_mut, .. } => get_mut(state),
            BindingStorage::Constant(_) => panic!("constant binding cannot be mutably borrowed"),
        }
    }

    pub fn set(&self, state: &mut State, value: T) {
        if let BindingStorage::Lens { get_mut, .. } = &self.storage {
            *get_mut(state) = value;
        }
    }

    pub fn update(&self, state: &mut State, f: impl FnOnce(&mut T)) {
        if let BindingStorage::Lens { get_mut, .. } = &self.storage {
            f(get_mut(state));
        }
    }
}
