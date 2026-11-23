//! `scripts/sync-kernel-release.sh --set-nightly` pins a commit only when every
//! platform of the image says it was built from that commit.
//!
//! The script reads the commit out of the image's build provenance. Here it runs
//! against a stub `docker` that answers `buildx imagetools inspect` the way the
//! real one does for a multi-platform image, `{}` for a platform the image does
//! not have, and stub `curl` and `git` that fail, so nothing reaches a network.
//! The script runs from a copy in a directory of its own under the system temp
//! directory, so a run that got further than it should could only ever rewrite
//! the copy's `kernel-release.toml`; each test removes the directory again.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output};

use ritrovo_demo_checks::repo_root;

const AMD64: &str = "1111111111111111111111111111111111111111";
const OTHER: &str = "2222222222222222222222222222222222222222";

/// `docker buildx imagetools inspect REF --format FMT`. The whole provenance
/// document for `{{json .Provenance}}`, one platform's for an `index` into it.
/// `STUB_ARM64` is that platform's commit, or empty for an image without it.
const DOCKER_STUB: &str = r#"#!/bin/sh
format=""
while [ $# -gt 0 ]; do
    [ "$1" = "--format" ] && { shift; format="$1"; }
    shift
done
amd64="{\"SLSA\":{\"metadata\":{\"buildkit_metadata\":{\"vcs:revision\":\"$STUB_AMD64\"}}}}"
if [ -n "$STUB_ARM64" ]; then
    arm64="{\"SLSA\":{\"metadata\":{\"buildkit_metadata\":{\"vcs:revision\":\"$STUB_ARM64\"}}}}"
    all="{\"linux/amd64\":$amd64,\"linux/arm64\":$arm64}"
else
    arm64="{}"
    all="{\"linux/amd64\":$amd64}"
fi
case "$format" in
    *linux/amd64*) printf '%s\n' "$amd64" ;;
    *linux/arm64*) printf '%s\n' "$arm64" ;;
    *linux/*) printf '{}\n' ;;
    *) printf '%s\n' "$all" ;;
esac
"#;

const FAIL_STUB: &str = "#!/bin/sh\nexit 1\n";

struct Copy(PathBuf);

impl Copy {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "ritrovo-nightly-provenance-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("scripts")).unwrap();
        fs::create_dir_all(dir.join("bin")).unwrap();
        let root = repo_root();
        fs::copy(
            root.join("scripts/sync-kernel-release.sh"),
            dir.join("scripts/sync-kernel-release.sh"),
        )
        .unwrap();
        fs::copy(
            root.join("kernel-release.toml"),
            dir.join("kernel-release.toml"),
        )
        .unwrap();
        for (name, body) in [
            ("docker", DOCKER_STUB),
            ("curl", FAIL_STUB),
            ("git", FAIL_STUB),
        ] {
            let path = dir.join("bin").join(name);
            fs::write(&path, body).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        Self(dir)
    }

    fn set_nightly(&self, arm64: &str) -> Output {
        let path = format!(
            "{}:{}",
            self.0.join("bin").display(),
            std::env::var("PATH").unwrap()
        );
        Command::new("bash")
            .arg(self.0.join("scripts/sync-kernel-release.sh"))
            .args(["--set-nightly", "main"])
            .current_dir(&self.0)
            .env("PATH", path)
            .env("STUB_AMD64", AMD64)
            .env("STUB_ARM64", arm64)
            .output()
            .unwrap()
    }

    fn contract(&self) -> String {
        fs::read_to_string(self.0.join("kernel-release.toml")).unwrap()
    }
}

impl Drop for Copy {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn an_image_with_no_arm64_provenance_is_refused() {
    let copy = Copy::new("missing");
    let before = copy.contract();
    let output = copy.set_nightly("");
    assert!(
        !output.status.success(),
        "pinned an image with no arm64 provenance: {}",
        text(&output)
    );
    assert!(
        text(&output).contains("linux/arm64 provenance"),
        "refused, but not for the missing platform: {}",
        text(&output)
    );
    assert_eq!(
        copy.contract(),
        before,
        "the refused pin rewrote the contract"
    );
}

#[test]
fn an_image_whose_platforms_name_different_commits_is_refused() {
    let copy = Copy::new("split");
    let before = copy.contract();
    let output = copy.set_nightly(OTHER);
    assert!(!output.status.success(), "{}", text(&output));
    assert!(
        text(&output).contains(&format!(
            "{AMD64} for one platform and {OTHER} for linux/arm64"
        )),
        "{}",
        text(&output)
    );
    assert_eq!(copy.contract(), before);
}

#[test]
fn an_image_whose_platforms_agree_resolves_to_that_commit() {
    let copy = Copy::new("agree");
    let output = copy.set_nightly(AMD64);
    // It goes on to read the kernel's Cargo.toml at that commit, which the
    // stub curl refuses; getting that far is the resolution succeeding.
    assert!(
        text(&output).contains(&format!("nightly-1111111 was built from {AMD64}")),
        "{}",
        text(&output)
    );
    assert!(
        text(&output).contains(&format!("cannot read Trovato's Cargo.toml at {AMD64}")),
        "{}",
        text(&output)
    );
}
