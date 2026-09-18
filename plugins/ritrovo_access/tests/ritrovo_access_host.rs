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
        host::import_demo_config(&pool).await;
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

/// The four role-and-stage combinations that decide the brief, driven through
/// the real roles rather than through a permission set the test made up.
///
/// The suite above proves the tap decides correctly when the kernel hands it a
/// permission set. It cannot prove that anybody can hold that set, because it
/// builds the set itself — and on this kernel that is precisely the thing most
/// likely to be false, since none of the permissions the brief names can be
/// granted to any role at all (`G-PERM-TAP-NOT-DISPATCHED`).
///
/// So this one imports `demo/config`, lets `tap_install` create the three
/// editorial users with their real roles, and resolves each viewer's permissions
/// the way a request does. If a role file stops granting what the plugin reads,
/// or the substitution drifts, this fails and the other suite still passes.
#[test]
fn the_real_roles_grant_what_the_editorial_workflow_needs() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::uninstall(&pool, PLUGIN).await;
        host::install_and_enable(&pool, PLUGIN).await.unwrap();
        host::import_demo_config(&pool).await;

        // tap_install is what creates editor_alice, publisher_bob and
        // viewer_carol, active and with roles. Dispatch it here rather than
        // inserting users by hand, so this also covers that it works.
        host::dispatch(
            &pool,
            &host::dispatcher(PLUGIN),
            PLUGIN,
            "tap_install",
            "{}",
            UserContext::background(),
            host::offline_client(),
        )
        .await;

        sqlx::query("DELETE FROM item WHERE type = 'conference'")
            .execute(&pool)
            .await
            .unwrap();

        let disp = host::dispatcher(PLUGIN);
        let items = host::items(&pool, &disp);
        let dates = serde_json::json!({
            "field_start_date": "2027-03-01",
            "field_end_date": "2027-03-02",
            "field_editor_notes": "Ask the organisers about the CFP deadline.",
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

        let reader = host::visitor_named(&pool, "viewer_carol").await;
        let editor = host::visitor_named(&pool, "editor_alice").await;
        let publisher = host::visitor_named(&pool, "publisher_bob").await;
        // The real anonymous visitor, not `UserContext::anonymous()`: that one
        // carries no permissions at all, so it would fail the kernel's
        // published-content check for a reason that has nothing to do with
        // stages. The seeded `anonymous` user resolves through the anonymous
        // role, which is where `access content` comes from.
        let anonymous = host::visitor_named(&pool, "anonymous").await;

        // The roles have to be real before anything below means anything.
        assert!(
            editor.permissions.iter().any(|p| p == "edit any content"),
            "editor_alice does not hold the permission the plugin reads as \
             editorial standing; demo/config/role.*.yml and EDITOR_PERMISSIONS \
             have drifted apart. She holds: {:?}",
            editor.permissions
        );
        assert!(
            publisher
                .permissions
                .iter()
                .any(|p| p == "delete any content"),
            "publisher_bob does not hold the permission that separates a \
             publisher from an editor. He holds: {:?}",
            publisher.permissions
        );
        assert!(
            !reader.permissions.iter().any(|p| p == "edit any content"),
            "viewer_carol holds an editor's permission, so the roles no longer \
             differ. She holds: {:?}",
            reader.permissions
        );

        // The four that matter: each internal stage, seen by someone with
        // editorial standing and by someone without.
        let cases = [
            ("anonymous", &anonymous, &incoming, "view", false),
            ("anonymous", &anonymous, &curated, "view", false),
            ("viewer_carol", &reader, &incoming, "view", false),
            ("viewer_carol", &reader, &curated, "view", false),
            ("editor_alice", &editor, &incoming, "view", true),
            ("editor_alice", &editor, &curated, "view", true),
            ("publisher_bob", &publisher, &incoming, "view", true),
            ("publisher_bob", &publisher, &curated, "view", true),
            // Live is public to anyone holding `access content`, which every
            // role here does, and the kernel answers before asking the plugin.
            ("anonymous", &anonymous, &live, "view", true),
            ("viewer_carol", &reader, &live, "view", true),
            // Reading an internal stage is not permission to change it.
            ("viewer_carol", &reader, &incoming, "edit", false),
            ("viewer_carol", &reader, &live, "edit", false),
            ("editor_alice", &editor, &incoming, "edit", true),
            ("editor_alice", &editor, &curated, "delete", true),
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
            "the real roles decide wrongly:\n{}",
            wrong.join("\n")
        );
    });
}

/// `editor_notes` is removed from the Item before anything renders it.
///
/// This is the test that says the field hiding is access control rather than
/// presentation: it never looks at HTML. It asks the kernel for the Item as each
/// viewer sees it and checks the field is not in `fields` at all. A template
/// that hid the value would pass no part of this.
#[test]
fn editor_notes_is_stripped_from_the_item_a_reader_is_given() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::uninstall(&pool, PLUGIN).await;
        host::install_and_enable(&pool, PLUGIN).await.unwrap();
        host::import_demo_config(&pool).await;
        host::dispatch(
            &pool,
            &host::dispatcher(PLUGIN),
            PLUGIN,
            "tap_install",
            "{}",
            UserContext::background(),
            host::offline_client(),
        )
        .await;

        sqlx::query("DELETE FROM item WHERE type = 'conference'")
            .execute(&pool)
            .await
            .unwrap();

        let disp = host::dispatcher(PLUGIN);
        let items = host::items(&pool, &disp);
        let notes = "Ask the organisers about the CFP deadline.";
        let live = host::create_conference(
            &pool,
            &items,
            "Public Conf",
            None,
            serde_json::json!({
                "field_start_date": "2027-03-01",
                "field_end_date": "2027-03-02",
                "field_city": "Trieste",
                "field_editor_notes": notes,
            }),
        )
        .await;

        let reader = host::visitor_named(&pool, "viewer_carol").await;
        let editor = host::visitor_named(&pool, "editor_alice").await;

        // `load_for_view` hands back the Item and whatever the view taps
        // appended; the Item is the half that matters here.
        let (as_reader, _) = items
            .load_for_view(live.id, &reader)
            .await
            .unwrap()
            .expect("a published Live conference is visible to a reader");
        assert!(
            as_reader.fields.get("field_editor_notes").is_none(),
            "editor_notes reached a reader; fields were {:?}",
            as_reader.fields
        );
        // The rest of the item is untouched: this hides one field, not the page.
        assert_eq!(
            as_reader.fields.get("field_city").and_then(|v| v.as_str()),
            Some("Trieste"),
            "hiding editor_notes took another field with it"
        );

        let (as_editor, _) = items
            .load_for_view(live.id, &editor)
            .await
            .unwrap()
            .expect("a published Live conference is visible to an editor");
        assert_eq!(
            as_editor
                .fields
                .get("field_editor_notes")
                .and_then(|v| v.as_str()),
            Some(notes),
            "an editor could not see editor_notes"
        );
    });
}
