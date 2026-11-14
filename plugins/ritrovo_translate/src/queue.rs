//! The translation queue: `/admin/content/translations`.
//!
//! A list, not an editor. Each row is an upcoming conference with no translation
//! in the site's other language, linked to the kernel's own translate screen,
//! `/admin/content/{id}/translate/{lang}`, which is where the translation is
//! written: through `ItemService::save_translation` into `item_translation`,
//! with the kernel's permission check and CSRF token. This plugin keeps no copy
//! of a translation. What it keeps is a model's unreviewed draft, when the
//! assisted path is on ([`crate::assist`]), shown here beside the link for the
//! editor to read, correct and save through that screen; no host call lets a
//! plugin save it for them (`G-TRANSLATION-NO-WRITE-PATH`, narrowed).
//!
//! # Which conferences
//!
//! Upcoming ones, on any stage: an editor translates a conference before it is
//! published as readily as after, and a conference that has already happened
//! is not worth an editor's time. The target is [`crate::target_language`] of
//! the item's own `language` column, which is what the kernel's screen compares
//! against when it refuses a translation into an item's own language; the SQL
//! repeats that rule as a `CASE` because it has to filter on it.

use trovato_sdk::host;
use trovato_sdk::prelude::*;

use crate::web::escape;

/// Where the queue is served.
///
/// The brief's own address. The kernel reserves no `/admin` prefix for itself
/// and has no route at this path (its content routes are
/// `/admin/content/{id}/…`, `/add`, `/bulk`, `/files` and `/comments`), so a
/// plugin route may live here.
pub const PATH: &str = "/admin/content/translations";

/// The permission the kernel's translate screen checks, declared by the
/// kernel's `trovato_content_translation` plugin.
pub const PERMISSION: &str = "translate content";

/// Rows per page.
pub const PAGE_SIZE: i64 = 50;

/// Stage uuids, as `demo/config/stage.*.yml` defines them, for the stage column.
const STAGES: &[(&str, &str)] = &[
    ("0193a5a0-0000-7000-8000-000000000001", "Live"),
    ("0193a5a0-0000-7000-8000-000000000002", "Incoming"),
    ("0193a5a0-0000-7000-8000-000000000003", "Curated"),
];

/// The SQL that finds the queue's rows.
///
/// `$1` is the offset. `count(*) OVER ()` carries the total on every row so one
/// round trip draws both the page and the pager.
pub const QUEUE_SQL: &str = "\
SELECT i.id::text AS id, i.title, i.language, \
       COALESCE(i.fields->>'field_start_date', '') AS start_date, \
       i.stage_id::text AS stage_id, \
       d.title AS draft_title, \
       d.fields->>'field_description' AS draft_description, \
       d.model AS draft_model, \
       count(*) OVER () AS total \
FROM item i \
LEFT JOIN ritrovo_translation_draft d \
  ON d.item_id = i.id \
 AND d.language = CASE WHEN i.language = 'it' THEN 'en' ELSE 'it' END \
WHERE i.type = 'conference' \
  AND COALESCE(i.fields->>'field_start_date', '') >= to_char(now(), 'YYYY-MM-DD') \
  AND NOT EXISTS ( \
    SELECT 1 FROM item_translation t \
    WHERE t.item_id = i.id \
      AND t.language = CASE WHEN i.language = 'it' THEN 'en' ELSE 'it' END) \
ORDER BY i.fields->>'field_start_date', i.title, i.id \
LIMIT 50 OFFSET $1";

/// One conference waiting for a translation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub id: String,
    pub title: String,
    pub language: String,
    pub start_date: String,
    pub stage_id: String,
    pub draft: Option<Draft>,
}

/// A model's draft, as stored, for the editor to review.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draft {
    pub title: String,
    pub description: String,
    pub model: String,
}

/// Read one page of the queue: the rows and the total across all pages.
pub fn load(page: i64) -> Result<(Vec<Row>, i64), i32> {
    let offset = (page.max(1) - 1) * PAGE_SIZE;
    let json = host::query_raw(QUEUE_SQL, &[serde_json::json!(offset)])?;
    Ok(parse(&json))
}

/// Turn `query_raw`'s JSON rows into [`Row`]s and the total.
pub fn parse(json: &str) -> (Vec<Row>, i64) {
    let rows: Vec<serde_json::Value> = serde_json::from_str(json).unwrap_or_default();
    let text = |row: &serde_json::Value, key: &str| -> String {
        row.get(key)
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    let total = rows
        .first()
        .and_then(|r| r.get("total"))
        .and_then(|t| t.as_i64().or_else(|| t.as_str()?.parse().ok()))
        .unwrap_or(0);
    let parsed = rows
        .iter()
        .map(|r| Row {
            id: text(r, "id"),
            title: text(r, "title"),
            language: text(r, "language"),
            start_date: text(r, "start_date"),
            stage_id: text(r, "stage_id"),
            draft: r
                .get("draft_title")
                .and_then(serde_json::Value::as_str)
                .map(|title| Draft {
                    title: title.to_string(),
                    description: text(r, "draft_description"),
                    model: text(r, "draft_model"),
                }),
        })
        .collect();
    (parsed, total)
}

/// Serve the queue.
pub fn page(request: &ApiRequest) -> ApiResponse {
    let page = request
        .query
        .get("page")
        .and_then(|p| p.parse::<i64>().ok())
        .unwrap_or(1)
        .max(1);
    match load(page) {
        Ok((rows, total)) => ApiResponse::themed("Translations", render(&rows, total, page)),
        Err(code) => {
            host::log(
                "error",
                "ritrovo_translate",
                &format!("translation queue query failed: host error {code}"),
            );
            ApiResponse::themed_with_status(
                500,
                "Translations",
                "<p>The translation queue could not be read. The site log has the error.</p>",
            )
        }
    }
}

/// The queue's HTML. Pure, so the tests can read it without a database.
pub fn render(rows: &[Row], total: i64, page: i64) -> String {
    let mut html = String::new();
    html.push_str(
        "<section class=\"translation-queue\">\
         <p>Upcoming conferences with no translation in the site's other language. \
         Italian conferences are translated into English, every other one into Italian. \
         <strong>Translate</strong> opens the translate screen, which saves the translation.</p>",
    );
    html.push_str(&format!(
        "<p class=\"translation-queue__count\" data-total=\"{total}\">{total} waiting</p>"
    ));
    if rows.is_empty() {
        html.push_str("<p class=\"translation-queue__empty\">Nothing is waiting for a translation.</p></section>");
        return html;
    }
    html.push_str(
        "<table class=\"table translation-queue__table\"><thead><tr>\
         <th>Conference</th><th>Starts</th><th>Stage</th><th>Language</th>\
         <th>Translate into</th><th>Draft</th></tr></thead><tbody>",
    );
    for row in rows {
        let target = crate::target_language(&row.language);
        let stage = STAGES
            .iter()
            .find(|(uuid, _)| *uuid == row.stage_id)
            .map_or("Other", |(_, name)| name);
        let draft = row.draft.as_ref().map_or_else(
            || "<span class=\"translation-queue__no-draft\">none</span>".to_string(),
            |d| {
                format!(
                    "<details class=\"translation-queue__draft\"><summary lang=\"{target}\">{title}</summary>\
                     <div lang=\"{target}\">{description}</div>\
                     <p class=\"translation-queue__draft-model\">Drafted by {model}; not reviewed.</p></details>",
                    title = escape(&d.title),
                    description = escape(&d.description),
                    model = escape(&d.model),
                )
            },
        );
        html.push_str(&format!(
            "<tr data-item=\"{id}\"><td><a href=\"/item/{id}\">{title}</a></td><td>{start}</td>\
             <td>{stage}</td><td><code>{lang}</code></td>\
             <td><a class=\"translation-queue__translate\" \
                 href=\"/admin/content/{id}/translate/{target}\">Translate into <code>{target}</code></a></td>\
             <td>{draft}</td></tr>",
            id = escape(&row.id),
            title = escape(&row.title),
            start = escape(&row.start_date),
            lang = escape(&row.language),
        ));
    }
    html.push_str("</tbody></table>");
    let pages = (total + PAGE_SIZE - 1) / PAGE_SIZE;
    if pages > 1 {
        html.push_str("<nav class=\"pager\">");
        if page > 1 {
            html.push_str(&format!(
                "<a href=\"{PATH}?page={}\">Previous</a> ",
                page - 1
            ));
        }
        html.push_str(&format!("Page {page} of {pages}"));
        if page < pages {
            html.push_str(&format!(" <a href=\"{PATH}?page={}\">Next</a>", page + 1));
        }
        html.push_str("</nav>");
    }
    html.push_str("</section>");
    html
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn row(id: &str, language: &str, draft: Option<Draft>) -> Row {
        Row {
            id: id.to_string(),
            title: "Festival <der> Zukunft".to_string(),
            language: language.to_string(),
            start_date: "2027-03-01".to_string(),
            stage_id: "0193a5a0-0000-7000-8000-000000000002".to_string(),
            draft,
        }
    }

    #[test]
    fn each_row_links_the_kernel_translate_screen_in_the_target_language() {
        let html = render(&[row("a", "de", None), row("b", "it", None)], 2, 1);
        assert!(
            html.contains("href=\"/admin/content/a/translate/it\""),
            "{html}"
        );
        assert!(
            html.contains("href=\"/admin/content/b/translate/en\""),
            "{html}"
        );
        assert!(html.contains("Incoming"), "{html}");
    }

    #[test]
    fn titles_and_drafts_are_escaped() {
        let draft = Draft {
            title: "Festival <script>".to_string(),
            description: "\"quoted\"".to_string(),
            model: "mock".to_string(),
        };
        let html = render(&[row("a", "de", Some(draft))], 1, 1);
        assert!(
            !html.contains("<der>") && !html.contains("<script>"),
            "{html}"
        );
        assert!(
            html.contains("&lt;der&gt;") && html.contains("&quot;quoted&quot;"),
            "{html}"
        );
        assert!(html.contains("Drafted by mock; not reviewed."), "{html}");
    }

    #[test]
    fn an_empty_queue_says_so() {
        let html = render(&[], 0, 1);
        assert!(html.contains("Nothing is waiting"), "{html}");
        assert!(html.contains("data-total=\"0\""), "{html}");
    }

    #[test]
    fn a_long_queue_pages() {
        let rows: Vec<Row> = (0..50).map(|n| row(&n.to_string(), "en", None)).collect();
        let html = render(&rows, 120, 2);
        assert!(html.contains("Page 2 of 3"), "{html}");
        assert!(
            html.contains("?page=1\">Previous") && html.contains("?page=3\">Next"),
            "{html}"
        );
    }

    #[test]
    fn parses_query_rows_with_and_without_a_draft() {
        let json = r#"[
            {"id":"a","title":"T","language":"it","start_date":"2027-01-01","stage_id":"s",
             "draft_title":"D","draft_description":"x","draft_model":"m","total":2},
            {"id":"b","title":"U","language":"en","start_date":"2027-01-02","stage_id":"s",
             "draft_title":null,"draft_description":null,"draft_model":null,"total":2}
        ]"#;
        let (rows, total) = parse(json);
        assert_eq!(total, 2);
        assert_eq!(rows[0].draft.as_ref().unwrap().title, "D");
        assert!(rows[1].draft.is_none());
    }

    #[test]
    fn the_sql_targets_the_same_language_as_the_rule() {
        // The CASE in the SQL and target_language must agree.
        assert!(QUEUE_SQL.contains("CASE WHEN i.language = 'it' THEN 'en' ELSE 'it' END"));
        assert_eq!(crate::target_language("it"), "en");
        assert_eq!(crate::target_language("xx"), "it");
    }
}
