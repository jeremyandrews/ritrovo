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

# The same, for values whose spaces matter. `sql` strips ALL whitespace, which is
# right for a count or a uuid and wrong for a title: it turned "Cloudconf Torino
# 2026" into "CloudconfTorino2026", which then matched nothing on the page.
sql_text() {
    (cd "$ROOT" && docker compose -f "$COMPOSE_FILE" exec -T postgres \
        psql -U trovato -d ritrovo -tAc "$1" 2>/dev/null | head -n 1 | sed 's/[[:space:]]*$//')
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
    for plugin in ritrovo_importer ritrovo_access ritrovo_cfp ritrovo_forms ritrovo_notify ritrovo_translate; do
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
# The marker is counted INSIDE the container, so only a number crosses back: the
# fragments for 5,656 conferences are megabytes, and none of it is interesting
# here. It travels as an environment variable rather than interpolated into the
# shell string, so a marker with a quote in it cannot rewrite the command.
# No output at all means the compose stack is not the one running, which is a
# skip; "0" means the stack answered and the index does not have the conference.
indexed="$( (cd "$ROOT" && docker compose -f "$COMPOSE_FILE" exec -T \
    -e MARKER="$INDEX_MARKER" trovato sh -c \
    'cat /var/lib/ritrovo/index/pagefind/fragment/*.pf_fragment 2>/dev/null | gzip -dc 2>/dev/null | grep -c "$MARKER"') 2>/dev/null )"
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

head_ "the content model"
# A seeded conference: its editorial fields come from demo/config, so what it
# shows is fixed rather than whatever confs.tech returned today.
seeded_id="$(sql "select id from item where type='conference' and fields ? 'field_speakers' order by title limit 1")"
seeded_title="$(sql_text "select title from item where id='$seeded_id'")"
if [ -z "$seeded_id" ]; then
    note "skipped: needs the $COMPOSE_FILE stack to find a seeded conference"
else
    seeded_page="$(curl -s "$BASE/item/$seeded_id")"
    if [[ "$seeded_page" == *'conf-detail__speakers'* ]]; then
        ok "the seeded conference \"$seeded_title\" lists its speakers"
    else
        bad "the seeded conference \"$seeded_title\" shows no speakers"
    fi
    # Follow the first speaker it names back to their own page, which must list
    # this conference: the same one field, read forwards there and backwards here.
    speaker_id="$(printf '%s' "$seeded_page" | tr '\n' ' ' \
        | grep -o 'conf-detail__speaker-list.*' \
        | grep -o '/item/[0-9a-f-]\{36\}' | head -n 1 | cut -d/ -f3)"
    if [ -z "$speaker_id" ]; then
        bad "the seeded conference names no speaker to follow"
    else
        speaker_page="$(curl -s "$BASE/item/$speaker_id")"
        if [[ "$speaker_page" == *"$seeded_title"* ]]; then
            ok "that speaker's page lists \"$seeded_title\" back"
        else
            bad "that speaker's page does not list \"$seeded_title\""
        fi
        if [[ "$speaker_page" == *'field__label'* ]]; then
            bad "the speaker page dumps its fields raw"
        else
            ok "the speaker page prints no raw field dump"
        fi
    fi
fi
speakers_total="$(sql "select count(*) from item where type='speaker'")"
[ -n "$speakers_total" ] && note "speakers in the database: $speakers_total"
topic_terms="$(sql "select count(*) from category_tag where category_id='topics'")"
[ -n "$topic_terms" ] && note "topic terms: $topic_terms"
# The tree is three levels deep and one term hangs off two parents.
depth3="$(sql "select count(*) from category_tag_hierarchy h join category_tag_hierarchy g on g.tag_id = h.parent_id")"
[ -n "$depth3" ] && [ "$depth3" != "0" ] && ok "the topic tree is three levels deep ($depth3 grandchild link(s))"
crosslisted="$(sql "select count(*) from (select tag_id from category_tag_hierarchy group by tag_id having count(*) > 1) t")"
if [ -n "$crosslisted" ]; then
    if [ "$crosslisted" -gt 0 ]; then ok "$crosslisted term(s) are cross-listed under two parents"; else bad "no term is cross-listed; Kotlin should be"; fi
fi
untagged_language="$(sql "select count(*) from item where type='conference' and not (fields ? 'field_language')")"
if [ -n "$untagged_language" ]; then
    if [ "$untagged_language" = "0" ]; then
        ok "every conference carries a language"
    else
        bad "$untagged_language conference(s) have no language, so the language filter hides them"
    fi
fi

head_ "the listings the model feeds"
for path in /conferences /speakers /cfps /topics /conferences/this-month /cfps/closing-soon; do
    status="$(code "$path")"
    if [ "$status" = "200" ]; then ok "$path renders $status"; else bad "$path answered $status"; fi
done
# The location pages had no template of their own and fell back to the kernel's
# generic table, which prints every column of every row including search_vector.
country="$(sql "select fields->>'field_country' from item where type='conference' \
    and fields->>'field_country' <> '' \
    and fields->>'field_start_date' >= to_char(now(), 'YYYY-MM-DD') \
    group by 1 order by count(*) desc limit 1")"
if [ -n "$country" ]; then
    location_page="$(curl -s "$BASE/location/$country")"
    if [[ "$location_page" == *'search_vector'* ]]; then
        bad "/location/$country dumps raw item columns"
    else
        ok "/location/$country renders without a raw column dump"
    fi
    if [[ "$location_page" == *'card--conf'* ]]; then
        ok "/location/$country renders conference cards"
    else
        bad "/location/$country renders no conference cards"
    fi
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

head_ "the editorial workflow"

# The brief's pipeline, checked as a visitor and as each of the three editorial
# users. This is the part of the demo that is Ritrovo's own: the kernel has no
# way to move an item between stages, so the screen these checks exercise is
# served by ritrovo_access (FRICTION.md, G-NO-ITEM-STAGE-TRANSITION).
EDITORIAL_PATH="/admin/content/editorial"

# One jar per user, because a session is what carries the role.
editor_jar="$(mktemp)"
publisher_jar="$(mktemp)"
viewer_jar="$(mktemp)"
trap 'rm -f "$JAR" "$editor_jar" "$publisher_jar" "$viewer_jar"' EXIT

# Log in, retrying past the rate limiter.
#
# The kernel counts one login against the limit TWICE, once in the middleware and
# once in the handler, so the documented five-per-minute is really two and a
# script that logs in three users in a row trips it
# (FRICTION.md, G-RATE-LIMIT-COUNTED-TWICE). Retrying is what makes these checks
# about the editorial workflow rather than about the rate limiter.
login_as() {
    jar="$1"
    username="$2"
    password="$3"
    attempt=0
    while [ "$attempt" -lt 10 ]; do
        if curl -s -c "$jar" -X POST "$BASE/user/login/json" \
            -H 'Content-Type: application/json' \
            -d "{\"username\":\"$username\",\"password\":\"$password\"}" \
            | grep -q '"success":true'; then
            return 0
        fi
        attempt=$((attempt + 1))
        sleep 10
    done
    return 1
}

code_with() { curl -s -b "$1" -o /dev/null -w '%{http_code}' "$BASE$2"; }

for account in \
    "editor_alice:ritrovo-editor-demo:$editor_jar" \
    "publisher_bob:ritrovo-publisher-demo:$publisher_jar" \
    "viewer_carol:ritrovo-viewer-demo:$viewer_jar"; do
    name="${account%%:*}"
    rest="${account#*:}"
    password="${rest%%:*}"
    jar="${rest#*:}"
    if login_as "$jar" "$name" "$password"; then
        ok "$name can log in"
    else
        bad "$name could not log in (the account, or its password, is wrong)"
    fi
done

# The Incoming listing: refused to a visitor, refused to a signed-in reader,
# served to an editor.
#
# Anonymous is 401 and not 403 on purpose: a plugin route answers UNAUTHORIZED to
# a caller with no session and FORBIDDEN to one that is signed in and lacks the
# permission (crates/kernel/src/routes/plugin_api.rs:277-284). Both are checked,
# because "refused" that cannot tell those two apart would pass while signed-in
# readers were being let in.
anon_code="$(code "$EDITORIAL_PATH")"
viewer_code="$(code_with "$viewer_jar" "$EDITORIAL_PATH")"
editor_code="$(code_with "$editor_jar" "$EDITORIAL_PATH")"
if [ "$anon_code" = "401" ] && [ "$viewer_code" = "403" ] && [ "$editor_code" = "200" ]; then
    ok "$EDITORIAL_PATH — anonymous $anon_code, viewer $viewer_code, editor $editor_code"
else
    bad "$EDITORIAL_PATH — anonymous $anon_code, viewer $viewer_code, editor $editor_code (want 401 / 403 / 200)"
fi

# A conference on Incoming: invisible to a visitor, visible to an editor.
#
# This is the access tap doing its job, not the screen's permission gate: the
# kernel asks ritrovo_access before it will render an item on an internal stage,
# and a Deny becomes the absence a 404 is built from.
incoming_id="$(sql "select id from item where type='conference' and stage_id='0193a5a0-0000-7000-8000-000000000002' limit 1")"
if [ -n "$incoming_id" ]; then
    anon_item="$(code "/item/$incoming_id")"
    viewer_item="$(code_with "$viewer_jar" "/item/$incoming_id")"
    editor_item="$(code_with "$editor_jar" "/item/$incoming_id")"
    if [ "$anon_item" = "404" ] && [ "$viewer_item" = "404" ] && [ "$editor_item" = "200" ]; then
        ok "a conference on Incoming — anonymous $anon_item, viewer $viewer_item, editor $editor_item"
    else
        bad "a conference on Incoming — anonymous $anon_item, viewer $viewer_item, editor $editor_item (want 404 / 404 / 200)"
    fi
else
    note "skipped the Incoming item check: nothing is on the Incoming stage"
fi

# Publishing is the publisher's, not the editor's.
#
# The button is drawn only for a viewer who may use it, and the handler re-checks
# before it writes, so this asserts the visible half of a gate that is enforced
# twice.
publish_control='value="live"'
if curl -s -b "$publisher_jar" "$BASE$EDITORIAL_PATH?stage=curated" | grep -q "$publish_control"; then
    if curl -s -b "$editor_jar" "$BASE$EDITORIAL_PATH?stage=curated" | grep -q "$publish_control"; then
        bad "the editor is offered Publish to Live, which is the publisher's transition"
    else
        ok "Publish to Live is offered to the publisher and not to the editor"
    fi
else
    bad "the publisher is not offered Publish to Live on the Curated queue"
fi

for stage_name in incoming curated live; do
    stage_uuid=""
    case "$stage_name" in
        incoming) stage_uuid="0193a5a0-0000-7000-8000-000000000002" ;;
        curated)  stage_uuid="0193a5a0-0000-7000-8000-000000000003" ;;
        live)     stage_uuid="0193a5a0-0000-7000-8000-000000000001" ;;
    esac
    count="$(sql "select count(*) from item where type='conference' and stage_id='$stage_uuid'")"
    [ -n "$count" ] && note "conferences on $stage_name: $count"
done

# The importer lands on Incoming, which is the change that makes the pipeline
# real: before it, every import went straight to Live and nothing was ever
# reviewed.
live_only="$(sql "select count(*) from item where type='conference' and stage_id='0193a5a0-0000-7000-8000-000000000002'")"
if [ -n "$live_only" ] && [ "$live_only" != "0" ]; then
    ok "the importer lands conferences on Incoming"
else
    bad "nothing is on Incoming; the importer is landing conferences somewhere else"
fi

printf '\n'
if [ "$failures" -eq 0 ]; then
    echo "all checks passed"
else
    echo "$failures check(s) failed" >&2
fi
exit "$failures"
