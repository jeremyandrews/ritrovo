# Installing Ritrovo on a fresh Trovato

Ritrovo installs as a pure overlay. Nothing is copied into the Trovato checkout
and nothing in it is edited: the kernel's three search-path settings
(`PLUGINS_DIR`, `TEMPLATES_DIR`, `STATIC_DIR`) each take a colon-separated list
where later entries win, so Ritrovo's plugins are simply appended to the end.

## API version

Each plugin declares `api_version = "0.99"` in its `.info.toml`, matching the
released kernel's `KERNEL_API_VERSION` of `(0, 99)`. That is the version the
plugins genuinely conform to: every host interface they import (`logging`,
`db`, `http`, `queue`) is present and identical in the `0.99` contract. The
value lives in the manifest, not the compiled `.wasm`, and the kernel reads it
at install time — so it is a plain declaration, not something baked in at build.

(Earlier revisions of these manifests declared `1.0`, copied from the SDK's
contract-freeze crate, which labels itself `1.0.0` ahead of the released
kernel. That mismatch made the kernel reject every plugin at enable time with
`requires API 1.0 but kernel provides API 0.99`. Declaring the released version
is the correct fix. Realigning the SDK crate's own version to the release it
ships against is a separate, Trovato-side cleanup that does not affect this
install.)

**One feature is not available on `0.99`:** the importer's admin screens
(`/admin/content/conferences`, `/admin/config/importer`). They are registered
through `tap_menu().callback()`, which the released kernel dispatches via
`tap-api` — an export added after `0.99`. On `0.99` those two paths return 404;
the importer's cron, queue and install taps run normally, so imports still work.
Everything else in this walkthrough — the public site, browsing, search, config
import, editorial stages — runs fully. The walkthrough below has been executed
end to end.

## Prerequisites

PostgreSQL 15+, Redis 7+, a Rust toolchain, and SSH access to the private
Trovato repository (see [Building](../README.md#building) — the SDK is a git
dependency and is published nowhere public).

## Steps

```bash
# 1. Build the plugins and stage them where a search path can reach them.
cd ritrovo
cargo build --target wasm32-wasip1 --release
scripts/assemble-overlay.sh                    # -> overlay/plugins/<name>/

# 2. Build the kernel from a Trovato checkout you never modify.
cd ../trovato
cargo build --release --bin trovato

# 3. Point the three search paths at both trees. Ritrovo comes last so it wins
#    name collisions. The FIRST static dir receives the generated Pagefind
#    index, so it must be writable and must not be inside the Trovato checkout.
export DATABASE_URL="postgres://trovato:trovato@localhost:5432/ritrovo"
export REDIS_URL="redis://127.0.0.1:6379/0"
export CRON_KEY="change-me"
export PLUGINS_DIR="$PWD/plugins:$RITROVO/overlay/plugins"
export TEMPLATES_DIR="$PWD/templates:$PWD/docs/tutorial/templates"
export STATIC_DIR="/var/tmp/ritrovo-index:$PWD/static:$RITROVO/docs/tutorial/static"
export UPLOADS_DIR="/var/tmp/ritrovo-uploads"

# 4. Start it, then complete the web installer at http://localhost:3000
#    (admin password minimum is 12 characters, not the 8 INSTALL.md claims).
./target/release/trovato

# 5. Import the config set BEFORE enabling the plugins — see the note below.
./target/release/trovato config import docs/tutorial/config --dry-run
./target/release/trovato config import docs/tutorial/config

# 6. Enable the five plugins. They install automatically but stay disabled,
#    because each declares default_enabled = false.
for p in ritrovo_importer ritrovo_access ritrovo_cfp ritrovo_notify ritrovo_translate; do
    ./target/release/trovato plugin enable $p
done

# 7. Restart, then drain the import queue. There is no internal scheduler, and
#    the worker handles 100 batches per run, so one cron call is not enough:
#    repeat until the queue is empty or the site has no upcoming conferences.
curl -X POST http://localhost:3000/cron/$CRON_KEY

# 8. Optional: Italian seed content, imported as its own set.
./target/release/trovato config import docs/tutorial/config/seed-italian
```

Finally, set the front page to `/conferences` at `/admin/config/site`. A
non-item front path redirects rather than rendering inline, which is the
documented behaviour.

**Ordering matters.** `ritrovo_importer` resolves the topic taxonomy once, in
`tap_install`. Enable it before importing the config and it finds 0 of 23 terms
and imports everything untagged. If that happens, the plugin says so in the log;
recover with `UPDATE plugin_status SET tap_install_called = FALSE WHERE name =
'ritrovo_importer';` and restart.

## Templates and static assets

The tutorial templates are referenced in place from the Trovato checkout rather
than copied into the overlay: they are Trovato's own tutorial assets, they layer
over the kernel's `templates/` by the same search-path mechanism, and copying
them would mean keeping a duplicate in sync for no gain. Only `ritrovo.css`
lives here, and it is likewise referenced in place from
`docs/tutorial/static`. So the overlay contains plugins and nothing else.
