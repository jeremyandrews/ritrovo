# Ritrovo

Ritrovo is a conference management reference application built on [Trovato](https://github.com/jeremyandrews/trovato). It plays the role for Trovato that Umami plays for Drupal: a fully-functional demo installation that exercises real-world plugin composition and doubles as end-to-end documentation.

## Status

Extracted from the Trovato monorepo on 2026-04-14 via `git filter-repo`. History is preserved for all Ritrovo-specific commits.

**Builds standalone, from public sources.** All five plugins compile to WebAssembly against the Trovato SDK as an external git dependency on the public Trovato repository, with no Trovato checkout anywhere on disk and no credentials. See "Building" below.

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
[docs/INSTALL.md](docs/INSTALL.md).

### Prerequisites

- **Rust toolchain** — pinned by `rust-toolchain.toml` (1.96.0) and installed
  automatically by rustup, including the `wasm32-wasip1` target. Nothing to do
  by hand.
- **Nothing else.** No credentials, no private repository, no Trovato checkout.
  `trovato-sdk` comes from the public
  [Trovato repository](https://github.com/jeremyandrews/trovato) over HTTPS, so
  `git clone` and `cargo build` is the whole story.

  This was not true until recently: the `trovato-sdk` dependency pointed at an
  unpublished development repository, so the build worked only for someone who
  already had access to a private repo, which is the opposite of what a public
  reference application is for. That repository is archived and is not the repo
  of record.

### Which SDK revision this builds against

The pin is a specific commit, not a branch, so "Ritrovo builds against the
published contract" names a contract rather than whatever `main` happens to be
today:

| | |
|---|---|
| `rev` | `50c46ee` (`jeremyandrews/trovato`, `main`) |
| SDK crate version | 0.99.0 |
| `KERNEL_API_VERSION` | (0, 99) |

All three agree, and the plugin manifests declare `api_version = "0.99"` to
match. Under the old pin they did not: that SDK crate labelled itself `1.0.0`
ahead of the kernel Trovato ships, and manifests copied from it were rejected at
enable time with `requires API 1.0 but kernel provides API 0.99`.

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
