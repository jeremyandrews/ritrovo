# Ritrovo

Ritrovo is a conference management reference application built on [Trovato](https://github.com/jeremyandrews/trovato). It plays the role for Trovato that Umami plays for Drupal: a fully-functional demo installation that exercises real-world plugin composition and doubles as end-to-end documentation.

## Status

Extracted from the Trovato monorepo on 2026-04-14 via `git filter-repo`. History is preserved for all Ritrovo-specific commits.

**Build is not yet wired up** — the plugins reference `trovato-sdk` via a path dependency that assumes a sibling Trovato checkout. See "Building" below.

## Repository layout

```
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

Expected directory layout:

```
<parent>/
  Trovato/trovato/          # Core Trovato repo (provides trovato-sdk)
  Ritrovo/                  # This repo
```

With both checked out as siblings, from this repo:

```bash
cargo build --target wasm32-wasip1 --release
```

If your Trovato checkout lives elsewhere, override the path in a local `.cargo/config.toml`:

```toml
[patch."path+file:///..."]
trovato-sdk = { path = "/absolute/path/to/trovato/crates/plugin-sdk" }
```

A proper git-based dependency on a versioned Trovato release is the eventual answer.

## Next steps (fix-it phase)

- [ ] Decide if `trovato-sdk` should be consumed by path (sibling checkout), git revision, or crates.io publish
- [ ] Verify each plugin builds standalone
- [ ] Add CI (GitHub Actions) for `cargo check` + `cargo clippy` on the wasm target
- [ ] Document the end-to-end "install Ritrovo on a fresh Trovato" walkthrough
- [ ] Decide: is Ritrovo a published product or a reference demo? The answer shapes release cadence, versioning, and public marketing

## License

GPL-2.0-or-later, matching Trovato.
