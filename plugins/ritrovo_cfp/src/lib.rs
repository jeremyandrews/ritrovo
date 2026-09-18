//! CFP (Call for Papers) deadline badge plugin for Ritrovo conferences.
//!
//! Displays countdown badges on conference items when a CFP deadline is set:
//! - Green badge: more than 14 days remaining
//! - Yellow badge: 8 to 14 days remaining
//! - Red badge: 7 days or fewer remaining
//! - No badge: deadline has passed
//!
//! "Remaining" is measured against the database clock, read at render time. It
//! used to be measured against the item's `changed` timestamp, on the grounds
//! that plugins have no clock, which made the badge describe the day the record
//! was last saved rather than today: a conference untouched for months kept
//! counting down from that day, and a CFP that closed long ago still read open.

use trovato_sdk::host;
use trovato_sdk::prelude::*;

const DAY: i64 = 86_400;

/// Render a CFP badge when viewing a conference with an active CFP deadline.
///
/// The clock is read only once a deadline is known to exist, so a conference
/// without one costs no query. If the clock cannot be read, no badge renders:
/// a missing badge is a smaller fault than one that states the wrong date.
#[plugin_tap]
pub fn tap_item_view(item: Item) -> String {
    if item.item_type != "conference" || cfp_deadline(&item).is_none() {
        return String::new();
    }
    match current_timestamp() {
        Some(now) => render_badge(&item, now),
        None => {
            host::log(
                "warn",
                "ritrovo_cfp",
                "database clock unavailable; rendering no CFP badge",
            );
            String::new()
        }
    }
}

/// The badge for `item` as of the Unix timestamp `now`.
///
/// Pure, so the countdown can be tested at any instant. `item.changed` plays no
/// part in it.
fn render_badge(item: &Item, now: i64) -> String {
    if item.item_type != "conference" {
        return String::new();
    }
    let Some(deadline) = cfp_deadline(item) else {
        return String::new();
    };

    let remaining_secs = deadline - now;
    if remaining_secs <= 0 {
        return String::new();
    }

    let remaining_days = remaining_secs / DAY;

    let (color_class, label) = if remaining_days > 14 {
        (
            "cfp-badge--open",
            format!("CFP Open \u{2014} {remaining_days} days left"),
        )
    } else if remaining_days > 7 {
        (
            "cfp-badge--closing",
            format!("CFP Closing \u{2014} {remaining_days} days left"),
        )
    } else if remaining_days > 1 {
        (
            "cfp-badge--urgent",
            format!("CFP Urgent \u{2014} {remaining_days} days left"),
        )
    } else if remaining_days == 1 {
        (
            "cfp-badge--urgent",
            "CFP Urgent \u{2014} 1 day left".to_string(),
        )
    } else {
        ("cfp-badge--urgent", "CFP Closes Today!".to_string())
    };

    format!(r#"<span class="cfp-badge {color_class}">{label}</span>"#,)
}

// ─── The date rule ───────────────────────────────────────────────────

/// Check a conference's dates, returning the complaint if there is one.
///
/// The rule is the brief's: **a call for papers cannot close after the
/// conference it is calling for has ended.** A conference whose CFP closes the
/// day it starts is odd and legal; one whose CFP closes after the last day is a
/// typo, every time.
///
/// Pure, and takes the two dates rather than an `Item`, so the same rule can be
/// applied to a form submission that has no Item yet. Both sides are read the
/// way the kernel stores a `Date` field, `YYYY-MM-DD`, and compared as days:
/// closing *on* the final day is allowed, closing the day after is not.
///
/// `None` when there is nothing to complain about, which includes every case
/// where a date is missing or unreadable. A missing date is not this rule's
/// business, and refusing to parse something is not the same as finding it
/// wrong.
pub fn date_complaint(cfp_end_date: Option<&str>, end_date: Option<&str>) -> Option<String> {
    let (cfp_text, end_text) = (cfp_end_date?, end_date?);
    let cfp = date_to_days(cfp_text)?;
    let end = date_to_days(end_text)?;
    (cfp > end).then(|| {
        format!(
            "The call for papers closes on {cfp_text}, after the conference ends \
             on {end_text}. A CFP cannot close after the event it is for."
        )
    })
}

/// Report a conference whose dates contradict each other.
///
/// # What this tap cannot do
///
/// **It cannot refuse the save.** The kernel dispatches `tap_item_presave`,
/// reads a `fields` object out of whatever comes back, merges it, and then saves
/// unconditionally: there is no error path and no veto, for create or for
/// update (`ItemService::create` and `::update` at the pinned release). Nor is
/// there any way to put a message in front of the editor who typed the dates —
/// `tap_form_validate` exists and is dispatched by a `FormService` that no route
/// calls. So the brief's "validation error returned on save" is not available
/// here. See FRICTION.md, `G-PRESAVE-CANNOT-REFUSE` and
/// `G-FORM-TAPS-UNREACHABLE`.
///
/// # What it does instead, and why not more
///
/// It logs, naming the conference and both dates, and changes nothing.
///
/// Clamping the CFP date to the end date was the other candidate and is worse:
/// the dates are each individually usable, so the plugin does not know which of
/// the two the editor mistyped, and silently rewriting one of them would turn a
/// visible contradiction into an invisible wrong answer. A record that is wrong
/// and says so beats a record that is wrong and looks fine.
///
/// **Where the rule is actually enforced** is the submission form Ritrovo serves
/// itself, which refuses the step before anything reaches the kernel. A rule can
/// only be enforced where something is allowed to say no.
#[plugin_tap]
pub fn tap_item_presave(input: serde_json::Value) -> serde_json::Value {
    let is_conference = input
        .get("item_type")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|t| t == "conference");
    if !is_conference {
        return serde_json::json!({});
    }

    let fields = input.get("fields");
    let read = |key: &str| -> Option<String> {
        fields?
            .get(key)
            .and_then(|v| v.get("value").unwrap_or(v).as_str())
            .map(str::to_string)
    };

    if let Some(complaint) = date_complaint(
        read("field_cfp_end_date").as_deref(),
        read("field_end_date").as_deref(),
    ) {
        let title = input
            .get("title")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("(untitled)");
        host::log(
            "warn",
            "ritrovo_cfp",
            &format!("saving \"{title}\" anyway: {complaint}"),
        );
    }

    // Nothing modified. An empty object is the "no changes" answer: the kernel
    // merges the `fields` it finds, and there are none.
    serde_json::json!({})
}

/// The instant the CFP closes, as a Unix timestamp, or `None` if the item has
/// no usable deadline.
///
/// `ritrovo_importer` stores `field_cfp_end_date` as a `YYYY-MM-DD` string (it is
/// a `FieldType::Date`), and a CFP that closes on a date is open through the end
/// of that day, so a date resolves to the following midnight, UTC. A bare Unix
/// timestamp, as a number or a numeric string, is taken as the instant itself.
fn cfp_deadline(item: &Item) -> Option<i64> {
    let deadline = match item.fields.get("field_cfp_end_date")? {
        serde_json::Value::Number(n) => n.as_i64()?,
        serde_json::Value::String(s) => match s.parse::<i64>() {
            Ok(ts) => ts,
            Err(_) => date_to_days(s)?.checked_add(1)?.checked_mul(DAY)?,
        },
        _ => return None,
    };
    (deadline != 0).then_some(deadline)
}

/// Days since 1970-01-01 for a `YYYY-MM-DD` date, or `None` if it is not one.
///
/// Howard Hinnant's `days_from_civil`, proleptic Gregorian.
fn date_to_days(date: &str) -> Option<i64> {
    let mut parts = date.splitn(3, '-');
    let (y, m, d) = (parts.next()?, parts.next()?, parts.next()?);
    if y.len() != 4 || m.len() != 2 || d.len() != 2 {
        return None;
    }
    let (y, m, d) = (
        y.parse::<i64>().ok()?,
        m.parse::<i64>().ok()?,
        d.parse::<i64>().ok()?,
    );
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

/// The current Unix timestamp from the database clock, or `None` if it cannot
/// be read.
///
/// The same query `ritrovo_importer` uses: the kernel links no WASI clock into a
/// plugin, so the database is the only clock a plugin can reach. Unlike the
/// importer's copy this does not fall back to 0, because a badge computed from
/// the epoch would show every deadline as decades away.
fn current_timestamp() -> Option<i64> {
    let result = host::query_raw("SELECT EXTRACT(EPOCH FROM NOW())::bigint AS ts", &[]);
    result
        .ok()
        .and_then(|s| serde_json::from_str::<Vec<serde_json::Value>>(&s).ok())
        .and_then(|rows| rows.into_iter().next())
        .and_then(|row| row.get("ts").and_then(|v| v.as_i64()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// 2026-09-17T12:00:00Z, a fixed "now" for every countdown test.
    const NOW: i64 = 1_789_646_400;

    fn make_item(item_type: &str, cfp_end: Option<serde_json::Value>, changed: i64) -> Item {
        let mut fields = HashMap::new();
        if let Some(cfp_end) = cfp_end {
            fields.insert("field_cfp_end_date".to_string(), cfp_end);
        }
        Item {
            id: Uuid::nil(),
            item_type: item_type.to_string(),
            title: "Test Conf".to_string(),
            fields,
            status: 1,
            author_id: Uuid::nil(),
            current_revision_id: None,
            stage_id: live_stage_id(),
            created: 0,
            changed,
            language: None,
        }
    }

    fn make_conference(cfp_end: i64, changed: i64) -> Item {
        make_item("conference", Some(serde_json::json!(cfp_end)), changed)
    }

    #[test]
    fn fixed_now_is_the_instant_it_names() {
        assert_eq!(date_to_days("2026-09-17").unwrap() * DAY + 12 * 3600, NOW);
    }

    #[test]
    fn no_badge_for_non_conference() {
        let item = make_item("blog", Some(serde_json::json!(NOW + 20 * DAY)), NOW);
        assert!(render_badge(&item, NOW).is_empty());
        assert!(__inner_tap_item_view(item).is_empty());
    }

    #[test]
    fn no_badge_when_no_cfp_date() {
        let item = make_item("conference", None, NOW);
        assert!(render_badge(&item, NOW).is_empty());
        assert!(__inner_tap_item_view(item).is_empty());
    }

    #[test]
    fn green_badge_more_than_14_days() {
        let result = render_badge(&make_conference(NOW + 20 * DAY, NOW), NOW);
        assert!(result.contains("cfp-badge--open"));
        assert!(result.contains("20 days left"));
    }

    #[test]
    fn yellow_badge_7_to_14_days() {
        let result = render_badge(&make_conference(NOW + 10 * DAY, NOW), NOW);
        assert!(result.contains("cfp-badge--closing"));
    }

    #[test]
    fn red_badge_less_than_7_days() {
        let result = render_badge(&make_conference(NOW + 3 * DAY, NOW), NOW);
        assert!(result.contains("cfp-badge--urgent"));
    }

    #[test]
    fn no_badge_past_deadline() {
        assert!(render_badge(&make_conference(NOW - DAY, NOW), NOW).is_empty());
    }

    // Regression: the countdown was measured from `item.changed`, so a record
    // saved 200 days before its CFP closed still showed "CFP Open" a month after
    // it had closed.
    #[test]
    fn stale_changed_does_not_reopen_a_passed_cfp() {
        let deadline = NOW - 30 * DAY;
        let item = make_conference(deadline, deadline - 200 * DAY);
        assert!(
            render_badge(&item, NOW).is_empty(),
            "a CFP that closed 30 days ago rendered a badge"
        );
    }

    // Regression: the same fault from the other side. Three days out is urgent
    // however long ago, or how recently, the record was saved.
    #[test]
    fn three_days_ahead_is_urgent_whatever_changed_says() {
        for changed in [0, NOW - 400 * DAY, NOW - 3 * DAY, NOW, NOW + 90 * DAY] {
            let result = render_badge(&make_conference(NOW + 3 * DAY, changed), NOW);
            assert!(
                result.contains("cfp-badge--urgent") && result.contains("3 days left"),
                "changed = {changed} rendered {result:?}"
            );
        }
    }

    // The tap must not fall back to `changed` when there is no clock. Natively
    // the SDK's `query_raw` stub returns no rows, which is exactly that case.
    #[test]
    fn tap_renders_no_badge_without_a_clock() {
        let item = make_conference(NOW + 3 * DAY, NOW);
        assert!(__inner_tap_item_view(item).is_empty());
    }

    // What the importer actually writes: a Date field, as `YYYY-MM-DD`. The old
    // parse accepted only integers, so no imported conference ever had a badge.
    #[test]
    fn date_string_deadline_counts_to_the_end_of_that_day() {
        let date = |s: &str| make_item("conference", Some(serde_json::json!(s)), 0);

        let urgent = render_badge(&date("2026-09-20"), NOW);
        assert!(urgent.contains("cfp-badge--urgent"), "{urgent:?}");
        assert!(urgent.contains("3 days left"), "{urgent:?}");

        let open = render_badge(&date("2026-10-17"), NOW);
        assert!(open.contains("cfp-badge--open"), "{open:?}");

        assert_eq!(
            render_badge(&date("2026-09-17"), NOW),
            r#"<span class="cfp-badge cfp-badge--urgent">CFP Closes Today!</span>"#
        );
        assert!(render_badge(&date("2026-09-18"), NOW).contains("1 day left"));
        assert!(render_badge(&date("2026-09-16"), NOW).is_empty());
    }

    #[test]
    fn malformed_deadlines_render_no_badge() {
        for bad in ["", "soon", "2026-13-01", "2026-9-17", "0"] {
            let item = make_item("conference", Some(serde_json::json!(bad)), NOW);
            assert!(render_badge(&item, NOW).is_empty(), "{bad:?} rendered");
        }
    }

    // ─── The date rule ───────────────────────────────────────────

    #[test]
    fn a_cfp_closing_before_the_end_is_fine() {
        assert_eq!(date_complaint(Some("2027-03-01"), Some("2027-05-03")), None);
    }

    #[test]
    fn a_cfp_closing_on_the_final_day_is_fine() {
        // The boundary is inclusive on purpose: a CFP that closes as the
        // conference ends is unusual and it is not a mistake.
        assert_eq!(date_complaint(Some("2027-05-03"), Some("2027-05-03")), None);
    }

    #[test]
    fn a_cfp_closing_the_day_after_the_end_is_refused() {
        let complaint = date_complaint(Some("2027-05-04"), Some("2027-05-03"))
            .expect("a CFP closing after the end is the whole rule");
        assert!(complaint.contains("2027-05-04"), "{complaint}");
        assert!(complaint.contains("2027-05-03"), "{complaint}");
    }

    #[test]
    fn a_cfp_closing_long_after_the_end_is_refused() {
        assert!(date_complaint(Some("2028-01-01"), Some("2027-05-03")).is_some());
    }

    #[test]
    fn a_missing_date_is_not_this_rules_business() {
        assert_eq!(date_complaint(None, Some("2027-05-03")), None);
        assert_eq!(date_complaint(Some("2027-05-04"), None), None);
        assert_eq!(date_complaint(None, None), None);
    }

    #[test]
    fn an_unreadable_date_is_not_reported_as_wrong() {
        // Refusing to parse something is not the same as finding it wrong, and
        // a complaint quoting "soon" would help nobody.
        assert_eq!(date_complaint(Some("soon"), Some("2027-05-03")), None);
        assert_eq!(date_complaint(Some("2027-05-04"), Some("whenever")), None);
        assert_eq!(date_complaint(Some("2027-5-4"), Some("2027-05-03")), None);
    }

    // Across a year boundary, where a string comparison would also have worked,
    // and across a month boundary, where it would not.
    #[test]
    fn the_comparison_is_by_date_not_by_string() {
        assert!(date_complaint(Some("2027-09-02"), Some("2027-10-01")).is_none());
        assert!(date_complaint(Some("2027-10-01"), Some("2027-09-02")).is_some());
    }

    // ─── The presave tap ──────────────────────────────────────────

    fn presave(item_type: &str, cfp_end: &str, end: &str) -> serde_json::Value {
        __inner_tap_item_presave(serde_json::json!({
            "item_type": item_type,
            "title": "Some Conf",
            "fields": {
                "field_cfp_end_date": cfp_end,
                "field_end_date": end,
            },
        }))
    }

    /// The tap never changes a field, and that is the design rather than an
    /// oversight: see `tap_item_presave`. An empty object is "no changes".
    #[test]
    fn the_presave_tap_modifies_nothing_whatever_the_dates_say() {
        for (cfp, end) in [("2027-05-04", "2027-05-03"), ("2027-03-01", "2027-05-03")] {
            let out = presave("conference", cfp, end);
            assert_eq!(out, serde_json::json!({}), "cfp {cfp}, end {end}");
        }
    }

    #[test]
    fn the_presave_tap_ignores_other_item_types() {
        assert_eq!(
            presave("speaker", "2027-05-04", "2027-05-03"),
            serde_json::json!({})
        );
    }

    /// A field stored as `{"value": ...}` reads the same as a bare string.
    ///
    /// Both shapes exist in the wild: the admin stack writes one and the
    /// importer the other, and a rule that only saw one of them would pass on
    /// half the site's content.
    #[test]
    fn the_presave_tap_reads_both_stored_field_shapes() {
        let wrapped = __inner_tap_item_presave(serde_json::json!({
            "item_type": "conference",
            "title": "Some Conf",
            "fields": {
                "field_cfp_end_date": {"value": "2027-05-04"},
                "field_end_date": {"value": "2027-05-03"},
            },
        }));
        assert_eq!(wrapped, serde_json::json!({}));
    }

    #[test]
    fn the_presave_tap_survives_an_item_with_no_fields() {
        let out = __inner_tap_item_presave(serde_json::json!({
            "item_type": "conference",
            "title": "Bare Conf",
        }));
        assert_eq!(out, serde_json::json!({}));
    }

    #[test]
    fn date_to_days_matches_known_dates() {
        assert_eq!(date_to_days("1970-01-01"), Some(0));
        assert_eq!(date_to_days("2000-03-01"), Some(11_017));
        assert_eq!(date_to_days("2024-02-29"), Some(19_782));
    }
}
