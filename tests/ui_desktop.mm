#import <AppKit/AppKit.h>
#include <skia_window/ui.hpp>
#include "include/core/SkCanvas.h"
#include <format>
#include <iostream>

namespace sw = skia_window;
namespace ui = sw::ui;
using namespace ui::literals;
using namespace std::chrono_literals;
#define CHECK(x)                                                                                             \
    do {                                                                                                     \
        if (!(x))                                                                                            \
            throw std::runtime_error(std::format("Line {}: {}", __LINE__, #x));                              \
    } while (false)
struct Scene {
    ui::State<bool> checked{false};
    ui::State<double> value{.5};
    ui::State<int> selected{0};
    int clicks{};
    ui::View body() {
        return ui::Column{
            ui::Button("Press me", [this] { ++clicks; }).key("press"),
            ui::CheckBox("Enabled", checked.binding()).key("check"),
            ui::Slider(value.binding(), {.minimum = 0, .maximum = 1, .step = .1})
                .accessible_label("Level")
                .key("slider"),
            ui::ContentSwitcher(selected.get(),
                                [](int n) -> ui::View {
                                    return ui::Canvas([n](SkCanvas& canvas, const auto&) {
                                               canvas.clear(n ? SK_ColorBLUE : SK_ColorRED);
                                           })
                                        .height(100_dp)
                                        .accessible_label(n ? "Blue canvas" : "Red canvas");
                                })
                .transition(ui::Transition::fade()),
        }
            .gap(10_dp)
            .padding(16_dp)
            .background(0xff10191f);
    }
};
ui::ElementInfo element(ui::Host& host, std::string_view key) {
    for (auto e : host.elements())
        if (e.key == key)
            return e;
    throw std::runtime_error("Missing UI element");
}
NSAccessibilityElement* native_element(NSView* view, NSString* label) {
    for (NSAccessibilityElement* e in view.accessibilityChildren)
        if ([e.accessibilityLabel isEqualToString:label])
            return e;
    throw std::runtime_error("Missing native accessibility element");
}
int run(bool actions_only) {
    bool failed = false, finished = false, started = false;
    int frames = 0, raw_pointer = 0;
    std::unique_ptr<sw::Application> app;
    auto made = sw::Application::create({.name = "UI Integration",
                                         .on_error =
                                             [&](const sw::Error& error) {
                                                 std::cerr << error.message << '\n';
                                                 failed = true;
                                                 if (app)
                                                     app->quit();
                                             },
                                         .on_new_window = {}});
    if (!made)
        throw std::runtime_error(made.error().message);
    app = std::move(*made);
    auto created =
        sw::Window::create(*app, {.title = "UI Integration", .size = {360, 360}, .minimum_size = {200, 200}});
    CHECK(created);
    auto window = std::move(*created);
    auto model = window->set_view<Scene>();
    NSWindow* native = nil;
    for (NSWindow* candidate in NSApp.windows)
        if ([candidate.title isEqualToString:@"UI Integration"])
            native = candidate;
    CHECK(native);
    auto fail = [&](const std::exception& error) {
        std::cerr << error.what() << '\n';
        failed = true;
        app->quit();
    };
    std::unique_ptr<sw::Dialog> dialog;
    auto inspect = [&] {
        try {
            auto host = window->ui_host();
            CHECK(host);
            CHECK(!window->is_animating());
            CHECK([native.contentView.accessibilityRole isEqualToString:NSAccessibilityGroupRole]);
            auto button = native_element(native.contentView, @"Press me");
            CHECK([button.accessibilityRole isEqualToString:NSAccessibilityButtonRole]);
            CHECK([button accessibilityPerformPress]);
            CHECK(model->clicks == 1);
            auto check = native_element(native.contentView, @"Enabled");
            CHECK([check.accessibilityRole isEqualToString:NSAccessibilityCheckBoxRole]);
            CHECK([check accessibilityPerformPress]);
            CHECK(model->checked.get());
            CHECK([check.accessibilityValue boolValue]);
            auto slider = native_element(native.contentView, @"Level");
            CHECK([slider.accessibilityRole isEqualToString:NSAccessibilitySliderRole]);
            CHECK([slider accessibilityPerformIncrement]);
            CHECK(std::abs(model->value.get() - .6) < .001);
            [slider setAccessibilityValue:@(.8)];
            CHECK(std::abs(model->value.get() - .8) < .001);
            auto info = element(*host, "press");
            NSPoint point =
                [native.contentView convertPoint:NSMakePoint(info.bounds.x + 10, info.bounds.y + 10)
                                          toView:nil];
            for (auto type : {NSEventTypeLeftMouseDown, NSEventTypeLeftMouseUp}) {
                auto event = [NSEvent mouseEventWithType:type
                                                location:point
                                           modifierFlags:0
                                               timestamp:0
                                            windowNumber:native.windowNumber
                                                 context:nil
                                             eventNumber:0
                                              clickCount:1
                                                pressure:0];
                if (type == NSEventTypeLeftMouseDown)
                    [native.contentView mouseDown:event];
                else
                    [native.contentView mouseUp:event];
            }
            CHECK(model->clicks == 2);
            CHECK(raw_pointer == 0);
            const int before = frames;
            ui::animate(ui::Animation::ease_in_out(200ms), [&] { model->selected.set(1); });
            auto complete = [&, before] {
                try {
                    auto host = window->ui_host();
                    if (!actions_only) {
                        CHECK(frames > before);
                        CHECK(host->transitions().empty());
                        CHECK(!host->needs_frame());
                    }
                    CHECK(native_element(native.contentView, @"Blue canvas"));
                    auto made_dialog = sw::Dialog::create(
                        *window, {.title = "UI Dialog", .size = {260, 160}, .minimum_size = {200, 120}});
                    CHECK(made_dialog);
                    dialog = std::move(*made_dialog);
                    dialog->set_content(ui::Column{ui::Text("Dialog controls"),
                                                   ui::CheckBox("Shared", model->checked.binding())}
                                            .padding(16_dp));
                    CHECK(dialog->show_modal({}));
                    CHECK(native.attachedSheet);
                    auto blocked = native_element(native.contentView, @"Press me");
                    CHECK(!blocked.accessibilityEnabled);
                    CHECK(![blocked accessibilityPerformPress]);
                    NSWindow* child = nil;
                    for (NSWindow* candidate in NSApp.windows)
                        if ([candidate.title isEqualToString:@"UI Dialog"])
                            child = candidate;
                    CHECK(child);
                    auto shared = native_element(child.contentView, @"Shared");
                    CHECK([shared.accessibilityValue boolValue]);
                    CHECK([shared accessibilityPerformPress]);
                    CHECK(!model->checked.get());
                    dialog->close();
                    CHECK(native_element(native.contentView, @"Press me").accessibilityEnabled);
                    // A failed replacement must leave the previous host's subscriptions usable.
                    bool threw = false;
                    try {
                        window->set_view_builder(
                            []() -> ui::View { throw std::runtime_error("expected mount failure"); });
                    } catch (const std::runtime_error&) {
                        threw = true;
                    }
                    CHECK(threw);
                    CHECK(window->ui_host() == host);
                    model->checked.set(true);
                    host->layout({360, 360});
                    CHECK([native_element(native.contentView, @"Enabled").accessibilityValue boolValue]);
                    window->close();
                    CHECK(!window->ui_host());
                    window->clear_content();
                    CHECK(![blocked accessibilityPerformPress]);
                    finished = true;
                    std::cout << (actions_only ? "PASS: native accessibility actions, consumed input, modal "
                                                 "bindings and mount lifetimes"
                                               : "PASS: native Metal UI frames, transitions, accessibility "
                                                 "and dialog bindings")
                              << '\n';
                    app->quit();
                } catch (const std::exception& error) {
                    fail(error);
                }
            };
            if (actions_only)
                complete();
            else
                dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 700 * NSEC_PER_MSEC),
                               dispatch_get_main_queue(), ^{
                                 complete();
                               });
        } catch (const std::exception& error) {
            fail(error);
        }
    };
    window->set_callbacks({.on_draw =
                               [&](SkCanvas& canvas, const auto&) {
                                   CHECK(canvas.recordingContext());
                                   ++frames;
                                   if (!started) {
                                       started = true;
                                       app->post(inspect);
                                   }
                               },
                           .on_event =
                               [&](const sw::Event& event) {
                                   if (auto p = std::get_if<sw::PointerEvent>(&event);
                                       p &&
                                       (p->kind == sw::PointerKind::down || p->kind == sw::PointerKind::up))
                                       ++raw_pointer;
                               },
                           .on_close_request = {},
                           .on_closed = {}});
    window->show();
    auto screen = NSScreen.mainScreen.visibleFrame;
    [native setFrameOrigin:NSMakePoint(screen.origin.x + 30, screen.origin.y + 30)];
    [native orderFrontRegardless];
    if (actions_only) {
        started = true;
        app->post(inspect);
    }
    auto timeout = [&] {
        if (!finished) {
            std::cerr << "UI desktop test timed out\n";
            failed = true;
            app->quit();
        }
    };
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 10 * NSEC_PER_SEC), dispatch_get_main_queue(), ^{
      timeout();
    });
    app->run();
    return failed || !finished ? 1 : 0;
}
int main(int argc, char** argv) {
    @autoreleasepool {
        try {
            return run(argc > 1 && std::string_view(argv[1]) == "--actions-only");
        } catch (const std::exception& error) {
            std::cerr << error.what() << '\n';
            return 1;
        }
    }
}
