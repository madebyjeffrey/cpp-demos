module;
#include <algorithm>
#include <functional>
#include <memory>
#include <optional>
#include <span>
#include <stdexcept>
#include <string>
#include <unordered_map>
#include <unordered_set>
#include <utility>
#include <vector>
module scars;
namespace scars {
struct TreeModel::Impl {
    struct Entry {
        TreeItem item;
        std::vector<std::string> children;
        std::function<std::vector<TreeNode>()> loader;
        bool loaded{}, explicit_expandable{};
    };
    std::unordered_map<std::string, Entry> nodes;
    std::vector<std::string> roots;
    // Build a complete staged subtree before publishing it. Failed loaders or
    // duplicate IDs leave the prior snapshot and its materialized branches intact.
    std::vector<std::string> add(std::vector<TreeNode> input, std::string parent = {},
                                 std::size_t depth = 0, bool enabled = true) {
        std::unordered_map<std::string, Entry> staged;
        struct Pending { TreeNode node; std::string parent; std::size_t depth; bool enabled; };
        std::vector<Pending> pending;
        std::vector<std::string> ids;
        for (auto& node : input) {
            ids.push_back(node.id);
            pending.push_back({std::move(node), parent, depth, enabled});
        }
        while (!pending.empty()) {
            auto p = std::move(pending.back()); pending.pop_back();
            auto node = std::move(p.node);
            if (node.id.empty() || nodes.contains(node.id) || staged.contains(node.id))
                throw std::invalid_argument("TreeModel IDs must be nonempty and globally unique");
            if (!node.children.empty() && node.lazy_children)
                throw std::invalid_argument("TreeModel nodes must choose eager or lazy children");
            Entry entry;
            entry.item = {node.id, node.label, p.parent, p.depth, p.enabled && node.enabled,
                          node.expandable || node.loading || !node.children.empty() || bool(node.lazy_children), false, node.loading, node.accessible_label};
            entry.explicit_expandable = node.expandable;
            entry.item.child_count = node.children.size();
            entry.loader = std::move(node.lazy_children);
            entry.loaded = !entry.loader;
            for (auto& child : node.children) {
                entry.children.push_back(child.id);
                pending.push_back({std::move(child), node.id, p.depth + 1, entry.item.enabled});
            }
            staged.emplace(node.id, std::move(entry));
        }
        nodes.reserve(nodes.size() + staged.size());
        nodes.merge(staged);
        return ids;
    }
    void load(const std::string& id) {
        auto& entry = nodes.at(id);
        if (entry.loaded || entry.item.loading) return;
        auto build = entry.loader;
        auto children = add(build(), id, entry.item.depth + 1, entry.item.enabled);
        entry.children = std::move(children);
        entry.loaded = true;
        entry.item.child_count = entry.children.size();
        entry.item.expandable = entry.explicit_expandable || !entry.children.empty();
        // An explicit expandable flag may represent a branch awaiting model data.
        // Keep it until that application-owned snapshot is replaced.
    }
};
TreeModel::TreeModel() = default;
TreeModel::TreeModel(std::vector<TreeNode> nodes) : impl_(std::make_shared<Impl>()) {
    impl_->roots = impl_->add(std::move(nodes));
}
std::size_t TreeModel::known_count() const { return impl_ ? impl_->nodes.size() : 0; }
std::optional<TreeItem> TreeModel::item(const std::string& id) const {
    if (!impl_) return {};
    const auto it = impl_->nodes.find(id);
    return it == impl_->nodes.end() ? std::optional<TreeItem>{} : it->second.item;
}
std::vector<TreeItem> TreeModel::flatten(std::span<const std::string> expanded) const {
    if (!impl_) return {};
    const std::unordered_set<std::string> open(expanded.begin(), expanded.end());
    std::vector<TreeItem> result;
    std::vector<std::string> pending(impl_->roots.rbegin(), impl_->roots.rend());
    while (!pending.empty()) {
        auto id = std::move(pending.back()); pending.pop_back();
        auto value = impl_->nodes.at(id).item;
        value.expanded = value.expandable && open.contains(id);
        if (value.expanded) {
            impl_->load(id);
            value = impl_->nodes.at(id).item;
            value.expanded = value.expandable;
        }
        result.push_back(std::move(value));
        if (result.back().expanded) {
            const auto& children = impl_->nodes.at(id).children;
            pending.insert(pending.end(), children.rbegin(), children.rend());
        }
    }
    return result;
}
} // namespace scars
