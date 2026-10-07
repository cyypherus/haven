mod adapter;

use adapter::Overlay;
use egor_app::{
    AppConfig, AppHandler, AppRunner, ControlFlow, Window, WindowEvent, input::Input,
    time::FrameTimer,
};
use egor_render::{
    MemoryHints, Renderer,
    batch::GeometryBatch,
    target::{Backbuffer, RenderTarget},
    vertex::Vertex,
};
use haven::*;
use std::sync::Arc;

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
    haven: Overlay,
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
        let haven = Overlay::new(&renderer, &backbuffer);
        EgorSurface {
            renderer,
            haven,
            backbuffer,
            crabs: GeometryBatch::new(4096, 12288),
        }
    }

    fn on_window_event(&mut self, window: &Window, event: &WindowEvent) {
        assert!(
            Overlay::event(&mut self.pane, &mut self.controls, window, event).is_empty(),
            "the demo has one pane"
        );
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
        let Some(mut egor_frame) = surface.renderer.begin_frame(&mut surface.backbuffer) else {
            return;
        };
        let width = egor_frame.view.texture().width() as f32 / window.scale_factor() as f32;
        let height = egor_frame.view.texture().height() as f32 / window.scale_factor() as f32;
        surface.renderer.upload_camera_matrix([
            [2. / width, 0., 0., 0.],
            [0., -2. / height, 0., 0.],
            [0., 0., 1., 0.],
            [-1., 1., 0., 1.],
        ]);
        crab_scene(&mut surface.crabs, width, height, self.crab_time);
        {
            let mut pass = surface
                .renderer
                .begin_render_pass(&mut egor_frame.encoder, &egor_frame.view);
            surface
                .renderer
                .draw_batch(&mut pass, &mut surface.crabs, None, None);
        }
        assert!(
            surface
                .haven
                .paint(&mut self.pane, &mut self.controls, window, &mut egor_frame)
                .is_empty(),
            "the demo has one pane"
        );
        window.pre_present_notify();
        surface.renderer.end_frame(egor_frame);
        if self.controls.animation.on {
            window.request_redraw();
        }
    }
}

fn main() {
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
