//! Portable reactive components, layout, editing, and input.
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

mod deadlines;
mod draft;
mod editing;
mod geometry;
mod host;
mod input;
mod layout;
mod metrics;
mod state;
pub mod text;
mod view;

pub use draft::{Draft, Number, Numeric, numeric_field};
pub use editing::{Selection, TextCommand, TextInput};
pub use geometry::*;
pub use host::{Error, Host, Node, NodeId};
pub use input::{Key, KeyEvent, Modifiers, Pointer, PointerKind};
pub use metrics::*;
pub use scars_ui_macros::view;
pub use state::{Binding, State};
pub use view::{CheckState, Kind, OptionItem, Value, View};
