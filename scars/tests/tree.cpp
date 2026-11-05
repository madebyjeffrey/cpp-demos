#include "include/core/SkCanvas.h"
#include "include/core/SkSurface.h"
#include <algorithm>
#include <iostream>
#include <functional>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>
import scars;
namespace s=scars;
namespace {
int checks{};
void require(bool value,const char* message){++checks;if(!value)throw std::runtime_error(message);}
s::Theme theme(bool light=false){auto t=light?s::Theme::light():s::Theme{};t.font_path=SCARS_FONT_PATH;return t;}
s::InspectionNode keyed(s::Host& h,const std::string& key){for(const auto& n:h.inspect())if(n.key==key)return n;throw std::runtime_error("Missing key: "+key);}
s::Element element(s::Host& h,const std::string& id){for(const auto& e:h.elements())if(e.id==id)return e;throw std::runtime_error("Missing element: "+id);}
void key(s::Host& h,s::Key k,bool shift=false,bool command=false){h.handle(s::KeyEvent{.key=k,.shift=shift,.command=command});}
void navigation(){
 for(bool light:{false,true}){
    int loaded{}, expanded_callbacks{};
    s::TreeNode lazy{"archive","Archive"};
    lazy.lazy_children=[&]{++loaded;return std::vector<s::TreeNode>{{"old/1%","Archived record"}};};
    s::TreeModel model({{"fleet","Fleet",true,false,{{"nav","Navigation",true,false,{{"route","Route"}}},{"offline","Offline",false}}},std::move(lazy),{"loading","Telemetry",true,true}});
    s::State<std::string> selected{"fleet"};s::State<std::vector<std::string>> expanded;
    s::Host h([&]{return s::TreeView(selected.binding(),model,expanded.binding(),{.row_height=28,.visible_rows=4,.on_expand=[&](const auto&,bool){++expanded_callbacks;}}).key("tree");},{},theme(light));
    h.layout({320,150});const auto id=keyed(h,"tree").id;
    require(element(h,id).role==s::Role::tree_view && keyed(h,"tree").list->logical_count==3 && loaded==0,"Collapsed branches exclude descendants and defer lazy children");
    h.focus(id);key(h,s::Key::right);
    require(expanded.get()==std::vector<std::string>{"fleet"}&&selected.get()=="fleet"&&keyed(h,"tree").list->logical_count==5,"Right expands the active branch without changing selection");
    key(h,s::Key::right);require(selected.get()=="nav","Right on an expanded branch enters its first enabled child");
    key(h,s::Key::right);key(h,s::Key::right);require(selected.get()=="route","Nested right navigation expands and enters a child");
    auto route=h.list_item(id+"/option/route");
    require(route&&route->tree->depth==2&&route->tree->parent=="nav"&&element(h,route->id).role==s::Role::tree_item,"Logical and mounted tree rows expose hierarchy metadata");
    key(h,s::Key::left);require(selected.get()=="nav","Left returns from a leaf to its parent");
    key(h,s::Key::left);require(!h.list_item(id+"/option/route")&&selected.get()=="nav","Left collapses an expanded branch");
    key(h,s::Key::down);require(selected.get()=="archive","Vertical navigation skips disabled nodes");
    key(h,s::Key::right);require(loaded==1&&h.list_item(id+"/option/old%2F1%25")->tree->parent=="archive","Expansion materializes a lazy branch with stable escaped identity");
    key(h,s::Key::right);require(selected.get()=="old/1%","A lazy child participates in normal keyboard selection");
    require(h.set_expanded(id+"/option/archive",false)&&selected.get()=="old/1%"&&keyed(h,"tree").list->active=="archive","Collapsing an active descendant retains model selection and moves logical focus to its ancestor");
    require(h.set_expanded(id+"/option/archive",true)&&loaded==1&&h.list_item(id+"/option/old%2F1%25")->selected,"Reopening uses cached metadata and restores visible selection");
    require(!h.set_expanded(id+"/option/offline",true)&&!h.focus(id+"/option/offline"),"Disabled nodes reject expansion and focus");
    key(h,s::Key::end);key(h,s::Key::right);
    require(selected.get()=="loading"&&h.list_item(id+"/option/loading")->tree->loading&&h.list_item(id+"/option/loading")->tree->expanded,"Loading branches expose expansion without fabricating children");
    h.handle(s::KeyEvent{.text="f"});require(selected.get()=="fleet","Typeahead searches visible tree labels");
    const auto fleet=element(h,id+"/option/fleet");
    h.handle(s::PointerEvent{s::PointerKind::down,{fleet.bounds.x+12,fleet.bounds.y+14}});
    require(!h.list_item(id+"/option/nav")&&expanded_callbacks>=5,"Disclosure clicks collapse branches through the shared input path");
    auto raster=SkSurfaces::Raster(SkImageInfo::MakeN32Premul(640,300));raster->getCanvas()->scale(2,2);h.paint(*raster->getCanvas(),0);
    require(!h.needs_frame(),"A static tree does not animate while idle");
 }
}
void large_and_multiple(){
 int loads{},hosts{};s::TreeNode root{"root","Dataset"};root.lazy_children=[&]{++loads;std::vector<s::TreeNode> rows;rows.reserve(100000);for(int i=0;i<100000;++i)rows.push_back({std::to_string(i),"Record "+std::to_string(i)});rows[500].enabled=false;return rows;};
 s::TreeModel model({std::move(root)});s::State<std::vector<std::string>> expanded,selected{{"root"}};
 s::Host h([&]{++hosts;return s::TreeView(selected.binding(),model,expanded.binding(),{.row_height=24,.visible_rows=5}).key("tree");},{},theme());
 h.layout({300,140});const auto id=keyed(h,"tree").id;h.focus(id);key(h,s::Key::right);
 require(loads==1&&keyed(h,"tree").list->logical_count==100001&&keyed(h,"tree").list->mounted_count<=12,"A large lazy branch expands metadata while keeping mounted rows bounded");
 require(h.tree_child(id+"/option/root",99999)->index==100000 && !h.tree_child(id+"/option/root",100000),"Immediate logical child access is indexed and bounded by the branch's metadata");
 const auto before=hosts;for(int i=0;i<20;++i)h.scroll_to(id,{0,double(i)*60000});
 require(hosts==before&&loads==1&&keyed(h,"tree").list->mounted_count<=12,"Scrolling does not rebuild, reload, or reflatten the hierarchy");
 key(h,s::Key::end,true);require(selected.get().size()==100000&&selected.get().back()=="99999"&&!h.list_item(id+"/option/500")->selected,"Distant Shift ranges select enabled logical tree rows");
 require(h.list_item(id,100000)->tree->depth==1&&keyed(h,"tree").tree->known_count==100001,"The inspector and logical rows expose full tree counts and depth");
 h.set_expanded(id+"/option/root",false);
 require(keyed(h,"tree").list->logical_count==1&&selected.get().size()==100000,"Collapsing a large branch retains business selection outside the visible row model");
 key(h,s::Key::a,false,true);require(selected.get()==std::vector<std::string>{"root"},"Select-all operates on currently expanded visible nodes");
}
void snapshots_and_failures(){
 bool duplicate{};try{s::TreeModel bad({{"a","A",true,false,{{"b","B"}}},{"b","Duplicate"}});}catch(const std::invalid_argument&){duplicate=true;}
 require(duplicate,"Tree IDs are globally unique across eager subtrees");
 bool fail=true;int calls{};s::TreeNode node{"lazy","Lazy"};node.lazy_children=[&]{++calls;return std::vector<s::TreeNode>{{fail?"lazy":"valid","Child"}};};s::TreeModel model({std::move(node)});
 const std::vector<std::string> open{"lazy"};bool caught{};try{model.flatten(open);}catch(const std::invalid_argument&){caught=true;}
 require(caught&&model.known_count()==1,"Invalid lazy subtrees leave the materialized snapshot unchanged");
 fail=false;require(model.flatten(open).size()==2&&calls==2,"Failed lazy loading can recover on a later expansion");
 s::TreeNode empty{"empty","Empty"};empty.lazy_children=[] {return std::vector<s::TreeNode>{};};s::TreeModel empties({std::move(empty)});
 require(!empties.flatten(std::vector<std::string>{"empty"})[0].expandable,"An empty lazy result becomes a leaf");
 s::State<std::string> selected{"child"};s::State<std::vector<std::string>> expanded{{"parent"}};s::State<bool> hidden{false};
 s::State<s::TreeModel> snapshot{s::TreeModel({{"parent","Parent",true,false,{{"child","Child"}}}})};
 s::Host h([&]{return s::TreeView(selected.binding(),snapshot.get(),expanded.binding()).key("tree").hidden(hidden.get());},{},theme());h.layout({300,160});auto id=keyed(h,"tree").id;
 snapshot.set(s::TreeModel({{"other","Other"},{"parent","Renamed",true,false,{{"child","Renamed child"}}}}));
 require(h.list_item(id+"/option/child")->selected&&h.list_item(id+"/option/child")->label=="Renamed child","Snapshot replacement preserves selected identity while updating order and labels");
 snapshot.set(s::TreeModel({{"other","Other"}}));require(!h.list_item(id+"/option/child")&&selected.get()=="child","Deleting a selected node removes its scene identity without silently writing model state");
 hidden.set(true);require(h.elements().empty()&&!h.list_item(id,0),"Hidden trees expose no semantic rows");
}
void lifecycle() {
    int loads{},changes{};s::TreeNode branch{"branch","Deferred branch"};branch.lazy_children=[&]{++loads;return std::vector<s::TreeNode>{{"child","Child"}};};
    s::TreeModel model({std::move(branch)});s::State<std::string> selected{"branch"};s::State<std::vector<std::string>> expanded{{"branch"}};s::State<bool> hidden{true},mounted{true};
    s::Host h([&]()->s::View{if(!mounted.get())return s::Button("Replacement",[]{});return s::TreeView(selected.binding(),model,expanded.binding(),{.on_expand=[&](const auto&,bool){++changes;mounted.set(false);}}).hidden(hidden.get());},{},theme());
    h.layout({300,160});require(loads==0,"Initially hidden trees do not invoke deferred child loaders");
    hidden.set(false);require(h.list_item("root",1)->label=="Child"&&loads==1,"Showing the tree materializes the explicitly expanded branch");
    require(h.set_expanded("root/option/branch",false)&&changes==1&&h.elements().size()==1&&h.elements()[0].role==s::Role::button,"Expansion callbacks may replace the tree safely and complete exactly once");
    s::State<int> illegal{0};s::TreeNode bad{"bad","Bad"};bad.lazy_children=[&]{illegal.set(1);return std::vector<s::TreeNode>{};};
    s::TreeModel broken({std::move(bad)});s::State<std::vector<std::string>> open{{"bad"}};
    s::Host pure([&]{return s::TreeView(selected.binding(),broken,open.binding());},{},theme());bool caught{};try{pure.layout({300,160});}catch(const std::logic_error&){caught=true;}
    require(caught&&illegal.get()==0,"Lazy tree loaders cannot mutate observable state during view construction");
    s::TreeNode nested{"leaf","Deep leaf"};for(int i=0;i<300;++i){s::TreeNode parent{"level/"+std::to_string(i),"Deep branch"};parent.children.push_back(std::move(nested));nested=std::move(parent);}
    s::TreeModel deep({std::move(nested)});std::vector<std::string> ids;for(int i=0;i<300;++i)ids.push_back("level/"+std::to_string(i));
    auto flat=deep.flatten(ids);require(flat.size()==301&&flat.back().depth==300,"Deep trees flatten iteratively while preserving complete ancestry");
}

void inspector_tree() {
    int commands{};
    auto source=std::make_shared<s::Host>([&]{std::vector<s::View> buttons;for(int i=0;i<1000;++i)buttons.push_back(s::Button("Button "+std::to_string(i),[&]{++commands;}).key(std::to_string(i)));return s::Block(std::move(buttons));},std::function<void()>{},theme());
    source->layout({400,300});
    auto inspector=std::make_shared<s::Inspector>(source);
    s::Host explorer([&]{return inspector->body();},{},theme());
    source->set_inspection_observer([&]{explorer.invalidate();});
    explorer.layout({980,760}); auto node=keyed(explorer,"element-tree"); const auto id=node.id;
    require(node.type=="TreeView"&&node.tree->known_count==1001&&node.list->mounted_count<=24,"The inspector uses the shared virtualized tree for large scene hierarchies");
    source->set_inspected_element("root/key/999");inspector->reveal_selection();
    auto last=explorer.list_item(id,1000);
    require(last&&last->selected&&last->bounds&&last->clip->height>0,"Picking or externally selecting a source view reveals its logical inspector row");
    auto root=explorer.list_item(id,0)->id;
    explorer.set_expanded(root,false);
    require(keyed(explorer,"element-tree").list->logical_count==1&&source->inspect().size()==1001,"Inspector expansion changes presentation without unmounting source views");
    inspector->reveal_selection();require(keyed(explorer,"element-tree").list->logical_count==1001,"Reveal selection restores the source node's ancestor path");
    std::string filter;for(const auto& e:explorer.elements())if(e.label=="Filter elements")filter=e.id;
    require(!filter.empty()&&explorer.activate(filter),"Inspector filtering remains accessible");
    explorer.activate(filter+"/menu/layout");
    require(keyed(explorer,"element-tree").list->logical_count==1,"The layout filter narrows the tree while retaining structural context");
    explorer.activate(filter);explorer.activate(filter+"/menu/all");
    auto first=explorer.list_item(id,1);
    require(first&&explorer.activate(first->id)&&source->inspected_element()=="root/key/0"&&commands==0,"Shared tree selection inspects source controls without activating them");
    source->set_inspection_observer({});
}

}
int main(){try{navigation();large_and_multiple();snapshots_and_failures();lifecycle();inspector_tree();std::cout<<checks<<" tree checks passed\n";return 0;}catch(const std::exception& e){std::cerr<<e.what()<<'\n';return 1;}}
