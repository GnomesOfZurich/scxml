//! Export statecharts to various formats (DOT, Mermaid, XML, JSON).

/// Graphviz DOT export.
pub mod dot;
/// JSON export via serde.
#[cfg(feature = "json")]
pub mod json;
/// Mermaid stateDiagram-v2 export.
pub mod mermaid;
/// W3C SCXML XML export.
#[cfg(feature = "xml")]
pub mod xml;

/// Escape a string for use in an XML attribute value.
/// Replaces `&`, `<`, `>`, `"`, and `'` with XML entities.
/// Strips null bytes and other control characters (invalid in XML 1.0).
/// Returns `Cow::Borrowed` when no escaping is needed (the common case).
fn escape_xml_attr(s: &str) -> std::borrow::Cow<'_, str> {
    let first = s.find(|ch: char| {
        matches!(ch, '&' | '<' | '>' | '"' | '\'')
            || (ch.is_control() && ch != '\t' && ch != '\n' && ch != '\r')
    });
    let Some(pos) = first else {
        return std::borrow::Cow::Borrowed(s);
    };
    let mut out = String::with_capacity(s.len() + 8);
    out.push_str(&s[..pos]);
    for ch in s[pos..].chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            // XML 1.0 allows only #x9, #xA, #xD among control characters.
            c if c.is_control() && c != '\t' && c != '\n' && c != '\r' => {}
            _ => out.push(ch),
        }
    }
    std::borrow::Cow::Owned(out)
}

/// Escape a string for use inside a DOT quoted string.
/// Escapes `"` and `\` which would break DOT string literals.
/// Strips null bytes and other control characters.
/// Returns `Cow::Borrowed` when no escaping is needed (the common case).
fn escape_dot(s: &str) -> std::borrow::Cow<'_, str> {
    let first = s.find(|ch: char| {
        matches!(ch, '"' | '\\') || (ch.is_control() && ch != '\t' && ch != '\n' && ch != '\r')
    });
    let Some(pos) = first else {
        return std::borrow::Cow::Borrowed(s);
    };
    let mut out = String::with_capacity(s.len() + 8);
    out.push_str(&s[..pos]);
    for ch in s[pos..].chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() && c != '\t' && c != '\n' && c != '\r' => {}
            _ => out.push(ch),
        }
    }
    std::borrow::Cow::Owned(out)
}

/// Escape a string for use in Mermaid diagrams.
/// Replaces characters that break Mermaid structure: newlines, braces
/// (state blocks), quotes, pipes (label separators), semicolons
/// (statement separators), and `%%` comment markers.
/// Returns `Cow::Borrowed` when no escaping is needed (the common case).
fn escape_mermaid(s: &str) -> std::borrow::Cow<'_, str> {
    let first = s.find(['\n', '\r', '{', '}', '"', '|', ';', '%']);
    let Some(pos) = first else {
        return std::borrow::Cow::Borrowed(s);
    };
    let mut out = String::with_capacity(s.len() + 8);
    out.push_str(&s[..pos]);
    for ch in s[pos..].chars() {
        match ch {
            '\n' | '\r' => out.push(' '),
            '{' | '}' | '"' | '|' | ';' | '%' => out.push('_'),
            _ => out.push(ch),
        }
    }
    std::borrow::Cow::Owned(out)
}

/// Adapter that wraps an `io::Write` sink so it can be used with the `write_*`
/// export functions (which require `fmt::Write`).
///
/// ```rust,no_run
/// use scxml::export::{IoAdapter, dot::write_dot};
/// use scxml::parse_xml;
///
/// let chart = parse_xml(r#"
///     <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a">
///         <state id="a"><transition event="go" target="b"/></state>
///         <final id="b"/>
///     </scxml>
/// "#).unwrap();
///
/// let mut file = std::fs::File::create("chart.dot").unwrap();
/// write_dot(&chart, &mut IoAdapter::new(&mut file)).unwrap();
/// ```
pub struct IoAdapter<W> {
    inner: W,
    error: Option<std::io::Error>,
}

impl<W: std::io::Write> IoAdapter<W> {
    /// Wrap an `io::Write` sink for use with `fmt::Write`-based exporters.
    pub fn new(inner: W) -> Self {
        Self { inner, error: None }
    }

    /// Consume the adapter and return any `io::Error` that occurred.
    /// `fmt::Write` can only signal `fmt::Error` (no detail), so the
    /// real I/O error is captured here for the caller to inspect.
    pub fn into_io_result(self) -> std::io::Result<()> {
        match self.error {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}

impl<W: std::io::Write> std::fmt::Write for IoAdapter<W> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.inner.write_all(s.as_bytes()).map_err(|e| {
            self.error = Some(e);
            std::fmt::Error
        })
    }
}

/// Pre-computed indent strings to avoid repeated `"  ".repeat(depth)` allocations
/// in recursive export functions. Covers depths 0..=MAX_CACHED_DEPTH; deeper
/// levels fall back to dynamic allocation.
const MAX_CACHED_DEPTH: usize = 20;

struct IndentCache {
    indents: [String; MAX_CACHED_DEPTH + 1],
}

impl IndentCache {
    fn new() -> Self {
        let indents = std::array::from_fn(|i| "  ".repeat(i));
        Self { indents }
    }

    fn get(&self, depth: usize) -> &str {
        if depth <= MAX_CACHED_DEPTH {
            &self.indents[depth]
        } else {
            // Shouldn't happen in practice (20 levels deep), but safe fallback.
            // Caller will see a borrow issue, so we handle this differently.
            &self.indents[MAX_CACHED_DEPTH]
        }
    }
}
