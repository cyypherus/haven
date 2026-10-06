use anyrender_vello_hybrid::{ImageManager, VelloHybridScenePainter};
use haven::render::{Frame, FramePainter};
use rustc_hash::FxHashMap;

pub(super) struct Painter {
    gpu: wgpu_context::DeviceHandle,
    painter: FramePainter,
    image_atlas: FxHashMap<u64, vello_common::paint::ImageId>,
    vello: vello_hybrid::Renderer,
    resources: vello_hybrid::Resources,
    scene: vello_hybrid::Scene,
    overlay: wgpu::TextureView,
    blitter: wgpu::util::TextureBlitter,
}

impl Painter {
    pub(super) fn new(
        instance: &wgpu::Instance,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Self {
        Self {
            gpu: wgpu_context::DeviceHandle {
                instance: instance.clone(),
                adapter: adapter.clone(),
                device: device.clone(),
                queue: queue.clone(),
            },
            painter: FramePainter::default(),
            image_atlas: FxHashMap::default(),
            vello: vello_hybrid::Renderer::new(
                device,
                &vello_hybrid::RenderTargetConfig {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    width: 1,
                    height: 1,
                },
            ),
            resources: vello_hybrid::Resources::new(),
            scene: vello_hybrid::Scene::new(1, 1),
            overlay: overlay_texture(device, 1, 1),
            blitter: wgpu::util::TextureBlitterBuilder::new(device, format.remove_srgb_suffix())
                .blend_state(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING)
                .build(),
        }
    }

    pub(super) fn paint(
        &mut self,
        frame: &Frame,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
    ) {
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
            self.painter.paint(frame, &mut scene);
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
        let target = target.texture().create_view(&wgpu::TextureViewDescriptor {
            format: Some(target.texture().format().remove_srgb_suffix()),
            ..Default::default()
        });
        self.blitter
            .copy(&self.gpu.device, encoder, &self.overlay, &target);
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
