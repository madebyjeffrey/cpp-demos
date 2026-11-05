# Migrating SCARS components to Rust

Assessment date: 2026-11-04. This is a source-based feasibility report, not a component implementation or a claim of Rust feature parity.

Product constraint clarified after the initial assessment: the C++ components have not been used by an application, so exact API, visual, and behavioral compatibility is not required. They are a reference implementation and a source of useful test scenarios. Existing user decisions about the Rust window API remain requirements.

Confirmed product direction:

- Deliver every current SCARS component eventually, in stages; early milestones do not narrow the eventual component inventory.
- Ship macOS first, with a portable component core. Windows and Linux native adapters are future work.
- Make Rust-like nested declarative macros with reactive state the primary application-facing API: `view! { column { button("Save") } }`.
- Recommend delivery order from shared dependencies; there is no first-application priority to optimize around.
- Assess localization/input trade-offs before choosing the initial scope. The input contract remains a product decision.

These answers are sufficient for this feasibility report. Detailed grammar and control contracts can be settled during implementation design. The localization recommendation below is a proposal for sequencing, not a recorded user decision.

## Recommendation

**A Rust component system is feasible on the current Rust window and Skia foundation.** Treat this as a Rust design informed by SCARS, with freedom to simplify its APIs, layout conventions, editing rules, and callback execution. The separation between descriptions, mounted state, rendering, and native integration remains useful. It does not require keeping C++ SCARS components behind an FFI boundary. Skia and the native code built by `skia-bindings` remain accepted implementation dependencies.

The main work is the component engine itself: identity, reconciliation, layout, focus and input routing, persistent editing sessions, overlays, virtualization, accessibility, and lifecycle behavior. Drawing buttons is a small part of that work. ICU4X is a suitable starting point for Unicode and localization services; there is no requirement to reproduce ICU4C's exact parsing behavior. The remaining design work includes defining editable formats, predictable callback execution, and native accessibility for virtual collections. Skia's Rust paragraph bindings already cover the drawing and geometry operations used by the C++ text engine.

Start with one complete vertical slice—reactive state, a small layout, label, button, and editable field, including accessibility and unmounting—then expand by dependency order. Use selected C++ examples to identify useful capabilities and edge cases, while defining the Rust behavior explicitly. The resulting Rust application should depend on Rust components and `scars-window`, without linking the C++ SCARS engine.

## Scope and evidence

The primary reference is the modern [`scars/`](../scars/) library in this checkout. Its public API and engine match the corresponding core files in the sibling standalone `/Users/jdrake/CLionProjects/scars` checkout when compared for this assessment. This report uses paths in this repository so there is one reviewable reference.

Modern SCARS is distinct from the older [`skia_window::ui`](../include/skia_window/ui.hpp) engine. Modern SCARS implements flex/block/inline layout and the current control suite; Grid, a custom Canvas node, and general animation/transition APIs from the older engine are additional scope if desired. They should not silently become prerequisites for rewriting the current SCARS components. The modern engine's boundary is documented in [`docs/scars.md`](scars.md), and its public descriptions are in [`scars.cppm`](../scars/src/scars.cppm#L535).

The existing [completion audit](scars-controls-verification.md) records 29 registered CTest tests passing in both C++ configurations on 2026-11-03, including 1,220 portable SCARS checks. Those are historical reference results, not tests rerun for this report. The Rust window crate has its own build and verification history; none of that establishes component parity because its component layer is absent.

## What exists in Rust, and what remains

[`rust/`](../rust/) is currently `scars-window`: Rust-defined AppKit view/delegate classes, native windows and dialogs, application/window lifecycle, menus and commands, clipboard, input events, redraw scheduling, appearance, and Skia rendering into Metal. Its public [`TextInputClient`](../rust/src/types.rs#L408) transports UTF-16 ranges and UTF-8 text to the renderer. It deliberately delegates editing storage and layout to that renderer. The macOS [`ContentView`](../rust/src/macos/view.rs#L1) implements native text-input transport; it does not implement an editor or a semantic widget tree.

The crate already supplies the relevant native foundation. Its [manifest](../rust/Cargo.toml) selects `skia-safe` 0.157.0 with Metal/Ganesh, text layout, embedded ICU data, SVG, JPEG, and WebP support. Its [Cargo configuration](../rust/.cargo/config.toml) forces source builds. These dependencies are independent of the C++ project's [Skia m154 pin](../dependencies/skia.json); Rust uses its bindings' matching m157 sources. Differences in rasterization and text layout are acceptable; establish new Rust baselines rather than requiring identical C++ output. The [Rust README](../rust/README.md) explains that boundary.

| Area | Existing C++ behavior | Rust migration assessment |
| --- | --- | --- |
| Descriptions and state | Immutable styled `View`; `State<T>`/`Binding<T>`; tracked reads; keyed rebuilding | Straightforward Rust API, with explicit ownership and notification rules |
| Layout and scene | Flex row/column, block, inline placeholders, layers, sizing/positioning, clipping, visibility | Define Rust geometry contracts and select suitable algorithms; Skia is the renderer, not this layout engine |
| Basic controls | Labels, buttons, checkbox/radio/switch, dropdown/split action, slider, progress, image, separators, grouped sections | Mostly tractable after identity, input, focus, layout, and semantics exist |
| Editing and value controls | Text/editor/search/password, integer/decimal fields and steppers, color/date/time controls | Substantial shared editor/session work; localization is a separate dependency decision |
| Navigation and overlays | Tabs, split panes, wizard, tooltips/popovers/context menus, combo box, alerts | Design capture, focus restoration, lazy state retention, timing, modal isolation, and completion rules |
| Large collections | Scroll routing/reveal, virtual lists, lazy trees, tables, transactional cell editing | Higher risk; retain stable logical identities independently of realized rows |
| Native integration | Host mount, IME, clipboard, accessibility, dialogs/sheets, scheduling | Rust window foundation exists; host adapter and widget accessibility provider must be written |
| Developer tooling | Inspector, picking, layout/semantic metadata, separate tools window | Reuse Rust components to rebuild after inspection data and core controls stabilize |

The authoritative API inventory is [`scars.cppm`](../scars/src/scars.cppm#L637); [`scars.cpp`](../scars/src/scars.cpp) contains the central engine. Supporting units isolate [row models](../scars/src/scars_rows.cpp), [trees](../scars/src/scars_tree.cpp), [tables](../scars/src/scars_table.cpp), [navigation](../scars/src/scars_navigation.cpp), and value/text/image services.

## Proposed Rust architecture

### Descriptions should remain easy to compose

C++ already erases widget types into an immutable `View` backed by a shared description. The Rust application API will use a declarative macro with reactive state. A concrete `View`, an internal enum for node kinds, typed property records, and shared callbacks can provide its underlying runtime. The macro can expand into ordinary construction code, with functions and builders remaining useful for reusable components and integration. Its syntax need not determine the host's ownership model. These are proposed design choices, not existing Rust APIs. See [`View`](../scars/src/scars.cppm#L535) and the current [`Description`](../scars/src/scars.cpp#L101).

Treat macro ergonomics as part of the first working slice. Define expressions in properties, event handlers, state bindings, conditional children, keyed repetition, and composition through user-defined components. Keys must remain explicit where children can reorder; declarative syntax does not establish stable mounted identity by itself. Specify how event closures capture state so applications do not have to manage host reference counting or weak pointers. Keep dependency tracking in the reactive runtime, with clear subscription cleanup, rather than making the macro guess dependencies from expression text.

Use Rust-like nested calls and blocks, with Rust expressions for property values and conditions. Evaluate small settings-form and dynamic-list examples before expanding the grammar across the full inventory. A proposed form is:

```rust
// API sketch only: these component/macro APIs do not exist yet.
// `model` retains reactive state; `save_settings` is an application handler.
view! {
    column(gap: 12, padding: 24) {
        label("Settings");
        text_field("Name", bind: model.name, key: "name");
        checkbox("Enabled", bind: model.enabled);

        if model.enabled.get() {
            label(format!("Hello, {}", model.name.get()));
        }

        button("Save", on_click: save_settings);
    }
}
```

In this proposal, `bind:` creates or accepts an owned binding handle without moving the model's value, and state reads during construction register dependencies. Event handlers execute during dispatch, outside construction. Ordinary Rust functions can return reusable component descriptions. Repetition needs an explicit stable key for each logical item; source position or list index alone is insufficient for reorderable data. Capture ergonomics should be tested with a real handler that reads and updates state before declaring the API settled.

For a full component grammar, I recommend a small function-like procedural macro backed by ordinary typed runtime constructors. This is a design recommendation: a parser can report malformed child blocks or properties near their source, while generated typed calls let Rust check binding and handler types. It adds a separate `proc-macro` crate and maintenance for expansion paths, spans, and capture rules. A small `macro_rules!` grammar is also feasible, but rich property/child syntax and useful errors should be demonstrated before selecting it. Rust documents [pattern-based expansion](https://doc.rust-lang.org/reference/macros-by-example.html) and [procedural token streams, spans and crate requirements](https://doc.rust-lang.org/reference/procedural-macros.html). Neither mechanism supplies reactivity automatically.

The first macro checks should cover a valid form, reusable component, conditional child, keyed list, multiple handlers sharing state, unknown property, wrong binding type, and an error in a nested Rust expression. Error messages should point to application code. The macro should expand without introducing AppKit/Metal types or requiring applications to manage host reference counting.

Keep application state separate from mounted control sessions. A keyed text field's selection, composition, undo history, or invalid numeric draft is not the same thing as its committed binding. Define the consequences of tab inactivity, hiding, removal, and model replacement for Rust. The C++ rules are useful examples, but can be simplified where that improves the application API.

The existing `State<T>` copies values on reads and skips equal writes where comparison is available. Rust should make its own bounds deliberate: a clone-based `get` can require `Clone`, while equality-aware writes require `PartialEq` or a supplied comparator. Suppressing unchanged writes is a useful optimization; an explicit always-notifying operation can support other values. These do not need to reproduce the C++ template's exact capabilities. `Rc`-owned state fits a single-owner-thread design; an `Arc<Mutex<_>>` tree is unnecessary. Define read tracking, dependency cleanup, and whether writes during view construction are rejected. The reference is [`State`/`Binding`](../scars/src/scars.cppm#L118).

### The host should own identity and mutation

The current [`Host::Impl`](../scars/src/scars.cpp#L1240) owns mounted nodes plus maps of editor, list, tree, table, tab, date, and color state. Raw node pointers support lookups and traversal in C++. Use owned nodes and stable handles/IDs in Rust, with a generation or equivalent stale-handle rule. Do not reproduce this as a graph of raw pointers, or default to `Rc<RefCell<Node>>` for every edge.

Callbacks can request binding changes, hide their owner, or remove the current row. Choose an explicit Rust dispatch contract: a command/effect queue that applies tree changes after the current event can simplify borrowing, and arbitrary synchronous host reentry does not have to be supported. If callbacks run synchronously, release host borrows first and resolve IDs/generations before continuing. If setters may reject or normalize input, expose the accepted result or read it back; the current [wizard navigation](../scars/src/scars_navigation.cpp#L59) illustrates why that matters. Snapshot observers before notifying them. Use RAII guards for build/subscription tracking and stage fallible rebuilds so a failure leaves a usable tree. The current [rebuild path](../scars/src/scars.cpp#L3886) and [row realization](../scars/src/scars.cpp#L3205) supply edge cases, not mandatory execution semantics.

Keep a platform-independent host with explicit logical geometry, time, clipboard services, and semantic queries. The existing [`Host` interface](../scars/src/scars.cppm#L1201) is a useful seam for deterministic tests and raster rendering. Specify padding, margins, unbounded constraints, and hidden versus removed allocation in the Rust API. Either adapt the existing algorithms or evaluate a Rust layout library against those chosen rules; matching SCARS's geometry is not a selection requirement. A narrow macOS mount adapter should connect the host to `scars-window` services without spreading AppKit through widgets. Today the Rust Skia dependency and re-export are macOS-gated: the confirmed portable-core requirement calls for moving that shared drawing boundary outside the macOS-only native implementation. Portable state, layout, editing and semantics should not depend on AppKit or Metal. Modules can establish the seams before a separate component crate is useful.

## Text and graphics: strong binding coverage, explicit index rules

The C++ [SkParagraph adapter](../scars/src/scars_text_skparagraph.cpp) uses styled spans, font fallback, embedded placeholders, wrapping/ellipsis/alignment, line metrics, glyph visits, hit testing, selection boxes, caret geometry, and paragraph painting. The locally installed **0.157.0 source** exposes corresponding operations:

- `ParagraphBuilder::new`, style push/pop, `add_text`, and `add_placeholder`;
- `FontCollection`/`TypefaceFontProvider`, default and asset fonts, and fallback;
- `Paragraph::layout`, `paint`, line metrics, range/placeholder rectangles, coordinate hit testing, glyph visiting with UTF-8 starts, and `get_glyph_info_at_utf16_offset`;
- text/paragraph style controls for height override, half leading, CSS letter spacing, and rounding behavior.

This makes a direct Rust text-layout adapter plausible without adding a project-specific C++ wrapper. Paragraph methods have differing `&self`/`&mut self` signatures, so the layout object's ownership must accommodate native lazy mutation. Do not design an immutable shared result by assuming every query is immutable internally. These observations come from the installed crate, whose `.cargo_vcs_info.json` identifies [upstream revision `30669978…`](https://github.com/rust-skia/rust-skia/tree/30669978c3f39a73e8a7ac74eead8d414eb1891d/skia-safe); version-specific web documentation was unavailable to the browser during this assessment. Upstream's [paragraph source](https://github.com/rust-skia/rust-skia/blob/master/skia-safe/src/modules/paragraph/paragraph.rs) is a navigational reference, not a substitute for checking the locked version.

Indexing needs one explicit conversion layer. C++ public editor ranges are UTF-8 byte offsets; AppKit transport uses UTF-16 code units; grapheme boundaries and glyph clusters are different again. Rust `String` validity removes much manual UTF-8 validation but does not make byte offsets safe to slice at arbitrary positions. Retain checked conversions, normalize surrogate/interior-byte positions consistently, and test emoji sequences, combining marks, RTL text, explicit newlines, placeholders, selections, and end-of-document carets. The reference conversions/editor are in [`scars_text_edit.cpp`](../scars/src/scars_text_edit.cpp#L66), with native conversion in [`scars.macos.cppm`](../scars/src/scars.macos.cppm#L585).

Images are a lower-risk service: C++ [`Bitmap`](../scars/src/scars_image.cpp) loads encoded data or copied RGBA pixels and paints fit/fill/stretch/center with clipping. The locked Rust bindings expose deferred encoded images, raster image creation, canvas image drawing, and SVG `Dom` parsing/rendering. Preserve alpha/color-space/stride checks and intrinsic-size semantics. SVG is available as a future extension; the current SCARS `Bitmap` implementation itself is not a general SVG component. Test actual supported encoded assets rather than inferring codec parity solely from feature names.

## Unicode and localization: start with ICU4X

Embedded ICU data enables Skia paragraph shaping; it is **not** a public Rust replacement for every ICU service SCARS calls. In 0.157.0, the `icu` module provides initialization. The builder source explicitly leaves custom Unicode/client break hooks unwrapped. The bindings inspected do not expose the general SkUnicode break iterator/case conversion API or ICU `unum`, `udat`, and `ucal` services used by SCARS.

That is a dependency boundary, not a migration blocker. Unicode's Rust-native [ICU4X `icu` crate](https://docs.rs/icu/latest/icu/) provides segmentation, case mapping, normalization, calendars, and localized formatting. Use its supported behavior as the starting point for the Rust product. Exact correspondence with the old ICU4C calls is unnecessary.

Three areas still need explicit application-facing rules:

1. **Editor segmentation and text matching.** ICU4X's [segmenter](https://docs.rs/icu_segmenter/latest/icu_segmenter/) supplies grapheme and word segmentation; [`unicode-segmentation`](https://docs.rs/unicode-segmentation/latest/unicode_segmentation/) is another candidate for UAX #29 behavior. Choose one service and test expected navigation/deletion for combining marks, emoji and relevant scripts. Its boundaries need not match the C++ implementation exactly. The editor and Skia paragraph index conversions must still agree on valid positions.
2. **Localized numeric editing.** ICU4X's [decimal formatter](https://docs.rs/icu_decimal/latest/icu_decimal/) supplies localized output. Define the accepted input grammar, separators, grouping, incomplete drafts, exponent support and error feedback for Rust; the formatter alone does not implement those rules. [`scars_numeric.cpp`](../scars/src/scars_numeric.cpp#L22) provides useful cases rather than a required parser contract. If an integer field promises exact `i64` values, its parsing and stepping must not pass through `f64`. Keep draft status and committed values separate.
3. **Localized dates and civil-time resolution.** Use ICU4X's [datetime service](https://docs.rs/icu_datetime/latest/icu_datetime/) for localized output. Editable components or a calendar picker can have a defined input format without promising arbitrary localized free-text date parsing. Choose the supported calendars, ranges and locale fallback behavior explicitly. For named-zone conversion, [Jiff](https://docs.rs/jiff/latest/jiff/) is a candidate with explicit ambiguous-time handling; select and test the product's daylight-saving gap/overlap policy. The older [date/time service](../scars/src/scars_datetime.cpp#L36) is a source of edge cases, not a policy to reproduce.

Recommended next step: exercise ICU4X with representative editor, number and date examples, and define the input rules as part of those controls. Reuse relevant [numeric](../scars/tests/numeric.cpp) and [datetime](../scars/tests/datetime.cpp) scenarios, rewriting expectations when the Rust contract intentionally differs. This is normal integration work rather than proof of ICU4C compatibility. Native [`rust_icu` bindings](https://github.com/google/rust_icu) remain an alternative if a specific missing capability warrants them; there is no compatibility requirement driving that choice today. No option requires retaining C++ SCARS components.

### Initial localization scope: trade-offs and recommendation

Separate Unicode-correct editing, localized display, and localized parsing. An editor must preserve valid text, grapheme-aware movement/deletion, composition, selection and bidi text geometry regardless of the numeric/date input scope. Locale-aware formatting is another service; accepting a localized typed value adds parsing and editing policy. Full UI mirroring for right-to-left layouts is a further capability that shaping alone does not implement.

| Approach | What it provides | Cost and limitations |
| --- | --- | --- |
| ICU4X with explicit editable formats | Unicode services and localized display; number/date controls advertise a chosen input grammar or use structured fields/pickers | Smallest initial input surface. Users may need to distinguish display and edit formats; show clear format guidance and validation. Do not silently present localized text that the field cannot accept back. |
| ICU4X with locale-aware structured entry | Numbers recognize the chosen locale's supported digits/separators; date/time fields follow explicit locale order, field boundaries and hour cycle | Better consistency between display and entry. Requires parsing rules, partial-draft states, caret/selection handling, and tests across supported locales. It can remain structured rather than accepting arbitrary date prose. |
| Broad localized free-text entry | Many localized number/date forms, optional formats and potentially ambiguous input | Largest parser, UX and test burden. Inputs such as `1,234` or `03/04/2026` need an explicit locale/format context. The documented ICU4X formatter APIs do not establish this parser capability; evaluate a parser implementation or native ICU bindings if this becomes a requirement. |

**Recommended sequence:** use ICU4X for segmentation/case services and localized display from the foundation. Make the first usable slice a Unicode-correct text field plus basic controls; it does not need a date parser to prove the architecture. Before NumericField and date/time controls ship, choose their editable grammar and demonstrate representative locales. Prefer locale-aware numeric entry and structured date/time entry as the next design target; treat broad free-text parsing as a separately justified feature. This keeps the eventual component inventory intact and avoids making every control wait on the widest parsing scope. This sequence is a recommendation, with the initial localization scope still open for the user to choose.

The service boundary should keep locale, supported grammar, validation state and formatting separate from the stored typed value. Exact integers stay integer-valued; date/time fields distinguish civil values from instants and named zones. Neither invalid nor incomplete drafts should overwrite committed values. Tests should cover unsupported locale handling, decimal/grouping ambiguity, paste, native digits where supported, incomplete edits, external state changes, and daylight-saving gaps/overlaps when zones are offered.

Start with compiled ICU4X locale data for straightforward integration, then measure size before narrowing the locale set. ICU4X supports both compiled data and explicit providers, and date formatting offers choices of field sets and calendars that affect included data. Selecting the specific component crates/services can keep the dependency surface focused. These are documented options, not measured binary-size results for this project. See [ICU4X data management](https://docs.rs/icu/latest/icu/) and [date formatter size considerations](https://docs.rs/icu_datetime/latest/icu_datetime/).

## Native integration still needs accessibility and lifetime rules

The existing C++ [`mount`](../scars/src/scars.macos.cppm#L69) combines host invalidation, scheduled wakeups, callbacks, text input, clipboard, and accessibility. The Rust adapter can use existing `Surface` methods for most transport, but it still needs the component-side implementations. Search debounce, tooltip deadlines, repeat stepping, and inactive animation must use one-shot wake scheduling when appropriate; an always-running animation loop would change idle behavior. Define callback ownership explicitly because `set_callbacks` replaces the full callback set. Use weak surface/host connections to avoid cycles and cancel scheduled work on unmount. Rust pointer events currently lack SCARS's explicit `Cancel` kind: focus loss, removal, and unmount need a deliberate gesture-cancellation route. The existing text client's `intercept_key` hook can handle host shortcuts before IME without adding widget flags to native text snapshots.

Widget accessibility is missing in Rust despite enabled AppKit accessibility bindings. C++ exposes both realized elements and indexed logical list/tree/table elements; the low-level native implementation is partly outside `scars/`, in [`AccessibilityProvider`](../include/skia_window/skia_window.hpp#L104) and [`backend.mm`](../src/macos/backend.mm). Use those as references when implementing accessible Rust controls, without copying the provider API wholesale. [`objc2-app-kit` 0.3.2's `NSAccessibility`](https://docs.rs/objc2-app-kit/0.3.2/objc2_app_kit/trait.NSAccessibility.html) supplies protocol bindings for text, rows, columns, cells, selection, focus, actions, and expansion. Protocol availability does not create the semantic model or manage retained objects automatically.

Build a semantic provider over stable host IDs, with native objects resolving the current generation through weak host ownership. Virtual collections need accessible logical identity and indexed access without realizing every row; unavailable geometry should remain unavailable. Native objects held by assistive technology must become inert after removal/unmount/close. The C++ [native tests](../scars/tests/native.mm) and [native lifecycle tests](../scars/tests/native.cpp) contain useful scenarios to select for the new controls.

Password secure-input ownership, composition cancellation/commit, clipboard restrictions, screen-space candidate geometry, and native text notifications still need correct behavior. Alerts also need clear completion, parent-sheet isolation, close/destruction and focus rules; a native dialog surface alone does not provide a component alert model. See [`AlertPresentation`](../scars/src/scars.macos.cppm#L732) for an existing example.

Keep application setup small. A mount helper should attach components to the existing window API; the component library should not require applications to construct native menu plumbing. Preserve the current defaults: a basic application menu with Quit, other menus explicitly chosen by the app, and quitting when the last window closes unless overridden. These are existing Rust [documented behaviors](../rust/README.md), not reasons to add a new application framework.

## Suggested migration sequence and exit criteria

| Stage | Deliverable | Evidence required before broadening scope |
| --- | --- | --- |
| 1. Foundation and service choices | Portable state/host boundaries, IDs/generations, immutable descriptions, tracked bindings; Rust-like `view!` macro including conditions/repetition; ICU4X integration and paragraph/index checks | Documented dispatch, layout and input rules; macro/state examples with stable keys; subscription cleanup and panic-safe build tracking; representative Unicode/localization examples work |
| 2. First usable slice and basic controls | Themes, flex/block/inline/layer layout and styling; labels, buttons, toggles/check/radio/switch/segmented selection, slider/progress, image and grouping primitives; one text field; macOS mount/focus/input/semantics | A small working form expressed through the macro; headless geometry/raster tests; useful macro errors; real IME/candidate geometry, accessible actions, and safe unmount/close |
| 3. Shared editing and numeric controls | Full TextBuffer/TextField/TextEditor, selection/history/clipboard, search/password, numeric field and stepper; reusable draft validation and deadline scheduling | Unicode/editor cases, secure-input ownership, agreed numeric grammar, precise integer operations, incomplete drafts and commits; idle controls settle |
| 4. Navigation and presentation | ScrollView, overlay/focus scopes, menus/dropdowns/split actions, tooltips/popovers/context menus, SplitView, TabView, wizard and alerts/sheets | Nested scrolling/reveal, capture cancellation, retained inactive page state, menu navigation, focus restoration, exactly-once alert completion and parent isolation |
| 5. Collections and structured editing | RowModel/ListBox virtualization, editable ComboBox, lazy TreeModel/TreeView, TableModel/TableView and indexed accessibility | Bounded realization/retention on large data; stable offscreen identities; correct sort/selection/reorder, edit conflicts and model changes |
| 6. Composite values and developer tools | ColorPicker/ColorWell, DatePicker/TimePicker/local and zoned DateTimePicker; inspector/picking/overrides and Lab | Chosen color/date/time editing policies; draft validation/commit/cancel; native lifetimes; inspection of real controls; both palettes and constrained-size previews |

The recommended first usable milestone is the end of stage 2. One complete text field deliberately tests the difficult editor/native boundary early; stage 3 then generalizes it instead of postponing IME until many controls exist. Basic controls reuse the same layout, focus, hit-testing and semantics. Stages 4 and 5 build the presentation and collection machinery used by the advanced controls and inspector. Stage 6's value controls depend mainly on stages 3–4 and can be brought forward independently of tables if useful. This ordering follows shared implementation dependencies rather than an assumed application.

For inventory closure, stage 2 also includes Separator, Spacer, Section, GroupBox, Panel, Bitmap/Image content modes and the `Text`/`CommandButton` aliases; the macro's condition/repetition covers the capabilities of `If`/`ForEach`. Stage 4 includes MenuButton, SelectionDropdown, SplitActionButton, WizardNavigation, Alert/confirmation presentation, and scrolling in both axes. View modifiers, theme changes, semantic labels and cancellation rules belong with the first control that uses them. The [public API inventory](../scars/src/scars.cppm#L637) is the checklist for eventual coverage, while spelling and exact semantics can change in Rust.

All current SCARS components are in eventual scope. Accessibility begins with the first interactive slice, then gains collection-specific behavior alongside virtualization. Localization service checks occur early even if date pickers arrive later. Features found only in the older UI engine remain outside the confirmed inventory unless separately requested.

The work should be estimated after the first slice establishes the Rust API and implementation costs. Source size and a compiling window example do not justify a reliable calendar estimate for the complete control suite. Removing compatibility requirements reduces the migration burden but does not remove the shared component runtime work.

## Validation and remaining decisions

Write tests for the Rust contracts, borrowing useful scenarios from C++ rather than porting all assertions. Use deterministic headless tests for geometry, state, editor commands, deadline scheduling, draft transactions, and logical semantics; raster previews for both themes at 1×/2× and constrained widths; native tests for actual AppKit transport, focus, clipboard, secure input, IME, notifications, accessibility, and sheet lifetimes. Rust ownership checks and successful compilation complement these tests; they cannot establish user-visible behavior by themselves.

Establish Rust baselines for bounds, identities, text/ranges, committed values, callback counts, semantic roles/actions, realized counts, and wake deadlines. Add compile checks for the macro's valid examples and representative diagnostics. Side-by-side C++ comparisons can help diagnose algorithms or assess visual changes, but they are optional tools rather than release gates. Control fonts and scale for reproducible Rust raster tests. macOS is the first native target; verify the portable core independently, with Windows/Linux native adapters remaining separate work.

Confirmed decisions are eventual coverage of all current SCARS components, macOS-first delivery with a portable core, Rust-like nested macros with reactive state, and delivery ordered by shared dependencies. No additional product answers are needed to establish feasibility or propose that order. The initial localization/input scope remains deliberately open: the comparison above supports making that choice, and the recommended sequence does not settle it on the user's behalf.

Implementation design should next settle the macro grammar and capture rules, event-dispatch contract, state API, layout conventions, and numeric/date input formats before those capabilities are built. ICU4X is the starting localization choice, subject to normal integration checks. The current Rust native/Skia layer is a suitable base for a Rust host and control system informed by the C++ work.

## Initial implementation history

The first six milestones produced examples and tests, but did not close every
original exit criterion. The [completion report and checklist](rust-completion-checklist.md)
is now the authoritative work list for the remaining macOS component and portable
core work. Native Windows/Linux backends remain outside the confirmed scope.

## Rust implementation checkpoints

The first implementation of all six stages has committed Rust examples and focused tests.
See the [quick guide](rust-guide.md) for commands and explicit current limits, and
[component contracts](../rust/components/README.md) for behavior. The original
feasibility discussion above is historical planning context, not a parity claim.

| Stage | Commit |
| --- | --- |
| Reactive foundation | `c0a2206` |
| Layout/basic controls/native mount | `536dbc7` |
| Editing/typed drafts/deadlines | `64c217d` |
| Scrolling/presentation | `aebbe14` |
| Virtual collections/structured edits | `5fc900f` |
| Composite values/developer tools | `cc9afc9` |

The final todo example exercises application-level bindings, keyed rows, inline
editing, filtering and persistence without application-side reference counting.
