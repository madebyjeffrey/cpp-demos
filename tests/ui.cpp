#include <skia_window/ui.hpp>
#include "include/core/SkCanvas.h"
#include "include/core/SkBitmap.h"
#include "include/core/SkSurface.h"
#include "include/core/SkFont.h"
#include "include/core/SkFontMgr.h"
#include "include/ports/SkFontMgr_mac_ct.h"
#include <cmath>
#include <format>
#include <iostream>
#include <thread>
#include <source_location>

namespace ui = skia_window::ui;
namespace sw = skia_window;
using namespace ui::literals;
using namespace std::chrono_literals;
#define CHECK(x)                                                                                             \
    do {                                                                                                     \
        if (!(x))                                                                                            \
            throw std::runtime_error(std::format("Line {}: {}", __LINE__, #x));                              \
    } while (false)
void near(double a, double b, std::source_location where = std::source_location::current()) {
    if (std::abs(a - b) > .05)
        throw std::runtime_error(std::format("Line {}: expected {}, got {}", where.line(), b, a));
}
ui::Canvas box() {
    return ui::Canvas([](auto&, const auto&) {});
}
ui::ElementInfo find(ui::Host& host, std::string_view key) {
    for (auto n : host.elements())
        if (n.key == key)
            return n;
    throw std::runtime_error("Missing element: " + std::string(key));
}
void paint(ui::Host& host, double time, sw::Size size = {500, 400}) {
    auto surface = SkSurfaces::Raster(SkImageInfo::MakeN32Premul((int)size.width, (int)size.height));
    host.paint(*surface->getCanvas(), sw::FrameInfo{.logical_size = size,
                                                    .pixel_size = size,
                                                    .elapsed = std::chrono::duration<double>(time)});
}
void click(ui::Host& host, std::string_view key) {
    auto n = find(host, key);
    sw::Point p{n.bounds.x + n.bounds.width / 2, n.bounds.y + n.bounds.height / 2};
    CHECK(host.handle(sw::PointerEvent{.kind = sw::PointerKind::down, .position = p}));
    CHECK(host.handle(sw::PointerEvent{.kind = sw::PointerKind::up, .position = p}));
}
int main() {
    try {
        {
            ui::Host host([] {
                return ui::Row{box().key("fixed").width(100_dp).height(30_dp),
                               box().key("one").item({.flex = {.grow = 1, .basis = 0_dp}}),
                               box().key("two").item({.flex = {.grow = 2, .basis = 0_dp}})}
                    .gap(10_dp)
                    .padding(10_dp)
                    .align_items(ui::Align::center);
            });
            host.layout({620, 100});
            near(find(host, "fixed").bounds.x, 10);
            near(find(host, "fixed").bounds.y, 35);
            near(find(host, "one").bounds.width, 160);
            near(find(host, "two").bounds.width, 320);
        }
        {
            ui::Host host([] {
                return ui::Row{box().width(80_dp).height(20_dp).key("a"),
                               box().width(80_dp).height(20_dp).key("b"),
                               box().width(80_dp).height(20_dp).key("c")}
                    .gap(10_dp)
                    .wrap(ui::Wrap::wrap)
                    .align_items(ui::Align::start);
            });
            host.layout({190, 100});
            near(find(host, "b").bounds.x, 90);
            near(find(host, "c").bounds.y, 30);
        }
        {
            ui::Host host([] {
                return ui::Grid({.columns = {100_dp, 1_fr, 2_fr}, .column_gap = 10_dp, .row_gap = 10_dp},
                                {box().key("a").height(24_dp), box().key("b").height(24_dp),
                                 box().key("c").height(24_dp),
                                 box().key("span").height(40_dp).item(
                                     {.grid = {.row = 1, .column = 0, .column_span = 3}})});
            });
            host.layout({600, 200});
            near(find(host, "b").bounds.width, 160);
            near(find(host, "c").bounds.width, 320);
            near(find(host, "span").bounds.width, 600);
            near(find(host, "span").bounds.y, 34);
        }
        {
            ui::Host host([] {
                return ui::Column{box().key("a").height(20_dp), box().key("b").height(20_dp)}
                    .gap(10_dp)
                    .responsive_layout(300_dp, ui::GridLayout{.columns = {1_fr, 1_fr}, .column_gap = 10_dp});
            });
            host.layout({200, 100});
            near(find(host, "b").bounds.y, 30);
            host.layout({400, 100});
            near(find(host, "b").bounds.y, 0);
            near(find(host, "b").bounds.x, 205);
        }
        {
            ui::Host host([] {
                return ui::Text("A longer paragraph that wraps with available width and keeps its natural "
                                "line height.")
                    .key("text");
            });
            host.layout({300, 200});
            auto wide = find(host, "text").bounds; // root fills; wrap test via child below.
            ui::Host wrapped([] {
                return ui::Column{ui::Text("A longer paragraph that wraps with available width and keeps its "
                                           "natural line height.")
                                      .key("text")};
            });
            wrapped.layout({300, 400});
            double h = find(wrapped, "text").bounds.height;
            wrapped.layout({100, 400});
            CHECK(find(wrapped, "text").bounds.height > h);
            (void)wide;
        }
        {
            ui::Theme theme;
            theme.typeface = SkFontMgr_New_CoreText(nullptr)->matchFamilyStyle(nullptr, SkFontStyle{});
            theme.font_size = 16;
            SkFont font(theme.typeface, 16);
            font.setSubpixel(true);
            auto width = [&](std::string_view s) {
                return font.measureText(s.data(), s.size(), SkTextEncoding::kUTF8);
            };
            const double narrow = std::max({width("éé"), width("😀😀"), width("Next")}) + .5;
            ui::Host host([] { return ui::Column{ui::Text("éé 😀😀\nNext\n").key("utf8")}; }, {}, theme);
            host.layout({narrow, 400});
            near(find(host, "utf8").bounds.height, 4 * 16 * 1.35);
            host.layout({400, 400});
            near(find(host, "utf8").bounds.height, 3 * 16 * 1.35);
            host.layout({narrow, 400});
            near(find(host, "utf8").bounds.height, 4 * 16 * 1.35);
        }
        {
            ui::State<bool> checked{false};
            ui::State<double> value{.5};
            int clicks = 0, builds = 0;
            ui::Host host([&] {
                ++builds;
                return ui::Column{
                    ui::Button("Click", [&] { ++clicks; }).key("button"),
                    ui::CheckBox("Check", checked.binding()).key("check"),
                    ui::Slider(value.binding(), {.minimum = 0, .maximum = 1, .step = .1}).key("slider")}
                    .gap(10_dp);
            });
            paint(host, 0);
            CHECK(!host.needs_frame());
            click(host, "button");
            CHECK(clicks == 1);
            click(host, "check");
            CHECK(checked.get());
            host.layout({500, 400});
            CHECK(find(host, "check").checked);
            CHECK(builds == 2);
            auto slider = find(host, "slider");
            CHECK(host.focus(slider.id));
            CHECK(host.handle(sw::KeyEvent{.key = sw::Key::right, .down = true}));
            near(value.get(), .6);
            host.layout({500, 400});
            CHECK(host.handle(sw::KeyEvent{.key = sw::Key::tab, .down = true}));
            CHECK(find(host, "button").focused);
            auto b = find(host, "button");
            sw::Point p{b.bounds.x + 5, b.bounds.y + 5};
            host.handle(sw::PointerEvent{.kind = sw::PointerKind::down, .position = p});
            host.handle(sw::PointerEvent{.kind = sw::PointerKind::up, .position = {999, 999}});
            CHECK(clicks == 1);
            bool wrong_thread = false;
            std::thread worker([&] {
                try {
                    checked.set(false);
                } catch (const std::logic_error&) {
                    wrong_thread = true;
                }
            });
            worker.join();
            CHECK(wrong_thread);
        }
        {
            ui::State<bool> state{false};
            auto description = ui::CheckBox("Static binding", state.binding()).key("static");
            ui::Host host([description] { return description; });
            paint(host, 0);
            click(host, "static");
            host.layout({500, 400});
            CHECK(find(host, "static").checked);
        }
        {
            ui::State<bool> shown{true};
            ui::State<int> unused{0};
            int wakes = 0;
            ui::Host host(
                [&] { return ui::when(shown.get(), [&] { return ui::Text(std::to_string(unused.get())); }); },
                [&] { ++wakes; });
            paint(host, 0);
            shown.set(false);
            paint(host, .1);
            int before = wakes;
            unused.set(1);
            CHECK(wakes == before);
        }
        {
            ui::State<std::vector<int>> items{{1, 2, 3}};
            ui::Host host([&] {
                return ui::for_each(
                    items.get(), [](int id) { return id; },
                    [](int id) { return ui::Button(std::to_string(id), [] {}); });
            });
            host.layout({300, 200});
            auto old = find(host, "2");
            CHECK(host.focus(old.id));
            items.set({3, 2, 1});
            host.layout({300, 200});
            CHECK(find(host, "2").focused);
            CHECK(find(host, "2").id == old.id);
        }
        {
            ui::Host host([] {
                std::vector<ui::View> children;
                for (int i = 0; i < 12; ++i)
                    children.push_back(
                        ui::Button(std::to_string(i), [] {}).key(std::to_string(i)).height(40_dp));
                return ui::ScrollView(ui::group(std::move(children)));
            });
            host.layout({300, 100});
            double before = find(host, "11").bounds.y;
            CHECK(host.focus(find(host, "11").id));
            host.layout({300, 100});
            CHECK(find(host, "11").bounds.y < before);
            CHECK(find(host, "11").bounds.y + find(host, "11").bounds.height <= 100.05);
        }
        {
            ui::State<int> selected{0};
            ui::Host host([&] {
                return ui::ContentSwitcher(
                           selected.get(),
                           [](int n) -> ui::View { return ui::Text(n ? "Second" : "First").key("label"); })
                    .key("switch")
                    .transition(ui::Transition::fade());
            });
            paint(host, 0);
            ui::animate(ui::Animation::ease_in_out(1s), [&] { selected.set(1); });
            paint(host, 0);
            CHECK(host.transitions().size() == 1);
            CHECK(host.transitions()[0].has_outgoing);
            paint(host, .5);
            near(host.transitions()[0].progress, .5);
            ui::animate(ui::Animation::ease_out(1s), [&] { selected.set(0); });
            paint(host, .5);
            CHECK(host.transitions().size() == 1);
            near(host.transitions()[0].progress, 0);
            paint(host, 1.6);
            CHECK(host.transitions().empty());
            CHECK(!host.needs_frame());
            ui::animate(ui::Animation::ease_out(1s), [&] { selected.set(1); });
            host.set_reduced_motion(true);
            paint(host, 1.6);
            CHECK(host.transitions().empty());
            CHECK(!host.needs_frame());
        }
        {
            ui::State<bool> shown{false};
            ui::Host host([&] {
                return ui::Column{ui::when(shown.get(), [] { return box().height(100_dp); })
                                      .transition(ui::Transition::fade())
                                      .size_transition(ui::SizeTransition::animate),
                                  ui::Text("After").key("after")};
            });
            paint(host, 0);
            ui::animate(ui::Animation::ease_in_out(1s), [&] { shown.set(true); });
            paint(host, 0);
            near(find(host, "after").bounds.y, 0);
            paint(host, .5);
            near(find(host, "after").bounds.y, 50);
            paint(host, 1);
            near(find(host, "after").bounds.y, 100);
        }
        {
            struct Diagonal : ui::LayoutAlgorithm {
                sw::Size measure(ui::LayoutContext& context, ui::Constraints constraints) const override {
                    sw::Size result{};
                    for (std::size_t i = 0; i < context.child_count(); ++i) {
                        auto size = context.measure_child(i, constraints);
                        result.width = std::max(result.width, i * 25 + size.width);
                        result.height = std::max(result.height, i * 25 + size.height);
                    }
                    return result;
                }
                void arrange(ui::LayoutContext& context, ui::Rect bounds) const override {
                    for (std::size_t i = 0; i < context.child_count(); ++i) {
                        auto m = context.measure_child(i, {.maximum = {bounds.width, bounds.height}});
                        context.place_child(i, {bounds.x + i * 25, bounds.y + i * 25, m.width, m.height});
                    }
                }
            };
            ui::Host host([] {
                return ui::Container(std::make_shared<Diagonal>(),
                                     {box().key("first").width(30_dp).height(20_dp),
                                      box().key("second").width(30_dp).height(20_dp)})
                    .padding(10_dp);
            });
            host.layout({200, 200});
            near(find(host, "second").bounds.x, 35);
            near(find(host, "second").bounds.y, 35);
        }
        {
            // Verify the visible pixels, not just the animation bookkeeping.
            ui::State<int> selected{0};
            ui::Host host([&] {
                return ui::ContentSwitcher(selected.get(),
                                           [](int n) -> ui::View {
                                               return ui::Canvas([n](SkCanvas& canvas, const auto&) {
                                                          canvas.clear(n ? SK_ColorBLUE : SK_ColorRED);
                                                      })
                                                   .width(80_dp)
                                                   .height(80_dp);
                                           })
                    .transition(ui::Transition::fade());
            });
            auto surface = SkSurfaces::Raster(SkImageInfo::MakeN32Premul(100, 100));
            auto render = [&](double time) {
                surface->getCanvas()->clear(SK_ColorTRANSPARENT);
                host.paint(*surface->getCanvas(),
                           sw::FrameInfo{.logical_size = {100, 100},
                                         .pixel_size = {100, 100},
                                         .elapsed = std::chrono::duration<double>(time)});
            };
            render(0);
            ui::animate(ui::Animation{1s, ui::Curve::linear}, [&] { selected.set(1); });
            render(0);
            render(.5);
            SkBitmap pixel;
            pixel.allocN32Pixels(1, 1);
            CHECK(surface->readPixels(pixel, 10, 10));
            auto color = pixel.getColor(0, 0);
            CHECK(SkColorGetA(color) >= 250);
            CHECK(SkColorGetR(color) > 110 && SkColorGetB(color) > 110);
            render(1);
            CHECK(surface->readPixels(pixel, 10, 10));
            CHECK(SkColorGetB(pixel.getColor(0, 0)) >= 250);
        }
        {
            ui::State<bool> bright{false};
            ui::Host host([&] {
                return box()
                    .background(bright.get() ? 0xff0000ff : 0xffff0000)
                    .opacity(bright.get() ? .5 : 1);
            });
            paint(host, 0);
            ui::animate(ui::Animation{1s, ui::Curve::linear}, [&] { bright.set(true); });
            paint(host, 0);
            CHECK(host.needs_frame());
            paint(host, 1);
            CHECK(!host.needs_frame());
            host.set_system_reduced_motion(true);
            host.set_reduced_motion(false);
            ui::animate(ui::Animation::ease_out(1s), [&] { bright.set(false); });
            paint(host, 1);
            CHECK(!host.needs_frame());
        }
        {
            struct Counter {
                ui::State<int> value{0};
                ui::View body(const std::string& title) {
                    return ui::Column{ui::Text(title).key("nested-title"),
                                      ui::Button(std::to_string(value.get()), [value = value.binding()] {
                                          value.set(value.get() + 1);
                                      }).key("nested-counter")};
                }
            };
            ui::State<std::string> title{"Initial"};
            ui::State<bool> mounted{true};
            ui::Host host([&] {
                return ui::when(mounted.get(),
                                [&] { return ui::component<Counter>("counter", title.get()); });
            });
            host.layout({300, 200});
            click(host, "nested-counter");
            host.layout({300, 200});
            CHECK(find(host, "nested-counter").label == "1");
            title.set("Updated props");
            host.layout({300, 200});
            CHECK(find(host, "nested-counter").label == "1");
            CHECK(find(host, "nested-title").label == "Updated props");
            mounted.set(false);
            host.layout({300, 200});
            mounted.set(true);
            host.layout({300, 200});
            CHECK(find(host, "nested-counter").label == "0");
        }
        {
            ui::State<int> selection{0};
            int incoming_clicks = 0;
            ui::Host host([&] {
                return ui::ContentSwitcher(selection.get(),
                                           [&](int value) -> ui::View {
                                               return ui::Button(value ? "New" : "Old",
                                                                 [&, value] {
                                                                     if (value)
                                                                         ++incoming_clicks;
                                                                 })
                                                   .key("moving");
                                           })
                    .transition(ui::Transition::push(ui::Edge::trailing));
            });
            paint(host, 0, {200, 60});
            ui::animate(ui::Animation{1s, ui::Curve::linear}, [&] { selection.set(1); });
            paint(host, 0, {200, 60});
            paint(host, .5, {200, 60});
            near(find(host, "moving").bounds.x, 100);
            CHECK(!host.handle(sw::PointerEvent{.kind = sw::PointerKind::down, .position = {25, 30}}));
            CHECK(host.handle(sw::PointerEvent{.kind = sw::PointerKind::down, .position = {150, 30}}));
            CHECK(host.handle(sw::PointerEvent{.kind = sw::PointerKind::up, .position = {150, 30}}));
            CHECK(incoming_clicks == 1);
        }
        {
            ui::Host host([] {
                return ui::Row{ui::Text("Baseline").font_size(16_dp).key("label"),
                               ui::Button("Button", [] {}).key("button")}
                    .align_items(ui::Align::baseline);
            });
            host.layout({300, 80});
            auto a = find(host, "label"), b = find(host, "button");
            near(a.bounds.y + 16, b.bounds.y + (b.bounds.height - 14 * 1.35) / 2 + 14);
        }
        {
            ui::Host host([] { return ui::Row{box().key("bounds").min_height(40_dp).max_height(20_dp)}; });
            host.layout({100, 100});
            near(find(host, "bounds").bounds.height, 40);
            ui::Host grid([] {
                return ui::Grid({.columns = {ui::minmax(20_dp, 50_dp)}}, {box().key("range").width(40_dp)});
            });
            grid.layout({200, 200});
            near(find(grid, "range").bounds.width, 40);
        }
        {
            int clicks = 0;
            ui::Host host(
                [&] { return ui::Button("Disabled", [&] { ++clicks; }).enabled(false).key("disabled"); });
            host.layout({100, 40});
            CHECK(host.handle(sw::PointerEvent{.kind = sw::PointerKind::down, .position = {10, 10}}));
            CHECK(host.handle(sw::PointerEvent{.kind = sw::PointerKind::up, .position = {10, 10}}));
            CHECK(!host.activate(find(host, "disabled").id));
            CHECK(clicks == 0);
        }
        {
            ui::State<bool> shown{true};
            ui::Host host([&] {
                return ui::Column{ui::when(shown.get(),
                                           [] {
                                               return ui::Canvas([](SkCanvas& canvas, const auto&) {
                                                          canvas.clear(SK_ColorRED);
                                                      })
                                                   .height(80_dp);
                                           })
                                      .transition(ui::Transition::fade()),
                                  ui::Spacer{}}
                    .gap(10_dp);
            });
            auto surface = SkSurfaces::Raster(SkImageInfo::MakeN32Premul(100, 100));
            auto render = [&](double time) {
                surface->getCanvas()->clear(SK_ColorTRANSPARENT);
                host.paint(*surface->getCanvas(),
                           sw::FrameInfo{.logical_size = {100, 100},
                                         .pixel_size = {100, 100},
                                         .elapsed = std::chrono::duration<double>(time)});
            };
            render(0);
            ui::animate(ui::Animation{1s, ui::Curve::linear}, [&] { shown.set(false); });
            render(0);
            render(.5);
            SkBitmap pixel;
            pixel.allocN32Pixels(1, 1);
            CHECK(surface->readPixels(pixel, 10, 10));
            CHECK(SkColorGetA(pixel.getColor(0, 0)) > 110 && SkColorGetA(pixel.getColor(0, 0)) < 145);
            render(1);
            CHECK(surface->readPixels(pixel, 10, 10));
            CHECK(SkColorGetA(pixel.getColor(0, 0)) == 0);
            CHECK(!host.needs_frame());
        }
        {
            ui::Host bad([] { return ui::Column{ui::Text("a").key("same"), ui::Text("b").key("same")}; });
            bool threw = false;
            try {
                bad.layout({100, 100});
            } catch (const std::invalid_argument&) {
                threw = true;
            }
            CHECK(threw);
            ui::State<int> state{0};
            ui::Host impure([&] {
                state.set(1);
                return ui::Text("bad");
            });
            threw = false;
            try {
                impure.layout({100, 100});
            } catch (const std::logic_error&) {
                threw = true;
            }
            CHECK(threw);
        }
        std::cout << "PASS: declarative state, flex/grid/responsive layout, controls, focus/scroll, "
                     "identity, transitions and reduced motion\n";
        return 0;
    } catch (const std::exception& error) {
        std::cerr << error.what() << '\n';
        return 1;
    }
}
