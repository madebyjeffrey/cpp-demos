//! AppKit boundary. Native delegate/view objects hold weak references to Rust state.
//! No RefCell borrow is held while invoking application code or reentering AppKit.

mod builder;
mod callbacks;
mod components;
mod input;
mod rendering;
mod text;
mod view;
use crate::types::*;
use block2::RcBlock;
pub use builder::WindowBuilder;
pub use callbacks::{Callbacks, DrawHandler};

// Native extension interfaces are exported by the selected backend.
pub use objc2 as objc;
use objc2::{
    AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel,
};
use objc2::{
    rc::{Retained, autoreleasepool},
    runtime::{AnyObject, ProtocolObject},
};
pub use objc2_app_kit as app_kit;
use objc2_app_kit::*;
pub use objc2_foundation as foundation;
use objc2_foundation::*;
pub use objc2_metal as metal;
use objc2_metal::*;
pub use objc2_quartz_core as quartz_core;
use objc2_quartz_core::*;
use raw_window_handle::*;
use std::{
    cell::{Cell, OnceCell, RefCell},
    ptr::NonNull,
    rc::{Rc, Weak},
    time::{Duration, Instant},
};
use text::{deactivate_input, sync_secure_input};
use view::ContentView;

type Completion = Rc<dyn Fn(DialogResult)>;
type WindowFactory = Rc<dyn Fn() -> WindowBuilder>;

struct AppState {
    native: Retained<NSApplication>,
    delegate: OnceCell<Retained<AppDelegate>>,
    default_menu: OnceCell<Retained<NSMenu>>,
    motion_observer: OnceCell<Retained<ProtocolObject<dyn NSObjectProtocol>>>,
    options: ApplicationOptions,
    surfaces: RefCell<Vec<Weak<SurfaceState>>>,
    managed_windows: RefCell<Vec<Window>>,
    window_factory: RefCell<Option<WindowFactory>>,
    renderer: rendering::Renderer,
    device: Retained<ProtocolObject<dyn MTLDevice>>,
    queue: Retained<ProtocolObject<dyn MTLCommandQueue>>,
    epoch: Instant,
    alive: Cell<bool>,
    running: Cell<bool>,
    quitting: Cell<bool>,
    quit_on_last_window_close: Cell<bool>,
    last_window_quit_pending: Cell<bool>,
    mtm: MainThreadMarker,
}

struct SurfaceState {
    components: RefCell<Option<Rc<components::Mounted>>>,
    app: Weak<AppState>,
    parent: Weak<SurfaceState>,
    children: RefCell<Vec<Weak<SurfaceState>>>,
    native: Retained<NSWindow>,
    view: Retained<ContentView>,
    delegate: Retained<SurfaceDelegate>,
    layer: Retained<CAMetalLayer>,
    display_link: OnceCell<Retained<CADisplayLink>>,
    callbacks: RefCell<Callbacks>,
    menu: RefCell<Option<Retained<NSMenu>>>,
    menu_items: RefCell<Vec<MenuItem>>,
    on_command: RefCell<Option<CommandHandler>>,
    theme: RefCell<Option<TitlebarButton>>,
    fullscreen_button: RefCell<Option<TitlebarButton>>,
    theme_changed: RefCell<Option<ThemeHandler>>,
    text_input: RefCell<Option<Rc<dyn TextInputClient>>>,
    completion: RefCell<Option<Completion>>,
    open: Cell<bool>,
    modal: Cell<bool>,
    dirty: Cell<bool>,
    animating: Cell<bool>,
    failed: Cell<bool>,
    rendering: Cell<bool>,
    redraw_timer: RefCell<Option<Retained<NSTimer>>>,
    synced_size: Cell<Size>,
    synced_scale: Cell<f64>,
    previous: Cell<Option<Instant>>,
    frame: Cell<u64>,
    unavailable_frames: Cell<u32>,
    cursor: Cell<Cursor>,
    secure_input: Cell<bool>,
    appearance: Cell<Appearance>,
    reduced_motion: Cell<bool>,
    fullscreen_capable: bool,
    fullscreen_transition: Cell<bool>,
    fullscreen_notified: Cell<bool>,
}
struct TitlebarButton {
    accessory: Retained<NSTitlebarAccessoryViewController>,
    button: Retained<NSButton>,
}

/// Owns the event loop and shared Metal/Skia renderer. Main thread only.
pub struct Application {
    state: Rc<AppState>,
}
/// Owning window. Dropping it forcibly closes its native window and child dialogs.
pub struct Window {
    surface: Surface,
}
/// Owning child dialog; may be modeless or presented as a parent-modal sheet.
pub struct Dialog {
    surface: Surface,
}
/// Cloneable control handle. The owning Window/Dialog still determines closure.
#[derive(Clone)]
pub struct Surface {
    state: Rc<SurfaceState>,
}
#[derive(Clone)]
pub struct WeakSurface {
    state: Weak<SurfaceState>,
}
impl WeakSurface {
    pub fn upgrade(&self) -> Option<Surface> {
        self.state.upgrade().map(|state| Surface { state })
    }
}
impl std::ops::Deref for Window {
    type Target = Surface;
    fn deref(&self) -> &Surface {
        &self.surface
    }
}
impl std::ops::Deref for Dialog {
    type Target = Surface;
    fn deref(&self) -> &Surface {
        &self.surface
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        close_surface(&self.state, DialogResult::Cancelled);
    }
}
impl Drop for Dialog {
    fn drop(&mut self) {
        close_surface(&self.state, DialogResult::Cancelled);
    }
}

/// A Skia canvas in logical window points, backed by the current Metal drawable.
/// The window manager flushes GPU work and presents after the callback returns.
pub struct Frame<'a> {
    pub info: FrameInfo,
    canvas: &'a skia_safe::Canvas,
    device: &'a ProtocolObject<dyn MTLDevice>,
    queue: &'a ProtocolObject<dyn MTLCommandQueue>,
    drawable: &'a ProtocolObject<dyn CAMetalDrawable>,
}
impl Frame<'_> {
    /// Draw in logical points with a top-left origin. Backing scale is applied
    /// automatically; the canvas and its drawable are valid only for this callback.
    pub fn canvas(&self) -> &skia_safe::Canvas {
        self.canvas
    }
    /// Native bridge for direct Metal rendering. Do not mix it with Skia in one frame.
    pub fn device(&self) -> &ProtocolObject<dyn MTLDevice> {
        self.device
    }
    pub fn queue(&self) -> &ProtocolObject<dyn MTLCommandQueue> {
        self.queue
    }
    pub fn texture(&self) -> Retained<ProtocolObject<dyn MTLTexture>> {
        self.drawable.texture()
    }
    /// Clear the frame through Skia using unpremultiplied RGBA components in 0..=1.
    pub fn clear(&self, rgba: [f64; 4]) {
        self.canvas.clear(skia_safe::Color4f::new(
            rgba[0] as f32,
            rgba[1] as f32,
            rgba[2] as f32,
            rgba[3] as f32,
        ));
    }
}

fn ns(s: &str) -> Retained<NSString> {
    NSString::from_str(s)
}
fn error(s: &str) -> Error {
    Error(s.into())
}
fn ns_size(s: Size) -> NSSize {
    NSSize::new(s.width, s.height)
}
fn size(s: NSSize) -> Size {
    Size::new(s.width, s.height)
}
fn make_titlebar_button(
    s: &SurfaceState,
    action: objc2::runtime::Sel,
    label: &str,
    identifier: &str,
    toggle: bool,
) -> TitlebarButton {
    // SAFETY: Main-thread AppKit controls. SurfaceState retains the target for
    // the accessory's lifetime, and callers supply matching action selectors.
    unsafe {
        let mtm = s.view.mtm();
        let accessory = NSTitlebarAccessoryViewController::new(mtm);
        accessory.setLayoutAttribute(NSLayoutAttribute::Right);
        let holder = NSView::initWithFrame(
            NSView::alloc(mtm),
            NSRect::new(NSPoint::new(0., 0.), NSSize::new(42., 32.)),
        );
        let button = NSButton::initWithFrame(
            NSButton::alloc(mtm),
            NSRect::new(NSPoint::new(6., 5.), NSSize::new(30., 22.)),
        );
        button.setButtonType(if toggle {
            NSButtonType::PushOnPushOff
        } else {
            NSButtonType::MomentaryPushIn
        });
        button.setBezelStyle(NSBezelStyle::AccessoryBarAction);
        button.setImagePosition(NSCellImagePosition::ImageOnly);
        button.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewMinXMargin
                | NSAutoresizingMaskOptions::ViewMaxXMargin
                | NSAutoresizingMaskOptions::ViewMinYMargin
                | NSAutoresizingMaskOptions::ViewMaxYMargin,
        );
        button.setTarget(Some(&s.delegate));
        button.setAction(Some(action));
        button.setAccessibilityLabel(Some(&ns(label)));
        button.setAccessibilityIdentifier(Some(&ns(identifier)));
        holder.addSubview(&button);
        accessory.setView(&holder);
        s.native.addTitlebarAccessoryViewController(&accessory);
        TitlebarButton { accessory, button }
    }
}
fn report(app: &AppState, e: Error) {
    if !app.alive.get() {
        return;
    }
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if let Some(f) = &app.options.on_error {
            f(&e);
        } else {
            eprintln!("scars-window: {e}");
        }
    }));
}
fn guarded<T>(app: &AppState, f: impl FnOnce() -> T) -> Option<T> {
    if !app.alive.get() {
        return None;
    }
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(v) => Some(v),
        Err(_) => {
            report(
                app,
                error("Panic in window callback (contained at the AppKit boundary)"),
            );
            None
        }
    }
}
fn with_surface<T>(s: &Weak<SurfaceState>, f: impl FnOnce(&Rc<SurfaceState>) -> T) -> Option<T> {
    let s = s.upgrade()?;
    let app = s.app.upgrade()?;
    guarded(&app, || f(&s))
}
fn schedule(
    mtm: MainThreadMarker,
    delay: Duration,
    f: impl FnOnce() + 'static,
) -> Retained<NSTimer> {
    let pending = RefCell::new(Some(f));
    let block = RcBlock::new(move |_: NonNull<NSTimer>| {
        let callback = pending.borrow_mut().take();
        if let Some(callback) = callback {
            callback();
        }
    });
    // SAFETY: Registered only on the main runloop; closure is never sent between threads.
    unsafe {
        let timer = NSTimer::timerWithTimeInterval_repeats_block(
            delay.as_secs_f64().max(0.000_001),
            false,
            &block,
        );
        NSRunLoop::mainRunLoop().addTimer_forMode(&timer, NSRunLoopCommonModes);
        let _ = mtm;
        timer
    }
}

impl Application {
    pub fn new(mut options: ApplicationOptions) -> Result<Self> {
        let mtm = MainThreadMarker::new()
            .ok_or_else(|| error("Application must be created on the main thread"))?;
        autoreleasepool(|_| {
            let native = NSApplication::sharedApplication(mtm);
            if native.delegate().is_some() {
                return Err(error("An application delegate is already installed"));
            }
            // Match the deployment contract of the current C++ backend.
            if NSProcessInfo::processInfo()
                .operatingSystemVersion()
                .majorVersion
                < 26
            {
                return Err(error("macOS 26 or later is required"));
            }
            if options.name.is_empty() {
                options.name = "SCARS Window".into();
            }
            let device = MTLCreateSystemDefaultDevice()
                .ok_or_else(|| error("No Metal device is available"))?;
            let queue = device
                .newCommandQueue()
                .ok_or_else(|| error("Cannot create Metal command queue"))?;
            let renderer = rendering::Renderer::new(&device, &queue)?;
            let quit_on_last_window_close = options.quit_on_last_window_close;
            let state = Rc::new(AppState {
                native,
                delegate: OnceCell::new(),
                default_menu: OnceCell::new(),
                motion_observer: OnceCell::new(),
                options,
                surfaces: RefCell::new(vec![]),
                managed_windows: RefCell::new(vec![]),
                window_factory: RefCell::new(None),
                renderer,
                device,
                queue,
                epoch: Instant::now(),
                alive: Cell::new(true),
                running: Cell::new(false),
                quitting: Cell::new(false),
                quit_on_last_window_close: Cell::new(quit_on_last_window_close),
                last_window_quit_pending: Cell::new(false),
                mtm,
            });
            let delegate = AppDelegate::new(mtm, Rc::downgrade(&state));
            state
                .native
                .setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
            state
                .delegate
                .set(delegate)
                .unwrap_or_else(|_| unreachable!());
            state
                .native
                .setActivationPolicy(NSApplicationActivationPolicy::Regular);
            let menu = base_menu(&state);
            state.native.setMainMenu(Some(&menu));
            state
                .default_menu
                .set(menu)
                .unwrap_or_else(|_| unreachable!());
            let bound = dispatch2::MainThreadBound::new(Rc::downgrade(&state), mtm);
            let block = RcBlock::new(move |_: NonNull<NSNotification>| {
                if let Some(mtm) = MainThreadMarker::new()
                    && let Some(app) = bound.get(mtm).upgrade()
                {
                    guarded(&app, || {
                        let surfaces = app.surfaces.borrow().clone();
                        for s in surfaces.into_iter().filter_map(|w| w.upgrade()) {
                            view::refresh_motion(&s);
                        }
                    });
                }
            });
            // SAFETY: A main-queue observer; the captured MainThreadBound is Send + Sync.
            let observer = unsafe {
                NSWorkspace::sharedWorkspace()
                    .notificationCenter()
                    .addObserverForName_object_queue_usingBlock(
                        Some(NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification),
                        None,
                        Some(&NSOperationQueue::mainQueue()),
                        &block,
                    )
            };
            state
                .motion_observer
                .set(observer)
                .unwrap_or_else(|_| unreachable!());
            state.native.finishLaunching();
            Ok(Self { state })
        })
    }
    pub fn run(&self) -> Result<()> {
        if self.state.running.replace(true) {
            return Err(error("Application::run cannot be nested"));
        }
        self.state.native.activate();
        self.state.native.run();
        self.state.running.set(false);
        Ok(())
    }
    /// Consult every close veto before closing any window; returns false if vetoed.
    pub fn quit(&self) -> bool {
        quit(&self.state)
    }
    /// Suppress automatic exit with false. The policy is checked after window
    /// closure callbacks; changing it alone does not quit an already windowless app.
    pub fn set_quit_on_last_window_close(&self, value: bool) {
        self.state.quit_on_last_window_close.set(value);
    }
    pub fn post(&self, f: impl FnOnce() + 'static) {
        self.post_after(Duration::ZERO, f);
    }
    pub fn post_after(&self, delay: Duration, f: impl FnOnce() + 'static) {
        let weak = Rc::downgrade(&self.state);
        schedule(self.state.mtm, delay, move || {
            if let Some(app) = weak.upgrade() {
                guarded(&app, f);
            }
        });
    }
    pub fn gpu_description(&self) -> String {
        format!("Metal / {}", self.state.device.name())
    }
    pub fn device(&self) -> &ProtocolObject<dyn MTLDevice> {
        &self.state.device
    }
    pub fn queue(&self) -> &ProtocolObject<dyn MTLCommandQueue> {
        &self.state.queue
    }
    pub fn clipboard_text(&self) -> String {
        // SAFETY: Pasteboard access is on the main thread, using the string pasteboard type.
        unsafe {
            NSPasteboard::generalPasteboard()
                .stringForType(NSPasteboardTypeString)
                .map(|s| s.to_string())
                .unwrap_or_default()
        }
    }
    pub fn set_clipboard_text(&self, text: &str) -> Result<()> {
        // SAFETY: Same main-thread pasteboard contract as clipboard_text.
        unsafe {
            let pasteboard = NSPasteboard::generalPasteboard();
            pasteboard.clearContents();
            if pasteboard.setString_forType(&ns(text), NSPasteboardTypeString) {
                Ok(())
            } else {
                Err(error("Cannot write clipboard text"))
            }
        }
    }
    /// Pump native events without entering `run`, useful to embedders and desktop tests.
    /// Calling this from a callback or while `run` is active is rejected.
    pub fn poll_events(&self, duration: Duration) -> Result<()> {
        if self.state.running.replace(true) {
            return Err(error("Cannot nest event loops"));
        }
        autoreleasepool(|_| {
            self.state.native.updateWindows();
            let deadline = NSDate::dateWithTimeIntervalSinceNow(duration.as_secs_f64());
            // SAFETY: This is the main AppKit event loop, not running concurrently with run().
            unsafe {
                while let Some(event) = self
                    .state
                    .native
                    .nextEventMatchingMask_untilDate_inMode_dequeue(
                        NSEventMask::Any,
                        Some(&deadline),
                        NSDefaultRunLoopMode,
                        true,
                    )
                {
                    self.state.native.sendEvent(&event);
                    self.state.native.updateWindows();
                    if deadline.timeIntervalSinceNow() <= 0. {
                        break;
                    }
                }
            }
        });
        self.state.running.set(false);
        Ok(())
    }
}
impl Drop for Application {
    fn drop(&mut self) {
        // Prevent new surfaces from being created by close/completion callbacks.
        self.state.quitting.set(true);
        let surfaces = self.state.surfaces.borrow().clone();
        for s in surfaces.into_iter().filter_map(|w| w.upgrade()) {
            close_surface(&s, DialogResult::Cancelled);
        }
        self.state.alive.set(false);
        if let Some(observer) = self.state.motion_observer.get() {
            // SAFETY: Removing the exact observer token on the main thread.
            unsafe {
                NSWorkspace::sharedWorkspace()
                    .notificationCenter()
                    .removeObserver((**observer).as_ref());
            }
        }
        self.state.native.setDelegate(None);
        self.state.native.setMainMenu(None);
    }
}
fn quit(app: &Rc<AppState>) -> bool {
    if app.quitting.replace(true) {
        return false;
    }
    let surfaces: Vec<_> = app
        .surfaces
        .borrow()
        .iter()
        .filter_map(Weak::upgrade)
        .collect();
    for s in &surfaces {
        let f = s.callbacks.borrow().on_close_request.clone();
        if s.open.get() && f.is_some_and(|f| guarded(app, || f()) != Some(true)) {
            app.quitting.set(false);
            return false;
        }
    }
    for s in surfaces {
        close_surface(&s, DialogResult::Cancelled);
    }
    app.native.stop(None);
    // Wake a run loop waiting in nextEvent so run() can return normally.
    let wake = NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2(
        NSEventType::ApplicationDefined, NSPoint::new(0.,0.), NSEventModifierFlags::empty(), 0., 0, None, 0, 0, 0,
    );
    if let Some(wake) = wake {
        app.native.postEvent_atStart(&wake, false);
    }
    app.quitting.set(false);
    true
}

fn has_open_windows(app: &AppState) -> bool {
    app.surfaces
        .borrow()
        .iter()
        .filter_map(Weak::upgrade)
        .any(|surface| surface.open.get() && surface.parent.upgrade().is_none())
}
fn quit_after_last_window(app: &Rc<AppState>) {
    if !app.alive.get()
        || app.quitting.get()
        || !app.quit_on_last_window_close.get()
        || has_open_windows(app)
        || app.last_window_quit_pending.replace(true)
    {
        return;
    }
    let weak = Rc::downgrade(app);
    // Defer until parent/child close cascades and user completion callbacks finish.
    // A callback may open a replacement window or suppress automatic exit.
    schedule(app.mtm, Duration::ZERO, move || {
        if let Some(app) = weak.upgrade() {
            guarded(&app, || {
                app.last_window_quit_pending.set(false);
                if app.running.get()
                    && !app.quitting.get()
                    && app.quit_on_last_window_close.get()
                    && !has_open_windows(&app)
                {
                    quit(&app);
                }
            });
        }
    });
}

impl Window {
    pub fn new(app: &Application, options: WindowOptions) -> Result<Self> {
        Ok(Self {
            surface: Surface {
                state: create_surface(&app.state, options, None)?,
            },
        })
    }
    pub fn set_menu(
        &self,
        items: Vec<MenuItem>,
        on_command: impl Fn(Command) + 'static,
    ) -> Result<()> {
        self.ensure_open()?;
        validate_menu(&items)?;
        let app = self
            .state
            .app
            .upgrade()
            .ok_or_else(|| error("Application is closed"))?;
        let bar = base_menu(&app);
        let extra = build_menu(&items, &self.state.delegate, app.mtm);
        for item in extra.itemArray().to_vec() {
            extra.removeItem(&item);
            bar.addItem(&item);
        }
        *self.state.menu_items.borrow_mut() = items;
        let previous = self.state.on_command.replace(Some(Rc::new(on_command)));
        drop(previous);
        *self.state.menu.borrow_mut() = Some(bar);
        sync_fullscreen_controls(&self.state);
        if self.state.native.isKeyWindow() || self.state.native.attachedSheet().is_some() {
            activate_menu(&self.state);
        }
        Ok(())
    }
    pub fn set_command_state(&self, command: Command, enabled: bool, checked: bool) {
        if let Some(item) = command_mut(&mut self.state.menu_items.borrow_mut(), command) {
            item.enabled = enabled;
            item.checked = checked;
        }
        if let Some(menu) = self.state.menu.borrow().as_ref() {
            update_native_command(menu, command, enabled, checked);
        }
        sync_fullscreen_controls(&self.state);
    }
    pub fn invoke_command(&self, command: Command) -> bool {
        invoke(&self.state, command)
    }
}
impl Dialog {
    pub fn new(parent: &Window, options: WindowOptions) -> Result<Self> {
        parent.ensure_open()?;
        let app = parent
            .state
            .app
            .upgrade()
            .ok_or_else(|| error("Application is closed"))?;
        Ok(Self {
            surface: Surface {
                state: create_surface(&app, options, Some(&parent.state))?,
            },
        })
    }
    pub fn show_modal(&self, completion: impl Fn(DialogResult) + 'static) -> Result<()> {
        self.ensure_open()?;
        let parent = self
            .state
            .parent
            .upgrade()
            .filter(|p| p.open.get())
            .ok_or_else(|| error("Parent is closed"))?;
        if self.state.modal.get() || parent.native.attachedSheet().is_some() {
            return Err(error("A sheet is already presented for this parent"));
        }
        self.state.modal.set(true);
        *self.state.completion.borrow_mut() = Some(Rc::new(completion));
        self.state.native.orderOut(None);
        if !self.is_open() || !parent.open.get() {
            return Err(error("Dialog closed during presentation"));
        }
        parent
            .native
            .beginSheet_completionHandler(&self.state.native, None);
        if self.is_open() {
            self.state.native.makeFirstResponder(Some(&self.state.view));
            activate_menu(&self.state);
            self.request_redraw();
        }
        Ok(())
    }
    pub fn dismiss(&self, result: DialogResult) {
        close_surface(&self.state, result);
    }
}

impl Surface {
    pub fn surface_clone(&self) -> Surface {
        self.clone()
    }
    pub fn downgrade(&self) -> WeakSurface {
        WeakSurface {
            state: Rc::downgrade(&self.state),
        }
    }
    fn ensure_open(&self) -> Result<()> {
        if self.is_open() {
            Ok(())
        } else {
            Err(error("Window is closed"))
        }
    }
    pub fn is_open(&self) -> bool {
        self.state.open.get()
    }
    pub fn set_callbacks(&self, callbacks: Callbacks) {
        if !self.is_open() {
            return;
        }
        self.clear_view();
        let previous = self.state.callbacks.replace(callbacks);
        drop(previous);
        self.state.failed.set(false);
        self.request_redraw();
    }
    pub fn set_title(&self, title: &str) {
        if self.is_open() {
            self.state.native.setTitle(&ns(title));
        }
    }
    pub fn title(&self) -> String {
        self.state.native.title().to_string()
    }
    pub fn show(&self) {
        if !self.is_open() || self.state.modal.get() {
            return;
        }
        if self.is_minimized() {
            self.state.native.deminiaturize(None);
        }
        self.state.native.makeKeyAndOrderFront(None);
        if self.is_open() {
            self.state.native.makeFirstResponder(Some(&self.state.view));
            self.request_redraw();
        }
    }
    pub fn hide(&self) -> Result<()> {
        self.ensure_open()?;
        if self.state.modal.get() {
            return Err(error("Dismiss a modal dialog instead of hiding it"));
        }
        self.state.native.orderOut(None);
        deactivate_input(&self.state);
        update_pause(&self.state);
        Ok(())
    }
    pub fn minimize(&self) {
        if self.is_open() {
            self.state.native.miniaturize(None);
            deactivate_input(&self.state);
            update_pause(&self.state);
        }
    }
    pub fn is_minimized(&self) -> bool {
        self.is_open() && self.state.native.isMiniaturized()
    }
    pub fn is_visible(&self) -> bool {
        self.is_open() && self.state.native.isVisible() && !self.is_minimized()
    }
    pub fn focus(&self) {
        if self.is_open() {
            self.show();
            if self.is_open() {
                activate_menu(&self.state);
            }
        }
    }
    pub fn close(&self) {
        close_surface(&self.state, DialogResult::Cancelled);
    }
    pub fn content_size(&self) -> Size {
        size(self.state.view.bounds().size)
    }
    pub fn set_content_size(&self, value: Size) -> Result<()> {
        self.ensure_open()?;
        if !valid_size(value, false) {
            return Err(error("Content size must be finite and positive"));
        }
        let minimum = self.state.native.contentMinSize();
        if value.width < minimum.width || value.height < minimum.height {
            return Err(error("Content size is smaller than minimum size"));
        }
        self.state.native.setContentSize(ns_size(value));
        Ok(())
    }
    /// Frame origin in AppKit screen coordinates (bottom-left origin, logical points).
    pub fn position(&self) -> Point {
        let p = self.state.native.frame().origin;
        Point::new(p.x, p.y)
    }
    pub fn set_position(&self, p: Point) -> Result<()> {
        self.ensure_open()?;
        if !p.x.is_finite() || !p.y.is_finite() {
            return Err(error("Position must be finite"));
        }
        self.state.native.setFrameOrigin(NSPoint::new(p.x, p.y));
        Ok(())
    }
    pub fn center(&self) {
        if self.is_open() {
            self.state.native.center();
        }
    }
    pub fn set_resizable(&self, value: bool) {
        if !self.is_open() {
            return;
        }
        let mut style = self.state.native.styleMask();
        style.set(NSWindowStyleMask::Resizable, value);
        self.state.native.setStyleMask(style);
        if value && self.state.fullscreen_capable {
            self.state.native.setCollectionBehavior(
                self.state.native.collectionBehavior()
                    | NSWindowCollectionBehavior::FullScreenPrimary,
            );
        }
        sync_fullscreen_controls(&self.state);
    }
    pub fn is_fullscreen(&self) -> bool {
        self.is_open()
            && self
                .state
                .native
                .styleMask()
                .contains(NSWindowStyleMask::FullScreen)
    }
    pub fn supports_fullscreen(&self) -> bool {
        self.is_open()
            && self.state.fullscreen_capable
            && self
                .state
                .native
                .styleMask()
                .contains(NSWindowStyleMask::Resizable)
    }
    /// Request an explicit state. Native transitions finish asynchronously and
    /// emit FullscreenChanged. Repeating a completed state is harmless; a request
    /// during an active transition returns an error.
    pub fn set_fullscreen(&self, value: bool) -> Result<()> {
        self.ensure_open()?;
        if self.state.fullscreen_transition.get() {
            return Err(error("A full-screen transition is in progress"));
        }
        if self.is_fullscreen() == value {
            return Ok(());
        }
        if self.state.native.attachedSheet().is_some() || (value && !self.supports_fullscreen()) {
            return Err(error("This window cannot change full-screen mode"));
        }
        self.state.fullscreen_transition.set(true);
        self.state.native.toggleFullScreen(None);
        sync_fullscreen_controls(&self.state);
        Ok(())
    }
    pub fn toggle_fullscreen(&self) {
        let _ = self.set_fullscreen(!self.is_fullscreen());
    }
    pub fn set_cursor(&self, cursor: Cursor) {
        self.state.cursor.set(cursor);
        if self.is_open() {
            self.state
                .native
                .invalidateCursorRectsForView(&self.state.view);
        }
    }
    pub fn set_appearance(&self, appearance: Appearance) {
        if !self.is_open() {
            return;
        }
        self.state.appearance.set(appearance);
        // SAFETY: These exported AppKit appearance constants are immutable.
        let native = unsafe {
            match appearance {
                Appearance::System => None,
                Appearance::Light => NSAppearance::appearanceNamed(NSAppearanceNameAqua),
                Appearance::Dark => NSAppearance::appearanceNamed(NSAppearanceNameDarkAqua),
            }
        };
        self.state.native.setAppearance(native.as_deref());
        self.request_redraw();
    }
    pub fn appearance(&self) -> Appearance {
        self.state.appearance.get()
    }
    pub fn effective_appearance(&self) -> Appearance {
        effective_appearance(&self.state)
    }
    /// Show/hide an explicit full-screen titlebar action. It uses the same
    /// transition/state handling as set_fullscreen and adds no menu or shortcut.
    pub fn set_titlebar_fullscreen_button(&self, visible: bool) {
        if !self.is_open() {
            return;
        }
        if visible {
            if self.state.fullscreen_button.borrow().is_none() {
                let button = make_titlebar_button(
                    &self.state,
                    sel!(fullScreenButton:),
                    "Enter Full Screen",
                    "scars-titlebar-fullscreen-button",
                    false,
                );
                *self.state.fullscreen_button.borrow_mut() = Some(button);
            }
        } else {
            let previous = self.state.fullscreen_button.borrow_mut().take();
            if let Some(previous) = previous {
                previous.button.setEnabled(false);
                // SAFETY: Disconnect removed controls so retained native references are inert.
                unsafe {
                    previous.button.setTarget(None);
                    previous.button.setAction(None);
                }
                let index = self
                    .state
                    .native
                    .titlebarAccessoryViewControllers()
                    .iter()
                    .position(|accessory| std::ptr::eq(&*accessory, &*previous.accessory));
                if let Some(index) = index {
                    self.state
                        .native
                        .removeTitlebarAccessoryViewControllerAtIndex(index as isize);
                }
            }
        }
        sync_fullscreen_controls(&self.state);
    }
    /// Adds/replaces the native sun/moon button. It updates native appearance,
    /// then calls `changed(light)` so the renderer can update its own palette.
    pub fn set_titlebar_theme_toggle(&self, light: bool, changed: impl Fn(bool) + 'static) {
        if !self.is_open() {
            return;
        }
        let previous = self.state.theme_changed.replace(Some(Rc::new(changed)));
        drop(previous);
        if self.state.theme.borrow().is_none() {
            let button = make_titlebar_button(
                &self.state,
                sel!(themeToggle:),
                "Light mode",
                "scars-titlebar-theme-toggle",
                true,
            );
            *self.state.theme.borrow_mut() = Some(button);
        }
        self.set_titlebar_theme_toggle_state(light);
    }
    /// Synchronizes the accessory and native appearance without firing `changed`.
    pub fn set_titlebar_theme_toggle_state(&self, light: bool) {
        if !self.is_open() {
            return;
        }
        let button = self.state.theme.borrow().as_ref().map(|t| t.button.clone());
        if let Some(button) = button {
            button.setState(if light { 1 } else { 0 });
            button.setImage(
                NSImage::imageWithSystemSymbolName_accessibilityDescription(
                    &ns(if light { "sun.max.fill" } else { "moon.fill" }),
                    None,
                )
                .as_deref(),
            );
            button.setToolTip(Some(&ns(if light {
                "Switch to dark mode"
            } else {
                "Switch to light mode"
            })));
        }
        self.set_appearance(if light {
            Appearance::Light
        } else {
            Appearance::Dark
        });
    }
    pub fn request_redraw(&self) {
        if self.is_open() {
            self.state.dirty.set(true);
            update_pause(&self.state);
        }
    }
    pub fn set_animating(&self, value: bool) {
        self.state.animating.set(value && self.is_open());
        self.state.previous.set(None);
        self.request_redraw();
    }
    pub fn is_animating(&self) -> bool {
        self.is_open() && self.state.animating.get()
    }
    pub fn set_redraw_delay(&self, delay: Option<Duration>) -> Result<()> {
        if delay.is_some_and(|d| d > Duration::from_secs(86400)) {
            return Err(error("Redraw delay must be at most one day"));
        }
        if let Some(timer) = self.state.redraw_timer.borrow_mut().take() {
            timer.invalidate();
        }
        if !self.is_open() {
            return Ok(());
        }
        if let Some(delay) = delay {
            let weak = Rc::downgrade(&self.state);
            let timer = schedule(self.state.view.mtm(), delay, move || {
                with_surface(&weak, |s| {
                    if s.open.get() {
                        s.dirty.set(true);
                        update_pause(s);
                    }
                });
            });
            *self.state.redraw_timer.borrow_mut() = Some(timer);
        }
        Ok(())
    }
    pub fn set_text_input_client(&self, client: Option<Rc<dyn TextInputClient>>) {
        if !self.is_open() {
            return;
        }
        let previous = self.state.text_input.borrow_mut().take();
        if let Some(previous) = previous
            && let Some(app) = self.state.app.upgrade()
        {
            guarded(&app, || {
                previous.unmark(true);
                previous.deactivate();
            });
        }
        if !self.is_open() {
            return;
        }
        *self.state.text_input.borrow_mut() = client;
        sync_secure_input(&self.state, false);
        // SAFETY: NSTextInputContext must be used on its view's main thread.
        if let Some(context) = self.state.view.inputContext() {
            context.invalidateCharacterCoordinates();
        }
    }
    /// Refresh secure-input status and IME candidate geometry after editor state changes.
    pub fn invalidate_text_input(&self) {
        sync_secure_input(&self.state, false);
        // SAFETY: Main-thread text input context.
        if let Some(context) = self.state.view.inputContext() {
            context.invalidateCharacterCoordinates();
        }
    }
    /// Native escape hatch for custom renderers and AppKit extensions.
    /// Changing this window's delegate/contentView breaks the manager's contracts.
    pub fn native_window(&self) -> &NSWindow {
        &self.state.native
    }
    pub fn native_view(&self) -> &NSView {
        &self.state.view
    }
    pub fn metal_layer(&self) -> &CAMetalLayer {
        &self.state.layer
    }
}

impl HasWindowHandle for Surface {
    fn window_handle(&self) -> std::result::Result<WindowHandle<'_>, HandleError> {
        if !self.is_open() {
            return Err(HandleError::Unavailable);
        }
        let ptr = NonNull::from(self.native_view()).cast();
        // SAFETY: Surface retains the NSView for the entire handle borrow. Main-thread only.
        Ok(unsafe {
            WindowHandle::borrow_raw(RawWindowHandle::AppKit(AppKitWindowHandle::new(ptr)))
        })
    }
}
impl HasDisplayHandle for Surface {
    fn display_handle(&self) -> std::result::Result<DisplayHandle<'_>, HandleError> {
        Ok(DisplayHandle::appkit())
    }
}
impl HasWindowHandle for Window {
    fn window_handle(&self) -> std::result::Result<WindowHandle<'_>, HandleError> {
        self.surface.window_handle()
    }
}
impl HasDisplayHandle for Window {
    fn display_handle(&self) -> std::result::Result<DisplayHandle<'_>, HandleError> {
        self.surface.display_handle()
    }
}
impl HasWindowHandle for Dialog {
    fn window_handle(&self) -> std::result::Result<WindowHandle<'_>, HandleError> {
        self.surface.window_handle()
    }
}
impl HasDisplayHandle for Dialog {
    fn display_handle(&self) -> std::result::Result<DisplayHandle<'_>, HandleError> {
        self.surface.display_handle()
    }
}

fn create_surface(
    app: &Rc<AppState>,
    options: WindowOptions,
    parent: Option<&Rc<SurfaceState>>,
) -> Result<Rc<SurfaceState>> {
    options.validate()?;
    if !app.alive.get() || app.quitting.get() {
        return Err(error("Application is closing"));
    }
    let mtm = app.mtm;
    // SAFETY: All AppKit allocations/configuration occur on the main thread.
    // releasedWhenClosed is disabled because Rust owns the retain.
    unsafe {
        let mut style = NSWindowStyleMask::Titled
            | NSWindowStyleMask::Closable
            | NSWindowStyleMask::Miniaturizable;
        if options.resizable {
            style |= NSWindowStyleMask::Resizable;
        }
        let rect = NSRect::new(NSPoint::new(0., 0.), ns_size(options.size));
        let native: Retained<NSWindow> = if options.floating {
            let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
                NSPanel::alloc(mtm),
                rect,
                style,
                NSBackingStoreType::Buffered,
                false,
            );
            panel.setFloatingPanel(true);
            panel.setHidesOnDeactivate(true);
            Retained::into_super(panel)
        } else {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                rect,
                style,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        native.setReleasedWhenClosed(false);
        native.setTabbingMode(NSWindowTabbingMode::Disallowed);
        native.setTitle(&ns(&options.title));
        native.setContentMinSize(ns_size(options.minimum_size));
        native.center();
        native.setAcceptsMouseMovedEvents(true);
        let fullscreen_capable = parent.is_none() && !options.floating;
        if fullscreen_capable && options.resizable {
            native.setCollectionBehavior(
                native.collectionBehavior() | NSWindowCollectionBehavior::FullScreenPrimary,
            );
        }
        let delegate = SurfaceDelegate::new(mtm);
        let view = ContentView::new(mtm, rect);
        let layer = CAMetalLayer::layer();
        layer.setDevice(Some(&app.device));
        layer.setPixelFormat(MTLPixelFormat::BGRA8Unorm);
        layer.setFramebufferOnly(false);
        layer.setOpaque(true);
        view.setLayer(Some(&layer));
        view.setWantsLayer(true);
        native.setContentView(Some(&view));
        let state = Rc::new(SurfaceState {
            components: RefCell::new(None),
            app: Rc::downgrade(app),
            parent: parent.map(Rc::downgrade).unwrap_or_default(),
            children: RefCell::new(vec![]),
            native,
            view,
            delegate,
            layer,
            display_link: OnceCell::new(),
            callbacks: RefCell::new(Callbacks::default()),
            menu: RefCell::new(None),
            menu_items: RefCell::new(vec![]),
            on_command: RefCell::new(None),
            theme: RefCell::new(None),
            fullscreen_button: RefCell::new(None),
            theme_changed: RefCell::new(None),
            text_input: RefCell::new(None),
            completion: RefCell::new(None),
            open: Cell::new(true),
            modal: Cell::new(false),
            dirty: Cell::new(true),
            animating: Cell::new(false),
            failed: Cell::new(false),
            rendering: Cell::new(false),
            redraw_timer: RefCell::new(None),
            synced_size: Cell::new(Size::new(-1., -1.)),
            synced_scale: Cell::new(0.),
            previous: Cell::new(None),
            frame: Cell::new(0),
            unavailable_frames: Cell::new(0),
            cursor: Cell::new(Cursor::Arrow),
            secure_input: Cell::new(false),
            appearance: Cell::new(Appearance::System),
            reduced_motion: Cell::new(
                NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion(),
            ),
            fullscreen_capable,
            fullscreen_transition: Cell::new(false),
            fullscreen_notified: Cell::new(false),
        });
        *state.delegate.ivars().state.borrow_mut() = Rc::downgrade(&state);
        state.view.connect(Rc::downgrade(&state));
        state
            .native
            .setDelegate(Some(ProtocolObject::from_ref(&*state.delegate)));
        let link = state
            .native
            .displayLinkWithTarget_selector(&state.view, sel!(tick:));
        link.setPaused(true);
        link.addToRunLoop_forMode(&NSRunLoop::mainRunLoop(), NSRunLoopCommonModes);
        state
            .display_link
            .set(link)
            .unwrap_or_else(|_| unreachable!());
        if let Some(parent) = parent {
            parent.children.borrow_mut().push(Rc::downgrade(&state));
            parent
                .native
                .addChildWindow_ordered(&state.native, NSWindowOrderingMode::Above);
        }
        app.surfaces.borrow_mut().retain(|s| s.strong_count() > 0);
        app.surfaces.borrow_mut().push(Rc::downgrade(&state));
        sync_size(&state);
        Ok(state)
    }
}
fn close_surface(s: &Rc<SurfaceState>, result: DialogResult) {
    if !s.open.replace(false) {
        return;
    }
    components::closed(s);
    sync_fullscreen_controls(s);
    s.animating.set(false);
    s.previous.set(None);
    if let Some(link) = s.display_link.get() {
        link.invalidate();
    }
    if let Some(timer) = s.redraw_timer.borrow_mut().take() {
        timer.invalidate();
    }
    let children = s.children.borrow().clone();
    for child in children.into_iter().filter_map(|w| w.upgrade()) {
        close_surface(&child, DialogResult::Cancelled);
    }
    if let Some(parent) = s.parent.upgrade() {
        if s.modal.replace(false) {
            parent.native.endSheet(&s.native);
        }
        parent.native.removeChildWindow(&s.native);
    }
    deactivate_input(s);
    let input = s.text_input.borrow_mut().take();
    if let (Some(input), Some(app)) = (input, s.app.upgrade()) {
        guarded(&app, || input.unmark(false));
    }
    s.native.setDelegate(None);
    s.native.orderOut(None);
    s.native.close();
    let completion = s.completion.borrow_mut().take();
    let callbacks = std::mem::take(&mut *s.callbacks.borrow_mut());
    let _command = s.on_command.borrow_mut().take();
    let _theme_changed = s.theme_changed.borrow_mut().take();
    if let Some(app) = s.app.upgrade() {
        if let Some(f) = completion {
            guarded(&app, || f(result));
        }
        if let Some(f) = callbacks.on_closed {
            guarded(&app, || f());
        }
        // Remove ownership only after closure callbacks; release the RefCell borrow
        // before dropping the Window, whose Drop calls this idempotent close path.
        let owner = {
            let mut windows = app.managed_windows.borrow_mut();
            windows
                .iter()
                .position(|w| Rc::ptr_eq(&w.state, s))
                .map(|index| windows.swap_remove(index))
        };
        drop(owner);
        if let Some(parent) = s.parent.upgrade().filter(|p| p.open.get()) {
            activate_menu(&parent);
        } else if app.native.keyWindow().is_none() {
            app.native.setMainMenu(app.default_menu.get().map(|m| &**m));
        }
        quit_after_last_window(&app);
    }
}
fn emit(s: &Rc<SurfaceState>, event: Event) {
    if !s.open.get() {
        return;
    }
    let callback = s.callbacks.borrow().on_event.clone();
    if let Some(f) = callback
        && let Some(app) = s.app.upgrade()
    {
        guarded(&app, || f(&event));
    }
    sync_secure_input(s, false);
}
fn effective_appearance(s: &SurfaceState) -> Appearance {
    // SAFETY: AppKit's exported immutable appearance-name constants.
    unsafe {
        let candidates = NSArray::from_slice(&[NSAppearanceNameAqua, NSAppearanceNameDarkAqua]);
        let matched = s
            .view
            .effectiveAppearance()
            .bestMatchFromAppearancesWithNames(&candidates);
        if matched.as_deref() == Some(NSAppearanceNameDarkAqua) {
            Appearance::Dark
        } else {
            Appearance::Light
        }
    }
}
fn sync_size(s: &Rc<SurfaceState>) -> bool {
    if !s.open.get() {
        return false;
    }
    let logical = size(s.view.bounds().size);
    let scale = s.native.backingScaleFactor();
    if s.synced_size.get() == logical && s.synced_scale.get() == scale {
        return false;
    }
    s.synced_size.set(logical);
    s.synced_scale.set(scale);
    let pixels = Size::new(
        (logical.width * scale).round(),
        (logical.height * scale).round(),
    );
    CATransaction::begin();
    CATransaction::setDisableActions(true);
    s.layer.setContentsScale(scale);
    s.layer.setDrawableSize(ns_size(pixels));
    CATransaction::commit();
    emit(
        s,
        Event::Resize {
            logical_size: logical,
            pixel_size: pixels,
            scale,
        },
    );
    s.dirty.set(true);
    true
}
fn update_pause(s: &SurfaceState) {
    let drawable = s.open.get()
        && s.native.isVisible()
        && !s.native.isMiniaturized()
        && s.native
            .occlusionState()
            .contains(NSWindowOcclusionState::Visible);
    let active = drawable && !s.failed.get() && (s.dirty.get() || s.animating.get());
    if !drawable {
        s.previous.set(None);
    }
    if let Some(link) = s.display_link.get() {
        link.setPaused(!active);
    }
}
fn render(s: &Rc<SurfaceState>, sizing: bool) {
    if !s.open.get() || s.failed.get() || s.rendering.get() {
        return;
    }
    if !s.native.isVisible() || s.native.isMiniaturized() {
        update_pause(s);
        return;
    }
    sync_size(s);
    if !s.open.get() || (!s.dirty.get() && !s.animating.get()) {
        update_pause(s);
        return;
    }
    let Some(app) = s.app.upgrade() else {
        return;
    };
    let Some(draw) = s.callbacks.borrow().on_draw.clone() else {
        s.dirty.set(false);
        update_pause(s);
        return;
    };
    autoreleasepool(|_| {
        let previous_presentation = s.layer.presentsWithTransaction();
        s.layer.setPresentsWithTransaction(sizing);
        let Some(drawable) = s.layer.nextDrawable() else {
            s.layer.setPresentsWithTransaction(previous_presentation);
            let count = s.unavailable_frames.get() + 1;
            s.unavailable_frames.set(count);
            if count >= 120 {
                s.failed.set(true);
                report(
                    &app,
                    error("Metal drawable unavailable for 120 consecutive frames"),
                );
                update_pause(s);
            }
            return;
        };
        s.unavailable_frames.set(0);
        let mut target = match app.renderer.target(&drawable.texture()) {
            Ok(target) => target,
            Err(err) => {
                s.layer.setPresentsWithTransaction(previous_presentation);
                s.failed.set(true);
                report(&app, err);
                update_pause(s);
                return;
            }
        };
        s.rendering.set(true);
        s.dirty.set(false);
        let now = Instant::now();
        let previous = s.previous.replace(Some(now));
        let info = FrameInfo {
            logical_size: s.synced_size.get(),
            pixel_size: size(s.layer.drawableSize()),
            scale: s.synced_scale.get(),
            elapsed: now - app.epoch,
            delta: previous.map(|p| now - p).unwrap_or_default(),
            frame_number: s.frame.get(),
            sizing_transaction: sizing,
        };
        s.frame.set(s.frame.get() + 1);
        target
            .canvas()
            .scale((info.scale as f32, info.scale as f32));
        if guarded(&app, || {
            draw(&Frame {
                info,
                canvas: target.canvas(),
                device: &app.device,
                queue: &app.queue,
                drawable: &drawable,
            })
        })
        .is_none()
        {
            s.failed.set(true);
        }
        if let Err(err) = app.renderer.submit(&mut target) {
            s.failed.set(true);
            report(&app, err);
        }
        // Release the frame's Skia/texture wrapper before native presentation.
        drop(target);
        if s.open.get() && !s.failed.get() {
            if let Some(buffer) = app.queue.commandBuffer() {
                if sizing {
                    buffer.commit();
                    buffer.waitUntilScheduled();
                    drawable.present();
                } else {
                    buffer.presentDrawable(ProtocolObject::from_ref(&*drawable));
                    buffer.commit();
                }
            } else {
                s.failed.set(true);
                report(
                    &app,
                    error("Cannot allocate Metal presentation command buffer"),
                );
            }
        }
        s.layer.setPresentsWithTransaction(previous_presentation);
        s.rendering.set(false);
        sync_secure_input(s, false);
        update_pause(s);
    });
}

fn base_menu(app: &Rc<AppState>) -> Retained<NSMenu> {
    // SAFETY: Target lives for app lifetime; selectors match the AppDelegate methods.
    unsafe {
        let mtm = app.mtm;
        let bar = NSMenu::new(mtm);
        let root = NSMenuItem::new(mtm);
        let application = NSMenu::new(mtm);
        root.setTitle(&ns(&app.options.name));
        application.setTitle(&ns(&app.options.name));
        let quit = NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            &ns(&format!("Quit {}", app.options.name)),
            Some(sel!(quit:)),
            &ns("q"),
        );
        quit.setTarget(app.delegate.get().map(|d| &***d as &AnyObject));
        application.addItem(&quit);
        root.setSubmenu(Some(&application));
        bar.addItem(&root);

        bar
    }
}
fn build_menu(
    items: &[MenuItem],
    target: &SurfaceDelegate,
    mtm: MainThreadMarker,
) -> Retained<NSMenu> {
    let menu = NSMenu::new(mtm);
    menu.setAutoenablesItems(false);
    for item in items {
        // SAFETY: Action target is retained by the window; command selector accepts NSMenuItem.
        unsafe {
            let native = if item.separator {
                NSMenuItem::separatorItem(mtm)
            } else {
                NSMenuItem::initWithTitle_action_keyEquivalent(
                    NSMenuItem::alloc(mtm),
                    &ns(&item.label),
                    if item.command != 0 {
                        Some(sel!(command:))
                    } else {
                        None
                    },
                    &ns(&item.shortcut.key.equivalent()),
                )
            };
            native.setTag(item.command as isize);
            native.setEnabled(item.enabled);
            native.setState(if item.checked { 1 } else { 0 });
            native.setKeyEquivalentModifierMask(view::flags(item.shortcut.modifiers));
            native.setTarget(Some(target));
            if !item.children.is_empty() {
                let submenu = build_menu(&item.children, target, mtm);
                submenu.setTitle(&ns(&item.label));
                native.setSubmenu(Some(&submenu));
            }
            menu.addItem(&native);
        }
    }
    menu
}
fn update_native_command(menu: &NSMenu, command: Command, enabled: bool, checked: bool) {
    for item in menu.itemArray() {
        if item.tag() == command as isize {
            item.setEnabled(enabled);
            item.setState(if checked { 1 } else { 0 });
        }
        if let Some(submenu) = item.submenu() {
            update_native_command(&submenu, command, enabled, checked);
        }
    }
}
fn activate_menu(s: &Rc<SurfaceState>) {
    let owner = s.parent.upgrade().unwrap_or_else(|| s.clone());
    sync_fullscreen_controls(&owner);
    let menu = owner.menu.borrow().clone();
    if let Some(app) = s.app.upgrade() {
        app.native.setMainMenu(
            menu.as_deref()
                .or_else(|| app.default_menu.get().map(|m| &**m)),
        );
    }
}
fn invoke(s: &Rc<SurfaceState>, command: Command) -> bool {
    if !s.open.get()
        || s.native.attachedSheet().is_some()
        || !command_enabled(&s.menu_items.borrow(), command)
    {
        return false;
    }
    let f = s.on_command.borrow().clone();
    if let (Some(app), Some(f)) = (s.app.upgrade(), f) {
        guarded(&app, || f(command)).is_some()
    } else {
        false
    }
}

fn fullscreen_control_enabled(s: &Rc<SurfaceState>) -> bool {
    let surface = Surface { state: s.clone() };
    s.open.get()
        && !s.fullscreen_transition.get()
        && s.native.attachedSheet().is_none()
        && (surface.is_fullscreen() || surface.supports_fullscreen())
}
fn sync_fullscreen_controls(s: &Rc<SurfaceState>) {
    let button = s
        .fullscreen_button
        .borrow()
        .as_ref()
        .map(|control| control.button.clone());
    if let Some(button) = button {
        let fullscreen = s.native.styleMask().contains(NSWindowStyleMask::FullScreen);
        let label = if fullscreen {
            "Exit Full Screen"
        } else {
            "Enter Full Screen"
        };
        let symbol = if fullscreen {
            "arrow.down.right.and.arrow.up.left"
        } else {
            "arrow.up.left.and.arrow.down.right"
        };
        button.setImage(
            NSImage::imageWithSystemSymbolName_accessibilityDescription(&ns(symbol), None)
                .as_deref(),
        );
        button.setToolTip(Some(&ns(label)));
        button.setAccessibilityLabel(Some(&ns(label)));
        button.setEnabled(fullscreen_control_enabled(s));
    }
}
fn fullscreen_finished(s: &Rc<SurfaceState>) {
    s.fullscreen_transition.set(false);
    sync_fullscreen_controls(s);
    let fullscreen = s.native.styleMask().contains(NSWindowStyleMask::FullScreen);
    if s.fullscreen_notified.replace(fullscreen) != fullscreen {
        emit(s, Event::FullscreenChanged(fullscreen));
    }
    if s.open.get() {
        s.dirty.set(true);
        update_pause(s);
    }
}

fn new_window(app: &Rc<AppState>) {
    let factory = app.window_factory.borrow().clone();
    if let Some(factory) = factory {
        if let Err(e) = builder::open_window(app, factory()) {
            report(app, e);
        }
    } else if let Some(f) = &app.options.on_new_window {
        f();
    }
}

struct DelegateIvars {
    state: RefCell<Weak<SurfaceState>>,
}
define_class!(
    // SAFETY: NSObject has no subclass invariants; all methods match AppKit signatures.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = DelegateIvars]
    struct SurfaceDelegate;
    unsafe impl NSObjectProtocol for SurfaceDelegate {}
    unsafe impl NSWindowDelegate for SurfaceDelegate {
        #[unsafe(method(windowShouldClose:))]
        fn should_close(&self, _: &NSWindow) -> bool {
            with_surface(&self.ivars().state.borrow().clone(),|s| {
                if !s.open.get() || s.native.attachedSheet().is_some() { return; }
                let request = s.callbacks.borrow().on_close_request.clone();
                if request.is_none_or(|f| f()) { close_surface(s,DialogResult::Cancelled); }
            }); false // close_surface owns native closure and exactly-once callbacks.
        }
        #[unsafe(method(windowDidBecomeKey:))]
        fn became_key(&self, _: &NSNotification) { with_surface(&self.ivars().state.borrow().clone(),|s| { activate_menu(s); emit(s,Event::Focus(true)); update_pause(s); }); }
        #[unsafe(method(windowDidResignKey:))]
        fn resigned_key(&self, _: &NSNotification) { with_surface(&self.ivars().state.borrow().clone(),|s| { emit(s,Event::Focus(false)); deactivate_input(s); }); }
        #[unsafe(method(windowDidResize:))]
        fn resized(&self, _: &NSNotification) { with_surface(&self.ivars().state.borrow().clone(),|s| { if sync_size(s) { render(s,true); } update_pause(s); }); }
        #[unsafe(method(windowDidChangeBackingProperties:))]
        fn backing_changed(&self, _: &NSNotification) { with_surface(&self.ivars().state.borrow().clone(),|s| { if sync_size(s) { render(s,true); } update_pause(s); }); }
        #[unsafe(method(windowDidChangeScreen:))]
        fn screen_changed(&self, _: &NSNotification) { with_surface(&self.ivars().state.borrow().clone(),|s| { sync_size(s); s.previous.set(None); update_pause(s); }); }
        #[unsafe(method(windowDidChangeOcclusionState:))]
        fn occlusion_changed(&self, _: &NSNotification) { with_surface(&self.ivars().state.borrow().clone(),|s| { s.dirty.set(true); update_pause(s); }); }
        #[unsafe(method(windowDidMiniaturize:))]
        fn minimized(&self, _: &NSNotification) { with_surface(&self.ivars().state.borrow().clone(),|s| { deactivate_input(s); update_pause(s); }); }
        #[unsafe(method(windowDidDeminiaturize:))]
        fn restored(&self, _: &NSNotification) { with_surface(&self.ivars().state.borrow().clone(),|s| { s.dirty.set(true); update_pause(s); }); }
        #[unsafe(method(windowWillEnterFullScreen:))]
        fn will_enter_fullscreen(&self, _: &NSNotification) { with_surface(&self.ivars().state.borrow().clone(),|s| { s.fullscreen_transition.set(true); sync_fullscreen_controls(s); }); }
        #[unsafe(method(windowWillExitFullScreen:))]
        fn will_exit_fullscreen(&self, _: &NSNotification) { with_surface(&self.ivars().state.borrow().clone(),|s| { s.fullscreen_transition.set(true); sync_fullscreen_controls(s); }); }
        #[unsafe(method(windowDidEnterFullScreen:))]
        fn entered_fullscreen(&self, _: &NSNotification) { with_surface(&self.ivars().state.borrow().clone(),fullscreen_finished); }
        #[unsafe(method(windowDidExitFullScreen:))]
        fn exited_fullscreen(&self, _: &NSNotification) { with_surface(&self.ivars().state.borrow().clone(),fullscreen_finished); }
        #[unsafe(method(windowDidFailToEnterFullScreen:))]
        fn failed_enter_fullscreen(&self, _: &NSWindow) { with_surface(&self.ivars().state.borrow().clone(),|s| {
            fullscreen_finished(s); if let Some(app)=s.app.upgrade() { report(&app,error("Could not enter full-screen mode")); }
        }); }
        #[unsafe(method(windowDidFailToExitFullScreen:))]
        fn failed_exit_fullscreen(&self, _: &NSWindow) { with_surface(&self.ivars().state.borrow().clone(),|s| {
            fullscreen_finished(s); if let Some(app)=s.app.upgrade() { report(&app,error("Could not exit full-screen mode")); }
        }); }
    }
    impl SurfaceDelegate {
        #[unsafe(method(fullScreenButton:))]
        fn fullscreen_button(&self, _: &NSButton) { with_surface(&self.ivars().state.borrow().clone(),|s| {
            if fullscreen_control_enabled(s) { Surface { state: s.clone() }.toggle_fullscreen(); }
        }); }
        #[unsafe(method(command:))]
        fn command(&self, sender: &NSMenuItem) { with_surface(&self.ivars().state.borrow().clone(),|s| { invoke(s,sender.tag() as Command); }); }
        #[unsafe(method(themeToggle:))]
        fn theme_toggle(&self, sender: &NSButton) { with_surface(&self.ivars().state.borrow().clone(),|s| {
            if !s.open.get() { return; }
            let light = sender.state() != 0;
            Surface { state: s.clone() }.set_titlebar_theme_toggle_state(light);
            let changed = s.theme_changed.borrow().clone(); if let Some(f) = changed { f(light); }
        }); }
    }
);
impl SurfaceDelegate {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        // SAFETY: NSObject's designated initializer, with Rust ivars initialized first.
        unsafe {
            msg_send![
                super(Self::alloc(mtm).set_ivars(DelegateIvars {
                    state: RefCell::new(Weak::new())
                })),
                init
            ]
        }
    }
}
struct AppDelegateIvars {
    state: Weak<AppState>,
}
define_class!(
    // SAFETY: NSObject has no subclass invariants. AppKit invokes these on the main thread.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = AppDelegateIvars]
    struct AppDelegate;
    unsafe impl NSObjectProtocol for AppDelegate {}
    unsafe impl NSApplicationDelegate for AppDelegate {
        #[unsafe(method(applicationShouldTerminateAfterLastWindowClosed:))]
        // close_surface schedules orderly quit after completion callbacks. Return
        // through Application::run instead of letting AppKit terminate the process.
        fn last_window(&self, _: &NSApplication) -> bool { false }
        #[unsafe(method(applicationShouldTerminate:))]
        fn should_terminate(&self, _: &NSApplication) -> NSApplicationTerminateReply {
            if let Some(app) = self.ivars().state.upgrade() { guarded(&app,|| quit(&app)); }
            NSApplicationTerminateReply::TerminateCancel
        }
        #[unsafe(method(applicationShouldHandleReopen:hasVisibleWindows:))]
        fn reopen(&self, _: &NSApplication, visible: bool) -> bool {
            if !visible && let Some(app) = self.ivars().state.upgrade() { guarded(&app,|| new_window(&app)); } true
        }
        #[unsafe(method(applicationDidResignActive:))]
        fn resigned(&self, _: &NSNotification) { if let Some(app) = self.ivars().state.upgrade() { guarded(&app,|| {
            let surfaces = app.surfaces.borrow().clone();
            for s in surfaces.into_iter().filter_map(|w| w.upgrade()) { if s.open.get() { emit(&s,Event::Focus(false)); deactivate_input(&s); } }
        }); } }
        #[unsafe(method(applicationDidBecomeActive:))]
        fn activated(&self, _: &NSNotification) { if let Some(app) = self.ivars().state.upgrade() { guarded(&app,|| {
            let surfaces = app.surfaces.borrow().clone();
            for s in surfaces.into_iter().filter_map(|w| w.upgrade()) { if s.open.get() && s.native.isKeyWindow() { emit(&s,Event::Focus(true)); } }
        }); } }
    }
    impl AppDelegate {
        #[unsafe(method(newWindow:))]
        fn new_window(&self, _: Option<&AnyObject>) { if let Some(app) = self.ivars().state.upgrade() { guarded(&app,|| new_window(&app)); } }
        #[unsafe(method(quit:))]
        fn quit_action(&self, _: Option<&AnyObject>) { if let Some(app) = self.ivars().state.upgrade() { guarded(&app,|| quit(&app)); } }
    }
);
impl AppDelegate {
    fn new(mtm: MainThreadMarker, state: Weak<AppState>) -> Retained<Self> {
        // SAFETY: NSObject designated initializer after setting Rust ivars.
        unsafe {
            msg_send![
                super(Self::alloc(mtm).set_ivars(AppDelegateIvars { state })),
                init
            ]
        }
    }
}
