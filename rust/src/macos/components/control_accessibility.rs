//! Control-specific state and guarded actions, shared by native attribute handlers.
use super::{
    accessibility::{Element, element},
    *,
};
use ui::{Kind, Node, Value};
pub(super) fn readable(m: &Mounted, n: &Node, nodes: &[Node]) -> bool {
    m.host.in_scope(n.id)
        || n.menu.as_ref().filter(|i| i.trigger).is_some_and(|i| {
            m.host
                .active_modal()
                .and_then(|id| nodes.iter().find(|n| n.id == id))
                .and_then(|n| n.menu.as_ref())
                .is_some_and(|active| active.id == i.id)
        })
}
pub(super) fn can_cancel(m: &Mounted, n: &Node) -> bool {
    let Some(scope) = m.host.active_modal() else {
        return false;
    };
    if !n.enabled
        || !m.host.in_scope(n.id)
        || !(n.id == scope || n.menu_panel || n.menu_item.is_some())
    {
        return false;
    }
    let nodes = m.host.snapshot().unwrap_or_default();
    let Some(root) = nodes.iter().find(|n| n.id == scope) else {
        return false;
    };
    root.menu.as_ref().is_some_and(|m| !m.path.is_empty())
        || root.overlay.is_some_and(|o| o.dismiss_on_escape)
}
pub(super) fn can_adjust(n: &Node, increase: bool) -> bool {
    if !n.enabled || n.read_only {
        return false;
    }
    if let Some(s) = &n.stepper {
        return if increase {
            s.can_increment
        } else {
            s.can_decrement
        };
    }
    if !matches!(n.kind, Kind::Slider | Kind::SplitDivider) {
        return false;
    }
    match (&n.value, n.range) {
        (Value::Number(v), Some((min, max, _))) => {
            if increase {
                *v < max
            } else {
                *v > min
            }
        }
        _ => false,
    }
}
pub(super) fn can_set(n: &Node) -> bool {
    n.enabled
        && !n.read_only
        && n.menu_item.is_none()
        && (n.table_cell.is_none() || n.table_editor.is_some())
        && matches!(
            n.kind,
            Kind::TextField
                | Kind::Slider
                | Kind::SplitDivider
                | Kind::Toggle
                | Kind::CheckBox
                | Kind::Radio
                | Kind::Segment
                | Kind::Tab
                | Kind::Switch
                | Kind::Tabs
                | Kind::RadioGroup
                | Kind::Segmented
        )
}
pub(super) fn help(n: &Node) -> Option<Retained<NSString>> {
    let mut parts = vec![];
    if let Some(message) = n
        .validation
        .as_ref()
        .and_then(|v| v.message.as_ref())
        .filter(|m| !m.is_empty())
    {
        parts.push(message.as_str())
    }
    if !n.help.is_empty() && !parts.contains(&n.help.as_str()) {
        parts.push(&n.help)
    }
    if let Some(item) = &n.menu_item
        && !item.shortcut.is_empty()
    {
        parts.push(&item.shortcut)
    }
    (!parts.is_empty()).then(|| ns(&parts.join(". ")))
}
pub(super) fn description(n: &Node) -> Option<Retained<NSString>> {
    if let Some(v) = &n.validation
        && v.status != ui::ValidationStatus::Valid
    {
        return Some(ns(match v.status {
            ui::ValidationStatus::Incomplete => "Incomplete value",
            ui::ValidationStatus::OutOfRange => "Value outside supported bounds",
            _ => "Invalid value",
        }));
    }
    if n.kind == Kind::SplitDivider {
        return n.split.map(|s| {
            ns(&match s.collapsed {
                ui::SplitPane::First => "First pane collapsed".into(),
                ui::SplitPane::Second => "Second pane collapsed".into(),
                ui::SplitPane::None => format!(
                    "{:.0} points",
                    if s.orientation == ui::Orientation::Horizontal {
                        s.first.width
                    } else {
                        s.first.height
                    }
                ),
            })
        });
    }
    if n.kind == Kind::Progress && n.value == Value::None {
        return Some(ns("In progress"));
    }
    None
}
impl Element {
    pub(super) fn control_array(&self, name: &str) -> Option<Retained<NSArray<AnyObject>>> {
        match name {
            "AXTabs" => self.accessibilityTabs(),
            "AXContents" => self.accessibilityContents(),
            "AXSplitters" => self.accessibilitySplitters(),
            "AXPreviousContents" => self.accessibilityPreviousContents(),
            "AXNextContents" => self.accessibilityNextContents(),
            "AXLinkedUIElements" => self.accessibilityLinkedUIElements(),
            "AXSelectedChildren"
                if self
                    .with(|_, _, n| Ok(n.collection.is_none()))
                    .unwrap_or(false) =>
            {
                self.accessibilitySelectedChildren()
            }
            _ => None,
        }
    }
    pub(super) fn control_range(
        &self,
        name: &str,
        start: usize,
        count: usize,
    ) -> Option<Retained<NSArray<AnyObject>>> {
        self.control_array(name).map(|array| {
            NSArray::from_retained_slice(&array.iter().skip(start).take(count).collect::<Vec<_>>())
        })
    }

    pub(super) fn set_control_value(&self, value: Option<&AnyObject>) {
        self.with(|m, _, n| {
            if !can_set(n) {
                return Ok(false);
            }
            if matches!(n.kind, Kind::Tabs | Kind::RadioGroup | Kind::Segmented) {
                return self.select_choice(value);
            }
            if let Some(text) = value.and_then(|v| v.downcast_ref::<NSString>()) {
                return if n.kind == Kind::TextField {
                    m.host.set_text(n.id, &text.to_string())
                } else {
                    Ok(false)
                };
            }
            let Some(number) = value.and_then(|v| v.downcast_ref::<NSNumber>()) else {
                return Ok(false);
            };
            if matches!(n.kind, Kind::Slider | Kind::SplitDivider) {
                return m.host.set_number(n.id, number.doubleValue());
            }
            if n.kind == Kind::CheckBox && matches!(n.value, Value::Check(_)) {
                let state = match number.intValue() {
                    0 => ui::CheckState::Unchecked,
                    1 => ui::CheckState::Checked,
                    2 => ui::CheckState::Indeterminate,
                    _ => return Ok(false),
                };
                m.host.set_check_state(n.id, state)
            } else {
                m.host.set_selected(n.id, number.boolValue())
            }
        });
    }
    fn select_choice(&self, value: Option<&AnyObject>) -> std::result::Result<bool, ui::Error> {
        Ok(self
            .with(|m, _, n| {
                if !n.enabled
                    || n.read_only
                    || !matches!(n.kind, Kind::Tabs | Kind::RadioGroup | Kind::Segmented)
                {
                    return Ok(false);
                }
                let Some(child) = value.and_then(|v| v.downcast_ref::<Element>()) else {
                    return Ok(false);
                };
                let Some(owner) = child.ivars().owner.upgrade() else {
                    return Ok(false);
                };
                if !std::ptr::eq(&*owner, m) {
                    return Ok(false);
                }
                let nodes = m.host.snapshot()?;
                let target = nodes.iter().find(|c| c.id == child.ivars().id);
                let valid = target.is_some_and(|c| {
                    if n.kind == Kind::Tabs {
                        super::accessibility::tab_group(c, &nodes) == Some(n.id)
                            && c.kind == Kind::Tab
                    } else {
                        c.parent == Some(n.id) && matches!(c.kind, Kind::Radio | Kind::Segment)
                    }
                });
                if valid {
                    m.host.set_selected(child.ivars().id, true)
                } else {
                    Ok(false)
                }
            })
            .unwrap_or(false))
    }
    pub(super) fn set_choice_children(&self, values: Option<&NSArray<AnyObject>>) {
        if let Some(values) = values
            && values.len() == 1
        {
            let _ = self.select_choice(Some(&values.objectAtIndex(0)));
        }
    }
    pub(super) fn choices(&self, selected: bool) -> Option<Retained<NSArray<AnyObject>>> {
        self.with(|m, _, n| {
            if !matches!(n.kind, Kind::RadioGroup | Kind::Segmented) {
                return Ok(None);
            }
            let values = m
                .host
                .snapshot()?
                .iter()
                .filter(|c| {
                    c.parent == Some(n.id) && !c.hidden && (!selected || c.value.selected())
                })
                .map(|c| unsafe { Retained::cast_unchecked(element(m, c.id)) })
                .collect::<Vec<Retained<AnyObject>>>();
            Ok(Some(NSArray::from_retained_slice(&values)))
        })
        .flatten()
    }
    pub(super) fn step_button(&self, increase: bool) -> Option<Retained<AnyObject>> {
        self.with(|m, _, n| {
            if n.kind != Kind::Stepper {
                return Ok(None);
            }
            let nodes = m.host.snapshot()?;
            let mut children = nodes
                .iter()
                .filter(|c| c.parent == Some(n.id) && c.kind == Kind::Button && !c.hidden);
            let node = if increase {
                children.nth(1)
            } else {
                children.next()
            };
            Ok(node.map(|n| unsafe { Retained::cast_unchecked(element(m, n.id)) }))
        })
        .flatten()
    }
    pub(super) fn confirm_control(&self) -> bool {
        self.with(|m, _, n| {
            if n.alert {
                let button = self.accessibilityDefaultButton();
                let target = button.as_deref().and_then(|b| b.downcast_ref::<Element>());
                return target.map_or(Ok(false), |e| m.host.activate(e.ivars().id));
            }
            if matches!(n.kind, Kind::Tab | Kind::Radio | Kind::Segment) {
                m.host.set_selected(n.id, true)
            } else {
                Ok(false)
            }
        })
        .unwrap_or(false)
    }
}
