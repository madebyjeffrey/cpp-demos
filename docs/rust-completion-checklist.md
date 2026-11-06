# Rust SCARS completion report and checklist

Date: 2026-11-05. Reviewed baseline: `82583e7`.

The Rust workspace is a usable first implementation across the six original
milestones. It is **not yet complete against their exit criteria**. This report
is the working checklist for closing that gap. A compiling example, a composition
of simpler controls, or an available native protocol does not by itself establish
that the corresponding behavior is finished.

## Scope and working rules

The user confirmed **macOS components and the portable core**. Native Windows and
Linux window backends are outside this goal. SCARS behavior is the reference;
Rust names, interfaces and visuals may improve without preserving C++ compatibility.
Skia's native engine remains acceptable; SCARS component logic stays in Rust.

Work through the milestones below in dependency order. Keep application setup and
`view!` usage concise; hide routine ownership, scheduling and native plumbing.
After each milestone, prepare a runnable example, a few meaningful tests, updated
documentation and a commit, then continue. Do not require another approval between
milestones. Reuse existing examples when that makes the result easier to understand.

Mark an item complete only with implementation and evidence. Record the commit,
example and checks under that milestone. A blocked desktop test stays unverified;
source inspection and a skipped harness are not substitute passes. Do not mark the
overall goal complete while an in-scope item remains open.

## Current coverage

| Area | Implemented foundation | Remaining closure |
| --- | --- | --- |
| State and descriptions | Reactive bindings, keys, macro conditions/repetition, host identity and inventory mapping | Ergonomic coverage of remaining component behaviors |
| Layout and basic controls | Inventory, constrained layout, rich text, style coverage and two-axis scrolling verified | Final cross-component visual/semantic audit in M5/M8 |
| Editing | Shared buffer, visual navigation/gestures, search/password/numeric policies and held steppers | Localization/value-control expansion in M6; final interactive IME audit in M5 |
| Presentation | Retained splits, hierarchical menus/split actions, configured portals/help, nested focus scopes, retained tabs and validated wizard flows | Alert and sheet completion transactions |
| Collections | Fixed-height virtualization, keyed selection, basic trees/tables, cell conflict checks | Lazy tree lifecycle, cell/column interactions, richer selection and semantics |
| Values and tooling | Typed drafts, sRGB/hex, calendar, ISO/zoned editing, picking/width overrides | Complete value-control policies and inspector/Lab coverage |
| Verification | Core/CPU tests and native input/accessibility protocol tests | Interactive audits, final visible GPU/full-screen runs and repeatable checks |

The main reference is the [current C++ public inventory](../scars/src/scars.cppm),
including options and modifiers, not only control names. The
[original migration sequence](rust-components-feasibility.md#suggested-migration-sequence-and-exit-criteria)
defines the intended evidence. Existing Rust contracts are in the
[component README](../rust/components/README.md).

## M1 — Inventory, layout and scrolling

- [x] **M1.1 Inventory closure:** map every current SCARS control, model, modifier and option family to an implemented Rust equivalent or a remaining checklist item. Document intentional interface changes; no unexplained omissions.
- [x] **M1.2 Style coverage:** finish per-edge insets, flex basis/self-alignment/spacing, relative positioning, text alignment/overflow/line limits, label styles, accent/density and separator orientation where the inventory requires them.
- [x] **M1.3 Layout correctness:** exercise wrapping/reflow, grow/shrink redistribution at min/max limits, percentages, baselines, clipping and layered hit order under constrained sizes.
- [x] **M1.4 Two-axis scrolling:** horizontal/vertical/both axes, visible draggable scrollbars, wheel/trackpad transport and programmatic offset/query interfaces with clamping.
- [x] **M1.5 Scroll/input lifetime:** nested wheel propagation, two-axis focus/caret reveal, keyboard scrolling, drag autoscroll and capture cancellation on blur, hide, removal and unmount.

Evidence: extend the layout/scroll examples; deterministic geometry, edge propagation
and cancellation tests; dark/light constrained previews. Likely changes:
[geometry/layout](../rust/components/src/layout.rs),
[input](../rust/components/src/input.rs), [native adapter](../rust/src/macos/components/input.rs).

Completion record:

- M1.1: [inventory matrix](rust-inventory.md) maps 162 exported types and all 47
  view/annotation names to existing behavior or explicit closure items.
  `python3 scripts/check_rust_inventory.py` verifies name coverage, not behavior.
- M1.2/M1.3 checkpoint: per-edge insets, flex basis/self-alignment/space-around,
  min/max redistribution, relative offsets, inherited accent/density, text roles,
  line limits/ellipsis/alignment/spacing and vertical separators implemented.
  Example: `cargo run --example m1_layout` (also `--light` / `--render file.png`).
  Evidence: four portable style/geometry tests plus a real SkParagraph limits,
  alignment and cache test; existing workspace tests and strict Clippy pass.
- M1.2/M1.3 closure: shared width-dependent reflow propagates wrapped heights to
  ancestors and following siblings; baseline alignment includes inline controls,
  nested padding and control text centering. `rich_text` shapes styled `Span`s and
  inline `View`s in one SkParagraph; clipped/ellipsized children keep identity but
  become inert. Five portable reflow/order tests and four raster rich-text tests
  cover resizing, baselines, clipping, subtree stacking, painting and interaction.
  Workspace tests and strict Clippy pass; dark/light `m1_layout` previews inspected.
  Checkpoint commits: `53a4202` (styles), `24156ef` (reflow/rich text).
- M1.4/M1.5 (`dcf5ea8`): `ScrollAxes::{Vertical, Horizontal, Both}`, clamped `scroll_info`,
  `scroll_to`/`scroll_by`, draggable overlay bars and track paging implemented.
  Native wheel transport includes both axes; residual diagonal deltas travel only
  through containing ancestors. Focus/caret reveal, keyboard movement and stationary
  drag-selection deadlines work through nested viewports; user scrolling does not
  repeatedly snap back to an unchanged caret. Blur/deactivation/hide/removal/cancel
  end capture. Native unmount ends the selection deadline and focus.
- Example: `cargo run --example m1_scroll` (`--light`, `--render file.png`, `--smoke`).
  Six portable scroll tests and three actual SkParagraph/raster tests pass, as do
  workspace tests, strict Clippy, portable Linux compilation and native component
  protocol tests. Dark/light previews inspected; native example smoke run passes.
  These are M1 checks; final visible GPU and interactive audits remain in M5/M8.


## M2 — Editing and specialized fields

- [x] **M2.1 Shared text model:** provide a reusable Rust text-buffer interface for selection, committed text, composition, history and external replacement without requiring a native window. Retain existing grapheme/UTF-16 guarantees.
- [x] **M2.2 Navigation:** visual up/down and line/page movement through wrapped text, preferred horizontal caret position, word/line deletion and selection extension; distinguish line boundaries from document boundaries.
- [x] **M2.3 Selection and history:** word/line selection gestures, drag selection outside the viewport, appropriate typing coalescing and history limits; composition, readonly and grapheme-counted maximum-length cases.
- [x] **M2.4 Search field:** integrated clear/cancel, query/submit callbacks and configurable debounce. Cancel pending work on replacement, removal and deactivation without an idle animation loop.
- [x] **M2.5 Password policy:** explicit reveal, undo, readonly and length options; verify masking, clipboard/accessibility restrictions, composition and balanced native secure-input ownership through every transition.
- [x] **M2.6 Numeric/stepper policy:** standalone stepper, configurable precision/validation states, blur/submit/cancel policy and press-and-hold repeat deadlines; preserve exact integer arithmetic and invalid/incomplete drafts.

Evidence: extend the editor example; Unicode/visual navigation and IME/history tests;
native clipboard/candidate-geometry checks. Main implementation:
[editing](../rust/components/src/editing.rs), [drafts](../rust/components/src/draft.rs).

Completion record:

- M2.1: public `TextBuffer`/`TextOptions` now own selection, committed/composing
  text, replacement, readonly/length policy and history. Mounted fields use the
  same buffer. External replacement cancels composition and clears history;
  UTF-8 ranges snap at ICU4X grapheme boundaries. Example without a native backend:
  `cargo run -p scars-ui --example text_buffer`.
- M2.2: shaped `TextLine` geometry and caret affinity drive wrapped up/down,
  line/page movement, preferred horizontal position and visual line deletion.
  Logical word/paragraph/document commands remain distinct. Native AppKit line,
  paragraph, document, deletion and page selectors map to the corresponding
  portable commands. UTF-16 line metrics convert to UTF-8, including blank final
  lines and emoji/combining sequences.
- M2.3: native double/triple clicks select words/visual lines; dragging extends
  whole units and uses M1's stationary autoscroll. Typing and repeated deletion
  coalesce within a configurable interval; selection, paste and composition break
  groups. History limits, readonly editing and grapheme-counted maximum lengths
  are configurable on ordinary fields (`max_length`, `undo_enabled`, `history_limit`).
- Evidence: five portable buffer/field tests, five real SkParagraph navigation and
  gesture tests, existing editing/scroll regressions, workspace tests, strict
  Clippy and portable compilation. Native protocol tests exercise explicit IME
  replacement as one undo step, line/document commands and candidate rectangle/
  index coordinate conversion. Dark/light `m2_editor` previews inspected and its
  native smoke run passes. Actual interactive VoiceOver/IME sessions remain M5.5.
- M2.4: search fields provide real clear buttons, Escape/cancel, query/submit
  hooks and scoped one-shot debounce. Only committed accepted edits are queried;
  external replacement, readonly/disable/hide/removal, blur/deactivation and
  unmount cancel pending work. Submit flushes its own query first and rechecks
  lifetime after callbacks. `State::handler` captures event payload callbacks
  without application-side reference-counting boilerplate.
- M2.5: passwords default to masked/no undo; reveal and undo are explicit options.
  Readonly and grapheme limits share the text buffer. Reveal remasks on Escape,
  focus loss/deactivation/hide/removal, remains sensitive for clipboard and AX,
  and shapes/paints without retaining plaintext in the paragraph cache. Native
  search/password subroles and accessible child buttons are implemented.
- Evidence: six portable field/callback tests; real renderer cache/painting test;
  native protocol tests for revealed-password AX/copy/cut/composition and scoped
  search unmount; secure-ownership success/failure/nesting unit test. Actual
  `SCARS_WINDOW_SECURE_INPUT_TESTS=1 cargo test --test secure_input` **passed**
  Carbon transitions through reveal, readonly/disable, focus/deactivation/hide,
  unmount/replacement and close on a foreground desktop. Workspace tests, strict
  Clippy and portable compilation pass. `m2_fields` dark/light previews inspected
  and native smoke run passes. These protocol/ownership checks do not replace the
  actual VoiceOver/IME audit in M5.5 or final Metal pixel runs in M8.1.
- M2.6: `Numeric<T>` provides significant-digit display precision, readonly,
  blur/submit/cancel policy and `Validation` (`Valid`, `Incomplete`, `Invalid`,
  `OutOfRange`, message/dirty). Integer formatting and checked stepping stay exact;
  invalid/incomplete or rejected edits stay visible. An untouched precision-formatted
  value is never rounded on blur. Drafts compare typed source values as well as
  their formatting, so external changes cannot hide behind rounded display text.
- `stepper(label, &number)` is a real compound control with one keyboard stop,
  arrow-key actions, exact value/bounds snapshots and native AX incrementor
  values/actions. Held buttons use one-shot repeat deadlines; bounds, release,
  cancel, focus change, deactivation, hide/disable/removal and unmount stop capture.
- Evidence: six portable numeric/repeat tests (including integer overflow, formatter
  collisions, commit/cancel, readonly and capture lifetimes); native NSNumber
  value/bounds/actions and repeat-unmount checks. Workspace tests, strict Clippy,
  portable compilation and native input/secure-ownership suites pass.
  `cargo run --example m2_numbers` demonstrates the completed policies; dark/light
  previews inspected and its native smoke run passes.
- M2 checkpoint commits: `8524b99` (buffer/navigation), `024a26d` (search/password)
  and `3b88eef` (numeric/stepper closure). Localization
  and fuller typed value controls remain M6; the actual VoiceOver/IME audit remains
  M5.5 and final Metal/visual/resource evidence remains M8.


## M3 — Navigation and presentation

- [x] **M3.1 Split views:** draggable dividers, both orientations, pane limits, collapse/restore and keyboard adjustment; resize cursors, capture cancellation and accessible split semantics.
- [x] **M3.2 Menus and split actions:** separators, checked/radio items, nested submenus, shortcut labels, disabled-item navigation, typeahead and complete open/close/activation behavior; a split action can retain its chosen primary action.
- [x] **M3.3 Overlay policy:** configurable placement, sizing, viewport fitting, focus preservation and dismissal callbacks for popovers/tooltips/context menus, including nested scopes and owner removal.
- [x] **M3.4 Tabs:** stable page identity, enabled tabs, keyboard navigation and restoration of editor history, selection, scroll and focus after switching pages. Application model state alone does not satisfy this item.
- [x] **M3.5 Wizards:** stable step IDs, validation that can block navigation, back/review policy, finish/cancel/change callbacks and valid behavior for empty/changed step sets.
- [ ] **M3.6 Alerts and sheets:** an explicit result/completion transaction with default/cancel/destructive actions; exactly-once completion through action, Escape, dismiss, close and destruction. Demonstrate native parent-sheet isolation and focus restoration.

Evidence: extend the workspace example with a nested menu, retained tab editor and
confirmation sheet; focused interaction/lifecycle tests. Main implementation:
[presentation](../rust/components/src/presentation.rs), [native mounting](../rust/src/macos/components/mod.rs).

Completion record (in progress):

- M3.1: dedicated `Split`/divider nodes replace the ratio-slider composition;
  `split_view(&ratio, first, second)` remains supported. `Split::new`/`bound`,
  builder options and an optional collapsed binding support both orientations,
  minima, divider width, keyboard step and programmatic collapse/restoration.
  Infeasible minima scale proportionally without rewriting the preferred ratio.
- Divider dragging preserves grab offset and clamps at pane limits. Arrows, Shift,
  Home/End and Enter/Space/double-click adjust or collapse; Escape restores a
  collapsed pane or rolls back a live drag. Resize cursors follow the active axis.
  Blur, focus change, deactivation, hide/disable/removal/unmount, orientation changes
  and external ratio/collapse changes cancel capture. Panes remain mounted while
  collapsed; hidden preedit cancels, while history/selection/scroll survive.
- Evidence: six portable geometry/input/lifetime tests, including the legacy helper
  and macro/model-name hygiene. Native AX split-group/splitter orientation, numeric
  value/bounds/actions, previous/next contents and inert collapsed/unmounted objects
  pass. Workspace tests, strict Clippy and portable compilation pass.
  `cargo run --example m3_split` demonstrates nested splits; dark/light previews
  inspected and native smoke run passes. Final interactive semantics remain M5.
- M3.2: `Menu`/`MenuAction` now cover stable keys, separators, check/radio
  bindings, shortcut hints, disabled navigation, nested branches and Unicode
  lowercase prefix typeahead with repeat-letter cycling/expiry. Pointer hover,
  drag-release, scoped arrows/Home/End, Enter/Space, Left/Escape, Tab, outside
  presses, cancellation, deactivation and anchor loss have explicit behavior.
  Branch replacement repairs focus; long menus scroll/reveal; panels flip/fit.
  `SplitAction` retains the chosen leaf path with disabled/removed-choice fallback.
- Evidence: ten portable menu interaction/lifetime tests; native printable-key
  routing without an editor, AX menu/item roles, check values, shortcut help,
  expanded empty branches and inert closed/unmounted elements pass. Workspace
  tests, strict Clippy, portable compilation, dark/light inspected previews and
  native `m3_menu --smoke` pass. The existing `Popup::menu` stays compatible.
- M3.3: shared `OverlayOptions`/placement fitting for popovers, menus and tooltips;
  intrinsic/explicit width, bounded scrolling, gap/margins, content/preserved focus
  and independent outside/Escape/deactivation policies. Portals escape owner clips
  and paint above ordinary content. Nested scopes restore individual prior controls;
  parent closure clears descendants and clipped-owner dismissal preserves scrolling.
- Presentation lifetimes report one dismissal reason, including direct binding
  changes, hidden/disabled/clipped/removed owners, deactivation and host/native
  unmount. Callbacks may remove/reopen content; coalesced dismiss/show has a fresh
  identity. Escape cancels preedit first. Outside presses consume their release.
  Attached context menus resolve the actual nearest owner for secondary/Control
  click and Shift-F10, including editor responders and later key modifiers.
- Configurable tooltips support hover/focus/noninteractive owners, wrapping/fitting,
  suppression, accessible help before display and dismissal callbacks. Delayed work
  cancels when invalid and settles when shown; geometry uses logical window points.
- Evidence: twelve focused portable overlay tests plus existing menu/presentation
  coverage and a Skia pixel regression proving nested portals cover owner scrollbars;
  native marked-text Escape, help, editor Shift-F10, nested return focus,
  fast open/dismiss and exactly-once unmount callbacks pass. Workspace tests,
  strict Clippy, portable compilation and example smoke pass; dark/light
  `m3_overlay` previews inspected. An additional secure-input rerun could not
  satisfy its awake foreground-desktop prerequisite; it is unavailable evidence,
  not a pass. The prior M2 ownership check remains recorded, and M8 still requires
  final native regression evidence. Interactive auditing remains M5.
- M3.4: `Tabs`/`Tab` retain visited keyed page trees, including editor history,
  selection, scroll offsets and focus; `Tab::lazy` defers unvisited page construction.
  Hidden virtual lists keep their viewport/range so retained row editors survive.
  Inactive pages are inert with zero geometry, hidden work cancels, overlays close,
  external values refresh, reorder retains IDs and removal releases host state.
  Empty/disabled/invalid selections have a visible fallback without source mutation.
- Headers form one Tab stop, with arrows/Home/End and nested Control-(Shift)-Tab;
  Tab enters remembered page content, explicit selection commits/revalidates preedit,
  external changes cancel it, and source setters may reject/normalize selection.
  Focus restoration respects outside focus. Overflow strips scroll horizontally,
  reveal focused headers and resize without overriding manual wheel movement.
- Evidence: ten focused portable interaction/layout/lifetime tests, legacy helper
  and macro coverage; native tab-group/radio-tab roles, selected children/value,
  active page contents, disabled/stale actions, IME commit/retained undo and native
  Control-Tab pass. Workspace tests, strict Clippy and portable compilation pass;
  dark/light `m3_tabs` previews inspected and native smoke passes. Final interactive
  auditing remains M5.
- M3.5: `Wizard`/`WizardStep` provide stable-ID flow definitions, pure reactive
  validation, guarded Next/Finish and independent Back/review/final-validation
  policies. Finish/cancel/change callbacks run outside host borrows and only accepted
  binding transitions report changes; normalized/rejected writes and reentrancy are
  covered. `set_steps` validates atomically; empty/unknown/removed/current IDs have
  defined fallback behavior without render-time source mutation.
- `wizard_view` supplies retained page bodies and `wizard_navigation` supplies only
  controls. The shared page stack preserves history, selection, scroll and focus
  without tab-strip shortcuts. Navigation commits preedit and rechecks validation;
  old action elements become inert after step changes, and captured tokens recheck
  nonreactive/live binding state. Cancel does not require forward validity; the
  application owns completion presentation and form reset/discard.
- Evidence: nine focused portable model/input/lifetime tests plus the existing tab
  regression suite; native validation feedback, enabled/stale actions, IME commit,
  accepted-change and finish/cancel callbacks pass. Workspace tests, strict Clippy,
  portable compilation, dark/light inspected `m3_wizard` previews and native smoke
  pass. Final interactive auditing remains M5.
- M3.6 remains open.


## M4 — Collections and structured editing

- [ ] **M4.1 List interaction:** anchor/range and additive selection, disabled-row skipping, activation/typeahead, configurable overscan and keyboard reveal while logical keys survive sorting and replacement.
- [ ] **M4.2 Virtual edit lifetime:** bounded realization/retention on large data; retain or explicitly pin active row edits through scrolling, and make removed/reinserted item lifetimes distinct.
- [ ] **M4.3 Tree model:** integrated lazy loading with expandable/loading/error metadata and safe model replacement; validate keys across collapsed branches and do not silently discard expansion errors.
- [ ] **M4.4 Tree interaction:** left/right expand/collapse and parent/child navigation, activation, selection and focus retention when branches change or disappear.
- [ ] **M4.5 Combo box:** separate query and selection, selection/free-text modes, filtering, loading/empty states, keyboard highlight/accept/cancel and accessible popup relationships; replace the fixed suggestion cap with bounded realization where needed.
- [ ] **M4.6 Table model and columns:** keyed columns, width bounds, resize/reorder, sort descriptors and row/cell selection modes. Keep selection attached to logical keys rather than indices.
- [ ] **M4.7 Table editing:** integrated cell activation/edit/commit/cancel, validation and readonly/custom content policies; preserve drafts through scrolling and reject stale commits after row/column/model changes.

Evidence: evolve the large collection example rather than building a second engine;
selection, loading, reordering, edit-retention/conflict and bounded-resource tests.
Main implementation: [collections](../rust/components/src/collections.rs).

Completion record: pending.

## M5 — Accessibility semantics and native behavior

Implement semantics alongside M1–M4; this milestone closes their combined audit.

- [ ] **M5.1 Tree semantics:** hierarchy, disclosure level, expanded/loading state and expand/collapse actions for logical offscreen items, with correct parent and selection relationships.
- [ ] **M5.2 Table semantics:** columns, cells, headers, row/cell selection, editing and sort information; indexed queries must remain bounded and avoid creating the full table.
- [ ] **M5.3 Control/presentation semantics:** correct menu, tab, split, alert, validation, help and value roles/states/actions, rather than exposing every composition merely as generic groups/buttons.
- [ ] **M5.4 Lifecycle/notifications:** stable logical identity, native focus/selection/value/layout notifications and coordinate conversion; retained objects become inert after removal, unmount or close, including nested presentations.
- [ ] **M5.5 Interactive audit:** record actual VoiceOver and IME sessions covering navigation, editing, popup/sheet transitions, password reveal and candidate placement at both backing scales. Fix issues found; keep unavailable checks open.

Evidence: semantic snapshot tests, native protocol/action tests and a short manual
results record. Main implementation: [native semantics](../rust/src/macos/components/accessibility.rs)
and [logical row proxies](../rust/src/macos/components/collection_accessibility.rs).

Completion record: pending.

## M6 — Localization and complete value controls

- [ ] **M6.1 Editable format policy:** specify supported locale/grammar combinations, display/edit round trips and unsupported-format behavior. Preserve an explicit ASCII/ISO mode; do not silently guess ambiguous grouping or dates.
- [ ] **M6.2 Number/date localization:** integrate the chosen Rust services with representative decimal-comma, grouped and non-Latin-digit cases; localized labels/formatting must match the advertised editable grammar and retain exact integers.
- [ ] **M6.3 Date control:** bounds, disabled dates, custom validation, readonly/blur/submit policy, calendar keyboard navigation and synchronization after external value changes.
- [ ] **M6.4 Time and datetime controls:** dedicated hour/minute/second interactions, 12/24-hour policy, step sizes, optional seconds, local/instant bounds and combined validation; verify DST gaps/overlaps and named-zone changes without corrupting drafts.
- [ ] **M6.5 Color control:** HSV as well as RGB, optional alpha, readonly and explicit transactional/live-preview behavior; cancel restores the source color, including through a color-well popup.
- [ ] **M6.6 Shared draft lifecycle:** consistent apply/blur/cancel, dirty/incomplete/invalid/out-of-range reporting, external-change handling and callbacks across all typed value controls.

Evidence: extend the value lab; round-trip, calendar-boundary, exact-value,
validation/transaction and timezone tests. Main implementation:
[values](../rust/components/src/values.rs), [drafts](../rust/components/src/draft.rs),
[Unicode/localization services](../rust/components/src/text.rs).

Completion record: pending.

## M7 — Developer tools and application ergonomics

- [ ] **M7.1 Inspector:** a navigable mounted hierarchy with layout, focus, selection, validation, overlay and collection-window details, synchronized with picking and removal.
- [ ] **M7.2 Overrides:** useful layout/style controls beyond width, clear indication of overridden values, reset and accurate measurement/clip feedback; preserve coherent IDs and restore original behavior after reset.
- [ ] **M7.3 Lab and ergonomics:** cover the full inventory in both themes and constrained sizes; demonstrate stateful custom components with concise macros/builders and no routine application-side reference-counting or native plumbing.

Evidence: inspector/Lab examples and model/geometry tests, plus a small external
application compile check using a renamed `scars-window` dependency.

Completion record: pending.

## M8 — Release evidence and documentation

- [ ] **M8.1 Native rendering:** rerun visible Metal pixel, lifecycle and full-screen suites on an awake foreground desktop; record failures accurately and fix genuine regressions.
- [ ] **M8.2 Visual/interaction matrix:** reproducible dark/light previews at 1×/2×, narrow/wide sizes, disabled/readonly/error/focused states and representative long/RTL/emoji text.
- [ ] **M8.3 Repeatable checks:** one documented local validation entry point and suitable CI checks for formatting, strict Clippy, core/macro/raster tests, external consumer compilation and portable-core compilation; native interactive checks remain explicit.
- [ ] **M8.4 Resource behavior:** measure large collections and long editing sessions for retained objects/caches and idle wakeups; verify inactive/hidden/unmounted controls cancel work and settle.
- [ ] **M8.5 Regression examples:** run the themed todo app and all milestone examples after the final interfaces settle; verify editing, filtering, persistence and external value updates still work.
- [ ] **M8.6 Inventory sign-off:** close every in-scope inventory row and checklist item with evidence, update public docs/examples and remove stale completion claims. Record remaining optional extensions separately before completing the goal.

Completion record: pending.

## Explicit boundaries and follow-up extensions

Windows/Linux native backends, pixel/API compatibility with C++, features exclusive
to the older UI engine, wide-gamut color management and variable-height collection
virtualization are outside this closure goal. The current SCARS reference uses
fixed row heights. These extensions must not be confused with missing reference
behavior, such as horizontal scrolling, HSV editing or table cell semantics.

Native desktop evidence is a real completion requirement. The previous GPU rerun
reported an inactive application and an occluded window; protocol tests passed,
but that did not establish a visible-frame pass. If hardware, permissions or an
interactive session prevents an audit, identify the exact remaining evidence and
request the necessary assistance rather than checking it off.
