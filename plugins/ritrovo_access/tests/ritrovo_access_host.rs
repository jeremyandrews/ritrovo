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

#[path = "../../../tests/host/ai.rs"]
mod ai;

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

        // Everyone who may read content may search it. The kernel gates
        // `/search` and `/api/v1/search` on `search content` (B160), and its
        // migration's grant is revoked by this import, whose role files are
        // authoritative; a role file that drops the line takes search away
        // from that role with a 401 or 403.
        for (who, viewer) in [
            ("anonymous", &anonymous),
            ("viewer_carol", &reader),
            ("editor_alice", &editor),
            ("publisher_bob", &publisher),
        ] {
            assert!(
                viewer.permissions.iter().any(|p| p == "access content"),
                "{who} cannot read content. Holds: {:?}",
                viewer.permissions
            );
            assert!(
                viewer.permissions.iter().any(|p| p == "search content"),
                "{who} may read content but not search it; a demo/config \
                 role file has dropped `search content`. Holds: {:?}",
                viewer.permissions
            );
        }
        // Per role, not per user: viewer_carol would still search through the
        // authenticated role if the viewer file alone dropped the line. The
        // five role files in demo/config, by their ids. Each grants
        // `access content`, so each has to grant `search content` as well.
        const DEMO_ROLE_IDS: [&str; 5] = [
            "00000000-0000-0000-0000-000000000001",
            "00000000-0000-0000-0000-000000000002",
            "0193a5a0-0002-7000-8000-000000000001",
            "0193a5a0-0002-7000-8000-000000000002",
            "0193a5a0-0002-7000-8000-000000000003",
        ];
        let can_search: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT p.role_id::text FROM role_permissions p \
             WHERE p.role_id::text = ANY($1) AND p.permission = 'search content'",
        )
        .bind(DEMO_ROLE_IDS.map(str::to_string).to_vec())
        .fetch_all(&pool)
        .await
        .unwrap();
        let cannot_search: Vec<&str> = DEMO_ROLE_IDS
            .into_iter()
            .filter(|id| !can_search.iter().any(|c| c == id))
            .collect();
        assert!(
            cannot_search.is_empty(),
            "these demo/config roles may read content but not search it: \
             {cannot_search:?}"
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

// ===========================================================================
// The AI surfaces a visitor and a member can reach (A9b, 31.1, 31.3, 31.4, 31.9)
// ===========================================================================
//
// The kernel's own chat, search and admin routes, served by the kernel's
// router over `demo/config`. Ritrovo's providers are pointed at the kernel's
// scripted provider (`tests/host/ai.rs`), so no model is called and every
// request that would have reached one is counted.

use trovato_kernel::services::ai_tools::scripted_provider::{self, Recorder, Scripted};

/// The app, `demo/config` imported, and Ritrovo's providers pointed at a fresh
/// scripted provider.
async fn ai_app() -> (&'static host::App, sqlx::PgPool, Recorder) {
    let pool = host::fresh_pool().await;
    host::prepare_app_database(&pool, &[PLUGIN]).await;
    host::import_demo_config(&pool).await;
    let app = host::app().await;
    host::clear_rate_limits().await;
    // The search settings are read through the kernel's cache for five
    // minutes, and an import does not drop the cached copy
    // (`crates/kernel/src/search/settings.rs`). Drop it, as saving the settings
    // screen does, so the stages are the ones `demo/config` just imported and
    // not whatever an earlier run left in Redis.
    app.state
        .cache()
        .invalidate(trovato_kernel::search::settings::SETTINGS_CACHE_KEY)
        .await;
    let (url, recorder) = scripted_provider::start().await;
    ai::point_providers_at(&pool, &url).await;
    (app, pool, recorder)
}

/// The session's CSRF token from a kernel page: the `csrf-token` meta tag the
/// base template renders, or a form's `_token`.
fn session_token(html: &str) -> String {
    for marker in [r#"name="csrf-token" content=""#, r#"name="_token" value=""#] {
        if let Some(start) = html.find(marker) {
            let rest = &html[start + marker.len()..];
            return rest[..rest.find('"').unwrap()].to_string();
        }
    }
    panic!("no CSRF token in: {html}")
}

/// POST a JSON body with the session's CSRF header, as the kernel's own
/// scripts do.
async fn post_json(
    app: &host::App,
    path: &str,
    body: serde_json::Value,
    cookies: &str,
    token: &str,
) -> host::Response {
    let mut request = axum::http::Request::post(path)
        .header("content-type", "application/json")
        .header("x-csrf-token", token);
    if !cookies.is_empty() {
        request = request.header(axum::http::header::COOKIE, cookies);
    }
    app.send(
        request
            .body(axum::body::Body::from(body.to_string()))
            .unwrap(),
    )
    .await
}

/// An anonymous session and its token, from the login form.
async fn anonymous_session(app: &host::App) -> (String, String) {
    let login = app.get("/user/login", "").await;
    assert_eq!(login.status, 200, "{}", login.body);
    (login.cookies.clone(), session_token(&login.body))
}

/// A member's session and token.
async fn member_session(app: &host::App) -> (String, String) {
    let (cookies, _) = app.login_as_new_member(&[]).await;
    let page = app.get("/user/profile", &cookies).await;
    assert_eq!(page.status, 200, "{}", page.body);
    (cookies.clone(), session_token(&page.body))
}

const SEARCH_AI: [&str; 3] = [
    "/api/v1/search/expand",
    "/api/v1/search/summarize",
    "/api/v1/search/followup",
];

fn search_ai_body() -> serde_json::Value {
    serde_json::json!({
        "query": "rust conferences in europe",
        "context": "RustConf Europe, Berlin",
        "messages": [{"role": "user", "content": "rust conferences in europe"}],
    })
}

/// Who holds which AI permission, as `demo/config` grants them: every signed-in
/// member `use ai` and `use ai chat`, the editorial roles `view ai usage` as
/// well, a visitor none of them. `configure ai` is the administrator's alone,
/// which the administrator flag carries.
#[test]
fn the_roles_grant_ai_to_members_usage_to_editors_and_nothing_to_visitors() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::import_demo_config(&pool).await;
        host::create_demo_users(&pool).await;
        for (name, usage) in [
            ("viewer_carol", false),
            ("editor_alice", true),
            ("publisher_bob", true),
        ] {
            let user = host::visitor_named(&pool, name).await;
            assert!(user.has_permission("use ai"), "{name}");
            assert!(user.has_permission("use ai chat"), "{name}");
            assert_eq!(user.has_permission("view ai usage"), usage, "{name}");
            assert!(!user.has_permission("configure ai"), "{name}");
        }
        let anonymous: Vec<String> = sqlx::query_scalar(
            "SELECT rp.permission FROM role_permissions rp JOIN roles r ON r.id = rp.role_id \
             WHERE r.name = 'anonymous user'",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert!(
            !anonymous.iter().any(|p| p.contains("ai")),
            "the anonymous role holds an AI permission: {anonymous:?}"
        );
    });
}

/// A visitor without `use ai` gets the kernel's clean refusal from chat (401,
/// JSON, sign in) and from the three AI search endpoints (404: Ritrovo keeps
/// those stages off, because the kernel checks no permission and no budget on
/// them). No provider is asked. A member is refused by search AI the same way.
#[test]
fn chat_and_search_ai_refuse_a_visitor_without_use_ai_and_ask_no_provider() {
    serial(async {
        let (app, _pool, recorder) = ai_app().await;
        recorder.push(Scripted::ok(ai::anthropic_reply("never", 1, 1)));

        let (anon, anon_token) = anonymous_session(app).await;
        let chat = post_json(
            app,
            "/api/v1/chat",
            serde_json::json!({"message": "What Rust conferences are in Europe?"}),
            &anon,
            &anon_token,
        )
        .await;
        assert_eq!(chat.status, 401, "{}", chat.body);
        assert!(chat.content_type.contains("json"), "{}", chat.content_type);

        let (member, member_token) = member_session(app).await;
        for (cookies, token) in [(&anon, &anon_token), (&member, &member_token)] {
            for path in SEARCH_AI {
                let refused = post_json(app, path, search_ai_body(), cookies, token).await;
                assert_eq!(refused.status, 404, "{path}: {}", refused.body);
                assert!(
                    refused.body.contains("switched off"),
                    "{path}: {}",
                    refused.body
                );
            }
        }
        assert_eq!(recorder.request_count(), 0, "a provider was asked");
    });
}

/// With no key set, which is CI and any install that has not configured one,
/// every AI surface Ritrovo exposes answers without a 500 and without a word of
/// the provider's error, and the pages around them still render.
#[test]
fn with_no_key_every_ai_surface_answers_without_a_500_or_provider_text() {
    serial(async {
        ai::unset_keys();
        let (app, _pool, recorder) = ai_app().await;
        for _ in 0..4 {
            recorder.push(Scripted {
                status: 401,
                body: ai::anthropic_no_key(),
                delay: std::time::Duration::ZERO,
            });
        }
        let provider_text = ["authentication_error", "x-api-key"];

        let (member, token) = member_session(app).await;
        let chat = post_json(
            app,
            "/api/v1/chat",
            serde_json::json!({"message": "What Rust conferences are in Europe?"}),
            &member,
            &token,
        )
        .await;
        assert_ne!(chat.status, 500, "{}", chat.body);
        assert_ne!(
            chat.status, 200,
            "chat claimed success with no key: {}",
            chat.body
        );
        for path in SEARCH_AI {
            let answer = post_json(app, path, search_ai_body(), &member, &token).await;
            assert_ne!(answer.status, 500, "{path}: {}", answer.body);
            for text in provider_text {
                assert!(!answer.body.contains(text), "{path}: {}", answer.body);
            }
        }
        for text in provider_text {
            assert!(!chat.body.contains(text), "chat: {}", chat.body);
        }

        let search = app.get("/search?q=rust", &member).await;
        assert_eq!(search.status, 200, "{}", search.body);
        assert!(search.body.contains("scolta-search"), "{}", search.body);

        let admin = app.login_as_new_admin().await;
        let providers = app.get("/admin/system/ai-providers", &admin).await;
        assert_eq!(providers.status, 200, "{}", providers.body);
        for env in [ai::ANTHROPIC_KEY_ENV, ai::OPENAI_KEY_ENV] {
            assert!(
                providers.body.contains(&format!("{env} (not set)")),
                "{env} is not shown as not set: {}",
                providers.body
            );
        }
        let budgets = app.get("/admin/system/ai-budgets", &admin).await;
        assert_eq!(budgets.status, 200, "{}", budgets.body);
    });
}

/// With a known fake key set, the value is in no page, no API answer, no
/// exported configuration, no stored row and no log line. The provider screen
/// shows the variable's name and "set".
///
/// The provider here answers as Anthropic does, without echoing the key. One
/// kernel path logs a provider's error body unredacted; what happens when a
/// provider does echo it is pinned by
/// `a_provider_error_that_echoes_the_key_reaches_the_log_through_the_chat_path`.
#[test]
fn the_key_value_appears_in_no_page_answer_export_stored_row_or_log_line() {
    serial(async {
        let log_before = ai::captured_log().len();
        let (app, pool, recorder) = ai_app().await;
        for _ in 0..4 {
            recorder.push(Scripted {
                status: 401,
                body: ai::anthropic_no_key(),
                delay: std::time::Duration::ZERO,
            });
        }
        ai::set_fake_keys();

        let mut seen = Vec::new();
        let (member, token) = member_session(app).await;
        let chat = post_json(
            app,
            "/api/v1/chat",
            serde_json::json!({"message": "What Rust conferences are in Europe?"}),
            &member,
            &token,
        )
        .await;
        seen.push(format!("chat: {}", chat.body));

        let admin = app.login_as_new_admin().await;
        for path in ["/admin/system/ai-providers", "/admin/system/ai-budgets"] {
            let page = app.get(path, &admin).await;
            assert_eq!(page.status, 200, "{path}: {}", page.body);
            seen.push(format!("{path}: {}", page.body));
        }
        let providers = &seen[1];
        for env in [ai::ANTHROPIC_KEY_ENV, ai::OPENAI_KEY_ENV] {
            assert!(
                providers.contains(&format!("{env} (set)")),
                "{env} is not shown as set: {providers}"
            );
        }
        let test = app
            .post_form(
                &format!("/admin/system/ai-providers/{}/test", ai::ANTHROPIC_PROVIDER),
                &format!("_token={}", session_token(providers)),
                &admin,
            )
            .await;
        seen.push(format!("connection test: {}", test.body));

        let export = std::env::temp_dir().join(format!("ritrovo-ai-export-{}", host::now()));
        let storage = trovato_kernel::config_storage::DirectConfigStorage::new(pool.clone());
        trovato_kernel::config_storage::yaml::export_config(&storage, &pool, &export, true)
            .await
            .unwrap();
        let mut exported = String::new();
        for entry in std::fs::read_dir(&export).unwrap() {
            let path = entry.unwrap().path();
            if path.is_file() {
                exported.push_str(&std::fs::read_to_string(&path).unwrap_or_default());
            }
        }
        let _ = std::fs::remove_dir_all(&export);
        assert!(
            exported.contains(ai::ANTHROPIC_KEY_ENV),
            "the export does not carry the provider's key variable name"
        );
        seen.push(format!("export: {exported}"));
        ai::unset_keys();

        seen.push(format!("stored: {}", ai::stored_ai_text(&pool).await));
        seen.push(format!("log: {}", &ai::captured_log()[log_before..]));
        for text in &seen {
            for key in [ai::FAKE_ANTHROPIC_KEY, ai::FAKE_OPENAI_KEY] {
                assert!(
                    !text.contains(key),
                    "the key value is in {}",
                    text.split(':').next().unwrap_or("?")
                );
            }
        }
    });
}

/// The authenticated role, as `demo/config` identifies it. Every signed-in
/// member holds `use ai` and `use ai chat` through it.
const AUTHENTICATED_ROLE: &str = "00000000-0000-0000-0000-000000000002";

/// Take `permission` from the authenticated role, or give it back: the only
/// way to have a signed-in member without it. A member's permissions are
/// cached by user, so this must happen before that member signs in.
async fn set_authenticated_permission(pool: &sqlx::PgPool, permission: &str, granted: bool) {
    let role = uuid::Uuid::parse_str(AUTHENTICATED_ROLE).unwrap();
    let sql = if granted {
        "INSERT INTO role_permissions (role_id, permission) VALUES ($1, $2) ON CONFLICT DO NOTHING"
    } else {
        "DELETE FROM role_permissions WHERE role_id = $1 AND permission = $2"
    };
    sqlx::query(sql)
        .bind(role)
        .bind(permission)
        .execute(pool)
        .await
        .unwrap();
}

/// A signed-in member who lacks `use ai chat`, or `use ai`, is refused by chat
/// with a 403 before any provider is asked. The sign-in check alone is not
/// what keeps chat to the members Ritrovo grants it.
#[test]
fn chat_refuses_a_member_without_use_ai_or_use_ai_chat_and_asks_no_provider() {
    serial(async {
        let (app, pool, recorder) = ai_app().await;
        recorder.push(Scripted::ok(ai::anthropic_reply("never", 1, 1)));
        for missing in ["use ai chat", "use ai"] {
            set_authenticated_permission(&pool, missing, false).await;
            let (member, token) = member_session(app).await;
            let chat = post_json(
                app,
                "/api/v1/chat",
                serde_json::json!({"message": "What Rust conferences are in Europe?"}),
                &member,
                &token,
            )
            .await;
            set_authenticated_permission(&pool, missing, true).await;
            assert_eq!(chat.status, 403, "without {missing}: {}", chat.body);
            assert_eq!(
                recorder.request_count(),
                0,
                "without {missing}, a provider was asked"
            );
        }
    });
}

/// With no key set, the kernel still sends a member's chat message to the
/// provider, without credentials, and the provider refuses it. The text leaves
/// the site even though nothing is spent. Pins `FRICTION.md`,
/// `G-AI-REQUESTS-SENT-WITHOUT-A-KEY`: when the kernel stops sending a request
/// it has no key for, this fails, and the entry and `docs/AI.md` change with it.
#[test]
fn with_no_key_the_kernel_still_sends_a_member_s_message_to_the_provider() {
    serial(async {
        ai::unset_keys();
        let (app, _pool, recorder) = ai_app().await;
        recorder.push(Scripted {
            status: 401,
            body: ai::anthropic_no_key(),
            delay: std::time::Duration::ZERO,
        });
        let probe = "no-key probe 4c1d9e";
        let (member, token) = member_session(app).await;
        let chat = post_json(
            app,
            "/api/v1/chat",
            serde_json::json!({"message": format!("{probe}: which Rust conferences are in Europe?")}),
            &member,
            &token,
        )
        .await;
        assert_eq!(chat.status, 503, "{}", chat.body);
        let sent = recorder.requests();
        assert_eq!(sent.len(), 1, "requests sent with no key: {sent:?}");
        assert!(
            sent[0].to_string().contains(probe),
            "the member's message did not reach the provider: {}",
            sent[0]
        );
    });
}

/// A provider error body that carries the key reaches the log through the chat
/// path, which logs the first 200 bytes of the body without redacting them.
/// The member still sees only the generic answer. Pins `FRICTION.md`,
/// `G-AI-CHAT-ERROR-LOGS-THE-PROVIDER-BODY-UNREDACTED`: when the kernel redacts
/// that path, this fails, and it turns into a test that the log is clean.
#[test]
fn a_provider_error_that_echoes_the_key_reaches_the_log_through_the_chat_path() {
    serial(async {
        let log_before = ai::captured_log().len();
        let (app, _pool, recorder) = ai_app().await;
        recorder.push(Scripted {
            status: 401,
            body: serde_json::json!({
                "type": "error",
                "error": {
                    "type": "authentication_error",
                    "message": format!("invalid x-api-key: {}", ai::FAKE_ANTHROPIC_KEY)
                }
            }),
            delay: std::time::Duration::ZERO,
        });
        ai::set_fake_keys();
        let (member, token) = member_session(app).await;
        let chat = post_json(
            app,
            "/api/v1/chat",
            serde_json::json!({"message": "What Rust conferences are in Europe?"}),
            &member,
            &token,
        )
        .await;
        ai::unset_keys();
        assert_eq!(chat.status, 503, "{}", chat.body);
        assert!(!chat.body.contains(ai::FAKE_ANTHROPIC_KEY), "{}", chat.body);
        assert_eq!(recorder.request_count(), 1, "the chat request was not sent");
        let log = ai::captured_log()[log_before..].to_string();
        assert!(
            log.contains(ai::FAKE_ANTHROPIC_KEY),
            "the echoed key is no longer in the log: the kernel now redacts the \
             chat error path, so G-AI-CHAT-ERROR-LOGS-THE-PROVIDER-BODY-UNREDACTED \
             is fixed and this test should assert the opposite"
        );
    });
}

/// Ritrovo's field rule (STATUS 31.5) is imported where the kernel's
/// `trovato_ai` plugin looks for it, and the conference type declares its
/// target field. The plugin never applies it: it reads the rule with
/// `WHERE name = $1`, and `site_config`'s column is `key`, so the query fails
/// and every save goes through with no rule. Pins `FRICTION.md`,
/// `G-AI-FIELD-RULES-NEVER-LOAD`: the query is read out of the kernel's own
/// `trovato_ai` source at the pinned rev, so when the kernel fixes it this test
/// fails, and 31.5 needs its live check and its row changed.
#[test]
fn trovato_ai_reads_its_field_rules_from_a_column_site_config_does_not_have() {
    serial(async {
        let pool = host::fresh_pool().await;
        host::import_demo_config(&pool).await;

        let stored = trovato_kernel::models::SiteConfig::get(&pool, "trovato_ai.field_rules")
            .await
            .unwrap()
            .expect("demo/config imported no trovato_ai.field_rules");
        let rule = &stored["rules"][0];
        assert_eq!(rule["item_type"], "conference", "{stored}");
        assert_eq!(rule["source_field"], "field_description", "{stored}");
        assert_eq!(rule["target_field"], "field_summary", "{stored}");
        assert_eq!(rule["behavior"], "fill_if_empty", "{stored}");
        let fields: serde_json::Value = sqlx::query_scalar(
            "SELECT settings->'fields' FROM item_type WHERE type = 'conference'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(
            fields
                .as_array()
                .is_some_and(|f| f.iter().any(|f| f["field_name"] == "field_summary")),
            "the conference type has no field_summary: {fields}"
        );

        let source = std::fs::read_to_string(
            host::kernel_source_dir().join("plugins/trovato_ai/src/lib.rs"),
        )
        .unwrap();
        let at = source
            .find("fn load_field_rules")
            .expect("trovato_ai has no load_field_rules");
        let call = &source[at..];
        let start = call
            .find("query_raw(")
            .expect("load_field_rules runs no query")
            + 10;
        let literal = &call[start..];
        let open = literal.find('"').unwrap() + 1;
        let sql = &literal[open..open + literal[open..].find('"').unwrap()];
        assert!(sql.contains("site_config"), "unexpected query: {sql}");

        let result = sqlx::query(sqlx::AssertSqlSafe(sql.to_string()))
            .bind("trovato_ai.field_rules")
            .fetch_all(&pool)
            .await;
        let error = match result {
            Ok(rows) => panic!(
                "trovato_ai's field-rule query now runs ({} row(s)): \
                 G-AI-FIELD-RULES-NEVER-LOAD is fixed in the kernel. Run 31.5's live \
                 check, update its row and FRICTION.md, and turn this test around: {sql}",
                rows.len()
            ),
            Err(e) => e.to_string(),
        };
        assert!(
            error.contains("column \"name\" does not exist"),
            "the query fails for another reason: {error}"
        );
    });
}

/// Why the search page's AI stages stay off even once the kernel checks a
/// permission on them: its search AI reads only the first content block of an
/// Anthropic answer, and Claude Haiku 5.5, Ritrovo's chat model, thinks by
/// default, so the first block is a thinking block with no text. Expansion
/// answers 200 with no terms and the summary 200 with an empty summary, though
/// the provider sent both. Pins `FRICTION.md`,
/// `G-SEARCH-AI-READS-ONLY-THE-FIRST-CONTENT-BLOCK`: when the kernel reads the
/// text blocks, this fails. The stages are switched on here, in the test
/// database, for this test alone; `demo/config` keeps them off.
#[test]
fn the_search_ai_stages_answer_empty_when_the_model_thinks_first() {
    serial(async {
        let (app, pool, recorder) = ai_app().await;
        let cache = app.state.cache();
        let mut settings = trovato_kernel::search::SearchSettings::load(&pool, cache).await;
        settings.expand = true;
        settings.summarize = true;
        settings.save(&pool, cache).await.unwrap();
        ai::set_fake_keys();

        recorder.push(Scripted::ok(ai::anthropic_reply(
            r#"["rust meetup", "rustconf", "systems programming conference"]"#,
            40,
            20,
        )));
        recorder.push(Scripted::ok(ai::anthropic_reply(
            "RustConf Europe in Berlin is the closest match.",
            80,
            20,
        )));
        let (member, token) = member_session(app).await;
        let expand = post_json(
            app,
            "/api/v1/search/expand",
            search_ai_body(),
            &member,
            &token,
        )
        .await;
        let summary = post_json(
            app,
            "/api/v1/search/summarize",
            search_ai_body(),
            &member,
            &token,
        )
        .await;
        ai::unset_keys();
        settings.expand = false;
        settings.summarize = false;
        settings.save(&pool, cache).await.unwrap();

        assert_eq!(
            recorder.request_count(),
            2,
            "the stages did not ask the provider"
        );
        assert_eq!(expand.status, 200, "{}", expand.body);
        let terms: serde_json::Value = serde_json::from_str(&expand.body).unwrap();
        assert_eq!(
            terms["terms"],
            serde_json::json!([]),
            "expansion now carries the model's terms: the kernel reads past the \
             thinking block, so G-SEARCH-AI-READS-ONLY-THE-FIRST-CONTENT-BLOCK is \
             fixed: {}",
            expand.body
        );
        assert_eq!(summary.status, 200, "{}", summary.body);
        let summary: serde_json::Value = serde_json::from_str(&summary.body).unwrap();
        assert_eq!(summary["summary"], "", "{summary}");
    });
}

/// The similar-conferences gather (STATUS 31.11) ranks Live conferences by the
/// kernel's `semantic_similarity` operator on the `q` parameter, with no sort of
/// its own so the kernel's relevance order stands, and the Postgres it runs on
/// has pgvector, so the kernel made its embedding table. With no embedding it
/// can reach, which is CI and every keyless install, the page answers 200 with
/// no rows and no error. The ranking itself needs a live embedding provider:
/// the kernel refuses a loopback one on this path and its scripted provider
/// serves no embeddings (`FRICTION.md`, `G-EMBEDDINGS-HAVE-NO-TEST-PROVIDER`).
#[test]
fn similar_conferences_ranks_live_conferences_by_q_and_is_empty_without_embeddings() {
    serial(async {
        let (app, pool, _recorder) = ai_app().await;
        let definition: serde_json::Value = sqlx::query_scalar(
            "SELECT definition FROM gather_query WHERE query_id = 'ritrovo.similar_conferences'",
        )
        .fetch_optional(&pool)
        .await
        .unwrap()
        .expect("demo/config imported no ritrovo.similar_conferences");
        assert_eq!(definition["item_type"], "conference", "{definition}");
        assert_eq!(definition["stage_aware"], true, "{definition}");
        assert_eq!(definition["sorts"], serde_json::json!([]), "{definition}");
        let filters = definition["filters"].as_array().unwrap();
        assert_eq!(filters.len(), 1, "{definition}");
        assert_eq!(
            filters[0]["operator"], "semantic_similarity",
            "{definition}"
        );
        assert_eq!(
            filters[0]["value"],
            serde_json::json!({"url_arg": "q"}),
            "{definition}"
        );

        let table: Option<String> =
            sqlx::query_scalar("SELECT to_regclass('item_embeddings')::text")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            table.as_deref(),
            Some("item_embeddings"),
            "no item_embeddings table: the test Postgres has no pgvector"
        );

        for query in ["?q=rust+conferences+in+europe", ""] {
            let page = app
                .get(&format!("/gather/ritrovo.similar_conferences{query}"), "")
                .await;
            assert_eq!(page.status, 200, "{query}: {}", page.body);
            assert!(
                page.body.contains("No similar conferences found."),
                "{query}: {}",
                page.body
            );
            assert!(!page.body.contains("card--conf"), "{query}: {}", page.body);
        }
    });
}

// ===========================================================================
// The chat tile and what its answers may read (A9b, 31.7)
// ===========================================================================

/// One Live, one Incoming and one Curated conference, all titled for the same
/// question, written through the kernel's item service.
async fn wombat_conferences(pool: &sqlx::PgPool) -> [trovato_kernel::models::Item; 3] {
    sqlx::query("DELETE FROM item WHERE type = 'conference' AND title LIKE 'Wombat Rust%'")
        .execute(pool)
        .await
        .unwrap();
    let disp = host::dispatcher(PLUGIN);
    let items = host::items(pool, &disp);
    let fields = serde_json::json!({
        "field_start_date": "2099-03-02",
        "field_end_date": "2099-03-04",
        "field_city": "Wombatville",
        "field_country": "Wombatland",
        "field_url": "https://wombat-rust.example",
    });
    let mut made = Vec::new();
    for (title, stage) in [
        ("Wombat Rust Days", None),
        ("Wombat Rust Incoming", Some(host::INCOMING_STAGE)),
        ("Wombat Rust Curated", Some(host::CURATED_STAGE)),
    ] {
        made.push(host::create_conference(pool, &items, title, stage, fields.clone()).await);
    }
    host::settle_cache_generations().await;
    made.try_into().unwrap()
}

/// `demo/config` places the kernel's chat widget in the sidebar for signed-in
/// members, and nowhere for an anonymous visitor.
#[test]
fn the_chat_tile_is_in_the_sidebar_for_a_member_and_absent_for_a_visitor() {
    serial(async {
        let (app, pool, recorder) = ai_app().await;
        let [live, _, _] = wombat_conferences(&pool).await;
        let page = format!("/item/{}", live.id);

        let (member, _) = app.login_as_new_member(&[]).await;
        let as_member = app.get(&page, &member).await;
        assert_eq!(as_member.status, 200, "{}", as_member.body);
        assert!(
            as_member.body.contains(r#"id="chat-widget-ask_ritrovo""#),
            "no chat tile for a member: {}",
            as_member.body
        );
        assert!(as_member.body.contains("/static/js/chat-widget.js"));

        let as_visitor = app.get(&page, "").await;
        assert_eq!(as_visitor.status, 200, "{}", as_visitor.body);
        assert!(
            !as_visitor.body.contains("chat-widget"),
            "an anonymous visitor got the chat tile"
        );
        assert_eq!(
            recorder.request_count(),
            0,
            "rendering a page asked a model"
        );
    });
}

/// A member's question reaches the model with Ritrovo's system prompt and the
/// matching Live conference in its context, and never an Incoming or Curated
/// one, though all three match the question word for word. Read from the
/// request the kernel actually sent the provider.
///
/// The context names the conference and carries its own website, and no
/// address on this site: the kernel's context builder writes the title and the
/// item's text fields only (FRICTION.md, G-CHAT-CONTEXT-HAS-NO-ITEM-LINKS).
#[test]
fn the_chat_context_carries_live_conferences_and_never_an_incoming_or_curated_one() {
    serial(async {
        let (app, pool, recorder) = ai_app().await;
        let [live, incoming, curated] = wombat_conferences(&pool).await;
        recorder.push(Scripted::ok(ai::anthropic_reply(
            "Wombat Rust Days.",
            50,
            5,
        )));

        ai::set_fake_keys();
        let (member, token) = member_session(app).await;
        let chat = post_json(
            app,
            "/api/v1/chat",
            serde_json::json!({"message": "Wombat Rust"}),
            &member,
            &token,
        )
        .await;
        ai::unset_keys();
        assert_eq!(chat.status, 200, "{}", chat.body);

        let requests = recorder.requests();
        assert_eq!(requests.len(), 1, "{requests:?}");
        let request = &requests[0];
        assert_eq!(request["model"], ai::CHAT_MODEL);
        assert!(request.get("temperature").is_none(), "{request}");
        let system = request["system"]
            .as_str()
            .unwrap_or_else(|| panic!("no system prompt: {request}"));

        assert!(
            system.contains("a directory of technology conferences"),
            "not Ritrovo's prompt: {system}"
        );
        assert!(system.contains("---BEGIN CONTEXT---"), "{system}");
        assert!(system.contains("Wombat Rust Days"), "{system}");
        assert!(
            system.contains("Url: https://wombat-rust.example"),
            "{system}"
        );
        for hidden in [&incoming, &curated] {
            assert!(
                !system.contains(&hidden.title),
                "{} reached the model: {system}",
                hidden.title
            );
        }
        // No address on this site for the conference, alias or item path.
        assert!(!system.contains(&live.id.to_string()), "{system}");
        assert!(!system.contains("/conferences/"), "{system}");
    });
}

// ===========================================================================
// The kernel's MCP server over Ritrovo (A9b, 31.10)
// ===========================================================================
//
// `trovato-mcp` at the pinned rev, served in process on one end of a byte pipe
// exactly as its binary serves it on STDIO, with an MCP client's raw JSON-RPC
// on the other end. Each session is opened with a real API token, resolved the
// way the binary resolves `--token`.

mod mcp {
    use rmcp::ServiceExt;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    /// An MCP session for one raw API token, over an in-process pipe.
    pub struct Session {
        write: tokio::io::WriteHalf<tokio::io::DuplexStream>,
        read: BufReader<tokio::io::ReadHalf<tokio::io::DuplexStream>>,
        next_id: u64,
    }

    impl Session {
        /// Resolve the token as `trovato-mcp --token` does and serve a session
        /// for its user, or the resolver's refusal.
        pub async fn open(state: &trovato_kernel::AppState, raw: &str) -> Result<Self, String> {
            let user = trovato_mcp::auth::resolve_token(state, raw)
                .await
                .map_err(|e| e.to_string())?;
            let ctx = trovato_mcp::auth::build_user_context(state, &user)
                .await
                .map_err(|e| e.to_string())?;
            let server =
                trovato_mcp::server::TrovatoMcpServer::new(state.clone(), raw.to_string(), ctx);
            let (client_end, server_end) = tokio::io::duplex(1024 * 1024);
            let (server_read, server_write) = tokio::io::split(server_end);
            tokio::spawn(async move {
                if let Ok(running) = server.serve((server_read, server_write)).await {
                    let _ = running.waiting().await;
                }
            });
            let (read, write) = tokio::io::split(client_end);
            let mut session = Self {
                write,
                read: BufReader::new(read),
                next_id: 1,
            };
            let init = session
                .request(
                    "initialize",
                    serde_json::json!({
                        "protocolVersion": "2025-06-18",
                        "capabilities": {},
                        "clientInfo": {"name": "ritrovo-host-test", "version": "0"}
                    }),
                )
                .await;
            assert!(init.get("result").is_some(), "initialize: {init}");
            session
                .send(serde_json::json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))
                .await;
            Ok(session)
        }

        async fn send(&mut self, message: serde_json::Value) {
            let mut line = serde_json::to_vec(&message).unwrap();
            line.push(b'\n');
            self.write.write_all(&line).await.unwrap();
            self.write.flush().await.unwrap();
        }

        /// One JSON-RPC request and its response.
        pub async fn request(
            &mut self,
            method: &str,
            params: serde_json::Value,
        ) -> serde_json::Value {
            let id = self.next_id;
            self.next_id += 1;
            self.send(
                serde_json::json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}),
            )
            .await;
            loop {
                let mut line = String::new();
                let n = self.read.read_line(&mut line).await.unwrap();
                assert!(
                    n > 0,
                    "the server closed the pipe before answering {method}"
                );
                let message: serde_json::Value = serde_json::from_str(&line).unwrap();
                if message.get("id").and_then(serde_json::Value::as_u64) == Some(id) {
                    return message;
                }
            }
        }

        /// `tools/call`, returning the whole response.
        pub async fn call(
            &mut self,
            tool: &str,
            arguments: serde_json::Value,
        ) -> serde_json::Value {
            self.request(
                "tools/call",
                serde_json::json!({"name": tool, "arguments": arguments}),
            )
            .await
        }
    }
}

/// A raw API token for `user`, as `/api/v1/tokens` issues one.
async fn api_token(pool: &sqlx::PgPool, user: uuid::Uuid) -> String {
    let (_, raw) =
        trovato_kernel::models::api_token::ApiToken::create(pool, user, "mcp host test", None)
            .await
            .unwrap();
    raw
}

/// An editor's token lists conference content and reads a Curated conference;
/// a member's token lists the Live conference and cannot read the Curated one;
/// a token that is not one is refused before any session exists.
#[test]
fn trovato_mcp_serves_an_editor_s_token_hides_curated_from_a_member_and_refuses_a_bad_token() {
    serial(async {
        let (app, pool, _recorder) = ai_app().await;
        let [live, _incoming, curated] = wombat_conferences(&pool).await;
        let (_, editor) = app.login_as_new_member(&["editor"]).await;
        let (_, member) = app.login_as_new_member(&[]).await;

        let mut as_editor = mcp::Session::open(&app.state, &api_token(&pool, editor).await)
            .await
            .unwrap();
        let tools = as_editor.request("tools/list", serde_json::json!({})).await;
        let mut names: Vec<String> = tools["result"]["tools"]
            .as_array()
            .unwrap_or_else(|| panic!("{tools}"))
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect();
        names.sort();
        assert_eq!(
            names,
            [
                "create_item",
                "delete_item",
                "get_item",
                "list_categories",
                "list_content_types",
                "list_items",
                "list_tags",
                "run_gather",
                "search",
                "update_item",
            ]
        );

        let listed = as_editor
            .call(
                "list_items",
                serde_json::json!({"content_type": "conference", "per_page": 100}),
            )
            .await;
        assert_eq!(listed["result"]["isError"], false, "{listed}");
        let text = listed["result"]["content"].to_string();
        assert!(text.contains("Wombat Rust Days"), "{text}");

        let types = as_editor
            .call("list_content_types", serde_json::json!({}))
            .await;
        assert!(
            types["result"]["content"]
                .to_string()
                .contains("conference"),
            "{types}"
        );

        let editor_reads = as_editor
            .call(
                "get_item",
                serde_json::json!({"id": curated.id.to_string()}),
            )
            .await;
        assert!(
            editor_reads["result"]["content"]
                .to_string()
                .contains("Wombat Rust Curated"),
            "an editor could not read a Curated conference: {editor_reads}"
        );

        let mut as_member = mcp::Session::open(&app.state, &api_token(&pool, member).await)
            .await
            .unwrap();
        let member_reads_live = as_member
            .call("get_item", serde_json::json!({"id": live.id.to_string()}))
            .await;
        assert!(
            member_reads_live["result"]["content"]
                .to_string()
                .contains("Wombat Rust Days"),
            "{member_reads_live}"
        );
        let member_reads_curated = as_member
            .call(
                "get_item",
                serde_json::json!({"id": curated.id.to_string()}),
            )
            .await;
        assert!(
            !member_reads_curated
                .to_string()
                .contains("Wombat Rust Curated"),
            "a member's token read a Curated conference: {member_reads_curated}"
        );
        assert!(
            member_reads_curated.get("error").is_some()
                || member_reads_curated["result"]["isError"] == true,
            "{member_reads_curated}"
        );

        for bad in ["", "not-a-token", &"0".repeat(64)] {
            assert!(
                mcp::Session::open(&app.state, bad).await.is_err(),
                "the token {bad:?} opened a session"
            );
        }
    });
}

/// `run_gather` refuses every one of Ritrovo's gathers, because their ids
/// carry a dot and the tool takes a bare machine name
/// (FRICTION.md, G-MCP-RUN-GATHER-REFUSES-DOTTED-QUERY-IDS). When this fails,
/// the kernel has fixed it: run `ritrovo.upcoming_conferences` here instead.
#[test]
fn trovato_mcp_cannot_run_ritrovo_upcoming_conferences_because_its_id_has_a_dot() {
    serial(async {
        let (app, pool, _recorder) = ai_app().await;
        let (_, editor) = app.login_as_new_member(&["editor"]).await;
        let mut session = mcp::Session::open(&app.state, &api_token(&pool, editor).await)
            .await
            .unwrap();
        let ran = session
            .call(
                "run_gather",
                serde_json::json!({"query_id": "ritrovo.upcoming_conferences"}),
            )
            .await;
        assert_eq!(ran["error"]["code"], -32602, "{ran}");
        assert!(
            ran["error"]["message"]
                .as_str()
                .is_some_and(|m| m.contains("invalid query_id")),
            "{ran}"
        );
    });
}

/// The kernel's chat retrieval needs every word of the question in a
/// conference's indexed text, so a natural question finds nothing: the Live
/// conference matches "Wombat Rust" (above) and is not retrieved for "Wombat
/// Rust conferences", because a conference's text does not say "conferences".
/// The model is then asked with no context at all. FRICTION.md,
/// G-CHAT-RAG-REQUIRES-EVERY-WORD; when this fails, the kernel has fixed it.
#[test]
fn a_question_with_one_word_no_conference_contains_retrieves_no_context() {
    serial(async {
        let (app, pool, recorder) = ai_app().await;
        wombat_conferences(&pool).await;
        recorder.push(Scripted::ok(ai::anthropic_reply("None.", 50, 5)));

        ai::set_fake_keys();
        let (member, token) = member_session(app).await;
        let chat = post_json(
            app,
            "/api/v1/chat",
            serde_json::json!({"message": "Wombat Rust conferences"}),
            &member,
            &token,
        )
        .await;
        ai::unset_keys();
        assert_eq!(chat.status, 200, "{}", chat.body);

        let requests = recorder.requests();
        assert_eq!(requests.len(), 1, "{requests:?}");
        let system = requests[0]["system"].as_str().unwrap_or_default();
        assert!(
            !system.contains("---BEGIN CONTEXT---"),
            "retrieval found a context for a question with a word no conference has: {system}"
        );
        assert!(!system.contains("Wombat Rust Days"), "{system}");
    });
}

// ===========================================================================
// AI Assist on the item forms (A9b, 31.6)
// ===========================================================================
//
// The buttons come from `trovato_ai`'s `tap_form_alter`, and `trovato_ai` is in
// the kernel image, not in the overlay these suites load. Whether they are on
// the conference edit form, and for whom, is checked on the demo
// (`scripts/verify-demo.sh`, "AI Assist"). What a button calls is the kernel's
// route, `POST /api/v1/ai/assist`, and that is served here.

/// A conference description, as an editor would shorten it.
const ASSIST_TEXT: &str = "Wombat Rust Days is a two-day community conference about Rust \
     in production: talks on async runtimes, embedded targets and WebAssembly, a workshop \
     day for newcomers, and an evening meetup for everyone who writes Rust or wants to.";

/// What the Shorten button posts for the field holding [`ASSIST_TEXT`].
fn shorten() -> serde_json::Value {
    serde_json::json!({"text": ASSIST_TEXT, "operation": "shorten"})
}

/// An Anthropic answer with a text block and no thinking block before it.
fn text_first_reply(text: &str, input_tokens: u64, output_tokens: u64) -> serde_json::Value {
    let mut reply = ai::anthropic_reply(text, input_tokens, output_tokens);
    reply["content"] = serde_json::json!([{"type": "text", "text": text}]);
    reply
}

/// A session with the given roles, its user id and its token.
async fn session_with(app: &host::App, roles: &[&str]) -> (String, uuid::Uuid, String) {
    let (cookies, id) = app.login_as_new_member(roles).await;
    let page = app.get("/user/profile", &cookies).await;
    assert_eq!(page.status, 200, "{}", page.body);
    let token = session_token(&page.body);
    (cookies, id, token)
}

/// Shorten as the button posts it: an editor gets the model's text back, and the
/// request carries the field's text under the kernel's shorten instruction, with
/// Ritrovo's chat model and no temperature. An anonymous visitor is refused and
/// no provider is asked.
#[test]
fn ai_assist_shortens_an_editor_s_text_with_ritrovo_s_model_and_refuses_a_visitor() {
    serial(async {
        let (app, _pool, recorder) = ai_app().await;
        let (anon, anon_token) = anonymous_session(app).await;
        let refused = post_json(app, "/api/v1/ai/assist", shorten(), &anon, &anon_token).await;
        assert_eq!(refused.status, 401, "{}", refused.body);
        assert_eq!(recorder.request_count(), 0, "a visitor reached a provider");

        let short = "Wombat Rust Days: two days on Rust in production, a newcomers' workshop \
                     and an evening meetup.";
        recorder.push(Scripted::ok(text_first_reply(short, 120, 30)));
        ai::set_fake_keys();
        let (editor, _, token) = session_with(app, &["editor"]).await;
        let answer = post_json(app, "/api/v1/ai/assist", shorten(), &editor, &token).await;
        ai::unset_keys();
        assert_eq!(answer.status, 200, "{}", answer.body);
        let body: serde_json::Value = serde_json::from_str(&answer.body).unwrap();
        assert_eq!(body["result"], short, "{body}");

        let requests = recorder.requests();
        assert_eq!(requests.len(), 1, "{requests:?}");
        let sent = &requests[0];
        assert_eq!(sent["model"], ai::CHAT_MODEL, "{sent}");
        assert!(sent.get("temperature").is_none(), "{sent}");
        let prompt = sent["messages"][0]["content"].as_str().unwrap_or_default();
        assert!(prompt.starts_with("Shorten the following text"), "{prompt}");
        assert!(prompt.contains(ASSIST_TEXT), "{prompt}");

        // The button sends the page's token every time, and the route spends
        // it and sends no new one, so the next press on the same page is
        // refused (`G-AI-ASSIST-WORKS-ONCE-PER-PAGE`). When the button can use a
        // fresh token, this fails: assert the second answer instead.
        ai::set_fake_keys();
        let again = post_json(app, "/api/v1/ai/assist", shorten(), &editor, &token).await;
        ai::unset_keys();
        assert_eq!(again.status, 403, "{}", again.body);
        assert_eq!(
            recorder.request_count(),
            1,
            "the refused press reached a provider"
        );
    });
}

/// The route reads the first content block only
/// (`G-AI-ASSIST-READS-ONLY-THE-FIRST-CONTENT-BLOCK`). Claude Haiku 5.5 may
/// think first, and then the editor is offered an empty field, after the tokens
/// are spent. When the kernel joins the text blocks, this test fails: assert the
/// shortened text instead and close the entry.
#[test]
fn ai_assist_answers_an_empty_result_when_the_model_thinks_first() {
    serial(async {
        let (app, _pool, recorder) = ai_app().await;
        recorder.push(Scripted::ok(ai::anthropic_reply(
            "A shorter description.",
            120,
            30,
        )));
        ai::set_fake_keys();
        let (editor, _, token) = session_with(app, &["editor"]).await;
        let answer = post_json(app, "/api/v1/ai/assist", shorten(), &editor, &token).await;
        ai::unset_keys();
        assert_eq!(answer.status, 200, "{}", answer.body);
        let body: serde_json::Value = serde_json::from_str(&answer.body).unwrap();
        assert_eq!(body["result"], "", "{body}");
        assert_eq!(body["tokens_used"], 150, "{body}");
        assert_eq!(recorder.request_count(), 1);
    });
}

/// The route checks no token budget (`G-AI-ASSIST-CHECKS-NO-BUDGET`). A member
/// who has spent the authenticated role's 20,000 tokens today is refused by
/// chat and still served by AI Assist, which then logs the whole call as output
/// tokens. When the kernel checks the budget here, this test fails: assert the
/// refusal instead and close the entry.
#[test]
fn ai_assist_spends_past_an_exhausted_budget_and_logs_every_token_as_output() {
    serial(async {
        let (app, pool, recorder) = ai_app().await;
        let (member, id, token) = session_with(app, &[]).await;
        trovato_kernel::services::ai_token_budget::AiTokenBudgetService::new(pool.clone())
            .record_usage(
                &pool,
                trovato_kernel::services::ai_token_budget::UsageLogEntry {
                    user_id: Some(id),
                    plugin_name: "ritrovo-test-spend".to_string(),
                    provider_id: ai::ANTHROPIC_PROVIDER.to_string(),
                    operation: "Chat".to_string(),
                    model: ai::CHAT_MODEL.to_string(),
                    prompt_tokens: 20_000,
                    completion_tokens: 0,
                    total_tokens: 20_000,
                    latency_ms: 0,
                    cost_estimate: None,
                },
            )
            .await
            .unwrap();
        recorder.push(Scripted::ok(text_first_reply("Shorter.", 120, 30)));

        ai::set_fake_keys();
        let chat = post_json(
            app,
            "/api/v1/chat",
            serde_json::json!({"message": "Rust"}),
            &member,
            &token,
        )
        .await;
        assert_eq!(chat.status, 429, "{}", chat.body);
        assert_eq!(recorder.request_count(), 0, "chat passed the budget");

        let page = app.get("/user/profile", &member).await;
        let token = session_token(&page.body);
        let answer = post_json(app, "/api/v1/ai/assist", shorten(), &member, &token).await;
        ai::unset_keys();
        assert_eq!(answer.status, 200, "{}", answer.body);
        assert_eq!(
            recorder.request_count(),
            1,
            "AI Assist did not reach the provider"
        );

        let logged: (String, i32, i32, i32) = sqlx::query_as(
            "SELECT plugin_name, prompt_tokens, completion_tokens, total_tokens \
             FROM ai_usage_log WHERE user_id = $1 AND plugin_name <> 'ritrovo-test-spend'",
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(logged, ("trovato_ai".to_string(), 0, 150, 150));
    });
}
