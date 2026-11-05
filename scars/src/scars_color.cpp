module;
#include <algorithm>
#include <cmath>
#include <cstdint>
#include <optional>
#include <stdexcept>
#include <string>
module scars;
namespace scars {
HSV color_hsv(Color color) {
    const double r = ((color >> 16) & 255) / 255., g = ((color >> 8) & 255) / 255., b = (color & 255) / 255.;
    const auto hi = std::max({r, g, b}), lo = std::min({r, g, b}), delta = hi - lo;
    HSV hsv{0, hi == 0 ? 0 : delta / hi, hi};
    if (delta > 0) {
        hsv.hue = 60 * (hi == r ? (g - b) / delta : hi == g ? (b - r) / delta + 2 : (r - g) / delta + 4);
        if (hsv.hue < 0)
            hsv.hue += 360;
    }
    return hsv;
}
Color color_from_hsv(HSV hsv, std::uint8_t alpha) {
    if (!std::isfinite(hsv.hue) || !std::isfinite(hsv.saturation) || !std::isfinite(hsv.value) ||
        hsv.hue < 0 || hsv.hue > 360 || hsv.saturation < 0 || hsv.saturation > 1 || hsv.value < 0 ||
        hsv.value > 1)
        throw std::invalid_argument("HSV components must be finite: hue 0..360, saturation/value 0..1");
    const auto h = std::fmod(hsv.hue, 360.) / 60, c = hsv.value * hsv.saturation,
               x = c * (1 - std::abs(std::fmod(h, 2.) - 1)), m = hsv.value - c;
    double r{}, g{}, b{};
    if (h < 1) {
        r = c;
        g = x;
    } else if (h < 2) {
        r = x;
        g = c;
    } else if (h < 3) {
        g = c;
        b = x;
    } else if (h < 4) {
        g = x;
        b = c;
    } else if (h < 5) {
        r = x;
        b = c;
    } else {
        r = c;
        b = x;
    }
    const auto byte = [m](double v) {
        return static_cast<Color>(std::lround(std::clamp(v + m, 0., 1.) * 255));
    };
    return (Color(alpha) << 24) | (byte(r) << 16) | (byte(g) << 8) | byte(b);
}
std::string format_color(Color color, bool alpha) {
    constexpr char digits[] = "0123456789ABCDEF";
    std::string result = "#";
    for (int shift : {16, 8, 0}) {
        result += digits[(color >> (shift + 4)) & 15];
        result += digits[(color >> shift) & 15];
    }
    if (alpha) {
        result += digits[(color >> 28) & 15];
        result += digits[(color >> 24) & 15];
    }
    return result;
}
std::optional<Color> parse_color(const std::string& text) {
    auto first = text.find_first_not_of(" \t\r\n"), last = text.find_last_not_of(" \t\r\n");
    if (first == std::string::npos)
        return {};
    if (text[first] == '#')
        ++first;
    const auto count = last >= first ? last - first + 1 : 0;
    if (count != 3 && count != 4 && count != 6 && count != 8)
        return {};
    Color bytes[4]{0, 0, 0, 255};
    const bool short_form = count < 5;
    for (std::size_t i = 0; i < count; ++i) {
        const auto ch = text[first + i];
        const int nibble = ch >= '0' && ch <= '9'   ? ch - '0'
                           : ch >= 'a' && ch <= 'f' ? ch - 'a' + 10
                           : ch >= 'A' && ch <= 'F' ? ch - 'A' + 10
                                                    : -1;
        if (nibble < 0)
            return {};
        if (short_form)
            bytes[i] = nibble * 17;
        else if (i % 2 == 0)
            bytes[i / 2] = nibble << 4;
        else
            bytes[i / 2] |= nibble;
    }
    return (bytes[3] << 24) | (bytes[0] << 16) | (bytes[1] << 8) | bytes[2];
}
} // namespace scars
