//! Select the native backend. Platform implementation belongs in its own folder.

#[cfg(target_os = "macos")]
#[path = "macos/mod.rs"]
mod backend;

#[cfg(target_os = "macos")]
pub use backend::*;
