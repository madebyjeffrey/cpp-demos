#include "include/core/SkCanvas.h"
#include "include/core/SkSurface.h"
#include <chrono>
#include <iostream>
#include <functional>
#include <optional>
#include <stdexcept>
#include <string>
#include <vector>
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
    auto result = light ? s::Theme::light() : s::Theme{};
    result.font_path = SCARS_FONT_PATH;
    return result;
}
s::InspectionNode keyed(s::Host& host, const std::string& key) {
    for (const auto& node : host.inspect())
        if (node.key == key)
            return node;
    throw std::runtime_error("Missing key: " + key);
}
void civil_values_and_locales() {
    require(s::valid(s::Date{2000, 2, 29}) && !s::valid(s::Date{1900, 2, 29}) &&
                !s::valid(s::Date{2026, 4, 31}) && !s::valid(s::Date{0, 1, 1}),
            "Gregorian validation handles century leap rules and invalid dates");
    require(!s::valid(s::Date{2024, 257, 1}) && !s::valid(s::Date{2024, 1, 257}),
            "Large component values cannot wrap through chrono's compact month/day storage");
    require(s::valid(s::Time{23, 59, 59}) && !s::valid(s::Time{24, 0, 0}) && !s::valid(s::Time{12, 60, 0}),
            "Time values represent a valid second within a civil day");
    for (const auto& locale : {"en_US_POSIX", "de_DE", "ja_JP", "ar_EG", "th_TH"})
        for (const auto& date : {s::Date{1, 1, 1}, s::Date{2024, 2, 29}, s::Date{9999, 12, 31}}) {
            const auto text = s::format_date(date, locale);
            if (s::parse_date(text, locale) != date)
                throw std::runtime_error(std::string("Date round trip: ") + locale + " " + text);
            require(true, "Localized Gregorian dates round-trip including supported boundaries");
        }
    require(s::parse_date("11/02/2026", "en_US_POSIX") == s::Date{2026, 11, 2} &&
                s::parse_date("02.11.2026", "de_DE") == s::Date{2026, 11, 2},
            "Date parsing respects locale field order");
    require(!s::parse_date("02/29/2025") && !s::parse_date("04/31/2026") && !s::parse_date("11/02/26") &&
                !s::parse_date("11/02/2026 trailing"),
            "Invalid, overflowing, two-digit-year, and trailing dates are rejected");
    for (auto cycle : {s::HourCycle::h24, s::HourCycle::h12})
        for (auto time : {s::Time{0, 0, 0}, s::Time{12, 0, 0}, s::Time{23, 59, 59}}) {
            s::TimePickerOptions options{.hour_cycle = cycle, .show_seconds = true};
            require(s::parse_time(s::format_time(time, options), options) == time,
                    "Time formatting preserves midnight, noon, and seconds in either hour cycle");
        }
    require(!s::parse_time("25:00") && !s::parse_time("12:60") && !s::parse_time("12:00 trailing"),
            "Strict time parsing rejects invalid components and trailing text");
    bool rejected{};
    try {
        s::format_date({2026, 1, 1}, "zz_Invalid");
    } catch (const std::invalid_argument&) {
        rejected = true;
    }
    require(rejected, "Unknown locales cannot silently fall back to the machine locale");
}
void zones() {
    const s::LocalDateTime gap{{2026, 3, 8}, {2, 30, 0}}, repeated{{2026, 11, 1}, {1, 30, 0}};
    require(!s::resolve_datetime(gap, "America/New_York").instant,
            "Nonexistent local times are rejected instead of silently shifted");
    require(!s::resolve_datetime(repeated, "America/New_York").instant,
            "Repeated local times require an explicit resolution policy");
    const auto first = s::resolve_datetime(repeated, "America/New_York", s::RepeatedTimePolicy::earlier);
    const auto last = s::resolve_datetime(repeated, "America/New_York", s::RepeatedTimePolicy::later);
    require(first.instant && last.instant && *last.instant - *first.instant == std::chrono::hours{1} &&
                s::local_datetime(*first.instant, "America/New_York") == repeated &&
                s::local_datetime(*last.instant, "America/New_York") == repeated,
            "Earlier and later policies select distinct instants for the same wall time");
    const auto utc = s::resolve_datetime({{2000, 2, 29}, {12, 34, 56}}, "UTC");
    require(utc.instant &&
                s::local_datetime(*utc.instant, "UTC") == s::LocalDateTime{{2000, 2, 29}, {12, 34, 56}},
            "UTC conversion is explicit and independent of the host time zone");
    bool rejected{};
    try {
        s::resolve_datetime(repeated, "Unknown/Zone");
    } catch (const std::invalid_argument&) {
        rejected = true;
    }
    require(rejected, "Unknown time zones are rejected before editing");
}
void fields() {
    for (bool light : {false, true}) {
        s::State<s::Date> date{s::Date{2024, 2, 29}};
        s::State<s::Time> time{s::Time{10, 15, 37}};
        s::State<bool> h12{false};
        s::Host host(
            [&] {
                return s::FlexColumn{
                    s::DatePicker("Date", date.binding(),
                                  {.min = {2024, 1, 1},
                                   .max = {2025, 12, 31},
                                   .disabled_date = [](s::Date date) { return date.day == 13; }})
                        .key("date"),
                    s::TimePicker("Time", time.binding(),
                                  {.hour_cycle = h12.get() ? s::HourCycle::h12 : s::HourCycle::h24})
                        .key("time")}
                    .gap(12)
                    .padding(12);
            },
            {}, theme(light));
        host.layout({360, 180});
        const auto did = keyed(host, "date").id, tid = keyed(host, "time").id;
        require(keyed(host, "date").type == "DatePicker" && keyed(host, "time").type == "TimePicker",
                "Temporal fields have distinct inspection identities");
        host.focus(did);
        host.set_text(did, "02/29/2025");
        require(!host.commit_field(did) && date.get() == s::Date{2024, 2, 29} &&
                    host.field_validation(did)->status == s::NumericStatus::invalid,
                "Invalid date drafts do not overwrite application data");
        host.set_text(did, "03/13/2024");
        require(!host.commit_field(did) &&
                    host.field_validation(did)->message.find("unavailable") != std::string::npos,
                "Disabled dates are rejected by typed commits as well as selection");
        host.handle(s::KeyEvent{s::Key::escape});
        require(!host.field_validation(did)->dirty, "Escape restores the committed date");
        host.set_text(did, "03/01/2024");
        require(host.commit_field(did) && date.get() == s::Date{2024, 3, 1}, "Valid date drafts commit");
        host.select_text(did, {0, 0});
        host.adjust(did, 1);
        require(date.get() == s::Date{2024, 4, 1}, "Date stepping adjusts the component at the caret");
        const auto unavailable =
            std::chrono::sys_days{std::chrono::year{2024} / 4 / 13}.time_since_epoch().count();
        require(!host.set_value(did, static_cast<double>(unavailable)) && date.get() == s::Date{2024, 4, 1},
                "Imperative value setters cannot bypass a temporal validator");
        host.focus(tid);
        require(host.commit_field(tid) && time.get().second == 37, "Untouched hidden seconds are preserved");
        host.select_text(tid, {3, 3});
        host.adjust(tid, 1);
        require(time.get() == s::Time{10, 16, 37}, "Minute stepping preserves the remaining components");
        host.set_text(tid, "11:30");
        require(host.commit_field(tid) && time.get() == s::Time{11, 30, 0},
                "Typed time commits use the displayed precision");
        host.set_text(tid, "invalid");
        h12.set(true);
        host.layout({360, 180});
        require(!host.field_validation(tid)->dirty &&
                    host.text_input()->text == s::format_time(time.get(), {.hour_cycle = s::HourCycle::h12}),
                "Hour-cycle changes replace drafts using the new presentation");
        auto surface = SkSurfaces::Raster(SkImageInfo::MakeN32Premul(720, 360));
        surface->getCanvas()->scale(2, 2);
        host.paint(*surface->getCanvas());
        host.blur();
        host.paint(*surface->getCanvas(), 1);
        require(!host.needs_frame(), "Temporal fields settle without continuous rendering");
    }
}
void calendar_and_steppers() {
    s::State<s::Date> date{s::Date{2024, 2, 29}};
    s::State<s::Time> time{s::Time{23, 30, 0}};
    s::State<bool> hidden{false}, readonly{false};
    s::Host host(
        [&] {
            return s::FlexColumn{
                s::DatePicker("Calendar date", date.binding(),
                              {.min = {2024, 1, 1},
                               .max = {2025, 12, 31},
                               .read_only = readonly.get(),
                               .disabled_date = [](s::Date date) { return date.day == 13; }})
                    .key("date")
                    .accent(0xff3040c0)
                    .hidden(hidden.get()),
                s::TimePicker("Time", time.binding(), {.hour_cycle = s::HourCycle::h12}).key("time")}
                .gap(12)
                .padding(12);
        },
        {}, theme());
    host.layout({360, 480});
    const auto id = keyed(host, "date").id, tid = keyed(host, "time").id;
    require(host.activate(id + "/action") && keyed(host, "date").calendar_open == true,
            "The calendar action opens an anchored SCARS popover");
    auto focused_day = [&]() -> std::optional<s::Date> {
        for (const auto& e : host.elements())
            if (e.focused && e.calendar_date)
                return e.calendar_date;
        return {};
    };
    require(focused_day() == date.get(), "Calendar opening focuses the selected day");
    bool styled{};
    for (const auto& node : host.inspect())
        if (node.calendar_date == date.get())
            styled = node.background == 0xff3040c0 && node.foreground == 0xfff7fbff;
    require(styled, "Calendar selection inherits chained accent styling and readable contrast");

    std::size_t days{};
    bool disabled{};
    for (const auto& e : host.elements())
        if (e.calendar_date) {
            ++days;
            if (e.calendar_date->day == 13)
                disabled |= !e.enabled;
        }
    require(days == 42 && disabled,
            "A six-week calendar exposes disabled dates without mounting unbounded content");
    host.handle(s::KeyEvent{s::Key::right});
    require(focused_day() == s::Date{2024, 3, 1} && date.get() == s::Date{2024, 2, 29},
            "Arrow navigation changes day focus and month without committing");
    host.handle(s::KeyEvent{s::Key::down});
    require(focused_day() == s::Date{2024, 3, 8}, "Vertical calendar navigation moves by a week");
    host.handle(s::KeyEvent{s::Key::page_down});
    require(focused_day() == s::Date{2024, 4, 8}, "Page Down navigates calendar months");
    host.handle(s::KeyEvent{s::Key::enter});
    require(date.get() == s::Date{2024, 4, 8} && keyed(host, "date").calendar_open == false &&
                host.text_input(),
            "Calendar acceptance commits once and restores the editable field");
    host.activate(id + "/action");
    host.handle(s::KeyEvent{s::Key::escape});
    require(date.get() == s::Date{2024, 4, 8} && keyed(host, "date").calendar_open == false,
            "Escape dismisses calendar navigation without changing the date");
    host.activate(id + "/action");
    const auto next = keyed(host, "Next year").id;
    host.activate(next);
    require(keyed(host, "calendar-title").text.find("2025") != std::string::npos,
            "Calendar header controls navigate years independently of selection");
    date.set({2024, 1, 2});
    host.layout({360, 480});
    require(keyed(host, "date").calendar_open == false,
            "External date replacement retires the old calendar session");
    host.activate(id + "/action");
    hidden.set(true);
    host.layout({360, 480});
    require(!host.popup_open(), "Hiding a date field removes its calendar and input scope");
    hidden.set(false);
    readonly.set(true);
    host.layout({360, 480});
    require(!host.activate(id + "/action"), "Read-only date fields disable calendar activation");
    readonly.set(false);
    host.layout({320, 180});
    host.activate(id + "/action");
    bool clipped_fits{};
    for (const auto& node : host.inspect())
        if (node.type == "Popover")
            clipped_fits = node.bounds.width <= 320 && node.bounds.height <= 180 && node.bounds.x >= 0 &&
                           node.bounds.y >= 0;
    require(clipped_fits,
            "Calendar popovers fit constrained viewports through the shared scroll/overlay layout");
    host.handle(s::KeyEvent{s::Key::escape});
    host.layout({360, 480});
    host.focus(tid);
    const auto text = host.text_input()->text;
    host.select_text(tid, {text.size(), text.size()});
    host.adjust(tid, 1);
    require(time.get() == s::Time{11, 30, 0}, "AM/PM stepping toggles the period while preserving minutes");
    host.select_text(tid, {0, 0});
    require(host.activate(tid + "/increase") && time.get() == s::Time{12, 30, 0},
            "Inline time steppers use the current caret component");
}
void composed_values() {
    s::State<s::LocalDateTime> value{s::LocalDateTime{{2026, 11, 2}, {12, 0, 0}}};
    s::Host host([&] { return s::DateTimePicker("Departure", value.binding()); }, {}, theme());
    host.layout({360, 240});
    const auto tid = keyed(host, "time").id;
    host.focus(tid);
    host.set_text(tid, "13:30");
    auto next = value.get();
    next.date = {2026, 11, 3};
    value.set(next);
    host.layout({360, 240});
    require(!host.field_validation(tid)->dirty && host.text_input()->text == "12:00",
            "Replacing the date cancels a time draft tied to the previous local value");
    auto initial = s::resolve_datetime({{2026, 11, 1}, {0, 30, 0}}, "America/New_York");
    s::State<std::chrono::sys_seconds> instant{*initial.instant};
    s::State<s::RepeatedTimePolicy> policy{s::RepeatedTimePolicy::reject};
    s::Host zoned(
        [&] {
            return s::DateTimePicker("Zoned", instant.binding(),
                                     {.time_zone = "America/New_York", .repeated_time = policy.get()});
        },
        {}, theme());
    zoned.layout({400, 240});
    const auto ztime = keyed(zoned, "time").id;
    zoned.focus(ztime);
    zoned.set_text(ztime, "01:30");
    require(!zoned.commit_field(ztime) && instant.get() == *initial.instant &&
                zoned.field_validation(ztime)->message.find("twice") != std::string::npos,
            "Zoned field validation rejects ambiguity without overwriting the instant");
    policy.set(s::RepeatedTimePolicy::earlier);
    require(zoned.commit_field(ztime) &&
                instant.get() == *s::resolve_datetime({{2026, 11, 1}, {1, 30, 0}}, "America/New_York",
                                                      s::RepeatedTimePolicy::earlier)
                                      .instant,
            "Explicit ambiguity policy permits committing the chosen instant");
    policy.set(s::RepeatedTimePolicy::reject);
    zoned.layout({400, 240});
    require(zoned.field_validation(ztime)->status == s::NumericStatus::valid,
            "An existing instant retains its known offset even when its wall time is ambiguous");
    zoned.set_text(ztime, "01:45");
    const auto replacement =
        *s::resolve_datetime({{2026, 11, 1}, {1, 30, 0}}, "America/New_York", s::RepeatedTimePolicy::later)
             .instant;
    instant.set(replacement);
    zoned.layout({400, 240});
    require(
        !zoned.field_validation(ztime)->dirty && zoned.text_input()->text == "01:30" &&
            instant.get() == replacement,
        "A distinct external overlap instant retires a draft even when its displayed wall time is unchanged");
}
void joint_drafts() {
    const s::LocalDateTime initial{{2026, 11, 2}, {23, 0, 0}}, target{{2026, 11, 3}, {1, 0, 0}};
    s::State<s::LocalDateTime> value{initial}, maximum{s::LocalDateTime{{2026, 11, 3}, {2, 0, 0}}};
    s::State<bool> hidden{false}, enabled{true}, readonly{false}, defer{false}, expanded{true}, mounted{true};
    bool accept = true, normalize = false;
    int writes{};
    bool valid_writes = true;
    std::function<void()> after_write;
    s::Binding<s::LocalDateTime> binding{
        [&] { return value.get(); },
        [&](s::LocalDateTime next) {
            ++writes;
            valid_writes &= next >= s::LocalDateTime{{2026, 11, 2}, {22, 0, 0}} && next <= maximum.get();
            if (accept) {
                if (normalize)
                    next.time = {1, 0, 0};
                value.set(next);
                if (after_write)
                    after_write();
            }
        }};
    s::Host host(
        [&] {
            return s::FlexColumn{
                s::DateTimePicker("Bounded", binding,
                                  {.date = {.read_only = readonly.get(), .commit_on_blur = !defer.get()},
                                   .time = {.read_only = readonly.get(), .commit_on_blur = !defer.get()},
                                   .min = {{2026, 11, 2}, {22, 0, 0}},
                                   .max = maximum.get()})
                    .key("joint")
                    .hidden(hidden.get())
                    .enabled(enabled.get())
                    .expanded(expanded.binding())
                    .visible(mounted.get()),
                s::Button("Outside", [] {}).key("outside")}
                .gap(12);
        },
        {}, theme());
    host.layout({400, 300});
    const auto group = keyed(host, "joint").id, date = keyed(host, "date").id, time = keyed(host, "time").id;
    const auto outside = keyed(host, "outside").id;
    host.focus(date);
    host.handle(s::KeyEvent{.key = s::Key::down, .alt = true});
    require(keyed(host, "date").calendar_open == true, "A composed date keeps its normal calendar access");
    value.set({{2026, 11, 2}, {22, 30, 0}});
    host.layout({400, 300});
    require(keyed(host, "date").calendar_open == false,
            "Replacing another component retires the old calendar source context");
    value.set(initial);
    host.layout({400, 300});

    auto partial = [&] {
        host.focus(date);
        host.set_text(date, "11/02/2026");
        host.focus(time);
    };
    host.focus(date);
    host.handle(s::KeyEvent{.key = s::Key::down, .alt = true});
    std::string tomorrow;
    for (const auto& element : host.elements())
        if (element.calendar_date == s::Date{2026, 11, 3})
            tomorrow = element.id;
    require(!tomorrow.empty() && host.activate(tomorrow) && value.get() == initial && writes == 0,
            "Calendar selection can stage the date half of an overnight transition");
    require(host.field_validation(group)->dirty && host.cancel_field(group) && value.get() == initial,
            "A partial calendar selection remains cancellable without a business-data write");

    host.focus(date);
    host.set_text(date, "11/03/2026");
    host.focus(time);
    require(value.get() == initial && writes == 0 && host.field_validation(group)->dirty,
            "A date change stages an overnight transition without publishing invalid intermediate state");
    require(keyed(host, "joint").date_time->draft == s::LocalDateTime{{2026, 11, 3}, {23, 0, 0}},
            "Inspector snapshots distinguish accepted component drafts from the committed model");
    host.set_text(time, "01:00");
    require(host.field_validation(group)->status == s::NumericStatus::valid && writes == 0,
            "Joint validation considers both pending components without publishing during construction");
    require(host.commit_field(group) && value.get() == target && writes == 1 && valid_writes,
            "Committing a joint date/time draft writes exactly one valid model value");
    partial();
    require(host.cancel_field(group) && value.get() == target && writes == 1 &&
                !host.field_validation(group)->dirty,
            "Explicit cancellation restores both components without writing the model");
    partial();
    host.mark_text("仮", {0, 3});
    host.handle(s::KeyEvent{s::Key::escape});
    require(host.field_validation(group)->dirty && !host.text_input()->marked,
            "Escape cancels composition before the pending joint transaction");
    host.handle(s::KeyEvent{s::Key::escape});
    require(!host.field_validation(group)->dirty && value.get() == target && writes == 1,
            "A subsequent Escape cancels the whole accepted component draft");
    partial();
    host.focus(outside);
    require(!host.field_validation(group)->dirty && value.get() == target && writes == 1,
            "Leaving the composite cancels an unfinished joint value");
    partial();
    hidden.set(true);
    host.layout({400, 300});
    hidden.set(false);
    host.layout({400, 300});
    require(!host.field_validation(group)->dirty && value.get() == target && writes == 1,
            "Hiding and showing a composite cannot resurrect its pending transaction");
    partial();
    enabled.set(false);
    host.layout({400, 300});
    enabled.set(true);
    host.layout({400, 300});
    require(!host.field_validation(group)->dirty && writes == 1,
            "Disabling a composite retires both its component drafts");
    partial();
    expanded.set(false);
    host.layout({400, 300});
    expanded.set(true);
    host.layout({400, 300});
    require(!host.field_validation(group)->dirty && writes == 1,
            "Collapsing a composite retires pending state before its fields remount");
    partial();
    mounted.set(false);
    host.layout({400, 300});
    require(!host.field_validation(group) && writes == 1,
            "Unmounting discards the host-owned joint transaction");
    mounted.set(true);
    host.layout({400, 300});
    require(!host.field_validation(group)->dirty, "Remounting starts from current application data");
    partial();
    host.blur();
    host.set_active(true);
    host.layout({400, 300});
    require(!host.field_validation(group)->dirty && writes == 1,
            "Window deactivation cancels unfinished joint state");
    partial();
    value.set({{2026, 11, 2}, {22, 30, 0}});
    host.layout({400, 300});
    require(!host.field_validation(group)->dirty && keyed(host, "joint").date_time->source == value.get(),
            "External model replacement retires a partial composite draft");
    accept = false;
    host.focus(date);
    host.set_text(date, "11/03/2026");
    host.focus(time);
    host.set_text(time, "01:30");
    require(!host.commit_field(group) && writes == 2 &&
                value.get() == s::LocalDateTime{{2026, 11, 2}, {22, 30, 0}} &&
                host.field_validation(group)->message.find("rejected") != std::string::npos,
            "Rejected setters retain the joint draft and explicit validation feedback");
    host.layout({400, 300});
    require(writes == 2 && host.field_validation(group)->dirty,
            "Ordinary rebuilds do not retry rejected model writes");
    host.cancel_field(group);
    accept = true;
    normalize = true;
    host.focus(date);
    host.set_text(date, "11/03/2026");
    host.focus(time);
    host.set_text(time, "01:45");
    require(host.commit_field(group) && writes == 3 && value.get() == target &&
                !host.field_validation(group)->dirty,
            "Application-normalized values replace both component drafts");
    normalize = false;
    partial();
    maximum.set({{2026, 11, 3}, {3, 0, 0}});
    host.layout({400, 300});
    require(!host.field_validation(group)->dirty && writes == 3,
            "Joint limit changes retire drafts from the old configuration");
    partial();
    readonly.set(true);
    host.layout({400, 300});
    readonly.set(false);
    host.layout({400, 300});
    require(!host.field_validation(group)->dirty && writes == 3,
            "Read-only changes cannot leave a hidden editable transaction");
    value.set({{2026, 11, 1}, {12, 0, 0}});
    host.layout({400, 300});
    require(host.field_validation(group)->status == s::NumericStatus::out_of_range,
            "An external value outside joint bounds is reported without silent correction");
    host.focus(date);
    host.set_text(date, "11/02/2026");
    host.focus(time);
    host.set_text(time, "23:00");
    require(host.commit_field(group) && value.get() == initial && writes == 4 && valid_writes,
            "Both components can recover an external out-of-range value atomically");
    defer.set(true);
    host.layout({400, 300});
    host.focus(date);
    host.set_text(date, "11/03/2026");
    host.focus(time);
    host.set_text(time, "01:00");
    require(value.get() == initial && writes == 4 && host.commit_field(group) && value.get() == target &&
                writes == 5,
            "An explicit group commit collects both deferred text drafts before one publication");
    bool nested = true;
    after_write = [&] {
        nested = host.commit_field(group);
        mounted.set(false);
        host.layout({400, 300});
    };
    host.focus(date);
    host.set_text(date, "11/02/2026");
    host.focus(time);
    host.set_text(time, "23:00");
    require(host.commit_field(group) && !nested && writes == 6 && value.get() == initial &&
                !host.field_validation(group),
            "Reentrant completion and owner removal cannot repeat or corrupt a joint publication");
}

void joint_construction_purity() {
    s::Host* current{};
    s::State<s::LocalDateTime> value;
    s::Host host(
        [&] {
            if (current)
                current->commit_field("root");
            return s::DateTimePicker("Pure", value.binding());
        },
        {}, theme());
    current = &host;
    bool rejected{};
    try {
        host.layout({400, 200});
    } catch (const std::logic_error&) {
        rejected = true;
    }
    require(rejected && value.get() == s::LocalDateTime{},
            "Joint field commits cannot mutate or reenter a building host");
}

} // namespace
int main() {
    try {
        civil_values_and_locales();
        zones();
        fields();
        calendar_and_steppers();
        composed_values();
        joint_drafts();
        joint_construction_purity();
        std::cout << checks << " date/time checks passed\n";
        return 0;
    } catch (const std::exception& e) {
        std::cerr << e.what() << '\n';
        return 1;
    }
}
