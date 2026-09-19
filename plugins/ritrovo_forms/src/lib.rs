//! The forms Ritrovo serves itself.
//!
//! # Why a plugin serves forms at all
//!
//! Trovato has a Form API: `FormService` builds a form from a definition,
//! validates it, submits it, and dispatches `tap_form_alter`,
//! `tap_form_validate`, `tap_form_submit` and `tap_form_ajax` along the way. At
//! the pinned release **no route calls it**. The kernel's own words, in
//! `crates/kernel/src/routes/plugin_api.rs`: "The form taps were unreachable:
//! `FormService` is constructed and exposed on `AppState`, and no route calls
//! `build` or `process`." So a plugin cannot add a field to a kernel form, alter
//! one, or serve one through the kernel's pipeline. See FRICTION.md,
//! G-FORM-TAPS-UNREACHABLE.
//!
//! What a plugin *can* do, since `KERNEL_API_VERSION (0,101)`, is own a route
//! end to end through `tap_api`, and serve a form on it that works with
//! JavaScript switched off. Two kernel changes made that true and both matter
//! here: a plugin-served POST accepts a `_token` field in the body as well as an
//! `X-CSRF-Token` header, because a plain `<form>` cannot set a header; and the
//! kernel mints a token per request and hands it over in
//! [`ApiRequest::csrf_token`] for the plugin to render into a hidden input.
//! `plugins/trovato_contact` in the kernel is the worked example, and this
//! plugin is built the same way.
//!
//! # What is served here
//!
//! - [`profile`] — `/user/bio`, the member profile the kernel's `/user/profile`
//!   has no room for.
//! - [`submit`] — `/conferences/submit`, a three-step "Submit a Conference" for
//!   signed-in members, with its state on the server between steps.
//!
//! # The rule every page here follows
//!
//! **It works without JavaScript.** Not as a fallback: there is no JavaScript in
//! any of it. Each page is a `<form method="post">` that posts to its own URL
//! and gets HTML back, which is the only shape available anyway — the kernel's
//! AJAX form helpers are admin-only and route through the `FormService` nothing
//! calls (FRICTION.md, G-AJAX-ADMIN-ONLY-NO-CONDITIONAL-FIELDS).
//!
//! **It escapes everything.** The kernel does not sanitize a plugin's response
//! body; that contract is stated in `plugin_api.rs` and it means an unescaped
//! interpolation here is a stored-XSS hole on a page any member can write to.
//! Everything goes through [`web::escape`] or [`web::paragraphs`].

use trovato_sdk::prelude::*;
use trovato_sdk::types::{ApiRequest, ApiResponse, MenuRoute};

mod dates;
mod profile;
mod submit;
mod web;

/// The permission every route here is gated on.
///
/// Seeded by a kernel migration onto the `authenticated user` role, which means
/// every signed-in visitor holds it and no anonymous one does — exactly the gate
/// these pages want, and a kernel permission rather than one of this plugin's
/// own, because a plugin's permissions are declared into a void on this release
/// (FRICTION.md, G-PERM-TAP-NOT-DISPATCHED).
///
/// The check happens in the kernel, before dispatch, so an anonymous request
/// never reaches this module.
const MEMBER: &str = "view own profile";

/// The permission the submission form is gated on.
///
/// `create content` is a kernel permission granted to the `authenticated user`
/// role and to no anonymous one, which is exactly what story 36.4 asks for
/// ("`create content` permission required; anonymous users redirected to
/// login"). The redirect is the half that is not available: a plugin route
/// cannot set a response header, so an anonymous visitor gets 401 rather than
/// the login form (FRICTION.md, `G-PLUGIN-ROUTE-NO-HEADERS`).
const SUBMITTER: &str = "create content";

/// Register the routes.
///
/// Two entries per page, one per method, because the form posts back to the URL
/// it was served from. Only the `GET` is `visible`: a navigation entry for a
/// `POST` would be a link that cannot be followed.
///
/// `parent("/user")` puts the entry under the user menu, which is where the
/// brief asks for it and where a person looks for their own profile.
#[plugin_tap]
pub fn tap_menu() -> Vec<MenuRoute> {
    vec![
        MenuRoute::api("GET", profile::PATH, "profile_show")
            .title("Your profile")
            .permission(MEMBER)
            .parent("/user")
            .weight(10)
            .visible(),
        MenuRoute::api("POST", profile::PATH, "profile_save")
            .title("Your profile")
            .permission(MEMBER),
        MenuRoute::api("GET", submit::PATH, "submit_show")
            .title("Submit a Conference")
            .permission(SUBMITTER)
            .weight(20)
            .visible(),
        MenuRoute::api("POST", submit::PATH, "submit_post").permission(SUBMITTER),
    ]
}

/// Serve one request.
///
/// Routed on `callback` rather than on the path, which is what the kernel hands
/// over and what lets one path carry two methods.
#[plugin_tap]
pub fn tap_api(request: ApiRequest) -> ApiResponse {
    match request.callback.as_str() {
        "profile_show" => profile::show(&request),
        "profile_save" => profile::save(&request),
        "submit_show" => submit::show(&request),
        "submit_post" => submit::post(&request),
        other => ApiResponse::error(404, &format!("no such callback: {other}")),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    /// A user id that satisfies the type without naming anybody.
    const NIL_UUID: &str = "00000000-0000-0000-0000-000000000000";

    fn routes() -> Vec<MenuRoute> {
        __inner_tap_menu()
    }

    #[test]
    fn every_route_is_reachable() {
        // A menu entry whose `handler_type` is not "api", or which names no
        // callback, is a path that 404s and a handler that is dead code. The
        // kernel logs it at startup; this fails the build instead.
        for route in routes() {
            assert_eq!(route.handler_type, "api", "{}", route.path);
            assert!(!route.callback.is_empty(), "{}", route.path);
        }
    }

    #[test]
    fn every_route_is_gated_on_a_permission_anonymous_cannot_hold() {
        // Both are kernel permissions held by the `authenticated user` role and
        // by no anonymous one. The kernel checks them before dispatch, so an
        // ungated route here would be a page a stranger could post to.
        for route in routes() {
            assert!(
                route.permission == MEMBER || route.permission == SUBMITTER,
                "{} is gated on {:?}",
                route.path,
                route.permission
            );
        }
    }

    #[test]
    fn every_registered_callback_is_served() {
        for route in routes() {
            let request =
                ApiRequest::new(&route.callback, &route.method, &route.path, NIL_UUID, true);
            let response = tap_api_dispatch(request);
            assert_ne!(
                response.status, 404,
                "{} dispatches to nothing",
                route.callback
            );
        }
    }

    #[test]
    fn an_unknown_callback_is_a_404() {
        let request = ApiRequest::new("no_such_thing", "GET", "/x", NIL_UUID, true);
        assert_eq!(tap_api_dispatch(request).status, 404);
    }

    #[test]
    fn only_a_get_ever_shows_in_navigation() {
        // A navigation entry for a POST is a link that cannot be followed.
        for route in routes().into_iter().filter(|r| r.visible) {
            assert_eq!(
                route.method, "GET",
                "{} is a visible {}",
                route.path, route.method
            );
        }
    }

    #[test]
    fn the_profile_sits_under_the_user_menu() {
        let profile = routes()
            .into_iter()
            .find(|r| r.path == profile::PATH && r.visible)
            .expect("the profile has a visible entry");
        assert_eq!(profile.parent.as_deref(), Some("/user"));
    }

    #[test]
    fn one_path_carries_both_methods() {
        for path in [profile::PATH, submit::PATH] {
            let methods: Vec<String> = routes()
                .into_iter()
                .filter(|r| r.path == path)
                .map(|r| r.method)
                .collect();
            assert!(methods.contains(&"GET".to_string()), "{path}: {methods:?}");
            assert!(methods.contains(&"POST".to_string()), "{path}: {methods:?}");
        }
    }

    /// Run a request through the tap's own body.
    ///
    /// Natively the SDK's host stubs return no rows, so a page renders as if the
    /// profile were empty, which is all these tests read.
    fn tap_api_dispatch(request: ApiRequest) -> ApiResponse {
        __inner_tap_api(request)
    }
}
