#include <skia_window/ui.hpp>
#include "include/core/SkCanvas.h"
#include <iostream>
#include <stdexcept>

namespace sw = skia_window;
namespace ui = sw::ui;
using namespace ui::literals;
using namespace std::chrono_literals;

struct CountLayout final : ui::LayoutAlgorithm {
    mutable unsigned measures{}, arrangements{};
    sw::Size measure(ui::LayoutContext&, ui::Constraints) const override {
        ++measures;
        return {100, 50};
    }
    void arrange(ui::LayoutContext&, ui::Rect) const override {
        ++arrangements;
    }
};
void check(bool condition, const char* message) {
    if (!condition)
        throw std::runtime_error(message);
}
int main() {
    try {
        auto counter = std::make_shared<CountLayout>();
        ui::State<int> label{0};
        ui::Host host([&] {
            ui::View view = ui::Container(counter, {});
            // Nested grid/overlay composition is common in the Controls Lab.
            for (int depth = 0; depth < 5; ++depth)
                view = ui::Grid({.columns = {1_fr}}, {view});
            return ui::Overlay{
                view, ui::Canvas([](auto&, const auto&) {}).animating(),
                ui::Text(std::to_string(label.get())).content_transition(ui::ContentTransition::cross_fade)};
        });
        SkCanvas canvas(1200, 900);
        auto paint = [&](double time, double width) {
            host.paint(canvas, {.logical_size = {width, 900},
                                .pixel_size = {width, 900},
                                .elapsed = std::chrono::duration<double>(time)});
        };
        paint(0, 1200);
        std::cout << "Initial leaf measurements: " << counter->measures << '\n';
        check(counter->measures <= 32, "Repeated subtree measurement exceeds linear layout budget");
        counter->measures = counter->arrangements = 0;
        for (int i = 1; i <= 60; ++i)
            paint(i / 120., 1200);
        std::cout << "Unchanged animation measurements: " << counter->measures << '\n';
        check(counter->measures == 0 && counter->arrangements == 0,
              "Canvas animation repeatedly lays out an unchanged tree");
        host.handle(sw::ResizeEvent{{1100, 900}, {1100, 900}, 1});
        paint(1, 1100);
        std::cout << "Resize leaf measurements: " << counter->measures << '\n';
        check(counter->measures > 0 && counter->measures <= 32,
              "Resize repeats old/new/draw layout or exponential subtree measurement");
        counter->measures = 0;
        ui::animate(ui::Animation::ease_in_out(250ms), [&] { label.set(1); });
        paint(2, 1100);
        check(counter->measures > 0, "State changes must invalidate cached layout");
        counter->measures = counter->arrangements = 0;
        for (int i = 1; i <= 60; ++i)
            paint(2 + i / 120., 1100);
        check(counter->measures == 0 && counter->arrangements == 0,
              "A content-only transition unnecessarily lays out every animation frame");
        std::cout << "PASS: bounded nested layout and no layout on canvas-only frames\n";
        return 0;
    } catch (const std::exception& error) {
        std::cerr << error.what() << '\n';
        return 1;
    }
}
