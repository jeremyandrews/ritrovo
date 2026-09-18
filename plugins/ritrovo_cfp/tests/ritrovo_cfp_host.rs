#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `ritrovo_cfp` on the real kernel: install, enable, the API check, and the
//! badge each CFP deadline renders through `ItemService::load_for_view`, which is
//! the path a conference page takes.
//!
//! The deadlines are stored the way the kernel stores a `Date` field and the way
//! `ritrovo_importer` writes one, `YYYY-MM-DD`, and the countdown is read against
//! the database clock through the kernel's `db` host function, which the unit
//! tests cannot reach: natively the SDK's `query_raw` stub returns no rows, so
//! there the tap renders nothing at all.
//!
//! Needs Postgres (`DATABASE_URL`) and the assembled overlay:
//!
//! ```text
//! cargo build --target wasm32-wasip1 --release && scripts/assemble-overlay.sh
//! ```

#[path = "../../../tests/host/mod.rs"]
mod host;

use host::serial;

const PLUGIN: &str = "ritrovo_cfp";

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

/// One conference per deadline, each rendered through the view path.
///
/// Each date is taken relative to the database's own today, which is the clock
/// the plugin reads. Every Item's `changed` is then pushed 400 days into the
/// past, because the badge used to count from `changed` and must not any more.
#[test]
fn each_deadline_renders_the_badge_for_its_distance() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::install_and_enable(&pool, PLUGIN).await.unwrap();
        host::import_demo_config(&pool).await;
        sqlx::query("DELETE FROM item WHERE type = 'conference'")
            .execute(&pool)
            .await
            .unwrap();

        let disp = host::dispatcher(PLUGIN);
        let items = host::items(&pool, &disp);
        let reader = host::visitor(&pool, &["access content"]).await;

        // (days from today, the badge it must render)
        let cases: &[(Option<i64>, Option<&str>)] = &[
            (
                Some(40),
                Some("cfp-badge--open\">CFP Open \u{2014} 40 days left"),
            ),
            (
                Some(15),
                Some("cfp-badge--open\">CFP Open \u{2014} 15 days left"),
            ),
            (
                Some(14),
                Some("cfp-badge--closing\">CFP Closing \u{2014} 14 days left"),
            ),
            (
                Some(8),
                Some("cfp-badge--closing\">CFP Closing \u{2014} 8 days left"),
            ),
            (
                Some(7),
                Some("cfp-badge--urgent\">CFP Urgent \u{2014} 7 days left"),
            ),
            (
                Some(2),
                Some("cfp-badge--urgent\">CFP Urgent \u{2014} 2 days left"),
            ),
            (
                Some(1),
                Some("cfp-badge--urgent\">CFP Urgent \u{2014} 1 day left"),
            ),
            (Some(0), Some("cfp-badge--urgent\">CFP Closes Today!")),
            (Some(-1), None),
            (Some(-30), None),
            (None, None),
        ];

        for (offset, expected) in cases {
            let item = host::create_conference(
                &pool,
                &items,
                &format!("CFP {offset:?}"),
                None,
                serde_json::json!({
                    "field_start_date": "2027-06-01",
                    "field_end_date": "2027-06-02",
                }),
            )
            .await;

            let date: Option<String> = match offset {
                Some(offset) => Some(
                    sqlx::query_scalar(
                        "SELECT to_char((now() AT TIME ZONE 'UTC')::date + $1::int, 'YYYY-MM-DD')",
                    )
                    .bind(*offset as i32)
                    .fetch_one(&pool)
                    .await
                    .unwrap(),
                ),
                None => None,
            };
            sqlx::query(
                "UPDATE item SET changed = changed - 400 * 86400, \
                 fields = CASE WHEN $1::text IS NULL THEN fields \
                          ELSE fields || jsonb_build_object('field_cfp_end_date', $1::text) END \
                 WHERE id = $2",
            )
            .bind(date)
            .bind(item.id)
            .execute(&pool)
            .await
            .unwrap();

            // A fresh service, so the view reads the row rather than the
            // create path's cached copy.
            let (_, rendered) = host::items(&pool, &disp)
                .load_for_view(item.id, &reader)
                .await
                .unwrap()
                .unwrap_or_else(|| panic!("conference {offset:?} was not viewable"));

            match expected {
                Some(badge) => {
                    assert_eq!(rendered.len(), 1, "{offset:?}: {rendered:?}");
                    assert!(
                        rendered[0].starts_with("<span class=\"cfp-badge ")
                            && rendered[0].contains(badge),
                        "deadline {offset:?} days out rendered {rendered:?}, want {badge}"
                    );
                }
                None => assert!(
                    rendered.is_empty(),
                    "deadline {offset:?} should render no badge, got {rendered:?}"
                ),
            }
        }
    });
}

/// The kernel saves a conference whose CFP closes after it ends, and the tap
/// cannot stop it.
///
/// This is a **pin on a gap**, not a feature. `tap_item_presave` is dispatched —
/// this test proves that much, because the plugin is loaded and the save goes
/// through the real `ItemService` — and the kernel then reads a `fields` object
/// out of the result, merges it, and writes the row regardless. There is no
/// return value that means "no". See FRICTION.md, `G-PRESAVE-CANNOT-REFUSE`.
///
/// The rule itself is unit-tested in the plugin (`date_complaint`) and enforced
/// by the submission form `ritrovo_forms` serves, which is allowed to say no.
///
/// **If this test starts failing because the create returned an error, the
/// kernel has grown a veto and STATUS 36.5 / P6 should be reopened.**
#[test]
fn the_kernel_saves_contradictory_cfp_dates_because_no_tap_can_refuse() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::install_and_enable(&pool, PLUGIN).await.unwrap();
        host::import_demo_config(&pool).await;
        sqlx::query("DELETE FROM item WHERE type = 'conference'")
            .execute(&pool)
            .await
            .unwrap();

        let disp = host::dispatcher(PLUGIN);
        let items = host::items(&pool, &disp);

        // A CFP closing the day after the conference ends: the one thing the
        // rule forbids.
        let conference = host::create_conference(
            &pool,
            &items,
            "Contradictory Conf",
            None,
            serde_json::json!({
                "field_start_date": "2027-05-01",
                "field_end_date": "2027-05-03",
                "field_cfp_end_date": "2027-05-04",
            }),
        )
        .await;

        let stored: serde_json::Value = sqlx::query_scalar("SELECT fields FROM item WHERE id = $1")
            .bind(conference.id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            stored["field_cfp_end_date"],
            serde_json::json!("2027-05-04"),
            "the contradictory date was neither refused nor rewritten, which is \
             what this test exists to record"
        );
    });
}
