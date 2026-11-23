//! Model-assisted drafts for the translation queue.
//!
//! **Off unless a site switches it on**, with the plugin variable
//! `assist_enabled` set to `1` (stored by the kernel as
//! `plugin.ritrovo_translate.assist_enabled`). Off, `tap_cron` returns at once
//! and makes no model call. On, each cron cycle takes the next
//! [`PER_CYCLE`] queued conferences that have no draft, asks the site's
//! configured AI provider for a translation through the kernel's `ai_request`
//! host call, and stores the answer in `ritrovo_translation_draft`.
//!
//! A draft is not a translation. Nothing renders it; the queue page shows it to
//! an editor beside the link to the kernel's translate screen, and the editor
//! saves the reviewed text there. No host call writes `item_translation`, so a
//! plugin cannot save even a reviewed draft itself; that is the residual of
//! `G-TRANSLATION-NO-WRITE-PATH`.
//!
//! The provider sits behind [`Translator`], so the tests drive the whole
//! drafting path with a mock and no model is ever called from a test.

use std::fmt::Write as _;

use trovato_sdk::host;
use trovato_sdk::prelude::*;

/// The plugin variable that switches drafting on. Anything but `"1"` is off.
pub const VARIABLE: &str = "assist_enabled";

/// How many drafts one cron cycle asks for, so a slow provider cannot hold the
/// cycle past the kernel's timeout.
pub const PER_CYCLE: i64 = 3;

/// The queued conferences with no draft yet, soonest first. `$1` is the limit.
/// The same selection as [`crate::queue::QUEUE_SQL`], minus the drafted ones.
const CANDIDATES_SQL: &str = "\
SELECT i.id::text AS id, i.title, i.language, \
       i.fields->'field_description' AS description \
FROM item i \
WHERE i.type = 'conference' \
  AND COALESCE(i.fields->>'field_start_date', '') >= to_char(now(), 'YYYY-MM-DD') \
  AND NOT EXISTS ( \
    SELECT 1 FROM item_translation t \
    WHERE t.item_id = i.id \
      AND t.language = CASE WHEN i.language = 'it' THEN 'en' ELSE 'it' END) \
  AND NOT EXISTS ( \
    SELECT 1 FROM ritrovo_translation_draft d \
    WHERE d.item_id = i.id \
      AND d.language = CASE WHEN i.language = 'it' THEN 'en' ELSE 'it' END) \
ORDER BY i.fields->>'field_start_date', i.title, i.id \
LIMIT $1";

const INSERT_SQL: &str = "\
INSERT INTO ritrovo_translation_draft (item_id, language, title, fields, model, created) \
VALUES ($1::uuid, $2, $3, $4::jsonb, $5, EXTRACT(EPOCH FROM now())::bigint) \
ON CONFLICT (item_id, language) DO UPDATE \
   SET title = EXCLUDED.title, fields = EXCLUDED.fields, \
       model = EXCLUDED.model, created = EXCLUDED.created";

/// Something that answers a chat prompt: the site's AI provider in production,
/// a mock in the tests.
pub trait Translator {
    /// Send a system and a user message; return the reply and the model name.
    fn complete(&self, system: &str, user: &str) -> Result<(String, String), String>;
}

/// The kernel's `ai_request` host call, with the site's default chat provider.
pub struct HostTranslator;

impl Translator for HostTranslator {
    fn complete(&self, system: &str, user: &str) -> Result<(String, String), String> {
        let request = AiRequest {
            operation: AiOperationType::Chat,
            provider_id: None,
            model: None,
            messages: vec![AiMessage::system(system), AiMessage::user(user)],
            input: None,
            options: AiRequestOptions {
                max_tokens: Some(1024),
                ..AiRequestOptions::default()
            },
        };
        host::ai_request(&request)
            .map(|r| (r.content, r.model))
            .map_err(|code| format!("ai_request failed: host error {code}"))
    }
}

/// A conference to be drafted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub id: String,
    pub title: String,
    pub description: String,
    pub language: String,
}

/// A model's answer, parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drafted {
    pub title: String,
    pub description: String,
    pub model: String,
}

/// A language's English name, for the prompt.
fn language_name(code: &str) -> &str {
    match code {
        "en" => "English",
        "it" => "Italian",
        "de" => "German",
        "fr" => "French",
        "es" => "Spanish",
        "pt" => "Portuguese",
        other => other,
    }
}

/// The two messages sent for one conference.
pub fn prompt(source: &Source, target: &str) -> (String, String) {
    let system = format!(
        "You translate conference listings from {from} into {to}. Keep brand names, \
         product names and the conference's own name as the organisers write them; \
         translate the descriptive words around them. Reply with a JSON object and \
         nothing else: {{\"title\": \"...\", \"description\": \"...\"}}. Leave \
         \"description\" empty when you were given none.",
        from = language_name(&source.language),
        to = language_name(target),
    );
    let mut user = format!("Title: {}\n", source.title);
    let _ = write!(user, "Description: {}", source.description);
    (system, user)
}

/// Read a model's reply: the JSON object the prompt asks for, tolerating the
/// code fence a chat model often wraps one in.
pub fn parse_reply(reply: &str) -> Result<(String, String), String> {
    let trimmed = reply.trim();
    let start = trimmed.find('{').ok_or("the reply holds no JSON object")?;
    let end = trimmed.rfind('}').ok_or("the reply holds no JSON object")?;
    let object: serde_json::Value = serde_json::from_str(&trimmed[start..=end])
        .map_err(|e| format!("the reply is not JSON: {e}"))?;
    let title = object
        .get("title")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .ok_or("the reply has no title")?
        .to_string();
    let description = object
        .get("description")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    Ok((title, description))
}

/// Draft one conference into `target`.
pub fn draft(
    translator: &dyn Translator,
    source: &Source,
    target: &str,
) -> Result<Drafted, String> {
    let (system, user) = prompt(source, target);
    let (reply, model) = translator.complete(&system, &user)?;
    let (title, description) = parse_reply(&reply)?;
    Ok(Drafted {
        title,
        description,
        model,
    })
}

/// A description's plain text: a string, or a `{value}` object. Anything else
/// (a block list) is left out rather than sent as JSON for a model to mangle.
fn description_text(value: Option<&serde_json::Value>) -> String {
    value
        .and_then(|v| v.get("value").unwrap_or(v).as_str())
        .unwrap_or("")
        .to_string()
}

/// One cron cycle.
pub fn run(translator: &dyn Translator) -> serde_json::Value {
    let enabled = host::variables_get(VARIABLE, "0").unwrap_or_else(|_| "0".to_string());
    if enabled.trim() != "1" {
        return serde_json::json!({"status": "off"});
    }

    let rows: Vec<serde_json::Value> =
        match host::query_raw(CANDIDATES_SQL, &[serde_json::json!(PER_CYCLE)]) {
            Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
            Err(code) => {
                host::log(
                    "error",
                    "ritrovo_translate",
                    &format!("draft candidates query failed: host error {code}"),
                );
                return serde_json::json!({"status": "error", "host_error": code});
            }
        };

    let mut drafted = 0u32;
    let mut failed = 0u32;
    for row in &rows {
        let text = |key: &str| {
            row.get(key)
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string()
        };
        let source = Source {
            id: text("id"),
            title: text("title"),
            description: description_text(row.get("description")),
            language: text("language"),
        };
        let target = crate::target_language(&source.language);
        match draft(translator, &source, target) {
            Ok(d) => {
                let fields = if d.description.is_empty() {
                    serde_json::json!({})
                } else {
                    serde_json::json!({"field_description": d.description})
                };
                match host::execute_raw(
                    INSERT_SQL,
                    &[
                        serde_json::json!(source.id),
                        serde_json::json!(target),
                        serde_json::json!(d.title),
                        serde_json::json!(fields.to_string()),
                        serde_json::json!(d.model),
                    ],
                ) {
                    Ok(_) => drafted += 1,
                    Err(code) => {
                        failed += 1;
                        host::log(
                            "error",
                            "ritrovo_translate",
                            &format!(
                                "could not store the draft for {}: host error {code}",
                                source.id
                            ),
                        );
                    }
                }
            }
            Err(reason) => {
                failed += 1;
                host::log(
                    "warn",
                    "ritrovo_translate",
                    &format!("no draft for {} ({}): {reason}", source.id, source.title),
                );
            }
        }
    }

    serde_json::json!({"status": "ran", "drafted": drafted, "failed": failed})
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// A provider that answers from a script and records what it was asked.
    struct Mock {
        reply: Result<String, String>,
        asked: RefCell<Vec<(String, String)>>,
    }

    impl Mock {
        fn answering(reply: &str) -> Self {
            Self {
                reply: Ok(reply.to_string()),
                asked: RefCell::new(Vec::new()),
            }
        }
    }

    impl Translator for Mock {
        fn complete(&self, system: &str, user: &str) -> Result<(String, String), String> {
            self.asked
                .borrow_mut()
                .push((system.to_string(), user.to_string()));
            self.reply.clone().map(|r| (r, "mock-model".to_string()))
        }
    }

    fn source(language: &str) -> Source {
        Source {
            id: "00000000-0000-0000-0000-000000000001".to_string(),
            title: "Festival der Zukunft".to_string(),
            description: "Zwei Tage über Software und Gesellschaft.".to_string(),
            language: language.to_string(),
        }
    }

    #[test]
    fn a_draft_is_the_providers_title_and_description() {
        let mock = Mock::answering(
            r#"{"title": "Festival del futuro", "description": "Due giorni su software e società."}"#,
        );
        let d = draft(&mock, &source("de"), "it").unwrap();
        assert_eq!(d.title, "Festival del futuro");
        assert_eq!(d.description, "Due giorni su software e società.");
        assert_eq!(d.model, "mock-model");
    }

    #[test]
    fn the_prompt_names_both_languages_and_carries_the_text() {
        let mock = Mock::answering(r#"{"title": "x", "description": ""}"#);
        draft(&mock, &source("de"), "it").unwrap();
        let asked = mock.asked.borrow();
        let (system, user) = &asked[0];
        assert!(system.contains("from German into Italian"), "{system}");
        assert!(user.contains("Title: Festival der Zukunft"), "{user}");
        assert!(user.contains("Zwei Tage"), "{user}");
    }

    #[test]
    fn a_fenced_reply_is_read() {
        let reply = "Here you go:\n```json\n{\"title\": \"Giornata\", \"description\": \"\"}\n```";
        assert_eq!(
            parse_reply(reply).unwrap(),
            ("Giornata".to_string(), String::new())
        );
    }

    #[test]
    fn a_reply_without_a_title_is_refused() {
        let mock = Mock::answering(r#"{"description": "only this"}"#);
        assert!(draft(&mock, &source("de"), "it").is_err());
        let mock = Mock::answering("I cannot help with that.");
        assert!(draft(&mock, &source("de"), "it").is_err());
    }

    #[test]
    fn a_provider_error_is_passed_on() {
        let mock = Mock {
            reply: Err("no provider configured".to_string()),
            asked: RefCell::new(Vec::new()),
        };
        assert_eq!(
            draft(&mock, &source("it"), "en").unwrap_err(),
            "no provider configured"
        );
    }

    /// Off by default: the native host stub answers a variable with its
    /// default, which is the state of a site that never set it, and the cycle
    /// returns before anything reaches the provider.
    #[test]
    fn off_by_default_and_then_the_provider_is_never_asked() {
        let mock = Mock::answering(r#"{"title": "x"}"#);
        assert_eq!(run(&mock), serde_json::json!({"status": "off"}));
        assert!(mock.asked.borrow().is_empty());
    }

    #[test]
    fn a_description_is_plain_text_or_nothing() {
        assert_eq!(description_text(Some(&serde_json::json!("plain"))), "plain");
        assert_eq!(
            description_text(Some(
                &serde_json::json!({"value": "<p>v</p>", "format": "filtered_html"})
            )),
            "<p>v</p>"
        );
        assert_eq!(
            description_text(Some(&serde_json::json!([{"type": "paragraph"}]))),
            ""
        );
        assert_eq!(description_text(None), "");
    }
}
