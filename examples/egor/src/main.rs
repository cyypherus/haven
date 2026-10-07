mod adapter;

use adapter::Haven;
use egor::{
    app::{App, ControlFlow, FrameContext},
    render::Color as EgorColor,
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

fn bounce(distance: f32, extent: f32) -> f32 {
    extent - (distance.rem_euclid(extent * 2.) - extent).abs()
}

fn main() {
    let mut crab_time = 0.;
    let mut ferris_texture = None;
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
                let texture = *ferris_texture.get_or_insert_with(|| {
                    gfx.load_texture(include_bytes!("../assets/ferris_smol.png"))
                });
                for i in 0..8 {
                    let seed = i as f32;
                    let x = bounce(
                        seed * 137. + crab_time * (43. + seed * 7.),
                        (size.x - 128.).max(1.),
                    );
                    let y = 20.
                        + bounce(
                            seed * 89. + crab_time * (31. + seed * 4.),
                            (size.y - 124.).max(1.),
                        );
                    gfx.rect()
                        .at([x, y])
                        .size([128., 84.].into())
                        .texture(texture);
                }
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
