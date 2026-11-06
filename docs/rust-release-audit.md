# Rust release audit

Reviewed 2026-11-06. The C++ reference is unchanged from `82583e7`; implementation
and example baseline is `c10bab6`, with the build/AX corrections recorded below.
The [44-item checklist](rust-completion-checklist.md) is **39 checked, 5 open**.
This is an evidence reconciliation, not final sign-off.

## Requirement-by-requirement evidence

`Checked` below records an implemented checklist item with its focused evidence.
Native protocol checkpoints are documented in the checklist; ordinary headless
harness skips are not native passes. Actual speech, IME placement and final
foreground runs remain the explicit open gates below.

Core test paths are relative to `rust/crates/scars-ui/tests/`; renderer/native test
paths are relative to `rust/crates/scars-window/tests/`. Examples run from `rust/` with
`cargo run --example NAME --locked --offline`.

| Item | Status | Concrete evidence / runnable example |
| --- | --- | --- |
| M1.1 Inventory | Checked | [Inventory](rust-inventory.md); `check_rust_inventory.py` maps 162 types and 47 view/annotation names |
| M1.2 Styles | Checked | Core `styles`, `reflow`; renderer `components_render`, `rich_text`; `m1_layout` |
| M1.3 Layout | Checked | Core `reflow`, `styles`; renderer `rich_text`, `scroll`; narrow/wide Lab matrix |
| M1.4 Scrolling | Checked | Core and renderer `scroll`; `m1_scroll` |
| M1.5 Input lifetime | Checked | Core `scroll`, `editors`, `split`; renderer `scroll`; native transport checkpoints in checklist |
| M2.1 Text buffer | Checked | Core `text_buffer`, `text`; portable `text_buffer` example |
| M2.2 Navigation | Checked | Core `editors`; renderer `paragraph`, `scroll`; `m2_editor` |
| M2.3 Selection/history | Checked | Core `text_buffer`, `editors`, `fields`; `m2_editor` |
| M2.4 Search | Checked | Core `fields`; cancellation/resource cycles; `m2_fields` |
| M2.5 Passwords | Checked | Core `fields`; sensitive paragraph-cache test; native `secure_input` checkpoint; `m2_fields` |
| M2.6 Numeric/stepper | Checked | Core `numeric`, `localized_values`; `m2_numbers` |
| M3.1 Split panes | Checked | Core `split`; native split semantic checkpoint; `m3_split` |
| M3.2 Menus | Checked | Core `menu`, `presentation`; native menu semantics checkpoint; `m3_menu` |
| M3.3 Overlays | Checked | Core `overlay`; native presentation checkpoint; `m3_overlay` |
| M3.4 Tabs | Checked | Core `tabs`; native tab selection/identity checkpoint; `m3_tabs`; speech recheck stays M5.5 |
| M3.5 Wizard | Checked | Core `wizard`; `m3_wizard` |
| M3.6 Alerts/sheets | Checked | Core `alert`; native `sheets` checkpoint; `m3_alerts --sheet`; final foreground rerun stays M8.1 |
| M4.1 List interaction | Checked | Core `list`, `collections`; `m4_list` |
| M4.2 Virtual edit lifetime | Checked | Core `list_edit`, `collection_versions`; `m4_list_edit` |
| M4.3 Tree model | Checked | Core `tree`; `m4_tree` |
| M4.4 Tree interaction | Checked | Core `tree_input`; `m4_tree` |
| M4.5 Combo box | Checked | Core `combo`; native combo checkpoint; `m4_combo` |
| M4.6 Table geometry | Checked | Core `table`; `m4_table` |
| M4.7 Table editing | Checked | Core `table_edit`; `m4_table_edit` |
| M5.1 Tree semantics | Checked | Core `tree_semantics`; native `tree_accessibility` checkpoint, including 100,000 logical rows |
| M5.2 Table semantics | Checked | Core `table_semantics`; native `table_accessibility` checkpoint and bounded lazy arrays |
| M5.3 Control semantics | Checked | Core `control_semantics`; native `control_accessibility` checkpoint; `m5_semantics` |
| M5.4 Native lifecycle/posts | Checked | Native `accessibility_lifecycle`, `accessibility_notifications`, `accessibility_focus` checkpoints |
| M5.5 Interactive audit | **Open** | Actual VoiceOver speech and composing IME placement at actual backing scales 1 and 2; tab speech recheck pending |
| M6.1 Format policy | Checked | [Grammar specification](rust-editable-formats.md); core `editable_formats`; `m6_formats` |
| M6.2 Localization | Checked | Core `localized_values`; native localized-value checkpoint; `m6_locales` |
| M6.3 Date picker | Checked | Core `date_picker`; native date checkpoint; `m6_date` |
| M6.4 Time/datetime/zone | Checked | Core `time_picker`, `datetime_picker`, `zoned_picker`; renderer/native checkpoints; corresponding M6 examples |
| M6.5 Color | Checked | Core `color_picker`; renderer `color_raster`; `m6_color` |
| M6.6 Typed draft lifecycle | Checked | Core `draft_lifecycle`, `draft_interaction`, `draft_prefixes` and all typed-control suites; `m6_drafts` |
| M7.1 Mounted inspector | Checked | Core `inspector`; renderer `inspector_raster`; native inspector checkpoint; `m7_inspector` |
| M7.2 Property overrides | Checked | Core `inspector`; renderer `inspector_raster`; renamed-consumer compilation |
| M7.3 Full Lab/ergonomics | Checked | `lab` covers 28 samples on 7 retained pages; independent component state and source-level consumer checks |
| M8.1 Visible native rendering | **Open** | Awake/foreground Metal, lifecycle, full-screen and native suite rerun |
| M8.2 Visual matrix | Checked | Six Lab tests and 644 paired CPU scenes; both themes/scales/sizes, states, long/RTL/emoji text |
| M8.3 Repeatable checks | Checked | `check_rust.py`, validator tests, standalone consumer, Linux core and checked CI workflow |
| M8.4 Resources | **Open** | [Measured CPU/hidden evidence](rust-resources.md) passes; prepared visible idle-frame cases still need the desktop |
| M8.5 Examples | **Open** | [37 headless examples / 156 PNGs](rust-example-regressions.md) and seven Todo tests pass; foreground examples remain unrun |
| M8.6 Final sign-off | **Open** | This audit, minimum compiler and documentation work are prepared; the four preceding open gates still prevent sign-off |

## Option-family reconciliation

Options were checked against the public C++ declarations and the Rust source,
in addition to the name-coverage script. Rust methods, models and owned options
replace C++ constructor-option bundles where that keeps application code concise.

| Reference family / fields | Rust equivalent and focused evidence |
| --- | --- |
| Theme palette, density/geometry, font path | `Theme`, inherited `Style`/`Density`/accent, per-control dimensions/radius and `Renderer::with_font(bytes)`; `styles`, `components_render`, Lab. The Rust palette is smaller; local style supplies component-specific colors/geometry |
| Text layout width/wrap/overflow/max lines/alignment and inline boxes | `TextStyle`, `Span`, `Measure::text/paragraph`, style modifiers; `reflow`, `rich_text`, `paragraph` |
| Scroll axes/show bars | `scroll_axes`, `scrollbars`; core/renderer `scroll` |
| Slider min/max/step/page step | Slider constructor plus checked `page_step`; `controls`, Lab |
| Split axis/minima/divider/key step/collapsed binding | `SplitOptions`, `Split::collapsed`; `split` |
| Text multiline/readonly/grapheme limit/submit | `text_editor`, `TextOptions`, `read_only`, `max_length`, `on_submit`; `text_buffer`, `editors` |
| Search debounce/clear/readonly/limit/query/submit/cancel | Field modifiers and callbacks; `fields` |
| Password reveal/undo/readonly/limit/submit | `allow_reveal`, `undo_enabled`, normal text options/callbacks; `fields`, sensitive cache test, native ownership checkpoint |
| Numeric bounds/step/locale/grouping/precision/blur/readonly/validation/submit/repeat | `Numeric<T>`, `EditLocale`, `significant_digits`, callbacks and `Repeat`; `numeric`, `localized_values`. Grouping is part of the checked locale profile; `ASCII` is ungrouped and custom `Draft` codecs supply other explicit grammars |
| Combo mode/filter/loading/readonly/highlight/limit/geometry/messages/callbacks | `ComboBoxOptions`, `on_query/accept/cancel`; `combo` |
| Menu action kind/check/radio/children/enabled/shortcut/invoke | `MenuAction`, stable paths, bindings and callbacks; `menu`, native control checkpoint |
| Tooltip delay/width/gap/margin/placement | `TooltipOptions`; `overlay` |
| Popover placement/focus/width/height/gap/margin/outside/Escape/dismiss callback | `OverlayOptions`, `Popup::on_dismiss`, typed dismissal reason; `overlay`. Bodies use ordinary styled containers |
| Wizard stable steps/validation/back/review/final check/labels/status/callbacks | `Wizard::bound`, `WizardOptions`, `WizardStep`, callbacks; `wizard` |
| List row height/visible rows/overscan/activation and custom rows | `ListModel`, `ListOptions`, `view_with`, `on_activate`; `list`, `list_edit`, `collections` |
| Tree row geometry/indentation/activation/expansion/loading/labels | `TreeOptions`, `TreeNode`, `TreeExpansion`, guarded loaders/callbacks; `tree`, `tree_input`, `tree_semantics` |
| Table row/header geometry/visible rows/overscan/selection/widths/sort and column content/editing | `TableOptions`, model-owned widths/sort with setters/callbacks, `TableColumn` configuration and `CellEdit`; `table`, `table_edit`, `table_semantics`. Shared models own this state rather than duplicating external bindings |
| Color RGB/HSV/alpha/live preview/readonly | `ColorPicker` methods and shared draft; `color_picker`, `color_raster` |
| Date limits/locale/readonly/blur/disabled dates/validation/submit | `DatePicker` methods, checked `EditLocale`; `date_picker`, `localized_values` |
| Time limits/locale/hour cycle/seconds/component steps/readonly/blur/callbacks | `TimePicker` methods and shared clock policy; `time_picker` |
| Civil datetime nested calendar/clock policy/joint bounds/validation | `DateTimePicker` methods and one shared draft; `datetime_picker` |
| Zoned time zone/fold policy/instant bounds/display policy | `ZonedPicker`, `repeated_time`, civil/instant limits and calendar/clock methods; `zoned_picker` |

There is no field-for-field ABI or visual-default compatibility claim. Editing
locale profiles and unsupported grammars are explicit in the format specification.
State, descriptions and option/model behavior remain in Rust; Skia's engine is
source-built C++ and AppKit is reached through Rust `objc2` bindings.

## Minimum compiler and build correction

The declared minimum is Rust 1.88.0. The `msrv` suite selects that exact child
toolchain and records metadata from the same environment. Actual local evidence:

- 277 core/macro tests passed with zero failures/ignored cases.
- `cargo check --workspace --all-targets` passed in dev and release profiles on
  macOS, including native bindings and examples. Both Skia build outputs explicitly
  recorded `STARTING A FULL BUILD`.
- Linux `scars-ui` compilation and the separate renamed-dependency consumer passed.
- The portable `foundation` example ran under Rust 1.88, demonstrating keyed
  state changes, ICU4X formatting and grapheme boundaries.
- Ten validator tests and the updated workflow's Actionlint check passed locally.
  Remote CI execution is still unclaimed.

Disabling debug information initially exposed an older compiler-toolchain issue:
`zerofrom_derive` passed code-signature verification but dyld rejected it with
`mis-aligned LINKEDIT string pool`. Keeping the same compiler/debug setting and
setting `strip=none` made the check pass. The shared Cargo configuration now
disables stripping in dev/test/release profiles, including for copied consumer setup.
This matches the upstream [Rust/macOS stripping issue](https://github.com/rust-lang/rust/issues/157750).
Debug information can still be disabled; this setting preserves the symbol layout.

Reproduce from the repository root:

```sh
rustup toolchain install 1.88.0 --profile minimal --target x86_64-unknown-linux-gnu
python3 scripts/check_rust.py --suite msrv --offline
```

## Native expansion protocol correction

The existing tab getter filter did not suppress the inherited expansion setter.
A hidden native test reproduced that tabs still advertised the unsupported setter.
Generic component elements now reject `setAccessibilityExpanded:`; menus retain
their expansion getter and guarded press/show actions. Logical tree rows have
their separate implemented setter. The hidden tab/menu/focus regression passes.

This does **not** establish corrected VoiceOver speech. Rebuild/restart
`m5_semantics` before the next speech recheck. The reported “selected collapsed
tab 1 of 2” remains an open M5.5 observation until tested with actual VoiceOver.

## Final gates and boundaries

The latest desktop probe reports zero active displays and the login window.
Native suites and native-only examples have not been rerun. Actual 2× display
backing scale, composing input-method candidate placement and complete speech
sessions remain unavailable evidence. The user has offered to help with the
interactive audit; no passing result is inferred from that intent.

After those sessions, rerun native validation, resolve any failures, reconcile
each affected inventory row and record final sign-off in M8.6. The goal remains
unachieved while these gates are open.

The agreed optional extensions remain separate: native Windows/Linux backends,
wide-gamut color management, variable-height collection virtualization and
features exclusive to the older C++ UI engine. They do not replace any of the
five in-scope open checklist items.
