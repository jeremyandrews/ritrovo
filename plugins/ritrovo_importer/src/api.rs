//! The two JSON API endpoints a gather cannot serve.
//!
//! Ritrovo's API is the kernel's wherever the kernel can serve it (`docs/API.md`):
//! the conference list is a URL alias onto the listing's own gather, and search
//! is the kernel's `/api/v1/search`. Two endpoints need code:
//!
//! - **`/api/v1/conferences/{id}`.** A gather is reached by a fixed address and
//!   takes its arguments as query parameters; an alias maps one fixed path to
//!   another. Neither can put an id in the path.
//! - **`/api/v1/topics`.** The kernel's category endpoints return the topic tags
//!   flat, with no parent on any of them (`/api/category/topics/tags`), or the
//!   roots alone (`/roots`). A client wanting the tree would make one request per
//!   node. This returns the tree in one.
//!
//! # What an anonymous caller sees
//!
//! Live and published conferences only, and only the fields on [`PUBLIC_FIELDS`].
//! The detail endpoint does not run the kernel's access taps (no host call
//! does), so it is written to need none: it selects nothing but Live, published
//! rows, and copies out an allowlist of fields rather than removing a denylist,
//! so a field added to the type later is private until it is listed here.
//! `field_editor_notes`, which `ritrovo_access` hides on the page, is not on it.

use trovato_sdk::host;
use trovato_sdk::prelude::*;
use trovato_sdk::types::{ApiRequest, ApiResponse};

/// The Live stage, as `demo/config/stage.*.yml` defines it.
pub const LIVE_STAGE_ID: &str = "0193a5a0-0000-7000-8000-000000000001";

/// The conference fields the API publishes. Everything else stays in.
pub const PUBLIC_FIELDS: &[&str] = &[
    "field_start_date",
    "field_end_date",
    "field_city",
    "field_country",
    "field_online",
    "field_language",
    "field_url",
    "field_cfp_url",
    "field_cfp_end_date",
    "field_description",
    "field_topics",
    "field_speakers",
];

/// One Live, published conference, its English alias, and its translations.
pub const CONFERENCE_SQL: &str = "\
SELECT i.id::text AS id, i.title, i.language, i.fields, i.created, i.changed, \
       (SELECT a.alias FROM url_alias a \
         WHERE a.source = '/item/' || i.id::text AND a.language = 'en' \
         ORDER BY a.created, a.alias LIMIT 1) AS alias, \
       COALESCE((SELECT json_agg(t.language ORDER BY t.language) FROM item_translation t \
         WHERE t.item_id = i.id), '[]'::json) AS translations \
FROM item i \
WHERE i.id = $1::uuid \
  AND i.type = 'conference' \
  AND i.status = 1 \
  AND i.stage_id = '0193a5a0-0000-7000-8000-000000000001'";

/// Every topic tag with its parent, one row per parent edge.
pub const TOPICS_SQL: &str = "\
SELECT t.id::text AS id, t.label, COALESCE(t.slug, '') AS slug, t.weight, \
       h.parent_id::text AS parent_id \
FROM category_tag t \
LEFT JOIN category_tag_hierarchy h ON h.tag_id = t.id \
WHERE t.category_id = 'topics' \
ORDER BY t.weight, t.label, t.id";

/// `GET /api/v1/conferences/{id}`.
pub fn conference(request: &ApiRequest) -> ApiResponse {
    let Some(id) = request
        .params
        .get("id")
        .and_then(|raw| Uuid::parse_str(raw).ok())
    else {
        return not_found();
    };
    let rows = match host::query_raw(CONFERENCE_SQL, &[serde_json::json!(id.to_string())]) {
        Ok(json) => json,
        Err(code) => return failed("conference", code),
    };
    match parse_rows(&rows).first().map(conference_json) {
        Some(body) => json_response(&body),
        None => not_found(),
    }
}

/// `GET /api/v1/topics`.
pub fn topics() -> ApiResponse {
    match host::query_raw(TOPICS_SQL, &[]) {
        Ok(json) => {
            let rows = parse_rows(&json);
            let tree = topic_tree(&rows);
            let total = count_tags(&rows);
            json_response(&serde_json::json!({ "data": tree, "total": total }))
        }
        Err(code) => failed("topics", code),
    }
}

/// The public shape of one conference row from [`CONFERENCE_SQL`].
///
/// Close to a row of the list endpoint, which is the kernel's: `id`, `title`,
/// `type`, `language`, `fields`, `url`, `created`, `changed`. Two keys the list
/// does not carry, `stage` (always `live` here) and `translations`.
pub fn conference_json(row: &serde_json::Value) -> serde_json::Value {
    let id = text(row, "id");
    let stored = json_value(row.get("fields"));
    let mut fields = serde_json::Map::new();
    for name in PUBLIC_FIELDS {
        if let Some(value) = stored.get(*name).filter(|v| !v.is_null()) {
            fields.insert((*name).to_string(), value.clone());
        }
    }
    let url = row
        .get("alias")
        .and_then(serde_json::Value::as_str)
        .filter(|a| !a.is_empty())
        .map_or_else(|| format!("/item/{id}"), str::to_string);
    serde_json::json!({
        "id": id,
        "type": "conference",
        "title": text(row, "title"),
        "language": text(row, "language"),
        "stage": "live",
        "fields": fields,
        "url": url,
        "translations": json_value(row.get("translations")),
        "created": row.get("created").cloned().unwrap_or(serde_json::Value::Null),
        "changed": row.get("changed").cloned().unwrap_or(serde_json::Value::Null),
    })
}

/// Nest the topic rows from [`TOPICS_SQL`] into a tree.
///
/// A tag with no parent is a root. A tag the hierarchy gives two parents
/// appears under each, which is what the category's `hierarchy: 2` (multiple
/// parents) allows. A cycle in the data cannot recurse forever: a tag is never
/// placed under itself, directly or further down.
pub fn topic_tree(rows: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let mut tags: Vec<(String, serde_json::Value)> = Vec::new();
    let mut children: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    let mut roots: Vec<String> = Vec::new();
    for row in rows {
        let id = text(row, "id");
        if id.is_empty() {
            continue;
        }
        if !tags.iter().any(|(known, _)| *known == id) {
            tags.push((
                id.clone(),
                serde_json::json!({
                    "id": id,
                    "label": text(row, "label"),
                    "slug": text(row, "slug"),
                    "weight": row.get("weight").cloned().unwrap_or(serde_json::json!(0)),
                }),
            ));
        }
        match row.get("parent_id").and_then(serde_json::Value::as_str) {
            Some(parent) if !parent.is_empty() => {
                let list = children.entry(parent.to_string()).or_default();
                if !list.contains(&id) {
                    list.push(id);
                }
            }
            _ => {
                if !roots.contains(&id) {
                    roots.push(id);
                }
            }
        }
    }
    let lookup: std::collections::HashMap<&str, &serde_json::Value> =
        tags.iter().map(|(id, tag)| (id.as_str(), tag)).collect();
    let mut path = Vec::new();
    roots
        .iter()
        .filter_map(|id| build_node(id, &lookup, &children, &mut path))
        .collect()
}

fn build_node(
    id: &str,
    lookup: &std::collections::HashMap<&str, &serde_json::Value>,
    children: &std::collections::HashMap<String, Vec<String>>,
    path: &mut Vec<String>,
) -> Option<serde_json::Value> {
    if path.iter().any(|seen| seen == id) {
        return None;
    }
    let mut node = (*lookup.get(id)?).clone();
    path.push(id.to_string());
    let kids: Vec<serde_json::Value> = children
        .get(id)
        .map(|ids| {
            ids.iter()
                .filter_map(|child| build_node(child, lookup, children, path))
                .collect()
        })
        .unwrap_or_default();
    path.pop();
    if let Some(obj) = node.as_object_mut() {
        if let Some(slug) = obj
            .get("slug")
            .and_then(serde_json::Value::as_str)
            .filter(|slug| !slug.is_empty())
        {
            let url = format!("/topics/{slug}");
            obj.insert("url".to_string(), serde_json::json!(url));
        }
        obj.insert("children".to_string(), serde_json::Value::Array(kids));
    }
    Some(node)
}

/// How many distinct tags the rows name.
fn count_tags(rows: &[serde_json::Value]) -> usize {
    let mut ids: Vec<String> = rows.iter().map(|r| text(r, "id")).collect();
    ids.sort();
    ids.dedup();
    ids.retain(|id| !id.is_empty());
    ids.len()
}

/// `query_raw`'s rows. A jsonb column may arrive as a value or as its text.
fn parse_rows(json: &str) -> Vec<serde_json::Value> {
    serde_json::from_str(json).unwrap_or_default()
}

fn json_value(value: Option<&serde_json::Value>) -> serde_json::Value {
    match value {
        Some(serde_json::Value::String(s)) => {
            serde_json::from_str(s).unwrap_or(serde_json::Value::Null)
        }
        Some(other) => other.clone(),
        None => serde_json::Value::Null,
    }
}

fn text(row: &serde_json::Value, key: &str) -> String {
    row.get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn json_response(body: &serde_json::Value) -> ApiResponse {
    ApiResponse::json(body)
        .unwrap_or_else(|_| ApiResponse::error(500, "could not encode the response"))
}

/// The same answer for a conference that does not exist, is not Live, or is
/// not a conference: the API does not say which.
fn not_found() -> ApiResponse {
    ApiResponse::error(404, "not found")
}

fn failed(what: &str, code: i32) -> ApiResponse {
    host::log(
        "error",
        crate::PLUGIN_NAME,
        &format!("API {what} query failed: host error {code}"),
    );
    ApiResponse::error(500, "the query failed; the site log has the error")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn tag(id: &str, label: &str, parent: Option<&str>) -> serde_json::Value {
        serde_json::json!({
            "id": id, "label": label, "slug": label.to_lowercase(), "weight": 0,
            "parent_id": parent,
        })
    }

    #[test]
    fn topics_nest_under_their_parents() {
        let rows = vec![
            tag("a", "Languages", None),
            tag("b", "Rust", Some("a")),
            tag("c", "Go", Some("a")),
            tag("d", "Infrastructure", None),
        ];
        let tree = topic_tree(&rows);
        assert_eq!(tree.len(), 2);
        assert_eq!(tree[0]["label"], "Languages");
        assert_eq!(tree[0]["children"].as_array().unwrap().len(), 2);
        assert_eq!(tree[0]["children"][0]["url"], "/topics/rust");
        assert_eq!(tree[1]["children"], serde_json::json!([]));
        assert_eq!(count_tags(&rows), 4);
    }

    #[test]
    fn a_tag_with_two_parents_appears_under_each() {
        let rows = vec![
            tag("a", "Web", None),
            tag("b", "Data", None),
            tag("c", "GraphQL", Some("a")),
            tag("c", "GraphQL", Some("b")),
        ];
        let tree = topic_tree(&rows);
        assert_eq!(tree[0]["children"][0]["id"], "c");
        assert_eq!(tree[1]["children"][0]["id"], "c");
        assert_eq!(count_tags(&rows), 3);
    }

    #[test]
    fn a_cycle_does_not_recurse_forever() {
        let rows = vec![
            tag("r", "Root", None),
            tag("a", "A", Some("r")),
            tag("b", "B", Some("a")),
            tag("a", "A", Some("b")),
        ];
        let tree = topic_tree(&rows);
        let a = &tree[0]["children"][0];
        assert_eq!(a["id"], "a");
        assert_eq!(a["children"][0]["id"], "b");
        assert_eq!(a["children"][0]["children"], serde_json::json!([]));
    }

    #[test]
    fn a_conference_publishes_only_its_public_fields() {
        let row = serde_json::json!({
            "id": "00000000-0000-0000-0000-000000000001",
            "title": "RustConf",
            "language": "en",
            "fields": {
                "field_city": "Montreal",
                "field_online": true,
                "field_topics": ["t1"],
                "field_editor_notes": "internal",
                "field_source_id": "rustconf-2027",
            },
            "alias": "/conferences/rustconf",
            "translations": ["it"],
            "created": 1, "changed": 2,
        });
        let out = conference_json(&row);
        assert_eq!(out["fields"]["field_city"], "Montreal");
        assert_eq!(out["fields"]["field_online"], true);
        assert_eq!(out["fields"]["field_topics"], serde_json::json!(["t1"]));
        assert!(out["fields"].get("field_editor_notes").is_none(), "{out}");
        assert!(out["fields"].get("field_source_id").is_none(), "{out}");
        assert_eq!(out["url"], "/conferences/rustconf");
        assert_eq!(out["translations"], serde_json::json!(["it"]));
        assert_eq!(out["stage"], "live");
    }

    #[test]
    fn fields_stored_as_text_are_read_as_json() {
        let row = serde_json::json!({
            "id": "x", "title": "T", "language": "en",
            "fields": "{\"field_city\":\"Roma\"}",
            "alias": null, "translations": "[]",
        });
        let out = conference_json(&row);
        assert_eq!(out["fields"]["field_city"], "Roma");
        assert_eq!(out["url"], "/item/x");
        assert_eq!(out["translations"], serde_json::json!([]));
    }

    #[test]
    fn the_detail_query_admits_live_published_conferences_only() {
        assert!(CONFERENCE_SQL.contains("i.type = 'conference'"));
        assert!(CONFERENCE_SQL.contains("i.status = 1"));
        assert!(CONFERENCE_SQL.contains(&format!("i.stage_id = '{LIVE_STAGE_ID}'")));
    }

    #[test]
    fn editor_notes_are_not_public() {
        assert!(!PUBLIC_FIELDS.contains(&"field_editor_notes"));
    }
}
