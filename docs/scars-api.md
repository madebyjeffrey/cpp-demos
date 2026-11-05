# SCARS API quick guide

SCARS is a standalone C++23 module-based UI engine. It renders through Skia, shapes text with SkParagraph, and currently mounts on macOS. Link `scars::macos` for a windowed app or `scars::scars` for a native-independent host. Build instructions and the complete contracts are in [scars.md](scars.md).

## Views, state, and modifiers

Include the standard headers your code uses; SCARS does not re-export the standard library. Every modifier returns a new view. `State<T>` belongs in a retained model; `.binding()` supplies two-way control access. State changes rebuild subscribed hosts on the owning thread.

```cpp
#include <memory>
#include <string>
#include <vector>
#include <skia_window/skia_window.hpp>
import scars;
import scars.macos;
namespace s = scars;

struct Settings {
    s::State<std::string> name{"Kestrel"}, notes{""}, station{"nav"};
    s::State<bool> showDetails{true};
    std::vector<s::Option> stations{{"nav", "Navigation"}, {"sci", "Science"}};

    s::View body() {
        return s::FlexColumn{
            s::TextField("Ship name", name.binding(), {.max_length = 48})
                .key("name").accessible_label("Ship name"),
            s::TextEditor(notes.binding(), "Mission notes").height(100).key("notes"),
            s::ListBox(station.binding(), stations).height(140).key("stations"),
            s::CheckBox("Show details", showDetails.binding()),
            s::If(showDetails.get(), [this] {
                return s::Label("Station: " + station.get());
            }),
            s::Button("Reset", [this] { name.set("Kestrel"); notes.set(""); }),
        }.gap(12).padding(24);
    }
};

// After creating an Application and Window with skia_window:
auto model = std::make_shared<Settings>();
auto host = s::macos::mount(*window, [model] { return model->body(); });
window->show();
```

Keep the application alive until its windows close. `mount` installs drawing, events, text input, clipboard, and accessibility. It owns no legacy UI controls. The window retains the mounted host; the model above is retained by the builder.

## Layout

| View / modifier | Use |
| --- | --- |
| `FlexRow{...}`, `FlexColumn{...}` | Flex layout; `.gap()`, `.grow()`, `.shrink()`, `.basis()`, `.wrap()`, `.align_items()`, `.justify()` |
| `Block{...}` | Vertical flow without flex shrinking |
| `Inline{...}` | Styled text spans and embedded views on text baselines |
| `ScrollView(child, options)` | Vertical (default), horizontal, or both-axis scrolling and focus reveal |
| `Layer{...}` | Overlap children; `.z_index(int)` orders sibling subtrees |
| `Panel(child)`, `Section(title, child)`, `GroupBox(title, child)` | Surfaces and semantic groups |

Dimensions accept logical pixels (`.width(160)`), percentages (`.width(s::percent(50))`), or `s::automatic`. Use min/max dimensions, `.padding()`, and `.margin()` as needed. Explicit dimensions include padding. `.position(s::Position::relative)` moves an element while keeping its original flow space. `.position(s::Position::absolute)` anchors it to its immediate parent's content box and removes it from flow. `.left()`, `.top()`, `.right()`, and `.bottom()` accept signed lengths and percentages; opposing anchors stretch automatic sizes.

Higher z-index values appear above lower ones; equal values preserve declaration order. `.hidden()` keeps layout space but hides the subtree and disables its input. `.visible(false)` removes it from layout. Use stable `.key()` values for reorderable views.

## Scrolling

```cpp
s::ScrollView(
    s::Layer{ /* document content */ }.width(1200).height(800),
    {.axes = s::ScrollAxes::both}
).key("map").height(300).padding(8);

// scrollID is the mounted ID reported by Host::inspect(), rather than a key alias.
host.scroll_to(scrollID, {120, 80});
auto offset = host.scroll_offset(scrollID); // optional<Point>
```

`ScrollAxes::{vertical,horizontal,both}` selects the unconstrained document dimensions; other dimensions fit the viewport. The document fills at least the viewport. View padding defines the viewport, and document margins contribute to its extent. Offsets use positive logical coordinates: increasing x/y moves content left/up. Limits update after resizing or document changes. Finite negative or oversized requests clamp; invalid IDs, hidden/disabled views, and blocked focus scopes reject `scroll_to`. Offsets survive hiding and keyed rebuilds, and are discarded on removal or replacement with a non-scrolling view.

Trackpad input can move both axes in one gesture. Shift-wheel maps vertical-only wheel movement to x. Nested scrolling consumes movement at the innermost matching viewport, then passes each axis's remainder to its ancestors. Text-editor and tab-strip boundaries also pass unused movement onward. Overlay scrolling stays inside its layout scope. Focus reveal resolves inner viewports before outer ones, including horizontal movement; it does not scroll the anchor's ancestors when focusing overlay content.

Thin scroll indicators overlay content without changing its extent. Set `.show_scrollbars = false` to hide them. `InspectionNode::scroll` reports axes, offsets, limits, document size, and viewport geometry, also shown by the developer inspector. `scroll_to` supports ScrollView and ListBox; ListBox remains vertical.

## Split views

Keep the divider fraction and optional collapsed pane in the retained model:

```cpp
s::State<double> fraction{.4};
s::State<s::SplitPane> collapsed{s::SplitPane::none};

s::SplitView(
    s::Panel(s::Label("Navigation")),
    s::Panel(s::TextEditor(notes.binding())),
    fraction.binding(),
    {.first_min = 160, .second_min = 220, .collapsed = collapsed.binding()})
    .height(280).accessible_label("Mission panes");
```

The fraction assigns the first pane's share after subtracting the divider. Horizontal places panes side by side; `.orientation = s::Orientation::vertical` stacks them. Options also include `divider_width` (12 logical pixels) and `keyboard_step` (10). Give a split an explicit height or flex growth when it should fill available space. Minimum sizes apply along the split axis; insufficient space scales both minima proportionally. Layout clamps the displayed fraction without overwriting the bound value.

Drag the divider, or Tab to it and use Left/Right for a horizontal split and Up/Down for a vertical split. Shift moves ten steps; Home/End reach the pane limits. Escape during a drag restores the starting fraction. Pointer cancellation or window deactivation ends capture at the current position.

With a collapse binding, Enter/Space or a double-click toggles the first pane; Shift targets the second. Escape restores collapsed panes. Collapse preserves the saved fraction and mounted editing/scroll state while suppressing the collapsed subtree's layout extent, input, accessibility, and animation. A later arrow or drag adjustment restores both panes and writes the adjusted fraction. Without the binding, collapse actions have no effect.

The inspector uses SplitView for its resizable tree and property panels. Split inspection snapshots include axis, saved fraction, effective pane extents, minima, divider extent, and collapse state. `Host::cursor(point)` reports divider resize cursors independently of the native adapter, and `PointerKind::cancel` releases an active pointer interaction.

## Tab views

Keep the selected tab ID and each page's business state in the retained model. Page content uses builders so unvisited pages are created lazily:

```cpp
s::State<std::string> selectedTab{"profile"};

s::TabView(selectedTab.binding(), {
    {"profile", "Profile", [this] {
        return s::TextField("Ship name", name.binding()).key("name");
    }},
    {"notes", "Notes", [this] {
        return s::TextEditor(notes.binding()).key("notes");
    }},
    {"offline", "Offline", [] { return s::Label("Unavailable"); }, false},
}).height(240).accessible_label("Ship settings");
```

Tab IDs must be nonempty and unique. A page builder runs when its page becomes active and on subsequent subscribed rebuilds while active. Inactive visited pages retain their last description and mounted editing/scroll state; they receive no layout allocation, input, accessibility elements, or animations. This differs from `.hidden()`, which ordinarily keeps layout space, and `.visible(false)`, which removes mounted state. Removing a tab or unmounting its TabView releases that page's host-owned state. Model values persist independently. Builders must not mutate observable state.

If the requested ID is missing or disabled, the first enabled tab becomes the displayed page. With no enabled tabs there is no active page. Construction never writes a fallback ID into the binding: an explicit tab activation commits the new selection. The inspector shows the requested and effective selection, logical and visited tab counts, and tab-strip scroll offset.

Tab navigation enters the selected enabled tab as one focus stop, followed by its active content. Left/Right on a tab select the previous/next enabled tab and wrap; Home/End select the first/last. Control-Tab and Control-Shift-Tab switch within the nearest containing TabView, including from an editor. An overflowing strip scrolls to reveal its focused tab and accepts wheel scrolling. Disabled tabs cannot be activated.

Navigation actions commit active input-method composition before switching and recheck the destination's availability after commit. A programmatic selection change follows the existing hidden-control policy: uncommitted composition is cancelled, and focus moves to the displayed tab if the previously focused page becomes inactive. Reordering tabs preserves state through stable IDs, including nested TabViews.

## Numeric fields, steppers, and switches

NumericField and Stepper accept `Binding<double>` or signed integral bindings, including exact `std::int64_t` values. Their typed `NumericOptions<T>` share range, increment, locale, and formatting rules:

```cpp
s::State<double> power{6.5};
s::State<int> crew{4};
s::State<bool> automaticScan{true};

s::NumericField("Power", power.binding(),
    {.min = 0, .max = 10, .step = .25, .significant_digits = 6})
    .accessible_label("Target power");
s::Stepper(crew.binding(), {.min = 1, .max = 20})
    .accessible_label("Crew count");
s::Switch("Automatic scan", automaticScan.binding());
```

NumericField keeps a text draft separate from its numeric binding. Enter commits valid input and invokes `on_submit`; Escape cancels composition first, then restores the committed value. Valid drafts commit on blur by default. Invalid drafts remain visible with an error border and validation help; focus can leave the field without clamping or losing the draft. Set `commit_on_blur = false` for explicit commits. Up/Down and Page Up/Down commit a valid draft and then adjust by one or ten steps. Text selection, clipboard, undo/redo, and IME use the existing editing engine. Undo edits the draft; Enter commits it.

Options default to the type's finite range, step 1, locale `en_US_POSIX`, 17 significant display digits for decimals, no grouping, and editable input. `locale = "de_DE"` with `grouping = true`, for example, accepts `1.234,5`. Parsing and formatting use the pinned ICU stack. Typed commits reject malformed, non-finite, overflowing, and out-of-range values. The step controls adjustment size; typed values are not rounded to a step grid. Display precision never changes an untouched model value. External model or format changes replace stale drafts and history.

`on_validation` receives changes to editing validation (`valid`, `incomplete`, `invalid`, `out_of_range`, message, and `dirty`) during events. It does not run during view construction or external model reconciliation. `Host::numeric_validation(id)` and inspection snapshots expose current validation; `Host::commit_numeric(id)` commits explicitly. Read-only options reject editing, commits, and adjustments.

Stepper has decrement/value/increment regions and one keyboard focus stop. Arrows adjust by one step, page keys by ten, and Home/End reach the limits. Pointer presses act immediately, then repeat after `repeat_delay` (400 ms) at `repeat_interval` (80 ms). Leaving the button pauses repeat; release, cancellation, blur, focus loss, hiding, disabling, or removal stops it. Endpoints disable the corresponding button. Wheel scrolling does not alter values. Switch activates through its full row or Space, with native switch accessibility semantics.

The macOS adapter schedules repeat redraws automatically. Custom hosts should schedule one redraw for `Host::next_wake()` after painting; `paint()` processes due work, or tests may call `advance_time()` explicitly. Inject a clock through the fifth Host argument (`HostOptions`) for deterministic tests. Repeat delay must be nonnegative and interval positive, both at most one day. Idle steppers schedule no work and do not require continuous frames.

## Repetition and conditions

`ForEach` expands directly into its parent's layout. Supply a stable, unique string or integer key, rather than an array position that can change.

```cpp
s::FlexRow{
    s::ForEach(stations,
        [](const s::Option& item) { return item.id; },
        [](const s::Option& item) {
            return s::Button(item.label, [id = item.id] { /* use id */ });
        }).key("station-buttons")
}.gap(8).wrap();

s::If(showDetails.get(),
    [this] { return s::Label("Selected: " + station.get()); },
    [] { return s::Label("Details hidden"); });
```

Only the selected `If` builder runs. The alternative is optional; an omitted branch contributes no space. Both structural views are transparent in a parent: apply layout/style modifiers to their generated children or wrap them in a layout container. `.key()`, `.hidden()`, `.visible()`, and `.enabled()` work on the structural group. Capture item IDs by value in callbacks. Removed views lose host-owned focus/edit history; model-owned `State` persists.

## Text input

`TextField(placeholder, Binding<std::string>, options)` is single-line. `TextEditor(binding, placeholder, options)` wraps multiline text. Both support style/layout modifiers and `.text_align()`.

`TextFieldOptions` has `read_only`, `max_length` (0 = unlimited; counts Unicode graphemes), and `on_submit`. Enter submits a single-line field; multiline Enter inserts a newline and Command-Enter submits. Read-only fields allow selection and copy. Disabled or hidden fields reject input.

| Input | Action |
| --- | --- |
| Click / drag; double / triple click | Place / extend selection; select word / line |
| Arrows; Shift + movement | Move caret; extend selection |
| Option-Left/Right; Option-Up/Down | Move by word; move by paragraph |
| Command-Left/Right; Command-Up/Down | Visual line start/end; document start/end |
| Home/End; Page Up/Down | Document start/end; page movement |
| Backspace / Forward Delete | Delete previous / next grapheme |
| Option-Delete; Command-Backspace | Delete a word; delete to visual line start |
| Command-A/C/X/V | Select all / copy / cut / paste |
| Command-Z; Shift-Command-Z | Undo / redo in the focused field |
| Tab / Shift-Tab; Escape | Change focus; cancel active composition |

AppKit supplies input methods/dead keys, marked-text callbacks, candidate-window anchoring, native Edit menus, and the context menu. SCARS owns the UTF-8 buffer, grapheme/word boundaries, selection, undo/redo, and SkParagraph layout. Clipboard and paste are plain text. Selection/caret reveal works inside fields; multiline editors also scroll internally.

For automation/custom adapters, use `Host::text_input()`, `insert_text`, `mark_text`, `unmark_text`, `text_command`, `select_text`, and `set_text`. SCARS ranges use **UTF-8 byte offsets**, snapped to grapheme boundaries. The macOS adapter converts AppKit's UTF-16 ranges. `TextBuffer` offers the underlying editor without a view; `Host::set_clipboard` accepts clipboard callbacks for headless use. External binding replacement clears stale undo/composition history.

## Search and password fields

Both controls use the existing text editor and accept a string binding:

```cpp
s::State<std::string> query{""}, filteredQuery{""}, accessCode{""};

s::SearchField("Find a station", query.binding(), {
    .debounce = std::chrono::milliseconds(180),
    .on_search = [this](const std::string& text) { filteredQuery.set(text); },
}).accessible_label("Search stations");
s::PasswordField("Access code", accessCode.binding(), {.allow_reveal = true})
    .accessible_label("Access code");
```

Search updates its binding on each committed edit. `on_search(query)` runs immediately by default, or after the configured debounce (zero through one day). New input replaces pending work. `on_submit(query)` handles explicit Enter; Enter flushes pending search first. The first Enter during composition accepts the candidate, and a subsequent Enter submits. Escape cancels composition first, then clears a nonempty query; on an empty query it invokes optional `on_cancel`. Clear also resets the filter immediately and returns focus to the editor.

Command-F selects the first enabled, visible SearchField in the current host. Use `Host::focus_search(id)` to target a particular field, or `flush_search(id)` to deliver its pending callback. `clear_button`, `read_only`, and grapheme `max_length` customize editing. Pending search survives field blur while the window remains active, but is cancelled on external query/policy replacement, hiding, disabling, removal, read-only mode, or window deactivation. Composition pauses pending work and resumes it after cancellation. External model pushes do not invoke editing callbacks.

PasswordField displays one bullet per Unicode grapheme and preserves normal cursor, selection, paste, and IME behavior. `allow_reveal` defaults to false; when enabled, Show/Hide preserves selection and stays within the field's editing scope. Leaving that scope, deactivation, hiding, disabling, read-only mode, or an external model replacement remasks it. `read_only`, grapheme `max_length`, and `on_submit` work as with single-line text input; a composition candidate is accepted before Enter can submit.

Password copy and cut are blocked, including during visual reveal. Undo is disabled by default; `allow_undo = true` permits undo/redo while editing, with history cleared on blur, hiding, or teardown. Inspection and accessibility always expose bullets, with secure field semantics and no accessible text selection. The macOS adapter balances secure keyboard input with actual editable focus and prevents native substring queries from exporting passwords.

`Host::text_input()` is a privileged adapter interface: its raw text and UTF-8 selection are needed for native input-method range conversion. Its `secure` and `active` flags identify password input and focus state. Use `elements()` or `inspect()` for redacted presentation. Password model values remain ordinary application-owned strings; this control does not provide encrypted storage.

## Tooltips and popovers

Attach help and arbitrary content with chained modifiers. Keep presentation and form data in the model:

```cpp
s::State<bool> showingSettings{false};

s::Button("Settings", [this] { showingSettings.set(true); })
    .tooltip("Change the scan settings.")
    .popover(showingSettings.binding(), [this] {
        return s::FlexColumn{
            s::TextField("Ship name", name.binding()),
            s::Switch("Automatic scan", automatic.binding()),
            s::Button("Done", [this] { showingSettings.set(false); }),
        }.gap(12);
    }, {.width = 320, .max_height = 400});
```

Tooltip defaults are a 600 ms delay, 280 px maximum width, and an 8 px gap and window margin. Hover or keyboard focus schedules one wake; movement off the owner, active input, hidden/disabled/removed owners, and window blur cancel it. A click suppresses reappearance until pointer reentry. Tooltips wrap, flip to fit, expose their text as accessibility help even while hidden, and add no focus stop. An empty tooltip string removes the attachment.

Popover content is built only while presented and shares normal SCARS state, editors, input methods, and inspection. It has a padded panel and vertical scrolling when `max_height` is exceeded. `OverlayPlacement::{below,above,left,right}` chooses the preferred side; placement flips or clamps within the window. The body escapes the anchor's layout and scroll clipping. `InspectionNode::overlay_scope` identifies this separate layout scope.

`PopoverFocus::content` (default) focuses the first control and confines Tab, focus, and accessible actions to the content. `preserve` keeps the previous editor active until content receives focus, useful for suggestions. Escape cancels marked text first, then closes the topmost presentation. Outside clicks close only the top overlay and consume the click. `dismiss_on_escape` and `dismiss_on_outside` can disable those dismissals. Closing restores prior focus when available; fully clipped anchors close without scrolling back to them. Hidden, disabled, removed, or inactive owners clear presentation and descendants. `on_dismiss` runs once, outside view construction, including external binding changes. Nest popovers or existing selection menus inside a body using the same modifiers.

## Context menus

Attach a menu to any control or layout view. A builder can derive items from current model state:

```cpp
s::Panel(s::Button("Report", [this] { report(); }))
    .context_menu([this] {
        return std::vector<s::MenuItem>{
            {"report", "Report", [this] { report(); }, true, "⌘R"},
            s::MenuItem::separator("settings"),
            s::MenuItem::check("grid", "Show grid", grid.binding()),
            s::MenuItem::submenu("station", "Station", {
                s::MenuItem::radio("nav", "Navigation", station.binding(), "nav"),
                s::MenuItem::radio("sci", "Science", station.binding(), "sci"),
            }),
            {"offline", "Offline", [] {}, false},
        };
    });
```

Right-click or Control-click invokes the nearest attached view under the pointer. Shift-F10 invokes the nearest attachment around the focused control, including an ancestor container. `Host::show_context_menu(ownerID, optionalPoint)` supports application-driven invocation; without a point it anchors below the owner. Menu and submenu geometry fits the window, escapes anchor clipping, and scrolls for long menus. Submenus open on hover, Right, Enter, or accessible activation.

Arrow keys and Home/End skip separators and disabled items; printable text moves to a matching item. Left or Escape closes one submenu at a time. Tab dismisses the chain and resumes ordinary focus navigation. Acceptance closes the chain before invoking the action; outside clicks dismiss and are consumed. Existing text composition commits before menu invocation, text-input routing pauses while menus are open, and closing restores prior focus. Context menus can live inside popovers.

Every sibling item needs a unique nonempty ID, including separators. Check and radio factories read/write their bindings, while `.enabled` can disable any item. Empty submenus are disabled. Builders and `is_checked` getters are pure and participate in normal state collection; reordered items keep their identity and highlight. Shortcut strings are display labels exposed to accessibility; application command routing owns shortcut execution. `Element` exposes selected state, submenu expansion, shortcut labels, and menu/menu-item roles. Inspector snapshots include menu hierarchy and separators. Hidden, disabled, removed, clipped, or inactive owners dismiss their menus.

## Combo boxes

Keep editable query text and the accepted stable option ID in separate bindings:

```cpp
s::State<std::string> stationQuery{"Navigation"}, selectedStation{"nav"};

s::ComboBox("Choose a station", stationQuery.binding(), selectedStation.binding(), {
    {"nav", "Navigation"}, {"sci", "Science"}, {"offline", "Offline", false},
}, {.on_query = [this](const auto& text) { /* optionally request suggestions */ },
    .on_accept = [this](const auto& id, const auto& text) { /* use accepted value */ } });
```

Typing updates the query and calls `on_query` without changing the selected ID. Default `ComboBoxMode::selection` accepts an enabled suggestion; Enter or a suggestion click writes its ID and label, closes the list, and calls `on_accept`. An unmatched query remains a draft with validation feedback. Escape restores the editing checkpoint; Tab, outside dismissal, focus loss, hiding, disabling, removal, and window deactivation restore unaccepted strict text. These restorations do not invoke `on_query` or `on_accept`.

Set `.mode = ComboBoxMode::free_text` to accept unmatched text with an empty selected ID. Free-text blur keeps the query and clears an ID whose label no longer matches, without firing explicit acceptance. Escape still restores the checkpoint. `.auto_highlight = false` lets Enter accept raw free text until the user explicitly highlights a suggestion.

`ComboBoxFilter::{contains,prefix,none}` controls filtering; the default uses case-insensitive Unicode matching. Use `none` when the application supplies already filtered or ranked results. Pass `.loading = true` while fetching suggestions, then update options and loading on the host's owning thread. Loading suppresses stale rows and acceptance. `loading_text` and `no_results` customize accessible status labels. `max_height` defaults to 240 px and `row_height` to 32 px (minimum 12). Normal text-field `read_only` and `max_length` policies apply to editing.

Up/Down and Page Up/Down browse enabled suggestions while retaining editor focus. Option-Down opens and Option-Up closes the list; Home/End retain their text-cursor behavior. Tab visits the next control without stopping on the arrow or suggestion rows. Enter accepts marked text before accepting a ComboBox value; Escape cancels composition before the draft. Suggestions are suspended during composition and return afterward. Clipboard, selection, cursor geometry, and undo use the normal SCARS text editor.

The application owns the initial and externally supplied query text: changing the selected ID alone does not overwrite it. External query/ID replacements close stale suggestions and establish a new checkpoint. `InspectionNode::combo` reports selection, highlight, loading, match count, and validation; `Element::combo` and `TextInputSnapshot::combo` identify editing semantics. The macOS bridge exposes a native combo-box role and a suggestion list while retaining the same input-method client. Suggestion row selection represents the navigation highlight; the accepted ID remains in the model and inspector.

## Wizard navigation

Retain a `Wizard` controller and its model data. `WizardNavigation` renders Back/Next/Finish, an optional Cancel action, a numbered step strip, and validation feedback:

```cpp
s::State<std::string> step{"identity"}, shipName{""};
s::Wizard wizard(step.binding(), {
    {"identity", "Identity", [shipName] {
        return shipName.get().empty() ? "Enter a ship name." : "";
    }},
    {"review", "Review", {}},
}, {.on_finish = [] { /* complete the workflow */ },
    .on_cancel = [] { /* close or reset it */ }});

s::FlexColumn{
    s::If(step.get() == "identity",
        [shipName] { return s::TextField("Ship name", shipName.binding()); },
        [shipName] { return s::Label("Ready: " + shipName.get()); }),
    s::WizardNavigation(wizard),
}.gap(16);
```

Step IDs must be unique and the bound current ID must exist. Validators are pure functions: return an empty string for valid data or an explanatory message to block forward navigation. They are reactive when they read `State`, and are checked again on activation. Finish validates every step by default. Back and Cancel remain available when a step is invalid; the step strip permits reviewing previous steps, never skipping ahead.

Use `wizard.next()`, `back()`, `go_to(id)`, `finish()`, or `cancel()` for programmatic navigation. `index()`, `is_last()`, `can_back()`, `can_advance()`, and `validation_message()` support a custom interface. Finish requires an `on_finish` handler; callbacks own completion/cancellation policy, including hiding or disabling the wizard during asynchronous work. `on_change(from, to)` reports accepted transitions.

`WizardOptions` controls Back/review policy, final validation, labels, and visibility of the step strip/status. For compact navigation set `show_steps` and/or `show_status` to false. The convenience constructor `WizardNavigation(binding, steps, options)` works without retaining a controller. Place step data in your model so it survives page unmounts; the navigation control does not own the form pages. The Lab includes a three-step setup demo.

## Lists and bitmaps

```cpp
s::State<std::string> selected{"nav"};
s::State<std::vector<std::string>> selectedMany{{"nav", "sci"}};
s::ListBox(selected.binding(), stations, {.visible_rows = 5});
s::ListBox(selectedMany.binding(), stations, {.row_height = 36, .visible_rows = 4});

// Load assets once in the model, rather than on every body rebuild:
auto bitmap = s::Bitmap::load("assets/telemetry.png");
s::Image(bitmap, s::ContentMode::fit).width(320).height(180)
    .accessible_label("Telemetry plot");
// Bitmap::from_rgba(width, height, bytes, optional_row_stride) copies RGBA8 data.
```

Lists support scrolling, arrows, Home/End, Page Up/Down, type-ahead, and optional `on_activate` on Enter or double-click. Multiple selection adds Command-click toggling, Shift ranges, and Command-A. Disabled `Option` rows remain visible and cannot be selected by user input. Selection values are stable option IDs.

ListBox virtualizes fixed-height rows: it mounts the visible range, `overscan_rows` on either side (default 2), and an active editor's row. The existing vector/initializer-list overloads use the same realization path. For large datasets retain a `RowModel` snapshot in the application model; constructing it validates unique nonempty IDs and indexes all metadata once. Copying the snapshot is inexpensive. Replace it explicitly when membership, order, labels, or enabled states change. Scroll frames reuse its index without scanning the full dataset.

```cpp
// Model members; construct this snapshot once, rather than in body():
s::State<std::string> selected{"0"};
s::RowModel rows{load_options()};

// body():
s::ListBox(selected.binding(), rows,
           {.row_height = 36, .visible_rows = 8, .overscan_rows = 2});

// Optional custom rows; callbacks run only for realized rows:
s::RowModel editable{load_options(), [model](const s::Option& row, std::size_t index) {
    return s::TextField("Observation", model->observation(row.id)).key("observation");
}};
```

The custom builder returns content inside the fixed-height row. Keep bindings and persistent values in the application model, keyed by row ID. Builders must not mutate observable state. Host-owned editor buffers, undo history, and subscriptions are released when a row is evicted; the focused editor stays mounted while offscreen, including during input-method composition. On leaving that editor, normal commit/cancel rules apply and its pin is released. Removing an editing row cancels its composition. Container focus uses normal list navigation; Tab can enter interactive content in realized custom rows. Row builders are synchronous; asynchronous loading belongs in the application model.

`host->list_item(listID, index)` or `host->list_item(rowID)` returns logical metadata even when a row is offscreen. Unrealized rows have no bounds or clip rectangle. `host->reveal_item(listID, index)` realizes and reveals a row without selecting it; `focus(rowID)` reveals and moves logical focus, and `activate(rowID)` applies the list's selection policy. Row IDs remain stable across reordering. Use the returned logical ID instead of manually escaping keys. `elements()` and `inspect()` enumerate mounted nodes; `Element::item_count` / `selected_items` and `InspectionNode::list` describe the full dataset and current visible/overscan ranges. Native accessibility provides lazy indexed rows, selected rows, actual visible rows, and focus-based offscreen reveal without constructing every row.

Bitmaps are immutable shared assets. File loading decodes formats enabled in the linked Skia build; invalid files throw. RGBA input is unpremultiplied and copied. `fit` preserves aspect ratio with letterboxing, `fill` crops, `stretch` fills both dimensions, and `center` keeps native pixel size. A single explicit image dimension derives the other from the aspect ratio. `.content_mode()` can change the mode after construction.

## Trees

Keep a `TreeModel` snapshot, stable selected node IDs, and expanded branch IDs in the application model:

```cpp
s::State<std::string> selected{"ship"};
s::State<std::vector<std::string>> expanded{{"fleet"}};
s::TreeModel tree{{{"fleet", "Fleet", true, false, {{"ship", "USS Kestrel"}}}}};

// body():
s::TreeView(selected.binding(), tree, expanded.binding(),
            {.row_height = 32, .visible_rows = 8, .overscan_rows = 2, .indentation = 18});
// A Binding<vector<string>> selection enables multiple selection.
```

Node IDs are globally unique and nonempty across eager and loaded subtrees. `TreeNode::children` supplies eager children; alternatively, `lazy_children` is a pure snapshot loader returning `vector<TreeNode>`. It runs once when that branch first expands in a visible tree. Collapsing and reopening reuse its metadata. Failed or duplicate-ID results leave the materialized snapshot intact and may be retried. A loader returning no children becomes a leaf unless `expandable` was explicitly set.

```cpp
s::TreeNode archive{"archive", "Archived observations"};
archive.lazy_children = [] { return load_archive_snapshot(); };
s::TreeModel treeWithArchive({std::move(archive)});
```

Loaders run synchronously during view construction and must not mutate observable state. For asynchronous data, use an application-owned `State<TreeModel>`, represent an unloaded/loading branch with `expandable = true` and/or `loading = true`, and request work from the expanded binding's setter or `TreeViewOptions::on_expand(id, expanded)`. Publish a replacement snapshot when data arrives. Metadata, order, enabled states, or loader results change through snapshot replacement; cached loaders are not reactive data sources. A disabled node and its descendants reject user selection and expansion. Loading branches show their status without inventing child rows.

Up/Down, Home/End, Page Up/Down, and typeahead navigate the expanded hierarchy. Right expands a branch or enters its first enabled child; Left collapses it or selects its parent. Enter calls `on_activate`, or toggles a branch when no activation callback is set. Disclosure clicks and branch double-clicks toggle expansion; leaf double-clicks invoke `on_activate`. Multiple selection adds Shift ranges, Command-click toggles, and Command-A over enabled nodes in the expanded hierarchy. Tab treats the tree as one control.

Selection and expansion belong to the model. Collapsing a selected descendant retains its selected ID while moving logical keyboard focus to the collapsing ancestor. Reopening restores its visible selection. Removed or collapsed IDs are not silently deleted from bindings during construction. Changed external single selection reveals a matching node if its ancestors are expanded; expand the ancestor path in the model to reveal a collapsed descendant.

TreeView shares ListBox's fixed-height viewport, overscan, focus routing, overlay handling, and bounded row realization. Flattening and indexing happen on explicit model/expansion changes, never on scroll frames. `Element::item_count` is the number of logical rows in the expanded hierarchy. `InspectionNode::tree` reports all known materialized nodes, expanded branches, and loading rows; `list` reports the viewport and mounted counts. Mounted elements and logical items provide `tree` metadata with parent node ID, depth (roots are zero), child count, enabled/expanded/loading states, and optional accessible label. `TreeModel::item(id)` looks up known snapshot metadata; `flatten(expanded)` provides an explicit expansion-specific preorder snapshot.

`host->set_expanded(rowID, bool)` changes a visible logical branch through its binding. `host->tree_child(rowID, index)` reads an immediate child from the expanded logical hierarchy without realizing it; `list_item`, `focus`, and `reveal_item` work as for ListBox. Unrealized rows have no bounds. Native accessibility exposes an outline with lazy rows, disclosure levels, disclosed parent/child relationships, expansion setters, selection, and focus-based offscreen reveal.

The developer inspector uses this same TreeView. Filters retain ancestors as structural context, and picking/revealing a source node opens its path and reveals its row. Branch folding only changes the inspector's presentation. The Lab's hierarchical browser demonstrates nested, disabled, lazy, and loading nodes; `--render-tree` renders a standalone preview.

## Tables

TableView provides virtual rows, row/cell selection, resizing, sorting, draft editing, and native indexed column/cell accessibility. Full native foreground/rendering verification passes in both configurations; see the [completion audit](scars-controls-verification.md).

```cpp
s::State<std::string> amount{"25"};
s::State<s::TableSelection> selected{s::TableSelection{{}, {{"record", "amount"}}}};
s::State<std::vector<s::TableColumnWidth>> widths;
s::State<std::vector<s::SortDescriptor>> sort;
s::TableModel table({{"record", "Observation"}}, {
    {.id = "name", .title = "Name", .sortable = true},
    {.id = "amount", .title = "Amount", .width = 120,
     .text = [&](const s::Option&, std::size_t) { return amount.get(); },
     .validate = [](const std::string&, const std::string& draft) {
         return draft.empty() ? "Enter an amount." : std::string{};
     },
     .commit = [&](const std::string&, std::string value) { amount.set(std::move(value)); }}
});

// body():
s::TableView(selected.binding(), table, {
    .selection_mode = s::TableSelectionMode::cells,
    .multiple_selection = true,
    .widths = widths.binding(), .sort = sort.binding(),
    .on_sort = [&](const auto& descriptors) { request_sorted_snapshot(descriptors); }
});
```

Retain TableModel in the application model. It indexes unique row and column IDs once; replace it to change membership, order, or schema. Text callbacks run only for realized cells or explicit logical queries and must be pure. The first column defaults to the row's label; other columns default to empty text. Optional `content` callbacks provide custom read-only display views. Fixed-height rows mount only around the viewport and overscan, with an additional active-editor row. The header stays fixed vertically and shares the body's horizontal offset. The Lab's `--render-workbench` preview combines SplitView, TreeView, TableView, SearchField, and context menus over 1,000 records.

The default selection mode is rows; `TableSelection::rows` holds stable row IDs. Cell mode uses `cells`, each with row and column IDs. A selection event writes the configured mode and clears the other vector. Enable multiple selection for Command-click toggles, Shift ranges/rectangles, and Command-A. Disabled rows cannot be selected by user input. Up/Down and Page/Home/End navigation retain the active column; Left/Right move between columns. Logical focus and selection are separate. `host->table_cell(tableID, {rowID, columnID})` provides metadata and optional actual bounds; `reveal_cell` reveals without selecting, and `focus(cellID)` / `activate(cellID)` operate on the returned stable ID.

Use `table_cell(cellID)` to refresh a held cell, `table_cell_at(rowID, columnIndex)` to index a row, and `table_column(tableID, columnIndex)` / `table_column(columnID)` to query column metadata. `table_column_cell(columnID, rowIndex)` reaches the same stable cell through its column. Indices follow the current model order; IDs survive reorder and become unavailable after removal. Offscreen cells have no fabricated bounds. Column metadata includes header identity, dimensions, sort direction/priority, and current row count.

`set_selected(itemID, bool)` adds or removes a logical selection while honoring single/multiple selection. `set_selection(tableID, itemIDs)` accepts the logical row IDs or cell IDs appropriate to the configured selection mode, validates the whole request, and writes the binding once. Unknown or disabled entries reject the batch without a partial write. Use IDs returned by logical queries rather than manually escaping row/column keys.

Columns declare initial, minimum, and maximum widths, with a minimum of at least 24 logical pixels. Handles support captured pointer dragging, Left/Right keyboard adjustment, Home/End bounds, and `Host::adjust` / `set_value`. Escape restores the drag's initial width; release/cancellation ends capture. Optional bound width overrides persist in the application model, and clearing the overrides restores schema defaults. Without a binding, the host retains resized widths until unmounting.

Sortable headings cycle ascending, descending, and no sort. Shift-click composes descriptors. The sort binding is updated before `on_sort` runs; the application then supplies a reordered/filtered TableModel. TableView does not sort business data internally. Stable row/column IDs preserve cell identity across model replacement. Projected sort metadata ignores unknown columns and duplicate descriptors, retaining the first descriptor per column and contiguous priorities, without rewriting the application binding during view construction.

Providing a column's `commit(rowID, string)` enables its standard text editor. Enter/F2 or a double-click starts editing. Text, selection, clipboard, undo, and IME use the shared input client; draft changes do not write application data. Validation runs on commit and returns an error string, or empty for acceptance. A changed external cell value rejects the stale draft instead of overwriting the update. Escape cancels, and blur cancels by default. Enter commits a single-line cell; a multiline column accepts newlines and commits with Command-Enter. Composition completes or cancels before the surrounding cell action. Tab commits and moves to the next enabled editable cell, Shift-Tab reverses, invalid drafts retain focus, and traversal re-resolves IDs after commit callbacks can change data. At table boundaries, Tab continues to other controls. Hiding/removing an edited row or column cancels its draft. `edit_cell`, `commit_table_edit`, and `cancel_table_edit` expose these actions to application code.

`edit_cell(cellID)` starts the same standard editor as keyboard activation. `set_cell_text(cellID, text)` opens that transaction, replaces its text, and validates/commits it. Rejected input retains the draft and validation message; active marked composition prevents replacement. These methods return success/failure and do not bypass validation or conflict checks.

Inspector table metadata shows effective widths, sort descriptors, active/editing cells, selection mode, and validation feedback. macOS accessibility provides lazy row/column/cell collections, indexed cell lookup, selected/visible cells, header relationships, sorting metadata, selection setters, and Confirm/value-set editing. Logical queries do not mount every row. The native table protocol assertions pass, including held objects after schema removal/window closure, alongside the full foreground/rendering and scheduler gates in both configurations.

## Alerts and confirmations

Retain one `Alert` handle for each presentation transaction. Its content is an arbitrary SCARS view, and its actions have stable IDs:

```cpp
s::Alert alert("Apply changes?", s::TextField("Name", draft_name.binding()), {
    {"cancel", "Cancel", s::AlertActionRole::cancel},
    {"apply", "Apply", s::AlertActionRole::default_action},
    {"delete", "Delete", s::AlertActionRole::destructive}
}, [](std::optional<std::string> action) {
    // An action ID, or nullopt for dismissal without choosing an action.
});
// An embedded view: return alert from a Host builder.
```

Pass a content builder instead of a View when labels, styles, or conditional children depend on state: `Alert("Review", [&] { return Label(status.get()); }, actions, completion)`. It runs during normal host construction and observes state without replacing the transaction. The same overload is available on `AlertPresentation::present`. Builders must be pure; bindings in an already constructed View remain live, while its computed text/style properties are snapshots.

Titles, action IDs, and labels must be nonempty, IDs must be unique, and at most one action may have each default/cancel role. Invalid definitions throw `invalid_argument`. Normal and destructive actions have no implicit keyboard shortcut. Return selects the explicit default when focus is not on another button; a focused button keeps its ordinary activation behavior. Escape selects the explicit enabled cancel action. An alert without that action ignores Escape. Disabled actions cannot complete it.

Composition, table-editor actions, menus, popovers, and field-specific submission/cancellation take priority. Return in multiline content inserts a newline; Return/Escape complete or cancel marked text before acting on the alert. Held Return cannot immediately accept after composition. Content bindings follow their ordinary editing policies: use application-owned drafts if Cancel must roll back form changes.

`choose(id)`, `cancel()`, and `dismiss()` return whether they completed the transaction. `cancel()` chooses the configured cancel action, while `dismiss()` returns no action ID. `completed()` distinguishes dismissal from a pending transaction; `result()` returns the selected ID. Copies share the same terminal transaction. Completion runs once, after terminal state is set, and may safely change presentation state. Completed content becomes disabled. Creating a fresh handle starts a new transaction. An embedded Alert supplies content and semantics; present it as a native sheet to impose modality.

For macOS, retain the returned presentation handle until completion:

```cpp
import scars.macos;
std::unique_ptr<s::macos::AlertPresentation> presentation;
auto shown = s::macos::AlertPresentation::present(*window, "Apply changes?",
    s::TextField("Name", draft_name.binding()), {
        {"cancel", "Cancel", s::AlertActionRole::cancel},
        {"apply", "Apply", s::AlertActionRole::default_action}
    }, [](auto action) { /* Handle the selected ID or dismissal. */ }, theme);
if (!shown)
    throw std::runtime_error(shown.error().message);
presentation = std::move(*shown);
```

`AlertPresentation` owns a native Dialog sheet and an independent SCARS Host. It blocks input to its parent while other windows remain interactive. Initial focus enters the first editable field, then the default action or another enabled non-destructive action. Native close requests, parent closure, explicit dismissal, and handle destruction complete without an action ID. AppKit sheets normally suppress traffic-light close controls. An existing sheet or closed parent produces a presentation error without calling completion. A completion callback may immediately present a replacement sheet.

`presentation->choose(id)` uses standard control activation, including composition completion and enabled/scope checks. `dismiss()` closes without choosing an action; `is_open()` reports presentation lifetime. `host()` exposes the independent host for inspection or theme changes. Keep the Application alive for the presentation lifetime. All operations run on the main thread.

`macos::confirm(parent, title, message, callback, theme, options)` is the two-action helper; its callback receives `true` only for acceptance and `false` for cancellation or dismissal. `ConfirmationOptions` supplies `accept_label`, `cancel_label`, and `destructive`. A destructive confirmation has no implicit affirmative default. Retain its returned handle in the same way.

`macos::mount` and `unmount` accept the shared native `Surface` interface, including Window and Dialog, while preserving existing Window call sites. The inspector identifies Alert transactions and action roles. Accessibility exposes a dialog group, default/cancel button relationships, action help, and parent blocking. The Lab includes review/reset sheets and isolated `--render-alert` previews.

## Date and time

DatePicker and TimePicker use portable Gregorian/ICU values and the standard SCARS draft editor. DateTimePicker has local and explicitly zoned overloads with a shared component draft.

```cpp
s::State<s::Date> date{s::Date{2026, 11, 2}};
s::State<s::Time> time{s::Time{14, 30, 15}};
s::State<s::LocalDateTime> departure{s::LocalDateTime{{2026, 11, 2}, {14, 30, 0}}};

s::DatePicker("Date", date.binding(), {
    .min = {2020, 1, 1}, .max = {2030, 12, 31}, .locale = "de_DE"
});
s::TimePicker("Time", time.binding(), {
    .hour_cycle = s::HourCycle::h12, .show_seconds = true, .minute_step = 5
});
s::DateTimePicker("Departure", departure.binding());
```

`Date` supports years 1–9999 in the proleptic Gregorian calendar. `Time` represents hours 0–23 and minutes/seconds 0–59; it has no date, offset, leap second, or midnight rollover. `LocalDateTime` combines these values without inferring the machine time zone. `valid` checks intrinsic validity. Invalid application values and malformed configuration throw `invalid_argument`; intrinsically valid values outside a picker's limits produce validation feedback.

The default locale is `en_US_POSIX`. Dates use locale-specific numeric order with a four-digit year. The calendar remains Gregorian even in locales with another default calendar. Time defaults to 24-hour display without seconds. `locale`, `hour_cycle`, and `show_seconds` affect presentation. `format_date` / `parse_date` and `format_time` / `parse_time` expose the same civil formatting and syntax parsing; the picker enforces its range and custom validators separately.

Typing changes a host-owned draft. Enter validates and commits, Escape restores the model value, and blur commits valid drafts by default; `commit_on_blur = false` defers that commit. Invalid drafts retain feedback and do not overwrite the model. `DateTimeValidation` uses the shared valid/incomplete/invalid/out-of-range statuses. `Host::field_validation(id)` and `commit_field(id)` operate on numeric and temporal fields. Format/locale changes or external source replacements retire stale drafts. Untouched hidden seconds survive formatting and stepping; a changed typed time without seconds commits at the displayed precision.

Up/Down adjusts the date or time component at the caret. Time's inline steppers use the same component, with held-button repeat; minute/second steps default to 1. AM/PM stepping toggles the period. Typed out-of-range values are rejected; stepping stops at bounds. Read-only fields retain text selection while disabling mutation and calendar/stepper actions.

DatePicker's trailing button or Option-Down opens an anchored calendar. Arrows navigate days/weeks; Home/End reach the locale's week boundaries; Page Up/Down changes month, and Shift changes year. Header controls also navigate months and years. Enter or Space accepts a day; Escape or outside dismissal keeps the existing selection. `disabled_date` and `validate` apply to typing, stepping, and calendar selection and must be pure. The popover scrolls in constrained windows and closes when its owner is hidden, disabled, removed, or replaced. Native text input retains the shared IME, candidate geometry, clipboard, and cursor behavior. Return/Escape handle marked text before submission/cancellation.

For an actual instant, bind `chrono::sys_seconds` and provide an IANA zone:

```cpp
s::State<std::chrono::sys_seconds> instant{
    *s::resolve_datetime({{2026, 11, 2}, {14, 30, 0}}, "UTC").instant
};
s::DateTimePicker("Arrival", instant.binding(), {
    .time_zone = "America/New_York",
    .repeated_time = s::RepeatedTimePolicy::reject
});
```

`local_datetime(instant, zone)` converts an instant to displayed civil values. `resolve_datetime(local, zone, policy)` returns an instant or an explanatory error. Unknown zones are configuration errors. Nonexistent wall times are rejected. Repeated wall times are rejected by default; `earlier` and `later` explicitly choose an occurrence. An unchanged existing instant keeps its known offset, including during an overlap. The pinned ICU time-zone data supplies conversion; no platform date-layout API or implicit host-zone conversion is involved.

`DateTimePickerOptions` supplies date/time options, combined civil `min`/`max`, and a pure `validate(LocalDateTime)`. `ZonedDateTimePickerOptions` supplies the named zone, overlap policy, optional instant limits, and `display` options.

The combined picker stages both components in one host-owned transaction. Moving between its date and time fields can retain an intermediate value outside the joint limits. The application binding receives only a valid combined value, once both fields are valid and no component text remains pending. Enter collects both drafts and attempts one publication; ordinary component blur follows `commit_on_blur`. For example, limits from November 2 at 22:00 through November 3 at 02:00 allow changing November 2 at 23:00 to November 3 at 01:00 without publishing either invalid intermediate value. View construction never publishes a draft.

`Host::field_validation(id)` accepts the combined picker or either component and reports prospective joint validation, including uncommitted text. `commit_field(id)` collects both fields atomically; `cancel_field(id)` restores both. Component `numeric_validation` and `on_validation` describe that individual editor. Escape first handles marked text, then a dirty component editor, then the staged joint draft. Leaving the group, window deactivation, hiding, disabling, collapsing, or unmounting discards unfinished joint edits. Changes to the model, locale/display format, zone, or bounds retire stale drafts; distinct overlap instants also reset drafts when their displayed wall times match. Rejected setters retain validation feedback until explicit retry or cancellation; accepted application normalization is read back into both components.

The Lab includes locale/hour-cycle controls, local, zoned, and bounded overnight fields, and `--render-datetime` previews with an open calendar. Inspector snapshots identify temporal fields, calendar state/day values, and validation.

## Color controls

```cpp
s::State<s::Color> paint{0x804488cc}; // Packed, unpremultiplied AARRGGBB.
s::ColorWell("Paint", paint.binding());
s::ColorPicker("Paint editor", paint.binding(), {
    .alpha = true, .live_preview = false, .read_only = false,
    .model = s::ColorModel::rgb
});
```

ColorWell is a compact swatch/button that opens an anchored picker. ColorPicker mounts the same editor inline. Both support RGB/HSV modes, component sliders and numeric fields, hexadecimal text, optional alpha, and a checkerboard transparency preview. The mode is host-owned presentation state; switching modes collects valid text and retains the draft. Invalid text blocks the switch. Sliders support the standard arrow, Home/End, Page Up/Down, pointer, and accessibility operations; fields retain ordinary text selection, cursor, clipboard, undo, and composition behavior. Hue is retained while editing gray/black so increasing saturation can recover the chosen hue.

By default, edits update one host-owned draft. Enter commits the focused component into that draft. **Apply** validates all pending component text and publishes the combined color once; **Cancel** discards it. `Host::color_validation(id)`, `apply_color(id)`, and `cancel_color(id)` accept the owner or a component ID. Validation includes invalid/incomplete text and marked composition; Apply refuses to interrupt composition. Return/Escape handle marked text before component actions. Escape restores a dirty component first, then cancels the session. An inline draft survives ordinary focus changes until Apply/Cancel. A well's outside dismissal consumes the click and cancels its session; shared popover focus, clipping, fitting, scrolling, and restoration rules apply.

Set `live_preview = true` explicitly to publish validated component edits and slider changes before Apply. The session retains its original source and last owned preview. Cancel requests restoration through the application binding only while the current model still equals that owned preview. An external value change replaces the source/draft and drops rollback ownership, preventing stale overwrites. Apply finalizes the preview. Hide, disable, collapse, removal, window deactivation/closure, and host destruction retire unfinished sessions and request rollback of owned previews. Cleanup setters run outside view construction. Application rejection of a commit retains feedback; explicit Apply can retry. Accepted normalization is read back into the editor. Configuration changes retire drafts; theme changes preserve them. `read_only` preserves text selection while disabling controls that change color. With `alpha = false`, editing preserves the source alpha rather than forcing opacity, and explicit alpha hex input is rejected.

Colors are 8-bit encoded sRGB channels with straight alpha; no linear-light conversion, wide-gamut metadata, or native color panel is inferred. HSV conversion operates on those encoded RGB channels. Public `HSV` uses hue in degrees [0, 360] and saturation/value in [0, 1]; `color_hsv(Color)` and `color_from_hsv(HSV, alphaByte)` convert independently of platform APIs. Invalid/non-finite HSV inputs throw. HSV-to-RGB rounds to the nearest channel byte. Achromatic conversion returns hue zero; the interactive picker retains hue intent separately.

`format_color(color)` returns CSS `#RRGGBBAA`; `format_color(color, false)` returns `#RRGGBB`. `parse_color(text)` accepts 3/4/6/8 RGB(A) hex digits with an optional `#` and surrounding ASCII whitespace. Missing alpha means opaque; short forms expand nibbles. For example, packed `0x804488cc` formats as `#4488CC80`, not `#804488CC`. Fully transparent values retain their RGB bytes. Skia handles premultiplication during rendering; application colors remain straight ARGB.

The Lab includes inline, swatch, live-preview, and alpha examples plus `--render-colors` previews. Inspector snapshots show source/draft, RGB/HSV mode, alpha/publication policy, popup state, and validation. Accessibility exposes the same standard button/group, slider, numeric/text, preview image, and Apply/Cancel semantics without a platform-specific color layout.

## Theme and developer tools

Use `s::Theme{}` for dark or `s::Theme::light()` for light, then `host->set_theme(theme)`. Preserve `font_path` when replacing a custom-font theme. Theme changes retain view state, field selection, and undo history.

SCARS Lab has an upper-right **native title-bar** light-mode button and **View → Light mode** (Command-Shift-L). For another app, install `window->set_titlebar_theme_toggle(initialLight, callback)`; in the callback apply the host theme, call `window->set_appearance(...)`, and update `set_titlebar_theme_toggle_state(...)`. This accessory remains outside the client area.

Retain `s::macos::DeveloperTools(application, host, theme)` and connect a menu shortcut to `toggle()`. The Lab uses Command-Option-I. The explorer provides picking, live layout/style/state, temporary property edits, box-model and flex overlays, position/z-index/visibility controls, and resets. It follows the source host's theme.
