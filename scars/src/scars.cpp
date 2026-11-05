module;
#include <algorithm>
#include <cctype>
#include <chrono>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <functional>
#include <limits>
#include <memory>
#include <numeric>
#include <optional>
#include <stdexcept>
#include <string>
#include <thread>
#include <unordered_map>
#include <unordered_set>
#include <utility>
#include <vector>
#include "include/core/SkCanvas.h"
#include "include/core/SkPaint.h"
#include "include/core/SkRRect.h"
module scars;

namespace scars {
namespace detail {
thread_local std::shared_ptr<Observer> collecting;
thread_local bool building{};
void CellBase::read() {
    if (owner != std::this_thread::get_id())
        throw std::logic_error("SCARS state accessed from another thread");
    if (!collecting)
        return;
    std::erase_if(observers, [](auto& item) { return item.first.expired(); });
    for (auto& [weak, epoch] : observers)
        if (weak.lock() == collecting) {
            epoch = collecting->epoch;
            return;
        }
    observers.emplace_back(collecting, collecting->epoch);
}
void CellBase::write() {
    if (owner != std::this_thread::get_id())
        throw std::logic_error("SCARS state changed from another thread");
    if (building)
        throw std::logic_error("SCARS state cannot change while building a view");
}
void CellBase::notify() {
    auto snapshot = observers;
    for (auto& [weak, epoch] : snapshot)
        if (auto o = weak.lock(); o && epoch == o->epoch)
            o->changed();
}
enum class Kind {
    empty,
    label,
    button,
    toggle,
    checkbox,
    radio,
    radio_option,
    segments,
    segment_option,
    menu,
    selection,
    split,
    split_arrow,
    slider,
    split_view,
    split_divider,
    tab_view,
    tab_bar,
    tab_option,
    tab_page,
    text_field,
    field_action,
    stepper,
    step_button,
    switch_control,
    listbox,
    list_option,
    table_header,
    table_column,
    table_resize,
    table_cell,
    image,
    fragment,
    progress,
    row,
    column,
    block,
    flow,
    layer,
    section,
    group,
    panel,
    separator,
    scroll,
    spacer
};
struct Description {
    Kind kind{Kind::empty};
    std::string key, text, accessible;
    std::vector<View> children;
    std::vector<Option> options;
    std::vector<Action> actions;
    std::vector<Tab> tabs;
    std::function<void()> click;
    struct AlertConfig {
        std::function<bool(bool)> respond;
        std::function<bool()> pending;
        std::function<View()> body;
    };
    std::optional<AlertConfig> alert;
    std::optional<AlertActionRole> alert_action;

    std::function<double()> read_number;
    std::function<void(double)> write_number;
    std::function<std::string()> read_selection;
    std::function<void(std::string)> write_selection;
    std::optional<Binding<bool>> expansion;
    struct Tooltip { std::string text; TooltipOptions options; };
    struct Popover { Binding<bool> presented; std::function<View()> build; PopoverOptions options; };
    std::optional<Tooltip> tooltip;
    std::optional<Popover> popover;
    std::function<std::vector<MenuItem>()> context_menu;
    SliderOptions range;
    SplitViewOptions split_options;
    TextFieldOptions text_options;
    std::optional<SearchFieldOptions> search;
    std::optional<PasswordFieldOptions> password;
    std::optional<ComboBoxOptions> combo;
    bool combo_action{}, calendar_action{};
    std::optional<detail::NumericConfig> numeric;
    std::optional<DatePickerOptions> date;
    std::optional<TimePickerOptions> time;
    std::string semantic_type;
    struct DateTime {
        Binding<LocalDateTime> value;
        DateTimePickerOptions options;
        std::string context;
        std::function<std::string()> source_token;
    };
    std::optional<DateTime> datetime;
    struct ColorConfig { Binding<Color> value; ColorPickerOptions options; bool well{}; };
    std::optional<ColorConfig> color;
    std::optional<Color> sample;
    bool color_component{};
    std::function<detail::NumericValue()> read_numeric;
    std::function<void(detail::NumericValue)> write_numeric;
    ListBoxOptions list_options;
    RowModel rows;
    struct Tree { TreeModel model; Binding<std::vector<std::string>> expanded; TreeViewOptions options; };
    std::optional<Tree> tree;
    struct Table { TableModel model; Binding<TableSelection> selection; TableViewOptions options; };
    std::optional<Table> table;
    ScrollViewOptions scroll_options;
    Bitmap bitmap;
    ContentMode content_mode{ContentMode::fit};
    std::function<std::vector<std::string>()> read_selected;
    std::function<void(std::vector<std::string>)> write_selected;
    std::function<std::string()> read_text;
    std::function<void(std::string)> write_text;
    Length width{automatic}, height{automatic}, basis{automatic};
    double min_width{}, min_height{}, max_width{infinity}, max_height{infinity};
    Insets padding{}, margin{};
    double gap{}, grow{}, shrink{1};
    Align align{Align::stretch};
    std::optional<Align> self;
    Justify justify{Justify::start};
    std::optional<Color> foreground, background, accent;
    std::optional<double> font_size, radius;
    bool wrap{}, bold{}, uppercase{}, enabled{true}, visible{true}, indeterminate{};
    LabelStyle style{LabelStyle::standard};
    TextWrap text_wrap{TextWrap::no_wrap};
    TextOverflow overflow{TextOverflow::clip};
    int max_lines{};
    Align text_align{Align::start};
    std::optional<Align> control_text_align;
    Density density{Density::standard};
    Orientation orientation{Orientation::horizontal};
    Position position{Position::normal};
    Length left{automatic}, top{automatic}, right{automatic}, bottom{automatic};
    int z_index{};
    bool hidden{};
    bool reveal_action{};
};
auto make(Kind kind) {
    auto p = std::make_shared<Description>();
    p->kind = kind;
    return p;
}
void nonnegative(double n, const char* name) {
    if (!std::isfinite(n) || n < 0)
        throw std::invalid_argument(std::string(name) + " must be finite and nonnegative");
}
void check_length(Length n) {
    if (n.unit != Length::Unit::automatic)
        nonnegative(n.value, "Length");
}
void check_offset(Length n) {
    if (n.unit != Length::Unit::automatic && !std::isfinite(n.value))
        throw std::invalid_argument("Position offsets must be finite");
}
void check_insets(Insets n) {
    for (double v : {n.top, n.right, n.bottom, n.left})
        nonnegative(v, "Insets");
}
void check_options(const std::vector<Option>& options) {
    std::unordered_set<std::string> ids;
    for (auto& o : options)
        if (o.id.empty() || !ids.insert(o.id).second)
            throw std::invalid_argument("Option IDs must be nonempty and unique");
}
std::vector<Option> action_options(const std::vector<Action>& actions) {
    std::vector<Option> result;
    for (auto& a : actions)
        result.push_back({a.id, a.label, a.enabled});
    check_options(result);
    return result;
}
} // namespace detail
using detail::Description;
using detail::Kind;
Theme Theme::light() {
    Theme result;
    result.window = 0xfff1f4f7;
    result.panel = 0xffe2e9ef;
    result.group = 0xffe8edf2;
    result.inactive = 0xffcbd5df;
    result.text = 0xff142738;
    result.secondary = 0xff4a6175;
    result.disabled = 0xff7a8895;
    result.border = 0xff9bafbf;
    result.primary = 0xffffaa34;
    result.blue = 0xff1479bb;
    result.cyan = 0xff087da0;
    result.error = 0xffbb2639;
    result.field = 0xffffffff;
    result.selection = 0xffb8dcf1;
    result.caret = 0xff006f98;
    result.light_mode = true;
    return result;
}
View::View() : description_(detail::make(Kind::empty)) {}
View::View(std::shared_ptr<const Description> p) : description_(std::move(p)) {}
const Description& View::description() const {
    return *description_;
}
View View::modify(const std::function<void(Description&)>& fn) const {
    auto p = std::make_shared<Description>(*description_);
    fn(*p);
    return View(p);
}
View View::key(std::string n) const {
    return modify([&](auto& d) { d.key = std::move(n); });
}
#define SCARS_LENGTH(name)                                                                                   \
    View View::name(Length n) const {                                                                        \
        detail::check_length(n);                                                                             \
        return modify([&](auto& d) { d.name = n; });                                                         \
    }
SCARS_LENGTH(width)
SCARS_LENGTH(height)
SCARS_LENGTH(basis)
#undef SCARS_LENGTH
#define SCARS_OFFSET(name)                                                                                   \
    View View::name(Length n) const {                                                                        \
        detail::check_offset(n);                                                                             \
        return modify([&](auto& d) { d.name = n; });                                                         \
    }
SCARS_OFFSET(left)
SCARS_OFFSET(top)
SCARS_OFFSET(right)
SCARS_OFFSET(bottom)
#undef SCARS_OFFSET
#define SCARS_POSITIVE(name)                                                                                 \
    View View::name(double n) const {                                                                        \
        detail::nonnegative(n, #name);                                                                       \
        return modify([&](auto& d) { d.name = n; });                                                         \
    }
SCARS_POSITIVE(min_width)
SCARS_POSITIVE(min_height)
SCARS_POSITIVE(gap)
SCARS_POSITIVE(grow)
SCARS_POSITIVE(shrink)
#undef SCARS_POSITIVE
View View::max_width(double n) const {
    if (n != infinity)
        detail::nonnegative(n, "max_width");
    return modify([&](auto& d) { d.max_width = n; });
}
View View::max_height(double n) const {
    if (n != infinity)
        detail::nonnegative(n, "max_height");
    return modify([&](auto& d) { d.max_height = n; });
}
View View::padding(double n) const {
    return padding(Insets::all(n));
}
View View::padding(Insets n) const {
    detail::check_insets(n);
    return modify([&](auto& d) { d.padding = n; });
}
View View::margin(double n) const {
    return margin(Insets::all(n));
}
View View::margin(Insets n) const {
    detail::check_insets(n);
    return modify([&](auto& d) { d.margin = n; });
}
#define SCARS_PROPERTY(name, field, type)                                                                    \
    View View::name(type n) const {                                                                          \
        return modify([&](auto& d) { d.field = n; });                                                        \
    }
SCARS_PROPERTY(wrap, wrap, bool)
SCARS_PROPERTY(align_items, align, Align)
SCARS_PROPERTY(align_self, self, Align)
SCARS_PROPERTY(justify, justify, Justify)
SCARS_PROPERTY(foreground, foreground, Color)
SCARS_PROPERTY(background, background, Color)
SCARS_PROPERTY(accent, accent, Color)
SCARS_PROPERTY(bold, bold, bool)
SCARS_PROPERTY(text_wrap, text_wrap, TextWrap)
SCARS_PROPERTY(text_overflow, overflow, TextOverflow)
SCARS_PROPERTY(uppercase, uppercase, bool)
SCARS_PROPERTY(density, density, Density)
SCARS_PROPERTY(enabled, enabled, bool)
SCARS_PROPERTY(visible, visible, bool)
SCARS_PROPERTY(hidden, hidden, bool)
SCARS_PROPERTY(position, position, Position)
SCARS_PROPERTY(z_index, z_index, int)
SCARS_PROPERTY(content_mode, content_mode, ContentMode)
#undef SCARS_PROPERTY
View View::text_align(Align n) const {
    return modify([&](auto& d) {
        d.text_align = n;
        d.control_text_align = n;
    });
}
View View::font_size(double n) const {
    detail::nonnegative(n, "font_size");
    if (n == 0)
        throw std::invalid_argument("font_size must be positive");
    return modify([&](auto& d) { d.font_size = n; });
}
View View::corner_radius(double n) const {
    detail::nonnegative(n, "radius");
    return modify([&](auto& d) { d.radius = n; });
}
View View::max_lines(int n) const {
    if (n < 0)
        throw std::invalid_argument("max_lines must be nonnegative");
    return modify([&](auto& d) { d.max_lines = n; });
}
View View::label_style(LabelStyle n) const {
    return modify([&](auto& d) {
        d.style = n;
        if (n == LabelStyle::micro) {
            d.uppercase = true;
            d.max_lines = 1;
            d.overflow = TextOverflow::ellipsis;
        }
    });
}
View View::accessible_label(std::string n) const {
    return modify([&](auto& d) { d.accessible = std::move(n); });
}
View View::expanded(Binding<bool> n) const {
    return modify([&](auto& d) { d.expansion = n; });
}
View View::tooltip(std::string text, TooltipOptions options) const {
    detail::nonnegative(options.max_width, "tooltip max_width");
    detail::nonnegative(options.gap, "tooltip gap");
    detail::nonnegative(options.margin, "tooltip margin");
    if (options.max_width == 0 || options.delay.count() < 0 || options.delay > std::chrono::hours(24))
        throw std::invalid_argument("Tooltip requires positive width and a delay between zero and 24 hours");
    return modify([&](auto& d) {
        d.tooltip.reset();
        if (!text.empty()) d.tooltip = Description::Tooltip{std::move(text), options};
    });
}
View View::popover(Binding<bool> presented, std::function<View()> build, PopoverOptions options) const {
    detail::nonnegative(options.width, "popover width");
    detail::nonnegative(options.max_height, "popover max_height");
    detail::nonnegative(options.gap, "popover gap");
    detail::nonnegative(options.margin, "popover margin");
    if (!build || options.width == 0 || options.max_height == 0)
        throw std::invalid_argument("Popover requires a content builder and positive width and height");
    return modify([&](auto& d) { d.popover = Description::Popover{presented, std::move(build), options}; });
}
MenuItem MenuItem::separator(std::string id) {
    MenuItem item;
    item.id = std::move(id); item.kind = MenuItemKind::separator; item.enabled = false;
    return item;
}
MenuItem MenuItem::check(std::string id, std::string label, Binding<bool> selected, std::string shortcut) {
    MenuItem item;
    item.id = std::move(id); item.label = std::move(label);
    item.invoke = [selected] { selected.set(!selected.get()); };
    item.kind = MenuItemKind::check;
    item.shortcut = std::move(shortcut);
    item.is_checked = [selected] { return selected.get(); };
    return item;
}
MenuItem MenuItem::radio(std::string id, std::string label, Binding<std::string> selected, std::string value) {
    MenuItem item;
    item.id = std::move(id); item.label = std::move(label);
    item.invoke = [selected, value] { selected.set(value); };
    item.kind = MenuItemKind::radio;
    item.is_checked = [selected, value] { return selected.get() == value; };
    return item;
}
MenuItem MenuItem::submenu(std::string id, std::string label, std::vector<MenuItem> children) {
    MenuItem item;
    item.id = std::move(id); item.label = std::move(label);
    item.kind = MenuItemKind::submenu; item.children = std::move(children);
    return item;
}
View View::context_menu(std::vector<MenuItem> items) const {
    return context_menu([items = std::move(items)] { return items; });
}
View View::context_menu(std::function<std::vector<MenuItem>()> items) const {
    return modify([&](auto& d) { d.context_menu = std::move(items); });
}
Label::Label(std::string text, LabelStyle style) : View(detail::make(Kind::label)) {
    auto d = std::make_shared<Description>(description());
    d->text = std::move(text);
    description_ = std::move(d);
    *static_cast<View*>(this) = label_style(style);
}
Button::Button(std::string text, std::function<void()> fn) : View(detail::make(Kind::button)) {
    auto d = std::make_shared<Description>(description());
    d->text = std::move(text);
    d->click = std::move(fn);
    description_ = std::move(d);
}
ToggleButton::ToggleButton(std::string text, Binding<bool> b) : View(detail::make(Kind::toggle)) {
    auto d = std::make_shared<Description>(description());
    d->text = std::move(text);
    d->read_number = [b] { return b.get() ? 1. : 0.; };
    d->write_number = [b](double n) { b.set(n != 0); };
    description_ = std::move(d);
}
CheckBox::CheckBox(std::string text, Binding<bool> b)
    : CheckBox(std::move(text),
               Binding<CheckState>([b] { return b.get() ? CheckState::checked : CheckState::unchecked; },
                                   [b](CheckState s) { b.set(s == CheckState::checked); })) {}
CheckBox::CheckBox(std::string text, Binding<CheckState> b) : View(detail::make(Kind::checkbox)) {
    auto d = std::make_shared<Description>(description());
    d->text = std::move(text);
    d->read_number = [b] { return static_cast<double>(b.get()); };
    d->write_number = [b](double n) { b.set(static_cast<CheckState>(static_cast<int>(n))); };
    description_ = std::move(d);
}
#define SCARS_CHOICE(Type, kindname)                                                                         \
    Type::Type(Binding<std::string> b, std::vector<Option> options) : View(detail::make(Kind::kindname)) {   \
        detail::check_options(options);                                                                      \
        auto d = std::make_shared<Description>(description());                                               \
        d->options = std::move(options);                                                                     \
        d->read_selection = [b] { return b.get(); };                                                         \
        d->write_selection = [b](std::string n) { b.set(std::move(n)); };                                    \
        description_ = std::move(d);                                                                         \
    }
SCARS_CHOICE(SegmentedControl, segments)
SCARS_CHOICE(RadioGroup, radio)
SCARS_CHOICE(SelectionDropdown, selection)
#undef SCARS_CHOICE
MenuButton::MenuButton(std::string text, std::vector<Action> actions) : View(detail::make(Kind::menu)) {
    auto d = std::make_shared<Description>(description());
    d->text = std::move(text);
    d->options = detail::action_options(actions);
    d->actions = std::move(actions);
    description_ = std::move(d);
}
SplitActionButton::SplitActionButton(Binding<std::string> b, std::vector<Action> actions)
    : View(detail::make(Kind::split)) {
    auto d = std::make_shared<Description>(description());
    d->options = detail::action_options(actions);
    d->actions = std::move(actions);
    d->read_selection = [b] { return b.get(); };
    d->write_selection = [b](std::string n) { b.set(std::move(n)); };
    description_ = std::move(d);
}
Slider::Slider(Binding<double> b, SliderOptions range) : View(detail::make(Kind::slider)) {
    if (!std::isfinite(range.min) || !std::isfinite(range.max) || range.min >= range.max ||
        !std::isfinite(range.step) || range.step <= 0 || !std::isfinite(range.page_step) ||
        range.page_step <= 0)
        throw std::invalid_argument("Invalid slider range or steps");
    auto d = std::make_shared<Description>(description());
    d->read_number = [b] { return b.get(); };
    d->write_number = [b](double n) { b.set(n); };
    d->range = range;
    description_ = std::move(d);
}
TabView::TabView(Binding<std::string> selection, std::vector<Tab> tabs) : View(detail::make(Kind::tab_view)) {
    auto d = std::make_shared<Description>(description());
    for (const auto& tab : tabs) {
        if (!tab.content)
            throw std::invalid_argument("TabView requires a content builder for each tab");
        d->options.push_back({tab.id, tab.label, tab.enabled});
    }
    detail::check_options(d->options);
    d->tabs = std::move(tabs);
    d->read_selection = [selection] { return selection.get(); };
    d->write_selection = [selection](std::string value) { selection.set(std::move(value)); };
    d->gap = 8;
    description_ = std::move(d);
}
SplitView::SplitView(View first, View second, Binding<double> fraction, SplitViewOptions options)
    : View(detail::make(Kind::split_view)) {
    detail::nonnegative(options.first_min, "First pane minimum");
    detail::nonnegative(options.second_min, "Second pane minimum");
    if (!std::isfinite(options.first_min + options.second_min) ||
        !std::isfinite(options.divider_width) || options.divider_width <= 0 ||
        !std::isfinite(options.keyboard_step) || options.keyboard_step <= 0)
        throw std::invalid_argument("SplitView divider width and keyboard step must be finite and positive");
    auto d = std::make_shared<Description>(*description_);
    d->split_options = std::move(options);
    d->orientation = d->split_options.orientation;
    d->read_number = [fraction] { return fraction.get(); };
    d->write_number = [fraction](double value) { fraction.set(value); };
    // Stable pane containers retain state even when the supplied content is empty or conditional.
    d->children = {Layer{std::move(first)}.key("first"), Layer{std::move(second)}.key("second")};
    description_ = std::move(d);
}
Image::Image(Bitmap bitmap, ContentMode mode) : View(detail::make(Kind::image)) {
    auto d = std::make_shared<Description>(description());
    d->bitmap = std::move(bitmap);
    d->content_mode = mode;
    description_ = std::move(d);
}
ForEach::ForEach(std::vector<View> children) : View(detail::make(Kind::fragment)) {
    std::unordered_set<std::string> keys;
    for (const auto& view : children) {
        const auto key = view.description().key;
        if (key.empty() || !keys.insert(key).second)
            throw std::invalid_argument("ForEach requires nonempty unique keys");
    }
    auto d = std::make_shared<Description>(description());
    d->text = "ForEach";
    d->children = std::move(children);
    description_ = std::move(d);
}
If::If(bool condition, View yes, View no) : View(detail::make(Kind::fragment)) {
    auto d = std::make_shared<Description>(description());
    d->text = "If";
    d->children = {condition ? yes.key("then") : no.key("else")};
    description_ = std::move(d);
}
If::If(bool condition, std::function<View()> yes, std::function<View()> no)
    : If(condition, condition && yes ? yes() : View{}, !condition && no ? no() : View{}) {}
ListBox::ListBox(Binding<std::string> selection, std::vector<Option> options, ListBoxOptions settings)
    : ListBox(std::move(selection), RowModel(std::move(options)), std::move(settings)) {}
ListBox::ListBox(Binding<std::string> selection, std::initializer_list<Option> options, ListBoxOptions settings)
    : ListBox(std::move(selection), std::vector<Option>(options), std::move(settings)) {}
ListBox::ListBox(Binding<std::string> selection, RowModel rows, ListBoxOptions settings)
    : View(detail::make(Kind::listbox)) {
    detail::nonnegative(settings.row_height, "row height");
    if (!settings.visible_rows || settings.row_height < 12)
        throw std::invalid_argument("ListBox requires positive visible rows and row height >= 12");
    auto d = std::make_shared<Description>(description());
    d->rows = std::move(rows);
    d->list_options = std::move(settings);
    d->read_selection = [selection] { return selection.get(); };
    d->write_selection = [selection](std::string value) { selection.set(std::move(value)); };
    d->padding = Insets::all(4);
    description_ = std::move(d);
}
ListBox::ListBox(Binding<std::vector<std::string>> selection, std::vector<Option> options,
                 ListBoxOptions settings)
    : ListBox(std::move(selection), RowModel(std::move(options)), std::move(settings)) {}
ListBox::ListBox(Binding<std::vector<std::string>> selection, std::initializer_list<Option> options, ListBoxOptions settings)
    : ListBox(std::move(selection), std::vector<Option>(options), std::move(settings)) {}
ListBox::ListBox(Binding<std::vector<std::string>> selection, RowModel rows, ListBoxOptions settings)
    : View(detail::make(Kind::listbox)) {
    detail::nonnegative(settings.row_height, "row height");
    if (!settings.visible_rows || settings.row_height < 12)
        throw std::invalid_argument("ListBox requires positive visible rows and row height >= 12");
    auto d = std::make_shared<Description>(description());
    d->rows = std::move(rows);
    d->list_options = std::move(settings);
    d->read_selected = [selection] { return selection.get(); };
    d->write_selected = [selection](std::vector<std::string> value) { selection.set(std::move(value)); };
    d->padding = Insets::all(4);
    description_ = std::move(d);
}
TreeView::TreeView(Binding<std::string> selection, TreeModel model, Binding<std::vector<std::string>> expanded,
                   TreeViewOptions options)
    : ListBox(std::move(selection), RowModel{}, {options.row_height, options.visible_rows, options.on_activate, options.overscan_rows}) {
    detail::nonnegative(options.indentation, "tree indentation");
    auto d = std::make_shared<Description>(description());
    d->tree = Description::Tree{std::move(model), std::move(expanded), std::move(options)};
    description_ = std::move(d);
}
TreeView::TreeView(Binding<std::vector<std::string>> selection, TreeModel model, Binding<std::vector<std::string>> expanded,
                   TreeViewOptions options)
    : ListBox(std::move(selection), RowModel{}, {options.row_height, options.visible_rows, options.on_activate, options.overscan_rows}) {
    detail::nonnegative(options.indentation, "tree indentation");
    auto d = std::make_shared<Description>(description());
    d->tree = Description::Tree{std::move(model), std::move(expanded), std::move(options)};
    description_ = std::move(d);
}
TableView::TableView(Binding<TableSelection> selection, TableModel model, TableViewOptions options)
    : ListBox(Binding<std::vector<std::string>>{[selection] { return selection.get().rows; },
          [selection](std::vector<std::string> rows) { selection.set(TableSelection{std::move(rows), {}}); }},
          model.rows(), {options.row_height, options.visible_rows, {}, options.overscan_rows}) {
    detail::nonnegative(options.header_height, "table header height");
    if (options.header_height < 24) throw std::invalid_argument("Table header height must be >= 24");
    auto d = std::make_shared<Description>(description());
    d->table = Description::Table{std::move(model), selection, options};
    if (!options.multiple_selection || options.selection_mode == TableSelectionMode::cells) {
        d->read_selected = {}; d->write_selected = {};
        d->read_selection = [selection, mode = options.selection_mode] {
            const auto value = selection.get();
            return mode == TableSelectionMode::cells ? value.cells.empty() ? std::string{} : value.cells.front().row :
                value.rows.empty() ? std::string{} : value.rows.front();
        };
        d->write_selection = [selection](std::string row) { selection.set(TableSelection{{std::move(row)}, {}}); };
    }
    description_ = std::move(d);
}
TextField::TextField(std::string placeholder, Binding<std::string> value, TextFieldOptions options)
    : View(detail::make(Kind::text_field)) {
    auto d = std::make_shared<Description>(description());
    d->text = std::move(placeholder);
    d->text_options = std::move(options);
    d->read_text = [value] { return value.get(); };
    d->write_text = [value](std::string text) { value.set(std::move(text)); };
    d->padding = Insets::symmetric(8, 12);
    description_ = std::move(d);
}
SearchField::SearchField(std::string placeholder, Binding<std::string> value, SearchFieldOptions options)
    : View(detail::make(Kind::text_field)) {
    if (options.debounce.count() < 0 || options.debounce > std::chrono::hours(24))
        throw std::invalid_argument("Search debounce must be between zero and one day");
    auto field = TextField(std::move(placeholder), std::move(value),
                          {.read_only = options.read_only, .max_length = options.max_length});
    auto d = std::make_shared<Description>(field.description());
    d->search = std::move(options);
    description_ = std::move(d);
}
PasswordField::PasswordField(std::string placeholder, Binding<std::string> value, PasswordFieldOptions options)
    : View(detail::make(Kind::text_field)) {
    auto field = TextField(std::move(placeholder), std::move(value),
        {.read_only = options.read_only, .max_length = options.max_length, .on_submit = options.on_submit});
    auto d = std::make_shared<Description>(field.description());
    d->password = std::move(options);
    description_ = std::move(d);
}
ComboBox::ComboBox(std::string placeholder, Binding<std::string> query, Binding<std::string> selection,
                   std::vector<Option> items, ComboBoxOptions options) : View(detail::make(Kind::text_field)) {
    detail::nonnegative(options.max_height, "combo max_height");
    detail::nonnegative(options.row_height, "combo row_height");
    if (options.max_height == 0 || options.row_height < 12)
        throw std::invalid_argument("ComboBox requires positive popup height and row height >= 12");
    std::unordered_set<std::string> ids;
    for (const auto& item : items) {
        if (item.id.empty() || !ids.insert(item.id).second)
            throw std::invalid_argument("ComboBox option IDs must be nonempty and unique");
        utf16_from_utf8(item.label);
    }
    auto field = TextField(std::move(placeholder), query, {.read_only = options.read_only, .max_length = options.max_length});
    auto d = std::make_shared<Description>(field.description());
    d->read_selection = [selection] { return selection.get(); };
    d->write_selection = [selection](std::string id) { selection.set(std::move(id)); };
    d->options = std::move(items);
    d->combo = std::move(options);
    description_ = std::move(d);
}
namespace {
template<class T> auto numeric_description(Kind kind, Binding<T> value, NumericOptions<T> options) {
    auto d = detail::make(kind);
    d->numeric = detail::NumericConfig{options.min, options.max, options.step, std::move(options.locale),
        options.significant_digits, options.grouping, options.commit_on_blur, options.read_only,
        std::move(options.on_validation), std::move(options.on_submit), options.repeat_delay, options.repeat_interval, {}, {}, {}, {}, {}};
    detail::check_numeric(*d->numeric);
    d->read_numeric = [value]() -> detail::NumericValue { return value.get(); };
    d->write_numeric = [value](detail::NumericValue number) { value.set(std::get<T>(number)); };
    d->text_options.read_only = options.read_only;
    d->padding = kind == Kind::text_field ? Insets::symmetric(8, 12) : Insets{};
    return d;
}
} // namespace
NumericField::NumericField(std::string placeholder, Binding<double> value, NumericOptions<double> options)
    : View(detail::make(Kind::text_field)) {
    auto d = numeric_description(Kind::text_field, std::move(value), std::move(options));
    d->text = std::move(placeholder);
    description_ = std::move(d);
}
NumericField::NumericField(std::string placeholder, Binding<std::int64_t> value, NumericOptions<std::int64_t> options)
    : View(detail::make(Kind::text_field)) {
    auto d = numeric_description(Kind::text_field, std::move(value), std::move(options));
    d->text = std::move(placeholder);
    description_ = std::move(d);
}
DatePicker::DatePicker(std::string label, Binding<Date> value, DatePickerOptions options)
    : View(detail::make(Kind::text_field)) {
    auto d = detail::make(Kind::text_field);
    d->text = label;
    d->accessible = std::move(label);
    d->numeric = detail::date_codec(options);
    d->date = std::move(options);
    d->read_numeric = [value]() -> detail::NumericValue { return detail::date_days(value.get()); };
    d->write_numeric = [value](detail::NumericValue n) { value.set(detail::date_from_days(std::get<0>(n))); };
    d->text_options.read_only = d->numeric->read_only;
    d->padding = Insets::symmetric(8, 12);
    d->semantic_type = "DatePicker";
    description_ = std::move(d);
}
TimePicker::TimePicker(std::string label, Binding<Time> value, TimePickerOptions options)
    : View(detail::make(Kind::text_field)) {
    auto d = detail::make(Kind::text_field);
    d->text = label;
    d->accessible = std::move(label);
    d->numeric = detail::time_codec(options);
    d->time = std::move(options);
    d->read_numeric = [value]() -> detail::NumericValue { return detail::time_seconds(value.get()); };
    d->write_numeric = [value](detail::NumericValue n) {
        value.set(detail::time_from_seconds(std::get<0>(n)));
    };
    d->text_options.read_only = d->numeric->read_only;
    d->padding = Insets::symmetric(8, 12);
    d->semantic_type = "TimePicker";
    description_ = std::move(d);
}
DateTimePicker::DateTimePicker(std::string label, Binding<LocalDateTime> value, DateTimePickerOptions options)
    : View(detail::make(Kind::group)) {
    if (!valid(options.min.date) || !valid(options.max.date) || !valid(options.min.time) ||
        !valid(options.max.time) || options.min > options.max)
        throw std::invalid_argument("Date/time bounds must be valid and ordered");
    options.date.min = std::max(options.date.min, options.min.date);
    options.date.max = std::min(options.date.max, options.max.date);
    detail::date_codec(options.date);
    detail::time_codec(options.time);
    const auto current = value.get();
    if (!valid(current.date) || !valid(current.time))
        throw std::invalid_argument("Invalid local date/time value");
    auto body = GroupBox(std::move(label), View{});
    auto d = std::make_shared<Description>(body.description());
    d->semantic_type = "DateTimePicker";
    d->datetime = Description::DateTime{std::move(value), std::move(options), "local", {}};
    description_ = std::move(d);
}
DateTimePicker::DateTimePicker(std::string label, Binding<std::chrono::sys_seconds> value,
                               ZonedDateTimePickerOptions options)
    : View(detail::make(Kind::group)) {
    if (options.min && options.max && *options.min > *options.max)
        throw std::invalid_argument("Instant bounds must be ordered");
    local_datetime(value.get(), options.time_zone); // Reject unavailable zones at construction.
    const auto resolve = [value, options](LocalDateTime local) {
        const auto current = value.get();
        if (local_datetime(current, options.time_zone) == local)
            return DateTimeResolution{current, {}};
        return resolve_datetime(local, options.time_zone, options.repeated_time);
    };
    auto display = options.display;
    const auto validate = display.validate;
    display.validate = [options, resolve, validate](LocalDateTime local) {
        if (validate) {
            auto error = validate(local);
            if (!error.empty())
                return error;
        }
        const auto result = resolve(local);
        if (!result.instant)
            return result.error;
        if ((options.min && *result.instant < *options.min) ||
            (options.max && *result.instant > *options.max))
            return std::string("Instant is outside the allowed range.");
        return std::string{};
    };
    Binding<LocalDateTime> local{[value, options] { return local_datetime(value.get(), options.time_zone); },
                                 [value, resolve](LocalDateTime next) {
                                     if (auto result = resolve(next); result.instant)
                                         value.set(*result.instant);
                                 }};
    auto body = DateTimePicker(label + " (" + options.time_zone + ")", local, std::move(display));
    auto d = std::make_shared<Description>(body.description());
    d->semantic_type = "ZonedDateTimePicker";
    d->datetime->context =
        "zone:" + options.time_zone +
        ":min:" + (options.min ? std::to_string(options.min->time_since_epoch().count()) : "none") +
        ":max:" + (options.max ? std::to_string(options.max->time_since_epoch().count()) : "none");
    d->datetime->source_token = [value] { return std::to_string(value.get().time_since_epoch().count()); };
    description_ = std::move(d);
}
ColorPicker::ColorPicker(std::string label, Binding<Color> value, ColorPickerOptions options)
    : View(detail::make(Kind::group)) {
    auto d = std::make_shared<Description>(GroupBox(std::move(label), View{}).description());
    d->semantic_type = "ColorPicker";
    d->color = Description::ColorConfig{std::move(value), options, false};
    description_ = std::move(d);
}
ColorWell::ColorWell(std::string label, Binding<Color> value, ColorPickerOptions options)
    : View(detail::make(Kind::button)) {
    auto d = std::make_shared<Description>(description());
    d->text = std::move(label);
    d->semantic_type = "ColorWell";
    d->color = Description::ColorConfig{std::move(value), options, true};
    description_ = std::move(d);
}
Stepper::Stepper(Binding<double> value, NumericOptions<double> options)
    : View(numeric_description(Kind::stepper, std::move(value), std::move(options))) {}
Stepper::Stepper(Binding<std::int64_t> value, NumericOptions<std::int64_t> options)
    : View(numeric_description(Kind::stepper, std::move(value), std::move(options))) {}
Switch::Switch(std::string text, Binding<bool> value) : View(detail::make(Kind::switch_control)) {
    auto d = std::make_shared<Description>(description());
    d->text = std::move(text);
    d->read_number = [value] { return value.get() ? 1. : 0.; };
    d->write_number = [value](double n) { value.set(n != 0); };
    description_ = std::move(d);
}
TextEditor::TextEditor(Binding<std::string> value, std::string placeholder, TextFieldOptions options)
    : TextField(std::move(placeholder), std::move(value), [&] {
          options.multiline = true;
          return options;
      }()) {}
ProgressBar::ProgressBar(double n) : ProgressBar(Binding<double>([n] { return n; }, [](double) {})) {
    if (!std::isfinite(n))
        throw std::invalid_argument("Progress must be finite");
}
ProgressBar::ProgressBar(Binding<double> b) : View(detail::make(Kind::progress)) {
    auto d = std::make_shared<Description>(description());
    d->read_number = [b] { return b.get(); };
    description_ = std::move(d);
}
ProgressBar::ProgressBar() : View(detail::make(Kind::progress)) {
    auto d = std::make_shared<Description>(description());
    d->indeterminate = true;
    description_ = std::move(d);
}
#define SCARS_CONTAINER(Type, kindname)                                                                      \
    Type::Type(std::initializer_list<View> v) : Type(std::vector<View>(v)) {}                                \
    Type::Type(std::vector<View> v) : View(detail::make(Kind::kindname)) {                                   \
        auto d = std::make_shared<Description>(description());                                               \
        d->children = std::move(v);                                                                          \
        description_ = std::move(d);                                                                         \
    }
SCARS_CONTAINER(FlexRow, row)
SCARS_CONTAINER(FlexColumn, column)
SCARS_CONTAINER(Block, block)
SCARS_CONTAINER(Inline, flow)
SCARS_CONTAINER(Layer, layer)
#undef SCARS_CONTAINER
#define SCARS_GROUP(Type, kindname)                                                                          \
    Type::Type(std::string title, View v) : View(detail::make(Kind::kindname)) {                             \
        auto d = std::make_shared<Description>(description());                                               \
        d->text = std::move(title);                                                                          \
        d->children = {std::move(v)};                                                                        \
        d->gap = 12;                                                                                         \
        if (Kind::kindname == Kind::group)                                                                   \
            d->padding = Insets::all(16);                                                                    \
        description_ = std::move(d);                                                                         \
    }
SCARS_GROUP(Section, section)
SCARS_GROUP(GroupBox, group)
#undef SCARS_GROUP
Panel::Panel(View v) : View(detail::make(Kind::panel)) {
    auto d = std::make_shared<Description>(description());
    d->children = {std::move(v)};
    d->padding = Insets::all(16);
    description_ = std::move(d);
}
struct Alert::Impl {
    std::vector<AlertAction> actions;
    std::function<void(std::optional<std::string>)> completion;
    State<bool> done{false};
    std::optional<std::string> selected;
    bool finish(std::optional<std::string> result) {
        if (done.get())
            return false;
        if (detail::building)
            throw std::logic_error("Cannot complete an alert while building views");
        selected = std::move(result);
        done.set(true);
        auto callback = std::exchange(completion, {});
        if (callback)
            callback(selected);
        return true;
    }
    bool choose(const std::string& id) {
        const auto found =
            std::find_if(actions.begin(), actions.end(), [&](const auto& action) { return action.id == id; });
        return found != actions.end() && found->enabled && finish(id);
    }
    bool respond(bool cancel) {
        const auto role = cancel ? AlertActionRole::cancel : AlertActionRole::default_action;
        const auto found = std::find_if(actions.begin(), actions.end(),
                                        [&](const auto& action) { return action.role == role; });
        return found != actions.end() && choose(found->id);
    }
};
Alert::Alert(std::string title, View content, std::vector<AlertAction> actions,
             std::function<void(std::optional<std::string>)> completion)
    : Alert(
          std::move(title), [content = std::move(content)] { return content; }, std::move(actions),
          std::move(completion)) {}
Alert::Alert(std::string title, std::function<View()> content, std::vector<AlertAction> actions,
             std::function<void(std::optional<std::string>)> completion)
    : impl_(std::make_shared<Impl>()) {
    if (title.empty() || actions.empty() || !content)
        throw std::invalid_argument("An alert requires a title, content builder, and at least one action");
    std::unordered_set<std::string> ids;
    bool has_default{}, has_cancel{};
    for (const auto& action : actions) {
        if (action.id.empty() || action.label.empty() || !ids.insert(action.id).second)
            throw std::invalid_argument("Alert action IDs must be unique and labels must not be empty");
        if (action.role == AlertActionRole::default_action && std::exchange(has_default, true))
            throw std::invalid_argument("An alert has at most one default action");
        if (action.role == AlertActionRole::cancel && std::exchange(has_cancel, true))
            throw std::invalid_argument("An alert has at most one cancel action");
    }
    impl_->actions = std::move(actions);
    impl_->completion = std::move(completion);
    std::vector<View> buttons;
    for (const auto& action : impl_->actions) {
        auto button = Button(action.label, [state = impl_, id = action.id] { state->choose(id); })
                          .key(action.id)
                          .enabled(action.enabled);
        button = button.modify([&](Description& d) { d.alert_action = action.role; });
        buttons.push_back(std::move(button));
    }
    auto panel = Panel(View{}).padding(24).accessible_label(title);
    auto d = std::make_shared<Description>(panel.description());
    d->children.clear();
    d->alert = Description::AlertConfig{
        [state = impl_](bool cancel) { return state->respond(cancel); },
        [state = impl_] { return !state->done.get(); },
        [title = std::move(title), content = std::move(content), buttons = std::move(buttons)] {
            return FlexColumn{Label(title, LabelStyle::heading).key("title").text_wrap(TextWrap::wrap),
                              content(), FlexRow(buttons).gap(10).wrap().justify(Justify::end).key("actions")}
                .gap(18);
        }};
    description_ = std::move(d);
}
bool Alert::choose(const std::string& id) const {
    return impl_->choose(id);
}
bool Alert::cancel() const {
    return impl_->respond(true);
}
bool Alert::dismiss() const {
    return impl_->finish({});
}
bool Alert::completed() const {
    return impl_->done.get();
}
std::optional<std::string> Alert::result() const {
    return impl_->selected;
}
const std::vector<AlertAction>& Alert::actions() const {
    return impl_->actions;
}
ScrollView::ScrollView(View v, ScrollViewOptions options) : View(detail::make(Kind::scroll)) {
    auto d = std::make_shared<Description>(description());
    d->children = {std::move(v)};
    d->scroll_options = options;
    description_ = std::move(d);
}
Separator::Separator(Orientation o) : View(detail::make(Kind::separator)) {
    auto d = std::make_shared<Description>(description());
    d->orientation = o;
    description_ = std::move(d);
}
Spacer::Spacer() : View(detail::make(Kind::spacer)) {
    auto d = std::make_shared<Description>(description());
    d->grow = 1;
    description_ = std::move(d);
}
View labeled(std::string annotation, View content) {
    if (content.description().accessible.empty())
        content = content.accessible_label(annotation);
    return FlexColumn{Label(std::move(annotation), LabelStyle::micro), std::move(content)}.gap(5);
}

namespace {
Rect intersect(Rect a, Rect b) {
    const double x = std::max(a.x, b.x), y = std::max(a.y, b.y);
    return {x, y, std::max(0., std::min(a.x + a.width, b.x + b.width) - x),
            std::max(0., std::min(a.y + a.height, b.y + b.height) - y)};
}
Rect inset(Rect r, Insets p) {
    return {r.x + p.left, r.y + p.top, std::max(0., r.width - p.left - p.right),
            std::max(0., r.height - p.top - p.bottom)};
}
double horizontal(Insets p) {
    return p.left + p.right;
}
double vertical(Insets p) {
    return p.top + p.bottom;
}
double resolve(Length l, double available, double fallback) {
    if (l.unit == Length::Unit::automatic)
        return fallback;
    if (l.unit == Length::Unit::percent)
        return std::isfinite(available) ? l.value * available : 0;
    return l.value;
}
Color contrast(Color color) {
    const auto luma = ((color >> 16) & 255) * .299 + ((color >> 8) & 255) * .587 + (color & 255) * .114;
    return luma >= 145 ? 0xff0f1924 : 0xfff7fbff;
}
Color brighten(Color c, double amount) {
    auto ch = [&](int shift) {
        return static_cast<Color>(std::clamp(((c >> shift) & 255) * amount, 0., 255.));
    };
    return (c & 0xff000000) | (ch(16) << 16) | (ch(8) << 8) | ch(0);
}
void box(SkCanvas& c, Rect r, Color color, double radius = 0, double stroke = 0) {
    if (r.width <= 0 || r.height <= 0)
        return;
    SkPaint p;
    p.setAntiAlias(true);
    p.setColor(color);
    if (stroke) {
        p.setStyle(SkPaint::kStroke_Style);
        p.setStrokeWidth(static_cast<float>(stroke));
        r = inset(r, Insets::all(stroke / 2));
    }
    c.drawRoundRect(SkRect::MakeXYWH(r.x, r.y, r.width, r.height), radius, radius, p);
}
void line(SkCanvas& c, double x, double y, double xx, double yy, Color color, double width = 1) {
    SkPaint p;
    p.setAntiAlias(true);
    p.setColor(color);
    p.setStrokeWidth(width);
    p.setStrokeCap(SkPaint::kRound_Cap);
    c.drawLine(x, y, xx, yy, p);
}
std::string capitalized(std::string s) {
    for (char& c : s)
        if (static_cast<unsigned char>(c) < 128)
            c = static_cast<char>(std::toupper(static_cast<unsigned char>(c)));
    return s;
}
std::size_t next_utf8(const std::string& s, std::size_t i) {
    if (i >= s.size())
        return s.size();
    ++i;
    while (i < s.size() && (static_cast<unsigned char>(s[i]) & 0xc0) == 0x80)
        ++i;
    return i;
}
std::string path_key(const std::string& key) {
    std::string result;
    for (const auto c : key) {
        if (c == '/')
            result += "%2F";
        else if (c == '%')
            result += "%25";
        else
            result += c;
    }
    return result;
}
std::optional<std::string> unpath_key(const std::string& encoded) {
    std::string result;
    for (std::size_t i = 0; i < encoded.size(); ++i) {
        if (encoded[i] != '%') result += encoded[i];
        else if (encoded.substr(i, 3) == "%2F") { result += '/'; i += 2; }
        else if (encoded.substr(i, 3) == "%25") { result += '%'; i += 2; }
        else return {};
    }
    return result;
}
struct MenuEntry {
    MenuItem item;
    bool checked{};
    std::vector<MenuEntry> children;
};
std::vector<MenuEntry> resolve_menu(std::vector<MenuItem> items) {
    std::vector<MenuEntry> result;
    std::unordered_set<std::string> ids;
    for (auto& item : items) {
        if (item.id.empty() || !ids.insert(item.id).second)
            throw std::invalid_argument("Context-menu sibling IDs must be nonempty and unique");
        if (item.kind != MenuItemKind::separator && item.label.empty())
            throw std::invalid_argument("Context-menu items require labels");
        MenuEntry entry;
        entry.checked = item.is_checked && item.is_checked();
        if (item.kind == MenuItemKind::submenu) entry.children = resolve_menu(std::move(item.children));
        else if (!item.children.empty()) throw std::invalid_argument("Only submenus can contain menu items");
        entry.item = std::move(item);
        result.push_back(std::move(entry));
    }
    return result;
}
struct Node {
    std::string datetime_owner, color_owner;
    bool datetime_component{};
    std::string calendar_owner;
    std::optional<Date> calendar_date;
    Description d;
    std::string id, group, selection, requested_selection;
    std::vector<std::string> selected_ids;
    std::vector<std::unique_ptr<Node>> children;
    std::vector<std::unique_ptr<Node>> overlay_children;
    std::string overlay_scope;
    std::string combo_owner;
    bool tooltip_root{}, popover_root{};
    std::vector<MenuEntry> context_menu;
    std::vector<Node*> paint_children;
    std::shared_ptr<const TextLayout> text;
    Point text_origin{};
    Rect bounds{}, clip{};
    Size desired{};
    Size scroll_content{};
    Point scroll_max{};
    std::size_t row_index{};
    bool custom_row{};
    std::shared_ptr<detail::Observer> row_observer;
    RowModel row_snapshot;
    std::optional<TreeItem> tree_item;
    std::string table_owner, column;
    std::vector<FlexLineInspection> flex_lines;
    double font{14}, line_height{18}, tracking{}, number{};
    std::optional<detail::NumericValue> numeric_value;
    std::string numeric_text;
    SplitPane collapsed{SplitPane::none};
    Color color{}, accent{};
    bool enabled{}, expanded{true}, layout_editable{}, hidden{};
};
std::string password_mask(const std::string& value) {
    const auto count = grapheme_boundaries(value).size() - 1;
    std::string result;
    for (std::size_t i = 0; i < count; ++i) result += "•";
    return result;
}
std::shared_ptr<const TextLayout> password_layout(std::shared_ptr<const TextLayout> displayed,
                                                std::vector<std::size_t> boundaries) {
    auto source = std::make_shared<const std::vector<std::size_t>>(std::move(boundaries));
    const auto original = [source](std::size_t masked) { return (*source)[std::min(masked / 3, source->size() - 1)]; };
    const auto masked = [source](std::size_t offset, bool end = false) {
        const auto it = end ? std::lower_bound(source->begin(), source->end(), offset)
                            : std::upper_bound(source->begin(), source->end(), offset);
        const auto index = end ? std::min<std::size_t>(it - source->begin(), source->size() - 1)
                               : (it == source->begin() ? 0 : std::size_t(it - source->begin() - 1));
        return index * 3;
    };
    auto result = std::make_shared<TextLayout>(*displayed);
    if (displayed->hit_test)
        result->hit_test = [displayed, original](Point point) { return original(displayed->hit_test(point)); };
    if (displayed->caret_rect)
        result->caret_rect = [displayed, masked](std::size_t offset) { return displayed->caret_rect(masked(offset)); };
    if (displayed->selection_rects)
        result->selection_rects = [displayed, masked](TextRange range) {
            const auto start = masked(range.start);
            const auto end = range.length ? masked(range.end(), true) : start;
            return displayed->selection_rects({start, end - start});
        };
    for (auto& line : result->lines) {
        const auto end = original(line.range.end());
        line.range.start = original(line.range.start);
        line.range.length = end - line.range.start;
    }
    for (auto& glyph : result->glyphs)
        glyph.utf8_offset = static_cast<std::uint32_t>(original(glyph.utf8_offset));
    return result;
}
bool interactive(const Node& n) {
    if (!n.enabled || n.hidden)
        return false;
    if(n.d.kind==Kind::table_column)return bool(n.d.click);
    switch (n.d.kind) {
    case Kind::button:
    case Kind::toggle:
    case Kind::checkbox:
    case Kind::radio_option:
    case Kind::segment_option:
    case Kind::menu:
    case Kind::selection:
    case Kind::split:
    case Kind::split_arrow:
    case Kind::slider:
    case Kind::split_divider:
    case Kind::tab_option:
    case Kind::text_field:
    case Kind::field_action:
    case Kind::stepper:
    case Kind::step_button:
    case Kind::switch_control:
    case Kind::listbox:
    case Kind::list_option:
    case Kind::table_resize:
    case Kind::table_cell:
        return true;
    case Kind::section:
    case Kind::group:
        return n.d.expansion.has_value();
    default:
        return false;
    }
}
bool text_kind(Kind k) {
    return k == Kind::label;
}
bool inline_text(const Node& n) {
    return text_kind(n.d.kind) && !n.d.background && horizontal(n.d.padding) == 0 &&
           vertical(n.d.padding) == 0 && horizontal(n.d.margin) == 0 && vertical(n.d.margin) == 0 &&
           n.d.width.unit == Length::Unit::automatic && n.d.height.unit == Length::Unit::automatic;
}
bool horizontal_kind(Kind k) {
    return k == Kind::row || k == Kind::segments || k == Kind::tab_bar;
}
struct FontEngine {
    std::shared_ptr<TextLayoutEngine> engine;
    explicit FontEngine(const Theme& theme, std::shared_ptr<TextLayoutEngine> custom = {})
        : engine(custom ? std::move(custom) : make_skparagraph_text_engine(theme.font_path)) {}
    TextSpan span(const Node& n, std::string text) const {
        TextSpan s;
        s.text = std::move(text);
        s.font_size = n.font;
        s.line_height = n.line_height;
        s.letter_spacing = n.tracking;
        s.bold = n.d.bold || n.d.style == LabelStyle::heading || n.d.style == LabelStyle::section;
        s.color = n.color;
        return s;
    }
    std::shared_ptr<const TextLayout> layout(const Node& n, const std::string& text,
                                             TextLayoutOptions options = {}) const {
        return engine->layout({span(n, text)}, options);
    }
    double width(const Node& n, const std::string& text) const {
        return layout(n, text, {infinity, TextWrap::no_wrap})->size.width;
    }
    double ascent(const Node& n) const {
        return layout(n, "", {infinity, TextWrap::no_wrap})->baseline;
    }
    void draw(SkCanvas& canvas, const Node& n, const std::string& text, double x, double baseline,
              Color color) const {
        auto s = span(n, text);
        s.color = color;
        auto paragraph = engine->layout({s}, {infinity, TextWrap::no_wrap});
        paragraph->paint(canvas, {x, baseline - paragraph->baseline});
    }
};
} // namespace

struct Host::Impl : std::enable_shared_from_this<Host::Impl> {
    std::function<View()> builder;
    std::function<void()> wake;
    std::function<void()> inspection_observer;
    std::string inspected, inspection_hovered;
    bool inspection_picking{}, inspection_capture{};
    bool inspection_flash_pending{};
    std::optional<double> inspection_flash_start;
    InspectionOverlay inspection_overlay{InspectionOverlay::selection};
    std::unordered_map<std::string, LayoutOverride> layout_overrides;
    struct Editing {
        TextBuffer buffer;
        Point scroll;
        std::optional<double> desired_x;
        bool reset_blink{true}, reveal_caret{true}, upstream{};
        std::optional<detail::NumericValue> numeric_source;
        std::string numeric_base, numeric_locale, numeric_format_id;
        int numeric_digits{};
        bool numeric_grouping{};
        NumericValidation validation;
        bool password_mode{}, search_mode{}, combo_mode{}, revealed{};
        std::string search_source;
        std::chrono::milliseconds search_delay{};
        struct Search {
            std::string query;
            std::chrono::steady_clock::time_point due;
            bool suspended{};
        };
        std::optional<Search> pending_search;
        std::optional<double> blink_epoch;
    };
    std::unordered_map<std::string, std::unique_ptr<Editing>> editors;
    struct DateTimeState {
        bool initialized{}, batch{}, publishing{}, rejected{};
        LocalDateTime source,draft;
        std::string token,configuration,error;
        std::uint64_t revision{};
    };
    std::unordered_map<std::string,DateTimeState> datetimes;
    struct ColorState {
        bool initialized{}, open{}, publishing{}, batch{};
        Color source{}, observed{}, draft{};
        HSV hsv;
        ColorModel model{};
        std::optional<Color> preview;
        std::optional<Description::ColorConfig> config;
        std::string configuration, error;
        std::uint64_t revision{};
    };
    std::unordered_map<std::string, ColorState> colors;
    struct ColorCleanup {
        Binding<Color> value;
        Color baseline{}, owned{};
    };
    std::vector<ColorCleanup> color_cleanup;
    struct CalendarState {
        bool initialized{}, open{};
        Date source, month, active, min, max;
        std::string locale,context;
    };
    std::unordered_map<std::string, CalendarState> calendars;
    struct ComboState {
        bool initialized{}, open{}, session{}, writing{}, reveal{true};
        std::string base_query, base_selection, source_query, source_selection, highlighted, validation;
        std::optional<std::string> selected_label;
        std::vector<Option> matches;
        std::function<std::string()> read_query, read_selection;
        std::function<void(std::string)> write_query, write_selection;
        ComboBoxMode mode{ComboBoxMode::selection};
    };
    std::unordered_map<std::string, ComboState> combos;
    struct ComboCleanup { std::function<std::string()> read; std::function<void(std::string)> write; std::string value; };
    std::vector<ComboCleanup> combo_cleanup;
    Clipboard clipboard;
    struct ListState {
        std::string active, anchor, query, source_selection;
        std::chrono::steady_clock::time_point typed_at{};
        std::unordered_set<std::string> selected;
        RowWindow window;
        double viewport_height{};
        std::string pinned;
    };
    std::unordered_map<std::string, ListState> lists;
    struct TreeState {
        TreeModel model;
        std::vector<std::string> expanded;
        std::vector<TreeItem> flat;
        RowModel rows;
        std::size_t expanded_count{}, loading_count{};
        bool prepared{};
        std::unordered_map<std::string, std::vector<std::size_t>> children;
    };
    std::unordered_map<std::string, TreeState> trees;
    struct TableState {
        std::unordered_map<std::string, double> widths, defaults;
        std::string column;
        TableCell anchor;
        std::vector<SortDescriptor> sort;
        struct Edit { TableCell cell; State<std::string> draft; std::string base, error; };
        std::optional<Edit> edit;
        TableSelection source;
        std::optional<TableCell> reveal;
        std::unordered_set<std::string> selected_cells;
    };
    std::unordered_map<std::string, TableState> tables;
    struct ColumnDrag { std::string table, column; double x{}, initial{}; };
    std::optional<ColumnDrag> column_drag;
    struct TabState {
        std::unordered_map<std::string, View> pages;
        double offset{}, maximum{};
        std::string active, reveal_target;
        double width{-1}, target_start{-1};
        bool reveal{true};
    };
    std::unordered_map<std::string, TabState> tabs;
    std::string text_drag;
    std::size_t text_drag_anchor{};
    Theme theme;
    bool custom_text{};
    FontEngine fonts;
    std::shared_ptr<detail::Observer> observer{std::make_shared<detail::Observer>()};
    std::thread::id owner{std::this_thread::get_id()};
    bool dirty{true}, geometry_dirty{true}, animating{}, active{true}, focus_visible{};
    Size viewport{};
    std::unique_ptr<Node> root;
    std::unordered_map<std::string, Node*> nodes;
    std::unordered_map<std::string, Point> scroll;
    std::string focused, hovered, pressed, dragging;
    double divider_drag_offset{}, divider_drag_initial{};
    SplitPane divider_drag_collapsed{SplitPane::none};
    bool split_arrow{};
    std::uint64_t generation{};
    enum class OverlayKind { menu, context, combo, popover, tooltip };
    struct Overlay {
        OverlayKind kind{OverlayKind::menu};
        std::string owner;
        std::string root_id, restore_focus;
        std::optional<Description::Popover> popover;
        bool initialized{}, restore_on_close{true};
        std::vector<Option> options;
        Rect bounds{};
        int highlighted{-1};
        double scroll{};
        struct Context {
            std::vector<MenuEntry> entries;
            std::vector<std::string> path;
            std::optional<Point> origin;
            std::string parent_menu, parent_item;
            bool restore_accents{};
        };
        std::optional<Context> context;
    };
    std::vector<Overlay> overlays, requested_overlays;
    struct ClosedOverlay { Overlay overlay; bool clear_binding{}; };
    std::vector<ClosedOverlay> closed_overlays;
    bool processing_overlays{};
    struct TooltipState {
        std::string owner, text;
        TooltipOptions options;
        std::chrono::steady_clock::time_point due;
        bool shown{};
    };
    std::optional<TooltipState> tooltip;
    std::string suppressed_tooltip;
    Overlay* interactive_overlay() {
        for (auto i = overlays.rbegin(); i != overlays.rend(); ++i)
            if (i->kind != OverlayKind::tooltip) return &*i;
        return nullptr;
    }
    const Overlay* interactive_overlay() const {
        return const_cast<Impl*>(this)->interactive_overlay();
    }
    Overlay* menu_popup() {
        auto* overlay = interactive_overlay();
        return overlay && overlay->kind == OverlayKind::menu ? overlay : nullptr;
    }
    const Overlay* menu_popup() const { return const_cast<Impl*>(this)->menu_popup(); }
    void drop_menu() { std::erase_if(overlays, [](const auto& o) { return o.kind == OverlayKind::menu || o.kind == OverlayKind::context; }); }
    Overlay* context_popup() {
        auto* top = interactive_overlay();
        return top && top->context ? top : nullptr;
    }
    const Overlay* context_popup() const { return const_cast<Impl*>(this)->context_popup(); }
    Overlay* combo_popup() {
        auto* top = interactive_overlay();
        return top && top->kind == OverlayKind::combo ? top : nullptr;
    }
    const Overlay* combo_popup() const { return const_cast<Impl*>(this)->combo_popup(); }
    const Overlay* focus_scope() const {
        for (auto i = overlays.rbegin(); i != overlays.rend(); ++i)
            if (i->popover && i->popover->options.focus == PopoverFocus::content) return &*i;
        return nullptr;
    }
    bool in_focus_scope(const Node& n) const {
        if (context_popup()) return false;
        const auto* scope = focus_scope();
        return !scope || n.id == scope->root_id || n.id.starts_with(scope->root_id + "/");
    }
    HostOptions options;
    struct Hold {
        std::string button, owner;
        double steps{};
        Point point;
        std::optional<std::chrono::steady_clock::time_point> due;
    };
    std::optional<Hold> hold;
    Impl(std::function<View()> b, std::function<void()> w, Theme t, std::shared_ptr<TextLayoutEngine> text, HostOptions o)
        : builder(std::move(b)), wake(std::move(w)), theme(std::move(t)),
          custom_text(static_cast<bool>(text)), fonts(theme, std::move(text)), options(std::move(o)) {}
    void check() const {
        if (owner != std::this_thread::get_id())
            throw std::logic_error("SCARS Host must be used on its owning thread");
    }
    void notify() {
        if (wake)
            wake();
        if (inspection_observer)
            inspection_observer();
    }
    void select_inspected(std::string id) {
        inspected = std::move(id);
        auto* node = find(inspected);
        inspection_flash_pending = !inspected.empty() && (!node || !node->hidden);
        inspection_flash_start.reset();
        notify();
    }
    void invalidate() {
        check();
        bool was = dirty;
        dirty = true;
        if (!was)
            notify();
    }
    double height(const Node& n) const {
        return n.d.density == Density::compact    ? theme.compact_height
               : n.d.density == Density::spacious ? theme.spacious_height
                                                  : theme.control_height;
    }
    std::string color_format_tag(const std::string& id) const {
        return ":color-session:" + id + ":" + std::to_string(colors.at(id).revision);
    }
    std::vector<std::string> color_fields(const std::string& id) const {
        std::vector<std::string> result;
        if (!colors.contains(id))
            return result;
        const auto tag = color_format_tag(id);
        for (const auto& [key, editor] : editors)
            if (editor->numeric_format_id.ends_with(tag))
                result.push_back(key);
        std::sort(result.begin(), result.end());
        return result;
    }
    NumericValidation color_status(const std::string& id) const {
        const auto it = colors.find(id);
        if (it == colors.end())
            return {};
        NumericValidation result{NumericStatus::valid, it->second.error,
                                 it->second.draft != it->second.source};
        for (const auto& key : color_fields(id)) {
            const auto editor = editors.find(key);
            if (editor == editors.end())
                continue;
            if (editor->second->buffer.marked_range())
                return {NumericStatus::incomplete, "Finish input-method composition before applying.", true};
            const auto& validation = editor->second->validation;
            result.dirty |= validation.dirty;
            if (validation.status != NumericStatus::valid)
                return {validation.status, validation.message, true};
        }
        if (!result.message.empty())
            result.status = NumericStatus::invalid;
        return result;
    }
    void release_colors() noexcept {
        auto cleanup = std::move(color_cleanup);
        for (const auto& [id, state] : colors)
            if (state.preview && state.config)
                cleanup.push_back({state.config->value, state.source, *state.preview});
        colors.clear();
        for (const auto& entry : cleanup) {
            try {
                if (entry.value.get() == entry.owned && entry.owned != entry.baseline)
                    entry.value.set(entry.baseline);
            } catch (...) { /* Destructors cannot propagate application setter failures. */
            }
        }
    }
    void retire_color(ColorState& state) {
        if (state.preview && state.config)
            color_cleanup.push_back({state.config->value, state.source, *state.preview});
        state.preview.reset();
        state.open = false;
        state.error.clear();
        state.draft = state.observed = state.source;
        ++state.revision;
    }
    bool color_write(const std::string& id, bool final) {
        auto* node = find(id);
        if (!node || !node->d.color || !node->enabled || node->hidden || !colors.contains(id) ||
            node->d.color->options.read_only)
            return false;
        const auto config = *node->d.color;
        auto& state = colors.at(id);
        if (state.publishing || state.batch)
            return false;
        const auto candidate = state.draft, before = config.value.get();
        const auto revision = state.revision;
        const auto baseline = state.source;
        if (before != state.observed) {
            invalidate();
            layout(viewport);
            return false;
        }
        state.publishing = true;
        if (candidate != before) {
            try {
                config.value.set(candidate);
            } catch (...) {
                if (colors.contains(id))
                    colors.at(id).publishing = false;
                invalidate();
                throw;
            }
        }
        const auto actual = config.value.get();
        const bool accepted = actual == candidate || actual != before;
        bool retained = false;
        if (colors.contains(id)) {
            auto& current = colors.at(id);
            current.publishing = false;
            if (current.revision == revision) {
                retained = true;
                if (accepted) {
                    current.observed = actual;
                    current.draft = actual;
                    current.error.clear();
                    const auto converted = color_hsv(actual);
                    if (converted.saturation > 0)
                        current.hsv.hue = converted.hue;
                    current.hsv.saturation = converted.saturation;
                    current.hsv.value = converted.value;
                    if (final) {
                        current.source = actual;
                        current.preview.reset();
                        ++current.revision;
                    } else
                        current.preview = actual;
                } else
                    current.error = "Application rejected the color.";
            }
        }
        if (!retained && !final && accepted)
            color_cleanup.push_back({config.value, baseline, actual});
        invalidate();
        return accepted;
    }
    void color_part(const std::string& id, int part, double value) {
        auto* owner = find(id);
        if (!owner || !owner->d.color || owner->hidden || !owner->enabled ||
            owner->d.color->options.read_only || !colors.contains(id))
            return;
        auto& state = colors.at(id);
        const auto config = *owner->d.color;
        if (config.value.get() != state.observed) {
            invalidate();
            return;
        }
        Color next = state.draft;
        if (part == 6)
            next = static_cast<Color>(value);
        else if (part == 3)
            next = (next & 0xffffff) | (static_cast<Color>(std::lround(value)) << 24);
        else if (state.model == ColorModel::rgb) {
            const auto shift = 16 - 8 * part;
            next = (next & ~(Color(255) << shift)) | (static_cast<Color>(std::lround(value)) << shift);
        } else {
            if (part == 0)
                state.hsv.hue = value;
            else if (part == 1)
                state.hsv.saturation = value / 100.;
            else
                state.hsv.value = value / 100.;
            next = color_from_hsv(state.hsv, static_cast<std::uint8_t>(next >> 24));
        }
        if (!config.options.alpha)
            next = (next & 0xffffff) | (state.source & 0xff000000);
        state.draft = next;
        state.error.clear();
        if (state.model == ColorModel::rgb || part == 6) {
            auto hsv = color_hsv(next);
            if (hsv.saturation == 0)
                hsv.hue = state.hsv.hue;
            state.hsv = hsv;
        }
        if (config.options.live_preview && !state.batch)
            color_write(id, false);
        invalidate();
    }
    bool collect_color(const std::string& id) {
        if (!colors.contains(id) || colors.at(id).batch || colors.at(id).publishing)
            return false;
        const auto revision = colors.at(id).revision;
        std::vector<std::pair<std::string, detail::NumericValue>> pending;
        for (const auto& field : color_fields(id)) {
            const auto& editor = *editors.at(field);
            if (editor.buffer.marked_range())
                return false;
            if (editor.validation.status != NumericStatus::valid)
                return false;
            if (editor.validation.dirty) {
                const auto parsed =
                    detail::parse_number(editor.buffer.committed_text(), *find(field)->d.numeric);
                if (!parsed.value)
                    return false;
                pending.emplace_back(field, *parsed.value);
            }
        }
        colors.at(id).error.clear();
        colors.at(id).batch = true;
        bool accepted = true;
        try {
            for (const auto& [field, value] : pending) {
                if (!colors.contains(id) || colors.at(id).revision != revision || !find(field)) {
                    accepted = false;
                    break;
                }
                write_numeric_value(*find(field), value);
            }
        } catch (...) {
            if (colors.contains(id))
                colors.at(id).batch = false;
            throw;
        }
        if (!colors.contains(id))
            return false;
        colors.at(id).batch = false;
        return accepted && colors.at(id).revision == revision &&
               color_status(id).status == NumericStatus::valid;
    }
    bool color_apply(std::string id) {
        auto* node = find(id);
        if (!node || !node->d.color || !node->enabled || node->hidden || node->d.color->options.read_only)
            return false;
        if (!collect_color(id) || !color_write(id, true)) {
            invalidate();
            layout(viewport);
            return false;
        }
        if (colors.contains(id))
            colors.at(id).open = false;
        invalidate();
        layout(viewport);
        notify();
        return true;
    }
    bool color_cancel(std::string id) {
        if (!colors.contains(id))
            return false;
        const bool pending =
            colors.at(id).open || color_status(id).dirty || colors.at(id).preview.has_value();
        retire_color(colors.at(id));
        invalidate();
        layout(viewport);
        notify();
        return pending;
    }
    void color_mode(std::string id, const std::string& mode) {
        if (!collect_color(id))
            return;
        auto& state = colors.at(id);
        if (state.config->options.live_preview && !color_write(id, false))
            return;
        if (!colors.contains(id))
            return;
        colors.at(id).model = mode == "hsv" ? ColorModel::hsv : ColorModel::rgb;
        ++colors.at(id).revision;
        invalidate();
        layout(viewport);
        notify();
    }
    View color_body(const std::string& id) {
        const auto& state = colors.at(id);
        const auto config = *state.config;
        std::weak_ptr<Impl> weak = shared_from_this();
        auto preview = std::make_shared<Description>(Label("Color preview").height(52).description());
        preview->sample = state.draft;
        preview->semantic_type = "ColorPreview";
        std::vector<View> body{View(preview).key("preview")};
        Binding<std::string> mode{[weak, id] {
                                      auto h = weak.lock();
                                      return h && h->colors.contains(id) &&
                                                     h->colors.at(id).model == ColorModel::hsv
                                                 ? std::string("hsv")
                                                 : std::string("rgb");
                                  },
                                  [weak, id](std::string value) {
                                      if (auto h = weak.lock())
                                          h->color_mode(id, value);
                                  }};
        body.push_back(SegmentedControl(mode, {{"rgb", "RGB"}, {"hsv", "HSV"}})
                           .enabled(!config.options.read_only)
                           .key("model"));
        for (int part = 0; part < (config.options.alpha ? 4 : 3); ++part) {
            const auto model = state.model;
            const double max = part == 3 || model == ColorModel::rgb ? 255 : part == 0 ? 360 : 100;
            const std::string label = part == 3                  ? "Alpha"
                                      : model == ColorModel::rgb ? (part == 0   ? "Red"
                                                                    : part == 1 ? "Green"
                                                                                : "Blue")
                                                                 : (part == 0   ? "Hue"
                                                                    : part == 1 ? "Saturation"
                                                                                : "Value");
            Binding<double> component{
                [weak, id, part, model] {
                    auto h = weak.lock();
                    if (!h || !h->colors.contains(id))
                        return 0.;
                    const auto& s = h->colors.at(id);
                    if (part == 3)
                        return double(s.draft >> 24);
                    if (model == ColorModel::rgb)
                        return double((s.draft >> (16 - 8 * part)) & 255);
                    return part == 0 ? s.hsv.hue : part == 1 ? s.hsv.saturation * 100 : s.hsv.value * 100;
                },
                [weak, id, part](double value) {
                    if (auto h = weak.lock())
                        h->color_part(id, part, value);
                }};
            auto slider = Slider(component, {.min = 0, .max = max, .step = 1, .page_step = max / 10})
                              .accessible_label(label)
                              .grow()
                              .basis(110)
                              .min_width(70)
                              .enabled(!config.options.read_only)
                              .key("slider");
            auto field = NumericField(label, component,
                                      {.min = 0,
                                       .max = max,
                                       .step = 1,
                                       .significant_digits = 7,
                                       .read_only = config.options.read_only})
                             .width(86)
                             .key("field");
            auto field_description = std::make_shared<Description>(field.description());
            field_description->color_component = true;
            field = View(field_description);
            body.push_back(
                FlexColumn{Label(label, LabelStyle::micro), FlexRow{slider, field}.gap(10)}.gap(4).key(
                    "channel" + std::to_string(part)));
        }
        Binding<std::int64_t> hex{[weak, id] {
                                      auto h = weak.lock();
                                      return h && h->colors.contains(id)
                                                 ? std::int64_t(h->colors.at(id).draft)
                                                 : 0;
                                  },
                                  [weak, id](std::int64_t color) {
                                      if (auto h = weak.lock())
                                          h->color_part(id, 6, double(color));
                                  }};
        auto field = NumericField(
            "Hex", hex, {.min = 0, .max = std::int64_t(0xffffffff), .read_only = config.options.read_only});
        auto desc = std::make_shared<Description>(field.description());
        const bool alpha = config.options.alpha;
        const auto fixed_alpha = state.source & 0xff000000;
        desc->color_component = true;
        desc->numeric->format_id = "color-hex:" + std::to_string(alpha);
        desc->numeric->format = [alpha](detail::NumericValue value) {
            return format_color(static_cast<Color>(std::get<0>(value)), alpha);
        };
        desc->numeric->parse = [alpha, fixed_alpha](const std::string& text) -> detail::ParsedNumber {
            auto color = parse_color(text);
            auto first = text.find_first_not_of(" \t\r\n"), last = text.find_last_not_of(" \t\r\n");
            const auto count = first == std::string::npos ? 0 : last - first + 1 - (text[first] == '#');
            if (!alpha && (count == 4 || count == 8))
                return {{}, {NumericStatus::invalid, "Alpha editing is disabled."}};
            if (!color)
                return {{},
                        {count < 8 ? NumericStatus::incomplete : NumericStatus::invalid,
                         "Enter hex RGB or RGBA digits."}};
            if (!alpha)
                *color = (*color & 0xffffff) | fixed_alpha;
            return {std::int64_t(*color), {}};
        };
        body.push_back(FlexColumn{Label(alpha ? "Hex · #RRGGBBAA" : "Hex · #RRGGBB", LabelStyle::micro),
                                  View(desc).key("hex")}
                           .gap(4)
                           .key("hex-row"));
        const auto validation = color_status(id);
        if (!validation.message.empty())
            body.push_back(
                Label(validation.message, LabelStyle::error).text_wrap(TextWrap::wrap).key("validation"));
        else
            body.push_back(Label(config.options.live_preview
                                     ? "Live preview · Cancel restores your starting color"
                                     : "Draft · Apply commits the color",
                                 LabelStyle::secondary)
                               .text_wrap(TextWrap::wrap)
                               .key("status"));
        body.push_back(FlexRow{Button("Cancel",
                                      [weak, id] {
                                          if (auto h = weak.lock())
                                              h->color_cancel(id);
                                      })
                                   .key("cancel"),
                               Button("Apply",
                                      [weak, id] {
                                          if (auto h = weak.lock())
                                              h->color_apply(id);
                                      })
                                   .enabled(!config.options.read_only)
                                   .key("apply")}
                           .gap(10)
                           .wrap());
        return FlexColumn(std::move(body)).gap(10);
    }
    void prepare_color(Node& node, Description& d) {
        const auto id = node.id;
        const auto config = *d.color;
        auto& state = colors[id];
        const auto actual = config.value.get();
        const auto signature =
            std::to_string(config.options.alpha) + ":" + std::to_string(config.options.live_preview) + ":" +
            std::to_string(config.options.read_only) + ":" + std::to_string(int(config.options.model));
        const bool external = state.initialized && !state.publishing && state.observed != actual;
        if (!state.initialized || external || state.configuration != signature) {
            if (state.initialized && !external)
                retire_color(state);
            state.initialized = true;
            state.source = state.observed = state.draft = actual;
            state.hsv = color_hsv(actual);
            state.configuration = signature;
            state.model = config.options.model;
            state.preview.reset();
            state.error.clear();
            ++state.revision;
        }
        state.config = config;
        if ((node.hidden || !node.enabled || !active || (d.expansion && !d.expansion->get())) &&
            (state.open || state.preview || state.draft != state.source || color_status(id).dirty))
            retire_color(state);
        std::weak_ptr<Impl> weak = shared_from_this();
        if (config.well) {
            d.sample = actual;
            d.click = [weak, id] {
                if (auto h = weak.lock()) {
                    auto* n = h->find(id);
                    if (!n || !n->d.color)
                        return;
                    if (h->colors.at(id).open)
                        h->color_cancel(id);
                    else {
                        h->colors.at(id).open = true;
                        h->invalidate();
                    }
                }
            };
            Binding<bool> open{[weak, id] {
                                   auto h = weak.lock();
                                   return h && h->colors.contains(id) && h->colors.at(id).open;
                               },
                               [weak, id](bool value) {
                                   if (auto h = weak.lock(); h && h->colors.contains(id)) {
                                       h->colors.at(id).open = value;
                                       h->invalidate();
                                   }
                               }};
            d.popover =
                Description::Popover{open,
                                     [weak, id] {
                                         auto h = weak.lock();
                                         return h && h->colors.contains(id) ? h->color_body(id) : View{};
                                     },
                                     {.width = 340, .max_height = 680, .on_dismiss = [weak, id] {
                                          if (auto h = weak.lock(); h && h->colors.contains(id))
                                              h->color_cancel(id);
                                      }}};
        } else
            d.children = {color_body(id).key("body")};
        node.d = d;
    }
    static std::string datetime_component_id(const std::string& id, bool date) {
        return id + "/@0/key/components/key/" + (date ? "date" : "time");
    }
    NumericValidation check_datetime(const Description::DateTime& config, LocalDateTime value) const {
        const auto& o = config.options;
        if (!valid(value.date) || !valid(value.time))
            return {NumericStatus::invalid, "Enter a valid date and time."};
        if (value.date < o.date.min || value.date > o.date.max || value.time < o.time.min ||
            value.time > o.time.max || value < o.min || value > o.max)
            return {NumericStatus::out_of_range, "Date and time are outside the allowed range."};
        if (o.date.disabled_date && o.date.disabled_date(value.date))
            return {NumericStatus::invalid, "This date is unavailable."};
        auto error = o.date.validate ? o.date.validate(value.date) : std::string{};
        if (error.empty() && o.time.validate)
            error = o.time.validate(value.time);
        if (error.empty() && o.validate)
            error = o.validate(value);
        return error.empty() ? NumericValidation{}
                             : NumericValidation{NumericStatus::invalid, std::move(error)};
    }
    std::string datetime_configuration(const Description::DateTime& c) const {
        const auto& o = c.options;
        return c.context + ":" + std::to_string(detail::date_days(o.min.date)) + ":" +
               std::to_string(detail::time_seconds(o.min.time)) + ":" +
               std::to_string(detail::date_days(o.max.date)) + ":" +
               std::to_string(detail::time_seconds(o.max.time)) + ":" +
               std::to_string(detail::date_days(o.date.min)) + ":" +
               std::to_string(detail::date_days(o.date.max)) + ":" +
               std::to_string(detail::time_seconds(o.time.min)) + ":" +
               std::to_string(detail::time_seconds(o.time.max)) + ":" + o.date.locale + ":" + o.time.locale +
               ":" + std::to_string(int(o.time.hour_cycle)) + ":" + std::to_string(o.time.show_seconds) +
               ":" + std::to_string(o.date.read_only) + ":" + std::to_string(o.time.read_only);
    }
    bool datetime_buffers_pending(const std::string& id) const {
        for (bool date : {true, false}) {
            const auto found = editors.find(datetime_component_id(id, date));
            if (found != editors.end() &&
                (found->second->validation.dirty || found->second->buffer.marked_range()))
                return true;
        }
        return false;
    }
    NumericValidation inspect_datetime_draft(const Description::DateTime& config, const std::string& id,
                                             const DateTimeState& state, bool buffers = true) const {
        auto prospective = state.draft;
        bool dirty = state.draft != state.source;
        if (buffers)
            for (bool date : {true, false}) {
                const auto field_id = datetime_component_id(id, date);
                const auto found = editors.find(field_id);
                if (found == editors.end())
                    continue;
                const auto& editor = *found->second;
                if (editor.buffer.marked_range())
                    return {NumericStatus::incomplete, "Finish input-method composition before committing.",
                            true};
                if (!editor.validation.dirty)
                    continue;
                dirty = true;
                const auto* field = find(field_id);
                const auto codec = field && field->d.numeric ? *field->d.numeric
                                   : date                    ? detail::date_codec(config.options.date)
                                                             : detail::time_codec(config.options.time);
                const auto parsed = detail::parse_number(editor.buffer.committed_text(), codec);
                if (!parsed.value) {
                    auto result = parsed.validation;
                    result.dirty = true;
                    return result;
                }
                if (date)
                    prospective.date = detail::date_from_days(std::get<0>(*parsed.value));
                else
                    prospective.time = detail::time_from_seconds(std::get<0>(*parsed.value));
            }
        auto result = check_datetime(config, prospective);
        if (result.status == NumericStatus::valid && state.rejected && prospective == state.draft)
            result = {NumericStatus::invalid, "Application rejected the date/time value."};
        result.dirty = dirty;
        return result;
    }
    void prepare_datetime(Node& node, Description& d) {
        const auto id = node.id;
        const auto config = *d.datetime;
        const auto source = config.value.get();
        if (!valid(source.date) || !valid(source.time))
            throw std::invalid_argument("Invalid local date/time value");
        const auto token = config.source_token ? config.source_token() : std::string{};
        const auto signature = datetime_configuration(config);
        auto& state = datetimes[id];
        const bool inactive = node.hidden || !node.enabled || !active || (d.expansion && !d.expansion->get());
        const bool reset = !state.initialized || state.source != source || state.token != token ||
                           state.configuration != signature ||
                           (inactive && (state.draft != state.source || datetime_buffers_pending(id)));
        if (reset) {
            state.initialized = true;
            state.source = source;
            state.draft = source;
            state.token = token;
            state.configuration = signature;
            state.rejected = false;
            ++state.revision;
        }
        const auto validation = inspect_datetime_draft(config, id, state, !reset);
        state.error = validation.message;
        std::weak_ptr<Impl> weak = shared_from_this();
        Binding<Date> date{[weak, id, source] {
                               auto h = weak.lock();
                               return h && h->datetimes.contains(id) ? h->datetimes.at(id).draft.date
                                                                     : source.date;
                           },
                           [weak, id](Date next) {
                               if (auto h = weak.lock())
                                   h->datetime_part(id, next, {});
                           }};
        Binding<Time> time{[weak, id, source] {
                               auto h = weak.lock();
                               return h && h->datetimes.contains(id) ? h->datetimes.at(id).draft.time
                                                                     : source.time;
                           },
                           [weak, id](Time next) {
                               if (auto h = weak.lock())
                                   h->datetime_part(id, {}, next);
                           }};
        std::vector<View> body{FlexRow{DatePicker(d.text + " date", date, config.options.date)
                                           .key("date")
                                           .grow()
                                           .basis(160)
                                           .min_width(150),
                                       TimePicker(d.text + " time", time, config.options.time)
                                           .key("time")
                                           .grow()
                                           .basis(150)
                                           .min_width(140)}
                                   .gap(12)
                                   .wrap()
                                   .key("components")};
        if (!state.error.empty())
            body.push_back(Label(state.error, validation.status == NumericStatus::incomplete
                                                  ? LabelStyle::secondary
                                                  : LabelStyle::error)
                               .text_wrap(TextWrap::wrap)
                               .key("validation"));
        else if (validation.dirty)
            body.push_back(Label("Pending date/time draft", LabelStyle::secondary).key("validation"));
        d.children = {FlexColumn(std::move(body)).gap(8)};
        node.d.children = d.children;
    }
    bool datetime_publish(std::string id) {
        auto* group = find(id);
        if (!group || !group->d.datetime || group->hidden || !group->enabled || !datetimes.contains(id))
            return false;
        const auto config = *group->d.datetime;
        auto& state = datetimes.at(id);
        if (state.batch || state.publishing)
            return false;
        const auto validation = check_datetime(config, state.draft);
        state.error = validation.message;
        if (validation.status != NumericStatus::valid || datetime_buffers_pending(id))
            return false;
        if (state.draft == state.source) {
            state.rejected = false;
            return true;
        }
        const auto candidate = state.draft, source = state.source;
        const auto revision = state.revision;
        state.publishing = true;
        try {
            config.value.set(candidate);
        } catch (...) {
            if (datetimes.contains(id))
                datetimes.at(id).publishing = false;
            invalidate();
            throw;
        }
        const auto actual = config.value.get();
        const bool accepted = actual == candidate || actual != source;
        if (datetimes.contains(id)) {
            auto& current = datetimes.at(id);
            current.publishing = false;
            if (current.revision == revision) {
                if (accepted) {
                    current.source = actual;
                    current.draft = actual;
                    current.token = config.source_token ? config.source_token() : std::string{};
                    current.rejected = false;
                    current.error = check_datetime(config, actual).message;
                } else {
                    current.rejected = true;
                    current.error = "Application rejected the date/time value.";
                }
            }
        }
        invalidate();
        layout(viewport);
        notify();
        return accepted && check_datetime(config, actual).status == NumericStatus::valid;
    }
    void datetime_part(std::string id, std::optional<Date> date, std::optional<Time> time) {
        auto* group = find(id);
        if (!group || !group->d.datetime || group->hidden || !group->enabled || !datetimes.contains(id))
            return;
        if ((date && group->d.datetime->options.date.read_only) ||
            (time && group->d.datetime->options.time.read_only))
            return;
        auto& state = datetimes.at(id);
        if (date)
            state.draft.date = *date;
        if (time)
            state.draft.time = *time;
        state.rejected = false;
        state.error = check_datetime(*group->d.datetime, state.draft).message;
        const bool publish = !state.batch;
        if (publish)
            datetime_publish(id);
        invalidate();
    }
    bool cancel_datetime(std::string id) {
        const auto* group = find(id);
        if (!group || !group->d.datetime || !datetimes.contains(id))
            return false;
        auto& state = datetimes.at(id);
        const bool pending = state.draft != state.source || datetime_buffers_pending(id) || state.rejected;
        if (!pending)
            return false;
        state.draft = state.source;
        state.rejected = false;
        ++state.revision;
        state.error = check_datetime(*group->d.datetime, state.source).message;
        invalidate();
        layout(viewport);
        notify();
        return true;
    }
    NumericValidation datetime_validation(const std::string& id) const {
        const auto* group = find(id);
        if (!group || !group->d.datetime || !datetimes.contains(id))
            return {};
        return inspect_datetime_draft(*group->d.datetime, id, datetimes.at(id));
    }
    bool commit_datetime(std::string id) {
        layout(viewport);
        auto* group = find(id);
        if (!group || !group->d.datetime || group->hidden || !group->enabled || !datetimes.contains(id))
            return false;
        if (!in_focus_scope(*group) && (!focus_scope() || !focus_scope()->root_id.starts_with(id + "/")))
            return false;
        if (datetimes.at(id).batch || datetimes.at(id).publishing)
            return false;
        const auto revision = datetimes.at(id).revision;
        datetimes.at(id).batch = true;
        bool valid = true;
        try {
            for (bool date : {true, false}) {
                const auto field_id = datetime_component_id(id, date);
                auto* field = find(field_id);
                if (!field || !datetimes.contains(id) || datetimes.at(id).revision != revision) {
                    valid = false;
                    break;
                }
                if (!field->d.numeric->read_only && !commit_numeric(field_id)) {
                    valid = false;
                    break;
                }
            }
        } catch (...) {
            if (datetimes.contains(id))
                datetimes.at(id).batch = false;
            throw;
        }
        if (!datetimes.contains(id))
            return false;
        datetimes.at(id).batch = false;
        if (!valid || datetimes.at(id).revision != revision)
            return false;
        return datetime_publish(id);
    }
    bool calendar_available(const Node& owner, Date date) const {
        return owner.d.date && owner.enabled && !owner.hidden && !owner.d.date->read_only &&
               owner.d.numeric->validate(detail::date_days(date)).status == NumericStatus::valid;
    }
    bool calendar_select(std::string id, Date date) {
        auto* owner = find(id);
        const auto* overlay = interactive_overlay();
        if (!owner || !calendar_available(*owner, date) || !overlay || overlay->owner != id + "/action")
            return false;
        calendars.at(id).open = false;
        write_numeric_value(*owner, detail::date_days(date));
        focus(id, focus_visible);
        return true;
    }
    bool calendar_month(std::string id, int delta) {
        auto* owner = find(id);
        if (!owner || !owner->d.date || !calendars.at(id).open || !owner->enabled || owner->hidden)
            return false;
        auto next = detail::shift_month(calendars.at(id).month, delta);
        next.day = 1;
        auto min = owner->d.date->min, max = owner->d.date->max;
        min.day = max.day = 1;
        if (next < min || next > max || next == calendars.at(id).month)
            return false;
        calendars.at(id).month = next;
        invalidate();
        layout(viewport);
        notify();
        return true;
    }
    View calendar_body(const std::string& id) {
        const auto* owner = find(id);
        if (!owner || !owner->d.date)
            return {};
        const auto state = calendars.at(id);
        const auto page = detail::calendar_page(state.month, owner->d.date->locale);
        const auto source = detail::date_from_days(std::get<0>(*owner->numeric_value));
        std::weak_ptr<Impl> weak = shared_from_this();
        auto navigation = [&](const char* caption, const char* label, int delta) -> View {
            auto target = detail::shift_month(state.month, delta);
            target.day = 1;
            auto min = owner->d.date->min, max = owner->d.date->max;
            min.day = max.day = 1;
            return Button(caption,
                          [weak, id, delta] {
                              if (auto host = weak.lock())
                                  host->calendar_month(id, delta);
                          })
                .accessible_label(label)
                .enabled(target != state.month && target >= min && target <= max)
                .key(label)
                .width(34)
                .shrink(0)
                .density(Density::compact)
                .font_size(12);
        };
        std::vector<View> rows;
        rows.push_back(FlexRow{navigation("«", "Previous year", -12), navigation("‹", "Previous month", -1),
                               Label(page.title)
                                   .grow()
                                   .shrink()
                                   .min_width(0)
                                   .text_align(Align::center)
                                   .font_size(13)
                                   .key("calendar-title"),
                               navigation("›", "Next month", 1), navigation("»", "Next year", 12)}
                           .gap(3)
                           .align_items(Align::center));
        std::vector<View> weekdays;
        for (const auto& label : page.weekdays)
            weekdays.push_back(Label(label, LabelStyle::secondary)
                                   .font_size(10)
                                   .grow()
                                   .basis(0)
                                   .min_width(0)
                                   .text_align(Align::center));
        rows.push_back(FlexRow(std::move(weekdays)).gap(4));
        for (std::size_t week = 0; week < 6; ++week) {
            std::vector<View> days;
            for (std::size_t day = 0; day < 7; ++day) {
                const auto index = week * 7 + day;
                if (!page.days[index]) {
                    days.push_back(Spacer{}.grow().basis(0));
                    continue;
                }
                const auto date = *page.days[index];
                days.push_back(Button(page.day_labels[index],
                                      [weak, id, date] {
                                          if (auto host = weak.lock())
                                              host->calendar_select(id, date);
                                      })
                                   .key("calendar-day-" + std::to_string(detail::date_days(date)))
                                   .accessible_label(format_date(date, owner->d.date->locale))
                                   .enabled(calendar_available(*owner, date))
                                   .font_size(12)
                                   .grow()
                                   .basis(0)
                                   .min_width(0)
                                   .height(32)
                                   .corner_radius(4)
                                   .background(date == source ? owner->accent : theme.inactive)
                                   .foreground(date == source                    ? contrast(owner->accent)
                                               : date.month == state.month.month ? theme.text
                                                                                 : theme.secondary));
            }
            rows.push_back(FlexRow(std::move(days)).gap(4).height(32).shrink(0));
        }
        return FlexColumn(std::move(rows)).gap(6);
    }
    void prepare_calendar(Node& owner) {
        const auto id = owner.id;
        auto& state = calendars[id];
        const auto source = detail::date_from_days(std::get<0>(*owner.numeric_value));
        if (!state.initialized || state.source != source || state.min != owner.d.date->min ||
            state.max != owner.d.date->max || state.locale != owner.d.date->locale || state.context!=owner.d.numeric->format_id) {
            state.initialized = true;
            state.source = source;
            state.min = owner.d.date->min;
            state.max = owner.d.date->max;
            state.locale = owner.d.date->locale;
            state.context=owner.d.numeric->format_id;
            state.month = std::clamp(source, state.min, state.max);
            state.month.day = 1;
            state.active = source;
            state.open = false;
        }
        if (owner.hidden || !owner.enabled || owner.d.date->read_only || !active)
            state.open = false;
        std::weak_ptr<Impl> weak = shared_from_this();
        Binding<bool> open{[weak, id] {
                               auto h = weak.lock();
                               return h && h->calendars.contains(id) && h->calendars.at(id).open;
                           },
                           [weak, id](bool value) {
                               if (auto h = weak.lock(); h && h->calendars.contains(id)) {
                                   h->calendars.at(id).open = value;
                                   h->invalidate();
                               }
                           }};
        auto action = Button("", {}).popover(open,
                                             [weak, id] {
                                                 if (auto h = weak.lock())
                                                     return h->calendar_body(id);
                                                 return View{};
                                             },
                                             {.width = 340, .max_height = 390});
        auto child = mount(action, id + "/action", &owner);
        child->d.kind = Kind::field_action;
        child->d.calendar_action = true;
        child->d.accessible = state.open ? "Hide calendar" : "Show calendar";
        child->group = id;
        child->enabled = child->enabled && !owner.d.date->read_only;
        child->layout_editable = false;
        const auto tag = [&](auto&& self, Node& node) -> void {
            if (node.d.key.starts_with("calendar-day-")) {
                node.calendar_owner = id;
                node.calendar_date = detail::date_from_days(std::stoll(node.d.key.substr(13)));
                node.number = *node.calendar_date == source;
            }
            for (auto& c : node.children)
                self(self, *c);
            for (auto& c : node.overlay_children)
                self(self, *c);
        };
        tag(tag, *child);
        if (auto* popup = find(id + "/action/popover"))
            popup->d.accessible = "Calendar for " + owner.d.accessible;
        owner.children.push_back(std::move(child));
    }
    bool calendar_focus(std::string id, Date target, int direction) {
        auto* owner = find(id);
        if (!owner || !owner->d.date || !calendars.contains(id) || !calendars.at(id).open)
            return false;
        const auto config = *owner->d.date;
        target = std::clamp(target, config.min, config.max);
        for (unsigned attempt = 0; attempt < 366; ++attempt) {
            owner = find(id);
            if (!owner || !calendar_available(*owner, target)) {
                const auto ordinal = detail::date_days(target) + direction;
                if (ordinal < detail::date_days(config.min) || ordinal > detail::date_days(config.max))
                    return true;
                target = detail::date_from_days(ordinal);
                continue;
            }
            auto& state = calendars.at(id);
            state.active = target;
            state.month = target;
            state.month.day = 1;
            invalidate();
            layout(viewport);
            std::string target_id;
            for (const auto& [node_id, node] : nodes)
                if (node->calendar_owner == id && node->calendar_date == target) {
                    target_id = node_id;
                    break;
                }
            return !target_id.empty() && focus(target_id, focus_visible);
        }
        return true;
    }
    bool open_calendar(std::string id, bool toggle = false) {
        auto* owner = find(id);
        if (!owner || !owner->d.date || !interactive(*owner) || owner->d.date->read_only)
            return false;
        if (calendars.at(id).open && toggle) {
            calendars.at(id).open = false;
            invalidate();
            layout(viewport);
            focus(id, false);
            return true;
        }
        if (!in_focus_scope(*owner) || !focus(id, false))
            return false;
        unmark_text(true);
        layout(viewport);
        owner = find(id);
        if (owner && owner->d.date && owner->d.date->commit_on_blur && editors.at(id)->validation.dirty &&
            editors.at(id)->validation.status == NumericStatus::valid)
            commit_numeric(id);
        layout(viewport);
        owner = find(id);
        if (!owner || !owner->d.date || !interactive(*owner) || owner->d.date->read_only)
            return false;
        auto& state = calendars.at(id);
        state.active = std::clamp(detail::date_from_days(std::get<0>(*owner->numeric_value)),
                                  owner->d.date->min, owner->d.date->max);
        state.month = state.active;
        state.month.day = 1;
        state.open = true;
        const auto target = state.active;
        invalidate();
        layout(viewport);
        calendar_focus(id, target, 1);
        return calendars.contains(id) && calendars.at(id).open;
    }
    bool calendar_key(const Node& day, const KeyEvent& event) {
        if (!day.calendar_date || event.command || event.control || event.alt)
            return false;
        const auto date = *day.calendar_date;
        const auto id = day.calendar_owner;
        const auto ordinal = detail::date_days(date);
        auto target = date;
        int direction = 1;
        switch (event.key) {
        case Key::left:
            direction = -1;
            [[fallthrough]];
        case Key::right:
            target = detail::date_from_days(std::clamp(ordinal + direction, detail::date_days({1, 1, 1}),
                                                       detail::date_days({9999, 12, 31})));
            break;
        case Key::up:
            direction = -1;
            [[fallthrough]];
        case Key::down:
            target = detail::date_from_days(std::clamp(ordinal + direction * 7, detail::date_days({1, 1, 1}),
                                                       detail::date_days({9999, 12, 31})));
            break;
        case Key::page_up:
            direction = -1;
            [[fallthrough]];
        case Key::page_down:
            target = detail::shift_month(date, direction * (event.shift ? 12 : 1));
            break;
        case Key::home:
            direction = -1;
            [[fallthrough]];
        case Key::end: {
            const auto* owner = find(id);
            if (!owner || !owner->d.date)
                return false;
            const auto page = detail::calendar_page(date, owner->d.date->locale);
            const auto weekday =
                std::chrono::weekday{std::chrono::sys_days{std::chrono::days{ordinal}}}.c_encoding();
            const auto offset = (weekday + 7 - page.first_weekday) % 7;
            target = detail::date_from_days(
                std::clamp(ordinal + (direction < 0 ? -std::int64_t(offset) : 6 - std::int64_t(offset)),
                           detail::date_days({1, 1, 1}), detail::date_days({9999, 12, 31})));
            break;
        }
        default:
            return false;
        }
        focus_visible = false;
        return calendar_focus(id, target, direction);
    }
    void collect_number(Node& n) {
        if (n.d.read_number) {
            n.number = n.d.read_number();
            if (!std::isfinite(n.number))
                throw std::invalid_argument("Control value must be finite");
            if (n.d.kind == Kind::slider)
                n.number = std::clamp(n.number, n.d.range.min, n.d.range.max);
            if (n.d.kind == Kind::progress || n.d.kind == Kind::split_view)
                n.number = std::clamp(n.number, 0., 1.);
        }
        if (n.d.read_selection)
            n.selection = n.d.read_selection();
        if (n.d.read_selected)
            n.selected_ids = n.d.read_selected();
        if (n.d.expansion)
            n.expanded = n.d.expansion->get();
        if (n.d.numeric) {
            n.numeric_value = n.d.read_numeric();
            n.numeric_text = detail::format_number(*n.numeric_value, *n.d.numeric);
            n.number = std::visit([](auto value) { return static_cast<double>(value); }, *n.numeric_value);
        }
        if (n.d.kind == Kind::split_view && n.d.split_options.collapsed)
            n.collapsed = n.d.split_options.collapsed->get();
    }
    void prepare_table(Description& d, const std::string& id) {
        auto& state = tables[id];
        const auto& config = *d.table;
        const auto& columns = config.model.columns();
        std::unordered_map<std::string, double> supplied;
        if (config.options.widths)
            for (const auto& width : config.options.widths->get()) {
                if (!std::isfinite(width.width) || !supplied.emplace(width.column, width.width).second)
                    throw std::invalid_argument("Table width overrides must be finite and unique");
            }
        for (const auto& col : columns) {
            if (!state.defaults.contains(col.id) || state.defaults.at(col.id) != col.width)
                state.widths[col.id] = col.width;
            state.defaults[col.id] = col.width;
            if (config.options.widths)
                state.widths[col.id] = supplied.contains(col.id) ? supplied.at(col.id) : col.width;
            state.widths[col.id] = std::clamp(state.widths.at(col.id), col.min_width, col.max_width);
        }
        std::erase_if(state.widths,
                      [&](const auto& value) { return !config.model.column_index(value.first); });
        std::erase_if(state.defaults,
                      [&](const auto& value) { return !config.model.column_index(value.first); });
        const auto selection = config.selection.get();
        if (state.source != selection && config.options.selection_mode == TableSelectionMode::cells &&
            !selection.cells.empty())
            state.column = selection.cells.front().column;
        if (state.source != selection && !state.edit &&
            config.options.selection_mode == TableSelectionMode::cells && !selection.cells.empty())
            state.reveal = selection.cells.front();
        state.source = selection;
        state.selected_cells.clear();
        for (const auto& cell : selection.cells)
            state.selected_cells.insert(table_cell_id(id, cell));
        if (!config.model.column_index(state.column))
            state.column = columns.empty() ? std::string{} : columns.front().id;
        if (config.options.sort)
            state.sort = config.options.sort->get();
        std::unordered_set<std::string> sorted_columns;
        std::erase_if(state.sort,[&](const auto& sort){return !config.model.column_index(sort.column)||!sorted_columns.insert(sort.column).second;});
        if (state.edit) {
            const auto row = config.model.rows().index_of(state.edit->cell.row);
            const auto col = config.model.column_index(state.edit->cell.column);
            if (!row || !col || !config.model.rows().at(*row).enabled || !columns[*col].commit || d.hidden ||
                !d.enabled) {
                if (auto editor = editors.find(focused); editor != editors.end())
                    editor->second->buffer.unmark(false);
                state.edit.reset();
                if (focused.starts_with(id + "/option/"))
                    focused = id;
            }
        }
        d.rows = config.model.rows();
    }
    std::unique_ptr<Node> mount_table_header(Node& list) {
        auto header = mount(Layer{}, list.id + "/header", &list);
        header->d.kind = Kind::table_header;
        header->layout_editable = false;
        header->table_owner = list.id;
        header->group = list.id;
        for (const auto& col : list.d.table->model.columns()) {
            auto item = mount(Button(col.title, {}), header->id + "/" + path_key(col.id), header.get());
            item->d.kind = Kind::table_column;
            if(col.sortable)item->d.click=[]{};
            item->table_owner = list.id;
            item->column = col.id;
            item->layout_editable = false;
            item->group = header->id;
            if (col.resizable) {
                auto resize = mount(Button("Resize " + col.title, {}), item->id + "/resize", item.get());
                resize->d.kind = Kind::table_resize;
                resize->table_owner = list.id;
                resize->column = col.id;
                resize->group = item->id;
                resize->layout_editable = false;
                item->paint_children.push_back(resize.get());
                item->children.push_back(std::move(resize));
            }
            header->paint_children.push_back(item.get());
            header->children.push_back(std::move(item));
        }
        return header;
    }
    std::unique_ptr<Node> mount_table_row(Node& list, std::size_t index) {
        const auto config = *list.d.table;
        const auto& option = config.model.rows().at(index);
        auto row = mount(Layer{}.enabled(option.enabled), list.id + "/option/" + path_key(option.id), &list);
        for (std::size_t i = 0; i < config.model.columns().size(); ++i) {
            const auto& column = config.model.columns()[i];
            const TableCell key{option.id, column.id};
            const auto value = config.model.text(index, i);
            auto& state = tables.at(list.id);
            View content;
            if (state.edit && state.edit->cell == key) {
                std::weak_ptr<Impl> weak = shared_from_this();
                content = TextField(column.title, state.edit->draft.binding(),
                                    {.multiline = column.multiline,
                                     .on_submit =
                                         [weak, id = list.id] {
                                             if (auto host = weak.lock())
                                                 host->table_commit(id);
                                         }})
                              .key("editor")
                              .accessible_label("Edit " + column.title);
            } else
                content = column.content ? column.content(option, index)
                                         : Label(value)
                                               .text_align(column.alignment)
                                               .text_overflow(TextOverflow::ellipsis)
                                               .max_lines(1);
            auto cell = mount(Layer{content}.padding(Insets::symmetric(3, 6)),
                              row->id + "/cell/" + path_key(column.id), row.get());
            cell->d.kind = Kind::table_cell;
            cell->d.text = value;
            cell->d.accessible = column.title;
            cell->table_owner = list.id;
            cell->column = column.id;
            cell->selection = option.id;
            cell->group = row->id;
            cell->row_index = index;
            cell->layout_editable = false;
            std::function<void(Node&)> tag = [&](Node& node) {
                node.table_owner = list.id;
                node.column = column.id;
                for (auto& child : node.children)
                    tag(*child);
            };
            tag(*cell);
            row->paint_children.push_back(cell.get());
            row->children.push_back(std::move(cell));
        }
        return row;
    }
    double table_extent(const Node& list) const {
        double width{};
        for (const auto& col : list.d.table->model.columns())
            width += tables.at(list.id).widths.at(col.id);
        return width;
    }
    std::string table_cell_id(const std::string& id, const TableCell& cell) const {
        return id + "/option/" + path_key(cell.row) + "/cell/" + path_key(cell.column);
    }
    bool table_reveal(std::string id, TableCell cell) {
        for (unsigned attempt = 0; attempt < 4; ++attempt) {
            layout(viewport);
            auto* list = find(id);
            if (!list || !list->d.table)
                return false;
            auto row = list->d.table->model.rows().index_of(cell.row);
            if (!row || !list->d.table->model.column_index(cell.column) || !reveal_item(id, *row))
                return false;
            list = find(id);
            if (!list || !list->d.table || list->hidden || !list->enabled || !in_focus_scope(*list))
                return false;
            const auto model = list->d.table->model;
            row = model.rows().index_of(cell.row);
            const auto column = model.column_index(cell.column);
            if (!row || !column)
                return false;
            const auto viewport = content_box(*list);
            double left{};
            for (std::size_t i = 0; i < *column; ++i)
                left += tables.at(id).widths.at(model.columns()[i].id);
            auto& x = scroll[id].x;
            const auto right = left + tables.at(id).widths.at(cell.column);
            const auto before = x;
            if (left < x)
                x = left;
            else if (right > x + viewport.width)
                x = std::min(left, right - viewport.width);
            x = std::clamp(x, 0., list->scroll_max.x);
            if (x != before) {
                geometry_dirty = true;
                layout(this->viewport);
                notify();
            }
            list = find(id);
            if (!dirty && list && list->d.table && !list->hidden && list->enabled && in_focus_scope(*list) &&
                list->d.table->model.rows().index_of(cell.row) == row &&
                list->d.table->model.column_index(cell.column) == column)
                return find(table_cell_id(id, cell)) != nullptr;
        }
        return false;
    }
    bool table_select(std::string id, TableCell cell, bool extend = false, bool toggle = false) {
        auto* list = find(id);
        if (!list || !list->d.table || !interactive(*list) || !in_focus_scope(*list))
            return false;
        auto config = *list->d.table;
        auto row = config.model.rows().index_of(cell.row), col = config.model.column_index(cell.column);
        if (!row || !col || !config.model.rows().at(*row).enabled)
            return false;
        if (focused != id) {
            finish_editing();
            layout(viewport);
            list = find(id);
            if (!list || !list->d.table || !interactive(*list) || !in_focus_scope(*list))
                return false;
            config = *list->d.table;
            row = config.model.rows().index_of(cell.row);
            col = config.model.column_index(cell.column);
            if (!row || !col || !config.model.rows().at(*row).enabled)
                return false;
        }
        auto& state = tables.at(id);
        state.column = cell.column;
        lists.at(id).active = cell.row;
        if (config.options.selection_mode == TableSelectionMode::rows)
            return list_select(*list, cell.row, extend, toggle);
        auto value = config.selection.get();
        value.rows.clear();
        if (!config.options.multiple_selection) {
            extend = false;
            toggle = false;
        }
        if (extend) {
            auto ids = state.selected_cells;
            const auto ar = config.model.rows().index_of(state.anchor.row).value_or(*row);
            const auto ac = config.model.column_index(state.anchor.column).value_or(*col);
            if (!toggle) {
                value.cells.clear();
                ids.clear();
            }
            for (auto r = std::min(ar, *row); r <= std::max(ar, *row); ++r)
                if (config.model.rows().at(r).enabled)
                    for (auto c = std::min(ac, *col); c <= std::max(ac, *col); ++c) {
                        TableCell selected{config.model.rows().at(r).id, config.model.columns()[c].id};
                        if (ids.insert(table_cell_id(id, selected)).second)
                            value.cells.push_back(std::move(selected));
                    }
        } else if (toggle) {
            auto found = std::find(value.cells.begin(), value.cells.end(), cell);
            if (found == value.cells.end())
                value.cells.push_back(cell);
            else
                value.cells.erase(found);
            state.anchor = cell;
        } else {
            value.cells = {cell};
            state.anchor = cell;
        }
        focused = id;
        if (value != config.selection.get())
            config.selection.set(std::move(value));
        invalidate();
        layout(viewport);
        table_reveal(id, cell);
        notify();
        return true;
    }
    bool table_resize(std::string id, std::string column, double value) {
        auto* list = find(id);
        if (!list || !list->d.table || !interactive(*list) || !in_focus_scope(*list) || !std::isfinite(value))
            return false;
        const auto config = *list->d.table;
        const auto index = config.model.column_index(column);
        if (!index || !config.model.columns()[*index].resizable)
            return false;
        const auto& col = config.model.columns()[*index];
        tables.at(id).widths[column] = std::clamp(value, col.min_width, col.max_width);
        if (config.options.widths) {
            std::vector<TableColumnWidth> widths;
            for (const auto& c : config.model.columns())
                widths.push_back({c.id, tables.at(id).widths.at(c.id)});
            config.options.widths->set(std::move(widths));
        }
        geometry_dirty = true;
        layout(viewport);
        notify();
        return true;
    }
    bool table_sort(std::string id, std::string column, bool multiple = false) {
        auto* list = find(id);
        if (!list || !list->d.table || !interactive(*list) || !in_focus_scope(*list))
            return false;
        const auto config = *list->d.table;
        const auto index = config.model.column_index(column);
        if (!index || !config.model.columns()[*index].sortable)
            return false;
        auto values = tables.at(id).sort;
        auto old =
            std::find_if(values.begin(), values.end(), [&](const auto& d) { return d.column == column; });
        std::optional<SortOrder> order = old == values.end() ? std::optional{SortOrder::ascending}
                                         : old->order == SortOrder::ascending
                                             ? std::optional{SortOrder::descending}
                                             : std::nullopt;
        if (multiple)
            std::erase_if(values, [&](const auto& d) { return d.column == column; });
        else
            values.clear();
        if (order)
            values.push_back({column, *order});
        tables.at(id).sort = values;
        if (config.options.sort)
            config.options.sort->set(values);
        invalidate();
        if (config.options.on_sort)
            config.options.on_sort(values);
        layout(viewport);
        notify();
        return true;
    }
    bool table_cancel(std::string id) {
        auto state = tables.find(id);
        if (state == tables.end() || !state->second.edit)
            return false;
        const auto editor = table_cell_id(id, state->second.edit->cell) + "/key/editor";
        if (auto found = editors.find(editor); found != editors.end())
            found->second->buffer.unmark(false);
        state->second.edit.reset();
        if (focused == editor)
            focused = id;
        invalidate();
        return true;
    }
    bool table_commit(std::string id) {
        auto* list = find(id);
        if (!list || !list->d.table || !tables.at(id).edit)
            return false;
        const auto config = *list->d.table;
        const auto cell = tables.at(id).edit->cell;
        const auto editor = table_cell_id(id, cell) + "/key/editor";
        if (auto it = editors.find(editor); it != editors.end() && it->second->buffer.marked_range()) {
            unmark_text(true);
            return false;
        }
        const auto row = config.model.rows().index_of(cell.row), col = config.model.column_index(cell.column);
        if (!row || !col || !config.model.columns()[*col].commit) {
            table_cancel(id);
            layout(viewport);
            return false;
        }
        const auto value = tables.at(id).edit->draft.get();
        const auto base = tables.at(id).edit->base;
        const auto column = config.model.columns()[*col];
        auto error = config.model.text(*row, *col) != base
                         ? std::string("Cell changed externally; cancel and restart editing.")
                     : column.validate ? column.validate(cell.row, value)
                                       : std::string{};
        if (!tables.contains(id) || !tables.at(id).edit || tables.at(id).edit->cell != cell)
            return false;
        if (!error.empty()) {
            tables.at(id).edit->error = std::move(error);
            invalidate();
            layout(viewport);
            notify();
            return false;
        }
        tables.at(id).edit.reset();
        focused = id;
        invalidate();
        column.commit(cell.row, value);
        layout(viewport);
        notify();
        return true;
    }
    bool table_edit(std::string id, TableCell cell) {
        auto* list = find(id);
        if (!list || !list->d.table || !interactive(*list) || !in_focus_scope(*list))
            return false;
        auto config = *list->d.table;
        const auto row = config.model.rows().index_of(cell.row), col = config.model.column_index(cell.column);
        if (!row || !col || !config.model.rows().at(*row).enabled || !config.model.columns()[*col].commit)
            return false;
        if (tables.at(id).edit && tables.at(id).edit->cell == cell)
            return true;
        finish_editing();
        layout(viewport);
        list = find(id);
        if (!list || !list->d.table)
            return false;
        config = *list->d.table;
        const auto r = config.model.rows().index_of(cell.row), c = config.model.column_index(cell.column);
        if (!r || !c || !config.model.columns()[*c].commit)
            return false;
        if (!table_select(id, cell))
            return false;
        list = find(id);
        if (!list || !list->d.table)
            return false;
        config = *list->d.table;
        const auto current_row = config.model.rows().index_of(cell.row),
                   current_col = config.model.column_index(cell.column);
        if (!current_row || !current_col || !config.model.rows().at(*current_row).enabled ||
            !config.model.columns()[*current_col].commit || !table_reveal(id, cell))
            return false;
        list = find(id);
        if (!list || !list->d.table || !interactive(*list) || !in_focus_scope(*list))
            return false;
        config = *list->d.table;
        const auto revealed_row = config.model.rows().index_of(cell.row),
                   revealed_col = config.model.column_index(cell.column);
        if (!revealed_row || !revealed_col || !config.model.rows().at(*revealed_row).enabled ||
            !config.model.columns()[*revealed_col].commit)
            return false;
        auto value = config.model.text(*revealed_row, *revealed_col);
        tables.at(id).edit.emplace(TableState::Edit{cell, State<std::string>{value}, value, {}});
        tables.at(id).column = cell.column;
        lists.at(id).active = cell.row;
        const auto editor = table_cell_id(id, cell) + "/key/editor";
        focused = editor;
        invalidate();
        layout(viewport);
        table_reveal(id, cell);
        if (auto* field = find(editor)) {
            editors.at(editor)->buffer.command(TextCommand::select_all);
            reveal(*field);
            notify();
            return true;
        }
        table_cancel(id);
        return false;
    }
    bool table_key(std::string id, const KeyEvent& e) {
        auto* list = find(id);
        if (!list || !list->d.table)
            return false;
        const auto config = *list->d.table;
        auto& state = tables.at(id);
        if (state.edit && focused == table_cell_id(id, state.edit->cell) + "/key/editor") {
            const auto cell = state.edit->cell;
            if (e.key == Key::enter || e.key == Key::escape || e.key == Key::tab) {
                if (editors.at(focused)->buffer.marked_range()) {
                    unmark_text(e.key != Key::escape);
                    return true;
                }
                if (e.key == Key::escape) {
                    table_cancel(id);
                    layout(viewport);
                    notify();
                    return true;
                }
                const auto c = config.model.column_index(cell.column),
                           r = config.model.rows().index_of(cell.row);
                if (!c || !r)
                    return false;
                if (e.key == Key::enter && config.model.columns()[*c].multiline && !e.command && !e.control)
                    return false;
                if (!table_commit(id))
                    return true;
                if (e.key == Key::tab) {
                    auto* current = find(id);
                    if (!current || !current->d.table)
                        return true;
                    const auto model = current->d.table->model;
                    const auto base_row = model.rows().index_of(cell.row),
                               base_col = model.column_index(cell.column);
                    if (!base_row || !base_col)
                        return true;
                    const auto count = model.columns().size(), total = model.rows().size() * count;
                    auto index = *base_row * count + *base_col;
                    for (std::size_t step = 0; step < total; ++step) {
                        if (e.shift) {
                            if (index == 0)
                                return false;
                            --index;
                        } else {
                            if (index + 1 >= total)
                                return false;
                            ++index;
                        }
                        if (model.rows().at(index / count).enabled && model.columns()[index % count].commit) {
                            table_edit(
                                id, {model.rows().at(index / count).id, model.columns()[index % count].id});
                            return true;
                        }
                    }
                }
                return true;
            }
            return false;
        }
        if (focused != id)
            return false;
        const auto row = config.model.rows().index_of(lists.at(id).active),
                   col = config.model.column_index(state.column);
        if ((e.key == Key::enter || e.key == Key::f2) && row && col) {
            table_edit(id, {config.model.rows().at(*row).id, state.column});
            return true;
        }
        if ((e.key == Key::left || e.key == Key::right) && row && col) {
            const auto next = e.key == Key::left ? *col ? *col - 1 : 0
                                                 : std::min(*col + 1, config.model.columns().size() - 1);
            table_select(id, {config.model.rows().at(*row).id, config.model.columns()[next].id}, e.shift,
                         e.command);
            return true;
        }
        if (e.key == Key::a && e.command && config.options.selection_mode == TableSelectionMode::cells &&
            config.options.multiple_selection) {
            TableSelection selected;
            for (const auto r : config.model.rows().enabled_indices())
                for (const auto& c : config.model.columns())
                    selected.cells.push_back({config.model.rows().at(r).id, c.id});
            config.selection.set(std::move(selected));
            invalidate();
            return true;
        }
        return false;
    }
    std::optional<std::pair<std::string, TableCell>> logical_cell_id(const std::string& id) const {
        for (const auto& [owner, state] : tables) {
            const auto prefix = owner + "/option/";
            if (!id.starts_with(prefix))
                continue;
            const auto separator = id.find("/cell/", prefix.size());
            if (separator == std::string::npos || id.find('/', separator + 6) != std::string::npos)
                continue;
            const auto row = unpath_key(id.substr(prefix.size(), separator - prefix.size()));
            const auto col = unpath_key(id.substr(separator + 6));
            auto* list = find(owner);
            if (row && col && list && list->d.table && list->d.rows.index_of(*row) &&
                list->d.table->model.column_index(*col))
                return {{owner, TableCell{*row, *col}}};
        }
        return {};
    }
    std::optional<LogicalCell> logical_cell(std::string id, const TableCell& key) const {
        auto* list = find(id);
        if (!list || !list->d.table || list->hidden)
            return {};
        const auto& model = list->d.table->model;
        const auto r = model.rows().index_of(key.row), c = model.column_index(key.column);
        if (!r || !c)
            return {};
        const auto& state = tables.at(id);
        LogicalCell result;
        result.id = table_cell_id(id, key);
        result.table = id;
        result.group = id + "/option/" + path_key(key.row);
        result.cell = key;
        result.label = model.columns()[*c].title;
        result.value = model.text(*r, *c);
        result.row_index = *r;
        result.column_index = *c;
        result.enabled = list->enabled && model.rows().at(*r).enabled && in_focus_scope(*list);
        result.editable = bool(model.columns()[*c].commit);
        result.editing = state.edit && state.edit->cell == key;
        if (result.editing) {
            result.value = state.edit->draft.get();
            result.validation = state.edit->error;
        }
        result.selected = list->d.table->options.selection_mode == TableSelectionMode::cells
                              ? state.selected_cells.contains(result.id)
                              : lists.at(id).selected.contains(key.row);
        result.focused = active && list->d.table->options.selection_mode == TableSelectionMode::cells &&
                         !menu_popup() && !context_popup() && in_focus_scope(*list) && focused == id &&
                         lists.at(id).active == key.row && state.column == key.column;
        if (auto* node = find(result.id)) {
            result.bounds = node->bounds;
            result.clip = node->clip;
        }
        return result;
    }
    std::optional<LogicalColumn> logical_column(const std::string& id, std::size_t index) const {
        const auto* table = find(id);
        if (!table || !table->d.table || table->hidden || index >= table->d.table->model.columns().size())
            return {};
        const auto& col = table->d.table->model.columns()[index];
        const auto& state = tables.at(id);
        LogicalColumn result;
        result.id = id + "/column/" + path_key(col.id);
        result.table = id;
        result.key = col.id;
        result.header_id = id + "/header/" + path_key(col.id);
        result.label = col.title;
        result.index = index;
        result.count = table->d.table->model.columns().size();
        result.row_count = table->d.rows.size();
        result.enabled = table->enabled && in_focus_scope(*table);
        result.sortable = col.sortable;
        result.resizable = col.resizable;
        result.width = state.widths.at(col.id);
        result.min_width = col.min_width;
        result.max_width = col.max_width;
        for (std::size_t i = 0; i < state.sort.size(); ++i)
            if (state.sort[i].column == col.id) {
                result.sort_order = state.sort[i].order;
                result.sort_priority = i;
                break;
            }
        if (const auto* header = find(result.header_id)) {
            const auto body = inset(table->bounds, table->d.padding);
            result.bounds = Rect{header->bounds.x, body.y, header->bounds.width, body.height};
            result.clip = intersect(table->clip, body);
        }
        return result;
    }
    std::optional<LogicalColumn> logical_column_id(const std::string& id) const {
        for (const auto& [owner, state] : tables) {
            const auto prefix = owner + "/column/";
            if (!id.starts_with(prefix) || id.find('/', prefix.size()) != std::string::npos)
                continue;
            const auto key = unpath_key(id.substr(prefix.size()));
            const auto* table = find(owner);
            if (key && table && table->d.table)
                if (const auto index = table->d.table->model.column_index(*key))
                    return logical_column(owner, *index);
        }
        return {};
    }
    bool set_selection(std::string id, const std::vector<std::string>& ids) {
        auto* list = find(id);
        if (!list || list->d.kind != Kind::listbox || !interactive(*list) || !in_focus_scope(*list))
            return false;
        const auto d = list->d;
        std::unordered_set<std::string> seen;
        if (d.table && d.table->options.selection_mode == TableSelectionMode::cells) {
            TableSelection value;
            for (const auto& candidate : ids) {
                const auto cell = logical_cell_id(candidate);
                if (!cell || cell->first != id || !logical_cell(id, cell->second)->enabled)
                    return false;
                if (seen.insert(candidate).second)
                    value.cells.push_back(cell->second);
            }
            if (!d.table->options.multiple_selection && value.cells.size() > 1)
                return false;
            d.table->selection.set(std::move(value));
        } else {
            std::vector<std::string> keys;
            for (const auto& candidate : ids) {
                const auto row = logical_row(candidate);
                if (!row || row->first->id != id || !logical_item(*row->first, row->second)->enabled)
                    return false;
                if (seen.insert(candidate).second)
                    keys.push_back(d.rows.at(row->second).id);
            }
            if (d.write_selected)
                d.write_selected(std::move(keys));
            else if (d.write_selection) {
                if (keys.size() > 1)
                    return false;
                d.write_selection(keys.empty() ? std::string{} : keys.front());
            } else
                return false;
        }
        invalidate();
        layout(viewport);
        notify();
        return true;
    }
    bool set_selected(std::string id, bool selected) {
        std::string owner;
        std::vector<std::string> ids;
        bool multiple{}, was_selected{};
        if (const auto cell = logical_cell_id(id)) {
            owner = cell->first;
            const auto* table = find(owner);
            if (table->d.table->options.selection_mode == TableSelectionMode::rows)
                id = owner + "/option/" + path_key(cell->second.row);
            else {
                if (!logical_cell(owner, cell->second)->enabled)
                    return false;
                multiple = table->d.table->options.multiple_selection;
                for (const auto& value : tables.at(owner).source.cells)
                    if (logical_cell(owner, value))
                        ids.push_back(table_cell_id(owner, value));
            }
        }
        if (const auto row = logical_row(id)) {
            owner = row->first->id;
            const auto& d = row->first->d;
            if ((d.table && d.table->options.selection_mode == TableSelectionMode::cells) ||
                !logical_item(*row->first, row->second)->enabled)
                return false;
            multiple = bool(d.write_selected);
            const auto values =
                d.read_selected ? d.read_selected() : std::vector<std::string>{d.read_selection()};
            for (const auto& key : values)
                if (d.rows.index_of(key))
                    ids.push_back(owner + "/option/" + path_key(key));
        }
        if (owner.empty())
            return false;
        was_selected = std::find(ids.begin(), ids.end(), id) != ids.end();
        if (was_selected == selected)
            return true;
        if (selected) {
            if (!multiple)
                ids.clear();
            ids.push_back(id);
        } else
            std::erase(ids, id);
        return set_selection(owner, ids);
    }
    void forget_row(Node& row) {
        for (auto& child : row.children) forget_row(*child);
        for (auto& child : row.overlay_children) forget_row(*child);
        if (auto editor = editors.find(row.id); editor != editors.end()) {
            editor->second->buffer.unmark(false);
            editors.erase(editor);
        }
        nodes.erase(row.id); lists.erase(row.id); trees.erase(row.id); tables.erase(row.id); tabs.erase(row.id); combos.erase(row.id);
        scroll.erase(row.id); layout_overrides.erase(row.id);
        if (hovered == row.id) hovered.clear();
        if (pressed == row.id) pressed.clear();
        if (text_drag == row.id) text_drag.clear();
        if (dragging == row.id) dragging.clear();
    }
    void realize_rows(Node& list, double viewport_height) {
        auto& state = lists.at(list.id);
        state.viewport_height = viewport_height;
        const auto& model = list.d.rows;
        const auto height = list.d.list_options.row_height;
        state.window = row_window(model.size(), scroll[list.id].y, list.hidden ? 0 : viewport_height,
                                  height, list.d.list_options.overscan_rows);
        state.pinned.clear();
        if (active && list.enabled && !list.hidden && (model.has_content() || list.d.table)) {
            const auto prefix = list.id + "/option/";
            for (const auto& [id, editor] : editors) {
                if ((focused != id && !focused.starts_with(id + "/")) || !id.starts_with(prefix)) continue;
                const auto rest = id.substr(prefix.size());
                const auto encoded = rest.substr(0, rest.find('/'));
                for (const auto& row : list.children)
                    if (row->id == prefix + encoded) { state.pinned = row->selection; break; }
                if (state.pinned.empty()) {
                    // On a description rebuild, the old row nodes are retained
                    // by the previous tree while its text buffers remain keyed.
                    auto key = unpath_key(encoded);
                    if (key && model.index_of(*key)) state.pinned = *key;
                }
            }
        }
        std::vector<std::size_t> wanted;
        for (auto i = state.window.mounted_begin; i < state.window.mounted_end; ++i) wanted.push_back(i);
        if (!state.pinned.empty()) if (auto index = model.index_of(state.pinned)) wanted.push_back(*index);
        std::sort(wanted.begin(), wanted.end()); wanted.erase(std::unique(wanted.begin(), wanted.end()), wanted.end());
        std::unordered_map<std::size_t, Node*> existing;
        for (const auto& row : list.children) if (row->d.kind == Kind::list_option) existing.emplace(row->row_index, row.get());
        std::vector<std::unique_ptr<Node>> added;
        const auto prior_nodes = nodes;
        const auto prior_requests = requested_overlays.size();
        try {
            for (const auto index : wanted) {
                if (existing.contains(index)) continue;
                const auto& option = model.at(index);
                const auto row_id = list.id + "/option/" + path_key(option.id);
                auto row_observer = std::make_shared<detail::Observer>();
                std::weak_ptr<Impl> weak = shared_from_this();
                row_observer->changed = [weak] { if (auto host = weak.lock()) host->invalidate(); };
                const auto previous = detail::collecting;
                const bool was_building = detail::building;
                detail::collecting = row_observer; detail::building = true;
                std::unique_ptr<Node> row;
                try {
                    if (list.d.table) row = mount_table_row(list,index);
                    else {
                        auto content = model.has_content() ? model.build(index) : View{};
                        row = mount(Layer{std::move(content)}.enabled(option.enabled)
                                    .padding(model.has_content() ? Insets::symmetric(0, 8) : Insets{}), row_id, &list);
                    }
                } catch (...) { detail::collecting = previous; detail::building = was_building; throw; }
                detail::collecting = previous; detail::building = was_building;
                row->row_observer = std::move(row_observer); row->row_snapshot = model;
                row->d.kind = Kind::list_option; row->d.text = option.label; row->selection = option.id;
                row->row_index = index; row->custom_row = model.has_content() || list.d.table.has_value(); row->group = list.id;
                row->number = state.selected.contains(option.id); row->layout_editable = false;
                if (list.d.tree) {
                    row->tree_item = trees.at(list.id).flat.at(index);
                    row->expanded = row->tree_item->expanded;
                    row->d.accessible = row->tree_item->accessible_label;
                }
                if (!list.combo_owner.empty()) {
                    std::function<void(Node&)> tag = [&](Node& node) {
                        node.combo_owner = list.combo_owner;
                        for (auto& child : node.children) tag(*child);
                    };
                    tag(*row);
                }
                added.push_back(std::move(row));
            }
        } catch (...) {
            nodes = prior_nodes; requested_overlays.resize(prior_requests);
            std::erase_if(editors, [&](const auto& e) { return !nodes.contains(e.first); });
            std::erase_if(lists, [&](const auto& e) { return !nodes.contains(e.first); });
            std::erase_if(trees, [&](const auto& e) { return !nodes.contains(e.first); });
            std::erase_if(tables, [&](const auto& e) { return !nodes.contains(e.first); });
            std::erase_if(tabs, [&](const auto& e) { return !nodes.contains(e.first); });
            std::erase_if(combos, [&](const auto& e) { return !nodes.contains(e.first); });
            throw;
        }
        bool changed = !added.empty();
        std::vector<std::unique_ptr<Node>> next;
        for (auto& row : list.children) {
            if (row->d.kind != Kind::list_option || std::binary_search(wanted.begin(), wanted.end(), row->row_index)) next.push_back(std::move(row));
            else {
                if (focused == row->id || focused.starts_with(row->id + "/")) focused = list.id;
                forget_row(*row); changed = true;
            }
        }
        for (auto& row : added) next.push_back(std::move(row));
        std::sort(next.begin(), next.end(), [](const auto& a, const auto& b) {
            if(a->d.kind != b->d.kind)return a->d.kind==Kind::table_header;
            return a->row_index < b->row_index;
        });
        list.children = std::move(next); list.paint_children.clear();
        for (const auto& row : list.children) { row->number = state.selected.contains(row->selection); list.paint_children.push_back(row.get()); }
        if (changed) ++generation;
    }
    void update_combo_node(Node& n) {
        auto& state = combos[n.id];
        const auto query = n.d.read_text();
        if (!state.initialized || (!state.writing &&
            (state.source_query != query || state.source_selection != n.selection))) {
            state.base_query = query; state.base_selection = n.selection;
            state.session = false; state.open = false; state.validation.clear();
        }
        state.initialized = true;
        state.source_query = query; state.source_selection = n.selection;
        state.read_query = n.d.read_text; state.write_query = n.d.write_text;
        state.read_selection = n.d.read_selection; state.write_selection = n.d.write_selection;
        state.mode = n.d.combo->mode;
        const auto previous_matches = state.matches;
        state.matches.clear(); state.selected_label.reset();
        const auto key = detail::search_key(query);
        for (const auto& option : n.d.options) {
            if (option.id == n.selection) state.selected_label = option.label;
            const auto label = detail::search_key(option.label);
            if (!n.d.combo->loading && (n.d.combo->filter == ComboBoxFilter::none ||
                (n.d.combo->filter == ComboBoxFilter::prefix ? label.starts_with(key) : label.find(key) != std::string::npos)))
                state.matches.push_back(option);
        }
        if (state.matches != previous_matches) state.reveal = true;
        const auto highlighted = std::find_if(state.matches.begin(), state.matches.end(), [&](const auto& item) {
            return item.enabled && item.id == state.highlighted;
        });
        if (highlighted == state.matches.end()) {
            state.highlighted.clear();
            if (n.d.combo->auto_highlight)
                for (const auto& item : state.matches) if (item.enabled) { state.highlighted = item.id; break; }
        }
    }
    std::unique_ptr<Node> mount(const View& view, const std::string& id, const Node* parent = nullptr,
                                const std::string& scope = {}) {
        auto d = view.description();
        if (!d.visible || d.kind == Kind::empty)
            return {};
        if (auto o = layout_overrides.find(id); o != layout_overrides.end()) {
            const auto& v = o->second;
            if (v.width)
                d.width = *v.width;
            if (v.height)
                d.height = *v.height;
            if (v.basis)
                d.basis = *v.basis;
            if (v.padding)
                d.padding = *v.padding;
            if (v.margin)
                d.margin = *v.margin;
            if (v.gap)
                d.gap = *v.gap;
            if (v.grow)
                d.grow = *v.grow;
            if (v.shrink)
                d.shrink = *v.shrink;
            if (v.align)
                d.align = *v.align;
            if (v.self)
                d.self = *v.self;
            if (v.justify)
                d.justify = *v.justify;
            if (v.wrap)
                d.wrap = *v.wrap;
            if (v.direction && (d.kind == Kind::row || d.kind == Kind::column))
                d.kind = *v.direction == Orientation::horizontal ? Kind::row : Kind::column;
            if (v.position)
                d.position = *v.position;
            if (v.left)
                d.left = *v.left;
            if (v.top)
                d.top = *v.top;
            if (v.right)
                d.right = *v.right;
            if (v.bottom)
                d.bottom = *v.bottom;
            if (v.z_index)
                d.z_index = *v.z_index;
            if (v.hidden)
                d.hidden = *v.hidden;
        }
        if (d.tree) {
            const auto expanded = d.tree->expanded.get();
            const bool hidden = d.hidden || (parent && parent->hidden);
            if (hidden) trees.try_emplace(id);
            const auto found = trees.find(id);
            if (!hidden && (found == trees.end() || !found->second.prepared || found->second.model != d.tree->model || found->second.expanded != expanded)) {
                TreeState next;
                next.prepared = true;
                next.model = d.tree->model; next.expanded = expanded;
                next.flat = next.model.flatten(expanded);
                std::vector<Option> options;
                options.reserve(next.flat.size());
                for (const auto& item : next.flat) {
                    if (!item.parent.empty()) next.children[item.parent].push_back(options.size());
                    options.push_back({item.id, item.label, item.enabled});
                    next.expanded_count += item.expanded; next.loading_count += item.loading;
                }
                next.rows = RowModel(std::move(options));
                trees.insert_or_assign(id, std::move(next));
            }
            d.rows = trees.at(id).rows;
        }
        if (d.table) prepare_table(d,id);
        const auto datetime_owner=d.datetime?id:(parent?parent->datetime_owner:std::string{});
        const bool datetime_component=!datetime_owner.empty()&&d.numeric&&
            (id==datetime_component_id(datetime_owner,true)||id==datetime_component_id(datetime_owner,false));
        if(datetime_component)d.numeric->format_id+=":transaction:"+std::to_string(datetimes.at(datetime_owner).revision);
        auto n = std::make_unique<Node>();
        n->datetime_owner=datetime_owner;n->datetime_component=datetime_component;
        n->color_owner=d.color?id:(parent?parent->color_owner:std::string{});
        if(d.numeric&&d.color_component&&!n->color_owner.empty())d.numeric->format_id+=color_format_tag(n->color_owner);
        n->d = d;
        n->id = id;
        n->overlay_scope = scope.empty() && parent ? parent->overlay_scope : scope;
        n->layout_editable = true;
        n->enabled = d.enabled && (!parent || parent->enabled) && (!d.alert || d.alert->pending());
        n->hidden = d.hidden || (parent && parent->hidden);
        if(d.datetime)prepare_datetime(*n,d);
        if(d.color)prepare_color(*n,d);
        if(d.table&&(n->hidden||!n->enabled)&&tables.at(id).edit){
            if(auto edit=editors.find(focused);edit!=editors.end())edit->second->buffer.unmark(false);
            tables.at(id).edit.reset();if(focused.starts_with(id+"/option/"))focused=id;
        }
        n->font = d.font_size.value_or(parent ? parent->font : 14);
        n->color = d.foreground.value_or(parent ? parent->color : theme.text);
        n->accent = d.accent.value_or(parent ? parent->accent : theme.primary);
        switch (d.style) {
        case LabelStyle::heading:
            n->font = d.font_size.value_or(24);
            break;
        case LabelStyle::section:
            n->font = d.font_size.value_or(18);
            break;
        case LabelStyle::secondary:
            n->font = d.font_size.value_or(12);
            n->color = d.foreground.value_or(theme.secondary);
            break;
        case LabelStyle::micro:
            n->font = d.font_size.value_or(11);
            n->color = d.foreground.value_or(theme.secondary);
            n->tracking = .5;
            break;
        case LabelStyle::error:
            n->color = d.foreground.value_or(theme.error);
            break;
        case LabelStyle::status:
            n->color = d.foreground.value_or(theme.cyan);
            break;
        default:
            break;
        }
        if (!n->enabled)
            n->color = theme.disabled;
        n->line_height = d.style == LabelStyle::micro ? 14 : std::max(18., n->font * 1.3);
        if (d.uppercase)
            n->d.text = capitalized(d.text);
        collect_number(*n);
        if (d.context_menu) n->context_menu = resolve_menu(d.context_menu());
        if (d.kind == Kind::text_field) {
            auto value = d.numeric ? n->numeric_text : d.read_text();
            auto& editor = editors[id];
            const bool mode_changed = editor && (editor->password_mode != d.password.has_value() ||
                                                 editor->search_mode != d.search.has_value() ||
                                                 editor->combo_mode != d.combo.has_value());
            if (!editor) {
                editor = std::make_unique<Editing>();
                editor->buffer.set_text(value);
            } else if (mode_changed || (d.numeric ?
                (!editor->numeric_source || editor->numeric_source != n->numeric_value ||
                 editor->numeric_locale != d.numeric->locale || editor->numeric_format_id != d.numeric->format_id || editor->numeric_digits != d.numeric->significant_digits ||
                 editor->numeric_grouping != d.numeric->grouping) :
                (editor->numeric_source.has_value() || editor->buffer.committed_text() != value))) {
                auto prior = editor->buffer.selection();
                editor->buffer.set_text(value);
                editor->buffer.select(prior);
                if (d.password)
                    editor->revealed = false;
                editor->reset_blink = true;
                editor->reveal_caret = true;
                editor->upstream = false;
            }
            if (mode_changed) {
                editor->revealed = false;
                editor->pending_search.reset();
            }
            editor->password_mode = d.password.has_value();
            editor->search_mode = d.search.has_value();
            editor->combo_mode = d.combo.has_value();
            if (d.password && (!active || n->hidden || !n->enabled || d.password->read_only)) {
                editor->revealed = false;
                editor->buffer.unmark(false);
                editor->buffer.clear_history();
            }
            if (d.search) {
                if (editor->search_source != value || editor->search_delay != d.search->debounce)
                    editor->pending_search.reset();
                editor->search_source = value;
                editor->search_delay = d.search->debounce;
                if (!active || n->hidden || !n->enabled || d.search->read_only || !d.search->on_search)
                    editor->pending_search.reset();
            } else
                editor->pending_search.reset();
            if (d.numeric) {
                editor->numeric_source = n->numeric_value;
                editor->numeric_base = value;
                editor->numeric_locale = d.numeric->locale;
                editor->numeric_format_id = d.numeric->format_id;
                editor->numeric_digits = d.numeric->significant_digits;
                editor->numeric_grouping = d.numeric->grouping;
                refresh_numeric(*n, false);
            } else {
                editor->numeric_source.reset();
                editor->validation = {};
            }
            if (d.text_options.read_only && editor->buffer.marked_range())
                editor->buffer.unmark(false);
        }
        if (d.combo) update_combo_node(*n);
        nodes.emplace(id, n.get());
        if(d.date)prepare_calendar(*n);
        if(d.time){
            for(int direction:{1,-1}){
                auto child=mount(Button(direction>0?"▴":"▾",{}).font_size(11),id+(direction>0?"/increase":"/decrease"),n.get());
                child->d.kind=Kind::step_button;child->d.accessible=direction>0?"Increase time component":"Decrease time component";
                child->number=direction;child->group=id;child->layout_editable=false;
                const auto next=detail::step_number(*n->numeric_value,direction,*d.numeric,editors.at(id)->buffer.selection().caret);
                child->enabled=child->enabled&&!d.time->read_only&&next&&*next!=*n->numeric_value;
                n->children.push_back(std::move(child));
            }
        }
        if (d.kind == Kind::text_field && ((d.search && d.search->clear_button) || (d.password && d.password->allow_reveal) || d.combo)) {
            const bool reveal = d.password.has_value();
            auto child = mount(Button(reveal ? (editors.at(id)->revealed ? "Hide" : "Show") : "", {}),
                               id + "/action", n.get());
            child->d.kind = Kind::field_action;
            child->d.reveal_action = reveal;
            child->d.combo_action = d.combo.has_value();
            child->d.accessible = reveal ? (editors.at(id)->revealed ? "Hide password" : "Show password") : "Clear search";
            if (d.combo) child->d.accessible = combos.at(id).open ? "Hide suggestions" : "Show suggestions";
            child->hidden = child->hidden || (!reveal && !d.combo && editors.at(id)->buffer.text().empty());
            child->enabled = child->enabled && !d.text_options.read_only;
            child->group = id;
            child->number = reveal && editors.at(id)->revealed;
            child->layout_editable = false;
            n->children.push_back(std::move(child));
        }
        if (d.kind == Kind::stepper) {
            for (int direction : {-1, 1}) {
                auto child = mount(Button(direction < 0 ? "−" : "+", {}),
                                   id + (direction < 0 ? "/decrease" : "/increase"), n.get());
                child->d.kind = Kind::step_button;
                child->d.accessible = (direction < 0 ? "Decrease " : "Increase ") +
                                     (d.accessible.empty() ? std::string("value") : d.accessible);
                child->number = direction;
                child->group = id;
                child->layout_editable = false;
                const auto next = detail::step_number(*n->numeric_value, direction, *d.numeric);
                child->enabled = child->enabled && !d.numeric->read_only && next && next != n->numeric_value;
                n->children.push_back(std::move(child));
            }
        }
        if (d.kind == Kind::progress && d.indeterminate && !n->hidden)
            animating = true;
        if (d.kind == Kind::tab_view) {
            auto& state = tabs[id];
            n->requested_selection = n->selection;
            const auto selected = std::find_if(d.tabs.begin(), d.tabs.end(), [&](const auto& tab) {
                return tab.id == n->selection && tab.enabled;
            });
            if (selected == d.tabs.end()) {
                n->selection.clear();
                for (const auto& tab : d.tabs)
                    if (tab.enabled) {
                        n->selection = tab.id;
                        break;
                    }
            }
            if (state.active != n->selection) {
                state.active = n->selection;
                state.reveal = true;
            }
            std::erase_if(state.pages, [&](const auto& entry) {
                return std::none_of(d.tabs.begin(), d.tabs.end(), [&](const auto& tab) { return tab.id == entry.first; });
            });
            auto bar = mount(FlexRow{}, id + "/tabs", n.get());
            bar->d.kind = Kind::tab_bar;
            bar->group = id;
            bar->layout_editable = false;
            for (const auto& tab : d.tabs) {
                auto button = mount(Button(tab.label, {}).enabled(tab.enabled).density(d.density),
                                    id + "/tab/" + path_key(tab.id), bar.get());
                button->d.kind = Kind::tab_option;
                button->d.shrink = 0;
                button->layout_editable = false;
                button->group = id;
                button->selection = tab.id;
                button->number = tab.id == n->selection ? 1 : 0;
                bar->paint_children.push_back(button.get());
                bar->children.push_back(std::move(button));
            }
            n->children.push_back(std::move(bar));
            for (const auto& tab : d.tabs) {
                if (!n->hidden && tab.id == n->selection)
                    state.pages.insert_or_assign(tab.id, tab.content());
                const auto page = state.pages.find(tab.id);
                if (page == state.pages.end())
                    continue;
                auto wrapper = Layer{page->second}.key(tab.id).hidden(tab.id != n->selection);
                auto body = mount(wrapper, id + "/page/" + path_key(tab.id), n.get());
                body->d.kind = Kind::tab_page;
                body->d.accessible = tab.label;
                body->layout_editable = false;
                body->group = id;
                n->children.push_back(std::move(body));
            }
        } else if (d.kind == Kind::listbox) {
            if(d.table)n->children.push_back(mount_table_header(*n));
            auto& state = lists[id];
            const bool selection_changed = state.source_selection != n->selection || state.active.empty();
            state.selected.clear();
            if (d.read_selected) state.selected.insert(n->selected_ids.begin(), n->selected_ids.end());
            else if (!n->selection.empty()) state.selected.insert(n->selection);
            const auto valid = [&](const std::string& value) {
                const auto index = d.rows.index_of(value);
                return index && d.rows.at(*index).enabled;
            };
            if (d.read_selection && valid(n->selection) &&
                selection_changed) state.active = n->selection;
            state.source_selection = n->selection;
            if (!valid(state.active) && d.tree) {
                auto ancestor = d.tree->model.item(state.active);
                while (ancestor && !ancestor->parent.empty()) {
                    if (valid(ancestor->parent)) { state.active = ancestor->parent; break; }
                    ancestor = d.tree->model.item(ancestor->parent);
                }
            }
            if (!valid(state.active)) {
                state.active.clear();
                if (const auto enabled = d.rows.enabled_indices(); !enabled.empty()) state.active = d.rows.at(enabled.front()).id;
                for (const auto& selected : n->selected_ids) if (valid(selected)) { state.active = selected; break; }
            }
            if (!valid(state.anchor)) state.anchor = state.active;
            const auto estimate = state.viewport_height > 0 ? state.viewport_height :
                d.list_options.visible_rows * d.list_options.row_height;
            if (d.tree && !n->hidden && selection_changed && d.read_selection && valid(n->selection)) {
                const auto index = *d.rows.index_of(n->selection);
                const auto top = index * d.list_options.row_height;
                auto& offset = scroll[id].y;
                if (top < offset) offset = top;
                else if (top + d.list_options.row_height > offset + estimate) offset = top + d.list_options.row_height - estimate;
            }
            realize_rows(*n, estimate);
        } else if (d.kind == Kind::segments || d.kind == Kind::radio) {
            if (d.kind == Kind::segments && !d.options.empty() &&
                std::none_of(d.options.begin(), d.options.end(),
                             [&](auto& o) { return o.id == n->selection; }))
                throw std::invalid_argument("SegmentedControl requires a valid selected ID");
            for (auto& o : d.options) {
                auto c = std::make_unique<Node>();
                c->d.kind = d.kind == Kind::segments ? Kind::segment_option : Kind::radio_option;
                c->d.text = o.label;
                c->id = id + "/option/" + path_key(o.id);
                c->group = id;
                c->selection = o.id;
                c->number = o.id == n->selection ? 1 : 0;
                c->enabled = n->enabled && o.enabled;
                c->hidden = n->hidden;
                c->font = n->font;
                c->line_height = n->line_height;
                c->color = c->enabled ? n->color : theme.disabled;
                c->accent = n->accent;
                c->d.density = d.density;
                c->d.click = [fn = d.write_selection, value = o.id] { fn(value); };
                nodes.emplace(c->id, c.get());
                n->children.push_back(std::move(c));
            }
            n->d.gap = d.kind == Kind::radio ? 4 : 0;
        } else if (n->expanded) {
            std::function<void(const std::vector<View>&, const std::string&, bool, bool)> append;
            append = [&](const auto& views, const auto& prefix, bool hidden, bool enabled) {
                std::unordered_set<std::string> keys;
                std::size_t index{};
                for (const auto& v : views) {
                    const auto& desc = v.description();
                    const auto key = desc.key;
                    if (!key.empty() && !keys.insert(key).second)
                        throw std::invalid_argument("Duplicate sibling SCARS key: " + key);
                    auto child_id =
                        prefix + "/" + (key.empty() ? "@" + std::to_string(index) : "key/" + path_key(key));
                    ++index;
                    if (desc.kind == Kind::fragment) {
                        if (desc.visible)
                            append(desc.children, child_id, hidden || desc.hidden, enabled && desc.enabled);
                    } else {
                        auto view = hidden ? v.hidden() : v;
                        if (!enabled)
                            view = view.enabled(false);
                        auto child = mount(view, child_id, n.get());
                        if (child)
                            n->children.push_back(std::move(child));
                    }
                }
            };
            if (d.kind == Kind::split_view) {
                auto panes = d.children;
                if (n->collapsed == SplitPane::first)
                    panes[0] = panes[0].hidden();
                if (n->collapsed == SplitPane::second)
                    panes[1] = panes[1].hidden();
                append(panes, id, false, true);
            } else if (d.alert) {
                append({d.alert->body()}, id, false, true);
            } else
                append(d.children, id, false, true);
        }
        if (d.kind == Kind::split_view) {
            auto divider = std::make_unique<Node>();
            divider->d.kind = Kind::split_divider;
            divider->d.text = d.accessible.empty() ? "Resize panes" : d.accessible + " divider";
            divider->d.orientation = d.orientation;
            divider->id = id + "/divider";
            divider->group = id;
            divider->enabled = n->enabled;
            divider->hidden = n->hidden;
            divider->color = n->color;
            divider->accent = n->accent;
            nodes.emplace(divider->id, divider.get());
            n->children.push_back(std::move(divider));
        }
        if (d.kind == Kind::split) {
            auto arrow = std::make_unique<Node>();
            arrow->d.kind = Kind::split_arrow;
            arrow->d.text = "Choose action";
            arrow->id = id + "/popup";
            arrow->group = id;
            arrow->enabled = n->enabled;
            arrow->hidden = n->hidden;
            arrow->font = n->font;
            arrow->line_height = n->line_height;
            arrow->color = n->color;
            arrow->accent = n->accent;
            nodes.emplace(arrow->id, arrow.get());
            n->children.push_back(std::move(arrow));
        }
        if (d.kind == Kind::selection || d.kind == Kind::split) {
            for (auto& o : d.options)
                if (o.id == n->selection)
                    n->d.text = o.label;
            if (d.kind == Kind::split && n->d.text.empty())
                throw std::invalid_argument("SplitActionButton requires a valid action ID");
        }
        n->paint_children.clear();
        for (const auto& child : n->children) {
            child->overlay_scope = n->overlay_scope;
            n->paint_children.push_back(child.get());
        }
        std::stable_sort(n->paint_children.begin(), n->paint_children.end(),
                         [](const auto* a, const auto* b) { return a->d.z_index < b->d.z_index; });
        // Overlay children share the state collector and node registry, but do
        // not participate in their anchor's measurement or clipping.
        if (n->enabled && !n->hidden && active && d.popover && d.popover->presented.get()) {
            const auto overlay_id = id + "/popover";
            Overlay overlay;
            overlay.kind = OverlayKind::popover;
            overlay.owner = id;
            overlay.root_id = overlay_id;
            overlay.popover = d.popover;
            overlay.restore_focus = focused;
            requested_overlays.push_back(std::move(overlay));
            auto content = Panel(ScrollView(d.popover->build()).grow()).padding(12)
                .width(d.popover->options.width).max_height(d.popover->options.max_height);
            if (auto child = mount(content, overlay_id, n.get(), overlay_id)) {
                child->popover_root = true;
                child->layout_editable = false;
                n->overlay_children.push_back(std::move(child));
            }
        }
        if (d.combo && combos.at(id).open && n->enabled && !n->hidden && !d.combo->read_only && active &&
            !editors.at(id)->buffer.marked_range()) {
            auto& state = combos.at(id);
            const auto overlay_id = id + "/suggestions";
            Overlay overlay; overlay.kind = OverlayKind::combo; overlay.owner = id; overlay.root_id = overlay_id;
            requested_overlays.push_back(std::move(overlay));
            View body;
            if (d.combo->loading || state.matches.empty())
                body = Panel(Label(d.combo->loading ? d.combo->loading_text : d.combo->no_results).text_wrap(TextWrap::wrap))
                    .padding(10).accessible_label(d.combo->loading ? d.combo->loading_text : d.combo->no_results);
            else body = ListBox(Binding<std::string>{[this, id] { return combos.at(id).highlighted; },
                                      [this, id](std::string value) { accept_combo(id, std::move(value)); }},
                                state.matches, {.row_height = d.combo->row_height})
                    .height(std::min(d.combo->max_height, d.combo->row_height * state.matches.size() + 8))
                    .accessible_label("Suggestions");
            if (auto child = mount(body, overlay_id, n.get(), overlay_id)) {
                std::function<void(Node&)> tag = [&](Node& node) {
                    node.combo_owner = id; node.layout_editable = false;
                    for (auto& descendant : node.children) tag(*descendant);
                };
                tag(*child);
                n->overlay_children.push_back(std::move(child));
            }
        }
        if (tooltip && tooltip->shown && tooltip->owner == id && d.tooltip &&
            d.tooltip->text == tooltip->text && n->enabled && !n->hidden && active) {
            const auto overlay_id = id + "/tooltip";
            Overlay overlay;
            overlay.kind = OverlayKind::tooltip;
            overlay.owner = id;
            overlay.root_id = overlay_id;
            requested_overlays.push_back(std::move(overlay));
            auto content = Panel(Label(tooltip->text).font_size(12).text_wrap(TextWrap::wrap))
                .padding(8).max_width(d.tooltip->options.max_width).foreground(theme.text);
            if (auto child = mount(content, overlay_id, n.get(), overlay_id)) {
                child->tooltip_root = true;
                child->layout_editable = false;
                n->overlay_children.push_back(std::move(child));
            }
        }
        return n;
    }
    Node* find(const std::string& id) const {
        auto it = nodes.find(id);
        return it == nodes.end() ? nullptr : it->second;
    }
    void reconcile_overlays() {
        std::vector<Overlay> next;
        for (const auto& old : overlays) {
            auto* anchor = find(old.owner);
            if (old.context) {
                const auto* entries = anchor && anchor->enabled && !anchor->hidden && active ? &anchor->context_menu : nullptr;
                for (const auto& part : old.context->path) {
                    if (!entries) break;
                    auto item = std::find_if(entries->begin(), entries->end(), [&](const auto& e) {
                        return e.item.id == part && menu_selectable(e) && e.item.kind == MenuItemKind::submenu;
                    });
                    entries = item == entries->end() ? nullptr : &item->children;
                }
                if (entries && !entries->empty()) {
                    auto menu = old;
                    std::string highlight = menu.highlighted >= 0 && menu.highlighted < static_cast<int>(menu.context->entries.size())
                        ? menu.context->entries[menu.highlighted].item.id : std::string{};
                    menu.context->entries = *entries;
                    menu.highlighted = -1;
                    for (std::size_t i = 0; i < entries->size(); ++i)
                        if (menu_selectable((*entries)[i]) && (menu.highlighted < 0 || (*entries)[i].item.id == highlight)) {
                            menu.highlighted = static_cast<int>(i);
                            if ((*entries)[i].item.id == highlight) break;
                        }
                    next.push_back(std::move(menu));
                }
                continue;
            }
            if (old.kind == OverlayKind::menu) {
                if (anchor && anchor->enabled && !anchor->hidden && active) {
                    auto menu = old;
                    menu.options = anchor->d.options;
                    if (menu.highlighted >= static_cast<int>(menu.options.size())) menu.highlighted = -1;
                    next.push_back(std::move(menu));
                }
                continue;
            }
            auto request = std::find_if(requested_overlays.begin(), requested_overlays.end(),
                [&](const auto& value) { return value.root_id == old.root_id; });
            if (request != requested_overlays.end()) {
                request->initialized = old.initialized;
                request->restore_focus = old.restore_focus;
                request->restore_on_close = old.restore_on_close;
                next.push_back(*request);
                requested_overlays.erase(request);
            } else if (find(old.root_id)) {
                next.push_back(old);
            } else if (old.popover) {
                closed_overlays.push_back({old, true});
            }
        }
        if (std::any_of(requested_overlays.begin(), requested_overlays.end(), [](const auto& o) { return o.popover.has_value(); }))
            std::erase_if(next, [](const auto& o) { return o.kind == OverlayKind::menu || o.context.has_value(); });
        for (auto& request : requested_overlays) if (find(request.root_id)) next.push_back(std::move(request));
        overlays = std::move(next);
        requested_overlays.clear();
    }
    void rebuild() {
        auto prior = detail::collecting;
        const bool was = detail::building;
        detail::collecting = observer;
        ++observer->epoch;
        detail::building = true;
        try {
            // Retain the old tree until a complete description has been constructed.
            auto view = builder();
            auto old_nodes = std::move(nodes);
            nodes.clear();
            animating = false;
            requested_overlays.clear();
            try {
                auto replacement = mount(view, "root");
                reconcile_overlays();
                root = std::move(replacement);
            } catch (...) {
                nodes = std::move(old_nodes);
                throw;
            }
            if (!find(focused) || !interactive(*find(focused))) {
                if (auto editor = editors.find(focused); editor != editors.end())
                    editor->second->buffer.unmark(false);
                // A changed tab selection can hide or remove the focused page. Move
                // focus to that container's active tab without writing model state.
                const bool dismissed_overlay = std::any_of(closed_overlays.begin(), closed_overlays.end(), [&](const auto& entry) {
                    return focused == entry.overlay.root_id || focused.starts_with(entry.overlay.root_id + "/");
                });
                Node* container = nullptr;
                for (const auto& [id, node] : nodes)
                    if (node->d.kind == Kind::tab_view && node->enabled && !node->hidden &&
                        focused.starts_with(id + "/") &&
                        !(dismissed_overlay && focused.starts_with(id + "/page/" + path_key(node->selection) + "/")) &&
                        (!container || id.size() > container->id.size()))
                        container = node;
                focused = container && !container->selection.empty()
                              ? container->id + "/tab/" + path_key(container->selection) : std::string{};
            }
            if (!find(pressed) || !interactive(*find(pressed)))
                pressed.clear();
            if (!find(dragging) || !interactive(*find(dragging)))
                dragging.clear();
            if (hold && (!active || focused != hold->owner || !find(hold->button) || !interactive(*find(hold->button)))) {
                hold.reset();
                pressed.clear();
            }
            if (auto* selected = find(inspected); selected && selected->hidden) {
                inspection_flash_pending = false;
                inspection_flash_start.reset();
            }
            std::erase_if(scroll, [&](const auto& p) {
                const auto* node = find(p.first);
                return !node || (node->d.kind != Kind::scroll && node->d.kind != Kind::listbox);
            });
            std::erase_if(lists,
                          [&](auto& p) { return !find(p.first) || find(p.first)->d.kind != Kind::listbox; });
            std::erase_if(trees, [&](auto& p) { return !find(p.first) || !find(p.first)->d.tree; });
            std::erase_if(colors,[&](auto& entry){
                if(find(entry.first)&&find(entry.first)->d.color)return false;
                retire_color(entry.second);return true;
            });
            std::erase_if(datetimes,[&](const auto& p){return !find(p.first)||!find(p.first)->d.datetime;});
            std::erase_if(calendars,[&](const auto& p){return !find(p.first)||!find(p.first)->d.date;});
            std::erase_if(tables, [&](auto& p) { return !find(p.first) || !find(p.first)->d.table; });
            std::erase_if(tabs, [&](const auto& p) { return !find(p.first) || find(p.first)->d.kind != Kind::tab_view; });
            std::erase_if(editors, [&](auto& p) {
                return !find(p.first) || find(p.first)->d.kind != Kind::text_field;
            });
            std::erase_if(combos, [&](auto& entry) {
                auto& state = entry.second;
                const auto* node = find(entry.first);
                const bool removed = !node || !node->d.combo;
                if (removed || !active || node->hidden || !node->enabled || node->d.combo->read_only) {
                    if (state.session) {
                        if (state.mode == ComboBoxMode::selection)
                            combo_cleanup.push_back({state.read_query, state.write_query, state.base_query});
                        else if (!state.selected_label || state.source_query != *state.selected_label)
                            combo_cleanup.push_back({state.read_selection, state.write_selection, ""});
                    }
                    state.open = state.session = false;
                }
                return removed;
            });
            if (!find(text_drag) || !interactive(*find(text_drag)))
                text_drag.clear();
            std::erase_if(layout_overrides, [&](auto& p) { return !find(p.first); });
            if (menu_popup()) {
                auto* trigger = find(menu_popup()->owner);
                if (!trigger || !trigger->enabled || trigger->hidden)
                    drop_menu();
                else {
                    menu_popup()->options = trigger->d.options;
                    if (menu_popup()->highlighted >= static_cast<int>(menu_popup()->options.size()))
                        menu_popup()->highlighted = -1;
                }
            }
            ++generation;
            dirty = false;
        } catch (...) {
            detail::collecting = prior;
            detail::building = was;
            throw;
        }
        detail::collecting = prior;
        detail::building = was;
    }
    Size text_layout(Node& n, double width) {
        n.text =
            fonts.layout(n, n.d.text, {width, n.d.text_wrap, n.d.overflow, n.d.max_lines, n.d.text_align});
        return n.text->size;
    }
    Size flow(Node& n, double width, bool arrange, Rect content = {}) {
        std::vector<Node*> children;
        bool separate = false;
        for (auto& child : n.children) {
            separate |= child->d.position != Position::normal || child->d.z_index != 0 || child->hidden;
            if (child->d.position != Position::absolute)
                children.push_back(child.get());
        }
        std::vector<TextSpan> spans;
        for (auto* c : children) {
            if (inline_text(*c)) {
                spans.push_back(fonts.span(*c, c->d.text));
                c->text.reset();
            } else {
                measure(*c, {0, 0}, {width, infinity});
                auto span = fonts.span(*c, "");
                span.box = TextInlineBox{
                    {c->desired.width + horizontal(c->d.margin), c->desired.height + vertical(c->d.margin)},
                    c->d.margin.top + std::max(0., (c->desired.height - c->line_height) / 2) +
                        fonts.ascent(*c)};
                spans.push_back(std::move(span));
            }
        }
        const TextLayoutOptions options{width, TextWrap::wrap, TextOverflow::clip, n.d.max_lines,
                                        n.d.justify == Justify::center ? Align::center
                                        : n.d.justify == Justify::end  ? Align::end
                                                                       : Align::start};
        auto text = fonts.engine->layout(spans, options);
        if (arrange) {
            n.text = separate ? nullptr : text;
            n.text_origin = {content.x, content.y};
            for (auto* child : children)
                if (inline_text(*child)) {
                    child->bounds = {content.x, content.y, 0, 0};
                    child->clip = n.clip;
                }
            std::unordered_set<std::size_t> placed;
            for (auto& p : text->placements) {
                auto& c = *children.at(p.span);
                Rect r = p.bounds;
                r.x += content.x;
                r.y += content.y;
                if (p.inline_box)
                    arrange_child(c, inset(r, c.d.margin), n, n.clip);
                else if (placed.insert(p.span).second)
                    c.bounds = r;
                else {
                    double right = std::max(c.bounds.x + c.bounds.width, r.x + r.width),
                           bottom = std::max(c.bounds.y + c.bounds.height, r.y + r.height);
                    c.bounds.x = std::min(c.bounds.x, r.x);
                    c.bounds.y = std::min(c.bounds.y, r.y);
                    c.bounds.width = right - c.bounds.x;
                    c.bounds.height = bottom - c.bounds.y;
                }
            }
            for (std::size_t i = 0; i < children.size(); ++i) {
                auto& child = *children[i];
                if (!inline_text(child))
                    continue;
                // Preserve shaping and wrapping of the whole paragraph while allowing
                // each styled span to participate in positioning and stacking separately.
                if (separate && !child.hidden) {
                    auto isolated = spans;
                    for (std::size_t j = 0; j < isolated.size(); ++j)
                        if (j != i)
                            isolated[j].color &= 0x00ffffff;
                    child.text = fonts.engine->layout(isolated, options);
                }
                if (separate) {
                    auto delta = relative_offset(child, content);
                    child.text_origin = {content.x + delta.x, content.y + delta.y};
                    child.bounds.x += delta.x;
                    child.bounds.y += delta.y;
                }
                child.clip = intersect(n.clip, child.bounds);
                child.desired = {child.bounds.width, child.bounds.height};
            }
        }
        return text->size;
    }
    double heading(const Node& n) const {
        return (n.d.kind == Kind::section || n.d.kind == Kind::group)
                   ? (n.d.kind == Kind::section ? 24 : 20) + (n.expanded ? n.d.gap : 0)
                   : 0;
    }
    Node* tab_page(const Node& n) const {
        for (std::size_t i = 1; i < n.children.size(); ++i)
            if (n.children[i]->d.key == n.selection)
                return n.children[i].get();
        return nullptr;
    }
    Size measure(Node& n, Size min, Size max) {
        auto& d = n.d;
        double target_w = resolve(d.width, max.width, infinity),
               target_h = resolve(d.height, max.height, infinity);
        double w = std::min(max.width, std::min(d.max_width, target_w)),
               h = std::min(max.height, std::min(d.max_height, target_h));
        double cw = std::max(0., w - horizontal(d.padding)),
               ch = std::max(0., h - vertical(d.padding) - heading(n));
        Size size{};
        switch (d.kind) {
        case Kind::label:
            size = text_layout(n, cw);
            break;
        case Kind::button:
        case Kind::toggle:
        case Kind::segment_option:
        case Kind::tab_option:
        case Kind::menu:
        case Kind::selection:
        case Kind::split:
            size = {fonts.width(n, d.text) + 24 + (d.color&&d.color->well?38:0) +
                        (d.kind == Kind::menu || d.kind == Kind::selection ? 32
                         : d.kind == Kind::split                           ? theme.split_width
                                                                           : 0),
                    height(n)};
            break;
        case Kind::checkbox:
        case Kind::radio_option:
            size = {30 + fonts.width(n, d.text), 36};
            break;
        case Kind::switch_control:
            size = {60 + fonts.width(n, d.text), 36};
            break;
        case Kind::stepper:
            size = {std::max(144., 88 + fonts.width(n, n.numeric_text)), height(n)};
            break;
        case Kind::step_button:
            size = {36, height(n)};
            break;
        case Kind::image: {
            size = d.bitmap.size();
            if (size.width > 0 && size.height > 0) {
                if (d.width.unit != Length::Unit::automatic && d.height.unit == Length::Unit::automatic)
                    size = {cw, std::min(ch, cw * size.height / size.width)};
                else if (d.height.unit != Length::Unit::automatic && d.width.unit == Length::Unit::automatic)
                    size = {std::min(cw, ch * size.width / size.height), ch};
                else {
                    double scale = std::min({1., cw / size.width, ch / size.height});
                    size = {size.width * scale, size.height * scale};
                }
            }
            break;
        }
        case Kind::list_option:
    case Kind::table_column:
    case Kind::table_resize:
    case Kind::table_cell:
            size = {fonts.width(n, d.text) + 24, 32};
            break;
        case Kind::listbox:
            size = {220, d.list_options.row_height * d.list_options.visible_rows + (d.table ? d.table->options.header_height : 0)};
            break;
        case Kind::text_field:
            size = {std::max(0., 220 - horizontal(d.padding)),
                    std::max(0., (d.text_options.multiline ? 120 : height(n)) - vertical(d.padding))};
            break;
        case Kind::tab_view: {
            const auto strip = measure(*n.children[0], {}, {cw, std::min(ch, height(n))});
            const auto* page = tab_page(n);
            const auto gap = page ? std::min(d.gap, std::max(0., ch - strip.height)) : 0.;
            const auto body = page ? measure(*tab_page(n), {}, {cw, std::max(0., ch - strip.height - gap)}) : Size{};
            size = {std::max(strip.width, body.width), strip.height + gap + body.height};
            break;
        }
        case Kind::split_view: {
            const bool row = d.orientation == Orientation::horizontal;
            const auto a = n.collapsed == SplitPane::first ? Size{} : measure(*n.children[0], {}, {cw, ch});
            const auto b = n.collapsed == SplitPane::second ? Size{} : measure(*n.children[1], {}, {cw, ch});
            const auto first = n.collapsed == SplitPane::first ? 0. :
                std::max(row ? a.width : a.height, d.split_options.first_min);
            const auto second = n.collapsed == SplitPane::second ? 0. :
                std::max(row ? b.width : b.height, d.split_options.second_min);
            const auto main = first + second + d.split_options.divider_width;
            size = row ? Size{main, std::max(a.height, b.height)} : Size{std::max(a.width, b.width), main};
            break;
        }
        case Kind::split_divider:
            size = {};
            break;
        case Kind::slider:
            size = {200, 36};
            break;
        case Kind::progress:
            size = {200, 16};
            break;
        case Kind::separator:
            size = d.orientation == Orientation::horizontal ? Size{std::isfinite(cw) ? cw : 0, 1}
                                                            : Size{1, std::isfinite(ch) ? ch : 0};
            break;
        case Kind::spacer:
            break;
        case Kind::flow:
            size = flow(n, cw, false);
            break;
        case Kind::scroll:
            if (!n.children.empty() && n.children.front()->d.position != Position::absolute) {
                const auto axes = d.scroll_options.axes;
                const auto m = n.children.front()->d.margin;
                size = measure(*n.children.front(), {},
                    {axes != ScrollAxes::vertical ? infinity : std::max(0., cw - horizontal(m)),
                     axes != ScrollAxes::horizontal ? infinity : std::max(0., ch - vertical(m))});
                size.width = std::min(cw, std::max(0., size.width + horizontal(m)));
                size.height = std::min(ch, std::max(0., size.height + vertical(m)));
            }
            break;
        default: {
            bool row = horizontal_kind(d.kind);
            double main = 0, cross = 0;
            std::vector<Size> child_sizes;
            for (auto& c : n.children) {
                if (c->d.position == Position::absolute)
                    continue;
                auto s = measure(
                    *c, {0, 0},
                    {std::max(0., cw - horizontal(c->d.margin)), std::max(0., ch - vertical(c->d.margin))});
                s.width += horizontal(c->d.margin);
                s.height += vertical(c->d.margin);
                child_sizes.push_back(s);
            }
            if (d.kind == Kind::layer || d.kind == Kind::tab_page) {
                for (auto s : child_sizes) {
                    size.width = std::max(size.width, s.width);
                    size.height = std::max(size.height, s.height);
                }
            } else if (row && d.wrap && std::isfinite(cw)) {
                double line_w = 0, line_h = 0;
                for (auto s : child_sizes) {
                    if (line_w > 0 && line_w + d.gap + s.width > cw) {
                        main = std::max(main, line_w);
                        cross += line_h + d.gap;
                        line_w = line_h = 0;
                    }
                    if (line_w > 0)
                        line_w += d.gap;
                    line_w += s.width;
                    line_h = std::max(line_h, s.height);
                }
                size = {std::max(main, line_w), cross + line_h};
            } else {
                for (auto s : child_sizes) {
                    main += row ? s.width : s.height;
                    cross = std::max(cross, row ? s.height : s.width);
                }
                if (child_sizes.size() > 1)
                    main += d.gap * (child_sizes.size() - 1);
                size = row ? Size{main, cross} : Size{cross, main};
            }
            break;
        }
        }
        size.width += horizontal(d.padding);
        size.height += vertical(d.padding) + heading(n);
        size.width = resolve(d.width, max.width, size.width);
        size.height = resolve(d.height, max.height, size.height);
        const double low_w = std::min(max.width, std::max(min.width, d.min_width)),
                     low_h = std::min(max.height, std::max(min.height, d.min_height));
        size.width = std::clamp(size.width, low_w, std::max(low_w, std::min(max.width, d.max_width)));
        size.height = std::clamp(size.height, low_h, std::max(low_h, std::min(max.height, d.max_height)));
        n.desired = size;
        return size;
    }
    void distribute(const std::vector<Node*>& children, std::vector<double>& sizes, bool row,
                    double available, double gap) {
        double used = std::accumulate(sizes.begin(), sizes.end(), 0.0) +
                      gap * std::max(0., static_cast<double>(sizes.size()) - 1);
        for (auto* c : children)
            used += row ? horizontal(c->d.margin) : vertical(c->d.margin);
        double free = available - used;
        if (!std::isfinite(free) || std::abs(free) < .001)
            return;
        std::vector<bool> frozen(sizes.size());
        for (std::size_t pass = 0; pass < children.size() + 1; ++pass) {
            double weights = 0;
            for (std::size_t i = 0; i < sizes.size(); ++i)
                if (!frozen[i])
                    weights +=
                        free > 0 ? children[i]->d.grow : children[i]->d.shrink * std::max(.001, sizes[i]);
            if (weights <= 0)
                break;
            double consumed = 0;
            for (std::size_t i = 0; i < sizes.size(); ++i)
                if (!frozen[i]) {
                    auto& d = children[i]->d;
                    double weight = free > 0 ? d.grow : d.shrink * std::max(.001, sizes[i]);
                    double desired = sizes[i] + free * weight / weights;
                    double bounded = std::clamp(
                        desired, row ? d.min_width : d.min_height,
                        std::max(row ? d.min_width : d.min_height, row ? d.max_width : d.max_height));
                    consumed += bounded - sizes[i];
                    sizes[i] = bounded;
                    if (std::abs(desired - bounded) > .001)
                        frozen[i] = true;
                }
            free -= consumed;
            if (std::abs(free) < .001)
                break;
        }
    }
    void arrange_flex_line(Node& n, const std::vector<Node*>& children, Rect content, bool row) {
        const double available = row ? content.width : content.height;
        std::vector<double> sizes;
        for (auto* c : children)
            sizes.push_back(resolve(c->d.basis, available, row ? c->desired.width : c->desired.height));
        distribute(children, sizes, row, available, n.d.gap);
        double used = std::accumulate(sizes.begin(), sizes.end(), 0.0) +
                      n.d.gap * std::max(0., static_cast<double>(sizes.size()) - 1);
        for (auto* c : children)
            used += row ? horizontal(c->d.margin) : vertical(c->d.margin);
        double extra = std::max(0., available - used), cursor = 0, gap = n.d.gap;
        if (n.d.justify == Justify::center)
            cursor = extra / 2;
        if (n.d.justify == Justify::end)
            cursor = extra;
        if (n.d.justify == Justify::space_between && children.size() > 1)
            gap += extra / (children.size() - 1);
        if (n.d.justify == Justify::space_around && !children.empty()) {
            gap += extra / children.size();
            cursor = extra / children.size() / 2;
        }
        if (n.d.kind == Kind::row || n.d.kind == Kind::column) {
            FlexLineInspection info{content, {}, gap, available - used};
            for (const auto* child : children)
                info.items.push_back(child->id);
            n.flex_lines.push_back(std::move(info));
        }
        double baseline = 0;
        if (row)
            for (auto* c : children)
                baseline = std::max(baseline, c->d.margin.top +
                                                  std::max(0., (c->desired.height - c->line_height) / 2) +
                                                  fonts.ascent(*c));
        for (std::size_t i = 0; i < children.size(); ++i) {
            auto& c = *children[i];
            auto m = c.d.margin;
            Align align = c.d.self.value_or(n.d.align);
            double cross_available =
                std::max(0., (row ? content.height : content.width) - (row ? vertical(m) : horizontal(m)));
            // Reflow after the flex algorithm has assigned the main-axis size.
            if (row)
                measure(c, {0, 0}, {sizes[i], cross_available});
            double cross = align == Align::stretch
                               ? cross_available
                               : std::min(cross_available, row ? c.desired.height : c.desired.width);
            if (row && c.d.height.unit != Length::Unit::automatic)
                cross = std::min(cross, resolve(c.d.height, content.height, cross));
            if (!row && c.d.width.unit != Length::Unit::automatic)
                cross = std::min(cross, resolve(c.d.width, content.width, cross));
            cross = std::clamp(cross, std::min(cross_available, row ? c.d.min_height : c.d.min_width),
                               std::max(std::min(cross_available, row ? c.d.min_height : c.d.min_width),
                                        std::min(cross_available, row ? c.d.max_height : c.d.max_width)));
            double offset = align == Align::center ? (cross_available - cross) / 2
                            : align == Align::end  ? cross_available - cross
                                                   : 0;
            if (row && align == Align::baseline)
                offset = baseline - m.top - (std::max(0., (cross - c.line_height) / 2) + fonts.ascent(c));
            Rect r = row ? Rect{content.x + cursor + m.left, content.y + m.top + offset, sizes[i], cross}
                         : Rect{content.x + m.left + offset, content.y + cursor + m.top, cross, sizes[i]};
            arrange_child(c, r, n, n.clip);
            cursor += sizes[i] + (row ? horizontal(m) : vertical(m)) + gap;
        }
    }
    Rect content_box(const Node& n) const {
        auto content = inset(n.bounds, n.d.padding);
        const double header = heading(n) + (n.d.table ? n.d.table->options.header_height : 0);
        content.y += header;
        content.height = std::max(0., content.height - header);
        return content;
    }
    Point relative_offset(const Node& n, Rect content) const {
        if (n.d.position != Position::relative)
            return {};
        return {n.d.left.unit != Length::Unit::automatic ? resolve(n.d.left, content.width, 0)
                                                         : -resolve(n.d.right, content.width, 0),
                n.d.top.unit != Length::Unit::automatic ? resolve(n.d.top, content.height, 0)
                                                        : -resolve(n.d.bottom, content.height, 0)};
    }
    void arrange_child(Node& n, Rect bounds, const Node& parent, Rect clip) {
        const auto delta = relative_offset(n, content_box(parent));
        bounds.x += delta.x;
        bounds.y += delta.y;
        arrange_node(n, bounds, clip);
    }
    void arrange_absolute(Node& n, Rect content, Rect clip) {
        const auto& d = n.d;
        const auto m = d.margin;
        const bool left = d.left.unit != Length::Unit::automatic,
                   right = d.right.unit != Length::Unit::automatic,
                   top = d.top.unit != Length::Unit::automatic,
                   bottom = d.bottom.unit != Length::Unit::automatic;
        const double l = resolve(d.left, content.width, 0), r = resolve(d.right, content.width, 0),
                     t = resolve(d.top, content.height, 0), b = resolve(d.bottom, content.height, 0);
        double width = std::max(0., content.width - horizontal(m) - (left ? l : 0) - (right ? r : 0)),
               height = std::max(0., content.height - vertical(m) - (top ? t : 0) - (bottom ? b : 0));
        // Percentage dimensions resolve against the complete containing block; opposing
        // anchors only constrain automatic sizes.
        auto size = measure(n, {0, 0}, {content.width, content.height});
        if (d.width.unit == Length::Unit::automatic && left && right)
            size.width = std::clamp(width, d.min_width, std::max(d.min_width, d.max_width));
        if (d.height.unit == Length::Unit::automatic && top && bottom)
            size.height = std::clamp(height, d.min_height, std::max(d.min_height, d.max_height));
        if (d.width.unit == Length::Unit::automatic && !(left && right))
            size.width = std::min(size.width, width);
        if (d.width.unit == Length::Unit::automatic) {
            auto reflowed = measure(n, {size.width, 0}, {size.width, content.height});
            if (d.height.unit == Length::Unit::automatic && !(top && bottom))
                size.height = reflowed.height;
        }
        if (d.height.unit == Length::Unit::automatic && !(top && bottom))
            size.height = std::min(size.height, height);
        const double x = left    ? content.x + l + m.left
                         : right ? content.x + content.width - r - m.right - size.width
                                 : content.x + m.left,
                     y = top      ? content.y + t + m.top
                         : bottom ? content.y + content.height - b - m.bottom - size.height
                                  : content.y + m.top;
        arrange_node(n, {x, y, size.width, size.height}, clip);
    }
    struct SplitMetrics {
        bool row{};
        double divider{}, available{}, low{}, high{}, first{};
    };
    SplitMetrics split_metrics(const Node& n) const {
        const auto r = content_box(n);
        const auto& options = n.d.split_options;
        SplitMetrics result;
        result.row = n.d.orientation == Orientation::horizontal;
        const auto extent = result.row ? r.width : r.height;
        result.divider = std::min(extent, options.divider_width);
        result.available = std::max(0., extent - result.divider);
        const auto minimum = options.first_min + options.second_min;
        if (minimum > result.available) {
            result.low = minimum > 0 ? result.available * (options.first_min / minimum) : 0;
            result.high = result.low;
        } else {
            result.low = options.first_min;
            result.high = result.available - options.second_min;
        }
        result.first = n.collapsed == SplitPane::first ? 0. :
                       n.collapsed == SplitPane::second ? result.available :
                       std::clamp(n.number * result.available, result.low, result.high);
        return result;
    }
    void arrange_node(Node& n, Rect bounds, Rect clip) {
        n.flex_lines.clear();
        n.bounds = bounds;
        n.clip = intersect(clip, bounds);
        Rect content = content_box(n);
        if (n.d.kind == Kind::stepper) {
            const auto width = std::min(36., content.width / 3);
            arrange_node(*n.children[0], {content.x, content.y, width, content.height}, n.clip);
            arrange_node(*n.children[1], {content.x + content.width - width, content.y, width, content.height}, n.clip);
            return;
        }
        if (n.d.kind == Kind::tab_view) {
            auto& bar = *n.children[0];
            const auto strip_height = bar.children.empty() ? 0. : std::min(content.height, height(n));
            bar.bounds = {content.x, content.y, content.width, strip_height};
            bar.clip = intersect(n.clip, bar.bounds);
            auto& state = tabs.at(n.id);
            auto* target = find(focused);
            if (!target || target->d.kind != Kind::tab_option || target->group != n.id)
                target = find(n.id + "/tab/" + path_key(n.selection));
            double total = 0, start = 0, end = 0;
            for (auto& tab : bar.children) {
                measure(*tab, {}, {content.width, strip_height});
                if (tab.get() == target) {
                    start = total;
                    end = start + tab->desired.width;
                }
                total += tab->desired.width;
            }
            const auto target_id = target ? target->id : std::string{};
            if (target && (state.reveal || state.reveal_target != target_id || state.width != content.width ||
                           state.target_start != start)) {
                if (start < state.offset)
                    state.offset = start;
                if (end > state.offset + content.width)
                    state.offset = end - content.width;
            }
            state.reveal = false;
            state.reveal_target = target_id;
            state.target_start = start;
            state.width = content.width;
            state.maximum = std::max(0., total - content.width);
            state.offset = std::clamp(state.offset, 0., state.maximum);
            double x = content.x - state.offset;
            for (auto& tab : bar.children) {
                arrange_node(*tab, {x, content.y, tab->desired.width, strip_height}, bar.clip);
                x += tab->desired.width;
            }
            if (auto* page = tab_page(n)) {
                const auto gap = std::min(n.d.gap, std::max(0., content.height - strip_height));
                const Rect body{content.x, content.y + strip_height + gap, content.width,
                                std::max(0., content.height - strip_height - gap)};
                measure(*page, {}, {body.width, body.height});
                arrange_node(*page, body, intersect(n.clip, body));
            }
            return;
        }
        if (n.d.kind == Kind::split_view) {
            const auto s = split_metrics(n);
            const auto child_clip = intersect(n.clip, content);
            const auto rect = [&](double start, double extent) {
                return s.row ? Rect{content.x + start, content.y, extent, content.height}
                             : Rect{content.x, content.y + start, content.width, extent};
            };
            const auto a = rect(0, s.first);
            const auto b = rect(s.first + s.divider, s.available - s.first);
            measure(*n.children[0], {}, {a.width, a.height});
            measure(*n.children[1], {}, {b.width, b.height});
            arrange_node(*n.children[0], a, child_clip);
            arrange_node(*n.children[1], b, child_clip);
            auto& divider = *n.children[2];
            divider.number = s.available > 0 ? s.first / s.available : 0;
            arrange_node(divider, rect(s.first, s.divider), child_clip);
            return;
        }
        for (auto& child : n.children)
            if (child->d.position == Position::absolute)
                arrange_absolute(*child, content, n.clip);
        if (n.d.kind == Kind::split) {
            if (!n.children.empty())
                arrange_node(*n.children.front(),
                             {bounds.x + std::max(0., bounds.width - theme.split_width), bounds.y,
                              std::min(bounds.width, theme.split_width), bounds.height},
                             clip);
            return;
        }
        if (n.d.kind == Kind::listbox) {
            auto& offset = scroll[n.id];
            const auto row_height = n.d.list_options.row_height;
            const auto document_width = n.d.table ? std::max(content.width, table_extent(n)) : content.width;
            n.scroll_content = {document_width, std::max(content.height, row_height * n.d.rows.size())};
            n.scroll_max = {std::max(0., document_width-content.width), std::max(0., n.scroll_content.height-content.height)};
            offset = {std::clamp(offset.x,0.,n.scroll_max.x), std::clamp(offset.y,0.,n.scroll_max.y)};
            if(n.d.table&&tables.at(n.id).reveal){
                const auto cell=*tables.at(n.id).reveal;tables.at(n.id).reveal.reset();
                const auto row=n.d.rows.index_of(cell.row),col=n.d.table->model.column_index(cell.column);
                if(row&&col){
                    double left{};for(std::size_t i=0;i<*col;++i)left+=tables.at(n.id).widths.at(n.d.table->model.columns()[i].id);
                    const auto right=left+tables.at(n.id).widths.at(cell.column),top=*row*row_height;
                    if(left<offset.x)offset.x=left;else if(right>offset.x+content.width)offset.x=std::min(left,right-content.width);
                    if(top<offset.y)offset.y=top;else if(top+row_height>offset.y+content.height)offset.y=top+row_height-content.height;
                    offset.x=std::clamp(offset.x,0.,n.scroll_max.x);offset.y=std::clamp(offset.y,0.,n.scroll_max.y);
                }
            }
            realize_rows(n, content.height);
            for (auto& row : n.children) {
                if(row->d.kind==Kind::table_header){
                    const auto raw=inset(n.bounds,n.d.padding);
                    arrange_node(*row,{raw.x-offset.x,raw.y,document_width,std::min(raw.height,n.d.table->options.header_height)},intersect(n.clip,{raw.x,raw.y,raw.width,std::min(raw.height,n.d.table->options.header_height)}));
                }else arrange_node(*row, {content.x-offset.x, content.y + row->row_index * row_height - offset.y, document_width, row_height}, intersect(n.clip, content));
            }
            return;
        }
        if(n.d.kind==Kind::table_header || (n.d.kind==Kind::list_option && find(n.group) && find(n.group)->d.table)){
            auto* table=n.d.kind==Kind::table_header?find(n.id.substr(0,n.id.size()-7)):find(n.group);
            if(!table||!table->d.table)return;
            double x=bounds.x;
            for(auto& child:n.children){
                const auto width=tables.at(table->id).widths.at(child->column);
                arrange_node(*child,{x,bounds.y,width,bounds.height},n.clip);x+=width;
            }
            return;
        }
        if(n.d.kind==Kind::table_column){
            for(auto& child:n.children)arrange_node(*child,{bounds.x+std::max(0.,bounds.width-8),bounds.y,std::min(8.,bounds.width),bounds.height},n.clip);
            return;
        }
        if(n.d.kind==Kind::table_cell){
            for(auto& child:n.children){measure(*child,{}, {content.width,content.height});arrange_child(*child,content,n,n.clip);}
            return;
        }
        if (n.d.kind == Kind::text_field) {
            arrange_field(n);
            return;
        }
        if (n.d.kind == Kind::list_option && n.custom_row) {
            for (auto& child : n.children) if (child->d.position != Position::absolute) {
                measure(*child, {}, {content.width, content.height});
                arrange_child(*child, content, n, n.clip);
            }
            return;
        }
        if (n.d.kind == Kind::label) {
            text_layout(n, content.width);
            n.text_origin = {content.x, content.y};
            return;
        }
        if (n.d.kind == Kind::flow) {
            flow(n, content.width, true, content);
            return;
        }
        if (n.d.kind == Kind::scroll) {
            auto& offset = scroll[n.id];
            n.scroll_content = {content.width, content.height};
            n.scroll_max = {};
            if (n.children.empty() || n.children.front()->d.position == Position::absolute) {
                offset = {};
                return;
            }
            auto& c = *n.children.front();
            const auto m = c.d.margin;
            const auto axes = n.d.scroll_options.axes;
            const auto w = std::max(0., content.width - horizontal(m)), h = std::max(0., content.height - vertical(m));
            auto s = measure(c, {}, {axes != ScrollAxes::vertical ? infinity : w, axes != ScrollAxes::horizontal ? infinity : h});
            s.width = axes != ScrollAxes::vertical ? std::max(w, s.width) : w;
            s.height = axes != ScrollAxes::horizontal ? std::max(h, s.height) : h;
            n.scroll_content = {std::max(0., s.width + horizontal(m)), std::max(0., s.height + vertical(m))};
            n.scroll_max = {axes != ScrollAxes::vertical ? std::max(0., n.scroll_content.width - content.width) : 0,
                            axes != ScrollAxes::horizontal ? std::max(0., n.scroll_content.height - content.height) : 0};
            offset = {std::clamp(offset.x, 0., n.scroll_max.x), std::clamp(offset.y, 0., n.scroll_max.y)};
            arrange_child(c,
                          {content.x + m.left - offset.x, content.y + m.top - offset.y, s.width, s.height},
                          n, intersect(n.clip, content));
            return;
        }
        if (n.children.empty())
            return;
        if (n.d.kind == Kind::layer || n.d.kind == Kind::tab_page) {
            for (auto& c : n.children) {
                if (c->d.position == Position::absolute)
                    continue;
                const auto m = c->d.margin;
                const Size available{std::max(0., content.width - horizontal(m)),
                                     std::max(0., content.height - vertical(m))};
                auto size = measure(*c, {0, 0}, available);
                const auto align = c->d.self.value_or(n.d.align);
                if (align == Align::stretch) {
                    if (c->d.width.unit == Length::Unit::automatic)
                        size.width =
                            std::clamp(available.width, std::min(available.width, c->d.min_width),
                                       std::max(std::min(available.width, c->d.min_width), c->d.max_width));
                    if (c->d.height.unit == Length::Unit::automatic)
                        size.height = std::clamp(
                            available.height, std::min(available.height, c->d.min_height),
                            std::max(std::min(available.height, c->d.min_height), c->d.max_height));
                }
                const double x = align == Align::center ? (available.width - size.width) / 2
                                 : align == Align::end  ? available.width - size.width
                                                        : 0,
                             y = align == Align::center ? (available.height - size.height) / 2
                                 : align == Align::end  ? available.height - size.height
                                                        : 0;
                arrange_child(*c, {content.x + m.left + x, content.y + m.top + y, size.width, size.height}, n,
                              n.clip);
            }
            return;
        }
        if (n.d.kind == Kind::block) {
            double y = content.y;
            for (auto& c : n.children) {
                if (c->d.position == Position::absolute)
                    continue;
                auto margin = c->d.margin;
                double available = std::max(0., content.width - horizontal(margin));
                auto size = measure(*c, {0, 0}, {available, infinity});
                auto align = c->d.self.value_or(n.d.align);
                double width = c->d.width.unit == Length::Unit::automatic && align == Align::stretch
                                   ? std::min(available, c->d.max_width)
                                   : size.width;
                double offset = align == Align::center ? (available - width) / 2
                                : align == Align::end  ? available - width
                                                       : 0;
                arrange_child(*c, {content.x + margin.left + offset, y + margin.top, width, size.height}, n,
                              n.clip);
                y += size.height + vertical(margin) + n.d.gap;
            }
            return;
        }
        bool row = horizontal_kind(n.d.kind);
        std::vector<Node*> children;
        for (auto& c : n.children)
            if (c->d.position != Position::absolute)
                children.push_back(c.get());
        if (row && n.d.wrap) {
            std::vector<Node*> group;
            double x = 0, y = 0, h = 0;
            auto flush = [&] {
                arrange_flex_line(n, group, {content.x, content.y + y, content.width, h}, true);
                y += h + n.d.gap;
                x = h = 0;
                group.clear();
            };
            for (auto* c : children) {
                auto s =
                    measure(*c, {0, 0}, {std::max(0., content.width - horizontal(c->d.margin)), infinity});
                if (!group.empty() && x + n.d.gap + s.width + horizontal(c->d.margin) > content.width)
                    flush();
                if (!group.empty())
                    x += n.d.gap;
                x += s.width + horizontal(c->d.margin);
                h = std::max(h, s.height + vertical(c->d.margin));
                group.push_back(c);
            }
            if (!group.empty())
                flush();
        } else {
            // Columns and blocks remeasure each child against their final available width.
            if (!row)
                for (auto* c : children)
                    measure(*c, {0, 0},
                            {std::max(0., content.width - horizontal(c->d.margin)), content.height});
            arrange_flex_line(n, children, content, row);
        }
    }
    static Rect place_overlay(Rect anchor, Size desired, Size viewport, OverlayPlacement placement,
                              double gap, double margin) {
        const auto mx = std::min(margin, viewport.width / 2), my = std::min(margin, viewport.height / 2);
        const auto w = std::min(desired.width, std::max(0., viewport.width - 2 * mx));
        const auto h = std::min(desired.height, std::max(0., viewport.height - 2 * my));
        const auto below = viewport.height - my - anchor.y - anchor.height - gap;
        const auto above = anchor.y - gap - my;
        const auto left = anchor.x - gap - mx;
        const auto right = viewport.width - mx - anchor.x - anchor.width - gap;
        double x = anchor.x, y = anchor.y;
        switch (placement) {
        case OverlayPlacement::below:
            y = h > below && above > below ? anchor.y - gap - h : anchor.y + anchor.height + gap; break;
        case OverlayPlacement::above:
            y = h > above && below > above ? anchor.y + anchor.height + gap : anchor.y - gap - h; break;
        case OverlayPlacement::left:
            x = w > left && right > left ? anchor.x + anchor.width + gap : anchor.x - gap - w; break;
        case OverlayPlacement::right:
            x = w > right && left > right ? anchor.x - gap - w : anchor.x + anchor.width + gap; break;
        }
        return {std::clamp(x, mx, viewport.width - mx - w),
                std::clamp(y, my, viewport.height - my - h), w, h};
    }
    void hide_tooltip() {
        if (!tooltip) return;
        const bool shown = tooltip->shown;
        tooltip.reset();
        if (shown) invalidate();
        else notify();
    }
    void schedule_tooltip(Node* n) {
        if (!active || !n || !n->d.tooltip || !n->enabled || n->hidden || n->id == suppressed_tooltip || menu_popup()) {
            hide_tooltip();
            return;
        }
        const auto& spec = *n->d.tooltip;
        if (tooltip && tooltip->owner == n->id && tooltip->text == spec.text &&
            same_tooltip_options(tooltip->options, spec.options)) return;
        hide_tooltip();
        tooltip = TooltipState{n->id, spec.text, spec.options, options.clock() + spec.options.delay};
        notify();
    }
    static bool same_tooltip_options(const TooltipOptions& a, const TooltipOptions& b) {
        return a.delay == b.delay && a.max_width == b.max_width && a.gap == b.gap &&
            a.margin == b.margin && a.placement == b.placement;
    }
    void dismiss_popover(const std::string& root_id, bool restore = true) {
        auto it = std::find_if(overlays.begin(), overlays.end(), [&](const auto& o) { return o.root_id == root_id; });
        if (it == overlays.end() || !it->popover) return;
        it->restore_on_close = restore;
        auto presentation = it->popover->presented;
        // Clear every descendant presentation before rebuilding; stale true
        // bindings must not resurrect a child when its parent is reopened.
        std::vector<Binding<bool>> descendants;
        for (auto& o : overlays)
            if (o.popover && o.root_id.starts_with(root_id + "/")) {
                o.restore_on_close = restore;
                descendants.push_back(o.popover->presented);
            }
        for (const auto& binding : descendants) if (binding.get()) binding.set(false);
        if (presentation.get()) presentation.set(false);
        invalidate();
        layout(viewport);
    }
    void position_overlays() {
        const Rect window{0, 0, viewport.width, viewport.height};
        for (auto& overlay : overlays) {
            if (overlay.kind == OverlayKind::menu || overlay.context) continue;
            auto* anchor = find(overlay.owner);
            auto* body = find(overlay.root_id);
            if (!anchor || !body) continue;
            if (overlay.kind == OverlayKind::combo) {
                const auto& config = *anchor->d.combo;
                body->d.width = anchor->bounds.width;
                const Size limit{std::max(0., viewport.width - 8), std::min(config.max_height, std::max(0., viewport.height - 8))};
                const auto size = measure(*body, {}, limit);
                overlay.bounds = place_overlay(anchor->bounds, size, viewport, OverlayPlacement::below, 4, 4);
                arrange_node(*body, overlay.bounds, window);
                auto& state = combos.at(anchor->id);
                if (state.reveal && body->d.kind == Kind::listbox) {
                    if (const auto index = body->d.rows.index_of(state.highlighted)) {
                        const auto content = content_box(*body);
                        const auto height = body->d.list_options.row_height;
                        const auto top = *index * height;
                        const auto before = scroll[body->id].y;
                        auto next = before;
                        if (top < next) next = top;
                        else if (top + height > next + content.height) next = top + height - content.height;
                        next = std::clamp(next, 0., body->scroll_max.y);
                        if (next != before) {
                            scroll[body->id].y = next;
                            arrange_node(*body, overlay.bounds, window);
                        }
                    }
                    state.reveal = false;
                }
                continue;
            }
            const auto placement = overlay.popover ? overlay.popover->options.placement : anchor->d.tooltip->options.placement;
            const auto margin = overlay.popover ? overlay.popover->options.margin : anchor->d.tooltip->options.margin;
            const auto gap = overlay.popover ? overlay.popover->options.gap : anchor->d.tooltip->options.gap;
            const auto mx = std::min(margin, viewport.width / 2), my = std::min(margin, viewport.height / 2);
            const Size limit{std::max(0., viewport.width - 2 * mx), std::max(0., viewport.height - 2 * my)};
            const auto size = measure(*body, {}, limit);
            overlay.bounds = place_overlay(anchor->bounds, size, viewport, placement, gap, margin);
            arrange_node(*body, overlay.bounds, window);
        }
    }
    void update_overlay_lifecycle() {
        if (processing_overlays) return;
        processing_overlays = true;
        try {
            // Call application setters only after leaving view construction.
            auto retired_colors=std::exchange(color_cleanup,{});
            for(const auto& entry:retired_colors)
                if(entry.value.get()==entry.owned&&entry.owned!=entry.baseline){entry.value.set(entry.baseline);invalidate();}
            auto cleanup = std::exchange(combo_cleanup, {});
            for (const auto& entry : cleanup)
                if (entry.read() != entry.value) { entry.write(entry.value); invalidate(); }
            auto closed = std::exchange(closed_overlays, {});
            std::string restore;
            bool restore_needed = false;
            for (const auto& entry : closed) {
                const auto& old = entry.overlay;
                if (old.restore_on_close && (focused.empty() || focused == old.root_id || focused.starts_with(old.root_id + "/"))) {
                    if (!restore_needed) restore = old.restore_focus.empty() ? old.owner : old.restore_focus;
                    restore_needed = true;
                }
                if (entry.clear_binding && old.popover->presented.get()) old.popover->presented.set(false);
                if (old.popover->options.on_dismiss) old.popover->options.on_dismiss();
            }
            if (dirty) layout(viewport);
            if (restore_needed && active) {
                auto* target = find(restore);
                if (target && interactive(*target) && in_focus_scope(*target)) focus(restore, focus_visible);
                else if (const auto controls = focusables(); !controls.empty()) focus(controls.front()->id, focus_visible);
            }
            std::vector<std::string> opening;
            for (auto& overlay : overlays)
                if (overlay.popover && !overlay.initialized) {
                    overlay.initialized = true;
                    if (overlay.popover->options.focus == PopoverFocus::content) opening.push_back(overlay.root_id);
                }
            if (!opening.empty() && active) {
                hide_tooltip();
                auto controls = focusables();
                const auto scope = opening.back();
                auto target = std::find_if(controls.begin(), controls.end(), [&](const auto* n) {
                    return n->id.starts_with(scope + "/");
                });
                if (target != controls.end()) focus((*target)->id, focus_visible);
                else { finish_editing(); focused.clear(); }
            }
            if (viewport.width > 0 && viewport.height > 0) {
                std::vector<std::string> clipped;
                for (const auto& overlay : overlays) {
                    const auto* anchor = find(overlay.owner);
                    const auto visible = anchor ? intersect(anchor->bounds, anchor->clip) : Rect{};
                    if (overlay.popover && (!anchor || anchor->hidden || !anchor->enabled || visible.width <= 0 || visible.height <= 0))
                        clipped.push_back(overlay.root_id);
                }
                for (const auto& id : clipped) dismiss_popover(id, false);
                std::vector<std::string> combo_clipped;
                for (const auto& overlay : overlays)
                    if (overlay.kind == OverlayKind::combo) {
                        const auto* anchor = find(overlay.owner);
                        const auto visible = anchor ? intersect(anchor->bounds, anchor->clip) : Rect{};
                        if (!anchor || visible.width <= 0 || visible.height <= 0) combo_clipped.push_back(overlay.owner);
                    }
                for (const auto& id : combo_clipped) finish_combo(id, false);
            }
            if (tooltip) {
                const auto* anchor = find(tooltip->owner);
                if (!anchor || !anchor->enabled || anchor->hidden || !anchor->d.tooltip ||
                    anchor->d.tooltip->text != tooltip->text ||
                    !same_tooltip_options(anchor->d.tooltip->options, tooltip->options) || !in_focus_scope(*anchor) ||
                    (viewport.width > 0 && viewport.height > 0 &&
                     (intersect(anchor->bounds, anchor->clip).width <= 0 || intersect(anchor->bounds, anchor->clip).height <= 0)))
                    hide_tooltip();
            }
        } catch (...) { processing_overlays = false; throw; }
        processing_overlays = false;
        if (dirty) layout(viewport);
        else if (!closed_overlays.empty() || std::any_of(overlays.begin(), overlays.end(), [](const auto& o) {
            return o.popover && !o.initialized;
        })) update_overlay_lifecycle();
    }
    static bool menu_selectable(const MenuEntry& entry) {
        return entry.item.enabled && entry.item.kind != MenuItemKind::separator &&
            (entry.item.kind == MenuItemKind::submenu ? !entry.children.empty() : static_cast<bool>(entry.item.invoke));
    }
    double context_row_height(const MenuEntry& entry) const {
        return entry.item.kind == MenuItemKind::separator ? 12 : theme.popup_item_height;
    }
    double context_height(const Overlay& menu) const {
        double height = 0;
        for (const auto& entry : menu.context->entries) height += context_row_height(entry);
        return height;
    }
    Rect context_item_rect(const Overlay& menu, std::size_t index) const {
        double y = menu.bounds.y + theme.popup_padding - menu.scroll;
        for (std::size_t i = 0; i < index; ++i) y += context_row_height(menu.context->entries[i]);
        return {menu.bounds.x + theme.popup_padding, y, std::max(0., menu.bounds.width - 2 * theme.popup_padding),
                context_row_height(menu.context->entries[index])};
    }
    static std::string context_item_id(const Overlay& menu, std::size_t index) {
        return menu.root_id + "/item/" + path_key(menu.context->entries[index].item.id);
    }
    int context_hit(const Overlay& menu, Point point) const {
        if (!inset(menu.bounds, Insets::all(theme.popup_padding)).contains(point)) return -1;
        for (std::size_t i = 0; i < menu.context->entries.size(); ++i)
            if (context_item_rect(menu, i).contains(point)) return static_cast<int>(i);
        return -1;
    }
    void position_contexts(bool reveal = false) {
        for (std::size_t index = 0; index < overlays.size(); ++index) {
            auto& menu = overlays[index];
            if (!menu.context) continue;
            const auto* owner = find(menu.owner);
            if (!owner) continue;
            Rect anchor = owner->bounds;
            const auto visible = intersect(anchor, owner->clip);
            bool invalid = owner->hidden || !owner->enabled ||
                (viewport.width > 0 && viewport.height > 0 && (visible.width <= 0 || visible.height <= 0));
            const bool child = !menu.context->parent_menu.empty();
            if (child) {
                const auto parent = std::find_if(overlays.begin(), overlays.begin() + index, [&](const auto& p) {
                    return p.root_id == menu.context->parent_menu;
                });
                if (parent == overlays.begin() + index || !parent->context) invalid = true;
                else {
                    const auto& entries = parent->context->entries;
                    auto entry = std::find_if(entries.begin(), entries.end(), [&](const auto& e) { return e.item.id == menu.context->parent_item; });
                    if (entry == entries.end() || !menu_selectable(*entry)) invalid = true;
                    else {
                        anchor = context_item_rect(*parent, std::distance(entries.begin(), entry));
                        if (intersect(anchor, inset(parent->bounds, Insets::all(theme.popup_padding))).height <= 0) invalid = true;
                    }
                }
            } else if (menu.context->origin) anchor = {menu.context->origin->x, menu.context->origin->y, 0, 0};
            if (invalid) {
                overlays.erase(overlays.begin() + index, overlays.end());
                ++generation;
                notify();
                break;
            }
            double width = 140;
            for (const auto& entry : menu.context->entries)
                width = std::max(width, fonts.width(*owner, entry.item.label) + fonts.width(*owner, entry.item.shortcut) + 88);
            const auto h = context_height(menu) + 2 * theme.popup_padding;
            menu.bounds = place_overlay(anchor, {width, h}, viewport,
                child ? OverlayPlacement::right : OverlayPlacement::below, child ? 2 : 0, 4);
            const auto inside = inset(menu.bounds, Insets::all(theme.popup_padding));
            const auto max_scroll = std::max(0., context_height(menu) - inside.height);
            menu.scroll = std::clamp(menu.scroll, 0., max_scroll);
            if (reveal && index + 1 == overlays.size() && menu.highlighted >= 0) {
                const auto row = context_item_rect(menu, menu.highlighted);
                if (row.y < inside.y) menu.scroll = std::max(0., menu.scroll + row.y - inside.y);
                else if (row.y + row.height > inside.y + inside.height)
                    menu.scroll = std::min(max_scroll, menu.scroll + row.y + row.height - inside.y - inside.height);
            }
        }
    }
    void paint_contexts(SkCanvas& canvas) {
        for (const auto& menu : overlays) {
            if (!menu.context) continue;
            auto* owner = find(menu.owner);
            if (!owner) continue;
            box(canvas, menu.bounds, theme.panel, 6);
            box(canvas, menu.bounds, theme.border, 6, 1);
            canvas.save();
            const auto clip = inset(menu.bounds, Insets::all(theme.popup_padding));
            canvas.clipRect(SkRect::MakeXYWH(clip.x, clip.y, clip.width, clip.height));
            for (std::size_t i = 0; i < menu.context->entries.size(); ++i) {
                const auto& entry = menu.context->entries[i];
                const auto& item = entry.item;
                const auto r = context_item_rect(menu, i);
                if (item.kind == MenuItemKind::separator) {
                    line(canvas, r.x + 8, r.y + r.height / 2, r.x + std::max(8., r.width - 8), r.y + r.height / 2, theme.border, 1);
                    continue;
                }
                const auto ink = menu_selectable(entry) ? theme.text : theme.disabled;
                if (static_cast<int>(i) == menu.highlighted) box(canvas, r, theme.inactive, 3);
                if (entry.checked) {
                    if (item.kind == MenuItemKind::radio) box(canvas, {r.x + 10, r.y + r.height / 2 - 3, 6, 6}, theme.primary, 3);
                    else checkmark(canvas, r.x + 3, r.y + (r.height - 20) / 2, theme.primary);
                }
                const auto trailing = item.kind == MenuItemKind::submenu ? 24. :
                    item.shortcut.empty() ? 12. : std::min(r.width * .45, fonts.width(*owner, item.shortcut) + 20);
                caption(canvas, *owner, item.label, {r.x + 30, r.y, std::max(0., r.width - 30 - trailing), r.height}, ink);
                if (item.kind == MenuItemKind::submenu) {
                    const auto x = r.x + r.width - 12, y = r.y + r.height / 2;
                    line(canvas, x - 3, y - 4, x + 1, y, ink, 1.5);
                    line(canvas, x + 1, y, x - 3, y + 4, ink, 1.5);
                } else if (!item.shortcut.empty())
                    caption(canvas, *owner, item.shortcut, {r.x + r.width - trailing, r.y, std::max(0., trailing - 8), r.height}, theme.secondary, Align::end);
            }
            canvas.restore();
        }
    }
    void position_popup(bool reveal_highlight = false) {
        if (!menu_popup())
            return;
        auto* n = find(menu_popup()->owner);
        if (!n) {
            drop_menu();
            return;
        }
        double width = n->bounds.width;
        for (auto& o : menu_popup()->options)
            width = std::max(width, fonts.width(*n, o.label) + 48 + theme.popup_padding * 2);
        width = std::min(width, viewport.width);
        double h = menu_popup()->options.size() * theme.popup_item_height + 2 * theme.popup_padding;
        double below = std::max(0., viewport.height - n->bounds.y - n->bounds.height - theme.popup_offset),
               above = std::max(0., n->bounds.y - theme.popup_offset);
        bool flip = h > below && above > below;
        h = std::min(h, flip ? above : below);
        // Tiny viewports still get a clipped, scrollable overlay within the viewport.
        if (h < theme.popup_item_height)
            h = std::min(viewport.height, std::max(theme.popup_item_height, h));
        menu_popup()->bounds = place_overlay(n->bounds, {width, h}, viewport,
            flip ? OverlayPlacement::above : OverlayPlacement::below, theme.popup_offset, 0);
        double max_scroll = std::max(0., menu_popup()->options.size() * theme.popup_item_height -
                                             std::max(0., h - 2 * theme.popup_padding));
        menu_popup()->scroll = std::clamp(menu_popup()->scroll, 0., max_scroll);
        if (reveal_highlight && menu_popup()->highlighted >= 0) {
            double top = menu_popup()->highlighted * theme.popup_item_height, bottom = top + theme.popup_item_height,
                   visible = std::max(0., h - 2 * theme.popup_padding);
            if (top < menu_popup()->scroll)
                menu_popup()->scroll = top;
            if (bottom > menu_popup()->scroll + visible)
                menu_popup()->scroll = std::min(max_scroll, bottom - visible);
        }
    }
    void layout(Size size) {
        check();
        if (!std::isfinite(size.width) || !std::isfinite(size.height) || size.width < 0 || size.height < 0)
            throw std::invalid_argument("Viewport must be finite and nonnegative");
        bool changed =
            dirty || geometry_dirty || size.width != viewport.width || size.height != viewport.height;
        viewport = size;
        if (dirty)
            rebuild();
        if (root && changed) {
            measure(*root, {0, 0}, size);
            arrange_node(*root, {0, 0, size.width, size.height}, {0, 0, size.width, size.height});
        }
        geometry_dirty = false;
        for (;;) {
            position_overlays();
            const bool missing = std::any_of(overlays.begin(), overlays.end(), [&](const auto& o) {
                return o.kind != OverlayKind::menu && !o.context && !find(o.root_id);
            });
            if (requested_overlays.empty() && !missing) break;
            reconcile_overlays();
        }
        animating = std::any_of(nodes.begin(), nodes.end(), [](const auto& entry) {
            const auto* n = entry.second;
            return n->d.kind == Kind::progress && n->d.indeterminate && !n->hidden && n->clip.width > 0 && n->clip.height > 0;
        });
        position_popup();
        position_contexts();
        update_overlay_lifecycle();
        if (changed && inspection_observer)
            inspection_observer();
    }
    Color fill(const Node& n, Color normal) const {
        if (!n.enabled)
            return theme.inactive;
        if (pressed == n.id)
            return brighten(normal, .78);
        if (hovered == n.id)
            return brighten(normal, 1.12);
        return normal;
    }
    void checkmark(SkCanvas& c, double x, double y, Color color) const {
        line(c, x + 4, y + 10, x + 8, y + 14, color, 2.5);
        line(c, x + 8, y + 14, x + 16, y + 5, color, 2.5);
    }
    void chevron(SkCanvas& c, double x, double y, Color color, bool up = false) const {
        line(c, x - 4, y + (up ? 2 : -2), x, y + (up ? -2 : 2), color, 2);
        line(c, x, y + (up ? -2 : 2), x + 4, y + (up ? 2 : -2), color, 2);
    }
    void caption(SkCanvas& c, const Node& n, std::string s, Rect r, Color color,
                 Align align = Align::start) const {
        if (r.width <= 0)
            return;
        if (fonts.width(n, s) > r.width) {
            while (!s.empty() && fonts.width(n, s + "…") > r.width) {
                std::size_t previous = 0;
                for (std::size_t i = 0; i < s.size();) {
                    previous = i;
                    i = next_utf8(s, i);
                }
                s.resize(previous);
            }
            s += "…";
        }
        double x = r.x + (align == Align::center ? std::max(0., (r.width - fonts.width(n, s)) / 2)
                          : align == Align::end  ? std::max(0., r.width - fonts.width(n, s))
                                                 : 0),
               y = r.y + (r.height - n.line_height) / 2 + fonts.ascent(n);
        c.save();
        c.clipRect(SkRect::MakeXYWH(r.x, r.y, r.width, r.height));
        fonts.draw(c, n, s, x, y, color);
        c.restore();
    }
    bool reveal_item(std::string id, std::size_t index) {
        auto* list = find(id);
        if (!list || list->d.kind != Kind::listbox || index >= list->d.rows.size())
            return false;
        const auto key = list->d.rows.at(index).id;
        const auto row_id = id + "/option/" + path_key(key);
        // Layout observers may replace the snapshot. Follow identity, and bound retries
        // when a caller keeps moving the target on every notification.
        for (unsigned attempt = 0; attempt < 4; ++attempt) {
            layout(viewport);
            list = find(id);
            if (!list || list->d.kind != Kind::listbox || list->hidden || !list->enabled ||
                !in_focus_scope(*list))
                return false;
            const auto current = list->d.rows.index_of(key);
            if (!current)
                return false;
            const auto height = list->d.list_options.row_height;
            const auto viewport_height = content_box(*list).height;
            auto& offset = scroll[id];
            const auto start = *current * height;
            const auto before = offset.y;
            if (start < offset.y)
                offset.y = start;
            else if (start + height > offset.y + viewport_height)
                offset.y = start + height - viewport_height;
            offset.y = std::clamp(offset.y, 0., list->scroll_max.y);
            if (offset.y != before || !find(row_id)) {
                geometry_dirty = true;
                layout(viewport);
                notify();
            }
            list = find(id);
            if (dirty || !list || list->d.kind != Kind::listbox || list->d.rows.index_of(key) != current)
                continue;
            if (auto* row = find(row_id))
                reveal(*row);
            list = find(id);
            if (!dirty && list && list->d.kind == Kind::listbox && !list->hidden && list->enabled &&
                in_focus_scope(*list) && list->d.rows.index_of(key) == current)
                return find(row_id) != nullptr;
        }
        return false;
    }
    std::optional<std::pair<Node*, std::size_t>> logical_row(const std::string& row_id) const {
        std::optional<std::pair<Node*, std::size_t>> result;
        for (const auto& [id, state] : lists) {
            const auto prefix = id + "/option/";
            if (!row_id.starts_with(prefix) || row_id.find('/', prefix.size()) != std::string::npos) continue;
            auto* list = find(id);
            if (!list || list->d.kind != Kind::listbox) continue;
            const auto key = unpath_key(row_id.substr(prefix.size()));
            const auto index = key ? list->d.rows.index_of(*key) : std::optional<std::size_t>{};
            if (index && (!result || id.size() > result->first->id.size())) result = {{list, *index}};
        }
        return result;
    }
    std::optional<LogicalItem> logical_item(Node& list, std::size_t index) const {
        if (index >= list.d.rows.size()) return {};
        const auto& option = list.d.rows.at(index);
        const auto& state = lists.at(list.id);
        LogicalItem item;
        item.id = list.id + "/option/" + path_key(option.id); item.label = option.label; item.group = list.id;
        item.index = index; item.count = list.d.rows.size();
        if(list.d.table)item.column_count=list.d.table->model.columns().size();
        item.enabled = list.enabled && !list.hidden && option.enabled && in_focus_scope(list);
        item.selected = !(list.d.table && list.d.table->options.selection_mode==TableSelectionMode::cells) && state.selected.contains(option.id); item.active = state.active == option.id;
        if (list.d.tree) {
            item.tree = trees.at(list.id).flat.at(index);
            if (!item.tree->accessible_label.empty()) item.label = item.tree->accessible_label;
        }
        item.focused = active && !menu_popup() && !context_popup() && in_focus_scope(list) && focused == list.id && item.active;
        if(list.d.table&&list.d.table->options.selection_mode==TableSelectionMode::cells)item.focused=false;
        if (const auto* row = find(item.id)) { item.bounds = row->bounds; item.clip = row->clip; }
        return item;
    }
    bool tree_expand(std::string id, std::string key, bool expanded) {
        auto* list = find(id);
        if (!list || !list->d.tree || !interactive(*list) || !in_focus_scope(*list)) return false;
        const auto index = list->d.rows.index_of(key);
        if (!index) return false;
        const auto item = trees.at(id).flat.at(*index);
        if (!item.enabled || !item.expandable) return false;
        const auto config = *list->d.tree;
        auto values = config.expanded.get();
        const bool present = std::find(values.begin(), values.end(), key) != values.end();
        if (present == expanded) return true;
        if (!expanded) {
            auto current = config.model.item(lists.at(id).active);
            while (current && !current->parent.empty()) {
                if (current->parent == key) { lists.at(id).active = key; break; }
                current = config.model.item(current->parent);
            }
            std::erase(values, key);
        } else values.push_back(key);
        config.expanded.set(std::move(values));
        invalidate();
        if (config.options.on_expand) config.options.on_expand(key, expanded);
        layout(viewport);
        if (auto* current = find(id); current && current->d.tree)
            if (const auto active = current->d.rows.index_of(lists.at(id).active)) reveal_item(id, *active);
        notify();
        return true;
    }
    bool list_select(Node& original, std::string option_id, bool extend = false, bool toggle = false) {
        if(original.d.table && original.d.table->options.selection_mode==TableSelectionMode::cells)
            return table_select(original.id,{option_id,tables.at(original.id).column},extend,toggle);
        const auto id = original.id;
        if (focused != id) { finish_editing(); layout(viewport); }
        auto* current = find(id);
        if (!current || current->d.kind != Kind::listbox) return false;
        const auto description = current->d;
        const auto model = description.rows;
        const auto index = model.index_of(option_id);
        if (!interactive(*current) || !in_focus_scope(*current) || !index || !model.at(*index).enabled) return false;
        auto& state = lists.at(id);
        if (state.anchor.empty()) state.anchor = state.active.empty() ? option_id : state.active;
        state.active = option_id;
        if (description.write_selected) {
            auto selected = description.read_selected();
            std::unordered_set<std::string> ids(selected.begin(), selected.end());
            if (extend) {
                const auto anchor = model.index_of(state.anchor).value_or(*index);
                if (!toggle) ids.clear();
                for (auto i = std::min(anchor, *index); i <= std::max(anchor, *index); ++i)
                    if (model.at(i).enabled) ids.insert(model.at(i).id);
            } else if (toggle) {
                if (!ids.erase(option_id)) ids.insert(option_id);
                state.anchor = option_id;
            } else { ids = {option_id}; state.anchor = option_id; }
            std::vector<std::pair<std::size_t, std::string>> ordered;
            for (const auto& key : ids) if (auto i = model.index_of(key)) ordered.emplace_back(*i, key);
            std::sort(ordered.begin(), ordered.end()); selected.clear();
            for (auto& [i, key] : ordered) selected.push_back(std::move(key));
            description.write_selected(std::move(selected));
        } else if (description.write_selection) {
            if (!extend) state.anchor = option_id;
            description.write_selection(option_id);
        }
        focused = id;
        invalidate(); layout(viewport);
        if (auto* list = find(id); list && list->d.kind == Kind::listbox)
            if (auto i = list->d.rows.index_of(option_id)) reveal_item(id, *i);
        notify(); return true;
    }
    bool list_key(Node& n, const KeyEvent& e) {
        const auto model = n.d.rows;
        const auto enabled = model.enabled_indices();
        auto& state = lists.at(n.id);
        const auto index = model.index_of(state.active);
        if (n.d.tree && index && (e.key == Key::left || e.key == Key::right)) {
            const auto item = trees.at(n.id).flat.at(*index);
            if (e.key == Key::right) {
                if (item.expandable && !item.expanded) return tree_expand(n.id, item.id, true);
                const auto& flat = trees.at(n.id).flat;
                for (auto next = *index + 1; next < flat.size() && flat[next].depth > item.depth; ++next)
                    if (flat[next].parent == item.id && flat[next].enabled) return list_select(n, flat[next].id, e.shift, e.command);
            } else {
                if (item.expandable && item.expanded) return tree_expand(n.id, item.id, false);
                if (!item.parent.empty()) return list_select(n, item.parent, e.shift, e.command);
            }
            return true;
        }
        if (e.key == Key::a && e.command && n.d.write_selected) {
            std::vector<std::string> all;
            for (const auto i : enabled) all.push_back(model.at(i).id);
            const auto write = n.d.write_selected; write(std::move(all)); invalidate(); return true;
        }
        if (e.key == Key::enter) {
            if (index && n.d.list_options.on_activate) { const auto fn = n.d.list_options.on_activate; const auto value = model.at(*index).id; fn(value); }
            else if (index && n.d.tree) {
                const auto item = trees.at(n.id).flat.at(*index);
                if (item.expandable) return tree_expand(n.id, item.id, !item.expanded);
            }
            return true;
        }
        if (e.key == Key::space && index) return list_select(n, model.at(*index).id, e.shift, bool(n.d.write_selected));
        if (enabled.empty()) return e.key == Key::home || e.key == Key::end ||
            e.key == Key::up || e.key == Key::down || e.key == Key::page_up || e.key == Key::page_down ||
            e.key == Key::space || (!e.text.empty() && !e.command && !e.control);
        std::size_t target = index.value_or(0);
        int direction = 1;
        if (e.key == Key::home) target = 0;
        else if (e.key == Key::end) { target = model.size() - 1; direction = -1; }
        else if (e.key == Key::up) {
            if (!index || *index == 0) return true;
            target = *index - 1; direction = -1;
        } else if (e.key == Key::down) {
            target = index ? *index + 1 : 0;
            if (target >= model.size()) return true;
        } else if (e.key == Key::page_up || e.key == Key::page_down) {
            const auto step = std::size_t(std::max(1., content_box(n).height / n.d.list_options.row_height));
            direction = e.key == Key::page_up ? -1 : 1;
            target = direction < 0 ? target > step ? target - step : 0 : std::min(model.size() - 1, target + step);
        } else if (!e.text.empty() && !e.command && !e.control) {
            const auto now = options.clock();
            const auto typed = detail::uppercase_text(e.text);
            if (now - state.typed_at > std::chrono::seconds(1) || state.query == typed) state.query.clear();
            state.typed_at = now; state.query += typed;
            const auto start = index.value_or(model.size() - 1);
            for (std::size_t step = 1; step <= model.size(); ++step) {
                const auto& option = model.at((start + step) % model.size());
                if (option.enabled && detail::uppercase_text(option.label).starts_with(state.query)) return list_select(n, option.id);
            }
            return true;
        } else return false;
        const auto found = direction > 0 ? std::lower_bound(enabled.begin(), enabled.end(), target) : std::upper_bound(enabled.begin(), enabled.end(), target);
        if (direction > 0) { if (found != enabled.end()) return list_select(n, model.at(*found).id, e.shift, e.command); }
        else if (found != enabled.begin()) return list_select(n, model.at(*std::prev(found)).id, e.shift, e.command);
        return true;
    }
    Node* field() const {
        if (context_popup()) return nullptr;
        auto* n = find(focused);
        return n && n->d.kind == Kind::text_field && interactive(*n) ? n : nullptr;
    }
    Node* field_owner(const std::string& id) const {
        auto* n = find(id);
        if (n && !n->combo_owner.empty()) n = find(n->combo_owner);
        if (n && n->d.kind == Kind::field_action)
            n = find(n->group);
        return n && n->d.kind == Kind::text_field ? n : nullptr;
    }
    Rect field_action_rect(const Node& n) const {
        const auto base = inset(n.bounds, n.d.padding);
        const auto width = std::min(base.width, n.d.password ? 48. : 28.);
        return {base.x + base.width - width, base.y, width, base.height};
    }
    Rect field_viewport(const Node& n) const {
        auto content = inset(n.bounds, n.d.padding);
        if (n.d.search) {
            const auto leading = std::min(24., content.width);
            content.x += leading;
            content.width -= leading;
        }
        if ((n.d.search && n.d.search->clear_button) || (n.d.password && n.d.password->allow_reveal) || n.d.combo || n.d.date || n.d.time)
            content.width = std::max(0., content.width - (n.d.password ? 56. : 36.));
        return content;
    }
    Rect field_caret(const Node& n) const {
        const auto& editor = *editors.at(n.id);
        const auto offset = editor.buffer.selection().caret;
        auto caret =
            n.text && n.text->caret_rect ? n.text->caret_rect(offset) : Rect{0, 0, 1.5, n.line_height};
        if (editor.upstream && n.text) {
            auto line = std::find_if(n.text->lines.begin(), n.text->lines.end(),
                                     [&](const auto& l) { return l.range.end() == offset; });
            if (line != n.text->lines.end())
                caret = {line->bounds.x + line->bounds.width, line->bounds.y, 1.5,
                         std::max(n.line_height, line->bounds.height)};
        }
        return caret;
    }
    void arrange_field(Node& n) {
        auto& editor = *editors.at(n.id);
        const auto content = field_viewport(n);
        const bool masked = n.d.password && !editor.revealed;
        n.text = fonts.layout(n, masked ? password_mask(editor.buffer.text()) : editor.buffer.text(),
                              {n.d.text_options.multiline ? content.width : infinity,
                               n.d.text_options.multiline ? TextWrap::wrap : TextWrap::no_wrap,
                               TextOverflow::clip, 0, n.d.text_align});
        if (masked)
            n.text = password_layout(n.text, grapheme_boundaries(editor.buffer.text()));
        const auto caret = field_caret(n);
        const double height = n.d.text_options.multiline ? content.height : n.line_height;
        if (focused == n.id && editor.reveal_caret) {
            if (caret.x < editor.scroll.x)
                editor.scroll.x = caret.x;
            if (caret.x + caret.width > editor.scroll.x + content.width)
                editor.scroll.x = caret.x + caret.width - content.width;
            if (caret.y < editor.scroll.y)
                editor.scroll.y = caret.y;
            if (caret.y + caret.height > editor.scroll.y + height)
                editor.scroll.y = caret.y + caret.height - height;
        }
        editor.reveal_caret = false;
        editor.scroll.x =
            std::clamp(editor.scroll.x, 0., std::max(0., n.text->size.width + 2 - content.width));
        editor.scroll.y = std::clamp(editor.scroll.y, 0., std::max(0., n.text->size.height - height));
        const double free_width = std::max(0., content.width - n.text->size.width);
        const double alignment = n.d.text_options.multiline        ? 0
                                 : n.d.text_align == Align::center ? free_width / 2
                                 : n.d.text_align == Align::end    ? free_width
                                                                   : 0;
        n.text_origin = {content.x + alignment - editor.scroll.x,
                         content.y + (n.d.text_options.multiline ? 0 : (content.height - n.line_height) / 2) -
                             editor.scroll.y};
        if (!n.children.empty() && n.children.front()->d.kind == Kind::field_action)
            arrange_node(*n.children.front(), field_action_rect(n), n.clip);
        if(n.d.time){
            auto current=*n.numeric_value;
            if(editor.validation.dirty){
                if(auto parsed=detail::parse_number(editor.buffer.committed_text(),*n.d.numeric);parsed.value)current=*parsed.value;
            }
            auto buttons=field_action_rect(n);
            for(std::size_t i=0;i<n.children.size();++i){
                auto& button=*n.children[i];
                const auto next=detail::step_number(current,button.number,*n.d.numeric,editor.buffer.selection().caret);
                button.enabled=n.enabled&&!n.d.time->read_only&&editor.validation.status==NumericStatus::valid&&next&&*next!=current;
                arrange_node(button,{buttons.x,buttons.y+i*buttons.height/2,buttons.width,buttons.height/2},n.clip);
            }
        }
    }
    void refresh_numeric(Node& n, bool signal) {
        auto& editor = *editors.at(n.id);
        NumericValidation validation;
        validation.dirty = editor.buffer.committed_text() != editor.numeric_base;
        if (validation.dirty) {
            auto parsed = detail::parse_number(editor.buffer.committed_text(), *n.d.numeric);
            validation = std::move(parsed.validation);
            validation.dirty = true;
        } else if (n.d.numeric->validate) {
            validation = n.d.numeric->validate(*editor.numeric_source);
        } else if (*editor.numeric_source < n.d.numeric->min || *editor.numeric_source > n.d.numeric->max) {
            validation.status = NumericStatus::out_of_range;
            validation.message = "Value is outside the configured range.";
        }
        const bool changed = editor.validation != validation;
        editor.validation = validation;
        auto callback = n.d.numeric->on_validation;
        if (signal && changed && callback)
            callback(validation);
    }
    bool flush_search(std::string id) {
        layout(viewport);
        auto* n = find(id);
        if (!active || !n || !n->d.search || !interactive(*n) || n->d.search->read_only)
            return false;
        auto& editor = *editors.at(id);
        if (!editor.pending_search || editor.pending_search->suspended || editor.buffer.marked_range())
            return false;
        const auto query = editor.pending_search->query;
        editor.pending_search.reset();
        const auto callback = n->d.search->on_search;
        if (query != n->d.read_text() || !callback)
            return false;
        callback(query);
        notify();
        return true;
    }
    bool clear_search(std::string id) {
        layout(viewport);
        auto* n = find(id);
        if (!n || !n->d.search || !interactive(*n) || n->d.search->read_only)
            return false;
        editors.at(id)->pending_search.reset();
        if (focused == id)
            editors.at(id)->buffer.unmark(false);
        if (!focus(id, false))
            return true;
        n = find(id);
        if (!n || !n->d.search || n->d.search->read_only)
            return true;
        auto& buffer = editors.at(id)->buffer;
        buffer.unmark(false);
        buffer.replace("", TextRange{0, buffer.text().size()});
        field_changed(*n, true);
        flush_search(id);
        return true;
    }
    bool toggle_password(std::string id) {
        layout(viewport);
        auto* n = find(id);
        if (!n || !n->d.password || !n->d.password->allow_reveal || !interactive(*n) || n->d.password->read_only)
            return false;
        if (!focus(id, false))
            return true;
        n = find(id);
        if (!n || !n->d.password || !n->d.password->allow_reveal || n->d.password->read_only)
            return true;
        auto& editor = *editors.at(id);
        editor.revealed = !editor.revealed;
        editor.reveal_caret = true;
        invalidate();
        return true;
    }
    void search_changed(std::string id, bool write, std::string value) {
        auto* n = find(id);
        if (!n || !n->d.search)
            return;
        if (write) {
            const auto writer = n->d.write_text;
            writer(std::move(value));
            invalidate();
            layout(viewport);
            n = find(id);
            if (!active || !n || !n->d.search || !interactive(*n) || n->d.search->read_only)
                return;
            if (n->d.search->on_search)
                editors.at(id)->pending_search = Editing::Search{n->d.read_text(), options.clock() + n->d.search->debounce};
        } else {
            auto& editor = *editors.at(id);
            if (editor.pending_search) {
                if (editor.buffer.marked_range())
                    editor.pending_search->suspended = true;
                else if (editor.pending_search->suspended) {
                    editor.pending_search->suspended = false;
                    editor.pending_search->due = options.clock() + n->d.search->debounce;
                }
            }
        }
        if (n && n->d.search && n->d.search->debounce.count() == 0)
            flush_search(id);
    }
    void begin_combo(const std::string& id) {
        auto& state = combos.at(id);
        if (!state.session) {
            state.base_query = state.read_query(); state.base_selection = state.read_selection(); state.session = true;
        }
    }
    void write_combo(std::string id, std::string query, std::string selection, bool complete) {
        auto* node = find(id);
        if (!node || !node->d.combo) return;
        const auto read_query = node->d.read_text, read_selection = node->d.read_selection;
        const auto write_query = node->d.write_text, write_selection = node->d.write_selection;
        auto& state = combos.at(id);
        state.writing = true;
        if (complete) { state.open = false; state.session = false; state.validation.clear(); }
        try {
            if (read_selection() != selection) write_selection(selection);
            if (read_query() != query) write_query(query);
        } catch (...) {
            if (auto it = combos.find(id); it != combos.end()) it->second.writing = false;
            throw;
        }
        if (auto it = combos.find(id); it != combos.end()) {
            auto& current = it->second;
            current.source_query = read_query(); current.source_selection = read_selection(); current.writing = false;
            if (complete) { current.base_query = current.source_query; current.base_selection = current.source_selection; }
        }
        invalidate(); layout(viewport);
    }
    bool accept_combo(std::string id, std::string option = {}) {
        auto* node = find(id);
        if (!node || !node->d.combo || !interactive(*node) || node->d.combo->read_only || node->d.combo->loading) return false;
        const auto config = *node->d.combo;
        const auto read_query = node->d.read_text, read_selection = node->d.read_selection;
        const auto matches = combos.at(id).matches;
        const auto query = node->d.read_text();
        std::string selected;
        std::string accepted = query;
        if (option.empty()) option = combos.at(id).highlighted;
        const auto found = std::find_if(matches.begin(), matches.end(), [&](const auto& item) { return item.id == option && item.enabled; });
        if (found != matches.end()) { selected = found->id; accepted = found->label; }
        else if (config.mode == ComboBoxMode::selection) {
            combos.at(id).validation = "Choose an available suggestion."; notify(); return false;
        }
        write_combo(id, std::move(accepted), std::move(selected), true);
        node = find(id);
        if (node && node->d.combo) {
            auto& editor = *editors.at(id);
            editor.buffer.select({editor.buffer.text().size(), editor.buffer.text().size()});
            editor.reveal_caret = true; geometry_dirty = true;
        }
        if (config.on_accept) {
            const auto selection = read_selection(), value = read_query();
            config.on_accept(selection, value);
        }
        return true;
    }
    void finish_combo(std::string id, bool cancel) {
        const auto* node = find(id);
        const auto it = combos.find(id);
        if (!node || !node->d.combo || it == combos.end()) return;
        const auto state = it->second;
        if (!state.session && !state.open) return;
        const auto callback = cancel ? node->d.combo->on_cancel : std::function<void()>{};
        const bool restore = cancel || state.mode == ComboBoxMode::selection;
        const auto query = restore ? state.base_query : state.read_query();
        const auto selected = restore ? state.base_selection :
            state.selected_label && query == *state.selected_label ? state.read_selection() : std::string{};
        write_combo(id, query, selected, true);
        if (callback) callback();
    }
    bool open_combo(std::string id, bool toggle = false) {
        auto* node = find(id);
        if (!node || !node->d.combo || !interactive(*node) || node->d.combo->read_only) return false;
        if (!focus(id, false)) return false;
        unmark_text(true); layout(viewport);
        node = find(id);
        if (!node || !node->d.combo) return false;
        begin_combo(id);
        auto& state = combos.at(id);
        state.open = toggle ? !state.open : true;
        state.reveal = true;
        state.validation.clear(); hide_tooltip(); invalidate(); layout(viewport);
        return true;
    }
    bool combo_navigate(std::string id, int delta) {
        const bool was_open = combos.at(id).open;
        if (!was_open && !open_combo(id)) return false;
        auto& state = combos.at(id);
        const auto count = static_cast<int>(state.matches.size());
        if (count == 0) return true;
        int current = -1;
        for (int i = 0; i < count; ++i) if (state.matches[i].id == state.highlighted) current = i;
        if (!was_open) current = delta < 0 ? count : -1;
        const auto direction = delta < 0 ? -1 : 1;
        const int steps = std::max(1, std::abs(delta));
        for (int step = 0; step < steps; ++step) {
            int candidate = current;
            bool found = false;
            for (int n = 0; n < count; ++n) {
                candidate = std::clamp(candidate + direction, 0, count - 1);
                if (state.matches[candidate].enabled) { current = candidate; found = true; break; }
                if (candidate == 0 || candidate == count - 1) break;
            }
            if (!found) break;
        }
        if (current >= 0 && current < count) state.highlighted = state.matches[current].id;
        state.reveal = true;
        const auto highlighted = state.highlighted;
        invalidate(); layout(viewport);
        if (auto* row = find(id + "/suggestions/option/" + path_key(highlighted))) reveal(*row);
        notify(); return true;
    }
    void combo_changed(std::string id, bool commit, std::string value) {
        auto* node = find(id);
        if (!node || !node->d.combo) return;
        const auto current = node->d.read_text();
        if (commit && current != value) {
            begin_combo(id);
            auto& state = combos.at(id); state.open = true; state.validation.clear();
            const auto selected = node->d.read_selection();
            const auto callback = node->d.combo->on_query;
            write_combo(id, std::move(value), selected, false);
            node = find(id);
            if (callback && node && node->d.combo) callback(node->d.read_text());
        } else if (combos.at(id).open || editors.at(id)->buffer.marked_range()) { invalidate(); }
        notify();
    }
    void field_changed(Node& n, bool commit, bool reset_x = true) {
        auto& editor = *editors.at(n.id);
        editor.reset_blink = true;
        editor.reveal_caret = true;
        editor.upstream = false;
        if (reset_x)
            editor.desired_x.reset();
        geometry_dirty = true;
        ++generation;
        if (n.d.password && !n.d.password->allow_undo)
            editor.buffer.clear_history();
        if (n.d.numeric) {
            const bool composite=n.datetime_component||!n.color_owner.empty();
            refresh_numeric(n, commit);
            if(composite)invalidate();
            notify();
            return;
        }
        auto writer = n.d.write_text;
        const auto value = editor.buffer.committed_text();
        if (n.d.combo) { combo_changed(n.id, commit, value); return; }
        if (n.d.search) {
            const auto id = n.id;
            const bool changed = commit && n.d.read_text() != value;
            search_changed(id, changed, value);
            notify();
            return;
        }
        if (commit && n.d.read_text() != value) {
            writer(value);
            invalidate();
        }
        notify();
    }
    bool commit_numeric(std::string id) {
        layout(viewport);
        auto* n = find(id);
        if (!n || !n->d.numeric || n->d.kind != Kind::text_field || !interactive(*n) || n->d.numeric->read_only)
            return false;
        if (focused == id)
            unmark_text(true);
        layout(viewport);
        n = find(id);
        if (!n || !n->d.numeric || n->d.kind != Kind::text_field || !interactive(*n) || n->d.numeric->read_only)
            return false;
        refresh_numeric(*n, true);
        layout(viewport);
        n = find(id);
        if (!n || !n->d.numeric || n->d.kind != Kind::text_field || !interactive(*n) || n->d.numeric->read_only)
            return false;
        auto& editor = *editors.at(id);
        if (editor.validation.status != NumericStatus::valid)
            return false;
        if (!editor.validation.dirty)
            return true;
        const auto parsed = detail::parse_number(editor.buffer.committed_text(), *n->d.numeric);
        if (!parsed.value)
            return false;
        write_numeric_value(*n, *parsed.value);
        return true;
    }
    void finish_editing() {
        layout(viewport);
        if(auto* current=field();current&&!current->table_owner.empty()){
            if(table_cancel(current->table_owner))return;
        }
        auto* n = field_owner(focused);
        if (!n)
            return;
        const auto id = n->id;
        const bool numeric = n->d.numeric && n->d.numeric->commit_on_blur;
        const bool password = n->d.password.has_value();
        const bool combo = n->d.combo.has_value();
        unmark_text(true);
        if (combo) finish_combo(id, false);
        if (numeric)
            commit_numeric(id);
        if (password) {
            if (auto it = editors.find(id); it != editors.end()) {
                const bool was_revealed = it->second->revealed;
                it->second->revealed = false;
                it->second->buffer.clear_history();
                if (was_revealed)
                    invalidate();
            }
        }
    }
    bool cancel_numeric(Node& n) {
        auto& editor = *editors.at(n.id);
        const auto current = n.d.read_numeric();
        editor.numeric_source = current;
        editor.numeric_base = detail::format_number(current, *n.d.numeric);
        editor.buffer.set_text(editor.numeric_base);
        field_changed(n, true);
        return true;
    }
    std::string field_insert(Node& n, std::string inserted, std::optional<TextRange> range) {
        utf16_from_utf8(inserted); // Validate before changing history or composition.
        std::string clean;
        for (std::size_t i = 0; i < inserted.size(); ++i) {
            auto c = static_cast<unsigned char>(inserted[i]);
            if (c == '\r') {
                if (i + 1 < inserted.size() && inserted[i + 1] == '\n')
                    ++i;
                c = '\n';
            }
            if (c == '\n' || c == '\t')
                clean += n.d.text_options.multiline ? char(c) : ' ';
            else if (c >= 32 && c != 127)
                clean += char(c);
        }
        if (n.d.text_options.max_length) {
            const auto& buffer = editors.at(n.id)->buffer;
            auto selected = range.value_or(buffer.marked_range().value_or(buffer.selection().range()));
            const auto& value = buffer.text();
            selected.start = std::min(selected.start, value.size());
            selected.length = std::min(selected.length, value.size() - selected.start);
            const auto boundaries = grapheme_boundaries(value);
            const auto end = std::lower_bound(boundaries.begin(), boundaries.end(), selected.end());
            const bool empty = selected.length == 0;
            selected.start = buffer.normalize(selected.start);
            selected.length = empty ? 0 : (end == boundaries.end() ? value.size() : *end) - selected.start;
            const auto existing = grapheme_boundaries(value).size() - 1;
            const auto removed =
                grapheme_boundaries(value.substr(selected.start, selected.length)).size() - 1;
            const auto available = n.d.text_options.max_length > existing - removed
                                       ? n.d.text_options.max_length - (existing - removed)
                                       : 0;
            const auto incoming = grapheme_boundaries(clean);
            if (incoming.size() - 1 > available)
                clean.resize(incoming[available]);
        }
        return clean;
    }
    bool insert_text(std::string inserted, std::optional<TextRange> range, bool coalesce = true) {
        layout(viewport);
        auto* n = field();
        if (!n || n->d.text_options.read_only)
            return false;
        inserted = field_insert(*n, std::move(inserted), range);
        editors.at(n->id)->buffer.replace(std::move(inserted), range, coalesce);
        field_changed(*n, true);
        return true;
    }
    void unmark_text(bool commit) {
        auto* n = field();
        if (!n)
            return;
        auto& buffer = editors.at(n->id)->buffer;
        if (!buffer.marked_range())
            return;
        if (commit && n->d.text_options.max_length) {
            auto range = *buffer.marked_range();
            auto inserted = buffer.text().substr(range.start, range.length);
            insert_text(std::move(inserted), range, false);
            return;
        }
        buffer.unmark(commit);
        field_changed(*n, commit);
    }
    bool text_command(TextCommand command, bool extend) {
        layout(viewport);
        auto* n = field();
        if (!n)
            return false;
        auto& editor = *editors.at(n->id);
        auto& buffer = editor.buffer;
        if (n->d.search && buffer.marked_range() &&
            (command == TextCommand::left || command == TextCommand::right || command == TextCommand::up ||
             command == TextCommand::down || command == TextCommand::page_up || command == TextCommand::page_down ||
             command == TextCommand::line_start || command == TextCommand::line_end)) {
            const auto id = n->id;
            unmark_text(true);
            layout(viewport);
            return focused == id && field() ? text_command(command, extend) : true;
        }
        const auto selected = buffer.selection().range();
        if (command == TextCommand::copy || command == TextCommand::cut) {
            if (n->d.password)
                return true;
            if (command == TextCommand::cut && n->d.text_options.read_only)
                return true;
            if (clipboard.write && selected.length)
                clipboard.write(buffer.text().substr(selected.start, selected.length));
            if (command == TextCommand::copy || n->d.text_options.read_only)
                return true;
            buffer.replace("");
            field_changed(*n, true);
            return true;
        }
        if (command == TextCommand::paste) {
            if (n->d.text_options.read_only)
                return true;
            if (!clipboard.read)
                return true;
            auto text = clipboard.read();
            return text.empty() ? true : insert_text(std::move(text), {}, false);
        }
        if (command == TextCommand::submit) {
            if(!n->table_owner.empty()&&tables.contains(n->table_owner)&&tables.at(n->table_owner).edit){const auto id=n->table_owner;if(buffer.marked_range())unmark_text(true);else table_commit(id);return true;}
            if ((n->d.search || n->d.password || n->d.combo || n->d.date || n->d.time || !n->color_owner.empty()) && buffer.marked_range()) {
                unmark_text(true);
                return true;
            }
            if (n->d.search) {
                const auto id = n->id;
                unmark_text(true);
                layout(viewport);
                n = find(id);
                if (!n || !n->d.search)
                    return true;
                const auto query = n->d.read_text();
                const auto callback = n->d.search->on_submit;
                flush_search(id);
                if (callback)
                    callback(query);
                return true;
            }
            if (n->d.combo) { accept_combo(n->id); return true; }
            if (n->d.numeric) {
                const auto id = n->id;
                const auto callback = n->d.numeric->on_submit;
                const auto owner=n->datetime_component?n->datetime_owner:std::string{};
                if ((owner.empty()?commit_numeric(id):commit_datetime(owner)) && callback)
                    callback();
                return true;
            }
            if (auto submit = n->d.text_options.on_submit) {
                unmark_text(true);
                submit();
            } else if (buffer.marked_range()) {
                unmark_text(true);
            } else if (!n->d.text_options.multiline) {
                alert_response(false);
            }
            return true;
        }
        if(command==TextCommand::cancel&&!n->table_owner.empty()&&tables.contains(n->table_owner)&&tables.at(n->table_owner).edit&&!buffer.marked_range()){
            table_cancel(n->table_owner);layout(viewport);notify();return true;
        }
        if (n->d.password && !n->d.password->allow_undo && (command == TextCommand::undo || command == TextCommand::redo))
            return true;
        const bool navigation =
            command == TextCommand::left || command == TextCommand::right ||
            command == TextCommand::word_left || command == TextCommand::word_right ||
            command == TextCommand::line_start || command == TextCommand::line_end ||
            command == TextCommand::document_start || command == TextCommand::document_end ||
            command == TextCommand::paragraph_start || command == TextCommand::paragraph_end ||
            command == TextCommand::up || command == TextCommand::down || command == TextCommand::page_up ||
            command == TextCommand::page_down || command == TextCommand::select_all ||
            command == TextCommand::cancel;
        if (!navigation && n->d.text_options.read_only)
            return true;
        if (n->d.combo && !buffer.marked_range()) {
            if (command == TextCommand::cancel) { finish_combo(n->id, true); return true; }
            if (command == TextCommand::up || command == TextCommand::down || command == TextCommand::page_up || command == TextCommand::page_down)
                return combo_navigate(n->id, command == TextCommand::up ? -1 : command == TextCommand::down ? 1 : command == TextCommand::page_up ? -5 : 5);
        }
        if (command == TextCommand::cancel && n->d.numeric && !buffer.marked_range()) {
            if(n->datetime_component&&!editor.validation.dirty&&cancel_datetime(n->datetime_owner))return true;
            if(!n->color_owner.empty()&&!editor.validation.dirty&&color_cancel(n->color_owner))return true;
            return cancel_numeric(*n);
        }
        if (command == TextCommand::cancel && n->d.search && !buffer.marked_range()) {
            if (!buffer.text().empty())
                return n->d.search->read_only || clear_search(n->id);
            editor.pending_search.reset();
            const auto callback = n->d.search->on_cancel;
            if (callback)
                callback();
            return true;
        }
        if (command == TextCommand::cancel && !buffer.marked_range() && !menu_popup() &&
            !context_popup() && !interactive_overlay() && alert_response(true))
            return true;
        if (n->d.numeric && !extend && !buffer.marked_range() &&
            (command == TextCommand::up || command == TextCommand::down ||
             command == TextCommand::page_up || command == TextCommand::page_down)) {
            const auto id = n->id;
            const double steps = command == TextCommand::up ? 1 : command == TextCommand::down ? -1 :
                                 command == TextCommand::page_up ? 10 : -10;
            if (commit_numeric(id))
                adjust(id, steps);
            return true;
        }
        if ((command == TextCommand::left || command == TextCommand::right) && n->text &&
            n->text->caret_rect) {
            unmark_text(true);
            const auto selection = buffer.selection();
            if (editor.upstream && command == TextCommand::right && !extend) {
                field_changed(*n, false);
                return true;
            }
            struct Stop {
                std::size_t offset;
                Rect rect;
            };
            std::vector<Stop> stops;
            for (const auto offset : grapheme_boundaries(buffer.text()))
                stops.push_back(
                    {offset, offset == selection.caret ? field_caret(*n) : n->text->caret_rect(offset)});
            std::stable_sort(stops.begin(), stops.end(), [](const auto& a, const auto& b) {
                const auto ay = std::llround(a.rect.y * 2), by = std::llround(b.rect.y * 2);
                if (ay != by)
                    return ay < by;
                return a.rect.x < b.rect.x;
            });
            auto current = std::find_if(stops.begin(), stops.end(),
                                        [&](const auto& stop) { return stop.offset == selection.caret; });
            if (current != stops.end()) {
                auto next = current;
                if (!extend && selection.range().length) {
                    auto anchor = std::find_if(stops.begin(), stops.end(), [&](const auto& stop) {
                        return stop.offset == selection.anchor;
                    });
                    if (anchor != stops.end())
                        next = command == TextCommand::left ? std::min(anchor, current)
                                                            : std::max(anchor, current);
                } else if (command == TextCommand::left && current != stops.begin())
                    --next;
                else if (command == TextCommand::right && std::next(current) != stops.end())
                    ++next;
                buffer.select({extend ? selection.anchor : next->offset, next->offset});
                field_changed(*n, true);
                return true;
            }
        }
        if ((command == TextCommand::up || command == TextCommand::down || command == TextCommand::page_up ||
             command == TextCommand::page_down) &&
            n->text && n->text->caret_rect && n->text->hit_test) {
            unmark_text(true);
            const auto selection = buffer.selection();
            const auto caret = field_caret(*n);
            if (!editor.desired_x)
                editor.desired_x = caret.x;
            const bool up = command == TextCommand::up || command == TextCommand::page_up;
            const double distance = command == TextCommand::page_up || command == TextCommand::page_down
                                        ? std::max(n->line_height, field_viewport(*n).height)
                                        : std::max(n->line_height, caret.height);
            const double y = caret.y + caret.height / 2 + (up ? -distance : distance);
            const auto next = y < 0                       ? 0
                              : y >= n->text->size.height ? buffer.text().size()
                                                          : n->text->hit_test({*editor.desired_x, y});
            buffer.select({extend ? selection.anchor : next, next});
            field_changed(*n, true, false);
            return true;
        }
        if ((command == TextCommand::line_start || command == TextCommand::line_end ||
             command == TextCommand::delete_line_backward || command == TextCommand::delete_line_forward) &&
            n->text && !n->text->lines.empty()) {
            unmark_text(true);
            const auto selection = buffer.selection();
            const auto caret = field_caret(*n);
            auto line = std::find_if(n->text->lines.begin(), n->text->lines.end(), [&](const auto& l) {
                return caret.y + caret.height / 2 >= l.bounds.y &&
                       caret.y + caret.height / 2 < l.bounds.y + l.bounds.height + .1;
            });
            if (line != n->text->lines.end()) {
                const auto next =
                    command == TextCommand::line_start || command == TextCommand::delete_line_backward
                        ? line->range.start
                        : line->range.end();
                if (command == TextCommand::delete_line_backward ||
                    command == TextCommand::delete_line_forward)
                    buffer.replace(
                        "", TextRange{std::min(next, selection.caret),
                                      std::max(next, selection.caret) - std::min(next, selection.caret)});
                else
                    buffer.select({extend ? selection.anchor : next, next});
                field_changed(*n, true);
                if (command == TextCommand::line_end && next == line->range.end())
                    editor.upstream = true;
                return true;
            }
        }
        buffer.command(command, extend);
        field_changed(*n, true);
        return true;
    }
    void paint_field(SkCanvas& canvas, Node& n, double time) {
        auto& editor = *editors.at(n.id);
        auto& buffer = editor.buffer;
        const auto r = n.bounds, content = field_viewport(n);
        box(canvas, r, n.d.background.value_or(theme.field), n.d.radius.value_or(6));
        box(canvas, r, (n.d.numeric && editor.validation.status != NumericStatus::valid) ||
                    (n.d.combo && !combos.at(n.id).validation.empty()) ? theme.error :
                       focused == n.id && active ? theme.cyan : theme.border, n.d.radius.value_or(6),
            focused == n.id && active ? 1.5 : 1);
        if (n.d.search) {
            const auto base = inset(r, n.d.padding);
            SkPaint paint;
            paint.setAntiAlias(true);
            paint.setStyle(SkPaint::kStroke_Style);
            paint.setStrokeWidth(1.5);
            paint.setColor(n.enabled ? theme.secondary : theme.disabled);
            const auto x = base.x + 7, y = base.y + base.height / 2 - 2;
            canvas.drawCircle(x, y, 5, paint);
            line(canvas, x + 4, y + 4, x + 9, y + 9, paint.getColor(), 1.5);
        }
        canvas.save();
        canvas.clipRect(SkRect::MakeXYWH(content.x, content.y, content.width, content.height));
        auto translated = [&](Rect r) {
            r.x += n.text_origin.x;
            r.y += n.text_origin.y;
            return r;
        };
        if (n.text && focused == n.id && n.text->selection_rects)
            for (auto rect : n.text->selection_rects(buffer.selection().range()))
                box(canvas, translated(rect), active ? theme.selection : theme.inactive);
        if (buffer.text().empty() && !n.d.text.empty()) {
            auto span = fonts.span(n, n.d.text);
            span.color = theme.secondary;
            auto placeholder =
                fonts.engine->layout({span}, {content.width, TextWrap::no_wrap, TextOverflow::ellipsis, 1});
            const double free_width = std::max(0., content.width - placeholder->size.width);
            const double offset = n.d.text_align == Align::center ? free_width / 2
                                  : n.d.text_align == Align::end  ? free_width
                                                                  : 0;
            placeholder->paint(canvas, {content.x + offset, n.text_origin.y});
        } else if (n.text)
            n.text->paint(canvas, n.text_origin);
        if (n.text && n.text->selection_rects && buffer.marked_range())
            for (auto rect : n.text->selection_rects(*buffer.marked_range())) {
                rect = translated(rect);
                line(canvas, rect.x, rect.y + rect.height - 1, rect.x + rect.width, rect.y + rect.height - 1,
                     theme.caret, 1.5);
            }
        if (editor.reset_blink || !editor.blink_epoch) {
            editor.blink_epoch = time;
            editor.reset_blink = false;
        }
        if (n.text && n.text->caret_rect && focused == n.id && active && !n.d.text_options.read_only &&
            !buffer.selection().range().length &&
            std::fmod(std::max(0., time - *editor.blink_epoch), 1.0) < .55)
            box(canvas, translated(field_caret(n)), theme.caret);
        canvas.restore();
    }
    void paint_swatch(SkCanvas& c, Rect r, Color color) const {
        c.save();
        c.clipRect(SkRect::MakeXYWH(r.x, r.y, r.width, r.height));
        for (int y = 0; y < std::ceil(r.height / 8); ++y)
            for (int x = 0; x < std::ceil(r.width / 8); ++x)
                box(c, {r.x + x * 8, r.y + y * 8, 8, 8}, (x + y) % 2 ? 0xffa0a0a0 : 0xffeeeeee);
        box(c, r, color);
        box(c, r, theme.border, 0, 1);
        c.restore();
    }
    void paint_node(SkCanvas& c, Node& n, double time) {
        if (n.hidden || n.clip.width <= 0 || n.clip.height <= 0)
            return;
        auto& d = n.d;
        auto r = n.bounds;
        Rect content = inset(r, d.padding);
        c.save();
        c.clipRect(SkRect::MakeXYWH(n.clip.x, n.clip.y, n.clip.width, n.clip.height));
        if (d.background && d.kind != Kind::text_field && d.kind != Kind::listbox)
            box(c, r, *d.background, d.radius.value_or(0));
        Color ink = n.enabled ? n.color : theme.disabled;
        switch (d.kind) {
        case Kind::table_header: break;
        case Kind::table_column: {
            box(c,r,theme.panel);
            line(c,r.x+r.width-1,r.y,r.x+r.width-1,r.y+r.height,theme.border,1);
            caption(c,n,d.text,{r.x+8,r.y,std::max(0.,r.width-30),r.height},ink);
            const auto& order=tables.at(n.table_owner).sort;
            auto sorted=std::find_if(order.begin(),order.end(),[&](const auto& sort){return sort.column==n.column;});
            if(sorted!=order.end())chevron(c,r.x+r.width-16,r.y+r.height/2,theme.cyan,sorted->order==SortOrder::ascending);
            break;
        }
        case Kind::table_resize:
            if(hovered==n.id||dragging==n.id)box(c,{r.x+r.width/2-1,r.y+4,2,std::max(0.,r.height-8)},theme.cyan);
            break;
        case Kind::table_cell: {
            const auto& state=tables.at(n.table_owner);
            if(state.selected_cells.contains(n.id))box(c,r,theme.selection,2);
            if(focused==n.table_owner&&lists.at(n.table_owner).active==n.selection&&state.column==n.column)
                box(c,r,theme.cyan,2,1);
            if(state.edit&&state.edit->cell==TableCell{n.selection,n.column}&&!state.edit->error.empty())box(c,r,theme.error,2,1.5);
            line(c,r.x+r.width-1,r.y,r.x+r.width-1,r.y+r.height,theme.border,1);
            break;
        }
        case Kind::field_action:
            if(d.calendar_action){
                const auto x=r.x+r.width/2,y=r.y+r.height/2;
                box(c,{x-7,y-6,14,13},ink,2,1);
                line(c,x-7,y-2,x+7,y-2,ink,1);
                line(c,x-3,y-8,x-3,y-4,ink,1.5);line(c,x+3,y-8,x+3,y-4,ink,1.5);
            } else if (d.combo_action) {
                const auto* owner = find(n.group);
                chevron(c, r.x + r.width / 2, r.y + r.height / 2, ink, owner && combos.at(owner->id).open);
            } else if (d.reveal_action) {
                box(c, r, fill(n, theme.inactive), 4);
                caption(c, n, d.text, r, ink, Align::center);
            } else {
                const auto x = r.x + r.width / 2, y = r.y + r.height / 2;
                line(c, x - 4, y - 4, x + 4, y + 4, ink, 1.5);
                line(c, x + 4, y - 4, x - 4, y + 4, ink, 1.5);
            }
            break;
        case Kind::stepper: {
            box(c, r, d.background.value_or(theme.field), d.radius.value_or(6));
            const bool invalid = *n.numeric_value < d.numeric->min || *n.numeric_value > d.numeric->max;
            box(c, r, invalid ? theme.error : theme.border, d.radius.value_or(6), 1);
            const auto button = std::min(36., content.width / 3);
            caption(c, n, n.numeric_text, {content.x + button, content.y, std::max(0., content.width - 2 * button), content.height},
                    ink, d.control_text_align.value_or(Align::center));
            break;
        }
        case Kind::step_button:
            box(c, r, fill(n, theme.inactive), 4);
            caption(c, n, d.text, r, ink, Align::center);
            break;
        case Kind::switch_control: {
            const double width = std::min(48., r.width), height = std::min(26., r.height);
            const Rect track{r.x, r.y + (r.height - height) / 2, width, height};
            box(c, track, fill(n, n.number ? n.accent : theme.inactive), height / 2);
            const auto diameter = std::max(0., std::min(height - 6, width - 6));
            box(c, {track.x + (n.number ? std::max(3., width - diameter - 3) : 3), track.y + 3, diameter, diameter},
                n.enabled ? (n.number ? contrast(n.accent) : theme.text) : theme.disabled, diameter / 2);
            caption(c, n, d.text, {r.x + 60, r.y, std::max(0., r.width - 60), r.height}, ink);
            break;
        }
        case Kind::split_divider: {
            const bool row = d.orientation == Orientation::horizontal;
            const auto extent = row ? r.height : r.width;
            const auto inset = std::min(6., extent / 2);
            const auto ink = n.enabled ? (hovered == n.id || dragging == n.id ? theme.cyan : theme.secondary)
                                       : theme.disabled;
            const auto rail = row ? Rect{r.x + std::max(0., (r.width - 2) / 2), r.y + inset,
                                         std::min(2., r.width), std::max(0., r.height - 2 * inset)}
                                  : Rect{r.x + inset, r.y + std::max(0., (r.height - 2) / 2),
                                         std::max(0., r.width - 2 * inset), std::min(2., r.height)};
            box(c, rail, ink, 1);
            const auto length = std::min(28., std::max(0., extent - 2 * inset));
            const auto grip = row ? Rect{r.x + std::max(0., (r.width - 4) / 2), r.y + (r.height - length) / 2,
                                         std::min(4., r.width), length}
                                  : Rect{r.x + (r.width - length) / 2, r.y + std::max(0., (r.height - 4) / 2),
                                         length, std::min(4., r.height)};
            box(c, grip, ink, 2);
            break;
        }
        case Kind::image:
            d.bitmap.paint(c, content, d.content_mode);
            break;
        case Kind::listbox:
            box(c, r, d.background.value_or(theme.field), d.radius.value_or(6));
            box(c, r, theme.border, d.radius.value_or(6), 1);
            break;
        case Kind::list_option: {
            const auto* parent=find(n.group);
            if (n.number && !(parent && parent->d.table && parent->d.table->options.selection_mode==TableSelectionMode::cells)) box(c, r, theme.selection, 3);
            else if (hovered == n.id) box(c, r, theme.inactive, 3);
            double leading = 10;
            if (n.tree_item) {
                const auto* tree = find(n.group);
                const auto indent = tree && tree->d.tree ? tree->d.tree->options.indentation : 18;
                leading += std::min(n.tree_item->depth * indent, std::max(0., r.width - 100));
                if (n.tree_item->expandable) {
                    const auto x = r.x + leading + 6, y = r.y + r.height / 2;
                    if (n.tree_item->expanded) {
                        line(c, x - 4, y - 2, x, y + 2, ink, 1.5); line(c, x, y + 2, x + 4, y - 2, ink, 1.5);
                    } else {
                        line(c, x - 2, y - 4, x + 2, y, ink, 1.5); line(c, x + 2, y, x - 2, y + 4, ink, 1.5);
                    }
                }
                leading += 22;
            }
            if (!n.custom_row) caption(c, n, d.text + (n.tree_item && n.tree_item->loading ? " (loading…)" : ""),
                                      {r.x + leading, r.y, std::max(0., r.width - leading - 10), r.height}, ink);
            break;
        }
        case Kind::text_field:
            paint_field(c, n, time);
            break;
        case Kind::label:
            if(d.sample){paint_swatch(c,r,*d.sample);break;}
            if (n.text)
                n.text->paint(c, n.text_origin);
            break;
        case Kind::button:
        case Kind::toggle:
        case Kind::segment_option:
        case Kind::tab_option:
        case Kind::menu:
        case Kind::selection:
        case Kind::split: {
            bool selected = d.kind == Kind::button || d.kind == Kind::menu || d.kind == Kind::selection ||
                            d.kind == Kind::split || n.number != 0;
            Color base = d.background.value_or(d.alert_action == AlertActionRole::destructive ? theme.error :
                                              d.alert_action == AlertActionRole::cancel || d.alert_action == AlertActionRole::normal ? theme.inactive :
                                              selected ? n.accent : theme.inactive);
            if(n.calendar_date&&focused==n.id&&n.number==0)base=theme.selection;
            box(c, r, fill(n, base),
                d.radius.value_or(d.kind == Kind::segment_option ? 0 : theme.button_radius));
            Color text = d.foreground.value_or(selected ? contrast(base) : theme.text);
            if (!n.enabled)
                text = theme.disabled;
            double reserve = (d.kind == Kind::menu || d.kind == Kind::selection) ? 32
                             : d.kind == Kind::split                             ? theme.split_width
                                                                                 : 0;
            const auto leading=d.color&&d.color->well?38.:0.;
            if(d.sample){const auto size=std::max(0.,std::min({26.,r.width-16,r.height-12}));paint_swatch(c,{r.x+8,r.y+(r.height-size)/2,size,size},*d.sample);}
            const auto text_padding=n.calendar_date?2.:12.;
            caption(c, n, d.text, {r.x + text_padding + leading, r.y, std::max(0., r.width - 2*text_padding - reserve - leading), r.height}, text,
                    d.control_text_align.value_or(Align::center));
            if (d.kind == Kind::menu || d.kind == Kind::selection || d.kind == Kind::split) {
                double x = r.x + r.width - (d.kind == Kind::split ? theme.split_width / 2 : 18);
                chevron(c, x, r.y + r.height / 2, text, menu_popup() && menu_popup()->owner == n.id);
                if (d.kind == Kind::split)
                    line(c, r.x + r.width - theme.split_width, r.y + 1, r.x + r.width - theme.split_width,
                         r.y + r.height - 1, theme.window, 1);
            }
            break;
        }
        case Kind::checkbox:
        case Kind::radio_option: {
            double y = r.y + (r.height - 20) / 2;
            Rect indicator{r.x, y, 20, 20};
            bool radio = d.kind == Kind::radio_option;
            Color base = n.number == 1 ? n.accent : theme.window;
            if (!n.enabled)
                base = n.number ? theme.inactive : theme.window;
            box(c, indicator, fill(n, base), radio ? 10 : 4);
            if (n.number != 1)
                box(c, indicator,
                    hovered == n.id ? theme.cyan
                    : n.enabled     ? theme.border
                                    : theme.disabled,
                    radio ? 10 : 4, 1.5);
            if (n.number == 1) {
                if (radio)
                    box(c, {r.x + 6, y + 6, 8, 8}, n.enabled ? contrast(base) : theme.disabled, 4);
                else
                    checkmark(c, r.x, y, n.enabled ? contrast(base) : theme.disabled);
            }
            if (n.number == 2)
                line(c, r.x + 5, y + 10, r.x + 15, y + 10, ink, 2.5);
            caption(c, n, d.text, {r.x + 30, r.y, std::max(0., r.width - 30), r.height}, ink);
            break;
        }
        case Kind::slider:
        case Kind::progress: {
            const double inset_x = d.kind == Kind::slider ? 8 : 0;
            Rect track{r.x + inset_x, r.y + (r.height - 8) / 2, std::max(0., r.width - 2 * inset_x), 8};
            box(c, track, theme.inactive, 4);
            double fraction = d.kind == Kind::slider
                                  ? std::clamp((n.number - d.range.min) / (d.range.max - d.range.min), 0., 1.)
                                  : std::clamp(n.number, 0., 1.);
            if (d.indeterminate) {
                double part = track.width * .25;
                double x = track.x + (track.width - part) * (.5 + .5 * std::sin(time * 2.6));
                box(c, {x, track.y, part, 8}, n.enabled ? n.accent : theme.disabled, 4);
            } else
                box(c, {track.x, track.y, track.width * fraction, 8}, n.enabled ? n.accent : theme.disabled,
                    4);
            if (d.kind == Kind::slider)
                box(c, {track.x + track.width * fraction - 8, r.y + r.height / 2 - 8, 16, 16},
                    n.enabled ? theme.cyan : theme.disabled, 8);
            break;
        }
        case Kind::group:
        case Kind::panel:
            if (!d.background)
                box(c, r, d.kind == Kind::group ? theme.group : theme.panel, d.radius.value_or(8));
            if (d.kind == Kind::group || n.popover_root || n.tooltip_root)
                box(c, r, theme.border, d.radius.value_or(8), 1);
            [[fallthrough]];
        case Kind::section:
            if (d.kind != Kind::panel) {
                Node label;
                label.font = d.kind == Kind::section ? 18 : 14;
                label.line_height = d.kind == Kind::section ? 24 : 20;
                label.d.bold = true;
                caption(c, label, d.text,
                        {content.x, content.y, content.width - (d.expansion ? 24 : 0),
                         d.kind == Kind::section ? 24. : 20.},
                        ink);
                if (d.expansion)
                    chevron(c, content.x + content.width - 10, content.y + 10, theme.cyan, n.expanded);
            }
            break;
        case Kind::separator:
            box(c, r, d.foreground.value_or(theme.border));
            break;
        default:
            break;
        }
        if (d.kind == Kind::flow && n.text)
            n.text->paint(c, n.text_origin);
        if (d.kind == Kind::segments) {
            c.save();
            c.clipRRect(SkRRect::MakeRectXY(SkRect::MakeXYWH(r.x, r.y, r.width, r.height),
                                            theme.button_radius, theme.button_radius),
                        true);
        }
        for (auto* child : n.paint_children)
            paint_node(c, *child, time);
        if (d.kind == Kind::segments) {
            c.restore();
            for (std::size_t i = 1; i < n.children.size(); ++i) {
                auto rr = n.children[i]->bounds;
                line(c, rr.x, rr.y + 1, rr.x, rr.y + rr.height - 1, theme.window, 1);
            }
        }
        if ((d.kind == Kind::listbox || d.kind == Kind::scroll) &&
            (d.kind == Kind::listbox || d.scroll_options.show_scrollbars)) {
            const auto offset = scroll[n.id];
            const bool x = n.scroll_max.x > 0 && content.width > 0;
            const bool y = n.scroll_max.y > 0 && content.height > 0;
            if (y) {
                const auto track = std::max(0., content.height - (x ? 8 : 0));
                const auto thumb = std::min(track, std::max(16., track * content.height / n.scroll_content.height));
                box(c, {content.x + content.width - 5, content.y + (track - thumb) * offset.y / n.scroll_max.y, 3, thumb},
                    theme.border, 1.5);
            }
            if (x) {
                const auto track = std::max(0., content.width - (y ? 8 : 0));
                const auto thumb = std::min(track, std::max(16., track * content.width / n.scroll_content.width));
                box(c, {content.x + (track - thumb) * offset.x / n.scroll_max.x, content.y + content.height - 5, thumb, 3},
                    theme.border, 1.5);
            }
        }
        c.restore();
    }
    void paint_focus(SkCanvas& c) {
        auto* target = find(focused);
        if (!active || !focus_visible || menu_popup() || !target || !interactive(*target) ||
            target->clip.width <= 0 || target->clip.height <= 0)
            return;
        auto& n = *target;
        const auto& d = n.d;
        Rect area = n.bounds;
        if (d.kind == Kind::checkbox || d.kind == Kind::radio_option) {
            const double content_height = std::max(20., n.line_height);
            const double content_width = std::min(area.width, 30 + fonts.width(n, d.text));
            area = {area.x - 10, area.y + (area.height - content_height) / 2 - 5, content_width + 20,
                    content_height + 10};
        } else if(n.calendar_date){
            area=inset(area,Insets::all(2));
        } else {
            if (d.kind == Kind::split)
                area.width = std::max(0., area.width - theme.split_width);
            if (d.kind == Kind::section || d.kind == Kind::group) {
                area = inset(area, d.padding);
                area.height = d.kind == Kind::section ? 24 : 20;
            }
            area = {area.x - 5, area.y - 4, area.width + 10, area.height + 8};
        }
        c.save();
        c.clipRect(SkRect::MakeWH(viewport.width, viewport.height));
        for (const auto& [id, ancestor] : nodes)
            if (ancestor->d.kind == Kind::scroll && ancestor->overlay_scope == n.overlay_scope && n.id.starts_with(id + "/")) {
                const auto clip = inset(ancestor->bounds, ancestor->d.padding);
                c.clipRect(SkRect::MakeXYWH(clip.x, clip.y, clip.width, clip.height));
            }
        const auto focus_color=n.calendar_date&&n.number?contrast(n.accent):theme.cyan;
        const double length = std::min({5., area.width / 4, area.height / 4});
        if (length > 0) {
            for (int horizontal : {-1, 1})
                for (int vertical : {-1, 1}) {
                    const double x = horizontal == 1 ? area.x : area.x + area.width;
                    const double y = vertical == 1 ? area.y : area.y + area.height;
                    line(c, x, y, x + horizontal * length, y, focus_color, theme.focus_width);
                    line(c, x, y, x, y + vertical * length, focus_color, theme.focus_width);
                }
        }
        c.restore();
    }
    Rect popup_item_rect(std::size_t index) const {
        auto r = menu_popup()->bounds;
        return {r.x + theme.popup_padding,
                r.y + theme.popup_padding + index * theme.popup_item_height - menu_popup()->scroll,
                std::max(0., r.width - 2 * theme.popup_padding), theme.popup_item_height};
    }
    void paint_popup(SkCanvas& c) {
        if (!menu_popup())
            return;
        auto* owner = find(menu_popup()->owner);
        if (!owner)
            return;
        auto r = menu_popup()->bounds;
        box(c, r, theme.panel, 6);
        box(c, r, theme.border, 6, 1);
        c.save();
        auto inside = inset(r, Insets::all(theme.popup_padding));
        c.clipRect(SkRect::MakeXYWH(inside.x, inside.y, inside.width, inside.height));
        for (std::size_t i = 0; i < menu_popup()->options.size(); ++i) {
            auto rr = popup_item_rect(i);
            auto& o = menu_popup()->options[i];
            bool selected = owner->selection == o.id;
            if (static_cast<int>(i) == menu_popup()->highlighted)
                box(c, rr, theme.inactive, 3);
            if (selected)
                checkmark(c, rr.x + 3, rr.y + (rr.height - 20) / 2, theme.primary);
            caption(c, *owner, o.label, {rr.x + 30, rr.y, std::max(0., rr.width - 38), rr.height},
                    o.enabled ? theme.text : theme.disabled);
        }
        c.restore();
    }
    void validate_hold() {
        if (!hold)
            return;
        const auto* button = find(hold->button);
        const auto* owner = find(hold->owner);
        if (!active || focused != hold->owner || !button || !interactive(*button) ||
            !owner || !owner->d.numeric || owner->d.numeric->read_only) {
            hold.reset();
            pressed.clear();
            return;
        }
        if (!button->clip.contains(hold->point)) {
            hold->due.reset();
            pressed.clear();
        }
    }
    void validate_searches() {
        for (auto& [id, editor] : editors) {
            if (!editor->pending_search)
                continue;
            const auto* n = find(id);
            if (!active || !n || !n->d.search || !interactive(*n) || n->d.search->read_only || !n->d.search->on_search)
                editor->pending_search.reset();
            else if (editor->buffer.marked_range())
                editor->pending_search->suspended = true;
        }
    }
    void advance_time() {
        layout(viewport);
        validate_hold();
        const auto now = options.clock();
        if (hold && hold->due && now >= *hold->due) {
            const auto owner = hold->owner;
            const auto steps = hold->steps;
            hold->due = now + find(owner)->d.numeric->repeat_interval;
            adjust(owner, steps);
            layout(viewport);
            validate_hold();
        }
        validate_searches();
        std::vector<std::string> due;
        for (const auto& [id, editor] : editors)
            if (editor->pending_search && !editor->pending_search->suspended && editor->pending_search->due <= now)
                due.push_back(id);
        for (const auto& id : due)
            flush_search(id);
        if (tooltip && !tooltip->shown && tooltip->due <= now) {
            tooltip->shown = true;
            invalidate();
            layout(viewport);
        }
    }
    std::optional<std::chrono::milliseconds> next_wake() {
        layout(viewport);
        validate_hold();
        validate_searches();
        std::optional<std::chrono::steady_clock::time_point> due;
        if (hold)
            due = hold->due;
        if (tooltip && !tooltip->shown && (!due || tooltip->due < *due)) due = tooltip->due;
        for (const auto& [id, editor] : editors)
            if (editor->pending_search && !editor->pending_search->suspended &&
                (!due || editor->pending_search->due < *due))
                due = editor->pending_search->due;
        if (!due)
            return {};
        return std::max(std::chrono::milliseconds(0),
                        std::chrono::ceil<std::chrono::milliseconds>(*due - options.clock()));
    }
    void paint(SkCanvas& c, double time) {
        advance_time();
        layout(viewport);
        c.clear(theme.window);
        if (root)
            paint_node(c, *root, time);
        for (const auto& overlay : overlays)
            if (overlay.kind != OverlayKind::menu && overlay.kind != OverlayKind::tooltip)
                if (auto* body = find(overlay.root_id)) paint_node(c, *body, time);
        paint_focus(c);
        paint_popup(c);
        paint_contexts(c);
        for (const auto& overlay : overlays)
            if (overlay.kind == OverlayKind::tooltip)
                if (auto* body = find(overlay.root_id)) paint_node(c, *body, time);
        paint_inspection(c, time);
    }
    std::vector<Node*> focusables() const {
        std::vector<Node*> result;
        std::function<void(Node&)> visit = [&](Node& n) {
            if (n.hidden)
                return;
            if (n.d.kind == Kind::listbox || n.d.kind == Kind::stepper || n.d.combo || n.d.date || n.d.time) {
                if (interactive(n))
                    result.push_back(&n);
                if (n.d.kind == Kind::listbox && n.d.rows.has_content() && !n.d.table)
                    for (auto& row : n.children) for (auto& child : row->children) visit(*child);
                return;
            }
            if (n.d.kind == Kind::radio || n.d.kind == Kind::segments || n.d.kind == Kind::tab_bar) {
                Node* target = nullptr;
                for (auto& child : n.children)
                    if (child->enabled) {
                        if (!target)
                            target = child.get();
                        if (child->number == 1) {
                            target = child.get();
                            break;
                        }
                    }
                if (target)
                    result.push_back(target);
                return;
            }
            if (interactive(n))
                result.push_back(&n);
            for (auto& child : n.children)
                visit(*child);
        };
        if (root && !focus_scope()) visit(*root);
        for (const auto& overlay : overlays)
            if (overlay.popover)
                if (auto* body = find(overlay.root_id); body && in_focus_scope(*body)) visit(*body);
        return result;
    }
    void reveal(Node& n) {
        if (n.d.kind == Kind::tab_option)
            tabs.at(n.group).reveal = true;
        const auto target_id = n.id;
        std::vector<std::string> ancestors;
        for (const auto& [id, parent] : nodes)
            if ((parent->d.kind == Kind::scroll || parent->d.kind == Kind::listbox) &&
                parent->overlay_scope == n.overlay_scope &&
                n.id.starts_with(id + "/")) ancestors.push_back(id);
        std::sort(ancestors.begin(), ancestors.end(), [](const auto& a, const auto& b) { return a.size() > b.size(); });
        for (const auto& id : ancestors) {
            auto* target = find(target_id);
            auto* parent = find(id);
            if (!target || !parent) break;
            auto viewport = content_box(*parent);
            if(parent->d.table && target_id.starts_with(parent->id+"/header")) {
                viewport.y-=parent->d.table->options.header_height;viewport.height+=parent->d.table->options.header_height;
            }
            const auto r = target->bounds;
            const auto delta = [](double start, double extent, double view_start, double view_extent) {
                if (start < view_start || extent > view_extent) return start - view_start;
                if (start + extent > view_start + view_extent) return start + extent - view_start - view_extent;
                return 0.;
            };
            auto& offset = scroll[id];
            const Point next{std::clamp(offset.x + delta(r.x, r.width, viewport.x, viewport.width), 0., parent->scroll_max.x),
                             std::clamp(offset.y + delta(r.y, r.height, viewport.y, viewport.height), 0., parent->scroll_max.y)};
            if (next.x != offset.x || next.y != offset.y) {
                offset = next; geometry_dirty = true; layout(this->viewport);
            }
        }
        geometry_dirty = true;
        layout(viewport);
    }
    bool scroll_to(std::string id, Point requested) {
        check();
        if (detail::building) throw std::logic_error("Cannot scroll while building views");
        if (!std::isfinite(requested.x) || !std::isfinite(requested.y))
            throw std::invalid_argument("Scroll offsets must be finite");
        layout(viewport);
        const auto* node = find(id);
        if (!node || (node->d.kind != Kind::scroll && node->d.kind != Kind::listbox) ||
            node->hidden || !node->enabled || !in_focus_scope(*node)) return false;
        auto& offset = scroll[id];
        const Point next{std::clamp(requested.x, 0., node->scroll_max.x), std::clamp(requested.y, 0., node->scroll_max.y)};
        if (next.x != offset.x || next.y != offset.y) {
            offset = next; geometry_dirty = true; layout(viewport); notify();
        }
        return true;
    }
    bool focus(std::string id, bool show_accents = true) {
        layout(viewport);
        if (auto logical = logical_row(id)) {
            const auto list_id = logical->first->id;
            if (!logical_item(*logical->first, logical->second)->enabled || !reveal_item(list_id, logical->second)) return false;
        }
        if(auto cell=logical_cell_id(id)){
            if(!table_reveal(cell->first,cell->second))return false;
            const auto info=logical_cell(cell->first,cell->second);if(!info||!info->enabled)return false;
            const bool result=focus(cell->first,show_accents);
            if(result){tables.at(cell->first).column=cell->second.column;lists.at(cell->first).active=cell->second.row;notify();}
            return result;
        }
        if(auto col=logical_column_id(id))return focus(col->header_id,show_accents);
        auto* n = find(id);
        if (n && !n->combo_owner.empty()) {
            if (!interactive(*n)) return false;
            const auto owner = n->combo_owner, selected = n->selection, row_id = n->id;
            if (!selected.empty() && n->enabled) { combos.at(owner).highlighted = selected; invalidate(); }
            const bool result = focus(owner, show_accents);
            if (result) if (auto* row = find(row_id)) reveal(*row);
            return result;
        }
        if (!n || !interactive(*n) || !in_focus_scope(*n))
            return false;
        if (n->d.kind == Kind::list_option) {
            const auto list_id = n->group, option_id = n->selection;
            const bool result = focus(list_id, show_accents);
            if (result) {
                lists.at(list_id).active = option_id;
                if (auto* list = find(list_id)) if (auto index = list->d.rows.index_of(option_id)) reveal_item(list_id, *index);
            }
            return result;
        }
        if (focused != id) {
            const auto* previous=find(focused);
            const auto previous_group=previous?previous->datetime_owner:std::string{};
            const auto* previous_owner = field_owner(focused);
            const auto* next_owner = field_owner(id);
            if (previous_owner && next_owner && previous_owner->id == next_owner->id)
                unmark_text(true);
            else
                finish_editing();
            layout(viewport);
            if(!previous_group.empty()&&id!=previous_group&&!id.starts_with(previous_group+"/"))cancel_datetime(previous_group);
            n = find(id);
            if (!n || !interactive(*n) || !in_focus_scope(*n))
                return false;
        }
        const bool changed = focused != id;
        focused = id;
        for (const auto& [row_id, row] : nodes)
            if (row->d.kind == Kind::list_option && id.starts_with(row_id + "/"))
                lists.at(row->group).active = row->selection;
        if (n->d.combo) begin_combo(id);
        if (changed && hold) {
            hold.reset();
            pressed.clear();
        }
        active = true;
        focus_visible = show_accents;
        if (n->d.kind == Kind::text_field) {
            auto& editor = *editors.at(id);
            editor.reset_blink = true;
            editor.reveal_caret = true;
            if (changed && show_accents && !n->d.text_options.multiline)
                editor.buffer.command(TextCommand::select_all);
        }
        reveal(*n);
        if (show_accents) {
            suppressed_tooltip.clear();
            schedule_tooltip(focus_tooltip_target());
        }
        notify();
        return true;
    }
    bool focus_search(std::string id) {
        layout(viewport);
        if (id.empty()) {
            std::function<void(const Node&)> visit = [&](const Node& node) {
                if (!id.empty() || node.hidden)
                    return;
                if (node.d.search && interactive(node) && in_focus_scope(node)) {
                    id = node.id;
                    return;
                }
                for (const auto& child : node.children)
                    visit(*child);
            };
            if (root)
                visit(*root);
            for (const auto& overlay : overlays)
                if (overlay.popover)
                    if (auto* body = find(overlay.root_id)) visit(*body);
        }
        const auto* n = find(id);
        if (!n || !n->d.search || !interactive(*n) || !focus(id, false))
            return false;
        unmark_text(true);
        layout(viewport);
        n = find(id);
        if (!n || !n->d.search || !interactive(*n))
            return true;
        editors.at(id)->buffer.command(TextCommand::select_all);
        field_changed(*find(id), false);
        return true;
    }
    bool open_context(std::string id, std::optional<Point> origin) {
        layout(viewport);
        auto* owner = find(id);
        if (!owner || !owner->enabled || owner->hidden || owner->context_menu.empty() || !in_focus_scope(*owner)) return false;
        const auto visible = intersect(owner->bounds, owner->clip);
        if (viewport.width > 0 && viewport.height > 0 && (visible.width <= 0 || visible.height <= 0)) return false;
        if (origin && (!std::isfinite(origin->x) || !std::isfinite(origin->y)))
            throw std::invalid_argument("Context-menu position must be finite");
        finish_editing();
        layout(viewport);
        owner = find(id);
        if (!owner || !owner->enabled || owner->hidden || owner->context_menu.empty() || !in_focus_scope(*owner)) return false;
        Overlay menu;
        menu.kind = OverlayKind::context; menu.owner = id; menu.root_id = id + "/context";
        menu.restore_focus = focused;
        menu.context = Overlay::Context{owner->context_menu, {}, origin, {}, {}, focus_visible};
        for (std::size_t i = 0; i < menu.context->entries.size(); ++i)
            if (menu_selectable(menu.context->entries[i])) { menu.highlighted = static_cast<int>(i); break; }
        hide_tooltip(); hold.reset(); dragging.clear(); text_drag.clear(); pressed.clear();
        drop_menu();
        overlays.push_back(std::move(menu));
        position_contexts(true);
        ++generation; notify();
        return true;
    }
    void close_context(bool all = false) {
        if (!context_popup()) return;
        const auto root = std::find_if(overlays.begin(), overlays.end(), [](const auto& o) { return o.context.has_value(); });
        const bool last = all || root->root_id == context_popup()->root_id;
        const auto restore = root->restore_focus;
        const auto accents = root->context->restore_accents;
        if (last) std::erase_if(overlays, [](const auto& o) { return o.context.has_value(); });
        else {
            const auto id = context_popup()->root_id;
            std::erase_if(overlays, [&](const auto& o) { return o.root_id == id; });
        }
        ++generation;
        if (last && active) {
            auto* target = find(restore);
            if (target && interactive(*target) && in_focus_scope(*target)) focus(restore, accents);
        }
        notify();
    }
    bool choose_context(std::string menu_id, int index) {
        const auto keep_alive = shared_from_this();
        auto menu = std::find_if(overlays.begin(), overlays.end(), [&](const auto& o) { return o.root_id == menu_id && o.context; });
        if (menu == overlays.end() || index < 0 || index >= static_cast<int>(menu->context->entries.size())) return false;
        const auto entry = menu->context->entries[index];
        if (!menu_selectable(entry)) return false;
        menu->highlighted = index;
        if (entry.item.kind == MenuItemKind::submenu) {
            Overlay child;
            child.kind = OverlayKind::context; child.owner = menu->owner;
            child.root_id = context_item_id(*menu, index) + "/submenu";
            child.restore_focus = menu->restore_focus;
            child.context = Overlay::Context{entry.children, menu->context->path, {}, menu->root_id, entry.item.id,
                                            menu->context->restore_accents};
            child.context->path.push_back(entry.item.id);
            for (std::size_t i = 0; i < entry.children.size(); ++i)
                if (menu_selectable(entry.children[i])) { child.highlighted = static_cast<int>(i); break; }
            overlays.erase(menu + 1, overlays.end());
            overlays.push_back(std::move(child));
            position_contexts(true);
            ++generation; notify();
        } else {
            close_context(true);
            entry.item.invoke();
            invalidate();
        }
        return true;
    }
    bool context_key(KeyEvent e) {
        auto* menu = context_popup();
        if (!menu) return false;
        if (e.key == Key::escape || e.key == Key::left) { close_context(); return true; }
        if (e.key == Key::tab) { close_context(true); return key(e); }
        if (e.key == Key::enter || e.key == Key::space || e.key == Key::right) {
            if (!e.repeat && menu->highlighted >= 0 &&
                (e.key != Key::right || menu->context->entries[menu->highlighted].item.kind == MenuItemKind::submenu))
                choose_context(menu->root_id, menu->highlighted);
            return true;
        }
        const auto count = static_cast<int>(menu->context->entries.size());
        if (e.key == Key::up || e.key == Key::down || e.key == Key::home || e.key == Key::end) {
            const int direction = e.key == Key::up || e.key == Key::end ? -1 : 1;
            int index = e.key == Key::home ? -1 : e.key == Key::end ? count : menu->highlighted;
            for (int i = 0; i < count; ++i) {
                index = (index + direction + count) % count;
                if (menu_selectable(menu->context->entries[index])) { menu->highlighted = index; break; }
            }
        } else if (!e.text.empty() && !e.command && !e.control && !e.alt) {
            for (int i = 1; i <= count; ++i) {
                const auto index = (menu->highlighted + i + count) % count;
                const auto& item = menu->context->entries[index];
                if (menu_selectable(item) && capitalized(item.item.label).starts_with(capitalized(e.text))) {
                    menu->highlighted = index; break;
                }
            }
        }
        position_contexts(true); notify();
        return true;
    }
    bool context_pointer(PointerEvent e) {
        if (!context_popup()) return false;
        Overlay* target = nullptr;
        for (auto it = overlays.rbegin(); it != overlays.rend(); ++it)
            if (it->context && it->bounds.contains(e.position)) { target = &*it; break; }
        if (!target) {
            if (e.kind == PointerKind::down) { close_context(true); pressed = "@dismissed"; }
            else if (e.kind == PointerKind::up) pressed.clear();
            return true;
        }
        const auto id = target->root_id;
        const auto index = context_hit(*target, e.position);
        if (e.kind == PointerKind::scroll) {
            target->scroll = std::clamp(target->scroll - e.scroll_y, 0.,
                std::max(0., context_height(*target) - std::max(0., target->bounds.height - 2 * theme.popup_padding)));
            position_contexts(); notify();
        } else if ((e.kind == PointerKind::move || e.kind == PointerKind::down) && index >= 0) {
            const auto& entry = target->context->entries[index];
            if (!menu_selectable(entry)) return true;
            const auto item_id = context_item_id(*target, index);
            const bool child_open = std::any_of(overlays.begin(), overlays.end(), [&](const auto& o) {
                return o.context && o.context->parent_menu == id && o.context->parent_item == entry.item.id;
            });
            target->highlighted = index;
            if (e.kind == PointerKind::down && e.button == 0) pressed = item_id;
            if (entry.item.kind == MenuItemKind::submenu && !child_open) choose_context(id, index);
            else if (!child_open) {
                const auto parent = std::find_if(overlays.begin(), overlays.end(), [&](const auto& o) { return o.root_id == id; });
                overlays.erase(parent + 1, overlays.end());
            }
            notify();
        } else if (e.kind == PointerKind::up && e.button == 0) {
            const auto prior = std::exchange(pressed, {});
            if (index >= 0 && prior == context_item_id(*target, index) &&
                target->context->entries[index].item.kind != MenuItemKind::submenu) choose_context(id, index);
        }
        return true;
    }
    void close_popup() {
        if (!menu_popup())
            return;
        focused = menu_popup()->owner;
        if (auto* n = find(focused); n && n->d.kind == Kind::split)
            focused += "/popup";
        drop_menu();
        ++generation;
        notify();
    }
    void open_popup(Node& n) {
        hide_tooltip();
        if (menu_popup() && menu_popup()->owner == n.id) {
            close_popup();
            return;
        }
        Overlay p;
        p.owner = n.id;
        p.options = n.d.options;
        for (std::size_t i = 0; i < p.options.size(); ++i)
            if (p.options[i].enabled) {
                if (p.highlighted < 0)
                    p.highlighted = static_cast<int>(i);
                if (p.options[i].id == n.selection) {
                    p.highlighted = static_cast<int>(i);
                    break;
                }
            }
        drop_menu();
        overlays.push_back(std::move(p));
        ++generation;
        focused = n.id;
        position_popup(true);
        notify();
    }
    void choose(int i) {
        if (!menu_popup() || i < 0 || i >= static_cast<int>(menu_popup()->options.size()) || !menu_popup()->options[i].enabled)
            return;
        auto owner_id = menu_popup()->owner;
        auto option = menu_popup()->options[i];
        auto* n = find(owner_id);
        if (!n) {
            close_popup();
            return;
        }
        auto d = n->d;
        close_popup();
        if (d.kind == Kind::menu) {
            for (auto& action : d.actions)
                if (action.id == option.id && action.enabled && action.invoke) {
                    action.invoke();
                    break;
                }
        } else if (d.write_selection)
            d.write_selection(option.id);
        invalidate();
    }
    std::string next_tab(const Node& container, const std::string& from, int direction, bool edge = false) const {
        std::vector<std::string> enabled;
        for (const auto& tab : container.d.tabs)
            if (tab.enabled)
                enabled.push_back(tab.id);
        if (enabled.empty())
            return {};
        if (edge)
            return direction > 0 ? enabled.front() : enabled.back();
        const auto current = std::find(enabled.begin(), enabled.end(), from);
        const auto index = current == enabled.end() ? 0 : std::distance(enabled.begin(), current);
        const auto count = static_cast<std::ptrdiff_t>(enabled.size());
        return enabled[(index + direction + count) % count];
    }
    bool select_tab(std::string owner, std::string selected, bool show_accents = false) {
        const auto available = [&](const Node* node) {
            return node && node->d.kind == Kind::tab_view && node->enabled && !node->hidden &&
                std::any_of(node->d.tabs.begin(), node->d.tabs.end(), [&](const auto& tab) {
                    return tab.id == selected && tab.enabled;
                });
        };
        auto* container = find(owner);
        if (!available(container))
            return false;
        const auto target = owner + "/tab/" + path_key(selected);
        finish_editing();
        layout(viewport);
        container = find(owner);
        // Committing an editor can change which pages are available. Consume the
        // navigation action, but do not write a now-invalid selection.
        if (!available(container))
            return true;
        const auto write = container->d.write_selection;
        write(std::move(selected));
        invalidate();
        // Selection callbacks may remove the owner. The action still completed.
        focus(target, show_accents);
        return true;
    }
    bool activate(const std::string& id, bool arrow = false) {
        layout(viewport);
        if(auto cell=logical_cell_id(id))return table_select(cell->first,cell->second);
        if (auto logical = logical_row(id)) {
            auto* list = logical->first;
            const auto item = logical_item(*list, logical->second);
            if (!item->enabled) return false;
            const auto key = list->d.rows.at(logical->second).id;
            if (!list->combo_owner.empty()) return accept_combo(list->combo_owner, key);
            return list_select(*list, key, false, bool(list->d.write_selected));
        }
        if(auto col=logical_column_id(id))return table_sort(col->table,col->key);
        auto* n = find(id);
        if (!n || !interactive(*n) || !in_focus_scope(*n))
            return false;
        if (!n->combo_owner.empty()) return accept_combo(n->combo_owner, n->selection);
        if (n->d.alert_action) {
            finish_editing();
            layout(viewport);
            n = find(id);
            if (!n || !n->d.alert_action || !interactive(*n) || !in_focus_scope(*n))
                return false;
        }
        auto d = n->d;
        switch (d.kind) {
        case Kind::table_column: return table_sort(n->table_owner,n->column);
        case Kind::table_cell: return table_select(n->table_owner,{n->selection,n->column});
        case Kind::field_action:
            if (d.calendar_action) return open_calendar(n->group,true);
            if (d.combo_action) return open_combo(n->group, true);
            return d.reveal_action ? toggle_password(n->group) : clear_search(n->group);
        case Kind::switch_control:
            d.write_number(n->number == 0 ? 1 : 0);
            break;
        case Kind::step_button:
            return adjust(n->group, n->number);
        case Kind::tab_option:
            return select_tab(n->group, n->selection);
        case Kind::split_divider: {
            auto* parent = find(n->group);
            return parent && collapse_split(*parent, SplitPane::first);
        }
        case Kind::list_option: {
            const auto option = n->selection;
            auto* parent = find(n->group);
            return parent && list_select(*parent, option, false, bool(parent->d.write_selected));
        }
        case Kind::listbox:
            return focus(id, false);
        case Kind::text_field:
            return focus(id, false);
        case Kind::button:
        case Kind::radio_option:
        case Kind::segment_option:
            if (d.click)
                d.click();
            break;
        case Kind::toggle:
            d.write_number(n->number == 0 ? 1 : 0);
            break;
        case Kind::checkbox:
            d.write_number(n->number == 1 ? 0 : 1);
            break;
        case Kind::menu:
        case Kind::selection:
            open_popup(*n);
            return true;
        case Kind::split_arrow:
            if (auto* parent = find(n->group))
                open_popup(*parent);
            return true;
        case Kind::split:
            if (arrow) {
                open_popup(*n);
                return true;
            }
            for (auto& action : d.actions)
                if (action.id == n->selection && action.enabled && action.invoke) {
                    action.invoke();
                    break;
                }
            break;
        case Kind::section:
        case Kind::group:
            if (d.expansion)
                d.expansion->set(!n->expanded);
            break;
        default:
            return false;
        }
        invalidate();
        return true;
    }
    bool collapse_split(const Node& n, SplitPane pane) {
        if (!n.enabled || n.hidden || !n.d.split_options.collapsed)
            return false;
        const auto binding = *n.d.split_options.collapsed;
        const auto next = n.collapsed == pane ? SplitPane::none : pane;
        unmark_text(true);
        binding.set(next);
        invalidate();
        return true;
    }
    void write_numeric_value(Node& n, detail::NumericValue value) {
        const auto id = n.id;
        const auto writer = n.d.write_numeric;
        std::optional<NumericValidation> previous_validation;
        if (n.d.kind == Kind::text_field) {
            auto& editor = *editors.at(n.id);
            const auto formatted = detail::format_number(value, *n.d.numeric);
            const auto selection = editor.buffer.selection();
            if (formatted != editor.buffer.text()) {
                editor.buffer.replace(formatted, TextRange{0, editor.buffer.text().size()});
                editor.buffer.select(selection);
            }
            editor.numeric_source = value;
            editor.numeric_base = formatted;
            previous_validation = editor.validation;
            editor.validation = {};
            editor.reset_blink = editor.reveal_caret = true;
            geometry_dirty = true;
        }
        writer(value);
        invalidate();
        layout(viewport);
        const auto* current = find(id);
        if (previous_validation && current && current->d.numeric && current->d.kind == Kind::text_field) {
            const auto validation = editors.at(id)->validation;
            const auto callback = current->d.numeric->on_validation;
            if (validation != *previous_validation && callback)
                callback(validation);
        }
    }
    bool set_value(const std::string& id, double value) {
        layout(viewport);
        auto* n = find(id);
        if (!n || !n->enabled || n->hidden || !in_focus_scope(*n) || !std::isfinite(value))
            return false;
        auto d = n->d;
        if(d.kind==Kind::table_resize)return table_resize(n->table_owner,n->column,value);
        if (d.numeric) {
            if (d.numeric->read_only)
                return false;
            detail::NumericValue number;
            if (d.numeric->min.index() == 0) {
                if (std::floor(value) != value || value < -9223372036854775808.0 || value >= 9223372036854775808.0)
                    return false;
                number = static_cast<std::int64_t>(value);
            } else
                number = value;
            number=std::clamp(number,d.numeric->min,d.numeric->max);
            if(d.numeric->validate&&d.numeric->validate(number).status!=NumericStatus::valid)
                return false;
            write_numeric_value(*n, number);
        } else if (d.kind == Kind::split_divider) {
            auto* parent = find(n->group);
            if (!parent)
                return false;
            const auto s = split_metrics(*parent);
            if (s.available <= 0)
                return false;
            const auto options = parent->d.split_options;
            const auto write = parent->d.write_number;
            value = std::clamp(value, s.low / s.available, s.high / s.available);
            if (options.collapsed)
                options.collapsed->set(SplitPane::none);
            write(value);
        } else if (d.kind == Kind::slider) {
            value = std::clamp(value, d.range.min, d.range.max);
            if (value != d.range.max)
                value = d.range.min + std::round((value - d.range.min) / d.range.step) * d.range.step;
            d.write_number(std::clamp(value, d.range.min, d.range.max));
        } else if (d.kind == Kind::checkbox) {
            if (value < 0 || value > 2 || std::floor(value) != value)
                return false;
            d.write_number(value);
        } else if (d.kind == Kind::toggle || d.kind == Kind::switch_control)
            d.write_number(value != 0 ? 1 : 0);
        else
            return false;
        invalidate();
        return true;
    }
    bool adjust(const std::string& id, double steps) {
        layout(viewport);
        auto* n = find(id);
        if (!n || !in_focus_scope(*n) || !std::isfinite(steps))
            return false;
        if (n->d.numeric) {
            if (!interactive(*n) || n->d.numeric->read_only)
                return false;
            if (n->d.numeric->step_value && n->d.kind == Kind::text_field) {
                const auto field_id = n->id;
                if (!commit_numeric(field_id))
                    return false;
                n = find(field_id);
                if (!n || !n->d.numeric || !interactive(*n) || n->d.numeric->read_only)
                    return false;
            }
            const auto caret = n->d.kind == Kind::text_field ? editors.at(n->id)->buffer.selection().caret : 0;
            const auto value = detail::step_number(*n->numeric_value, steps, *n->d.numeric, caret);
            if (!value)
                return false;
            write_numeric_value(*n, *value);
            return true;
        }
        if (n->d.kind == Kind::split_divider) {
            const auto* parent = find(n->group);
            if (!parent)
                return false;
            const auto s = split_metrics(*parent);
            return s.available > 0 && set_value(id, n->number + steps * parent->d.split_options.keyboard_step / s.available);
        }
        if(n->d.kind==Kind::table_resize)return table_resize(n->table_owner,n->column,tables.at(n->table_owner).widths.at(n->column)+steps*10);
        if (n->d.kind != Kind::slider)
            return false;
        return set_value(id, n->number + steps * n->d.range.step);
    }
    Node* focus_tooltip_target() const {
        const auto* target = find(focused);
        if (!target) return nullptr;
        Node* result = nullptr;
        for (const auto& [id, n] : nodes)
            if (n->d.tooltip && n->overlay_scope == target->overlay_scope &&
                (id == focused || focused.starts_with(id + "/")) && (!result || id.size() > result->id.size())) result = n;
        return result;
    }
    Node* attachment_hit(Point point, bool context) const {
        std::function<Node*(Node&)> search = [&](Node& n) -> Node* {
            if (n.hidden || !n.enabled || !n.clip.contains(point)) return nullptr;
            for (auto i = n.paint_children.rbegin(); i != n.paint_children.rend(); ++i)
                if (auto* found = search(**i)) return found;
            return (context ? !n.context_menu.empty() : n.d.tooltip.has_value()) && n.bounds.contains(point) ? &n : nullptr;
        };
        if (const auto* overlay = interactive_overlay()) {
            if (overlay->kind == OverlayKind::menu) return nullptr;
            auto* body = find(overlay->root_id);
            return body ? search(*body) : nullptr;
        }
        return root ? search(*root) : nullptr;
    }
    Node* tooltip_hit(Point point) const { return attachment_hit(point, false); }
    Node* hit(Point point, bool include_scroll = false) const {
        std::function<Node*(Node&)> search = [&](Node& n) -> Node* {
            if (n.hidden || !n.clip.contains(point))
                return nullptr;
            for (auto i = n.paint_children.rbegin(); i != n.paint_children.rend(); ++i)
                if (auto* found = search(**i))
                    return found;
            if ((interactive(n) || (include_scroll && n.enabled && (n.d.kind == Kind::scroll || n.d.kind == Kind::tab_bar))) &&
                n.bounds.contains(point)) {
                if ((n.d.kind == Kind::section || n.d.kind == Kind::group) &&
                    point.y > n.bounds.y + n.d.padding.top + (n.d.kind == Kind::section ? 24 : 20))
                    return nullptr;
                return &n;
            }
            return nullptr;
        };
        if (const auto* overlay = interactive_overlay(); overlay && (overlay->popover || overlay->kind == OverlayKind::combo)) {
            auto* body = find(overlay->root_id);
            if (overlay->kind == OverlayKind::combo) {
                const auto* owner = find(overlay->owner);
                if (owner && owner->bounds.contains(point) && owner->clip.contains(point)) return search(*find(overlay->owner));
            }
            return body ? search(*body) : nullptr;
        }
        return root ? search(*root) : nullptr;
    }
    int popup_hit(Point point) const {
        if (!menu_popup() || !inset(menu_popup()->bounds, Insets::all(theme.popup_padding)).contains(point))
            return -1;
        int i = static_cast<int>((point.y - menu_popup()->bounds.y - theme.popup_padding + menu_popup()->scroll) /
                                 theme.popup_item_height);
        return i >= 0 && i < static_cast<int>(menu_popup()->options.size()) ? i : -1;
    }
    bool combo_pointer(PointerEvent e) {
        auto* overlay = combo_popup();
        if (!overlay) return false;
        const auto id = overlay->owner;
        const auto* owner = find(id);
        if (owner && owner->bounds.contains(e.position) && owner->clip.contains(e.position)) return false;
        if (!overlay->bounds.contains(e.position)) {
            if (e.kind == PointerKind::down) { finish_combo(id, false); pressed = "@dismissed"; }
            else if (e.kind == PointerKind::up) pressed.clear();
            return true;
        }
        if (e.kind == PointerKind::scroll) return false;
        auto* row = hit(e.position);
        if (row && row->d.kind == Kind::list_option && row->combo_owner == id && row->enabled) {
            const auto selected = row->selection, row_id = row->id;
            if (e.kind == PointerKind::move) {
                auto& state = combos.at(id);
                if (state.highlighted != selected) { state.highlighted = selected; invalidate(); }
            } else if (e.kind == PointerKind::down && e.button == 0) pressed = row_id;
            else if (e.kind == PointerKind::up && e.button == 0) {
                const auto prior = std::exchange(pressed, {});
                if (prior == row_id) accept_combo(id, selected);
            }
        } else if (e.kind == PointerKind::up) pressed.clear();
        return true;
    }
    bool pointer(PointerEvent e) {
        layout(viewport);
        if(column_drag){
            auto* table=find(column_drag->table);
            if(!table||!table->d.table||!interactive(*table)||!find(dragging)){column_drag.reset();dragging.clear();}
            else if(e.kind==PointerKind::cancel){column_drag.reset();dragging.clear();pressed.clear();notify();return true;}
            else if(e.kind==PointerKind::move||(e.kind==PointerKind::up&&e.button==0)){
                const auto drag=*column_drag;
                if(e.kind==PointerKind::up){column_drag.reset();dragging.clear();pressed.clear();}
                table_resize(drag.table,drag.column,drag.initial+e.position.x-drag.x);return true;
            }
        }
        if (e.kind == PointerKind::move) {
            auto* target = tooltip_hit(e.position);
            if (!target || target->id != suppressed_tooltip) suppressed_tooltip.clear();
            schedule_tooltip(target);
        } else if (e.kind == PointerKind::down || e.kind == PointerKind::scroll ||
                   e.kind == PointerKind::leave || e.kind == PointerKind::cancel) {
            if (auto* target = tooltip_hit(e.position)) suppressed_tooltip = target->id;
            hide_tooltip();
        }
        if (e.kind == PointerKind::cancel) {
            const bool captured = !dragging.empty() || !text_drag.empty() || !pressed.empty() || hold.has_value();
            hold.reset();
            dragging.clear();
            text_drag.clear();
            pressed.clear();
            hovered.clear();
            inspection_capture = false;
            notify();
            return captured;
        }
        if (inspection_capture && e.kind == PointerKind::up && e.button == 0) {
            inspection_capture = false;
            return true;
        }
        if (inspection_picking && e.kind == PointerKind::move) {
            auto id = pick(e.position);
            if (id != inspection_hovered) {
                inspection_hovered = std::move(id);
                notify();
            }
            return true;
        }
        if (inspection_picking && e.kind == PointerKind::down) {
            if (e.button == 0) {
                auto id = pick(e.position);
                inspection_picking = false;
                inspection_hovered.clear();
                inspection_capture = true;
                pressed.clear();
                dragging.clear();
                select_inspected(std::move(id));
            }
            return true;
        }
        if (e.kind == PointerKind::down || e.kind == PointerKind::scroll) {
            focus_visible = false;
            notify();
        }
        if (e.kind == PointerKind::leave) {
            if (hold) {
                hold->due.reset();
                pressed.clear();
            }
            hovered.clear();
            notify();
            return false;
        }
        if (context_popup()) return context_pointer(e);
        if (!menu_popup() && e.kind == PointerKind::down && (e.button == 1 || (e.button == 0 && e.control))) {
            if (auto* target = attachment_hit(e.position, true)) {
                if (open_context(target->id, e.position)) return true;
            }
        }
        if (combo_popup() && combo_pointer(e)) return true;
        if (menu_popup()) {
            if (e.kind == PointerKind::scroll && menu_popup()->bounds.contains(e.position)) {
                double maximum = std::max(0., menu_popup()->options.size() * theme.popup_item_height -
                                                  menu_popup()->bounds.height + 2 * theme.popup_padding);
                menu_popup()->scroll = std::clamp(menu_popup()->scroll - e.scroll_y, 0., maximum);
                notify();
                return true;
            }
            if (e.kind == PointerKind::down) {
                if (e.button != 0)
                    return true;
                if (!menu_popup()->bounds.contains(e.position)) {
                    close_popup();
                    pressed = "@dismissed";
                    return true;
                }
                int i = popup_hit(e.position);
                if (i >= 0 && menu_popup()->options[i].enabled) {
                    menu_popup()->highlighted = i;
                    pressed = "@popup/" + menu_popup()->options[i].id;
                }
                return true;
            }
            if (e.kind == PointerKind::move) {
                int i = popup_hit(e.position);
                if (i >= 0 && menu_popup()->options[i].enabled)
                    menu_popup()->highlighted = i;
                notify();
                return true;
            }
            if (e.kind == PointerKind::up) {
                int i = popup_hit(e.position);
                auto p = std::exchange(pressed, {});
                if (e.button == 0 && i >= 0 && p == "@popup/" + menu_popup()->options[i].id)
                    choose(i);
                return true;
            }
            return true;
        }
        if (const auto* overlay = interactive_overlay(); overlay && overlay->popover && !overlay->bounds.contains(e.position)) {
            if (e.kind == PointerKind::down) {
                const auto id = overlay->root_id;
                const bool dismiss = overlay->popover->options.dismiss_on_outside;
                if (dismiss) dismiss_popover(id);
                pressed = "@dismissed";
            }
            if (e.kind == PointerKind::up) pressed.clear();
            // Captured drags can leave the overlay; an ordinary outside event
            // never falls through to the scene beneath it.
            if (dragging.empty() && text_drag.empty() && !hold) return true;
        }
        if (e.kind == PointerKind::up && pressed == "@dismissed") {
            pressed.clear();
            return true;
        }
        if (hold && (e.kind == PointerKind::move || (e.kind == PointerKind::up && e.button == 0))) {
            if (e.kind == PointerKind::up) {
                hold.reset();
                pressed.clear();
                notify();
                return true;
            }
            hold->point = e.position;
            const auto* button = find(hold->button);
            const auto* owner = find(hold->owner);
            const bool inside = button && button->clip.contains(e.position);
            pressed = inside ? hold->button : std::string{};
            if (!inside)
                hold->due.reset();
            else if (!hold->due && owner)
                hold->due = options.clock() + owner->d.numeric->repeat_interval;
            notify();
            return true;
        }
        if (e.kind == PointerKind::scroll) {
            auto* over = hit(e.position, true);
            if (!over) return false;
            Point remaining{-e.scroll_x, -e.scroll_y};
            if (e.shift && e.scroll_x == 0) { remaining.x = remaining.y; remaining.y = 0; }
            bool changed = false;
            if (over->d.kind == Kind::tab_option || over->d.kind == Kind::tab_bar) {
                if (const auto* container = find(over->group); container && container->enabled) {
                    auto& state = tabs.at(container->id);
                    auto& movement = remaining.x != 0 ? remaining.x : remaining.y;
                    const auto next = std::clamp(state.offset + movement, 0., state.maximum);
                    movement -= next - state.offset;
                    changed = changed || next != state.offset;
                    state.offset = next;
                    state.reveal = false;
                }
            }
            if (over->d.kind == Kind::text_field) {
                auto& editor = *editors.at(over->id);
                const auto content = field_viewport(*over);
                const Point next{std::clamp(editor.scroll.x + remaining.x, 0., std::max(0., over->text->size.width - content.width)),
                                 std::clamp(editor.scroll.y + remaining.y, 0., std::max(0., over->text->size.height - content.height))};
                remaining.x -= next.x - editor.scroll.x;
                remaining.y -= next.y - editor.scroll.y;
                if (next.x != editor.scroll.x || next.y != editor.scroll.y) {
                    editor.scroll = next;
                    editor.reveal_caret = false;
                    changed = true;
                }
            }
            std::vector<Node*> chain;
            for (const auto& [id, node] : nodes)
                if ((node->d.kind == Kind::scroll || node->d.kind == Kind::listbox) && !node->hidden && node->enabled &&
                    node->overlay_scope == over->overlay_scope && node->clip.contains(e.position) &&
                    (over->id == id || over->id.starts_with(id + "/"))) chain.push_back(node);
            std::sort(chain.begin(), chain.end(), [](const auto* a, const auto* b) { return a->id.size() > b->id.size(); });
            for (const auto* node : chain) {
                auto& offset = scroll[node->id];
                const Point next{std::clamp(offset.x + remaining.x, 0., node->scroll_max.x),
                                 std::clamp(offset.y + remaining.y, 0., node->scroll_max.y)};
                remaining.x -= next.x - offset.x;
                remaining.y -= next.y - offset.y;
                changed = changed || next.x != offset.x || next.y != offset.y;
                offset = next;
            }
            if (changed) { geometry_dirty = true; layout(viewport); notify(); }
            return changed;
        }
        if (e.kind == PointerKind::move && !text_drag.empty()) {
            auto* target = find(text_drag);
            if (target && target->text && target->text->hit_test) {
                auto& editor = *editors.at(text_drag);
                const auto next = target->text->hit_test(
                    {e.position.x - target->text_origin.x, e.position.y - target->text_origin.y});
                editor.buffer.select({text_drag_anchor, next});
                field_changed(*target, false);
            }
            return true;
        }
        if (e.kind == PointerKind::up && !text_drag.empty()) {
            text_drag.clear();
            pressed.clear();
            notify();
            return true;
        }
        auto* n = hit(e.position);
        if(e.kind==PointerKind::down&&e.button==0&&n){
            if(n->d.kind==Kind::table_resize){
                const auto owner=n->table_owner,column=n->column,node=n->id;
                focus(node,false);pressed.clear();dragging=node;
                column_drag=ColumnDrag{owner,column,e.position.x,tables.at(owner).widths.at(column)};notify();return true;
            }
            if(n->d.kind==Kind::table_column){
                const auto owner=n->table_owner,column=n->column;focus(n->id,false);pressed.clear();
                table_sort(owner,column,e.shift);return true;
            }
            if(n->d.kind==Kind::table_cell){
                const auto owner=n->table_owner;const TableCell cell{n->selection,n->column};
                focus_visible=false;pressed.clear();table_select(owner,cell,e.shift,e.command);
                if(e.click_count>=2)table_edit(owner,cell);return true;
            }
        }
        if (e.kind == PointerKind::down && e.button == 0 && n && n->id != focused) {
            const auto* previous = field_owner(focused);
            const auto* next = field_owner(n->id);
            if (previous && (!next || next->id != previous->id)) {
                finish_editing();
                layout(viewport);
                n = hit(e.position);
            }
        }
        if (e.kind == PointerKind::down && e.button == 0 && n && n->d.kind == Kind::field_action) {
            const auto id = n->id, owner = n->group;
            if (focus(owner, false) && find(id) && interactive(*find(id)))
                pressed = id;
            notify();
            return true;
        }
        if (e.kind == PointerKind::down && e.button == 0 && n && n->d.kind == Kind::step_button) {
            const auto owner_id = n->group;
            const auto button_id = n->id;
            const auto steps = n->number;
            if (!focus(owner_id, false))
                return true;
            const auto* owner = find(owner_id);
            if (!owner || !owner->d.numeric)
                return true;
            hold = Hold{button_id, owner_id, steps, e.position, options.clock() + owner->d.numeric->repeat_delay};
            pressed = button_id;
            adjust(owner_id, steps);
            return true;
        }
        if (e.kind == PointerKind::down && e.button == 0 && n && n->d.kind == Kind::list_option) {
            auto* parent = find(n->group);
            if (!parent)
                return false;
            const auto selected = n->selection;
            if (n->tree_item && n->tree_item->expandable) {
                const auto item = *n->tree_item;
                const auto id = parent->id;
                const auto leading = n->bounds.x + 10 + std::min(item.depth * parent->d.tree->options.indentation,
                                                                                std::max(0., n->bounds.width - 100));
                if (e.position.x < leading + 22 || e.click_count >= 2) {
                    focus(id, false);
                    if (lists.contains(id)) lists.at(id).active = selected;
                    return tree_expand(id, selected, !item.expanded);
                }
            }
            const auto callback = parent->d.list_options.on_activate;
            unmark_text(true);
            focus_visible = false;
            const bool result = list_select(*parent, selected, e.shift, e.command);
            if (result && e.click_count >= 2 && callback)
                callback(selected);
            pressed.clear();
            return result;
        }
        if (e.kind == PointerKind::down && e.button == 1 && n && n->d.kind == Kind::text_field) {
            if (focused != n->id) {
                focus(n->id, false);
                const auto next = n->text->hit_test ? n->text->hit_test({e.position.x - n->text_origin.x,
                                                                         e.position.y - n->text_origin.y})
                                                    : 0;
                editors.at(n->id)->buffer.select({next, next});
                field_changed(*n, false);
            }
            return true;
        }
        if (e.kind == PointerKind::down && e.button == 0 && n && n->d.kind == Kind::text_field) {
            focus(n->id, false);
            unmark_text(true);
            layout(viewport);
            n = find(focused);
            auto& editor = *editors.at(n->id);
            const auto next =
                n->text && n->text->hit_test
                    ? n->text->hit_test({e.position.x - n->text_origin.x, e.position.y - n->text_origin.y})
                    : 0;
            auto anchor = e.shift ? editor.buffer.selection().anchor : next;
            editor.buffer.select({anchor, next});
            if (e.click_count >= 3) {
                editor.buffer.command(TextCommand::line_start);
                anchor = editor.buffer.selection().caret;
                editor.buffer.command(TextCommand::line_end, true);
            } else if (e.click_count == 2) {
                editor.buffer.command(TextCommand::word_right);
                editor.buffer.command(TextCommand::word_left, true);
                anchor = editor.buffer.selection().anchor;
            }
            text_drag_anchor = anchor;
            text_drag = n->id;
            pressed = n->id;
            field_changed(*n, false);
            return true;
        }
        if (e.kind == PointerKind::down && n && focused != n->id)
            unmark_text(true);
        std::string id = n ? n->id : "";
        if (e.kind == PointerKind::move) {
            if (!dragging.empty()) {
                auto* slider = find(dragging);
                if (slider) {
                    if (slider->d.kind == Kind::split_divider) {
                        const auto* parent = find(slider->group);
                        if (parent) {
                            const auto s = split_metrics(*parent);
                            const auto r = content_box(*parent);
                            const auto position = s.row ? e.position.x - r.x : e.position.y - r.y;
                            if (s.available > 0)
                                set_value(slider->id, (position - divider_drag_offset - s.divider / 2) / s.available);
                        }
                        return true;
                    }
                    auto r = slider->bounds;
                    double t = std::clamp((e.position.x - r.x - 8) / std::max(1., r.width - 16), 0., 1.);
                    set_value(slider->id,
                              slider->d.range.min + t * (slider->d.range.max - slider->d.range.min));
                }
                return true;
            }
            if (hovered != id) {
                hovered = id;
                notify();
            }
            return n != nullptr;
        }
        if (e.button != 0)
            return false;
        if (e.kind == PointerKind::down) {
            pressed = id;
            if (n) {
                focused = id;
                active = true;
                split_arrow = n->d.kind == Kind::split &&
                              e.position.x >= n->bounds.x + n->bounds.width - theme.split_width;
                if (n->d.kind == Kind::slider) {
                    dragging = id;
                    pointer({PointerKind::move, e.position});
                } else if (n->d.kind == Kind::split_divider) {
                    auto* parent = find(n->group);
                    if (parent && e.click_count >= 2 && parent->d.split_options.collapsed) {
                        pressed.clear();
                        return collapse_split(*parent, e.shift ? SplitPane::second : SplitPane::first);
                    }
                    if (parent) {
                        dragging = id;
                        divider_drag_initial = parent->number;
                        divider_drag_collapsed = parent->collapsed;
                        const bool row = parent->d.orientation == Orientation::horizontal;
                        divider_drag_offset = row ? e.position.x - n->bounds.x - n->bounds.width / 2
                                                  : e.position.y - n->bounds.y - n->bounds.height / 2;
                    }
                }
                notify();
                return true;
            }
            return false;
        }
        if (e.kind == PointerKind::up) {
            bool was_drag = !dragging.empty();
            dragging.clear();
            auto prior = std::exchange(pressed, {});
            notify();
            if (n && prior == id && !was_drag) {
                bool arrow = n->d.kind == Kind::split &&
                             e.position.x >= n->bounds.x + n->bounds.width - theme.split_width;
                if (n->d.kind == Kind::split && arrow != split_arrow)
                    return true;
                return activate(id, arrow);
            }
            return was_drag || !prior.empty();
        }
        return false;
    }
    Node* alert_owner() const {
        Node* owner = nullptr;
        for (const auto& [id, node] : nodes)
            if (node->d.alert && node->enabled && !node->hidden && in_focus_scope(*node) &&
                (id == focused || focused.starts_with(id + "/") || (focused.empty() && node == root.get())) &&
                (!owner || id.size() > owner->id.size()))
                owner = node;
        return owner;
    }
    bool alert_response(bool cancel) {
        const auto* owner = alert_owner();
        if (!owner)
            return false;
        const auto respond = owner->d.alert->respond;
        const bool handled = respond(cancel);
        if (handled) {
            invalidate();
            notify();
        }
        return handled;
    }
    bool key(KeyEvent e) {
        layout(viewport);
        if (!e.down)
            return false;
        if (e.repeat && (e.key == Key::enter || e.key == Key::escape) && alert_owner())
            return true;
        if (e.key == Key::escape && field() && !menu_popup() && editors.at(field()->id)->buffer.marked_range())
            return text_command(TextCommand::cancel, false);
        const bool tooltip_was_shown = tooltip && tooltip->shown;
        hide_tooltip();
        if (e.key == Key::escape && tooltip_was_shown) return true;
        if (context_popup()) return context_key(e);
        if(const auto* day=find(focused);day&&day->calendar_date&&calendar_key(*day,e))return true;
        if(auto* date=field();date&&date->d.date&&e.alt&&!e.command&&!e.control&&!e.shift&&
            (e.key==Key::down||e.key==Key::up)){
            if(e.key==Key::up&&calendars.at(date->id).open){calendars.at(date->id).open=false;invalidate();layout(viewport);return true;}
            return open_calendar(date->id);
        }
        if(!menu_popup()&&!(e.key==Key::tab&&e.control))
            if(auto* editor=field();editor&&!editor->table_owner.empty())
                if(table_key(editor->table_owner,e))return true;
        if (auto* n = field(); n && n->d.combo && !editors.at(n->id)->buffer.marked_range()) {
            if (e.key == Key::escape && combos.at(n->id).open) { finish_combo(n->id, true); return true; }
            if (e.alt && !e.shift && !e.command && !e.control && (e.key == Key::down || e.key == Key::up)) {
                if (e.key == Key::down) return open_combo(n->id);
                combos.at(n->id).open = false; invalidate(); layout(viewport); return true;
            }
        }
        if (e.key == Key::f10 && e.shift && !e.command && !e.control && !e.alt) {
            if (e.repeat) return true;
            auto* focused_node = find(focused);
            Node* target = nullptr;
            for (const auto& [id, node] : nodes)
                if (focused_node && !node->context_menu.empty() && node->overlay_scope == focused_node->overlay_scope &&
                    (id == focused || focused.starts_with(id + "/")) && (!target || id.size() > target->id.size())) target = node;
            return target && open_context(target->id, {});
        }
        if (e.key == Key::escape) {
            if (const auto* overlay = interactive_overlay(); overlay && overlay->popover) {
                const auto id = overlay->root_id;
                if (overlay->popover->options.dismiss_on_escape) dismiss_popover(id);
                return true;
            }
        }
        if (e.key == Key::f && e.command && !menu_popup())
            return focus_search({});
        if (hold && e.key == Key::escape) {
            hold.reset();
            pressed.clear();
            notify();
            return true;
        }
        if (e.key == Key::tab && e.control && !e.alt && !e.command && !menu_popup()) {
            Node* container = nullptr;
            for (const auto& [id, node] : nodes)
                if (node->d.kind == Kind::tab_view && node->enabled && !node->hidden &&
                    focused.starts_with(id + "/") && (!container || id.size() > container->id.size()))
                    container = node;
            if (container)
                return select_tab(container->id, next_tab(*container, container->selection, e.shift ? -1 : 1), true);
            return false;
        }
        if(e.key==Key::escape&&column_drag){
            const auto drag=*column_drag;column_drag.reset();dragging.clear();pressed.clear();
            table_resize(drag.table,drag.column,drag.initial);return true;
        }
        if (e.key == Key::escape && !dragging.empty()) {
            auto* divider = find(dragging);
            if (divider && divider->d.kind == Kind::split_divider) {
                if (auto* parent = find(divider->group)) {
                    const auto d = parent->d;
                    dragging.clear();
                    pressed.clear();
                    d.write_number(divider_drag_initial);
                    if (d.split_options.collapsed)
                        d.split_options.collapsed->set(divider_drag_collapsed);
                    invalidate();
                    return true;
                }
            }
        }
        if (inspection_picking && e.key == Key::escape) {
            inspection_picking = false;
            inspection_hovered.clear();
            notify();
            return true;
        }
        // Tab mode persists without a timer. Other key actions end it; key-up and
        // passive pointer movement leave the current mode alone.
        if(!menu_popup()&&!context_popup()){
            std::string table;
            if(auto* node=find(focused);node){if(node->d.table)table=node->id;else table=node->table_owner;}
            if(!table.empty()&&table_key(table,e))return true;
        }
        if (auto* selected = find(focused); selected && selected->d.kind == Kind::listbox && !menu_popup())
            if (list_key(*selected, e))
                return true;
        if (field() && !menu_popup() && e.key != Key::tab) {
            if (e.command || e.control) {
                if (e.key == Key::a)
                    return text_command(TextCommand::select_all, false);
                if (e.key == Key::c)
                    return text_command(TextCommand::copy, false);
                if (e.key == Key::x)
                    return text_command(TextCommand::cut, false);
                if (e.key == Key::v)
                    return text_command(TextCommand::paste, false);
                if (e.key == Key::z)
                    return text_command(e.shift ? TextCommand::redo : TextCommand::undo, false);
                if (e.key == Key::y)
                    return text_command(TextCommand::redo, false);
            }
            switch (e.key) {
            case Key::left:
                return text_command(e.command            ? TextCommand::line_start
                                    : e.alt || e.control ? TextCommand::word_left
                                                         : TextCommand::left,
                                    e.shift);
            case Key::right:
                return text_command(e.command            ? TextCommand::line_end
                                    : e.alt || e.control ? TextCommand::word_right
                                                         : TextCommand::right,
                                    e.shift);
            case Key::up:
                return text_command(e.command ? TextCommand::document_start
                                    : e.alt   ? TextCommand::paragraph_start
                                              : TextCommand::up,
                                    e.shift);
            case Key::down:
                return text_command(e.command ? TextCommand::document_end
                                    : e.alt   ? TextCommand::paragraph_end
                                              : TextCommand::down,
                                    e.shift);
            case Key::home:
                return text_command(TextCommand::document_start, e.shift);
            case Key::end:
                return text_command(TextCommand::document_end, e.shift);
            case Key::page_up:
                return text_command(TextCommand::page_up, e.shift);
            case Key::page_down:
                return text_command(TextCommand::page_down, e.shift);
            case Key::backspace:
                return text_command(e.command            ? TextCommand::delete_line_backward
                                    : e.alt || e.control ? TextCommand::delete_word_backward
                                                         : TextCommand::backspace,
                                    false);
            case Key::delete_forward:
                return text_command(e.command            ? TextCommand::delete_line_forward
                                    : e.alt || e.control ? TextCommand::delete_word_forward
                                                         : TextCommand::delete_forward,
                                    false);
            case Key::enter:
                return field()->d.text_options.multiline && !e.command
                           ? insert_text("\n", {}, false)
                           : text_command(TextCommand::submit, false);
            case Key::escape:
                return text_command(TextCommand::cancel, false);
            default:
                break;
            }
        }
        focus_visible = e.key == Key::tab;
        notify();
        if (menu_popup()) {
            if (e.key == Key::escape) {
                close_popup();
                return true;
            }
            if (e.key == Key::tab) {
                close_popup();
                return key(e);
            }
            if (e.key == Key::enter || e.key == Key::space) {
                if (!e.repeat)
                    choose(menu_popup()->highlighted);
                return true;
            }
            if (e.key == Key::down || e.key == Key::up || e.key == Key::home || e.key == Key::end) {
                int count = static_cast<int>(menu_popup()->options.size()),
                    direction = e.key == Key::up || e.key == Key::end ? -1 : 1, index = menu_popup()->highlighted;
                if (e.key == Key::home)
                    index = -1;
                if (e.key == Key::end)
                    index = count;
                for (int j = 0; j < count; ++j) {
                    index = (index + direction + count) % count;
                    if (menu_popup()->options[index].enabled) {
                        menu_popup()->highlighted = index;
                        break;
                    }
                }
                position_popup(true);
                notify();
                return true;
            }
            return true;
        }
        if (e.key == Key::tab) {
            auto list = focusables();
            if (list.empty())
                return focus_scope() != nullptr;
            int index = e.shift ? 0 : -1;
            for (std::size_t i = 0; i < list.size(); ++i)
                if (list[i]->id == focused ||
                    (!list[i]->group.empty() && find(focused) && find(focused)->group == list[i]->group)) {
                    index = static_cast<int>(i);
                    break;
                }
            index =
                (index + (e.shift ? -1 : 1) + static_cast<int>(list.size())) % static_cast<int>(list.size());
            return focus(list[index]->id);
        }
        auto* n = find(focused);
        if (!e.command && !e.control && !e.alt && !e.shift && !e.repeat &&
            ((e.key == Key::escape && alert_response(true)) ||
             (e.key == Key::enter && (!n || n->d.kind != Kind::button) && alert_response(false))))
            return true;
        n = find(focused);
        if (!n || !interactive(*n))
            return false;
        if (n->d.kind == Kind::stepper) {
            if (e.key == Key::up || e.key == Key::right || e.key == Key::down || e.key == Key::left ||
                e.key == Key::page_up || e.key == Key::page_down)
                return adjust(n->id, e.key == Key::up || e.key == Key::right ? 1 :
                                      e.key == Key::down || e.key == Key::left ? -1 :
                                      e.key == Key::page_up ? 10 : -10);
            if (e.key == Key::home || e.key == Key::end) {
                if (n->d.numeric->read_only)
                    return false;
                write_numeric_value(*n, e.key == Key::home ? n->d.numeric->min : n->d.numeric->max);
                return true;
            }
        }
        if (n->d.kind == Kind::tab_option && (e.key == Key::left || e.key == Key::right ||
                                             e.key == Key::home || e.key == Key::end)) {
            const auto* container = find(n->group);
            if (!container)
                return false;
            const auto direction = e.key == Key::left || e.key == Key::end ? -1 : 1;
            return select_tab(container->id, next_tab(*container, n->selection, direction,
                                                     e.key == Key::home || e.key == Key::end));
        }
        if(n->d.kind==Kind::table_resize){
            if(e.key==Key::left||e.key==Key::right)return adjust(n->id,e.key==Key::left?-1:1);
            if(e.key==Key::home||e.key==Key::end){
                const auto& model=find(n->table_owner)->d.table->model;
                const auto& col=model.columns()[*model.column_index(n->column)];
                return table_resize(n->table_owner,n->column,e.key==Key::home?col.min_width:col.max_width);
            }
        }
        if (n->d.kind == Kind::split_divider) {
            auto* parent = find(n->group);
            if (!parent)
                return false;
            const auto s = split_metrics(*parent);
            const bool backwards = s.row ? e.key == Key::left : e.key == Key::up;
            const bool forwards = s.row ? e.key == Key::right : e.key == Key::down;
            if (backwards || forwards)
                return adjust(n->id, (backwards ? -1. : 1.) * (e.shift ? 10 : 1));
            if (e.key == Key::home || e.key == Key::end)
                return set_value(n->id, e.key == Key::home ? 0 : 1);
            if (e.key == Key::enter || e.key == Key::space)
                return e.repeat || collapse_split(*parent, e.shift ? SplitPane::second : SplitPane::first);
            if (e.key == Key::escape && parent->collapsed != SplitPane::none)
                return collapse_split(*parent, SplitPane::none);
            return false;
        }
        if ((n->d.kind == Kind::radio_option || n->d.kind == Kind::segment_option) &&
            (e.key == Key::left || e.key == Key::right || e.key == Key::up || e.key == Key::down ||
             e.key == Key::home || e.key == Key::end)) {
            auto* parent = find(n->group);
            if (!parent)
                return false;
            int count = static_cast<int>(parent->children.size()), index = 0;
            for (int i = 0; i < count; ++i)
                if (parent->children[i]->id == n->id)
                    index = i;
            int dir = e.key == Key::left || e.key == Key::up || e.key == Key::end ? -1 : 1;
            if (e.key == Key::home)
                index = -1;
            if (e.key == Key::end)
                index = count;
            for (int j = 0; j < count; ++j) {
                index = (index + dir + count) % count;
                if (parent->children[index]->enabled) {
                    auto id = parent->children[index]->id;
                    focus(id, false);
                    return activate(id);
                }
            }
            return true;
        }
        if (n->d.kind == Kind::slider) {
            double value = n->number;
            switch (e.key) {
            case Key::left:
            case Key::down:
                value -= n->d.range.step;
                break;
            case Key::right:
            case Key::up:
                value += n->d.range.step;
                break;
            case Key::page_up:
                value += n->d.range.page_step;
                break;
            case Key::page_down:
                value -= n->d.range.page_step;
                break;
            case Key::home:
                value = n->d.range.min;
                break;
            case Key::end:
                value = n->d.range.max;
                break;
            default:
                goto activation;
            }
            return set_value(n->id, value);
        }
    activation:
        if ((n->d.kind == Kind::menu || n->d.kind == Kind::selection || n->d.kind == Kind::split ||
             n->d.kind == Kind::split_arrow) &&
            e.key == Key::down) {
            if (n->d.kind == Kind::split_arrow) {
                if (auto* parent = find(n->group))
                    open_popup(*parent);
            } else
                open_popup(*n);
            return true;
        }
        if (e.key == Key::enter || e.key == Key::space) {
            if (e.repeat)
                return true;
            return activate(n->id);
        }
        return false;
    }
    Element element(const Node& n) const {
        Element e;
        e.id = n.id;
        e.label = n.d.accessible.empty() ? n.d.text : n.d.accessible;
        e.group = n.group;
        e.bounds = n.bounds;
        e.clip = n.clip;
        e.enabled = n.enabled && in_focus_scope(n);
        e.secure = n.d.password.has_value();
        e.search = n.d.search.has_value();
        e.combo = n.d.combo.has_value();
        e.calendar_date=n.calendar_date;
        if (n.d.combo) {
            const auto& state = combos.at(n.id);
            e.expanded = state.open;
            e.invalid = !state.validation.empty(); e.help = state.validation;
        }
        if (n.d.tooltip) { if (!e.help.empty()) e.help += " "; e.help += n.d.tooltip->text; }
        e.focused = active && !menu_popup() && focused == n.id;
        e.selected = n.number == 1;
        e.mixed = n.d.kind == Kind::checkbox && n.number == 2;
        e.value = n.number;
        e.expanded = n.d.date ? calendars.at(n.id).open : n.d.combo ? combos.at(n.id).open && !editors.at(n.id)->buffer.marked_range() : n.expanded;
        if (n.d.numeric) {
            e.selected = false;
            e.min = std::visit([](auto value) { return static_cast<double>(value); }, n.d.numeric->min);
            e.max = std::visit([](auto value) { return static_cast<double>(value); }, n.d.numeric->max);
            e.read_only = n.d.numeric->read_only;
            if (n.d.kind == Kind::text_field) {
                const auto& validation = editors.at(n.id)->validation;
                if (!validation.message.empty()) {
                    if (!e.help.empty()) e.help += " ";
                    e.help += validation.message;
                }
                e.invalid = validation.status != NumericStatus::valid;
            } else if (*n.numeric_value < n.d.numeric->min || *n.numeric_value > n.d.numeric->max) {
                e.invalid = true;
                e.help = "Value is outside the configured range.";
            }
        }
        switch (n.d.kind) {
        case Kind::table_header: e.role=Role::group;e.label="Column headers";e.group=n.table_owner;break;
        case Kind::table_column: {
            e.role=Role::column_header;e.selected=false;e.table_id=n.table_owner;
            if(auto* table=find(n.table_owner)){
                e.column_index=table->d.table->model.column_index(n.column);e.column_count=table->d.table->model.columns().size();
                if(e.column_index)e.sortable=table->d.table->model.columns()[*e.column_index].sortable;
                if(e.column_index)if(auto info=logical_column(table->id,*e.column_index)){e.sort_order=info->sort_order;e.sort_priority=info->sort_priority;}
            }
            break;
        }
        case Kind::table_resize: {
            e.role=Role::splitter;e.selected=false;e.orientation=Orientation::horizontal;
            if(auto* table=find(n.table_owner))if(auto index=table->d.table->model.column_index(n.column)){
                const auto& col=table->d.table->model.columns()[*index];e.value=tables.at(n.table_owner).widths.at(n.column);e.min=col.min_width;e.max=col.max_width;
            }
            break;
        }
        case Kind::table_cell: {
            e.role=Role::table_cell;e.table_id=n.table_owner;e.cell=TableCell{n.selection,n.column};e.item_index=n.row_index;
            const auto& state=tables.at(n.table_owner);
            const auto* table=find(n.table_owner);
            e.column_index=table->d.table->model.column_index(n.column);e.column_count=table->d.table->model.columns().size();
            e.value_text=n.d.text;
            e.selected=table->d.table->options.selection_mode==TableSelectionMode::cells?state.selected_cells.contains(n.id):lists.at(n.table_owner).selected.contains(n.selection);
            e.focused=active&&table->d.table->options.selection_mode==TableSelectionMode::cells&&focused==n.table_owner&&lists.at(n.table_owner).active==n.selection&&state.column==n.column;
            e.read_only=!table->d.table->model.columns()[*e.column_index].commit;
            if(state.edit&&state.edit->cell==*e.cell){e.value_text=state.edit->draft.get();e.help=state.edit->error;e.invalid=!e.help.empty();}
            break;
        }
        case Kind::field_action:
            e.role = Role::button;
            e.selected = false;
            break;
        case Kind::stepper:
            e.role = Role::stepper;
            e.value_text = n.numeric_text;
            e.selected = false;
            break;
        case Kind::step_button:
            e.role = Role::button;
            e.selected = false;
            break;
        case Kind::switch_control:
            e.role = Role::switch_control;
            break;
        case Kind::tab_view:
            e.role = Role::tab_group;
            e.value_text = n.selection;
            e.selected = false;
            break;
        case Kind::tab_page:
            e.role = Role::group;
            break;
        case Kind::tab_option:
            e.role = Role::tab;
            break;
        case Kind::split_view:
            e.role = Role::group;
            e.selected = false;
            break;
        case Kind::split_divider: {
            e.role = Role::splitter;
            e.selected = false;
            e.orientation = n.d.orientation;
            if (const auto* parent = find(n.group)) {
                const auto s = split_metrics(*parent);
                if (parent->collapsed == SplitPane::none && s.available > 0) {
                    e.min = s.low / s.available;
                    e.max = s.high / s.available;
                }
            }
            break;
        }
        case Kind::image:
            e.role = Role::image;
            break;
        case Kind::listbox:
            e.role = n.d.table ? Role::table : n.d.tree ? Role::tree_view : Role::list_box;
            if(n.d.table){
                e.table_id=n.id;e.column_count=n.d.table->model.columns().size();
                for(const auto& cell:tables.at(n.id).source.cells)if(n.d.rows.index_of(cell.row)&&n.d.table->model.column_index(cell.column))e.selected_cells.push_back(table_cell_id(n.id,cell));
            }
            e.selected = false;
            e.item_count = n.d.rows.size();
            if (find(n.id + "/option/" + path_key(lists.at(n.id).active))) e.focused = false;
            for (const auto& id : lists.at(n.id).selected)
                if (auto index = n.d.rows.index_of(id)) e.selected_items.push_back(*index);
            std::sort(e.selected_items.begin(), e.selected_items.end());
            if(n.d.table&&n.d.table->options.selection_mode==TableSelectionMode::cells)e.selected_items.clear();
            break;
        case Kind::list_option:
            e.role = find(n.group) && find(n.group)->d.table ? Role::table_row : n.tree_item ? Role::tree_item : Role::list_item;
            e.tree = n.tree_item;
            if (n.tree_item && n.tree_item->loading) e.help = "Loading children";
            e.item_index = n.row_index;
            if(const auto* table=find(n.group);table&&table->d.table){e.table_id=table->id;e.column_count=table->d.table->model.columns().size();}
            e.focused = active && !menu_popup() && !context_popup() && in_focus_scope(n) && focused == n.group && lists.at(n.group).active == n.selection;
            if(const auto* parent=find(n.group);parent&&parent->d.table&&parent->d.table->options.selection_mode==TableSelectionMode::cells){e.focused=false;e.selected=false;}
            break;
        case Kind::text_field: {
            e.role = Role::text_field;
            const auto& buffer = editors.at(n.id)->buffer;
            e.value_text = e.secure ? password_mask(buffer.text()) : buffer.text();
            e.text_selection = e.secure ? TextSelection{} : buffer.selection();
            e.read_only = n.d.text_options.read_only;
            e.multiline = n.d.text_options.multiline;
            break;
        }
        case Kind::label:
            e.role = Role::text;
            break;
        case Kind::toggle:
            e.role = Role::toggle;
            break;
        case Kind::checkbox:
            e.role = Role::checkbox;
            break;
        case Kind::radio_option:
            e.role = Role::radio;
            break;
        case Kind::segment_option:
            e.role = Role::segment;
            break;
        case Kind::menu:
        case Kind::split_arrow:
            e.role = Role::menu_button;
            e.expanded = menu_popup() && menu_popup()->owner == (n.d.kind == Kind::split_arrow ? n.group : n.id);
            break;
        case Kind::selection:
            e.role = Role::selection;
            e.value_text = n.d.text;
            e.expanded = menu_popup() && menu_popup()->owner == n.id;
            break;
        case Kind::split:
            e.role = Role::split_button;
            e.value_text = n.d.text;
            e.bounds.width = std::max(0., e.bounds.width - theme.split_width);
            break;
        case Kind::slider:
            e.role = Role::slider;
            e.min = n.d.range.min;
            e.max = n.d.range.max;
            break;
        case Kind::progress:
            e.role = Role::progress;
            e.value_text = n.d.indeterminate ? "In progress" : "";
            break;
        case Kind::radio:
        case Kind::segments:
        case Kind::section:
        case Kind::group:
        case Kind::panel:
            e.role = Role::group;
            break;
        default:
            e.role = Role::button;
            break;
        }
        if ((n.d.tooltip || n.d.context_menu) && !interactive(n) && !text_kind(n.d.kind)) e.role = Role::group;
        if(n.d.datetime||n.datetime_component){
            const auto validation=datetime_validation(n.d.datetime?n.id:n.datetime_owner);
            if(validation.status!=NumericStatus::valid){
                e.invalid=true;
                if(!validation.message.empty()&&e.help.find(validation.message)==std::string::npos)e.help+=(e.help.empty()?"":" ")+validation.message;
            }
        }
        if(n.d.sample){
            e.value_text=format_color(*n.d.sample);
            if(!n.d.color)e.role=Role::image;
        }
        if(n.d.color){
            const auto& color=colors.at(n.id);
            const auto validation=color_status(n.id);
            e.expanded=color.open;e.read_only=n.d.color->options.read_only;
            e.invalid=validation.status!=NumericStatus::valid;
            e.help+=(e.help.empty()?"":" ")+std::string(n.d.color->options.live_preview?"Live preview color":"Apply/Cancel color draft");
            if(!validation.message.empty())e.help+=". "+validation.message;
        }
        if (n.d.alert)
            e.role = Role::alert;
        e.alert_action = n.d.alert_action;
        if (e.alert_action)
            e.help = *e.alert_action == AlertActionRole::default_action ? "Default action" :
                     *e.alert_action == AlertActionRole::cancel ? "Cancel action" :
                     *e.alert_action == AlertActionRole::destructive ? "Destructive action" : "";
        return e;
    }
    static std::string type_name(Kind kind) {
        switch (kind) {
        case Kind::table_header: return "TableHeader";
        case Kind::table_column: return "TableColumn";
        case Kind::table_resize: return "TableColumnResize";
        case Kind::table_cell: return "TableCell";
        case Kind::field_action:
            return "FieldAction";
        case Kind::stepper:
            return "Stepper";
        case Kind::step_button:
            return "StepButton";
        case Kind::switch_control:
            return "Switch";
        case Kind::tab_view:
            return "TabView";
        case Kind::tab_bar:
            return "TabStrip";
        case Kind::tab_option:
            return "Tab";
        case Kind::tab_page:
            return "TabPage";
        case Kind::split_view:
            return "SplitView";
        case Kind::split_divider:
            return "SplitDivider";
        case Kind::empty:
            return "Empty";
        case Kind::label:
            return "Label";
        case Kind::button:
            return "Button";
        case Kind::toggle:
            return "ToggleButton";
        case Kind::checkbox:
            return "CheckBox";
        case Kind::radio:
            return "RadioGroup";
        case Kind::radio_option:
            return "RadioOption";
        case Kind::segments:
            return "SegmentedControl";
        case Kind::segment_option:
            return "Segment";
        case Kind::menu:
            return "MenuButton";
        case Kind::selection:
            return "SelectionDropdown";
        case Kind::split:
            return "SplitActionButton";
        case Kind::split_arrow:
            return "SplitArrow";
        case Kind::fragment:
            return "Fragment";
        case Kind::image:
            return "Image";
        case Kind::listbox:
            return "ListBox";
        case Kind::list_option:
            return "ListItem";
        case Kind::text_field:
            return "TextField";
        case Kind::slider:
            return "Slider";
        case Kind::progress:
            return "ProgressBar";
        case Kind::row:
            return "FlexRow";
        case Kind::column:
            return "FlexColumn";
        case Kind::block:
            return "Block";
        case Kind::flow:
            return "Inline";
        case Kind::layer:
            return "Layer";
        case Kind::section:
            return "Section";
        case Kind::group:
            return "GroupBox";
        case Kind::panel:
            return "Panel";
        case Kind::separator:
            return "Separator";
        case Kind::scroll:
            return "ScrollView";
        case Kind::spacer:
            return "Spacer";
        }
        return "View";
    }
    std::vector<InspectionNode> inspect() {
        layout(viewport);
        std::vector<InspectionNode> out;
        std::function<void(Node&, const std::string&, std::size_t)> visit =
            [&](Node& n, const std::string& parent, std::size_t depth) {
                const auto& d = n.d;
                InspectionNode item;
                item.id = n.id;
                item.overlay_scope = n.overlay_scope;
                item.parent_id = parent;
                item.depth = depth;
                item.type = !d.semantic_type.empty() ? d.semantic_type : d.alert ? "Alert" : d.table ? "TableView" : d.tree ? "TreeView" : n.tree_item ? "TreeItem" : d.combo ? "ComboBox" : d.password ? "PasswordField" : d.search ? "SearchField" :
                    d.numeric && d.kind == Kind::text_field ? "NumericField" :
                    d.kind == Kind::text_field && d.text_options.multiline ? "TextEditor" : type_name(d.kind);
                if (n.popover_root) item.type = "Popover";
                if (n.tooltip_root) item.type = "Tooltip";
                if (d.alert)
                    item.alert_pending = d.alert->pending();
                item.alert_action = d.alert_action;
                if(d.color){
                    const auto& color=colors.at(n.id);
                    item.color=InspectionNode::ColorDraft{color.source,color.draft,color.open,d.color->options.live_preview,d.color->options.alpha,color.model,color_status(n.id)};
                }
                if(d.datetime){
                    const auto& state=datetimes.at(n.id);
                    item.date_time=InspectionNode::DateTimeDraft{state.source,state.draft,datetime_validation(n.id)};
                }
                item.calendar_date=n.calendar_date;
                if(n.calendar_date)item.type="CalendarDay";
                if(d.date)item.calendar_open=calendars.at(n.id).open;
                item.key = d.key;
                item.text = d.text;
                item.label = d.accessible.empty() ? d.text : d.accessible;
                item.selection = d.kind == Kind::text_field ?
                    (d.password ? password_mask(editors.at(n.id)->buffer.text()) : editors.at(n.id)->buffer.text()) : n.selection;
                item.secure = d.password.has_value();
                item.revealed = d.password && editors.at(n.id)->revealed;
                item.search_pending = d.search && editors.at(n.id)->pending_search.has_value();
                if (d.combo) {
                    const auto& state = combos.at(n.id);
                    item.combo = InspectionNode::Combo{state.open && !editors.at(n.id)->buffer.marked_range(), d.combo->loading, n.selection, state.highlighted,
                                                      state.validation, state.matches.size()};
                }
                item.bounds = n.bounds;
                item.clip = n.clip;
                item.content_bounds = inset(n.bounds, d.padding);
                item.content_bounds.y += heading(n);
                item.content_bounds.height = std::max(0., item.content_bounds.height - heading(n));
                item.measured = n.desired;
                item.padding = d.padding;
                item.margin = d.margin;
                item.width = d.width;
                item.height = d.height;
                item.basis = d.basis;
                item.min_width = d.min_width;
                item.min_height = d.min_height;
                item.max_width = d.max_width;
                item.max_height = d.max_height;
                item.gap = d.gap;
                item.grow = d.grow;
                item.shrink = d.shrink;
                item.align = d.align;
                item.self = d.self;
                item.justify = d.justify;
                item.wrap = d.wrap;
                item.layout_editable = n.layout_editable;
                item.layout_overridden = layout_overrides.contains(n.id);
                item.flex_lines = n.flex_lines;
                if (d.kind == Kind::split_view) {
                    const auto s = split_metrics(n);
                    item.split = SplitInspection{d.orientation, n.number, s.first, s.available - s.first,
                                                 s.divider, d.split_options.first_min, d.split_options.second_min,
                                                 n.collapsed};
                }
                if (d.kind == Kind::tab_view) {
                    const auto& state = tabs.at(n.id);
                    item.tabs = TabInspection{n.requested_selection, d.tabs.size(), state.pages.size(), state.offset};
                }
                if (d.numeric && d.kind == Kind::text_field)
                    item.numeric_validation = editors.at(n.id)->validation;
                if (d.kind == Kind::scroll || d.kind == Kind::listbox)
                    item.scroll = ScrollInspection{d.kind == Kind::listbox ? d.table ? ScrollAxes::both : ScrollAxes::vertical : d.scroll_options.axes,
                                                   scroll[n.id], n.scroll_max, n.scroll_content, content_box(n)};
                if (d.kind == Kind::listbox) {
                    const auto& state = lists.at(n.id);
                    std::size_t retained{};
                    for (const auto& [id, editor] : editors) if (id.starts_with(n.id + "/option/")) ++retained;
                    item.list = ListInspection{d.rows.size(), std::size_t(std::count_if(n.children.begin(),n.children.end(),[](const auto& row){return row->d.kind==Kind::list_option;})), retained, state.window, state.active, state.pinned};
                    if (d.tree) {
                        const auto& tree = trees.at(n.id);
                        item.tree = InspectionNode::Tree{d.tree->model.known_count(), tree.expanded_count, tree.loading_count};
                    }
                }
                item.tree_item = n.tree_item;
                if(d.table){
                    const auto& state=tables.at(n.id);
                    InspectionNode::Table table;
                    for(const auto& col:d.table->model.columns())table.columns.push_back({col.id,state.widths.at(col.id)});
                    table.sort=state.sort;table.selection_mode=d.table->options.selection_mode;
                    table.active={lists.at(n.id).active,state.column};
                    if(state.edit){table.editing=state.edit->cell;table.validation=state.edit->error;}
                    item.table=std::move(table);
                }
                item.position = d.position;
                item.left = d.left;
                item.top = d.top;
                item.right = d.right;
                item.bottom = d.bottom;
                item.z_index = d.z_index;
                item.hidden = n.hidden;
                item.own_hidden = d.hidden;
                item.font_size = n.font;
                item.line_height = n.line_height;
                item.letter_spacing = n.tracking;
                item.bold = d.bold || d.style == LabelStyle::heading || d.style == LabelStyle::section;
                item.corner_radius = d.radius.value_or(d.kind == Kind::group || d.kind == Kind::panel ? 8
                                                       : d.kind == Kind::checkbox                     ? 4
                                                       : d.kind == Kind::radio_option                 ? 10
                                                                                                      : 0);
                item.foreground = n.color;
                item.accent = n.accent;
                item.background = d.background;
                switch (d.kind) {
                case Kind::text_field:
                    item.background = d.background.value_or(theme.field);
                    item.corner_radius = d.radius.value_or(6);
                    break;
                case Kind::button:
                case Kind::toggle:
                case Kind::segment_option:
                case Kind::tab_option:
                case Kind::menu:
                case Kind::selection:
                case Kind::split: {
                    const bool selected = d.kind == Kind::button || d.kind == Kind::menu ||
                                          d.kind == Kind::selection || d.kind == Kind::split || n.number != 0;
                    item.background = fill(n, d.background.value_or(
                        d.alert_action == AlertActionRole::destructive ? theme.error :
                        d.alert_action == AlertActionRole::cancel || d.alert_action == AlertActionRole::normal ? theme.inactive :
                        selected ? n.accent : theme.inactive));
                    item.foreground =
                        n.enabled ? d.foreground.value_or(selected ? contrast(*item.background) : theme.text)
                                  : theme.disabled;
                    item.corner_radius =
                        d.radius.value_or(d.kind == Kind::segment_option ? 0 : theme.button_radius);
                    break;
                }
                case Kind::panel:
                    if (!item.background)
                        item.background = theme.panel;
                    break;
                case Kind::group:
                    if (!item.background)
                        item.background = theme.group;
                    break;
                default:
                    break;
                }
                if (d.read_number || d.numeric || d.kind == Kind::split_divider)
                    item.value = n.number;
                item.enabled = n.enabled;
                item.focused = active && focused == n.id;
                item.selected = n.number == 1 && !d.numeric && d.kind != Kind::split_view && d.kind != Kind::split_divider;
                item.expanded = d.combo ? combos.at(n.id).open && !editors.at(n.id)->buffer.marked_range() : n.expanded;
                item.interactive = interactive(n);
                out.push_back(std::move(item));
                for (auto& child : n.children)
                    visit(*child, n.id, depth + 1);
                for (auto& child : n.overlay_children)
                    visit(*child, n.id, depth + 1);
            };
        if (root)
            visit(*root, "", 0);
        if (menu_popup()) {
            auto owner =
                std::find_if(out.begin(), out.end(), [&](const auto& n) { return n.id == menu_popup()->owner; });
            const auto depth = owner == out.end() ? 1 : owner->depth + 1;
            InspectionNode overlay;
            overlay.id = menu_popup()->owner + "/overlay";
            overlay.parent_id = menu_popup()->owner;
            overlay.type = "PopupMenu";
            overlay.depth = depth;
            overlay.bounds = menu_popup()->bounds;
            overlay.clip = {0, 0, viewport.width, viewport.height};
            overlay.enabled = true;
            overlay.expanded = true;
            const auto* trigger = find(menu_popup()->owner);
            overlay.font_size = trigger ? trigger->font : 14;
            overlay.line_height = trigger ? trigger->line_height : 18;
            overlay.measured = {overlay.bounds.width, overlay.bounds.height};
            overlay.padding = Insets::all(theme.popup_padding);
            overlay.corner_radius = 6;
            overlay.foreground = theme.text;
            overlay.background = theme.panel;
            overlay.accent = theme.primary;
            out.push_back(overlay);
            for (std::size_t i = 0; i < menu_popup()->options.size(); ++i) {
                InspectionNode item;
                item.id = menu_popup()->owner + "/menu/" + path_key(menu_popup()->options[i].id);
                item.parent_id = overlay.id;
                item.type = "MenuItem";
                item.depth = depth + 1;
                item.label = menu_popup()->options[i].label;
                item.text = item.label;
                item.bounds = popup_item_rect(i);
                item.clip = inset(menu_popup()->bounds, Insets::all(theme.popup_padding));
                item.enabled = menu_popup()->options[i].enabled;
                item.interactive = item.enabled;
                item.focused = menu_popup()->highlighted == static_cast<int>(i);
                item.font_size = overlay.font_size;
                item.line_height = overlay.line_height;
                item.measured = {item.bounds.width, item.bounds.height};
                item.foreground = item.enabled ? theme.text : theme.disabled;
                item.background = item.focused ? theme.inactive : theme.panel;
                item.accent = theme.primary;
                item.corner_radius = 3;
                item.selected = trigger && trigger->selection == menu_popup()->options[i].id;
                out.push_back(std::move(item));
            }
        }
        for (const auto& menu : overlays) {
            if (!menu.context) continue;
            const auto parent_id = menu.context->parent_menu.empty() ? menu.owner : menu.context->parent_menu + "/item/" + path_key(menu.context->parent_item);
            const auto parent = std::find_if(out.begin(), out.end(), [&](const auto& n) { return n.id == parent_id; });
            InspectionNode group;
            group.id = menu.root_id; group.parent_id = parent_id; group.overlay_scope = menu.root_id;
            group.type = "ContextMenu"; group.label = "Context menu";
            group.depth = parent == out.end() ? 1 : parent->depth + 1;
            group.bounds = menu.bounds; group.clip = {0, 0, viewport.width, viewport.height};
            group.content_bounds = inset(menu.bounds, Insets::all(theme.popup_padding));
            group.enabled = true; group.background = theme.panel; group.foreground = theme.text;
            out.push_back(group);
            for (std::size_t i = 0; i < menu.context->entries.size(); ++i) {
                const auto& entry = menu.context->entries[i];
                InspectionNode item;
                item.id = context_item_id(menu, i); item.parent_id = group.id; item.overlay_scope = group.id;
                item.depth = group.depth + 1;
                item.type = entry.item.kind == MenuItemKind::separator ? "MenuSeparator" : "ContextMenuItem";
                item.label = entry.item.label; item.text = entry.item.label; item.key = entry.item.id;
                item.shortcut = entry.item.shortcut;
                item.bounds = context_item_rect(menu, i); item.clip = group.content_bounds;
                item.content_bounds = item.bounds; item.measured = {item.bounds.width, item.bounds.height};
                item.interactive = item.enabled = menu_selectable(entry); item.selected = entry.checked;
                item.focused = active && context_popup() == &menu && menu.highlighted == static_cast<int>(i);
                item.expanded = std::any_of(overlays.begin(), overlays.end(), [&](const auto& o) { return o.context && o.context->parent_menu == menu.root_id && o.context->parent_item == entry.item.id; });
                item.foreground = item.enabled ? theme.text : theme.disabled;
                out.push_back(std::move(item));
            }
        }
        if (!inspected.empty() &&
            std::none_of(out.begin(), out.end(), [&](const auto& n) { return n.id == inspected; })) {
            inspected.clear();
            inspection_flash_pending = false;
            inspection_flash_start.reset();
        }
        return out;
    }
    std::string pick(Point point) {
        for (auto it = overlays.rbegin(); it != overlays.rend(); ++it)
            if (it->context && it->bounds.contains(point)) {
                const auto index = context_hit(*it, point);
                return index >= 0 ? context_item_id(*it, index) : it->root_id;
            }
        if (menu_popup() && menu_popup()->bounds.contains(point)) {
            int item = popup_hit(point);
            return item >= 0 ? menu_popup()->owner + "/menu/" + path_key(menu_popup()->options[item].id)
                             : menu_popup()->owner + "/overlay";
        }
        std::function<Node*(Node&)> visit = [&](Node& n) -> Node* {
            if (n.hidden || !n.clip.contains(point))
                return nullptr;
            for (auto i = n.paint_children.rbegin(); i != n.paint_children.rend(); ++i)
                if (auto* hit = visit(**i))
                    return hit;
            return n.bounds.contains(point) ? &n : nullptr;
        };
        for (auto i = overlays.rbegin(); i != overlays.rend(); ++i)
            if (auto* body = find(i->root_id))
                if (auto* target = visit(*body)) return target->id;
        auto* hit = root ? visit(*root) : nullptr;
        return hit ? hit->id : std::string{};
    }
    void paint_inspection(SkCanvas& canvas, double time) {
        // Start at the first visible frame, even after a long idle interval.
        if (inspection_flash_pending) {
            inspection_flash_start = time;
            inspection_flash_pending = false;
        }
        double flash = 0;
        if (inspection_flash_start) {
            const double progress = std::clamp((time - *inspection_flash_start) / .48, 0., 1.);
            flash = (1 - progress) * (1 - progress);
            if (progress >= 1)
                inspection_flash_start.reset();
        }
        const auto& id = inspection_picking && !inspection_hovered.empty() ? inspection_hovered : inspected;
        if (id.empty())
            return;
        auto snapshot = inspect();
        auto target =
            std::find_if(snapshot.begin(), snapshot.end(), [&](const auto& n) { return n.id == id; });
        if (target == snapshot.end())
            return;
        if (target->hidden) {
            inspection_flash_pending = false;
            inspection_flash_start.reset();
            return;
        }
        const auto selected_bounds = intersect(target->bounds, target->clip);
        // Flex diagnostics follow the closest flex ancestor when a child is selected.
        if (inspection_overlay == InspectionOverlay::flex) {
            auto flex = target;
            while (flex != snapshot.end() && flex->type != "FlexRow" && flex->type != "FlexColumn") {
                auto parent = flex->parent_id;
                const auto scope = flex->overlay_scope;
                flex = std::find_if(snapshot.begin(), snapshot.end(),
                                    [&](const auto& n) { return n.id == parent && n.overlay_scope == scope; });
            }
            if (flex != snapshot.end())
                target = flex;
        }
        auto parent = std::find_if(snapshot.begin(), snapshot.end(),
                                   [&](const auto& n) { return n.id == target->parent_id; });
        const auto clip =
            parent == snapshot.end() || parent->overlay_scope != target->overlay_scope
                ? Rect{0, 0, viewport.width, viewport.height} : parent->clip;
        canvas.save();
        canvas.clipRect(SkRect::MakeXYWH(clip.x, clip.y, clip.width, clip.height));
        auto ring = [&](Rect outer, Rect inner, Color color) {
            box(canvas, {outer.x, outer.y, outer.width, std::max(0., inner.y - outer.y)}, color);
            box(canvas,
                {outer.x, inner.y + inner.height, outer.width,
                 std::max(0., outer.y + outer.height - inner.y - inner.height)},
                color);
            box(canvas, {outer.x, inner.y, std::max(0., inner.x - outer.x), inner.height}, color);
            box(canvas,
                {inner.x + inner.width, inner.y, std::max(0., outer.x + outer.width - inner.x - inner.width),
                 inner.height},
                color);
        };
        if (inspection_overlay == InspectionOverlay::selection) {
            box(canvas, intersect(target->bounds, target->clip), 0x154dd2ff);
            box(canvas, intersect(target->bounds, target->clip), theme.cyan, 0, 1.5);
        } else {
            const auto r = target->bounds;
            const auto m = target->margin;
            Rect outer{r.x - m.left, r.y - m.top, r.width + horizontal(m), r.height + vertical(m)};
            ring(outer, r, 0x70ffaa34);
            ring(r, inset(r, target->padding), 0x703bd6a0);
            box(canvas, target->content_bounds, 0x184dd2ff);
            box(canvas, r, theme.cyan, 0, 1.5);
            if (inspection_overlay == InspectionOverlay::flex &&
                (target->type == "FlexRow" || target->type == "FlexColumn")) {
                const bool row = target->type == "FlexRow";
                const auto content = target->content_bounds;
                canvas.save();
                canvas.clipRect(SkRect::MakeXYWH(content.x, content.y, content.width, content.height));
                // The layout engine records real line membership and resolved gaps;
                // diagnostics do not infer wrapping from overlapping child rectangles.
                for (const auto& flex_line : target->flex_lines) {
                    auto r = flex_line.bounds;
                    double cursor = row ? r.x : r.y;
                    auto shade = [&](double end) {
                        if (end > cursor)
                            box(canvas,
                                row ? Rect{cursor, r.y, end - cursor, r.height}
                                    : Rect{r.x, cursor, r.width, end - cursor},
                                0x704dd2ff);
                    };
                    for (const auto& id : flex_line.items) {
                        auto c = std::find_if(snapshot.begin(), snapshot.end(),
                                              [&](const auto& item) { return item.id == id; });
                        if (c == snapshot.end())
                            continue;
                        const auto b = c->bounds;
                        shade(row ? b.x - c->margin.left : b.y - c->margin.top);
                        cursor = row ? b.x + b.width + c->margin.right : b.y + b.height + c->margin.bottom;
                    }
                    shade(row ? r.x + r.width : r.y + r.height);
                    box(canvas, r, 0x808b9bb5, 0, 1);
                }
                std::size_t order{};
                for (const auto& child : snapshot) {
                    if (child.parent_id != target->id || child.position == Position::absolute)
                        continue;
                    const auto b = child.bounds;
                    const auto cm = child.margin;
                    Rect occupied{b.x - cm.left, b.y - cm.top, b.width + horizontal(cm),
                                  b.height + vertical(cm)};
                    ring(occupied, b, 0x70ffaa34);
                    box(canvas, b, 0xffb39dff, 0, 1.5);
                    const double x = b.x + 3, y = b.y + 3;
                    box(canvas, {x, y, 24, 16}, 0xff192b3d, 2);
                    auto* source_node = find(target->id);
                    if (source_node)
                        fonts.draw(canvas, *source_node, std::to_string(++order), x + 4, y + 12, 0xffe6f0ff);
                }
                // Main axis: magenta. Cross axis: cyan. Arrowheads show direction.
                auto arrow = [&](double x, double y, double xx, double yy, Color color) {
                    line(canvas, x, y, xx, yy, color, 2);
                    if (xx != x) {
                        line(canvas, xx - 5, yy - 4, xx, yy, color, 2);
                        line(canvas, xx - 5, yy + 4, xx, yy, color, 2);
                    } else {
                        line(canvas, xx - 4, yy - 5, xx, yy, color, 2);
                        line(canvas, xx + 4, yy - 5, xx, yy, color, 2);
                    }
                };
                if (content.width > 20 && content.height > 20) {
                    arrow(content.x + 6, content.y + 9, content.x + content.width - 6, content.y + 9,
                          row ? 0xffff80c0 : theme.cyan);
                    arrow(content.x + 9, content.y + 6, content.x + 9, content.y + content.height - 6,
                          row ? theme.cyan : 0xffff80c0);
                }
                canvas.restore();
            }
        }
        // Keep the flash on the selected item when flex debug outlines its ancestor.
        if (flash > 0 && id == inspected) {
            auto color = [&](double alpha) {
                return (static_cast<Color>(std::lround(alpha * flash)) << 24) | (theme.cyan & 0xffffff);
            };
            box(canvas, selected_bounds, color(36));
            box(canvas, selected_bounds, color(176), 0, 2.5);
        }
        canvas.restore();
    }

    std::vector<Element> elements() {
        layout(viewport);
        std::vector<Element> out;
        std::uint64_t next_order{};
        std::unordered_map<std::string, std::uint64_t> paint_order;
        std::function<void(Node&)> rank = [&](Node& n) {
            if (n.hidden)
                return;
            paint_order[n.id] = next_order++;
            for (auto* child : n.paint_children)
                rank(*child);
        };
        if (root)
            rank(*root);
        for (const auto& overlay : overlays)
            if (overlay.popover || overlay.kind == OverlayKind::combo)
                if (auto* body = find(overlay.root_id)) rank(*body);
        std::function<void(Node&, std::string)> visit = [&](Node& n, std::string parent) {
            if (n.hidden)
                return;
            bool semantic =
                n.d.alert || n.d.kind==Kind::table_header || n.d.kind==Kind::table_cell || n.d.kind==Kind::table_column || n.d.kind==Kind::table_resize || text_kind(n.d.kind) || n.d.kind == Kind::button || n.d.kind == Kind::toggle ||
                n.d.kind == Kind::checkbox || n.d.kind == Kind::radio_option ||
                n.d.kind == Kind::segment_option || n.d.kind == Kind::menu || n.d.kind == Kind::selection ||
                n.d.kind == Kind::split || n.d.kind == Kind::split_arrow || n.d.kind == Kind::slider ||
                n.d.kind == Kind::progress || n.d.kind == Kind::image || n.d.kind == Kind::listbox ||
                n.d.kind == Kind::split_view || n.d.kind == Kind::split_divider ||
                n.d.kind == Kind::tab_view || n.d.kind == Kind::tab_option ||
                n.d.kind == Kind::tab_page ||
                n.d.kind == Kind::stepper || n.d.kind == Kind::step_button || n.d.kind == Kind::switch_control ||
                n.d.kind == Kind::field_action || n.popover_root || n.d.tooltip.has_value() || n.d.context_menu ||
                n.d.kind == Kind::list_option || n.d.kind == Kind::text_field || n.d.kind == Kind::radio ||
                n.d.kind == Kind::segments || n.d.kind == Kind::section || n.d.kind == Kind::group;
            if (semantic) {
                auto e = element(n);
                if (e.group.empty())
                    e.group = parent;
                e.paint_order = paint_order.at(n.id);
                out.push_back(std::move(e));
            }
            if (n.d.alert || n.d.datetime || n.d.color || n.d.kind == Kind::tab_view || n.d.kind == Kind::tab_page || n.popover_root || n.d.combo || n.d.kind == Kind::listbox || n.d.kind == Kind::list_option || n.d.kind==Kind::table_header || n.d.kind==Kind::table_cell || n.d.kind==Kind::table_column)
                parent = n.id;
            for (auto& c : n.children)
                visit(*c, parent);
        };
        if (root)
            visit(*root, {});
        for (const auto& overlay : overlays)
            if (overlay.popover || overlay.kind == OverlayKind::combo)
                if (auto* body = find(overlay.root_id)) visit(*body, overlay.owner);
        if (menu_popup())
            for (std::size_t i = 0; i < menu_popup()->options.size(); ++i) {
                auto& o = menu_popup()->options[i];
                Element e;
                e.id = menu_popup()->owner + "/menu/" + path_key(o.id);
                e.label = o.label;
                e.group = menu_popup()->owner;
                e.role = Role::menu_item;
                e.paint_order = next_order++;
                e.bounds = popup_item_rect(i);
                e.clip = inset(menu_popup()->bounds, Insets::all(theme.popup_padding));
                e.enabled = o.enabled;
                e.focused = active && static_cast<int>(i) == menu_popup()->highlighted;
                auto* n = find(menu_popup()->owner);
                e.selected = n && n->selection == o.id;
                out.push_back(std::move(e));
            }
        for (const auto& menu : overlays) {
            if (!menu.context) continue;
            Element group;
            group.id = menu.root_id; group.label = "Context menu"; group.role = Role::menu;
            group.group = menu.context->parent_menu.empty() ? menu.owner : menu.context->parent_menu + "/item/" + path_key(menu.context->parent_item);
            group.bounds = menu.bounds; group.clip = {0, 0, viewport.width, viewport.height}; group.paint_order = next_order++;
            out.push_back(group);
            for (std::size_t i = 0; i < menu.context->entries.size(); ++i) {
                const auto& entry = menu.context->entries[i];
                if (entry.item.kind == MenuItemKind::separator) continue;
                Element item;
                item.id = context_item_id(menu, i); item.label = entry.item.label; item.group = menu.root_id;
                item.role = Role::menu_item; item.bounds = context_item_rect(menu, i);
                item.clip = inset(menu.bounds, Insets::all(theme.popup_padding)); item.paint_order = next_order++;
                item.enabled = menu_selectable(entry); item.selected = entry.checked;
                if (entry.item.kind == MenuItemKind::check || entry.item.kind == MenuItemKind::radio)
                    item.value_text = entry.checked ? "Checked" : "Unchecked";
                item.focused = active && context_popup() == &menu && menu.highlighted == static_cast<int>(i);
                item.shortcut = entry.item.shortcut; item.help = entry.item.shortcut;
                item.has_submenu = entry.item.kind == MenuItemKind::submenu;
                item.expanded = std::any_of(overlays.begin(), overlays.end(), [&](const auto& o) { return o.context && o.context->parent_menu == menu.root_id && o.context->parent_item == entry.item.id; });
                out.push_back(std::move(item));
            }
        }
        return out;
    }
};
Host::Host(std::function<View()> builder, std::function<void()> wake, Theme theme,
           std::shared_ptr<TextLayoutEngine> text, HostOptions options) {
    if (!builder)
        throw std::invalid_argument("SCARS Host requires a builder");
    if (!options.clock)
        throw std::invalid_argument("SCARS Host requires a clock");
    impl_ = std::make_shared<Impl>(std::move(builder), std::move(wake), std::move(theme), std::move(text), std::move(options));
    std::weak_ptr<Impl> weak = impl_;
    impl_->observer->changed = [weak] {
        if (auto self = weak.lock())
            self->invalidate();
    };
}
Host::~Host() { impl_->release_colors(); }
void Host::invalidate() {
    impl_->invalidate();
}
void Host::set_theme(Theme theme) {
    impl_->check();
    if (!impl_->custom_text && theme.font_path != impl_->theme.font_path)
        impl_->fonts = FontEngine(theme);
    impl_->theme = std::move(theme);
    impl_->invalidate();
}
Theme Host::theme() const {
    impl_->check();
    return impl_->theme;
}
void Host::set_clipboard(Clipboard clipboard) {
    impl_->check();
    impl_->clipboard = std::move(clipboard);
}
std::optional<TextInputSnapshot> Host::text_input() {
    impl_->layout(impl_->viewport);
    auto* n = impl_->field();
    if (!n)
        return {};
    auto& buffer = impl_->editors.at(n->id)->buffer;
    return TextInputSnapshot{buffer.text(),
                             buffer.selection(),
                             buffer.marked_range(),
                             text_range_rect({buffer.selection().caret, 0}),
                             impl_->field_viewport(*n),
                             !n->d.text_options.read_only,
                             n->d.text_options.multiline,
                             (!n->d.password || n->d.password->allow_undo) && buffer.can_undo(),
                             (!n->d.password || n->d.password->allow_undo) && buffer.can_redo(),
                             n->d.password.has_value(), impl_->active, n->d.combo.has_value(), n->d.date.has_value()};
}
bool Host::insert_text(std::string text, std::optional<TextRange> range) {
    return impl_->insert_text(std::move(text), range);
}
bool Host::mark_text(std::string text, TextRange selected, std::optional<TextRange> replacement) {
    impl_->layout(impl_->viewport);
    auto* n = impl_->field();
    if (!n || n->d.text_options.read_only)
        return false;
    utf16_from_utf8(text);
    impl_->editors.at(n->id)->buffer.mark(std::move(text), selected, replacement);
    impl_->field_changed(*n, false);
    return true;
}
void Host::unmark_text(bool commit) {
    impl_->check();
    impl_->unmark_text(commit);
}
bool Host::text_command(TextCommand command, bool extend) {
    return impl_->text_command(command, extend);
}
Rect Host::text_range_rect(TextRange range) {
    impl_->layout(impl_->viewport);
    auto* n = impl_->field();
    if (!n || !n->text)
        return {};
    Rect rect;
    if (range.length && n->text->selection_rects) {
        auto list = n->text->selection_rects(range);
        if (!list.empty())
            rect = list.front();
    } else if (n->text->caret_rect)
        rect = range.start == impl_->editors.at(n->id)->buffer.selection().caret
                   ? impl_->field_caret(*n)
                   : n->text->caret_rect(range.start);
    rect.x += n->text_origin.x;
    rect.y += n->text_origin.y;
    return rect;
}
std::size_t Host::text_index_at(Point point) {
    impl_->layout(impl_->viewport);
    auto* n = impl_->field();
    return n && n->text && n->text->hit_test ? impl_->editors.at(n->id)->buffer.normalize(n->text->hit_test(
                                                   {point.x - n->text_origin.x, point.y - n->text_origin.y}))
                                             : 0;
}
bool Host::set_text(const std::string& id, std::string text) {
    impl_->layout(impl_->viewport);
    auto* n = impl_->find(id);
    if (!n || n->d.kind != Kind::text_field || !interactive(*n) || !impl_->in_focus_scope(*n) || n->d.text_options.read_only)
        return false;
    utf16_from_utf8(text);
    auto& editor = *impl_->editors.at(id);
    editor.buffer.select({0, editor.buffer.text().size()});
    text = impl_->field_insert(*n, std::move(text), {});
    editor.buffer.replace(std::move(text));
    impl_->field_changed(*n, true);
    return true;
}
bool Host::select_text(const std::string& id, TextSelection selection) {
    impl_->layout(impl_->viewport);
    auto* n = impl_->find(id);
    if (!n || n->d.kind != Kind::text_field || !interactive(*n))
        return false;
    if (!impl_->focus(id, false))
        return false;
    impl_->editors.at(id)->buffer.select(selection);
    impl_->field_changed(*n, false);
    return true;
}
void Host::layout(Size size) {
    impl_->layout(size);
}
bool Host::scroll_to(const std::string& id, Point offset) { return impl_->scroll_to(id, offset); }
std::optional<Point> Host::scroll_offset(const std::string& id) {
    impl_->layout(impl_->viewport);
    const auto* node = impl_->find(id);
    if (!node || (node->d.kind != Kind::scroll && node->d.kind != Kind::listbox)) return {};
    const auto found = impl_->scroll.find(id);
    return found == impl_->scroll.end() ? Point{} : found->second;
}
void Host::paint(SkCanvas& canvas, double seconds) {
    impl_->paint(canvas, seconds);
}
bool Host::handle(PointerEvent e) {
    return impl_->pointer(e);
}
bool Host::handle(KeyEvent e) {
    return impl_->key(e);
}
Cursor Host::cursor(Point point) {
    impl_->layout(impl_->viewport);
    if (impl_->menu_popup() || impl_->context_popup() || impl_->inspection_picking)
        return Cursor::arrow;
    auto* node = impl_->find(impl_->dragging);
    if (!node)
        node = impl_->hit(point);
    if(node&&node->d.kind==Kind::table_resize)return Cursor::resize_horizontal;
    if (node && node->d.kind == Kind::split_divider)
        return node->d.orientation == Orientation::horizontal ? Cursor::resize_horizontal : Cursor::resize_vertical;
    return node && node->d.kind == Kind::text_field ? Cursor::text : Cursor::arrow;
}
std::optional<DateTimeValidation> Host::field_validation(const std::string& id) {
    impl_->layout(impl_->viewport);
    const auto* node=impl_->find(id);
    if(node&&(node->d.datetime||node->datetime_component))
        return impl_->datetime_validation(node->d.datetime?id:node->datetime_owner);
    return numeric_validation(id);
}
bool Host::commit_field(const std::string& id) {
    if(detail::building)throw std::logic_error("Cannot commit fields while building views");
    impl_->layout(impl_->viewport);
    const auto* node=impl_->find(id);
    if(node&&(node->d.datetime||node->datetime_component))
        return impl_->commit_datetime(node->d.datetime?id:node->datetime_owner);
    return impl_->commit_numeric(id);
}
bool Host::cancel_field(const std::string& id) {
    if(detail::building)throw std::logic_error("Cannot cancel fields while building views");
    impl_->layout(impl_->viewport);
    auto* node=impl_->find(id);
    if(!node)return false;
    if(node->d.datetime||node->datetime_component)
        return impl_->cancel_datetime(node->d.datetime?id:node->datetime_owner);
    if(node->d.kind==Kind::text_field&&node->d.numeric)return impl_->cancel_numeric(*node);
    return false;
}
std::optional<NumericValidation> Host::numeric_validation(const std::string& id) {
    impl_->layout(impl_->viewport);
    const auto* node = impl_->find(id);
    if (!node || !node->d.numeric)
        return {};
    if (node->d.kind == Kind::text_field)
        return impl_->editors.at(id)->validation;
    if (*node->numeric_value < node->d.numeric->min || *node->numeric_value > node->d.numeric->max)
        return NumericValidation{NumericStatus::out_of_range, "Value is outside the configured range.", false};
    return NumericValidation{};
}
std::optional<NumericValidation> Host::color_validation(const std::string& id) {
    impl_->check();
    impl_->layout(impl_->viewport);
    const auto* node = impl_->find(id);
    if (!node || node->color_owner.empty())
        return {};
    return impl_->color_status(node->color_owner);
}
bool Host::apply_color(const std::string& id) {
    impl_->check();
    if (detail::building)
        throw std::logic_error("SCARS color cannot be applied while building a view");
    impl_->layout(impl_->viewport);
    const auto* node = impl_->find(id);
    if (!node || node->color_owner.empty() ||
        (!impl_->in_focus_scope(*node) &&
         (!impl_->focus_scope() || !impl_->focus_scope()->root_id.starts_with(node->color_owner + "/"))))
        return false;
    return impl_->color_apply(node->color_owner);
}
bool Host::cancel_color(const std::string& id) {
    impl_->check();
    if (detail::building)
        throw std::logic_error("SCARS color cannot be cancelled while building a view");
    impl_->layout(impl_->viewport);
    const auto* node = impl_->find(id);
    return node && !node->color_owner.empty() && impl_->color_cancel(node->color_owner);
}
bool Host::commit_numeric(const std::string& id) {
    return impl_->commit_numeric(id);
}
bool Host::focus_search(const std::string& id) {
    return impl_->focus_search(id);
}
bool Host::flush_search(const std::string& id) {
    return impl_->flush_search(id);
}
void Host::advance_time() {
    impl_->advance_time();
}
std::optional<std::chrono::milliseconds> Host::next_wake() {
    return impl_->next_wake();
}
void Host::blur() {
    impl_->check();
    impl_->finish_editing();
    impl_->hold.reset();
    impl_->active = false;
    std::vector<std::string> color_ids;
    for(const auto& [id,state]:impl_->colors)color_ids.push_back(id);
    for(const auto& id:color_ids)impl_->color_cancel(id);
    std::vector<std::string> datetime_ids;
    for(const auto& [id,state]:impl_->datetimes)datetime_ids.push_back(id);
    for(const auto& id:datetime_ids)impl_->cancel_datetime(id);
    for (auto& [id, editor] : impl_->editors)
        editor->pending_search.reset();
    impl_->text_drag.clear();
    impl_->pressed.clear();
    impl_->dragging.clear();
    impl_->column_drag.reset();
    impl_->drop_menu();
    impl_->hide_tooltip();
    std::vector<Binding<bool>> presentations;
    for (const auto& overlay : impl_->overlays)
        if (overlay.popover) presentations.push_back(overlay.popover->presented);
    for (const auto& presentation : presentations)
        if (presentation.get()) presentation.set(false);
    impl_->invalidate();
    impl_->layout(impl_->viewport);
    impl_->notify();
}
void Host::set_active(bool active) {
    impl_->check();
    if (!active) {
        blur();
        return;
    }
    if (!impl_->active) {
        impl_->active = true;
        impl_->notify();
    }
}
bool Host::needs_frame() const {
    impl_->check();
    return impl_->dirty || impl_->geometry_dirty || impl_->animating || impl_->inspection_flash_pending ||
           impl_->inspection_flash_start.has_value() ||
           (impl_->active && impl_->field() && !impl_->field()->d.text_options.read_only &&
            !impl_->editors.at(impl_->field()->id)->buffer.selection().range().length);
}
std::vector<Element> Host::elements() {
    return impl_->elements();
}
std::optional<LogicalItem> Host::list_item(const std::string& id, std::size_t index) {
    impl_->layout(impl_->viewport);
    auto* list = impl_->find(id);
    if (!list || list->hidden || list->d.kind != Kind::listbox) return {};
    return impl_->logical_item(*list, index);
}
std::optional<LogicalItem> Host::list_item(const std::string& id) {
    impl_->layout(impl_->viewport);
    const auto row = impl_->logical_row(id);
    if (!row || row->first->hidden) return {};
    return impl_->logical_item(*row->first, row->second);
}
bool Host::reveal_item(const std::string& id, std::size_t index) {
    impl_->layout(impl_->viewport);
    return impl_->reveal_item(id, index);
}
bool Host::set_expanded(const std::string& id, bool expanded) {
    impl_->layout(impl_->viewport);
    const auto row = impl_->logical_row(id);
    if (!row) return false;
    const auto key = row->first->d.rows.at(row->second).id;
    return impl_->tree_expand(row->first->id, key, expanded);
}
std::optional<LogicalItem> Host::tree_child(const std::string& id, std::size_t index) {
    impl_->layout(impl_->viewport);
    const auto row = impl_->logical_row(id);
    if (!row || !row->first->d.tree || row->first->hidden) return {};
    const auto& state = impl_->trees.at(row->first->id);
    const auto& key = state.flat.at(row->second).id;
    const auto children = state.children.find(key);
    if (children == state.children.end() || index >= children->second.size()) return {};
    return impl_->logical_item(*row->first, children->second[index]);
}
std::optional<LogicalCell> Host::table_cell(const std::string& id,const TableCell& cell) {
    impl_->layout(impl_->viewport);return impl_->logical_cell(id,cell);
}
std::optional<LogicalCell> Host::table_cell(const std::string& id) {
    impl_->layout(impl_->viewport);const auto cell=impl_->logical_cell_id(id);
    return cell?impl_->logical_cell(cell->first,cell->second):std::nullopt;
}
std::optional<LogicalCell> Host::table_cell_at(const std::string& id,std::size_t column) {
    impl_->layout(impl_->viewport);const auto row=impl_->logical_row(id);
    if(!row||!row->first->d.table||column>=row->first->d.table->model.columns().size())return {};
    return impl_->logical_cell(row->first->id,{row->first->d.rows.at(row->second).id,row->first->d.table->model.columns()[column].id});
}
std::optional<LogicalColumn> Host::table_column(const std::string& id,std::size_t column) {
    impl_->layout(impl_->viewport);return impl_->logical_column(id,column);
}
std::optional<LogicalColumn> Host::table_column(const std::string& id) {
    impl_->layout(impl_->viewport);return impl_->logical_column_id(id);
}
std::optional<LogicalCell> Host::table_column_cell(const std::string& id,std::size_t row) {
    impl_->layout(impl_->viewport);const auto column=impl_->logical_column_id(id);
    if(!column)return {};const auto* table=impl_->find(column->table);
    if(row>=table->d.rows.size())return {};
    return impl_->logical_cell(column->table,{table->d.rows.at(row).id,column->key});
}
bool Host::set_selection(const std::string& id,std::vector<std::string> items) {
    impl_->layout(impl_->viewport);return impl_->set_selection(id,items);
}
bool Host::set_selected(const std::string& id,bool value) {
    impl_->layout(impl_->viewport);return impl_->set_selected(id,value);
}
bool Host::edit_cell(const std::string& id) {
    impl_->layout(impl_->viewport);const auto cell=impl_->logical_cell_id(id);
    return cell&&impl_->table_edit(cell->first,cell->second);
}
bool Host::set_cell_text(const std::string& id,std::string text) {
    impl_->layout(impl_->viewport);utf16_from_utf8(text);
    const auto cell=impl_->logical_cell_id(id);if(!cell)return false;
    if(impl_->tables.at(cell->first).edit&&impl_->tables.at(cell->first).edit->cell==cell->second)
        if(auto* field=impl_->field();field&&impl_->editors.at(field->id)->buffer.marked_range())return false;
    if(!impl_->table_edit(cell->first,cell->second))return false;
    const auto editor=impl_->table_cell_id(cell->first,cell->second)+"/key/editor";
    if(!set_text(editor,std::move(text)))return false;
    return impl_->table_commit(cell->first);
}
bool Host::reveal_cell(const std::string& id,const TableCell& cell) {
    impl_->layout(impl_->viewport);return impl_->table_reveal(id,cell);
}
bool Host::edit_cell(const std::string& id,const TableCell& cell) {
    impl_->layout(impl_->viewport);return impl_->table_edit(id,cell);
}
bool Host::commit_table_edit(const std::string& id) {
    impl_->layout(impl_->viewport);return impl_->table_commit(id);
}
bool Host::cancel_table_edit(const std::string& id) {
    impl_->layout(impl_->viewport);const bool result=impl_->table_cancel(id);impl_->layout(impl_->viewport);impl_->notify();return result;
}
bool Host::focus(const std::string& id) {
    impl_->layout(impl_->viewport);
    if (impl_->context_popup()) {
        for (auto it = impl_->overlays.begin(); it != impl_->overlays.end(); ++it)
            if (it->context)
                for (std::size_t i = 0; i < it->context->entries.size(); ++i)
                    if (Impl::context_item_id(*it, i) == id && Impl::menu_selectable(it->context->entries[i])) {
                        it->highlighted = static_cast<int>(i);
                        impl_->overlays.erase(it + 1, impl_->overlays.end());
                        impl_->position_contexts(true); impl_->notify();
                        return true;
                    }
        return false;
    }
    if (impl_->menu_popup())
        for (std::size_t i = 0; i < impl_->menu_popup()->options.size(); ++i)
            if (id == impl_->menu_popup()->owner + "/menu/" + path_key(impl_->menu_popup()->options[i].id)) {
                if (!impl_->menu_popup()->options[i].enabled)
                    return false;
                impl_->menu_popup()->highlighted = static_cast<int>(i);
                impl_->position_popup(true);
                impl_->notify();
                return true;
            }
    return impl_->focus(id);
}
bool Host::reveal(const std::string& id) {
    impl_->layout(impl_->viewport);
    if (auto logical = impl_->logical_row(id)) return impl_->reveal_item(logical->first->id, logical->second);
    if(auto cell=impl_->logical_cell_id(id))return impl_->table_reveal(cell->first,cell->second);
    auto* node = impl_->find(id);
    if (!node)
        return false;
    impl_->reveal(*node);
    impl_->notify();
    return true;
}
bool Host::activate(const std::string& id) {
    impl_->layout(impl_->viewport);
    if (impl_->context_popup()) {
        for (const auto& menu : impl_->overlays)
            if (menu.context)
                for (std::size_t i = 0; i < menu.context->entries.size(); ++i)
                    if (Impl::context_item_id(menu, i) == id) return impl_->choose_context(menu.root_id, static_cast<int>(i));
        return false;
    }
    if (impl_->menu_popup()) {
        for (std::size_t i = 0; i < impl_->menu_popup()->options.size(); ++i)
            if (id == impl_->menu_popup()->owner + "/menu/" + path_key(impl_->menu_popup()->options[i].id)) {
                if (!impl_->menu_popup()->options[i].enabled)
                    return false;
                impl_->choose(static_cast<int>(i));
                return true;
            }
    }
    return impl_->activate(id);
}
bool Host::adjust(const std::string& id, double steps) {
    return impl_->adjust(id, steps);
}
bool Host::set_value(const std::string& id, double value) {
    return impl_->set_value(id, value);
}
bool Host::popup_open() const {
    impl_->check();
    return impl_->menu_popup() != nullptr || impl_->context_popup() != nullptr || impl_->combo_popup() != nullptr;
}
Rect Host::popup_bounds() const {
    impl_->check();
    if (const auto* context = impl_->context_popup()) return context->bounds;
    if (const auto* combo = impl_->combo_popup()) return combo->bounds;
    return impl_->menu_popup() ? impl_->menu_popup()->bounds : Rect{};
}
bool Host::show_context_menu(const std::string& id, std::optional<Point> origin) {
    return impl_->open_context(id, origin);
}
std::uint64_t Host::generation() const {
    impl_->check();
    return impl_->generation;
}
std::vector<InspectionNode> Host::inspect() {
    impl_->check();
    return impl_->inspect();
}
void Host::set_inspected_element(std::string id) {
    impl_->check();
    if (!id.empty()) {
        auto snapshot = impl_->inspect();
        if (std::none_of(snapshot.begin(), snapshot.end(), [&](const auto& n) { return n.id == id; }))
            throw std::invalid_argument("Unknown inspected element: " + id);
    }
    if (!id.empty() || impl_->inspected != id)
        impl_->select_inspected(std::move(id));
}
std::string Host::inspected_element() const {
    impl_->check();
    return impl_->inspected;
}
void Host::set_inspection_picking(bool value) {
    impl_->check();
    if (impl_->inspection_picking == value)
        return;
    impl_->inspection_picking = value;
    impl_->inspection_hovered.clear();
    impl_->pressed.clear();
    impl_->dragging.clear();
    impl_->notify();
}
bool Host::inspection_picking() const {
    impl_->check();
    return impl_->inspection_picking;
}
void Host::set_inspection_overlay(InspectionOverlay mode) {
    impl_->check();
    if (impl_->inspection_overlay != mode) {
        impl_->inspection_overlay = mode;
        impl_->notify();
    }
}
InspectionOverlay Host::inspection_overlay() const {
    impl_->check();
    return impl_->inspection_overlay;
}
void Host::set_layout_override(const std::string& id, LayoutOverride value) {
    impl_->check();
    if (detail::building)
        throw std::logic_error("Cannot edit layout while building views");
    for (const auto& n : {value.width, value.height, value.basis})
        if (n)
            detail::check_length(*n);
    for (const auto& n : {value.left, value.top, value.right, value.bottom})
        if (n)
            detail::check_offset(*n);
    for (const auto& n : {value.padding, value.margin})
        if (n)
            detail::check_insets(*n);
    for (const auto& n : {value.gap, value.grow, value.shrink})
        if (n)
            detail::nonnegative(*n, "Layout override");
    impl_->layout(impl_->viewport);
    auto* node = impl_->find(id);
    if (!node || !node->layout_editable)
        throw std::invalid_argument("Element does not support layout overrides: " + id);
    if (value.direction && node->d.kind != Kind::row && node->d.kind != Kind::column)
        throw std::invalid_argument("Only flex containers support direction overrides");
    if (!value.width && !value.height && !value.basis && !value.padding && !value.margin && !value.gap &&
        !value.grow && !value.shrink && !value.align && !value.self && !value.justify && !value.wrap &&
        !value.direction && !value.position && !value.left && !value.top && !value.right && !value.bottom &&
        !value.z_index && !value.hidden) {
        clear_layout_override(id);
        return;
    }
    impl_->layout_overrides[id] = std::move(value);
    impl_->invalidate();
}
std::optional<LayoutOverride> Host::layout_override(const std::string& id) const {
    impl_->check();
    auto i = impl_->layout_overrides.find(id);
    return i == impl_->layout_overrides.end() ? std::nullopt : std::optional(i->second);
}
void Host::clear_layout_override(const std::string& id) {
    impl_->check();
    if (detail::building)
        throw std::logic_error("Cannot edit layout while building views");
    if (impl_->layout_overrides.erase(id))
        impl_->invalidate();
}
void Host::clear_layout_overrides() {
    impl_->check();
    if (detail::building)
        throw std::logic_error("Cannot edit layout while building views");
    if (!impl_->layout_overrides.empty()) {
        impl_->layout_overrides.clear();
        impl_->invalidate();
    }
}
void Host::set_inspection_observer(std::function<void()> callback) {
    impl_->check();
    impl_->inspection_observer = std::move(callback);
}
} // namespace scars
