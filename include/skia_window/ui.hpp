#pragma once

#include <skia_window/skia_window.hpp>
#include "include/core/SkRefCnt.h"
#include "include/core/SkTypeface.h"
#include "include/core/SkImage.h"
#include <algorithm>
#include <concepts>
#include <initializer_list>
#include <limits>
#include <optional>
#include <stdexcept>
#include <thread>
#include <type_traits>
#include <typeinfo>

class SkTypeface;
class SkImage;

namespace skia_window::ui {

struct Rect {
    double x{}, y{}, width{}, height{};
    bool contains(Point p) const {
        return p.x >= x && p.y >= y && p.x < x + width && p.y < y + height;
    }
};
struct Length {
    double value{};
    enum class Unit { dp, percent, em } unit{Unit::dp};
};
struct Dimension {
    enum class Kind { automatic, length, minimum_content, maximum_content, unlimited } kind{Kind::automatic};
    Length value;
    constexpr Dimension() = default;
    constexpr Dimension(Length length) : kind(Kind::length), value(length) {}
    constexpr explicit Dimension(Kind kind) : kind(kind) {}
};
inline constexpr Dimension auto_size{}, min_content{Dimension::Kind::minimum_content},
    max_content{Dimension::Kind::maximum_content}, unbounded{Dimension::Kind::unlimited};
struct Fraction {
    double value{1};
};
namespace literals {
constexpr Length operator""_dp(unsigned long long n) {
    return {static_cast<double>(n)};
}
constexpr Length operator""_dp(long double n) {
    return {static_cast<double>(n)};
}
constexpr Length operator""_pct(unsigned long long n) {
    return {static_cast<double>(n) / 100, Length::Unit::percent};
}
constexpr Length operator""_pct(long double n) {
    return {static_cast<double>(n) / 100, Length::Unit::percent};
}
constexpr Length operator""_em(unsigned long long n) {
    return {static_cast<double>(n), Length::Unit::em};
}
constexpr Length operator""_em(long double n) {
    return {static_cast<double>(n), Length::Unit::em};
}
constexpr Fraction operator""_fr(unsigned long long n) {
    return {static_cast<double>(n)};
}
constexpr Fraction operator""_fr(long double n) {
    return {static_cast<double>(n)};
}
} // namespace literals
struct Edges {
    Length top{}, right{}, bottom{}, left{};
    static constexpr Edges all(Length n) {
        return {n, n, n, n};
    }
    static constexpr Edges symmetric(Length vertical, Length horizontal) {
        return {vertical, horizontal, vertical, horizontal};
    }
};
enum class Align { start, center, end, stretch, baseline };
enum class Justify { start, center, end, space_between, space_around };
enum class Direction { row, column };
enum class Wrap { none, wrap };
struct BoxStyle {
    Dimension width{}, height{}, min_width{}, min_height{}, max_width{unbounded}, max_height{unbounded};
    Edges margin{}, padding{};
    std::optional<double> aspect_ratio;
};
struct Flex {
    Direction direction{Direction::row};
    Wrap wrap{Wrap::none};
    Length gap{};
    Justify justify{Justify::start};
    Align align{Align::stretch};
};
struct TrackSize {
    Dimension minimum{Length{0}}, maximum{};
    double fraction{};
    TrackSize() = default;
    TrackSize(Length n) : minimum(n), maximum(n) {}
    TrackSize(Fraction n) : fraction(n.value) {}
    TrackSize(Dimension n) : maximum(n) {}
};
TrackSize minmax(Dimension minimum, Fraction maximum);
TrackSize minmax(Dimension minimum, Dimension maximum);
struct GridLayout {
    std::vector<TrackSize> columns{Fraction{1}}, rows;
    Length column_gap{}, row_gap{};
};
struct OverlayLayout {
    Align horizontal{Align::center}, vertical{Align::center};
};
class LayoutAlgorithm;
using Layout = std::variant<Flex, GridLayout, OverlayLayout, std::shared_ptr<const LayoutAlgorithm>>;
struct FlexItem {
    double grow{}, shrink{1};
    Dimension basis{};
    std::optional<Align> align_self;
};
struct GridItem {
    std::optional<std::size_t> row, column; // Zero-based; omitted positions are auto-placed.
    std::size_t row_span{1}, column_span{1};
    Align horizontal{Align::stretch}, vertical{Align::stretch};
};
struct OverlayItem {
    Align horizontal{Align::center}, vertical{Align::center};
    Point offset{};
};
struct ItemStyle {
    FlexItem flex;
    GridItem grid;
    OverlayItem overlay;
};
struct Constraints {
    Size minimum{};
    Size maximum{std::numeric_limits<double>::infinity(), std::numeric_limits<double>::infinity()};
};
class LayoutContext {
  public:
    virtual ~LayoutContext() = default;
    virtual std::size_t child_count() const = 0;
    virtual const BoxStyle& child_box(std::size_t) const = 0;
    virtual const ItemStyle& child_item(std::size_t) const = 0;
    virtual Size measure_child(std::size_t, Constraints) = 0;
    virtual void place_child(std::size_t, Rect) = 0;
};
class LayoutAlgorithm {
  public:
    virtual ~LayoutAlgorithm() = default;
    // Sizes and rectangles describe the container's content box, excluding its own padding.
    // Layout is cached: these operations must be deterministic for the descriptions/constraints.
    // Use observed state or Host::invalidate() when external layout inputs change.
    virtual Size measure(LayoutContext&, Constraints) const = 0;
    virtual void arrange(LayoutContext&, Rect content_bounds) const = 0;
};

enum class Curve { linear, ease_out, ease_in_out, spring };
struct Animation {
    std::chrono::duration<double> duration{};
    Curve curve{Curve::ease_in_out};
    static Animation ease_out(std::chrono::duration<double> n) {
        return {n, Curve::ease_out};
    }
    static Animation ease_in_out(std::chrono::duration<double> n) {
        return {n, Curve::ease_in_out};
    }
    static Animation spring(std::chrono::duration<double> n = std::chrono::milliseconds(600)) {
        return {n, Curve::spring};
    }
};
namespace detail {
struct Observer {
    std::function<void(Animation)> changed;
    std::uint64_t epoch{};
};
struct Subscription {
    std::weak_ptr<Observer> observer;
    std::uint64_t epoch{};
};
struct StateCellBase {
    std::thread::id owner{std::this_thread::get_id()};
    std::vector<Subscription> observers;
    void read();
    void changed();
    void check_thread() const;
};
template <class T> struct StateCell : StateCellBase {
    T value;
    explicit StateCell(T value) : value(std::move(value)) {}
};
Animation exchange_animation(Animation);
bool building_view();
struct Description;
} // namespace detail
template <class F> decltype(auto) animate(Animation animation, F&& change) {
    struct Scope {
        Animation previous;
        ~Scope() {
            detail::exchange_animation(previous);
        }
    } scope{detail::exchange_animation(animation)};
    return std::forward<F>(change)();
}
template <class T> class Binding {
  public:
    T get() const {
        cell_->read();
        return cell_->value;
    }
    void set(T value) const {
        cell_->check_thread();
        if (detail::building_view())
            throw std::logic_error("State cannot be changed inside body()");
        if constexpr (std::equality_comparable<T>)
            if (cell_->value == value)
                return;
        cell_->value = std::move(value);
        cell_->changed();
    }

  private:
    explicit Binding(std::shared_ptr<detail::StateCell<T>> cell) : cell_(std::move(cell)) {}
    std::shared_ptr<detail::StateCell<T>> cell_;
    template <class U> friend class State;
};
template <class T> class State {
  public:
    explicit State(T value) : cell_(std::make_shared<detail::StateCell<T>>(std::move(value))) {}
    T get() const {
        return binding().get();
    }
    void set(T value) const {
        binding().set(std::move(value));
    }
    Binding<T> binding() const {
        return Binding<T>{cell_};
    }

  private:
    std::shared_ptr<detail::StateCell<T>> cell_;
};

enum class Edge { leading, trailing, top, bottom };
struct TransitionEffect {
    double opacity{1}, scale{1};
    double x{}, y{}; // Fractions of the content's width/height at the inactive endpoint.
};
struct Transition {
    TransitionEffect enter{}, leave{};
    bool enabled{};
    static Transition fade();
    static Transition scale(double factor = .92);
    static Transition slide_from(Edge);
    static Transition push(Edge);
    static Transition asymmetric(Transition enter, Transition leave);
    Transition combined_with(Transition) const;
};
enum class ContentTransition { none, cross_fade, numeric_text };
enum class SizeTransition { immediate, animate };
enum class FontWeight { regular, semibold };
struct PaintInfo {
    Size content_size;
    double scale{1};
    std::chrono::duration<double> elapsed{}, delta{};
};
struct Theme {
    sk_sp<SkTypeface> typeface;
    std::uint32_t background{0xff10191f}, surface{0xff1c2b34}, foreground{0xffeaf4f6}, muted{0xff91a9b6},
        accent{0xff66e0c2}, border{0xff3a505d};
    double font_size{14};
};
struct SliderOptions {
    double minimum{}, maximum{1}, step{.01};
};

// Immutable descriptions: modifiers return a new description; mounted identity lives in Host.
class View {
  public:
    View();
    View key(std::string) const;
    View box(BoxStyle) const;
    View item(ItemStyle) const;
    View width(Dimension) const;
    View height(Dimension) const;
    View min_width(Dimension) const;
    View min_height(Dimension) const;
    View max_width(Dimension) const;
    View max_height(Dimension) const;
    View padding(Length) const;
    View padding(Edges) const;
    View margin(Edges) const;
    View grow(double n = 1) const;
    View shrink(double n) const;
    View gap(Length) const;
    View align_items(Align) const;
    View justify(Justify) const;
    View wrap(Wrap) const;
    View layout(Layout) const;
    View responsive_layout(Length minimum_width, Layout wide) const;
    View font_size(Length) const;
    View font_weight(FontWeight) const;
    View foreground(std::uint32_t) const;
    View background(std::uint32_t) const;
    View corner_radius(double) const;
    View opacity(double) const;
    View enabled(bool) const;
    View visible(bool) const;
    View clip(bool = true) const;
    View text_wrap(bool) const;
    View accessible_label(std::string) const;
    View animating(bool = true) const;
    View content_transition(ContentTransition) const;
    View transition(Transition) const;
    View size_transition(SizeTransition) const;
    View animation(Animation) const;
    View animate_layout(bool = true) const;

  private:
    std::shared_ptr<const detail::Description> description_;
    explicit View(std::shared_ptr<const detail::Description>);
    View modify(const std::function<void(detail::Description&)>&) const;
    friend struct detail::Description;
    friend struct Text;
    friend struct Button;
    friend struct CheckBox;
    friend struct Slider;
    friend struct Row;
    friend struct Column;
    friend struct Container;
    friend struct Grid;
    friend struct Overlay;
    friend struct ScrollView;
    friend struct Canvas;
    friend struct Image;
    friend struct Spacer;
    friend class Host;
    friend View switch_content(std::string, View);
    friend View group(std::vector<View>);
    friend View component_description(std::string, std::string, std::function<std::shared_ptr<void>()>,
                                      std::function<View(const std::shared_ptr<void>&)>);
};
struct Text : View {
    explicit Text(std::string);
};
using Label = Text;
struct Button : View {
    Button(std::string text, std::function<void()> on_click);
};
struct CheckBox : View {
    CheckBox(std::string text, Binding<bool>);
};
struct Slider : View {
    Slider(Binding<double>, SliderOptions = {});
};
struct Row : View {
    Row(std::initializer_list<View>);
};
struct Column : View {
    Column(std::initializer_list<View>);
};
struct Container : View {
    Container(Layout, std::vector<View>);
};
struct Grid : View {
    Grid(GridLayout, std::initializer_list<View>);
};
struct Overlay : View {
    Overlay(std::initializer_list<View>);
};
struct ScrollView : View {
    explicit ScrollView(View);
};
struct Canvas : View {
    explicit Canvas(std::function<void(SkCanvas&, const PaintInfo&)>);
};
struct Image : View {
    explicit Image(sk_sp<SkImage>);
};
struct Spacer : View {
    Spacer();
};
using Empty = View;
View group(std::vector<View>);
View switch_content(std::string selection, View);
View component_description(std::string key, std::string type, std::function<std::shared_ptr<void>()> mount,
                           std::function<View(const std::shared_ptr<void>&)> body);
// A nested model is created once per keyed identity. Props are fresh values on every update.
template <class T, class Props> View component(std::string key, Props props) {
    return component_description(
        std::move(key), typeid(T).name(),
        [] { return std::static_pointer_cast<void>(std::make_shared<T>()); },
        [props = std::move(props)](const std::shared_ptr<void>& model) {
            return std::static_pointer_cast<T>(model)->body(props);
        });
}
template <class T> View component(std::string key) {
    return component_description(
        std::move(key), typeid(T).name(),
        [] { return std::static_pointer_cast<void>(std::make_shared<T>()); },
        [](const std::shared_ptr<void>& model) { return std::static_pointer_cast<T>(model)->body(); });
}

template <class T> std::string identity(const T& value) {
    if constexpr (std::is_enum_v<T>)
        return std::to_string(std::to_underlying(value));
    else if constexpr (std::is_integral_v<T>)
        return std::to_string(value);
    else
        return std::string(value);
}
struct ContentSwitcher : View {
    template <class Key, class Builder>
    ContentSwitcher(const Key& selection, Builder&& builder)
        : View(switch_content(identity(selection), std::forward<Builder>(builder)(selection))) {}
};
template <class Builder> View when(bool present, Builder&& builder) {
    return switch_content(present ? "present" : "absent",
                          present ? std::forward<Builder>(builder)() : View{});
}
template <class Range, class Id, class Builder> View for_each(const Range& values, Id id, Builder&& builder) {
    std::vector<View> children;
    for (const auto& value : values)
        children.push_back(builder(value).key(identity(std::invoke(id, value))));
    return group(std::move(children));
}

enum class Role { text, button, checkbox, slider, image, canvas };
struct ElementInfo {
    std::string id, key, label;
    Role role{};
    Rect bounds, clip;
    bool enabled{}, focused{}, checked{};
    double value{}, minimum{}, maximum{};
};
struct TransitionInfo {
    std::string id;
    double progress{};
    bool has_outgoing{};
};

// A desktop-independent runtime, also suitable for deterministic layout/input/paint tests.
class Host final {
  public:
    explicit Host(std::function<View()> builder, std::function<void()> wake = {}, Theme = {});
    ~Host();
    Host(const Host&) = delete;
    Host& operator=(const Host&) = delete;
    void set_theme(Theme);
    void set_reduced_motion(bool);        // Application preference; combined with the platform setting.
    void set_system_reduced_motion(bool); // Environment input, used by native hosts.
    void invalidate();
    void layout(Size);
    void paint(SkCanvas&, const FrameInfo&);
    bool handle(const Event&);
    bool needs_frame() const;
    std::vector<ElementInfo> elements() const;
    std::vector<TransitionInfo> transitions() const;
    bool activate(const std::string& id);
    bool adjust(const std::string& id, double steps);
    bool set_value(const std::string& id, double value);
    bool focus(const std::string& id);
    std::uint64_t generation() const;

  private:
    struct Impl;
    std::shared_ptr<Impl> impl_;
};

} // namespace skia_window::ui

namespace skia_window {
template <class T, class... Args> std::shared_ptr<T> Surface::set_view(Args&&... args) {
    auto model = std::make_shared<T>(std::forward<Args>(args)...);
    set_view_builder([model] { return model->body(); });
    return model;
}
} // namespace skia_window
