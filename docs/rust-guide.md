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
| 6 | `stage6` | Color/date/time values and inspection |
| M7.3 | `lab` | Full catalog, retained pages and stateful custom components |
| M7.1–M7.2 | `m7_inspector` | Mounted hierarchy, partial property overrides and layout diagnostics |

Examples 2–6, Lab and Todo accept `--light`, `--smoke`, and `--render file.png`.
Render mode uses CPU Skia and accepts `--inspect` to print layout bounds. Shared
examples also accept `--size WIDTHxHEIGHT` and raster `--reveal-key KEY`.
`--scale 1|2` chooses raster pixel density; native windows follow their display.
See the [Lab guide](rust-lab.md) for all 28 samples, page/state/presentation flags,
reproducible previews and concise custom component functions.
Inspected raster examples also accept `--select-key KEY` to select a source node.
The [example regression runner](rust-example-regressions.md) checks every Cargo
example and keeps unavailable native smoke checks explicit.

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
plumbing; `Menu::view` provides the overlay. See the [menu contracts](../rust/crates/scars-ui/README.md#completion-m3--menu-trees-and-split-actions).

`cargo run --example m3_overlay` demonstrates configurable popovers, tooltips and
attached context menus. `Popup::options` controls placement, sizing and focus;
`view.context_menu(&menu, items)` attaches a menu without a separate overlay.
See the [overlay contracts](../rust/crates/scars-ui/README.md#completion-m3--overlay-policy).

`cargo run --example m3_tabs` demonstrates retained tab pages. `Tabs`/`Tab` preserve
visited controls and their editor/scroll/focus state; `Tab::lazy` defers heavier pages.
See the [tab contracts](../rust/crates/scars-ui/README.md#completion-m3--retained-tabs).

`cargo run --example m3_wizard` demonstrates validated setup/review navigation.
`Wizard` owns step policy and callbacks; `wizard_view(&model, pages)` retains page
content. See the [wizard contracts](../rust/crates/scars-ui/README.md#completion-m3--validated-wizards).

`cargo run --example m3_alerts` demonstrates single-use alert transactions; add
`--sheet` for a native parent-modal sheet. `Alert::view` supplies an overlay and
`surface.show_sheet(alert, body_builder)` owns the native dialog through completion.
Use `view_with_surface` when component actions need window context. See the
[alert contracts](../rust/crates/scars-ui/README.md#completion-m3--alert-transactions-and-native-sheets).

`cargo run --example m4_list` demonstrates keyed list selection, navigation,
typeahead, activation and configurable overscan across 100,000 rows. `ListOptions`
sets realization policy and `select_with` makes range/additive operations explicit.
See the [list contracts](../rust/crates/scars-ui/README.md#completion-m4--list-interaction).

`cargo run --example m4_list_edit` demonstrates retained row edits. Hold a
`list.pin(key)` handle while editing and release it on Save/Cancel; host undo,
selection and composition survive scrolling. `on_activate_with_model` supplies
model context without a strong self-capture. See the
[virtual edit contracts](../rust/crates/scars-ui/README.md#completion-m4--virtual-edit-lifetimes).

`cargo run --example m4_tree` demonstrates cached lazy/deferred branches, visible
load failure/retry and generation-checked results. `TreeNode` is recursive input;
`TreeItem` is queried metadata. `expand`/`collapse` return results and model replacement
retires old requests. `TreeOptions` configures selection/geometry; Left/Right navigate
branches, and activation/expansion callbacks keep application behavior explicit.
See the [tree interaction contracts](../rust/crates/scars-ui/README.md#completion-m4--tree-interaction)
and [tree model contracts](../rust/crates/scars-ui/README.md#completion-m4--lazy-tree-models).

`cargo run --example m4_combo` demonstrates separate draft/committed choice,
selection/free-text modes, loading and a virtual 100,000-option popup. Build a
`ComboBox` once and `embed(combo.view("Find an option"))`; query/accept/cancel
callbacks keep application policy explicit. Add `--open` to preview suggestions.
See the [combo contracts](../rust/crates/scars-ui/README.md#completion-m4--combo-boxes).

`cargo run --example m4_table` demonstrates keyed rows/columns, bounds-checked
resize/reorder, sort descriptors and row/cell selection across 100,000 rows.
Construct `TableColumn`s and keyed `TableRow::cell` values, then embed `table.view()`.
Header drag/keyboard controls and shared scrolling preserve logical cell identity.
Native AXTable rows, columns, headers and cells include offscreen data through lazy
arrays. `--semantics` prints portable metadata; `table_accessibility` is the opt-in
native protocol test. See the [table semantic contracts](../rust/crates/scars-ui/README.md#completion-m5--table-semantics).
See the [table contracts](../rust/crates/scars-ui/README.md#completion-m4--keyed-tables-and-columns).

`cargo run --example m4_table_edit` adds integrated editing with `.editable(true)`,
pure validators, multiline text and readonly custom display. The model owns the
active draft and pin; use `on_commit` for application updates. Add `--invalid` to
preview validation, or use the example's conflict button to simulate remote data.
See the [table editing contracts](../rust/crates/scars-ui/README.md#completion-m4--integrated-table-editing).

Tree views expose logical AppKit outline rows with disclosure links/actions,
selection and loading/failure descriptions, including offscreen rows. Run
`cargo run --example m4_tree -- --semantics` for a portable row-state listing.
The native suite is `SCARS_WINDOW_DESKTOP_TESTS=1 cargo test --test tree_accessibility`.
See the [tree semantic contracts](../rust/crates/scars-ui/README.md#completion-m5--tree-semantics).

`cargo run --example m5_semantics` is a compact settings example combining tabs,
menus, validated fields and an alert. Numeric/value fields expose validation
without extra wiring; ordinary fields accept `validation:` and `accessible_help:`.
`--semantics` prints metadata. Native identity, scoped hit testing, real screen
coordinates and posting without drawing are exercised by `accessibility_lifecycle`
(the broader notification audit is still open). See the [control semantic contracts](../rust/crates/scars-ui/README.md#completion-m5--control-and-presentation-semantics).

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
`Numeric`, `ColorPicker`, `DatePicker`, `TimePicker`, `DateTimePicker`, `ZonedPicker` and the time helpers build on
it. Initial editable formats are explicit ASCII numbers and ISO dates/times;
ICU4X supplies Unicode boundaries and localized integer display. Named zones use
Jiff and bundled timezone data. The zoned control rejects nonexistent wall times;
repeated times require an explicit earlier/later policy. Low-level `resolve_zoned`
retains its explicit general resolution policies.

`TimePicker::new(value.binding())` adds dedicated component steppers around one
draft. Configure `hour_cycle`, `show_seconds`, bounds and step sizes before mounting,
then embed `picker.view()`. `m6_time` demonstrates two clocks sharing an exact value;
hour/minute edits preserve hidden seconds/nanoseconds. `DateTimePicker::new(value.binding())`
provides one civil datetime draft with a calendar and clock steppers; `m6_datetime`
demonstrates joint overnight bounds without intermediate application writes.
`ZonedPicker::new(instant.binding(), "America/Edmonton")?` binds a `Timestamp` and
provides the same calendar/clock with instant bounds and explicit display-zone
changes. `m6_zoned` demonstrates gaps/folds and KeepInstant versus KeepLocal.

`ColorPicker::new(paint.binding())` stages RGB/HSV/hex changes until Apply/Enter.
`live_preview(true)` publishes valid edits and rolls back only owned previews on
Cancel. `alpha(false)` preserves hidden alpha. `m6_color` shows these policies and
an anchored well; use one picker per independent editing session.

`cargo run --example m6_drafts` demonstrates application acceptance and retry
across all six typed editors. They share `binding()`, `validation()`, `commit()`
and `cancel()`: rejected updates retain text and feedback, accepted normalization
is read back, and newer callback edits supersede the original action. Blur applies
when leaving the whole editor; internal Revert does not apply early. Host actions
wait for preedit confirmation. `--prefix --tab=zone` demonstrates unfinished input;
the example also has a blur-policy toggle.

`cargo run --example m7_inspector` supplies a virtual mounted hierarchy with
keyboard browsing and picking. It reports live geometry, focus/selection,
validation, overlay scope and collection windows. Picking preserves input state;
source removal/teardown clears selection. `Inspector::nodes()` returns owned,
model-free snapshots; `select(id)` reveals a node. Richer overrides and the full
Lab remain in M7.2/M7.3.

See [component contracts](../rust/crates/scars-ui/README.md) for layouts, controls,
selection, popup placement, virtual models, deadlines, native semantics and
inspection. [Window contracts](../rust/README.md) cover ownership and rendering.

## Validation and current limits

The [completion checklist](rust-completion-checklist.md) tracks the remaining
behavior and validation work against the original SCARS inventory.

Run from the repository root:

```sh
python3 scripts/check_rust.py --offline
python3 scripts/check_rust.py --suite portable --offline
# On an awake, unlocked foreground desktop, after the headless build:
python3 scripts/check_rust.py --suite native --offline
```

The [validation guide](rust-validation.md) describes setup, suite scope, per-step
logs/JSON reports, Lab artifacts and CI. Native harness skips in ordinary tests do
not establish native passes. Actual VoiceOver speech and IME candidate placement
remain explicit manual checks. Linux validates the portable core; its native
window backend is not implemented.

This is the first Rust implementation of the staged component system. Collection
virtualization uses fixed row heights; durable per-row edit state belongs outside
the viewport. Trees accept application-loaded lazy children. Table accessibility
exposes logical rows/columns/cells, indexed queries, selection and validated editing. Layout supports
rich inline paragraphs, constrained reflow and both-axis scrolling with draggable
bars. Split views have draggable, keyboard-adjustable dividers with collapse and
retained pane state. Color controls provide RGB/HSV/hex editing over encoded sRGB. These interfaces do not claim full C++ behavior or pixel parity.
The [completion checklist](rust-completion-checklist.md) tracks remaining work,
including richer inspector overrides, Lab expansion and an actual VoiceOver/IME audit.
Checked `EditLocale` profiles now share explicit number/date editing grammars;
ASCII/ISO remains the default. See the [format contract](rust-editable-formats.md). Native notification
targets, bounded snapshots and lifecycle contracts have dedicated AppKit tests.
Variable-height virtualization is a separate extension.
