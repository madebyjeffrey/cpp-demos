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
bounded history (100 checkpoints by default); composition is one undo step. External model changes
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
`cancel_deadline`. Tasks and callbacks run on the owning thread. Search fields
provide their own scoped query/debounce policy, described below.

## Stage 4 — scrolling and presentation

`cargo run --example stage4` demonstrates a scrolling workspace, tabs, a resizable
split ratio, anchored dropdown and action menus, and an alert. `embed(view)` inserts
an already-built description into `view!`; custom functions also compose directly.

`scroll { … }` defaults to vertical. `scroll_axes: ScrollAxes::Horizontal` or
`Both` enables horizontal documents. Offsets belong to mounted identity. Give the
container a height or `grow: 1`; long documents do not expand its viewport. Set a
child width/minimum width when it should exceed the horizontal viewport.
Visible overlay scrollbars support dragging and track paging; `scrollbars: false`
hides them. Wheel residuals pass to ancestors at each axis edge. Arrow/page/home/end
keys scroll when the focused control does not consume them (Shift selects the
horizontal axis for page/home/end). Focus and caret movement reveal through nested
viewports; an unchanged caret does not override explicit user scrolling.
`Host::scroll_info(id)` queries viewport/content/offset/limits; `scroll_to` and
`scroll_by` accept a `Point` and clamp both axes. `scroll_at(point, delta)` routes a
wheel delta; `scroll(point, delta_y)` is the vertical shorthand. Call layout after
changes before geometric queries. Drag selection beyond a viewport uses one-shot
deadlines while needed and cancels on release, blur, deactivation or unmount.
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
presentation; it does not create an AppKit sheet or a new native window. Kinetic
scrolling follows native wheel events without a second synthetic animation loop.

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


## Completion M1 — layout, typography and scrolling

`cargo run --example m1_layout` demonstrates the current work; it also supports
`--light`, `--render file.png` and resize. `cargo run --example m1_scroll` demonstrates
nested two-axis scrolling. M1 evidence is recorded in
[the checklist](../../docs/rust-completion-checklist.md); later milestones remain open.

`padding` and `margin` accept a scalar, `Insets::symmetric(vertical, horizontal)`,
or `[top, right, bottom, left]` values. `basis` controls the flex main-axis basis;
`align_self` overrides a parent's cross alignment, and `Justify::SpaceAround`
distributes remaining space. Flex items that reach min/max limits give remaining
space to eligible siblings. `Position::Relative` offsets the visual subtree while
preserving its original flow slot; percentages use the containing content box.

Typography properties include `label_style`, `text_wrap`, `text_overflow`,
`max_lines` (0 means unlimited), `text_align`, `uppercase`, `line_height` and
`letter_spacing`. Ellipsis with no wrapping defaults to one line. Uppercase affects
presentation text, not bound editor values. `TextWrap::Wrap` and `BreakWord` use
SkParagraph's Unicode line breaking, including fallback breaking of long words.
`accent` and `density` inherit through containers; minimum text height still limits
how small a compact control can be. Separators accept `orientation`.

Custom `Measure` implementations can use `TextStyle::default()` for added text
options. `Style.padding`/`margin` are now `Insets`; ordinary `.padding(24)` builder
and macro code remains concise. Width-dependent reflow propagates wrapping to
ancestors and following siblings; baseline alignment uses the shaped text baseline.

`rich_text(vec!["Read ".into(), Span::new("this").bold(true).into(),
View::button("Open", action).into()])` shapes spans and real inline controls together.
Spans inherit text styles and can override font size, color, weight, line height and
spacing. Inline views keep normal state, events and keys; ellipsis hides omitted
controls from input. Custom measurement providers implementing rich text must
provide `Measure::paragraph` and its `TextLayout.placeholders`; the default returns
an unsupported error. `TextLayout.baseline` is relative to the paragraph origin.


## Completion M2 — text model and navigation checkpoint

`cargo run --example m2_editor` demonstrates wrapping, readonly fields and a
12-grapheme limit in both themes. `cargo run -p scars-ui --example text_buffer`
demonstrates the same model without a window, native backend or Skia renderer.
The remaining search/password/numeric policies are tracked in the checklist.

`TextBuffer::new(text)` owns selection, composition and history. `select`, `insert`,
`replace`, `mark`/`unmark`, `undo`/`redo` and `set_text` work independently of a host.
`text()` includes preedit; `committed_text()` excludes it until acceptance. External
`set_text` clears history/composition and deliberately accepts the application's
full replacement even if it exceeds an input length limit. User edits enforce
`TextOptions`; a lowered limit never silently truncates an existing model value.
`max_length: 0` means unlimited. Limits count extended graphemes, including accents
and combined emoji. Readonly fields still allow selection/copy. `history_limit: 0`
or `undo_enabled: false` disables history; composition still works transactionally.

Typing and repeated deletion coalesce until a pause, selection or discrete action;
paste and composition are separate transactions. `insert_at` accepts an injected
`Instant` for deterministic runtimes. `TextBuffer::navigate` accepts pure
`TextLayout` geometry; `command` handles logical Unicode commands without geometry.
`Start`/`End` mean document boundaries; `LineStart`/`LineEnd` mean visual lines when
geometry is supplied. Word/paragraph deletion and transpose remain grapheme safe.
Up/down preserve the preferred horizontal position through shorter lines; page
movement uses the editor viewport. Double/triple clicks select words/visual lines,
then dragging extends whole units. Native adapters pass click counts through
`Host::pointer_click`; ordinary applications need no pointer bookkeeping.

Custom `Measure` providers supply `TextLayout.lines` for visual navigation (an empty
list retains logical-only behavior). `TextLine` uses UTF-8 ranges and paragraph-local
geometry. `caret_rect(index, affinity)` distinguishes both sides of a soft wrap;
newline bytes belong to `end_including_newline`, not the visible line range.


## Completion M2 — search and passwords

`cargo run --example m2_fields` demonstrates both (`--light`, `--render file.png`,
`--smoke`). Ordinary macro/builder usage stays compact:

```rust,ignore
search_field("Find…", bind: query, debounce: Duration::from_millis(250),
    on_query: results.handler(|state, query: String| state.set(query)));
password_field("Password", bind: password, allow_reveal: true, max_length: 24);
```

`State::handler` is the payload-taking counterpart to `State::action`. Search's
`on_query` receives accepted committed edits, including composition commits. Zero
debounce dispatches on the next one-shot task turn (`run_due` on detached hosts).
External model replacement cancels pending work and does not generate a query.
`clear_button: false` hides the default clear action. Clear/Escape notify an empty
query immediately on the next task turn; `on_cancel` handles Escape. Enter flushes
only that field's pending query before `on_submit`. Removal/replacement, hiding,
disabling, readonly, blur/deactivation and native unmount cancel pending queries;
callbacks run without host borrows and may remove the field safely.

Passwords default to no reveal and no undo; opt in with `allow_reveal` and
`undo_enabled`. Readonly/max-length policies are shared with ordinary fields.
Reveal has a real accessible button and `Host::set_password_revealed` supports
explicit application commands. Escape and focus loss/deactivation hide it again.
Revealed text remains sensitive: copy/cut, native substring and AX value queries
stay blocked. Secure-input ownership continues while an editable password owns
focus, including during reveal. Shaping/painting revealed text bypasses the
renderer paragraph cache. Custom `Measure` providers must implement
`sensitive_text` without retaining plaintext to support reveal; the default
returns an explicit unsupported error.

Run `SCARS_WINDOW_SECURE_INPUT_TESTS=1 cargo test --test secure_input` on a
foreground desktop to exercise actual Carbon ownership transitions. The normal
unit test covers successful ownership, failed enables/disables and nested owners;
the native harness verifies readonly/disable/focus/hide/deactivation/unmount/
replacement/close without silently skipping unavailable activation.
