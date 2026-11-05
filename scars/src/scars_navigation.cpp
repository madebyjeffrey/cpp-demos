module;
#include <algorithm>
#include <functional>
#include <memory>
#include <stdexcept>
#include <string>
#include <unordered_set>
#include <utility>
#include <vector>
module scars;

namespace scars {
struct Wizard::Impl {
    Binding<std::string> selection;
    std::vector<WizardStep> steps;
    WizardOptions options;
    Impl(Binding<std::string> value, std::vector<WizardStep> pages, WizardOptions settings)
        : selection(std::move(value)), steps(std::move(pages)), options(std::move(settings)) {
        if (steps.empty())
            throw std::invalid_argument("Wizard requires at least one step");
        std::unordered_set<std::string> keys;
        for (const auto& step : steps)
            if (step.id.empty() || !keys.insert(step.id).second)
                throw std::invalid_argument("Wizard step IDs must be nonempty and unique");
        for (const auto& label :
             {options.back_label, options.next_label, options.finish_label, options.cancel_label})
            if (label.empty())
                throw std::invalid_argument("Wizard action labels must not be empty");
        index();
    }
    std::size_t index() const {
        const auto value = selection.get();
        const auto found =
            std::find_if(steps.begin(), steps.end(), [&](const auto& step) { return step.id == value; });
        if (found == steps.end())
            throw std::invalid_argument("Wizard requires a valid current step ID: " + value);
        return std::size_t(found - steps.begin());
    }
    std::string error(std::size_t current) const {
        const bool check_all = current + 1 == steps.size() && options.validate_all_on_finish;
        for (std::size_t i = check_all ? 0 : current; i <= current; ++i) {
            if (!steps[i].validate)
                continue;
            auto message = steps[i].validate();
            if (!message.empty()) {
                if (i != current)
                    message = (steps[i].title.empty() ? steps[i].id : steps[i].title) + ": " + message;
                return message;
            }
        }
        return {};
    }
    bool transition(std::size_t target) const {
        const auto from = selection.get();
        const auto to = steps.at(target).id;
        if (from == to)
            return false;
        selection.set(to);
        const auto actual = selection.get();
        // A binding may reject or normalize a proposed transition.
        if (actual == from)
            return false;
        index();
        if (auto changed = options.on_change)
            changed(from, actual);
        return true;
    }
    View body(Wizard wizard) const {
        const auto current_index = index();
        const auto current_id = steps[current_index].id;
        const bool last = current_index + 1 == steps.size();
        const auto message = error(current_index);
        std::vector<View> content;
        if (options.show_steps) {
            std::vector<Option> choices;
            for (std::size_t i = 0; i < steps.size(); ++i) {
                auto title =
                    std::to_string(i + 1) + ". " + (steps[i].title.empty() ? steps[i].id : steps[i].title);
                if (i < current_index)
                    title = "✓ " + title;
                choices.push_back({steps[i].id, std::move(title),
                                   i == current_index ||
                                       (i < current_index && options.allow_back && options.review_previous)});
            }
            Binding<std::string> review([wizard] { return wizard.current(); },
                                        [wizard](std::string target) { wizard.go_to(target); });
            content.push_back(SegmentedControl(review, std::move(choices))
                                  .density(Density::compact)
                                  .key("steps")
                                  .accessible_label("Wizard steps"));
        }
        if (options.show_status)
            content.push_back(
                Label(message.empty() ? last ? "Ready to finish." : "Ready for the next step." : message,
                      message.empty() ? LabelStyle::secondary : LabelStyle::error)
                    .text_wrap(TextWrap::wrap)
                    .key("status")
                    .accessible_label(message.empty() ? "Wizard ready" : "Wizard validation: " + message));
        std::vector<View> actions;
        if (options.on_cancel)
            actions.push_back(Button(options.cancel_label, [wizard] { wizard.cancel(); })
                                  .density(Density::compact)
                                  .key("cancel"));
        actions.push_back(Spacer{}.key("space"));
        actions.push_back(Button(options.back_label,
                                 [wizard, current_id] {
                                     if (wizard.current() == current_id)
                                         wizard.back();
                                 })
                              .enabled(options.allow_back && current_index > 0)
                              .key("back"));
        actions.push_back(Button(last ? options.finish_label : options.next_label,
                                 [wizard, current_id, last] {
                                     if (wizard.current() != current_id)
                                         return;
                                     if (last)
                                         wizard.finish();
                                     else
                                         wizard.next();
                                 })
                              .enabled(message.empty() && (!last || bool(options.on_finish)))
                              .key("advance"));
        content.push_back(
            FlexRow(std::move(actions)).gap(10).wrap().align_items(Align::center).key("actions"));
        return Section("Step " + std::to_string(current_index + 1) + " of " + std::to_string(steps.size()) +
                           " · " +
                           (steps[current_index].title.empty() ? current_id : steps[current_index].title),
                       FlexColumn(std::move(content)).gap(10))
            .accessible_label("Wizard navigation");
    }
};
Wizard::Wizard(Binding<std::string> current, std::vector<WizardStep> steps, WizardOptions options)
    : impl_(std::make_shared<Impl>(std::move(current), std::move(steps), std::move(options))) {}
std::size_t Wizard::index() const {
    return impl_->index();
}
std::string Wizard::current() const {
    return impl_->steps[index()].id;
}
bool Wizard::is_last() const {
    return index() + 1 == impl_->steps.size();
}
bool Wizard::can_back() const {
    return impl_->options.allow_back && index() > 0;
}
std::string Wizard::validation_message() const {
    return impl_->error(index());
}
bool Wizard::can_advance() const {
    const auto current = index();
    return impl_->error(current).empty() &&
           (current + 1 < impl_->steps.size() || bool(impl_->options.on_finish));
}
bool Wizard::back() const {
    const auto current = index();
    return impl_->options.allow_back && current > 0 && impl_->transition(current - 1);
}
bool Wizard::next() const {
    const auto current = index();
    return current + 1 < impl_->steps.size() && impl_->error(current).empty() &&
           impl_->transition(current + 1);
}
bool Wizard::finish() const {
    const auto current = index();
    if (current + 1 != impl_->steps.size() || !impl_->options.on_finish || !impl_->error(current).empty())
        return false;
    auto finish = impl_->options.on_finish;
    finish();
    return true;
}
bool Wizard::go_to(const std::string& target) const {
    const auto current = index();
    const auto found = std::find_if(impl_->steps.begin(), impl_->steps.end(),
                                    [&](const auto& step) { return step.id == target; });
    if (found == impl_->steps.end())
        return false;
    const auto next = std::size_t(found - impl_->steps.begin());
    return impl_->options.allow_back && impl_->options.review_previous && next < current &&
           impl_->transition(next);
}
bool Wizard::cancel() const {
    if (!impl_->options.on_cancel)
        return false;
    auto cancel = impl_->options.on_cancel;
    cancel();
    return true;
}
WizardNavigation::WizardNavigation(Wizard wizard) : View(wizard.impl_->body(wizard)) {}
WizardNavigation::WizardNavigation(Binding<std::string> current, std::vector<WizardStep> steps,
                                   WizardOptions options)
    : WizardNavigation(Wizard(std::move(current), std::move(steps), std::move(options))) {}
} // namespace scars
