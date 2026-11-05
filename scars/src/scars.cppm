module;
#include <algorithm>
#include <chrono>
#include <concepts>
#include <compare>
#include <cstdint>
#include <functional>
#include <initializer_list>
#include <limits>
#include <memory>
#include <optional>
#include <ranges>
#include <span>
#include <type_traits>
#include <stdexcept>
#include <string>
#include <thread>
#include <utility>
#include <variant>
#include <vector>
class SkCanvas;

export module scars;

export namespace scars {
using Color = std::uint32_t;
inline constexpr double infinity = std::numeric_limits<double>::infinity();
struct Point {
    double x{}, y{};
};
struct Size {
    double width{}, height{};
};
struct Rect {
    double x{}, y{}, width{}, height{};
    bool contains(Point p) const {
        return p.x >= x && p.y >= y && p.x < x + width && p.y < y + height;
    }
};
struct Constraints {
    Size min{}, max{infinity, infinity};
};
struct Length {
    double value{};
    enum class Unit { logical, percent, automatic } unit{Unit::logical};
    constexpr Length(double n = 0) : value(n) {}
    constexpr Length(double n, Unit u) : value(n), unit(u) {}
};
inline constexpr Length automatic{0, Length::Unit::automatic};
constexpr Length percent(double n) {
    return {n / 100, Length::Unit::percent};
}
namespace literals {
constexpr Length operator""_lp(unsigned long long n) {
    return Length(static_cast<double>(n));
}
constexpr Length operator""_lp(long double n) {
    return Length(static_cast<double>(n));
}
constexpr Length operator""_pct(unsigned long long n) {
    return percent(static_cast<double>(n));
}
} // namespace literals
struct Insets {
    double top{}, right{}, bottom{}, left{};
    static Insets all(double n) {
        return {n, n, n, n};
    }
    static Insets symmetric(double vertical, double horizontal) {
        return {vertical, horizontal, vertical, horizontal};
    }
};
enum class Align { start, center, end, stretch, baseline };
enum class Justify { start, center, end, space_between, space_around };
enum class TextWrap { no_wrap, wrap, break_word };
enum class TextOverflow { clip, ellipsis };
enum class LabelStyle { standard, heading, section, secondary, micro, status, error };
enum class CheckState { unchecked, checked, indeterminate };
enum class Density { compact, standard, spacious };
enum class Orientation { horizontal, vertical };
enum class Position { normal, relative, absolute };
enum class ScrollAxes { horizontal, vertical, both };
struct ScrollViewOptions {
    ScrollAxes axes{ScrollAxes::vertical};
    bool show_scrollbars{true};
};
struct Theme {
    Color window{0xff0f1924}, panel{0xff192b3d}, group{0xff172738}, inactive{0xff334252};
    Color text{0xffe6f0ff}, secondary{0xff9fb0c8}, disabled{0xff718096};
    Color primary{0xffffaa34}, blue{0xff168bf0}, cyan{0xff4dd2ff}, error{0xffff5a5a}, border{0xff395269};
    double control_height{40}, compact_height{32}, spacious_height{48}, button_radius{12};
    double split_width{40}, popup_item_height{36}, popup_padding{4}, popup_offset{4}, focus_width{1};
    // Empty uses the platform's sans-serif. A path makes previews and tests reproducible.
    std::string font_path;
    Color field{0xff142332}, selection{0xff235679}, caret{0xff4dd2ff};
    bool light_mode{};
    static Theme light();
};
namespace detail {
std::string uppercase_text(const std::string&);
struct Observer {
    std::uint64_t epoch{};
    std::function<void()> changed;
};
struct CellBase {
    std::thread::id owner{std::this_thread::get_id()};
    std::vector<std::pair<std::weak_ptr<Observer>, std::uint64_t>> observers;
    void read();
    void write();
    void notify();
};
template <class T> struct Cell : CellBase {
    T value;
    explicit Cell(T n) : value(std::move(n)) {}
};
struct Description;
} // namespace detail
template <class T> class Binding {
    std::function<T()> read_;
    std::function<void(T)> write_;

  public:
    Binding(std::function<T()> read, std::function<void(T)> write)
        : read_(std::move(read)), write_(std::move(write)) {}
    T get() const {
        return read_();
    }
    void set(T n) const {
        write_(std::move(n));
    }
};
template <class T> class State {
    std::shared_ptr<detail::Cell<T>> cell_;

  public:
    explicit State(T value = {}) : cell_(std::make_shared<detail::Cell<T>>(std::move(value))) {}
    T get() const {
        cell_->read();
        return cell_->value;
    }
    void set(T n) const {
        cell_->write();
        if constexpr (std::equality_comparable<T>) {
            if (cell_->value == n)
                return;
        }
        cell_->value = std::move(n);
        cell_->notify();
    }
    Binding<T> binding() const {
        return {[self = *this] { return self.get(); }, [self = *this](T n) { self.set(std::move(n)); }};
    }
};
// A replaceable paragraph-layout seam. Public data uses logical coordinates and UTF-8;
// neither CoreText nor SkParagraph types escape into the view or layout engine.
struct TextInlineBox {
    Size size;
    double baseline{};
};
struct TextSpan {
    std::string text;
    double font_size{14}, line_height{18}, letter_spacing{};
    bool bold{};
    Color color{0xffe6f0ff};
    std::optional<TextInlineBox> box;
};
struct TextLayoutOptions {
    double width{infinity};
    TextWrap wrap{TextWrap::wrap};
    TextOverflow overflow{TextOverflow::clip};
    int max_lines{};
    Align align{Align::start};
};
struct TextPlacement {
    std::size_t span{};
    Rect bounds;
    bool inline_box{};
};
// Text offsets and ranges in SCARS are UTF-8 byte offsets, always normalized to
// extended grapheme boundaries. The native adapter converts AppKit's UTF-16 ranges.
struct TextRange {
    std::size_t start{}, length{};
    std::size_t end() const {
        return start + length;
    }
    friend bool operator==(const TextRange&, const TextRange&) = default;
};
struct TextSelection {
    std::size_t anchor{}, caret{};
    TextRange range() const {
        return {std::min(anchor, caret), std::max(anchor, caret) - std::min(anchor, caret)};
    }
    friend bool operator==(const TextSelection&, const TextSelection&) = default;
};
enum class TextCommand {
    left,
    right,
    word_left,
    word_right,
    line_start,
    line_end,
    document_start,
    document_end,
    up,
    down,
    page_up,
    page_down,
    paragraph_start,
    paragraph_end,
    backspace,
    delete_forward,
    delete_word_backward,
    delete_word_forward,
    delete_line_backward,
    delete_line_forward,
    select_all,
    copy,
    cut,
    paste,
    undo,
    redo,
    submit,
    cancel,
    transpose,
    delete_paragraph_backward,
    delete_paragraph_forward
};
class TextBuffer {
  public:
    explicit TextBuffer(std::string = {});
    ~TextBuffer();
    TextBuffer(TextBuffer&&) noexcept;
    TextBuffer& operator=(TextBuffer&&) noexcept;
    TextBuffer(const TextBuffer&) = delete;
    TextBuffer& operator=(const TextBuffer&) = delete;
    const std::string& text() const;
    std::string committed_text() const;
    TextSelection selection() const;
    void select(TextSelection);
    void set_text(std::string); // External replacement clears history.
    void replace(std::string, std::optional<TextRange> = {}, bool coalesce_typing = false);
    void mark(std::string, TextRange selection_inside, std::optional<TextRange> replacement = {});
    std::optional<TextRange> marked_range() const;
    void unmark(bool commit = true);
    bool undo();
    bool redo();
    bool can_undo() const;
    bool can_redo() const;
    void clear_history(); // Keeps current text, selection, and any active composition.
    void command(TextCommand, bool extend = false);
    std::size_t normalize(std::size_t) const;

  private:
    struct Impl;
    std::unique_ptr<Impl> impl_;
};
std::u16string utf16_from_utf8(const std::string&);
std::string utf8_from_utf16(const std::u16string&);
std::size_t utf16_offset(const std::string&, std::size_t utf8_offset);
std::size_t utf8_offset(const std::string&, std::size_t utf16_offset);
std::vector<std::size_t> grapheme_boundaries(const std::string&);
struct TextLine {
    TextRange range;
    Rect bounds;
};
struct GlyphPosition {
    std::uint16_t glyph{};
    Point position;
    std::uint32_t utf8_offset{};
};
struct TextLayout {
    Size size;
    double baseline{};
    std::vector<TextPlacement> placements;
    std::vector<GlyphPosition> glyphs;
    // Owns the shaped result; painting needs no platform layout calls.
    std::function<void(SkCanvas&, Point)> paint;
    std::function<std::size_t(Point)> hit_test;
    std::function<Rect(std::size_t)> caret_rect;
    std::function<std::vector<Rect>(TextRange)> selection_rects;
    std::vector<TextLine> lines;
};
class TextLayoutEngine {
  public:
    virtual ~TextLayoutEngine() = default;
    virtual std::shared_ptr<const TextLayout> layout(const std::vector<TextSpan>&, TextLayoutOptions) = 0;
};
std::shared_ptr<TextLayoutEngine> make_skparagraph_text_engine(std::string font_path = {});

struct Option {
    std::string id, label;
    bool enabled{true};
    friend bool operator==(const Option&, const Option&) = default;
};
struct Action {
    std::string id, label;
    std::function<void()> invoke;
    bool enabled{true};
};
enum class MenuItemKind { action, separator, check, radio, submenu };
struct MenuItem {
    std::string id, label;
    std::function<void()> invoke;
    bool enabled{true};
    std::string shortcut; // Display label; application commands own shortcut dispatch.
    MenuItemKind kind{MenuItemKind::action};
    std::function<bool()> is_checked;
    std::vector<MenuItem> children;
    static MenuItem separator(std::string id);
    static MenuItem check(std::string id, std::string label, Binding<bool>, std::string shortcut = {});
    static MenuItem radio(std::string id, std::string label, Binding<std::string>, std::string value);
    static MenuItem submenu(std::string id, std::string label, std::vector<MenuItem>);
};
struct SliderOptions {
    double min{}, max{1}, step{.01}, page_step{.1};
};
enum class SplitPane { none, first, second };
struct SplitViewOptions {
    Orientation orientation{Orientation::horizontal}; // Axis along which panes are arranged.
    double first_min{100}, second_min{100}, divider_width{12}, keyboard_step{10};
    std::optional<Binding<SplitPane>> collapsed;
};
struct TextFieldOptions {
    bool multiline{}, read_only{};
    std::size_t max_length{}; // 0 = unlimited; counts extended grapheme clusters.
    std::function<void()> on_submit;
};
struct SearchFieldOptions {
    std::chrono::milliseconds debounce{};
    bool clear_button{true}, read_only{};
    std::size_t max_length{};
    std::function<void(const std::string&)> on_search, on_submit;
    std::function<void()> on_cancel;
};
struct PasswordFieldOptions {
    bool allow_reveal{}, allow_undo{}, read_only{};
    std::size_t max_length{};
    std::function<void()> on_submit;
};
enum class ComboBoxMode { selection, free_text };
enum class ComboBoxFilter { contains, prefix, none };
struct ComboBoxOptions {
    ComboBoxMode mode{ComboBoxMode::selection};
    ComboBoxFilter filter{ComboBoxFilter::contains};
    bool loading{}, read_only{}, auto_highlight{true};
    std::size_t max_length{};
    double max_height{240}, row_height{32};
    std::string no_results{"No matches"}, loading_text{"Loading…"};
    std::function<void(const std::string&)> on_query;
    std::function<void(const std::string&, const std::string&)> on_accept;
    std::function<void()> on_cancel;
};
enum class NumericStatus { valid, incomplete, invalid, out_of_range };
struct NumericValidation {
    NumericStatus status{NumericStatus::valid};
    std::string message;
    bool dirty{};
    friend bool operator==(const NumericValidation&, const NumericValidation&) = default;
};
template<class T> requires (std::signed_integral<T> || std::same_as<T, double>)
struct NumericOptions {
    T min{std::numeric_limits<T>::lowest()}, max{std::numeric_limits<T>::max()}, step{1};
    std::string locale{"en_US_POSIX"};
    int significant_digits{17}; // Display precision for decimals; integer formatting stays exact.
    bool grouping{}, commit_on_blur{true}, read_only{};
    std::function<void(const NumericValidation&)> on_validation;
    std::function<void()> on_submit;
    std::chrono::milliseconds repeat_delay{400}, repeat_interval{80};
};
enum class ColorModel { rgb, hsv };
struct HSV {
    double hue{}, saturation{}, value{}; // Degrees [0,360]; saturation/value [0,1].
    friend bool operator==(const HSV&, const HSV&) = default;
};
HSV color_hsv(Color);
Color color_from_hsv(HSV, std::uint8_t alpha = 255);
std::string format_color(Color, bool alpha = true); // CSS #RRGGBBAA, never packed #AARRGGBB.
std::optional<Color> parse_color(const std::string&);
struct ColorPickerOptions {
    bool alpha{true}, live_preview{}, read_only{};
    ColorModel model{ColorModel::rgb};
};
struct Date {
    int year{1970};
    unsigned month{1}, day{1};
    friend auto operator<=>(const Date&, const Date&) = default;
};
struct Time {
    unsigned hour{}, minute{}, second{};
    friend auto operator<=>(const Time&, const Time&) = default;
};
struct LocalDateTime {
    Date date;
    Time time;
    friend auto operator<=>(const LocalDateTime&, const LocalDateTime&) = default;
};
using DateTimeValidation = NumericValidation;
enum class HourCycle { h24, h12 };
enum class RepeatedTimePolicy { reject, earlier, later };
struct DatePickerOptions {
    Date min{1, 1, 1}, max{9999, 12, 31};
    std::string locale{"en_US_POSIX"};
    bool read_only{}, commit_on_blur{true};
    std::function<bool(Date)> disabled_date;
    std::function<std::string(Date)> validate;
    std::function<void(const DateTimeValidation&)> on_validation;
    std::function<void()> on_submit;
};
struct TimePickerOptions {
    Time min{}, max{23, 59, 59};
    std::string locale{"en_US_POSIX"};
    HourCycle hour_cycle{HourCycle::h24};
    bool show_seconds{}, read_only{}, commit_on_blur{true};
    unsigned minute_step{1}, second_step{1};
    std::function<std::string(Time)> validate;
    std::function<void(const DateTimeValidation&)> on_validation;
    std::function<void()> on_submit;
};
struct DateTimePickerOptions {
    DatePickerOptions date;
    TimePickerOptions time;
    LocalDateTime min{{1, 1, 1}, {}}, max{{9999, 12, 31}, {23, 59, 59}};
    std::function<std::string(LocalDateTime)> validate;
};
struct ZonedDateTimePickerOptions {
    std::string time_zone{"UTC"};
    RepeatedTimePolicy repeated_time{RepeatedTimePolicy::reject};
    std::optional<std::chrono::sys_seconds> min, max;
    DateTimePickerOptions display;
};
struct DateTimeResolution {
    std::optional<std::chrono::sys_seconds> instant;
    std::string error;
};
bool valid(Date);
bool valid(Time);
std::string format_date(Date, std::string locale = "en_US_POSIX");
std::optional<Date> parse_date(const std::string&, std::string locale = "en_US_POSIX");
std::string format_time(Time, TimePickerOptions = {});
std::optional<Time> parse_time(const std::string&, TimePickerOptions = {});
LocalDateTime local_datetime(std::chrono::sys_seconds, const std::string& time_zone);
// Nonexistent wall times are rejected; repeated times follow the explicit policy.
DateTimeResolution resolve_datetime(LocalDateTime, const std::string& time_zone,
                                    RepeatedTimePolicy = RepeatedTimePolicy::reject);
namespace detail {
template<std::signed_integral T> NumericOptions<std::int64_t> widen_numeric(NumericOptions<T> o) {
    return {o.min, o.max, o.step, std::move(o.locale), o.significant_digits, o.grouping,
            o.commit_on_blur, o.read_only, std::move(o.on_validation), std::move(o.on_submit),
            o.repeat_delay, o.repeat_interval};
}
} // namespace detail
struct Clipboard {
    std::function<std::string()> read;
    std::function<void(const std::string&)> write;
};
struct TextInputSnapshot {
    std::string text;
    TextSelection selection;
    std::optional<TextRange> marked;
    Rect caret, viewport;
    bool editable{}, multiline{}, can_undo{}, can_redo{};
    bool secure{}; // Privileged text-input bridge; text remains raw for native range conversion.
    bool active{true};
    bool combo{}, date_picker{};
};
struct ListBoxOptions {
    double row_height{32};
    std::size_t visible_rows{5};
    std::function<void(const std::string&)> on_activate;
    std::size_t overscan_rows{2};
};
struct WizardStep {
    std::string id, title;
    // Pure and reactive: return an explanation to block forward navigation,
    // or an empty string when the step is valid. Rechecked at activation time.
    std::function<std::string()> validate;
};
struct WizardOptions {
    bool allow_back{true}, review_previous{true}, validate_all_on_finish{true};
    bool show_steps{true}, show_status{true};
    std::string back_label{"Back"}, next_label{"Next"}, finish_label{"Finish"}, cancel_label{"Cancel"};
    std::function<void()> on_finish, on_cancel;
    std::function<void(const std::string& from, const std::string& to)> on_change;
};
class Wizard {
  public:
    Wizard(Binding<std::string> current_step, std::vector<WizardStep>, WizardOptions = {});
    std::string current() const;
    std::size_t index() const;
    bool is_last() const;
    bool can_back() const;
    bool can_advance() const;
    std::string validation_message() const;
    bool back() const;
    bool next() const;
    bool finish() const;
    bool go_to(const std::string& step_id) const;
    bool cancel() const;

  private:
    struct Impl;
    std::shared_ptr<Impl> impl_;
    friend struct WizardNavigation;
};
enum class ContentMode { fit, fill, stretch, center };
class Bitmap {
  public:
    Bitmap();
    static Bitmap load(const std::string& path);
    static Bitmap from_rgba(std::size_t width, std::size_t height, std::span<const std::uint8_t>,
                            std::size_t row_bytes = 0);
    Size size() const;
    bool empty() const;
    void paint(SkCanvas&, Rect, ContentMode = ContentMode::fit) const;

  private:
    struct Impl;
    std::shared_ptr<Impl> impl_;
};

enum class OverlayPlacement { below, above, left, right };
enum class PopoverFocus { content, preserve };
struct TooltipOptions {
    std::chrono::milliseconds delay{600};
    double max_width{280}, gap{8}, margin{8};
    OverlayPlacement placement{OverlayPlacement::below};
};
struct PopoverOptions {
    OverlayPlacement placement{OverlayPlacement::below};
    PopoverFocus focus{PopoverFocus::content};
    double width{320}, max_height{480}, gap{8}, margin{8};
    bool dismiss_on_outside{true}, dismiss_on_escape{true};
    std::function<void()> on_dismiss;
};
class View {
  public:
    View();
    View key(std::string) const;
    View width(Length) const;
    View height(Length) const;
    View min_width(double) const;
    View max_width(double) const;
    View min_height(double) const;
    View max_height(double) const;
    View padding(double) const;
    View padding(Insets) const;
    View margin(double) const;
    View margin(Insets) const;
    View gap(double) const;
    View grow(double = 1) const;
    View shrink(double = 1) const;
    View basis(Length) const;
    View wrap(bool = true) const;
    View align_items(Align) const;
    View align_self(Align) const;
    View justify(Justify) const;
    View foreground(Color) const;
    View background(Color) const;
    View accent(Color) const;
    View corner_radius(double) const;
    View font_size(double) const;
    View bold(bool = true) const;
    View label_style(LabelStyle) const;
    View text_wrap(TextWrap) const;
    View text_overflow(TextOverflow) const;
    View max_lines(int) const;
    View text_align(Align) const;
    View uppercase(bool = true) const;
    View density(Density) const;
    View enabled(bool) const;
    View visible(bool) const;
    View hidden(bool = true) const;
    View position(Position) const;
    View left(Length) const;
    View top(Length) const;
    View right(Length) const;
    View bottom(Length) const;
    View z_index(int) const;
    View content_mode(ContentMode) const;
    View accessible_label(std::string) const;
    View expanded(Binding<bool>) const;
    View tooltip(std::string, TooltipOptions = {}) const;
    View popover(Binding<bool>, std::function<View()>, PopoverOptions = {}) const;
    View context_menu(std::vector<MenuItem>) const;
    View context_menu(std::function<std::vector<MenuItem>()>) const;
    const detail::Description& description() const;

  private:
    std::shared_ptr<const detail::Description> description_;
    explicit View(std::shared_ptr<const detail::Description>);
    View modify(const std::function<void(detail::Description&)>&) const;
    friend struct Label;
    friend struct Button;
    friend class Alert;
    friend struct ToggleButton;
    friend struct SegmentedControl;
    friend struct CheckBox;
    friend struct RadioGroup;
    friend struct MenuButton;
    friend struct SelectionDropdown;
    friend struct SplitActionButton;
    friend struct Slider;
    friend struct SplitView;
    friend struct TabView;
    friend struct NumericField;
    friend struct DatePicker;
    friend struct TimePicker;
    friend struct DateTimePicker;
    friend class Host;
    friend struct ColorPicker;
    friend struct ColorWell;
    friend struct Stepper;
    friend struct Switch;
    friend struct SearchField;
    friend struct PasswordField;
    friend struct ComboBox;
    friend struct TextField;
    friend struct ListBox;
    friend struct TreeView;
    friend struct TableView;
    friend struct Image;
    friend struct ForEach;
    friend struct If;
    friend struct ProgressBar;
    friend struct FlexRow;
    friend struct FlexColumn;
    friend struct Block;
    friend struct Inline;
    friend struct Layer;
    friend struct Section;
    friend struct GroupBox;
    friend struct Panel;
    friend struct Separator;
    friend struct ScrollView;
    friend struct Spacer;
};
struct Label : View {
    explicit Label(std::string, LabelStyle = LabelStyle::standard);
};
using Text = Label;
struct Button : View {
    Button(std::string, std::function<void()>);
};
using CommandButton = Button;
struct ToggleButton : View {
    ToggleButton(std::string, Binding<bool>);
};
struct SegmentedControl : View {
    SegmentedControl(Binding<std::string>, std::vector<Option>);
};
struct CheckBox : View {
    CheckBox(std::string, Binding<bool>);
    CheckBox(std::string, Binding<CheckState>);
};
struct RadioGroup : View {
    RadioGroup(Binding<std::string>, std::vector<Option>);
};
struct MenuButton : View {
    MenuButton(std::string, std::vector<Action>);
};
struct SelectionDropdown : View {
    SelectionDropdown(Binding<std::string>, std::vector<Option>);
};
struct ComboBox : View {
    ComboBox(std::string placeholder, Binding<std::string> query, Binding<std::string> selection,
             std::vector<Option>, ComboBoxOptions = {});
};
struct SplitActionButton : View {
    SplitActionButton(Binding<std::string>, std::vector<Action>);
};
struct Slider : View {
    explicit Slider(Binding<double>, SliderOptions = {});
};
struct SplitView : View {
    // Fraction of the available pane extent assigned to the first pane, excluding the divider.
    SplitView(View first, View second, Binding<double> fraction, SplitViewOptions = {});
};
struct Tab {
    std::string id, label;
    std::function<View()> content;
    bool enabled{true};
};
struct TabView : View {
    TabView(Binding<std::string> selection, std::vector<Tab>);
};
struct TextField : View {
    TextField(std::string placeholder, Binding<std::string>, TextFieldOptions = {});
};
struct SearchField : View {
    SearchField(std::string placeholder, Binding<std::string>, SearchFieldOptions = {});
};
struct PasswordField : View {
    PasswordField(std::string placeholder, Binding<std::string>, PasswordFieldOptions = {});
};
struct NumericField : View {
    NumericField(std::string placeholder, Binding<double>, NumericOptions<double> = {});
    NumericField(std::string placeholder, Binding<std::int64_t>, NumericOptions<std::int64_t> = {});
    template<std::signed_integral T> requires (!std::same_as<T, std::int64_t>)
    NumericField(std::string placeholder, Binding<T> value, NumericOptions<T> options = {})
        : NumericField(std::move(placeholder), Binding<std::int64_t>{[value] { return value.get(); },
              [value](std::int64_t n) { value.set(static_cast<T>(n)); }}, detail::widen_numeric(std::move(options))) {}
};
struct Stepper : View {
    explicit Stepper(Binding<double>, NumericOptions<double> = {});
    explicit Stepper(Binding<std::int64_t>, NumericOptions<std::int64_t> = {});
    template<std::signed_integral T> requires (!std::same_as<T, std::int64_t>)
    explicit Stepper(Binding<T> value, NumericOptions<T> options = {})
        : Stepper(Binding<std::int64_t>{[value] { return value.get(); },
              [value](std::int64_t n) { value.set(static_cast<T>(n)); }}, detail::widen_numeric(std::move(options))) {}
};
struct Switch : View {
    Switch(std::string label, Binding<bool>);
};
struct DatePicker : View {
    DatePicker(std::string label, Binding<Date>, DatePickerOptions = {});
};
struct TimePicker : View {
    TimePicker(std::string label, Binding<Time>, TimePickerOptions = {});
};
struct DateTimePicker : View {
    DateTimePicker(std::string label, Binding<LocalDateTime>, DateTimePickerOptions = {});
    DateTimePicker(std::string label, Binding<std::chrono::sys_seconds>, ZonedDateTimePickerOptions = {});
};
struct ColorPicker : View {
    ColorPicker(std::string label, Binding<Color>, ColorPickerOptions = {});
};
struct ColorWell : View {
    ColorWell(std::string label, Binding<Color>, ColorPickerOptions = {});
};
struct TextEditor : TextField {
    explicit TextEditor(Binding<std::string>, std::string placeholder = {}, TextFieldOptions = {});
};
struct RowWindow {
    std::size_t visible_begin{}, visible_end{}, mounted_begin{}, mounted_end{};
};
RowWindow row_window(std::size_t count, double offset, double viewport_height, double row_height,
                     std::size_t overscan = 2);
class RowModel {
  public:
    using Builder = std::function<View(const Option&, std::size_t)>;
    RowModel();
    explicit RowModel(std::vector<Option>, Builder = {});
    std::size_t size() const;
    const Option& at(std::size_t) const;
    std::optional<std::size_t> index_of(const std::string&) const;
    std::span<const std::size_t> enabled_indices() const;
    bool has_content() const;
    View build(std::size_t) const;
    friend bool operator==(const RowModel&, const RowModel&) = default;
  private:
    struct Impl;
    std::shared_ptr<const Impl> impl_;
};
struct ListBox : View {
    ListBox(Binding<std::string>, std::vector<Option>, ListBoxOptions = {});
    ListBox(Binding<std::vector<std::string>>, std::vector<Option>, ListBoxOptions = {});
    ListBox(Binding<std::string>, RowModel, ListBoxOptions = {});
    ListBox(Binding<std::vector<std::string>>, RowModel, ListBoxOptions = {});
    ListBox(Binding<std::string>, std::initializer_list<Option>, ListBoxOptions = {});
    ListBox(Binding<std::vector<std::string>>, std::initializer_list<Option>, ListBoxOptions = {});
};
struct TreeNode {
    std::string id, label;
    bool enabled{true}, loading{};
    std::vector<TreeNode> children;
    // Pure snapshot loader, evaluated once when this branch first expands.
    // Replace TreeModel to publish new or asynchronously loaded metadata.
    std::function<std::vector<TreeNode>()> lazy_children;
    bool expandable{};
    std::string accessible_label;
};
struct TreeItem {
    std::string id, label, parent;
    std::size_t depth{};
    bool enabled{true}, expandable{}, expanded{}, loading{};
    std::string accessible_label;
    std::size_t child_count{};
};
class TreeModel {
  public:
    TreeModel();
    explicit TreeModel(std::vector<TreeNode>);
    std::vector<TreeItem> flatten(std::span<const std::string> expanded) const;
    std::optional<TreeItem> item(const std::string&) const;
    std::size_t known_count() const;
    friend bool operator==(const TreeModel&, const TreeModel&) = default;
  private:
    struct Impl;
    std::shared_ptr<Impl> impl_;
};
struct TreeViewOptions {
    double row_height{32};
    std::size_t visible_rows{8}, overscan_rows{2};
    double indentation{18};
    std::function<void(const std::string&)> on_activate;
    std::function<void(const std::string&, bool)> on_expand;
};
struct TreeView : ListBox {
    TreeView(Binding<std::string> selection, TreeModel, Binding<std::vector<std::string>> expanded,
             TreeViewOptions = {});
    TreeView(Binding<std::vector<std::string>> selection, TreeModel, Binding<std::vector<std::string>> expanded,
             TreeViewOptions = {});
};
struct TableCell {
    std::string row, column;
    friend bool operator==(const TableCell&, const TableCell&) = default;
};
struct TableSelection {
    std::vector<std::string> rows;
    std::vector<TableCell> cells;
    friend bool operator==(const TableSelection&, const TableSelection&) = default;
};
enum class TableSelectionMode { rows, cells };
enum class SortOrder { ascending, descending };
struct SortDescriptor {
    std::string column;
    SortOrder order{SortOrder::ascending};
    friend bool operator==(const SortDescriptor&, const SortDescriptor&) = default;
};
struct TableColumnWidth {
    std::string column;
    double width{};
    friend bool operator==(const TableColumnWidth&, const TableColumnWidth&) = default;
};
struct TableColumn {
    std::string id, title;
    double width{160}, min_width{48}, max_width{800};
    bool resizable{true}, sortable{};
    Align alignment{Align::start};
    std::function<std::string(const Option&, std::size_t)> text;
    std::function<View(const Option&, std::size_t)> content;
    // A commit callback enables the standard draft text editor for this column.
    std::function<std::string(const std::string& row, const std::string& draft)> validate;
    std::function<void(const std::string& row, std::string value)> commit;
    bool multiline{};
};
class TableModel {
  public:
    TableModel();
    TableModel(std::vector<Option> rows, std::vector<TableColumn> columns);
    const RowModel& rows() const;
    const std::vector<TableColumn>& columns() const;
    std::optional<std::size_t> column_index(const std::string&) const;
    std::string text(std::size_t row, std::size_t column) const;
    friend bool operator==(const TableModel&, const TableModel&) = default;
  private:
    struct Impl;
    std::shared_ptr<const Impl> impl_;
};
struct TableViewOptions {
    double row_height{32}, header_height{34};
    std::size_t visible_rows{8}, overscan_rows{2};
    TableSelectionMode selection_mode{TableSelectionMode::rows};
    bool multiple_selection{};
    std::optional<Binding<std::vector<TableColumnWidth>>> widths;
    std::optional<Binding<std::vector<SortDescriptor>>> sort;
    std::function<void(const std::vector<SortDescriptor>&)> on_sort;
};
struct TableView : ListBox {
    TableView(Binding<TableSelection>, TableModel, TableViewOptions = {});
};
struct Image : View {
    explicit Image(Bitmap, ContentMode = ContentMode::fit);
};
struct WizardNavigation : View {
    explicit WizardNavigation(Wizard);
    WizardNavigation(Binding<std::string> current_step, std::vector<WizardStep>, WizardOptions = {});
};
// Structural views contribute their children directly to the surrounding layout.
struct ForEach : View {
    explicit ForEach(std::vector<View> keyed_views);
    template <std::ranges::input_range Range, class KeyFn, class Builder>
    ForEach(Range&& values, KeyFn key, Builder build)
        : ForEach([&] {
              std::vector<View> result;
              for (auto&& value : values) {
                  auto identity = std::invoke(key, value);
                  std::string id;
                  if constexpr (std::is_integral_v<decltype(identity)>)
                      id = std::to_string(identity);
                  else
                      id = std::string(identity);
                  result.push_back(std::invoke(build, value).key(std::move(id)));
              }
              return result;
          }()) {}
};
struct If : View {
    If(bool, View then_view, View else_view = {});
    If(bool, std::function<View()> then_builder, std::function<View()> else_builder = {});
};
struct ProgressBar : View {
    explicit ProgressBar(double fraction);
    explicit ProgressBar(Binding<double>);
    ProgressBar(); // Indeterminate.
};
struct FlexRow : View {
    FlexRow(std::initializer_list<View>);
    explicit FlexRow(std::vector<View>);
};
struct FlexColumn : View {
    FlexColumn(std::initializer_list<View>);
    explicit FlexColumn(std::vector<View>);
};
struct Block : View {
    Block(std::initializer_list<View>);
    explicit Block(std::vector<View>);
};
struct Inline : View {
    Inline(std::initializer_list<View>);
    explicit Inline(std::vector<View>);
};
// Children share one content box. Each subtree is stacked as a unit by z-index.
struct Layer : View {
    Layer(std::initializer_list<View>);
    explicit Layer(std::vector<View>);
};
struct Section : View {
    Section(std::string title, View content);
};
struct GroupBox : View {
    GroupBox(std::string title, View content);
};
struct Panel : View {
    explicit Panel(View content);
};
enum class AlertActionRole { normal, default_action, cancel, destructive };
struct AlertAction {
    std::string id, label;
    AlertActionRole role{AlertActionRole::normal};
    bool enabled{true};
};
// One presentation transaction. Retain the handle across view rebuilds.
class Alert : public View {
  public:
    Alert(std::string title, View content, std::vector<AlertAction>,
          std::function<void(std::optional<std::string>)> completion = {});
    Alert(std::string title, std::function<View()> content, std::vector<AlertAction>,
          std::function<void(std::optional<std::string>)> completion = {});
    bool choose(const std::string& action_id) const;
    bool cancel() const;  // Selects the explicit enabled cancel action, if present.
    bool dismiss() const; // Completes without an action ID.
    bool completed() const;
    std::optional<std::string> result() const;
    const std::vector<AlertAction>& actions() const;

  private:
    struct Impl;
    std::shared_ptr<Impl> impl_;
};
struct Separator : View {
    explicit Separator(Orientation = Orientation::horizontal);
};
struct ScrollView : View {
    explicit ScrollView(View content, ScrollViewOptions = {});
};
struct Spacer : View {
    Spacer();
};
// Annotates any view; the association also supplies an accessible name when none was set.
View labeled(std::string annotation, View content);

// Native-independent input and semantic snapshots. Coordinates use logical pixels.
enum class PointerKind { move, down, up, scroll, leave, cancel };
enum class Cursor { arrow, text, resize_horizontal, resize_vertical };
struct PointerEvent {
    PointerKind kind{};
    Point position;
    int button{};
    double scroll_x{}, scroll_y{};
    bool shift{};
    int click_count{1};
    bool command{};
    bool control{};
};
enum class Key {
    unknown,
    tab,
    space,
    enter,
    escape,
    left,
    right,
    up,
    down,
    home,
    end,
    page_up,
    page_down,
    f2,
    backspace,
    delete_forward,
    a,
    c,
    x,
    v,
    z,
    y,
    f,
    f10
};
struct KeyEvent {
    Key key{};
    bool down{true}, repeat{}, shift{};
    bool command{}, alt{}, control{};
    std::string text{}; // Printable text for non-editing controls such as list type-ahead.
};
enum class Role {
    text,
    text_field,
    list_box,
    list_item,
    image,
    button,
    toggle,
    checkbox,
    radio,
    segment,
    menu_button,
    selection,
    split_button,
    slider,
    progress,
    group,
    menu_item,
    splitter,
    tab,
    tab_group,
    stepper,
    switch_control,
    menu,
    tree_view,
    tree_item,
    table,
    table_row,
    table_cell,
    column_header,
    alert
};
struct Element {
    std::string id, label, group, value_text;
    Role role{};
    Rect bounds{}, clip{};
    bool enabled{true}, focused{}, selected{}, mixed{}, expanded{};
    double value{}, min{}, max{1};
    std::uint64_t paint_order{};
    TextSelection text_selection;
    bool read_only{}, multiline{};
    Orientation orientation{Orientation::horizontal};
    std::string help;
    bool invalid{};
    bool secure{}, search{}, combo{};
    bool has_submenu{};
    std::string shortcut;
    std::optional<std::size_t> item_count, item_index;
    std::vector<std::size_t> selected_items;
    std::optional<TreeItem> tree;
    std::optional<TableCell> cell;
    std::optional<std::size_t> column_index, column_count;
    std::optional<SortOrder> sort_order;
    std::optional<std::size_t> sort_priority;
    std::vector<std::string> selected_cells;
    std::string table_id;
    bool sortable{};
    std::optional<AlertActionRole> alert_action;
    std::optional<Date> calendar_date;
};
struct LogicalItem {
    std::string id, label, group;
    std::size_t index{}, count{};
    bool enabled{}, selected{}, active{}, focused{};
    std::optional<Rect> bounds, clip;
    std::optional<TreeItem> tree;
    std::optional<std::size_t> column_count;
};
struct LogicalCell {
    std::string id, group, label, value;
    TableCell cell;
    std::size_t row_index{}, column_index{};
    bool enabled{}, selected{}, focused{}, editable{}, editing{};
    std::optional<Rect> bounds, clip;
    std::string validation;
    std::string table;
};
struct LogicalColumn {
    std::string id, table, key, header_id, label;
    std::size_t index{}, count{}, row_count{};
    bool enabled{}, sortable{}, resizable{};
    double width{}, min_width{}, max_width{};
    std::optional<SortOrder> sort_order;
    std::optional<std::size_t> sort_priority;
    std::optional<Rect> bounds, clip;
};
// Temporary developer edits, applied after the view builder's modifiers. Replacing
// an override restores any omitted properties to their application-defined values.
struct LayoutOverride {
    std::optional<Length> width, height, basis;
    std::optional<Insets> padding, margin;
    std::optional<double> gap, grow, shrink;
    std::optional<Align> align, self;
    std::optional<Justify> justify;
    std::optional<bool> wrap;
    std::optional<Orientation> direction; // FlexRow / FlexColumn only.
    std::optional<Position> position;
    std::optional<Length> left, top, right, bottom;
    std::optional<int> z_index;
    std::optional<bool> hidden;
};
enum class InspectionOverlay { selection, box_model, flex };
struct FlexLineInspection {
    Rect bounds;
    std::vector<std::string> items;
    double gap{}, free_space{}; // Resolved gap; signed space left after flex sizing, before justification.
};
struct SplitInspection {
    Orientation orientation{Orientation::horizontal};
    double fraction{}, first_extent{}, second_extent{}, divider_extent{}, first_min{}, second_min{};
    SplitPane collapsed{SplitPane::none};
};
struct TabInspection {
    std::string requested_selection;
    std::size_t tab_count{}, visited_count{};
    double strip_offset{};
};
struct ScrollInspection {
    ScrollAxes axes{ScrollAxes::vertical};
    Point offset{}, maximum{};
    Size content{};
    Rect viewport{};
};
struct ListInspection {
    std::size_t logical_count{}, mounted_count{}, retained_editors{};
    RowWindow window;
    std::string active, pinned;
};
// A preorder snapshot of every mounted view, including layout containers.
struct InspectionNode {
    std::string id, parent_id, type, key, text, label, selection;
    std::string overlay_scope;
    std::string shortcut;
    std::size_t depth{};
    Rect bounds{}, clip{}, content_bounds{};
    Size measured{};
    Insets padding{}, margin{};
    Length width{automatic}, height{automatic}, basis{automatic};
    double min_width{}, min_height{}, max_width{infinity}, max_height{infinity};
    double gap{}, grow{}, shrink{1}, font_size{}, line_height{}, letter_spacing{}, corner_radius{};
    Align align{Align::stretch};
    std::optional<Align> self;
    Justify justify{Justify::start};
    Color foreground{}, accent{};
    std::optional<Color> background;
    std::optional<double> value;
    bool enabled{}, focused{}, selected{}, expanded{}, interactive{}, bold{};
    bool wrap{}, layout_editable{}, layout_overridden{};
    std::vector<FlexLineInspection> flex_lines;
    std::optional<SplitInspection> split;
    std::optional<TabInspection> tabs;
    std::optional<NumericValidation> numeric_validation;
    std::optional<ScrollInspection> scroll;
    std::optional<ListInspection> list;
    struct Tree { std::size_t known_count{}, expanded_count{}, loading_count{}; };
    std::optional<Tree> tree;
    std::optional<TreeItem> tree_item;
    struct Table {
        std::vector<TableColumnWidth> columns;
        std::vector<SortDescriptor> sort;
        TableSelectionMode selection_mode{};
        TableCell active;
        std::optional<TableCell> editing;
        std::string validation;
    };
    std::optional<Table> table;
    Position position{Position::normal};
    Length left{automatic}, top{automatic}, right{automatic}, bottom{automatic};
    int z_index{};
    bool hidden{}, own_hidden{};
    bool secure{}, revealed{}, search_pending{};
    struct Combo { bool open{}, loading{}; std::string selection, highlighted, validation; std::size_t matches{}; };
    std::optional<Combo> combo;
    std::optional<Date> calendar_date;
    struct DateTimeDraft {
        LocalDateTime source,draft;
        DateTimeValidation validation;
    };
    std::optional<DateTimeDraft> date_time;
    struct ColorDraft {
        Color source{}, draft{};
        bool open{}, live_preview{}, alpha{};
        ColorModel model{};
        NumericValidation validation;
    };
    std::optional<ColorDraft> color;
    std::optional<bool> calendar_open;
    std::optional<bool> alert_pending;
    std::optional<AlertActionRole> alert_action;
};
struct HostOptions {
    std::function<std::chrono::steady_clock::time_point()> clock{[] { return std::chrono::steady_clock::now(); }};
};
class Host {
  public:
    explicit Host(std::function<View()> builder, std::function<void()> wake = {}, Theme = {},
                  std::shared_ptr<TextLayoutEngine> text_engine = {}, HostOptions = {});
    ~Host();
    Host(const Host&) = delete;
    Host& operator=(const Host&) = delete;
    void invalidate();
    void set_theme(Theme);
    Theme theme() const;
    void set_clipboard(Clipboard);
    std::optional<TextInputSnapshot> text_input();
    bool insert_text(std::string, std::optional<TextRange> replacement = {});
    bool mark_text(std::string, TextRange selection_inside, std::optional<TextRange> replacement = {});
    void unmark_text(bool commit = true);
    bool text_command(TextCommand, bool extend = false);
    Rect text_range_rect(TextRange);
    std::size_t text_index_at(Point);
    bool set_text(const std::string& id, std::string);
    bool select_text(const std::string& id, TextSelection);
    void layout(Size);
    bool scroll_to(const std::string& id, Point offset);
    std::optional<Point> scroll_offset(const std::string& id);
    std::optional<LogicalItem> list_item(const std::string& id, std::size_t index);
    std::optional<LogicalItem> list_item(const std::string& row_id);
    bool reveal_item(const std::string& id, std::size_t index);
    bool set_expanded(const std::string& row_id, bool expanded);
    std::optional<LogicalItem> tree_child(const std::string& row_id, std::size_t child_index);
    std::optional<LogicalCell> table_cell(const std::string& table_id, const TableCell&);
    std::optional<LogicalCell> table_cell(const std::string& cell_id);
    std::optional<LogicalCell> table_cell_at(const std::string& row_id, std::size_t column);
    std::optional<LogicalColumn> table_column(const std::string& table_id, std::size_t column);
    std::optional<LogicalColumn> table_column(const std::string& column_id);
    std::optional<LogicalCell> table_column_cell(const std::string& column_id, std::size_t row);
    bool set_selected(const std::string& item_id, bool);
    bool set_selection(const std::string& container_id, std::vector<std::string> item_ids);
    bool edit_cell(const std::string& cell_id);
    bool set_cell_text(const std::string& cell_id, std::string);
    bool reveal_cell(const std::string& table_id, const TableCell&);
    bool edit_cell(const std::string& table_id, const TableCell&);
    bool commit_table_edit(const std::string& table_id);
    bool cancel_table_edit(const std::string& table_id);
    void paint(SkCanvas&, double elapsed_seconds = 0);
    bool handle(PointerEvent);
    bool handle(KeyEvent);
    Cursor cursor(Point);
    std::optional<NumericValidation> numeric_validation(const std::string& id);
    std::optional<DateTimeValidation> field_validation(const std::string& id);
    bool commit_field(const std::string& id);
    bool cancel_field(const std::string& id);
    std::optional<NumericValidation> color_validation(const std::string& id);
    bool apply_color(const std::string& id);
    bool cancel_color(const std::string& id);
    bool commit_numeric(const std::string& id);
    bool focus_search(const std::string& id = {});
    bool flush_search(const std::string& id);
    void advance_time();
    std::optional<std::chrono::milliseconds> next_wake();
    void blur();
    void set_active(bool);
    bool needs_frame() const;
    std::vector<Element> elements();
    bool focus(const std::string& id);
    bool reveal(const std::string& id);
    bool activate(const std::string& id);
    bool adjust(const std::string& id, double steps);
    bool set_value(const std::string& id, double value);
    bool popup_open() const;
    Rect popup_bounds() const;
    bool show_context_menu(const std::string& owner, std::optional<Point> = {});
    std::uint64_t generation() const;
    std::vector<InspectionNode> inspect();
    void set_inspected_element(std::string id);
    std::string inspected_element() const;
    void set_inspection_picking(bool);
    bool inspection_picking() const;
    void set_inspection_overlay(InspectionOverlay);
    InspectionOverlay inspection_overlay() const;
    void set_layout_override(const std::string& id, LayoutOverride);
    std::optional<LayoutOverride> layout_override(const std::string& id) const;
    void clear_layout_override(const std::string& id);
    void clear_layout_overrides();
    // Receives invalidation signals on the owning thread. Schedule a refresh; do not
    // mutate the source view model in this callback. Clear when the inspector closes.
    void set_inspection_observer(std::function<void()>);

  private:
    struct Impl;
    std::shared_ptr<Impl> impl_;
};
// Native-independent explorer view, reusable in any SCARS host.
class Inspector {
  public:
    explicit Inspector(std::shared_ptr<Host> source);
    ~Inspector();
    View body();
    void reveal_selection();

  private:
    struct Impl;
    std::shared_ptr<Impl> impl_;
};
} // namespace scars

// Shared implementation contracts for portable numeric controls. These declarations
// are available to SCARS module implementation units and are not exported to clients.
namespace scars::detail {
using NumericValue = std::variant<std::int64_t, double>;
struct ParsedNumber {
    std::optional<NumericValue> value;
    NumericValidation validation;
};
struct NumericConfig {
    NumericValue min, max, step;
    std::string locale;
    int significant_digits{};
    bool grouping{}, commit_on_blur{}, read_only{};
    std::function<void(const NumericValidation&)> on_validation;
    std::function<void()> on_submit;
    std::chrono::milliseconds repeat_delay{}, repeat_interval{};
    std::string format_id;
    std::function<std::string(NumericValue)> format;
    std::function<ParsedNumber(const std::string&)> parse;
    std::function<NumericValidation(NumericValue)> validate;
    std::function<std::optional<NumericValue>(NumericValue, double, std::size_t)> step_value;
};
void check_numeric(const NumericConfig&);
std::string format_number(NumericValue, const NumericConfig&);
ParsedNumber parse_number(const std::string&, const NumericConfig&);
std::optional<NumericValue> step_number(NumericValue, double steps, const NumericConfig&, std::size_t caret = 0);
std::int64_t date_days(Date);
Date date_from_days(std::int64_t);
std::int64_t time_seconds(Time);
Time time_from_seconds(std::int64_t);
Date shift_month(Date, int months);
NumericConfig date_codec(DatePickerOptions);
NumericConfig time_codec(TimePickerOptions);
struct CalendarPage {
    std::string title;
    std::vector<std::string> weekdays, day_labels;
    std::vector<std::optional<Date>> days;
    unsigned first_weekday{};
};
CalendarPage calendar_page(Date, const std::string& locale);
std::string search_key(const std::string&);
} // namespace scars::detail
