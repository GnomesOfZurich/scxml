use std::fmt;

use super::{IndentCache, escape_xml_attr};
use crate::model::*;

/// Write ` name="escaped_value"` to the output.
fn attr(out: &mut impl fmt::Write, name: &str, value: &str) -> fmt::Result {
    out.write_char(' ')?;
    out.write_str(name)?;
    out.write_str("=\"")?;
    out.write_str(&escape_xml_attr(value))?;
    out.write_char('"')
}

/// Export a statechart back to W3C SCXML XML, writing to an `impl fmt::Write` sink.
///
/// Produces a well-formed SCXML document suitable for roundtrip
/// (parse -> modify -> export -> parse again).
pub fn write_xml(chart: &Statechart, out: &mut impl fmt::Write) -> fmt::Result {
    let cache = IndentCache::new();
    let limit = crate::max_depth();

    out.write_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n")?;
    out.write_str("<scxml")?;
    attr(out, "xmlns", &chart.xmlns)?;
    // Emit gnomes namespace if any transition uses quorum.
    let has_quorum = chart
        .iter_all_states()
        .any(|s| s.transitions.iter().any(|t| t.quorum.is_some()));
    if has_quorum {
        attr(out, "xmlns:gnomes", "http://gnomes.dev/scxml")?;
    }
    attr(out, "version", &chart.version)?;
    attr(out, "initial", &chart.initial)?;
    if let Some(ref name) = chart.name {
        attr(out, "name", name)?;
    }
    if chart.binding != Binding::Early {
        attr(out, "binding", "late")?;
    }
    out.write_str(">\n")?;

    // Datamodel.
    if !chart.datamodel.items.is_empty() {
        out.write_str("  <datamodel>\n")?;
        for item in &chart.datamodel.items {
            out.write_str("    <data")?;
            attr(out, "id", &item.id)?;
            if let Some(ref expr) = item.expr {
                attr(out, "expr", expr)?;
            }
            if let Some(ref src) = item.src {
                attr(out, "src", src)?;
            }
            out.write_str("/>\n")?;
        }
        out.write_str("  </datamodel>\n")?;
    }

    // States.
    for state in &chart.states {
        emit_state_xml(out, state, 1, limit, &cache)?;
    }

    out.write_str("</scxml>\n")
}

/// Export a statechart back to W3C SCXML XML.
///
/// Produces a well-formed SCXML document suitable for roundtrip
/// (parse -> modify -> export -> parse again).
pub fn to_xml(chart: &Statechart) -> String {
    // Estimate total states from top-level count. Compound/parallel states
    // typically have 3-5 children, so multiply by 4 as a heuristic.
    let estimated_states = chart.states.len() * 4;
    let mut out = String::with_capacity(200 + estimated_states * 120);
    write_xml(chart, &mut out).expect("String::write_str never fails");
    out
}

fn emit_state_xml(
    out: &mut impl fmt::Write,
    state: &State,
    depth: usize,
    limit: usize,
    cache: &IndentCache,
) -> fmt::Result {
    if depth > limit {
        return Ok(());
    }
    let indent = cache.get(depth);

    let (tag, self_closing) = match state.kind {
        StateKind::Final => (
            "final",
            state.on_entry.is_empty() && state.on_exit.is_empty(),
        ),
        StateKind::Parallel => ("parallel", false),
        StateKind::History(kind) => {
            // History state.
            out.write_str(indent)?;
            out.write_str("<history")?;
            attr(out, "id", &state.id)?;
            match kind {
                HistoryKind::Deep => attr(out, "type", "deep")?,
                HistoryKind::Shallow => {} // shallow is default
            }
            if state.transitions.is_empty() {
                out.write_str("/>\n")?;
            } else {
                out.write_str(">\n")?;
                for t in &state.transitions {
                    emit_transition_xml(out, t, depth + 1, cache)?;
                }
                out.write_str(indent)?;
                out.write_str("</history>\n")?;
            }
            return Ok(());
        }
        _ => ("state", false),
    };

    // Check if this is a simple leaf with no children, actions, or transitions.
    let is_simple_leaf = state.children.is_empty()
        && state.transitions.is_empty()
        && state.on_entry.is_empty()
        && state.on_exit.is_empty();

    if is_simple_leaf && self_closing {
        out.write_str(indent)?;
        out.write_char('<')?;
        out.write_str(tag)?;
        attr(out, "id", &state.id)?;
        out.write_str("/>\n")?;
        return Ok(());
    }

    // Opening tag.
    out.write_str(indent)?;
    out.write_char('<')?;
    out.write_str(tag)?;
    attr(out, "id", &state.id)?;
    if let Some(ref init) = state.initial {
        attr(out, "initial", init)?;
    }
    if is_simple_leaf {
        out.write_str("/>\n")?;
        return Ok(());
    }
    out.write_str(">\n")?;

    // Entry actions.
    if !state.on_entry.is_empty() {
        out.write_str(indent)?;
        out.write_str("  <onentry>\n")?;
        for action in &state.on_entry {
            emit_action_xml(out, action, depth + 2, cache)?;
        }
        out.write_str(indent)?;
        out.write_str("  </onentry>\n")?;
    }

    // Exit actions.
    if !state.on_exit.is_empty() {
        out.write_str(indent)?;
        out.write_str("  <onexit>\n")?;
        for action in &state.on_exit {
            emit_action_xml(out, action, depth + 2, cache)?;
        }
        out.write_str(indent)?;
        out.write_str("  </onexit>\n")?;
    }

    // Transitions.
    for t in &state.transitions {
        emit_transition_xml(out, t, depth + 1, cache)?;
    }

    // Children.
    for child in &state.children {
        emit_state_xml(out, child, depth + 1, limit, cache)?;
    }

    out.write_str(indent)?;
    out.write_str("</")?;
    out.write_str(tag)?;
    out.write_str(">\n")
}

fn emit_transition_xml(
    out: &mut impl fmt::Write,
    t: &Transition,
    depth: usize,
    cache: &IndentCache,
) -> fmt::Result {
    let indent = cache.get(depth);

    out.write_str(indent)?;
    out.write_str("<transition")?;
    if let Some(ref event) = t.event {
        attr(out, "event", event)?;
    }
    if let Some(ref guard) = t.guard {
        attr(out, "cond", guard)?;
    }
    if !t.targets.is_empty() {
        // Multiple targets are space-separated in one attribute.
        let escaped: Vec<_> = t.targets.iter().map(|t| escape_xml_attr(t)).collect();
        out.write_str(" target=\"")?;
        out.write_str(&escaped.join(" "))?;
        out.write_char('"')?;
    }
    if t.transition_type == TransitionType::Internal {
        attr(out, "type", "internal")?;
    }
    if let Some(delay) = &t.delay {
        attr(out, "delay", delay)?;
    }
    if let Some(quorum) = t.quorum {
        attr(out, "gnomes:quorum", &quorum.to_string())?;
    }

    if t.actions.is_empty() {
        out.write_str("/>\n")
    } else {
        out.write_str(">\n")?;
        for action in &t.actions {
            emit_action_xml(out, action, depth + 1, cache)?;
        }
        out.write_str(indent)?;
        out.write_str("</transition>\n")
    }
}

fn emit_action_xml(
    out: &mut impl fmt::Write,
    action: &Action,
    depth: usize,
    cache: &IndentCache,
) -> fmt::Result {
    let indent = cache.get(depth);

    match &action.kind {
        ActionKind::Raise { event } => {
            out.write_str(indent)?;
            out.write_str("<raise")?;
            attr(out, "event", event)?;
            out.write_str("/>\n")
        }
        ActionKind::Send {
            event,
            target,
            delay,
        } => {
            out.write_str(indent)?;
            out.write_str("<send")?;
            attr(out, "event", event)?;
            if let Some(t) = target {
                attr(out, "target", t)?;
            }
            if let Some(d) = delay {
                attr(out, "delay", d)?;
            }
            out.write_str("/>\n")
        }
        ActionKind::Assign { location, expr } => {
            out.write_str(indent)?;
            out.write_str("<assign")?;
            attr(out, "location", location)?;
            attr(out, "expr", expr)?;
            out.write_str("/>\n")
        }
        ActionKind::Log { label, expr } => {
            out.write_str(indent)?;
            out.write_str("<log")?;
            if let Some(l) = label {
                attr(out, "label", l)?;
            }
            if let Some(e) = expr {
                attr(out, "expr", e)?;
            }
            out.write_str("/>\n")
        }
        ActionKind::Cancel { sendid } => {
            out.write_str(indent)?;
            out.write_str("<cancel")?;
            attr(out, "sendid", sendid)?;
            out.write_str("/>\n")
        }
        ActionKind::If { branches, actions } => {
            let mut action_offset = 0;
            for (i, branch) in branches.iter().enumerate() {
                if i == 0 {
                    out.write_str(indent)?;
                    out.write_str("<if")?;
                    if let Some(guard) = &branch.guard {
                        attr(out, "cond", guard)?;
                    }
                    out.write_str(">\n")?;
                } else if branch.guard.is_some() {
                    out.write_str(indent)?;
                    out.write_str("  <elseif")?;
                    attr(out, "cond", branch.guard.as_ref().unwrap())?;
                    out.write_str("/>\n")?;
                } else {
                    out.write_str(indent)?;
                    out.write_str("  <else/>\n")?;
                }
                let end = (action_offset + branch.action_count).min(actions.len());
                for action in &actions[action_offset..end] {
                    emit_action_xml(out, action, depth + 1, cache)?;
                }
                action_offset = end;
            }
            out.write_str(indent)?;
            out.write_str("</if>\n")
        }
        ActionKind::Foreach {
            array,
            item,
            index,
            actions,
        } => {
            out.write_str(indent)?;
            out.write_str("<foreach")?;
            attr(out, "array", array)?;
            attr(out, "item", item)?;
            if let Some(idx) = index {
                attr(out, "index", idx)?;
            }
            if actions.is_empty() {
                out.write_str("/>\n")
            } else {
                out.write_str(">\n")?;
                for action in actions {
                    emit_action_xml(out, action, depth + 1, cache)?;
                }
                out.write_str(indent)?;
                out.write_str("</foreach>\n")
            }
        }
        ActionKind::Script { content } => {
            out.write_str(indent)?;
            out.write_str("<script>")?;
            out.write_str(&escape_xml_attr(content))?;
            out.write_str("</script>\n")
        }
        ActionKind::Invoke {
            invoke_type,
            src,
            id,
        } => {
            out.write_str(indent)?;
            out.write_str("<invoke")?;
            if let Some(t) = invoke_type {
                attr(out, "type", t)?;
            }
            if let Some(s) = src {
                attr(out, "src", s)?;
            }
            if let Some(i) = id {
                attr(out, "id", i)?;
            }
            out.write_str("/>\n")
        }
        ActionKind::Custom { name, .. } => {
            // Custom action names are used as XML element names.
            // Escape to prevent tag injection.
            let safe_name = escape_xml_attr(name);
            out.write_str(indent)?;
            out.write_char('<')?;
            out.write_str(&safe_name)?;
            out.write_str("/>\n")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{State, Transition};

    #[test]
    fn xml_output_is_well_formed() {
        let chart = Statechart::new(
            "draft",
            vec![
                {
                    let mut s = State::atomic("draft");
                    s.transitions.push(Transition::new("submit", "done"));
                    s
                },
                State::final_state("done"),
            ],
        );

        let xml = to_xml(&chart);
        assert!(xml.contains("<?xml"));
        assert!(xml.contains("<scxml"));
        assert!(xml.contains("initial=\"draft\""));
        assert!(xml.contains("<state id=\"draft\""));
        assert!(xml.contains("<transition event=\"submit\" target=\"done\""));
        assert!(xml.contains("<final id=\"done\""));
        assert!(xml.contains("</scxml>"));
    }

    #[test]
    fn write_xml_matches_to_xml() {
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

        let from_to = to_xml(&chart);
        let mut from_write = String::new();
        write_xml(&chart, &mut from_write).unwrap();
        assert_eq!(from_to, from_write);
    }
}
