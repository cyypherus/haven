#[cfg(feature = "winit")]
use crate::MouseButton;
use crate::{Key, Modifiers, NamedKey};

#[cfg(feature = "vello")]
use crate::app::{Pane, PaneEffect};
#[cfg(feature = "debug-overlay")]
use crate::primitives::{PathData, text};
#[cfg(feature = "debug-overlay")]
use crate::render::Frame;
#[cfg(feature = "debug-overlay")]
use crate::{Area, Color, Stroke};
#[cfg(feature = "vello")]
use crate::{PaneBuilder, ScrollDelta};
#[cfg(feature = "vello")]
use std::collections::HashMap;
#[cfg(feature = "debug-overlay")]
use std::collections::VecDeque;
#[cfg(feature = "vello")]
use std::sync::Arc;
#[cfg(feature = "debug-overlay")]
use std::time::Instant;
#[cfg(feature = "vello")]
use winit::application::ApplicationHandler;
#[cfg(feature = "vello")]
use winit::dpi::LogicalSize;
#[cfg(feature = "vello")]
use winit::event::MouseScrollDelta;
#[cfg(feature = "vello")]
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
#[cfg(feature = "vello")]
use winit::window::{Icon, Window as WinitWindow, WindowId};

#[cfg(all(feature = "vello", target_os = "macos"))]
use winit::platform::macos::WindowAttributesExtMacOS;
#[cfg(all(feature = "vello", target_os = "windows"))]
use winit::platform::windows::WindowAttributesExtWindows;

pub(crate) fn key(value: winit::keyboard::Key) -> Option<Key> {
    match value {
        winit::keyboard::Key::Named(named_key) => named_key_from_winit(named_key).map(Key::Named),
        winit::keyboard::Key::Character(c) => Some(Key::Character(c.to_string())),
        winit::keyboard::Key::Unidentified(_) | winit::keyboard::Key::Dead(_) => None,
    }
}

pub(crate) fn modifiers(value: winit::event::Modifiers) -> Modifiers {
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
    window_icon: Option<Icon>,
}

#[cfg(feature = "vello")]
struct WinitSurface<State> {
    surface: Surface,
    window: Arc<WinitWindow>,
    pane: Pane<State>,
    #[cfg(feature = "debug-overlay")]
    debug_overlay: DebugOverlayState,
}

#[cfg(feature = "debug-overlay")]
#[derive(Default)]
struct DebugOverlayState {
    last_redraw: Option<Instant>,
    frame_ms: Option<f64>,
    smoothed_fps: Option<f64>,
    frame_times: VecDeque<(Instant, f64)>,
}

#[cfg(feature = "debug-overlay")]
impl DebugOverlayState {
    fn update(&mut self, target_frame_ms: f64) -> DebugOverlayMetrics {
        let now = Instant::now();
        let redraw_interval_ms = self.last_redraw.and_then(|last_redraw| {
            let elapsed = now.duration_since(last_redraw).as_secs_f64();
            (elapsed > 0.).then_some(elapsed * 1000.)
        });
        self.last_redraw = Some(now);

        if let Some(redraw_interval_ms) = redraw_interval_ms {
            let fps = 1000. / redraw_interval_ms;
            let smoothed_fps = self
                .smoothed_fps
                .map(|previous| previous * 0.85 + fps * 0.15)
                .unwrap_or(fps);
            self.smoothed_fps = Some(smoothed_fps);
        }

        while self
            .frame_times
            .front()
            .is_some_and(|(sample_time, _)| now.duration_since(*sample_time).as_secs_f64() > 1.)
        {
            self.frame_times.pop_front();
        }

        DebugOverlayMetrics {
            fps: self.smoothed_fps,
            max_fps: fps_from_frame_ms(self.frame_ms),
            frame_ms: self.frame_ms,
            budget_percent: percent_of(self.frame_ms, target_frame_ms),
            p99_ms: percentile(&self.frame_times, 0.99),
            max_ms: self
                .frame_times
                .iter()
                .map(|(_, frame_ms)| *frame_ms)
                .reduce(f64::max),
        }
    }

    fn append_to<State>(
        &mut self,
        frame: &mut Frame,
        pane: &mut Pane<State>,
        target_frame_ms: f64,
    ) {
        let label = self.update(target_frame_ms).label();
        let line_count = label.lines().count() as f32;
        let max_line_len = label.lines().map(str::len).max().unwrap_or_default() as f32;
        let width = 16. + max_line_len * 7.;
        let height = 12. + line_count * 14.;
        let logical_width = frame.width as f32 / frame.scale_factor as f32;
        let logical_height = frame.height as f32 / frame.scale_factor as f32;
        let background_area = Area {
            x: (logical_width - width - 8.).max(8.),
            y: (logical_height - height - 8.).max(8.),
            width,
            height,
        };
        let text_area = Area {
            x: background_area.x + 6.,
            y: background_area.y + 5.,
            width: width - 12.,
            height: height - 10.,
        };

        frame.items.push(crate::render::RenderItem::Path {
            path: Box::new(PathData {
                id: crate::const_hash(file!(), line!(), column!()),
                builder: crate::primitives::shape::rect_path((5., 5., 5., 5.)),
                fill: Some(Color::from_rgb8(0, 0, 0).with_alpha(0.68).into()),
                stroke: Some((
                    Color::from_rgb8(255, 255, 255).with_alpha(0.18).into(),
                    Stroke::new(1.),
                )),
            }),
            area: background_area,
        });
        frame.items.push(
            text(crate::const_hash(file!(), line!(), column!()), label)
                .font_size(12)
                .align(parley::Alignment::Start)
                .fill(Color::from_rgb8(245, 245, 245))
                .render_item(frame.scale_factor, text_area, &mut pane.pane_state),
        );
    }

    fn finish_frame(&mut self, frame_started: Instant) {
        let now = Instant::now();
        let frame_ms = now.duration_since(frame_started).as_secs_f64() * 1000.;
        self.frame_ms = Some(frame_ms);
        self.frame_times.push_back((now, frame_ms));
    }
}

#[cfg(feature = "debug-overlay")]
struct DebugOverlayMetrics {
    fps: Option<f64>,
    max_fps: Option<f64>,
    frame_ms: Option<f64>,
    budget_percent: Option<f64>,
    p99_ms: Option<f64>,
    max_ms: Option<f64>,
}

#[cfg(feature = "debug-overlay")]
impl DebugOverlayMetrics {
    fn label(&self) -> String {
        format!(
            "FPS {}\nmax FPS {}\nframe {}\nbudget {}\n1s p99 {} max {}",
            format_value(self.fps, 4, 1),
            format_value(self.max_fps, 4, 1),
            format_ms(self.frame_ms),
            format_percent(self.budget_percent),
            format_ms(self.p99_ms),
            format_ms(self.max_ms),
        )
    }
}

#[cfg(feature = "debug-overlay")]
fn target_frame_ms(refresh_rate_millihertz: Option<u32>) -> f64 {
    let refresh_rate_hz = refresh_rate_millihertz
        .filter(|refresh_rate| *refresh_rate > 0)
        .map(|refresh_rate| refresh_rate as f64 / 1000.)
        .unwrap_or(60.);
    1000. / refresh_rate_hz
}

#[cfg(feature = "debug-overlay")]
fn percent_of(value: Option<f64>, target: f64) -> Option<f64> {
    value.and_then(|value| (target > 0.).then_some(value / target * 100.))
}

#[cfg(feature = "debug-overlay")]
fn fps_from_frame_ms(frame_ms: Option<f64>) -> Option<f64> {
    frame_ms.and_then(|frame_ms| (frame_ms > 0.).then_some(1000. / frame_ms))
}

#[cfg(feature = "debug-overlay")]
fn percentile(samples: &VecDeque<(Instant, f64)>, percentile: f64) -> Option<f64> {
    if samples.is_empty() {
        return None;
    }

    let mut sorted = samples
        .iter()
        .map(|(_, frame_ms)| *frame_ms)
        .collect::<Vec<_>>();
    sorted.sort_by(f64::total_cmp);
    let index = ((sorted.len() as f64 * percentile).ceil() as usize)
        .saturating_sub(1)
        .min(sorted.len() - 1);
    Some(sorted[index])
}

#[cfg(feature = "debug-overlay")]
fn format_ms(value: Option<f64>) -> String {
    match value {
        Some(value) => format!("{value:>4.1}ms"),
        None => "--.-ms".to_string(),
    }
}

#[cfg(feature = "debug-overlay")]
fn format_percent(value: Option<f64>) -> String {
    match value {
        Some(value) => format!("{value:>5.0}%"),
        None => " ----%".to_string(),
    }
}

#[cfg(feature = "debug-overlay")]
fn format_value(value: Option<f64>, width: usize, precision: usize) -> String {
    match value {
        Some(value) => format!("{value:>width$.precision$}"),
        None => "-".repeat(width + 1 + precision),
    }
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
            window_icon: None,
        }
    }

    pub fn window_icon(mut self, icon: Icon) -> Self {
        self.window_icon = Some(icon);
        self
    }

    pub fn pane(mut self, pane: PaneBuilder<State>) -> Self {
        self.panes.insert(pane.name, pane);
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

        let inner_size = config.inner_size.unwrap_or((1044, 800));
        let resizable = config.resizable.unwrap_or(true);
        let transparent = config.transparent.unwrap_or(false);
        let decorations = config.decorations.unwrap_or(true);

        #[cfg(target_os = "macos")]
        let mut attributes = WinitWindow::default_attributes()
            .with_inner_size(LogicalSize::new(inner_size.0, inner_size.1))
            .with_resizable(resizable)
            .with_transparent(transparent)
            .with_decorations(decorations)
            .with_window_icon(self.window_icon.clone())
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
            .with_window_icon(self.window_icon.clone())
            .with_taskbar_icon(self.window_icon.clone())
            .with_visible(false);

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let mut attributes = WinitWindow::default_attributes()
            .with_inner_size(LogicalSize::new(inner_size.0, inner_size.1))
            .with_resizable(resizable)
            .with_transparent(transparent)
            .with_decorations(decorations)
            .with_window_icon(self.window_icon.clone());

        if let Some(ref title) = config.title {
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

        let pane_name = config.name;
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
                #[cfg(feature = "debug-overlay")]
                debug_overlay: DebugOverlayState::default(),
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
        #[cfg(feature = "debug-overlay")]
        let frame_started = Instant::now();

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

        #[cfg(feature = "debug-overlay")]
        let mut frame = frame;

        #[cfg(feature = "debug-overlay")]
        let target_frame_ms = target_frame_ms(
            surface
                .window
                .current_monitor()
                .and_then(|monitor| monitor.refresh_rate_millihertz()),
        );

        #[cfg(feature = "debug-overlay")]
        surface
            .debug_overlay
            .append_to(&mut frame, &mut surface.pane, target_frame_ms);

        let window = surface.window.clone();
        self.renderer.render(&mut surface.surface, &frame, || {
            window.pre_present_notify();
        });

        #[cfg(feature = "debug-overlay")]
        surface.debug_overlay.finish_frame(frame_started);

        effects
    }
}

#[cfg(feature = "vello")]
impl<State: 'static> ApplicationHandler<WinitEvent> for WinitApp<State> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let panes: Vec<_> = self
            .panes
            .iter()
            .filter_map(|(name, pane)| pane.open_at_start.then_some(*name))
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
        let mut redraw_all_now = false;
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
                    redraw_all_now = true;
                    let position = position.to_logical(surface.window.scale_factor());
                    surface
                        .pane
                        .move_to(&mut self.state, kurbo::Point::new(position.x, position.y))
                } else {
                    Vec::new()
                }
            }
            winit::event::WindowEvent::MouseInput { state, button, .. } => {
                if let Some(surface) = self.windows.get_mut(&window_id) {
                    invalidate_all = true;
                    let button = mouse_button(button);
                    match state {
                        winit::event::ElementState::Pressed => {
                            surface.pane.press_button(&mut self.state, button)
                        }
                        winit::event::ElementState::Released => {
                            surface.pane.release_button(&mut self.state, button)
                        }
                    }
                } else {
                    Vec::new()
                }
            }
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
            winit::event::WindowEvent::Resized(_) => {
                if let Some(surface) = self.windows.get(&window_id) {
                    surface.window.request_redraw();
                }
                Vec::new()
            }
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
        self.apply_effects(event_loop, window_id, effects);
        if redraw_all_now {
            let window_ids = self.windows.keys().copied().collect::<Vec<_>>();
            for window_id in window_ids {
                let effects = self.redraw(window_id);
                self.apply_effects(event_loop, window_id, effects);
            }
        } else if invalidate_all {
            self.request_all_redraws();
        }
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

#[cfg(feature = "winit")]
fn mouse_button(value: winit::event::MouseButton) -> MouseButton {
    match value {
        winit::event::MouseButton::Left => MouseButton::Left,
        winit::event::MouseButton::Right => MouseButton::Right,
        winit::event::MouseButton::Middle => MouseButton::Middle,
        winit::event::MouseButton::Back => MouseButton::Back,
        winit::event::MouseButton::Forward => MouseButton::Forward,
        winit::event::MouseButton::Other(value) => MouseButton::Other(value),
    }
}
