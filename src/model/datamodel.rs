use compact_str::CompactString;
use serde::{Deserialize, Serialize};

/// Data model declarations: variable definitions, not runtime values.
///
/// Maps to `<datamodel>` / `<data>` in W3C SCXML. We store declarations only;
/// no ECMAScript evaluation or runtime data binding.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[non_exhaustive]
pub struct DataModel {
    /// Declared data items.
    pub items: Vec<DataItem>,
}

/// A single data declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[non_exhaustive]
pub struct DataItem {
    /// Variable identifier (`<data id="...">`).
    pub id: CompactString,

    /// Optional initial value expression (stored as string, not evaluated).
    pub expr: Option<CompactString>,

    /// Optional source URI for external data.
    pub src: Option<CompactString>,
}

impl DataModel {
    /// Create an empty data model.
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Add a data item declaration.
    pub fn with_item(mut self, item: DataItem) -> Self {
        self.items.push(item);
        self
    }
}

impl DataItem {
    /// Create a data item with just an id.
    pub fn new(id: impl Into<CompactString>) -> Self {
        Self {
            id: id.into(),
            expr: None,
            src: None,
        }
    }

    /// Create a data item with an initial value expression.
    pub fn with_expr(id: impl Into<CompactString>, expr: impl Into<CompactString>) -> Self {
        Self {
            id: id.into(),
            expr: Some(expr.into()),
            src: None,
        }
    }
}
