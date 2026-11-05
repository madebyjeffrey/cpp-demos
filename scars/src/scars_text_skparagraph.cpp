module;
#include <algorithm>
#include <bit>
#include <cmath>
#include <cstdint>
#include <functional>
#include <limits>
#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <unordered_map>
#include <utility>
#include <vector>
#include "include/core/SkCanvas.h"
#include "include/core/SkFontMgr.h"
#include "include/core/SkString.h"
#include "include/core/SkTypeface.h"
#include "modules/skparagraph/include/FontCollection.h"
#include "modules/skparagraph/include/Paragraph.h"
#include "modules/skparagraph/include/ParagraphBuilder.h"
#include "modules/skparagraph/include/TypefaceFontProvider.h"
#include "modules/skunicode/include/SkUnicode.h"
#include "modules/skunicode/include/SkUnicode_icu.h"
#if defined(__APPLE__)
#include "include/ports/SkFontMgr_mac_ct.h"
#else
#include "include/ports/SkFontMgr_empty.h"
#endif
module scars;

namespace scars {
namespace {
namespace sp = skia::textlayout;
class SkParagraphEngine final : public TextLayoutEngine {
    sk_sp<sp::FontCollection> fonts_{sk_make_sp<sp::FontCollection>()};
    sk_sp<SkUnicode> unicode_{SkUnicodes::ICU::Make()};
    std::string family_;
    std::unordered_map<std::string, std::shared_ptr<const TextLayout>> cache_;
    static void append(std::string& key, double n) {
        auto bits = std::bit_cast<std::uint64_t>(n);
        key.append(reinterpret_cast<const char*>(&bits), sizeof(bits));
    }
    static std::size_t utf16_length(const std::string& text) {
        if (text.size() > static_cast<std::size_t>(std::numeric_limits<int>::max()))
            throw std::invalid_argument("Text is too large");
        auto converted = SkUnicode::convertUtf8ToUtf16(text.data(), static_cast<int>(text.size()));
        if (!text.empty() && converted.empty())
            throw std::invalid_argument("Text must be valid UTF-8");
        return converted.size();
    }
    static sp::TextStyle style(const TextSpan& span, const std::string& family) {
        sp::TextStyle s;
        s.setFontSize(span.font_size);
        s.setFontFamilies({SkString(family)});
        s.setColor(span.color);
        s.setLetterSpacing(span.letter_spacing);
        s.setFontStyle(span.bold ? SkFontStyle::Bold() : SkFontStyle::Normal());
        s.setHeight(span.line_height / span.font_size);
        s.setHeightOverride(true);
        s.setHalfLeading(true);
        return s;
    }

  public:
    explicit SkParagraphEngine(const std::string& path) {
        if (!unicode_)
            throw std::runtime_error("SCARS could not initialize ICU Unicode processing");
#if defined(__APPLE__)
        auto manager = SkFontMgr_New_CoreText(nullptr);
#else
        auto manager = SkFontMgr_New_Custom_Empty();
#endif
        if (!path.empty()) {
            auto face = manager->makeFromFile(path.c_str());
            if (!face)
                throw std::runtime_error("Cannot load SCARS font: " + path);
            auto assets = sk_make_sp<sp::TypefaceFontProvider>();
            family_ = "SCARS Sans";
            assets->registerTypeface(std::move(face), SkString(family_));
            fonts_->setAssetFontManager(assets);
        } else
            family_ = "sans-serif";
        fonts_->setDefaultFontManager(manager);
        fonts_->enableFontFallback();
    }
    std::shared_ptr<const TextLayout> layout(const std::vector<TextSpan>& spans,
                                             TextLayoutOptions options) override {
        if (std::isnan(options.width) || options.width < 0 || options.max_lines < 0)
            throw std::invalid_argument("Invalid text layout constraints");
        std::string key;
        append(key, options.width);
        append(key, static_cast<double>(options.wrap));
        append(key, static_cast<double>(options.overflow));
        append(key, options.max_lines);
        append(key, static_cast<double>(options.align));
        for (auto& span : spans) {
            if (!std::isfinite(span.font_size) || span.font_size <= 0 || !std::isfinite(span.line_height) ||
                span.line_height < 0 || !std::isfinite(span.letter_spacing))
                throw std::invalid_argument("Invalid text style");
            append(key, span.text.size());
            key += span.text;
            append(key, span.font_size);
            append(key, span.line_height);
            append(key, span.letter_spacing);
            append(key, span.bold);
            append(key, span.color);
            append(key, span.box.has_value());
            if (span.box) {
                append(key, span.box->size.width);
                append(key, span.box->size.height);
                append(key, span.box->baseline);
            }
        }
        if (auto i = cache_.find(key); i != cache_.end())
            return i->second;
        sp::ParagraphStyle paragraph_style;
        paragraph_style.setApplyRoundingHack(false);
        paragraph_style.setLetterSpacingByCSSSpec(true);
        if (!spans.empty())
            paragraph_style.setTextStyle(style(spans.front(), family_));
        bool natural_width = options.wrap == TextWrap::no_wrap && options.overflow != TextOverflow::ellipsis;
        paragraph_style.setTextAlign(!natural_width && options.align == Align::center ? sp::TextAlign::kCenter
                                     : !natural_width && options.align == Align::end  ? sp::TextAlign::kRight
                                                                                      : sp::TextAlign::kLeft);
        if (options.max_lines > 0)
            paragraph_style.setMaxLines(options.max_lines);
        if (options.overflow == TextOverflow::ellipsis)
            paragraph_style.setEllipsis(SkString("…"));
        auto builder = sp::ParagraphBuilder::make(paragraph_style, fonts_, unicode_);
        if (!builder)
            throw std::runtime_error("Cannot create SkParagraph builder");
        struct Range {
            unsigned start{}, end{};
        };
        std::vector<Range> ranges;
        std::vector<std::size_t> boxes;
        unsigned utf16 = 0;
        for (std::size_t i = 0; i < spans.size(); ++i) {
            auto& span = spans[i];
            builder->pushStyle(style(span, family_));
            if (span.box) {
                auto b = *span.box;
                if (!std::isfinite(b.size.width) || !std::isfinite(b.size.height) ||
                    !std::isfinite(b.baseline) || b.size.width < 0 || b.size.height < 0)
                    throw std::invalid_argument("Invalid inline box");
                builder->addPlaceholder(sp::PlaceholderStyle(b.size.width, b.size.height,
                                                             sp::PlaceholderAlignment::kBaseline,
                                                             sp::TextBaseline::kAlphabetic, b.baseline));
                ranges.push_back({utf16, utf16 + 1});
                ++utf16;
                boxes.push_back(i);
            } else {
                auto units = static_cast<unsigned>(utf16_length(span.text));
                ranges.push_back({utf16, utf16 + units});
                utf16 += units;
                builder->addText(span.text.data(), span.text.size());
            }
            builder->pop();
        }
        std::shared_ptr<sp::Paragraph> paragraph = builder->Build();
        double width = std::isfinite(options.width) ? std::max(.01, options.width) : 1e8;
        // NoWrap still permits explicit newlines. Ellipsis uses a single constrained line.
        if (options.wrap == TextWrap::no_wrap && options.overflow != TextOverflow::ellipsis)
            paragraph->layout(1e8);
        else
            paragraph->layout(width);
        double shift = 0;
        if (natural_width && std::isfinite(options.width)) {
            double extra = std::max(0., options.width - paragraph->getLongestLine());
            shift = options.align == Align::center ? extra / 2 : options.align == Align::end ? extra : 0;
        }
        auto result = std::make_shared<TextLayout>();
        result->size = {std::min(options.width, static_cast<double>(paragraph->getLongestLine())),
                        paragraph->getHeight()};
        result->baseline = paragraph->getAlphabeticBaseline();
        for (std::size_t i = 0; i < spans.size(); ++i)
            if (!spans[i].box) {
                for (auto& box :
                     paragraph->getRectsForRange(ranges[i].start, ranges[i].end, sp::RectHeightStyle::kMax,
                                                 sp::RectWidthStyle::kTight))
                    result->placements.push_back(
                        {i,
                         {box.rect.x() + shift, box.rect.y(), box.rect.width(), box.rect.height()},
                         false});
            }
        auto placeholders = paragraph->getRectsForPlaceholders();
        for (std::size_t i = 0; i < std::min(boxes.size(), placeholders.size()); ++i) {
            auto r = placeholders[i].rect;
            result->placements.push_back({boxes[i], {r.x() + shift, r.y(), r.width(), r.height()}, true});
        }
        paragraph->visit([&](int, const sp::Paragraph::VisitorInfo* info) {
            if (!info)
                return;
            for (int i = 0; i < info->count; ++i)
                result->glyphs.push_back({info->glyphs[i],
                                          {info->origin.x() + info->positions[i].x() + shift,
                                           info->origin.y() + info->positions[i].y()},
                                          info->utf8Starts[i]});
        });
        std::string document;
        for (const auto& span : spans)
            document += span.box ? "\xef\xbf\xbc" : span.text;
        std::vector<sp::LineMetrics> metrics;
        paragraph->getLineMetrics(metrics);
        for (const auto& line : metrics)
            result->lines.push_back(
                {{utf8_offset(document, line.fStartIndex),
                  utf8_offset(document, line.fEndIndex) - utf8_offset(document, line.fStartIndex)},
                 {line.fLeft + shift, line.fBaseline - line.fAscent, line.fWidth, line.fHeight}});
        const double line_height = spans.empty() ? 18 : spans.front().line_height;
        result->hit_test = [paragraph, document, shift](Point point) {
            const auto position = paragraph->getGlyphPositionAtCoordinate(point.x - shift, point.y);
            return utf8_offset(document, std::max(0, position.position));
        };
        result->selection_rects = [paragraph, document, shift](TextRange range) {
            std::vector<Rect> result;
            const auto start = utf16_offset(document, range.start);
            const auto end = utf16_offset(
                document, range.start + std::min(range.length,
                                                 document.size() - std::min(range.start, document.size())));
            for (const auto& box : paragraph->getRectsForRange(start, end, sp::RectHeightStyle::kMax,
                                                               sp::RectWidthStyle::kTight))
                result.push_back({box.rect.x() + shift, box.rect.y(), box.rect.width(), box.rect.height()});
            return result;
        };
        result->caret_rect = [paragraph, document, metrics, shift, line_height](std::size_t byte) {
            byte = std::min(byte, document.size());
            const auto offset = utf16_offset(document, byte);
            const auto count = utf16_from_utf8(document).size();
            sp::Paragraph::GlyphInfo info;
            double x = shift, y = 0, h = line_height;
            if (offset < count && paragraph->getGlyphInfoAtUTF16Offset(offset, &info)) {
                x += info.fDirection == sp::TextDirection::kRtl ? info.fGraphemeLayoutBounds.right()
                                                                : info.fGraphemeLayoutBounds.left();
                y = info.fGraphemeLayoutBounds.y();
                h = info.fGraphemeLayoutBounds.height();
            } else if (count && paragraph->getGlyphInfoAtUTF16Offset(count - 1, &info)) {
                x += info.fDirection == sp::TextDirection::kRtl ? info.fGraphemeLayoutBounds.left()
                                                                : info.fGraphemeLayoutBounds.right();
                y = info.fGraphemeLayoutBounds.y();
                h = info.fGraphemeLayoutBounds.height();
            }
            int line_index = paragraph->getLineNumberAtUTF16Offset(offset);
            if (line_index < 0 && byte == document.size() && !metrics.empty())
                line_index = int(metrics.size() - 1);
            if (line_index >= 0 && std::size_t(line_index) < metrics.size()) {
                const auto& line = metrics[line_index];
                y = line.fBaseline - line.fAscent;
                h = std::max(line_height, line.fHeight);
            }
            if (!metrics.empty() &&
                (document.empty() || (byte == document.size() && document.back() == '\n'))) {
                const auto& line = metrics.back();
                x = line.fLeft + shift;
                y = line.fBaseline - line.fAscent;
                h = std::max(line_height, line.fHeight);
            }
            return Rect{x, y, 1.5, std::max(1., h)};
        };
        result->paint = [paragraph, shift](SkCanvas& canvas, Point origin) {
            paragraph->paint(&canvas, origin.x + shift, origin.y);
        };
        if (cache_.size() >= 512)
            cache_.clear();
        cache_.emplace(std::move(key), result);
        return result;
    }
};
} // namespace
std::shared_ptr<TextLayoutEngine> make_skparagraph_text_engine(std::string font_path) {
    return std::make_shared<SkParagraphEngine>(font_path);
}
} // namespace scars
