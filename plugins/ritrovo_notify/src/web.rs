//! The small HTML and form helpers every page in this plugin uses.
//!
//! Local to this crate rather than shared with `ritrovo_forms`, which has its
//! own copy. The plugins compile to separate `cdylib`s for `wasm32-wasip1` and
//! there is no shared library between them; a workspace crate holding forty
//! lines would be one more member that `cargo build --target wasm32-wasip1`
//! would try to build as a module. `ritrovo_access` made the same call for the
//! same reason.
//!
//! **The kernel does not sanitize a plugin's response body.** That contract is
//! stated in `crates/kernel/src/routes/plugin_api.rs`, and it means an
//! unescaped interpolation here is a stored-XSS hole on a page that prints
//! conference titles imported from a third-party feed. Everything printed goes
//! through [`escape`].

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

/// One field out of an `application/x-www-form-urlencoded` body.
///
/// The last occurrence wins, which is what a browser sends when a form has two
/// inputs of one name and is the same rule the kernel's own extractor applies.
pub fn field(body: &str, name: &str) -> String {
    body.split('&')
        .filter(|pair| !pair.is_empty())
        .filter_map(|pair| {
            let (raw_key, raw_value) = pair.split_once('=').unwrap_or((pair, ""));
            (percent_decode(raw_key) == name).then(|| percent_decode(raw_value))
        })
        .next_back()
        .unwrap_or_default()
}

/// Percent-decode one form field, turning `+` into a space.
///
/// A malformed escape is left as written rather than dropped, so a value can
/// never be silently truncated into a different uuid.
fn percent_decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => match u8::from_str_radix(&raw[i + 1..i + 3], 16) {
                Ok(byte) => {
                    out.push(byte);
                    i += 3;
                }
                Err(_) => {
                    out.push(bytes[i]);
                    i += 1;
                }
            },
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Whether `raw` is a lowercase-or-uppercase hyphenated uuid.
///
/// Every id this plugin puts into SQL goes through here first. The statements
/// are parameterized, so this is not the injection fence; it is what keeps a
/// malformed id from reaching the database as a cast error the visitor sees as
/// a 500, and what makes "no such conference" a 404 rather than a stack trace.
pub fn is_uuid(raw: &str) -> bool {
    let bytes = raw.as_bytes();
    if bytes.len() != 36 {
        return false;
    }
    bytes.iter().enumerate().all(|(i, &b)| match i {
        8 | 13 | 18 | 23 => b == b'-',
        _ => b.is_ascii_hexdigit(),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn escaping_closes_every_way_out_of_a_text_node_or_attribute() {
        assert_eq!(
            escape(r#"<a href="x" onclick='y'>&"#),
            "&lt;a href=&quot;x&quot; onclick=&#39;y&#39;&gt;&amp;"
        );
    }

    #[test]
    fn a_field_is_decoded() {
        assert_eq!(field("item_id=a+b%2Fc&_token=t", "item_id"), "a b/c");
        assert_eq!(field("item_id=x&_token=t", "_token"), "t");
    }

    #[test]
    fn a_missing_field_is_empty_rather_than_an_error() {
        assert_eq!(field("a=1", "item_id"), "");
        assert_eq!(field("", "item_id"), "");
    }

    #[test]
    fn the_last_occurrence_of_a_repeated_field_wins() {
        // A hidden input plus a visible one of the same name is how a value gets
        // smuggled past a handler that reads the first.
        assert_eq!(field("item_id=first&item_id=second", "item_id"), "second");
    }

    #[test]
    fn a_malformed_escape_is_kept_rather_than_truncated() {
        assert_eq!(field("item_id=%zz", "item_id"), "%zz");
    }

    #[test]
    fn a_uuid_is_recognised_and_anything_else_is_not() {
        assert!(is_uuid("27bedde2-2356-451e-be0a-86e5baaed1a1"));
        assert!(is_uuid("27BEDDE2-2356-451E-BE0A-86E5BAAED1A1"));
        assert!(!is_uuid(""));
        assert!(!is_uuid("27bedde2235645 1ebe0a86e5baaed1a1"));
        assert!(!is_uuid("27bedde2-2356-451e-be0a-86e5baaed1a"));
        assert!(!is_uuid("'; DROP TABLE user_subscriptions; --"));
        assert!(!is_uuid("27bedde2-2356-451e-be0a-86e5baaed1az"));
    }
}
