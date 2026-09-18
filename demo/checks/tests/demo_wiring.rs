//! The demo's wiring, asserted.
//!
//! Each test here pins one thing that is invisible to the compiler and only
//! shows up as a broken demo: a plugin missing from the enable list, a search
//! path in the wrong order, the front-page config gone, the kernel image drifting
//! away from the release the docs name. All of them read the real files.

// Test code may panic: a file that cannot be read or parsed IS the failure this
// test reports, so `unwrap` here is the assertion, not a shortcut.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::PathBuf;

use ritrovo_demo_checks::repo_root;

/// The released kernel the demo runs against. Every place that names it must
/// agree, which is what `kernel_release_is_named_consistently` checks.
const KERNEL_IMAGE: &str = "ghcr.io/jeremyandrews/trovato:0.102.0";

/// The compose file the demo is driven from.
const COMPOSE: &str = "docker-compose.demo.yml";

/// Every template on the demo's `TEMPLATES_DIR` search path.
///
/// Some are vendored from the Trovato release and some are Ritrovo's own;
/// `scripts/check-tutorial-templates.sh` holds the first kind to the release and
/// names the second kind as owned. This list is what must EXIST, either way: a
/// gather whose template is missing falls back to the kernel's generic table,
/// which prints every column of every row including `search_vector`.
const TUTORIAL_TEMPLATES: &[&str] = &[
    "page--front.html",
    "elements/item--conference.html",
    "elements/item--speaker.html",
    "gather/includes/conf-card.html",
    "gather/query--ritrovo.all_speakers.html",
    "gather/query--ritrovo.by_city.html",
    "gather/query--ritrovo.by_country.html",
    "gather/query--ritrovo.by_topic.html",
    "gather/query--ritrovo.cfps_closing_soon.html",
    "gather/query--ritrovo.conferences_this_month.html",
    "gather/query--ritrovo.open_cfps.html",
    "gather/query--ritrovo.upcoming_conferences.html",
    "gather/query--upcoming_conferences.html",
];

/// Every gather that has a route of its own must have a template of its own.
///
/// The kernel falls back to a generic table that prints every column of every
/// row, `search_vector` included, which is what /location/Germany rendered before
/// this repository owned the location templates.
const ROUTED_GATHERS: &[&str] = &[
    "ritrovo.all_speakers",
    "ritrovo.by_city",
    "ritrovo.by_country",
    "ritrovo.by_topic",
    "ritrovo.cfps_closing_soon",
    "ritrovo.conferences_this_month",
    "ritrovo.open_cfps",
    "ritrovo.upcoming_conferences",
];

fn read(relative: &str) -> String {
    let path = repo_root().join(relative);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// The value of a `KEY: value` line in the compose file, with quotes stripped.
///
/// Deliberately a line scan rather than a YAML parse: the point is to assert on
/// what a reader sees in the file, and pulling a YAML crate into a workspace that
/// otherwise compiles to WASM buys nothing.
fn compose_env(key: &str) -> String {
    let compose = read(COMPOSE);
    let needle = format!("{key}:");
    let line = compose
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with(&needle))
        .unwrap_or_else(|| panic!("{COMPOSE} has no {key} entry"));
    line[needle.len()..]
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}

/// Directory names under `plugins/`, which are also the plugin machine names.
fn workspace_plugins() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(repo_root().join("plugins"))
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn every_plugin_is_in_the_demo_enable_list() {
    let bootstrap = read("scripts/demo-bootstrap.sh");
    let plugins = workspace_plugins();
    assert_eq!(
        plugins.len(),
        5,
        "expected five Ritrovo plugins: {plugins:?}"
    );

    for name in plugins {
        assert!(
            bootstrap.contains(&name),
            "scripts/demo-bootstrap.sh never mentions '{name}', so the demo \
             installs a plugin it never enables"
        );
    }
}

/// The line number of the first line that RUNS the given trovato subcommand.
///
/// Commands only, never comments. Matching the bare strings found both in the
/// header comment, which explains the ordering, and in the code that implements
/// it, and the header happens to mention them in the right order, so the test
/// passed by reading prose rather than by reading the script.
fn first_command_line(script: &str, subcommand: &str) -> usize {
    script
        .lines()
        .position(|line| {
            let line = line.trim_start();
            line.starts_with("./trovato") && line.contains(subcommand)
        })
        .unwrap_or_else(|| panic!("demo-bootstrap.sh never runs `trovato {subcommand}`"))
}

#[test]
fn config_is_imported_before_any_plugin_is_enabled() {
    // The install-order trap: ritrovo_importer resolves the topic taxonomy once,
    // in tap_install, and tap_install fires when the plugin is first enabled and
    // the server restarts. Enable it before the taxonomy exists and it imports
    // every conference untagged, recoverable only by resetting
    // tap_install_called and restarting. Line order in the script is the fix, so
    // line order is what this pins.
    let bootstrap = read("scripts/demo-bootstrap.sh");
    let import = first_command_line(&bootstrap, "config import");
    let enable = first_command_line(&bootstrap, "plugin enable");
    assert!(
        import < enable,
        "demo-bootstrap.sh runs `plugin enable` (line {}) before `config import` \
         (line {}); the importer will find zero taxonomy terms",
        enable + 1,
        import + 1
    );
}

#[test]
fn the_bootstrap_imports_ritrovos_own_configuration_set() {
    // The set used to come from the kernel image's tutorial directory. It is this
    // repository's now, and the demo has to be reading the copy that the model is
    // built in rather than the one the image still ships.
    let bootstrap = read("scripts/demo-bootstrap.sh");
    assert!(
        bootstrap.contains("RITROVO_CONFIG_DIR:-/ritrovo/demo/config"),
        "demo-bootstrap.sh does not default its config directory to demo/config"
    );
    assert!(
        !bootstrap.contains("/app/docs/tutorial/config"),
        "demo-bootstrap.sh still reads the config set out of the kernel image"
    );
    let compose = read(COMPOSE);
    assert!(
        compose.contains("RITROVO_CONFIG_DIR: /ritrovo/demo/config"),
        "{COMPOSE} does not point the kernel at demo/config"
    );
}

#[test]
fn every_routed_gather_has_a_template_of_its_own() {
    for query_id in ROUTED_GATHERS {
        let config: PathBuf = repo_root()
            .join("demo/config")
            .join(format!("gather_query.{query_id}.yml"));
        assert!(
            config.is_file(),
            "{} is routed but has no gather definition",
            config.display()
        );
        let template: PathBuf = repo_root()
            .join("docs/tutorial/templates/gather")
            .join(format!("query--{query_id}.html"));
        assert!(
            template.is_file(),
            "gather {query_id} has no template, so its route renders the kernel's \
             generic table: every column of every row, search_vector included"
        );
    }
}

#[test]
fn the_conference_type_declares_the_fields_the_templates_and_importer_use() {
    // The type, the templates and the importer have to agree on field names.
    // A rename that reaches two of the three is invisible until a page renders
    // blank, which is how field_venue_photo and field_photo survived a model
    // change in the first place.
    let conference = read("demo/config/item_type.conference.yml");
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
            conference.contains(field),
            "the conference type does not declare {field}"
        );
    }
    // Deliberately absent: the kernel has no category-reference field kind, and
    // declaring topics as anything else puts a widget on the edit form that
    // replaces the uuid array with a string on save.
    assert!(
        !conference.contains("field_name: field_topics"),
        "field_topics must not be declared; see the comment in the type file"
    );

    let speaker = read("demo/config/item_type.speaker.yml");
    for field in ["field_bio", "field_headshot", "field_website"] {
        assert!(
            speaker.contains(field),
            "the speaker type does not declare {field}"
        );
    }
    // The declaration, not the word: both type files explain in their comments
    // what they deliberately do NOT declare, and a bare substring match reads
    // those explanations as declarations.
    assert!(
        !speaker.contains("field_name: field_conferences"),
        "speaker must not hold a forward conference reference: the conference \
         holds field_speakers and the kernel computes the reverse"
    );
}

#[test]
fn ritrovo_overlay_wins_the_plugin_search_path() {
    let plugins_dir = compose_env("PLUGINS_DIR");
    let entries: Vec<&str> = plugins_dir.split(':').collect();
    assert!(
        entries.len() >= 2,
        "PLUGINS_DIR must append the Ritrovo overlay to the kernel's own \
         plugins dir, got {plugins_dir:?}"
    );
    assert_eq!(
        entries.first().copied(),
        Some("/app/plugins"),
        "the kernel's own plugins must come first in {plugins_dir:?}"
    );
    assert!(
        entries.last().unwrap().contains("overlay/plugins"),
        "the Ritrovo overlay must come last so it wins name collisions, \
         got {plugins_dir:?}"
    );
}

#[test]
fn tutorial_templates_win_the_template_search_path() {
    let templates_dir = compose_env("TEMPLATES_DIR");
    let entries: Vec<&str> = templates_dir.split(':').collect();
    assert_eq!(
        entries.first().copied(),
        Some("/app/templates"),
        "the kernel's own templates must come first in {templates_dir:?}"
    );
    assert!(
        entries.last().unwrap().contains("docs/tutorial/templates"),
        "the tutorial templates must come last so page--front.html and the \
         gather templates override the kernel's, got {templates_dir:?}"
    );
}

#[test]
fn pagefind_index_dir_is_first_and_is_not_a_read_only_mount() {
    // The first STATIC_DIR entry receives the generated Pagefind index, so it has
    // to be writable. Both other entries are read-only mounts; putting either
    // first makes the index build fail at cron time and nowhere earlier.
    let static_dir = compose_env("STATIC_DIR");
    let entries: Vec<&str> = static_dir.split(':').collect();
    let first = entries.first().copied().unwrap_or_default();
    assert!(
        first.contains("index"),
        "the first STATIC_DIR entry receives the Pagefind index and must be the \
         writable volume, got {static_dir:?}"
    );
    assert!(
        !first.starts_with("/app/") && !first.contains("/ritrovo/docs"),
        "the first STATIC_DIR entry is a read-only path: {first}"
    );
}

#[test]
fn demo_config_sets_the_front_page_to_the_conference_listing() {
    // Setting the front page is the last step of the install, and the only one
    // the tutorial config set does not carry. It rides in as its own one-file
    // config set so the demo needs no admin form and no SQL.
    let front_page = read("demo/config/variable.site_front_page.yml");
    assert!(
        front_page.contains("key: site_front_page"),
        "demo/config/variable.site_front_page.yml must set the \
         site_front_page variable, got:\n{front_page}"
    );
    assert!(
        front_page.contains("/conferences"),
        "the demo front page must be /conferences, got:\n{front_page}"
    );
}

#[test]
fn the_bootstrap_enables_the_plugin_that_builds_the_search_index() {
    // Without trovato_search nothing ever builds a Pagefind index, and /search is
    // blank in a browser however many results the server found: the page always
    // loads scolta.js, which clears the container when the index 404s
    // (G-SEARCH-PAGE-BLANK-WITHOUT-INDEX). The plugin ships in the image and
    // declares default_enabled = false, so the demo has to ask for it.
    //
    // It has to be asked for BEFORE the final serve, too: the kernel reads the
    // enabled set once when it builds its state, so enabling this on a running
    // server leaves the rebuild switched off until the next restart.
    let bootstrap = read("scripts/demo-bootstrap.sh");
    assert!(
        bootstrap.contains("trovato_search"),
        "scripts/demo-bootstrap.sh never enables trovato_search, so the demo's \
         search page has no index to load"
    );
    let enable = bootstrap
        .rfind("plugin enable")
        .expect("demo-bootstrap.sh never enables a plugin");
    let serve = bootstrap
        .rfind("exec ./trovato serve")
        .expect("demo-bootstrap.sh never execs the server");
    assert!(
        enable < serve,
        "demo-bootstrap.sh enables a plugin after the final serve, which the \
         kernel will not notice until the next restart"
    );
}

#[test]
fn the_demo_corrects_the_tutorials_call_for_papers_link() {
    // Trovato's tutorial set points the main menu's "Call for Papers" at
    // /open-cfps, which is not a route and not an alias: the gather is at /cfps.
    // The demo overrides the row by importing a same-uuid copy after the tutorial
    // set, so the id has to match and the path has to be the corrected one.
    let link = read("demo/config/menu_link.0193a5a0-0004-7000-8000-000000000003.yml");
    assert!(
        link.contains("id: 0193a5a0-0004-7000-8000-000000000003"),
        "the corrected menu link must carry the tutorial row's id, or it adds a \
         second link instead of replacing the broken one, got:\n{link}"
    );
    // The `path:` line, not the whole file: the comment above it names the broken
    // path on purpose, to say what is being corrected and why.
    let path_line = link
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("path:"))
        .unwrap_or_else(|| panic!("the corrected menu link declares no path:\n{link}"));
    assert_eq!(
        path_line, "path: /cfps",
        "the corrected menu link must point at /cfps, the gather's canonical url"
    );
}

#[test]
fn the_conference_template_renders_its_own_fields_and_not_the_kernels_dump() {
    // The kernel concatenates a generic `label: value` dump of every scalar field
    // and every plugin's tap_item_view output into one `children` string
    // (G-RENDER-CHILDREN-MIXES-FIELD-DUMP-AND-PLUGIN-OUTPUT). Rendering it printed
    // every conference field twice, internal ones included. This template renders
    // its fields in their own places instead, so it must not reach for `children`.
    let template = read("docs/tutorial/templates/elements/item--conference.html");
    assert!(
        !template.contains("{{ children"),
        "elements/item--conference.html renders `children`, which puts the \
         kernel's raw field dump back on the page"
    );
    for field in [
        "field_start_date",
        "field_end_date",
        "field_city",
        "field_country",
        "field_language",
    ] {
        assert!(
            template.contains(field),
            "elements/item--conference.html no longer renders {field}, which the \
             kernel's dump used to cover for"
        );
    }
}

#[test]
fn every_tutorial_template_the_demo_needs_is_vendored() {
    for name in TUTORIAL_TEMPLATES {
        let path: PathBuf = repo_root().join("docs/tutorial/templates").join(name);
        let body = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("missing vendored template {}: {e}", path.display()));
        assert!(
            !body.trim().is_empty(),
            "vendored template {} is empty",
            path.display()
        );
    }
}

#[test]
fn no_stray_files_in_the_vendored_template_tree() {
    // A file here that this list does not have would render in the demo and be
    // accounted for nowhere.
    let root = repo_root().join("docs/tutorial/templates");
    let mut found = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap().filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let relative = path
                    .strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                found.push(relative);
            }
        }
    }
    found.sort();
    let mut expected: Vec<String> = TUTORIAL_TEMPLATES
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    expected.sort();
    assert_eq!(found, expected);
}

#[test]
fn kernel_release_is_named_consistently() {
    let compose = read(COMPOSE);
    assert!(
        compose.contains(KERNEL_IMAGE),
        "{COMPOSE} must pin {KERNEL_IMAGE}"
    );
    for doc in ["README.md", "docs/INSTALL.md"] {
        let body = read(doc);
        assert!(
            body.contains("0.102.0"),
            "{doc} must name the kernel release the demo runs against"
        );
    }
    let checker = read("scripts/check-tutorial-templates.sh");
    assert!(
        checker.contains("v0.102.0"),
        "scripts/check-tutorial-templates.sh must diff the vendored templates \
         against the same release the demo runs"
    );
}

#[test]
fn demo_scripts_are_executable() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for script in [
            "scripts/serve-demo.sh",
            "scripts/demo-bootstrap.sh",
            "scripts/demo-cron.sh",
            "scripts/verify-demo.sh",
            "scripts/check-tutorial-templates.sh",
        ] {
            let path = repo_root().join(script);
            let mode = fs::metadata(&path)
                .unwrap_or_else(|e| panic!("missing {script}: {e}"))
                .permissions()
                .mode();
            assert!(
                mode & 0o111 != 0,
                "{script} is not executable; compose runs it directly"
            );
        }
    }
}

#[test]
fn compose_runs_the_bootstrap_and_the_cron_poker() {
    let compose = read(COMPOSE);
    for script in ["demo-bootstrap.sh", "demo-cron.sh"] {
        assert!(compose.contains(script), "{COMPOSE} never runs {script}");
    }
}
