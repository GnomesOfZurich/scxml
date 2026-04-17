#![warn(missing_docs)]

//! # scxml: rkyv-native W3C SCXML statechart library
//!
//! A serialization, validation, and visualization library for Harel statecharts.
//! **Not a runtime executor.** Compiled native types remain the fast path.
//!
//! ## Quick start
//!
//! ```rust
//! use scxml::{parse_xml, validate, export};
//!
//! let xml = r#"
//!     <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="draft">
//!         <state id="draft">
//!             <transition event="submit" target="review"/>
//!         </state>
//!         <state id="review">
//!             <transition event="approve" target="done"/>
//!         </state>
//!         <final id="done"/>
//!     </scxml>
//! "#;
//!
//! let chart = parse_xml(xml).unwrap();
//! validate(&chart).unwrap();
//! let dot = export::dot::to_dot(&chart);
//! assert!(dot.contains("draft"));
//! ```

/// Ergonomic statechart construction.
pub mod builder;
/// Semantic comparison of two statecharts.
pub mod diff;
/// Error types for parsing, validation, and simulation.
pub mod error;
/// Core model types: Statechart, State, Transition, Action, DataModel.
pub mod model;
/// Parsers for statechart definition formats.
pub mod parse;
/// Lightweight test executor for event simulation.
pub mod simulate;
/// Summary metrics for a statechart.
pub mod stats;

/// Structural, liveness, and semantic validation.
#[cfg(feature = "validate")]
pub mod validate;

/// Input sanitization for untrusted SCXML (re-exported from [`parse::sanitize`]).
#[cfg(feature = "xml")]
pub use parse::sanitize;

/// Pre-built state index for O(1) lookups.
pub mod index;

/// Export statecharts to DOT, Mermaid, XML, and JSON.
#[cfg(feature = "export")]
pub mod export;

/// Flat state/transition lists for frontend rendering.
#[cfg(feature = "export")]
pub mod flatten;

#[cfg(feature = "xstate")]
pub mod xstate;

#[cfg(feature = "wasm")]
pub mod wasm;

/// Maximum nesting depth for recursive operations on the state tree.
///
/// Prevents stack overflow when processing deeply nested statecharts built
/// programmatically (untrusted input is already limited by `InputLimits::max_depth`).
/// Override at compile time with `--cfg scxml_max_depth="512"` or set a custom
/// value via [`set_max_depth`] at runtime.
///
/// Default: 64 (generous for any real workflow; `parse_untrusted` defaults to 20).
pub const DEFAULT_MAX_DEPTH: usize = 64;

static MAX_DEPTH: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(DEFAULT_MAX_DEPTH);

/// Override the maximum nesting depth for recursive operations.
///
/// Call this once at startup if the default of 64 is too low or too high.
/// Affects all export, validation, diff, and simulation functions.
pub fn set_max_depth(depth: usize) {
    MAX_DEPTH.store(depth, std::sync::atomic::Ordering::Relaxed);
}

/// Returns the current maximum nesting depth.
pub fn max_depth() -> usize {
    MAX_DEPTH.load(std::sync::atomic::Ordering::Relaxed)
}

// Re-export top-level API.
pub use error::{Result, ScxmlError};
pub use index::StateIndex;
pub use model::{
    Action, ActionKind, Binding, DataItem, DataModel, HistoryKind, IfBranch, State, StateKind,
    Statechart, Transition, TransitionType,
};
pub use stats::{StatechartStats, stats};

#[cfg(feature = "validate")]
pub use validate::{
    ValidationReport, validate, validate_all, validate_report, validate_report_with_hash,
};

#[cfg(feature = "export")]
pub use flatten::{FlatState, FlatTransition, flatten};

/// Parse a W3C SCXML XML string into a [`Statechart`].
#[cfg(feature = "xml")]
pub fn parse_xml(xml: &str) -> Result<Statechart> {
    parse::xml::parse_xml(xml)
}

/// Parse a JSON string into a [`Statechart`].
#[cfg(feature = "json")]
pub fn parse_json(json: &str) -> Result<Statechart> {
    parse::json::parse_json(json)
}

/// Parse an XState v5 machine JSON string into a [`Statechart`].
#[cfg(feature = "xstate")]
pub fn parse_xstate(json: &str) -> Result<Statechart> {
    xstate::import::parse_xstate(json)
}

/// Export a [`Statechart`] to XState v5 machine JSON (pretty-printed).
#[cfg(feature = "xstate")]
pub fn to_xstate(chart: &Statechart) -> std::result::Result<String, serde_json::Error> {
    xstate::export::to_xstate(chart)
}

/// Access an archived [`Statechart`] from bytes with validation.
///
/// This is the safe way to access rkyv-serialized statecharts from untrusted
/// sources (e.g., Valkey, memory-mapped files). It runs `check_archived_root`
/// to validate the byte layout before returning a reference to the archived data.
///
/// For trusted bytes from your own serialization, you can use
/// `rkyv::api::high::access` directly for slightly less overhead.
#[cfg(feature = "rkyv")]
pub fn from_archived_bytes(
    bytes: &[u8],
) -> std::result::Result<&model::statechart::ArchivedStatechart, rkyv::rancor::Error> {
    rkyv::api::high::access::<model::statechart::ArchivedStatechart, rkyv::rancor::Error>(bytes)
}
