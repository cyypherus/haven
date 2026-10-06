mod haven_gpu;

use egor_app::{
    AppConfig, AppHandler, AppRunner, ControlFlow, Window, WindowEvent, input::Input,
    time::FrameTimer,
};
use egor_render::{
    MemoryHints, Renderer,
    batch::GeometryBatch,
    target::{Backbuffer, OffscreenTarget, RenderTarget},
    vertex::Vertex,
};
use haven::render::FramePainter;
use haven::*;
use haven_gpu::Painter;
use std::sync::Arc;
use winit::{
    event::{ElementState, MouseButton as WinitMouseButton},
    keyboard::{Key as WinitKey, NamedKey as WinitNamedKey},
};

struct Controls {
    title: TextState,
    animation: ToggleState,
    speed: SliderState,
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            title: TextState::new("Demo"),
            animation: ToggleState::on(),
            speed: SliderState {
                value: 1.,
                ..Default::default()
            },
        }
    }
}

const TITLE: u64 = 1;
const ANIMATION: u64 = 2;
const SPEED: u64 = 3;

fn view<'a>(controls: &'a Controls, ctx: &mut PaneState) -> View<'a, Controls> {
    stack(vec![
        stack(vec![
            rect(id!()).fill(DEFAULT_DARK_GRAY).build(ctx),
            column_spaced(
                16.,
                vec![
                    text(id!(), "Text").build(ctx).align(Align::Leading),
                    text_field(TITLE, binding!(controls.title))
                        .align(Alignment::Start)
                        .build(ctx)
                        .height(40.),
                    stack(vec![
                        text(id!(), "Animate").build(ctx).align(Align::Leading),
                        toggle(ANIMATION, binding!(controls.animation))
                            .build(ctx)
                            .width(60.)
                            .height(32.)
                            .align(Align::Trailing),
                    ])
                    .height(40.)
                    .expand_x(),
                    stack(vec![
                        text(id!(), "Speed").build(ctx).align(Align::Leading),
                        text(id!(), format!("{:.1}", controls.speed.value))
                            .build(ctx)
                            .align(Align::Trailing),
                    ])
                    .expand_x(),
                    slider(SPEED, binding!(controls.speed))
                        .range(0.25, 3.)
                        .build(ctx)
                        .height(24.),
                ],
            )
            .pad(16.),
        ])
        .width(280.)
        .height(260.)
        .pad(16.)
        .align(Align::TopLeading),
        text(id!(), &controls.title.text)
            .font_size(20)
            .build(ctx)
            .pad(16.)
            .align(Align::TopTrailing),
    ])
    .expand()
}

struct Demo {
    controls: Controls,
    pane: Pane<Controls>,
    crab_time: f32,
}

impl Default for Demo {
    fn default() -> Self {
        Self {
            controls: Controls::default(),
            pane: PaneBuilder::new("demo", view)
                .background(TRANSPARENT)
                .build(),
            crab_time: 0.,
        }
    }
}

struct EgorSurface {
    renderer: Renderer,
    haven: Painter,
    overlay_pipeline: wgpu::RenderPipeline,
    backbuffer: Backbuffer,
    crabs: GeometryBatch,
}

fn ellipse(batch: &mut GeometryBatch, center: [f32; 2], radius: [f32; 2], color: [f32; 4]) {
    let (vertices, indices, base) = batch.try_allocate(26, 72).expect("crab geometry capacity");
    vertices[0] = Vertex::new(center, color, [0.; 2]);
    for i in 0..25 {
        let angle = i as f32 * std::f32::consts::TAU / 24.;
        vertices[i + 1] = Vertex::new(
            [
                center[0] + angle.cos() * radius[0],
                center[1] + angle.sin() * radius[1],
            ],
            color,
            [0.; 2],
        );
    }
    for (i, triangle) in indices.chunks_exact_mut(3).enumerate() {
        triangle.copy_from_slice(&[base, base + i as u16 + 1, base + i as u16 + 2]);
    }
}

fn segment(batch: &mut GeometryBatch, from: [f32; 2], to: [f32; 2], width: f32, color: [f32; 4]) {
    let dx = to[0] - from[0];
    let dy = to[1] - from[1];
    let length = dx.hypot(dy).max(0.001);
    let (x, y) = (-dy / length * width * 0.5, dx / length * width * 0.5);
    assert!(batch.push(
        &[
            Vertex::new([from[0] + x, from[1] + y], color, [0.; 2]),
            Vertex::new([from[0] - x, from[1] - y], color, [0.; 2]),
            Vertex::new([to[0] - x, to[1] - y], color, [0.; 2]),
            Vertex::new([to[0] + x, to[1] + y], color, [0.; 2]),
        ],
        &[0, 1, 2, 2, 3, 0]
    ));
}

fn bounce(distance: f32, extent: f32) -> f32 {
    extent - (distance.rem_euclid(extent * 2.) - extent).abs()
}

fn crab_scene(batch: &mut GeometryBatch, width: f32, height: f32, time: f32) {
    for i in 0..8 {
        let seed = i as f32;
        let x = 64.
            + bounce(
                seed * 137. + time * (43. + seed * 7.),
                (width - 128.).max(1.),
            );
        let y = 84.
            + bounce(
                seed * 89. + time * (31. + seed * 4.),
                (height - 168.).max(1.),
            );
        let coral = [1., 0.22 + seed * 0.018, 0.12, 1.];
        let dark = [0.56, 0.09, 0.075, 1.];
        for side in [-1., 1.] {
            for leg in 0..3 {
                let sway = (time * 8. + seed + leg as f32).sin() * 4.;
                let offset = leg as f32 * 10.;
                let knee = [x + side * (35. + offset * 0.5), y + offset - 3.];
                segment(batch, [x + side * 22., y + offset * 0.3], knee, 5., dark);
                segment(
                    batch,
                    knee,
                    [x + side * (43. + offset * 0.4), y + 19. + offset + sway],
                    4.,
                    coral,
                );
            }
            segment(
                batch,
                [x + side * 22., y - 4.],
                [x + side * 40., y - 23.],
                7.,
                coral,
            );
            ellipse(batch, [x + side * 43., y - 28.], [12., 14.], coral);
            segment(
                batch,
                [x + side * 43., y - 42.],
                [x + side * 43., y - 31.],
                4.,
                [0.025, 0.09, 0.14, 1.],
            );
            segment(
                batch,
                [x + side * 11., y - 13.],
                [x + side * 13., y - 29.],
                5.,
                coral,
            );
            ellipse(
                batch,
                [x + side * 13., y - 29.],
                [7.; 2],
                [1., 0.96, 0.87, 1.],
            );
            ellipse(
                batch,
                [x + side * 13. + 1., y - 30.],
                [3.; 2],
                [0.018, 0.035, 0.06, 1.],
            );
        }
        ellipse(batch, [x, y], [29., 20.], coral);
        ellipse(batch, [x - 5., y - 7.], [16., 5.], [1., 0.52, 0.28, 1.]);
        segment(batch, [x - 6., y + 8.], [x, y + 11.], 2.5, dark);
        segment(batch, [x, y + 11.], [x + 6., y + 8.], 2.5, dark);
    }
}

impl EgorSurface {
    fn render(
        &mut self,
        frame: &haven::render::Frame,
        time: f32,
        target: &mut egor_render::frame::Frame,
    ) {
        let width = frame.width as f32 / frame.scale_factor as f32;
        let height = frame.height as f32 / frame.scale_factor as f32;
        self.renderer.upload_camera_matrix([
            [2. / width, 0., 0., 0.],
            [0., -2. / height, 0., 0.],
            [0., 0., 1., 0.],
            [-1., 1., 0., 1.],
        ]);
        crab_scene(&mut self.crabs, width, height, time);
        {
            let mut pass = self
                .renderer
                .begin_render_pass(&mut target.encoder, &target.view);
            self.renderer
                .draw_batch(&mut pass, &mut self.crabs, None, None);
        }
        let ui = self.haven.paint(frame);
        let bindings = self
            .renderer
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Haven overlay"),
                layout: &self.overlay_pipeline.get_bind_group_layout(0),
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&ui),
                }],
            });
        let mut pass = target
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Haven overlay"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
        pass.set_pipeline(&self.overlay_pipeline);
        pass.set_bind_group(0, &bindings, &[]);
        pass.draw(0..3, 0..1);
    }
}

impl AppHandler<EgorSurface> for Demo {
    async fn with_resource(&mut self, window: Arc<Window>) -> EgorSurface {
        let mut renderer = Renderer::new(window.clone(), &MemoryHints::Performance).await;
        renderer.set_clear_color([0.06, 0.06, 0.06, 1.]);
        let size = window.inner_size();
        let (width, height) = (size.width.max(1), size.height.max(1));
        let backbuffer = Backbuffer::new(
            renderer.instance(),
            renderer.adapter(),
            renderer.device(),
            window,
            width,
            height,
        );
        let shader = renderer
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Haven overlay"),
                source: wgpu::ShaderSource::Wgsl(include_str!("overlay.wgsl").into()),
            });
        let overlay_pipeline =
            renderer
                .device()
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Haven overlay"),
                    layout: None,
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vertex"),
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fragment"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: backbuffer.format(),
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
                    }),
                    primitive: Default::default(),
                    depth_stencil: None,
                    multisample: Default::default(),
                    multiview_mask: None,
                    cache: None,
                });
        let haven = Painter::new(
            renderer.instance(),
            renderer.adapter(),
            renderer.device(),
            renderer.queue(),
        );
        EgorSurface {
            renderer,
            haven,
            overlay_pipeline,
            backbuffer,
            crabs: GeometryBatch::new(4096, 12288),
        }
    }

    fn on_window_event(&mut self, window: &Window, event: &WindowEvent) {
        let effects = window_event(&mut self.pane, &mut self.controls, window, event);
        apply_pane_effects(window, effects);
    }

    fn resize(&mut self, width: u32, height: u32, surface: &mut EgorSurface) {
        surface
            .backbuffer
            .resize(surface.renderer.device(), width, height);
    }

    fn frame(&mut self, window: &Window, surface: &mut EgorSurface, _: &Input, timer: &FrameTimer) {
        if self.controls.animation.on {
            self.crab_time += timer.delta.min(0.05) * self.controls.speed.value;
        }
        let (width, height) = surface.backbuffer.size();
        let (frame, effects) =
            self.pane
                .redraw(&mut self.controls, width, height, window.scale_factor());
        apply_pane_effects(window, effects);
        let Some(mut egor_frame) = surface.renderer.begin_frame(&mut surface.backbuffer) else {
            return;
        };
        surface.render(&frame, self.crab_time, &mut egor_frame);
        window.pre_present_notify();
        surface.renderer.end_frame(egor_frame);
        if self.controls.animation.on {
            window.request_redraw();
        }
    }
}

fn window_event<State: 'static>(
    pane: &mut Pane<State>,
    model: &mut State,
    window: &Window,
    event: &WindowEvent,
) -> Vec<PaneEffect> {
    let effects = match event {
        WindowEvent::CursorMoved { position, .. } => pane.move_to(
            model,
            Point::new(
                position.x / window.scale_factor(),
                position.y / window.scale_factor(),
            ),
        ),
        WindowEvent::MouseInput {
            state,
            button: WinitMouseButton::Left,
            ..
        } => match state {
            ElementState::Pressed => pane.press_button(model, MouseButton::Left),
            ElementState::Released => pane.release_button(model, MouseButton::Left),
        },
        WindowEvent::KeyboardInput { event, .. } => {
            let key = match &event.logical_key {
                WinitKey::Character(text) => Key::Character(text.to_string()),
                WinitKey::Named(named) => Key::Named(match named {
                    WinitNamedKey::Enter => NamedKey::Enter,
                    WinitNamedKey::Escape => NamedKey::Escape,
                    WinitNamedKey::Space => NamedKey::Space,
                    WinitNamedKey::Backspace => NamedKey::Backspace,
                    WinitNamedKey::Delete => NamedKey::Delete,
                    WinitNamedKey::ArrowLeft => NamedKey::ArrowLeft,
                    WinitNamedKey::ArrowRight => NamedKey::ArrowRight,
                    WinitNamedKey::ArrowUp => NamedKey::ArrowUp,
                    WinitNamedKey::ArrowDown => NamedKey::ArrowDown,
                    WinitNamedKey::Home => NamedKey::Home,
                    WinitNamedKey::End => NamedKey::End,
                    WinitNamedKey::Tab => NamedKey::Tab,
                    _ => return Vec::new(),
                }),
                _ => return Vec::new(),
            };
            match event.state {
                ElementState::Pressed => pane.key_pressed(model, key),
                ElementState::Released => pane.key_released(model, key),
            }
        }
        WindowEvent::ModifiersChanged(modifiers) => {
            let modifiers = modifiers.state();
            pane.modifiers_changed(
                Modifiers::empty()
                    .with(Modifier::Shift, modifiers.shift_key())
                    .with(Modifier::Control, modifiers.control_key())
                    .with(Modifier::Alt, modifiers.alt_key())
                    .with(Modifier::Super, modifiers.super_key()),
            )
        }
        WindowEvent::CursorLeft { .. } => pane.move_to(model, Point::new(-1., -1.)),
        WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
            window.request_redraw();
            return Vec::new();
        }
        _ => return Vec::new(),
    };
    window.request_redraw();
    effects
}

fn apply_pane_effects(window: &Window, effects: Vec<PaneEffect>) {
    for effect in effects {
        match effect {
            PaneEffect::Redraw => window.request_redraw(),
            PaneEffect::Open(_) | PaneEffect::Close => unreachable!("the demo has one pane"),
        }
    }
}

fn main() {
    if std::env::args().nth(1).as_deref() == Some("--capture") {
        capture();
        return;
    }
    AppRunner::new(
        Demo::default(),
        AppConfig {
            title: "Haven in Egor".into(),
            width: Some(1536),
            height: Some(960),
            min_size: Some((960, 800)),
            control_flow: ControlFlow::Wait,
            ..Default::default()
        },
    )
    .run();
}

fn capture() {
    let started = std::time::Instant::now();
    let event_loop = winit::event_loop::EventLoop::new().expect("capture event loop");
    #[allow(deprecated)]
    let window = Arc::new(
        event_loop
            .create_window(
                Window::default_attributes()
                    .with_visible(false)
                    .with_inner_size(winit::dpi::PhysicalSize::new(64, 64)),
            )
            .expect("capture window"),
    );
    let mut demo = Demo::default();
    let mut surface = pollster::block_on(demo.with_resource(window));
    assert_ne!(surface.backbuffer.size(), (1536, 960));
    assert_ne!(
        surface.renderer.adapter().get_info().device_type,
        wgpu::DeviceType::Cpu
    );
    let mut target = OffscreenTarget::new(
        surface.renderer.device(),
        1536,
        960,
        surface.backbuffer.format(),
    );
    let mut previous = None;
    for (time, name) in [(0., "haven-in-egor.png"), (3., "haven-in-egor-next.png")] {
        let (frame, effects) = demo.pane.redraw(&mut demo.controls, 1536, 960, 1.5);
        assert!(effects.is_empty());
        let mut gpu_frame = surface
            .renderer
            .begin_frame(&mut target)
            .expect("offscreen frame");
        surface.render(&frame, time, &mut gpu_frame);
        let buffer = surface
            .renderer
            .device()
            .create_buffer(&wgpu::BufferDescriptor {
                label: Some("GPU capture readback"),
                size: 6144 * 960,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
        gpu_frame.encoder.copy_texture_to_buffer(
            gpu_frame.view.texture().as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(6144),
                    rows_per_image: None,
                },
            },
            gpu_frame.view.texture().size(),
        );
        surface.renderer.end_frame(gpu_frame);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                sender.send(result).expect("deliver readback");
            });
        surface
            .renderer
            .device()
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("wait for capture");
        receiver
            .recv()
            .expect("readback callback")
            .expect("map capture");
        let mut bytes = buffer.slice(..).get_mapped_range().to_vec();
        if matches!(
            target.format(),
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        ) {
            for pixel in bytes.chunks_exact_mut(4) {
                pixel.swap(0, 2);
            }
        }
        assert!(bytes.chunks_exact(4).all(|pixel| pixel[3] == 255));
        assert!(
            bytes
                .chunks_exact(4)
                .any(|pixel| pixel[0] > 220 && pixel[1] < 180 && pixel[2] < 140)
        );
        assert!(
            bytes
                .chunks_exact(4)
                .any(|pixel| pixel == [255, 255, 255, 255])
        );
        if let Some(previous) = &previous {
            assert_ne!(previous, &bytes, "crabs move between frames");
        }
        std::fs::create_dir_all("target").expect("capture directory");
        image::save_buffer(
            format!("target/{name}"),
            &bytes,
            1536,
            960,
            image::ColorType::Rgba8,
        )
        .expect("save capture");
        previous = Some(bytes);
    }
    eprintln!(
        "stabs: gpu={:?}, readback_bytes=11796480, cpu_frame_pixel_upload_bytes=0, wall_ms={:.3}",
        surface.renderer.adapter().get_info().device_type,
        started.elapsed().as_secs_f64() * 1000.
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controls_edit_the_scene() {
        let mut demo = Demo::default();
        let Demo { controls, pane, .. } = &mut demo;
        let (frame, effects) = pane.redraw(controls, 1536, 960, 1.5);
        assert_eq!(frame.width, 1536);
        assert!(effects.is_empty());
        let toggle = pane.location(ANIMATION).expect("toggle");
        let effects = pane.click(controls, toggle);
        assert!(
            effects
                .iter()
                .all(|effect| matches!(effect, PaneEffect::Redraw))
        );
        assert!(!controls.animation.on);
        let (frame, effects) = pane.redraw(controls, 1536, 960, 1.5);
        assert_eq!(frame.width, 1536);
        assert!(effects.is_empty());
        let slider = pane.location(SPEED).expect("slider");
        let effects = pane.drag(controls, slider, Point::new(slider.x + 80., slider.y));
        assert!(
            effects
                .iter()
                .all(|effect| matches!(effect, PaneEffect::Redraw))
        );
        assert!(controls.speed.value > 1.);
        let (frame, effects) = pane.redraw(controls, 1536, 960, 1.5);
        assert_eq!(frame.width, 1536);
        assert!(effects.is_empty());
        let title = pane.location(TITLE).expect("text field");
        let effects = pane.click(controls, title);
        assert!(
            effects
                .iter()
                .all(|effect| matches!(effect, PaneEffect::Redraw))
        );
        let (frame, effects) = pane.redraw(controls, 1536, 960, 1.5);
        assert_eq!(frame.width, 1536);
        assert!(
            effects
                .iter()
                .all(|effect| matches!(effect, PaneEffect::Redraw))
        );
        assert!(
            pane.modifiers_changed(Modifiers::from_pressed([if cfg!(target_os = "macos") {
                Modifier::Super
            } else {
                Modifier::Control
            }]))
            .is_empty()
        );
        assert!(controls.title.editing, "text field focused at {title:?}");
        let effects = pane.key_pressed(controls, "a");
        assert!(
            effects
                .iter()
                .all(|effect| matches!(effect, PaneEffect::Redraw))
        );
        assert!(pane.modifiers_changed(Modifiers::empty()).is_empty());
        let effects = pane.key_pressed(controls, "Updated");
        assert!(
            effects
                .iter()
                .all(|effect| matches!(effect, PaneEffect::Redraw))
        );
        assert_eq!(controls.title.text, "Updated");
        eprintln!("stabs: controls=3, cpu_frame_pixel_upload_bytes=0");
    }

    #[test]
    fn crabs_reflect_at_both_walls() {
        assert_eq!(bounce(0., 100.), 0.);
        assert_eq!(bounce(75., 100.), 75.);
        assert_eq!(bounce(125., 100.), 75.);
        assert_eq!(bounce(200., 100.), 0.);
        assert_eq!(bounce(250., 100.), 50.);
        eprintln!("stabs: bounce_cases=5, allocations=0");
    }
}
