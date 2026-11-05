# Structured Control and Rendering System (SCARS)

SCARS is an independent C++23 view, state, layout, input, and rendering engine. It uses standard named C++ modules throughout its own library. It borrows the existing native Skia/Metal window backend, but never mounts or wraps `skia_window::ui`. The original gallery and Controls Lab remain separate.

The [control implementation plan](scars-controls-plan.md) and [completion audit](scars-controls-verification.md) record all eight completed phases. Both full builds and all 29 registered CTest tests pass in Debug and RelWithDebInfo, including 1,220 portable SCARS checks, all six SCARS native suites, and the previous UI/window regressions. Full foreground, frame-settling, one-shot scheduling, and secure-input gates pass on an awake/unlocked desktop. Other compiler targets remain unvalidated as detailed below.

## Build and run

The verified configuration is Apple Clang 21, CMake 4.4.4, Ninja 1.13.2, and macOS 26+. SCARS requires CMake 3.28 or newer. Compiler targets also include upstream Clang 23, GCC 16, and MSVC with C++23/module support; those builds have not been validated. Linux/Windows native backends and dependency bootstraps are not implemented.

```sh
# Existing pinned renderer, if it has not been bootstrapped already:
python3 scripts/bootstrap_skia.py --config Debug

# Additional, isolated paragraph/shaping/Unicode build:
python3 scripts/bootstrap_scars_text.py --config Debug
cmake --preset debug
cmake --build --preset debug --target scars_lab scars_tests scars_features_tests scars_tabs_tests scars_numeric_tests scars_text_controls_tests scars_overlays_tests scars_combo_tests scars_scrolling_tests scars_virtualization_tests scars_tree_tests scars_table_tests scars_alerts_tests scars_datetime_tests scars_colors_tests scars_native_tests scars_native_alerts_tests scars_native_datetime_tests scars_native_colors_tests
open build/debug/scars/scars_lab.app
```

Substitute `RelWithDebInfo` and the `relwithdebinfo` preset for the optimized build. In CLion, select the `scars_lab` target. Set `SCARS_BUILD=OFF` to build only the previous demos/library.

`bootstrap_scars_text.py` builds SkParagraph, SkShaper, SkUnicode, HarfBuzz, and ICU at the existing pinned Skia revision. It uses `.deps/skia/out/scars-text-<arch>-<config>` and leaves the existing renderer's output/configuration intact. Only the additional text archives are linked from that directory; the app links one copy of the existing core Skia archive. CMake checks revision, architecture, configuration, deployment target, and required archives. CMake never downloads or builds Skia implicitly.

The app bundles Roboto Regular, its license, and the text stack's third-party notices. Moving the app bundle does not require the source checkout for font access. Other applications may provide a font file through `Theme::font_path`, or use platform font discovery by leaving it empty.

The [native verification guide](scars-native-verification.md) describes foreground requirements and the standalone desktop probe (`python3 scripts/check_scars_desktop.py`). A visible window alone does not establish display-link delivery or key-window ownership. Run native configurations sequentially; keep unavailable foreground checks separate from passing protocol coverage.

## Modules and linking

```cmake
target_link_libraries(my_app PRIVATE scars::macos)
```

```cpp
#include <skia_window/skia_window.hpp>
import scars;
import scars.macos;

namespace s = scars;
struct Settings {
    s::State<bool> grid{true};
    s::State<std::string> quality{"normal"};
    s::State<std::string> mode{"flight"};

    s::View body() {
        return s::FlexColumn{
            s::Label("Display settings", s::LabelStyle::heading),
            s::CheckBox("Show grid", grid.binding()).key("grid"),
            s::FlexRow{
                s::SelectionDropdown(quality.binding(), {
                    {"normal", "Normal"}, {"high", "High fidelity"}
                }).accent(0xff4dd2ff),
                s::Button("Apply", [] { /* application command */ }),
            }.gap(12).wrap(),
        }.gap(16).padding(24);
    }
};

auto model = std::make_shared<Settings>();
auto host = s::macos::mount(*window, [model] { return model->body(); });
```

When adding this project as a subdirectory, set `SCARS_BUILD=ON` explicitly (subprojects default it off). Link `scars::scars` for the native-independent host without the window adapter. No SCARS public headers are required. Consumers should include the standard headers they use (for example, `<string>` and `<vector>`); `import scars` does not export the standard library. Standard-library and third-party headers are included inside global module fragments; this does not require header units or `import std`.

The Apple target propagates standard C++23 mode to importers, including consumers whose parent project defaults to GNU extensions. Apple's installed compiler supports named modules with `-fcxx-modules` but ships no `clang-scan-deps`. The Apple CMake path explicitly orders this small module graph, tracks header dependencies, and propagates `-fmodule-file-deps=all` so interface edits also rebuild importing sources. BMIs are built separately for each configuration and must never be shared across compilers, architectures, SDK/deployment targets, or incompatible compiler flags. The other compiler path uses CMake's `CXX_MODULES` file sets.

## State and identity

`State<T>` and `Binding<T>` share a cell. Reads while building or mounting subscribe the current host. A changed value invalidates dependent hosts; repeated writes coalesce until the next layout/query/paint, and equality-comparable values skip unchanged writes. Removed dependencies stop invalidating their former host. State and hosts belong to their creating thread. Mutating state during view construction is rejected.

Views are immutable descriptions. Every modifier returns a new value, so styling a copy cannot change its source. Use `.key("stable-id")` for children that can reorder: focus and scroll state follow mounted identity. Duplicate sibling keys are rejected. Removing a keyed view removes its mounted control state. Business state belongs to the view model.

The native adapter owns the host through window callbacks. Its wake connection expires when unmounted and is cancelled when the window closes, so retaining a host or a binding afterward is safe. `scars::macos::unmount(window)` removes both callbacks and the accessibility provider. Mount SCARS on a surface dedicated to it; the adapter owns that surface's draw/input callbacks.

## Layout

All geometry uses logical pixels; the existing native canvas applies display scaling.

| View | Placement |
| --- | --- |
| `FlexRow{...}` | Horizontal flex, optional wrapping |
| `FlexColumn{...}` | Vertical flex |
| `Block{...}` | Vertical block flow, full available width by default, no flex shrinking |
| `Inline{...}` | Styled paragraph spans and indivisible embedded views on text baselines |
| `ForEach(...)`, `If(...)` | Transparent keyed repetition and lazy conditional children |
| `Layer{...}` | Overlapping children in a shared content box, ordered by z-index |
| `ScrollView(child)` | Clipped viewport with vertical, horizontal, or combined-axis scrolling and focus reveal |
| `SplitView(first, second, fraction, options)` | Two resizable panes with a bound fraction, axis minima, and optional collapse |
| `TabView(selection, tabs)` | Stable tab IDs, lazy page builders, and retained visited pages |
| `Section(title, child)` | Open semantic grouping, optional bound expansion |
| `GroupBox(title, child)` | Bordered semantic grouping |
| `Panel(child)` | Filled supplementary surface |

Sizing modifiers include `width`, `height`, min/max dimensions, `padding`, `margin`, `gap`, `grow`, `shrink`, `basis`, `align_items`, `align_self`, and `justify`. Dimensions accept logical lengths (`120`, `120_lp`), percentages (`percent(50)`, `50_pct`), or `automatic`. Insets can be uniform or explicit. Explicit sizes include padding; margins remain outside the assigned bounds. Unresolved percentages contribute zero when measured against unbounded space. Constraints, allocated bounds, and intrinsic measurements are distinct.

SplitView reserves a divider inside its content box and assigns the remaining extent by a finite bound fraction. Pane minima constrain the effective fraction; when their sum exceeds available space, both minima scale proportionally. Window resizing never writes a corrected fraction into application state. Optional bound collapse retains both pane subtrees and their editor/scroll state while hiding the collapsed subtree and giving it zero extent along the split axis. The synthetic divider participates in normal focus, paint/hit order, inspection, and accessibility as a splitter. Drag capture survives pointer movement outside its bounds and ends on release, cancellation, disabling, hiding, removal, or window deactivation. See [SplitView usage and keyboard behavior](scars-api.md#split-views).

TabView mounts its selected page lazily and retains visited pages under stable tab IDs. Inactive pages keep editor and scroll state while contributing no layout, input, accessibility, or animation. Their builders resume when selected; removing a tab releases its mounted state. A missing or disabled selected ID displays the first enabled page without writing state during construction. The selected tab is one keyboard focus stop, with arrow/Home/End navigation and scoped Control-Tab cycling. Headers scroll when they overflow the viewport. Native tab-group and page-group semantics share the same active subtree as painting and input. See [TabView usage and lifecycle](scars-api.md#tab-views).

Text-only labels inside `Inline` become paragraph spans. Labels with background, padding, margins, or explicit dimensions become inline boxes, as do controls and containers. Embedded boxes remain intact when the line wraps. Nested layout inside an embedded box still uses the normal engine. This is a defined web-inspired subset: no CSS parsing/cascade, grid, floats, margin collapse, or complete browser flexbox compatibility.

### Layers, positioning, and visibility

Every view supports `.z_index(int)`. Higher values paint above lower values among siblings; negative values are allowed. Equal values preserve declaration order, with later siblings on top. A child's z-index stays inside its parent's subtree, so a high-z child cannot jump in front of a higher-z sibling of its parent. Painting, pointer targeting, inspector picking, and native accessibility hit testing share this order. Keyboard traversal and semantic reading order retain declaration order. Popups, keyboard focus accents, and developer overlays remain host overlays above scene content.

`Layer{...}` overlaps its children instead of arranging them sequentially. Its intrinsic size is the maximum extent of its normal-flow children, including margins. Automatic child dimensions stretch to its content box by default; explicit sizes are preserved. `align_items` / `align_self` can place smaller children at the start, center, or end of both axes. Absolute children do not contribute to intrinsic size, so give a layer an explicit size or a flex allocation when all its children are absolute.

- `.position(Position::normal)` is the default. Edge offsets have no effect.
- `.position(Position::relative)` offsets a view from its ordinary layout position and leaves its original space reserved. Its descendants move with it.
- `.position(Position::absolute)` removes a child from flex/block/inline flow and anchors it to its immediate parent's content box (inside padding and below any section heading). It does not contribute to flex lines, gaps, or intrinsic parent size.

Use `.left`, `.top`, `.right`, and `.bottom` with signed logical lengths, percentages, or `automatic` to clear an anchor. Percentages resolve against the parent's content width or height. With opposing anchors and an automatic dimension, the object stretches between the anchors, accounting for margins and min/max dimensions. With an explicit dimension, left/top take precedence over right/bottom. With neither anchor on an axis, absolute placement starts at that content edge. Positioned children remain clipped to ancestor bounds and scroll viewports. The host root always occupies the viewport; positioning is applied to children.

`.hidden(true)` retains a view and its layout space, but suppresses painting, input, accessibility elements, and indeterminate animation throughout its subtree. It also clears hidden keyboard focus and closes hidden popups. `.hidden(false)` restores the same mounted identity. `.visible(false)` removes the subtree entirely, including its layout space. Use view-model state in either modifier to change visibility reactively.

```cpp
s::Layer{
    s::Panel(s::Label("Background")).z_index(-1),
    s::Button("Overlay", [] {})
        .position(s::Position::absolute)
        .right(16).top(12).width(140).height(40)
        .z_index(10).hidden(!showOverlay.get()),
    s::Label("Relative annotation")
        .width(180).height(24)
        .position(s::Position::relative).left(12).top(60)
        .z_index(2),
}.height(220);
```

SCARS Lab's **Layers & positioning** section demonstrates overlap, changing the rear layer's z-index, hiding/showing an overlay, a relative annotation, and right/bottom anchoring.

## Text layout

SCARS uses **SkParagraph + HarfBuzz + SkUnicode/ICU**, including styled runs, Unicode shaping/bidi, line breaking, ellipsis, and inline placeholders. There are no direct CoreText paragraph-layout calls in SCARS. Native font discovery/rasterization may still use Skia's platform font manager on macOS; that is separate from paragraph layout.

`Label` defaults to `TextWrap::no_wrap` and clipped overflow. Use `text_wrap(TextWrap::wrap)` or `break_word`, `text_overflow(TextOverflow::ellipsis)`, `max_lines(n)`, and `text_align(...)`. Wrapping uses SkParagraph's Unicode word boundaries and emergency cluster breaks for oversized words. Both wrapping modes currently use that behavior; a strict word-only overflow policy is not implemented. Explicit newlines work with NoWrap. `max_lines(0)` means unlimited. Micro labels default to uppercase, 11 lp / 14 lp, 0.5 lp tracking, one line, and ellipsis.

The replaceable `TextLayoutEngine` interface accepts styled UTF-8 spans and inline-box metrics. It returns a retained `TextLayout` containing size/baseline metrics, span/box placements, positioned-glyph diagnostics, and a paint operation. SkParagraph types remain private to one module implementation unit. Pass another implementation as the fourth argument to `Host` or `scars::macos::mount`; a behavioral test uses a substitute engine without initializing the default font stack. A custom engine owns its font policy.

`TextField` and `TextEditor` add plain UTF-8 editing: grapheme-aware cursor/deletion, word and paragraph navigation, mouse selection, clipboard, per-field undo/redo, internal scrolling, read-only and length constraints, and submit callbacks. AppKit's `NSTextInputClient` supplies input-method integration and converts UTF-16 ranges; editing state and layout remain in SCARS/SkParagraph. See the [API quick guide](scars-api.md#text-input) for constructors, shortcuts, and range semantics.

## Theme

`Theme{}` is dark and `Theme::light()` supplies the light palette, including text fields, lists, selection/caret colors, surfaces, disabled text, and borders. `Host::set_theme` preserves mounted editing and control state. SCARS Lab installs a native titlebar accessory at the upper right to toggle appearance; it does not occupy client space. View → Light mode (Command-Shift-L) offers the same action. The inspector follows the source host's theme. `Window::set_titlebar_theme_toggle` and `set_appearance` expose the optional native integration.

## Controls and interaction

| Control | Contract |
| --- | --- |
| `TabView` | Lazy keyed pages, retained inactive editing/scroll state, disabled tabs, keyboard navigation |
| `NumericField` | Deferred typed drafts, portable locale formatting/parsing, range validation, explicit commit/cancel |
| `Stepper` | Exact signed integer or decimal increments, bounded values, keyboard controls, cancellable pointer repeat |
| `Switch` | Bound Boolean, full-row pointer activation, Space, and native switch semantics |
| `SearchField` | Incremental/explicit search, cancellable debounce, clear/cancel, and host-scoped Command-F |
| `PasswordField` | Grapheme masking, optional reveal, restricted clipboard/history, and secure native input |
| `Wizard` / `WizardNavigation` | Bound step navigation, Back/Next/Finish/Cancel, review links and per-step validation |
| `TextField` / `TextEditor` | Single-line / wrapped multiline plain text, selection, clipboard, undo/redo, IME |
| `ListBox` | Single or multiple selection, disabled rows, scrolling, keyboard/type-ahead |
| `Image` / `Bitmap` | File or copied RGBA bitmap; fit/fill/stretch/center rendering |
| `Button` / `CommandButton` | Executes callback; no persistent selection |
| `ToggleButton` | Independent bound boolean; active/inactive filled surfaces |
| `SegmentedControl` | One group-owned selected ID; connected straight seams |
| `CheckBox` | Boolean or tri-state binding; clicking the full label row activates |
| `RadioGroup` | Group-owned selected ID; selecting the current item leaves it selected |
| `MenuButton` | Fixed caption; menu choice executes its command |
| `SelectionDropdown` | Choice changes selected ID and caption |
| `SplitActionButton` | Main region executes; arrow opens choices; choosing changes the remembered command without executing |
| `Slider` | Bound numeric value; pointer dragging, keyboard steps/page steps, clamping and quantization |
| `ProgressBar` | Read-only fraction, bound fraction, or indeterminate activity |
| `Separator` | Noninteractive horizontal or vertical divider |

Option/action IDs must be nonempty and unique. Segmented controls and split actions require a valid selected ID. Radio groups may start with an empty selection. Indeterminate checkboxes activate to checked. Disabled controls remain in the semantic tree and cannot activate. The split-arrow region defaults to the companion dropdown guide's 40 lp, configurable in `Theme`.

Tab navigates controls; radio/segment groups have a roving tab stop, with arrows/Home/End selecting enabled options. Both split regions have their own tab stop. Tab and Shift-Tab show persistent cyan corner accents on the focused control, with no fade or timeout. Another key action or pointer click ends that visual mode; passive pointer movement does not. Explicit accessibility/programmatic focus can also show the accents. Checkbox and radio accents sit outside the indicator-and-label content with extra clearance. Pointer focus does not leave an outline. Selected toggles and segments use filled surfaces without underlines. Space/Enter activate controls, with key repeat suppressed for commands. Sliders support arrows, Home/End, and Page Up/Down using `SliderOptions`.

Popups paint in a top-level overlay outside scrolling/ancestor clips. They anchor to the trigger, flip and constrain to the viewport, scroll long lists, skip disabled choices, and show current selection. Escape restores trigger/region focus. Outside clicks dismiss and are consumed. Nested submenus and editable combo boxes are deferred.

The macOS adapter publishes SCARS roles, names, states, groups, bounds, and actions through the native backend's optional accessibility provider. It does not translate SCARS into the old UI engine. This includes separate split regions and popup menu items. Automated native checks cover checkbox/radio/slider/dropdown/split actions, group association, frames, static-frame settling, and host/window lifetime. A complete VoiceOver usability audit remains future validation.

## Developer mode and element explorer

In SCARS Lab, press **Command-Option-I** or choose **Developer → Toggle Element Explorer**. The explorer is a separate resizable, minimizable utility panel. It stays above this application's normal windows while the app is active. The same shortcut restores a minimized explorer; when it is already visible, the shortcut closes it. Closing the source Lab window also closes its explorer.

The explorer provides:

- A live, collapsible tree of all mounted views, including flex/block/inline containers and synthetic control regions.
- Filters for all elements, interactive controls, and layout containers.
- Selection highlighting in the source window, with a subtle 480 ms cyan flash when an element is selected or reselected. The flash settles into the persistent highlight, including in box-model and flex debug modes. Picking hover does not flash. Automatic tree reveal follows a different source element being picked.
- A **Pick element** mode that previews the hovered view and consumes the selection click, including on disabled controls. Escape cancels from either window.
- A **Layout** tab with a colour-coded box-model diagram (margin, padding, and resolved content dimensions). The diagram is a schematic, with values in logical pixels. Its overlay shows the actual geometry in the source window.
- Live dimension edits in automatic, logical-pixel, and percentage units, plus independent top/right/bottom/left padding and margin controls. Sliders and steppers apply changes immediately.
- **Position & stacking** controls in Layout for normal/relative/absolute placement, signed edge offsets, z-index, and hiding/showing the selected element. Hidden elements remain in the tree with a marker; inherited hiding identifies the ancestor as the place to restore visibility.
- A **Flex** tab for row/column direction, row wrapping, gap, align-items, and justification. Direct flex children expose grow, shrink, basis, and align-self. Column wrapping is not supported by the current engine.
- **Flex debug overlay** draws the main axis in pink, the cross axis and actual free spaces in cyan, item bounds and order in purple, and margins in orange. It follows the closest flex container when a child is selected. The Flex tab reports each real flex line's item count, resolved gap after justification, and remaining space or overflow after flex sizing. Child rows select the corresponding element.
- A **Details** tab with read-only identity, text/accessible label, measured/final/clipped geometry, computed colours, typography, and bound-state details.

Edits are temporary overrides on the source host; they do not change application code or bound control values. They survive reactive rebuilds for the same mounted element ID. **Reset** restores the selected element's current application-defined values; **Reset all edits** clears every override. Unmounting an element removes its override. Closing the native explorer clears edits, overlays, picking, highlights, and the observer.

Updates are coalesced through the source host's inspection observer; no polling or continuous rendering is required while idle.

To add the window to another macOS application, retain `scars::macos::DeveloperTools(application, sourceHost, theme)` and route an application menu shortcut to `toggle()`. The caller owns menu integration and must keep the application alive until the tools are destroyed. `Inspector(sourceHost)` supplies the native-independent explorer view; `Host::inspect()` supplies a preorder `InspectionNode` snapshot. `set_inspected_element`, `set_inspection_picking`, and `set_inspection_observer` connect selection and live updates. Each source host supports one inspection observer; clear it when detaching your inspector. `Host::reveal` scrolls a mounted view into view without changing keyboard focus.

`Host::set_inspection_overlay(InspectionOverlay::box_model)` and `InspectionOverlay::flex` select debug rendering. `set_layout_override(id, LayoutOverride)` replaces an element's patch; omitted fields use application values. To compose edits, start from `layout_override(id).value_or(LayoutOverride{})`, change the desired fields, and submit the result. Empty patches remove the override. `clear_layout_override(id)` and `clear_layout_overrides()` reset edits. The APIs reject negative/nonfinite dimensions and spacing, unknown targets, generated control regions, and direction edits on non-flex containers. `InspectionNode::content_bounds`, `flex_lines`, `layout_editable`, and `layout_overridden` expose the effective geometry and editing status. These APIs run on the host's owning thread and edits must happen outside view builders.

Normal keyboard focus remains separate from inspection highlighting. Button focus corners now sit outside the filled silhouette so they remain visible against the dark canvas.

## Validation and previews

```sh
ctest --test-dir build/debug -R '^scars\.(engine|features|tabs|numeric|text_controls|overlays|combo|scrolling)$' --output-on-failure
# Native checks require an unrestricted process, Metal, and a WindowServer session:
ctest --test-dir build/debug -R '^scars\.native(_actions|_foreground)?$' --output-on-failure

build/debug/scars/scars_lab.app/Contents/MacOS/scars_lab --render /tmp/scars.png 1180 1280
build/debug/scars/scars_lab.app/Contents/MacOS/scars_lab --render /tmp/scars-narrow.png 480 2100
build/debug/scars/scars_lab.app/Contents/MacOS/scars_lab --render-inspector /tmp/scars-inspector.png 980 760
build/debug/scars/scars_lab.app/Contents/MacOS/scars_lab --render-split /tmp/scars-split.png 760 440
build/debug/scars/scars_lab.app/Contents/MacOS/scars_lab --render-split /tmp/scars-split-light.png 420 480 --light
build/debug/scars/scars_lab.app/Contents/MacOS/scars_lab --render-tabs /tmp/scars-tabs.png 760 440
build/debug/scars/scars_lab.app/Contents/MacOS/scars_lab --render-tabs /tmp/scars-tabs-light.png 420 440 --light
build/debug/scars/scars_lab.app/Contents/MacOS/scars_lab --render-inputs /tmp/scars-inputs.png 760 440
build/debug/scars/scars_lab.app/Contents/MacOS/scars_lab --render-inputs /tmp/scars-inputs-light.png 420 480 --light
build/debug/scars/scars_lab.app/Contents/MacOS/scars_lab --render-text-controls /tmp/scars-text-controls.png 760 600
build/debug/scars/scars_lab.app/Contents/MacOS/scars_lab --render-text-controls /tmp/scars-text-controls-light.png 420 600 --light
build/debug/scars/scars_lab.app/Contents/MacOS/scars_lab --render-overlays /tmp/scars-overlays.png 760 600
build/debug/scars/scars_lab.app/Contents/MacOS/scars_lab --render-overlays /tmp/scars-overlays-light.png 420 600 --light
build/debug/scars/scars_lab.app/Contents/MacOS/scars_lab --render-context-menu /tmp/scars-context-menu.png 760 600
build/debug/scars/scars_lab.app/Contents/MacOS/scars_lab --render-context-menu /tmp/scars-context-menu-light.png 420 600 --light
build/debug/scars/scars_lab.app/Contents/MacOS/scars_lab --render-combo /tmp/scars-combo.png 760 600
build/debug/scars/scars_lab.app/Contents/MacOS/scars_lab --render-combo /tmp/scars-combo-light.png 420 600 --light
build/debug/scars/scars_lab.app/Contents/MacOS/scars_lab --render-scroll /tmp/scars-scroll.png 760 600
build/debug/scars/scars_lab.app/Contents/MacOS/scars_lab --render-scroll /tmp/scars-scroll-light.png 420 600 --light
build/debug/scars/scars_lab.app/Contents/MacOS/scars_lab --developer
build/debug/scars/scars_lab.app/Contents/MacOS/scars_lab --smoke-test
```

The direct native frame-settling test reports skipped when macOS keeps its test window outside the active Space; its interaction-only counterpart checks window behavior, menus, accessibility, picking, and cleanup. `scars.native_foreground` launches the test bundle through LaunchServices and requires actual foreground secure-input ownership, balanced release across focus/hide/unmount/close, real one-shot redraw delivery, frame settling, and the suite's completion marker. It fails rather than silently omitting those branches. Native tests share a CTest resource lock; run configurations sequentially because they use application focus and the system clipboard. A restricted sandbox can hide the Metal device even on this Mac; run native verification outside it. Debug and RelWithDebInfo native protocol, frame, and foreground suites are verified.

The Lab demonstrates the control families in a responsive ship-settings form, including compact/standard/spacious controls, disabled/mixed states, inline badges and buttons, command logging, and resizing. The engine tests cover reactive dependencies, value semantics, flex constraints, block overflow, inline reflow/hit testing, keyboard navigation, keyed focus, popup behavior, sliders, scrolling, Unicode text, and text-engine substitution.

Light preview: add `--light` after explicit width/height, for example `scars_lab --render /tmp/scars-light.png 1180 2400 --light`. The compact [API guide](scars-api.md) covers repeated/conditional views, text editing, lists, bitmaps, layers, and native setup.

## Numeric controls and scheduling

NumericField reuses the SCARS text buffer, SkParagraph geometry, and native input-method bridge. It retains a draft separately from the bound number, validates draft changes, and writes the numeric model only on successful commit. Input-method preview is not parsed as committed input. Enter commits and invokes submission, blur optionally commits, and Escape restores the model after composition has been cancelled. Invalid drafts survive blur with validation feedback. Application-provided setters may normalize values or change constraints; the field reconciles and reports the resulting validation after the setter returns. Observable state is never mutated by the control during construction.

NumericField and Stepper share a private numeric implementation unit in the `scars` module. It uses the existing pinned ICU number-format APIs, explicit locales, strict parsing, full-input consumption, finite decimal checks, and exact signed 64-bit parsing/arithmetic. No new dependency build or OS number formatter is required. Display precision affects presentation, not an untouched model. Model pushes or locale/format changes replace stale draft/history state. Increment actions clamp at bounds; typed commits reject out-of-range input. See [numeric API usage](scars-api.md#numeric-fields-steppers-and-switches).

Stepper repeat uses the host's monotonic clock and a single deadline. A late wake performs one adjustment and schedules the next interval; it does not replay a backlog of missed actions. Hidden, disabled, removed, unfocused, cancelled, or released owners stop repeat. The native surface provides a cancellable one-shot redraw through `set_redraw_delay`; the SCARS adapter updates it from `Host::next_wake()` after painting. Weak surface ownership and a generation token discard cancelled or closed-window wakes. Existing frame animation remains separate, so an idle stepper needs no continuous display loop. Standalone hosts can use `advance_time()` and inject `HostOptions::clock` for deterministic checks.

## Search and secure text input

SearchField and PasswordField use the same TextBuffer, SkParagraph text geometry, input-method bridge, and immutable modifiers as TextField. Search callbacks run outside view construction. Debounce uses the existing host clock and next-wake scheduling, coalesces newer queries, pauses for marked text, and cancels work when its owner stops participating in the active interface. Submission and clear can flush pending work without duplicate callbacks. Enter during marked text accepts composition before any explicit submission.

Masked password layout shapes bullets and maps their positions back to the original grapheme boundaries for hit testing, caret movement, selection, marked ranges, and scrolling. Visual reveal changes painting while preserving the original selection. Accessible values and inspector snapshots remain masked even during reveal; accessible selection and native substring export are suppressed. Copy/cut are blocked in both core commands and native menu validation. Default password editing retains no undo history, while opt-in history is cleared when editing ends. Reveal is also reset on model replacement and inactive/hidden/disabled/read-only state.

The native adapter marks password text-input snapshots secure, while retaining raw text only for privileged input-method UTF-16 conversion. The backend enters secure event input for an editable password in the active key window and balances that ownership when focus, application activity, surface visibility, input-client lifetime, or window lifetime changes. Input-client deactivation clears password reveal/history and pending search in a retained host on unmount. Carbon supplies only secure event-input ownership; paragraph layout and editing remain portable SCARS code. Passwords are application-owned strings, with no encrypted-memory or secure-erasure guarantee. See [constructors and policies](scars-api.md#search-and-password-fields).

## Overlays

Menus, popovers, and tooltips share an ordered overlay stack and window-fitting geometry. Popover and tooltip bodies are owned by their anchors in the existing scene tree, with separate layout scopes that exclude them from anchor measurement, clipping, scrolling, and ordinary scene hit testing. There is one state collector, editor store, node registry, inspector, and input dispatcher. Existing dropdown/menu behavior remains available inside popovers. Arbitrary popover content uses normal SCARS controls and the same native input/accessibility bridge.

Default popovers establish a focus scope, support nested presentation, consume outside clicks, and restore focus after dismissal. Content changes rebuild through the normal state observer; binding cleanup and dismissal callbacks run after construction. Clipped anchors dismiss without revealing or scrolling the anchor back into view. Tooltip deadlines share the monotonic clock and one-shot redraw scheduler used by steppers and search, and disappear when the owning interaction ends. Tooltip help remains in accessibility independently of visual presentation.

Context menus use the same stack for nested menus with stable item IDs, checked/radio bindings, separators, shortcuts, and disabled states. The core handles right-click, Control-click, and Shift-F10; the macOS adapter maps native events and pauses the text-input bridge during menu interaction. Binding reads and builders share normal state collection, and callbacks run after dismissal. Menu hierarchy, checked state, shortcut help, and expansion flow through the existing accessibility provider, with a native menu role added to the window transport.

The Lab includes nested settings/help popovers, delayed help, and station context menus in both themes. The overlay suite exercises timing, input consumption, keyboard scope, marked-text handling, nested menus, arbitrary editing, clipping, resize/fitting, lifecycle cleanup, reordering, builder failures, and idle behavior. Native tests include help semantics, text editing in a popover, all three context-menu invocation paths, checked and nested actions, and Escape/focus restoration. See [tooltip/popover usage](scars-api.md#tooltips-and-popovers) and [context-menu usage](scars-api.md#context-menus).

## Editable selection

ComboBox extends the existing text-field description and TextBuffer with separate query and selected-ID bindings. Its suggestions are an ordinary ListBox mounted in the shared overlay tree, using stable option IDs and the same hit testing, scroll geometry, accessibility provider, and inspection paths. Pointer and keyboard navigation update a logical highlight while keeping input focus in the editor. ListBox virtualizes its suggestion rows without changing the ComboBox API. Logical row metadata supports distant navigation and accessibility reveal while the editor retains input-method focus.

Default filtering uses SkUnicode/ICU uppercase keys in the root locale, including expanding Unicode mappings. Contains/prefix filtering and application-owned ranking/filtering are supported. Loading and empty results use accessible SCARS labels. Strict editing restores unaccepted text on cancellation or loss of interaction; free text permits unmatched values with no accepted option ID. Binding setters, query callbacks, acceptance, and lifecycle restoration run outside view construction. Model replacements establish a new checkpoint.

The native bridge exposes combo-box semantics while retaining the same editable text client. Composition suspends suggestions; Enter/Escape handle composition before acceptance/cancellation. The Lab demonstrates strict and free-text modes and simulated loading in both themes. Portable tests cover filtering, disabled options, commit/cancel/blur policies, callbacks, model changes, row reveal, reordering, geometry, 2x raster painting, and IME behavior. Native tests include accessible editing, suggestion focus/acceptance, combo roles, and Escape restoration. See [API policies and usage](scars-api.md#combo-boxes).

TreeView uses the same virtual row nodes, scroll geometry, selection, focus routing, overlays, and logical-accessibility APIs as ListBox. Its `TreeModel` implementation unit owns immutable snapshot metadata, global identity validation, cached lazy child materialization, and iterative preorder flattening. The host caches an indexed flattened RowModel and direct-child indices only on model/expansion changes. Ordinary scrolling realizes bounded row nodes without walking the tree. Hidden initial trees defer loaders; selection and expansion writes occur only during events. The inspector now renders its element hierarchy with TreeView, retaining ancestor context under filters and revealing picked source nodes. See [TreeView API and lifecycle](scars-api.md#trees).

## Scroll viewports

ScrollView supports vertical, horizontal, and combined axes through the same node layout and clipping path. Host-owned two-coordinate offsets are bounded by cached viewport/document geometry; ListBox uses that same state with a zero horizontal limit. Painting, focus reveal, inspector metadata, and imperative scroll operations use these bounds. Layout remeasures scrolling axes without a finite content limit and constrains the remaining dimensions to their viewport. Empty documents reset offsets; resize and content changes clamp retained offsets.

Wheel routing consumes a gesture from the deepest matching viewport outward, passing remaining movement independently per axis. Text editors and tab strips also participate before scroll ancestors, and overlay scopes prevent input from moving the underlying page. Reveal resolves each inner viewport before recomputing an outer ancestor's movement. The Lab's sector map demonstrates diagonal panning, clipping, and both indicators; native tests exercise horizontal and vertical wheel metadata and geometry. See [API defaults and offset ownership](scars-api.md#scrolling).

## Wizard navigation

`Wizard` owns navigation rules while `WizardNavigation` composes standard SCARS controls into a theme-aware navigation bar. Current-step IDs and form values remain in the application model. Back/review never bypass forward validation; future steps cannot be selected through the step strip. Validation explanations disable Next/Finish, are rechecked on activation, and are exposed as accessible text. Finish validates all steps by default. The control uses ordinary Tab, arrow, Enter, and Space behavior; applications may connect a text field's submit handler to `Wizard::next()`. Completion/cancellation callbacks decide whether to close, reset, or disable the workflow. See the [quick guide](scars-api.md#wizard-navigation) for API usage.

The Lab's **Wizard / guided setup** demo preserves entries when revisiting steps. Render it alone with `scars_lab --render-wizard /tmp/scars-wizard.png 760 480`, adding `--light` to preview the light theme.

TableView extends the same virtual row machinery with fixed headers, shared horizontal geometry, stable cell IDs, bound column widths and sort requests, and a host-owned draft editor using the existing TextField/TextBuffer path. TableModel's implementation unit validates and indexes immutable row/schema metadata; cell values stay application-owned. The workbench demo and interaction regression are present. Native logical columns/cells share stable IDs, lazy indexing, selection batches, sort/header metadata, and the standard validated editor transaction. Protocol checks cover input methods, clipboard/undo, navigation, schema changes, and held accessibility objects after closure. Full native foreground/rendering and scheduler gates now pass in both configurations alongside these protocol assertions. See [TableView API and policies](scars-api.md#tables).


## Alerts and native sheets

`Alert` composes ordinary SCARS views and buttons around an exactly-once transaction. It validates action identity and roles, exposes default/cancel semantics, and becomes disabled after completion. Existing input-method, text-editor, and overlay policies take priority over alert shortcuts. Draft/business-data ownership stays in the application.

`scars::macos::mount` now accepts the existing native `Surface` interface, so the same independent host can render into a Window or Dialog. `AlertPresentation` uses Dialog's asynchronous native sheet transport and owns its host, surface, and completion lifetime. `confirm` supplies the common affirmative/cancel pair. Closing/destroying the sheet or parent cancels pending completion; completion callbacks can present a replacement without reusing a retired transaction. Parent input and accessibility actions are blocked while unrelated windows remain interactive.

The Lab's **Alerts and confirmations** section demonstrates editable content and destructive confirmation. Use `scars_lab --render-alert /tmp/scars-alert.png 480 320`, adding `--light` for the light theme. Portable `scars.alerts` and native `scars.native_alerts` tests exercise transaction, input, modality, accessibility, and lifecycle contracts. Full foreground/rendering gates are tracked separately in the roadmap. See [API usage and ownership](scars-api.md#alerts-and-confirmations).


## Date and time controls

The `scars_datetime.cpp` module implementation owns Gregorian civil arithmetic, locale-aware parsing/formatting, calendar labels, and explicit ICU-zone conversion. DatePicker and TimePicker use private codecs in the same deferred numeric/editor path, preserving the existing text buffer, IME, clipboard, validation, repeat scheduling, and rendering behavior. The calendar is ordinary SCARS content in the shared overlay engine; AppKit supplies native input/window/accessibility transport only.

DateTimePicker distinguishes `LocalDateTime` from a zone-qualified `chrono::sys_seconds` binding. Ambiguous wall times use an explicit policy, gaps are rejected, and the machine's current zone is never inferred. Combined date/time editors stage both components in one host-owned draft, allowing edits across tight joint limits before one valid application update. `field_validation`, `commit_field`, and `cancel_field` accept the group or either component. Model/zone/range changes reset drafts; rejected setters retain feedback. The roadmap records verification and outstanding foreground gates.

ColorWell and ColorPicker share host-owned RGB/HSV/hex drafts, optional alpha, checkerboard previews, and Apply/Cancel. Live preview is explicit and rolls back only while the bound value remains owned by the session. External value changes replace the draft; lifecycle cleanup runs outside construction. Pure conversion/hex helpers use straight encoded sRGB packed ARGB and CSS RGB(A) notation. The [quick guide](scars-api.md#color-controls) documents publication, keyboard, alpha, and lifecycle policies.


The Lab's **Date and time** section demonstrates locales, 12/24-hour display, local and zoned values, and calendars with unavailable Sundays. Render it with `scars_lab --render-datetime /tmp/scars-datetime.png 760 700`, adding `--light` for the light theme. See [current API and limitations](scars-api.md#date-and-time).
