#pragma once

#include <chrono>
#include <cstdint>
#include <expected>
#include <functional>
#include <memory>
#include <optional>
#include <span>
#include <string>
#include <variant>
#include <vector>

class SkCanvas;

namespace skia_window {

struct Error { std::string message; };
struct Size { double width{}, height{}; };
struct Point { double x{}, y{}; };
struct Modifiers {
    bool shift{}, control{}, alt{}, command{};
    friend bool operator==(const Modifiers&, const Modifiers&) = default;
};

enum class Key {
    unknown, a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u, v, w, x, y, z,
    num0, num1, num2, num3, num4, num5, num6, num7, num8, num9,
    escape, enter, tab, space, backspace, delete_forward, left, right, up, down,
    home, end, page_up, page_down, f1, f2, f3, f4, f5, f6, f7, f8, f9, f10, f11, f12
};
enum class PointerKind { move, down, up, scroll, enter, leave };
struct PointerEvent {
    PointerKind kind{};
    Point position;
    int button{}; // 0 = primary, 1 = secondary, 2+ = additional buttons
    double scroll_x{}, scroll_y{}; // Native signed scroll deltas (points on precise devices).
    Modifiers modifiers;
    int click_count{1};
};
struct KeyEvent {
    Key key{};
    bool down{}, repeat{};
    Modifiers modifiers;
    std::uint16_t native_code{};
    std::string text{};
};
struct ModifierEvent { Modifiers modifiers; };
struct FocusEvent { bool focused{}; };
struct ResizeEvent { Size logical_size, pixel_size; double scale{}; };
using Event = std::variant<PointerEvent, KeyEvent, ModifierEvent, FocusEvent, ResizeEvent>;

struct FrameInfo {
    Size logical_size, pixel_size;
    double scale{1};
    std::chrono::duration<double> elapsed{}, delta{};
    std::uint64_t frame_number{};
};
struct Callbacks {
    // The canvas is borrowed, already scaled to logical units, and valid only during this call.
    std::function<void(SkCanvas&, const FrameInfo&)> on_draw;
    std::function<void(const Event&)> on_event;
    std::function<bool()> on_close_request; // False vetoes a user-initiated close.
    std::function<void()> on_closed;
};
struct WindowOptions {
    std::string title{"Skia window"};
    Size size{1100, 800};
    Size minimum_size{480, 360};
    bool resizable{true};
    bool floating{}; // App-local utility panel; remains minimizable.
};
// Optional semantic bridge for independent renderers. This does not mount a UI engine.
enum class Cursor { arrow, text, resize_horizontal, resize_vertical };
enum class AccessibilityRole {
    text, button, checkbox, radio, slider, progress, group, menu_button, selection, menu_item, image, text_field, list_box, list_item, splitter, tab, tab_group, stepper, switch_control, menu, combo_box, tree_view, table, column_header, cell, column, alert
};
struct AccessibilityElement {
    std::string id, label, group, value_text;
    AccessibilityRole role{};
    Point origin, clip_origin;
    Size size, clip_size;
    bool enabled{true}, focused{}, selected{}, mixed{}, expanded{};
    double value{}, minimum{}, maximum{1};
    std::uint64_t paint_order{}; // Optional hit-test order, independent of semantic reading order.
    std::size_t text_selection_start{}, text_selection_length{}; // UTF-16, as required by AppKit.
    bool read_only{}, multiline{};
    bool vertical{}; // Orientation of the accessible divider or control itself.
    std::string help;
    bool secure{}, search{};
    std::optional<std::size_t> item_count, item_index;
    std::vector<std::size_t> selected_items;
    std::optional<std::size_t> tree_level;
    std::string tree_parent;
    bool expandable{}, loading{};
    std::size_t disclosed_count{};
    std::optional<std::size_t> column_index, column_count, sort_priority;
    std::optional<bool> sort_ascending;
    std::vector<std::string> selected_cells;
    std::string table_id, header_id;
    bool sortable{};
    bool default_action{}, cancel_action{}, destructive_action{};
};
struct AccessibilityProvider {
    std::function<std::vector<AccessibilityElement>(Size)> elements;
    std::function<std::uint64_t()> generation;
    std::function<bool(const std::string&)> focus, activate;
    std::function<bool(const std::string&, double)> adjust, set_value;
    std::function<bool(const std::string&, std::string)> set_text;
    std::function<bool(const std::string&, std::size_t, std::size_t)> select_text;
    // Optional logical rows. Indexed access must not realize the entire list.
    std::function<std::optional<AccessibilityElement>(const std::string&, std::size_t)> item_at;
    std::function<std::optional<AccessibilityElement>(const std::string&)> logical_element;
    std::function<bool(const std::string&, bool)> set_expanded;
    std::function<std::optional<AccessibilityElement>(const std::string&, std::size_t)> child_at;
    std::function<void(Size)> prepare;
    std::function<std::optional<AccessibilityElement>(const std::string&, std::size_t)> cell_at, column_at, column_cell_at;
    std::function<bool(const std::string&)> begin_edit;
    std::function<bool(const std::string&, bool)> set_selected;
    std::function<bool(const std::string&, std::vector<std::string>)> set_selection;
};
// Text input only; the independent renderer retains ownership of editing and layout.
// All native ranges use UTF-16 code units. Strings are UTF-8.
struct TextInputRange { std::size_t start{}, length{}; };
struct TextInputState {
    std::string text;
    TextInputRange selection;
    std::optional<TextInputRange> marked;
    bool editable{}, multiline{}, can_undo{}, can_redo{};
    bool secure{}, active{true};
};
struct TextInputRect { Point origin; Size size; };
struct TextInputClient {
    std::function<std::optional<TextInputState>()> state;
    std::function<void(std::string, std::optional<TextInputRange>)> insert;
    std::function<void(std::string, TextInputRange, std::optional<TextInputRange>)> mark;
    std::function<void(bool)> unmark;
    std::function<bool(const std::string&)> command;
    std::function<TextInputRect(TextInputRange)> rectangle;
    std::function<std::size_t(Point)> index;
    // Return true to consume a shortcut before AppKit interprets it as editing.
    std::function<bool(const KeyEvent&)> intercept_key;
    std::function<void()> deactivate; // Releases focus-related state before replacement or close.
};
std::string clipboard_text();
void set_clipboard_text(const std::string&);
enum class Appearance { system, light, dark };
struct ApplicationOptions {
    std::string name{"Skia Window"};
    std::function<void(const Error&)> on_error;
    std::function<void()> on_new_window;
};
struct Shortcut { Key key{}; Modifiers modifiers{.command = true}; };
using Command = std::uint32_t;
struct MenuItem {
    Command command{}; // Nonzero and unique within a window for actionable entries.
    std::string label;
    Shortcut shortcut;
    bool enabled{true}, checked{}, separator{};
    std::vector<MenuItem> children;

    static MenuItem action(Command command, std::string label, Shortcut shortcut = {});
    static MenuItem submenu(std::string label, std::vector<MenuItem> children);
    static MenuItem divider();
};

namespace detail { struct ApplicationState; struct SurfaceState; }
namespace ui { class View; class Host; }

// All operations and callbacks run on the main thread. Application must outlive its surfaces.
class Application final {
public:
    static std::expected<std::unique_ptr<Application>, Error> create(ApplicationOptions = {});
    ~Application();
    Application(const Application&) = delete;
    Application& operator=(const Application&) = delete;
    int run();
    void quit();
    // Deferred onto the main event loop; pending functions are discarded on application destruction.
    void post(std::function<void()>);
    std::string gpu_description() const;

private:
    explicit Application(std::shared_ptr<detail::ApplicationState>);
    std::shared_ptr<detail::ApplicationState> state_;
    friend class Window;
};

class Surface {
public:
    virtual ~Surface();
    Surface(const Surface&) = delete;
    Surface& operator=(const Surface&) = delete;
    void set_callbacks(Callbacks);
    void set_accessibility_provider(std::shared_ptr<AccessibilityProvider>);
    void set_text_input_client(std::shared_ptr<TextInputClient>);
    void set_cursor(Cursor);
    // Include ui.hpp for declarative view construction. The host owns the mounted model.
    void set_view_builder(std::function<ui::View()>);
    void set_content(ui::View);
    void clear_content();
    std::shared_ptr<ui::Host> ui_host() const;
    template<class T, class... Args> std::shared_ptr<T> set_view(Args&&... args);
    void set_title(std::string);
    void show();
    void hide();
    void minimize();
    bool is_minimized() const;
    void focus();
    void close(); // Programmatic close; does not consult on_close_request. Idempotent.
    bool is_open() const;
    void request_redraw();
    void set_redraw_delay(std::optional<std::chrono::milliseconds>); // Replaces/cancels the pending one-shot redraw.
    void set_animating(bool);
    bool is_animating() const;
    Size content_size() const;

protected:
    explicit Surface(std::shared_ptr<detail::SurfaceState>);
    std::shared_ptr<detail::SurfaceState> state_;
};

class Window final : public Surface {
public:
    static std::expected<std::unique_ptr<Window>, Error> create(Application&, WindowOptions = {});
    ~Window() override;
    std::expected<void, Error> set_menu(std::span<const MenuItem>, std::function<void(Command)>);
    void set_command_state(Command, bool enabled, bool checked);
    // Uses the same validation and callback path as a native menu action; useful for automation.
    bool invoke_command(Command);
    void set_appearance(Appearance);
    void set_titlebar_theme_toggle(bool light, std::function<void(bool)>);
    void set_titlebar_theme_toggle_state(bool light);

private:
    using Surface::Surface;
    friend class Dialog;
};

enum class DialogResult { accepted, cancelled };
class Dialog final : public Surface {
public:
    static std::expected<std::unique_ptr<Dialog>, Error> create(Window&, WindowOptions = {});
    ~Dialog() override;
    // Returns immediately. Completion runs once; close, destruction, or parent close cancels it.
    std::expected<void, Error> show_modal(std::function<void(DialogResult)> completion);
    void dismiss(DialogResult = DialogResult::cancelled);

private:
    using Surface::Surface;
};

} // namespace skia_window
