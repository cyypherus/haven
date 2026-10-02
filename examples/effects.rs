use haven::winit::WinitApp;
use haven::*;

fn main() {
    WinitApp::new(())
        .pane(
            PaneBuilder::new("effects", |_: &(), app: &mut PaneState| {
                column_spaced(
                    40.,
                    vec![
                        text(id!(), "Vello Hybrid")
                            .font_size(42)
                            .fill(Color::WHITE)
                            .build(app)
                            .shadow((12., 12.), 5., Color::from_rgb8(245, 90, 140)),
                        row_spaced(
                            32.,
                            vec![
                                rect(id!())
                                    .fill(Color::from_rgb8(90, 190, 245))
                                    .corner_rounding(16.)
                                    .build(app)
                                    .width(120.)
                                    .height(80.)
                                    .blur(8.),
                                rect(id!())
                                    .fill(Color::from_rgb8(90, 190, 245))
                                    .corner_rounding(16.)
                                    .build(app)
                                    .width(120.)
                                    .height(80.)
                                    .shadow((12., 12.), 6., Color::BLACK),
                            ],
                        ),
                    ],
                )
                .pad(48.)
            })
            .title("Haven effects")
            .background(Color::from_rgb8(32, 36, 48))
            .inner_size(480, 300),
        )
        .run();
}
