# SCARS for Rust

`scars-window` is a standalone macOS window-management and rendering crate,
implemented in Rust using `objc2` bindings to AppKit, QuartzCore and Metal, with
`skia-safe` for drawing. Native Objective-C view and delegate classes are defined
in Rust. Cargo builds Skia and its C++ bindings through `skia-bindings`; there is
no dependency on the parent C++ project's CMake build or Skia checkout.

Requires macOS 26+, Rust 1.88+, the Xcode command-line tools, Python 3, Ninja and Git.
The first build needs internet access to fetch Skia sources, dependencies and GN.
The native backend currently supports macOS only. Geometry, input, menu and text-input contracts
are platform independent. The local Cargo configuration sets the macOS deployment
target to 26.0. The application also checks the OS version at runtime.

Start with the [Rust quick guide](../docs/rust-guide.md) for reactive components and
`cargo run --locked --example todo`. Each stage has its own example and focused
tests; [component contracts](components/README.md) describe the current coverage.
The [full Lab](../docs/rust-lab.md) covers all control families in retained pages;
use `cargo run --example lab --locked --offline -- --page values --size 440x740`.

## Source layout

- `components/` is the portable reactive foundation; `macros/` implements its
  Rust-like `view!` syntax. See [stage 1 examples and contracts](components/README.md).
- `src/lib.rs` exposes the public interface; `src/types.rs` holds shared geometry,
  input events, menus, options and text-input contracts.
- `src/platform.rs` selects the backend with target configuration. A future backend
  gets its own folder and an entry here.
- `src/macos/` contains all AppKit/Metal implementation, native dependency exports,
  frame callbacks and native key translation. Its `mod.rs` is the backend entry point.
- `tests/macos/` contains the native integration suites and rendering helpers;
  `tests/desktop.rs` and `tests/fullscreen.rs` only select and run those suites.

Public paths such as `scars_window::Window`, `scars_window::Callbacks` and
`scars_window::app_kit` are unchanged. Targets without a backend can still use the
shared contracts and shared Skia types; native windows remain available only on macOS.

Stage 1 examples (run from `rust/`):

```sh
cargo run --locked -p scars-ui --example foundation # portable console example
cargo run --locked --example stage1                 # native diagnostic window
```

## Run

```sh
cd rust
cargo run --locked --example windows
```

The example opens a Metal-backed window with sun/moon and full-screen titlebar
buttons. The theme button updates native appearance and the content palette; the
full-screen button enters/exits full screen and updates its icon and tooltip.
The example explicitly defines its File, View and Window menus: Command-N opens
another window, Command-L switches its theme, Command-M minimizes it, Command-W
closes it, and Command-Q quits. The full-screen titlebar button is independent of
those menu declarations. Closing the last window quits the application.
`--smoke` exits after one second.

## Skia source builds

The Cargo lockfile pins `skia-safe` and `skia-bindings` to 0.157.0. The bindings
select their matching Skia source revision; this build is independent of the
C++ project's Skia version. Enabled features provide Metal/Ganesh rendering,
SkParagraph text layout, HarfBuzz shaping, embedded ICU data, SVG, and image
decoding/encoding including PNG, JPEG and WebP. Platform fonts use CoreText.

The local `.cargo/config.toml` forces `FORCE_SKIA_BUILD=1`: Skia and its bundled
native dependencies compile from source. Upstream's `binary-cache` feature remains
enabled because it also provides the archive handling used to download **sources**;
the forced environment setting prevents downloading prebuilt Skia libraries.
Skia is optimized even in development builds while application Rust code remains
debuggable. Cargo reuses locally compiled artifacts on subsequent builds.

For a completely clean rebuild of this crate and its dependencies:

```sh
cd rust
cargo clean
cargo build --locked --all-targets
```

Cargo reads configuration from the invocation directory and its ancestors, not
from path dependencies. When using this crate from a different Cargo project,
copy the `[env]` settings from this crate's `.cargo/config.toml` into that project's
configuration (including the macOS deployment target), and copy the
`[profile.dev.package.skia-bindings]` optimization override into its root manifest.
Alternatively set `FORCE_SKIA_BUILD=1` and `MACOSX_DEPLOYMENT_TARGET=26.0` in that
project's build environment. No separate Skia bootstrap command is needed.

Use it from another Cargo project:

```toml
[dependencies]
scars-window = { path = "/path/to/cpp-demos/rust" }
```

```rust,no_run
use scars_window::{Application, Appearance, Window};

fn main() -> scars_window::Result<()> {
    Application::named("My application")?.run_with_windows(|| {
        Window::builder("A Rust window")
            .theme_toggle(Appearance::Dark)
            .full_screen_button()
            .on_draw(|window, frame| {
                frame.clear(match window.effective_appearance() {
                    Appearance::Light => [0.95, 0.95, 0.97, 1.0],
                    _ => [0.04, 0.06, 0.10, 1.0],
                });
                let mut paint = scars_window::skia::Paint::default();
                paint.set_anti_alias(true).set_color(scars_window::skia::Color::BLUE);
                frame.canvas().draw_circle((60., 60.), 24., &paint);
            })
    })
}
```

`run_with_windows` creates the first window and registers your factory for explicit
`window.request_new_window()` calls and Dock reopen. The application owns these
windows until they close. Callbacks
receive the window directly; reference counting and weak captures stay inside the
crate. The theme button synchronizes native appearance and schedules drawing, so
the example reads the current appearance instead of maintaining a second theme flag.

The crate's only automatic menu is the basic application menu containing Quit (Command-Q).
Native window tabbing is disabled, matching the C++ backend's independent windows.
The application chooses all other menus, labels, shortcuts and command handlers
through `.menu(items, |window, command| { ... })`. Neither `run_with_windows` nor
`theme_toggle` or `full_screen_button` adds File, Edit, View or Window menus. The longer executable example
shows those choices explicitly; the minimal example above has only Quit.
AppKit can augment a supplied View menu with its native full-screen command.
That system behavior remains enabled independently of the explicit titlebar button.

Menu handlers can call `window.request_new_window()`, `window.request_close()`,
`window.toggle_theme()` or `window.minimize()` without capturing the application or
using native AppKit calls. `request_close` honors the close veto; `close` forces
programmatic closure. `Shortcut::command(Key::N)` constructs a Command shortcut.

Use `app.open_window(Window::builder("Inspector").floating(true))?` to open an
application-owned window separately. It returns a control handle; dropping that
handle leaves the window open. For explicit ownership, use
`Window::builder("Inspector").build(&app)?`, retain the returned `Window`, and call
`show()` when ready. The original `Window::new` and `set_callbacks` interfaces also
remain available for integrations that need direct control.

## Explicit full-screen control

Add a titlebar button beside the theme toggle:

```rust
Window::builder("My window")
    .theme_toggle(Appearance::Dark)
    .full_screen_button()
```

The backend supplies the expand/restore icon, Enter/Exit tooltip and native button
action. The button stays synchronized with full-screen changes made through the
system green button, menus or window methods. It is disabled during transitions,
while a modal sheet blocks the window, and for unsupported window types. This
option adds no menus or keyboard shortcuts. Use
`window.set_titlebar_fullscreen_button(false)` to remove it, or `true` to show it
again. The interface exposes no AppKit types, so future backends can provide the
same explicit titlebar control. Only the macOS backend is currently implemented.

Other UI controls can call `window.toggle_fullscreen()`, or use
`window.set_fullscreen(true/false)` when they need an explicit state and an error
result. Query `is_fullscreen()` and `supports_fullscreen()`. Changes finish
asynchronously; `Event::FullscreenChanged(bool)` reports completed mode changes,
including changes initiated by macOS's green titlebar button. Failed native
transitions are reported through the application error handler. Fixed-size
windows, utility panels and dialogs do not offer full-screen entry.

## Window-management coverage

| Capability | Rust interface |
| --- | --- |
| Application creation, event loop and orderly quit | `Application::named`, `run_with_windows`, `run`, `quit` |
| Application-owned windows and New Window/Dock factories | `open_window`, `run_with_windows`, `Window::builder` |
| Deferred work and delayed work on the main runloop | `post`, `post_after`, `quit_after` |
| Independent windows and app-local floating utility panels | `Window::new`, `WindowOptions::floating` |
| Show, hide, focus, minimize, restore, center, close | `Surface` methods; `show` restores a minimized window |
| Title, content size, screen position, resizability | `set_title`, `set_content_size`, `set_position`, `set_resizable` |
| Explicit full-screen titlebar button | `WindowBuilder::full_screen_button`, `set_titlebar_fullscreen_button` |
| Full-screen window state | `set_fullscreen`, `toggle_fullscreen`, `is_fullscreen`, `supports_fullscreen` |
| System, light and dark native appearance | `set_appearance`, `appearance`, `effective_appearance` |
| Native sun/moon titlebar accessory | `WindowBuilder::theme_toggle`, `Surface::toggle_theme` |
| Per-window menu trees, shortcuts, enabled/checked state and invocation | `Window::set_menu`, `set_command_state`, `invoke_command` |
| Native first-responder editing actions | `TextInputClient::command`; the application supplies any Edit menu |
| Parent-owned dialogs and transactional component sheets | `Dialog::new`, `show_modal`, `dismiss`; `Surface::show_sheet`, `ui::Alert` |
| User-close vetoes, forced programmatic close and exactly-once closed/completion callbacks | `Callbacks`, `DialogResult` |
| Pointer, keyboard, modifiers, focus, resize, effective appearance, full-screen and Reduce Motion events | `Event` |
| Cursor and text clipboard | `set_cursor`, `Application::clipboard_text`, `set_clipboard_text` |
| Native IME composition, UTF-16 ranges, candidate geometry, edit menu and secure input | `TextInputClient`, `TextInputState` |
| Display-paced Metal drawing, coalesced invalidation and one-shot redraw | `Callbacks::on_draw`, `request_redraw`, `set_animating`, `set_redraw_delay` |
| Interoperability with other renderers/native extensions | `raw-window-handle` 0.6, `native_window`, `native_view`, `metal_layer`, `Frame` |

AppKit provides accessibility for native windows, menus and the titlebar buttons.
Mounted components supply their own accessibility semantics.

The Rust component layer provides layout, interactive basic controls and text input.
See [component contracts and examples](components/README.md). GPU-capture diagnostics remain future work.

## Ownership, callbacks and modal behavior

- Create `Application` on the main thread. Applications, windows, surfaces and native
  callbacks stay on that thread; their Rust types cannot be sent to worker threads.
- `open_window` and `run_with_windows` keep windows alive inside the application
  and release them when they close. You do not need a window registry.
- Closing the last top-level window quits by default, returning from `run` or
  `run_with_windows`. Dialogs do not count as independent top-level windows;
  floating windows do. Completion and closed callbacks run before the exit check,
  so opening a replacement window in a callback keeps the application running.
- To keep a background application alive without windows, set
  `ApplicationOptions::quit_on_last_window_close` to `false`, or call
  `app.set_quit_on_last_window_close(false)` before entering the event loop.
- When using `Window::new`, `WindowBuilder::build` or `Dialog::new` for explicit
  ownership, retain the returned owner while it is needed. Dropping it closes it.
  `Surface` clones are control handles and do not extend the owner's open lifetime.
- Dropping the application forcibly closes remaining windows, making retained
  surface handles inert. Deferred application work is discarded after destruction.
- `close()` is idempotent and bypasses user-close vetoes. A native close request
  consults `on_close_request`. Quit checks every veto before closing anything.
- Closing a parent cancels and closes its dialogs. Modal completion runs once,
  before `on_closed`. Completion may drop its own dialog. A sheet blocks its parent's
  menu/input/accessibility actions while other windows remain interactive. Use
  `dismiss`, `request_close` or `close` to end a sheet; `hide` returns an error for
  modal dialogs. `Surface::show_sheet(ui::Alert, body_builder)` retains a component
  sheet automatically and releases its parent before application completion.
  `view_with_surface` supplies window context to component actions.
- Builder callbacks take ordinary closures and receive `&Surface` as their first
  argument. The lower-level `Callbacks` struct uses `Rc<dyn Fn(...)>` and can be
  replaced from inside callbacks. Capture `WeakSurface` if a lower-level callback
  needs to refer back to its window. Release application-state borrows before
  operations that may synchronously dispatch another callback.
- Callback panics are caught and reported to `ApplicationOptions::on_error` with
  Rust's normal unwinding panic strategy. A draw panic pauses that surface's
  rendering; replacing its callbacks permits retrying. `panic = "abort"` prevents
  recovery, as it does in any Rust application.

`WindowBuilder::theme_toggle` installs and wires the sun/moon button. Drawing
can read `window.effective_appearance()` with no separate theme state. Starting
with `Appearance::System` preserves system inheritance until the first explicit
choice. `window.toggle_theme()` synchronizes the button and requests redraw.

For lower-level integration, `set_titlebar_theme_toggle` accepts a change callback;
`set_titlebar_theme_toggle_state` synchronizes the native button and appearance
without calling it. `set_appearance(Appearance::System)` restores system inheritance.

## Rendering and native bridges

Frames provide logical size, pixel size, backing scale, elapsed time, delta time,
frame number, and whether they occur in a native sizing transaction. The first
frame and the first after resuming visibility/animation have zero delta.

The crate shares one Metal device/queue and Skia Ganesh context across its windows
and gives every window a `CAMetalLayer`. Each draw callback receives
`frame.canvas()`, a borrowed `skia::Canvas` wrapping the current drawable. Draw in
logical window points with a top-left origin: the backend applies Retina scale,
flushes and submits Skia work, releases the frame's surface wrapper, then presents
on that same Metal queue. Canvas state is fresh for each frame. Callers must draw
the entire frame; drawable contents from a previous frame are not preserved.

Skia types are re-exported through `scars_window::skia`, including `Paint`, `Path`,
images, `svg` and `textlayout`. Components can use those APIs directly without a
second Skia dependency or native build. Cache fonts, paragraphs and images in
application/component state; never retain a frame's canvas or drawable.

No mutable context borrow is held across application callbacks. A Skia surface or
submission failure is reported through the application error handler and pauses
that window, as does a draw panic. During resize, the backend
waits for command scheduling before presenting inside AppKit's transaction. It
does not wait for GPU completion on ordinary frames. Native operations use logical
points; pointer and renderer-provider coordinates use a top-left origin. Window
screen positions use AppKit's bottom-left screen origin.

The lower-level `Frame::device`, `queue` and `texture` bridges remain available for
direct Metal renderers. Use either Skia drawing or direct Metal encoding for a
given frame; automatic Skia submission happens after the callback. Submit direct
Metal work on `frame.queue()` before returning and leave presentation to the crate.

Rendering pauses for hidden, minimized and occluded windows. Dirty frames coalesce.
`set_animating(false)` requests one final frame; `set_redraw_delay(None)` cancels a
pending timed invalidation. There is no CPU fallback when a Metal device is absent.
Repeated drawable acquisition failure pauses rendering and reports an error.

`TextInputClient` owns editing storage and layout. All ranges at this boundary use
UTF-16 units, including selection, marked text, replacements and geometry queries.
The bridge avoids splitting surrogate pairs when returning attributed substrings,
and does not expose secure text through that IME query. It balances Carbon secure
input on focus loss, hide, minimize, client replacement and closure.

Native APIs are re-exported as `objc`, `app_kit`, `foundation`, `metal` and
`quartz_core`. Replacing the manager's content view, delegate or Metal layer is
outside its supported lifetime/rendering contract.

## Validation

```sh
cargo fmt --all -- --check
cargo clippy --workspace --locked --all-targets -- -D warnings
cargo test --workspace --locked
cargo check -p scars-ui --lib --target x86_64-unknown-linux-gnu --locked
SCARS_WINDOW_DESKTOP_TESTS=1 cargo test --locked --test desktop
SCARS_WINDOW_FULLSCREEN_TESTS=1 cargo test --locked --test fullscreen
SCARS_WINDOW_SECURE_INPUT_TESTS=1 cargo test --locked --test secure_input
cargo run --locked --example windows -- --smoke
cargo run --locked --example stage1 -- --smoke
```

The desktop suite has its own main-thread harness. Normal `cargo test` reports it
as skipped unless explicitly enabled. Run it on an awake, unlocked desktop with
Metal and WindowServer access; sandboxed runs may not expose a Metal device. The
separately enabled full-screen test enters and exits a real macOS Space, checks
clicks on the explicit titlebar button in both directions, synchronized button
labels and completed-state events, then closes
the window. It is opt-in because it temporarily changes the desktop presentation.

The suite reads real Metal drawable pixels to verify Skia colors, origin and
backing scale, SVG rendering, shaped Unicode text, and PNG/JPEG/WebP image uploads.
It also exercises native window operations, the actual titlebar button action,
menu validation/invocation, sheets and cancellation, IME protocol calls, Skia frame
delivery, animation/idling/timed invalidation, close vetoes, reentrant callbacks and
panic recovery, plus builder callback context, managed-owner release and factory
windows through an application-defined New Window menu. Protocol tests do not replace an
interactive input-method audit. Default last-window exit, suppression, and continued
execution while another window is open are also verified. Apple Silicon is the runtime
validation target;
Intel is untested.

`publish = false` is intentional: this project has not selected a distribution
license. Dependency notices and licenses remain those of their respective crates.
