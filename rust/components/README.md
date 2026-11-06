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
Blur, Enter or stepping applies valid edits by default. Invalid and incomplete drafts stay visible.

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
help after a configurable one-shot deadline. `tabs` retains visited keyed pages;
`wizard` adapts positional steps, while `Wizard` provides validated stable-ID
navigation; `split_button` combines a primary action with a menu anchor.
`split_view` uses a dedicated adjustable divider and retained panes. `Alert` supplies
a single-use result transaction with default/cancel/destructive actions.

For an actual macOS sheet, use `surface.show_sheet(alert, body_builder)`. It retains
the native dialog until completion. `Dialog::set_view` with `show_modal`/`dismiss`
remains available for lower-level ownership. `Popup::modal` is an in-window
presentation; it does not create an AppKit sheet or a new native window. Kinetic
scrolling follows native wheel events without a second synthetic animation loop.

## Stage 5 — virtual collections and structured edits

`cargo run --example stage5` opens a 100,000-row table, an expandable tree, an editable
combo box and a selected-cell editor. Click headers to sort; arrows/Home/End/Page
keys move through a collection. The test fixture keeps fewer than 50 mounted nodes
while selecting and revealing the last of 100,000 rows.

`ListModel` stores keyed `ListItem` rows, selection, a range anchor and a logical
cursor. `view()` realizes the viewport plus configurable overscan; `view_with`
customizes each realized row. Fixed positive row heights make indexed geometry
predictable. `set_items` validates unique keys before replacement, retains selected
keys/cursor/anchor through reordering, and prunes removed or disabled selections.
Configure `.multiple()` or `ListOptions::selection`; shared clones see option changes.
Shift extends a range; Command toggles one row; Command-Shift adds a range. Arrows
skip disabled rows, Home/End jump to enabled edges and Page keys use viewport rows.
Return/double-click activate through `on_activate`; ordinary selection does not.
Models and mutations stay outside builders. Hold a `list.pin(key)` handle through
an active edit to retain its host state while scrolling. Durable application
values and drafts stay outside builders.

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
Search/password and numeric policies are demonstrated by `m2_fields` and
`m2_numbers`; the full evidence is recorded in the completion checklist.

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


## Completion M2 — numeric fields and steppers

`cargo run --example m2_numbers` demonstrates the final M2 policies in both themes.
Store a `Numeric::new(value.binding(), min, max, positive_step)` alongside your
state, then use `numeric_field("Count", &number)` or standalone
`stepper("Count", &number)`. Steppers have one tab stop, arrow-key adjustment and
native incrementor semantics; integer values/bounds never pass through `f64`.

Builders include `significant_digits(1..=17)` (decimal display only), `read_only`,
`commit_on_blur`, `repeat(Repeat { delay, interval })`, `on_validation`, `on_submit`
and `on_cancel`. Default decimal formatting preserves a round trip; integer
formatting always stays exact. Precision does not round an untouched value on
blur/submit/step. Nonrepresentable rounded decimal displays fall back to a finite
round-trip representation. Source values outside configured bounds remain invalid
rather than becoming valid through rounded display text.

`validation()` returns status/message/dirty state. Partial signs/decimal exponents
are `Incomplete`, malformed/nonfinite syntax is `Invalid`, and bounds/integer
overflow are `OutOfRange`. Invalid/incomplete drafts survive failed commit/step;
rejected binding writes retain their edits. Enter applies a valid draft and calls
`on_submit` only after acceptance. Escape restores the current source and invokes
`on_cancel`. Blur applies valid drafts by default; `commit_on_blur(false)` retains
them until explicit submit or stepping. Application source replacement supersedes
a draft even if the formatted source text is unchanged. `Draft<T>` now requires
`PartialEq` for that comparison; getters/formatters remain pure.

Readonly models reject edits, commits and stepping. `can_step` checks draft validity
and bounds; `step` reports an accepted change. Checked overflow clamps at the
configured bound. `Repeat` defaults to a 400ms delay and 80ms interval. Press activates once,
then deadlines repeat without catch-up bursts. Release, cancel, leaving the button,
focus loss, hide/disable/removal and unmount cancel repeats; no idle animation is
needed. `on_blur` and `repeat` also work on ordinary builder/macro controls (repeat
requires a button and a nonzero interval). Call layout before geometric input on
a detached host, as usual.


## Completion M3 — split views

`cargo run --example m3_split` demonstrates nested horizontal/vertical splits.
Store `Split::new(0.35)` and use `split(&model, first, second)` in `view!`, or
`model.view(first, second)` from ordinary Rust. `Split::bound(ratio.binding())`
connects an existing source; the older `split_view(&ratio, first, second)` call
keeps working with mounted collapse state. Macro calls resolve through the crate,
so a local model named `split` does not shadow the built-in component.

Builders: `orientation`, `limits(first_min, second_min)`, `divider_width`,
`keyboard_step`, `collapsed(binding)` and `options(SplitOptions { .. })`.
Orientation is the axis along which panes are arranged. `ratio`/`set_ratio` query
or set the preferred fraction; `collapse(SplitPane::First/Second)` and `restore`
change pane visibility without losing that fraction. Pane minima constrain actual
geometry. If both cannot fit, minima scale proportionally; resizing alone never
rewrites the preferred fraction. `Host::split_info` reports effective pane/divider
rectangles and limits after layout; `resize_split`, `set_split_ratio` and
`collapse_split` support host-driven commands, including the legacy helper.

Drag the divider; focus it with Tab and use arrows along the split axis. Shift
moves ten keyboard steps, Home/End reach pane limits, and Enter/Space or a double
click toggles the smaller pane. Escape restores a collapsed split; during a drag
it restores the starting ratio/collapse state. Pointer cancellation stops at the
last accepted position. Focus loss, deactivation, hide/disable/removal/unmount,
orientation changes and external ratio/collapse changes cancel capture.

Collapsed panes stay mounted with hidden, inert descendants. Editor history,
selection and scroll offsets survive restoration; preedit still present when
layout hides a pane is cancelled. Focus in the hidden pane moves to the divider.
Native split groups expose splitters and previous/next pane contents, orientation,
value/range, resize and collapse actions. This protocol coverage does not replace
the later interactive VoiceOver audit.

## Completion M3 — menu trees and split actions

`cargo run --example m3_menu` demonstrates nested menus and retained primary actions.
Create `Menu`/`SplitAction` once, then describe the current items in each builder:

```rust
let menu = Menu::new();
let checked = State::new(true);
// Inside the view builder, place trigger and overlay in the same layer:
View::layer(vec![
    menu.button("Document ▾", "document"),
    menu.view("document", vec![
        MenuAction::new("Save", || {}).key("save").shortcut("⌘S"),
        MenuAction::separator(),
        MenuAction::check("Show guides", checked.binding()),
        MenuAction::submenu("Quality", vec![MenuAction::new("Best", || {})]),
    ]),
])
```

`radio(label, binding, choice)` selects a string choice. `enabled(false)` excludes
an item from keyboard/typeahead navigation and activation. Keys default to labels;
provide stable keys for changing labels. Keys are unique among siblings and cannot
start with `$`. Shortcut strings are display hints; applications register actual
accelerators in their window menus. The existing `Popup::menu` helper uses this
same menu implementation.

Up/Down wrap within the current level, Home/End choose its first/last enabled item,
Right/Enter open submenus, Left/Escape return, and Escape at the root or Tab dismiss.
Printable input searches a case-insensitive prefix; repeating a letter cycles
matches, and a one-second pause resets it. This creates no editor or idle timer.
Hover opens a branch; click or drag-release activates a leaf. Outside presses,
pointer cancellation, deactivation and loss of the anchor dismiss. Leaf activation
closes before invoking application code. Menus share one focus scope and restore
the previous control on dismissal. Panels flip/fit inside the viewport and scroll
long lists; keyboard focus reveals offscreen items.

`SplitAction::view(key, items)` returns `(button, overlay)`. Selecting an enabled
leaf remembers its key path and updates the primary button. If it becomes disabled
or disappears, the button uses the first enabled leaf; restoring that key restores
the retained choice. With no available leaf the primary button is disabled.
`selected()` reports the retained path. `split_button` still supports a fixed
primary callback. Native accessibility exposes menu/item roles, checked values,
shortcut help and expanded branches; final interactive VoiceOver auditing is M5.

## Completion M3 — overlay policy

`cargo run --example m3_overlay` demonstrates placement, nested scopes, retained
editing, configurable help and attached context menus. Models stay outside builders:

```rust
let popup = Popup::new()
    .options(OverlayOptions {
        placement: OverlayPlacement::Right,
        width: Some(300.), max_height: 240., margin: 8.,
        ..Default::default()
    })
    .on_dismiss(|reason| println!("Closed: {reason:?}"));
// In the builder:
View::layer(vec![
    popup.button("Details…", "details"),
    popup.popover("details", View::label("Popover content")),
])
```

`OverlayOptions` selects Below/Above/Left/Right, intrinsic or explicit width,
maximum height, gap and viewport margin. Placement flips to the opposite side
when it offers more room, then clamps. Popover bodies scroll inside their fitted
size. Portals paint above regular content and escape owner clipping. Focusing an
overlay control never scrolls the owner's ancestors. Fully clipped, hidden,
disabled or removed owners dismiss the presentation without scrolling back.

Content focus is the default and traps Tab in the top scope. `OverlayFocus::Preserve`
keeps the invoking control/editor focused; explicitly focusing a control inside is
still allowed. Closing nested content restores its prior control, then closing the
parent restores the original one. Closing a parent also clears descendant models.
Escape cancels marked text before dismissing an overlay. Outside presses consume
the entire click. Outside/Escape/deactivation dismissal can each be disabled;
`dismiss_on_deactivate: false` keeps content for reactivation. Inactive hosts do
not regain focus through dismissal.

`on_dismiss` receives a `DismissReason` once per observed presentation: action,
Escape, outside press, programmatic close, owner loss, deactivation or unmount.
Callbacks run outside construction and host borrows. They can remove the owner or
reopen the model. `dismiss(); show()` starts a fresh presentation even when the
changes coalesce before rebuilding. Raw `View::modal`/`popover` use the same lifetime
and scope machinery; `overlay_options` configures raw popover placement and
dismissal policy. Dialog sizing and completion transactions are covered separately.

Attach a context menu with `view.context_menu(&menu, items)`; no separate overlay
is needed. Secondary-click, Control-click and Shift-F10 on the owner or a descendant
invoke its nearest menu. `Host::show_context_menu(id, point)` also supports explicit
invocation; `None` places it by the owner. `Menu::options`/`on_dismiss` configure its
presentation and `show_at` supplies a pointer position. Keys applied after attachment
remain valid because attached menus track the owner node directly.

`tooltip(text).tooltip_options(TooltipOptions { .. })` configures delay, maximum
width, placement, gap and margin. Help works on hover or keyboard focus, including
noninteractive labels. Click/Escape suppress help until focus or hover changes;
it never adds a focus stop. `on_tooltip_dismiss` reports closure after visible help.
Owner loss/deactivation/unmount cancel it; pending and shown help settle without
continuous frames. `advance_overlays_at` supports deterministic clocks and
`tooltip_presentation` exposes fitted logical-point geometry. Native accessibility
exposes help before the tooltip appears. Final interactive auditing remains M5.


## Completion M3 — retained tabs

`cargo run --example m3_tabs` demonstrates editing, scrolling and returning to a page.
Create the selection model outside the builder; page IDs remain stable across reorders:

```rust
let pages = Tabs::new("draft");
// In the builder:
view! { tab_view(&pages, vec![
    Tab::new("draft", "Draft", View::label("Editor content")),
    Tab::new("preview", "Preview", View::label("Preview content")),
    Tab::new("activity", "Activity", View::label("Activity")).enabled(false),
], grow: 1); }
```

`Tabs::bound(binding)` accepts application-owned selection. `select` changes the
requested ID, and `selected` reads it. `Host::tab_info` reports requested/effective
selection, enabled items and visited count. A missing/disabled request displays the
first enabled page without rewriting the binding. Empty/all-disabled groups have no
active page. IDs must be nonempty and unique; labels can change independently.

Visited pages keep mounted control IDs, editor undo/selection, scroll offsets and
remembered focus. Inactive content has zero geometry and is unavailable to input or
accessibility. Hidden search/repeat/help work cancels and page-owned popovers close.
External model updates still refresh retained controls. Removing a page releases
its host state; reintroducing that ID constructs fresh controls. Reordering keeps it.
`Tab::lazy(id, label, || content)` defers a heavy page until first selection, then
rebuilds it with other visited pages under the normal pure-builder contract. A failed
build leaves the previous tree and visited cache intact. `Tab::new` takes an ordinary
view; the existing tuple-based `tabs(&state, pages)` helper now retains pages too.

The strip is one Tab stop. Arrows/Home/End activate enabled headers, skipping disabled
ones; Tab enters the active page's remembered control (or first control). Control-Tab
and Control-Shift-Tab cycle the nearest containing group, including from native
editors. Pointer/keyboard activation focuses the selected header. A programmatic
switch restores page focus when the group already owned it, and otherwise leaves
outside focus alone; inactive windows defer restoration until reactivation.
`Host::focus_tab_page` explicitly restores page focus. Overflowing strips accept
horizontal scrolling, reveal selected/focused headers and refit after resize or reorder.

Explicit activation commits preedit and rechecks availability after the commit's
application callbacks. External selection changes cancel inactive preedit. Selection
setters may reject/normalize requests; the displayed selection follows the accepted
value. macOS exposes tab-group/radio-tab roles, selected children, page contents,
values and disabled actions. Final interactive VoiceOver/IME auditing remains M5.


## Completion M3 — validated wizards

`cargo run --example m3_wizard` demonstrates a complete setup/review flow. Create
state and the model outside builders, then describe page content by stable ID:

```rust
let name = State::new(String::new());
let checked_name = name.clone();
let setup = Wizard::new("name", vec![
    WizardStep::new("name", "Identity").validate(move || {
        if checked_name.get().trim().is_empty() { Err("Enter a name.".into()) }
        else { Ok(()) }
    }),
    WizardStep::new("review", "Review"),
])?.on_finish(|| println!("Submitted"));
// In the builder:
view! { wizard_view(&setup, vec![
    ("name", view! { text_field("Name", bind: name); }),
    ("review", View::label(format!("Create {}", name.get()))),
], grow: 1); }
```

`next`, `back`, `go_to`, `finish` and `cancel` work independently of a window.
`info` exposes requested/effective selection, index, step metadata, validation
issue and available actions. `Wizard::bound` accepts an application binding;
`on_change` reports its actual accepted from/to IDs, including normalized values.
Rejected writes and no-ops emit no change. Reentrant navigation from a callback is
ignored. Finish/cancel callbacks leave form data, reset and presentation lifetime
to the application; register a Finish handler to enable the final action.

Validators are pure and reactive `Result<(), String>` queries. State writes are
rejected, including outside a host; dependencies still subscribe the outer builder.
Forward navigation validates the current step. Finish validates all steps by default
and identifies the first invalid one. `WizardOptions` independently controls Back,
direct review of earlier steps, final validation scope, step/status visibility and
accessible action labels. Direct forward skipping is unavailable. Back/review and
Cancel remain independent of forward validation. Options return an error for blank
action labels.

`set_steps` atomically replaces definitions after validating unique nonempty IDs.
Reorder/title changes preserve the selected ID and retained page controls. Missing
selection falls back to the first step without writing during rendering. Empty sets
have no current step and disabled navigation, with a valid "No steps" presentation.
Body descriptions for removed steps are ignored; omitted bodies are empty. Duplicate
or empty body IDs fail host construction. `navigation()`/`wizard_navigation(&setup)`
provide controls alone for application-arranged content.

`view`/`wizard_view` retain visited page controls using the same page machinery as
tabs, without exposing a tab strip or Control-Tab bypass. Editor history/selection,
scroll and focus survive Back/review; inactive work and overlays cancel normally.
Explicit navigation commits current preedit before rechecking availability/validation.
Old Next/Back elements become inert across step/definition changes; captured actions
also check live step identity and validation when bindings are not reactive. Cancel
uses normal button editing-commit behavior; applications own discarding form data.
The legacy `wizard(&index_state, pages)` helper remains available, including empty
page lists, but positional identity is intended for fixed step sets. Native protocol
checks cover validation feedback, enabled actions, IME commit, callbacks and stale
accessibility objects. Interactive VoiceOver/IME auditing remains M5.


## Completion M3 — alert transactions and native sheets

`cargo run --example m3_alerts` shows a component alert; add `--sheet` for a native
parent-modal sheet. Both accept `--light`, `--smoke` and `--render file.png` (sheet rendering previews
the same component content used inside the native sheet).

```rust
use scars_ui::{Alert, AlertAction, AlertActionRole, View};
let alert = Alert::new("Save changes?", vec![
    AlertAction::new("cancel", "Cancel").role(AlertActionRole::Cancel),
    AlertAction::new("save", "Save").role(AlertActionRole::Default),
], |result| println!("{:?}", result))?;
// Retain alert across rebuilds; embed this description in the root layer.
let view = alert.view(View::label("Your document has unsaved changes."));
```

Each `Alert` is one transaction. Clones share identity and `result()`; equality
compares transaction identity, allowing `State<Option<Alert>>`. Create a new alert
for the next presentation. IDs and labels must be nonempty; IDs must be unique,
with at most one default and one cancel action. Actions and their enabled state
are fixed for that transaction. Empty action sets can still dismiss or escape.

`choose(id)` accepts an enabled action; `cancel()` and Escape choose the enabled
cancel action if present, otherwise complete without an action. `dismiss()` has
no action. `AlertResult` includes the action ID and `AlertReason`: action, cancel,
Escape, dismissal, close, owner removal, unmount or destruction. `completed()`
distinguishes pending from a completed result without an action. Completion runs
once, after publishing the result, outside host borrows. Reentrant calls, duplicate
closes and stale controls are inert. Last-handle destruction resolves a pending
transaction with `Destroyed`; removing mounted content/unmounting resolves its
observed presentation. Avoid capturing the alert strongly inside its own callback.

Return invokes only the enabled default action; it never falls through to an
arbitrary destructive/cancel control. Space activates the focused button. Return
commits marked text first without submitting; multiline editors retain Return for
newlines. Escape cancels preedit first, then cancels the alert on the next press.
Action activation commits preedit; the cancel button discards it. Alerts preserve
their transaction on outside clicks and deactivation. Modal scope traps focus and
restores the invoking control after removal/layout. Default and destructive actions
have distinct visuals; native accessibility exposes modal/default/cancel controls.

On macOS, `surface.show_sheet(alert, || body)` supplies the same component content
inside an application-owned native sheet and returns a `Surface` control handle.
Discarding that handle keeps the sheet open. Completion releases the native sheet
and restores parent focus before invoking application code, preserving another
window's key status when a background sheet completes. Other windows remain
usable; the parent blocks menu commands, text input and retained accessibility
actions. Close, parent closure/destruction, application teardown and component
unmount resolve the same transaction. `sheet.request_close()` honors close policy;
`close()` forces closure. `hide()` is unavailable while modal. The returned surface
can be resized; the initial size is 460×300, with a 300×160 minimum.

`WindowBuilder::view_with_surface(|surface| ...)` and
`Surface::set_view_with_surface` supply window context for component actions such
as opening sheets, without application-managed weak captures. `AlertSubscription`
and `observe_completion` support presentation adapters; subscriptions do not retain
the alert, and completed transactions notify new observers immediately. Adapter
observers run before application completion.

Checks: seven portable transaction/input/lifetime tests and
`SCARS_WINDOW_DESKTOP_TESTS=1 cargo test --test sheets` cover native actions,
accessibility roles, marked text, isolation, focus restoration and closure paths.
`cargo test --test sheets -- --foreground` additionally requires another native
window to become key and verifies that background completion preserves it. This
foreground check is explicit and is not counted when unavailable. Interactive
VoiceOver/IME and visible GPU auditing remain in M5/M8.


## Completion M4 — list interaction

`cargo run --example m4_list` demonstrates 100,000 rows, multiple selection,
activation, replacement/reversal and overscan configuration. It supports `--light`,
`--render file.png` and `--smoke`.

```rust
use scars_ui::{ListItem, ListModel, ListSelection};
let list = ListModel::new(vec![
    ListItem::new("one", "First document"),
    ListItem::new("two", "Second document"),
], 32.).multiple().on_activate(|key| println!("Open {key}"));
list.select("one", false);
list.select_with("two", ListSelection::Range);
let view = list.view();
```

`ListSelection::{Replace, Toggle, Range, AddRange, Focus}` makes programmatic
selection explicit. `select(key, extend)` remains a compatibility shorthand:
`false` replaces, `true` toggles additively. Single-selection mode turns selection
operations into replacement; `Focus` only moves the cursor. `selected()` returns a
snapshot; `is_selected` and `selection_count` query without cloning the set.
`clear_selection` keeps the cursor/anchor; `select_all` selects enabled rows in
multiple mode. A range follows the anchor's logical key through reordered data and
skips disabled rows. Toggle/replacement establish a new anchor; ranges preserve it.
Changing to single mode keeps the selected cursor, otherwise one existing selection.
Removed/disabled cursors fall back to a nearby enabled item when data is replaced.
Validation failure leaves data, tokens and selection intact.

`ListOptions` configures selection mode, preferred `visible_rows` (five by default),
`overscan_rows` (two) and positive `typeahead_timeout` (one second). `options` and
`set_options` validate before applying. Preferred height is rows × row height;
ordinary height/grow modifiers and viewport constraints still apply. `visible_range`
returns the realized range including overscan. Realization is bounded by viewport
rows plus twice the configured overscan, clamped to the data extent.

A list has one row Tab stop. Arrows/Home/End/Page reveal logical targets; Command
or Control movement preserves selection, Shift extends it and Command/Control-Shift
adds a range. Command/Control-A selects all enabled rows. Space selects the focused
row (Command/Control-Space toggles); Return invokes activation once per key press.
Double-click activates after selection. Disabled rows cannot select or activate.
Activation callbacks run outside model/host borrows and may replace the data;
recursive activation is ignored. Right-click preserves an existing selection or
selects an unselected target before its context action.

Printable native keys use Unicode lowercase prefix typeahead, skipping disabled
rows, cycling on a repeated prefix and resetting after the configured timeout.
`Host::collection_text`/`collection_text_at` expose the same behavior to portable
adapters and deterministic tests. Embedded editors retain their ordinary text
input. Navigation uses `select_collection_item_with` to select/reveal/focus;
`select_collection_item` retains the boolean compatibility shorthand. Reordering
keeps focused logical keys visible. Wheel scrolling can unrealize the focused row
without snapping back: focus remains in the collection and the next navigation
reveals its logical target. Explicit active editor retention is described below.

Seven focused portable tests cover selection, pointer/keyboard routing, typeahead,
reordering, callback replacement and bounded 100,000-row realization. Native tests
cover Shift-arrow skipping, typeahead, activation, Select All, indexed offscreen
proxies and proxy identity after reordering. Final interactive auditing remains M5.


## Completion M4 — virtual edit lifetimes

`cargo run --example m4_list_edit` demonstrates one active edit across 100,000 rows.
Use First/Last rows and Back to edit to move away and return; undo and selection
stay with the pinned editor. Save/Cancel release it. The example supports `--light`,
`--render file.png` and `--smoke`.

`let edit = list.pin(key)?` returns a cloneable `RowPin`. Keep it while the row is
being edited; dropping the last clone releases its retention slot. Each distinct
row consumes one slot, including pins currently onscreen. `ListOptions::max_pinned_rows`
defaults to one; pinning another row returns an error until a slot is released.
Increasing the limit supports simultaneous edits; lowering it below the active
pin count fails without changing options. Duplicate pins/clones share one slot.

Pinned rows are realized alongside the viewport/overscan indices, with spacer
geometry preserving their logical positions and the full scroll extent. A pin
keeps the row's normally described, keyed descendants mounted; it does not freeze
a description. Text selection, preedit, bounded undo/redo history and focus survive
scrolling offscreen and back. Changing a descendant's key/kind or externally
replacing its value still follows ordinary control reset rules. Typing/focusing
reveals the caret; merely scrolling does not pull the row back into view.
`visible_range()` remains the viewport/overscan range; `realized_indices()` includes
pins and `pinned_keys()` reports retained logical keys. Realization is bounded by
viewport/overscan rows plus the configured maximum pin count.

`RowPin::key`, `token` and `is_valid` identify/check the edited item. Removal or
disabling invalidates its pins immediately. Reinserting/re-enabling does not revive
an old pin, and dropping an old handle cannot release a newer edit. Pins use weak
model ownership, so they do not keep a discarded dataset alive. Creation remains
outside view construction; last-handle cleanup safely defers during description
replacement and runs on builder unwinding too. A pin retains controls; application
code owns commit/cancel and source conflict policy.

Every realized row's host identity includes its item lifetime token. Sorting and
ordinary data replacement preserve tokens; removing and reinserting a key creates
a new one, even when both mutations occur before the next frame. Descendant editor
IDs, undo buffers, focus/capture and native accessibility objects follow that
lifetime. A different `ListModel` cannot reuse an old editor with the same key.
After releasing an offscreen pin, its controls and host editor storage are pruned;
logical keyboard navigation remains in the collection. Native accessibility caches
also prune during synchronization when no drawing frame is delivered.

`on_activate_with_model(|list, key| ...)` supplies model context for starting an edit
without a strong self-capture in the activation callback. Retain the returned pin
in application state, describe its editor with `view_with`, then release it from
Save/Cancel. The example keeps this ownership inside ordinary `State<Option<RowPin>>`.

Evidence: eight portable lifetime/input tests and a 200-jump, 100,000-row test that
inspects actual host editor storage. Native checks cover offscreen preedit,
commit/undo and inert retained accessibility editors after coalesced reinsertion.
The actual VoiceOver/IME candidate-placement and visible GPU audits remain M5/M8.
