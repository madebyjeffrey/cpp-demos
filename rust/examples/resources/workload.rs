use scars_window::{skia, ui::*};
use std::time::Instant;

#[derive(Debug)]
pub struct Report {
    pub rows: usize,
    pub visits: usize,
    pub edits: usize,
    pub list_nodes: usize,
    pub table_nodes: usize,
    pub tree_nodes: usize,
    pub paragraphs: usize,
    pub images: usize,
    pub undo: usize,
    pub after_clear: RendererCacheStats,
    pub milliseconds: u128,
}
impl Report {
    pub fn verify(&self) {
        assert!(self.list_nodes < 80, "list realization grew: {self:?}");
        assert!(self.table_nodes < 160, "table realization grew: {self:?}");
        assert!(self.tree_nodes < 80, "tree realization grew: {self:?}");
        assert!(self.paragraphs <= 512 && self.images <= 128, "{self:?}");
        assert_eq!(self.undo, self.edits.min(100), "{self:?}");
        assert_eq!(self.after_clear, RendererCacheStats::default());
    }
}

fn visit(
    renderer: &Renderer,
    rows: usize,
    visits: usize,
    build: impl Fn() -> View + 'static,
    reveal: impl Fn(usize),
) -> Result<usize, Error> {
    let host = Host::new(build);
    let size = Size::new(480., 240.);
    host.layout(size, renderer)?;
    let mut peak = 0;
    for step in 0..visits {
        reveal(step * (rows - 1) / (visits - 1));
        host.layout(size, renderer)?;
        peak = peak.max(host.snapshot()?.len());
        assert!(host.next_deadline().is_none() && !host.needs_animation());
    }
    Ok(peak)
}

/// Exercise real Skia layout, virtual collections, discrete edits and decoded images.
pub fn run(rows: usize, visits: usize, edits: usize) -> Result<Report, Box<dyn std::error::Error>> {
    if rows < 2 || visits < 2 || edits == 0 {
        return Err("Use at least two rows/visits and one edit".into());
    }
    let started = Instant::now();
    let renderer = Renderer::default();
    let mut report = Report {
        rows,
        visits,
        edits,
        list_nodes: 0,
        table_nodes: 0,
        tree_nodes: 0,
        paragraphs: 0,
        images: 0,
        undo: 0,
        after_clear: RendererCacheStats::default(),
        milliseconds: 0,
    };
    {
        let model = ListModel::new(
            (0..rows)
                .map(|i| ListItem::new(i.to_string(), format!("Row {i}")))
                .collect(),
            28.,
        );
        let view = model.clone();
        report.list_nodes = visit(
            &renderer,
            rows,
            visits,
            move || view.view(),
            |i| model.reveal(i),
        )?;
    }
    {
        let model = TableModel::with_columns(
            vec![TableColumn::new("name", "Name")],
            (0..rows)
                .map(|i| TableRow::new(i.to_string()).cell("name", format!("Row {i}")))
                .collect(),
        )?;
        let view = model.clone();
        report.table_nodes = visit(
            &renderer,
            rows,
            visits,
            move || view.view(),
            |i| {
                model.reveal_row(&i.to_string());
            },
        )?;
    }
    {
        let model = TreeModel::from_nodes(
            (0..rows)
                .map(|i| TreeNode::new(i.to_string(), format!("Row {i}")))
                .collect(),
        )?;
        let view = model.clone();
        report.tree_nodes = visit(
            &renderer,
            rows,
            visits,
            move || view.view(),
            |i| {
                model.reveal(&i.to_string()).unwrap();
            },
        )?;
    }
    {
        let text = State::new(String::new());
        let image_index = State::new(0usize);
        let bitmaps = (0..300)
            .map(|i| Bitmap::from_rgba(1, 1, &[i as u8, (i >> 8) as u8, 0, 255], 4))
            .collect::<Result<Vec<_>, _>>()?;
        let host = Host::new({
            let (text, image_index) = (text.clone(), image_index.clone());
            move || view! { column { text_field("Draft",bind:text,key:"draft"); image(bitmaps[image_index.get()].clone(),width:16,height:16); } }
        });
        let size = Size::new(480., 120.);
        host.layout(size, &renderer)?;
        let field = host
            .snapshot()?
            .into_iter()
            .find(|n| n.key.as_deref() == Some("draft"))
            .unwrap()
            .id;
        host.focus(field)?;
        let mut surface =
            skia::surfaces::raster_n32_premul((480, 120)).ok_or("Raster surface unavailable")?;
        for step in 0..edits {
            let value = format!("Revision {step}: editing 👩🏽‍💻 مرحبا e\u{301}");
            host.set_text(field, &value)?;
            image_index.set(step % 300);
            host.layout(size, &renderer)?;
            renderer.paint(&host, surface.canvas(), 0.)?;
            let stats = renderer.cache_stats();
            report.paragraphs = report.paragraphs.max(stats.paragraphs);
            report.images = report.images.max(stats.images);
        }
        assert_eq!(host.text_input()?.unwrap().text, text.get());
        let latest = text.get();
        while host.text_input()?.unwrap().can_undo {
            assert!(host.text_command(TextCommand::Undo, false)?);
            report.undo += 1;
            assert!(
                report.undo <= 100,
                "mounted undo history exceeded its limit"
            );
        }
        for _ in 0..report.undo {
            assert!(host.text_command(TextCommand::Redo, false)?);
        }
        assert_eq!(text.get(), latest, "undo/redo must restore the latest edit");
        assert!(host.next_deadline().is_none() && !host.needs_animation());
    }
    renderer.clear_caches();
    report.after_clear = renderer.cache_stats();
    report.milliseconds = started.elapsed().as_millis();
    report.verify();
    Ok(report)
}
