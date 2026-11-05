#include "include/core/SkCanvas.h"
#include "include/core/SkSurface.h"
#include <algorithm>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>
import scars;
namespace s = scars;
namespace {
int checks{};
void require(bool ok, const char* message) { ++checks; if (!ok) throw std::runtime_error(message); }
s::Theme theme(bool light = false) { auto t = light ? s::Theme::light() : s::Theme{}; t.font_path = SCARS_FONT_PATH; return t; }
s::InspectionNode keyed(s::Host& h, const std::string& key) {
    for (const auto& n : h.inspect()) if (n.key == key) return n;
    throw std::runtime_error("Missing key: " + key);
}
s::Element element(s::Host& h, const std::string& id) {
    for (const auto& e : h.elements()) if (e.id == id) return e;
    throw std::runtime_error("Missing element: " + id);
}
std::vector<s::Element> rows(s::Host& h) {
    std::vector<s::Element> result;
    for (const auto& e : h.elements()) if (e.role == s::Role::list_item) result.push_back(e);
    return result;
}
void key(s::Host& h, s::Key value) { h.handle(s::KeyEvent{value}); }
void type(s::Host& h, const std::string& id, std::string value) {
    h.focus(id); h.text_command(s::TextCommand::select_all); h.insert_text(std::move(value));
}
s::Point center(s::Rect r) { return {r.x + r.width / 2, r.y + r.height / 2}; }
void click(s::Host& h, s::Point p) { h.handle(s::PointerEvent{s::PointerKind::down, p}); h.handle(s::PointerEvent{s::PointerKind::up, p}); }
void selection() {
    s::State<std::string> query{"Navigation"}, selected{"nav"};
    s::State<bool> loading{false}, hidden{false}, enabled{true}, mounted{true}, readonly{false};
    s::State<std::vector<s::Option>> options{{{"nav", "Navigation"}, {"sci", "Science"}, {"locked", "Scientific archive", false}, {"scan", "Scanners"}, {"uni", "Straße"}}};
    std::vector<std::string> queries, accepted;
    int cancelled{}, behind{};
    s::Host h([&] { return s::Block({s::ComboBox("Station", query.binding(), selected.binding(), options.get(), {
        .loading = loading.get(), .read_only = readonly.get(), .max_height = 104,
        .on_query = [&](const auto& value) { queries.push_back(value); },
        .on_accept = [&](const auto& id, const auto& value) { accepted.push_back(id + ":" + value); },
        .on_cancel = [&] { ++cancelled; }
    }).key("combo").visible(mounted.get()).hidden(hidden.get()).enabled(enabled.get()),
        s::Button("Behind", [&] { ++behind; }).key("behind")}).padding(12).gap(16); }, {}, theme());
    h.layout({420, 300}); const auto id = keyed(h, "combo").id;
    require(keyed(h, "combo").type == "ComboBox" && element(h, id).combo && !h.popup_open(), "Combo starts as a closed editor");
    h.focus(id); require(h.text_input()->combo, "Combo uses the existing input bridge");
    type(h, id, "sc");
    require(query.get() == "sc" && selected.get() == "nav" && queries == std::vector<std::string>{"sc"}, "Query edits do not commit selection");
    require(h.popup_open() && element(h, id).expanded && rows(h).size() == 3 && keyed(h, "combo").combo->matches == 3, "Filtering includes disabled matches");
    require(element(h, id).focused && h.text_input().has_value(), "Suggestions retain editing focus");
    key(h, s::Key::down);
    require(keyed(h, "combo").combo->highlighted == "scan" && selected.get() == "nav", "Arrows skip disabled suggestions without committing");
    key(h, s::Key::enter);
    require(selected.get() == "scan" && query.get() == "Scanners" && accepted == std::vector<std::string>{"scan:Scanners"} && !h.popup_open(), "Enter commits ID and label once");
    type(h, id, "No station"); key(h, s::Key::enter);
    require(query.get() == "No station" && selected.get() == "scan" && accepted.size() == 1 && element(h, id).invalid, "Strict mode rejects unmatched input");
    bool no_results{}; for (const auto& e : h.elements()) if (e.label == "No matches") no_results = true;
    require(no_results && rows(h).empty(), "No-results status is accessible");
    key(h, s::Key::escape);
    require(query.get() == "Scanners" && !h.popup_open() && cancelled == 1 && !element(h, id).invalid, "Escape restores accepted query");
    type(h, id, "Sci"); key(h, s::Key::tab);
    require(query.get() == "Scanners" && selected.get() == "scan" && !h.popup_open() && element(h, keyed(h, "behind").id).focused, "Tab restores strict draft and skips dropdown action");
    type(h, id, "SCIENCE"); require(rows(h).size() == 1 && rows(h)[0].label == "Science", "Filtering ignores case");
    key(h, s::Key::enter); type(h, id, "STRASSE");
    require(rows(h).size() == 1 && rows(h)[0].label == "Straße", "Filtering supports expanding Unicode case mappings");
    click(h, center(rows(h)[0].bounds));
    require(selected.get() == "uni" && query.get() == "Straße" && element(h, id).focused && !h.popup_open(), "Pointer acceptance retains editor focus");
    type(h, id, "s"); const auto row = rows(h).back(); h.focus(row.id);
    require(element(h, id).focused && h.text_input().has_value() && keyed(h, "combo").combo->highlighted == "uni", "Accessible row focus retains the text client");
    loading.set(true); h.layout({420, 300});
    bool loading_label{}; for (const auto& e : h.elements()) if (e.label == "Loading…") loading_label = true;
    require(loading_label && rows(h).empty() && keyed(h, "combo").combo->loading, "Loading replaces suggestions with accessible status");
    key(h, s::Key::enter); require(selected.get() == "uni", "Loading cannot accept stale results");
    loading.set(false); h.layout({420, 300}); click(h, {410, 290});
    require(!h.popup_open() && query.get() == "Straße" && behind == 0, "Outside dismissal consumes click and restores strict draft");
    type(h, id, "pending"); hidden.set(true); h.layout({420, 300});
    require(query.get() == "Straße" && !h.popup_open(), "Hiding restores strict draft");
    hidden.set(false); h.layout({420, 300}); type(h, id, "pending"); enabled.set(false); h.layout({420, 300});
    require(query.get() == "Straße" && !h.popup_open(), "Disabling restores strict draft");
    enabled.set(true); h.layout({420, 300}); type(h, id, "pending"); readonly.set(true); h.layout({420, 300});
    require(query.get() == "Straße" && !h.popup_open(), "Read-only transition restores strict draft");
    readonly.set(false); h.layout({420, 300}); type(h, id, "pending"); mounted.set(false); h.layout({420, 300});
    require(query.get() == "Straße" && !h.popup_open(), "Removal restores strict draft");
    mounted.set(true); h.layout({420, 300}); type(h, id, "pending"); h.blur();
    require(query.get() == "Straße" && !h.popup_open(), "Window blur restores strict draft");
}
void free_text_and_composition() {
    s::State<std::string> query{"Alpha"}, selected{"a"};
    std::vector<std::string> accepted;
    s::Host h([&] { return s::Block({s::ComboBox("Value", query.binding(), selected.binding(), {{"a", "Alpha"}, {"b", "Beta"}}, {
        .mode = s::ComboBoxMode::free_text, .auto_highlight = false,
        .on_accept = [&](const auto& id, const auto& text) { accepted.push_back(id + ":" + text); }
    }).key("combo"), s::Button("Other", [] {}).key("other")}).padding(12).gap(12); }, {}, theme());
    h.layout({420, 240}); const auto id = keyed(h, "combo").id;
    type(h, id, "Unlisted"); key(h, s::Key::enter);
    require(query.get() == "Unlisted" && selected.get().empty() && accepted == std::vector<std::string>{":Unlisted"}, "Free text accepts unmatched query with empty ID");
    type(h, id, "Draft"); key(h, s::Key::escape);
    require(query.get() == "Unlisted" && selected.get().empty(), "Free-text Escape restores checkpoint");
    type(h, id, "Blur value"); h.focus(keyed(h, "other").id);
    require(query.get() == "Blur value" && selected.get().empty() && accepted.size() == 1, "Free-text blur retains query without explicit acceptance");
    type(h, id, "Be"); key(h, s::Key::down); key(h, s::Key::enter);
    require(query.get() == "Beta" && selected.get() == "b", "Free text can select an explicit suggestion");
    h.text_command(s::TextCommand::select_all); h.mark_text("海", {0, 3});
    require(h.text_input()->marked && !h.popup_open() && query.get() == "Beta", "Composition suspends suggestions without changing committed query");
    key(h, s::Key::enter);
    require(!h.text_input()->marked && query.get() == "海" && selected.get() == "b" && accepted.size() == 2, "Composition Enter precedes combo acceptance");
    key(h, s::Key::enter);
    require(selected.get().empty() && accepted.back() == ":海", "Second Enter accepts composed free text");
    type(h, id, "A"); h.mark_text("字", {0, 3}); key(h, s::Key::escape);
    require(!h.text_input()->marked && h.popup_open() && query.get() == "A", "Composition Escape restores suggestions");
    key(h, s::Key::escape); require(query.get() == "海" && !h.popup_open(), "Second Escape restores combo checkpoint");
}
void geometry_and_model_changes() {
    for (const bool light : {false, true}) {
        s::State<std::string> query{""}, selected{""};
        s::State<bool> reverse{false};
        s::Host h([&] {
            std::vector<s::Option> items;
            for (int i = 0; i < 24; ++i) items.push_back({std::to_string(i), "Choice " + std::to_string(i)});
            if (reverse.get()) std::reverse(items.begin(), items.end());
            return s::Layer({s::ComboBox("Edge", query.binding(), selected.binding(), std::move(items),
                {.filter = s::ComboBoxFilter::none, .max_height = 104}).key("combo").width(200)
                .position(s::Position::absolute).right(4).bottom(4)});
        }, {}, theme(light));
        h.layout({320, 220}); const auto id = keyed(h, "combo").id;
        h.focus(id); key(h, s::Key::down);
        const auto bounds = h.popup_bounds();
        require(bounds.x >= 0 && bounds.y >= 0 && bounds.x + bounds.width <= 320 && bounds.y + bounds.height <= 220 &&
                bounds.y < keyed(h, "combo").bounds.y, "Combo suggestions flip above an edge anchor");
        key(h, s::Key::page_down);
        require(keyed(h, "combo").combo->highlighted == "5" && element(h, id).focused, "Page navigation retains logical editing focus");
        for (int i=0; i<18; ++i) key(h, s::Key::down);
        const auto navigated = element(h, id + "/suggestions/option/23");
        require(navigated.bounds.y >= navigated.clip.y && navigated.bounds.y + navigated.bounds.height <= navigated.clip.y + navigated.clip.height + .001,
                "Keyboard navigation realizes and reveals distant suggestions while keeping editor focus");
        h.focus(id + "/suggestions/option/23");
        const auto last = element(h, id + "/suggestions/option/23");
        require(last.bounds.y >= last.clip.y && last.bounds.y + last.bounds.height <= last.clip.y + last.clip.height + .001,
                "Accessible suggestion focus reveals an offscreen row");
        reverse.set(true);
        require(keyed(h, "combo").combo->highlighted == "23", "Suggestion reordering preserves highlighted stable ID");
        auto surface = SkSurfaces::Raster(SkImageInfo::MakeN32Premul(640, 440));
        surface->getCanvas()->scale(2, 2); h.paint(*surface->getCanvas(), 0);
        query.set("Model push"); h.layout({320, 220});
        require(!h.popup_open() && h.text_input()->text == "Model push", "External query replacement clears stale draft presentation");
        key(h, s::Key::down); selected.set("3"); h.layout({320, 220});
        require(!h.popup_open() && query.get() == "Model push" && selected.get() == "3", "External selected ID stays separate from application-owned query text");
        h.handle(s::KeyEvent{s::Key::down, true, false, false, false, true});
        require(h.popup_open(), "Option-Down explicitly opens suggestions");
        h.handle(s::KeyEvent{s::Key::up, true, false, false, false, true});
        require(!h.popup_open(), "Option-Up closes suggestions without accepting");
        h.select_text(id, {0, 0});
        h.handle(s::KeyEvent{s::Key::down, true, false, true, false, true});
        require(!h.popup_open() && h.text_input()->selection.range().length == query.get().size(),
                "Option-Shift-Down retains normal text-selection behavior");
    }
    s::State<std::string> query{""}, selected{""}; s::State<bool> mounted{true};
    int accepted{};
    s::Host h([&] { return s::ComboBox("Prefix", query.binding(), selected.binding(), {{"a", "Alpha"}, {"b", "Beta"}}, {
        .filter = s::ComboBoxFilter::prefix,
        .on_accept = [&](const auto&, const auto&) { ++accepted; mounted.set(false); }
    }).key("combo").visible(mounted.get()); }, {}, theme());
    h.layout({320, 220}); const auto id = keyed(h, "combo").id;
    type(h, id, "ph"); require(rows(h).empty(), "Prefix filtering rejects interior matches");
    type(h, id, "al"); key(h, s::Key::enter); h.layout({320, 220});
    require(accepted == 1 && !mounted.get() && selected.get() == "a" && !h.popup_open(), "Acceptance callbacks can remove the owning control safely");
    bool caught{};
    try { s::ComboBox invalid("", query.binding(), selected.binding(), {{"a", "A"}, {"a", "B"}}); }
    catch (const std::invalid_argument&) { caught = true; }
    require(caught, "ComboBox rejects duplicate option IDs");
    caught = false;
    try { s::ComboBox invalid("", query.binding(), selected.binding(), {}, {.row_height = 1}); }
    catch (const std::invalid_argument&) { caught = true; }
    require(caught, "ComboBox validates row geometry at construction");
}
}
int main() {
    try { selection(); free_text_and_composition(); geometry_and_model_changes(); std::cout << checks << " combo checks passed\n"; }
    catch (const std::exception& error) { std::cerr << error.what() << '\n'; return 1; }
}
