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
const KERNEL_IMAGE: &str = "ghcr.io/jeremyandrews/trovato:0.101.0";

/// The compose file the demo is driven from.
const COMPOSE: &str = "docker-compose.demo.yml";

/// The tutorial templates the demo needs on its `TEMPLATES_DIR` search path.
/// Vendored from the Trovato release; see `scripts/check-tutorial-templates.sh`.
const TUTORIAL_TEMPLATES: &[&str] = &[
    "page--front.html",
    "elements/item--conference.html",
    "elements/item--speaker.html",
    "gather/includes/conf-card.html",
    "gather/query--ritrovo.all_speakers.html",
    "gather/query--ritrovo.by_topic.html",
    "gather/query--ritrovo.open_cfps.html",
    "gather/query--ritrovo.upcoming_conferences.html",
    "gather/query--upcoming_conferences.html",
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

#[test]
fn config_is_imported_before_any_plugin_is_enabled() {
    // The install-order trap: ritrovo_importer resolves the topic taxonomy once,
    // in tap_install, and tap_install fires when the plugin is first enabled and
    // the server restarts. Enable it before the taxonomy exists and it imports
    // every conference untagged. Line order in the script is the fix, so line
    // order is what this pins.
    let bootstrap = read("scripts/demo-bootstrap.sh");
    let import = bootstrap
        .find("config import")
        .expect("demo-bootstrap.sh never imports the config set");
    let enable = bootstrap
        .find("plugin enable")
        .expect("demo-bootstrap.sh never enables a plugin");
    assert!(
        import < enable,
        "demo-bootstrap.sh enables a plugin before importing the config set; \
         the importer will find zero taxonomy terms"
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
    // A file here that the release does not have is drift in the other
    // direction: it would render in the demo and nowhere else.
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
            body.contains("0.101.0"),
            "{doc} must name the kernel release the demo runs against"
        );
    }
    let checker = read("scripts/check-tutorial-templates.sh");
    assert!(
        checker.contains("v0.101.0"),
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
