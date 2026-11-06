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

mod alert;
mod collections;
mod color;
mod control_semantics;
mod date;
mod datetime;
mod deadlines;
mod draft;
mod editing;
mod fields;
mod geometry;
mod host;
mod input;
mod inspector;
mod layout;
mod locale;
mod menu;
mod metrics;
mod overlay;
mod prefix;
mod presentation;
mod repeat;
mod rich_text;
mod scroll;
mod split;
mod state;
mod tabs;
pub mod text;
mod text_buffer;
mod time;
mod tooltip;
mod value_edit;
mod values;
mod view;
mod wizard;
mod zoned;

pub use alert::{Alert, AlertAction, AlertActionRole, AlertReason, AlertResult, AlertSubscription};
pub use collections::*;
pub use date::{CalendarDay, CalendarInfo, DatePicker};
pub use datetime::DateTimePicker;
pub use draft::{
    Draft, Number, Numeric, StepperInfo, Validation, ValidationStatus, numeric_field, stepper,
};
pub use editing::TextInput;
pub use fields::TextFieldKind;
pub use geometry::*;
pub use host::{Error, Host, Node, NodeId};
pub use input::{Key, KeyEvent, Modifiers, Pointer, PointerKind};
pub use inspector::{
    Inspection, InspectionCollection, InspectionFlexLine, InspectionHighlight, InspectionOverlay,
    Inspector, InspectorPage, StyleOverride, StyleProperty,
};
pub use locale::EditLocale;
pub use menu::{Menu, MenuAction, MenuInfo, MenuItemInfo, SplitAction};
pub use metrics::*;
pub use overlay::{DismissReason, OverlayFocus, OverlayOptions, OverlayPlacement, overlay_bounds};
pub use presentation::*;
pub use repeat::Repeat;
pub use rich_text::{Inline, Span, SpanStyle};
pub use scars_ui_macros::view;
pub use scroll::{ScrollAxes, ScrollInfo, Scrollbar};
pub use split::{PointerCursor, Split, SplitInfo, SplitOptions, SplitPane, split, split_view};
pub use state::{Binding, State};
pub use tabs::{Tab, TabItemInfo, Tabs, TabsInfo, tab_view, tabs};
pub use text_buffer::{Selection, TextBuffer, TextCommand, TextOptions};
pub use time::{HourCycle, TimePart, TimePicker};
pub use tooltip::{TooltipOptions, TooltipPresentation};
pub use values::*;
pub use view::{CheckState, Kind, OptionItem, Value, View};
pub use wizard::{
    Wizard, WizardChange, WizardInfo, WizardIssue, WizardOptions, WizardStep, WizardStepInfo,
    wizard, wizard_navigation, wizard_view,
};

pub use zoned::{RepeatedTimePolicy, ZoneChange, ZonedPicker};

pub use color::{ColorChannel, ColorInfo, ColorModel, ColorPicker, Hsv};
