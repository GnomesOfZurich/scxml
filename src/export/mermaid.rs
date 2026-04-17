//! Export a statechart as a Mermaid stateDiagram-v2.
//!
//! Mermaid renders natively on GitHub, GitLab, Notion, and many other platforms.
//!
//! ```rust
//! use scxml::export::mermaid::to_mermaid;
//! use scxml::model::{State, Statechart, Transition};
//!
//! let chart = Statechart::new("draft", vec![
//!     { let mut s = State::atomic("draft");
//!       s.transitions.push(Transition::new("submit", "done"));
//!       s },
//!     State::final_state("done"),
//! ]);
//!
//! let diagram = to_mermaid(&chart);
//! assert!(diagram.contains("stateDiagram-v2"));
//! ```

use std::fmt;

use super::{IndentCache, escape_mermaid};
use crate::model::{State, StateKind, Statechart};

/// Export a statechart as a Mermaid stateDiagram-v2, writing to an `impl fmt::Write` sink.
///
/// Produces output suitable for embedding in Markdown:
/// ````markdown
/// ```mermaid
/// stateDiagram-v2
///   [*] --> draft
///   draft --> review : submit
///   review --> done : approve [manager_ok]
///   done : [*]
/// ```
/// ````
pub fn write_mermaid(chart: &Statechart, out: &mut impl fmt::Write) -> fmt::Result {
    let cache = IndentCache::new();

    out.write_str("stateDiagram-v2\n")?;

    // Entry point.
    out.write_str("  [*] --> ")?;
    out.write_str(&escape_mermaid(&chart.initial))?;
    out.write_char('\n')?;

    // Emit states.
    let limit = crate::max_depth();
    for state in &chart.states {
        emit_state_mermaid(out, state, 1, &cache, limit)?;
    }

    Ok(())
}

/// Export a statechart as a Mermaid stateDiagram-v2 string.
///
/// Produces output suitable for embedding in Markdown:
/// ````markdown
/// ```mermaid
/// stateDiagram-v2
///   [*] --> draft
///   draft --> review : submit
///   review --> done : approve [manager_ok]
///   done : [*]
/// ```
/// ````
pub fn to_mermaid(chart: &Statechart) -> String {
    let estimated_states = chart.states.len() * 4;
    let mut out = String::with_capacity(100 + estimated_states * 80);
    write_mermaid(chart, &mut out).expect("String::write_str never fails");
    out
}

fn emit_state_mermaid(
    out: &mut impl fmt::Write,
    state: &State,
    depth: usize,
    cache: &IndentCache,
    limit: usize,
) -> fmt::Result {
    if depth > limit {
        return Ok(());
    }
    let indent = cache.get(depth);

    let esc_id = escape_mermaid(&state.id);

    match state.kind {
        StateKind::Final => {
            out.write_str(indent)?;
            out.write_str(&esc_id)?;
            out.write_str(" --> [*]\n")?;
        }
        StateKind::Compound => {
            out.write_str(indent)?;
            out.write_str("state ")?;
            out.write_str(&esc_id)?;
            out.write_str(" {\n")?;
            if let Some(init) = &state.initial {
                out.write_str(indent)?;
                out.write_str("  [*] --> ")?;
                out.write_str(&escape_mermaid(init))?;
                out.write_char('\n')?;
            } else if let Some(first) = state.children.first() {
                out.write_str(indent)?;
                out.write_str("  [*] --> ")?;
                out.write_str(&escape_mermaid(&first.id))?;
                out.write_char('\n')?;
            }
            for child in &state.children {
                emit_state_mermaid(out, child, depth + 1, cache, limit)?;
            }
            for child in &state.children {
                emit_transitions_mermaid(out, child, depth + 1, cache)?;
            }
            out.write_str(indent)?;
            out.write_str("}\n")?;
            emit_transitions_mermaid(out, state, depth, cache)?;
        }
        StateKind::Parallel => {
            out.write_str(indent)?;
            out.write_str("state ")?;
            out.write_str(&esc_id)?;
            out.write_str(" {\n")?;
            for (i, child) in state.children.iter().enumerate() {
                if i > 0 {
                    out.write_str(indent)?;
                    out.write_str("  --\n")?;
                }
                emit_state_mermaid(out, child, depth + 1, cache, limit)?;
                for grandchild in &child.children {
                    emit_transitions_mermaid(out, grandchild, depth + 1, cache)?;
                }
            }
            out.write_str(indent)?;
            out.write_str("}\n")?;
            emit_transitions_mermaid(out, state, depth, cache)?;
        }
        StateKind::History(_) => {
            out.write_str(indent)?;
            out.write_str("note right of ")?;
            out.write_str(&esc_id)?;
            out.write_str(" : history\n")?;
        }
        StateKind::Atomic => {
            emit_transitions_mermaid(out, state, depth, cache)?;
        }
    }
    Ok(())
}

fn emit_transitions_mermaid(
    out: &mut impl fmt::Write,
    state: &State,
    depth: usize,
    cache: &IndentCache,
) -> fmt::Result {
    let indent = cache.get(depth);

    for t in &state.transitions {
        for target in &t.targets {
            let has_label = t.event.is_some() || t.guard.is_some() || t.delay.is_some();

            out.write_str(indent)?;
            out.write_str(&escape_mermaid(&state.id))?;
            out.write_str(" --> ")?;
            out.write_str(&escape_mermaid(target))?;

            if has_label {
                out.write_str(" : ")?;
                let mut need_space = false;
                if let Some(event) = &t.event {
                    out.write_str(&escape_mermaid(event))?;
                    need_space = true;
                }
                if let Some(guard) = &t.guard {
                    if need_space {
                        out.write_char(' ')?;
                    }
                    out.write_char('[')?;
                    out.write_str(&escape_mermaid(guard))?;
                    out.write_char(']')?;
                    need_space = true;
                }
                if let Some(delay) = &t.delay {
                    if need_space {
                        out.write_char(' ')?;
                    }
                    out.write_str("after ")?;
                    out.write_str(&escape_mermaid(delay))?;
                }
            }

            out.write_char('\n')?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Transition;

    #[test]
    fn simple_mermaid() {
        let chart = Statechart::new(
            "draft",
            vec![
                {
                    let mut s = State::atomic("draft");
                    s.transitions.push(Transition::new("submit", "review"));
                    s
                },
                {
                    let mut s = State::atomic("review");
                    s.transitions
                        .push(Transition::new("approve", "done").with_guard("manager_ok"));
                    s.transitions.push(Transition::new("reject", "draft"));
                    s
                },
                State::final_state("done"),
            ],
        );

        let diagram = to_mermaid(&chart);
        assert!(diagram.contains("stateDiagram-v2"));
        assert!(diagram.contains("[*] --> draft"));
        assert!(diagram.contains("draft --> review : submit"));
        assert!(diagram.contains("review --> done : approve [manager_ok]"));
        assert!(diagram.contains("review --> draft : reject"));
        assert!(diagram.contains("done --> [*]"));
    }

    #[test]
    fn mermaid_with_delay() {
        let mut s = State::atomic("pending");
        s.transitions
            .push(Transition::new("timeout", "expired").with_delay("PT48H"));

        let chart = Statechart::new("pending", vec![s, State::final_state("expired")]);
        let diagram = to_mermaid(&chart);
        assert!(diagram.contains("after PT48H"));
    }

    #[test]
    fn mermaid_compound_state() {
        let chart = Statechart::new(
            "main",
            vec![State::compound(
                "main",
                "a",
                vec![
                    {
                        let mut s = State::atomic("a");
                        s.transitions.push(Transition::new("next", "b"));
                        s
                    },
                    State::final_state("b"),
                ],
            )],
        );

        let diagram = to_mermaid(&chart);
        assert!(diagram.contains("state main {"));
        assert!(diagram.contains("[*] --> a"));
        assert!(diagram.contains("a --> b : next"));
    }

    #[test]
    fn mermaid_parallel_state() {
        let chart = Statechart::new(
            "p",
            vec![State::parallel(
                "p",
                vec![
                    State::compound(
                        "r1",
                        "r1a",
                        vec![
                            {
                                let mut s = State::atomic("r1a");
                                s.transitions.push(Transition::new("done", "r1b"));
                                s
                            },
                            State::final_state("r1b"),
                        ],
                    ),
                    State::compound(
                        "r2",
                        "r2a",
                        vec![
                            {
                                let mut s = State::atomic("r2a");
                                s.transitions.push(Transition::new("done", "r2b"));
                                s
                            },
                            State::final_state("r2b"),
                        ],
                    ),
                ],
            )],
        );

        let diagram = to_mermaid(&chart);
        assert!(diagram.contains("state p {"));
        assert!(diagram.contains("--")); // parallel region separator
    }

    #[test]
    fn write_mermaid_matches_to_mermaid() {
        let chart = Statechart::new(
            "a",
            vec![
                {
                    let mut s = State::atomic("a");
                    s.transitions.push(Transition::new("go", "b"));
                    s
                },
                State::final_state("b"),
            ],
        );

        let from_to = to_mermaid(&chart);
        let mut from_write = String::new();
        write_mermaid(&chart, &mut from_write).unwrap();
        assert_eq!(from_to, from_write);
    }
}
