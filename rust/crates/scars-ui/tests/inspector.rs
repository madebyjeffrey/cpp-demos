use scars_ui::*;
use std::rc::Rc;
struct Metrics;
impl Measure for Metrics {
    fn text(&self, s: &str, _: TextStyle, width: f64) -> TextLayout {
        TextLayout {
            size: Size::new((s.chars().count() as f64 * 7.).min(width), 18.),
            baseline: 14.,
            lines: vec![],
            placeholders: vec![],
            hit_test: Rc::new(|_| 0),
            range_rects: Rc::new(|_| vec![]),
        }
    }
    fn image(&self, _: &Bitmap) -> Result<Size, Error> {
        Ok(Size::default())
    }
}
fn layout(h: &Host) {
    h.layout(Size::new(1000., 900.), &Metrics).unwrap();
}
fn node(h: &Host, key: &str) -> Node {
    h.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.key.as_deref() == Some(key))
        .unwrap()
}
fn center(n: &Node) -> Point {
    Point {
        x: n.bounds.x + n.bounds.width / 2.,
        y: n.bounds.y + n.bounds.height / 2.,
    }
}
fn key(h: &Host, key: Key) {
    h.key(KeyEvent {
        key,
        down: true,
        repeat: false,
        modifiers: Default::default(),
    })
    .unwrap();
}
fn pointer(h: &Host, p: Point, kind: PointerKind) {
    h.pointer(Pointer {
        kind,
        position: p,
        button: 0,
        modifiers: Default::default(),
    })
    .unwrap();
}

#[test]
fn hierarchy_excludes_its_own_controls_and_picking_preserves_focus_and_preedit() {
    let inspector = Inspector::new();
    let text = State::new("Name".to_owned());
    let clicks = State::new(0);
    let h = Host::new({
        let (i, t, c) = (inspector.clone(), text.clone(), clicks.clone());
        move || {
            View::row(vec![
                View::column(vec![
                    View::text_field("Name", t.binding()).key("name"),
                    View::button("Save", c.action(|s| s.set(1))).key("save"),
                ])
                .grow(1.),
                i.view().width(380.).key("tools"),
            ])
        }
    });
    h.set_inspector(inspector.clone());
    layout(&h);
    let source = node(&h, "name");
    let save = node(&h, "save");
    let tools = node(&h, "tools");
    assert!(inspector.nodes().iter().any(|n| n.id == source.id));
    assert!(
        !inspector
            .nodes()
            .iter()
            .any(|n| n.id == tools.id || n.label == "Mounted hierarchy")
    );
    assert!(inspector.select(source.id));
    layout(&h);
    h.focus(source.id).unwrap();
    h.mark_text("preedit", 0..0, None).unwrap();
    inspector.picking.set(true);
    layout(&h);
    pointer(&h, center(&save), PointerKind::Down);
    pointer(&h, center(&save), PointerKind::Up);
    assert_eq!(inspector.selected.get().unwrap().id, save.id);
    assert_eq!(clicks.get(), 0);
    assert_eq!(h.focused(), Some(source.id));
    assert!(h.text_input().unwrap().unwrap().marked.is_some());
    assert_eq!(text.get(), "Name");
    layout(&h);
    assert_eq!(inspector.selected.get().unwrap().id, save.id);
    let pick = h
        .snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.label == "Pick a control · Escape exits")
        .unwrap();
    assert!(h.pick(center(&pick)).unwrap().is_none());
    pointer(&h, center(&pick), PointerKind::Down);
    layout(&h);
    pointer(&h, center(&pick), PointerKind::Up);
    layout(&h);
    assert!(
        !inspector.picking.get(),
        "inspector controls remain operable during picking"
    );
    pointer(&h, center(&save), PointerKind::Down);
    layout(&h);
    inspector.picking.set(true);
    layout(&h);
    pointer(&h, center(&save), PointerKind::Down);
    layout(&h);
    pointer(&h, center(&save), PointerKind::Up);
    layout(&h);
    assert_eq!(
        clicks.get(),
        0,
        "picking must retire prior pointer capture before release"
    );
}

#[test]
fn tree_navigation_picking_and_dynamic_labels_share_stable_rows_and_disclosure() {
    let inspector = Inspector::new();
    let label = State::new("Save".to_owned());
    let present = State::new(true);
    let h = Host::new({
        let (i, l, p) = (inspector.clone(), label.clone(), present.clone());
        move || {
            View::row(vec![
                View::column(if p.get() {
                    std::iter::once(View::button(l.get(), || {}))
                        .chain((0..30).map(|j| View::button(format!("Item {j}"), || {})))
                        .collect()
                } else {
                    vec![]
                })
                .key("body")
                .grow(1.),
                i.view().width(380.),
            ])
        }
    });
    h.set_inspector(inspector.clone());
    layout(&h);
    let target = h
        .snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.text == "Save")
        .unwrap()
        .id;
    let parent = node(&h, "body").id;
    assert!(inspector.select(target));
    layout(&h);
    let tree = h
        .snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.label == "Mounted hierarchy")
        .unwrap()
        .tree_model
        .unwrap();
    let row = node(&h, &target.to_string());
    let token = tree.item_token(&target.to_string()).unwrap();
    h.focus(row.id).unwrap();
    key(&h, Key::Left);
    layout(&h);
    assert_eq!(inspector.selected.get().unwrap().id, parent);
    key(&h, Key::Right);
    key(&h, Key::Right);
    layout(&h);
    assert_eq!(inspector.selected.get().unwrap().id, target);
    label.set("Saved".into());
    layout(&h);
    assert_eq!(node(&h, &target.to_string()).id, row.id);
    let last = inspector
        .nodes()
        .into_iter()
        .find(|n| n.text == "Item 29")
        .unwrap()
        .id;
    tree.reveal(&last.to_string()).unwrap();
    layout(&h);
    let offset = h
        .snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.label == "Mounted hierarchy")
        .unwrap()
        .collection
        .unwrap()
        .offset();
    assert!(offset > 0.);
    label.set("Saved again".into());
    layout(&h);
    assert_eq!(
        h.snapshot()
            .unwrap()
            .into_iter()
            .find(|n| n.label == "Mounted hierarchy")
            .unwrap()
            .collection
            .unwrap()
            .offset(),
        offset,
        "metadata refresh must not snap the browser back"
    );
    assert_eq!(tree.item_token(&target.to_string()), Some(token));
    assert!(
        tree.item(&target.to_string())
            .unwrap()
            .label
            .contains("Saved again")
    );
    assert_eq!(inspector.selected.get().unwrap().text, "Saved again");
    present.set(false);
    layout(&h);
    assert!(inspector.selected.get().is_none());
    assert_eq!(tree.selection_count(), 0);
    assert!(!inspector.nodes().iter().any(|n| n.id == target));
    layout(&h);
    assert!(
        inspector.selected.get().is_none(),
        "removed selection must not reappear from the tree cursor"
    );
}

#[test]
fn live_diagnostics_track_focus_selection_validation_clipping_and_mask_passwords() {
    let inspector = Inspector::new();
    let text = State::new("Hello".to_owned());
    let secret = State::new("do-not-cache-this".to_owned());
    let validation = State::new(Validation {
        status: ValidationStatus::Valid,
        message: None,
        dirty: false,
    });
    let hidden = State::new(false);
    let h = Host::new({
        let (t, s, v, hide) = (
            text.clone(),
            secret.clone(),
            validation.clone(),
            hidden.clone(),
        );
        move || {
            View::column(vec![
                View::scroll(vec![
                    View::text_field("Name", t.binding())
                        .validation(v.get())
                        .key("name")
                        .hidden(hide.get()),
                    View::password_field("Password", s.binding()).key("password"),
                ])
                .width(200.)
                .height(80.),
            ])
        }
    });
    h.set_inspector(inspector.clone());
    layout(&h);
    let input = node(&h, "name").id;
    h.focus(input).unwrap();
    inspector.select(input);
    h.select_text(
        input,
        Selection {
            anchor: 1,
            caret: 4,
        },
    )
    .unwrap();
    validation.set(Validation {
        status: ValidationStatus::Incomplete,
        message: Some("Finish the name".into()),
        dirty: true,
    });
    layout(&h);
    let info = inspector.selected.get().unwrap();
    assert!(info.focused && info.enabled && info.in_scope);
    assert_eq!(
        info.text_selection,
        Some(Selection {
            anchor: 1,
            caret: 4
        })
    );
    assert_eq!(
        info.validation.unwrap().status,
        ValidationStatus::Incomplete
    );
    assert!(info.clip.width <= 200.);
    let password = inspector
        .nodes()
        .into_iter()
        .find(|n| n.key.as_deref() == Some("password"))
        .unwrap();
    assert_eq!(password.value, "Hidden password");
    assert!(!format!("{password:?}").contains(&secret.get()));
    hidden.set(true);
    layout(&h);
    assert!(inspector.selected.get().unwrap().hidden);
    assert_eq!(
        inspector.nodes().iter().filter(|n| n.id == input).count(),
        1
    );
}

#[test]
fn collection_windows_are_bounded_and_snapshots_do_not_retain_application_models() {
    let inspector = Inspector::new();
    let retained = Rc::new(());
    let weak = Rc::downgrade(&retained);
    let list = ListModel::new(
        (0..100_000)
            .map(|i| ListItem::new(format!("r{i}"), format!("Row {i}")))
            .collect(),
        24.,
    )
    .on_activate(move |_| {
        let _ = &retained;
    });
    let pin = list.pin("r99999").unwrap();
    list.select("r500", false);
    list.reveal(500);
    let h = Host::new({
        let l = list.clone();
        move || View::column(vec![l.view().height(120.).key("rows")])
    });
    h.set_inspector(inspector.clone());
    layout(&h);
    let info = inspector
        .nodes()
        .into_iter()
        .find(|n| n.key.as_deref() == Some("rows"))
        .unwrap();
    let c = info.collection.unwrap();
    assert_eq!(c.total, 100_000);
    assert_eq!(c.selected, 1);
    assert_eq!(c.pinned, 1);
    assert!(c.window.start > 0 && c.window.end - c.window.start < 20);
    assert!(c.realized < 20);
    assert!(inspector.nodes().len() < 100);
    inspector.select(info.id);
    let snapshot = inspector.nodes();
    drop(pin);
    drop(list);
    drop(h);
    assert!(
        weak.upgrade().is_none(),
        "inspector snapshots must not retain list callbacks"
    );
    assert!(inspector.selected.get().is_none());
    assert!(inspector.nodes().is_empty());
    assert!(snapshot.iter().any(|n| n.collection.is_some()));
}

#[test]
fn overlay_scope_and_removal_are_reflected_without_retaining_presentation() {
    let inspector = Inspector::new();
    let popup = Popup::new();
    let h = Host::new({
        let p = popup.clone();
        move || {
            View::layer(vec![
                View::button("Open", || {}).key("open"),
                p.popover("open", View::button("Close", || {})),
            ])
        }
    });
    h.set_inspector(inspector.clone());
    layout(&h);
    popup.show();
    layout(&h);
    let modal = h
        .snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.kind == Kind::Modal)
        .unwrap();
    inspector.select(modal.id);
    layout(&h);
    let info = inspector.selected.get().unwrap();
    assert!(info.overlay.unwrap().active);
    assert!(info.in_scope);
    assert!(info.clip.width > 0.);
    assert!(
        !inspector
            .nodes()
            .into_iter()
            .find(|n| n.key.as_deref() == Some("open"))
            .unwrap()
            .in_scope
    );
    popup.dismiss();
    layout(&h);
    assert!(inspector.selected.get().is_none());
}

#[test]
fn separate_inspector_views_drive_source_selection_and_table_diagnostics_settle() {
    let inspector = Inspector::new();
    let table = TableModel::with_columns(
        vec![TableColumn::new("a", "A"), TableColumn::new("b", "B")],
        vec![TableRow::new("row").cell("a", "one").cell("b", "two")],
    )
    .unwrap();
    table.set_selection_mode(TableSelectionMode::Cells, false);
    table
        .set_selection(TableSelection {
            cells: [TableCell::new("row", "b")].into(),
            ..Default::default()
        })
        .unwrap();
    let source = Host::new({
        let t = table.clone();
        move || {
            View::column(vec![
                View::label("Source").key("title"),
                t.view().key("table"),
            ])
        }
    });
    source.set_inspector(inspector.clone());
    layout(&source);
    let panel = Host::new({
        let i = inspector.clone();
        move || i.view()
    });
    layout(&panel);
    let table_id = node(&source, "table").id;
    inspector.select(table_id);
    layout(&source);
    layout(&panel);
    let info = inspector.selected.get().unwrap().collection.unwrap();
    assert_eq!(info.columns, 2);
    assert_eq!(info.rows, 1);
    assert_eq!(info.selected, 1);
    assert_eq!(info.cursor.as_deref(), Some("row / b"));
    let outline = panel
        .snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.label == "Mounted hierarchy")
        .unwrap()
        .tree_model
        .unwrap();
    let title = node(&source, "title").id;
    outline.select(&title.to_string(), false);
    assert_eq!(
        inspector.selected.get().unwrap().id,
        title,
        "browsing must update while the source is not rendering"
    );
    layout(&source);
    layout(&panel);
    assert_eq!(inspector.selected.get().unwrap().id, title);
    assert!(
        inspector.nodes().iter().any(|n| n.id == table_id),
        "the view host must not replace the source hierarchy"
    );
    outline.clear_selection();
    assert!(
        inspector.selected.get().is_none(),
        "cleared hierarchy selection must clear details immediately"
    );
    layout(&source);
    layout(&panel);
    let generation = source.generation();
    let panel_generation = panel.generation();
    layout(&source);
    layout(&panel);
    layout(&source);
    layout(&panel);
    assert_eq!(source.generation(), generation);
    assert_eq!(panel.generation(), panel_generation);
}

#[test]
fn source_rebinding_cannot_publish_or_retire_another_hosts_selection() {
    let inspector = Inspector::new();
    let first = Host::new(|| View::button("First", || {}).key("first"));
    first.set_inspector(inspector.clone());
    layout(&first);
    let second = Host::new(|| View::button("Second", || {}).key("second"));
    second.set_inspector(inspector.clone());
    layout(&second);
    let id = node(&second, "second").id;
    inspector.select(id);
    layout(&second);
    layout(&first);
    inspector.picking.set(true);
    layout(&first);
    pointer(&first, center(&node(&first, "first")), PointerKind::Down);
    pointer(&first, center(&node(&first, "first")), PointerKind::Up);
    assert_eq!(
        inspector.selected.get().unwrap().id,
        id,
        "old source picking must not publish into the new source"
    );
    key(&first, Key::Escape);
    assert!(
        inspector.picking.get(),
        "an old source cannot cancel the new source's picking mode"
    );
    assert!(first.inspection().is_none());
    assert_eq!(second.inspection().unwrap().id, id);
    assert_eq!(inspector.nodes().len(), 1);
    drop(first);
    assert_eq!(inspector.selected.get().unwrap().id, id);
    drop(second);
    assert!(inspector.nodes().is_empty());
    assert!(inspector.selected.get().is_none());
}

#[test]
fn property_patches_follow_reactive_source_and_reset_each_property() {
    let inspector = Inspector::new();
    let large = State::new(false);
    let present = State::new(true);
    let source = Host::new({
        let (large, present) = (large.clone(), present.clone());
        move || {
            View::column(if present.get() {
                vec![
                    View::button("Target", || {})
                        .key("target")
                        .width(if large.get() { 220 } else { 100 })
                        .padding(if large.get() { 12 } else { 4 })
                        .foreground(if large.get() { 0xff123456 } else { 0xffabcdef }),
                ]
            } else {
                vec![]
            })
            .align(Align::Start)
        }
    });
    source.set_inspector(inspector.clone());
    layout(&source);
    let id = node(&source, "target").id;
    inspector.select(id);
    inspector
        .set_override(id, StyleOverride::default().width(160).bold(true))
        .unwrap();
    large.set(true);
    layout(&source);
    let info = inspector.selected.get().unwrap();
    assert_eq!(info.id, id);
    assert_eq!(info.bounds.width, 160.);
    assert_eq!(info.style.padding, Insets::all(12.));
    assert_eq!(info.style.foreground, Some(0xff123456));
    assert_eq!(info.source_style.width, Length::Points(220.));
    assert_eq!(
        info.overridden,
        vec![StyleProperty::Width, StyleProperty::Bold]
    );
    assert_eq!(info.measured.width, 160.);
    assert_eq!(info.content_bounds.width, 136.);
    inspector.reset_property(id, StyleProperty::Width);
    layout(&source);
    assert_eq!(node(&source, "target").bounds.width, 220.);
    assert!(node(&source, "target").style.bold);
    inspector
        .set_override(id, inspector.style_override(id).foreground(None))
        .unwrap();
    layout(&source);
    assert_eq!(node(&source, "target").style.foreground, None);
    assert_eq!(
        node(&source, "target").text_style.color,
        source.theme().text
    );
    inspector.reset_override(id);
    layout(&source);
    assert_eq!(node(&source, "target").text_style.color, 0xff123456);
    inspector
        .set_override(id, StyleOverride::default().width(220))
        .unwrap();
    large.set(false);
    layout(&source);
    assert_eq!(
        node(&source, "target").bounds.width,
        220.,
        "equal-valued overrides remain explicit"
    );
    present.set(false);
    layout(&source);
    assert!(inspector.style_override(id).is_empty());
    assert!(
        !inspector
            .set_override(id, StyleOverride::default().width(50))
            .unwrap()
    );
    present.set(true);
    layout(&source);
    assert_ne!(node(&source, "target").id, id);
    assert_eq!(node(&source, "target").bounds.width, 100.);
}

#[test]
fn invalid_patches_are_atomic_and_reactive_conflicts_are_visible_and_recoverable() {
    let inspector = Inspector::new();
    let minimum = State::new(0.);
    let source = Host::new({
        let minimum = minimum.clone();
        move || {
            View::column(vec![
                View::label("Target").key("target").min_width(minimum.get()),
            ])
        }
    });
    source.set_inspector(inspector.clone());
    layout(&source);
    let id = node(&source, "target").id;
    inspector.select(id);
    let patch = StyleOverride::default().max_width(120).padding(6);
    inspector.set_override(id, patch).unwrap();
    assert!(inspector.set_override(id, patch.padding(-1)).is_err());
    assert!(
        inspector
            .set_override(id, patch.font_size(Some(f64::NAN)))
            .is_err()
    );
    assert_eq!(inspector.style_override(id), patch);
    minimum.set(200.);
    layout(&source);
    let info = inspector.selected.get().unwrap();
    assert!(info.override_error.is_some());
    assert_eq!(
        info.style, info.source_style,
        "conflicting patches suspend as a unit"
    );
    minimum.set(0.);
    layout(&source);
    assert!(inspector.selected.get().unwrap().override_error.is_none());
    assert_eq!(node(&source, "target").style.padding, Insets::all(6.));
    let other = Host::new(|| View::label("New source"));
    other.set_inspector(inspector.clone());
    assert!(inspector.nodes().is_empty());
    assert!(inspector.style_override(id).is_empty());
    layout(&other);
    drop(source);
    assert_eq!(inspector.nodes().len(), 1);
}

#[test]
fn property_editors_preserve_invalid_drafts_and_commit_only_the_edited_property() {
    let inspector = Inspector::new();
    let source =
        Host::new(|| View::column(vec![View::button("Target", || {}).key("target").width(100)]));
    source.set_inspector(inspector.clone());
    layout(&source);
    let id = node(&source, "target").id;
    inspector.select(id);
    inspector.show_page(InspectorPage::Layout);
    let panel = Host::new({
        let i = inspector.clone();
        move || i.view()
    });
    panel.set_active(true);
    layout(&panel);
    let field = node(&panel, "edit-width").id;
    panel.focus(field).unwrap();
    assert!(panel.set_text(field, "-").unwrap());
    layout(&panel);
    key(&panel, Key::Enter);
    layout(&panel);
    assert_eq!(node(&panel, "edit-width").value, Value::Text("-".into()));
    assert_eq!(
        node(&panel, "edit-width").validation.unwrap().status,
        ValidationStatus::Invalid
    );
    assert!(inspector.style_override(id).is_empty());
    panel.set_text(field, "50%").unwrap();
    layout(&panel);
    key(&panel, Key::Enter);
    layout(&source);
    layout(&panel);
    assert_eq!(inspector.style_override(id).width, Some(percent(50.)));
    assert_eq!(
        inspector.selected.get().unwrap().overridden,
        vec![StyleProperty::Width]
    );
    inspector.reset_property(id, StyleProperty::Width);
    layout(&source);
    layout(&panel);
    assert_eq!(node(&panel, "edit-width").value, Value::Text("100".into()));
    assert!(
        panel
            .snapshot()
            .unwrap()
            .iter()
            .any(|n| n.text == "width · source")
    );
    // Other codecs exercise real retained editors rather than a parallel parser.
    for (property, text) in [
        ("padding", "2 4"),
        ("position", "relative"),
        ("left", "-5"),
        ("max_width", "inf"),
    ] {
        let field = node(&panel, &format!("edit-{property}")).id;
        panel.focus(field).unwrap();
        panel.set_text(field, text).unwrap();
        layout(&panel);
        key(&panel, Key::Enter);
        layout(&source);
        layout(&panel);
    }
    assert_eq!(
        inspector.style_override(id).padding,
        Some(Insets::symmetric(2., 4.))
    );
    assert_eq!(inspector.style_override(id).left, Some(Length::Points(-5.)));
    inspector.show_page(InspectorPage::Style);
    layout(&panel);
    let field = node(&panel, "edit-foreground").id;
    panel.focus(field).unwrap();
    panel.set_text(field, "#12345680").unwrap();
    layout(&panel);
    key(&panel, Key::Enter);
    layout(&source);
    assert_eq!(node(&source, "target").style.foreground, Some(0x80123456));
    inspector.reset_all_overrides();
    layout(&source);
    assert_eq!(
        node(&source, "target").style,
        inspector.selected.get().unwrap().source_style
    );
}

#[test]
fn flex_diagnostics_report_real_wrap_lines_and_overflow() {
    let inspector = Inspector::new();
    let source = Host::new(|| {
        View::column(vec![
            View::row(vec![
                View::label("A").width(80).shrink(0.),
                View::label("B").width(80).shrink(0.),
            ])
            .key("row")
            .width(120)
            .wrap(true)
            .gap(10.),
        ])
    });
    source.set_inspector(inspector.clone());
    layout(&source);
    let id = node(&source, "row").id;
    inspector.select(id);
    let info = inspector.selected.get().unwrap();
    assert_eq!(info.flex_lines.len(), 2);
    assert!(
        info.flex_lines
            .iter()
            .all(|line| line.items.len() == 1 && line.free == 40.)
    );
    inspector
        .set_override(id, StyleOverride::default().wrap(false))
        .unwrap();
    layout(&source);
    let info = inspector.selected.get().unwrap();
    assert_eq!(info.flex_lines.len(), 1);
    assert_eq!(info.flex_lines[0].free, -50.);
    assert_eq!(info.flex_lines[0].gap, 10.);
}
