//! Parsers for statechart definition formats (SCXML XML, JSON)
//! and input sanitization for untrusted sources.

/// JSON parser (serde-based).
#[cfg(feature = "json")]
pub mod json;
/// Input sanitization and limits for untrusted SCXML.
#[cfg(feature = "xml")]
pub mod sanitize;
/// W3C SCXML XML parser (quick-xml-based).
#[cfg(feature = "xml")]
pub mod xml;
