//! Subscriptions and notifications for Ritrovo conferences.
//!
//! A signed-in member follows a conference and sees what they follow on a page
//! of their own. That much is built, on Trovato 0.103.0, and it is three of the
//! seven taps `README.md` describes. The other four wait on the kernel, and
//! what each waits on is named there rather than guessed at here.
//!
//! # What changed at 0.103.0, and why this plugin stopped being a placeholder
//!
//! It was stripped to nothing in an earlier pass because every tap it declared
//! had nothing behind it: a `/user/subscriptions` menu entry with no `tap_api`,
//! so the path 404ed; a Subscribe button on every conference page with no form
//! or endpoint, so it did nothing; a `ritrovo_notifications` queue with no
//! worker; and two permissions nothing checked. A route that 404s and a button
//! that does nothing are worse than their absence.
//!
//! The one gap that kept the rest of it unbuildable was `tap_perm`: declared in
//! the WIT, dispatched nowhere, so a plugin's permissions existed in no list
//! the kernel could consult and no role could hold one. 0.103.0 dispatches it
//! at boot into a registry and a `plugin_permission` table that config import
//! reads, so [`tap_perm`] below is now a declaration with a consequence, and
//! `demo/config` grants it. `FRICTION.md`, `G-PERM-TAP-NOT-DISPATCHED`.
//!
//! # The rule every tap here follows
//!
//! **Nothing is declared that has nothing behind it.** That is the rule this
//! plugin was emptied for, so it is worth stating what it costs here:
//!
//! - One permission is declared, not the two `README.md` lists.
//!   `manage own subscriptions` gates all three routes and the kernel checks it
//!   before dispatch. `administer notifications` would gate a settings screen
//!   and a queue view, and neither exists, so declaring it would put a
//!   permission on the grid that an administrator could grant to no effect.
//!   It lands with the screen it is for.
//! - Every menu entry is a `MenuRoute::api` whose callback [`tap_api`] serves,
//!   asserted by a unit test below rather than by reading.
//! - [`tap_queue_info`] is the single deliberate exception, and it is argued in
//!   full at its own doc comment.

use trovato_sdk::prelude::*;
use trovato_sdk::types::{ApiRequest, ApiResponse, MenuRoute};

mod subscriptions;
mod web;

/// Plugin name, for logging calls.
pub const PLUGIN_NAME: &str = "ritrovo_notify";

/// The permission every route here is gated on.
///
/// This plugin's own, declared by [`tap_perm`] and granted to the
/// `authenticated user` role in `demo/config`, so every signed-in member holds
/// it and no anonymous visitor does. The kernel checks it before dispatch, so
/// an anonymous request never reaches this module — and, because the kernel
/// filters the menu it renders to what the viewer may open, an anonymous
/// visitor is not shown the entry either.
///
/// `ritrovo_forms` had to borrow a kernel permission for the same job, because
/// at 0.102 a plugin's own permission could not be granted to anybody. That
/// workaround is not needed here.
const MEMBER: &str = "manage own subscriptions";

/// The queue this plugin owns.
const QUEUE_NAME: &str = "ritrovo_notifications";

/// Declare this plugin's permissions.
///
/// One, for now. See the module comment for why `administer notifications` is
/// not here: it would gate a settings screen that does not exist.
#[plugin_tap]
pub fn tap_perm() -> Vec<PermissionDefinition> {
    vec![PermissionDefinition::new(
        MEMBER,
        "Subscribe to conferences, and see your own subscriptions",
    )]
}

/// Register the routes.
///
/// Three entries: the member's list, and one per write. Only the `GET` is
/// `visible`, because a navigation entry for a `POST` is a link that cannot be
/// followed. `parent("/user")` puts it in the user menu, beside the profile
/// `ritrovo_forms` serves, which is where a person looks for their own things
/// and — since the kernel filters that menu per viewer — the one affordance
/// that is correctly hidden from an anonymous visitor.
///
/// `{uid}` is a path parameter; the kernel extracts it and hands it over in
/// `ApiRequest::params`. It is in the path because the privacy rule needs
/// something to refuse: a list at `/user/subscriptions` could only ever be the
/// caller's own, and story 37.3 asks that another member's be refused.
#[plugin_tap]
pub fn tap_menu() -> Vec<MenuRoute> {
    vec![
        MenuRoute::api("GET", subscriptions::PATH_LIST, "subscriptions_show")
            .title("Your subscriptions")
            .permission(MEMBER)
            .parent("/user")
            .weight(15)
            .visible(),
        MenuRoute::api("POST", subscriptions::PATH_SUBSCRIBE, "subscribe").permission(MEMBER),
        MenuRoute::api("POST", subscriptions::PATH_UNSUBSCRIBE, "unsubscribe").permission(MEMBER),
    ]
}

/// Serve one request.
///
/// Routed on `callback` rather than on the path, which is what the kernel hands
/// over and what lets one path carry two methods.
#[plugin_tap]
pub fn tap_api(request: ApiRequest) -> ApiResponse {
    match request.callback.as_str() {
        "subscriptions_show" => subscriptions::show(&request),
        "subscribe" => subscriptions::subscribe(&request),
        "unsubscribe" => subscriptions::unsubscribe(&request),
        other => ApiResponse::error(404, &format!("no such callback: {other}")),
    }
}

/// Declare the `ritrovo_notifications` queue.
///
/// **This is the one declaration here with nothing behind it, and it is
/// deliberate.** The rule everywhere else in this plugin is that a declaration
/// has an implementation; this is the exception, so here is the argument.
///
/// The queue's producer is blocked, not unwritten. Two things would fill it and
/// neither can. A plugin's item write fires no taps, so the importer changing a
/// conference notifies nothing (`G-ITEM-API-BYPASSES-ITEM-SERVICE`). And a
/// queue belongs to the plugin that pushes into it — the host stamps the
/// caller's own name onto every job and the drain hands it back to that same
/// plugin — so `ritrovo_cfp` cannot enqueue a `cfp_closing_soon` event onto
/// this one (`G-QUEUE-NO-CROSS-PLUGIN`). A `tap_queue_worker` added now would
/// be a worker for a queue nothing can fill, which is the other half of the
/// thing this plugin was stripped for.
///
/// What the declaration is for is that the eventual fix attaches to it. It
/// costs nothing while it is empty: the kernel reads `tap_queue_info` only to
/// size a plugin's drain width, and a plugin exporting no `tap_queue_worker` is
/// skipped by the drain entirely.
///
/// The shape is what the kernel reads, which is not what this plugin used to
/// declare. It must be a JSON **array**: `parse_max_concurrency` returns 1 for
/// an object, so the old object form was ignored in full. `concurrency` is the
/// only key read. `max_retries` and `retry_delay_seconds` are gone because the
/// kernel reads neither, and a declaration the kernel ignores is a promise to
/// the next reader that nothing keeps.
#[plugin_tap]
pub fn tap_queue_info() -> serde_json::Value {
    serde_json::json!([
        {
            "name": QUEUE_NAME,
            "concurrency": 1
        }
    ])
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    const NIL_UUID: &str = "00000000-0000-0000-0000-000000000000";

    fn routes() -> Vec<MenuRoute> {
        __inner_tap_menu()
    }

    #[test]
    fn every_route_is_reachable() {
        // A menu entry whose `handler_type` is not "api", or which names no
        // callback, is a path that 404s and a handler that is dead code. This
        // plugin was emptied for shipping exactly that.
        for route in routes() {
            assert_eq!(route.handler_type, "api", "{}", route.path);
            assert!(!route.callback.is_empty(), "{}", route.path);
        }
    }

    #[test]
    fn every_registered_callback_is_served() {
        for route in routes() {
            let request =
                ApiRequest::new(&route.callback, &route.method, &route.path, NIL_UUID, true);
            assert_ne!(
                __inner_tap_api(request).status,
                404,
                "{} dispatches to nothing",
                route.callback
            );
        }
    }

    #[test]
    fn an_unknown_callback_is_a_404() {
        let request = ApiRequest::new("no_such_thing", "GET", "/x", NIL_UUID, true);
        assert_eq!(__inner_tap_api(request).status, 404);
    }

    #[test]
    fn every_route_is_gated_on_the_members_permission() {
        // An ungated route here would be a page a stranger could post to. The
        // kernel treats an empty permission as public, and `MenuRoute::api`
        // defaults to empty, so forgetting one is silent without this.
        for route in routes() {
            assert_eq!(route.permission, MEMBER, "{} is ungated", route.path);
        }
    }

    #[test]
    fn the_permission_every_route_is_gated_on_is_one_this_plugin_declares() {
        // The gate and the declaration drifting apart would leave every route
        // permanently 403: the kernel checks a string no role can be granted.
        let declared: Vec<String> = __inner_tap_perm().into_iter().map(|p| p.name).collect();
        assert!(declared.contains(&MEMBER.to_string()), "{declared:?}");
    }

    #[test]
    fn no_permission_is_declared_that_nothing_checks() {
        // The rule this plugin was stripped for. `administer notifications` is
        // absent on purpose; it lands with the screen it gates.
        let declared = __inner_tap_perm();
        assert_eq!(declared.len(), 1, "{:?}", declared.len());
        assert_eq!(declared[0].name, MEMBER);
    }

    #[test]
    fn only_a_get_ever_shows_in_navigation() {
        for route in routes().into_iter().filter(|r| r.visible) {
            assert_eq!(route.method, "GET", "{} is a visible POST", route.path);
        }
    }

    #[test]
    fn the_list_sits_under_the_user_menu() {
        let entry = routes()
            .into_iter()
            .find(|r| r.path == subscriptions::PATH_LIST && r.visible)
            .expect("the list has a visible entry");
        assert_eq!(entry.parent.as_deref(), Some("/user"));
    }

    #[test]
    fn every_write_is_a_post() {
        for path in [
            subscriptions::PATH_SUBSCRIBE,
            subscriptions::PATH_UNSUBSCRIBE,
        ] {
            let route = routes()
                .into_iter()
                .find(|r| r.path == path)
                .expect("the write is registered");
            assert_eq!(route.method, "POST", "{path} writes on a {}", route.method);
        }
    }

    #[test]
    fn every_route_is_scoped_to_one_member_by_its_path() {
        // The privacy rule needs something to refuse. A path with no `{uid}`
        // could only ever be the caller's own.
        for route in routes() {
            assert!(route.path.contains("{uid}"), "{} is not scoped", route.path);
        }
    }

    #[test]
    fn the_queue_is_declared_as_the_array_the_kernel_reads() {
        // `parse_max_concurrency` reads a JSON array and returns 1 for anything
        // else, so the object this plugin used to return was ignored in full.
        let declared = __inner_tap_queue_info();
        let queues = declared.as_array().expect("an array: {declared}");
        assert_eq!(queues.len(), 1);
        assert_eq!(queues[0]["name"], QUEUE_NAME);
        assert_eq!(queues[0]["concurrency"], 1);
    }

    #[test]
    fn the_queue_declares_no_key_the_kernel_ignores() {
        // `max_retries` and `retry_delay_seconds` were declared here and read
        // by nothing. A declaration the kernel ignores is a promise to the next
        // reader that nothing keeps.
        let declared = __inner_tap_queue_info();
        let queue = &declared.as_array().expect("an array")[0];
        let keys: Vec<&String> = queue.as_object().expect("an object").keys().collect();
        assert_eq!(keys.len(), 2, "{keys:?}");
        for ignored in ["max_retries", "retry_delay_seconds"] {
            assert!(queue.get(ignored).is_none(), "{ignored} is declared again");
        }
    }
}
