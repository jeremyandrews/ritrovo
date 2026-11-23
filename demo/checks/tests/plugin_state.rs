//! `scripts/plugin-state.sh` reads a plugin's state off `trovato plugin list`,
//! and `verify-demo.sh` fails unless `trovato_ai` reads `enabled`.
//!
//! The list is a table: name, version, status ("enabled", "disabled" or "not
//! installed"), auto-enable, migrations. A substring test is not enough:
//! "disabled" contains "enabled", and a list that does not name the plugin at
//! all is a failure, not a reason to skip. Only no list at all, which is what a
//! run without the Compose stack gets, is "unknown".

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::Write;
use std::process::{Command, Stdio};

use ritrovo_demo_checks::repo_root;

const HEADER: &str = "PLUGIN               VERSION      STATUS         AUTO-ENABLE   MIGRATIONS\n\
-----------------------------------------------------------------------\n";

fn row(name: &str, status: &str) -> String {
    format!("{name:<20} {:<12} {status:<14} {:<13} 0\n", "1.0.0", "no")
}

fn state(list: &str) -> String {
    let mut child = Command::new("bash")
        .arg(repo_root().join("scripts/plugin-state.sh"))
        .arg("trovato_ai")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(list.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "plugin-state.sh failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .unwrap()
        .trim_end()
        .to_string()
}

#[test]
fn an_enabled_plugin_reads_enabled() {
    let list = format!(
        "{HEADER}{}{}",
        row("trovato_search", "enabled"),
        row("trovato_ai", "enabled")
    );
    assert_eq!(state(&list), "enabled");
}

#[test]
fn a_disabled_plugin_reads_disabled_not_enabled() {
    let list = format!(
        "{HEADER}{}{}",
        row("trovato_ai_extra", "enabled"),
        row("trovato_ai", "disabled")
    );
    assert_eq!(state(&list), "disabled");
}

#[test]
fn a_plugin_on_disk_but_never_installed_reads_not_installed() {
    let list = format!("{HEADER}{}", row("trovato_ai", "not installed"));
    assert_eq!(state(&list), "not installed");
}

#[test]
fn a_list_that_does_not_name_the_plugin_reads_absent() {
    let list = format!("{HEADER}{}", row("trovato_search", "enabled"));
    assert_eq!(state(&list), "absent");
    assert_eq!(state("No plugins found.\n"), "absent");
}

#[test]
fn no_list_at_all_reads_unknown() {
    assert_eq!(state(""), "unknown");
    assert_eq!(
        state("Error: could not connect to the database\n"),
        "unknown"
    );
}

/// `verify-demo.sh` reads the state through the script, and skips only on
/// "unknown".
#[test]
fn verify_demo_fails_unless_trovato_ai_reads_enabled() {
    let script = std::fs::read_to_string(repo_root().join("scripts/verify-demo.sh")).unwrap();
    let section = &script[script.find("head_ \"AI\"").expect("no AI section")..];
    let check = &section[..section.find("for path in").expect("no screen checks")];
    assert!(
        check.contains("scripts/plugin-state.sh\" trovato_ai"),
        "the AI section does not read trovato_ai's state through plugin-state.sh:\n{check}"
    );
    assert!(
        check.contains("enabled) ok")
            && check.contains("unknown) note")
            && check.contains("*) bad"),
        "the AI section must pass only on enabled, skip only on unknown and fail on the rest:\n{check}"
    );
    assert!(
        !check.contains("*enabled*"),
        "a substring test passes \"disabled\":\n{check}"
    );
}
