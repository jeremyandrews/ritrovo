//! Reading a form body and writing HTML, for a plugin that has neither.
//!
//! A plugin serving a page gets [`ApiRequest::body`] as a string and returns a
//! body as a string. There is no extractor, no template engine and no escaping
//! helper on the other side of the WASM boundary, and the kernel explicitly does
//! **not** sanitize a plugin's response ("a plugin that emits HTML is
//! responsible for escaping it"). So both halves live here, in one module with
//! tests, rather than being written again at each call site — which is how one
//! of them eventually gets written without the escaping.
//!
//! [`ApiRequest::body`]: trovato_sdk::types::ApiRequest::body

/// The first value posted under `name`, percent-decoded.
///
/// First occurrence wins. A browser posts a checkbox once or not at all and a
/// text input once, so a repeat is either a crafted body or a mistake, and
/// taking the first is the choice that cannot be steered by appending.
pub fn field(body: &str, name: &str) -> String {
    pairs(body)
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
        .unwrap_or_default()
}

/// Whether `name` was posted at all, whatever its value.
///
/// An unchecked checkbox posts nothing, so presence is the answer rather than
/// the value: a form that read the value instead would keep the previous answer
/// every time somebody unticked a box.
pub fn present(body: &str, name: &str) -> bool {
    pairs(body).any(|(key, _)| key == name)
}

/// Decode a form-urlencoded body into key/value pairs.
///
/// Both halves are decoded: a key can be percent-encoded too, and a decoder that
/// skips the key matches `field_cfp_url` but not the identical `field%5Fcfp_url`.
fn pairs(body: &str) -> impl Iterator<Item = (String, String)> + '_ {
    body.split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| match pair.split_once('=') {
            Some((key, value)) => (decode(key), decode(value)),
            None => (decode(pair), String::new()),
        })
}

/// Percent-decode one form value, `+` meaning a space.
///
/// An invalid escape is left as written rather than dropped: a stray `%` in
/// something a person typed is a character, not an error, and silently eating it
/// would corrupt the value it was typed into.
fn decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(byte) => {
                        out.push(byte);
                        i += 3;
                    }
                    Err(_) => {
                        out.push(bytes[i]);
                        i += 1;
                    }
                }
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Escape text for HTML text content and for a double-quoted attribute.
///
/// All five, including the single quote: this is interpolated into
/// `value="..."` as well as into element content, and an attribute written with
/// single quotes anywhere in this plugin would otherwise be an injection point
/// the day someone adds one.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

/// Render plain text as paragraphs, escaped.
///
/// This is what Ritrovo has instead of rich text, and the reason is a kernel
/// gap rather than a preference: the kernel's `filtered_html` format runs a
/// stored string through ammonia on the way to the page, and **no host function
/// exposes that filter to a plugin**. A plugin that accepted HTML would have to
/// carry its own sanitizer, which is neither small nor Ritrovo's to own, so it
/// accepts text and renders it. See FRICTION.md, G-NO-TEXT-FORMAT-HOST-API.
///
/// A blank line starts a paragraph; a single newline is a line break. Nothing
/// else is markup, and everything is escaped first, so there is no input to this
/// function that produces a tag.
pub fn paragraphs(text: &str) -> String {
    let mut out = String::new();
    for block in text.split("\n\n") {
        let block = block.trim();
        if block.is_empty() {
            continue;
        }
        let lines: Vec<String> = block.lines().map(|line| escape(line.trim())).collect();
        out.push_str("<p>");
        out.push_str(&lines.join("<br>"));
        out.push_str("</p>");
    }
    out
}

/// Normalize a submitted line ending and trim, so stored text is `\n`-only.
///
/// A browser posts `\r\n` for a textarea newline. Storing that means the
/// round-trip through the form grows a `\r` per line per save, and every length
/// check counts characters the writer did not type.
pub fn normalize(text: &str) -> String {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .trim()
        .to_string()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn field_reads_a_plain_value() {
        assert_eq!(field("name=Ada&city=Torino", "city"), "Torino");
    }

    #[test]
    fn field_is_empty_when_absent() {
        assert_eq!(field("name=Ada", "city"), "");
    }

    #[test]
    fn field_decodes_plus_and_percent() {
        assert_eq!(field("t=Rust+Fest+2027", "t"), "Rust Fest 2027");
        assert_eq!(field("t=caf%C3%A8", "t"), "caf\u{e8}");
        assert_eq!(field("t=a%26b", "t"), "a&b");
    }

    #[test]
    fn field_decodes_the_key_too() {
        assert_eq!(field("a%5Fb=yes", "a_b"), "yes");
    }

    #[test]
    fn field_takes_the_first_of_a_repeat() {
        assert_eq!(field("a=1&a=2", "a"), "1");
    }

    #[test]
    fn a_stray_percent_survives() {
        assert_eq!(field("t=100%+sure", "t"), "100% sure");
        assert_eq!(field("t=%zz", "t"), "%zz");
        assert_eq!(field("t=%", "t"), "%");
    }

    #[test]
    fn a_valueless_key_is_empty_not_missing() {
        assert_eq!(field("a=&b=2", "a"), "");
        assert_eq!(field("a=&b=2", "b"), "2");
        assert!(present("a=&b=2", "a"));
        assert!(!present("a=&b=2", "c"));
    }

    #[test]
    fn an_unticked_checkbox_posts_nothing_at_all() {
        // The whole reason `present` exists: a browser omits an unchecked box,
        // so absence is the answer "no" rather than "unanswered".
        assert!(present("name=X&online=1", "online"));
        assert!(!present("name=X", "online"));
    }

    #[test]
    fn an_empty_body_yields_nothing() {
        assert_eq!(field("", "a"), "");
        assert!(!present("", "a"));
    }

    #[test]
    fn escape_covers_all_five() {
        assert_eq!(
            escape(r#"<a href="x">&'</a>"#),
            "&lt;a href=&quot;x&quot;&gt;&amp;&#39;&lt;/a&gt;"
        );
    }

    #[test]
    fn a_posted_script_tag_cannot_survive_escaping() {
        let posted = field("bio=%3Cscript%3Ealert(1)%3C%2Fscript%3E", "bio");
        assert_eq!(posted, "<script>alert(1)</script>");
        assert!(!escape(&posted).contains('<'));
        assert!(!paragraphs(&posted).contains("<script"));
    }

    #[test]
    fn paragraphs_split_on_a_blank_line() {
        assert_eq!(paragraphs("one\n\ntwo"), "<p>one</p><p>two</p>");
    }

    #[test]
    fn a_single_newline_is_a_break() {
        assert_eq!(paragraphs("one\ntwo"), "<p>one<br>two</p>");
    }

    #[test]
    fn paragraphs_of_nothing_is_nothing() {
        assert_eq!(paragraphs(""), "");
        assert_eq!(paragraphs("   \n\n  "), "");
    }

    #[test]
    fn normalize_removes_carriage_returns() {
        assert_eq!(normalize("a\r\nb\r\n\r\nc"), "a\nb\n\nc");
        assert_eq!(normalize("  padded  "), "padded");
    }

    // Regression guard for the round-trip that grows: a textarea posts \r\n, and
    // a value stored with it comes back, is posted again, and is stored again.
    #[test]
    fn normalizing_twice_changes_nothing() {
        let once = normalize("a\r\nb");
        assert_eq!(normalize(&once), once);
    }
}
