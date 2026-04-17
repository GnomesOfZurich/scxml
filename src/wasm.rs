//! WebAssembly bindings for the scxml crate.
//!
//! Exposes core functionality to JavaScript via wasm-bindgen:
//! - `parse_xml`: parse SCXML XML to internal model
//! - `parse_json`: parse JSON to internal model
//! - `validate`: run structural + liveness checks
//! - `to_dot`: export DOT graph
//! - `to_mermaid`: export Mermaid stateDiagram-v2 text
//! - `to_json`: export JSON representation
//! - `flatten`: produce flat state/transition arrays for rendering
//!
//! Build with: `wasm-pack build --target web --features wasm`

use wasm_bindgen::prelude::*;

/// Parse and validate an SCXML XML string. Returns a JSON representation
/// of the parsed statechart, or throws on parse/validation error.
///
/// Uses `parse_untrusted()` with default limits to sanitize input:
/// rejects DOCTYPE/ENTITY declarations, validates identifiers, and enforces
/// size/depth limits. Use this for any browser-facing SCXML input.
#[wasm_bindgen(js_name = "parseXml")]
pub fn wasm_parse_xml(xml: &str) -> Result<String, JsValue> {
    let chart = crate::parse::sanitize::parse_untrusted(
        xml,
        &crate::parse::sanitize::InputLimits::default(),
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_json::to_string(&chart).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Parse a JSON statechart string. Returns the normalized JSON, or throws.
#[wasm_bindgen(js_name = "parseJson")]
pub fn wasm_parse_json(json: &str) -> Result<String, JsValue> {
    let chart =
        crate::parse::json::parse_json(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_json::to_string(&chart).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Validate a statechart (passed as JSON). Returns "ok" or throws with
/// the validation error message.
#[wasm_bindgen(js_name = "validate")]
pub fn wasm_validate(json: &str) -> Result<String, JsValue> {
    let chart =
        crate::parse::json::parse_json(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    crate::validate::validate(&chart).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok("ok".into())
}

/// Generate a DOT graph from a statechart (passed as JSON).
#[wasm_bindgen(js_name = "toDot")]
pub fn wasm_to_dot(json: &str) -> Result<String, JsValue> {
    let chart =
        crate::parse::json::parse_json(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(crate::export::dot::to_dot(&chart))
}

/// Generate Mermaid stateDiagram-v2 text from a statechart (passed as JSON).
#[wasm_bindgen(js_name = "toMermaid")]
pub fn wasm_to_mermaid(json: &str) -> Result<String, JsValue> {
    let chart =
        crate::parse::json::parse_json(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(crate::export::mermaid::to_mermaid(&chart))
}

/// Export a statechart (passed as JSON) back to SCXML XML.
#[wasm_bindgen(js_name = "toXml")]
pub fn wasm_to_xml(json: &str) -> Result<String, JsValue> {
    let chart =
        crate::parse::json::parse_json(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(crate::export::xml::to_xml(&chart))
}

/// Flatten a statechart (passed as JSON) into arrays of flat states and
/// transitions. Returns JSON: `{ "states": [...], "transitions": [...] }`.
#[wasm_bindgen(js_name = "flatten")]
pub fn wasm_flatten(json: &str) -> Result<String, JsValue> {
    let chart =
        crate::parse::json::parse_json(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let (states, transitions) = crate::flatten::flatten(&chart);

    let result = serde_json::json!({
        "states": states,
        "transitions": transitions,
    });
    serde_json::to_string(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Parse SCXML XML, validate, and return DOT. Single call for the common
/// "render this SCXML" use case. Throws if parse or validation fails.
///
/// Uses `parse_untrusted()` for input sanitization.
#[wasm_bindgen(js_name = "xmlToDot")]
pub fn wasm_xml_to_dot(xml: &str) -> Result<String, JsValue> {
    let chart = crate::parse::sanitize::parse_untrusted(
        xml,
        &crate::parse::sanitize::InputLimits::default(),
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))?;
    crate::validate::validate(&chart).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(crate::export::dot::to_dot(&chart))
}

/// Validate a statechart (passed as JSON) and return ALL errors as a JSON array.
/// Returns `"[]"` if valid. Unlike `validate`, this does not throw on the first
/// error; it collects every structural, liveness, and semantic issue.
#[wasm_bindgen(js_name = "validateAll")]
pub fn wasm_validate_all(json: &str) -> Result<String, JsValue> {
    let chart =
        crate::parse::json::parse_json(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let errors = crate::validate::validate_all(&chart);
    let error_strings: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
    serde_json::to_string(&error_strings).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Parse an XState v5 machine JSON string into a statechart. Returns a JSON
/// representation of the parsed statechart, or throws on error.
#[cfg(feature = "xstate")]
#[wasm_bindgen(js_name = "parseXstate")]
pub fn wasm_parse_xstate(json: &str) -> Result<String, JsValue> {
    let chart =
        crate::xstate::import::parse_xstate(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_json::to_string(&chart).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Export a statechart (passed as JSON) to XState v5 machine JSON.
#[cfg(feature = "xstate")]
#[wasm_bindgen(js_name = "toXstate")]
pub fn wasm_to_xstate(json: &str) -> Result<String, JsValue> {
    let chart =
        crate::parse::json::parse_json(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    crate::xstate::export::to_xstate(&chart).map_err(|e| JsValue::from_str(&e.to_string()))
}

// ── Simulation ──────────────────────────────────────────────────────────────

/// Simulate a single step: given a statechart JSON + current state + event,
/// return the next state. Stateless API; no persistent simulator object.
///
/// Returns JSON: `{ "state": "<new_state>", "ok": true }` on success,
/// or `{ "state": "<unchanged>", "ok": false, "error": "..." }` on failure.
#[wasm_bindgen(js_name = "simulateStep")]
pub fn wasm_simulate_step(json: &str, current_state: &str, event: &str) -> Result<String, JsValue> {
    let chart =
        crate::parse::json::parse_json(json).map_err(|e| JsValue::from_str(&e.to_string()))?;

    // Simulator always starts at the chart's initial state, so clone the chart
    // and override initial to the caller's current state before constructing it.
    let mut modified_chart = chart;
    modified_chart.initial = current_state.into();
    let mut sim = crate::simulate::Simulator::new(&modified_chart);

    match sim.send(event) {
        Ok(new_state) => {
            let result = serde_json::json!({
                "state": new_state,
                "ok": true,
            });
            serde_json::to_string(&result).map_err(|e| JsValue::from_str(&e.to_string()))
        }
        Err(e) => {
            let result = serde_json::json!({
                "state": current_state,
                "ok": false,
                "error": e.to_string(),
            });
            serde_json::to_string(&result).map_err(|e| JsValue::from_str(&e.to_string()))
        }
    }
}

/// Get available events from a given state. Returns JSON array of event names.
#[wasm_bindgen(js_name = "availableEvents")]
pub fn wasm_available_events(json: &str, current_state: &str) -> Result<String, JsValue> {
    let chart =
        crate::parse::json::parse_json(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let index = crate::index::StateIndex::new(&chart);

    let mut events = Vec::new();
    // Collect events from the current state and ancestors
    let mut state_id: Option<&str> = Some(current_state);
    while let Some(sid) = state_id {
        if let Some(state) = index.state_map().get(sid) {
            for t in &state.transitions {
                if let Some(event) = &t.event {
                    let ev = event.to_string();
                    if !events.contains(&ev) {
                        events.push(ev);
                    }
                }
            }
        }
        state_id = index.parent_map().get(sid).copied();
    }

    serde_json::to_string(&events).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Diff two statecharts (both passed as JSON). Returns a JSON array of differences:
/// `[{ "path": "...", "kind": "Changed|Added|Removed", "old"?: "...", "new"?: "...", "value"?: "..." }]`.
#[wasm_bindgen(js_name = "diff")]
pub fn wasm_diff(json_a: &str, json_b: &str) -> Result<String, JsValue> {
    let chart_a =
        crate::parse::json::parse_json(json_a).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let chart_b =
        crate::parse::json::parse_json(json_b).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let diffs = crate::diff::diff(&chart_a, &chart_b);
    let result: Vec<serde_json::Value> = diffs
        .iter()
        .map(|d| {
            let mut obj = serde_json::json!({ "path": d.path });
            match &d.kind {
                crate::diff::DiffKind::Changed { old, new } => {
                    obj["kind"] = "Changed".into();
                    obj["old"] = old.clone().into();
                    obj["new"] = new.clone().into();
                }
                crate::diff::DiffKind::Added { value } => {
                    obj["kind"] = "Added".into();
                    obj["value"] = value.clone().into();
                }
                crate::diff::DiffKind::Removed { value } => {
                    obj["kind"] = "Removed".into();
                    obj["value"] = value.clone().into();
                }
            }
            obj
        })
        .collect();
    serde_json::to_string(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Diff two statecharts provided as XML. Parses both, diffs, and returns JSON array.
#[wasm_bindgen(js_name = "xmlDiff")]
pub fn wasm_xml_diff(xml_a: &str, xml_b: &str) -> Result<String, JsValue> {
    let limits = crate::parse::sanitize::InputLimits::default();
    let chart_a = crate::parse::sanitize::parse_untrusted(xml_a, &limits)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let chart_b = crate::parse::sanitize::parse_untrusted(xml_b, &limits)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let diffs = crate::diff::diff(&chart_a, &chart_b);
    let result: Vec<serde_json::Value> = diffs
        .iter()
        .map(|d| {
            let mut obj = serde_json::json!({ "path": d.path });
            match &d.kind {
                crate::diff::DiffKind::Changed { old, new } => {
                    obj["kind"] = "Changed".into();
                    obj["old"] = old.clone().into();
                    obj["new"] = new.clone().into();
                }
                crate::diff::DiffKind::Added { value } => {
                    obj["kind"] = "Added".into();
                    obj["value"] = value.clone().into();
                }
                crate::diff::DiffKind::Removed { value } => {
                    obj["kind"] = "Removed".into();
                    obj["value"] = value.clone().into();
                }
            }
            obj
        })
        .collect();
    serde_json::to_string(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

// ── Tests ───────────────────────────────────────────────────────────────────
//
// The wasm-bindgen functions use JsValue which panics on non-wasm targets.
// We test the same code paths via the underlying crate functions directly.

#[cfg(test)]
mod tests {
    const SIMPLE_XML: &str = r#"
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a">
            <state id="a"><transition event="go" target="b"/></state>
            <final id="b"/>
        </scxml>
    "#;

    #[test]
    fn parse_xml_to_json() {
        let chart = crate::parse::xml::parse_xml(SIMPLE_XML).unwrap();
        let json = serde_json::to_string(&chart).unwrap();
        assert!(json.contains("\"initial\":\"a\""));
        assert!(json.contains("\"id\":\"a\""));
    }

    #[test]
    fn parse_xml_error_on_invalid() {
        let result = crate::parse::xml::parse_xml("<not-scxml/>");
        assert!(result.is_err());
    }

    #[test]
    fn json_roundtrip() {
        let chart = crate::parse::xml::parse_xml(SIMPLE_XML).unwrap();
        let json = serde_json::to_string(&chart).unwrap();
        let chart2 = crate::parse::json::parse_json(&json).unwrap();
        let json2 = serde_json::to_string(&chart2).unwrap();
        assert!(json2.contains("\"initial\":\"a\""));
    }

    #[test]
    fn validate_valid_chart() {
        let chart = crate::parse::xml::parse_xml(SIMPLE_XML).unwrap();
        crate::validate::validate(&chart).unwrap();
    }

    #[test]
    fn validate_invalid_chart() {
        let bad_json = r#"{"initial":"missing","states":[{"id":"a","kind":"atomic","transitions":[],"on_entry":[],"on_exit":[],"children":[],"initial":null}]}"#;
        let chart = crate::parse::json::parse_json(bad_json).unwrap();
        assert!(crate::validate::validate(&chart).is_err());
    }

    #[test]
    fn to_dot_produces_graph() {
        let chart = crate::parse::xml::parse_xml(SIMPLE_XML).unwrap();
        let dot = crate::export::dot::to_dot(&chart);
        assert!(dot.contains("digraph statechart"));
        assert!(dot.contains("\"a\""));
    }

    #[test]
    fn to_xml_roundtrips() {
        let chart = crate::parse::xml::parse_xml(SIMPLE_XML).unwrap();
        let xml = crate::export::xml::to_xml(&chart);
        assert!(xml.contains("<scxml"));
        assert!(xml.contains("initial=\"a\""));
        let chart2 = crate::parse::xml::parse_xml(&xml).unwrap();
        assert_eq!(chart2.initial.as_str(), "a");
    }

    #[test]
    fn flatten_returns_states_and_transitions() {
        let chart = crate::parse::xml::parse_xml(SIMPLE_XML).unwrap();
        let (states, transitions) = crate::flatten::flatten(&chart);
        assert_eq!(states.len(), 2);
        assert_eq!(transitions.len(), 1);
    }

    #[test]
    fn xml_to_dot_pipeline() {
        let chart = crate::parse::xml::parse_xml(SIMPLE_XML).unwrap();
        crate::validate::validate(&chart).unwrap();
        let dot = crate::export::dot::to_dot(&chart);
        assert!(dot.contains("digraph statechart"));
        assert!(dot.contains("\"a\""));
        assert!(dot.contains("\"b\""));
    }

    #[test]
    fn xml_to_dot_error_on_invalid() {
        let result = crate::parse::xml::parse_xml("<invalid/>");
        assert!(result.is_err());
    }
}
