use std::fmt;

use super::{IndentCache, escape_dot};
use crate::model::{State, StateKind, Statechart};

/// Export a statechart as a DOT graph, writing to an `impl fmt::Write` sink.
///
/// State colors follow lifecycle categories:
/// - Atomic: `#4A90D9` (blue)
/// - Compound: `#7B68EE` (purple)
/// - Parallel: `#FF8C00` (orange)
/// - Final: `#2ECC71` (green)
/// - History: `#95A5A6` (grey)
pub fn write_dot(chart: &Statechart, out: &mut impl fmt::Write) -> fmt::Result {
    let cache = IndentCache::new();

    out.write_str("digraph statechart {\n")?;
    out.write_str("  rankdir=TB;\n")?;
    out.write_str("  node [fontname=\"Helvetica\", fontsize=11];\n")?;
    out.write_str("  edge [fontname=\"Helvetica\", fontsize=9];\n")?;
    out.write_str("  compound=true;\n\n")?;

    // Entry point.
    out.write_str("  __start [shape=point, width=0.2, height=0.2];\n")?;
    out.write_str("  __start -> \"")?;
    out.write_str(&escape_dot(&chart.initial))?;
    out.write_str("\";\n\n")?;

    // Emit states.
    let limit = crate::max_depth();
    for state in &chart.states {
        emit_state(out, state, 1, &cache, limit)?;
    }

    out.write_str("}\n")
}

/// Export a statechart as a DOT graph for Graphviz rendering.
///
/// State colors follow lifecycle categories:
/// - Atomic: `#4A90D9` (blue)
/// - Compound: `#7B68EE` (purple)
/// - Parallel: `#FF8C00` (orange)
/// - Final: `#2ECC71` (green)
/// - History: `#95A5A6` (grey)
pub fn to_dot(chart: &Statechart) -> String {
    let estimated_states = chart.states.len() * 4;
    let mut out = String::with_capacity(200 + estimated_states * 150);
    write_dot(chart, &mut out).expect("String::write_str never fails");
    out
}

fn emit_state(
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

    let esc_id = escape_dot(&state.id);

    if state.is_composite() {
        // Subgraph for compound/parallel states.
        out.write_str(indent)?;
        out.write_str("subgraph \"cluster_")?;
        out.write_str(&esc_id)?;
        out.write_str("\" {\n")?;
        out.write_str(indent)?;
        out.write_str("  label=\"")?;
        out.write_str(&esc_id)?;
        out.write_str("\";\n")?;

        let (style, color) = match state.kind {
            StateKind::Parallel => ("dashed", "#FF8C00"),
            _ => ("solid", "#7B68EE"),
        };
        out.write_str(indent)?;
        out.write_str("  style=")?;
        out.write_str(style)?;
        out.write_str("; color=\"")?;
        out.write_str(color)?;
        out.write_str("\";\n")?;

        for child in &state.children {
            emit_state(out, child, depth + 1, cache, limit)?;
        }

        // Emit transitions from the composite state itself.
        emit_transitions(out, state, indent)?;

        out.write_str(indent)?;
        out.write_str("}\n")?;
    } else {
        // Leaf node.
        let (shape, color) = match state.kind {
            StateKind::Final => ("doublecircle", "#2ECC71"),
            StateKind::History(_) => ("diamond", "#95A5A6"),
            _ => ("box", "#4A90D9"),
        };

        // Build label with entry/exit annotations.
        let mut node_label = esc_id.to_string();
        if !state.on_entry.is_empty() {
            let actions: Vec<String> = state
                .on_entry
                .iter()
                .map(|a| escape_dot(&action_summary(a)).into_owned())
                .collect();
            node_label.push_str("\\nentry/ ");
            node_label.push_str(&actions.join(", "));
        }
        if !state.on_exit.is_empty() {
            let actions: Vec<String> = state
                .on_exit
                .iter()
                .map(|a| escape_dot(&action_summary(a)).into_owned())
                .collect();
            node_label.push_str("\\nexit/ ");
            node_label.push_str(&actions.join(", "));
        }

        out.write_str(indent)?;
        out.write_char('"')?;
        out.write_str(&esc_id)?;
        out.write_str("\" [shape=")?;
        out.write_str(shape)?;
        out.write_str(", style=filled, fillcolor=\"")?;
        out.write_str(color)?;
        out.write_str("\", fontcolor=white, label=\"")?;
        out.write_str(&node_label)?;
        out.write_str("\"];\n")?;

        emit_transitions(out, state, indent)?;
    }
    Ok(())
}

fn action_summary(action: &crate::model::Action) -> String {
    match &action.kind {
        crate::model::ActionKind::Raise { event } => {
            let mut s = String::with_capacity(8 + event.len());
            s.push_str("raise(");
            s.push_str(event);
            s.push(')');
            s
        }
        crate::model::ActionKind::Send { event, .. } => {
            let mut s = String::with_capacity(7 + event.len());
            s.push_str("send(");
            s.push_str(event);
            s.push(')');
            s
        }
        crate::model::ActionKind::Assign { location, .. } => {
            let mut s = String::with_capacity(9 + location.len());
            s.push_str("assign(");
            s.push_str(location);
            s.push(')');
            s
        }
        crate::model::ActionKind::Log { label, .. } => {
            let mut s = String::with_capacity(6 + label.as_ref().map_or(0, |l| l.len()));
            s.push_str("log(");
            if let Some(l) = label {
                s.push_str(l);
            }
            s.push(')');
            s
        }
        crate::model::ActionKind::Cancel { sendid } => {
            let mut s = String::with_capacity(9 + sendid.len());
            s.push_str("cancel(");
            s.push_str(sendid);
            s.push(')');
            s
        }
        crate::model::ActionKind::If { .. } => "if(...)".to_string(),
        crate::model::ActionKind::Foreach { array, .. } => {
            let mut s = String::with_capacity(10 + array.len());
            s.push_str("foreach(");
            s.push_str(array);
            s.push(')');
            s
        }
        crate::model::ActionKind::Script { .. } => "script".to_string(),
        crate::model::ActionKind::Invoke { src, .. } => {
            let mut s = String::with_capacity(9 + src.as_ref().map_or(0, |s| s.len()));
            s.push_str("invoke(");
            if let Some(src) = src {
                s.push_str(src);
            }
            s.push(')');
            s
        }
        crate::model::ActionKind::Custom { name, .. } => name.to_string(),
    }
}

fn emit_transitions(out: &mut impl fmt::Write, state: &State, indent: &str) -> fmt::Result {
    for t in &state.transitions {
        for target in &t.targets {
            // Build label inline.
            let has_label =
                t.event.is_some() || t.guard.is_some() || t.delay.is_some() || t.quorum.is_some();

            let esc_src = escape_dot(&state.id);
            let esc_tgt = escape_dot(target);

            out.write_str(indent)?;
            out.write_char('"')?;
            out.write_str(&esc_src)?;
            out.write_str("\" -> \"")?;
            out.write_str(&esc_tgt)?;
            out.write_char('"')?;

            if has_label || t.delay.is_some() {
                out.write_str(" [")?;
                let mut need_comma = false;

                if has_label {
                    out.write_str("label=\"")?;
                    let mut need_space = false;
                    if let Some(ref event) = t.event {
                        out.write_str(&escape_dot(event))?;
                        need_space = true;
                    }
                    if let Some(ref guard) = t.guard {
                        if need_space {
                            out.write_char(' ')?;
                        }
                        out.write_char('[')?;
                        out.write_str(&escape_dot(guard))?;
                        out.write_char(']')?;
                        need_space = true;
                    }
                    if let Some(ref delay) = t.delay {
                        if need_space {
                            out.write_char(' ')?;
                        }
                        out.write_char('\u{23F1}')?;
                        out.write_str(&escape_dot(delay))?;
                        need_space = true;
                    }
                    if let Some(quorum) = t.quorum {
                        if need_space {
                            out.write_char(' ')?;
                        }
                        out.write_char('\u{26A1}')?;
                        write!(out, "{quorum}")?;
                    }
                    out.write_char('"')?;
                    need_comma = true;
                }

                // Style deadline transitions with dashed lines.
                if t.delay.is_some() {
                    if need_comma {
                        out.write_str(", ")?;
                    }
                    out.write_str("style=dashed, color=\"#E74C3C\"")?;
                }

                out.write_char(']')?;
            }
            out.write_str(";\n")?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{State, Transition};

    #[test]
    fn dot_output_has_structure() {
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
                    s
                },
                State::final_state("done"),
            ],
        );

        let dot = to_dot(&chart);
        assert!(dot.contains("digraph statechart"));
        assert!(dot.contains("__start"));
        assert!(dot.contains("\"draft\""));
        assert!(dot.contains("\"review\""));
        assert!(dot.contains("\"done\""));
        assert!(dot.contains("[manager_ok]"));
        assert!(dot.contains("doublecircle"));
    }

    #[test]
    fn write_dot_matches_to_dot() {
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

        let from_to = to_dot(&chart);
        let mut from_write = String::new();
        write_dot(&chart, &mut from_write).unwrap();
        assert_eq!(from_to, from_write);
    }
}
