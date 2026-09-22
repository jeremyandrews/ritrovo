//! A member's conference subscriptions: the list, and the two writes.
//!
//! # Where a subscription is stored, and why it is the kernel's table
//!
//! In `user_subscriptions`, which the kernel created in
//! `migrations/20260315000001_create_user_subscriptions.sql` and which nothing
//! in the kernel has ever called. It is `(user_id, item_id, created)` with the
//! pair as its primary key and `ON DELETE CASCADE` to both `users` and `item`.
//!
//! This plugin owns no subscriptions table and deliberately did not add one.
//! The kernel's table was tried first because a subscription its own
//! `Subscription` model can read is worth more than a private one: the day the
//! kernel grows a subscriptions route, a digest, or an "N people are watching
//! this" count, every row this plugin wrote is already in the right place.
//!
//! **The database policy admits it.** `crates/kernel/src/plugin/db_policy.rs`
//! derives a plugin's allowlist as the union of the tables its own migrations
//! create and the tables its manifest names in `db_tables`, and
//! `check_table` is a bare set membership on that union. There is no ownership
//! concept, no denylist of kernel tables, and no cross-reference to which
//! migration created what. Two plugins shipped in the kernel image already rely
//! on that: `trovato_book` declares `item` and `trovato_spam` declares
//! `comment`. So naming `user_subscriptions` and `item` here is the documented
//! path rather than a hole, and the fallback of a private table was not needed.
//!
//! The honest footnote is that the declaration is advisory for this plugin.
//! The SDK wraps only `query_raw` and `execute_raw`, not the four structured
//! `db` calls the WIT declares, so every statement here is raw SQL — and
//! `check_raw_sql` reads only the `raw_sql` flag and never consults the table
//! list. `db_tables` is therefore a statement of reach that nothing enforces
//! while `raw_sql = true`. It is still the right declaration, because it is the
//! one an auditor reads. `FRICTION.md`, `G-PLUGIN-CANNOT-READ-BACK-AN-INSERT`.
//!
//! # Why the toggle is not on the conference page
//!
//! It cannot be, on this kernel, and it takes two gaps together to make that
//! true. Each is survivable alone.
//!
//! - The item template's context does not carry the viewer. The kernel builds
//!   a fresh context with seven keys — `item`, `children`, `referenced_items`,
//!   `reverse_references`, `safe_urls`, `active_language`, `text_direction` —
//!   and renders `elements/item--conference.html` from it. The viewer is loaded
//!   for that request and used for access control, and is never put in the
//!   context. So a template cannot tell a signed-in visitor from an anonymous
//!   one, and a Subscribe control rendered there would be shown to everyone
//!   including the visitors it would refuse. `G-ITEM-TEMPLATE-HAS-NO-VIEWER`.
//! - `tap_item_view` *does* know the viewer, through `current-user-id` on the
//!   user host interface. But its output is appended into `children`, the same
//!   string the kernel fills with a generic dump of every scalar field, and
//!   Ritrovo's conference template does not render `children` for exactly that
//!   reason. There is no handle on the plugin half alone, so a view tap's
//!   output is invisible on the page it would be for.
//!   `G-RENDER-CHILDREN-MIXES-FIELD-DUMP-AND-PLUGIN-OUTPUT`.
//!
//! So the control lives on this plugin's own page, which is authenticated, and
//! every response states the resulting state in words. The way a member reaches
//! it is the user menu: `tap_menu`'s entry is gated on a permission no
//! anonymous visitor holds, and the kernel filters the menu it renders to what
//! the viewer may open. An anonymous visitor is therefore never shown a control
//! that would refuse them, which is the rule this design exists to keep.

use trovato_sdk::host;
use trovato_sdk::types::{ApiRequest, ApiResponse};

use crate::web;

/// The member's own list. `{uid}` is a path parameter the kernel extracts.
pub const PATH_LIST: &str = "/user/{uid}/subscriptions";

/// Where the Subscribe form posts.
pub const PATH_SUBSCRIBE: &str = "/user/{uid}/subscriptions/subscribe";

/// Where the Unsubscribe form posts.
pub const PATH_UNSUBSCRIBE: &str = "/user/{uid}/subscriptions/unsubscribe";

/// One row of the member's list.
struct Row {
    item_id: String,
    title: String,
}

/// What a write did, so the page can say it.
enum Outcome {
    Subscribed(String),
    Unsubscribed(String),
    AlreadySubscribed(String),
    NotSubscribed(String),
    NoSuchConference,
    Failed,
}

impl Outcome {
    /// The sentence shown above the list.
    fn message(&self) -> String {
        match self {
            Self::Subscribed(title) => {
                format!("You are now subscribed to {}.", web::escape(title))
            }
            Self::Unsubscribed(title) => {
                format!("You are no longer subscribed to {}.", web::escape(title))
            }
            Self::AlreadySubscribed(title) => {
                format!("You were already subscribed to {}.", web::escape(title))
            }
            Self::NotSubscribed(title) => {
                format!("You were not subscribed to {}.", web::escape(title))
            }
            Self::NoSuchConference => {
                "That conference could not be found. It may have been removed.".to_string()
            }
            Self::Failed => {
                "Your subscriptions could not be changed. Please try again.".to_string()
            }
        }
    }

    /// The CSS class, so a failure does not read like a success.
    fn class(&self) -> &'static str {
        match self {
            Self::NoSuchConference | Self::Failed => "ritrovo-errors",
            _ => "ritrovo-saved",
        }
    }

    /// The HTTP status. A refused write is not a 200 dressed as one.
    fn status(&self) -> u16 {
        match self {
            Self::NoSuchConference => 404,
            Self::Failed => 500,
            _ => 200,
        }
    }
}

// ─── Handlers ─────────────────────────────────────────────────────────

/// The member's own list of subscriptions.
///
/// `?conference={uuid}` additionally renders the Subscribe or Unsubscribe form
/// for that conference, with its current state. That is the address a link on a
/// conference page would point at, once the kernel gives a template a way to
/// render one only for a signed-in visitor.
pub fn show(request: &ApiRequest) -> ApiResponse {
    let Some(uid) = own_uid(request) else {
        return refused();
    };
    let focus = request.query.get("conference").map(String::as_str);
    ApiResponse::themed_with_status(
        200,
        "Your subscriptions",
        page(&uid, &request.csrf_token, focus, None),
    )
}

/// Subscribe the signed-in member to one conference.
pub fn subscribe(request: &ApiRequest) -> ApiResponse {
    write(request, true)
}

/// Unsubscribe the signed-in member from one conference.
pub fn unsubscribe(request: &ApiRequest) -> ApiResponse {
    write(request, false)
}

/// The body both writes share.
///
/// The CSRF token was already checked by the kernel, before dispatch, for every
/// state-changing method; this never sees a request that failed it.
fn write(request: &ApiRequest, subscribing: bool) -> ApiResponse {
    let Some(uid) = own_uid(request) else {
        return refused();
    };

    let item_id = web::field(&request.body, "item_id");
    if !web::is_uuid(&item_id) {
        return rendered(&uid, request, Some(&item_id), Outcome::NoSuchConference);
    }

    // Resolve the title first, which is also the existence and eligibility
    // check: a member may only subscribe to a published conference. Without it
    // this endpoint would accept any uuid, and a row naming an item the member
    // cannot see would put its title on their list the day it was published.
    let Some(title) = conference_title(&item_id) else {
        return rendered(&uid, request, Some(&item_id), Outcome::NoSuchConference);
    };

    let already = is_subscribed(&uid, &item_id);
    let outcome = match (subscribing, already) {
        (true, true) => Outcome::AlreadySubscribed(title),
        (false, false) => Outcome::NotSubscribed(title),
        (true, false) => {
            if store(&uid, &item_id) {
                Outcome::Subscribed(title)
            } else {
                Outcome::Failed
            }
        }
        (false, true) => {
            if remove(&uid, &item_id) {
                Outcome::Unsubscribed(title)
            } else {
                Outcome::Failed
            }
        }
    };

    rendered(&uid, request, Some(&item_id), outcome)
}

/// Render the list with an outcome above it.
fn rendered(uid: &str, request: &ApiRequest, focus: Option<&str>, outcome: Outcome) -> ApiResponse {
    ApiResponse::themed_with_status(
        outcome.status(),
        "Your subscriptions",
        page(uid, &request.csrf_token, focus, Some(&outcome)),
    )
}

/// The uid in the path, when it is the caller's own.
///
/// The privacy rule, and the only one this plugin has: a member may read and
/// change their own subscriptions and nobody else's. The kernel has already
/// established that the caller holds `manage own subscriptions` and is signed
/// in; what it cannot know is that `{uid}` names them, because the permission
/// is not per-row.
fn own_uid(request: &ApiRequest) -> Option<String> {
    let uid = request.params.get("uid")?;
    // An anonymous caller never reaches here — the route's permission is one no
    // anonymous role holds — but the check does not rely on that, because a
    // misconfigured grant should cost a 403 rather than somebody's list.
    if request.authenticated && !request.user_id.is_empty() && uid == &request.user_id {
        Some(uid.clone())
    } else {
        None
    }
}

/// Another member's list is a 403, and says so without naming what is there.
fn refused() -> ApiResponse {
    ApiResponse::themed_with_status(
        403,
        "Your subscriptions",
        "<p class=\"ritrovo-errors\">You can only see your own subscriptions.</p>\
         <p><a href=\"/user\">Your account</a></p>"
            .to_string(),
    )
}

// ─── Storage ──────────────────────────────────────────────────────────

/// Every conference this member subscribes to, newest first.
///
/// Joined to `item` so the list reads as conference names rather than uuids,
/// and filtered to published conferences so a withdrawn one leaves the list
/// rather than sitting on it as a dead link. The row stays in the table: the
/// kernel's cascade is what removes it when the item is really deleted, and a
/// conference pulled back to an internal stage may well come back.
fn list(user_id: &str) -> Vec<Row> {
    let raw = match host::query_raw(
        "SELECT s.item_id::text AS item_id, i.title \
         FROM user_subscriptions s \
         JOIN item i ON i.id = s.item_id \
         WHERE s.user_id = $1::uuid AND i.type = 'conference' AND i.status = 1 \
         ORDER BY s.created DESC",
        &[serde_json::json!(user_id)],
    ) {
        Ok(raw) => raw,
        Err(code) => {
            host::log(
                "error",
                crate::PLUGIN_NAME,
                &format!("could not read subscriptions: host error {code}"),
            );
            return Vec::new();
        }
    };

    serde_json::from_str::<Vec<serde_json::Value>>(&raw)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| {
            Some(Row {
                item_id: row.get("item_id")?.as_str()?.to_string(),
                title: row.get("title")?.as_str()?.to_string(),
            })
        })
        .collect()
}

/// The title of a published conference, or `None` when there is no such thing.
fn conference_title(item_id: &str) -> Option<String> {
    let raw = host::query_raw(
        "SELECT title FROM item \
         WHERE id = $1::uuid AND type = 'conference' AND status = 1",
        &[serde_json::json!(item_id)],
    )
    .ok()?;

    serde_json::from_str::<Vec<serde_json::Value>>(&raw)
        .ok()?
        .into_iter()
        .next()?
        .get("title")?
        .as_str()
        .map(str::to_string)
}

/// Whether this member already subscribes to this conference.
fn is_subscribed(user_id: &str, item_id: &str) -> bool {
    let Ok(raw) = host::query_raw(
        "SELECT 1 AS present FROM user_subscriptions \
         WHERE user_id = $1::uuid AND item_id = $2::uuid",
        &[serde_json::json!(user_id), serde_json::json!(item_id)],
    ) else {
        return false;
    };

    serde_json::from_str::<Vec<serde_json::Value>>(&raw)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false)
}

/// Write the subscription, returning whether it is now there.
///
/// `ON CONFLICT DO NOTHING` rather than a check first: two tabs pressing
/// Subscribe would otherwise race, and the loser would be a primary-key
/// violation rather than the state the member asked for. It also means a zero
/// row count is success, which is why the caller establishes `already` before
/// deciding what to say.
fn store(user_id: &str, item_id: &str) -> bool {
    host::execute_raw(
        "INSERT INTO user_subscriptions (user_id, item_id) \
         VALUES ($1::uuid, $2::uuid) ON CONFLICT (user_id, item_id) DO NOTHING",
        &[serde_json::json!(user_id), serde_json::json!(item_id)],
    )
    .is_ok()
}

/// Remove the subscription, returning whether the statement ran.
fn remove(user_id: &str, item_id: &str) -> bool {
    host::execute_raw(
        "DELETE FROM user_subscriptions WHERE user_id = $1::uuid AND item_id = $2::uuid",
        &[serde_json::json!(user_id), serde_json::json!(item_id)],
    )
    .is_ok()
}

// ─── The page ─────────────────────────────────────────────────────────

/// The whole page: an outcome, the focused conference's form, and the list.
fn page(uid: &str, token: &str, focus: Option<&str>, outcome: Option<&Outcome>) -> String {
    let mut html = String::new();

    if let Some(outcome) = outcome {
        html.push_str(&format!(
            r#"<p class="{}">{}</p>"#,
            outcome.class(),
            outcome.message()
        ));
    }

    let rows = list(uid);

    // The focused conference, when the caller named one and it exists. Rendered
    // above the list because it is what they came here to change.
    if let Some(item_id) = focus.filter(|id| web::is_uuid(id))
        && let Some(title) = conference_title(item_id)
    {
        let subscribed = rows.iter().any(|row| row.item_id == item_id);
        html.push_str(&format!(
            r#"<div class="ritrovo-subscription-toggle"><h2>{title}</h2>{form}</div>"#,
            title = web::escape(&title),
            form = toggle_form(uid, token, item_id, subscribed, "this conference"),
        ));
    }

    html.push_str("<h2>Conferences you follow</h2>");

    if rows.is_empty() {
        html.push_str(r#"<p class="ritrovo-empty">You are not following any conferences yet.</p>"#);
    } else {
        html.push_str(r#"<ul class="ritrovo-subscriptions">"#);
        for row in &rows {
            html.push_str(&format!(
                r#"<li class="ritrovo-subscription"><a href="/item/{id}">{title}</a> {form}</li>"#,
                id = web::escape(&row.item_id),
                title = web::escape(&row.title),
                form = toggle_form(uid, token, &row.item_id, true, &row.title),
            ));
        }
        html.push_str("</ul>");
    }

    html.push_str(
        r#"<p class="form-help">You will be told here when a conference you follow changes its
dates, its venue or its call for papers. Nothing is emailed: a plugin on this
release has no way to send mail to a site's own member, so the notification
list is the deliverable and the digest is not built.</p>"#,
    );

    html
}

/// One Subscribe or Unsubscribe button, as a form that needs no JavaScript.
///
/// A `<form method="post">` rather than a link, because it changes something:
/// a GET that writes is a GET a crawler will make. There is no AJAX half at
/// all — the kernel's form AJAX is administrator-only and routes through a
/// form service no route calls (`G-AJAX-ADMIN-ONLY-NO-CONDITIONAL-FIELDS`).
fn toggle_form(uid: &str, token: &str, item_id: &str, subscribed: bool, label: &str) -> String {
    let (path, verb) = if subscribed {
        (PATH_UNSUBSCRIBE, "Unsubscribe")
    } else {
        (PATH_SUBSCRIBE, "Subscribe")
    };
    format!(
        r#"<form method="post" action="{action}" class="ritrovo-subscribe-form">
<input type="hidden" name="_token" value="{token}">
<input type="hidden" name="item_id" value="{item_id}">
<button type="submit">{verb}<span class="sr-only"> from {label}</span></button>
</form>"#,
        action = web::escape(&path.replace("{uid}", uid)),
        token = web::escape(token),
        item_id = web::escape(item_id),
        label = web::escape(label),
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    const UID: &str = "11111111-1111-4111-8111-111111111111";
    const OTHER: &str = "22222222-2222-4222-8222-222222222222";
    const CONF: &str = "33333333-3333-4333-8333-333333333333";

    fn request(uid: &str, caller: &str, authenticated: bool) -> ApiRequest {
        let mut request = ApiRequest::new(
            "subscriptions_show",
            "GET",
            PATH_LIST,
            caller,
            authenticated,
        );
        request.params.insert("uid".to_string(), uid.to_string());
        request
    }

    #[test]
    fn a_member_may_read_their_own_list() {
        assert_eq!(own_uid(&request(UID, UID, true)).as_deref(), Some(UID));
    }

    #[test]
    fn another_members_list_is_refused() {
        assert!(own_uid(&request(OTHER, UID, true)).is_none());
        assert_eq!(refused().status, 403);
    }

    #[test]
    fn an_unauthenticated_caller_is_refused_even_naming_their_own_uid() {
        // The route's permission already stops this. The check does not lean on
        // that, because a grant that reached the anonymous role by mistake
        // should cost a 403 rather than every member's list.
        assert!(own_uid(&request(UID, UID, false)).is_none());
    }

    #[test]
    fn a_request_with_no_uid_in_the_path_is_refused() {
        let request = ApiRequest::new("subscriptions_show", "GET", PATH_LIST, UID, true);
        assert!(own_uid(&request).is_none());
    }

    #[test]
    fn the_form_carries_the_token_and_the_item_and_needs_no_javascript() {
        let html = toggle_form(UID, "tok-123", CONF, false, "RustConf");
        assert!(html.contains(r#"name="_token" value="tok-123""#), "{html}");
        assert!(
            html.contains(&format!(r#"name="item_id" value="{CONF}""#)),
            "{html}"
        );
        assert!(html.contains(r#"method="post""#), "{html}");
        assert!(!html.contains("<script"), "{html}");
        assert!(!html.contains("onclick"), "{html}");
    }

    #[test]
    fn the_form_posts_to_the_members_own_path() {
        let html = toggle_form(UID, "tok", CONF, false, "RustConf");
        assert!(
            html.contains(&format!(r#"action="/user/{UID}/subscriptions/subscribe""#)),
            "{html}"
        );
        assert!(!html.contains("{uid}"), "the path template leaked: {html}");
    }

    #[test]
    fn a_subscribed_conference_offers_unsubscribe_and_the_reverse() {
        assert!(toggle_form(UID, "t", CONF, true, "x").contains(">Unsubscribe"));
        assert!(toggle_form(UID, "t", CONF, false, "x").contains(">Subscribe"));
    }

    #[test]
    fn a_title_is_escaped_into_the_button_label() {
        let html = toggle_form(UID, "t", CONF, true, "<script>alert(1)</script>");
        assert!(!html.contains("<script>"), "{html}");
        assert!(html.contains("&lt;script&gt;"), "{html}");
    }

    #[test]
    fn every_outcome_says_what_happened_and_carries_an_honest_status() {
        assert!(
            Outcome::Subscribed("RustConf".into())
                .message()
                .contains("now subscribed to RustConf")
        );
        assert!(
            Outcome::Unsubscribed("RustConf".into())
                .message()
                .contains("no longer subscribed")
        );
        assert!(
            Outcome::AlreadySubscribed("RustConf".into())
                .message()
                .contains("already subscribed")
        );
        assert!(
            Outcome::NotSubscribed("RustConf".into())
                .message()
                .contains("were not subscribed")
        );
        assert_eq!(Outcome::Subscribed("x".into()).status(), 200);
        assert_eq!(Outcome::NoSuchConference.status(), 404);
        assert_eq!(Outcome::Failed.status(), 500);
    }

    #[test]
    fn a_failure_does_not_read_like_a_success() {
        assert_eq!(Outcome::Failed.class(), "ritrovo-errors");
        assert_eq!(Outcome::NoSuchConference.class(), "ritrovo-errors");
        assert_eq!(Outcome::Subscribed("x".into()).class(), "ritrovo-saved");
    }

    #[test]
    fn a_conference_title_is_escaped_into_the_outcome() {
        let message = Outcome::Subscribed("<b>x</b>".into()).message();
        assert!(!message.contains("<b>"), "{message}");
        assert!(message.contains("&lt;b&gt;"), "{message}");
    }
}
