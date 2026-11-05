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
| `Point`, `Size`, `Rect`, `Length`, `Constraints`, `Insets` | Geometry/points/percent/Auto exist; constraints represented by min/max style and viewport, not a duplicate public constraint object; per-edge insets pending | M1.2–M1.3 |
| `Align`, `Justify`, `Position`, `Orientation`, `Density`, `TextWrap`, `TextOverflow`, `LabelStyle` | Partial style enums; missing variants/options mapped below | M1.2 |
| `Theme` | Existing light/dark palette; density, label/accent roles, font configuration and remaining relevant style values partial | M1.2, M7.3 |
| `State`, `Binding` | Existing tracked owned Rust handles; thread confinement enforced by types | M8.4 |
| `View` | Existing immutable descriptions, typed constructors and `view!`; description internals stay private, inspection uses snapshots | M1.2, M7.3 |
| `TextInlineBox`, `TextSpan`, `TextLayoutOptions`, `TextPlacement`, `GlyphPosition`, `TextLine`, `TextLayout`, `TextLayoutEngine` | `Measure`/`TextLayout` and cached SkParagraph renderer exist; span/inline-box, line geometry and richer text options remain. Keep renderer types out of the portable core | M1.2, M2.2 |
| `TextRange`, `TextSelection`, `TextCommand`, `TextBuffer` | Standard byte ranges/`Selection` and host editor commands exist; reusable buffer and remaining navigation/selection/history behavior pending | M2.1–M2.3 |
| `Clipboard`, `TextInputSnapshot` | Native clipboard and Rust `TextInputClient`/`TextInputState`; portable `TextInput` | M2.3, M5.4–M5.5 |
| `PointerKind`, `PointerEvent`, `Key`, `KeyEvent`, `Cursor` | Portable/native event types exist; full click/scroll/cancel transport and keyboard command coverage incomplete | M1.5, M2.2–M2.3, M3.1 |
| `Role`, `Element`, `LogicalItem`, `LogicalCell`, `LogicalColumn` | Native semantic adapter and logical row proxies exist; complete portable semantic snapshots, cells/columns/tree metadata/actions pending | M5.1–M5.4 |
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
| `Slider`, `SliderOptions` | Existing stepped bounded slider; page-step configuration and complete gesture/key/lifetime behavior partial | M1.5, M2.6, M5.3 |
| `ProgressBar` | Existing determinate/indeterminate progress | M5.3, M8.4 |
| `Bitmap`, `Image`, `ContentMode` | Existing validated encoded/RGBA data and fit/fill/stretch/center rendering; resource lifetime and preview coverage retained | M7.3, M8.2–M8.4 |
| `FlexRow`, `FlexColumn`, `Block`, `Inline`, `Layer` | Existing row/column/block/inline/layer; complete layout and rich inline text behavior partial | M1.2–M1.3 |
| `Section`, `GroupBox`, `Panel`, `Spacer`, `Separator` | Existing grouping/spacer and horizontal separator; orientation/styles partial | M1.2 |
| `ForEach`, `If` | Rust `for`/`if` inside `view!`, stable keyed descriptions; no duplicate wrapper classes required | M7.3 |
| `ScrollView`, `ScrollAxes`, `ScrollViewOptions` | Vertical `scroll` exists; axes, scrollbar and input/query contracts incomplete | M1.4–M1.5 |
| `TextField`, `TextFieldOptions`, `TextEditor` | Existing single/multiline editable controls; maximum length, visual navigation, rich gestures and remaining editor policies partial | M2.1–M2.3 |
| `SearchField`, `SearchFieldOptions` | Currently a plain field alias with generic deadline primitives; integrated search semantics/options missing | M2.4 |
| `PasswordField`, `PasswordFieldOptions` | Masking/native secure input/clipboard restrictions exist; reveal, undo and length policies partial | M2.5 |
| `NumericField`, `Stepper`, `NumericOptions`, `NumericStatus`, `NumericValidation` | Exact `Numeric<T>`/`Draft<T>` and +/- composition exist; standalone repeat stepper and complete precision/commit/validation policies pending | M2.6, M6.1–M6.2, M6.6 |
| `MenuButton`, `MenuItem`, `MenuItemKind`, `Action` | Flat `Popup::menu`/`MenuAction` composition; menu kinds, hierarchy, metadata and navigation partial | M3.2 |
| `SelectionDropdown` | Existing dropdown/button+overlay composition; richer menu navigation and semantics pending | M3.2, M5.3 |
| `SplitActionButton` | Existing primary+arrow composition; selected default action and full menu behavior pending | M3.2 |
| `OverlayPlacement`, `PopoverFocus`, `TooltipOptions`, `PopoverOptions` | Basic anchored/clamped popover, modal scope and delayed tooltip; configurable placement/focus/dismissal/size/help policies partial | M3.3, M5.3 |
| `SplitView`, `SplitViewOptions`, `SplitPane` | Two-pane composition with ratio slider; divider/orientation/collapse/minima and native semantics missing | M3.1 |
| `Tab`, `TabView` | Segmented selection mounts only active view; retained host-owned state/focus and enabled/key behavior incomplete | M3.4 |
| `Wizard`, `WizardStep`, `WizardOptions`, `WizardNavigation` | Simple index/back/next composition; validation, stable steps, review, finish/cancel/change policies missing | M3.5 |
| `Alert`, `AlertAction`, `AlertActionRole` | Basic Done popup and low-level native dialog exist; component result transaction/action roles and complete native sheet integration missing | M3.6 |
| `RowWindow`, `RowModel`, `ListBox`, `ListBoxOptions` | Existing `ListModel`/`ListItem`, fixed-height virtual range/row builder and keyed selection; complete selection/activation/retention policies partial | M4.1–M4.2 |
| `ComboBox`, `ComboBoxOptions`, `ComboBoxMode`, `ComboBoxFilter` | Editable field plus capped suggestion menu; independent query/selection, loading/filter policies and full interaction pending | M4.5 |
| `TreeNode`, `TreeItem`, `TreeModel`, `TreeView`, `TreeViewOptions` | Flattened expanded Rust tree; integrated lazy metadata/loading, full navigation and semantics missing | M4.3–M4.4, M5.1 |
| `TableCell`, `TableSelection`, `TableSelectionMode`, `TableColumnWidth`, `TableColumn`, `SortOrder`, `SortDescriptor`, `TableModel`, `TableView`, `TableViewOptions` | Virtual string-cell rows, simple sort and conflict-checked cell drafts exist; keyed column model/resize/order, selection, integrated editing and semantics partial | M4.6–M4.7, M5.2 |
| `ColorPicker`, `ColorWell`, `ColorPickerOptions`, `ColorModel`, `HSV` | Existing sRGB RGBA/hex picker/well; HSV and alpha/readonly/live-preview/transaction policies missing | M6.5–M6.6 |
| `Date`, `Time`, `LocalDateTime`, `DateTimeResolution`, `RepeatedTimePolicy`, `DateTimeValidation` | Jiff civil/zoned values and explicit resolution, typed drafts; validation/status and localized formatting partial | M6.1–M6.4, M6.6 |
| `DatePicker`, `DatePickerOptions` | Calendar and ISO draft exist; bounds/disabled-date/validation, readonly, keyboard and commit policies partial | M6.3 |
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
| `padding`, `margin` | Existing uniform values; per-edge `Insets` overloads missing | M1.2 |
| `gap`, `grow`, `shrink`, `basis` | First three exist; basis and min/max-aware redistribution pending | M1.2–M1.3 |
| `wrap`, `align_items`, `align_self`, `justify` | `wrap`, `align`, `justify` exist; self-alignment/space-around and full reflow partial | M1.2–M1.3 |
| `foreground`, `background`, `accent`, `corner_radius` | Existing foreground/background/`radius`; inherited accent pending | M1.2 |
| `font_size`, `bold`, `label_style`, `text_wrap`, `text_overflow`, `max_lines`, `text_align`, `uppercase`, `density` | Size/bold/basic wrap exist; remaining typed text/density roles and shaping/paint behavior pending | M1.2 |
| `position`, `left`, `top`, `right`, `bottom`, `z_index` | Flow/absolute/anchors/z-order exist; relative positioning and regression coverage pending | M1.2–M1.3 |
| `content_mode` | Existing image constructor mode (`content_mode:` in macro); no need for a second stateful modifier | M7.3 |
| `expanded` | Tree expansion state exists separately; public disclosure annotation and semantics partial | M4.3–M4.4, M5.1/M5.3 |
| `tooltip`, `popover`, `context_menu` | Existing `tooltip`, `Popup` compositions and `on_context`; full options and menu-tree contracts pending | M3.2–M3.3 |
| `description` | Private immutable Rust description, public owned host snapshots; deliberate information hiding | M7.1 |
| `labeled` free annotation helper | Explicit accessible labels exist; annotation/name association remains part of Lab/semantics coverage | M5.3, M7.3 |

## Inspection and host operations

| C++ types/family | Rust mapping / remaining work | Closure |
| --- | --- | --- |
| `LayoutOverride`, `InspectionOverlay`, `FlexLineInspection`, `SplitInspection`, `TabInspection`, `ScrollInspection`, `ListInspection`, `InspectionNode`, `Inspector` | `Inspection`, `Inspector`, node snapshots and whole-style overrides exist; hierarchy, box/flex/split/tab/scroll/collection/draft/semantic state, overlay modes and UI editing need completion | M7.1–M7.2 |
| `Host` construction/options/theme/text-layout assignment/refresh | Rust host constructor, `set_theme`, `layout(..., Measure)` and refresh; native connection provided by window mounting | M1, M7.3 |
| `Host` pointer/key/clipboard/editing/focus/cursor and text-input queries | Existing core dispatch and native adapter; input/geometry/secure/notification completion assigned above | M1.5, M2, M5 |
| `Host` layout/scroll/list/tree/table/logical queries and actions | Snapshots/models exist; bounded logical semantics and full query/action behavior must be completed without forcing native types into the core | M1.4, M4, M5 |
| `Host` menu/popover/tooltip/alert presentation, scheduling and invalidation | Existing popup/deadline/mount mechanisms; lifetime, exactly-once completion and idle/cancel contracts require closure | M3, M8.4 |
| `Host` inspection/picking/layout overrides/previews | Existing picking and width UI; full developer tool contracts and diagnostic previews pending | M7, M8.2 |

Run `python3 scripts/check_rust_inventory.py` after changing the reference or matrix.
It verifies that exported type names and all `View` modifier names are represented;
it does not certify implementation, option-level behavior or tests. Native adapters
in `scars.macos.cppm` remain reference evidence for M3/M5/M8, alongside the existing
Rust window-management contracts.
