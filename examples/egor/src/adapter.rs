use anyrender_vello_hybrid::{ImageManager, VelloHybridScenePainter};
use egor::app::{Ui, WindowEvent};
use egor_render::Renderer;
use haven::render::FramePainter;
use haven::{Key, Modifier, Modifiers, MouseButton, NamedKey, Pane, PaneEffect, Point};
use rustc_hash::FxHashMap;
use winit::window::Window;
use winit::{
    event::{ElementState, MouseButton as WinitMouseButton},
    keyboard::{Key as WinitKey, NamedKey as WinitNamedKey},
};

pub(super) struct Haven<State> {
    pub(super) model: State,
    pane: Pane<State>,
    gpu: wgpu_context::DeviceHandle,
    painter: FramePainter,
    image_atlas: FxHashMap<u64, vello_common::paint::ImageId>,
    vello: vello_hybrid::Renderer,
    resources: vello_hybrid::Resources,
    scene: vello_hybrid::Scene,
    overlay: wgpu::TextureView,
    blitter: wgpu::util::TextureBlitter,
}

impl<State: 'static> Haven<State> {
    pub(super) fn new(
        model: State,
        pane: Pane<State>,
        renderer: &Renderer,
        window: &Window,
        format: wgpu::TextureFormat,
    ) -> Self {
        let device = renderer.device();
        let queue = renderer.queue();
        let size = window.inner_size();
        let (width, height) = (size.width.max(1), size.height.max(1));
        Self {
            model,
            pane,
            gpu: wgpu_context::DeviceHandle {
                instance: renderer.instance().clone(),
                adapter: renderer.adapter().clone(),
                device: device.clone(),
                queue: queue.clone(),
            },
            painter: FramePainter::default(),
            image_atlas: FxHashMap::default(),
            vello: vello_hybrid::Renderer::new(
                device,
                &vello_hybrid::RenderTargetConfig {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    width,
                    height,
                },
            ),
            resources: vello_hybrid::Resources::new(),
            scene: vello_hybrid::Scene::new(
                width.try_into().expect("Haven width"),
                height.try_into().expect("Haven height"),
            ),
            overlay: overlay_texture(device, width, height),
            blitter: wgpu::util::TextureBlitterBuilder::new(device, format.remove_srgb_suffix())
                .blend_state(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING)
                .build(),
        }
    }
}

impl<State: 'static> Ui for Haven<State> {
    fn render(&mut self, _: &Renderer, window: &Window, target: &mut egor_render::frame::Frame) {
        let (frame, effects) = self.pane.redraw(
            &mut self.model,
            target.view.texture().width(),
            target.view.texture().height(),
            window.scale_factor(),
        );
        apply_pane_effects(window, effects);
        let encoder = &mut target.encoder;
        if self.overlay.texture().width() != frame.width
            || self.overlay.texture().height() != frame.height
        {
            self.overlay = overlay_texture(&self.gpu.device, frame.width, frame.height);
        }
        self.scene.reset_and_resize(
            frame.width.try_into().expect("Haven width"),
            frame.height.try_into().expect("Haven height"),
        );
        let mut texture_bindings = FxHashMap::default();
        {
            let images = ImageManager::new(
                &mut self.vello,
                &mut self.resources,
                &self.gpu.device,
                &self.gpu.queue,
                encoder,
                &mut self.image_atlas,
            );
            let mut scene = VelloHybridScenePainter::new(
                &mut self.scene,
                images,
                &mut texture_bindings,
                &self.gpu,
            );
            self.painter.paint(&frame, &mut scene);
        }
        assert!(texture_bindings.is_empty());
        self.vello
            .render(
                &self.scene,
                &mut self.resources,
                &self.gpu.device,
                &self.gpu.queue,
                encoder,
                &vello_hybrid::RenderSize {
                    width: frame.width,
                    height: frame.height,
                },
                &self.overlay,
                &vello_hybrid::TextureBindings::new(),
            )
            .expect("paint Haven frame");
        let target = target
            .view
            .texture()
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(target.view.texture().format().remove_srgb_suffix()),
                ..Default::default()
            });
        self.blitter
            .copy(&self.gpu.device, encoder, &self.overlay, &target);
    }

    fn handle_event(&mut self, window: &Window, event: &WindowEvent) {
        let pane = &mut self.pane;
        let model = &mut self.model;
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
                        _ => return,
                    }),
                    _ => return,
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
                return;
            }
            _ => return,
        };
        window.request_redraw();
        apply_pane_effects(window, effects);
    }
}

fn overlay_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("Haven overlay"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&Default::default())
}

fn apply_pane_effects(window: &Window, effects: Vec<PaneEffect>) {
    for effect in effects {
        match effect {
            PaneEffect::Redraw => window.request_redraw(),
            PaneEffect::Open(_) | PaneEffect::Close => panic!("the example has one pane"),
        }
    }
}
