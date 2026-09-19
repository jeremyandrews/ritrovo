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
/// Two absences are asserted as carefully as the presences, because they are the
/// ones that cost content:
///
/// - **`field_topics` has no widget at all.** It is not declared as a field and
///   cannot be: the kernel has no taxonomic `FieldType`
///   (`G-NO-CATEGORY-REFERENCE-FIELD-KIND`), and declaring it as `Text` would put
///   a single-line box over a JSON array.
/// - **No block editor.** `field_description` is a `Blocks` field, which is the
///   kernel's rich text, and the form renders the container the editor attaches
///   to — but this page is a bare HTML document with a `<style>` block and no
///   script of any kind, so nothing ever attaches. The kernel's WYSIWYG exists
///   and lives on the administrator-only content form.
#[test]
fn the_conference_edit_form_is_generated_from_the_type_and_has_no_editor() {
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
            "field_venue_photos",
            "field_schedule_pdf",
            "field_speakers",
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
            "field_topics has a widget now; G-NO-CATEGORY-REFERENCE-FIELD-KIND may have been fixed"
        );

        // The block editor's container is rendered and nothing loads the editor.
        assert!(
            form.body.contains("data-block-editor"),
            "the Blocks field rendered no editor container: {}",
            form.body
        );
        assert!(
            !form.body.contains("<script"),
            "the edit page now carries script; the WYSIWYG may be reachable: {}",
            form.body
        );
    });
}

/// Saving through that form keeps what it rendered and destroys what it did not.
///
/// This is the test the A6 prompt asks for, and it is a **pin on a loss**, not a
/// feature: `ItemSubmission::from_form` builds a fresh field map out of the POST
/// body and `Item::update` writes it whole, so a field the form could not render
/// was not posted, is not in the map, and is gone from the item. See FRICTION.md,
/// `G-FORM-SAVE-DROPS-EVERY-FIELD-THE-FORM-DID-NOT-RENDER`.
///
/// Three of the brief's fields are lost and each for its own reason:
///
/// - `field_topics` — no widget exists to render it, so it is never posted.
/// - `field_speakers` and `field_venue_photos` — declared `cardinality: -1`,
///   which nothing in the kernel reads (`G-CARDINALITY-IS-INERT`), so the form
///   renders one widget over an array and the array does not survive the round
///   trip.
///
/// **If this test starts failing because the values survived, the kernel has
/// been fixed and the STATUS rows it blocks should be reopened, not the test
/// relaxed.**
#[test]
fn saving_the_edit_form_keeps_rendered_fields_and_drops_the_rest() {
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
            format!("_csrf={}", csrf(&form.body)),
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

        // Lost: the three the form could not round-trip.
        assert!(
            after.get("field_topics").is_none(),
            "field_topics survived a save; G-NO-CATEGORY-REFERENCE-FIELD-KIND \
             and G-FORM-SAVE-DROPS-EVERY-FIELD-THE-FORM-DID-NOT-RENDER may be \
             fixed, so reopen STATUS 36.1 rather than relaxing this: {after}"
        );
        assert!(
            after.get("field_speakers").is_none(),
            "field_speakers survived a save; G-CARDINALITY-IS-INERT may be fixed: {after}"
        );
        assert!(
            after.get("field_venue_photos").is_none(),
            "field_venue_photos survived a save; G-CARDINALITY-IS-INERT may be fixed: {after}"
        );

        // And every value the form did store is a string, whatever the field's
        // declared type: the boolean and the date went in typed and came back as
        // text. FRICTION.md, G-FRONTEND-ITEM-FORM-STORES-EVERY-FIELD-AS-A-STRING.
        assert!(
            after["field_start_date"].is_string(),
            "a Date field came back typed: {after}"
        );
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

/// The CSRF token out of a kernel-rendered form, which names it `_csrf`.
fn csrf(html: &str) -> String {
    let marker = r#"name="_csrf" value=""#;
    let start = html
        .find(marker)
        .unwrap_or_else(|| panic!("no _csrf input in: {html}"))
        + marker.len();
    let rest = &html[start..];
    let end = rest.find('"').unwrap();
    rest[..end].to_string()
}
