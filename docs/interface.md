# Interface contracts

## Ownership and threading

Create one `Application` on the main thread and keep it alive until all window/dialog wrappers have been destroyed. All public methods, callbacks, and Skia drawing execute on that thread. Violating the thread contract raises `std::logic_error` (application creation returns an error); destructors require the main thread.

Creation returns `std::expected<std::unique_ptr<T>, Error>`. Objects are neither copyable nor movable; their owning `unique_ptr` may move. Native resources close on wrapper destruction. `close()` is idempotent and terminal; create another object to reopen content.

Windows track their child dialogs without retaining their C++ wrappers. Closing/destroying a parent closes its dialogs, cancels outstanding modal completion, and stops their display links. Child wrappers can subsequently be inspected (`is_open()` returns false) or destroyed safely. Native targets and posted work hold weak references. Closing during a callback, including destroying a dialog from its own completion, is supported.

User close requests and application Quit consult `on_close_request`; returning false vetoes the operation. Programmatic `close()` and destruction force closure. `on_closed` runs once, after the native window is closed. On application Quit, all close requests are checked before anything closes, and `Application::run()` returns normally so C++ destruction runs. With no windows open, the event loop remains running until Quit.

`Application::post()` queues work on the main event loop; it must itself be called on the main thread. Posted functions are discarded after application destruction. Capture weak references or otherwise ensure your own objects outlive any functions you post. Exceptions raised by user callbacks are caught at every native callback entry and passed to `ApplicationOptions::on_error`. Exceptions from the error handler are caught and logged. A draw exception stops that surface's rendering; replacing callbacks allows retrying.

## Rendering and input

Set callbacks before showing a surface. The draw callback receives a borrowed `SkCanvas&`; never store its address, use it from another thread, or retain it beyond the callback. Drawing code can use Skia's full public drawing interface. No GPU device, queue, drawable or presentation operations are required in application code.

`FrameInfo` supplies logical size, drawable pixel size, backing scale, monotonic elapsed time, interval since the preceding draw, and a zero-based per-surface frame number. The first frame, and the first after animation mode changes or becoming visible again, has zero delta. User drawing runs with a top-left origin and a logical-unit scale already installed. Pixel sizes are rounded to integral device pixels.

`request_redraw()` coalesces invalidations. `set_animating(true)` enables display-paced drawing; `false` schedules one final redraw and returns to invalidation-driven drawing. Hidden, minimized, and fully occluded windows suspend rendering. Resizing, restoring visibility, and backing-scale changes invalidate content. Skia uses a shared Ganesh/Metal context on the main thread, with separate drawable surfaces for every window and dialog.

Each frame wraps a `CAMetalDrawable` texture in a GPU Skia surface, invokes drawing, flushes/submits Skia's commands, and presents on the same Metal queue. There is no CPU raster buffer in the presentation path. An unavailable drawable skips that frame. Repeated acquisition failure, a lost context, or submission failure is reported and stops rendering. Creation fails if a Metal device/context cannot be created. No CPU fallback is provided.

Input is a `std::variant` of pointer, key, modifier, focus, and resize events. Pointer coordinates are in the same logical units as drawing. Button 0 is primary, 1 secondary, and 2+ auxiliary. Scroll values retain native signed deltas. Key events include a normalized key, press/release, repeat, modifiers, and an optional platform-specific native code. Modifier changes have a separate event. These raw events do not provide Unicode text composition or IME. The optional declarative UI layer builds controls and native accessibility on top of them.

## Declarative content

`Surface::set_view<T>()`, `set_view_builder()` and `set_content()` attach the optional UI runtime to either a window or dialog. Include `skia_window/ui.hpp` for these operations. `ui_host()` exposes the mounted host for theme/environment settings, semantic inspection and automation. `clear_content()` unmounts it; closing a surface also clears it.

The legacy draw callback paints underneath UI content. UI controls consume input before unhandled events reach the legacy event callback. The UI host independently requests frames for invalidations, transitions and animated canvases; `Surface::is_animating()` reports the explicit legacy flag. See [the UI guide](ui.md) for the full declarative interface and ownership contracts.

## Menus and dialogs

`Window::set_menu()` copies a tree of top-level submenus. Leaves have unique, nonzero command IDs; separators and submenu nodes have no command ID. Invalid trees return an error without replacing the existing menu. Shortcuts use the same `Key`/`Modifiers` types as input; their default modifier is Command. `set_command_state()` updates an action's enabled and checked state. Native actions and `invoke_command()` use the same validation and callback path, including disabled ancestors and parent-modal blocking.

The active window's tree appears in the macOS system menu bar, alongside standard application commands. When a dialog receives focus, its parent's tree remains active. `Dialog` has no menu-setting interface. The platform-neutral tree is intended for native in-window menu bars in future Windows/Linux backends; none are currently implemented.

A modeless dialog is a secondary window associated with its parent; `show()` presents it without blocking parent interaction. `show_modal(completion)` instead presents a parent-modal sheet and returns immediately. The application event loop continues normally. Presenting a second sheet for the same parent returns an error. `dismiss(accepted)` or `dismiss(cancelled)` closes the dialog and completes exactly once; user close, wrapper destruction, and parent closure cancel it. Do not hide a modal dialog: dismiss it. Modal completion is delivered before `on_closed`. Create a fresh dialog for another presentation.

The caller must retain the dialog wrapper while it is needed. A temporary `unique_ptr` that immediately goes out of scope closes the native dialog too.

## Floating utility windows

`WindowOptions::floating` creates an app-local floating utility panel with the normal minimize control. It hides when the application deactivates and retains its level when another application window is focused. The default remains a normal window. `Surface::minimize()` and `is_minimized()` expose minimization; `show()` restores a minimized surface.
