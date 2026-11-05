#include <skia_window/ui.hpp>
#include "include/core/SkCanvas.h"
#include "include/core/SkFont.h"
#include "include/core/SkFontMetrics.h"
#include "include/core/SkImage.h"
#include "include/core/SkPaint.h"
#include "include/core/SkPathBuilder.h"
#include "include/core/SkPicture.h"
#include "include/core/SkPictureRecorder.h"
#include "include/core/SkRRect.h"
#include "include/core/SkTypeface.h"
#include <cmath>
#include <numeric>
#include <unordered_map>
#include <unordered_set>

namespace skia_window::detail {
sk_sp<SkTypeface> default_typeface();
}
namespace skia_window::ui {
namespace detail {
thread_local std::shared_ptr<Observer> collecting;
thread_local bool in_body = false;
thread_local Animation transaction;
bool building_view() {
    return in_body;
}
Animation exchange_animation(Animation value) {
    if (!std::isfinite(value.duration.count()) || value.duration.count() < 0)
        throw std::invalid_argument("Animation duration must be finite and nonnegative");
    return std::exchange(transaction, value);
}
void StateCellBase::check_thread() const {
    if (owner != std::this_thread::get_id())
        throw std::logic_error("UI state must be accessed on its owning thread");
}
void StateCellBase::read() {
    check_thread();
    if (!collecting)
        return;
    std::erase_if(observers, [](const auto& value) { return value.observer.expired(); });
    for (auto& entry : observers)
        if (entry.observer.lock() == collecting) {
            entry.epoch = collecting->epoch;
            return;
        }
    observers.push_back({collecting, collecting->epoch});
}
void StateCellBase::changed() {
    auto snapshot = observers;
    for (const auto& entry : snapshot)
        if (auto observer = entry.observer.lock(); observer && entry.epoch == observer->epoch)
            observer->changed(transaction);
}
enum class Kind {
    empty,
    text,
    button,
    checkbox,
    slider,
    container,
    scroll,
    canvas,
    image,
    spacer,
    switcher,
    component
};
struct Description {
    Kind kind{Kind::empty};
    BoxStyle box;
    ItemStyle item;
    Layout layout{Flex{}};
    std::optional<std::pair<Length, Layout>> responsive;
    std::vector<View> children;
    std::string key, text, label, selection, component_type;
    std::function<std::shared_ptr<void>()> mount;
    std::function<View(const std::shared_ptr<void>&)> component_body;
    std::function<void()> click;
    std::function<void(double)> change;
    std::function<double()> read_value;
    std::function<void(SkCanvas&, const PaintInfo&)> draw;
    sk_sp<SkImage> image;
    double value{}, minimum{}, maximum{1}, step{.01};
    std::optional<Length> font_size;
    std::optional<std::uint32_t> foreground, background;
    FontWeight weight{FontWeight::regular};
    double radius{8}, opacity{1};
    bool enabled{true}, visible{true}, clip{}, wrap_text{true}, animating{}, animate_layout{};
    ContentTransition content_transition{ContentTransition::none};
    Transition transition;
    SizeTransition size_transition{SizeTransition::immediate};
    std::optional<Animation> animation;
    static const Description& of(const View& view) {
        return *view.description_;
    }
    static View refresh(View view) {
        if (!of(view).read_value)
            return view;
        auto copy = std::make_shared<Description>(of(view));
        copy->value = copy->read_value();
        return View(copy);
    }
};
static auto make(Kind kind) {
    auto value = std::make_shared<Description>();
    value->kind = kind;
    return value;
}
} // namespace detail
using detail::Description;
using detail::Kind;

TrackSize minmax(Dimension minimum, Fraction maximum) {
    TrackSize t(maximum);
    t.minimum = minimum;
    return t;
}
TrackSize minmax(Dimension minimum, Dimension maximum) {
    TrackSize t(maximum);
    t.minimum = minimum;
    return t;
}
Transition Transition::fade() {
    return {{0, 1, 0, 0}, {0, 1, 0, 0}, true};
}
Transition Transition::scale(double factor) {
    if (!std::isfinite(factor) || factor <= 0)
        throw std::invalid_argument("Transition scale must be positive");
    return {{1, factor, 0, 0}, {1, factor, 0, 0}, true};
}
Transition Transition::slide_from(Edge edge) {
    Transition t;
    t.enabled = true;
    switch (edge) {
    case Edge::leading:
        t.enter.x = -1;
        break;
    case Edge::trailing:
        t.enter.x = 1;
        break;
    case Edge::top:
        t.enter.y = -1;
        break;
    case Edge::bottom:
        t.enter.y = 1;
        break;
    }
    t.leave = t.enter;
    return t;
}
Transition Transition::push(Edge edge) {
    auto t = slide_from(edge);
    t.leave.x = -t.enter.x;
    t.leave.y = -t.enter.y;
    return t;
}
Transition Transition::asymmetric(Transition enter, Transition leave) {
    return {enter.enter, leave.leave, true};
}
Transition Transition::combined_with(Transition other) const {
    auto combine = [](auto a, auto b) {
        return TransitionEffect{a.opacity * b.opacity, a.scale * b.scale, a.x + b.x, a.y + b.y};
    };
    return {combine(enter, other.enter), combine(leave, other.leave), enabled || other.enabled};
}
View::View() : description_(detail::make(Kind::empty)) {}
View::View(std::shared_ptr<const Description> value) : description_(std::move(value)) {}
View View::modify(const std::function<void(Description&)>& edit) const {
    auto copy = std::make_shared<Description>(*description_);
    edit(*copy);
    return View(std::move(copy));
}
View View::key(std::string n) const {
    return modify([&](auto& d) { d.key = std::move(n); });
}
View View::box(BoxStyle n) const {
    return modify([&](auto& d) { d.box = n; });
}
View View::item(ItemStyle n) const {
    return modify([&](auto& d) { d.item = n; });
}
#define SW_BOX_METHOD(name)                                                                                  \
    View View::name(Dimension n) const {                                                                     \
        return modify([&](auto& d) { d.box.name = n; });                                                     \
    }
SW_BOX_METHOD(width)
SW_BOX_METHOD(height)
SW_BOX_METHOD(min_width)
SW_BOX_METHOD(min_height)
SW_BOX_METHOD(max_width)
SW_BOX_METHOD(max_height)
#undef SW_BOX_METHOD
View View::padding(Length n) const {
    return padding(Edges::all(n));
}
View View::padding(Edges n) const {
    return modify([&](auto& d) { d.box.padding = n; });
}
View View::margin(Edges n) const {
    return modify([&](auto& d) { d.box.margin = n; });
}
View View::grow(double n) const {
    if (!std::isfinite(n) || n < 0)
        throw std::invalid_argument("Flex grow must be finite and nonnegative");
    return modify([&](auto& d) { d.item.flex.grow = n; });
}
View View::shrink(double n) const {
    if (!std::isfinite(n) || n < 0)
        throw std::invalid_argument("Flex shrink must be finite and nonnegative");
    return modify([&](auto& d) { d.item.flex.shrink = n; });
}
View View::gap(Length n) const {
    return modify([&](auto& d) {
        if (auto* f = std::get_if<Flex>(&d.layout))
            f->gap = n;
        if (auto* g = std::get_if<GridLayout>(&d.layout))
            g->column_gap = g->row_gap = n;
    });
}
View View::align_items(Align n) const {
    return modify([&](auto& d) {
        if (auto* f = std::get_if<Flex>(&d.layout))
            f->align = n;
    });
}
View View::justify(Justify n) const {
    return modify([&](auto& d) {
        if (auto* f = std::get_if<Flex>(&d.layout))
            f->justify = n;
    });
}
View View::wrap(Wrap n) const {
    return modify([&](auto& d) {
        if (auto* f = std::get_if<Flex>(&d.layout))
            f->wrap = n;
    });
}
View View::layout(Layout n) const {
    return modify([&](auto& d) { d.layout = std::move(n); });
}
View View::responsive_layout(Length width, Layout n) const {
    return modify([&](auto& d) { d.responsive = std::pair{width, std::move(n)}; });
}
View View::font_size(Length n) const {
    return modify([&](auto& d) { d.font_size = n; });
}
View View::font_weight(FontWeight n) const {
    return modify([&](auto& d) { d.weight = n; });
}
View View::foreground(std::uint32_t n) const {
    return modify([&](auto& d) { d.foreground = n; });
}
View View::background(std::uint32_t n) const {
    return modify([&](auto& d) { d.background = n; });
}
View View::corner_radius(double n) const {
    return modify([&](auto& d) { d.radius = std::max(0.0, n); });
}
View View::opacity(double n) const {
    return modify([&](auto& d) { d.opacity = std::clamp(n, 0.0, 1.0); });
}
View View::enabled(bool n) const {
    return modify([&](auto& d) { d.enabled = n; });
}
View View::visible(bool n) const {
    return modify([&](auto& d) { d.visible = n; });
}
View View::clip(bool n) const {
    return modify([&](auto& d) { d.clip = n; });
}
View View::text_wrap(bool n) const {
    return modify([&](auto& d) { d.wrap_text = n; });
}
View View::accessible_label(std::string n) const {
    return modify([&](auto& d) { d.label = std::move(n); });
}
View View::animating(bool n) const {
    return modify([&](auto& d) { d.animating = n; });
}
View View::content_transition(ContentTransition n) const {
    return modify([&](auto& d) { d.content_transition = n; });
}
View View::transition(Transition n) const {
    return modify([&](auto& d) { d.transition = n; });
}
View View::size_transition(SizeTransition n) const {
    return modify([&](auto& d) { d.size_transition = n; });
}
View View::animation(Animation n) const {
    return modify([&](auto& d) { d.animation = n; });
}
View View::animate_layout(bool n) const {
    return modify([&](auto& d) { d.animate_layout = n; });
}

Text::Text(std::string text)
    : View([&] {
          auto d = detail::make(Kind::text);
          d->text = std::move(text);
          return d;
      }()) {}
Button::Button(std::string text, std::function<void()> click)
    : View([&] {
          auto d = detail::make(Kind::button);
          d->text = std::move(text);
          d->click = std::move(click);
          return d;
      }()) {}
CheckBox::CheckBox(std::string text, Binding<bool> value)
    : View([&] {
          auto d = detail::make(Kind::checkbox);
          d->text = std::move(text);
          d->value = value.get();
          d->read_value = [value] { return value.get() ? 1.0 : 0.0; };
          d->change = [value](double n) { value.set(n != 0); };
          return d;
      }()) {}
Slider::Slider(Binding<double> value, SliderOptions options)
    : View([&] {
          if (!std::isfinite(options.minimum) || !std::isfinite(options.maximum) ||
              options.maximum <= options.minimum || !std::isfinite(options.step) || options.step <= 0)
              throw std::invalid_argument("Slider needs a finite increasing range and positive step");
          auto d = detail::make(Kind::slider);
          d->value = std::clamp(value.get(), options.minimum, options.maximum);
          d->read_value = [value, options] {
              auto n = value.get();
              if (!std::isfinite(n))
                  throw std::invalid_argument("Slider value must be finite");
              return std::clamp(n, options.minimum, options.maximum);
          };
          d->minimum = options.minimum;
          d->maximum = options.maximum;
          d->step = options.step;
          d->change = [value](double n) { value.set(n); };
          return d;
      }()) {}
Row::Row(std::initializer_list<View> children)
    : View([&] {
          auto d = detail::make(Kind::container);
          d->children = children;
          return d;
      }()) {}
Column::Column(std::initializer_list<View> children)
    : View([&] {
          auto d = detail::make(Kind::container);
          d->layout = Flex{.direction = Direction::column};
          d->children = children;
          return d;
      }()) {}
Container::Container(Layout layout, std::vector<View> children)
    : View([&] {
          auto d = detail::make(Kind::container);
          d->layout = std::move(layout);
          d->children = std::move(children);
          return d;
      }()) {}
Grid::Grid(GridLayout layout, std::initializer_list<View> children)
    : View([&] {
          auto d = detail::make(Kind::container);
          d->layout = std::move(layout);
          d->children = children;
          return d;
      }()) {}
Overlay::Overlay(std::initializer_list<View> children)
    : View([&] {
          auto d = detail::make(Kind::container);
          d->layout = OverlayLayout{};
          d->children = children;
          return d;
      }()) {}
ScrollView::ScrollView(View child)
    : View([&] {
          auto d = detail::make(Kind::scroll);
          d->children.push_back(std::move(child));
          d->clip = true;
          return d;
      }()) {}
Canvas::Canvas(std::function<void(SkCanvas&, const PaintInfo&)> draw)
    : View([&] {
          auto d = detail::make(Kind::canvas);
          d->draw = std::move(draw);
          return d;
      }()) {}
Image::Image(sk_sp<SkImage> image)
    : View([&] {
          auto d = detail::make(Kind::image);
          d->image = std::move(image);
          return d;
      }()) {}
Spacer::Spacer()
    : View([&] {
          auto d = detail::make(Kind::spacer);
          d->item.flex.grow = 1;
          return d;
      }()) {}
View group(std::vector<View> children) {
    return Container(Flex{.direction = Direction::column}, std::move(children));
}
View switch_content(std::string selection, View child) {
    auto d = detail::make(Kind::switcher);
    d->selection = std::move(selection);
    d->children.push_back(std::move(child));
    return View(d);
}
View component_description(std::string key, std::string type, std::function<std::shared_ptr<void>()> mount,
                           std::function<View(const std::shared_ptr<void>&)> body) {
    auto d = detail::make(Kind::component);
    d->key = std::move(key);
    d->component_type = std::move(type);
    d->mount = std::move(mount);
    d->component_body = std::move(body);
    return View(d);
}

namespace {
constexpr double infinity = std::numeric_limits<double>::infinity();
double finite(double n, double fallback = 0) {
    return std::isfinite(n) ? n : fallback;
}
double lerp(double a, double b, double t) {
    return a + (b - a) * t;
}
Rect intersect(Rect a, Rect b) {
    double x = std::max(a.x, b.x), y = std::max(a.y, b.y);
    return {x, y, std::max(0.0, std::min(a.x + a.width, b.x + b.width) - x),
            std::max(0.0, std::min(a.y + a.height, b.y + b.height) - y)};
}
SkRect skrect(Rect r) {
    return SkRect::MakeXYWH((float)r.x, (float)r.y, (float)r.width, (float)r.height);
}
struct Insets {
    double top{}, right{}, bottom{}, left{};
    double horizontal() const {
        return left + right;
    }
    double vertical() const {
        return top + bottom;
    }
};
double resolve(Length n, double available, double em) {
    if (!std::isfinite(n.value))
        throw std::invalid_argument("Lengths must be finite");
    if (n.unit == Length::Unit::percent)
        return std::isfinite(available) ? n.value * available : 0;
    return n.value * (n.unit == Length::Unit::em ? em : 1);
}
double resolve(Dimension n, double available, double em, double natural) {
    switch (n.kind) {
    case Dimension::Kind::length:
        return std::max(0.0, resolve(n.value, available, em));
    case Dimension::Kind::unlimited:
        return infinity;
    default:
        return natural;
    }
}
double bounded(double value, Dimension minimum, Dimension maximum, double available, double font) {
    const auto low = resolve(minimum, available, font, 0);
    const auto high = resolve(maximum, available, font, infinity);
    if (!std::isfinite(low))
        throw std::invalid_argument("Minimum sizes must be finite");
    return std::clamp(value, low, std::max(low, high));
}
Insets insets(Edges n, double width, double em) {
    return {resolve(n.top, width, em), resolve(n.right, width, em), resolve(n.bottom, width, em),
            resolve(n.left, width, em)};
}
Rect inset(Rect r, Insets i) {
    return {r.x + i.left, r.y + i.top, std::max(0.0, r.width - i.horizontal()),
            std::max(0.0, r.height - i.vertical())};
}
struct Node;
bool empty_slot(const std::shared_ptr<Node>&);
bool focusable(Kind k) {
    return k == Kind::button || k == Kind::checkbox || k == Kind::slider;
}
bool semantic(Kind k) {
    return focusable(k) || k == Kind::text || k == Kind::image || k == Kind::canvas;
}
double curve(Animation a, double t) {
    t = std::clamp(t, 0.0, 1.0);
    switch (a.curve) {
    case Curve::linear:
        return t;
    case Curve::ease_out:
        return 1 - std::pow(1 - t, 3);
    case Curve::ease_in_out:
        return t * t * (3 - 2 * t);
    case Curve::spring:
        return t >= 1 ? 1 : 1 - std::exp(-8 * t) * std::cos(14 * t);
    }
    return t;
}
struct Node {
    struct Measurement {
        Size available, result, intrinsic;
        Layout layout;
    };
    View view;
    std::string id, parent;
    Rect bounds{}, clip{};
    Size measured{};
    double font{14};
    std::uint32_t color{};
    bool enabled{}, visible{};
    Layout layout{Flex{}};
    std::vector<Measurement> measurements;
    mutable std::vector<double> text_advances;
    std::uint64_t measurement_epoch{};
    std::vector<std::shared_ptr<Node>> children;
    std::weak_ptr<Node> parent_node;
    const Description& d() const {
        return Description::of(view);
    }
};
bool empty_slot(const std::shared_ptr<Node>& node) {
    return (node->d().kind == Kind::switcher || node->d().kind == Kind::component) &&
           node->measured.width == 0 && node->measured.height == 0;
}
struct Persistent {
    Rect bounds{}, from_bounds{};
    Size measured{}, from_size{};
    double scroll{}, scroll_max{}, started{};
    Animation animation;
    Transition transition;
    ContentTransition content_transition{};
    sk_sp<SkPicture> presented, outgoing;
    std::string signature;
    std::shared_ptr<void> model;
    bool active{}, entering{}, layout_active{}, seen{}, property_active{};
    double from_opacity{1}, opacity{1}, target_opacity{1}, from_font{14}, font{14}, target_font{14};
    std::uint32_t from_color{}, color{}, target_color{}, from_background{}, background{}, target_background{};
};
struct Ghost {
    sk_sp<SkPicture> picture;
    Rect bounds, clip;
    TransitionEffect effect;
    double started;
    Animation animation;
};
struct Collect {
    std::shared_ptr<detail::Observer> previous;
    bool was_building;
    Collect(std::shared_ptr<detail::Observer> observer, bool body = false)
        : previous(std::exchange(detail::collecting, std::move(observer))),
          was_building(std::exchange(detail::in_body, body)) {}
    ~Collect() {
        detail::collecting = std::move(previous);
        detail::in_body = was_building;
    }
};
} // namespace

struct Host::Impl : std::enable_shared_from_this<Impl> {
    std::function<View()> builder;
    std::function<void()> wake;
    Theme theme;
    std::shared_ptr<detail::Observer> observer;
    std::shared_ptr<Node> root;
    std::unordered_map<std::string, Persistent> states;
    std::unordered_map<std::string, std::shared_ptr<Node>> index;
    std::vector<std::shared_ptr<Node>> order;
    std::vector<Ghost> ghosts;
    Size viewport{};
    Size laid_out_size{-1, -1};
    FrameInfo frame;
    Animation pending;
    bool dirty{true}, layout_dirty{true}, repaint{true}, reduced{}, user_reduced{}, system_reduced{},
        window_focused{true};
    std::string focused, hovered, captured, keyboard_pressed;
    std::uint64_t revision{}, measurement_epoch{};
    std::thread::id owner{std::this_thread::get_id()};
    double now{};

    void check() const {
        if (owner != std::this_thread::get_id())
            throw std::logic_error("UI Host must be used on its owning thread");
    }
    void request_paint() {
        repaint = true;
        if (wake)
            wake();
    }
    void invalidate(Animation animation = {}) {
        dirty = true;
        layout_dirty = true;
        pending = animation;
        request_paint();
    }
    double progress(const Persistent& p) const {
        if (!std::isfinite(p.started))
            return 0;
        return p.animation.duration.count() > 0
                   ? std::clamp((now - p.started) / p.animation.duration.count(), 0.0, 1.0)
                   : 1;
    }
    SkFont font(const Node& n, bool presentation = false) const {
        SkFont f(theme.typeface, (float)(presentation ? states.at(n.id).font : n.font));
        f.setSubpixel(true);
        f.setEmbolden(n.d().weight == FontWeight::semibold);
        return f;
    }
    double baseline(const std::shared_ptr<Node>& node, double height) const {
        const auto& d = node->d();
        const auto padding = insets(d.box.padding, node->measured.width, node->font);
        if (d.kind == Kind::text)
            return padding.top + node->font;
        if (d.kind == Kind::button || d.kind == Kind::checkbox)
            return padding.top + (height - padding.vertical() - node->font * 1.35) / 2 + node->font;
        if ((d.kind == Kind::component || d.kind == Kind::switcher) && !node->children.empty())
            return padding.top + baseline(node->children.front(), node->children.front()->measured.height);
        return height;
    }
    void prepare_text(const Node& n) const;
    double text_width(const Node& n, std::string_view text) const {
        if (text.data() == n.d().text.data() && text.size() == n.d().text.size()) {
            prepare_text(n);
            return n.text_advances.back();
        }
        return font(n).measureText(text.data(), text.size(), SkTextEncoding::kUTF8);
    }
    std::vector<std::string> lines(const Node& n, double width) const;
    std::shared_ptr<Node> build(View view, std::string parent, std::size_t position, double inherited_font,
                                std::uint32_t inherited_color, bool enabled, bool visible);
    void rebuild();
    Size measure(const std::shared_ptr<Node>& n, Size available);
    std::vector<std::size_t> grid_positions(const std::shared_ptr<Node>& n, std::size_t columns,
                                            std::size_t& rows);
    std::vector<double> tracks(const std::shared_ptr<Node>& n, bool horizontal, double available,
                               const std::vector<std::size_t>& positions, std::size_t count);
    void arrange(const std::shared_ptr<Node>& n, Rect bounds, Rect clip);
    void perform_layout();
    void paint_node(SkCanvas& canvas, const std::shared_ptr<Node>& n, bool record = true);
    void paint_contents(SkCanvas& canvas, const std::shared_ptr<Node>& n);
    void picture(SkCanvas&, const sk_sp<SkPicture>&, Rect, TransitionEffect, double opacity,
                 SkBlendMode blend = SkBlendMode::kSrcOver);
    Point point_for(const std::shared_ptr<Node>&, Point) const;
    Rect presentation_bounds(const std::shared_ptr<Node>&) const;
    void sample_properties();
    bool busy() const;
    std::shared_ptr<Node> hit(Point point) const;
    bool activate(const std::string& id);
    bool adjust(const std::string& id, double steps);
    bool set_value(const std::string& id, double value);
    bool focus(const std::string& id);
    bool handle(const Event&);
    void slider_pointer(const std::shared_ptr<Node>& n, double x);
    std::vector<ElementInfo> elements() const;
};

void Host::Impl::prepare_text(const Node& n) const {
    if (!n.text_advances.empty())
        return;
    const auto& text = n.d().text;
    auto f = font(n);
    std::vector<SkGlyphID> glyphs(f.countText(text.data(), text.size(), SkTextEncoding::kUTF8));
    f.textToGlyphs(text.data(), text.size(), SkTextEncoding::kUTF8, {glyphs.data(), glyphs.size()});
    std::vector<SkScalar> widths(glyphs.size());
    f.getWidths({glyphs.data(), glyphs.size()}, {widths.data(), widths.size()});
    n.text_advances.resize(text.size() + 1);
    std::size_t glyph = 0;
    double advance = 0;
    for (std::size_t i = 0; i < text.size();) {
        const auto byte = static_cast<unsigned char>(text[i]);
        const auto length = std::min<std::size_t>(byte < 0x80   ? 1
                                                  : byte < 0xe0 ? 2
                                                  : byte < 0xf0 ? 3
                                                                : 4,
                                                  text.size() - i);
        if (glyph < widths.size())
            advance += widths[glyph++];
        i += length;
        n.text_advances[i] = advance;
    }
}
std::vector<std::string> Host::Impl::lines(const Node& n, double width) const {
    prepare_text(n);
    std::vector<std::string> result;
    const auto& text = n.d().text;
    std::size_t start = 0;
    // Break on UTF-8 code point boundaries; remember the last whitespace for word wrapping.
    // Bulk glyph advances make wrapping linear, rather than measuring every growing prefix.
    std::size_t last_space = std::string::npos;
    for (std::size_t i = 0; i < text.size();) {
        auto byte = static_cast<unsigned char>(text[i]);
        const auto length = std::min<std::size_t>(byte < 0x80 ? 1 : (byte < 0xe0 ? 2 : (byte < 0xf0 ? 3 : 4)),
                                                  text.size() - i);
        if (text[i] == '\n') {
            result.push_back(text.substr(start, i - start));
            start = i + 1;
            last_space = std::string::npos;
            ++i;
            continue;
        }
        if (n.d().wrap_text && i > start && std::isfinite(width) &&
            n.text_advances[i + length] - n.text_advances[start] > std::max(1.0, width)) {
            if (last_space != std::string::npos) {
                result.push_back(text.substr(start, last_space - start));
                start = last_space + 1;
            } else {
                result.push_back(text.substr(start, i - start));
                start = i;
            }
            last_space = std::string::npos;
        }
        if (text[i] == ' ' || text[i] == '\t')
            last_space = i;
        i += length;
    }
    result.push_back(text.substr(start));
    return result;
}
std::shared_ptr<Node> Host::Impl::build(View view, std::string parent, std::size_t position,
                                        double inherited_font, std::uint32_t inherited_color, bool enabled,
                                        bool visible) {
    auto node = std::make_shared<Node>();
    node->view = Description::refresh(std::move(view));
    const auto& d = node->d();
    node->parent = parent;
    node->id =
        parent + "/" + std::to_string((int)d.kind) + ":" + d.component_type + ":" +
        (d.key.empty() ? "#" + std::to_string(position) : "@" + std::to_string(d.key.size()) + ":" + d.key);
    node->font =
        d.font_size ? std::max(1.0, resolve(*d.font_size, inherited_font, inherited_font)) : inherited_font;
    node->color = d.foreground.value_or(inherited_color);
    node->enabled = enabled && d.enabled;
    node->visible = visible && d.visible;
    index[node->id] = node;
    order.push_back(node);
    auto [it, inserted] = states.try_emplace(node->id);
    auto& state = it->second;
    state.seen = true;
    auto chosen = d.animation.value_or(pending);
    if (reduced)
        chosen.duration = {};
    const auto signature = d.kind == Kind::switcher ? d.selection : d.text;
    bool changed = !inserted && state.signature != signature;
    if (changed && (d.kind == Kind::switcher || d.content_transition != ContentTransition::none)) {
        state.outgoing = state.presented;
        state.from_size = state.measured;
        state.from_bounds = state.bounds;
        state.started = infinity;
        state.animation = chosen;
        state.transition = d.transition.enabled ? d.transition : Transition::fade();
        state.content_transition = d.content_transition;
        state.active = chosen.duration.count() > 0;
        state.entering = !state.outgoing;
    } else if (inserted && revision > 0 && d.transition.enabled && chosen.duration.count() > 0) {
        state.started = infinity;
        state.animation = chosen;
        state.transition = d.transition;
        state.active = true;
        state.entering = true;
    }
    if (d.animate_layout && chosen.duration.count() > 0 && !inserted) {
        state.from_bounds = state.bounds;
        state.started = infinity;
        state.animation = chosen;
        state.layout_active = true;
    }
    const auto background = d.background.value_or(d.kind == Kind::button ? theme.surface : 0);
    if (inserted) {
        state.opacity = state.target_opacity = d.opacity;
        state.font = state.target_font = node->font;
        state.color = state.target_color = node->color;
        state.background = state.target_background = background;
    } else if (state.target_opacity != d.opacity || state.target_font != node->font ||
               state.target_color != node->color || state.target_background != background) {
        state.from_opacity = state.opacity;
        state.from_font = state.font;
        state.from_color = state.color;
        state.from_background = state.background;
        state.target_opacity = d.opacity;
        state.target_font = node->font;
        state.target_color = node->color;
        state.target_background = background;
        state.started = infinity;
        state.animation = chosen;
        state.property_active = chosen.duration.count() > 0;
        if (!state.property_active) {
            state.opacity = d.opacity;
            state.font = node->font;
            state.color = node->color;
            state.background = background;
        }
        if (state.active)
            state.outgoing = state.presented;
    }
    state.signature = signature;
    if (!state.active)
        state.outgoing.reset();
    auto children = d.children;
    if (d.kind == Kind::component) {
        if (!state.model)
            state.model = d.mount();
        children.push_back(d.component_body(state.model));
    }
    std::unordered_set<std::string> keys;
    for (std::size_t i = 0; i < children.size(); ++i) {
        const auto& child = Description::of(children[i]);
        if (!child.key.empty() && !keys.insert(child.key).second)
            throw std::invalid_argument("Sibling view keys must be unique: " + child.key);
        const auto path =
            d.kind == Kind::switcher
                ? node->id + "/selection:" + std::to_string(d.selection.size()) + ":" + d.selection
                : node->id;
        auto child_node = build(children[i], path, i, node->font, node->color, node->enabled, node->visible);
        child_node->parent_node = node;
        node->children.push_back(std::move(child_node));
    }
    return node;
}
void Host::Impl::rebuild() {
    for (auto& [_, p] : states)
        p.seen = false;
    auto old_index = std::move(index);
    auto old_order = std::move(order);
    index.clear();
    order.clear();
    ++observer->epoch;
    Collect collect(observer, true);
    View description = builder();
    root = build(std::move(description), "root", 0, theme.font_size, theme.foreground, true, true);
    // Removed views contribute frozen pictures only, never callbacks, focus, or accessibility nodes.
    for (const auto& old : old_order) {
        auto it = states.find(old->id);
        if (it == states.end() || it->second.seen)
            continue;
        const auto& d = old->d();
        const auto chosen = d.animation.value_or(pending);
        bool parent_removed = !old->parent.empty() && !index.contains(old->parent);
        // A switcher already owns its outgoing subtree's picture.
        if (d.transition.enabled && !parent_removed && !reduced && chosen.duration.count() > 0 &&
            it->second.presented)
            ghosts.push_back(
                {it->second.presented, it->second.bounds, old->clip, d.transition.leave, infinity, chosen});
    }
    std::erase_if(states, [](const auto& pair) { return !pair.second.seen; });
    if (!index.contains(focused) || !index.at(focused)->enabled || !index.at(focused)->visible) {
        if (!focused.empty()) {
            std::shared_ptr<Node> ancestor;
            if (auto it = old_index.find(focused); it != old_index.end())
                ancestor = it->second->parent_node.lock();
            while (ancestor && !index.contains(ancestor->id))
                ancestor = ancestor->parent_node.lock();
            auto matches = [&](const auto& n) {
                return n->enabled && n->visible && focusable(n->d().kind) &&
                       (!ancestor || n->id.starts_with(ancestor->id + "/"));
            };
            auto next = std::find_if(order.begin(), order.end(), matches);
            if (next == order.end())
                next = std::find_if(order.begin(), order.end(), [](const auto& n) {
                    return n->enabled && n->visible && focusable(n->d().kind);
                });
            focused = next == order.end() ? "" : (*next)->id;
        }
    }
    if (!index.contains(captured) || !index.at(captured)->enabled || !index.at(captured)->visible)
        captured.clear();
    dirty = false;
    pending = {};
    ++revision;
}

namespace {
void validate_constraints(Constraints c) {
    for (auto pair :
         {std::pair{c.minimum.width, c.maximum.width}, std::pair{c.minimum.height, c.maximum.height}})
        if (!std::isfinite(pair.first) || std::isnan(pair.second) || pair.first < 0 ||
            pair.second < pair.first)
            throw std::invalid_argument("Invalid layout constraints");
}
class AlgorithmContext final : public LayoutContext {
  public:
    std::shared_ptr<Node> node;
    std::function<Size(std::size_t, Constraints)> measure;
    std::function<void(std::size_t, Rect)> place;
    std::size_t child_count() const override {
        return node->children.size();
    }
    const BoxStyle& child_box(std::size_t i) const override {
        return node->children.at(i)->d().box;
    }
    const ItemStyle& child_item(std::size_t i) const override {
        return node->children.at(i)->d().item;
    }
    Size measure_child(std::size_t i, Constraints c) override {
        validate_constraints(c);
        return measure(i, c);
    }
    void place_child(std::size_t i, Rect rect) override {
        if (!place)
            throw std::logic_error("Placement is only valid during arrange()");
        place(i, rect);
    }
};
} // namespace

std::vector<std::size_t> Host::Impl::grid_positions(const std::shared_ptr<Node>& n, std::size_t columns,
                                                    std::size_t& rows) {
    std::vector<std::size_t> result(n->children.size(), std::numeric_limits<std::size_t>::max());
    std::unordered_set<std::size_t> occupied;
    std::size_t cursor = 0;
    for (std::size_t i = 0; i < n->children.size(); ++i) {
        const auto& child = n->children[i];
        if (!child->visible || child->d().kind == Kind::empty || empty_slot(child))
            continue;
        const auto& p = child->d().item.grid;
        if (p.row_span == 0 || p.column_span == 0 || p.column_span > columns || p.row_span > 4096 ||
            p.row.value_or(0) > 4095 || p.column.value_or(0) >= columns)
            throw std::invalid_argument("Invalid grid placement or span");
        auto fits = [&](std::size_t r, std::size_t c) {
            if (c + p.column_span > columns)
                return false;
            for (std::size_t y = 0; y < p.row_span; ++y)
                for (std::size_t x = 0; x < p.column_span; ++x)
                    if (occupied.contains((r + y) * columns + c + x))
                        return false;
            return true;
        };
        std::size_t r = p.row.value_or(cursor / columns), c = p.column.value_or(cursor % columns);
        if (!(p.row && p.column)) {
            if (p.row)
                c = p.column.value_or(0);
            if (p.column)
                r = p.row.value_or(0);
            for (std::size_t attempts = 0; !fits(r, c); ++attempts) {
                if (attempts >= 4096 * columns)
                    throw std::invalid_argument("Grid placement exceeded 4096 rows");
                if (p.column)
                    ++r;
                else {
                    ++c;
                    if (c + p.column_span > columns) {
                        c = 0;
                        ++r;
                        if (p.row)
                            throw std::invalid_argument("Grid row has no available cell for this span");
                    }
                }
            }
        }
        result[i] = r * columns + c;
        rows = std::max(rows, r + p.row_span);
        if (rows > 4096)
            throw std::invalid_argument("Grid supports at most 4096 rows");
        for (std::size_t y = 0; y < p.row_span; ++y)
            for (std::size_t x = 0; x < p.column_span; ++x)
                occupied.insert((r + y) * columns + c + x);
        if (!p.row && !p.column)
            cursor = r * columns + c + p.column_span;
    }
    return result;
}
std::vector<double> Host::Impl::tracks(const std::shared_ptr<Node>& n, bool horizontal, double available,
                                       const std::vector<std::size_t>& positions, std::size_t count) {
    const auto& grid = std::get<GridLayout>(n->layout);
    const auto& specs = horizontal ? grid.columns : grid.rows;
    const auto columns = grid.columns.size();
    const auto gap = resolve(horizontal ? grid.column_gap : grid.row_gap, available, n->font);
    std::vector<double> sizes(count), maxima(count, infinity), fractions(count);
    std::vector<bool> automatic(count, true);
    for (std::size_t i = 0; i < count; ++i) {
        TrackSize t = i < specs.size() ? specs[i] : TrackSize{auto_size};
        if (t.fraction < 0 || !std::isfinite(t.fraction))
            throw std::invalid_argument("Grid fractions must be finite and nonnegative");
        sizes[i] = resolve(t.minimum, available, n->font, 0);
        maxima[i] = resolve(t.maximum, available, n->font, infinity);
        if (maxima[i] < sizes[i])
            maxima[i] = sizes[i];
        fractions[i] = t.fraction;
        const bool fixed =
            t.minimum.kind == Dimension::Kind::length && t.maximum.kind == Dimension::Kind::length &&
            t.minimum.value.unit == t.maximum.value.unit && t.minimum.value.value == t.maximum.value.value;
        automatic[i] = !fixed && !(t.fraction > 0 && std::isfinite(available));
    }
    for (std::size_t i = 0; i < n->children.size(); ++i) {
        if (positions[i] == std::numeric_limits<std::size_t>::max())
            continue;
        auto& child = n->children[i];
        const auto& p = child->d().item.grid;
        const auto start = horizontal ? positions[i] % columns : positions[i] / columns;
        const auto span = horizontal ? p.column_span : p.row_span;
        const auto margin = insets(child->d().box.margin, available, child->font);
        double need = (horizontal ? child->measured.width + margin.horizontal()
                                  : child->measured.height + margin.vertical()) -
                      gap * (span - 1);
        double sum = std::accumulate(sizes.begin() + start, sizes.begin() + start + span, 0.0);
        for (std::size_t pass = 0; need > sum + .01 && pass < span + 1; ++pass) {
            std::size_t eligible = 0;
            for (auto j = start; j < start + span; ++j)
                if (automatic[j] && sizes[j] < maxima[j])
                    ++eligible;
            if (!eligible)
                break;
            const double extra = (need - sum) / eligible;
            for (auto j = start; j < start + span; ++j)
                if (automatic[j])
                    sizes[j] = std::min(maxima[j], sizes[j] + extra);
            sum = std::accumulate(sizes.begin() + start, sizes.begin() + start + span, 0.0);
        }
    }
    if (std::isfinite(available)) {
        // Fractional tracks share remaining space while respecting minimums.
        std::vector<bool> frozen(count);
        for (std::size_t pass = 0; pass <= count; ++pass) {
            double fixed = gap * (count ? count - 1 : 0), weight = 0;
            for (std::size_t i = 0; i < count; ++i) {
                if (fractions[i] > 0 && !frozen[i])
                    weight += fractions[i];
                else
                    fixed += sizes[i];
            }
            if (weight == 0)
                break;
            const auto unit = std::max(0.0, available - fixed) / weight;
            bool again = false;
            for (std::size_t i = 0; i < count; ++i)
                if (fractions[i] > 0 && !frozen[i]) {
                    double target = unit * fractions[i];
                    if (target < sizes[i] || target > maxima[i]) {
                        sizes[i] = std::clamp(target, sizes[i], maxima[i]);
                        frozen[i] = true;
                        again = true;
                    }
                }
            if (!again) {
                for (std::size_t i = 0; i < count; ++i)
                    if (fractions[i] > 0 && !frozen[i])
                        sizes[i] = unit * fractions[i];
                break;
            }
        }
    }
    return sizes;
}
Size Host::Impl::measure(const std::shared_ptr<Node>& n, Size available) {
    const auto& d = n->d();
    if (!n->visible || d.kind == Kind::empty)
        return n->measured = {};
    // Parents ask for intrinsic and constrained sizes repeatedly during measurement and arrangement.
    // Cache proposals for this layout pass so nested grids do not multiply descendant work.
    if (n->measurement_epoch != measurement_epoch) {
        n->measurements.clear();
        n->measurement_epoch = measurement_epoch;
    }
    for (const auto& cached : n->measurements)
        if (cached.available.width == available.width && cached.available.height == available.height) {
            states.at(n->id).measured = cached.intrinsic;
            n->layout = cached.layout;
            return n->measured = cached.result;
        }
    if (d.box.width.kind == Dimension::Kind::unlimited || d.box.height.kind == Dimension::Kind::unlimited)
        throw std::invalid_argument("unbounded is only valid for maximum sizes");
    const auto pad = insets(d.box.padding, available.width, n->font);
    const double specified_w = resolve(d.box.width, available.width, n->font, infinity);
    const double specified_h = resolve(d.box.height, available.height, n->font, infinity);
    Size inner{std::max(0.0, std::min(available.width, specified_w) - pad.horizontal()),
               std::max(0.0, std::min(available.height, specified_h) - pad.vertical())};
    n->layout = d.responsive && std::isfinite(inner.width) &&
                        inner.width >= resolve(d.responsive->first, inner.width, n->font)
                    ? d.responsive->second
                    : d.layout;
    Size result;
    switch (d.kind) {
    case Kind::text: {
        double preferred = text_width(*n, d.text);
        if (d.box.width.kind == Dimension::Kind::minimum_content) {
            preferred = 0;
            std::string word;
            for (char c : d.text) {
                if (c == ' ' || c == '\n') {
                    preferred = std::max(preferred, text_width(*n, word));
                    word.clear();
                } else
                    word += c;
            }
            preferred = std::max(preferred, text_width(*n, word));
        }
        if (d.box.width.kind != Dimension::Kind::maximum_content)
            preferred = std::min(preferred, inner.width);
        const auto wrapped = lines(*n, preferred);
        result = {preferred, wrapped.size() * n->font * 1.35};
        break;
    }
    case Kind::button:
        result = {text_width(*n, d.text) + 28, std::max(36.0, n->font * 1.35 + 16)};
        break;
    case Kind::checkbox:
        result = {text_width(*n, d.text) + 32, std::max(32.0, n->font * 1.35 + 12)};
        break;
    case Kind::slider:
        result = {180, 36};
        break;
    case Kind::canvas:
        result = {160, 120};
        break;
    case Kind::image:
        if (d.image)
            result = {(double)d.image->width(), (double)d.image->height()};
        break;
    case Kind::scroll: {
        if (!n->children.empty()) {
            auto content = measure(n->children[0], {inner.width, infinity});
            result = {content.width, std::min(content.height, inner.height)};
        }
        break;
    }
    case Kind::component:
    case Kind::switcher:
        if (!n->children.empty())
            result = measure(n->children[0], inner);
        break;
    case Kind::container: {
        if (auto* flex = std::get_if<Flex>(&n->layout)) {
            const bool row = flex->direction == Direction::row;
            const double gap = resolve(flex->gap, inner.width, n->font);
            double main = 0, cross = 0, total_cross = 0, max_main = 0;
            std::size_t count = 0;
            for (auto& child : n->children) {
                if (!child->visible || child->d().kind == Kind::empty)
                    continue;
                auto m = measure(child, inner);
                if (empty_slot(child))
                    continue;
                auto margins = insets(child->d().box.margin, inner.width, child->font);
                const auto cm = (row ? m.width + margins.horizontal() : m.height + margins.vertical());
                const auto cc = (row ? m.height + margins.vertical() : m.width + margins.horizontal());
                const auto limit = row ? inner.width : inner.height;
                if (flex->wrap == Wrap::wrap && count && main + gap + cm > limit) {
                    max_main = std::max(max_main, main);
                    total_cross += cross + gap;
                    main = 0;
                    cross = 0;
                    count = 0;
                }
                main += (count++ ? gap : 0) + cm;
                cross = std::max(cross, cc);
            }
            main = std::max(max_main, main);
            cross += total_cross;
            result = row ? Size{main, cross} : Size{cross, main};
        } else if (auto* grid = std::get_if<GridLayout>(&n->layout)) {
            if (grid->columns.empty() || grid->columns.size() > 4096)
                throw std::invalid_argument("Grid needs 1 to 4096 columns");
            for (auto& child : n->children)
                measure(child, inner);
            std::size_t rows = grid->rows.size();
            auto positions = grid_positions(n, grid->columns.size(), rows);
            auto widths = tracks(n, true, inner.width, positions, grid->columns.size());
            const auto column_gap = resolve(grid->column_gap, inner.width, n->font),
                       row_gap = resolve(grid->row_gap, inner.height, n->font);
            for (std::size_t i = 0; i < n->children.size(); ++i)
                if (positions[i] != std::numeric_limits<std::size_t>::max()) {
                    auto start = positions[i] % widths.size(),
                         span = n->children[i]->d().item.grid.column_span;
                    double width =
                        std::accumulate(widths.begin() + start, widths.begin() + start + span, 0.0) +
                        column_gap * (span - 1);
                    measure(n->children[i], {width, inner.height});
                }
            auto heights = tracks(n, false, infinity, positions, rows);
            result = {std::accumulate(widths.begin(), widths.end(), 0.0) + column_gap * (widths.size() - 1),
                      std::accumulate(heights.begin(), heights.end(), 0.0) + row_gap * (rows ? rows - 1 : 0)};
        } else if (std::holds_alternative<OverlayLayout>(n->layout)) {
            for (auto& child : n->children) {
                auto m = measure(child, inner);
                result.width = std::max(result.width, m.width);
                result.height = std::max(result.height, m.height);
            }
        } else {
            auto algorithm = std::get<std::shared_ptr<const LayoutAlgorithm>>(n->layout);
            if (!algorithm)
                throw std::invalid_argument("Custom layout cannot be null");
            AlgorithmContext context;
            context.node = n;
            context.measure = [&](std::size_t i, Constraints c) {
                auto m = measure(n->children.at(i), c.maximum);
                return Size{std::clamp(m.width, c.minimum.width, c.maximum.width),
                            std::clamp(m.height, c.minimum.height, c.maximum.height)};
            };
            result = algorithm->measure(context, Constraints{.maximum = inner});
            if (!std::isfinite(result.width) || !std::isfinite(result.height) || result.width < 0 ||
                result.height < 0)
                throw std::invalid_argument("Custom layout returned an invalid size");
        }
        break;
    }
    default:
        break;
    }
    result.width += pad.horizontal();
    result.height += pad.vertical();
    result.width = resolve(d.box.width, available.width, n->font, result.width);
    result.height = resolve(d.box.height, available.height, n->font, result.height);
    if (d.box.aspect_ratio && *d.box.aspect_ratio > 0) {
        if (d.box.height.kind == Dimension::Kind::automatic)
            result.height = result.width / *d.box.aspect_ratio;
        else if (d.box.width.kind == Dimension::Kind::automatic)
            result.width = result.height * *d.box.aspect_ratio;
    }
    auto clamp_dimension = [&](double value, Dimension minimum, Dimension maximum, double available) {
        auto low = resolve(minimum, available, n->font, 0),
             high = resolve(maximum, available, n->font, infinity);
        return std::clamp(finite(value), low, std::max(low, high));
    };
    result.width = clamp_dimension(result.width, d.box.min_width, d.box.max_width, available.width);
    result.height = clamp_dimension(result.height, d.box.min_height, d.box.max_height, available.height);
    auto& p = states.at(n->id);
    p.measured = result;
    if (p.active && d.size_transition == SizeTransition::animate) {
        const auto t = std::clamp(curve(p.animation, progress(p)), 0.0, 1.0);
        if (d.box.width.kind == Dimension::Kind::automatic)
            result.width = lerp(p.from_size.width, result.width, t);
        if (d.box.height.kind == Dimension::Kind::automatic)
            result.height = lerp(p.from_size.height, result.height, t);
    }
    // Custom strategies may propose arbitrarily many constraints. Bound retained cache storage.
    if (n->measurements.size() == 16)
        n->measurements.erase(n->measurements.begin());
    n->measurements.push_back({available, result, p.measured, n->layout});
    return n->measured = result;
}
void Host::Impl::arrange(const std::shared_ptr<Node>& n, Rect bounds, Rect clip) {
    const auto& d = n->d();
    auto& persistent = states.at(n->id);
    if (!n->visible || d.kind == Kind::empty) {
        n->bounds = {};
        n->clip = {};
        return;
    }
    if (persistent.layout_active) {
        const double t = curve(persistent.animation, progress(persistent));
        const auto from = persistent.from_bounds;
        bounds = {lerp(from.x, bounds.x, t), lerp(from.y, bounds.y, t), lerp(from.width, bounds.width, t),
                  lerp(from.height, bounds.height, t)};
        if (progress(persistent) >= 1)
            persistent.layout_active = false;
    }
    n->bounds = bounds;
    persistent.bounds = bounds;
    n->clip = clip;
    const auto inner = inset(bounds, insets(d.box.padding, bounds.width, n->font));
    const auto child_clip = d.clip || d.kind == Kind::scroll ? intersect(clip, inner) : clip;
    if (d.kind == Kind::scroll && !n->children.empty()) {
        auto measured = measure(n->children[0], {inner.width, infinity});
        persistent.scroll_max = std::max(0.0, measured.height - inner.height);
        persistent.scroll = std::clamp(persistent.scroll, 0.0, persistent.scroll_max);
        arrange(n->children[0],
                {inner.x, inner.y - persistent.scroll, inner.width, std::max(inner.height, measured.height)},
                child_clip);
        return;
    }
    if ((d.kind == Kind::switcher || d.kind == Kind::component) && !n->children.empty()) {
        arrange(n->children[0], inner, intersect(child_clip, inner));
        return;
    }
    if (d.kind != Kind::container)
        return;
    // Resolve responsive policy from the assigned width, not the container's intrinsic width.
    n->layout = d.responsive && inner.width >= resolve(d.responsive->first, inner.width, n->font)
                    ? d.responsive->second
                    : d.layout;
    if (auto* flex = std::get_if<Flex>(&n->layout)) {
        const bool row = flex->direction == Direction::row;
        double capacity = row ? inner.width : inner.height;
        const double gap = std::max(0.0, resolve(flex->gap, inner.width, n->font));
        struct Item {
            std::shared_ptr<Node> node;
            Insets margin;
            double main, cross, low, high;
        };
        std::vector<std::vector<Item>> lines(1);
        double used = 0;
        for (auto& child : n->children) {
            if (!child->visible || child->d().kind == Kind::empty)
                continue;
            const auto m = measure(child, {inner.width, inner.height});
            if (empty_slot(child)) {
                arrange(child, {inner.x, inner.y, 0, 0}, child_clip);
                continue;
            }
            const auto& box = child->d().box;
            auto margin = insets(box.margin, inner.width, child->font);
            double base =
                resolve(child->d().item.flex.basis, capacity, child->font, row ? m.width : m.height);
            double low = resolve(row ? box.min_width : box.min_height, capacity, child->font, 0);
            double high =
                std::max(low, resolve(row ? box.max_width : box.max_height, capacity, child->font, infinity));
            base = std::clamp(base, low, high);
            const double outer = base + (row ? margin.horizontal() : margin.vertical());
            if (flex->wrap == Wrap::wrap && !lines.back().empty() && used + gap + outer > capacity) {
                lines.emplace_back();
                used = 0;
            }
            used += (lines.back().empty() ? 0 : gap) + outer;
            lines.back().push_back({child, margin, base, row ? m.height : m.width, low, high});
        }
        double cross_cursor = row ? inner.y : inner.x;
        for (auto& line : lines) {
            if (line.empty())
                continue;
            auto sum = [&] {
                double n = gap * (line.size() - 1);
                for (auto& i : line)
                    n += i.main + (row ? i.margin.horizontal() : i.margin.vertical());
                return n;
            };
            std::vector<bool> frozen(line.size());
            for (std::size_t pass = 0; pass <= line.size(); ++pass) {
                double free = capacity - sum();
                if (std::abs(free) < .01)
                    break;
                double weights = 0;
                for (std::size_t i = 0; i < line.size(); ++i)
                    if (!frozen[i])
                        weights += free > 0 ? line[i].node->d().item.flex.grow
                                            : line[i].node->d().item.flex.shrink * line[i].main;
                if (weights <= 0)
                    break;
                bool clamped = false;
                for (std::size_t i = 0; i < line.size(); ++i)
                    if (!frozen[i]) {
                        auto& item = line[i];
                        double weight = free > 0 ? item.node->d().item.flex.grow
                                                 : item.node->d().item.flex.shrink * item.main;
                        double proposed = item.main + free * weight / weights;
                        double next = std::clamp(proposed, item.low, item.high);
                        if (std::abs(next - proposed) > .001) {
                            frozen[i] = true;
                            clamped = true;
                        }
                        item.main = next;
                    }
                if (!clamped)
                    break;
            }
            double cross = 0, line_baseline = 0;
            for (auto& item : line) {
                // Constrained width determines wrapped text height and baseline placement.
                if (row) {
                    auto m = measure(item.node, {item.main, inner.height});
                    item.cross = m.height;
                }
                const auto& box = item.node->d().box;
                item.cross = bounded(item.cross, row ? box.min_height : box.min_width,
                                     row ? box.max_height : box.max_width, row ? inner.height : inner.width,
                                     item.node->font);
                cross =
                    std::max(cross, item.cross + (row ? item.margin.vertical() : item.margin.horizontal()));
                line_baseline = std::max(line_baseline, baseline(item.node, item.cross) +
                                                            (row ? item.margin.top : item.margin.left));
            }
            if (lines.size() == 1)
                cross = row ? inner.height : inner.width;
            double free = std::max(0.0, capacity - sum()), spacing = gap, offset = 0;
            switch (flex->justify) {
            case Justify::center:
                offset = free / 2;
                break;
            case Justify::end:
                offset = free;
                break;
            case Justify::space_between:
                if (line.size() > 1)
                    spacing += free / (line.size() - 1);
                break;
            case Justify::space_around:
                spacing += free / line.size();
                offset = free / line.size() / 2;
                break;
            default:
                break;
            }
            double cursor = (row ? inner.x : inner.y) + offset;
            for (auto& item : line) {
                auto alignment = item.node->d().item.flex.align_self.value_or(flex->align);
                const double before = row ? item.margin.left : item.margin.top,
                             after = row ? item.margin.right : item.margin.bottom;
                const double cross_before = row ? item.margin.top : item.margin.left,
                             cross_after = row ? item.margin.bottom : item.margin.right;
                double cross_size = item.cross;
                const auto& box = item.node->d().box;
                const bool auto_cross = (row ? box.height : box.width).kind == Dimension::Kind::automatic;
                if (alignment == Align::stretch && auto_cross)
                    cross_size = std::max(0.0, cross - cross_before - cross_after);
                cross_size = bounded(cross_size, row ? box.min_height : box.min_width,
                                     row ? box.max_height : box.max_width, cross, item.node->font);
                double cross_offset = cross_before;
                if (alignment == Align::center)
                    cross_offset += (cross - cross_before - cross_after - cross_size) / 2;
                if (alignment == Align::end)
                    cross_offset = cross - cross_after - cross_size;
                if (alignment == Align::baseline && row)
                    cross_offset = line_baseline - baseline(item.node, cross_size);
                Rect target = row ? Rect{cursor + before, cross_cursor + cross_offset, item.main, cross_size}
                                  : Rect{cross_cursor + cross_offset, cursor + before, cross_size, item.main};
                arrange(item.node, target, child_clip);
                cursor += before + item.main + after + spacing;
            }
            cross_cursor += cross + gap;
        }
    } else if (auto* grid = std::get_if<GridLayout>(&n->layout)) {
        if (grid->columns.empty())
            throw std::invalid_argument("Grid needs columns");
        for (auto& child : n->children)
            measure(child, {inner.width, inner.height});
        std::size_t rows = grid->rows.size();
        const auto positions = grid_positions(n, grid->columns.size(), rows);
        const double cg = resolve(grid->column_gap, inner.width, n->font),
                     rg = resolve(grid->row_gap, inner.height, n->font);
        auto widths = tracks(n, true, inner.width, positions, grid->columns.size());
        for (std::size_t i = 0; i < n->children.size(); ++i)
            if (positions[i] != std::numeric_limits<std::size_t>::max()) {
                auto start = positions[i] % widths.size(), span = n->children[i]->d().item.grid.column_span;
                const auto width =
                    std::accumulate(widths.begin() + start, widths.begin() + start + span, 0.0) +
                    cg * (span - 1);
                measure(n->children[i], {width, inner.height});
            }
        auto heights = tracks(n, false, inner.height, positions, rows);
        for (std::size_t i = 0; i < n->children.size(); ++i) {
            auto child = n->children[i];
            if (positions[i] == std::numeric_limits<std::size_t>::max()) {
                arrange(child, {inner.x, inner.y, 0, 0}, child_clip);
                continue;
            }
            const auto& p = child->d().item.grid;
            std::size_t col = positions[i] % widths.size(), row = positions[i] / widths.size();
            Rect cell{inner.x + std::accumulate(widths.begin(), widths.begin() + col, 0.0) + cg * col,
                      inner.y + std::accumulate(heights.begin(), heights.begin() + row, 0.0) + rg * row,
                      std::accumulate(widths.begin() + col, widths.begin() + col + p.column_span, 0.0) +
                          cg * (p.column_span - 1),
                      std::accumulate(heights.begin() + row, heights.begin() + row + p.row_span, 0.0) +
                          rg * (p.row_span - 1)};
            cell = inset(cell, insets(child->d().box.margin, inner.width, child->font));
            auto m = measure(child, {cell.width, cell.height});
            auto align_axis = [](double origin, double space, double wanted, Align align) {
                return origin + (align == Align::center ? (space - wanted) / 2
                                 : align == Align::end  ? space - wanted
                                                        : 0);
            };
            const auto& box = child->d().box;
            double w = p.horizontal == Align::stretch && box.width.kind == Dimension::Kind::automatic
                           ? cell.width
                           : std::min(cell.width, m.width);
            double h = p.vertical == Align::stretch && box.height.kind == Dimension::Kind::automatic
                           ? cell.height
                           : std::min(cell.height, m.height);
            auto low_w = resolve(box.min_width, cell.width, child->font, 0),
                 low_h = resolve(box.min_height, cell.height, child->font, 0);
            w = std::clamp(w, low_w,
                           std::max(low_w, resolve(box.max_width, cell.width, child->font, infinity)));
            h = std::clamp(h, low_h,
                           std::max(low_h, resolve(box.max_height, cell.height, child->font, infinity)));
            arrange(child,
                    {align_axis(cell.x, cell.width, w, p.horizontal),
                     align_axis(cell.y, cell.height, h, p.vertical), w, h},
                    child_clip);
        }
    } else if (std::holds_alternative<OverlayLayout>(n->layout)) {
        const auto overlay = std::get<OverlayLayout>(n->layout);
        for (auto& child : n->children) {
            auto m = measure(child, {inner.width, inner.height});
            auto style = child->d().item.overlay;
            auto x = style.horizontal == Align::center ? overlay.horizontal : style.horizontal;
            auto y = style.vertical == Align::center ? overlay.vertical : style.vertical;
            double w = x == Align::stretch ? inner.width : std::min(inner.width, m.width),
                   h = y == Align::stretch ? inner.height : std::min(inner.height, m.height);
            arrange(child,
                    {inner.x +
                         (x == Align::center ? (inner.width - w) / 2
                          : x == Align::end  ? inner.width - w
                                             : 0) +
                         style.offset.x,
                     inner.y +
                         (y == Align::center ? (inner.height - h) / 2
                          : y == Align::end  ? inner.height - h
                                             : 0) +
                         style.offset.y,
                     w, h},
                    child_clip);
        }
    } else {
        auto algorithm = std::get<std::shared_ptr<const LayoutAlgorithm>>(n->layout);
        if (!algorithm)
            throw std::invalid_argument("Custom layout cannot be null");
        AlgorithmContext context;
        context.node = n;
        context.measure = [&](std::size_t i, Constraints c) {
            auto m = measure(n->children.at(i), c.maximum);
            return Size{std::clamp(m.width, c.minimum.width, c.maximum.width),
                        std::clamp(m.height, c.minimum.height, c.maximum.height)};
        };
        context.place = [&](std::size_t i, Rect r) {
            if (!std::isfinite(r.x) || !std::isfinite(r.y) || !std::isfinite(r.width) ||
                !std::isfinite(r.height) || r.width < 0 || r.height < 0)
                throw std::invalid_argument("Custom layout returned an invalid rectangle");
            arrange(n->children.at(i), r, child_clip);
        };
        algorithm->arrange(context, inner);
    }
}
void Host::Impl::perform_layout() {
    if (dirty)
        rebuild();
    if (!layout_dirty && laid_out_size.width == viewport.width && laid_out_size.height == viewport.height)
        return;
    if (root) {
        ++measurement_epoch;
        measure(root, viewport);
        arrange(root, {0, 0, viewport.width, viewport.height}, {0, 0, viewport.width, viewport.height});
    }
    laid_out_size = viewport;
    layout_dirty = false;
}

namespace {
void rounded(SkCanvas& canvas, Rect rect, std::uint32_t color, double radius = 0, bool stroke = false,
             double width = 1) {
    SkPaint p;
    p.setAntiAlias(true);
    p.setColor(color);
    if (stroke) {
        p.setStyle(SkPaint::kStroke_Style);
        p.setStrokeWidth((float)width);
    }
    canvas.drawRoundRect(skrect(rect), (float)radius, (float)radius, p);
}
std::uint32_t alpha(std::uint32_t color, double value) {
    return (color & 0x00ffffffu) | (std::uint32_t(std::clamp(value, 0.0, 1.0) * 255) << 24);
}
} // namespace
void Host::Impl::picture(SkCanvas& canvas, const sk_sp<SkPicture>& image, Rect bounds,
                         TransitionEffect effect, double opacity, SkBlendMode blend) {
    if (!image || opacity <= 0)
        return;
    SkAutoCanvasRestore restore(&canvas, true);
    canvas.translate((float)(bounds.x + effect.x * bounds.width),
                     (float)(bounds.y + effect.y * bounds.height));
    canvas.translate((float)bounds.width / 2, (float)bounds.height / 2);
    canvas.scale((float)effect.scale, (float)effect.scale);
    canvas.translate((float)-bounds.width / 2, (float)-bounds.height / 2);
    SkPaint p;
    p.setAlphaf((float)std::clamp(opacity * effect.opacity, 0.0, 1.0));
    p.setBlendMode(blend);
    canvas.drawPicture(image, nullptr, &p);
}
void Host::Impl::paint_contents(SkCanvas& canvas, const std::shared_ptr<Node>& n) {
    const auto& d = n->d();
    const auto b = n->bounds;
    const auto& style = states.at(n->id);
    if (style.background >> 24)
        rounded(canvas, b, style.background, d.radius);
    auto inner = inset(b, insets(d.box.padding, b.width, n->font));
    const bool pressed = captured == n->id || keyboard_pressed == n->id;
    auto draw_text = [&](std::string_view value, double x, double baseline, std::uint32_t color) {
        SkPaint p;
        p.setAntiAlias(true);
        p.setColor(color);
        auto f = font(*n, true);
        canvas.drawSimpleText(value.data(), value.size(), SkTextEncoding::kUTF8, (float)x, (float)baseline, f,
                              p);
    };
    switch (d.kind) {
    case Kind::text: {
        const auto wrapped = lines(*n, inner.width);
        double baseline = inner.y + n->font;
        for (auto& line : wrapped) {
            draw_text(line, inner.x, baseline, style.color);
            baseline += n->font * 1.35;
        }
        break;
    }
    case Kind::button: {
        auto color = style.background;
        if (pressed)
            color = alpha(theme.accent, .35);
        else if (hovered == n->id)
            color = alpha(theme.accent, .18);
        rounded(canvas, inner, color, d.radius);
        rounded(canvas, inner, theme.border, d.radius, true);
        draw_text(d.text, inner.x + (inner.width - text_width(*n, d.text)) / 2,
                  inner.y + (inner.height - n->font * 1.35) / 2 + n->font, style.color);
        break;
    }
    case Kind::checkbox: {
        Rect box{inner.x + 2, inner.y + (inner.height - 19) / 2, 19, 19};
        rounded(canvas, box, d.value ? theme.accent : theme.surface, 5);
        rounded(canvas, box, d.value ? theme.accent : theme.border, 5, true);
        if (d.value) {
            SkPaint p;
            p.setAntiAlias(true);
            p.setStrokeWidth(2);
            p.setColor(theme.background);
            p.setStyle(SkPaint::kStroke_Style);
            SkPathBuilder path;
            path.moveTo((float)box.x + 4, (float)box.y + 10)
                .lineTo((float)box.x + 8, (float)box.y + 14)
                .lineTo((float)box.x + 15, (float)box.y + 5);
            canvas.drawPath(path.detach(), p);
        }
        draw_text(d.text, inner.x + 31, inner.y + (inner.height - n->font * 1.35) / 2 + n->font, style.color);
        break;
    }
    case Kind::slider: {
        const double y = inner.y + inner.height / 2, width = std::max(0.0, inner.width - 24);
        const double value = std::clamp((d.value - d.minimum) / (d.maximum - d.minimum), 0.0, 1.0);
        rounded(canvas, {inner.x + 12, y - 3, width, 6}, theme.border, 3);
        rounded(canvas, {inner.x + 12, y - 3, width * value, 6}, theme.accent, 3);
        SkPaint p;
        p.setAntiAlias(true);
        p.setColor(pressed ? theme.foreground : theme.accent);
        canvas.drawCircle((float)(inner.x + 12 + width * value), (float)y, 8, p);
        break;
    }
    case Kind::canvas: {
        if (d.draw) {
            SkAutoCanvasRestore restore(&canvas, true);
            canvas.clipRect(skrect(inner));
            canvas.translate((float)inner.x, (float)inner.y);
            auto callback = d.draw;
            Collect collect(observer);
            callback(canvas, PaintInfo{{inner.width, inner.height}, frame.scale, frame.elapsed, frame.delta});
        }
        break;
    }
    case Kind::image:
        if (d.image)
            canvas.drawImageRect(d.image, skrect(inner), SkSamplingOptions(SkFilterMode::kLinear));
        break;
    default:
        for (auto& child : n->children)
            paint_node(canvas, child);
        break;
    }
    if (focused == n->id && window_focused && focusable(d.kind))
        rounded(canvas, {b.x + 1, b.y + 1, std::max(0.0, b.width - 2), std::max(0.0, b.height - 2)},
                theme.accent, d.radius, true, 2);
    if (d.kind == Kind::scroll) {
        const auto& state = states.at(n->id);
        if (state.scroll_max > 0) {
            double h = std::max(22.0, inner.height * inner.height / (inner.height + state.scroll_max));
            rounded(canvas,
                    {inner.x + inner.width - 5,
                     inner.y + (inner.height - h) * state.scroll / state.scroll_max, 3, h},
                    alpha(theme.muted, .65), 2);
        }
    }
}
void Host::Impl::paint_node(SkCanvas& canvas, const std::shared_ptr<Node>& n, bool record) {
    if (!n->visible || n->d().kind == Kind::empty)
        return;
    // Immediate layout collapse must not discard the exiting visual. It has no live input subtree.
    if (n->bounds.width <= 0 || n->bounds.height <= 0) {
        auto& p = states.at(n->id);
        if (p.active && p.outgoing && p.from_bounds.width > 0 && p.from_bounds.height > 0) {
            const auto t = curve(p.animation, progress(p));
            const auto effect = p.transition.leave;
            TransitionEffect leaving{lerp(1, effect.opacity, t), std::max(.001, lerp(1, effect.scale, t)),
                                     effect.x * t, effect.y * t};
            SkPictureRecorder recorder;
            auto* out = recorder.beginRecording((float)p.from_bounds.width, (float)p.from_bounds.height);
            picture(*out, p.outgoing, {0, 0, p.from_bounds.width, p.from_bounds.height}, leaving, 1);
            p.presented = recorder.finishRecordingAsPicture();
            SkAutoCanvasRestore restore(&canvas, true);
            canvas.clipRect(skrect(n->clip));
            picture(canvas, p.presented, p.from_bounds, {}, p.opacity * (n->enabled ? 1 : .45));
        } else {
            // A non-clipping ancestor may also have collapsed; retiring descendants still paint.
            for (const auto& child : n->children)
                paint_node(canvas, child);
        }
        return;
    }
    if (intersect(n->bounds, n->clip).width <= 0 || intersect(n->bounds, n->clip).height <= 0)
        return;
    auto& p = states.at(n->id);
    const auto& d = n->d();
    SkAutoCanvasRestore restore(&canvas, true);
    canvas.clipRect(skrect(n->clip));
    if (focusable(d.kind) || d.kind == Kind::text || d.kind == Kind::image || d.kind == Kind::switcher)
        canvas.clipRect(skrect(n->bounds));
    const double opacity = p.opacity * (n->enabled ? 1 : .45);
    if (opacity < 1) {
        SkPaint paint;
        paint.setAlphaf((float)opacity);
        canvas.saveLayer(nullptr, &paint);
    }
    if (record && (d.transition.enabled || d.content_transition != ContentTransition::none ||
                   d.kind == Kind::switcher)) {
        SkPictureRecorder incoming_recorder;
        auto* c = incoming_recorder.beginRecording((float)n->bounds.width, (float)n->bounds.height);
        c->translate((float)-n->bounds.x, (float)-n->bounds.y);
        paint_contents(*c, n);
        auto incoming = incoming_recorder.finishRecordingAsPicture();
        const double raw = progress(p), t = curve(p.animation, raw);
        if (p.active && raw >= 1) {
            p.active = false;
            p.outgoing.reset();
        }
        if (p.active) {
            SkPictureRecorder composite_recorder;
            auto* out = composite_recorder.beginRecording((float)n->bounds.width, (float)n->bounds.height);
            auto interpolate = [](TransitionEffect effect, double amount) {
                return TransitionEffect{lerp(effect.opacity, 1, amount),
                                        std::max(.001, lerp(effect.scale, 1, amount)),
                                        effect.x * (1 - amount), effect.y * (1 - amount)};
            };
            auto enter = interpolate(p.transition.enter, t), leave = interpolate(p.transition.leave, 1 - t);
            if (p.content_transition == ContentTransition::numeric_text) {
                enter.y = (1 - t) * .7;
                leave.y = -t * .7;
            }
            Rect local{0, 0, n->bounds.width, n->bounds.height};
            picture(*out, p.outgoing, local, leave, 1);
            picture(*out, incoming, local, enter, 1,
                    (p.transition.enter.opacity < 1 && p.transition.leave.opacity < 1)
                        ? SkBlendMode::kPlus
                        : SkBlendMode::kSrcOver);
            p.presented = composite_recorder.finishRecordingAsPicture();
        } else
            p.presented = std::move(incoming);
        picture(canvas, p.presented, n->bounds, {}, 1);
    } else
        paint_contents(canvas, n);
}
bool Host::Impl::busy() const {
    if (dirty || repaint)
        return true;
    for (const auto& [_, p] : states)
        if (p.active || p.layout_active || p.property_active)
            return true;
    if (!ghosts.empty())
        return true;
    for (const auto& n : order)
        if (n->visible && n->d().animating)
            return true;
    return false;
}
Point Host::Impl::point_for(const std::shared_ptr<Node>& node, Point point) const {
    std::vector<std::shared_ptr<Node>> chain;
    for (auto n = node; n; n = n->parent_node.lock())
        chain.push_back(n);
    for (auto it = chain.rbegin(); it != chain.rend(); ++it) {
        auto n = *it;
        const auto& p = states.at(n->id);
        if (!p.active)
            continue;
        const auto t = curve(p.animation, progress(p));
        const auto effect = p.transition.enter;
        const auto s = std::max(.001, lerp(effect.scale, 1, t));
        auto center = Point{n->bounds.x + n->bounds.width / 2, n->bounds.y + n->bounds.height / 2};
        point.x = (point.x - center.x - effect.x * (1 - t) * n->bounds.width) / s + center.x;
        point.y = (point.y - center.y - effect.y * (1 - t) * n->bounds.height) / s + center.y;
    }
    return point;
}
Rect Host::Impl::presentation_bounds(const std::shared_ptr<Node>& node) const {
    Rect rect = node->bounds;
    for (auto n = node; n; n = n->parent_node.lock()) {
        const auto& p = states.at(n->id);
        if (!p.active)
            continue;
        const auto t = curve(p.animation, progress(p));
        const auto e = p.transition.enter;
        const auto s = std::max(.001, lerp(e.scale, 1, t));
        const Point center{n->bounds.x + n->bounds.width / 2, n->bounds.y + n->bounds.height / 2};
        rect = {center.x + (rect.x - center.x) * s + e.x * (1 - t) * n->bounds.width,
                center.y + (rect.y - center.y) * s + e.y * (1 - t) * n->bounds.height, rect.width * s,
                rect.height * s};
    }
    return rect;
}
void Host::Impl::sample_properties() {
    auto color = [](std::uint32_t a, std::uint32_t b, double t) {
        std::uint32_t result = 0;
        for (int shift : {0, 8, 16, 24})
            result |= std::uint32_t(std::clamp(lerp((a >> shift) & 255, (b >> shift) & 255, t), 0.0, 255.0))
                      << shift;
        return result;
    };
    for (auto& [_, p] : states)
        if (p.property_active) {
            double raw = progress(p), t = curve(p.animation, raw);
            p.opacity = std::clamp(lerp(p.from_opacity, p.target_opacity, t), 0.0, 1.0);
            p.font = std::max(1.0, lerp(p.from_font, p.target_font, t));
            p.color = color(p.from_color, p.target_color, t);
            p.background = color(p.from_background, p.target_background, t);
            if (raw >= 1)
                p.property_active = false;
        }
}
std::shared_ptr<Node> Host::Impl::hit(Point point) const {
    for (auto it = order.rbegin(); it != order.rend(); ++it) {
        auto n = *it;
        if (!n->visible || !n->clip.contains(point))
            continue;
        if (n->bounds.contains(point_for(n, point)) &&
            (focusable(n->d().kind) || n->d().kind == Kind::scroll || n->d().kind == Kind::canvas))
            return n;
    }
    return {};
}
bool Host::Impl::focus(const std::string& id) {
    auto it = index.find(id);
    if (it == index.end() || !it->second->enabled || !it->second->visible || !focusable(it->second->d().kind))
        return false;
    focused = id;
    keyboard_pressed.clear();
    auto node = it->second;
    // Reveal a tab-focused child in any containing scroll view.
    for (const auto& ancestor : order)
        if (ancestor->d().kind == Kind::scroll && node->id.starts_with(ancestor->id + "/")) {
            auto& p = states.at(ancestor->id);
            const auto previous = p.scroll;
            auto clip = ancestor->bounds;
            if (node->bounds.y < clip.y)
                p.scroll += node->bounds.y - clip.y;
            if (node->bounds.y + node->bounds.height > clip.y + clip.height)
                p.scroll += node->bounds.y + node->bounds.height - clip.y - clip.height;
            p.scroll = std::clamp(p.scroll, 0.0, p.scroll_max);
            layout_dirty |= p.scroll != previous;
        }
    request_paint();
    return true;
}
bool Host::Impl::activate(const std::string& id) {
    auto it = index.find(id);
    if (it == index.end())
        return false;
    auto n = it->second;
    if (!n->enabled || !n->visible)
        return false;
    if (n->d().kind == Kind::button) {
        auto callback = n->d().click;
        if (callback)
            callback();
        return true;
    }
    if (n->d().kind == Kind::checkbox) {
        auto callback = n->d().change;
        const auto value = n->d().value;
        if (callback)
            callback(!value);
        return true;
    }
    return false;
}
bool Host::Impl::set_value(const std::string& id, double value) {
    if (!std::isfinite(value))
        return false;
    auto it = index.find(id);
    if (it == index.end())
        return false;
    auto n = it->second;
    if (!n->enabled || !n->visible)
        return false;
    const auto& d = n->d();
    if (d.kind == Kind::checkbox) {
        auto callback = d.change;
        if (callback)
            callback(value != 0);
        return true;
    }
    if (d.kind != Kind::slider)
        return false;
    const auto clamped = std::clamp(value, d.minimum, d.maximum);
    const auto snapped =
        std::clamp(d.minimum + std::round((clamped - d.minimum) / d.step) * d.step, d.minimum, d.maximum);
    auto callback = d.change;
    if (callback)
        callback(snapped);
    return true;
}
bool Host::Impl::adjust(const std::string& id, double steps) {
    if (!std::isfinite(steps))
        return false;
    auto it = index.find(id);
    if (it == index.end() || it->second->d().kind != Kind::slider)
        return false;
    auto n = it->second;
    return set_value(id, n->d().value + steps * n->d().step);
}
void Host::Impl::slider_pointer(const std::shared_ptr<Node>& n, double x) {
    auto inner = inset(n->bounds, insets(n->d().box.padding, n->bounds.width, n->font));
    const auto& d = n->d();
    double raw = d.minimum + (d.maximum - d.minimum) *
                                 std::clamp((x - inner.x - 12) / std::max(1.0, inner.width - 24), 0.0, 1.0);
    double snapped = d.minimum + std::round((raw - d.minimum) / d.step) * d.step;
    auto callback = d.change;
    if (callback)
        callback(std::clamp(snapped, d.minimum, d.maximum));
}
bool Host::Impl::handle(const Event& event) {
    if (auto pointer = std::get_if<PointerEvent>(&event);
        pointer && (!std::isfinite(pointer->position.x) || !std::isfinite(pointer->position.y) ||
                    !std::isfinite(pointer->scroll_x) || !std::isfinite(pointer->scroll_y)))
        return false;
    if (auto* resize = std::get_if<ResizeEvent>(&event)) {
        viewport = resize->logical_size;
        perform_layout();
        request_paint();
        return false;
    }
    perform_layout();
    if (auto* focus_event = std::get_if<FocusEvent>(&event)) {
        window_focused = focus_event->focused;
        if (!window_focused) {
            captured.clear();
            keyboard_pressed.clear();
        }
        request_paint();
        return false;
    }
    if (auto* pointer = std::get_if<PointerEvent>(&event)) {
        auto target = hit(pointer->position);
        auto next_hover = pointer->kind == PointerKind::leave ? ""
                          : target && target->enabled         ? target->id
                                                              : "";
        if (hovered != next_hover) {
            hovered = std::move(next_hover);
            request_paint();
        }
        // Disabled controls still occlude input; they must not click through to legacy content.
        if (target && !target->enabled)
            return true;
        if (pointer->kind == PointerKind::scroll) {
            // Innermost scroll region consumes first; an exhausted region yields to its parent.
            for (auto it = order.rbegin(); it != order.rend(); ++it) {
                auto n = *it;
                if (!n->visible || !n->enabled || n->d().kind != Kind::scroll ||
                    !n->bounds.contains(pointer->position) || !n->clip.contains(pointer->position))
                    continue;
                auto& p = states.at(n->id);
                auto next = std::clamp(p.scroll - pointer->scroll_y, 0.0, p.scroll_max);
                if (next != p.scroll) {
                    p.scroll = next;
                    layout_dirty = true;
                    request_paint();
                    return true;
                }
            }
            return false;
        }
        if (pointer->kind == PointerKind::down && pointer->button == 0 && target &&
            focusable(target->d().kind)) {
            focus(target->id);
            captured = target->id;
            if (target->d().kind == Kind::slider)
                slider_pointer(target, point_for(target, pointer->position).x);
            request_paint();
            return true;
        }
        if (pointer->kind == PointerKind::move && !captured.empty()) {
            auto it = index.find(captured);
            if (it != index.end() && it->second->d().kind == Kind::slider)
                slider_pointer(it->second, point_for(it->second, pointer->position).x);
            return true;
        }
        if (pointer->kind == PointerKind::up && pointer->button == 0 && !captured.empty()) {
            const auto id = std::exchange(captured, {});
            request_paint();
            if (target && target->id == id)
                activate(id);
            return true;
        }
        return false;
    }
    if (auto* key = std::get_if<KeyEvent>(&event)) {
        if (key->modifiers.command || key->modifiers.control || key->modifiers.alt)
            return false;
        if (key->key == Key::tab && key->down) {
            std::vector<std::string> candidates;
            for (auto& n : order)
                if (n->enabled && n->visible && focusable(n->d().kind))
                    candidates.push_back(n->id);
            if (candidates.empty())
                return false;
            auto it = std::find(candidates.begin(), candidates.end(), focused);
            auto position =
                it == candidates.end() ? (key->modifiers.shift ? 0 : -1) : int(it - candidates.begin());
            position = (position + (key->modifiers.shift ? -1 : 1) + int(candidates.size())) %
                       int(candidates.size());
            return focus(candidates[position]);
        }
        auto it = index.find(focused);
        if (it == index.end())
            return false;
        auto n = it->second;
        if (key->down && n->d().kind == Kind::slider) {
            if (key->key == Key::left || key->key == Key::down)
                return adjust(focused, -1);
            if (key->key == Key::right || key->key == Key::up)
                return adjust(focused, 1);
            if (key->key == Key::home || key->key == Key::end) {
                auto callback = n->d().change;
                if (callback)
                    callback(key->key == Key::home ? n->d().minimum : n->d().maximum);
                return true;
            }
        }
        if (key->key == Key::enter && key->down && !key->repeat)
            return activate(focused);
        if (key->key == Key::space && focusable(n->d().kind) && n->d().kind != Kind::slider) {
            if (key->down) {
                if (!key->repeat)
                    keyboard_pressed = focused;
            } else {
                const auto id = std::exchange(keyboard_pressed, {});
                request_paint();
                if (id == focused)
                    activate(id);
            }
            request_paint();
            return true;
        }
    }
    return false;
}
std::vector<ElementInfo> Host::Impl::elements() const {
    std::vector<ElementInfo> result;
    for (const auto& n : order) {
        const auto& d = n->d();
        if (!n->visible || !semantic(d.kind))
            continue;
        auto role = d.kind == Kind::text       ? Role::text
                    : d.kind == Kind::button   ? Role::button
                    : d.kind == Kind::checkbox ? Role::checkbox
                    : d.kind == Kind::slider   ? Role::slider
                    : d.kind == Kind::image    ? Role::image
                                               : Role::canvas;
        result.push_back({n->id, d.key, d.label.empty() ? d.text : d.label, role, presentation_bounds(n),
                          n->clip, n->enabled, n->id == focused, d.kind == Kind::checkbox && d.value != 0,
                          d.value, d.minimum, d.maximum});
    }
    return result;
}
Host::Host(std::function<View()> builder, std::function<void()> wake, Theme theme)
    : impl_(std::make_shared<Impl>()) {
    if (!builder)
        throw std::invalid_argument("Host needs a view builder");
    impl_->builder = std::move(builder);
    impl_->wake = std::move(wake);
    impl_->theme = std::move(theme);
    if (!impl_->theme.typeface)
        impl_->theme.typeface = skia_window::detail::default_typeface();
    impl_->observer = std::make_shared<detail::Observer>();
    std::weak_ptr<Impl> weak = impl_;
    impl_->observer->changed = [weak](Animation animation) {
        if (auto p = weak.lock())
            p->invalidate(animation);
    };
}
Host::~Host() = default;
void Host::set_theme(Theme theme) {
    auto p = impl_;
    p->check();
    if (!theme.typeface)
        theme.typeface = skia_window::detail::default_typeface();
    p->theme = std::move(theme);
    p->invalidate();
}
void Host::set_reduced_motion(bool value) {
    auto p = impl_;
    p->check();
    p->user_reduced = value;
    const bool reduced = value || p->system_reduced;
    if (p->reduced == reduced)
        return;
    p->reduced = reduced;
    if (reduced) {
        for (auto& [_, s] : p->states) {
            s.active = s.layout_active = s.property_active = false;
            s.outgoing.reset();
            s.opacity = s.target_opacity;
            s.font = s.target_font;
            s.color = s.target_color;
            s.background = s.target_background;
        }
        p->ghosts.clear();
    }
    p->invalidate();
}
void Host::set_system_reduced_motion(bool value) {
    auto p = impl_;
    p->check();
    p->system_reduced = value;
    set_reduced_motion(p->user_reduced);
}
void Host::invalidate() {
    auto p = impl_;
    p->check();
    p->invalidate();
}
void Host::layout(Size size) {
    auto p = impl_;
    p->check();
    if (!std::isfinite(size.width) || !std::isfinite(size.height) || size.width < 0 || size.height < 0)
        throw std::invalid_argument("Layout size must be finite and nonnegative");
    p->viewport = size;
    p->perform_layout();
}
void Host::paint(SkCanvas& canvas, const FrameInfo& frame) {
    auto p = impl_;
    p->check();
    p->frame = frame;
    p->repaint = false;
    p->now = frame.elapsed.count();
    p->viewport = frame.logical_size;
    // Content crossfades, colors and custom canvases need paint, but only bounds/size transitions
    // change layout. Include their final frame so the tree settles at the exact target geometry.
    for (const auto& n : p->order) {
        const auto& s = p->states.at(n->id);
        p->layout_dirty |= s.layout_active || (s.active && n->d().size_transition == SizeTransition::animate);
    }
    for (auto& [_, s] : p->states)
        if (!std::isfinite(s.started))
            s.started = p->now;
    for (auto& [_, s] : p->states)
        if (p->progress(s) >= 1) {
            s.active = s.layout_active = false;
            s.outgoing.reset();
        }
    p->perform_layout();
    for (auto& [_, s] : p->states)
        if (!std::isfinite(s.started))
            s.started = p->now;
    for (auto& ghost : p->ghosts)
        if (!std::isfinite(ghost.started))
            ghost.started = p->now;
    p->sample_properties();
    if (p->root)
        p->paint_node(canvas, p->root);
    for (auto& ghost : p->ghosts) {
        const auto raw = std::clamp((p->now - ghost.started) / ghost.animation.duration.count(), 0.0, 1.0);
        const auto t = curve(ghost.animation, raw);
        auto e = ghost.effect;
        e.opacity = lerp(1, e.opacity, t);
        e.scale = lerp(1, e.scale, t);
        e.x *= t;
        e.y *= t;
        SkAutoCanvasRestore restore(&canvas, true);
        canvas.clipRect(skrect(ghost.clip));
        p->picture(canvas, ghost.picture, ghost.bounds, e, 1);
    }
    std::erase_if(p->ghosts, [&](const auto& ghost) {
        return p->now - ghost.started >= ghost.animation.duration.count();
    });
}
bool Host::handle(const Event& event) {
    auto p = impl_;
    p->check();
    return p->handle(event);
}
bool Host::needs_frame() const {
    impl_->check();
    return impl_->busy();
}
std::vector<ElementInfo> Host::elements() const {
    impl_->check();
    return impl_->elements();
}
std::vector<TransitionInfo> Host::transitions() const {
    impl_->check();
    std::vector<TransitionInfo> result;
    for (const auto& [id, p] : impl_->states)
        if (p.active)
            result.push_back({id, impl_->progress(p), bool(p.outgoing)});
    return result;
}
bool Host::activate(const std::string& id) {
    auto p = impl_;
    p->check();
    p->perform_layout();
    return p->activate(id);
}
bool Host::adjust(const std::string& id, double steps) {
    auto p = impl_;
    p->check();
    p->perform_layout();
    return p->adjust(id, steps);
}
bool Host::set_value(const std::string& id, double value) {
    auto p = impl_;
    p->check();
    p->perform_layout();
    return p->set_value(id, value);
}
bool Host::focus(const std::string& id) {
    auto p = impl_;
    p->check();
    p->perform_layout();
    return p->focus(id);
}
std::uint64_t Host::generation() const {
    impl_->check();
    return impl_->revision;
}

} // namespace skia_window::ui
