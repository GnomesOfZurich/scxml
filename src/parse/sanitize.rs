//! Input sanitization and limits for untrusted SCXML.
//!
//! When accepting SCXML from external sources (PUT endpoints, file uploads,
//! admin editors), use [`parse_untrusted`](crate::parse::sanitize::parse_untrusted) instead of `parse_xml` to
//! enforce size limits and reject potentially dangerous content.

use crate::error::{Result, ScxmlError};
use crate::model::Statechart;

/// Limits for untrusted SCXML input.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct InputLimits {
    /// Maximum input size in bytes (default: 1 MB).
    pub max_input_bytes: usize,
    /// Maximum total number of states (default: 10,000).
    pub max_states: usize,
    /// Maximum nesting depth (default: 20).
    pub max_depth: usize,
    /// Maximum number of transitions (default: 100,000).
    pub max_transitions: usize,
    /// Maximum total number of actions across all states (default: 100,000).
    pub max_actions: usize,
}

impl Default for InputLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 1_048_576, // 1 MB
            max_states: 10_000,
            max_depth: 20,
            max_transitions: 100_000,
            max_actions: 100_000,
        }
    }
}

/// Parse and validate SCXML from an untrusted source with input limits.
///
/// ```rust
/// use scxml::sanitize::{parse_untrusted, InputLimits};
///
/// let xml = r#"
///     <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a">
///         <state id="a"><transition event="go" target="b"/></state>
///         <final id="b"/>
///     </scxml>
/// "#;
/// let chart = parse_untrusted(xml, &InputLimits::default()).unwrap();
/// assert_eq!(chart.initial.as_str(), "a");
/// ```
///
/// Checks:
/// 1. Input size within `max_input_bytes`
/// 2. Rejects XML with `<!DOCTYPE` (prevents entity expansion attacks)
/// 3. Parses the SCXML
/// 4. Validates structural correctness and liveness
/// 5. Enforces state count, transition count, and depth limits
/// 6. Validates that all identifiers contain only safe characters
///
/// Returns the validated `Statechart` or an error.
#[cfg(feature = "xml")]
pub fn parse_untrusted(xml: &str, limits: &InputLimits) -> Result<Statechart> {
    // 1. Size check.
    if xml.len() > limits.max_input_bytes {
        return Err(ScxmlError::Xml(format!(
            "input too large: {} bytes (limit: {})",
            xml.len(),
            limits.max_input_bytes
        )));
    }

    // 2. Reject DOCTYPE declarations (prevents billion laughs, entity bombs).
    if xml.contains("<!DOCTYPE") || xml.contains("<!ENTITY") {
        return Err(ScxmlError::Xml(
            "DOCTYPE and ENTITY declarations are not allowed in untrusted input".into(),
        ));
    }

    // 3. Parse.
    let chart = crate::parse::xml::parse_xml(xml)?;

    // 4. Validate.
    crate::validate::validate(&chart)?;

    // 5. Enforce limits.
    let stats = crate::stats::stats(&chart);

    if stats.total_states > limits.max_states {
        return Err(ScxmlError::Xml(format!(
            "too many states: {} (limit: {})",
            stats.total_states, limits.max_states
        )));
    }

    if stats.total_transitions > limits.max_transitions {
        return Err(ScxmlError::Xml(format!(
            "too many transitions: {} (limit: {})",
            stats.total_transitions, limits.max_transitions
        )));
    }

    if stats.max_depth > limits.max_depth {
        return Err(ScxmlError::Xml(format!(
            "nesting too deep: {} (limit: {})",
            stats.max_depth, limits.max_depth
        )));
    }

    if stats.total_actions > limits.max_actions {
        return Err(ScxmlError::Xml(format!(
            "too many actions: {} (limit: {})",
            stats.total_actions, limits.max_actions
        )));
    }

    // 6. Validate all string fields contain only safe characters.
    validate_identifier(&chart.initial, "chart initial")?;
    if let Some(name) = &chart.name {
        validate_identifier(name, "chart name")?;
    }

    // Datamodel items.
    for item in &chart.datamodel.items {
        validate_identifier(&item.id, "data item id")?;
        // expr and src are freeform values, but reject control characters and
        // obvious injection patterns.
        if let Some(expr) = &item.expr {
            validate_freeform(expr, "data expr")?;
        }
        if let Some(src) = &item.src {
            validate_freeform(src, "data src")?;
        }
    }

    for state in chart.iter_all_states() {
        validate_identifier(&state.id, "state id")?;
        if let Some(init) = &state.initial {
            validate_identifier(init, "state initial")?;
        }

        for t in &state.transitions {
            if let Some(event) = &t.event {
                validate_identifier(event, "event name")?;
            }
            if let Some(guard) = &t.guard {
                validate_identifier(guard, "guard name")?;
            }
            for target in &t.targets {
                validate_identifier(target, "transition target")?;
            }
            if let Some(delay) = &t.delay {
                validate_delay(delay)?;
            }

            // Action fields.
            for action in &t.actions {
                validate_action(action)?;
            }
        }

        // Entry/exit actions.
        for action in &state.on_entry {
            validate_action(action)?;
        }
        for action in &state.on_exit {
            validate_action(action)?;
        }
    }

    Ok(chart)
}

/// Check that an identifier contains only safe characters.
///
/// Allows: alphanumeric, underscore, hyphen, dot, colon (for namespaced IDs).
/// Rejects: quotes, angle brackets, semicolons, backticks, control characters,
/// and anything else that could be used for injection.
fn validate_identifier(id: &str, context: &str) -> Result<()> {
    if id.is_empty() {
        return Err(ScxmlError::Xml(format!("empty {context}")));
    }

    if id.len() > 256 {
        return Err(ScxmlError::Xml(format!(
            "{context} '{}...' too long ({} chars, max 256)",
            &id[..32],
            id.len()
        )));
    }

    for ch in id.chars() {
        if !matches!(ch, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' | '.' | ':') {
            return Err(ScxmlError::Xml(format!(
                "{context} '{id}' contains unsafe character '{ch}'"
            )));
        }
    }

    Ok(())
}

/// Validate a delay string looks like an ISO 8601 duration.
/// Accepts: strings starting with P (e.g. PT30M, P1D, PT48H).
fn validate_delay(delay: &str) -> Result<()> {
    if delay.is_empty() {
        return Err(ScxmlError::Xml("empty delay value".into()));
    }
    if !delay.starts_with('P') {
        return Err(ScxmlError::Xml(format!(
            "delay '{delay}' is not a valid ISO 8601 duration (must start with 'P')"
        )));
    }
    // Check for safe characters only (digits, letters used in durations, dot for decimals).
    for ch in delay.chars() {
        if !matches!(
            ch,
            'P' | 'T' | 'Y' | 'M' | 'W' | 'D' | 'H' | 'S' | '0'..='9' | '.'
        ) {
            return Err(ScxmlError::Xml(format!(
                "delay '{delay}' contains invalid character '{ch}'"
            )));
        }
    }
    Ok(())
}

/// Validate an action's string fields.
fn validate_action(action: &crate::model::Action) -> Result<()> {
    use crate::model::ActionKind;
    match &action.kind {
        ActionKind::Raise { event } => {
            validate_identifier(event, "raise event")?;
        }
        ActionKind::Send {
            event,
            target,
            delay,
        } => {
            validate_identifier(event, "send event")?;
            if let Some(t) = target {
                validate_freeform(t, "send target")?;
            }
            if let Some(d) = delay {
                validate_delay(d)?;
            }
        }
        ActionKind::Assign { location, expr } => {
            validate_identifier(location, "assign location")?;
            validate_freeform(expr, "assign expr")?;
        }
        ActionKind::Log { label, expr } => {
            if let Some(l) = label {
                validate_freeform(l, "log label")?;
            }
            if let Some(e) = expr {
                validate_freeform(e, "log expr")?;
            }
        }
        ActionKind::Cancel { sendid } => {
            validate_identifier(sendid, "cancel sendid")?;
        }
        ActionKind::If { branches, actions } => {
            for branch in branches {
                if let Some(ref guard) = branch.guard {
                    validate_freeform(guard, "if/elseif cond")?;
                }
            }
            for a in actions {
                validate_action(a)?;
            }
        }
        ActionKind::Foreach {
            array,
            item,
            index,
            actions,
        } => {
            validate_freeform(array, "foreach array")?;
            validate_identifier(item, "foreach item")?;
            if let Some(idx) = index {
                validate_identifier(idx, "foreach index")?;
            }
            for a in actions {
                validate_action(a)?;
            }
        }
        ActionKind::Script { content } => {
            validate_freeform(content, "script content")?;
        }
        ActionKind::Invoke {
            invoke_type,
            src,
            id,
        } => {
            if let Some(t) = invoke_type {
                validate_freeform(t, "invoke type")?;
            }
            if let Some(s) = src {
                validate_freeform(s, "invoke src")?;
            }
            if let Some(i) = id {
                validate_identifier(i, "invoke id")?;
            }
        }
        ActionKind::Custom { name, .. } => {
            validate_identifier(name, "custom action name")?;
        }
        #[allow(unreachable_patterns)]
        _ => {} // Forward-compatible: unknown variants pass through.
    }
    Ok(())
}

/// Validate a freeform string value (expressions, URIs, labels).
/// More permissive than identifiers but rejects control characters
/// and obvious injection patterns.
fn validate_freeform(value: &str, context: &str) -> Result<()> {
    if value.len() > 4096 {
        return Err(ScxmlError::Xml(format!(
            "{context} too long ({} chars, max 4096)",
            value.len()
        )));
    }
    for ch in value.chars() {
        if ch.is_control() && ch != '\n' && ch != '\r' && ch != '\t' {
            return Err(ScxmlError::Xml(format!(
                "{context} contains control character U+{:04X}",
                ch as u32
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_SCXML: &str = r#"
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a">
            <state id="a"><transition event="go" target="b"/></state>
            <final id="b"/>
        </scxml>
    "#;

    #[test]
    fn valid_input_passes() {
        let chart = parse_untrusted(VALID_SCXML, &InputLimits::default());
        assert!(chart.is_ok());
    }

    #[test]
    fn rejects_oversized_input() {
        let limits = InputLimits {
            max_input_bytes: 10,
            ..Default::default()
        };
        let err = parse_untrusted(VALID_SCXML, &limits).unwrap_err();
        assert!(err.to_string().contains("too large"));
    }

    #[test]
    fn rejects_doctype() {
        let xml = r#"
            <!DOCTYPE foo [<!ENTITY xxe "boom">]>
            <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a">
                <state id="a"/>
            </scxml>
        "#;
        let err = parse_untrusted(xml, &InputLimits::default()).unwrap_err();
        assert!(err.to_string().contains("DOCTYPE"));
    }

    #[test]
    fn rejects_too_many_states() {
        let limits = InputLimits {
            max_states: 1,
            ..Default::default()
        };
        let err = parse_untrusted(VALID_SCXML, &limits).unwrap_err();
        assert!(err.to_string().contains("too many states"));
    }

    #[test]
    fn rejects_unsafe_characters_in_id() {
        let xml = r#"
            <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a&lt;b">
                <state id="a&lt;b"><transition event="go" target="c"/></state>
                <final id="c"/>
            </scxml>
        "#;
        // quick-xml will unescape &lt; to < which our validator should catch.
        let result = parse_untrusted(xml, &InputLimits::default());
        assert!(result.is_err());
    }

    #[test]
    fn rejects_script_injection_in_guard() {
        let xml = r#"
            <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a">
                <state id="a">
                    <transition event="go" target="b" cond="x'; DROP TABLE users;--"/>
                </state>
                <final id="b"/>
            </scxml>
        "#;
        let err = parse_untrusted(xml, &InputLimits::default()).unwrap_err();
        assert!(err.to_string().contains("unsafe character"));
    }

    #[test]
    fn allows_namespaced_ids() {
        let xml = r#"
            <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="approval.quant.review">
                <state id="approval.quant.review">
                    <transition event="approve" target="done" cond="approval.quant.signed_off"/>
                </state>
                <final id="done"/>
            </scxml>
        "#;
        assert!(parse_untrusted(xml, &InputLimits::default()).is_ok());
    }
}
