//! Native windows, sheets, menus, appearance controls, input and Skia canvases.
//!
//! Use `Window::builder` and `Application::run_with_windows` for application-owned
//! windows and callbacks that receive their window directly. All native objects
//! and callbacks belong to the main thread. Explicit `Window`/`Dialog` owners close
//! on drop; `Surface` clones provide control without extending the open lifetime.
//! See the README and `examples/windows.rs` for an executable example.

mod platform;
mod types;
pub mod ui;

// Unsupported targets expose only the shared contracts until a backend exists.
#[allow(unused_imports)]
pub use platform::*;
pub use types::*;

/// Shared drawing types are available independently of the native window backend.
pub use skia_safe as skia;
