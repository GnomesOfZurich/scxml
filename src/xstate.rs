//! XState v5 JSON import and export.
//!
//! This module converts between XState v5 machine configuration JSON and the
//! crate's canonical [`Statechart`](crate::model::statechart::Statechart) model.
//!
//! Requires the `xstate` feature flag.
//!
//! # Mapping summary
//!
//! | XState | SCXML model |
//! |--------|-------------|
//! | `states: { name: { ... } }` | `Vec<State>` with `id` |
//! | `on: { EVENT: target }` | `Transition { event, targets }` |
//! | `always` | `Transition { event: None }` |
//! | `after: { ms: target }` | `Transition { delay }` (ISO 8601) |
//! | `type: "final"` | `StateKind::Final` |
//! | `type: "parallel"` | `StateKind::Parallel` |
//! | `type: "history"` | `StateKind::History` |
//! | `guard` | `Transition.guard` |
//! | `entry` / `exit` | `on_entry` / `on_exit` |
//! | `context` | `DataModel` |

pub mod export;
pub mod import;
pub(crate) mod types;

pub use export::{to_xstate, to_xstate_value};
pub use import::{parse_xstate, parse_xstate_value};
