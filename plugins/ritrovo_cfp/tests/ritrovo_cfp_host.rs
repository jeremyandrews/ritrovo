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

// ===========================================================================
// The conference assistant (A9b, STATUS 31.8)
// ===========================================================================
//
// `ritrovo_cfp`'s scope in the kernel's AI assistant, driven through the
// kernel's own routes as an editor's browser drives them: the conversation
// page, the message stream, and the proposal's Apply button. Ritrovo's chat
// provider is pointed at the kernel's scripted provider (`tests/host/ai.rs`),
// which plays the model: it says which tool to call, and records every request
// the kernel sends it, so the tool results the model would have read can be
// checked. No model is called.

#[path = "../../../tests/host/ai.rs"]
mod ai;

use trovato_kernel::services::ai_tools::scripted_provider::{self, Recorder, Scripted};

const SCOPE: &str = "ritrovo_conference";

/// The app with `ritrovo_cfp` loaded and `demo/config` imported, Ritrovo's
/// providers pointed at a fresh scripted provider, and three conferences: one
/// Live, one Incoming and one Curated, all on the same topic in the same city.
struct AssistantFixture {
    app: &'static host::App,
    pool: sqlx::PgPool,
    recorder: Recorder,
    live: trovato_kernel::models::Item,
    hidden: [String; 2],
    cfp_end: String,
    corrected: String,
}

async fn assistant_fixture() -> AssistantFixture {
    let pool = host::fresh_pool().await;
    host::prepare_app_database(&pool, &[PLUGIN]).await;
    host::import_demo_config(&pool).await;
    let app = host::app().await;
    host::clear_rate_limits().await;
    let (url, recorder) = scripted_provider::start().await;
    ai::point_providers_at(&pool, &url).await;

    sqlx::query("DELETE FROM item WHERE type = 'conference' AND title LIKE 'Wombat Rust%'")
        .execute(&pool)
        .await
        .unwrap();
    let day = |offset: i32| {
        let pool = pool.clone();
        async move {
            sqlx::query_scalar::<_, String>("SELECT to_char(CURRENT_DATE + $1::int, 'YYYY-MM-DD')")
                .bind(offset)
                .fetch_one(&pool)
                .await
                .unwrap()
        }
    };
    let (start, end, cfp_end, corrected) =
        (day(60).await, day(62).await, day(5).await, day(12).await);
    let fields = |city: &str| {
        serde_json::json!({
            "field_start_date": start,
            "field_end_date": end,
            "field_city": city,
            "field_country": "Wombatland",
            "field_topic_names": "Rust, Systems",
            "field_url": "https://wombat-rust.example",
            "field_cfp_url": "https://wombat-rust.example/cfp",
            "field_cfp_end_date": cfp_end,
        })
    };
    let disp = host::dispatcher(PLUGIN);
    let items = host::items(&pool, &disp);
    let live = host::create_conference(
        &pool,
        &items,
        "Wombat Rust Days",
        None,
        fields("Wombatville"),
    )
    .await;
    for (title, stage) in [
        ("Wombat Rust Incoming", host::INCOMING_STAGE),
        ("Wombat Rust Curated", host::CURATED_STAGE),
    ] {
        host::create_conference(&pool, &items, title, Some(stage), fields("Wombatville")).await;
    }
    host::settle_cache_generations().await;

    AssistantFixture {
        app,
        pool,
        recorder,
        live,
        hidden: [
            "Wombat Rust Incoming".to_string(),
            "Wombat Rust Curated".to_string(),
        ],
        cfp_end,
        corrected,
    }
}

/// The session's CSRF token from a kernel page's `csrf-token` meta tag.
fn csrf(html: &str) -> String {
    let marker = r#"name="csrf-token" content=""#;
    let start = html
        .find(marker)
        .unwrap_or_else(|| panic!("no CSRF meta tag in: {html:.600}"))
        + marker.len();
    html[start..start + html[start..].find('"').unwrap()].to_string()
}

/// A fresh token for this session: tokens are single use.
async fn fresh_token(app: &host::App, cookies: &str) -> String {
    let page = app.get("/user/profile", cookies).await;
    assert_eq!(page.status, 200, "{}", page.body);
    csrf(&page.body)
}

async fn post_json(
    app: &host::App,
    path: &str,
    body: serde_json::Value,
    cookies: &str,
) -> host::Response {
    let token = fresh_token(app, cookies).await;
    app.send(
        axum::http::Request::post(path)
            .header("content-type", "application/json")
            .header("x-csrf-token", token)
            .header(axum::http::header::COOKIE, cookies)
            .body(axum::body::Body::from(body.to_string()))
            .unwrap(),
    )
    .await
}

/// Open (or reopen) the conversation about `conference`, returning its id.
async fn open_conversation(app: &host::App, cookies: &str, conference: uuid::Uuid) -> uuid::Uuid {
    let page = app
        .get(&format!("/ai/assistant/{SCOPE}/{conference}"), cookies)
        .await;
    assert_eq!(page.status, 200, "{}", page.body);
    let marker = "data-conversation-id=\"";
    let start = page
        .body
        .find(marker)
        .unwrap_or_else(|| panic!("no conversation id in: {:.600}", page.body))
        + marker.len();
    let end = start + page.body[start..].find('"').unwrap();
    uuid::Uuid::parse_str(&page.body[start..end]).unwrap()
}

/// Every SSE event in a body, parsed.
fn sse_events(sse: &str) -> Vec<serde_json::Value> {
    sse.lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .filter_map(|payload| serde_json::from_str(payload.trim()).ok())
        .collect()
}

/// The Anthropic Messages shape of a tool call.
fn calls(id: &str, tool: &str, input: serde_json::Value) -> Scripted {
    Scripted::ok(serde_json::json!({
        "model": ai::CHAT_MODEL,
        "content": [{"type": "tool_use", "id": id, "name": tool, "input": input}],
        "stop_reason": "tool_use",
        "usage": {"input_tokens": 40, "output_tokens": 12}
    }))
}

async fn stored_cfp_end(pool: &sqlx::PgPool, id: uuid::Uuid) -> String {
    sqlx::query_scalar("SELECT fields->>'field_cfp_end_date' FROM item WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// An editor opens the conference assistant on a Live conference and asks two
/// things. The read tool runs at once, through the gather pipeline as the
/// editor on Live, and its result reaches the model with the Live conference
/// in it and neither the Incoming nor the Curated one. The write tool only
/// proposes: the CFP end date is unchanged until the editor applies the
/// proposal, and applying it makes the change through the kernel's item
/// service, once.
#[test]
fn an_editor_s_read_tool_runs_and_the_cfp_date_write_only_proposes_until_applied() {
    serial(async {
        let f = assistant_fixture().await;
        let app = f.app;
        ai::set_fake_keys();
        f.recorder.push(calls(
            "t1",
            "find_conferences",
            serde_json::json!({"topic": "rust", "city": "wombatville"}),
        ));
        f.recorder.push(calls(
            "t2",
            "propose_cfp_end_date",
            serde_json::json!({"date": f.corrected, "reason": "The organisers extended it."}),
        ));
        f.recorder.push(Scripted::ok(ai::anthropic_reply(
            "I found Wombat Rust Days and proposed the new date.",
            30,
            10,
        )));

        let (editor, _) = app.login_as_new_member(&["editor"]).await;

        // The kernel's launcher is on the conference page for an editor.
        let page = app.get(&format!("/item/{}", f.live.id), &editor).await;
        assert_eq!(page.status, 200, "{}", page.body);
        assert!(
            page.body
                .contains(&format!("/ai/assistant/{SCOPE}/{}", f.live.id)),
            "no assistant launcher on the conference page for an editor"
        );

        let conversation = open_conversation(app, &editor, f.live.id).await;
        let answer = post_json(
            app,
            &format!("/api/v1/assistant/{conversation}/message"),
            serde_json::json!({"message": "Which Rust conferences are in Wombatville? Also the CFP was extended a week."}),
            &editor,
        )
        .await;
        ai::unset_keys();
        assert_eq!(answer.status, 200, "{}", answer.body);
        let events = sse_events(&answer.body);
        let types: Vec<&str> = events.iter().filter_map(|e| e["type"].as_str()).collect();
        assert_eq!(
            types,
            [
                "turn_start",
                "tool_call",
                "tool_result",
                "proposal",
                "assistant",
                "done"
            ],
            "{}",
            answer.body
        );

        // The snapshot the model starts from is this conference, from the
        // plugin's context tap.
        let requests = f.recorder.requests();
        assert_eq!(requests.len(), 3, "{requests:?}");
        let system = requests[0]["system"].to_string();
        assert!(system.contains("Wombat Rust Days"), "{system}");
        assert!(
            system.contains(&format!("CFP end date: {}", f.cfp_end)),
            "{system}"
        );
        assert_eq!(requests[0]["model"], ai::CHAT_MODEL);
        assert!(
            requests[0].get("temperature").is_none(),
            "{:?}",
            requests[0]
        );

        // The read ran, and what the model read back is the Live conference,
        // with its address, and nothing from another stage.
        let read_back = requests[1]["messages"].to_string();
        assert!(read_back.contains("tool_result"), "{read_back}");
        assert!(read_back.contains("Wombat Rust Days"), "{read_back}");
        assert!(read_back.contains("Wombatville"), "{read_back}");
        assert!(
            read_back.contains("\\\"url\\\":\\\"/"),
            "no address in {read_back}"
        );
        for hidden in &f.hidden {
            assert!(
                !read_back.contains(hidden.as_str()),
                "{hidden} reached the model: {read_back}"
            );
        }

        // The write only proposed.
        let proposal = events
            .iter()
            .find(|e| e["type"] == "proposal")
            .unwrap()
            .clone();
        let summary = proposal.to_string();
        assert!(
            summary.contains(&format!(
                "Change the CFP end date of Wombat Rust Days from {} to {}",
                f.cfp_end, f.corrected
            )),
            "{summary}"
        );
        assert_eq!(stored_cfp_end(&f.pool, f.live.id).await, f.cfp_end);
        let (proposal_id, status): (uuid::Uuid, String) =
            sqlx::query_as("SELECT id, status FROM ai_proposal WHERE conversation_id = $1")
                .bind(conversation)
                .fetch_one(&f.pool)
                .await
                .unwrap();
        assert_eq!(status, "proposed");

        // The assistant's calls are metered like every other chat call.
        let usage: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ai_usage_log WHERE plugin_name = 'kernel_assistant' AND user_id = \
             (SELECT user_id FROM ai_conversation WHERE id = $1)",
        )
        .bind(conversation)
        .fetch_one(&f.pool)
        .await
        .unwrap();
        assert!(usage >= 1, "the assistant's turn wrote no usage row");

        // Applying is what makes the change, through the item service, once.
        let apply = format!("/api/v1/assistant/{conversation}/proposals/{proposal_id}/apply");
        let applied = post_json(app, &apply, serde_json::json!({}), &editor).await;
        assert_eq!(applied.status, 200, "{}", applied.body);
        assert_eq!(stored_cfp_end(&f.pool, f.live.id).await, f.corrected);
        let log: Option<String> = sqlx::query_scalar(
            "SELECT log FROM item_revision WHERE item_id = $1 ORDER BY id DESC LIMIT 1",
        )
        .bind(f.live.id)
        .fetch_one(&f.pool)
        .await
        .unwrap();
        assert!(
            log.as_deref()
                .is_some_and(|l| l.contains("The organisers extended it.")),
            "{log:?}"
        );
        let again = post_json(app, &apply, serde_json::json!({}), &editor).await;
        assert_eq!(again.status, 409, "{}", again.body);
        assert_eq!(
            f.recorder.request_count(),
            3,
            "applying asked the model again"
        );
    });
}

/// A date that would close the CFP after the conference ends is never saved.
///
/// The plugin refuses it when the kernel asks it to describe the change, and
/// refuses it again when the editor applies. The kernel still files a proposal
/// for the refused description, described from the raw call instead of the
/// plugin's summary (`crates/kernel/src/services/ai_assistant.rs`, the Write
/// arm: "a Describe that failed ... still yields a proposal"), so applying is
/// where the refusal lands: the proposal is marked failed with the plugin's
/// reason and the conference is unchanged. FRICTION.md,
/// G-ASSISTANT-WRITE-CANNOT-REFUSE-A-PROPOSAL.
#[test]
fn a_cfp_date_after_the_conference_ends_is_refused_at_apply_and_never_saved() {
    serial(async {
        let f = assistant_fixture().await;
        let app = f.app;
        f.recorder.push(calls(
            "t1",
            "propose_cfp_end_date",
            serde_json::json!({"date": "2999-01-01"}),
        ));
        f.recorder
            .push(Scripted::ok(ai::anthropic_reply("I proposed it.", 30, 10)));

        let (editor, _) = app.login_as_new_member(&["editor"]).await;
        let conversation = open_conversation(app, &editor, f.live.id).await;
        let answer = post_json(
            app,
            &format!("/api/v1/assistant/{conversation}/message"),
            serde_json::json!({"message": "Move the CFP to 2999-01-01"}),
            &editor,
        )
        .await;
        assert_eq!(answer.status, 200, "{}", answer.body);
        let proposal = sse_events(&answer.body)
            .into_iter()
            .find(|e| e["type"] == "proposal")
            .unwrap_or_else(|| panic!("{}", answer.body));
        // Not the plugin's summary: the plugin refused to describe it.
        let description = proposal["description"].as_str().unwrap();
        assert!(
            !description.contains("Change the CFP end date"),
            "{description}"
        );

        let proposal_id = proposal["proposal_id"].as_str().unwrap();
        let applied = post_json(
            app,
            &format!("/api/v1/assistant/{conversation}/proposals/{proposal_id}/apply"),
            serde_json::json!({}),
            &editor,
        )
        .await;
        let status: String = sqlx::query_scalar("SELECT status FROM ai_proposal WHERE id = $1")
            .bind(uuid::Uuid::parse_str(proposal_id).unwrap())
            .fetch_one(&f.pool)
            .await
            .unwrap();
        assert_eq!(status, "failed", "{}", applied.body);
        assert!(
            applied.body.contains("A CFP cannot close after"),
            "{}",
            applied.body
        );
        assert_eq!(stored_cfp_end(&f.pool, f.live.id).await, f.cfp_end);
    });
}

/// A member holds `use ai` but neither `use ai assistant` nor `edit any
/// content`: no launcher on the conference page, the conversation page is
/// refused, an editor's conversation is not theirs to post to, and no provider
/// is asked anything.
#[test]
fn a_member_without_the_scope_s_permission_is_refused_and_no_model_is_asked() {
    serial(async {
        let f = assistant_fixture().await;
        let app = f.app;
        let (member, _) = app.login_as_new_member(&[]).await;

        let page = app.get(&format!("/item/{}", f.live.id), &member).await;
        assert_eq!(page.status, 200, "{}", page.body);
        assert!(
            !page.body.contains("/ai/assistant/"),
            "a member was offered the assistant launcher"
        );

        let refused = app
            .get(&format!("/ai/assistant/{SCOPE}/{}", f.live.id), &member)
            .await;
        assert_eq!(refused.status, 403, "{}", refused.body);

        let (editor, _) = app.login_as_new_member(&["editor"]).await;
        let conversation = open_conversation(app, &editor, f.live.id).await;
        let posted = post_json(
            app,
            &format!("/api/v1/assistant/{conversation}/message"),
            serde_json::json!({"message": "Change the CFP date"}),
            &member,
        )
        .await;
        assert_ne!(posted.status, 200, "{}", posted.body);
        assert_eq!(f.recorder.request_count(), 0, "a provider was asked");
        assert_eq!(stored_cfp_end(&f.pool, f.live.id).await, f.cfp_end);
    });
}
