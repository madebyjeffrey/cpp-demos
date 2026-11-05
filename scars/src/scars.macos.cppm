module;
#include <memory>
#include <expected>
#include <optional>
#include <string>
#include <unordered_map>
#include <functional>
#include <utility>
#include <variant>
#include <stdexcept>
#include <vector>
#include <algorithm>
#include <skia_window/skia_window.hpp>
export module scars.macos;
import scars;

export namespace scars::macos {
struct MountOptions {
    std::function<void()> on_closed;
    std::function<bool(const skia_window::Event&)> on_event;
};
class AlertPresentation {
  public:
    static std::expected<std::unique_ptr<AlertPresentation>, skia_window::Error>
    present(skia_window::Window& parent, std::string title, View content,
            std::vector<AlertAction>, std::function<void(std::optional<std::string>)> completion = {},
            Theme = {}, skia_window::WindowOptions = {.size = {480, 320}, .minimum_size = {320, 200}});
    static std::expected<std::unique_ptr<AlertPresentation>, skia_window::Error>
    present(skia_window::Window& parent, std::string title, std::function<View()> content,
            std::vector<AlertAction>, std::function<void(std::optional<std::string>)> completion = {},
            Theme = {}, skia_window::WindowOptions = {.size = {480, 320}, .minimum_size = {320, 200}});
    ~AlertPresentation();
    AlertPresentation(const AlertPresentation&) = delete;
    AlertPresentation& operator=(const AlertPresentation&) = delete;
    bool choose(const std::string& action_id);
    void dismiss();
    bool is_open() const;
    std::shared_ptr<Host> host() const;

  private:
    struct Impl;
    explicit AlertPresentation(std::shared_ptr<Impl>);
    std::shared_ptr<Impl> impl_;
};
struct ConfirmationOptions {
    std::string accept_label{"Continue"}, cancel_label{"Cancel"};
    bool destructive{};
};
std::expected<std::unique_ptr<AlertPresentation>, skia_window::Error>
confirm(skia_window::Window& parent, std::string title, std::string message,
        std::function<void(bool)> completion, Theme = {}, ConfirmationOptions = {});
class DeveloperTools {
  public:
    DeveloperTools(skia_window::Application&, std::shared_ptr<Host> source, Theme = {});
    ~DeveloperTools();
    DeveloperTools(const DeveloperTools&) = delete;
    DeveloperTools& operator=(const DeveloperTools&) = delete;
    void show();
    void toggle();
    void close();
    bool is_open() const;

  private:
    struct Impl;
    std::shared_ptr<Impl> impl_;
};
// The surface owns callbacks retaining the host. Unmounting or closing cancels its wake
// connection, so retained hosts cannot call a destroyed surface. No legacy UI is mounted.
std::shared_ptr<Host> mount(skia_window::Surface& window, std::function<View()> builder, Theme theme = {},
                            std::shared_ptr<TextLayoutEngine> text_engine = {}, MountOptions options = {}) {
    namespace sw = skia_window;
    auto live = std::make_shared<sw::Surface*>(&window);
    std::weak_ptr<sw::Surface*> weak_window = live;
    auto host = std::make_shared<Host>(
        std::move(builder),
        [weak_window] {
            if (auto live = weak_window.lock(); live && *live) {
                (*live)->request_redraw();
            }
        },
        std::move(theme), std::move(text_engine));
    window.clear_content();
    window.set_cursor(sw::Cursor::arrow);
    window.set_callbacks({.on_draw =
                              [host, live, &window](SkCanvas& canvas, const sw::FrameInfo& frame) {
                                  host->layout({frame.logical_size.width, frame.logical_size.height});
                                  host->paint(canvas, frame.elapsed.count());
                                  window.set_redraw_delay(host->next_wake());
                                  const bool animate = host->needs_frame();
                                  if (window.is_animating() != animate)
                                      window.set_animating(animate);
                              },
                          .on_event =
                              [host, &window, intercept = std::move(options.on_event)](const sw::Event& event) {
                                  if (intercept && intercept(event))
                                      return;
                                  if (auto p = std::get_if<sw::PointerEvent>(&event)) {
                                      PointerKind kind;
                                      switch (p->kind) {
                                      case sw::PointerKind::down:
                                          kind = PointerKind::down;
                                          break;
                                      case sw::PointerKind::up:
                                          kind = PointerKind::up;
                                          break;
                                      case sw::PointerKind::scroll:
                                          kind = PointerKind::scroll;
                                          break;
                                      case sw::PointerKind::leave:
                                          kind = PointerKind::leave;
                                          break;
                                      default:
                                          kind = PointerKind::move;
                                          break;
                                      }
                                      host->handle(PointerEvent{kind,
                                                                {p->position.x, p->position.y},
                                                                p->button,
                                                                p->scroll_x,
                                                                p->scroll_y,
                                                                p->modifiers.shift,
                                                                p->click_count,
                                                                p->modifiers.command,
                                                                p->modifiers.control});
                                      auto cursor = sw::Cursor::arrow;
                                      switch (host->cursor({p->position.x, p->position.y})) {
                                      case Cursor::text: cursor = sw::Cursor::text; break;
                                      case Cursor::resize_horizontal: cursor = sw::Cursor::resize_horizontal; break;
                                      case Cursor::resize_vertical: cursor = sw::Cursor::resize_vertical; break;
                                      default: break;
                                      }
                                      window.set_cursor(cursor);
                                  } else if (auto k = std::get_if<sw::KeyEvent>(&event)) {
                                      Key key = Key::unknown;
                                      switch (k->key) {
                                      case sw::Key::f10:
                                          key = Key::f10;
                                          break;
                                      case sw::Key::f:
                                          key = Key::f;
                                          break;
                                      case sw::Key::backspace:
                                          key = Key::backspace;
                                          break;
                                      case sw::Key::delete_forward:
                                          key = Key::delete_forward;
                                          break;
                                      case sw::Key::a:
                                          key = Key::a;
                                          break;
                                      case sw::Key::c:
                                          key = Key::c;
                                          break;
                                      case sw::Key::x:
                                          key = Key::x;
                                          break;
                                      case sw::Key::v:
                                          key = Key::v;
                                          break;
                                      case sw::Key::z:
                                          key = Key::z;
                                          break;
                                      case sw::Key::y:
                                          key = Key::y;
                                          break;
                                      case sw::Key::tab:
                                          key = Key::tab;
                                          break;
                                      case sw::Key::space:
                                          key = Key::space;
                                          break;
                                      case sw::Key::enter:
                                          key = Key::enter;
                                          break;
                                      case sw::Key::escape:
                                          key = Key::escape;
                                          break;
                                      case sw::Key::left:
                                          key = Key::left;
                                          break;
                                      case sw::Key::right:
                                          key = Key::right;
                                          break;
                                      case sw::Key::up:
                                          key = Key::up;
                                          break;
                                      case sw::Key::down:
                                          key = Key::down;
                                          break;
                                      case sw::Key::home:
                                          key = Key::home;
                                          break;
                                      case sw::Key::end:
                                          key = Key::end;
                                          break;
                                      case sw::Key::f2:
                                          key = Key::f2;
                                          break;
                                      case sw::Key::page_up:
                                          key = Key::page_up;
                                          break;
                                      case sw::Key::page_down:
                                          key = Key::page_down;
                                          break;
                                      default:
                                          break;
                                      }
                                      host->handle(KeyEvent{key, k->down, k->repeat, k->modifiers.shift,
                                                            k->modifiers.command, k->modifiers.alt,
                                                            k->modifiers.control, k->text});
                                  } else if (auto f = std::get_if<sw::FocusEvent>(&event)) {
                                      host->set_active(f->focused);
                                      if (!f->focused)
                                          window.set_cursor(sw::Cursor::arrow);
                                  }
                              },
                          .on_close_request = {},
                          .on_closed =
                              [host, live, closed = std::move(options.on_closed)] {
                                  *live = nullptr;
                                  host->blur();
                                  if (closed)
                                      closed();
                              }});
    auto provider = std::make_shared<sw::AccessibilityProvider>();
    std::weak_ptr<Host> weak = host;
    provider->prepare = [weak](sw::Size size) {if(auto h=weak.lock())h->layout({size.width,size.height});};
    provider->generation = [weak] {
        if (auto h = weak.lock())
            return h->generation();
        return std::uint64_t{0};
    };
    provider->elements = [weak](sw::Size size) {
        std::vector<sw::AccessibilityElement> out;
        if (auto host = weak.lock()) {
            host->layout({size.width, size.height});
            for (const auto& e : host->elements()) {
                sw::AccessibilityElement n;
                n.id = e.id;
                n.label = e.label;
                n.group = e.group;
                n.value_text = e.value_text;
                n.origin = {e.bounds.x, e.bounds.y};
                n.size = {e.bounds.width, e.bounds.height};
                n.clip_origin = {e.clip.x, e.clip.y};
                n.clip_size = {e.clip.width, e.clip.height};
                n.enabled = e.enabled;
                n.focused = e.focused;
                n.selected = e.selected;
                n.mixed = e.mixed;
                n.expanded = e.expanded;
                n.value = e.value;
                n.paint_order = e.paint_order;
                n.minimum = e.min;
                n.maximum = e.max;
                n.help = e.help;
                n.default_action = e.alert_action == AlertActionRole::default_action;
                n.cancel_action = e.alert_action == AlertActionRole::cancel;
                n.destructive_action = e.alert_action == AlertActionRole::destructive;
                n.secure = e.secure;
                n.search = e.search;
                n.item_count = e.item_count;
                n.item_index = e.item_index;
                n.selected_items = e.selected_items;
                n.column_index=e.column_index;n.column_count=e.column_count;n.selected_cells=e.selected_cells;n.table_id=e.table_id;
                n.sortable=e.sortable;n.sort_priority=e.sort_priority;
                if(e.sort_priority)n.help+=(n.help.empty()?"":" ")+std::string("Sort priority ")+std::to_string(*e.sort_priority+1);
                if(e.sort_order)n.sort_ascending=*e.sort_order==SortOrder::ascending;
                if(e.role==Role::table)n.header_id=e.id+"/header";
                if (e.tree) {
                    n.tree_level = e.tree->depth; n.expandable = e.tree->expandable; n.loading = e.tree->loading;
                    n.disclosed_count = e.tree->expanded ? e.tree->child_count : 0;
                    if (!e.tree->parent.empty()) {
                        auto parent = e.tree->parent;
                        std::string encoded;
                        for (char c : parent) { if (c == '/') encoded += "%2F"; else if (c == '%') encoded += "%25"; else encoded += c; }
                        n.tree_parent = e.group + "/option/" + encoded;
                    }
                }
                n.vertical = e.role == Role::splitter ? e.orientation == Orientation::horizontal
                                                     : e.orientation == Orientation::vertical;
                switch (e.role) {
                case Role::alert:
                    n.role = sw::AccessibilityRole::alert;
                    break;
                case Role::list_box:
                    n.role = sw::AccessibilityRole::list_box;
                    break;
                case Role::table:
                    n.role = sw::AccessibilityRole::table;
                    break;
                case Role::table_row:
                    n.role = sw::AccessibilityRole::list_item;
                    break;
                case Role::column_header:
                    n.role = sw::AccessibilityRole::column_header;
                    break;
                case Role::table_cell:
                    n.role = sw::AccessibilityRole::cell;
                    break;
                case Role::tree_view:
                    n.role = sw::AccessibilityRole::tree_view;
                    break;
                case Role::tree_item:
                case Role::list_item:
                    n.role = sw::AccessibilityRole::list_item;
                    break;
                case Role::image:
                    n.role = sw::AccessibilityRole::image;
                    break;
                case Role::text_field: {
                    n.role = e.combo ? sw::AccessibilityRole::combo_box : sw::AccessibilityRole::text_field;
                    const auto range = e.text_selection.range();
                    n.text_selection_start = utf16_offset(e.value_text, range.start);
                    n.text_selection_length =
                        utf16_offset(e.value_text, range.end()) - n.text_selection_start;
                    n.read_only = e.read_only;
                    n.multiline = e.multiline;
                    break;
                }
                case Role::text:
                    n.role = sw::AccessibilityRole::text;
                    break;
                case Role::toggle:
                case Role::checkbox:
                    n.role = sw::AccessibilityRole::checkbox;
                    break;
                case Role::radio:
                case Role::segment:
                    n.role = sw::AccessibilityRole::radio;
                    break;
                case Role::menu_button:
                    n.role = sw::AccessibilityRole::menu_button;
                    break;
                case Role::selection:
                    n.role = sw::AccessibilityRole::selection;
                    break;
                case Role::slider:
                    n.role = sw::AccessibilityRole::slider;
                    break;
                case Role::splitter:
                    n.role = sw::AccessibilityRole::splitter;
                    break;
                case Role::tab:
                    n.role = sw::AccessibilityRole::tab;
                    break;
                case Role::tab_group:
                    n.role = sw::AccessibilityRole::tab_group;
                    break;
                case Role::stepper:
                    n.role = sw::AccessibilityRole::stepper;
                    break;
                case Role::switch_control:
                    n.role = sw::AccessibilityRole::switch_control;
                    break;
                case Role::progress:
                    n.role = sw::AccessibilityRole::progress;
                    break;
                case Role::group:
                    n.role = sw::AccessibilityRole::group;
                    break;
                case Role::menu_item:
                    n.role = sw::AccessibilityRole::menu_item;
                    break;
                case Role::menu:
                    n.role = sw::AccessibilityRole::menu;
                    break;
                default:
                    n.role = sw::AccessibilityRole::button;
                    break;
                }
                out.push_back(std::move(n));
            }
        }
        return out;
    };
    const auto logical = [](const LogicalItem& item) {
        sw::AccessibilityElement n;
        n.id = item.id; n.label = item.label; n.group = item.group;
        n.role = sw::AccessibilityRole::list_item;
        n.column_count=item.column_count;if(item.column_count)n.table_id=item.group;
        n.item_index = item.index; n.enabled = item.enabled; n.selected = item.selected; n.focused = item.focused;
        if (item.tree) {
            n.tree_level = item.tree->depth; n.expandable = item.tree->expandable;
            n.expanded = item.tree->expanded; n.loading = item.tree->loading;
            n.disclosed_count = item.tree->expanded ? item.tree->child_count : 0;
            if (n.loading) n.help = "Loading children";
            if (!item.tree->parent.empty()) {
                std::string encoded;
                for (char c : item.tree->parent) { if (c == '/') encoded += "%2F"; else if (c == '%') encoded += "%25"; else encoded += c; }
                n.tree_parent = item.group + "/option/" + encoded;
            }
        }
        if (item.bounds && item.clip) {
            n.origin = {item.bounds->x, item.bounds->y}; n.size = {item.bounds->width, item.bounds->height};
            n.clip_origin = {item.clip->x, item.clip->y}; n.clip_size = {item.clip->width, item.clip->height};
        }
        return n;
    };
    provider->item_at = [weak, logical](const std::string& id, std::size_t index) -> std::optional<sw::AccessibilityElement> {
        if (auto h = weak.lock()) if (auto item = h->list_item(id, index)) return logical(*item);
        return {};
    };
    const auto logical_cell = [](const LogicalCell& cell) {
        sw::AccessibilityElement n;
        n.id = cell.id;
        n.label = cell.label;
        n.group = cell.group;
        n.value_text = cell.value;
        n.role = sw::AccessibilityRole::cell;
        n.item_index = cell.row_index;
        n.column_index = cell.column_index;
        n.table_id = cell.table;
        n.enabled = cell.enabled;
        n.selected = cell.selected;
        n.focused = cell.focused;
        n.read_only = !cell.editable;
        n.help = cell.validation;
        if (cell.bounds && cell.clip) {
            n.origin = {cell.bounds->x, cell.bounds->y};
            n.size = {cell.bounds->width, cell.bounds->height};
            n.clip_origin = {cell.clip->x, cell.clip->y};
            n.clip_size = {cell.clip->width, cell.clip->height};
        }
        return n;
    };
    const auto logical_column = [](const LogicalColumn& col) {
        sw::AccessibilityElement n;
        n.id = col.id;
        n.label = col.label;
        n.group = col.table;
        n.table_id = col.table;
        n.header_id = col.header_id;
        n.role = sw::AccessibilityRole::column;
        n.column_index = col.index;
        n.column_count = col.count;
        n.item_count = col.row_count;
        n.enabled = col.enabled;
        n.value = col.width;
        n.minimum = col.min_width;
        n.maximum = col.max_width;
        n.sortable = col.sortable;
        n.sort_priority = col.sort_priority;
        if(col.sort_priority)n.help="Sort priority "+std::to_string(*col.sort_priority+1);
        if (col.sort_order)
            n.sort_ascending = *col.sort_order == SortOrder::ascending;
        if (col.bounds && col.clip) {
            n.origin = {col.bounds->x, col.bounds->y};
            n.size = {col.bounds->width, col.bounds->height};
            n.clip_origin = {col.clip->x, col.clip->y};
            n.clip_size = {col.clip->width, col.clip->height};
        }
        return n;
    };
    provider->logical_element = [weak, logical, logical_cell, logical_column](
                                    const std::string& id) -> std::optional<sw::AccessibilityElement> {
        if (auto h = weak.lock()) {
            if (auto cell = h->table_cell(id))
                return logical_cell(*cell);
            if (auto column = h->table_column(id))
                return logical_column(*column);
            if (auto row = h->list_item(id))
                return logical(*row);
        }
        return {};
    };
    provider->cell_at = [weak, logical_cell](const std::string& id,
                                             std::size_t index) -> std::optional<sw::AccessibilityElement> {
        if (auto h = weak.lock())
            if (auto cell = h->table_cell_at(id, index))
                return logical_cell(*cell);
        return {};
    };
    provider->column_at = [weak,
                           logical_column](const std::string& id,
                                           std::size_t index) -> std::optional<sw::AccessibilityElement> {
        if (auto h = weak.lock())
            if (auto col = h->table_column(id, index))
                return logical_column(*col);
        return {};
    };
    provider->column_cell_at = [weak,
                                logical_cell](const std::string& id,
                                              std::size_t index) -> std::optional<sw::AccessibilityElement> {
        if (auto h = weak.lock())
            if (auto cell = h->table_column_cell(id, index))
                return logical_cell(*cell);
        return {};
    };
    provider->begin_edit = [weak](const std::string& id) {
        if (auto h = weak.lock())
            return h->edit_cell(id);
        return false;
    };
    provider->set_selected = [weak](const std::string& id, bool value) {
        if (auto h = weak.lock())
            return h->set_selected(id, value);
        return false;
    };
    provider->set_selection = [weak](const std::string& id, std::vector<std::string> values) {
        if (auto h = weak.lock())
            return h->set_selection(id, std::move(values));
        return false;
    };
    provider->child_at = [weak, logical](const std::string& id,
                                         std::size_t index) -> std::optional<sw::AccessibilityElement> {
        if (auto h = weak.lock())
            if (auto item = h->tree_child(id, index))
                return logical(*item);
        return {};
    };
    provider->focus = [weak](const std::string& id) {
        if (auto h = weak.lock())
            return h->focus(id);
        return false;
    };
    provider->set_expanded = [weak](const std::string& id, bool expanded) {
        if (auto h = weak.lock()) return h->set_expanded(id, expanded);
        return false;
    };
    provider->activate = [weak](const std::string& id) {
        if (auto h = weak.lock())
            return h->activate(id);
        return false;
    };
    provider->adjust = [weak](const std::string& id, double n) {
        if (auto h = weak.lock())
            return h->adjust(id, n);
        return false;
    };
    provider->set_value = [weak](const std::string& id, double n) {
        if (auto h = weak.lock())
            return h->set_value(id, n);
        return false;
    };
    provider->set_text = [weak](const std::string& id, std::string value) {
        if (auto h = weak.lock()) {
            if(h->table_cell(id))return h->set_cell_text(id,std::move(value));
            return h->set_text(id, std::move(value));
        }
        return false;
    };
    provider->select_text = [weak](const std::string& id, std::size_t start, std::size_t length) {
        auto h = weak.lock();
        if (!h)
            return false;
        for (const auto& element : h->elements())
            if (element.id == id && element.role == Role::text_field && !element.secure)
                return h->select_text(id, {utf8_offset(element.value_text, start),
                                           utf8_offset(element.value_text, start + length)});
        return false;
    };
    window.set_accessibility_provider(std::move(provider));
    host->set_clipboard(
        {[] { return sw::clipboard_text(); }, [](const std::string& text) { sw::set_clipboard_text(text); }});
    auto input = std::make_shared<sw::TextInputClient>();
    input->deactivate = [weak] { if (auto host = weak.lock()) host->blur(); };
    input->intercept_key = [weak](const sw::KeyEvent& event) {
        if (event.modifiers.alt && !event.modifiers.shift && !event.modifiers.command && !event.modifiers.control &&
            (event.key == sw::Key::down || event.key == sw::Key::up)) {
            if (auto host = weak.lock()) {
                const auto state = host->text_input();
                if (state && (state->combo || state->date_picker) && !state->marked)
                    return host->handle(KeyEvent{event.key == sw::Key::down ? Key::down : Key::up,
                        event.down, event.repeat, event.modifiers.shift, event.modifiers.command, true, event.modifiers.control});
            }
        }
        if (event.key == sw::Key::f10 && event.modifiers.shift) {
            if (auto host = weak.lock()) return host->handle(KeyEvent{Key::f10, event.down, event.repeat, true,
                event.modifiers.command, event.modifiers.alt, event.modifiers.control});
            return false;
        }
        if (event.key == sw::Key::f && event.modifiers.command) {
            if (auto host = weak.lock())
                return host->handle(KeyEvent{Key::f, event.down, event.repeat, event.modifiers.shift, true});
            return false;
        }
        if (event.key != sw::Key::tab || !event.modifiers.control || event.modifiers.alt || event.modifiers.command)
            return false;
        if (auto host = weak.lock())
            return host->handle(KeyEvent{Key::tab, event.down, event.repeat, event.modifiers.shift,
                                         false, false, true});
        return false;
    };
    input->state = [weak]() -> std::optional<sw::TextInputState> {
        auto h = weak.lock();
        if (!h)
            return {};
        auto state = h->text_input();
        if (!state)
            return {};
        auto native_range = [&](TextRange range) {
            const auto start = utf16_offset(state->text, range.start);
            return sw::TextInputRange{start, utf16_offset(state->text, range.end()) - start};
        };
        sw::TextInputState result{state->text,
                                  native_range(state->selection.range()),
                                  {},
                                  state->editable,
                                  state->multiline,
                                  state->can_undo,
                                  state->can_redo,
                                  state->secure, state->active};
        if (state->marked)
            result.marked = native_range(*state->marked);
        return result;
    };
    auto replacement = [weak](std::optional<sw::TextInputRange> range) -> std::optional<TextRange> {
        auto h = weak.lock();
        if (!h || !range)
            return {};
        auto state = h->text_input();
        if (!state)
            return {};
        const auto start = utf8_offset(state->text, range->start);
        return TextRange{start, utf8_offset(state->text, range->start + range->length) - start};
    };
    input->insert = [weak, replacement](std::string text, std::optional<sw::TextInputRange> range) {
        if (auto h = weak.lock())
            h->insert_text(std::move(text), replacement(range));
    };
    input->mark = [weak, replacement](std::string text, sw::TextInputRange selected,
                                      std::optional<sw::TextInputRange> range) {
        if (auto h = weak.lock()) {
            const auto start = utf8_offset(text, selected.start);
            const auto end = utf8_offset(text, selected.start + selected.length);
            h->mark_text(std::move(text), {start, end - start}, replacement(range));
        }
    };
    input->unmark = [weak](bool commit) {
        if (auto h = weak.lock())
            h->unmark_text(commit);
    };
    input->rectangle = [weak, replacement](sw::TextInputRange range) {
        Rect r;
        if (auto h = weak.lock())
            if (auto utf8 = replacement(range))
                r = h->text_range_rect(*utf8);
        return sw::TextInputRect{{r.x, r.y}, {r.width, r.height}};
    };
    input->index = [weak](sw::Point point) -> std::size_t {
        auto h = weak.lock();
        if (!h)
            return 0;
        const auto index = h->text_index_at({point.x, point.y});
        auto state = h->text_input();
        return state ? utf16_offset(state->text, index) : 0;
    };
    input->command = [weak](const std::string& selector) {
        auto h = weak.lock();
        if (!h || !h->text_input())
            return false;
        if (selector == "insertTab:" || selector == "insertBacktab:")
            return h->handle(KeyEvent{Key::tab, true, false, selector == "insertBacktab:"});
        if (selector == "insertTabIgnoringFieldEditor:")
            return h->insert_text("\t");
        if (selector == "insertNewline:" || selector == "insertNewlineIgnoringFieldEditor:")
            return h->handle(KeyEvent{Key::enter});
        if (selector == "cancelOperation:")
            return h->handle(KeyEvent{Key::escape});
        if (selector == "selectWord:") {
            h->text_command(TextCommand::word_right);
            return h->text_command(TextCommand::word_left, true);
        }
        if (selector == "selectLine:" || selector == "selectParagraph:") {
            h->text_command(TextCommand::line_start);
            return h->text_command(TextCommand::line_end, true);
        }
        auto name = selector;
        const bool extend = name.ends_with("AndModifySelection:");
        if (extend)
            name.replace(name.size() - std::string("AndModifySelection:").size(), std::string::npos, ":");
        static const std::unordered_map<std::string, TextCommand> commands{
            {"moveLeft:", TextCommand::left},
            {"moveBackward:", TextCommand::left},
            {"moveRight:", TextCommand::right},
            {"moveForward:", TextCommand::right},
            {"moveWordLeft:", TextCommand::word_left},
            {"moveWordBackward:", TextCommand::word_left},
            {"moveWordRight:", TextCommand::word_right},
            {"moveWordForward:", TextCommand::word_right},
            {"moveUp:", TextCommand::up},
            {"moveDown:", TextCommand::down},
            {"pageUp:", TextCommand::page_up},
            {"pageDown:", TextCommand::page_down},
            {"moveToBeginningOfLine:", TextCommand::line_start},
            {"moveToEndOfLine:", TextCommand::line_end},
            {"moveToBeginningOfParagraph:", TextCommand::paragraph_start},
            {"moveToEndOfParagraph:", TextCommand::paragraph_end},
            {"moveParagraphBackward:", TextCommand::paragraph_start},
            {"moveParagraphForward:", TextCommand::paragraph_end},
            {"scrollToBeginningOfDocument:", TextCommand::document_start},
            {"scrollToEndOfDocument:", TextCommand::document_end},
            {"scrollPageUp:", TextCommand::page_up},
            {"scrollPageDown:", TextCommand::page_down},
            {"moveToBeginningOfDocument:", TextCommand::document_start},
            {"moveToEndOfDocument:", TextCommand::document_end},
            {"deleteBackward:", TextCommand::backspace},
            {"deleteForward:", TextCommand::delete_forward},
            {"deleteBackwardByDecomposingPreviousCharacter:", TextCommand::backspace},
            {"deleteWordBackward:", TextCommand::delete_word_backward},
            {"deleteWordForward:", TextCommand::delete_word_forward},
            {"deleteToBeginningOfLine:", TextCommand::delete_line_backward},
            {"deleteToEndOfLine:", TextCommand::delete_line_forward},
            {"deleteToBeginningOfParagraph:", TextCommand::delete_paragraph_backward},
            {"deleteToEndOfParagraph:", TextCommand::delete_paragraph_forward},
            {"selectAll:", TextCommand::select_all},
            {"copy:", TextCommand::copy},
            {"cut:", TextCommand::cut},
            {"paste:", TextCommand::paste},
            {"undo:", TextCommand::undo},
            {"redo:", TextCommand::redo},
            {"transpose:", TextCommand::transpose}};
        auto i = commands.find(name);
        return i != commands.end() && h->text_command(i->second, extend);
    };
    window.set_text_input_client(std::move(input));

    return host;
}
void unmount(skia_window::Surface& window) {
    window.set_redraw_delay({});
    window.set_cursor(skia_window::Cursor::arrow);
    window.set_text_input_client({});
    window.set_callbacks({});
    window.set_accessibility_provider({});
    window.set_animating(false);
}
} // namespace scars::macos

namespace scars::macos {
struct AlertPresentation::Impl {
    std::unique_ptr<skia_window::Dialog> dialog;
    std::shared_ptr<Alert> alert;
    std::shared_ptr<Host> host;
    std::function<void(std::optional<std::string>)> completion;
    bool finished{};
    void finish() {
        if (std::exchange(finished, true))
            return;
        if (!alert->completed())
            alert->dismiss();
        const auto result = alert->result();
        auto callback = std::exchange(completion, {});
        if (callback)
            callback(result);
    }
    void respond() {
        if (finished)
            return;
        if (dialog && dialog->is_open())
            dialog->dismiss(alert->result() ? skia_window::DialogResult::accepted
                                            : skia_window::DialogResult::cancelled);
        else
            finish();
    }
    void dismiss() {
        if (!finished)
            alert->dismiss();
    }
};
AlertPresentation::AlertPresentation(std::shared_ptr<Impl> impl) : impl_(std::move(impl)) {}
AlertPresentation::~AlertPresentation() {
    auto state = std::move(impl_);
    if (state)
        state->dismiss();
}
std::expected<std::unique_ptr<AlertPresentation>, skia_window::Error>
AlertPresentation::present(skia_window::Window& parent, std::string title, View content,
                           std::vector<AlertAction> actions,
                           std::function<void(std::optional<std::string>)> completion, Theme theme,
                           skia_window::WindowOptions options) {
    return present(
        parent, std::move(title), [content = std::move(content)] { return content; }, std::move(actions),
        std::move(completion), std::move(theme), std::move(options));
}
std::expected<std::unique_ptr<AlertPresentation>, skia_window::Error>
AlertPresentation::present(skia_window::Window& parent, std::string title, std::function<View()> content,
                           std::vector<AlertAction> actions,
                           std::function<void(std::optional<std::string>)> completion, Theme theme,
                           skia_window::WindowOptions options) {
    options.title = title;
    auto dialog = skia_window::Dialog::create(parent, std::move(options));
    if (!dialog)
        return std::unexpected(dialog.error());
    auto state = std::make_shared<Impl>();
    state->dialog = std::move(*dialog);
    state->completion = std::move(completion);
    std::weak_ptr<Impl> weak = state;
    state->alert =
        std::make_shared<Alert>(std::move(title), std::move(content), std::move(actions), [weak](auto) {
            if (auto live = weak.lock())
                live->respond();
        });
    state->host = mount(
        *state->dialog, [alert = state->alert] { return alert->enabled(!alert->completed()); },
        std::move(theme));
    const auto size = state->dialog->content_size();
    state->host->layout({size.width, size.height});
    const auto elements = state->host->elements();
    auto initial = std::find_if(elements.begin(), elements.end(), [](const auto& element) {
        return element.role == Role::text_field && element.enabled && !element.read_only;
    });
    if (initial == elements.end())
        initial = std::find_if(elements.begin(), elements.end(), [](const auto& element) {
            return element.enabled && element.alert_action == AlertActionRole::default_action;
        });
    if (initial == elements.end())
        initial = std::find_if(elements.begin(), elements.end(), [](const auto& element) {
            return element.enabled && element.alert_action.has_value() &&
                   element.alert_action != AlertActionRole::destructive;
        });
    if (initial != elements.end())
        state->host->focus(initial->id);
    auto shown = state->dialog->show_modal([weak](auto) {
        if (auto live = weak.lock())
            live->finish();
    });
    if (!shown) {
        state->finished = true;
        state->completion = {};
        return std::unexpected(shown.error());
    }
    return std::unique_ptr<AlertPresentation>(new AlertPresentation(std::move(state)));
}
bool AlertPresentation::choose(const std::string& id) {
    auto state = impl_;
    if (!state || state->finished || !state->dialog->is_open())
        return false;
    for (const auto& node : state->host->inspect())
        if (node.key == id)
            for (const auto& element : state->host->elements())
                if (element.id == node.id && element.alert_action)
                    return state->host->activate(element.id);
    return false;
}
void AlertPresentation::dismiss() {
    auto state = impl_;
    if (state)
        state->dismiss();
}
bool AlertPresentation::is_open() const {
    return impl_ && !impl_->finished && impl_->dialog && impl_->dialog->is_open();
}
std::shared_ptr<Host> AlertPresentation::host() const {
    return impl_ ? impl_->host : nullptr;
}
std::expected<std::unique_ptr<AlertPresentation>, skia_window::Error>
confirm(skia_window::Window& parent, std::string title, std::string message,
        std::function<void(bool)> completion, Theme theme, ConfirmationOptions options) {
    return AlertPresentation::present(
        parent, std::move(title), Label(std::move(message)).text_wrap(TextWrap::wrap),
        {{"cancel", std::move(options.cancel_label), AlertActionRole::cancel},
         {"accept", std::move(options.accept_label),
          options.destructive ? AlertActionRole::destructive : AlertActionRole::default_action}},
        [completion = std::move(completion)](auto result) {
            if (completion)
                completion(result == "accept");
        },
        std::move(theme));
}
struct DeveloperTools::Impl : std::enable_shared_from_this<DeveloperTools::Impl> {
    skia_window::Application* application;
    std::shared_ptr<Host> source;
    Theme theme;
    std::shared_ptr<Inspector> model;
    std::unique_ptr<skia_window::Window> window;
    std::shared_ptr<Host> host;
    bool refresh_pending{};
    std::string revealed;
    Impl(skia_window::Application& app, std::shared_ptr<Host> target, Theme t)
        : application(&app), source(std::move(target)), theme(std::move(t)) {
        if (!source)
            throw std::invalid_argument("DeveloperTools requires a source host");
        model = std::make_shared<Inspector>(source);
    }
    void detached() {
        source->set_inspection_observer({});
        source->set_inspection_picking(false);
        source->set_inspected_element({});
        source->set_inspection_overlay(InspectionOverlay::selection);
        source->clear_layout_overrides();
        host.reset();
    }
    void close() {
        detached();
        if (window && window->is_open())
            window->close();
        window.reset();
    }
    void show() {
        namespace sw = skia_window;
        if (window && window->is_open()) {
            window->show();
            window->focus();
            return;
        }
        window.reset();
        auto created = sw::Window::create(*application, {.title = "SCARS Developer Tools",
                                                         .size = {980, 760},
                                                         .minimum_size = {720, 480},
                                                         .resizable = true,
                                                         .floating = true});
        if (!created)
            throw std::runtime_error(created.error().message);
        window = std::move(*created);
        std::weak_ptr<Impl> weak = shared_from_this();
        theme = source->theme();
        window->set_appearance(theme.light_mode ? skia_window::Appearance::light
                                                : skia_window::Appearance::dark);
        host = mount(*window, [model = model] { return model->body(); }, theme, {},
                     {.on_closed =
                          [weak] {
                              if (auto self = weak.lock())
                                  self->detached();
                          },
                      .on_event =
                          [weak](const sw::Event& event) {
                              if (auto self = weak.lock())
                                  if (auto key = std::get_if<sw::KeyEvent>(&event);
                                      key && key->down && key->key == sw::Key::escape &&
                                      self->source->inspection_picking()) {
                                      self->source->set_inspection_picking(false);
                                      return true;
                                  }
                              return false;
                          }});
        source->set_inspection_observer([weak] {
            if (auto self = weak.lock())
                self->refresh();
        });
        auto menus = std::vector<sw::MenuItem>{
            sw::MenuItem::submenu("SCARS Lab", {sw::MenuItem::action(1, "Quit SCARS Lab", {sw::Key::q})}),
            sw::MenuItem::submenu("Developer",
                                  {sw::MenuItem::action(2, "Toggle Element Explorer",
                                                        {sw::Key::i, {.alt = true, .command = true}})})};
        auto result = window->set_menu(menus, [weak](sw::Command command) {
            if (auto self = weak.lock()) {
                if (command == 1)
                    self->application->quit();
                else if (command == 2)
                    self->toggle();
            }
        });
        if (!result)
            throw std::runtime_error(result.error().message);
        window->show();
        window->focus();
        refresh();
    }
    void refresh() {
        if (!host || !window || !window->is_open())
            return;
        host->invalidate();
        if (refresh_pending)
            return;
        refresh_pending = true;
        std::weak_ptr<Impl> weak = shared_from_this();
        application->post([weak] {
            auto self = weak.lock();
            if (!self)
                return;
            if (self->host && self->window && self->window->is_open()) {
                auto theme = self->source->theme();
                self->host->set_theme(theme);
                self->window->set_appearance(theme.light_mode ? skia_window::Appearance::light
                                                              : skia_window::Appearance::dark);
                const auto target = self->source->inspected_element();
                if (target != self->revealed) {
                    self->model->reveal_selection();
                    const auto size = self->window->content_size();
                    self->host->layout({size.width, size.height});
                    auto tree = self->host->inspect();
                    auto row = std::find_if(tree.begin(), tree.end(),
                                            [&](const auto& n) { return n.key == target; });
                    if (row != tree.end())
                        self->host->reveal(row->id);
                    self->revealed = target;
                }
            }
            self->refresh_pending = false;
        });
    }
    void toggle() {
        if (window && window->is_open() && !window->is_minimized())
            close();
        else
            show();
    }
};
DeveloperTools::DeveloperTools(skia_window::Application& app, std::shared_ptr<Host> source, Theme theme)
    : impl_(std::make_shared<Impl>(app, std::move(source), std::move(theme))) {}
DeveloperTools::~DeveloperTools() {
    impl_->close();
}
void DeveloperTools::show() {
    impl_->show();
}
void DeveloperTools::toggle() {
    impl_->toggle();
}
void DeveloperTools::close() {
    impl_->close();
}
bool DeveloperTools::is_open() const {
    return impl_->window && impl_->window->is_open();
}
} // namespace scars::macos
