//! Validated navigation with stable step IDs and application-owned form data.
use crate::{Binding, Error, Host, Kind, NodeId, State, View};
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    rc::Rc,
};
#[derive(Clone)]
pub struct WizardStep {
    pub id: String,
    pub title: String,
    validate: Option<Rc<dyn Fn() -> Result<(), String>>>,
}
impl WizardStep {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            validate: None,
        }
    }
    /// Pure/reactive validation. Rechecked when navigation is activated.
    pub fn validate(mut self, validate: impl Fn() -> Result<(), String> + 'static) -> Self {
        self.validate = Some(Rc::new(validate));
        self
    }
    fn title(&self) -> &str {
        if self.title.trim().is_empty() {
            &self.id
        } else {
            &self.title
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WizardOptions {
    pub allow_back: bool,
    pub review_previous: bool,
    pub validate_all_on_finish: bool,
    pub show_steps: bool,
    pub show_status: bool,
    pub back_label: String,
    pub next_label: String,
    pub finish_label: String,
    pub cancel_label: String,
}
impl Default for WizardOptions {
    fn default() -> Self {
        Self {
            allow_back: true,
            review_previous: true,
            validate_all_on_finish: true,
            show_steps: true,
            show_status: true,
            back_label: "Back".into(),
            next_label: "Next".into(),
            finish_label: "Finish".into(),
            cancel_label: "Cancel".into(),
        }
    }
}
impl WizardOptions {
    fn validate(&self) -> Result<(), Error> {
        if [
            &self.back_label,
            &self.next_label,
            &self.finish_label,
            &self.cancel_label,
        ]
        .iter()
        .any(|s| s.trim().is_empty())
        {
            return Err(Error("Wizard action labels must not be empty".into()));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WizardChange {
    pub from: String,
    pub to: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WizardIssue {
    pub step_id: String,
    pub message: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WizardStepInfo {
    pub id: String,
    pub title: String,
    pub can_review: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WizardInfo {
    pub requested: String,
    pub current: Option<String>,
    pub index: Option<usize>,
    pub steps: Vec<WizardStepInfo>,
    pub issue: Option<WizardIssue>,
    pub can_back: bool,
    pub can_advance: bool,
    pub can_cancel: bool,
    pub is_last: bool,
}
struct Inner {
    selection: Binding<String>,
    steps: RefCell<Vec<WizardStep>>,
    revision: State<u64>,
    busy: Cell<bool>,
}
/// Create once outside builders. Clones share selection and the current step set.
#[derive(Clone)]
pub struct Wizard {
    inner: Rc<Inner>,
    options: WizardOptions,
    finish: Option<Rc<dyn Fn()>>,
    cancel: Option<Rc<dyn Fn()>>,
    change: Option<Rc<dyn Fn(WizardChange)>>,
}
struct Busy<'a>(&'a Cell<bool>);
impl Drop for Busy<'_> {
    fn drop(&mut self) {
        self.0.set(false)
    }
}
impl Wizard {
    pub fn new(current: impl Into<String>, steps: Vec<WizardStep>) -> Result<Self, Error> {
        Self::bound(State::new(current.into()).binding(), steps)
    }
    pub fn bound(current: Binding<String>, steps: Vec<WizardStep>) -> Result<Self, Error> {
        validate_steps(&steps)?;
        Ok(Self {
            inner: Rc::new(Inner {
                selection: current,
                steps: RefCell::new(steps),
                revision: State::new(0),
                busy: Cell::new(false),
            }),
            options: Default::default(),
            finish: None,
            cancel: None,
            change: None,
        })
    }
    pub fn options(mut self, options: WizardOptions) -> Result<Self, Error> {
        options.validate()?;
        self.options = options;
        Ok(self)
    }
    pub fn on_finish(mut self, callback: impl Fn() + 'static) -> Self {
        self.finish = Some(Rc::new(callback));
        self
    }
    pub fn on_cancel(mut self, callback: impl Fn() + 'static) -> Self {
        self.cancel = Some(Rc::new(callback));
        self
    }
    pub fn on_change(mut self, callback: impl Fn(WizardChange) + 'static) -> Self {
        self.change = Some(Rc::new(callback));
        self
    }
    /// Replace atomically. Existing IDs retain selection and mounted page identity.
    pub fn set_steps(&self, steps: Vec<WizardStep>) -> Result<(), Error> {
        crate::state::check_write();
        validate_steps(&steps)?;
        let old = self.inner.steps.replace(steps);
        self.inner.revision.update(|n| *n = n.wrapping_add(1));
        drop(old);
        Ok(())
    }
    fn steps(&self) -> Vec<WizardStep> {
        let _ = self.inner.revision.get();
        self.inner.steps.borrow().clone()
    }
    fn position(steps: &[WizardStep], requested: &str) -> Option<usize> {
        steps
            .iter()
            .position(|s| s.id == requested)
            .or((!steps.is_empty()).then_some(0))
    }
    pub fn current(&self) -> Option<String> {
        let steps = self.steps();
        Self::position(&steps, &self.inner.selection.get()).map(|i| steps[i].id.clone())
    }
    pub fn index(&self) -> Option<usize> {
        Self::position(&self.steps(), &self.inner.selection.get())
    }
    fn issue(&self, steps: &[WizardStep], index: usize) -> Option<WizardIssue> {
        let first = if index + 1 == steps.len() && self.options.validate_all_on_finish {
            0
        } else {
            index
        };
        crate::state::read_only(|| {
            for step in &steps[first..=index] {
                if let Some(validate) = &step.validate
                    && let Err(message) = validate()
                {
                    return Some(WizardIssue {
                        step_id: step.id.clone(),
                        message: if message.trim().is_empty() {
                            "This step is not valid.".into()
                        } else {
                            message
                        },
                    });
                }
            }
            None
        })
    }
    pub fn info(&self) -> WizardInfo {
        let steps = self.steps();
        let requested = self.inner.selection.get();
        let index = Self::position(&steps, &requested);
        let is_last = index.is_some_and(|i| i + 1 == steps.len());
        let issue = index.and_then(|i| self.issue(&steps, i));
        WizardInfo {
            requested,
            current: index.map(|i| steps[i].id.clone()),
            index,
            steps: steps
                .iter()
                .enumerate()
                .map(|(i, s)| WizardStepInfo {
                    id: s.id.clone(),
                    title: s.title().into(),
                    can_review: index.is_some_and(|current| {
                        i == current
                            || i < current
                                && self.options.allow_back
                                && self.options.review_previous
                    }),
                })
                .collect(),
            can_back: self.options.allow_back && index.is_some_and(|i| i > 0),
            can_advance: index.is_some() && issue.is_none() && (!is_last || self.finish.is_some()),
            can_cancel: self.cancel.is_some(),
            issue,
            is_last,
        }
    }
    fn begin(&self) -> Option<Busy<'_>> {
        crate::state::check_write();
        if self.inner.busy.replace(true) {
            None
        } else {
            Some(Busy(&self.inner.busy))
        }
    }
    fn transition(&self, target: &str) -> bool {
        let from = self.inner.selection.get();
        if from == target {
            return false;
        }
        self.inner.selection.set(target.into());
        let to = self.inner.selection.get();
        if to == from || !self.steps().iter().any(|s| s.id == to) {
            return false;
        }
        if let Some(change) = &self.change {
            change(WizardChange { from, to });
        }
        true
    }
    pub fn next(&self) -> bool {
        let Some(_busy) = self.begin() else {
            return false;
        };
        let steps = self.steps();
        let Some(index) = Self::position(&steps, &self.inner.selection.get()) else {
            return false;
        };
        index + 1 < steps.len()
            && self.issue(&steps, index).is_none()
            && self.transition(&steps[index + 1].id)
    }
    pub fn back(&self) -> bool {
        let Some(_busy) = self.begin() else {
            return false;
        };
        let steps = self.steps();
        let Some(index) = Self::position(&steps, &self.inner.selection.get()) else {
            return false;
        };
        self.options.allow_back && index > 0 && self.transition(&steps[index - 1].id)
    }
    /// Review an earlier step; forward skips are never allowed through this API.
    pub fn go_to(&self, id: &str) -> bool {
        let Some(_busy) = self.begin() else {
            return false;
        };
        let steps = self.steps();
        let Some(index) = Self::position(&steps, &self.inner.selection.get()) else {
            return false;
        };
        let Some(target) = steps.iter().position(|s| s.id == id) else {
            return false;
        };
        (target == index
            || target < index && self.options.allow_back && self.options.review_previous)
            && self.transition(id)
    }
    pub fn finish(&self) -> bool {
        let Some(_busy) = self.begin() else {
            return false;
        };
        let info = self.info();
        if !info.is_last || !info.can_advance {
            return false;
        }
        if let Some(finish) = &self.finish {
            finish();
            true
        } else {
            false
        }
    }
    pub fn cancel(&self) -> bool {
        let Some(_busy) = self.begin() else {
            return false;
        };
        if let Some(cancel) = &self.cancel {
            cancel();
            true
        } else {
            false
        }
    }
    pub fn navigation(&self) -> View {
        self.build(None)
    }
    /// Declare bodies by ID. Removed-step bodies are ignored; missing bodies are empty.
    pub fn view<S: Into<String>>(&self, pages: Vec<(S, View)>) -> View {
        self.build(Some(
            pages
                .into_iter()
                .map(|(id, body)| (id.into(), body))
                .collect(),
        ))
    }
    fn build(&self, pages: Option<Vec<(String, View)>>) -> View {
        let info = self.info();
        let token = Token {
            revision: self.inner.revision.get(),
            requested: info.requested.clone(),
            current: info.current.clone(),
        };
        let mut children = vec![
            View::label(match info.index {
                Some(i) => format!(
                    "Step {} of {} · {}",
                    i + 1,
                    info.steps.len(),
                    info.steps[i].title
                ),
                None => "No steps".into(),
            })
            .bold(true)
            .key("$title")
            .shrink(0.),
        ];
        if self.options.show_steps {
            children.push(
                View::row(
                    info.steps
                        .iter()
                        .enumerate()
                        .map(|(i, step)| {
                            let mut button = self
                                .button(
                                    format!("{}. {}", i + 1, step.title),
                                    Command::Review(step.id.clone()),
                                    &token,
                                )
                                .key(format!("$step-{}", step.id))
                                .enabled(step.can_review)
                                .shrink(0.);
                            let selected = info.current.as_ref() == Some(&step.id);
                            Rc::make_mut(&mut button.0).data =
                                crate::view::Data::Bool(Binding::new(move || selected, |_| {}));
                            button
                        })
                        .collect(),
                )
                .wrap(true)
                .key("$steps")
                .shrink(0.),
            );
        }
        let page_ids = pages
            .as_ref()
            .map_or_else(Vec::new, |p| p.iter().map(|(id, _)| id.clone()).collect());
        if let Some(pages) = pages {
            let mut bodies: HashMap<_, _> = pages.into_iter().collect();
            let read = self.clone();
            let binding = Binding::new(move || read.current().unwrap_or_default(), |_| {});
            let pages = info
                .steps
                .iter()
                .map(|step| {
                    crate::Tab::new(
                        &step.id,
                        &step.title,
                        bodies
                            .remove(&step.id)
                            .unwrap_or_else(|| View::fragment(vec![])),
                    )
                })
                .collect();
            children.push(
                crate::tabs::page_stack(binding, pages)
                    .key("$pages")
                    .grow(1.),
            );
        }
        if self.options.show_status {
            let message = if let Some(issue) = &info.issue {
                let title = info
                    .steps
                    .iter()
                    .find(|s| s.id == issue.step_id)
                    .map(|s| s.title.as_str())
                    .unwrap_or(&issue.step_id);
                if info.current.as_ref() != Some(&issue.step_id) {
                    format!("{title}: {}", issue.message)
                } else {
                    issue.message.clone()
                }
            } else if info.current.is_none() {
                "No steps available.".into()
            } else if info.is_last {
                "Ready to finish.".into()
            } else {
                "Ready for the next step.".into()
            };
            children.push(
                View::label(&message)
                    .wrap(true)
                    .label_style(if info.issue.is_some() {
                        crate::LabelStyle::Error
                    } else {
                        crate::LabelStyle::Secondary
                    })
                    .accessible_label(if info.issue.is_some() {
                        format!("Wizard validation: {message}")
                    } else {
                        "Wizard status".into()
                    })
                    .key("$status")
                    .shrink(0.),
            );
        }
        let mut actions = vec![];
        if self.cancel.is_some() {
            actions.push(
                self.button(self.options.cancel_label.clone(), Command::Cancel, &token)
                    .key("$cancel"),
            );
        }
        actions.push(View::spacer().grow(1.));
        actions.push(
            self.button(self.options.back_label.clone(), Command::Back, &token)
                .enabled(info.can_back)
                .key(format!("$back-{}-{}", token.revision, token.requested)),
        );
        actions.push(
            self.button(
                if info.is_last {
                    self.options.finish_label.clone()
                } else {
                    self.options.next_label.clone()
                },
                if info.is_last {
                    Command::Finish
                } else {
                    Command::Next
                },
                &token,
            )
            .enabled(info.can_advance)
            .key(format!("$advance-{}-{}", token.revision, token.requested)),
        );
        children.push(
            View::row(actions)
                .gap(10.)
                .wrap(true)
                .key("$actions")
                .shrink(0.),
        );
        let mut view = View::new(Kind::Wizard, String::new(), children)
            .gap(12.)
            .accessible_label("Wizard navigation");
        Rc::make_mut(&mut view.0).wizard = Some(WizardData { info, page_ids });
        view
    }
    fn button(&self, label: String, command: Command, token: &Token) -> View {
        let mut button = View::button(label, || {});
        Rc::make_mut(&mut button.0).wizard_action = Some(WizardAction {
            model: self.clone(),
            command,
            token: token.clone(),
        });
        button
    }
}
fn validate_steps(steps: &[WizardStep]) -> Result<(), Error> {
    let mut ids = HashSet::new();
    for step in steps {
        if step.id.is_empty() || !ids.insert(&step.id) {
            return Err(Error("Wizard step IDs must be nonempty and unique".into()));
        }
    }
    Ok(())
}
#[derive(Clone)]
pub(crate) struct WizardData {
    pub info: WizardInfo,
    pub page_ids: Vec<String>,
}
impl WizardData {
    pub fn validate(&self) -> Result<(), Error> {
        let mut keys = HashSet::new();
        if self
            .page_ids
            .iter()
            .any(|id| id.is_empty() || !keys.insert(id))
        {
            return Err(Error("Wizard page IDs must be nonempty and unique".into()));
        }
        Ok(())
    }
}
#[derive(Clone)]
struct Token {
    revision: u64,
    requested: String,
    current: Option<String>,
}
#[derive(Clone)]
enum Command {
    Next,
    Back,
    Review(String),
    Finish,
    Cancel,
}
#[derive(Clone)]
pub(crate) struct WizardAction {
    model: Wizard,
    command: Command,
    token: Token,
}
impl WizardAction {
    pub(crate) fn is_advance(&self) -> bool {
        matches!(self.command, Command::Next | Command::Finish)
    }
    fn execute(&self) -> bool {
        if matches!(self.command, Command::Cancel) {
            return self.model.cancel();
        }
        if self.model.inner.revision.get() != self.token.revision
            || self.model.inner.selection.get() != self.token.requested
            || self.model.current() != self.token.current
        {
            return false;
        }
        match &self.command {
            Command::Next => self.model.next(),
            Command::Back => self.model.back(),
            Command::Review(id) => self.model.inner.selection.get() == *id || self.model.go_to(id),
            Command::Finish => self.model.finish(),
            Command::Cancel => self.model.cancel(),
        }
    }
}
impl Host {
    pub(crate) fn wizard_action(&self, id: NodeId, action: &WizardAction) -> Result<bool, Error> {
        self.unmark_text(true)?;
        self.refresh()?;
        let valid =
            self.0.mounted.borrow().nodes.iter().any(|n| {
                n.id == id && n.enabled && !n.hidden && !n.layout_hidden && self.in_scope(id)
            });
        let performed = valid && action.execute();
        self.0.observer.dirty.set(true);
        self.redraw();
        Ok(performed)
    }
}
pub fn wizard_view<S: Into<String>>(wizard: &Wizard, pages: Vec<(S, View)>) -> View {
    wizard.view(pages)
}
pub fn wizard_navigation(wizard: &Wizard) -> View {
    wizard.navigation()
}
/// Compatibility for positional descriptions. Prefer stable IDs for changing steps.
pub fn wizard(step: &State<usize>, pages: Vec<View>) -> View {
    let count = pages.len();
    let read = step.clone();
    let write = step.clone();
    let model = Wizard::bound(
        Binding::new(
            move || {
                if count == 0 {
                    String::new()
                } else {
                    read.get().min(count - 1).to_string()
                }
            },
            move |id| {
                if let Ok(index) = id.parse() {
                    write.set(index)
                }
            },
        ),
        (0..count)
            .map(|i| WizardStep::new(i.to_string(), format!("Page {}", i + 1)))
            .collect(),
    )
    .expect("generated wizard IDs are unique");
    model.view(
        pages
            .into_iter()
            .enumerate()
            .map(|(i, body)| (i.to_string(), body))
            .collect(),
    )
}
