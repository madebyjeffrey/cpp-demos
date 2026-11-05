#include "include/core/SkBitmap.h"
#include "include/core/SkCanvas.h"
#include "include/core/SkSurface.h"
#include "include/core/SkStream.h"
#include "include/encode/SkPngEncoder.h"
#include <algorithm>
#include <chrono>
#include <cmath>
#include <cstdint>
#include <filesystem>
#include <functional>
#include <iostream>
#include <limits>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>
import scars;
namespace s = scars;
namespace {
int checks{};
void require(bool value, const std::string& message) {
    ++checks;
    if (!value)
        throw std::runtime_error(message);
}
void throws(const std::function<void()>& fn, const std::string& message) {
    bool threw = false;
    try {
        fn();
    } catch (const std::exception&) {
        threw = true;
    }
    require(threw, message);
}
bool near(double a, double b) {
    return std::abs(a - b) < .1;
}
s::Theme theme(bool light = false) {
    auto t = light ? s::Theme::light() : s::Theme{};
    t.font_path = SCARS_FONT_PATH;
    return t;
}
s::Element named(s::Host& host, const std::string& name) {
    for (const auto& e : host.elements())
        if (e.label == name)
            return e;
    throw std::runtime_error("Missing element: " + name);
}
s::InspectionNode keyed(s::Host& host, const std::string& key) {
    std::string available;
    for (const auto& e : host.inspect())
        if (e.key == key)
            return e;
        else
            available += " " + e.type + "(" + e.key + ")";
    throw std::runtime_error("Missing key: " + key + "; mounted:" + available);
}
void key(s::Host& host, s::Key k, bool shift = false, bool command = false, bool alt = false) {
    host.handle(s::KeyEvent{k, true, false, shift, command, alt});
}
SkBitmap pixels(s::Host& host, int width = 420, int height = 280, double time = 0) {
    auto surface = SkSurfaces::Raster(SkImageInfo::MakeN32Premul(width, height));
    host.layout({double(width), double(height)});
    host.paint(*surface->getCanvas(), time);
    SkBitmap image;
    image.allocN32Pixels(width, height);
    if (!surface->readPixels(image, 0, 0))
        throw std::runtime_error("Cannot read pixels");
    return image;
}
void buffer() {
    const std::string family = "👩‍👩‍👧‍👦";
    s::TextBuffer text("A" + family + "e\xcc\x81🇨🇦");
    const auto end = text.text().size();
    text.command(s::TextCommand::left);
    require(text.selection().caret == end - std::string("🇨🇦").size(),
            "Left moves over a complete flag grapheme");
    text.command(s::TextCommand::backspace);
    require(text.text() == "A" + family + "🇨🇦", "Backspace removes a combining sequence as one grapheme");
    text.command(s::TextCommand::backspace);
    require(text.text() == "A🇨🇦", "Backspace removes a whole emoji family sequence");
    text.undo();
    require(text.text() == "A" + family + "🇨🇦", "Undo restores Unicode text and selection");
    text.redo();
    require(text.text() == "A🇨🇦", "Redo restores Unicode edits");
    text.set_text("");
    text.replace("a", {}, true);
    text.replace("b", {}, true);
    text.replace("c", {}, true);
    text.undo();
    require(text.text().empty(), "Consecutive typing is one undo group");
    text.redo();
    require(text.text() == "abc", "Grouped typing supports redo");
    text.select({0, 2});
    text.replace("Z");
    require(text.text() == "Zc", "Typing replaces the selected range");
    text.set_text("original");
    text.select({0, text.text().size()});
    text.replace("x", {}, true);
    text.replace("y", {}, true);
    text.undo();
    require(text.text() == "original", "Typing over a selection and continuing remains one undo group");
    text.set_text("e\xcc\x81");
    text.replace("x", s::TextRange{1, 0});
    require(text.text() == "xe\xcc\x81",
            "An insertion inside a grapheme snaps without deleting the grapheme");
    text.set_text("one two\nthree");
    text.command(s::TextCommand::document_start);
    text.command(s::TextCommand::word_right);
    require(text.selection().caret == 3, "Word navigation uses Unicode word boundaries");
    text.command(s::TextCommand::word_right, true);
    require(text.selection().range() == s::TextRange{3, 4}, "Shift extends a selection by words");
    text.command(s::TextCommand::backspace);
    require(text.text() == "one\nthree", "Deletion consumes the selection first");
    text.command(s::TextCommand::document_end);
    text.command(s::TextCommand::delete_word_backward);
    require(text.text() == "one\n", "Word deletion stops at a word boundary");
    text.set_text("AB");
    text.select({1, 1});
    text.mark("に", {3, 0});
    require(text.text() == "AにB" && text.committed_text() == "AB" &&
                text.marked_range() == s::TextRange{1, 3},
            "IME preview remains separate from committed text");
    text.mark("日本", {6, 0});
    text.replace("日本");
    require(text.text() == "A日本B" && !text.marked_range(), "IME insertion replaces its marked range");
    text.undo();
    require(text.text() == "AB", "An entire IME composition is one undo transaction");
    text.mark("仮", {3, 0});
    text.unmark(false);
    require(text.text() == "AB", "Cancelling composition restores the original text");
    text.mark("確定", {6, 0});
    text.unmark();
    require(text.committed_text() == "A確定B", "Unmark commits composition");
    text.set_text("external");
    require(!text.can_undo() && !text.marked_range(), "External replacement clears stale editing history");
    const std::string mixed = "海🚀e\xcc\x81";
    require(s::utf8_from_utf16(s::utf16_from_utf8(mixed)) == mixed && s::utf16_offset(mixed, 7) == 3 &&
                s::utf8_offset(mixed, 2) == 3,
            "Native range conversion preserves UTF-8 and snaps split surrogate pairs safely");
    throws([] { s::utf8_from_utf16(std::u16string(1, char16_t(0xd800))); },
           "Unpaired UTF-16 surrogates are rejected");
    throws([] { s::TextBuffer invalid(std::string("\xff")); }, "Malformed UTF-8 is rejected before editing");
}
void fields() {
    s::State<std::string> value{"hello world"}, notes{"one\ntwo\nthree\nfour\nfive\nsix\nseven\neight"},
        read{"locked"};
    s::State<bool> hidden{false};
    int submitted = 0;
    std::string clipboard;
    s::Host host(
        [&] {
            return s::Block{s::TextField("Call sign", value.binding(),
                                         {.max_length = 24, .on_submit = [&] { ++submitted; }})
                                .width(190)
                                .key("field")
                                .hidden(hidden.get())
                                .accessible_label("Call sign"),
                            s::TextEditor(notes.binding(), "Notes")
                                .width(240)
                                .height(90)
                                .key("notes")
                                .accessible_label("Notes"),
                            s::TextField("", read.binding(), {.read_only = true})
                                .width(190)
                                .key("readonly")
                                .accessible_label("Read only")}
                .gap(10)
                .padding(12);
        },
        {}, theme());
    host.set_clipboard({[&] { return clipboard; }, [&](const auto& text) { clipboard = text; }});
    host.layout({420, 280});
    const auto id = keyed(host, "field").id;
    host.focus(id);
    require(host.text_input()->selection.range().length == value.get().size(),
            "Keyboard focus selects single-line field contents");
    key(host, s::Key::v, false, true);
    require(value.get() == "hello world", "Pasting an empty clipboard does not erase the selection");
    key(host, s::Key::c, false, true);
    require(clipboard == "hello world", "Command-C copies the selected text");
    host.select_text(id, {5, 5});
    key(host, s::Key::right, true, false, true);
    require(host.text_input()->selection.range() == s::TextRange{5, 6},
            "Option-Shift-Right extends to the next word boundary");
    key(host, s::Key::x, false, true);
    require(value.get() == "hello" && clipboard == " world", "Command-X cuts selection through the binding");
    key(host, s::Key::z, false, true);
    require(value.get() == "hello world", "Command-Z undoes field edits");
    key(host, s::Key::z, true, true);
    require(value.get() == "hello", "Shift-Command-Z redoes field edits");
    clipboard = "\r\n🚀\tX";
    key(host, s::Key::v, false, true);
    require(value.get() == "hello 🚀 X", "Single-line paste normalizes CRLF and tabs without losing Unicode");
    host.select_text(id, {0, value.get().size()});
    host.insert_text("abcdefghijklmnopqrstuvwxYZ");
    require(value.get() == "abcdefghijklmnopqrstuvwx", "Maximum length constrains a pasted replacement");
    host.set_text(id, "海🚀");
    host.select_text(id, {value.get().size(), value.get().size()});
    host.mark_text("に", {3, 0});
    require(value.get() == "海🚀" && host.text_input()->marked.has_value(),
            "Marked text does not write a provisional binding value");
    host.insert_text("日本");
    require(value.get() == "海🚀日本" && !host.text_input()->marked,
            "Committing marked text updates the binding once");
    key(host, s::Key::enter);
    require(submitted == 1, "Enter invokes the field's submit handler");
    const auto field = keyed(host, "field");
    const auto caret = host.text_range_rect({value.get().size(), 0});
    require(caret.x >= field.content_bounds.x &&
                caret.x + caret.width <= field.content_bounds.x + field.content_bounds.width + 1,
            "The insertion caret is revealed inside a horizontally scrolled field");
    const auto hit = host.text_index_at({caret.x, caret.y + caret.height / 2});
    require(hit == value.get().size(), "SkParagraph hit testing agrees with the Unicode caret position");
    pixels(host);
    require(host.needs_frame(), "A focused editable field schedules caret blinking");
    host.blur();
    pixels(host);
    require(!host.needs_frame(), "Caret animation stops when the window loses focus");
    const auto readonly = keyed(host, "readonly").id;
    host.focus(readonly);
    host.text_command(s::TextCommand::select_all);
    host.text_command(s::TextCommand::copy);
    require(clipboard == "locked" && !host.insert_text("bad") && !host.set_text(readonly, "bad"),
            "Read-only fields support selection and copy while rejecting edits");
    host.text_command(s::TextCommand::cut);
    require(read.get() == "locked", "Read-only cut does not change the value");
    const auto editor = keyed(host, "notes").id;
    host.select_text(editor, {0, 0});
    key(host, s::Key::down);
    require(host.text_input()->selection.caret == 4, "Down preserves the visual column in multiline text");
    key(host, s::Key::end, true);
    require(host.text_input()->selection.range().end() == notes.get().size(),
            "Shift-End selects to the document end");
    host.insert_text("\nnew line");
    require(notes.get() == "one\n\nnew line", "Multiline insertion retains line breaks");
    host.select_text(id, {0, 0});
    host.mark_text("仮", {3, 0});
    hidden.set(true);
    host.layout({420, 280});
    require(!host.text_input(), "Hiding a focused text field disconnects text input");
    hidden.set(false);
    host.focus(id);
    require(!host.text_input()->marked, "Hidden composition cannot reappear when a field is shown again");
}
void unicode_fields() {
    s::State<std::string> value{""};
    s::Host limited([&] { return s::TextField("", value.binding(), {.max_length = 2}).key("limited"); }, {},
                    theme());
    limited.layout({240, 60});
    limited.focus(keyed(limited, "limited").id);
    limited.insert_text("👩‍👩‍👧‍👦🇨🇦X");
    require(value.get() == "👩‍👩‍👧‍👦🇨🇦",
            "Maximum length counts complete graphemes rather than bytes");
    key(limited, s::Key::left);
    const auto position = limited.text_input()->selection.caret;
    require(position == std::string("👩‍👩‍👧‍👦").size(),
            "Field cursor movement does not split a flag sequence");
    s::Host rejecting(
        [] {
            return s::TextField("", s::Binding<std::string>([] { return std::string("accepted"); },
                                                            [](std::string) {}))
                .key("reject");
        },
        {}, theme());
    rejecting.layout({240, 60});
    rejecting.focus(keyed(rejecting, "reject").id);
    rejecting.insert_text("rejected");
    require(rejecting.text_input()->text == "accepted",
            "A rejecting binding remains authoritative over the displayed field value");
    s::State<std::string> rtl{"שלום"};
    s::Host hebrew([&] { return s::TextField("", rtl.binding()).key("rtl"); }, {}, theme());
    hebrew.layout({240, 50});
    const auto id = keyed(hebrew, "rtl").id;
    hebrew.select_text(id, {0, 0});
    const auto before = hebrew.text_range_rect({0, 0});
    key(hebrew, s::Key::left);
    const auto after = hebrew.text_range_rect({hebrew.text_input()->selection.caret, 0});
    require(hebrew.text_input()->selection.caret > 0 && after.x < before.x,
            "Left arrow follows visual order in RTL text");
    s::State<std::string> wrapped{"alpha beta gamma delta epsilon zeta"};
    s::Host wrap([&] { return s::TextEditor(wrapped.binding()).width(100).height(100).key("wrap"); }, {},
                 theme());
    wrap.layout({100, 100});
    wrap.select_text(keyed(wrap, "wrap").id, {0, 0});
    const auto initial_line = wrap.text_range_rect({0, 0});
    key(wrap, s::Key::right, false, true);
    const auto end_line = wrap.text_range_rect({wrap.text_input()->selection.caret, 0});
    require(near(initial_line.y, end_line.y),
            "Command-Right retains the caret at the current wrapped line's end (" +
                std::to_string(initial_line.y) + " -> " + std::to_string(end_line.y) + ")");
    s::State<std::string> editable{"alpha beta"};
    s::Host mouse([&] { return s::TextField("", editable.binding()).key("mouse"); }, {}, theme());
    mouse.layout({300, 60});
    const auto field = keyed(mouse, "mouse");
    mouse.handle(s::PointerEvent{.kind = s::PointerKind::down,
                                 .position = {field.content_bounds.x + 15, field.content_bounds.y + 10},
                                 .click_count = 2});
    mouse.handle(s::PointerEvent{.kind = s::PointerKind::up, .position = {30, 20}});
    require(mouse.text_input()->text.substr(mouse.text_input()->selection.range().start,
                                            mouse.text_input()->selection.range().length) == "alpha",
            "Double click selects a whole word");
    mouse.handle(s::PointerEvent{.kind = s::PointerKind::down,
                                 .position = {field.content_bounds.x + 20, field.content_bounds.y + 10},
                                 .click_count = 3});
    mouse.handle(s::PointerEvent{.kind = s::PointerKind::up});
    require(mouse.text_input()->selection.range().length == editable.get().size(),
            "Triple click selects the line");
    mouse.insert_text("edited");
    editable.set("external");
    mouse.text_command(s::TextCommand::undo);
    require(editable.get() == "external", "External binding updates clear stale field undo history");
}
void collections() {
    s::State<bool> reverse{false}, show{true};
    s::State<std::string> a{"A"}, b{"B"};
    int yes = 0, no = 0;
    s::Host host(
        [&] {
            std::vector<int> order{1, 2};
            if (reverse.get())
                std::reverse(order.begin(), order.end());
            return s::FlexRow{s::Button("Before", [] {}).width(40).height(30).key("before"),
                              s::ForEach(
                                  order, [](int id) { return id; },
                                  [&](int id) {
                                      return s::TextField("", id == 1 ? a.binding() : b.binding())
                                          .width(80)
                                          .height(30)
                                          .accessible_label(id == 1 ? "A" : "B");
                                  })
                                  .key("fields"),
                              s::If(
                                  show.get(),
                                  [&] {
                                      ++yes;
                                      return s::Button("Then", [] {}).width(50).height(30);
                                  },
                                  [&] {
                                      ++no;
                                      return s::View{};
                                  })
                                  .key("if"),
                              s::Button("After", [] {}).width(40).height(30).key("after")}
                .gap(8);
        },
        {}, theme());
    host.layout({420, 120});
    auto first = named(host, "A");
    require(yes == 1 && no == 0, "Conditional views evaluate only the selected builder");
    require(near(first.bounds.x, 48) && near(named(host, "B").bounds.x, 136) &&
                near(named(host, "After").bounds.x, 282),
            "ForEach and If contribute direct flex children with exactly one gap per item");
    host.select_text(first.id, {1, 1});
    host.insert_text("!");
    reverse.set(true);
    host.layout({420, 120});
    require(named(host, "A").id == first.id && named(host, "A").focused &&
                near(named(host, "A").bounds.x, 136),
            "Keyed ForEach preserves text-field focus and identity through reorder");
    host.text_command(s::TextCommand::undo);
    require(a.get() == "A", "Per-field undo survives collection reorder");
    show.set(false);
    host.layout({420, 120});
    require(no > 0 && near(named(host, "After").bounds.x, 224),
            "An empty conditional branch leaves no ghost spacing");
    s::Host escaped(
        [] {
            return s::Block{s::ForEach(
                std::vector<std::string>{"a/b", "a%2Fb"}, [](const auto& id) { return id; },
                [](const auto& id) { return s::Button(id, [] {}); })};
        },
        {}, theme());
    escaped.layout({200, 100});
    require(named(escaped, "a/b").id != named(escaped, "a%2Fb").id,
            "Collection keys escape path separators without identity collisions");
    throws([] { s::ForEach(std::vector<s::View>{s::Label("A").key("x"), s::Label("B").key("x")}); },
           "ForEach rejects duplicate keys");
}
void lists() {
    s::State<std::string> selected{"a"};
    s::State<std::vector<std::string>> multi{{"a"}};
    int activated = 0;
    const std::vector<s::Option> options{{"a", "Alpha"},   {"b", "Beta"},  {"off", "Disabled", false},
                                         {"c", "Charlie"}, {"d", "Delta"}, {"e", "Echo"}};
    s::Host host(
        [&] {
            return s::FlexRow{
                s::ListBox(selected.binding(), options,
                           {.visible_rows = 3, .on_activate = [&](const auto&) { ++activated; }})
                    .width(180)
                    .key("single")
                    .accessible_label("Single"),
                s::ListBox(multi.binding(), options, {.visible_rows = 3})
                    .width(180)
                    .key("multi")
                    .accessible_label("Multiple")}
                .gap(12);
        },
        {}, theme());
    host.layout({420, 150});
    host.focus(keyed(host, "single").id);
    key(host, s::Key::down);
    require(selected.get() == "b", "List arrows update single selection");
    key(host, s::Key::down);
    require(selected.get() == "c", "List navigation skips disabled rows");
    key(host, s::Key::end);
    require(selected.get() == "e" && named(host, "Echo").clip.height > 0,
            "End selects and reveals the final row");
    key(host, s::Key::enter);
    require(activated == 1, "Enter activates the selected list item");
    host.handle(s::KeyEvent{.text = "b"});
    require(selected.get() == "b", "List type-ahead selects a matching label");
    host.focus(keyed(host, "multi").id);
    key(host, s::Key::down, true);
    require(multi.get() == std::vector<std::string>({"a", "b"}),
            "Shift navigation extends a multiple selection");
    key(host, s::Key::down, true);
    require(multi.get() == std::vector<std::string>({"a", "b", "c"}), "Range selection skips disabled rows");
    key(host, s::Key::a, false, true);
    require(multi.get().size() == 5, "Command-A selects all enabled list rows");
    const auto id = keyed(host, "multi").id;
    const auto row = named(host, "Beta");
    for (const auto& e : host.elements())
        if (e.group == id && e.label == "Beta") {
            host.activate(e.id);
            break;
        }
    const auto remaining = multi.get();
    require(std::find(remaining.begin(), remaining.end(), "b") == remaining.end(),
            "Programmatic row activation toggles multi-selection");
    require(named(host, "Multiple").role == s::Role::list_box, "List semantics expose a list role");
    pixels(host);
}
void wizard_navigation() {
    s::State<std::string> current{"name"}, name{""}, destination{""};
    int finished = 0, cancelled = 0;
    std::vector<std::string> transitions;
    const std::vector<s::WizardStep> steps{
        {"name", "Identity", [&] { return name.get().empty() ? "A name is required." : ""; }},
        {"destination", "Destination",
         [&] { return destination.get().empty() ? "Choose a destination." : ""; }},
        {"review", "Review", {}}};
    s::Wizard wizard(
        current.binding(), steps,
        {.on_finish = [&] { ++finished; },
         .on_cancel = [&] { ++cancelled; },
         .on_change = [&](const auto& from, const auto& to) { transitions.push_back(from + "/" + to); }});
    require(wizard.index() == 0 && !wizard.can_back() && !wizard.is_last(),
            "Wizard starts on its bound step with Back disabled");
    require(!wizard.can_advance() && !wizard.next() && current.get() == "name",
            "Step validation blocks forward navigation");
    require(!wizard.go_to("review") && !wizard.finish(),
            "A wizard cannot skip forward or finish before its final step");
    name.set("Kestrel");
    require(wizard.next() && current.get() == "destination" &&
                transitions == std::vector<std::string>{"name/destination"},
            "Next updates the binding and reports accepted transitions");
    require(!wizard.next() && wizard.back() && name.get() == "Kestrel",
            "Back remains available on an invalid step and preserves model data");
    wizard.next();
    destination.set("outer");
    wizard.next();
    require(wizard.is_last() && wizard.can_advance() && wizard.finish() && finished == 1,
            "Finish runs only after a valid final step");
    name.set("");
    require(wizard.validation_message() == "Identity: A name is required." && !wizard.finish() &&
                finished == 1,
            "Finish rechecks earlier steps and identifies invalid prior data");
    require(wizard.go_to("name") && current.get() == "name",
            "Completed steps can be revisited without forward validation");
    require(wizard.cancel() && cancelled == 1 && current.get() == "name",
            "Cancellation is independent of validation and leaves reset policy to the caller");
    require(!wizard.go_to("missing") && !wizard.go_to("name"),
            "Unknown and already-current review targets are no-ops");
    throws([&] { s::Wizard invalid(current.binding(), {}); }, "A wizard requires at least one step");
    throws([&] { s::Wizard invalid(current.binding(), {{"name", "A", {}}, {"name", "B", {}}}); },
           "Wizard step IDs must be unique");
    throws([&] { s::Wizard invalid(current.binding(), steps, {.next_label = ""}); },
           "Wizard action labels must remain accessible");
    s::State<std::string> bad{"missing"};
    throws([&] { s::Wizard invalid(bad.binding(), steps); }, "A wizard rejects an unknown current step ID");
    current.set("review");
    name.set("valid");
    s::Wizard locked(current.binding(), steps, {.allow_back = false});
    require(!locked.can_back() && !locked.back() && !locked.go_to("name"),
            "Back and review links obey the back policy");
    s::Wizard no_review(current.binding(), steps, {.review_previous = false});
    require(!no_review.go_to("name") && no_review.back(),
            "Direct review can be disabled independently of Back");
    current.set("review");
    s::Wizard no_finish(current.binding(), steps);
    require(!no_finish.can_advance() && !no_finish.finish(),
            "A final step needs a Finish handler before it can be submitted");
    name.set("");
    s::Wizard current_only(current.binding(), steps,
                           {.validate_all_on_finish = false, .on_finish = [&] { ++finished; }});
    require(current_only.finish() && finished == 2,
            "Applications may explicitly choose current-step-only final validation");
    // Drive the actual composite through Host actions, text input, and its step strip.
    current.set("name");
    destination.set("outer");
    s::Host host(
        [&] {
            return s::Panel(s::FlexColumn{s::If(current.get() == "name",
                                                [&] {
                                                    return s::TextField("Name", name.binding())
                                                        .key("name")
                                                        .accessible_label("Wizard name");
                                                }),
                                          s::WizardNavigation(wizard).key("navigation")}
                                .gap(12));
        },
        {}, theme());
    host.layout({420, 320});
    require(!named(host, "Back").enabled && !named(host, "Next").enabled,
            "Wizard navigation renders unavailable actions disabled");
    require(named(host, "Wizard validation: A name is required.").role == s::Role::text,
            "The validation explanation is accessible");
    const auto next = named(host, "Next").id;
    require(!host.activate(next), "Disabled Next cannot activate through the accessibility/action path");
    host.focus(named(host, "1. Identity").id);
    key(host, s::Key::right);
    require(current.get() == "name", "Step-strip arrow navigation cannot skip to a future step");
    host.set_text(named(host, "Wizard name").id, "Enterprise");
    require(named(host, "Next").enabled && host.activate(named(host, "Next").id) &&
                current.get() == "destination",
            "Editing a field reactively enables and advances the wizard");
    require(host.activate(named(host, "Back").id) && name.get() == "Enterprise",
            "Back remounts step content without losing bound values");
    host.activate(named(host, "Next").id);
    host.activate(named(host, "Next").id);
    require(current.get() == "review" && named(host, "Finish").enabled,
            "The last step replaces Next with Finish");
    require(host.activate(named(host, "Finish").id) && finished == 3,
            "The rendered Finish action invokes the completion callback");
    require(host.activate(named(host, "✓ 1. Identity").id) && current.get() == "name",
            "The step strip allows reviewing an earlier step");
    host.activate(named(host, "Cancel").id);
    require(cancelled == 2, "The rendered Cancel action invokes its callback");
    pixels(host, 420, 320);
    host.set_theme(theme(true));
    pixels(host, 420, 320);
    require(!host.needs_frame(), "Wizard navigation settles in either theme without continuous animation");
    // Live checks protect callbacks retained by a stale, nonreactive binding.
    std::string external = "name";
    bool valid = true;
    int stale_finished = 0;
    s::Wizard live(
        s::Binding<std::string>([&] { return external; }, [&](auto value) { external = std::move(value); }),
        {{"name", "Name", [&] { return valid ? "" : "No longer valid"; }}, {"review", "Review", {}}},
        {.on_finish = [&] { ++stale_finished; }});
    s::Host stale([&] { return s::WizardNavigation(live); }, {}, theme());
    stale.layout({420, 240});
    const auto stale_next = named(stale, "Next").id;
    valid = false;
    stale.activate(stale_next);
    require(external == "name", "Next revalidates live data even if a binding did not invalidate the view");
    valid = true;
    stale.invalidate();
    stale.layout({420, 240});
    external = "review";
    stale.activate(stale_next);
    require(external == "review" && stale_finished == 0,
            "Stale actions cannot unexpectedly submit a different current step");
    int rejected_changes = 0;
    s::Wizard rejecting(s::Binding<std::string>([] { return std::string("name"); }, [](std::string) {}),
                        {{"name", "Name", {}}, {"review", "Review", {}}},
                        {.on_change = [&](const auto&, const auto&) { ++rejected_changes; }});
    require(!rejecting.next() && rejected_changes == 0,
            "Rejected binding transitions do not report a step change");
}
void split_views() {
    s::State<double> fraction{.5};
    s::State<s::SplitPane> collapsed{s::SplitPane::none};
    s::State<bool> hidden{false}, enabled{true}, mounted{true}, vertical{false};
    s::State<std::string> text{"retained"};
    s::Host host([&] {
        return s::If(mounted.get(), [&] {
            return s::SplitView(
                s::TextField("", text.binding()).key("left"),
                s::Button("Second pane", [] {}).key("right"), fraction.binding(),
                {.orientation = vertical.get() ? s::Orientation::vertical : s::Orientation::horizontal,
                 .first_min = 80, .second_min = 120, .collapsed = collapsed.binding()})
                .key("panes").grow().hidden(hidden.get()).enabled(enabled.get()).accessible_label("Test panes");
        });
    }, {}, theme());
    host.layout({600, 240});
    auto divider = named(host, "Test panes divider");
    const auto id = divider.id;
    require(divider.role == s::Role::splitter && near(divider.bounds.x, 294) &&
                near(divider.value, .5) && near(divider.min, 80. / 588) && near(divider.max, 468. / 588),
            "Split divider semantics describe the actual bounded pane fraction");
    require(near(keyed(host, "left").bounds.width, 294) && near(keyed(host, "right").bounds.x, 306),
            "Horizontal split reserves a divider between its two panes");
    require(host.cursor({300, 80}) == s::Cursor::resize_horizontal &&
                host.cursor({20, 20}) == s::Cursor::text,
            "Split dividers and text editors expose the appropriate pointer cursors");
    const s::Point grab{divider.bounds.x + 2, 80};
    host.handle({s::PointerKind::down, grab});
    host.handle({s::PointerKind::move, grab});
    require(near(fraction.get(), .5), "Grabbing off-center does not jump the divider");
    host.handle({s::PointerKind::move, {grab.x + 100, grab.y}});
    require(near(named(host, "Test panes divider").bounds.x, 394),
            "Dragging changes the bound split fraction by the pointer delta");
    host.handle({s::PointerKind::move, {10000, -100}});
    require(near(keyed(host, "right").bounds.width, 120) &&
                host.cursor({10000, -100}) == s::Cursor::resize_horizontal,
            "Captured divider dragging remains active outside the viewport and clamps to pane minima");
    key(host, s::Key::escape);
    require(near(fraction.get(), .5) && host.cursor({10000, -100}) == s::Cursor::arrow,
            "Escape cancels divider dragging and restores the initial fraction");
    host.handle({s::PointerKind::up, {10000, -100}});
    require(host.focus(id), "Split dividers accept keyboard and accessibility focus");
    key(host, s::Key::left);
    require(near(named(host, "Test panes divider").bounds.x, 284),
            "Left moves a horizontal divider by its logical-pixel keyboard step");
    key(host, s::Key::right, true);
    require(near(named(host, "Test panes divider").bounds.x, 384),
            "Shift accelerates divider keyboard movement");
    key(host, s::Key::home);
    require(near(keyed(host, "left").bounds.width, 80), "Home selects the first pane minimum");
    key(host, s::Key::end);
    require(near(keyed(host, "right").bounds.width, 120), "End selects the second pane minimum");
    require(host.set_value(id, .5) && host.adjust(id, 1) &&
                near(named(host, "Test panes divider").bounds.x, 304),
            "Accessibility value and increment operations use the same split geometry");
    fraction.set(.5);
    const auto left_id = keyed(host, "left").id;
    host.select_text(left_id, {text.get().size(), text.get().size()});
    host.insert_text("!");
    host.focus(id);
    key(host, s::Key::enter);
    require(collapsed.get() == s::SplitPane::first && keyed(host, "left").hidden &&
                near(keyed(host, "left").bounds.width, 0) && near(keyed(host, "right").bounds.width, 588),
            "Enter collapses and hides the first pane while retaining its mounted identity");
    require(!host.focus(left_id) && !host.text_input() && !host.needs_frame(),
            "Collapsed panes cannot receive focus or keep their caret animation running");
    const auto collapsed_elements = host.elements();
    require(std::none_of(collapsed_elements.begin(), collapsed_elements.end(), [&](const auto& e) { return e.id == left_id; }),
            "Collapsed pane descendants are omitted from accessibility");
    key(host, s::Key::escape);
    require(collapsed.get() == s::SplitPane::none && keyed(host, "left").id == left_id &&
                host.focus(left_id) && host.text_command(s::TextCommand::undo) && text.get() == "retained",
            "Restoring a pane preserves its editor identity and undo history");
    host.focus(id);
    key(host, s::Key::enter, true);
    require(collapsed.get() == s::SplitPane::second && keyed(host, "right").hidden,
            "Shift-Enter can collapse the second pane");
    key(host, s::Key::escape);
    divider = named(host, "Test panes divider");
    host.handle({s::PointerKind::down, {divider.bounds.x + 6, 80}, 0, 0, 0, false, 2});
    require(collapsed.get() == s::SplitPane::first, "Double-clicking a divider toggles first-pane collapse");
    collapsed.set(s::SplitPane::none);
    fraction.set(.25);
    host.layout({112, 100});
    require(near(keyed(host, "left").bounds.width, 40) && near(keyed(host, "right").bounds.width, 60) &&
                near(fraction.get(), .25),
            "Insufficient space scales minima proportionally without overwriting the saved fraction");
    host.layout({600, 240});
    require(near(keyed(host, "left").bounds.width, 147), "Growing the window restores the saved fraction");
    host.layout({5, 0});
    require(near(named(host, "Test panes divider").bounds.width, 5) &&
                near(keyed(host, "left").bounds.width, 0) && !host.set_value(id, .5),
            "A viewport smaller than the divider has finite zero-sized panes");
    host.layout({600, 240});
    vertical.set(true);
    divider = named(host, "Test panes divider");
    require(near(divider.bounds.height, 12) && near(divider.bounds.y, 80) &&
                host.cursor({80, divider.bounds.y + 6}) == s::Cursor::resize_vertical,
            "Vertical splits arrange stacked panes and expose a vertical resize cursor");
    host.focus(id);
    key(host, s::Key::down);
    require(near(named(host, "Test panes divider").bounds.y, 90),
            "Down increases the first pane height in a vertical split");
    const auto saved = fraction.get();
    divider = named(host, "Test panes divider");
    host.handle({s::PointerKind::down, {80, divider.bounds.y + 6}});
    host.handle({s::PointerKind::cancel});
    host.handle({s::PointerKind::move, {80, 2000}});
    require(near(fraction.get(), saved), "Pointer cancellation releases divider capture");
    host.handle({s::PointerKind::down, {80, divider.bounds.y + 6}});
    host.blur();
    host.handle({s::PointerKind::move, {80, 2000}});
    require(near(fraction.get(), saved), "Window deactivation releases divider capture");
    host.set_active(true);
    for (int mode = 0; mode < 3; ++mode) {
        divider = named(host, "Test panes divider");
        host.handle({s::PointerKind::down, {80, divider.bounds.y + 6}});
        if (mode == 0) hidden.set(true);
        if (mode == 1) enabled.set(false);
        if (mode == 2) mounted.set(false);
        host.handle({s::PointerKind::move, {80, 2000}});
        require(near(fraction.get(), saved), "Hiding, disabling, or removing a split cancels divider capture");
        hidden.set(false); enabled.set(true); mounted.set(true);
    }
    vertical.set(false);
    fraction.set(.5);
    const auto split = keyed(host, "then").split;
    require(split && split->orientation == s::Orientation::horizontal && near(split->first_min, 80) &&
                near(split->first_extent, 294), "Inspector snapshots expose split layout properties");
    const auto dark = pixels(host, 600, 240);
    host.set_theme(theme(true));
    const auto light = pixels(host, 600, 240);
    require(dark.getColor(300, 100) != light.getColor(300, 100),
            "Divider rails use the active dark or light palette");
    auto scaled_surface = SkSurfaces::Raster(SkImageInfo::MakeN32Premul(1200, 480));
    scaled_surface->getCanvas()->scale(2, 2);
    host.paint(*scaled_surface->getCanvas());
    SkBitmap scaled;
    scaled.allocN32Pixels(1200, 480);
    require(scaled_surface->readPixels(scaled, 0, 0) &&
                scaled.getColor(600, 200) == light.getColor(300, 100) &&
                near(named(host, "Test panes divider").bounds.x, 294),
            "High-density rendering preserves logical split geometry and divider colors");
    fraction.set(2);
    require(near(keyed(host, "left").bounds.width, 468) && near(fraction.get(), 2),
            "External fractions are constrained for layout without mutating model state");
    fraction.set(std::numeric_limits<double>::quiet_NaN());
    throws([&] { host.layout({600, 240}); }, "Non-finite split fractions are rejected");
    fraction.set(.5);
    host.layout({600, 240});
    s::State<double> scroll_fraction{.5};
    s::State<s::SplitPane> scroll_collapsed{s::SplitPane::none};
    s::Host scrolling([&] {
        std::vector<s::View> rows;
        for (int i = 0; i < 20; ++i)
            rows.push_back(s::Label("Row " + std::to_string(i)).height(30).key("row-" + std::to_string(i)));
        return s::SplitView(s::ScrollView(s::Block(std::move(rows))).key("scroller"),
                            s::Label("Other pane"), scroll_fraction.binding(),
                            {.collapsed = scroll_collapsed.binding()});
    }, {}, theme());
    scrolling.layout({600, 200});
    scrolling.handle({s::PointerKind::scroll, {30, 50}, 0, 0, -150});
    const auto scrolled = keyed(scrolling, "row-0").bounds.y;
    scroll_collapsed.set(s::SplitPane::first);
    scrolling.layout({600, 200});
    scroll_collapsed.set(s::SplitPane::none);
    require(scrolled < 0 && near(keyed(scrolling, "row-0").bounds.y, scrolled),
            "Collapsed split panes retain their scroll position");
    require(scrolling.activate(named(scrolling, "Resize panes").id) &&
                scroll_collapsed.get() == s::SplitPane::first,
            "Accessibility activation follows the configured collapse binding");
    s::Host fixed([&] { return s::SplitView({}, {}, fraction.binding()); }, {}, theme());
    fixed.layout({400, 200});
    require(!fixed.activate(named(fixed, "Resize panes").id),
            "A split without a collapse binding cannot be collapsed by activation");
    throws([&] { s::SplitView({}, {}, fraction.binding(), {.first_min = -1}); },
           "Negative split minima are rejected");
    throws([&] { s::SplitView({}, {}, fraction.binding(), {.divider_width = 0}); },
           "Zero-width split dividers are rejected");
    throws([&] { s::SplitView({}, {}, fraction.binding(), {.keyboard_step = 0}); },
           "Zero split keyboard steps are rejected");
}
void bitmaps_and_theme() {
    std::vector<std::uint8_t> rgba{255, 0, 0, 255, 0, 0, 255, 255};
    auto bitmap = s::Bitmap::from_rgba(2, 1, rgba);
    rgba.assign(rgba.size(), 0);
    s::Host host([&] { return s::Image(bitmap, s::ContentMode::stretch).accessible_label("Test bitmap"); },
                 {}, theme());
    auto image = pixels(host, 40, 20);
    require(image.getColor(4, 10) == 0xffff0000 && image.getColor(35, 10) == 0xff0000ff,
            "Bitmap display owns a copy of RGBA data and paints both pixels");
    require(named(host, "Test bitmap").role == s::Role::image && near(bitmap.size().width, 2),
            "Bitmap views expose intrinsic dimensions and image semantics");
    s::Host sized(
        [&] { return s::Block{s::Image(bitmap).width(200).key("image")}.align_items(s::Align::start); }, {},
        theme());
    sized.layout({420, 280});
    require(near(keyed(sized, "image").bounds.height, 100),
            "A single explicit bitmap dimension preserves its intrinsic aspect ratio");
    throws([] { s::Bitmap::from_rgba(2, 2, std::vector<std::uint8_t>{1}); },
           "Short bitmap buffers are rejected");
    throws([] { s::Bitmap::load("/missing/scars-image.png"); }, "Missing bitmap files produce an error");
    auto path = std::filesystem::temp_directory_path() /
                ("scars-feature-" +
                 std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()) + ".png");
    {
        SkFILEWStream out(path.c_str());
        require(SkPngEncoder::Encode(&out, image.pixmap(), {}), "Bitmap fixture encodes to PNG");
    }
    auto loaded = s::Bitmap::load(path.string());
    std::filesystem::remove(path);
    require(near(loaded.size().width, 40) && near(loaded.size().height, 20),
            "Bitmap loading decodes PNG dimensions");
    s::State<std::string> value{"Kept across themes"};
    s::Host themed(
        [&] {
            return s::Block{s::TextField("", value.binding()).key("field"),
                            s::Button("Apply", [] {}).key("button")}
                .padding(12)
                .gap(12);
        },
        {}, theme());
    auto dark = pixels(themed);
    const auto id = keyed(themed, "field").id;
    themed.select_text(id, {0, 4});
    themed.set_theme(theme(true));
    auto light = pixels(themed);
    require(themed.theme().light_mode && light.getColor(0, 0) != dark.getColor(0, 0),
            "Light mode changes the background palette");
    require(themed.text_input()->selection.range() == s::TextRange{0, 4} &&
                value.get() == "Kept across themes",
            "Theme switching preserves text and selection");
    const auto button = keyed(themed, "button");
    require(button.foreground == 0xff0f1924, "Light-theme accent buttons retain contrasting dark captions");
}
} // namespace
int main() {
    try {
        buffer();
        fields();
        unicode_fields();
        collections();
        lists();
        wizard_navigation();
        split_views();
        bitmaps_and_theme();
        std::cout << "SCARS features: " << checks << " checks passed\n";
        return 0;
    } catch (const std::exception& e) {
        std::cerr << "SCARS features failed after " << checks << " checks: " << e.what() << '\n';
        return 1;
    }
}
