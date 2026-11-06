//! The common application path owns its windows and supplies callback context.
//! Callers do not need reference counting or self-referential callback captures.
use super::*;

type WindowDraw = Rc<dyn Fn(&Surface, &Frame<'_>)>;
type WindowEvent = Rc<dyn Fn(&Surface, &Event)>;
type WindowCloseRequest = Rc<dyn Fn(&Surface) -> bool>;
type WindowClosed = Rc<dyn Fn(&Surface)>;
type WindowCommand = Rc<dyn Fn(&Surface, Command)>;
type WindowView = Rc<dyn Fn(&Surface) -> crate::ui::View>;

/// Configure a window before creating its native resources.
/// `Application::open_window` owns and shows it; `build` returns an owning Window
/// for callers that prefer explicit lifetime management.
pub struct WindowBuilder {
    options: WindowOptions,
    appearance: Appearance,
    theme_toggle: bool,
    fullscreen_button: bool,
    draw: Option<WindowDraw>,
    event: Option<WindowEvent>,
    close_request: Option<WindowCloseRequest>,
    closed: Option<WindowClosed>,
    menu: Option<(Vec<MenuItem>, WindowCommand)>,
    view: Option<WindowView>,
}

impl Window {
    pub fn builder(title: impl Into<String>) -> WindowBuilder {
        WindowBuilder {
            options: WindowOptions {
                title: title.into(),
                ..Default::default()
            },
            appearance: Appearance::System,
            theme_toggle: false,
            fullscreen_button: false,
            draw: None,
            event: None,
            close_request: None,
            closed: None,
            menu: None,
            view: None,
        }
    }
}

impl WindowBuilder {
    /// Mount a reactive component tree. Drawing, input, IME and accessibility are wired internally.
    pub fn view(mut self, builder: impl Fn() -> crate::ui::View + 'static) -> Self {
        self.view = Some(Rc::new(move |_| builder()));
        self
    }
    /// Build components with their window context, for actions such as native sheets.
    pub fn view_with_surface(
        mut self,
        builder: impl Fn(&Surface) -> crate::ui::View + 'static,
    ) -> Self {
        self.view = Some(Rc::new(builder));
        self
    }
    pub fn size(mut self, width: f64, height: f64) -> Self {
        self.options.size = Size::new(width, height);
        self
    }
    pub fn minimum_size(mut self, width: f64, height: f64) -> Self {
        self.options.minimum_size = Size::new(width, height);
        self
    }
    pub fn resizable(mut self, value: bool) -> Self {
        self.options.resizable = value;
        self
    }
    pub fn floating(mut self, value: bool) -> Self {
        self.options.floating = value;
        self
    }
    pub fn appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = appearance;
        self
    }
    /// Install a native sun/moon button. Drawing can
    /// read `window.effective_appearance()`; theme changes request redraw automatically.
    pub fn theme_toggle(mut self, initial: Appearance) -> Self {
        self.theme_toggle = true;
        self.appearance = initial;
        self
    }
    /// Add an explicit full-screen titlebar button beside other accessories.
    /// The backend owns its icon, state and transition handling. No menu is added.
    pub fn full_screen_button(mut self) -> Self {
        self.fullscreen_button = true;
        self
    }
    /// Draw through `frame.canvas()` in logical window points. The backend handles
    /// Skia submission, backing scale and presentation after this callback returns.
    pub fn on_draw(mut self, draw: impl Fn(&Surface, &Frame<'_>) + 'static) -> Self {
        self.draw = Some(Rc::new(draw));
        self
    }
    pub fn on_event(mut self, event: impl Fn(&Surface, &Event) + 'static) -> Self {
        self.event = Some(Rc::new(event));
        self
    }
    pub fn on_close_request(mut self, request: impl Fn(&Surface) -> bool + 'static) -> Self {
        self.close_request = Some(Rc::new(request));
        self
    }
    pub fn on_closed(mut self, closed: impl Fn(&Surface) + 'static) -> Self {
        self.closed = Some(Rc::new(closed));
        self
    }
    /// Install exactly the menu trees declared by the application, alongside Quit.
    pub fn menu(
        mut self,
        items: Vec<MenuItem>,
        command: impl Fn(&Surface, Command) + 'static,
    ) -> Self {
        self.menu = Some((items, Rc::new(command)));
        self
    }
    /// Create a hidden, caller-owned window. Dropping it closes it.
    pub fn build(self, app: &Application) -> Result<Window> {
        self.build_in(&app.state)
    }

    fn build_in(self, app: &Rc<AppState>) -> Result<Window> {
        let window = Window {
            surface: Surface {
                state: create_surface(app, self.options, None)?,
            },
        };
        window.set_appearance(self.appearance);
        if self.theme_toggle {
            window.set_titlebar_theme_toggle(
                window.effective_appearance() == Appearance::Light,
                |_| {},
            );
            // Preserve system inheritance until the user makes an explicit theme choice.
            if self.appearance == Appearance::System {
                window.set_appearance(Appearance::System);
            }
        }
        if self.fullscreen_button {
            window.set_titlebar_fullscreen_button(true);
        }
        if let Some((items, command)) = self.menu {
            let weak = window.downgrade();
            window.set_menu(items, move |id| {
                if let Some(surface) = weak.upgrade() {
                    command(&surface, id);
                }
            })?;
        }
        let on_draw = self.draw.map(|draw| {
            let weak = window.downgrade();
            Rc::new(move |frame: &Frame<'_>| {
                if let Some(window) = weak.upgrade() {
                    draw(&window, frame);
                }
            }) as DrawHandler
        });
        let on_event = self.event.map(|event| {
            let weak = window.downgrade();
            Rc::new(move |value: &Event| {
                if let Some(window) = weak.upgrade() {
                    event(&window, value);
                }
            }) as EventHandler
        });
        let on_close_request = self.close_request.map(|request| {
            let weak = window.downgrade();
            Rc::new(move || weak.upgrade().is_none_or(|window| request(&window)))
                as Rc<dyn Fn() -> bool>
        });
        let on_closed = self.closed.map(|closed| {
            let weak = window.downgrade();
            Rc::new(move || {
                if let Some(window) = weak.upgrade() {
                    closed(&window);
                }
            }) as Rc<dyn Fn()>
        });
        window.set_callbacks(Callbacks {
            on_draw,
            on_event,
            on_close_request,
            on_closed,
        });
        if let Some(view) = self.view {
            window.set_view_with_surface(move |surface| view(surface))?;
        }
        Ok(window)
    }
}

impl Application {
    pub fn named(name: impl Into<String>) -> Result<Self> {
        Self::new(ApplicationOptions {
            name: name.into(),
            ..Default::default()
        })
    }
    /// Create and show a window owned by this application. The returned handle
    /// controls it; discarding the handle does not close the window.
    pub fn open_window(&self, builder: WindowBuilder) -> Result<Surface> {
        open_window(&self.state, builder)
    }
    /// Open the first window and reuse the factory for explicit new-window requests
    /// and Dock reopen. This does not install a New Window menu or shortcut.
    /// The application retains each window until it closes. Closing the last
    /// top-level window quits unless set_quit_on_last_window_close(false) is set.
    pub fn run_with_windows(&self, factory: impl Fn() -> WindowBuilder + 'static) -> Result<()> {
        if self.state.running.get() {
            return Err(error("Application::run cannot be nested"));
        }
        let factory: WindowFactory = Rc::new(factory);
        self.open_window(factory())?;
        self.state.window_factory.replace(Some(factory));
        self.run()
    }
    /// Schedule orderly quit (including close vetoes) without capturing the application.
    pub fn quit_after(&self, delay: Duration) {
        let weak = Rc::downgrade(&self.state);
        self.post_after(delay, move || {
            if let Some(app) = weak.upgrade() {
                quit(&app);
            }
        });
    }
}

pub(super) fn open_window(app: &Rc<AppState>, builder: WindowBuilder) -> Result<Surface> {
    let window = builder.build_in(app)?;
    let surface = window.surface.clone();
    app.managed_windows.borrow_mut().push(window);
    surface.show();
    Ok(surface)
}

impl Surface {
    /// Ask to create a window using the application's registered factory/callback.
    /// The application chooses whether to expose this in a menu or shortcut.
    pub fn request_new_window(&self) {
        if !self.is_open() {
            return;
        }
        if let Some(app) = self.state.app.upgrade() {
            guarded(&app, || new_window(&app));
        }
    }
    /// Request a user-initiated close, honoring the window's close veto and sheets.
    pub fn request_close(&self) {
        if self.state.modal.get() {
            if let Some(app) = self.state.app.upgrade() {
                guarded(&app, || request_surface_close(&self.state));
            }
        } else if self.is_open() {
            self.state.native.performClose(None);
        }
    }
    /// Toggle native appearance, synchronize the titlebar button and redraw.
    pub fn toggle_theme(&self) {
        if !self.is_open() {
            return;
        }
        let light = self.effective_appearance() != Appearance::Light;
        self.set_titlebar_theme_toggle_state(light);
        let changed = self.state.theme_changed.borrow().clone();
        if let (Some(changed), Some(app)) = (changed, self.state.app.upgrade()) {
            guarded(&app, || changed(light));
        }
    }
}
