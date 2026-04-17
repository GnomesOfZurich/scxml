use compact_str::CompactString;
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

use crate::error::{Result, ScxmlError};
use crate::model::action;
use crate::model::*;

/// Maximum nesting depth for action elements (`<if>`, `<foreach>` containing actions).
/// Separate from `max_depth` which controls state tree depth.
const MAX_ACTION_DEPTH: usize = 32;

/// Parse a W3C SCXML XML document into a [`Statechart`].
///
/// Supports all W3C SCXML elements: `<scxml>`, `<state>`, `<parallel>`,
/// `<final>`, `<history>`, `<transition>`, `<onentry>`, `<onexit>`,
/// `<datamodel>`, `<data>`, `<raise>`, `<send>`, `<assign>`, `<log>`,
/// `<cancel>`, `<if>`/`<elseif>`/`<else>`, `<foreach>`, `<script>`, `<invoke>`.
///
/// Executable content elements are stored as action descriptors, never executed.
pub fn parse_xml(xml: &str) -> Result<Statechart> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"scxml" => {
                return parse_scxml_element(&mut reader, e);
            }
            Ok(Event::Eof) => {
                return Err(ScxmlError::Xml("no <scxml> root element found".into()));
            }
            Err(e) => return Err(ScxmlError::Xml(e.to_string())),
            _ => {} // skip XML declaration, comments, etc.
        }
        buf.clear();
    }
}

fn parse_scxml_element(reader: &mut Reader<&[u8]>, start: &BytesStart) -> Result<Statechart> {
    let initial_attr = attr_str(start, "initial")?;

    let name = attr_str(start, "name")?;
    let version = attr_str(start, "version")?.unwrap_or_else(|| CompactString::const_new("1.0"));
    let binding = match attr_str(start, "binding")?.as_deref() {
        Some("late") => Binding::Late,
        _ => Binding::Early,
    };

    let mut states = Vec::new();
    let mut datamodel = DataModel::default();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"state" => states.push(parse_state(reader, e, StateKind::Compound)?),
                b"parallel" => states.push(parse_state(reader, e, StateKind::Parallel)?),
                b"final" => states.push(parse_state(reader, e, StateKind::Final)?),
                b"datamodel" => datamodel = parse_datamodel(reader)?,
                other => {
                    skip_element(reader, other)?;
                }
            },
            Ok(Event::Empty(ref e)) => match e.name().as_ref() {
                b"state" => states.push(parse_empty_state(e)?),
                b"final" => states.push(parse_empty_final(e)?),
                _ => {}
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == b"scxml" => break,
            Ok(Event::Eof) => return Err(ScxmlError::Xml("unexpected EOF in <scxml>".into())),
            Err(e) => return Err(ScxmlError::Xml(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    Ok(Statechart {
        name,
        initial: initial_attr
            .or_else(|| states.first().map(|s| s.id.clone()))
            .ok_or(ScxmlError::MissingAttribute {
                element: "scxml",
                attribute: "initial",
            })?,
        states,
        datamodel,
        binding,
        version,
        xmlns: CompactString::const_new("http://www.w3.org/2005/07/scxml"),
        definition_version: None,
    })
}

fn parse_state(reader: &mut Reader<&[u8]>, start: &BytesStart, hint: StateKind) -> Result<State> {
    let id = attr_str(start, "id")?.ok_or(ScxmlError::MissingAttribute {
        element: "state",
        attribute: "id",
    })?;
    let initial_attr = attr_str(start, "initial")?;

    let mut transitions = Vec::new();
    let mut on_entry = Vec::new();
    let mut on_exit = Vec::new();
    let mut children = Vec::new();
    let mut buf = Vec::new();

    let tag_name = start.name().as_ref().to_vec();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                match e.name().as_ref() {
                    b"state" => children.push(parse_state(reader, e, StateKind::Compound)?),
                    b"parallel" => children.push(parse_state(reader, e, StateKind::Parallel)?),
                    b"final" => children.push(parse_state(reader, e, StateKind::Final)?),
                    b"history" => children.push(parse_history(reader, e)?),
                    b"transition" => transitions.push(parse_transition(reader, e)?),
                    b"onentry" => on_entry.extend(parse_action_block(reader, b"onentry", 0)?),
                    b"onexit" => on_exit.extend(parse_action_block(reader, b"onexit", 0)?),
                    b"initial" => {
                        // <initial> element contains a <transition> child
                        skip_element(reader, b"initial")?;
                    }
                    b"datamodel" => {
                        let _ = parse_datamodel(reader)?;
                    }
                    b"invoke" => {
                        let invoke_type = attr_str(e, "type")?;
                        let src = attr_str(e, "src")?;
                        let id = attr_str(e, "id")?;
                        skip_element(reader, b"invoke")?;
                        on_entry.push(Action {
                            kind: ActionKind::Invoke {
                                invoke_type,
                                src,
                                id,
                            },
                        });
                    }
                    other => {
                        skip_element(reader, other)?;
                    }
                }
            }
            Ok(Event::Empty(ref e)) => match e.name().as_ref() {
                b"state" => children.push(parse_empty_state(e)?),
                b"final" => children.push(parse_empty_final(e)?),
                b"transition" => transitions.push(parse_empty_transition(e)?),
                b"history" => children.push(parse_empty_history(e)?),
                b"invoke" => {
                    let invoke_type = attr_str(e, "type")?;
                    let src = attr_str(e, "src")?;
                    let id = attr_str(e, "id")?;
                    on_entry.push(Action {
                        kind: ActionKind::Invoke {
                            invoke_type,
                            src,
                            id,
                        },
                    });
                }
                _ => {}
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == tag_name => break,
            Ok(Event::Eof) => {
                return Err(ScxmlError::Xml(format!(
                    "unexpected EOF in <{}>",
                    String::from_utf8_lossy(&tag_name)
                )));
            }
            Err(e) => return Err(ScxmlError::Xml(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    // Determine the actual kind based on hint and children.
    let kind = match hint {
        StateKind::Final => StateKind::Final,
        StateKind::Parallel => StateKind::Parallel,
        _ => {
            if children.is_empty() {
                StateKind::Atomic
            } else {
                StateKind::Compound
            }
        }
    };

    Ok(State {
        id,
        kind,
        transitions,
        on_entry,
        on_exit,
        children,
        initial: initial_attr,
    })
}

fn parse_empty_state(e: &BytesStart) -> Result<State> {
    let id = attr_str(e, "id")?.ok_or(ScxmlError::MissingAttribute {
        element: "state",
        attribute: "id",
    })?;
    Ok(State::atomic(id))
}

fn parse_empty_final(e: &BytesStart) -> Result<State> {
    let id = attr_str(e, "id")?.ok_or(ScxmlError::MissingAttribute {
        element: "final",
        attribute: "id",
    })?;
    Ok(State::final_state(id))
}

fn parse_history(reader: &mut Reader<&[u8]>, start: &BytesStart) -> Result<State> {
    let id = attr_str(start, "id")?.ok_or(ScxmlError::MissingAttribute {
        element: "history",
        attribute: "id",
    })?;
    let kind = match attr_str(start, "type")?.as_deref() {
        Some("deep") => HistoryKind::Deep,
        _ => HistoryKind::Shallow,
    };

    // History may contain a <transition> for the default history target.
    let mut transitions = Vec::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"transition" => {
                transitions.push(parse_transition(reader, e)?);
            }
            Ok(Event::Empty(ref e)) if e.name().as_ref() == b"transition" => {
                transitions.push(parse_empty_transition(e)?);
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"history" => break,
            Ok(Event::Eof) => {
                return Err(ScxmlError::Xml("unexpected EOF in <history>".into()));
            }
            Err(e) => return Err(ScxmlError::Xml(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    let mut state = State::history(id, kind);
    state.transitions = transitions;
    Ok(state)
}

fn parse_empty_history(e: &BytesStart) -> Result<State> {
    let id = attr_str(e, "id")?.ok_or(ScxmlError::MissingAttribute {
        element: "history",
        attribute: "id",
    })?;
    let kind = match attr_str(e, "type")?.as_deref() {
        Some("deep") => HistoryKind::Deep,
        _ => HistoryKind::Shallow,
    };
    Ok(State::history(id, kind))
}

fn parse_transition(reader: &mut Reader<&[u8]>, start: &BytesStart) -> Result<Transition> {
    let mut t = build_transition_from_attrs(start)?;

    // Parse any action children inside <transition>...</transition>.
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                if let Some(action) = try_parse_action_element(reader, e, 0)? {
                    t.actions.push(action);
                }
            }
            Ok(Event::Empty(ref e)) => {
                if let Some(action) = try_parse_empty_action(e)? {
                    t.actions.push(action);
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"transition" => break,
            Ok(Event::Eof) => {
                return Err(ScxmlError::Xml("unexpected EOF in <transition>".into()));
            }
            Err(e) => return Err(ScxmlError::Xml(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    Ok(t)
}

fn parse_empty_transition(e: &BytesStart) -> Result<Transition> {
    build_transition_from_attrs(e)
}

fn build_transition_from_attrs(e: &BytesStart) -> Result<Transition> {
    let event = attr_str(e, "event")?;
    let guard = attr_str(e, "cond")?;
    let target_str = attr_str(e, "target")?;
    let delay = attr_str(e, "delay")?;
    let quorum = attr_str(e, "gnomes:quorum")?.and_then(|s| s.parse::<u32>().ok());
    let transition_type = match attr_str(e, "type")?.as_deref() {
        Some("internal") => TransitionType::Internal,
        _ => TransitionType::External,
    };

    let targets: Vec<CompactString> = target_str
        .map(|s| s.split_whitespace().map(CompactString::from).collect())
        .unwrap_or_default();

    Ok(Transition {
        event,
        guard,
        targets,
        transition_type,
        actions: Vec::new(),
        delay,
        quorum,
    })
}

fn parse_action_block(
    reader: &mut Reader<&[u8]>,
    end_tag: &[u8],
    action_depth: usize,
) -> Result<Vec<Action>> {
    let mut actions = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                if let Some(action) = try_parse_action_element(reader, e, action_depth)? {
                    actions.push(action);
                }
            }
            Ok(Event::Empty(ref e)) => {
                if let Some(action) = try_parse_empty_action(e)? {
                    actions.push(action);
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == end_tag => break,
            Ok(Event::Eof) => {
                return Err(ScxmlError::Xml(format!(
                    "unexpected EOF in <{}>",
                    String::from_utf8_lossy(end_tag)
                )));
            }
            Err(e) => return Err(ScxmlError::Xml(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    Ok(actions)
}

fn try_parse_action_element(
    reader: &mut Reader<&[u8]>,
    e: &BytesStart,
    action_depth: usize,
) -> Result<Option<Action>> {
    if action_depth > MAX_ACTION_DEPTH {
        return Err(ScxmlError::Xml(format!(
            "action nesting too deep (limit: {MAX_ACTION_DEPTH})"
        )));
    }
    let name = e.name();
    match name.as_ref() {
        b"raise" => {
            let event = attr_str(e, "event")?.ok_or(ScxmlError::MissingAttribute {
                element: "raise",
                attribute: "event",
            })?;
            skip_element(reader, b"raise")?;
            Ok(Some(Action::raise(event)))
        }
        b"send" => {
            let event = attr_str(e, "event")?.unwrap_or_default();
            let target = attr_str(e, "target")?;
            let delay = attr_str(e, "delay")?;
            skip_element(reader, b"send")?;
            Ok(Some(Action {
                kind: ActionKind::Send {
                    event,
                    target,
                    delay,
                },
            }))
        }
        b"assign" => {
            let location = attr_str(e, "location")?.ok_or(ScxmlError::MissingAttribute {
                element: "assign",
                attribute: "location",
            })?;
            let expr = attr_str(e, "expr")?.unwrap_or_default();
            skip_element(reader, b"assign")?;
            Ok(Some(Action::assign(location, expr)))
        }
        b"log" => {
            let label = attr_str(e, "label")?;
            let expr = attr_str(e, "expr")?;
            skip_element(reader, b"log")?;
            Ok(Some(Action::log(label, expr)))
        }
        b"cancel" => {
            let sendid = attr_str(e, "sendid")?.unwrap_or_default();
            skip_element(reader, b"cancel")?;
            Ok(Some(Action {
                kind: ActionKind::Cancel { sendid },
            }))
        }
        b"if" => {
            let (branches, actions) = parse_if_block(reader, e, action_depth + 1)?;
            Ok(Some(Action {
                kind: ActionKind::If { branches, actions },
            }))
        }
        b"foreach" => {
            let array = attr_str(e, "array")?.unwrap_or_default();
            let item = attr_str(e, "item")?.unwrap_or_default();
            let index = attr_str(e, "index")?;
            let actions = parse_action_block(reader, b"foreach", action_depth + 1)?;
            Ok(Some(Action {
                kind: ActionKind::Foreach {
                    array,
                    item,
                    index,
                    actions,
                },
            }))
        }
        b"script" => {
            let content = read_text_content(reader, b"script")?;
            Ok(Some(Action {
                kind: ActionKind::Script { content },
            }))
        }
        b"invoke" => {
            let invoke_type = attr_str(e, "type")?;
            let src = attr_str(e, "src")?;
            let id = attr_str(e, "id")?;
            skip_element(reader, b"invoke")?;
            Ok(Some(Action {
                kind: ActionKind::Invoke {
                    invoke_type,
                    src,
                    id,
                },
            }))
        }
        _ => {
            // Unknown element: treat as custom action.
            let action_name = String::from_utf8_lossy(name.as_ref());
            skip_element(reader, name.as_ref())?;
            Ok(Some(Action::custom(action_name.as_ref())))
        }
    }
}

fn try_parse_empty_action(e: &BytesStart) -> Result<Option<Action>> {
    let name = e.name();
    match name.as_ref() {
        b"raise" => {
            let event = attr_str(e, "event")?.ok_or(ScxmlError::MissingAttribute {
                element: "raise",
                attribute: "event",
            })?;
            Ok(Some(Action::raise(event)))
        }
        b"send" => {
            let event = attr_str(e, "event")?.unwrap_or_default();
            let target = attr_str(e, "target")?;
            let delay = attr_str(e, "delay")?;
            Ok(Some(Action {
                kind: ActionKind::Send {
                    event,
                    target,
                    delay,
                },
            }))
        }
        b"assign" => {
            let location = attr_str(e, "location")?.ok_or(ScxmlError::MissingAttribute {
                element: "assign",
                attribute: "location",
            })?;
            let expr = attr_str(e, "expr")?.unwrap_or_default();
            Ok(Some(Action::assign(location, expr)))
        }
        b"log" => {
            let label = attr_str(e, "label")?;
            let expr = attr_str(e, "expr")?;
            Ok(Some(Action::log(label, expr)))
        }
        b"cancel" => {
            let sendid = attr_str(e, "sendid")?.unwrap_or_default();
            Ok(Some(Action {
                kind: ActionKind::Cancel { sendid },
            }))
        }
        b"invoke" => {
            let invoke_type = attr_str(e, "type")?;
            let src = attr_str(e, "src")?;
            let id = attr_str(e, "id")?;
            Ok(Some(Action {
                kind: ActionKind::Invoke {
                    invoke_type,
                    src,
                    id,
                },
            }))
        }
        _ => Ok(None),
    }
}

/// Parse an `<if>` block with optional `<elseif>` and `<else>` branches.
fn parse_if_block(
    reader: &mut Reader<&[u8]>,
    start: &BytesStart,
    action_depth: usize,
) -> Result<(Vec<action::IfBranch>, Vec<Action>)> {
    let cond = attr_str(start, "cond")?;
    let mut branches = Vec::new();
    let mut all_actions = Vec::new();
    let mut current_actions = Vec::new();
    let mut current_guard = cond;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"elseif" => {
                    branches.push(action::IfBranch {
                        guard: current_guard.take(),
                        action_count: current_actions.len(),
                    });
                    all_actions.append(&mut current_actions);
                    current_guard = attr_str(e, "cond")?;
                    skip_element(reader, b"elseif")?;
                }
                b"else" => {
                    branches.push(action::IfBranch {
                        guard: current_guard.take(),
                        action_count: current_actions.len(),
                    });
                    all_actions.append(&mut current_actions);
                    current_guard = None;
                    skip_element(reader, b"else")?;
                }
                _ => {
                    if let Some(action) = try_parse_action_element(reader, e, action_depth)? {
                        current_actions.push(action);
                    }
                }
            },
            Ok(Event::Empty(ref e)) => match e.name().as_ref() {
                b"elseif" => {
                    branches.push(action::IfBranch {
                        guard: current_guard.take(),
                        action_count: current_actions.len(),
                    });
                    all_actions.append(&mut current_actions);
                    current_guard = attr_str(e, "cond")?;
                }
                b"else" => {
                    branches.push(action::IfBranch {
                        guard: current_guard.take(),
                        action_count: current_actions.len(),
                    });
                    all_actions.append(&mut current_actions);
                    current_guard = None;
                }
                _ => {
                    if let Some(action) = try_parse_empty_action(e)? {
                        current_actions.push(action);
                    }
                }
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == b"if" => {
                branches.push(action::IfBranch {
                    guard: current_guard.take(),
                    action_count: current_actions.len(),
                });
                all_actions.append(&mut current_actions);
                break;
            }
            Ok(Event::Eof) => return Err(ScxmlError::Xml("unexpected EOF in <if>".into())),
            Err(e) => return Err(ScxmlError::Xml(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    Ok((branches, all_actions))
}

/// Read text content until the closing tag, returning it as a CompactString.
fn read_text_content(reader: &mut Reader<&[u8]>, end_tag: &[u8]) -> Result<CompactString> {
    let mut content = String::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Text(ref t)) => {
                content.push_str(&String::from_utf8_lossy(t.as_ref()));
            }
            Ok(Event::CData(ref t)) => {
                content.push_str(&String::from_utf8_lossy(t.as_ref()));
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == end_tag => break,
            Ok(Event::Eof) => {
                return Err(ScxmlError::Xml(format!(
                    "unexpected EOF in <{}>",
                    String::from_utf8_lossy(end_tag)
                )));
            }
            Err(e) => return Err(ScxmlError::Xml(e.to_string())),
            _ => {}
        }
        buf.clear();
    }
    Ok(CompactString::from(content.trim()))
}

fn parse_datamodel(reader: &mut Reader<&[u8]>) -> Result<DataModel> {
    let mut items = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"data" => {
                let id = attr_str(e, "id")?.ok_or(ScxmlError::MissingAttribute {
                    element: "data",
                    attribute: "id",
                })?;
                let expr = attr_str(e, "expr")?;
                let src = attr_str(e, "src")?;
                // Skip to closing </data>
                skip_element(reader, b"data")?;
                items.push(DataItem { id, expr, src });
            }
            Ok(Event::Empty(ref e)) if e.name().as_ref() == b"data" => {
                let id = attr_str(e, "id")?.ok_or(ScxmlError::MissingAttribute {
                    element: "data",
                    attribute: "id",
                })?;
                let expr = attr_str(e, "expr")?;
                let src = attr_str(e, "src")?;
                items.push(DataItem { id, expr, src });
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"datamodel" => break,
            Ok(Event::Eof) => {
                return Err(ScxmlError::Xml("unexpected EOF in <datamodel>".into()));
            }
            Err(e) => return Err(ScxmlError::Xml(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    Ok(DataModel { items })
}

fn skip_element(reader: &mut Reader<&[u8]>, end_tag: &[u8]) -> Result<()> {
    let mut depth = 1u32;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == end_tag => depth += 1,
            Ok(Event::End(ref e)) if e.name().as_ref() == end_tag => {
                depth -= 1;
                if depth == 0 {
                    return Ok(());
                }
            }
            Ok(Event::Eof) => {
                return Err(ScxmlError::Xml(format!(
                    "unexpected EOF skipping <{}>",
                    String::from_utf8_lossy(end_tag)
                )));
            }
            Err(e) => return Err(ScxmlError::Xml(e.to_string())),
            _ => {}
        }
        buf.clear();
    }
}

/// Extract a UTF-8 attribute value from an element.
///
/// Uses raw byte access and avoids XML entity unescaping for the common case
/// where attribute values contain no `&` characters (identifiers, state IDs).
fn attr_str(e: &BytesStart, name: &str) -> Result<Option<CompactString>> {
    for attr in e.attributes().flatten() {
        if attr.key.as_ref() == name.as_bytes() {
            let raw = attr.value.as_ref();
            // Fast path: no entity references, just convert bytes to str.
            if !raw.contains(&b'&') {
                let s = std::str::from_utf8(raw).map_err(|e| ScxmlError::Xml(e.to_string()))?;
                return Ok(Some(CompactString::from(s)));
            }
            // Slow path: unescape XML entities.
            let value = attr
                .unescape_value()
                .map_err(|e| ScxmlError::Xml(e.to_string()))?;
            return Ok(Some(CompactString::from(value.as_ref())));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_scxml() {
        let xml = r#"
            <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="draft">
                <state id="draft">
                    <transition event="submit" target="review"/>
                </state>
                <state id="review">
                    <transition event="approve" target="approved"/>
                    <transition event="reject" target="draft"/>
                </state>
                <final id="approved"/>
            </scxml>
        "#;

        let chart = parse_xml(xml).unwrap();
        assert_eq!(chart.initial.as_str(), "draft");
        assert_eq!(chart.states.len(), 3);
        assert_eq!(chart.states[0].id.as_str(), "draft");
        assert_eq!(chart.states[0].transitions.len(), 1);
        assert_eq!(chart.states[1].transitions.len(), 2);
        assert_eq!(chart.states[2].kind, StateKind::Final);
    }

    #[test]
    fn parse_with_guards() {
        let xml = r#"
            <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="pending">
                <state id="pending">
                    <transition event="advance" target="approved" cond="quant_approved"/>
                    <transition event="advance" target="rejected" cond="quant_rejected"/>
                </state>
                <final id="approved"/>
                <final id="rejected"/>
            </scxml>
        "#;

        let chart = parse_xml(xml).unwrap();
        let pending = &chart.states[0];
        assert_eq!(
            pending.transitions[0].guard.as_deref(),
            Some("quant_approved")
        );
        assert_eq!(
            pending.transitions[1].guard.as_deref(),
            Some("quant_rejected")
        );
    }

    #[test]
    fn parse_parallel_state() {
        let xml = r#"
            <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="active">
                <parallel id="active">
                    <state id="region1" initial="r1_idle">
                        <state id="r1_idle"/>
                    </state>
                    <state id="region2" initial="r2_idle">
                        <state id="r2_idle"/>
                    </state>
                </parallel>
            </scxml>
        "#;

        let chart = parse_xml(xml).unwrap();
        assert_eq!(chart.states[0].kind, StateKind::Parallel);
        assert_eq!(chart.states[0].children.len(), 2);
    }

    #[test]
    fn parse_history_state() {
        let xml = r#"
            <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="main">
                <state id="main" initial="a">
                    <history id="hist" type="deep">
                        <transition target="a"/>
                    </history>
                    <state id="a">
                        <transition event="next" target="b"/>
                    </state>
                    <state id="b"/>
                </state>
            </scxml>
        "#;

        let chart = parse_xml(xml).unwrap();
        let main = &chart.states[0];
        let hist = &main.children[0];
        assert_eq!(hist.kind, StateKind::History(HistoryKind::Deep));
        assert_eq!(hist.transitions.len(), 1);
    }

    #[test]
    fn parse_actions() {
        let xml = r##"
            <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
                <state id="s1">
                    <onentry>
                        <raise event="entered"/>
                        <log label="info" expr="entering s1"/>
                    </onentry>
                    <transition event="go" target="s2">
                        <send event="notify" target="#_parent"/>
                    </transition>
                </state>
                <state id="s2"/>
            </scxml>
        "##;

        let chart = parse_xml(xml).unwrap();
        let s1 = &chart.states[0];
        assert_eq!(s1.on_entry.len(), 2);
        assert!(matches!(s1.on_entry[0].kind, ActionKind::Raise { .. }));
        assert!(matches!(s1.on_entry[1].kind, ActionKind::Log { .. }));
        assert_eq!(s1.transitions[0].actions.len(), 1);
        assert!(matches!(
            s1.transitions[0].actions[0].kind,
            ActionKind::Send { .. }
        ));
    }

    #[test]
    fn parse_script_as_descriptor() {
        let xml = r#"
            <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
                <state id="s1">
                    <onentry>
                        <script>console.log("hello")</script>
                    </onentry>
                </state>
            </scxml>
        "#;

        let chart = parse_xml(xml).unwrap();
        let s1 = &chart.states[0];
        assert_eq!(s1.on_entry.len(), 1);
        assert!(
            matches!(&s1.on_entry[0].kind, ActionKind::Script { content } if content == "console.log(\"hello\")")
        );
    }

    #[test]
    fn parse_if_block() {
        let xml = r#"
            <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
                <state id="s1">
                    <onentry>
                        <if cond="x > 0">
                            <log label="positive"/>
                        <elseif cond="x == 0"/>
                            <log label="zero"/>
                        <else/>
                            <log label="negative"/>
                        </if>
                    </onentry>
                </state>
            </scxml>
        "#;

        let chart = parse_xml(xml).unwrap();
        let s1 = &chart.states[0];
        assert_eq!(s1.on_entry.len(), 1);
        if let ActionKind::If { branches, actions } = &s1.on_entry[0].kind {
            assert_eq!(branches.len(), 3);
            assert_eq!(branches[0].guard.as_deref(), Some("x > 0"));
            assert_eq!(branches[1].guard.as_deref(), Some("x == 0"));
            assert!(branches[2].guard.is_none()); // <else>
            assert_eq!(actions.len(), 3);
        } else {
            panic!("expected If action");
        }
    }

    #[test]
    fn parse_foreach_block() {
        let xml = r#"
            <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
                <state id="s1">
                    <onentry>
                        <foreach array="items" item="x" index="i">
                            <log label="item"/>
                        </foreach>
                    </onentry>
                </state>
            </scxml>
        "#;

        let chart = parse_xml(xml).unwrap();
        let s1 = &chart.states[0];
        assert_eq!(s1.on_entry.len(), 1);
        if let ActionKind::Foreach {
            array,
            item,
            index,
            actions,
        } = &s1.on_entry[0].kind
        {
            assert_eq!(array.as_str(), "items");
            assert_eq!(item.as_str(), "x");
            assert_eq!(index.as_deref(), Some("i"));
            assert_eq!(actions.len(), 1);
        } else {
            panic!("expected Foreach action");
        }
    }

    #[test]
    fn parse_cancel_element() {
        let xml = r#"
            <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
                <state id="s1">
                    <onentry>
                        <cancel sendid="timer1"/>
                    </onentry>
                </state>
            </scxml>
        "#;

        let chart = parse_xml(xml).unwrap();
        let s1 = &chart.states[0];
        assert_eq!(s1.on_entry.len(), 1);
        assert!(
            matches!(&s1.on_entry[0].kind, ActionKind::Cancel { sendid } if sendid == "timer1")
        );
    }

    #[test]
    fn parse_invoke_element() {
        let xml = r#"
            <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
                <state id="s1">
                    <invoke type="scxml" src="child.scxml" id="child1"/>
                </state>
            </scxml>
        "#;

        let chart = parse_xml(xml).unwrap();
        let s1 = &chart.states[0];
        assert_eq!(s1.on_entry.len(), 1);
        if let ActionKind::Invoke {
            invoke_type,
            src,
            id,
        } = &s1.on_entry[0].kind
        {
            assert_eq!(invoke_type.as_deref(), Some("scxml"));
            assert_eq!(src.as_deref(), Some("child.scxml"));
            assert_eq!(id.as_deref(), Some("child1"));
        } else {
            panic!("expected Invoke action");
        }
    }

    #[test]
    fn parse_datamodel() {
        let xml = r#"
            <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
                <datamodel>
                    <data id="counter" expr="0"/>
                    <data id="name"/>
                </datamodel>
                <state id="s1"/>
            </scxml>
        "#;

        let chart = parse_xml(xml).unwrap();
        assert_eq!(chart.datamodel.items.len(), 2);
        assert_eq!(chart.datamodel.items[0].id.as_str(), "counter");
        assert_eq!(chart.datamodel.items[0].expr.as_deref(), Some("0"));
        assert!(chart.datamodel.items[1].expr.is_none());
    }
}
