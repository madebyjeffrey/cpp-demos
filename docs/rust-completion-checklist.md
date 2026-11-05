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
| State and descriptions | Reactive bindings, keys, macro conditions/repetition, host identity | Complete inventory mapping and ergonomic coverage of the remaining behaviors |
| Layout and basic controls | Themes, containers, basic controls, Skia text/images | Two-axis scrolling, fuller styling/text options and interaction hardening |
| Editing | Grapheme-safe edits, history, composition, secure fields, multiline text | Visual navigation, selection gestures, specialized field policies and reusable buffer interface |
| Presentation | Popups, focus scopes, simple menus, tabs, slider-controlled split, basic alerts | Complete interactions, retained page state and presentation transactions |
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
- [ ] **M1.2 Style coverage:** finish per-edge insets, flex basis/self-alignment/spacing, relative positioning, text alignment/overflow/line limits, label styles, accent/density and separator orientation where the inventory requires them.
- [ ] **M1.3 Layout correctness:** exercise wrapping/reflow, grow/shrink redistribution at min/max limits, percentages, baselines, clipping and layered hit order under constrained sizes.
- [ ] **M1.4 Two-axis scrolling:** horizontal/vertical/both axes, visible draggable scrollbars, wheel/trackpad transport and programmatic offset/query interfaces with clamping.
- [ ] **M1.5 Scroll/input lifetime:** nested wheel propagation, two-axis focus/caret reveal, keyboard scrolling, drag autoscroll and capture cancellation on blur, hide, removal and unmount.

Evidence: extend the layout/scroll examples; deterministic geometry, edge propagation
and cancellation tests; dark/light constrained previews. Likely changes:
[geometry/layout](../rust/components/src/layout.rs),
[input](../rust/components/src/input.rs), [native adapter](../rust/src/macos/components/input.rs).

Completion record (in progress):

- M1.1: [inventory matrix](rust-inventory.md) maps 162 exported types and all 47
  view/annotation names to existing behavior or explicit closure items.
  `python3 scripts/check_rust_inventory.py` verifies name coverage, not behavior.
- M1.2/M1.3 checkpoint: per-edge insets, flex basis/self-alignment/space-around,
  min/max redistribution, relative offsets, inherited accent/density, text roles,
  line limits/ellipsis/alignment/spacing and vertical separators implemented.
  Example: `cargo run --example m1_layout` (also `--light` / `--render file.png`).
  Evidence: four portable style/geometry tests plus a real SkParagraph limits,
  alignment and cache test; existing workspace tests and strict Clippy pass.
- M1 remains open: rich inline text/box behavior, complete wrapping/reflow and
  baseline cases, two-axis scrolling/scrollbars and input lifetime closure still
  require implementation and evidence. The other M1 checkboxes remain unchecked.


## M2 — Editing and specialized fields

- [ ] **M2.1 Shared text model:** provide a reusable Rust text-buffer interface for selection, committed text, composition, history and external replacement without requiring a native window. Retain existing grapheme/UTF-16 guarantees.
- [ ] **M2.2 Navigation:** visual up/down and line/page movement through wrapped text, preferred horizontal caret position, word/line deletion and selection extension; distinguish line boundaries from document boundaries.
- [ ] **M2.3 Selection and history:** word/line selection gestures, drag selection outside the viewport, appropriate typing coalescing and history limits; composition, readonly and grapheme-counted maximum-length cases.
- [ ] **M2.4 Search field:** integrated clear/cancel, query/submit callbacks and configurable debounce. Cancel pending work on replacement, removal and deactivation without an idle animation loop.
- [ ] **M2.5 Password policy:** explicit reveal, undo, readonly and length options; verify masking, clipboard/accessibility restrictions, composition and balanced native secure-input ownership through every transition.
- [ ] **M2.6 Numeric/stepper policy:** standalone stepper, configurable precision/validation states, blur/submit/cancel policy and press-and-hold repeat deadlines; preserve exact integer arithmetic and invalid/incomplete drafts.

Evidence: extend the editor example; Unicode/visual navigation and IME/history tests;
native clipboard/candidate-geometry checks. Main implementation:
[editing](../rust/components/src/editing.rs), [drafts](../rust/components/src/draft.rs).

Completion record: pending.

## M3 — Navigation and presentation

- [ ] **M3.1 Split views:** draggable dividers, both orientations, pane limits, collapse/restore and keyboard adjustment; resize cursors, capture cancellation and accessible split semantics.
- [ ] **M3.2 Menus and split actions:** separators, checked/radio items, nested submenus, shortcut labels, disabled-item navigation, typeahead and complete open/close/activation behavior; a split action can retain its chosen primary action.
- [ ] **M3.3 Overlay policy:** configurable placement, sizing, viewport fitting, focus preservation and dismissal callbacks for popovers/tooltips/context menus, including nested scopes and owner removal.
- [ ] **M3.4 Tabs:** stable page identity, enabled tabs, keyboard navigation and restoration of editor history, selection, scroll and focus after switching pages. Application model state alone does not satisfy this item.
- [ ] **M3.5 Wizards:** stable step IDs, validation that can block navigation, back/review policy, finish/cancel/change callbacks and valid behavior for empty/changed step sets.
- [ ] **M3.6 Alerts and sheets:** an explicit result/completion transaction with default/cancel/destructive actions; exactly-once completion through action, Escape, dismiss, close and destruction. Demonstrate native parent-sheet isolation and focus restoration.

Evidence: extend the workspace example with a nested menu, retained tab editor and
confirmation sheet; focused interaction/lifecycle tests. Main implementation:
[presentation](../rust/components/src/presentation.rs), [native mounting](../rust/src/macos/components/mod.rs).

Completion record: pending.

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
