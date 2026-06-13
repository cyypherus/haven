use backer::Area;

use crate::{Key, Modifiers, PaneState, Point};
pub use predicates::{ButtonPredicate, KeyPredicate, ModifierPredicate};
use std::{
    fmt::{self, Debug, Formatter},
    rc::Rc,
};

mod predicates;
pub(crate) mod regions;

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
            x: self.global.x - self.area.x,
            y: self.global.y - self.area.y,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScrollAxes {
    pub(crate) x: bool,
    pub(crate) y: bool,
}

impl ScrollAxes {
    pub(crate) const BOTH: Self = Self { x: true, y: true };
    pub(crate) const HORIZONTAL: Self = Self { x: true, y: false };
    pub(crate) const VERTICAL: Self = Self { x: false, y: true };
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
    pub(crate) positive_by_default: bool,
    pub(crate) kind: GestureKind,
    pub(crate) interaction_handler: InteractionHandler<State, PaneState>,
}

impl<State: ?Sized> Clone for GestureHandler<State> {
    fn clone(&self) -> Self {
        Self {
            modifiers: self.modifiers.clone(),
            propagation: self.propagation,
            positive_by_default: self.positive_by_default,
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
            .field("positive_by_default", &self.positive_by_default)
            .field("kind", &self.kind)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub(crate) enum GestureKind {
    Click { buttons: ButtonPredicate },
    Drag { button: ButtonPredicate },
    Hover,
    Scroll { axes: ScrollAxes },
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
pub(crate) enum GestureAreaOperation {
    Include,
    Occlude,
}

pub(crate) struct GestureAreaComponent<State: ?Sized> {
    pub(crate) operation: GestureAreaOperation,
    pub(crate) gesture: Gesture<State>,
    pub(crate) rect: Option<Area>,
}

impl<State: ?Sized> Clone for GestureAreaComponent<State> {
    fn clone(&self) -> Self {
        Self {
            operation: self.operation,
            gesture: self.gesture.clone(),
            rect: self.rect,
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
            positive_by_default: false,
        }
    }

    pub fn drag(id: u64) -> DragGesture {
        DragGesture {
            id: GestureId::new(id),
            button: ButtonPredicate::any(),
            modifiers: ModifierPredicate::any(),
            propagation: GesturePropagation::Stop,
            positive_by_default: false,
        }
    }

    pub fn scroll(id: u64) -> ScrollGesture {
        ScrollGesture {
            id: GestureId::new(id),
            modifiers: ModifierPredicate::any(),
            propagation: GesturePropagation::Stop,
            positive_by_default: false,
            axes: ScrollAxes::BOTH,
        }
    }

    pub fn hover(id: u64) -> HoverGesture {
        HoverGesture {
            id: GestureId::new(id),
            modifiers: ModifierPredicate::any(),
            propagation: GesturePropagation::Stop,
            positive_by_default: false,
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
    positive_by_default: bool,
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

    pub fn anywhere(mut self) -> Self {
        self.positive_by_default = true;
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
                positive_by_default: self.positive_by_default,
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
    positive_by_default: bool,
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

    pub fn anywhere(mut self) -> Self {
        self.positive_by_default = true;
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
                positive_by_default: self.positive_by_default,
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
    positive_by_default: bool,
    axes: ScrollAxes,
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

    pub fn anywhere(mut self) -> Self {
        self.positive_by_default = true;
        self
    }

    pub fn horizontal(mut self) -> Self {
        self.axes = ScrollAxes::HORIZONTAL;
        self
    }

    pub fn vertical(mut self) -> Self {
        self.axes = ScrollAxes::VERTICAL;
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
                positive_by_default: self.positive_by_default,
                kind: GestureKind::Scroll { axes: self.axes },
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
    positive_by_default: bool,
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

    pub fn anywhere(mut self) -> Self {
        self.positive_by_default = true;
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
                positive_by_default: self.positive_by_default,
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
                positive_by_default: false,
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

#[cfg(test)]
mod tests {
    use crate::*;

    #[derive(Default)]
    struct State {
        clicks: usize,
        other_clicks: usize,
    }

    const CLICK: u64 = 9010;
    const OTHER_CLICK: u64 = 9011;

    fn test_pane<State: 'static>(builder: PaneBuilder<State>) -> Pane<State> {
        builder.build()
    }

    fn click() -> Gesture<State> {
        gesture::click(CLICK).run(|state: &mut State, _, event| {
            if event.state == ClickPhase::Completed {
                state.clicks += 1;
            }
        })
    }

    fn other_click() -> Gesture<State> {
        gesture::click(OTHER_CLICK).run(|state: &mut State, _, event| {
            if event.state == ClickPhase::Completed {
                state.other_clicks += 1;
            }
        })
    }

    fn click_target(pane: &mut Pane<State>, state: &mut State, id: u64) {
        let point = pane.location(id).expect("target present");
        pane.click(state, point);
    }

    #[test]
    fn regions_can_be_added() {
        const A: u64 = 9020;
        const B: u64 = 9021;

        fn view<'a>(_: &'a State, app: &mut PaneState) -> View<'a, State> {
            let click = click();
            row(vec![
                rect(A)
                    .fill(TRANSPARENT)
                    .view()
                    .include(&click)
                    .build(app)
                    .width(80.)
                    .height(40.),
                rect(B)
                    .fill(TRANSPARENT)
                    .view()
                    .include(&click)
                    .build(app)
                    .width(80.)
                    .height(40.),
            ])
        }

        let mut state = State::default();
        let mut pane = test_pane(PaneBuilder::new("test", view));
        pane.redraw(&mut state, 300, 200, 1.0);

        click_target(&mut pane, &mut state, A);
        click_target(&mut pane, &mut state, B);

        assert_eq!(state.clicks, 2);
    }

    #[test]
    fn added_regions_can_be_occluded() {
        const A: u64 = 9030;
        const B: u64 = 9031;
        const C: u64 = 9032;

        fn view<'a>(_: &'a State, app: &mut PaneState) -> View<'a, State> {
            let click = click();
            stack_aligned(
                Align::TopLeading,
                vec![
                    rect(A)
                        .fill(TRANSPARENT)
                        .view()
                        .include(&click)
                        .build(app)
                        .width(100.)
                        .height(60.),
                    rect(B)
                        .fill(TRANSPARENT)
                        .view()
                        .include(&click)
                        .build(app)
                        .width(100.)
                        .height(60.)
                        .offset_x(100.),
                    rect(C)
                        .fill(TRANSPARENT)
                        .view()
                        .occlude(&click)
                        .build(app)
                        .width(40.)
                        .height(60.)
                        .offset_x(130.),
                ],
            )
            .width(200.)
            .height(60.)
        }

        let mut state = State::default();
        let mut pane = test_pane(PaneBuilder::new("test", view));
        pane.redraw(&mut state, 300, 200, 1.0);

        click_target(&mut pane, &mut state, A);
        let b = pane.location(B).expect("b present");
        pane.click(&mut state, Point::new(b.x - 45., b.y));
        click_target(&mut pane, &mut state, C);

        assert_eq!(state.clicks, 2);
    }

    #[test]
    fn region_can_have_multiple_occlusions() {
        const A: u64 = 9040;
        const B: u64 = 9041;
        const C: u64 = 9042;

        fn view<'a>(_: &'a State, app: &mut PaneState) -> View<'a, State> {
            let click = click();
            stack_aligned(
                Align::TopLeading,
                vec![
                    rect(A)
                        .fill(TRANSPARENT)
                        .view()
                        .include(&click)
                        .build(app)
                        .width(180.)
                        .height(60.),
                    rect(B)
                        .fill(TRANSPARENT)
                        .view()
                        .occlude(&click)
                        .build(app)
                        .width(30.)
                        .height(60.)
                        .offset_x(20.),
                    rect(C)
                        .fill(TRANSPARENT)
                        .view()
                        .occlude(&click)
                        .build(app)
                        .width(30.)
                        .height(60.)
                        .offset_x(130.),
                ],
            )
            .width(180.)
            .height(60.)
        }

        let mut state = State::default();
        let mut pane = test_pane(PaneBuilder::new("test", view));
        pane.redraw(&mut state, 300, 200, 1.0);

        click_target(&mut pane, &mut state, A);
        click_target(&mut pane, &mut state, B);
        click_target(&mut pane, &mut state, C);

        assert_eq!(state.clicks, 1);
    }

    #[test]
    fn added_regions_can_be_clipped() {
        const A: u64 = 9050;
        const B: u64 = 9051;

        fn view<'a>(_: &'a State, app: &mut PaneState) -> View<'a, State> {
            let click = click();
            row(vec![
                rect(A)
                    .fill(TRANSPARENT)
                    .view()
                    .include(&click)
                    .build(app)
                    .width(80.)
                    .height(100.),
                rect(B)
                    .fill(TRANSPARENT)
                    .view()
                    .include(&click)
                    .build(app)
                    .width(80.)
                    .height(100.),
            ])
            .clipped(rect_path)
            .width(160.)
            .height(50.)
        }

        let mut state = State::default();
        let mut pane = test_pane(PaneBuilder::new("test", view));
        pane.redraw(&mut state, 300, 200, 1.0);

        let a = pane.location(A).expect("a present");
        let b = pane.location(B).expect("b present");
        pane.click(&mut state, Point::new(a.x, a.y - 25.));
        pane.click(&mut state, Point::new(b.x, b.y - 25.));
        pane.click(&mut state, Point::new(a.x, a.y + 25.));

        assert_eq!(state.clicks, 2);
    }

    #[test]
    fn added_and_occluded_regions_can_be_clipped() {
        const A: u64 = 9060;
        const B: u64 = 9061;
        const C: u64 = 9062;

        fn view<'a>(_: &'a State, app: &mut PaneState) -> View<'a, State> {
            let click = click();
            stack_aligned(
                Align::TopLeading,
                vec![
                    rect(A)
                        .fill(TRANSPARENT)
                        .view()
                        .include(&click)
                        .build(app)
                        .width(100.)
                        .height(100.),
                    rect(B)
                        .fill(TRANSPARENT)
                        .view()
                        .include(&click)
                        .build(app)
                        .width(100.)
                        .height(100.)
                        .offset_x(100.),
                    rect(C)
                        .fill(TRANSPARENT)
                        .view()
                        .occlude(&click)
                        .build(app)
                        .width(40.)
                        .height(100.)
                        .offset_x(130.),
                ],
            )
            .clipped(rect_path)
            .width(200.)
            .height(50.)
        }

        let mut state = State::default();
        let mut pane = test_pane(PaneBuilder::new("test", view));
        pane.redraw(&mut state, 300, 200, 1.0);

        let a = pane.location(A).expect("a present");
        let b = pane.location(B).expect("b present");
        let c = pane.location(C).expect("c present");
        pane.click(&mut state, Point::new(a.x, a.y - 25.));
        pane.click(&mut state, Point::new(b.x - 45., b.y - 25.));
        pane.click(&mut state, Point::new(c.x, c.y - 25.));
        pane.click(&mut state, Point::new(a.x, a.y + 25.));

        assert_eq!(state.clicks, 2);
    }

    #[test]
    fn multiply_occluded_region_can_be_clipped() {
        const A: u64 = 9070;
        const B: u64 = 9071;
        const C: u64 = 9072;

        fn view<'a>(_: &'a State, app: &mut PaneState) -> View<'a, State> {
            let click = click();
            stack_aligned(
                Align::TopLeading,
                vec![
                    rect(A)
                        .fill(TRANSPARENT)
                        .view()
                        .include(&click)
                        .build(app)
                        .width(180.)
                        .height(100.),
                    rect(B)
                        .fill(TRANSPARENT)
                        .view()
                        .occlude(&click)
                        .build(app)
                        .width(30.)
                        .height(100.)
                        .offset_x(20.),
                    rect(C)
                        .fill(TRANSPARENT)
                        .view()
                        .occlude(&click)
                        .build(app)
                        .width(30.)
                        .height(100.)
                        .offset_x(130.),
                ],
            )
            .clipped(rect_path)
            .width(180.)
            .height(50.)
        }

        let mut state = State::default();
        let mut pane = test_pane(PaneBuilder::new("test", view));
        pane.redraw(&mut state, 300, 200, 1.0);

        let a = pane.location(A).expect("a present");
        let b = pane.location(B).expect("b present");
        let c = pane.location(C).expect("c present");
        pane.click(&mut state, Point::new(a.x, a.y - 25.));
        pane.click(&mut state, Point::new(b.x, b.y - 25.));
        pane.click(&mut state, Point::new(c.x, c.y - 25.));
        pane.click(&mut state, Point::new(a.x, a.y + 25.));

        assert_eq!(state.clicks, 1);
    }

    #[test]
    fn overlapping_includes_for_one_gesture_fire_once() {
        const A: u64 = 9080;
        const B: u64 = 9081;

        fn view<'a>(_: &'a State, app: &mut PaneState) -> View<'a, State> {
            let click = click();
            stack(vec![
                rect(A)
                    .fill(TRANSPARENT)
                    .view()
                    .include(&click)
                    .build(app)
                    .width(80.)
                    .height(80.),
                rect(B)
                    .fill(TRANSPARENT)
                    .view()
                    .include(&click)
                    .build(app)
                    .width(80.)
                    .height(80.),
            ])
        }

        let mut state = State::default();
        let mut pane = test_pane(PaneBuilder::new("test", view));
        pane.redraw(&mut state, 300, 200, 1.0);

        click_target(&mut pane, &mut state, A);

        assert_eq!(state.clicks, 1);
    }

    #[test]
    fn occlusion_only_affects_the_matching_gesture() {
        const A: u64 = 9090;
        const B: u64 = 9091;

        fn view<'a>(_: &'a State, app: &mut PaneState) -> View<'a, State> {
            let click = click();
            let other_click = other_click();
            stack(vec![
                rect(A)
                    .fill(TRANSPARENT)
                    .view()
                    .include(&click)
                    .include(&other_click)
                    .build(app)
                    .width(80.)
                    .height(80.),
                rect(B)
                    .fill(TRANSPARENT)
                    .view()
                    .occlude(&click)
                    .build(app)
                    .width(80.)
                    .height(80.),
            ])
        }

        let mut state = State::default();
        let mut pane = test_pane(PaneBuilder::new("test", view));
        pane.redraw(&mut state, 300, 200, 1.0);

        click_target(&mut pane, &mut state, B);

        assert_eq!(state.clicks, 0);
        assert_eq!(state.other_clicks, 1);
    }
}
