//! Editorial workflow access control plugin for Ritrovo conferences.
//!
//! Provides permissions and access rules for conference editorial workflow:
//! - Stage-based visibility (Incoming, Curated, Live)
//! - Editor-only fields (editor_notes)
//! - Role-based permissions for conference management
//! - The editorial screen that moves conferences between stages
//!
//! # Why this plugin owns stage transitions
//!
//! The brief's pipeline is Incoming, then Curated, then Live, and the kernel at
//! the pinned release has no way to move one item between stages: not a route,
//! not a form field, not a bulk action, not a host call. `StageService::publish`
//! moves an entire stage at once and no route reaches it, and the workflow file
//! that describes the transitions is read by nothing (`G-NO-ITEM-STAGE-TRANSITION`).
//! The kernel's own bulk operations on `/admin/content` are a hardcoded
//! `publish | unpublish | delete`, with no tap, registry or any other seam a
//! plugin could add a fourth action to.
//!
//! So Ritrovo serves its own screen. That is not a workaround bolted onto a
//! kernel feature; it is the only path, and it is a supported one: a plugin
//! registers a route through `tap_menu`, serves it through `tap_api`, receives a
//! kernel-minted CSRF token to put in its form, and writes through the `db` host
//! interface. Everything here is the published plugin contract.
//!
//! # What is real access control here and what is not
//!
//! `tap_item_access` is real: the kernel asks this plugin before it will show an
//! item on an internal stage, and a `Deny` is final. `tap_field_access` is real
//! for the same reason: the kernel drops a denied field before it renders
//! anything. Both are enforced by the kernel, not by a template.
//!
//! The permission names are the compromise. Every permission this plugin
//! declares through `tap_perm` is invisible to the kernel, which never
//! dispatches that tap, so no role can hold one (`G-PERM-TAP-NOT-DISPATCHED`).
//! The editorial roles therefore carry kernel permissions instead, and this
//! plugin reads those as its editorial signal. See [`EDITOR_PERMISSIONS`] and
//! [`PUBLISHER_PERMISSIONS`], and `demo/config/role.*.yml`, which say the same
//! thing from the configuration side.

use trovato_sdk::host;
use trovato_sdk::prelude::*;
use trovato_sdk::types::{ApiRequest, ApiResponse, MenuRoute};

// ─── Stages ──────────────────────────────────────────────────────────

/// The Incoming stage: imported, unreviewed. `demo/config/stage.*.yml`.
const INCOMING_STAGE: &str = "0193a5a0-0000-7000-8000-000000000002";

/// The Curated stage: reviewed, awaiting publication.
const CURATED_STAGE: &str = "0193a5a0-0000-7000-8000-000000000003";

/// The Live stage: public. The one stage the kernel knows by name, so the SDK
/// exports it and this plugin does not redeclare it.
const LIVE_STAGE: &str = LIVE_STAGE_UUID;

/// Machine name of each stage, for display and for the access tap.
const INCOMING: &str = "incoming";
const CURATED: &str = "curated";

// ─── The permission substitution ─────────────────────────────────────

/// What this plugin accepts as "this viewer is an editor".
///
/// `edit conferences` is what the brief asks for and what `tap_perm` declares.
/// It is listed first because it is the right answer and the one this plugin
/// will use alone once the kernel dispatches `tap_perm`. Until then no role can
/// hold it, so `edit any content` — a kernel permission, granted to the editor
/// and publisher roles in `demo/config` — stands in for it.
///
/// The substitution is coarser than the brief: `edit any content` is site-wide,
/// so it makes a Ritrovo editor an editor of every item type rather than of
/// conferences only. That is a real widening of access and it is recorded in
/// `FRICTION.md` under `G-PERM-TAP-NOT-DISPATCHED`, not hidden here.
const EDITOR_PERMISSIONS: &[&str] = &["edit conferences", "edit any content"];

/// What this plugin accepts as "this viewer may publish".
///
/// `publish conferences` is the brief's permission and is equally ungrantable.
/// The kernel has no permission that means "publish", because promoting a stage
/// is not an operation it models at all, so there is nothing to substitute that
/// means the same thing. `delete any content` is used as the marker that
/// separates the publisher role from the editor role, and it is a stand-in
/// rather than an equivalent. The editorial screen says so where it matters, on
/// the control it gates.
const PUBLISHER_PERMISSIONS: &[&str] = &["publish conferences", "delete any content"];

/// The administrator's permission.
///
/// Needed explicitly because `current-user-has-permission` is a literal
/// membership test with no administrator short circuit, unlike every kernel
/// route (`G-USER-API-NO-ADMIN-BYPASS`). Without this an administrator would be
/// refused by this plugin's own screen.
const ADMIN_PERMISSION: &str = "administer site";

/// Register conference-specific permissions.
#[plugin_tap]
pub fn tap_perm() -> Vec<PermissionDefinition> {
    vec![
        PermissionDefinition::new(
            "view incoming conferences",
            "View incoming (unreviewed) conferences",
        ),
        PermissionDefinition::new(
            "view curated conferences",
            "View curated (reviewed) conferences",
        ),
        PermissionDefinition::new("edit conferences", "Edit conference content"),
        PermissionDefinition::new("publish conferences", "Publish conferences to live"),
        PermissionDefinition::new("post comments", "Post comments on conferences"),
        PermissionDefinition::new("edit own comments", "Edit own comments"),
        PermissionDefinition::new("edit any comments", "Edit any user's comments"),
    ]
}

/// Control access to conference items based on editorial stage and operation.
///
/// **View operations** on internal stages:
/// - **Incoming:** `view incoming conferences`, or an editor's permission
/// - **Curated:** `view curated conferences`, or an editor's permission
/// - **Live (or no stage info):** Neutral, so the kernel decides public access
///
/// **Non-view operations** (`edit`, `delete`):
/// - Require an editor's permission on any stage
/// - A viewer who may see an internal stage still may not change what is on it
///
/// Non-conference items always return Neutral, so this plugin never speaks about
/// content it does not own.
///
/// "An editor's permission" is [`EDITOR_PERMISSIONS`], which is the brief's
/// `edit conferences` plus the kernel permission standing in for it until
/// `tap_perm` is dispatched. Reading both means the plugin needs no change on
/// the day the kernel starts dispatching it.
///
/// # What this tap is never asked
///
/// Two layers run before it and can answer without it
/// (`crates/kernel/src/content/item_service.rs:1017-1155`): an administrator is
/// granted at `:1023`, and a published item on a public stage is granted for
/// `view` at `:1072-1082` to anyone holding `access content`. So this tap is
/// never consulted about a published Live conference, which is most of the site,
/// and it cannot Deny one. The kernel also denies an unauthenticated viewer any
/// item on an internal stage before dispatching (`:1070`), so every input here
/// is an authenticated viewer.
///
/// A `Deny` from any plugin wins outright and returns immediately; a `Grant`
/// wins over `Neutral`; all-`Neutral` falls through to the kernel's role
/// fallback (`:1109-1126`). `publish conferences` is deliberately not checked
/// here: publishing is a stage transition, enforced by [`TRANSITIONS`] on the
/// editorial screen, not an item-access operation. The kernel passes only
/// `view`, `edit` and `delete`.
#[plugin_tap]
pub fn tap_item_access(input: ItemAccessInput) -> AccessResult {
    // Only control access to conference items
    if input.item_type != "conference" {
        return AccessResult::Neutral;
    }

    // No stage info means live/default — let kernel handle
    let Some(stage) = input.stage_machine_name.as_deref() else {
        return AccessResult::Neutral;
    };

    // Non-view operations (edit, delete) require an editor's permission
    // regardless of stage. This prevents viewers from mutating content
    // on internal stages even though they can see it.
    if input.operation != "view" {
        return if has_any_permission(&input, EDITOR_PERMISSIONS) {
            AccessResult::Grant
        } else {
            AccessResult::Deny
        };
    }

    // View operations: check stage-specific permissions
    match stage {
        INCOMING => {
            if has_any_permission(&input, &["view incoming conferences"])
                || has_any_permission(&input, EDITOR_PERMISSIONS)
            {
                AccessResult::Grant
            } else {
                AccessResult::Deny
            }
        }
        CURATED => {
            if has_any_permission(&input, &["view curated conferences"])
                || has_any_permission(&input, EDITOR_PERMISSIONS)
            {
                AccessResult::Grant
            } else {
                AccessResult::Deny
            }
        }
        // Live/public or unknown stages — let kernel's default logic handle
        _ => AccessResult::Neutral,
    }
}

/// Check if the user has any of the given permissions.
fn has_any_permission(input: &ItemAccessInput, perms: &[&str]) -> bool {
    perms
        .iter()
        .any(|p| input.user_permissions.iter().any(|up| up == p))
}

/// The field only an editor may read.
///
/// Declared by `demo/config/item_type.conference.yml`, and the brief's one
/// example of field-level access: "editors see CFP notes, anonymous don't".
const EDITOR_ONLY_FIELD: &str = "field_editor_notes";

/// Hide `field_editor_notes` from anyone who is not an editor.
///
/// # This is access control, not presentation
///
/// The kernel drops a denied field from the Item **before** it renders anything
/// and before any view tap runs (`crates/kernel/src/content/item_service.rs:573`,
/// dispatching this tap at `:1236`). A template never sees the value, so there
/// is no markup to inspect, no API response carrying it, and nothing for a
/// theme change to undo. That is the difference between this and hiding a field
/// in a template, which would leave the value in every other representation of
/// the item.
///
/// # Why this tap and not `tap_item_view`
///
/// `tap_item_view` was the plan and is the wrong tool twice over: its input is
/// the `Item` alone with no viewer in it, and a view tap can only append HTML,
/// never remove a field. The kernel's own documentation describes a signature
/// (`tap_item_view(input: ItemViewInput) -> RenderElement`) that does not exist
/// in the SDK, which is what sent the previous implementation to wait for a
/// kernel change nobody needed. `FRICTION.md`,
/// `G-VIEW-TAP-INPUT-CARRIES-NO-VIEWER`. The no-op stub it left behind is gone
/// with this commit; it fired on every item view and did nothing.
///
/// # The three things this tap's contract makes non-obvious
///
/// **Fail open.** Field access refines item access, so the kernel treats
/// `NoOpinion`, an absent field, an unparseable answer and no implementer alike:
/// all mean visible (`aggregate_field_decisions`, `item_service.rs:274-293`).
/// Hiding therefore requires an explicit `Deny`; returning nothing shows the
/// field. Deny still wins over any other plugin's Allow.
///
/// **It is a batch, and a partial one.** The kernel caches decisions per
/// `(permission set, type, field, operation)` and asks only about fields it has
/// not already resolved, so this is handed some subset of the type's fields and
/// must answer about what it was given rather than about what it expects.
///
/// **It is type-level.** There is no item id in the payload, so a rule can say
/// "editor notes on a conference" and cannot say "on this conference". Nothing
/// here needs per-item granularity; a rule that did could not be written.
///
/// An administrator never reaches this tap: the kernel returns before dispatch
/// for `is_admin` (`item_service.rs:1194-1200`). So, unlike the editorial screen,
/// this needs no administrator special case.
///
/// # Known hole, and it is the kernel's
///
/// The kernel evaluates field access for `view` only (`routes/item.rs:369`,
/// `:1382`), so a field hidden on the page is still rendered on the **edit
/// form** for anyone who can open it. Nobody who lacks an editor's permission
/// can open a conference's edit form — `tap_item_access` denies the `edit`
/// operation to them — so the hole is closed by the item-access tap rather than
/// by this one, and it would open the moment that stopped being true.
#[plugin_tap]
pub fn tap_field_access(input: FieldAccessBatchInput) -> FieldAccessBatchResult {
    let mut decisions = std::collections::HashMap::new();

    // Say nothing about other people's content types.
    if input.item_type != "conference" {
        return FieldAccessBatchResult { decisions };
    }

    let is_editor = input
        .user
        .permissions
        .iter()
        .any(|held| EDITOR_PERMISSIONS.iter().any(|p| held == p));

    // Answer only about the fields the kernel actually asked about, and only
    // about the one field this plugin has a rule for. Every other field is left
    // absent, which the kernel reads as NoOpinion.
    for field in &input.fields {
        if field == EDITOR_ONLY_FIELD {
            decisions.insert(
                field.clone(),
                if is_editor {
                    FieldAccessResult::Allow
                } else {
                    FieldAccessResult::Deny
                },
            );
        }
    }

    FieldAccessBatchResult { decisions }
}

// ═══ The editorial screen ════════════════════════════════════════════

/// How many conferences one page of the editorial queue shows.
const QUEUE_PAGE_SIZE: i64 = 50;

/// One editorial transition: which stage, to which stage, and who may.
struct Transition {
    /// Machine name of the stage being left.
    from: &'static str,
    /// Machine name of the stage being entered.
    to: &'static str,
    /// Stage uuid being entered.
    to_uuid: &'static str,
    /// The label on the button.
    label: &'static str,
    /// Permissions that allow it, any one of which is enough.
    allowed_by: &'static [&'static str],
    /// Whether `allowed_by` is a stand-in rather than the brief's permission.
    substituted: bool,
}

/// The whole workflow, as the brief defines it.
///
/// This table is the workflow. The kernel ships a
/// `variable.workflow.editorial.yml` describing the same transitions and reads
/// it from nowhere (`G-NO-ITEM-STAGE-TRANSITION`), so putting the rules in a
/// configuration file would be writing them down twice and enforcing them
/// never. They live here, where they are enforced.
const TRANSITIONS: &[Transition] = &[
    Transition {
        from: INCOMING,
        to: CURATED,
        to_uuid: CURATED_STAGE,
        label: "Promote to Curated",
        allowed_by: EDITOR_PERMISSIONS,
        substituted: true,
    },
    Transition {
        from: CURATED,
        to: INCOMING,
        to_uuid: INCOMING_STAGE,
        label: "Send back to Incoming",
        allowed_by: EDITOR_PERMISSIONS,
        substituted: true,
    },
    Transition {
        from: CURATED,
        to: "live",
        to_uuid: LIVE_STAGE,
        label: "Publish to Live",
        allowed_by: PUBLISHER_PERMISSIONS,
        substituted: true,
    },
    Transition {
        from: "live",
        to: CURATED,
        to_uuid: CURATED_STAGE,
        label: "Unpublish to Curated",
        allowed_by: PUBLISHER_PERMISSIONS,
        substituted: true,
    },
];

/// Does the viewer hold any of these permissions?
///
/// Administrators are admitted explicitly. Every kernel route treats an
/// administrator as holding every permission, and this host call does not
/// (`G-USER-API-NO-ADMIN-BYPASS`), so without the second line the site
/// administrator would be refused by Ritrovo's own editorial screen.
fn viewer_may(permissions: &[&str]) -> bool {
    permissions
        .iter()
        .any(|p| host::current_user_has_permission(p))
        || host::current_user_has_permission(ADMIN_PERMISSION)
}

/// The transitions available from a stage to this viewer.
fn available_transitions(from: &str) -> Vec<&'static Transition> {
    TRANSITIONS
        .iter()
        .filter(|t| t.from == from && viewer_may(t.allowed_by))
        .collect()
}

/// Escape text for an HTML text node or a double-quoted attribute.
///
/// The kernel serves a plugin's response body as-is and does not sanitize it,
/// so escaping is this plugin's job. Conference titles come from a third-party
/// dataset and from editors, so every one of them is untrusted input.
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

/// Decode one `application/x-www-form-urlencoded` body into its pairs.
///
/// Hand-rolled because the plugin compiles to `wasm32-wasip1` and this is the
/// only place in Ritrovo that needs it; a dependency for twenty lines would
/// travel into every shipped module. A malformed escape is left as written
/// rather than dropped, so a value can never be silently truncated into a
/// different uuid.
fn parse_form(body: &str) -> Vec<(String, String)> {
    body.split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (raw_key, raw_value) = match pair.split_once('=') {
                Some((k, v)) => (k, v),
                None => (pair, ""),
            };
            (percent_decode(raw_key), percent_decode(raw_value))
        })
        .collect()
}

/// Percent-decode one form field, turning `+` into a space.
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
            b'%' if i + 2 < bytes.len() => {
                let hex = &raw[i + 1..i + 3];
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
    String::from_utf8(out).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}

/// Is this string a uuid, in the shape the item table stores?
///
/// Every id this screen acts on arrives from a form, so it is attacker
/// controlled. It is bound as a parameter rather than interpolated, so this is
/// belt and braces, but it also keeps a malformed id out of the database
/// round trip entirely.
fn is_uuid(candidate: &str) -> bool {
    let bytes = candidate.as_bytes();
    bytes.len() == 36
        && bytes.iter().enumerate().all(|(i, b)| match i {
            8 | 13 | 18 | 23 => *b == b'-',
            _ => b.is_ascii_hexdigit(),
        })
}

/// Register the editorial screen.
///
/// Two entries for one path: the kernel registers one route per method, so the
/// GET that renders the queue and the POST that acts on it are separate menu
/// entries. Only the GET is `.visible()`; a POST entry in the navigation would
/// be a link nobody can follow.
///
/// Both are gated on the editor permission rather than on the administrator
/// flag, which is the whole point of the screen: the kernel's own
/// `/admin/content` requires `is_admin` whatever a role grants
/// (`G-ADMIN-SCREENS-ARE-ADMIN-ONLY`), so an editor cannot use it. This one an
/// editor can use. The publisher-only controls are gated inside the handler.
#[plugin_tap]
pub fn tap_menu() -> Vec<MenuRoute> {
    vec![
        MenuRoute::api("GET", "/admin/content/editorial", "editorial_queue")
            .title("Editorial queue")
            .permission("edit any content")
            .parent("/admin/content")
            .visible(),
        MenuRoute::api("POST", "/admin/content/editorial", "editorial_move")
            .title("Editorial queue")
            .permission("edit any content"),
    ]
}

/// Serve one request for a menu entry this plugin registered.
///
/// The kernel has already checked the entry's permission and, for the POST, the
/// CSRF token, so a request that reaches here holds both.
#[plugin_tap]
pub fn tap_api(request: ApiRequest) -> ApiResponse {
    match request.callback.as_str() {
        "editorial_queue" => editorial_queue(&request, None),
        "editorial_move" => editorial_move(&request),
        other => ApiResponse::error(404, &format!("no such callback: {other}")),
    }
}

/// Wrap a body fragment in a minimal standalone document.
fn editorial_page(title: &str, body: &str) -> ApiResponse {
    let html = format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <title>{t}</title>\n</head>\n<body>\n<h1>{t}</h1>\n{body}\n</body>\n</html>\n",
        t = escape_html(title),
    );
    ApiResponse::with_status(200, html).content_type("text/html; charset=utf-8")
}

/// The editorial queue: what is waiting on each stage, and what may be done.
///
/// `notice` carries the outcome of a POST. It is rendered into this same page
/// rather than flashed through a redirect because a plugin cannot redirect: an
/// `ApiResponse` has no headers field, so a 302 would go out with no `Location`,
/// and the kernel's flash message lives in a session no host call can write
/// (`G-PLUGIN-ROUTE-NO-HEADERS`). The cost is a page that re-submits its POST on
/// reload, which is why the form carries the stage it acted on.
fn editorial_queue(request: &ApiRequest, notice: Option<&str>) -> ApiResponse {
    let stage = request
        .query
        .get("stage")
        .map(String::as_str)
        .filter(|s| *s == INCOMING || *s == CURATED || *s == "live")
        .unwrap_or(INCOMING);

    let stage_uuid = match stage {
        INCOMING => INCOMING_STAGE,
        CURATED => CURATED_STAGE,
        _ => LIVE_STAGE,
    };

    let rows = match host::query_raw(
        "SELECT id::text AS id, title, \
                COALESCE(fields->>'field_start_date', '') AS start_date, \
                COALESCE(fields->>'field_city', '') AS city \
         FROM item \
         WHERE type = 'conference' AND stage_id = $1::uuid \
         ORDER BY fields->>'field_start_date' NULLS LAST, title \
         LIMIT $2",
        &[
            serde_json::json!(stage_uuid),
            serde_json::json!(QUEUE_PAGE_SIZE),
        ],
    ) {
        Ok(json) => json,
        Err(code) => {
            host::log(
                "error",
                "ritrovo_access",
                &format!("editorial queue query failed: host error {code}"),
            );
            return ApiResponse::error(500, "failed to read the editorial queue");
        }
    };

    let rows: Vec<serde_json::Value> = serde_json::from_str(&rows).unwrap_or_default();
    let counts = stage_counts();
    let transitions = available_transitions(stage);

    let mut body = String::new();

    if let Some(message) = notice {
        body.push_str(&format!(
            "<p><strong>{}</strong></p>\n",
            escape_html(message)
        ));
    }

    body.push_str("<p>");
    for (name, label) in [
        (INCOMING, "Incoming"),
        (CURATED, "Curated"),
        ("live", "Live"),
    ] {
        let count = counts.get(name).copied().unwrap_or(0);
        if name == stage {
            body.push_str(&format!("<strong>{label} ({count})</strong> "));
        } else {
            body.push_str(&format!(
                "<a href=\"/admin/content/editorial?stage={name}\">{label} ({count})</a> "
            ));
        }
    }
    body.push_str("</p>\n");

    if transitions.is_empty() {
        body.push_str(
            "<p>You hold no permission that moves a conference off this stage. \
             An editor promotes Incoming to Curated; a publisher publishes Curated to Live.</p>\n",
        );
    }

    if rows.is_empty() {
        body.push_str("<p>Nothing on this stage.</p>\n");
        return editorial_page("Editorial queue", &body);
    }

    body.push_str(&format!(
        "<form method=\"post\" action=\"/admin/content/editorial?stage={stage}\">\n\
         <input type=\"hidden\" name=\"_token\" value=\"{token}\">\n\
         <input type=\"hidden\" name=\"from\" value=\"{stage}\">\n\
         <table>\n<tr><th></th><th>Conference</th><th>Starts</th><th>City</th></tr>\n",
        token = escape_html(&request.csrf_token),
    ));

    for row in &rows {
        let id = row
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        if !is_uuid(id) {
            continue;
        }
        body.push_str(&format!(
            "<tr><td><input type=\"checkbox\" name=\"id\" value=\"{id}\"></td>\
             <td><a href=\"/item/{id}\">{title}</a></td><td>{start}</td><td>{city}</td></tr>\n",
            title = cell(row, "title"),
            start = cell(row, "start_date"),
            city = cell(row, "city"),
        ));
    }
    body.push_str("</table>\n");

    for transition in &transitions {
        body.push_str(&format!(
            "<button type=\"submit\" name=\"to\" value=\"{to}\">{label}</button>\n",
            to = transition.to,
            label = escape_html(transition.label),
        ));
    }
    body.push_str("</form>\n");

    if transitions.iter().any(|t| t.substituted) {
        body.push_str(
            "<p><small>Which of these controls you see is decided by a kernel permission \
             standing in for the one the brief names, because the kernel does not dispatch \
             <code>tap_perm</code> and so cannot be told about a plugin's permissions. \
             See FRICTION.md, G-PERM-TAP-NOT-DISPATCHED.</small></p>\n",
        );
    }

    editorial_page("Editorial queue", &body)
}

/// How many conferences sit on each stage.
fn stage_counts() -> std::collections::HashMap<String, i64> {
    let mut counts = std::collections::HashMap::new();
    let Ok(json) = host::query_raw(
        "SELECT sc.machine_name AS machine_name, count(i.id) AS total \
         FROM stage_config sc \
         LEFT JOIN item i ON i.stage_id = sc.tag_id AND i.type = 'conference' \
         GROUP BY sc.machine_name",
        &[],
    ) else {
        return counts;
    };
    let rows: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap_or_default();
    for row in rows {
        let name = row
            .get("machine_name")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string();
        let total = row
            .get("total")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0);
        counts.insert(name, total);
    }
    counts
}

/// Read one string field out of a row, escaped, or a placeholder.
fn cell(row: &serde_json::Value, key: &str) -> String {
    row.get(key)
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .map_or_else(|| "&mdash;".to_string(), escape_html)
}

/// Move the selected conferences from one stage to another.
///
/// The transition is validated against [`TRANSITIONS`] before anything is
/// written: an arbitrary `from`/`to` pair in the form body is refused even if
/// both name real stages, so the workflow is enforced rather than suggested.
fn editorial_move(request: &ApiRequest) -> ApiResponse {
    let form = parse_form(&request.body);

    let field = |name: &str| {
        form.iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
            .unwrap_or_default()
    };
    let from = field("from").to_string();
    let to = field("to").to_string();

    let ids: Vec<String> = form
        .iter()
        .filter(|(k, _)| k == "id")
        .map(|(_, v)| v.clone())
        .filter(|id| is_uuid(id))
        .collect();

    let Some(transition) = TRANSITIONS.iter().find(|t| t.from == from && t.to == to) else {
        return editorial_queue(
            request,
            Some(&format!(
                "No editorial transition goes from '{from}' to '{to}'."
            )),
        );
    };

    // Re-check the permission here rather than trusting the rendered form. The
    // buttons a viewer cannot use are not drawn, and a form body can say
    // anything, so the gate that matters is this one.
    if !viewer_may(transition.allowed_by) {
        return ApiResponse::error(403, "You may not make that editorial transition.");
    }

    if ids.is_empty() {
        return editorial_queue(request, Some("Nothing was selected."));
    }

    let mut moved = 0u64;
    let mut failed = 0usize;
    for id in &ids {
        // Bound parameters, and `WHERE stage_id` as well as `WHERE id`, so a
        // stale form cannot move an item that has since left the stage the
        // editor was looking at.
        match host::execute_raw(
            &format!(
                "UPDATE item SET stage_id = $1::uuid, changed = {CHANGED_NOW} \
                 WHERE id = $2::uuid AND type = 'conference' AND stage_id = $3::uuid"
            ),
            &[
                serde_json::json!(transition.to_uuid),
                serde_json::json!(id),
                serde_json::json!(stage_uuid_of(transition.from)),
            ],
        ) {
            Ok(count) => moved += count,
            Err(code) => {
                failed += 1;
                host::log(
                    "error",
                    "ritrovo_access",
                    &format!("stage move failed for {id}: host error {code}"),
                );
            }
        }
    }

    let skipped = ids.len() as u64 - moved - failed as u64;
    let mut notice = format!(
        "Moved {moved} conference(s) from {} to {}.",
        transition.from, transition.to
    );
    if skipped > 0 {
        notice.push_str(&format!(
            " {skipped} had already left {} and were left alone.",
            transition.from
        ));
    }
    if failed > 0 {
        notice.push_str(&format!(" {failed} failed; see the log."));
    }
    notice.push_str(
        " A conference promoted to Live can take up to the item cache's lifetime to appear \
         on its own page, because a plugin cannot invalidate the kernel's item cache.",
    );

    editorial_queue(request, Some(&notice))
}

/// The uuid of a stage, by machine name.
fn stage_uuid_of(machine_name: &str) -> &'static str {
    match machine_name {
        INCOMING => INCOMING_STAGE,
        CURATED => CURATED_STAGE,
        _ => LIVE_STAGE,
    }
}

/// The `changed` timestamp, written by the database rather than by this plugin.
///
/// Not a parameter, and deliberately not `SystemTime::now()`. A plugin compiled
/// for `wasm32-wasip1` that reads the clock imports `clock_time_get`, which the
/// kernel's linker does not provide: the module then fails to instantiate, and
/// every tap on it fails with "failed to instantiate plugin", naming neither the
/// import nor the reason (`G-NO-CLOCK-AND-NO-DIAGNOSIS`). Taking the time from
/// Postgres avoids the import altogether, and is the better answer anyway: the
/// row's other timestamps come from the same clock.
const CHANGED_NOW: &str = "extract(epoch from now())::bigint";

// ═══ The demo's editorial users ══════════════════════════════════════

/// The demo's editorial users: name, role, email, and password hash.
///
/// # These passwords are published on purpose
///
/// The point of these three accounts is that a reader logs in as each one and
/// sees a different site, so the credentials are in `docs/EDITORIAL.md` next to
/// the walkthrough that uses them. They are:
///
/// | user | password | role |
/// | :- | :- | :- |
/// | `editor_alice` | `ritrovo-editor-demo` | editor |
/// | `publisher_bob` | `ritrovo-publisher-demo` | publisher |
/// | `viewer_carol` | `ritrovo-viewer-demo` | viewer |
///
/// The hashes are Argon2id, RFC 9106's second recommended parameter set
/// (m=65536, t=3, p=4), the same the kernel hashes with. The kernel verifies by
/// parsing the PHC string, so these verify exactly as a hash it wrote would.
///
/// # Why the hashes are here rather than registered
///
/// The brief's recipe is to create these through the registration form and then
/// activate them by SQL, and that is what this did first. It does not work: the
/// kernel counts one registration against the rate limit TWICE, once in the
/// middleware and once in the handler, so a limit of three per hour admits ONE
/// account per hour and the second POST is refused on a completely clean
/// install. Verified on a fresh stack: the first registration succeeds, the
/// second is refused by the handler, the third by the middleware, each with a
/// different error shape. `FRICTION.md`, `G-RATE-LIMIT-COUNTED-TWICE`.
///
/// Creating the rows here needs no HTTP, no ordering against the registration
/// setting, and no second server start. A reader who wants to see registration
/// itself registers a fourth account by hand; `docs/EDITORIAL.md` says so, and
/// says that only one will succeed per hour and why.
const DEMO_USERS: &[DemoUser] = &[
    DemoUser {
        name: "editor_alice",
        role: "editor",
        mail: "editor_alice@ritrovo.example",
        pass: "$argon2id$v=19$m=65536,t=3,p=4$tJH+13r6X0ydF13JMlUrng$\
               BazcHjuGCcQkPkOS401WbT/ILJ0afd2nS0KIrVg2PMw",
    },
    DemoUser {
        name: "publisher_bob",
        role: "publisher",
        mail: "publisher_bob@ritrovo.example",
        pass: "$argon2id$v=19$m=65536,t=3,p=4$BhDJHybsUah4eDEPrqAnPw$\
               yBTn/f9IOdyxQ2Kuof8ee/xVMLvhNLdhShl/nLfKQuo",
    },
    DemoUser {
        name: "viewer_carol",
        role: "viewer",
        mail: "viewer_carol@ritrovo.example",
        pass: "$argon2id$v=19$m=65536,t=3,p=4$5Hs4HMy86IllRQuOVfhBQA$\
               otVfNkDz09huegk3fTpuNBN8dbAJd1dzYE2o7iZmQfs",
    },
];

/// One of the demo's editorial users.
struct DemoUser {
    name: &'static str,
    role: &'static str,
    mail: &'static str,
    pass: &'static str,
}

/// Create the demo's editorial users, active, with their roles.
///
/// # Why a plugin does this
///
/// The kernel has no route and no CLI that creates an active user with a role.
/// Registration creates one with `status = 0` and no role, there is no admin UI
/// for user-to-role assignment, and `trovato user` has a single subcommand,
/// `reset-password`. The brief says as much itself ("Role assignment via SQL").
/// This is that SQL, in the one place in Ritrovo that can run it.
///
/// # Why `tap_install`
///
/// It fires on the first server start after the plugin is enabled, which is the
/// last step of `scripts/demo-bootstrap.sh`, so the three accounts work the
/// moment the demo is up. Cron would leave them unusable for an interval, and
/// nothing else runs later: the bootstrap's last line is an `exec`.
///
/// Idempotent three times over: the insert is conditional on the name not
/// existing, the role insert is `ON CONFLICT DO NOTHING`, and `tap_install`
/// itself fires once per install.
#[plugin_tap]
pub fn tap_install() -> serde_json::Value {
    let mut created = 0u64;
    let mut assigned = 0u64;

    for user in DEMO_USERS {
        // `status = 1` and no verification row: these accounts exist to be
        // logged into. A user created here skips the email verification a
        // registered user would wait for, which is the point of creating them
        // here rather than registering them.
        match host::execute_raw(
            "INSERT INTO users (id, name, pass, mail, status, is_admin) \
             SELECT gen_random_uuid(), $1, $2, $3, 1, false \
             WHERE NOT EXISTS (SELECT 1 FROM users WHERE lower(name) = lower($1))",
            &[
                serde_json::json!(user.name),
                serde_json::json!(user.pass),
                serde_json::json!(user.mail),
            ],
        ) {
            Ok(count) => created += count,
            Err(code) => host::log(
                "error",
                "ritrovo_access",
                &format!("could not create {}: host error {code}", user.name),
            ),
        }

        match host::execute_raw(
            "INSERT INTO user_roles (user_id, role_id) \
             SELECT u.id, r.id FROM users u, roles r \
             WHERE u.name = $1 AND r.name = $2 \
             ON CONFLICT (user_id, role_id) DO NOTHING",
            &[serde_json::json!(user.name), serde_json::json!(user.role)],
        ) {
            Ok(count) => assigned += count,
            Err(code) => host::log(
                "error",
                "ritrovo_access",
                &format!(
                    "could not give {} the {} role: host error {code}",
                    user.name, user.role
                ),
            ),
        }
    }

    host::log(
        "info",
        "ritrovo_access",
        &format!("demo editorial users: {created} created, {assigned} role(s) assigned"),
    );

    serde_json::json!({"created": created, "roles_assigned": assigned})
}

// ═══ The demo seed ═══════════════════════════════════════════════════

/// Variable recording that the demo's opening editorial state has been set up.
const SEED_FLAG: &str = "ritrovo_access.demo_seed_done";

/// How many conferences the seed publishes to Live.
const SEED_TO_LIVE: i64 = 60;

/// How many it leaves on Curated, for the editorial queue to have work in it.
const SEED_TO_CURATED: i64 = 20;

/// Give the demo an opening editorial state, once.
///
/// # Why this exists
///
/// The importer lands every conference on Incoming, which is correct and which
/// would leave the public site empty, because a demo has no editors: nobody is
/// going to log in and promote five thousand conferences before a visitor looks
/// at it. This stands in for the editorial work that has not happened yet,
/// publishing the soonest upcoming conferences and leaving a smaller set on
/// Curated so the editorial queue has something in it at every stage.
///
/// It is a seed, not a feature, and it is deliberately the same UPDATE the
/// editorial screen runs: if this works and the screen does not, the difference
/// is the screen, not the stage machinery.
///
/// # Why cron and not `tap_install`
///
/// `tap_install` fires at the first server start after the plugin is enabled,
/// which is the same start on which the importer begins fetching. There are no
/// conferences yet at that moment. Cron runs later and repeatedly, so the seed
/// waits for content to exist and then runs exactly once.
///
/// # Why it is safe to run repeatedly
///
/// Guarded by a variable, and by its own precondition: it does nothing until
/// something has been imported. An operator who wants the demo back in its
/// opening state clears the variable.
#[plugin_tap]
pub fn tap_cron(_input: CronInput) -> serde_json::Value {
    if host::variables_get(SEED_FLAG, "0").unwrap_or_else(|_| "0".to_string()) == "1" {
        return serde_json::json!({"status": "skipped", "reason": "already_seeded"});
    }

    // Wait for enough of the import to have arrived, not merely for the first
    // batch. The importer works forward from 2015, so the early cron cycles see
    // thousands of conferences that finished years ago and almost no upcoming
    // ones. Seeding then published the thirteen upcoming conferences that
    // happened to exist and left nothing for Curated. The dataset carries on the
    // order of 150 upcoming conferences, so this threshold is reached on a
    // normal run and the seed simply waits until it is.
    let upcoming = count_upcoming_on_stage(INCOMING_STAGE);
    if upcoming < SEED_TO_LIVE + SEED_TO_CURATED {
        return serde_json::json!({
            "status": "waiting",
            "reason": "not_enough_upcoming_imported_yet",
            "upcoming": upcoming,
        });
    }

    let published = promote_soonest(INCOMING_STAGE, LIVE_STAGE, SEED_TO_LIVE);
    let curated = promote_soonest(INCOMING_STAGE, CURATED_STAGE, SEED_TO_CURATED);

    if let Err(code) = host::variables_set(SEED_FLAG, "1") {
        // The seed has already run; failing to record that would run it again
        // next cycle and publish another batch, so say so loudly.
        host::log(
            "error",
            "ritrovo_access",
            &format!("demo seed ran but could not set {SEED_FLAG}: host error {code}"),
        );
    }

    host::log(
        "info",
        "ritrovo_access",
        &format!("demo seed: published {published}, curated {curated}, of {upcoming} upcoming"),
    );

    serde_json::json!({
        "status": "seeded",
        "published": published,
        "curated": curated,
    })
}

/// How many conferences on one stage have not happened yet.
fn count_upcoming_on_stage(stage_uuid: &str) -> i64 {
    let Ok(json) = host::query_raw(
        "SELECT count(*) AS total FROM item \
         WHERE type = 'conference' AND stage_id = $1::uuid \
           AND fields->>'field_start_date' >= to_char(now(), 'YYYY-MM-DD')",
        &[serde_json::json!(stage_uuid)],
    ) else {
        return 0;
    };
    serde_json::from_str::<Vec<serde_json::Value>>(&json)
        .ok()
        .and_then(|rows| rows.first().and_then(|r| r.get("total")?.as_i64()))
        .unwrap_or(0)
}

/// Move the soonest upcoming conferences from one stage to another.
///
/// "Soonest upcoming" rather than "first N": a demo whose front page is full of
/// conferences that finished two years ago is a worse demo. Conferences with no
/// start date sort last and are not picked while dated ones remain.
fn promote_soonest(from_uuid: &str, to_uuid: &str, limit: i64) -> u64 {
    host::execute_raw(
        &format!(
            "UPDATE item SET stage_id = $1::uuid, changed = {CHANGED_NOW} \
             WHERE id IN ( \
               SELECT id FROM item \
               WHERE type = 'conference' \
                 AND stage_id = $2::uuid \
                 AND fields->>'field_start_date' >= to_char(now(), 'YYYY-MM-DD') \
               ORDER BY fields->>'field_start_date' \
               LIMIT $3 \
             )"
        ),
        &[
            serde_json::json!(to_uuid),
            serde_json::json!(from_uuid),
            serde_json::json!(limit),
        ],
    )
    .unwrap_or_else(|code| {
        host::log(
            "error",
            "ritrovo_access",
            &format!("demo seed promote failed: host error {code}"),
        );
        0
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn make_input(
        item_type: &str,
        stage: Option<&str>,
        permissions: &[&str],
        authenticated: bool,
    ) -> ItemAccessInput {
        make_input_op(item_type, stage, permissions, authenticated, "view")
    }

    fn make_input_op(
        item_type: &str,
        stage: Option<&str>,
        permissions: &[&str],
        authenticated: bool,
        operation: &str,
    ) -> ItemAccessInput {
        ItemAccessInput {
            item_id: Uuid::nil(),
            item_type: item_type.to_string(),
            author_id: Uuid::nil(),
            operation: operation.to_string(),
            user_id: Uuid::nil(),
            user_authenticated: authenticated,
            user_permissions: permissions.iter().map(|s| s.to_string()).collect(),
            stage_id: None,
            stage_machine_name: stage.map(|s| s.to_string()),
        }
    }

    #[test]
    fn permissions_count() {
        let perms = __inner_tap_perm();
        assert_eq!(perms.len(), 7);
    }

    #[test]
    fn permission_names_unique() {
        let perms = __inner_tap_perm();
        let names: std::collections::HashSet<&str> =
            perms.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names.len(), perms.len(), "duplicate permission names");
    }

    #[test]
    fn neutral_for_non_conference() {
        let input = make_input("blog", Some("incoming"), &[], true);
        assert_eq!(__inner_tap_item_access(input), AccessResult::Neutral);
    }

    #[test]
    fn neutral_for_live_stage() {
        let input = make_input("conference", Some("live"), &["access content"], true);
        assert_eq!(__inner_tap_item_access(input), AccessResult::Neutral);
    }

    #[test]
    fn neutral_for_no_stage() {
        let input = make_input("conference", None, &["access content"], true);
        assert_eq!(__inner_tap_item_access(input), AccessResult::Neutral);
    }

    #[test]
    fn deny_incoming_without_permission() {
        let input = make_input("conference", Some("incoming"), &["access content"], true);
        assert_eq!(__inner_tap_item_access(input), AccessResult::Deny);
    }

    #[test]
    fn grant_incoming_with_view_permission() {
        let input = make_input(
            "conference",
            Some("incoming"),
            &["access content", "view incoming conferences"],
            true,
        );
        assert_eq!(__inner_tap_item_access(input), AccessResult::Grant);
    }

    #[test]
    fn grant_incoming_with_edit_permission() {
        let input = make_input(
            "conference",
            Some("incoming"),
            &["access content", "edit conferences"],
            true,
        );
        assert_eq!(__inner_tap_item_access(input), AccessResult::Grant);
    }

    #[test]
    fn deny_curated_without_permission() {
        let input = make_input("conference", Some("curated"), &["access content"], true);
        assert_eq!(__inner_tap_item_access(input), AccessResult::Deny);
    }

    #[test]
    fn grant_curated_with_view_permission() {
        let input = make_input(
            "conference",
            Some("curated"),
            &["access content", "view curated conferences"],
            true,
        );
        assert_eq!(__inner_tap_item_access(input), AccessResult::Grant);
    }

    // --- Operation-aware tests (edit/delete/update) ---

    #[test]
    fn deny_edit_incoming_with_only_view_permission() {
        let input = make_input_op(
            "conference",
            Some("incoming"),
            &["access content", "view incoming conferences"],
            true,
            "edit",
        );
        assert_eq!(__inner_tap_item_access(input), AccessResult::Deny);
    }

    #[test]
    fn grant_edit_incoming_with_edit_permission() {
        let input = make_input_op(
            "conference",
            Some("incoming"),
            &["access content", "edit conferences"],
            true,
            "edit",
        );
        assert_eq!(__inner_tap_item_access(input), AccessResult::Grant);
    }

    #[test]
    fn deny_delete_curated_with_only_view_permission() {
        let input = make_input_op(
            "conference",
            Some("curated"),
            &["access content", "view curated conferences"],
            true,
            "delete",
        );
        assert_eq!(__inner_tap_item_access(input), AccessResult::Deny);
    }

    #[test]
    fn grant_delete_curated_with_edit_permission() {
        let input = make_input_op(
            "conference",
            Some("curated"),
            &["access content", "edit conferences"],
            true,
            "delete",
        );
        assert_eq!(__inner_tap_item_access(input), AccessResult::Grant);
    }

    #[test]
    fn deny_update_live_without_edit_permission() {
        let input = make_input_op(
            "conference",
            Some("live"),
            &["access content"],
            true,
            "update",
        );
        assert_eq!(__inner_tap_item_access(input), AccessResult::Deny);
    }

    #[test]
    fn grant_update_live_with_edit_permission() {
        let input = make_input_op(
            "conference",
            Some("live"),
            &["access content", "edit conferences"],
            true,
            "update",
        );
        assert_eq!(__inner_tap_item_access(input), AccessResult::Grant);
    }

    #[test]
    fn neutral_edit_non_conference() {
        let input = make_input_op(
            "blog",
            Some("incoming"),
            &["edit conferences"],
            true,
            "edit",
        );
        assert_eq!(__inner_tap_item_access(input), AccessResult::Neutral);
    }

    // ── The aggregation table ────────────────────────────────────────
    //
    // The tests above check one decision each. This checks the whole table in
    // one place, so that a change to the rules shows up as a diff of the table
    // rather than as a scatter of individually-plausible edits.
    //
    // Columns: who, which stage, which operation, what this tap must answer.
    // "who" is the permission set, named for the role in demo/config that holds
    // it. The kernel permissions are the ones those roles really carry; the
    // `conference` permissions are the ones the brief asks for and no role can
    // hold until `tap_perm` is dispatched. Both are here because the tap accepts
    // either, and must keep accepting either.

    /// The permission sets the five roles really resolve to.
    fn anonymous_perms() -> Vec<&'static str> {
        vec!["access content"]
    }
    fn viewer_perms() -> Vec<&'static str> {
        vec!["access content"]
    }
    fn editor_perms() -> Vec<&'static str> {
        vec!["access content", "edit any content"]
    }
    fn publisher_perms() -> Vec<&'static str> {
        vec!["access content", "edit any content", "delete any content"]
    }
    /// What an editor would hold if `tap_perm` were dispatched.
    fn brief_editor_perms() -> Vec<&'static str> {
        vec!["access content", "edit conferences"]
    }
    /// A reader granted one internal stage and nothing else.
    fn incoming_reader_perms() -> Vec<&'static str> {
        vec!["access content", "view incoming conferences"]
    }

    #[test]
    fn the_aggregation_table_holds() {
        use AccessResult::{Deny, Grant, Neutral};

        // Factored into an alias because the row is six columns wide and
        // clippy rightly refuses the bare tuple type inline.
        type Case = (
            &'static str,
            Vec<&'static str>,
            &'static str,
            Option<&'static str>,
            &'static str,
            AccessResult,
        );

        // (who, permissions, item type, stage, operation, expected)
        let table: &[Case] = &[
            // Live is the kernel's business, not this plugin's: it answers
            // Neutral so the published-content fast path and the role fallback
            // decide. In practice the kernel never even asks for a published
            // Live item viewed by someone holding `access content`.
            (
                "viewer",
                viewer_perms(),
                "conference",
                Some("live"),
                "view",
                Neutral,
            ),
            (
                "editor",
                editor_perms(),
                "conference",
                Some("live"),
                "view",
                Neutral,
            ),
            (
                "viewer",
                viewer_perms(),
                "conference",
                None,
                "view",
                Neutral,
            ),
            // An unknown stage is somebody else's; say nothing.
            (
                "editor",
                editor_perms(),
                "conference",
                Some("legal_review"),
                "view",
                Neutral,
            ),
            // Another plugin's content type is never this plugin's business,
            // whatever the stage or the operation.
            (
                "editor",
                editor_perms(),
                "blog",
                Some("incoming"),
                "view",
                Neutral,
            ),
            (
                "editor",
                editor_perms(),
                "speaker",
                Some("curated"),
                "edit",
                Neutral,
            ),
            // Viewing an internal stage: denied to a plain reader, granted to an
            // editor, and granted to a reader holding just that stage's
            // permission — and only that stage's.
            (
                "viewer",
                viewer_perms(),
                "conference",
                Some("incoming"),
                "view",
                Deny,
            ),
            (
                "viewer",
                viewer_perms(),
                "conference",
                Some("curated"),
                "view",
                Deny,
            ),
            (
                "editor",
                editor_perms(),
                "conference",
                Some("incoming"),
                "view",
                Grant,
            ),
            (
                "editor",
                editor_perms(),
                "conference",
                Some("curated"),
                "view",
                Grant,
            ),
            (
                "publisher",
                publisher_perms(),
                "conference",
                Some("incoming"),
                "view",
                Grant,
            ),
            (
                "publisher",
                publisher_perms(),
                "conference",
                Some("curated"),
                "view",
                Grant,
            ),
            (
                "brief editor",
                brief_editor_perms(),
                "conference",
                Some("incoming"),
                "view",
                Grant,
            ),
            (
                "brief editor",
                brief_editor_perms(),
                "conference",
                Some("curated"),
                "view",
                Grant,
            ),
            (
                "incoming reader",
                incoming_reader_perms(),
                "conference",
                Some("incoming"),
                "view",
                Grant,
            ),
            (
                "incoming reader",
                incoming_reader_perms(),
                "conference",
                Some("curated"),
                "view",
                Deny,
            ),
            // Changing anything needs an editor's permission, on every stage,
            // including Live. Seeing an internal stage is not permission to
            // change what is on it.
            (
                "viewer",
                viewer_perms(),
                "conference",
                Some("live"),
                "edit",
                Deny,
            ),
            (
                "viewer",
                viewer_perms(),
                "conference",
                Some("incoming"),
                "edit",
                Deny,
            ),
            (
                "incoming reader",
                incoming_reader_perms(),
                "conference",
                Some("incoming"),
                "edit",
                Deny,
            ),
            (
                "incoming reader",
                incoming_reader_perms(),
                "conference",
                Some("incoming"),
                "delete",
                Deny,
            ),
            (
                "editor",
                editor_perms(),
                "conference",
                Some("incoming"),
                "edit",
                Grant,
            ),
            (
                "editor",
                editor_perms(),
                "conference",
                Some("curated"),
                "delete",
                Grant,
            ),
            (
                "editor",
                editor_perms(),
                "conference",
                Some("live"),
                "edit",
                Grant,
            ),
            (
                "brief editor",
                brief_editor_perms(),
                "conference",
                Some("curated"),
                "edit",
                Grant,
            ),
            // Anonymous never reaches this tap for an internal stage — the
            // kernel denies first — but if it ever did, the answer is Deny.
            (
                "anonymous",
                anonymous_perms(),
                "conference",
                Some("incoming"),
                "view",
                Deny,
            ),
        ];

        let mut wrong = Vec::new();
        for (who, perms, item_type, stage, operation, expected) in table {
            let input = make_input_op(item_type, *stage, perms, *who != "anonymous", operation);
            let got = __inner_tap_item_access(input);
            if got != *expected {
                wrong.push(format!(
                    "{who} {operation} {item_type} on {}: got {got:?}, want {expected:?}",
                    stage.unwrap_or("(no stage)")
                ));
            }
        }
        assert!(
            wrong.is_empty(),
            "aggregation table wrong:\n{}",
            wrong.join("\n")
        );
    }

    // ── Field access ─────────────────────────────────────────────────

    fn field_input(
        item_type: &str,
        permissions: &[&str],
        fields: &[&str],
    ) -> FieldAccessBatchInput {
        FieldAccessBatchInput {
            user: FieldAccessUser {
                user_id: Uuid::nil(),
                authenticated: true,
                permissions: permissions.iter().map(|s| s.to_string()).collect(),
            },
            item_type: item_type.to_string(),
            operation: "view".to_string(),
            fields: fields.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn editor_notes_denied_to_a_reader_and_allowed_to_an_editor() {
        let denied = __inner_tap_field_access(field_input(
            "conference",
            &viewer_perms(),
            &[EDITOR_ONLY_FIELD],
        ));
        assert_eq!(
            denied.decisions.get(EDITOR_ONLY_FIELD),
            Some(&FieldAccessResult::Deny)
        );

        for who in [editor_perms(), publisher_perms(), brief_editor_perms()] {
            let allowed =
                __inner_tap_field_access(field_input("conference", &who, &[EDITOR_ONLY_FIELD]));
            assert_eq!(
                allowed.decisions.get(EDITOR_ONLY_FIELD),
                Some(&FieldAccessResult::Allow),
                "{who:?} should see {EDITOR_ONLY_FIELD}"
            );
        }
    }

    /// Every other field is left absent, which the kernel reads as NoOpinion.
    ///
    /// This is the test that stops a future edit from denying a whole type by
    /// accident: the tap must speak about one field and stay silent about the
    /// rest, because silence is what keeps the other fields visible.
    #[test]
    fn no_opinion_on_every_other_field() {
        let result = __inner_tap_field_access(field_input(
            "conference",
            &viewer_perms(),
            &["field_city", "field_start_date", EDITOR_ONLY_FIELD],
        ));
        assert_eq!(
            result.decisions.len(),
            1,
            "answered about more than one field"
        );
        assert!(result.decisions.contains_key(EDITOR_ONLY_FIELD));
    }

    /// The kernel asks about a subset, so the tap must answer about what it was
    /// handed rather than about what it expects to be handed.
    #[test]
    fn a_batch_without_the_field_gets_no_decisions() {
        let result = __inner_tap_field_access(field_input(
            "conference",
            &viewer_perms(),
            &["field_city", "field_country"],
        ));
        assert!(result.decisions.is_empty());
    }

    #[test]
    fn says_nothing_about_another_content_type() {
        let result =
            __inner_tap_field_access(field_input("blog", &viewer_perms(), &[EDITOR_ONLY_FIELD]));
        assert!(
            result.decisions.is_empty(),
            "this plugin must not speak about another type's fields"
        );
    }
}
