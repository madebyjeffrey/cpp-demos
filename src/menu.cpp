#include <skia_window/skia_window.hpp>

namespace skia_window {
MenuItem MenuItem::action(Command command, std::string label, Shortcut shortcut) {
    MenuItem item;
    item.command = command;
    item.label = std::move(label);
    item.shortcut = shortcut;
    return item;
}
MenuItem MenuItem::submenu(std::string label, std::vector<MenuItem> children) {
    MenuItem item;
    item.label = std::move(label);
    item.children = std::move(children);
    return item;
}
MenuItem MenuItem::divider() {
    MenuItem item;
    item.separator = true;
    return item;
}
} // namespace skia_window
