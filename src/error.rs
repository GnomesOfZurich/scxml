/// All errors produced by this crate.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ScxmlError {
    // ── Parse errors ────────────────────────────────────────────────
    /// XML parse or sanitization error.
    #[error("XML parse error: {0}")]
    Xml(String),

    /// JSON parse error.
    #[error("JSON parse error: {0}")]
    Json(String),

    /// A required XML attribute is missing.
    #[error("missing required attribute '{attribute}' on <{element}>")]
    MissingAttribute {
        /// The element that was missing the attribute.
        element: &'static str,
        /// The name of the missing attribute.
        attribute: &'static str,
    },

    /// An unrecognised state kind string was encountered.
    #[error("invalid state kind '{0}'")]
    InvalidStateKind(String),

    /// An unrecognised history kind string was encountered.
    #[error("invalid history kind '{0}', expected \"shallow\" or \"deep\"")]
    InvalidHistoryKind(String),

    /// An unrecognised transition type string was encountered.
    #[error("invalid transition type '{0}', expected \"internal\" or \"external\"")]
    InvalidTransitionType(String),

    /// XState v5 JSON parse or conversion error.
    #[error("XState JSON error: {0}")]
    XState(String),

    /// The state tree exceeds the maximum nesting depth.
    #[error("state tree exceeds maximum depth of {limit} at state '{state}'")]
    DepthLimitExceeded {
        /// The state where the limit was hit.
        state: String,
        /// The configured depth limit.
        limit: usize,
    },

    // ── Structural validation errors ────────────────────────────────
    /// A transition targets a state that does not exist.
    #[error("transition in state '{src}' targets unknown state '{target}'")]
    UnknownTarget {
        /// The source state containing the transition.
        src: String,
        /// The target state that was not found.
        target: String,
    },

    /// Two or more states share the same id.
    #[error("duplicate state id '{0}'")]
    DuplicateStateId(String),

    /// The chart's initial state does not exist.
    #[error("initial state '{0}' does not exist")]
    InvalidInitial(String),

    /// A final state has outgoing transitions (not allowed by W3C).
    #[error("final state '{0}' has outgoing transitions")]
    FinalHasTransitions(String),

    /// A compound state has no initial child state.
    #[error("compound state '{0}' has no initial child")]
    CompoundNoInitial(String),

    /// A parallel state has fewer than two child regions.
    #[error("parallel state '{0}' must have at least 2 child regions")]
    ParallelTooFewRegions(String),

    /// A history state exists at the top level (no parent compound/parallel).
    #[error("history state '{0}' has no parent compound/parallel state")]
    OrphanHistory(String),

    // ── Liveness validation errors ──────────────────────────────────
    /// A state cannot be reached from the initial configuration.
    #[error("state '{0}' is unreachable from the initial configuration")]
    Unreachable(String),

    /// A non-final state has no outgoing transitions (potential deadlock).
    #[error("non-final state '{0}' has no outgoing transitions (potential deadlock)")]
    Deadlock(String),

    // ── Simulation errors ──────────────────────────────────────────
    /// No transition matches the event in the current state.
    #[error("no transition for event '{event}' in state '{state}'")]
    SimNoTransition {
        /// The current state.
        state: String,
        /// The event that had no matching transition.
        event: String,
    },

    /// The machine is in a final state and cannot accept further events.
    #[error("state '{state}' is final; no further transitions")]
    SimFinal {
        /// The final state.
        state: String,
    },

    /// A guard condition prevented the transition from firing.
    #[error("guard '{guard}' blocked transition on '{event}' in state '{state}'")]
    SimGuardBlocked {
        /// The current state.
        state: String,
        /// The event that was sent.
        event: String,
        /// The guard that blocked the transition.
        guard: String,
    },
}

/// Convenience alias.
pub type Result<T> = std::result::Result<T, ScxmlError>;
