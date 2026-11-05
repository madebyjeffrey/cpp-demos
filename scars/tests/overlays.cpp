#include "include/core/SkCanvas.h"
#include "include/core/SkSurface.h"
#include <algorithm>
#include <chrono>
#include <functional>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>
import scars;
namespace s = scars;
using namespace std::chrono_literals;
namespace {
int checks{};
void require(bool condition, const char* message) {
    ++checks;
    if (!condition) throw std::runtime_error(message);
}
s::Theme theme(bool light = false) {
    auto t = light ? s::Theme::light() : s::Theme{};
    t.font_path = SCARS_FONT_PATH;
    return t;
}
s::InspectionNode keyed(s::Host& h, const std::string& key) {
    for (const auto& n : h.inspect()) if (n.key == key) return n;
    throw std::runtime_error("Missing key: " + key);
}
std::vector<s::InspectionNode> overlays(s::Host& h, const std::string& type) {
    std::vector<s::InspectionNode> result;
    for (const auto& n : h.inspect()) if (n.type == type) result.push_back(n);
    return result;
}
s::Element element(s::Host& h, const std::string& id) {
    for (const auto& e : h.elements()) if (e.id == id) return e;
    throw std::runtime_error("Missing element");
}
s::Point center(s::Rect r) { return {r.x + r.width / 2, r.y + r.height / 2}; }
void pointer(s::Host& h, s::PointerKind kind, s::Point p) { h.handle(s::PointerEvent{kind, p}); }
void click(s::Host& h, s::Point p) { pointer(h, s::PointerKind::down, p); pointer(h, s::PointerKind::up, p); }
void key(s::Host& h, s::Key k, bool shift = false) { h.handle(s::KeyEvent{k, true, false, shift}); }
bool fits(s::Rect r, s::Size v) {
    return r.x >= 0 && r.y >= 0 && r.width >= 0 && r.height >= 0 &&
           r.x + r.width <= v.width + .001 && r.y + r.height <= v.height + .001;
}
void tooltips() {
    auto now = std::chrono::steady_clock::time_point{};
    s::State<bool> hidden{false}, enabled{true}, mounted{true};
    s::State<std::string> help{"A longer explanation that wraps within its configured width."};
    int clicks{};
    s::Host h([&] {
        return s::Block({s::Button("Help", [&] { ++clicks; }).key("help").hidden(hidden.get()).enabled(enabled.get())
            .visible(mounted.get()).tooltip(help.get(), {.delay = 100ms, .max_width = 180}),
            s::Button("Next", [] {}).key("next")}).padding(12).gap(16);
    }, {}, theme(), {}, {.clock = [&] { return now; }});
    h.layout({320, 200});
    auto anchor = keyed(h, "help");
    require(element(h, anchor.id).help == help.get(), "Tooltip help is accessible before presentation");
    pointer(h, s::PointerKind::move, center(anchor.bounds));
    require(h.next_wake() == 100ms && overlays(h, "Tooltip").empty(), "Hover schedules a single delayed wake");
    now += 99ms; h.advance_time();
    require(overlays(h, "Tooltip").empty() && h.next_wake() == 1ms, "Tooltip respects its deadline");
    now += 1ms; h.advance_time();
    auto tips = overlays(h, "Tooltip");
    require(tips.size() == 1 && fits(tips[0].bounds, {320, 200}) && tips[0].bounds.width <= 180,
            "Tooltip wraps and fits its viewport");
    require(tips[0].bounds.height > 30 && !h.next_wake() && !h.needs_frame(), "Shown tooltip requires no continuous frames");
    require(tips[0].overlay_scope == tips[0].id && !tips[0].interactive, "Tooltip has inspectable scope without a focus stop");
    click(h, center(anchor.bounds));
    require(clicks == 1 && overlays(h, "Tooltip").empty(), "Tooltip does not intercept its anchor's click");
    pointer(h, s::PointerKind::move, center(anchor.bounds));
    require(!h.next_wake(), "Click suppresses repeated tooltip presentation until pointer leaves");
    pointer(h, s::PointerKind::move, {310, 190});
    pointer(h, s::PointerKind::move, center(anchor.bounds));
    require(h.next_wake() == 100ms, "Reentering starts a fresh tooltip delay");
    pointer(h, s::PointerKind::leave, {0, 0});
    require(!h.next_wake(), "Pointer leave cancels pending presentation");
    h.focus(anchor.id);
    require(h.next_wake() == 100ms, "Keyboard focus schedules tooltip help");
    now += 100ms; h.advance_time();
    key(h, s::Key::escape);
    require(overlays(h, "Tooltip").empty() && element(h, anchor.id).focused, "Escape hides tooltip and preserves focus");
    key(h, s::Key::tab);
    require(element(h, keyed(h, "next").id).focused, "Tooltip adds no Tab stop");
    h.focus(anchor.id); hidden.set(true); h.advance_time();
    require(!h.next_wake() && overlays(h, "Tooltip").empty(), "Hidden owner cancels tooltip");
    hidden.set(false); h.layout({320, 200}); h.focus(anchor.id);
    enabled.set(false); h.advance_time();
    require(!h.next_wake(), "Disabled owner cancels tooltip");
    enabled.set(true); h.layout({320, 200}); h.focus(anchor.id);
    help.set(""); h.advance_time();
    require(!h.next_wake() && element(h, anchor.id).help.empty(), "Removing tooltip text removes accessible help and deadline");
    help.set("Restored"); h.layout({320, 200}); h.focus(anchor.id); mounted.set(false); h.advance_time();
    require(!h.next_wake() && overlays(h, "Tooltip").empty(), "Removed owner cancels tooltip");
}
void popovers() {
    s::State<bool> open{false}, nested{false}, hidden{false}, mounted{true}, enabled{true};
    s::State<std::string> text{"draft"}, selected{"one"};
    int body_builds{}, dismissed{}, nested_dismissed{}, behind{};
    s::Host h([&] {
        return s::Block({s::Button("Anchor", [&] { open.set(!open.get()); }).key("anchor")
            .visible(mounted.get()).hidden(hidden.get()).enabled(enabled.get())
            .popover(open.binding(), [&] {
                ++body_builds;
                return s::Block({s::TextField("Content", text.binding()).key("editor"),
                    s::SelectionDropdown(selected.binding(), {{"one", "One"}, {"two", "Two"}}).key("menu"),
                    s::Button("Nested", [&] { nested.set(true); }).key("nested")
                        .popover(nested.binding(), [&] { return s::Button("Child", [] {}).key("child"); },
                                 {.width = 170, .on_dismiss = [&] { ++nested_dismissed; }}),
                    s::Button("Close", [&] { open.set(false); }).key("close")}).gap(10);
            }, {.width = 260, .max_height = 220, .on_dismiss = [&] { ++dismissed; }}),
            s::Button("Behind", [&] { ++behind; }).key("behind")}).padding(12).gap(16);
    }, {}, theme());
    h.layout({600, 420});
    auto anchor = keyed(h, "anchor");
    require(body_builds == 0 && overlays(h, "Popover").empty(), "Closed popovers never invoke their builders");
    h.focus(anchor.id); h.activate(anchor.id); h.layout({600, 420});
    auto editor = keyed(h, "editor");
    auto pop = overlays(h, "Popover");
    require(pop.size() == 1 && pop[0].bounds.y >= anchor.bounds.y + anchor.bounds.height,
            "Popover defaults to placement below its anchor");
    require(pop[0].parent_id == anchor.id && editor.overlay_scope == pop[0].id,
            "Overlay bodies remain in the inspector hierarchy with a separate layout scope");
    require(element(h, editor.id).focused && element(h, pop[0].id).role == s::Role::group,
            "Content policy focuses the first control and exposes a semantic group");
    require(!h.focus(anchor.id) && !h.activate(keyed(h, "behind").id), "Content focus scope blocks programmatic underlying activation");
    h.text_command(s::TextCommand::select_all); h.insert_text("edited");
    require(text.get() == "edited", "Popover editors use normal bindings and editing");
    key(h, s::Key::tab, true);
    require(element(h, keyed(h, "close").id).focused, "Reverse Tab wraps within popover content");
    key(h, s::Key::tab);
    require(element(h, editor.id).focused, "Tab wraps back to first overlay control");
    h.activate(keyed(h, "menu").id);
    require(h.popup_open(), "Existing selection menus open within popovers");
    key(h, s::Key::down); key(h, s::Key::enter);
    require(selected.get() == "two" && open.get() && !h.popup_open(), "Menu acceptance leaves its parent popover open");
    h.focus(keyed(h, "nested").id); h.activate(keyed(h, "nested").id); h.layout({600, 420});
    require(overlays(h, "Popover").size() == 2 && element(h, keyed(h, "child").id).focused,
            "Nested popovers form a topmost focus scope");
    key(h, s::Key::escape);
    require(open.get() && !nested.get() && nested_dismissed == 1 && overlays(h, "Popover").size() == 1,
            "Escape dismisses only the topmost popover exactly once");
    require(element(h, keyed(h, "nested").id).focused, "Nested dismissal restores the invoking control");
    h.focus(editor.id); h.mark_text("candidate", {0, 9});
    key(h, s::Key::escape);
    require(open.get() && !h.text_input()->marked, "Composition cancellation precedes popover Escape");
    key(h, s::Key::escape);
    require(!open.get() && overlays(h, "Popover").empty() && dismissed == 1 && element(h, anchor.id).focused,
            "Popover Escape restores anchor focus and invokes dismissal once");
    open.set(true); h.layout({600, 420});
    click(h, center(keyed(h, "behind").bounds));
    require(!open.get() && behind == 0 && dismissed == 2, "Outside dismissal consumes the entire click");
    open.set(true); h.layout({600, 420}); hidden.set(true); h.layout({600, 420});
    require(!open.get() && dismissed == 3 && overlays(h, "Popover").empty(), "Hiding the owner clears presentation outside construction");
    hidden.set(false); open.set(true); h.layout({600, 420}); enabled.set(false); h.layout({600, 420});
    require(!open.get() && dismissed == 4, "Disabling the owner dismisses its popover");
    enabled.set(true); open.set(true); h.layout({600, 420}); nested.set(true); h.layout({600, 420});
    key(h, s::Key::escape); open.set(false); h.layout({600, 420});
    require(!nested.get() && dismissed == 5, "External presentation changes perform normal dismissal");
    open.set(true); h.layout({600, 420}); mounted.set(false); h.layout({600, 420});
    require(!open.get() && dismissed == 6 && overlays(h, "Popover").empty(), "Removing the anchor closes its presentation");
    mounted.set(true); open.set(true); h.layout({600, 420}); h.blur();
    require(!open.get() && dismissed == 7 && overlays(h, "Popover").empty(), "Window blur clears presentation without refocusing an inactive host");
}
void geometry_and_policy() {
    for (const bool light : {false, true}) for (const auto placement : {s::OverlayPlacement::below, s::OverlayPlacement::above,
            s::OverlayPlacement::left, s::OverlayPlacement::right}) {
        s::State<bool> open{true};
        s::Host h([&] { return s::Layer({s::Button("Edge", [] {}).key("anchor")
            .position(s::Position::absolute).right(4).bottom(4)
            .popover(open.binding(), [] { return s::Block({s::Label("A wide body that wraps"), s::Button("Inside", [] {})}); },
                {.placement = placement, .width = 300, .max_height = 200})}); }, {}, theme(light));
        h.layout({240, 140});
        auto body = overlays(h, "Popover").at(0);
        require(fits(body.bounds, {240, 140}) && body.bounds.x >= 8 && body.bounds.y >= 8,
                "All placements flip or clamp within window margins in both themes");
        auto image = SkSurfaces::Raster(SkImageInfo::MakeN32Premul(240, 140));
        h.paint(*image->getCanvas(), 0);
        h.layout({40, 30});
        require(fits(overlays(h, "Popover").at(0).bounds, {40, 30}), "Resize remeasures and fits even tiny viewports");
    }
    s::State<bool> open{false}; s::State<std::string> value{"keep"};
    s::Host h([&] { return s::Block({s::TextField("Owner", value.binding()).key("owner")
        .popover(open.binding(), [] { return s::Button("Suggestion", [] {}).key("suggestion"); },
                 {.focus = s::PopoverFocus::preserve, .dismiss_on_outside = false, .dismiss_on_escape = false}),
        s::Button("Other", [] {}).key("other")}); }, {}, theme());
    h.layout({500, 300}); h.focus(keyed(h, "owner").id); open.set(true); h.layout({500, 300});
    require(element(h, keyed(h, "owner").id).focused, "Preserve policy keeps editor focus while content is presented");
    h.insert_text("new"); require(value.get() == "new", "Preserved editing continues through the main input bridge");
    key(h, s::Key::escape); require(open.get(), "Escape dismissal is configurable");
    click(h, {490, 290}); require(open.get(), "Outside dismissal is configurable and still consumes input");
    click(h, center(keyed(h, "suggestion").bounds));
    require(element(h, keyed(h, "suggestion").id).focused, "Preserve policy allows explicitly focusing content");
    open.set(false); h.layout({500, 300});
    require(element(h, keyed(h, "owner").id).focused, "Closing preserve-mode content restores its prior editor");
}
void clipping_and_lifecycle() {
    s::State<bool> open{true}, nested{false};
    s::State<std::string> outside_query{""}, inside_query{""};
    int dismissed{};
    s::Host h([&] {
        return s::Block({s::SearchField("Outside", outside_query.binding()).key("outside"),
            s::ScrollView(s::Block({
                s::Button("Anchor", [] {}).key("anchor").popover(open.binding(), [&] {
                    std::vector<s::View> controls{s::SearchField("Inside", inside_query.binding()).key("inside")};
                    for (int i = 0; i < 8; ++i) controls.push_back(s::Button("Item " + std::to_string(i), [] {}).key("item" + std::to_string(i)));
                    controls.push_back(s::Button("Nested", [&] { nested.set(true); }).key("nested")
                        .popover(nested.binding(), [] { return s::Button("Child", [] {}); }));
                    return s::Block(std::move(controls)).gap(8);
                }, {.width = 240, .max_height = 160, .on_dismiss = [&] { ++dismissed; }}),
                s::Spacer{}.height(600), s::Button("Below", [] {}).key("below")
            }).gap(8)).height(90).key("scroller")}).gap(10).padding(12);
    }, {}, theme());
    h.layout({500, 400});
    auto anchor = keyed(h, "anchor");
    auto scroller = keyed(h, "scroller");
    auto body = overlays(h, "Popover").at(0);
    require(body.bounds.y + body.bounds.height > scroller.bounds.y + scroller.bounds.height &&
            body.clip.height == body.bounds.height, "Popover escapes its anchor's scroll clipping");
    require(!element(h, keyed(h, "outside").id).enabled && !h.set_text(keyed(h, "outside").id, "blocked"),
            "Modal scope also protects accessibility value mutations beneath it");
    h.focus(keyed(h, "item7").id);
    require(keyed(h, "anchor").bounds.y == anchor.bounds.y, "Revealing overlay controls never scrolls the anchor's ancestors");
    require(h.focus_search() && element(h, keyed(h, "inside").id).focused,
            "Search shortcut resolves within the active overlay scope");
    h.focus(keyed(h, "nested").id); h.activate(keyed(h, "nested").id); h.layout({500, 400});
    require(nested.get() && overlays(h, "Popover").size() == 2, "Nested content can be anchored inside a scrolled popover");
    open.set(false); h.layout({500, 400});
    require(!nested.get() && dismissed == 1 && overlays(h, "Popover").empty(),
            "Closing a parent clears descendant presentation bindings");
    open.set(true); h.layout({500, 400});
    require(overlays(h, "Popover").size() == 1, "Reopening parent does not resurrect dismissed descendants");
    h.reveal(keyed(h, "below").id);
    require(!open.get() && dismissed == 2 && overlays(h, "Popover").empty(), "Fully clipped anchors dismiss popovers");
    require(keyed(h, "anchor").clip.height == 0, "Clipped-owner dismissal preserves the user's scroll position");
    s::State<bool> presented{false}, fail{false};
    s::Host throwing([&] {
        return s::Button("Safe anchor", [] {}).key("safe").popover(presented.binding(), [&]() -> s::View {
            if (fail.get()) throw std::runtime_error("builder failure");
            return s::Button("Safe body", [] {}).key("body");
        });
    }, {}, theme());
    throwing.layout({320, 220}); presented.set(true); throwing.layout({320, 220});
    fail.set(true);
    bool caught{};
    try { throwing.layout({320, 220}); } catch (const std::runtime_error&) { caught = true; }
    require(caught && presented.get(), "Failed content construction preserves presentation state");
    fail.set(false); throwing.layout({320, 220});
    require(overlays(throwing, "Popover").size() == 1, "Failed overlay construction can recover on the next rebuild");
    s::State<bool> illegal{true};
    s::Host pure([&] { return s::Button("Pure", [] {}).popover(illegal.binding(), [&] {
        illegal.set(false); return s::Label("Invalid");
    }); }, {}, theme());
    caught = false;
    try { pure.layout({320, 220}); } catch (const std::logic_error&) { caught = true; }
    require(caught && illegal.get(), "Popover builders share the normal prohibition on state writes");
}
std::vector<s::Element> menu_items(s::Host& h) {
    std::vector<s::Element> result;
    for (const auto& e : h.elements()) if (e.role == s::Role::menu_item) result.push_back(e);
    return result;
}
s::Element menu_item(s::Host& h, const std::string& label) {
    for (const auto& e : menu_items(h)) if (e.label == label) return e;
    throw std::runtime_error("Missing menu item: " + label);
}
void context_menus() {
    s::State<bool> checked{false}, hidden{false}, enabled{true}, mounted{true}, available{true};
    s::State<std::string> mode{"a"};
    int actions{}, ordinary{}, tail{}, deep{};
    s::Host h([&] {
        auto menu = [&] {
            return std::vector<s::MenuItem>{
                {"run", "Run", [&] { ++actions; }, available.get(), "⌘R"},
                s::MenuItem::separator("divider"),
                {"disabled", "Disabled", [&] { ++actions; }, false},
                s::MenuItem::check("checked", "Grid", checked.binding(), "⌘G"),
                s::MenuItem::submenu("mode", "Mode", {
                    s::MenuItem::radio("a", "Alpha", mode.binding(), "a"),
                    s::MenuItem::radio("b", "Beta", mode.binding(), "b"),
                    s::MenuItem::submenu("more", "More", {{"deep/key", "Deep action", [&] { ++deep; }}})
                }),
                s::MenuItem::submenu("empty", "Empty", {}),
                {"tail", "Tail", [&] { ++tail; }}
            };
        };
        return s::Block({s::Panel(s::Block({s::Label("Target"), s::Button("Child", [&] { ++ordinary; }).key("child")}))
                .key("target").visible(mounted.get()).hidden(hidden.get()).enabled(enabled.get()).context_menu(menu),
            s::Button("Behind", [&] { ++ordinary; }).key("behind")}).padding(12).gap(12);
    }, {}, theme());
    h.layout({640, 480});
    const auto target = keyed(h, "target").id, child = keyed(h, "child").id;
    h.focus(child);
    require(!h.handle(s::KeyEvent{s::Key::f10}) && !h.popup_open(), "Context shortcut requires Shift-F10");
    key(h, s::Key::f10, true);
    require(h.popup_open() && overlays(h, "ContextMenu").size() == 1, "Shift-F10 invokes the nearest attached ancestor");
    require(menu_item(h, "Run").focused && menu_item(h, "Run").shortcut == "⌘R", "Menu highlights first enabled action and exposes its shortcut label");
    require(element(h, target).role == s::Role::group && !element(h, child).enabled,
            "Container anchors have group semantics and menu scope blocks underlying actions");
    require(!h.activate(child) && !h.focus(child) && h.cursor(center(keyed(h, "child").bounds)) == s::Cursor::arrow,
            "Underlying activation and focus do not bypass an active context menu");
    require(menu_items(h).size() == 6 && !menu_item(h, "Disabled").enabled && !menu_item(h, "Empty").enabled,
            "Separators add no accessible action and disabled or empty submenus cannot activate");
    require(!h.activate(menu_item(h, "Disabled").id), "Disabled context action is inert");
    key(h, s::Key::enter);
    require(actions == 1 && ordinary == 0 && !h.popup_open() && element(h, child).focused,
            "Acceptance runs exactly once, suppresses the ordinary click, and restores prior focus");
    auto p = center(keyed(h, "child").bounds);
    s::PointerEvent control_click{s::PointerKind::down, p}; control_click.control = true;
    h.handle(control_click); pointer(h, s::PointerKind::up, p);
    require(h.popup_open() && actions == 1 && ordinary == 0, "Control-click opens a menu and consumes its release");
    key(h, s::Key::up);
    require(menu_item(h, "Tail").focused, "Up wraps to the last selectable menu item");
    key(h, s::Key::home); key(h, s::Key::down);
    require(menu_item(h, "Grid").focused, "Keyboard navigation skips separators and disabled items");
    key(h, s::Key::space);
    require(checked.get() && !h.popup_open(), "Check menu items write their binding and close");
    require(h.show_context_menu(target), "Context menus can also be invoked explicitly");
    require(menu_item(h, "Grid").selected && menu_item(h, "Grid").value_text == "Checked", "Checked state is exposed through accessibility");
    checked.set(false);
    require(h.needs_frame() && !menu_item(h, "Grid").selected, "Menu check bindings participate in reactive state collection");
    h.focus(menu_item(h, "Mode").id); key(h, s::Key::right);
    require(overlays(h, "ContextMenu").size() == 2 && menu_item(h, "Mode").expanded && menu_item(h, "Alpha").selected,
            "Right opens a submenu with current radio selection and a parent relationship");
    const auto beta = menu_item(h, "Beta");
    require(element(h, beta.group).role == s::Role::menu && element(h, beta.group).group == menu_item(h, "Mode").id,
            "Submenus have accessible menu roles beneath their invoking item");
    key(h, s::Key::down); key(h, s::Key::enter);
    require(mode.get() == "b" && !h.popup_open(), "Radio acceptance updates its stable selected value and dismisses the chain");
    h.show_context_menu(target); h.activate(menu_item(h, "Mode").id); h.activate(menu_item(h, "More").id);
    require(overlays(h, "ContextMenu").size() == 3 && menu_item(h, "Deep action").id.find("deep%2Fkey") != std::string::npos,
            "Deep submenus have escaped stable item paths");
    key(h, s::Key::escape);
    require(overlays(h, "ContextMenu").size() == 2 && h.popup_open(), "Escape closes only the deepest submenu");
    key(h, s::Key::left);
    require(overlays(h, "ContextMenu").size() == 1 && menu_item(h, "Mode").focused, "Left returns to the parent menu highlight");
    h.handle(s::KeyEvent{s::Key::unknown, true, false, false, false, false, false, "t"});
    require(menu_item(h, "Tail").focused, "Printable menu typeahead moves to a matching enabled item");
    key(h, s::Key::tab);
    require(!h.popup_open() && element(h, keyed(h, "behind").id).focused, "Tab dismisses the chain and resumes normal focus order");
    pointer(h, s::PointerKind::down, p); pointer(h, s::PointerKind::up, p);
    require(ordinary == 1 && !h.popup_open(), "Ordinary clicks retain normal control behavior");
    s::PointerEvent right_click{s::PointerKind::down, p}; right_click.button = 1;
    h.handle(right_click);
    require(h.popup_open(), "Right-click invokes a context menu on an attached ancestor");
    pointer(h, s::PointerKind::move, center(menu_item(h, "Mode").bounds));
    require(overlays(h, "ContextMenu").size() == 2, "Pointer hover opens submenu content");
    pointer(h, s::PointerKind::move, center(menu_item(h, "Tail").bounds));
    require(overlays(h, "ContextMenu").size() == 1, "Hovering a different parent item closes its former submenu");
    click(h, center(menu_item(h, "Tail").bounds));
    require(tail == 1 && !h.popup_open(), "Pointer acceptance invokes the matching pressed item exactly once");
    h.show_context_menu(target); click(h, {630, 470});
    require(!h.popup_open() && ordinary == 1, "Outside dismissal consumes the entire click");
    h.show_context_menu(target); available.set(false); h.layout({640, 480});
    require(!menu_item(h, "Run").enabled && menu_item(h, "Grid").focused, "Reactive disabling relocates an invalid highlight");
    hidden.set(true); h.layout({640, 480});
    require(!h.popup_open(), "Hiding the owner dismisses all its context menus");
    hidden.set(false); h.layout({640, 480}); h.show_context_menu(target); enabled.set(false); h.layout({640, 480});
    require(!h.popup_open(), "Disabling the owner dismisses context menus");
    enabled.set(true); h.layout({640, 480}); h.show_context_menu(target); mounted.set(false); h.layout({640, 480});
    require(!h.popup_open(), "Removing the owner dismisses context menus");
    mounted.set(true); h.layout({640, 480}); h.show_context_menu(target); h.blur();
    require(!h.popup_open(), "Window blur dismisses context menus");
}
void context_geometry_and_editing() {
    for (const bool light : {false, true}) {
        s::Host h([] {
            std::vector<s::MenuItem> items;
            for (int i = 0; i < 24; ++i) items.push_back({std::to_string(i), "Command " + std::to_string(i), [] {}});
            items.push_back(s::MenuItem::submenu("nested", "Nested", {{"action", "Nested action", [] {}}}));
            return s::Layer({s::Label("Edge").key("edge").position(s::Position::absolute).right(4).bottom(4)
                .context_menu(std::move(items))});
        }, {}, theme(light));
        h.layout({240, 140});
        require(h.show_context_menu(keyed(h, "edge").id, s::Point{238, 138}) && fits(h.popup_bounds(), {240, 140}),
                "Context menus on noninteractive labels fit the viewport in both themes");
        key(h, s::Key::end);
        const auto last = menu_item(h, "Nested");
        require(last.focused && last.bounds.y >= last.clip.y && last.bounds.y + last.bounds.height <= last.clip.y + last.clip.height + .001,
                "Keyboard End reveals the highlighted row in a long menu");
        key(h, s::Key::right);
        const auto menus = overlays(h, "ContextMenu");
        require(menus.size() == 2 && fits(menus[0].bounds, {240, 140}) && fits(menus[1].bounds, {240, 140}),
                "Submenus flip and clamp at a screen edge");
        auto raster = SkSurfaces::Raster(SkImageInfo::MakeN32Premul(480, 280));
        raster->getCanvas()->scale(2, 2); h.paint(*raster->getCanvas(), 0);
        require(!h.needs_frame() && !h.next_wake(), "Static context menus settle after a scaled raster frame");
        h.layout({40, 30});
        require(fits(h.popup_bounds(), {40, 30}), "Resize safely fits a context menu into a tiny viewport");
    }
    s::State<bool> visible{true}, nested_enabled{true}, parent_open{false};
    s::State<std::string> text{"original"};
    int invoked{};
    s::Host h([&] {
        return s::Block({s::Button("Popover", [&] { parent_open.set(true); }).key("anchor")
            .popover(parent_open.binding(), [&] {
                auto submenu = s::MenuItem::submenu("sub", "Submenu", {{"run", "Run in popover", [&] { ++invoked; }}});
                submenu.enabled = nested_enabled.get();
                return s::TextField("Inside", text.binding()).key("editor").context_menu({std::move(submenu)});
            }), s::Button("Another", [] {}).visible(visible.get())}).padding(12);
    }, {}, theme());
    h.layout({500, 320}); h.focus(keyed(h, "anchor").id); h.activate(keyed(h, "anchor").id); h.layout({500, 320});
    const auto editor = keyed(h, "editor").id;
    h.select_text(editor, {0, 0});
    h.mark_text("海", {0, 3});
    key(h, s::Key::f10, true);
    require(h.popup_open() && !h.text_input() && text.get() == "海original", "Context invocation commits composition and suspends text-input routing");
    h.activate(menu_item(h, "Submenu").id);
    require(overlays(h, "ContextMenu").size() == 2 && parent_open.get(), "Context submenus coexist with the enclosing popover");
    nested_enabled.set(false); h.layout({500, 320});
    require(overlays(h, "ContextMenu").size() == 1 && !menu_item(h, "Submenu").enabled,
            "Disabling a submenu removes its open descendants");
    key(h, s::Key::enter);
    require(h.popup_open() && invoked == 0, "A menu with no selectable items stays inert");
    key(h, s::Key::escape);
    require(parent_open.get() && !h.popup_open() && element(h, editor).focused && h.text_input().has_value(),
            "Context Escape restores its editor without closing the parent popover");
    nested_enabled.set(true); h.layout({500, 320}); h.show_context_menu(editor); h.activate(menu_item(h, "Submenu").id);
    h.activate(menu_item(h, "Run in popover").id);
    require(invoked == 1 && parent_open.get() && !h.popup_open(), "Nested context acceptance closes menus while preserving the enclosing popover");
    h.show_context_menu(editor); parent_open.set(false); h.layout({500, 320});
    require(!h.popup_open() && overlays(h, "ContextMenu").empty(), "Removing a popover also removes its context-menu stack");
    s::State<bool> bad{false};
    s::Host duplicate([] { return s::Label("Invalid").context_menu({{"same", "First", [] {}}, {"same", "Second", [] {}}}); }, {}, theme());
    bool caught{};
    try { duplicate.layout({320, 240}); } catch (const std::invalid_argument&) { caught = true; }
    require(caught, "Duplicate sibling context-menu IDs are rejected");
    s::Host pure([&] { return s::Label("Pure").context_menu([&] { bad.set(true); return std::vector<s::MenuItem>{}; }); }, {}, theme());
    caught = false;
    try { pure.layout({320, 240}); } catch (const std::logic_error&) { caught = true; }
    require(caught && !bad.get(), "Context-menu builders follow normal state-write restrictions");
}
void context_identity() {
    s::State<bool> reverse{false}, include_submenu{true}, mounted{true};
    s::Host h([&] {
        return s::Label("Dynamic").key("dynamic").visible(mounted.get()).context_menu([&] {
            std::vector<s::MenuItem> entries{{"a", "First", [] {}}, {"b", "Second", [] {}}};
            if (include_submenu.get()) {
                std::vector<s::MenuItem> children{{"x", "Child first", [] {}}, {"y", "Child second", [] {}}};
                if (reverse.get()) std::reverse(children.begin(), children.end());
                entries.push_back(s::MenuItem::submenu("sub", "Dynamic submenu", std::move(children)));
            }
            entries.push_back({"remove", "Remove owner", [&] { mounted.set(false); }});
            if (reverse.get()) std::reverse(entries.begin(), entries.end());
            return entries;
        });
    }, {}, theme());
    h.layout({480, 400}); const auto id = keyed(h, "dynamic").id;
    h.show_context_menu(id); const auto second = menu_item(h, "Second").id; h.focus(second);
    reverse.set(true);
    require(menu_item(h, "Second").id == second && menu_item(h, "Second").focused,
            "Reordering a menu preserves stable item identity and highlight");
    h.activate(menu_item(h, "Dynamic submenu").id);
    const auto child = menu_item(h, "Child second").id; h.focus(child); reverse.set(false);
    require(menu_item(h, "Child second").id == child && menu_item(h, "Child second").focused && overlays(h, "ContextMenu").size() == 2,
            "An open submenu also retains identity and highlight through reordering");
    include_submenu.set(false); h.layout({480, 400});
    require(overlays(h, "ContextMenu").size() == 1, "Removing a submenu closes its mounted descendant menus");
    h.activate(menu_item(h, "Remove owner").id); h.layout({480, 400});
    require(!mounted.get() && !h.popup_open() && menu_items(h).empty(), "Context callbacks can remove their owner after the chain is dismissed");
}
void tabbed_popover_focus() {
    s::State<std::string> tab{"a"}, value{"Initial"}; s::State<bool> open{false};
    s::Host h([&] { return s::Block({s::Button("Before tabs", [] {}), s::TabView(tab.binding(), {
        {"a", "First", [&] { return s::Button("Open", [&] { open.set(true); }).key("anchor")
            .popover(open.binding(), [&] { return s::TextField("Value", value.binding()).key("editor"); }); }},
        {"b", "Second", [] { return s::Label("Other page"); }}
    }).height(180)}); }, {}, theme());
    h.layout({500, 300}); const auto anchor = keyed(h, "anchor").id;
    h.focus(anchor); h.activate(anchor); h.layout({500, 300});
    h.set_text(keyed(h, "editor").id, "Edited"); key(h, s::Key::escape);
    require(!open.get() && element(h, anchor).focused, "Popover dismissal inside an active tab restores its invoking control");
    h.activate(anchor); h.layout({500, 300}); tab.set("b"); h.layout({500, 300});
    bool active_tab_focused{};
    for (const auto& e : h.elements()) if (e.role == s::Role::tab && e.label == "Second") active_tab_focused = e.focused;
    require(!open.get() && active_tab_focused, "Switching a popover's page still restores focus to the newly selected tab");
}
}
int main() {
    try { tooltips(); popovers(); geometry_and_policy(); clipping_and_lifecycle(); context_menus(); context_geometry_and_editing(); context_identity(); tabbed_popover_focus(); std::cout << checks << " overlay checks passed\n"; }
    catch (const std::exception& e) { std::cerr << e.what() << '\n'; return 1; }
}
