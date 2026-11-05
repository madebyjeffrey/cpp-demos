#include "include/core/SkCanvas.h"
#include "include/core/SkSurface.h"
#include <cmath>
#include <iostream>
#include <functional>
#include <memory>
#include <limits>
#include <stdexcept>
#include <string>
import scars;
namespace s = scars;
namespace {
int checks{};
void require(bool value, const char* message) {
    ++checks;
    if (!value)
        throw std::runtime_error(message);
}
s::Theme theme(bool light = false) {
    auto t = light ? s::Theme::light() : s::Theme{};
    t.font_path = SCARS_FONT_PATH;
    return t;
}
s::InspectionNode keyed(s::Host& h, const std::string& key) {
    for (auto n : h.inspect())
        if (n.key == key)
            return n;
    throw std::runtime_error("Missing key " + key);
}
s::Element element(s::Host& h, const std::string& label, s::Role role = s::Role::text_field) {
    for (auto n : h.elements())
        if (n.label == label && n.role == role)
            return n;
    throw std::runtime_error("Missing element " + label);
}
bool popover(s::Host& h) {
    for (const auto& n : h.inspect())
        if (n.type == "Popover")
            return true;
    return false;
}
void conversions() {
    require(s::format_color(0x804488cc) == "#4488CC80" && s::format_color(0x804488cc, false) == "#4488CC",
            "Hex display explicitly reorders packed ARGB into CSS RGBA");
    require(s::parse_color("#4488cc80") == 0x804488cc && s::parse_color("4488CC") == 0xff4488cc,
            "Long CSS hex preserves alpha and defaults to opaque RGB");
    require(s::parse_color(" #abc8 ") == 0x88aabbcc && s::parse_color("#abc") == 0xffaabbcc,
            "Short hex expands each nibble");
    require(!s::parse_color("#") && !s::parse_color("#abcdefghi") && !s::parse_color("#ggg") &&
                !s::parse_color("#12345"),
            "Invalid hex syntax cannot silently truncate");
    bool roundtrip = true;
    for (unsigned r = 0; r < 256; r += 17)
        for (unsigned g = 0; g < 256; g += 17)
            for (unsigned b = 0; b < 256; b += 17) {
                const s::Color color = 0x7f000000 | (r << 16) | (g << 8) | b;
                roundtrip &= s::color_from_hsv(s::color_hsv(color), 0x7f) == color &&
                             s::parse_color(s::format_color(color)) == color;
            }
    require(roundtrip, "4096 sRGB/HSV/hex round trips preserve all channels including alpha");
    require(s::color_from_hsv({360, 1, 1}, 0) == 0x00ff0000 && s::color_hsv(0xff808080).saturation == 0,
            "Hue wraps at 360 and achromatic conversion is defined");
    bool rejected{};
    try {
        s::color_from_hsv({0, 2, 1});
    } catch (const std::invalid_argument&) {
        rejected = true;
    }
    require(rejected, "HSV conversion rejects out-of-range components");
    rejected = false;
    try {
        s::color_from_hsv({std::numeric_limits<double>::quiet_NaN(), 0, 1});
    } catch (const std::invalid_argument&) {
        rejected = true;
    }
    require(rejected, "HSV conversion rejects non-finite hue");
}
void drafts() {
    s::State<s::Color> value{0x804488cc};
    s::State<bool> hidden{false}, enabled{true}, mounted{true}, alpha{true}, readonly{false};
    int writes{};
    bool accept = true, normalize = false;
    s::Binding<s::Color> binding{[&] { return value.get(); },
                                 [&](s::Color color) {
                                     ++writes;
                                     if (accept)
                                         value.set(normalize ? 0xff102030 : color);
                                 }};
    s::Host h(
        [&] {
            return s::Block{
                s::ColorPicker("Paint", binding, {.alpha = alpha.get(), .read_only = readonly.get()})
                    .key("picker")
                    .hidden(hidden.get())
                    .visible(mounted.get())
                    .enabled(enabled.get()),
                s::Button("Outside", [] {}).key("outside")}
                .padding(12)
                .gap(12);
        },
        {}, theme());
    h.layout({380, 800});
    const auto id = keyed(h, "picker").id;
    require(writes == 0 && !h.color_validation(id)->dirty,
            "Constructing the picker never writes application state");
    auto red = element(h, "Red").id;
    h.focus(red);
    h.set_text(red, "255");
    h.commit_numeric(red);
    require(value.get() == 0x804488cc && writes == 0 && keyed(h, "picker").color->draft == 0x80ff88cc,
            "Component commits update the shared draft only");
    h.focus(keyed(h, "outside").id);
    require(h.color_validation(id)->dirty && value.get() == 0x804488cc,
            "Default draft survives focus outside until Apply or Cancel");
    require(h.apply_color(id) && writes == 1 && value.get() == 0x80ff88cc && !h.color_validation(id)->dirty,
            "Apply publishes one complete color");
    auto hex = element(h, "Hex").id;
    h.focus(hex);
    h.set_text(hex, "#12345");
    require(!h.apply_color(id) && value.get() == 0x80ff88cc &&
                h.color_validation(id)->status != s::NumericStatus::valid,
            "Incomplete hex blocks Apply and preserves the binding");
    h.cancel_color(id);
    h.focus(hex);
    h.set_text(hex, "#abcdef20");
    require(h.apply_color(id) && value.get() == 0x20abcdef && writes == 2,
            "Apply collects a pending hex editor before publication");
    h.focus(red);
    h.set_text(red, "20");
    h.commit_numeric(red);
    h.cancel_color(id);
    require(!h.color_validation(id)->dirty && value.get() == 0x20abcdef && writes == 2,
            "Cancel restores the draft without a default binding write");
    h.focus(red);
    h.set_text(red, "30");
    h.commit_numeric(red);
    value.set(0xff020406);
    h.layout({380, 800});
    require(!h.color_validation(id)->dirty && keyed(h, "picker").color->source == 0xff020406,
            "External updates replace a pending draft without stale overwrite");
    h.focus(red);
    h.set_text(red, "40");
    h.commit_numeric(red);
    accept = false;
    require(!h.apply_color(id) && h.color_validation(id)->status == s::NumericStatus::invalid &&
                value.get() == 0xff020406,
            "Rejected setters retain an error and the draft");
    accept = true;
    normalize = true;
    require(h.apply_color(id) && value.get() == 0xff102030 && !h.color_validation(id)->dirty,
            "Explicit retry reads application normalization back into the draft");
    normalize = false;
    alpha.set(false);
    h.layout({380, 800});
    hex = element(h, "Hex").id;
    h.focus(hex);
    h.set_text(hex, "#abcdef11");
    require(!h.apply_color(id), "Alpha-disabled hex rejects explicit alpha input");
    h.cancel_color(id);
    h.focus(hex);
    h.set_text(hex, "#123456");
    h.apply_color(id);
    require(value.get() == 0xff123456, "Alpha-disabled edits preserve the source alpha");
    value.set(0x40123456);
    h.layout({380, 800});
    h.focus(hex);
    h.set_text(hex, "#00ff00");
    h.apply_color(id);
    require(value.get() == 0x4000ff00,
            "Disabled alpha preserves translucent sources rather than forcing opacity");
    auto stage = [&] {
        red = element(h, "Red").id;
        h.focus(red);
        h.set_text(red, "100");
        h.commit_numeric(red);
    };
    stage();
    hidden.set(true);
    h.layout({380, 800});
    hidden.set(false);
    h.layout({380, 800});
    require(!h.color_validation(id)->dirty, "Hide/show retires staged color and text edits");
    stage();
    enabled.set(false);
    h.layout({380, 800});
    enabled.set(true);
    h.layout({380, 800});
    require(!h.color_validation(id)->dirty, "Disabling the picker retires its draft");
    stage();
    h.blur();
    h.set_active(true);
    h.layout({380, 800});
    require(!h.color_validation(id)->dirty, "Window deactivation retires unfinished color changes");
    stage();
    mounted.set(false);
    h.layout({380, 800});
    mounted.set(true);
    h.layout({380, 800});
    require(!h.color_validation(keyed(h, "picker").id)->dirty,
            "Unmounting does not retain a stale color draft");
    readonly.set(true);
    h.layout({380, 800});
    require(!h.apply_color(id) && element(h, "Red").enabled && element(h, "Red").read_only,
            "Read-only fields permit selection but reject Apply");
}
void live_and_popup() {
    s::State<s::Color> value{0x80507090};
    s::State<bool> mounted{true}, hidden{false};
    int writes{};
    s::Binding<s::Color> binding{[&] { return value.get(); },
                                 [&](s::Color color) {
                                     ++writes;
                                     value.set(color);
                                 }};
    s::Host h(
        [&] {
            return s::Block{s::ColorWell("Paint", binding, {.live_preview = true})
                                .key("well")
                                .visible(mounted.get())
                                .hidden(hidden.get()),
                            s::Button("Behind", [] {}).key("behind")}
                .padding(12)
                .gap(12);
        },
        {}, theme());
    h.layout({400, 760});
    auto id = keyed(h, "well").id;
    require(!popover(h) && writes == 0, "A closed ColorWell builds no editor and writes nothing");
    h.activate(id);
    h.layout({400, 760});
    require(popover(h) && keyed(h, "well").color->open, "ColorWell opens an anchored picker");
    auto red = element(h, "Red").id;
    h.focus(red);
    h.set_text(red, "200");
    h.commit_numeric(red);
    require(value.get() == 0x80c87090 && writes == 1,
            "Explicit live preview publishes a valid component edit");
    h.cancel_color(id);
    require(!popover(h) && value.get() == 0x80507090 && writes == 2,
            "Cancel restores only the owned live preview");
    h.activate(id);
    h.layout({400, 760});
    red = element(h, "Red").id;
    h.focus(red);
    h.set_text(red, "210");
    h.commit_numeric(red);
    value.set(0xff112233);
    h.layout({400, 760});
    h.cancel_color(id);
    require(value.get() == 0xff112233 && writes == 3,
            "External replacement prevents stale live-preview rollback");
    h.activate(id);
    h.layout({400, 760});
    red = element(h, "Red").id;
    h.focus(red);
    h.set_text(red, "230");
    h.commit_numeric(red);
    hidden.set(true);
    h.layout({400, 760});
    require(!popover(h) && value.get() == 0xff112233 && writes == 5,
            "Hiding a live well rolls back after view construction");
    hidden.set(false);
    h.layout({400, 760});
    h.activate(id);
    h.layout({400, 760});
    red = element(h, "Red").id;
    h.focus(red);
    h.set_text(red, "240");
    h.commit_numeric(red);
    mounted.set(false);
    h.layout({400, 760});
    require(!popover(h) && value.get() == 0xff112233 && writes == 7,
            "Removing a live well safely restores its starting color");
    mounted.set(true);
    h.layout({400, 760});
    id = keyed(h, "well").id;
    h.activate(id);
    h.layout({400, 760});
    auto hex = element(h, "Hex").id;
    h.focus(hex);
    h.mark_text("仮", {0, 3});
    require(!h.apply_color(id) && h.text_input()->marked.has_value(),
            "Apply cannot bypass an active input-method composition");
    h.handle(s::KeyEvent{s::Key::escape});
    require(popover(h) && !h.text_input()->marked,
            "Escape first cancels composition without closing the picker");
    h.cancel_color(id);
}
void raw_draft_feedback() {
    s::State<s::Color> value{0xff112233};
    s::State<bool> hidden{false}, enabled{true};
    s::Host h(
        [&] {
            return s::ColorPicker("Paint", value.binding())
                .key("picker")
                .hidden(hidden.get())
                .enabled(enabled.get());
        },
        {}, theme());
    h.layout({360, 800});
    const auto id = keyed(h, "picker").id;
    auto invalid = [&] {
        auto hex = element(h, "Hex").id;
        h.focus(hex);
        h.set_text(hex, "#GGGGGG");
        h.layout({360, 800});
    };
    invalid();
    bool feedback = false;
    for (const auto& node : h.inspect())
        if (node.key == "validation" && !node.text.empty())
            feedback = true;
    require(feedback && h.color_validation(id)->status != s::NumericStatus::valid,
            "Invalid raw text produces visible feedback across a scene rebuild");
    hidden.set(true);
    h.layout({360, 800});
    hidden.set(false);
    h.layout({360, 800});
    require(!h.color_validation(id)->dirty && element(h, "Hex").value_text == "#112233FF",
            "Hiding cancels invalid raw text even when no scalar component was staged");
    invalid();
    enabled.set(false);
    h.layout({360, 800});
    enabled.set(true);
    h.layout({360, 800});
    require(!h.color_validation(id)->dirty, "Disabling retires raw component editor drafts");
    invalid();
    value.set(0xffaabbcc);
    h.layout({360, 800});
    feedback = false;
    for (const auto& node : h.inspect())
        if (node.key == "validation")
            feedback = true;
    require(!feedback && !h.color_validation(id)->dirty,
            "External replacement clears raw feedback without a stale validation frame");
}
void mode_and_dismissal() {
    s::State<s::Color> color{0x80112233};
    int clicks{};
    s::Host h(
        [&] {
            return s::FlexColumn{s::ColorWell("Paint", color.binding()).key("well"),
                                 s::Button("Behind", [&] { ++clicks; }).key("behind")}
                .padding(12)
                .gap(12);
        },
        {}, theme());
    h.layout({400, 800});
    const auto id = keyed(h, "well").id;
    h.activate(id);
    h.layout({400, 800});
    const auto hex = element(h, "Hex").id;
    h.focus(hex);
    h.set_text(hex, "#badhex");
    h.activate(element(h, "HSV", s::Role::segment).id);
    h.layout({400, 800});
    require(keyed(h, "well").color->model == s::ColorModel::rgb && color.get() == 0x80112233,
            "Mode changes cannot discard an invalid hex draft");
    h.set_text(hex, "#ff000080");
    h.activate(element(h, "HSV", s::Role::segment).id);
    h.layout({400, 800});
    require(keyed(h, "well").color->model == s::ColorModel::hsv &&
                keyed(h, "well").color->draft == 0x80ff0000 && color.get() == 0x80112233,
            "Switching RGB/HSV collects valid pending text without a default publication");
    h.handle(s::PointerEvent{s::PointerKind::down, {390, 790}});
    h.handle(s::PointerEvent{s::PointerKind::up, {390, 790}});
    require(!popover(h) && color.get() == 0x80112233 && clicks == 0,
            "Outside dismissal cancels the default draft and consumes the click");
    h.activate(id);
    h.layout({400, 800});
    require(keyed(h, "well").color->draft == color.get(),
            "Reopening a well begins from the latest application color");
}
void lifecycle_and_callbacks() {
    s::State<s::Color> value{0x80112233};
    s::State<bool> mounted{true}, hidden{false}, expanded{true}, alpha{true};
    std::function<void()> after_write;
    int writes{};
    s::Binding<s::Color> binding{[&] { return value.get(); },
                                 [&](auto next) {
                                     ++writes;
                                     value.set(next);
                                     if (after_write)
                                         after_write();
                                 }};
    auto h = std::make_unique<s::Host>(
        [&] {
            return s::ColorPicker("Live", binding, {.alpha = alpha.get(), .live_preview = true})
                .expanded(expanded.binding())
                .key("picker")
                .visible(mounted.get())
                .hidden(hidden.get());
        },
        std::function<void()>{}, theme());
    h->layout({360, 800});
    auto id = keyed(*h, "picker").id;
    auto stage = [&](int channel) {
        auto red = element(*h, "Red").id;
        h->focus(red);
        h->set_text(red, std::to_string(channel));
        h->commit_numeric(red);
    };
    stage(100);
    h->cancel_color(id);
    require(value.get() == 0x80112233 && writes == 2, "Standalone live preview rolls back just like a well");
    stage(110);
    expanded.set(false);
    h->layout({360, 800});
    require(value.get() == 0x80112233 && writes == 4,
            "Collapsing a live picker restores the owned preview outside construction");
    expanded.set(true);
    h->layout({360, 800});
    stage(120);
    alpha.set(false);
    h->layout({360, 800});
    require(value.get() == 0x80112233 && writes == 6 && !h->color_validation(id)->dirty,
            "Configuration changes retire and restore a pending live session");
    stage(130);
    h->blur();
    h->set_active(true);
    h->layout({360, 800});
    require(value.get() == 0x80112233 && writes == 8, "Window blur restores a standalone live preview");
    stage(140);
    h.reset();
    require(value.get() == 0x80112233 && writes == 10, "Host destruction releases an owned live preview");
    h = std::make_unique<s::Host>(
        [&] {
            return s::ColorPicker("Live", binding, {.live_preview = true})
                .visible(mounted.get())
                .key("picker");
        },
        std::function<void()>{}, theme());
    h->layout({360, 800});
    id = keyed(*h, "picker").id;
    after_write = [&] { h->layout({360, 800}); };
    stage(150);
    after_write = {};
    h->cancel_color(id);
    require(value.get() == 0x80112233 && writes == 12,
            "Reentrant layout during a preview setter preserves rollback ownership");
    after_write = [&] {
        mounted.set(false);
        h->layout({360, 800});
    };
    stage(160);
    after_write = {};
    h->layout({360, 800});
    require(value.get() == 0x80112233 && writes == 14,
            "Removal during preview publication restores the original value once");
    mounted.set(true);
    h->layout({360, 800});
    id = keyed(*h, "picker").id;
    stage(170);
    value.set(0xffaabbcc);
    h.reset();
    require(value.get() == 0xffaabbcc && writes == 15,
            "Destruction never overwrites an externally replaced preview");
}
void construction_purity() {
    s::State<s::Color> value{0xff112233};
    s::Host* pointer{};
    bool call{};
    s::Host h(
        [&] {
            if (call)
                pointer->apply_color("root");
            return s::ColorPicker("Paint", value.binding());
        },
        {}, theme());
    pointer = &h;
    h.layout({360, 800});
    call = true;
    h.invalidate();
    bool rejected{};
    try {
        h.layout({360, 800});
    } catch (const std::logic_error&) {
        rejected = true;
    }
    require(rejected && value.get() == 0xff112233,
            "Imperative Apply refuses view construction before publishing or reentering layout");
}
void hsv_keyboard_and_render() {
    s::State<s::Color> value{0xff808080};
    s::Host h(
        [&] {
            return s::ColorPicker("HSV", value.binding(), {.model = s::ColorModel::hsv})
                .key("picker")
                .padding(12);
        },
        {}, theme());
    h.layout({360, 800});
    const auto id = keyed(h, "picker").id;
    auto hue = element(h, "Hue").id;
    h.focus(hue);
    h.set_text(hue, "240");
    h.commit_numeric(hue);
    auto sat = element(h, "Saturation").id;
    h.focus(sat);
    h.set_text(sat, "100");
    h.commit_numeric(sat);
    h.apply_color(id);
    require(value.get() == 0xff000080, "Hue intent survives editing an achromatic source before saturation");
    auto slider = element(h, "Hue", s::Role::slider);
    h.focus(slider.id);
    h.handle(s::KeyEvent{s::Key::right});
    h.apply_color(id);
    require(value.get() != 0xff000080, "Standard slider keyboard controls update the color draft");
    for (bool light : {false, true}) {
        h.set_theme(theme(light));
        for (auto size : {s::Size{360, 800}, s::Size{250, 900}}) {
            h.layout(size);
            auto surface =
                SkSurfaces::Raster(SkImageInfo::MakeN32Premul(int(size.width * 2), int(size.height * 2)));
            require(bool(surface), "Color raster surface exists at 2x");
            surface->getCanvas()->scale(2, 2);
            h.paint(*surface->getCanvas());
            require(keyed(h, "picker").bounds.width <= size.width,
                    "Color picker stays within a constrained viewport");
        }
    }
}
} // namespace
int main() {
    try {
        conversions();
        drafts();
        live_and_popup();
        raw_draft_feedback();
        mode_and_dismissal();
        lifecycle_and_callbacks();
        construction_purity();
        hsv_keyboard_and_render();
        std::cout << checks << " color checks passed\n";
        return 0;
    } catch (const std::exception& e) {
        std::cerr << e.what() << " (check " << checks << ")\n";
        return 1;
    }
}
