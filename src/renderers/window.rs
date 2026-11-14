use crate::render::{Frame, TextureRenderer};

pub struct WindowRenderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    config: wgpu::SurfaceConfiguration,
    renderer: TextureRenderer,
}

impl WindowRenderer {
    pub fn new(window: impl Into<wgpu::SurfaceTarget<'static>>, width: u32, height: u32) -> Self {
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window).expect("window surface");
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .expect("graphics adapter");
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("graphics device");
        let formats = surface.get_capabilities(&adapter).formats;
        let format = [
            wgpu::TextureFormat::Bgra8Unorm,
            wgpu::TextureFormat::Rgba8Unorm,
        ]
        .into_iter()
        .find(|format| formats.contains(format))
        .expect("supported surface format");
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: width.max(1),
            height: height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: Vec::new(),
        };
        surface.configure(&device, &config);
        Self {
            renderer: TextureRenderer::new(&device, &queue, format),
            surface,
            device,
            config,
        }
    }

    pub fn render(&mut self, frame: &Frame) {
        if frame.width == 0 || frame.height == 0 {
            return;
        }
        if (self.config.width, self.config.height) != (frame.width, frame.height) {
            self.config.width = frame.width;
            self.config.height = frame.height;
            self.surface.configure(&self.device, &self.config);
        }
        let texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture)
            | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            _ => return,
        };
        self.renderer.render(
            frame,
            &texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );
        texture.present();
    }
}
