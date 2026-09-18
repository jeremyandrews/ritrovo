#!/usr/bin/env sh
#
# The Ritrovo install recipe, executable, run inside the released Trovato image.
#
# This is the entrypoint of the `trovato` service in docker-compose.demo.yml. It
# is the whole of docs/INSTALL.md's walkthrough with the manual steps taken out,
# and it is a script rather than a list of compose commands for one reason: the
# steps are ORDERED, and two of the orderings are traps.
#
#   1. The kernel migrates its own schema on startup, and `config import` writes
#      into tables that migration creates. So the server has to come up once
#      before any CLI step can run at all.
#
#   2. ritrovo_importer resolves the topic taxonomy ONCE, in tap_install, and
#      tap_install fires at the first server start after the plugin is enabled.
#      Enable it before the taxonomy exists and it finds 0 of 23 terms and imports
#      every conference untagged — recoverable only by resetting
#      tap_install_called and restarting. So the config set is imported BEFORE
#      any plugin is enabled, and nothing is enabled while the final server is up.
#
# Hence: serve (briefly) -> install -> stop -> import -> enable -> serve. The
# last step is an exec, so the kernel is PID 1 for the life of the container and
# `docker compose stop` reaches it.
#
# Every step is idempotent, so the script runs unconditionally on every start:
# the installer forms redirect and do nothing once the site is installed, config
# import upserts, and plugin enable is a no-op on an enabled plugin. There is no
# "already bootstrapped" flag to get out of step with the database.
#
# Environment (all defaulted by docker-compose.demo.yml):
#   PORT, DATABASE_URL, REDIS_URL, CRON_KEY   the kernel's own settings
#   PLUGINS_DIR, TEMPLATES_DIR, STATIC_DIR    the three search paths
#   DEMO_ADMIN_USER / _PASSWORD / _EMAIL      the account the installer creates
#   RITROVO_CONFIG_DIR                        Ritrovo's config set (this repo)

set -eu

PORT="${PORT:-3000}"
BASE="http://127.0.0.1:${PORT}"
DEMO_ADMIN_USER="${DEMO_ADMIN_USER:-admin}"
DEMO_ADMIN_PASSWORD="${DEMO_ADMIN_PASSWORD:-ritrovo-demo-password}"
DEMO_ADMIN_EMAIL="${DEMO_ADMIN_EMAIL:-admin@ritrovo.example}"
RITROVO_CONFIG_DIR="${RITROVO_CONFIG_DIR:-/ritrovo/demo/config}"

# The five plugins, in dependency order: the importer first because it is the one
# whose tap_install needs the taxonomy, ritrovo_translate last because it depends
# on the kernel's trovato_content_translation.
RITROVO_PLUGINS="ritrovo_importer ritrovo_access ritrovo_cfp ritrovo_notify ritrovo_translate"

# Kernel plugins the demo switches on. These ship in the image and are installed
# automatically at first start, but they declare default_enabled = false, so a
# stock kernel comes up without them.
#
#   trovato_search  builds the Pagefind index. Without it /search is blank in a
#                   browser: the page always loads scolta.js, which imports
#                   /static/pagefind/pagefind.js, and when that 404s the script
#                   clears the results the server had already rendered
#                   (G-SEARCH-PAGE-BLANK-WITHOUT-INDEX). The plugin's tap_cron
#                   compares the newest published item against last_indexed_at and
#                   asks for a rebuild when content has moved on, and the rebuild
#                   runs later in that same cron cycle — so enabling it here is the
#                   whole fix: the index is built on the first cron poke after this
#                   script hands over, and rebuilt after every import cycle that
#                   changes anything, with no further wiring.
KERNEL_PLUGINS="trovato_search"

say() { printf '\n==> %s\n' "$*"; }

# The install-check middleware redirects every path to /install until the site is
# installed, so a Location header naming /install is the "not installed" signal.
# Asking the kernel beats keeping a flag of our own.
site_installed() {
    location="$(curl -s -o /dev/null -w '%{redirect_url}' "$BASE/")"
    case "$location" in
        */install*) return 1 ;;
        *) return 0 ;;
    esac
}

wait_for_health() {
    i=0
    while [ "$i" -lt 120 ]; do
        if curl -fsS "$BASE/health" >/dev/null 2>&1; then
            return 0
        fi
        i=$((i + 1))
        sleep 1
    done
    echo "error: the kernel never became healthy on $BASE" >&2
    return 1
}

# ── 1. Bring the kernel up once, so it migrates and discovers plugins ─────────
#
# This start also auto-installs the five Ritrovo plugins into plugin_status. They
# land DISABLED, because every Ritrovo manifest declares default_enabled = false,
# which is exactly what step 4 needs: `plugin enable` requires an installed
# plugin, and tap_install has not fired for a disabled one.

say "starting the kernel once (schema migration, plugin discovery)"
./trovato serve >/tmp/bootstrap-serve.log 2>&1 &
bootstrap_pid=$!
if ! wait_for_health; then
    cat /tmp/bootstrap-serve.log >&2
    exit 1
fi

# ── 2. Complete the installer ────────────────────────────────────────────────
#
# The installer is two plain form POSTs with no CSRF token, so curl is enough.
# Passwords are validated at a 12-character minimum, which the default clears.

if site_installed; then
    say "site already installed, leaving the admin account alone"
else
    say "completing the installer as '$DEMO_ADMIN_USER'"
    curl -fsS -o /dev/null -X POST "$BASE/install/admin" \
        --data-urlencode "username=$DEMO_ADMIN_USER" \
        --data-urlencode "email=$DEMO_ADMIN_EMAIL" \
        --data-urlencode "password=$DEMO_ADMIN_PASSWORD" \
        --data-urlencode "password_confirm=$DEMO_ADMIN_PASSWORD"
    curl -fsS -o /dev/null -X POST "$BASE/install/site" \
        --data-urlencode "site_name=Ritrovo" \
        --data-urlencode "site_slogan=Conferences worth the trip" \
        --data-urlencode "site_mail=$DEMO_ADMIN_EMAIL"

    # A form that fails validation renders the form again with a 200, so curl's
    # -f cannot tell us. Ask the kernel whether it worked.
    if ! site_installed; then
        echo "error: the installer did not complete; see the log below" >&2
        cat /tmp/bootstrap-serve.log >&2
        exit 1
    fi
fi

say "stopping the bootstrap server"
kill "$bootstrap_pid" 2>/dev/null || true
wait "$bootstrap_pid" 2>/dev/null || true

# ── 3. Import the config set, BEFORE enabling anything ───────────────────────
#
# The set is Ritrovo's own, in this repository, mounted at /ritrovo/demo/config.
# It used to be Trovato's tutorial copy inside the image; the content model is
# Ritrovo's to define, so the whole set moved here and the image's copy is no
# longer read. See demo/config/README.md.
#
# It imports as ONE set because import will not take it any other way: references
# are resolved across the whole directory and then the database, and an
# unresolved one fails the import with nothing written. The search field configs
# name the conference and speaker types, the tiles and aliases name the gathers,
# the tags name their category and their parents, and the seed's items name the
# type and the stage.
#
# --dry-run first because that validation is the expensive half and runs either
# way: a preflight costs a second and names every offending file.

say "importing Ritrovo's config set from $RITROVO_CONFIG_DIR"
./trovato config import "$RITROVO_CONFIG_DIR" --dry-run
./trovato config import "$RITROVO_CONFIG_DIR"

# ── 4. Enable the five plugins ───────────────────────────────────────────────

say "enabling the Ritrovo plugins"
for plugin in $RITROVO_PLUGINS; do
    ./trovato plugin enable "$plugin"
done

# Enabled here rather than anywhere later because the kernel snapshots the enabled
# set when it builds its state: CronService reads it once at startup, and enabling
# trovato_search on a server that is already running leaves the Pagefind rebuild
# switched off until the next restart. Step 6 is that restart.
say "enabling the kernel plugins the demo uses"
for plugin in $KERNEL_PLUGINS; do
    ./trovato plugin enable "$plugin"
done

# ── 5. The seeded content ────────────────────────────────────────────────────
#
# Its own directory and its own import because config import does not recurse:
# it reads the .yml files in the directory it is given and skips subdirectories,
# which is what keeps the seeds out of the configuration set proper.
#
# Two seeds, both pure database writes, so they run here rather than against the
# live server. seed-italian is Part 7's content-translation demonstration.
# seed-content is Ritrovo's: descriptions, speakers, topics and editor notes on
# the conferences the importer landed bare, so the demo shows a filled-in
# conference without anyone typing one in.

# Speakers first: the seeded conferences carry field_speakers referencing them,
# and while nothing validates an item's field references, a conference imported
# before its speakers renders a list of nothing until the next import.
say "importing the seeded speakers"
./trovato config import "$RITROVO_CONFIG_DIR/seed-content"

say "importing the Italian seed content"
./trovato config import "$RITROVO_CONFIG_DIR/seed-italian"

# ── 6. Serve ─────────────────────────────────────────────────────────────────
#
# tap_install fires here, on this start, for the five plugins step 4 enabled.
# ritrovo_importer's runs in a background task: it fetches every confs.tech
# topic/year file from 2015 to now and pushes the batches onto the ritrovo_import
# queue, which takes minutes. The cron service drains the queue meanwhile.

say "serving on port $PORT"
exec ./trovato serve
