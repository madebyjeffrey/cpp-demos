# SCARS public interface → Rust completion inventory

Reference: [`scars.cppm`](../scars/src/scars.cppm). Reviewed 2026-11-05 against
`82583e7`. This maps exported control/model types, option families and view
modifiers. It records coverage, not a claim of behavioral parity or passed audits.
`Existing` means a callable Rust equivalent exists; `Partial` or `Pending` names
its closure item in the [completion checklist](rust-completion-checklist.md).

Rust deliberately uses ordinary numbers/`Length`, typed enums, standard byte ranges,
owned snapshots and private description/ownership internals. C++ spelling, literal
operators, inheritance, storage layout and provider signatures are not requirements.
The older UI engine and C++ `detail` namespace are excluded.

## Foundation, rendering and input

| C++ types/family | Rust equivalent and status | Closure |
| --- | --- | --- |
| `Color` | Existing ARGB u32 styling and typed sRGB `Color` value | M6.5 |
| `Point`, `Size`, `Rect`, `Length`, `Constraints`, `Insets` | Geometry/points/percent/Auto and per-edge insets implemented; constraints represented by min/max style and viewport, not a duplicate public constraint object | M1.2–M1.3 |
| `Align`, `Justify`, `Position`, `Orientation`, `Density`, `TextWrap`, `TextOverflow`, `LabelStyle` | Typed style enums and required variants implemented; regression coverage in styles/reflow tests | M1.2 |
| `Theme` | Light/dark palette, inherited density/accent and label roles implemented; full Lab coverage remains | M1.2, M7.3 |
| `State`, `Binding` | Existing tracked owned Rust handles; thread confinement enforced by types | M8.4 |
| `View` | Existing immutable descriptions, typed constructors and `view!`; description internals stay private, inspection uses snapshots | M1.2, M7.3 |
| `TextInlineBox`, `TextSpan`, `TextLayoutOptions`, `TextPlacement`, `GlyphPosition`, `TextLine`, `TextLayout`, `TextLayoutEngine` | `Measure`/`TextLayout`, styled `Span`, inline `View` boxes, UTF-8 visual `TextLine` geometry/caret affinity and cached SkParagraph implemented. Renderer types stay out of the portable core | M1.2, M2.2 |
| `TextRange`, `TextSelection`, `TextCommand`, `TextBuffer` | Standard byte ranges/`Selection`, reusable `TextBuffer`/`TextOptions`, visual/logical commands, gestures, composition and bounded/coalesced history implemented | M2.1–M2.3 |
| `Clipboard`, `TextInputSnapshot` | Native clipboard and Rust `TextInputClient`/`TextInputState`; portable `TextInput` | M2.3, M5.4–M5.5 |
| `PointerKind`, `PointerEvent`, `Key`, `KeyEvent`, `Cursor` | Portable/native events include both wheel axes, capture cancellation, text gestures/commands and resize cursor transport; final interactive audit remains | M5.4–M5.5 |
| `Role`, `Element`, `LogicalItem`, `LogicalCell`, `LogicalColumn` | Portable tree/table metadata and lazy native outline/table rows, cells, columns, headers and guarded actions are implemented; typed control/presentation roles, validation/help and guarded actions are implemented; combined notification/interactive audits remain | M5.1–M5.4 |
| `HostOptions`, `Host` | Host refresh/layout/input/mount exist; relevant invalidation, cursor, scroll, text, selection, presentation, inspection and logical-query contracts are assigned across the checklist | M1.4–M1.5, M2, M3, M4, M5, M7 |

Public Unicode/UTF conversion helpers map to `text`; SkParagraph construction to
`Renderer`. `automatic`, `infinity`, `percent` and C++ length literals map to
`Length::Auto`, `f64::INFINITY`, `percent` and ordinary numeric arguments.
Rich inline text/boxes need a portable description and measurement interface;
wrapping a row of independent labels alone does not close that behavior.

## Controls, models and option families

| C++ types/family | Rust equivalent and status | Closure |
| --- | --- | --- |
| `Label`, `Text`, `Button`, `CommandButton` | Existing label/button constructors and macro aliases | M1.2 text options, M5.3, M7.3 |
| `ToggleButton`, `CheckBox`, `CheckState`, `Switch`, `RadioGroup`, `SegmentedControl`, `Option` | Existing bound controls, `CheckState`, `OptionItem` | M1.5, M5.3, M7.3 |
| `Slider`, `SliderOptions` | Stepped bounded slider with pointer/key/capture lifetime; configurable page-step remains for full Lab/inventory closure | M7.3, M8.6 |
| `ProgressBar` | Existing determinate/indeterminate progress | M5.3, M8.4 |
| `Bitmap`, `Image`, `ContentMode` | Existing validated encoded/RGBA data and fit/fill/stretch/center rendering; resource lifetime and preview coverage retained | M7.3, M8.2–M8.4 |
| `FlexRow`, `FlexColumn`, `Block`, `Inline`, `Layer` | Row/column/block/inline/layer and shaped rich inline text implemented with width-dependent reflow and baselines | M1.2–M1.3 |
| `Section`, `GroupBox`, `Panel`, `Spacer`, `Separator` | Grouping/spacer and both separator orientations implemented | M1.2 |
| `ForEach`, `If` | Rust `for`/`if` inside `view!`, stable keyed descriptions; no duplicate wrapper classes required | M7.3 |
| `ScrollView`, `ScrollAxes`, `ScrollViewOptions` | Both-axis scrolling, overlay bars, clamped queries/actions, nested wheel transport, keyboard/caret reveal and capture lifetime implemented | M1.4–M1.5 |
| `TextField`, `TextFieldOptions`, `TextEditor` | Single/multiline controls share TextBuffer; readonly/length/history policy, visual navigation and word/line selection implemented | M2.1–M2.3 |
| `SearchField`, `SearchFieldOptions` | Integrated clear/cancel, committed query/submit hooks, scoped one-shot debounce and lifetime cancellation implemented | M2.4 |
| `PasswordField`, `PasswordFieldOptions` | Explicit reveal/undo, readonly/grapheme limits, uncached sensitive painting and clipboard/AX restrictions implemented; actual native ownership transitions tested | M2.5 |
| `NumericField`, `Stepper`, `NumericOptions`, `NumericStatus`, `NumericValidation` | Exact `Numeric<T>`/`Draft<T>`, `Validation`/`ValidationStatus`, precision/blur/submit/cancel and standalone repeating `stepper`, native direction availability/button links and automatic validation semantics implemented; checked `EditLocale` numeric grammar implemented for five profiles; shared typed-control expansion remains | M2.6, M6.1–M6.2, M6.6 |
| `MenuButton`, `MenuItem`, `MenuItemKind`, `Action` | `Menu`/`MenuAction` tree: separator/check/radio/submenu, shortcut hints, disabled navigation/typeahead; `MenuInfo`, idempotent native show/cancel, opener links and submenu hierarchy; legacy `Popup::menu` supported | M3.2, M5.3 |
| `SelectionDropdown` | Dropdown uses stable option IDs with full menu navigation/check/radio/submenu support; combined native semantic audit remains | M3.2, M5.3 |
| `SplitActionButton` | Retained `SplitAction`: selected leaf key path, primary invocation and unavailable-choice fallback; fixed `split_button` helper retained | M3.2 |
| `OverlayPlacement`, `PopoverFocus`, `TooltipOptions`, `PopoverOptions` | `OverlayOptions`/`OverlayPlacement`/`OverlayFocus` and `TooltipOptions`: fitted/scrolled portals, nested focus return, explicit dismissal reasons, help and owner lifetimes | M3.3, M5.3 |
| `SplitView`, `SplitViewOptions`, `SplitPane` | `Split` model and retained panes, draggable dividers, both axes/minima, collapse/restoration, keyboard/cursor/capture policy and native split semantics implemented | M3.1 |
| `Tab`, `TabView` | `Tabs`/`Tab` retain visited keyed pages, undo/selection/scroll/focus, enabled fallback and nested keyboard navigation; lazy construction and native tab-group semantics | M3.4 |
| `Wizard`, `WizardStep`, `WizardOptions`, `WizardNavigation` | `Wizard`/`WizardStep`/`WizardOptions`: stable IDs, reactive validation, Back/review/final policies, callbacks and atomic changed step sets; retained `wizard_view`, standalone `wizard_navigation` and legacy positional helper | M3.5 |
| `Alert`, `AlertAction`, `AlertActionRole` | Single-use `Alert`/`AlertResult` transactions, typed action roles, Return/Escape policy and once-only lifetime completion; component overlays and application-owned native `Surface::show_sheet` share the model; native dialog/default/cancel semantics and actions | M3.6, M5.3 |
| `RowWindow`, `RowModel`, `ListBox`, `ListBoxOptions` | `ListModel`/`ListItem`/`ListOptions` provide keyed selection, navigation, activation/typeahead and configurable realization; bounded `RowPin` retention preserves active editors, with distinct host/AX lifetimes after reinsertion | M4.1–M4.2 |
| `ComboBox`, `ComboBoxOptions`, `ComboBoxMode`, `ComboBoxFilter` | `ComboBox` owns independent query/choice, configured modes/filter/loading, keyboard/pointer acceptance and cancellation; virtual suggestions and native popup links replace the capped helper | M4.5 |
| `TreeNode`, `TreeItem`, `TreeModel`, `TreeView`, `TreeViewOptions` | `TreeNode` input, `TreeItem`/`TreeRowInfo` metadata and `TreeOptions`; checked loading/navigation/selection plus logical AppKit outline hierarchy/actions, loading/error descriptions and lazy immutable row arrays; combined interactive audits remain | M4.3–M4.4, M5.1 |
| `TableCell`, `TableSelection`, `TableSelectionMode`, `TableColumnWidth`, `TableColumn`, `SortOrder`, `SortDescriptor`, `TableModel`, `TableView`, `TableViewOptions` | `TableRow` keyed text and checked column geometry/order, multi-sort and row/cell selection; integrated pinned edits, validation/readonly/custom display and generation-checked commits; callback-free `TableSnapshot`/`TableColumnInfo`/`TableCellInfo`, lazy native cells/columns/headers and bounded logical actions; combined interactive audits remain | M4.6–M4.7, M5.2 |
| `ColorPicker`, `ColorWell`, `ColorPickerOptions`, `ColorModel`, `HSV` | Existing sRGB RGBA/hex picker/well; HSV and alpha/readonly/live-preview/transaction policies missing | M6.5–M6.6 |
| `Date`, `Time`, `LocalDateTime`, `DateTimeResolution`, `RepeatedTimePolicy`, `DateTimeValidation` | Jiff civil/zoned values, explicit resolution and strict ISO consumption; checked numeric Gregorian date localization implemented; full time/value policies remain | M6.1–M6.4, M6.6 |
| `DatePicker`, `DatePickerOptions` | ISO and checked localized date drafts/calendar labels, week start, Arabic grid direction and narrow cell layout implemented; bounds/disabled-date/validation, readonly, keyboard and commit policies partial | M6.3 |
| `TimePicker`, `TimePickerOptions`, `HourCycle` | ISO `time_draft` exists; dedicated time interactions, hour-cycle/seconds/step/bounds policies missing | M6.4 |
| `DateTimePicker`, `DateTimePickerOptions`, `ZonedDateTimePickerOptions` | Local ISO draft and `ZonedPicker` exist; combined controls, bounds and validation/commit options incomplete | M6.4–M6.6 |

Color format/parse/HSV functions belong to M6.5. Numeric display/parse/configuration
helpers belong to M2.6/M6.1–M6.2; exact integers must not pass through f64. Date/time
format/parse/validity/local-time conversion/resolution and calendar-page helpers
belong to M6.1–M6.4. These helper names need not be copied into the Rust interface.

## View modifiers and annotations

Every public C++ `View` modifier is listed, including those with existing Rust names.

| C++ modifier family | Rust mapping / remaining work | Closure |
| --- | --- | --- |
| `key`, `enabled`, `visible`, `hidden`, `accessible_label` | Existing matching modifiers; lifetime, interaction and semantic consequences require coverage | M1.5, M5.3–M5.4 |
| `width`, `height`, `min_width`, `max_width`, `min_height`, `max_height` | Existing typed dimensions and limits; min/max redistribution and percentage reflow coverage | M1.3 |
| `padding`, `margin` | Scalar and per-edge `Insets` overloads implemented | M1.2 |
| `gap`, `grow`, `shrink`, `basis` | Basis and min/max-aware grow/shrink redistribution implemented | M1.2–M1.3 |
| `wrap`, `align_items`, `align_self`, `justify` | Wrapping, `align`, self-alignment, space-around and width-dependent reflow implemented | M1.2–M1.3 |
| `foreground`, `background`, `accent`, `corner_radius` | Foreground/background/`radius` and inherited accent implemented | M1.2 |
| `font_size`, `bold`, `label_style`, `text_wrap`, `text_overflow`, `max_lines`, `text_align`, `uppercase`, `density` | Typed text/density roles, line limits, ellipsis, alignment and spacing implemented and raster tested | M1.2 |
| `position`, `left`, `top`, `right`, `bottom`, `z_index` | Flow/relative/absolute/anchors/z-order implemented with constrained layout and subtree hit-order coverage | M1.2–M1.3 |
| `content_mode` | Existing image constructor mode (`content_mode:` in macro); no need for a second stateful modifier | M7.3 |
| `expanded` | Checked expansion/loading/failure metadata and callbacks, keyboard/pointer interaction and logical native disclosure links/actions are integrated; combined interactive auditing remains | M4.3–M4.4, M5.1/M5.3 |
| `tooltip`, `popover`, `context_menu` | Configurable `tooltip`/`Popup` and attached `View::context_menu`: nearest-owner pointer/keyboard invocation, hierarchy, scope and lifetime policies; legacy `on_context` supported | M3.2–M3.3 |
| `description` | Private immutable Rust description, public owned host snapshots; deliberate information hiding | M7.1 |
| `labeled` free annotation helper | Explicit accessible labels exist; annotation/name association remains part of Lab/semantics coverage | M5.3, M7.3 |

## Inspection and host operations

| C++ types/family | Rust mapping / remaining work | Closure |
| --- | --- | --- |
| `LayoutOverride`, `InspectionOverlay`, `FlexLineInspection`, `SplitInspection`, `TabInspection`, `ScrollInspection`, `ListInspection`, `InspectionNode`, `Inspector` | `Inspection`, `Inspector`, node snapshots and whole-style overrides exist; hierarchy, box/flex/split/tab/scroll/collection/draft/semantic state, overlay modes and UI editing need completion | M7.1–M7.2 |
| `Host` construction/options/theme/text-layout assignment/refresh | Rust host constructor, `set_theme`, `layout(..., Measure)` and refresh; native connection provided by window mounting | M1, M7.3 |
| `Host` pointer/key/clipboard/editing/focus/cursor and text-input queries | Existing core dispatch and native adapter; input/geometry/secure/notification completion assigned above | M1.5, M2, M5 |
| `Host` layout/scroll/list/tree/table/logical queries and actions | Snapshots/models exist; bounded logical semantics and full query/action behavior must be completed without forcing native types into the core | M1.4, M4, M5 |
| `Host` menu/popover/tooltip/alert presentation, scheduling and invalidation | Menus/popovers/tooltips/alerts have scoped lifetimes and dismissal/completion callbacks; native sheets share alert transactions; final idle/cancel regression evidence remains | M3.6, M8.4 |
| `Host` inspection/picking/layout overrides/previews | Existing picking and width UI; full developer tool contracts and diagnostic previews pending | M7, M8.2 |

Run `python3 scripts/check_rust_inventory.py` after changing the reference or matrix.
It verifies that exported type names and all `View` modifier names are represented;
it does not certify implementation, option-level behavior or tests. Native adapters
in `scars.macos.cppm` remain reference evidence for M3/M5/M8, alongside the existing
Rust window-management contracts.
