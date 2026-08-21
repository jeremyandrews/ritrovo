# Ritrovo

Ritrovo is a conference management reference application built on [Trovato](https://github.com/jeremyandrews/trovato). It plays the role for Trovato that Umami plays for Drupal: a fully-functional demo installation that exercises real-world plugin composition and doubles as end-to-end documentation.

## Status

Extracted from the Trovato monorepo on 2026-04-14 via `git filter-repo`. History is preserved for all Ritrovo-specific commits.

**Builds standalone, from public sources.** All five plugins compile to WebAssembly against the Trovato SDK as an external git dependency on the public Trovato repository, with no Trovato checkout anywhere on disk and no credentials. See "Building" below.

**Runs standalone, on the released kernel.** `scripts/serve-demo.sh` stands the
whole site up against `ghcr.io/jeremyandrews/trovato:0.101.0` with Docker as the
only prerequisite. Verified against that release on 2026-08-21, from an empty
Docker to a checked site in 1m41s: all five plugins enabled, 269 import batches
drained, 5,644 conferences landed on that run (160 of them upcoming), the
importer's two admin screens answering 401 to an anonymous visitor and 200 to an
administrator, `/search?q=rust` returning 37 results, and `/it/conferenze`
rendering with `lang="it"`. The conference count moves run to run: confs.tech is
live data.

## The demo, in one command

```bash
git clone https://github.com/jeremyandrews/ritrovo.git
cd ritrovo
scripts/serve-demo.sh
```

Docker is the only prerequisite. No Trovato checkout, no Rust toolchain, no
credentials. When it finishes, <http://localhost:3000> is a populated conference
site: a filterable listing of upcoming conferences imported from confs.tech,
speaker pages, open calls for papers, search, the importer's two admin screens,
and the same listing in Italian.

What it stands up, in order, because the order is the interesting part:

| | |
|---|---|
| Postgres 16, Redis 7 | the kernel's two dependencies |
| the five Ritrovo plugins | compiled to WebAssembly in a throwaway `rust:1-bookworm` container, staged into an overlay volume |
| `ghcr.io/jeremyandrews/trovato:0.101.0` | the **released** kernel, unmodified, with the overlay appended to its three search paths |
| the installer | completed over HTTP, so nobody has to fill in a form |
| the tutorial config set | imported **before** the plugins are enabled, which is the trap: `ritrovo_importer` resolves the topic taxonomy once, in `tap_install` |
| the five plugins | enabled, then a restart, which is when `tap_install` fires and the conference import begins |
| a cron poker | the kernel has no scheduler, so something has to `POST /cron/<key>` until the import queue drains |
| the Italian seed content | Part 7's content-translation set |
| the front page | pointed at `/conferences` |

`scripts/serve-demo.sh` is a wrapper that waits for all of that and then checks
it. The demo itself is the compose file, so this works too, it just does not tell
you when it has finished:

```bash
docker compose -f docker-compose.demo.yml up
```

To check a running demo, or to see what it found:

```bash
scripts/verify-demo.sh          # plugin status, conference counts, 401/200, search, Italian
scripts/serve-demo.sh --down    # stop it and delete its volumes
```

The full walkthrough, including installing Ritrovo onto a Trovato you built
yourself, is [docs/INSTALL.md](docs/INSTALL.md).

## Repository layout

```
docker-compose.demo.yml   The one-command demo: Postgres, Redis, the released
                          kernel image, a plugin builder, a cron poker
scripts/
  serve-demo.sh           Bring the demo up, wait for the import, check it
  demo-bootstrap.sh       The install recipe, run inside the kernel container
  demo-cron.sh            The cron poker: no scheduler lives in the kernel
  verify-demo.sh          What a populated site looks like, asserted over HTTP
  assemble-overlay.sh     Stage built plugins into overlay/ for PLUGINS_DIR
  check-tutorial-templates.sh
                          Diff the vendored templates against the release
plugins/
  ritrovo_access/         Editorial workflow / role-based access control
  ritrovo_cfp/            Call for Papers submission + review
  ritrovo_importer/       Content import tooling
  ritrovo_notify/         Email / notification system
  ritrovo_translate/      Multi-language content translation workflow
demo/
  config/                 The two pieces of config the tutorial set does not
                          carry: the front page and the Italian aliases
  checks/                 Tests pinning the demo's wiring to the repository
docs/
  INSTALL.md              The demo, then the manual walkthrough
  ritrovo/                Architecture, epics, design docs
  tutorial/templates/     Trovato's tutorial templates, vendored for the demo
  tutorial/static/        ritrovo.css
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
- [x] Add CI (GitHub Actions) for `cargo check` + `cargo clippy` on the wasm target — `.github/workflows/ci.yml`, which also gates the demo's wiring
- [x] A one-command demo on the released kernel, standing up unattended — `docker-compose.demo.yml`
- [x] Document the end-to-end "install Ritrovo on a fresh Trovato" walkthrough — [docs/INSTALL.md](docs/INSTALL.md)
- [ ] Decide: is Ritrovo a published product or a reference demo? The answer shapes release cadence, versioning, and public marketing

## License

GPL-2.0-or-later, matching Trovato.
