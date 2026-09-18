#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `ritrovo_forms` on the real kernel: install, enable, migrate, and serve the
//! member profile over HTTP through the kernel's own plugin-API router.
//!
//! Every request here goes through `build_plugin_api_router`, a Redis-backed
//! session and the kernel's CSRF verification, as a browser's would. In
//! particular the POSTs are `application/x-www-form-urlencoded` with the token in
//! a `_token` body field and **no `X-CSRF-Token` header**, because that is what a
//! plain HTML `<form>` can send, and "works without JavaScript" is a claim these
//! tests are supposed to be able to falsify.
//!
//! Needs Postgres (`DATABASE_URL`), Redis (`REDIS_URL`) and the assembled
//! overlay:
//!
//! ```text
//! cargo build --target wasm32-wasip1 --release && scripts/assemble-overlay.sh
//! ```

#[path = "../../../tests/host/mod.rs"]
mod host;

use host::serial;

const PLUGIN: &str = "ritrovo_forms";
const PATH: &str = "/user/bio";

#[test]
fn the_kernel_installs_enables_and_loads_the_module() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::uninstall(&pool, PLUGIN).await;

        host::install_and_enable(&pool, PLUGIN)
            .await
            .unwrap_or_else(|e| panic!("the kernel would not install {PLUGIN}: {e}"));
        assert_eq!(
            host::plugin_status(&pool, PLUGIN).await,
            Some(trovato_kernel::plugin::status::STATUS_ENABLED)
        );

        let disp = host::dispatcher(PLUGIN);
        let compiled = disp.runtime().get_plugin(PLUGIN).unwrap();
        compiled
            .info
            .check_api_compatibility()
            .unwrap_or_else(|e| panic!("{e:#}"));
    });
}

/// The plugin owns a table, so the kernel has to have created it.
///
/// A plugin-owned table is the only place a bio can live
/// (`G-USER-PROFILE-NOT-EXTENSIBLE`), so this is the feature's foundation rather
/// than a detail: if the migration does not run, every page below fails in a way
/// that looks like a routing bug.
#[test]
fn the_migration_runs_and_creates_the_profile_table() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::uninstall(&pool, PLUGIN).await;
        sqlx::query("DROP TABLE IF EXISTS ritrovo_profile")
            .execute(&pool)
            .await
            .unwrap();

        host::install_and_enable(&pool, PLUGIN).await.unwrap();

        let applied = host::applied_migrations(&pool, PLUGIN).await;
        assert_eq!(
            applied,
            vec!["migrations/001_create_ritrovo_profile.sql".to_string()],
            "the kernel records a migration under the path the manifest lists"
        );

        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables \
             WHERE table_name = 'ritrovo_profile')",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(exists, "the migration ran and the table is not there");
    });
}

/// The whole feature, driven the way a person drives it.
///
/// Sign in, open the page, type a bio, save it, and come back to find it. The
/// intermediate assertions are what a browser would have had to be able to do:
/// read a token out of the form, and post without a header.
#[test]
fn a_member_writes_a_bio_and_finds_it_again() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::prepare_app_database(&pool, &[PLUGIN]).await;
        host::import_demo_config(&pool).await;
        let app = host::app().await;
        let (cookies, user_id) = app.login_as_new_member(&[]).await;

        // The form, as served.
        let form = app.get(PATH, &cookies).await;
        assert_eq!(form.status, 200, "{}", form.body);
        assert!(
            form.body.contains(r#"name="bio""#),
            "no bio field: {}",
            form.body
        );
        assert!(
            !form.body.contains("<script"),
            "the form carries JavaScript: {}",
            form.body
        );

        // Saved, with the token the form itself carried and nothing else.
        let bio = "Runs the Torino Rust meetup.\n\nFond of long trains.";
        let saved = app
            .post_form(
                PATH,
                &format!("_token={}&bio={}", token(&form.body), encode(bio)),
                &cookies,
            )
            .await;
        assert_eq!(saved.status, 200, "{}", saved.body);
        assert!(saved.body.contains("has been saved"), "{}", saved.body);

        // In the table, under this user, exactly as typed.
        let stored: String =
            sqlx::query_scalar("SELECT bio FROM ritrovo_profile WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(stored, bio);

        // And on the page the next time it is opened.
        let again = app.get(PATH, &cookies).await;
        assert_eq!(again.status, 200);
        assert!(
            again.body.contains("Runs the Torino Rust meetup."),
            "the saved bio did not come back: {}",
            again.body
        );
        // Rendered as paragraphs, which is what this plugin has instead of rich
        // text; see FRICTION.md, G-NO-TEXT-FORMAT-HOST-API.
        assert!(
            again.body.contains("<p>Fond of long trains.</p>"),
            "{}",
            again.body
        );
    });
}

/// Saving twice updates the one row rather than failing on the primary key.
#[test]
fn saving_again_replaces_the_bio_and_keeps_one_row() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::prepare_app_database(&pool, &[PLUGIN]).await;
        host::import_demo_config(&pool).await;
        let app = host::app().await;
        let (cookies, user_id) = app.login_as_new_member(&[]).await;

        for bio in ["first", "second"] {
            let form = app.get(PATH, &cookies).await;
            let saved = app
                .post_form(
                    PATH,
                    &format!("_token={}&bio={bio}", token(&form.body)),
                    &cookies,
                )
                .await;
            assert_eq!(saved.status, 200, "{}", saved.body);
        }

        let rows: Vec<String> =
            sqlx::query_scalar("SELECT bio FROM ritrovo_profile WHERE user_id = $1")
                .bind(user_id)
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(rows, vec!["second".to_string()]);
    });
}

/// A POST with no token is refused by the kernel, before the plugin sees it.
///
/// This is the half of CSRF a plugin does not implement and must not be able to
/// opt out of: the check is in `plugin_api.rs`, ahead of dispatch. If this ever
/// returns 200, a plugin-served write is forgeable from any page a member
/// visits.
#[test]
fn a_post_without_a_token_is_refused_and_writes_nothing() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::prepare_app_database(&pool, &[PLUGIN]).await;
        host::import_demo_config(&pool).await;
        let app = host::app().await;
        let (cookies, user_id) = app.login_as_new_member(&[]).await;

        let refused = app.post_form(PATH, "bio=forged", &cookies).await;
        assert_eq!(refused.status, 403, "{}", refused.body);

        let rows: i64 =
            sqlx::query_scalar("SELECT count(*) FROM ritrovo_profile WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(rows, 0, "a tokenless POST wrote a row");
    });
}

/// A token is single-use, so replaying one is refused.
#[test]
fn a_replayed_token_is_refused() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::prepare_app_database(&pool, &[PLUGIN]).await;
        host::import_demo_config(&pool).await;
        let app = host::app().await;
        let (cookies, _) = app.login_as_new_member(&[]).await;

        let form = app.get(PATH, &cookies).await;
        let token = token(&form.body);

        let first = app
            .post_form(PATH, &format!("_token={token}&bio=once"), &cookies)
            .await;
        assert_eq!(first.status, 200, "{}", first.body);

        let replayed = app
            .post_form(PATH, &format!("_token={token}&bio=twice"), &cookies)
            .await;
        assert_eq!(replayed.status, 403, "a spent token was accepted");
    });
}

/// An over-long bio comes back with the message and is not stored.
///
/// 422 rather than 200: the request was understood and not acted on. The page it
/// comes back on carries a fresh token, because the submitted one has been spent
/// — a validation failure that could not be corrected and resubmitted would be
/// worse than no validation.
#[test]
fn an_over_long_bio_is_refused_with_a_message_and_a_usable_form() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::prepare_app_database(&pool, &[PLUGIN]).await;
        host::import_demo_config(&pool).await;
        let app = host::app().await;
        let (cookies, user_id) = app.login_as_new_member(&[]).await;

        let form = app.get(PATH, &cookies).await;
        let refused = app
            .post_form(
                PATH,
                &format!("_token={}&bio={}", token(&form.body), "a".repeat(2_001)),
                &cookies,
            )
            .await;
        assert_eq!(refused.status, 422, "{}", refused.body);
        assert!(
            refused.body.contains("The limit is 2000"),
            "{}",
            refused.body
        );

        let rows: i64 =
            sqlx::query_scalar("SELECT count(*) FROM ritrovo_profile WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(rows, 0, "a refused bio was stored anyway");

        // The rejection is correctable: a second attempt with the token from the
        // page that refused the first one succeeds.
        let fixed = app
            .post_form(
                PATH,
                &format!("_token={}&bio=short+enough", token(&refused.body)),
                &cookies,
            )
            .await;
        assert_eq!(fixed.status, 200, "{}", fixed.body);
    });
}

/// HTML in a bio is stored verbatim and rendered as text, never as markup.
///
/// The kernel does not sanitize a plugin's response body, and this page renders
/// one member's text to that member. It is still the difference between a bio
/// and a script tag.
#[test]
fn html_in_a_bio_is_escaped_on_the_way_out() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::prepare_app_database(&pool, &[PLUGIN]).await;
        host::import_demo_config(&pool).await;
        let app = host::app().await;
        let (cookies, _) = app.login_as_new_member(&[]).await;

        let form = app.get(PATH, &cookies).await;
        let saved = app
            .post_form(
                PATH,
                &format!(
                    "_token={}&bio={}",
                    token(&form.body),
                    encode("<script>alert(1)</script>")
                ),
                &cookies,
            )
            .await;
        assert_eq!(saved.status, 200, "{}", saved.body);
        assert!(
            !saved.body.contains("<script>alert(1)</script>"),
            "a script tag was rendered: {}",
            saved.body
        );
        assert!(saved.body.contains("&lt;script&gt;"), "{}", saved.body);
    });
}

/// Anonymous gets nothing, and the kernel is what says so.
///
/// The route's `permission` is checked in `plugin_api.rs` before dispatch, so
/// the plugin never runs. **401, not a redirect to the login page**: a plugin
/// route cannot set a response header, so nothing in this path can send a
/// `Location`, and a visitor who followed a link to their profile gets a status
/// code rather than the login form. The brief asks for the redirect; it is not
/// available. See FRICTION.md, `G-PLUGIN-ROUTE-NO-HEADERS`.
#[test]
fn an_anonymous_visitor_is_refused_by_the_kernel_before_the_plugin_runs() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::prepare_app_database(&pool, &[PLUGIN]).await;
        host::import_demo_config(&pool).await;
        let app = host::app().await;

        let anonymous = app.get(PATH, "").await;
        assert_eq!(
            anonymous.status, 401,
            "an anonymous visitor was not refused: {}",
            anonymous.body
        );
        assert!(
            !anonymous.body.contains(r#"name="bio""#),
            "an anonymous visitor was served the form: {}",
            anonymous.body
        );
    });
}

/// The CSRF token out of a rendered form.
fn token(html: &str) -> String {
    let marker = r#"name="_token" value=""#;
    let start = html
        .find(marker)
        .unwrap_or_else(|| panic!("no _token input in: {html}"))
        + marker.len();
    let rest = &html[start..];
    let end = rest.find('"').unwrap();
    rest[..end].to_string()
}

/// Percent-encode a value for a form body.
///
/// Only what these tests actually post. Deliberately not a general encoder: a
/// half-right one used widely is worse than a small one used knowingly.
fn encode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char);
            }
            b' ' => out.push('+'),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}
