// rkyv-generated Archived* types trigger missing_docs on their fields.
// This is a known limitation; suppress at module level.
#![allow(missing_docs)]

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

/// An action descriptor: a named reference to an action, NOT executable code.
///
/// Actions are stored as data in the statechart model. The calling code
/// resolves action names to actual implementations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[cfg_attr(
    feature = "rkyv",
    rkyv(
        serialize_bounds(__S: rkyv::ser::Writer + rkyv::ser::Allocator, <__S as rkyv::rancor::Fallible>::Error: rkyv::rancor::Source),
        deserialize_bounds(__D::Error: rkyv::rancor::Source),
        bytecheck(bounds(
            __C: rkyv::validation::ArchiveContext,
            <__C as rkyv::rancor::Fallible>::Error: rkyv::rancor::Source,
        )),
    )
)]
#[non_exhaustive]
#[allow(missing_docs)] // rkyv-generated ArchivedAction triggers false positive
pub struct Action {
    /// The kind of action.
    #[cfg_attr(feature = "rkyv", rkyv(omit_bounds))]
    pub kind: ActionKind,
}

/// Discriminated action types matching W3C SCXML executable content elements.
/// All are stored as descriptors (names/targets), never executed by this crate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[cfg_attr(
    feature = "rkyv",
    rkyv(
        serialize_bounds(__S: rkyv::ser::Writer + rkyv::ser::Allocator, <__S as rkyv::rancor::Fallible>::Error: rkyv::rancor::Source),
        deserialize_bounds(__D::Error: rkyv::rancor::Source),
        bytecheck(bounds(
            __C: rkyv::validation::ArchiveContext,
            <__C as rkyv::rancor::Fallible>::Error: rkyv::rancor::Source,
        )),
    )
)]
#[non_exhaustive]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(missing_docs)] // rkyv-generated ArchivedActionKind triggers false positive
pub enum ActionKind {
    /// `<raise event="..."/>`: queue an internal event.
    Raise {
        /// Event name to raise.
        event: CompactString,
    },

    /// `<send event="..." target="..."/>`. Sends an event to an external target.
    Send {
        /// Event name to send.
        event: CompactString,
        /// Optional target for the event.
        target: Option<CompactString>,
        /// Optional delay before sending.
        delay: Option<CompactString>,
    },

    /// `<assign location="..." expr="..."/>`. Data mutation descriptor.
    Assign {
        /// Data location to assign to.
        location: CompactString,
        /// Value expression (stored as string, not evaluated).
        expr: CompactString,
    },

    /// `<log label="..." expr="..."/>`. Logging descriptor.
    Log {
        /// Optional log label.
        label: Option<CompactString>,
        /// Optional log expression.
        expr: Option<CompactString>,
    },

    /// `<cancel sendid="..."/>`. Cancels a delayed send.
    Cancel {
        /// The send ID to cancel.
        sendid: CompactString,
    },

    /// `<if cond="..."> ... <elseif cond="..."/> ... <else/> ... </if>` conditional block.
    /// Children are stored as a flat sequence of actions. Condition boundaries
    /// are preserved via the `branches` field.
    If {
        /// Condition branches: `(guard, action_count)` pairs.
        /// The first entry is the `<if cond>` itself. Subsequent entries are
        /// `<elseif cond>`. An `<else>` has `guard: None`.
        branches: Vec<IfBranch>,
        /// All actions across all branches, concatenated. Use `branches` to
        /// determine which actions belong to which condition.
        #[cfg_attr(feature = "rkyv", rkyv(omit_bounds))]
        actions: Vec<Action>,
    },

    /// `<foreach array="..." item="..." index="..."> ... </foreach>` iteration block.
    Foreach {
        /// Array expression to iterate over.
        array: CompactString,
        /// Variable name bound to the current item.
        item: CompactString,
        /// Optional variable name bound to the current index.
        index: Option<CompactString>,
        /// Actions executed for each iteration.
        #[cfg_attr(feature = "rkyv", rkyv(omit_bounds))]
        actions: Vec<Action>,
    },

    /// `<script>...</script>`: inline script content (stored, never executed).
    Script {
        /// The script source text.
        content: CompactString,
    },

    /// `<invoke>` child session invocation descriptor (stored, never executed).
    Invoke {
        /// Service type (e.g. `"scxml"`, `"http://www.w3.org/TR/scxml/"`).
        invoke_type: Option<CompactString>,
        /// Source URI for the invoked service.
        src: Option<CompactString>,
        /// Invocation identifier.
        id: Option<CompactString>,
    },

    /// Custom named action, resolved by the caller.
    Custom {
        /// Action name.
        name: CompactString,
        /// Optional key-value parameters.
        #[serde(default)]
        params: Vec<(CompactString, CompactString)>,
    },
}

/// A branch in an `<if>` / `<elseif>` / `<else>` conditional block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[non_exhaustive]
pub struct IfBranch {
    /// The guard condition. `None` for `<else>`.
    pub guard: Option<CompactString>,
    /// Number of actions in this branch (index into the parent's `actions` vec).
    pub action_count: usize,
}

impl Action {
    /// Create a raise action.
    pub fn raise(event: impl Into<CompactString>) -> Self {
        Self {
            kind: ActionKind::Raise {
                event: event.into(),
            },
        }
    }

    /// Create a send action.
    pub fn send(event: impl Into<CompactString>) -> Self {
        Self {
            kind: ActionKind::Send {
                event: event.into(),
                target: None,
                delay: None,
            },
        }
    }

    /// Create a custom named action.
    pub fn custom(name: impl Into<CompactString>) -> Self {
        Self {
            kind: ActionKind::Custom {
                name: name.into(),
                params: Vec::new(),
            },
        }
    }

    /// Create an assign action descriptor.
    pub fn assign(location: impl Into<CompactString>, expr: impl Into<CompactString>) -> Self {
        Self {
            kind: ActionKind::Assign {
                location: location.into(),
                expr: expr.into(),
            },
        }
    }

    /// Create a log action descriptor.
    pub fn log(label: Option<CompactString>, expr: Option<CompactString>) -> Self {
        Self {
            kind: ActionKind::Log { label, expr },
        }
    }

    /// Create a send action with a target.
    pub fn send_to(event: impl Into<CompactString>, target: impl Into<CompactString>) -> Self {
        Self {
            kind: ActionKind::Send {
                event: event.into(),
                target: Some(target.into()),
                delay: None,
            },
        }
    }

    /// Create a cancel action descriptor.
    pub fn cancel(sendid: impl Into<CompactString>) -> Self {
        Self {
            kind: ActionKind::Cancel {
                sendid: sendid.into(),
            },
        }
    }

    /// Create a script action descriptor.
    pub fn script(content: impl Into<CompactString>) -> Self {
        Self {
            kind: ActionKind::Script {
                content: content.into(),
            },
        }
    }

    /// Create an invoke action descriptor.
    pub fn invoke(
        invoke_type: Option<CompactString>,
        src: Option<CompactString>,
        id: Option<CompactString>,
    ) -> Self {
        Self {
            kind: ActionKind::Invoke {
                invoke_type,
                src,
                id,
            },
        }
    }
}
