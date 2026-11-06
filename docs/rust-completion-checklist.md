# Rust SCARS completion report and checklist

Baseline: 2026-11-05, `82583e7`. Current release audit: 2026-11-06.

The Rust workspace is a usable first implementation across the six original
milestones. It is **not yet complete against their exit criteria**. This report
is the working checklist for closing that gap. A compiling example, a composition
of simpler controls, or an available native protocol does not by itself establish
that the corresponding behavior is finished.

## Scope and working rules

The user confirmed **macOS components and the portable core**. Native Windows and
Linux window backends are outside this goal. M5.5 is explicitly deferred to
[`rust-todo.md`](rust-todo.md) by the user and is not a gate for the current M8
close-out; it remains incomplete and must not be reported as passed. SCARS behavior is the reference;
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
| State and descriptions | Reactive bindings, keys, macro conditions/repetition, host identity and inventory mapping | Final inventory/API sign-off in M8.6 |
| Layout and basic controls | Inventory, constrained layout, rich text, style coverage and two-axis scrolling verified | Final cross-component visual/semantic audit in M5/M8 |
| Editing | Shared buffer, visual navigation/gestures, localized typed drafts, search/password policies and held steppers | Final interactive VoiceOver/IME audit in M5 |
| Presentation | Retained splits/tabs/wizards, hierarchical menus, configured portals, transactional alerts and native sheets | Final interactive and visible desktop audits in M5/M8 |
| Collections | Virtual lists, lazy trees, combo boxes and keyed tables with retained validated edits | Combined native semantics and interactive audits |
| Values and tooling | Shared typed draft lifecycle, RGB/HSV/hex, calendars, localized civil/zoned editing, mounted inspector/property overrides and box/flex feedback | Final visual/interaction evidence in M8 |
| Verification | Repeatable local/CI checks, core/CPU/native-protocol tests, example matrix and minimum-Rust compilation | Interactive audits, final visible GPU/full-screen/idle/example runs and inventory sign-off |

The [release audit](rust-release-audit.md) reconciles all 44 entries: 41 complete,
two open M8 gates, and M5.5 deferred. The inventory is implementation coverage;
it does not close native or interactive verification automatically.

The main reference is the [current C++ public inventory](../scars/src/scars.cppm),
including options and modifiers, not only control names. The
[original migration sequence](rust-components-feasibility.md#suggested-migration-sequence-and-exit-criteria)
defines the intended evidence. Existing Rust contracts are in the
[component README](../rust/crates/scars-ui/README.md).

## M1 — Inventory, layout and scrolling

- [x] **M1.1 Inventory closure:** map every current SCARS control, model, modifier and option family to an implemented Rust equivalent or a remaining checklist item. Document intentional interface changes; no unexplained omissions.
- [x] **M1.2 Style coverage:** finish per-edge insets, flex basis/self-alignment/spacing, relative positioning, text alignment/overflow/line limits, label styles, accent/density and separator orientation where the inventory requires them.
- [x] **M1.3 Layout correctness:** exercise wrapping/reflow, grow/shrink redistribution at min/max limits, percentages, baselines, clipping and layered hit order under constrained sizes.
- [x] **M1.4 Two-axis scrolling:** horizontal/vertical/both axes, visible draggable scrollbars, wheel/trackpad transport and programmatic offset/query interfaces with clamping.
- [x] **M1.5 Scroll/input lifetime:** nested wheel propagation, two-axis focus/caret reveal, keyboard scrolling, drag autoscroll and capture cancellation on blur, hide, removal and unmount.

Evidence: extend the layout/scroll examples; deterministic geometry, edge propagation
and cancellation tests; dark/light constrained previews. Likely changes:
[geometry/layout](../rust/crates/scars-ui/src/layout.rs),
[input](../rust/crates/scars-ui/src/input.rs), [native adapter](../rust/crates/scars-window/src/macos/components/input.rs).

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
[editing](../rust/crates/scars-ui/src/editing.rs), [drafts](../rust/crates/scars-ui/src/draft.rs).

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
- [x] **M3.6 Alerts and sheets:** an explicit result/completion transaction with default/cancel/destructive actions; exactly-once completion through action, Escape, dismiss, close and destruction. Demonstrate native parent-sheet isolation and focus restoration.

Evidence: extend the workspace example with a nested menu, retained tab editor and
confirmation sheet; focused interaction/lifecycle tests. Main implementation:
[presentation](../rust/crates/scars-ui/src/presentation.rs), [native mounting](../rust/crates/scars-window/src/macos/components/mod.rs).

Completion record:

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
- M3.5 (`eb69c35`): `Wizard`/`WizardStep` provide stable-ID flow definitions, pure reactive
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
- M3.6: single-use `Alert`/`AlertResult` transactions validate stable action IDs
  and default/cancel/destructive roles. Result publication precedes exactly-once
  completion; action, Escape, programmatic dismissal, close, owner removal, unmount
  and destruction share that gate. Return only invokes the enabled default; marked
  text commits first without submission, and multiline editing keeps Return.
- `Surface::show_sheet(alert, body_builder)` retains a native parent-modal dialog,
  releases it before application completion, blocks parent commands/input/retained
  accessibility actions and restores the native responder and logical focus.
  `view_with_surface` supplies component action context without weak-handle setup.
  Native default/cancel accessibility attributes and stale-object guards are covered.
- Evidence: seven portable transaction/input/lifetime tests; dedicated native sheet
  checks cover IME, actions, isolation, actual native-responder restoration, Escape,
  dismissal, unmount, close, parent destruction and application teardown. Existing
  native component protocol tests pass. Workspace tests, strict Clippy and portable
  compilation pass. `m3_alerts` demonstrates overlays and `--sheet` demonstrates
  native sheets; dark/light overlay and sheet-content previews inspected, native
  sheet smoke passes.
- Verification limits: the foreground key-window audit could not establish another
  key window; it is **unverified**, not a pass. Re-run
  `SCARS_WINDOW_DESKTOP_TESTS=1 cargo test --test sheets -- --foreground` on an active
  desktop in M8. The broader native desktop suite also stopped at its visible Metal
  frame prerequisite (`active=false`, occlusion 8192). M5 interactive audits and
  M8 visible rendering/activation evidence remain open.


## M4 — Collections and structured editing

- [x] **M4.1 List interaction:** anchor/range and additive selection, disabled-row skipping, activation/typeahead, configurable overscan and keyboard reveal while logical keys survive sorting and replacement.
- [x] **M4.2 Virtual edit lifetime:** bounded realization/retention on large data; retain or explicitly pin active row edits through scrolling, and make removed/reinserted item lifetimes distinct.
- [x] **M4.3 Tree model:** integrated lazy loading with expandable/loading/error metadata and safe model replacement; validate keys across collapsed branches and do not silently discard expansion errors.
- [x] **M4.4 Tree interaction:** left/right expand/collapse and parent/child navigation, activation, selection and focus retention when branches change or disappear.
- [x] **M4.5 Combo box:** separate query and selection, selection/free-text modes, filtering, loading/empty states, keyboard highlight/accept/cancel and accessible popup relationships; replace the fixed suggestion cap with bounded realization where needed.
- [x] **M4.6 Table model and columns:** keyed columns, width bounds, resize/reorder, sort descriptors and row/cell selection modes. Keep selection attached to logical keys rather than indices.
- [x] **M4.7 Table editing:** integrated cell activation/edit/commit/cancel, validation and readonly/custom content policies; preserve drafts through scrolling and reject stale commits after row/column/model changes.

Evidence: evolve the large collection example rather than building a second engine;
selection, loading, reordering, edit-retention/conflict and bounded-resource tests.
Main implementation: [collections](../rust/crates/scars-ui/src/collections.rs).

Completion record (in progress):

- M4.1 (`e1adbcc`): `ListOptions`/`ListSelection` cover configurable overscan/preferred rows,
  replace/toggle/range/add-range/focus-only operations, a stable anchor and cursor,
  select-all/clear, disabled-row skipping and activation callbacks. Shared model
  options replace the previous configure-before-cloning restriction. Atomic item
  validation preserves keys/tokens/selection; reordering restores logical focus.
- Lists have one row Tab stop, viewport-sized Page navigation, keyboard reveal,
  Unicode lowercase prefix typeahead with cycling/timeout and Return/double-click
  activation. Native printable keys and Select All use the same core behavior.
  Manual wheel scrolling does not snap to an unrealized focused row; subsequent
  keyboard navigation reveals its logical target. Selection queries avoid cloning
  the full selected set during per-row rendering/accessibility.
- Evidence: seven focused portable interaction/model/resource tests plus existing
  collection/tab regressions; native Shift-arrow disabled skipping, typeahead,
  activation, Select All and retained indexed proxies through sorting pass.
  Workspace tests, strict Clippy and portable compilation pass. `m4_list` provides
  a 100,000-row example; inspected dark/light previews and native smoke pass.
  Active row editor retention/lifetime separation is covered in M4.2 below; M5/M8
  interactive and visible desktop audits remain open.

- M4.2 (`d46a720`): `ListModel::pin`/`RowPin` retain explicitly active edits alongside the
  viewport/overscan range. The configurable limit defaults to one distinct row;
  clones share a slot, invalid/over-limit requests fail and last-handle release
  prunes offscreen controls. Geometry preserves the full indexed scroll extent.
  Pins invalidate on removal/disable and weak ownership does not retain datasets.
- Host identity includes the item token, so removed/reinserted keys and replacement
  models receive new editor/AX lifetimes even when updates coalesce before a frame.
  Reordering preserves live editor IDs, selection, history and preedit. Caret reveal
  synchronizes host/model offsets; plain wheel scrolling does not snap back.
  Resource cleanup defers across pure construction and safely runs on unwinding.
  `on_activate_with_model` avoids strong self-captures in edit-start callbacks.
- Evidence: eight focused portable pin/lifetime/input tests and one internal test
  checking actual editor-map bounds across 200 jumps through 100,000 rows. Existing
  list/tab/scroll/foundation/wizard regressions pass. Native component checks retain
  offscreen preedit, commit/undo and reject stale retained AX editors after
  reinsertion. Native caches prune without needing a visible draw frame.
  Workspace tests, strict Clippy and portable compilation pass; `m4_list_edit`
  provides the runnable example, inspected dark/light previews and native smoke.
  Actual interactive VoiceOver/IME placement and visible GPU audits remain M5/M8.

- M4.3 (`53db9dd`): `TreeNode` describes eager/lazy/deferred sources and `TreeItem` reports
  known-node metadata, including parent/depth/enabled/expandable/loading/error state.
  All known keys validate across collapsed branches before structural replacement.
  Pure synchronous loaders cache results; failures/panics and invalid lazy children
  remain visible and return errors. Retry is explicit; empty loaded branches become
  leaves unless explicitly expandable.
- `TreeLoad` carries weak ownership and node/request generations. Deferred callbacks
  run outside borrows; stale/duplicate results are inert after retry, subtree or
  whole-model replacement. Collapse permits background completion without reopening.
  Metadata and derived rows publish before schedulers run; old loader resources
  drop outside state borrows. Whole snapshots start fresh collapsed lifetimes.
- Evidence: eleven portable model/view/lifecycle tests, including a 2,501-node
  collapsed chain, hidden duplicates, failure/retry, stale nested requests, reentrant
  completion, scheduler consistency and resource teardown. Existing list/edit
  regressions and workspace tests pass; strict Clippy and portable compilation pass.
  Native component tests retire old row proxies after same-key snapshot replacement.
  `m4_tree` provides the runnable model example; dark/light previews inspected and
  native smoke passes. Tree interaction is covered below; hierarchical native
  semantics/interactive auditing remain M5.

- M4.4 (`9acaee0`): `TreeOptions` configures selection and bounded row geometry/overscan.
  Left/Right close/open branches or visit enabled parents/children; shared list
  range/additive selection, typeahead, reveal and activation remain available.
  Disclosure/retry controls focus their owning row, keeping one row Tab stop.
  Return/double-click invoke application activation or default branch toggle/retry.
- Collapse projects selected/focused descendants to a visible enabled ancestor,
  preserving unrelated selection. Removal falls back to an ancestor or nearby
  cursor row; fresh same-key snapshots retain logical focus with new host/AX IDs.
  Completion does not steal focus from another control. `reveal` opens known
  ancestors without selecting; expansion/activation callbacks permit model updates
  outside borrows, with nested notifications suppressed.
- Evidence: eight focused portable interaction tests include collapse/removal,
  deferred/error branches, callback replacement, modifier selection and bounded
  navigation through 100,000 children. Existing model/list/edit and workspace tests
  pass; strict Clippy and portable Linux compilation pass. Native key/AX protocol
  tests cover parent/child navigation, callbacks and stale proxy retirement.
  `m4_tree` demonstrates the interaction contracts; dark/light previews inspected
  and native smoke passes. Full hierarchy semantics and interactive audits remain M5.

- M4.5 (`798f7a0`): `ComboBox` separates the draft query, committed ID and highlighted result.
  Configured selection/free-text modes, contains/prefix/no filtering, disabled
  options, readonly/length policies, loading/empty text and row/popup heights replace
  the capped four-argument helper. `combo.view(placeholder)` owns field and portal.
- Keyboard highlight/reveal keeps focus in the field; Return or pointer/AX press
  accepts, composition commits first, and repeated native Return is suppressed.
  Escape/outside/blur/deactivate/removal restore the opening query without changing
  the choice; Tab closes and traverses normally. Callbacks run after publication,
  permit checked result replacement and suppress recursive notifications.
- Suggestions reuse list virtualization without a fixed cap. The opaque popup matches
  field width and fits/flips in the viewport. AXComboBox expansion, linked popup,
  parent/child and bounded highlighted-child relationships are exposed; indexed
  offscreen acceptance works and retained dismissed/unmounted objects become inert.
- Evidence: seven focused portable tests cover model policies, callbacks, replacement,
  composition, pointer/Tab/cancellation, geometry and bounded 100,000-option mounting.
  Workspace tests, strict Clippy and portable Linux compilation pass. Native component
  tests cover links, selection beyond the old cap, commands, repeated Return and stale
  objects. `m4_combo` supplies the runnable example; dark/light open-popup previews
  inspected and native smoke passes. Actual VoiceOver/IME auditing remains M5/M8.

- M4.6 (`26cae99`): `TableRow` stores text by column ID; checked `TableColumn`/`TableOptions`
  define width bounds, resizable/sortable/alignment and realization/selection policy.
  Width snapshots, resize and reorder preserve surviving column/row lifetimes.
  Width-only changes do not rebuild or sort the dataset. Multi-column descriptors
  apply a stable string sort; clearing restores source order. Callbacks run after
  publication outside borrows and suppress nested notifications.
- `TableSelection` holds row keys or `TableCell` pairs. Single/multiple selection,
  rectangular range/additive/toggle/focus-only operations, disabled-row skipping,
  select-all and mode changes share the existing list's row navigation/reveal.
  Reordering/sorting retain logical focus and selection; column removal/reinsertion
  receives new control IDs, including equal-text changes. Drafts use logical keys
  and check row/column lifetimes plus original values.
- Pointer header drag and resize grips have keyboard equivalents; Escape/Cancel
  rollback, bounds, native resize cursors and capture invalidation are covered.
  A horizontal outer scroller keeps header/body geometry aligned, with one roving
  body row/cell Tab stop and bounded virtual row realization.
- Evidence: eight focused tests cover validation, multi-sort, rectangular selection,
  geometry/navigation, pointer/keyboard/capture, mode changes, stale identities and
  100,000-row realization. Workspace tests, strict Clippy and portable compilation
  pass. Native component tests cover header sorting, actual key-driven cell movement,
  retained/replaced focus and resize commands. `m4_table` supplies the runnable
  example; dark/light previews inspected and native smoke passes. Integrated editing
  is covered below; complete native table semantics/interactive audits remain M5/M8.

- M4.7 (`1378a01`): column editing is opt-in, with multiline/length policy, pure validation and
  custom display builders. Return/double-click activates; composition commits before
  submission, repeated Return does not submit, and Command/Control-Return saves
  multiline content. Escape cancels; Tab saves and traverses. Errors keep drafts
  visible with outline/help and table error text. Blur retains the draft; activating
  another editable cell validates the existing one before changing editors.
- One owned row pin preserves the shared buffer, undo, selection and preedit through
  scrolling/sort/order changes. Weak `TableEdit` handles and fresh editor IDs prevent
  stale actions. Row/column/policy generations and per-cell value versions reject
  stale/equal-text ABA commits without blocking unrelated cell changes. Structural
  retirement, disable, model replacement, unmount and native detach release the edit,
  including when an old Host is retained. Commit callbacks run after publication and
  pin release and may start the next edit.
- `set_text` applies authoritative data without a user commit event and exposes active
  conflicts; `reveal_row`/`realized_row_count` avoid application-side list wiring.
  `m4_table_edit` demonstrates 100,000 rows, validation, multiline and readonly custom
  display, retained scrolling and remote conflicts (`--invalid` previews validation).
- Evidence: seven focused editing tests plus existing table/collection regressions,
  covering versions/validation, purity, pointer/keyboard/Tab, Unicode length, disabled
  and replaced data/policies/models, reentrant commit and bounded retained editing.
  Workspace tests, strict Clippy and portable compilation pass. Native tests preserve
  marked text through scroll/sort/reorder, separate composition from commit, reject
  repeated Return/stale objects and cover validation/cancel, actual Command/Control-
  Return and detach with a retained host. Dark/light and invalid previews inspected;
  native smoke passes. Full table semantics and actual VoiceOver/IME audits remain M5.

## M5 — Accessibility semantics and native behavior

Implement semantics alongside M1–M4; this milestone closes their combined audit.

- [x] **M5.1 Tree semantics:** hierarchy, disclosure level, expanded/loading state and expand/collapse actions for logical offscreen items, with correct parent and selection relationships.
- [x] **M5.2 Table semantics:** columns, cells, headers, row/cell selection, editing and sort information; indexed queries must remain bounded and avoid creating the full table.
- [x] **M5.3 Control/presentation semantics:** correct menu, tab, split, alert, validation, help and value roles/states/actions, rather than exposing every composition merely as generic groups/buttons.
- [x] **M5.4 Lifecycle/notifications:** stable logical identity, native focus/selection/value/layout notifications and coordinate conversion; retained objects become inert after removal, unmount or close, including nested presentations.
- [deferred] **M5.5 Interactive audit:** record actual VoiceOver and IME sessions covering navigation, editing, popup/sheet transitions, password reveal and candidate placement at both backing scales. Track it in [`rust-todo.md`](rust-todo.md); it is incomplete and outside this M8 close-out.

Evidence: semantic snapshot tests, native protocol/action tests and a short manual
results record. Main implementation: [native semantics](../rust/crates/scars-window/src/macos/components/accessibility.rs)
and [logical row proxies](../rust/crates/scars-window/src/macos/components/collection_accessibility.rs).

Completion record:

- M5.1 (`5ec5034`): tree views expose AXOutline/AXRow semantics with logical parent/direct-child
  links, disclosure levels, selected/enabled/focused state, accessible labels and
  loading/failure descriptions. Selection and disclosure actions validate mount,
  modal scope, effective enabled state and lifetime tokens; offscreen actions do not
  require realizing the target widget. Retry buttons belong to the logical row.
- Immutable shared snapshots back lazy native row arrays. Counts/copy do not allocate
  every proxy; requested entries preserve identity across row arrays, focus and hit
  testing. Weak cache entries retire when proxies deallocate. Collapsed rows become
  inert, reopening preserves surviving identity, and replaced/unmounted rows reject
  stale actions. Snapshots do not retain loader callbacks or application resources.
- Evidence: five core semantic tests cover hierarchy, snapshots, atomic selection,
  modal scope, offscreen loading/retry and resource release. Dedicated AppKit tests
  cover actual protocol attributes/actions, disabled ancestors, selection/focus/hit
  identity, stale objects, a 100,000-child outline and 200 transient queries returning
  the weak cache to its original size. Existing native component regressions,
  workspace tests, strict Clippy, formatting and portable compilation pass.
  `m4_tree` is the runnable example; `--semantics` prints portable semantic state.
  Dark/light previews inspected and native smoke passes. Combined notification and
  interactive VoiceOver/IME audits remain open under M5.4/M5.5.

- M5.2 (`2569c73`): one AXTable exposes keyed rows/cells/columns, a header group, header links,
  live indexes, row/cell selection, sort direction/precedence and validated editing.
  Ordinary child traversal and table-specific attributes share proxies. Resize and
  custom controls remain actionable; native focus/hit testing preserves custom
  controls and editor text APIs. Internal scroll/layout scaffolding is not exposed
  as a competing table. Independent column selection is explicitly unsupported.
- Callback-free immutable `TableSnapshot` metadata backs lazy native arrays, including
  column-cell arrays and full child hierarchies. Count/copy and bounded range queries
  do not create every proxy or widget. Shared weak identities survive sort/reorder;
  replacement/unmount retires old objects. Querying offscreen cells does not realize
  them; explicit focus/editing reveals only the target row. Bulk selection validates
  all targets before mutation and preserves outside focus. Mutations respect enabled
  ancestors/modal scope and recheck lifetimes after reentrant commit callbacks.
- Evidence: five portable semantic tests plus table/edit regressions cover immutable
  data/order/tokens, callback-resource release, multi-sort metadata, atomic selection,
  100,000-row offscreen edits, validation and stale/reentrant actions. AppKit tests
  verify the actual hierarchy/roles, lazy count/copy/ranges, linked identities,
  index changes, row/cell modes, custom focus/hit testing, editor parentage/errors,
  readonly/disabled guards and 200 transient queries returning cache size to baseline.
  Reference AppKit table children were inspected to verify the header-group mapping.
  Existing component/tree native suites, workspace tests, strict Clippy, formatting,
  inventory mapping and portable compilation pass. `m4_table --semantics` provides
  the portable example; `m4_table` and `m4_table_edit` provide native demonstrations.
  Dark/light previews inspected and native example smoke passes. Actual interactive
  VoiceOver/IME and combined notifications remain M5.4/M5.5.

- M5.3 (`7461f52`): fields/editors, toggles/checks/radios/segments, steppers/sliders, tabs,
  splits, menus, dialogs/alerts, popovers and color wells expose their native
  roles/subroles and current values/actions. Stepper button links and directional
  availability, split bounds/adjacent contents, choice selection and alert
  default/cancel semantics use guarded host actions. Readonly controls cannot
  mutate through native press/value assignment; real table text editors retain
  their native value setter and shared input/validation path.
- `accessible_help` and `validation` work in builders/macros; numeric/value fields
  attach validation automatically. Standard help/value descriptions carry validation
  and instructions while snapshots preserve typed status/dirty/message metadata.
  Color channels and civil fields have explicit labels; color wells carry their
  value. Expanded submenu panels have their item as parent. Menu openers remain
  readable/linked during their own menu; show is idempotent, cancellation closes
  submenu levels first and respects active scope and Escape policy. Parent/child
  enumeration uses consistent readable scope.
- Evidence: five focused core semantic tests plus existing control/numeric/value/
  menu/tab/split tests; dedicated AppKit tests verify roles/descriptions, exact
  values, validation/help, choice setters, bounds/actions, menu hierarchy/owner
  identity, dialog policies and color wells. Existing native component/tree/table
  suites pass, including marked table edits and a regression asserting the editor's
  value setter remains available. Native sheet action/input/ownership tests pass;
  their foreground key-window audit was explicitly not run. Workspace tests,
  strict Clippy, portable compilation, formatting and inventory mapping pass.
  `m5_semantics` supplies concise settings composition and a `--semantics` dump;
  dark/light previews inspected and native smoke passes. Count/range queries for
  tabs, selected children and splitters match their direct getters in native tests.
  Actual interactive/foreground and combined notification audits remain M5.4/M5.5/M8.

- M5.4 (`5133b26`): generic proxies survive hidden retained pages, use globally unique native
  identifiers and retire on actual removal/unmount/close. Nested dialogs preserve
  owner identity while suppressing inactive actions. Hit testing shares readable
  modal scope; custom list controls retain logical row parentage and their own focus.
- Main-runloop posting is coalesced independently of drawing. Bounded snapshots and
  constant-space collection stamps distinguish same-count row/cell selection,
  logical focus, content, disclosure, row counts, text values/ranges/composition,
  title/geometry and destruction. Layout changes carry the standard changed-element
  payload. Same-length password edits are detected without retained plaintext in
  snapshots/traces. Retired targets receive destruction once; pending work cancels
  on teardown. Released logical proxies remove their snapshot entries.
- Evidence: three portable version tests, dedicated native lifecycle/notification
  suites, existing native component/control/tree/table/sheet regressions, workspace
  tests, strict Clippy, formatting, inventory mapping and portable Linux compilation
  pass. A 100,000-row native list stays below 200 snapshot entries; 200 transient
  range queries return that count to baseline. Native move/resize and UTF-16 emoji
  coordinate round trips pass at actual backing scale 1. `m5_semantics` supplies the
  runnable example; existing dark/light previews apply. The earlier identity/posting
  checkpoint is `4df142a`. This proves AppKit emission, not external-client receipt.

M5.5 interactive audit is deferred by user request; its speech/IME work remains
incomplete. An earlier readiness probe reported an active screen and display-link
callbacks, but no application/key-window ownership:

```text
active-displays=1 display-error=0 front=com.utmapp.UTM app-active=0 key-window=0 visible=1 callbacks=72 ready=0
```

Repeat on a foreground desktop at actual backing scales 1 and 2, recording scale,
input source, actions and observed speech/candidate behavior:

| Session | Runnable example | Required observation | Status |
| --- | --- | --- | --- |
| Navigation, selection and disclosure | `m4_tree`, `m4_table` | VoiceOver announces actual logical focus, selection, expansion and sort | Pending |
| Editing, validation and password reveal | `m2_fields`, `m4_table_edit` | Speech reflects edits/errors without exposing masked secrets | Pending |
| Popup and nested presentation transitions | `m5_semantics`, `m4_combo`, native sheets | Correct focus restoration, readable scope and announcement | Pending |
| Actual input-method composition | `m2_editor`, `m4_table_edit`, native sheets | Candidate window follows caret through move/resize/scroll; commit/cancel work | Pending |

Run examples with `cargo run --example NAME --locked --offline` from `rust/`.
The sheet foreground harness is `SCARS_WINDOW_DESKTOP_TESTS=1 cargo test --test sheets --locked --offline -- --foreground`.
Protocol composition tests and synthetic raster scales do not close this audit.

User-assisted session, 2026-11-06: macOS 27.2, PA278CV at actual backing scale 1,
Canadian keyboard layout (system reads). The user reports the `m5_semantics`
settings navigation/focus-return check passed, but flagged verbose application
information: “collapsed” on Options and “selected collapsed tab 1 of 2.” Options
is a menu opener and legitimately has expansion state. Tabs must not expose it.
The native selector check reproduced that defect. A separate hidden-window test
also reproduced redundant initial text-selection and focus notifications on entry.
Both fixes pass the hidden native regression (`29362a5`); actual speech recheck remains pending. Other matrix
sessions, composing IME candidate placement and actual scale 2 remain open.

Follow-up: the running `m5_semantics` process was confirmed to have started before
`29362a5`. It was replaced with the rebuilt example and the hidden tab/focus
regression passed again. Speech from that restarted process is still awaiting the
user's report; the earlier utterance is not evidence against the rebuilt version.

The hidden native regression also checks both selected and unselected tabs before
and after a tab switch: selection updates while the expansion selector remains
unavailable. This protocol evidence does not substitute for the speech recheck.

A further protocol audit reproduced an unsupported inherited expansion setter
still being advertised by generic tab elements. Generic elements now reject
`setAccessibilityExpanded:`; menu expansion reads and guarded press/show actions
remain available, while logical tree rows keep their separately implemented
setter. The hidden tab/menu/focus regression passes. Rebuild/restart
`m5_semantics` before the actual speech recheck; this is not a VoiceOver pass.

## M6 — Localization and complete value controls

- [x] **M6.1 Editable format policy:** specify supported locale/grammar combinations, display/edit round trips and unsupported-format behavior. Preserve an explicit ASCII/ISO mode; do not silently guess ambiguous grouping or dates.
- [x] **M6.2 Number/date localization:** integrate the chosen Rust services with representative decimal-comma, grouped and non-Latin-digit cases; localized labels/formatting must match the advertised editable grammar and retain exact integers.
- [x] **M6.3 Date control:** bounds, disabled dates, custom validation, readonly/blur/submit policy, calendar keyboard navigation and synchronization after external value changes. The narrow calendar grid overflow was corrected in M6.2.
- [x] **M6.4 Time and datetime controls:** dedicated hour/minute/second interactions, 12/24-hour policy, step sizes, optional seconds, local/instant bounds and combined validation; verify DST gaps/overlaps and named-zone changes without corrupting drafts.
- [x] **M6.5 Color control:** HSV as well as RGB, optional alpha, readonly and explicit transactional/live-preview behavior; cancel restores the source color, including through a color-well popup.
- [x] **M6.6 Shared draft lifecycle:** consistent apply/blur/cancel, dirty/incomplete/invalid/out-of-range reporting, external-change handling and callbacks across all typed value controls.

Evidence: extend the value lab; round-trip, calendar-boundary, exact-value,
validation/transaction and timezone tests. Main implementation:
[values](../rust/crates/scars-ui/src/values.rs), [date control](../rust/crates/scars-ui/src/date.rs),
[drafts](../rust/crates/scars-ui/src/draft.rs),
[Unicode/localization services](../rust/crates/scars-ui/src/text.rs).

Completion record:

- M6.1 (`1a5fb39`): [editable formats](rust-editable-formats.md) defines default ASCII/ISO
  grammar, round trips, precision and unsupported-format behavior, plus the explicit
  locale profiles/strict grouping and digit policies for M6.2. Civil editors now
  reject discarded timestamp/offset/zone information and leap-second clamping;
  named-zone edits validate the civil portion before resolution. Generic drafts
  retain application-defined parsers. `m6_formats` is the concise runnable lab.
- Evidence: four format-policy tests cover exact signed/unsigned integers, ambiguous
  grouping/digits, date/time extremes/nanoseconds, suffix rejection and named-zone
  input. Existing numeric/value tests, workspace tests, strict Clippy, portable Linux
  compilation, formatting, inventory mapping and native component regressions pass.
  Dark/light `m6_formats` previews inspected; its native smoke and the existing
  `stage6` value-lab smoke pass. Interactive/visible GPU evidence remains M5.5/M8.

- M6.2: `EditLocale` is one checked setting shared by `Numeric` and `DatePicker`.
  Five explicit profiles cover decimal-comma, strict optional three-digit grouping,
  French narrow no-break spaces and Arabic-Indic digits/sign affixes. Unsupported
  profiles/extensions fail construction. ASCII/ISO remains the default. ICU4X
  supplies numeric output; integers parse/step without `f64`. Localized drafts,
  bounds/errors, rounded display and readonly behavior share the existing paths.
- Calendar field order/hints, month/week/action text, week start and Arabic grid
  direction match the profile. Gregorian years 0001–9999 are supported; ISO keeps
  the full year range. Calendar cells divide remaining row space after gaps,
  correcting the narrow grid overflow. The [format contract](rust-editable-formats.md)
  records exact separators, digit/sign rules and configuration-time draft reset.
- Evidence: six portable tests cover supported/unsupported profiles, exact integer
  limits, malformed/mixed input, float extremes/subnormals/signed zero, precision,
  stepping, readonly/external values, date order/year limits and narrow calendar
  geometry/actions. Native tests cover actual localized text, exact AX numbers,
  staged edits, stepping, submission and invalid date preservation. Existing native
  input/control/notification suites, workspace tests, strict Clippy, formatting,
  inventory mapping and portable Linux compilation pass. `m6_locales` provides the
  concise runnable lab; all five pages inspected in dark/light, native smoke passes.
  Interactive VoiceOver/IME and final visible GPU evidence remain M5.5/M8.

M6.3 completion record (`ddcaf28`):

- `DatePicker` adds inclusive bounds, pure disabled-date/custom validators, readonly,
  blur/submit/cancel callbacks and one roving calendar cursor. Arrows, month/year
  paging and week/month endpoints inspect dates without committing; unavailable days
  within bounds remain readable/focusable, while activation is guarded. Header
  navigation respects bounds. Valid choices, Apply, Enter and blur share the draft.
- External values synchronize month/draft/focus without clamping committed data.
  Stable internal keys preserve the field through row-count changes. State-backed
  and explicitly revisioned custom bindings prevent stale A→B→A drafts. Calendar
  focus reveal runs after layout and settles after manual scrolling. Validation
  callbacks may supersede actions; mount/model/scope are rechecked before committing.
- Eight focused core tests and hidden native date/focus tests pass. These cover
  Gregorian boundaries, readonly/blur, callback purity/removal/replacement, rejected
  setters, source revisions, retained fields, scrolling, AX selected children,
  unavailable focus, native text transport and composition-before-submission.
  Workspace tests, strict Clippy, portable Linux compilation, formatting and
  inventory mapping pass.
  `m6_date` is the runnable example; dark/light previews inspected. Foreground
  suites are left for the dedicated desktop audit while user-assisted VoiceOver
  testing is in progress. No unavailable interactive evidence is counted as a pass.

M6.4 standalone time checkpoint (`02b1f19`):

- `TimePicker` adds one transactional field, hour/minute/optional-second steppers,
  explicit 12/24-hour display, AM/PM selection, 1–60 minute/second steps, inclusive
  bounds, pure custom validation, readonly/blur/submit/cancel policy and locale
  digits/fraction separators. Hour/minute edits retain hidden seconds/nanoseconds.
  Steps carry components without wrapping days; invalid text/bounds remain intact.
- Six focused portable tests cover exact values, localized grammar, midnight/noon,
  steps/bounds/custom validation, keyboard/blur/callbacks, rejected setters and
  source revisions. `m6_time` is the runnable two-clock example; dark/light raster
  previews inspected. Hidden native time/date/focus tests pass, covering text/AX
  values/actions, preedit confirmation, readonly and retired objects while asserting
  windows stay invisible. Workspace tests, strict Clippy, portable Linux compilation,
  formatting and inventory mapping pass. Actual speech/candidate and visible GPU
  sessions remain M5.5/M8.
- Combined local datetime and zoned completion follow below.

M6.4 local datetime checkpoint (`4124481`):

- `DateTimePicker` has one full-value draft and shared calendar/clock controls.
  Joint inclusive bounds, disabled dates and custom validation guard every write.
  Typed date/time changes can cross tight overnight bounds without an intermediate
  model update. Exact clock steps carry through midnight/leap/year boundaries;
  calendar choices retain the staged clock. Locale grammar, hidden precision,
  readonly/blur/callback policies and source revisions follow the standalone controls.
- Six focused core tests pass, including a one-write overnight transition, exact
  extremes/carry, retained editor, calendar/clock coordination, mixed digits across
  the whole value, callback authority, rejected setters and external A→B→A changes.
  `m6_datetime` is the runnable example; dark/light previews inspected. Hidden native
  datetime/time/date/focus tests pass, including joint draft/AX values, disabled
  date choices, retained field, composition, readonly and retired objects while
  asserting windows stay invisible. Workspace tests, strict Clippy, portable Linux
  compilation, formatting and inventory mapping pass.
- Zoned completion follows below. No interactive speech/candidate checks are closed here.

M6.4 zoned completion record (`b2a52c0`):

- `ZonedPicker` binds `Timestamp` and owns its display zone, avoiding Jiff's
  instant-only `Zoned` equality dropping zone-only changes. It reuses one civil
  datetime draft/calendar/clock. Inclusive instant limits, civil limits, disabled
  dates and pure `validate(&Zoned)` guard all candidate writes in the selected zone.
- Gaps reject without shifting. New folds require explicit Reject/Earlier/Later
  policy; changing it revalidates unfinished text without resetting it. An existing
  fold retains its known occurrence; Apply occurrence explicitly changes it. Wall
  steps resolve real transitions, including Lord Howe's 30-minute fold. Exact hidden
  precision survives edits and zone changes.
- Explicit `change_zone` chooses KeepInstant or KeepLocal. It validates a staged
  candidate before publication, rejects invalid drafts/targets/bounds/DST, and
  respects rejected setters and superseding callbacks. Source/zone revisions and
  stable keys retain coherent text/calendar/focus. Zone errors expose AX validation.
  The prototype's zoned binding/string-resolution/inline-zone grammar is replaced
  by this explicit interface; the format contract records the migration and the
  unchanged low-level conversion helpers.
- Nine focused zoned core tests pass, alongside time/local datetime regressions.
  They cover both occurrence/zone modes, exact limits, readonly/locales/extremes,
  invalid-text preservation, source revisions, pure validation and reentrant setters/
  callbacks. A reproduced equal-wall-time callback defect now revalidates the
  resulting instant before reporting a successful step. `m6_zoned` is the runnable
  example; dark/light previews inspected. Workspace tests, strict Clippy, portable
  Linux compilation, formatting and inventory mapping pass. Five hidden AppKit
  harnesses (zoned/local datetime, time, date and focus) pass, including actual
  text/AX policy/zone/occurrence actions, error help, retained field, composition,
  readonly and retired objects, with invisible-window assertions.
- Actual speech, IME candidate placement and foreground/Metal runs remain M5.5/M8;
  no skipped or protocol-only check is counted as that evidence.

M6.5 completion record (`0a7f750`):

- `ColorPicker` now owns one RGB/HSV/hex draft, numeric component fields/sliders,
  model switching, alpha policy and checkerboard preview. Public HSV conversions
  operate on encoded sRGB; the session retains hue through gray/black and saturation
  intent through black. Rounded display does not reparse untouched HSV values.
  Hidden alpha is preserved and explicit alpha input is rejected when disabled.
- Transactional edits wait for Apply/Enter. Explicit live preview retains baseline
  and last-owned source/revision; Cancel rolls back only that preview. Apply
  finalizes without equal writes. External changes, rejected/normalized setters
  and reentrant callbacks remain authoritative. Lifecycle cleanup runs outside
  construction/borrows for hide, disable, collapse, removal, deactivation, unmount,
  configuration changes and controller drop. Theme changes retain the session.
- The well now uses an anchored/fitted/scrollable popover. Apply closes after
  finalization; Cancel, outside and Escape restore the source. Existing popup
  dismissal observers are preserved, and replacement presentations retire prior
  preview ownership. Host color actions accept group/child IDs and refuse Apply
  during marked text. Readonly retains model, focus and text selection.
  Natural measurement/reflow now honor an explicit scroll height cap; the popover
  exposes the full editor when space permits and scrolls in constrained viewports.
- Eleven focused core tests cover conversion/limits, exact/retained HSV intent,
  alpha, shared input, both publication modes, source authority, readonly and
  lifetime/popup generations. Raster checks verify actual straight-alpha compositing
  and transparent RGB preservation. The hidden native harness covers text/sliders,
  RGB/HSV identity, composition, alpha validation, popover actions/rollback,
  readonly and native unmount. Workspace tests, strict Clippy, portable Linux
  compilation, formatting and inventory mapping pass; targeted core/native checks
  also pass after final popover styling. `m6_color` is the runnable example, with
  `--well` for the open popover. Dark/light inline and open-well previews inspected.
  Foreground/interactive evidence remains M5.5/M8; native protocol/raster evidence
  does not close those gates.

M6.6 acceptance checkpoint (`8bc8844`; milestone was still open):

- Shared drafts retain rejected text with feedback and explicit retry, read back
  application normalization, preserve exact pristine values without equal writes,
  and protect newer edits made by setters or validation callbacks. Codec/status
  queries are pure. Invalid non-reflexive values can be corrected; formatter
  context changes replace stale text. Numeric/time/datetime stepping uses the
  same acceptance path. Zoned changes and color Apply honor newer callback edits.
- All six editors expose `binding()`, `validation()`, `commit()` and `cancel()`.
  Date selection cannot commit text staged by a newer callback, and apply/cancel
  preserve callback calendar navigation. Generic value fields support Escape and
  host readonly state; numeric steppers announce their staged candidate.
- `m6_drafts` demonstrates six editors with an application acceptance switch.
  Eight shared lifecycle tests plus focused numeric/calendar tests cover rejection,
  retry, normalization, callbacks, purity, exactness and host behavior. Dark/light
  previews inspected. Strict Clippy, portable Linux compilation, formatting,
  inventory mapping and hidden native date/time/datetime/zoned/color/focus checks
  pass. Full workspace tests pass; their skipped native harnesses are not counted
  as desktop evidence.
- At this checkpoint, appendable-prefix classification and the shared
  blur/composition/lifecycle matrix remained open. They are closed below; actual
  VoiceOver/IME evidence remains M5.5.

M6.6 completion record (`f15de33`):

- Appendable date/clock prefixes respect Gregorian validity, localized order and
  digits, ISO BCE years, optional seconds/fractions and AM/PM placement. Number
  grouping and ICU sign prefixes, hex and color numeric fields distinguish
  incomplete/invalid/out-of-range states. Mixed digit sets and impossible suffixes
  stay invalid. `Draft::incomplete_when` supports pure application codecs; legacy
  ISO helpers use the same rules. All-prefix sweeps cover representative formatted
  values across every supported locale and all six controls.
- Blur applies on leaving the whole editor, explicit blur or deactivation according
  to policy. Internal controls do not apply before Revert. Host/AX Apply, steppers,
  calendars, sliders and mode changes wait for that editor's preedit; independent
  editors remain usable. Enter/Escape retain their two-step composition behavior,
  while Revert/Cancel buttons discard preedit and revert together. Datetime Cancel
  preserves newer callback calendar navigation.
- Unavailable preedit cancels on hide/disable/collapse/readonly/unmount. Ordinary
  model-owned drafts survive hide/unmount; color retires owned previews under its
  existing policy. Binding revisions reach native-independent editors, so equal-
  display replacements (including A→B→A and hidden color alpha changes) cancel
  stale preedit. Own edits retain undo/redo. Custom providers must supply revisions
  for equal-looking replacements; pure queries do not emit editing callbacks.
- Fourteen additional focused prefix/interaction tests pass, alongside the earlier
  shared acceptance tests. Full workspace tests, strict Clippy, portable Linux
  compilation, formatting and inventory mapping pass. Hidden AppKit date/time/
  datetime/zoned/color/focus checks pass; time tests explicitly verify AX prefix
  descriptions, preedit action guards and revisioned replacement. Skipped harnesses
  in the ordinary workspace run do not count as native passes.
- `m6_drafts` is the runnable six-control example, with application acceptance and
  blur toggles plus `--prefix --tab=number|date|time|datetime|zone|color`. Dark/light
  zoned and color prefix previews were inspected. Actual speech/candidate placement
  and visible GPU evidence remain open in M5.5/M8. M7 inspector work is next.

## M7 — Developer tools and application ergonomics

- [x] **M7.1 Inspector:** a navigable mounted hierarchy with layout, focus, selection, validation, overlay and collection-window details, synchronized with picking and removal.
- [x] **M7.2 Overrides:** useful layout/style controls beyond width, clear indication of overridden values, reset and accurate measurement/clip feedback; preserve coherent IDs and restore original behavior after reset.
- [x] **M7.3 Lab and ergonomics:** cover the full inventory in both themes and constrained sizes; demonstrate stateful custom components with concise macros/builders and no routine application-side reference-counting or native plumbing.

Evidence: inspector/Lab examples and model/geometry tests, plus a small external
application compile check using a renamed `scars-window` dependency.

M7.1 completion record (`43b59c1`):

- `Inspector::view()` provides a virtual mounted outline, keyboard disclosure and
  selection, and live layout/clip/text metrics, focus/text selection/preedit,
  validation, input scope, overlay and collection-window diagnostics. Its own UI is
  excluded. Picking preserves application focus/preedit, retires pointer captures
  and consumes release without activating the target. Tool controls remain usable
  while picking. Highlights honor clipping and hidden state.
- Owned snapshots contain no application models/builders/providers/callbacks;
  text previews are bounded and passwords masked. Collection diagnostics report
  counts, visible/overscan windows, realized/pinned rows, selection/cursor and table
  columns without cloning datasets or realizing logical rows. A 100,000-row test
  verifies bounded snapshots and callback release.
- Hierarchy selection/clear updates immediately from captured data, independently
  of source rendering; geometry follows layout. Label refresh preserves row
  lifetimes/disclosure and manual scrolling. Removal clears selection. Rebinding
  transfers ownership; old source teardown cannot clear the new source. Teardown
  clears the live inspector while previously returned snapshots remain usable.
  A separate view host can display the source inspector without replacing its data.
- `m7_inspector` demonstrates forms, visibility, popovers and 10,000 virtual rows.
  Dark/light selected collection previews were inspected. Raster examples accept
  `--select-key KEY`. Seven focused core tests, actual pixel clipping/hidden-target
  tests and hidden AppKit outline/focus tests pass. Full workspace tests, strict
  Clippy, portable Linux compilation, formatting and inventory mapping pass.
  `checks/renamed-consumer` is a locked standalone compile fixture using the `kit`
  dependency name, `view!` and inspector interfaces; its compile check passes.
- Rich override editing/reset is completed in M7.2 below; full Lab and application
  ergonomics are completed in M7.3 below. Hidden protocol and CPU raster checks do not close M5.5/M8's actual
  speech, composing-input candidate placement or visible Metal requirements.

M7.2 completion record (`54606ad`):

- Layout/Style pages edit individual dimensions, constraints, insets, flex,
  positioning, typography and appearance. Retained drafts preserve invalid input;
  Enter/Apply commits valid edits. Each row identifies source/overridden values,
  with property/node/all resets. Application code uses a small typed
  `StyleOverride` builder; untouched properties remain reactive. Explicit equal
  values and inheritance overrides remain distinct from no override.
- Invalid patch assignments are atomic. Later reactive constraint conflicts
  suspend the whole patch, render the source and expose an error; valid patches
  resume or reset. Mounted identities retain patches across rebuilds; unmount and
  source rebinding retire them without affecting another source.
- Snapshots distinguish source/effective styles, explicit overrides, final
  preferred measurement, allocation, padding-inset content, clip and resolved text.
  Actual flex plans report line/item geometry, justified gaps and signed free
  space/overflow. Outline/Box/Flex highlights respect clipping and hidden state.
- `m7_inspector` demonstrates reactive preview padding/background, property pages,
  resets and diagnostic overlays. Dark/light property and box/flex raster previews
  were inspected. Raster flags include `--select-key preview --override-preview
  --inspector-page layout`; `--box`/`--flex` also select highlight modes.
- Eleven focused inspector tests cover property drafts/codecs, reactive behavior,
  atomic validation/conflict recovery, lifecycle and flex overflow alongside M7.1
  contracts. Two CPU pixel tests verify real diagnostic changes and clipping.
  Hidden AppKit tests pass for property editing/reset, source nonactivation,
  outline selection/retirement and tab/focus protocol regressions. Full workspace
  tests, strict Clippy, portable Linux compilation, renamed-consumer compilation,
  formatting and inventory checks pass. Native harness skips in the ordinary
  workspace run are not counted as passes. Actual M5.5/M8 audits remain.

M7.3 completion record (`8782764`):

- `lab` now contains seven retained pages and 28 named samples covering the full
  control inventory: basic/choice/progress controls, typography/rich inline text,
  grouping/flex/positioning, all image modes, two-axis scroll, fields and typed
  values, menu/presentation/split/tab/wizard controls, virtual collections and
  inspection. The [Lab guide](rust-lab.md) maps samples and reproducible commands;
  native sheet integration remains demonstrated by `m3_alerts --sheet` using the
  same portable alert transaction.
- Two reusable Counter models/functions demonstrate independent reactive state,
  keyed macro composition and conditional content. Models, callbacks and resources
  are created outside view builders. Application code contains no `Rc`, weak
  ownership or native adapter plumbing. A renamed-dependency fixture compiles
  custom function components and the new annotation/slider interfaces.
- `labeled`/`View::labeled` provide visible annotation plus fallback accessible
  names while preserving explicit names. Slider `page_step` independently controls
  page keys with finite-positive validation and normal snap/clamp/readonly policy.
  Focused core tests cover these remaining inventory gaps.
- Cards wrap and pages scroll at narrow widths. The shared example runner accepts
  `--size` and `--reveal-key`; Lab accepts page/state/presentation flags. Startup
  presentations reveal/focus actual openers after layout, fixing premature closure
  of offscreen combo/color popups. All six documented presentation commands were
  checked for actual modal scene output and their raster previews were reviewed.
- Four application tests use real Skia to select/reveal/paint every sample in both
  themes at 440×740 and 1120×900, including bottom views of tall cards. Tests also
  cover custom/editor identity, readonly policy, modal/menu scope, real collection
  edits and startup presentations; realization stays bounded. The emitted gallery
  and focused previews were inspected; a stretched field, clipped positioned label
  and transparent sample popover were corrected.
- Workspace tests, final catalog tests, strict all-target Clippy, portable Linux
  core compilation, renamed-consumer compilation, formatting and inventory checks
  pass. Native harness skips are not counted as passes. Actual VoiceOver/IME,
  backing-scale and visible Metal release evidence remain M5.5/M8.

## M8 — Release evidence and documentation

M8.1 native rendering completion record (2026-11-06):

- On macOS 27.2, the native `desktop` suite passed visible Metal pixel checks,
  window lifecycle, sheets, input, Skia and callback behavior. The separate
  full-screen Space transition test passed. Secure-input and the native component,
  accessibility, picker and inspector harnesses also passed in the full native
  run. Manual VoiceOver/IME remains deferred under M5.5.
- The first desktop attempt exposed an undersized M8.4 fixture window. It now uses
  520×400 points, meeting the backend's default minimum; the rerun passed.
- `python3 scripts/check_rust.py --suite native --offline` passed its first 19
  steps. Its final M8.5 example step stopped at the foreground readiness guard;
  that does not invalidate the separately passed native rendering checks.

- [x] **M8.1 Native rendering:** rerun visible Metal pixel, lifecycle and full-screen suites on an awake foreground desktop; record failures accurately and fix genuine regressions.
- [x] **M8.2 Visual/interaction matrix:** reproducible dark/light previews at 1×/2×, narrow/wide sizes, disabled/readonly/error/focused states and representative long/RTL/emoji text.
- [x] **M8.3 Repeatable checks:** one documented local validation entry point and suitable CI checks for formatting, strict Clippy, core/macro/raster tests, external consumer compilation and portable-core compilation; native interactive checks remain explicit.
- [x] **M8.4 Resource behavior:** measure large collections and long editing sessions for retained objects/caches and idle wakeups; verify inactive/hidden/unmounted controls cancel work and settle.
- [ ] **M8.5 Regression examples:** run the themed todo app and all milestone examples after the final interfaces settle; verify editing, filtering, persistence and external value updates still work.
- [ ] **M8.6 Inventory sign-off:** close every in-scope inventory row and checklist item with evidence, update public docs/examples and remove stale completion claims. Record remaining optional extensions separately before completing the goal.

M8.2 completion record:

- The Lab paints all 28 samples in both themes at logical sizes 440×740 and
  1120×900, across default/disabled/readonly/error/focused states and explicit
  CPU raster scales 1 and 2. Focus previews use real interactive descendants;
  static samples do not invent a focus target. Tall cards also receive bottom
  views. Invalid numeric/date/time/datetime/zoned/color drafts and a rejected
  table edit remain visible as real component state.
- Six Lab tests pass. Added tests cover long/RTL/emoji text across themes,
  widths and availability states; editable changes succeed while disabled and
  readonly mutations fail. Text hit tests remain on grapheme boundaries.
  Actual field pixels match the theme at both scales, and active focus changes
  pixels inside its field only. Existing instance/identity, scope, collection
  action and six startup-presentation tests also pass.
- The full headless validation passed all nine steps: formatting, strict
  Clippy, workspace tests, inventory, Linux-core and renamed-consumer checks,
  validator tests and gallery verification. This run produced 644 paired scenes
  (1,288 PNGs); the gallery verifies every 2× image has twice the framebuffer
  width and height. Each run has a fresh artifact directory, recorded under
  `lab_artifacts`, and a searchable `index.html` pairs both densities.
- Reviewed representative narrow/wide, dark/light, error, disabled, readonly,
  focused, RTL, emoji and long-text previews. Nine actual Lab CLI renders also
  passed dimension checks, including all six light-theme presentations at 2×.
  Invalid raster scales and requesting a synthetic native scale are rejected.
  The [Lab guide](rust-lab.md) gives concise reproduction commands.
- These are CPU raster and portable interaction results. The reported tab
  utterance still needs an actual speech recheck under deferred M5.5.

M8.4 resource behavior completion record:

- `m8_resources` is a runnable terminal workload using real Skia layout/painting.
  Two measured runs use 100,000 rows each for list/table/tree, 128 visits and
  5,000/20,000 edits. Both peak at 29/38/47 mounted nodes, 512 paragraph and 128
  image entries, and 100 undo entries. Explicit cache clearing returns entry
  counts to zero. Peak process RSS was 285,376,512 / 285,425,664 bytes; these
  observations are not universal byte budgets or timing test thresholds.
- Four focused tests pass: real renderer/collection/edit churn, 300 editor
  replacements plus actual mounted undo history and callback retirement,
  repeated hidden/inactive/removed search/repeat/progress cancellation, and
  paragraph ownership across cache clearing/live-layout release. The cancellation
  test observes zero subsequent invalidations/callbacks after settling.
- `Renderer::cache_stats()` and `clear_caches()` expose concise resource
  diagnostics/release without exposing internal ownership. The hidden AppKit
  lifecycle regression also passed, including idle notification coalescing and
  unmounted native proxy/callback teardown.
- The full headless validation entry point passed all nine checks. The final
  workload counts actual mounted undo entries and verifies redo restores the
  latest text; its focused test, rebuilt example and strict all-target Clippy
  passed again after that refinement. Both reported measurements use this final
  executable. Native skips in the headless run do not count as native passes.
- On the foreground desktop, the lifecycle cases delivered 26/28/29 Metal frames
  before settling for hidden, inactive and unmounted states respectively, then
  observed zero idle frames, deadlines or callbacks. This closes the visible
  portion alongside the measured workload and focused retention tests above.
  The [resource report](rust-resources.md) records commands and limits.

M8.5 headless regression evidence prepared; native examples remain open:

- The example runner classifies all 39 Cargo targets. After the crate move, the
  headless example runner passed all 161 steps: 37 executed examples, including
  three terminal demonstrations, and 156 dark/light 1×/2× PNGs (78 verified
  pairs). Sheet content, Arabic localization and inspector box/flex/popover
  variants also run. The two native-only diagnostics, `stage1` and `windows`, are
  explicitly unrun pending the foreground run.
- Seven Todo tests pass, including the mounted view in both themes: adding,
  editing/cancel/save, completion/filter controls, persistence/reload, recovery
  after a failed save, external input/item updates and retained keyed identity.
  A new external-item regression reproduced duplicate task IDs `[1, 1]` before
  the fix; allocation now advances beyond both its counter and current items,
  while deleted bindings remain inert for later tasks.
- Todo raster runs receive deliberately invalid saved data. All four preview
  renders succeed with sample tasks in their inspected scenes and leave that
  fixture unchanged. Reviewed Todo in both themes, Arabic localized values and
  the inspector popover preview. Visual review found that popover's transparent
  body overlapping underlying content; the example now supplies a themed frame,
  padding and width. Strict Clippy and the complete example matrix passed again
  after that correction; both themed popovers were visually rechecked.
  The [example guide](rust-example-regressions.md)
  provides runnable commands, coverage and artifact locations.
- Nine validator tests pass, including exhaustive Cargo target classification,
  read-only native planning and cancellation of a nested runner's separate child
  process group. The full entry point and CI now include example regression
  checks; native mode includes foreground example/sheet smoke runs.
- The native matrix stopped at its desktop guard after focus returned to Codex;
  no native example was counted as run. M8.5 remains open until foreground smoke
  runs pass. CPU previews and model/view tests do not count as native windowing
  evidence.

M8.6 audit and build evidence prepared; sign-off remains open:

- The [release audit](rust-release-audit.md) reconciles all 44 checklist entries
  against concrete tests/examples: 41 complete, two open M8 gates, and M5.5
  deferred to the TODO list. An option-family table maps C++ fields to Rust options,
  modifiers, model ownership and callbacks. The reference source is unchanged
  from `82583e7`; name coverage still maps 162 types and 47 modifiers/annotations.
- Inventory/component documentation no longer defers completed native semantics,
  localization, shared drafts, inspector overrides or the full Lab to earlier
  milestones. Intentional palette/style/font, locale and model-ownership interface
  changes are explicit. Optional extensions retain their original scope.
- Actual Rust 1.88.0 validation passes: 277 core/macro tests, dev and release
  workspace/all-target compilation, Linux-core compilation and the separate
  renamed consumer. Both native-engine profiles record full source Skia builds.
  The portable `foundation` example also runs under the minimum compiler.
- An older stripping-tool issue on macOS 27 was reproduced in debug-disabled
  and release builds, then resolved by shared dev/test/release `strip=none`
  configuration. The `msrv` entry point selects its own exact compiler, records
  matching metadata and runs all five gates. Ten validator tests pass; the
  primary/minimum CI matrix passes Actionlint. Remote CI execution is unclaimed.
- The hidden native expansion-selector regression found and corrected the
  unsupported inherited setter described under deferred M5.5. Actual tab speech
  remains unverified and tracked in the TODO list. M8.6 stays unchecked until
  M8.5 closes with real foreground evidence and final inventory sign-off
  is recorded.
- Final local reruns pass all ten headless gates and all five minimum-Rust gates,
  including dev/release source builds. The audit's 44 statuses match the checklist
  and local documentation links resolve. No unavailable foreground or interactive
  work is counted as passed.

M8.3 completion record:

- `python3 scripts/check_rust.py` is the documented local entry point. Headless
  mode runs formatting, strict workspace/all-target Clippy, core/macro/raster/Lab
  tests, the inventory mapping, renamed-consumer compilation and Linux core
  compilation. Portable mode runs the applicable core/macro checks. Each mode
  passed locally with `--offline`; native skips are explicitly outside those passes.
- Per-step logs and atomic JSON reports record source/toolchain/platform/profile
  metadata, exact commands, exit codes and timings. Headless runs emit the Lab
  gallery. Failures stop later checks; timeouts/interruption retire the Cargo
  process group. Five validator tests exercise child-process flag isolation,
  failure/report propagation, descendant timeout cleanup, native-harness coverage
  and planning without execution.
- Native mode requires the foreground readiness probe, then serially runs visible
  Metal/lifecycle, full-screen, secure-input, foreground-sheet and all opt-in
  protocol harnesses with their explicit flags. Headless/portable modes strip
  inherited native opt-ins. Actual VoiceOver/IME sessions remain unverified in
  every automated report.
- The GitHub Actions workflow uses the same entry point on Ubuntu 24.04 and macOS
  26, with the verified Rust 1.99.1 toolchain and pinned official action commits.
  Skia is source-built without target caching; reports/logs/gallery are uploaded.
  Workflow syntax/schema passed `actionlint` 1.7.12. Commands passed locally; no
  remote Actions execution or native/interactive CI pass is claimed.
- The [validation guide](rust-validation.md) describes setup, commands, artifacts
  and exact scope. The runnable command and emitted Lab examples demonstrate this
  stage; no additional application or native plumbing is required.

M8.1 readiness record, 2026-11-06:

```text
active-displays=0 display-error=0 front=com.apple.loginwindow app-active=0 key-window=0 visible=1 link-paused=0 callbacks=0 ready=0
```

The independent AppKit probe failed its awake/foreground gate. Visible native
suites were not run or counted as passes. The user has been asked to wake/unlock
the Mac; actual VoiceOver/IME and backing-scale sessions remain open. Independent
release work continues while that desktop condition is unavailable.

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
