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

`TreeNode` describes eager/lazy/deferred branches; `TreeModel` validates all known
keys and publishes loading/error metadata through checked requests. `TreeItem` is
a flat metadata query, including hidden known nodes. `expand`/`collapse`/`toggle`
return results instead of discarding failures. `TableModel` owns keyed rows/columns,
checked widths/order/sort and row/cell selection. Its external cell drafts reject
retired row/column lifetimes or an intervening cell change. The example edits in a popup so scrolling
does not destroy a draft. `ComboBox` owns separate query/selection and a virtual
suggestion popup; `combo.view(placeholder)` needs no separate portal wiring.

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

`ColorPicker` and its `well` composition share a typed `Color` binding and one
RGB/HSV/hex session. Transactional editing is the default; live preview is explicit
and cancellation rolls back only owned previews. Hex accepts `#RRGGBB`/`#RRGGBBAA`;
the stored u32 uses straight ARGB. See M6.5 below. `DatePicker` uses Gregorian
calendar values. Time/local datetime editing uses explicit ISO syntax. Jiff supplies
civil validation and a bundled timezone database in Rust, alongside ICU4X's Unicode
and localized integer services. The lockfile pins the data version; upgrading it
is how applications receive timezone-rule updates.

`ZonedPicker::new(instant.binding(), "America/Edmonton")?` binds `Timestamp` and
owns an explicit display zone. Its civil field shares the datetime grammar; zone
changes separately choose KeepInstant or KeepLocal. Gaps reject; repeated wall times
require an explicit policy, while existing folds retain their known instant.
Invalid drafts preserve the committed instant. See M6.4 below for the full contract.
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


## Completion M4 — lazy tree models

`cargo run --example m4_tree` demonstrates eager branches, a cached lazy loader,
visible failure/retry, deferred completion and replacement that retires outstanding
requests. It supports `--light`, `--render file.png` and `--smoke`.

```rust
use scars_ui::{TreeModel, TreeNode};
let tree = TreeModel::from_nodes(vec![
    TreeNode::new("workspace", "Workspace").children(vec![
        TreeNode::new("local", "Local").children(vec![TreeNode::new("notes", "Notes.md")]),
        TreeNode::new("remote", "Remote").lazy(|| Ok(vec![TreeNode::new("roadmap", "Roadmap.md")])),
    ]),
])?;
tree.expand("workspace")?;
```

`TreeNode` is the recursive input; `TreeItem` is flat, queried metadata. This replaces
the earlier recursive `TreeItem` stub. `from_nodes`/`set_nodes` take owned input;
`set_tree(&[TreeNode])` is a borrowed convenience. Every known key, including eager
children of collapsed branches, must be nonempty/non-whitespace and globally unique.
Validation failure leaves the existing snapshot and rows intact. Traversal and
metadata indexing are iterative; the tests include a 2,501-node collapsed chain.
`known_count`, `visible_count`, `flatten`, `item(key)` and `expanded` expose state
without mutating it. The old public `expanded`/`list` fields are replaced by these
queries and checked expansion methods, keeping rows synchronized with metadata.

`TreeItem` reports parent, depth, effective enabled state, expandability, expansion,
loading/error state, child count and accessible label. Disabled ancestors disable
known descendants. `TreeLoadState::{Unloaded, Loading, Loaded, Failed}` separates
not-yet-loaded from an empty completed branch. Empty successful loads become leaves
unless explicitly marked expandable. `loading(true)` represents an already-running
external request; `failed(message)` seeds a known failure. Accessible labels default
to the visible label.

`lazy` is a synchronous pure snapshot query, invoked on first expansion and cached
across collapse/reopen. It must not mutate reactive/model state. Expected errors and
loader panics become visible `Failed` metadata and return `Err`; invalid returned
keys fail the load without installing partial children. `last_failure` reports the
most recent operation failure. Reopening a failed branch returns its cached error;
`retry` explicitly starts another attempt. Views render failure text and a retry
control. Building/querying a tree never invokes loaders.

For deferred work, use `TreeNode::deferred()` and `tree.on_load(|request| ...)`.
Expansion marks Loading before invoking the callback outside model borrows. Call
`request.complete(Ok(children))` or `complete(Err(error))` on the owning UI thread.
`TreeLoad` is cloneable, holds weak model ownership and is intentionally not Send;
background tasks can return plain data to the UI thread for conversion/completion.
The model does not supply an executor. `pending_load(key)` retrieves a current
request, and `begin_load(key)` starts/supersedes one for manual dispatch. Dropping a
handle does not cancel it; `pending_load` can retrieve it again.

Requests include node-lifetime and request generations. Stale/duplicate completion
returns `Ok(false)` and changes nothing; an accepted failure returns `Err` and records
its message. Collapse keeps an in-flight request valid so completion can cache
children without reopening the branch. Retry supersedes an earlier request.
Existing children stay available during a refresh or failure. `set_children` starts
a checked branch replacement; successful publication replaces that subtree and
retires its old requests/lifetimes. Keys may be reused within that replaced subtree,
but cannot collide with another known branch, including a collapsed one.

`set_nodes` replaces the entire validated snapshot with fresh node lifetimes,
retiring requests and lazy caches; new branches start collapsed. Use checked branch
completion to update children while preserving other branches. Visible list rows
use tree item tokens, so cached host/native objects cannot act on a replacement that
reuses the same key. Related metadata/row updates publish before external scheduling;
old loader resources drop outside state borrows and may safely query new state.

Eleven focused tests cover metadata, hidden-key validation, caching/empty results,
failures/retry, read-only queries, deferred completion, replacement generations,
reentrant callbacks, scheduler consistency, disclosure rendering and resource teardown.
Native checks cover retired row proxies after snapshot replacement. Interaction is
covered below; hierarchical native semantics and the actual interactive audit remain M5.

## Completion M4 — tree interaction

`TreeModel::with_options(nodes, TreeOptions { selection: ListSelectionMode::Multiple,
..Default::default() })` configures selection, row height, visible rows, overscan
and indentation. Geometry must be finite and valid. The `m4_tree` example adds
keyboard instructions, selection/cursor status and activation/expansion callbacks.

A tree has one row Tab stop. Up/Down, Home/End, Page keys and typeahead use the list
navigation rules. Right opens a closed branch or enters its first enabled child;
Left closes an open branch or visits an enabled ancestor. Shift extends the range,
Command/Control moves the cursor without replacing selection, and combined modifiers
add a range. Disclosure and Retry remain distinct pointer actions, both focusing
their row. Disabled nodes and descendants cannot be selected or expanded.

`select_with`, `selected`, `selection_count`, `cursor`, `clear_selection` and
`select_all` expose the shared selection rules. Collapse maps selected descendants,
the cursor and range anchor to their nearest visible enabled ancestor. Unrelated
selection survives; reopening does not restore hidden selection. Removal uses the
same ancestor fallback; if none survives, the cursor moves to a nearby enabled row
without implicitly selecting it. Empty trees clear the cursor. Replacement preserves
surviving logical keys but gives new nodes fresh host/native lifetimes. Changes do
not pull focus back from another control.

`reveal(key)` opens known ancestors and scrolls the item into view without selecting
it. It validates item lifetimes across callbacks and reports loading/expansion errors.
It cannot discover an unknown key inside an unloaded branch.

`on_activate(|key| ...)` handles Return/double-click; `on_activate_with_model` supplies
model context without a strong self-capture. A custom handler owns activation. Without
one, branches toggle, failed branches retry, and leaves have no default action.
Repeated Return does not repeatedly activate. `on_expand` receives `TreeExpansion {
key, expanded }` after a current transition publishes; snapshot replacement does not
emit disclosure events. Callbacks run outside borrows and can change the model.
Nested notifications are suppressed to prevent recursive callback loops.

Eight focused interaction tests cover keyboard/selection/focus retention, model
replacement inside callbacks, deferred/error states and bounded 100,000-child
navigation. Native protocol tests exercise actual AppKit keys and focused AX objects.
Hierarchical/offscreen native tree semantics and interactive VoiceOver auditing remain M5.


## Completion M4 — combo boxes

```rust
let choice = ComboBox::new(vec![
    OptionItem::new("local", "Local workspace"),
    OptionItem::new("team", "Team workspace"),
])?.on_accept(|accepted| println!("{:?}: {}", accepted.selection, accepted.text));
// Inside view!:
// embed(choice.view("Find a workspace"));
```

`ComboBox::with_options` accepts `ComboBoxOptions`: Selection/FreeText mode,
Contains/Prefix/None filtering, auto-highlight, readonly/loading states,
grapheme-counted draft length, row/popup heights and loading/empty text. IDs must
be nonempty and unique; geometry/extents must be finite and positive. Filtering
uses Unicode lowercase labels; it is not locale-sensitive collation. Disabled
options remain visible but cannot highlight or commit.

`query()` is a draft; `selection()` is the committed option ID. Editing filters and
opens the popup without committing. `set_query` updates the draft without emitting
`on_query`; `set_selection` validates an enabled option, copies its label and closes
without emitting acceptance. `set_selection(None)` clears the choice/query.
`set_items` validates atomically, preserves surviving choice/highlight keys and
clears a removed/disabled choice without rewriting the draft. `info()` exposes
query, selection, highlight, match count, open/loading/readonly state.

Up/Down open and navigate; Home/End and Page keys navigate an open popup. Selection
stays unchanged while suggestions highlight and reveal. Focus remains in the field;
the arrow and suggestions add no Tab stops. Return accepts the enabled highlight;
FreeText also accepts the draft with `selection: None` when no highlight exists.
Loading blocks acceptance. Return commits marked text before a later acceptance;
Escape first cancels composition, then cancels the popup. Repeated Return does not
accept. Pointer/AX press accepts a suggestion, including indexed offscreen rows.

A session remembers its opening query. `cancel`, Escape, outside dismissal, blur,
deactivation or owner removal restore that query and keep the committed choice.
Tab cancels and continues ordinary traversal. `show`, `accept` and `accept_key`
provide programmatic equivalents. Acceptance is inert while closed/loading/readonly
or when the requested option is missing/disabled. Changing readonly closes a session. Programmatic draft/choice changes
do not emit user callbacks.

`on_query`, `on_accept(ComboAccept)` and `on_cancel` run after publication, outside
borrows. `on_query_with_model` supplies a weakly captured model for synchronous
result replacement. Nested callback notifications are suppressed. For asynchronous
sources, the application compares its request/query generation before calling
`set_items`/`set_loading`; the component does not provide an executor.

Suggestions use `ListModel` virtualization with no fixed result cap. The popup
matches the field width, fits/flips within the viewport and paints an opaque body.
macOS exposes AXComboBox with expansion state, linked popup, parent/child and single
highlight relationships; indexed queries remain bounded and old popup proxies
become inert after dismissal/unmount. Seven focused tests cover query/commit/cancel,
filters/loading/readonly, IME, pointer/Tab, replacement/lifecycle, geometry and a
100,000-option popup. Native protocol tests cover links, indexed acceptance beyond
100 suggestions, commands and stale objects. Interactive VoiceOver/IME audit remains M5.

`cargo run --example m4_combo` demonstrates both modes and runtime loading/results.
Add `--open` for an initially open popup, `--light`, `--render file.png` or `--smoke`.
The old four-argument `combo_box` tuple helper is replaced by the model-owned view;
`stage5` uses the new interface too.


## Completion M4 — keyed tables and columns

```rust
let table = TableModel::with_columns(
    vec![TableColumn::new("name", "Name").width(240.).limits(100., 500.),
         TableColumn::new("status", "Status")],
    vec![TableRow::new("draft-1").cell("name", "Notes").cell("status", "Draft")],
)?;
// In view!: embed(table.view(), grow: 1);
```

`TableRow` stores text by column ID; reordering columns never reinterprets positional
cells. Missing values are empty strings; extra keyed values remain available if a
column is later added. Row/column IDs must be nonempty and unique. `with_options`
configures finite row/header heights, preferred/overscan rows, Rows/Cells selection
and single/multiple selection. Columns carry title, finite width/min/max, resizable,
sortable and alignment. New columns default to sortable and resizable.

`columns`, `column_index`, `row_count`, `row(index)` and `row_index` query visual
order; `text(row_key, column_key)` queries logical data. `set_rows`/`set_columns`
validate before publication, preserve surviving keys and prune retired selection.
`widths`/`set_widths` provide persisted width snapshots; `resize_column` clamps to
bounds and respects resizability. `move_column(key, destination_index)` is checked.
Width-only changes update geometry without rebuilding/sorting the dataset.

`set_sort(Vec<SortDescriptor>)` validates unique sortable columns and applies a
stable lexicographic string sort, in descriptor priority order. This is not numeric
or locale collation. Clearing sort restores source row order. Header clicks cycle
ascending/descending/none; Shift-click adds/cycles a descriptor. `on_sort` publishes
accepted descriptors after model updates; `on_sort_with_model` supplies weak model
context. Callbacks run outside borrows; nested sort notifications are suppressed.

Drag a header to reorder or its right edge to resize. Resize grips accept Left/Right
in eight-point steps and Home/End for limits. Alt-Left/Right reorders a focused
header; Return/Space cycles sorting and Shift adds a descriptor. Escape or pointer
Cancel rolls back an active resize. Blur/deactivation/removal end capture; external
width/order/lifetime changes retire it without overwriting the new state.

`TableSelection` holds row keys or `TableCell { row, column }` keys. `select_row` and
`select_cell` accept the shared Replace/Toggle/Range/AddRange/Focus operations.
Cell ranges are rectangles in current visual order, skipping disabled rows.
`selection`, `selection_count`, `cursor`, `row_cursor`, `clear_selection`, `select_all`
and `set_selection_mode` expose model policy. Changing mode clears selection.
Selection remains attached to keys through sorting, resizing and reordering;
removed/disabled rows and removed columns prune it. Header controls have their own
Tab stops; the body has one roving row or cell stop. Arrows/Page/Home/End navigate and
reveal the body; horizontal cell navigation keeps header/body scrolling aligned.

Rows use the existing virtual list; columns use shared geometry in header and body,
inside a horizontal scroller. Cell IDs include column lifetimes, preserving surviving
controls across reordering while retiring removed/reinserted columns even with equal
text. `begin_edit(row, visual_column_index)` and `edit_cell(&TableCell)` return an
external keyed `CellEdit`; `commit` checks row/column lifetimes and original value.
Integrated editing is described below.
Full native table/column/cell semantics and interactive audits remain M5.

`TableModel::new(headers, Vec<ListItem>)` remains a positional convenience with
`column-N` IDs. The old public `rows`/`headers` fields are replaced by queries and
checked mutations; `sort_by(index, descending)` now returns a Result and CellEdit's
column is a logical key. `stage5` uses the updated interface.

`cargo run --example m4_table` demonstrates 100,000 rows, both selection modes,
resize/reorder and multi-sort (`--light`, `--render file.png`, `--smoke`). Eight focused
tests cover validation, key/lifetime retention, sort stability, rectangle selection,
geometry, keyboard/pointer capture and bounded realization. Native tests cover
header sorting, cell navigation, retained/replaced focus and resize commands.


## Completion M4 — integrated table editing

```rust
let table = TableModel::with_columns(
    vec![TableColumn::new("name", "Name").editable(true).validate(|_, text| {
        if text.trim().is_empty() { Err(Error("A name is required".into())) } else { Ok(()) }
    })],
    vec![TableRow::new("one").cell("name", "Notes")],
)?.on_commit(|event| println!("Saved {:?}: {}", event.cell, event.text));
// embed(table.view());
```

Columns created with `TableColumn::new` are readonly until `.editable(true)`.
`is_editable` queries that policy; `table.is_editable(&cell)` also checks the live row.
The legacy positional `TableModel::new` enables editing for its generated columns.
`multiline(true)` and `max_length(n)` configure the standard shared text editor.
Length counts extended grapheme clusters; programmatic drafts also validate length
and supported line/control characters at commit.

Return/double-click activates an editable cell. In row-selection mode Return chooses
the first editable column. Return saves single-line text; multiline Return inserts
a line break and Command/Control-Return saves. Marked text commits before a later
submission; repeated Return never submits. Escape cancels composition first, then
the edit. Tab/Shift-Tab save and continue ordinary traversal. Failed validation keeps
the draft and editor, with a red outline, tooltip/help and visible table error text.

Blur preserves the draft and does not autosave. Activating another editable cell
first tries to commit the existing edit; failure restores its selection/focus.
Readonly targets do not commit it. Applications can call `start_edit(TableCell)`
and use the returned weak `TableEdit`: `info`, `set_text`, `focus`, `commit`, `cancel`
and `is_current`. `active_edit`, `commit_edit`, `cancel_edit` and `edit_error` provide
model-level equivalents. There is one active editor and one owned row pin.

The pin retains the host buffer, undo, selection and preedit through scrolling,
sorting and column reordering; no manual application-side pin wiring is needed.
`reveal_row` and `realized_row_count` support navigation/status without exposing the
internal list. A new session gets a fresh editor identity, even for the same cell.
Retired weak handles return no info/false and cannot change a later edit.

`validate` is a pure callback returning `Result<(), Error>`; validation panics or
state writes become commit errors. Row, column and editing-policy lifetimes plus
per-cell value versions gate commits. A value that changes and returns to the old
text still conflicts; unrelated cell updates remain valid. Equal-text duplicate
commits also retire the snapshot. External value conflicts retain the draft until
Cancel/reopen. Removed/disabled rows, removed/reinserted columns and policy changes
retire the editor. Unmount or whole mounted-model replacement cancels it; native
detach also cancels when the old Host is retained.

`set_text(row, column, text)` applies authoritative data without a user commit callback,
including readonly cells; active conflicts are reported. `on_commit(TableCommit)`
runs after data publication, editor retirement and pin release. It can update data
or start the next edit; `on_commit_with_model` supplies weak model context.

`content(|cell, text| View)` supplies a pure custom display builder. Editing uses the
standard text field when opted in. Custom content remains readonly by default;
embedded controls retain their ordinary input behavior and Tab stops. Rendering,
column alignment and width constraints still apply to the cell container.

`cargo run --example m4_table_edit` demonstrates 100,000 rows, validation, multiline
text, readonly custom content, scrolling and simulated remote conflicts. Supports
`--light`, `--render file.png`, `--smoke` and `--invalid` for a validation preview.
Seven focused tests cover activation/validation/cancel, versions and ABA conflicts,
100,000-row retention of history/preedit, row/column/policy/model lifetimes, reentrant
commit/unmount, Unicode length and multiline/Tab behavior. Native protocol tests
cover retained preedit, repeated Return, validation/cancel, real Command/Control-Return
and native detach with a retained host. Full native table semantics and actual
interactive VoiceOver/IME auditing remain M5.


## Completion M5 — tree semantics

Tree views expose an AppKit outline with logical rows. Rows report accessible labels,
zero-based disclosure levels, expanded/disclosed state and selected/enabled/focused
state. A row's AX parent is the outline; `accessibilityDisclosedByRow` links its
logical parent and `accessibilityDisclosedRows` lists direct children. Collapsed
children leave the exposed row set; known metadata remains queryable in Rust.

`TreeModel::row`/`rows(start, count)` return `TreeRowInfo` independently of viewport
realization. `disclosed_count`, `disclosed_rows` and `disclosed_snapshot` query direct
membership without invoking loaders. `row_snapshot`/`ListSnapshot` share immutable
row order and lifetime tokens. Child snapshots retain names/tokens, not loaders or
application callbacks. Current actions still validate the live mount and item token.

Ordinary native row arrays implement NSArray's count/object primitives over these
shared snapshots. Counts and ranged row requests do not create every proxy; copying
an immutable array preserves laziness. Enumerating all entries creates the requested
proxies, without realizing all widgets. Array membership/order is immutable while
each live row reports current model attributes. Native proxy caches use weak entries;
held rows share one identity across focus, hit-testing and hierarchy queries.
Entries retire when the final proxy reference is released, without rescanning the
cache on every attribute query.
Collapse makes hidden rows inert, then reopening restores the same surviving item;
removal/replacement gives new tokens and permanently retires old proxies.

Native `accessibilityRows`, visible rows, selected rows/children and range hooks
return logical proxies, including offscreen items. Visible-row queries exclude
overscan. Selection setters reject foreign, stale, hidden or disabled rows atomically
and respect single/multiple selection. Focusing a tree row preserves selection;
selection updates do not take focus from another control. Disclosure setters work
on offscreen visible logical rows and reject disabled/leaf/stale requests. Disabled
ancestors keep row metadata readable while blocking actions.

Loading appears in `accessibilityValueDescription`; failure text appears there and
in help. Confirm invokes the application activation handler or the default
branch/retry behavior. Realized failed rows also expose their Retry button as a
child, with the logical row as parent. Queries never trigger lazy loading.

Contracts follow Apple's [outline protocol](https://developer.apple.com/documentation/appkit/nsaccessibilityoutline),
[outline-row properties](https://developer.apple.com/documentation/appkit/nsaccessibilityprotocol)
and [NSArray subclass primitives](https://developer.apple.com/documentation/foundation/nsarray).

`cargo run --example m4_tree` supplies the native example, now with descriptive
outline/document labels. `--semantics` prints portable row state without opening a
window. Five focused tests cover hierarchy, immutable snapshots, selection/scope,
offscreen actions, loading/failure and loader-resource release. Run the native suite
with `SCARS_WINDOW_DESKTOP_TESTS=1 cargo test --test tree_accessibility --locked --offline`:
it checks actual outline/row attributes, direct links, selection/focus/hit identity,
loading/retry, stale objects and a 100,000-child outline. Full-array count/copy creates
zero proxies; requested entries alone populate the lazy array's cache. Two hundred
transient ranged queries return the weak registry to its original size. Combined
notifications/lifecycle and actual VoiceOver/IME sessions remain M5.4/M5.5.

## Completion M5 — table semantics

`table.view()` exposes one native AXTable with its application-supplied accessible
label. Logical AXRows contain AXCells; AXColumns link the same cells and expose
header buttons. Ordinary children include rows, columns and a header group, also
reachable through the table header attribute. Header/column sort direction and help describe multi-sort precedence.
Pressing a sortable header uses the existing sort cycle. Resize controls and custom
cell controls keep their native actions; custom control focus/hit testing is preserved.

`TableModel::snapshot()` shares immutable row data/order/tokens and copies callback-free
`TableColumnInfo` metadata. It never retains validators, display builders or the model.
`TableCellInfo` reports live indexes, lifetimes, selection, editability, current draft
text and validation errors without realizing a row. Changing sort or column order
preserves surviving logical identities; removal and reinsertion create new lifetimes.

Native row, column and column-cell arrays are lazy immutable NSArray subclasses.
Counts/copy do not create every proxy; indexed requests allocate only requested
entries. `accessibilityCellForColumn:row:` reaches offscreen cells and rejects invalid
indexes. Visible rows exclude overscan; visible cells/columns respect horizontal
clipping. Weak caches share identities across hierarchy, indexed queries, focus and
hit testing, then remove entries on proxy deallocation. Retained arrays preserve their
original membership/order; entries read current attributes and become inert after
replacement, removal or unmount.

Selection arrays/setters follow row/cell mode and reject foreign, stale or disabled
targets atomically. Selection ranges use stable key order. Replacing selection does
not take focus from another control; explicit focus reveals only the target row and
preserves multiselection. Columns do not support independent column selection.

Confirm starts an editable cell's shared text editor. Setting its value uses the same
validation/commit path as keyboard editing; invalid drafts remain available through
value/help and the native editor child. The editor retains text input/selection APIs,
and its parent is the logical cell. Readonly cells, disabled ancestors and modal
isolation block mutation. Switching editors rechecks data/model lifetimes after
application commit callbacks. A stale proxy cannot edit replacement data.

The mappings follow Apple's [table protocol](https://developer.apple.com/documentation/appkit/nsaccessibilitytable)
and [cell-based table properties](https://developer.apple.com/documentation/appkit/nsaccessibilityprotocol).
Run `cargo run --example m4_table -- --semantics` for portable metadata, `m4_table`
for sorting/selection/columns, or `m4_table_edit` for validated drafts. Focused core
checks are `cargo test -p scars-ui --test table_semantics`; native checks are
`SCARS_WINDOW_DESKTOP_TESTS=1 cargo test --test table_accessibility --locked --offline`.
The latter exercises a 100,000-row table, lazy allocation, shared identities, index
changes, row/cell selection, custom controls, editing/errors and stale-object/resource
cleanup. Combined notification and actual VoiceOver/IME audits remain M5.4/M5.5.

## Completion M5 — control and presentation semantics

Application composition supplies semantic state automatically. `Menu::button`,
`numeric_field`, `value_field`, `stepper`, tabs, splits, color wells and alerts keep
native details out of application code. `accessible_help` supplies instructions
independent of a visual tooltip; `validation(model.validation())` attaches typed
validation to an ordinary field. Both are supported by `view!`. Numeric/value fields
attach validation themselves, and numeric steppers report their available directions.

| Control | Native contract |
| --- | --- |
| Text field/editor | AXTextField/AXTextArea; validation message plus instructions in help; invalid/incomplete/out-of-range descriptions |
| Toggle/check/radio/segment | Typed role/subrole and value/selection; exact boolean/tristate assignment; groups expose selected children |
| Stepper/slider | Exact numeric value and bounds; advertised increment/decrement availability; stepper button links |
| Tabs | AXTabGroup, tab-button subroles, selected tab/contents links and guarded selected-child/value setters |
| Split | AXSplitGroup/AXSplitter, orientation, bounds, adjacent contents and adjustment/value/collapse actions |
| Menus | AXMenuButton/AXMenu/AXMenuItem; expanded/checked state, shortcut help, linked opener and explicit submenu parentage |
| Dialog/alert | Named dialog subrole and modal state; live default/cancel buttons and guarded confirm/cancel actions |
| Popover/color well | Popover/color-well roles; color value, labeled channels and existing popup actions |

Validation uses AppKit's standard help/value-description attributes; it does not
invent a platform-specific invalid-state selector. Portable snapshots retain the
full `Validation` status/message/dirty state. Typed value controls still follow their
current explicit ASCII/ISO grammar; broader value policies/localization remain M6.

`Host::set_selected`/`set_check_state` assign a control's value without synthesizing
an unrelated button click. Readonly, disabled, hidden, modal and stale targets are
guarded. A selected radio/tab cannot be cleared by assigning false. Active table
text editors retain the native value setter and shared validation/input APIs.

Menu show actions are idempotent. A menu opener remains readable and linked during
its own menu while input focus stays scoped. Expanded submenu panels belong to their
menu items, not to an unrelated overlay/group. Cancel closes the current submenu
before the root menu, respects Escape policy and cannot dismiss an inactive owner.
Native parent links use the same readable scope as child enumeration. Count/range
queries for tabs, selected children, contents, splitters and links agree with their
direct array getters.

`cargo run --example m5_semantics` demonstrates ordinary project-settings code with
validation, numeric fields, tabs, menus and an alert. `--semantics` prints portable
metadata. Five focused core tests cover annotations, exact assignments/guards,
selection, menu paths and cancellation scope/policy. Run native protocol/action tests
with `SCARS_WINDOW_DESKTOP_TESTS=1 cargo test --test control_accessibility --locked --offline`.
They cover real roles/subroles, value/selection setters, validation/help, bounds,
stepper links, tabs/split contents, menu identity/hierarchy, dialogs and color wells.
Combined lifecycle/notification evidence is recorded below; interactive VoiceOver/IME sessions remain M5.5.

The native mappings follow Apple's [accessibility protocol](https://developer.apple.com/documentation/appkit/nsaccessibilityprotocol)
and [role/subrole definitions](https://developer.apple.com/documentation/appkit/nsaccessibility-swift.struct/subrole/toggle).

## M5.4 — native identity and notifications

A queried generic accessibility element is retained for its mounted node's lifetime,
including hidden retained tab pages. The cache prunes retired node IDs, rather than
hidden nodes. Logical collection arrays remain lazy. Native identifiers include the
mounted node ID, so equal application keys under different parents cannot collide.
Held objects become inert after removal, unmount or close; a removed/reinserted view
gets a new identity. Native proxies keep weak owners and do not retain application
callbacks. Parent dialogs are temporarily inaccessible while a nested dialog is active,
then recover their surviving identity.

Hit testing filters through the same readable modal scope as attribute queries.
In particular, a point outside a suggestion popup cannot return an inaccessible
background control. Frame/range conversion uses AppKit's view/window/screen conversion
APIs in logical points. Native tests move/resize a real window, round-trip frame
coordinates, verify UTF-16 emoji ranges and reject split surrogate ranges. The recorded
run used actual backing scale 1; it does not establish the separate scale-2 audit.

Posting runs on a coalesced main-runloop task independently of drawing. Semantic
snapshots contain mounted nodes and only the logical rows/cells an AX client has
requested. Constant-space model stamps distinguish content and selection changes;
focus-only movement preserves multiselection. Released logical proxies leave no
snapshot entries. A 100,000-row test keeps fewer than 200 entries.

AppKit receives value/title, text selection/composition, logical focus, row/cell/
child selection, disclosure, row-count, geometry and destruction notifications.
Layout posts include the standard changed-elements payload. Same-length password
edits produce value changes without storing plaintext in notification snapshots or
traces. Removed targets receive destruction once; hidden retained pages preserve
identity. Unmount/close cancel pending posts. Custom list controls retain their own
focus, text APIs and logical row parent.

Run these native contracts from `rust/`:

```sh
SCARS_WINDOW_DESKTOP_TESTS=1 cargo test --test accessibility_lifecycle --test accessibility_notifications --locked --offline
cargo run --example m5_semantics --locked --offline
```

The example demonstrates retained tab editing and scoped alerts without native
application plumbing. The lifecycle test verifies posts with zero draw callbacks;
the notification suite checks exact names/targets, payload presence, bounded
snapshots, idle quiescence and teardown. Internal diagnostics count calls to the
real AppKit API; they do not establish external-client receipt. Actual VoiceOver/
IME sessions at both backing scales remain M5.5. Layout payloads follow Apple's
[notification API](https://developer.apple.com/documentation/appkit/nsaccessibility-swift.struct/notification/layoutchanged).

## M6.1 — editable format policy

[Editable formats](../../docs/rust-editable-formats.md) specifies the implemented
ASCII/ISO grammar and checked locale profiles. Civil editors
consume their entire input: a date cannot silently discard a time, and a local
time/datetime cannot discard an offset or zone. Leap seconds are rejected rather
than clamped. Named-zone input still uses explicit gap/overlap resolution.

`cargo run --example m6_formats` demonstrates exact integers, calendar dates and
nanosecond time/local datetime drafts. Four portable tests cover ambiguity,
integer precision, date/time extremes, suffix rejection and named-zone parsing.
Localized display-only formatters do not change the default editing grammar.

## M6.2 — explicit number/date localization

```rust
let locale = EditLocale::new("fr-FR")?;
let amount = Numeric::new(value.binding(), -1000., 1000., 0.5).locale(locale.clone());
let calendar = DatePicker::new(date.binding()).locale(locale);
```

`en-US`, `en-GB`, `de-DE`, `fr-FR` and `ar-EG` have explicit editable grammars;
unsupported profiles/extensions return an error. `EditLocale::ASCII` is the default
and preserves ISO dates and ungrouped ASCII numbers. Configure builders before
mounting: changing locale/precision restores their committed value. Application
labels stay application-owned; this does not supply a general translation catalog.

ICU4X formats exact decimal text; integer parsing/stepping stays in the integer type.
Grouping is optional and checked in groups of three; fractions/exponents cannot
contain grouping. Arabic accepts its digits or ASCII digits with the same separators,
rejects mixed digit sets and consumes only ICU's exact sign affixes or ASCII signs.
Rounded display never overwrites an untouched precise model. Calendar hints, action
labels, month/day text, field order, week start and Arabic grid direction match the
selected profile. Localized Gregorian years are 0001–9999; ISO keeps the full range.
Calendar cells share remaining row space after gaps and fit narrow widths.

`cargo run --example m6_locales` opens the five-profile lab; `--locale ar-EG` selects
its initial page. Six portable tests cover precision, grammar, stepping, external
changes, date order, calendar labels/actions and narrow geometry. Run the native
text/AX transport checks with
`SCARS_WINDOW_DESKTOP_TESTS=1 cargo test -p scars-window --test localized_values --locked --offline`.
Actual VoiceOver/IME sessions remain M5.5; date-policy evidence follows below.

## M6.3 — date control policies

```rust
let picker = DatePicker::new(value.binding())
    .bounds(min, max)
    .disabled_dates(|date| date.weekday().to_monday_zero_offset() >= 5)
    .validate(|date| validate_booking_date(date))
    .on_submit(saved.action(|s| s.set(true)));
```

Bounds are inclusive. ASCII/ISO supports Jiff's full date range; localized profiles
support years 0001–9999. Invalid committed values remain visible and invalid rather
than being silently clamped. Invalid/incomplete/out-of-range drafts retain the
committed value. Validators and disabled-date predicates are read-only queries;
reactive reads track dependencies, while writes are rejected. Custom errors belong
to the application. Configure locale, bounds and predicates before mounting; changing
those builders restores the committed draft. Readonly/blur flags can vary per view.

`commit_on_blur(true)` is the default. Apply/Enter commits and calls `on_submit`
only on success; blur commits without submitting. Choosing a day commits directly.
Revert/Escape restores the model/draft/cursor. `read_only(true)` disables editing,
Apply/Revert buttons and day activation while allowing calendar inspection/navigation.
`on_validation` runs outside state borrows and suppresses recursive notifications.
Callbacks may publish newer authoritative values. Native day actions recheck owner,
mount, enabled state and modal scope after callbacks, so removed/replaced controls
cannot finish a stale commit. Rejected custom setters preserve the draft.

The calendar has one roving day stop, alongside its month buttons and text controls.
Arrows move one day horizontally or one week vertically; horizontal movement follows
RTL direction. Page Up/Down moves a month, Shift moves a year, Home/End selects the
week endpoints, and Command/Control-Home/End selects month endpoints. Movement never
commits; Enter/Space chooses the focused day. Unavailable days within bounds remain
inspectable but cannot be activated. Month buttons stop at bounds. Focus reveal runs
after layout; later manual scrolling does not repeatedly snap back.

External value changes synchronize the draft, visible month and calendar focus.
The text editor keeps its identity when the number of calendar rows changes.
State-backed bindings automatically carry revisions, preventing stale A→B→A drafts;
custom providers can attach the same signal with `Binding::with_revision`.
A revision must describe accepted-value changes and be a read-only query.
Composition confirmation commits preedit before a later Enter submits the value.

`cargo run --example m6_date` demonstrates these policies. Eight portable tests cover
rules, navigation/leaps/extremes, blur/readonly/callbacks, source revisions, replacement,
editor identity and constrained scrolling. Hidden-window native tests verify actual
AX selected children/day state, unavailable focus, retained field identity, native
text/IME transport and inert stale objects. The hidden `accessibility_focus` test
also checks meaningful expansion attributes and nonduplicated text-focus posts.
Interactive VoiceOver/IME and visible GPU checks remain explicit in M5.5/M8.

## M6.4 checkpoint — standalone time control

```rust
let picker = TimePicker::new(value.binding())
    .hour_cycle(HourCycle::H12)
    .show_seconds(true)
    .minute_step(5)
    .bounds(min, max)
    .validate(|time| validate_booking_time(time));
// In a builder: embed(picker.view());
```

One typed field owns the draft; hour/minute/optional-second steppers and AM/PM
share it. A valid step commits the whole candidate. Invalid text stays editable
and disables stepping. Hour/minute edits preserve hidden seconds/nanoseconds;
explicit seconds replace them. Steps use exact arithmetic and carry into larger
components, rejecting day/bounds/custom-rule violations. Steps are 1–60 for minutes
and seconds. Focus a stepper and use arrow keys; native AX increment/decrement
exposes exact component values and direction availability.

`HourCycle` is explicit rather than inferred from locale. `EditLocale` supplies
digits, fractional separators, period/action labels and grammar hints. Bounds are
inclusive and validators are pure. Configure once before mounting; configuration
builders reset the draft. Readonly and blur flags can vary per view. The default
commits on blur without submitting. Apply/Enter commits and calls `on_submit` on
success; Revert/Escape restores the source and calls `on_cancel`. External source
revisions invalidate stale edits; rejected setters retain text. Callbacks can
publish newer authoritative values without a subsequent stale write.

`binding()`, `draft()`, `validation()`, `commit()`, `cancel()`, `can_step()` and
`step(TimePart, increase)` expose the same behavior to application code.
`cargo run --example m6_time` opens the concise two-clock example. Six portable
tests cover exact values, locales, boundaries, callbacks, keyboard/blur and source
changes. The hidden native `time_picker` harness covers AX values/actions, marked
text confirmation, readonly behavior and retired objects. This is a checkpoint:
zoned instant validation and timezone editing follow below.

## M6.4 checkpoint — combined civil datetime

```rust
let picker = DateTimePicker::new(value.binding())
    .bounds(min_datetime, max_datetime)
    .show_seconds(true)
    .minute_step(15)
    .validate(|datetime| validate_booking(datetime));
// In a builder: embed(picker.view());
```

One field owns the full draft: date, literal `T`, then the clock grammar. Locale,
hour cycle, optional seconds and precision follow `DatePicker`/`TimePicker`.
Applications can edit both parts across tight overnight limits before one valid
write; no application-side date/time staging is needed. Calendar choices keep the
staged clock, and clock steps carry exactly through midnight, leap and year changes.
Every write checks joint inclusive bounds, disabled dates and the pure custom
validator. Invalid text and rejected setters remain editable; invalid external
values remain visible. A malformed draft disables choices/steps; clock steps also
require a valid whole candidate. A calendar choice can repair a parseable invalid
date only when the resulting whole value is valid.

The group shares readonly, blur/submit/cancel and validation callbacks with the
standalone controls. Configure before mounting. External revisions reset stale
text, calendar selection and focus; changing the number of calendar rows retains
the field. Calendar arrows/page keys inspect without writing, and Escape restores
the group. `binding()`, `draft()`, `validation()`, `commit()`, `cancel()`, `select()`,
`calendar()` and `step(TimePart, increase)` expose the same behavior directly.

`cargo run --example m6_datetime` demonstrates an overnight range. Six focused
portable tests cover one-write joint editing, exact carry/extremes, localized
grammar, shared calendar/clock state, readonly/lifecycle/source changes, rejected
setters and callback authority. Native text/AX/composition coverage is in the
hidden `datetime_picker` harness. Zoned instant limits, DST and zone changes follow below.

## M6.4 — zoned datetime

```rust
let instant = State::new("2024-11-03T06:30:00Z".parse::<Timestamp>()?);
let picker = ZonedPicker::new(instant.binding(), "America/Edmonton")?
    .bounds(min_instant, max_instant)
    .show_seconds(true)
    .minute_step(15);
// In a builder: embed(picker.view());
```

The control binds an instant and owns its explicit display zone. `zoned()` returns
that pair; `time_zone()` reads the selected zone. This replaces the prototype's
`Binding<Zoned>` and public string resolution state: Jiff compares `Zoned` by instant,
so a zone-only metadata write could otherwise disappear as an equal reactive value.
The civil field shares the combined datetime grammar and rejects offset/zone
suffixes. IANA names/fixed offsets are edited separately with an explicit operation.

`change_zone(name, ZoneChange::KeepInstant/KeepLocal)` commits a valid staged
candidate while preserving either its instant or its wall time. Whole-value civil
limits, exact inclusive instant bounds and `validate(&Zoned)` apply in the target
zone before any write. Failed input, bounds, DST or rejected setters retain the
draft and zone; normalized/reentrant application writes stay authoritative.
Readonly disables edits, occurrence selection and zone changes. Configuration
builders run before mounting; readonly/blur flags can vary per view clone.

Gaps always fail. `set_repeated_time(RepeatedTimePolicy)` revalidates the existing
text without resetting it. The default rejects a new repeated wall time; Earlier
and Later choose a real occurrence. An existing unchanged fold retains its known
instant, while `choose_occurrence(policy)` explicitly switches it. The UI exposes
the policy and Apply occurrence action. Clock stepping applies wall arithmetic then
resolution, including half-hour transitions; no fixed DST duration is assumed.
`resolve_zoned`/`Resolution` remain general low-level conversion helpers with their
previous explicit gap-shifting behavior. The control intentionally has a stricter
gap policy.

The same binding/validation/commit/cancel, calendar, stepping and callbacks as the
civil editor remain available. The zone field exposes validation to accessibility.
Stable keys retain the focused civil field when fold controls/errors appear or
zones change. Source revisions detect stale drafts even when a fold's two instants
share identical civil text. A callback can supersede a step; its resulting instant
is revalidated before success is reported.

`cargo run --example m6_zoned` opens the runnable gap/fold/zone lab. Nine core tests
cover exact instant/civil bounds, half-hour folds, rejected gaps, both zone-change
modes, hidden precision, localized input, readonly/extremes, source changes,
rejected setters and callback authority. The hidden native `zoned_picker` harness
covers actual text/AX transport, policy/occurrence/zone actions, errors, retained
field identity, composition-before-submit, readonly and retired objects. Actual
VoiceOver/IME candidate and visible rendering evidence remain M5.5/M8.

## M6.5 — color control

```rust
let picker = ColorPicker::new(paint.binding()); // transactional
let preview = ColorPicker::new(other.binding())
    .model(ColorModel::Hsv)
    .alpha(false)
    .live_preview(true);
// In a builder: embed(picker.view());
let (button, popover) = picker.well(&popup, "paint");
```

RGB/HSV fields, sliders and hex share a draft. Apply/Enter publishes the valid
whole color; Cancel/Escape discards it. Blur retains unfinished text. Marked text
confirms first, and `Host::apply_color(id)` refuses while a color field composes.
`Host::cancel_color` and `color_validation` also accept the group or a descendant.
Invalid text blocks switching models/sliders and the other input representation
while leaving the offending fields editable. Rejected setters retain feedback and
allow Apply to retry; accepted normalization and newer callback values win.

RGB/alpha channels use whole bytes 0–255. Hue uses degrees 0–360; saturation/value
fields and `ColorChannel` use percentages 0–100. Public `Hsv` uses fractions 0–1;
`Color::hsv()`/`from_hsv()` convert encoded sRGB with nearest-byte rounding. Hue survives
gray/black and saturation intent survives black; untouched precision survives rounded
three-decimal field display. Hidden alpha is preserved; explicit alpha hex input
fails when disabled. Checkered previews preserve transparent RGB payloads.

`live_preview(true)` publishes valid edits but retains the original source until
Apply finalizes the session. Cancel requests rollback only if the source/revision
still matches the last owned preview. External changes reset the draft and drop
ownership, including A→B→A with State or revisioned custom bindings. Reentrant
setters/callbacks can supersede actions without a subsequent stale write.
Hide/disable/collapse/removal/deactivation/unmount and controller drop retire
unfinished sessions outside construction/borrows. Policy builders run before
mounting; readonly disables edits while retaining selectable text. Theme changes
preserve the session. Retain one picker per independent editor session.

`well` opens the editor in an anchored popover with scrolling/fitting and focus
restoration. Apply finalizes/closes; Cancel, outside and Escape cancel it. Existing
Popup dismissal observers are retained, and a reopened generation cannot inherit
the prior preview. This replaces the prototype's immediate channel writes and
public `draft` with `binding()`, `validation()`, `info()`, `commit()`, `cancel()`,
`set_component()`, `set_model()` and optional validation/submit/cancel callbacks.

`cargo run --example m6_color` demonstrates transactional/live/alpha and well
behavior. Focused core tests cover conversion, precision, shared editing, source
authority and ownership/lifetime transitions. A raster test checks actual alpha
compositing; hidden native tests cover RGB/HSV fields/sliders, retained identity,
composition, alpha validation, popup rollback/Apply, readonly and unmount. Actual
VoiceOver/IME candidate and visible GPU evidence remain M5.5/M8.

Use `--well` to start with the color popover open. It sizes to its content up to
the configured maximum height and scrolls when the viewport is constrained.
