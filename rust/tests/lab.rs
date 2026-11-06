//! Exercise the application catalog with real Skia layout and painting.
#[path = "../examples/lab/view.rs"]
mod catalog;
#[path = "../examples/lab/model.rs"]
mod model;
use model::Lab;
use scars_window::{skia, ui::*};
fn mounted(lab: &Lab) -> Host {
    let h = Host::new({
        let lab = lab.clone();
        move || catalog::view(&lab)
    });
    h.set_inspector(lab.inspector.clone());
    h.set_active(true);
    h
}
fn layout(h: &Host, size: Size, r: &Renderer) {
    h.layout(size, r).unwrap();
}
fn find(h: &Host, key: &str) -> Node {
    h.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.key.as_deref() == Some(key))
        .unwrap_or_else(|| panic!("Missing {key}"))
}
fn paint(h: &Host, size: (i32, i32), r: &Renderer, name: &str) {
    let mut surface = skia::surfaces::raster_n32_premul(size).unwrap();
    surface.canvas().clear(skia::Color::new(h.theme().window));
    r.paint(h, surface.canvas(), 0.).unwrap();
    if let Some(directory) = std::env::var_os("SCARS_LAB_ARTIFACTS") {
        std::fs::create_dir_all(&directory).unwrap();
        let png = surface
            .image_snapshot()
            .encode(None, skia::EncodedImageFormat::PNG, None)
            .unwrap();
        std::fs::write(
            std::path::PathBuf::from(directory).join(format!("{name}.png")),
            png.as_bytes(),
        )
        .unwrap();
    }
}
#[test]
fn every_sample_is_reachable_and_paints_in_both_themes_at_narrow_and_wide_sizes() {
    let r = Renderer::default();
    let mut kinds = std::collections::HashSet::new();
    for (theme, palette) in [("dark", Theme::default()), ("light", Theme::light())] {
        for (width, height) in [(440, 740), (1120, 900)] {
            let lab = Lab::new().unwrap();
            let h = mounted(&lab);
            h.set_theme(palette);
            let size = Size::new(width as f64, height as f64);
            for (sample, page) in catalog::SAMPLES {
                lab.pages.select(*page);
                layout(&h, size, &r);
                let card = find(&h, &format!("sample-{sample}"));
                assert!(!card.hidden, "{sample} must be on its declared page");
                assert!(
                    card.bounds.width > 0. && card.bounds.height > 0.,
                    "{sample}: {:?}",
                    card.bounds
                );
                h.reveal_rect(card.id, card.bounds);
                layout(&h, size, &r);
                let card = find(&h, &format!("sample-{sample}"));
                assert!(
                    card.clip.width > 0. && card.clip.height > 0.,
                    "{sample} unreachable at {width}"
                );
                for node in h.snapshot().unwrap().into_iter().filter(|n| !n.hidden) {
                    kinds.insert(node.kind);
                    assert!(
                        [
                            node.bounds.x,
                            node.bounds.y,
                            node.bounds.width,
                            node.bounds.height,
                            node.clip.width,
                            node.clip.height
                        ]
                        .iter()
                        .all(|v| v.is_finite()),
                        "{sample}: {:?}",
                        node.kind
                    );
                    assert!(node.clip.width >= 0. && node.clip.height >= 0.);
                }
                paint(
                    &h,
                    (width, height),
                    &r,
                    &format!("{theme}-{width}-{sample}"),
                );
                if card.bounds.height > card.clip.height + 1. {
                    h.reveal_rect(
                        card.id,
                        Rect::new(
                            card.bounds.x,
                            card.bounds.y + card.bounds.height - 1.,
                            card.bounds.width,
                            1.,
                        ),
                    );
                    layout(&h, size, &r);
                    paint(
                        &h,
                        (width, height),
                        &r,
                        &format!("{theme}-{width}-{sample}-bottom"),
                    );
                }
            }
            assert!(lab.list.realized_indices().len() < 30);
            assert!(lab.table.realized_row_count() < 30);
        }
    }
    for kind in [
        Kind::Column,
        Kind::Row,
        Kind::Block,
        Kind::Inline,
        Kind::Layer,
        Kind::Label,
        Kind::RichText,
        Kind::Button,
        Kind::Toggle,
        Kind::CheckBox,
        Kind::RadioGroup,
        Kind::Radio,
        Kind::Segmented,
        Kind::Segment,
        Kind::Tabs,
        Kind::Tab,
        Kind::TabPanel,
        Kind::Switch,
        Kind::Slider,
        Kind::Stepper,
        Kind::Split,
        Kind::SplitDivider,
        Kind::Progress,
        Kind::Image,
        Kind::Section,
        Kind::GroupBox,
        Kind::Panel,
        Kind::Spacer,
        Kind::Separator,
        Kind::TextField,
        Kind::Scroll,
        Kind::CollectionRow,
        Kind::Wizard,
    ] {
        assert!(kinds.contains(&kind), "Catalog never mounted {kind:?}");
    }
}
#[test]
fn custom_instances_and_visited_editors_retain_independent_state_and_identity() {
    let lab = Lab::new().unwrap();
    let h = mounted(&lab);
    let r = Renderer::default();
    let size = Size::new(900., 800.);
    layout(&h, size, &r);
    let increase = h
        .snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.label == "Increase Downloads")
        .unwrap()
        .id;
    h.activate(increase).unwrap();
    assert_eq!(lab.counters[0].count.get(), 1);
    assert_eq!(lab.counters[1].count.get(), 0);
    lab.pages.select("fields");
    layout(&h, size, &r);
    let notes = find(&h, "notes").id;
    h.focus(notes).unwrap();
    h.set_text(notes, "Retained 👩🏽‍💻 notes").unwrap();
    lab.pages.select("layout");
    layout(&h, size, &r);
    lab.pages.select("fields");
    layout(&h, size, &r);
    assert_eq!(find(&h, "notes").id, notes);
    assert_eq!(lab.notes.get(), "Retained 👩🏽‍💻 notes");
    lab.pages.select("controls");
    layout(&h, size, &r);
    assert!(
        h.snapshot()
            .unwrap()
            .iter()
            .any(|n| n.id == increase && !n.hidden)
    );
    h.activate(increase).unwrap();
    assert_eq!(lab.counters[0].count.get(), 2);
    lab.set_readonly(true);
    lab.pages.select("fields");
    layout(&h, size, &r);
    assert!(find(&h, "notes").read_only);
    assert!(!h.set_text(notes, "Should not change").unwrap());
    assert!(lab.table.start_edit(TableCell::new("0", "name")).is_err());
}
#[test]
fn catalog_overlays_are_scoped_and_data_actions_operate_on_shared_models() {
    let lab = Lab::new().unwrap();
    let h = mounted(&lab);
    let r = Renderer::default();
    let size = Size::new(1120., 900.);
    lab.pages.select("flow");
    layout(&h, size, &r);
    h.activate(find(&h, "confirm").id).unwrap();
    layout(&h, size, &r);
    assert!(h.snapshot().unwrap().iter().any(|n| n.alert && !n.hidden));
    assert!(lab.active.get().unwrap().choose("apply"));
    layout(&h, size, &r);
    assert!(lab.status.get().starts_with("Alert:"));
    lab.menu.show();
    layout(&h, size, &r);
    assert!(
        h.snapshot()
            .unwrap()
            .iter()
            .any(|n| n.kind == Kind::Modal && !n.hidden)
    );
    lab.menu.dismiss();
    lab.pages.select("data");
    layout(&h, size, &r);
    lab.list.reveal(9999);
    layout(&h, size, &r);
    assert!(lab.list.visible_range().contains(&9999));
    let edit = lab.table.start_edit(TableCell::new("0", "name")).unwrap();
    edit.set_text("Renamed");
    assert!(edit.commit().unwrap());
    assert_eq!(
        lab.table
            .cell_info(&TableCell::new("0", "name"))
            .unwrap()
            .text,
        "Renamed"
    );
    lab.tree.expand("lazy").unwrap();
    assert!(lab.tree.item("loaded").is_some());
}

#[test]
fn startup_presentations_reveal_offscreen_openers_before_opening() {
    let r = Renderer::default();
    let size = Size::new(440., 740.);
    for (kind, page) in [
        ("menu", "flow"),
        ("dropdown", "flow"),
        ("popover", "flow"),
        ("alert", "flow"),
        ("combo", "data"),
        ("color", "values"),
    ] {
        let lab = Lab::new().unwrap();
        lab.pages.select(page);
        let h = mounted(&lab);
        layout(&h, size, &r);
        lab.present(kind, &h).unwrap();
        layout(&h, size, &r);
        let portals: Vec<_> = h
            .snapshot()
            .unwrap()
            .into_iter()
            .filter(|n| n.kind == Kind::Modal && !n.hidden)
            .collect();
        assert!(
            !portals.is_empty(),
            "{kind} closed before its opener became visible"
        );
        assert!(h.focused().is_some());
        assert!(
            portals
                .iter()
                .all(|n| n.clip.width > 0. && n.clip.height > 0.)
        );
        paint(&h, (440, 740), &r, &format!("startup-{kind}"));
    }
}
