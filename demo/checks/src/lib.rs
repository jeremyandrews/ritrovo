//! Consistency checks for the one-command Ritrovo demo.
//!
//! The demo is shell and YAML: `docker-compose.demo.yml` plus the scripts under
//! `scripts/`. None of that is compiled, so nothing normally catches the failure
//! mode this crate exists for — the demo silently drifting out of step with the
//! repository it demonstrates. A sixth plugin added under `plugins/` that nobody
//! adds to the enable list does not break the build, does not break the tests,
//! and does not error at run time: the demo just quietly stops showing it.
//!
//! So the wiring is asserted instead. The tests live in `tests/demo_wiring.rs`
//! and read the demo's own files off disk, which is why this library has no
//! code: it exists to give those tests a package to hang off.
//!
//! Every path is resolved from [`repo_root`] rather than the process working
//! directory, so `cargo test` finds the same files from anywhere.

use std::path::{Path, PathBuf};

/// Absolute path to the repository root.
///
/// `CARGO_MANIFEST_DIR` is `<root>/demo/checks`, so the root is two levels up.
/// Resolved at compile time from the manifest location, never from the current
/// directory, which `cargo test` does not promise.
#[must_use]
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}
