#include "include/core/SkBitmap.h"
#include "include/core/SkCanvas.h"
#include "include/core/SkSurface.h"
#include <algorithm>
#include <chrono>
#include <functional>
#include <iostream>
#include <memory>
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
s::Theme theme(bool light = false) {
    auto result = light ? s::Theme::light() : s::Theme{};
    result.font_path = SCARS_FONT_PATH;
    return result;
}
s::InspectionNode keyed(s::Host& host, const std::string& key) {
    for (const auto& node : host.inspect()) if (node.key == key) return node;
    throw std::runtime_error("Missing key: " + key);
}
s::Element element(s::Host& host, const std::string& id) {
    for (const auto& e : host.elements()) if (e.id == id) return e;
    throw std::runtime_error("Missing control element");
}
s::Element named(s::Host& host, const std::string& label) {
    for (const auto& e : host.elements()) if (e.label == label) return e;
    throw std::runtime_error("Missing control label: " + label);
}
void type(s::Host& host, const std::string& id, std::string value) {
    host.focus(id);
    host.text_command(s::TextCommand::select_all);
    host.insert_text(std::move(value));
}
void key(s::Host& host, s::Key value) { host.handle(s::KeyEvent{value}); }
s::Point center(s::Rect r) { return {r.x + r.width / 2, r.y + r.height / 2}; }
void search() {
    auto now = std::chrono::steady_clock::time_point{};
    s::State<std::string> query{""}, other{"other"};
    s::State<bool> hidden{false}, enabled{true}, mounted{true}, readonly{false};
    s::State<int> delay{100};
    std::vector<std::string> searches, submissions;
    int cancelled = 0;
    s::Host host([&] {
        std::vector<s::View> views;
        if (mounted.get()) views.push_back(s::SearchField("Find a station", query.binding(), {
            .debounce = std::chrono::milliseconds(delay.get()), .read_only = readonly.get(),
            .on_search = [&](const auto& q) { searches.push_back(q); },
            .on_submit = [&](const auto& q) { submissions.push_back(q); }, .on_cancel = [&] { ++cancelled; }
        }).key("search").hidden(hidden.get()).enabled(enabled.get()));
        views.push_back(s::TextField("Other", other.binding()).key("other"));
        views.push_back(s::Button("Leave", [] {}).key("leave"));
        return s::Block(std::move(views)).gap(12).padding(12);
    }, {}, theme(), {}, {.clock = [&] { return now; }});
    host.layout({420, 220});
    const auto id = keyed(host, "search").id;
    require(keyed(host, "search").type == "SearchField" && element(host, id).search && searches.empty(),
            "Search fields identify their semantics without firing an initial search");
    host.focus(keyed(host, "other").id);
    require(host.handle(s::KeyEvent{s::Key::f, true, false, false, true}) && element(host, id).focused,
            "Command-F focuses a search field in the current host");
    type(host, id, "a");
    require(query.get() == "a" && searches.empty() && host.next_wake() == 100ms && keyed(host, "search").search_pending,
            "Search typing updates the binding immediately and schedules a debounced callback");
    now += 60ms;
    type(host, id, "ab");
    now += 99ms; host.advance_time();
    require(searches.empty() && host.next_wake() == 1ms, "A newer query replaces the previous deadline");
    now += 1ms; host.advance_time();
    require(searches == std::vector<std::string>{"ab"} && !host.next_wake(), "Debounce delivers only the latest query once");
    type(host, id, "beta");
    host.focus(keyed(host, "leave").id);
    now += 100ms; host.advance_time();
    require(searches.back() == "beta" && !host.needs_frame(), "A visible search can finish after field blur without continuous frames");
    type(host, id, "submit");
    key(host, s::Key::enter);
    const auto count_after_submit = searches.size();
    require(searches.back() == "submit" && submissions == std::vector<std::string>{"submit"} && !host.next_wake(),
            "Explicit submission flushes pending search and passes the accepted query");
    now += 1s; host.advance_time();
    require(searches.size() == count_after_submit, "Submitting cancels the old debounce deadline");
    type(host, id, "clear me");
    const auto clear = named(host, "Clear search");
    require(clear.bounds.x >= host.text_input()->viewport.x + host.text_input()->viewport.width,
            "The clear action has its own hit region outside text geometry");
    const auto point = center(clear.bounds);
    host.handle({s::PointerKind::down, point}); host.handle({s::PointerKind::up, point});
    require(query.get().empty() && searches.back().empty() && !host.next_wake() && element(host, id).focused,
            "Clear resets the query immediately and preserves editor focus");
    key(host, s::Key::escape);
    require(cancelled == 1, "Escape on an empty search invokes cancellation");
    type(host, id, "pending");
    host.mark_text("に", {3, 0});
    const auto before_composition = searches.size();
    now += 1s; host.advance_time();
    require(query.get() == "pending" && searches.size() == before_composition && !host.next_wake(),
            "Marked composition pauses an older pending search");
    key(host, s::Key::escape);
    require(query.get() == "pending" && cancelled == 1 && host.next_wake() == 100ms,
            "Escape cancels composition before clearing and resumes its previous pending query");
    now += 100ms; host.advance_time();
    require(searches.back() == "pending", "A cancelled composition does not lose the final committed query");
    type(host, id, "");
    host.mark_text("日本", {6, 0});
    const auto submitted_before_ime = submissions.size();
    key(host, s::Key::enter);
    require(query.get() == "日本" && submissions.size() == submitted_before_ime && host.next_wake() == 100ms,
            "Enter first commits an input-method candidate without submitting the search");
    key(host, s::Key::enter);
    require(submissions.back() == "日本" && searches.back() == "日本" && !host.next_wake(),
            "A second Enter submits the committed composition and flushes search");
    host.mark_text("語", {3, 0});
    key(host, s::Key::right);
    require(query.get() == "日本語" && !host.text_input()->marked,
            "Search cursor movement reacquires layout after committing composition");
    type(host, id, "stale");
    query.set("external");
    const auto before_external = searches.size();
    now += 1s; host.advance_time();
    require(searches.size() == before_external && !host.next_wake() && element(host, id).value_text == "external",
            "External query replacement cancels stale callbacks without pretending to be a user edit");
    require(host.focus_search() && host.text_input()->selection.range() == s::TextRange{0, 8},
            "Search focus selects the existing query even when it already owns focus");
    type(host, id, "policy");
    delay.set(10);
    require(!host.next_wake(), "Changing debounce policy cancels its old pending callback");
    delay.set(100);
    for (int mode = 0; mode < 5; ++mode) {
        type(host, id, "query-" + std::to_string(mode));
        const auto before = searches.size();
        if (mode == 0) hidden.set(true);
        if (mode == 1) enabled.set(false);
        if (mode == 2) mounted.set(false);
        if (mode == 3) readonly.set(true);
        if (mode == 4) host.blur();
        now += 1s; host.advance_time();
        require(searches.size() == before && !host.next_wake(),
                "Hiding, disabling, removing, making read-only, or deactivating cancels pending search");
        hidden.set(false); enabled.set(true); mounted.set(true); readonly.set(false); host.set_active(true);
    }
    delay.set(0);
    type(host, id, "instant");
    require(searches.back() == "instant" && !host.next_wake(), "Zero debounce gives immediate incremental search");
    std::string clipboard{"pasted query"};
    host.set_clipboard({[&] { return clipboard; }, [&](const auto& q) { clipboard = q; }});
    host.text_command(s::TextCommand::select_all); host.text_command(s::TextCommand::paste);
    require(query.get() == "pasted query" && searches.back() == "pasted query", "Search paste shares normal text editing and callbacks");
    readonly.set(true);
    require(!host.activate(named(host, "Clear search").id) && !host.set_text(id, "blocked"), "Read-only search rejects clear and text assignment");
    s::Host outside([&] { return s::TextField("", other.binding()); }, {}, theme());
    outside.layout({200, 60});
    require(!outside.handle(s::KeyEvent{s::Key::f, true, false, false, true}) && !outside.focus_search(),
            "Search shortcuts do not claim hosts with no search field");
}
struct RecordingTextEngine : s::TextLayoutEngine {
    std::vector<std::string> inputs;
    std::shared_ptr<s::TextLayoutEngine> inner{s::make_skparagraph_text_engine(SCARS_FONT_PATH)};
    std::shared_ptr<const s::TextLayout> layout(const std::vector<s::TextSpan>& spans, s::TextLayoutOptions options) override {
        for (const auto& span : spans) inputs.push_back(span.text);
        return inner->layout(spans, options);
    }
};
void password() {
    const std::string original = "海🚀e\xcc\x81👩‍👩‍👧‍👦";
    s::State<std::string> value{original};
    s::State<bool> hidden{false}, enabled{true}, readonly{false};
    auto engine = std::make_shared<RecordingTextEngine>();
    int submitted = 0;
    s::Host host([&] { return s::Block{
        s::PasswordField("Access code", value.binding(), {.allow_reveal = true, .read_only = readonly.get(), .max_length = 32,
            .on_submit = [&] { ++submitted; }}).key("password").hidden(hidden.get()).enabled(enabled.get()),
        s::Button("Other", [] {}).key("other")}.padding(12).gap(12); }, {}, theme(), engine);
    host.layout({420, 180});
    const auto id = keyed(host, "password").id;
    require(element(host, id).secure && element(host, id).value_text == "••••" &&
                keyed(host, "password").selection == "••••" && keyed(host, "password").secure,
            "Password semantics and inspection expose one bullet per grapheme");
    require(std::none_of(engine->inputs.begin(), engine->inputs.end(), [&](const auto& text) { return text == original; }),
            "Masked password layout never sends plaintext to the shaping engine");
    host.select_text(id, {0, 0});
    require(host.text_input()->secure && host.text_input()->active && host.text_input()->text == original,
            "The privileged native input bridge retains raw text and a secure marker");
    const auto boundaries = s::grapheme_boundaries(original);
    for (std::size_t i = 1; i < boundaries.size(); ++i) {
        host.text_command(s::TextCommand::right);
        require(host.text_input()->selection.caret == boundaries[i], "Masked cursor movement follows original grapheme boundaries");
        const auto caret = host.text_range_rect({boundaries[i], 0});
        const auto hit = host.text_index_at({caret.x + .1, caret.y + caret.height / 2});
        require(std::find(boundaries.begin(), boundaries.end(), hit) != boundaries.end(),
                "Masked hit testing maps visual positions back to original text boundaries");
    }
    host.text_command(s::TextCommand::backspace);
    require(value.get() == original.substr(0, boundaries[3]) && !host.text_input()->can_undo,
            "Password deletion consumes a complete emoji grapheme without retaining default undo history");
    const auto after_delete = value.get();
    host.text_command(s::TextCommand::undo);
    require(value.get() == after_delete, "Password undo is disabled by default");
    value.set(original);
    host.select_text(id, {0, original.size()});
    std::string clipboard{"untouched clipboard"};
    host.set_clipboard({[&] { return clipboard; }, [&](const auto& text) { clipboard = text; }});
    host.text_command(s::TextCommand::copy); host.text_command(s::TextCommand::cut);
    require(clipboard == "untouched clipboard" && value.get() == original, "Password copy and cut neither export nor delete the secret");
    const auto selection = host.text_input()->selection;
    require(host.activate(named(host, "Show password").id), "Password reveal is available only when explicitly enabled");
    require(keyed(host, "password").revealed && host.text_input()->selection == selection &&
                element(host, id).value_text == "••••" && element(host, id).text_selection == s::TextSelection{},
            "Revealing preserves real selection while keeping accessibility redacted");
    require(keyed(host, "password").selection == "••••" &&
                std::find(engine->inputs.begin(), engine->inputs.end(), original) != engine->inputs.end(),
            "Only explicit visual reveal permits plaintext shaping; inspector data remains masked");
    host.text_command(s::TextCommand::copy);
    require(clipboard == "untouched clipboard", "Revealed passwords still cannot be copied");
    host.activate(named(host, "Hide password").id);
    clipboard = std::string(40, 'x');
    host.text_command(s::TextCommand::paste);
    require(value.get() == std::string(32, 'x') && !host.text_input()->can_undo, "Password paste respects grapheme limits and the undo policy");
    host.select_text(id, {0, value.get().size()});
    host.mark_text("日本", {6, 0});
    require(host.text_input()->marked && element(host, id).value_text == "••", "Password composition remains visually and semantically masked");
    key(host, s::Key::enter);
    require(value.get() == "日本" && submitted == 0 && !host.text_input()->marked,
            "Password Enter accepts a composition before it can submit");
    key(host, s::Key::enter);
    require(submitted == 1, "A subsequent Enter submits the committed password");
    host.activate(named(host, "Show password").id);
    value.set("new external password");
    require(!keyed(host, "password").revealed, "External password replacement remasks the control");
    host.activate(named(host, "Show password").id);
    const auto leave = center(keyed(host, "other").bounds);
    host.handle({s::PointerKind::down, leave}); host.handle({s::PointerKind::up, leave});
    require(!keyed(host, "password").revealed && !host.text_input(), "Leaving a password remasks it and ends native text input focus");
    host.focus(id); host.activate(named(host, "Show password").id);
    host.blur();
    require(!keyed(host, "password").revealed && host.text_input()->secure && !host.text_input()->active,
            "Window deactivation remasks the field and marks native secure focus inactive");
    host.set_active(true);
    for (int mode = 0; mode < 3; ++mode) {
        host.focus(id); host.activate(named(host, "Show password").id);
        if (mode == 0) hidden.set(true);
        if (mode == 1) enabled.set(false);
        if (mode == 2) readonly.set(true);
        const auto node = keyed(host, "password");
        const auto input = host.text_input();
        require(!node.revealed && (!input || !input->editable), "Hidden, disabled, or read-only passwords drop reveal and editable input focus");
        hidden.set(false); enabled.set(true); readonly.set(false);
    }
    s::State<std::string> undo_value{"initial"};
    s::Host undo([&] { return s::Block{
        s::PasswordField("", undo_value.binding(), {.allow_reveal = true, .allow_undo = true}).key("password"),
        s::Button("Leave", [] {}).key("leave")}; }, {}, theme());
    undo.layout({420, 150});
    const auto undo_id = keyed(undo, "password").id;
    undo.select_text(undo_id, {7, 7}); undo.insert_text("!");
    require(undo.text_input()->can_undo, "Applications may explicitly enable password undo while editing");
    undo.text_command(s::TextCommand::undo);
    require(undo_value.get() == "initial", "Opted-in password undo updates the bound value");
    undo.text_command(s::TextCommand::redo);
    undo.focus(named(undo, "Show password").id);
    key(undo, s::Key::space);
    require(keyed(undo, "password").revealed && undo.text_input()->can_undo, "Keyboard reveal stays within the password editing scope");
    undo.focus(keyed(undo, "leave").id); undo.focus(undo_id);
    require(!undo.text_input()->can_undo && !undo.text_input()->can_redo && !keyed(undo, "password").revealed,
            "Password history is cleared on blur even when editing undo was enabled");
    s::TextBuffer buffer("before");
    buffer.mark("候補", {6, 0}); buffer.clear_history();
    require(buffer.marked_range().has_value(), "Clearing undo history preserves active composition state");
    buffer.unmark(false);
    require(buffer.text() == "before", "A composition can still be cancelled after history is cleared");
}
void lifetime_and_rendering() {
    auto now = std::chrono::steady_clock::time_point{};
    s::State<bool> mounted{true};
    s::State<std::string> query{""};
    std::shared_ptr<s::Host> host;
    int searched = 0;
    host = std::make_shared<s::Host>([&]() -> s::View {
        if (!mounted.get()) return {};
        return s::SearchField("Search", query.binding(), {.debounce = 10ms, .on_search = [&](const auto&) {
            ++searched; mounted.set(false); host->inspect();
        }}).key("search");
    }, std::function<void()>{}, theme(), std::shared_ptr<s::TextLayoutEngine>{}, s::HostOptions{.clock = [&] { return now; }});
    host->layout({320, 60});
    type(*host, keyed(*host, "search").id, "remove");
    now += 10ms; host->advance_time();
    require(searched == 1 && host->inspect().empty() && !host->next_wake(), "Search callbacks may remove their own control safely");
    s::State<std::string> secret{"abcd"};
    s::Host password([&] { return s::PasswordField("", secret.binding()).key("password"); }, {}, theme());
    const auto draw = [&](bool light, int scale) {
        password.set_theme(theme(light)); password.layout({320, 60});
        auto surface = SkSurfaces::Raster(SkImageInfo::MakeN32Premul(320 * scale, 60 * scale));
        surface->getCanvas()->scale(scale, scale);
        password.paint(*surface->getCanvas());
        SkBitmap image; image.allocN32Pixels(320 * scale, 60 * scale);
        require(surface->readPixels(image, 0, 0), "Password previews render to a raster surface");
        return image;
    };
    const auto ascii = draw(false, 1);
    secret.set("海🚀e\xcc\x81👩‍👩‍👧‍👦");
    const auto unicode = draw(false, 1);
    bool equal = true;
    for (int y = 0; y < 60; ++y) for (int x = 0; x < 320; ++x)
        equal = equal && ascii.getColor(x, y) == unicode.getColor(x, y);
    require(equal, "Passwords with the same grapheme count paint identically regardless of plaintext");
    const auto light = draw(true, 1), scaled = draw(true, 2);
    require(light.getColor(300, 40) != unicode.getColor(300, 40) && scaled.getColor(600, 80) == light.getColor(300, 40),
            "Password fields support both themes and 2x logical rendering");
}
} // namespace
int main() {
    try {
        search(); password(); lifetime_and_rendering();
        std::cout << "SCARS search/password: " << checks << " checks passed\n";
        return 0;
    } catch (const std::exception& error) {
        std::cerr << "SCARS search/password failed after " << checks << " checks: " << error.what() << '\n';
        return 1;
    }
}
