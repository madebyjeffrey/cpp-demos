#include <skia_window/skia_window.hpp>
#include <skia_window/ui.hpp>
#include <array>
#include <type_traits>

static_assert(!std::is_copy_constructible_v<skia_window::Application>);
static_assert(!std::is_copy_constructible_v<skia_window::Window>);
static_assert(!std::is_copy_constructible_v<skia_window::Dialog>);
template <class T>
concept HasMenu = requires(T& value) { value.set_menu({}, {}); };
static_assert(HasMenu<skia_window::Window>);
static_assert(!HasMenu<skia_window::Dialog>);
int main() {
    using namespace skia_window;
    std::array entries{MenuItem::action(1, "Example", {Key::e}), MenuItem::divider()};
    auto menu = MenuItem::submenu("File", {entries.begin(), entries.end()});
    if (menu.children.size() != 2 || menu.children[0].command != 1)
        return 1;
    namespace ui = skia_window::ui;
    static_assert(!std::is_convertible_v<ui::Fraction, ui::Dimension>);
    ui::State<int> count{1};
    ui::Host host([&] {
        return ui::Column{ui::Text(std::to_string(count.get())).key("count"),
                          ui::Button("Increment", [&] { count.set(count.get() + 1); }).key("increment")};
    });
    host.layout({200, 100});
    for (const auto& element : host.elements())
        if (element.key == "increment")
            host.activate(element.id);
    host.layout({200, 100});
    for (const auto& element : host.elements())
        if (element.key == "count")
            return element.label == "2" ? 0 : 1;
    return 1;
}
