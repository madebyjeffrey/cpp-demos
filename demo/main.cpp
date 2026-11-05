#include <skia_window/skia_window.hpp>
#include "controls.hpp"
#include "include/core/SkBitmap.h"
#include "include/core/SkCanvas.h"
#include "include/core/SkData.h"
#include "include/core/SkFont.h"
#include "include/core/SkFontMgr.h"
#include "include/core/SkImage.h"
#include "include/core/SkM44.h"
#include "include/core/SkPaint.h"
#include "include/core/SkPath.h"
#include "include/core/SkPathBuilder.h"
#include "include/core/SkRRect.h"
#include "include/core/SkStream.h"
#include "include/core/SkTypeface.h"
#include "include/effects/SkGradient.h"
#include "include/effects/SkImageFilters.h"
#include "include/effects/SkRuntimeEffect.h"
#include "include/encode/SkPngEncoder.h"
#include "include/ports/SkFontMgr_mac_ct.h"
#include <mach-o/dyld.h>

#include <algorithm>
#include <array>
#include <cmath>
#include <cstdlib>
#include <filesystem>
#include <format>
#include <iostream>
#include <memory>
#include <numbers>
#include <stdexcept>
#include <string_view>

namespace sw = skia_window;
namespace {
constexpr SkColor background = 0xff10191f, panel = 0xff19262e, border = 0xff30424d;
constexpr SkColor white = 0xffeaf4f6, muted = 0xff91a9b6, mint = 0xff66e0c2, blue = 0xff73a9ff;
constexpr sw::Command new_window = 1, pause_animation = 2, reset_animation = 3,
    open_inspector = 4, open_sheet = 5, close_window = 6, controls_lab = 7;

void fill(SkCanvas& canvas, const SkRect& rect, SkColor color, float radius = 0) {
    SkPaint paint; paint.setAntiAlias(true); paint.setColor(color);
    canvas.drawRoundRect(rect, radius, radius, paint);
}
void text(SkCanvas& canvas, const sk_sp<SkTypeface>& face, std::string_view value,
          float x, float y, float size, SkColor color = white) {
    SkFont font(face, size);
    font.setSubpixel(true);
    SkPaint paint; paint.setColor(color); paint.setAntiAlias(true);
    canvas.drawSimpleText(value.data(), value.size(), SkTextEncoding::kUTF8, x, y, font, paint);
}
std::filesystem::path resources() {
    std::uint32_t length = 0;
    _NSGetExecutablePath(nullptr, &length);
    std::vector<char> buffer(length);
    if (_NSGetExecutablePath(buffer.data(), &length)) throw std::runtime_error("Cannot locate executable");
    return std::filesystem::weakly_canonical(buffer.data()).parent_path().parent_path() / "Resources";
}
struct Assets {
    sk_sp<SkTypeface> face;
    sk_sp<SkImage> image;
    sk_sp<SkRuntimeEffect> effect;
    Assets() {
        const auto root = resources();
        auto font_manager = SkFontMgr_New_CoreText(nullptr);
        face = font_manager->makeFromFile((root / "Roboto-Regular.ttf").c_str());
        image = SkImages::DeferredFromEncodedData(SkData::MakeFromFileName((root / "texture.png").c_str()));
        if (!face || !image) throw std::runtime_error("Bundled font or texture is missing");
        auto compiled = SkRuntimeEffect::MakeForShader(SkString(R"(
            uniform float time;
            uniform float2 resolution;
            half4 main(float2 point) {
                float2 uv = point / resolution;
                float wave = sin(uv.x * 9 + time) * cos(uv.y * 8 - time * .7);
                float rings = sin(length((uv - .5) * float2(1.7, 1)) * 23 - time * 2);
                float mixValue = .5 + .25 * wave + .25 * rings;
                return half4(mix(half3(.12, .25, .52), half3(.32, .92, .72), mixValue), 1);
            }
        )"));
        if (!compiled.effect) throw std::runtime_error(compiled.errorText.c_str());
        effect = std::move(compiled.effect);
    }
};
struct Demo;
struct Gallery {
    Demo& demo;
    std::unique_ptr<sw::Window> window;
    std::unique_ptr<sw::Dialog> inspector, sheet;
    std::array<SkRect, 6> cards{};
    sw::Point pointer{-1, -1};
    double time{}, speed{1}, smoothed_interval{};
    int hovered{-1};
    bool animating{true};
    bool capture_done{};
    std::string status{"Move over a card. Scroll to change animation speed."};
    Gallery(Demo&, int);
    void draw(SkCanvas&, const sw::FrameInfo&);
    void draw_example(SkCanvas&, int);
    void event(const sw::Event&);
    void command(sw::Command);
    void show_inspector();
    void show_sheet();
};
struct Demo {
    std::unique_ptr<sw::Application> application;
    Assets assets;
    std::vector<std::unique_ptr<Gallery>> galleries;
    std::unique_ptr<demo::ControlsLab> controls;
    std::string gpu;
    int count{};
    bool smoke{}, failed{};
    explicit Demo(bool smoke_mode) : smoke(smoke_mode) {
        auto created = sw::Application::create({.name = "Skia Gallery",
            .on_error = [this](const sw::Error& error) {
                failed = true;
                std::cerr << error.message << '\n';
                if (smoke && application) application->post([this] { application->quit(); });
            }, .on_new_window = [this] { add_window(); }});
        if (!created) throw std::runtime_error(created.error().message);
        application = std::move(*created);
        gpu = application->gpu_description();
        std::cout << gpu << std::endl;
    }
    void show_controls() {
        if (controls && controls->window->is_open()) { controls->window->focus(); return; }
        controls = demo::ControlsLab::create(*application, assets.face, assets.image, assets.effect, gpu);
    }
    void add_window() { galleries.push_back(std::make_unique<Gallery>(*this, ++count)); }
    void cleanup() {
        application->post([this] {
            std::erase_if(galleries, [](const auto& gallery) { return !gallery->window->is_open(); });
        });
    }
};

Gallery::Gallery(Demo& owner, int index) : demo(owner) {
    auto created = sw::Window::create(*demo.application, {
        .title = std::format("Skia Gallery — Window {}", index),
        .size = {1200, 860}, .minimum_size = {840, 650}});
    if (!created) throw std::runtime_error(created.error().message);
    window = std::move(*created);
    const std::array menu{
        sw::MenuItem::submenu("File", {
            sw::MenuItem::action(new_window, "New Window", {sw::Key::n}),
            sw::MenuItem::divider(), sw::MenuItem::action(close_window, "Close Window", {sw::Key::w})}),
        sw::MenuItem::submenu("View", {
            sw::MenuItem::action(pause_animation, "Animate", {sw::Key::p}),
            sw::MenuItem::action(reset_animation, "Reset", {sw::Key::r}),
            sw::MenuItem::divider(), sw::MenuItem::action(controls_lab, "Controls Lab", {sw::Key::l})}),
        sw::MenuItem::submenu("Dialogs", {
            sw::MenuItem::action(open_inspector, "Modeless Inspector", {sw::Key::i}),
            sw::MenuItem::action(open_sheet, "Modal Example", {sw::Key::d})})};
    auto menu_result = window->set_menu(menu, [this](auto id) { command(id); });
    if (!menu_result) throw std::runtime_error(menu_result.error().message);
    window->set_command_state(pause_animation, true, true);
    window->set_callbacks({.on_draw = [this](auto& canvas, const auto& frame) { draw(canvas, frame); },
        .on_event = [this](const auto& input) { event(input); },
        .on_close_request = {}, .on_closed = [this] { demo.cleanup(); }});
    window->set_animating(true);
    window->show();
}
void Gallery::command(sw::Command id) {
    switch (id) {
        case new_window: demo.add_window(); break;
        case pause_animation:
            animating = !animating;
            window->set_animating(animating);
            window->set_command_state(pause_animation, true, animating);
            break;
        case reset_animation:
            time = 0; speed = 1;
            window->request_redraw(); break;
        case controls_lab: demo.show_controls(); break;
        case open_inspector: show_inspector(); break;
        case open_sheet: show_sheet(); break;
        case close_window: window->close(); break;
        default: break;
    }
}
void Gallery::event(const sw::Event& input) {
    if (auto p = std::get_if<sw::PointerEvent>(&input)) {
        pointer = p->position;
        hovered = -1;
        if (p->kind != sw::PointerKind::leave) {
            for (int i = 0; i < 6; ++i) if (cards[i].contains((float)pointer.x, (float)pointer.y)) hovered = i;
        }
        if (p->kind == sw::PointerKind::scroll) speed = std::clamp(speed + p->scroll_y * .01, .1, 4.0);
        window->request_redraw();
    }
    if (auto k = std::get_if<sw::KeyEvent>(&input); k && k->down && !k->repeat) {
        if (k->key == sw::Key::space) command(pause_animation);
        if (k->key == sw::Key::r) command(reset_animation);
    }
}
void Gallery::draw(SkCanvas& canvas, const sw::FrameInfo& frame) {
    if (!canvas.recordingContext()) throw std::runtime_error("Gallery requires a GPU-backed SkCanvas");
    const double dt = std::min(frame.delta.count(), .1);
    if (animating) time += dt * speed;
    if (frame.delta.count() > 0) smoothed_interval = smoothed_interval == 0 ? frame.delta.count() :
        .8 * smoothed_interval + .2 * frame.delta.count();
    canvas.clear(background);
    const float width = (float)frame.logical_size.width;
    const float height = (float)frame.logical_size.height;
    auto& assets = demo.assets;
    text(canvas, assets.face, "SKIA / NATIVE WINDOW LAB", 28, 34, 12, mint);
    text(canvas, assets.face, "A canvas, accelerated.", 28, 82, 38);
    text(canvas, assets.face, "C++23  /  macOS 26+  /  GPU rendering in every window and dialog", 29, 110, 14, muted);
    fill(canvas, SkRect::MakeXYWH(width - 161, 26, 132, 30), 0xff234038, 15);
    text(canvas, assets.face, animating ? "METAL / LIVE" : "METAL / PAUSED", width - 145, 46, 11, mint);

    const int columns = width >= 950 ? 3 : 2;
    const int rows = 6 / columns;
    const float gap = 16, card_width = (width - 56 - gap * (columns - 1)) / columns;
    const float card_height = (height - 230 - gap * (rows - 1)) / rows;
    constexpr std::array names{"01  Geometry & paths", "02  Gradients & transforms", "03  Text & typography",
                               "04  Image & clipping", "05  Shadows & blending", "06  SkSL runtime shader"};
    for (int i = 0; i < 6; ++i) {
        cards[i] = SkRect::MakeXYWH(28 + (i % columns) * (card_width + gap),
            141 + (i / columns) * (card_height + gap), card_width, card_height);
        const auto rect = cards[i];
        fill(canvas, rect, hovered == i ? 0xff20343d : panel, 14);
        SkPaint stroke; stroke.setAntiAlias(true); stroke.setColor(hovered == i ? mint : border);
        stroke.setStyle(SkPaint::kStroke_Style); stroke.setStrokeWidth(1);
        canvas.drawRoundRect(rect, 14, 14, stroke);
        text(canvas, assets.face, names[i], rect.x() + 16, rect.y() + 28, 13, hovered == i ? mint : muted);
        canvas.save();
        canvas.clipRRect(SkRRect::MakeRectXY(rect.makeInset(1, 1), 13, 13), true);
        const float scale = std::min((card_width - 24) / 300, (card_height - 50) / 170);
        canvas.translate(rect.centerX() - 150 * scale, rect.y() + 43 + (card_height - 50 - 170 * scale) / 2);
        canvas.scale(scale, scale);
        draw_example(canvas, i);
        canvas.restore();
    }
    text(canvas, assets.face, demo.gpu, 28, height - 55, 12, mint);
    auto metrics = std::format("{:.0f} fps  /  {:.2f} ms frame interval  /  {:.0f}x scale  /  {:.1f}x speed",
        smoothed_interval > 0 ? 1 / smoothed_interval : 0, smoothed_interval * 1000, frame.scale, speed);
    text(canvas, assets.face, metrics, 28, height - 34, 12, muted);
    text(canvas, assets.face, status, 28, height - 13, 11, muted);

    // Explicit diagnostic mode only: no readback occurs in the normal frame path.
    if (frame.frame_number >= 30 && !capture_done && (demo.smoke || std::getenv("SKIA_WINDOW_SCREENSHOT"))) {
        capture_done = true;
        if (const char* output = std::getenv("SKIA_WINDOW_SCREENSHOT")) {
            SkBitmap bitmap;
            bitmap.allocN32Pixels((int)frame.pixel_size.width, (int)frame.pixel_size.height);
            if (!canvas.readPixels(bitmap, 0, 0)) throw std::runtime_error("Screenshot readback failed");
            SkFILEWStream stream(output);
            if (!SkPngEncoder::Encode(&stream, bitmap.pixmap(), {})) throw std::runtime_error("Screenshot encoding failed");
            std::cout << "Screenshot: " << output << std::endl;
        }
        if (demo.smoke) demo.application->post([this] { demo.application->quit(); });
    }
    if (inspector && inspector->is_open()) inspector->request_redraw();
}
void Gallery::draw_example(SkCanvas& canvas, int example) {
    const float t = (float)time;
    SkPaint paint; paint.setAntiAlias(true);
    switch (example) {
        case 0: {
            paint.setStyle(SkPaint::kStroke_Style); paint.setStrokeWidth(3); paint.setColor(mint);
            SkPathBuilder path;
            path.moveTo(15, 115).cubicTo(80, 10 + 25 * std::sin(t), 195, 155, 285, 45);
            canvas.drawPath(path.detach(), paint);
            for (int i = 0; i < 14; ++i) {
                const float angle = t * .6f + i * 2 * std::numbers::pi_v<float> / 14;
                paint.setStyle(SkPaint::kFill_Style);
                paint.setColor(i % 2 ? mint : blue);
                paint.setAlphaf(.35f + .65f * (i / 14.f));
                canvas.drawCircle(150 + 65 * std::cos(angle), 85 + 55 * std::sin(angle), 4 + i * .4f, paint);
            }
            break;
        }
        case 1: {
            canvas.translate(150, 85); canvas.rotate(t * 18);
            const SkPoint points[]{{-80, -55}, {80, 55}};
            const SkColor4f colors[]{SkColor4f::FromColor(mint), SkColor4f::FromColor(blue), SkColor4f::FromColor(0xffc291ff)};
            paint.setShader(SkShaders::LinearGradient(points, SkGradient({colors, SkTileMode::kClamp}, {})));
            canvas.drawRoundRect(SkRect::MakeXYWH(-85, -55, 170, 110), 22, 22, paint);
            paint.setShader(nullptr); paint.setColor(0x55ffffff); paint.setStyle(SkPaint::kStroke_Style); paint.setStrokeWidth(2);
            canvas.drawRoundRect(SkRect::MakeXYWH(-105, -70, 210, 140), 30, 30, paint);
            break;
        }
        case 2:
            text(canvas, demo.assets.face, "Aa", 20, 102, 100, mint);
            text(canvas, demo.assets.face, "Skia", 160, 74, 40);
            text(canvas, demo.assets.face, "Pixels with personality.", 162, 100, 12, muted);
            text(canvas, demo.assets.face, "Roboto / bundled offline", 23, 145, 12, muted);
            break;
        case 3: {
            canvas.save();
            canvas.translate(150, 82); canvas.rotate(std::sin(t * .5f) * 10);
            canvas.clipRRect(SkRRect::MakeRectXY(SkRect::MakeXYWH(-105, -66, 210, 132), 35, 35), true);
            canvas.drawImageRect(demo.assets.image, SkRect::MakeXYWH(-105, -66, 210, 132), SkSamplingOptions(SkFilterMode::kLinear));
            paint.setColor(0x99ffffff); paint.setStyle(SkPaint::kStroke_Style); paint.setStrokeWidth(2);
            canvas.drawCircle(std::sin(t) * 40, 0, 42, paint);
            canvas.restore();
            break;
        }
        case 4: {
            paint.setColor(0xaa000000);
            paint.setImageFilter(SkImageFilters::Blur(9, 9, nullptr));
            canvas.drawRoundRect(SkRect::MakeXYWH(55, 53, 190, 95), 20, 20, paint);
            paint.setImageFilter(nullptr);
            paint.setColor(mint); canvas.drawCircle(115 + 15 * std::sin(t), 79, 48, paint);
            paint.setBlendMode(SkBlendMode::kScreen); paint.setColor(0xbb826bff);
            canvas.drawCircle(179, 91, 48, paint);
            break;
        }
        case 5: {
            SkRuntimeShaderBuilder builder(demo.assets.effect);
            builder.uniform("time") = t;
            builder.uniform("resolution") = SkV2{300, 170};
            paint.setShader(builder.makeShader());
            if (!paint.getShader()) throw std::runtime_error("Runtime shader instantiation failed");
            canvas.drawRoundRect(SkRect::MakeWH(300, 170), 12, 12, paint);
            break;
        }
    }
}
void Gallery::show_inspector() {
    if (inspector && inspector->is_open()) { inspector->focus(); return; }
    auto created = sw::Dialog::create(*window, {.title = "Modeless Inspector", .size = {500, 310}, .minimum_size = {460, 280}});
    if (!created) throw std::runtime_error(created.error().message);
    inspector = std::move(*created);
    inspector->set_callbacks({.on_draw = [this](SkCanvas& canvas, const sw::FrameInfo& frame) {
        canvas.clear(background);
        text(canvas, demo.assets.face, "MODELESS / GPU CANVAS", 24, 32, 12, mint);
        text(canvas, demo.assets.face, "Keep exploring.", 24, 77, 30);
        text(canvas, demo.assets.face, "The gallery remains interactive behind this dialog.", 24, 108, 13, muted);
        text(canvas, demo.assets.face, std::format("Animation speed: {:.1f}x", speed), 24, 149, 15);
        text(canvas, demo.assets.face, std::format("Hovered card: {}", hovered < 0 ? "none" : std::to_string(hovered + 1)), 24, 177, 15);
        text(canvas, demo.assets.face, std::format("Drawable: {:.0f} x {:.0f} px", frame.pixel_size.width, frame.pixel_size.height), 24, 205, 13, muted);
        fill(canvas, SkRect::MakeXYWH(24, (float)frame.logical_size.height - 67, 145, 40), 0xff24463d, 10);
        text(canvas, demo.assets.face, "Close / Escape", 42, (float)frame.logical_size.height - 42, 13, mint);
    }, .on_event = [this](const sw::Event& input) {
        if (auto k = std::get_if<sw::KeyEvent>(&input); k && k->down && k->key == sw::Key::escape) inspector->close();
        if (auto p = std::get_if<sw::PointerEvent>(&input); p && p->kind == sw::PointerKind::up &&
            p->position.x >= 24 && p->position.x <= 169 && p->position.y >= inspector->content_size().height - 67 &&
            p->position.y <= inspector->content_size().height - 27) inspector->close();
    }, .on_close_request = {}, .on_closed = {}});
    inspector->show();
}
void Gallery::show_sheet() {
    if (sheet && sheet->is_open()) { sheet->focus(); return; }
    auto created = sw::Dialog::create(*window, {.title = "Modal Example", .size = {510, 320}, .minimum_size = {510, 320}, .resizable = false});
    if (!created) throw std::runtime_error(created.error().message);
    sheet = std::move(*created);
    sheet->set_callbacks({.on_draw = [this](SkCanvas& canvas, const sw::FrameInfo& frame) {
        canvas.clear(background);
        text(canvas, demo.assets.face, "PARENT-MODAL / METAL", 26, 35, 12, mint);
        text(canvas, demo.assets.face, "One window takes a break.", 26, 82, 29);
        text(canvas, demo.assets.face, "Other gallery windows keep accepting input.", 26, 113, 14, muted);
        SkPaint paint; paint.setAntiAlias(true); paint.setColor(mint);
        const float x = 255 + 105 * std::sin((float)frame.elapsed.count() * 2);
        canvas.drawCircle(x, 174, 17, paint);
        fill(canvas, SkRect::MakeXYWH(26, 248, 145, 42), 0xff283b47, 10);
        fill(canvas, SkRect::MakeXYWH(338, 248, 145, 42), 0xff24463d, 10);
        text(canvas, demo.assets.face, "Cancel / Esc", 47, 275, 13, muted);
        text(canvas, demo.assets.face, "Accept / Enter", 355, 275, 13, mint);
    }, .on_event = [this](const sw::Event& input) {
        if (auto k = std::get_if<sw::KeyEvent>(&input); k && k->down) {
            if (k->key == sw::Key::escape) sheet->dismiss();
            if (k->key == sw::Key::enter) sheet->dismiss(sw::DialogResult::accepted);
        }
        if (auto p = std::get_if<sw::PointerEvent>(&input); p && p->kind == sw::PointerKind::up && p->button == 0 &&
            p->position.y >= 248 && p->position.y <= 290) {
            if (p->position.x >= 26 && p->position.x <= 171) sheet->dismiss();
            if (p->position.x >= 338 && p->position.x <= 483) sheet->dismiss(sw::DialogResult::accepted);
        }
    }, .on_close_request = {}, .on_closed = {}});
    sheet->set_animating(true);
    auto result = sheet->show_modal([this](sw::DialogResult result) {
        status = result == sw::DialogResult::accepted ? "Modal dialog accepted." : "Modal dialog cancelled.";
        window->request_redraw();
    });
    if (!result) throw std::runtime_error(result.error().message);
}
} // namespace

int main(int argc, char** argv) {
    try {
        if (argc > 1 && std::string_view(argv[1]) == "--benchmark-controls") {
            Assets assets;
            return demo::benchmark_controls(assets.face, assets.image, assets.effect);
        }
        if (argc > 2 && std::string_view(argv[1]) == "--render-controls") {
            Assets assets;
            const double width = argc > 3 ? std::stod(argv[3]) : 1220;
            const int panel = argc > 4 ? std::stoi(argv[4]) : 0;
            if (!std::isfinite(width) || width < 300 || width > 4096) throw std::invalid_argument("Preview width must be 300..4096");
            demo::render_controls_preview(assets.face, assets.image, assets.effect, argv[2], {width,900}, panel);
            std::cout << "Controls preview: " << argv[2] << std::endl;
            return 0;
        }
        const bool smoke = argc > 1 && std::string_view(argv[1]) == "--smoke-test";
        Demo demo(smoke);
        demo.add_window();
        if (!smoke) demo.show_controls();
        const int result = demo.application->run();
        return demo.failed ? 1 : result;
    } catch (const std::exception& error) {
        std::cerr << "Skia Gallery: " << error.what() << '\n';
        return 1;
    }
}
