//! Which language a conference's own words are in.
//!
//! Shared by two plugins that share no crate: `ritrovo_translate` compiles this
//! file as its `detect` module, and `ritrovo_importer` includes the same file by
//! path. One copy of the word lists, two callers, no third crate for the
//! plugin builder to stage.
//!
//! # What it is for
//!
//! confs.tech says which language a conference is held in through `locales`,
//! and leaves it out often enough that "absent" cannot mean "English". A
//! conference a person creates has no feed at all. Both fall back to this,
//! which reads the only text there is, the conference's name, and answers for
//! the five languages other than English that the feed actually carries:
//! German, French, Spanish, Portuguese and Italian.
//!
//! # How, and why so cautious
//!
//! A conference name is three to eight words, most of them brand names and
//! English loanwords: "Codemotion Roma", "DevFest Lisboa", "JSDay". A
//! statistical model trained on sentences guesses wildly on that. So this looks
//! only for evidence a name cannot carry by accident: function words and the
//! handful of conference nouns each language uses ("Jornadas", "Giornata",
//! "Tage", "Rencontres"), and letters only one or two of the five languages
//! write. A word that two languages share counts half for each.
//!
//! It answers only when one language clearly leads, with at least one
//! unambiguous word or two shared ones, and otherwise says nothing, which the
//! callers turn into the site default. A wrong "Italian" puts an English
//! conference on the translation queue under the wrong target; a missed one
//! leaves it where it would have been anyway. So silence is the cheap error,
//! and it is the one this makes.

/// Languages this detects, as ISO 639-1 codes. English is not among them: it is
/// what the callers fall back to, never something this claims.
pub const DETECTED: &[&str] = &["de", "es", "fr", "it", "pt"];

/// Words that mark exactly one language in a conference name. Two points each.
const STRONG: &[(&str, &[&str])] = &[
    (
        "de",
        &[
            "der",
            "und",
            "für",
            "fuer",
            "im",
            "zum",
            "zur",
            "vom",
            "mit",
            "tage",
            "konferenz",
            "fachkonferenz",
            "entwickler",
            "entwicklertag",
            "unternehmen",
            "anwendertreffen",
            "treffen",
            "praxis",
            "nutzer",
            "deutsche",
            "deutschen",
            "softwareentwicklung",
            "agilen",
            "sicherheit",
        ],
    ),
    (
        "fr",
        &[
            "les",
            "du",
            "et",
            "pour",
            "avec",
            "aux",
            "journée",
            "journées",
            "journee",
            "journees",
            "rencontres",
            "conférence",
            "développeurs",
            "developpeurs",
            "développement",
            "sécurité",
            "francophone",
        ],
    ),
    (
        "es",
        &[
            "el",
            "jornadas",
            "jornada",
            "congreso",
            "conferencia",
            "desarrolladores",
            "encuentro",
            "comunidad",
            "desarrollo",
            "seguridad",
        ],
    ),
    (
        "pt",
        &[
            "os",
            "encontro",
            "conferência",
            "desenvolvedores",
            "desenvolvimento",
            "comunidade",
            "segurança",
        ],
    ),
    (
        "it",
        &[
            "il",
            "gli",
            "della",
            "dello",
            "delle",
            "degli",
            "nel",
            "nella",
            "sul",
            "sulla",
            "giornata",
            "giornate",
            "conferenza",
            "convegno",
            "incontro",
            "settimana",
            "sviluppatori",
            "sviluppo",
            "nazionale",
            "sicurezza",
            "libero",
            "italiana",
            "italiano",
        ],
    ),
];

/// Words that are weak evidence: shared by two of the five languages, or common
/// in English names too ("Los Angeles", "Des Moines", "KubeCon NA"). One point
/// to each language listed, so one of them alone never decides.
///
/// Left out on purpose, because English conference names use them as words of
/// their own: "con" (AI Con), "com" (the domain), "e" and "o" (e-commerce,
/// O'Reilly), "libre" (Libre Graphics Meeting). "y" is here rather than strong
/// for "UX Y'all" and "Y Oslo".
const SHARED: &[(&str, &[&str])] = &[
    (
        "de",
        &[
            "die", "das", "den", "dem", "des", "von", "ein", "eine", "auf",
        ],
    ),
    (
        "fr",
        &["le", "la", "de", "des", "en", "au", "sur", "semaine"],
    ),
    (
        "es",
        &[
            "y", "la", "los", "las", "de", "del", "dos", "en", "para", "semana", "al",
        ],
    ),
    (
        "pt",
        &[
            "da", "do", "das", "dos", "de", "em", "para", "semana", "na", "no",
        ],
    ),
    ("it", &["la", "di", "del", "per", "al", "da"]),
];

/// Letters that mark a language. One point each, counted once per letter.
const LETTERS: &[(char, &[&str])] = &[
    ('ß', &["de"]),
    ('ä', &["de"]),
    ('ö', &["de"]),
    ('ü', &["de"]),
    ('ñ', &["es"]),
    ('¿', &["es"]),
    ('ã', &["pt"]),
    ('õ', &["pt"]),
    ('ç', &["fr", "pt"]),
    ('é', &["fr", "es", "pt"]),
    ('è', &["fr", "it"]),
    ('ê', &["fr", "pt"]),
    ('à', &["fr", "it", "pt"]),
    ('ì', &["it"]),
    ('ò', &["it"]),
    ('ù', &["fr", "it"]),
    ('í', &["es", "pt"]),
    ('ó', &["es", "pt"]),
    ('ú', &["es", "pt"]),
];

/// The language of `text`, when one of [`DETECTED`] clearly leads, or `None`.
///
/// Scores are in half points so a shared word can count for each language that
/// writes it without floating point: a strong word is 4, a shared word 2, a
/// marked letter 1. The winner needs 4 (one strong word, or two shared ones)
/// and must beat the runner-up outright; a tie is silence.
pub fn detect(text: &str) -> Option<&'static str> {
    let lower = text.to_lowercase();
    // Words, lowercased, minus acronyms: a word written in capitals is a name,
    // not a function word. "MIT AI Conference" is not German and "JDevSummit IL"
    // is not Italian, while "IT-Tage" still reads as German from "Tage".
    let owned: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty() && !is_acronym(w))
        .map(str::to_lowercase)
        .collect();
    let words: Vec<&str> = owned.iter().map(String::as_str).collect();

    let mut scores = [0u32; 5];
    let slot = |lang: &str| DETECTED.iter().position(|l| *l == lang);

    for (lang, list) in STRONG {
        if let Some(i) = slot(lang) {
            scores[i] += 4 * count_matches(&words, &lower, list);
        }
    }
    for (lang, list) in SHARED {
        if let Some(i) = slot(lang) {
            scores[i] += 2 * count_matches(&words, &lower, list);
        }
    }
    for (letter, langs) in LETTERS {
        if lower.contains(*letter) {
            for lang in *langs {
                if let Some(i) = slot(lang) {
                    scores[i] += 1;
                }
            }
        }
    }

    let mut ranked: Vec<(u32, &'static str)> = scores
        .iter()
        .zip(DETECTED.iter())
        .map(|(s, l)| (*s, *l))
        .collect();
    ranked.sort_by_key(|r| std::cmp::Reverse(r.0));
    match ranked.as_slice() {
        [(best, lang), (second, _), ..] if *best >= 4 && best > second => Some(lang),
        _ => None,
    }
}

/// A word of two or more letters, all of them capitals.
fn is_acronym(word: &str) -> bool {
    word.chars().filter(|c| c.is_alphabetic()).count() >= 2
        && word.chars().all(|c| !c.is_alphabetic() || c.is_uppercase())
}

/// How many distinct entries of `list` occur in the text: single words as
/// whole words, multi-word entries as a phrase in the lowercased text.
fn count_matches(words: &[&str], lower: &str, list: &[&str]) -> u32 {
    let mut n = 0;
    for entry in list {
        let hit = if entry.contains(' ') {
            lower.contains(entry)
        } else {
            words.contains(entry)
        };
        if hit {
            n += 1;
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::detect;

    /// Real conference names, verbatim, from the confs.tech data the demo
    /// imports (2015 to 2027, read out of the demo database on 2026-11-14), and
    /// from Ritrovo's Italian seed. Each is one this detector names.
    #[test]
    fn names_the_language_of_real_conference_titles() {
        for (title, want) in [
            ("Minds Mastering Machines - M3: LLMs im Unternehmen", "de"),
            (
                "inside agile: Agile Softwareentwicklung im Unternehmen",
                "de",
            ),
            ("heise devSec: KI und Security", "de"),
            ("Festival der Zukunft", "de"),
            ("IT-Tage", "de"),
            ("Bob Konferenz", "de"),
            ("Deutsch OpenStack Tage", "de"),
            ("Das große Trainingsevent für Softwarearchitektur", "de"),
            ("it-sa - Europas führende Fachmesse für IT-Sicherheit", "de"),
            ("Conferencia Rails", "es"),
            ("Incontro DevOps Italia", "it"),
            ("Giornata Nazionale del Testing 2026", "it"),
            ("Conferenza Italiana sul Software Libero 2026", "it"),
        ] {
            assert_eq!(detect(title), Some(want), "{title}");
        }
    }

    /// The other two languages the brief names, which the feed has no title in
    /// at the moment: phrases a French or Portuguese conference is called by.
    #[test]
    fn names_french_and_portuguese_conference_phrases() {
        for (title, want) in [
            ("Rencontres Mondiales du Logiciel Libre", "fr"),
            ("Journées du Développement Logiciel", "fr"),
            ("Encontro de Desenvolvedores Python", "pt"),
            ("Semana da Comunidade de Segurança", "pt"),
            ("Jornadas de Software Libre", "es"),
        ] {
            assert_eq!(detect(title), Some(want), "{title}");
        }
    }

    /// Real titles this says nothing about. The first four are the false
    /// positives an earlier draft produced against the same data: an acronym
    /// read as a function word, and a one-letter word read as Spanish.
    #[test]
    fn says_nothing_about_english_names_and_acronyms() {
        for title in [
            "MIT AI Conference",
            "JDevSummit IL",
            "UX Y'all",
            "Y Oslo",
            "DAM Los Angeles",
            "Data Con LA",
            "AI Con USA",
            "KubeCon + CloudNativeCon NA",
            "No Fluff Just Stuff",
            "Codemotion Roma 2026",
            "RustLab Firenze 2026",
            "PyCon Italia 2026",
            "EuroRust",
            "Libre Graphics Meeting",
            "",
        ] {
            assert_eq!(detect(title), None, "{title}");
        }
    }

    /// A tie is silence: "la" and "de" are three languages' words at once.
    #[test]
    fn a_tie_between_languages_is_silence() {
        assert_eq!(detect("La de"), None);
    }
}
