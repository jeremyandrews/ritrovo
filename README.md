# Ritrovo

Ritrovo is a conference management reference application built on [Trovato](https://github.com/jeremyandrews/trovato). It plays the role for Trovato that Umami plays for Drupal: a fully-functional demo installation that exercises real-world plugin composition and doubles as end-to-end documentation.

## Status

Extracted from the Trovato monorepo on 2026-04-14 via `git filter-repo`. History is preserved for all Ritrovo-specific commits.

**Builds standalone.** All five plugins compile to WebAssembly against the Trovato SDK as an external git dependency, with no Trovato checkout anywhere on disk. See "Building" below.

## Repository layout

```
scripts/
  assemble-overlay.sh   Stage built plugins into overlay/ for PLUGINS_DIR
plugins/
  ritrovo_access/       Editorial workflow / role-based access control
  ritrovo_cfp/          Call for Papers submission + review
  ritrovo_importer/     Content import tooling
  ritrovo_notify/       Email / notification system
  ritrovo_translate/    Multi-language content translation workflow
docs/
  ritrovo/              Architecture, epics, design docs
```

## Building

Ritrovo plugins compile to WebAssembly modules that Trovato loads at runtime.

**No Trovato checkout is required.** `trovato-sdk` is consumed as a git dependency
pinned by revision in the workspace `Cargo.toml`, so the entire build is:

```bash
git clone git@github.com:jeremyandrews/ritrovo.git
cd ritrovo
cargo build --target wasm32-wasip1 --release
```

Artifacts land in `target/wasm32-wasip1/release/ritrovo_*.wasm`. Install each
alongside its `*.info.toml` (and `migrations/`, where the plugin has them), or
run `scripts/assemble-overlay.sh` to stage all five into `overlay/plugins/` in
the layout `PLUGINS_DIR` expects. Installing into a Trovato instance is
[docs/INSTALL.md](docs/INSTALL.md) — note the API version blocker recorded
there.

### Prerequisites

- **Rust toolchain** — pinned by `rust-toolchain.toml` (1.96.0) and installed
  automatically by rustup, including the `wasm32-wasip1` target. Nothing to do
  by hand.
- **Access to the Trovato repository — currently a hard blocker for most
  readers.** This repository is public. The Trovato repository it depends on
  (`trovato-private`) is not, and `trovato-sdk` is published nowhere else: not on
  crates.io, not in a public mirror. The build below therefore works only for
  someone who already has access to a private repo, which is the opposite of what
  a public reference application is for.

  Nothing in Ritrovo can fix this — the SDK has to become publicly obtainable
  (crates.io, or a public repo carrying the SDK crates). Until then, treat these
  instructions as working-but-gated. When it is fixed, the change here is one
  line: the `trovato-sdk` entry in the workspace `Cargo.toml` becomes a version
  dependency instead of a git one.

  With access, a loaded `ssh-agent` key is enough. If cargo's built-in git client
  cannot use your key (agent forwarding, a hardware key, or a passphrase prompt),
  tell it to shell out to the system `git` instead, which honors your full SSH
  configuration:

  ```toml
  # ~/.cargo/config.toml
  [net]
  git-fetch-with-cli = true
  ```

  A failure here surfaces as `failed to authenticate when downloading repository`,
  not as anything Ritrovo-specific.

### Which SDK revision this builds against

The pin is a specific commit, not a branch: the PF-5 contract-freeze revision,
where the SDK crates read `1.0.0` and the kernel's `KERNEL_API_VERSION` reads
`(1, 0)`. That is the same commit `cargo-semver-checks` uses as its baseline in
Trovato CI, so "Ritrovo builds against the published contract" means something
checkable rather than "against whatever `main` happens to be today".

To build against a different contract revision, change `rev` in the
`[workspace.dependencies]` entry in the root `Cargo.toml`; the bump protocol is
documented there.

### Verifying a build

A plugin's `[capabilities] host_interfaces` must list exactly the
`trovato:kernel/<iface>` imports its compiled module actually has. The kernel
rejects the plugin at load time otherwise. Derive the list from the artifact, not
from reading the source — the SDK's tap macros generate host calls that no tap
body contains:

```bash
wasm-tools print target/wasm32-wasip1/release/ritrovo_notify.wasm \
  | grep '(import "trovato:kernel/'
```

## Next steps (fix-it phase)

- [x] Decide if `trovato-sdk` should be consumed by path (sibling checkout), git revision, or crates.io publish — **git revision**, pinned to the contract-freeze commit
- [x] Verify each plugin builds standalone
- [ ] Add CI (GitHub Actions) for `cargo check` + `cargo clippy` on the wasm target
- [x] Document the end-to-end "install Ritrovo on a fresh Trovato" walkthrough — [docs/INSTALL.md](docs/INSTALL.md)
- [ ] Decide: is Ritrovo a published product or a reference demo? The answer shapes release cadence, versioning, and public marketing

## License

GPL-2.0-or-later, matching Trovato.
