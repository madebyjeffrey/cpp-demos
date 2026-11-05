#pragma once
#include <skia_window/ui.hpp>
class SkRuntimeEffect;
namespace demo {
struct ControlsLab {
    std::unique_ptr<skia_window::Window> window;
    static std::unique_ptr<ControlsLab> create(skia_window::Application&, sk_sp<SkTypeface>, sk_sp<SkImage>,
                                               sk_sp<SkRuntimeEffect>, std::string gpu);
};
void render_controls_preview(sk_sp<SkTypeface>, sk_sp<SkImage>, sk_sp<SkRuntimeEffect>, const char* output,
                             skia_window::Size, int panel = 0);
int benchmark_controls(sk_sp<SkTypeface>, sk_sp<SkImage>, sk_sp<SkRuntimeEffect>);
} // namespace demo
