module;
#include <algorithm>
#include <chrono>
#include <cmath>
#include <cstdint>
#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>
#include <unicode/ucal.h>
#include <unicode/uchar.h>
#include <unicode/udat.h>
#include <unicode/udatpg.h>
#include <unicode/ufieldpositer.h>
#include <unicode/uloc.h>
module scars;

namespace scars {
namespace {
using Calendar = std::unique_ptr<UCalendar, decltype(&ucal_close)>;
using Formatter = std::unique_ptr<UDateFormat, decltype(&udat_close)>;
template <class Fn> std::u16string icu_text(Fn fn) {
    UErrorCode error = U_ZERO_ERROR;
    const auto length = fn(nullptr, 0, &error);
    if (error != U_BUFFER_OVERFLOW_ERROR && U_FAILURE(error))
        throw std::invalid_argument("ICU date/time operation failed");
    std::u16string output(std::max(0, length), u'\0');
    error = U_ZERO_ERROR;
    fn(output.data(), length, &error);
    if (U_FAILURE(error))
        throw std::invalid_argument("ICU date/time operation failed");
    return output;
}
void check_locale(const std::string& locale) {
    if (locale.empty() || locale.find('\0') != std::string::npos)
        throw std::invalid_argument("Invalid date/time locale");
    UErrorCode error = U_ZERO_ERROR;
    char language[ULOC_LANG_CAPACITY]{};
    uloc_getLanguage(locale.c_str(), language, sizeof(language), &error);
    Formatter probe(udat_open(UDAT_SHORT, UDAT_SHORT, locale.c_str(), u"UTC", 3, nullptr, 0, &error),
                    udat_close);
    if (U_FAILURE(error) || !probe)
        throw std::invalid_argument("Unsupported date/time locale: " + locale);
    const auto actual = udat_getLocaleByType(probe.get(), ULOC_ACTUAL_LOCALE, &error);
    char resolved[ULOC_LANG_CAPACITY]{};
    uloc_getLanguage(actual, resolved, sizeof(resolved), &error);
    if (U_FAILURE(error) || (locale != "root" && std::string(language) != resolved))
        throw std::invalid_argument("Unsupported date/time locale: " + locale);
}
std::string gregorian_locale(std::string locale) {
    check_locale(locale);
    // Set the calendar keyword instead of inheriting Buddhist/Japanese defaults.
    std::vector<char> buffer(locale.size() + 128);
    std::copy(locale.begin(), locale.end(), buffer.begin());
    UErrorCode error = U_ZERO_ERROR;
    uloc_setKeywordValue("calendar", "gregorian", buffer.data(), buffer.size(), &error);
    if (U_FAILURE(error))
        throw std::invalid_argument("Invalid date/time locale");
    return buffer.data();
}
Calendar calendar(const std::string& locale, const std::string& zone = "UTC") {
    const auto name = utf16_from_utf8(zone);
    if (zone.empty() || zone.find('\0') != std::string::npos)
        throw std::invalid_argument("A named time zone is required");
    UErrorCode error = U_ZERO_ERROR;
    UBool system{};
    // Unlike most ICU text APIs, this pinned entry point rejects null preflight buffers.
    std::vector<UChar> canonical(256);
    auto length = ucal_getCanonicalTimeZoneID(name.data(), name.size(), canonical.data(), canonical.size(),
                                              &system, &error);
    if (error == U_BUFFER_OVERFLOW_ERROR) {
        canonical.resize(length + 1);
        error = U_ZERO_ERROR;
        ucal_getCanonicalTimeZoneID(name.data(), name.size(), canonical.data(), canonical.size(), &system,
                                    &error);
    }
    if (U_FAILURE(error))
        throw std::invalid_argument("Unknown IANA time zone: " + zone);
    if (!system)
        throw std::invalid_argument("Unknown IANA time zone: " + zone);
    Calendar result(ucal_open(name.data(), name.size(), locale.c_str(), UCAL_GREGORIAN, &error), ucal_close);
    if (U_FAILURE(error) || !result)
        throw std::invalid_argument("Cannot create date/time calendar");
    ucal_setGregorianChange(result.get(), -8.64e15, &error);
    ucal_setAttribute(result.get(), UCAL_LENIENT, false);
    if (U_FAILURE(error))
        throw std::invalid_argument("Cannot configure Gregorian calendar");
    return result;
}
std::u16string pattern(const std::string& locale, const std::u16string& skeleton) {
    UErrorCode error = U_ZERO_ERROR;
    std::unique_ptr<UDateTimePatternGenerator, decltype(&udatpg_close)> generator(
        udatpg_open(locale.c_str(), &error), udatpg_close);
    if (U_FAILURE(error) || !generator)
        throw std::invalid_argument("Unsupported date/time pattern");
    return icu_text([&](UChar* out, int32_t capacity, UErrorCode* status) {
        return udatpg_getBestPatternWithOptions(generator.get(), skeleton.data(), skeleton.size(),
                                                UDATPG_MATCH_ALL_FIELDS_LENGTH, out, capacity, status);
    });
}
Formatter formatter(const std::string& locale, const std::u16string& layout) {
    UErrorCode error = U_ZERO_ERROR;
    Formatter result(udat_open(UDAT_PATTERN, UDAT_PATTERN, locale.c_str(), u"UTC", 3, layout.data(),
                               layout.size(), &error),
                     udat_close);
    if (U_FAILURE(error) || !result)
        throw std::invalid_argument("Cannot create date/time formatter");
    auto cal = calendar(locale);
    udat_setCalendar(result.get(), cal.get());
    udat_setLenient(result.get(), false);
    return result;
}
std::u16string time_pattern(const TimePickerOptions& options) {
    return pattern(gregorian_locale(options.locale), options.hour_cycle == HourCycle::h12
                                                         ? options.show_seconds ? u"hhmmssa" : u"hhmma"
                                                     : options.show_seconds ? u"HHmmss"
                                                                            : u"HHmm");
}
std::string formatted(UDateFormat* format, UDate value) {
    return utf8_from_utf16(icu_text([&](UChar* out, int32_t capacity, UErrorCode* error) {
        return udat_format(format, value, out, capacity, nullptr, error);
    }));
}
std::optional<Calendar> parsed(UDateFormat* format, const std::string& text) {
    auto input = utf16_from_utf8(text);
    if (input.empty() || input.size() > 1024)
        return {};
    UErrorCode error = U_ZERO_ERROR;
    Calendar cal(ucal_clone(udat_getCalendar(format), &error), ucal_close);
    if (U_FAILURE(error) || !cal)
        return {};
    ucal_setMillis(cal.get(), 0, &error);
    int32_t end{};
    udat_parseCalendar(format, cal.get(), input.data(), input.size(), &end, &error);
    while (end < static_cast<int32_t>(input.size()) && u_isUWhiteSpace(input[end]))
        ++end;
    if (U_FAILURE(error) || end != static_cast<int32_t>(input.size()))
        return {};
    ucal_getMillis(cal.get(), &error); // Force strict field validation.
    if (U_FAILURE(error))
        return {};
    return cal;
}
LocalDateTime fields(UCalendar* cal) {
    UErrorCode error = U_ZERO_ERROR;
    LocalDateTime result{{ucal_get(cal, UCAL_YEAR, &error), unsigned(ucal_get(cal, UCAL_MONTH, &error) + 1),
                          unsigned(ucal_get(cal, UCAL_DATE, &error))},
                         {unsigned(ucal_get(cal, UCAL_HOUR_OF_DAY, &error)),
                          unsigned(ucal_get(cal, UCAL_MINUTE, &error)),
                          unsigned(ucal_get(cal, UCAL_SECOND, &error))}};
    if (U_FAILURE(error) || ucal_get(cal, UCAL_ERA, &error) != 1 || !valid(result.date) ||
        !valid(result.time))
        throw std::invalid_argument("Date/time lies outside years 0001 through 9999");
    return result;
}
bool four_digit_year(const std::string& text, const std::u16string& layout) {
    std::vector<unsigned> lengths;
    bool digits{};
    for (const auto c : utf16_from_utf8(text)) {
        if (u_charDigitValue(c) >= 0) {
            if (!digits)
                lengths.push_back(0);
            ++lengths.back();
            digits = true;
        } else
            digits = false;
    }
    if (lengths.size() != 3)
        return false;
    std::vector<char16_t> order;
    bool quoted{};
    for (auto c : layout) {
        if (c == u'\'') {
            quoted = !quoted;
            continue;
        }
        if (!quoted && (c == u'y' || c == u'M' || c == u'd') &&
            std::find(order.begin(), order.end(), c) == order.end())
            order.push_back(c);
    }
    const auto year = std::find(order.begin(), order.end(), u'y');
    return year != order.end() && lengths[year - order.begin()] == 4;
}
int field_at(UDateFormat* format, UDate value, std::size_t byte_caret) {
    const auto text = formatted(format, value);
    auto caret = utf16_from_utf8(text.substr(0, std::min(byte_caret, text.size()))).size();
    UErrorCode error = U_ZERO_ERROR;
    std::unique_ptr<UFieldPositionIterator, decltype(&ufieldpositer_close)> iterator(
        ufieldpositer_open(&error), ufieldpositer_close);
    auto output = utf16_from_utf8(text);
    udat_formatForFields(format, value, output.data(), output.size(), iterator.get(), &error);
    if (U_FAILURE(error))
        throw std::runtime_error("Cannot locate date/time component");
    struct Field {
        int kind, start, end;
    };
    std::vector<Field> fields;
    int32_t start{}, end{}, kind{};
    while ((kind = ufieldpositer_next(iterator.get(), &start, &end)) >= 0)
        fields.push_back({kind, start, end});
    std::sort(fields.begin(), fields.end(), [](const auto& a, const auto& b) { return a.start < b.start; });
    for (const auto& field : fields)
        if (caret <= static_cast<std::size_t>(field.end))
            return field.kind;
    return fields.empty() ? UDAT_MINUTE_FIELD : fields.back().kind;
}
NumericValidation date_validation(Date value, const DatePickerOptions& options) {
    if (value < options.min || value > options.max)
        return {NumericStatus::out_of_range, "Date is outside the allowed range."};
    if (options.disabled_date && options.disabled_date(value))
        return {NumericStatus::invalid, "This date is unavailable."};
    if (options.validate) {
        auto error = options.validate(value);
        if (!error.empty())
            return {NumericStatus::invalid, std::move(error)};
    }
    return {};
}
NumericValidation time_validation(Time value, const TimePickerOptions& options) {
    if (value < options.min || value > options.max)
        return {NumericStatus::out_of_range, "Time is outside the allowed range."};
    if (options.validate) {
        auto error = options.validate(value);
        if (!error.empty())
            return {NumericStatus::invalid, std::move(error)};
    }
    return {};
}
} // namespace
bool valid(Date value) {
    return value.year >= 1 && value.year <= 9999 && value.month >= 1 && value.month <= 12 && value.day >= 1 &&
           value.day <= 31 &&
           std::chrono::year_month_day{std::chrono::year{value.year}, std::chrono::month{value.month},
                                       std::chrono::day{value.day}}
               .ok();
}
bool valid(Time value) {
    return value.hour < 24 && value.minute < 60 && value.second < 60;
}
std::string format_date(Date value, std::string locale) {
    if (!valid(value))
        throw std::invalid_argument("Invalid Gregorian date");
    const auto name = gregorian_locale(std::move(locale));
    auto format = formatter(name, pattern(name, u"yyyyMMdd"));
    return formatted(format.get(), static_cast<double>(detail::date_days(value)) * 86400000.);
}
std::optional<Date> parse_date(const std::string& text, std::string locale) {
    const auto name = gregorian_locale(std::move(locale));
    const auto layout = pattern(name, u"yyyyMMdd");
    if (!four_digit_year(text, layout))
        return {};
    auto format = formatter(name, layout);
    auto cal = parsed(format.get(), text);
    if (!cal)
        return {};
    try {
        return fields(cal->get()).date;
    } catch (const std::invalid_argument&) {
        return {};
    }
}
std::string format_time(Time value, TimePickerOptions options) {
    if (!valid(value))
        throw std::invalid_argument("Invalid time of day");
    auto format = formatter(gregorian_locale(options.locale), time_pattern(options));
    return formatted(format.get(), detail::time_seconds(value) * 1000.);
}
std::optional<Time> parse_time(const std::string& text, TimePickerOptions options) {
    auto format = formatter(gregorian_locale(options.locale), time_pattern(options));
    auto cal = parsed(format.get(), text);
    if (!cal)
        return {};
    try {
        return fields(cal->get()).time;
    } catch (const std::invalid_argument&) {
        return {};
    }
}
LocalDateTime local_datetime(std::chrono::sys_seconds instant, const std::string& zone) {
    auto cal = calendar("en_US_POSIX", zone);
    UErrorCode error = U_ZERO_ERROR;
    ucal_setMillis(cal.get(), static_cast<double>(instant.time_since_epoch().count()) * 1000., &error);
    if (U_FAILURE(error))
        throw std::invalid_argument("Cannot convert zoned date/time");
    return fields(cal.get());
}
DateTimeResolution resolve_datetime(LocalDateTime local, const std::string& zone, RepeatedTimePolicy policy) {
    if (!valid(local.date) || !valid(local.time))
        return {{}, "Enter a valid local date and time."};
    auto cal = calendar("en_US_POSIX", zone);
    const auto candidate = [&](UCalendarWallTimeOption choice) -> std::optional<std::chrono::sys_seconds> {
        ucal_clear(cal.get());
        ucal_setAttribute(cal.get(), UCAL_REPEATED_WALL_TIME, choice);
        UErrorCode error = U_ZERO_ERROR;
        ucal_setDateTime(cal.get(), local.date.year, local.date.month - 1, local.date.day, local.time.hour,
                         local.time.minute, local.time.second, &error);
        const auto millis = ucal_getMillis(cal.get(), &error);
        if (U_FAILURE(error) || fields(cal.get()) != local)
            return {};
        return std::chrono::sys_seconds{
            std::chrono::seconds{static_cast<std::int64_t>(std::llround(millis / 1000.))}};
    };
    const auto first = candidate(UCAL_WALLTIME_FIRST), last = candidate(UCAL_WALLTIME_LAST);
    if (!first || !last)
        return {{}, "This local time does not exist in the selected time zone."};
    if (first != last && policy == RepeatedTimePolicy::reject)
        return {{}, "This local time occurs twice; select an earlier or later resolution policy."};
    return {policy == RepeatedTimePolicy::later ? std::max(*first, *last) : std::min(*first, *last), {}};
}

namespace detail {
std::int64_t date_days(Date value) {
    if (!valid(value))
        throw std::invalid_argument("Invalid Gregorian date");
    return std::chrono::sys_days{std::chrono::year_month_day{std::chrono::year{value.year},
                                                             std::chrono::month{value.month},
                                                             std::chrono::day{value.day}}}
        .time_since_epoch()
        .count();
}
Date date_from_days(std::int64_t value) {
    const std::chrono::year_month_day result{std::chrono::sys_days{std::chrono::days{value}}};
    Date date{int(result.year()), unsigned(result.month()), unsigned(result.day())};
    if (!valid(date))
        throw std::invalid_argument("Date lies outside years 0001 through 9999");
    return date;
}
std::int64_t time_seconds(Time value) {
    if (!valid(value))
        throw std::invalid_argument("Invalid time of day");
    return value.hour * 3600 + value.minute * 60 + value.second;
}
Time time_from_seconds(std::int64_t value) {
    if (value < 0 || value >= 86400)
        throw std::invalid_argument("Invalid time of day");
    return {unsigned(value / 3600), unsigned(value / 60 % 60), unsigned(value % 60)};
}
Date shift_month(Date value, int months) {
    if (!valid(value))
        throw std::invalid_argument("Invalid Gregorian date");
    const auto index = std::clamp(std::int64_t(value.year - 1) * 12 + value.month - 1 + months,
                                  std::int64_t{0}, std::int64_t{9999 * 12 - 1});
    const int year = int(index / 12 + 1);
    const unsigned month = unsigned(index % 12 + 1);
    const auto last = std::chrono::year_month_day_last{
        std::chrono::year{year}, std::chrono::month_day_last{std::chrono::month{month}}};
    return {year, month, std::min(value.day, unsigned(last.day()))};
}
NumericConfig date_codec(DatePickerOptions options) {
    if (!valid(options.min) || !valid(options.max) || options.min > options.max)
        throw std::invalid_argument("Date bounds must be valid and ordered");
    format_date(options.min, options.locale);
    NumericConfig config{date_days(options.min),
                         date_days(options.max),
                         std::int64_t{1},
                         options.locale,
                         17,
                         false,
                         options.commit_on_blur,
                         options.read_only,
                         options.on_validation,
                         options.on_submit,
                         std::chrono::milliseconds{400},
                         std::chrono::milliseconds{80},
                         {},
                         {},
                         {},
                         {},
                         {}};
    config.format_id = "date:" + options.locale;
    config.format = [options](NumericValue value) {
        return format_date(date_from_days(std::get<0>(value)), options.locale);
    };
    config.validate = [options](NumericValue value) {
        return date_validation(date_from_days(std::get<0>(value)), options);
    };
    config.parse = [options](const std::string& text) -> ParsedNumber {
        const auto date = parse_date(text, options.locale);
        if (!date)
            return {{},
                    {text.empty() ? NumericStatus::incomplete : NumericStatus::invalid,
                     "Enter a valid date with a four-digit year."}};
        auto validation = date_validation(*date, options);
        return {validation.status == NumericStatus::valid ? std::optional<NumericValue>{date_days(*date)}
                                                          : std::nullopt,
                std::move(validation)};
    };
    config.step_value = [options](NumericValue value, double steps,
                                  std::size_t caret) -> std::optional<NumericValue> {
        if (!std::isfinite(steps) || std::floor(steps) != steps)
            return {};
        auto date = date_from_days(std::get<0>(value));
        auto name = gregorian_locale(options.locale);
        auto fmt = formatter(name, pattern(name, u"yyyyMMdd"));
        const auto field = field_at(fmt.get(), std::get<0>(value) * 86400000., caret);
        if (field == UDAT_YEAR_FIELD || field == UDAT_MONTH_FIELD) {
            const auto months = std::clamp(steps * (field == UDAT_YEAR_FIELD ? 12. : 1.), -120000., 120000.);
            date = shift_month(date, static_cast<int>(months));
        } else {
            const auto days = std::clamp(static_cast<long double>(std::get<0>(value)) + steps,
                                         static_cast<long double>(date_days(options.min)),
                                         static_cast<long double>(date_days(options.max)));
            date = date_from_days(static_cast<std::int64_t>(days));
        }
        date = std::clamp(date, options.min, options.max);
        if (date_validation(date, options).status != NumericStatus::valid)
            return {};
        return NumericValue{date_days(date)};
    };
    return config;
}
NumericConfig time_codec(TimePickerOptions options) {
    if (!valid(options.min) || !valid(options.max) || options.min > options.max || options.minute_step == 0 ||
        options.minute_step > 60 || options.second_step == 0 || options.second_step > 60)
        throw std::invalid_argument("Invalid time bounds or component step");
    format_time(options.min, options);
    NumericConfig config{time_seconds(options.min),
                         time_seconds(options.max),
                         std::int64_t{1},
                         options.locale,
                         17,
                         false,
                         options.commit_on_blur,
                         options.read_only,
                         options.on_validation,
                         options.on_submit,
                         std::chrono::milliseconds{400},
                         std::chrono::milliseconds{80},
                         {},
                         {},
                         {},
                         {},
                         {}};
    config.format_id = "time:" + options.locale + ":" + std::to_string(int(options.hour_cycle)) + ":" +
                       std::to_string(options.show_seconds);
    config.format = [options](NumericValue value) {
        return format_time(time_from_seconds(std::get<0>(value)), options);
    };
    config.validate = [options](NumericValue value) {
        return time_validation(time_from_seconds(std::get<0>(value)), options);
    };
    config.parse = [options](const std::string& text) -> ParsedNumber {
        const auto time = parse_time(text, options);
        if (!time)
            return {
                {},
                {text.empty() ? NumericStatus::incomplete : NumericStatus::invalid, "Enter a valid time."}};
        auto validation = time_validation(*time, options);
        return {validation.status == NumericStatus::valid ? std::optional<NumericValue>{time_seconds(*time)}
                                                          : std::nullopt,
                std::move(validation)};
    };
    config.step_value = [options](NumericValue value, double steps,
                                  std::size_t caret) -> std::optional<NumericValue> {
        if (!std::isfinite(steps) || std::floor(steps) != steps)
            return {};
        auto fmt = formatter(gregorian_locale(options.locale), time_pattern(options));
        const auto field = field_at(fmt.get(), std::get<0>(value) * 1000., caret);
        if (field == UDAT_AM_PM_FIELD) {
            const auto toggle = std::fmod(std::abs(steps), 2.) != 0;
            const auto next = (std::get<0>(value) + (toggle ? 43200 : 0)) % 86400;
            if (time_validation(time_from_seconds(next), options).status != NumericStatus::valid)
                return {};
            return NumericValue{next};
        }
        const auto unit = field == UDAT_HOUR_OF_DAY0_FIELD || field == UDAT_HOUR_OF_DAY1_FIELD ||
                                  field == UDAT_HOUR0_FIELD || field == UDAT_HOUR1_FIELD
                              ? 3600u
                          : field == UDAT_AM_PM_FIELD  ? 43200u
                          : field == UDAT_SECOND_FIELD ? options.second_step
                                                       : options.minute_step * 60;
        const auto next =
            std::clamp(static_cast<long double>(std::get<0>(value)) + static_cast<long double>(steps) * unit,
                       static_cast<long double>(time_seconds(options.min)),
                       static_cast<long double>(time_seconds(options.max)));
        const auto result = static_cast<std::int64_t>(next);
        if (time_validation(time_from_seconds(result), options).status != NumericStatus::valid)
            return {};
        return NumericValue{result};
    };
    return config;
}
CalendarPage calendar_page(Date month, const std::string& locale) {
    month.day = 1;
    if (!valid(month))
        throw std::invalid_argument("Invalid calendar month");
    const auto name = gregorian_locale(locale);
    auto cal = calendar(name);
    auto fmt = formatter(name, pattern(name, u"yyyyMMMM"));
    CalendarPage result;
    result.title = formatted(fmt.get(), date_days(month) * 86400000.);
    result.first_weekday = ucal_getAttribute(cal.get(), UCAL_FIRST_DAY_OF_WEEK) - 1;
    for (unsigned i = 0; i < 7; ++i) {
        const int index = (result.first_weekday + i) % 7 + 1;
        result.weekdays.push_back(utf8_from_utf16(icu_text([&](UChar* out, int32_t cap, UErrorCode* error) {
            return udat_getSymbols(fmt.get(), UDAT_SHORT_WEEKDAYS, index, out, cap, error);
        })));
    }
    const auto first = date_days(month);
    const auto weekday = std::chrono::weekday{std::chrono::sys_days{std::chrono::days{first}}}.c_encoding();
    const auto start = first - (weekday + 7 - result.first_weekday) % 7;
    auto day_format = formatter(name, u"d");
    for (int i = 0; i < 42; ++i) {
        const auto ordinal = start + i;
        const bool available = ordinal >= date_days({1, 1, 1}) && ordinal <= date_days({9999, 12, 31});
        result.days.push_back(available ? std::optional<Date>{date_from_days(ordinal)} : std::nullopt);
        result.day_labels.push_back(available ? formatted(day_format.get(), ordinal * 86400000.) : "");
    }
    return result;
}
} // namespace detail
} // namespace scars
