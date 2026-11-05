use std::{collections::HashSet, fmt, rc::Rc, time::Duration};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error(pub String);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}
impl Size {
    pub const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
impl Point {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub command: bool,
}
impl Modifiers {
    pub const COMMAND: Self = Self {
        shift: false,
        control: false,
        alt: false,
        command: true,
    };
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Key {
    #[default]
    Unknown,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    Escape,
    Enter,
    Tab,
    Space,
    Backspace,
    DeleteForward,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerKind {
    Move,
    Down,
    Up,
    Scroll,
    Enter,
    Leave,
}
#[derive(Clone, Debug, PartialEq)]
pub struct PointerEvent {
    pub kind: PointerKind,
    pub position: Point,
    pub button: i32,
    pub scroll_x: f64,
    pub scroll_y: f64,
    pub modifiers: Modifiers,
    pub click_count: u32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct KeyEvent {
    pub key: Key,
    pub down: bool,
    pub repeat: bool,
    pub modifiers: Modifiers,
    pub native_code: u16,
    pub text: String,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Pointer(PointerEvent),
    Key(KeyEvent),
    Modifiers(Modifiers),
    Focus(bool),
    Resize {
        logical_size: Size,
        pixel_size: Size,
        scale: f64,
    },
    AppearanceChanged(Appearance),
    ReducedMotionChanged(bool),
    /// A native full-screen transition completed.
    FullscreenChanged(bool),
}
#[derive(Clone, Copy, Debug)]
pub struct FrameInfo {
    pub logical_size: Size,
    pub pixel_size: Size,
    pub scale: f64,
    pub elapsed: Duration,
    pub delta: Duration,
    pub frame_number: u64,
    /// Render and present inside the native window sizing transaction.
    pub sizing_transaction: bool,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Appearance {
    #[default]
    System,
    Light,
    Dark,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Cursor {
    #[default]
    Arrow,
    Text,
    ResizeHorizontal,
    ResizeVertical,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogResult {
    Accepted,
    Cancelled,
}

#[derive(Clone, Debug)]
pub struct WindowOptions {
    pub title: String,
    pub size: Size,
    pub minimum_size: Size,
    pub resizable: bool,
    pub floating: bool,
}
impl Default for WindowOptions {
    fn default() -> Self {
        Self {
            title: "SCARS window".into(),
            size: Size::new(1100., 800.),
            minimum_size: Size::new(480., 360.),
            resizable: true,
            floating: false,
        }
    }
}
impl WindowOptions {
    pub(crate) fn validate(&self) -> Result<()> {
        if !valid_size(self.size, false) || !valid_size(self.minimum_size, true) {
            return Err(Error(
                "Window dimensions must be finite; size positive and minima nonnegative".into(),
            ));
        }
        if self.size.width < self.minimum_size.width || self.size.height < self.minimum_size.height
        {
            return Err(Error(
                "Initial content size is smaller than minimum size".into(),
            ));
        }
        Ok(())
    }
}
pub(crate) fn valid_size(s: Size, zero: bool) -> bool {
    s.width.is_finite()
        && s.height.is_finite()
        && if zero {
            s.width >= 0. && s.height >= 0.
        } else {
            s.width > 0. && s.height > 0.
        }
}

pub type ErrorHandler = Rc<dyn Fn(&Error)>;
pub type EventHandler = Rc<dyn Fn(&Event)>;
pub type CommandHandler = Rc<dyn Fn(Command)>;
pub type ThemeHandler = Rc<dyn Fn(bool)>;

pub struct ApplicationOptions {
    pub name: String,
    pub on_error: Option<ErrorHandler>,
    pub on_new_window: Option<Rc<dyn Fn()>>,
    /// Quit after the last top-level window closes. Set false for a background app.
    pub quit_on_last_window_close: bool,
}
impl Default for ApplicationOptions {
    fn default() -> Self {
        Self {
            name: String::new(),
            on_error: None,
            on_new_window: None,
            quit_on_last_window_close: true,
        }
    }
}

pub type Command = u32;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shortcut {
    pub key: Key,
    pub modifiers: Modifiers,
}
impl Default for Shortcut {
    fn default() -> Self {
        Self {
            key: Key::Unknown,
            modifiers: Modifiers::COMMAND,
        }
    }
}
impl Shortcut {
    pub const fn command(key: Key) -> Self {
        Self {
            key,
            modifiers: Modifiers::COMMAND,
        }
    }
}
#[derive(Clone, Debug)]
pub struct MenuItem {
    pub command: Command,
    pub label: String,
    pub shortcut: Shortcut,
    pub enabled: bool,
    pub checked: bool,
    pub separator: bool,
    pub children: Vec<MenuItem>,
}
impl MenuItem {
    pub fn action(command: Command, label: impl Into<String>, shortcut: Shortcut) -> Self {
        Self {
            command,
            label: label.into(),
            shortcut,
            enabled: true,
            checked: false,
            separator: false,
            children: vec![],
        }
    }
    pub fn submenu(label: impl Into<String>, children: Vec<Self>) -> Self {
        Self {
            command: 0,
            label: label.into(),
            shortcut: Shortcut::default(),
            enabled: true,
            checked: false,
            separator: false,
            children,
        }
    }
    pub fn divider() -> Self {
        Self {
            separator: true,
            ..Self::submenu("", vec![])
        }
    }
}
/// Validate atomically before installing a window's menu.
pub fn validate_menu(items: &[MenuItem]) -> Result<()> {
    fn visit(
        items: &[MenuItem],
        top: bool,
        ids: &mut HashSet<Command>,
        depth: usize,
    ) -> Result<()> {
        if depth > 32 {
            return Err(Error("Menu nesting exceeds 32 levels".into()));
        }
        for item in items {
            if item.separator {
                if top || item.command != 0 || !item.children.is_empty() {
                    return Err(Error("Invalid menu separator".into()));
                }
            } else if !item.children.is_empty() || top {
                if item.command != 0 || item.children.is_empty() {
                    return Err(Error(
                        "Menu bar entries must be nonempty submenus with no command".into(),
                    ));
                }
                visit(&item.children, false, ids, depth + 1)?;
            } else if item.command == 0 || !ids.insert(item.command) {
                return Err(Error("Menu actions require unique nonzero commands".into()));
            }
        }
        Ok(())
    }
    visit(items, true, &mut HashSet::new(), 0)
}
pub(crate) fn command_enabled(items: &[MenuItem], command: Command) -> bool {
    items.iter().any(|i| {
        i.enabled && (i.command == command && command != 0 || command_enabled(&i.children, command))
    })
}
pub(crate) fn command_mut(items: &mut [MenuItem], command: Command) -> Option<&mut MenuItem> {
    for item in items {
        if item.command == command && command != 0 {
            return Some(item);
        }
        if let Some(found) = command_mut(&mut item.children, command) {
            return Some(found);
        }
    }
    None
}
/// Native text ranges are UTF-16 code units; content strings are UTF-8.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextRange {
    pub start: usize,
    pub length: usize,
}
#[derive(Clone, Debug)]
pub struct TextInputState {
    pub text: String,
    pub selection: TextRange,
    pub marked: Option<TextRange>,
    pub editable: bool,
    pub multiline: bool,
    pub can_undo: bool,
    pub can_redo: bool,
    pub secure: bool,
    pub active: bool,
}
impl Default for TextInputState {
    fn default() -> Self {
        Self {
            text: String::new(),
            selection: TextRange::default(),
            marked: None,
            editable: false,
            multiline: false,
            can_undo: false,
            can_redo: false,
            secure: false,
            active: true,
        }
    }
}
/// Editing storage and text layout belong to the renderer, not the window manager.
pub trait TextInputClient {
    fn state(&self) -> Option<TextInputState>;
    fn insert(&self, text: String, replacement: Option<TextRange>);
    fn mark(&self, text: String, selection: TextRange, replacement: Option<TextRange>);
    fn unmark(&self, commit: bool);
    fn command(&self, selector: &str) -> bool;
    fn rectangle(&self, range: TextRange) -> Rect;
    fn index(&self, point: Point) -> usize;
    fn intercept_key(&self, _event: &KeyEvent) -> bool {
        false
    }
    fn deactivate(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn menu_validation_and_disabled_ancestors() {
        let mut menu = vec![MenuItem::submenu(
            "File",
            vec![MenuItem::action(1, "Open", Shortcut::default())],
        )];
        assert!(validate_menu(&menu).is_ok());
        assert!(command_enabled(&menu, 1));
        menu[0].enabled = false;
        assert!(!command_enabled(&menu, 1));
        menu[0]
            .children
            .push(MenuItem::action(1, "Duplicate", Shortcut::default()));
        assert!(validate_menu(&menu).is_err());
        menu[0].children.pop();
        menu[0].children[0].command = 0;
        assert!(validate_menu(&menu).is_err());
        assert!(validate_menu(&[MenuItem::divider()]).is_err());
    }
    #[test]
    fn dimension_validation() {
        let mut o = WindowOptions::default();
        assert!(o.validate().is_ok());
        o.size.width = f64::NAN;
        assert!(o.validate().is_err());
        o.size.width = 1.;
        assert!(o.validate().is_err());
        o.minimum_size = Size::new(0., 0.);
        assert!(o.validate().is_ok());
    }
}
