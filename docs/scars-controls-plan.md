# SCARS control implementation plan

Implement the remaining common controls in eight phases, beginning with SplitView and its integration into the developer inspector. Each phase delivers working controls, gallery examples, API documentation, and verification. Unchecked items remain open until implementation and applicable verification meet the completion criteria.

## Tracked goal

Implement all eight phases below while preserving SCARS as an independent C++23 named-module engine. Deliver the controls, inspector integration, Lab examples, API documentation, and required behavioral, rendering, and macOS native verification. Keep compiler coverage explicit. The goal is complete only when every phase meets the completion criteria; completing an individual phase is a checkpoint.

All eight phases are implemented and have met the available-platform completion criteria. Both full builds and all 29 registered CTest tests pass in Debug and RelWithDebInfo, including 1,220 portable SCARS checks, all six SCARS native suites, and the previous UI/window regressions. The full foreground gates now verify secure-input ownership, one-shot scheduling, and real rendering. The [completion audit](scars-controls-verification.md) maps each phase and shared requirement to current source, examples, documentation, and verification. Apple Clang 21 is verified; upstream Clang 23, GCC 16, and MSVC remain explicit, unvalidated compiler targets.

### Status checkpoint — 2026-11-03

| Phase | Implementation | Verification / next work |
| --- | --- | --- |
| 1 SplitView / TabView | Complete | Required available-platform checks passed |
| 2 Input controls | Complete | Required available-platform checks passed |
| 3 Overlays / ComboBox | Complete | Required available-platform checks passed |
| 4 Scrolling / virtualization | Complete | Portable, rendering, and required native gates pass in both configurations |
| 5 TreeView / TableView | Complete | Portable, rendering, and required native gates pass in both configurations |
| 6 Alerts / confirmations | Complete | Portable, rendering, and required native gates pass in both configurations |
| 7 Date / time | Complete | Portable, rendering, and required native gates pass in both configurations |
| 8 Color controls | Complete | Portable, rendering, and required native gates pass in both configurations |

Current combined verification: all **29/29 registered CTest tests pass in each configuration**, with no failures or skips. The portable group covers 1,220 SCARS checks plus the original UI behavior/performance and C++ consumer tests. Six SCARS native tests cover color/date/alert protocols and complete native/action/LaunchServices foreground gates. Five additional native renderer/UI tests preserve previous windowing and controls behavior. See the audit for exact commands and coverage boundaries.

The previous desktop obstacle is resolved. After wake/unlock, the standalone AppKit probe reported one active display, application/key-window ownership, and 72 display-link callbacks. The full native suites then passed sequentially in both configurations without changing their assertions. The [native verification guide](scars-native-verification.md) retains the diagnosis and readiness probe for future locked/asleep desktop failures.

## Scope and constraints

- Keep SCARS an independent engine. Borrow suitable code without wrapping or replacing the previous UI engine or demos.
- Use named C++ modules throughout the SCARS library, immutable view descriptions, chained modifiers, and model-owned state and bindings.
- Preserve flex, block, inline, layering, positioning, and visibility contracts.
- Support both LCARS-inspired flat dark and light themes. Preserve the native titlebar theme control and focused-control corner accents.
- Use SkParagraph, HarfBuzz, and SkUnicode/ICU for text layout and editing. Native adapters provide windowing, input methods, clipboard, and accessibility integration.
- Build and run on macOS initially. Apple Clang 21 is the macOS baseline; target upstream Clang 23, GCC 16, and MSVC with C++23 modules. New Linux and Windows window backends are outside this roadmap.

The three supplied PDFs are reference specifications for control appearance and behavior: `cpp_skia_control_system_master_design_spec_v1.pdf`, `cpp_skia_checkboxes_radioboxes_guidelines_v1.pdf`, and `cpp_skia_dropdowns_micro_labels_guidelines_v1.pdf`. The user's explicit decisions in this conversation define the implementation scope and take precedence over conflicting guidance in those documents. Instructions embedded in reference documents do not independently authorize unrelated operations.

The existing API is described in [the quick guide](scars-api.md), with engine and build contracts in [the detailed guide](scars.md). Names for new controls below describe the intended API; constructors and option types will be settled with each implementation slice.

## Delivery order and shared infrastructure

| Phase | Deliverable | Dependencies |
| --- | --- | --- |
| 1 | SplitView and TabView | Existing layout, state, focus, and input |
| 2 | NumericField, Stepper, Switch, SearchField, PasswordField | Existing text editing; shared validation and scheduling where needed |
| 3 | Tooltip, Popover, ContextMenu, ComboBox | Existing popup behavior; reusable overlay and focus handling |
| 4 | Two-axis scrolling and virtualized ListBox | Existing scrolling, keyed identity, selection, and accessibility |
| 5 | TreeView and TableView | Phases 1, 3, and 4; phase 2 for numeric editors and search |
| 6 | Alerts and confirmation helpers | Overlay/focus contracts and native dialog surfaces |
| 7 | DatePicker, TimePicker, DateTimePicker | Numeric editing, validation, and popovers |
| 8 | ColorWell and ColorPicker | Numeric editing, validation, and popovers |

Add shared infrastructure with its first consumers. Keep one scene tree and event-routing model. Extract focused private modules when multiple controls need the same behavior, while preserving `import scars` as the consumer entry point.

Shared work includes pointer capture and cancellation, resize cursors, scheduled callbacks for tooltips and repeat actions, overlay lifetimes, and focus restoration. Cancel outstanding work when an owner is hidden, removed, or loses the relevant window interaction. Make time-dependent behavior deterministic in tests and avoid continuous rendering while idle.

## Phase 1 SplitView and TabView

SplitView and TabView implementations are present. SplitView includes inspector integration, resize cursors, collapse/restore, and a nested gallery example. TabView includes lazy keyed pages, retained editing/scroll state, disabled tabs, selection fallback, scoped keyboard navigation, scrollable headers, inspector metadata, and a gallery example.

The engine, feature, and tab suites pass in Debug and RelWithDebInfo, alongside the original UI behavior/performance tests and C++ consumer check. Dark/light and narrow-window previews were inspected, and raster checks cover 2x display scaling. Native protocol, frame-settling, and foreground suites now pass in both configurations, including splitter/tab semantics, page visibility, editor Control-Tab, and inspection/window lifetime. A popover dismissed inside an active tab restores its invoking control; changing pages still moves focus to the newly selected tab. Apple Clang 21 is verified; other compiler targets remain unvalidated.

- [x] **SplitView:** horizontal and vertical arrangements, bound divider positions, minimum pane sizes, dragging, keyboard resizing, resize cursors, and optional collapse/restore.
- [x] Define deterministic behavior when the available space cannot satisfy pane minima. Cancel dragging safely on capture loss, hiding, removal, or window deactivation.
- [x] Use SplitView between the inspector's element tree and property panel.
- [x] **TabView:** stable tab IDs, disabled tabs, keyboard navigation, lazy initial page creation, and retained state for visited pages.
- [x] Exclude inactive pages from layout, input, accessibility, and animation while retaining their editing and scroll state. Document how this differs from existing visibility modifiers.
- [x] Define fallback selection when the selected tab disappears. Apply bound-state corrections during events or model updates, never during view construction.

Verify constrained resizing, capture outside divider bounds, pane collapse, page focus transitions, IME completion, and retained state through tab changes and reordering.

## Phase 2 Input controls

All five phase 2 controls are implemented. NumericField, Stepper, and Switch provide signed integral/double bindings, portable ICU parsing/formatting, deferred drafts and validation, exact 64-bit increments, and cancellable held-button repeat. SearchField adds incremental/submitted queries, cancellable debounce, host-scoped Command-F, and IME-aware clear/cancel behavior. PasswordField adds grapheme masking with original-text geometry, optional reveal, clipboard restrictions, explicit undo policy, and redacted inspector/accessibility output. Native secure event-input ownership follows editable focus, window/application activity, visibility, and input-client/window lifetime. Gallery examples, inspection metadata, native accessibility mappings, and API documentation are included.

The phase 1–2 portable suites pass all 551 checks in Debug and RelWithDebInfo (185 engine, 153 features, 59 tabs, 84 numeric/input, and 70 search/password checks), plus the original UI behavior/performance and C++ consumer checks. Coverage includes localized numbers, exact signed bounds, subnormal decimals, validation after model setters, repeat cancellation, search coalescing and lifecycle, IME submission/cancellation/navigation, Unicode password geometry, clipboard/undo policy, redaction during reveal, both themes, and 2x rendering. Native protocol, frame-settling, and foreground suites pass in both configurations. Foreground coverage explicitly verifies secure input on password focus, release on focus change, hide/show, unmount, and window close, retained-host cleanup, and a real debounced search delivered by the one-shot redraw scheduler without continuous animation. Later phase implementations and the current native verification gaps are recorded below.

- [x] **NumericField:** integer and decimal bindings, ranges, precision, formatting, validation, and separate draft and committed values. Preserve partial input such as `-` while editing. Enter commits, Escape restores the committed value, and blur follows a documented commit policy. Invalid values produce validation feedback rather than silent clamping.
- [x] **Stepper:** use the same range and step rules, with increment/decrement buttons, keyboard operation, and held-button repeat. Stop repeat on release, cancellation, focus loss, or removal.
- [x] **Switch:** binary binding, click and Space activation, disabled styling, focused-control accents, and accessibility semantics.
- [x] **SearchField:** clear action, incremental changes, explicit submission, optional debounce, and application-scoped search focus. Respect IME composition before interpreting Escape or submission.
- [x] **PasswordField:** masked rendering, optional reveal without losing selection, normal cursor movement and paste, and explicit undo and input-method policies. Suppress password copying and plaintext exposure through inspector output, logs, and accessibility snapshots. Balance native secure-input activation with focus and window lifetime.

Reuse the existing text buffer and input-method bridge. Keep parsing and formatting portable; verify the pinned ICU build's required facilities before adding dependencies. Test overflow, non-finite numbers, partial drafts, Unicode editing, composition, clipboard rules, delayed callbacks, and cancellation.

## Phase 3 Overlays and editable selection

Shared overlay handling, Tooltip, Popover, ContextMenu, and ComboBox implementations are present. Menus and arbitrary popover content share stacking, fitting, input routing, inspection, and lifecycle handling. Popovers support nested content and menus, explicit focus policy, scroll clipping independence, outside-click consumption, composition-first Escape, binding cleanup, and focus restoration. Tooltips use delayed pointer/keyboard presentation, accessible help text, and one-shot scheduling. ContextMenu attaches to arbitrary views, supports right-click/Control-click/Shift-F10, nested menus, separators, checked/radio bindings, disabled items, shortcut labels, stable identity, and menu accessibility. ComboBox separates query and accepted ID bindings, uses the shared text editor and ListBox overlay, preserves editing focus during navigation, and supports strict/free-text modes, Unicode filtering, loading/no-results states, composition, and lifecycle draft policies. The Lab, inspector, native accessibility bridge, and API guide include the phase 3 controls. Two-axis scrolling and virtualization are implemented in phase 4; phase 5 TreeView/TableView and inspector integration are implemented with the verification gaps recorded below.

The phase 1–3 portable suites pass all 736 checks in Debug and RelWithDebInfo, including 131 overlay and 54 ComboBox checks, alongside the original UI behavior/performance and C++ consumer checks. Coverage includes both themes, viewport fitting, marked text, input consumption, nesting, arbitrary editors, scrolling, cleanup, builder failures, menu reordering/removal, all context-menu invocation paths, check/radio actions, and menus inside popovers. ComboBox coverage includes Unicode filtering, strict/free-text policies, disabled/loading states, composition, callbacks, model replacements, stable highlight identity, accessible row reveal, and text-selection shortcuts. Dark/narrow light previews and 2x raster checks were inspected. Native protocol, frame-settling, and foreground suites pass in both configurations, including popover focus restoration within tabs, help semantics, context-menu invocation/actions, combo roles and acceptance, and actual foreground input ownership. Restricted launches had hidden Metal access; unrestricted execution and a LaunchServices foreground test provide the required native verification. Phase 3 implementation and required available-platform verification are complete.

- [x] Extend the current single popup into shared overlay handling with nesting, anchor placement, window fitting, dismissal, and focus restoration. Preserve SelectionDropdown, MenuButton, and SplitActionButton behavior.
- [x] **Tooltip:** attach through a view modifier; support delayed pointer/focus presentation, accessibility help text, wrapping, and dismissal without adding a focus stop.
- [x] **Popover:** anchored arbitrary SCARS content, transient dismissal, and explicit focus behavior for interactive content.
- [x] **ContextMenu:** attach to any view and invoke through right-click, Control-click, or Shift-F10. Support actions, separators, checked/radio items, disabled items, shortcut labels, and submenus.
- [x] **ComboBox:** separate editable query text from selected stable ID. Support filtering, suggestions, strict selection or explicit free-text mode, loading/no-results presentation, and standard keyboard acceptance and cancellation.

Extend input mapping as needed. Keep the ComboBox editor's input-method focus while navigating suggestions. Test nested overlays, outside-click consumption, topmost Escape handling, hidden or removed owners, clipping, and focus restoration. ComboBox may initially reuse ListBox and adopt virtualization in phase 4 without changing its public contract.

## Phase 4 Scrolling and virtualization

Two-axis ScrollView support is implemented, including bounded two-coordinate offsets, content/viewport geometry, thin indicators, Shift-wheel, per-axis residual wheel routing through nested scrollers/editors/tab strips, deterministic nested focus reveal, imperative scroll APIs, inspector properties, and a sector-map demo. Scrolling remains scoped to overlays. Offset retention and cleanup cover hiding, replacement, resize, and document changes.

The verified two-axis scrolling baseline passed all 783 portable SCARS checks in Debug and RelWithDebInfo, including 47 scrolling checks. Both full builds and all three native suites passed, with horizontal/vertical wheel assertions and a foreground run that verifies secure-input and scheduler lifetimes. Dark/light narrow previews and 2x raster checks were inspected. The final combined verification above includes subsequent virtualization and all later controls.

Virtualization implementation is present: immutable indexed `RowModel` snapshots, fixed-height visible/overscan ranges, lazy custom rows, reuse of realized nodes, per-row observers, indexed selection/navigation, and an active-editor pin. Eviction bounds host-owned buffers and subscriptions; model-owned values survive remounting. Logical Host APIs and lazy native accessibility expose indexed/offscreen rows without fabricating bounds. Inspector properties show logical/mounted counts, ranges, active/pinned IDs, and retained editors. The Lab includes a 100,000-row editable observations example, and the API guide describes ownership and lifecycle. ComboBox suggestions use the same virtualization path; distant keyboard and accessibility focus reveal rows while retaining the text client.

At the virtualization checkpoint, both full builds and all 845 portable SCARS checks passed in Debug and RelWithDebInfo (including 60 virtualization and 56 ComboBox checks), alongside the original UI behavior/performance and C++ consumer checks. Coverage includes distant selection/ranges, reordering, deletion, disabled rows, observer eviction, composition and undo while pinned, host rebuilds, hiding, zero/constrained viewports, row-builder exceptions and mutation guards, and overlay cleanup. Dark and narrow light gallery previews were inspected; tests render at 2x. A sample optimized run indexed 100,000 rows in 4.7–6.6 ms and performed initial mounting, navigation, and 40 distant scrolls in 12.1–19.7 ms. Those operations constructed 570 row contents in total; the final constrained viewport retained six rows. These are measurements, not timing assertions.

**Native verification complete:** both configurations pass the full native/action/LaunchServices foreground suites. These include the 100,000-row logical-accessibility assertions, indexed metadata, absent offscreen frames, distant focus/selection, selected/visible rows, and composition retained while scrolling. Real frame settling, secure-input ownership, and one-shot search scheduling also pass on the restored desktop. The earlier inactive/zero-frame failures were independently reproduced in AppKit and resolved by wake/unlock; no gate was weakened.

- [x] Extend ScrollView to horizontal, vertical, and combined axes, with bounded offsets, shared viewport geometry, focus reveal, and nested wheel routing.
- [x] Add a reusable virtual row model using stable keys, logical selection/focus, visible ranges, and overscan.
- [x] Upgrade ListBox to mount only visible rows plus overscan. Avoid scanning or constructing every row during each scroll frame.
- [x] Keep an active editor or input-method composition alive until commit/cancel. Bound cached host-owned row state; keep persistent business state in the model.
- [x] Expose logical item counts, selection, and offscreen reveal through accessibility. Show logical and mounted counts and visible ranges in the inspector without inventing geometry for unrealized rows.

Verify a 100,000-row dataset with structural checks that builder calls and mounted nodes are bounded by the viewport, overscan, and any active editor. Check distant navigation, reordering, filtering, deletion, bounded retained state, and idle rendering. Record performance measurements without fragile frame-rate assertions.

## Phase 5 TreeView and TableView

TreeView and its inspector integration are implemented. An independent `TreeModel` module provides globally stable IDs, eager/lazy children, cached materialization, snapshot replacement, loading/disabled states, and iterative flattening. TreeView shares the virtual ListBox viewport and input paths, with expansion bindings, single/multiple selection, typeahead, parent/child keyboard navigation, disclosure clicks, and safe callbacks. Flattened rows and direct-child indices are rebuilt on model/expansion changes; scrolling does not walk the hierarchy. The inspector now uses TreeView with filtering that retains ancestor context, independent folding, source selection/picking, and selected-row reveal. The Lab and API guide include the tree controls.

The TreeView checkpoint passed 905 portable SCARS checks, including 60 tree checks, plus the original UI behavior/performance and C++ consumer checks. Tree coverage includes a 100,000-child lazy branch with bounded mounting, multi-selection, loading and snapshot changes, lazy-load failures, hidden initial loaders, construction purity, callback-driven removal, deep ancestry, and a 1,001-element inspector scene. Existing inspector tests now exercise the shared TreeView expansion API. Dark, narrow light, and inspector previews were inspected, and tree raster tests use 2x scaling.

Native tree protocol assertions pass for outline/row roles, lazy disclosed children, disclosure levels and parent relationships, expansion setters, distant selection/reveal, loading metadata, and stale row objects after model replacement. The full native foreground/rendering, delayed-search scheduling, and secure-input gates now pass in both configurations alongside these protocol assertions. TableView's portable foundation is now implemented: immutable indexed row/schema snapshots, fixed headers with shared horizontal scrolling, virtual rows and lazy cells, row/cell selection, bounded column resizing by pointer/keyboard/accessibility adjustment, bound widths, and explicit single/multiple sort requests. Standard text editors use separate drafts, validation, external-update conflict rejection, explicit commit/cancel, composition-first actions, and logical Tab traversal. The workbench combines SplitView, TreeView, TableView, SearchField, and context menus with application-owned filtering, sorting, numeric validation, and cell values. Inspector table properties and API documentation are present.

The table accessibility checkpoint passed both full builds and 1,011 portable SCARS checks in Debug/RelWithDebInfo, including 78 table checks and 28 integrated workbench checks, alongside the original UI behavior/performance and C++ consumer checks (15 CTest tests per configuration). This supersedes the previous 959-check baseline for portable verification; full native verification remains separate. Table tests cover 100,000 rows with bounded construction, fixed/aligned headers, logical offscreen cells, selection rectangles and row selection, resize capture/Escape/keyboard/adjustment, sort-request cycles without hidden reordering, editor drafts, marked text and pinning, validation, external conflicts, editable-cell traversal, commit-driven row reordering, edited-column removal, and Escape priority inside popovers. Dark and narrow light workbench previews were inspected; table raster tests cover 2x scaling.

Native table transport now exposes lazy indexed columns and cells, header relationships, sort direction/priority, batch row/cell selection, offscreen reveal, standard editor activation, validated value replacement, and safe held accessibility objects after schema removal/window closure. Native action runs have passed the new table protocol checks, including clipboard/undo, candidate geometry, marked-text preservation, F2/Tab editing, column resizing/reordering/removal, and close-time object lifetime. The final native runs also pass the full foreground, scheduling, rendering, secure-input, and lifetime assertions in both configurations.

Additional portable tests cover editing across row/column reorder, filtering and hiding, disabling, capture cancellation, failed cell realization/recovery, construction mutation guards, and schema-aware sort metadata. A 28-check workbench interaction test exercises tree filtering, search, sorting, rejected/accepted edits, resize/reset, context menus, selection, theme/viewport changes, and idle settling.

**Phase 5 verification complete:** full native suites pass in both configurations on the restored desktop. Stable-key revalidation in row/cell reveal and editor activation prevents layout notifications from leaving obsolete indices or base values. Three regressions cover removal, reordering, and value replacement during reveal, bringing the table suite to 81 checks; moving-target retries are bounded. The workbench, inspector integration, documentation, dark/narrow light previews, 2× raster checks, and required native gates are verified.

- [x] **TreeView:** stable node IDs, parent relationships, expansion bindings, lazy children, single/multiple selection, typeahead, and standard keyboard navigation. Represent loading and disabled nodes. Keep asynchronous loading in the application model.
- [x] Replace the inspector's custom element tree with TreeView while preserving filtering, expansion, source selection, and click-to-pick behavior.
- [x] **TableView:** stable row/column IDs, headers that remain aligned during scrolling, bounded/resizable column widths, sort descriptors, row or cell selection, and virtualized rows.
- [x] Add editable cells with Enter/F2 activation, explicit commit/cancel, validation, and Tab traversal. Preserve active editors through virtualization and give input-method composition priority.
- [x] Keep sorting and data ownership explicit: request model changes through descriptors/callbacks rather than silently reordering application data.
- [x] Add a data-workbench example combining SplitView, TreeView, TableView, SearchField, and context menus.

Verify flattened tree navigation, expansion during data changes, selection identity, synchronized headers, column resizing, and editing during scroll/filter/sort operations. Full-data sorting or tree flattening may occur on explicit data changes, not on every scroll frame.

## Phase 6 Alerts and confirmations

Alert and confirmation implementations are present. `Alert` composes arbitrary content and role-aware action buttons around a shared, exactly-once transaction. Its deferred content builder observes state without replacing the transaction and obeys construction purity. It validates identity/default/cancel roles, disables completed content, and gives composition, multiline editing, field-specific actions, and nested overlays priority over default/cancel shortcuts. Destructive actions have explicit styling and no implicit affirmative default. The inspector reports transaction status and action roles; native accessibility includes a dialog group and default/cancel button relationships.

The macOS adapter now mounts the same independent Host into the existing native Surface interface, preserving Window call sites and adding Dialog support. `AlertPresentation` owns a native asynchronous sheet and its host; `confirm` provides affirmative/cancel helpers. Parent input and accessibility actions are blocked while other windows remain interactive. Initial editable/default focus, retained parent responder, native close requests, explicit dismissal, handle destruction, parent closure, failed/duplicate presentation, and completion-driven sheet replacement have regression coverage. AppKit sheets suppress ordinary traffic-light close controls; native close-request coverage exercises the window delegate path.

The Lab includes editable review and destructive reset sheets, current-theme propagation, and `--render-alert` previews. Dark and narrow light previews were inspected. Portable alert coverage includes 37 checks and 2x raster rendering. The alert checkpoint portable total was 1,051 checks, including 81 table and 28 workbench checks, plus the previous UI behavior/performance and C++ consumer regressions (16 selected CTest tests per configuration). Both full builds and all of those portable regressions passed in Debug and RelWithDebInfo; the current combined checkpoint is recorded above.

**Native verification complete:** the dedicated `scars.native_alerts` suite passes in both configurations for real sheet attachment, parent-only input blocking, action roles/default/cancel relationships, candidate geometry, composition-first Return, Escape, native close requests, responder retention, replacement callbacks, and teardown. The broader native/action/foreground suites also pass on the restored desktop, including actual secure-input ownership, frame settling, and one-shot redraw scheduling.

- [x] **Alert:** composable content and action IDs with explicit default, cancel, and destructive roles.
- [x] **Confirmation helpers:** return the selected action with exactly-once completion.
- [x] Generalize the macOS mounting adapter to the existing native surface abstraction so SCARS content can occupy a dialog/sheet while preserving current window call sites.
- [x] Handle modal focus entry/restoration, parent blocking, titlebar closure, parent closure, teardown, and callbacks that change presentation state.

Use native dialog/sheet transport with SCARS-rendered content and themes. Test modality across multiple windows and ensure Return/Escape do not prematurely interrupt text composition or multiline editing.

## Phase 7 Date and time controls

The portable date/time implementation is present in its own SCARS module implementation unit. It uses proleptic Gregorian civil values, pinned ICU locale patterns/digits and explicit IANA-zone conversion, without platform date layouts or the machine's implicit time zone. DatePicker and TimePicker extend the existing draft/validation/editor path through private codecs. Local and `chrono::sys_seconds` DateTimePicker overloads keep civil values and instants distinct. Nonexistent wall times are rejected; repeated times require reject/earlier/later policy, while an unchanged existing instant retains its known offset.

DatePicker has a six-week localized calendar using shared SCARS popovers, month/year controls, day/week and month/year keyboard navigation, disabled dates, scoped focus, constrained scrolling, and owner lifecycle cleanup. TimePicker supports explicit 12/24-hour display, optional seconds, caret-based component stepping, AM/PM toggling, inline repeat buttons, and hidden-second retention. The Lab includes local/zoned forms and locale/display controls; dark and narrow light calendar previews were inspected. Inspector and native input mapping include temporal fields, calendar values/open state, Option-Down, and validation.

At the shared date/time draft checkpoint, both full builds and Debug/RelWithDebInfo portable regressions passed 1,158 SCARS checks plus the previous UI behavior/performance and C++ consumer checks (17 selected CTest tests per configuration), including 107 date/time checks covering locale/boundary round trips, leap years, strict parsing, ranges/disabled dates, draft resets, hidden precision, component stepping, calendar navigation and lifecycle, constrained layout, and daylight-saving gaps/overlaps. Dedicated native date/time input assertions pass for drafts, marked text and candidate geometry, native calendar invocation/selection, disabled/selected semantics, stepper actions, read-only behavior, and inert held accessibility objects after closure. Dedicated native date/time and alert regressions pass in Debug and RelWithDebInfo. Distinct overlap instants also invalidate pending drafts even when their displayed wall times match. Required foreground/rendering gates now pass in both configurations alongside phases 4–6.

**Joint draft checkpoint:** DateTimePicker now stages its date and time components together, publishes only valid combined values, and supports explicit group commit/cancel. The bounded overnight transition (`2026-11-02 23:00` to `2026-11-03 01:00` within `22:00`–`02:00` limits) is covered without invalid intermediate model writes. Regressions include prospective validation while typing, cancellation/composition, focus/window/hide/disable/collapse/unmount lifetimes, rejected and normalized setters, source/range changes, deferred component blur, calendar staging, callback-driven removal, reentrant commits, and construction purity. Both full builds and all 17 selected portable regression suites pass in Debug/RelWithDebInfo, totaling 1,158 SCARS checks. The dedicated native date/time and alert suites also pass in both configurations, including native joint-component protocol coverage. Calendar selection now honors chained accent styling. The final foreground gates pass in both configurations; phase 7 meets the completion criteria.

- [x] **DatePicker:** calendar popover, month/year navigation, editable date, range limits, disabled dates, and keyboard operation. Start with the Gregorian calendar.
- [x] **TimePicker:** editable time components, stepper support, and explicit 12/24-hour display.
- [x] **DateTimePicker:** compose date and time editing with explicit local date/time versus time-zone-qualified instant semantics.
- [x] Keep model values separate from locale display. Define ambiguous and nonexistent local-time handling when zoned values are supported.

Verify leap years, month/year boundaries, limits, locale parsing/formatting, cancellation, and daylight-saving transitions for zoned values.

## Phase 8 Color controls

ColorWell and ColorPicker implementations are present. The compact well opens a shared anchored picker; the inline picker uses the same RGB/HSV sliders, numeric fields, hex editor, optional alpha, and checkerboard preview. Pure color conversion/formatting lives in its own SCARS module implementation unit. Application colors remain straight 8-bit encoded sRGB packed ARGB; displayed hex uses CSS RGB(A), with explicit channel reordering and nibble expansion. Alpha-disabled editing preserves source alpha; interactive HSV retains hue through achromatic changes.

Both controls use host-owned Apply/Cancel drafts by default. Explicit live preview tracks the original source and owned preview, restoring through the binding only if the application value still matches. External updates replace drafts without stale rollback. Lifecycle cleanup covers hiding, disabling, collapsing, removal, window blur/close, and host destruction outside view construction. Rejected setters retain feedback and can be retried; normalized accepted values are read back. Lab examples, inspector metadata, API documentation, native standard control roles, composition-first Return/Escape, and keyboard sliders are included.

**Verification checkpoint:** all 62 portable color checks pass in Debug/RelWithDebInfo. Coverage includes 4,096 conversion samples in one round-trip assertion, default/preview drafts, hex/alpha, external updates, rejected/normalized setters, lifecycle and reentrant callbacks, construction purity, HSV hue retention, RGB/HSV transitions, outside dismissal, and constrained 2× rendering. Retained editor identity now preserves visible raw-text feedback across rebuilds and retires invalid text on hide/disable/source replacement. Dark and narrow light previews were inspected. Both full builds and all 18 selected portable regression suites pass, totaling 1,220 SCARS checks plus the original UI behavior/performance and C++ consumer regressions.

The dedicated native color suite passes in both configurations for hex validation, Apply, live preview, composition/candidate geometry, Escape rollback, read-only actions, accessibility values, window closure, and held-object lifetime. Native date/time and alert suites pass alongside it. All three broader native/action/foreground suites also pass on the restored desktop, establishing the required foreground, scheduling, rendering, and secure-input coverage. Phase 8 meets the completion criteria.

- [x] **ColorWell:** a compact bound swatch that opens the picker.
- [x] **ColorPicker:** RGB/HSV editing, hexadecimal input, optional alpha, transparency preview, and keyboard operation.
- [x] Use draft values with Apply/Cancel by default and make live preview explicit. Define behavior when the bound value changes externally while editing.
- [x] Define conversion between display hex notation and SCARS' existing packed ARGB color representation, including color-space and alpha rules.

Verify conversion round trips, boundary values, alpha, invalid input, dismissal, cancellation, and external updates without stale overwrites.

## Completion criteria

Every phase must meet these criteria before its checkboxes are completed:

- Document public interfaces, state ownership, identity, lifecycle, keyboard behavior, and relevant validation or dismissal policies.
- Provide working SCARS Lab examples and update the API guide. Add inspector properties for new layout and control state where useful.
- Cover focus, accessibility, enabled/hidden states, z-order, clipping, and applicable input-method behavior through meaningful interface-level tests.
- Check both themes, constrained windows, and multiple display scales. Keep focus accents visible and outside control content.
- Pass appropriate behavioral and rendering checks in Debug and RelWithDebInfo, preserving existing SCARS and previous UI regressions.
- Run native input, windowing, and accessibility checks on a usable macOS desktop with Metal. Record unavailable checks separately from passing checks; native verification remains required for native behavior.
- Maintain the Apple Clang 21 build. Validate module interfaces with upstream Clang 23, GCC 16, and MSVC as toolchains become available; report compiler/dependency coverage separately from native runtime support.

All eight phases and required available-platform verification are complete. Maintain these regression gates for future changes. Upstream Clang 23, GCC 16, and MSVC coverage remains unvalidated and must be recorded separately when those toolchains and matching dependencies become available; Linux/Windows native backends remain outside this roadmap.