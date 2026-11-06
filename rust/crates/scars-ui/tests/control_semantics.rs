use scars_ui::*;
use std::rc::Rc;
struct Metrics;
impl Measure for Metrics {
    fn text(&self, s: &str, _: TextStyle, _: f64) -> TextLayout {
        TextLayout {
            size: Size::new(s.len() as f64 * 7., 18.),
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
    h.layout(Size::new(600., 500.), &Metrics).unwrap();
}
fn keyed(h: &Host, key: &str) -> Node {
    h.snapshot()
        .unwrap()
        .into_iter()
        .find(|n| n.key.as_deref() == Some(key))
        .unwrap()
}
#[test]
fn fields_expose_live_validation_and_help_independent_of_tooltips() {
    let value = State::new(4i64);
    let number = Numeric::new(value.binding(), 0, 10, 1);
    let text = State::new(String::new());
    let h = Host::new({
        let number = number.clone();
        move || view! {column{embed(View::numeric_field("Count",&number));text_field("Name",bind:text,key:"name",accessible_help:"Use your full name",tooltip:"Short hint",validation:Validation{status:ValidationStatus::Incomplete,message:Some("A name is required".into()),dirty:true});}}
    });
    layout(&h);
    let name = keyed(&h, "name");
    assert_eq!(name.help, "Use your full name");
    assert_eq!(
        name.validation.unwrap().status,
        ValidationStatus::Incomplete
    );
    number.draft.binding().set("-".into());
    layout(&h);
    let nodes = h.snapshot().unwrap();
    let field = nodes
        .iter()
        .find(|n| n.kind == Kind::TextField && n.label == "Count")
        .unwrap();
    assert_eq!(
        field.validation.as_ref().unwrap().status,
        ValidationStatus::Incomplete
    );
    let stepper = nodes.iter().find_map(|n| n.stepper.as_ref()).unwrap();
    assert!(!stepper.can_increment && !stepper.can_decrement);
    assert_eq!(value.get(), 4);
    number.cancel();
    value.set(10);
    layout(&h);
    let nodes = h.snapshot().unwrap();
    let step = nodes.iter().find_map(|n| n.stepper.as_ref()).unwrap();
    assert!(!step.can_increment && step.can_decrement);
}
#[test]
fn exact_boolean_and_tristate_assignments_respect_readonly_disabled_and_removed_controls() {
    let checked = State::new(CheckState::Indeterminate);
    let toggle = State::new(false);
    let enabled = State::new(true);
    let readonly = State::new(false);
    let visible = State::new(true);
    let h = Host::new({
        let c = checked.clone();
        let t = toggle.clone();
        let e = enabled.clone();
        let r = readonly.clone();
        let v = visible.clone();
        move || {
            if v.get() {
                View::column(vec![
                    View::tri_checkbox("Check", c.binding()).key("check"),
                    View::toggle("Toggle", t.binding()).key("toggle"),
                ])
                .enabled(e.get())
                .read_only(r.get())
            } else {
                View::label("Removed")
            }
        }
    });
    layout(&h);
    let check = keyed(&h, "check").id;
    let t = keyed(&h, "toggle").id;
    assert!(h.set_selected(check, false).unwrap());
    assert_eq!(checked.get(), CheckState::Unchecked);
    assert!(h.set_check_state(check, CheckState::Indeterminate).unwrap());
    assert!(h.set_selected(t, true).unwrap());
    assert!(toggle.get());
    enabled.set(false);
    assert!(!h.set_selected(t, false).unwrap());
    assert!(!h.set_check_state(check, CheckState::Checked).unwrap());
    enabled.set(true);
    // Readonly belongs to the actual control; layout containers do not inherit it.
    let read = Host::new({
        let c = checked.clone();
        move || {
            View::tri_checkbox("Readonly", c.binding())
                .read_only(true)
                .key("read")
        }
    });
    layout(&read);
    assert!(
        !read
            .set_check_state(keyed(&read, "read").id, CheckState::Checked)
            .unwrap()
    );
    visible.set(false);
    layout(&h);
    assert!(!h.set_selected(t, false).unwrap());
    assert!(toggle.get());
}
#[test]
fn radio_assignment_cannot_deselect_the_only_choice_or_trigger_unrelated_buttons() {
    let choice = State::new("a".to_owned());
    let clicks = State::new(0);
    let h = Host::new({
        let choice = choice.clone();
        let clicks = clicks.clone();
        move || {
            View::column(vec![
                View::radio_group(
                    "Choice",
                    choice.binding(),
                    vec![OptionItem::new("a", "A"), OptionItem::new("b", "B")],
                ),
                View::button("Run", clicks.action(|n| n.update(|v| *v += 1))).key("run"),
            ])
        }
    });
    layout(&h);
    assert!(!h.set_selected(keyed(&h, "a").id, false).unwrap());
    assert_eq!(choice.get(), "a");
    assert!(h.set_selected(keyed(&h, "b").id, true).unwrap());
    assert_eq!(choice.get(), "b");
    assert!(!h.set_selected(keyed(&h, "run").id, true).unwrap());
    assert_eq!(clicks.get(), 0);
}
#[test]
fn show_menu_is_idempotent_and_cancel_walks_submenus_before_dismissing() {
    let menu = Menu::new();
    let h = Host::new({
        let menu = menu.clone();
        move || {
            View::layer(vec![
                menu.button("Actions", "actions"),
                menu.view(
                    "actions",
                    vec![
                        MenuAction::submenu("More", vec![MenuAction::new("Run", || {})])
                            .key("more"),
                    ],
                ),
            ])
        }
    });
    layout(&h);
    let trigger = keyed(&h, "actions").id;
    assert!(h.show_menu(trigger).unwrap());
    layout(&h);
    let scope = h.active_modal().unwrap();
    assert!(h.show_menu(trigger).unwrap());
    assert_eq!(h.active_modal(), Some(scope));
    assert!(keyed(&h, "actions").menu.unwrap().expanded);
    let more = keyed(&h, "more").id;
    assert!(h.show_menu(more).unwrap());
    layout(&h);
    assert_eq!(
        h.snapshot()
            .unwrap()
            .iter()
            .find(|n| n.menu_root)
            .unwrap()
            .menu
            .as_ref()
            .unwrap()
            .path,
        vec!["more"]
    );
    assert!(h.cancel_presentation(scope).unwrap());
    layout(&h);
    assert!(menu.open.get());
    assert!(h.cancel_presentation(scope).unwrap());
    layout(&h);
    assert!(!menu.open.get());
    assert!(!h.cancel_presentation(scope).unwrap());
}
#[test]
fn native_style_cancellation_cannot_dismiss_an_inactive_owner_or_ignore_escape_policy() {
    let outer = Popup::new();
    let inner = Popup::new();
    outer.show();
    let h = Host::new({
        let outer = outer.clone();
        let inner = inner.clone();
        move || {
            View::layer(vec![
                outer.modal("Outer", View::label("Body")),
                inner.modal("Inner", View::label("Body")),
            ])
        }
    });
    layout(&h);
    let owner = h.active_modal().unwrap();
    inner.show();
    layout(&h);
    let active = h.active_modal().unwrap();
    assert_ne!(owner, active);
    assert!(!h.cancel_presentation(owner).unwrap());
    assert!(h.cancel_presentation(active).unwrap());
    layout(&h);
    assert!(outer.open.get());
    let protected = Popup::new().options(OverlayOptions {
        dismiss_on_escape: false,
        ..Default::default()
    });
    protected.show();
    let h = Host::new({
        let protected = protected.clone();
        move || protected.modal("Required", View::label("Body"))
    });
    layout(&h);
    assert!(!h.cancel_presentation(h.active_modal().unwrap()).unwrap());
    assert!(protected.open.get());
}
