use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use super::action::Action;

/// A transition between states, following W3C SCXML semantics.
///
/// Transitions are triggered by events, optionally guarded by named predicates,
/// and may execute named actions. Guards and actions are **string references**;
/// the calling code provides implementations at evaluation time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[non_exhaustive]
pub struct Transition {
    /// Event that triggers this transition. `None` = eventless (always-enabled).
    pub event: Option<CompactString>,

    /// Named guard predicate. `None` = unconditional.
    /// Resolved by the caller's `GuardEvaluator`, not by this crate.
    #[serde(default)]
    pub guard: Option<CompactString>,

    /// Target state id(s). Multiple targets for parallel state entry.
    /// Empty = self-transition (re-enter current state).
    #[serde(default)]
    pub targets: Vec<CompactString>,

    /// Transition type: internal (no exit/entry of source) or external (default).
    #[serde(default)]
    pub transition_type: TransitionType,

    /// Actions to execute during the transition.
    #[serde(default)]
    pub actions: Vec<Action>,

    /// Delay before this transition fires (ISO 8601 duration, e.g. "PT30M").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delay: Option<CompactString>,

    /// Required approval quorum count (Gnomes extension: `gnomes:quorum="3"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quorum: Option<u32>,
}

/// Whether a transition exits the source state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum TransitionType {
    /// Default: source state is exited and re-entered.
    #[default]
    External,
    /// Source state is NOT exited (only valid when target is a descendant).
    Internal,
}

impl Transition {
    /// Create a simple event-triggered transition to a single target.
    pub fn new(event: impl Into<CompactString>, target: impl Into<CompactString>) -> Self {
        Self {
            event: Some(event.into()),
            guard: None,
            targets: vec![target.into()],
            transition_type: TransitionType::External,
            actions: Vec::new(),
            delay: None,
            quorum: None,
        }
    }

    /// Create an eventless (always-enabled) transition.
    pub fn eventless(target: impl Into<CompactString>) -> Self {
        Self {
            event: None,
            guard: None,
            targets: vec![target.into()],
            transition_type: TransitionType::External,
            actions: Vec::new(),
            delay: None,
            quorum: None,
        }
    }

    /// Add a named guard to this transition.
    pub fn with_guard(mut self, guard: impl Into<CompactString>) -> Self {
        self.guard = Some(guard.into());
        self
    }

    /// Set transition type to internal.
    pub fn internal(mut self) -> Self {
        self.transition_type = TransitionType::Internal;
        self
    }

    /// Add an action to execute during this transition.
    pub fn with_action(mut self, action: Action) -> Self {
        self.actions.push(action);
        self
    }

    /// Set a delay (ISO 8601 duration) for this transition.
    pub fn with_delay(mut self, delay: impl Into<CompactString>) -> Self {
        self.delay = Some(delay.into());
        self
    }

    /// Set a quorum requirement for this transition.
    pub fn with_quorum(mut self, quorum: u32) -> Self {
        self.quorum = Some(quorum);
        self
    }

    // ── Mutating setters (for use with &mut references, e.g. builder) ──

    /// Set guard on a mutable reference (builder-friendly).
    pub fn set_guard(&mut self, guard: impl Into<CompactString>) -> &mut Self {
        self.guard = Some(guard.into());
        self
    }

    /// Set delay on a mutable reference (builder-friendly).
    pub fn set_delay(&mut self, delay: impl Into<CompactString>) -> &mut Self {
        self.delay = Some(delay.into());
        self
    }

    /// Set quorum on a mutable reference (builder-friendly).
    pub fn set_quorum(&mut self, quorum: u32) -> &mut Self {
        self.quorum = Some(quorum);
        self
    }
}
