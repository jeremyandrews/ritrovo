#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `ritrovo_notify` on the real kernel: install, enable, the migration, the
//! subscription routes driven the way a browser drives them, and the comment
//! thread `demo/config` now opens to every signed-in member.
//!
//! Everything here runs against the **compiled** module loaded from the
//! assembled overlay and the kernel's own router, session and database. No SDK
//! stub answers anything: a test that passes because a host function returned a
//! default is a test that proves nothing about what ships.
//!
//! Needs Postgres (`DATABASE_URL`) and Redis (`REDIS_URL`), and the assembled
//! overlay:
//!
//! ```text
//! cargo build --target wasm32-wasip1 --release && scripts/assemble-overlay.sh
//! ```

#[path = "../../../tests/host/mod.rs"]
mod host;

use host::serial;
use uuid::Uuid;

const PLUGIN: &str = "ritrovo_notify";

/// The path the plugin registers, with `{uid}` filled in.
fn list_path(uid: Uuid) -> String {
    format!("/user/{uid}/subscriptions")
}

fn subscribe_path(uid: Uuid) -> String {
    format!("/user/{uid}/subscriptions/subscribe")
}

fn unsubscribe_path(uid: Uuid) -> String {
    format!("/user/{uid}/subscriptions/unsubscribe")
}

/// Pull the plugin's own CSRF token out of a page it rendered.
///
/// The token is single-use and minted per request, so a write has to be driven
/// from the page that offered it, exactly as a browser does.
fn token_from(html: &str) -> String {
    let marker = r#"name="_token" value=""#;
    let start = html
        .find(marker)
        .unwrap_or_else(|| panic!("no _token in the page: {html}"))
        + marker.len();
    let rest = &html[start..];
    let end = rest.find('"').expect("the token is quoted");
    rest[..end].to_string()
}

/// How many rows the kernel's table holds for this member and item.
async fn rows(pool: &sqlx::PgPool, user_id: Uuid, item_id: Uuid) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM user_subscriptions WHERE user_id = $1 AND item_id = $2",
    )
    .bind(user_id)
    .bind(item_id)
    .fetch_one(pool)
    .await
    .unwrap()
}

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

        // The module loaded at all, which is the check that the manifest's
        // `host_interfaces` covers what the compiled module imports: the kernel
        // builds this plugin's linker from that list and refuses a module that
        // imports anything else. The list is `db` and `logging`, derived from
        // the import section rather than from reading the source.
        assert_eq!(
            compiled
                .info
                .capabilities
                .as_ref()
                .map(|c| c.host_interfaces.clone())
                .unwrap_or_default(),
            ["db", "logging"]
        );
    });
}

#[test]
fn the_migrations_apply_and_apply_again_as_a_no_op() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::uninstall(&pool, PLUGIN).await;
        sqlx::raw_sql("DROP TABLE IF EXISTS pending_notifications")
            .execute(&pool)
            .await
            .unwrap();

        let first = host::run_migrations(&pool, PLUGIN).await;
        assert_eq!(first, ["migrations/001_create_pending_notifications.sql"]);
        assert_eq!(host::applied_migrations(&pool, PLUGIN).await, first);

        assert!(
            host::run_migrations(&pool, PLUGIN).await.is_empty(),
            "the runner re-applied a recorded migration"
        );
        host::replay_migrations_raw(&pool, PLUGIN).await;

        let columns: Vec<String> = sqlx::query_scalar(
            "SELECT column_name::text FROM information_schema.columns \
             WHERE table_name = 'pending_notifications' ORDER BY ordinal_position",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            columns,
            [
                "id",
                "user_id",
                "event_type",
                "item_id",
                "payload",
                "created",
                "sent",
                "sent_at"
            ]
        );
    });
}

/// The declaration the eventual queue fix attaches to.
///
/// Asserted through the kernel's dispatcher rather than by calling the tap
/// function, because the shape that matters is the JSON the kernel parses. It
/// reads a JSON **array** and returns a concurrency of 1 for anything else, so
/// the object this plugin used to return was ignored in full.
#[test]
fn the_queue_declaration_loads_and_the_kernel_reads_it_as_an_array() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::install_and_enable(&pool, PLUGIN).await.unwrap();

        let disp = host::dispatcher(PLUGIN);
        assert!(
            !disp.registry().get_handlers("tap_queue_info").is_empty(),
            "tap_queue_info is not registered"
        );

        let output = host::dispatch(
            &pool,
            &disp,
            PLUGIN,
            "tap_queue_info",
            "{}",
            host::visitor(&pool, &[]).await,
            host::offline_client(),
        )
        .await;
        let declared = host::json(&output);
        // `#[plugin_tap]` may hand back a JSON string wrapping the value, which
        // is the same double encoding the kernel's own parser accepts.
        let declared = match declared.as_str() {
            Some(inner) => host::json(inner),
            None => declared,
        };

        let queues = declared
            .as_array()
            .unwrap_or_else(|| panic!("the kernel reads an array; got {declared}"));
        assert_eq!(queues.len(), 1, "{declared}");
        assert_eq!(queues[0]["name"], "ritrovo_notifications");
        assert_eq!(queues[0]["concurrency"], 1);

        // Keys the kernel ignores are not declared. `max_retries` and
        // `retry_delay_seconds` were here and read by nothing.
        let object = queues[0].as_object().unwrap();
        assert_eq!(object.len(), 2, "{object:?}");
    });
}

/// The permission the routes are gated on is one a role can actually hold.
///
/// This is the tap that was dispatched nowhere at 0.102. It fails against a
/// kernel that does not dispatch `tap_perm`, which is the point: it is the
/// assertion that the gap closed.
#[test]
fn the_declared_permission_is_stored_where_the_kernel_looks() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::install_and_enable(&pool, PLUGIN).await.unwrap();

        let disp = host::dispatcher(PLUGIN);
        let output = host::dispatch(
            &pool,
            &disp,
            PLUGIN,
            "tap_perm",
            "{}",
            host::visitor(&pool, &[]).await,
            host::offline_client(),
        )
        .await;

        let registry =
            trovato_kernel::plugin::permission_registry::PluginPermissionRegistry::from_tap_results(
                vec![(PLUGIN.to_string(), output)],
            );
        trovato_kernel::plugin::permission_registry::persist(&pool, &registry)
            .await
            .expect("persist declared permissions");

        let stored: Option<String> = sqlx::query_scalar(
            "SELECT plugin FROM plugin_permission WHERE name = 'manage own subscriptions'",
        )
        .fetch_optional(&pool)
        .await
        .unwrap();
        assert_eq!(stored.as_deref(), Some(PLUGIN));
    });
}

/// Subscribe, then unsubscribe, through the real routes as a real member.
///
/// The row is read back from `user_subscriptions`, the kernel's own table, which
/// is the half of step 3's first decision that had to be proved rather than
/// argued: the database policy admits a kernel table a plugin did not create.
#[test]
fn a_member_subscribes_and_unsubscribes_and_the_row_lands_in_the_kernels_table() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::prepare_app_database(&pool, &[PLUGIN]).await;
        host::import_demo_config(&pool).await;
        let app = host::app().await;

        let disp = host::dispatcher(PLUGIN);
        let items = host::items(&pool, &disp);
        let conference = host::create_conference(
            &pool,
            &items,
            "Subscribable Conf",
            None,
            serde_json::json!({ "field_start_date": "2027-05-01" }),
        )
        .await;

        let (cookies, uid) = app.login_as_new_member(&[]).await;

        // The list starts empty and offers the Subscribe form for this
        // conference, because the page was asked for with it in focus.
        let page = app
            .get(
                &format!("{}?conference={}", list_path(uid), conference.id),
                &cookies,
            )
            .await;
        assert_eq!(page.status, 200, "{}", page.body);
        assert!(
            page.body.contains("not following any conferences"),
            "{}",
            page.body
        );
        assert!(page.body.contains(">Subscribe"), "{}", page.body);
        assert_eq!(rows(&pool, uid, conference.id).await, 0);

        // Subscribe, posting the way a browser posts: form-urlencoded, the
        // token in the body, no header.
        let token = token_from(&page.body);
        let posted = app
            .post_form(
                &subscribe_path(uid),
                &format!("_token={token}&item_id={}", conference.id),
                &cookies,
            )
            .await;
        assert_eq!(posted.status, 200, "{}", posted.body);
        assert!(posted.body.contains("now subscribed to"), "{}", posted.body);
        assert_eq!(
            rows(&pool, uid, conference.id).await,
            1,
            "the row did not land in user_subscriptions"
        );

        // The list now names it, and offers Unsubscribe instead.
        let page = app.get(&list_path(uid), &cookies).await;
        assert!(page.body.contains("Subscribable Conf"), "{}", page.body);
        assert!(page.body.contains(">Unsubscribe"), "{}", page.body);

        // Unsubscribe, and it is gone.
        let token = token_from(&page.body);
        let posted = app
            .post_form(
                &unsubscribe_path(uid),
                &format!("_token={token}&item_id={}", conference.id),
                &cookies,
            )
            .await;
        assert_eq!(posted.status, 200, "{}", posted.body);
        assert!(
            posted.body.contains("no longer subscribed"),
            "{}",
            posted.body
        );
        assert_eq!(rows(&pool, uid, conference.id).await, 0, "the row survived");

        sqlx::query("DELETE FROM item WHERE id = $1")
            .bind(conference.id)
            .execute(&pool)
            .await
            .unwrap();
    });
}

/// One member's list is not another's.
#[test]
fn the_list_is_the_members_own_and_another_members_is_refused() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::prepare_app_database(&pool, &[PLUGIN]).await;
        host::import_demo_config(&pool).await;
        let app = host::app().await;

        let disp = host::dispatcher(PLUGIN);
        let items = host::items(&pool, &disp);
        let conference = host::create_conference(
            &pool,
            &items,
            "Private Conf",
            None,
            serde_json::json!({ "field_start_date": "2027-06-01" }),
        )
        .await;

        // One member subscribes.
        let (cookies, uid) = app.login_as_new_member(&[]).await;
        let page = app
            .get(
                &format!("{}?conference={}", list_path(uid), conference.id),
                &cookies,
            )
            .await;
        let token = token_from(&page.body);
        app.post_form(
            &subscribe_path(uid),
            &format!("_token={token}&item_id={}", conference.id),
            &cookies,
        )
        .await;

        // Their own list shows it.
        let own = app.get(&list_path(uid), &cookies).await;
        assert_eq!(own.status, 200);
        assert!(own.body.contains("Private Conf"), "{}", own.body);

        // Another member, holding the same permission, is refused — and is not
        // told what is on the list they were refused.
        let (other_cookies, other_uid) = app.login_as_new_member(&[]).await;
        let theirs = app.get(&list_path(uid), &other_cookies).await;
        assert_eq!(theirs.status, 403, "{}", theirs.body);
        assert!(!theirs.body.contains("Private Conf"), "{}", theirs.body);

        // And their own list is genuinely empty rather than the first member's.
        let own_other = app.get(&list_path(other_uid), &other_cookies).await;
        assert_eq!(own_other.status, 200);
        assert!(
            !own_other.body.contains("Private Conf"),
            "{}",
            own_other.body
        );

        sqlx::query("DELETE FROM item WHERE id = $1")
            .bind(conference.id)
            .execute(&pool)
            .await
            .unwrap();
    });
}

/// An anonymous visitor is offered nothing and can post nothing.
#[test]
fn an_anonymous_visitor_sees_no_control_and_cannot_post_to_the_endpoint() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::prepare_app_database(&pool, &[PLUGIN]).await;
        host::import_demo_config(&pool).await;
        let app = host::app().await;

        let disp = host::dispatcher(PLUGIN);
        let items = host::items(&pool, &disp);
        let conference = host::create_conference(
            &pool,
            &items,
            "Anonymous Conf",
            None,
            serde_json::json!({ "field_start_date": "2027-07-01" }),
        )
        .await;
        let stranger = Uuid::now_v7();

        // The list is not served to an anonymous visitor at all, so there is no
        // page on which a Subscribe control could appear. 401 rather than 403
        // because they are not signed in; a redirect to the login form is what
        // this would be if a plugin route could set a header
        // (`G-PLUGIN-ROUTE-NO-HEADERS`).
        let page = app.get(&list_path(stranger), "").await;
        assert_eq!(page.status, 401, "{}", page.body);
        assert!(!page.body.contains("Subscribe"), "{}", page.body);

        // The conference page offers no subscribe control either, which is the
        // finding rather than the feature: the item template's context carries
        // no viewer, so a control rendered there would be shown to everyone.
        let conference_page = app.get(&format!("/item/{}", conference.id), "").await;
        assert_eq!(conference_page.status, 200);
        assert!(
            !conference_page.body.contains("ritrovo-subscribe-form"),
            "a subscribe control reached an anonymous conference page"
        );

        // A direct post is refused, with nothing written.
        let posted = app
            .post_form(
                &subscribe_path(stranger),
                &format!("_token=forged&item_id={}", conference.id),
                "",
            )
            .await;
        assert!(
            posted.status == 401 || posted.status == 403,
            "an anonymous post was answered {}: {}",
            posted.status,
            posted.body
        );
        let total: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM user_subscriptions WHERE item_id = $1")
                .bind(conference.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(total, 0, "an anonymous post wrote a subscription");

        sqlx::query("DELETE FROM item WHERE id = $1")
            .bind(conference.id)
            .execute(&pool)
            .await
            .unwrap();
    });
}

/// A subscription to something that is not a published conference is refused.
///
/// Without this the endpoint would take any uuid, and a row naming an item the
/// member cannot see would put its title on their list the day it was published.
#[test]
fn a_subscription_to_something_that_is_not_a_published_conference_is_refused() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::prepare_app_database(&pool, &[PLUGIN]).await;
        host::import_demo_config(&pool).await;
        let app = host::app().await;

        let disp = host::dispatcher(PLUGIN);
        let items = host::items(&pool, &disp);
        // A real conference, only so the page has a form on it to take a token
        // from: an empty list offers no control and therefore no token, which is
        // correct and is why this fixture exists.
        let real = host::create_conference(
            &pool,
            &items,
            "Real Conf",
            None,
            serde_json::json!({ "field_start_date": "2027-09-01" }),
        )
        .await;

        let (cookies, uid) = app.login_as_new_member(&[]).await;

        for bogus in [Uuid::now_v7().to_string(), "not-a-uuid".to_string()] {
            // A fresh page each time: the token is single-use.
            let page = app
                .get(
                    &format!("{}?conference={}", list_path(uid), real.id),
                    &cookies,
                )
                .await;
            let token = token_from(&page.body);

            let posted = app
                .post_form(
                    &subscribe_path(uid),
                    &format!("_token={token}&item_id={bogus}"),
                    &cookies,
                )
                .await;
            assert_eq!(posted.status, 404, "{bogus} was accepted: {}", posted.body);
            assert!(
                posted.body.contains("could not be found"),
                "{}",
                posted.body
            );
        }

        let total: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM user_subscriptions WHERE user_id = $1")
                .bind(uid)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(total, 0, "a refused subscription still wrote a row");

        sqlx::query("DELETE FROM item WHERE id = $1")
            .bind(real.id)
            .execute(&pool)
            .await
            .unwrap();
    });
}

/// A signed-in member posts a comment on a conference, and the reply nests.
///
/// This is story 37.1 driven through the kernel's real comment route, and what
/// it really tests is `demo/config`: the member holds `post comments` only
/// because the authenticated role grants it, which is a thing no role could
/// hold before 0.103.0 dispatched `tap_perm`. Before that change this same
/// request was a 403.
///
/// **Where the token comes from, and why that is honest.** A browser would take
/// it from the comment form. This `AppState` renders no Tera template at all —
/// the suites configure no `TEMPLATES_DIR`, so the kernel's
/// `elements/comments.html` is not on disk here and the item page falls back to
/// a bare title — so there is no form to read one from. The token is taken from
/// a page this plugin renders instead, which works because there is one CSRF
/// store per session and both routes use it: the kernel mints into
/// `csrf_tokens` on the session and `require_csrf` checks the same set. Only the
/// field name differs, `_token` for a plugin route and `_csrf` for a kernel one.
/// So the token is genuinely session-bound and genuinely single-use, and the
/// route under test is the kernel's own.
///
/// **What this does not cover, and where that is covered.** The rendered thread:
/// the depth class, the reply link, the form a signed-in member sees instead of
/// "Log in to post a comment". None of it can render without the kernel's
/// templates. `scripts/verify-demo.sh` asserts all of it against the released
/// image, where those templates are the ones the site actually serves.
#[test]
fn a_member_posts_a_comment_on_a_conference_and_a_reply_nests_under_it() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::prepare_app_database(&pool, &[PLUGIN]).await;
        host::import_demo_config(&pool).await;
        let app = host::app().await;
        host::enable_comments(&app.state, &pool).await;

        let disp = host::dispatcher(PLUGIN);
        let items = host::items(&pool, &disp);
        let conference = host::create_conference(
            &pool,
            &items,
            "Commentable Conf",
            None,
            serde_json::json!({ "field_start_date": "2027-08-01" }),
        )
        .await;

        let (cookies, uid) = app.login_as_new_member(&[]).await;

        // One token per post: the store is single-use.
        let token = |app: &'static host::App, cookies: String, uid: Uuid| async move {
            let page = app
                .get(
                    &format!("{}?conference={}", list_path(uid), conference.id),
                    &cookies,
                )
                .await;
            token_from(&page.body)
        };

        let first = token(app, cookies.clone(), uid).await;
        let posted = app
            .post_form(
                &format!("/api/item/{}/comments", conference.id),
                &format!("_csrf={first}&parent_id=&body=A+first+comment"),
                &cookies,
            )
            .await;
        assert!(
            posted.status < 400,
            "a signed-in member could not comment: {} {}",
            posted.status,
            posted.body
        );

        let (parent, depth): (Uuid, i16) = sqlx::query_as(
            "SELECT id, depth FROM comment WHERE item_id = $1 ORDER BY created DESC LIMIT 1",
        )
        .bind(conference.id)
        .fetch_one(&pool)
        .await
        .expect("the comment landed");
        assert_eq!(depth, 0, "a comment with no parent is not at the root");

        let second = token(app, cookies.clone(), uid).await;
        let replied = app
            .post_form(
                &format!("/api/item/{}/comments", conference.id),
                &format!("_csrf={second}&parent_id={parent}&body=A+threaded+reply"),
                &cookies,
            )
            .await;
        assert!(
            replied.status < 400,
            "the reply was refused: {} {}",
            replied.status,
            replied.body
        );

        // The kernel's trigger computes depth from the parent, and the thread
        // reader is what the template renders from: this is the nesting.
        let thread = app
            .state
            .comments_if_enabled()
            .expect("comments are enabled")
            .list_for_item(conference.id)
            .await
            .expect("read the thread");
        assert_eq!(thread.len(), 2, "{thread:?}");
        assert_eq!(thread[0].depth, 0);
        assert_eq!(
            thread[1].depth, 1,
            "the reply is not nested under its parent"
        );
        assert_eq!(thread[1].parent_id, Some(parent));
        assert!(thread[0].body.contains("A first comment"));
        assert!(thread[1].body.contains("A threaded reply"));

        sqlx::query("DELETE FROM item WHERE id = $1")
            .bind(conference.id)
            .execute(&pool)
            .await
            .unwrap();
    });
}

/// An anonymous visitor cannot comment, which is the other half of the grant.
#[test]
fn an_anonymous_visitor_cannot_comment() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::prepare_app_database(&pool, &[PLUGIN]).await;
        host::import_demo_config(&pool).await;
        let app = host::app().await;
        host::enable_comments(&app.state, &pool).await;

        let disp = host::dispatcher(PLUGIN);
        let items = host::items(&pool, &disp);
        let conference = host::create_conference(
            &pool,
            &items,
            "Anonymous Comment Conf",
            None,
            serde_json::json!({ "field_start_date": "2027-10-01" }),
        )
        .await;

        let posted = app
            .post_form(
                &format!("/api/item/{}/comments", conference.id),
                "_csrf=forged&parent_id=&body=Spam",
                "",
            )
            .await;
        // Not a success. A form post is answered with a redirect back to the
        // item carrying `?comment=error` rather than a status, which is why
        // this asks "not 2xx" rather than naming one code, and why the row
        // count below is the assertion that actually matters: a redirect that
        // had written the comment would look identical here.
        assert!(
            !(200..300).contains(&posted.status),
            "an anonymous comment was accepted: {} {}",
            posted.status,
            posted.body
        );

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM comment WHERE item_id = $1")
            .bind(conference.id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(total, 0, "an anonymous post wrote a comment");

        sqlx::query("DELETE FROM item WHERE id = $1")
            .bind(conference.id)
            .execute(&pool)
            .await
            .unwrap();
    });
}
