//! The sidebar and front-page tiles, rendered here because the kernel renders none.
//!
//! The brief's tiles are views of the conference data: CFPs closing soon,
//! conferences this month, a topic cloud with counts, recent comments, and the
//! front page's upcoming conferences. The kernel has a tile type for exactly this,
//! `gather_query`, and it renders an empty `<div data-query-id>` that nothing ever
//! fills (`crates/kernel/src/services/tile.rs:109-119`); the type list is closed,
//! so a plugin cannot add one (FRICTION.md, G-TILE-GATHER-QUERY-RENDERS-NOTHING).
//! The one type that does render server-side is `custom_html`, whose body is a
//! row in the kernel's `tile` table, read on every request.
//!
//! So this module keeps those bodies current. On every cron call it runs one
//! query per tile, renders the rows to HTML here, escaping every value, and
//! writes the body back only when it changed. The tiles are still the kernel's:
//! placed, weighted and shown or hidden by role and path through the kernel's own
//! visibility rules in `demo/config/tile.*.yml`, and editable at
//! `/admin/structure/tiles`. What this module owns is the content, which is stale
//! by at most one cron period (a minute on the demo).
//!
//! Two windows are the brief's own and are exact here, where the gathers behind
//! `/cfps/closing-soon` and `/conferences/this-month` can only count: a gather
//! can say "today" and nothing relative to it (G-NO-RELATIVE-DATE-FILTER-VALUES),
//! and SQL can say `current_date + 14`.
//!
//! What cannot be done this way is anything per viewer. A tile body is one
//! string for everybody, so "My Subscriptions" can only be a link to the page
//! that lists them, shown to signed-in members by a role rule.

use trovato_sdk::host;

/// The Live stage, which is all a tile shows: a tile is rendered for whoever is
/// looking, so it may only carry what an anonymous visitor may see.
const LIVE: &str = "0193a5a0-0000-7000-8000-000000000001";

/// Escape text for HTML element content and double-quoted attributes.
pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
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

/// A uuid as it may appear in a URL: 36 hex digits and dashes, or nothing. The
/// value comes out of the database as text and goes into an `href`.
fn uuid(s: &str) -> Option<&str> {
    (s.len() == 36 && s.chars().all(|c| c.is_ascii_hexdigit() || c == '-')).then_some(s)
}

fn text(row: &serde_json::Value, key: &str) -> String {
    match row.get(key) {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

fn int(row: &serde_json::Value, key: &str) -> i64 {
    match row.get(key) {
        Some(serde_json::Value::Number(n)) => n.as_i64().unwrap_or(0),
        Some(serde_json::Value::String(s)) => s.parse().unwrap_or(0),
        _ => 0,
    }
}

fn rows(sql: &str) -> Option<Vec<serde_json::Value>> {
    let raw = host::query_raw(sql, &[]).ok()?;
    serde_json::from_str(&raw).ok()
}

/// A short list of conferences, each a link with one line of detail under it,
/// and a link to the full listing.
fn conference_list(
    rows: &[serde_json::Value],
    detail: impl Fn(&serde_json::Value) -> String,
    empty: &str,
    more: (&str, &str),
) -> String {
    let mut html = String::new();
    if rows.is_empty() {
        html.push_str(&format!("<p class=\"tile-list__empty\">{}</p>", esc(empty)));
    } else {
        html.push_str("<ul class=\"tile-list\">");
        for row in rows {
            let id = text(row, "id");
            let Some(id) = uuid(&id) else { continue };
            html.push_str(&format!(
                "<li class=\"tile-list__item\"><a href=\"/item/{id}\">{}</a><span class=\"tile-list__detail\">{}</span></li>",
                esc(&text(row, "title")),
                detail(row)
            ));
        }
        html.push_str("</ul>");
    }
    html.push_str(&format!(
        "<p class=\"tile-more\"><a href=\"{}\">{} &rarr;</a></p>",
        more.0,
        esc(more.1)
    ));
    html
}

fn place(row: &serde_json::Value) -> String {
    let city = text(row, "city");
    let country = text(row, "country");
    match (city.is_empty(), country.is_empty()) {
        (false, false) => format!("{city}, {country}"),
        (false, true) => city,
        (true, false) => country,
        (true, true) => String::new(),
    }
}

/// The brief's "CFPs Closing Soon": calls for papers closing today or in the next
/// fourteen days, with a submission link, soonest first, five at most.
fn cfps_closing_soon() -> Option<String> {
    let sql = format!(
        "SELECT id::text AS id, title, fields->>'field_cfp_end_date' AS closes, \
                (to_date(fields->>'field_cfp_end_date', 'YYYY-MM-DD') - current_date) AS days \
           FROM item \
          WHERE type = 'conference' AND status = 1 AND stage_id = '{LIVE}' \
            AND fields->>'field_cfp_end_date' ~ '^[0-9]{{4}}-[0-9]{{2}}-[0-9]{{2}}$' \
            AND fields->>'field_cfp_end_date' BETWEEN to_char(current_date, 'YYYY-MM-DD') \
                                                  AND to_char(current_date + 14, 'YYYY-MM-DD') \
            AND COALESCE(fields->>'field_cfp_url', '') <> '' \
          ORDER BY fields->>'field_cfp_end_date', title LIMIT 5"
    );
    let found = rows(&sql)?;
    Some(conference_list(
        &found,
        |row| {
            let days = int(row, "days");
            let when = match days {
                0 => "closes today".to_string(),
                1 => "closes tomorrow".to_string(),
                n => format!("{n} days left"),
            };
            format!("{} &middot; {}", esc(&text(row, "closes")), esc(&when))
        },
        "No calls for papers close in the next two weeks.",
        ("/cfps", "All open calls for papers"),
    ))
}

/// The brief's "Conferences This Month": starting from the first of this
/// calendar month to the first of the next, earliest first, five at most.
fn conferences_this_month() -> Option<String> {
    let sql = format!(
        "SELECT id::text AS id, title, fields->>'field_start_date' AS starts, \
                fields->>'field_city' AS city, fields->>'field_country' AS country \
           FROM item \
          WHERE type = 'conference' AND status = 1 AND stage_id = '{LIVE}' \
            AND fields->>'field_start_date' >= to_char(date_trunc('month', current_date), 'YYYY-MM-DD') \
            AND fields->>'field_start_date' <  to_char(date_trunc('month', current_date) + interval '1 month', 'YYYY-MM-DD') \
          ORDER BY fields->>'field_start_date', title LIMIT 5"
    );
    let found = rows(&sql)?;
    Some(conference_list(
        &found,
        |row| {
            let place = place(row);
            if place.is_empty() {
                esc(&text(row, "starts"))
            } else {
                format!("{} &middot; {}", esc(&text(row, "starts")), esc(&place))
            }
        },
        "Nothing on the calendar for the rest of this month.",
        ("/conferences/this-month", "This month"),
    ))
}

/// The front page's upcoming conferences: the next six to start, as cards.
fn front_upcoming() -> Option<String> {
    let sql = format!(
        "SELECT id::text AS id, title, fields->>'field_start_date' AS starts, \
                fields->>'field_end_date' AS ends, fields->>'field_city' AS city, \
                fields->>'field_country' AS country, fields->>'field_language' AS language \
           FROM item \
          WHERE type = 'conference' AND status = 1 AND stage_id = '{LIVE}' \
            AND fields->>'field_start_date' >= to_char(current_date, 'YYYY-MM-DD') \
          ORDER BY fields->>'field_start_date', title LIMIT 6"
    );
    let found = rows(&sql)?;
    let mut html = String::from("<div class=\"front-upcoming\">");
    if found.is_empty() {
        html.push_str("<p class=\"tile-list__empty\">No upcoming conferences yet.</p>");
    }
    for row in &found {
        let id = text(row, "id");
        let Some(id) = uuid(&id) else { continue };
        let ends = text(row, "ends");
        let dates = if ends.is_empty() || ends == text(row, "starts") {
            esc(&text(row, "starts"))
        } else {
            format!("{} &ndash; {}", esc(&text(row, "starts")), esc(&ends))
        };
        let language = text(row, "language").to_lowercase();
        let badge = if language.len() == 2 && language.chars().all(|c| c.is_ascii_lowercase()) {
            format!(
                "<span class=\"lang-badge\" lang=\"{language}\">{}</span>",
                language.to_uppercase()
            )
        } else {
            String::new()
        };
        html.push_str(&format!(
            "<article class=\"front-card\"><h3 class=\"front-card__title\"><a href=\"/item/{id}\">{}</a></h3>\
             <p class=\"front-card__meta\"><span class=\"front-card__dates\">{dates}</span> \
             <span class=\"front-card__place\">{}</span> {badge}</p></article>",
            esc(&text(row, "title")),
            esc(&place(row)),
        ));
    }
    html.push_str("</div><p class=\"tile-more\"><a href=\"/conferences\">Every upcoming conference &rarr;</a></p>");
    Some(html)
}

/// The brief's "Topic Cloud": every topic with at least one upcoming Live
/// conference, sized by how many, each linked to its listing. A gather cannot
/// count (no aggregation in a gather definition), which is why this one is code
/// even where the other tiles could have been a gather.
fn topic_cloud() -> Option<String> {
    let sql = format!(
        "SELECT c.slug, c.label, count(DISTINCT i.id) AS n \
           FROM item i \
           CROSS JOIN LATERAL jsonb_array_elements_text( \
                CASE WHEN jsonb_typeof(i.fields->'field_topics') = 'array' \
                     THEN i.fields->'field_topics' ELSE '[]'::jsonb END) AS t(value) \
           JOIN category_tag c ON c.id::text = t.value AND c.category_id = 'topics' \
          WHERE i.type = 'conference' AND i.status = 1 AND i.stage_id = '{LIVE}' \
            AND i.fields->>'field_start_date' >= to_char(current_date, 'YYYY-MM-DD') \
            AND c.slug IS NOT NULL \
          GROUP BY c.slug, c.label ORDER BY c.label"
    );
    let found = rows(&sql)?;
    let max = found.iter().map(|r| int(r, "n")).max().unwrap_or(0).max(1);
    let mut html = String::new();
    if found.is_empty() {
        html.push_str("<p class=\"tile-list__empty\">No upcoming conference is tagged yet.</p>");
    } else {
        html.push_str("<ul class=\"topic-cloud\">");
        for row in &found {
            let n = int(row, "n");
            // Five sizes, by share of the busiest topic.
            let size = 1 + (n * 4) / max;
            let slug = text(row, "slug");
            if !slug
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                continue;
            }
            html.push_str(&format!(
                "<li><a class=\"topic-cloud__term topic-cloud__term--{size}\" href=\"/topics/{slug}\">{} <span class=\"topic-cloud__count\">{n}</span></a></li>",
                esc(&text(row, "label"))
            ));
        }
        html.push_str("</ul>");
    }
    html.push_str("<p class=\"tile-more\"><a href=\"/topics\">The whole topic tree &rarr;</a></p>");
    Some(html)
}

/// The brief's "Recent Comments": the five newest published comments on Live
/// items, with who wrote them and where, linked to the comment itself. Markup
/// in a comment body is stripped, then the text escaped; a tile is no place for
/// a commenter's formatting.
///
/// The authors' names come from the kernel's user directory
/// (`host::load_users`, the manifest's `user_directory`): `users` is a table no
/// plugin may read since the kernel's 0.107 line, which is right, and the
/// directory answers a name without anything else about the account.
fn recent_comments() -> Option<String> {
    let sql = format!(
        "SELECT c.id::text AS id, c.item_id::text AS item_id, i.title, \
                c.author_id::text AS author_id, \
                left(regexp_replace(c.body, '<[^>]*>', '', 'g'), 140) AS excerpt \
           FROM comment c \
           JOIN item i ON i.id = c.item_id \
          WHERE c.status = 1 AND i.status = 1 AND i.stage_id = '{LIVE}' \
          ORDER BY c.created DESC, c.id LIMIT 5"
    );
    let found = rows(&sql)?;
    let author_ids: Vec<String> = found.iter().map(|row| text(row, "author_id")).collect();
    let author_refs: Vec<&str> = author_ids
        .iter()
        .map(String::as_str)
        .filter(|id| uuid(id).is_some())
        .collect();
    let authors: std::collections::HashMap<String, String> = if author_refs.is_empty() {
        std::collections::HashMap::new()
    } else {
        host::load_users(&author_refs)
            .ok()?
            .into_iter()
            .map(|user| (user.id, user.name))
            .collect()
    };
    let mut html = String::new();
    if found.is_empty() {
        html.push_str("<p class=\"tile-list__empty\">No comments yet.</p>");
        return Some(html);
    }
    html.push_str("<ul class=\"tile-list tile-list--comments\">");
    for row in &found {
        let (id, item) = (text(row, "id"), text(row, "item_id"));
        let (Some(id), Some(item)) = (uuid(&id), uuid(&item)) else {
            continue;
        };
        html.push_str(&format!(
            "<li class=\"tile-list__item\"><a href=\"/item/{item}#comment-{id}\">{}</a>\
             <span class=\"tile-list__detail\">{} on {}</span></li>",
            esc(text(row, "excerpt").trim()),
            esc(authors
                .get(&text(row, "author_id"))
                .map_or("Anonymous", String::as_str)),
            esc(&text(row, "title")),
        ));
    }
    html.push_str("</ul>");
    Some(html)
}

/// Write one tile's body if it changed. Returns whether a row was written; a
/// tile missing from the table (its config not imported) writes nothing.
fn store(machine_name: &str, body: &str) -> bool {
    host::execute_raw(
        "UPDATE tile \
            SET config = jsonb_build_object('body', $1::text, 'format', 'full_html'), \
                tile_type = 'custom_html', \
                changed = EXTRACT(EPOCH FROM NOW())::bigint \
          WHERE machine_name = $2 AND config->>'body' IS DISTINCT FROM $1::text",
        &[serde_json::json!(body), serde_json::json!(machine_name)],
    )
    .map(|n| n > 0)
    .unwrap_or(false)
}

/// The tiles this module keeps, by the machine names `demo/config/tile.*.yml`
/// gives them.
pub const TILES: [&str; 5] = [
    "cfps_closing_soon",
    "conferences_this_month",
    "front_upcoming",
    "topic_cloud",
    "recent_comments",
];

/// Re-render every tile. Reports, per tile, `"written"`, `"unchanged"` or
/// `"query failed"`; a failure leaves that tile's last body in place.
pub fn refresh() -> serde_json::Value {
    let mut out = serde_json::Map::new();
    for name in TILES {
        let body = match name {
            "cfps_closing_soon" => cfps_closing_soon(),
            "conferences_this_month" => conferences_this_month(),
            "front_upcoming" => front_upcoming(),
            "topic_cloud" => topic_cloud(),
            _ => recent_comments(),
        };
        let state = match body {
            Some(body) if store(name, &body) => "written",
            Some(_) => "unchanged",
            None => "query failed",
        };
        out.insert(name.to_string(), serde_json::json!(state));
    }
    serde_json::Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_are_escaped_for_content_and_attributes() {
        assert_eq!(
            esc(r#"<b>"Rust" & 'Go'</b>"#),
            "&lt;b&gt;&quot;Rust&quot; &amp; &#39;Go&#39;&lt;/b&gt;"
        );
    }

    #[test]
    fn only_a_uuid_reaches_an_href() {
        assert!(uuid("0193a5a0-0000-7000-8000-000000000001").is_some());
        assert!(uuid("javascript:alert(1)//aaaaaaaaaaaaaaaaaaaaaaaa").is_none());
        assert!(uuid("\"><script>").is_none());
    }

    #[test]
    fn a_list_escapes_titles_and_skips_rows_without_a_real_id() {
        let rows = vec![
            serde_json::json!({"id": "0193a5a0-0000-7000-8000-000000000001", "title": "<i>Conf</i>"}),
            serde_json::json!({"id": "not-an-id", "title": "Dropped"}),
        ];
        let html = conference_list(&rows, |_| String::new(), "none", ("/x", "More"));
        assert!(html.contains("&lt;i&gt;Conf&lt;/i&gt;"));
        assert!(!html.contains("Dropped"));
        assert!(!html.contains("<i>"));
    }

    #[test]
    fn an_empty_list_says_so_and_still_links_onward() {
        let html = conference_list(&[], |_| String::new(), "Nothing here.", ("/cfps", "All"));
        assert!(html.contains("Nothing here."));
        assert!(html.contains("href=\"/cfps\""));
    }

    #[test]
    fn refresh_reports_every_tile() {
        let out = refresh();
        for name in TILES {
            assert!(out.get(name).is_some(), "{name} missing from {out}");
        }
    }
}
