use crate::draw_layout::draw_layout;
use crate::gestures::{ClickLocation, Interaction, ScrollDelta};

use crate::text::TextLayout;
use crate::view::DrawableType;
use crate::{ClickState, DragState, Editor, GestureHandler, Point, area_contains};
use crate::{GestureState, RUBIK_FONT, area_contains_padded, event};
use backer::{Area, Layout};
use parley::fontique::Blob;
use parley::fontique::FontInfoOverride;
use parley::{
    Alignment, FontContext, FontWeight, LayoutContext, LineHeight, OverflowWrap, PlainEditor,
    StyleProperty,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use vello_svg::vello::kurbo::{Affine, BezPath};
use vello_svg::vello::peniko::{Brush, Color, Fill, Mix};
use vello_svg::vello::util::{RenderContext, RenderSurface};
use vello_svg::vello::{Renderer, RendererOptions, Scene};
use winit::event::{Modifiers, MouseScrollDelta};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Fullscreen, WindowId};
use winit::{
    application::ApplicationHandler, event_loop::EventLoop, window::Window as WinitWindow,
};
use winit::{dpi::LogicalSize, event::MouseButton};

#[cfg(target_os = "macos")]
use winit::platform::macos::WindowAttributesExtMacOS;

type FontEntry = (Arc<Vec<u8>>, Option<String>);

type ViewFn<State> = for<'a> fn(&'a State, &mut AppState) -> Layout<'a, View<State>, AppCtx>;

pub struct Window<State> {
    name: &'static str,
    view: ViewFn<State>,
    inner_size: Option<(u32, u32)>,
    resizable: Option<bool>,
    title: Option<String>,
    transparent: Option<bool>,
    decorations: Option<bool>,
    open_at_start: bool,
}

impl<State> Window<State> {
    pub fn new(name: &'static str, view: ViewFn<State>) -> Self {
        Self {
            name,
            view,
            inner_size: None,
            resizable: None,
            title: None,
            transparent: None,
            decorations: None,
            open_at_start: true,
        }
    }

    pub fn inner_size(mut self, width: u32, height: u32) -> Self {
        self.inner_size = Some((width, height));
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = Some(resizable);
        self
    }

    pub fn title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn transparent(mut self, transparent: bool) -> Self {
        self.transparent = Some(transparent);
        self
    }

    pub fn decorations(mut self, decorations: bool) -> Self {
        self.decorations = Some(decorations);
        self
    }

    pub fn open_at_start(mut self, open: bool) -> Self {
        self.open_at_start = open;
        self
    }
}

pub struct AppBuilder<State> {
    state: State,
    window_registry: HashMap<&'static str, Window<State>>,
    initial_windows: Vec<&'static str>,
    on_frame: fn(&mut State, &mut AppState) -> (),
    on_start: fn(&mut State, &mut AppState) -> (),
    on_exit: fn(&mut State, &mut AppState) -> (),
    custom_fonts: Vec<FontEntry>,
}

impl<State: 'static> AppBuilder<State> {
    pub fn new(state: State, window: Window<State>) -> Self {
        let mut registry = HashMap::new();
        let mut initial_windows = Vec::new();
        if window.open_at_start {
            initial_windows.push(window.name);
        }
        let name = window.name;
        registry.insert(name, window);
        Self {
            state,
            window_registry: registry,
            initial_windows,
            on_frame: |_, _| {},
            on_start: |_, _| {},
            on_exit: |_, _| {},
            custom_fonts: Vec::new(),
        }
    }

    pub fn window(mut self, window: Window<State>) -> Self {
        if window.open_at_start {
            self.initial_windows.push(window.name);
        }
        let name = window.name;
        self.window_registry.insert(name, window);
        self
    }

    pub fn add_font_bytes(mut self, bytes: Vec<u8>, family: Option<&str>) -> Self {
        self.custom_fonts
            .push((Arc::new(bytes), family.map(|s| s.to_string())));
        self
    }

    pub fn on_frame(mut self, on_frame: fn(&mut State, &mut AppState) -> ()) -> Self {
        self.on_frame = on_frame;
        self
    }

    pub fn on_start(mut self, on_start: fn(&mut State, &mut AppState) -> ()) -> Self {
        self.on_start = on_start;
        self
    }

    pub fn on_exit(mut self, on_exit: fn(&mut State, &mut AppState) -> ()) -> Self {
        self.on_exit = on_exit;
        self
    }

    pub fn start(self) {
        let event_loop: EventLoop<AppEvent> = EventLoop::with_user_event()
            .build()
            .expect("Could not create event loop");
        #[allow(unused_mut)]
        let mut render_cx = RenderContext::new();
        #[cfg(not(target_arch = "wasm32"))]
        {
            App::run(
                self.state,
                event_loop,
                render_cx,
                self.window_registry,
                self.initial_windows,
                self.on_frame,
                self.on_start,
                self.on_exit,
                self.custom_fonts,
            );
        }
    }
}

pub struct App<'s, State> {
    pub(crate) context: RenderContext,
    pub(crate) renderers: Vec<Option<Renderer>>,
    pub(crate) windows: HashMap<winit::window::WindowId, WindowState<'s, State>>,
    pub(crate) window_registry: HashMap<&'static str, Window<State>>,
    pub(crate) initial_windows: Vec<&'static str>,
    pub(crate) app_state: AppState,
    pub state: State,
    pub(crate) on_frame: fn(&mut State, &mut AppState) -> (),
    pub(crate) on_start: fn(&mut State, &mut AppState) -> (),
    pub(crate) on_exit: fn(&mut State, &mut AppState) -> (),
    pub(crate) started: bool,
}

pub(crate) struct WindowState<'surface, State> {
    // SAFETY: We MUST drop the surface before the `window`, so the fields
    // must be in this order
    pub(crate) surface: RenderSurface<'surface>,
    pub(crate) window: Arc<WinitWindow>,
    pub(crate) scene: Scene,
    pub(crate) name: &'static str,
    pub(crate) view: ViewFn<State>,
    pub(crate) gesture_handlers: Vec<(u64, Area, GestureHandler<State, AppState>)>,
    pub(crate) cursor_position: Option<Point>,
    pub(crate) gesture_state: GestureState,
    pub(crate) last_window_size: Option<winit::dpi::PhysicalSize<u32>>,
    pub(crate) fullscreen_requested: bool,
}

pub(crate) type LayoutCache = HashMap<u64, Vec<(String, f32, parley::Layout<Brush>)>>;

pub struct AppCtx {
    pub(crate) text_layout: TextLayout,
    pub(crate) font_cx: FontContext,
    pub(crate) layout_cx: LayoutContext<Brush>,
    pub(crate) scale_factor: f64,
    pub(crate) editor: Option<EditState>,
    pub(crate) editor_areas: HashMap<u64, Area>,
    pub(crate) scrollers: HashMap<u64, crate::scroller::ScrollerState>,
}

pub struct AppState {
    pub(crate) runtime: Runtime,
    pub(crate) cancellation_token: CancellationToken,
    pub(crate) task_tracker: TaskTracker,
    pub(crate) app_context: AppCtx,
    pub(crate) layout_cache: LayoutCache,
    pub(crate) svg_scenes: HashMap<String, (Scene, f32, f32)>,
    pub(crate) image_scenes: HashMap<u64, (Scene, f32, f32)>,
    pub(crate) modifiers: Option<Modifiers>,
    pub(crate) redraw: Sender<()>,
    pub(crate) event_proxy: winit::event_loop::EventLoopProxy<AppEvent>,
    pub(crate) cursor_position: Option<Point>,
}

pub enum View<State> {
    Draw {
        view: Box<DrawableType>,
        gesture_handlers: Vec<GestureHandler<State, AppState>>,
        area: Area,
    },
    PushClip {
        path: BezPath,
    },
    PopClip,
    EditorArea(u64, Area),
    Empty,
}

pub(crate) struct EditState {
    pub(crate) id: u64,
    pub(crate) editor: Editor,
    pub(crate) editing: bool,
    pub(crate) cursor_color: Brush,
    pub(crate) highlight_color: Brush,
}

impl Clone for EditState {
    fn clone(&self) -> Self {
        EditState {
            id: self.id,
            editor: self.editor.clone(),
            editing: self.editing,
            cursor_color: self.cursor_color.clone(),
            highlight_color: self.highlight_color.clone(),
        }
    }
}

impl AppState {
    pub fn ctx(&mut self) -> &mut AppCtx {
        &mut self.app_context
    }

    pub fn end_editing(&mut self) {
        if self.app_context.editor.is_some() {
            self.app_context.editor = None;
        }
    }

    pub fn open_window(&mut self, name: &'static str) {
        let _ = self.event_proxy.send_event(AppEvent::OpenWindow(name));
    }

    pub fn close_window(&mut self, id: winit::window::WindowId) {
        let _ = self.event_proxy.send_event(AppEvent::CloseWindow(id));
    }

    pub(crate) fn begin_editing(
        &mut self,
        id: u64,
        text: String,
        fill: Brush,
        font_family: String,
        font_weight: FontWeight,
        line_height: f32,
        font_size: f32,
        overflow_wrap: OverflowWrap,
        alignment: Alignment,
        cursor_fill: Brush,
        highlight_fill: Brush,
        wrap: bool,
    ) {
        if self.app_context.editor.is_some() {
            return;
        }
        let Some(area) = self.app_context.editor_areas.get(&id) else {
            return;
        };
        let mut editor = PlainEditor::new(font_size);
        editor.set_text(&text);
        let styles = editor.edit_styles();

        styles.insert(parley::StyleProperty::Brush(fill));
        styles.insert(parley::FontFamily::Named(font_family.into()).into());
        styles.insert(StyleProperty::FontWeight(font_weight));
        styles.insert(StyleProperty::LineHeight(LineHeight::FontSizeRelative(
            line_height,
        )));
        styles.insert(StyleProperty::FontSize(font_size));
        styles.insert(StyleProperty::OverflowWrap(overflow_wrap));

        editor.set_alignment(alignment);
        if wrap {
            editor.set_width(Some(area.width));
        }
        let mut editor = Editor {
            editor,
            last_click_time: Default::default(),
            click_count: Default::default(),
            pointer_down: Default::default(),
            cursor_pos: Default::default(),
            cursor_visible: Default::default(),
            modifiers: Default::default(),
            start_time: Default::default(),
            blink_period: Default::default(),
        };

        if let Some(pos) = self.cursor_position {
            editor.mouse_moved(
                Point::new(pos.x - area.x as f64, pos.y - area.y as f64),
                &mut self.app_context.layout_cx,
                &mut self.app_context.font_cx,
            );
        }
        self.app_context.editor = Some(EditState {
            id,
            editor,
            editing: true,
            cursor_color: cursor_fill,
            highlight_color: highlight_fill,
        });
    }

    pub fn spawn(&self, task: impl std::future::Future<Output = ()> + Send + 'static) {
        self.task_tracker.spawn_on(task, self.runtime.handle());
    }

    pub fn redraw_trigger(&self) -> RedrawTrigger {
        RedrawTrigger::new(self.redraw.clone())
    }

    pub fn redraw(&self) {
        let _ = self.redraw.blocking_send(());
    }

    pub fn callback<State: 'static, T: Send + 'static>(
        &self,
        f: impl Fn(&mut State, T) + Send + Sync + 'static,
    ) -> Callback<T> {
        let f = Arc::new(f);
        Callback {
            event_proxy: self.event_proxy.clone(),
            handler: Arc::new(move |val| {
                let f = f.clone();
                Box::new(move |any: &mut dyn std::any::Any| {
                    if let Some(state) = any.downcast_mut::<State>() {
                        f(state, val);
                    }
                })
            }),
        }
    }
}

#[derive(Clone)]
pub struct Callback<T> {
    event_proxy: winit::event_loop::EventLoopProxy<AppEvent>,
    handler: Arc<dyn Fn(T) -> Box<dyn FnOnce(&mut dyn std::any::Any) + Send> + Send + Sync>,
}

impl<T> Callback<T> {
    pub fn send(&self, value: T) {
        let cb = (self.handler)(value);
        let _ = self.event_proxy.send_event(AppEvent::Callback(cb));
    }
}

#[derive(Debug, Clone)]
pub struct RedrawTrigger {
    sender: Sender<()>,
}

impl RedrawTrigger {
    pub(crate) fn new(sender: Sender<()>) -> Self {
        Self { sender }
    }

    pub async fn trigger(&self) {
        self.sender.send(()).await.ok();
    }
}

impl<State: 'static> App<'_, State> {
    pub fn start(state: State, window: Window<State>) {
        AppBuilder::new(state, window).start();
    }

    pub fn builder(state: State, window: Window<State>) -> AppBuilder<State> {
        AppBuilder::new(state, window)
    }

    fn request_redraw(&self) {
        for ws in self.windows.values() {
            ws.window.request_redraw();
        }
    }

    fn request_redraw_window(&self, window_id: winit::window::WindowId) {
        if let Some(ws) = self.windows.get(&window_id) {
            ws.window.request_redraw();
        }
    }

    fn gesture_handlers(
        &self,
        window_id: winit::window::WindowId,
    ) -> Vec<(u64, Area, GestureHandler<State, AppState>)> {
        self.windows
            .get(&window_id)
            .map(|ws| ws.gesture_handlers.clone())
            .unwrap_or_default()
    }

    fn remove_window(&mut self, id: WindowId) {
        self.windows.remove(&id);
        if !self.windows.is_empty() {
            self.request_redraw();
        }
    }
    fn create_window(&mut self, event_loop: &ActiveEventLoop, name: &'static str) {
        if let Some(ws) = self.windows.values().find(|ws| ws.name == name) {
            ws.window.focus_window();
            return;
        }
        let Some(config) = self.window_registry.get(name) else {
            eprintln!("No registered window named '{name}'");
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
        let mut attributes = WinitWindow::default_attributes();

        if let Some(ref title) = config.title {
            attributes = attributes.with_title(title.clone());
        }

        let window = Arc::new(event_loop.create_window(attributes).unwrap());
        let size = window.inner_size();
        let surface_future = self.context.create_surface(
            window.clone(),
            size.width,
            size.height,
            vello_svg::vello::wgpu::PresentMode::AutoNoVsync,
        );
        let mut surface = pollster::block_on(surface_future).expect("Error creating surface");

        if transparent {
            let device = &self.context.devices[surface.dev_id].device;
            let capabilities = surface
                .surface
                .get_capabilities(self.context.devices[surface.dev_id].adapter());
            if capabilities
                .alpha_modes
                .contains(&wgpu::CompositeAlphaMode::PostMultiplied)
            {
                surface.config.alpha_mode = wgpu::CompositeAlphaMode::PostMultiplied;
            }
            surface.surface.configure(device, &surface.config);
        }

        let dev_id = surface.dev_id;
        let devices_len = self.context.devices.len();
        self.renderers.resize_with(devices_len, || None);
        self.renderers[dev_id].get_or_insert_with(|| {
            Renderer::new(
                &self.context.devices[dev_id].device,
                RendererOptions::default(),
            )
            .expect("Failed to create renderer")
        });

        let window_id = window.id();

        #[cfg(target_os = "windows")]
        window.set_visible(true);

        self.windows.insert(
            window_id,
            WindowState {
                surface,
                window,
                scene: Scene::new(),
                name,
                view: config.view,
                gesture_handlers: Vec::new(),
                cursor_position: None,
                gesture_state: GestureState::None,
                last_window_size: None,
                fullscreen_requested: false,
            },
        );
    }

    fn run(
        state: State,
        event_loop: EventLoop<AppEvent>,
        render_cx: RenderContext,
        #[cfg(target_arch = "wasm32")] render_state: RenderState,
        window_registry: HashMap<&'static str, Window<State>>,
        initial_windows: Vec<&'static str>,
        on_frame: fn(&mut State, &mut AppState) -> (),
        on_start: fn(&mut State, &mut AppState) -> (),
        on_exit: fn(&mut State, &mut AppState) -> (),
        custom_fonts: Vec<FontEntry>,
    ) {
        #[allow(unused_mut)]
        let mut renderers: Vec<Option<Renderer>> = vec![];

        let mut font_cx = FontContext::new();

        font_cx
            .collection
            .register_fonts(Blob::new(Arc::new(RUBIK_FONT)), None);

        for (font_bytes, family_opt) in custom_fonts.into_iter() {
            font_cx.collection.register_fonts(
                Blob::new(font_bytes),
                Some(FontInfoOverride {
                    family_name: family_opt.as_deref(),
                    ..Default::default()
                }),
            );
        }

        let runtime = Runtime::new().expect("Failed to create runtime");

        let redraw_proxy = event_loop.create_proxy();
        let event_proxy = event_loop.create_proxy();
        let (redraw_sender, mut redraw_receiver) = tokio::sync::mpsc::channel::<()>(10);
        runtime.spawn(async move {
            loop {
                if redraw_receiver.recv().await.is_some() {
                    redraw_proxy
                        .send_event(AppEvent::RequestRedraw)
                        .expect("Event send failed");
                }
            }
        });

        let layout_cache = HashMap::new();
        let layout_cx = LayoutContext::new();
        let font_cx_inner = FontContext::new();

        let mut app = Self {
            context: render_cx,
            renderers,
            windows: HashMap::new(),
            window_registry,
            initial_windows,
            state,
            app_state: AppState {
                runtime,
                cancellation_token: CancellationToken::new(),
                task_tracker: TaskTracker::new(),
                app_context: AppCtx {
                    text_layout: TextLayout::new(layout_cache, font_cx_inner, layout_cx),
                    font_cx: FontContext::new(),
                    layout_cx: LayoutContext::new(),
                    scale_factor: 1.,
                    editor: None,
                    editor_areas: HashMap::new(),
                    scrollers: HashMap::new(),
                },
                layout_cache: HashMap::new(),
                image_scenes: HashMap::new(),
                svg_scenes: HashMap::new(),
                modifiers: None,
                redraw: redraw_sender,
                event_proxy,
                cursor_position: None,
            },
            on_frame,
            on_start,
            on_exit,
            started: false,
        };

        event_loop.run_app(&mut app).expect("run to completion");
        (app.on_exit)(&mut app.state, &mut app.app_state);

        app.app_state.cancellation_token.cancel();

        app.app_state.task_tracker.close();

        app.app_state.runtime.block_on(async {
            tokio::time::timeout(Duration::from_secs(1), app.app_state.task_tracker.wait())
                .await
                .ok();
        });
    }

    fn redraw(&mut self, window_id: winit::window::WindowId) {
        if !self.started {
            self.started = true;
            (self.on_start)(&mut self.state, &mut self.app_state);
        }

        let Some(ws) = self.windows.get_mut(&window_id) else {
            return;
        };

        ws.gesture_handlers.clear();
        let size = ws.window.inner_size();
        ws.last_window_size = Some(size);
        self.app_state.app_context.scale_factor = ws.window.scale_factor();
        let width = size.width;
        let height = size.height;
        if ws.surface.config.width != width || ws.surface.config.height != height {
            self.context.resize_surface(&mut ws.surface, width, height);
        }

        let view = ws.view;
        let draw_items = {
            let mut layout = view(&self.state, &mut self.app_state);
            layout.draw(
                Area {
                    x: 0.,
                    y: 0.,
                    width: ((width as f64) / self.app_state.app_context.scale_factor) as f32,
                    height: ((height as f64) / self.app_state.app_context.scale_factor) as f32,
                },
                &mut self.app_state.app_context,
            )
        };

        let ws = self.windows.get_mut(&window_id).unwrap();
        for item in draw_items {
            match item {
                View::PushClip { path } => {
                    ws.scene.push_layer(
                        Fill::NonZero,
                        Mix::Normal,
                        1.,
                        Affine::scale(self.app_state.app_context.scale_factor),
                        &path,
                    );
                }
                View::PopClip => {
                    ws.scene.pop_layer();
                }
                View::EditorArea(id, area) => {
                    self.app_state.app_context.editor_areas.insert(id, area);
                }
                View::Draw {
                    mut view,
                    gesture_handlers,
                    area,
                } => {
                    let id = view.id();
                    let draw_area = area;

                    ws.gesture_handlers.extend(
                        gesture_handlers
                            .into_iter()
                            .map(|handler| (id, draw_area, handler)),
                    );

                    match &mut *view {
                        DrawableType::Text(v) => {
                            v.draw(draw_area, area, &mut ws.scene, &mut self.app_state)
                        }
                        DrawableType::Layout(boxed) => {
                            let (layout, transform) = boxed.as_mut();
                            draw_layout(None, *transform, layout, &mut ws.scene)
                        }
                        DrawableType::Path(v) => v.draw(
                            &mut ws.scene,
                            draw_area,
                            self.app_state.app_context.scale_factor,
                        ),
                        DrawableType::Svg(v) => {
                            v.draw(draw_area, &mut ws.scene, &mut self.app_state)
                        }
                        DrawableType::Image(v) => {
                            v.draw(draw_area, &mut ws.scene, &mut self.app_state)
                        }
                    }
                }
                View::Empty => (),
            }
        }

        (self.on_frame)(&mut self.state, &mut self.app_state);

        let ws = self.windows.get_mut(&window_id).unwrap();
        let size = ws.window.inner_size();
        let width = size.width;
        let height = size.height;

        let device_handle = &self.context.devices[ws.surface.dev_id];

        let render_params = vello_svg::vello::RenderParams {
            base_color: Color::TRANSPARENT,
            width,
            height,
            antialiasing_method: vello_svg::vello::AaConfig::Msaa8,
        };

        ws.window.pre_present_notify();

        self.renderers[ws.surface.dev_id]
            .as_mut()
            .unwrap()
            .render_to_texture(
                &device_handle.device,
                &device_handle.queue,
                &ws.scene,
                &ws.surface.target_view,
                &render_params,
            )
            .expect("failed to render to texture");

        let surface_texture = ws
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
        ws.surface.blitter.copy(
            &device_handle.device,
            &mut encoder,
            &ws.surface.target_view,
            &surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );
        device_handle.queue.submit([encoder.finish()]);
        surface_texture.present();

        ws.scene.reset();
    }
}

pub(crate) enum AppEvent {
    RequestRedraw,
    Callback(Box<dyn FnOnce(&mut dyn std::any::Any) + Send>),
    OpenWindow(&'static str),
    CloseWindow(WindowId),
}

impl std::fmt::Debug for AppEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppEvent::RequestRedraw => write!(f, "RequestRedraw"),
            AppEvent::Callback(_) => write!(f, "Callback"),
            AppEvent::OpenWindow(name) => write!(f, "OpenWindow({name})"),
            AppEvent::CloseWindow(id) => write!(f, "CloseWindow({id:?})"),
        }
    }
}

impl<State: 'static> ApplicationHandler<AppEvent> for App<'_, State> {
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::RequestRedraw => {
                self.request_redraw();
            }
            AppEvent::Callback(cb) => {
                cb(&mut self.state);
                self.request_redraw();
            }
            AppEvent::OpenWindow(name) => {
                self.create_window(event_loop, name);
            }
            AppEvent::CloseWindow(id) => {
                self.remove_window(id);
            }
        }
    }

    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        for window in self.initial_windows.clone() {
            self.create_window(event_loop, window);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if let Some(event) = crate::event::WindowEvent::from_winit_window_event(event) {
            match event {
                event::WindowEvent::Moved(_) => {}
                event::WindowEvent::KeyPressed(key) => {
                    let Some(key) = crate::Key::from(key) else {
                        return;
                    };
                    let mut needs_redraw = false;
                    for (_id, _area, handler) in self.gesture_handlers(window_id) {
                        if let Some(ref interaction_handler) = handler.interaction_handler
                            && handler.interaction_type.key
                        {
                            needs_redraw = true;
                            interaction_handler(
                                &mut self.state,
                                &mut self.app_state,
                                Interaction::Key(key.clone()),
                            );
                        }
                    }
                    if needs_redraw {
                        self.request_redraw_window(window_id);
                    }
                }
                event::WindowEvent::KeyReleased(_) => {}
                event::WindowEvent::MouseMoved(pos) => self.mouse_moved(window_id, pos),
                event::WindowEvent::MousePressed(MouseButton::Left) => {
                    self.mouse_pressed(window_id)
                }
                event::WindowEvent::MouseReleased(MouseButton::Left) => {
                    self.mouse_released(window_id)
                }
                event::WindowEvent::MousePressed(_) => {}
                event::WindowEvent::MouseReleased(_) => {}
                event::WindowEvent::MouseEntered => {}
                event::WindowEvent::MouseExited => {
                    if let Some(ws) = self.windows.get_mut(&window_id) {
                        ws.cursor_position = None;
                    }
                    let mut needs_redraw = false;
                    for (_, _, gh) in self.gesture_handlers(window_id) {
                        if gh.interaction_type.hover
                            && let Some(ref on_hover) = gh.interaction_handler
                        {
                            needs_redraw = true;
                            on_hover(
                                &mut self.state,
                                &mut self.app_state,
                                Interaction::Hover(false),
                            );
                        }
                    }
                    if needs_redraw {
                        self.request_redraw_window(window_id);
                    }
                }
                event::WindowEvent::MouseWheel(delta, _phase) => {
                    self.scrolled(window_id, delta);
                }
                event::WindowEvent::Resized(_) => {}
                event::WindowEvent::HoveredFile(_) => {}
                event::WindowEvent::DroppedFile(_) => {}
                event::WindowEvent::HoveredFileCancelled => {}
                event::WindowEvent::Touch(_) => {}
                event::WindowEvent::TouchPressure(_) => {}
                event::WindowEvent::Focused => {
                    self.request_redraw_window(window_id);
                }
                event::WindowEvent::Unfocused => {}
                event::WindowEvent::Closed => {
                    self.windows.remove(&window_id);
                    if self.windows.is_empty() {
                        event_loop.exit();
                    }
                }
                event::WindowEvent::RedrawRequested => self.redraw(window_id),
                event::WindowEvent::ScaleFactorChanged(scale_factor) => {
                    self.app_state.app_context.scale_factor = scale_factor;
                    self.app_state.layout_cache.clear();
                    self.request_redraw();
                }
                event::WindowEvent::ModifiersChanged(modifiers) => {
                    self.app_state.modifiers = Some(modifiers);
                }
            }
        }
    }
}
impl<State: 'static> App<'_, State> {
    pub(crate) fn mouse_moved(&mut self, window_id: winit::window::WindowId, pos: Point) {
        let pos = Point::new(
            pos.x / self.app_state.app_context.scale_factor,
            pos.y / self.app_state.app_context.scale_factor,
        );
        let mut needs_redraw = false;
        if let Some(ws) = self.windows.get_mut(&window_id) {
            ws.cursor_position = Some(pos);
        }
        self.app_state.cursor_position = Some(pos);
        if let Some(EditState { id, editor, .. }) = self.app_state.app_context.editor.as_mut()
            && let Some(area) = self.app_state.app_context.editor_areas.get(id).copied()
        {
            needs_redraw = true;
            editor.mouse_moved(
                Point::new(pos.x - area.x as f64, pos.y - area.y as f64),
                &mut self.app_state.app_context.layout_cx,
                &mut self.app_state.app_context.font_cx,
            );
        }
        self.gesture_handlers(window_id)
            .iter()
            .for_each(|(_, area, gh)| {
                if gh.interaction_type.hover
                    && let Some(ref on_hover) = gh.interaction_handler
                {
                    needs_redraw = true;
                    (on_hover)(
                        &mut self.state,
                        &mut self.app_state,
                        Interaction::Hover(area_contains(area, pos)),
                    );
                }
            });
        let gesture_state = self
            .windows
            .get(&window_id)
            .map(|ws| ws.gesture_state)
            .unwrap_or(GestureState::None);
        if let GestureState::Dragging {
            start,
            last_position,
            capturer,
        } = gesture_state
        {
            let distance = start.distance(pos);
            let delta = Point {
                x: pos.x - last_position.x,
                y: pos.y - last_position.y,
            };
            self.gesture_handlers(window_id)
                .iter()
                .filter(|(id, _, gh)| *id == capturer && gh.interaction_type.drag)
                .for_each(|(_, area, gh)| {
                    needs_redraw = true;
                    if let Some(handler) = &gh.interaction_handler {
                        (handler)(
                            &mut self.state,
                            &mut self.app_state,
                            Interaction::Drag(DragState::Updated {
                                start: Point {
                                    x: start.x - area.x as f64,
                                    y: start.y - area.y as f64,
                                },
                                current: Point {
                                    x: pos.x - area.x as f64,
                                    y: pos.y - area.y as f64,
                                },
                                start_global: start,
                                current_global: pos,
                                delta,
                                distance: distance as f32,
                            }),
                        );
                    }
                });
            if let Some(ws) = self.windows.get_mut(&window_id) {
                ws.gesture_state = GestureState::Dragging {
                    start,
                    last_position: pos,
                    capturer,
                };
            }
        }
        if needs_redraw {
            self.request_redraw_window(window_id);
        }
    }
    pub(crate) fn mouse_pressed(&mut self, window_id: winit::window::WindowId) {
        let mut needs_redraw = false;
        let cursor_position = self
            .windows
            .get(&window_id)
            .and_then(|ws| ws.cursor_position);
        if let Some(point) = cursor_position {
            for (_, area, handler) in
                self.gesture_handlers(window_id)
                    .iter()
                    .rev()
                    .filter(|(_, area, handler)| {
                        handler.interaction_type.click_outside
                            && !area_contains_padded(area, point, 10.)
                    })
            {
                if handler.interaction_type.click_outside
                    && let Some(ref on_click_outside) = handler.interaction_handler
                {
                    on_click_outside(
                        &mut self.state,
                        &mut self.app_state,
                        Interaction::ClickOutside(
                            ClickState::Started,
                            ClickLocation::new(point, *area),
                        ),
                    );
                }
            }
            let handlers = self.gesture_handlers(window_id);
            if let Some((capturer, area, handler)) = handlers
                .iter()
                .rev()
                .find(|(_, area, handler)| {
                    area_contains(area, point)
                        && (handler.interaction_type.click || handler.interaction_type.drag)
                })
                .or(handlers.iter().rev().find(|(_, area, handler)| {
                    area_contains(
                        &Area {
                            x: area.x - 10.,
                            y: area.y - 10.,
                            width: area.width + 20.,
                            height: area.height + 20.,
                        },
                        point,
                    ) && (handler.interaction_type.click || handler.interaction_type.drag)
                }))
            {
                needs_redraw = true;
                if handler.interaction_type.click
                    && let Some(ref on_click) = handler.interaction_handler
                {
                    on_click(
                        &mut self.state,
                        &mut self.app_state,
                        Interaction::Click(ClickState::Started, ClickLocation::new(point, *area)),
                    );
                } else if handler.interaction_type.drag
                    && let Some(ref on_drag) = handler.interaction_handler
                {
                    on_drag(
                        &mut self.state,
                        &mut self.app_state,
                        Interaction::Drag(DragState::Began {
                            start: Point {
                                x: point.x - area.x as f64,
                                y: point.y - area.y as f64,
                            },
                            start_global: point,
                        }),
                    );
                }
                if let Some(ws) = self.windows.get_mut(&window_id) {
                    ws.gesture_state = GestureState::Dragging {
                        start: point,
                        last_position: point,
                        capturer: *capturer,
                    };
                }
            }
            if let Some(EditState { id, editor, .. }) = self.app_state.app_context.editor.as_mut()
                && let Some(area) = self.app_state.app_context.editor_areas.get(id).cloned()
                && area_contains_padded(&area, point, 10.)
            {
                editor.mouse_pressed(
                    &mut self.app_state.app_context.layout_cx,
                    &mut self.app_state.app_context.font_cx,
                );
            }
        }

        if needs_redraw {
            self.request_redraw_window(window_id);
        }
    }
    pub(crate) fn mouse_released(&mut self, window_id: winit::window::WindowId) {
        let mut needs_redraw = false;
        let cursor_position = self
            .windows
            .get(&window_id)
            .and_then(|ws| ws.cursor_position);
        let gesture_state = self
            .windows
            .get(&window_id)
            .map(|ws| ws.gesture_state)
            .unwrap_or(GestureState::None);
        if let Some(current) = cursor_position {
            if let Some(EditState { id, editor, .. }) = self.app_state.app_context.editor.as_mut()
                && let Some(area) = self.app_state.app_context.editor_areas.get(id)
            {
                editor.mouse_released();
                needs_redraw = true;
                if !area_contains(area, current)
                    && (!matches!(gesture_state, GestureState::Dragging { .. })
                        || match gesture_state {
                            GestureState::Dragging { capturer, .. } => capturer != *id,
                            _ => false,
                        })
                {
                    self.app_state.end_editing();
                }
            }
            if let GestureState::Dragging {
                start,
                last_position,
                capturer,
            } = gesture_state
            {
                let distance = start.distance(current);
                let delta = Point {
                    x: current.x - last_position.x,
                    y: current.y - last_position.y,
                };
                self.gesture_handlers(window_id)
                    .iter()
                    .filter(|(id, _, _)| *id == capturer)
                    .for_each(|(_, area, gh)| {
                        if let (Some(on_click), true) =
                            (&gh.interaction_handler, gh.interaction_type.click)
                        {
                            needs_redraw = true;
                            if area_contains(area, current) {
                                on_click(
                                    &mut self.state,
                                    &mut self.app_state,
                                    Interaction::Click(
                                        ClickState::Completed,
                                        ClickLocation::new(current, *area),
                                    ),
                                );
                            } else {
                                on_click(
                                    &mut self.state,
                                    &mut self.app_state,
                                    Interaction::Click(
                                        ClickState::Cancelled,
                                        ClickLocation::new(current, *area),
                                    ),
                                );
                            }
                        }
                        if let (Some(on_drag), true) =
                            (&gh.interaction_handler, gh.interaction_type.drag)
                        {
                            needs_redraw = true;
                            on_drag(
                                &mut self.state,
                                &mut self.app_state,
                                Interaction::Drag(DragState::Completed {
                                    start: Point {
                                        x: start.x - area.x as f64,
                                        y: start.y - area.y as f64,
                                    },
                                    current: Point {
                                        x: current.x - area.x as f64,
                                        y: current.y - area.y as f64,
                                    },
                                    start_global: start,
                                    current_global: current,
                                    delta,
                                    distance: distance as f32,
                                }),
                            );
                        }
                    });
            }
            let press_start = match gesture_state {
                GestureState::Dragging { start, .. } => Some(start),
                _ => None,
            };
            for (_, area, handler) in self
                .gesture_handlers(window_id)
                .iter()
                .filter(|(_, _, h)| h.interaction_type.click_outside)
            {
                if !area_contains_padded(area, current, 10.)
                    && press_start.is_some_and(|s| !area_contains_padded(area, s, 10.))
                    && let Some(ref handler) = handler.interaction_handler
                {
                    needs_redraw = true;
                    handler(
                        &mut self.state,
                        &mut self.app_state,
                        Interaction::ClickOutside(
                            ClickState::Completed,
                            ClickLocation::new(current, *area),
                        ),
                    );
                }
            }
        }
        if let Some(ws) = self.windows.get_mut(&window_id) {
            ws.gesture_state = GestureState::None;
        }
        if needs_redraw {
            self.request_redraw_window(window_id);
        }

        if let Some(ws) = self.windows.get(&window_id) {
            if ws.fullscreen_requested
                && ws.window.fullscreen() != Some(Fullscreen::Borderless(None))
            {
                ws.window
                    .set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
            } else if ws.window.fullscreen().is_some() {
                ws.window.set_fullscreen(None);
            }
        }
    }

    pub(crate) fn scrolled(&mut self, window_id: winit::window::WindowId, delta: MouseScrollDelta) {
        let mut needs_redraw = false;
        let cursor_position = self
            .windows
            .get(&window_id)
            .and_then(|ws| ws.cursor_position);
        if let Some(current) = cursor_position
            && let Some((_, _, handler)) =
                self.gesture_handlers(window_id)
                    .iter()
                    .rev()
                    .find(|(_, area, handler)| {
                        area_contains(area, current) && (handler.interaction_type.scroll)
                    })
            && let Some(ref on_scroll) = handler.interaction_handler
        {
            needs_redraw = true;
            on_scroll(
                &mut self.state,
                &mut self.app_state,
                Interaction::Scroll(match delta {
                    MouseScrollDelta::LineDelta(x, y) => ScrollDelta {
                        x: x * 10.,
                        y: y * 10.,
                    },
                    MouseScrollDelta::PixelDelta(physical_position) => ScrollDelta {
                        x: physical_position.x as f32,
                        y: physical_position.y as f32,
                    },
                }),
            );
        }
        if needs_redraw {
            self.request_redraw_window(window_id);
        }
    }
}
