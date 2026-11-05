#include <skia_window/skia_window.hpp>
#include <iostream>
#include <memory>
#include <stdexcept>
#include <string>
import scars;
import scars.macos;
namespace s = scars;
namespace sw = skia_window;
void scars_datetime_native_step(int);
void scars_datetime_native_closed();
namespace {
void check(bool value, const char* message) {
    if (!value)
        throw std::runtime_error(message);
}
void run(sw::Application& app) {
    auto created = sw::Window::create(
        app, {.title = "SCARS Date Time Test", .size = {500, 480}, .minimum_size = {320, 200}});
    check(bool(created), "Create native date/time test window");
    auto window = std::move(*created);
    s::State<s::Date> date{s::Date{2024, 2, 29}};
    s::State<s::Time> time{s::Time{10, 15, 37}};
    s::State<bool> readonly{false};
    int commits{}, joint_commits{};
    s::State<s::LocalDateTime> joint{s::LocalDateTime{{2026, 11, 2}, {23, 0, 0}}};
    s::Binding<s::LocalDateTime> joint_binding{[&] { return joint.get(); },
                                               [&](auto value) {
                                                   ++joint_commits;
                                                   joint.set(value);
                                               }};
    s::Binding<s::Date> binding{[&] { return date.get(); },
                                [&](s::Date value) {
                                    ++commits;
                                    date.set(value);
                                }};
    auto host = s::macos::mount(*window, [&] {
        return s::FlexColumn{
            s::DatePicker("Departure date", binding,
                          {.min = {2024, 1, 1},
                           .max = {2025, 12, 31},
                           .disabled_date = [](s::Date value) { return value.day == 13; }}),
            s::TimePicker("Departure time", time.binding(), {.read_only = readonly.get()}),
            s::DateTimePicker("Bounded", joint_binding,
                              {.min = {{2026, 11, 2}, {22, 0, 0}}, .max = {{2026, 11, 3}, {2, 0, 0}}}),
            s::Button("Next", [] {})}
            .gap(12)
            .padding(16);
    });
    window->show();
    host->layout({500, 480});
    scars_datetime_native_step(0);
    check(date.get() == s::Date{2024, 2, 29} && commits == 0, "Invalid native date drafts cannot commit");
    scars_datetime_native_step(1);
    check(date.get() == s::Date{2024, 3, 1} && commits == 1,
          "Valid native date commits use the application binding once");
    scars_datetime_native_step(2);
    check(date.get() == s::Date{2024, 3, 1} && commits == 1 && !host->text_input()->marked,
          "Native date Return first completes composition without submitting it");
    scars_datetime_native_step(3);
    check(date.get() == s::Date{2024, 3, 4} && commits == 2, "A subsequent native Return commits the date");
    scars_datetime_native_step(4);
    check(date.get() == s::Date{2024, 3, 5} && commits == 3 && host->text_input() &&
              host->text_input()->date_picker,
          "Native calendar selection commits and restores the input client");
    scars_datetime_native_step(5);
    check(time.get() == s::Time{10, 16, 37}, "Native arrow input adjusts the active time component");
    scars_datetime_native_step(6);
    check(time.get() == s::Time{10, 17, 37}, "Native time stepper activation preserves hidden seconds");
    readonly.set(true);
    host->layout({500, 480});
    scars_datetime_native_step(7);
    check(time.get() == s::Time{10, 17, 37} && host->text_input() && !host->text_input()->editable,
          "Read-only temporal inputs reject native value replacement and stepping");
    scars_datetime_native_step(8);
    check(joint_commits == 0 && joint.get() == s::LocalDateTime{{2026, 11, 2}, {23, 0, 0}},
          "Native component submission cannot publish an invalid overnight intermediate");
    scars_datetime_native_step(9);
    check(joint_commits == 1 && joint.get() == s::LocalDateTime{{2026, 11, 3}, {1, 0, 0}},
          "Native joint submission writes one valid overnight date/time");
    window->close();
    scars_datetime_native_closed();
    host->invalidate();
    const auto retained = host->text_input();
    check(!retained || (!retained->active && !retained->marked),
          "Closing a temporal host deactivates retained editing metadata and composition");
    std::cout << "Native date/time drafts, composition, calendar, stepping, accessibility, and lifetime "
                 "checks passed.\n";
}
} // namespace
int main() {
    try {
        std::string failure;
        auto created =
            sw::Application::create({.name = "SCARS Date Time Tests",
                                     .on_error = [&](const auto& error) { failure = error.message; }});
        if (!created)
            throw std::runtime_error(created.error().message);
        auto app = std::move(*created);
        app->post([&] {
            try {
                run(*app);
            } catch (const std::exception& error) {
                failure = error.what();
            }
            app->quit();
        });
        app->run();
        if (!failure.empty())
            throw std::runtime_error(failure);
        return 0;
    } catch (const std::exception& error) {
        std::cerr << "SCARS native date/time: " << error.what() << '\n';
        return 1;
    }
}
