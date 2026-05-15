use crate::{Key, Modifiers, NamedKey};

#[cfg(feature = "vello")]
use crate::{Pane, PaneBuilder, PaneEffect, ScrollDelta};
#[cfg(feature = "vello")]
use std::collections::HashMap;
#[cfg(feature = "vello")]
use std::sync::Arc;
#[cfg(feature = "vello")]
use winit::application::ApplicationHandler;
#[cfg(feature = "vello")]
use winit::dpi::LogicalSize;
#[cfg(feature = "vello")]
use winit::event::MouseScrollDelta;
#[cfg(feature = "vello")]
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
#[cfg(feature = "vello")]
use winit::window::{Window as WinitWindow, WindowId};

#[cfg(all(feature = "vello", target_os = "macos"))]
use winit::platform::macos::WindowAttributesExtMacOS;

pub fn key(value: winit::keyboard::Key) -> Option<Key> {
    match value {
        winit::keyboard::Key::Named(named_key) => named_key_from_winit(named_key).map(Key::Named),
        winit::keyboard::Key::Character(c) => Some(Key::Character(c.to_string())),
        winit::keyboard::Key::Unidentified(_) | winit::keyboard::Key::Dead(_) => None,
    }
}

pub fn modifiers(value: winit::event::Modifiers) -> Modifiers {
    let state = value.state();
    Modifiers {
        shift: state.shift_key(),
        control: state.control_key(),
        alt: state.alt_key(),
        super_key: state.super_key(),
    }
}

fn named_key_from_winit(value: winit::keyboard::NamedKey) -> Option<NamedKey> {
    let key = match value {
        winit::keyboard::NamedKey::Enter => NamedKey::Enter,
        winit::keyboard::NamedKey::Escape => NamedKey::Escape,
        winit::keyboard::NamedKey::Space => NamedKey::Space,
        winit::keyboard::NamedKey::Backspace => NamedKey::Backspace,
        winit::keyboard::NamedKey::Delete => NamedKey::Delete,
        winit::keyboard::NamedKey::ArrowLeft => NamedKey::ArrowLeft,
        winit::keyboard::NamedKey::ArrowRight => NamedKey::ArrowRight,
        winit::keyboard::NamedKey::ArrowUp => NamedKey::ArrowUp,
        winit::keyboard::NamedKey::ArrowDown => NamedKey::ArrowDown,
        winit::keyboard::NamedKey::Home => NamedKey::Home,
        winit::keyboard::NamedKey::End => NamedKey::End,
        winit::keyboard::NamedKey::Tab => NamedKey::Tab,
        _ => return None,
    };
    Some(key)
}

#[cfg(feature = "vello")]
enum WinitEvent {
    Wake(WindowId),
}

#[cfg(feature = "vello")]
use crate::renderers::vello::{VelloRenderer as Renderer, VelloSurface as Surface};

#[cfg(feature = "vello")]
pub struct WinitApp<State> {
    state: State,
    panes: HashMap<&'static str, PaneBuilder<State>>,
    windows: HashMap<WindowId, WinitSurface<State>>,
    pane_windows: HashMap<&'static str, WindowId>,
    renderer: Renderer,
    proxy: Option<EventLoopProxy<WinitEvent>>,
}

#[cfg(feature = "vello")]
struct WinitSurface<State> {
    surface: Surface,
    window: Arc<WinitWindow>,
    pane: Pane<State>,
}

#[cfg(feature = "vello")]
impl<State: 'static> WinitApp<State> {
    pub fn new(state: State) -> Self {
        Self {
            state,
            panes: HashMap::new(),
            windows: HashMap::new(),
            pane_windows: HashMap::new(),
            renderer: Renderer::new(),
            proxy: None,
        }
    }

    pub fn pane(mut self, pane: PaneBuilder<State>) -> Self {
        self.panes.insert(pane.name(), pane);
        self
    }

    pub fn run(mut self) {
        let event_loop = EventLoop::with_user_event()
            .build()
            .expect("Could not create event loop");
        self.proxy = Some(event_loop.create_proxy());
        event_loop.run_app(&mut self).expect("run to completion");
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop, name: &'static str) {
        if let Some(window_id) = self.pane_windows.get(name).copied()
            && let Some(surface) = self.windows.get(&window_id)
        {
            surface.window.focus_window();
            return;
        }

        let Some(config) = self.panes.get(name).cloned() else {
            return;
        };

        let inner_size = config.inner_size_value().unwrap_or((1044, 800));
        let resizable = config.resizable_value().unwrap_or(true);
        let transparent = config.transparent_value().unwrap_or(false);
        let decorations = config.decorations_value().unwrap_or(true);

        #[cfg(target_os = "macos")]
        let mut attributes = WinitWindow::default_attributes()
            .with_inner_size(LogicalSize::new(inner_size.0, inner_size.1))
            .with_resizable(resizable)
            .with_transparent(transparent)
            .with_decorations(decorations)
            .with_titlebar_hidden(false)
            .with_titlebar_transparent(true)
            .with_title_hidden(true)
            .with_fullsize_content_view(true);

        #[cfg(target_os = "windows")]
        let mut attributes = WinitWindow::default_attributes()
            .with_inner_size(LogicalSize::new(inner_size.0, inner_size.1))
            .with_resizable(resizable)
            .with_transparent(transparent)
            .with_decorations(decorations)
            .with_visible(false);

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let mut attributes = WinitWindow::default_attributes()
            .with_inner_size(LogicalSize::new(inner_size.0, inner_size.1))
            .with_resizable(resizable)
            .with_transparent(transparent)
            .with_decorations(decorations);

        if let Some(title) = config.title_value() {
            attributes = attributes.with_title(title.to_string());
        }

        let window = Arc::new(event_loop.create_window(attributes).unwrap());
        let size = window.inner_size();
        let window_id = window.id();
        let surface =
            self.renderer
                .create_surface(window.clone(), size.width, size.height, transparent);

        #[cfg(target_os = "windows")]
        window.set_visible(true);

        let pane_name = config.name();
        let mut pane = config.build();
        if let Some(proxy) = self.proxy.clone() {
            pane.set_wake_handler(Arc::new(move || {
                let _ = proxy.send_event(WinitEvent::Wake(window_id));
            }));
        }
        self.pane_windows.insert(pane_name, window_id);
        self.windows.insert(
            window_id,
            WinitSurface {
                surface,
                window,
                pane,
            },
        );
    }

    fn apply_effects(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        effects: Vec<PaneEffect>,
    ) {
        for effect in effects {
            match effect {
                PaneEffect::Open(name) => self.create_window(event_loop, name),
                PaneEffect::Close => self.close_window(event_loop, window_id),
                PaneEffect::Redraw => {
                    if let Some(surface) = self.windows.get(&window_id) {
                        surface.window.request_redraw();
                    }
                }
            }
        }
    }

    fn request_all_redraws(&self) {
        for surface in self.windows.values() {
            surface.window.request_redraw();
        }
    }

    fn close_window(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId) {
        if let Some(surface) = self.windows.remove(&window_id) {
            let name = surface.pane.name();
            surface.pane.close(&mut self.state);
            self.pane_windows.remove(name);
            self.request_all_redraws();
        }
        if self.windows.is_empty() {
            event_loop.exit();
        }
    }

    fn redraw(&mut self, window_id: WindowId) -> Vec<PaneEffect> {
        let Some(surface) = self.windows.get_mut(&window_id) else {
            return Vec::new();
        };

        let size = surface.window.inner_size();
        let width = size.width;
        let height = size.height;
        self.renderer.resize(&mut surface.surface, width, height);

        let (frame, effects) = surface.pane.redraw(
            &mut self.state,
            width,
            height,
            surface.window.scale_factor(),
        );

        surface.window.pre_present_notify();
        self.renderer.render(&mut surface.surface, &frame);
        effects
    }
}

#[cfg(feature = "vello")]
impl<State: 'static> ApplicationHandler<WinitEvent> for WinitApp<State> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let panes: Vec<_> = self
            .panes
            .iter()
            .filter_map(|(name, pane)| pane.open_at_start_value().then_some(*name))
            .collect();
        for name in panes {
            self.create_window(event_loop, name);
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: WinitEvent) {
        match event {
            WinitEvent::Wake(window_id) => {
                let effects = if let Some(surface) = self.windows.get_mut(&window_id) {
                    surface.window.request_redraw();
                    surface.pane.wake(&mut self.state)
                } else {
                    Vec::new()
                };
                self.apply_effects(event_loop, window_id, effects);
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: winit::event::WindowEvent,
    ) {
        let mut invalidate_all = false;
        let effects = match event {
            winit::event::WindowEvent::Moved(_) => Vec::new(),
            winit::event::WindowEvent::KeyboardInput { event, .. } => {
                let Some(key) = key(event.logical_key) else {
                    return;
                };
                if let Some(surface) = self.windows.get_mut(&window_id) {
                    invalidate_all = true;
                    match event.state {
                        winit::event::ElementState::Pressed => {
                            surface.pane.key_pressed(&mut self.state, key)
                        }
                        winit::event::ElementState::Released => {
                            surface.pane.key_released(&mut self.state, key)
                        }
                    }
                } else {
                    Vec::new()
                }
            }
            winit::event::WindowEvent::CursorMoved { position, .. } => {
                if let Some(surface) = self.windows.get_mut(&window_id) {
                    invalidate_all = true;
                    surface
                        .pane
                        .move_to(&mut self.state, kurbo::Point::new(position.x, position.y))
                } else {
                    Vec::new()
                }
            }
            winit::event::WindowEvent::MouseInput {
                state,
                button: winit::event::MouseButton::Left,
                ..
            } => {
                if let Some(surface) = self.windows.get_mut(&window_id) {
                    invalidate_all = true;
                    match state {
                        winit::event::ElementState::Pressed => surface.pane.press(&mut self.state),
                        winit::event::ElementState::Released => {
                            surface.pane.release(&mut self.state)
                        }
                    }
                } else {
                    Vec::new()
                }
            }
            winit::event::WindowEvent::MouseInput { .. } => Vec::new(),
            winit::event::WindowEvent::CursorEntered { .. } => Vec::new(),
            winit::event::WindowEvent::CursorLeft { .. } => {
                if let Some(surface) = self.windows.get_mut(&window_id) {
                    invalidate_all = true;
                    surface.pane.exit(&mut self.state)
                } else {
                    Vec::new()
                }
            }
            winit::event::WindowEvent::MouseWheel { delta, .. } => {
                if let Some(surface) = self.windows.get_mut(&window_id) {
                    invalidate_all = true;
                    surface.pane.scroll(&mut self.state, scroll_delta(delta))
                } else {
                    Vec::new()
                }
            }
            winit::event::WindowEvent::Resized(_) => Vec::new(),
            winit::event::WindowEvent::HoveredFile(_) => Vec::new(),
            winit::event::WindowEvent::DroppedFile(_) => Vec::new(),
            winit::event::WindowEvent::HoveredFileCancelled => Vec::new(),
            winit::event::WindowEvent::Touch(_) => Vec::new(),
            winit::event::WindowEvent::TouchpadPressure { .. } => Vec::new(),
            winit::event::WindowEvent::Focused(focused) => {
                if focused && let Some(surface) = self.windows.get(&window_id) {
                    surface.window.request_redraw();
                }
                Vec::new()
            }
            winit::event::WindowEvent::CloseRequested | winit::event::WindowEvent::Destroyed => {
                self.close_window(event_loop, window_id);
                Vec::new()
            }
            winit::event::WindowEvent::RedrawRequested => self.redraw(window_id),
            winit::event::WindowEvent::ScaleFactorChanged { scale_factor, .. } => self
                .windows
                .get_mut(&window_id)
                .map(|surface| surface.pane.scale_factor_changed(scale_factor))
                .unwrap_or_default(),
            winit::event::WindowEvent::ModifiersChanged(modifiers) => self
                .windows
                .get_mut(&window_id)
                .map(|surface| surface.pane.modifiers_changed(self::modifiers(modifiers)))
                .unwrap_or_default(),
            winit::event::WindowEvent::AxisMotion { .. }
            | winit::event::WindowEvent::ThemeChanged(_)
            | winit::event::WindowEvent::Ime(_)
            | winit::event::WindowEvent::Occluded(_)
            | winit::event::WindowEvent::ActivationTokenDone { .. }
            | winit::event::WindowEvent::PinchGesture { .. }
            | winit::event::WindowEvent::PanGesture { .. }
            | winit::event::WindowEvent::DoubleTapGesture { .. }
            | winit::event::WindowEvent::RotationGesture { .. } => Vec::new(),
        };
        if invalidate_all {
            self.request_all_redraws();
        }
        self.apply_effects(event_loop, window_id, effects);
    }
}

#[cfg(feature = "vello")]
fn scroll_delta(delta: MouseScrollDelta) -> ScrollDelta {
    match delta {
        MouseScrollDelta::LineDelta(x, y) => ScrollDelta { x, y },
        MouseScrollDelta::PixelDelta(physical_position) => ScrollDelta {
            x: physical_position.x as f32,
            y: physical_position.y as f32,
        },
    }
}
