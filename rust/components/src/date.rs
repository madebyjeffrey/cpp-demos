//! Validated date drafts and one roving, non-committing calendar cursor.
use crate::locale::{ActionLabel, DateProblem};
use crate::{
    Binding, Date, Draft, EditLocale, Error, Host, Key, KeyEvent, NodeId, State, Validation,
    ValidationStatus, View,
};
use std::rc::Rc;
type Disabled = Rc<dyn Fn(Date) -> bool>;
type Validator = Rc<dyn Fn(Date) -> Result<(), Error>>;
#[derive(Clone)]
struct Rules {
    min: Date,
    max: Date,
    locale: EditLocale,
    disabled: Option<Disabled>,
    validate: Option<Validator>,
}
impl Rules {
    fn check(&self, date: Date) -> Validation {
        let (status, message) = if !self.locale.date_supported(date) {
            (
                ValidationStatus::Invalid,
                Some(
                    self.locale
                        .date_problem(DateProblem::UnsupportedYear)
                        .into(),
                ),
            )
        } else if date < self.min || date > self.max {
            (
                ValidationStatus::OutOfRange,
                Some(self.locale.date_range_error(self.min, self.max)),
            )
        } else {
            crate::state::read_only(|| {
                if self.disabled.as_ref().is_some_and(|f| f(date)) {
                    (
                        ValidationStatus::Invalid,
                        Some(self.locale.date_problem(DateProblem::Unavailable).into()),
                    )
                } else if let Some(error) = self.validate.as_ref().and_then(|f| f(date).err()) {
                    (ValidationStatus::Invalid, Some(error.0))
                } else {
                    (ValidationStatus::Valid, None)
                }
            })
        };
        Validation {
            status,
            message,
            dirty: false,
        }
    }
    fn parse(&self, text: &str) -> Result<Date, String> {
        let date = self.locale.date_input(text)?;
        let validation = self.check(date);
        if validation.status == ValidationStatus::Valid {
            Ok(date)
        } else {
            Err(validation.message.unwrap())
        }
    }
    fn status(&self, text: &str) -> ValidationStatus {
        if text.is_empty() {
            return ValidationStatus::Incomplete;
        }
        match self.locale.date_input(text) {
            Ok(date) => self.check(date).status,
            Err(_) => ValidationStatus::Invalid,
        }
    }
    fn clamp(&self, date: Date) -> Date {
        let (min, max) = if self.locale == EditLocale::ASCII {
            (self.min, self.max)
        } else {
            (
                self.min.max(Date::new(1, 1, 1).unwrap()),
                self.max.min(Date::new(9999, 12, 31).unwrap()),
            )
        };
        date.max(min).min(max)
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct Page {
    base: Date,
    revision: Option<u64>,
    month: Date,
    cursor: Date,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CalendarInfo {
    pub month: Date,
    pub cursor: Date,
    pub selected: Date,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CalendarDay {
    pub date: Date,
    pub current: bool,
    pub selected: bool,
    pub navigable: bool,
}
#[derive(Clone)]
pub struct DatePicker {
    id: Rc<()>,
    value: Binding<Date>,
    rules: Rules,
    page: State<Page>,
    pub draft: Draft<Date>,
    read_only: bool,
    commit_on_blur: bool,
    submit: Option<Rc<dyn Fn()>>,
    cancel: Option<Rc<dyn Fn()>>,
}
impl DatePicker {
    pub fn new(value: Binding<Date>) -> Self {
        let base = value.get();
        let revision = value.revision();
        Self {
            id: Rc::new(()),
            draft: Draft::new(value.clone(), crate::values::parse_date, |v| v.to_string()),
            value,
            rules: Rules {
                min: Date::MIN,
                max: Date::MAX,
                locale: EditLocale::ASCII,
                disabled: None,
                validate: None,
            },
            page: State::new(Page {
                base,
                revision,
                month: base.first_of_month(),
                cursor: base,
            }),
            read_only: false,
            commit_on_blur: true,
            submit: None,
            cancel: None,
        }
        .configured()
    }
    fn configured(mut self) -> Self {
        assert!(self.rules.min <= self.rules.max, "Invalid date bounds");
        if self.rules.locale != EditLocale::ASCII {
            assert!(
                self.rules.max >= Date::new(1, 1, 1).unwrap()
                    && self.rules.min <= Date::new(9999, 12, 31).unwrap(),
                "Date bounds do not intersect the locale's supported years"
            );
        }
        let (parse, format, status, model) = (
            self.rules.clone(),
            self.rules.locale.clone(),
            self.rules.clone(),
            self.rules.clone(),
        );
        let draft = Draft::new(
            self.value.clone(),
            move |s| parse.parse(s),
            move |v| format.date(*v),
        )
        .classifiers(move |s| status.status(s), move |v| model.check(*v))
        .read_only(self.read_only);
        self.draft = self.draft.reconfigured(draft).read_only(self.read_only);
        self
    }
    pub fn locale(mut self, locale: EditLocale) -> Self {
        self.rules.locale = locale;
        self.configured()
    }
    pub fn bounds(mut self, min: Date, max: Date) -> Self {
        self.rules.min = min;
        self.rules.max = max;
        self.configured()
    }
    pub fn disabled_dates(mut self, disabled: impl Fn(Date) -> bool + 'static) -> Self {
        self.rules.disabled = Some(Rc::new(disabled));
        self.configured()
    }
    pub fn validate(mut self, validate: impl Fn(Date) -> Result<(), Error> + 'static) -> Self {
        self.rules.validate = Some(Rc::new(validate));
        self.configured()
    }
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self.draft = self.draft.read_only(read_only);
        self
    }
    pub fn commit_on_blur(mut self, commit: bool) -> Self {
        self.commit_on_blur = commit;
        self
    }
    pub fn on_validation(mut self, callback: impl Fn(Validation) + 'static) -> Self {
        self.draft = self.draft.on_validation(callback);
        self
    }
    pub fn on_submit(mut self, callback: impl Fn() + 'static) -> Self {
        self.submit = Some(Rc::new(callback));
        self
    }
    pub fn on_cancel(mut self, callback: impl Fn() + 'static) -> Self {
        self.cancel = Some(Rc::new(callback));
        self
    }
    pub fn validation(&self) -> Validation {
        self.draft.validation()
    }
    fn page(&self) -> Page {
        let page = self.page.get();
        let base = self.value.get();
        let revision = self.value.revision();
        if page.base != base || page.revision != revision {
            let cursor = self.rules.clamp(base);
            Page {
                base,
                revision,
                month: cursor.first_of_month(),
                cursor,
            }
        } else {
            let cursor = self.rules.clamp(page.cursor);
            if cursor != page.cursor {
                Page {
                    cursor,
                    month: cursor.first_of_month(),
                    ..page
                }
            } else {
                page
            }
        }
    }
    pub fn calendar(&self) -> CalendarInfo {
        let page = self.page();
        CalendarInfo {
            month: page.month,
            cursor: page.cursor,
            selected: self.value.get(),
        }
    }
    pub fn commit(&self) -> bool {
        crate::state::batch(|| {
            let accepted =
                self.draft.commit() && self.validation().status == ValidationStatus::Valid;
            if accepted {
                let base = self.value.get();
                let cursor = self.rules.clamp(base);
                self.page.set(Page {
                    base,
                    revision: self.value.revision(),
                    month: cursor.first_of_month(),
                    cursor,
                });
            }
            accepted
        })
    }
    pub fn cancel(&self) {
        crate::state::batch(|| {
            self.draft.cancel();
            let base = self.value.get();
            let cursor = self.rules.clamp(base);
            self.page.set(Page {
                base,
                revision: self.value.revision(),
                month: cursor.first_of_month(),
                cursor,
            });
            if let Some(callback) = &self.cancel {
                callback()
            }
        })
    }
    fn stage(&self, date: Date) -> Option<(Date, Option<u64>)> {
        if self.read_only || self.rules.check(date).status != ValidationStatus::Valid {
            return None;
        }
        let before = (self.value.get(), self.value.revision());
        self.draft.binding().set(self.rules.locale.date(date));
        ((self.value.get(), self.value.revision()) == before).then_some(before)
    }
    pub fn select(&self, date: Date) -> bool {
        crate::state::batch(|| {
            self.stage(date).is_some() && self.commit() && self.value.get() == date
        })
    }
    fn submit(&self) {
        if self.commit()
            && let Some(callback) = &self.submit
        {
            callback()
        }
    }
    fn cursor(&self, date: Date) {
        let cursor = self.rules.clamp(date);
        let base = self.value.get();
        self.page.set(Page {
            base,
            revision: self.value.revision(),
            month: cursor.first_of_month(),
            cursor,
        });
    }
    fn adjacent_month(&self, step: i32) -> Option<Date> {
        let cursor = self.page().cursor;
        cursor
            .first_of_month()
            .checked_add(jiff::Span::new().months(step))
            .ok()
            .map(|m| Date::new(m.year(), m.month(), cursor.day().min(m.days_in_month())).unwrap())
    }
    fn can_browse(&self, step: i32) -> bool {
        self.adjacent_month(step).is_some_and(|d| {
            d.last_of_month() >= self.rules.min
                && d.first_of_month() <= self.rules.max
                && self.rules.locale.date_supported(d)
        })
    }
    fn browse(&self, step: i32) {
        if self.can_browse(step)
            && let Some(date) = self.adjacent_month(step)
        {
            self.cursor(date)
        }
    }
    fn navigate(&self, event: KeyEvent) -> bool {
        let cursor = self.page().cursor;
        let days = match event.key {
            Key::Left => Some(if self.rules.locale.rtl() { 1 } else { -1 }),
            Key::Right => Some(if self.rules.locale.rtl() { -1 } else { 1 }),
            Key::Up => Some(-7),
            Key::Down => Some(7),
            _ => None,
        };
        let target = if let Some(days) = days {
            cursor.checked_add(jiff::Span::new().days(days)).ok()
        } else {
            match event.key {
                Key::PageUp | Key::PageDown => {
                    let step = if event.key == Key::PageUp { -1 } else { 1 }
                        * if event.modifiers.shift { 12 } else { 1 };
                    cursor
                        .first_of_month()
                        .checked_add(jiff::Span::new().months(step))
                        .ok()
                        .map(|m| {
                            Date::new(m.year(), m.month(), cursor.day().min(m.days_in_month()))
                                .unwrap()
                        })
                }
                Key::Home | Key::End => {
                    if event.modifiers.control || event.modifiers.command {
                        Some(if event.key == Key::Home {
                            cursor.first_of_month()
                        } else {
                            cursor.last_of_month()
                        })
                    } else {
                        let offset = (cursor.weekday().to_monday_zero_offset()
                            - self.rules.locale.week_start()
                            + 7)
                            % 7;
                        cursor
                            .checked_add(jiff::Span::new().days(if event.key == Key::Home {
                                -offset
                            } else {
                                6 - offset
                            }))
                            .ok()
                    }
                }
                _ => return false,
            }
        };
        if let Some(target) = target {
            self.cursor(target)
        }
        true
    }
    pub fn view(&self) -> View {
        let month = self.page().month;
        let prev = self.clone();
        let next = self.clone();
        let mut header = vec![
            View::button(
                if self.rules.locale.rtl() {
                    "›"
                } else {
                    "‹"
                },
                move || {
                    prev.browse(-1);
                },
            )
            .width(36)
            .accessible_label(self.rules.locale.action(ActionLabel::PreviousMonth))
            .enabled(self.can_browse(-1)),
            View::label(self.rules.locale.month(month))
                .grow(1.)
                .min_width(0.),
            View::button(
                if self.rules.locale.rtl() {
                    "‹"
                } else {
                    "›"
                },
                move || {
                    next.browse(1);
                },
            )
            .width(36)
            .accessible_label(self.rules.locale.action(ActionLabel::NextMonth))
            .enabled(self.can_browse(1)),
        ];
        if self.rules.locale.rtl() {
            header.reverse();
        }
        let mut children = vec![View::row(header).key("$date-header")];
        let weekdays = self.rules.locale.weekdays();
        let mut labels = (0..7)
            .map(|i| {
                View::label(weekdays[(i + self.rules.locale.week_start() as usize) % 7])
                    .grow(1.)
                    .basis(0)
                    .min_width(0.)
                    .text_align(crate::Align::Center)
            })
            .collect::<Vec<_>>();
        if self.rules.locale.rtl() {
            labels.reverse();
        }
        children.push(View::row(labels).gap(4.).key("$date-weekdays"));
        let offset =
            (month.weekday().to_monday_zero_offset() - self.rules.locale.week_start() + 7) % 7;
        for week in 0..6 {
            let mut cells = Vec::new();
            for day in 0..7 {
                let number = week * 7 + day - offset + 1;
                if number < 1 || number > month.days_in_month() {
                    cells.push(View::label("").grow(1.).basis(0).min_width(0.));
                    continue;
                }
                let date = Date::new(month.year(), month.month(), number).unwrap();
                let validation = self.rules.check(date);
                let mut day = View::button(self.rules.locale.day(number), || {})
                    .grow(1.)
                    .basis(0)
                    .min_width(0.)
                    .enabled(!self.read_only && validation.status == ValidationStatus::Valid)
                    .bold(date == self.value.get())
                    .accessible_label(self.rules.locale.date(date))
                    .tooltip(self.rules.locale.date(date))
                    .accessible_help(validation.message.unwrap_or_default())
                    .key(date.to_string());
                Rc::make_mut(&mut day.0).calendar_day = Some(CalendarDay {
                    date,
                    current: date == self.page().cursor,
                    selected: date == self.value.get(),
                    navigable: date >= self.rules.min
                        && date <= self.rules.max
                        && self.rules.locale.date_supported(date),
                });
                cells.push(day);
            }
            if self.rules.locale.rtl() {
                cells.reverse();
            }
            children.push(View::row(cells).gap(4.).key(format!("$date-week/{week}")));
            if (week + 1) * 7 - offset >= month.days_in_month() {
                break;
            }
        }
        let (submit, cancel, blur, apply, revert) = (
            self.clone(),
            self.clone(),
            self.clone(),
            self.clone(),
            self.clone(),
        );
        let mut fields = vec![
            View::label(self.rules.locale.date_hint()),
            View::text_field(self.rules.locale.date_hint(), self.draft.binding())
                .key("date-field")
                .read_only(self.read_only)
                .accessible_label(self.rules.locale.date_hint())
                .validation(self.validation())
                .on_submit(move || submit.submit())
                .on_cancel(move || cancel.cancel())
                .on_blur(move || {
                    if blur.commit_on_blur {
                        blur.commit();
                    }
                }),
            View::row(vec![
                View::button(self.rules.locale.action(ActionLabel::Apply), move || {
                    apply.submit()
                })
                .enabled(!self.read_only),
                View::button(self.rules.locale.action(ActionLabel::Revert), move || {
                    revert.cancel()
                })
                .enabled(!self.read_only),
            ]),
        ];
        if let Some(error) = self.draft.error() {
            fields.push(View::label(error).wrap(true).foreground(0xffff8787));
        }
        children.push(View::column(fields).key("$date-fields"));
        let mut root = View::panel(children).accessible_label(self.rules.locale.date_hint());
        Rc::make_mut(&mut root.0).calendar = Some(self.clone());
        root
    }
}
impl Host {
    fn calendar_owner(&self, id: NodeId) -> Option<(NodeId, DatePicker)> {
        let mounted = self.0.mounted.borrow();
        let mut current = Some(id);
        while let Some(id) = current {
            if let Some(picker) = mounted
                .descriptions
                .get(&id)
                .and_then(|d| d.calendar.clone())
            {
                return Some((id, picker));
            }
            current = mounted.nodes.iter().find(|n| n.id == id)?.parent;
        }
        None
    }
    pub(crate) fn restore_calendar_focus(&self, prior: Option<NodeId>, old: &crate::host::Mounted) {
        let Some(prior) = prior else { return };
        if old
            .nodes
            .iter()
            .find(|n| n.id == prior)
            .is_none_or(|n| n.calendar_day.is_none())
        {
            return;
        }
        if self.focused().is_some_and(|id| id != prior) {
            return;
        }
        let mut parent = old
            .nodes
            .iter()
            .find(|n| n.id == prior)
            .and_then(|n| n.parent);
        while let Some(id) = parent {
            if let Some(before) = old.descriptions.get(&id).and_then(|d| d.calendar.as_ref()) {
                let m = self.0.mounted.borrow();
                if m.descriptions
                    .get(&id)
                    .and_then(|d| d.calendar.as_ref())
                    .is_none_or(|p| !Rc::ptr_eq(&p.id, &before.id))
                {
                    self.0.interaction.borrow_mut().focus = None;
                    return;
                }
                let target = m
                    .nodes
                    .iter()
                    .find(|n| {
                        n.calendar_day.is_some_and(|d| d.current)
                            && n.focus_enabled()
                            && !n.hidden
                            && !n.layout_hidden
                            && Self::within(&m.nodes, n.id, id)
                            && self.in_scope(n.id)
                    })
                    .map(|n| n.id);
                let mut interaction = self.0.interaction.borrow_mut();
                if target != Some(prior) {
                    interaction.pending_calendar_reveal = target;
                }
                interaction.focus = target;
                return;
            }
            parent = old.nodes.iter().find(|n| n.id == id).and_then(|n| n.parent);
        }
    }
    pub(crate) fn activate_calendar_day(&self, id: NodeId, date: Date) -> Result<bool, Error> {
        let Some((root, picker)) = self.calendar_owner(id) else {
            return Ok(false);
        };
        crate::state::batch(|| {
            let Some(before) = picker.stage(date) else {
                return Ok(false);
            };
            // A validation callback may unmount, disable, replace or modal-block the control.
            self.refresh()?;
            let current = {
                let m = self.0.mounted.borrow();
                m.descriptions
                    .get(&root)
                    .and_then(|d| d.calendar.as_ref())
                    .is_some_and(|p| Rc::ptr_eq(&p.id, &picker.id))
                    && m.nodes.iter().any(|n| {
                        n.id == id
                            && n.enabled
                            && !n.hidden
                            && !n.layout_hidden
                            && n.calendar_day.is_some_and(|d| d.date == date)
                    })
            };
            if !current
                || !self.in_scope(id)
                || (picker.value.get(), picker.value.revision()) != before
            {
                return Ok(false);
            }
            Ok(picker.commit() && picker.value.get() == date)
        })
    }
    pub(crate) fn remember_calendar_focus(&self, id: NodeId) {
        let date = self
            .0
            .mounted
            .borrow()
            .nodes
            .iter()
            .find(|n| n.id == id)
            .and_then(|n| n.calendar_day)
            .map(|d| d.date);
        if let Some(date) = date
            && let Some((_, picker)) = self.calendar_owner(id)
        {
            picker.cursor(date);
        }
    }
    pub(crate) fn calendar_key(&self, id: NodeId, event: KeyEvent) -> Result<Option<bool>, Error> {
        let day = self
            .0
            .mounted
            .borrow()
            .nodes
            .iter()
            .find(|n| n.id == id)
            .and_then(|n| n.calendar_day);
        let Some(day) = day else { return Ok(None) };
        if !day.navigable || !self.in_scope(id) {
            return Ok(Some(false));
        }
        let Some((root, picker)) = self.calendar_owner(id) else {
            return Ok(None);
        };
        if event.key == Key::Escape {
            picker.cancel();
            self.redraw();
            return Ok(Some(true));
        }
        if !picker.navigate(event) {
            return Ok(None);
        }
        let cursor = picker.calendar().cursor;
        self.refresh()?;
        let target = self
            .0
            .mounted
            .borrow()
            .nodes
            .iter()
            .find(|n| {
                n.calendar_day.is_some_and(|d| d.date == cursor)
                    && Self::within(&self.0.mounted.borrow().nodes, n.id, root)
            })
            .map(|n| n.id);
        if let Some(target) = target {
            self.focus(target)?;
        }
        self.redraw();
        Ok(Some(true))
    }
}
