#include "include/core/SkCanvas.h"
#include "include/core/SkSurface.h"
#include <algorithm>
#include <iostream>
#include <stdexcept>
#include <string>
#include <unordered_map>
#include <vector>
import scars;
namespace s = scars;
namespace {
int checks{};
void require(bool ok, const char* message) {
    ++checks;
    if (!ok)
        throw std::runtime_error(message);
}
s::Theme theme(bool light = false) {
    auto t = light ? s::Theme::light() : s::Theme{};
    t.font_path = SCARS_FONT_PATH;
    return t;
}
s::InspectionNode keyed(s::Host& h, const std::string& key) {
    for (const auto& n : h.inspect())
        if (n.key == key)
            return n;
    throw std::runtime_error("Missing key: " + key);
}
s::Element element(s::Host& h, const std::string& id) {
    for (const auto& e : h.elements())
        if (e.id == id)
            return e;
    throw std::runtime_error("Missing element: " + id);
}
void key(s::Host& h, s::Key k, bool shift = false, bool command = false) {
    h.handle(s::KeyEvent{.key = k, .shift = shift, .command = command});
}
std::vector<s::Option> rows(std::size_t n) {
    std::vector<s::Option> result;
    for (std::size_t i = 0; i < n; ++i)
        result.push_back({std::to_string(i), "Record " + std::to_string(i), i != 1});
    return result;
}
void geometry_and_selection() {
    for (bool light : {false, true}) {
        s::State<s::TableSelection> selected{s::TableSelection{{}, {{"0", "power"}}}};
        s::State<std::vector<s::TableColumnWidth>> widths;
        s::State<std::vector<s::SortDescriptor>> sort;
        int reads{}, hosts{}, requests{};
        s::TableColumn name{
            .id = "name", .title = "Name", .width = 140, .min_width = 60, .max_width = 300, .sortable = true};
        s::TableColumn power{.id = "power",
                             .title = "Power",
                             .width = 100,
                             .min_width = 60,
                             .max_width = 200,
                             .sortable = true,
                             .text = [&](const auto&, std::size_t i) {
                                 ++reads;
                                 return std::to_string(i % 100);
                             }};
        s::TableColumn status{.id = "status", .title = "Status", .width = 140};
        s::TableModel model(rows(100000), {name, power, status});
        s::Host h(
            [&] {
                ++hosts;
                return s::TableView(selected.binding(), model,
                                    {.row_height = 28,
                                     .selection_mode = s::TableSelectionMode::cells,
                                     .multiple_selection = true,
                                     .widths = widths.binding(),
                                     .sort = sort.binding(),
                                     .on_sort = [&](const auto&) { ++requests; }})
                    .key("table");
            },
            {}, theme(light));
        h.layout({300, 180});
        const auto id = keyed(h, "table").id;
        require(keyed(h, "table").type == "TableView" && keyed(h, "table").list->logical_count == 100000 &&
                    keyed(h, "table").list->mounted_count <= 10,
                "Tables virtualize rows beneath a fixed header");
        require(element(h, id).role == s::Role::table && element(h, id).column_count == 3,
                "Table semantics expose the schema and logical row count");
        auto far = *h.table_cell(id, {"99999", "power"});
        require(!far.bounds && far.row_index == 99999 && far.column_index == 1 && far.value == "99",
                "Offscreen cells expose indexed metadata without fabricated geometry");
        const auto initial = reads;
        h.scroll_to(id, {50, 28000});
        require(hosts == 1 && reads - initial <= 12 && keyed(h, "table").list->mounted_count <= 12,
                "Table scrolling constructs only visible cells and does not rebuild the host");
        const auto header = element(h, id + "/header/power");
        const auto item = h.list_item(id, 1000);
        const auto cell = *h.table_cell(id, {"1000", "power"});
        require(cell.bounds && header.bounds.x == cell.bounds->x && header.bounds.y == 4,
                "Headers remain aligned horizontally and fixed vertically");
        require(h.focus(far.id) && h.activate(far.id) &&
                    selected.get().cells == std::vector<s::TableCell>{{"99999", "power"}},
                "Logical cell focus and activation reveal distant cells by stable keys");
        key(h, s::Key::home);
        key(h, s::Key::down);
        require(selected.get().cells == std::vector<s::TableCell>{{"2", "power"}},
                "Vertical cell navigation skips disabled rows");
        key(h, s::Key::right, true);
        require(selected.get().cells.size() == 2, "Shift horizontal navigation extends a cell rectangle");
        require(h.set_value(id + "/header/power/resize", 1000) && widths.get()[1].width == 200,
                "Column widths clamp to schema bounds and write their binding");
        const auto resize = element(h, id + "/header/power/resize");
        h.handle(s::PointerEvent{s::PointerKind::down,
                                 {resize.bounds.x + resize.bounds.width / 2, resize.bounds.y + 10}});
        h.handle(s::PointerEvent{s::PointerKind::move, {resize.bounds.x - 30, resize.bounds.y + 10}});
        require(keyed(h, "table").table->columns[1].width < 200,
                "Pointer capture resizes a column beyond the handle");
        key(h, s::Key::escape);
        require(keyed(h, "table").table->columns[1].width == 200,
                "Escape restores the initial captured column width");
        h.focus(id + "/header/power/resize");
        key(h, s::Key::left);
        require(keyed(h, "table").table->columns[1].width == 190,
                "Focused column dividers support keyboard resizing");
        require(h.adjust(id + "/header/power/resize", 1) && keyed(h, "table").table->columns[1].width == 200,
                "Accessible adjustment uses bounded column widths");
        require(h.activate(id + "/header/power") &&
                    sort.get() == std::vector<s::SortDescriptor>{{"power", s::SortOrder::ascending}} &&
                    requests == 1,
                "Header activation issues an explicit ascending sort request");
        require(h.list_item(id, 0)->label == "Record 0",
                "Sorting does not silently reorder application rows");
        h.activate(id + "/header/power");
        require(sort.get()[0].order == s::SortOrder::descending && requests == 2,
                "Repeated header activation requests descending order");
        h.activate(id + "/header/power");
        require(sort.get().empty() && requests == 3,
                "The third header activation clears the sort descriptor");
        auto raster = SkSurfaces::Raster(SkImageInfo::MakeN32Premul(600, 360));
        raster->getCanvas()->scale(2, 2);
        h.paint(*raster->getCanvas(), 0);
        require(!h.needs_frame(), "Readonly tables settle without continuous rendering");
    }
}
void editing() {
    s::State<std::unordered_map<std::string, std::string>> values;
    s::State<s::TableSelection> selected{s::TableSelection{{}, {{"0", "power"}}}};
    int commits{};
    s::TableColumn name{.id = "name", .title = "Name", .width = 140};
    s::TableColumn power{
        .id = "power",
        .title = "Power",
        .width = 120,
        .text =
            [&](const auto& row, std::size_t) {
                const auto& map = values.get();
                auto i = map.find(row.id);
                return i == map.end() ? std::string("20") : i->second;
            },
        .validate = [](const auto&,
                       const auto& value) { return value == "bad" ? "Enter a power value" : std::string{}; },
        .commit =
            [&](const auto& row, std::string value) {
                ++commits;
                auto map = values.get();
                map[row] = std::move(value);
                values.set(std::move(map));
            }};
    s::TableModel model(rows(1000), {name, power});
    s::Host h(
        [&] {
            return s::Block(
                {s::TableView(
                     selected.binding(), model,
                     {.row_height = 36, .visible_rows = 4, .selection_mode = s::TableSelectionMode::cells})
                     .key("table"),
                 s::Button("Leave", [] {}).key("leave")});
        },
        {}, theme());
    h.layout({360, 240});
    auto id = keyed(h, "table").id;
    h.focus(id);
    key(h, s::Key::f2);
    require(h.text_input() && h.text_input()->text == "20" &&
                keyed(h, "table").table->editing == s::TableCell{"0", "power"},
            "F2 opens the standard input client for the active editable cell");
    h.insert_text("33");
    require(values.get().empty() && h.text_input()->text == "33",
            "Cell typing changes a draft without changing business data");
    h.mark_text("仮", {0, 3});
    h.scroll_to(id, {0, 18000});
    require(h.text_input()->marked && keyed(h, "table").list->pinned == "0",
            "Virtualization preserves the active cell and native composition");
    key(h, s::Key::enter);
    require(!h.text_input()->marked && commits == 0, "Composition completion precedes table commit");
    h.cancel_table_edit(id);
    require(!h.text_input() && values.get().empty() && keyed(h, "table").list->retained_editors == 0,
            "Cancel discards the draft and releases the offscreen editor pin");
    h.edit_cell(id, {"0", "power"});
    h.insert_text("bad");
    require(!h.commit_table_edit(id) && commits == 0 &&
                keyed(h, "table").table->validation == "Enter a power value",
            "Invalid cell values retain the editor with validation feedback");
    key(h, s::Key::tab);
    require(h.text_input() && keyed(h, "table").table->editing == s::TableCell{"0", "power"},
            "Tab cannot leave an invalid cell");
    key(h, s::Key::escape);
    require(!h.text_input() && commits == 0, "Escape explicitly cancels a cell draft");
    h.edit_cell(id, {"0", "power"});
    h.insert_text("44");
    key(h, s::Key::tab);
    require(commits == 1 && values.get().at("0") == "44" &&
                keyed(h, "table").table->editing == s::TableCell{"2", "power"},
            "Tab commits once and finds the next enabled editable cell");
    key(h, s::Key::tab, true);
    require(commits == 2 && keyed(h, "table").table->editing == s::TableCell{"0", "power"},
            "Shift-Tab traverses editable cells in reverse");
    values.set({{"0", "external"}});
    h.inspect();
    require(!h.commit_table_edit(id) &&
                keyed(h, "table").table->validation.find("externally") != std::string::npos,
            "External cell changes reject stale draft overwrites");
    h.focus(keyed(h, "leave").id);
    require(!keyed(h, "table").table->editing && values.get().at("0") == "external",
            "Blur cancels a draft without overwriting application changes");
}
void logical_table_access() {
    s::State<s::TableSelection> selected;
    s::State<std::string> value{"10"};
    s::State<std::vector<s::SortDescriptor>> sort;
    int writes{}, commits{};
    s::Binding<s::TableSelection> binding{[&] { return selected.get(); },
                                          [&](s::TableSelection next) {
                                              ++writes;
                                              selected.set(std::move(next));
                                          }};
    s::TableColumn name{.id = "name", .title = "Name", .sortable = true};
    s::TableColumn power{
        .id = "power/level",
        .title = "Power",
        .sortable = true,
        .text = [&](const auto&, std::size_t) { return value.get(); },
        .validate = [](const auto&,
                       const auto& text) { return text == "bad" ? std::string("Invalid") : std::string{}; },
        .commit =
            [&](const auto&, std::string next) {
                ++commits;
                value.set(std::move(next));
            }};
    s::State<s::TableModel> model{s::TableModel(rows(200), {name, power})};
    s::Host h(
        [&] {
            return s::TableView(binding, model.get(),
                                {.selection_mode = s::TableSelectionMode::cells,
                                 .multiple_selection = true,
                                 .sort = sort.binding()});
        },
        {}, theme());
    h.layout({300, 160});
    const auto column = *h.table_column("root", 1);
    const auto far = *h.table_cell_at("root/option/199", 1), first = *h.table_cell_at("root/option/0", 1);
    require(column.id == "root/column/power%2Flevel" && column.index == 1 && column.row_count == 200 &&
                column.header_id == "root/header/power%2Flevel",
            "Logical columns expose stable identity and current schema geometry");
    require(h.table_column_cell(column.id, 199)->id == far.id && h.table_cell(far.id)->column_index == 1 &&
                !far.bounds,
            "Row and column indexing reach the same unrealized cell");
    require(h.set_selection("root", {first.id, far.id}) && writes == 1 && selected.get().cells.size() == 2,
            "Batch logical cell selection writes its binding exactly once");
    require(h.set_selected(first.id, false) &&
                selected.get().cells == std::vector<s::TableCell>{{"199", "power/level"}},
            "Logical deselection removes the requested cell instead of reselecting it");
    require(h.set_selected(first.id, true) && selected.get().cells.size() == 2,
            "Logical selection can add a cell without clearing a multiple selection");
    const auto before = writes;
    require(!h.set_selection("root", {"root/option/1/cell/name"}) && writes == before &&
                selected.get().cells.size() == 2,
            "Invalid or disabled batch entries are rejected without partial writes");
    h.activate(column.header_id);
    require(h.table_column(column.id)->sort_order == s::SortOrder::ascending &&
                element(h, column.header_id).sort_order == s::SortOrder::ascending,
            "Columns and header controls expose the same sort direction");
    sort.set({{"missing", s::SortOrder::ascending},
              {"power/level", s::SortOrder::descending},
              {"power/level", s::SortOrder::ascending},
              {"name", s::SortOrder::ascending}});
    require(h.table_column(column.id)->sort_priority == 0 &&
                h.table_column(column.id)->sort_order == s::SortOrder::descending &&
                h.table_column("root", 0)->sort_priority == 1 && sort.get().size() == 4,
            "Sort metadata ignores unknown/duplicate descriptors without correcting application bindings "
            "during construction");
    require(h.set_cell_text(first.id, "33") && commits == 1 && value.get() == "33",
            "Logical value replacement uses the standard validated edit transaction");
    require(!h.set_cell_text(first.id, "bad") && commits == 1 &&
                h.table_cell(first.id)->validation == "Invalid",
            "Rejected logical value replacement retains a draft and validation feedback");
    h.mark_text("仮", {0, 3});
    require(!h.set_cell_text(first.id, "44") && h.text_input()->marked && commits == 1,
            "Logical value setters do not interrupt active input-method composition");
    h.cancel_table_edit("root");
    model.set(s::TableModel(rows(200), {power, name}));
    require(h.table_column(column.id)->index == 0 && h.table_cell(far.id)->column_index == 0 &&
                h.table_column_cell(column.id, 199)->id == far.id,
            "Held logical column and cell identities follow schema reordering");
    model.set(s::TableModel(rows(200), {name}));
    require(!h.table_column(column.id) && !h.table_cell(far.id),
            "Removed columns invalidate held logical access objects");
    require(h.table_column("root", 0)->sort_priority == 0 && sort.get().size() == 4,
            "Schema removal renumbers projected sort priority while preserving application descriptors");
}
void lifetimes_and_row_selection() {
    s::State<s::TableSelection> selection{s::TableSelection{{"0"}, {}}};
    s::TableColumn name{.id = "name", .title = "Name"};
    s::TableModel plain(rows(3), {name});
    s::Host rows_host([&] { return s::TableView(selection.binding(), plain, {.multiple_selection = true}); },
                      {}, theme());
    rows_host.layout({300, 160});
    rows_host.activate("root/option/2");
    require(selection.get().rows == std::vector<std::string>({"0", "2"}),
            "Row-selection tables preserve logical multi-selection toggles");
    rows_host.activate("root/option/2/cell/name");
    require(selection.get().rows == std::vector<std::string>{"2"},
            "A normal cell click uses row selection when configured");

    s::State<s::TableSelection> selected{s::TableSelection{{}, {{"0", "power"}}}};
    s::State<s::TableModel> data;
    s::TableColumn power{
        .id = "power", .title = "Power", .text = [](const auto&, std::size_t) { return std::string("10"); }};
    power.commit = [&](const auto&, const auto&) {
        auto reversed = rows(3);
        std::reverse(reversed.begin(), reversed.end());
        data.set(s::TableModel(std::move(reversed), {name, power}));
    };
    data.set(s::TableModel(rows(3), {name, power}));
    s::Host h(
        [&] {
            return s::Block({s::TableView(selected.binding(), data.get(),
                                          {.selection_mode = s::TableSelectionMode::cells})
                                 .key("table"),
                             s::Button("Next", [] {}).key("next")});
        },
        {}, theme());
    h.layout({380, 250});
    const auto id = keyed(h, "table").id;
    h.edit_cell(id, {"0", "power"});
    h.insert_text("11");
    key(h, s::Key::tab);
    require(!keyed(h, "table").table->editing && element(h, keyed(h, "next").id).focused,
            "Tab resolves stable IDs again after a commit callback reorders data");
    h.edit_cell(id, {"2", "power"});
    h.mark_text("仮", {0, 3});
    data.set(s::TableModel(rows(3), {name}));
    require(!h.text_input() && !keyed(h, "table").table->editing,
            "Removing an edited column cancels its composition and draft");

    s::State<bool> open{true};
    s::TableColumn edit{.id = "value", .title = "Value", .commit = [](const auto&, const auto&) {}};
    s::TableModel model(rows(3), {edit});
    s::Host popup(
        [&] {
            return s::Button("Anchor", [] {}).popover(open.binding(), [&] {
                return s::TableView(selected.binding(), model,
                                    {.selection_mode = s::TableSelectionMode::cells})
                    .key("inside");
            });
        },
        {}, theme());
    popup.layout({500, 400});
    const auto inside = keyed(popup, "inside").id;
    popup.edit_cell(inside, {"0", "value"});
    key(popup, s::Key::escape);
    require(open.get() && !keyed(popup, "inside").table->editing,
            "Escape cancels a table draft before dismissing its containing popover");
    key(popup, s::Key::escape);
    require(!open.get(), "A subsequent Escape dismisses the containing popover");
}

void lifecycle_changes() {
    s::State<bool> visible{true}, enabled{true};
    s::State<s::TableSelection> selected{s::TableSelection{{}, {{"0", "value"}}}};
    s::State<std::unordered_map<std::string, std::string>> values;
    int commits{};
    s::TableColumn name{.id = "name", .title = "Name", .width = 120};
    s::TableColumn value{.id = "value",
                         .title = "Value",
                         .width = 140,
                         .text =
                             [&](const auto& row, std::size_t) {
                                 const auto& all = values.get();
                                 auto found = all.find(row.id);
                                 return found == all.end() ? std::string("base") : found->second;
                             },
                         .commit =
                             [&](const auto& row, std::string text) {
                                 ++commits;
                                 auto all = values.get();
                                 all[row] = std::move(text);
                                 values.set(std::move(all));
                             }};
    s::State<s::TableModel> model{s::TableModel(rows(100), {name, value})};
    s::Host h(
        [&] {
            return s::TableView(selected.binding(), model.get(),
                                {.row_height = 32, .selection_mode = s::TableSelectionMode::cells})
                .hidden(!visible.get())
                .enabled(enabled.get());
        },
        {}, theme());
    h.layout({300, 180});
    h.edit_cell("root", {"0", "value"});
    h.insert_text("draft");
    h.mark_text("仮", {0, 3});
    auto reordered = rows(100);
    std::reverse(reordered.begin(), reordered.end());
    model.set(s::TableModel(std::move(reordered), {value, name}));
    h.inspect();
    require(h.text_input() && h.text_input()->marked && h.table_cell("root", {"0", "value"})->editing,
            "Row and column reordering preserve the same marked editor by stable IDs");
    h.unmark_text(false);
    h.insert_text("saved");
    require(h.commit_table_edit("root") && commits == 1 &&
                values.get().at("0").find("saved") != std::string::npos,
            "A reordered editor commits to its original row and column");
    h.edit_cell("root", {"0", "value"});
    h.mark_text("仮", {0, 3});
    auto filtered = rows(100);
    filtered.erase(filtered.begin());
    model.set(s::TableModel(std::move(filtered), {name, value}));
    h.inspect();
    require(!h.text_input() && !h.inspect()[0].table->editing && commits == 1,
            "Filtering out the edited row cancels composition without committing stale data");
    model.set(s::TableModel(rows(100), {name, value}));
    h.edit_cell("root", {"2", "value"});
    h.insert_text("hidden");
    visible.set(false);
    h.inspect();
    require(!h.text_input() && h.elements().empty() && commits == 1,
            "Hiding a table releases its editor without an implicit commit");
    visible.set(true);
    h.inspect();
    require(!h.inspect()[0].table->editing && !h.table_cell("root", {"2", "value"})->editing,
            "Showing a table cannot resurrect a canceled editor");
    h.edit_cell("root", {"2", "value"});
    h.mark_text("仮", {0, 3});
    enabled.set(false);
    h.inspect();
    require(!h.text_input() && !h.inspect()[0].table->editing && commits == 1,
            "Disabling an ancestor table cancels its active input client");
    enabled.set(true);
    h.inspect();
    auto resize = element(h, "root/header/name/resize");
    const s::Point point{resize.bounds.x + 2, resize.bounds.y + 10};
    h.handle(s::PointerEvent{s::PointerKind::down, point});
    h.handle(s::PointerEvent{s::PointerKind::move, {point.x + 20, point.y}});
    const auto width = h.table_column("root", 0)->width;
    h.handle(s::PointerEvent{s::PointerKind::cancel, {}});
    h.handle(s::PointerEvent{s::PointerKind::move, {point.x + 70, point.y}});
    require(h.table_column("root", 0)->width == width,
            "Pointer cancellation ends column capture at its last bounded width");
    resize = element(h, "root/header/name/resize");
    h.handle(s::PointerEvent{s::PointerKind::down, {resize.bounds.x + 2, resize.bounds.y + 10}});
    visible.set(false);
    h.inspect();
    visible.set(true);
    h.inspect();
    h.handle(s::PointerEvent{s::PointerKind::move, {2000, 0}});
    require(h.table_column("root", 0)->width == width,
            "Hide/show cannot revive an earlier column resize capture");

    bool fail = true;
    int hosts{};
    s::TableColumn throwing{.id = "value", .title = "Value", .text = [&](const auto& row, std::size_t index) {
                                if (fail && index >= 40)
                                    throw std::runtime_error("Cell failure");
                                return row.label;
                            }};
    s::TableModel data(rows(100), {throwing});
    s::Host robust(
        [&] {
            ++hosts;
            return s::TableView(selected.binding(), data, {.row_height = 24});
        },
        {}, theme());
    robust.layout({300, 160});
    bool caught{};
    try {
        robust.scroll_to("root", {0, 1200});
    } catch (const std::runtime_error&) {
        caught = true;
    }
    fail = false;
    robust.layout({300, 160});
    require(caught && hosts == 1 && robust.table_cell("root", {"50", "value"})->bounds &&
                robust.inspect()[0].list->mounted_count <= 10,
            "A failed lazy cell build rolls back and recovers without rebuilding the full dataset");
    s::State<int> illegal{0};
    s::TableColumn impure{.id = "value", .title = "Value", .text = [&](const auto&, std::size_t) {
                              illegal.set(1);
                              return std::string{};
                          }};
    s::Host pure([&] { return s::TableView(selected.binding(), s::TableModel(rows(3), {impure})); }, {},
                 theme());
    caught = false;
    try {
        pure.layout({300, 160});
    } catch (const std::logic_error&) {
        caught = true;
    }
    require(caught && illegal.get() == 0, "Lazy cell readers enforce the view-construction mutation guard");
}
void reveal_during_model_changes() {
    s::State<s::TableSelection> selection;
    s::TableColumn name{.id = "name", .title = "Name", .width = 180};
    s::TableColumn value{.id = "value",
                         .title = "Value",
                         .width = 180,
                         .text = [](const auto& row, std::size_t) { return row.id; },
                         .commit = [](const auto&, const auto&) {}};
    s::State<s::TableModel> model{s::TableModel(rows(100), {name, value})};
    s::Host h(
        [&] {
            return s::TableView(selection.binding(), model.get(),
                                {.row_height = 28, .selection_mode = s::TableSelectionMode::cells});
        },
        {}, theme());
    h.layout({250, 160});
    bool armed = true;
    h.set_inspection_observer([&] {
        if (!armed)
            return;
        armed = false;
        model.set(s::TableModel(rows(3), {name}));
        h.layout({250, 160});
    });
    require(!h.reveal_cell("root", {"99", "value"}) && !h.table_cell("root", {"99", "value"}),
            "Reveal rejects a target removed synchronously during layout without using stale indices");
    model.set(s::TableModel(rows(100), {name, value}));
    h.layout({250, 160});
    armed = true;
    h.set_inspection_observer([&] {
        if (!armed)
            return;
        armed = false;
        auto reordered = rows(100);
        std::reverse(reordered.begin(), reordered.end());
        model.set(s::TableModel(std::move(reordered), {value, name}));
        h.layout({250, 160});
    });
    require(h.reveal_cell("root", {"99", "value"}) && h.table_cell("root", {"99", "value"})->row_index == 0 &&
                h.table_cell("root", {"99", "value"})->column_index == 0,
            "Reveal follows the same stable cell when a layout observer reorders rows and columns");
    h.set_inspection_observer({});
    model.set(s::TableModel(rows(100), {name, value}));
    h.layout({250, 160});
    armed = true;
    h.set_inspection_observer([&] {
        if (!armed || h.scroll_offset("root")->y < 100)
            return;
        armed = false;
        auto changed = value;
        changed.text = [](const auto&, std::size_t) { return std::string("new base"); };
        model.set(s::TableModel(rows(100), {name, changed}));
        h.layout({250, 160});
    });
    require(h.edit_cell("root", {"99", "value"}) && h.text_input()->text == "new base",
            "Editor activation reads the current cell snapshot after reveal-time model replacement");
    h.set_inspection_observer({});
}

} // namespace
int main() {
    try {
        geometry_and_selection();
        editing();
        lifetimes_and_row_selection();
        logical_table_access();
        lifecycle_changes();
        reveal_during_model_changes();
        std::cout << checks << " table checks passed\n";
        return 0;
    } catch (const std::exception& e) {
        std::cerr << e.what() << '\n';
        return 1;
    }
}
