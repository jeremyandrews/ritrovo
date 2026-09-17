#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `ritrovo_access` on the real kernel: install, enable, the API check, the
//! permissions it declares, and the editorial-stage gate as
//! `ItemService::check_access` applies it.
//!
//! The unit tests call `tap_item_access` with hand-built input. This suite puts
//! real conference Items on the tutorial's real `incoming` and `curated` stages
//! and asks the kernel, so it also covers what the unit tests cannot: that the
//! kernel resolves the stage's machine name and passes it, that it consults the
//! plugin at all for an internal stage, and that it lets a Deny win.
//!
//! Needs Postgres (`DATABASE_URL`) and the assembled overlay:
//!
//! ```text
//! cargo build --target wasm32-wasip1 --release && scripts/assemble-overlay.sh
//! ```

#[path = "../../../tests/host/mod.rs"]
mod host;

use trovato_kernel::tap::UserContext;

use host::{CURATED_STAGE, INCOMING_STAGE, serial};

const PLUGIN: &str = "ritrovo_access";

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
        assert!(
            host::applied_migrations(&pool, PLUGIN).await.is_empty(),
            "{PLUGIN} declares no migrations"
        );

        let disp = host::dispatcher(PLUGIN);
        let compiled = disp.runtime().get_plugin(PLUGIN).unwrap();
        compiled
            .info
            .check_api_compatibility()
            .unwrap_or_else(|e| panic!("{e:#}"));
    });
}

#[test]
fn tap_perm_declares_the_editorial_permissions() {
    serial(async {
        let pool = host::fresh_pool().await;
        let disp = host::dispatcher(PLUGIN);
        let output = host::dispatch(
            &pool,
            &disp,
            PLUGIN,
            "tap_perm",
            "{}",
            UserContext::background(),
            host::offline_client(),
        )
        .await;
        let mut names: Vec<String> = host::json(&output)
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["name"].as_str().unwrap().to_string())
            .collect();
        names.sort();
        assert_eq!(
            names,
            [
                "edit any comments",
                "edit conferences",
                "edit own comments",
                "post comments",
                "publish conferences",
                "view curated conferences",
                "view incoming conferences",
            ]
        );
    });
}

#[test]
fn incoming_and_curated_conferences_are_gated_by_permission() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::install_and_enable(&pool, PLUGIN).await.unwrap();
        host::import_tutorial_config(&pool).await;
        sqlx::query("DELETE FROM item WHERE type = 'conference'")
            .execute(&pool)
            .await
            .unwrap();

        let disp = host::dispatcher(PLUGIN);
        let items = host::items(&pool, &disp);
        let dates = serde_json::json!({
            "field_start_date": "2027-03-01",
            "field_end_date": "2027-03-02",
        });
        let incoming = host::create_conference(
            &pool,
            &items,
            "Unreviewed Conf",
            Some(INCOMING_STAGE),
            dates.clone(),
        )
        .await;
        let curated = host::create_conference(
            &pool,
            &items,
            "Reviewed Conf",
            Some(CURATED_STAGE),
            dates.clone(),
        )
        .await;
        let live = host::create_conference(&pool, &items, "Public Conf", None, dates).await;

        let reader = host::visitor(&pool, &["access content"]).await;
        let incoming_viewer =
            host::visitor(&pool, &["access content", "view incoming conferences"]).await;
        let curated_viewer =
            host::visitor(&pool, &["access content", "view curated conferences"]).await;
        let editor = host::visitor(&pool, &["access content", "edit conferences"]).await;

        // (who, which Item, operation, allowed)
        let cases = [
            (
                "anonymous",
                &UserContext::anonymous(),
                &incoming,
                "view",
                false,
            ),
            (
                "anonymous",
                &UserContext::anonymous(),
                &curated,
                "view",
                false,
            ),
            ("reader", &reader, &incoming, "view", false),
            ("reader", &reader, &curated, "view", false),
            ("reader", &reader, &live, "view", true),
            ("incoming viewer", &incoming_viewer, &incoming, "view", true),
            ("incoming viewer", &incoming_viewer, &curated, "view", false),
            (
                "incoming viewer",
                &incoming_viewer,
                &incoming,
                "edit",
                false,
            ),
            ("curated viewer", &curated_viewer, &curated, "view", true),
            ("curated viewer", &curated_viewer, &incoming, "view", false),
            ("curated viewer", &curated_viewer, &curated, "delete", false),
            ("editor", &editor, &incoming, "view", true),
            ("editor", &editor, &curated, "view", true),
            ("editor", &editor, &incoming, "edit", true),
            ("editor", &editor, &curated, "delete", true),
            ("editor", &editor, &live, "update", true),
            ("reader", &reader, &live, "update", false),
        ];

        let mut wrong = Vec::new();
        for (who, user, item, operation, allowed) in cases {
            let got = items.check_access(item, operation, user).await.unwrap();
            if got != allowed {
                wrong.push(format!(
                    "{who} {operation} '{}': got {got}, want {allowed}",
                    item.title
                ));
            }
        }
        assert!(
            wrong.is_empty(),
            "access decisions wrong:\n{}",
            wrong.join("\n")
        );

        // And the view path turns a Deny into the absence a 404 is built from.
        assert!(
            items
                .load_for_view(incoming.id, &reader)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            items
                .load_for_view(incoming.id, &incoming_viewer)
                .await
                .unwrap()
                .is_some()
        );
    });
}
