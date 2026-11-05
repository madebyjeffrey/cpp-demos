module;
#include <algorithm>
#include <cmath>
#include <format>
#include <functional>
#include <memory>
#include <limits>
#include <stdexcept>
#include <string>
#include <unordered_set>
#include <unordered_map>
#include <utility>
#include <vector>
module scars;

namespace scars {
namespace {
std::string number(double value) {
    return std::isfinite(value) ? std::format("{:.1f}", value) : "unbounded";
}
std::string length(Length value) {
    if (value.unit == Length::Unit::automatic)
        return "auto";
    return number(value.value * (value.unit == Length::Unit::percent ? 100 : 1)) +
           (value.unit == Length::Unit::percent ? "%" : " lp");
}
std::string rectangle(Rect r) {
    return std::format("x {}  y {}  /  {} × {}", number(r.x), number(r.y), number(r.width), number(r.height));
}
std::string insets(Insets i) {
    return std::format("{}  {}  {}  {}", number(i.top), number(i.right), number(i.bottom), number(i.left));
}
std::string alignment(Align value) {
    switch (value) {
    case Align::start:
        return "start";
    case Align::center:
        return "center";
    case Align::end:
        return "end";
    case Align::stretch:
        return "stretch";
    case Align::baseline:
        return "baseline";
    }
    return {};
}
std::string justification(Justify value) {
    switch (value) {
    case Justify::start:
        return "start";
    case Justify::center:
        return "center";
    case Justify::end:
        return "end";
    case Justify::space_between:
        return "space-between";
    case Justify::space_around:
        return "space-around";
    }
    return {};
}
View property(std::string name, std::string value) {
    return FlexRow{Label(std::move(name), LabelStyle::secondary).width(94).shrink(0),
                   Label(std::move(value), LabelStyle::secondary).text_wrap(TextWrap::break_word).grow()}
        .gap(10)
        .align_items(Align::start);
}
View properties(std::string title, std::vector<View> rows) {
    return Section(std::move(title), FlexColumn(std::move(rows)).gap(8));
}
bool container(const InspectionNode& n) {
    return n.type == "Layer" || n.type == "FlexRow" || n.type == "FlexColumn" || n.type == "Block" ||
           n.type == "Inline" || n.type == "ScrollView" || n.type == "Section" || n.type == "GroupBox" ||
           n.type == "Panel";
}
// Edits always start from the latest patch so independent controls compose.
void edit(const std::shared_ptr<Host>& source, const std::string& id,
          const std::function<void(LayoutOverride&)>& change) {
    const auto snapshot = source->inspect();
    if (std::none_of(snapshot.begin(), snapshot.end(),
                     [&](const auto& n) { return n.id == id && n.layout_editable; }))
        return;
    auto value = source->layout_override(id).value_or(LayoutOverride{});
    change(value);
    source->set_layout_override(id, std::move(value));
}
View stepper(std::string name, double value, std::function<void(double)> write, double step = 1,
             double minimum = 0) {
    return FlexRow{Label(name, LabelStyle::secondary).grow().min_width(64),
                   Button("−", [write, value, step, minimum] { write(std::max(minimum, value - step)); })
                       .width(36)
                       .height(28)
                       .font_size(12)
                       .corner_radius(4)
                       .accessible_label("Decrease " + name),
                   Label(number(value), LabelStyle::secondary).width(52).text_align(Align::center),
                   Button("+", [write, value, step] { write(value + step); })
                       .width(36)
                       .height(28)
                       .font_size(12)
                       .corner_radius(4)
                       .accessible_label("Increase " + name)}
        .gap(4)
        .align_items(Align::center);
}
View scalar(std::string name, double value, std::function<void(double)> write, double step = 1,
            double maximum = 128, bool signed_values = false) {
    const double extent = std::max(maximum, std::abs(value) * 1.5);
    return FlexColumn{stepper(name, value, write, step, signed_values ? -infinity : 0),
                      Slider(Binding<double>([value] { return value; }, std::move(write)),
                             {signed_values ? -extent : 0, extent, step, step * 10})
                          .height(24)
                          .accessible_label("Edit " + name)}
        .gap(2);
}
View length_editor(const std::shared_ptr<Host>& source, const InspectionNode& n, std::string name,
                   Length value, double resolved, std::optional<Length> LayoutOverride::* member,
                   bool signed_values = false) {
    const std::string unit = value.unit == Length::Unit::automatic ? "auto"
                             : value.unit == Length::Unit::percent ? "%"
                                                                   : "lp";
    Binding<std::string> binding(
        [unit] { return unit; },
        [source, id = n.id, value, resolved, member](std::string next) {
            Length length = next == "auto" ? automatic : next == "%" ? percent(100) : Length(resolved);
            if ((next == "%" && value.unit == Length::Unit::percent) ||
                (next == "lp" && value.unit == Length::Unit::logical))
                length = value;
            edit(source, id, [&](auto& patch) { patch.*member = length; });
        });
    std::vector<View> rows{FlexRow{Label(name, LabelStyle::secondary).grow(),
                                   SelectionDropdown(binding, {{"auto", "Auto"}, {"lp", "lp"}, {"%", "%"}})
                                       .density(Density::compact)
                                       .width(112)
                                       .accessible_label(name + " unit")}
                               .gap(8)
                               .align_items(Align::center)};
    if (unit != "auto") {
        const auto display = value.value * (unit == "%" ? 100 : 1);
        rows.push_back(scalar(
            name, display,
            [source, id = n.id, unit, member](double next) {
                edit(source, id,
                     [&](auto& patch) { patch.*member = unit == "%" ? percent(next) : Length(next); });
            },
            1, unit == "%" ? 100 : 512, signed_values));
    }
    return FlexColumn(std::move(rows)).gap(4);
}
View spacing_editor(const std::shared_ptr<Host>& source, const InspectionNode& n, std::string name,
                    Insets value, std::optional<Insets> LayoutOverride::* member) {
    std::vector<View> rows;
    const std::vector<std::pair<std::string, double Insets::*>> sides{{"top", &Insets::top},
                                                                      {"right", &Insets::right},
                                                                      {"bottom", &Insets::bottom},
                                                                      {"left", &Insets::left}};
    for (const auto& [side, field] : sides)
        rows.push_back(
            stepper(name + " " + side, value.*field, [source, id = n.id, value, member, field](double next) {
                edit(source, id, [&](auto& patch) {
                    auto insets = (patch.*member).value_or(value);
                    insets.*field = next;
                    patch.*member = insets;
                });
            }));
    return FlexColumn(std::move(rows)).gap(4);
}
View box_model(const InspectionNode& n) {
    auto caption = [](std::string text, Color color) {
        return Label(std::move(text), LabelStyle::secondary).foreground(color).text_align(Align::center);
    };
    auto content = FlexColumn{caption("CONTENT", 0xff4dd2ff), caption(number(n.content_bounds.width) + " × " +
                                                                          number(n.content_bounds.height),
                                                                      0xffe6f0ff)}
                       .gap(4)
                       .padding(12)
                       .background(0xff163c50)
                       .grow();
    auto padding = FlexColumn{caption("PADDING  " + number(n.padding.top), 0xff70e5b9),
                              FlexRow{caption(number(n.padding.left), 0xff70e5b9).width(40), content,
                                      caption(number(n.padding.right), 0xff70e5b9).width(40)}
                                  .gap(4)
                                  .align_items(Align::center),
                              caption(number(n.padding.bottom), 0xff70e5b9)}
                       .gap(6)
                       .padding(8)
                       .background(0xff214638)
                       .grow();
    return FlexColumn{FlexColumn{caption("MARGIN  " + number(n.margin.top), 0xffffc174),
                                 FlexRow{caption(number(n.margin.left), 0xffffc174).width(32), padding,
                                         caption(number(n.margin.right), 0xffffc174).width(32)}
                                     .gap(4)
                                     .align_items(Align::center),
                                 caption(number(n.margin.bottom), 0xffffc174)}
                          .gap(6)
                          .padding(8)
                          .background(0xff493b28)
                          .corner_radius(6),
                      Label("Schematic · logical pixels · spacing is not drawn to scale", LabelStyle::micro)
                          .text_wrap(TextWrap::wrap)}
        .gap(8);
}
} // namespace
struct Inspector::Impl : std::enable_shared_from_this<Inspector::Impl> {
    std::shared_ptr<Host> source;
    State<std::vector<std::string>> collapsed;
    State<std::string> filter{"all"};
    State<std::string> tab{"layout"};
    State<double> pane_fraction{.42};
    TreeModel element_tree;
    std::vector<std::string> tree_signature;
    explicit Impl(std::shared_ptr<Host> host) : source(std::move(host)) {}
    View facts(const InspectionNode* node) {
        if (!node)
            return FlexColumn{
                Label("Select an element", LabelStyle::section),
                Label("Choose a row in the tree, or enable Pick element and click a view in the application.",
                      LabelStyle::secondary)
                    .text_wrap(TextWrap::wrap),
                Label("Layout edits are temporary and can be reset. Picking does not activate controls.",
                      LabelStyle::secondary)
                    .text_wrap(TextWrap::wrap)}
                .gap(16);
        const auto& n = *node;
        std::vector<View> state{property("Enabled", n.enabled ? "true" : "false"),
                                property("Focusable", n.interactive ? "true" : "false"),
                                property("Focused", n.focused ? "true" : "false"),
                                property("Selected", n.selected ? "true" : "false"),
                                property("Expanded", n.expanded ? "true" : "false")};
        if (n.value)
            state.push_back(property("Value", number(*n.value)));
        if (!n.selection.empty())
            state.push_back(property(n.combo ? "Query" : "Selection", n.selection));
        if (n.combo) {
            state.push_back(property("Accepted ID", n.combo->selection.empty() ? "(none)" : n.combo->selection));
            state.push_back(property("Suggestion highlight", n.combo->highlighted.empty() ? "(none)" : n.combo->highlighted));
            state.push_back(property("Matching options", std::to_string(n.combo->matches)));
            state.push_back(property("Suggestions", n.combo->open ? "open" : "closed"));
            state.push_back(property("Loading", n.combo->loading ? "true" : "false"));
            if (!n.combo->validation.empty()) state.push_back(property("Validation", n.combo->validation));
        }
        if (n.split) {
            const auto& split = *n.split;
            state.push_back(property("Split axis", split.orientation == Orientation::horizontal ? "horizontal" : "vertical"));
            state.push_back(property("Split fraction", number(split.fraction)));
            state.push_back(property("Pane extents", number(split.first_extent) + " / " + number(split.second_extent)));
            state.push_back(property("Pane minima", number(split.first_min) + " / " + number(split.second_min)));
            state.push_back(property("Divider extent", number(split.divider_extent)));
            state.push_back(property("Collapsed pane", split.collapsed == SplitPane::first ? "first" :
                                                       split.collapsed == SplitPane::second ? "second" : "none"));
        }
        if (n.tabs) {
            state.push_back(property("Requested tab", n.tabs->requested_selection));
            state.push_back(property("Tabs / visited", std::to_string(n.tabs->tab_count) + " / " +
                                                       std::to_string(n.tabs->visited_count)));
            state.push_back(property("Tab strip scroll", number(n.tabs->strip_offset)));
        }
        if (n.scroll) {
            const auto& scroll = *n.scroll;
            state.push_back(property("Scroll axes", scroll.axes == ScrollAxes::both ? "both" :
                                                      scroll.axes == ScrollAxes::horizontal ? "horizontal" : "vertical"));
            state.push_back(property("Scroll offset", number(scroll.offset.x) + " / " + number(scroll.offset.y)));
            state.push_back(property("Maximum offset", number(scroll.maximum.x) + " / " + number(scroll.maximum.y)));
            state.push_back(property("Document size", number(scroll.content.width) + " × " + number(scroll.content.height)));
            state.push_back(property("Scroll viewport", number(scroll.viewport.width) + " × " + number(scroll.viewport.height)));
        }
        if (n.list) {
            const auto& list = *n.list;
            state.push_back(property("Logical / mounted rows", std::to_string(list.logical_count) + " / " + std::to_string(list.mounted_count)));
            state.push_back(property("Visible range [begin, end)", std::to_string(list.window.visible_begin) + " / " + std::to_string(list.window.visible_end)));
            state.push_back(property("Overscan range [begin, end)", std::to_string(list.window.mounted_begin) + " / " + std::to_string(list.window.mounted_end)));
            state.push_back(property("Active row", list.active.empty() ? "(none)" : list.active));
            state.push_back(property("Pinned editor row", list.pinned.empty() ? "(none)" : list.pinned));
            state.push_back(property("Retained editors", std::to_string(list.retained_editors)));
        }
        if (n.alert_pending)
            state.push_back(property("Alert transaction", *n.alert_pending ? "pending" : "completed"));
        if (n.alert_action)
            state.push_back(property("Alert action", *n.alert_action == AlertActionRole::default_action ? "default" :
                *n.alert_action == AlertActionRole::cancel ? "cancel" :
                *n.alert_action == AlertActionRole::destructive ? "destructive" : "normal"));
        if(n.table){
            state.push_back(property("Active cell",n.table->active.row+" / "+n.table->active.column));
            state.push_back(property("Selection mode",n.table->selection_mode==TableSelectionMode::cells?"cells":"rows"));
            for(const auto& column:n.table->columns)state.push_back(property("Column "+column.column,number(column.width)));
            for(const auto& sort:n.table->sort)state.push_back(property("Sort "+sort.column,sort.order==SortOrder::ascending?"ascending":"descending"));
            if(n.table->editing)state.push_back(property("Editing cell",n.table->editing->row+" / "+n.table->editing->column));
            if(!n.table->validation.empty())state.push_back(property("Validation",n.table->validation));
        }
        if (n.tree) {
            state.push_back(property("Known tree nodes", std::to_string(n.tree->known_count)));
            state.push_back(property("Expanded / loading", std::to_string(n.tree->expanded_count) + " / " + std::to_string(n.tree->loading_count)));
        }
        if (n.tree_item) {
            state.push_back(property("Tree parent", n.tree_item->parent.empty() ? "(root)" : n.tree_item->parent));
            state.push_back(property("Tree depth", std::to_string(n.tree_item->depth)));
            state.push_back(property("Expandable / expanded", std::string(n.tree_item->expandable ? "yes" : "no") + " / " + (n.tree_item->expanded ? "yes" : "no")));
        }
        if (n.color) {
            const auto& color = *n.color;
            state.push_back(property("Color source", format_color(color.source)));
            state.push_back(property("Color draft", format_color(color.draft)));
            state.push_back(property("Color editing", color.model == ColorModel::hsv ? "HSV" : "RGB"));
            state.push_back(
                property("Color publication", color.live_preview ? "live preview" : "Apply / Cancel"));
            state.push_back(property("Alpha editing", color.alpha ? "enabled" : "preserved"));
            state.push_back(property("Color session", color.open               ? "popover open"
                                                      : color.validation.dirty ? "pending"
                                                                               : "unchanged"));
            if (!color.validation.message.empty())
                state.push_back(property("Color validation", color.validation.message));
        }
        if (n.date_time) {
            const auto& draft = *n.date_time;
            state.push_back(
                property("Committed date/time", format_date(draft.source.date) + " " +
                                                    format_time(draft.source.time, {.show_seconds = true})));
            state.push_back(
                property("Component draft", format_date(draft.draft.date) + " " +
                                                format_time(draft.draft.time, {.show_seconds = true})));
            state.push_back(property("Joint draft", draft.validation.dirty ? "pending" : "unchanged"));
            if (!draft.validation.message.empty())
                state.push_back(property("Joint validation", draft.validation.message));
        }
        if (n.calendar_open)
            state.push_back(property("Calendar", *n.calendar_open ? "open" : "closed"));
        if (n.calendar_date)
            state.push_back(property("Calendar date", format_date(*n.calendar_date)));
        if (n.numeric_validation) {
            const auto& value = *n.numeric_validation;
            state.push_back(property(n.type=="DatePicker"||n.type=="TimePicker"?"Date/time validation":"Numeric validation", value.status == NumericStatus::valid ? "valid" :
                value.status == NumericStatus::incomplete ? "incomplete" :
                value.status == NumericStatus::out_of_range ? "out of range" : "invalid"));
            state.push_back(property("Pending draft", value.dirty ? "true" : "false"));
            if (!value.message.empty())
                state.push_back(property("Validation message", value.message));
        }
        if (n.secure) {
            state.push_back(property("Password value", "redacted"));
            state.push_back(property("Visual reveal", n.revealed ? "true" : "false"));
        }
        if (n.type == "SearchField")
            state.push_back(property("Search callback", n.search_pending ? "pending" : "idle"));
        return FlexColumn{
            Label(n.type, LabelStyle::heading).foreground(source->theme().cyan),
            properties("Identity", {property("Key", n.key.empty() ? "(structural)" : n.key),
                                    property("Text", n.text.empty() ? "—" : n.text),
                                    property("Label", n.label.empty() ? "—" : n.label), property("ID", n.id),
                                    property("Parent", n.parent_id.empty() ? "(root)" : n.parent_id)}),
            Separator{},
            properties("Layout",
                       {property("Bounds", rectangle(n.bounds)), property("Clip", rectangle(n.clip)),
                        property("Measured", number(n.measured.width) + " × " + number(n.measured.height)),
                        property("Position", n.position == Position::absolute   ? "absolute"
                                             : n.position == Position::relative ? "relative"
                                                                                : "normal"),
                        property("Z-index", std::to_string(n.z_index)),
                        property("Hidden", n.hidden ? "true" : "false"),
                        property("Left / top", length(n.left) + " / " + length(n.top)),
                        property("Right / bottom", length(n.right) + " / " + length(n.bottom)),
                        property("Width", length(n.width)), property("Height", length(n.height)),
                        property("Min", number(n.min_width) + " × " + number(n.min_height)),
                        property("Max", number(n.max_width) + " × " + number(n.max_height)),
                        property("Padding", insets(n.padding)), property("Margin", insets(n.margin)),
                        property("Gap", number(n.gap)),
                        property("Grow / shrink", number(n.grow) + " / " + number(n.shrink)),
                        property("Basis", length(n.basis)), property("Align", alignment(n.align)),
                        property("Justify", justification(n.justify))}),
            Separator{},
            properties("Style", {property("Font size", number(n.font_size) + " lp"),
                                 property("Line height", number(n.line_height) + " lp"),
                                 property("Tracking", number(n.letter_spacing)),
                                 property("Weight", n.bold ? "bold" : "regular"),
                                 property("Radius", number(n.corner_radius) + " lp"),
                                 property("Foreground", std::format("#{:08X}", n.foreground)),
                                 property("Accent", std::format("#{:08X}", n.accent)),
                                 property("Background", n.background ? std::format("#{:08X}", *n.background)
                                                                     : "transparent / inherited")}),
            Separator{},
            properties("State", std::move(state))}
            .gap(20);
    }
    View layout_editor(const InspectionNode& n) {
        Binding<bool> overlay(
            [source = source] { return source->inspection_overlay() == InspectionOverlay::box_model; },
            [source = source](bool value) {
                source->set_inspection_overlay(value ? InspectionOverlay::box_model
                                                     : InspectionOverlay::selection);
            });
        std::vector<View> rows{
            ToggleButton("Show box model overlay", overlay).density(Density::compact),
            Label("Box model", LabelStyle::section),
            box_model(n),
            property("Bounds", rectangle(n.bounds)),
            length_editor(source, n, "Width", n.width, n.bounds.width, &LayoutOverride::width),
            length_editor(source, n, "Height", n.height, n.bounds.height, &LayoutOverride::height),
            Separator{},
            Label("Spacing", LabelStyle::section),
            spacing_editor(source, n, "Padding", n.padding, &LayoutOverride::padding),
            Separator{},
            spacing_editor(source, n, "Margin", n.margin, &LayoutOverride::margin)};
        Binding<std::string> position(
            [value = n.position] {
                return value == Position::absolute   ? "absolute"
                       : value == Position::relative ? "relative"
                                                     : "normal";
            },
            [source = source, id = n.id](std::string value) {
                edit(source, id, [&](auto& p) {
                    p.position = value == "absolute"   ? Position::absolute
                                 : value == "relative" ? Position::relative
                                                       : Position::normal;
                });
            });
        Binding<bool> hidden([value = n.own_hidden] { return value; },
                             [source = source, id = n.id](bool value) {
                                 edit(source, id, [&](auto& p) { p.hidden = value; });
                             });
        rows.push_back(Separator{});
        rows.push_back(Label("Position & stacking", LabelStyle::section));
        rows.push_back(
            SelectionDropdown(position,
                              {{"normal", "Normal flow"}, {"relative", "Relative"}, {"absolute", "Absolute"}})
                .density(Density::compact)
                .accessible_label("Position mode"));
        rows.push_back(stepper(
            "Z-index", n.z_index,
            [source = source, id = n.id](double value) {
                edit(source, id, [&](auto& p) {
                    p.z_index = static_cast<int>(std::clamp(value, double(std::numeric_limits<int>::min()),
                                                            double(std::numeric_limits<int>::max())));
                });
            },
            1, double(std::numeric_limits<int>::min())));
        rows.push_back(CheckBox("Hide element", hidden));
        if (n.hidden && !n.own_hidden)
            rows.push_back(Label("Hidden by an ancestor. Select that ancestor to show this element.",
                                 LabelStyle::secondary)
                               .text_wrap(TextWrap::wrap));
        if (n.position != Position::normal) {
            rows.push_back(length_editor(source, n, "Left", n.left, 0, &LayoutOverride::left, true));
            rows.push_back(length_editor(source, n, "Top", n.top, 0, &LayoutOverride::top, true));
            rows.push_back(length_editor(source, n, "Right", n.right, 0, &LayoutOverride::right, true));
            rows.push_back(length_editor(source, n, "Bottom", n.bottom, 0, &LayoutOverride::bottom, true));
        }
        return FlexColumn(std::move(rows)).gap(14);
    }
    View flex_editor(const InspectionNode& n, const std::vector<InspectionNode>& snapshot) {
        const bool is_flex = n.type == "FlexRow" || n.type == "FlexColumn";
        const auto parent = std::find_if(snapshot.begin(), snapshot.end(),
                                         [&](const auto& p) { return p.id == n.parent_id; });
        const bool flex_child = n.position != Position::absolute && parent != snapshot.end() &&
                                (parent->type == "FlexRow" || parent->type == "FlexColumn");
        auto id = n.id;
        std::vector<View> rows;
        if (is_flex || flex_child) {
            Binding<bool> overlay(
                [source = source] { return source->inspection_overlay() == InspectionOverlay::flex; },
                [source = source](bool value) {
                    source->set_inspection_overlay(value ? InspectionOverlay::flex
                                                         : InspectionOverlay::selection);
                });
            rows.push_back(ToggleButton("Show flex debug overlay", overlay).density(Density::compact));
        }
        if (is_flex) {
            auto direction = n.type == "FlexRow" ? "row" : "column";
            Binding<std::string> binding([direction] { return std::string(direction); },
                                         [source = source, id](std::string value) {
                                             edit(source, id, [&](auto& p) {
                                                 p.direction = value == "row" ? Orientation::horizontal
                                                                              : Orientation::vertical;
                                             });
                                         });
            Binding<bool> wrap(
                [value = n.wrap] { return value; },
                [source = source, id](bool value) { edit(source, id, [&](auto& p) { p.wrap = value; }); });
            rows.push_back(Label("Flex container", LabelStyle::section));
            rows.push_back(SelectionDropdown(binding, {{"row", "Row →"}, {"column", "Column ↓"}})
                               .density(Density::compact)
                               .accessible_label("Flex direction"));
            rows.push_back(CheckBox("Wrap rows", wrap).enabled(n.type == "FlexRow"));
            if (n.type == "FlexColumn")
                rows.push_back(
                    Label("Column wrapping is not supported by this engine.", LabelStyle::secondary)
                        .text_wrap(TextWrap::wrap));
            rows.push_back(scalar("Gap", n.gap, [source = source, id](double v) {
                edit(source, id, [&](auto& p) { p.gap = v; });
            }));
            Binding<std::string> align([value = n.align] { return alignment(value); },
                                       [source = source, id](std::string v) {
                                           Align value = v == "start"      ? Align::start
                                                         : v == "center"   ? Align::center
                                                         : v == "end"      ? Align::end
                                                         : v == "baseline" ? Align::baseline
                                                                           : Align::stretch;
                                           edit(source, id, [&](auto& p) { p.align = value; });
                                       });
            rows.push_back(Label("Align items · cross axis", LabelStyle::secondary));
            rows.push_back(SelectionDropdown(align, {{"start", "Start"},
                                                     {"center", "Center"},
                                                     {"end", "End"},
                                                     {"stretch", "Stretch"},
                                                     {"baseline", "Baseline"}})
                               .density(Density::compact)
                               .accessible_label("Align items"));
            Binding<std::string> justify([value = n.justify] { return justification(value); },
                                         [source = source, id](std::string v) {
                                             Justify value = v == "start"           ? Justify::start
                                                             : v == "center"        ? Justify::center
                                                             : v == "end"           ? Justify::end
                                                             : v == "space-between" ? Justify::space_between
                                                                                    : Justify::space_around;
                                             edit(source, id, [&](auto& p) { p.justify = value; });
                                         });
            rows.push_back(Label("Justify · main axis", LabelStyle::secondary));
            rows.push_back(SelectionDropdown(justify, {{"start", "Start"},
                                                       {"center", "Center"},
                                                       {"end", "End"},
                                                       {"space-between", "Space between"},
                                                       {"space-around", "Space around"}})
                               .density(Density::compact)
                               .accessible_label("Justify content"));
            rows.push_back(Separator{});
            rows.push_back(Label("Resolved flex lines", LabelStyle::section));
            std::size_t line_index = 0;
            for (const auto& line : n.flex_lines)
                rows.push_back(
                    property("Line " + std::to_string(++line_index),
                             std::format("{} items · gap {} · {} {}", line.items.size(), number(line.gap),
                                         line.free_space < 0 ? "overflow" : "free space",
                                         number(std::abs(line.free_space)))));
            std::size_t index = 0;
            for (const auto& child : snapshot) {
                if (child.parent_id != n.id || child.position == Position::absolute)
                    continue;
                auto caption =
                    std::format("{}  {}{} · {} × {} · grow {} / shrink {}", ++index, child.type,
                                child.key.empty() ? "" : " #" + child.key, number(child.bounds.width),
                                number(child.bounds.height), number(child.grow), number(child.shrink));
                rows.push_back(
                    Button(caption, [source = source, id = child.id] { source->set_inspected_element(id); })
                        .density(Density::compact)
                        .text_align(Align::start)
                        .background(source->theme().inactive)
                        .accessible_label("Inspect flex child " + std::to_string(index)));
            }
        }
        if (flex_child) {
            rows.push_back(Label("Flex item", LabelStyle::section));
            rows.push_back(Button("Inspect parent " + parent->type, [source = source, id = parent->id] {
                               source->set_inspected_element(id);
                           }).density(Density::compact));
            rows.push_back(scalar(
                "Grow", n.grow,
                [source = source, id](double v) { edit(source, id, [&](auto& p) { p.grow = v; }); }, .1, 5));
            rows.push_back(scalar(
                "Shrink", n.shrink,
                [source = source, id](double v) { edit(source, id, [&](auto& p) { p.shrink = v; }); }, .1,
                5));
            rows.push_back(length_editor(source, n, "Basis", n.basis,
                                         parent->type == "FlexRow" ? n.bounds.width : n.bounds.height,
                                         &LayoutOverride::basis));
            Binding<std::string> self([value = n.self] { return value ? alignment(*value) : "inherit"; },
                                      [source = source, id](std::string v) {
                                          if (v == "inherit") {
                                              edit(source, id, [](auto& p) { p.self.reset(); });
                                          } else {
                                              Align value = v == "start"      ? Align::start
                                                            : v == "center"   ? Align::center
                                                            : v == "end"      ? Align::end
                                                            : v == "baseline" ? Align::baseline
                                                                              : Align::stretch;
                                              edit(source, id, [&](auto& p) { p.self = value; });
                                          }
                                      });
            rows.push_back(Label("Align self · cross axis", LabelStyle::secondary));
            rows.push_back(SelectionDropdown(self, {{"inherit", "Application default"},
                                                    {"start", "Start"},
                                                    {"center", "Center"},
                                                    {"end", "End"},
                                                    {"stretch", "Stretch"},
                                                    {"baseline", "Baseline"}})
                               .density(Density::compact)
                               .accessible_label("Align self"));
        }
        if (!is_flex && !flex_child)
            rows.push_back(
                Label("Select a FlexRow, FlexColumn, or one of its direct children to edit flex properties.",
                      LabelStyle::secondary)
                    .text_wrap(TextWrap::wrap));
        return FlexColumn(std::move(rows)).gap(12);
    }
    View details(const InspectionNode* node, const std::vector<InspectionNode>& snapshot) {
        if (!node)
            return facts(nullptr);
        const auto& n = *node;
        const auto active_tab = tab.get();
        auto content = active_tab == "details" ? facts(node)
                       : active_tab == "flex"  ? flex_editor(n, snapshot)
                                               : layout_editor(n);
        return FlexColumn{
            Label(n.type + (n.key.empty() ? "" : " #" + n.key), LabelStyle::section)
                .foreground(source->theme().cyan),
            SegmentedControl(tab.binding(), {{"layout", "Layout"}, {"flex", "Flex"}, {"details", "Details"}})
                .density(Density::compact)
                .accessible_label("Inspector properties"),
            FlexRow{Label(n.layout_overridden ? "Temporary edits active" : "Application values",
                          LabelStyle::secondary)
                        .grow(),
                    Button("Reset", [source = source, id = n.id] { source->clear_layout_override(id); })
                        .density(Density::compact)
                        .enabled(n.layout_overridden)
                        .accessible_label("Reset selected layout")}
                .gap(8)
                .align_items(Align::center),
            n.layout_editable || active_tab == "details"
                ? content.key(active_tab)
                : Label("This region is generated by its control. Select the parent to edit layout.",
                        LabelStyle::secondary)
                      .text_wrap(TextWrap::wrap)}
            .gap(12);
    }
    void reveal_selection() {
        auto snapshot = source->inspect();
        auto id = source->inspected_element();
        auto next = collapsed.get();
        while (!id.empty()) {
            auto n =
                std::find_if(snapshot.begin(), snapshot.end(), [&](const auto& n) { return n.id == id; });
            if (n == snapshot.end())
                break;
            std::erase(next, n->parent_id);
            id = n->parent_id;
        }
        collapsed.set(std::move(next));
        filter.set("all");
    }
    View body() {
        auto snapshot = source->inspect();
        const auto selection = source->inspected_element();
        auto selected =
            std::find_if(snapshot.begin(), snapshot.end(), [&](const auto& n) { return n.id == selection; });
        const auto active_filter = filter.get();
        std::unordered_map<std::string, const InspectionNode*> by_id;
        for (const auto& node : snapshot) by_id.emplace(node.id, &node);
        std::unordered_set<std::string> included;
        for (const auto& node : snapshot) {
            if (active_filter == "controls" && !node.interactive) continue;
            if (active_filter == "layout" && !container(node)) continue;
            auto* current = &node;
            while (current && included.insert(current->id).second) {
                const auto parent = by_id.find(current->parent_id);
                current = parent == by_id.end() ? nullptr : parent->second;
            }
        }
        std::vector<std::string> signature, parents;
        std::unordered_map<std::string, std::vector<TreeNode>> children;
        std::vector<TreeNode> roots;
        for (auto i = snapshot.rbegin(); i != snapshot.rend(); ++i) {
            const auto& node = *i;
            if (!included.contains(node.id)) continue;
            std::string caption = node.type + (node.hidden ? " (hidden)" : "");
            if (!node.key.empty()) caption += " #" + node.key;
            if (node.type == "Layer" || node.z_index != 0) caption += " [z " + std::to_string(node.z_index) + "]";
            if (node.position != Position::normal) caption += node.position == Position::absolute ? " · absolute" : " · relative";
            if (!node.text.empty() || !node.label.empty()) {
                auto label = node.text.empty() ? node.label : node.text;
                std::replace(label.begin(), label.end(), '\n', ' ');
                caption += "  " + label;
            }
            signature.push_back(node.id); signature.push_back(node.parent_id); signature.push_back(caption);
            TreeNode item; item.id = node.id; item.label = std::move(caption);
            item.accessible_label = "Inspect " + node.type + (node.key.empty() ? "" : " " + node.key);
            item.children = std::move(children[node.id]);
            std::reverse(item.children.begin(), item.children.end());
            if (!item.children.empty()) parents.push_back(node.id);
            if (included.contains(node.parent_id)) children[node.parent_id].push_back(std::move(item));
            else roots.push_back(std::move(item));
        }
        if (signature != tree_signature) {
            std::reverse(roots.begin(), roots.end());
            element_tree = TreeModel(std::move(roots)); tree_signature = std::move(signature);
        }
        std::sort(parents.begin(), parents.end());
        auto self = shared_from_this();
        Binding<std::vector<std::string>> expansion(
            [self, parents] {
                const auto& hidden = self->collapsed.get();
                std::vector<std::string> open;
                for (const auto& id : parents) if (std::find(hidden.begin(), hidden.end(), id) == hidden.end()) open.push_back(id);
                return open;
            },
            [self, parents](std::vector<std::string> open) {
                auto folded = self->collapsed.get();
                std::erase_if(folded, [&](const auto& id) { return std::binary_search(parents.begin(), parents.end(), id); });
                const std::unordered_set<std::string> expanded(open.begin(), open.end());
                for (const auto& id : parents) if (!expanded.contains(id)) folded.push_back(id);
                self->collapsed.set(std::move(folded));
            });
        Binding<std::string> tree_selection(
            [source = source] { return source->inspected_element(); },
            [source = source](std::string id) {
                const auto snapshot = source->inspect();
                if (std::any_of(snapshot.begin(), snapshot.end(), [&](const auto& node) { return node.id == id; })) source->set_inspected_element(std::move(id));
            });
        auto tree = TreeView(std::move(tree_selection), element_tree, std::move(expansion),
                             {.row_height = 28, .visible_rows = 16, .indentation = 12})
                        .key("element-tree").font_size(12).accessible_label("Element tree")
                        .background(source->theme().panel).grow();
        Binding<bool> picking([source = source] { return source->inspection_picking(); },
                              [source = source](bool value) { source->set_inspection_picking(value); });
        Binding<std::string> overlay(
            [source = source] {
                switch (source->inspection_overlay()) {
                case InspectionOverlay::box_model:
                    return std::string("box");
                case InspectionOverlay::flex:
                    return std::string("flex");
                default:
                    return std::string("selection");
                }
            },
            [source = source](std::string value) {
                source->set_inspection_overlay(value == "box"    ? InspectionOverlay::box_model
                                               : value == "flex" ? InspectionOverlay::flex
                                                                 : InspectionOverlay::selection);
            });
        return FlexColumn{
            FlexColumn{
                FlexRow{Label("SCARS Developer Tools", LabelStyle::heading).grow(),
                        Label("⌘⌥I", LabelStyle::micro)}
                    .align_items(Align::center)
                    .gap(12),
                FlexRow{ToggleButton("Pick element", picking)
                            .accent(source->theme().cyan)
                            .density(Density::compact)
                            .accessible_label("Pick an element in the application"),
                        Button("Refresh", [source = source] { source->invalidate(); })
                            .density(Density::compact)
                            .accent(source->theme().cyan),
                        SelectionDropdown(filter.binding(), {{"all", "All elements"},
                                                             {"controls", "Interactive controls"},
                                                             {"layout", "Layout containers"}})
                            .density(Density::compact)
                            .min_width(180)
                            .accessible_label("Filter elements")
                            .accent(source->theme().cyan)}
                    .gap(10)
                    .wrap(),
                FlexRow{SelectionDropdown(overlay, {{"selection", "Selection overlay"},
                                                    {"box", "Box model overlay"},
                                                    {"flex", "Flex debug overlay"}})
                            .density(Density::compact)
                            .width(210)
                            .accessible_label("Layout overlay"),
                        Button("Reset all edits", [source = source] { source->clear_layout_overrides(); })
                            .density(Density::compact)
                            .accessible_label("Reset all layout overrides")}
                    .gap(10)
                    .wrap(),
                Label(source->inspection_overlay() == InspectionOverlay::flex
                          ? "Flex debug · pink: main axis · cyan: cross axis / gaps · purple: numbered items "
                            "· orange: margins"
                      : source->inspection_overlay() == InspectionOverlay::box_model
                          ? "Box model · orange: margin · green: padding · cyan: content"
                      : source->inspection_picking()
                          ? "Click a view in the application to inspect it. Escape cancels."
                          : "Live mounted views · Select an element to inspect its layout, style, and state.",
                      LabelStyle::secondary)
                    .text_wrap(TextWrap::wrap)}
                .gap(10)
                .shrink(0),
            SplitView(Panel(FlexColumn{Label("ELEMENTS", LabelStyle::micro).shrink(0),
                                     tree}
                              .gap(10)),
                    Panel(ScrollView(details(selected == snapshot.end() ? nullptr : &*selected, snapshot))
                              .key("properties/" + selection)),
                    pane_fraction.binding(), {.first_min = 240, .second_min = 300})
                .accessible_label("Inspector panels")
                .key("inspector-panes")
                .grow(),
            Label(std::format("{} mounted elements  /  generation {}  /  temporary layout edits",
                              snapshot.size(), source->generation()),
                  LabelStyle::micro)
                .shrink(0)}
            .padding(16)
            .gap(12);
    }
};
Inspector::Inspector(std::shared_ptr<Host> source) {
    if (!source)
        throw std::invalid_argument("Inspector requires a source host");
    impl_ = std::make_shared<Impl>(std::move(source));
}
Inspector::~Inspector() = default;
View Inspector::body() {
    return impl_->body();
}
void Inspector::reveal_selection() {
    impl_->reveal_selection();
}
} // namespace scars
