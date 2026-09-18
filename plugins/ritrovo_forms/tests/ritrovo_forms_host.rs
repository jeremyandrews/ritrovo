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
const SUBMIT: &str = "/conferences/submit";

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
        for table in ["ritrovo_profile", "ritrovo_form_state"] {
            sqlx::query(&format!("DROP TABLE IF EXISTS {table}"))
                .execute(&pool)
                .await
                .unwrap();
        }

        host::install_and_enable(&pool, PLUGIN).await.unwrap();

        let applied = host::applied_migrations(&pool, PLUGIN).await;
        assert_eq!(
            applied,
            vec![
                "migrations/001_create_ritrovo_profile.sql".to_string(),
                "migrations/002_create_ritrovo_form_state.sql".to_string(),
            ],
            "the kernel records a migration under the path the manifest lists"
        );

        for table in ["ritrovo_profile", "ritrovo_form_state"] {
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM information_schema.tables \
                 WHERE table_name = $1)",
            )
            .bind(table)
            .fetch_one(&pool)
            .await
            .unwrap();
            assert!(exists, "the migration ran and {table} is not there");
        }
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

// ===========================================================================
// Submit a Conference (A6, story 36.4)
// ===========================================================================

/// The whole flow, in order, as a member drives it.
///
/// Three steps, each posted the way a browser posts, and one conference on the
/// Incoming stage at the end of it, carrying every answer and recording who
/// submitted it. The assertions in between are the ones a browser would have had
/// to satisfy: each step renders the next, and the draft id travels in the page
/// while the answers stay on the server.
#[test]
fn the_three_steps_in_order_create_one_incoming_conference_with_every_field() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::prepare_app_database(&pool, &[PLUGIN]).await;
        host::import_demo_config(&pool).await;
        clear_conferences(&pool).await;
        let app = host::app().await;
        let (cookies, user_id) = app.login_as_new_member(&[]).await;

        // Step 1, as served.
        let start = app.get(SUBMIT, &cookies).await;
        assert_eq!(start.status, 200, "{}", start.body);
        assert!(start.body.contains("Step 1 of 3"), "{}", start.body);
        assert!(
            !start.body.contains("<script"),
            "the form carries JavaScript: {}",
            start.body
        );

        // Step 1 posted. The answers go to the server; the page comes back as
        // step 2 carrying nothing but the draft's id.
        let step2 = app
            .post_form(
                SUBMIT,
                &format!(
                    "_token={}&step=1&draft=&name={}&url={}&start_date=2027-05-01\
                     &end_date=2027-05-03&city=Torino&country=Italy",
                    token(&start.body),
                    encode("Rust Fest Torino"),
                    encode("https://example.org"),
                ),
                &cookies,
            )
            .await;
        assert_eq!(step2.status, 200, "{}", step2.body);
        assert!(step2.body.contains("Step 2 of 3"), "{}", step2.body);
        let draft = draft_id(&step2.body);
        assert!(
            !draft.is_empty(),
            "step 2 carries no draft id: {}",
            step2.body
        );

        // The answers are in the table, not in the page.
        let state: serde_json::Value =
            sqlx::query_scalar("SELECT state FROM ritrovo_form_state WHERE id = $1::uuid")
                .bind(&draft)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(state["name"], serde_json::json!("Rust Fest Torino"));
        assert!(
            !step2.body.contains("Rust Fest Torino"),
            "step 2 echoes step 1's answers back into the page: {}",
            step2.body
        );

        // Step 2 posted, review rendered, with everything on it.
        let review = app
            .post_form(
                SUBMIT,
                &format!(
                    "_token={}&step=2&draft={draft}&cfp_url={}&cfp_end_date=2027-03-01\
                     &description={}",
                    token(&step2.body),
                    encode("https://example.org/cfp"),
                    encode("Two days of Rust in Torino."),
                ),
                &cookies,
            )
            .await;
        assert_eq!(review.status, 200, "{}", review.body);
        assert!(review.body.contains("Step 3 of 3"), "{}", review.body);
        for value in ["Rust Fest Torino", "2027-05-01", "Torino", "2027-03-01"] {
            assert!(
                review.body.contains(value),
                "{value} missing: {}",
                review.body
            );
        }
        // And it says what it could not ask for, rather than leaving a reader to
        // wonder where the logo field went.
        assert!(review.body.contains("logo"), "{}", review.body);
        assert!(review.body.contains("topics"), "{}", review.body);

        // Confirmed.
        let done = app
            .post_form(
                SUBMIT,
                &format!("_token={}&step=3&draft={draft}", token(&review.body)),
                &cookies,
            )
            .await;
        assert_eq!(done.status, 200, "{}", done.body);
        assert!(done.body.contains("not on the site yet"), "{}", done.body);

        // One conference, on Incoming, authored by the submitter, with the
        // answers as fields the edit form can read back.
        let row = sqlx::query_as::<
            _,
            (
                uuid::Uuid,
                String,
                i16,
                Option<uuid::Uuid>,
                serde_json::Value,
            ),
        >(
            "SELECT author_id, title, status, stage_id, fields FROM item WHERE type = 'conference'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let (author, title, status, stage, fields) = row;
        assert_eq!(
            author, user_id,
            "the submitter was not recorded as the author"
        );
        assert_eq!(title, "Rust Fest Torino");
        assert_eq!(status, 0, "a submission arrived published");
        assert_eq!(
            stage.map(|s| s.to_string()).as_deref(),
            Some("0193a5a0-0000-7000-8000-000000000002"),
            "the submission did not land on Incoming"
        );
        assert_eq!(fields["field_city"], serde_json::json!("Torino"));
        assert_eq!(fields["field_country"], serde_json::json!("Italy"));
        assert_eq!(fields["field_start_date"], serde_json::json!("2027-05-01"));
        assert_eq!(fields["field_end_date"], serde_json::json!("2027-05-03"));
        assert_eq!(
            fields["field_cfp_end_date"],
            serde_json::json!("2027-03-01")
        );
        assert_eq!(fields["field_online"], serde_json::json!("0"));
        assert!(
            fields["field_description"]
                .as_str()
                .unwrap_or_default()
                .contains("Two days of Rust"),
            "{fields}"
        );

        // The draft is gone, so the same answers cannot be submitted twice.
        let drafts: i64 = sqlx::query_scalar("SELECT count(*) FROM ritrovo_form_state")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(drafts, 0, "the finished draft was left behind");
    });
}

/// Jumping to the end is refused, and the refusal is on the server.
///
/// The draft row records how far it got. Posting step 3 against a draft that has
/// only answered step 1 is refused however the page was edited, because the page
/// is not what decides.
#[test]
fn skipping_a_step_is_refused_and_creates_nothing() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::prepare_app_database(&pool, &[PLUGIN]).await;
        host::import_demo_config(&pool).await;
        clear_conferences(&pool).await;
        let app = host::app().await;
        let (cookies, _) = app.login_as_new_member(&[]).await;

        // Step 1 only.
        let start = app.get(SUBMIT, &cookies).await;
        let step2 = app
            .post_form(
                SUBMIT,
                &format!(
                    "_token={}&step=1&draft=&name=Half+A+Conference\
                     &start_date=2027-05-01&end_date=2027-05-03&country=Italy",
                    token(&start.body)
                ),
                &cookies,
            )
            .await;
        let draft = draft_id(&step2.body);

        // Straight to the confirmation.
        let jumped = app
            .post_form(
                SUBMIT,
                &format!("_token={}&step=3&draft={draft}", token(&step2.body)),
                &cookies,
            )
            .await;
        assert_eq!(jumped.status, 422, "{}", jumped.body);
        assert!(jumped.body.contains("not finished"), "{}", jumped.body);

        let conferences: i64 =
            sqlx::query_scalar("SELECT count(*) FROM item WHERE type = 'conference'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(conferences, 0, "a skipped step still created a conference");
    });
}

/// A draft id is useless to anybody but the member it belongs to.
///
/// Every read is filtered on `user_id`, so a leaked or guessed draft id neither
/// reads somebody's answers nor submits in their name.
#[test]
fn another_members_draft_id_reads_nothing_and_submits_nothing() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::prepare_app_database(&pool, &[PLUGIN]).await;
        host::import_demo_config(&pool).await;
        clear_conferences(&pool).await;
        let app = host::app().await;

        let (alice, _) = app.login_as_new_member(&[]).await;
        let start = app.get(SUBMIT, &alice).await;
        let step2 = app
            .post_form(
                SUBMIT,
                &format!(
                    "_token={}&step=1&draft=&name=Alices+Conference\
                     &start_date=2027-05-01&end_date=2027-05-03&country=Italy",
                    token(&start.body)
                ),
                &alice,
            )
            .await;
        let draft = draft_id(&step2.body);

        let (bob, _) = app.login_as_new_member(&[]).await;

        // Bob opens Alice's draft: he gets an empty step 1, not her answers.
        let peek = app
            .get(&format!("{SUBMIT}?draft={draft}&step=2"), &bob)
            .await;
        assert_eq!(peek.status, 200, "{}", peek.body);
        assert!(
            !peek.body.contains("Alices Conference"),
            "another member's answers were served: {}",
            peek.body
        );

        // And he cannot advance it.
        let steal = app
            .post_form(
                SUBMIT,
                &format!(
                    "_token={}&step=2&draft={draft}&description=Mine+now",
                    token(&peek.body)
                ),
                &bob,
            )
            .await;
        assert_eq!(steal.status, 422, "{}", steal.body);

        let conferences: i64 =
            sqlx::query_scalar("SELECT count(*) FROM item WHERE type = 'conference'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(conferences, 0);
    });
}

/// A step posted without a token is refused by the kernel, before the plugin runs.
#[test]
fn a_submission_step_without_a_token_is_refused() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::prepare_app_database(&pool, &[PLUGIN]).await;
        host::import_demo_config(&pool).await;
        clear_conferences(&pool).await;
        let app = host::app().await;
        let (cookies, _) = app.login_as_new_member(&[]).await;

        let forged = app
            .post_form(
                SUBMIT,
                "step=1&draft=&name=Forged&start_date=2027-05-01&end_date=2027-05-03&country=Italy",
                &cookies,
            )
            .await;
        assert_eq!(forged.status, 403, "{}", forged.body);

        let drafts: i64 = sqlx::query_scalar("SELECT count(*) FROM ritrovo_form_state")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(drafts, 0, "a tokenless step wrote a draft");
    });
}

/// An anonymous visitor cannot reach the form, and is not redirected to log in.
///
/// The gate is the route's `create content` permission, checked by the kernel
/// before dispatch, which is what story 36.4 asks for. The redirect the story
/// also asks for is **not available**: a plugin route cannot set a response
/// header, so nothing in this path can send a `Location`. See FRICTION.md,
/// `G-PLUGIN-ROUTE-NO-HEADERS`.
///
/// **If this starts returning 302, the kernel has grown response headers and
/// that entry should be reopened.**
#[test]
fn an_anonymous_visitor_cannot_reach_the_submission_form() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::prepare_app_database(&pool, &[PLUGIN]).await;
        host::import_demo_config(&pool).await;
        let app = host::app().await;

        let anonymous = app.get(SUBMIT, "").await;
        assert_eq!(
            anonymous.status, 401,
            "an anonymous visitor was served the submission form: {}",
            anonymous.body
        );
        assert!(
            !anonymous.body.contains("Step 1 of 3"),
            "{}",
            anonymous.body
        );
    });
}

/// The CFP rule is enforced here, where something is allowed to say no.
///
/// `ritrovo_cfp`'s presave tap can only report the same contradiction, because
/// no tap can refuse a save (`G-PRESAVE-CANNOT-REFUSE`). This form refuses it
/// before anything reaches the kernel, which is the point of Ritrovo serving it.
#[test]
fn a_cfp_closing_after_the_conference_is_refused_on_step_two() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::prepare_app_database(&pool, &[PLUGIN]).await;
        host::import_demo_config(&pool).await;
        clear_conferences(&pool).await;
        let app = host::app().await;
        let (cookies, _) = app.login_as_new_member(&[]).await;

        let start = app.get(SUBMIT, &cookies).await;
        let step2 = app
            .post_form(
                SUBMIT,
                &format!(
                    "_token={}&step=1&draft=&name=Late+CFP+Conf\
                     &start_date=2027-05-01&end_date=2027-05-03&country=Italy",
                    token(&start.body)
                ),
                &cookies,
            )
            .await;
        let draft = draft_id(&step2.body);

        let refused = app
            .post_form(
                SUBMIT,
                &format!(
                    "_token={}&step=2&draft={draft}&cfp_end_date=2027-05-04",
                    token(&step2.body)
                ),
                &cookies,
            )
            .await;
        assert_eq!(refused.status, 422, "{}", refused.body);
        assert!(
            refused.body.contains("cannot close after"),
            "{}",
            refused.body
        );
        assert!(refused.body.contains("Step 2 of 3"), "{}", refused.body);

        // Correctable: the page that refused carries a fresh token.
        let fixed = app
            .post_form(
                SUBMIT,
                &format!(
                    "_token={}&step=2&draft={draft}&cfp_end_date=2027-03-01",
                    token(&refused.body)
                ),
                &cookies,
            )
            .await;
        assert_eq!(fixed.status, 200, "{}", fixed.body);
        assert!(fixed.body.contains("Step 3 of 3"), "{}", fixed.body);
    });
}

/// Going back changes an answer and keeps the rest.
#[test]
fn going_back_to_step_one_keeps_the_answers_and_the_draft() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::prepare_app_database(&pool, &[PLUGIN]).await;
        host::import_demo_config(&pool).await;
        clear_conferences(&pool).await;
        let app = host::app().await;
        let (cookies, _) = app.login_as_new_member(&[]).await;

        let start = app.get(SUBMIT, &cookies).await;
        let step2 = app
            .post_form(
                SUBMIT,
                &format!(
                    "_token={}&step=1&draft=&name=First+Name\
                     &start_date=2027-05-01&end_date=2027-05-03&country=Italy",
                    token(&start.body)
                ),
                &cookies,
            )
            .await;
        let draft = draft_id(&step2.body);
        let review = app
            .post_form(
                SUBMIT,
                &format!(
                    "_token={}&step=2&draft={draft}&description=Some+text.",
                    token(&step2.body)
                ),
                &cookies,
            )
            .await;
        assert!(review.body.contains("Step 3 of 3"), "{}", review.body);

        // Back to step 1, which comes back filled in.
        let back = app
            .get(&format!("{SUBMIT}?draft={draft}&step=1"), &cookies)
            .await;
        assert_eq!(back.status, 200, "{}", back.body);
        assert!(back.body.contains("First Name"), "{}", back.body);

        // Change the name, and the description typed in step 2 survives.
        let again = app
            .post_form(
                SUBMIT,
                &format!(
                    "_token={}&step=1&draft={draft}&name=Second+Name\
                     &start_date=2027-05-01&end_date=2027-05-03&country=Italy",
                    token(&back.body)
                ),
                &cookies,
            )
            .await;
        assert_eq!(again.status, 200, "{}", again.body);

        let state: serde_json::Value =
            sqlx::query_scalar("SELECT state FROM ritrovo_form_state WHERE id = $1::uuid")
                .bind(&draft)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(state["name"], serde_json::json!("Second Name"));
        assert_eq!(state["description"], serde_json::json!("Some text."));

        // And the review is still reachable: revising step 1 did not rewind the
        // draft to step 1.
        let step: i16 =
            sqlx::query_scalar("SELECT step FROM ritrovo_form_state WHERE id = $1::uuid")
                .bind(&draft)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(step, 2, "revising step 1 made the review unreachable");
    });
}

/// Nothing to do with this plugin should survive between tests.
async fn clear_conferences(pool: &sqlx::PgPool) {
    sqlx::query("DELETE FROM item WHERE type = 'conference'")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("TRUNCATE ritrovo_form_state")
        .execute(pool)
        .await
        .unwrap();
}

/// The draft id out of a rendered step.
fn draft_id(html: &str) -> String {
    let marker = r#"name="draft" value=""#;
    let start = html
        .find(marker)
        .unwrap_or_else(|| panic!("no draft input in: {html}"))
        + marker.len();
    let rest = &html[start..];
    let end = rest.find('"').unwrap();
    rest[..end].to_string()
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
