//! Core model types for W3C SCXML statecharts.

/// Action descriptors (raise, send, assign, log, cancel, if, foreach, script, invoke, custom).
pub mod action;
/// Data model declarations.
pub mod datamodel;
/// State types and iterators.
pub mod state;
/// Statechart root type.
pub mod statechart;
/// Transition types.
pub mod transition;

pub use action::{Action, ActionKind, IfBranch};
pub use datamodel::{DataItem, DataModel};
pub use state::{HistoryKind, State, StateKind};
pub use statechart::{Binding, Statechart};
pub use transition::{Transition, TransitionType};
