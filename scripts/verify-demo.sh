#!/usr/bin/env bash
#
# Check that the running demo is actually a populated Ritrovo site.
#
# WHY THIS IS NOT THE HEALTH CHECK
# /health answers 200 from a kernel with no plugins, no content and no config. So
# does a kernel that is redirecting every path to its own installer. Everything
# this demo is for lives past that point, so each check below asks for something
# that only exists when one specific part of the install worked, and prints what
# it saw rather than a bare pass.
#
# Usage: scripts/verify-demo.sh [base-url]        (default http://localhost:3000)
#
# Environment:
#   DEMO_ADMIN_USER / DEMO_ADMIN_PASSWORD   the account demo-bootstrap.sh created
#   COMPOSE_FILE                            for the counts read out of Postgres
#
# Exits non-zero on the first failed check, and says which.

set -uo pipefail

BASE="${1:-http://localhost:3000}"
DEMO_ADMIN_USER="${DEMO_ADMIN_USER:-admin}"
DEMO_ADMIN_PASSWORD="${DEMO_ADMIN_PASSWORD:-ritrovo-demo-password}"
COMPOSE_FILE="${COMPOSE_FILE:-docker-compose.demo.yml}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
JAR="$(mktemp)"
trap 'rm -f "$JAR"' EXIT

failures=0

ok()   { printf '  ok    %s\n' "$*"; }
bad()  { printf '  FAIL  %s\n' "$*"; failures=$((failures + 1)); }
note() { printf '        %s\n' "$*"; }
head_() { printf '\n%s\n' "$*"; }

code() { curl -s -o /dev/null -w '%{http_code}' "$BASE$1"; }
code_as_admin() { curl -s -b "$JAR" -o /dev/null -w '%{http_code}' "$BASE$1"; }

# Numbers straight out of Postgres. Optional: the HTTP checks are the contract,
# these are the "how much of it is there" the report wants. Silent if the compose
# stack is not the one running.
sql() {
    (cd "$ROOT" && docker compose -f "$COMPOSE_FILE" exec -T postgres \
        psql -U trovato -d ritrovo -tAc "$1" 2>/dev/null | tr -d '[:space:]')
}

head_ "kernel"
release="$(curl -s "$BASE/health" >/dev/null 2>&1 && echo up || echo down)"
if [ "$release" = "up" ]; then ok "/health answers"; else bad "/health does not answer at $BASE"; fi

head_ "plugins"
# `plugin list` reads the same plugin_status rows the kernel loads from, and it
# runs inside the container, so it sees the real search path — including whether
# the overlay won it. Only reachable through the compose stack, so a manual
# install gets a note here instead of five failures it cannot fix.
plugin_list="$(cd "$ROOT" && docker compose -f "$COMPOSE_FILE" exec -T trovato \
    ./trovato plugin list 2>/dev/null | grep '^ritrovo_')"
if [ -z "$plugin_list" ]; then
    note "skipped: needs the $COMPOSE_FILE stack. Check by hand with"
    note "\`trovato plugin list | grep ritrovo_\`"
else
    for plugin in ritrovo_importer ritrovo_access ritrovo_cfp ritrovo_notify ritrovo_translate; do
        line="$(printf '%s\n' "$plugin_list" | grep "^$plugin ")"
        if [[ "$line" == *enabled* ]]; then
            ok "$(printf '%s' "$line" | awk '{printf "%-20s %-8s %s", $1, $2, $3}')"
        else
            bad "$plugin is not enabled (got: ${line:-nothing})"
        fi
    done
fi

head_ "front page"
front="$(curl -s -o /dev/null -w '%{http_code} %{redirect_url}' "$BASE/")"
case "$front" in
    30*conferences) ok "/ redirects to the conference listing ($front)" ;;
    *) bad "/ answered '$front', expected a redirect to /conferences" ;;
esac

head_ "content"
if [ "$(code /conferences)" = "200" ]; then
    cards="$(curl -s "$BASE/conferences" | grep -c 'card--conf')"
    ok "/conferences renders 200 with $cards conference cards on page one"
    [ "$cards" -gt 0 ] || bad "/conferences is empty; the import did not land"
else
    bad "/conferences answered $(code /conferences)"
fi
total="$(sql "select count(*) from item where type='conference'")"
upcoming="$(sql "select count(*) from item where type='conference' and fields->>'field_start_date' >= to_char(now(), 'YYYY-MM-DD')")"
[ -n "$total" ] && note "conferences in the database: $total, of which $upcoming are upcoming"
queued="$(sql 'select count(*) from plugin_queue')"
if [ -n "$queued" ]; then
    if [ "$queued" = "0" ]; then ok "the import queue is empty"; else note "$queued job(s) still queued"; fi
fi

head_ "the importer's admin screens"
curl -s -c "$JAR" -o /dev/null -X POST "$BASE/user/login/json" \
    -H 'Content-Type: application/json' \
    -d "{\"username\":\"$DEMO_ADMIN_USER\",\"password\":\"$DEMO_ADMIN_PASSWORD\"}"
for path in /admin/content/conferences /admin/config/importer; do
    anon="$(code "$path")"
    admin="$(code_as_admin "$path")"
    if [ "$anon" = "401" ] && [ "$admin" = "200" ]; then
        ok "$path — anonymous $anon, administrator $admin"
    else
        bad "$path — anonymous $anon, administrator $admin (want 401 / 200)"
    fi
done

head_ "search"
results="$(curl -s "$BASE/search?q=rust" | grep -o '[0-9]\+ results\? for' | head -n 1)"
if [ -n "$results" ]; then
    ok "/search?q=rust returns $results \"rust\" in the server-rendered HTML"
else
    bad "/search?q=rust returned no result count"
fi
# That count is not what a visitor sees. templates/search.html always loads
# scolta.js, which imports the Pagefind index and CLEARS the server's results when
# the import 404s (G-SEARCH-PAGE-BLANK-WITHOUT-INDEX). The check that matches the
# browser is therefore whether the script the page asks for is actually there.
pagefind_js="$(code /static/pagefind/pagefind.js)"
if [ "$pagefind_js" = "200" ]; then
    ok "/static/pagefind/pagefind.js answers 200 — the search page has an index to load"
else
    bad "/static/pagefind/pagefind.js answered $pagefind_js — /search is blank in a browser"
fi
# ...and whether the index has the site's content in it rather than being an empty
# shell. Pagefind writes gzipped JSON fragments into the first STATIC_DIR entry,
# which is the writable volume; gzip reads the concatenation of them as one stream.
# INDEX_MARKER is a conference the Italian seed guarantees on any bootstrapped
# demo, so its absence means the index is stale or empty, not that data varies.
INDEX_MARKER="${INDEX_MARKER:-Codemotion Roma 2026}"
indexed="$(cd "$ROOT" && docker compose -f "$COMPOSE_FILE" exec -T trovato sh -c \
    'cat /var/lib/ritrovo/index/pagefind/fragment/*.pf_fragment 2>/dev/null | gzip -dc 2>/dev/null' \
    2>/dev/null | grep -c "$INDEX_MARKER" || true)"
if [ -z "$indexed" ]; then
    note "skipped the index content check: needs the $COMPOSE_FILE stack"
elif [ "$indexed" -gt 0 ]; then
    ok "the Pagefind index contains \"$INDEX_MARKER\""
else
    bad "the Pagefind index does not contain \"$INDEX_MARKER\" — it is empty or stale"
fi

head_ "conference detail page"
# The seeded conference, so the name, dates and city are fixed rather than whatever
# confs.tech happened to return today.
conf_id="$(sql "select id from item where type='conference' and title='$INDEX_MARKER' limit 1")"
if [ -z "$conf_id" ]; then
    note "skipped: needs the $COMPOSE_FILE stack to find \"$INDEX_MARKER\""
else
    conf_page="$(curl -s "$BASE/item/$conf_id")"
    # The kernel renders every scalar field a second time as `label: value` inside
    # `children`, wrapped in this class. A template that renders its fields in
    # their own places must not also print that.
    if [[ "$conf_page" == *'field__label'* ]]; then
        bad "the conference page dumps its fields raw (found class=\"field__label\")"
    else
        ok "the conference page prints no raw field dump"
    fi
    for want in "$INDEX_MARKER" "2026-03-25" "Roma"; do
        if [[ "$conf_page" == *"$want"* ]]; then
            ok "the conference page shows \"$want\""
        else
            bad "the conference page does not show \"$want\""
        fi
    done
fi

head_ "navigation"
# Every seeded menu link, followed as rendered. A link in the site chrome that
# 404s is the one defect a visitor meets before any content loads.
nav_page="$(curl -s "$BASE/conferences")"
nav_links="$(printf '%s\n' "$nav_page" \
    | grep -o 'href="[^"]*"[^>]*>[^<]*</a>' \
    | grep -E 'Call for Papers|Conferences</a>|Speakers</a>|Topics</a>' \
    | sed 's/href="//; s/"[^>]*>/ /; s/<\/a>//; s/&#x2F;/\//g')"
if [ -z "$nav_links" ]; then
    bad "no main-menu links rendered on /conferences"
else
    while read -r href label; do
        [ -n "$href" ] || continue
        status="$(code "$href")"
        if [ "$status" = "200" ]; then
            ok "main menu \"$label\" -> $href is $status"
        else
            bad "main menu \"$label\" -> $href is $status"
        fi
    done <<< "$nav_links"
fi

head_ "Italian"
for path in /it/conferenze /it/relatori /it/argomenti; do
    if [ "$(code "$path")" = "200" ]; then ok "$path renders 200"; else bad "$path answered $(code "$path")"; fi
done
# Match on the captured page, never `curl | grep -q`. With pipefail set, grep -q
# exits at the first match, near the top of the page, curl takes a SIGPIPE writing
# the rest, and the pipeline reports failure for a page that passed. It did, on
# roughly one run in eight.
italian_page="$(curl -s "$BASE/it/conferenze")"
if [[ "$italian_page" == *'lang="it"'* ]]; then
    ok '/it/conferenze renders with lang="it"'
else
    bad '/it/conferenze did not render in Italian'
fi
italian="$(sql "select count(*) from item where type='conference' and fields->>'primary_language'='it'")"
[ -n "$italian" ] && note "conferences whose primary language is Italian: $italian"

printf '\n'
if [ "$failures" -eq 0 ]; then
    echo "all checks passed"
else
    echo "$failures check(s) failed" >&2
fi
exit "$failures"
