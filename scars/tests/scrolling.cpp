#include "include/core/SkCanvas.h"
#include "include/core/SkSurface.h"
#include <algorithm>
#include <cmath>
#include <iostream>
#include <limits>
#include <stdexcept>
#include <string>
#include <vector>
import scars;
namespace s = scars;
namespace {
int checks{};
void require(bool ok, const char* message) { ++checks; if (!ok) throw std::runtime_error(message); }
bool near(double a, double b) { return std::abs(a - b) < .001; }
s::Theme theme(bool light = false) { auto t = light ? s::Theme::light() : s::Theme{}; t.font_path = SCARS_FONT_PATH; return t; }
s::InspectionNode keyed(s::Host& h, const std::string& key) {
    for (const auto& n : h.inspect()) if (n.key == key) return n;
    throw std::runtime_error("Missing key: " + key);
}
s::Point offset(s::Host& h, const std::string& key) { return *h.scroll_offset(keyed(h, key).id); }
bool wheel(s::Host& h, s::Point p, double x, double y, bool shift = false) {
    return h.handle(s::PointerEvent{s::PointerKind::scroll, p, 0, x, y, shift});
}
s::View document(double w, double h) { return s::Layer({s::Button("Far", [] {}).key("far").position(s::Position::absolute)
    .left(w - 100).top(h - 100).width(80).height(40)}).width(w).height(h).key("document"); }
void geometry() {
    for (const bool light : {false, true}) {
        s::State<s::ScrollAxes> axes{s::ScrollAxes::both}; s::State<bool> enabled{true}, hidden{false};
        s::Host h([&] { return s::ScrollView(document(900, 700), {.axes = axes.get()}).padding(10).key("scroll")
            .enabled(enabled.get()).hidden(hidden.get()); }, {}, theme(light));
        h.layout({320, 200}); const auto id = keyed(h, "scroll").id;
        const auto initial = *keyed(h, "scroll").scroll;
        require(initial.axes == s::ScrollAxes::both && near(initial.maximum.x, 600) && near(initial.maximum.y, 520), "Both axes use the padded content viewport for bounds");
        require(h.scroll_to(id, {100, 200}) && near(offset(h, "scroll").x, 100) && near(offset(h, "scroll").y, 200), "Programmatic offsets apply independently on each axis");
        require(near(keyed(h, "document").bounds.x, -90) && near(keyed(h, "document").bounds.y, -190), "Document geometry reflects both offsets");
        require(wheel(h, {15, 15}, -40, -50) && near(offset(h, "scroll").x, 140) && near(offset(h, "scroll").y, 250), "A diagonal wheel gesture moves both axes");
        require(wheel(h, {15, 15}, 0, -20, true) && near(offset(h, "scroll").x, 160) && near(offset(h, "scroll").y, 250), "Shift-wheel maps vertical wheel input onto horizontal scrolling");
        h.scroll_to(id, {1e9, 1e9});
        require(near(offset(h, "scroll").x, 600) && near(offset(h, "scroll").y, 520), "Large offsets clamp at document limits");
        require(!wheel(h, {15, 15}, -40, -50), "A fully bounded gesture reports no consumed movement");
        h.scroll_to(id, {-100, -200});
        require(near(offset(h, "scroll").x, 0) && near(offset(h, "scroll").y, 0), "Negative offsets clamp to the origin");
        h.focus(keyed(h, "far").id);
        require(near(offset(h, "scroll").x, 580) && near(offset(h, "scroll").y, 460) && near(keyed(h, "far").clip.width, 80), "Focus reveal moves both axes by the required amount");
        auto raster = SkSurfaces::Raster(SkImageInfo::MakeN32Premul(640, 400)); raster->getCanvas()->scale(2, 2); h.paint(*raster->getCanvas(), 0);
        h.layout({1000, 800});
        require(near(offset(h, "scroll").x, 0) && near(offset(h, "scroll").y, 0) && near(keyed(h, "scroll").scroll->maximum.x, 0), "Viewport growth clamps stale offsets and fits short documents");
        h.layout({320, 200}); h.scroll_to(id, {100, 200}); axes.set(s::ScrollAxes::vertical); h.layout({320, 200});
        require(near(offset(h, "scroll").x, 0) && near(offset(h, "scroll").y, 200), "Switching axes clears unsupported movement without resetting the supported axis");
        axes.set(s::ScrollAxes::horizontal); h.layout({320, 200});
        require(near(offset(h, "scroll").y, 0) && keyed(h, "scroll").scroll->maximum.x > 0, "Horizontal-only mode constrains the vertical dimension");
        enabled.set(false); h.layout({320, 200});
        require(!h.scroll_to(id, {20, 0}) && !wheel(h, {20, 20}, -20, 0), "Disabled scrollers reject direct and wheel input");
        enabled.set(true); hidden.set(true); h.layout({320, 200});
        require(!h.scroll_to(id, {20, 0}) && !wheel(h, {20, 20}, -20, 0), "Hidden scrollers reject scrolling input");
        hidden.set(false); h.layout({320, 200});
        require(!h.scroll_offset(keyed(h, "far").id) && !h.scroll_to(keyed(h, "far").id, {}), "Non-scrolling elements do not expose scroll operations");
        bool caught{};
        try { h.scroll_to(id, {std::numeric_limits<double>::quiet_NaN(), 0}); } catch (const std::invalid_argument&) { caught = true; }
        require(caught, "Non-finite offsets are rejected");
    }
}
void nested() {
    s::Host h([] { return s::ScrollView(s::Layer({
        s::ScrollView(s::Label("Inner").width(600).height(180), {.axes = s::ScrollAxes::both})
            .width(160).height(100).position(s::Position::absolute).left(20).top(20).key("inner")
    }).width(900).height(800), {.axes = s::ScrollAxes::both}).key("outer"); }, {}, theme());
    h.layout({400, 300});
    require(wheel(h, {30, 30}, -500, -150), "Nested diagonal movement is handled");
    require(near(offset(h, "inner").x, 440) && near(offset(h, "inner").y, 80), "The innermost viewport consumes movement up to each boundary");
    require(near(offset(h, "outer").x, 60) && near(offset(h, "outer").y, 70), "Only residual movement passes to the parent on each axis");
    wheel(h, {10, 10}, 500, 150);
    require(near(offset(h, "inner").x, 0) && near(offset(h, "inner").y, 0) && near(offset(h, "outer").x, 0) && near(offset(h, "outer").y, 0), "Reverse movement chains at the leading boundaries too");
    s::Host mixed([] { return s::ScrollView(s::Layer({
        s::ScrollView(s::Label("Horizontal").width(600), {.axes = s::ScrollAxes::horizontal})
            .width(200).height(100).position(s::Position::absolute).left(20).top(20).key("inner")
    }).height(800)).key("outer"); }, {}, theme());
    mixed.layout({400, 300}); wheel(mixed, {30, 30}, 0, -80);
    require(near(offset(mixed, "inner").x, 0) && near(offset(mixed, "outer").y, 80), "Unsupported vertical input passes through a horizontal scroller");
    wheel(mixed, {30, 10}, -50, 0);
    require(near(offset(mixed, "inner").x, 50) && near(offset(mixed, "outer").y, 80), "Horizontal movement stays within the horizontal child");
}
void reveal() {
    s::Host h([] { return s::ScrollView(s::Layer({
        s::ScrollView(document(600, 500), {.axes = s::ScrollAxes::both}).width(200).height(160)
            .position(s::Position::absolute).left(700).top(500).key("inner")
    }).width(1000).height(900), {.axes = s::ScrollAxes::both}).key("outer"); }, {}, theme());
    h.layout({320, 240}); h.focus(keyed(h, "far").id);
    const auto far = keyed(h, "far");
    require(near(offset(h, "inner").x, 380) && near(offset(h, "inner").y, 280), "Reveal resolves the inner viewport first");
    require(near(offset(h, "outer").x, 580) && near(offset(h, "outer").y, 420), "Reveal recomputes outer movement after inner geometry changes");
    require(near(far.clip.width, 80) && near(far.clip.height, 40), "Nested offscreen controls become fully visible");
}
void child_routing() {
    s::State<std::string> value{""}, tab{"a"};
    for (int i = 0; i < 30; ++i) value.set(value.get() + "Text line\n");
    s::Host h([&] { return s::ScrollView(s::Block({s::TextEditor(value.binding()).height(60).key("editor"), s::Spacer{}.height(700)})).key("outer"); }, {}, theme());
    h.layout({400, 300}); wheel(h, {20, 20}, 0, -2000);
    require(offset(h, "outer").y > 0, "Wheel movement left over after text-editor scrolling reaches its parent");
    s::Host tabs([&] { return s::ScrollView(s::Block({s::TabView(tab.binding(), {
        {"a", "A", [] { return s::Label("First"); }}, {"b", "B", [] { return s::Label("Second"); }}}).height(120),
        s::Spacer{}.height(700)})).key("outer"); }, {}, theme());
    tabs.layout({400, 300}); wheel(tabs, {10, 10}, 0, -40);
    require(near(offset(tabs, "outer").y, 40), "A fitting tab strip passes wheel input to its scrolling parent");
}
void lifecycle() {
    s::State<bool> scroller{true}, hidden{false}; s::State<double> extent{500};
    s::Host h([&]() -> s::View {
        if (!scroller.get()) return s::Label("Replacement");
        return s::ScrollView(s::Label("Content").width(extent.get()).height(extent.get()), {.axes = s::ScrollAxes::both})
            .hidden(hidden.get());
    }, {}, theme());
    h.layout({200, 180}); h.scroll_to("root", {150, 150}); hidden.set(true); h.layout({200, 180});
    require(near(h.scroll_offset("root")->x, 150) && near(h.scroll_offset("root")->y, 150), "Hidden scrollers retain their bounded position");
    hidden.set(false); extent.set(210); h.layout({200, 180});
    require(near(h.scroll_offset("root")->x, 10) && near(h.scroll_offset("root")->y, 30), "Document shrink clamps each stored coordinate");
    scroller.set(false); h.layout({200, 180}); scroller.set(true); h.layout({200, 180});
    require(near(h.scroll_offset("root")->x, 0) && near(h.scroll_offset("root")->y, 0), "Replacing a scroller's kind discards its former position");
    s::State<std::string> tab{"a"};
    s::Host tabs([&] { return s::TabView(tab.binding(), {
        {"a", "A", [] { return s::Label("A"); }},
        {"b", "B", [] { return s::ScrollView(s::Label("Lazy").height(1000)).key("lazy"); }}
    }); }, {}, theme());
    tabs.layout({200, 180}); tab.set("b"); tabs.layout({200, 180}); tab.set("a"); tabs.layout({200, 180});
    const auto lazy = keyed(tabs, "lazy");
    require(tabs.scroll_offset(lazy.id).has_value(), "Retained hidden tab scrollers can report their offset safely");
}
}
int main() {
    try { geometry(); nested(); reveal(); child_routing(); lifecycle(); std::cout << checks << " scrolling checks passed\n"; }
    catch (const std::exception& error) { std::cerr << error.what() << '\n'; return 1; }
}
