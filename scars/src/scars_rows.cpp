module;
#include <algorithm>
#include <cmath>
#include <functional>
#include <memory>
#include <optional>
#include <span>
#include <stdexcept>
#include <string>
#include <unordered_map>
#include <utility>
#include <vector>
module scars;
namespace scars {
struct RowModel::Impl {
    std::vector<Option> rows;
    std::unordered_map<std::string, std::size_t> indices;
    std::vector<std::size_t> enabled;
    Builder builder;
};
RowModel::RowModel() = default;
RowModel::RowModel(std::vector<Option> rows, Builder builder) {
    auto result = std::make_shared<Impl>();
    result->rows = std::move(rows);
    result->builder = std::move(builder);
    result->indices.reserve(result->rows.size());
    for (std::size_t i = 0; i < result->rows.size(); ++i) {
        const auto& row = result->rows[i];
        if (row.id.empty() || !result->indices.emplace(row.id, i).second)
            throw std::invalid_argument("RowModel IDs must be nonempty and unique");
        if (row.enabled) result->enabled.push_back(i);
    }
    impl_ = std::move(result);
}
std::size_t RowModel::size() const { return impl_ ? impl_->rows.size() : 0; }
const Option& RowModel::at(std::size_t index) const {
    if (!impl_) throw std::out_of_range("RowModel index");
    return impl_->rows.at(index);
}
std::optional<std::size_t> RowModel::index_of(const std::string& id) const {
    if (!impl_) return {};
    const auto found = impl_->indices.find(id);
    return found == impl_->indices.end() ? std::optional<std::size_t>{} : found->second;
}
std::span<const std::size_t> RowModel::enabled_indices() const {
    return impl_ ? std::span<const std::size_t>(impl_->enabled) : std::span<const std::size_t>{};
}
bool RowModel::has_content() const { return impl_ && bool(impl_->builder); }
View RowModel::build(std::size_t index) const {
    const auto& row = at(index);
    return impl_->builder ? impl_->builder(row, index) : View{};
}
RowWindow row_window(std::size_t count, double offset, double height, double row_height, std::size_t overscan) {
    if (!std::isfinite(offset) || !std::isfinite(height) || !std::isfinite(row_height) ||
        offset < 0 || height < 0 || row_height <= 0 || !std::isfinite(double(count) * row_height))
        throw std::invalid_argument("Virtual row geometry must be finite and nonnegative with positive row height");
    const auto bounded_index = [count](double value) { return value >= double(count) ? count : std::size_t(value); };
    const auto first = bounded_index(std::floor(offset / row_height));
    if (height == 0 || first == count) return {first, first, first, first};
    const auto total = double(count) * row_height;
    const auto last = height >= total - offset ? count : bounded_index(std::ceil((offset + height) / row_height));
    return {first, last, first > overscan ? first - overscan : 0, count - last < overscan ? count : last + overscan};
}
} // namespace scars
