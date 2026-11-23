//! `scripts/open-issue-number.sh` finds the kernel nightly's one issue however
//! many issues are open.
//!
//! A stub `gh` stands in for `gh api`: with `--paginate` it prints every page,
//! one JSON array after another as the real one does, and without it only the
//! first. The issue is on the last page, behind a pull request with the same
//! title, which the issues endpoint returns too. The stub lives in a directory
//! of its own under the system temp directory, removed again by each test.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output};

use ritrovo_demo_checks::repo_root;

const TITLE: &str = "Kernel main breaks Ritrovo";

const GH_STUB: &str = r#"#!/bin/sh
[ "$1" = api ] || { echo "unexpected: $*" >&2; exit 2; }
paginate=0
for arg in "$@"; do
    [ "$arg" = --paginate ] && paginate=1
    case "$arg" in repos/*) echo "$arg" > "$STUB_DIR/endpoint" ;; esac
done
page() {
    from=$1 to=$2 n=$1 sep=""
    printf '['
    while [ "$n" -le "$to" ]; do
        printf '%s{"number":%s,"title":"Something else %s"}' "$sep" "$n" "$n"
        sep=","
        n=$((n + 1))
    done
    [ -n "$3" ] && printf ',%s' "$3"
    printf ']\n'
}
page 1 99 "{\"number\":100,\"title\":\"$STUB_TITLE\",\"pull_request\":{\"url\":\"x\"}}"
if [ "$paginate" = 1 ]; then
    page 101 199 ""
    page 200 249 "$STUB_LAST"
fi
"#;

struct Stub(PathBuf);

impl Stub {
    fn new(tag: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("ritrovo-open-issue-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("bin")).unwrap();
        let gh = dir.join("bin/gh");
        fs::write(&gh, GH_STUB).unwrap();
        fs::set_permissions(&gh, fs::Permissions::from_mode(0o755)).unwrap();
        Self(dir)
    }

    fn run(&self, last: &str) -> Output {
        let path = format!(
            "{}:{}",
            self.0.join("bin").display(),
            std::env::var("PATH").unwrap()
        );
        Command::new("bash")
            .arg(repo_root().join("scripts/open-issue-number.sh"))
            .arg(TITLE)
            .env("PATH", path)
            .env("STUB_DIR", &self.0)
            .env("STUB_TITLE", TITLE)
            .env("STUB_LAST", last)
            .env("GH_REPO", "owner/repo")
            .output()
            .unwrap()
    }
}

impl Drop for Stub {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn the_issue_is_found_on_the_last_page_and_a_pull_request_is_not_it() {
    let stub = Stub::new("found");
    let output = stub.run(&format!(r#"{{"number":250,"title":"{TITLE}"}}"#));
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "250\n");
    assert_eq!(
        fs::read_to_string(stub.0.join("endpoint")).unwrap().trim(),
        "repos/owner/repo/issues?state=open&per_page=100"
    );
}

#[test]
fn no_open_issue_prints_nothing_and_succeeds() {
    let stub = Stub::new("none");
    let output = stub.run("");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
}
