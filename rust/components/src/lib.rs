//! Portable reactive descriptions and mounted identity. Native widgets arrive in stage 2.
//! State and hosts stay on their creating thread. A host rebuilds lazily after
//! observed state changes; actions execute without holding host or state borrows.
//!
//! ```
//! use scars_ui::{Host, State, view};
//! let name = State::new(String::from("Rust"));
//! let host = Host::new(move || view! { column { label(name.get(), key: "name"); } });
//! assert_eq!(host.snapshot().unwrap().last().unwrap().text, "Rust");
//! ```
//!
//! Properties are checked at the element, not silently ignored:
//! ```compile_fail
//! use scars_ui::view;
//! let _ = view! { label("Hello", unknown: true); };
//! ```
//! Binding/argument type errors are checked by Rust:
//! ```compile_fail
//! use scars_ui::view;
//! let _ = view! { label(42); };
//! ```

extern crate self as scars_ui;

mod host;
mod state;
pub mod text;
mod view;

pub use host::{Error, Host, Node, NodeId};
pub use scars_ui_macros::view;
pub use state::{Binding, State};
pub use view::{Kind, View};
