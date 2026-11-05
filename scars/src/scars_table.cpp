module;
#include <cmath>
#include <functional>
#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <unordered_map>
#include <utility>
#include <vector>
module scars;
namespace scars {
struct TableModel::Impl {
    RowModel rows;
    std::vector<TableColumn> columns;
    std::unordered_map<std::string, std::size_t> indices;
};
TableModel::TableModel() : impl_(std::make_shared<Impl>()) {}
TableModel::TableModel(std::vector<Option> rows, std::vector<TableColumn> columns) {
    auto model = std::make_shared<Impl>();
    model->rows = RowModel(std::move(rows));
    model->columns = std::move(columns);
    for (std::size_t i = 0; i < model->columns.size(); ++i) {
        const auto& col = model->columns[i];
        if (col.id.empty() || !model->indices.emplace(col.id, i).second)
            throw std::invalid_argument("Table column IDs must be nonempty and unique");
        if (!std::isfinite(col.width) || !std::isfinite(col.min_width) || !std::isfinite(col.max_width) ||
            col.min_width < 24 || col.max_width < col.min_width || col.width < col.min_width ||
            col.width > col.max_width)
            throw std::invalid_argument(
                "Table column widths must be finite and within bounds, with minimum >= 24");
    }
    impl_ = std::move(model);
}
const RowModel& TableModel::rows() const {
    return impl_->rows;
}
const std::vector<TableColumn>& TableModel::columns() const {
    return impl_->columns;
}
std::optional<std::size_t> TableModel::column_index(const std::string& id) const {
    const auto it = impl_->indices.find(id);
    return it == impl_->indices.end() ? std::optional<std::size_t>{} : it->second;
}
std::string TableModel::text(std::size_t row, std::size_t column) const {
    const auto& col = impl_->columns.at(column);
    const auto& option = impl_->rows.at(row);
    return col.text ? col.text(option, row) : column == 0 ? option.label : std::string{};
}
} // namespace scars
