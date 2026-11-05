#include "controls.hpp"
#include "include/core/SkCanvas.h"
#include "include/core/SkBitmap.h"
#include "include/core/SkSurface.h"
#include "include/core/SkStream.h"
#include "include/encode/SkPngEncoder.h"
#include "include/core/SkM44.h"
#include "include/core/SkPaint.h"
#include "include/effects/SkRuntimeEffect.h"
#include <array>
#include <cmath>
#include <format>
#include <iostream>

namespace demo {
namespace ui = skia_window::ui;
namespace sw = skia_window;
using namespace ui::literals;
using namespace std::chrono_literals;
namespace {
constexpr std::uint32_t ink = 0xff10191f, panel = 0xff192a33, mint = 0xff66e0c2, muted = 0xff91a9b6;
enum class Panel { editor, metrics, collection };
struct Item {
    int id;
    std::string title;
    friend bool operator==(const Item&, const Item&) = default;
};

struct LocalCounter {
    ui::State<int> value{0};
    ui::View body(const std::string& title) {
        return ui::Row{ui::Text(title).foreground(muted).grow(1),
                       ui::Text(std::to_string(value.get()))
                           .key("local-value")
                           .width(28_dp)
                           .foreground(mint)
                           .content_transition(ui::ContentTransition::numeric_text)
                           .animation(ui::Animation::ease_out(200ms)),
                       ui::Button("+", [binding = value.binding()] { binding.set(binding.get() + 1); })
                           .key("local-plus")}
            .gap(8_dp)
            .align_items(ui::Align::center);
    }
};
struct ControlsView {
    sw::Window* window;
    sk_sp<SkImage> image;
    sk_sp<SkRuntimeEffect> shader;
    std::string gpu;
    sk_sp<SkTypeface> face;
    std::unique_ptr<sw::Dialog> dialog;
    ui::State<Panel> selected{Panel::editor};
    ui::State<double> speed{1};
    ui::State<bool> playing{true}, extra{false}, disabled{false}, saved{false}, reduced{false};
    ui::State<int> count{0}, effect{0};
    ui::State<std::vector<Item>> items{{{1, "Geometry"}, {2, "Gradients"}, {3, "Typography"}, {4, "Images"}}};
    int next_id{5};
    double time{};

    void select(Panel value) {
        ui::animate(ui::Animation::ease_in_out(350ms), [&] { selected.set(value); });
    }
    ui::Transition transition() {
        switch (effect.get()) {
        case 1:
            return ui::Transition::push(ui::Edge::trailing).combined_with(ui::Transition::fade());
        case 2:
            return ui::Transition::scale(.9).combined_with(ui::Transition::fade());
        default:
            return ui::Transition::fade();
        }
    }
    ui::View badge(std::string label) {
        return ui::Text(std::move(label))
            .font_size(11_dp)
            .foreground(mint)
            .padding(ui::Edges::symmetric(7_dp, 12_dp))
            .background(0xff244039)
            .corner_radius(14);
    }
    ui::View tab(std::string label, Panel value) {
        return ui::Button(std::move(label), [this, value] { select(value); })
            .key("tab-" + ui::identity(value))
            .foreground(selected.get() == value ? mint : 0xffeaf4f6)
            .background(selected.get() == value ? 0xff244039 : panel);
    }
    ui::View preview() {
        const bool animate = playing.get();
        const double rate = speed.get();
        return ui::Canvas([this, animate, rate](SkCanvas& canvas, const ui::PaintInfo& frame) {
                   if (animate)
                       time += std::min(.05, frame.delta.count()) * rate;
                   SkRuntimeShaderBuilder builder(shader);
                   builder.uniform("time") = (float)time;
                   builder.uniform("resolution") =
                       SkV2{(float)frame.content_size.width, (float)frame.content_size.height};
                   SkPaint paint;
                   paint.setAntiAlias(true);
                   paint.setShader(builder.makeShader());
                   canvas.drawRoundRect(
                       SkRect::MakeWH((float)frame.content_size.width, (float)frame.content_size.height), 12,
                       12, paint);
                   paint.setShader(nullptr);
                   paint.setColor(0xaaeaf4f6);
                   canvas.drawCircle((float)(frame.content_size.width * (.5 + .23 * std::sin(time))),
                                     (float)(frame.content_size.height * .5), 18, paint);
               })
            .key("preview-canvas")
            .height(210_dp)
            .grow(1)
            .animating(animate)
            .accessible_label("Animated Metal shader preview");
    }
    void show_dialog() {
        if (dialog && dialog->is_open()) {
            dialog->focus();
            return;
        }
        auto result = sw::Dialog::create(*window, {.title = "Declarative Controls Dialog",
                                                   .size = {460, 280},
                                                   .minimum_size = {460, 280},
                                                   .resizable = false});
        if (!result)
            throw std::runtime_error(result.error().message);
        dialog = std::move(*result);
        auto* target = dialog.get();
        target->set_content(ui::Column{
            ui::Text("The same view system, in a sheet.").font_size(22_dp),
            ui::CheckBox("Animate preview", playing.binding()).key("dialog-play"),
            ui::Slider(speed.binding(), {.minimum = .1, .maximum = 4, .step = .1})
                .accessible_label("Shared animation speed"),
            ui::Text("Bindings are shared with the parent controls.").foreground(muted).font_size(13_dp),
            ui::Row{ui::Spacer{}, ui::Button("Cancel", [target] { target->dismiss(); }),
                    ui::Button("Accept", [target] { target->dismiss(sw::DialogResult::accepted); })
                        .foreground(mint)}
                .gap(10_dp),
        }
                                .gap(14_dp)
                                .padding(22_dp)
                                .background(ink));
        ui::Theme theme;
        theme.typeface = face;
        target->ui_host()->set_theme(std::move(theme));
        target->set_callbacks({.on_draw = {},
                               .on_event =
                                   [target](const sw::Event& event) {
                                       if (auto key = std::get_if<sw::KeyEvent>(&event); key && key->down) {
                                           if (key->key == sw::Key::escape)
                                               target->dismiss();
                                           if (key->key == sw::Key::enter)
                                               target->dismiss(sw::DialogResult::accepted);
                                       }
                                   },
                               .on_close_request = {},
                               .on_closed = {}});
        auto shown = target->show_modal(
            [binding = saved.binding()](auto result) { binding.set(result == sw::DialogResult::accepted); });
        if (!shown)
            throw std::runtime_error(shown.error().message);
    }
    ui::View editor() {
        return ui::Column{
            ui::Row{ui::Text("Live preview").font_size(21_dp).font_weight(ui::FontWeight::semibold),
                    ui::Spacer{}, badge("SKIA / METAL")}
                .align_items(ui::Align::center),
            preview(),
            ui::Text(
                "The slider updates a binding. The canvas shares the same layout and GPU as the controls.")
                .foreground(muted)
                .font_size(13_dp),
            ui::Row{
                ui::Button(
                    "Save preset",
                    [this] { ui::animate(ui::Animation::ease_out(200ms), [&] { saved.set(!saved.get()); }); })
                    .key("save-preset")
                    .enabled(!disabled.get()),
                ui::Text(saved.get() ? "Preset saved" : "Unsaved changes")
                    .key("save-status")
                    .foreground(saved.get() ? mint : muted)
                    .content_transition(ui::ContentTransition::cross_fade),
                ui::Spacer{},
                ui::Button("+1",
                           [this] {
                               ui::animate(ui::Animation::spring(400ms), [&] { count.set(count.get() + 1); });
                           })
                    .key("increment"),
                ui::Text(std::format("{}", count.get()))
                    .key("count")
                    .font_size(24_dp)
                    .width(55_dp)
                    .content_transition(ui::ContentTransition::numeric_text),
            }
                .gap(12_dp)
                .align_items(ui::Align::center)
                .wrap(ui::Wrap::wrap),
        }
            .gap(16_dp)
            .padding(20_dp)
            .background(panel)
            .key("editor-panel");
    }
    ui::View metrics() {
        auto card = [&](std::string title, std::string value, std::uint32_t color) {
            return ui::Column{ui::Text(std::move(title)).foreground(muted).font_size(12_dp),
                              ui::Text(std::move(value)).font_size(30_dp).foreground(color)}
                .gap(8_dp)
                .padding(18_dp)
                .background(0xff223640);
        };
        auto footer = ui::Row{ui::Text("Grid placement: this footer spans both columns.")
                                  .font_size(12_dp)
                                  .foreground(muted)}
                          .item({.grid = {.row = 1, .column = 0, .column_span = 2}});
        return ui::Column{
            ui::Text("Auto layout, real controls").font_size(21_dp).font_weight(ui::FontWeight::semibold),
            ui::Grid({.columns = {1_fr, 1_fr}, .column_gap = 12_dp, .row_gap = 12_dp},
                     {
                         card("Animation speed", std::format("{:.1f}x", speed.get()), mint),
                         card("Reactive counter", std::to_string(count.get()), 0xffa798ff),
                         footer,
                     }),
            ui::Overlay{
                ui::Image(image).width(110_dp).height(90_dp).opacity(.8),
                badge("OVERLAY").item(
                    {.overlay = {.horizontal = ui::Align::end, .vertical = ui::Align::end}}),
            }
                .height(100_dp)
                .background(0xff14232a)
                .padding(12_dp),
            ui::Text("Resize this window: the two-column workspace becomes a vertical stack. No coordinates "
                     "are stored in the view definition.")
                .foreground(muted)
                .font_size(13_dp),
        }
            .gap(16_dp)
            .padding(20_dp)
            .background(panel)
            .key("metrics-panel");
    }
    ui::View collection() {
        return ui::Column{
            ui::Row{
                ui::Text("Stable identities").font_size(21_dp),
                ui::Spacer{},
                ui::Button("Reverse",
                           [this] {
                               auto value = items.get();
                               std::reverse(value.begin(), value.end());
                               ui::animate(ui::Animation::ease_in_out(250ms),
                                           [&] { items.set(std::move(value)); });
                           })
                    .key("reverse-items"),
            }
                .gap(12_dp),
            ui::Text("Tab through rows, then reorder or add items. Focus follows each item's ID. Scroll the "
                     "list independently.")
                .foreground(muted)
                .font_size(13_dp),
            ui::ScrollView(
                ui::for_each(items.get(), &Item::id,
                             [this](const Item& item) {
                                 return ui::Row{
                                     ui::Text(std::format("{:02d}  {}", item.id, item.title)).grow(1),
                                     ui::Button("Remove",
                                                [this, id = item.id] {
                                                    auto value = items.get();
                                                    std::erase_if(value, [&](const Item& item) {
                                                        return item.id == id;
                                                    });
                                                    ui::animate(ui::Animation::ease_out(220ms),
                                                                [&] { items.set(std::move(value)); });
                                                })
                                         .key("remove"),
                                 }
                                     .gap(10_dp)
                                     .align_items(ui::Align::center)
                                     .padding(10_dp)
                                     .margin(ui::Edges::symmetric(4_dp, 0_dp))
                                     .background(0xff233943)
                                     .transition(ui::Transition::fade())
                                     .animate_layout();
                             }))
                .height(260_dp)
                .key("collection-scroll"),
            ui::Button("Add item",
                       [this] {
                           auto value = items.get();
                           value.push_back({next_id++, "New layer"});
                           ui::animate(ui::Animation::ease_out(220ms), [&] { items.set(std::move(value)); });
                       })
                .key("add-item"),
        }
            .gap(16_dp)
            .padding(20_dp)
            .background(panel)
            .key("collection-panel");
    }
    ui::View sidebar() {
        return ui::ScrollView(
                   ui::Column{
                       ui::Text("Bindings & state").font_size(18_dp).font_weight(ui::FontWeight::semibold),
                       ui::CheckBox("Animate preview", playing.binding()).key("play"),
                       ui::Row{
                           ui::Text("Speed").foreground(muted), ui::Spacer{},
                           ui::Text(std::format("{:.1f}x", speed.get())).foreground(mint).key("speed-label")},
                       ui::Slider(speed.binding(), {.minimum = .1, .maximum = 4, .step = .1})
                           .key("speed")
                           .accessible_label("Animation speed"),
                       ui::CheckBox("Show details", extra.binding())
                           .key("details")
                           .animation(ui::Animation::ease_in_out(250ms)),
                       ui::CheckBox("Disable Save", disabled.binding()).key("disable-save"),
                       ui::component<LocalCounter>("sidebar-counter", std::string("Nested view state")),
                       ui::Text("Replacement transition").foreground(muted).font_size(12_dp),
                       ui::Row{
                           ui::Button("Fade", [this] { effect.set(0); }).key("fade"),
                           ui::Button("Push", [this] { effect.set(1); }).key("push"),
                           ui::Button("Scale", [this] { effect.set(2); }).key("scale"),
                       }
                           .gap(6_dp)
                           .wrap(ui::Wrap::wrap),
                       ui::Button(reduced.get() ? "Motion: reduced" : "Motion: system default",
                                  [this] {
                                      reduced.set(!reduced.get());
                                      window->ui_host()->set_reduced_motion(reduced.get());
                                  })
                           .key("reduced-motion"),
                       ui::Text("Keyboard: Tab / Shift-Tab, Space or Enter, and arrow keys on sliders.")
                           .font_size(12_dp)
                           .foreground(muted),
                   }
                       .gap(12_dp)
                       .padding(18_dp))
            .height(500_dp)
            .background(panel)
            .key("settings-scroll");
    }
    ui::View body() {
        return ui::ScrollView(
                   ui::Column{
                       ui::Text("C++23 / DECLARATIVE CONTROLS LAB").font_size(12_dp).foreground(mint),
                       ui::Row{ui::Text("Describe it. Let it move.")
                                   .font_size(32_dp)
                                   .font_weight(ui::FontWeight::semibold),
                               ui::Spacer{}, badge("GPU UI")}
                           .align_items(ui::Align::center)
                           .gap(12_dp)
                           .wrap(ui::Wrap::wrap),
                       ui::Text("SwiftUI-inspired composition, web-inspired layout, and interruptible "
                                "transitions on Skia.")
                           .foreground(muted),
                       ui::Row{tab("Editor", Panel::editor), tab("Layout", Panel::metrics),
                               tab("Collection", Panel::collection), ui::Spacer{},
                               ui::Button("Open dialog", [this] { show_dialog(); }).key("open-dialog")}
                           .gap(10_dp)
                           .wrap(ui::Wrap::wrap),
                       ui::Container(ui::Flex{.direction = ui::Direction::column, .gap = 16_dp},
                                     {
                                         sidebar(),
                                         ui::ContentSwitcher(selected.get(),
                                                             [this](Panel panel) -> ui::View {
                                                                 switch (panel) {
                                                                 case Panel::editor:
                                                                     return editor();
                                                                 case Panel::metrics:
                                                                     return metrics();
                                                                 case Panel::collection:
                                                                     return collection();
                                                                 }
                                                                 return ui::Empty{};
                                                             })
                                             .key("panel")
                                             .transition(transition())
                                             .size_transition(ui::SizeTransition::animate)
                                             .min_height(330_dp),
                                     })
                           .responsive_layout(900_dp,
                                              ui::GridLayout{.columns = {280_dp, ui::minmax(0_dp, 1_fr)},
                                                             .column_gap = 18_dp}),
                       ui::when(extra.get(),
                                [] {
                                    return ui::Column{
                                        ui::Text("This panel is conditional content.")
                                            .foreground(mint)
                                            .font_size(18_dp),
                                        ui::Text("Its structural slot stays stable. Insertion/removal "
                                                 "changes the measured height and surrounding layout.")
                                            .foreground(muted),
                                    }
                                        .gap(8_dp)
                                        .padding(18_dp)
                                        .background(0xff213a35);
                                })
                           .key("extra-details")
                           .transition(ui::Transition::fade())
                           .size_transition(ui::SizeTransition::animate)
                           .animation(ui::Animation::ease_in_out(250ms)),
                       ui::Text(gpu + " / Tab to explore / Resize to reflow")
                           .foreground(muted)
                           .font_size(12_dp),
                   }
                       .gap(16_dp)
                       .padding(24_dp))
            .background(ink);
    }
};
} // namespace
int benchmark_controls(sk_sp<SkTypeface> face, sk_sp<SkImage> image, sk_sp<SkRuntimeEffect> shader) {
    auto model =
        std::make_shared<ControlsView>(nullptr, std::move(image), std::move(shader), "CPU benchmark");
    ui::Theme theme;
    theme.typeface = std::move(face);
    ui::Host host([model] { return model->body(); }, {}, std::move(theme));
    // A no-pixels canvas isolates body/layout/paint recording costs from GPU execution and presentation.
    SkCanvas canvas(1400, 1000);
    bool passed = true;
    double time = 0;
    for (int scenario = 0; scenario < 3; ++scenario) {
        std::vector<double> milliseconds;
        for (int i = 0; i < 150; ++i) {
            const sw::Size size{scenario == 0 ? 1220.0 : 760.0 + (i % 80) * 8, 900};
            const auto start = std::chrono::steady_clock::now();
            if (scenario != 0)
                host.handle(sw::ResizeEvent{size, size, 1});
            if (scenario == 2 && i % 30 == 0)
                ui::animate(ui::Animation::ease_in_out(350ms),
                            [&] { model->selected.set(static_cast<Panel>((i / 30) % 3)); });
            host.paint(canvas, sw::FrameInfo{.logical_size = size,
                                             .pixel_size = size,
                                             .elapsed = std::chrono::duration<double>(time),
                                             .delta = 1s / 120.});
            time += 1. / 120;
            if (i >= 10)
                milliseconds.push_back(
                    std::chrono::duration<double, std::milli>(std::chrono::steady_clock::now() - start)
                        .count());
        }
        std::ranges::sort(milliseconds);
        const double p95 = milliseconds[milliseconds.size() * 95 / 100];
        std::cout << std::format("{}: CPU p50 {:.3f} ms, p95 {:.3f} ms, max {:.3f} ms\n",
                                 scenario == 0   ? "Steady animation"
                                 : scenario == 1 ? "Resizing"
                                                 : "Resize + transitions",
                                 milliseconds[milliseconds.size() / 2], p95, milliseconds.back());
        // A diagnostic budget, not a machine-independent CTest threshold. Leaves part of an 8.33 ms
        // frame for GPU encoding/submission and other windows on a 120 Hz display.
        passed &= p95 < 4;
    }
    return passed ? 0 : 1;
}
void render_controls_preview(sk_sp<SkTypeface> face, sk_sp<SkImage> image, sk_sp<SkRuntimeEffect> shader,
                             const char* output, sw::Size size, int panel_index) {
    auto model = std::make_shared<ControlsView>(nullptr, std::move(image), std::move(shader),
                                                "Offline layout preview / normal app uses Metal");
    model->selected.set(static_cast<Panel>(std::clamp(panel_index, 0, 2)));
    ui::Theme theme;
    theme.typeface = std::move(face);
    ui::Host host([model] { return model->body(); }, {}, std::move(theme));
    auto surface = SkSurfaces::Raster(SkImageInfo::MakeN32Premul((int)size.width, (int)size.height));
    if (!surface)
        throw std::runtime_error("Cannot allocate preview surface");
    host.paint(*surface->getCanvas(), sw::FrameInfo{.logical_size = size, .pixel_size = size});
    SkBitmap bitmap;
    bitmap.allocN32Pixels((int)size.width, (int)size.height);
    if (!surface->readPixels(bitmap, 0, 0))
        throw std::runtime_error("Preview readback failed");
    SkFILEWStream stream(output);
    if (!SkPngEncoder::Encode(&stream, bitmap.pixmap(), {}))
        throw std::runtime_error("Cannot write preview PNG");
}
std::unique_ptr<ControlsLab> ControlsLab::create(sw::Application& app, sk_sp<SkTypeface> face,
                                                 sk_sp<SkImage> image, sk_sp<SkRuntimeEffect> shader,
                                                 std::string gpu) {
    auto lab = std::make_unique<ControlsLab>();
    auto result = sw::Window::create(
        app, {.title = "Skia Controls Lab", .size = {1220, 900}, .minimum_size = {760, 650}});
    if (!result)
        throw std::runtime_error(result.error().message);
    lab->window = std::move(*result);
    auto model = lab->window->set_view<ControlsView>(lab->window.get(), std::move(image), std::move(shader),
                                                     std::move(gpu));
    model->face = face;
    ui::Theme theme;
    theme.typeface = std::move(face);
    lab->window->ui_host()->set_theme(std::move(theme));
    const std::array menu{
        sw::MenuItem::submenu("File", {sw::MenuItem::action(1, "Close Controls Lab", {sw::Key::w})})};
    auto* window = lab->window.get();
    auto menu_result = window->set_menu(menu, [window](auto) { window->close(); });
    if (!menu_result)
        throw std::runtime_error(menu_result.error().message);
    window->show();
    return lab;
}
} // namespace demo
