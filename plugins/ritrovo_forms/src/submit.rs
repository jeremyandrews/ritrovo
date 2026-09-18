//! "Submit a Conference": three steps, state on the server, no JavaScript.
//!
//! # The shape, and why it is this shape
//!
//! Step 1 collects what a conference is: name, dates, where. Step 2 collects the
//! call for papers and the description. Step 3 shows it all back and asks for a
//! confirmation. Each step validates before the next one renders, and the
//! answers live in a row keyed by a draft id, never in the page.
//!
//! **The browser carries an id, not the answers.** A hidden field holding the
//! accumulated conference would need no table and would let anyone submit
//! anything under anyone's name by editing it. The row is the authority and it
//! records `user_id`; every read is filtered on the signed-in user, so one
//! member's draft id is useless to another.
//!
//! # What the kernel does not provide
//!
//! - **Form state.** The kernel has a `form_state_cache` table with exactly the
//!   right columns and a cron task to expire it, written by a `FormService` no
//!   route calls and reachable through no host interface. This plugin owns its
//!   own table instead of squatting in the kernel's.
//!   `G-FORM-TAPS-UNREACHABLE`, `G-NO-FORM-STATE-HOST-API`.
//! - **A logo upload.** Step 2 has no file field. A plugin cannot accept a file:
//!   there is no file host interface, and `tap_api` hands over a body string
//!   rather than a parsed multipart request. `G-FILE-NO-HOST-API`. The review
//!   step says so rather than leaving a reader wondering.
//! - **Topics.** No field is collected, because there is nothing to collect into:
//!   the kernel has no taxonomic `FieldType`, so a topic cannot be declared, a
//!   widget cannot be rendered for it, and nothing can autocomplete against the
//!   category tree. `G-NO-CATEGORY-REFERENCE-FIELD-KIND`. An editor adds topics
//!   after the conference lands.
//! - **Landing on Incoming.** Nothing in the kernel moves an item to a
//!   non-default stage, and nothing honours the stage marked default on create.
//!   The insert binds `stage_id` in its own SQL, which is what
//!   `ritrovo_importer` already does and for the same reason.
//!   `G-NO-ITEM-STAGE-TRANSITION`, `G-DEFAULT-STAGE-IGNORED-ON-CREATE`. The
//!   cost is real: an insert that bypasses `ItemService` fires no taps
//!   (`G-ITEM-API-BYPASSES-ITEM-SERVICE`), so this submission's dates are
//!   checked here, by this form, rather than by `ritrovo_cfp`'s presave tap.

use trovato_sdk::host;
use trovato_sdk::types::{ApiRequest, ApiResponse};

use crate::dates;
use crate::web;

/// Where the form lives. One path, every step.
pub const PATH: &str = "/conferences/submit";

/// Which form the state rows belong to.
const FORM_ID: &str = "conference_submission";

/// The Incoming stage, as `demo/config/stage.*.yml` identifies it. The same
/// constant `ritrovo_access` and `ritrovo_importer` carry, for the same reason:
/// there is no host call that resolves a stage by machine name.
const INCOMING_STAGE: &str = "0193a5a0-0000-7000-8000-000000000002";

/// Longest description accepted, in characters.
const MAX_DESCRIPTION: usize = 5_000;

/// Longest single-line value accepted, in characters.
const MAX_LINE: usize = 200;

/// Every answer the three steps collect.
///
/// One flat struct rather than one per step: the review step needs all of it at
/// once, and a shape that changes per step is a shape that has to be merged.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Answers {
    pub name: String,
    pub url: String,
    pub start_date: String,
    pub end_date: String,
    pub city: String,
    pub country: String,
    pub online: bool,
    pub cfp_url: String,
    pub cfp_end_date: String,
    pub description: String,
}

impl Answers {
    /// Read the step-1 answers out of a posted body, keeping the rest.
    fn read_step1(&mut self, body: &str) {
        self.name = web::normalize(&web::field(body, "name"));
        self.url = web::normalize(&web::field(body, "url"));
        self.start_date = web::normalize(&web::field(body, "start_date"));
        self.end_date = web::normalize(&web::field(body, "end_date"));
        self.city = web::normalize(&web::field(body, "city"));
        self.country = web::normalize(&web::field(body, "country"));
        self.online = web::present(body, "online");
    }

    /// Read the step-2 answers out of a posted body, keeping the rest.
    fn read_step2(&mut self, body: &str) {
        self.cfp_url = web::normalize(&web::field(body, "cfp_url"));
        self.cfp_end_date = web::normalize(&web::field(body, "cfp_end_date"));
        self.description = web::normalize(&web::field(body, "description"));
    }

    /// As stored between steps.
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "url": self.url,
            "start_date": self.start_date,
            "end_date": self.end_date,
            "city": self.city,
            "country": self.country,
            "online": self.online,
            "cfp_url": self.cfp_url,
            "cfp_end_date": self.cfp_end_date,
            "description": self.description,
        })
    }

    /// As read back. A missing key is an empty answer, so a row written by an
    /// older version of this form still opens.
    fn from_json(value: &serde_json::Value) -> Self {
        let text = |key: &str| {
            value
                .get(key)
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string()
        };
        Self {
            name: text("name"),
            url: text("url"),
            start_date: text("start_date"),
            end_date: text("end_date"),
            city: text("city"),
            country: text("country"),
            online: value
                .get("online")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
            cfp_url: text("cfp_url"),
            cfp_end_date: text("cfp_end_date"),
            description: text("description"),
        }
    }

    /// The conference item's `fields`, as the kernel stores them.
    ///
    /// Flat values, matching what the admin stack writes and what
    /// `FormBuilder` reads back, so a submitted conference opens in the edit
    /// form like an imported one.
    fn to_fields(&self) -> serde_json::Value {
        let mut fields = serde_json::Map::new();
        let mut put = |key: &str, value: &str| {
            if !value.is_empty() {
                fields.insert(
                    key.to_string(),
                    serde_json::Value::String(value.to_string()),
                );
            }
        };
        put("field_url", &self.url);
        put("field_start_date", &self.start_date);
        put("field_end_date", &self.end_date);
        put("field_city", &self.city);
        put("field_country", &self.country);
        put("field_cfp_url", &self.cfp_url);
        put("field_cfp_end_date", &self.cfp_end_date);
        put("field_description", &self.description);
        fields.insert(
            "field_online".to_string(),
            serde_json::Value::String(if self.online { "1" } else { "0" }.to_string()),
        );
        serde_json::Value::Object(fields)
    }
}

// ─── Validation ──────────────────────────────────────────────────────

/// Everything wrong with the step-1 answers.
///
/// All of them at once rather than the first: somebody who has filled in a form
/// should not discover its problems one round-trip at a time.
pub fn step1_errors(answers: &Answers) -> Vec<String> {
    let mut errors = Vec::new();

    if answers.name.is_empty() {
        errors.push("Give the conference a name.".to_string());
    } else if answers.name.chars().count() > MAX_LINE {
        errors.push(format!("The name is longer than {MAX_LINE} characters."));
    }

    if !answers.url.is_empty() && !looks_like_url(&answers.url) {
        errors.push("The website address should start with http:// or https://.".to_string());
    }

    for (label, value) in [
        ("start date", &answers.start_date),
        ("end date", &answers.end_date),
    ] {
        if value.is_empty() {
            errors.push(format!("Give the conference a {label}."));
        } else if !dates::is_valid(value) {
            errors.push(format!("The {label} should look like 2027-05-01."));
        }
    }

    if dates::is_after(&answers.start_date, &answers.end_date) {
        errors.push("The conference starts after it ends.".to_string());
    }

    // An online-only event has no city to give, and an in-person one without a
    // country cannot be found by anyone browsing by country, which is one of
    // the two ways this site is browsed.
    if !answers.online && answers.country.is_empty() {
        errors.push("Give a country, or mark the conference as online.".to_string());
    }

    errors
}

/// Everything wrong with the step-2 answers.
pub fn step2_errors(answers: &Answers) -> Vec<String> {
    let mut errors = Vec::new();

    if !answers.cfp_url.is_empty() && !looks_like_url(&answers.cfp_url) {
        errors.push("The CFP address should start with http:// or https://.".to_string());
    }

    if !answers.cfp_end_date.is_empty() {
        if !dates::is_valid(&answers.cfp_end_date) {
            errors.push("The CFP closing date should look like 2027-03-01.".to_string());
        } else if dates::is_after(&answers.cfp_end_date, &answers.end_date) {
            // The brief's CFP rule, enforced here because here is where
            // something is allowed to say no.
            //
            // `ritrovo_cfp::date_complaint` is the same rule and is NOT called
            // from here: the two plugins are separate WASM modules with no
            // shared code, and the kernel's cross-plugin `invoke` would make
            // this check vanish whenever `ritrovo_cfp` happened to be disabled.
            // A validation rule that silently stops is worse than one written
            // twice. Keep the two in step.
            errors.push(
                "The call for papers closes after the conference ends. \
                 A CFP cannot close after the event it is for."
                    .to_string(),
            );
        }
    }

    if answers.description.chars().count() > MAX_DESCRIPTION {
        errors.push(format!(
            "The description is longer than {MAX_DESCRIPTION} characters."
        ));
    }

    errors
}

/// Whether a value is an address this form will store.
///
/// Scheme only. Anything more would be a URL parser written inside a WASM
/// module to reject strings a person can see are wrong, and the value is
/// rendered escaped in an `href` either way.
fn looks_like_url(value: &str) -> bool {
    (value.starts_with("http://") || value.starts_with("https://"))
        && value.len() > "https://".len()
        && !value.contains(char::is_whitespace)
}

// ─── Serving ─────────────────────────────────────────────────────────

/// `GET /conferences/submit`, with an optional `?draft=<id>&step=<n>`.
///
/// No draft: step 1, empty. With one: that step, filled in from the row, which
/// is what the review step's "Change" links use and what a person's Back button
/// lands on.
pub fn show(request: &ApiRequest) -> ApiResponse {
    let Some(draft) = request.query.get("draft") else {
        return page(1, "", &Answers::default(), &[], request);
    };

    let Some((reached, answers)) = load(&request.user_id, draft) else {
        // A draft id that names no row of this member's is not an error worth
        // explaining: it is an expired draft, or somebody else's. Start again.
        return page(1, "", &Answers::default(), &[], request);
    };

    let wanted = request
        .query
        .get("step")
        .and_then(|s| s.parse::<i16>().ok())
        .unwrap_or(1);
    // Never past what has been answered. A person cannot reach the review step
    // by typing a number into the address bar.
    let step = wanted.clamp(1, reached + 1);
    page(step, draft, &answers, &[], request)
}

/// `POST /conferences/submit`: validate the step that was submitted, advance.
pub fn post(request: &ApiRequest) -> ApiResponse {
    let step = web::field(&request.body, "step")
        .parse::<i16>()
        .unwrap_or(0);
    let draft = web::field(&request.body, "draft");

    match step {
        1 => submit_step1(request, &draft),
        2 => submit_step2(request, &draft),
        3 => confirm(request, &draft),
        _ => refuse(request, "That form step does not exist. Start again below."),
    }
}

/// Step 1 posted: validate, store, and render step 2.
fn submit_step1(request: &ApiRequest, draft: &str) -> ApiResponse {
    // Step 1 may start a draft or revise one, so an unknown id is not fatal
    // here: it becomes a new draft.
    let mut answers = load(&request.user_id, draft)
        .map(|(_, answers)| answers)
        .unwrap_or_default();
    answers.read_step1(&request.body);

    let errors = step1_errors(&answers);
    if !errors.is_empty() {
        return page_with_status(422, 1, draft, &answers, &errors, request);
    }

    let Some(id) = store(&request.user_id, draft, 1, &answers) else {
        return failed(request, 1, draft, &answers);
    };
    page(2, &id, &answers, &[], request)
}

/// Step 2 posted: validate, store, and render the review.
fn submit_step2(request: &ApiRequest, draft: &str) -> ApiResponse {
    // Here an unknown draft IS fatal: there is no step 1 behind it, so
    // accepting this would submit a conference with no name and no dates.
    let Some((reached, mut answers)) = load(&request.user_id, draft) else {
        return refuse(
            request,
            "That submission was not found, or has expired. Start again below.",
        );
    };
    if reached < 1 {
        return refuse(request, "Fill in the conference details first.");
    }
    answers.read_step2(&request.body);

    let errors = step2_errors(&answers);
    if !errors.is_empty() {
        return page_with_status(422, 2, draft, &answers, &errors, request);
    }

    if store(&request.user_id, draft, 2, &answers).is_none() {
        return failed(request, 2, draft, &answers);
    }
    page(3, draft, &answers, &[], request)
}

/// The review confirmed: create the conference, and say what happens next.
fn confirm(request: &ApiRequest, draft: &str) -> ApiResponse {
    let Some((reached, answers)) = load(&request.user_id, draft) else {
        return refuse(
            request,
            "That submission was not found, or has expired. Start again below.",
        );
    };
    // Both steps, in order. This is what makes "skip to the end" impossible:
    // the row records how far it got, and the row is on the server.
    if reached < 2 {
        return refuse(
            request,
            "That submission is not finished. Fill in both steps first.",
        );
    }

    // Validated once more at the point of use. The answers were checked when
    // they were typed, and between then and now they have been through a
    // database; a rule worth enforcing is worth enforcing where it takes effect.
    let errors: Vec<String> = step1_errors(&answers)
        .into_iter()
        .chain(step2_errors(&answers))
        .collect();
    if !errors.is_empty() {
        return page_with_status(422, 1, draft, &answers, &errors, request);
    }

    let Some(item_id) = create_incoming(&request.user_id, &answers) else {
        host::log(
            "error",
            "ritrovo_forms",
            &format!(
                "could not create a submitted conference for {}",
                request.user_id
            ),
        );
        return page_with_status(
            500,
            3,
            draft,
            &answers,
            &["The conference could not be saved. Please try again.".to_string()],
            request,
        );
    };

    // The draft has become a conference. Leaving the row would offer a second
    // "Submit" on the same answers.
    discard(&request.user_id, draft);

    host::log(
        "info",
        "ritrovo_forms",
        &format!("conference {item_id} submitted by {}", request.user_id),
    );
    ApiResponse::themed("Thank you", thanks_html(&answers))
}

/// A refusal that is still a usable page: says what happened, offers the start.
fn refuse(request: &ApiRequest, message: &str) -> ApiResponse {
    ApiResponse::themed_with_status(
        422,
        "Submit a Conference",
        format!(
            r#"<div class="ritrovo-errors"><p>{}</p></div>{}"#,
            web::escape(message),
            step1_html("", &Answers::default(), &request.csrf_token)
        ),
    )
}

/// A write that did not land.
fn failed(request: &ApiRequest, step: i16, draft: &str, answers: &Answers) -> ApiResponse {
    host::log(
        "error",
        "ritrovo_forms",
        &format!(
            "could not save submission step {step} for {}",
            request.user_id
        ),
    );
    page_with_status(
        500,
        step,
        draft,
        answers,
        &["Your answers could not be saved. Please try again.".to_string()],
        request,
    )
}

// ─── The state rows ──────────────────────────────────────────────────

/// Load a draft belonging to this member: how far it got, and its answers.
///
/// `user_id` is part of the query rather than checked afterwards, so a draft id
/// alone is never enough.
fn load(user_id: &str, draft: &str) -> Option<(i16, Answers)> {
    if draft.is_empty() {
        return None;
    }
    let raw = host::query_raw(
        "SELECT step, state FROM ritrovo_form_state \
         WHERE id = $1::uuid AND user_id = $2::uuid AND form_id = $3",
        &[
            serde_json::json!(draft),
            serde_json::json!(user_id),
            serde_json::json!(FORM_ID),
        ],
    )
    .ok()?;

    let row = serde_json::from_str::<Vec<serde_json::Value>>(&raw)
        .ok()?
        .into_iter()
        .next()?;
    let step = row.get("step")?.as_i64()? as i16;
    let state = row.get("state")?;
    // The column is JSONB; depending on the driver it arrives as an object or
    // as a string holding one.
    let state = match state.as_str() {
        Some(text) => serde_json::from_str(text).ok()?,
        None => state.clone(),
    };
    Some((step, Answers::from_json(&state)))
}

/// Write a draft's answers, returning its id.
///
/// Creates the row when `draft` names none. `step` only ever moves forward:
/// going back to step 1 from the review and saving again must not forget that
/// step 2 was answered, or the review would become unreachable.
fn store(user_id: &str, draft: &str, step: i16, answers: &Answers) -> Option<String> {
    let state = answers.to_json().to_string();

    if !draft.is_empty() && load(user_id, draft).is_some() {
        let updated = host::execute_raw(
            "UPDATE ritrovo_form_state \
             SET state = $1::jsonb, \
                 step = GREATEST(step, $2), \
                 updated = EXTRACT(EPOCH FROM NOW())::bigint \
             WHERE id = $3::uuid AND user_id = $4::uuid",
            &[
                serde_json::json!(state),
                serde_json::json!(step),
                serde_json::json!(draft),
                serde_json::json!(user_id),
            ],
        );
        return matches!(updated, Ok(1)).then(|| draft.to_string());
    }

    let id = new_uuid()?;
    let inserted = host::execute_raw(
        "INSERT INTO ritrovo_form_state (id, user_id, form_id, step, state, created, updated) \
         VALUES ($1::uuid, $2::uuid, $3, $4, $5::jsonb, \
                 EXTRACT(EPOCH FROM NOW())::bigint, EXTRACT(EPOCH FROM NOW())::bigint)",
        &[
            serde_json::json!(id),
            serde_json::json!(user_id),
            serde_json::json!(FORM_ID),
            serde_json::json!(step),
            serde_json::json!(state),
        ],
    );
    matches!(inserted, Ok(1)).then_some(id)
}

/// A random UUID, generated here rather than by the database.
///
/// **Why the plugin generates the key.** `INSERT ... RETURNING` is the obvious
/// way to learn the id of a row just written, and a plugin cannot use it:
/// `query_raw` refuses any statement that is not read-only, and `execute_raw`
/// returns a row count and nothing else. The WIT declares a structured
/// `db.insert` that does return the inserted row, and the SDK does not wrap it,
/// so there is no way to call it from a plugin either. See FRICTION.md,
/// `G-PLUGIN-CANNOT-READ-BACK-AN-INSERT`.
///
/// Generating the key on the writing side is a normal answer to that and not a
/// workaround: the id is random, the database still enforces the primary key,
/// and the value never depends on what the row looks like.
///
/// Version 4, from the kernel's CSPRNG. The draft id is a capability — it names
/// a row somebody can continue — so it has to be unguessable as well as unique,
/// which rules out anything derived from a counter or a clock.
fn new_uuid() -> Option<String> {
    let hex = host::crypto_random_bytes(16).ok()?;
    if hex.len() != 32 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    // Set the version and variant nibbles, so what is stored is a valid v4 and
    // not merely 32 random hex characters shaped like one.
    let mut bytes: Vec<char> = hex.chars().collect();
    bytes[12] = '4';
    bytes[16] = ['8', '9', 'a', 'b'][(bytes[16].to_digit(16)? % 4) as usize];
    let hex: String = bytes.into_iter().collect();
    Some(format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    ))
}

/// Remove a draft once it has become a conference.
fn discard(user_id: &str, draft: &str) {
    let _ = host::execute_raw(
        "DELETE FROM ritrovo_form_state WHERE id = $1::uuid AND user_id = $2::uuid",
        &[serde_json::json!(draft), serde_json::json!(user_id)],
    );
}

/// Create the conference on the Incoming stage, authored by the submitter.
///
/// A raw insert rather than a host call, because there is no host call that
/// creates an item on a named stage, and the kernel's own create ignores the
/// stage marked default. This is the same insert `ritrovo_importer` makes and
/// carries the same cost: it fires no taps.
/// `G-NO-ITEM-STAGE-TRANSITION`, `G-ITEM-API-BYPASSES-ITEM-SERVICE`.
///
/// `status = 0`: unpublished as well as unreviewed. The stage is what keeps it
/// off the public site, and the status is a second answer to the same question
/// from the kernel's own model, so a site that has not configured stages still
/// does not show a stranger's submission.
fn create_incoming(user_id: &str, answers: &Answers) -> Option<String> {
    let id = new_uuid()?;
    let inserted = host::execute_raw(
        "INSERT INTO item (id, type, title, status, author_id, stage_id, created, changed, fields) \
         VALUES ($1::uuid, 'conference', $2, 0, $3::uuid, $4::uuid, \
                 EXTRACT(EPOCH FROM NOW())::bigint, EXTRACT(EPOCH FROM NOW())::bigint, \
                 $5::jsonb)",
        &[
            serde_json::json!(id),
            serde_json::json!(answers.name),
            serde_json::json!(user_id),
            serde_json::json!(INCOMING_STAGE),
            serde_json::json!(answers.to_fields().to_string()),
        ],
    );
    matches!(inserted, Ok(1)).then_some(id)
}

// ─── The pages ───────────────────────────────────────────────────────

fn page(
    step: i16,
    draft: &str,
    answers: &Answers,
    errors: &[String],
    request: &ApiRequest,
) -> ApiResponse {
    page_with_status(200, step, draft, answers, errors, request)
}

fn page_with_status(
    status: u16,
    step: i16,
    draft: &str,
    answers: &Answers,
    errors: &[String],
    request: &ApiRequest,
) -> ApiResponse {
    let token = &request.csrf_token;
    let body = format!(
        "{}{}{}",
        progress(step),
        errors_html(errors),
        match step {
            2 => step2_html(draft, answers, token),
            3 => review_html(draft, answers, token),
            _ => step1_html(draft, answers, token),
        }
    );
    ApiResponse::themed_with_status(status, "Submit a Conference", body)
}

/// Which of the three steps this is. Plain text, because it has to be readable
/// without CSS as well as without JavaScript.
fn progress(step: i16) -> String {
    let name = match step {
        2 => "Call for papers and description",
        3 => "Review",
        _ => "The conference",
    };
    format!(
        r#"<p class="ritrovo-progress">Step {step} of 3: {}</p>"#,
        web::escape(name)
    )
}

fn errors_html(errors: &[String]) -> String {
    if errors.is_empty() {
        return String::new();
    }
    let mut html = String::from(r#"<div class="ritrovo-errors"><ul>"#);
    for error in errors {
        html.push_str(&format!("<li>{}</li>", web::escape(error)));
    }
    html.push_str("</ul></div>");
    html
}

/// The hidden fields every step posts: the token, the draft, the step number.
fn hidden(draft: &str, step: i16, token: &str) -> String {
    format!(
        r#"<input type="hidden" name="_token" value="{token}">
<input type="hidden" name="draft" value="{draft}">
<input type="hidden" name="step" value="{step}">"#,
        token = web::escape(token),
        draft = web::escape(draft),
    )
}

fn step1_html(draft: &str, answers: &Answers, token: &str) -> String {
    format!(
        r#"<form method="post" action="{path}" class="ritrovo-submit-form">
{hidden}
<p><label for="sub-name">Conference name</label>
<input type="text" id="sub-name" name="name" value="{name}" maxlength="{max_line}" required></p>
<p><label for="sub-url">Website</label>
<input type="url" id="sub-url" name="url" value="{url}" placeholder="https://example.org"></p>
<p><label for="sub-start">Start date</label>
<input type="date" id="sub-start" name="start_date" value="{start}" required></p>
<p><label for="sub-end">End date</label>
<input type="date" id="sub-end" name="end_date" value="{end}" required></p>
<p><label for="sub-city">City</label>
<input type="text" id="sub-city" name="city" value="{city}" maxlength="{max_line}"></p>
<p><label for="sub-country">Country</label>
<input type="text" id="sub-country" name="country" value="{country}" maxlength="{max_line}"></p>
<p><label><input type="checkbox" name="online" value="1"{online}> This conference is online</label></p>
<p><button type="submit">Continue</button></p>
</form>"#,
        path = web::escape(PATH),
        hidden = hidden(draft, 1, token),
        name = web::escape(&answers.name),
        url = web::escape(&answers.url),
        start = web::escape(&answers.start_date),
        end = web::escape(&answers.end_date),
        city = web::escape(&answers.city),
        country = web::escape(&answers.country),
        online = if answers.online { " checked" } else { "" },
        max_line = MAX_LINE,
    )
}

fn step2_html(draft: &str, answers: &Answers, token: &str) -> String {
    format!(
        r#"<form method="post" action="{path}" class="ritrovo-submit-form">
{hidden}
<p><label for="sub-cfp-url">Call for papers address</label>
<input type="url" id="sub-cfp-url" name="cfp_url" value="{cfp_url}" placeholder="https://example.org/cfp"></p>
<p><label for="sub-cfp-end">Call for papers closes</label>
<input type="date" id="sub-cfp-end" name="cfp_end_date" value="{cfp_end}">
<span class="form-help">Leave both blank if this conference has no call for papers.</span></p>
<p><label for="sub-description">Description</label>
<textarea id="sub-description" name="description" rows="10" maxlength="{max_description}">{description}</textarea>
<span class="form-help">Plain text. A blank line starts a new paragraph.</span></p>
<p><button type="submit">Review</button></p>
</form>
<p class="form-help"><a href="{path}?draft={draft_q}&amp;step=1">Back to the conference details</a></p>"#,
        path = web::escape(PATH),
        hidden = hidden(draft, 2, token),
        cfp_url = web::escape(&answers.cfp_url),
        cfp_end = web::escape(&answers.cfp_end_date),
        description = web::escape(&answers.description),
        draft_q = web::escape(draft),
        max_description = MAX_DESCRIPTION,
    )
}

fn review_html(draft: &str, answers: &Answers, token: &str) -> String {
    let row = |label: &str, value: &str| {
        format!(
            "<tr><th scope=\"row\">{}</th><td>{}</td></tr>",
            web::escape(label),
            if value.is_empty() {
                "<em>not given</em>".to_string()
            } else {
                web::escape(value)
            }
        )
    };

    format!(
        r#"<table class="ritrovo-review">
{name}{url}{start}{end}{city}{country}{online}{cfp_url}{cfp_end}
</table>
<h2>Description</h2>
{description}
<form method="post" action="{path}" class="ritrovo-submit-form">
{hidden}
<p><button type="submit">Submit this conference</button></p>
</form>
<p class="form-help">
<a href="{path}?draft={draft_q}&amp;step=1">Change the conference details</a> &middot;
<a href="{path}?draft={draft_q}&amp;step=2">Change the call for papers or description</a>
</p>
<p class="form-help">Two things the brief asks for are not on this form, because
this release of Trovato cannot carry them: a <strong>logo</strong>, because a
plugin has no way to accept a file, and <strong>topics</strong>, because a
category reference cannot be declared as a field. An editor adds both after your
conference arrives.</p>"#,
        name = row("Name", &answers.name),
        url = row("Website", &answers.url),
        start = row("Starts", &answers.start_date),
        end = row("Ends", &answers.end_date),
        city = row("City", &answers.city),
        country = row("Country", &answers.country),
        online = row("Online", if answers.online { "Yes" } else { "No" }),
        cfp_url = row("Call for papers", &answers.cfp_url),
        cfp_end = row("Call for papers closes", &answers.cfp_end_date),
        description = if answers.description.trim().is_empty() {
            "<p><em>not given</em></p>".to_string()
        } else {
            web::paragraphs(&answers.description)
        },
        path = web::escape(PATH),
        hidden = hidden(draft, 3, token),
        draft_q = web::escape(draft),
    )
}

/// The confirmation, which says what happens next rather than only "thank you".
///
/// A submission that disappears into a queue with no word about moderation
/// reads as a submission that was lost.
fn thanks_html(answers: &Answers) -> String {
    format!(
        r#"<p class="ritrovo-submitted">Thank you. <strong>{name}</strong> has been sent to the
editors.</p>
<p>It is not on the site yet. Every submission arrives on the Incoming stage and
is read by an editor before it is published, so it will not appear in the
conference listing, or in search, until one of them has reviewed it. Nobody will
chase you for more detail: if something is missing, an editor fills it in.</p>
<p><a href="{path}">Submit another conference</a> &middot;
<a href="/conferences">Back to the conferences</a></p>"#,
        name = web::escape(&answers.name),
        path = web::escape(PATH),
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn valid() -> Answers {
        Answers {
            name: "Rust Fest Torino".to_string(),
            url: "https://example.org".to_string(),
            start_date: "2027-05-01".to_string(),
            end_date: "2027-05-03".to_string(),
            city: "Torino".to_string(),
            country: "Italy".to_string(),
            online: false,
            cfp_url: "https://example.org/cfp".to_string(),
            cfp_end_date: "2027-03-01".to_string(),
            description: "Two days of Rust.".to_string(),
        }
    }

    // ─── Step 1 ───────────────────────────────────────────────────

    #[test]
    fn a_complete_step_one_is_valid() {
        assert!(step1_errors(&valid()).is_empty());
    }

    #[test]
    fn a_conference_needs_a_name() {
        let mut answers = valid();
        answers.name = String::new();
        assert_eq!(step1_errors(&answers).len(), 1);
    }

    #[test]
    fn a_conference_needs_both_dates() {
        let mut answers = valid();
        answers.start_date = String::new();
        answers.end_date = String::new();
        assert_eq!(step1_errors(&answers).len(), 2);
    }

    #[test]
    fn a_conference_cannot_end_before_it_starts() {
        let mut answers = valid();
        answers.start_date = "2027-05-05".to_string();
        let errors = step1_errors(&answers);
        assert!(
            errors.iter().any(|e| e.contains("starts after it ends")),
            "{errors:?}"
        );
    }

    #[test]
    fn a_one_day_conference_is_fine() {
        let mut answers = valid();
        answers.start_date = "2027-05-01".to_string();
        answers.end_date = "2027-05-01".to_string();
        assert!(step1_errors(&answers).is_empty());
    }

    #[test]
    fn an_in_person_conference_needs_a_country() {
        let mut answers = valid();
        answers.country = String::new();
        assert_eq!(step1_errors(&answers).len(), 1);
    }

    #[test]
    fn an_online_conference_needs_no_country() {
        let mut answers = valid();
        answers.country = String::new();
        answers.city = String::new();
        answers.online = true;
        assert!(step1_errors(&answers).is_empty());
    }

    #[test]
    fn a_website_that_is_not_an_address_is_refused() {
        let mut answers = valid();
        for bad in [
            "example.org",
            "javascript:alert(1)",
            "https://",
            "http:// x",
        ] {
            answers.url = bad.to_string();
            assert_eq!(step1_errors(&answers).len(), 1, "{bad:?} was accepted");
        }
    }

    #[test]
    fn a_website_is_optional() {
        let mut answers = valid();
        answers.url = String::new();
        assert!(step1_errors(&answers).is_empty());
    }

    /// Every problem at once, not the first one found.
    #[test]
    fn a_wholly_empty_step_one_reports_everything() {
        let errors = step1_errors(&Answers::default());
        assert_eq!(errors.len(), 4, "{errors:?}"); // name, start, end, country
    }

    // ─── Step 2 ───────────────────────────────────────────────────

    #[test]
    fn a_complete_step_two_is_valid() {
        assert!(step2_errors(&valid()).is_empty());
    }

    #[test]
    fn a_conference_with_no_cfp_is_valid() {
        let mut answers = valid();
        answers.cfp_url = String::new();
        answers.cfp_end_date = String::new();
        assert!(step2_errors(&answers).is_empty());
    }

    /// The brief's rule, and the reason this form exists rather than the
    /// kernel's: here, something is allowed to say no.
    #[test]
    fn a_cfp_closing_after_the_conference_ends_is_refused() {
        let mut answers = valid();
        answers.cfp_end_date = "2027-05-04".to_string();
        let errors = step2_errors(&answers);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("cannot close after"), "{errors:?}");
    }

    #[test]
    fn a_cfp_closing_on_the_final_day_is_allowed() {
        let mut answers = valid();
        answers.cfp_end_date = "2027-05-03".to_string();
        assert!(step2_errors(&answers).is_empty());
    }

    #[test]
    fn an_over_long_description_is_refused() {
        let mut answers = valid();
        answers.description = "a".repeat(MAX_DESCRIPTION + 1);
        assert_eq!(step2_errors(&answers).len(), 1);
    }

    // ─── Round trips ──────────────────────────────────────────────

    #[test]
    fn answers_survive_the_state_column() {
        let answers = valid();
        assert_eq!(Answers::from_json(&answers.to_json()), answers);
    }

    #[test]
    fn a_state_row_missing_keys_opens_as_blanks() {
        let sparse = serde_json::json!({"name": "Half A Conference"});
        let answers = Answers::from_json(&sparse);
        assert_eq!(answers.name, "Half A Conference");
        assert!(answers.end_date.is_empty());
        assert!(!answers.online);
    }

    #[test]
    fn a_posted_body_becomes_answers() {
        let mut answers = Answers::default();
        answers.read_step1(
            "name=Rust+Fest&start_date=2027-05-01&end_date=2027-05-03&country=Italy&online=1",
        );
        assert_eq!(answers.name, "Rust Fest");
        assert_eq!(answers.start_date, "2027-05-01");
        assert!(answers.online);
    }

    #[test]
    fn an_unchecked_box_reads_as_not_online() {
        let mut answers = Answers {
            online: true,
            ..Answers::default()
        };
        answers.read_step1("name=X&start_date=2027-05-01&end_date=2027-05-03");
        assert!(!answers.online, "an unchecked box left the old answer");
    }

    #[test]
    fn step_two_does_not_disturb_step_one() {
        let mut answers = valid();
        answers.read_step2("cfp_url=&cfp_end_date=&description=Changed.");
        assert_eq!(answers.name, "Rust Fest Torino");
        assert_eq!(answers.start_date, "2027-05-01");
        assert_eq!(answers.description, "Changed.");
        assert!(answers.cfp_url.is_empty());
    }

    // ─── The item it becomes ──────────────────────────────────────

    #[test]
    fn the_fields_are_flat_strings_the_edit_form_can_read() {
        let fields = valid().to_fields();
        assert_eq!(fields["field_city"], serde_json::json!("Torino"));
        assert_eq!(fields["field_start_date"], serde_json::json!("2027-05-01"));
        assert_eq!(fields["field_online"], serde_json::json!("0"));
        assert!(
            fields.get("field_topics").is_none(),
            "topics cannot be collected"
        );
        assert!(
            fields.get("field_logo").is_none(),
            "a logo cannot be uploaded"
        );
    }

    #[test]
    fn an_empty_answer_writes_no_field() {
        let mut answers = valid();
        answers.cfp_url = String::new();
        let fields = answers.to_fields();
        assert!(fields.get("field_cfp_url").is_none());
        // Except the boolean, which has no "unanswered": a checkbox is on or off.
        assert_eq!(fields["field_online"], serde_json::json!("0"));
    }

    #[test]
    fn online_is_stored_as_the_boolean_the_gathers_filter_on() {
        let mut answers = valid();
        answers.online = true;
        assert_eq!(answers.to_fields()["field_online"], serde_json::json!("1"));
    }

    // ─── The pages ────────────────────────────────────────────────

    #[test]
    fn every_step_carries_the_token_the_draft_and_its_number() {
        for (step, html) in [
            (1, step1_html("d-1", &valid(), "tok")),
            (2, step2_html("d-1", &valid(), "tok")),
            (3, review_html("d-1", &valid(), "tok")),
        ] {
            assert!(html.contains(r#"name="_token" value="tok""#), "step {step}");
            assert!(html.contains(r#"name="draft" value="d-1""#), "step {step}");
            assert!(
                html.contains(&format!(r#"name="step" value="{step}""#)),
                "step {step}"
            );
        }
    }

    #[test]
    fn no_step_needs_javascript() {
        for html in [
            step1_html("d", &valid(), "t"),
            step2_html("d", &valid(), "t"),
            review_html("d", &valid(), "t"),
            thanks_html(&valid()),
        ] {
            assert!(!html.contains("<script"), "{html}");
            assert!(!html.contains("onclick"), "{html}");
        }
    }

    #[test]
    fn the_review_shows_every_answer() {
        let html = review_html("d", &valid(), "t");
        for value in [
            "Rust Fest Torino",
            "2027-05-01",
            "Torino",
            "Italy",
            "2027-03-01",
        ] {
            assert!(html.contains(value), "{value} is not on the review: {html}");
        }
        assert!(html.contains("Two days of Rust."), "{html}");
    }

    #[test]
    fn the_review_says_what_is_missing_and_why() {
        let html = review_html("d", &valid(), "t");
        assert!(html.contains("logo"), "{html}");
        assert!(html.contains("topics"), "{html}");
    }

    #[test]
    fn the_review_offers_a_way_back_to_each_step() {
        let html = review_html("draft-7", &valid(), "t");
        assert!(html.contains("draft=draft-7&amp;step=1"), "{html}");
        assert!(html.contains("draft=draft-7&amp;step=2"), "{html}");
    }

    #[test]
    fn an_unanswered_value_reads_as_not_given_rather_than_blank() {
        let mut answers = valid();
        answers.city = String::new();
        assert!(review_html("d", &answers, "t").contains("not given"));
    }

    #[test]
    fn the_confirmation_states_the_moderation_expectation() {
        let html = thanks_html(&valid());
        assert!(html.contains("not on the site yet"), "{html}");
        assert!(html.contains("editor"), "{html}");
    }

    #[test]
    fn a_name_with_markup_in_it_cannot_escape_any_page() {
        let mut answers = valid();
        answers.name = "<script>alert(1)</script>".to_string();
        for html in [
            step1_html("d", &answers, "t"),
            review_html("d", &answers, "t"),
            thanks_html(&answers),
        ] {
            assert!(!html.contains("<script>alert"), "{html}");
            assert!(html.contains("&lt;script&gt;"), "{html}");
        }
    }
}
