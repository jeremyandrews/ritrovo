# Ritrovo's JSON API

Read-only, public, JSON. Every response below was captured with `curl` against the demo
(`scripts/serve-demo.sh`, kernel 0.103.0) on 2026-11-14 and re-checked on Trovato `main`
`81290cc` on 2026-11-23 (A11), and `scripts/verify-demo.sh` checks each claim on this page
against a running site. Long arrays are cut where marked.

| Endpoint | Served by | Why |
|---|---|---|
| `GET /api/v1/conferences` | **a gather** (`ritrovo.upcoming_conferences`) behind a URL alias | The kernel already pages, filters and access-checks a gather. The alias, `demo/config/url_alias.47c640fe-….yml`, gives its JSON route the published address. |
| `GET /api/v1/conferences/{id}` | **plugin** (`ritrovo_importer`, `tap_api`) | A gather has a fixed address and takes arguments as query parameters, and an alias maps one fixed path to another. Neither can carry an id in the path. |
| `GET /api/v1/topics` | **plugin** (`ritrovo_importer`, `tap_api`) | The kernel returns the topic tags flat with no parents (`/api/category/topics/tags`), or the roots alone. A client would need one request per node to build the tree. |
| `GET /api/v1/search?q=` | **the kernel** (`crates/kernel/src/routes/api_v1.rs:71`) | Nothing to add. |

There is no write endpoint. Through 0.103 a plugin could write `item` only by raw SQL,
which skips the kernel's access checks, revisions and taps. On the pinned kernel a
plugin's item write from a request goes through `ItemService`, so a write endpoint is
buildable and not built (`docs/ritrovo/STATUS.md`, 38.5).

## What an anonymous caller sees

**Live, published conferences only.** For the list and search, the kernel decides: a
gather and a search run against the Live stage and pass every row through the item
access taps (`ritrovo_access`'s `tap_item_access`), whatever stage is asked for. The
detail endpoint selects only Live, published rows itself (`plugins/ritrovo_importer/src/api.rs`,
`CONFERENCE_SQL`). Checked on the demo by `verify-demo.sh`:

- an upcoming Incoming conference appears on no page of `/api/v1/conferences`;
- `/api/v1/conferences?stage=<Incoming uuid>` returns `"total":0` to an anonymous caller;
- `/api/v1/conferences/{that id}` is 404, the same answer as for an id that does not exist;
- a search for that conference's own title does not return it.

**No editor's notes.** On the list, `ritrovo_access`'s `tap_field_access` removes
`field_editor_notes` from every row for anyone but an editor. The detail endpoint copies
out an allowlist of public fields (`PUBLIC_FIELDS`), so it never selects the notes, and a
field added to the type later stays private until it is listed.

> **The kernel's ad-hoc query route no longer leaks them.** Through 0.103,
> `POST /api/gather/query` answered anonymous callers and did not field-filter a
> definition that selected the whole `fields` column, so editors' notes were readable
> there (`G-ADHOC-GATHER-BYPASSES-FIELD-ACCESS`). On the pinned kernel it needs
> `administer site` and answers an anonymous caller 401. `verify-demo.sh` asserts that
> no notes come back.

## `GET /api/v1/conferences`

Upcoming conferences (start date today or later), soonest first, 25 to a page. This is
the same gather `/conferences` renders, so the page and the API always agree.

| Parameter | Example | Meaning |
|---|---|---|
| `page` | `page=2` | 1-based. |
| `fields.field_topics` | `fields.field_topics=5a4988ec-…` | A topic **tag uuid** (see `/api/v1/topics`). Matches the topic and every topic under it. |
| `fields.field_country` | `fields.field_country=Germany` | Exact match on the stored value. |
| `fields.field_online` | `fields.field_online=true` | `true` or `false`, parsed as JSON. |
| `fields.field_language` | `fields.field_language=it` | ISO 639-1 code of the language the conference is held in. |

The parameter names are the field paths, because the kernel keys a gather's exposed
filters by field (`crates/kernel/src/gather/gather_service.rs:1174`). **An unknown
parameter is ignored, not refused**: `?country=Germany` returns everything.

```console
$ curl -s 'http://localhost:3000/api/v1/conferences?fields.field_language=it'
```
```json
{
    "data": [
        {
            "author_id": "00000000-0000-0000-0000-000000000000",
            "changed": 1741824000,
            "comment_count": 0,
            "created": 1741824000,
            "current_revision_id": null,
            "fields": {
                "field_city": "Brescia",
                "field_country": "IT",
                "field_description": [
                    {
                        "data": {
                            "text": "Italian Agile Days is the Italian conference on agile and lean methodologies. Two days of sessions on Scrum, Kanban, DevOps, and digital transformation in Brescia."
                        },
                        "type": "paragraph"
                    }
                ],
                "field_end_date": "2026-11-15",
                "field_language": "it",
                "field_online": false,
                "field_source_id": "italian-agile-days-2026-2026-11-14-brescia",
                "field_speakers": ["e005a307-597a-5ae3-b0aa-38cfac177d34", "0201cb8e-7b4d-590f-9193-a303d9514f35"],
                "field_start_date": "2026-11-14",
                "field_topics": ["324db1b0-5e62-5b62-838d-c260c9dfece5"],
                "field_url": "https://www.agileday.it"
            },
            "id": "66997692-1471-4aeb-8e8b-c5b91273951c",
            "item_group_id": "01a1da38-e6e7-76a3-925d-365ee244318f",
            "language": "en",
            "promote": 0,
            "retention_days": null,
            "search_vector": "'2026':4A 'agil':2A,9B,16B …",
            "stage_id": "0193a5a0-0000-7000-8000-000000000001",
            "status": 1,
            "sticky": 0,
            "tenant_id": "0193a5a0-0001-7000-8000-000000000001",
            "title": "Italian Agile Days 2026",
            "type": "conference",
            "url": "/conferences/italian-agile-days-2026"
        }
    ],
    "total": 1,
    "page": 1,
    "per_page": 25
}
```

Every kernel list is this shape since the kernel's one API convention (Trovato
`3b86b5a`): the rows in `data`, then `total`, `page` and `per_page`. There is no
`total_pages`, `has_next` or `has_prev`. The page count is `ceil(total / per_page)`,
and a client stops when `page * per_page >= total`. (Captured at 0.103.0, when the rows
were under `items`. The row itself is unchanged.)

Each row is the kernel's whole `item` row. **Use `id`, `title`, `language`, `fields`,
`url`, `created` and `changed`; ignore the rest.** `search_vector`, `tenant_id`,
`item_group_id`, `retention_days` and friends are kernel internals that a gather selecting
`item.*` cannot leave out. A gather that names its columns instead returns every JSON value
as text (`"false"`, a JSON array inside a string) and loses `url` to the kernel's own
(`G-GATHER-EXPLICIT-COLUMNS-ARE-TEXT`), and selecting the whole `fields` column skips
field access (above), so `item.*` is the only shape that is both typed and safe.

`language` is the language the stored record is written in, always the site default here;
`fields.field_language` is the language the conference is **held** in. `url` is the
conference's alias when it has one, otherwise `/item/{id}`.

Under `/it/` (`/it/api/v1/conferences`) the kernel overlays Italian translations. Today
that changes the title only: the gather merges a translation's fields with the original
winning (`G-GATHER-TRANSLATION-MERGE-IS-BACKWARDS`).

`fields.field_online` is a JSON boolean for imported conferences. A conference submitted
through `/conferences/submit` or saved through the kernel's edit form stores `"1"` or
`"0"` instead (`G-FRONTEND-ITEM-FORM-STORES-EVERY-FIELD-AS-A-STRING`), and
`fields.field_online=true` does not match those.

## `GET /api/v1/conferences/{id}`

One Live, published conference, upcoming or past. 404 for anything else: an unknown id,
a malformed one, an item that is not a conference, or a conference on any other stage.
The answer does not say which.

```console
$ curl -s http://localhost:3000/api/v1/conferences/031e0e55-59d2-4f4c-a2e2-81c50bce5b62
```
```json
{
    "changed": 1741824000,
    "created": 1741824000,
    "fields": {
        "field_city": "Roma",
        "field_country": "IT",
        "field_description": [
            {
                "data": {
                    "text": "Codemotion is the largest tech conference in Italy. Two days of talks, workshops, and networking with over 2,000 developers from across Europe."
                },
                "type": "paragraph"
            }
        ],
        "field_end_date": "2026-03-26",
        "field_language": "it",
        "field_online": false,
        "field_speakers": ["e005a307-597a-5ae3-b0aa-38cfac177d34", "0201cb8e-7b4d-590f-9193-a303d9514f35"],
        "field_start_date": "2026-03-25",
        "field_topics": ["8ad812c0-2305-5c02-adca-dffcd1f4a996"],
        "field_url": "https://www.codemotion.com"
    },
    "id": "031e0e55-59d2-4f4c-a2e2-81c50bce5b62",
    "language": "en",
    "stage": "live",
    "title": "Codemotion Roma 2026",
    "translations": ["it"],
    "type": "conference",
    "url": "/conferences/codemotion-roma-2026"
}
```

```console
$ curl -s -i http://localhost:3000/api/v1/conferences/00000000-0000-0000-0000-00000000dead
HTTP/1.1 404 Not Found
{"error":"not found"}
```

The fields are the public ones only: dates, place, `field_online`, `field_language`, the
three URLs and the CFP close date, the description as Editor.js blocks, and topic and
speaker uuids. `translations` lists the languages with a translation, each readable at
`/{lang}/item/{id}`.

## `GET /api/v1/topics`

The topic taxonomy as a tree. `total` counts distinct tags. A tag the hierarchy gives two
parents appears under each. `url` is the topic's listing page.

```console
$ curl -s http://localhost:3000/api/v1/topics
```
```json
{
    "data": [
        {
            "children": [
                {
                    "children": [
                        {"children": [], "id": "2b01a107-36e3-5435-ab6f-0a679ebb4f9f", "label": "Rust", "slug": "rust", "url": "/topics/rust", "weight": 0}
                    ],
                    "id": "057dd875-cf7e-5f34-99bc-15d8e7c0a529",
                    "label": "Systems",
                    "slug": "systems",
                    "url": "/topics/systems",
                    "weight": 0
                },
                "…"
            ],
            "id": "5a4988ec-5ef6-5235-8f03-c906f2184d74",
            "label": "Languages",
            "slug": "languages",
            "url": "/topics/languages",
            "weight": 0
        },
        "…"
    ],
    "total": 44
}
```

## `GET /api/v1/search?q=`

The kernel's full-text search over Live content (`crates/kernel/src/routes/api_v1.rs:129`),
access-filtered like everything else. `page` and `per_page` (1 to 100, default 25) page
it, and an empty `q` returns no results. It searches every content type: filter on
`type` for conferences.

```console
$ curl -s 'http://localhost:3000/api/v1/search?q=Codemotion'
```
```json
{
    "data": [
        {
            "id": "031e0e55-59d2-4f4c-a2e2-81c50bce5b62",
            "snippet": "<mark>Codemotion</mark> Roma 2026 ",
            "title": "Codemotion Roma 2026",
            "type": "conference"
        }
    ],
    "total": 1,
    "page": 1,
    "per_page": 25
}
```

`snippet` contains `<mark>` markup; escape the rest of it before rendering.

## Authentication and tokens

None is needed: everything above is public. A signed-in session cookie makes the
**kernel's** endpoints answer for that user: on the demo, `editor_alice` sees
`field_editor_notes` in `/api/v1/conferences` and gets Incoming conferences from
`?stage=0193a5a0-0000-7000-8000-000000000002`, as the editorial roles allow. The two plugin
endpoints answer the same for everyone: Live only, public fields only.

The kernel issues API tokens to signed-in users: `POST /api/v1/tokens` with a JSON body
`{"name": "…"}` and the session's CSRF token in `X-CSRF-Token` answers
`201 {"id","name","token","expires_at"}`. `GET /api/v1/tokens` lists them and
`DELETE /api/v1/tokens/{id}` revokes one. The CSRF token is single-use, so read a fresh one
before the revoke. A token authenticates a request on its own:
`Authorization: Bearer <token>` makes the request the token owner's, with no session.
`verify-demo.sh` gets 200 from `GET /api/v1/tokens` with a token and 401 with none or a
forged one. Through 0.103 every bearer was refused (`G-API-TOKENS-NEVER-AUTHENTICATE`,
fixed). An invalid or revoked token is still a 401, so do not send a stale one to a
public endpoint.

## Rate limits

The kernel's `api` bucket covers every path under `/api/`, including `/api/v1/search`:
**100 requests a minute**, counted per client IP, and again per user for a signed-in
session (`crates/kernel/src/middleware/rate_limit.rs:179`, `:491-494`, `:655`). The
window is fixed at a minute. The count is **not** fixed, though the brief assumed it was:
an operator can set it for the whole site with `TROVATO_RATE_LIMIT_API=<n>` in the
environment or `rate_limit.api` in site configuration, the environment winning
(`rate_limit.rs:248-292`, line numbers at 0.103.0). On the pinned kernel a limit can also
be set per role, `TROVATO_RATE_LIMIT_API__ANONYMOUS_USER=<n>` and the like
(`crates/kernel/src/middleware/rate_limit.rs:210`, `:403-421`), so the brief's 60 for
anonymous and 300 for authenticated clients can be expressed. The demo does not set it:
buildable, not built (`docs/ritrovo/STATUS.md`, 38.6). Over the limit:

```
HTTP/1.1 429 Too Many Requests
retry-after: <seconds>
content-type: application/json

{"code":"rate_limited","message":"Rate limit exceeded for requests. Retry after <seconds>s.","request_id":"…"}
```

(From the kernel's `rate_limit_response` and `AppError`, `rate_limit.rs:847` and
`error.rs:309`. The demo is not driven over the limit to capture this.)

## OpenAPI

`/api/v1/openapi.json` is generated from the served routes (Trovato `226fa5d`,
`3b86b5a`) and lists the two plugin routes, `/api/v1/conferences/{id}` and
`/api/v1/topics`, which `verify-demo.sh` asserts. It does not list the alias
`/api/v1/conferences`, whose parameters are a gather's, and it says nothing about stages or
field access. This page is the reference. Through 0.103 the document was a hand-written
list (`G-OPENAPI-IS-A-STATIC-LIST`, fixed).

## Errors

| Status | Body | When |
|---|---|---|
| 404 | `{"error":"not found"}` | `/api/v1/conferences/{id}` for anything but a Live, published conference. |
| 401 | `{"code":"unauthorized","message":"Invalid or expired API token",…}` | An `Authorization: Bearer` header with a token that is not live. |
| 429 | `{"code":"rate_limited","message":…,"request_id":…}` | Over the `api` bucket. |
| 500 | `{"code":"internal_error",…}` | The kernel's answer to a gather it cannot find (`G-UNKNOWN-GATHER-IS-A-500`). Not reachable through the addresses on this page. |
