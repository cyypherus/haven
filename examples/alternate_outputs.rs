use anyrender::ImageRenderer;
use anyrender_vello_cpu::VelloCpuImageRenderer;
use haven::render::{Frame, FrameOutput, FramePainter};
use haven::{Color, PaneBuilder, PaneState, Role, View, column, rect, text};
use image::RgbaImage;
use std::io::Write;
use std::process::{Command, Stdio};

const WIDTH: u32 = 320;
const HEIGHT: u32 = 180;

struct State {
    frame: u32,
}

fn view<'a>(state: &'a State, ctx: &mut PaneState) -> View<'a, State> {
    column(vec![
        text(1, "Haven")
            .view()
            .accessibility_role(Role::Label)
            .accessibility_label("Haven")
            .build(ctx)
            .height(60.),
        rect(2)
            .fill(Color::from_rgb8((state.frame * 4 % 256) as u8, 70, 232))
            .view()
            .accessibility_role(Role::Image)
            .accessibility_label("Animated color")
            .build(ctx)
            .width(120.)
            .height(80.),
    ])
}

struct Pixels {
    renderer: VelloCpuImageRenderer,
    painter: FramePainter,
}

impl FrameOutput for Pixels {
    type Output = RgbaImage;

    fn render(&mut self, frame: &Frame) -> Self::Output {
        let mut bytes = Vec::new();
        self.renderer
            .render_to_vec(|scene| self.painter.paint(frame, scene), &mut bytes);
        for pixel in bytes.chunks_exact_mut(4) {
            let alpha = u32::from(pixel[3]);
            if alpha == 0 {
                pixel[..3].fill(0);
            } else if alpha < 255 {
                for channel in &mut pixel[..3] {
                    *channel = ((u32::from(*channel) * 255 + alpha / 2) / alpha).min(255) as u8;
                }
            }
        }
        RgbaImage::from_raw(frame.width, frame.height, bytes).expect("RGBA frame size")
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let output_dir = args
        .iter()
        .find(|arg| !arg.starts_with("--"))
        .cloned()
        .unwrap_or_else(|| "target/alternate_outputs".to_string());
    std::fs::create_dir_all(&output_dir)?;

    let mut state = State { frame: 0 };
    let mut pane = PaneBuilder::new("example", view).build();
    let (frame, _) = pane.redraw(&mut state, WIDTH, HEIGHT, 1.0);
    let mut pixels = Pixels {
        renderer: VelloCpuImageRenderer::new(WIDTH, HEIGHT),
        painter: FramePainter::default(),
    };

    let mut png = |frame: &Frame| pixels.render(frame).save(format!("{output_dir}/frame.png"));
    png.render(&frame)?;

    let mut json = |frame: &Frame| {
        let nodes = frame
            .semantics
            .nodes
            .iter()
            .map(|(id, node)| serde_json::json!({ "id": id.0, "node": node }))
            .collect::<Vec<_>>();
        serde_json::to_string_pretty(&serde_json::json!({
            "width": frame.width,
            "height": frame.height,
            "focus": frame.semantics.focus.0,
            "nodes": nodes,
        }))
    };
    std::fs::write(format!("{output_dir}/frame.json"), json.render(&frame)?)?;

    let mut llm_text = |frame: &Frame| {
        let mut lines = vec![format!("Canvas: {}x{}", frame.width, frame.height)];
        for (id, node) in &frame.semantics.nodes {
            if frame
                .semantics
                .tree
                .as_ref()
                .is_some_and(|tree| tree.root == *id)
            {
                continue;
            }
            let mut line = format!("id={} role={:?}", id.0, node.role());
            if let Some(label) = node.label() {
                line.push_str(&format!(" label={label:?}"));
            }
            if let Some(value) = node.value() {
                line.push_str(&format!(" value={value:?}"));
            }
            if let Some(bounds) = node.bounds() {
                line.push_str(&format!(
                    " bounds=({:.0},{:.0},{:.0},{:.0})",
                    bounds.x0, bounds.y0, bounds.x1, bounds.y1
                ));
            }
            lines.push(line);
        }
        lines.join("\n") + "\n"
    };
    std::fs::write(format!("{output_dir}/frame.txt"), llm_text.render(&frame))?;

    if args.iter().any(|arg| arg == "--video") {
        let mut encoder = Command::new("ffmpeg")
            .args([
                "-y",
                "-loglevel",
                "error",
                "-f",
                "rawvideo",
                "-pixel_format",
                "rgba",
                "-video_size",
                "320x180",
                "-framerate",
                "30",
                "-i",
                "pipe:0",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
            ])
            .arg(format!("{output_dir}/video.mp4"))
            .stdin(Stdio::piped())
            .spawn()?;
        {
            let mut video = |frame: &Frame| {
                encoder
                    .stdin
                    .as_mut()
                    .expect("ffmpeg stdin")
                    .write_all(pixels.render(frame).as_raw())
            };
            for index in 0..60 {
                state.frame = index;
                let (frame, _) = pane.redraw(&mut state, WIDTH, HEIGHT, 1.0);
                video.render(&frame)?;
            }
        }
        encoder.stdin.take();
        if !encoder.wait()?.success() {
            return Err("ffmpeg failed".into());
        }
    }

    Ok(())
}
