use crate::{Key, Modifiers, NamedKey, Pane, PaneConfig, PaneEffect, Redraw, ScrollDelta};
use std::collections::HashMap;
use std::sync::Arc;
use vello_svg::vello::util::{RenderContext, RenderSurface};
use vello_svg::vello::{Renderer, RendererOptions};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::MouseScrollDelta;
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::window::{Window as WinitWindow, WindowId};

#[cfg(target_os = "macos")]
use winit::platform::macos::WindowAttributesExtMacOS;

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

enum WinitEvent {
    Redraw(WindowId),
}

pub struct WinitApp<State> {
    state: Option<State>,
    panes: HashMap<&'static str, PaneConfig<State>>,
    windows: HashMap<WindowId, WinitSurface<State>>,
    pane_windows: HashMap<&'static str, WindowId>,
    render_context: RenderContext,
    renderers: Vec<Option<Renderer>>,
    proxy: Option<EventLoopProxy<WinitEvent>>,
}

struct WinitSurface<State> {
    surface: RenderSurface<'static>,
    window: Arc<WinitWindow>,
    pane: Pane<State>,
}

impl<State: Clone + 'static> WinitApp<State> {
    pub fn new(state: State) -> Self {
        Self {
            state: Some(state),
            panes: HashMap::new(),
            windows: HashMap::new(),
            pane_windows: HashMap::new(),
            render_context: RenderContext::new(),
            renderers: Vec::new(),
            proxy: None,
        }
    }

    pub fn pane(mut self, pane: PaneConfig<State>) -> Self {
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
        let surface_future = self.render_context.create_surface(
            window.clone(),
            size.width,
            size.height,
            vello_svg::vello::wgpu::PresentMode::AutoNoVsync,
        );
        let mut surface = pollster::block_on(surface_future).expect("Error creating surface");

        if transparent {
            let device = &self.render_context.devices[surface.dev_id].device;
            let capabilities = surface
                .surface
                .get_capabilities(self.render_context.devices[surface.dev_id].adapter());
            if capabilities
                .alpha_modes
                .contains(&wgpu::CompositeAlphaMode::PostMultiplied)
            {
                surface.config.alpha_mode = wgpu::CompositeAlphaMode::PostMultiplied;
            }
            surface.surface.configure(device, &surface.config);
        }

        let dev_id = surface.dev_id;
        self.renderers
            .resize_with(self.render_context.devices.len(), || None);
        self.renderers[dev_id].get_or_insert_with(|| {
            Renderer::new(
                &self.render_context.devices[dev_id].device,
                RendererOptions::default(),
            )
            .expect("Failed to create renderer")
        });

        #[cfg(target_os = "windows")]
        window.set_visible(true);

        let Some(proxy) = self.proxy.clone() else {
            return;
        };
        let window_id = window.id();
        let redraw = Redraw::new(move || {
            let _ = proxy.send_event(WinitEvent::Redraw(window_id));
        });
        let state = self
            .state
            .as_ref()
            .expect("state must exist while creating windows")
            .clone();
        let pane_name = config.name();
        let pane = config.build(state, redraw);
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

    fn close_window(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId) {
        if let Some(surface) = self.windows.remove(&window_id) {
            let name = surface.pane.name();
            surface.pane.close();
            self.pane_windows.remove(name);
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
        if surface.surface.config.width != width || surface.surface.config.height != height {
            self.render_context
                .resize_surface(&mut surface.surface, width, height);
        }

        let effects = surface
            .pane
            .redraw(width, height, surface.window.scale_factor());

        let device_handle = &self.render_context.devices[surface.surface.dev_id];
        let render_params = vello_svg::vello::RenderParams {
            base_color: surface.pane.base_color(),
            width,
            height,
            antialiasing_method: vello_svg::vello::AaConfig::Msaa8,
        };

        surface.window.pre_present_notify();

        self.renderers[surface.surface.dev_id]
            .as_mut()
            .unwrap()
            .render_to_texture(
                &device_handle.device,
                &device_handle.queue,
                surface.pane.scene(),
                &surface.surface.target_view,
                &render_params,
            )
            .expect("failed to render to texture");

        let surface_texture = surface
            .surface
            .surface
            .get_current_texture()
            .expect("failed to get surface texture");

        let mut encoder =
            device_handle
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Surface Blit"),
                });
        surface.surface.blitter.copy(
            &device_handle.device,
            &mut encoder,
            &surface.surface.target_view,
            &surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );
        device_handle.queue.submit([encoder.finish()]);
        surface_texture.present();
        surface.pane.reset_scene();
        effects
    }
}

impl<State: Clone + 'static> ApplicationHandler<WinitEvent> for WinitApp<State> {
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

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: WinitEvent) {
        match event {
            WinitEvent::Redraw(window_id) => {
                if let Some(surface) = self.windows.get(&window_id) {
                    surface.window.request_redraw();
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: winit::event::WindowEvent,
    ) {
        let effects = match event {
            winit::event::WindowEvent::Moved(_) => Vec::new(),
            winit::event::WindowEvent::KeyboardInput { event, .. } => {
                if event.state != winit::event::ElementState::Pressed {
                    return;
                }
                let Some(key) = key(event.logical_key) else {
                    return;
                };
                self.windows
                    .get_mut(&window_id)
                    .map(|surface| surface.pane.key_pressed(key))
                    .unwrap_or_default()
            }
            winit::event::WindowEvent::CursorMoved { position, .. } => self
                .windows
                .get_mut(&window_id)
                .map(|surface| {
                    surface
                        .pane
                        .move_to(vello_svg::vello::kurbo::Point::new(position.x, position.y))
                })
                .unwrap_or_default(),
            winit::event::WindowEvent::MouseInput {
                state,
                button: winit::event::MouseButton::Left,
                ..
            } => self
                .windows
                .get_mut(&window_id)
                .map(|surface| match state {
                    winit::event::ElementState::Pressed => surface.pane.press_current(),
                    winit::event::ElementState::Released => surface.pane.release_current(),
                })
                .unwrap_or_default(),
            winit::event::WindowEvent::MouseInput { .. } => Vec::new(),
            winit::event::WindowEvent::CursorEntered { .. } => Vec::new(),
            winit::event::WindowEvent::CursorLeft { .. } => self
                .windows
                .get_mut(&window_id)
                .map(|surface| surface.pane.exit())
                .unwrap_or_default(),
            winit::event::WindowEvent::MouseWheel { delta, .. } => self
                .windows
                .get_mut(&window_id)
                .map(|surface| surface.pane.scroll(scroll_delta(delta)))
                .unwrap_or_default(),
            winit::event::WindowEvent::Resized(_) => Vec::new(),
            winit::event::WindowEvent::HoveredFile(_) => Vec::new(),
            winit::event::WindowEvent::DroppedFile(_) => Vec::new(),
            winit::event::WindowEvent::HoveredFileCancelled => Vec::new(),
            winit::event::WindowEvent::Touch(_) => Vec::new(),
            winit::event::WindowEvent::TouchpadPressure { .. } => Vec::new(),
            winit::event::WindowEvent::Focused(focused) => {
                if focused {
                    if let Some(surface) = self.windows.get(&window_id) {
                        surface.window.request_redraw();
                    }
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
        self.apply_effects(event_loop, window_id, effects);
    }
}

fn scroll_delta(delta: MouseScrollDelta) -> ScrollDelta {
    match delta {
        MouseScrollDelta::LineDelta(x, y) => ScrollDelta {
            x: x * 10.,
            y: y * 10.,
        },
        MouseScrollDelta::PixelDelta(physical_position) => ScrollDelta {
            x: physical_position.x as f32,
            y: physical_position.y as f32,
        },
    }
}
