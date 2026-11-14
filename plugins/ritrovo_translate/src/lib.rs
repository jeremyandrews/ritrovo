//! Language detection and the translation workflow for Ritrovo conferences.
//!
//! - **Detection.** `tap_item_presave` writes `field_language` for a conference
//!   a person creates without one, from [`detect`]. The kernel merges a presave
//!   tap's `fields` into the item before it is written
//!   (`crates/kernel/src/content/item_service.rs:353-387` at the pinned rev).
//!   Imported conferences never pass through here, because the importer writes
//!   `item` by raw SQL (`G-ITEM-API-BYPASSES-ITEM-SERVICE`), so the importer
//!   includes the same [`detect`] module and applies it itself.
//! - **Badge and switcher.** `tap_item_view`, from the item's own language.
//! - **The translation queue.** `/admin/content/translations`, for anyone
//!   holding the kernel's `translate content`: upcoming conferences with no
//!   translation in the site's other language, each linked to the kernel's own
//!   translate screen, which is where a translation is written. See [`queue`].
//! - **Drafts.** `tap_cron` asks a model for a draft of the next few, behind a
//!   site variable that is off by default. See [`assist`].
//!
//! There used to be a `tap_item_insert` here returning a detection result. The
//! kernel discards every `tap_item_insert` output
//! (`G-ITEM-INSERT-OUTPUT-DISCARDED`), so the finding went nowhere; it is gone,
//! and detection lives in presave, whose output the kernel keeps.

use trovato_sdk::prelude::*;

pub mod assist;
pub mod detect;
pub mod queue;
mod web;

/// The site's default language, which the kernel serves unprefixed.
pub const DEFAULT_LANGUAGE: &str = "en";

/// The site's other language, served under `/it/`.
pub const OTHER_LANGUAGE: &str = "it";

/// The language a conference written in `language` should be translated into.
///
/// Italian conferences get English; everything else gets Italian. The site has
/// two languages, so "the other one" is the whole rule, and a German or French
/// conference is translated into Italian rather than into English, which the
/// listing at `/` already reads it in as far as the site is concerned.
pub fn target_language(language: &str) -> &'static str {
    if language == OTHER_LANGUAGE {
        DEFAULT_LANGUAGE
    } else {
        OTHER_LANGUAGE
    }
}

/// The text a conference's language is read from: its title, and its
/// description when that is a plain string or a `{value}` object.
fn conference_text(title: &str, fields: Option<&serde_json::Value>) -> String {
    let description = fields
        .and_then(|f| f.get("field_description"))
        .and_then(|v| v.get("value").unwrap_or(v).as_str())
        .unwrap_or("");
    format!("{title} {description}")
}

/// Write `field_language` on a conference that does not carry one.
///
/// Only when it is absent or empty: a value a person or a form chose is theirs,
/// and the importer's own value (feed first, then the same detector) never
/// reaches this tap. Undetected is English, the site default, so the field is
/// always set and the language filter never hides a conference for lacking it.
#[plugin_tap]
pub fn tap_item_presave(input: serde_json::Value) -> serde_json::Value {
    let is_conference = input
        .get("item_type")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|t| t == "conference");
    if !is_conference {
        return serde_json::json!({});
    }
    let fields = input.get("fields");
    let already = fields
        .and_then(|f| f.get("field_language"))
        .and_then(|v| v.get("value").unwrap_or(v).as_str())
        .is_some_and(|s| !s.trim().is_empty());
    if already {
        return serde_json::json!({});
    }
    let title = input
        .get("title")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let language = detect::detect(&conference_text(title, fields)).unwrap_or(DEFAULT_LANGUAGE);
    serde_json::json!({ "fields": { "field_language": language } })
}

/// The language a conference is held in, as the item says it.
///
/// `field_language` first, the brief's own field, which the importer and
/// [`tap_item_presave`] always set; the item's `language` column after it.
fn item_language(item: &Item) -> String {
    item.fields
        .get("field_language")
        .and_then(|v| v.get("value").unwrap_or(v).as_str())
        .filter(|s| !s.trim().is_empty())
        .map(str::to_lowercase)
        .or_else(|| item.language.clone())
        .unwrap_or_else(|| DEFAULT_LANGUAGE.to_string())
}

/// A language's name in that language, for the badge.
fn endonym(code: &str) -> String {
    match code {
        "en" => "English".to_string(),
        "it" => "Italiano".to_string(),
        "de" => "Deutsch".to_string(),
        "fr" => "Français".to_string(),
        "es" => "Español".to_string(),
        "pt" => "Português".to_string(),
        other => other.to_uppercase(),
    }
}

/// The language badge and both language links for a conference page.
///
/// **Both links, always.** The view tap cannot know which language the page is
/// being rendered in (`G-NO-REQUEST-LANGUAGE`: the input is the stored Item,
/// and the tap runs before the translation overlay), so it cannot pick "the
/// other one". It emits the English link and the Italian link, each marked with
/// its language, and leaves the choice to whatever renders the page, which does
/// know: a theme hides the link whose language is the page's own.
///
/// **This output does not reach the conference page today.** The kernel hands a
/// template the plugin output and its own raw field dump in one string, and
/// `item--conference.html` drops that string to keep the dump off the page
/// (`G-RENDER-CHILDREN-MIXES-FIELD-DUMP-AND-PLUGIN-OUTPUT`). The page's switcher
/// is therefore drawn by that template, which is given `active_language` and
/// shows the one of these same two addresses that is not the page's own; this
/// output stays correct for any template that renders `children`.
#[plugin_tap]
pub fn tap_item_view(item: Item) -> String {
    if item.item_type != "conference" {
        return String::new();
    }
    let lang = item_language(&item);
    let id = item.id;
    format!(
        "<div class=\"lang-badge-switcher\">\
         <span class=\"lang-badge\" lang=\"{lang}\">{name}</span> \
         <a href=\"/item/{id}\" hreflang=\"en\" lang=\"en\" \
            class=\"lang-switcher__link lang-switcher__link--en\">English</a> \
         <a href=\"/it/item/{id}\" hreflang=\"it\" lang=\"it\" \
            class=\"lang-switcher__link lang-switcher__link--it\">Italiano</a>\
         </div>",
        lang = web::escape(&lang),
        name = web::escape(&endonym(&lang)),
    )
}

/// The translation queue's route.
///
/// Gated by the kernel on `translate content`, the permission its own translate
/// screen checks, so the queue and the screen it links to admit the same people.
#[plugin_tap]
pub fn tap_menu() -> Vec<MenuRoute> {
    vec![
        MenuRoute::api("GET", queue::PATH, "translation_queue")
            .title("Translations")
            .permission(queue::PERMISSION)
            .parent("/admin/content")
            .weight(20)
            .visible(),
    ]
}

/// Serve one request.
#[plugin_tap]
pub fn tap_api(request: ApiRequest) -> ApiResponse {
    match request.callback.as_str() {
        "translation_queue" => queue::page(&request),
        other => ApiResponse::error(404, &format!("no such callback: {other}")),
    }
}

/// Draft translations with a model, when a site has switched that on.
#[plugin_tap]
pub fn tap_cron(_input: CronInput) -> serde_json::Value {
    assist::run(&assist::HostTranslator)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn conference(title: &str, fields: HashMap<String, serde_json::Value>) -> Item {
        Item {
            id: Uuid::nil(),
            item_type: "conference".to_string(),
            title: title.to_string(),
            fields,
            status: 1,
            author_id: Uuid::nil(),
            current_revision_id: None,
            stage_id: live_stage_id(),
            created: 0,
            changed: 0,
            language: None,
        }
    }

    #[test]
    fn italian_conferences_get_english_and_everything_else_italian() {
        assert_eq!(target_language("it"), "en");
        assert_eq!(target_language("en"), "it");
        assert_eq!(target_language("de"), "it");
        assert_eq!(target_language("fr"), "it");
    }

    #[test]
    fn presave_writes_the_detected_language_of_an_unmarked_conference() {
        let out = __inner_tap_item_presave(serde_json::json!({
            "item_type": "conference",
            "title": "Giornata Nazionale del Testing 2026",
            "fields": {},
            "status": 1,
        }));
        assert_eq!(out["fields"]["field_language"], "it");
    }

    #[test]
    fn presave_reads_the_description_as_well_as_the_title() {
        let out = __inner_tap_item_presave(serde_json::json!({
            "item_type": "conference",
            "title": "Coderful",
            "fields": {"field_description": "Due giornate della comunità, con il meglio degli sviluppatori"},
        }));
        assert_eq!(out["fields"]["field_language"], "it");
    }

    #[test]
    fn presave_defaults_an_undetected_conference_to_english() {
        let out = __inner_tap_item_presave(serde_json::json!({
            "item_type": "conference",
            "title": "RustConf",
            "fields": {},
        }));
        assert_eq!(out["fields"]["field_language"], "en");
    }

    #[test]
    fn presave_leaves_a_chosen_language_alone() {
        let out = __inner_tap_item_presave(serde_json::json!({
            "item_type": "conference",
            "title": "Giornata Nazionale del Testing",
            "fields": {"field_language": "en"},
        }));
        assert_eq!(out, serde_json::json!({}));
    }

    #[test]
    fn presave_ignores_other_types() {
        let out = __inner_tap_item_presave(serde_json::json!({
            "item_type": "speaker",
            "title": "Giornata del relatore",
            "fields": {},
        }));
        assert_eq!(out, serde_json::json!({}));
    }

    #[test]
    fn view_badges_from_field_language_not_the_title() {
        // An Italian conference with an English-looking title, the case the old
        // title heuristic got wrong (Codemotion Roma was badged "English").
        let mut fields = HashMap::new();
        fields.insert("field_language".to_string(), serde_json::json!("it"));
        let html = __inner_tap_item_view(conference("Codemotion Roma 2026", fields));
        assert!(
            html.contains("<span class=\"lang-badge\" lang=\"it\">Italiano</span>"),
            "{html}"
        );
    }

    #[test]
    fn view_falls_back_to_the_item_language() {
        let mut item = conference("Festival der Zukunft", HashMap::new());
        item.language = Some("de".to_string());
        let html = __inner_tap_item_view(item);
        assert!(html.contains("lang=\"de\">Deutsch</span>"), "{html}");
    }

    #[test]
    fn view_links_both_languages_at_routes_that_exist() {
        let html = __inner_tap_item_view(conference("RustConf", HashMap::new()));
        let id = Uuid::nil();
        assert!(
            html.contains(&format!("href=\"/item/{id}\" hreflang=\"en\"")),
            "{html}"
        );
        assert!(
            html.contains(&format!("href=\"/it/item/{id}\" hreflang=\"it\"")),
            "{html}"
        );
        assert!(!html.contains("/conferences/"), "the old 404 link: {html}");
    }

    #[test]
    fn view_is_empty_for_other_types() {
        let mut item = conference("Ada", HashMap::new());
        item.item_type = "speaker".to_string();
        assert!(__inner_tap_item_view(item).is_empty());
    }
}
