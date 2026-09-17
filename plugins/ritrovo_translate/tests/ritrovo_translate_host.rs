#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `ritrovo_translate` on the real kernel: install (including the dependency
//! check), enable, the API check, language detection on insert, and the
//! language badge on view.
//!
//! The fixtures are `tests/fixtures/{italian,english}-conference.json`.
//!
//! Detection is asserted twice over, because the kernel and the plugin disagree
//! about what `tap_item_insert` is for. The plugin returns its finding as the
//! tap's output; the kernel discards every `tap_item_insert` output
//! (`FRICTION.md`, `G-ITEM-INSERT-OUTPUT-DISCARDED`). So one test asserts the
//! finding the compiled module reports through the real dispatcher, and another
//! pins that creating the Item records nothing, with a failure message that
//! names the finding, so the day the kernel starts keeping the output a test
//! says what changed.
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
    host::import_tutorial_config(pool).await;
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

/// What the compiled module reports for each fixture, on its stored Item,
/// through the real dispatcher.
#[test]
fn an_italian_conference_is_marked_for_translation_and_an_english_one_is_not() {
    serial(async {
        let pool = host::fresh_pool().await;
        clean_install(&pool).await;
        let disp = host::dispatcher(PLUGIN);
        let items = host::items(&pool, &disp);

        let mut findings = Vec::new();
        for name in ["italian-conference", "english-conference"] {
            let f = fixture(name);
            let item = host::create_conference(&pool, &items, &f.title, None, f.fields).await;
            let output = host::dispatch(
                &pool,
                &disp,
                PLUGIN,
                "tap_item_insert",
                &serde_json::to_string(&item).unwrap(),
                UserContext::background(),
                host::offline_client(),
            )
            .await;
            // The tap returns a String, which the macro JSON-encodes; the finding
            // is the JSON inside it.
            let inner: String = serde_json::from_str(&output).unwrap();
            findings.push(host::json(&inner));
        }

        assert_eq!(findings[0]["detected_language"], "it", "{}", findings[0]);
        assert_eq!(
            findings[0]["translation_status"], "needs_translation",
            "{}",
            findings[0]
        );

        assert_eq!(findings[1]["detected_language"], "en", "{}", findings[1]);
        assert_ne!(
            findings[1]["translation_status"], "needs_translation",
            "{}",
            findings[1]
        );
    });
}

/// Creating the Item fires `tap_item_insert`, and the kernel keeps nothing the
/// tap returned: the stored Item carries no trace of the finding.
#[test]
fn the_kernel_records_nothing_the_insert_tap_returns() {
    serial(async {
        let pool = host::fresh_pool().await;
        clean_install(&pool).await;
        let disp = host::dispatcher(PLUGIN);
        let items = host::items(&pool, &disp);

        let f = fixture("italian-conference");
        let submitted = f.fields.clone();
        let item = host::create_conference(&pool, &items, &f.title, None, f.fields).await;

        let row: (serde_json::Value, Option<String>) =
            sqlx::query_as("SELECT fields, language FROM item WHERE id = $1")
                .bind(item.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        let stored = serde_json::to_string(&row).unwrap();
        assert!(
            !stored.contains("needs_translation") && !stored.contains("detected_language"),
            "the kernel now records tap_item_insert output ({stored}); \
             FRICTION.md G-ITEM-INSERT-OUTPUT-DISCARDED is fixed or changed, and \
             ritrovo_translate should rely on it instead of returning a finding \
             nothing reads"
        );
        assert_eq!(
            row.0, submitted,
            "the stored fields are the submitted fields"
        );
    });
}

/// The view path renders the language badge and the link to the other language.
#[test]
fn each_conference_renders_its_language_badge_and_switcher() {
    serial(async {
        let pool = host::fresh_pool().await;
        clean_install(&pool).await;
        let disp = host::dispatcher(PLUGIN);
        let items = host::items(&pool, &disp);
        let reader = host::visitor(&pool, &["access content"]).await;

        for (name, lang, label, link) in [
            ("italian-conference", "it", "Italiano", "View in English"),
            ("english-conference", "en", "English", "Vedi in italiano"),
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
            assert!(html.contains(link), "{name}: {html}");
        }
    });
}
