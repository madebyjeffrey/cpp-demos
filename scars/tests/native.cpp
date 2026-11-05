#include <skia_window/skia_window.hpp>
#include <iostream>
#include <cmath>
#include <stdexcept>
#include <string>
#include <unordered_map>
import scars;
import scars.macos;
void scars_inspect_native();
void scars_scroll_native(int vertical, int horizontal);
void scars_inspect_text_and_titlebar();
void scars_report_native_visibility();
bool scars_native_window_on_active_space();
void scars_inspect_developer_tools();
void scars_minimize_developer_tools();
void scars_check_developer_tools_restored();
void scars_after_delay(std::function<void()>);
bool scars_focus_lifetime_password(bool allow_existing = false);
void scars_check_secure_input_off();
void scars_inspect_virtual_list(const std::function<void()>& scroll_away);
void scars_inspect_tree(const std::function<void()>& replace_model);
void scars_inspect_table(const std::function<void(int)>&);
void scars_check_table_closed();
void check_native_table(skia_window::Application& app) {
    namespace s=scars;
    auto result=skia_window::Window::create(app,{.title="SCARS Table Test",.size={380,260},.minimum_size={320,200}});
    if(!result)throw std::runtime_error(result.error().message);
    auto window=std::move(*result);
    s::State<s::TableSelection> selected;
    s::State<bool> row_mode{false};
    s::State<std::vector<s::SortDescriptor>> sort;
    s::State<std::unordered_map<std::string,std::string>> values;
    int reads{},commits{};
    std::vector<s::Option> rows;rows.reserve(100000);
    for(int i=0;i<100000;++i)rows.push_back({std::to_string(i),"Record "+std::to_string(i),i!=1});
    s::TableColumn name{.id="name",.title="Record",.width=140,.sortable=true};
    s::TableColumn value{.id="value",.title="Value",.width=140,.sortable=true,
        .text=[&](const auto& row,std::size_t){++reads;const auto& all=values.get();auto it=all.find(row.id);return it==all.end()?std::string("10"):it->second;},
        .validate=[](const auto&,const std::string& text){return !text.empty()&&text.find_first_not_of("0123456789")==std::string::npos?std::string{}:std::string("Use digits");},
        .commit=[&](const auto& row,std::string text){++commits;auto all=values.get();all[row]=std::move(text);values.set(std::move(all));}};
    s::TableColumn status{.id="status",.title="Status",.width=140};
    s::State<s::TableModel> model{s::TableModel(rows,{name,value,status})};
    auto host=s::macos::mount(*window,[&]{return s::TableView(selected.binding(),model.get(),{
        .selection_mode=row_mode.get()?s::TableSelectionMode::rows:s::TableSelectionMode::cells,.multiple_selection=true,.sort=sort.binding()});});
    host->layout({380,260});
    scars_inspect_table([&](int stage){
        if(stage==0){host->scroll_to("root",{0,0});if(!host->text_input()||!host->text_input()->marked)throw std::runtime_error("Native table composition was not pinned");}
        if(stage==1){row_mode.set(true);selected.set({{"0"},{}});}
        if(stage==2){row_mode.set(false);selected.set({{},{{"99999","value"}}});model.set(s::TableModel(rows,{value,name,status}));}
        if(stage==3)model.set(s::TableModel(rows,{name,status}));
        if(stage==5&&commits!=0)throw std::runtime_error("Native table draft changed business data");
        if(stage==6&&(commits!=1||values.get().at("99999")!="42"))throw std::runtime_error("Native table commit did not complete exactly once");
        if(stage==7&&(commits!=2||values.get().at("99997")!="43"))throw std::runtime_error("Native table Tab commit did not complete exactly once");
        host->layout({380,260});
    });
    if(commits!=2||reads>5000||host->elements().size()>180)throw std::runtime_error("Native table queries realized the dataset or duplicated editing callbacks");
    window->close();scars_check_table_closed();
    std::cout<<"Native table indexed columns/cells, selection, sorting, editing, IME, and schema lifetime checks passed.\n";
}
void check_native_tree(skia_window::Application& app) {
    namespace s = scars;
    auto result = skia_window::Window::create(app, {.title = "SCARS Tree Test", .size = {360, 300}, .minimum_size = {320, 200}});
    if (!result) throw std::runtime_error(result.error().message);
    auto window = std::move(*result);
    int loads{};
    s::TreeNode branch{"fleet", "Native fleet"};
    branch.lazy_children = [&] {
        ++loads;
        std::vector<s::TreeNode> children;
        for (int i = 0; i < 100; ++i) children.push_back({std::to_string(i), "Native node " + std::to_string(i)});
        return children;
    };
    s::State<s::TreeModel> model{s::TreeModel({std::move(branch), {"loading", "Native loading", true, true}})};
    s::State<std::string> selected{"fleet"};
    s::State<std::vector<std::string>> expanded;
    auto host = s::macos::mount(*window, [&] {
        return s::TreeView(selected.binding(), model.get(), expanded.binding(), {.row_height=28,.visible_rows=6})
            .accessible_label("Native outline");
    });
    host->layout({360,300});
    scars_inspect_tree([&] { model.set(s::TreeModel({{"fleet", "Native renamed fleet", true, false, {{"replacement", "New node"}}}})); });
    if (loads != 1 || selected.get() != "99" || host->elements().size() > 25)
        throw std::runtime_error("Native tree lost stable selection, reloaded a branch, or realized offscreen rows");
    std::cout << "Native tree outline, lazy children, disclosure, logical selection, and replacement checks passed.\n";
    window->close();
}
void check_native_virtual_list(skia_window::Application& app) {
    namespace s = scars;
    auto result = skia_window::Window::create(app, {.title = "SCARS Virtual List Test", .size = {360, 220}, .minimum_size = {320, 200}});
    if (!result) throw std::runtime_error(result.error().message);
    auto window = std::move(*result);
    s::State<std::string> selected{"0"}, value{"Native draft"};
    int built{};
    std::vector<s::Option> rows;
    rows.reserve(100000);
    for (int i=0; i<100000; ++i) rows.push_back({std::to_string(i), "Record " + std::to_string(i)});
    s::RowModel model(std::move(rows), [&](const s::Option& row, std::size_t i) -> s::View {
        ++built;
        if (i == 0) return s::TextField("Native virtual editor", value.binding()).accessible_label("Native virtual editor").key("editor");
        return s::Label(row.label);
    });
    auto host = s::macos::mount(*window, [&] { return s::ListBox(selected.binding(), model, {.row_height=36,.visible_rows=5}).accessible_label("Native virtual list"); });
    host->layout({360,220});
    scars_inspect_virtual_list([&] {
        host->scroll_to("root", {0,36000});
        if (!host->text_input() || !host->text_input()->marked)
            throw std::runtime_error("Native virtualized editor lost composition");
    });
    if (selected.get() != "99999" || built > 70 || host->elements().size() > 50)
        throw std::runtime_error("Native logical row access selected incorrectly or realized the dataset");
    std::cout << "Native logical accessibility verified 100000 rows with bounded realization.\n";
    window->close();
}
void check_native_scrolling(skia_window::Application& app) {
    namespace s = scars;
    namespace sw = skia_window;
    auto created = sw::Window::create(app, {.title = "SCARS Wheel Test", .size = {640, 600}});
    if (!created)
        throw std::runtime_error(created.error().message);
    auto window = std::move(*created);
    int events = 0;
    int vertical = -40, horizontal = 12;
    auto host = s::macos::mount(
        *window, [] { return s::ScrollView(s::Label("Wheel content").width(1200).height(1200), {.axes = s::ScrollAxes::both}); }, {}, {},
        {.on_event = [&](const sw::Event& event) {
             if (auto p = std::get_if<sw::PointerEvent>(&event); p && p->kind == sw::PointerKind::scroll) {
                 if (p->scroll_y != vertical || p->scroll_x != horizontal || p->button != 0 ||
                     p->click_count != 0)
                     throw std::runtime_error("Native wheel event metadata is incorrect");
                 ++events;
             }
             return false;
         }});
    host->layout({640, 600});
    const auto before = host->elements().at(0).bounds.y;
    const auto before_x = host->elements().at(0).bounds.x;
    scars_scroll_native(vertical, horizontal);
    if (events != 1 || std::abs(host->elements().at(0).bounds.y - (before - 40)) > .001)
        throw std::runtime_error("Native wheel event did not scroll SCARS content");
    vertical = 20;
    horizontal = -6;
    scars_scroll_native(vertical, horizontal);
    if (events != 2 || std::abs(host->elements().at(0).bounds.y - (before - 20)) > .001)
        throw std::runtime_error("Native wheel event did not scroll SCARS content back up");
    if (std::abs(host->elements().at(0).bounds.x - (before_x - 6)) > .001)
        throw std::runtime_error("Native horizontal wheel event did not scroll SCARS content");
    window->close();
}
void check_password_unmount(skia_window::Application& app) {
    namespace s = scars;
    auto created = skia_window::Window::create(app, {.title = "SCARS Input Lifetime Test"});
    if (!created) throw std::runtime_error(created.error().message);
    auto window = std::move(*created);
    s::State<std::string> value{"retained"};
    auto host = s::macos::mount(*window, [&] {
        return s::PasswordField("", value.binding(), {.allow_reveal = true, .allow_undo = true})
            .key("password").accessible_label("Lifetime password");
    });
    window->show();
    host->layout({320, 80});
    host->focus("root");
    host->insert_text("edited");
    for (const auto& e : host->elements())
        if (e.label == "Show password") { host->activate(e.id); break; }
    if (!host->text_input()->can_undo || !host->inspect().at(0).revealed)
        throw std::runtime_error("Password lifetime fixture did not enter a revealed editing state");
    const bool secure = scars_focus_lifetime_password();
    if (secure) {
        window->hide(); scars_check_secure_input_off(); window->show();
        if (!scars_focus_lifetime_password(true)) throw std::runtime_error("Restored password window did not regain secure input");
        host->insert_text("again");
        for (const auto& e : host->elements()) if (e.label == "Show password") { host->activate(e.id); break; }
    }
    s::macos::unmount(*window);
    const auto input = host->text_input();
    if (!input || input->active || input->can_undo || input->can_redo || host->inspect().at(0).revealed)
        throw std::runtime_error("Unmount retained password reveal, editing history, or input focus");
    if (secure) {
        scars_check_secure_input_off();
        auto replacement = s::macos::mount(*window, [&] {
            return s::PasswordField("", value.binding()).accessible_label("Lifetime password");
        });
        if (!scars_focus_lifetime_password()) throw std::runtime_error("Replacement password client did not acquire secure input");
    }
    window->close();
    if (secure) {
        scars_check_secure_input_off();
        std::cout << "Secure-input ownership balanced across hide, unmount, and window close.\n";
    }
}
int main(int argc, char** argv) {
    try {
        namespace s = scars;
        namespace sw = skia_window;
        bool failed = false, desktop_unavailable = false;
        const bool actions_only = argc > 1 && std::string(argv[1]) == "--actions-only";
        std::unique_ptr<sw::Application> app;
        auto created =
            sw::Application::create({.name = "SCARS Native Tests", .on_error = [&](const sw::Error& e) {
                                         std::cerr << e.message << '\n';
                                         failed = true;
                                         if (app)
                                             app->quit();
                                     }});
        if (!created)
            throw std::runtime_error(created.error().message);
        app = std::move(*created);
        auto result = sw::Window::create(*app, {.title = "SCARS Native Test", .size = {640, 720}});
        if (!result)
            throw std::runtime_error(result.error().message);
        auto window = std::move(*result);
        s::State<bool> checked{false};
        s::State<std::string> field_value{"initial"}, list_value{"a"};
        s::State<double> level{.5};
        s::State<double> pane_fraction{.5};
        s::State<std::string> native_tab{"first"};
        s::State<double> native_amount{2.5};
        s::State<int> native_count{3};
        s::State<bool> native_switch{false};
        s::State<std::string> native_query{""}, native_password{"native-demo"}, credential_status{"Waiting"};
        s::State<bool> native_overlay{false};
        s::State<bool> native_pinned{false};
        s::State<std::string> native_combo_query{"Alpha"}, native_combo_selection{"a"};
        bool combo_accepted{};
        int search_callbacks{}, expected_search_callbacks{};
        std::string last_search;
        s::State<std::string> overlay_text{"Overlay initial"};
        s::State<std::string> radio{"a"}, action{"a"}, selection{"a"};
        int commands = 0;
        struct CountingTextEngine final : s::TextLayoutEngine {
            std::shared_ptr<s::TextLayoutEngine> inner{s::make_skparagraph_text_engine()};
            int paints{};
            std::shared_ptr<const s::TextLayout> layout(const std::vector<s::TextSpan>& spans,
                                                        s::TextLayoutOptions options) override {
                auto source = inner->layout(spans, options);
                auto result = std::make_shared<s::TextLayout>(*source);
                result->paint = [this, source](SkCanvas& canvas, s::Point origin) {
                    ++paints;
                    source->paint(canvas, origin);
                };
                return result;
            }
        };
        auto text = std::make_shared<CountingTextEngine>();
        auto host = s::macos::mount(
            *window,
            [&] {
                return s::FlexColumn{
                    s::CheckBox("Native checkbox", checked.binding()).key("check"),
                    s::RadioGroup(radio.binding(), {{"a", "Native radio A"}, {"b", "Native radio B"}}),
                    s::Slider(level.binding(), {0, 1, .1, .2}).accessible_label("Native slider"),
                    s::SelectionDropdown(selection.binding(),
                                         {{"a", "Native value A"}, {"b", "Native value B"}})
                        .accessible_label("Native selection"),
                    s::SplitActionButton(action.binding(), {{"a", "Native action A", [&] { ++commands; }},
                                                            {"b", "Native action B", [&] { ++commands; }}})
                        .accessible_label("Native split"),
                    s::TabView(native_tab.binding(), {
                        {"first", "Native tab A", [&] {
                            return s::FlexColumn{
                                s::TextField("Type here", field_value.binding())
                                    .accessible_label("Native text").key("text").shrink(0),
                                s::ListBox(list_value.binding(), {{"a", "List alpha"}, {"b", "List beta"}},
                                           {.visible_rows = 2})
                                    .accessible_label("Native list").key("list").height(72).shrink(0)
                            }.gap(12);
                        }},
                        {"second", "Native tab B", [&] { return s::FlexColumn{
                            s::NumericField("Amount", native_amount.binding(), {.min = -10, .max = 10, .step = .25})
                                .accessible_label("Native amount"),
                            s::Stepper(native_count.binding(), {.min = 0, .max = 10}).accessible_label("Native count"),
                            s::Switch("Native switch", native_switch.binding())}.gap(8); }},
                        {"credentials", "Native credentials", [&] { return s::FlexColumn{
                            s::SearchField("Search", native_query.binding(), {
                                .debounce = std::chrono::milliseconds(80),
                                .on_search = [&](const auto& query) { ++search_callbacks; last_search = query; },
                                .on_submit = [&](const auto&) { native_overlay.set(true); }})
                                .accessible_label("Native search").tooltip("Search help")
                                .context_menu({
                                    s::MenuItem::check("pinned", "Native pin search", native_pinned.binding()),
                                    s::MenuItem::submenu("more", "Native more actions", {
                                        {"nested", "Native context action", [&] { credential_status.set("Context action"); }}
                                    })
                                })
                                .popover(native_overlay.binding(), [&] {
                                    return s::TextField("Overlay editor", overlay_text.binding())
                                        .accessible_label("Native overlay editor");
                                }, {.width = 240}),
                            s::PasswordField("Password", native_password.binding(), {.allow_reveal = true,
                                .on_submit = [&] { credential_status.set("Native credentials submitted"); }})
                                .accessible_label("Native password"),
                            s::Label(credential_status.get())}.gap(8); }},
                        {"choices", "Native choices", [&] { return s::FlexColumn{
                            s::ComboBox("Choose", native_combo_query.binding(), native_combo_selection.binding(),
                                {{"a", "Alpha"}, {"b", "Beta"}},
                                {.on_accept = [&](const auto&, const auto&) { combo_accepted = true; }})
                                .accessible_label("Native combo"),
                            s::Label("Query and selection have separate bindings.")}.gap(8); }}
                    }).accessible_label("Native pages").height(180).shrink(0),
                    s::SplitView(s::Label("Native first pane"), s::Label("Native second pane"),
                                 pane_fraction.binding())
                        .accessible_label("Native panes").height(80).shrink(0)}
                    .gap(12)
                    .padding(20);
            },
            {}, text);
        window->set_titlebar_theme_toggle(false, [&](bool light) {
            host->set_theme(light ? s::Theme::light() : s::Theme{});
            window->set_appearance(light ? sw::Appearance::light : sw::Appearance::dark);
            window->set_titlebar_theme_toggle_state(light);
        });
        window->set_appearance(sw::Appearance::dark);
        if (window->ui_host())
            throw std::runtime_error("SCARS mounted the legacy UI engine");
        window->show();
        window->focus();
        auto tools = std::make_shared<s::macos::DeveloperTools>(*app, host);
        std::function<void(int)> wait_for_frame;
        // Allow the activation/key-window events requested by run()/show() to
        // arrive before exercising input ownership, rather than testing an
        // application that has not entered its native event loop yet.
        scars_after_delay([&] {
            try {
                check_native_scrolling(*app);
                check_native_virtual_list(*app);
                check_native_tree(*app);
                check_native_table(*app);
                scars_inspect_native();
                scars_inspect_text_and_titlebar();
                if (!native_pinned.get() || credential_status.get() != "Context action")
                    throw std::runtime_error("Native context-menu actions did not update their model");
                if (!combo_accepted || native_combo_query.get() != "Beta" || native_combo_selection.get() != "b")
                    throw std::runtime_error("Native combo acceptance did not update query and selection");
                if (field_value.get() != "海🚀日本" || list_value.get() != "b" || host->theme().light_mode)
                    throw std::runtime_error(
                        "Native text, list, or theme integration did not update the model");
                if (!checked.get() || std::abs(level.get() - .7) > .001 || radio.get() != "b" ||
                    selection.get() != "b" || action.get() != "b" || commands != 1)
                    throw std::runtime_error(
                        "Native accessibility actions did not update SCARS state correctly");
                if (window->is_animating())
                    throw std::runtime_error("Static SCARS content kept continuous native rendering enabled");
                native_tab.set("credentials");
                host->layout({window->content_size().width, window->content_size().height});
                for (const auto& e : host->elements())
                    if (e.label == "Native search") {
                        host->focus(e.id); host->set_text(e.id, "Scheduled query"); break;
                    }
                host->focus("root/key/check");
                expected_search_callbacks = search_callbacks + 1;
            } catch (const std::exception& e) {
                std::cerr << e.what() << '\n';
                failed = true;
            }
            wait_for_frame = [&](int attempts) {
                if (!actions_only && text->paints == 0 && attempts > 0) {
                    scars_after_delay([&, attempts] { wait_for_frame(attempts - 1); });
                    return;
                }
                const int before = text->paints;
                scars_after_delay([&, before] {
                    if (!actions_only && (before == 0 || text->paints != before)) {
                        if (before == 0 && !scars_native_window_on_active_space()) {
                            desktop_unavailable = true;
                            std::cout << "Frame settling requires an active desktop Space; inspector "
                                         "interaction checks still run.\n";
                        } else {
                            if (before == 0)
                                scars_report_native_visibility();
                            std::cerr << "SCARS static window did not render and settle: " << before << " -> "
                                      << text->paints << '\n';
                            failed = true;
                        }
                    }
                    try {
                        tools->show();
                        scars_inspect_developer_tools();
                        if (!host->inspection_picking())
                            throw std::runtime_error(
                                "Inspector Pick element button is not connected to the source host");
                        scars_minimize_developer_tools();
                    } catch (const std::exception& e) {
                        std::cerr << e.what() << '\n';
                        failed = true;
                        tools->close();
                        window->close();
                        app->quit();
                        return;
                    }
                    scars_after_delay([&] {
                        try {
                            tools->toggle();
                            if (!tools->is_open())
                                throw std::runtime_error(
                                    "Inspector shortcut closed a minimized window instead of restoring it");
                            scars_check_developer_tools_restored();
                            s::LayoutOverride patch;
                            patch.gap = 24;
                            host->set_layout_override("root", patch);
                            host->set_inspection_overlay(s::InspectionOverlay::flex);
                            tools->toggle();
                            if (tools->is_open() || host->inspection_picking() ||
                                !host->inspected_element().empty() || host->layout_override("root") ||
                                host->inspection_overlay() != s::InspectionOverlay::selection)
                                throw std::runtime_error(
                                    "Inspector close did not clear its hooks, highlight, overlays and edits");
                            tools->show();
                            tools->close();
                            check_password_unmount(*app);
                        } catch (const std::exception& e) {
                            std::cerr << e.what() << '\n';
                            failed = true;
                        }
                        tools->close();
                        window->close();
                        app->quit();
                    });
                });
            };
            scars_after_delay([&] {
                if (search_callbacks != expected_search_callbacks || last_search != "Scheduled query" || host->next_wake() || window->is_animating()) {
                    std::cerr << "Native one-shot redraw did not deliver and clear the debounced search: callbacks="
                              << search_callbacks << "/" << expected_search_callbacks << ", query=" << last_search
                              << ", pending=" << host->next_wake().has_value() << ", animating=" << window->is_animating() << '\n';
                    failed = true;
                } else std::cout << "Native one-shot redraw delivered the debounced search without continuous animation.\n";
                wait_for_frame(8);
            });
        });
        app->run();
        // A retained host and binding are safe after the native surface has closed.
        checked.set(false);
        host->layout({400, 500});
        if (failed)
            return 1;
        std::cout << "SCARS native scrolling, accessibility, inspector, and lifetime checks passed\n";
        return desktop_unavailable ? 77 : 0;
    } catch (const std::exception& e) {
        std::cerr << "SCARS native: " << e.what() << '\n';
        return 1;
    }
}
