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

#[cfg(test)]
mod tests {
    /// The SDK the plugins are compiled against and the kernel their
    /// host-in-the-loop suites run on must be the same revision of Trovato.
    ///
    /// They are two entries in the workspace manifest, and nothing else would
    /// notice them drifting apart. A module built against one contract and
    /// exercised on another kernel is a test that proves nothing about what
    /// ships: it would pass against host functions the released kernel no longer
    /// has, or fail against ones it does not have yet, and either way say nothing
    /// true about the pinned release.
    #[test]
    fn the_sdk_and_the_test_kernel_pin_the_same_trovato() {
        let manifest = include_str!("../../../Cargo.toml");
        let pin = |crate_name: &str| -> Option<String> {
            manifest
                .lines()
                .find(|line| line.starts_with(&format!("{crate_name} =")))
                .and_then(|line| line.split("rev = \"").nth(1))
                .and_then(|rest| rest.split('"').next())
                .map(str::to_string)
        };

        let sdk = pin("trovato-sdk");
        let kernel = pin("trovato-kernel");
        assert!(sdk.is_some(), "trovato-sdk has no pinned rev in Cargo.toml");
        assert!(
            kernel.is_some(),
            "trovato-kernel has no pinned rev in Cargo.toml"
        );
        assert_eq!(
            sdk, kernel,
            "trovato-sdk and trovato-kernel pin different Trovato revisions"
        );
    }
}
