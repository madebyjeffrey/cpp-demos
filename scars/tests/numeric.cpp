#include "include/core/SkBitmap.h"
#include "include/core/SkCanvas.h"
#include "include/core/SkSurface.h"
#include <algorithm>
#include <chrono>
#include <cmath>
#include <cstdint>
#include <functional>
#include <iostream>
#include <limits>
#include <optional>
#include <stdexcept>
#include <string>
#include <vector>
import scars;
namespace s = scars;
using namespace std::chrono_literals;
namespace {
int checks{};
void require(bool condition, const char* message) {
    ++checks;
    if (!condition) throw std::runtime_error(message);
}
void throws(const std::function<void()>& action, const char* message) {
    bool caught = false;
    try { action(); } catch (const std::exception&) { caught = true; }
    require(caught, message);
}
bool near(double a, double b) { return std::abs(a - b) < .000001; }
s::Theme theme(bool light = false) {
    auto t = light ? s::Theme::light() : s::Theme{};
    t.font_path = SCARS_FONT_PATH;
    return t;
}
s::InspectionNode keyed(s::Host& host, const std::string& key) {
    for (const auto& n : host.inspect()) if (n.key == key) return n;
    throw std::runtime_error("Missing control: " + key);
}
s::Element element(s::Host& host, const std::string& id) {
    for (const auto& e : host.elements()) if (e.id == id) return e;
    throw std::runtime_error("Missing element: " + id);
}
s::Element named(s::Host& host, const std::string& label) {
    for (const auto& e : host.elements()) if (e.label == label) return e;
    throw std::runtime_error("Missing element: " + label);
}
void type(s::Host& host, const std::string& id, std::string value) {
    host.select_text(id, {0, element(host, id).value_text.size()});
    host.insert_text(std::move(value));
}
void key(s::Host& host, s::Key key) { host.handle(s::KeyEvent{key}); }
s::Point center(s::Rect r) { return {r.x + r.width / 2, r.y + r.height / 2}; }
void drafts() {
    s::State<double> value{3.5}, maximum{10};
    s::State<s::NumericValidation> validation;
    int submissions = 0;
    s::Host host([&] {
        return s::Block{
            s::NumericField("Amount", value.binding(), {.min = -10, .max = maximum.get(), .step = .25,
                .significant_digits = 6, .on_validation = [&](const auto& v) { validation.set(v); },
                .on_submit = [&] { ++submissions; }}).key("number").accessible_label("Amount"),
            s::Button("Other", [] {}).key("other")}.gap(12);
    }, {}, theme());
    host.layout({420, 180});
    const auto id = keyed(host, "number").id, other = keyed(host, "other").id;
    require(keyed(host, "number").type == "NumericField" && element(host, id).value_text == "3.5",
            "Numeric fields format the initial model and identify themselves in the inspector");
    type(host, id, "-");
    require(near(value.get(), 3.5) && host.numeric_validation(id)->status == s::NumericStatus::incomplete &&
                validation.get().dirty, "Partial drafts update validation without changing the bound number");
    key(host, s::Key::enter);
    require(near(value.get(), 3.5) && submissions == 0, "Invalid submission never updates the model or invokes submit");
    key(host, s::Key::escape);
    require(element(host, id).value_text == "3.5" && !host.numeric_validation(id)->dirty,
            "Escape restores the committed model value");
    type(host, id, "1e");
    require(host.numeric_validation(id)->status == s::NumericStatus::incomplete, "An unfinished exponent remains editable");
    type(host, id, "1e2");
    require(host.numeric_validation(id)->status == s::NumericStatus::out_of_range && element(host, id).invalid &&
                !element(host, id).help.empty(), "Range errors expose validation feedback and accessible help");
    type(host, id, "1.25");
    require(host.numeric_validation(id)->status == s::NumericStatus::valid && near(value.get(), 3.5),
            "A valid draft remains separate from the committed value");
    key(host, s::Key::enter);
    require(near(value.get(), 1.25) && submissions == 1 && !host.numeric_validation(id)->dirty,
            "Enter commits valid numeric input and invokes submit once");
    type(host, id, "2.5");
    host.focus(other);
    require(near(value.get(), 2.5) && !host.numeric_validation(id)->dirty, "The default blur policy commits valid drafts");
    type(host, id, "100");
    host.focus(other);
    require(near(value.get(), 2.5) && element(host, id).value_text == "100" && host.numeric_validation(id)->dirty,
            "Invalid drafts survive blur without silently clamping the model");
    host.focus(id);
    key(host, s::Key::escape);
    std::string clipboard{"+4.5"};
    host.set_clipboard({[&] { return clipboard; }, [&](const auto& text) { clipboard = text; }});
    host.select_text(id, {0, 3});
    host.text_command(s::TextCommand::paste);
    require(element(host, id).value_text == "+4.5" && near(value.get(), 2.5), "Pasting edits the numeric draft");
    host.text_command(s::TextCommand::undo);
    require(element(host, id).value_text == "2.5", "Numeric drafts retain text undo support");
    host.text_command(s::TextCommand::redo);
    key(host, s::Key::enter);
    require(near(value.get(), 4.5), "Redo and explicit positive signs can be committed");
    host.select_text(id, {0, element(host, id).value_text.size()});
    host.mark_text("5", {1, 0});
    require(near(value.get(), 4.5) && !host.numeric_validation(id)->dirty, "IME preview does not become a numeric draft before composition commits");
    host.unmark_text();
    require(host.numeric_validation(id)->dirty && near(value.get(), 4.5), "Committed IME text is still a deferred numeric draft");
    key(host, s::Key::enter);
    require(near(value.get(), 5), "Numeric IME input commits through the same validation path");
    type(host, id, "6");
    maximum.set(2);
    require(host.numeric_validation(id)->status == s::NumericStatus::out_of_range && !host.commit_numeric(id),
            "Changing a range revalidates the current draft before commit");
    maximum.set(10);
    value.set(7.25);
    require(element(host, id).value_text == "7.25" && !host.numeric_validation(id)->dirty && !host.text_input()->can_undo,
            "External model changes replace stale drafts and editing history");
    key(host, s::Key::up);
    require(near(value.get(), 7.5), "Up increments a numeric field by its configured step");
    type(host, id, "bad");
    key(host, s::Key::down);
    require(near(value.get(), 7.5), "Increment shortcuts cannot discard invalid drafts");
    key(host, s::Key::escape);
    require(host.adjust(id, 100) && near(value.get(), 10), "Explicit numeric adjustments clamp at the configured limit");
    require(!host.adjust(id, .5), "Fractional step counts are rejected");
    s::State<double> deferred{2};
    s::Host explicit_commit([&] { return s::Block{
        s::NumericField("", deferred.binding(), {.commit_on_blur = false}).key("deferred"),
        s::Button("Leave", [] {}).key("leave")}; }, {}, theme());
    explicit_commit.layout({300, 120});
    const auto deferred_id = keyed(explicit_commit, "deferred").id;
    type(explicit_commit, deferred_id, "8");
    explicit_commit.focus(keyed(explicit_commit, "leave").id);
    require(near(deferred.get(), 2) && explicit_commit.numeric_validation(deferred_id)->dirty,
            "Disabling blur commit retains a valid pending draft");
    require(explicit_commit.commit_numeric(deferred_id) && near(deferred.get(), 8), "Applications can explicitly commit a numeric field");
    s::State<double> accepted{1}, limit{10};
    s::State<s::NumericValidation> accepted_validation;
    s::Binding<double> changing_setter{[&] { return accepted.get(); }, [&](double v) {
        accepted.set(v);
        limit.set(v - .5);
    }};
    s::Host changed([&] { return s::NumericField("", changing_setter,
        {.min = 0, .max = limit.get(), .on_validation = [&](const auto& v) { accepted_validation.set(v); }})
        .key("changing"); }, {}, theme());
    changed.layout({300, 80});
    const auto changing_id = keyed(changed, "changing").id;
    type(changed, changing_id, "2");
    require(changed.commit_numeric(changing_id) && near(accepted.get(), 2) &&
                accepted_validation.get().status == s::NumericStatus::out_of_range &&
                changed.numeric_validation(changing_id)->status == s::NumericStatus::out_of_range,
            "Validation callbacks reflect constraints changed by the application setter");
}
void locales_and_limits() {
    s::State<std::int64_t> integer{0};
    s::State<double> german{1.5}, arabic{0}, french{0}, precise{1.234567};
    s::State<int> small{1};
    s::Host host([&] { return s::Block{
        s::NumericField("", integer.binding(), {.locale = "en_US", .grouping = true}).key("integer"),
        s::NumericField("", german.binding(), {.min = -1000000, .max = 1000000, .locale = "de_DE", .grouping = true}).key("de"),
        s::NumericField("", arabic.binding(), {.locale = "ar_EG"}).key("ar"),
        s::NumericField("", french.binding(), {.locale = "fr_FR", .grouping = true}).key("fr"),
        s::NumericField("", precise.binding(), {.significant_digits = 3}).key("precision"),
        s::NumericField("", small.binding(), {.min = -2, .max = 2}).key("small")}; }, {}, theme());
    host.layout({420, 360});
    const auto integer_id = keyed(host, "integer").id, de = keyed(host, "de").id,
        ar = keyed(host, "ar").id, fr = keyed(host, "fr").id;
    require(element(host, de).value_text == "1,5", "Decimal display uses the explicit portable locale");
    type(host, de, "1.234,5");
    require(host.commit_numeric(de) && near(german.get(), 1234.5), "Locale grouping and decimal separators parse together");
    type(host, ar, "١٢٣٫٥");
    require(host.commit_numeric(ar) && near(arabic.get(), 123.5), "Arabic digits and decimal separators parse through ICU");
    type(host, fr, "1 234,5");
    require(host.commit_numeric(fr) && near(french.get(), 1234.5), "French narrow-space grouping parses without platform APIs");
    type(host, integer_id, "9223372036854775807");
    require(host.commit_numeric(integer_id) && integer.get() == std::numeric_limits<std::int64_t>::max(),
            "Integer fields parse the full signed 64-bit maximum exactly");
    require(host.adjust(integer_id, -1) && integer.get() == std::numeric_limits<std::int64_t>::max() - 1,
            "Integer stepping above double precision remains exact");
    host.adjust(integer_id, 100);
    require(integer.get() == std::numeric_limits<std::int64_t>::max(), "Integer stepping clamps without overflow");
    type(host, integer_id, "9223372036854775808");
    require(!host.commit_numeric(integer_id) && host.numeric_validation(integer_id)->status == s::NumericStatus::invalid,
            "Integer overflow cannot silently become a rounded or truncated value");
    type(host, integer_id, "-9223372036854775808");
    require(host.commit_numeric(integer_id) && integer.get() == std::numeric_limits<std::int64_t>::min(),
            "Integer fields parse the full signed 64-bit minimum exactly");
    host.adjust(integer_id, -100);
    require(integer.get() == std::numeric_limits<std::int64_t>::min(), "Integer decrement cannot underflow");
    host.adjust(integer_id, 18446744073709551616.0);
    require(integer.get() == std::numeric_limits<std::int64_t>::max(), "Large integer step counts saturate across the full signed range");
    type(host, integer_id, "1.25");
    require(!host.commit_numeric(integer_id), "Integer fields reject fractional drafts");
    key(host, s::Key::escape);
    for (const auto* invalid : {"NaN", "Infinity", "1e999", "1,23garbage"}) {
        type(host, de, invalid);
        require(!host.commit_numeric(de), "Non-finite and malformed numbers are rejected");
    }
    const auto precision_id = keyed(host, "precision").id;
    require(element(host, precision_id).value_text == "1.23" && host.commit_numeric(precision_id) && near(precise.get(), 1.234567),
            "Display precision does not round an untouched model value on commit");
    const auto small_id = keyed(host, "small").id;
    type(host, small_id, "-2");
    require(host.commit_numeric(small_id) && small.get() == -2 && host.adjust(small_id, -1) && small.get() == -2,
            "Signed integral bindings retain their own type limits");
    require(!host.set_value(integer_id, 9223372036854775808.0) && !host.set_value(integer_id, 1.5),
            "The double-valued accessibility setter rejects unsafe integer conversions");
    s::State<double> extreme{std::numeric_limits<double>::max()};
    s::Host extremes([&] { return s::NumericField("", extreme.binding()).key("extreme"); }, {}, theme());
    extremes.layout({420, 100});
    const auto extreme_id = keyed(extremes, "extreme").id;
    require(extremes.commit_numeric(extreme_id) && extreme.get() == std::numeric_limits<double>::max(),
            "Untouched finite values at the double limit retain their exact model value");
    type(extremes, extreme_id, "4.9406564584124654e-324");
    require(extremes.commit_numeric(extreme_id) && extreme.get() == std::numeric_limits<double>::denorm_min() &&
                element(extremes, extreme_id).value_text != "0", "Subnormal finite values parse and format without becoming zero");
    throws([&] { s::NumericField("", german.binding(), {.min = 2, .max = 1}); }, "Reversed numeric ranges are rejected");
    throws([&] { s::NumericField("", german.binding(), {.step = 0}); }, "Zero numeric steps are rejected");
    throws([&] { s::NumericField("", german.binding(), {.significant_digits = 18}); }, "Unsupported numeric display precision is rejected");
    throws([&] { s::NumericField("", german.binding(), {.locale = "zz_invalid"}); }, "Unknown locales cannot silently use the machine locale");
    german.set(std::numeric_limits<double>::quiet_NaN());
    throws([&] { host.layout({420, 360}); }, "Non-finite model values are rejected before rendering");
}
void repeat_and_switch() {
    auto now = std::chrono::steady_clock::time_point{};
    s::State<int> count{10};
    s::State<bool> on{false}, hidden{false}, enabled{true}, mounted{true};
    s::Host host([&]() -> s::View {
        if (!mounted.get()) return {};
        return s::Block{
            s::Stepper(count.binding(), {.min = 0, .max = 100, .repeat_delay = 200ms, .repeat_interval = 50ms})
                .key("stepper").accessible_label("Count").hidden(hidden.get()).enabled(enabled.get()),
            s::Switch("Enabled", on.binding()).key("switch"), s::Button("Other", [] {}).key("other")}.gap(12);
    }, {}, theme(), {}, {.clock = [&] { return now; }});
    host.layout({420, 200});
    const auto id = keyed(host, "stepper").id;
    require(element(host, id).role == s::Role::stepper && element(host, id).value_text == "10" && !host.next_wake(),
            "Steppers expose their numeric range and start without scheduled work");
    host.focus(id);
    key(host, s::Key::up); key(host, s::Key::page_up);
    require(count.get() == 21, "Stepper arrow and page keys use shared numeric steps");
    key(host, s::Key::home);
    require(count.get() == 0 && !named(host, "Decrease Count").enabled, "Home reaches the lower bound and disables decrement");
    key(host, s::Key::end);
    require(count.get() == 100 && !named(host, "Increase Count").enabled, "End reaches the upper bound and disables increment");
    count.set(10);
    auto point = center(named(host, "Increase Count").bounds);
    host.handle({s::PointerKind::down, point});
    require(count.get() == 11 && host.next_wake() == 200ms && !host.needs_frame(),
            "Press increments once and schedules repeat without continuous idle animation");
    now += 199ms; host.advance_time();
    require(count.get() == 11 && host.next_wake() == 1ms, "Repeat waits for its initial deadline");
    now += 1ms; host.advance_time();
    require(count.get() == 12 && host.next_wake() == 50ms, "Repeat uses the configured interval after the initial delay");
    now += 2s; host.advance_time();
    require(count.get() == 13 && host.next_wake() == 50ms, "An overdue repeat does not generate a burst of accumulated actions");
    host.handle({s::PointerKind::move, {-50, -50}});
    now += 1s; host.advance_time();
    require(count.get() == 13 && !host.next_wake(), "Moving outside the captured button pauses repeat");
    host.handle({s::PointerKind::move, point});
    now += 50ms; host.advance_time();
    require(count.get() == 14, "Reentering the held button resumes repeat");
    host.handle({s::PointerKind::up, {-50, -50}});
    now += 1s; host.advance_time();
    require(count.get() == 14 && !host.next_wake(), "Releasing capture stops repeat without a second release action");
    for (int mode = 0; mode < 6; ++mode) {
        point = center(named(host, "Increase Count").bounds);
        host.handle({s::PointerKind::down, point});
        const auto saved = count.get();
        if (mode == 0) host.handle({s::PointerKind::cancel});
        if (mode == 1) host.blur();
        if (mode == 2) host.focus(keyed(host, "other").id);
        if (mode == 3) hidden.set(true);
        if (mode == 4) enabled.set(false);
        if (mode == 5) mounted.set(false);
        now += 1s; host.advance_time();
        host.handle({s::PointerKind::up, point});
        require(count.get() == saved && !host.next_wake(), "Cancellation, blur, focus loss, hiding, disabling, and removal stop repeat");
        host.set_active(true); hidden.set(false); enabled.set(true); mounted.set(true);
    }
    count.set(99);
    point = center(named(host, "Increase Count").bounds);
    host.handle({s::PointerKind::down, point});
    require(count.get() == 100 && !host.next_wake(), "Reaching a range endpoint cancels pending repeat");
    host.handle({s::PointerKind::up, point});
    const auto switch_id = keyed(host, "switch").id;
    require(element(host, switch_id).role == s::Role::switch_control, "Switches expose distinct boolean semantics");
    point = center(element(host, switch_id).bounds);
    host.handle({s::PointerKind::down, point}); host.handle({s::PointerKind::up, point});
    require(on.get() && element(host, switch_id).selected, "The entire switch label row is clickable");
    host.focus(switch_id); key(host, s::Key::space);
    require(!on.get(), "Space toggles a focused switch");
    host.handle(s::KeyEvent{s::Key::space, true, true});
    require(!on.get(), "Held Space does not repeatedly toggle a switch");
    require(host.set_value(switch_id, 1) && on.get(), "Switch values support accessibility assignment");
    s::State<int> read{3};
    s::Host readonly([&] { return s::Block{
        s::NumericField("", read.binding(), {.read_only = true}).key("field"),
        s::Stepper(read.binding(), {.read_only = true}).key("stepper")}; }, {}, theme());
    readonly.layout({300, 120});
    const auto read_id = keyed(readonly, "field").id;
    require(!readonly.set_text(read_id, "7") && !readonly.commit_numeric(read_id) &&
                !readonly.adjust(keyed(readonly, "stepper").id, 1) && read.get() == 3,
            "Read-only numeric inputs reject text, commit, and increment actions");
}
void rendering() {
    s::State<double> amount{4.5};
    s::State<int> count{3};
    s::State<bool> on{false};
    s::Host host([&] { return s::Block{
        s::NumericField("Amount", amount.binding()).key("amount"),
        s::Stepper(count.binding()).key("count"), s::Switch("Automation", on.binding()).key("on")
    }.padding(12).gap(12); }, {}, theme());
    const auto draw = [&](bool light, int scale) {
        host.set_theme(theme(light));
        host.layout({420, 240});
        auto surface = SkSurfaces::Raster(SkImageInfo::MakeN32Premul(420 * scale, 240 * scale));
        surface->getCanvas()->scale(scale, scale);
        host.paint(*surface->getCanvas());
        SkBitmap image;
        image.allocN32Pixels(420 * scale, 240 * scale);
        require(surface->readPixels(image, 0, 0), "Numeric previews render to a raster surface");
        return image;
    };
    const auto dark = draw(false, 1);
    const auto light = draw(true, 1);
    const auto scaled = draw(true, 2);
    require(dark.getColor(380, 40) != light.getColor(380, 40), "Numeric fields use the active theme palette");
    require(scaled.getColor(760, 80) == light.getColor(380, 40), "Numeric controls preserve logical geometry at 2x display scale");
    const auto id = keyed(host, "amount").id;
    type(host, id, "invalid");
    const auto invalid = draw(false, 1);
    int error_pixels = 0;
    const auto bounds = keyed(host, "amount").bounds;
    for (int y = bounds.y; y < bounds.y + bounds.height; ++y)
        for (int x = bounds.x; x < bounds.x + bounds.width; ++x) {
            const auto color = invalid.getColor(x, y);
            if (SkColorGetR(color) > 190 && SkColorGetG(color) < 150 && SkColorGetB(color) < 150)
                ++error_pixels;
        }
    require(error_pixels > 10, "Invalid numeric drafts paint a visible error border");
    const auto switch_bounds = keyed(host, "on").bounds;
    on.set(true);
    const auto selected = draw(false, 1);
    require(selected.getColor(switch_bounds.x + 6, switch_bounds.y + switch_bounds.height / 2) !=
                invalid.getColor(switch_bounds.x + 6, switch_bounds.y + switch_bounds.height / 2),
            "Switch state changes its track and thumb rendering");
}
} // namespace
int main() {
    try {
        drafts(); locales_and_limits(); repeat_and_switch(); rendering();
        std::cout << "SCARS numeric inputs: " << checks << " checks passed\n";
        return 0;
    } catch (const std::exception& error) {
        std::cerr << "SCARS numeric inputs failed after " << checks << " checks: " << error.what() << '\n';
        return 1;
    }
}
