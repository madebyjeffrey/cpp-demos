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

mod collections;
mod deadlines;
mod draft;
mod editing;
mod geometry;
mod host;
mod input;
mod inspector;
mod layout;
mod metrics;
mod presentation;
mod rich_text;
mod scroll;
mod state;
pub mod text;
mod text_buffer;
mod values;
mod view;

pub use collections::*;
pub use draft::{Draft, Number, Numeric, numeric_field};
pub use editing::TextInput;
pub use geometry::*;
pub use host::{Error, Host, Node, NodeId};
pub use input::{Key, KeyEvent, Modifiers, Pointer, PointerKind};
pub use inspector::{Inspection, Inspector};
pub use metrics::*;
pub use presentation::*;
pub use rich_text::{Inline, Span, SpanStyle};
pub use scars_ui_macros::view;
pub use scroll::{ScrollAxes, ScrollInfo, Scrollbar};
pub use state::{Binding, State};
pub use text_buffer::{Selection, TextBuffer, TextCommand, TextOptions};
pub use values::*;
pub use view::{CheckState, Kind, OptionItem, Value, View};
