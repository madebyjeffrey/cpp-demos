use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Fragment,
    Column,
    Row,
    Label,
    Button,
}

/// Immutable description. Modifiers use copy-on-write and never edit another clone.
#[derive(Clone)]
pub struct View(pub(crate) Rc<Description>);
#[derive(Clone)]
pub(crate) struct Description {
    pub kind: Kind,
    pub key: Option<String>,
    pub text: String,
    pub children: Vec<View>,
    pub action: Option<Rc<dyn Fn()>>,
}
impl View {
    fn new(kind: Kind, text: String, children: Vec<View>) -> Self {
        Self(Rc::new(Description {
            kind,
            key: None,
            text,
            children,
            action: None,
        }))
    }
    pub fn fragment(children: Vec<Self>) -> Self {
        Self::new(Kind::Fragment, String::new(), children)
    }
    pub fn column(children: Vec<Self>) -> Self {
        Self::new(Kind::Column, String::new(), children)
    }
    pub fn row(children: Vec<Self>) -> Self {
        Self::new(Kind::Row, String::new(), children)
    }
    pub fn label(text: impl Into<String>) -> Self {
        Self::new(Kind::Label, text.into(), Vec::new())
    }
    pub fn button(text: impl Into<String>, action: impl Fn() + 'static) -> Self {
        let mut view = Self::new(Kind::Button, text.into(), Vec::new());
        Rc::make_mut(&mut view.0).action = Some(Rc::new(action));
        view
    }
    pub fn key(mut self, key: impl ToString) -> Self {
        Rc::make_mut(&mut self.0).key = Some(key.to_string());
        self
    }
}
