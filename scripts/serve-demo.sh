#!/usr/bin/env bash
#
# Stand Ritrovo up on the released Trovato, from nothing, in one command.
#
#   scripts/serve-demo.sh
#
# What that gets you: Postgres and Redis, the five Ritrovo plugins compiled to
# WebAssembly in a throwaway container, the published Trovato image with the
# Ritrovo overlay appended to its three search paths, the installer completed, the
# tutorial config set imported, the plugins enabled in the order that matters, the
# conference import queue drained, the Italian seed content loaded, and the front
# page pointed at /conferences. Then it checks all of that and tells you what it
# saw.
#
# Requires Docker and nothing else. No Trovato checkout, no Rust toolchain, no
# credentials — which is the claim this script exists to keep honest: Ritrovo is
# an overlay on a stock release, so a stranger has to be able to see it work
# without building the kernel.
#
# The compose file is the demo; this is a wrapper that waits for it and reports.
# `docker compose -f docker-compose.demo.yml up` on its own reaches the same site,
# it just does not tell you when it is finished.
#
# Usage:
#   scripts/serve-demo.sh [--fresh] [--no-verify] [--no-wait]
#   scripts/serve-demo.sh --down
#
#   --fresh       throw away the existing database and volumes first
#   --no-verify   bring it up, skip the checks
#   --no-wait     do not wait for the import queue to drain
#   --down        stop everything and delete its volumes
#
# Environment:
#   RITROVO_DEMO_PORT   host port to publish on           (3000)
#   DRAIN_TIMEOUT       seconds to wait for the import     (1200)
#   DEMO_ADMIN_USER / DEMO_ADMIN_PASSWORD                  (admin / ritrovo-demo-password)

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_FILE="${COMPOSE_FILE:-docker-compose.demo.yml}"
PORT="${RITROVO_DEMO_PORT:-3000}"
BASE="http://localhost:$PORT"
DRAIN_TIMEOUT="${DRAIN_TIMEOUT:-1200}"

FRESH=0
VERIFY=1
WAIT=1
DOWN=0
for arg in "$@"; do
    case "$arg" in
        --fresh)     FRESH=1 ;;
        --no-verify) VERIFY=0 ;;
        --no-wait)   WAIT=0 ;;
        --down)      DOWN=1 ;;
        # Print this file's header comment, which IS the usage text. Bounded by
        # the first line of code rather than a line number, so editing the header
        # cannot make --help start printing the script.
        -h|--help)   sed -n '2,/^set -euo/p' "${BASH_SOURCE[0]}" | sed '$d'; exit 0 ;;
        *) echo "error: unknown argument $arg" >&2; exit 1 ;;
    esac
done

cd "$ROOT"
compose() { docker compose -f "$COMPOSE_FILE" "$@"; }

if ! docker info >/dev/null 2>&1; then
    echo "error: Docker is not running, and it is the only thing this demo needs" >&2
    exit 1
fi

if [ "$DOWN" = "1" ]; then
    echo "==> tearing the demo down, volumes and all"
    compose down -v
    exit 0
fi

if [ "$FRESH" = "1" ]; then
    echo "==> discarding the previous demo"
    compose down -v
fi

echo "==> building the plugins and starting the stack"
echo "    (first run compiles five WASM modules and pulls four images)"
compose up -d

echo "==> waiting for the kernel on $BASE"
deadline=$((SECONDS + 300))
until curl -fsS "$BASE/health" >/dev/null 2>&1; do
    if [ "$SECONDS" -ge "$deadline" ]; then
        echo "error: the kernel never became healthy; last 40 lines of its log:" >&2
        compose logs --tail 40 trovato >&2
        exit 1
    fi
    sleep 2
done
echo "    up"

if [ "$WAIT" = "1" ]; then
    # ritrovo_importer's tap_install fetches every confs.tech topic/year file from
    # 2015 to now and queues a batch each; the cron service drains them. This is
    # the slow part of the demo, and the site is already browsable while it runs —
    # it just has fewer conferences in it than it will have.
    echo "==> waiting for the conference import to drain (up to ${DRAIN_TIMEOUT}s)"
    deadline=$((SECONDS + DRAIN_TIMEOUT))
    last=""
    # The cron service's log is the progress bar. Read it into a variable once per
    # pass and match against that: with `pipefail` set, `docker compose logs | grep`
    # reports the pipeline as failed both when grep finds nothing AND when grep
    # exits first and `logs` takes a SIGPIPE, so neither status means what it looks
    # like it means. Every grep below is therefore `|| true`, and the status this
    # loop turns on is the presence of a string, not an exit code.
    while true; do
        log="$(compose logs --tail 500 cron 2>/dev/null || true)"
        drained="$(printf '%s\n' "$log" | grep 'import queue drained' | tail -n 1 || true)"
        if [ -n "$drained" ]; then
            printf '%s\n' "$drained" | sed 's/^cron-1  *| *\[cron\] /    /'
            break
        fi
        if [ "$SECONDS" -ge "$deadline" ]; then
            echo "    still importing after ${DRAIN_TIMEOUT}s; carrying on anyway"
            break
        fi
        progress="$(printf '%s\n' "$log" | grep -o '[0-9]\+ so far' | tail -n 1 || true)"
        if [ -n "$progress" ] && [ "$progress" != "$last" ]; then
            echo "    imported $progress"
            last="$progress"
        fi
        sleep 5
    done
fi

if [ "$VERIFY" = "1" ]; then
    echo "==> checking the site"
    COMPOSE_FILE="$COMPOSE_FILE" "$ROOT/scripts/verify-demo.sh" "$BASE"
fi

cat <<EOF

Ritrovo is serving on $BASE

  $BASE/                        the front page, which redirects to /conferences
  $BASE/conferences             upcoming conferences, filterable
  $BASE/speakers                speakers
  $BASE/cfps                    open calls for papers
  $BASE/it/conferenze           the same listing in Italian
  $BASE/search?q=rust           search
  $BASE/admin/content/conferences   what the importer has landed
  $BASE/admin/config/importer       the importer's own state

  log in as ${DEMO_ADMIN_USER:-admin} / ${DEMO_ADMIN_PASSWORD:-ritrovo-demo-password}

  logs:  docker compose -f $COMPOSE_FILE logs -f trovato
  stop:  scripts/serve-demo.sh --down
EOF
