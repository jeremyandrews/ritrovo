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
/// So this one imports `demo/config`, creates the three editorial users with
/// their real roles as the demo bootstrap does, and resolves each viewer's permissions
/// the way a request does. If a role file stops granting what the plugin reads,
/// or the substitution drifts, this fails and the other suite still passes.
#[test]
fn the_real_roles_grant_what_the_editorial_workflow_needs() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::uninstall(&pool, PLUGIN).await;
        host::install_and_enable(&pool, PLUGIN).await.unwrap();
        host::import_demo_config(&pool).await;

        // editor_alice, publisher_bob and viewer_carol, active and with
        // their real roles, made as the demo bootstrap makes them.
        host::create_demo_users(&pool).await;

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

/// Publishing on the editorial screen needs the kernel's `publish content`, the
/// permission the kernel itself requires to change what is published.
///
/// The screen moves stages with a raw `UPDATE`, so the kernel's own check never
/// runs on it and the plugin's gate is the only one. A role with every editor's
/// permission and `delete any content`, the old stand-in for a publisher, but
/// without `publish content`, is refused and the conference stays on Curated.
/// The real publisher role, which holds `publish content`, moves it to Live.
#[test]
fn publishing_on_the_editorial_screen_needs_publish_content() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::prepare_app_database(&pool, &[PLUGIN]).await;
        host::import_demo_config(&pool).await;
        let app = host::app().await;
        host::clear_rate_limits().await;

        sqlx::query("DELETE FROM item WHERE type = 'conference'")
            .execute(&pool)
            .await
            .unwrap();
        let items = host::items(&pool, &host::dispatcher(PLUGIN));
        let curated = host::create_conference(
            &pool,
            &items,
            "Gatekept Conf",
            Some(CURATED_STAGE),
            serde_json::json!({
                "field_start_date": "2027-03-01",
                "field_end_date": "2027-03-02",
            }),
        )
        .await;

        // A role that is everything a publisher used to be on this screen
        // except the kernel's publish permission.
        let role = "ritrovo-test-deleter";
        sqlx::query("DELETE FROM roles WHERE name = $1")
            .bind(role)
            .execute(&pool)
            .await
            .unwrap();
        let role_id = uuid::Uuid::now_v7();
        sqlx::query("INSERT INTO roles (id, name) VALUES ($1, $2)")
            .bind(role_id)
            .bind(role)
            .execute(&pool)
            .await
            .unwrap();
        for permission in [
            "access content",
            "access administration pages",
            "edit any content",
            "delete any content",
        ] {
            sqlx::query("INSERT INTO role_permissions (role_id, permission) VALUES ($1, $2)")
                .bind(role_id)
                .bind(permission)
                .execute(&pool)
                .await
                .unwrap();
        }

        let stage_of = |id: uuid::Uuid| {
            let pool = pool.clone();
            async move {
                sqlx::query_scalar::<_, uuid::Uuid>("SELECT stage_id FROM item WHERE id = $1")
                    .bind(id)
                    .fetch_one(&pool)
                    .await
                    .unwrap()
                    .to_string()
            }
        };
        let publish = |cookies: String| {
            let app = &app;
            async move {
                let queue = app
                    .get("/admin/content/editorial?stage=curated", &cookies)
                    .await;
                assert_eq!(queue.status, 200, "{}", queue.body);
                let body = format!(
                    "_token={}&from=curated&to=live&id={}",
                    csrf(&queue.body),
                    curated.id
                );
                app.post_form("/admin/content/editorial?stage=curated", &body, &cookies)
                    .await
            }
        };

        let (deleter, _) = app.login_as_new_member(&[role]).await;
        let refused = publish(deleter).await;
        assert_eq!(
            refused.status, 403,
            "a role without `publish content` was allowed to publish: {}",
            refused.body
        );
        assert_eq!(
            stage_of(curated.id).await,
            CURATED_STAGE,
            "the refused publish moved the conference anyway"
        );

        let (publisher, _) = app.login_as_new_member(&["publisher"]).await;
        let published = publish(publisher).await;
        assert_eq!(published.status, 200, "{}", published.body);
        assert_eq!(
            stage_of(curated.id).await,
            trovato_kernel::models::stage::LIVE_STAGE_ID.to_string(),
            "the publisher's publish did not move the conference: {}",
            published.body
        );

        sqlx::query("DELETE FROM roles WHERE name = $1")
            .bind(role)
            .execute(&pool)
            .await
            .unwrap();
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
        // editor_alice, publisher_bob and viewer_carol, active and with
        // their real roles, made as the demo bootstrap makes them.
        host::create_demo_users(&pool).await;

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

// ===========================================================================
// The conference edit form (A6, story 36.1)
// ===========================================================================
//
// The form an editor uses to change a conference is the KERNEL's, at
// `/item/{id}/edit`, generated from `demo/config/item_type.conference.yml`.
// Ritrovo writes no code for it and this suite writes no code to test it: what
// these two tests do is drive the real route, through the real router, as the
// editor `ritrovo_access` decides may reach it, and record what comes back.
//
// They live in this suite because reaching that form at all is this plugin's
// decision: `tap_item_access` is what turns "holds `edit any content`" into a
// 200 on an internal stage, and a test of the form that ran as an administrator
// would prove nothing about whether an editor can use it.

/// An editor can open the conference edit form, and it is built from the type.
///
/// Every field `item_type.conference.yml` declares gets a widget without anybody
/// writing one, which is the story's "auto-generated from the Item Type
/// definition" and the reason Ritrovo configures rather than codes here.
///
/// Two things are asserted as carefully as the widgets:
///
/// - **`field_topics` has no widget at all.** It is not declared as a field.
///   Through 0.103 it could not be (`G-NO-CATEGORY-REFERENCE-FIELD-KIND`); the
///   pinned kernel has a `RecordReference("category_term")` widget, and
///   declaring the field with it is a feature not yet built, so this pins the
///   form as it is.
/// - **The block editor loads.** `field_description` is a `Blocks` field, the
///   kernel's rich text. Through 0.103 this page rendered the editor's
///   container with no script, so nothing attached
///   (`G-ITEM-EDIT-FORM-LOADS-NO-ASSETS`); since the kernel's one form pipeline
///   the page loads `block-editor.js`, so an editor gets the WYSIWYG.
#[test]
fn the_conference_edit_form_is_generated_from_the_type_and_loads_the_block_editor() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::prepare_app_database(&pool, &[PLUGIN]).await;
        host::import_demo_config(&pool).await;
        let app = host::app().await;

        // A clean slate: the conference type carries a unique index on
        // `field_source_id`, so a fixture left by an earlier test collides.
        sqlx::query("DELETE FROM item WHERE type = 'conference'")
            .execute(&pool)
            .await
            .unwrap();

        let items = host::items(&pool, &host::dispatcher(PLUGIN));
        let conference = host::create_conference(
            &pool,
            &items,
            "Editable Conf",
            None,
            conference_fields(&pool).await,
        )
        .await;

        let (cookies, _) = app.login_as_new_member(&["editor"]).await;
        let form = app
            .get(&format!("/item/{}/edit", conference.id), &cookies)
            .await;
        assert_eq!(form.status, 200, "{}", form.body);

        // Declared, so rendered. Nobody wrote these widgets.
        for field in [
            "field_url",
            "field_start_date",
            "field_end_date",
            "field_city",
            "field_country",
            "field_online",
            "field_cfp_url",
            "field_cfp_end_date",
            "field_description",
            "field_logo",
            // Multi-value fields render one widget per value since the
            // kernel's 0.107 line (G-CARDINALITY-IS-INERT, fixed).
            "field_venue_photos[0]",
            "field_venue_photos[1]",
            "field_schedule_pdf",
            "field_speakers[0]",
            "field_language",
            "field_source_id",
            "field_editor_notes",
        ] {
            assert!(
                form.body.contains(&format!(r#"name="{field}""#)),
                "{field} has no widget on the generated form"
            );
        }

        // Not declared, so not rendered. This is the one that costs data below.
        assert!(
            !form.body.contains(r#"name="field_topics""#),
            "field_topics has a widget now; update STATUS 29.1"
        );

        // The block editor's container is rendered and, since the kernel's
        // one form pipeline, the editor itself is loaded with it: the Blocks
        // field is a WYSIWYG on this form now, not a container nothing fills.
        assert!(
            form.body.contains("data-block-editor"),
            "the Blocks field rendered no editor container: {}",
            form.body
        );
        assert!(
            form.body.contains("block-editor.js"),
            "the edit page no longer loads the block editor: {}",
            form.body
        );
    });
}

/// Saving through that form keeps every field, including the ones it did not
/// render.
///
/// This was a **pin on a loss** through Trovato 0.103: the form built a fresh
/// field map out of the POST body and `Item::update` wrote it whole, so
/// `field_topics` (no widget) and the two multi-value fields (one widget over an
/// array) were gone after any editor's save (`G-FORM-SAVE-DROPS-EVERY-FIELD-THE-
/// FORM-DID-NOT-RENDER`, `G-CARDINALITY-IS-INERT`). Since the kernel's 0.107
/// line a form save starts from the stored fields and overwrites only what the
/// form rendered, a multi-value field posts `name[0]`, `name[1]` and is stored
/// as an array, and a Boolean is stored as a JSON boolean. This now pins that.
#[test]
fn saving_the_edit_form_keeps_every_field_including_the_ones_it_did_not_render() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::prepare_app_database(&pool, &[PLUGIN]).await;
        host::import_demo_config(&pool).await;
        let app = host::app().await;

        sqlx::query("DELETE FROM item WHERE type = 'conference'")
            .execute(&pool)
            .await
            .unwrap();

        let items = host::items(&pool, &host::dispatcher(PLUGIN));
        let before = conference_fields(&pool).await;
        let conference =
            host::create_conference(&pool, &items, "Round Trip Conf", None, before.clone()).await;

        // What went in: three multi-value fields, each a JSON array.
        for field in ["field_topics", "field_speakers", "field_venue_photos"] {
            assert!(
                before[field].is_array(),
                "{field} was not stored as an array to begin with"
            );
        }

        let (cookies, _) = app.login_as_new_member(&["editor"]).await;
        let path = format!("/item/{}/edit", conference.id);
        let form = app.get(&path, &cookies).await;
        assert_eq!(form.status, 200, "{}", form.body);

        // Submit the way a browser does: the fields the form rendered, and only
        // those. A file input posts nothing in a urlencoded body, which is also
        // what a browser sends when the visitor picks no new file.
        let body = [
            form_tokens(&form.body),
            "title=Round+Trip+Conf".to_string(),
            "status=1".to_string(),
            "field_url=https%3A%2F%2Fexample.test".to_string(),
            "field_start_date=2027-05-01".to_string(),
            "field_end_date=2027-05-03".to_string(),
            "field_city=Torino".to_string(),
            "field_country=Italy".to_string(),
            "field_cfp_url=https%3A%2F%2Fexample.test%2Fcfp".to_string(),
            "field_cfp_end_date=2027-03-01".to_string(),
            "field_language=en".to_string(),
            "field_source_id=round-trip".to_string(),
            "field_editor_notes=Checked+the+dates.".to_string(),
            "field_speakers%5B0%5D=0193a5a0-0005-7000-8000-000000000001".to_string(),
            "field_venue_photos%5B0%5D=public%3A%2F%2Fvenue-a.jpg".to_string(),
            "field_venue_photos%5B1%5D=public%3A%2F%2Fvenue-b.jpg".to_string(),
        ]
        .join("&");
        let saved = app.post_form(&path, &body, &cookies).await;
        assert_eq!(saved.status, 200, "the save was refused: {}", saved.body);

        let after: serde_json::Value = sqlx::query_scalar("SELECT fields FROM item WHERE id = $1")
            .bind(conference.id)
            .fetch_one(&pool)
            .await
            .unwrap();

        // Kept: everything the form rendered and the browser posted.
        assert_eq!(after["field_city"], serde_json::json!("Torino"));
        assert_eq!(after["field_country"], serde_json::json!("Italy"));
        assert_eq!(after["field_start_date"], serde_json::json!("2027-05-01"));
        assert_eq!(after["field_end_date"], serde_json::json!("2027-05-03"));
        assert_eq!(
            after["field_editor_notes"],
            serde_json::json!("Checked the dates.")
        );

        // Kept: the one the form could not render, untouched.
        assert_eq!(
            after["field_topics"], before["field_topics"],
            "field_topics was lost on a save: {after}"
        );
        // Kept: the two multi-value fields, as arrays.
        assert_eq!(
            after["field_speakers"], before["field_speakers"],
            "field_speakers did not round-trip: {after}"
        );
        assert_eq!(
            after["field_venue_photos"], before["field_venue_photos"],
            "field_venue_photos did not round-trip: {after}"
        );

        // A Boolean the form showed and the browser left unchecked is stored as
        // JSON `false`, not a string.
        assert_eq!(
            after["field_online"],
            serde_json::json!(false),
            "an unchecked Boolean was not stored as false: {after}"
        );
    });
}

/// Story 35.4 on the pinned kernel: an editor's save writes a revision with its
/// log message, and the same editor can list, preview, compare and revert. At
/// 0.103.0 the history page was a 500 for any item with a revision
/// (`G-REVISION-HISTORY-500`) and compare and preview did not exist
/// (`G-REVISION-NO-COMPARE`). This drives the kernel's own routes, as a
/// browser would, and reads the result back from the item.
#[test]
fn an_editor_lists_previews_compares_and_reverts_revisions() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::prepare_app_database(&pool, &[PLUGIN]).await;
        host::import_demo_config(&pool).await;
        let app = host::app().await;

        sqlx::query("DELETE FROM item WHERE type = 'conference'")
            .execute(&pool)
            .await
            .unwrap();

        let items = host::items(&pool, &host::dispatcher(PLUGIN));
        let fields = conference_fields(&pool).await;
        let conference =
            host::create_conference(&pool, &items, "Revision Conf", None, fields).await;
        let first: uuid::Uuid = sqlx::query_scalar(
            "SELECT id FROM item_revision WHERE item_id = $1 ORDER BY created, id LIMIT 1",
        )
        .bind(conference.id)
        .fetch_one(&pool)
        .await
        .unwrap();

        // The edit: Torino becomes Milano, with a log message.
        let (cookies, _) = app.login_as_new_member(&["editor"]).await;
        let path = format!("/item/{}/edit", conference.id);
        let form = app.get(&path, &cookies).await;
        assert_eq!(form.status, 200, "{}", form.body);
        let body = [
            form_tokens(&form.body),
            "title=Revision+Conf".to_string(),
            "status=1".to_string(),
            "field_start_date=2027-05-01".to_string(),
            "field_end_date=2027-05-03".to_string(),
            "field_city=Milano".to_string(),
            "field_country=Italy".to_string(),
            "log=Moved+to+Milano".to_string(),
        ]
        .join("&");
        let saved = app.post_form(&path, &body, &cookies).await;
        assert_eq!(saved.status, 200, "the save was refused: {}", saved.body);
        let second: uuid::Uuid = sqlx::query_scalar(
            "SELECT id FROM item_revision WHERE item_id = $1 ORDER BY created DESC, id DESC LIMIT 1",
        )
        .bind(conference.id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_ne!(first, second, "the save wrote no revision");

        // The history: both revisions and the log message.
        let history_path = format!("/item/{}/revisions", conference.id);
        let history = app.get(&history_path, &cookies).await;
        assert_eq!(history.status, 200, "{}", history.body);
        assert!(history.body.contains("Moved to Milano"), "{}", history.body);
        assert!(
            history.body.contains(&first.to_string()),
            "{}",
            history.body
        );

        // Preview of the first revision shows its value.
        let preview = app
            .get(
                &format!("/item/{}/revisions/{first}", conference.id),
                &cookies,
            )
            .await;
        assert_eq!(preview.status, 200, "{}", preview.body);
        assert!(preview.body.contains("Torino"), "{}", preview.body);

        // Compare shows both sides of the change.
        let compare = app
            .get(
                &format!(
                    "/item/{}/revisions/compare?from={first}&to={second}",
                    conference.id
                ),
                &cookies,
            )
            .await;
        assert_eq!(compare.status, 200, "{}", compare.body);
        assert!(
            compare.body.contains("Torino") && compare.body.contains("Milano"),
            "{}",
            compare.body
        );

        // Revert to the first revision, with the history page's own token.
        let reverted = app
            .post_form(
                &format!("/item/{}/revert/{first}", conference.id),
                &format!("_token={}", csrf(&history.body)),
                &cookies,
            )
            .await;
        assert!(
            (300..400).contains(&reverted.status),
            "the revert was refused: {} {}",
            reverted.status,
            reverted.body
        );
        let city: Option<String> =
            sqlx::query_scalar("SELECT fields->>'field_city' FROM item WHERE id = $1")
                .bind(conference.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(city.as_deref(), Some("Torino"), "the revert did not apply");
    });
}

/// A conference carrying every field the brief's model gives one.
async fn conference_fields(pool: &sqlx::PgPool) -> serde_json::Value {
    serde_json::json!({
        "field_url": "https://example.test",
        "field_start_date": "2027-05-01",
        "field_end_date": "2027-05-03",
        "field_city": "Torino",
        "field_country": "Italy",
        "field_online": false,
        "field_cfp_url": "https://example.test/cfp",
        "field_cfp_end_date": "2027-03-01",
        "field_language": "en",
        "field_source_id": "round-trip",
        "field_editor_notes": "Ask about the CFP deadline.",
        "field_topics": [host::topic_term(pool, "Rust").await],
        "field_speakers": ["0193a5a0-0005-7000-8000-000000000001"],
        "field_venue_photos": ["public://venue-a.jpg", "public://venue-b.jpg"],
    })
}

/// One hidden input's value out of a kernel-rendered form.
fn hidden(html: &str, name: &str) -> String {
    let marker = format!(r#"name="{name}" value=""#);
    let start = html
        .find(&marker)
        .unwrap_or_else(|| panic!("no {name} input in: {html}"))
        + marker.len();
    let rest = &html[start..];
    let end = rest.find('"').unwrap();
    rest[..end].to_string()
}

/// The CSRF token out of a kernel-rendered form, which names it `_token` since
/// the kernel's one form renderer (it was `_csrf`).
fn csrf(html: &str) -> String {
    hidden(html, "_token")
}

/// What a urlencoded save of a kernel item form must carry: the CSRF token and
/// the build id of the form it was rendered for, without which the kernel
/// answers "This form has expired" and saves nothing.
fn form_tokens(html: &str) -> String {
    format!(
        "_token={}&_form_build_id={}",
        hidden(html, "_token"),
        hidden(html, "_form_build_id")
    )
}

/// The demo seed runs once the import has finished, however small the feed.
///
/// It used to wait for 80 upcoming conferences, and the live feed fell to 54 in
/// November 2026: the seed never ran, and the weekly demo found nothing on
/// Curated and almost nothing Live. This drives the real `tap_cron` over eight
/// upcoming conferences and one past one, first with the import still arriving
/// (it must wait), then with the importer's completion recorded and its queue
/// empty (it must seed, splitting three to one and leaving the past one alone).
#[test]
fn the_demo_seed_waits_for_the_import_and_then_seeds_a_small_feed() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::install_and_enable(&pool, PLUGIN).await.unwrap();
        host::import_demo_config(&pool).await;
        // The importer's state table, which the seed reads; created by the
        // importer's own migration rather than by hand.
        host::run_migrations(&pool, "ritrovo_importer").await;
        sqlx::query("DELETE FROM item WHERE type = 'conference'")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM ritrovo_state WHERE name = 'historical_import_queued'")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM plugin_queue WHERE plugin_name = 'ritrovo_importer'")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM site_config WHERE key = 'plugin.ritrovo_access.ritrovo_access.demo_seed_done'")
            .execute(&pool)
            .await
            .unwrap();

        let disp = host::dispatcher(PLUGIN);
        let items = host::items(&pool, &disp);
        for n in 0..8 {
            let dates = serde_json::json!({
                "field_start_date": format!("2099-01-{:02}", n + 1),
                "field_end_date": format!("2099-01-{:02}", n + 1),
            });
            host::create_conference(
                &pool,
                &items,
                &format!("Upcoming {n}"),
                Some(INCOMING_STAGE),
                dates,
            )
            .await;
        }
        let past = serde_json::json!({
            "field_start_date": "2001-01-01",
            "field_end_date": "2001-01-02",
        });
        host::create_conference(&pool, &items, "Long Gone", Some(INCOMING_STAGE), past).await;

        let cron = |pool: sqlx::PgPool,
                    disp: std::sync::Arc<trovato_kernel::tap::TapDispatcher>| async move {
            host::json(
                &host::dispatch(
                    &pool,
                    &disp,
                    PLUGIN,
                    "tap_cron",
                    &serde_json::json!({"timestamp": host::now()}).to_string(),
                    UserContext::background(),
                    host::offline_client(),
                )
                .await,
            )
        };

        let waiting = cron(pool.clone(), disp.clone()).await;
        assert_eq!(
            waiting["status"], "waiting",
            "seeded before the import finished: {waiting}"
        );

        sqlx::query(
            "INSERT INTO ritrovo_state (name, value) VALUES ('historical_import_queued', '1') \
             ON CONFLICT (name) DO UPDATE SET value = '1'",
        )
        .execute(&pool)
        .await
        .unwrap();

        let seeded = cron(pool.clone(), disp.clone()).await;
        assert_eq!(seeded["status"], "seeded", "{seeded}");
        assert_eq!(seeded["published"], 6, "{seeded}");
        assert_eq!(seeded["curated"], 2, "{seeded}");

        let on_stage = |stage: &'static str| {
            let pool = pool.clone();
            async move {
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM item WHERE type = 'conference' AND stage_id = $1::uuid",
                )
                .bind(stage)
                .fetch_one(&pool)
                .await
                .unwrap()
            }
        };
        assert_eq!(on_stage(CURATED_STAGE).await, 2);
        assert_eq!(
            on_stage(INCOMING_STAGE).await,
            1,
            "the past conference stays on Incoming"
        );

        let again = cron(pool.clone(), disp.clone()).await;
        assert_eq!(again["status"], "skipped", "the seed ran twice: {again}");
    });
}

/// The titles `/api/search` answered, in order.
fn search_titles(body: &str) -> Vec<String> {
    let value = host::json(body);
    value["data"]
        .as_array()
        .unwrap_or_else(|| panic!("no data array in {body}"))
        .iter()
        .filter_map(|r| r["title"].as_str().map(str::to_string))
        .collect()
}

/// Search is stage-aware, and nobody reaches an editorial stage through it but
/// an administrator who has switched to that stage.
///
/// One conference on each of the three stages, all three titled with the same
/// unusual word, then the kernel's own search route asked for it:
///
/// - **anonymous**: the Live one only;
/// - **an editor** (the demo's real `editor` role, which may read Incoming and
///   Curated through `ritrovo_access`): the Live one only as well. Server search
///   reads the stage from the session (`crates/kernel/src/routes/search.rs:26-46`)
///   and only an administrator may set it (`routes/admin.rs:55`); the editor's
///   attempt is refused, and search never offers the stages the editor may read
///   (FRICTION.md, G-STAGE-SWITCH-IS-ADMIN-ONLY);
/// - **an administrator switched to Incoming**: the Incoming one and the Live
///   one, and still never the Curated one, because the stage set is the active
///   stage plus Live, not every stage.
///
/// The browser's search is the Pagefind index, which the kernel builds from Live
/// items only (`crates/kernel/src/cron/pagefind.rs:135-147`), so it has no
/// editor view at all; `scripts/verify-demo.sh` holds the demo's index to the
/// Live count and queries it for an Incoming and a Curated title.
#[test]
fn search_returns_live_to_everyone_and_another_stage_only_to_an_administrator_on_it() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::prepare_app_database(&pool, &[PLUGIN]).await;
        host::import_demo_config(&pool).await;
        let app = host::app().await;
        host::clear_rate_limits().await;

        sqlx::query("DELETE FROM item WHERE type = 'conference'")
            .execute(&pool)
            .await
            .unwrap();
        let items = host::items(&pool, &host::dispatcher(PLUGIN));
        let mut fields = conference_fields(&pool).await;
        let mut live_id = None;
        for (title, stage, source) in [
            ("Quokkafest Live", None, "stage-search-live"),
            (
                "Quokkafest Incoming",
                Some(INCOMING_STAGE),
                "stage-search-incoming",
            ),
            (
                "Quokkafest Curated",
                Some(CURATED_STAGE),
                "stage-search-curated",
            ),
        ] {
            fields["field_source_id"] = serde_json::json!(source);
            let item = host::create_conference(&pool, &items, title, stage, fields.clone()).await;
            if stage.is_none() {
                live_id = Some(item.id);
            }
        }
        // A session's CSRF token, from the kernel's own edit form for the Live
        // conference, which both an editor and an administrator may open.
        let edit_path = format!("/item/{}/edit", live_id.unwrap());
        // The fixture really is on three stages, and the tsvector trigger has
        // indexed all three: what search leaves out, it leaves out by stage.
        let indexed: i64 = sqlx::query_scalar(
            "SELECT count(DISTINCT stage_id) FROM item \
              WHERE title LIKE 'Quokkafest %' \
                AND search_vector @@ to_tsquery('english', 'quokkafest')",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            indexed, 3,
            "the three fixtures are not indexed on three stages"
        );

        let anonymous = app.get("/api/v1/search?q=quokkafest", "").await;
        assert_eq!(anonymous.status, 200, "{}", anonymous.body);
        assert_eq!(search_titles(&anonymous.body), vec!["Quokkafest Live"]);

        let (editor, _) = app.login_as_new_member(&["editor"]).await;
        let as_editor = app.get("/api/v1/search?q=quokkafest", &editor).await;
        assert_eq!(search_titles(&as_editor.body), vec!["Quokkafest Live"]);

        // The editor asks for Incoming and is refused.
        let page = app.get(&edit_path, &editor).await;
        let switch = app
            .send(
                axum::http::Request::post("/admin/stage/switch")
                    .header("content-type", "application/json")
                    .header("x-csrf-token", csrf(&page.body))
                    .header(axum::http::header::COOKIE, &editor)
                    .body(axum::body::Body::from(format!(
                        r#"{{"stage_id":"{INCOMING_STAGE}"}}"#
                    )))
                    .unwrap(),
            )
            .await;
        // Refused for being an editor, not for the token: the same kind of
        // token, taken the same way, is accepted for the administrator below.
        assert_eq!(
            switch.status, 403,
            "an editor's stage switch was not refused: {}",
            switch.body
        );
        assert!(
            !switch.body.contains("CSRF"),
            "the editor's switch failed on its token, which proves nothing: {}",
            switch.body
        );
        let still = app.get("/api/v1/search?q=quokkafest", &editor).await;
        assert_eq!(search_titles(&still.body), vec!["Quokkafest Live"]);

        // An administrator on Incoming sees Incoming and Live, never Curated.
        let admin = app.login_as_new_admin().await;
        let page = app.get(&edit_path, &admin).await;
        let switch = app
            .send(
                axum::http::Request::post("/admin/stage/switch")
                    .header("content-type", "application/json")
                    .header("x-csrf-token", csrf(&page.body))
                    .header(axum::http::header::COOKIE, &admin)
                    .body(axum::body::Body::from(format!(
                        r#"{{"stage_id":"{INCOMING_STAGE}"}}"#
                    )))
                    .unwrap(),
            )
            .await;
        assert_eq!(switch.status, 200, "{}", switch.body);
        let mut on_incoming =
            search_titles(&app.get("/api/v1/search?q=quokkafest", &admin).await.body);
        on_incoming.sort();
        assert_eq!(on_incoming, vec!["Quokkafest Incoming", "Quokkafest Live"]);

        // And the anonymous answer did not change while an administrator was
        // looking at Incoming: the stage is per session, never shared.
        let anonymous = app.get("/api/v1/search?q=quokkafest", "").await;
        assert_eq!(search_titles(&anonymous.body), vec!["Quokkafest Live"]);
    });
}

/// What a page shows after each kind of write, read through the kernel's own
/// item route, which reads through `ItemService`'s in-process cache.
///
/// - **A kernel edit** (the edit form, as an editor) is on the page at once:
///   `ItemService::update` invalidates its own entry.
/// - **A raw SQL write**, which is how `ritrovo_importer` updates a conference it
///   has already landed (and how the submission form and config import write),
///   is NOT: the page keeps the cached title until the entry expires
///   (`CACHE_TTL_ITEMS`, 300 seconds by default and 10 on the demo), because no
///   host call reaches `ItemService::invalidate`
///   (FRICTION.md, G-PLUGIN-CANNOT-INVALIDATE-THE-ITEM-CACHE). The test then
///   invalidates the entry the way only the kernel can, and the page catches up:
///   the staleness is the cache's and nothing else's.
/// - **An editor's view of a Curated conference never reaches an anonymous
///   visitor.** The cache holds Items, not pages, and the access check runs on
///   every request after the cache is read, so an editor loading the Curated
///   item (and so caching it) changes nothing for the next anonymous request.
///   There is no page cache for it to leak through: the kernel caches no
///   rendered HTML (docs/CACHING.md).
#[test]
fn a_kernel_edit_shows_at_once_a_raw_write_waits_for_the_cache_and_no_view_leaks() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::prepare_app_database(&pool, &[PLUGIN]).await;
        host::import_demo_config(&pool).await;
        let app = host::app().await;
        host::clear_rate_limits().await;

        sqlx::query("DELETE FROM item WHERE type = 'conference'")
            .execute(&pool)
            .await
            .unwrap();
        let items = host::items(&pool, &host::dispatcher(PLUGIN));
        let mut fields = conference_fields(&pool).await;
        fields["field_source_id"] = serde_json::json!("cache-live");
        let live =
            host::create_conference(&pool, &items, "Wombatcon Original", None, fields.clone())
                .await;
        fields["field_source_id"] = serde_json::json!("cache-curated");
        let curated = host::create_conference(
            &pool,
            &items,
            "Wombatcon Curated Secret",
            Some(CURATED_STAGE),
            fields.clone(),
        )
        .await;
        let page = format!("/item/{}", live.id);

        // Warm the cache.
        let first = app.get(&page, "").await;
        assert_eq!(first.status, 200, "{}", first.body);
        assert!(first.body.contains("Wombatcon Original"));

        // A kernel edit: through the edit form, as an editor.
        let (editor, _) = app.login_as_new_member(&["editor"]).await;
        let edit = format!("/item/{}/edit", live.id);
        let form = app.get(&edit, &editor).await;
        assert_eq!(form.status, 200, "{}", form.body);
        let body = [
            form_tokens(&form.body),
            "title=Wombatcon+Edited".to_string(),
            "status=1".to_string(),
            "field_start_date=2027-05-01".to_string(),
            // Required on the type, and the kernel's form pipeline validates it
            // now (it used to save whatever arrived).
            "field_end_date=2027-05-03".to_string(),
            "field_source_id=cache-live".to_string(),
        ]
        .join("&");
        let saved = app.post_form(&edit, &body, &editor).await;
        assert_eq!(saved.status, 200, "the edit was refused: {}", saved.body);
        // The edit also bumps the item cache's generation, and the watcher
        // clears the whole kind up to 250 ms later. Let that land before the
        // read below warms the entry the raw write is measured against.
        host::settle_cache_generations().await;
        let after_edit = app.get(&page, "").await;
        assert!(
            after_edit.body.contains("Wombatcon Edited")
                && !after_edit.body.contains("Wombatcon Original"),
            "a kernel edit did not reach the page at once"
        );

        // A raw write, the importer's path.
        sqlx::query("UPDATE item SET title = 'Wombatcon Imported' WHERE id = $1")
            .bind(live.id)
            .execute(&pool)
            .await
            .unwrap();
        let after_raw = app.get(&page, "").await;
        assert!(
            after_raw.body.contains("Wombatcon Edited")
                && !after_raw.body.contains("Wombatcon Imported"),
            "a raw write reached the page before the cache expired; \
             G-PLUGIN-CANNOT-INVALIDATE-THE-ITEM-CACHE may be fixed, so update \
             docs/CACHING.md rather than relaxing this"
        );
        app.state.items().invalidate(live.id);
        let after_invalidate = app.get(&page, "").await;
        assert!(
            after_invalidate.body.contains("Wombatcon Imported"),
            "the page did not catch up once the entry was invalidated"
        );

        // The editor reads the Curated conference, which caches it...
        let curated_page = format!("/item/{}", curated.id);
        let as_editor = app.get(&curated_page, &editor).await;
        assert_eq!(as_editor.status, 200, "{}", as_editor.body);
        assert!(as_editor.body.contains("Wombatcon Curated Secret"));
        // ...and an anonymous visitor straight after is refused it.
        let as_anonymous = app.get(&curated_page, "").await;
        assert_ne!(as_anonymous.status, 200, "{}", as_anonymous.body);
        assert!(
            !as_anonymous.body.contains("Wombatcon Curated Secret"),
            "an editor's view of a Curated conference reached an anonymous visitor"
        );
    });
}
