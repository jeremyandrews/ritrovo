//! HTML escaping for the pages and fragments this plugin prints.
//!
//! The kernel does not sanitize a plugin's response body
//! (`crates/kernel/src/routes/plugin_api.rs`), and the queue prints conference
//! titles imported from a third-party feed, so every interpolated value goes
//! through [`escape`]. A local copy, as `ritrovo_notify` and `ritrovo_forms`
//! keep theirs: the plugins share no crate.

/// Escape text for an HTML text node or a double-quoted attribute.
pub fn escape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for c in raw.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::escape;

    #[test]
    fn escapes_markup_and_quotes() {
        assert_eq!(
            escape(r#"<a href="x">'&'</a>"#),
            "&lt;a href=&quot;x&quot;&gt;&#39;&amp;&#39;&lt;/a&gt;"
        );
    }
}
