//! The member profile: a bio, on a page Ritrovo serves.
//!
//! # Why this is not on `/user/profile`
//!
//! The brief wants a profile with a display name, a bio and an avatar. The
//! kernel's profile form has username, email and timezone, and it is not a form
//! a plugin can reach: `profile_update` extracts a fixed
//! `Form<ProfileFormRequest>`, so an extra input posted to it is discarded by
//! serde before any code runs, and no tap is dispatched anywhere along that
//! route. There is no seam. See FRICTION.md, G-USER-PROFILE-NOT-EXTENSIBLE.
//!
//! So Ritrovo serves its own page, at its own path, holding the fields the
//! kernel's page does not. Two pages rather than one is the honest shape of what
//! this kernel allows, and the user menu links both.
//!
//! # What is here and what is not
//!
//! - **Bio.** Here. Plain text in, paragraphs out, escaped
//!   ([`crate::web::paragraphs`]). Not rich text: the kernel's `filtered_html`
//!   format is an ammonia pass the kernel runs, and no host function exposes it
//!   to a plugin, so accepting HTML would mean shipping a sanitizer inside a
//!   WASM module. FRICTION.md, G-NO-TEXT-FORMAT-HOST-API.
//! - **Avatar.** Not here, and not for want of trying. A plugin cannot accept a
//!   file: there is no host interface for one, `tap_api` hands over a body
//!   string rather than a parsed multipart request, and nothing a plugin can
//!   call stores an upload or promotes it out of temporary. FRICTION.md,
//!   G-FILE-NO-HOST-API. The A7 work that renders an avatar beside a comment
//!   waits on the same entry.

use trovato_sdk::host;
use trovato_sdk::types::{ApiRequest, ApiResponse};

use crate::web;

/// Where the page lives. One path, two methods, so the form posts back to the
/// URL it was served from.
pub const PATH: &str = "/user/bio";

/// Longest bio accepted, in characters.
///
/// Bounded because it is stored: an unbounded textarea is a way to fill a
/// table one save at a time. Generous enough for a few paragraphs about
/// yourself, which is what the field is for.
pub const MAX_BIO: usize = 2_000;

/// Serve the form.
pub fn show(request: &ApiRequest) -> ApiResponse {
    let bio = load(&request.user_id).unwrap_or_default();
    ApiResponse::themed("Your profile", page(&request.csrf_token, &bio, &[], false))
}

/// Validate and save.
pub fn save(request: &ApiRequest) -> ApiResponse {
    let bio = web::normalize(&web::field(&request.body, "bio"));

    let errors = errors(&bio);
    if !errors.is_empty() {
        // 422: the request was understood and not acted on. The token rendered
        // back is the fresh one the kernel minted for *this* request, because a
        // token is single-use and the one that arrived has been spent.
        return ApiResponse::themed_with_status(
            422,
            "Your profile",
            page(&request.csrf_token, &bio, &errors, false),
        );
    }

    if !store(&request.user_id, &bio) {
        host::log(
            "error",
            "ritrovo_forms",
            &format!("could not save the bio for user {}", request.user_id),
        );
        return ApiResponse::themed_with_status(
            500,
            "Your profile",
            page(
                &request.csrf_token,
                &bio,
                &["Your profile could not be saved. Please try again.".to_string()],
                false,
            ),
        );
    }

    ApiResponse::themed("Your profile", page(&request.csrf_token, &bio, &[], true))
}

/// Everything wrong with a submitted bio.
///
/// Pure, so the rule is testable without a database. All of them at once rather
/// than the first, for the same reason the kernel's contact form does it: a
/// person who has typed a long answer should not find the problems with it one
/// round-trip at a time.
pub fn errors(bio: &str) -> Vec<String> {
    let mut errors = Vec::new();
    let length = bio.chars().count();
    if length > MAX_BIO {
        errors.push(format!(
            "Your bio is {length} characters long. The limit is {MAX_BIO}."
        ));
    }
    errors
}

/// The stored bio for a user, or `None` when there is no row or no database.
///
/// An unreadable table and an empty bio render the same page on purpose: the
/// form is still usable, and a person editing their own profile cannot act on
/// "the query failed" anyway. The log is where that goes.
fn load(user_id: &str) -> Option<String> {
    let raw = host::query_raw(
        "SELECT bio FROM ritrovo_profile WHERE user_id = $1::uuid",
        &[serde_json::json!(user_id)],
    )
    .inspect_err(|code| {
        host::log(
            "error",
            "ritrovo_forms",
            &format!("could not read a profile: host error {code}"),
        );
    })
    .ok()?;

    serde_json::from_str::<Vec<serde_json::Value>>(&raw)
        .ok()?
        .into_iter()
        .next()?
        .get("bio")?
        .as_str()
        .map(str::to_string)
}

/// Write a user's bio, returning whether the row landed.
///
/// An upsert rather than a check-then-insert: two saves from two tabs would
/// otherwise race, and the loser would be a primary-key violation rather than
/// the later value.
fn store(user_id: &str, bio: &str) -> bool {
    let result = host::execute_raw(
        "INSERT INTO ritrovo_profile (user_id, bio, updated) \
         VALUES ($1::uuid, $2, EXTRACT(EPOCH FROM NOW())::bigint) \
         ON CONFLICT (user_id) DO UPDATE SET \
           bio = EXCLUDED.bio, \
           updated = EXCLUDED.updated",
        &[serde_json::json!(user_id), serde_json::json!(bio)],
    );
    matches!(result, Ok(1))
}

/// The page: the form, what it will look like, and why the avatar is missing.
fn page(token: &str, bio: &str, errors: &[String], saved: bool) -> String {
    let mut html = String::new();

    if saved {
        html.push_str(r#"<p class="ritrovo-saved">Your profile has been saved.</p>"#);
    }

    if !errors.is_empty() {
        html.push_str(r#"<div class="ritrovo-errors"><ul>"#);
        for error in errors {
            html.push_str(&format!("<li>{}</li>", web::escape(error)));
        }
        html.push_str("</ul></div>");
    }

    html.push_str(&format!(
        r#"<form method="post" action="{path}" class="ritrovo-profile-form">
<input type="hidden" name="_token" value="{token}">
<p><label for="ritrovo-bio">About you</label>
<textarea id="ritrovo-bio" name="bio" rows="10" maxlength="{max}">{bio}</textarea></p>
<p class="form-help">Plain text. A blank line starts a new paragraph. At most {max} characters.</p>
<p><button type="submit">Save profile</button></p>
</form>"#,
        path = web::escape(PATH),
        token = web::escape(token),
        bio = web::escape(bio),
        max = MAX_BIO,
    ));

    if !bio.trim().is_empty() {
        html.push_str(r#"<h2>How it will read</h2><div class="ritrovo-bio">"#);
        html.push_str(&web::paragraphs(bio));
        html.push_str("</div>");
    }

    html.push_str(&format!(
        r#"<p class="form-help">Your username, email address and timezone are on
<a href="/user/profile">your account page</a>. An avatar cannot be uploaded on this
release: a plugin has no way to accept a file. At most {MAX_BIO} characters here.</p>"#
    ));

    html
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn a_short_bio_is_valid() {
        assert!(errors("Runs a conference or two.").is_empty());
    }

    #[test]
    fn an_empty_bio_is_valid() {
        // Clearing the field is how a person removes their bio, so it cannot be
        // an error.
        assert!(errors("").is_empty());
    }

    #[test]
    fn a_bio_at_the_limit_is_valid() {
        assert!(errors(&"a".repeat(MAX_BIO)).is_empty());
    }

    #[test]
    fn a_bio_over_the_limit_is_refused_and_says_by_how_much() {
        let errors = errors(&"a".repeat(MAX_BIO + 1));
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains(&(MAX_BIO + 1).to_string()), "{errors:?}");
    }

    // The limit counts characters, not bytes: a bio of accented text was refused
    // at well under the stated length when this counted `len()`.
    #[test]
    fn the_limit_counts_characters_not_bytes() {
        assert!(errors(&"\u{e8}".repeat(MAX_BIO)).is_empty());
    }

    #[test]
    fn the_form_carries_the_token_and_posts_to_its_own_path() {
        let html = page("tok-123", "", &[], false);
        assert!(html.contains(r#"name="_token" value="tok-123""#), "{html}");
        assert!(html.contains(&format!(r#"action="{PATH}""#)), "{html}");
        assert!(html.contains(r#"method="post""#), "{html}");
    }

    #[test]
    fn the_form_needs_no_javascript() {
        let html = page("tok", "hello", &[], false);
        assert!(!html.contains("<script"), "{html}");
        assert!(!html.contains("onclick"), "{html}");
    }

    #[test]
    fn a_stored_bio_comes_back_in_the_textarea_escaped() {
        let html = page("tok", "<b>hi</b>", &[], false);
        assert!(html.contains("&lt;b&gt;hi&lt;/b&gt;"), "{html}");
        assert!(!html.contains("<b>"), "{html}");
    }

    #[test]
    fn errors_render_above_the_form() {
        let html = page("tok", "x", &["Too long.".to_string()], false);
        let error_at = html.find("Too long.").unwrap();
        let form_at = html.find("<form").unwrap();
        assert!(error_at < form_at, "{html}");
    }

    #[test]
    fn saving_says_so() {
        assert!(page("tok", "x", &[], true).contains("has been saved"));
        assert!(!page("tok", "x", &[], false).contains("has been saved"));
    }

    #[test]
    fn the_page_points_at_the_kernels_own_profile() {
        assert!(page("tok", "", &[], false).contains(r#"href="/user/profile""#));
    }
}
