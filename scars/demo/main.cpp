#include <skia_window/skia_window.hpp>
#include "include/core/SkBitmap.h"
#include "include/core/SkCanvas.h"
#include "include/core/SkSurface.h"
#include "include/core/SkStream.h"
#include "include/encode/SkPngEncoder.h"
#include <cmath>
#include <charconv>
#include <cctype>
#include <chrono>
#include <algorithm>
#include <cstdint>
#include <filesystem>
#include <mach-o/dyld.h>
#include <cstdlib>
#include <iostream>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>
#include <unordered_map>
import scars;
import scars.macos;

namespace s = scars;
namespace sw = skia_window;
namespace {
std::string bundled_font() {
    std::uint32_t count = 0;
    _NSGetExecutablePath(nullptr, &count);
    std::vector<char> path(count);
    if (_NSGetExecutablePath(path.data(), &count) != 0)
        return {};
    auto resource = std::filesystem::weakly_canonical(path.data()).parent_path().parent_path() / "Resources" /
                    "Roboto-Regular.ttf";
    return std::filesystem::exists(resource) ? resource.string() : std::string{};
}
struct Lab {
    s::State<s::Color> paint{0xa04dd2ff}, live_paint{0xc0ffaa34};
    s::State<bool> color_alpha{true};
    s::View colors_section() {
        return s::Section(
            "Color controls",
            s::FlexColumn{
                s::Label(
                    "Pick an sRGB color using RGB, HSV, or CSS hex. The checkerboard shows transparency. "
                    "Apply commits the shared draft; Cancel restores the starting color.",
                    s::LabelStyle::secondary)
                    .text_wrap(s::TextWrap::wrap),
                s::Switch("Edit alpha", color_alpha.binding()),
                s::FlexRow{
                    s::ColorWell("Paint", paint.binding(), {.alpha = color_alpha.get()}).key("paint-well"),
                    s::ColorWell("Live preview", live_paint.binding(),
                                 {.alpha = color_alpha.get(), .live_preview = true})
                        .key("live-paint-well")}
                    .gap(12)
                    .wrap(),
                s::ColorPicker("Paint editor", paint.binding(), {.alpha = color_alpha.get()})
                    .key("paint-editor"),
                s::Label("Committed paint: " + s::format_color(paint.get()) +
                             " · live: " + s::format_color(live_paint.get()),
                         s::LabelStyle::status)
                    .text_wrap(s::TextWrap::wrap)}
                .gap(14));
    }
    s::State<s::Date> mission_date{s::Date{2026, 11, 2}};
    s::State<s::Time> mission_time{s::Time{14, 30, 15}};
    s::State<s::LocalDateTime> departure{s::LocalDateTime{{2026, 11, 2}, {14, 30, 0}}};
    s::State<s::LocalDateTime> overnight{s::LocalDateTime{{2026, 11, 2}, {23, 0, 0}}};
    s::State<std::chrono::sys_seconds> zoned_departure{
        *s::resolve_datetime({{2026, 11, 2}, {14, 30, 0}}, "UTC").instant};
    s::State<bool> twelve_hour{false}, show_seconds{true};
    s::State<std::string> date_locale{"en_US_POSIX"}, repeated_time{"reject"};
    s::View datetime_section() {
        s::DatePickerOptions dates{.min = {2020, 1, 1},
                                   .max = {2030, 12, 31},
                                   .locale = date_locale.get(),
                                   .disabled_date = [](s::Date date) {
                                       const auto day = std::chrono::sys_days{std::chrono::year_month_day{
                                           std::chrono::year{date.year}, std::chrono::month{date.month},
                                           std::chrono::day{date.day}}};
                                       return std::chrono::weekday{day}.c_encoding() == 0;
                                   }};
        s::TimePickerOptions times{.locale = date_locale.get(),
                                   .hour_cycle = twelve_hour.get() ? s::HourCycle::h12 : s::HourCycle::h24,
                                   .show_seconds = show_seconds.get(),
                                   .minute_step = 5};
        return s::Section(
            "Date and time",
            s::FlexColumn{
                s::Label("Edit a date or open its calendar with Option-Down. Arrow keys adjust the component "
                         "at the caret. "
                         "Calendar arrows move by day/week; Page Up/Down changes months and Shift adds "
                         "years. Sundays are unavailable.",
                         s::LabelStyle::secondary)
                    .text_wrap(s::TextWrap::wrap),
                s::FlexRow{s::SelectionDropdown(
                               date_locale.binding(),
                               {{"en_US_POSIX", "US English"}, {"de_DE", "German"}, {"ja_JP", "Japanese"}})
                               .width(180),
                           s::Switch("12-hour clock", twelve_hour.binding()),
                           s::Switch("Seconds", show_seconds.binding())}
                    .gap(12)
                    .wrap(),
                s::FlexRow{s::DatePicker("Mission date", mission_date.binding(), dates)
                               .grow()
                               .basis(190)
                               .key("mission-date"),
                           s::TimePicker("Mission time", mission_time.binding(), times)
                               .grow()
                               .basis(170)
                               .key("mission-time")}
                    .gap(12)
                    .wrap(),
                s::DateTimePicker("Local departure", departure.binding(), {.date = dates, .time = times})
                    .key("local-departure"),
                s::DateTimePicker("Overnight window", overnight.binding(),
                                  {.date = {.locale = date_locale.get()},
                                   .time = times,
                                   .min = {{2026, 11, 2}, {22, 0, 0}},
                                   .max = {{2026, 11, 3}, {2, 0, 0}}})
                    .key("overnight-window"),
                s::Label("Allowed: November 2 at 22:00 through November 3 at 02:00. Edit both parts, "
                         "then press Enter. Committed: " +
                             s::format_date(overnight.get().date, date_locale.get()) + " " +
                             s::format_time(overnight.get().time, times),
                         s::LabelStyle::secondary)
                    .text_wrap(s::TextWrap::wrap),
                s::Label("Repeated New York wall times require an explicit policy. Nonexistent times are "
                         "rejected.",
                         s::LabelStyle::secondary)
                    .text_wrap(s::TextWrap::wrap),
                s::SelectionDropdown(repeated_time.binding(), {{"reject", "Reject repeated time"},
                                                               {"earlier", "Earlier instant"},
                                                               {"later", "Later instant"}}),
                s::DateTimePicker(
                    "Zoned departure", zoned_departure.binding(),
                    {.time_zone = "America/New_York",
                     .repeated_time = repeated_time.get() == "earlier" ? s::RepeatedTimePolicy::earlier
                                      : repeated_time.get() == "later" ? s::RepeatedTimePolicy::later
                                                                       : s::RepeatedTimePolicy::reject,
                     .display = {.date = dates, .time = times}})
                    .key("zoned-departure"),
                s::Label("Mission: " + s::format_date(mission_date.get(), date_locale.get()) + " · " +
                             s::format_time(mission_time.get(), times),
                         s::LabelStyle::status)
                    .text_wrap(s::TextWrap::wrap)}
                .gap(14));
    }
    std::function<void(bool)> present_alert;
    s::State<std::string> alert_status{"No alert presented."}, alert_note{"Scan 17"};
    s::View alert_content() {
        return s::FlexColumn{
            s::Label("Review the scan name before applying changes. Cancel keeps the current scan.",
                     s::LabelStyle::secondary)
                .text_wrap(s::TextWrap::wrap),
            s::TextField("Scan name", alert_note.binding()).key("scan-name")}
            .gap(12);
    }
    s::View alert_preview() {
        return s::Alert("Apply scan changes?", alert_content(),
                        {{"cancel", "Cancel", s::AlertActionRole::cancel},
                         {"archive", "Archive", s::AlertActionRole::destructive},
                         {"apply", "Apply", s::AlertActionRole::default_action}});
    }
    s::View alerts_section() {
        return s::Section(
            "Alerts and confirmations",
            s::FlexColumn{s::Label("Sheets block their parent window while other windows stay interactive. "
                                   "Return chooses the default action; Escape cancels after closing nested "
                                   "content or composition.",
                                   s::LabelStyle::secondary)
                              .text_wrap(s::TextWrap::wrap),
                          s::FlexRow{s::Button("Review scan",
                                               [this] {
                                                   if (present_alert)
                                                       present_alert(false);
                                               }),
                                     s::Button("Confirm reset",
                                               [this] {
                                                   if (present_alert)
                                                       present_alert(true);
                                               })}
                              .gap(12)
                              .wrap(),
                          s::Label(alert_status.get(), s::LabelStyle::status).text_wrap(s::TextWrap::wrap)}
                .gap(14));
    }
    s::State<bool> grid{true}, telemetry{true}, alerts{false}, expanded{true}, disabled{false};
    s::State<bool> show_overlay{true}, raise_rear_layer{false}, light_mode{false};
    s::State<bool> show_settings{false}, show_overlay_help{false};
    s::State<std::string> combo_query{"Navigation"}, combo_selection{"nav"}, free_query{""}, free_selection{""};
    s::State<bool> combo_loading{false};
    s::State<std::string> selected_sector{"01 / 01"};
    s::State<std::string> browser_selection{"navigation"};
    s::State<std::vector<std::string>> browser_expanded{{"fleet", "kestrel", "telemetry"}};
    s::State<bool> browser_loaded{false};
    s::TreeModel make_browser_tree(bool loaded) {
        s::TreeNode archive{"archive", "Archived observations"};
        archive.lazy_children = [] {
            std::vector<s::TreeNode> nodes;
            for (int i = 1; i <= 2000; ++i) nodes.push_back({"archive/" + std::to_string(i), "Observation " + std::to_string(i)});
            return nodes;
        };
        s::TreeNode telemetry{"telemetry", "Remote telemetry", true, !loaded};
        telemetry.expandable = true;
        if (loaded) telemetry.children = {{"temperature", "Temperature samples"}, {"radiation", "Radiation samples"}};
        return s::TreeModel({
            {"fleet", "Survey fleet", true, false, {
                {"kestrel", "USS Kestrel", true, false, {{"navigation", "Navigation"}, {"science", "Science"}, {"engineering", "Engineering"}}},
                {"relay", "Offline relay", false}}},
            std::move(archive), std::move(telemetry)
        });
    }
    s::State<s::TreeModel> browser_tree{make_browser_tree(false)};

    s::State<std::string> workbench_query{""}, workbench_group{"all"};
    s::State<std::vector<std::string>> workbench_expanded{{"collections"}};
    s::State<s::TableSelection> workbench_selection{s::TableSelection{{}, {{"0", "power"}}}};
    s::State<std::vector<s::TableColumnWidth>> workbench_widths;
    s::State<std::vector<s::SortDescriptor>> workbench_sort;
    s::State<std::unordered_map<std::string,std::string>> workbench_values;
    s::State<double> workbench_fraction{.25};
    s::TreeModel workbench_folders{{{"collections", "Collections", true, false,
        {{"all", "All observations"}, {"active", "Active"}, {"archived", "Archived"}}}}};
    s::TableModel make_workbench_table() {
        std::vector<s::Option> rows;
        const auto query = workbench_query.get(), group = workbench_group.get();
        for (int i=0;i<1000;++i) {
            const auto label="Observation "+std::to_string(i);
            if(group=="active"&&i%2!=0)continue;
            if(group=="archived"&&i%2==0)continue;
            if(!query.empty()&&label.find(query)==std::string::npos)continue;
            rows.push_back({std::to_string(i),label});
        }
        const auto power = [this](const std::string& id) {
            const auto& values=workbench_values.get();const auto found=values.find(id);
            return found==values.end()?std::to_string(std::stoi(id)%101):found->second;
        };
        const auto order=workbench_sort.get();
        std::stable_sort(rows.begin(),rows.end(),[&](const auto& a,const auto& b){
            for(const auto& sort:order){
                int comparison{};
                if(sort.column=="power")comparison=std::stoi(power(a.id))-std::stoi(power(b.id));
                else if(sort.column=="category")comparison=(std::stoi(a.id)%2)-(std::stoi(b.id)%2);
                else comparison=a.label.compare(b.label);
                if(comparison!=0)return sort.order==s::SortOrder::ascending?comparison<0:comparison>0;
            }
            return false;
        });
        return s::TableModel(std::move(rows),{
            {.id="name",.title="Observation",.width=220,.min_width=90,.max_width=480,.sortable=true},
            {.id="category",.title="Collection",.width=140,.sortable=true,
             .text=[](const auto& row,std::size_t){return std::stoi(row.id)%2==0?std::string("Active"):std::string("Archived");}},
            {.id="power",.title="Power (%)",.width=120,.min_width=70,.max_width=220,.sortable=true,.alignment=s::Align::end,
             .text=[power](const auto& row,std::size_t){return power(row.id);},
             .validate=[](const auto&,const std::string& text){int value{};const auto parsed=std::from_chars(text.data(),text.data()+text.size(),value);
                 return parsed.ec==std::errc{}&&parsed.ptr==text.data()+text.size()&&value>=0&&value<=100?std::string{}:std::string("Enter a whole number from 0 to 100.");},
             .commit=[this](const auto& id,std::string text){auto values=workbench_values.get();values[id]=std::move(text);workbench_values.set(std::move(values));
                 if(!workbench_sort.get().empty())workbench_table.set(make_workbench_table());}}
        });
    }
    s::State<s::TableModel> workbench_table{make_workbench_table()};
    s::State<std::string> virtual_selected{"0"};
    s::State<std::unordered_map<std::string, std::string>> virtual_edits;
    s::RowModel make_virtual_rows() {
        std::vector<s::Option> rows;
        rows.reserve(100000);
        for (int i = 0; i < 100000; ++i) rows.push_back({std::to_string(i), "Record " + std::to_string(i + 1)});
        return s::RowModel(std::move(rows), [this](const s::Option& row, std::size_t) {
            const auto id = row.id;
            s::Binding<std::string> memo{
                [this, id] { const auto& values = virtual_edits.get(); const auto i = values.find(id);
                             return i == values.end() ? std::string{} : i->second; },
                [this, id](std::string text) { auto values = virtual_edits.get(); values[id] = std::move(text); virtual_edits.set(std::move(values)); }
            };
            return s::FlexRow{
                s::Label(row.label).font_size(12).width(112),
                s::TextField("Observation", std::move(memo)).key("memo").grow().height(32)
            }.align_items(s::Align::center).gap(10);
        });
    }
    s::RowModel virtual_rows{make_virtual_rows()};
    s::State<std::string> call_sign{"USS Kestrel"},
        notes{"Survey the outer sector.\nSend a status report when the scan is complete."},
        assigned_id{"SCARS-1701"};
    s::State<std::string> station{"nav"};
    s::State<std::vector<std::string>> monitored{{"sci"}};
    s::State<std::vector<s::Option>> stations{{{"nav", "Navigation"},
                                               {"sci", "Science"},
                                               {"eng", "Engineering"},
                                               {"ops", "Operations"},
                                               {"locked", "Offline", false}}};
    int next_station{1};
    s::Theme palette() const {
        return light_mode.get() ? s::Theme::light() : s::Theme{};
    }
    s::State<std::string> wizard_step{"identity"}, wizard_name{""}, wizard_destination{"outer"};
    s::State<bool> wizard_done{false}, wizard_cancelled{false};
    s::Wizard setup_wizard{
        wizard_step.binding(),
        {{"identity", "Identity",
          [this] {
              const auto name = wizard_name.get();
              return name.find_first_not_of(" \t\r\n") == std::string::npos ? "Enter a ship name to continue."
                                                                            : "";
          }},
         {"destination", "Destination",
          [this] { return wizard_destination.get().empty() ? "Choose a destination to continue." : ""; }},
         {"review", "Review", {}}},
        {.on_finish =
             [this] {
                 wizard_done.set(true);
                 command("Setup complete: " + wizard_name.get() + " → " + wizard_destination.get());
             },
         .on_cancel = [this] { wizard_cancelled.set(true); },
         .on_change = [this](const std::string&,
                             const std::string& to) { status.set("Wizard step: " + to); }}};
    s::State<s::CheckState> subsystems{s::CheckState::indeterminate};
    s::State<std::string> mode{"flight"}, quality{"normal"}, export_action{"pdf"}, difficulty{"normal"};
    s::State<double> volume{.7}, progress{.62};
    s::State<double> split_fraction{.4}, vertical_fraction{.5};
    s::State<std::string> selected_tab{"mission"};
    s::State<double> target_power{6.5};
    s::State<int> crew_count{4};
    s::State<bool> automatic_scan{true};
    s::State<s::NumericValidation> numeric_validation;
    s::State<std::string> search_text{""}, last_search{""}, search_selected{"nav"};
    s::State<std::string> access_code{"scars-demo"}, credential_status{"Press Enter to submit the demo access code."};
    s::State<s::SplitPane> collapsed_pane{s::SplitPane::none};
    s::State<int> commands{0};
    s::State<std::string> status{"All systems nominal. Awaiting command."};
    std::vector<s::Option> modes{{"flight", "Flight"}, {"science", "Science"}, {"systems", "Systems"}};
    std::vector<s::Option> qualities{
        {"low", "Low power"}, {"normal", "Normal"}, {"high", "High fidelity"}, {"offline", "Offline", false}};
    std::vector<s::Action> exports() {
        return {{"pdf", "Export PDF", [this] { command("PDF export complete."); }},
                {"svg", "Export SVG", [this] { command("SVG export complete."); }},
                {"png", "Export PNG", [this] { command("PNG export complete."); }},
                {"archive", "Archive", {}, false}};
    }
    void command(std::string message) {
        commands.set(commands.get() + 1);
        status.set(std::move(message));
    }
    s::View badge(std::string value, s::Color color) {
        const auto luma = ((color >> 16) & 255) * .299 + ((color >> 8) & 255) * .587 + (color & 255) * .114;
        return s::Label(std::move(value), s::LabelStyle::micro)
            .foreground(luma >= 145 ? 0xff0f1924 : 0xfff7fbff)
            .background(color)
            .padding(s::Insets::symmetric(6, 12))
            .corner_radius(4)
            .max_lines(1);
    }
    s::View header() {
        return s::FlexColumn{
            s::FlexRow{badge("SCARS / 01", 0xffffaa34), s::Spacer{},
                       s::Label("STRUCTURED CONTROL AND RENDERING SYSTEM", s::LabelStyle::micro)}
                .align_items(s::Align::center)
                .gap(16),
            s::Label("Ship systems", s::LabelStyle::heading).font_size(32),
            s::Inline{
                s::Label("A new C++ engine. "),
                s::Label("Declarative views").foreground(palette().cyan).bold(),
                s::Label(", web-inspired flow, and Skia on Metal. Resize this window to reflow the bridge.")
                    .foreground(palette().secondary)},
            s::Separator{}.foreground(palette().light_mode ? 0xff9a4e00 : palette().primary).height(3)}
            .gap(12);
    }
    s::View command_section() {
        return s::Section(
            "Command deck",
            s::FlexColumn{
                s::FlexRow{
                    s::labeled("COMMAND",
                               s::Button("Engage", [this] { command("Engage command acknowledged."); })
                                   .key("engage")
                                   .min_width(140)),
                    s::labeled("INDEPENDENT TOGGLE", s::ToggleButton("Show grid", grid.binding())
                                                         .key("grid")
                                                         .accent(0xff4dd2ff)
                                                         .min_width(160)),
                    s::labeled("EXCLUSIVE MODE", s::SegmentedControl(mode.binding(), modes).key("mode"))}
                    .gap(20)
                    .wrap()
                    .align_items(s::Align::start),
                s::Inline{s::Label("Active mode: ").foreground(palette().secondary),
                          s::Label(mode.get())
                              .foreground(palette().light_mode ? 0xff9a4e00 : palette().primary)
                              .bold(),
                          s::Label(". Grid overlay is ").foreground(palette().secondary),
                          s::Label(grid.get() ? "enabled." : "disabled.")}}
                .gap(14));
    }
    s::View dropdown_section() {
        return s::Section(
            "Routing & actions",
            s::FlexRow{
                s::labeled("COMMAND MENU",
                           s::MenuButton("Actions",
                                         {{"reset", "Reset telemetry",
                                           [this] {
                                               volume.set(.7);
                                               progress.set(.62);
                                               command("Telemetry reset.");
                                           }},
                                          {"diagnostic", "Run diagnostics",
                                           [this] { command("Diagnostics complete. No faults detected."); }},
                                          {"unavailable", "Unavailable command", {}, false}})
                               .key("actions")
                               .min_width(180)),
                s::labeled("PERSISTENT SELECTION", s::SelectionDropdown(quality.binding(), qualities)
                                                       .key("quality")
                                                       .accent(0xff4dd2ff)
                                                       .min_width(200)),
                s::labeled("REMEMBERED ACTION", s::SplitActionButton(export_action.binding(), exports())
                                                    .key("export")
                                                    .accent(0xff168bf0)
                                                    .min_width(210))}
                .gap(24)
                .wrap()
                .align_items(s::Align::start));
    }
    s::View form() {
        return s::Section(
                   "Environment configuration",
                   s::FlexRow{
                       s::FlexColumn{
                           s::Label("SUBSYSTEMS", s::LabelStyle::micro),
                           s::CheckBox("Telemetry feed", telemetry.binding()).key("telemetry"),
                           s::CheckBox("Priority alerts", alerts.binding()).key("alerts"),
                           s::CheckBox("Partial subsystem selection", subsystems.binding()).key("mixed"),
                           s::CheckBox("Long-range uplink unavailable", alerts.binding()).enabled(false)}
                           .gap(4)
                           .min_width(280)
                           .grow(),
                       s::FlexColumn{s::Label("SIMULATION DIFFICULTY", s::LabelStyle::micro),
                                     s::RadioGroup(difficulty.binding(), {{"normal", "Normal"},
                                                                          {"hard", "Hard"},
                                                                          {"expert", "Expert"},
                                                                          {"locked", "Classified", false}})
                                         .key("difficulty")}
                           .gap(4)
                           .min_width(220)
                           .grow(),
                       s::FlexColumn{
                           s::labeled(
                               "MASTER VOLUME / " +
                                   std::to_string(static_cast<int>(std::round(volume.get() * 100))) + "%",
                               s::Slider(volume.binding(), {0, 1, .01, .1}).key("volume").min_width(200)),
                           s::labeled("SYSTEM SYNCHRONIZATION / " +
                                          std::to_string(static_cast<int>(std::round(progress.get() * 100))) +
                                          "%",
                                      s::ProgressBar(progress.binding()).key("progress")),
                           s::Button("Advance synchronization",
                                     [this] { progress.set(std::fmod(progress.get() + .1, 1.01)); })
                               .density(s::Density::compact)
                               .accent(0xff4dd2ff)}
                           .gap(16)
                           .min_width(250)
                           .grow()}
                       .gap(24)
                       .wrap()
                       .align_items(s::Align::start))
            .expanded(expanded.binding())
            .key("environment");
    }
    s::View typography() {
        return s::GroupBox(
            "Block & inline composition",
            s::Block{
                s::Inline{
                    s::Label("Inline content flows naturally, including an embedded ")
                        .text_wrap(s::TextWrap::wrap),
                    badge("LIVE", 0xff4dd2ff), s::Label(" status badge and an "),
                    s::Button("Acknowledge", [this] { command("Inline command acknowledged."); })
                        .density(s::Density::compact)
                        .accent(0xff4dd2ff)
                        .key("inline-action"),
                    s::Label(
                        " command. Text wraps around the indivisible views as the available width changes.")
                        .foreground(palette().secondary)},
                s::Label("Block children stack vertically and use the available width. FlexRow and "
                         "FlexColumn add flexible sizing and alignment.",
                         s::LabelStyle::secondary)
                    .text_wrap(s::TextWrap::wrap),
                s::Separator{},
                s::FlexRow{
                    s::Button("Compact", [this] { command("Compact command."); })
                        .density(s::Density::compact),
                    s::Button("Standard", [this] { command("Standard command."); }),
                    s::Button("Spacious", [this] { command("Spacious command."); })
                        .density(s::Density::spacious),
                    s::Button("Disabled", [] {}).enabled(false),
                    s::Label("A deliberately long micro annotation with ellipsis", s::LabelStyle::micro)
                        .width(190)}
                    .gap(12)
                    .wrap()
                    .align_items(s::Align::baseline)}
                .gap(16));
    }
    s::View layers_section() {
        return s::Section(
            "Layers & positioning",
            s::FlexColumn{
                s::FlexRow{s::CheckBox("Show overlay layer", show_overlay.binding()).key("show-overlay"),
                           s::CheckBox("Raise rear layer", raise_rear_layer.binding()).key("raise-rear"),
                           s::Label("RELATIVE +4", s::LabelStyle::micro)
                               .position(s::Position::relative)
                               .top(4)
                               .foreground(palette().cyan)}
                    .gap(16)
                    .wrap(),
                s::Layer{s::Panel(s::Spacer{})
                             .background(palette().light_mode ? 0xffe5ecf2 : 0xff122230)
                             .key("base-layer")
                             .z_index(-1),
                         s::Panel(s::FlexColumn{
                                      s::Label("REAR LAYER", s::LabelStyle::micro),
                                      s::Button("Rear command", [this] { command("Rear layer command."); })}
                                      .gap(10))
                             .background(palette().light_mode ? 0xffc5dfec : 0xff29495c)
                             .position(s::Position::absolute)
                             .left(16)
                             .top(20)
                             .width(s::percent(62))
                             .height(110)
                             .z_index(raise_rear_layer.get() ? 3 : 1)
                             .key("rear-layer"),
                         s::Panel(s::FlexColumn{s::Label("OVERLAY LAYER", s::LabelStyle::micro),
                                                s::Button("Overlay command",
                                                          [this] { command("Overlay layer command."); })
                                                    .accent(0xff4dd2ff)}
                                      .gap(10))
                             .background(palette().light_mode ? 0xffecdbc9 : 0xff493a32)
                             .position(s::Position::absolute)
                             .right(16)
                             .top(78)
                             .width(s::percent(62))
                             .height(110)
                             .z_index(2)
                             .hidden(!show_overlay.get())
                             .key("overlay-layer"),
                         s::Label("BOTTOM / RIGHT ANCHORED", s::LabelStyle::micro)
                             .position(s::Position::absolute)
                             .right(12)
                             .bottom(10)
                             .z_index(4)
                             .foreground(palette().secondary)}
                    .height(232)
                    .key("layer-stage"),
                s::Label("Absolute layers share the stage's content box. Hide the overlay or change its "
                         "stacking order above.",
                         s::LabelStyle::secondary)
                    .text_wrap(s::TextWrap::wrap)}
                .gap(14));
    }
    s::View wizard_section() {
        return s::Section(
            "Wizard / guided setup",
            s::Panel(s::If(
                !wizard_done.get() && !wizard_cancelled.get(),
                [this] {
                    s::View page;
                    const auto step = wizard_step.get();
                    if (step == "identity")
                        page = s::labeled(
                            "SHIP NAME",
                            s::TextField("Name your ship", wizard_name.binding(),
                                         {.max_length = 48, .on_submit = [this] { setup_wizard.next(); }})
                                .key("wizard-name")
                                .accessible_label("Wizard ship name"));
                    else if (step == "destination")
                        page = s::labeled("DESTINATION", s::ListBox(wizard_destination.binding(),
                                                                    {{"outer", "Outer sector"},
                                                                     {"nebula", "Nebula survey"},
                                                                     {"dock", "Orbital dock"}},
                                                                    {.visible_rows = 3})
                                                             .height(104)
                                                             .key("wizard-destination")
                                                             .accessible_label("Wizard destination"));
                    else
                        page = s::FlexColumn{s::Label("Ready to configure", s::LabelStyle::section),
                                             s::Label("Ship: " + wizard_name.get()),
                                             s::Label("Destination: " + wizard_destination.get()),
                                             s::Label(
                                                 "Go Back to revise a step, or Finish to complete this demo.",
                                                 s::LabelStyle::secondary)
                                                 .text_wrap(s::TextWrap::wrap)}
                                   .gap(8);
                    return s::FlexColumn{s::Label("Move through the steps without losing your entries.",
                                                  s::LabelStyle::secondary)
                                             .text_wrap(s::TextWrap::wrap),
                                         page.key("wizard-page-" + step), s::Separator{},
                                         s::WizardNavigation(setup_wizard).key("wizard-navigation")}
                        .gap(16);
                },
                [this] {
                    return s::FlexColumn{s::Label(wizard_done.get() ? "Setup complete" : "Setup cancelled",
                                                  s::LabelStyle::section),
                                         s::Label("Your entries stay in the model. Restart to revisit them.",
                                                  s::LabelStyle::secondary)
                                             .text_wrap(s::TextWrap::wrap),
                                         s::Button("Restart setup",
                                                   [this] {
                                                       wizard_step.set("identity");
                                                       wizard_done.set(false);
                                                       wizard_cancelled.set(false);
                                                   })
                                             .density(s::Density::compact)}
                        .gap(12);
                })));
    }
    s::View editing_section() {
        return s::Section(
            "Mission text",
            s::FlexColumn{
                s::labeled(
                    "CALL SIGN / ENTER TO SUBMIT",
                    s::TextField("Ship call sign", call_sign.binding(),
                                 {.max_length = 48,
                                  .on_submit = [this] { command("Call sign saved: " + call_sign.get()); }})
                        .key("call-sign")
                        .accessible_label("Ship call sign")),
                s::labeled("MISSION NOTES", s::TextEditor(notes.binding(), "Write a briefing…")
                                                .height(100)
                                                .key("mission-notes")
                                                .accessible_label("Mission notes")),
                s::labeled("READ ONLY", s::TextField("", assigned_id.binding(), {.read_only = true})
                                            .key("assigned-id")
                                            .accessible_label("Assigned identifier")),
                s::Label("Click or drag to select · double-click a word · ⌘A/C/X/V · ⌘Z / ⇧⌘Z · ⌥ arrows for "
                         "words",
                         s::LabelStyle::secondary)
                    .text_wrap(s::TextWrap::wrap)}
                .gap(12));
    }
    s::View collections_section() {
        auto items = stations.get();
        return s::Section(
            "Collections & bitmap display",
            s::FlexColumn{s::FlexRow{s::labeled("STATION / SINGLE SELECTION",
                                                s::ListBox(station.binding(), items,
                                                           {.visible_rows = 4,
                                                            .on_activate =
                                                                [this](const std::string& id) {
                                                                    command("Station opened: " + id);
                                                                }})
                                                    .key("station-list")
                                                    .accessible_label("Stations"))
                                         .min_width(220)
                                         .grow(),
                                     s::labeled("MONITOR / MULTIPLE SELECTION",
                                                s::ListBox(monitored.binding(), items, {.visible_rows = 4})
                                                    .key("monitor-list")
                                                    .accessible_label("Monitored stations"))
                                         .min_width(220)
                                         .grow(),
                                     s::FlexColumn{s::Label("BITMAP / RGBA", s::LabelStyle::micro),
                                                   s::Image(telemetry_bitmap())
                                                       .height(136)
                                                       .width(256)
                                                       .accessible_label("Telemetry bitmap")}
                                         .gap(6)}
                              .gap(16)
                              .wrap()
                              .align_items(s::Align::start),
                          s::FlexRow{s::Button("Add station",
                                               [this] {
                                                   auto next = stations.get();
                                                   const auto id = "unit-" + std::to_string(next_station++);
                                                   next.push_back({id, "Station " + id});
                                                   stations.set(std::move(next));
                                               })
                                         .density(s::Density::compact),
                                     s::Button("Remove selected",
                                               [this] {
                                                   auto next = stations.get();
                                                   const auto selected = station.get();
                                                   std::erase_if(
                                                       next, [&](const auto& o) { return o.id == selected; });
                                                   station.set(next.empty() ? "" : next.front().id);
                                                   stations.set(std::move(next));
                                               })
                                         .density(s::Density::compact)
                                         .enabled(!items.empty())}
                              .gap(10),
                          s::FlexRow{
                              s::ForEach(
                                  items, [](const s::Option& o) { return o.id; },
                                  [this](const s::Option& o) { return badge(o.label, palette().cyan); })
                                  .key("station-tags")}
                              .gap(8)
                              .wrap(),
                          s::If(
                              !station.get().empty(),
                              [this] {
                                  return s::Label("Selected station: " + station.get(),
                                                  s::LabelStyle::status);
                              },
                              [] {
                                  return s::Label("Add a station to get started.", s::LabelStyle::secondary);
                              })
                              .key("station-detail")}
                .gap(14));
    }
    static s::Bitmap telemetry_bitmap() {
        static const auto image = [] {
            constexpr int w = 256, h = 128;
            std::vector<std::uint8_t> pixels(w * h * 4);
            for (int y = 0; y < h; ++y)
                for (int x = 0; x < w; ++x) {
                    const double wave = 64 + 28 * std::sin(x * .07) + 10 * std::sin(x * .19);
                    auto color = std::abs(y - wave) < 2       ? 0xff4dd2ffu
                                 : x % 32 == 0 || y % 32 == 0 ? 0xff294252u
                                                              : 0xff112331u;
                    if (x > 212 && y > 8 && y < 22)
                        color = 0xffffaa34u;
                    const int i = (y * w + x) * 4;
                    pixels[i] = (color >> 16) & 255;
                    pixels[i + 1] = (color >> 8) & 255;
                    pixels[i + 2] = color & 255;
                    pixels[i + 3] = 255;
                }
            return s::Bitmap::from_rgba(w, h, pixels);
        }();
        return image;
    }
    s::View split_section() {
        return s::Section("Resizable panes",
            s::FlexColumn{
                s::Label("Drag a divider or Tab to it and use the arrow keys. Shift moves faster. "
                         "Enter collapses the first pane; Shift-Enter collapses the second; Escape restores it.",
                         s::LabelStyle::secondary).text_wrap(s::TextWrap::wrap),
                s::SplitView(
                    s::Panel(s::FlexColumn{s::Label("CREW", s::LabelStyle::micro),
                                          s::TextField("Call sign", call_sign.binding()),
                                          s::Label("Pane state survives collapse.", s::LabelStyle::secondary)
                                              .text_wrap(s::TextWrap::wrap)}.gap(12).padding(12)),
                    s::SplitView(
                        s::Panel(s::Label("Navigation").padding(12)),
                        s::Panel(s::Label("Telemetry").padding(12)), vertical_fraction.binding(),
                        {.orientation = s::Orientation::vertical, .first_min = 48, .second_min = 48})
                        .accessible_label("Navigation and telemetry"),
                    split_fraction.binding(), {.first_min = 180, .second_min = 160,
                                               .collapsed = collapsed_pane.binding()})
                    .height(230).key("split-demo").accessible_label("Crew and systems"),
                s::Button("Restore both panes", [this] { collapsed_pane.set(s::SplitPane::none); })
            }.gap(12));
    }
    s::View tabs_section() {
        return s::Section("Tabbed pages",
            s::FlexColumn{
                s::Label("Tab enters the selected page. Use Left/Right on the strip, or Control-Tab "
                         "within a page, to switch. Editing and scroll state survive page changes.",
                         s::LabelStyle::secondary).text_wrap(s::TextWrap::wrap),
                s::TabView(selected_tab.binding(), {
                    {"mission", "Mission", [this] {
                        return s::Panel(s::FlexColumn{
                            s::Label("MISSION NOTES", s::LabelStyle::micro),
                            s::TextEditor(notes.binding(), "Mission notes").grow().key("tab-notes")
                        }.gap(12).padding(12));
                    }},
                    {"telemetry", "Telemetry", [this] {
                        return s::Panel(s::FlexColumn{
                            s::Label("SIGNAL STRENGTH", s::LabelStyle::micro),
                            s::Slider(volume.binding()), s::ProgressBar(progress.binding()),
                            s::Label("Return to Mission to continue editing.", s::LabelStyle::secondary)
                                .text_wrap(s::TextWrap::wrap)
                        }.gap(12).padding(12));
                    }},
                    {"offline", "Offline", [] { return s::Label("Unavailable"); }, false}
                }).height(240).key("tabs-demo").accessible_label("Mission console")
            }.gap(12));
    }
    s::View numeric_section() {
        const auto validation = numeric_validation.get();
        return s::Section("Numeric inputs and switches",
            s::FlexColumn{
                s::Label("Numbers keep a draft until Enter or blur. Escape restores the committed value. "
                         "Use arrow keys or hold a stepper button to adjust.",
                         s::LabelStyle::secondary).text_wrap(s::TextWrap::wrap),
                s::FlexRow{
                    s::FlexColumn{s::Label("TARGET POWER", s::LabelStyle::micro),
                        s::NumericField("Power", target_power.binding(), {.min = 0, .max = 10, .step = .25,
                            .significant_digits = 5, .on_validation = [this](const auto& v) { numeric_validation.set(v); }})
                            .key("target-power").accessible_label("Target power")}.gap(8).width(180),
                    s::FlexColumn{s::Label("CREW", s::LabelStyle::micro),
                        s::Stepper(crew_count.binding(), {.min = 1, .max = 20})
                            .key("crew-stepper").accessible_label("Crew count")}.gap(8).width(180),
                    s::FlexColumn{s::Label("AUTOMATION", s::LabelStyle::micro),
                        s::Switch("Automatic scan", automatic_scan.binding()).key("auto-scan")}.gap(8).grow()
                }.gap(20).wrap(),
                s::Label(validation.message.empty() ? (validation.dirty ? "Valid draft · press Enter to commit" :
                         "Committed power: " + std::to_string(target_power.get())) : validation.message,
                         validation.status == s::NumericStatus::valid ? s::LabelStyle::secondary : s::LabelStyle::error)
                    .text_wrap(s::TextWrap::wrap)
            }.gap(16));
    }
    s::View text_controls_section() {
        auto lower = [](std::string text) {
            std::transform(text.begin(), text.end(), text.begin(), [](unsigned char c) { return char(std::tolower(c)); });
            return text;
        };
        const auto filter = lower(last_search.get());
        std::vector<s::Option> matches;
        for (const auto& station : stations.get())
            if (lower(station.label).find(filter) != std::string::npos)
                matches.push_back(station);
        return s::Section("Search and password fields",
            s::FlexColumn{
                s::Label("Command-F focuses search. Type to filter stations, Enter to submit, or Escape to clear. "
                         "Show reveals an access code while editing; leaving the field hides it again.",
                         s::LabelStyle::secondary).text_wrap(s::TextWrap::wrap),
                s::SearchField("Find a station", search_text.binding(), {
                    .debounce = std::chrono::milliseconds(180),
                    .on_search = [this](const auto& query) { last_search.set(query); },
                    .on_submit = [this](const auto& query) { status.set("Search submitted: " + query); },
                    .on_cancel = [this] { last_search.set(""); }
                }).key("station-search").accessible_label("Search stations"),
                s::ListBox(search_selected.binding(), std::move(matches), {.visible_rows = 3})
                    .height(104).key("search-results").accessible_label("Matching stations"),
                s::Label("ACCESS CODE", s::LabelStyle::micro),
                s::PasswordField("Access code", access_code.binding(), {.allow_reveal = true, .max_length = 64,
                    .on_submit = [this] { credential_status.set("Demo access code submitted."); }})
                    .key("access-code").accessible_label("Demo access code"),
                s::Label(credential_status.get(), s::LabelStyle::secondary).text_wrap(s::TextWrap::wrap)
            }.gap(12));
    }
    s::View overlays_section() {
        return s::Section("Overlays", s::FlexColumn{
            s::Label("Hover or Tab to a control for help. Popovers fit the window, keep their own Tab order, "
                     "and close on Escape or an outside click.", s::LabelStyle::secondary).text_wrap(s::TextWrap::wrap),
            s::FlexRow{
                s::Button("Scan settings", [this] { show_settings.set(true); }).key("overlay-settings")
                    .tooltip("Set the scan target and automation without leaving the current view.")
                    .popover(show_settings.binding(), [this] {
                        return s::FlexColumn{
                            s::Label("SCAN SETTINGS", s::LabelStyle::micro),
                            s::TextField("Ship call sign", call_sign.binding()).accessible_label("Popover call sign"),
                            s::Switch("Automatic scan", automatic_scan.binding()),
                            s::Label("TARGET POWER", s::LabelStyle::micro),
                            s::NumericField("Power", target_power.binding(), {.min = 0, .max = 10, .step = .25}),
                            s::FlexRow{
                                s::Button("Help", [this] { show_overlay_help.set(true); })
                                    .popover(show_overlay_help.binding(), [] {
                                        return s::Label("Power controls the scan's range. Automation runs another scan after each result.")
                                            .text_wrap(s::TextWrap::wrap);
                                    }, {.placement = s::OverlayPlacement::right, .width = 240}),
                                s::Spacer{}, s::Button("Done", [this] { show_settings.set(false); })
                            }.gap(8)
                        }.gap(12);
                    }, {.width = 330}),
                s::Button("Run scan", [this] { command("Scan started"); })
                    .tooltip("Start one scan using the current settings.")
            }.gap(16).wrap(),
            s::Panel(s::FlexColumn{
                s::Label("STATION ACTIONS", s::LabelStyle::micro),
                s::Label("Right-click or Control-click here for actions. Tab to the button and press Shift-F10.")
                    .text_wrap(s::TextWrap::wrap),
                s::Button("Report station", [this] { command("Reporting " + station.get()); })
            }.gap(10)).padding(16).key("context-target").context_menu([this] {
                return std::vector<s::MenuItem>{
                    {"report", "Report station", [this] { command("Reporting " + station.get()); }, true, "⌘R"},
                    s::MenuItem::separator("settings-divider"),
                    s::MenuItem::check("grid", "Show grid", grid.binding()),
                    s::MenuItem::submenu("station", "Station", {
                        s::MenuItem::radio("nav", "Navigation", station.binding(), "nav"),
                        s::MenuItem::radio("sci", "Science", station.binding(), "sci"),
                        s::MenuItem::radio("eng", "Engineering", station.binding(), "eng")
                    }),
                    {"offline", "Offline station", [] {}, false}
                };
            })
        }.gap(16));
    }
    s::View combo_section() {
        return s::Section("Editable selection", s::FlexColumn{
            s::Label("Type to filter stations. Arrow keys browse suggestions while the cursor stays in the editor. "
                     "Enter accepts; Escape restores the last accepted value.", s::LabelStyle::secondary).text_wrap(s::TextWrap::wrap),
            s::Label("STATION · STRICT SELECTION", s::LabelStyle::micro),
            s::ComboBox("Choose a station", combo_query.binding(), combo_selection.binding(), stations.get(), {
                .loading = combo_loading.get(),
                .on_accept = [this](const auto& id, const auto& label) { command("Station: " + id + " · " + label); }
            }).key("station-combo").accessible_label("Station suggestions"),
            s::Switch("Simulate loading", combo_loading.binding()),
            s::Label("SHIP · FREE TEXT", s::LabelStyle::micro),
            s::ComboBox("Choose a ship or enter another", free_query.binding(), free_selection.binding(),
                {{"kestrel", "USS Kestrel"}, {"eclipse", "USS Eclipse"}},
                {.mode = s::ComboBoxMode::free_text, .auto_highlight = false,
                 .on_accept = [this](const auto&, const auto& label) { command("Ship: " + label); }}).key("ship-combo"),
            s::Label("Station ID: " + combo_selection.get() + " · Ship ID: " +
                (free_selection.get().empty() ? "free text" : free_selection.get()), s::LabelStyle::secondary)
        }.gap(12));
    }
    s::View scrolling_section() {
        std::vector<s::View> rows;
        for (int row = 1; row <= 8; ++row) {
            std::vector<s::View> cells;
            for (int column = 1; column <= 6; ++column) {
                const auto id = std::to_string(row) + " / " + std::to_string(column);
                cells.push_back(s::Panel(s::FlexColumn{
                    s::Label("SECTOR " + id, s::LabelStyle::micro),
                    s::Button("Select sector", [this, id] { selected_sector.set(id); }).density(s::Density::compact)
                }.gap(8)).width(180).height(90).padding(12).key(id));
            }
            rows.push_back(s::FlexRow(std::move(cells)).gap(12).key(std::to_string(row)));
        }
        return s::Section("Two-axis scrolling", s::FlexColumn{
            s::Label("Pan the sector map with a trackpad, or hold Shift while using a wheel to move horizontally. "
                     "Tab reveals a control through every containing viewport.", s::LabelStyle::secondary).text_wrap(s::TextWrap::wrap),
            s::ScrollView(s::FlexColumn(std::move(rows)).gap(12), {.axes = s::ScrollAxes::both})
                .key("sector-scroll").height(280).padding(8).background(palette().group).corner_radius(8),
            s::Label("Selected sector: " + selected_sector.get(), s::LabelStyle::status)
        }.gap(14));
    }
    s::View workbench_section() {
        s::Binding<std::string> group{
            [this]{return workbench_group.get();},[this](std::string value){workbench_group.set(std::move(value));workbench_table.set(make_workbench_table());}};
        return s::Section("Data workbench",s::FlexColumn{
            s::Label("Select a collection, filter observations, or click a column heading to request a sort. "
                     "Double-click Power or press F2 to edit; Enter applies, Escape cancels, and Tab commits and moves to the next editable cell.",
                     s::LabelStyle::secondary).text_wrap(s::TextWrap::wrap),
            s::SearchField("Filter observations",workbench_query.binding(),{
                .on_search=[this](const auto&){workbench_table.set(make_workbench_table());}}).key("workbench-search"),
            s::SplitView(
                s::TreeView(group,workbench_folders,workbench_expanded.binding(),{.visible_rows=6}).key("workbench-tree").font_size(12),
                s::TableView(workbench_selection.binding(),workbench_table.get(),{
                    .selection_mode=s::TableSelectionMode::cells,.multiple_selection=true,.widths=workbench_widths.binding(),.sort=workbench_sort.binding(),
                    .on_sort=[this](const auto&){workbench_table.set(make_workbench_table());}})
                    .key("workbench-table").font_size(12).accessible_label("Observation data")
                    .context_menu({s::MenuItem{"clear","Clear selection",[this]{workbench_selection.set({});}},
                                   s::MenuItem{"widths","Reset column widths",[this]{workbench_widths.set({});}}}),
                workbench_fraction.binding(),{.first_min=140,.second_min=200}).height(360).key("workbench-split"),
            s::Label(std::to_string(workbench_table.get().rows().size())+" records / "+
                     std::to_string(workbench_selection.get().cells.size())+" selected cells",s::LabelStyle::status)
        }.gap(14));
    }
    s::View tree_section() {
        return s::Section("Hierarchical browser", s::FlexColumn{
            s::Label("Use Left/Right to collapse, expand, and enter branches; Up/Down and typeahead navigate visible nodes. "
                     "The archive loads 2,000 child records on first expansion.", s::LabelStyle::secondary).text_wrap(s::TextWrap::wrap),
            s::TreeView(browser_selection.binding(), browser_tree.get(), browser_expanded.binding(),
                        {.row_height = 32, .visible_rows = 8})
                .key("systems-tree").height(282).accessible_label("Systems hierarchy")
                .context_menu({s::MenuItem{"open-fleet", "Expand survey fleet", [this] {
                    auto open = browser_expanded.get();
                    if (std::find(open.begin(), open.end(), "fleet") == open.end()) open.push_back("fleet");
                    browser_expanded.set(std::move(open));
                }}}),
            s::FlexRow{
                s::Button("Load remote data", [this] { browser_loaded.set(true); browser_tree.set(make_browser_tree(true)); })
                    .enabled(!browser_loaded.get()).density(s::Density::compact),
                s::Label("Selected: " + browser_selection.get(), s::LabelStyle::status).text_wrap(s::TextWrap::break_word).grow()
            }.gap(12).wrap().align_items(s::Align::center),
            s::Label("Loading and data replacement belong to the application model. Disabled nodes remain visible.",
                     s::LabelStyle::micro).text_wrap(s::TextWrap::wrap).max_lines(0).text_overflow(s::TextOverflow::clip)
        }.gap(14));
    }
    s::View virtualization_section() {
        return s::Section("100,000-row observations", s::FlexColumn{
            s::Label("Only visible rows, overscan, and an active editor are mounted. Focus the list and use Home/End "
                     "to jump. Edit an observation, then scroll away; composition stays alive until editing ends.",
                     s::LabelStyle::secondary).text_wrap(s::TextWrap::wrap),
            s::ListBox(virtual_selected.binding(), virtual_rows, {.row_height = 44, .visible_rows = 7, .overscan_rows = 2})
                .key("virtual-list").height(310).accessible_label("Observation records"),
            s::Label("Selected record ID: " + virtual_selected.get(), s::LabelStyle::status),
            s::Label("Edits saved in the application model: " + std::to_string(virtual_edits.get().size()), s::LabelStyle::micro)
        }.gap(14));
    }
    s::View body() {
        return s::ScrollView(
                   s::FlexColumn{
                       header(), command_section(), s::Separator{}, dropdown_section(), s::Separator{},
                       form(), split_section(), tabs_section(), numeric_section(), text_controls_section(), overlays_section(), combo_section(), scrolling_section(), virtualization_section(), tree_section(), workbench_section(), alerts_section(), datetime_section(), colors_section(), wizard_section(), editing_section(), collections_section(), layers_section(),
                       typography(),
                       s::Panel(s::FlexRow{s::FlexColumn{s::Label("COMMAND LOG", s::LabelStyle::micro),
                                                         s::Label(status.get(), s::LabelStyle::status)
                                                             .text_wrap(s::TextWrap::wrap)}
                                               .gap(6)
                                               .grow(),
                                           badge(std::to_string(commands.get()) + " EXECUTED", 0xffffaa34)}
                                    .gap(16)
                                    .align_items(s::Align::center)),
                       s::Label("TAB TO NAVIGATE  /  ARROWS FOR GROUPS & MENUS  /  SPACE TO ACTIVATE  /  ESC "
                                "TO DISMISS",
                                s::LabelStyle::micro)
                           .text_wrap(s::TextWrap::wrap)
                           .text_overflow(s::TextOverflow::clip)
                           .max_lines(0)}
                       .gap(24)
                       .padding(28))
            .key("viewport");
    }
};
void check_workbench() {
    Lab model;
    auto theme = s::Theme{};
    theme.font_path = bundled_font();
    s::Host host([&] { return model.workbench_section().padding(20); }, {}, theme);
    host.layout({1000, 700});
    int checks{};
    const auto require = [&](bool value, const char* text) {
        ++checks;
        if (!value)
            throw std::runtime_error(text);
    };
    const auto keyed = [&](const std::string& key) {
        for (const auto& node : host.inspect())
            if (node.key == key)
                return node;
        throw std::runtime_error("Missing workbench element: " + key);
    };
    const auto table = keyed("workbench-table").id, tree = keyed("workbench-tree").id,
               search = keyed("workbench-search").id;
    require(keyed("workbench-table").list->logical_count == 1000, "Workbench initial snapshot is incomplete");
    host.activate(tree + "/option/archived");
    require(model.workbench_group.get() == "archived" && keyed("workbench-table").list->logical_count == 500,
            "Workbench tree selection did not filter the table");
    host.focus(search);
    host.set_text(search, "19");
    host.advance_time();
    const auto count = keyed("workbench-table").list->logical_count;
    require(count > 0 && count < 500, "Workbench search did not publish a filtered snapshot");
    for (std::size_t i = 0; i < count; ++i) {
        const auto row = *host.list_item(table, i);
        const auto cell = *host.table_cell_at(row.id, 2);
        require(row.label.find("19") != std::string::npos && std::stoi(cell.cell.row) % 2 != 0,
                "Workbench filtering lost its tree or query constraint");
    }
    host.activate(table + "/header/power");
    auto first = *host.table_cell_at(host.list_item(table, 0)->id, 2),
         last = *host.table_cell_at(host.list_item(table, count - 1)->id, 2);
    require(std::stoi(first.value) <= std::stoi(last.value) &&
                model.workbench_sort.get()[0].order == s::SortOrder::ascending,
            "Workbench ascending sort request did not reorder its model");
    host.activate(table + "/header/power");
    first = *host.table_cell_at(host.list_item(table, 0)->id, 2);
    last = *host.table_cell_at(host.list_item(table, count - 1)->id, 2);
    require(std::stoi(first.value) >= std::stoi(last.value),
            "Workbench descending sort did not reorder its model");
    require(host.edit_cell(table, first.cell), "Workbench cell could not enter editing");
    host.insert_text("101");
    require(!host.commit_table_edit(table) && model.workbench_values.get().empty(),
            "Workbench accepted an invalid numeric draft");
    host.text_command(s::TextCommand::select_all);
    host.insert_text("55");
    require(host.commit_table_edit(table) && model.workbench_values.get().at(first.cell.row) == "55" &&
                host.table_cell(first.id)->value == "55",
            "Workbench edit lost identity after sorting on commit");
    host.set_value(table + "/header/name/resize", 333);
    require(keyed("workbench-table").table->columns[0].width == 333,
            "Workbench column width binding did not update");
    const auto menu_action = [&](const std::string& label) {
        if (!host.show_context_menu(table))
            throw std::runtime_error("Workbench context menu did not open");
        for (const auto& element : host.elements())
            if (element.role == s::Role::menu_item && element.label == label) {
                host.activate(element.id);
                return;
            }
        throw std::runtime_error("Missing workbench menu action: " + label);
    };
    menu_action("Reset column widths");
    require(keyed("workbench-table").table->columns[0].width == 220,
            "Workbench reset did not restore schema widths");
    menu_action("Clear selection");
    require(model.workbench_selection.get().cells.empty(),
            "Workbench context menu did not clear its selection");
    model.light_mode.set(true);
    auto light = s::Theme::light();
    light.font_path = bundled_font();
    host.set_theme(light);
    host.layout({360, 760});
    host.reveal_cell(table, first.cell);
    auto raster = SkSurfaces::Raster(SkImageInfo::MakeN32Premul(720, 1520));
    raster->getCanvas()->scale(2, 2);
    host.paint(*raster->getCanvas(), 0);
    require(model.workbench_values.get().at(first.cell.row) == "55" && model.workbench_query.get() == "19" &&
                model.workbench_group.get() == "archived",
            "Workbench theme/resize changes lost application state");
    host.focus(table);
    host.paint(*raster->getCanvas(), 1);
    require(!host.needs_frame(), "An idle workbench kept scheduling frames");
    std::cout << checks << " workbench interaction checks passed\n";
}
void render(const char* output, double width, double height, bool explorer = false, bool light = false,
            bool wizard = false, bool split = false, bool tabs = false, bool numeric = false, bool text_controls = false,
            bool overlays = false, bool context = false, bool combo = false, bool scrolling = false, bool virtualization = false, bool tree = false, bool workbench = false, bool alert = false, bool datetime = false, bool colors = false) {
    Lab model;
    model.light_mode.set(light);
    if (overlays) model.show_settings.set(true);
    auto theme = light ? s::Theme::light() : s::Theme{};
    theme.font_path = bundled_font();
    auto surface =
        SkSurfaces::Raster(SkImageInfo::MakeN32Premul(static_cast<int>(width), static_cast<int>(height)));
    if (!surface)
        throw std::runtime_error("Cannot create preview surface");
    auto body = [&] {
        if (wizard) return model.wizard_section().padding(20);
        if (split) return model.split_section().padding(20);
        if (tabs) return model.tabs_section().padding(20);
        if (numeric) return model.numeric_section().padding(20);
        if (text_controls) return model.text_controls_section().padding(20);
        if (overlays || context) return model.overlays_section().padding(20);
        if (combo) return model.combo_section().padding(20);
        if (scrolling) return model.scrolling_section().padding(20);
        if (virtualization) return model.virtualization_section().padding(20);
        if (tree) return model.tree_section().padding(20);
        if(datetime)return model.datetime_section().padding(20);
        if(colors)return model.colors_section().padding(20);
        if (alert) return model.alert_preview();
        if (workbench) return model.workbench_section().padding(20);
        return model.body();
    };
    auto source = std::make_shared<s::Host>(body, std::function<void()>{}, theme);
    source->layout(explorer ? s::Size{1180, 1000} : s::Size{width, height});
    if(datetime){for(const auto& n:source->inspect())if(n.key=="mission-date"){source->activate(n.id+"/action");break;}}
    if (context) {
        for (const auto& n : source->inspect())
            if (n.key == "context-target") { source->show_context_menu(n.id, s::Point{n.bounds.x + 24, n.bounds.y + 24}); break; }
        for (const auto& e : source->elements())
            if (e.role == s::Role::menu_item && e.label == "Station") { source->activate(e.id); break; }
    }
    if (combo) {
        for (const auto& n : source->inspect()) if (n.key == "station-combo") {
            source->focus(n.id); source->text_command(s::TextCommand::select_all); source->insert_text("s"); break;
        }
    }
    if (scrolling) {
        for (const auto& n : source->inspect()) if (n.key == "sector-scroll") {
            source->scroll_to(n.id, {260, 170}); break;
        }
    }
    if (explorer) {
        for (const auto& n : source->inspect())
            if (n.key == "grid") {
                source->set_inspected_element(n.id);
                break;
            }
        s::Inspector inspector(source);
        inspector.reveal_selection();
        s::Host view([&] { return inspector.body(); }, {}, theme);
        view.layout({width, height});
        for (const auto& n : view.inspect())
            if (n.key == source->inspected_element()) {
                view.reveal(n.id);
                break;
            }
        view.paint(*surface->getCanvas());
    } else
        source->paint(*surface->getCanvas());
    SkBitmap bitmap;
    bitmap.allocN32Pixels(width, height);
    if (!surface->readPixels(bitmap, 0, 0))
        throw std::runtime_error("Preview readback failed");
    SkFILEWStream stream(output);
    if (!SkPngEncoder::Encode(&stream, bitmap.pixmap(), {}))
        throw std::runtime_error("Preview encoding failed");
    std::cout << "SCARS preview: " << output << '\n';
}
} // namespace
int main(int argc, char** argv) {
    try {
        if(argc>1&&std::string(argv[1])=="--check-workbench"){check_workbench();return 0;}
        if (argc > 2 && (std::string(argv[1]) == "--render" || std::string(argv[1]) == "--render-inspector" ||
                         std::string(argv[1]) == "--render-wizard" || std::string(argv[1]) == "--render-split" ||
                         std::string(argv[1]) == "--render-tabs" || std::string(argv[1]) == "--render-inputs" ||
                         std::string(argv[1]) == "--render-text-controls" ||
                         std::string(argv[1]) == "--render-overlays" ||
                         std::string(argv[1]) == "--render-context-menu" ||
                         std::string(argv[1]) == "--render-combo" ||
                         std::string(argv[1]) == "--render-scroll" ||
                         std::string(argv[1]) == "--render-virtualization" ||
                         std::string(argv[1]) == "--render-tree" ||
                         std::string(argv[1]) == "--render-workbench" || std::string(argv[1]) == "--render-alert" || std::string(argv[1]) == "--render-datetime" || std::string(argv[1]) == "--render-colors")) {
            const bool explorer = std::string(argv[1]) == "--render-inspector";
            const bool wizard = std::string(argv[1]) == "--render-wizard";
            const bool split = std::string(argv[1]) == "--render-split";
            const bool tabs = std::string(argv[1]) == "--render-tabs";
            const bool numeric = std::string(argv[1]) == "--render-inputs";
            const bool text_controls = std::string(argv[1]) == "--render-text-controls";
            const bool overlays = std::string(argv[1]) == "--render-overlays";
            const bool context = std::string(argv[1]) == "--render-context-menu";
            const bool combo = std::string(argv[1]) == "--render-combo";
            const bool scrolling = std::string(argv[1]) == "--render-scroll";
            const bool virtualization = std::string(argv[1]) == "--render-virtualization";
            const bool tree = std::string(argv[1]) == "--render-tree";
            const bool workbench = std::string(argv[1]) == "--render-workbench";
            const bool alert = std::string(argv[1]) == "--render-alert";
            const bool datetime = std::string(argv[1]) == "--render-datetime";
            const bool colors = std::string(argv[1]) == "--render-colors";
            double width = argc > 3   ? std::stod(argv[3])
                           : datetime ? 760
                           : alert ? 480
                           : wizard || split || tabs || numeric || text_controls || overlays || context || combo || scrolling || virtualization || tree || workbench ? 760
                           : explorer ? 980
                                      : 1180,
                   height = argc > 4   ? std::stod(argv[4])
                            : datetime ? 700
                            : alert ? 320
                            : workbench ? 700
                            : text_controls || overlays || context || combo || scrolling || virtualization || tree || workbench ? 600
                            : wizard || split || tabs || numeric ? 480
                            : explorer ? 760
                                       : 1300;
            if (!std::isfinite(width) || !std::isfinite(height) || width < 320 || width > 4096 ||
                height < 240 || height > 4096)
                throw std::invalid_argument("Preview dimensions out of range");
            const bool light = argc > 5 && std::string(argv[5]) == "--light";
            render(argv[2], width, height, explorer, light, wizard, split, tabs, numeric, text_controls, overlays, context, combo, scrolling, virtualization, tree, workbench, alert, datetime, colors);
            return 0;
        }
        auto application = sw::Application::create({.name = "SCARS Lab"});
        if (!application)
            throw std::runtime_error(application.error().message);
        auto app = std::move(*application);
        auto result = sw::Window::create(
            *app, {.title = "SCARS Lab / Ship systems", .size = {1180, 1000}, .minimum_size = {420, 480}});
        if (!result)
            throw std::runtime_error(result.error().message);
        auto window = std::move(*result);
        auto model = std::make_shared<Lab>();
        s::Theme theme;
        theme.font_path = bundled_font();
        auto tools_slot = std::make_shared<std::weak_ptr<s::macos::DeveloperTools>>();
        auto host = s::macos::mount(*window, [model] { return model->body(); }, theme, {},
                                    {.on_closed = [tools_slot] {
                                        if (auto tools = tools_slot->lock())
                                            tools->close();
                                    }});
        auto presentation = std::make_shared<std::unique_ptr<s::macos::AlertPresentation>>();
        std::weak_ptr<Lab> weak_model = model;
        model->present_alert = [weak_model, presentation, parent = window.get()](bool reset) {
            auto model = weak_model.lock();
            if (!model || (*presentation && (*presentation)->is_open()))
                return;
            auto theme = model->palette();
            theme.font_path = bundled_font();
            if (reset) {
                auto shown = s::macos::confirm(*parent, "Reset scan?", "Confirm resetting the scan settings.",
                                               [weak_model](bool accepted) {
                                                   if (auto model = weak_model.lock())
                                                       model->alert_status.set(accepted ? "Reset confirmed."
                                                                                        : "Reset cancelled.");
                                               },
                                               theme, {.accept_label = "Reset", .destructive = true});
                if (!shown) {
                    model->alert_status.set(shown.error().message);
                    return;
                }
                *presentation = std::move(*shown);
            } else {
                auto shown = s::macos::AlertPresentation::present(
                    *parent, "Apply scan changes?", model->alert_content(),
                    {{"cancel", "Cancel", s::AlertActionRole::cancel},
                     {"archive", "Archive", s::AlertActionRole::destructive},
                     {"apply", "Apply", s::AlertActionRole::default_action}},
                    [weak_model](auto action) {
                        if (auto model = weak_model.lock())
                            model->alert_status.set(action ? "Selected " + *action + " for " +
                                                                 model->alert_note.get()
                                                           : "Alert dismissed.");
                    },
                    theme);
                if (!shown) {
                    model->alert_status.set(shown.error().message);
                    return;
                }
                *presentation = std::move(*shown);
            }
        };
        auto apply_theme = [model, host, presentation, native = window.get()](bool light) {
            model->light_mode.set(light);
            auto next = light ? s::Theme::light() : s::Theme{};
            next.font_path = bundled_font();
            host->set_theme(next);
            if (*presentation && (*presentation)->is_open())
                (*presentation)->host()->set_theme(next);
            native->set_appearance(light ? sw::Appearance::light : sw::Appearance::dark);
            native->set_titlebar_theme_toggle_state(light);
            native->set_command_state(3, true, light);
        };
        window->set_titlebar_theme_toggle(false, apply_theme);
        window->set_appearance(sw::Appearance::dark);
        auto tools = std::make_shared<s::macos::DeveloperTools>(*app, host, theme);
        *tools_slot = tools;
        auto menu = std::vector<sw::MenuItem>{
            sw::MenuItem::submenu("SCARS Lab", {sw::MenuItem::action(1, "Quit SCARS Lab", {sw::Key::q})}),
            sw::MenuItem::submenu(
                "View",
                {sw::MenuItem::action(3, "Light mode", {sw::Key::l, {.shift = true, .command = true}})}),
            sw::MenuItem::submenu("Developer",
                                  {sw::MenuItem::action(2, "Toggle Element Explorer",
                                                        {sw::Key::i, {.alt = true, .command = true}})})};
        auto menu_result = window->set_menu(menu, [&](sw::Command command) {
            if (command == 1)
                app->quit();
            else if (command == 2)
                tools->toggle();
            else if (command == 3)
                apply_theme(!model->light_mode.get());
        });
        if (!menu_result)
            throw std::runtime_error(menu_result.error().message);
        window->show();
        if (argc > 1 && std::string(argv[1]) == "--smoke-test")
            app->post([&] {
                window->close();
                app->quit();
            });
        if (argc > 1 && std::string(argv[1]) == "--developer")
            tools->show();
        int exit_code = app->run();
        tools->close();
        window->close();
        return exit_code;
    } catch (const std::exception& e) {
        std::cerr << "SCARS Lab: " << e.what() << '\n';
        return 1;
    }
}
