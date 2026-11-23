//! The conference assistant: `ritrovo_cfp`'s scope in the kernel's AI assistant
//! (STATUS 31.8).
//!
//! An editor opens it from a conference page (the kernel puts the launcher on
//! every item of a type an item scope names) and talks to Ritrovo's chat model
//! about that conference. The model can call three tools, all of them this
//! plugin's:
//!
//! - `find_conferences`, a read: upcoming conferences by topic and by city.
//! - `cfps_closing_soon`, a read: open calls for papers closing within some
//!   days, soonest first.
//! - `propose_cfp_end_date`, the one write: a corrected CFP end date for the
//!   conference the conversation is about.
//!
//! The kernel runs a read as soon as the model calls it. It never runs a write
//! on the model's say-so: it asks this plugin to *describe* the change, stores
//! that description as a proposal, and only once the editor presses Apply does
//! it call the tool again to *execute* it. So `Describe` here changes nothing,
//! by contract, and `Execute` is reached through a person's click alone.
//!
//! Both reads go through the kernel's `query` host interface, the gather
//! pipeline every listing on the site uses: they read for the editor, on the
//! Live stage, with item and field access applied. The write goes through
//! `item-api`'s `save_item`, which is the kernel's item service, so the edit
//! is a revision with a log message and every other plugin's taps fire, exactly
//! as an edit from the form would.

use trovato_sdk::host;
use trovato_sdk::prelude::*;

use crate::{DAY, date_complaint, date_to_days};

/// The scope's machine name, as the assistant's URLs carry it:
/// `/ai/assistant/ritrovo_conference/{conference id}`.
pub const SCOPE: &str = "ritrovo_conference";

/// Who may open the scope: editors and publishers hold it, a member does not.
/// It is also the permission an edit of somebody else's conference needs, which
/// is what the one write tool does.
pub const PERMISSION: &str = "edit any content";

/// Rows a read tool returns at most. Enough to answer a question, small enough
/// that a tool result stays well inside the kernel's 16 KiB cap.
const MAX_ROWS: u32 = 10;

/// The longest window `cfps_closing_soon` accepts, in days.
const MAX_DAYS: i64 = 90;

/// The scope declaration.
pub fn scopes() -> Vec<AssistantScope> {
    vec![
        AssistantScope::new(
            SCOPE,
            "Conference assistant",
            PERMISSION,
            AssistantIdKind::Item,
        )
        .description(
            "Look up conferences and calls for papers, and propose a corrected \
             CFP end date for this conference.",
        )
        .item_types(["conference"])
        .prompt(
            "You help a Ritrovo editor look after one conference, described in \
             the snapshot. Use find_conferences to look conferences up by topic \
             or city, and cfps_closing_soon for calls for papers that close soon. \
             Answer from what the tools return, never from memory, and name each \
             conference by its exact title. When the editor asks to correct this \
             conference's CFP end date, call propose_cfp_end_date once with the \
             date as YYYY-MM-DD: that only proposes the change, and the editor \
             applies it. A CFP cannot close after the conference ends.",
        )
        .suggestions([
            "Which Rust conferences are coming up?",
            "Which calls for papers close in the next two weeks?",
            "The CFP for this conference actually closes a week later",
        ])
        .tool(
            AssistantTool::read(
                "find_conferences",
                "Upcoming conferences on the site, soonest first, filtered by topic, \
                 by city, or both. Matching is by substring, ignoring case.",
            )
            .parameters(serde_json::json!({
                "type": "object",
                "properties": {
                    "topic": {
                        "type": "string",
                        "description": "A topic name, such as Rust, JavaScript or DevOps"
                    },
                    "city": {
                        "type": "string",
                        "description": "A city name, such as Berlin"
                    }
                }
            })),
        )
        .tool(
            AssistantTool::read(
                "cfps_closing_soon",
                "Open calls for papers that close within the next `days` days \
                 (default 14, at most 90), soonest first.",
            )
            .parameters(serde_json::json!({
                "type": "object",
                "properties": {
                    "days": {"type": "integer", "description": "How many days ahead to look"}
                }
            })),
        )
        .tool(
            AssistantTool::write(
                "propose_cfp_end_date",
                "Propose a corrected CFP end date for the conference this \
                 conversation is about. The editor reviews and applies it.",
                AssistantRisk::Normal,
            )
            .parameters(serde_json::json!({
                "type": "object",
                "required": ["date"],
                "properties": {
                    "date": {"type": "string", "description": "The corrected date, YYYY-MM-DD"},
                    "reason": {"type": "string", "description": "Why the date is wrong, in a sentence"}
                }
            })),
        ),
    ]
}

/// What the model is told about the conference before the first message.
pub fn context(request: &AssistantContextRequest) -> AssistantContext {
    let id = request.scope_id.clone().unwrap_or_default();
    let link = format!("/item/{id}");
    match host::get_item(&id) {
        Ok(Some(item)) => {
            AssistantContext::new(item.title.clone(), snapshot(&item)).link("View conference", link)
        }
        // The kernel has already checked the item exists and that this editor
        // may view it; reaching here means it changed in between. Say so rather
        // than invent a snapshot.
        _ => AssistantContext::new(
            "Conference",
            format!("Conference {id} could not be loaded. Tell the editor."),
        )
        .link("View conference", link),
    }
}

/// One conference, as plain lines for the model.
fn snapshot(item: &Item) -> String {
    let text = |key: &str| field_text(item.fields.get(key)).unwrap_or_else(|| "(not set)".into());
    let mut lines = vec![
        format!("Conference: {}", item.title),
        format!("Id: {}", item.id),
        format!(
            "Dates: {} to {}",
            text("field_start_date"),
            text("field_end_date")
        ),
        format!("City: {}, {}", text("field_city"), text("field_country")),
        format!("Website: {}", text("field_url")),
        format!("CFP page: {}", text("field_cfp_url")),
        format!("CFP end date: {}", text("field_cfp_end_date")),
    ];
    if let Some(complaint) = date_complaint(
        field_text(item.fields.get("field_cfp_end_date")).as_deref(),
        field_text(item.fields.get("field_end_date")).as_deref(),
    ) {
        lines.push(format!("Problem: {complaint}"));
    }
    lines.join("\n")
}

/// Answer one tool call.
pub fn tool(call: &AssistantToolCall) -> AssistantToolResult {
    // The kernel checked the scope's permission before it opened the
    // conversation. This checks again at the moment of the call, because that
    // is where the reading and the change happen.
    if !host::current_user_has_permission(PERMISSION) {
        return AssistantToolResult::failed(
            "You need the edit any content permission to use the conference assistant.",
        );
    }
    match call.tool.as_str() {
        "find_conferences" => find_conferences(&call.arguments),
        "cfps_closing_soon" => cfps_closing_soon(&call.arguments),
        "propose_cfp_end_date" => propose_cfp_end_date(call),
        other => AssistantToolResult::failed(format!("no such tool: {other}")),
    }
}

// ─── The reads ───────────────────────────────────────────────────────

/// Upcoming conferences by topic, city, or both.
fn find_conferences(arguments: &serde_json::Value) -> AssistantToolResult {
    let topic = non_empty(arguments, "topic");
    let city = non_empty(arguments, "city");
    if topic.is_none() && city.is_none() {
        return AssistantToolResult::failed("give a topic, a city, or both");
    }
    run(&find_definition(topic.as_deref(), city.as_deref()))
}

/// The gather `find_conferences` runs.
///
/// The Live, upcoming conferences, soonest first: the `ritrovo.by_topic` and
/// `ritrovo.by_city` pages' filters, matched by substring on the indexed topic
/// labels (`field_topic_names`, which `ritrovo_importer` writes) and the city,
/// because a model names a topic by its label and not by its term id.
pub fn find_definition(topic: Option<&str>, city: Option<&str>) -> serde_json::Value {
    let mut filters = vec![upcoming_filter()];
    if let Some(topic) = topic {
        filters.push(serde_json::json!({
            "field": "fields.field_topic_names", "operator": "contains", "value": topic
        }));
    }
    if let Some(city) = city {
        filters.push(serde_json::json!({
            "field": "fields.field_city", "operator": "contains", "value": city
        }));
    }
    conference_definition(filters, "fields.field_start_date")
}

/// Open CFPs closing within `days`.
fn cfps_closing_soon(arguments: &serde_json::Value) -> AssistantToolResult {
    let days = arguments
        .get("days")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(14);
    if !(1..=MAX_DAYS).contains(&days) {
        return AssistantToolResult::failed(format!("days must be from 1 to {MAX_DAYS}"));
    }
    let Some(today) = crate::current_timestamp().map(|now| now.div_euclid(DAY)) else {
        return AssistantToolResult::failed("the database clock could not be read");
    };
    run(&closing_definition(&days_to_date(today + days)))
}

/// The gather `cfps_closing_soon` runs: `ritrovo.cfps_closing_soon`'s filters,
/// with an upper bound.
///
/// The bound is a date this plugin works out, because a gather filter can name
/// today and no other day (FRICTION.md, G-NO-RELATIVE-DATE-FILTER-VALUES).
pub fn closing_definition(last_day: &str) -> serde_json::Value {
    conference_definition(
        vec![
            serde_json::json!({
                "field": "fields.field_cfp_end_date", "operator": "greater_or_equal",
                "value": "current_date"
            }),
            serde_json::json!({
                "field": "fields.field_cfp_end_date", "operator": "less_or_equal",
                "value": last_day
            }),
            serde_json::json!({
                "field": "fields.field_cfp_url", "operator": "is_not_null", "value": null
            }),
        ],
        "fields.field_cfp_end_date",
    )
}

fn upcoming_filter() -> serde_json::Value {
    serde_json::json!({
        "field": "fields.field_start_date", "operator": "greater_or_equal",
        "value": "current_date"
    })
}

fn conference_definition(filters: Vec<serde_json::Value>, sort: &str) -> serde_json::Value {
    serde_json::json!({
        "base_table": "item",
        "item_type": "conference",
        "fields": [],
        "filters": filters,
        "sorts": [{"field": sort, "direction": "asc"}],
        "stage_aware": true,
    })
}

/// Run a definition and hand the model a compact list.
fn run(definition: &serde_json::Value) -> AssistantToolResult {
    match host::query(definition, &serde_json::json!({"per_page": MAX_ROWS})) {
        Ok(result) => {
            let rows: Vec<serde_json::Value> = result.items.iter().map(compact).collect();
            AssistantToolResult::data(
                serde_json::json!({"total": result.total, "conferences": rows}).to_string(),
            )
        }
        Err(code) => {
            host::log(
                "warn",
                "ritrovo_cfp",
                &format!("assistant query failed with host error {code}"),
            );
            AssistantToolResult::failed(format!("the search failed (host error {code})"))
        }
    }
}

/// One gather row as the model needs it: what to call it, where it is, when,
/// and the address to link.
fn compact(row: &serde_json::Value) -> serde_json::Value {
    let field = |key: &str| {
        row.get("fields")
            .and_then(|fields| field_text(fields.get(key)))
            .or_else(|| field_text(row.get(key)))
    };
    serde_json::json!({
        "title": row.get("title"),
        "url": row.get("url"),
        "city": field("field_city"),
        "country": field("field_country"),
        "start_date": field("field_start_date"),
        "end_date": field("field_end_date"),
        "cfp_end_date": field("field_cfp_end_date"),
        "topics": field("field_topic_names"),
    })
}

// ─── The write ───────────────────────────────────────────────────────

/// Propose, or once applied make, a corrected CFP end date.
fn propose_cfp_end_date(call: &AssistantToolCall) -> AssistantToolResult {
    let Some(date) = non_empty(&call.arguments, "date") else {
        return AssistantToolResult::failed("propose_cfp_end_date needs a `date`, YYYY-MM-DD");
    };
    if date_to_days(&date).is_none() {
        return AssistantToolResult::failed(format!("{date} is not a date in the form YYYY-MM-DD"));
    }
    let reason = non_empty(&call.arguments, "reason");
    let Some(id) = call.scope_id.as_deref() else {
        return AssistantToolResult::failed("this conversation is not about a conference");
    };
    let item = match host::get_item(id) {
        Ok(Some(item)) => item,
        _ => return AssistantToolResult::failed("the conference could not be loaded"),
    };
    let current = field_text(item.fields.get("field_cfp_end_date"));
    let end = field_text(item.fields.get("field_end_date"));
    // The date rule this plugin owns: refused here, before a proposal exists,
    // so an editor is never offered a change the site would complain about.
    if let Some(complaint) = date_complaint(Some(date.as_str()), end.as_deref()) {
        return AssistantToolResult::failed(complaint);
    }
    let summary = change_summary(&item.title, current.as_deref(), &date);

    match call.mode {
        // Describe changes nothing: what the editor reads on the proposal card
        // is produced without the change happening.
        AssistantToolMode::Describe => {
            let because = reason.map(|r| format!(" Reason: {r}")).unwrap_or_default();
            AssistantToolResult::ok(format!("Proposed: {summary}.{because}"), summary)
        }
        AssistantToolMode::Execute => {
            let mut fields = item.fields.clone();
            fields.insert(
                "field_cfp_end_date".to_string(),
                serde_json::Value::String(date.clone()),
            );
            let log = match &reason {
                Some(reason) => format!("CFP end date corrected in the assistant: {reason}"),
                None => "CFP end date corrected in the assistant".to_string(),
            };
            match host::save_item(&serde_json::json!({
                "id": item.id.to_string(),
                "fields": fields,
                "log": log,
            })) {
                Ok(Some(_)) => AssistantToolResult::ok(
                    format!("Done: {summary}."),
                    format!("CFP end date of {} is now {date}", item.title),
                ),
                Ok(None) => AssistantToolResult::failed("the conference no longer exists"),
                Err(code) => AssistantToolResult::failed(format!(
                    "the conference could not be saved (host error {code})"
                )),
            }
        }
    }
}

/// The one line a proposal card shows.
pub fn change_summary(title: &str, current: Option<&str>, date: &str) -> String {
    match current {
        Some(current) => format!("Change the CFP end date of {title} from {current} to {date}"),
        None => format!("Set the CFP end date of {title} to {date}"),
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────

/// A stored field's text, whether it is a bare string or `{"value": ...}`.
fn field_text(value: Option<&serde_json::Value>) -> Option<String> {
    let value = value?;
    let text = value.get("value").unwrap_or(value).as_str()?.trim();
    (!text.is_empty()).then(|| text.to_string())
}

/// A string argument, trimmed, if it has any text.
fn non_empty(arguments: &serde_json::Value, key: &str) -> Option<String> {
    let text = arguments.get(key)?.as_str()?.trim();
    (!text.is_empty()).then(|| text.to_string())
}

/// `YYYY-MM-DD` for days since 1970-01-01: Howard Hinnant's `civil_from_days`,
/// the inverse of [`date_to_days`].
pub fn days_to_date(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn days_to_date_inverts_date_to_days() {
        for date in [
            "1970-01-01",
            "2000-02-29",
            "2000-03-01",
            "2024-02-29",
            "2026-12-31",
            "2027-01-01",
        ] {
            assert_eq!(days_to_date(date_to_days(date).unwrap()), date);
        }
    }

    #[test]
    fn the_scope_is_one_item_scope_on_conference_for_editors() {
        let scopes = scopes();
        assert_eq!(scopes.len(), 1);
        let json = serde_json::to_value(&scopes[0]).unwrap();
        assert_eq!(json["name"], SCOPE);
        assert_eq!(json["permission"], PERMISSION);
        assert_eq!(json["id_kind"], "item");
        assert_eq!(json["item_types"], serde_json::json!(["conference"]));
        let tools: Vec<(String, String)> = json["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| {
                (
                    t["name"].as_str().unwrap().to_string(),
                    t["kind"].as_str().unwrap().to_string(),
                )
            })
            .collect();
        assert_eq!(
            tools,
            [
                ("find_conferences".into(), "read".into()),
                ("cfps_closing_soon".into(), "read".into()),
                ("propose_cfp_end_date".into(), "write".into()),
            ]
        );
        for tool in json["tools"].as_array().unwrap() {
            assert_eq!(tool["parameters"]["type"], "object", "{tool}");
        }
    }

    #[test]
    fn find_filters_by_topic_and_city_on_upcoming_live_conferences() {
        let def = find_definition(Some("Rust"), Some("Berlin"));
        assert_eq!(def["item_type"], "conference");
        assert_eq!(def["stage_aware"], true);
        let filters = def["filters"].as_array().unwrap();
        assert_eq!(filters.len(), 3);
        assert_eq!(filters[0]["value"], "current_date");
        assert_eq!(filters[1]["field"], "fields.field_topic_names");
        assert_eq!(filters[1]["value"], "Rust");
        assert_eq!(filters[2]["field"], "fields.field_city");
        assert_eq!(filters[2]["value"], "Berlin");
        assert_eq!(
            find_definition(None, Some("Rome"))["filters"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn closing_soon_is_bounded_by_today_and_the_last_day() {
        let def = closing_definition("2027-01-15");
        let filters = def["filters"].as_array().unwrap();
        assert_eq!(filters[0]["value"], "current_date");
        assert_eq!(filters[1]["operator"], "less_or_equal");
        assert_eq!(filters[1]["value"], "2027-01-15");
        assert_eq!(def["sorts"][0]["field"], "fields.field_cfp_end_date");
    }

    #[test]
    fn a_summary_names_the_conference_and_both_dates() {
        assert_eq!(
            change_summary("RustFest", Some("2027-01-10"), "2027-01-17"),
            "Change the CFP end date of RustFest from 2027-01-10 to 2027-01-17"
        );
        assert_eq!(
            change_summary("RustFest", None, "2027-01-17"),
            "Set the CFP end date of RustFest to 2027-01-17"
        );
    }

    #[test]
    fn compact_reads_both_row_shapes() {
        let nested = compact(&serde_json::json!({
            "title": "A", "url": "/a",
            "fields": {"field_city": {"value": "Rome"}, "field_start_date": "2027-03-01"}
        }));
        assert_eq!(nested["city"], "Rome");
        assert_eq!(nested["start_date"], "2027-03-01");
        assert_eq!(nested["url"], "/a");
        let flat = compact(&serde_json::json!({"title": "B", "field_city": "Milan"}));
        assert_eq!(flat["city"], "Milan");
    }
}
