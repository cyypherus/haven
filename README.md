<div align="center">

# Haven

**A declarative UI crate for native applications.**

</div>

Haven handles windowing, layout, rendering, state, and user interaction for native Rust apps.

Built with [winit](https://github.com/rust-windowing/winit), [backer](https://github.com/cyypherus/backer), [anyrender](https://github.com/dioxuslabs/anyrender), and [parley](https://github.com/linebender/parley).

_This library is functional but experimental. API stability is not a goal at this stage & it is likely you will encounter bugs._

### Features

- Declarative API: The code should look like the structure it defines
- Flexible layout: Constraint-based layout powered by backer
- App runtime: Winit integration for running panes
- Multiple windows: Run more than one pane from the same app
- Headless interaction: Drive panes directly for automated testing
- Rendering: AnyRender with Vello Hybrid by default; Vello CPU, classic Vello, and Skia are optional backends
- Effects: Gaussian blur and alpha-based drop shadows on views and groups
- Interaction: Gestures, text editing, scrolling, buttons, toggles, sliders, and dropdowns

> [!WARNING]
> **Limitations**:
>
> Haven is probably not a good choice right now if you need:
>
> - Accessibility
> - Video or gif support
> - Rotation
> - A stable, mature library which will rarely have bugs

# Quick Start

Define some state, write a view function, and pass that view into a pane.

```rust
use haven::winit::WinitApp;
use haven::*;

#[derive(Clone, Default)]
struct State {
    count: i32,
    button: ButtonState,
}

fn view<'a>(state: &'a State, app: &mut PaneState) -> View<'a, State> {
    column_spaced(
        20.,
        vec![
            text(id!(), format!("Count: {}", state.count))
                .fill(Color::WHITE)
                .build(app),
            button(id!(), binding!(state.button))
                .text_label("Increment")
                .on_click(|state, _app| state.count += 1)
                .build(app),
        ],
    )
    .pad(20.)
}

fn main() {
    WinitApp::new(State::default())
        .pane(
            PaneBuilder::new("main", view)
                .title("Counter")
                .inner_size(320, 180),
        )
        .run();
}
```

## Effects

Apply effects to a built view, including text, paths, images, or a group of views:

```rust
text(id!(), "Haven")
    .fill(Color::WHITE)
    .build(app)
    .shadow((12., 12.), 6., Color::BLACK)
    .blur(2.)
```

`.blur(radius)` applies a Gaussian blur. `.shadow((x, y), blur, color)` draws a shadow from the
content's alpha and retains the original content. Blur values are Gaussian standard deviations in
logical pixels; offsets also use logical pixels. Negative blur values become zero, and non-finite
blur values or offsets panic.

Effects compose in call order and do not change layout or hit regions. They can extend beyond the
view's layout bounds; use `.clipped(...)` to constrain them.

The default `renderer-vello-hybrid` backend runs blur and shadow filters on the GPU.
`renderer-vello-cpu` and `renderer-skia` also support these filters. Classic `renderer-vello`
ignores them. To select another backend, disable default features and enable exactly one renderer
feature, for example:

```sh
cargo run --no-default-features --features renderer-vello-cpu --example effects
```

## Examples

Examples can be run directly with `cargo run --example <name>`.

- `buttons`: Button styling, labels, and click handlers
- `text_fields`: Text editing, wrapping, alignment, filtering, and focus
- `scroller`: Scrollable content with gesture handling
- `gestures`: Click, hover, drag, predicates, and gesture regions
- `image`: Loading and drawing image content
- `async`: Waking panes from async callbacks
- `productivity`: A larger app-shaped example
- `effects`: Gaussian blur and drop shadows on text and rounded rectangles

## Status

Haven is usable but new! Breaking changes may be relatively frequent as the crate matures.

## Contributing

This project is unlikely to be able to support any substantial volume of contributions as it's just a hobby project maintained during spare time. If you're interested in seeing a change in the library, feel free to open an issue.
