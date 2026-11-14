# Caching on the Ritrovo demo

What is cached when a page is served, what empties each cache, and the commands
that show a hit and a miss. Everything here is checked against Trovato 0.103.0 at
the pinned revision. Every claim about invalidation is also asserted:

- by `scripts/verify-demo.sh`, section "search and the cache, by stage";
- by the host test
  `a_kernel_edit_shows_at_once_a_raw_write_waits_for_the_cache_and_no_view_leaks`
  in `plugins/ritrovo_access/tests/ritrovo_access_host.rs`.

The commands assume the demo is running from `scripts/serve-demo.sh` on
`http://localhost:3000`; set `BASE` for another port.

```sh
BASE=http://localhost:3000
psql_demo() { docker compose -f docker-compose.demo.yml exec -T postgres psql -U trovato -d ritrovo -tAc "$1"; }
```

## What is cached

| Cache | Where | Keyed by | Lifetime | Emptied by |
| :- | :- | :- | :- | :- |
| **Item cache** | in process, `ItemService` (`crates/kernel/src/content/item_service.rs:323-331`) | item uuid | `CACHE_TTL_ITEMS`: 300 s by default, **10 s on the demo** (`docker-compose.demo.yml`) | `ItemService::update` (`:736`), `delete` (`:910`) and expiry. Not by a raw SQL write. |
| Cache layer | L1 in process, L2 Redis (`crates/kernel/src/cache/mod.rs`) | string key, with tags | per entry | tag and stage invalidation. Ritrovo reads nothing through it; the kernel's only reader on this site is AI search expansion, which is off. |
| Static files | the browser | URL | `Cache-Control: public, max-age=86400` on `/static/*`, including the Pagefind index | a new deployment of the file. The index is rebuilt by cron when Live content is newer than the last build. |
| Rendered pages | **none** | | | Every page, gather listing and tile is rendered per request. No HTML is cached on the server, and a page carries no `Cache-Control`. |

The item cache holds **Items, not pages, and not decisions about who may see
them**. The access check (`tap_item_access`, which is how `ritrovo_access` keeps
Incoming and Curated from visitors) runs on every request after the cache is
read. So one viewer's read can never reach another viewer. Nothing rendered is
shared, and the cached item is checked again for each viewer.

## A hit and a miss, by hand

The item cache logs nothing and counts nothing: `/metrics` registers
`cache_hits_total` and `cache_misses_total`, and nothing increments them. So a
hit shows as an answer the database no longer agrees with, and a miss as the
database's answer.

```sh
ID=$(psql_demo "select id from item where type='conference' and status=1 and stage_id='0193a5a0-0000-7000-8000-000000000001' order by title limit 1")
TITLE=$(psql_demo "select title from item where id='$ID'")

curl -s "$BASE/item/$ID" -o /dev/null                    # MISS: loads the item and caches it
psql_demo "update item set title = title || ' (raw)' where id='$ID'"
curl -s "$BASE/item/$ID" | grep -c "(raw)"               # HIT: 0, the cached title
sleep 11                                                  # the demo's TTL is 10 s
curl -s "$BASE/item/$ID" | grep -c "(raw)"               # MISS: 1, reloaded from the database
psql_demo "update item set title = '$TITLE' where id='$ID'"
```

And the counters that never move:

```sh
curl -s "$BASE/metrics" | grep '^cache_'
# cache_hits_total_total 0
# cache_misses_total_total 0
```

## After a kernel edit, and after an importer update

**A kernel edit is on the page at once.** The edit form at `/item/{id}/edit`,
the admin content form and `ItemService` in general invalidate the item's entry
when they write. Edit a Live conference's title as `admin` and reload its page:
the new title is there on the first request.

**An importer update is not, for up to `CACHE_TTL_ITEMS`.** `ritrovo_importer`
writes conferences with raw SQL through `host::execute_raw`, and so do
`ritrovo_forms`' submission form, `ritrovo_importer`'s search text and tiles, and
config import. No host call reaches `ItemService::invalidate`, and a plugin's
cache keys are namespaced under the plugin's own name, so it cannot reach the
kernel's entries even by key (FRICTION.md,
`G-PLUGIN-CANNOT-INVALIDATE-THE-ITEM-CACHE`). The commands above are this path.

**What Ritrovo does about it.** It sets `CACHE_TTL_ITEMS: 10` on the demo, which
bounds the staleness at ten seconds. That bounds the gap and does not fix it: on
a default install the same import is invisible on the item's own page for up to
five minutes. Listings, tiles and search are not affected, because they read the
database or the index, never the item cache. There is nothing more Ritrovo can do
without a host call.

## An editor's view never reaches a visitor

```sh
CURATED=$(psql_demo "select id from item where type='conference' and stage_id='0193a5a0-0000-7000-8000-000000000003' order by title limit 1")
curl -s -c /tmp/editor -X POST "$BASE/user/login/json" -H 'Content-Type: application/json' \
     -d '{"username":"editor_alice","password":"ritrovo-editor-demo"}' >/dev/null
curl -s -o /dev/null -w '%{http_code}\n' -b /tmp/editor "$BASE/item/$CURATED"   # 200, and now cached
curl -s -o /dev/null -w '%{http_code}\n' "$BASE/item/$CURATED"                   # 403 or 404, never 200
```

## What a request profile shows

Every response carries `Server-Timing: total;dur=<ms>`. The kernel layers
`track_request_timing` outermost (`crates/kernel/src/main.rs:410-415`), and it logs
a request slower than the configured threshold.

```sh
curl -sI "$BASE/conferences" | grep -i server-timing
```

The header carries one number: no query count, no cache hits, no breakdown.
Per-query timing "would require wrapping the `PgPool` and is not done"
(`crates/kernel/src/middleware/query_profiler.rs:47-50`). Nothing called Gander
exists. The brief's "cache walkthrough demonstrated via Gander profiling" has no
profiler to run, and `G-NO-REQUEST-PROFILER` stays open.

The cache layer does log at debug level. Restart the demo with

```sh
RUST_LOG='info,sqlx=warn,tower_http=warn,trovato_kernel::cache=debug' scripts/serve-demo.sh
docker compose -f docker-compose.demo.yml logs -f trovato | grep -E 'cache (L1|L2) hit|cache set|invalidated'
```

On this site that stream stays quiet apart from stage invalidations, because
nothing Ritrovo serves goes through that layer. That is itself a finding: the
page a visitor loads is never served from a cache that could be inspected.
