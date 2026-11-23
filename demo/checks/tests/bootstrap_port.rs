//! Nothing outside the kernel container reaches the kernel before Ritrovo's
//! configuration is in.
//!
//! `scripts/demo-bootstrap.sh` starts `trovato serve` twice before it imports
//! `demo/config`, to migrate and to register a permission. On the published
//! port, the cron poker (`scripts/demo-cron.sh`, which waits only for
//! `/health`) reached those servers and ran cron, which read the search settings
//! while `site_config.search` did not exist yet and cached the defaults for 300
//! seconds. `config import` writes the key without dropping the cached copy
//! (FRICTION.md, `G-CONFIG-IMPORT-LEAVES-SEARCH-SETTINGS-CACHED`), so the final
//! server answered with the search AI stages on for up to five minutes, against
//! `variable.search.yml`. With a key set, that is any visitor spending the
//! site's tokens.
//!
//! So `PORT` is a private port, which neither the cron container nor the host
//! can reach, from before the first server the bootstrap starts until just
//! before the final `exec ./trovato serve`, after every import. `SITE_URL` is
//! the final server's own default throughout, so every start agrees on it.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use ritrovo_demo_checks::repo_root;

fn bootstrap() -> String {
    std::fs::read_to_string(repo_root().join("scripts/demo-bootstrap.sh")).unwrap()
}

/// The script's lines with comments dropped.
fn code(script: &str) -> Vec<&str> {
    script
        .lines()
        .map(str::trim)
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .collect()
}

#[test]
fn the_bootstrap_servers_listen_on_a_private_port_and_only_the_last_on_port() {
    let script = bootstrap();
    let lines = code(&script);
    let at = |wanted: &str| {
        lines
            .iter()
            .position(|l| *l == wanted)
            .unwrap_or_else(|| panic!("demo-bootstrap.sh has no line `{wanted}`"))
    };
    let private = lines
        .iter()
        .find_map(|l| l.strip_prefix("BOOTSTRAP_PORT="))
        .expect("no BOOTSTRAP_PORT in demo-bootstrap.sh");
    assert!(
        !private.contains("3000") && !private.contains("PORT"),
        "BOOTSTRAP_PORT must differ from the published port: {private}"
    );
    at(r#"SERVE_PORT="${PORT:-3000}""#);
    at(r#"BASE="http://127.0.0.1:${BOOTSTRAP_PORT}""#);

    let serves: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.contains("./trovato serve"))
        .map(|(i, _)| i)
        .collect();
    let (&last, earlier) = serves.split_last().expect("no trovato serve at all");
    assert_eq!(
        lines[last], "exec ./trovato serve",
        "the final server is not the last start"
    );
    let (&first, _) = earlier.split_first().expect("no temporary server");
    let to_private = at(r#"export PORT="$BOOTSTRAP_PORT""#);
    let to_public = at(r#"export PORT="$SERVE_PORT""#);
    assert!(
        to_private < first,
        "the first temporary server starts before PORT is the private port"
    );
    for &serve in earlier {
        assert!(
            to_private < serve && serve < to_public,
            "a temporary server is not on the private port: {}",
            lines[serve]
        );
    }
    assert!(
        to_public < last,
        "the final server does not start on the published port"
    );
    assert!(
        lines[to_public + 1..last]
            .iter()
            .all(|l| !l.starts_with("export PORT=")),
        "PORT changes again between the switch back and the final server"
    );
}

#[test]
fn every_config_import_runs_before_the_final_server_starts() {
    let script = bootstrap();
    let lines = code(&script);
    let last_import = lines
        .iter()
        .rposition(|l| l.contains("./trovato config import"))
        .expect("no config import");
    let final_serve = lines
        .iter()
        .position(|l| *l == "exec ./trovato serve")
        .expect("no final serve");
    assert!(last_import < final_serve);
}

#[test]
fn the_final_site_url_is_set_before_any_server_starts() {
    let script = bootstrap();
    let lines = code(&script);
    let site_url = lines
        .iter()
        .position(|l| *l == r#"export SITE_URL="${SITE_URL:-http://localhost:${SERVE_PORT}}""#)
        .expect("SITE_URL is not defaulted to the final server's own default");
    let first_serve = lines
        .iter()
        .position(|l| l.contains("./trovato serve"))
        .unwrap();
    assert!(site_url < first_serve);
}
