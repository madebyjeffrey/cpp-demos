# Declarative views, layout, controls and transitions

Include `<skia_window/ui.hpp>` and link the existing `skia_window::skia_window` target. The UI layer is C++23. It renders through Skia inside the same native `Window` and `Dialog` surfaces; it does not create an OS child window for every control.

## Define and mount a view

```cpp
#include <skia_window/ui.hpp>
#include <format>

namespace ui = skia_window::ui;
using namespace ui::literals;
using namespace std::chrono_literals;

struct SettingsView {
    ui::State<bool> enabled{true};
    ui::State<double> speed{1.0};
    ui::State<int> counter{0};

    ui::View body() {
        return ui::Column{
            ui::Text("Renderer settings").font_size(24_dp),
            ui::CheckBox("Animate", enabled.binding()),
            ui::Row{
                ui::Slider(speed.binding(), {.minimum = .1, .maximum = 4, .step = .1})
                    .grow(1).accessible_label("Animation speed"),
                ui::Text(std::format("{:.1f}x", speed.get())),
            }.gap(12_dp),
            ui::Button("Increment", [value = counter.binding()] {
                ui::animate(ui::Animation::ease_out(200ms), [&] {
                    value.set(value.get() + 1);
                });
            }),
            ui::Text(std::to_string(counter.get()))
                .content_transition(ui::ContentTransition::numeric_text),
        }.gap(14_dp).padding(20_dp).background(0xff10191f);
    }
};

// The native surface owns the host; the returned shared model is optional.
auto settings = window->set_view<SettingsView>();
// The identical operation is supported on a Dialog.
```

A `View` is an immutable description. Modifiers return another value, so copying a view does not allow one caller's styling to mutate another caller's description. The host keeps mounted identity, focus, scroll positions and transition resources separately from descriptions.

`set_view<T>(constructor_args...)` creates the root model once. `set_view_builder(function)` supports an externally owned model, and `set_content(view)` mounts static descriptions, including bound controls. `clear_content()` unmounts the UI and stops its animation scheduling. Mounting a new tree preserves the previous host if construction throws.

Existing `Callbacks::on_draw` paints underneath the UI. UI controls consume their pointer/key input before unhandled events reach `Callbacks::on_event`. Close/lifecycle callbacks continue working. `Surface::set_animating()` still controls the legacy draw loop; UI transitions and animated canvases schedule their own frames independently.

## State, composition and identity

`State<T>::get()` returns a value and registers a dependency while evaluating the view. `set()` invalidates dependent hosts, unless an equality-comparable value is unchanged. A copied `State` or `Binding` refers to the same cell. State and hosts are used on their owning thread; native surfaces use the main thread. Writes inside `body()` are rejected.

Updates are coalesced until layout/painting or the next input/accessibility query. The initial implementation rebuilds the affected host's descriptions; it does not promise independently scheduled updates for every nested subtree. Dependencies that disappear from the description stop invalidating that host.

Ordinary helper functions can return composed `View` values. For a nested model with persistent local state, use a keyed view model:

```cpp
struct Counter {
    ui::State<int> value{0};

    ui::View body(const std::string& title) {
        return ui::Row{
            ui::Text(title),
            ui::Button(std::to_string(value.get()), [binding = value.binding()] {
                binding.set(binding.get() + 1);
            }),
        }.gap(8_dp);
    }
};

return ui::component<Counter>("counter", std::string("Local state"));
```

The model is created once per parent/key/type identity; its `body(props)` receives fresh properties on updates. Removing it destroys its local state, even while an outgoing picture remains visible. Reintroducing the key creates fresh state. A no-properties `component<T>(key)` calls `body()`.

Use `when(condition, builder)` for a conditional structural slot. Its identity slot stays in the parent's child sequence when absent, while contributing no layout size or gap. Use `for_each(values, id_accessor, builder)` for dynamic lists; keys must be stable and unique among siblings. Keys preserve focus and scroll state during insertion/reordering. Duplicate keys are errors. Changing a `ContentSwitcher` selection unmounts the previous subtree; inactive panels are not cached automatically.

Callbacks should capture bindings, values or appropriately managed model references. Framework subscriptions use weak references; application-created asynchronous functions still need their own lifetime discipline.

## Auto layout

| Declaration | Behavior |
| --- | --- |
| `Row{...}` / `Column{...}` | Flex layout, growth/shrinkage, basis, gaps, wrapping, alignment and justification |
| `Grid(options, {...})` | Fixed, automatic, fractional and min/max tracks; row-major auto placement and explicit row/column spans |
| `Overlay{...}` | Layered children with alignment and offsets |
| `ScrollView(child)` | Vertical viewport; independent scroll position, clipping and focus reveal |
| `Container(layout, children)` | Explicit built-in or custom layout strategy |

Dimensions support logical units (`20_dp`), percentages (`50_pct`), font-relative lengths (`1.5_em`), `auto_size`, `min_content`, `max_content`, and unbounded maximums. Fractional units (`1_fr`) are grid track values and cannot be assigned to widget widths.

A widget's `BoxStyle` defines preferred/minimum/maximum dimensions, padding, margin and optional aspect ratio. Explicit dimensions include padding; margins do not collapse. Flex item settings, grid placement and overlay placement live in separate fields of `ItemStyle`, so responsive layout changes can retain each parent's applicable settings.

```cpp
ui::Container(ui::Flex{.direction = ui::Direction::column, .gap = 12_dp}, {
    sidebar(),
    content(),
}).responsive_layout(900_dp, ui::GridLayout{
    .columns = {260_dp, ui::minmax(0_dp, 1_fr)},
    .column_gap = 18_dp,
});
```

The breakpoint uses the container's available/assigned content width. The same children and their identities remain mounted. Text measurement is repeated at constrained widths, and row alignment supports text/control baselines. Constraints and scroll viewports determine overflow; content is not assigned persistent application-written coordinates.

This is a defined desktop layout subset inspired by the web. It does not implement the CSS cascade, arbitrary CSS parsing, named grid areas, subgrid, dense auto-placement or browser compatibility rules. Grid positions are zero-based. Fractional tracks have zero minimum unless specified with `minmax`; unresolved percentages contribute zero during unbounded intrinsic measurement and resolve when space becomes definite. Text supports UTF-8, newlines and basic wrapping, without full shaping/bidi typography.

For other algorithms, implement `LayoutAlgorithm::measure(context, constraints)` and `arrange(context, content_bounds)`, then pass `shared_ptr<const LayoutAlgorithm>` as the container's layout. `LayoutContext` exposes child styles, constrained child measurement and rectangle placement. Algorithms return content-box sizes; the host handles the container's padding and clipping. The custom strategy can be tested without a window or GPU.

## Controls, input and accessibility

Provided views: `Text`/`Label`, `Button`, `CheckBox`, `Slider`, `Image`, `Canvas`, `Spacer`, the containers above, and `ContentSwitcher`.

Controls support enabled state, hover/pressed appearance, pointer capture and keyboard focus. Tab/Shift-Tab traverse controls, Space/Enter activate buttons and checkboxes, and arrows/Home/End change sliders. Focus follows keyed controls when reordered and is moved into replacement content when its focused subtree disappears. Tab focus reveals controls inside scroll viewports. Nested scroll views consume scrolling from the innermost region that can move.

macOS exposes semantic text, buttons, checkboxes, sliders, images and labeled canvases through native accessibility elements. Labels, values, enabled/focused state, frames, press actions and slider adjustment/value setting are bridged to the UI host. Outgoing snapshots have no accessibility nodes. Parent-modal sheets disable the parent's accessibility actions as well as native input. This bridge has automated coverage; a full VoiceOver usability audit is separate from those tests.

`Canvas` receives a local, clipped `SkCanvas` and `PaintInfo` containing content size, scale and timing. `.animating(true)` requests display-paced painting. Its drawing commands may be recorded into a `SkPicture` for transitions and then replayed on the GPU; a canvas callback must not assume a non-null GPU recording context. Legacy surface-level `on_draw` still receives the actual GPU-backed canvas.

Text entry/IME, complex text shaping, dropdowns and list virtualization are not part of this first control set. No text field is simulated from raw key events.

## Transitions and animation

```cpp
ui::ContentSwitcher(selected.get(), [this](Panel panel) -> ui::View {
    return panel == Panel::editor ? editor() : preview();
})
    .transition(ui::Transition::push(ui::Edge::trailing)
        .combined_with(ui::Transition::fade()))
    .size_transition(ui::SizeTransition::animate);

ui::animate(ui::Animation::ease_in_out(250ms), [&] {
    selected.set(Panel::preview);
});
```

Use `content_transition(cross_fade)` for changing a label's content, and `numeric_text` for a vertically moving/fading numeric label. The first implementation moves the complete label rather than individual digits. `ContentSwitcher` replaces keyed subtrees. `Transition::fade`, `slide_from`, `push`, and `scale` compose with `combined_with`; `asymmetric(enter, leave)` gives each direction its own effect. Views with `.transition()` also animate insertion/removal inside a retained parent.

An animation transaction supplies duration and a linear, ease-out, ease-in-out or damped spring curve. Without a transaction, updates are immediate. `.animation(animation)` provides a local override for that description. Color, background, opacity and font size interpolate; `.animate_layout()` additionally interpolates a view's bounds during state-driven layout changes.

A switcher normally adopts its target layout immediately. `.size_transition(animate)` interpolates intrinsic size on automatic axes and reflows surrounding layout. Outgoing content is a frozen paint picture, not a second live control tree. Replacements retain no outgoing callbacks or input targets. Cross-fades blend premultiplied layers without a dark opacity dip; normal rendering has no CPU pixel readback.

Interrupted changes begin from the last displayed picture/properties/bounds. Hit testing and accessibility bounds follow the incoming presentation transform. Frames stop when transitions finish and no visible canvas requests animation. Hidden native windows pause rendering; completed timelines settle when shown again.

The native host supplies the platform Reduce Motion setting, and `Host::set_reduced_motion(true)` can additionally request reduced motion. An application override cannot disable the system preference. Reduced motion settles transitions immediately. Arbitrary custom shaders as replacement effects and matched-geometry transitions are future extensions.

## Demo and validation

Launching the normal app opens the original gallery and the Controls Lab. From the gallery, View → Controls Lab or Command-L reopens it. The lab demonstrates shared bindings, a nested stateful view, independent scroll viewports, responsive flex/grid layout, grid spans, overlays, a GPU canvas, keyed collection reordering, content replacement, numeric labels, reduced motion and a declarative modal sheet.

For an explicit offline layout preview (no window or GPU required):

```sh
build/relwithdebinfo/skia_window_demo.app/Contents/MacOS/skia_window_demo \
  --render-controls "$PWD/build/controls.png" 1220 0
```

The optional last arguments select width and panel (0 editor, 1 layout, 2 collection). This diagnostic uses a raster surface to write a PNG; it is not a fallback for the normal app's Metal presentation.

`skia_window.ui` tests layout, state, input, focus/scroll, nested identity, custom algorithms, transition pixels/interruption, properties and reduced motion. `skia_window.ui_gpu` repeats control/transition rendering on an actual offscreen Metal surface with pixel readback. `skia_window.ui_native_actions` exercises native accessibility, input consumption, modal bindings and mount/unmount lifetimes without waiting for a visible frame. `skia_window.ui_desktop` additionally checks visible window frames and transition-driven native frame scheduling.

Native/GPU tests need appropriate Metal and WindowServer access; a restricted sandbox may hide the device. To run tests that do not require visible windows, use `ctest --preset relwithdebinfo -LE desktop` (or the Debug preset). The standalone consumer now compiles and exercises declarative views using C++ source only.

Layout is reused between paint-only frames. Animating a canvas, a color, or a content cross-fade does not require remeasuring the tree. State updates, viewport changes, scrolling, and size/bounds animations invalidate the appropriate layout. Within a layout pass, repeated proposals for a subtree reuse its measurement; text advances are measured in bulk and reused for wrapping. Custom `LayoutAlgorithm` implementations must return deterministic results for their view descriptions and constraints. Change observed state or call `Host::invalidate()` when a non-observed external layout input changes.

AppKit resize/backing-scale changes draw the new content inside the sizing transaction. This keeps the Metal drawable aligned with native window bounds instead of stretching the previous frame while awaiting a display-link tick. Normal animation uses asynchronous Metal presentation; the resize path waits only for command scheduling, not GPU completion. See [Apple's transaction presentation contract](https://developer.apple.com/documentation/quartzcore/cametallayer/presentswithtransaction).

For a repeatable CPU diagnostic using the actual Controls Lab descriptions:

```sh
build/relwithdebinfo/skia_window_demo.app/Contents/MacOS/skia_window_demo --benchmark-controls
```

It reports median, 95th-percentile and maximum UI CPU times for steady animation, resizing, and resizing with transitions. A no-pixels canvas isolates body/layout/paint-recording work; these are not GPU execution times or end-to-end frame intervals. The command returns nonzero if any scenario exceeds a 4 ms 95th-percentile diagnostic budget. This machine-dependent timing threshold is deliberately outside CTest. `skia_window.ui_performance` instead asserts deterministic bounds on repeated measurement and paint-only layout work. `skia_window.resize` checks native transaction-aligned resize drawing, deduplicated resize notifications, and continuing animation in AppKit's event-tracking mode.
