//! Keyed visited pages retain host-owned editing, layout and focus state.
use crate::{Binding, Error, Host, Key, KeyEvent, Kind, NodeId, State, View};
use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};
#[derive(Clone)]
pub struct Tab {
    pub id: String,
    pub label: String,
    pub enabled: bool,
    content: Rc<dyn Fn() -> View>,
}
impl Tab {
    pub fn new(id: impl Into<String>, label: impl Into<String>, content: View) -> Self {
        Self::lazy(id, label, move || content.clone())
    }
    /// Builds on first selection, then rebuilds visited pages with the host.
    pub fn lazy(
        id: impl Into<String>,
        label: impl Into<String>,
        content: impl Fn() -> View + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            enabled: true,
            content: Rc::new(content),
        }
    }
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}
#[derive(Clone)]
pub struct Tabs {
    selection: Binding<String>,
}
impl Tabs {
    pub fn new(selected: impl Into<String>) -> Self {
        Self::bound(State::new(selected.into()).binding())
    }
    pub fn bound(selection: Binding<String>) -> Self {
        Self { selection }
    }
    /// The requested selection; `Host::tab_info` also exposes the effective page.
    pub fn selected(&self) -> String {
        self.selection.get()
    }
    pub fn select(&self, id: impl Into<String>) {
        self.selection.set(id.into());
    }
    pub fn view(&self, pages: Vec<Tab>) -> View {
        let mut view = View::new(Kind::Tabs, String::new(), vec![]).gap(8.);
        Rc::make_mut(&mut view.0).tabs = Some(TabsData {
            selection: self.selection.clone(),
            pages,
            header: true,
        });
        view
    }
}
pub(crate) fn page_stack(selection: Binding<String>, pages: Vec<Tab>) -> View {
    let mut view = View::new(Kind::PageStack, String::new(), vec![]).gap(0.);
    Rc::make_mut(&mut view.0).tabs = Some(TabsData {
        selection,
        pages,
        header: false,
    });
    view
}
pub fn tab_view(model: &Tabs, pages: Vec<Tab>) -> View {
    model.view(pages)
}
/// Existing tuple descriptions now use the retained implementation.
pub fn tabs(selected: &State<String>, pages: Vec<(String, String, View)>) -> View {
    Tabs::bound(selected.binding()).view(
        pages
            .into_iter()
            .map(|(id, label, view)| Tab::new(id, label, view))
            .collect(),
    )
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabItemInfo {
    pub id: String,
    pub label: String,
    pub enabled: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabsInfo {
    pub requested: String,
    pub selected: Option<String>,
    pub items: Vec<TabItemInfo>,
    pub visited: usize,
}
#[derive(Clone)]
pub(crate) struct TabsData {
    pub selection: Binding<String>,
    pub pages: Vec<Tab>,
    pub header: bool,
}
impl TabsData {
    pub fn info(&self) -> TabsInfo {
        let requested = self.selection.get();
        let selected = self
            .pages
            .iter()
            .find(|p| p.enabled && p.id == requested)
            .or_else(|| self.pages.iter().find(|p| p.enabled))
            .map(|p| p.id.clone());
        TabsInfo {
            requested,
            selected,
            items: self
                .pages
                .iter()
                .map(|p| TabItemInfo {
                    id: p.id.clone(),
                    label: p.label.clone(),
                    enabled: p.enabled,
                })
                .collect(),
            visited: 0,
        }
    }
    pub fn validate(&self) -> Result<(), Error> {
        let mut ids = HashSet::new();
        for page in &self.pages {
            if page.id.is_empty() || !ids.insert(&page.id) {
                return Err(Error("Tab IDs must be nonempty and unique".into()));
            }
        }
        Ok(())
    }
    pub fn children(&self, info: &mut TabsInfo, retained: Option<&TabsRuntime>) -> Vec<View> {
        let headers = self
            .pages
            .iter()
            .filter(|_| self.header)
            .map(|p| {
                let selected = info.selected.as_ref() == Some(&p.id);
                let mut header = View::new(Kind::Tab, p.label.clone(), vec![])
                    .key(&p.id)
                    .enabled(p.enabled)
                    .shrink(0.)
                    .text_overflow(crate::TextOverflow::Ellipsis);
                let d = Rc::make_mut(&mut header.0);
                d.tab_id = Some(p.id.clone());
                d.data = crate::view::Data::Bool(Binding::new(move || selected, |_| {}));
                header
            })
            .collect();
        let mut strip = View::scroll(vec![View::row(headers).gap(2.).key("$headers")])
            .scroll_axes(crate::ScrollAxes::Horizontal)
            .scrollbars(false)
            .key("$strip")
            .shrink(0.);
        if !self.header {
            strip = strip.hidden(true).height(0);
        }
        Rc::make_mut(&mut strip.0).tab_strip = true;
        let mut children = vec![strip];
        for page in &self.pages {
            let active = info.selected.as_ref() == Some(&page.id);
            if active || retained.is_some_and(|r| r.pages.contains_key(&page.id)) {
                let mut panel = View::new(Kind::TabPanel, String::new(), vec![(page.content)()])
                    .key(format!("$page-{}", page.id))
                    .accessible_label(&page.label)
                    .hidden(!active);
                Rc::make_mut(&mut panel.0).tab_id = Some(page.id.clone());
                children.push(panel);
            }
        }
        info.visited = children.len() - 1;
        children
    }
}
#[derive(Clone, Default)]
pub(crate) struct TabsRuntime {
    pub pages: HashMap<String, Option<NodeId>>,
    pub selected: Option<String>,
    pub had_focus: bool,
    pub pending_header: bool,
    pub strip_width: f64,
    pub strip_content: f64,
    pub header_items: Vec<TabItemInfo>,
}
impl Host {
    fn tab_owner(&self, mut id: NodeId) -> Option<NodeId> {
        let m = self.0.mounted.borrow();
        for _ in 0..m.nodes.len() {
            let n = m.nodes.iter().find(|n| n.id == id)?;
            if matches!(n.kind, Kind::Tabs | Kind::PageStack) {
                return Some(id);
            }
            if n.kind == Kind::Modal {
                return None;
            }
            id = n.parent?;
        }
        None
    }
    pub fn tab_info(&self, id: NodeId) -> Result<Option<TabsInfo>, Error> {
        self.refresh()?;
        let owner = self.tab_owner(id);
        Ok(self
            .0
            .mounted
            .borrow()
            .nodes
            .iter()
            .find(|n| Some(n.id) == owner)
            .and_then(|n| n.tabs.clone()))
    }
    pub(crate) fn remember_tab_focus(&self, focus: Option<NodeId>) {
        let m = self.0.mounted.borrow();
        let mut cache = self.0.tabs.borrow_mut();
        for (root, runtime) in cache.iter_mut() {
            let owner = m
                .nodes
                .iter()
                .find(|n| n.id == *root && n.kind == Kind::PageStack)
                .and_then(|n| n.parent)
                .filter(|id| {
                    m.nodes
                        .iter()
                        .any(|n| n.id == *id && n.kind == Kind::Wizard)
                })
                .unwrap_or(*root);
            runtime.had_focus = focus.is_some_and(|id| Self::within(&m.nodes, id, owner));
        }
        let Some(mut id) = focus else { return };
        for _ in 0..m.nodes.len() {
            let Some(n) = m.nodes.iter().find(|n| n.id == id) else {
                break;
            };
            if n.kind == Kind::Modal {
                break;
            }
            if n.kind == Kind::TabPanel
                && let (Some(root), Some(page)) = (n.parent, n.tab_id.as_ref())
                && let Some(r) = cache.get_mut(&root)
            {
                r.pages.insert(page.clone(), focus);
            }
            let Some(parent) = n.parent else { break };
            id = parent;
        }
    }
    /// Validate and activate an enabled tab, committing current preedit first.
    /// Availability is checked again after that commit's application callbacks.
    pub fn select_tab(&self, id: NodeId, page: &str) -> Result<bool, Error> {
        self.refresh()?;
        let Some(owner) = self.tab_owner(id) else {
            return Ok(false);
        };
        if !self
            .0
            .mounted
            .borrow()
            .nodes
            .iter()
            .any(|n| n.id == owner && n.kind == Kind::Tabs)
        {
            return Ok(false);
        }
        let valid = || {
            let m = self.0.mounted.borrow();
            m.nodes.iter().any(|n| {
                n.id == owner
                    && n.enabled
                    && !n.hidden
                    && !n.layout_hidden
                    && self.in_scope(owner)
                    && n.tabs
                        .as_ref()
                        .is_some_and(|t| t.items.iter().any(|p| p.id == page && p.enabled))
            })
        };
        if !valid() {
            return Ok(false);
        }
        self.unmark_text(true)?;
        self.refresh()?;
        if !valid() {
            return Ok(true);
        }
        let data = self
            .0
            .mounted
            .borrow()
            .descriptions
            .get(&owner)
            .and_then(|d| d.tabs.clone());
        if let Some(data) = data {
            self.0
                .tabs
                .borrow_mut()
                .entry(owner)
                .or_default()
                .pending_header = true;
            data.selection.set(page.to_owned());
            self.redraw();
            return Ok(true);
        }
        Ok(false)
    }
    fn selected_header(nodes: &[crate::Node], root: NodeId) -> Option<NodeId> {
        let selected = nodes
            .iter()
            .find(|n| n.id == root)?
            .tabs
            .as_ref()?
            .selected
            .as_ref()?;
        nodes
            .iter()
            .find(|n| {
                n.kind == Kind::Tab
                    && !n.hidden
                    && !n.layout_hidden
                    && n.tab_id.as_ref() == Some(selected)
                    && Self::within(nodes, n.id, root)
            })
            .map(|n| n.id)
    }
    fn page_focus(&self, root: NodeId) -> Option<NodeId> {
        let m = self.0.mounted.borrow();
        let group = m.nodes.iter().find(|n| n.id == root)?;
        let selected = group.tabs.as_ref()?.selected.as_ref()?;
        let page = m.nodes.iter().find(|n| {
            n.kind == Kind::TabPanel
                && n.parent == Some(root)
                && n.tab_id.as_ref() == Some(selected)
        })?;
        let remembered = self
            .0
            .tabs
            .borrow()
            .get(&root)
            .and_then(|r| r.pages.get(selected).copied().flatten());
        remembered
            .filter(|id| {
                m.nodes.iter().any(|n| {
                    n.id == *id && n.enabled && !n.hidden && !n.layout_hidden && self.in_scope(*id)
                })
            })
            .or_else(|| {
                m.nodes
                    .iter()
                    .find(|n| {
                        n.enabled
                            && !n.hidden
                            && !n.layout_hidden
                            && crate::host::focusable(n.kind)
                            && Self::within(&m.nodes, n.id, page.id)
                            && self.in_scope(n.id)
                    })
                    .map(|n| n.id)
            })
            .or_else(|| {
                let wizard = group.parent.filter(|id| {
                    m.nodes
                        .iter()
                        .any(|n| n.id == *id && n.kind == Kind::Wizard)
                })?;
                let actions = m
                    .nodes
                    .iter()
                    .find(|n| n.parent == Some(wizard) && n.key.as_deref() == Some("$actions"))?;
                m.nodes
                    .iter()
                    .find(|n| {
                        n.parent == Some(actions.id)
                            && n.enabled
                            && !n.hidden
                            && !n.layout_hidden
                            && m.descriptions[&n.id]
                                .wizard_action
                                .as_ref()
                                .is_some_and(|a| a.is_advance())
                    })
                    .map(|n| n.id)
            })
    }
    /// Focus the active page's last control, falling back to its first control.
    pub fn focus_tab_page(&self, id: NodeId) -> Result<bool, Error> {
        self.refresh()?;
        let Some(root) = self.tab_owner(id) else {
            return Ok(false);
        };
        match self.page_focus(root) {
            Some(id) => self.focus(id),
            None => Ok(false),
        }
    }
    pub(crate) fn tab_key(&self, event: KeyEvent) -> Result<Option<bool>, Error> {
        let Some(focused) = self.focused() else {
            return Ok(None);
        };
        let Some(root) = self.tab_owner(focused) else {
            return Ok(None);
        };
        if self
            .0
            .mounted
            .borrow()
            .nodes
            .iter()
            .any(|n| n.id == root && n.kind == Kind::PageStack)
        {
            return Ok(None);
        }
        if event.key == Key::Tab
            && !event.modifiers.control
            && !event.modifiers.shift
            && self
                .0
                .mounted
                .borrow()
                .nodes
                .iter()
                .any(|n| n.id == focused && n.kind == Kind::Tab)
        {
            return Ok(Some(if self.page_focus(root).is_some() {
                self.focus_tab_page(root)?
            } else {
                self.focus_next(false)?
            }));
        }
        let header = self
            .0
            .mounted
            .borrow()
            .nodes
            .iter()
            .any(|n| n.id == focused && n.kind == Kind::Tab);
        let cycle = event.key == Key::Tab && event.modifiers.control;
        if !cycle
            && !(header
                && matches!(
                    event.key,
                    Key::Left | Key::Right | Key::Up | Key::Down | Key::Home | Key::End
                ))
        {
            return Ok(None);
        }
        let info = self.tab_info(root)?.unwrap();
        let items: Vec<_> = info.items.iter().filter(|p| p.enabled).collect();
        if items.is_empty() {
            return Ok(Some(false));
        }
        let focused_page = if header && !cycle {
            self.0
                .mounted
                .borrow()
                .nodes
                .iter()
                .find(|n| n.id == focused)
                .and_then(|n| n.tab_id.clone())
        } else {
            None
        };
        let current = focused_page
            .as_ref()
            .or(info.selected.as_ref())
            .and_then(|id| items.iter().position(|p| &p.id == id))
            .unwrap_or(0);
        let next = match event.key {
            Key::Home => 0,
            Key::End => items.len() - 1,
            Key::Left | Key::Up => (current + items.len() - 1) % items.len(),
            Key::Tab if event.modifiers.shift => (current + items.len() - 1) % items.len(),
            _ => (current + 1) % items.len(),
        };
        Ok(Some(self.select_tab(root, &items[next].id)?))
    }
    pub(crate) fn reconcile_tabs(&self) -> Result<(), Error> {
        let nodes = self.0.mounted.borrow().nodes.clone();
        let groups: Vec<_> = nodes
            .iter()
            .filter(|n| matches!(n.kind, Kind::Tabs | Kind::PageStack))
            .collect();
        for group in groups.iter().rev() {
            let info = group.tabs.as_ref().unwrap();
            if group.hidden
                || group.layout_hidden
                || !group.enabled
                || !self.in_scope(group.id)
                || !self.0.interaction.borrow().active
            {
                continue;
            }
            let strip = nodes
                .iter()
                .find(|n| n.parent == Some(group.id) && n.tab_strip);
            let strip_width = strip.map_or(0., |n| n.bounds.width);
            let strip_content = strip.and_then(|n| n.scroll).map_or(0., |s| s.content.width);
            let (changed, pending, had_focus, resized) = {
                let mut cache = self.0.tabs.borrow_mut();
                let runtime = cache.entry(group.id).or_default();
                let changed = runtime.selected != info.selected;
                runtime.selected = info.selected.clone();
                let resized = runtime.strip_width != strip_width
                    || (runtime.strip_content - strip_content).abs() > 0.01
                    || runtime.header_items != info.items;
                runtime.strip_width = strip_width;
                runtime.strip_content = strip_content;
                runtime.header_items = info.items.clone();
                (
                    changed,
                    std::mem::take(&mut runtime.pending_header),
                    runtime.had_focus,
                    resized,
                )
            };
            if pending || changed && had_focus {
                let fallback = info
                    .selected
                    .as_ref()
                    .is_none_or(|id| *id != info.requested);
                let target = if pending || fallback {
                    Self::selected_header(&nodes, group.id)
                } else {
                    self.page_focus(group.id)
                        .or_else(|| Self::selected_header(&nodes, group.id))
                };
                if let Some(id) = target {
                    self.focus(id)?;
                }
            }
            let reveal = if resized || changed {
                self.focused()
                    .filter(|id| {
                        self.tab_owner(*id) == Some(group.id)
                            && nodes.iter().any(|n| n.id == *id && n.kind == Kind::Tab)
                    })
                    .or_else(|| Self::selected_header(&nodes, group.id))
            } else {
                None
            };
            if let Some(n) = reveal.and_then(|id| nodes.iter().find(|n| n.id == id))
                && let Some(strip) = strip.and_then(|n| n.scroll.map(|s| (n.id, s)))
            {
                let (strip_id, info) = strip;
                let dx = if n.bounds.x < info.viewport.x {
                    n.bounds.x - info.viewport.x
                } else {
                    (n.bounds.x + n.bounds.width - info.viewport.x - info.viewport.width).max(0.)
                };
                self.scroll_to(
                    strip_id,
                    crate::Point {
                        x: info.offset.x + dx,
                        y: info.offset.y,
                    },
                )?;
            }
        }
        Ok(())
    }
}
