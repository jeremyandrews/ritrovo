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

    #[test]
    fn date_to_days_matches_known_dates() {
        assert_eq!(date_to_days("1970-01-01"), Some(0));
        assert_eq!(date_to_days("2000-03-01"), Some(11_017));
        assert_eq!(date_to_days("2024-02-29"), Some(19_782));
    }
}
