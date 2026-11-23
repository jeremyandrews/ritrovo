//! What the suites that exercise Ritrovo's AI configuration share.
//!
//! Included beside `mod.rs` with `#[path = "../../../tests/host/ai.rs"] mod ai;`
//! by the two suites that need it, `ritrovo_translate` (whose drafting job is
//! the one place Ritrovo's own code calls `ai_request`) and `ritrovo_access`
//! (which serves the kernel's chat, search and admin routes). Kept out of
//! `mod.rs` because it needs `tracing-subscriber`, and every suite includes
//! `mod.rs`.
//!
//! **No model is ever called from a test.** The providers are Ritrovo's own, as
//! `demo/config` imports them, with one change made here: their `base_url` is
//! pointed at the kernel's scripted provider
//! (`trovato_kernel::services::ai_tools::scripted_provider`), an HTTP server on
//! loopback that answers from a queue and records every request body. That is
//! how the kernel's own AI tests run (`crates/kernel/tests/ai_assistant_test.rs`
//! writes it straight into `site_config.ai_providers`). Everything else, the
//! models, the key variable names, the defaults, the budgets and the prices,
//! is the configuration under test.

#![allow(dead_code)]

use std::sync::{Arc, Mutex, OnceLock};

use sqlx::PgPool;
use trovato_kernel::models::SiteConfig;
use trovato_kernel::services::ai_provider::AiProviderService;
use trovato_kernel::services::ai_token_budget::AiTokenBudgetService;
use trovato_kernel::tap::{RequestServices, TapDispatcher};

/// Ritrovo's chat provider and its embedding provider, as
/// `demo/config/variable.ai_providers.yml` identifies them.
pub const ANTHROPIC_PROVIDER: &str = "0193a5a0-0005-7000-8000-000000000001";
pub const OPENAI_PROVIDER: &str = "0193a5a0-0005-7000-8000-000000000002";

/// The environment variables the two providers name. The kernel reads them at
/// call time; nothing stores the value.
pub const ANTHROPIC_KEY_ENV: &str = "RITROVO_ANTHROPIC_API_KEY";
pub const OPENAI_KEY_ENV: &str = "RITROVO_OPENAI_API_KEY";

/// The chat model Ritrovo configures.
pub const CHAT_MODEL: &str = "claude-haiku-5-5";

/// Known fake key values. Shaped so nothing could mistake them for a real key,
/// and so a search for them cannot match anything but themselves.
pub const FAKE_ANTHROPIC_KEY: &str = "ritrovo-fake-anthropic-key-0f3b9c2e7a41";
pub const FAKE_OPENAI_KEY: &str = "ritrovo-fake-openai-key-5d8e1a6c0b92";

/// Set both key variables to the fake values.
///
/// The suites run their tests one at a time ([`crate::host::serial`]), so nothing
/// reads the environment while this writes it.
pub fn set_fake_keys() {
    // SAFETY: the test binary runs one test at a time under `serial`, and no
    // other thread reads these two variables while it runs.
    unsafe {
        std::env::set_var(ANTHROPIC_KEY_ENV, FAKE_ANTHROPIC_KEY);
        std::env::set_var(OPENAI_KEY_ENV, FAKE_OPENAI_KEY);
    }
}

/// Remove both key variables: the state of CI and of a demo started without
/// keys.
pub fn unset_keys() {
    // SAFETY: as for `set_fake_keys`.
    unsafe {
        std::env::remove_var(ANTHROPIC_KEY_ENV);
        std::env::remove_var(OPENAI_KEY_ENV);
    }
}

/// Point Ritrovo's imported providers at `base_url`, and nothing else.
///
/// Panics, naming what is missing, unless `demo/config` has put both of
/// Ritrovo's providers in `site_config` with the key variables and models this
/// repository documents: a test that silently made up its own providers would
/// prove nothing about the configuration that ships.
pub async fn point_providers_at(pool: &PgPool, base_url: &str) {
    let stored = SiteConfig::get(pool, "ai_providers")
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("demo/config imported no ai_providers"));
    let mut providers = stored
        .as_array()
        .cloned()
        .unwrap_or_else(|| panic!("ai_providers is not a list: {stored}"));
    for (id, env, operation, model) in [
        (ANTHROPIC_PROVIDER, ANTHROPIC_KEY_ENV, "chat", CHAT_MODEL),
        (
            OPENAI_PROVIDER,
            OPENAI_KEY_ENV,
            "embedding",
            "text-embedding-3-small",
        ),
    ] {
        let provider = providers
            .iter()
            .find(|p| p["id"] == id)
            .unwrap_or_else(|| panic!("no provider {id} in {stored}"));
        assert_eq!(provider["api_key_env"], env, "{provider}");
        assert_eq!(provider["enabled"], true, "{provider}");
        assert!(
            provider["models"].as_array().is_some_and(|m| m
                .iter()
                .any(|m| m["operation"] == operation && m["model"] == model)),
            "provider {id} has no {operation} model {model}: {provider}"
        );
    }
    for provider in &mut providers {
        provider["base_url"] = serde_json::json!(base_url);
    }
    SiteConfig::set(pool, "ai_providers", serde_json::json!(providers))
        .await
        .unwrap();
}

/// Request services the way a server builds them for a tap: with the kernel's
/// AI provider and budget services, which [`crate::host::services`] leaves out.
pub fn services(pool: &PgPool, disp: &Arc<TapDispatcher>) -> RequestServices {
    RequestServices::for_background(
        pool.clone(),
        Some(Arc::new(AiProviderService::new(pool.clone()))),
        Some(Arc::new(AiTokenBudgetService::new(pool.clone()))),
        crate::host::offline_client(),
    )
    .with_plugin_runtime(disp.runtime().clone())
}

/// An Anthropic Messages API answer carrying `text`, the shape the provider
/// sends: a thinking block first (Haiku 5.5 thinks by default), then the text.
pub fn anthropic_reply(text: &str, input_tokens: u64, output_tokens: u64) -> serde_json::Value {
    serde_json::json!({
        "id": "msg_scripted",
        "type": "message",
        "role": "assistant",
        "model": CHAT_MODEL,
        "content": [
            {"type": "thinking", "thinking": "", "signature": "scripted"},
            {"type": "text", "text": text}
        ],
        "stop_reason": "end_turn",
        "usage": {"input_tokens": input_tokens, "output_tokens": output_tokens}
    })
}

/// What Anthropic answers a request that carries no key. It does not echo the
/// key it was given, and neither does this.
pub fn anthropic_no_key() -> serde_json::Value {
    serde_json::json!({
        "type": "error",
        "error": {
            "type": "authentication_error",
            "message": "x-api-key header is required"
        }
    })
}

/// Every log line the kernel and the plugins have written in this binary,
/// from the first call of this function on.
///
/// A global `tracing` subscriber writing into memory. Installed once per test
/// binary; the first call installs it, so call it at the start of a test.
pub fn captured_log() -> String {
    let buffer = capture();
    let bytes = buffer.lock().unwrap_or_else(|p| p.into_inner());
    String::from_utf8_lossy(&bytes).into_owned()
}

fn capture() -> Arc<Mutex<Vec<u8>>> {
    static BUFFER: OnceLock<Arc<Mutex<Vec<u8>>>> = OnceLock::new();
    BUFFER
        .get_or_init(|| {
            let buffer = Arc::new(Mutex::new(Vec::new()));
            let writer = Captured(buffer.clone());
            let subscriber = tracing_subscriber::fmt()
                .with_max_level(tracing::Level::TRACE)
                .with_ansi(false)
                .with_writer(move || writer.clone())
                .finish();
            tracing::subscriber::set_global_default(subscriber)
                .expect("no other global subscriber in this test binary");
            buffer
        })
        .clone()
}

#[derive(Clone)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for Captured {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Every text value in the tables the AI path writes, as one string, for a
/// search for a key value.
pub async fn stored_ai_text(pool: &PgPool) -> String {
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT key || ' ' || value::text FROM site_config \
         UNION ALL SELECT row_to_json(l)::text FROM ai_usage_log l",
    )
    .fetch_all(pool)
    .await
    .unwrap();
    rows.join("\n")
}
