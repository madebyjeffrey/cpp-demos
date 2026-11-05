#import <Metal/Metal.h>
#include <skia_window/ui.hpp>
#include "include/core/SkBitmap.h"
#include "include/core/SkCanvas.h"
#include "include/core/SkSurface.h"
#include "include/gpu/ganesh/GrDirectContext.h"
#include "include/gpu/ganesh/SkSurfaceGanesh.h"
#include "include/gpu/ganesh/mtl/GrMtlBackendContext.h"
#include "include/gpu/ganesh/mtl/GrMtlDirectContext.h"
#include <cstdlib>
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
int main() {
    @autoreleasepool {
        try {
            id<MTLDevice> device = MTLCreateSystemDefaultDevice();
            CHECK(device);
            bool capturing = false;
            if (const char* path = std::getenv("SKIA_WINDOW_UI_CAPTURE")) {
                MTLCaptureDescriptor* descriptor = [MTLCaptureDescriptor new];
                descriptor.captureObject = device;
                descriptor.destination = MTLCaptureDestinationGPUTraceDocument;
                descriptor.outputURL = [NSURL fileURLWithPath:[NSString stringWithUTF8String:path]];
                NSError* error = nil;
                capturing = [[MTLCaptureManager sharedCaptureManager] startCaptureWithDescriptor:descriptor
                                                                                           error:&error];
                if (!capturing)
                    throw std::runtime_error(error.localizedDescription.UTF8String);
            }
            struct CaptureScope {
                bool& active;
                ~CaptureScope() {
                    if (active)
                        [[MTLCaptureManager sharedCaptureManager] stopCapture];
                }
            } scope{capturing};
            id<MTLCommandQueue> queue = [device newCommandQueue];
            CHECK(queue);
            queue.label = @"Declarative UI offscreen validation";
            GrMtlBackendContext backend;
            backend.fDevice.retain((__bridge GrMTLHandle)device);
            backend.fQueue.retain((__bridge GrMTLHandle)queue);
            auto context = GrDirectContexts::MakeMetal(backend);
            CHECK(context);
            auto surface = SkSurfaces::RenderTarget(context.get(), skgpu::Budgeted::kNo,
                                                    SkImageInfo::MakeN32Premul(360, 360));
            CHECK(surface);
            CHECK(surface->getCanvas()->recordingContext());
            ui::State<bool> checked{false};
            ui::State<double> level{.5};
            ui::State<int> selected{0};
            int clicks = 0;
            ui::Host host([&] {
                return ui::Column{
                    ui::Text("GPU controls and transitions"),
                    ui::Button("Action", [&] { ++clicks; }).key("button"),
                    ui::CheckBox("Checked", checked.binding()).key("check"),
                    ui::Slider(level.binding()).key("slider"),
                    ui::ContentSwitcher(selected.get(),
                                        [](int n) -> ui::View {
                                            return ui::Canvas([n](SkCanvas& canvas, const auto&) {
                                                       canvas.clear(n ? SK_ColorBLUE : SK_ColorRED);
                                                   })
                                                .key("color")
                                                .height(100_dp);
                                        })
                        .transition(ui::Transition::fade()),
                }
                    .gap(10_dp)
                    .padding(12_dp)
                    .background(0xff10191f);
            });
            auto render = [&](double time) {
                surface->getCanvas()->clear(SK_ColorTRANSPARENT);
                host.paint(*surface->getCanvas(),
                           sw::FrameInfo{.logical_size = {360, 360},
                                         .pixel_size = {360, 360},
                                         .elapsed = std::chrono::duration<double>(time)});
                CHECK(context->flushAndSubmit().fSuccess);
            };
            auto find = [&](std::string_view key) {
                for (auto e : host.elements())
                    if (e.key == key)
                        return e;
                throw std::runtime_error("Missing GPU UI element");
            };
            auto sample = [&] {
                auto rect = find("color").bounds;
                SkBitmap pixel;
                pixel.allocN32Pixels(1, 1);
                CHECK(surface->readPixels(pixel, (int)rect.x + 10, (int)rect.y + 10));
                return pixel.getColor(0, 0);
            };
            render(0);
            CHECK(SkColorGetR(sample()) >= 250);
            CHECK(!host.needs_frame());
            CHECK(host.activate(find("button").id));
            CHECK(clicks == 1);
            CHECK(host.activate(find("check").id));
            CHECK(checked.get());
            CHECK(host.adjust(find("slider").id, 10));
            render(.1);
            CHECK(level.get() > .5);
            ui::animate(ui::Animation{1s, ui::Curve::linear}, [&] { selected.set(1); });
            render(1);
            CHECK(SkColorGetR(sample()) >= 250);
            render(1.5);
            auto blended = sample();
            CHECK(SkColorGetA(blended) >= 250);
            CHECK(SkColorGetR(blended) > 110 && SkColorGetB(blended) > 110);
            render(2);
            CHECK(SkColorGetB(sample()) >= 250);
            CHECK(host.transitions().empty());
            CHECK(!host.needs_frame());
            CHECK(context->flushAndSubmit(GrSyncCpu::kYes).fSuccess);
            if (capturing) {
                [[MTLCaptureManager sharedCaptureManager] stopCapture];
                capturing = false;
            }
            std::cout << "PASS: real Metal UI rendering, control bindings, opaque cross-fade pixels and idle "
                         "scheduling on "
                      << device.name.UTF8String << '\n';
            return 0;
        } catch (const std::exception& error) {
            std::cerr << error.what() << '\n';
            return 1;
        }
    }
}
