#include <skia_window/skia_window.hpp>
#include <iostream>
#include <stdexcept>
#include <string>
import scars;
import scars.macos;
namespace s = scars;
namespace sw = skia_window;
void scars_colors_native_step(int);
void scars_colors_native_closed();
namespace {
void check(bool value, const char* message) {
    if (!value)
        throw std::runtime_error(message);
}
void run(sw::Application& app) {
    auto created = sw::Window::create(
        app, {.title = "SCARS Color Test", .size = {420, 900}, .minimum_size = {320, 300}});
    check(bool(created), "Create native color window");
    auto window = std::move(*created);
    s::State<s::Color> paint{0x80507090}, preview{0xff112233};
    s::State<bool> readonly{false};
    int commits{}, previews{};
    s::Binding<s::Color> binding{[&] { return paint.get(); },
                                 [&](auto value) {
                                     ++commits;
                                     paint.set(value);
                                 }};
    s::Binding<s::Color> live{[&] { return preview.get(); },
                              [&](auto value) {
                                  ++previews;
                                  preview.set(value);
                              }};
    auto host = s::macos::mount(*window, [&] {
        return s::FlexColumn{s::ColorWell("Preview", live, {.live_preview = true}).key("well"),
                             s::ColorPicker("Paint", binding, {.read_only = readonly.get()}).key("picker")}
            .gap(14)
            .padding(16);
    });
    window->show();
    host->layout({420, 900});
    scars_colors_native_step(0);
    check(commits == 0 && paint.get() == 0x80507090, "Native invalid hex cannot reach the binding");
    scars_colors_native_step(1);
    check(commits == 0 && paint.get() == 0x80507090, "Native component Return stages default color drafts");
    scars_colors_native_step(2);
    check(commits == 1 && paint.get() == 0x40123456, "Native Apply publishes one packed ARGB value");
    scars_colors_native_step(3);
    check(previews == 1 && preview.get() == 0xffc82233, "Native live preview publishes component edits");
    scars_colors_native_step(4);
    check(previews == 1 && preview.get() == 0xffc82233 && !host->text_input()->marked,
          "Native Return completes color composition before preview publication");
    scars_colors_native_step(5);
    check(previews == 2 && preview.get() == 0xffdc2233,
          "A subsequent Return commits the composed color component");
    scars_colors_native_step(6);
    check(previews == 3 && preview.get() == 0xff112233,
          "Native Escape cancels and restores an owned live preview");
    readonly.set(true);
    host->layout({420, 900});
    scars_colors_native_step(7);
    check(commits == 1 && paint.get() == 0x40123456,
          "Read-only color editors reject native replacement and Apply");
    scars_colors_native_step(3);
    check(previews == 4 && preview.get() == 0xffc82233,
          "A second native preview starts from the current model");
    window->close();
    check(previews == 5 && preview.get() == 0xff112233,
          "Window closure rolls back a retained host's live preview");
    scars_colors_native_closed();
    std::cout
        << "Native color editing, composition, Apply/Cancel, accessibility, and lifetime checks passed.\n";
}
} // namespace
int main() {
    try {
        std::string failure;
        auto created = sw::Application::create(
            {.name = "SCARS Color Tests", .on_error = [&](const auto& e) { failure = e.message; }});
        if (!created)
            throw std::runtime_error(created.error().message);
        auto app = std::move(*created);
        app->post([&] {
            try {
                run(*app);
            } catch (const std::exception& e) {
                failure = e.what();
            }
            app->quit();
        });
        app->run();
        if (!failure.empty())
            throw std::runtime_error(failure);
        return 0;
    } catch (const std::exception& e) {
        std::cerr << "SCARS native color: " << e.what() << '\n';
        return 1;
    }
}
