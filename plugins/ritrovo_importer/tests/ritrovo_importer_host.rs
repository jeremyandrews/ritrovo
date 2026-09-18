#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `ritrovo_importer` on the real kernel, end to end, without the internet.
//!
//! The unit tests in `src/lib.rs` settle validation, slugs and HTML escaping. What
//! is asserted here only shows up with a host involved:
//!
//! - the kernel installs and enables the module and accepts its `api_version`;
//! - its migrations apply through the kernel's runner, and again as a no-op;
//! - `tap_install` resolves the topic taxonomy, fetches confs.tech and queues the
//!   batches, through the kernel's `http` and `queue` host functions;
//! - the kernel's queue drain runs `tap_queue_worker` over those batches and the
//!   conferences land, with a second topic merged into a conference two topic
//!   files both list;
//! - replaying the import changes nothing;
//! - the two admin screens answer 401 to an anonymous visitor and 200, with the
//!   HTML they say they serve, to an administrator, through the kernel's router.
//!
//! confs.tech is served from `tests/fixtures/confs-tech/` by
//! [`host::FixtureServer`]; see there for how the unmodified plugin's real URL
//! reaches a local file.
//!
//! Needs Postgres (`DATABASE_URL`), Redis (`REDIS_URL`, for the admin-screen
//! sessions) and the assembled overlay:
//!
//! ```text
//! cargo build --target wasm32-wasip1 --release && scripts/assemble-overlay.sh
//! ```

#[path = "../../../tests/host/mod.rs"]
mod host;

use std::sync::Arc;

use sqlx::PgPool;
use trovato_kernel::cron::CronService;
use trovato_kernel::tap::UserContext;

use host::{FixtureServer, serial};

const PLUGIN: &str = "ritrovo_importer";

/// The host and path prefix the plugin's `DATA_BASE_URL` names.
const UPSTREAM_HOST: &str = "raw.githubusercontent.com";
const UPSTREAM_PREFIX: &str = "/tech-conferences/conference-data/main/conferences";

/// The 25 confs.tech topics the plugin walks, per year.
const TOPIC_COUNT: usize = 25;

/// How many of the 25 confs.tech feeds `data/confs-tech-topics.json` points at a
/// term in Ritrovo's topic tree. Three do not: the brief's tree has no General,
/// Open Source or Testing term. `tap_install` resolves one term per mapped feed,
/// counting the two that both point at Mobile separately, so it reports 22.
const MAPPED_TERMS: i64 = 22;

/// What the fixture files hold. `rust.json` and `data.json` are upstream
/// verbatim; `css.json` repeats one `rust.json` conference and adds one the
/// importer must reject. See `tests/fixtures/confs-tech/README.md`.
const RUST_CONFERENCES: i64 = 5;
const DATA_CONFERENCES: i64 = 98;
const DISTINCT_VALID_CONFERENCES: i64 = RUST_CONFERENCES + DATA_CONFERENCES;
/// `rust.json` is one batch, `data.json` two (50 per batch), `css.json` one.
const FIXTURE_BATCHES: i64 = 4;

async fn fixtures() -> FixtureServer {
    FixtureServer::start(
        UPSTREAM_HOST,
        UPSTREAM_PREFIX,
        host::repo_root().join("plugins/ritrovo_importer/tests/fixtures/confs-tech"),
    )
    .await
}

/// The importer installed, and none of its work left over from a previous test.
async fn clean_install(pool: &PgPool) {
    host::install_and_enable(pool, PLUGIN)
        .await
        .unwrap_or_else(|e| panic!("install {PLUGIN}: {e}"));
    for sql in [
        "DELETE FROM item WHERE type = 'conference'",
        "DELETE FROM ritrovo_state",
        "DELETE FROM plugin_queue WHERE plugin_name = 'ritrovo_importer'",
    ] {
        sqlx::query(sql).execute(pool).await.unwrap();
    }
    // Before `tap_install`, as in the demo: the plugin resolves the topic terms
    // by label once, at install, and caches the ids.
    host::import_demo_config(pool).await;
}

async fn run_tap_install(pool: &PgPool, server: &FixtureServer) -> serde_json::Value {
    let disp = host::dispatcher(PLUGIN);
    host::json(
        &host::dispatch(
            pool,
            &disp,
            PLUGIN,
            "tap_install",
            "{}",
            UserContext::background(),
            server.client(),
        )
        .await,
    )
}

/// Drain the plugin queue the way the cron route does, until nothing is left
/// to claim, and return the totals.
async fn drain(pool: &PgPool) -> trovato_kernel::cron::QueueDrainStats {
    // The drain itself never touches Redis; `CronService` holds a client for the
    // kernel's other queues, and opening one does not connect.
    let redis = redis::Client::open(host::redis_url()).unwrap();
    let mut cron = CronService::new(redis, pool.clone());
    cron.set_tap_dispatcher(host::dispatcher(PLUGIN));
    let cron = Arc::new(cron);

    let mut total = trovato_kernel::cron::QueueDrainStats::default();
    loop {
        let stats = cron
            .drain_plugin_queues()
            .await
            .expect("drain plugin queues");
        let handled = stats.succeeded + stats.retried + stats.dead_lettered + stats.errors;
        total.succeeded += stats.succeeded;
        total.retried += stats.retried;
        total.dead_lettered += stats.dead_lettered;
        total.errors += stats.errors;
        if handled == 0 {
            return total;
        }
    }
}

async fn count(pool: &PgPool, sql: &str) -> i64 {
    sqlx::query_scalar(sql).fetch_one(pool).await.unwrap()
}

async fn queued(pool: &PgPool) -> i64 {
    count(
        pool,
        "SELECT count(*) FROM plugin_queue WHERE plugin_name = 'ritrovo_importer'",
    )
    .await
}

async fn conferences(pool: &PgPool) -> i64 {
    count(pool, "SELECT count(*) FROM item WHERE type = 'conference'").await
}

async fn topics_of(pool: &PgPool, title: &str) -> Vec<String> {
    let topics: serde_json::Value = sqlx::query_scalar(
        "SELECT fields->'field_topics' FROM item WHERE type = 'conference' AND title = $1",
    )
    .bind(title)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("no conference titled {title}: {e}"));
    let mut topics: Vec<String> = topics
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t.as_str().unwrap().to_string())
        .collect();
    topics.sort();
    topics
}

// ===========================================================================
// Install, enable, migrations
// ===========================================================================

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

        // Loading is where the kernel compiles the module and checks its imports
        // against `[capabilities] host_interfaces`, in both directions.
        let disp = host::dispatcher(PLUGIN);
        let compiled = disp.runtime().get_plugin(PLUGIN).unwrap();
        compiled
            .info
            .check_api_compatibility()
            .unwrap_or_else(|e| panic!("{e:#}"));
        for tap in [
            "tap_install",
            "tap_perm",
            "tap_menu",
            "tap_api",
            "tap_cron",
            "tap_queue_info",
            "tap_queue_worker",
        ] {
            assert!(
                disp.registry()
                    .get_handlers(tap)
                    .iter()
                    .any(|h| h.plugin.info.name == PLUGIN),
                "{tap} is not registered"
            );
        }
    });
}

#[test]
fn the_migrations_apply_and_apply_again_as_a_no_op() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::uninstall(&pool, PLUGIN).await;
        sqlx::raw_sql(
            "DROP TABLE IF EXISTS ritrovo_state; \
             DROP INDEX IF EXISTS uniq_item_conference_source_id;",
        )
        .execute(&pool)
        .await
        .unwrap();

        let first = host::run_migrations(&pool, PLUGIN).await;
        assert_eq!(
            first,
            [
                "migrations/001_create_ritrovo_state.sql",
                "migrations/002_dedup_conferences.sql",
                "migrations/003_remove_conferences_alias.sql",
                "migrations/004_restore_blanked_source_ids.sql",
            ]
        );
        assert_eq!(host::applied_migrations(&pool, PLUGIN).await, first);

        assert!(
            host::run_migrations(&pool, PLUGIN).await.is_empty(),
            "the runner re-applied a recorded migration"
        );
        host::replay_migrations_raw(&pool, PLUGIN).await;

        // The index is what turns concurrent inserts of one conference into an
        // ON CONFLICT merge rather than a duplicate; without it the worker's
        // upsert is a SQL error.
        let index = count(
            &pool,
            "SELECT count(*) FROM pg_indexes WHERE indexname = 'uniq_item_conference_source_id'",
        )
        .await;
        assert_eq!(index, 1);
    });
}

// ===========================================================================
// The import
// ===========================================================================

#[test]
fn tap_install_resolves_the_taxonomy_fetches_confs_tech_and_queues_the_batches() {
    serial(async {
        let pool = host::fresh_pool().await;
        clean_install(&pool).await;
        let server = fixtures().await;

        let report = run_tap_install(&pool, &server).await;

        assert_eq!(report["status"], "ok", "{report}");
        assert_eq!(report["errors"], 0, "{report}");
        assert_eq!(report["queued"], FIXTURE_BATCHES, "{report}");
        assert_eq!(report["discovered_terms"], MAPPED_TERMS, "{report}");
        assert_eq!(queued(&pool).await, FIXTURE_BATCHES);

        // Every request went to the one upstream path, once per topic per year
        // from 2015 through the current year, and nowhere else.
        let current_year: i64 = count(&pool, "SELECT EXTRACT(YEAR FROM now())::bigint").await;
        let requests = server.requests();
        assert_eq!(
            requests.len() as i64,
            (current_year - 2015 + 1) * TOPIC_COUNT as i64,
            "unexpected request count: {requests:?}"
        );
        assert!(
            requests.iter().all(|p| p.starts_with(UPSTREAM_PREFIX)),
            "a request left the upstream path: {requests:?}"
        );
        assert!(requests.contains(&format!("{UPSTREAM_PREFIX}/2026/rust.json")));

        // ETags are kept for the files that existed, so cron can ask for changes
        // only; and the resolved term ids are cached for the worker.
        let etags = count(
            &pool,
            "SELECT count(*) FROM ritrovo_state WHERE name LIKE 'etag.%.2026'",
        )
        .await;
        assert_eq!(etags, 3);
        let rust_term: String =
            sqlx::query_scalar("SELECT value FROM ritrovo_state WHERE name = 'topic_term.rust'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(rust_term, host::topic_term(&pool, "Rust").await);
    });
}

#[test]
fn the_queue_worker_drains_the_fixture_batches_into_conferences() {
    serial(async {
        let pool = host::fresh_pool().await;
        clean_install(&pool).await;
        let server = fixtures().await;
        run_tap_install(&pool, &server).await;

        let stats = drain(&pool).await;

        assert_eq!(stats.succeeded, FIXTURE_BATCHES as u64, "{stats:?}");
        assert_eq!(stats.retried, 0, "{stats:?}");
        assert_eq!(stats.dead_lettered, 0, "{stats:?}");
        assert_eq!(queued(&pool).await, 0, "the queue did not drain");

        // Every valid conference, once. The backwards-dated entry is rejected.
        assert_eq!(conferences(&pool).await, DISTINCT_VALID_CONFERENCES);
        assert_eq!(
            count(
                &pool,
                "SELECT count(*) FROM item WHERE type = 'conference' AND title LIKE 'Backwards%'"
            )
            .await,
            0
        );

        // TokioConf is in rust.json and css.json: one Item, both topics.
        assert_eq!(
            count(
                &pool,
                "SELECT count(*) FROM item WHERE type = 'conference' AND title = 'TokioConf'"
            )
            .await,
            1
        );
        let mut both = vec![
            host::topic_term(&pool, "Rust").await,
            host::topic_term(&pool, "CSS").await,
        ];
        both.sort();
        assert_eq!(topics_of(&pool, "TokioConf").await, both);

        // What an imported conference looks like: published, on Live, with the
        // source fields and a stable source id.
        let row: (i16, String, serde_json::Value) = sqlx::query_as(
            "SELECT status, stage_id::text, fields FROM item \
             WHERE type = 'conference' AND title = 'TokioConf'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.0, 1);
        assert_eq!(row.1, trovato_sdk::prelude::LIVE_STAGE_UUID);
        assert_eq!(row.2["field_start_date"], "2026-04-20");
        assert_eq!(row.2["field_cfp_end_date"], "2025-12-08");
        assert_eq!(row.2["field_city"], "Portland, OR");
        assert_eq!(row.2["field_source_id"], "tokioconf-2026-04-20-portland-or");
    });
}

#[test]
fn replaying_the_import_adds_no_conference_and_no_duplicate() {
    serial(async {
        let pool = host::fresh_pool().await;
        clean_install(&pool).await;
        let server = fixtures().await;
        run_tap_install(&pool, &server).await;
        drain(&pool).await;
        let before: Vec<(String, String)> = sqlx::query_as(
            "SELECT id::text, fields->>'field_source_id' FROM item \
             WHERE type = 'conference' ORDER BY 2",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(before.len() as i64, DISTINCT_VALID_CONFERENCES);

        // The whole install again, twice: every file fetched and queued, every
        // batch worked. Twice because one replay only updates; it is the replay
        // after an update that finds out whether the update kept the dedup key.
        for _ in 0..2 {
            let report = run_tap_install(&pool, &server).await;
            assert_eq!(report["queued"], FIXTURE_BATCHES, "{report}");
            let stats = drain(&pool).await;
            assert_eq!(stats.succeeded, FIXTURE_BATCHES as u64, "{stats:?}");
        }

        let after: Vec<(String, String)> = sqlx::query_as(
            "SELECT id::text, fields->>'field_source_id' FROM item \
             WHERE type = 'conference' ORDER BY 2",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            after, before,
            "a replay changed the set of conference Items"
        );
        let mut both = vec![
            host::topic_term(&pool, "Rust").await,
            host::topic_term(&pool, "CSS").await,
        ];
        both.sort();
        assert_eq!(topics_of(&pool, "TokioConf").await, both);
    });
}

// ===========================================================================
// The admin screens
// ===========================================================================

#[test]
fn the_admin_screens_are_401_anonymous_and_200_html_for_an_administrator() {
    serial(async {
        let pool = host::fresh_pool().await;
        clean_install(&pool).await;
        let server = fixtures().await;
        run_tap_install(&pool, &server).await;
        drain(&pool).await;
        host::prepare_app_database(&pool, &[PLUGIN]).await;

        let app = host::app().await;
        assert!(app.state.is_plugin_enabled(PLUGIN));
        let admin = app.login_as_new_admin().await;

        for path in ["/admin/content/conferences", "/admin/config/importer"] {
            let anonymous = app.get(path, "").await;
            assert_eq!(
                anonymous.status, 401,
                "{path} anonymous: {}",
                anonymous.body
            );
            assert_eq!(host::json(&anonymous.body)["error"], "Access denied");
        }

        let list = app.get("/admin/content/conferences", &admin).await;
        assert_eq!(list.status, 200, "{}", list.body);
        assert!(
            list.content_type.starts_with("text/html"),
            "{}",
            list.content_type
        );
        assert!(list.body.starts_with("<!doctype html>"), "{}", list.body);
        assert!(list.body.contains("<h1>Conferences</h1>"), "{}", list.body);
        assert!(
            list.body.contains(&format!(
                "<p>{DISTINCT_VALID_CONFERENCES} conference(s) imported.</p>"
            )),
            "{}",
            list.body
        );
        // Newest first, so page one holds the autumn conferences.
        assert!(list.body.contains("<td>RustConf</td>"), "{}", list.body);
        assert!(list.body.contains("Page 1 of 3."), "{}", list.body);

        let page_three = app.get("/admin/content/conferences?page=3", &admin).await;
        assert_eq!(page_three.status, 200);
        assert!(
            page_three.body.contains("Page 3 of 3."),
            "{}",
            page_three.body
        );

        let status = app.get("/admin/config/importer", &admin).await;
        assert_eq!(status.status, 200, "{}", status.body);
        assert!(
            status.content_type.starts_with("text/html"),
            "{}",
            status.content_type
        );
        assert!(
            status.body.contains("<h1>Conference Import</h1>"),
            "{}",
            status.body
        );
        assert!(
            status.body.contains("<h2>Import status</h2>"),
            "{}",
            status.body
        );
        // Terms, not feeds: 22 feeds map to 21 distinct terms, because android
        // and ios both mean Mobile, and three feeds mean no term at all.
        assert!(
            status
                .body
                .contains("<dt>Resolved topic terms</dt><dd>21 of 21</dd>"),
            "{}",
            status.body
        );
        assert!(
            status.body.contains("<dt>Cached ETags</dt><dd>3</dd>"),
            "{}",
            status.body
        );
        assert!(
            status
                .body
                .contains("<dt>Jobs waiting in the import queue</dt><dd>0</dd>"),
            "{}",
            status.body
        );
        assert!(
            status.body.contains(
                "<dt>Feeds with no topic term</dt><dd>3 (general, opensource, testing)</dd>"
            ),
            "the screen should name the feeds that import untagged: {}",
            status.body
        );
        assert!(
            status.body.contains("<dt>Failed batches</dt><dd>none</dd>"),
            "{}",
            status.body
        );
    });
}

// ===========================================================================
// Repairing what the source-id bug left behind
// ===========================================================================

/// `004_restore_blanked_source_ids.sql` against the state the old
/// `update_conference` left: every updated conference's key blanked, and a
/// second Item for each one the next import saw again.
#[test]
fn migration_004_restores_blanked_keys_and_merges_the_duplicates_they_let_in() {
    serial(async {
        let pool = host::fresh_pool().await;
        clean_install(&pool).await;
        let server = fixtures().await;
        run_tap_install(&pool, &server).await;
        drain(&pool).await;

        let keys = |pool: PgPool| async move {
            sqlx::query_as::<_, (String, String)>(
                "SELECT id::text, fields->>'field_source_id' FROM item \
                 WHERE type = 'conference' ORDER BY 2",
            )
            .fetch_all(&pool)
            .await
            .unwrap()
        };
        let good = keys(pool.clone()).await;
        let rust = host::topic_term(&pool, "Rust").await;
        let data = host::topic_term(&pool, "Data Engineering").await;

        // The damage: blank every key, then add a later copy of RustConf under a
        // second topic, as the next import did.
        sqlx::query(
            "UPDATE item SET fields = fields || '{\"field_source_id\": \"\"}'::jsonb \
             WHERE type = 'conference'",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO item (id, type, title, status, author_id, stage_id, created, changed, fields) \
             SELECT gen_random_uuid(), type, title, status, author_id, stage_id, created + 60, \
                    changed + 60, fields || jsonb_build_object( \
                        'field_source_id', 'rustconf-2026-09-08-montreal', \
                        'field_topics', jsonb_build_array($1::text)) \
             FROM item WHERE type = 'conference' AND title = 'RustConf'",
        )
        .bind(&data)
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(conferences(&pool).await, DISTINCT_VALID_CONFERENCES + 1);

        let sql = std::fs::read_to_string(
            host::plugin_dir(PLUGIN).join("migrations/004_restore_blanked_source_ids.sql"),
        )
        .unwrap();
        sqlx::raw_sql(&sql).execute(&pool).await.unwrap();

        // Every key back exactly as the plugin computed it, on the original Item.
        assert_eq!(keys(pool.clone()).await, good);
        let mut merged = vec![rust, data];
        merged.sort();
        assert_eq!(topics_of(&pool, "RustConf").await, merged);

        // And replaying the repair is a no-op.
        sqlx::raw_sql(&sql).execute(&pool).await.unwrap();
        assert_eq!(keys(pool.clone()).await, good);
    });
}

/// A batch the worker cannot process is retried and then dead-lettered, and the
/// reason it failed is still readable afterwards.
///
/// This is the half of the brief's "bad data is logged and skipped, not silently
/// dropped" that used to be false. The worker was a `#[plugin_tap]` returning
/// `{"status": "error"}`, and the kernel counts any returned output as success
/// (G-QUEUE-WORKER-ERROR-IS-SUCCESS), so a malformed batch had its job deleted on
/// the first pass: no retry, no dead letter, no row to find afterwards. It is now
/// a `#[plugin_tap_result]`, so an Err reaches the kernel's retry-and-dead-letter
/// path, which already existed and was simply never reachable from here.
///
/// Time is moved rather than waited for: a failed job is rescheduled with
/// exponential backoff, so the test clears `next_attempt_at` between drains to
/// stand in for the minutes passing.
#[test]
fn a_batch_that_cannot_be_processed_retries_then_dead_letters_with_its_reason() {
    host::serial(async {
        let pool = host::fresh_pool().await;
        clean_install(&pool).await;

        // One job whose `conferences` payload is not JSON. Everything else about
        // it is well formed, so it is the worker that rejects it, not the drain.
        sqlx::query(
            "INSERT INTO plugin_queue (plugin_name, queue_name, payload, created_at) \
             VALUES ('ritrovo_importer', 'ritrovo_import', $1, $2)",
        )
        .bind(serde_json::json!({
            "topic": "rust",
            "year": 2026,
            "conferences": "{ this is not the JSON array the worker expects",
        }))
        .bind(host::now())
        .execute(&pool)
        .await
        .unwrap();

        let max_attempts: i32 =
            sqlx::query_scalar("SELECT max_attempts FROM plugin_queue LIMIT 1")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(max_attempts > 1, "a single attempt is not a retry policy");

        // Drain once per attempt, releasing the backoff each time.
        let mut retried = 0u64;
        let mut dead = 0u64;
        for _ in 0..max_attempts {
            let stats = drain(&pool).await;
            retried += stats.retried;
            dead += stats.dead_lettered;
            assert_eq!(stats.succeeded, 0, "a malformed batch must not succeed");
            sqlx::query("UPDATE plugin_queue SET next_attempt_at = 0 WHERE status = 'ready'")
                .execute(&pool)
                .await
                .unwrap();
        }

        assert_eq!(
            retried,
            (max_attempts - 1) as u64,
            "the job should have been retried up to its limit before dying"
        );
        assert_eq!(dead, 1, "the job should have been dead-lettered exactly once");

        // The row is still there, marked dead, with its attempts spent. Never
        // deleted: a job that vanishes is the thing this test exists to prevent.
        let (status, attempts, last_error): (String, i32, Option<String>) = sqlx::query_as(
            "SELECT status, attempts, last_error FROM plugin_queue \
             WHERE plugin_name = 'ritrovo_importer'",
        )
        .fetch_one(&pool)
        .await
        .expect("the dead-lettered job must still be on the queue");
        assert_eq!(status, "dead");
        assert_eq!(attempts, max_attempts);
        assert!(last_error.is_some(), "a dead job with no error is not a report");

        // The kernel's own last_error is a fixed string: a tap's Err value never
        // crosses the ABI, only its negative length does
        // (G-QUEUE-DEAD-LETTER-DISCARDS-THE-PLUGINS-ERROR). So the reason has to
        // come from the plugin's own state, which is what it writes on the way out.
        let reason: String = sqlx::query_scalar(
            "SELECT value FROM ritrovo_state WHERE name = 'failed.rust.2026'",
        )
        .fetch_optional(&pool)
        .await
        .unwrap()
        .expect("the importer must record why the batch failed");
        assert!(
            reason.contains("parse_error"),
            "the recorded reason should say what went wrong, got: {reason}"
        );

        // And nothing was imported from it.
        assert_eq!(conferences(&pool).await, 0);
    });
}
