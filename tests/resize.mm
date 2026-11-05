#import <AppKit/AppKit.h>
#import <QuartzCore/CAMetalLayer.h>
#include <skia_window/ui.hpp>
#include "include/core/SkCanvas.h"
#include <iostream>
#include <stdexcept>

namespace sw = skia_window;
namespace ui = sw::ui;
using namespace ui::literals;
static void check(bool value, const char* message) {
    if (!value)
        throw std::runtime_error(message);
}
int main() {
    @autoreleasepool {
        try {
            bool failed = false, finished = false, started = false, resizing = false;
            unsigned frames = 0, resize_events = 0;
            auto made = sw::Application::create({.name = "Resize test", .on_error = [&](const sw::Error& e) {
                                                     failed = true;
                                                     std::cerr << e.message << '\n';
                                                 }});
            check(bool(made), "Application creation failed");
            auto app = std::move(*made);
            auto created = sw::Window::create(
                *app, {.title = "Resize pacing regression", .size = {400, 300}, .minimum_size = {200, 200}});
            check(bool(created), "Window creation failed");
            auto window = std::move(*created);
            window->set_content(
                ui::Column{ui::Text("Resize"), ui::Canvas([](auto&, const auto&) {}).animating()});
            NSWindow* native = nil;
            for (NSWindow* candidate in NSApp.windows)
                if ([candidate.title isEqualToString:@"Resize pacing regression"])
                    native = candidate;
            check(native != nil, "Missing native window");
            auto inspect = [&] {
                try {
                    auto layer = (CAMetalLayer*)native.contentView.layer;
                    // Each AppKit resize must draw the matching content before the sizing transaction
                    // can commit. Waiting for the next display-link tick reuses/stretches the old frame.
                    for (int i = 0; i < 8; ++i) {
                        const auto before = frames;
                        const auto events_before = resize_events;
                        resizing = true;
                        [native setContentSize:NSMakeSize(410 + i * 11, 310 + i * 7)];
                        resizing = false;
                        check(frames > before, "Resize committed without a matching rendered frame");
                        check(resize_events == events_before + 1,
                              "Duplicate resize notification for the same content size");
                        check(!layer.presentsWithTransaction,
                              "Synchronous presentation leaked into animation frames");
                    }
                    const auto before = frames;
                    // AppKit uses event-tracking mode while dragging. Animation must continue there.
                    const auto deadline = [NSDate dateWithTimeIntervalSinceNow:.15];
                    while (deadline.timeIntervalSinceNow > 0)
                        [NSRunLoop.mainRunLoop runMode:NSEventTrackingRunLoopMode beforeDate:deadline];
                    check(frames >= before + 2, "Animation stopped in event-tracking run-loop mode");
                    finished = true;
                    window->close();
                    app->quit();
                    std::cout << "PASS: resize draws inside the sizing transaction, deduplicated events, "
                                 "tracking-mode animation\n";
                } catch (const std::exception& e) {
                    std::cerr << e.what() << '\n';
                    failed = true;
                    app->quit();
                }
            };
            window->set_callbacks(
                {.on_draw =
                     [&](SkCanvas& canvas, const sw::FrameInfo& frame) {
                         check(canvas.recordingContext(), "Expected GPU canvas");
                         ++frames;
                         const auto bounds = native.contentView.bounds.size;
                         check(frame.logical_size.width == bounds.width &&
                                   frame.logical_size.height == bounds.height,
                               "Stale logical frame size");
                         if (resizing)
                             check(((CAMetalLayer*)native.contentView.layer).presentsWithTransaction,
                                   "Resize frame is presented outside the sizing transaction");
                         if (!started) {
                             started = true;
                             app->post(inspect);
                         }
                     },
                 .on_event =
                     [&](const sw::Event& e) {
                         if (std::holds_alternative<sw::ResizeEvent>(e))
                             ++resize_events;
                     }});
            window->show();
            [native orderFrontRegardless];
            auto timeout = [&] {
                if (!finished) {
                    failed = true;
                    std::cerr << "Resize test timed out\n";
                    app->quit();
                }
            };
            dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 8 * NSEC_PER_SEC), dispatch_get_main_queue(), ^{
              timeout();
            });
            app->run();
            return failed || !finished ? 1 : 0;
        } catch (const std::exception& e) {
            std::cerr << e.what() << '\n';
            return 1;
        }
    }
}
