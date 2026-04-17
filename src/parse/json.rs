use crate::error::{Result, ScxmlError};
use crate::model::Statechart;

/// Parse a JSON representation of a statechart into a [`Statechart`].
///
/// The JSON schema matches the serde serialization of [`Statechart`]:
/// a direct JSON mapping of the Rust model types.
pub fn parse_json(json: &str) -> Result<Statechart> {
    serde_json::from_str(json).map_err(|e| ScxmlError::Json(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_json() {
        let json = r#"{
            "initial": "draft",
            "states": [
                {
                    "id": "draft",
                    "kind": "atomic",
                    "transitions": [
                        { "event": "submit", "targets": ["review"], "actions": [] }
                    ],
                    "on_entry": [],
                    "on_exit": [],
                    "children": [],
                    "initial": null
                },
                {
                    "id": "review",
                    "kind": "atomic",
                    "transitions": [
                        { "event": "approve", "targets": ["approved"], "actions": [] }
                    ],
                    "on_entry": [],
                    "on_exit": [],
                    "children": [],
                    "initial": null
                },
                {
                    "id": "approved",
                    "kind": "final",
                    "transitions": [],
                    "on_entry": [],
                    "on_exit": [],
                    "children": [],
                    "initial": null
                }
            ]
        }"#;

        let chart = parse_json(json).unwrap();
        assert_eq!(chart.initial.as_str(), "draft");
        assert_eq!(chart.states.len(), 3);
    }
}
