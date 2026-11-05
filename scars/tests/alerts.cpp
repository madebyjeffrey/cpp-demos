#include "include/core/SkCanvas.h"
#include "include/core/SkSurface.h"
#include <algorithm>
#include <iostream>
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
std::vector<s::AlertAction> actions() {
    return {{"cancel", "Cancel", s::AlertActionRole::cancel},
            {"delete", "Delete", s::AlertActionRole::destructive},
            {"save", "Save", s::AlertActionRole::default_action},
            {"disabled", "Unavailable", s::AlertActionRole::normal, false}};
}
s::InspectionNode keyed(s::Host& host, const std::string& key) {
    for (const auto& node : host.inspect())
        if (node.key == key)
            return node;
    throw std::runtime_error("Missing key: " + key);
}
void transactions() {
    int calls{};
    std::optional<std::string> result;
    s::Alert alert("Save changes?", s::Label("Review the changes before continuing."), actions(),
                   [&](auto selected) {
                       ++calls;
                       result = selected;
                       require(alert.completed() && !alert.choose("delete"),
                               "Completion is terminal before reentrant user callbacks");
                   });
    auto copy = alert;
    require(!alert.completed() && !alert.result(), "New alerts start pending");
    require(!alert.choose("missing") && !alert.choose("disabled") && calls == 0,
            "Unknown and disabled actions cannot complete an alert");
    require(copy.choose("save") && calls == 1 && result == "save" && alert.result() == "save",
            "Copied handles share one transaction and its selected action");
    require(!alert.cancel() && !copy.dismiss() && calls == 1,
            "Subsequent cancellation or dismissal cannot repeat completion");
    s::Alert cancel("Cancel?", s::Label("Content"), actions(), [&](auto id) {
        result = id;
        ++calls;
    });
    require(cancel.cancel() && result == "cancel", "Cancel returns the explicit cancel action ID");
    s::Alert closed("Close?", s::Label("Content"), actions(), [&](auto id) {
        result = id;
        ++calls;
    });
    require(closed.dismiss() && closed.completed() && !result && calls == 3,
            "Dismissal completes once with no selected action");
    bool rejected{};
    try {
        s::Alert bad("Invalid", s::Label(""), {{"x", "A"}, {"x", "B"}});
    } catch (const std::invalid_argument&) {
        rejected = true;
    }
    require(rejected, "Duplicate action IDs are rejected");
    rejected = false;
    try {
        s::Alert bad(
            "Invalid", s::Label(""),
            {{"a", "A", s::AlertActionRole::default_action}, {"b", "B", s::AlertActionRole::default_action}});
    } catch (const std::invalid_argument&) {
        rejected = true;
    }
    require(rejected, "Ambiguous default roles are rejected");
    rejected = false;
    try {
        s::Alert bad("Invalid", s::Label(""),
                     {{"a", "A", s::AlertActionRole::cancel}, {"b", "B", s::AlertActionRole::cancel}});
    } catch (const std::invalid_argument&) {
        rejected = true;
    }
    require(rejected, "Ambiguous cancel roles are rejected");
}
void keyboard_and_editing() {
    for (bool light : {false, true}) {
        s::State<std::string> text{"Draft"};
        s::Alert alert("Save document", s::TextField("Name", text.binding()).key("name"), actions());
        s::Host host([&] { return alert; }, {}, theme(light));
        host.layout({360, 300});
        const auto all = host.elements();
        require(std::count_if(all.begin(), all.end(),
                              [](const auto& e) { return e.role == s::Role::alert; }) == 1,
                "An alert exposes a semantic dialog group");
        auto destructive = std::find_if(all.begin(), all.end(), [](const auto& e) {
            return e.alert_action == s::AlertActionRole::destructive;
        });
        require(destructive != all.end() && destructive->help == "Destructive action" &&
                    destructive->group == "root",
                "Action roles and dialog relationships are accessible");
        host.focus(keyed(host, "name").id);
        host.mark_text("仮", {0, 3});
        host.handle(s::KeyEvent{s::Key::enter});
        require(!alert.completed() && !host.text_input()->marked,
                "Return accepts composition before it can activate the default action");
        host.handle(s::KeyEvent{.key = s::Key::enter, .repeat = true});
        require(!alert.completed(), "Held Return cannot accept the alert after completing composition");
        host.handle(s::KeyEvent{s::Key::enter});
        require(alert.result() == "save",
                "A subsequent Return in a plain single-line field selects the default");
        const auto finished = host.elements();
        require(std::none_of(finished.begin(), finished.end(), [](const auto& e) { return e.enabled; }),
                "Completed alert content becomes disabled without reconstructing its transaction");
        auto surface = SkSurfaces::Raster(SkImageInfo::MakeN32Premul(720, 600));
        surface->getCanvas()->scale(2, 2);
        host.paint(*surface->getCanvas());
        require(!host.needs_frame(), "Static themed alerts settle without continuous rendering");
    }
    s::State<std::string> multiline{"Notes"};
    s::Alert editor("Review", s::TextEditor(multiline.binding(), "Notes").key("notes"), actions());
    s::Host host([&] { return editor; }, {}, theme());
    host.layout({420, 360});
    host.focus(keyed(host, "notes").id);
    host.handle(s::KeyEvent{s::Key::enter});
    require(!editor.completed() && multiline.get().find('\n') != std::string::npos,
            "Return in multiline content inserts a newline without dismissing");
    host.mark_text("仮", {0, 3});
    host.handle(s::KeyEvent{s::Key::escape});
    require(!editor.completed() && !host.text_input()->marked, "Escape cancels marked text before the alert");
    host.handle(s::KeyEvent{s::Key::escape});
    require(editor.result() == "cancel", "A subsequent Escape selects the explicit cancel action");
    s::Alert only_destructive("Delete?", s::Label("Cannot undo"),
                              {{"delete", "Delete", s::AlertActionRole::destructive}});
    s::Host safe([&] { return only_destructive; }, {}, theme());
    safe.layout({360, 220});
    safe.handle(s::KeyEvent{s::Key::enter});
    safe.handle(s::KeyEvent{s::Key::escape});
    require(!only_destructive.completed(),
            "Destructive actions acquire no implicit default or cancel shortcut");
    safe.focus(keyed(safe, "delete").id);
    safe.handle(s::KeyEvent{s::Key::space});
    require(only_destructive.result() == "delete",
            "A deliberately focused destructive action can be activated");
    s::State<bool> shown{true}, enabled{true};
    s::Alert hidden("Hidden", s::Label("Content"), actions());
    s::Host guarded([&] { return hidden.hidden(!shown.get()).enabled(enabled.get()); }, {}, theme());
    guarded.layout({360, 220});
    shown.set(false);
    guarded.handle(s::KeyEvent{s::Key::enter});
    require(!hidden.completed(), "Hidden alerts do not receive default shortcuts");
    shown.set(true);
    enabled.set(false);
    guarded.handle(s::KeyEvent{s::Key::escape});
    require(!hidden.completed(), "Disabled alerts do not receive cancellation shortcuts");
}
void reactive_content() {
    s::State<std::string> message{"Initial"};
    int builds{}, completions{};
    s::Alert alert(
        "Reactive",
        [&] {
            ++builds;
            return s::Label(message.get()).key("message");
        },
        actions(), [&](auto) { ++completions; });
    require(builds == 0, "Content builders are deferred until the host constructs the alert");
    s::Host host([&] { return alert; }, {}, theme());
    host.layout({360, 240});
    message.set("Updated");
    host.layout({360, 240});
    require(keyed(host, "message").text == "Updated" && builds == 2 && !alert.completed(),
            "Reactive content updates without replacing the alert transaction");
    host.handle(s::KeyEvent{s::Key::enter});
    require(alert.result() == "save" && completions == 1,
            "Reactive rebuilds preserve exactly-once completion");
    s::State<int> illegal{0};
    s::Alert impure(
        "Invalid",
        [&] {
            illegal.set(1);
            return s::Label("Invalid");
        },
        actions());
    s::Host pure([&] { return impure; }, {}, theme());
    bool rejected{};
    try {
        pure.layout({360, 240});
    } catch (const std::logic_error&) {
        rejected = true;
    }
    require(rejected && illegal.get() == 0, "Alert content builders obey the construction mutation guard");
}
void overlay_priority() {
    s::State<bool> open{true};
    s::Alert alert("Parent",
                   s::Button("Help", [] {}).popover(open.binding(), [] { return s::Label("Help"); }),
                   actions());
    s::Host host([&] { return alert; }, {}, theme());
    host.layout({400, 320});
    host.handle(s::KeyEvent{s::Key::escape});
    require(!open.get() && !alert.completed(), "Escape dismisses a nested popover before its alert");
    host.handle(s::KeyEvent{s::Key::escape});
    require(alert.result() == "cancel", "The next Escape reaches the containing alert");
}
} // namespace
int main() {
    try {
        transactions();
        keyboard_and_editing();
        overlay_priority();
        reactive_content();
        std::cout << checks << " alert checks passed\n";
        return 0;
    } catch (const std::exception& e) {
        std::cerr << e.what() << '\n';
        return 1;
    }
}
