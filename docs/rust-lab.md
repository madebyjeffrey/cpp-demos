# Rust component Lab

Run from the Rust workspace:

```sh
cargo run --example lab --locked --offline
cargo run --example lab --locked --offline -- --page values --light --size 440x740
cargo run --example lab --locked --offline -- --list
```

Seven retained pages contain 28 named samples. The page strip scrolls at narrow
widths; arrow keys navigate it. Cards wrap into columns according to available
width, and each page scrolls. Visited pages retain editing and component state.
Enable samples and Read-only editors exercise availability without rebuilding
application models. Native sheet integration remains demonstrated by
`m3_alerts --sheet`; Lab's alert uses the same portable transaction.

| Page | Samples | Inventory coverage |
| --- | --- | --- |
| Controls | actions, choices, slider, custom | Buttons/commands, toggle/check/three-state/switch/radio/segmented choices, slider, both progress modes, reusable stateful components |
| Layout | typography, boxes, images, styles, scroll | Labels/text roles, styled paragraphs and inline controls, row/column/block/inline/layer, section/group/panel/spacer/separators, all four image modes, flex/constraints/inherited styles, two-axis scrolling |
| Fields | text, search, password, numeric | Annotated single/multiline fields, validation/readonly, search debounce/clear, password reveal, typed numeric drafts and standalone steppers |
| Flow | menus, overlays, splits, tabs, wizard | Check/radio/submenus, dropdown and remembered split action, tooltip/context/popover/alert, both split axes, retained tabs, validated wizard |
| Data | list, tree, combo, table | 10,000 virtual rows, lazy tree, 1,000 combo choices, keyed sorting/resizing/reordering and validated cell edits |
| Values | date, time, datetime, zoned, color | Localized calendar, clock components, civil/instant and zone policies, RGB/HSV/hex/alpha plus anchored color well |
| Tools | inspector | Mounted hierarchy, picking, diagnostics, property patches/reset, clipped box/flex highlights |

The full public type/modifier mapping is in [the inventory](rust-inventory.md).
This catalog groups models, options and helper APIs with the controls that use
them; it does not duplicate the lower-level diagnostic examples or certify the
remaining interactive/Metal release audits.

## Reproduce a view

`--page` accepts the page IDs in the table. `--state` accepts `default`, `disabled`,
`readonly` or `error`. `--show menu|dropdown|popover|alert|combo|color` opens a
presentation and chooses its page. Startup actions reveal and focus their actual
opener after layout, so offscreen anchors remain valid.
All examples using the shared runner accept `--size WIDTHxHEIGHT`; raster mode
also supports `--reveal-key KEY` to scroll a mounted target into view.

```sh
cargo run --example lab --locked --offline -- --page layout --light \
  --size 440x740 --reveal-key sample-images --render /tmp/lab-images.png
cargo run --example lab --locked --offline -- --show combo \
  --render /tmp/lab-combo.png
cargo run --example lab --locked --offline -- --page fields --state error \
  --reveal-key sample-numeric --render /tmp/lab-error.png
```

The raster runner uses CPU Skia. These dimensions are logical points at raster
scale 1; they do not establish actual display backing-scale or visible Metal
passes. Actual VoiceOver/IME and release visual evidence remain on the checklist.

## Writing a component

Stateful components are ordinary Rust models plus functions returning `View`.
Create models once outside the application builder, and pass them to `view!`:

```rust
fn counter(count: &State<i64>) -> View {
    view! { row {
        label(count.get().to_string());
        button("+1", on_click: count.action(|s| s.update(|n| *n += 1)));
    } }
}
// Create these before constructing the view builder.
let downloads = State::new(0i64);
let uploads = State::new(0i64);
let build = move || view! { column {
    counter(&downloads, key: "downloads");
    counter(&uploads, key: "uploads");
} };
```

The Lab's [Counter](../rust/examples/lab/model.rs) also has independent disclosure
state. Its [view](../rust/examples/lab/view.rs) uses conditional content and normal
state actions. The application contains no reference-counting or native ownership
plumbing. Keys retain mounted identity; state reads subscribe automatically.
Custom function arguments are positional; named properties apply normal `View`
modifiers to the returned component. Child content can be passed as a `View`
argument. Constructors and reusable resources stay outside pure view builders.

`labeled("Level", control)` or `control.labeled("Level")` adds a visible annotation
and supplies an accessible name unless the control already has an explicit one.
A slider's `page_step:` property (or `.page_step(value)`) configures Page Up/Down
independently of arrow-key steps. The default is ten arrow steps; all changes snap
to the slider's step grid and clamp to its bounds. Page steps must be finite and
positive.

## Checks and artifacts

```sh
cargo test --test lab --locked --offline
SCARS_LAB_ARTIFACTS=/tmp/scars-lab-gallery cargo test --test lab --locked --offline
cargo test -p scars-ui --test controls --locked --offline
```

The catalog test selects and reveals every sample in both themes at 440×740 and
1120×900, validates finite/reachable geometry and actually paints with Skia. Tall
cards also get a bottom view. The optional directory receives those PNGs. Other
tests exercise independent custom instances, retained editor identity, readonly
changes, modal/menu scope, real collection edits and all six startup presentations. Collection realization stays
bounded. A separate renamed-dependency consumer compiles custom macro components,
annotations and slider/property interfaces.
