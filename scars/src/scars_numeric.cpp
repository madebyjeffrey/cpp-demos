module;
#include <algorithm>
#include <bit>
#include <cmath>
#include <chrono>
#include <cstdint>
#include <limits>
#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <variant>
#include <vector>
#include <unicode/uchar.h>
#include <unicode/uloc.h>
#include <unicode/unum.h>
module scars;

namespace scars::detail {
namespace {
using Formatter = std::unique_ptr<UNumberFormat, decltype(&unum_close)>;
Formatter formatter(const NumericConfig& config) {
    UErrorCode error = U_ZERO_ERROR;
    Formatter result(unum_open(UNUM_DECIMAL, nullptr, 0, config.locale.c_str(), nullptr, &error), unum_close);
    if (U_FAILURE(error) || !result)
        throw std::invalid_argument("Unsupported numeric locale: " + config.locale);
    // ICU can report a default-resource warning even for an exact locale match.
    // Reject a machine-locale fallback by checking the resolved language instead.
    error = U_ZERO_ERROR;
    const auto* actual = unum_getLocaleByType(result.get(), ULOC_ACTUAL_LOCALE, &error);
    char requested_language[ULOC_LANG_CAPACITY]{}, actual_language[ULOC_LANG_CAPACITY]{};
    uloc_getLanguage(config.locale.c_str(), requested_language, ULOC_LANG_CAPACITY, &error);
    uloc_getLanguage(actual, actual_language, ULOC_LANG_CAPACITY, &error);
    if (U_FAILURE(error) || (config.locale != "root" && std::string(requested_language) != actual_language))
        throw std::invalid_argument("Unsupported numeric locale: " + config.locale);
    unum_setAttribute(result.get(), UNUM_LENIENT_PARSE, 0);
    unum_setAttribute(result.get(), UNUM_GROUPING_USED, config.grouping);
    unum_setAttribute(result.get(), UNUM_PARSE_INT_ONLY, config.min.index() == 0);
    if (config.min.index() == 1) {
        unum_setAttribute(result.get(), UNUM_SIGNIFICANT_DIGITS_USED, 1);
        unum_setAttribute(result.get(), UNUM_MIN_SIGNIFICANT_DIGITS, 1);
        unum_setAttribute(result.get(), UNUM_MAX_SIGNIFICANT_DIGITS, config.significant_digits);
    }
    return result;
}
template<class Fn> std::u16string utf16_result(Fn fn) {
    UErrorCode error = U_ZERO_ERROR;
    const auto length = fn(nullptr, 0, &error);
    if (error != U_BUFFER_OVERFLOW_ERROR && U_FAILURE(error))
        throw std::runtime_error("ICU numeric formatting failed");
    std::u16string output(std::max(0, length), u'\0');
    error = U_ZERO_ERROR;
    fn(output.data(), length, &error);
    if (U_FAILURE(error)) throw std::runtime_error("ICU numeric formatting failed");
    return output;
}
std::u16string symbol(UNumberFormat* format, UNumberFormatSymbol which) {
    return utf16_result([&](UChar* output, int32_t capacity, UErrorCode* error) {
        return unum_getSymbol(format, which, output, capacity, error);
    });
}
ParsedNumber failure(NumericStatus status, std::string message) {
    return {{}, {status, std::move(message), false}};
}
} // namespace
void check_numeric(const NumericConfig& config) {
    if (config.min.index() != config.max.index() || config.min.index() != config.step.index())
        throw std::invalid_argument("Numeric bounds and step must have the same type");
    if (config.min.index() == 0) {
        if (std::get<0>(config.min) > std::get<0>(config.max) || std::get<0>(config.step) <= 0)
            throw std::invalid_argument("Numeric bounds must be ordered and the step positive");
    } else {
        const auto min = std::get<1>(config.min), max = std::get<1>(config.max), step = std::get<1>(config.step);
        if (!std::isfinite(min) || !std::isfinite(max) || !std::isfinite(step) || min > max || step <= 0)
            throw std::invalid_argument("Numeric bounds and step must be finite, ordered, and positive");
    }
    if (config.significant_digits < 1 || config.significant_digits > 17 || config.locale.empty() ||
        config.locale.find('\0') != std::string::npos || config.repeat_delay.count() < 0 ||
        config.repeat_interval.count() <= 0 || config.repeat_delay > std::chrono::hours(24) ||
        config.repeat_interval > std::chrono::hours(24))
        throw std::invalid_argument("Invalid numeric precision, locale, or repeat timing");
    formatter(config); // Validate locale at construction, outside the view's event callbacks.
}
std::string format_number(NumericValue value, const NumericConfig& config) {
    if (value.index() != config.min.index() || (value.index() == 1 && !std::isfinite(std::get<1>(value))))
        throw std::invalid_argument("Numeric values must be finite and match the configured type");
    if (config.format)
        return config.format(value);
    auto format = formatter(config);
    const auto output = utf16_result([&](UChar* output, int32_t capacity, UErrorCode* error) {
        return value.index() == 0
            ? unum_formatInt64(format.get(), std::get<0>(value), output, capacity, nullptr, error)
            : unum_formatDouble(format.get(), std::get<1>(value), output, capacity, nullptr, error);
    });
    return utf8_from_utf16(output);
}
ParsedNumber parse_number(const std::string& text, const NumericConfig& config) {
    if (config.parse)
        return config.parse(text);
    auto input = utf16_from_utf8(text);
    while (!input.empty() && u_isUWhiteSpace(input.front())) input.erase(input.begin());
    while (!input.empty() && u_isUWhiteSpace(input.back())) input.pop_back();
    auto format = formatter(config);
    const auto plus = symbol(format.get(), UNUM_PLUS_SIGN_SYMBOL);
    const auto minus = symbol(format.get(), UNUM_MINUS_SIGN_SYMBOL);
    const auto decimal = symbol(format.get(), UNUM_DECIMAL_SEPARATOR_SYMBOL);
    if (input.empty() || input == u"+" || input == u"-" || input == plus || input == minus ||
        input == decimal || input == minus + decimal || input == plus + decimal)
        return failure(NumericStatus::incomplete, "Enter a number.");
    if (input.starts_with(plus)) input.erase(0, plus.size());
    else if (input.starts_with(u"+")) input.erase(0, 1);
    if (input.size() > static_cast<std::size_t>(std::numeric_limits<int32_t>::max()))
        return failure(NumericStatus::invalid, "Number is too long.");
    int32_t end = 0;
    UErrorCode error = U_ZERO_ERROR;
    NumericValue value;
    if (config.min.index() == 0)
        value = unum_parseInt64(format.get(), input.data(), static_cast<int32_t>(input.size()), &end, &error);
    else
        value = unum_parseDouble(format.get(), input.data(), static_cast<int32_t>(input.size()), &end, &error);
    if (end > 0 && end < static_cast<int32_t>(input.size())) {
        const auto suffix = input.substr(end);
        if (suffix == u"e" || suffix == u"E" || suffix == u"e+" || suffix == u"E+" || suffix == u"e-" || suffix == u"E-")
            return failure(NumericStatus::incomplete, "Complete the exponent.");
    }
    if (U_FAILURE(error) || end != static_cast<int32_t>(input.size()) ||
        (value.index() == 1 && !std::isfinite(std::get<1>(value))))
        return failure(NumericStatus::invalid, "Enter a valid finite number.");
    if (value < config.min || value > config.max)
        return failure(NumericStatus::out_of_range,
                       "Value must be between " + format_number(config.min, config) + " and " + format_number(config.max, config) + ".");
    return {value, {}};
}
std::optional<NumericValue> step_number(NumericValue current, double steps, const NumericConfig& config, std::size_t caret) {
    if (config.step_value)
        return config.step_value(current, steps, caret);
    if (!std::isfinite(steps) || std::floor(steps) != steps || current.index() != config.min.index()) return {};
    if (current.index() == 0) {
        const auto min = std::get<0>(config.min), max = std::get<0>(config.max);
        const auto value = std::clamp(std::get<0>(current), min, max);
        const auto step = static_cast<std::uint64_t>(std::get<0>(config.step));
        const auto magnitude = std::abs(steps);
        const auto room = steps < 0 ? std::uint64_t(value) - std::uint64_t(min)
                                    : std::uint64_t(max) - std::uint64_t(value);
        if (magnitude >= 18446744073709551616.0 || magnitude > static_cast<double>(room / step))
            return NumericValue{steps < 0 ? min : max};
        const auto count = static_cast<std::uint64_t>(magnitude);
        // Recheck with integer arithmetic because conversion of a large quotient to
        // double can round upward. All value arithmetic below is exact modulo 2^64.
        if (count > room / step) return NumericValue{steps < 0 ? min : max};
        const auto bits = steps < 0 ? std::uint64_t(value) - count * step : std::uint64_t(value) + count * step;
        return NumericValue{std::bit_cast<std::int64_t>(bits)};
    }
    const auto value = std::get<1>(current);
    if (!std::isfinite(value)) return {};
    return NumericValue{std::clamp(std::fma(steps, std::get<1>(config.step), value),
                                   std::get<1>(config.min), std::get<1>(config.max))};
}
} // namespace scars::detail
