#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `ritrovo_translate` on the real kernel: install (including the dependency
//! check), enable, the API check, language detection on save, the badge and
//! switcher on view, the translation queue over the kernel's own
//! `item_translation`, and the drafting cron job.
//!
//! The fixtures are `tests/fixtures/{italian,english}-conference.json`.
//!
//! Detection is asserted on what the kernel stores. It used to be asserted on
//! what `tap_item_insert` returned, and a second test pinned that the kernel
//! kept none of it (`G-ITEM-INSERT-OUTPUT-DISCARDED`); detection moved to
//! `tap_item_presave`, whose `fields` the kernel merges, so the stored item is
//! now the thing to check and the insert tap is gone.
//!
//! No model is called here. The drafting job is driven with the variable off
//! (it must do nothing) and on with no provider configured (it must store
//! nothing and say why); the drafting logic itself is unit-tested against a
//! mock provider in `src/assist.rs`.
//!
//! Needs Postgres (`DATABASE_URL`) and the assembled overlay:
//!
//! ```text
//! cargo build --target wasm32-wasip1 --release && scripts/assemble-overlay.sh
//! ```

#[path = "../../../tests/host/mod.rs"]
mod host;

use sqlx::PgPool;
use trovato_kernel::tap::UserContext;

use host::serial;

const PLUGIN: &str = "ritrovo_translate";

/// The kernel plugin `ritrovo_translate` declares as a dependency. It ships in
/// the Trovato image; this repository has no copy of its module.
const DEPENDENCY: &str = "trovato_content_translation";

struct Fixture {
    title: String,
    fields: serde_json::Value,
}

fn fixture(name: &str) -> Fixture {
    let path = host::repo_root().join(format!(
        "plugins/ritrovo_translate/tests/fixtures/{name}.json"
    ));
    let raw = std::fs::read_to_string(&path).unwrap();
    let value = host::json(&raw);
    Fixture {
        title: value["title"].as_str().unwrap().to_string(),
        fields: value["fields"].clone(),
    }
}

async fn clean_install(pool: &PgPool) {
    host::uninstall(pool, PLUGIN).await;
    trovato_kernel::plugin::status::install_plugin(pool, DEPENDENCY, "0.102.0")
        .await
        .unwrap();
    host::install_and_enable(pool, PLUGIN)
        .await
        .unwrap_or_else(|e| panic!("install {PLUGIN}: {e}"));
    host::import_demo_config(pool).await;
    sqlx::query("DELETE FROM item WHERE type = 'conference'")
        .execute(pool)
        .await
        .unwrap();
}

#[test]
fn the_kernel_refuses_the_install_until_its_dependency_is_installed() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::uninstall(&pool, PLUGIN).await;
        host::uninstall(&pool, DEPENDENCY).await;

        let refused = host::install_and_enable(&pool, PLUGIN).await;
        let error = refused.expect_err("installed without its dependency");
        assert!(
            error.contains(&format!("depends on '{DEPENDENCY}' which is not installed")),
            "{error}"
        );
        assert_eq!(host::plugin_status(&pool, PLUGIN).await, None);

        // The dependency's status row, as `trovato plugin install` leaves it on the
        // released image. Its module is the kernel's and is not in this overlay;
        // the dependency check reads the status table, not the search path.
        trovato_kernel::plugin::status::install_plugin(&pool, DEPENDENCY, "0.102.0")
            .await
            .unwrap();

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

/// The kernel's `item_translation`, as its own `trovato_content_translation`
/// migration creates it.
///
/// That plugin ships in the kernel image and not in this overlay, so its
/// migration cannot be run here the way a Ritrovo plugin's is. The columns are
/// `plugins/trovato_content_translation/migrations/001_create_item_translation.sql`
/// at the pinned revision, verbatim; rows are written only through the kernel's
/// own `ItemService::save_translation`.
async fn kernel_translation_table(pool: &PgPool) {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS item_translation ( \
            item_id UUID NOT NULL, \
            language VARCHAR(12) NOT NULL, \
            title TEXT NOT NULL DEFAULT '', \
            fields JSONB NOT NULL DEFAULT '{}', \
            created BIGINT NOT NULL DEFAULT EXTRACT(EPOCH FROM NOW())::bigint, \
            changed BIGINT NOT NULL DEFAULT EXTRACT(EPOCH FROM NOW())::bigint, \
            PRIMARY KEY (item_id, language))",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("DELETE FROM item_translation")
        .execute(pool)
        .await
        .unwrap();
}

/// What `field_language` the kernel stored for each fixture, created through
/// `ItemService::create`, which dispatches the compiled `tap_item_presave` and
/// merges its `fields`.
#[test]
fn an_italian_conference_is_marked_for_translation_and_an_english_one_is_not() {
    serial(async {
        let pool = host::fresh_pool().await;
        clean_install(&pool).await;
        let disp = host::dispatcher(PLUGIN);
        let items = host::items(&pool, &disp);

        for (name, want) in [("italian-conference", "it"), ("english-conference", "en")] {
            let f = fixture(name);
            let item = host::create_conference(&pool, &items, &f.title, None, f.fields).await;
            let stored: Option<String> =
                sqlx::query_scalar("SELECT fields->>'field_language' FROM item WHERE id = $1")
                    .bind(item.id)
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_eq!(stored.as_deref(), Some(want), "{name}");
            // Marked for translation means: the queue's target is the other
            // language. Italian goes to English, everything else to Italian.
            let target = if want == "it" { "en" } else { "it" };
            assert_ne!(stored.as_deref(), Some(target), "{name}");
        }
    });
}

/// A language someone chose is not second-guessed by the detector.
#[test]
fn a_chosen_language_survives_the_save() {
    serial(async {
        let pool = host::fresh_pool().await;
        clean_install(&pool).await;
        let disp = host::dispatcher(PLUGIN);
        let items = host::items(&pool, &disp);

        let f = fixture("italian-conference");
        let mut fields = f.fields.clone();
        fields["field_language"] = serde_json::json!("en");
        let item = host::create_conference(&pool, &items, &f.title, None, fields).await;
        let stored: Option<String> =
            sqlx::query_scalar("SELECT fields->>'field_language' FROM item WHERE id = $1")
                .bind(item.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(stored.as_deref(), Some("en"));
    });
}

/// The view path renders the badge from the stored language and links both
/// languages at routes the kernel serves.
#[test]
fn each_conference_renders_its_language_badge_and_switcher() {
    serial(async {
        let pool = host::fresh_pool().await;
        clean_install(&pool).await;
        let disp = host::dispatcher(PLUGIN);
        let items = host::items(&pool, &disp);
        let reader = host::visitor(&pool, &["access content"]).await;

        for (name, lang, label) in [
            ("italian-conference", "it", "Italiano"),
            ("english-conference", "en", "English"),
        ] {
            let f = fixture(name);
            let item = host::create_conference(&pool, &items, &f.title, None, f.fields).await;
            let (_, rendered) = items
                .load_for_view(item.id, &reader)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(rendered.len(), 1, "{name}: {rendered:?}");
            let html = &rendered[0];
            assert!(
                html.contains(&format!(
                    "<span class=\"lang-badge\" lang=\"{lang}\">{label}</span>"
                )),
                "{name}: {html}"
            );
            let id = item.id;
            assert!(
                html.contains(&format!("href=\"/item/{id}\"")),
                "{name}: {html}"
            );
            assert!(
                html.contains(&format!("href=\"/it/item/{id}\"")),
                "{name}: {html}"
            );
        }
    });
}

fn queue_request() -> String {
    serde_json::json!({
        "callback": "translation_queue",
        "method": "GET",
        "path": "/admin/content/translations",
        "params": {},
        "query": {},
        "body": "",
        "user_id": "00000000-0000-0000-0000-000000000000",
        "authenticated": true,
        "csrf_token": "",
    })
    .to_string()
}

async fn queue_html(
    pool: &PgPool,
    disp: &std::sync::Arc<trovato_kernel::tap::TapDispatcher>,
) -> String {
    let out = host::dispatch(
        pool,
        disp,
        PLUGIN,
        "tap_api",
        &queue_request(),
        UserContext::background(),
        host::offline_client(),
    )
    .await;
    let response = host::json(&out);
    assert_eq!(response["status"], 200, "{response}");
    response["body"].as_str().unwrap().to_string()
}

/// The queue lists upcoming conferences with no translation in the other
/// language, each linked to the kernel's translate screen in the right target,
/// and a translation saved through the kernel's own writer takes it off.
#[test]
fn the_queue_lists_what_needs_translating_and_drops_what_the_kernel_saved() {
    serial(async {
        let pool = host::fresh_pool().await;
        clean_install(&pool).await;
        kernel_translation_table(&pool).await;
        let disp = host::dispatcher(PLUGIN);
        let items = host::items(&pool, &disp);

        let f = fixture("english-conference");
        let english = host::create_conference(&pool, &items, &f.title, None, f.fields).await;
        let f = fixture("italian-conference");
        let italian = host::create_conference(&pool, &items, &f.title, None, f.fields).await;
        // The importer writes the item's own language column; ItemService::create
        // is handed none here, so set it the way the importer would.
        sqlx::query("UPDATE item SET language = 'it' WHERE id = $1")
            .bind(italian.id)
            .execute(&pool)
            .await
            .unwrap();
        let past = host::create_conference(
            &pool,
            &items,
            "Long Gone Conf",
            None,
            serde_json::json!({"field_start_date": "2001-01-01", "field_end_date": "2001-01-02"}),
        )
        .await;

        let html = queue_html(&pool, &disp).await;
        assert!(
            html.contains(&format!(
                "href=\"/admin/content/{}/translate/it\"",
                english.id
            )),
            "{html}"
        );
        assert!(
            html.contains(&format!(
                "href=\"/admin/content/{}/translate/en\"",
                italian.id
            )),
            "{html}"
        );
        assert!(
            !html.contains(&past.id.to_string()),
            "a past conference is queued: {html}"
        );
        assert!(html.contains("data-total=\"2\""), "{html}");

        items
            .save_translation(
                english.id,
                "it",
                "Giornata del software libero",
                &serde_json::json!({}),
            )
            .await
            .unwrap();
        let html = queue_html(&pool, &disp).await;
        assert!(
            !html.contains(&english.id.to_string()),
            "still queued after saving: {html}"
        );
        assert!(html.contains(&italian.id.to_string()), "{html}");
        assert!(html.contains("data-total=\"1\""), "{html}");
    });
}

async fn cron(
    pool: &PgPool,
    disp: &std::sync::Arc<trovato_kernel::tap::TapDispatcher>,
) -> serde_json::Value {
    host::json(
        &host::dispatch(
            pool,
            disp,
            PLUGIN,
            "tap_cron",
            &serde_json::json!({"timestamp": host::now()}).to_string(),
            UserContext::background(),
            host::offline_client(),
        )
        .await,
    )
}

/// Off by default; on, with no provider configured, it stores nothing.
///
/// The second half is what an operator who flips the switch before configuring
/// a provider gets: a cycle that reports its failures and writes no draft. It
/// is also the proof that no test here reaches a model, because none exists.
#[test]
fn drafting_is_off_by_default_and_drafts_nothing_without_a_provider() {
    serial(async {
        let pool = host::fresh_pool().await;
        clean_install(&pool).await;
        kernel_translation_table(&pool).await;
        sqlx::query("DELETE FROM ritrovo_translation_draft")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "DELETE FROM site_config WHERE key = 'plugin.ritrovo_translate.assist_enabled'",
        )
        .execute(&pool)
        .await
        .unwrap();
        let disp = host::dispatcher(PLUGIN);
        let items = host::items(&pool, &disp);
        let f = fixture("english-conference");
        host::create_conference(&pool, &items, &f.title, None, f.fields).await;

        assert_eq!(
            cron(&pool, &disp).await,
            serde_json::json!({"status": "off"})
        );

        sqlx::query(
            "INSERT INTO site_config (key, value) VALUES ('plugin.ritrovo_translate.assist_enabled', '\"1\"') \
             ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value",
        )
        .execute(&pool)
        .await
        .unwrap();
        let ran = cron(&pool, &disp).await;
        sqlx::query(
            "DELETE FROM site_config WHERE key = 'plugin.ritrovo_translate.assist_enabled'",
        )
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(ran["status"], "ran", "{ran}");
        assert_eq!(ran["drafted"], 0, "{ran}");
        assert_eq!(ran["failed"], 1, "{ran}");
        let drafts: i64 = sqlx::query_scalar("SELECT count(*) FROM ritrovo_translation_draft")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(drafts, 0);
    });
}
