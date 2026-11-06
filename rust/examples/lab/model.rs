//! Application state and reusable controllers are created once, outside rendering.
use scars_window::ui::*;
#[derive(Clone)]
pub struct Counter {
    pub count: State<i64>,
    pub details: State<bool>,
}
impl Counter {
    fn new() -> Self {
        Self {
            count: State::new(0),
            details: State::new(false),
        }
    }
}
#[derive(Clone)]
pub struct Lab {
    pub pages: Tabs,
    pub enabled: State<bool>,
    pub readonly: State<bool>,
    pub checked: State<bool>,
    pub tri: State<CheckState>,
    pub choice: State<String>,
    pub level: State<f64>,
    pub text: State<String>,
    pub notes: State<String>,
    pub search: State<String>,
    pub password: State<String>,
    pub status: State<String>,
    pub counters: [Counter; 2],
    pub number: Numeric<i64>,
    pub date: DatePicker,
    pub time: TimePicker,
    pub datetime: DateTimePicker,
    pub zoned: ZonedPicker,
    pub color: ColorPicker,
    pub menu: Menu,
    pub context: Menu,
    pub dropdown: Popup,
    pub popup: Popup,
    pub color_popup: Popup,
    pub export: SplitAction,
    pub active: State<Option<Alert>>,
    pub split: Split,
    pub vertical: Split,
    pub tabs: Tabs,
    pub wizard: Wizard,
    pub list: ListModel,
    pub tree: TreeModel,
    pub combo: ComboBox,
    pub table: TableModel,
    pub image: Bitmap,
    pub inspector: Inspector,
}
impl Lab {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let text = State::new("Kestrel project".to_owned());
        let status = State::new("Ready to explore".to_owned());
        let name = text.clone();
        let wizard = Wizard::new(
            "name",
            vec![
                WizardStep::new("name", "Name").validate(move || {
                    if name.get().trim().is_empty() {
                        Err("Enter a project name".into())
                    } else {
                        Ok(())
                    }
                }),
                WizardStep::new("review", "Review"),
            ],
        )?
        .on_finish(status.action(|s| s.set("Wizard finished".into())));
        let tree = TreeModel::from_nodes(vec![TreeNode::new("workspace", "Workspace").children(
            vec![
                TreeNode::new("notes", "Notes.md"),
                TreeNode::new("lazy", "Lazy branch")
                    .lazy(|| Ok(vec![TreeNode::new("loaded", "Loaded document")])),
                TreeNode::new("unavailable", "Unavailable").enabled(false),
            ],
        )])?;
        tree.expand("workspace")?;
        let table = TableModel::with_options(
            vec![
                TableColumn::new("name", "Document")
                    .width(190.)
                    .editable(true)
                    .validate(|_, s| {
                        if s.trim().is_empty() {
                            Err(Error("A name is required".into()))
                        } else {
                            Ok(())
                        }
                    }),
                TableColumn::new("state", "Status").width(130.),
            ],
            (0..1_000)
                .map(|i| {
                    TableRow::new(i.to_string())
                        .cell("name", format!("Document {i}"))
                        .cell("state", "Ready")
                })
                .collect(),
            TableOptions {
                selection_mode: TableSelectionMode::Cells,
                ..Default::default()
            },
        )?
        .on_commit(status.handler(|s, event: TableCommit| s.set(format!("Saved {}", event.text))));
        let pixels: Vec<u8> = (0..32)
            .flat_map(|y| {
                (0..48).flat_map(move |x| {
                    if (x / 8 + y / 8) % 2 == 0 {
                        [42, 112, 190, 255]
                    } else {
                        [245, 181, 71, 255]
                    }
                })
            })
            .collect();
        Ok(Self {
            pages: Tabs::new("controls"),
            enabled: State::new(true),
            readonly: State::new(false),
            checked: State::new(true),
            tri: State::new(CheckState::Indeterminate),
            choice: State::new("a".into()),
            level: State::new(0.4),
            text,
            notes: State::new("Rust components, native input.\nمرحبا · שלום · café · 👩🏽‍💻".into()),
            search: State::new(String::new()),
            password: State::new("private sample".into()),
            status,
            counters: [Counter::new(), Counter::new()],
            number: Numeric::new(State::new(3i64).binding(), 0, 20, 1),
            date: DatePicker::new(State::new("2026-11-06".parse::<Date>()?).binding())
                .locale(EditLocale::new("de-DE")?),
            time: TimePicker::new(State::new("14:30:00".parse::<Time>()?).binding())
                .hour_cycle(HourCycle::H12),
            datetime: DateTimePicker::new(
                State::new("2026-11-06T14:30".parse::<DateTime>()?).binding(),
            ),
            zoned: ZonedPicker::new(
                State::new("2026-11-06T21:30:00Z".parse::<Timestamp>()?).binding(),
                "America/Edmonton",
            )?,
            color: ColorPicker::new(State::new(Color::rgba(55, 135, 220, 180)).binding()),
            menu: Menu::new(),
            context: Menu::new(),
            dropdown: Popup::new(),
            popup: Popup::new().options(OverlayOptions {
                width: Some(280.),
                max_height: 320.,
                ..Default::default()
            }),
            color_popup: Popup::new(),
            export: SplitAction::new(),
            active: State::new(None),
            split: Split::new(0.35).limits(40., 40.),
            vertical: Split::new(0.55)
                .orientation(Orientation::Vertical)
                .limits(30., 30.),
            tabs: Tabs::new("first"),
            wizard,
            list: ListModel::new(
                (0..10_000)
                    .map(|i| ListItem::new(i.to_string(), format!("Item {i:05}")))
                    .collect(),
                28.,
            ),
            tree,
            combo: ComboBox::new(
                (0..1_000)
                    .map(|i| OptionItem::new(i.to_string(), format!("Document {i:04}")))
                    .collect(),
            )?,
            table,
            image: Bitmap::from_rgba(48, 32, &pixels, 0)?,
            inspector: Inspector::new(),
        })
    }
    pub fn confirm(&self) {
        self.active.set(Some(
            Alert::new(
                "Apply settings?",
                vec![
                    AlertAction::new("cancel", "Cancel").role(AlertActionRole::Cancel),
                    AlertAction::new("apply", "Apply").role(AlertActionRole::Default),
                ],
                self.status
                    .handler(|s, result: AlertResult| s.set(format!("Alert: {:?}", result.reason))),
            )
            .unwrap(),
        ));
    }
    pub fn set_readonly(&self, value: bool) {
        self.readonly.set(value);
        self.table
            .set_columns(
                self.table
                    .columns()
                    .into_iter()
                    .map(|column| {
                        let editable = column.id == "name" && !value;
                        column.editable(editable)
                    })
                    .collect(),
            )
            .unwrap();
    }
    pub fn readonly_binding(&self) -> Binding<bool> {
        let (read, write) = (self.readonly.clone(), self.clone());
        Binding::new(move || read.get(), move |value| write.set_readonly(value))
    }
    /// CLI previews drive actual openers after layout, including offscreen cards.
    pub fn present(&self, kind: &str, host: &Host) -> Result<(), Error> {
        let key = match kind {
            "menu" => "lab-actions",
            "dropdown" => "lab-choice",
            "popover" => "lab-popover",
            "alert" => "confirm",
            "color" => "lab-color",
            "combo" => "",
            _ => return Err(Error("Unknown presentation".into())),
        };
        let opener = host
            .snapshot()?
            .into_iter()
            .find(|n| {
                if kind == "combo" {
                    n.combo.is_some()
                } else {
                    n.key.as_deref() == Some(key)
                }
            })
            .ok_or_else(|| Error("Presentation opener is not mounted".into()))?;
        if !opener.enabled || opener.hidden {
            return Err(Error("Presentation opener is unavailable".into()));
        }
        host.reveal_rect(opener.id, opener.bounds);
        host.focus(opener.id)?;
        if kind == "combo" {
            self.combo.show();
        } else {
            host.activate(opener.id)?;
        }
        Ok(())
    }
}
