module;
#include <algorithm>
#include <cctype>
#include <limits>
#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>
#include "modules/skunicode/include/SkUnicode.h"
#include "modules/skunicode/include/SkUnicode_icu.h"
module scars;
namespace scars {
namespace {
sk_sp<SkUnicode> unicode() {
    thread_local auto engine = SkUnicodes::ICU::Make();
    if (!engine)
        throw std::runtime_error("Unicode editing requires SkUnicode/ICU");
    return engine;
}
void valid(const std::string& text) {
    if (text.size() > static_cast<std::size_t>(std::numeric_limits<int>::max()))
        throw std::invalid_argument("Text is too large");
    for (std::size_t i = 0; i < text.size();) {
        const auto first = static_cast<unsigned char>(text[i++]);
        if (first < 0x80)
            continue;
        const int count = first >= 0xc2 && first <= 0xdf   ? 1
                          : first >= 0xe0 && first <= 0xef ? 2
                          : first >= 0xf0 && first <= 0xf4 ? 3
                                                           : -1;
        if (count < 0 || text.size() - i < std::size_t(count))
            throw std::invalid_argument("Text must be valid UTF-8");
        std::uint32_t codepoint = first & ((1u << (6 - count)) - 1);
        for (int j = 0; j < count; ++j) {
            const auto next = static_cast<unsigned char>(text[i++]);
            if ((next & 0xc0) != 0x80)
                throw std::invalid_argument("Text must be valid UTF-8");
            codepoint = (codepoint << 6) | (next & 0x3f);
        }
        if ((count == 1 && codepoint < 0x80) || (count == 2 && codepoint < 0x800) ||
            (count == 3 && codepoint < 0x10000) || codepoint > 0x10ffff ||
            (codepoint >= 0xd800 && codepoint <= 0xdfff))
            throw std::invalid_argument("Text must be valid UTF-8");
    }
}
std::vector<std::size_t> breaks(const std::string& text, SkUnicode::BreakType type) {
    valid(text);
    auto iterator = unicode()->makeBreakIterator(type);
    if (!iterator || !iterator->setText(text.data(), int(text.size())))
        throw std::runtime_error("Cannot segment text");
    std::vector<std::size_t> result;
    for (auto i = iterator->first(); !iterator->isDone(); i = iterator->next())
        result.push_back(static_cast<std::size_t>(i));
    if (result.empty() || result.back() != text.size())
        result.push_back(text.size());
    return result;
}
} // namespace
std::string detail::search_key(const std::string& text) {
    valid(text);
    const auto key = unicode()->toUpper(SkString(text.data(), text.size()), "root");
    return {key.data(), key.size()};
}
std::u16string utf16_from_utf8(const std::string& text) {
    valid(text);
    return SkUnicode::convertUtf8ToUtf16(text.data(), int(text.size()));
}
std::vector<std::size_t> grapheme_boundaries(const std::string& text) {
    return breaks(text, SkUnicode::BreakType::kGraphemes);
}
std::string detail::uppercase_text(const std::string& text) {
    valid(text);
    const auto result = unicode()->toUpper(SkString(text));
    return {result.c_str(), result.size()};
}
std::string utf8_from_utf16(const std::u16string& text) {
    if (text.size() > static_cast<std::size_t>(std::numeric_limits<int>::max()))
        throw std::invalid_argument("Text is too large");
    for (std::size_t i = 0; i < text.size(); ++i) {
        if (text[i] >= 0xd800 && text[i] <= 0xdbff) {
            if (++i >= text.size() || text[i] < 0xdc00 || text[i] > 0xdfff)
                throw std::invalid_argument("Text must be valid UTF-16");
        } else if (text[i] >= 0xdc00 && text[i] <= 0xdfff)
            throw std::invalid_argument("Text must be valid UTF-16");
    }
    const auto converted = SkUnicode::convertUtf16ToUtf8(text);
    if (!text.empty() && converted.isEmpty())
        throw std::invalid_argument("Text must be valid UTF-16");
    return {converted.c_str(), converted.size()};
}
std::size_t utf16_offset(const std::string& text, std::size_t offset) {
    valid(text);
    offset = std::min(offset, text.size());
    while (offset && offset < text.size() && (static_cast<unsigned char>(text[offset]) & 0xc0) == 0x80)
        --offset;
    return utf16_from_utf8(text.substr(0, offset)).size();
}
std::size_t utf8_offset(const std::string& text, std::size_t offset) {
    auto converted = utf16_from_utf8(text);
    offset = std::min(offset, converted.size());
    if (offset && offset < converted.size() && converted[offset] >= 0xdc00 && converted[offset] <= 0xdfff)
        --offset;
    return utf8_from_utf16(converted.substr(0, offset)).size();
}
struct TextBuffer::Impl {
    struct Snapshot {
        std::string text;
        TextSelection selection;
    };
    std::string text;
    TextSelection selection;
    std::vector<Snapshot> undo, redo;
    std::optional<Snapshot> before_mark;
    std::optional<TextRange> mark;
    bool typing{};
    std::size_t normalize(std::size_t value) const {
        const auto list = breaks(text, SkUnicode::BreakType::kGraphemes);
        auto it = std::upper_bound(list.begin(), list.end(), std::min(value, text.size()));
        return it == list.begin() ? 0 : *std::prev(it);
    }
    TextRange range(TextRange r) const {
        if (!r.length)
            return {normalize(r.start), 0};
        const auto list = breaks(text, SkUnicode::BreakType::kGraphemes);
        const auto start = normalize(r.start);
        const auto end =
            std::min(text.size(), r.start + std::min(r.length, text.size() - std::min(r.start, text.size())));
        auto i = std::lower_bound(list.begin(), list.end(), end);
        return {start, (i == list.end() ? text.size() : *i) - start};
    }
    void save(Snapshot snapshot) {
        if (undo.size() >= 200)
            undo.erase(undo.begin());
        undo.push_back(std::move(snapshot));
        redo.clear();
    }
    std::size_t word(bool forward, std::size_t caret) const {
        auto iterator = unicode()->makeBreakIterator(SkUnicode::BreakType::kWords);
        iterator->setText(text.data(), int(text.size()));
        std::vector<TextRange> words;
        auto start = iterator->first();
        for (auto end = iterator->next(); !iterator->isDone(); start = end, end = iterator->next())
            if (iterator->status() >= 100)
                words.push_back({std::size_t(start), std::size_t(end - start)});
        if (forward) {
            for (auto r : words)
                if (r.end() > caret)
                    return r.end();
            return text.size();
        }
        for (auto i = words.rbegin(); i != words.rend(); ++i)
            if (i->start < caret)
                return i->start;
        return 0;
    }
};
TextBuffer::TextBuffer(std::string text) : impl_(std::make_unique<Impl>()) {
    set_text(std::move(text));
}
TextBuffer::~TextBuffer() = default;
TextBuffer::TextBuffer(TextBuffer&&) noexcept = default;
TextBuffer& TextBuffer::operator=(TextBuffer&&) noexcept = default;
const std::string& TextBuffer::text() const {
    return impl_->text;
}
std::string TextBuffer::committed_text() const {
    return impl_->before_mark ? impl_->before_mark->text : impl_->text;
}
TextSelection TextBuffer::selection() const {
    return impl_->selection;
}
std::size_t TextBuffer::normalize(std::size_t i) const {
    return impl_->normalize(i);
}
void TextBuffer::select(TextSelection selection) {
    impl_->selection = {normalize(selection.anchor), normalize(selection.caret)};
    impl_->typing = false;
}
void TextBuffer::set_text(std::string text) {
    valid(text);
    impl_->text = std::move(text);
    impl_->selection = {impl_->text.size(), impl_->text.size()};
    impl_->undo.clear();
    impl_->redo.clear();
    impl_->before_mark.reset();
    impl_->mark.reset();
    impl_->typing = false;
}
void TextBuffer::clear_history() {
    impl_->undo.clear();
    impl_->redo.clear();
    impl_->typing = false;
}
void TextBuffer::replace(std::string inserted, std::optional<TextRange> replacement, bool coalesce) {
    valid(inserted);
    auto& s = *impl_;
    auto range = s.range(replacement.value_or(s.mark.value_or(s.selection.range())));
    auto before = s.before_mark.value_or(Impl::Snapshot{s.text, s.selection});
    auto next = s.text;
    next.replace(range.start, range.length, inserted);
    const bool changed = next != before.text;
    if (changed && !(coalesce && s.typing && !s.before_mark && !range.length))
        s.save(before);
    s.text = std::move(next);
    s.before_mark.reset();
    s.mark.reset();
    const auto caret = s.normalize(range.start + inserted.size());
    s.selection = {caret, caret};
    s.typing = coalesce && !inserted.empty() && changed;
}
void TextBuffer::mark(std::string inserted, TextRange selected, std::optional<TextRange> replacement) {
    valid(inserted);
    auto& s = *impl_;
    auto range = s.range(replacement.value_or(s.mark.value_or(s.selection.range())));
    if (!s.before_mark)
        s.before_mark = Impl::Snapshot{s.text, s.selection};
    s.text.replace(range.start, range.length, inserted);
    s.mark = TextRange{range.start, inserted.size()};
    s.selection = {normalize(range.start + std::min(selected.start, inserted.size())),
                   normalize(range.start + std::min(selected.end(), inserted.size()))};
    s.typing = false;
}
std::optional<TextRange> TextBuffer::marked_range() const {
    return impl_->mark;
}
void TextBuffer::unmark(bool commit) {
    auto& s = *impl_;
    if (!s.before_mark)
        return;
    auto before = std::move(*s.before_mark);
    s.before_mark.reset();
    s.mark.reset();
    if (commit) {
        if (s.text != before.text)
            s.save(std::move(before));
    } else {
        s.text = std::move(before.text);
        s.selection = before.selection;
    }
    s.typing = false;
}
bool TextBuffer::can_undo() const {
    return !impl_->undo.empty() || impl_->before_mark.has_value();
}
bool TextBuffer::can_redo() const {
    return !impl_->redo.empty();
}
bool TextBuffer::undo() {
    auto& s = *impl_;
    if (s.before_mark) {
        unmark(false);
        return true;
    }
    if (s.undo.empty())
        return false;
    s.redo.push_back({s.text, s.selection});
    auto before = std::move(s.undo.back());
    s.undo.pop_back();
    s.text = std::move(before.text);
    s.selection = before.selection;
    s.typing = false;
    return true;
}
bool TextBuffer::redo() {
    auto& s = *impl_;
    if (s.redo.empty())
        return false;
    s.undo.push_back({s.text, s.selection});
    auto after = std::move(s.redo.back());
    s.redo.pop_back();
    s.text = std::move(after.text);
    s.selection = after.selection;
    s.typing = false;
    return true;
}
void TextBuffer::command(TextCommand command, bool extend) {
    auto& s = *impl_;
    s.typing = false;
    if (command == TextCommand::undo) {
        undo();
        return;
    }
    if (command == TextCommand::redo) {
        redo();
        return;
    }
    if (command == TextCommand::cancel) {
        unmark(false);
        return;
    }
    if (s.mark)
        unmark(true);
    auto caret = s.selection.caret;
    if (command == TextCommand::select_all) {
        s.selection = {0, s.text.size()};
        return;
    }
    const auto list = breaks(s.text, SkUnicode::BreakType::kGraphemes);
    auto previous = [&] {
        auto i = std::lower_bound(list.begin(), list.end(), caret);
        return i == list.begin() ? 0 : *std::prev(i);
    };
    auto next = [&] {
        auto i = std::upper_bound(list.begin(), list.end(), caret);
        return i == list.end() ? s.text.size() : *i;
    };
    const auto line_start = caret ? s.text.rfind('\n', caret - 1) : std::string::npos;
    const auto line_end = s.text.find('\n', caret);
    const bool deleting =
        command == TextCommand::backspace || command == TextCommand::delete_forward ||
        command == TextCommand::delete_word_backward || command == TextCommand::delete_word_forward ||
        command == TextCommand::delete_line_backward || command == TextCommand::delete_line_forward ||
        command == TextCommand::delete_paragraph_backward || command == TextCommand::delete_paragraph_forward;
    if (deleting && s.selection.range().length) {
        replace("");
        return;
    }
    if (command == TextCommand::transpose) {
        auto end = caret == s.text.size() ? caret : next();
        auto i = std::lower_bound(list.begin(), list.end(), end);
        if (i - list.begin() < 2)
            return;
        auto middle = *std::prev(i), start = *std::prev(i, 2);
        replace(s.text.substr(middle, end - middle) + s.text.substr(start, middle - start),
                TextRange{start, end - start});
        return;
    }
    std::size_t target = caret;
    switch (command) {
    case TextCommand::left:
        target = !extend && s.selection.range().length ? s.selection.range().start : previous();
        break;
    case TextCommand::right:
        target = !extend && s.selection.range().length ? s.selection.range().end() : next();
        break;
    case TextCommand::backspace:
        target = previous();
        break;
    case TextCommand::delete_forward:
        target = next();
        break;
    case TextCommand::word_left:
    case TextCommand::delete_word_backward:
        target = s.word(false, caret);
        break;
    case TextCommand::word_right:
    case TextCommand::delete_word_forward:
        target = s.word(true, caret);
        break;
    case TextCommand::paragraph_start: {
        auto start = line_start;
        if (start != std::string::npos && start + 1 == caret)
            start = start ? s.text.rfind('\n', start - 1) : std::string::npos;
        target = start == std::string::npos ? 0 : start + 1;
        break;
    }
    case TextCommand::paragraph_end: {
        auto end = line_end;
        if (end != std::string::npos && end == caret)
            end = s.text.find('\n', end + 1);
        target = end == std::string::npos ? s.text.size() : end;
        break;
    }
    case TextCommand::line_start:
    case TextCommand::delete_line_backward:
    case TextCommand::delete_paragraph_backward:
        target = line_start == std::string::npos ? 0 : line_start + 1;
        break;
    case TextCommand::line_end:
    case TextCommand::delete_line_forward:
    case TextCommand::delete_paragraph_forward:
        target = line_end == std::string::npos ? s.text.size() : line_end;
        break;
    case TextCommand::document_start:
        target = 0;
        break;
    case TextCommand::document_end:
        target = s.text.size();
        break;
    default:
        return;
    }
    if (command == TextCommand::delete_paragraph_forward && target == caret && caret < s.text.size())
        target = next();
    if (deleting)
        replace("", TextRange{std::min(caret, target), std::max(caret, target) - std::min(caret, target)});
    else
        s.selection = {extend ? s.selection.anchor : target, target};
}
} // namespace scars
