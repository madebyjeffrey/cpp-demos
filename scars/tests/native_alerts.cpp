#include <skia_window/skia_window.hpp>
#include <functional>
#include <iostream>
#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
import scars;
import scars.macos;
namespace s = scars;
namespace sw = skia_window;
void scars_inspect_alert_sheet();
void scars_alert_command(bool escape);
void scars_close_alert_sheet();
void scars_check_alert_parent_restored();
namespace {
void check(bool value, const char* message) {
    if (!value)
        throw std::runtime_error(message);
}
void run(sw::Application& app) {
    auto created = sw::Window::create(app, {.title = "SCARS Alert Parent", .size = {500, 360}});
    check(bool(created), "Create alert parent");
    auto parent = std::move(*created);
    created = sw::Window::create(app, {.title = "SCARS Alert Other", .size = {500, 360}});
    check(bool(created), "Create unrelated alert test window");
    auto other = std::move(*created);
    int blocked{}, allowed{}, completions{};
    auto parent_host =
        s::macos::mount(*parent, [&] { return s::Button("Parent action", [&] { ++blocked; }); });
    auto other_host = s::macos::mount(*other, [&] { return s::Button("Other action", [&] { ++allowed; }); });
    parent->show();
    other->show();
    s::State<std::string> draft{"Draft"};
    std::optional<std::string> result;
    auto shown = s::macos::AlertPresentation::present(
        *parent, "SCARS Alert Sheet", [&] { return s::TextField("Alert name", draft.binding()); },
        {{"cancel", "Cancel", s::AlertActionRole::cancel},
         {"accept", "Save", s::AlertActionRole::default_action},
         {"delete", "Delete", s::AlertActionRole::destructive},
         {"disabled", "Disabled", s::AlertActionRole::normal, false}},
        [&](auto id) {
            ++completions;
            result = id;
        });
    check(bool(shown), "Present SCARS content as a native sheet");
    auto alert = std::move(*shown);
    check(alert->is_open() && !parent->ui_host() && !alert->host()->text_input()->text.empty(),
          "Native alert has an independent host and initial editor focus");
    auto duplicate = s::macos::confirm(*parent, "Duplicate", "Not shown", [&](bool) { ++completions; });
    check(!duplicate && completions == 0,
          "Duplicate presentation reports failure without invoking completion");
    scars_inspect_alert_sheet();
    check(blocked == 0 && allowed == 1, "A sheet blocks only its parent window's input");
    check(!alert->choose("disabled") && !alert->choose("missing") && completions == 0,
          "Disabled or missing native alert actions do not complete");
    scars_alert_command(false);
    check(alert->is_open() && !alert->host()->text_input()->marked && completions == 0,
          "Native Return first completes marked text");
    scars_alert_command(false);
    check(!alert->is_open() && completions == 1 && result == "accept",
          "Native Return chooses the default exactly once");
    scars_check_alert_parent_restored();
    auto retained = alert->host();
    alert->dismiss();
    alert.reset();
    retained->invalidate();
    check(completions == 1, "Held hosts and dismissed presentations cannot repeat completion");
    shown = s::macos::AlertPresentation::present(*parent, "SCARS Alert Sheet", s::Label("Cancel?"),
                                                 {{"cancel", "Cancel", s::AlertActionRole::cancel}},
                                                 [&](auto id) {
                                                     ++completions;
                                                     result = id;
                                                 });
    check(bool(shown), "Present a cancellation alert");
    alert = std::move(*shown);
    scars_alert_command(true);
    check(!alert->is_open() && result == "cancel" && completions == 2,
          "Native Escape selects the cancel action");
    shown = s::macos::AlertPresentation::present(
        *parent, "SCARS Alert Sheet", s::Label("Close?"), {{"cancel", "Cancel", s::AlertActionRole::cancel}},
        [&](auto id) { check(!id, "Titlebar closure has no action ID"); });
    check(bool(shown), "Present a titlebar-close alert");
    alert = std::move(*shown);
    scars_close_alert_sheet();
    check(!alert->is_open(), "Native titlebar closure dismisses the alert");
    scars_check_alert_parent_restored();
    std::unique_ptr<s::macos::AlertPresentation> replacement;
    auto confirmation = s::macos::confirm(*parent, "Confirm", "Continue?", [&](bool accepted) {
        check(accepted, "Confirmation acceptance");
        ++completions;
        auto next = s::macos::confirm(*parent, "Replacement", "New presentation", [&](bool value) {
            check(!value, "Replacement cancellation");
            ++completions;
        });
        check(bool(next), "Completion can immediately present the next sheet");
        replacement = std::move(*next);
    });
    check(bool(confirmation), "Create confirmation helper");
    check((*confirmation)->choose("accept") && replacement && replacement->is_open() && completions == 3,
          "Action completion safely changes presentation state");
    replacement.reset();
    check(completions == 4, "Dropping a presentation handle cancels completion once");
    confirmation = s::macos::confirm(*parent, "Parent closure", "Pending", [&](bool value) {
        check(!value, "Parent closure must cancel");
        ++completions;
    });
    check(bool(confirmation), "Create parent-close confirmation");
    parent->close();
    check(!(*confirmation)->is_open() && completions == 5,
          "Closing a parent closes its child and completes once");
    (*confirmation)->dismiss();
    confirmation->reset();
    check(completions == 5, "Parent close and later destruction cannot repeat completion");
    auto unavailable = s::macos::confirm(*parent, "Closed", "Unavailable", [&](bool) { ++completions; });
    check(!unavailable && completions == 5, "Closed parents report a presentation error");
    auto titlebar = s::macos::confirm(*other, "Explicit dismissal", "Pending", [&](bool accepted) {
        check(!accepted, "Explicit dismissal must cancel");
        ++completions;
    });
    check(bool(titlebar), "Present explicit-dismissal confirmation");
    (*titlebar)->dismiss();
    check(completions == 6, "Explicit dismissal completes once");
    auto raw = sw::Dialog::create(*other, {.title = "Mounted dialog"});
    check(bool(raw), "Create raw native surface");
    auto mounted = s::macos::mount(**raw, [] { return s::Button("Dialog action", [] {}); });
    check(!(*raw)->ui_host(), "Surface mounting never wraps the previous UI engine");
    s::macos::unmount(**raw);
    mounted->invalidate();
    (*raw)->close();
    other->close();
    std::cout << "Native alert sheet, parent isolation, composition, action, reentrancy, and lifetime checks "
                 "passed.\n";
}
} // namespace
int main() {
    try {
        std::string failure;
        auto created = sw::Application::create(
            {.name = "SCARS Alert Tests", .on_error = [&](const auto& e) { failure = e.message; }});
        if (!created)
            throw std::runtime_error(created.error().message);
        auto app = std::move(*created);
        app->post([&] {
            try {
                run(*app);
            } catch (const std::exception& e) {
                failure = e.what();
            }
            app->quit();
        });
        app->run();
        if (!failure.empty())
            throw std::runtime_error(failure);
        return 0;
    } catch (const std::exception& e) {
        std::cerr << "SCARS native alerts: " << e.what() << '\n';
        return 1;
    }
}
