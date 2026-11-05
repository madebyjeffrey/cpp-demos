#include "include/core/SkBitmap.h"
#include "include/core/SkCanvas.h"
#include "include/core/SkSurface.h"
#include <algorithm>
#include <cmath>
#include <functional>
#include <iostream>
#include <limits>
#include <stdexcept>
#include <string>
#include <thread>
#include <vector>
import scars;
namespace s = scars;
namespace {
int checks{};
void require(bool condition, const std::string& message) {
    ++checks;
    if (!condition)
        throw std::runtime_error(message);
}
bool near(double a, double b) {
    return std::abs(a - b) < .05;
}
s::Element named(s::Host& host, const std::string& label) {
    for (auto& e : host.elements())
        if (e.label == label)
            return e;
    throw std::runtime_error("Missing element: " + label);
}
s::Element keyed(s::Host& host, const std::string& key) {
    for (auto& e : host.elements())
        if (e.id.ends_with("/key/" + key))
            return e;
    throw std::runtime_error("Missing key: " + key);
}
s::Element focused(s::Host& host) {
    for (auto& e : host.elements())
        if (e.focused)
            return e;
    throw std::runtime_error("No focused element");
}
void click(s::Host& h, s::Point p) {
    h.handle(s::PointerEvent{s::PointerKind::down, p});
    h.handle(s::PointerEvent{s::PointerKind::up, p});
}
void press(s::Host& h, s::Key key, bool shift = false) {
    h.handle(s::KeyEvent{key, true, false, shift});
}
void expect_throw(const std::function<void()>& fn, const std::string& message) {
    bool threw = false;
    try {
        fn();
    } catch (const std::exception&) {
        threw = true;
    }
    require(threw, message);
}
s::Theme theme() {
    s::Theme t;
    t.font_path = SCARS_FONT_PATH;
    return t;
}
void reactive() {
    s::State<int> a{1}, b{2};
    s::State<bool> choose{true};
    int builds = 0, wakes = 0;
    s::Host host(
        [&] {
            ++builds;
            return s::Label(std::to_string(choose.get() ? a.get() : b.get()));
        },
        [&] { ++wakes; }, theme());
    host.layout({200, 100});
    a.set(3);
    a.set(4);
    require(wakes == 1, "State writes coalesce");
    host.layout({200, 100});
    require(builds == 2 && named(host, "4").role == s::Role::text, "State rebuild uses latest value");
    a.set(4);
    require(!host.needs_frame(), "Equal state does not invalidate");
    choose.set(false);
    host.layout({200, 100});
    a.set(5);
    require(!host.needs_frame(), "Removed state dependency unsubscribes");
    b.set(7);
    require(host.needs_frame(), "New state dependency invalidates");
    host.layout({200, 100});
    s::Host bad([&] {
        a.set(10);
        return s::Label("Bad");
    });
    expect_throw([&] { bad.layout({100, 100}); }, "View builders cannot mutate state");
    bool thread_rejected = false;
    std::thread other([&] {
        try {
            a.set(10);
        } catch (const std::logic_error&) {
            thread_rejected = true;
        }
    });
    other.join();
    require(thread_rejected, "State rejects cross-thread mutation");
    {
        s::Host temporary([&] { return s::Label(std::to_string(a.get())); });
        temporary.layout({100, 100});
    }
    a.set(6);
    require(a.get() == 6, "State can outlive mounted host");
    int bound_wakes = 0;
    s::State<bool> binding{false};
    s::View static_view = s::CheckBox("Bound", binding.binding());
    s::Host bound([&] { return static_view; }, [&] { ++bound_wakes; });
    bound.layout({200, 100});
    binding.set(true);
    require(bound_wakes == 1, "Bindings subscribe even when builder does not read State");
}
void layout() {
    s::View base = s::Button("Base", [] {}).width(80).height(32);
    auto styled = base.width(120);
    s::Host host(
        [&] {
            return s::FlexColumn{
                s::FlexRow{base.key("base"), styled.key("styled"),
                           s::Button("Growing", [] {}).grow().max_width(200).key("growing")}
                    .gap(10)
                    .height(40),
                s::Block{s::Label("Block one").key("block1"), s::Label("Block two").key("block2")}.gap(7).key(
                    "block")}
                .padding(10)
                .gap(10);
        },
        {}, theme());
    host.layout({500, 300});
    auto a = keyed(host, "base"), b = keyed(host, "styled"), g = keyed(host, "growing");
    require(near(a.bounds.width, 80) && near(b.bounds.width, 120), "Modifiers preserve value semantics");
    require(near(g.bounds.width, 200), "Flex grow freezes at maximum");
    require(near(a.bounds.x, 10) && near(b.bounds.x, 100), "Padding and gaps participate in layout");
    auto one = keyed(host, "block1"), two = keyed(host, "block2");
    require(near(one.bounds.width, 480) && near(two.bounds.y - one.bounds.y - one.bounds.height, 7),
            "Block fills available width and stacks children");
    s::Host shrink([] {
        return s::FlexRow{s::Button("A", [] {}).width(150).min_width(100).key("a"),
                          s::Button("B", [] {}).width(150).key("b")}
            .gap(10);
    });
    shrink.layout({200, 40});
    require(near(keyed(shrink, "a").bounds.width, 100) && near(keyed(shrink, "b").bounds.width, 90),
            "Flex shrink redistributes after minimum clamp");
    s::Host wrap([] {
        return s::FlexRow{s::Button("One", [] {}).width(100).key("one"),
                          s::Button("Two", [] {}).width(100).key("two"),
                          s::Button("Three", [] {}).width(100).key("three")}
            .wrap()
            .gap(8)
            .align_items(s::Align::start);
    });
    wrap.layout({220, 200});
    require(near(keyed(wrap, "one").bounds.y, keyed(wrap, "two").bounds.y) &&
                keyed(wrap, "three").bounds.y > keyed(wrap, "one").bounds.y,
            "Flex wrapping creates lines");
    s::Host percentages([] {
        return s::FlexColumn{
            s::Label("Half").width(s::percent(50)).margin(s::Insets::symmetric(0, 10)).key("half")}
            .padding(10);
    });
    percentages.layout({240, 100});
    require(near(keyed(percentages, "half").bounds.width, 110),
            "Percent width resolves against parent content box");
    s::State<bool> expanded{true};
    s::Host collapse([&] {
        return s::Block{
            s::Section("Heading", s::Button("Child", [] {})).expanded(expanded.binding()).key("section"),
            s::Label("After").key("after")}
            .gap(8);
    });
    collapse.layout({400, 200});
    double before = keyed(collapse, "after").bounds.y;
    collapse.activate(keyed(collapse, "section").id);
    collapse.layout({400, 200});
    require(!expanded.get() && keyed(collapse, "after").bounds.y < before,
            "Collapsed content is removed from layout");
    auto collapsed_elements = collapse.elements();
    require(std::none_of(collapsed_elements.begin(), collapsed_elements.end(),
                         [](const auto& e) { return e.label == "Child"; }),
            "Collapsed controls leave semantic tree");
    s::Host blocks([] {
        return s::Block{s::Button("First block", [] {}).height(40).key("first"),
                        s::Button("Second block", [] {}).height(40).key("second")}
            .gap(8);
    });
    blocks.layout({200, 50});
    require(near(keyed(blocks, "first").bounds.height, 40) && near(keyed(blocks, "second").bounds.y, 48),
            "Block flow overflows its viewport without flex shrinking children");
    s::Host duplicates([] { return s::FlexRow{s::Label("A").key("same"), s::Label("B").key("same")}; });
    expect_throw([&] { duplicates.layout({200, 100}); }, "Duplicate sibling keys rejected");
}
void inline_flow() {
    int actions = 0;
    s::Host host(
        [&] {
            return s::Block{
                s::Inline{s::Label("A sentence with styled "),
                          s::Label("words ").bold().foreground(0xff4dd2ff),
                          s::Button("Inline", [&] { ++actions; }).density(s::Density::compact).key("inline"),
                          s::Label(" and more words that wrap onto a fresh line.")}
                    .key("flow"),
                s::Label("After flow").key("after")}
                .gap(10);
        },
        {}, theme());
    host.layout({700, 400});
    double wide = keyed(host, "after").bounds.y;
    host.layout({180, 400});
    auto button = keyed(host, "inline");
    require(keyed(host, "after").bounds.y > wide, "Inline text remeasures at allocated width");
    require(near(button.bounds.height, 32) && button.bounds.width <= 180,
            "Inline control remains an indivisible box");
    click(host, {button.bounds.x + button.bounds.width / 2, button.bounds.y + 16});
    require(actions == 1, "Embedded controls hit-test their arranged position");
    auto surface = SkSurfaces::Raster(SkImageInfo::MakeN32Premul(180, 400));
    host.paint(*surface->getCanvas());
    require(!host.needs_frame(), "Static inline content does not animate");
    s::Host unicode([] { return s::Label("Café 日本語 🚀").text_wrap(s::TextWrap::break_word).width(35); },
                    {}, theme());
    unicode.layout({100, 300});
    unicode.paint(*surface->getCanvas());
    require(named(unicode, "Café 日本語 🚀").bounds.height > 18, "BreakWord handles complete UTF-8 scalars");
}
void choices() {
    s::State<bool> toggle{false}, disabled{false};
    s::State<s::CheckState> mixed{s::CheckState::indeterminate};
    s::State<std::string> radio{"a"}, segment{"a"};
    int clicks = 0;
    s::Host host(
        [&] {
            return s::FlexColumn{
                s::Button("Command", [&] { ++clicks; }).key("command"),
                s::ToggleButton("Toggle", toggle.binding()).key("toggle"),
                s::CheckBox("Mixed", mixed.binding()).key("mixed"),
                s::CheckBox("Unavailable", disabled.binding()).enabled(false),
                s::RadioGroup(radio.binding(), {{"a", "Radio A"}, {"b", "Radio B", false}, {"c", "Radio C"}})
                    .key("radio"),
                s::SegmentedControl(segment.binding(), {{"a", "Segment A"}, {"b", "Segment B"}})
                    .key("segments")}
                .gap(4);
        },
        {}, theme());
    host.layout({400, 500});
    auto command = keyed(host, "command");
    host.focus(command.id);
    press(host, s::Key::space);
    host.handle(s::KeyEvent{s::Key::space, true, true});
    require(clicks == 1, "Commands execute once and ignore key repeat");
    host.activate(keyed(host, "toggle").id);
    require(toggle.get(), "Toggle persists independent state");
    auto checkbox = keyed(host, "mixed");
    require(checkbox.mixed, "Mixed checkbox exposes semantic state");
    click(host, {checkbox.bounds.x + checkbox.bounds.width - 5, checkbox.bounds.y + 18});
    require(mixed.get() == s::CheckState::checked, "Label row toggles mixed checkbox to checked");
    host.activate(checkbox.id);
    require(mixed.get() == s::CheckState::unchecked, "Checked checkbox toggles to unchecked");
    require(!host.activate(named(host, "Unavailable").id) && !disabled.get(),
            "Disabled controls reject activation");
    host.focus(named(host, "Radio A").id);
    press(host, s::Key::right);
    require(radio.get() == "c" && focused(host).label == "Radio C",
            "Radio arrow navigation skips disabled options");
    press(host, s::Key::space);
    require(radio.get() == "c", "Selected radio cannot be deselected");
    press(host, s::Key::tab);
    require(focused(host).label == "Segment A", "Radio group has a single tab stop");
    press(host, s::Key::right);
    require(segment.get() == "b", "Segments select exclusively with arrow keys");
    host.blur();
    auto blurred = host.elements();
    require(std::none_of(blurred.begin(), blurred.end(), [](const auto& e) { return e.focused; }),
            "Window blur removes active focus indication");
    host.set_active(true);
    require(focused(host).label == "Segment B", "Window activation restores retained keyboard focus");
}
void tab_focus_mode() {
    s::State<bool> checked{false};
    auto builder = [&] {
        return s::Block{s::Button("Command", [] {}).key("command"),
                        s::CheckBox("Grid overlay", checked.binding()).key("check")}
            .padding(24)
            .gap(16);
    };
    s::Host host(builder, {}, theme()), reference(builder, {}, theme());
    host.layout({320, 180});
    reference.layout({320, 180});
    auto snapshot = [](s::Host& h, double time = 0) {
        auto surface = SkSurfaces::Raster(SkImageInfo::MakeN32Premul(320, 180));
        h.paint(*surface->getCanvas(), time);
        SkBitmap image;
        image.allocN32Pixels(320, 180);
        if (!surface->readPixels(image, 0, 0))
            throw std::runtime_error("Cannot capture focus state");
        return image;
    };
    auto different = [](const SkBitmap& a, const SkBitmap& b) {
        for (int y = 0; y < a.height(); ++y)
            for (int x = 0; x < a.width(); ++x)
                if (a.getColor(x, y) != b.getColor(x, y))
                    return true;
        return false;
    };
    press(host, s::Key::tab);
    auto command_focus = snapshot(host);
    require(different(command_focus, snapshot(reference)), "Tab displays accents on the focused control");
    require(!different(command_focus, snapshot(host, 3600)), "Tab accents persist while idle without fading");
    press(host, s::Key::tab);
    require(focused(host).label == "Grid overlay", "Tab moves accents and focus to the next control");
    auto checkbox_focus = snapshot(host);
    host.handle(s::PointerEvent{s::PointerKind::move, {300, 170}});
    require(!different(checkbox_focus, snapshot(host)), "Passive pointer motion does not end Tab mode");
    press(host, s::Key::space);
    require(checked.get() && !different(snapshot(host), snapshot(reference)),
            "Activation ends Tab accents while retaining control focus");
    press(host, s::Key::tab, true);
    require(different(snapshot(host), snapshot(reference)), "Shift-Tab re-enters accented navigation");
    host.handle(s::PointerEvent{s::PointerKind::scroll, {300, 170}, 0, 0, -10});
    require(!different(snapshot(host), snapshot(reference)), "An explicit scroll action ends Tab mode");
}
void menus() {
    s::State<std::string> selected{"a"}, action{"a"};
    int run_a = 0, run_b = 0, underlying = 0;
    s::Host host(
        [&] {
            return s::FlexColumn{
                s::MenuButton("Actions", {{"a", "Action A", [&] { ++run_a; }},
                                          {"x", "Disabled", [&] { ++run_b; }, false},
                                          {"b", "Action B", [&] { ++run_b; }}})
                    .key("menu"),
                s::SelectionDropdown(selected.binding(), {{"a", "Value A"}, {"b", "Value B"}})
                    .key("selection"),
                s::SplitActionButton(action.binding(), {{"a", "Execute A", [&] { ++run_a; }},
                                                        {"b", "Execute B", [&] { ++run_b; }}})
                    .key("split"),
                s::Button("Underlying", [&] { ++underlying; }).key("under")}
                .gap(10)
                .padding(10);
        },
        {}, theme());
    host.layout({400, 400});
    auto menu = keyed(host, "menu");
    host.activate(menu.id);
    require(host.popup_open(), "Menu opens overlay");
    press(host, s::Key::down);
    press(host, s::Key::enter);
    require(run_b == 1 && named(host, "Actions").label == "Actions",
            "Menu selection executes command without changing caption");
    host.activate(keyed(host, "selection").id);
    press(host, s::Key::down);
    press(host, s::Key::enter);
    require(selected.get() == "b" && run_b == 1 && keyed(host, "selection").value_text == "Value B",
            "Selection dropdown changes value without executing action");
    auto split = keyed(host, "split");
    host.activate(split.id + "/popup");
    press(host, s::Key::down);
    press(host, s::Key::enter);
    require(action.get() == "b" && run_b == 1, "Split popup changes remembered action without execution");
    host.activate(split.id);
    require(run_b == 2, "Split main region executes remembered command");
    require(focused(host).id == split.id + "/popup", "Split popup restores arrow focus");
    host.activate(menu.id);
    auto under = keyed(host, "under");
    click(host, {under.bounds.x + 10, under.bounds.y + 20});
    require(!host.popup_open() && underlying == 0, "Outside click dismisses and is consumed");
    host.activate(menu.id);
    press(host, s::Key::escape);
    require(!host.popup_open() && focused(host).id == menu.id, "Escape restores trigger focus");
    host.activate(menu.id);
    require(!host.activate(menu.id + "/menu/x") && run_b == 2,
            "Disabled menu item rejects semantic activation");
    press(host, s::Key::escape);
    host.focus(split.id);
    press(host, s::Key::tab);
    require(focused(host).id == split.id + "/popup", "Split regions have separate tab stops");
    press(host, s::Key::space);
    require(host.popup_open(), "Keyboard activates split arrow independently");
    press(host, s::Key::escape);
    s::State<std::string> long_value{"0"};
    std::vector<s::Option> options;
    for (int i = 0; i < 40; ++i)
        options.push_back({std::to_string(i), "Option " + std::to_string(i)});
    s::Host long_menu(
        [&] {
            return s::FlexColumn{s::Spacer{}, s::SelectionDropdown(long_value.binding(), options).key("long")}
                .padding(8);
        },
        {}, theme());
    long_menu.layout({240, 200});
    auto trigger = keyed(long_menu, "long");
    long_menu.activate(trigger.id);
    auto popup = long_menu.popup_bounds();
    require(popup.y >= 0 && popup.y + popup.height <= 200 && popup.y < trigger.bounds.y,
            "Popup flips and stays inside viewport");
    press(long_menu, s::Key::end);
    auto last = named(long_menu, "Option 39");
    require(last.focused && last.bounds.y >= popup.y &&
                last.bounds.y + last.bounds.height <= popup.y + popup.height,
            "Long popup scrolls highlighted option into view");
    press(long_menu, s::Key::enter);
    require(long_value.get() == "39", "Long popup commits last item");
}
void element_inspection() {
    s::State<bool> checked{false}, show{true};
    int commands = 0, updates = 0;
    auto source = std::make_shared<s::Host>(
        [&] {
            std::vector<s::View> children{
                s::FlexRow{s::Label("Annotation", s::LabelStyle::micro).key("label"),
                           s::Button("Run", [&] { ++commands; }).key("run")}
                    .gap(12)
                    .key("row")};
            if (show.get())
                children.push_back(s::CheckBox("Inspectable checkbox", checked.binding()).key("check"));
            children.push_back(s::Button("Disabled", [] {}).enabled(false).key("disabled"));
            return s::Block(std::move(children)).padding(20).gap(12);
        },
        std::function<void()>{}, theme());
    source->layout({500, 300});
    source->set_inspection_observer([&] { ++updates; });
    auto tree = source->inspect();
    require(tree.front().type == "Block" && tree.front().parent_id.empty(),
            "Inspector includes the nonsemantic root container");
    auto row = std::find_if(tree.begin(), tree.end(), [](const auto& n) { return n.key == "row"; });
    require(row != tree.end() && row->parent_id == "root" && row->depth == 1 && near(row->gap, 12),
            "Inspector preserves hierarchy and declared layout properties");
    auto annotation = std::find_if(tree.begin(), tree.end(), [](const auto& n) { return n.key == "label"; });
    require(annotation != tree.end() && near(annotation->font_size, 11) &&
                near(annotation->line_height, 14) && near(annotation->letter_spacing, .5),
            "Inspector reports the measured micro-label typography");
    auto button = std::find_if(tree.begin(), tree.end(), [](const auto& n) { return n.key == "run"; });
    require(button != tree.end() && button->background == theme().primary &&
                button->foreground == theme().window && near(button->corner_radius, theme().button_radius),
            "Inspector reports computed button paint colours and geometry");
    auto check = keyed(*source, "check");
    source->set_inspected_element(check.id);
    require(source->inspected_element() == check.id && updates > 0 && !checked.get(),
            "Selecting an inspection target does not change application state");
    auto disabled = keyed(*source, "disabled");
    source->set_inspection_picking(true);
    click(*source, {disabled.bounds.x + 10, disabled.bounds.y + 15});
    require(source->inspected_element() == disabled.id && !source->inspection_picking(),
            "Pick mode can inspect disabled controls");
    auto command = keyed(*source, "run");
    source->set_inspection_picking(true);
    click(*source, {command.bounds.x + 10, command.bounds.y + 15});
    require(source->inspected_element() == command.id && commands == 0,
            "Pick selection consumes the full click without executing commands");
    source->set_inspection_picking(true);
    press(*source, s::Key::escape);
    require(!source->inspection_picking() && commands == 0, "Escape cancels picking without activation");
    auto model = std::make_shared<s::Inspector>(source);
    auto explorer =
        std::make_shared<s::Host>([model] { return model->body(); }, std::function<void()>{}, theme());
    std::weak_ptr<s::Host> weak = explorer;
    source->set_inspection_observer([weak] {
        if (auto view = weak.lock())
            view->invalidate();
    });
    explorer->layout({980, 760});
    auto inspect_check = named(*explorer, "Inspect CheckBox check");
    explorer->activate(inspect_check.id);
    require(source->inspected_element() == check.id && !checked.get(),
            "Element-tree rows select source views without activating them");
    checked.set(true);
    require(explorer->needs_frame(), "Application state invalidation refreshes the explorer without polling");
    explorer->layout({980, 760});
    tree = source->inspect();
    auto value = std::find_if(tree.begin(), tree.end(), [&](const auto& n) { return n.id == check.id; });
    require(value != tree.end() && value->selected && value->value == 1,
            "Inspector snapshots expose current bound values and selection");
    const auto expanded_count = explorer->elements().size();
    explorer->set_expanded(named(*explorer, "Inspect Block").id, false);
    require(explorer->elements().size() < expanded_count,
            "Explorer branches collapse independently of application layout");
    require(source->inspect().size() == tree.size(), "Collapsing the explorer leaves source views mounted");
    show.set(false);
    source->inspect();
    require(source->inspected_element().empty(),
            "Unmounting the selected view clears stale inspection selection");
    source->set_inspection_observer({});
    auto generation = source->generation();
    source->invalidate();
    source->set_inspected_element({});
    require(source->generation() == generation,
            "Clearing inspection does not evaluate a pending application view rebuild");
    auto surface = SkSurfaces::Raster(SkImageInfo::MakeN32Premul(980, 760));
    explorer->paint(*surface->getCanvas());
    require(!explorer->needs_frame(), "Idle explorer content does not schedule animation frames");
}
s::InspectionNode inspected(s::Host& host, const std::string& key) {
    for (const auto& n : host.inspect())
        if (n.key == key)
            return n;
    throw std::runtime_error("Missing inspection key: " + key);
}
void layers_and_positioning() {
    s::Host intrinsic(
        [] {
            return s::Block{s::Layer{s::Button("One", [] {}).width(100).height(30).margin(5),
                                     s::Button("Two", [] {}).width(80).height(60)}
                                .key("layer")}
                .align_items(s::Align::start);
        },
        {}, theme());
    intrinsic.layout({400, 240});
    require(near(inspected(intrinsic, "layer").bounds.width, 110) &&
                near(inspected(intrinsic, "layer").bounds.height, 60),
            "Layer intrinsic size is the maximum child extent rather than their sum");
    s::Host centered(
        [] {
            return s::Layer{
                s::Panel(s::Spacer{}).key("fill"),
                s::Button("Center", [] {}).width(80).height(30).align_self(s::Align::center).key("center")};
        },
        {}, theme());
    centered.layout({200, 120});
    require(near(inspected(centered, "fill").bounds.width, 200) &&
                near(inspected(centered, "fill").bounds.height, 120) &&
                near(keyed(centered, "center").bounds.x, 60) && near(keyed(centered, "center").bounds.y, 45),
            "Layers stretch automatic children and align explicitly sized children");
    s::Host positioned(
        [] {
            return s::Block{
                s::Button("Flow A", [] {}).width(80).height(30).key("a"),
                s::Button("Absolute", [] {})
                    .position(s::Position::absolute)
                    .left(10)
                    .top(20)
                    .width(100)
                    .height(50)
                    .key("absolute"),
                s::Button("Percent", [] {})
                    .position(s::Position::absolute)
                    .left(s::percent(25))
                    .top(s::percent(10))
                    .width(s::percent(50))
                    .height(s::percent(20))
                    .key("percent"),
                s::Panel(s::Spacer{})
                    .position(s::Position::absolute)
                    .left(10)
                    .right(20)
                    .top(10)
                    .bottom(20)
                    .margin(2)
                    .key("stretch"),
                s::Button("Corner", [] {})
                    .position(s::Position::absolute)
                    .right(8)
                    .bottom(12)
                    .width(80)
                    .height(40)
                    .key("corner"),
                s::Button("Negative", [] {})
                    .position(s::Position::absolute)
                    .left(-25)
                    .top(-23)
                    .width(40)
                    .height(20)
                    .key("negative"),
                s::Button("Left wins", [] {})
                    .position(s::Position::absolute)
                    .left(20)
                    .right(0)
                    .top(0)
                    .width(70)
                    .height(20)
                    .key("conflict"),
                s::Label("A paragraph long enough to wrap across several lines at the anchored width.")
                    .text_wrap(s::TextWrap::wrap)
                    .position(s::Position::absolute)
                    .left(0)
                    .right(200)
                    .top(0)
                    .key("reflow"),
                s::Button("Flow B", [] {}).width(80).height(30).key("b")}
                .padding(20)
                .gap(10)
                .key("root");
        },
        {}, theme());
    positioned.layout({400, 240});
    require(near(keyed(positioned, "a").bounds.y, 20) && near(keyed(positioned, "b").bounds.y, 60) &&
                near(inspected(positioned, "root").measured.height, 110),
            "Absolute children do not reserve block flow space or create extra gaps");
    auto absolute = inspected(positioned, "absolute");
    require(near(absolute.bounds.x, 30) && near(absolute.bounds.y, 40) &&
                absolute.position == s::Position::absolute && near(absolute.left.value, 10),
            "Absolute positioning uses the immediate parent's content box");
    auto percentage = keyed(positioned, "percent").bounds;
    require(near(percentage.x, 110) && near(percentage.y, 40) && near(percentage.width, 180) &&
                near(percentage.height, 40),
            "Absolute percentage offsets and dimensions resolve against the containing content box");
    auto stretch = inspected(positioned, "stretch").bounds;
    require(near(stretch.x, 32) && near(stretch.y, 32) && near(stretch.width, 326) &&
                near(stretch.height, 166),
            "Opposing absolute anchors stretch automatic sizes and account for margins");
    require(near(keyed(positioned, "corner").bounds.x, 292) &&
                near(keyed(positioned, "corner").bounds.y, 168),
            "Right and bottom anchors place explicitly sized objects");
    require(near(keyed(positioned, "negative").bounds.x, -5) &&
                near(keyed(positioned, "negative").bounds.y, -3) &&
                near(keyed(positioned, "negative").clip.x, 0),
            "Signed offsets are supported while parent clipping remains effective");
    require(near(keyed(positioned, "conflict").bounds.x, 40) &&
                near(keyed(positioned, "conflict").bounds.width, 70),
            "Left wins over right when an explicit width prevents stretching");
    require(near(keyed(positioned, "reflow").bounds.width, 160) &&
                keyed(positioned, "reflow").bounds.height > 36,
            "Anchored automatic text height reflows at its final stretched width");
    s::Host constrained(
        [] {
            return s::Layer{s::Panel(s::Spacer{})
                                .position(s::Position::absolute)
                                .left(0)
                                .right(0)
                                .top(0)
                                .bottom(0)
                                .min_width(120)
                                .max_width(140)
                                .max_height(80)
                                .key("clamped")};
        },
        {}, theme());
    constrained.layout({200, 150});
    require(near(inspected(constrained, "clamped").bounds.width, 140) &&
                near(inspected(constrained, "clamped").bounds.height, 80),
            "Anchored stretching respects min and max dimensions");
    s::Host flex(
        [] {
            return s::FlexRow{s::Button("A", [] {}).width(80).height(40).shrink(0).key("a"),
                              s::Button("Out of flow", [] {})
                                  .position(s::Position::absolute)
                                  .width(200)
                                  .height(120)
                                  .grow(99),
                              s::Button("B", [] {}).width(80).height(40).shrink(0).key("b")}
                .padding(10)
                .gap(10)
                .wrap()
                .key("row");
        },
        {}, theme());
    flex.layout({200, 140});
    auto row = inspected(flex, "row");
    require(row.flex_lines.size() == 1 && row.flex_lines.front().items.size() == 2 &&
                near(keyed(flex, "a").bounds.x, 10) && near(keyed(flex, "b").bounds.x, 100),
            "Absolute children do not participate in flex sizing, wrapping or line diagnostics");
    s::Host relative(
        [] {
            return s::Block{s::Button("Relative", [] {})
                                .position(s::Position::relative)
                                .left(s::percent(10))
                                .top(10)
                                .width(80)
                                .height(30)
                                .key("relative"),
                            s::Button("Following", [] {}).width(80).height(30).key("following"),
                            s::Button("Right relative", [] {})
                                .position(s::Position::relative)
                                .right(5)
                                .bottom(-3)
                                .width(80)
                                .height(30)
                                .key("right"),
                            s::Button("Normal", [] {}).left(40).top(40).width(80).height(30).key("normal")}
                .gap(8)
                .padding(20);
        },
        {}, theme());
    relative.layout({400, 260});
    require(near(keyed(relative, "relative").bounds.x, 56) &&
                near(keyed(relative, "relative").bounds.y, 30) &&
                near(keyed(relative, "following").bounds.y, 58),
            "Relative offsets preserve original flow space for following siblings");
    require(near(keyed(relative, "right").bounds.x, 15) && near(keyed(relative, "right").bounds.y, 99) &&
                near(keyed(relative, "normal").bounds.x, 20) && near(keyed(relative, "normal").bounds.y, 134),
            "Relative right/bottom offsets are signed and normal flow ignores anchors");
    s::Host nested(
        [] {
            return s::Block{s::Layer{s::Button("Nested", [] {})
                                         .position(s::Position::absolute)
                                         .right(5)
                                         .bottom(5)
                                         .width(30)
                                         .height(20)
                                         .key("child")}
                                .width(200)
                                .height(150)
                                .padding(10)
                                .position(s::Position::relative)
                                .left(20)
                                .top(10)
                                .key("parent")}
                .padding(20);
        },
        {}, theme());
    nested.layout({400, 300});
    const auto parent = inspected(nested, "parent").content_bounds;
    const auto child = keyed(nested, "child").bounds;
    require(near(child.x, parent.x + parent.width - 35) && near(child.y, parent.y + parent.height - 25),
            "Nested absolute anchors follow a relatively positioned parent's actual content box");
    auto inline_builder = [](bool advanced) {
        std::vector<s::View> children{
            s::Label("A styled paragraph with enough text to wrap naturally. ").key("text"),
            s::Label("A second span follows it.").key("tail")};
        if (advanced)
            children.insert(
                children.begin() + 1,
                s::Label("Overlay").position(s::Position::absolute).left(4).top(4).width(300).height(80));
        return s::Block{s::Inline(std::move(children)).key("inline")}.padding(12);
    };
    s::Host plain_inline([&] { return inline_builder(false); }, {}, theme());
    s::Host absolute_inline([&] { return inline_builder(true); }, {}, theme());
    plain_inline.layout({240, 200});
    absolute_inline.layout({240, 200});
    require(near(inspected(plain_inline, "inline").bounds.height,
                 inspected(absolute_inline, "inline").bounds.height) &&
                near(keyed(plain_inline, "tail").bounds.y, keyed(absolute_inline, "tail").bounds.y),
            "Absolute inline overlays leave paragraph wrapping and subsequent span placement unchanged");
    s::Host scrolling(
        [] {
            return s::ScrollView(
                s::Block{s::Spacer{}.height(180), s::Layer{s::Button("Scroll target", [] {})
                                                               .position(s::Position::absolute)
                                                               .left(12)
                                                               .top(10)
                                                               .width(140)
                                                               .height(40)
                                                               .key("target")}
                                                      .height(80)});
        },
        {}, theme());
    scrolling.layout({240, 120});
    const auto target = keyed(scrolling, "target");
    scrolling.focus(target.id);
    require(keyed(scrolling, "target").bounds.y >= 0 && keyed(scrolling, "target").bounds.y + 40 <= 120,
            "Focus reveal scrolls positioned descendants into view");
    expect_throw([] { s::Button("Bad", [] {}).left(std::numeric_limits<double>::quiet_NaN()); },
                 "Nonfinite positioning offsets are rejected");
}
void layer_stacking_and_visibility() {
    s::State<int> first_z{10};
    s::State<bool> hide_first{false}, hide_second{false};
    int first_clicks = 0, second_clicks = 0;
    auto source = std::make_shared<s::Host>(
        [&] {
            return s::Layer{s::Button("First", [&] { ++first_clicks; })
                                .width(140)
                                .height(80)
                                .background(0xffff0000)
                                .corner_radius(0)
                                .z_index(first_z.get())
                                .hidden(hide_first.get())
                                .key("first"),
                            s::Button("Second", [&] { ++second_clicks; })
                                .width(140)
                                .height(80)
                                .background(0xff0000ff)
                                .corner_radius(0)
                                .z_index(0)
                                .hidden(hide_second.get())
                                .key("second")}
                .key("stage");
        },
        std::function<void()>{}, theme());
    source->layout({240, 160});
    auto color = [&] {
        auto surface = SkSurfaces::Raster(SkImageInfo::MakeN32Premul(240, 160));
        source->paint(*surface->getCanvas(), 0);
        source->paint(*surface->getCanvas(), 1);
        SkBitmap image;
        image.allocN32Pixels(240, 160);
        if (!surface->readPixels(image, 0, 0))
            throw std::runtime_error("Cannot read layer pixels");
        return image.getColor(20, 20);
    };
    const auto first = inspected(*source, "first"), second = inspected(*source, "second");
    require(color() == 0xffff0000, "Higher z-index paints above later-declared siblings");
    auto elements = source->elements();
    require(elements.front().id == first.id &&
                keyed(*source, "first").paint_order > keyed(*source, "second").paint_order,
            "Accessibility paint order follows stacking while semantic reading order remains stable");
    click(*source, {20, 20});
    require(first_clicks == 1 && second_clicks == 0,
            "Pointer hit testing targets the highest overlapping layer");
    source->set_inspection_picking(true);
    click(*source, {20, 20});
    require(source->inspected_element() == first.id && first_clicks == 1,
            "Inspector picking follows the same z-index order without activating controls");
    source->set_inspected_element({});
    first_z.set(0);
    require(color() == 0xff0000ff,
            "Equal z-index values preserve declaration order with later siblings on top");
    click(*source, {20, 20});
    require(second_clicks == 1, "Equal-z pointer targeting agrees with painting");
    first_z.set(-10);
    require(color() == 0xff0000ff, "Negative z-index values are supported");
    first_z.set(20);
    source->focus(first.id);
    hide_first.set(true);
    require(color() == 0xff0000ff && inspected(*source, "first").hidden &&
                near(inspected(*source, "first").bounds.width, first.bounds.width),
            "Hidden layers keep geometry and inspection identity while revealing lower layers");
    elements = source->elements();
    require(std::none_of(elements.begin(), elements.end(),
                         [&](const auto& e) { return e.id == first.id || e.focused; }),
            "Hidden controls leave accessibility and lose keyboard focus");
    require(!source->activate(first.id) && !source->focus(first.id),
            "Hidden controls reject activation and focus");
    click(*source, {20, 20});
    require(first_clicks == 1 && second_clicks == 2, "Hidden layers do not intercept input");
    source->set_inspection_picking(true);
    click(*source, {20, 20});
    require(source->inspected_element() == second.id, "Picking skips hidden layers");
    source->set_inspected_element({});
    hide_first.set(false);
    require(color() == 0xffff0000 && inspected(*source, "first").id == first.id,
            "Showing a retained layer restores its original stacking and identity");
    s::State<bool> hide_parent{false};
    int front_clicks = 0, rear_clicks = 0;
    s::State<double> hidden_level{.5};
    s::Host nested(
        [&] {
            return s::Layer{s::Layer{s::Button("Nested rear", [&] { ++rear_clicks; })
                                         .width(140)
                                         .height(80)
                                         .background(0xffff0000)
                                         .corner_radius(0)
                                         .z_index(999)
                                         .key("rear")}
                                .z_index(0),
                            s::Layer{s::Button("Nested front", [&] { ++front_clicks; })
                                         .width(140)
                                         .height(80)
                                         .background(0xff0000ff)
                                         .corner_radius(0)
                                         .z_index(-999)
                                         .key("front"),
                                     s::Slider(hidden_level.binding())
                                         .position(s::Position::absolute)
                                         .right(0)
                                         .bottom(0)
                                         .width(80)
                                         .height(24)
                                         .key("hidden-slider"),
                                     s::ProgressBar().position(s::Position::absolute).bottom(0).width(100)}
                                .z_index(1)
                                .hidden(hide_parent.get())
                                .key("front-layer")};
        },
        {}, theme());
    nested.layout({240, 160});
    click(nested, {20, 20});
    require(front_clicks == 1 && rear_clicks == 0,
            "Nested z-index values cannot escape their sibling subtree's stacking order");
    require(nested.needs_frame(), "Visible indeterminate content in a layer can animate");
    hide_parent.set(true);
    nested.layout({240, 160});
    const auto hidden_child = inspected(nested, "front");
    require(hidden_child.hidden && !hidden_child.own_hidden && !nested.needs_frame(),
            "Hiding a layer propagates to descendants and suspends hidden animation");
    click(nested, {20, 20});
    require(rear_clicks == 1 && !nested.set_value(inspected(nested, "hidden-slider").id, 1) &&
                near(hidden_level.get(), .5),
            "Hidden subtrees do not receive pointer or accessibility value actions");
    nested.blur();
    press(nested, s::Key::tab);
    require(focused(nested).label == "Nested rear", "Keyboard traversal skips hidden subtrees");
    s::State<bool> hidden_flow{true};
    s::Host flow(
        [&] {
            return s::Block{
                s::Button("Hidden in flow", [] {}).height(40).hidden(hidden_flow.get()).key("hidden"),
                s::Button("After hidden", [] {}).height(30).key("after")}
                .gap(10);
        },
        {}, theme());
    flow.layout({240, 150});
    require(near(keyed(flow, "after").bounds.y, 50),
            "Hidden normal-flow elements continue reserving layout space");
    hidden_flow.set(false);
    require(near(keyed(flow, "after").bounds.y, 50), "Showing a hidden element does not reflow siblings");
    // Exercise edits through the actual inspector, including unhide while selected.
    auto model = std::make_shared<s::Inspector>(source);
    auto explorer =
        std::make_shared<s::Host>([model] { return model->body(); }, std::function<void()>{}, theme());
    std::weak_ptr<s::Host> weak = explorer;
    source->set_inspection_observer([weak] {
        if (auto host = weak.lock())
            host->invalidate();
    });
    source->set_inspected_element(first.id);
    explorer->layout({1100, 960});
    explorer->activate(named(*explorer, "Hide element").id);
    require(inspected(*source, "first").own_hidden && source->inspected_element() == first.id,
            "Inspector can hide an element while keeping it selected for further editing");
    explorer->activate(named(*explorer, "Hide element").id);
    explorer->activate(named(*explorer, "Decrease Z-index").id);
    require(!inspected(*source, "first").hidden && inspected(*source, "first").z_index == 19,
            "Inspector can unhide elements and edit their stacking order");
    explorer->activate(named(*explorer, "Position mode").id);
    explorer->activate(named(*explorer, "Absolute").id);
    explorer->activate(named(*explorer, "Left unit").id);
    explorer->activate(named(*explorer, "lp").id);
    explorer->activate(named(*explorer, "Decrease Left").id);
    require(inspected(*source, "first").position == s::Position::absolute &&
                near(inspected(*source, "first").left.value, -1),
            "Inspector position controls accept signed offsets");
    explorer->activate(named(*explorer, "Reset selected layout").id);
    require(inspected(*source, "first").position == s::Position::normal &&
                inspected(*source, "first").z_index == 20,
            "Inspector reset restores application positioning and stacking values");
    source->set_inspection_observer({});
}
void inline_stacking_and_hidden_popups() {
    s::State<int> cover_z{-1};
    s::State<bool> hide_text{false};
    s::State<double> text_offset{0};
    s::Host host(
        [&] {
            return s::Inline{s::Label("AAAAA")
                                 .font_size(24)
                                 .key("first")
                                 .hidden(hide_text.get())
                                 .position(s::Position::relative)
                                 .left(text_offset.get())
                                 .top(text_offset.get()),
                             s::Label("BBB").font_size(24).key("second"),
                             s::Panel(s::Spacer{})
                                 .position(s::Position::absolute)
                                 .left(0)
                                 .top(0)
                                 .width(200)
                                 .height(80)
                                 .background(0xffff0000)
                                 .corner_radius(0)
                                 .z_index(cover_z.get())};
        },
        {}, theme());
    host.layout({200, 80});
    auto whites = [&] {
        auto surface = SkSurfaces::Raster(SkImageInfo::MakeN32Premul(200, 80));
        host.paint(*surface->getCanvas());
        SkBitmap image;
        image.allocN32Pixels(200, 80);
        if (!surface->readPixels(image, 0, 0))
            throw std::runtime_error("Cannot read stacked inline spans");
        int count = 0;
        for (int y = 0; y < 80; ++y)
            for (int x = 0; x < 200; ++x) {
                const auto color = image.getColor(x, y);
                if (((color >> 16) & 255) > 200 && ((color >> 8) & 255) > 200 && (color & 255) > 200)
                    ++count;
            }
        return count;
    };
    const auto full = whites();
    require(full > 50, "Inline text paints above a negative-z absolute overlay");
    const auto first = inspected(host, "first"), second = inspected(host, "second");
    text_offset.set(6);
    require(near(inspected(host, "first").bounds.x, first.bounds.x + 6) &&
                near(inspected(host, "first").bounds.y, first.bounds.y + 6) &&
                near(inspected(host, "second").bounds.x, second.bounds.x),
            "Relative paragraph spans move independently without changing subsequent text flow");
    hide_text.set(true);
    const auto partial = whites();
    require(partial > 0 && partial < full && near(inspected(host, "second").bounds.x, second.bounds.x),
            "Hidden inline spans retain their shaping space without painting glyphs");
    cover_z.set(1);
    require(whites() == 0, "A higher-z inline overlay paints above separately composed text spans");
    s::State<bool> hidden{false};
    s::Host popup(
        [&] {
            return s::Layer{s::MenuButton("Menu", {{"go", "Go", [] {}}})
                                .width(140)
                                .height(40)
                                .hidden(hidden.get())
                                .key("menu")};
        },
        {}, theme());
    popup.layout({300, 200});
    const auto menu = keyed(popup, "menu");
    popup.activate(menu.id);
    require(popup.popup_open(), "A visible layer control can open a popup");
    hidden.set(true);
    popup.inspect();
    require(!popup.popup_open() && popup.elements().empty(),
            "Hiding a popup owner closes its popup and removes its accessibility subtree");
}
void stacked_scroll_targeting() {
    s::State<bool> hide_front{false};
    auto rows = [](const std::string& prefix) {
        std::vector<s::View> result;
        for (int i = 0; i < 8; ++i)
            result.push_back(
                s::Button(prefix + std::to_string(i), [] {}).height(40).key(prefix + std::to_string(i)));
        return s::Block(std::move(result)).gap(4);
    };
    s::Host host(
        [&] {
            return s::Layer{
                s::ScrollView(rows("front")).z_index(10).hidden(hide_front.get()).key("front-scroll"),
                s::ScrollView(rows("rear")).z_index(0).key("rear-scroll")};
        },
        {}, theme());
    host.layout({240, 140});
    host.handle(s::PointerEvent{s::PointerKind::scroll, {20, 20}, 0, 0, -50});
    require(near(inspected(host, "front0").bounds.y, -50) && near(inspected(host, "rear0").bounds.y, 0),
            "Wheel events target the highest overlapping scroll layer");
    hide_front.set(true);
    host.handle(s::PointerEvent{s::PointerKind::scroll, {20, 20}, 0, 0, -50});
    require(near(inspected(host, "front0").bounds.y, -50) && near(inspected(host, "rear0").bounds.y, -50),
            "Hidden scroll layers retain their offset and do not consume wheel events");
}
void layout_debugging() {
    s::State<bool> checked{false}, visible{true};
    s::State<double> application_gap{12};
    int commands = 0;
    auto source = std::make_shared<s::Host>(
        [&] {
            return s::Block{
                s::FlexRow{
                    s::Button("First", [&] { ++commands; }).key("first").width(80).height(40).shrink(0),
                    s::Button("Second", [] {}).key("second").width(80).height(40).shrink(0)}
                    .key("flex")
                    .height(140)
                    .padding(10)
                    .gap(application_gap.get())
                    .align_items(s::Align::start)
                    .visible(visible.get()),
                s::CheckBox("Application state", checked.binding()).key("state")}
                .padding(20)
                .gap(8);
        },
        std::function<void()>{}, theme());
    source->layout({400, 400});
    const auto original = inspected(*source, "flex");
    const auto first = inspected(*source, "first");
    const auto second = inspected(*source, "second");
    require(near(original.content_bounds.x, 30) && near(original.content_bounds.width, 340) &&
                original.layout_editable && !original.layout_overridden,
            "Inspection exposes the resolved content box and layout edit capability");
    require(original.flex_lines.size() == 1 && original.flex_lines[0].items.size() == 2 &&
                near(original.flex_lines[0].gap, 12) && near(original.flex_lines[0].free_space, 168),
            "Flex diagnostics report actual line membership, resolved gap and remaining space");
    source->focus(first.id);
    s::LayoutOverride patch;
    patch.gap = 28;
    patch.padding = s::Insets{20, 14, 8, 18};
    source->set_layout_override(original.id, patch);
    auto edited = inspected(*source, "flex");
    require(edited.layout_overridden && near(edited.gap, 28) && near(edited.content_bounds.x, 38) &&
                near(inspected(*source, "second").bounds.x - inspected(*source, "first").bounds.x, 108),
            "Temporary padding and gap edits update the actual source layout");
    require(focused(*source).id == first.id && !checked.get() && commands == 0,
            "Layout edits preserve application state and keyboard focus without activating controls");
    application_gap.set(7);
    require(near(inspected(*source, "flex").gap, 28),
            "Inspector overrides survive reactive application rebuilds");
    auto invalid = patch;
    invalid.gap = -1;
    expect_throw([&] { source->set_layout_override(original.id, invalid); },
                 "Negative layout edits are rejected");
    invalid = patch;
    invalid.width = std::numeric_limits<double>::quiet_NaN();
    expect_throw([&] { source->set_layout_override(original.id, invalid); },
                 "Nonfinite layout edits are rejected");
    require(near(inspected(*source, "flex").gap, 28), "Invalid edits leave the previous override intact");
    patch.direction = s::Orientation::vertical;
    source->set_layout_override(original.id, patch);
    require(inspected(*source, "flex").type == "FlexColumn" &&
                near(inspected(*source, "second").bounds.y - inspected(*source, "first").bounds.y, 68),
            "Changing flex direction changes real item placement");
    source->clear_layout_override(original.id);
    require(near(inspected(*source, "flex").gap, 7) && inspected(*source, "flex").type == "FlexRow" &&
                !source->layout_override(original.id),
            "Reset restores the latest application values rather than a stale initial snapshot");
    patch = {};
    patch.justify = s::Justify::space_between;
    source->set_layout_override(original.id, patch);
    auto line = inspected(*source, "flex").flex_lines.front();
    require(near(line.gap, 180) && near(inspected(*source, "second").bounds.x, 290),
            "Flex line diagnostics include spacing added by justification");
    patch = {};
    patch.wrap = true;
    source->set_layout_override(original.id, patch);
    source->layout({220, 400});
    edited = inspected(*source, "flex");
    require(edited.flex_lines.size() == 2 && edited.flex_lines[0].items == std::vector{first.id} &&
                edited.flex_lines[1].items == std::vector{second.id},
            "Wrapped rows expose distinct resolved flex lines");
    source->layout({400, 400});
    source->clear_layout_overrides();
    patch = {};
    patch.basis = 50;
    patch.grow = 1;
    patch.self = s::Align::end;
    source->set_layout_override(first.id, patch);
    require(near(inspected(*source, "first").bounds.width, 253) &&
                inspected(*source, "first").self == s::Align::end &&
                near(inspected(*source, "first").bounds.y, 110),
            "Flex item basis, grow and align-self edits feed the layout algorithm");
    source->clear_layout_overrides();
    patch = {};
    patch.width = s::percent(50);
    patch.height = 60;
    patch.margin = s::Insets::all(4);
    source->set_layout_override(first.id, patch);
    require(inspected(*source, "first").width.unit == s::Length::Unit::percent &&
                near(inspected(*source, "first").bounds.height, 60) &&
                near(inspected(*source, "first").margin.left, 4),
            "Dimension units and per-side margins are retained in snapshots");
    s::LayoutOverride bad_direction;
    bad_direction.direction = s::Orientation::vertical;
    expect_throw([&] { source->set_layout_override(first.id, bad_direction); },
                 "Flex direction edits reject non-flex views");
    expect_throw([&] { source->set_layout_override("missing", {}); }, "Unknown layout targets are rejected");
    source->set_layout_override(first.id, {});
    require(!source->layout_override(first.id), "An empty replacement removes the temporary patch");
    source->clear_layout_overrides();
    source->set_inspected_element(original.id);
    auto pixels = [&] {
        auto surface = SkSurfaces::Raster(SkImageInfo::MakeN32Premul(400, 400));
        source->paint(*surface->getCanvas(), 0);
        source->paint(*surface->getCanvas(), 1); // Capture the settled debug overlay.
        SkBitmap bitmap;
        bitmap.allocN32Pixels(400, 400);
        if (!surface->readPixels(bitmap, 0, 0))
            throw std::runtime_error("Cannot read layout overlay");
        return bitmap;
    };
    const auto selected = pixels();
    source->set_inspection_overlay(s::InspectionOverlay::box_model);
    const auto boxes = pixels();
    require(boxes.getColor(25, 65) != selected.getColor(25, 65),
            "Box-model overlay paints real padding geometry in the source");
    source->set_inspection_overlay(s::InspectionOverlay::flex);
    const auto flex = pixels();
    require(flex.getColor(112, 50) != boxes.getColor(112, 50),
            "Flex overlay visualizes actual free space between items");
    source->set_inspected_element(first.id);
    const auto via_child = pixels();
    require(via_child.getColor(112, 50) == flex.getColor(112, 50),
            "Flex debug follows the selected item's closest flex container");
    require(!source->needs_frame(), "Static layout overlays settle without continuous animation");
    source->set_inspection_overlay(s::InspectionOverlay::selection);
    // Drive the real inspector controls, including independent edits and resets.
    auto model = std::make_shared<s::Inspector>(source);
    auto explorer =
        std::make_shared<s::Host>([model] { return model->body(); }, std::function<void()>{}, theme());
    std::weak_ptr<s::Host> weak = explorer;
    source->set_inspection_observer([weak] {
        if (auto host = weak.lock())
            host->invalidate();
    });
    explorer->layout({1100, 900});
    explorer->activate(named(*explorer, "Increase Padding top").id);
    explorer->activate(named(*explorer, "Increase Margin right").id);
    auto item = inspected(*source, "first");
    require(near(item.padding.top, 1) && near(item.margin.right, 1),
            "Inspector spacing controls compose independent temporary edits");
    explorer->activate(named(*explorer, "Show box model overlay").id);
    require(source->inspection_overlay() == s::InspectionOverlay::box_model,
            "Inspector box-model toggle updates the source overlay");
    explorer->activate(named(*explorer, "Reset selected layout").id);
    require(!inspected(*source, "first").layout_overridden &&
                near(inspected(*source, "first").padding.top, 0),
            "Inspector Reset restores the selected view");
    explorer->activate(named(*explorer, "Flex").id);
    explorer->set_value(named(*explorer, "Edit Grow").id, 2);
    require(near(inspected(*source, "first").grow, 2), "Inspector grow slider changes actual flex sizing");
    explorer->activate(named(*explorer, "Show flex debug overlay").id);
    require(source->inspection_overlay() == s::InspectionOverlay::flex,
            "Inspector flex toggle updates the source overlay");
    explorer->activate(named(*explorer, "Layout").id);
    explorer->activate(named(*explorer, "Width unit").id);
    explorer->activate(named(*explorer, "%").id);
    explorer->set_value(named(*explorer, "Edit Width").id, 25);
    require(inspected(*source, "first").width.unit == s::Length::Unit::percent &&
                near(inspected(*source, "first").width.value, .25),
            "Inspector dimension unit selection and slider edit percentage lengths");
    explorer->activate(named(*explorer, "Flex").id);
    source->set_inspected_element(original.id);
    explorer->activate(named(*explorer, "Flex direction").id);
    explorer->activate(named(*explorer, "Column ↓").id);
    require(inspected(*source, "flex").type == "FlexColumn",
            "Inspector direction dropdown switches the source flex mode");
    explorer->activate(named(*explorer, "Reset selected layout").id);
    explorer->activate(named(*explorer, "Justify content").id);
    explorer->activate(named(*explorer, "Space between").id);
    require(inspected(*source, "flex").justify == s::Justify::space_between,
            "Inspector justification dropdown updates the source container");
    explorer->activate(named(*explorer, "Increase Gap").id);
    require(near(inspected(*source, "flex").gap, 8), "Inspector flex gap stepper is connected");
    explorer->activate(named(*explorer, "Reset all layout overrides").id);
    require(!inspected(*source, "first").layout_overridden && !inspected(*source, "flex").layout_overridden,
            "Inspector Reset all clears edits across multiple views");
    patch = {};
    patch.gap = 42;
    source->set_layout_override(original.id, patch);
    visible.set(false);
    source->inspect();
    require(!source->layout_override(original.id), "Unmounting prunes obsolete layout overrides");
    visible.set(true);
    require(near(inspected(*source, "flex").gap, 7), "Remounted views start with application layout values");
    source->set_inspection_observer({});
}
void inspection_selection_flash() {
    s::State<bool> show{true};
    int commands = 0;
    auto source = std::make_shared<s::Host>(
        [&] {
            return s::Block{
                s::FlexRow{s::Button("First", [&] { ++commands; }).key("first").width(120).height(48),
                           s::Button("Second", [] {}).key("second").width(120).height(48)}
                    .key("flex")
                    .padding(12)
                    .gap(16)
                    .visible(show.get())}
                .padding(24);
        },
        std::function<void()>{}, theme());
    source->layout({400, 200});
    const auto first = inspected(*source, "first");
    const auto second = inspected(*source, "second");
    auto pixels = [&](double time) {
        auto surface = SkSurfaces::Raster(SkImageInfo::MakeN32Premul(400, 200));
        source->paint(*surface->getCanvas(), time);
        SkBitmap bitmap;
        bitmap.allocN32Pixels(400, 200);
        if (!surface->readPixels(bitmap, 0, 0))
            throw std::runtime_error("Cannot read selection flash");
        return bitmap;
    };
    auto sample = [&](const SkBitmap& image, const s::InspectionNode& node) {
        return image.getColor(node.bounds.x + 20, node.bounds.y + 10);
    };
    source->set_inspected_element(first.id);
    require(source->needs_frame(), "Inspector selection requests frames for a brief flash");
    const auto peak = pixels(10);
    const auto middle = pixels(10.24);
    const auto settled = pixels(10.5);
    require(sample(peak, first) != sample(middle, first) && sample(middle, first) != sample(settled, first),
            "Selection highlight eases from a subtle flash to the persistent selection");
    require(sample(peak, second) == sample(settled, second),
            "Selection flash leaves other controls unchanged");
    require(!source->needs_frame(), "Selection flash stops requesting frames after it finishes");
    source->set_inspected_element(first.id);
    const auto repeated = pixels(100);
    require(source->needs_frame() && sample(repeated, first) == sample(peak, first),
            "Reselecting the same element restarts its flash after a long idle interval");
    source->set_inspected_element({});
    require(!source->needs_frame() && source->inspected_element().empty(),
            "Clearing selection immediately cancels the flash");
    source->set_inspection_picking(true);
    source->handle(s::PointerEvent{s::PointerKind::move, {first.bounds.x + 20, first.bounds.y + 10}});
    require(!source->needs_frame(), "Picking hover does not start a flash");
    click(*source, {first.bounds.x + 20, first.bounds.y + 10});
    require(source->needs_frame() && source->inspected_element() == first.id && commands == 0,
            "Picking selection starts the flash without activating the application control");
    pixels(200);
    pixels(201);
    source->set_inspection_overlay(s::InspectionOverlay::flex);
    const auto flex_base = pixels(202);
    source->set_inspected_element(first.id);
    const auto flex_flash = pixels(203);
    require(sample(flex_base, first) != sample(flex_flash, first) &&
                sample(flex_base, second) == sample(flex_flash, second),
            "Flex debug flashes the selected child rather than its whole ancestor");
    show.set(false);
    pixels(203.1);
    require(source->inspected_element().empty() && !source->needs_frame(),
            "Unmounting the selected element cancels its pending flash");
}
void text_engine_contract() {
    struct FixedEngine final : s::TextLayoutEngine {
        int calls{};
        std::shared_ptr<const s::TextLayout> layout(const std::vector<s::TextSpan>&,
                                                    s::TextLayoutOptions) override {
            ++calls;
            auto result = std::make_shared<s::TextLayout>();
            result->size = {123, 21};
            result->baseline = 15;
            result->paint = [](SkCanvas&, s::Point) {};
            return result;
        }
    };
    auto fixed = std::make_shared<FixedEngine>();
    s::Theme t;
    t.font_path = "/missing/unused/font.ttf";
    s::Host host(
        [] { return s::Block{s::Label("External layout").key("external")}.align_items(s::Align::start); }, {},
        t, fixed);
    host.layout({400, 100});
    require(fixed->calls > 0 && near(keyed(host, "external").bounds.width, 123) &&
                near(keyed(host, "external").bounds.height, 21),
            "Host accepts a substitute text engine without initializing the default font stack");
    auto engine = s::make_skparagraph_text_engine(SCARS_FONT_PATH);
    s::TextSpan latin;
    latin.text = "Office café a\xcc\x81";
    auto shaped = engine->layout({latin}, {300, s::TextWrap::wrap});
    require(!shaped->glyphs.empty() && shaped->baseline > 0,
            "SkParagraph returns positioned glyphs and baseline metrics");
    s::TextSpan rtl;
    rtl.text = "English العربية עברית 日本語 🚀";
    auto unicode = engine->layout({rtl}, {140, s::TextWrap::wrap});
    require(unicode->size.height > 18 && std::all_of(unicode->glyphs.begin(), unicode->glyphs.end(),
                                                     [](const auto& g) {
                                                         return std::isfinite(g.position.x) &&
                                                                std::isfinite(g.position.y);
                                                     }),
            "Mixed Unicode scripts shape and wrap with finite geometry");
    s::TextSpan box;
    box.box = s::TextInlineBox{{60, 32}, 24};
    s::TextSpan tail;
    tail.text = " after";
    auto inline_layout = engine->layout({latin, box, tail}, {180, s::TextWrap::wrap});
    auto placement = std::find_if(inline_layout->placements.begin(), inline_layout->placements.end(),
                                  [](auto& p) { return p.inline_box; });
    require(placement != inline_layout->placements.end() && placement->span == 1 &&
                near(placement->bounds.width, 60) && near(placement->bounds.height, 32),
            "SkParagraph placeholders retain inline-view dimensions and association");
    auto surface = SkSurfaces::Raster(SkImageInfo::MakeN32Premul(300, 200));
    unicode->paint(*surface->getCanvas(), {0, 0});
    inline_layout->paint(*surface->getCanvas(), {0, 80});
    s::TextSpan long_text;
    long_text.text = "A long annotation that must remain one line and end in an ellipsis.";
    auto ellipsis = engine->layout({long_text}, {90, s::TextWrap::no_wrap, s::TextOverflow::ellipsis, 1});
    require(ellipsis->size.width <= 90 && near(ellipsis->size.height, 18),
            "Single-line ellipsis preserves assigned geometry");
    auto nowrap = engine->layout({long_text}, {90, s::TextWrap::no_wrap});
    require(near(nowrap->size.height, 18), "NoWrap clips without creating additional lines");
    require(engine->layout({latin}, {300, s::TextWrap::wrap}) == shaped,
            "Shaped paragraph results are reused");
}
void sliders_identity_and_scroll() {
    s::State<double> value{.35};
    s::State<bool> reverse{false};
    s::State<bool> show{true};
    s::Host host(
        [&] {
            std::vector<s::View> children{s::Slider(value.binding(), {0, 1, .1, .3}).key("slider"),
                                          s::Button("Second", [] {}).key("second")};
            if (reverse.get())
                std::reverse(children.begin(), children.end());
            return s::FlexColumn(std::move(children));
        },
        {}, theme());
    host.layout({400, 200});
    auto slider = keyed(host, "slider");
    host.focus(slider.id);
    press(host, s::Key::end);
    require(near(value.get(), 1), "Slider End reaches inclusive maximum");
    press(host, s::Key::home);
    press(host, s::Key::page_up);
    require(near(value.get(), .3), "Slider supports range clamping and page steps");
    host.set_value(slider.id, .76);
    require(near(value.get(), .8), "Slider quantizes relative to minimum");
    require(!host.set_value(slider.id, std::numeric_limits<double>::quiet_NaN()), "Slider rejects NaN");
    value.set(5);
    host.layout({400, 200});
    require(near(keyed(host, "slider").value, 1),
            "Externally supplied slider values are clamped for rendering and accessibility");
    reverse.set(true);
    host.layout({400, 200});
    require(focused(host).id == slider.id, "Keyed focus survives sibling reorder");
    std::vector<s::View> rows;
    for (int i = 0; i < 30; ++i)
        rows.push_back(s::Button("Row " + std::to_string(i), [] {}).key(std::to_string(i)));
    s::Host scrolling([&] { return s::ScrollView(s::Block(rows).gap(6)); }, {}, theme());
    scrolling.layout({220, 160});
    auto last = named(scrolling, "Row 29");
    require(last.bounds.y > 160, "Scroll content retains intrinsic height");
    scrolling.focus(last.id);
    last = named(scrolling, "Row 29");
    require(last.bounds.y >= 0 && last.bounds.y + last.bounds.height <= 160,
            "Keyboard focus reveals scroll content");
    s::Host progress([] { return s::ProgressBar(); });
    progress.layout({200, 40});
    require(progress.needs_frame(), "Indeterminate progress schedules frames");
    expect_throw([] { s::Slider(s::State<double>{0}.binding(), {0, 0, .1, .2}); },
                 "Invalid slider range rejected");
    expect_throw([] { s::Label("Bad").width(-1); }, "Negative lengths rejected");
}
} // namespace
int main() {
    try {
        reactive();
        layout();
        inline_flow();
        choices();
        tab_focus_mode();
        menus();
        element_inspection();
        layers_and_positioning();
        layer_stacking_and_visibility();
        inline_stacking_and_hidden_popups();
        stacked_scroll_targeting();
        layout_debugging();
        inspection_selection_flash();
        text_engine_contract();
        sliders_identity_and_scroll();
        std::cout << "SCARS: " << checks << " behavioral checks passed\n";
        return 0;
    } catch (const std::exception& e) {
        std::cerr << "SCARS test failed after " << checks << " checks: " << e.what() << '\n';
        return 1;
    }
}
