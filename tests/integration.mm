#import <AppKit/AppKit.h>
#import <QuartzCore/QuartzCore.h>
#include <skia_window/skia_window.hpp>
#include "include/core/SkBitmap.h"
#include "include/core/SkCanvas.h"
#include "include/core/SkPaint.h"

#include <chrono>
#include <cmath>
#include <format>
#include <iostream>
#include <stdexcept>
#include <thread>

namespace sw = skia_window;
#define CHECK(condition) do { if (!(condition)) throw std::runtime_error(std::format("Line {}: {}", __LINE__, #condition)); } while (false)

NSWindow* native_window(NSString* title) {
    for (NSWindow* window in NSApp.windows) if ([window.title isEqualToString:title]) return window;
    throw std::runtime_error("Native window not found");
}
NSMenuItem* command_item(NSMenu* menu, sw::Command command) {
    for (NSMenuItem* item in menu.itemArray) {
        if (item.tag == command && item.action) return item;
        if (item.submenu) if (auto result = command_item(item.submenu, command)) return result;
    }
    return nil;
}
void send_command(NSMenuItem* item) { CHECK(item); [NSApp sendAction:item.action to:item.target from:item]; }

int main() {
    @autoreleasepool {
        bool failed = false, finished = false, pixel_verified = false, second_frame = false;
        bool modeless_frame = false, modal_frame = false, modal_done = false;
        bool minimize_started = false, minimized = false, restored = false;
        id minimize_observer = nil, restore_observer = nil;
        int errors = 0, expected_errors = 0, commands = 0, parent_closed = 0, child_closed = 0;
        int modal_completions = 0, draw_calls = 0;
        std::unique_ptr<sw::Application> app;
        auto created = sw::Application::create({.name = "Skia Window Tests", .on_error = [&](const sw::Error& error) {
            ++errors;
            if (error.message != "expected callback exception") {
                std::cerr << "Callback error: " << error.message << std::endl;
                failed = true;
                if (app) app->post([&] { app->quit(); });
            }
        }, .on_new_window = {}});
        if (!created) { std::cerr << created.error().message << '\n'; return 1; }
        app = std::move(*created);
        std::cout << app->gpu_description() << std::endl;
        std::unique_ptr<sw::Window> first, second;
        std::unique_ptr<sw::Dialog> modeless, modal;
        bool behavioral_tests_scheduled = false;
        auto fail = [&](const std::exception& error) {
            failed = true; std::cerr << error.what() << std::endl; app->quit();
        };
        auto check_progress = [&] {
            if (!pixel_verified || !second_frame || !modeless_frame || !modal_frame || !modal_done || !restored || finished) return;
            finished = true;
            app->post([&] {
                try {
                    CHECK(errors == expected_errors);
                    // Focus selects each window's menu, while a modeless dialog keeps its parent's.
                    second->focus();
                    CHECK(command_item(NSApp.mainMenu, 22) != nil);
                    first->focus();
                    CHECK(command_item(NSApp.mainMenu, 11) != nil);
                    modeless->focus();
                    CHECK(command_item(NSApp.mainMenu, 11) != nil);
                    CHECK(first->invoke_command(11));
                    CHECK(commands == 1);
                    auto* item = command_item(NSApp.mainMenu, 11);
                    send_command(item);
                    CHECK(commands == 2);
                    CHECK([item.keyEquivalent isEqualToString:@"e"]);
                    CHECK(item.keyEquivalentModifierMask == NSEventModifierFlagCommand);
                    NSEvent* shortcut = [NSEvent keyEventWithType:NSEventTypeKeyDown location:NSZeroPoint
                        modifierFlags:NSEventModifierFlagCommand timestamp:0 windowNumber:0 context:nil
                        characters:@"e" charactersIgnoringModifiers:@"e" isARepeat:NO keyCode:14];
                    CHECK([NSApp.mainMenu performKeyEquivalent:shortcut]);
                    CHECK(commands == 3);
                    first->set_command_state(11, false, true);
                    CHECK(item.state == NSControlStateValueOn);
                    CHECK(!item.enabled);
                    CHECK(!first->invoke_command(11));
                    send_command(item); // Even a stale native action must respect disabled state.
                    CHECK(commands == 3);
                    first->set_command_state(11, true, false);
                    CHECK(item.state == NSControlStateValueOff);
                    CHECK(item.enabled);
                    CHECK(first->invoke_command(11));
                    CHECK(commands == 4);
                    CHECK(!first->invoke_command(999));
                    auto invalid = std::array{sw::MenuItem::submenu("Bad", {
                        sw::MenuItem::action(1, "One"), sw::MenuItem::action(1, "Duplicate")})};
                    CHECK(!first->set_menu(invalid, {}));
                    CHECK(first->invoke_command(11)); // Invalid replacement is atomic.
                    CHECK(commands == 5);
                    auto throwing = std::array{sw::MenuItem::submenu("Throw", {sw::MenuItem::action(12, "Throw")})};
                    CHECK(first->set_menu(throwing, [](auto) { throw std::runtime_error("expected callback exception"); }));
                    ++expected_errors;
                    CHECK(first->invoke_command(12));
                    CHECK(errors == expected_errors);

                    // A completion may destroy its own dialog wrapper without invalidating native callbacks.
                    auto reentrant = sw::Dialog::create(*first, {.title = "Reentrant", .size = {200, 150}, .minimum_size = {100, 80}});
                    CHECK(reentrant);
                    auto owned_dialog = std::move(*reentrant);
                    bool reentrant_completed = false;
                    CHECK(owned_dialog->show_modal([&](auto result) {
                        CHECK(result == sw::DialogResult::accepted);
                        reentrant_completed = true;
                        owned_dialog.reset();
                    }));
                    owned_dialog->dismiss(sw::DialogResult::accepted);
                    CHECK(!owned_dialog && reentrant_completed);

                    // A user close can be vetoed, then allowed. Programmatic close is idempotent.
                    second->set_callbacks({.on_draw = {}, .on_event = {}, .on_close_request = [] { return false; }, .on_closed = {}});
                    [native_window(@"Second") performClose:nil];
                    CHECK(second->is_open());
                    second->set_callbacks({.on_draw = {}, .on_event = {}, .on_close_request = [] { return true; }, .on_closed = {}});
                    [native_window(@"Second") performClose:nil];
                    CHECK(!second->is_open());
                    second->close();

                    // Destroy a visible, animating wrapper while its display callback is pending.
                    for (int i = 0; i < 20; ++i) {
                        auto temporary = sw::Window::create(*app, {.title = "Temporary", .size = {200, 150}, .minimum_size = {100, 80}});
                        CHECK(temporary);
                        (*temporary)->set_callbacks({.on_draw = [](auto& canvas, const auto&) { canvas.clear(SK_ColorBLUE); },
                            .on_event = {}, .on_close_request = {}, .on_closed = {}});
                        (*temporary)->set_animating(true);
                        (*temporary)->show();
                    }
                    // Closing the parent cancels a sheet exactly once and closes modeless children.
                    auto another = sw::Dialog::create(*first, {.title = "Cancelled", .size = {200, 150}, .minimum_size = {100, 80}});
                    CHECK(another);
                    CHECK((*another)->show_modal([&](auto result) { CHECK(result == sw::DialogResult::cancelled); ++modal_completions; }));
                    first->close();
                    CHECK(!first->is_open()); CHECK(!modeless->is_open()); CHECK(!(*another)->is_open());
                    CHECK(parent_closed == 1); CHECK(child_closed == 1); CHECK(modal_completions == 2);
                    first->close(); (*another)->dismiss();
                    CHECK(modal_completions == 2);
                    CHECK(!sw::Dialog::create(*first));
                    std::cout << "PASS: Metal pixel readback, both dialog renderers, menus, modality, close veto, lifetimes" << std::endl;
                    // Let stale native callbacks execute before ending the loop.
                    app->post([&] { app->post([&] { app->quit(); }); });
                } catch (const std::exception& error) { fail(error); }
            });
        };
        try {
            sw::WindowOptions invalid; invalid.size.width = -1;
            CHECK(!sw::Window::create(*app, invalid));
            std::expected<std::unique_ptr<sw::Application>, sw::Error> off_thread;
            std::thread worker([&] { off_thread = sw::Application::create(); });
            worker.join(); CHECK(!off_thread);
            auto one = sw::Window::create(*app, {.title = "First", .size = {360, 260}, .minimum_size = {100, 80}});
            auto two = sw::Window::create(*app, {.title = "Second", .size = {360, 260}, .minimum_size = {100, 80}});
            CHECK(one && two);
            first = std::move(*one); second = std::move(*two);
            auto first_menu = std::array{sw::MenuItem::submenu("First", {sw::MenuItem::action(11, "One", {sw::Key::e})})};
            auto second_menu = std::array{sw::MenuItem::submenu("Second", {sw::MenuItem::action(22, "Two")})};
            CHECK(first->set_menu(first_menu, [&](auto) { ++commands; }));
            CHECK(second->set_menu(second_menu, [&](auto) { commands += 100; }));
            first->set_callbacks({.on_draw = [&](SkCanvas& canvas, const sw::FrameInfo& frame) {
                ++draw_calls;
                try {
                    CHECK(canvas.recordingContext());
                    CHECK(frame.scale >= 1);
                    CHECK(std::abs(frame.pixel_size.width - frame.logical_size.width * frame.scale) <= 1);
                    canvas.clear(SK_ColorBLACK);
                    SkPaint paint; paint.setColor(SK_ColorRED);
                    canvas.drawRect(SkRect::MakeXYWH(2, 2, 24, 24), paint);
                    // readPixels ignores the canvas matrix; sample in physical pixels.
                    SkBitmap pixels; pixels.allocN32Pixels(1, 1);
                    CHECK(canvas.readPixels(pixels, (int)(8 * frame.scale), (int)(8 * frame.scale)));
                    const auto color = pixels.getColor(0, 0);
                    CHECK(SkColorGetR(color) >= 250 && SkColorGetG(color) <= 5 && SkColorGetB(color) <= 5);
                    pixel_verified = true;
                    if (!behavioral_tests_scheduled) {
                        behavioral_tests_scheduled = true;
                        app->post([&] {
                            try {
                                // Native content resize, logical pointer coordinates, and focus/input events.
                                auto* native = native_window(@"First");
                                bool resized = false, pointer_seen = false, key_seen = false;
                                first->set_callbacks({.on_draw = [](auto& c, const auto&) { c.clear(SK_ColorBLACK); },
                                    .on_event = [&](const sw::Event& event) {
                                        if (auto resize = std::get_if<sw::ResizeEvent>(&event)) resized = resize->logical_size.width == 400;
                                        if (auto p = std::get_if<sw::PointerEvent>(&event)) pointer_seen = p->position.x == 20 && p->position.y == 30;
                                        if (auto k = std::get_if<sw::KeyEvent>(&event)) key_seen = k->key == sw::Key::a && k->down;
                                    }, .on_close_request = {}, .on_closed = [&] { ++parent_closed; }});
                                [native setContentSize:NSMakeSize(400, 280)];
                                CHECK(resized);
                                NSPoint location = [native.contentView convertPoint:NSMakePoint(20, 30) toView:nil];
                                NSEvent* pointer = [NSEvent mouseEventWithType:NSEventTypeMouseMoved location:location modifierFlags:0
                                    timestamp:0 windowNumber:native.windowNumber context:nil eventNumber:0 clickCount:0 pressure:0];
                                [native.contentView mouseMoved:pointer];
                                CHECK(pointer_seen);
                                NSEvent* key = [NSEvent keyEventWithType:NSEventTypeKeyDown location:NSZeroPoint modifierFlags:0 timestamp:0
                                    windowNumber:native.windowNumber context:nil characters:@"a" charactersIgnoringModifiers:@"a" isARepeat:NO keyCode:0];
                                [native.contentView keyDown:key]; CHECK(key_seen);
                                // These locals must not remain captured by later native notifications.
                                first->set_callbacks({.on_draw = [](auto& c, const auto&) { c.clear(SK_ColorBLACK); },
                                    .on_event = {}, .on_close_request = {}, .on_closed = [&] { ++parent_closed; }});
                                first->hide(); first->show(); first->set_animating(false);
                                auto child = sw::Dialog::create(*first, {.title = "Modeless", .size = {250, 180}, .minimum_size = {100, 80}});
                                auto sheet = sw::Dialog::create(*first, {.title = "Modal", .size = {250, 180}, .minimum_size = {100, 80}});
                                CHECK(child && sheet); modeless = std::move(*child); modal = std::move(*sheet);
                                modeless->set_callbacks({.on_draw = [&](auto& c, const auto&) {
                                    c.clear(SK_ColorGREEN); modeless_frame = c.recordingContext() != nullptr; check_progress();
                                }, .on_event = {}, .on_close_request = {}, .on_closed = [&] { ++child_closed; }});
                                modeless->show();
                                [native_window(@"Modeless") orderFrontRegardless];
                                modal->set_callbacks({.on_draw = [&](auto& c, const auto&) {
                                    c.clear(SK_ColorBLUE); modal_frame = c.recordingContext() != nullptr;
                                    app->post([&] { modal->dismiss(sw::DialogResult::accepted); });
                                }, .on_event = {}, .on_close_request = {}, .on_closed = {}});
                                CHECK(modal->show_modal([&](auto result) {
                                    CHECK(result == sw::DialogResult::accepted); ++modal_completions;
                                    modal_done = true; check_progress();
                                }));
                                CHECK(!first->invoke_command(11));
                                CHECK(commands == 0);
                                auto duplicate = sw::Dialog::create(*first, {.title = "Duplicate", .size = {250, 180}, .minimum_size = {100, 80}});
                                CHECK(duplicate); CHECK(!(*duplicate)->show_modal({}));
                                CHECK(native.attachedSheet != nil);
                                [native orderFrontRegardless];
                                [native.attachedSheet orderFrontRegardless];
                            } catch (const std::exception& error) { fail(error); }
                        });
                    }
                    check_progress();
                } catch (const std::exception& error) { fail(error); }
            }, .on_event = {}, .on_close_request = {}, .on_closed = [&] { ++parent_closed; }});
            second->set_callbacks({.on_draw = [&](auto& canvas, const auto&) {
                canvas.clear(SK_ColorBLUE); second_frame = canvas.recordingContext() != nullptr; check_progress();
                if (!minimize_started) {
                    minimize_started = true;
                    app->post([&] { [native_window(@"Second") miniaturize:nil]; });
                }
            }, .on_event = {}, .on_close_request = {}, .on_closed = {}});
            auto did_minimize = [&] {
                try {
                    CHECK(native_window(@"Second").miniaturized);
                    minimized = true;
                    app->post([&] { [native_window(@"Second") deminiaturize:nil]; });
                } catch (const std::exception& error) { fail(error); }
            };
            auto did_restore = [&] {
                try {
                    CHECK(minimized && !native_window(@"Second").miniaturized);
                    restored = true;
                    check_progress();
                } catch (const std::exception& error) { fail(error); }
            };
            minimize_observer = [NSNotificationCenter.defaultCenter addObserverForName:NSWindowDidMiniaturizeNotification
                object:native_window(@"Second") queue:nil usingBlock:^(NSNotification*) { did_minimize(); }];
            restore_observer = [NSNotificationCenter.defaultCenter addObserverForName:NSWindowDidDeminiaturizeNotification
                object:native_window(@"Second") queue:nil usingBlock:^(NSNotification*) { did_restore(); }];
            first->show(); second->show();
            // CLI test runners are background processes: do not rely on cooperative app activation.
            // Keep both canvases exposed; fully occluded windows intentionally do not render.
            const auto screen = NSScreen.mainScreen.visibleFrame;
            [native_window(@"First") setFrameOrigin:NSMakePoint(screen.origin.x + 20, screen.origin.y + 20)];
            [native_window(@"Second") setFrameOrigin:NSMakePoint(screen.origin.x + 440, screen.origin.y + 20)];
            [native_window(@"First") orderFrontRegardless];
            [native_window(@"Second") orderFrontRegardless];
            auto timeout = [&] {
                if (!finished) {
                    std::cerr << "Timed out waiting for GPU/window callbacks: " << pixel_verified << second_frame << modeless_frame << modal_frame << modal_done << '\n';
                    failed = true;
                    app->quit();
                }
            };
            dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 20 * NSEC_PER_SEC), dispatch_get_main_queue(), ^{ timeout(); });
            app->run();
            [NSNotificationCenter.defaultCenter removeObserver:minimize_observer];
            [NSNotificationCenter.defaultCenter removeObserver:restore_observer];
        } catch (const std::exception& error) { std::cerr << error.what() << '\n'; failed = true; }
        modeless.reset(); modal.reset(); first.reset(); second.reset();
        // Application destruction must drop pending posted work, including functions holding user state.
        bool pending_called = false;
        app->post([&] { pending_called = true; });
        app.reset();
        [NSRunLoop.mainRunLoop runUntilDate:[NSDate dateWithTimeIntervalSinceNow:.02]];
        if (pending_called) { std::cerr << "Pending callback survived application destruction\n"; failed = true; }
        return failed || !finished || !pixel_verified || !draw_calls ? 1 : 0;
    }
}
