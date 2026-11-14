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

# The index covers what is on Live, all of it: Pagefind's entry file counts the
# pages it holds, and one page is one item. serve-demo.sh waits for this before
# it reports ready, so a short index here means it was rebuilt since, wrongly.
live_items="$(sql "select count(*) from item where status = 1 and stage_id = '0193a5a0-0000-7000-8000-000000000001'")"
index_pages="$(curl -s "$BASE/static/pagefind/pagefind-entry.json" \
    | grep -o '"page_count":[0-9]*' | cut -d: -f2 | awk '{ n += $1 } END { print n + 0 }')"
if [ -z "$live_items" ]; then
    note "skipped the index size check: needs the $COMPOSE_FILE stack"
elif [ "$index_pages" = "$live_items" ]; then
    ok "the Pagefind index holds $index_pages pages, one per Live item"
else
    bad "the Pagefind index holds ${index_pages:-no} pages for $live_items Live items"
fi

# What a visitor's browser gets back. The page hands the query to Pagefind, so
# the question is what Pagefind answers, asked through the same pagefind.js the
# page loads (scripts/pagefind-query.mjs). Before A9 the fragment check above
# passed while every title search in Chrome said "No results found": the
# kernel's export leaves titles out of the indexed body
# (G-PAGEFIND-INDEX-OMITS-TITLES), which ritrovo_importer now covers.
#
# A conference by its title and a speaker by name, both seeded so the answer is
# fixed. Needs Node; without it the checks above are all there is.
SPEAKER_MARKER="${SPEAKER_MARKER:-Bruno Sala}"
if command -v node >/dev/null 2>&1; then
    for want in "$INDEX_MARKER" "$SPEAKER_MARKER"; do
        hits="$(node "$ROOT/scripts/pagefind-query.mjs" "$BASE" "$want" 2>&1)"
        hit="$(printf '%s\n' "$hits" | awk -F '\t' -v t="$want" '$1 == t { print; exit }')"
        if [ -n "$hit" ]; then
            ok "a Pagefind search for \"$want\" returns it ($(printf '%s' "$hits" | grep -c .) hit(s))"
        else
            bad "a Pagefind search for \"$want\" does not return it: $(printf '%s' "$hits" | head -c 160)"
            continue
        fi
        # Where the result links. The kernel picks one alias per item whatever
        # its language and drops the prefix, so a seeded conference with an
        # Italian alias can link to /conferenze/... with no /it/ in front, which
        # is 404 (G-PAGEFIND-RESULT-URL-IGNORES-ALIAS-LANGUAGE). Reported, not
        # failed: nothing in Ritrovo decides which alias the kernel picks.
        href="$(printf '%s' "$hit" | cut -f2)"
        href_code="$(code "$href")"
        if [ "$href_code" = "200" ]; then
            ok "  and links to $href, which is $href_code"
        else
            note "its result links to $href, which is $href_code (kernel: G-PAGEFIND-RESULT-URL-IGNORES-ALIAS-LANGUAGE)"
        fi
    done
else
    note "skipped the Pagefind queries: needs node to load pagefind.js"
fi
# The brief's weights, read back out of the tsvector the server ranks by: title
# A, description B, city and topics C. The topics are the denormalized labels
# ritrovo_importer writes (field_topic_names), because the tag uuids in
# field_topics are not words (G-SEARCH-CANNOT-INDEX-CATEGORY-LABELS). Codemotion
# Roma 2026 is seeded in Roma under Web Platform, so all three are fixed.
weighted="$(sql "select search_vector @@ to_tsquery('english', 'codemotion:A & roma:C & platform:C') from item where title = '$INDEX_MARKER'")"
if [ -z "$weighted" ]; then
    note "skipped the weight check: needs the $COMPOSE_FILE stack"
elif [ "$weighted" = "t" ]; then
    ok "\"$INDEX_MARKER\" is indexed with its title at A and its city and topic at C"
else
    bad "\"$INDEX_MARKER\" is not indexed with title A, city C and topic C"
fi
# The server's own search, the no-JavaScript half, finds the speaker too: the
# brief's search is across conferences and speakers.
server_speaker="$(curl -s -G "$BASE/search" --data-urlencode "q=$SPEAKER_MARKER")"
if [[ "$server_speaker" == *">$SPEAKER_MARKER</a>"* ]]; then
    ok "/search?q=$SPEAKER_MARKER finds the speaker in the server-rendered results"
else
    bad "/search?q=$SPEAKER_MARKER does not find the speaker server-side"
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

# Topic pills, from the generated labels (includes/topics.html), each a link to
# its listing. The marker is seeded under Web Platform.
if [ -n "$conf_id" ]; then
    pill_href="$(printf '%s' "$conf_page" | tr '\n' ' ' | grep -o 'class="topic-pill" href="[^"]*"' | head -n 1 | sed 's/.*href="//; s/"$//')"
    if [ -z "$pill_href" ]; then
        bad "the conference page renders no topic pills"
    elif [ "$(code "$pill_href")" = "200" ]; then
        ok "the conference page renders topic pills; the first, $pill_href, is 200"
    else
        bad "the conference page's topic pill $pill_href answered $(code "$pill_href")"
    fi
fi
# The CFP badge, on a Live conference whose call for papers is still open. Drawn
# by the template from the same rule as ritrovo_cfp's, because the plugin's own
# badge arrives in the `children` string the template cannot render
# (G-RENDER-CHILDREN-MIXES-FIELD-DUMP-AND-PLUGIN-OUTPUT).
open_cfp_id="$(sql "select id from item where type='conference' and status = 1 \
    and stage_id = '0193a5a0-0000-7000-8000-000000000001' \
    and fields->>'field_cfp_end_date' >= to_char(now() at time zone 'utc', 'YYYY-MM-DD') \
    order by fields->>'field_cfp_end_date' limit 1")"
if [ -z "$open_cfp_id" ]; then
    note "skipped the CFP badge check: no Live conference has an open call for papers today"
else
    badge="$(curl -s "$BASE/item/$open_cfp_id" | grep -o '<span class="cfp-badge [^<]*' | head -n 1 | sed 's/<span class="//; s/">/: /')"
    if [ -n "$badge" ]; then
        ok "a Live conference with an open CFP shows its badge ($badge)"
    else
        bad "Live conference $open_cfp_id has an open CFP and shows no badge"
    fi
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
#
# The country is taken from what the listing can show: upcoming AND Live, because
# the gather behind /location/{country} is stage-aware and a visitor sees Live
# only. It used to be the busiest upcoming country on ANY stage, which named
# Germany in November 2026 while every German conference sat on Incoming, and so
# asked a correct page for cards it was right not to show. The demo seed puts
# conferences on Live, so a Live upcoming conference with a country is something
# the bootstrap guarantees, and its absence is a failure, not a skip.
# Space-free countries only: the value goes into a URL path unencoded.
LIVE_STAGE="0193a5a0-0000-7000-8000-000000000001"
live_upcoming="$(sql "select count(*) from item where type='conference' \
    and stage_id = '$LIVE_STAGE' \
    and fields->>'field_start_date' >= to_char(now(), 'YYYY-MM-DD')")"
country="$(sql "select fields->>'field_country' from item where type='conference' \
    and stage_id = '$LIVE_STAGE' \
    and fields->>'field_country' ~ '^[A-Za-z.-]+$' \
    and fields->>'field_start_date' >= to_char(now(), 'YYYY-MM-DD') \
    group by 1 order by count(*) desc, 1 limit 1")"
if [ -z "$live_upcoming" ]; then
    :   # Postgres is not reachable through compose; the counts are optional.
elif [ "$live_upcoming" = "0" ] || [ -z "$country" ]; then
    bad "no upcoming Live conference has a country ($live_upcoming upcoming on Live), so the demo seed did not run"
else
    ok "$live_upcoming upcoming conference(s) on Live; checking /location/$country"
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
# Every link in the main and footer menus, followed as rendered. A link in the
# site chrome that 404s is the one defect a visitor meets before any content
# loads, and "Call for Papers", "About" and "Contact" all did. Read off the page
# rather than out of demo/config, so a link that renders and a link that is
# configured cannot disagree without this noticing; each menu must render at
# least the links the config names for it.
nav_page="$(curl -s "$BASE/conferences")"
menu_links() {
    # $1 is the nav's class. Prints "href<TAB>label" per link.
    printf '%s\n' "$nav_page" | tr '\n' ' ' \
        | grep -o "<nav class=\"$1\"[^>]*>.*" | sed 's#</nav>.*##' \
        | grep -o '<a href="[^"]*"[^>]*>[^<]*</a>' \
        | sed 's/<a href="\([^"]*\)"[^>]*>\([^<]*\)<\/a>/\1	\2/; s/&#x2F;/\//g'
}
for menu in main:site-nav footer:footer-nav; do
    menu_name="${menu%%:*}"
    nav_class="${menu#*:}"
    links="$(menu_links "$nav_class")"
    configured="$(grep -l "^menu_name: $menu_name\$" "$ROOT"/demo/config/menu_link.*.yml 2>/dev/null | wc -l | tr -d ' ')"
    rendered="$(printf '%s\n' "$links" | grep -c '^/' || true)"
    if [ "$rendered" -lt "$configured" ]; then
        bad "the $menu_name menu renders $rendered link(s); demo/config names $configured"
    fi
    if [ -z "$links" ]; then
        bad "no $menu_name-menu links rendered on /conferences"
        continue
    fi
    while IFS=$'\t' read -r href label; do
        [ -n "$href" ] || continue
        status="$(code "$href")"
        if [ "$status" = "200" ]; then
            ok "$menu_name menu \"$label\" -> $href is $status"
        else
            bad "$menu_name menu \"$label\" -> $href is $status"
        fi
    done <<< "$links"
done
# The static pages carry their own content, not an empty page shell.
for page in "/about:confs.tech" "/data-sources:confs.tech" "/privacy:cookie" "/contact:issues" "/api-docs:/api/v1/conferences"; do
    page_path="${page%%:*}"
    page_want="${page#*:}"
    page_body="$(curl -s "$BASE$page_path")"
    if [[ "$page_body" == *"$page_want"* ]] && [[ "$page_body" == *'item--page'* ]]; then
        ok "$page_path renders its page, mentioning \"$page_want\""
    else
        bad "$page_path does not render its page content (looked for \"$page_want\")"
    fi
done

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
italian="$(sql "select count(*) from item where type='conference' and fields->>'field_language'='it'")"
[ -n "$italian" ] && note "conferences held in Italian: $italian"

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

head_ "submit a conference"

# The three-step submission form, driven end to end as viewer_carol: a signed-in
# member with no editorial standing, which is exactly who the form is for.
#
# Every POST here is form-urlencoded with the token in a `_token` body field and
# NO X-CSRF-Token header, because that is all a plain HTML <form> can send. If
# this passes, the form works with JavaScript switched off; there is no separate
# way to check that, because there is no JavaScript in it to switch off.
SUBMIT_PATH="/conferences/submit"

# Pull the CSRF token out of a rendered form. Single-use, so every step reads the
# one on the page it is answering.
token_from() { printf '%s' "$1" | sed -n 's/.*name="_token" value="\([^"]*\)".*/\1/p' | head -1; }
draft_from() { printf '%s' "$1" | sed -n 's/.*name="draft" value="\([^"]*\)".*/\1/p' | head -1; }

submit_anon="$(code "$SUBMIT_PATH")"
if [ "$submit_anon" = "401" ]; then
    ok "$SUBMIT_PATH is 401 to an anonymous visitor"
else
    bad "$SUBMIT_PATH answered $submit_anon to an anonymous visitor, expected 401"
fi

step1_page="$(curl -s -b "$viewer_jar" "$BASE$SUBMIT_PATH")"
if printf '%s' "$step1_page" | grep -q "Step 1 of 3"; then
    ok "$SUBMIT_PATH serves step 1 to a signed-in member"
else
    bad "$SUBMIT_PATH did not serve step 1 to a signed-in member"
fi

# A name nothing else in the demo uses, so the assertions below cannot match an
# imported conference.
SUBMITTED_NAME="Verify Demo Conference"

step2_page="$(curl -s -b "$viewer_jar" -X POST "$BASE$SUBMIT_PATH" \
    --data-urlencode "_token=$(token_from "$step1_page")" \
    --data 'step=1' --data 'draft=' \
    --data-urlencode "name=$SUBMITTED_NAME" \
    --data-urlencode 'url=https://example.org/verify' \
    --data 'start_date=2029-06-01' --data 'end_date=2029-06-03' \
    --data-urlencode 'city=Torino' --data-urlencode 'country=Italy')"
draft="$(draft_from "$step2_page")"
if printf '%s' "$step2_page" | grep -q "Step 2 of 3" && [ -n "$draft" ]; then
    ok "step 1 validates and advances, with its state on the server"
else
    bad "step 1 did not advance to step 2"
fi

review_page="$(curl -s -b "$viewer_jar" -X POST "$BASE$SUBMIT_PATH" \
    --data-urlencode "_token=$(token_from "$step2_page")" \
    --data 'step=2' --data-urlencode "draft=$draft" \
    --data-urlencode 'cfp_url=https://example.org/verify/cfp' \
    --data 'cfp_end_date=2029-04-01' \
    --data-urlencode 'description=Submitted by <b id="verify-escape">verify-demo.sh</b>.')"
if printf '%s' "$review_page" | grep -q "Step 3 of 3" \
    && printf '%s' "$review_page" | grep -q "$SUBMITTED_NAME"; then
    ok "step 2 validates and the review shows the answers back"
else
    bad "step 2 did not reach a review carrying the answers"
fi

done_page="$(curl -s -b "$viewer_jar" -X POST "$BASE$SUBMIT_PATH" \
    --data-urlencode "_token=$(token_from "$review_page")" \
    --data 'step=3' --data-urlencode "draft=$draft")"
if printf '%s' "$done_page" | grep -q "not on the site yet"; then
    ok "step 3 confirms, and says the submission is awaiting moderation"
else
    bad "step 3 did not confirm the submission"
fi

# It landed where an editor will find it, and nowhere a visitor will.
submitted_stage="$(sql "select stage_id from item where type='conference' and title='$SUBMITTED_NAME' limit 1")"
if [ "$submitted_stage" = "0193a5a0-0000-7000-8000-000000000002" ]; then
    ok "the submitted conference is on Incoming"
else
    bad "the submitted conference is on stage '${submitted_stage:-nothing}', expected Incoming"
fi

submitted_author="$(sql "select u.name from item i join users u on u.id = i.author_id where i.type='conference' and i.title='$SUBMITTED_NAME' limit 1")"
if [ "$submitted_author" = "viewer_carol" ]; then
    ok "the submitter is recorded as the author (viewer_carol)"
else
    bad "the submitted conference is authored by '${submitted_author:-nobody}', expected viewer_carol"
fi

# A member's description is text, and the page an editor opens to review it
# prints it as text. The form stores it as a string, render_blocks hands a string
# back unsanitized (FRICTION.md, G-RENDER-BLOCKS-PASSES-STRINGS-UNSANITIZED), and
# the conference template used to mark that output safe, so this markup rendered
# live for the editor. The template now escapes a string description.
submitted_id="$(sql "select id from item where type='conference' and title='$SUBMITTED_NAME' limit 1")"
submitted_page="$(curl -s -b "$editor_jar" "$BASE/item/$submitted_id")"
if printf '%s' "$submitted_page" | grep -q '<b id="verify-escape">'; then
    bad "the submitted description's markup rendered raw on /item/$submitted_id for an editor"
elif printf '%s' "$submitted_page" | grep -q '&lt;b id=&quot;verify-escape&quot;&gt;\|&lt;b id=&#34;verify-escape&#34;&gt;\|&lt;b id="verify-escape"&gt;'; then
    ok "the submitted description renders as escaped text for an editor"
else
    bad "the submitted description is not on /item/$submitted_id for an editor"
fi

submitted_lang="$(sql "select coalesce(fields->>'field_language', '') || '/' || coalesce(language, '') from item where type='conference' and title='$SUBMITTED_NAME' limit 1")"
if [ "$submitted_lang" = "en/en" ]; then
    ok "the submitted conference carries its detected language (en)"
else
    bad "the submitted conference's field_language/language is '${submitted_lang:-nothing}', expected en/en"
fi

# A CFP that closes after the conference ends is refused, on the step that
# collects it. This is the brief's rule, enforced where something is allowed to
# say no: ritrovo_cfp's presave tap can only report it
# (FRICTION.md, G-PRESAVE-CANNOT-REFUSE).
bad_start="$(curl -s -b "$viewer_jar" "$BASE$SUBMIT_PATH")"
bad_step2="$(curl -s -b "$viewer_jar" -X POST "$BASE$SUBMIT_PATH" \
    --data-urlencode "_token=$(token_from "$bad_start")" \
    --data 'step=1' --data 'draft=' \
    --data-urlencode 'name=Late CFP Check' \
    --data 'start_date=2029-06-01' --data 'end_date=2029-06-03' \
    --data-urlencode 'country=Italy')"
bad_draft="$(draft_from "$bad_step2")"
refused="$(curl -s -b "$viewer_jar" -X POST "$BASE$SUBMIT_PATH" \
    --data-urlencode "_token=$(token_from "$bad_step2")" \
    --data 'step=2' --data-urlencode "draft=$bad_draft" \
    --data 'cfp_end_date=2029-06-04')"
if printf '%s' "$refused" | grep -q "cannot close after"; then
    ok "a CFP closing after the conference ends is refused on step 2"
else
    bad "a CFP closing after the conference ends was accepted"
fi

# And a step posted without a token is refused by the kernel, before the plugin
# is dispatched at all.
notoken="$(curl -s -b "$viewer_jar" -o /dev/null -w '%{http_code}' -X POST "$BASE$SUBMIT_PATH" \
    --data 'step=1' --data 'draft=' --data-urlencode 'name=No Token' \
    --data 'start_date=2029-06-01' --data 'end_date=2029-06-03' \
    --data-urlencode 'country=Italy')"
if [ "$notoken" = "403" ]; then
    ok "a submission step with no CSRF token is 403"
else
    bad "a submission step with no CSRF token answered $notoken, expected 403"
fi

head_ "comments"

# Story 37.1, on the released image, where the kernel's own comment templates
# are the ones the site serves. The host-in-the-loop suite drives the same
# routes but configures no TEMPLATES_DIR, so the rendered thread — the form, the
# depth class, the login prompt — can only be checked here.
#
# What this really checks is `demo/config`: `post comments` belongs to
# `ritrovo_access` and is granted to the authenticated role, which is a thing no
# role could hold before Trovato 0.103.0 dispatched `tap_perm`.
COMMENT_CONF="$(sql "SELECT id FROM item WHERE type = 'conference' AND status = 1 ORDER BY created LIMIT 1")"

if [ -z "$COMMENT_CONF" ]; then
    bad "no published conference to comment on"
else
    # Matched with `case`, never `printf | grep -q`. Under `pipefail` a `grep -q`
    # that exits on its first match can SIGPIPE the printf still writing a
    # 13KB page, and the pipeline's failure is indistinguishable from "not
    # found" — the same trap that once failed the Italian check for a page that
    # was correct.
    anon_page="$(curl -s "$BASE/item/$COMMENT_CONF")"
    case "$anon_page" in
        *'to post a comment'*) ok "an anonymous visitor is asked to log in rather than shown a form" ;;
        *) bad "an anonymous visitor was not offered the login prompt" ;;
    esac
    case "$anon_page" in
        *'comment-form'*) bad "an anonymous visitor was shown the comment form" ;;
        *) ok "an anonymous visitor is shown no comment form" ;;
    esac

    member_page="$(curl -s -b "$viewer_jar" "$BASE/item/$COMMENT_CONF")"
    case "$member_page" in
        *'comment-form'*) ok "a signed-in member is shown the comment form" ;;
        *) bad "a signed-in member was not shown the comment form; is 'post comments' granted?" ;;
    esac

    # The kernel's comment form posts `_csrf`, not the `_token` a plugin route
    # takes. Two names, one session store.
    csrf_from() { printf '%s' "$1" | sed -n 's/.*name="_csrf" value="\([^"]*\)".*/\1/p' | head -1; }

    COMMENT_BODY="Verify demo comment $$"
    REPLY_BODY="Verify demo reply $$"

    curl -s -b "$viewer_jar" -o /dev/null -X POST "$BASE/api/item/$COMMENT_CONF/comments" \
        --data-urlencode "_csrf=$(csrf_from "$member_page")" \
        --data 'parent_id=' \
        --data-urlencode "body=$COMMENT_BODY"

    parent="$(sql "SELECT id FROM comment WHERE item_id = '$COMMENT_CONF' AND body LIKE 'Verify demo comment%' ORDER BY created DESC LIMIT 1")"
    if [ -n "$parent" ]; then
        ok "a signed-in member's comment is stored"
    else
        bad "a signed-in member's comment was not stored"
    fi

    if [ -n "$parent" ]; then
        member_page="$(curl -s -b "$viewer_jar" "$BASE/item/$COMMENT_CONF")"
        curl -s -b "$viewer_jar" -o /dev/null -X POST "$BASE/api/item/$COMMENT_CONF/comments" \
            --data-urlencode "_csrf=$(csrf_from "$member_page")" \
            --data "parent_id=$parent" \
            --data-urlencode "body=$REPLY_BODY"

        reply_depth="$(sql "SELECT depth FROM comment WHERE item_id = '$COMMENT_CONF' AND parent_id = '$parent' LIMIT 1")"
        if [ "$reply_depth" = "1" ]; then
            ok "a reply is nested one level under its parent"
        else
            bad "the reply landed at depth '$reply_depth', expected 1"
        fi

        thread_page="$(curl -s -b "$viewer_jar" "$BASE/item/$COMMENT_CONF")"
        case "$thread_page" in
            *'comment--depth-1'*) ok "the reply renders with the kernel's own depth class" ;;
            *) bad "the reply did not render at depth 1 on the page" ;;
        esac
        case "$thread_page" in
            *"$COMMENT_BODY"*) ok "the comment renders under the conference" ;;
            *) bad "the comment does not render under the conference" ;;
        esac
        case "$thread_page" in
            *"$REPLY_BODY"*) ok "the reply renders under the conference" ;;
            *) bad "the reply does not render under the conference" ;;
        esac
    fi

    # Story 37.2. At 0.102 this screen called require_admin and no grant could
    # reach it (G-ADMIN-SCREENS-ARE-ADMIN-ONLY); at 0.103.0 it asks for
    # `administer comments`, which demo/config grants to the editorial roles.
    moderation_editor="$(code_with "$editor_jar" /admin/content/comments)"
    if [ "$moderation_editor" = "200" ]; then
        ok "editor_alice can open the comment moderation queue"
    else
        bad "the moderation queue answered $moderation_editor to editor_alice, expected 200"
    fi
    moderation_viewer="$(code_with "$viewer_jar" /admin/content/comments)"
    if [ "$moderation_viewer" = "403" ] || [ "$moderation_viewer" = "302" ]; then
        ok "viewer_carol cannot open the comment moderation queue"
    else
        bad "the moderation queue answered $moderation_viewer to viewer_carol, expected 403"
    fi
fi

head_ "subscriptions"

# Story 37.3 as far as this kernel allows it. The toggle is NOT on the
# conference page and cannot be: the item template's context carries no viewer,
# so a control there would be shown to visitors it would refuse, and a
# tap_item_view toggle renders into `children`, which this repository's
# conference template drops to keep the raw field dump off the page. So the
# control lives on the member's own page, reached from the user menu, which the
# kernel filters per viewer. FRICTION.md, G-ITEM-TEMPLATE-HAS-NO-VIEWER.
viewer_uid="$(sql "SELECT id FROM users WHERE name = 'viewer_carol'")"
SUBS_PATH="/user/$viewer_uid/subscriptions"

subs_anon="$(code "$SUBS_PATH")"
if [ "$subs_anon" = "401" ]; then
    ok "$SUBS_PATH is 401 to an anonymous visitor"
else
    bad "$SUBS_PATH answered $subs_anon to an anonymous visitor, expected 401"
fi

conf_page="$(curl -s "$BASE/item/$COMMENT_CONF")"
case "$conf_page" in
    *'ritrovo-subscribe-form'*) bad "a subscribe control reached an anonymous conference page" ;;
    *) ok "no subscribe control is offered to an anonymous visitor" ;;
esac

subs_page="$(curl -s -b "$viewer_jar" "$BASE$SUBS_PATH?conference=$COMMENT_CONF")"
case "$subs_page" in
    *'>Subscribe'*) ok "a signed-in member is offered the Subscribe control" ;;
    *) bad "a signed-in member was not offered the Subscribe control" ;;
esac

curl -s -b "$viewer_jar" -o /dev/null -X POST "$BASE$SUBS_PATH/subscribe" \
    --data-urlencode "_token=$(token_from "$subs_page")" \
    --data "item_id=$COMMENT_CONF"

subscribed="$(sql "SELECT COUNT(*) FROM user_subscriptions WHERE user_id = '$viewer_uid' AND item_id = '$COMMENT_CONF'")"
if [ "$subscribed" = "1" ]; then
    ok "the subscription lands in the kernel's own user_subscriptions table"
else
    bad "the subscription did not land; user_subscriptions holds $subscribed row(s)"
fi

subs_page="$(curl -s -b "$viewer_jar" "$BASE$SUBS_PATH")"
case "$subs_page" in
    *'>Unsubscribe'*) ok "the list offers Unsubscribe for a conference already followed" ;;
    *) bad "the list did not offer Unsubscribe" ;;
esac

# One member's list is not another's.
other_uid="$(sql "SELECT id FROM users WHERE name = 'editor_alice'")"
subs_other="$(code_with "$viewer_jar" "/user/$other_uid/subscriptions")"
if [ "$subs_other" = "403" ]; then
    ok "another member's subscription list is 403"
else
    bad "another member's list answered $subs_other, expected 403"
fi

curl -s -b "$viewer_jar" -o /dev/null -X POST "$BASE$SUBS_PATH/unsubscribe" \
    --data-urlencode "_token=$(token_from "$subs_page")" \
    --data "item_id=$COMMENT_CONF"

still="$(sql "SELECT COUNT(*) FROM user_subscriptions WHERE user_id = '$viewer_uid' AND item_id = '$COMMENT_CONF'")"
if [ "$still" = "0" ]; then
    ok "unsubscribing removes the row"
else
    bad "the subscription survived unsubscribing; $still row(s) left"
fi

# The permission the routes are gated on is one the kernel knows about, which is
# the 0.103.0 change this whole plugin waited on.
declared="$(sql "SELECT plugin FROM plugin_permission WHERE name = 'manage own subscriptions'")"
if [ "$declared" = "ritrovo_notify" ]; then
    ok "tap_perm's declaration reached the kernel's plugin_permission table"
else
    bad "'manage own subscriptions' is declared by '$declared', expected ritrovo_notify"
fi

head_ "translations"

# A8: content in two languages through the kernel's own translation model. The
# Italian seed is fifteen conferences written in English with an
# item_translation row each in Italian; the kernel overlays the row under /it/.
#
# The conference used is fixed by the seed: its two titles differ, so a page
# showing one or the other says which language it rendered in.
TR_ID="37c34d73-22b2-43ab-ae98-611aeed7a1c0"
TR_EN="National Testing Day 2026"
TR_IT="Giornata Nazionale del Testing 2026"
TR_ALIAS_EN="/conferences/national-testing-day-2026"
TR_ALIAS_IT="/it/conferenze/giornata-nazionale-del-testing-2026"
html_lang() { printf '%s' "$1" | grep -o '<html lang="[^"]*"' | head -1 | sed 's/.*lang="//; s/"//'; }
title_of() { printf '%s' "$1" | sed -n 's/.*<h1 class="item__title">\([^<]*\)<\/h1>.*/\1/p' | head -1; }

translations="$(sql "select count(*) from item_translation t join item i on i.id = t.item_id where i.type = 'conference'")"
if [ -n "$translations" ]; then
    if [ "$translations" -ge 15 ]; then
        ok "$translations conference translation(s) in item_translation (the seed ships 15)"
    else
        bad "only ${translations} conference translation(s) in item_translation; the seed ships 15"
    fi
fi
leftover="$(sql "select count(*) from item where type='conference' and (fields ? 'primary_language' or fields ? 'translation_status' or jsonb_typeof(fields->'field_description') = 'object')")"
if [ -n "$leftover" ]; then
    if [ "$leftover" = "0" ]; then
        ok "no conference carries the old two-languages-in-one-field shape"
    else
        bad "$leftover conference(s) still carry primary_language, translation_status or a per-language description"
    fi
fi

en_page="$(curl -s "$BASE/item/$TR_ID")"
it_page="$(curl -s "$BASE/it/item/$TR_ID")"
if [ "$(title_of "$en_page")" = "$TR_EN" ] && [ "$(html_lang "$en_page")" = "en" ]; then
    ok "/item/$TR_ID renders \"$TR_EN\" with lang=\"en\""
else
    bad "/item/$TR_ID rendered \"$(title_of "$en_page")\" with lang=\"$(html_lang "$en_page")\", want \"$TR_EN\" / en"
fi
if [ "$(title_of "$it_page")" = "$TR_IT" ] && [ "$(html_lang "$it_page")" = "it" ]; then
    ok "/it/item/$TR_ID renders its translation \"$TR_IT\" with lang=\"it\""
else
    bad "/it/item/$TR_ID rendered \"$(title_of "$it_page")\" with lang=\"$(html_lang "$it_page")\", want \"$TR_IT\" / it"
fi
if [[ "$it_page" == *'Sessioni su test automation'* ]] && [[ "$en_page" == *'Sessions on test automation'* ]]; then
    ok "the description is translated too, not only the title"
else
    bad "the description did not follow the language (English page or Italian page shows the wrong text)"
fi

alias_en="$(curl -s "$BASE$TR_ALIAS_EN")"
alias_it="$(curl -s "$BASE$TR_ALIAS_IT")"
if [ "$(title_of "$alias_en")" = "$TR_EN" ]; then
    ok "$TR_ALIAS_EN resolves to the conference"
else
    bad "$TR_ALIAS_EN did not resolve to \"$TR_EN\" (got \"$(title_of "$alias_en")\")"
fi
if [ "$(title_of "$alias_it")" = "$TR_IT" ] && [ "$(html_lang "$alias_it")" = "it" ]; then
    ok "$TR_ALIAS_IT resolves to the conference, in Italian"
else
    bad "$TR_ALIAS_IT did not resolve to \"$TR_IT\" in Italian (got \"$(title_of "$alias_it")\", lang=\"$(html_lang "$alias_it")\")"
fi

# The switcher, followed both ways from the English alias: it must offer the
# other language only, and the round trip must land back on the same item.
switch_href() { printf '%s' "$1" | sed -n "s/.*href=\"\\([^\"]*\\)\"[^>]*class=\"lang-switcher__link lang-switcher__link--$2\".*/\\1/p" | head -1; }
to_it="$(switch_href "$alias_en" it)"
if [ -z "$to_it" ]; then
    bad "the English page offers no switcher link to Italian"
elif [ -n "$(switch_href "$alias_en" en)" ]; then
    bad "the English page's switcher offers English, the language it is already in"
else
    via_it="$(curl -s "$BASE$to_it")"
    back="$(switch_href "$via_it" en)"
    via_en="$(curl -s "$BASE$back")"
    if [ "$(title_of "$via_it")" = "$TR_IT" ] && [ -n "$back" ] && [ "$(title_of "$via_en")" = "$TR_EN" ] \
        && [ -z "$(switch_href "$via_it" it)" ]; then
        ok "switcher round trip: $TR_ALIAS_EN -> $to_it -> $back, the same conference both ways"
    else
        bad "switcher round trip broke: $to_it gave \"$(title_of "$via_it")\", back link \"$back\" gave \"$(title_of "$via_en")\""
    fi
fi

# The listing pages under /it/. /it/argomenti renders lang="en" and that is the
# kernel's: its alias points at /topics, a handler that never reads the request
# language (FRICTION.md, G-TOPICS-PAGE-IGNORES-LANGUAGE). Reported, not asserted.
for path in /it/relatori /it/argomenti; do
    note "$path renders lang=\"$(html_lang "$(curl -s "$BASE$path")")\""
done

# A translated conference on the /it/ listing: the gather overlays the title but
# merges the fields with the original winning, so the card's description stays in
# the original language (FRICTION.md, G-GATHER-TRANSLATION-MERGE-IS-BACKWARDS).
# Reported, not asserted: the fix is the kernel's, and which seeded conference is
# still upcoming changes with the date.
it_listing="$(curl -s "$BASE/it/conferenze")"
listed_tr="$(sql_text "select t.item_id || '|' || left(t.fields->'field_description'->0->'data'->>'text', 40) || '|' || left(i.fields->'field_description'->0->'data'->>'text', 40) from item_translation t join item i on i.id = t.item_id where t.language = 'it' and i.stage_id = '0193a5a0-0000-7000-8000-000000000001' and i.fields->>'field_start_date' >= to_char(now(), 'YYYY-MM-DD') order by i.fields->>'field_start_date' limit 1")"
if [ -n "$listed_tr" ]; then
    lt_id="${listed_tr%%|*}"; lt_rest="${listed_tr#*|}"; lt_it="${lt_rest%%|*}"; lt_en="${lt_rest#*|}"
    if printf '%s' "$it_listing" | grep -qF "$lt_it"; then
        note "/it/conferenze shows the Italian description of $lt_id"
    elif printf '%s' "$it_listing" | grep -qF "$lt_en"; then
        note "/it/conferenze shows the ORIGINAL description of translated $lt_id (G-GATHER-TRANSLATION-MERGE-IS-BACKWARDS)"
    else
        note "translated $lt_id is not on the first page of /it/conferenze"
    fi
fi

# The translation queue: refused to a visitor, refused to a signed-in reader,
# served to an editor. Anonymous is 401 rather than 403 for the same reason as
# the editorial screen above: the kernel answers a plugin route's permission
# gate with UNAUTHORIZED when there is no session at all.
QUEUE_PATH="/admin/content/translations"
q_anon="$(code "$QUEUE_PATH")"
q_viewer="$(code_with "$viewer_jar" "$QUEUE_PATH")"
q_editor="$(code_with "$editor_jar" "$QUEUE_PATH")"
if [ "$q_anon" = "401" ] && [ "$q_viewer" = "403" ] && [ "$q_editor" = "200" ]; then
    ok "$QUEUE_PATH — anonymous $q_anon, viewer $q_viewer, editor $q_editor"
else
    bad "$QUEUE_PATH — anonymous $q_anon, viewer $q_viewer, editor $q_editor (want 401 / 403 / 200)"
fi

# The workflow end to end, as editor_alice: take the first conference off the
# queue, open the kernel's translate screen it links to, save a translation
# there, see it render under the prefix and leave the queue, then delete it
# through the kernel's own delete route so the demo is left as it was found.
queue_page="$(curl -s -b "$editor_jar" "$BASE$QUEUE_PATH")"
# In the admin layout, not served bare. The kernel cannot render a plugin page
# under /admin into its own admin template (FRICTION.md,
# G-THEMED-PLUGIN-PAGE-UNDER-ADMIN-RENDERS-UNWRAPPED); a path template in this
# repository supplies the one missing variable.
if printf '%s' "$queue_page" | grep -q 'class="admin-content"'; then
    ok "the queue renders inside the admin layout"
else
    bad "the queue was served without the admin layout"
fi
translate_href="$(printf '%s' "$queue_page" | sed -n 's/.*class="translation-queue__translate" *href="\([^"]*\)".*/\1/p' | head -1)"
if [ -z "$translate_href" ]; then
    bad "the translation queue lists nothing to translate (no upcoming conference without one?)"
else
    q_id="$(printf '%s' "$translate_href" | sed 's#/admin/content/\([^/]*\)/translate/.*#\1#')"
    q_lang="${translate_href##*/}"
    prefix="/$q_lang"; [ "$q_lang" = "en" ] && prefix=""
    screen="$(curl -s -b "$editor_jar" -c "$editor_jar" "$BASE$translate_href")"
    if [[ "$screen" == *'name="title"'* ]]; then
        ok "the queue links the kernel's translate screen, and it renders for an editor ($translate_href)"
    else
        bad "the kernel's translate screen at $translate_href did not render a form for the editor"
    fi
    marker="Verifica traduzione $(date +%s)"
    saved="$(curl -s -b "$editor_jar" -c "$editor_jar" -o /dev/null -w '%{http_code}' -X POST "$BASE$translate_href" \
        --data-urlencode "_token=$(token_from "$screen")" \
        --data-urlencode "_form_build_id=$(printf '%s' "$screen" | sed -n 's/.*name="_form_build_id" value="\([^"]*\)".*/\1/p' | head -1)" \
        --data-urlencode "title=$marker")"
    # As the editor: the queue lists conferences on every stage, and one on
    # Incoming or Curated is 404 to a visitor by design.
    shown="$(title_of "$(curl -s -b "$editor_jar" "$BASE$prefix/item/$q_id")")"
    still_queued="$(curl -s -b "$editor_jar" "$BASE$QUEUE_PATH" | grep -c "data-item=\"$q_id\"" || true)"
    if [ "$saved" = "303" ] && { [ "$shown" = "$marker" ] || [ "$q_lang" = "en" ]; } && [ "$still_queued" = "0" ]; then
        ok "saved through the kernel screen: $prefix/item/$q_id renders it and the queue drops the conference"
    else
        bad "translate-and-save: POST answered $saved, $prefix/item/$q_id shows \"$shown\", still queued: $still_queued"
    fi
    # The delete form is on the translate screen itself, below the save form,
    # once a translation exists; its token is the page's last one.
    delete_screen="$(curl -s -b "$editor_jar" -c "$editor_jar" "$BASE$translate_href")"
    delete_token="$(printf '%s' "$delete_screen" | sed -n 's/.*name="_token" value="\([^"]*\)".*/\1/p' | tail -1)"
    curl -s -b "$editor_jar" -c "$editor_jar" -o /dev/null -X POST "$BASE$translate_href/delete" \
        --data-urlencode "_token=$delete_token"
    remaining="$(sql "select count(*) from item_translation where item_id = '$q_id' and language = '$q_lang'")"
    [ -n "$remaining" ] && [ "$remaining" != "0" ] && bad "the verification translation of $q_id was not deleted"
fi

translate_perm="$(sql "select count(*) from role_permissions rp join roles r on r.id = rp.role_id where r.name = 'editor' and rp.permission = 'translate content'")"
if [ -n "$translate_perm" ]; then
    if [ "$translate_perm" = "1" ]; then ok "the editor role holds translate content"; else bad "the editor role does not hold translate content"; fi
fi

head_ "the JSON API"

# docs/API.md, checked against the running site. /api/v1/conferences is a URL
# alias onto the listing's own gather; /api/v1/conferences/{id} and
# /api/v1/topics are ritrovo_importer's; /api/v1/search is the kernel's. Every
# request here is anonymous unless it says otherwise.
LIVE_STAGE="0193a5a0-0000-7000-8000-000000000001"
INCOMING_STAGE="0193a5a0-0000-7000-8000-000000000002"
total_of() { printf '%s' "$1" | grep -o '"total":[0-9]*' | head -1 | sed 's/.*://'; }
pages_of() { printf '%s' "$1" | grep -o '"total_pages":[0-9]*' | head -1 | sed 's/.*://'; }
# Every page of a list request, concatenated. $1 is the path with its query.
all_pages() {
    first="$(curl -s "$BASE$1")"
    printf '%s' "$first"
    n="$(pages_of "$first")"
    p=2
    while [ -n "$n" ] && [ "$p" -le "$n" ]; do
        case "$1" in *\?*) sep='&' ;; *) sep='?' ;; esac
        curl -s "$BASE$1${sep}page=$p"
        p=$((p + 1))
    done
}

api_type="$(curl -s -o /dev/null -w '%{http_code} %{content_type}' "$BASE/api/v1/conferences")"
api_list="$(curl -s "$BASE/api/v1/conferences")"
api_total="$(total_of "$api_list")"
case "$api_type" in
    "200 application/json"*)
        if [ -n "$api_total" ] && [ "$api_total" -gt 0 ]; then
            ok "/api/v1/conferences answers JSON with $api_total upcoming conference(s)"
        else
            bad "/api/v1/conferences answered JSON with no conferences in it"
        fi ;;
    *) bad "/api/v1/conferences answered '$api_type', expected 200 application/json" ;;
esac

listed_upcoming="$(sql "select count(*) from item where type='conference' and status=1 and stage_id='$LIVE_STAGE' and fields->>'field_start_date' >= to_char(now(), 'YYYY-MM-DD')")"
if [ -n "$listed_upcoming" ] && [ "$api_total" = "$listed_upcoming" ]; then
    ok "it lists exactly the Live upcoming conferences ($listed_upcoming)"
else
    bad "/api/v1/conferences lists ${api_total:-nothing}; the database has ${listed_upcoming:-?} Live upcoming"
fi

api_all="$(all_pages /api/v1/conferences)"
other_stages="$(printf '%s' "$api_all" | grep -o '"stage_id":"[^"]*"' | grep -vc "$LIVE_STAGE")"
if [ "$other_stages" = "0" ]; then
    ok "every conference it lists is Live"
else
    bad "$other_stages conference(s) on /api/v1/conferences are not Live"
fi

# The proof that matters: a non-Live conference that WOULD match is absent.
hidden_id="$(sql "select id from item where type='conference' and stage_id='$INCOMING_STAGE' and fields->>'field_start_date' >= to_char(now(), 'YYYY-MM-DD') order by fields->>'field_start_date' limit 1")"
if [ -z "$hidden_id" ]; then
    bad "no upcoming Incoming conference to prove the API withholds"
else
    if printf '%s' "$api_all" | grep -q "$hidden_id"; then
        bad "Incoming conference $hidden_id is on /api/v1/conferences for an anonymous caller"
    else
        ok "an upcoming Incoming conference ($hidden_id) is absent from every page"
    fi
    staged="$(curl -s "$BASE/api/v1/conferences?stage=$INCOMING_STAGE")"
    if [ "$(total_of "$staged")" = "0" ]; then
        ok "asking for the Incoming stage anonymously returns nothing"
    else
        bad "?stage=Incoming returned $(total_of "$staged") conference(s) to an anonymous caller"
    fi
    hidden_code="$(code "/api/v1/conferences/$hidden_id")"
    if [ "$hidden_code" = "404" ]; then
        ok "/api/v1/conferences/{id} is 404 for that Incoming conference"
    else
        bad "/api/v1/conferences/$hidden_id answered $hidden_code, expected 404"
    fi
fi

page_two="$(curl -s "$BASE/api/v1/conferences?page=2")"
if [ "$(pages_of "$api_list")" -ge 2 ] 2>/dev/null; then
    if printf '%s' "$page_two" | grep -q '"page":2' && printf '%s' "$page_two" | grep -q '"has_prev":true'; then
        ok "it pages: page 2 says so and has a previous page"
    else
        bad "?page=2 did not return page 2"
    fi
else
    note "one page of conferences, so paging is not exercised"
fi

# The four filters, each against the database's own count.
api_country="$(sql_text "select fields->>'field_country' from item where type='conference' and status=1 and stage_id='$LIVE_STAGE' and fields->>'field_start_date' >= to_char(now(), 'YYYY-MM-DD') and coalesce(fields->>'field_country','') <> '' group by 1 order by count(*) desc, 1 limit 1")"
if [ -n "$api_country" ]; then
    want="$(sql "select count(*) from item where type='conference' and status=1 and stage_id='$LIVE_STAGE' and fields->>'field_start_date' >= to_char(now(), 'YYYY-MM-DD') and fields->>'field_country' = '$api_country'")"
    got="$(total_of "$(curl -s -G "$BASE/api/v1/conferences" --data-urlencode "fields.field_country=$api_country")")"
    if [ "$got" = "$want" ]; then ok "country filter: $got in $api_country"; else bad "country filter for $api_country returned ${got:-nothing}, expected $want"; fi
fi
want="$(sql "select count(*) from item where type='conference' and status=1 and stage_id='$LIVE_STAGE' and fields->>'field_start_date' >= to_char(now(), 'YYYY-MM-DD') and fields->'field_online' = 'true'::jsonb")"
got="$(total_of "$(curl -s "$BASE/api/v1/conferences?fields.field_online=true")")"
if [ "$got" = "$want" ]; then ok "online filter: $got online"; else bad "online filter returned ${got:-nothing}, expected $want"; fi
want="$(sql "select count(*) from item where type='conference' and status=1 and stage_id='$LIVE_STAGE' and fields->>'field_start_date' >= to_char(now(), 'YYYY-MM-DD') and fields->>'field_language' = 'it'")"
got="$(total_of "$(curl -s "$BASE/api/v1/conferences?fields.field_language=it")")"
if [ "$got" = "$want" ]; then ok "language filter: $got in Italian"; else bad "language filter returned ${got:-nothing}, expected $want"; fi
api_topic="$(sql "select t.value from item i, jsonb_array_elements_text(i.fields->'field_topics') t where i.type='conference' and i.status=1 and i.stage_id='$LIVE_STAGE' and i.fields->>'field_start_date' >= to_char(now(), 'YYYY-MM-DD') group by 1 order by count(*) desc limit 1")"
if [ -n "$api_topic" ]; then
    want="$(sql "select count(distinct i.id) from item i where i.type='conference' and i.status=1 and i.stage_id='$LIVE_STAGE' and i.fields->>'field_start_date' >= to_char(now(), 'YYYY-MM-DD') and exists (select 1 from jsonb_array_elements_text(i.fields->'field_topics') t where t.value::uuid in (with recursive d(id) as (select '$api_topic'::uuid union select h.tag_id from category_tag_hierarchy h join d on h.parent_id = d.id) select id from d))")"
    got="$(total_of "$(curl -s "$BASE/api/v1/conferences?fields.field_topics=$api_topic")")"
    if [ "$got" = "$want" ]; then ok "topic filter: $got under topic $api_topic"; else bad "topic filter for $api_topic returned ${got:-nothing}, expected $want"; fi
fi

# One conference, with its public fields and nothing an editor wrote.
noted_id="$(sql "select id from item where type='conference' and status=1 and stage_id='$LIVE_STAGE' and coalesce(fields->>'field_editor_notes','') <> '' limit 1")"
if [ -n "$noted_id" ]; then
    one="$(curl -s "$BASE/api/v1/conferences/$noted_id")"
    one_code="$(code "/api/v1/conferences/$noted_id")"
    if [ "$one_code" = "200" ] && printf '%s' "$one" | grep -q '"stage":"live"' && printf '%s' "$one" | grep -q "\"id\":\"$noted_id\""; then
        ok "/api/v1/conferences/{id} serves a Live conference"
    else
        bad "/api/v1/conferences/$noted_id answered $one_code: $(printf '%s' "$one" | head -c 120)"
    fi
    if printf '%s' "$one" | grep -q 'field_editor_notes'; then
        bad "/api/v1/conferences/$noted_id carries the editor's notes"
    else
        ok "and leaves out the editor's notes it has"
    fi
fi
bad_id_code="$(code "/api/v1/conferences/not-a-uuid")"
if [ "$bad_id_code" = "404" ]; then ok "/api/v1/conferences/not-a-uuid is 404"; else bad "/api/v1/conferences/not-a-uuid answered $bad_id_code"; fi

topics_body="$(curl -s "$BASE/api/v1/topics")"
topic_tags="$(sql "select count(*) from category_tag where category_id='topics'")"
if [ "$(total_of "$topics_body")" = "$topic_tags" ] && printf '%s' "$topics_body" | grep -q '"children":\[{'; then
    ok "/api/v1/topics returns all $topic_tags topics as a tree"
else
    bad "/api/v1/topics did not return the $topic_tags topics nested: $(printf '%s' "$topics_body" | head -c 120)"
fi

# Search is the kernel's, Live only. A seeded Live conference is found; an
# Incoming conference's own title finds nothing of it.
search_live="$(curl -s -G "$BASE/api/v1/search" --data-urlencode 'q=Codemotion')"
if printf '%s' "$search_live" | grep -q '"title":"Codemotion Roma 2026"'; then
    ok "/api/v1/search?q=Codemotion finds the Live seed conference"
else
    bad "/api/v1/search?q=Codemotion did not find Codemotion Roma 2026: $(printf '%s' "$search_live" | head -c 120)"
fi
if [ -n "$hidden_id" ]; then
    hidden_title="$(sql_text "select title from item where id='$hidden_id'")"
    search_hidden="$(curl -s -G "$BASE/api/v1/search" --data-urlencode "q=$hidden_title")"
    if printf '%s' "$search_hidden" | grep -q "$hidden_id"; then
        bad "/api/v1/search returns Incoming conference $hidden_id"
    else
        ok "/api/v1/search does not return the Incoming conference"
    fi
fi

# Tokens: issued by the kernel, refused by the kernel. Reported, not asserted
# as a pass: a token that authenticates nothing is FRICTION.md's
# G-API-TOKENS-NEVER-AUTHENTICATE, and this line will say so until it changes.
token_csrf="$(curl -s -b "$editor_jar" "$BASE/user/profile" | sed -n 's/.*name="csrf-token" content="\([^"]*\)".*/\1/p' | head -1)"
token_json="$(curl -s -b "$editor_jar" -X POST "$BASE/api/tokens" -H 'Content-Type: application/json' -H "X-CSRF-Token: $token_csrf" -d '{"name":"verify-demo"}')"
api_token="$(printf '%s' "$token_json" | sed -n 's/.*"token":"\([^"]*\)".*/\1/p')"
token_id="$(printf '%s' "$token_json" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')"
if [ -n "$api_token" ]; then
    ok "an editor can create an API token at /api/tokens"
    bearer_code="$(curl -s -o /dev/null -w '%{http_code}' -H "Authorization: Bearer $api_token" "$BASE/api/v1/topics")"
    note "that token, sent as a bearer to /api/v1/topics, answers $bearer_code (G-API-TOKENS-NEVER-AUTHENTICATE)"
    curl -s -o /dev/null -b "$editor_jar" -X DELETE -H "X-CSRF-Token: $token_csrf" "$BASE/api/tokens/$token_id"
else
    bad "/api/tokens issued no token to an editor: $(printf '%s' "$token_json" | head -c 120)"
fi

# The kernel's ad-hoc gather route, which Ritrovo cannot close: an anonymous
# definition that selects the whole `fields` column skips field access. Reported
# so the day it stops leaking is visible here (G-ADHOC-GATHER-BYPASSES-FIELD-ACCESS).
adhoc="$(curl -s -X POST "$BASE/api/gather/query" -H 'Content-Type: application/json' \
    -d '{"definition":{"base_table":"item","item_type":"conference","fields":[{"field_name":"id"},{"field_name":"fields"}],"filters":[],"sorts":[],"relationships":[],"includes":{},"stage_aware":true},"display":{"format":"table","items_per_page":100,"pager":{"enabled":false,"show_count":false,"style":"full"}}}')"
if printf '%s' "$adhoc" | grep -q 'field_editor_notes'; then
    note "an anonymous POST to /api/gather/query still returns editors' notes (G-ADHOC-GATHER-BYPASSES-FIELD-ACCESS)"
else
    note "an anonymous POST to /api/gather/query no longer returns editors' notes; re-check G-ADHOC-GATHER-BYPASSES-FIELD-ACCESS"
fi

openapi_paths="$(curl -s "$BASE/api/openapi.json" | grep -o '"/api/v1/[a-z/{}]*"' | sort -u | tr '\n' ' ')"
case "$openapi_paths" in
    *conferences*) note "/api/openapi.json lists Ritrovo's conference routes" ;;
    *) note "/api/openapi.json lists none of Ritrovo's routes (G-OPENAPI-IS-A-STATIC-LIST)" ;;
esac

printf '\n'
if [ "$failures" -eq 0 ]; then
    echo "all checks passed"
else
    echo "$failures check(s) failed" >&2
fi
exit "$failures"
