# Changelog

## Unreleased

- The pinned Trovato release is authored once, in `kernel-release.toml`, and written into the eleven places that repeat it by `scripts/sync-kernel-release.sh`; `--set-version X.Y.Z` resolves the tag against the public Trovato repository first. `demo/checks/tests/demo_wiring.rs` no longer carries a copy of the release and checks all eleven instead, including the five plugin manifests and both git `rev`s, which nothing checked before. Bumping the kernel is one command and a diff review.
- `ritrovo_cfp` 1.1.0: the CFP badge counted down from the item's last save (`changed`), not today; it now reads the database clock. Also parses the importer's `YYYY-MM-DD` deadlines, which previously showed no badge.
- `ritrovo_importer` 1.2.0: updating a conference wrote an empty `field_source_id`, so the next import could not find it and inserted a duplicate; the key is now written back, and migration 004 restores blanked keys and merges the duplicates.
- `scripts/verify-demo.sh`: the Italian check piped `curl` into `grep -q` under `pipefail`, so grep's early exit could SIGPIPE curl and fail a page that passed; it now matches the captured page.
- `ritrovo_notify` 1.1.0: removed the `/user/subscriptions` route that 404ed, the Subscribe button that did nothing, the queue nothing drained and the permissions nothing checked; its README describes the plugin still to be built.
