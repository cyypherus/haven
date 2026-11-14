use super::Renderer;
use crate::render::Frame;

impl Renderer {
    pub(crate) fn for_window(
        window: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
    ) -> (Self, wgpu::Surface<'static>) {
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
        surface.configure(
            &device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                width: width.max(1),
                height: height.max(1),
                present_mode: wgpu::PresentMode::AutoVsync,
                desired_maximum_frame_latency: 2,
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                view_formats: Vec::new(),
            },
        );
        (
            Self::with_device(&device, &queue, format).expect("supported surface format"),
            surface,
        )
    }

    pub(crate) fn render_window(&mut self, frame: &Frame, surface: &wgpu::Surface<'_>) {
        if frame.width == 0 || frame.height == 0 {
            return;
        }
        let mut config = surface
            .get_configuration()
            .expect("configured window surface");
        if (config.width, config.height) != (frame.width, frame.height) {
            config.width = frame.width;
            config.height = frame.height;
            surface.configure(&self.device, &config);
        }
        let texture = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture)
            | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                surface.configure(&self.device, &config);
                return;
            }
            _ => return,
        };
        self.render_texture(
            frame,
            &texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );
        texture.present();
    }
}
