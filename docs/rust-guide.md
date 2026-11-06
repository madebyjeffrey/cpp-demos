# Rust SCARS quick guide

The Rust workspace lives in `rust/`. It has a portable `scars-ui` core, a checked
`view!` macro, and `scars-window` for macOS/AppKit and Skia/Metal. SCARS controls,
layout, state, editing and value models are Rust. Skia still compiles its native
C++ engine from source; the C++ SCARS engine is not linked.

## Run

Requires macOS 26+, Rust 1.88+, Xcode command-line tools, Python 3, Ninja and Git.
Cargo fetches and builds Skia on the first build. Later builds reuse local outputs.

```sh
cd rust
cargo run --locked --example todo
cargo run --locked --example lab -- --light
```

The todo app supports adding, editing, completion, select-all, All/Active/Completed
filters, deletion, and clearing completed tasks. Enter submits; Escape cancels an
edit. It saves to `~/Library/Application Support/SCARS Todo/tasks.tsv` with a
versioned UTF-8 format and atomic replacement. `--data /path/tasks.tsv` chooses a
file. Load errors stop startup; save errors remain visible while changes stay in
memory. One process should own a data file at a time. `--preview`, `--smoke`, and
`--render file.png` use sample tasks without reading or writing saved tasks.

All stages have runnable examples:

| Stage | Example | Demonstrates |
| --- | --- | --- |
| 1 | `foundation` (`-p scars-ui`), `stage1` | State, keys, macros and Unicode services |
| 2 | `stage2` | Layout, basic controls, native input and semantics |
| 3 | `stage3` | History, password/multiline fields, typed numeric drafts |
| M2 | `m2_editor`, `m2_fields`, `m2_numbers` | Completed editing, scoped field policies and repeating steppers |
| 4 | `stage4` | Scroll, popup/menu, focus restoration, tabs and split views |
| 5 | `stage5` | 100,000 virtual rows, trees, sorting and conflict-checked edits |
| 6 | `stage6`, `lab` | Color/date/time values, inspector and control catalog |

Examples 2–6, Lab and Todo accept `--light`, `--smoke`, and `--render file.png`.
Render mode uses CPU Skia and accepts `--inspect` to print layout bounds.

## A small application

From another project, depend on `scars-window` by path. Copy the source-build
configuration as described in [the Rust README](../rust/README.md#skia-source-builds).

```rust,no_run
use scars_window::{Application, Appearance, Window, ui::*};

fn main() -> scars_window::Result<()> {
    let name = State::new(String::new());
    Application::named("Profile")?.run_with_windows(move || {
        let name = name.clone();
        Window::builder("Profile")
            .theme_toggle(Appearance::Dark)
            .full_screen_button()
            .view(move || view! {
                column(padding: 24, gap: 12) {
                    text_field("Your name", bind: name, autofocus: true);
                    label(format!("Hello, {}", name.get()));
                    button("Clear", on_click: name.action(|v| v.set(String::new())));
                }
            })
    })
}
```

Create state outside builders. Clone a `State` to share it; `binding()` exposes an
owned getter/setter. `State::action` captures a state handle for an event. Builders
are pure: reads subscribe the host and writes during construction are rejected.
Rebuilds are lazy and coalesced. Keep image resources and reusable draft/popup/value
models outside builders as well.

`cargo run --example m3_menu` demonstrates hierarchical menus and a split action
that remembers its selected primary action. `Menu::button` hides trigger callback
plumbing; `Menu::view` provides the overlay. See the [menu contracts](../rust/components/README.md#completion-m3--menu-trees-and-split-actions).

`cargo run --example m3_overlay` demonstrates configurable popovers, tooltips and
attached context menus. `Popup::options` controls placement, sizing and focus;
`view.context_menu(&menu, items)` attaches a menu without a separate overlay.
See the [overlay contracts](../rust/components/README.md#completion-m3--overlay-policy).

`cargo run --example m3_tabs` demonstrates retained tab pages. `Tabs`/`Tab` preserve
visited controls and their editor/scroll/focus state; `Tab::lazy` defers heavier pages.
See the [tab contracts](../rust/components/README.md#completion-m3--retained-tabs).

`cargo run --example m3_wizard` demonstrates validated setup/review navigation.
`Wizard` owns step policy and callbacks; `wizard_view(&model, pages)` retains page
content. See the [wizard contracts](../rust/components/README.md#completion-m3--validated-wizards).

`cargo run --example m3_alerts` demonstrates single-use alert transactions; add
`--sheet` for a native parent-modal sheet. `Alert::view` supplies an overlay and
`surface.show_sheet(alert, body_builder)` owns the native dialog through completion.
Use `view_with_surface` when component actions need window context. See the
[alert contracts](../rust/components/README.md#completion-m3--alert-transactions-and-native-sheets).

`cargo run --example m4_list` demonstrates keyed list selection, navigation,
typeahead, activation and configurable overscan across 100,000 rows. `ListOptions`
sets realization policy and `select_with` makes range/additive operations explicit.
See the [list contracts](../rust/components/README.md#completion-m4--list-interaction).

`cargo run --example m4_list_edit` demonstrates retained row edits. Hold a
`list.pin(key)` handle while editing and release it on Save/Cancel; host undo,
selection and composition survive scrolling. `on_activate_with_model` supplies
model context without a strong self-capture. See the
[virtual edit contracts](../rust/components/README.md#completion-m4--virtual-edit-lifetimes).

`cargo run --example m4_tree` demonstrates cached lazy/deferred branches, visible
load failure/retry and generation-checked results. `TreeNode` is recursive input;
`TreeItem` is queried metadata. `expand`/`collapse` return results and model replacement
retires old requests. See the [tree model contracts](../rust/components/README.md#completion-m4--lazy-tree-models).

`view!` supports ordinary expressions, `if`/`else`, keyed `for` loops, custom
functions returning `View`, and `embed(view)` for an already-built value. Unknown
properties fail compilation. Use stable keys for reordered items; unkeyed identity
follows sibling position. No application-side `Rc` or weak window ownership is
needed for ordinary builder code.

`WindowBuilder::view` mounts controls and connects drawing, events, native IME,
clipboard and accessibility. `Surface::set_view` replaces a mount transactionally;
`clear_view` detaches it. Applications choose menus beyond the default Quit item.
Closing the last top-level window quits unless explicitly suppressed.

Typed editors use `Draft<T>` to separate unfinished text from committed values.
`Numeric`, `ColorPicker`, `DatePicker`, `ZonedPicker` and the time helpers build on
it. Initial editable formats are explicit ASCII numbers and ISO dates/times;
ICU4X supplies Unicode boundaries and localized integer display. Named zones use
Jiff and bundled timezone data. Ambiguous/nonexistent local times are rejected by
default, with explicit earlier/later resolution available.

See [component contracts](../rust/components/README.md) for layouts, controls,
selection, popup placement, virtual models, deadlines, native semantics and
inspection. [Window contracts](../rust/README.md) cover ownership and rendering.

## Validation and current limits

The [completion checklist](rust-completion-checklist.md) tracks the remaining
behavior and validation work against the original SCARS inventory.

```sh
cd rust
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo check -p scars-ui --lib --target x86_64-unknown-linux-gnu --locked
SCARS_WINDOW_DESKTOP_TESTS=1 cargo test --locked --test components
SCARS_WINDOW_DESKTOP_TESTS=1 cargo test --locked --test desktop
SCARS_WINDOW_FULLSCREEN_TESTS=1 cargo test --locked --test fullscreen
SCARS_WINDOW_SECURE_INPUT_TESTS=1 cargo test --locked --test secure_input
```

Native tests require an awake desktop with WindowServer/Metal access; ordinary
Cargo tests skip those harnesses. Core/CPU tests run without a native window.
Linux is a compile check for the portable core; its native backend is not implemented.

This is the first Rust implementation of the staged component system. Collection
virtualization uses fixed row heights; durable per-row edit state belongs outside
the viewport. Trees accept application-loaded lazy children. Table accessibility
currently exposes logical row summaries and indexed row actions. Layout supports
rich inline paragraphs, constrained reflow and both-axis scrolling with draggable
bars. Split views have draggable, keyboard-adjustable dividers with collapse and
retained pane state. Color controls currently expose sRGB channels. These interfaces do not claim full C++ behavior or pixel parity.
The [completion checklist](rust-completion-checklist.md) tracks remaining work,
including localized editing, table-cell semantics and an actual VoiceOver/IME audit.
Variable-height virtualization is a separate extension.
