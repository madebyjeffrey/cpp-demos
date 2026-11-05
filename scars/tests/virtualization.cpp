#include "include/core/SkCanvas.h"
#include "include/core/SkSurface.h"
#include <algorithm>
#include <chrono>
#include <iostream>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>
import scars;
namespace s = scars;
namespace {
int checks{};
void require(bool ok, const char* message) { ++checks; if (!ok) throw std::runtime_error(message); }
s::Theme theme(bool light = false) { auto t = light ? s::Theme::light() : s::Theme{}; t.font_path = SCARS_FONT_PATH; return t; }
s::InspectionNode keyed(s::Host& h, const std::string& key) {
    for (const auto& n : h.inspect()) if (n.key == key) return n;
    throw std::runtime_error("Missing key: " + key);
}
s::Element element(s::Host& h, const std::string& id) {
    for (const auto& n : h.elements()) if (n.id == id) return n;
    throw std::runtime_error("Missing element: " + id);
}
std::vector<s::Option> options(std::size_t count) {
    std::vector<s::Option> result; result.reserve(count);
    for (std::size_t i = 0; i < count; ++i) result.push_back({std::to_string(i), "Record " + std::to_string(i), i != 99998});
    return result;
}
void large_list() {
    for (const bool light : {false, true}) {
        const auto start = std::chrono::steady_clock::now();
        int built{}, hosts{};
        s::RowModel model(options(100000), [&](const s::Option& row, std::size_t) { ++built; return s::Label(row.label); });
        const auto indexed = std::chrono::steady_clock::now();
        s::State<std::string> selected{"0"};
        s::Host h([&] { ++hosts; return s::ListBox(selected.binding(), model, {.row_height=24, .visible_rows=8, .overscan_rows=2}).key("list"); }, {}, theme(light));
        h.layout({320, 194}); const auto id = keyed(h, "list").id;
        auto list = *keyed(h, "list").list;
        require(list.logical_count == 100000 && list.mounted_count <= 13 && built <= 24, "Initial mounting is bounded by viewport and overscan");
        require(element(h,id).item_count == 100000 && element(h,id).selected_items == std::vector<std::size_t>{0}, "Semantic list exposes logical count and indexed selection");
        auto far = *h.list_item(id, 99999);
        require(far.index == 99999 && far.count == 100000 && !far.bounds && !far.clip && far.enabled, "Offscreen logical items have metadata without fabricated bounds");
        require(!h.list_item(id,100000) && !h.list_item("unknown",0) && !h.reveal_item(id,100000), "Logical APIs reject invalid indices and lists");
        const int previous = built;
        h.scroll_to(id, {0,24000});
        require(hosts == 1 && built-previous <= 15 && keyed(h,"list").list->mounted_count <= 14, "Scrolling builds only newly visible rows without rebuilding the host");
        const auto same = built; h.scroll_to(id,{0,24000}); h.inspect(); h.elements();
        require(built == same, "Repeated geometry and semantic queries reuse mounted rows");
        require(h.focus(far.id) && element(h,far.id).focused && !element(h,id).focused, "Distant logical focus reveals a row and retains list keyboard routing");
        require(!h.focus(id+"/option/99998") && !h.activate(id+"/option/99998"), "Disabled logical rows reject focus and selection");
        require(h.list_item(far.id)->bounds.has_value() && !h.list_item(id,0)->bounds, "Only realized rows expose actual geometry");
        require(h.activate(far.id) && selected.get() == "99999", "Logical activation selects by stable identity");
        h.handle(s::KeyEvent{s::Key::up});
        require(selected.get() == "99997", "Distant keyboard navigation skips disabled metadata");
        h.handle(s::KeyEvent{s::Key::home}); require(selected.get() == "0", "Home reveals and selects the first logical item");
        h.handle(s::KeyEvent{s::Key::end}); require(selected.get() == "99999", "End reveals and selects the last logical item");
        const auto before = built;
        for (int i=0;i<40;++i) h.scroll_to(id,{0,double(i)*4800});
        require(built-before <= 600 && keyed(h,"list").list->mounted_count <= 14 && keyed(h,"list").list->retained_editors == 0, "Repeated distant scrolling bounds construction and retained state");
        h.layout({180,50}); require(keyed(h,"list").list->mounted_count <= 8, "A constrained viewport reduces the realized range");
        auto surface=SkSurfaces::Raster(SkImageInfo::MakeN32Premul(360,100)); surface->getCanvas()->scale(2,2); h.paint(*surface->getCanvas(),0);
        require(!h.needs_frame(), "A nonediting virtual list settles without continuous rendering");
        const auto end=std::chrono::steady_clock::now();
        std::cout << (light?"Light":"Dark") << " 100000 rows: index " << std::chrono::duration<double,std::milli>(indexed-start).count()
                  << "ms, mount/navigation/40 distant scrolls " << std::chrono::duration<double,std::milli>(end-indexed).count()
                  << "ms, built " << built << ", mounted " << keyed(h,"list").list->mounted_count << '\n';
    }
}
void editing_and_observers() {
    auto cells=std::make_shared<std::vector<s::State<std::string>>>();
    for (int i=0;i<200;++i) cells->emplace_back("Value "+std::to_string(i));
    int hosts{}, built{};
    s::RowModel model(options(200), [cells,&built](const s::Option&,std::size_t i) { ++built; return s::TextField("Value", (*cells)[i].binding()).key("editor"); });
    s::State<std::string> selected{"0"};
    s::Host h([&] { ++hosts; return s::Block({s::ListBox(selected.binding(),model,{.row_height=36,.visible_rows=4}).key("list"),s::Button("Leave",[]{}).key("leave")}); },{},theme());
    h.layout({360,220}); const auto id=keyed(h,"list").id;
    const auto editor=id+"/option/0/key/editor";
    require(h.focus(editor) && h.text_input().has_value(), "Custom row editors share the normal input client");
    h.insert_text("Draft"); h.mark_text("仮",{0,3});
    const auto marked=h.text_input()->text;
    h.scroll_to(id,{0,3600});
    auto info=*keyed(h,"list").list;
    require(info.pinned=="0" && info.retained_editors<=info.mounted_count && info.mounted_count<=10, "Active offscreen editor adds at most one pinned row");
    require(h.text_input()->marked && h.text_input()->text==marked, "Scrolling preserves active input-method composition");
    h.unmark_text(true); require((*cells)[0].get().find("仮")!=std::string::npos, "Pinned composition commits to model-owned state");
    h.text_command(s::TextCommand::undo); require((*cells)[0].get()=="Draft", "Pinned editor preserves its undo transaction");
    h.inspect(); const auto before=hosts; (*cells)[150].set("Offscreen update"); h.inspect();
    require(hosts==before, "Unrealized cell state does not subscribe the host");
    h.focus(keyed(h,"leave").id); info=*keyed(h,"list").list;
    // Focusing Leave can reveal the list's parent, but must release the old pin.
    require(info.pinned.empty() && !h.text_input(), "Leaving the editor releases its virtualization pin");
    h.reveal_item(id,150); const auto remote=id+"/option/150/key/editor";
    require(h.focus(remote) && h.text_input()->text=="Offscreen update", "Newly realized cells read persistent application state");
    h.focus(keyed(h,"leave").id); h.scroll_to(id,{0,0});
    const auto count=hosts; (*cells)[150].set("Evicted update"); h.inspect();
    require(hosts==count, "Evicted row observers expire instead of retaining every visited row");
}
void model_changes_and_failures() {
    s::State<std::vector<std::string>> selected{{"c"}};
    s::State<s::RowModel> model{s::RowModel({{"a/b%","A"},{"b","B",false},{"c","C"}})};
    s::Host h([&]{return s::Block({s::ListBox(selected.binding(),model.get(),{.row_height=24,.visible_rows=2}).key("list"),s::Button("Next",[]{}).key("next")});},{},theme());
    h.layout({300,130}); auto id=keyed(h,"list").id;
    const auto escaped=id+"/option/a%2Fb%25";
    require(h.list_item(escaped)->index==0 && h.activate(escaped), "Escaped row identities round-trip through logical APIs");
    model.set(s::RowModel({{"c","C"},{"a/b%","A"},{"d","D"}})); h.inspect();
    require(h.list_item(escaped)->index==1 && h.list_item(escaped)->selected, "Reordering preserves selection by stable key");
    model.set(s::RowModel{}); h.inspect(); h.focus(id); h.handle(s::KeyEvent{s::Key::tab});
    require(element(h,keyed(h,"next").id).focused && keyed(h,"list").list->logical_count==0, "An empty list permits keyboard traversal and has no rows");
    require(!h.list_item(escaped), "Removed rows have no logical accessibility identity");
    bool duplicate{}; try { s::RowModel bad({{"x","X"},{"x","Again"}}); } catch(const std::invalid_argument&){duplicate=true;}
    require(duplicate,"Snapshots reject duplicate stable IDs");
    bool fail=true; s::State<std::string> selection{"0"};
    s::RowModel throwing(options(100),[&](const s::Option& row,std::size_t i){if(fail&&i>=40)throw std::runtime_error("row failure");return s::Label(row.label);});
    s::Host robust([&]{return s::ListBox(selection.binding(),throwing,{.row_height=24,.visible_rows=4}).key("list");},{},theme());
    robust.layout({300,100}); const auto rid=keyed(robust,"list").id;
    bool caught{}; try{robust.scroll_to(rid,{0,1200});}catch(const std::runtime_error&){caught=true;}
    require(caught,"Lazy builder exceptions reach the caller");
    fail=false; robust.layout({300,100});
    require(robust.list_item(rid,50)->bounds.has_value() && keyed(robust,"list").list->mounted_count<=9,"A failed realization can recover without stale nodes");
}
void multiple_selection() {
    auto data=options(1000); data[400].enabled=false;
    s::RowModel model(std::move(data));
    s::State<std::vector<std::string>> selected{{"0"}};
    s::Host h([&]{return s::ListBox(selected.binding(),model,{.row_height=24,.visible_rows=4}).key("list");},{},theme());
    h.layout({300,100}); const auto id=keyed(h,"list").id;
    h.focus(id); h.handle(s::KeyEvent{.key=s::Key::end,.shift=true});
    require(selected.get().size()==999 && !h.list_item(id,400)->selected && selected.get().back()=="999", "Distant Shift ranges select enabled logical rows without mounting every row");
    require(keyed(h,"list").list->mounted_count<=9 && element(h,id).selected_items.size()==999, "Large selections retain bounded mounting and expose logical selected indices");
    h.activate(id+"/option/500");
    require(!h.list_item(id,500)->selected && selected.get().size()==998, "Logical activation preserves multiple-selection toggle semantics");
    h.handle(s::KeyEvent{.key=s::Key::a,.command=true});
    require(selected.get().size()==999 && h.list_item(id,500)->selected, "Command-A selects enabled metadata independently of the realized range");
}
void lifecycle_and_overlays() {
    s::State<std::string> selected{"0"}, value{"Original"};
    s::State<bool> hidden{false}, shown{false};
    s::State<int> revision{0};
    auto make = [&](std::size_t count) {
        return s::RowModel(options(count), [&](const s::Option& row, std::size_t i) -> s::View {
            if (i == 0) return s::TextField("Value",value.binding()).key("editor");
            return s::Label(row.label);
        });
    };
    s::State<s::RowModel> model{make(300)};
    s::Host h([&] { revision.get(); return s::ListBox(selected.binding(),model.get(),{.row_height=32,.visible_rows=4}).key("list").hidden(hidden.get()); },{},theme());
    h.layout({300,130}); auto id=keyed(h,"list").id;
    h.focus(id+"/option/0/key/editor"); h.mark_text("仮",{0,3}); h.scroll_to(id,{0,3200});
    revision.set(1); h.inspect();
    require(h.text_input()->marked && keyed(h,"list").list->pinned=="0", "An unrelated host rebuild preserves the offscreen editor and composition");
    model.set(s::RowModel({{"other","Replacement"}})); h.inspect();
    require(!h.text_input() && value.get()=="Original" && keyed(h,"list").list->retained_editors==0, "Deleting an editing row cancels composition and removes retained buffers");
    model.set(make(300)); h.inspect(); h.reveal_item(id,0); h.focus(id+"/option/0/key/editor"); h.mark_text("隠",{0,3});
    hidden.set(true); h.inspect();
    require(!h.text_input() && !h.list_item(id,0) && keyed(h,"list").list->mounted_count==0, "Hiding a list releases realized editors and semantic rows");
    hidden.set(false); h.layout({0,0});
    require(keyed(h,"list").list->mounted_count==0, "A zero-height viewport realizes no rows");
    h.layout({300,130}); require(keyed(h,"list").list->mounted_count<=7, "Growing a hidden or zero viewport realizes a bounded range again");

    int dismissals{};
    s::State<std::string> choice{"0"};
    s::RowModel overlay_rows(options(100),[&](const s::Option& row,std::size_t i)->s::View {
        if(i!=50)return s::Label(row.label);
        return s::Button("Row details",[&]{shown.set(true);}).key("details")
            .popover(shown.binding(),[]{return s::Button("Dismiss",[]{}).key("dismiss");},
                     {.focus=s::PopoverFocus::preserve,.width=160,.on_dismiss=[&]{++dismissals;}});
    });
    s::Host pop([&]{return s::ListBox(choice.binding(),overlay_rows,{.row_height=32,.visible_rows=4}).key("list");},{},theme());
    pop.layout({360,160}); const auto lid=keyed(pop,"list").id;
    pop.reveal_item(lid,50); pop.activate(lid+"/option/50/key/details");
    require(shown.get() && keyed(pop,"dismiss").type=="Button", "Lazily realized rows participate in the shared overlay system");
    pop.scroll_to(lid,{0,0}); pop.inspect();
    require(!shown.get() && dismissals==1 && !pop.list_item(lid,50)->bounds, "Evicting an overlay owner dismisses it once without retaining row geometry");

    s::State<int> illegal{0};
    s::RowModel bad(options(100),[&](const s::Option& row,std::size_t i){if(i>20)illegal.set(1);return s::Label(row.label);});
    s::Host pure([&]{return s::ListBox(choice.binding(),bad,{.row_height=24,.visible_rows=4});},{},theme());
    pure.layout({300,100}); bool caught{};
    try{pure.scroll_to("root",{0,1200});}catch(const std::logic_error&){caught=true;}
    require(caught && illegal.get()==0, "Deferred row builders enforce the same state-mutation prohibition as body builders");
}

}
int main(){try{large_list();editing_and_observers();model_changes_and_failures();multiple_selection();lifecycle_and_overlays();std::cout<<checks<<" virtualization checks passed\n";return 0;}catch(const std::exception& e){std::cerr<<e.what()<<'\n';return 1;}}
