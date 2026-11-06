//! Owned, bounded inspector data. Never retain live models, providers or callbacks.
use crate::{
    Kind, Node, NodeId, OverlayOptions, Point, Rect, ScrollInfo, Selection, Size, Style,
    StyleProperty, TextFieldKind, TextStyle, Validation, Value,
};
use std::ops::Range;
#[derive(Clone, Debug, PartialEq)]
pub struct InspectionFlexLine {
    pub bounds: Rect,
    pub items: Vec<NodeId>,
    pub gap: f64,
    /// Signed space after flex allocation, before justification; negative means overflow.
    pub free: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct InspectionCollection {
    pub total: usize,
    pub rows: usize,
    /// Realized viewport window, including overscan; pinned rows may lie outside it.
    pub window: Range<usize>,
    pub realized: usize,
    pub pinned: usize,
    pub selected: usize,
    pub cursor: Option<String>,
    pub columns: usize,
    pub offset: f64,
    pub viewport: f64,
    pub row_height: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct InspectionOverlay {
    pub owner: Option<NodeId>,
    pub anchor: Option<String>,
    pub point: Option<Point>,
    pub options: Option<OverlayOptions>,
    pub active: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Inspection {
    pub id: NodeId,
    pub parent: Option<NodeId>,
    pub depth: usize,
    pub kind: Kind,
    pub key: Option<String>,
    /// Text previews are bounded to 256 Unicode scalar values.
    pub text: String,
    pub label: String,
    pub value: String,
    pub bounds: Rect,
    pub clip: Rect,
    pub text_bounds: Rect,
    pub baseline: f64,
    pub measured: Size,
    pub content_bounds: Rect,
    pub text_style: TextStyle,
    pub source_style: Style,
    pub overridden: Vec<StyleProperty>,
    pub override_error: Option<String>,
    pub flex_lines: Vec<InspectionFlexLine>,
    pub style: Style,
    pub enabled: bool,
    pub hidden: bool,
    pub read_only: bool,
    pub focused: bool,
    pub pressed: bool,
    pub in_scope: bool,
    pub selected: Option<bool>,
    pub text_selection: Option<Selection>,
    pub marked: bool,
    pub validation: Option<Validation>,
    pub scroll: Option<ScrollInfo>,
    pub overlay: Option<InspectionOverlay>,
    pub collection: Option<InspectionCollection>,
    pub details: Vec<String>,
}
pub(super) fn preview(text: &str) -> String {
    let mut chars = text.chars();
    let mut out: String = chars.by_ref().take(256).collect();
    if chars.next().is_some() {
        out.push('…');
    }
    out
}
impl Inspection {
    pub(super) fn from_node(n: &Node, nodes: &[Node]) -> Self {
        let password = n.secure || n.field_kind == TextFieldKind::Password;
        let value = if password {
            "Hidden password".into()
        } else {
            match &n.value {
                Value::None => String::new(),
                Value::Text(s) => preview(s),
                Value::Bool(v) => v.to_string(),
                Value::Number(v) => v.to_string(),
                Value::Check(v) => format!("{v:?}"),
            }
        };
        let table = n.table_root.as_ref().or(n.table_model.as_ref());
        let list = n.collection.as_ref().or_else(|| {
            n.table_root.as_ref().and_then(|t| {
                nodes
                    .iter()
                    .find(|n| n.table_model.as_ref() == Some(t) && n.collection.is_some())
                    .and_then(|n| n.collection.as_ref())
            })
        });
        let collection = list.map(|m| InspectionCollection {
            total: n.tree_model.as_ref().map_or(m.len(), |t| t.known_count()),
            rows: m.len(),
            window: m.visible_range(),
            realized: m.realized_indices().len(),
            pinned: m.pinned_keys().len(),
            selected: table.map_or_else(|| m.selection_count(), |t| t.selection_count()),
            cursor: table
                .and_then(|t| t.cursor())
                .map(|c| format!("{} / {}", c.row, c.column))
                .or_else(|| m.cursor()),
            columns: table.map_or(1, |t| t.snapshot().column_count()),
            offset: m.offset(),
            viewport: m.viewport_height(),
            row_height: m.row_height(),
        });
        let mut details = vec![];
        if let Some(t) = &n.tabs {
            details.push(format!("Tab: {:?}; {} visited", t.selected, t.visited));
        }
        if let Some(t) = &n.tree_item {
            details.push(format!(
                "Tree: level {}, expanded {}, {:?}",
                t.depth, t.expanded, t.load_state
            ));
        }
        if let Some(c) = &n.combo {
            details.push(format!(
                "Combo: {:?}; {} matches; open {}",
                c.selection, c.matches, c.open
            ));
        }
        if let Some(c) = &n.calendar {
            details.push(format!(
                "Calendar: {}; cursor {}; selected {}",
                c.month, c.cursor, c.selected
            ));
        }
        if let Some(s) = &n.stepper {
            details.push(format!(
                "Stepper: {}; {}–{}; decrement {}, increment {}",
                s.value, s.min, s.max, s.can_decrement, s.can_increment
            ));
        }
        if let Some((min, max, step)) = n.range {
            details.push(format!("Range: {min}–{max}, step {step}"));
        }
        if let Some(s) = &n.split {
            details.push(format!(
                "Split: {:?}; ratio {:.3}; {:?}",
                s.orientation, s.effective_ratio, s.collapsed
            ));
        }
        if let Some(c) = &n.color_picker {
            details.push(format!(
                "Color: {:?}; alpha {}; live preview {}",
                c.model, c.alpha, c.live_preview
            ));
        }
        if let Some(m) = &n.menu {
            details.push(format!("Menu: expanded {}; path {:?}", m.expanded, m.path));
        }
        if let Some(e) = &n.table_edit_error {
            details.push(format!("Table edit: {}", preview(e)));
        }
        let selected = n
            .table_cell
            .as_ref()
            .and_then(|cell| table.and_then(|t| t.cell_info(cell)).map(|c| c.selected))
            .or_else(|| {
                matches!(n.value, Value::Bool(_) | Value::Check(_)).then(|| n.value.selected())
            });
        Self {
            id: n.id,
            parent: n.parent,
            depth: n.depth,
            kind: n.kind,
            key: n.key.clone(),
            text: if password {
                String::new()
            } else {
                preview(&n.text)
            },
            label: preview(&n.label),
            value,
            bounds: n.bounds,
            clip: n.clip,
            text_bounds: n.text_bounds,
            baseline: n.baseline,
            measured: n.measured,
            content_bounds: n.bounds.inset(n.style.padding),
            text_style: n.text_style,
            source_style: n.style,
            overridden: vec![],
            override_error: None,
            flex_lines: n.flex_lines.clone(),
            style: n.style,
            enabled: n.enabled,
            hidden: n.hidden || n.layout_hidden,
            read_only: n.read_only,
            focused: n.focused,
            pressed: n.pressed,
            in_scope: true,
            selected,
            text_selection: None,
            marked: false,
            validation: n.validation.clone(),
            scroll: n.scroll,
            overlay: (n.overlay.is_some() || n.kind == Kind::Modal).then(|| InspectionOverlay {
                owner: n.overlay_owner,
                anchor: n.anchor.clone(),
                point: n.overlay_point,
                options: n.overlay,
                active: false,
            }),
            collection,
            details,
        }
    }
    pub(super) fn outline_label(&self) -> String {
        let name = self.key.as_deref().unwrap_or(&self.label);
        let name: String = name
            .chars()
            .take(48)
            .map(|c| if c == '\n' { '↵' } else { c })
            .collect();
        format!(
            "{:?} #{}{}{}{}",
            self.kind,
            self.id,
            if name.is_empty() {
                String::new()
            } else {
                format!(" · {name}")
            },
            if self.hidden { " [hidden]" } else { "" },
            if !self.enabled { " [disabled]" } else { "" }
        )
    }
}
impl From<Node> for Inspection {
    fn from(n: Node) -> Self {
        Self::from_node(&n, &[])
    }
}
