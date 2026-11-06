//! Retained property drafts and validated per-node patch editing.
use super::*;
use crate::{Binding, Draft, Style};
use overrides::Page;

fn source(nodes: &State<Vec<Inspection>>, id: NodeId) -> Result<Style, Error> {
    nodes
        .with(|nodes| nodes.iter().find(|n| n.id == id).map(|n| n.source_style))
        .ok_or_else(|| Error("The inspected node is no longer mounted".into()))
}
impl Inspector {
    pub fn style_override(&self, id: NodeId) -> StyleOverride {
        self.overrides
            .with(|m| m.get(&id).copied().unwrap_or_default())
    }
    /// Replace this node's patch. Invalid patches leave the previous patch intact.
    /// Returns false for a node outside the captured source hierarchy.
    pub fn set_override(&self, id: NodeId, patch: StyleOverride) -> Result<bool, Error> {
        let Ok(source) = source(&self.data.nodes, id) else {
            return Ok(false);
        };
        crate::host::validate_style(patch.apply(source))?;
        self.overrides.update(|m| {
            if patch.is_empty() {
                m.remove(&id);
            } else {
                m.insert(id, patch);
            }
        });
        Ok(true)
    }
    pub fn reset_override(&self, id: NodeId) {
        crate::state::batch(|| {
            self.overrides.update(|m| {
                m.remove(&id);
            });
            for ((node, _), draft) in self.data.editors.borrow().iter() {
                if *node == id {
                    draft.cancel();
                }
            }
        });
    }
    pub fn reset_property(&self, id: NodeId, property: StyleProperty) {
        crate::state::batch(|| {
            self.overrides.update(|m| {
                if let Some(patch) = m.get_mut(&id) {
                    patch.clear(property);
                    if patch.is_empty() {
                        m.remove(&id);
                    }
                }
            });
            if let Some(draft) = self.data.editors.borrow().get(&(id, property)) {
                draft.cancel();
            }
        });
    }
    pub fn reset_all_overrides(&self) {
        crate::state::batch(|| {
            self.overrides.set(HashMap::new());
            for draft in self.data.editors.borrow().values() {
                draft.cancel();
            }
        });
    }
    /// Choose the inspector's details or property page without changing source selection.
    pub fn show_page(&self, page: InspectorPage) {
        self.data.page.set(
            match page {
                InspectorPage::Details => "details",
                InspectorPage::Layout => "layout",
                InspectorPage::Style => "style",
            }
            .into(),
        );
    }
    fn property_draft(&self, id: NodeId, property: StyleProperty) -> Draft<String> {
        let mut editors = self.data.editors.borrow_mut();
        // One selected node's drafts are sufficient; edits never migrate to another node.
        editors.retain(|(node, _), _| *node == id);
        editors
            .entry((id, property))
            .or_insert_with(|| {
                let nodes = self.data.nodes.clone();
                let patches = self.overrides.clone();
                let (read_nodes, read_patches) = (nodes.clone(), patches.clone());
                let (parse_nodes, parse_patches) = (nodes.clone(), patches.clone());
                Draft::new(
                    Binding::new(
                        move || {
                            let base = source(&read_nodes, id).unwrap_or_default();
                            let patch =
                                read_patches.with(|m| m.get(&id).copied().unwrap_or_default());
                            property.display(patch.apply(base))
                        },
                        move |text: String| {
                            let Ok(base) = source(&nodes, id) else { return };
                            let patch = patches.with(|m| m.get(&id).copied().unwrap_or_default());
                            if let Ok(patch) = property.parse(&text, patch, base) {
                                patches.update(|m| {
                                    m.insert(id, patch);
                                });
                            }
                        },
                    ),
                    move |text| {
                        let base = source(&parse_nodes, id).map_err(|e| e.0)?;
                        let patch = parse_patches.with(|m| m.get(&id).copied().unwrap_or_default());
                        property
                            .parse(text, patch, base)
                            .map(|patch| property.display(patch.apply(base)))
                            .map_err(|e| e.0)
                    },
                    Clone::clone,
                )
            })
            .clone()
    }
    pub(super) fn property_view(&self, info: &Inspection) -> View {
        let page = if self.data.page.get() == "style" {
            Page::Style
        } else {
            Page::Layout
        };
        let patch = self.style_override(info.id);
        let mut rows = vec![View::label("Enter or Apply commits. Reset follows the latest application value. Optional values accept inherit.").wrap(true)];
        if let Some(error) = &info.override_error {
            rows.push(
                View::label(format!("Override suspended: {error}"))
                    .wrap(true)
                    .label_style(crate::LabelStyle::Error),
            );
        }
        for property in StyleProperty::ALL
            .iter()
            .copied()
            .filter(|p| p.page() == page)
        {
            let name = property.name();
            let draft = self.property_draft(info.id, property);
            let validation = draft.validation();
            let (submit, cancel, apply, reset) =
                (draft.clone(), draft.clone(), draft.clone(), self.clone());
            let id = info.id;
            let mut fields = vec![
                View::row(vec![
                    View::label(format!(
                        "{name}{}",
                        if patch.contains(property) {
                            " · overridden"
                        } else {
                            " · source"
                        }
                    ))
                    .grow(1.),
                    View::button("Reset", move || reset.reset_property(id, property))
                        .accessible_label(format!("Reset {name}"))
                        .value_cancel()
                        .enabled(patch.contains(property) || validation.dirty),
                ]),
                View::row(vec![
                    View::text_field(property.syntax(), draft.binding())
                        .key(format!("edit-{name}"))
                        .grow(1.)
                        .min_width(0.)
                        .accessible_label(format!("Override {name}"))
                        .accessible_help(format!(
                            "{}; optional values also accept inherit",
                            property.syntax()
                        ))
                        .validation(validation.clone())
                        .on_submit(move || {
                            submit.commit();
                        })
                        .on_cancel(move || cancel.cancel()),
                    View::button("Apply", move || {
                        apply.commit();
                    })
                    .accessible_label(format!("Apply {name}"))
                    .enabled(
                        validation.dirty && validation.status == crate::ValidationStatus::Valid,
                    ),
                ]),
                View::label(format!("Source: {}", property.display(info.source_style)))
                    .label_style(crate::LabelStyle::Secondary)
                    .wrap(true),
                View::label(property.syntax())
                    .label_style(crate::LabelStyle::Micro)
                    .wrap(true),
            ];
            if let Some(error) = draft.error() {
                fields.push(
                    View::label(error)
                        .wrap(true)
                        .label_style(crate::LabelStyle::Error),
                );
            }
            rows.push(View::column(fields).gap(4.).key(name));
        }
        View::column(rows).gap(14.)
    }
}
