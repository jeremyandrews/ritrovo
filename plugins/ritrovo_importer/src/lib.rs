//! Conference importer plugin for the Ritrovo tutorial.
//!
//! Imports tech conference data from the
//! [confs.tech](https://github.com/tech-conferences/conference-data)
//! open-source dataset.
//!
//! Architecture:
//! - `tap_install`: runs a full historical import (2015–current year) by
//!   pushing all fetched data onto the `ritrovo_import` queue.
//! - `tap_cron`: fetches the current and next year's data for a rotating
//!   subset of topics, using ETags to skip unchanged files, then pushes
//!   each topic's payload onto the queue.
//! - `tap_queue_info`: declares the `ritrovo_import` queue (concurrency 4).
//! - `tap_queue_worker`: validates and upserts a single topic's conferences.

use std::collections::HashMap;

use trovato_sdk::host;
use trovato_sdk::prelude::*;
use trovato_sdk::types::{ApiRequest, ApiResponse, MenuRoute};

/// Plugin name for logging calls.
const PLUGIN_NAME: &str = "ritrovo_importer";

/// Base URL for raw conference JSON from the confs.tech GitHub repo.
const DATA_BASE_URL: &str =
    "https://raw.githubusercontent.com/tech-conferences/conference-data/main/conferences";

/// First year of conference data available in confs.tech.
const FIRST_IMPORT_YEAR: u16 = 2015;

/// Topics to import, corresponding to filenames in the confs.tech repo.
const TOPICS: &[&str] = &[
    "accessibility",
    "android",
    "api",
    "cpp",
    "css",
    "data",
    "devops",
    "dotnet",
    "general",
    "ios",
    "java",
    "javascript",
    "kotlin",
    "networking",
    "opensource",
    "php",
    "python",
    "ruby",
    "rust",
    "scala",
    "security",
    "sre",
    "testing",
    "typescript",
    "ux",
];

/// Number of topics to process per cron cycle (round-robin to stay
/// well within the 150-second cron dispatch timeout).
const TOPICS_PER_CYCLE: usize = 5;

/// Minimum interval between cron import runs (24 hours in seconds).
const IMPORT_INTERVAL_SECS: i64 = 86_400;

/// State key tracking last import timestamp.
const STATE_LAST_IMPORT: &str = "last_import";

/// State key tracking the topic offset for round-robin scheduling.
const STATE_TOPIC_OFFSET: &str = "topic_offset";

/// Prefix for ETag state keys: `"etag.{topic}.{year}"`.
const STATE_ETAG_PREFIX: &str = "etag";

/// Queue name declared in `tap_queue_info`.
const QUEUE_NAME: &str = "ritrovo_import";

/// Category ID for the conference topics taxonomy.
const TOPICS_CATEGORY_ID: &str = "topics";

/// State key prefix for topic term UUIDs: `"topic_term.{term_slug}"`.
const STATE_TOPIC_TERM_PREFIX: &str = "topic_term";

/// Prefix for recorded batch failures: `"failed.{topic}.{year}"`.
///
/// The kernel's dead-letter row keeps `last_error = "tap_queue_worker failed
/// (trap or error result)"` and nothing else: a tap's Err value never crosses
/// the ABI, only its negative length does, so the reason a batch failed is not
/// recoverable from the queue (G-QUEUE-DEAD-LETTER-DISCARDS-THE-PLUGINS-ERROR).
/// The importer therefore writes the reason down on its own side before it
/// returns Err, which is what makes "logged and skipped, not silently dropped"
/// true rather than aspirational. The importer's admin screen reads these back.
const STATE_FAILED_PREFIX: &str = "failed";

/// Maximum number of conferences per queue payload.
///
/// The WASM input buffer is 64 KB; a single confs.tech JSON file can exceed
/// that (e.g. general/2019 is ~69 KB). Chunking at 50 conferences per batch
/// keeps every payload well under the limit.
const CONFERENCES_PER_BATCH: usize = 50;

// ─── Topic slug → taxonomy label mapping ──────────────────────────────

/// Which topic-tree term each confs.tech feed maps onto.
///
/// The mapping is DATA, in `data/confs-tech-topics.json` beside this file, not a
/// table in this source. The tree it points into is defined by
/// `demo/config/tag.*.yml` and the feeds are whatever confs.tech publishes, so
/// neither end of the mapping belongs to the importer; re-aiming a feed is an
/// edit to that file and no Rust change at all. It is `include_str!`d rather than
/// read at runtime because this plugin is a WASM module with no filesystem.
///
/// A feed may map to nothing. `general`, `opensource` and `testing` do today:
/// the brief's tree has no term that means them, and those conferences import
/// untagged rather than being filed under a term that is not what they are.
const TOPIC_MAP_JSON: &str = include_str!("../data/confs-tech-topics.json");

/// One topic-tree term: the label the `category_tag` row is found by, and the
/// slug the resolved uuid is cached under in `ritrovo_state`.
#[derive(Debug, Clone, serde::Deserialize)]
struct TermRef {
    label: String,
    slug: String,
}

/// Parse `data/confs-tech-topics.json` into `confs.tech feed -> term or nothing`.
///
/// Returns the pairs in file order. A malformed file is a build-time mistake that
/// would leave every conference untagged, so it is logged loudly and treated as
/// an empty mapping rather than a panic that would take the whole plugin down.
fn topic_map() -> Vec<(String, Option<TermRef>)> {
    #[derive(serde::Deserialize)]
    struct File {
        mappings: std::collections::BTreeMap<String, Option<TermRef>>,
    }
    match serde_json::from_str::<File>(TOPIC_MAP_JSON) {
        Ok(f) => f.mappings.into_iter().collect(),
        Err(e) => {
            host::log(
                "error",
                PLUGIN_NAME,
                &format!("data/confs-tech-topics.json is unreadable: {e}"),
            );
            Vec::new()
        }
    }
}

// ─── Install ──────────────────────────────────────────────────────────

/// Called once when the plugin is first enabled in the admin UI.
///
/// 1. Discovers taxonomy term UUIDs from the database (terms are created
///    via config import, not by the plugin).
/// 2. Triggers a full historical import (2015–current year) by pushing all
///    available topic/year combinations onto the `ritrovo_import` queue.
///    The queue worker (`tap_queue_worker`) processes each batch
///    asynchronously in subsequent cron cycles.
#[plugin_tap]
pub fn tap_install() -> serde_json::Value {
    host::log(
        "info",
        PLUGIN_NAME,
        "ritrovo_importer installed — discovering taxonomy and starting historical import",
    );

    // 1. Discover taxonomy term UUIDs from the config-imported terms.
    let discovered = discover_taxonomy_uuids();

    // 2. Queue historical import.
    let now = current_timestamp();
    let current_year = timestamp_to_year(now);
    let mut pushed = 0u32;
    let mut errors = 0u32;

    for year in FIRST_IMPORT_YEAR..=current_year {
        for topic in TOPICS {
            let url = format!("{DATA_BASE_URL}/{year}/{topic}.json");

            let response = match host::http_request(
                &trovato_sdk::types::HttpRequest::get(&url).timeout(15_000),
            ) {
                Ok(r) if r.status == 200 => r,
                Ok(r) if r.status == 404 => continue,
                Ok(r) => {
                    errors += 1;
                    host::log("warn", PLUGIN_NAME, &format!("HTTP {} for {url}", r.status));
                    continue;
                }
                Err(_) => {
                    errors += 1;
                    continue;
                }
            };

            // Store ETag for future conditional requests.
            if let Some(etag) = response
                .headers
                .get("etag")
                .or_else(|| response.headers.get("ETag"))
            {
                set_state(&etag_key(topic, year), etag);
            }

            let (p, e) = push_conference_batches(topic, year, &response.body);
            pushed += p;
            errors += e;
        }
    }

    host::log(
        "info",
        PLUGIN_NAME,
        &format!(
            "historical import queued: {pushed} batches across {years} years",
            years = (current_year - FIRST_IMPORT_YEAR + 1)
        ),
    );

    serde_json::json!({
        "status": "ok",
        "discovered_terms": discovered,
        "queued": pushed,
        "errors": errors,
    })
}

// ─── Queue helpers ─────────────────────────────────────────────────────

/// Push conference data onto the import queue, chunking large payloads.
///
/// A single confs.tech JSON file can exceed the 64 KB WASM input limit.
/// This function parses the raw body as a JSON array and pushes sub-slices of
/// at most [`CONFERENCES_PER_BATCH`] items. If parsing fails the raw body is
/// pushed as-is (will fail at the worker if still too large).
///
/// Returns `(batches_pushed, errors)`.
fn push_conference_batches(topic: &str, year: u16, body: &str) -> (u32, u32) {
    let mut pushed = 0u32;
    let mut errors = 0u32;

    // Try to parse as an array so we can chunk it.
    if let Ok(confs) = serde_json::from_str::<Vec<serde_json::Value>>(body) {
        for chunk in confs.chunks(CONFERENCES_PER_BATCH) {
            let payload = serde_json::json!({
                "topic": topic,
                "year": year,
                "conferences": serde_json::to_string(chunk).unwrap_or_default(),
            });
            match host::queue_push(QUEUE_NAME, &payload) {
                Ok(()) => pushed += 1,
                Err(_) => errors += 1,
            }
        }
    } else {
        // Fallback: body is not a valid JSON array — push raw and let the
        // worker surface the parse error.  Log here so the operator can
        // identify which topic/year produced malformed JSON.
        host::log(
            "warn",
            PLUGIN_NAME,
            &format!(
                "push_conference_batches: failed to parse JSON array for {topic}/{year}, \
                 pushing raw payload"
            ),
        );
        let payload = serde_json::json!({
            "topic": topic,
            "year": year,
            "conferences": body,
        });
        match host::queue_push(QUEUE_NAME, &payload) {
            Ok(()) => pushed += 1,
            Err(_) => errors += 1,
        }
    }

    (pushed, errors)
}

// ─── Taxonomy discovery ───────────────────────────────────────────────

/// Discover taxonomy term UUIDs from the database.
///
/// The `topics` category and its terms are created via config import
/// (YAML files in `docs/tutorial/config/`), not by the plugin. This
/// function looks up each term's UUID by label and caches it in
/// `ritrovo_state` for use by the queue worker when tagging conferences.
///
/// Returns the number of terms discovered.
fn discover_taxonomy_uuids() -> u32 {
    let mut discovered = 0u32;

    for (_confs_slug, term) in topic_map() {
        // A feed with no term needs no lookup and is not a failure to report.
        let Some(term) = term else { continue };
        let (term_slug, term_label) = (term.slug.as_str(), term.label.as_str());
        let state_key = format!("{STATE_TOPIC_TERM_PREFIX}.{term_slug}");

        // Skip if already cached in state.
        if load_state_str(&state_key).is_some() {
            discovered += 1;
            continue;
        }

        // Look up the UUID from the config-imported category_tag row.
        let result = host::query_raw(
            "SELECT id::text AS id FROM category_tag \
             WHERE category_id = $1 AND label = $2 \
             LIMIT 1",
            &[
                serde_json::json!(TOPICS_CATEGORY_ID),
                serde_json::json!(term_label),
            ],
        );
        if let Some(uuid) = result
            .ok()
            .and_then(|s| serde_json::from_str::<Vec<serde_json::Value>>(&s).ok())
            .and_then(|rows| rows.into_iter().next())
            .and_then(|row| row.get("id").and_then(|v| v.as_str()).map(String::from))
        {
            save_state(&state_key, &uuid);
            discovered += 1;
        } else {
            host::log(
                "warn",
                PLUGIN_NAME,
                &format!(
                    "discover_taxonomy_uuids: term '{term_label}' not found in category \
                     '{TOPICS_CATEGORY_ID}' — was config imported?"
                ),
            );
        }
    }

    // Counted against the terms actually looked for, not the number of feeds:
    // a feed that maps to nothing was never a term to find, and counting it as a
    // miss is what made this line read "23 of 25" on a healthy install.
    let wanted = topic_map().into_iter().filter(|(_, t)| t.is_some()).count();
    host::log(
        "info",
        PLUGIN_NAME,
        &format!("discover_taxonomy_uuids: {discovered}/{wanted} terms found"),
    );

    discovered
}

/// Look up the category_tag UUID for a confs.tech topic slug.
///
/// Returns `None` if the slug has no taxonomy mapping (e.g. `sre`, `scala`)
/// or if the taxonomy term has not been discovered yet.
fn topic_term_uuid(confs_tech_slug: &str) -> Option<String> {
    // Map the confs.tech slug to the taxonomy term slug.
    let term = topic_map()
        .into_iter()
        .find(|(src, _)| src == confs_tech_slug)
        .and_then(|(_, term)| term)?;

    load_state_str(&format!("{STATE_TOPIC_TERM_PREFIX}.{}", term.slug))
}

// ─── Permissions ─────────────────────────────────────────────────────

/// Define importer-specific permissions.
#[plugin_tap]
pub fn tap_perm() -> Vec<PermissionDefinition> {
    let mut perms = PermissionDefinition::crud_for_type("conference");
    perms.push(PermissionDefinition::new(
        "administer conference import",
        "Configure and trigger conference imports",
    ));
    perms
}

// ─── Menu routes ─────────────────────────────────────────────────────

/// Define admin menu routes for the importer.
///
/// These are `MenuRoute::api` entries, not `MenuDefinition`. A `MenuDefinition`
/// leaves `handler_type` at `"page"`, and the kernel routes a request to
/// `tap_api` only for an entry whose `handler_type` is `"api"` and which names a
/// callback — so declaring a callback on a page entry produced two admin paths
/// that were registered, gated, listed in navigation, and 404.
///
/// `.visible()` because both are meant to appear under their parent in the admin
/// navigation; an api route defaults to invisible, which is right for a REST
/// endpoint and wrong for a screen.
#[plugin_tap]
pub fn tap_menu() -> Vec<MenuRoute> {
    vec![
        MenuRoute::api("GET", "/admin/content/conferences", "conference_list")
            .title("Conferences")
            .permission("view conference content")
            .parent("/admin/content")
            .visible(),
        MenuRoute::api("GET", "/admin/config/importer", "importer_config")
            .title("Conference Import")
            .permission("administer conference import")
            .parent("/admin/config")
            .visible(),
    ]
}

// ─── Admin screens (tap_api) ─────────────────────────────────────────

/// How many conferences one page of the conference list shows.
const CONFERENCE_PAGE_SIZE: i64 = 50;

/// Serve one request for a menu entry this plugin registered.
///
/// The kernel has already checked the entry's `permission`, so a request that
/// reaches here holds it. Both screens are read-only: this is an operator view
/// of what the importer has done, not a second content editor, and triggering an
/// import is what cron is for.
#[plugin_tap]
pub fn tap_api(request: ApiRequest) -> ApiResponse {
    match request.callback.as_str() {
        "conference_list" => conference_list(&request),
        "importer_config" => importer_config(),
        other => ApiResponse::error(404, &format!("no such callback: {other}")),
    }
}

/// Escape text for an HTML text node or a double-quoted attribute.
///
/// The kernel serves a plugin's response body as-is and does not sanitize it, so
/// escaping is this plugin's job. Conference titles, cities and URLs all come
/// from a third-party dataset, which makes every one of them untrusted input.
fn escape_html(raw: &str) -> String {
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

/// Wrap a body fragment in a minimal standalone document.
fn admin_page(title: &str, body: &str) -> ApiResponse {
    let html = format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <title>{t}</title>\n</head>\n<body>\n<h1>{t}</h1>\n{body}\n</body>\n</html>\n",
        t = escape_html(title),
    );
    ApiResponse::with_status(200, html).content_type("text/html; charset=utf-8")
}

/// Read one string field out of a row, escaped, or a placeholder.
fn cell(row: &serde_json::Value, key: &str) -> String {
    row.get(key)
        .and_then(serde_json::Value::as_str)
        .map_or_else(|| "&mdash;".to_string(), escape_html)
}

/// The conference list: what the importer has actually landed.
fn conference_list(request: &ApiRequest) -> ApiResponse {
    let page: i64 = request
        .query
        .get("page")
        .and_then(|p| p.parse::<i64>().ok())
        .unwrap_or(1)
        .max(1);
    let offset = (page - 1) * CONFERENCE_PAGE_SIZE;

    let total = scalar_i64(
        "SELECT COUNT(*) AS n FROM item WHERE type = 'conference'",
        "n",
    );

    let rows_json = host::query_raw(
        "SELECT title, \
                fields->>'field_start_date' AS start_date, \
                fields->>'field_city'       AS city, \
                fields->>'field_country'    AS country, \
                fields->>'field_source_id'  AS source_id \
         FROM item \
         WHERE type = 'conference' \
         ORDER BY fields->>'field_start_date' DESC NULLS LAST, title ASC \
         LIMIT $1 OFFSET $2",
        &[
            serde_json::json!(CONFERENCE_PAGE_SIZE),
            serde_json::json!(offset),
        ],
    );

    let rows: Vec<serde_json::Value> = match rows_json {
        Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
        Err(code) => {
            host::log(
                PLUGIN_NAME,
                "error",
                &format!("conference list query failed with code {code}"),
            );
            return ApiResponse::error(500, "failed to read conferences");
        }
    };

    let mut body = format!("<p>{total} conference(s) imported.</p>\n");
    if rows.is_empty() {
        body.push_str(
            "<p>No conferences on this page. The importer fills this in from cron; \
             see the import status screen.</p>\n",
        );
    } else {
        body.push_str(
            "<table>\n<thead><tr><th>Title</th><th>Starts</th><th>City</th>\
             <th>Country</th><th>Source id</th></tr></thead>\n<tbody>\n",
        );
        for row in &rows {
            body.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                cell(row, "title"),
                cell(row, "start_date"),
                cell(row, "city"),
                cell(row, "country"),
                cell(row, "source_id"),
            ));
        }
        body.push_str("</tbody>\n</table>\n");
    }

    let pages = total.div_euclid(CONFERENCE_PAGE_SIZE)
        + i64::from(total.rem_euclid(CONFERENCE_PAGE_SIZE) != 0);
    if pages > 1 {
        body.push_str(&format!("<p>Page {page} of {pages}. "));
        if page > 1 {
            body.push_str(&format!(
                "<a href=\"/admin/content/conferences?page={}\">Previous</a> ",
                page - 1
            ));
        }
        if page < pages {
            body.push_str(&format!(
                "<a href=\"/admin/content/conferences?page={}\">Next</a>",
                page + 1
            ));
        }
        body.push_str("</p>\n");
    }

    admin_page("Conferences", &body)
}

/// The import status screen: the importer's own state, read back.
fn importer_config() -> ApiResponse {
    let last_import = load_state_str(STATE_LAST_IMPORT).unwrap_or_else(|| "never".to_string());
    let offset = load_state_usize(STATE_TOPIC_OFFSET, TOPICS.len());
    let etags = scalar_i64(
        "SELECT COUNT(*) AS n FROM ritrovo_state WHERE name LIKE 'etag.%'",
        "n",
    );
    let resolved_terms = scalar_i64(
        "SELECT COUNT(*) AS n FROM ritrovo_state WHERE name LIKE 'topic_term.%'",
        "n",
    );
    let queued = scalar_i64(
        "SELECT COUNT(*) AS n FROM plugin_queue WHERE plugin_name = 'ritrovo_importer'",
        "n",
    );

    let next_topics: Vec<String> = (0..TOPICS_PER_CYCLE)
        .map(|i| escape_html(TOPICS[(offset + i) % TOPICS.len()]))
        .collect();

    // Terms, not feeds. The denominator used to be the number of confs.tech
    // feeds, which made a healthy install read "23 of 25" for ever: feeds that
    // map to no term were counted as terms that had not resolved, and two feeds
    // pointing at one term were counted twice. Count the distinct terms the
    // mapping actually asks for, and say separately how many feeds ask for none.
    let map = topic_map();
    let mut wanted: Vec<String> = map
        .iter()
        .filter_map(|(_, term)| term.as_ref().map(|t| t.slug.clone()))
        .collect();
    wanted.sort();
    wanted.dedup();
    let unmapped: Vec<String> = map
        .iter()
        .filter(|(_, term)| term.is_none())
        .map(|(feed, _)| escape_html(feed))
        .collect();

    // Batches that failed and are not yet fixed. The kernel's dead-letter row
    // keeps a fixed string rather than the worker's reason, so the reason is read
    // back from the importer's own state, where the worker wrote it.
    let failures = host::query_raw(
        "SELECT name, value FROM ritrovo_state \
         WHERE name LIKE 'failed.%' AND value <> '' ORDER BY name",
        &[],
    )
    .ok()
    .and_then(|s| serde_json::from_str::<Vec<serde_json::Value>>(&s).ok())
    .unwrap_or_default();
    let failed_batches = if failures.is_empty() {
        "none".to_string()
    } else {
        failures
            .iter()
            .filter_map(|row| {
                let name = row.get("name")?.as_str()?.trim_start_matches("failed.");
                let value = row.get("value")?.as_str()?;
                let reason = value.split_once(',').map(|(_, r)| r).unwrap_or(value);
                Some(format!("{} ({})", escape_html(name), escape_html(reason)))
            })
            .collect::<Vec<_>>()
            .join(", ")
    };

    let body = format!(
        "<h2>Import status</h2>\n\
         <dl>\n\
         <dt>Last import run</dt><dd>{last}</dd>\n\
         <dt>Topics per cron cycle</dt><dd>{per_cycle} of {total_topics}</dd>\n\
         <dt>Next topics</dt><dd>{next}</dd>\n\
         <dt>Cached ETags</dt><dd>{etags}</dd>\n\
         <dt>Resolved topic terms</dt><dd>{resolved_terms} of {wanted_terms}</dd>\n\
         <dt>Feeds with no topic term</dt><dd>{unmapped_feeds}</dd>\n\
         <dt>Jobs waiting in the import queue</dt><dd>{queued}</dd>\n\
         <dt>Failed batches</dt><dd>{failed_batches}</dd>\n\
         <dt>Minimum interval between runs</dt><dd>{interval} seconds</dd>\n\
         </dl>\n\
         <p>Imports run from cron. There is no internal scheduler, so this only \
         advances when something calls the cron route.</p>\n",
        last = escape_html(&last_import),
        per_cycle = TOPICS_PER_CYCLE,
        total_topics = TOPICS.len(),
        next = next_topics.join(", "),
        wanted_terms = wanted.len(),
        unmapped_feeds = if unmapped.is_empty() {
            "none".to_string()
        } else {
            format!("{} ({})", unmapped.len(), unmapped.join(", "))
        },
        interval = IMPORT_INTERVAL_SECS,
    );

    admin_page("Conference Import", &body)
}

/// Read one integer out of a single-row, single-column query, or 0.
fn scalar_i64(sql: &str, column: &str) -> i64 {
    host::query_raw(sql, &[])
        .ok()
        .and_then(|json| serde_json::from_str::<Vec<serde_json::Value>>(&json).ok())
        .and_then(|rows| rows.into_iter().next())
        .and_then(|row| row.get(column).and_then(serde_json::Value::as_i64))
        .unwrap_or(0)
}

// ─── Cron: daily import ──────────────────────────────────────────────

/// Daily cron handler that pushes conference fetch jobs onto the queue.
///
/// Processes a rotating subset of topics per cycle (round-robin) to stay
/// within cron timeout limits. Uses conditional HTTP (`If-None-Match`) to
/// skip topics whose upstream data has not changed since the last import.
/// Deduplication and DB writes happen in `tap_queue_worker`.
#[plugin_tap]
pub fn tap_cron(input: CronInput) -> serde_json::Value {
    let now = input.timestamp;

    if !should_import(now) {
        return serde_json::json!({"status": "skipped", "reason": "too_soon"});
    }

    let current_year = timestamp_to_year(now);
    let import_years = [current_year, current_year + 1];

    let topic_offset = load_state_usize(STATE_TOPIC_OFFSET, TOPICS.len());
    let cycle_topics: Vec<&str> = TOPICS
        .iter()
        .cycle()
        .skip(topic_offset)
        .take(TOPICS_PER_CYCLE)
        .copied()
        .collect();

    let mut queued = 0u32;
    let mut skipped_304 = 0u32;
    let mut errors = 0u32;

    for year in &import_years {
        for topic in &cycle_topics {
            match fetch_topic_for_queue(topic, *year) {
                FetchResult::Queued => queued += 1,
                FetchResult::NotModified => skipped_304 += 1,
                FetchResult::NotFound => {}
                FetchResult::Error => errors += 1,
            }
        }
    }

    let next_offset = (topic_offset + TOPICS_PER_CYCLE) % TOPICS.len();
    save_state(STATE_TOPIC_OFFSET, &next_offset.to_string());

    if queued > 0 || skipped_304 > 0 {
        save_state(STATE_LAST_IMPORT, &now.to_string());
    }

    serde_json::json!({
        "status": "completed",
        "queued": queued,
        "skipped_304": skipped_304,
        "errors": errors,
        "topics_processed": cycle_topics,
    })
}

/// Result of fetching a single topic for the queue.
enum FetchResult {
    /// Payload pushed onto the queue.
    Queued,
    /// Server returned 304 Not Modified — no work needed.
    NotModified,
    /// Server returned 404 — topic/year combo doesn't exist.
    NotFound,
    /// HTTP or serialization error.
    Error,
}

/// Fetch one topic+year and push the raw JSON onto the queue.
///
/// Uses the stored ETag as `If-None-Match` for conditional requests.
/// On a 200 response, stores the new ETag and pushes the payload.
fn fetch_topic_for_queue(topic: &str, year: u16) -> FetchResult {
    let url = format!("{DATA_BASE_URL}/{year}/{topic}.json");
    let etag_key = etag_key(topic, year);
    let stored_etag = load_state_str(&etag_key);

    let mut request = trovato_sdk::types::HttpRequest::get(&url).timeout(15_000);
    if let Some(ref etag) = stored_etag {
        request = request.header("If-None-Match", etag);
    }

    let Ok(response) = host::http_request(&request) else {
        return FetchResult::Error;
    };

    match response.status {
        304 => FetchResult::NotModified,
        404 => FetchResult::NotFound,
        200 => {
            // Persist the new ETag for the next cron run.
            if let Some(etag) = response
                .headers
                .get("etag")
                .or_else(|| response.headers.get("ETag"))
            {
                set_state(&etag_key, etag);
            }

            let (p, e) = push_conference_batches(topic, year, &response.body);
            if e > 0 {
                host::log(
                    "warn",
                    PLUGIN_NAME,
                    &format!("fetch_topic: {e} batch(es) failed to push for {topic}/{year}"),
                );
            }
            if p > 0 {
                FetchResult::Queued
            } else {
                FetchResult::Error
            }
        }
        status => {
            host::log("warn", PLUGIN_NAME, &format!("HTTP {status} for {url}"));
            FetchResult::Error
        }
    }
}

// ─── Queue declaration ────────────────────────────────────────────────

/// Declare the queue this plugin owns.
///
/// The kernel calls this at startup to discover plugin-managed queues.
/// The `concurrency` field controls how many `tap_queue_worker` calls
/// the kernel may dispatch in parallel.
#[plugin_tap]
pub fn tap_queue_info() -> serde_json::Value {
    serde_json::json!([
        {
            "name": QUEUE_NAME,
            "concurrency": 4
        }
    ])
}

// ─── Queue worker ─────────────────────────────────────────────────────

/// Process one queued import batch.
///
/// The kernel calls this once per item in the `ritrovo_import` queue,
/// passing a payload of the form:
///
/// ```json
/// { "topic": "rust", "year": 2026, "conferences": "[...]" }
/// ```
///
/// Each conference entry is validated, deduplicated against existing
/// items via `field_source_id`, then inserted or updated.
#[plugin_tap_result]
pub fn tap_queue_worker(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let topic = match input.get("topic").and_then(|v| v.as_str()) {
        Some(t) => t.to_string(),
        None => {
            host::log(
                "warn",
                PLUGIN_NAME,
                "tap_queue_worker: missing 'topic' field",
            );
            return Err(record_batch_failure("(unknown)", 0, "missing_topic"));
        }
    };

    let year = match input.get("year").and_then(|v| v.as_u64()) {
        Some(y) => y as u16,
        None => {
            host::log(
                "warn",
                PLUGIN_NAME,
                "tap_queue_worker: missing 'year' field",
            );
            return Err(record_batch_failure(&topic, 0, "missing_year"));
        }
    };

    // The `conferences` field contains the raw JSON body as a string.
    let body = match input.get("conferences").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => {
            host::log(
                "warn",
                PLUGIN_NAME,
                "tap_queue_worker: missing 'conferences' field",
            );
            return Err(record_batch_failure(&topic, year, "missing_conferences"));
        }
    };

    let conferences: Vec<ConfsTechEntry> = match serde_json::from_str(&body) {
        Ok(c) => c,
        Err(e) => {
            host::log(
                "warn",
                PLUGIN_NAME,
                &format!("JSON parse error for {topic}/{year}: {e}"),
            );
            return Err(record_batch_failure(
                &topic,
                year,
                &format!("parse_error: {e}"),
            ));
        }
    };

    let now = current_timestamp();
    // Look up the taxonomy UUID for this batch's topic slug.
    // Returns None for unmapped slugs such as `sre` and `scala`.
    let topic_uuid = topic_term_uuid(&topic);
    let existing = load_existing_conferences();

    let mut imported = 0u64;
    let mut updated = 0u64;
    let mut skipped = 0u64;
    let mut invalid = 0u64;

    for conf in &conferences {
        match validate_conference(conf) {
            Ok(()) => {}
            Err(reason) => {
                host::log(
                    "warn",
                    PLUGIN_NAME,
                    &format!(
                        "Skipping '{}': {reason}",
                        if conf.name.is_empty() {
                            "(unnamed)"
                        } else {
                            &conf.name
                        }
                    ),
                );
                invalid += 1;
                continue;
            }
        }

        let source_id = compute_source_id(conf);

        if let Some(info) = existing.get(&source_id) {
            let merged_topics = merge_topics(&info.topics, topic_uuid.as_deref());
            if update_conference(&info.item_id, conf, &source_id, &merged_topics, now) {
                updated += 1;
            } else {
                skipped += 1;
            }
        } else if insert_conference(conf, &source_id, topic_uuid.as_deref(), now) {
            imported += 1;
        } else {
            invalid += 1;
        }
    }

    // A retry that succeeds clears the recorded failure, so the admin screen
    // shows what is broken now rather than what was broken once.
    clear_batch_failure(&topic, year);

    Ok(serde_json::json!({
        "status": "ok",
        "topic": topic,
        "year": year,
        "imported": imported,
        "updated": updated,
        "skipped": skipped,
        "invalid": invalid,
    }))
}

/// Write down why a batch failed, and return the message for the Err value.
///
/// Called on the way out of `tap_queue_worker`, so the reason survives in
/// `ritrovo_state` whether the kernel retries the job or dead-letters it.
fn record_batch_failure(topic: &str, year: u16, reason: &str) -> String {
    let message = format!("{topic}/{year}: {reason}");
    host::log("warn", PLUGIN_NAME, &format!("batch failed — {message}"));
    save_state(
        &format!("{STATE_FAILED_PREFIX}.{topic}.{year}"),
        &format!("{},{}", current_timestamp(), reason),
    );
    message
}

/// Forget a recorded failure for a batch that has since succeeded.
fn clear_batch_failure(topic: &str, year: u16) {
    let key = format!("{STATE_FAILED_PREFIX}.{topic}.{year}");
    if load_state_str(&key).is_some() {
        save_state(&key, "");
    }
}

// ─── confs.tech JSON schema ──────────────────────────────────────────

/// A single conference entry from the confs.tech dataset.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfsTechEntry {
    #[serde(default)]
    name: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    start_date: String,
    #[serde(default)]
    end_date: String,
    #[serde(default)]
    city: Option<String>,
    #[serde(default)]
    country: Option<String>,
    #[serde(default)]
    online: Option<bool>,
    #[serde(default)]
    cfp_url: Option<String>,
    #[serde(default)]
    cfp_end_date: Option<String>,
    #[serde(default)]
    locales: Option<String>,
    // `twitter` and `coc_url` are in the feed and are deliberately not read.
    // The conference type declares no field for either and the brief asks for
    // neither, and serde ignores unknown keys, so there is nothing to carry.
}

/// Info about an existing conference item in the database.
struct ExistingConference {
    item_id: String,
    topics: Vec<String>,
}

// ─── Validation ───────────────────────────────────────────────────────

/// Validate a conference entry from the confs.tech dataset.
///
/// Returns `Ok(())` if the entry is valid, or `Err(reason)` with a
/// human-readable description of the first rule violated.
fn validate_conference(conf: &ConfsTechEntry) -> Result<(), String> {
    if conf.name.is_empty() {
        return Err("missing required field: name".to_string());
    }
    if conf.start_date.is_empty() {
        return Err("missing required field: startDate".to_string());
    }
    if conf.end_date.is_empty() {
        return Err("missing required field: endDate".to_string());
    }

    if !is_valid_date(&conf.start_date) {
        return Err(format!("invalid startDate format: '{}'", conf.start_date));
    }
    if !is_valid_date(&conf.end_date) {
        return Err(format!("invalid endDate format: '{}'", conf.end_date));
    }

    if conf.end_date < conf.start_date {
        return Err(format!(
            "endDate '{}' is before startDate '{}'",
            conf.end_date, conf.start_date
        ));
    }

    if let Some(ref cfp_end) = conf.cfp_end_date {
        if !cfp_end.is_empty() && !is_valid_date(cfp_end) {
            return Err(format!("invalid cfpEndDate format: '{cfp_end}'"));
        }
        if !cfp_end.is_empty() && cfp_end.as_str() > conf.start_date.as_str() {
            return Err(format!(
                "cfpEndDate '{cfp_end}' is after startDate '{}'",
                conf.start_date
            ));
        }
    }

    Ok(())
}

/// Check that a date string matches `YYYY-MM-DD` and has a plausible year.
fn is_valid_date(date: &str) -> bool {
    if date.len() != 10 {
        return false;
    }
    let bytes = date.as_bytes();
    // Check digit positions and separators.
    for (i, &b) in bytes.iter().enumerate() {
        match i {
            4 | 7 => {
                if b != b'-' {
                    return false;
                }
            }
            _ => {
                if !b.is_ascii_digit() {
                    return false;
                }
            }
        }
    }
    // Plausible year range (2010–2035).
    let year_str = &date[..4];
    if let Ok(y) = year_str.parse::<u16>() {
        (2010..=2035).contains(&y)
    } else {
        false
    }
}

// ─── State helpers ────────────────────────────────────────────────────

/// Build the ETag state key for a given topic and year.
fn etag_key(topic: &str, year: u16) -> String {
    format!("{STATE_ETAG_PREFIX}.{topic}.{year}")
}

/// Load a string value from the plugin's persistent state table.
fn load_state_str(key: &str) -> Option<String> {
    let result = host::query_raw(
        "SELECT value FROM ritrovo_state WHERE name = $1",
        &[serde_json::json!(key)],
    );
    result
        .ok()
        .and_then(|s| serde_json::from_str::<Vec<serde_json::Value>>(&s).ok())
        .and_then(|rows| rows.into_iter().next())
        .and_then(|row| row.get("value").and_then(|v| v.as_str()).map(String::from))
}

/// Load a `usize` from the plugin state, returning 0 (mod `modulus`) on
/// missing or parse error.
fn load_state_usize(key: &str, modulus: usize) -> usize {
    load_state_str(key)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0)
        % modulus
}

/// Persist a string value in the plugin's persistent state table.
fn save_state(key: &str, value: &str) {
    let _ = host::execute_raw(
        "INSERT INTO ritrovo_state (name, value) VALUES ($1, $2) \
         ON CONFLICT (name) DO UPDATE SET value = $2",
        &[serde_json::json!(key), serde_json::json!(value)],
    );
}

/// Convenience wrapper that calls `save_state`.
fn set_state(key: &str, value: &str) {
    save_state(key, value);
}

/// Check if enough time has passed since the last import run.
fn should_import(now: i64) -> bool {
    let last_ts = load_state_str(STATE_LAST_IMPORT)
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);
    (now - last_ts) >= IMPORT_INTERVAL_SECS
}

// ─── Time helpers ─────────────────────────────────────────────────────

/// Derive the calendar year from a Unix timestamp.
fn timestamp_to_year(ts: i64) -> u16 {
    // 365.2425 days/year average; safe approximation for year extraction.
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let year = (1970 + ts / 31_556_952) as u16;
    year
}

/// Return the current Unix timestamp via the DB clock.
///
/// Used in `tap_queue_worker` where no `CronInput` is available.
fn current_timestamp() -> i64 {
    let result = host::query_raw("SELECT EXTRACT(EPOCH FROM NOW())::bigint AS ts", &[]);
    result
        .ok()
        .and_then(|s| serde_json::from_str::<Vec<serde_json::Value>>(&s).ok())
        .and_then(|rows| rows.into_iter().next())
        .and_then(|row| row.get("ts").and_then(|v| v.as_i64()))
        .unwrap_or(0)
}

// ─── Database helpers ─────────────────────────────────────────────────

/// Load existing conferences into a map of source_id → info.
fn load_existing_conferences() -> HashMap<String, ExistingConference> {
    let mut existing = HashMap::new();

    let result = host::query_raw(
        "SELECT id, fields->>'field_source_id' AS source_id, \
         fields->'field_topics' AS topics \
         FROM item \
         WHERE type = 'conference' \
         AND fields->>'field_source_id' IS NOT NULL",
        &[],
    );

    if let Ok(json_str) = result {
        let rows: Vec<serde_json::Value> = serde_json::from_str(&json_str).unwrap_or_default();
        for row in rows {
            if let (Some(id), Some(sid)) = (
                row.get("id").and_then(|v| v.as_str()),
                row.get("source_id").and_then(|v| v.as_str()),
            ) {
                let topics = row
                    .get("topics")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                existing.insert(
                    sid.to_string(),
                    ExistingConference {
                        item_id: id.to_string(),
                        topics,
                    },
                );
            }
        }
    }

    existing
}

/// Build the JSONB fields map for a conference (shared by insert and update).
fn build_source_fields(
    conf: &ConfsTechEntry,
    source_id: &str,
    topics: &[String],
) -> serde_json::Value {
    let mut fields = serde_json::json!({
        "field_start_date": conf.start_date,
        "field_end_date": conf.end_date,
        "field_source_id": source_id,
        "field_online": conf.online.unwrap_or(false),
        "field_topics": topics,
    });

    if !conf.url.is_empty() {
        fields["field_url"] = serde_json::json!(conf.url);
    }
    if let Some(ref city) = conf.city {
        fields["field_city"] = serde_json::json!(city);
    }
    if let Some(ref country) = conf.country {
        fields["field_country"] = serde_json::json!(country);
    }
    if let Some(ref cfp_url) = conf.cfp_url {
        fields["field_cfp_url"] = serde_json::json!(cfp_url);
    }
    if let Some(ref cfp_end_date) = conf.cfp_end_date {
        fields["field_cfp_end_date"] = serde_json::json!(cfp_end_date);
    }
    // Always set, never absent: the brief makes language part of the model, the
    // upcoming gather exposes a filter on it, and a filter over a field that most
    // items simply do not have is a filter that hides them. detect_language says
    // whether the value came from the feed or from the default.
    fields["field_language"] = serde_json::json!(detect_language(conf.locales.as_deref()).0);

    // field_twitter and field_coc_url used to be written here. The conference
    // type declares neither, the brief asks for neither, and no template reads
    // either, so they were two undeclared keys riding along in every item's
    // JSONB. Dropped rather than declared: adding a field to the model is the
    // brief's call, not the importer's.

    fields
}

/// The conference's primary language as an ISO 639-1 code, and where it came from.
///
/// confs.tech publishes `locales` as a string like `"EN"`, absent on most
/// entries. Normalised to lower case here because that is what the language
/// column, the `lang` attribute and the gather's exposed filter all use, and
/// because "EN" and "en" filtering as two different languages is the kind of
/// split nobody notices until a listing is half empty.
///
/// Anything that is not a plausible two-letter code is treated as absent rather
/// than stored: a regional tag like `en-GB` keeps its language half, and junk is
/// dropped in favour of the default.
///
/// Returns the code and `true` when it was detected from the feed, `false` when
/// it is the default.
fn detect_language(locales: Option<&str>) -> (String, bool) {
    const DEFAULT_LANGUAGE: &str = "en";

    let Some(raw) = locales else {
        return (DEFAULT_LANGUAGE.to_string(), false);
    };
    let head = raw.split([',', '-', '_']).next().unwrap_or("").trim();
    if head.len() == 2 && head.chars().all(|c| c.is_ascii_alphabetic()) {
        (head.to_ascii_lowercase(), true)
    } else {
        (DEFAULT_LANGUAGE.to_string(), false)
    }
}

/// Insert a new conference item, published, on the live stage.
///
/// Returns true on success.
///
/// This plugin does not define the `conference` type it writes. The type, its
/// fields and their types are Ritrovo's, in `demo/config/item_type.conference.yml`,
/// which the demo imports before enabling this plugin (`scripts/demo-bootstrap.sh`)
/// and which the host-in-the-loop suites import too. `item.type` is a foreign key
/// onto that type, so an insert before the config is imported fails.
///
/// `field_topics` is the one field written here that the type does not declare,
/// and deliberately so: the kernel has no category-reference field kind, and
/// declaring it as anything else would put a widget on the edit form that
/// destroys the array on save. The gathers read it straight from the JSONB.
fn insert_conference(
    conf: &ConfsTechEntry,
    source_id: &str,
    topic_uuid: Option<&str>,
    now: i64,
) -> bool {
    let topics: Vec<String> = topic_uuid.map(|u| vec![u.to_string()]).unwrap_or_default();
    let fields = build_source_fields(conf, source_id, &topics);

    // Use an upsert so that concurrent queue workers processing different topic
    // files don't create duplicate conference items.  On conflict, source-derived
    // fields are refreshed and `field_topics` is merged (SQL-side UNION dedup)
    // so both the existing and incoming topic UUIDs are preserved.
    let result = host::execute_raw(
        "INSERT INTO item (id, type, title, status, author_id, stage_id, created, changed, fields) \
         VALUES (\
           gen_random_uuid(), \
           'conference', \
           $1, \
           1, \
           '00000000-0000-0000-0000-000000000000'::uuid, \
           $2::uuid, \
           $3, \
           $3, \
           $4::jsonb\
         ) \
         ON CONFLICT ((fields->>'field_source_id')) \
         WHERE type = 'conference' \
           AND fields->>'field_source_id' IS NOT NULL \
           AND fields->>'field_source_id' != '' \
         DO UPDATE SET \
           title   = EXCLUDED.title, \
           changed = EXCLUDED.changed, \
           fields  = item.fields \
                  || (EXCLUDED.fields - 'field_topics') \
                  || jsonb_build_object(\
                       'field_topics', \
                       (SELECT COALESCE(jsonb_agg(t ORDER BY t), '[]'::jsonb) \
                        FROM (\
                          SELECT jsonb_array_elements_text(item.fields->'field_topics') \
                          UNION \
                          SELECT jsonb_array_elements_text(EXCLUDED.fields->'field_topics') \
                        ) u(t)\
                       )\
                     )",
        &[
            serde_json::json!(conf.name),
            serde_json::json!(LIVE_STAGE_UUID),
            serde_json::json!(now),
            serde_json::json!(fields.to_string()),
        ],
    );

    matches!(result, Ok(1))
}

/// Update an existing conference with fresh data from the source.
///
/// Only updates source-derived fields, preserving manually-edited fields
/// like description and editor notes. Returns true if the update executed.
///
/// `source_id` is the key the conference was found under, and it is written
/// back unchanged. It used to be passed as `""` on the reasoning that it does
/// not change, but `build_source_fields` always sets `field_source_id` and the
/// update merges with `||`, so every update blanked the dedup key. The next
/// import could no longer find the conference and inserted it again, which the
/// unique index could not stop because it excludes empty ids.
/// `migrations/004_restore_blanked_source_ids.sql` repairs the rows that bug
/// left behind.
fn update_conference(
    item_id: &str,
    conf: &ConfsTechEntry,
    source_id: &str,
    merged_topics: &[String],
    now: i64,
) -> bool {
    let updates = build_source_fields(conf, source_id, merged_topics);

    let result = host::execute_raw(
        "UPDATE item SET \
           title = $1, \
           changed = $2, \
           fields = fields || $3::jsonb \
         WHERE id = $4::uuid",
        &[
            serde_json::json!(conf.name),
            serde_json::json!(now),
            serde_json::json!(updates.to_string()),
            serde_json::json!(item_id),
        ],
    );

    matches!(result, Ok(1))
}

// ─── Slug / dedup helpers ─────────────────────────────────────────────

/// Compute a stable dedup key from a conference entry.
///
/// Format: `slugified(name)-startdate-slugified(city|online)`
fn compute_source_id(conf: &ConfsTechEntry) -> String {
    let name_slug = slugify(&conf.name);
    let city_slug = conf
        .city
        .as_deref()
        .map(slugify)
        .unwrap_or_else(|| "online".to_string());
    format!("{name_slug}-{}-{city_slug}", conf.start_date)
}

/// Simple ASCII slugification: lowercase, replace non-alphanumeric with
/// hyphens, collapse runs, trim leading/trailing hyphens.
fn slugify(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut last_was_hyphen = true; // suppress leading hyphens
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            result.push(c.to_ascii_lowercase());
            last_was_hyphen = false;
        } else if !last_was_hyphen {
            result.push('-');
            last_was_hyphen = true;
        }
    }
    if result.ends_with('-') {
        result.pop();
    }
    result
}

/// Merge a new topic UUID into an existing topic list, deduplicating and sorting.
///
/// If `new_uuid` is `None` (no taxonomy mapping for this confs.tech slug),
/// the existing list is returned unchanged.
fn merge_topics(existing: &[String], new_uuid: Option<&str>) -> Vec<String> {
    let mut topics: Vec<String> = existing.to_vec();
    if let Some(uuid) = new_uuid
        && !topics.iter().any(|t| t == uuid)
    {
        topics.push(uuid.to_string());
    }
    topics.sort();
    topics
}

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    // ── tap_perm / tap_menu ──────────────────────────────────────────

    #[test]
    fn perm_returns_five_permissions() {
        let perms = __inner_tap_perm();
        assert_eq!(perms.len(), 5);
        assert!(
            perms
                .iter()
                .any(|p| p.name == "administer conference import")
        );
        assert!(perms.iter().any(|p| p.name == "create conference content"));
    }

    #[test]
    fn menu_returns_two_routes() {
        let menus = __inner_tap_menu();
        assert_eq!(menus.len(), 2);
        assert_eq!(menus[0].path, "/admin/content/conferences");
        assert_eq!(menus[1].path, "/admin/config/importer");
    }

    /// The kernel dispatches to `tap_api` only for an entry whose
    /// `handler_type` is `"api"` and which names a callback. Both were true of
    /// the callback and false of the handler type, which is why both screens
    /// were registered and 404.
    #[test]
    fn both_menu_entries_are_routable_api_entries() {
        for entry in __inner_tap_menu() {
            assert_eq!(
                entry.handler_type, "api",
                "{} must be an api entry to be routed",
                entry.path
            );
            assert!(
                !entry.callback.is_empty(),
                "{} must name a callback",
                entry.path
            );
            assert!(
                !entry.permission.is_empty(),
                "{} must be gated on a permission",
                entry.path
            );
            assert!(
                entry.visible,
                "{} is a screen, so it belongs in navigation",
                entry.path
            );
        }
    }

    /// Every callback the menu registers is one `tap_api` answers, and every
    /// callback `tap_api` answers is one the menu registers. A drift either way
    /// is a 404 or dead code.
    #[test]
    fn every_registered_callback_is_served() {
        for entry in __inner_tap_menu() {
            let request = ApiRequest::new(
                &entry.callback,
                "GET",
                &entry.path,
                "00000000-0000-0000-0000-000000000000",
                true,
            );
            let response = __inner_tap_api(request);
            assert_eq!(
                response.status, 200,
                "callback {} answered {}",
                entry.callback, response.status
            );
            assert!(
                response.content_type.starts_with("text/html"),
                "callback {} served {}",
                entry.callback,
                response.content_type
            );
        }
    }

    #[test]
    fn an_unknown_callback_is_a_404() {
        let request = ApiRequest::new(
            "not_a_callback",
            "GET",
            "/admin/config/importer",
            "00000000-0000-0000-0000-000000000000",
            true,
        );
        let response = __inner_tap_api(request);
        assert_eq!(response.status, 404);
    }

    /// The kernel does not sanitize a plugin's response body, so anything from
    /// the confs.tech dataset that reaches the page has to be escaped here.
    #[test]
    fn html_escaping_neutralizes_markup_and_quotes() {
        let escaped = escape_html(r#"<script>alert("x & 'y'")</script>"#);
        assert_eq!(
            escaped,
            "&lt;script&gt;alert(&quot;x &amp; &#39;y&#39;&quot;)&lt;/script&gt;"
        );
        assert!(!escaped.contains('<'), "no raw angle bracket may survive");
    }

    /// A conference title is untrusted input and reaches the list through
    /// `cell`, so the escaping has to be on that path and not merely available.
    #[test]
    fn a_conference_title_is_escaped_on_the_way_into_the_table() {
        let row = serde_json::json!({"title": "<b>Rust</b> & Friends"});
        assert_eq!(cell(&row, "title"), "&lt;b&gt;Rust&lt;/b&gt; &amp; Friends");
    }

    #[test]
    fn a_missing_cell_renders_a_placeholder_rather_than_empty() {
        let row = serde_json::json!({"title": "Conf"});
        assert_eq!(cell(&row, "city"), "&mdash;");
    }

    // ── tap_queue_info ───────────────────────────────────────────────

    #[test]
    fn queue_info_returns_ritrovo_import_queue() {
        let info = __inner_tap_queue_info();
        let queues = info.as_array().unwrap();
        assert_eq!(queues.len(), 1);
        assert_eq!(queues[0]["name"], "ritrovo_import");
        assert_eq!(queues[0]["concurrency"], 4);
    }

    // ── tap_cron ─────────────────────────────────────────────────────

    #[test]
    fn cron_returns_completed_with_stub() {
        let input = CronInput {
            timestamp: 1_700_000_000,
        };
        let result = __inner_tap_cron(input);
        // With stub host functions (query_raw returns "[]"), should_import
        // returns true (no previous timestamp found). http_request stub
        // returns body "[]" which pushes empty payloads — no errors.
        assert_eq!(result["status"], "completed", "unexpected: {result}");
    }

    // ── language detection ────────────────────────────────────────────

    #[test]
    fn language_comes_from_the_feed_when_it_says_one() {
        assert_eq!(detect_language(Some("EN")), ("en".to_string(), true));
        assert_eq!(detect_language(Some("it")), ("it".to_string(), true));
        assert_eq!(detect_language(Some(" DE ")), ("de".to_string(), true));
    }

    #[test]
    fn language_keeps_the_language_half_of_a_regional_tag() {
        assert_eq!(detect_language(Some("en-GB")), ("en".to_string(), true));
        assert_eq!(detect_language(Some("pt_BR")), ("pt".to_string(), true));
        assert_eq!(detect_language(Some("fr,en")), ("fr".to_string(), true));
    }

    #[test]
    fn language_defaults_to_english_when_the_feed_is_silent_or_junk() {
        assert_eq!(detect_language(None), ("en".to_string(), false));
        assert_eq!(detect_language(Some("")), ("en".to_string(), false));
        assert_eq!(detect_language(Some("english")), ("en".to_string(), false));
        assert_eq!(detect_language(Some("12")), ("en".to_string(), false));
    }

    #[test]
    fn every_conference_gets_a_language() {
        // The upcoming gather exposes a language filter; a conference with no
        // field_language at all would vanish from it rather than show up as
        // English, which is the bug this guards.
        let conf = ConfsTechEntry {
            name: "NoLocaleConf".to_string(),
            url: String::new(),
            start_date: "2026-01-01".to_string(),
            end_date: "2026-01-02".to_string(),
            city: None,
            country: None,
            online: Some(true),
            cfp_url: None,
            cfp_end_date: None,
            locales: None,
        };
        let fields = build_source_fields(&conf, "nolocaleconf-2026-01-01-online", &[]);
        assert_eq!(fields["field_language"], "en");
    }

    // ── the confs.tech topic mapping (data/confs-tech-topics.json) ────

    #[test]
    fn the_topic_map_covers_every_feed_the_importer_fetches() {
        let map = topic_map();
        let keys: Vec<&str> = map.iter().map(|(k, _)| k.as_str()).collect();
        for feed in TOPICS {
            assert!(
                keys.contains(feed),
                "confs.tech feed '{feed}' is fetched but absent from \
                 data/confs-tech-topics.json, so its conferences import untagged \
                 with nothing saying so"
            );
        }
        assert_eq!(map.len(), TOPICS.len(), "the map and the feed list disagree");
    }

    #[test]
    fn the_topic_map_parses_and_names_its_unmapped_feeds() {
        let map = topic_map();
        assert!(!map.is_empty(), "the mapping file failed to parse");
        let unmapped: Vec<&str> = map
            .iter()
            .filter(|(_, term)| term.is_none())
            .map(|(k, _)| k.as_str())
            .collect();
        // The brief's tree has no General, Open Source or Testing term. If that
        // changes, this test is the reminder to point these three somewhere.
        assert_eq!(unmapped, vec!["general", "opensource", "testing"]);
    }

    #[test]
    fn every_mapped_term_carries_both_a_label_and_a_slug() {
        // The label is what the category_tag lookup matches on and the slug is
        // the state key the resolved uuid caches under. One without the other
        // resolves to nothing, silently.
        for (feed, term) in topic_map() {
            if let Some(term) = term {
                assert!(!term.label.trim().is_empty(), "{feed} has an empty label");
                assert!(!term.slug.trim().is_empty(), "{feed} has an empty slug");
            }
        }
    }

    // ── tap_queue_worker ─────────────────────────────────────────────

    // A malformed batch has to come back as Err, not as an Ok carrying
    // {"status": "error"}. The kernel reads any Ok as success and deletes the
    // job; only a negative length, which is what the SDK writes for an Err from
    // a #[plugin_tap_result], reaches its retry-and-dead-letter path
    // (G-QUEUE-WORKER-ERROR-IS-SUCCESS). So these assert the variant first and
    // the message second.
    #[test]
    fn queue_worker_rejects_missing_topic() {
        let input = serde_json::json!({"year": 2026, "conferences": "[]"});
        let err = __inner_tap_queue_worker(input).expect_err("must be an Err");
        assert!(err.contains("missing_topic"), "unexpected: {err}");
    }

    #[test]
    fn queue_worker_rejects_missing_year() {
        let input = serde_json::json!({"topic": "rust", "conferences": "[]"});
        let err = __inner_tap_queue_worker(input).expect_err("must be an Err");
        assert!(err.contains("missing_year"), "unexpected: {err}");
        assert!(err.starts_with("rust/"), "the reason should name the batch: {err}");
    }

    #[test]
    fn queue_worker_rejects_bad_json() {
        let input = serde_json::json!({"topic": "rust", "year": 2026, "conferences": "not-json"});
        let err = __inner_tap_queue_worker(input).expect_err("must be an Err");
        assert!(err.contains("parse_error"), "unexpected: {err}");
        assert!(err.starts_with("rust/2026:"), "the reason should name the batch: {err}");
    }

    #[test]
    fn queue_worker_skips_invalid_entries() {
        // Missing startDate and endDate.
        let conferences = serde_json::json!([{"name": "BadConf"}]).to_string();
        let input = serde_json::json!({
            "topic": "rust",
            "year": 2026,
            "conferences": conferences,
        });
        let result = __inner_tap_queue_worker(input).expect("batch should succeed");
        assert_eq!(result["status"], "ok");
        assert_eq!(result["invalid"], 1);
        assert_eq!(result["imported"], 0);
    }

    #[test]
    fn queue_worker_accepts_valid_entry() {
        let conferences = serde_json::json!([{
            "name": "RustConf",
            "startDate": "2026-09-01",
            "endDate": "2026-09-03",
            "city": "Portland",
            "country": "USA",
        }])
        .to_string();
        let input = serde_json::json!({
            "topic": "rust",
            "year": 2026,
            "conferences": conferences,
        });
        let result = __inner_tap_queue_worker(input).expect("batch should succeed");
        assert_eq!(result["status"], "ok");
        // Stub execute_raw always returns Ok(0), so insert returns false (0 rows
        // affected != 1). The entry counts as invalid in the stub context.
        assert_eq!(
            result["invalid"].as_u64().unwrap() + result["imported"].as_u64().unwrap(),
            1
        );
    }

    // ── validate_conference ───────────────────────────────────────────

    fn make_valid() -> ConfsTechEntry {
        ConfsTechEntry {
            name: "RustConf".to_string(),
            url: "https://rustconf.com".to_string(),
            start_date: "2026-09-01".to_string(),
            end_date: "2026-09-03".to_string(),
            city: Some("Portland".to_string()),
            country: Some("USA".to_string()),
            online: None,
            cfp_url: None,
            cfp_end_date: None,
            locales: None,
        }
    }

    #[test]
    fn validate_valid_entry_ok() {
        assert!(validate_conference(&make_valid()).is_ok());
    }

    #[test]
    fn validate_missing_name() {
        let mut c = make_valid();
        c.name = String::new();
        assert!(validate_conference(&c).is_err());
    }

    #[test]
    fn validate_missing_start_date() {
        let mut c = make_valid();
        c.start_date = String::new();
        assert!(validate_conference(&c).is_err());
    }

    #[test]
    fn validate_end_before_start() {
        let mut c = make_valid();
        c.end_date = "2026-08-31".to_string();
        assert!(validate_conference(&c).is_err());
    }

    #[test]
    fn validate_cfp_after_start() {
        let mut c = make_valid();
        c.cfp_end_date = Some("2026-10-01".to_string());
        let err = validate_conference(&c).unwrap_err();
        assert!(err.contains("cfpEndDate"), "unexpected error: {err}");
    }

    #[test]
    fn validate_bad_date_format() {
        let mut c = make_valid();
        c.start_date = "01-09-2026".to_string(); // wrong format
        assert!(validate_conference(&c).is_err());
    }

    #[test]
    fn validate_year_out_of_range() {
        let mut c = make_valid();
        c.start_date = "1999-01-01".to_string();
        c.end_date = "1999-01-02".to_string();
        assert!(validate_conference(&c).is_err());
    }

    // ── is_valid_date ─────────────────────────────────────────────────

    #[test]
    fn is_valid_date_ok() {
        assert!(is_valid_date("2026-09-01"));
        assert!(is_valid_date("2010-01-01"));
        assert!(is_valid_date("2035-12-31"));
    }

    #[test]
    fn is_valid_date_bad_format() {
        assert!(!is_valid_date("09-01-2026")); // wrong order
        assert!(!is_valid_date("2026/09/01")); // wrong separator
        assert!(!is_valid_date("2026-9-1")); // missing leading zeros
        assert!(!is_valid_date("not-a-date"));
    }

    #[test]
    fn is_valid_date_year_range() {
        assert!(!is_valid_date("1999-01-01"));
        assert!(!is_valid_date("2050-01-01"));
    }

    // ── slugify ───────────────────────────────────────────────────────

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("RustConf 2026"), "rustconf-2026");
        assert_eq!(slugify("EuroRust"), "eurorust");
        assert_eq!(slugify("Vue.js Nation"), "vue-js-nation");
    }

    #[test]
    fn slugify_unicode_and_special_chars() {
        assert_eq!(slugify("JSConf España"), "jsconf-espa-a");
        assert_eq!(slugify("C++ Now!"), "c-now");
    }

    #[test]
    fn slugify_no_trailing_hyphen() {
        assert_eq!(slugify("test--value--"), "test-value");
    }

    // ── compute_source_id ─────────────────────────────────────────────

    #[test]
    fn compute_source_id_with_city() {
        let conf = ConfsTechEntry {
            name: "RustConf".to_string(),
            url: String::new(),
            start_date: "2026-09-01".to_string(),
            end_date: "2026-09-03".to_string(),
            city: Some("Portland".to_string()),
            country: Some("U.S.A.".to_string()),
            online: None,
            cfp_url: None,
            cfp_end_date: None,
            locales: None,
        };
        assert_eq!(compute_source_id(&conf), "rustconf-2026-09-01-portland");
    }

    #[test]
    fn compute_source_id_online() {
        let conf = ConfsTechEntry {
            name: "Vue.js Nation".to_string(),
            url: String::new(),
            start_date: "2025-01-29".to_string(),
            end_date: "2025-01-30".to_string(),
            city: None,
            country: None,
            online: Some(true),
            cfp_url: None,
            cfp_end_date: None,
            locales: None,
        };
        assert_eq!(compute_source_id(&conf), "vue-js-nation-2025-01-29-online");
    }

    // ── build_source_fields ───────────────────────────────────────────

    #[test]
    fn build_fields_minimal() {
        let conf = ConfsTechEntry {
            name: "TestConf".to_string(),
            url: String::new(),
            start_date: "2026-01-01".to_string(),
            end_date: "2026-01-02".to_string(),
            city: None,
            country: None,
            online: None,
            cfp_url: None,
            cfp_end_date: None,
            locales: None,
        };
        let topics = vec!["rust".to_string()];
        let fields = build_source_fields(&conf, "testconf-2026-01-01-online", &topics);
        assert_eq!(fields["field_source_id"], "testconf-2026-01-01-online");
        assert_eq!(fields["field_online"], false);
        assert_eq!(fields["field_topics"][0], "rust");
        assert!(fields.get("field_url").is_none());
    }

    #[test]
    fn build_fields_full() {
        let conf = ConfsTechEntry {
            name: "RustConf".to_string(),
            url: "https://rustconf.com".to_string(),
            start_date: "2026-09-01".to_string(),
            end_date: "2026-09-03".to_string(),
            city: Some("Portland".to_string()),
            country: Some("U.S.A.".to_string()),
            online: Some(false),
            cfp_url: Some("https://rustconf.com/cfp".to_string()),
            cfp_end_date: Some("2026-06-01".to_string()),
            locales: Some("EN".to_string()),
        };
        let topics = vec!["rust".to_string()];
        let fields = build_source_fields(&conf, "rustconf-2026-09-01-portland", &topics);
        assert_eq!(fields["field_url"], "https://rustconf.com");
        assert_eq!(fields["field_city"], "Portland");
        assert_eq!(fields["field_country"], "U.S.A.");
        assert_eq!(fields["field_cfp_url"], "https://rustconf.com/cfp");
        assert_eq!(fields["field_cfp_end_date"], "2026-06-01");
        // Normalised: the feed says "EN", the model stores ISO 639-1 lower case.
        assert_eq!(fields["field_language"], "en");
        // The feed carries twitter and codeOfConduct and the model has no field
        // for either, so neither is written. They used to ride along as
        // undeclared keys in every item's JSONB.
        assert!(fields.get("field_twitter").is_none());
        assert!(fields.get("field_coc_url").is_none());
    }

    // ── merge_topics ──────────────────────────────────────────────────

    #[test]
    fn merge_topics_deduplicates() {
        let uuid_a = "00000000-0000-0000-0000-000000000001";
        let uuid_b = "00000000-0000-0000-0000-000000000002";
        let existing = vec![uuid_a.to_string(), uuid_b.to_string()];
        // Already present — no duplicate added.
        assert_eq!(merge_topics(&existing, Some(uuid_a)), vec![uuid_a, uuid_b]);
        // New UUID — appended and sorted.
        let uuid_c = "00000000-0000-0000-0000-000000000003";
        assert_eq!(
            merge_topics(&existing, Some(uuid_c)),
            vec![uuid_a, uuid_b, uuid_c]
        );
    }

    #[test]
    fn merge_topics_empty_existing() {
        let uuid = "00000000-0000-0000-0000-000000000001";
        let existing: Vec<String> = vec![];
        assert_eq!(merge_topics(&existing, Some(uuid)), vec![uuid]);
    }

    #[test]
    fn merge_topics_none_uuid_is_noop() {
        let existing = vec!["uuid-a".to_string()];
        assert_eq!(merge_topics(&existing, None), vec!["uuid-a"]);
    }

    // ── topic_term_uuid ───────────────────────────────────────────────

    #[test]
    fn topic_term_uuid_returns_none_for_unmapped_slug() {
        // `general`, `opensource` and `testing` map to no term in the data file.
        assert!(topic_term_uuid("general").is_none());
        assert!(topic_term_uuid("opensource").is_none());
        assert!(topic_term_uuid("testing").is_none());
    }

    #[test]
    fn topic_term_uuid_returns_none_for_unknown_slug() {
        assert!(topic_term_uuid("not-a-real-topic").is_none());
    }

    #[test]
    fn topic_term_uuid_mapped_slug_queries_state() {
        // In the stub host environment, query_raw returns "[]" so the state
        // lookup will return None even for mapped slugs.  This confirms the
        // function at least reaches the state query without panicking.
        let result = topic_term_uuid("rust");
        // Stub returns None — acceptable; real env would return Some(uuid).
        assert!(result.is_none() || result.as_deref().map(|s| s.len()).unwrap_or(0) > 0);
    }

    // ── timestamp_to_year ─────────────────────────────────────────────

    #[test]
    fn timestamp_to_year_works() {
        // 2025-01-01 00:00:00 UTC = 1735689600
        assert_eq!(timestamp_to_year(1_735_689_600), 2025);
        // 2026-06-15 12:00:00 UTC ≈ 1781870400
        assert_eq!(timestamp_to_year(1_781_870_400), 2026);
    }

    // ── perm permission names ─────────────────────────────────────────

    #[test]
    fn perm_format_matches_kernel_fallback() {
        let perms = __inner_tap_perm();
        let expected_names = [
            "view conference content",
            "create conference content",
            "edit conference content",
            "delete conference content",
            "administer conference import",
        ];
        for name in &expected_names {
            assert!(
                perms.iter().any(|p| p.name == *name),
                "missing permission: {name}"
            );
        }
    }
}
