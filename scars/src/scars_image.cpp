module;
#include <algorithm>
#include <cstdint>
#include <limits>
#include <memory>
#include <span>
#include <stdexcept>
#include <string>
#include "include/core/SkCanvas.h"
#include "include/core/SkColorSpace.h"
#include "include/core/SkData.h"
#include "include/core/SkImage.h"
#include "include/core/SkImageInfo.h"
#include "include/core/SkSamplingOptions.h"
module scars;
namespace scars {
struct Bitmap::Impl {
    sk_sp<SkImage> image;
};
Bitmap::Bitmap() = default;
Bitmap Bitmap::load(const std::string& path) {
    auto data = SkData::MakeFromFileName(path.c_str());
    auto image = data ? SkImages::DeferredFromEncodedData(std::move(data)) : nullptr;
    if (image)
        image = image->makeRasterImage();
    if (!image)
        throw std::runtime_error("Cannot load bitmap: " + path);
    Bitmap result;
    result.impl_ = std::make_shared<Impl>();
    result.impl_->image = std::move(image);
    return result;
}
Bitmap Bitmap::from_rgba(std::size_t width, std::size_t height, std::span<const std::uint8_t> pixels,
                         std::size_t stride) {
    if (!width || !height || width > 32768 || height > 32768)
        throw std::invalid_argument("Bitmap dimensions must be between 1 and 32768");
    if (!stride)
        stride = width * 4;
    if (stride < width * 4 || stride > std::numeric_limits<std::size_t>::max() / height ||
        pixels.size() < stride * height)
        throw std::invalid_argument("RGBA data or row stride is too small");
    auto data = SkData::MakeWithCopy(pixels.data(), stride * height);
    auto info = SkImageInfo::Make(int(width), int(height), kRGBA_8888_SkColorType, kUnpremul_SkAlphaType,
                                  SkColorSpace::MakeSRGB());
    auto image = SkImages::RasterFromData(info, std::move(data), stride);
    if (!image)
        throw std::runtime_error("Cannot create RGBA bitmap");
    Bitmap result;
    result.impl_ = std::make_shared<Impl>();
    result.impl_->image = std::move(image);
    return result;
}
Size Bitmap::size() const {
    return empty() ? Size{} : Size{double(impl_->image->width()), double(impl_->image->height())};
}
bool Bitmap::empty() const {
    return !impl_ || !impl_->image;
}
void Bitmap::paint(SkCanvas& canvas, Rect target, ContentMode mode) const {
    if (empty() || target.width <= 0 || target.height <= 0)
        return;
    const auto intrinsic = size();
    Rect drawn = target;
    if (mode != ContentMode::stretch) {
        const auto sx = target.width / intrinsic.width, sy = target.height / intrinsic.height;
        const auto scale = mode == ContentMode::center ? 1.
                           : mode == ContentMode::fill ? std::max(sx, sy)
                                                       : std::min(sx, sy);
        drawn.width = intrinsic.width * scale;
        drawn.height = intrinsic.height * scale;
        drawn.x += (target.width - drawn.width) / 2;
        drawn.y += (target.height - drawn.height) / 2;
    }
    canvas.save();
    canvas.clipRect(SkRect::MakeXYWH(target.x, target.y, target.width, target.height));
    canvas.drawImageRect(impl_->image, SkRect::MakeXYWH(drawn.x, drawn.y, drawn.width, drawn.height),
                         SkSamplingOptions(SkFilterMode::kLinear), nullptr);
    canvas.restore();
}
} // namespace scars
