use haven::winit::WinitApp;
use haven::*;

#[derive(Clone)]
struct State {
    texts: Vec<String>,
}

fn backing<State: 'static>(app: &mut PaneState) -> Layout<'static, View<State>, PaneState> {
    rect(id!())
        .corner_rounding(DEFAULT_CORNER_ROUNDING)
        .stroke(Color::from_rgb8(50, 50, 50), Stroke::new(2.))
        .fill(Color::from_rgb8(30, 30, 30))
        .build(app)
}

fn text_cell<'a>(i: usize, s: &str, ctx: &mut PaneState) -> Layout<'a, View<State>, PaneState> {
    stack(vec![
        rect(id!(i as u64))
            .fill(Color::from_rgb8(40, 40, 40))
            .corner_rounding(5.)
            .build(ctx),
        row(vec![
            text(id!(i as u64), s)
                .fill(Color::WHITE)
                .align(parley::Alignment::Start)
                .wrap()
                .build(ctx)
                .pad(10.),
            svg(id!(i as u64), include_str!("../assets/tiger.svg"))
                .finish(ctx)
                .height(100.),
        ]),
    ])
    .pad(6.)
}

fn main() {
    WinitApp::new(State { texts: texts() })
        .pane(
            PaneConfig::new("main", |state: &State, app: &mut PaneState| {
                let short = [state.texts[0].clone()];
                let long = state.texts.clone();

                let short_scroller = scroller(
                    id!(),
                    Some(backing(app)),
                    move |index, _id, ctx| short.get(index).map(|s| text_cell(index, s, ctx)),
                    app,
                );

                let long_scroller = scroller(
                    id!(),
                    Some(backing(app)),
                    {
                        let long = long.clone();
                        move |index, _id, ctx| long.get(index).map(|s| text_cell(index, s, ctx))
                    },
                    app,
                );

                let unconstrained_scroller = scroller(
                    id!(),
                    Some(backing(app)),
                    move |index, _id, ctx| {
                        if index < 3 {
                            Some(
                                rect(id!(index as u64))
                                    .fill(match index % 3 {
                                        0 => Color::from_rgb8(80, 40, 40),
                                        1 => Color::from_rgb8(40, 80, 40),
                                        _ => Color::from_rgb8(40, 40, 80),
                                    })
                                    .corner_rounding(5.)
                                    .build(ctx)
                                    .pad(6.),
                            )
                        } else {
                            None
                        }
                    },
                    app,
                );

                row_spaced(
                    12.,
                    vec![short_scroller, long_scroller, unconstrained_scroller],
                )
                .pad(20.)
            })
            .title("Scroller")
            .inner_size(1000, 640),
        )
        .run();
}

fn texts() -> Vec<String> {
    vec![
        "Rendering must not wait for logic resolution unless explicitly requested.".to_string(),
        "Event dispatch latency is a function of input queuing and frame scheduling.".to_string(),
        "A critical but often overlooked aspect of UI performance is how input events interact with ongoing animations and updates. Consider a scenario where a user is scrolling through a feed while asynchronous content loads in the background. If the rendering pipeline does not correctly prioritize the scroll event over the content updates, the experience feels sluggish.".to_string(),
        "Compositional hierarchies introduce deferred constraints on visual updates.".to_string(),
        "The pipeline favors consistency over immediate reflectivity; \nstate transitions must be acknowledged.".to_string(),
        "A view's presence in the tree does not guarantee its materialization at all times.".to_string(),
        "Preemption allows mid-frame adjustments but requires careful arbitration.".to_string(),
        "Scheduling decisions should account for both animation pacing\nand input sampling windows.".to_string(),
        "Hybrid models allow speculative rendering with rollback on reconciliation failures.".to_string(),
        "The challenge is not just in rendering individual components but in managing the lifecycle of entire UI trees over time. Every frame represents a snapshot of a constantly evolving system where changes must be synchronized without stalling user interactions.".to_string(),
        "Timeout thresholds must align with perceptual boundaries, \nnot just system constraints.".to_string(),
        "Metrics must be captured at multiple levels to ensure coherence across update cycles.".to_string(),
        "Even small state mutations can trigger cascading updates, \nwhich must be throttled accordingly.".to_string(),
        "If the tree changes mid-frame, what happens to the pending operations?".to_string(),
        "A single frame drop is perceptible, \nbut a consistent 30ms jitter is often unnoticed.".to_string(),
        "Optimizations that assume a fixed frame budget \nare brittle in asynchronous environments.".to_string(),
        "Not every component needs to respond to every state change, \nonly those within its dependency graph.".to_string(),
        "The renderer should avoid overcommitting resources \nuntil confirmation of stable state.".to_string(),
        "There is a threshold where deferred updates create a feeling of lag, \nwhich must be minimized.".to_string(),
        "Frame timings are not absolute; \nperceptual smoothness is what actually matters.".to_string(),
        "Visual coherence is often more important than immediate accuracy.".to_string(),
        "In extreme cases, dropping frames can be better than introducing stutter.".to_string(),
    ]
}
