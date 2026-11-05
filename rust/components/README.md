# SCARS components

The portable foundation is implemented: reactive `State`/`Binding`, immutable
descriptions, `view!`, mounted identity, action dispatch and ICU4X text services.
Layout, interactive controls and native widget accessibility are implemented in stage 2.

From `rust/`:

```sh
# Portable, deterministic demonstration; no window or GPU needed.
cargo run --locked -p scars-ui --example foundation

# macOS diagnostic window: Space activates, R reorders, Escape closes.
cargo run --locked --example stage1

# Pure Rust core and macro diagnostics.
cargo test --locked -p scars-ui
# Shared Skia paragraph geometry; no window/GPU needed.
cargo test --locked -p scars-window --test paragraph
```

The diagnostic window paints an inspection snapshot through the existing Skia
callback. Its rows are descriptions, not finished interactive widgets.

## Small application interface

```rust
use scars_ui::{Host, State, view};

let show_details = State::new(true);
let host = Host::new(move || view! {
    column {
        label("Settings");
        if show_details.get() {
            label("Additional details", key: "details");
        }
        button("Toggle", key: "toggle", on_click: {
            let show_details = show_details.clone();
            move || show_details.update(|value| *value = !*value)
        });
    }
});
let nodes = host.snapshot()?;
```

Custom component functions return `View` and can be called inside `view!`.
`if`/`else if`/`else` and `for` accept ordinary Rust expressions. Repeated children
should declare stable `key:` values. Unsupported properties produce a compiler error. Controls accept typed bindings; see stage 2 below.

## Contracts

- Keep model state outside the builder. Cloning a `State` shares its value.
  `get` requires `Clone`; `set` requires `PartialEq` and suppresses equal writes.
  `update` transforms a snapshot. `binding()` or `Binding::new` supplies owned
  getter/setter access; a custom setter may normalize or reject a proposed value.
- State and hosts are single-threaded Rust types. Builders are pure: state writes
  during construction panic. Construction tracking is restored on unwind.
- Changed observed values mark the host dirty. Writes coalesce until `refresh`,
  `snapshot` or `activate`. Dependencies are replaced only after a successful
  rebuild. A failed builder or duplicate key leaves the mounted tree intact.
- Identity follows parent identity, sibling key (or unkeyed position), and kind.
  Keys must be nonempty and unique among siblings. Removed IDs are not reused;
  stale or foreign actions return `false`. `generation` counts successful builds.
- `snapshot` returns owned inspection data. `activate` invokes the current action
  outside host borrows; an action may update state and query the host. Rebuilds
  happen on the next query. Recursive construction of the same host is rejected.
- `on_invalidate` connects a redraw scheduler. Use a weak window capture, as in the
  native diagnostic example; it should schedule work without rebuilding immediately.
  The host releases its state subscriptions through weak observer ownership.
- `text` provides ICU4X grapheme/word boundaries, checked UTF-8-byte/UTF-16-unit
  conversions, and integer display with BCP 47 locales and standard ICU4X fallback.
  Interior byte/surrogate positions return `None`. No localized parsing policy is
  selected here. ICU4X 2.4 raises the workspace minimum to Rust 1.88.

The core's dependencies contain neither Skia nor native windowing. The window
crate's paragraph test uses source-built Skia and a bundled font to check
geometry and index conversions without a window or Metal device. The window crate
re-exports shared Skia types outside its native backend; all native implementation
continues to live under its platform folder.

## Stage 2 — usable controls

`cargo run --example stage2` opens interactive preferences. Tab/Shift-Tab moves
focus; arrow keys adjust sliders and grouped selections. `--render /tmp/ui.png`
writes a CPU-rendered preview; `--smoke` closes automatically.

```rust,ignore
use scars_window::{Window, ui::*};
let name = State::new(String::new());
Window::builder("Profile").view(move || view! {
    column(padding: 24, gap: 12) {
        text_field("Your name", bind: name, accessible_label: "Name");
        label(format!("Hello, {}", name.get()));
        button("Clear", on_click: name.action(|name| name.set(String::new())));
    }
});
```

Available controls: labels, buttons, toggles, bool/tri-state checkboxes, switches,
radio groups, segmented selection, sliders, determinate/indeterminate progress,
encoded/RGBA images and a single-line text field. Containers include column, row,
block, wrapping inline, layer, section, group box and panel. Common properties
include padding/gap/margin, width/height (points or `percent`), min/max, grow/shrink,
alignment/justification, wrapping, absolute anchors, z-index, colors, font and radius.
The single top-level description fills the viewport; `hidden` retains layout but
removes paint/input/semantics; `visible: false` removes the description.

`State::action` captures a shared state handle without application-side `Rc`.
Bindings accept either `State` or `Binding`. Builders stay pure. Keep expensive
resources, including images, outside them. All coordinates are logical points.

`WindowBuilder::view` installs rendering, pointer/keyboard dispatch, native IME,
clipboard shortcuts and accessibility semantics. `Surface::set_view` replaces a
mount transactionally; `clear_view` detaches it. Existing drawing is an underlay;
unhandled events reach the previous callback. Closing/detaching invalidates cached
accessibility elements. The portable `Host` can also use any pure `Measure` provider;
`scars_window::ui::Renderer` supplies SkParagraph shaping and Skia painting.

Editing uses byte indices internally and checked UTF-16 conversion at AppKit.
IME drafts do not reach the binding until committed; external binding changes
cancel drafts. Selection/deletion use ICU4X grapheme boundaries. This milestone
has no undo history or multiline/password/numeric editing; those belong to stage 3.

Tests: `cargo test --workspace`; native protocol/actions additionally use
`SCARS_WINDOW_DESKTOP_TESTS=1 cargo test --test components` on an awake desktop.

## Stage 3 — editing and typed drafts

`cargo run --example stage3` demonstrates search, masked passwords, a multiline
editor and an integer field with step buttons. Command-Z/Shift-Command-Z uses
bounded (100 edit) history; composition is one undo step. External model changes
reset history. Text storage stays UTF-8, with native UTF-16 conversion at the edge.
Password text is excluded from shaping caches, copy/cut, and accessibility value
and substring queries. Native secure input follows focus and application lifetime.
Secure storage is an ordinary Rust `String`, not a cryptographic vault.

`Draft<T>` owns editable text independently of a committed typed binding. Supply
parser and formatter closures; `commit()` accepts only valid input, `cancel()`
restores the model, and `error()` returns the current validation message. External
model changes supersede drafts. `Numeric<T>` supplies bounded numeric validation
and checked steps for i32/i64/u32/u64/f64; integer arithmetic never passes through
f64. `numeric_field("Copies", &number)` composes the editor and step buttons;
Enter or stepping commits. Invalid and incomplete drafts stay visible.

The initial editable grammar is ungrouped ASCII numbers (`.` for decimal values).
Grouping separators and localized digits are not accepted by these parsers. Use a
custom `Draft` parser/formatter when an application selects a localized grammar.
ICU4X localized integer display remains available independently.

`Host::debounce(key, delay, action)` replaces pending work under the same key.
Mounted windows schedule the earliest deadline as a one-shot redraw; detached
portable hosts use `run_due` (or `run_due_at` in deterministic tests). Cancel with
`cancel_deadline`. Tasks and callbacks run on the owning thread. Search is an
ordinary bound field; applications decide filtering and debounce policy.

## Stage 4 — scrolling and presentation

`cargo run --example stage4` demonstrates a scrolling workspace, tabs, a resizable
split ratio, anchored dropdown and action menus, and an alert. `embed(view)` inserts
an already-built description into `view!`; custom functions also compose directly.

`scroll { … }` clips a vertical document, clamps wheel movement at its edges, and
reveals focused descendants. Scroll offsets belong to mounted identity. Give the
scroll container a height or `grow: 1`; long documents do not expand its viewport.
`layer` stretches ordinary auto-sized children to its bounds and orders absolute
children by z-index. Headers/footers may use `shrink: 0` in constrained layouts.

Keep `Popup` in application state. `modal(title, body)`, `popover(anchor_key, body)`
and `menu(anchor_key, actions)` return descriptions for a top-level `layer`.
Popovers clamp to the viewport and flip above their anchor when needed. Escape and
outside clicks dismiss; Tab stays inside the topmost popup; dismissal restores
focus. Background accessibility elements and actions are unavailable while modal
content is mounted. Put overlays after their anchors in the same layer.

`dropdown` returns a button and its overlay so the button can live in a toolbar
while the overlay lives in the root layer. Menus support enabled actions, arrows,
Tab and activation. `on_context` handles a secondary click; `tooltip` displays
help after a 600ms one-shot deadline. `tabs` mounts the active page; `wizard` adds
step navigation; `split_button` combines a primary action with a menu anchor.
`split_view` uses an accessible slider for the pane ratio. These are Rust
compositions, and applications may build other presentations from the same pieces.

For an actual macOS sheet, attach components with `Dialog::set_view` and use the
existing `show_modal`/`dismiss` lifetime interface. `Popup::modal` is an in-window
presentation; it does not create an AppKit sheet or a new native window. Scrolling
is currently vertical; kinetic scrolling follows native wheel events, without a
second synthetic animation loop.

## Stage 5 — virtual collections and structured edits

`cargo run --example stage5` opens a 100,000-row table, an expandable tree, an editable
combo box and a selected-cell editor. Click headers to sort; arrows/Home/End/Page
keys move through a collection. The test fixture keeps fewer than 50 mounted nodes
while selecting and revealing the last of 100,000 rows.

`ListModel` stores keyed `ListItem` rows and selection. `view()` realizes the viewport
plus two rows of overscan; `view_with` customizes each realized row. Fixed positive
row heights make indexed geometry deterministic. `set_items` validates unique keys
before replacing data; selection follows keys through sorting/reordering and is
pruned on removal. Configure `.multiple()` before sharing a model; Shift/Command
activation adds/removes an individual selection. Models and mutations stay on the
owning thread, outside builders. Row descriptions should keep durable edit state
outside the virtual viewport.

`TreeModel` flattens expanded branches. Applications fetch lazy children and call
`set_tree`; `toggle` changes expansion. `TableModel` adds keyed cell drafts and
sorting: `begin_edit` snapshots the original cell, and `commit` rejects a removed
row/column or an intervening cell change. The example edits in a popup so scrolling
does not destroy a draft. `combo_box` preserves editable text beyond its suggested
options and limits suggestions to 100 matches.

macOS exposes logical row proxies with stable identifiers across scrolling/sorts.
Indexed `AXChildren`/`AXRows` queries realize only the requested range; offscreen
press/focus selects and reveals the row. Removal or unmount makes retained proxies
inert; reinserting a key gets a new lifetime token. Ordinary child queries return
the realized range. The indexed hooks follow Apple's
[array-range accessibility contract](https://developer.apple.com/documentation/applicationservices/1462060-axuielementcopyattributevalues).
Native protocol tests cover the final two rows of a 100,000-row collection and
retained proxies after detach. A full VoiceOver interaction audit remains separate.

## Stage 6 — typed values and developer tools

`cargo run --example stage6` shows sRGB RGBA channels/hex drafts, a calendar,
ISO date/time drafts, named-zone editing and a live inspector. `cargo run --example
lab` provides a small control catalog. Both support `--light`, `--render file.png`
and `--smoke`; render mode additionally accepts `--inspect` for bounds output.

`ColorPicker` and its `well` composition share a typed `Color` binding. Hex editing
accepts `#RRGGBB` or `#RRGGBBAA`; the stored u32 uses ARGB. `DatePicker` uses Gregorian
calendar values. Time/local datetime editing uses explicit ISO syntax. Jiff supplies
civil validation and a bundled timezone database in Rust, alongside ICU4X's Unicode
and localized integer services. The lockfile pins the data version; upgrading it
is how applications receive timezone-rule updates.

`ZonedPicker` accepts full zoned ISO strings with `[IANA/Zone]`, or local ISO text in
the current zone. The default rejects gaps and overlaps; users may choose earlier
or later resolution. Invalid drafts preserve the committed instant. This follows
[Jiff's explicit disambiguation model](https://docs.rs/jiff/0.2.39/jiff/tz/enum.Disambiguation.html).
These editors do not guess localized number/date grammars or convert civil dates
into instants implicitly. Color editing is sRGB; wide-gamut profiles are not exposed.

Create `Inspector` outside the builder, include `inspector.view()` if desired, and
attach it using `window.view_host().unwrap().set_inspector(inspector)`. Picking
outlines a real mounted node and reports its key, kind and bounds. Width overrides
are keyed by mounted identity and can be reset; invalid style values return layout
errors. Escape exits picking. `Host::pick` also works without the inspector panel.
Use `autofocus: true` on a control to focus it when the host has no valid focus;
a newly mounted autofocus control also takes focus, with later controls taking
precedence (useful for inline editing). `on_cancel` runs on Escape after any IME
composition has first been cancelled.
