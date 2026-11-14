use super::Renderer;
use crate::render::Frame;
use image::RgbaImage;

impl Renderer {
    pub fn render_image(&mut self, frame: &Frame) -> RgbaImage {
        let bgra = matches!(
            self.format,
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        );
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: frame.width,
                height: frame.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        self.render_texture(
            frame,
            &texture.create_view(&wgpu::TextureViewDescriptor::default()),
        );

        let row_bytes = frame.width as usize * 4;
        let padded_row_bytes =
            row_bytes.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (padded_row_bytes * frame.height as usize) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row_bytes as u32),
                    rows_per_image: None,
                },
            },
            texture.size(),
        );
        self.queue.submit([encoder.finish()]);
        buffer.slice(..).map_async(wgpu::MapMode::Read, |result| {
            result.expect("map image buffer")
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("wait for image");

        let mut pixels = Vec::with_capacity(row_bytes * frame.height as usize);
        for row in buffer.slice(..).get_mapped_range().chunks(padded_row_bytes) {
            for pixel in row[..row_bytes].chunks(4) {
                let alpha = pixel[3] as u32;
                let straight = |channel: u8| match alpha {
                    0 => 0,
                    _ => ((channel as u32 * 255 + alpha / 2) / alpha).min(255) as u8,
                };
                pixels.extend([
                    straight(pixel[if bgra { 2 } else { 0 }]),
                    straight(pixel[1]),
                    straight(pixel[if bgra { 0 } else { 2 }]),
                    pixel[3],
                ]);
            }
        }
        RgbaImage::from_raw(frame.width, frame.height, pixels).expect("rgba buffer")
    }
}
