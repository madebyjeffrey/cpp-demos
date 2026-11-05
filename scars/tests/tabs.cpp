#include "include/core/SkBitmap.h"
#include "include/core/SkCanvas.h"
#include "include/core/SkSurface.h"
#include <algorithm>
#include <cmath>
#include <functional>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>
import scars;
namespace s = scars;
namespace {
int checks{};
void require(bool condition, const char* message) {
    ++checks;
    if (!condition) throw std::runtime_error(message);
}
void throws(const std::function<void()>& fn, const char* message) {
    bool caught = false;
    try { fn(); } catch (const std::exception&) { caught = true; }
    require(caught, message);
}
bool near(double a, double b) { return std::abs(a - b) < .1; }
s::Theme theme(bool light = false) {
    auto result = light ? s::Theme::light() : s::Theme{};
    result.font_path = SCARS_FONT_PATH;
    return result;
}
s::InspectionNode keyed(s::Host& host, const std::string& key) {
    for (const auto& node : host.inspect()) if (node.key == key) return node;
    throw std::runtime_error("Missing key: " + key);
}
s::Element tab(s::Host& host, const std::string& label) {
    for (const auto& element : host.elements())
        if (element.role == s::Role::tab && element.label == label) return element;
    throw std::runtime_error("Missing tab: " + label);
}
void key(s::Host& host, s::Key value, bool shift = false, bool control = false) {
    host.handle(s::KeyEvent{value, true, false, shift, false, false, control});
}
SkBitmap pixels(s::Host& host, bool light = false, int scale = 1) {
    host.set_theme(theme(light));
    host.layout({420, 240});
    auto surface = SkSurfaces::Raster(SkImageInfo::MakeN32Premul(420 * scale, 240 * scale));
    surface->getCanvas()->scale(scale, scale);
    host.paint(*surface->getCanvas());
    SkBitmap result;
    result.allocN32Pixels(420 * scale, 240 * scale);
    require(surface->readPixels(result, 0, 0), "Tab preview pixels can be read");
    return result;
}
void lifecycle() {
    s::State<std::string> selected{"a"}, text{"Alpha"};
    s::State<std::vector<std::string>> order{{"a", "disabled", "b", "c"}};
    s::State<bool> disable_b{false}, disable_all{false}, hidden{false}, mounted{true};
    int builds_a = 0, builds_b = 0, builds_c = 0, disabled_builds = 0;
    s::Host host([&]() -> s::View {
        if (!mounted.get()) return {};
        std::vector<s::Tab> items;
        for (const auto& id : order.get()) {
            if (id == "a") items.push_back({id, "Alpha", [&] {
                ++builds_a;
                std::vector<s::View> rows{s::TextField("", text.binding()).key("field-a").height(40)};
                for (int i = 0; i < 20; ++i)
                    rows.push_back(s::Label("Row " + std::to_string(i)).height(30).key("row-" + std::to_string(i)));
                return s::ScrollView(s::Block(std::move(rows))).key("scroll-a");
            }, !disable_all.get()});
            if (id == "b") items.push_back({id, "Beta", [&] {
                ++builds_b;
                return s::Button("Beta action", [] {}).key("button-b");
            }, !disable_b.get() && !disable_all.get()});
            if (id == "c") items.push_back({id, "Activity", [&] {
                ++builds_c;
                return s::ProgressBar{}.key("activity");
            }, !disable_all.get()});
            if (id == "disabled") items.push_back({id, "Unavailable", [&] {
                ++disabled_builds;
                return s::Label("Never mounted");
            }, false});
        }
        return s::TabView(selected.binding(), std::move(items)).key("pages")
            .accessible_label("Test pages").hidden(hidden.get());
    }, {}, theme());
    host.layout({420, 240});
    const auto a = tab(host, "Alpha").id, b = tab(host, "Beta").id, c = tab(host, "Activity").id;
    const auto field = keyed(host, "field-a").id;
    require(builds_a == 1 && builds_b == 0 && builds_c == 0 && disabled_builds == 0,
            "Only the initially active page builder runs");
    require(keyed(host, "pages").tabs->visited_count == 1 && keyed(host, "pages").tabs->tab_count == 4,
            "Inspector tracks logical tabs separately from visited pages");
    require(tab(host, "Alpha").selected && !tab(host, "Unavailable").enabled,
            "Tab semantics expose selection and disabled states");
    const auto initial_elements = host.elements();
    const auto field_element = std::find_if(initial_elements.begin(), initial_elements.end(), [&](const auto& e) { return e.id == field; });
    require(field_element != initial_elements.end() && field_element->group.ends_with("/page/a"),
            "Active page controls belong to their accessible page group");
    const auto group = std::find_if(initial_elements.begin(), initial_elements.end(), [](const auto& e) { return e.role == s::Role::tab_group; });
    require(group != initial_elements.end() && group->label == "Test pages" && tab(host, "Alpha").group == group->id,
            "Tabs belong to an accessible tab group");
    require(!host.activate(tab(host, "Unavailable").id) && disabled_builds == 0,
            "Disabled tab actions never construct their page");
    host.focus(a);
    key(host, s::Key::tab);
    require(host.text_input().has_value(), "The selected tab is one Tab stop before its active page");
    key(host, s::Key::tab, true);
    require(tab(host, "Alpha").focused, "Shift-Tab returns from the page to its selected tab");
    host.select_text(field, {5, 5});
    host.insert_text("!");
    host.mark_text("仮", {3, 0});
    require(text.get() == "Alpha!" && host.text_input()->marked.has_value(),
            "Marked page text remains uncommitted before switching");
    require(host.activate(b) && selected.get() == "b" && text.get() == "Alpha!仮",
            "Selecting another page commits active input-method composition");
    require(builds_b > 0 && keyed(host, "field-a").hidden && near(keyed(host, "field-a").bounds.height, 0),
            "Visited inactive pages remain mounted without layout allocation");
    const auto beta_elements = host.elements();
    require(std::none_of(beta_elements.begin(), beta_elements.end(), [&](const auto& e) { return e.id == field; }) &&
                !host.focus(field) && !host.set_text(field, "hidden edit"),
            "Inactive page controls are excluded from accessibility and editing");
    const auto inactive_builds = builds_a;
    host.activate(c);
    host.layout({420, 240});
    require(host.needs_frame(), "The active page may animate");
    host.activate(b);
    host.layout({420, 240});
    require(!host.needs_frame() && builds_a == inactive_builds,
            "Inactive page builders and animations stay idle");
    host.activate(a);
    require(keyed(host, "field-a").id == field && host.focus(field) && host.text_command(s::TextCommand::undo) && text.get() == "Alpha!",
            "Revisiting a page retains editor identity and composition undo history");
    host.activate(a);
    host.handle({s::PointerKind::scroll, {20, 100}, 0, 0, -180});
    const auto scrolled = keyed(host, "row-0").bounds.y;
    host.activate(b);
    host.activate(a);
    require(scrolled < 0 && near(keyed(host, "row-0").bounds.y, scrolled),
            "Revisiting a tab retains its scroll position");
    order.set({"c", "b", "disabled", "a"});
    require(keyed(host, "field-a").id == field && near(keyed(host, "row-0").bounds.y, scrolled),
            "Reordering tabs preserves keyed page identity and scroll state");
    order.set({"a", "disabled", "b", "c"});
    host.focus(a);
    key(host, s::Key::right);
    require(selected.get() == "b" && tab(host, "Beta").focused, "Right skips disabled tabs and activates the next tab");
    key(host, s::Key::right);
    require(selected.get() == "c", "Arrow navigation follows declaration order");
    key(host, s::Key::right);
    require(selected.get() == "a", "Arrow navigation wraps through enabled tabs");
    key(host, s::Key::end);
    require(selected.get() == "c", "End activates the last enabled tab");
    key(host, s::Key::home);
    require(selected.get() == "a", "Home activates the first enabled tab");
    host.focus(field);
    key(host, s::Key::tab, false, true);
    require(selected.get() == "b", "Control-Tab cycles tabs from a page editor");
    key(host, s::Key::tab, true, true);
    require(selected.get() == "a", "Control-Shift-Tab cycles backward");
    selected.set("b");
    disable_b.set(true);
    require(keyed(host, "pages").selection == "a" && selected.get() == "b" &&
                keyed(host, "pages").tabs->requested_selection == "b",
            "Invalid or disabled selections use a visible fallback without mutating the binding");
    host.activate(a);
    require(selected.get() == "a", "Activating the fallback commits a valid selection through the normal event path");
    host.focus(field);
    const auto committed = text.get();
    host.mark_text("未確定", {9, 0});
    selected.set("c");
    host.layout({420, 240});
    selected.set("a");
    require(host.focus(field) && text.get() == committed && !host.text_input()->marked,
            "Programmatic page changes cancel uncommitted composition using the normal hidden-control policy");
    host.focus(field);
    order.set({"disabled", "b", "c"});
    require(keyed(host, "pages").selection == "c" && tab(host, "Activity").focused && selected.get() == "a",
            "Removing the selected page moves focus to its enabled fallback");
    order.set({"a", "disabled", "b", "c"});
    selected.set("a");
    require(host.focus(keyed(host, "field-a").id) && !host.text_input()->can_undo && text.get() == "Alpha!",
            "Removing and readding a tab clears host-owned history while preserving model values");
    hidden.set(true);
    host.layout({420, 240});
    const auto hidden_builds = builds_a;
    text.set("External update");
    host.layout({420, 240});
    require(host.elements().empty() && !host.needs_frame() && builds_a == hidden_builds,
            "A hidden tab view does not rebuild page content or animate");
    hidden.set(false);
    require(host.focus(keyed(host, "field-a").id) && host.text_input()->text == "External update",
            "Showing a retained page refreshes it from the current model");
    disable_all.set(true);
    require(keyed(host, "pages").selection.empty() && selected.get() == "a" && !host.text_input(),
            "A tab view with no enabled tabs has no active page");
    order.set({});
    require(keyed(host, "pages").tabs->tab_count == 0 && !host.needs_frame(),
            "An empty tab view is valid and remains idle");
    order.set({"a", "b"}); disable_all.set(false); disable_b.set(false);
    host.layout({420, 240});
    mounted.set(false);
    require(host.inspect().empty(), "Unmounting removes the entire retained page tree");
    mounted.set(true);
    require(keyed(host, "pages").tabs->visited_count == 1,
            "Remounting starts a fresh lazy page cache");
}
void nested_and_overflow() {
    s::State<std::string> outer{"one"}, inner{"first"}, text{"value"};
    s::Host nested([&] {
        return s::TabView(outer.binding(), {
            {"one", "Outer one", [&] { return s::TabView(inner.binding(), {
                {"first", "Inner first", [&] { return s::TextField("", text.binding()).key("nested-editor"); }},
                {"second", "Inner second", [] { return s::Label("Inner content"); }}
            }); }},
            {"two", "Outer two", [] { return s::Label("Outer content"); }}
        });
    }, {}, theme());
    nested.layout({420, 240});
    nested.focus(keyed(nested, "nested-editor").id);
    key(nested, s::Key::tab, false, true);
    require(inner.get() == "second" && outer.get() == "one", "Control-Tab targets the nearest containing tab view");
    nested.focus(tab(nested, "Outer one").id);
    key(nested, s::Key::tab, false, true);
    require(outer.get() == "two" && inner.get() == "second", "An outer tab strip cycles its own pages");
    key(nested, s::Key::tab, true, true);
    require(tab(nested, "Inner second").selected, "Nested tab state survives outer page changes");
    s::State<std::string> chosen{"0"};
    s::Host overflow([&] {
        std::vector<s::Tab> tabs;
        for (int i = 0; i < 10; ++i)
            tabs.push_back({std::to_string(i), "Long tab " + std::to_string(i), [] { return s::Label("Page"); }});
        return s::TabView(chosen.binding(), std::move(tabs)).key("overflow");
    }, {}, theme());
    overflow.layout({180, 180});
    overflow.focus(tab(overflow, "Long tab 9").id);
    auto last = tab(overflow, "Long tab 9");
    require(last.clip.width > 0 && last.bounds.x >= 0 && last.bounds.x + last.bounds.width <= 180.1 &&
                keyed(overflow, "overflow").tabs->strip_offset > 0,
            "Focusing an overflowing tab scrolls its header into view");
    key(overflow, s::Key::space);
    require(chosen.get() == "9", "An overflowing tab can be activated from the keyboard");
    overflow.layout({90, 180});
    last = tab(overflow, "Long tab 9");
    require(last.clip.width > 0 && last.bounds.x >= 0 && last.bounds.x + last.bounds.width <= 90.1,
            "Resizing keeps the focused tab visible");
    key(overflow, s::Key::home);
    require(chosen.get() == "0" && near(keyed(overflow, "overflow").tabs->strip_offset, 0),
            "Home reveals the first tab in an overflowing strip");
    overflow.handle({s::PointerKind::scroll, {20, 20}, 0, -120, 0});
    require(keyed(overflow, "overflow").tabs->strip_offset > 0,
            "The tab strip accepts horizontal wheel scrolling");
    const auto elements = overflow.elements();
    const auto visible = std::find_if(elements.begin(), elements.end(), [](const auto& e) {
        return e.role == s::Role::tab && !e.selected && e.clip.width > 20;
    });
    require(visible != elements.end(), "An overflowing strip exposes visible tab hit regions");
    const s::Point click{visible->clip.x + visible->clip.width / 2, visible->clip.y + visible->clip.height / 2};
    overflow.handle({s::PointerKind::down, click});
    overflow.handle({s::PointerKind::up, click});
    require(chosen.get() != "0", "Pointer selection uses the scrolled header geometry");
    overflow.layout({0, 0});
    require(near(keyed(overflow, "overflow").bounds.width, 0), "Zero-sized tab viewports remain finite");
    s::Host outside([&] { return s::TextField("", text.binding()).key("outside"); }, {}, theme());
    outside.layout({300, 80});
    outside.focus(keyed(outside, "outside").id);
    require(!outside.handle(s::KeyEvent{s::Key::tab, true, false, false, false, false, true}),
            "Control-Tab has no tab-navigation effect outside a tab view");
    s::State<std::string> active{"first"}, committed{"text"};
    s::State<bool> available{true};
    s::Binding<std::string> editing{[&] { return committed.get(); }, [&](std::string value) {
        committed.set(std::move(value));
        available.set(false);
    }};
    s::Host validation([&] { return s::TabView(active.binding(), {
        {"first", "Editor", [&] { return s::TextField("", editing).key("commit-editor"); }},
        {"second", "Commit target", [] { return s::Label("Target"); }, available.get()}
    }); }, {}, theme());
    validation.layout({300, 160});
    validation.select_text(keyed(validation, "commit-editor").id, {4, 4});
    validation.mark_text("!", {1, 0});
    require(validation.activate(tab(validation, "Commit target").id) && active.get() == "first" &&
                !tab(validation, "Commit target").enabled && committed.get() == "text!",
            "Tab selection revalidates availability after committing an editor");
}
void rendering_and_validation() {
    s::State<std::string> selected{"red"};
    s::Host host([&] { return s::TabView(selected.binding(), {
        {"red", "Red", [] { return s::Panel(s::Spacer{}).background(0xffc02030); }},
        {"blue", "Blue", [] { return s::Panel(s::Spacer{}).background(0xff2040c0); }}
    }); }, {}, theme());
    const auto red = pixels(host);
    selected.set("blue");
    const auto blue = pixels(host);
    require(red.getColor(210, 120) == 0xffc02030 && blue.getColor(210, 120) == 0xff2040c0,
            "Only the active page contributes pixels");
    const auto light = pixels(host, true);
    require(light.getColor(20, 20) != blue.getColor(20, 20), "Tab headers use the current theme palette");
    const auto scaled = pixels(host, true, 2);
    require(scaled.getColor(420, 240) == light.getColor(210, 120), "Tab pages retain logical geometry at 2x scale");
    throws([&] { s::TabView(selected.binding(), {{"same", "A", [] { return s::View{}; }},
                                                 {"same", "B", [] { return s::View{}; }}}); },
            "Duplicate tab IDs are rejected");
    throws([&] { s::TabView(selected.binding(), {{"", "A", [] { return s::View{}; }}}); },
            "Empty tab IDs are rejected");
    throws([&] { s::TabView(selected.binding(), {{"missing", "Missing builder", {}}}); },
            "Missing page builders are rejected");
    s::State<int> value{0};
    s::Host invalid([&] { return s::TabView(selected.binding(), {
        {"blue", "Invalid", [&] { value.set(1); return s::View{}; }}
    }); }, {}, theme());
    throws([&] { invalid.layout({200, 100}); }, "Page builders cannot mutate observable state");
}
} // namespace
int main() {
    try {
        lifecycle();
        nested_and_overflow();
        rendering_and_validation();
        std::cout << "SCARS tabs: " << checks << " checks passed\n";
        return 0;
    } catch (const std::exception& error) {
        std::cerr << "SCARS tabs failed after " << checks << " checks: " << error.what() << '\n';
        return 1;
    }
}
