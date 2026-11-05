# SCARS controls completion audit

Verified 2026-11-03 on macOS/arm64 with Apple Clang 21.0.0. All eight roadmap phases meet their available-platform completion criteria. Both full builds are current, and all **29 registered CTest tests pass in each configuration**, without failures or skips. The test groups were run separately to serialize desktop/input ownership; their union covers every registered test.

## Phase evidence

The [public module](../scars/src/scars.cppm) exports the control interfaces. The independent [engine](../scars/src/scars.cpp) owns layout, focus, input, state reconciliation, rendering, and control sessions. The [Lab](../scars/demo/main.cpp) and [API guide](scars-api.md) contain working examples and the ownership, lifecycle, validation, identity, and keyboard contracts.

| Phase | Delivered behavior | Behavioral and integration evidence |
| --- | --- | --- |
| 1 SplitView / TabView | Bound/minimum divider geometry, capture/cancellation, collapse, inspector split; keyed lazy pages, selection fallback, retained/inactive state, scoped navigation | `scars.engine`, `scars.features`, `scars.tabs`; native splitter/tab/editor/focus/inspector assertions; Lab `split_section` / `tabs_section` |
| 2 Inputs | Integer/decimal drafts, portable numeric formatting/ranges, exact stepping/repeat; Switch; search clear/debounce/submit/Command-F; masked/reveal/undo/clipboard/IME/password ownership | `scars.numeric`, `scars.text_controls`; native secure-input acquisition/release, hide/unmount/window lifetime, and actual scheduled search delivery; Lab numeric/text sections |
| 3 Overlays / editable selection | Nested/fitted/scoped Tooltip, Popover, ContextMenu; checked/radio/submenu actions; distinct ComboBox query/ID bindings, filtering, loading/no-results, acceptance/cancellation | `scars.overlays`, `scars.combo`; native context/keyboard/IME/AX and focus-restoration assertions; Lab overlay/combo sections |
| 4 Scrolling / virtualization | Two-axis bounded offsets, nested residual routing, reveal, indicators; stable indexed lazy rows, bounded realization/retention, active-editor pin, offscreen logical AX | `scars.scrolling`, `scars.virtualization`, `scars.combo`; 100,000-row native indexed access/selection/reveal and retained composition; inspector viewport/count state; Lab sector map / 100,000 observations |
| 5 Tree / table | Keyed eager/lazy/loading trees, expansion/typeahead/navigation; inspector TreeView; fixed aligned table headers, virtual cells, selection/resize/sort requests, transactional/conflict-aware edits and traversal | `scars.tree`, `scars.table`, `scars.workbench`; native indexed outline/column/cell, selection, editing/IME, schema/lifetime assertions; Lab integrated workbench; inspector table/tree state |
| 6 Alerts / confirmations | Composable/reactive content, action IDs/roles, exactly-once completion; native sheets, parent isolation, focus/close/destruction/replacement handling | `scars.alerts`, `scars.native_alerts`; [native adapter](../scars/src/scars.macos.cppm) `AlertPresentation` / `confirm`; Lab review/reset sheets; inspector roles/status |
| 7 Date / time | Gregorian locale fields/calendar, disabled dates/ranges, component stepping, 12/24-hour display; shared local/zoned drafts, atomic joint validation/publication, explicit gap/overlap resolution | `scars.datetime`, `scars.native_datetime`; [portable codecs/conversion](../scars/src/scars_datetime.cpp); overnight-range and lifecycle/rejection/normalization regressions; native calendar/IME/AX; Lab local/zoned/overnight forms |
| 8 Color | Compact well and inline RGB/HSV/hex editor, optional preserved alpha, transparency preview, keyboard; default Apply/Cancel and explicit live preview with external-update ownership | `scars.colors`, `scars.native_colors`; [portable conversion](../scars/src/scars_color.cpp); draft/rollback/normalization/reentrancy/construction-purity cases, native IME/AX/closure; Lab inline/well/preview examples; inspector color metadata |

## Shared requirements

| Requirement | Current evidence / result |
| --- | --- |
| Independent engine; previous UI remains separate | `scars` implementation units declare `module scars`; adapter declares `export module scars.macos`. No legacy UI host is mounted. Native assertions check independent hosts; all original UI/window tests pass. Only the existing low-level native Skia/Metal backend is shared. |
| Named C++23 modules and chained immutable descriptions | Public `scars` / `scars.macos` modules, implementation units, private codecs/models, and explicit Apple BMI graph in [CMake](../scars/CMakeLists.txt); complete module/consumer builds pass in both configurations. |
| Flex/block/inline layout, layers/position/visibility | Public containers/modifiers and independent engine paths remain present. Engine/feature regressions verify layout, z-order, positioning, visibility, hit testing, and inspection. |
| Portable text layout/editing | [SkParagraph engine](../scars/src/scars_text_skparagraph.cpp) and [Unicode editor](../scars/src/scars_text_edit.cpp) use SkParagraph/HarfBuzz/SkUnicode/ICU. Native font discovery/window/input transport does not supply paragraph layout. Public and native tests cover Unicode selection, candidate geometry, composition, clipboard, and editing. |
| Both LCARS themes, titlebar toggle, visible focus corners | Engine/feature/control raster checks exercise dark/light and 1×/2× logical rendering. Native titlebar/focus assertions pass. Dark and 360-pixel light workbench/alert/date/color previews were regenerated and visually inspected at the completion checkpoint; earlier split/tab/input/overlay/scroll/tree previews and raster checks cover the other sections. |
| Inspector integration | [Inspector](../scars/src/scars_inspector.cpp) uses SplitView/TreeView, property editors and layout/flex overlays, and metadata for virtual lists/tables, alerts, calendars, joint drafts, and colors. Full native picking/minimize/restore/close-lifetime assertions pass. |
| Scheduling/capture/overlay lifetimes | Deterministic portable clocks and mutation/callback cases cover cancellation and construction purity. Full native suites prove delayed search is delivered without continuous animation, static rendering settles, and secure input is acquired/released across focus/hide/unmount/close. |
| macOS-only runtime scope and compiler coverage | Apple Clang 21.0.0 macOS builds are verified. Upstream Clang 23, GCC 16, and MSVC are compatibility targets, **not validated builds**. The roadmap requires reporting this boundary and validating them when toolchains/dependencies become available. Linux/Windows native backends remain outside this roadmap. |
| Supplied specs versus explicit requests | The roadmap retains the supplied PDFs as reference specifications and explicitly gives the user's decisions precedence. No document-embedded instruction expands authorization. |

## Test coverage and results

Both `cmake --build --preset debug` and `cmake --build --preset relwithdebinfo` succeed and report no remaining work. The following disjoint groups pass in **both** configurations:

| Group | Result per configuration |
| --- | --- |
| Portable SCARS, previous UI behavior/performance, and C++ consumer | 18/18 CTest tests; includes 1,220 SCARS checks |
| SCARS native protocol/action/foreground | 6/6 CTest tests |
| Previous native renderer/window/UI | 5/5 CTest tests |
| Union of registered tests | **29/29 passed, zero failures/skips** |

SCARS check counts: engine 185, features 153, tabs 59, numeric 84, text controls 70, overlays 131, combo 56, scrolling 47, virtualization 60, tree 60, table 81, workbench 28, alerts 37, date/time 107, colors 62. Color conversion includes 4,096 RGB/HSV/hex samples within its round-trip assertion. Large-data suites assert viewport-bounded work and retention; recorded performance measurements are not fragile frame-rate gates.

Commands used, substituting each of `debug` and `relwithdebinfo` for `CONFIG`:

```sh
ctest --test-dir build/CONFIG -R '^(scars\.(engine|features|tabs|numeric|text_controls|overlays|combo|scrolling|virtualization|tree|table|workbench|alerts|datetime|colors)|skia_window\.(ui|ui_performance|cpp_consumer))$' --output-on-failure
ctest --test-dir build/CONFIG -R '^scars\.native(_actions|_foreground|_alerts|_datetime|_colors)?$' --output-on-failure
ctest --test-dir build/CONFIG -R '^skia_window\.(integration|resize|ui_desktop|ui_gpu|ui_native_actions)$' --output-on-failure
```

Both LaunchServices foreground logs include the actual secure-input, one-shot search, and final post-assertion completion markers. The [native verification guide](scars-native-verification.md) records those markers and the resolved locked-desktop diagnosis. Its standalone probe now reports one active display, application/key-window ownership, and real callbacks. No assertions were weakened or skipped to close the earlier verification gap.

Visual reproduction uses the Lab's `--render-*` commands. For example:

```sh
build/debug/scars/scars_lab.app/Contents/MacOS/scars_lab --render-colors /tmp/scars-colors.png 760 1000
build/debug/scars/scars_lab.app/Contents/MacOS/scars_lab --render-colors /tmp/scars-colors-light.png 360 1200 --light
```

The completion review also regenerated `--render-workbench`, `--render-alert`, and `--render-datetime` in both palettes. Together with the per-control tests and documented earlier previews, this covers constrained layouts and both themes without substituting raster output for required native verification.
