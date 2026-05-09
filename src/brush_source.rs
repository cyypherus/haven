use backer::Area;
use std::{fmt::Debug, rc::Rc};
use vello_svg::vello::peniko::{Brush, Gradient};

use crate::Color;

#[derive(Clone)]
pub enum BrushSource<V> {
    Static(Brush),
    Dynamic(Rc<dyn Fn(Area, &V) -> Brush>),
}

impl<V> BrushSource<V> {
    pub(crate) fn resolve(&self, area: Area, state: &V) -> Brush {
        match self {
            BrushSource::Static(brush) => brush.clone(),
            BrushSource::Dynamic(func) => func(area, state),
        }
    }

    pub(crate) fn resolve_to_stateless(&self, state: &V) -> BrushSource<()>
    where
        V: Clone + 'static,
    {
        match self {
            BrushSource::Static(brush) => BrushSource::Static(brush.clone()),
            BrushSource::Dynamic(func) => {
                let func = func.clone();
                let state = state.clone();
                BrushSource::Dynamic(Rc::new(move |area, _| func(area, &state)))
            }
        }
    }
}

impl<V> From<Brush> for BrushSource<V> {
    fn from(value: Brush) -> Self {
        BrushSource::Static(value)
    }
}

impl<V> From<Color> for BrushSource<V> {
    fn from(value: Color) -> Self {
        BrushSource::Static(Brush::Solid(value))
    }
}

impl<V> From<Gradient> for BrushSource<V> {
    fn from(value: Gradient) -> Self {
        BrushSource::Static(Brush::Gradient(value))
    }
}

impl<T, V> From<T> for BrushSource<V>
where
    T: Fn(Area, &V) -> Brush + 'static,
{
    fn from(value: T) -> Self {
        BrushSource::Dynamic(Rc::new(value))
    }
}

impl<V> Debug for BrushSource<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrushSource::Static(brush) => write!(f, "Brush: {:?}", brush),
            BrushSource::Dynamic(_) => write!(f, "Dynamic <function>"),
        }
    }
}
