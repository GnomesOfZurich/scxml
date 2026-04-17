use crate::error::{Result, ScxmlError};
use crate::model::Statechart;

/// Export a statechart as a JSON value.
pub fn to_json(chart: &Statechart) -> Result<serde_json::Value> {
    serde_json::to_value(chart).map_err(|e| ScxmlError::Json(e.to_string()))
}

/// Export a statechart as a pretty-printed JSON string.
pub fn to_json_string(chart: &Statechart) -> Result<String> {
    serde_json::to_string_pretty(chart).map_err(|e| ScxmlError::Json(e.to_string()))
}
