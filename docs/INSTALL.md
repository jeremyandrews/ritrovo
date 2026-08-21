# Installing Ritrovo

Two paths. The first needs Docker and five minutes. The second is the one a
developer wants, and it is the same steps with the automation taken off.

Ritrovo installs as a pure overlay either way. Nothing is copied into Trovato and
nothing in it is edited: the kernel's three search-path settings (`PLUGINS_DIR`,
`TEMPLATES_DIR`, `STATIC_DIR`) each take a colon-separated list where later
entries win, so Ritrovo is simply appended to the end.

## The one-command demo

```bash
scripts/serve-demo.sh
```

That brings up Postgres and Redis, compiles the five plugins to WebAssembly in a
throwaway container, starts the **released** kernel image
(`ghcr.io/jeremyandrews/trovato:0.101.0`) with the Ritrovo overlay on its search
paths, completes the installer over HTTP, imports the tutorial config set, enables
the plugins, drains the conference import queue, loads the Italian seed content,
points the front page at `/conferences`, and then checks the result and prints
what it saw. `http://localhost:3000` is a populated site when it returns.

The demo is the compose file, so this reaches the same site on its own — it just
does not report when it has finished:

```bash
docker compose -f docker-compose.demo.yml up
```

| | |
|---|---|
| check a running demo | `scripts/verify-demo.sh` |
| logs | `docker compose -f docker-compose.demo.yml logs -f trovato` |
| the cron poker's progress | `docker compose -f docker-compose.demo.yml logs -f cron` |
| start over | `scripts/serve-demo.sh --fresh` |
| stop, and delete the volumes | `scripts/serve-demo.sh --down` |

Defaults worth knowing: the admin account is `admin` / `ritrovo-demo-password`
(the kernel's minimum is 12 characters, not the 8 an older draft of this file
claimed), the host port is 3000, and `CRON_KEY` is `ritrovo-demo-cron-key`. All
three are environment variables; see the header of `docker-compose.demo.yml`.

### What takes the time

`ritrovo_importer`'s `tap_install` fetches every confs.tech topic/year file from
2015 to the current year and pushes each as a queue batch. That is a few hundred
HTTP requests, and the kernel drains at most 100 queue items per cron call, so
the import is minutes rather than seconds. The site is browsable throughout; it
just has fewer conferences in it than it will have. `serve-demo.sh` waits for the
`cron` service to report the queue drained before it runs its checks.

### Two harmless noises in the log

* `no .info.toml file found, skipping dir=/app/plugins/ritrovo_importer`. The
  released image creates that directory and puts nothing in it: Trovato's own
  repository commits `plugins/ritrovo_importer/ritrovo_importer.wasm` but no
  manifest, and its Dockerfile makes a directory per plugin source directory. An
  entry with no manifest cannot be discovered, so the kernel skips it and finds
  the real one later on the search path, in the Ritrovo overlay. Nothing to do.
* `menu entry declares a callback but handler_type is not "api" … plugin=ritrovo_notify path=/user/subscriptions`.
  Real, and Ritrovo's own: `ritrovo_notify` registers that path with
  `MenuDefinition`, which leaves `handler_type` at `"page"`, and exports no
  `tap_api`, so the path 404s. It is the same defect the importer's two admin
  screens had before they were fixed, still outstanding for this one route.

## API version

Each plugin declares `api_version = "0.99"` in its `.info.toml`, matching the
`KERNEL_API_VERSION` of `(0, 99)` the SDK is pinned to. That is the version the
plugins genuinely conform to: every host interface they import (`logging`,
`db`, `http`, `queue`) is present and identical in the `0.99` contract. The
value lives in the manifest, not the compiled `.wasm`, and the kernel reads it
at install time — so it is a plain declaration, not something baked in at build.

**`0.99` plugins run on the `0.101` kernel unchanged, and no bump was needed.**
The kernel accepts a plugin when the plugin's major equals the kernel's and the
plugin's minor is less than or equal to the kernel's, so `0.99 <= 0.101` passes.
All five plugins install and enable on `ghcr.io/jeremyandrews/trovato:0.101.0`
with these manifests exactly as committed — verified, not assumed. A newer
kernel is never the reason to bump this number; a host interface the plugins
import changing shape would be.

(Earlier revisions of these manifests declared `1.0`, copied from an SDK crate
that labelled itself `1.0.0` ahead of the released kernel. That mismatch made the
kernel reject every plugin at enable time with `requires API 1.0 but kernel
provides API 0.99`. Declaring the released version was the correct fix, and the
mismatch is gone at the source too: the SDK is now pinned at a commit of the
public Trovato repository where the crate version, the manifests and
`KERNEL_API_VERSION` all read 0.99.)

**Every feature is available.** The importer's two admin screens
(`/admin/content/conferences`, `/admin/config/importer`) used to 404, and the
caveat that lived here blamed the kernel: it said `tap-api` was "an export added
after 0.99". That was wrong on both counts. `tap-api` shipped *in* 0.99.0, and
the reason those paths 404ed was on this side of the boundary — the entries were
built with `MenuDefinition`, which leaves `handler_type` at `"page"`, and the
plugin exported no `tap_api` at all. The kernel routes a request to `tap_api`
only for an entry whose `handler_type` is `"api"`, so it had nothing to dispatch
to and correctly served a 404. Both are fixed: the entries are `MenuRoute::api`
and the plugin serves them. The walkthrough below has been executed end to end,
most recently against the released `0.101.0` kernel on 2026-08-21.

## Installing by hand

The demo above is this section, automated. Follow it when you are working on
Ritrovo itself, or against a Trovato you built rather than the released image.

### Prerequisites

PostgreSQL 15+, Redis 7+, and a Rust toolchain. No credentials and no private
repository: the SDK is a git dependency on the public Trovato repository (see
[Building](../README.md#building)).

### Steps

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
export TEMPLATES_DIR="$PWD/templates:$RITROVO/docs/tutorial/templates"
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

# 8. Italian seed content, imported as its own set.
./target/release/trovato config import docs/tutorial/config/seed-italian

# 9. Ritrovo's own two-file config set: the front page, and the Italian aliases
#    stored the way the language middleware asks for them. See demo/config/README.md.
./target/release/trovato config import $RITROVO/demo/config
```

Step 9 is what the tutorial does by hand at `/admin/config/site`. Either way, a
non-item front path redirects rather than rendering inline, which is the
documented behaviour, so `/` answers 307 to `/conferences`.

### Checking the install

Log in as the admin you created and open the importer's two screens:

- `/admin/content/conferences` — what the importer has landed, newest first.
- `/admin/config/importer` — the importer's own state: last run, which topics
  the next cron cycle takes, how many ETags are cached, how many topic terms
  resolved, and how many jobs are waiting in the queue.

Both are read-only operator views. Both must return 200 for an administrator and
401 for an anonymous visitor; the kernel checks each entry's declared permission
(`view conference content` and `administer conference import`) before the plugin
sees the request.

Also worth opening: `/conferences` (the front page redirects here), `/speakers`,
`/cfps`, `/search?q=rust`, and `/it/conferenze` — the Italian listing, which
renders with `lang="it"`. `scripts/verify-demo.sh` checks all of these against a
running site and prints what it found, including the conference counts, so it is
worth running by hand after a manual install too.

**Ordering matters.** `ritrovo_importer` resolves the topic taxonomy once, in
`tap_install`. Enable it before importing the config and it finds 0 of 23 terms
and imports everything untagged. If that happens, the plugin says so in the log;
recover with `UPDATE plugin_status SET tap_install_called = FALSE WHERE name =
'ritrovo_importer';` and restart. On a correct run the log says
`discover_taxonomy_uuids: 23/23 terms found`, which is the line to grep for.

**The Italian URLs need Ritrovo's own aliases.** Trovato's tutorial config set
declares them with the language prefix included (`/it/conferenze`), but the
kernel's language middleware strips `/it/` before the alias lookup runs, so those
rows can never match and the paths 404. `demo/config/` carries the same three
aliases stored prefix-free, which is what the middleware asks for; step 9 above
imports them. The reasoning is in `demo/config/README.md`.

## Templates and static assets

Nine template files under `docs/tutorial/templates` are Trovato's, not Ritrovo's:
`page--front.html`, two item templates, and the six gather templates that give
the conference and speaker listings their card layout. They are the tutorial's
own assets and they layer over the kernel's `templates/` by the same search-path
mechanism as everything else. `ritrovo.css` under `docs/tutorial/static` is the
only asset this repository authored.

**They are vendored here, copied from the release.** This is the one thing the
container demo forced a decision on, so the reasoning is written down rather than
left in the compose file.

The released image ships `templates/`, `static/` and `docs/tutorial/config/`, and
stops there — `docs/tutorial/templates` is not in it. A demo that runs on the
image alone therefore has to get those nine files from somewhere, and there were
three somewheres:

| | |
|---|---|
| mount them from a Trovato checkout | rejected: it reintroduces the checkout the demo exists to do without |
| fetch the release tarball at start-up | rejected: it adds a network failure mode between a stranger and their first look, and the demo already has to pull four images |
| copy them into this repository | chosen |

Earlier revisions of this file argued the opposite — that referencing them in
place from a Trovato checkout beat "keeping a duplicate in sync for no gain". The
gain is what changed: a stranger with only Docker can now see Ritrovo work. The
cost is the copy going stale, and it is paid rather than hoped away:
`scripts/check-tutorial-templates.sh` diffs the vendored files against
`docs/tutorial/templates` in the Trovato release the demo runs, and CI fails on a
difference. `--update` takes the release's version.

The overlay itself is still plugins and nothing else. These files sit in
`docs/`, on `TEMPLATES_DIR`, where a reader can see whose they are.
