<div align="center">

# Haven

**A WIP declarative UI crate for native applications.**

</div>

Haven handles windowing, layout, rendering, state, and user interaction for native Rust apps.

Built with [winit](https://github.com/rust-windowing/winit), [backer](https://github.com/cyypherus/backer), [anyrender](https://github.com/cyypherus/anyrender), [kurbo](https://github.com/linebender/kurbo) `0.13.0`, and [parley](https://github.com/linebender/parley) `0.7.0`.

_This library is functional but very experimental. API stability is not a goal at this stage & it is likely you will encounter bugs._

### Features

- Declarative API: The code should look like the structure it defines
- Flexible layout: Constraint-based layout powered by backer
- App runtime: Winit integration for running panes
- Rendering: Anyrender with Vello by default
- Interaction: Gestures, text editing, scrolling, buttons, toggles, sliders, and dropdowns

> [!WARNING]
> **Limitations**:
>
> Haven is probably not a good choice right now if you need:
>
> - Accessibility
> - Video or gif support
> - Rotation
> - Robust effects like blur and shadow
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

## Examples

Examples can be run directly with `cargo run --example <name>`.

- `buttons`: Button styling, labels, and click handlers
- `text_fields`: Text editing, wrapping, alignment, filtering, and focus
- `scroller`: Scrollable content with gesture handling
- `gestures`: Click, hover, drag, predicates, and gesture regions
- `image`: Loading and drawing image content
- `async`: Waking panes from async callbacks
- `productivity`: A larger app-shaped example

## Status

Haven is usable but new! Breaking changes may be relatively frequent as the crate matures.

## Contributing

This project is unlikely to be able to support any substantial volume of contributions as it's just a hobby project maintained during spare time. If you're interested in seeing a change in the library, feel free to open an issue.
