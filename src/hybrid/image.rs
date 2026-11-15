use super::HybridTextureRenderer;
use crate::render::Frame;
use image::RgbaImage;

pub struct HybridImageRenderer {
    renderer: HybridTextureRenderer,
    target: Option<(wgpu::Texture, wgpu::Buffer)>,
}

impl HybridImageRenderer {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let instance = wgpu::Instance::default();
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .expect("graphics adapter");
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("graphics device");
        Self {
            renderer: HybridTextureRenderer::with_device(
                &device,
                &queue,
                wgpu::TextureFormat::Rgba8Unorm,
            )
            .expect("supported image format"),
            target: None,
        }
    }

    pub fn render(&mut self, frame: &Frame) -> RgbaImage {
        let renderer = &mut self.renderer;
        let row_bytes = frame.width as usize * 4;
        let padded_row_bytes =
            row_bytes.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize);
        let (texture, buffer) = match &self.target {
            Some((texture, buffer))
                if (texture.width(), texture.height()) == (frame.width, frame.height) =>
            {
                (texture, buffer)
            }
            _ => {
                let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size: wgpu::Extent3d {
                        width: frame.width,
                        height: frame.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                });
                let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
                    label: None,
                    size: (padded_row_bytes * frame.height as usize) as u64,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                let (texture, buffer) = self.target.insert((texture, buffer));
                (&*texture, &*buffer)
            }
        };
        renderer.render(
            frame,
            &texture.create_view(&wgpu::TextureViewDescriptor::default()),
        );
        let mut encoder = renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row_bytes as u32),
                    rows_per_image: None,
                },
            },
            texture.size(),
        );
        renderer.queue.submit([encoder.finish()]);
        buffer.slice(..).map_async(wgpu::MapMode::Read, |result| {
            result.expect("map image buffer")
        });
        renderer
            .device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("wait for image");

        let mut pixels = vec![0; row_bytes * frame.height as usize];
        for (source, target) in buffer
            .slice(..)
            .get_mapped_range()
            .chunks(padded_row_bytes)
            .zip(pixels.chunks_exact_mut(row_bytes))
        {
            target.copy_from_slice(&source[..row_bytes]);
        }
        for pixel in pixels.as_chunks_mut::<4>().0 {
            let alpha = pixel[3] as u32;
            if alpha < 255 {
                for channel in &mut pixel[..3] {
                    *channel = match alpha {
                        0 => 0,
                        _ => ((*channel as u32 * 255 + alpha / 2) / alpha).min(255) as u8,
                    };
                }
            }
        }
        buffer.unmap();
        RgbaImage::from_raw(frame.width, frame.height, pixels).expect("rgba buffer")
    }
}
