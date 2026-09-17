#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `ritrovo_notify` on the real kernel: install, enable, the API check, and the
//! notification table's migration.
//!
//! Needs Postgres (`DATABASE_URL`) and the assembled overlay:
//!
//! ```text
//! cargo build --target wasm32-wasip1 --release && scripts/assemble-overlay.sh
//! ```

#[path = "../../../tests/host/mod.rs"]
mod host;

use host::serial;

const PLUGIN: &str = "ritrovo_notify";

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
