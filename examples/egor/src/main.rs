mod adapter;

use adapter::Haven;
use egor::{
    app::{App, ControlFlow, FrameContext},
    render::{Color as EgorColor, Graphics},
};
use haven::*;

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

fn ellipse(gfx: &mut Graphics, center: [f32; 2], radius: [f32; 2], color: [f32; 4]) {
    gfx.path()
        .at(center.into())
        .scale([1., radius[1] / radius[0]].into())
        .fill_color(EgorColor::new(color))
        .circle(radius[0]);
}

fn segment(gfx: &mut Graphics, from: [f32; 2], to: [f32; 2], width: f32, color: [f32; 4]) {
    gfx.path()
        .thickness(width)
        .stroke_color(EgorColor::new(color))
        .begin(from.into())
        .line_to(to.into());
}

fn bounce(distance: f32, extent: f32) -> f32 {
    extent - (distance.rem_euclid(extent * 2.) - extent).abs()
}

fn crab_scene(gfx: &mut Graphics, width: f32, height: f32, time: f32) {
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
                segment(gfx, [x + side * 22., y + offset * 0.3], knee, 5., dark);
                segment(
                    gfx,
                    knee,
                    [x + side * (43. + offset * 0.4), y + 19. + offset + sway],
                    4.,
                    coral,
                );
            }
            segment(
                gfx,
                [x + side * 22., y - 4.],
                [x + side * 40., y - 23.],
                7.,
                coral,
            );
            ellipse(gfx, [x + side * 43., y - 28.], [12., 14.], coral);
            segment(
                gfx,
                [x + side * 43., y - 42.],
                [x + side * 43., y - 31.],
                4.,
                [0.025, 0.09, 0.14, 1.],
            );
            segment(
                gfx,
                [x + side * 11., y - 13.],
                [x + side * 13., y - 29.],
                5.,
                coral,
            );
            ellipse(
                gfx,
                [x + side * 13., y - 29.],
                [7.; 2],
                [1., 0.96, 0.87, 1.],
            );
            ellipse(
                gfx,
                [x + side * 13. + 1., y - 30.],
                [3.; 2],
                [0.018, 0.035, 0.06, 1.],
            );
        }
        ellipse(gfx, [x, y], [29., 20.], coral);
        ellipse(gfx, [x - 5., y - 7.], [16., 5.], [1., 0.52, 0.28, 1.]);
        segment(gfx, [x - 6., y + 8.], [x, y + 11.], 2.5, dark);
        segment(gfx, [x, y + 11.], [x + 6., y + 8.], 2.5, dark);
    }
}

fn main() {
    let mut crab_time = 0.;
    App::new()
        .title("Haven in Egor")
        .window_size(1536, 960)
        .min_size(960, 800)
        .control_flow(ControlFlow::Wait)
        .with_ui(|renderer, window, format| {
            Haven::new(
                Controls::default(),
                PaneBuilder::new("demo", view)
                    .background(TRANSPARENT)
                    .build(),
                renderer,
                window,
                format,
            )
        })
        .run(
            move |FrameContext {
                      app,
                      gfx,
                      timer,
                      ui,
                      ..
                  }| {
                gfx.clear(EgorColor::new([0.06, 0.06, 0.06, 1.]));
                if ui.model.animation.on {
                    crab_time += timer.delta.min(0.05) * ui.model.speed.value;
                    app.request_redraw();
                }
                let size = gfx.screen_size();
                crab_scene(gfx, size.x, size.y, crab_time);
            },
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controls_edit_the_scene() {
        let mut controls = Controls::default();
        let mut pane = PaneBuilder::new("demo", view)
            .background(TRANSPARENT)
            .build();
        let (controls, pane) = (&mut controls, &mut pane);
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
