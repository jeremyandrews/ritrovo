# Ritrovo: Status Against the Brief

Every promise in the design brief, set against the running demo and the kernel it
runs on, with the prompt in the build series that closes it.

**Audited:** 2026-09-17. Ritrovo `main` at `9d5dd35` (CI green on that commit).

**Partly superseded, 2026-09-18.** Two pull requests have landed against this
audit. The first fixed the three defects a visitor met on the demo: the blank
search page, the raw field dump on every conference page and the 404 "Call for
Papers" link. The second moved the configuration set into `demo/config` and built
the brief's content model on it. Rows they closed say **done (A4)** with the
evidence; rows they proved impossible on this kernel say **blocked** and name the
`FRICTION.md` entry. Three of the audit's own conclusions were wrong and are
corrected below, each marked **[corrected]**.
Trovato `v0.102.0`, `rev 20baa121810b5c656b3f80028335770069fab5e0`, the image
`ghcr.io/jeremyandrews/trovato:0.102.0` the demo pins.

**Brief:** `docs/ritrovo/overview.md`; `epic-01.md` to `epic-09.md`;
`epic-production-ready.md`. Epics 10 to 19, their summary and the dependency graph
describe kernel infrastructure and are out of scope.

**How it was checked.** The demo was stood up from empty volumes with
`scripts/serve-demo.sh`, and every one of its own checks passed:
5,656 conferences, 158 upcoming, 263 import batches drained. It was then driven in
Google Chrome (headless, through Playwright), as an anonymous visitor and as the
demo administrator: page status, rendered text, forms, failed resources and
screenshots. One comment, one reply and one edit were made on the conference
"Gerrit User Summit" to find out whether comments, revisions and revert work; they
are still in that demo database. Kernel claims cite a file and line at the pinned
revision, with paths relative to the Trovato checkout. Paths under `plugins/`,
`scripts/`, `demo/` and `docs/` are this repository's. Gaps in the kernel are
entries in [`FRICTION.md`](../../FRICTION.md), named here by their `G-` id.

## Status values

| Status | Meaning |
| :- | :- |
| **done** | Works in the running demo; the evidence says where it was seen. |
| **kernel has it, Ritrovo does not use it** | The pinned kernel provides it; the evidence names the feature and the file. Ritrovo has not switched it on, configured it or built on it. |
| **Ritrovo must build it** | Nothing in the kernel stops it; the work is configuration, plugin code or templates in this repository. |
| **blocked on the kernel** | Cannot be built on the pinned kernel without a kludge. The evidence names the `FRICTION.md` entry. |

A row that is partly one and partly another takes the status of whatever stops it
being done, and says what else is true.

**Where it would live:** `config` (kernel configuration, in Ritrovo's
`demo/config`), `plugin` (plugin code), `template` (Tera templates and CSS), or
`kernel` (a kernel feature used as it is).

**Series prompts:** A4 content model, A5 editorial, A6 forms, A7 community, A8
global and API, A9 layout and search.

**Row ids:** stories keep their BMAD numbers (29.1 to 39.7); Epic 4's stories,
which carry none of their own, are E4.1 to E4.5; the "What It Demonstrates" table
is D1 to D25 in its order; the intended taps in the brief's "Plugins" section are
P1 to P19.

## Tally

| Status | Stories (63) | What It Demonstrates (25) | Plugin taps (19) | All (107) |
| :- | -: | -: | -: | -: |
| done | 9 | 1 | 4 | 14 |
| kernel has it, Ritrovo does not use it | 21 | 5 | 0 | 26 |
| Ritrovo must build it | 8 | 5 | 6 | 19 |
| blocked on the kernel | 25 | 14 | 9 | 48 |

---

## What changes the plan

1. **Pull request #6 is not on `main`.** "Stop the pretend features from
   pretending" merged at 13:43 into `host-in-the-loop-tests`, five minutes after
   that branch had itself merged into `main` as #5. Its changes are stranded: on
   `main`, `ritrovo_notify` still renders a Subscribe button that does nothing on
   every conference page and registers a `/user/subscriptions` entry that 404s, and
   its three `FRICTION.md` entries were missing (they are re-verified and included in
   this pull request). This audit is of `main`. Re-landing #6's code changes is the
   first thing to do before A4.
2. **The demo's search page is blank in a browser.** `/search?q=rust` requests
   `/static/pagefind/pagefind.js`, gets 404 (`trovato_search` is not enabled, so no
   index exists), and the search script clears the 37 server results the HTML
   carried. `scripts/verify-demo.sh` greps the HTML, so its search check passes for a
   page no visitor can use. `G-SEARCH-PAGE-BLANK-WITHOUT-INDEX`.
3. **A conference edit loses data.** The tutorial's `conference` type declares no
   `field_topics`. After one save through `/item/{id}/edit`, the item's
   `field_topics` key was gone, so the conference left every topic listing. (Its
   `field_online` key went too; that field is declared, and an unchecked box saves
   as absent.) Until the content model is fixed (A4), no editorial work in A5 or A6
   is safe to demonstrate.
4. **Revisions 500 once there is one.** After that edit, `/item/{id}/revisions`
   answered 500. `G-REVISION-HISTORY-500`. Imported conferences have no revisions at
   all, because the importer writes raw SQL.
5. **Permissions are the single largest blocker.** `tap_perm` is not dispatched and
   config import refuses plugin permissions, so no role holds any Ritrovo
   permission, and the permission grid revokes any granted by SQL.
   `G-PERM-TAP-NOT-DISPATCHED`, `G-PERM-GRID-SAVE-REVOKES-PLUGIN-GRANTS`.
6. **Nothing moves an item between stages.** `G-NO-ITEM-STAGE-TRANSITION`. This is
   why the importer's landing stage cannot simply be changed (see "The importer's
   stage").

## Where the content model lives

**It moved, 2026-09-18.** The whole set is `demo/config/` in this repository: 94
files, imported as one unit by `scripts/demo-bootstrap.sh` before any plugin is
enabled, and imported by the host-in-the-loop suites too, so
`tests/host/tutorial-config/` is retired. The kernel image still ships
`docs/tutorial/config/` and its tutorial still imports it;
`scripts/check-tutorial-templates.sh` now compares the kernel's copy against
Ritrovo's and **warns without failing** when they differ, which they do and are
meant to. It becomes an error, or goes away, when the kernel stops shipping a
copy. The kernel-side half is its own pull request.

The rest of this section is the audit's reasoning for the move, kept because it
is still why the set has to travel together.

**Before the move.** `conference` and `speaker`, the `topics` category and its 32 tags, the
six gathers, the roles, the stages, the menu links, the tiles, the aliases, the
search field configs, the languages, the Italian seed and the locale file are all
in the kernel's `docs/tutorial/config/` (79 entries). The image ships that
directory (`Dockerfile:72`), and the demo imports it from inside the image before
it enables the plugins (`scripts/demo-bootstrap.sh:131-132`), then the seed
(`:150`), then this repository's own two-file set (`:153`).
`tests/host/tutorial-config/` holds a byte-identical 38-file subset, and
`scripts/check-tutorial-templates.sh` fails CI when it drifts from the release.

**It does not match the brief.** Compared field by field with
`overview.md`, "Content Model":

- `conference` has no `topics` (the importer writes it undeclared, and a form save
  drops it), no `speakers`, no `schedule_pdf`; `venue_photos` is a single
  `field_venue_photo`; `description` is `Blocks`, not `filtered_html`; nothing makes
  a logo required for Live.
- `speaker` has `field_photo` for `headshot`, an extra `field_company`, and stores a
  forward `field_conferences` reference where the brief computes a reverse one.
- There is no `comment` item type; the kernel's comments are their own table,
  which serves the brief as well.
- The topic tree is three levels only under Languages, and differs in terms: Go, C,
  Scala, Clojure, Swift, the Functional group, Kubernetes, Cloud, Observability, AI,
  Machine Learning, LLMs, GraphQL, WebAssembly, Privacy and Cryptography are absent;
  .NET, Android, iOS, Testing, API, General and Open Source are extra; Kotlin is not
  cross-listed.
- Gathers: "Conferences This Month" and "CFPs Closing Soon" do not exist; their
  tiles reuse the upcoming and open-CFP queries. By Location is two gathers. The
  upcoming pager is 20, not 25.
- Roles: `viewer`, `editor`, `publisher`, holding kernel permissions only.

**Recommendation: move it into `demo/config/`.** The type definitions, the
category and tags, the gathers, the roles, the stages, the workflow variable, the
search field configs, the tiles, the menu links, the aliases, the pathauto
patterns, the languages, the seed and the locale file become Ritrovo's, versioned
with the plugins that depend on them, and the brief's model is built there. The
kernel tutorial points at a pinned Ritrovo tag for its Ritrovo parts.

They have to move together. Import validates references against the set and then
the database and rejects the whole set on any miss
(`crates/kernel/src/config_storage/yaml.rs:517-521`, `:775-784`): search field
configs name the `conference` and `speaker` bundles, the seed needs the type
(`item.type` is a foreign key), and tiles, aliases, pathauto and the workflow name
gathers, types and stages.

**What breaks in the kernel if it moves.**

- `cargo test -p trovato-kernel --lib` stops compiling:
  `crates/kernel/src/config_storage/yaml.rs:1776` does
  `include_str!("../../../../docs/tutorial/config/item_type.conference.yml")`.
- `crates/kernel/tests/config_import_test.rs:101-102` reads the directory, and
  `tutorial_config_set_imports_clean_on_a_fresh_database` (`:275`) and
  `tutorial_config_set_is_idempotent` (`:366`) assert its counts: 3 roles, 4
  stages, 5 tiles, 6 menu links.
- The tutorial: Part 1 builds `conference` by hand in the admin UI
  (`docs/tutorial/part-01-hello-trovato.md`, "Creating the Type") and offers the
  import only as a shortcut (`:126-133`), so it survives. Parts 2, 3, 4 and 7 have
  no hand-built path: Part 2 requires the import before it starts (`:97-106`), Part
  3's speaker type, search configs, tiles and menus come only from it, Part 4's
  roles and stages come only from it, and Part 7 imports the languages and
  `seed-italian` (`:294-298`). Recipes 01, 03, 04 and 07 import it too. Part 4 also
  tells readers that editing `variable.workflow.editorial.yml` changes the workflow
  with no code (`:345-348`), which is not true (`G-NO-ITEM-STAGE-TRANSITION`).
- `Dockerfile:72` copies a directory that would no longer be the source of truth.

**What the kernel keeps.** Part 1's hand-built walkthrough; Part 8's generic
explanation of export and import; `tutorial_test.rs`, which already seeds its own
`conference` type in Rust (`crates/kernel/tests/common/mod.rs:548-590`); and a small
kernel-owned fixture (one type, a category and tag, a role, a stage, a tile, a menu
link) under `crates/kernel/tests/fixtures/` for the two import tests and the unit
test.

**Order.** Ritrovo first: copy the set into `demo/config/`, import it before
enabling plugins, retire `tests/host/tutorial-config/` and the config half of
`check-tutorial-templates.sh`. Then the kernel moves its tests onto the fixture and
its tutorial onto a pinned Ritrovo tag. Until the kernel side lands, Ritrovo's copy
and the kernel's diverge, which is the point. This is the first job of A4, before
any field is added, so that A4's model changes land in Ritrovo's copy.

## What the kernel already provides that Ritrovo does not use

Each exists at the pinned release. "In the demo" is what the running demo showed.

| Feature | Kernel evidence | In the demo |
| :- | :- | :- |
| Comments, threaded | `crates/kernel/src/routes/comment.rs:1167-1175` (routes), `crates/kernel/src/models/comment.rs:300-322` (ordered by `sort_path`), `templates/elements/comments.html:19` (indent by depth); `trovato_comments`, enabled by default | Admin posted a comment and a reply on a conference; the reply rendered at `comment--depth-1`. No other role holds `post comments`. |
| Comment moderation queue | `crates/kernel/src/routes/admin.rs:977-1003`, statuses at `models/comment.rs:36-41` | `/admin/content/comments` renders for the administrator only (`G-ADMIN-SCREENS-ARE-ADMIN-ONLY`). |
| Registration, login, logout, password reset, passkeys | `crates/kernel/src/routes/auth.rs:1621-1639`, `routes/password_reset.rs:171-178`, `routes/webauthn.rs:957-964` | Browser login works; `/user/register` is open because the tutorial set imports `allow_user_registration`. Only the administrator account exists. |
| Profiles | `/user/profile`, `crates/kernel/src/routes/auth.rs:1092-1101`: username, email, timezone, password | Renders. No bio, avatar, display name or preferences (`G-USER-PROFILE-NOT-EXTENSIBLE`). |
| Stages | `crates/kernel/src/models/stage.rs:25-46`, admin at `routes/admin_stage.rs:528-536`, gather stage filter `gather/query_builder.rs:59-80` | `/admin/structure/stages` lists Incoming, Curated, Legal Review (all "in use by nothing") and Live (5,656 items). |
| Revisions and revert | `/item/{id}/revisions` and `POST /item/{id}/revert/{rev}`, `crates/kernel/src/routes/item.rs:279-280`, `:1243-1319` | The edit wrote a revision; the history page then answered 500 (`G-REVISION-HISTORY-500`). |
| Bulk operations | `POST /admin/content/bulk`: publish, unpublish, delete, `crates/kernel/src/routes/admin_content.rs:611-673` | Present at `/admin/content` for the administrator. No stage action. |
| Form API (build, validate, submit, AJAX, `form_state_cache`) | `crates/kernel/src/form/service.rs:41-161`, `:193-222`, `:410-501` | Unreachable from any route except administrator AJAX (`G-FORM-TAPS-UNREACHABLE`). |
| WYSIWYG | Editor.js for `Blocks` fields, `templates/admin/content-form.html:164-171`, `crates/kernel/src/content/form.rs:243-290`; upload and preview gated on `trovato_block_editor` (`crates/kernel/src/routes/file.rs:30-35`), disabled by default | The conference edit form shows a "Description" label and no editor. `TextLong` fields get a plain textarea. |
| File fields, uploads, image styles | `crates/kernel/src/routes/file.rs:25-27`, `crates/kernel/src/file/service.rs:42-45` (magic bytes), `:504-602` (temporary to permanent), `crates/kernel/src/cron/tasks.rs:181-184` (cleanup); image styles `crates/kernel/src/routes/image_style.rs:18`, `trovato_image_styles` enabled, three styles seeded | Logo and venue photo file inputs render on the edit form. `/admin/content/files`: "No files found." |
| Menus and breadcrumbs | `menu_link` admin `crates/kernel/src/routes/admin_menu.rs:892-910`; main and footer injected, `routes/helpers.rs:249-261`; breadcrumbs `routes/item.rs:714-725`, `routes/gather.rs:427`; tag breadcrumb API `routes/category.rs:76` | Main and footer menus and breadcrumbs render. "Call for Papers", "About" and "Contact" are 404. |
| Tiles and slots | `crates/kernel/src/services/tile.rs:76-126`, admin `routes/tile_admin.rs:274-286`, role and path visibility `models/tile.rs:211-243` (config import only) | Five tiles placed; the two `gather_query` tiles render empty (`G-TILE-GATHER-QUERY-RENDERS-NOTHING`). |
| `mail` host interface | `crates/wit/kernel.wit:212-225`, `crates/kernel/src/host/mail.rs:10-14`: site contact only | Unused. Cannot address a user (`G-MAIL-CANNOT-REACH-A-USER`). |
| Config import and export | CLI `crates/kernel/src/main.rs:94-110`; 13 entity types in dependency order, `crates/kernel/src/config_storage/yaml.rs:51-65` | Used by the demo for import. Export unused. |
| Pagefind search | `crates/kernel/src/cron/pagefind.rs:1-8`, `plugins/trovato_search` (disabled by default), `templates/search.html:100-121`, `static/js/scolta.js` | Not enabled; `/static/pagefind/pagefind.js` is 404 and `/search` is blank in a browser. |
| tsvector search and field weights | `crates/kernel/src/search/mod.rs:58-176`, trigger `crates/kernel/migrations/20260213000002_create_search_trigger.sql`, admin `routes/admin_content_type.rs:963-994` | `/api/search?q=rust` and `/api/v1/search?q=rust` return ranked JSON; weights shown at `/admin/structure/types/conference/search`. |
| Versioned JSON API | `crates/kernel/src/routes/api_v1.rs:69-92` (search, autocomplete, media, OpenAPI), `x-api-version` header | `/api/v1/search` 200. No per-type endpoints exist in the kernel. |
| Gather and item JSON routes | `GET /api/query/{id}/execute`, `crates/kernel/src/routes/gather.rs:43-47`; `GET /api/items/{type}`, `/api/item/{id}`, `routes/item.rs:282-286`; categories `routes/category.rs:57-76` | `/api/item/{id}` and `/api/content-types` 200. |
| API tokens and bearer auth | `crates/kernel/src/routes/api_token.rs:162-163`, `crates/kernel/src/middleware/api_token.rs:1-5`; OAuth2 `routes/oauth.rs:86-88` | Unused; no token management page (`G-API-RATE-LIMITS-FIXED`). |
| Rate limiting | `crates/kernel/src/middleware/rate_limit.rs:99-120` | Active (the audit's own logins hit the 5 a minute login limit). Not per role. |
| Language negotiation | `crates/kernel/src/middleware/language.rs:91-161`: session, URL prefix, `Accept-Language` | `/it/conferenze` renders `lang="it"`. |
| Translated aliases, hreflang | `crates/kernel/src/models/url_alias.rs:25`, `middleware/path_alias.rs:68-90`; hreflang `routes/helpers.rs:961-980`, `templates/base.html:7-8` | Italian aliases work through `demo/config`. No hreflang: no translations exist. |
| Content translation overlay | `item_translation` from `trovato_content_translation`; read in `crates/kernel/src/content/item_service.rs:467-536` | 0 rows; nothing can write one (`G-TRANSLATION-NO-WRITE-PATH`). |
| Tag-based caching | `crates/kernel/src/cache/mod.rs:8-20`, `:84-208` (moka L1, Redis L2, tag sets, stage-scoped keys) | Invisible from a browser. The importer's raw SQL bypasses item cache invalidation. |
| Cron lock, queue drain | `crates/kernel/src/cron/mod.rs:35-45`, `:606-626`; `MAX_QUEUE_ITEMS_PER_CYCLE = 100` at `:45` | In use: the cron poker drains the import. |
| Metrics and health | `crates/kernel/src/routes/metrics.rs:12-39`, `routes/health.rs:17-56` | `/metrics` and `/health` 200. |
| Pathauto | `crates/kernel/src/services/pathauto.rs:64-204`, admin `routes/admin_pathauto.rs:300-310` | Configured for no type; conference pages are `/item/{uuid}`. |
| Subscriptions table | `crates/kernel/migrations/20260315000001_create_user_subscriptions.sql`, `crates/kernel/src/models/subscription.rs:22-76`; no caller | Unused. |
| AI: providers, `ai_request`, budgets, chat tile, assistant taps, MCP server, vector store, search expand and summarize | `routes/admin_ai_provider.rs`, `crates/wit/kernel.wit` `ai-api`, `routes/admin_ai_budget.rs`, `routes/api_chat.rs:433`, `services/ai_assistant.rs`, `crates/mcp-server`, `services/vector_store.rs:56`, `routes/api_search.rs:25-30` | `trovato_ai` disabled, no provider configured. |
| Others a conference site would use | sitemap `routes/sitemap.rs:155-156`; RSS per gather `routes/feed.rs:156-162`; SEO meta and JSON-LD `content/page_meta.rs`, `trovato_seo`; scheduled publishing `trovato_scheduled_publishing`; redirects `middleware/redirect.rs`; contact form `trovato_contact` (disabled); spam classifier `trovato_spam` (disabled); data export and account deletion `routes/auth.rs:1635-1638`, `routes/user_delete.rs:599-603` | `/sitemap.xml` and `/robots.txt` 200. The rest unused. |

## What the kernel cannot do yet

Every gap below is an entry in [`FRICTION.md`](../../FRICTION.md) with evidence,
impact and a recommendation, and names the rows it blocks.

| Entry | Severity | Blocks |
| :- | :- | :- |
| `G-PERM-TAP-NOT-DISPATCHED` | High | 35.2, 36.6, 37.2, 37.3, D3, D16, P17, P18 |
| `G-PERM-GRID-SAVE-REVOKES-PLUGIN-GRANTS` | High | the only workaround for the entry above |
| `G-REVISION-HISTORY-500` | High | 35.4, D3, D20 |
| `G-NO-ITEM-STAGE-TRANSITION` | High | 35.3, 36.4, 39.2, D3, D25, P3 |
| `G-TRANSLATION-NO-WRITE-PATH` | High | 38.1, 38.3, 38.4, D21, P15 |
| `G-ITEM-API-BYPASSES-ITEM-SERVICE` | High | 38.5, 37.4, P6, P9, P13 |
| `G-ITEM-INSERT-OUTPUT-DISCARDED` | Medium | 38.3, 36.5 |
| `G-FORM-TAPS-UNREACHABLE` | Medium | 36.1, 36.4, 36.7, D6, D22, P16 |
| `G-MAIL-CANNOT-REACH-A-USER` | Medium | 37.4, 37.6, D10, P11, P12 |
| `G-MAIL-UNAVAILABLE-IN-BACKGROUND` | Medium | 37.4, D10, P11, P12 |
| `G-QUEUE-NO-CROSS-PLUGIN` | Medium | 37.5, 36.5, D9, P6 |
| `G-TILE-GATHER-QUERY-RENDERS-NOTHING` | Medium | 34.4, D12 |
| `G-SEARCH-PAGE-BLANK-WITHOUT-INDEX` | Medium | E4.5, 34.5, D14 |
| `G-ADMIN-SCREENS-ARE-ADMIN-ONLY` | Medium | 37.2, 39.2, D25 |
| `G-USER-PROFILE-NOT-EXTENSIBLE` | Medium | 36.7, D15 |
| `G-BATCH-NO-EXECUTOR` | Medium | 39.2, D25 |
| `G-PRESAVE-CANNOT-REFUSE` | Medium | 36.5, P6 |
| `G-NO-REQUEST-LANGUAGE` | Medium | 38.3, P14 |
| `G-LOCALE-STRINGS-NEVER-LOADED` | Medium | 38.2, D21 |
| `G-DEFAULT-STAGE-IGNORED-ON-CREATE` | Medium | 36.4, 35.3 |
| `G-NO-USER-DIRECTORY` | Medium | 37.4, 37.6 |
| `G-USER-API-NO-ADMIN-BYPASS` | Medium | P8, P19 once they check the viewer |
| `G-AJAX-ADMIN-ONLY-NO-CONDITIONAL-FIELDS` | Low | 36.3, D23 |
| `G-TUTORIAL-CONFIG-SET-DEFECTS` | Low | 29.1, E4.3, D18, 38.2 |
| `G-QUEUE-WORKER-ERROR-IS-SUCCESS` | Low | P3 |
| `G-FILE-NO-HOST-API` | Low | 36.4, 36.7 |
| `G-PLUGIN-ROUTE-NO-HEADERS` | Low | 37.3 |
| `G-API-RATE-LIMITS-FIXED` | Low | 38.6 |
| `G-S3-STORAGE-REMOVED` | Low | 39.3, D17 |
| `G-NO-REQUEST-PROFILER` | Low | D24 |
| `G-REVISION-NO-COMPARE` | Low | 35.4, D20 |
| `G-SEARCH-NO-ADMIN-OR-ANALYTICS` | Low | 30.3, 30.6, 30.7 |
| `G-VIEW-TAP-INPUT-CARRIES-NO-VIEWER` | Low | nothing (documentation) |

Two points from the brief's own list, settled:

- **`ritrovo_access` is Ritrovo work, as A2 found.** A view tap can reach the viewer
  through `current-user-id` and `current-user-has-permission` once the plugin
  declares `user-api`, and hiding `editor_notes` is a `tap_field_access` job, which
  receives the viewer and removes fields before any view tap runs
  (`crates/kernel/src/content/item_service.rs:572-573`, `:1222-1236`). What does
  block it is `tap_perm`: its permissions can be granted to no role.
- **`tap_form_alter` is in the WIT** (`crates/wit/kernel.wit:331`) and in
  `KNOWN_TAPS` (`crates/kernel/src/plugin/info_parser.rs:311`), and is dispatched
  only by `FormService::build`, which no route calls. It is effectively absent.

## The importer's stage

**Imports land on Live, now as on 2026-08-17.** `insert_conference` binds
`LIVE_STAGE_UUID` in a raw `INSERT INTO item` (`plugins/ritrovo_importer/src/lib.rs:1228`,
`:1261`), and its own doc comment says "on the live stage" (`:1211`). In the demo
database every one of the 5,656 conferences has stage Live, status 1, and no
revision. The tutorial set's README and the brief both say imports land in Incoming.

**The bootstrap does nothing about it.** `scripts/demo-bootstrap.sh` imports the
stages and never mentions landing or promotion; nothing in the demo moves an item.

**Why it cannot simply be changed.** Binding the Incoming stage is a one-line change,
and the result would be a public site with no conferences and no way to publish
any: no route or host call moves an item between stages
(`G-NO-ITEM-STAGE-TRANSITION`), editors cannot be given the permissions
`ritrovo_access` checks (`G-PERM-TAP-NOT-DISPATCHED`), and the stage marked default
is not honoured on create (`G-DEFAULT-STAGE-IGNORED-ON-CREATE`). The row is P3.

## Findings from earlier runs, re-verified

| Finding | Now | Evidence |
| :- | :- | :- |
| Seeded "Call for Papers" link points at `/open-cfps`; canonical is `/cfps` | **Still stands.** | `menu_link` row path `/open-cfps` in the demo database, from the tutorial set's `menu_link.0193a5a0-0004-7000-8000-000000000003.yml:4`. In Chrome, `/open-cfps` is 404 and `/cfps` renders the open CFPs. `G-TUTORIAL-CONFIG-SET-DEFECTS`. |
| Italian aliases ship double-prefixed | **Still stands; the demo works around it.** | The tutorial set stores `/it/conferenze`, `/it/relatori`, `/it/argomenti` with `language: it`. `/it/it/conferenze` is 200 in the demo, which is the only request those rows can match. `/it/conferenze` works because `demo/config/url_alias.*.yml` adds prefixless Italian rows; `/conferenze` alone is 404. `/it/argomenti` renders with `lang="en"`. |
| Config import must precede plugin enable, or taxonomy resolves `0/23` | **Still stands, and the bootstrap is ordered for it.** | `discover_taxonomy_uuids` runs once, from `tap_install` only (`plugins/ritrovo_importer/src/lib.rs:240`, defined `:363`). The bootstrap imports at `scripts/demo-bootstrap.sh:131-132` before enabling at `:138`. The server log for this run: `discover_taxonomy_uuids: 23/23 terms found`. The importer screen's "23 of 25" counts the two confs.tech topics with no mapping (`sre`, `scala`, `lib.rs:100-102`). |
| A single cron call caps at 100 batches | **Holds as worded for this importer, with a correction.** | The cap is 100 queue jobs per plugin per drain cycle, `MAX_QUEUE_ITEMS_PER_CYCLE: i64 = 100` (`crates/kernel/src/cron/mod.rs:45`, applied at `:984-985`), claimed in chunks of at most the declared concurrency, capped at 4. The importer pushes one job per batch, so for it 100 jobs is 100 batches; another plugin with work gets its own 100. |

## Status by story

### Epic 1: Hello, Trovato (Part 1)

| # | The brief promises | Where | Status | Evidence | Closes in |
| :- | :- | :- | :- | :- | :- |
| 29.1 | `conference` type with every field in the content model | config | **blocked on the kernel** | **done in A4 except `topics`.** `demo/config/item_type.conference.yml` declares 16 fields: `speakers` and `schedule_pdf` added, `venue_photo` now multi-value `venue_photos`, `editor_notes` plain text. `description` stays `Blocks`, which is the kernel's rich text and the only kind its editor edits. `topics` is deliberately NOT declared: no field kind is taxonomic (`G-NO-CATEGORY-REFERENCE-FIELD-KIND`), and declaring it as `Text` would put a text widget over a uuid array. **[corrected]** the audit read the save that dropped `field_topics` as a consequence of the field being undeclared; it is not. Either form replaces the whole field set with what it rendered (`G-FORM-SAVE-DROPS-EVERY-FIELD-THE-FORM-DID-NOT-RENDER`), so declaring it changes nothing. `cardinality: -1` on the two multi-value fields is recorded and read by nothing (`G-CARDINALITY-IS-INERT`). | A4 |
| 29.2 | Admin form for manual conference creation, dates, checkbox, required fields | kernel | **done** | `/item/add/conference` and `/item/{id}/edit` render every declared field with date inputs, a checkbox and required markers; an edit saved. Data loss on save is 29.1's. | A4 |
| 29.3 | Upcoming Conferences gather, paged, at `/conferences` | config | **done** | `/conferences` renders 25 cards sorted by start date from today, with a pager. Page size is the brief's 25 now, here and on the topic and location gathers; `verify-demo.sh` counts the cards. | A4 |

### Epic 2: Search That Thinks

| # | The brief promises | Where | Status | Evidence | Closes in |
| :- | :- | :- | :- | :- | :- |
| 30.1 | Pagefind index rebuilt by cron on content change | config | **kernel has it, Ritrovo does not use it** | `crates/kernel/src/cron/pagefind.rs:1-8` with `plugins/trovato_search` (disabled by default; not enabled by the demo). The pagefind CLI is in the image (`Dockerfile:43-47`). No admin or CLI re-index trigger. | A9 |
| 30.2 | Instant client-side search UI with no-JS fallback | config | **kernel has it, Ritrovo does not use it** | `templates/search.html:100-121` loads `static/js/scolta.js`. In the demo it 404s on `pagefind.js` and blanks the page (`G-SEARCH-PAGE-BLANK-WITHOUT-INDEX`). | A9 |
| 30.3 | Configurable client-side ranking signals | kernel | **blocked on the kernel** | Scoring exists in `static/js/scolta.js`, with its values hard-coded in `templates/search.html:100-121`; there is no configuration for them (`G-SEARCH-NO-ADMIN-OR-ANALYTICS`). | A9 |
| 30.4 | AI query expansion | config | **kernel has it, Ritrovo does not use it** | `POST /api/v1/search/expand`, `crates/kernel/src/routes/api_search.rs:25-30`; needs a Chat provider. None configured. | A9 |
| 30.5 | AI summary streamed over SSE | config | **kernel has it, Ritrovo does not use it** | `/api/v1/search/summarize`, `routes/api_search.rs:25-30`. | A9 |
| 30.6 | Conversational follow-up and sentiment analysis | kernel | **blocked on the kernel** | Follow-up exists (`/api/v1/search/followup`); no sentiment classification exists in `crates/kernel/src` (`G-SEARCH-NO-ADMIN-OR-ANALYTICS`). | A9 |
| 30.7 | Search configuration admin and analytics dashboard | kernel | **blocked on the kernel** | No search settings screen and no query log (`G-SEARCH-NO-ADMIN-OR-ANALYTICS`). Per-type field weights exist at `/admin/structure/types/{type}/search`. | A9 |

### Epic 3: AI as a Building Block

| # | The brief promises | Where | Status | Evidence | Closes in |
| :- | :- | :- | :- | :- | :- |
| 31.1 | AI provider registry, keys never in the database | config | **kernel has it, Ritrovo does not use it** | `crates/kernel/src/routes/admin_ai_provider.rs`, `services/ai_provider.rs`; admin at `/admin/system/ai-providers` (linked from `/admin` in the demo). | A9 |
| 31.2 | `ai_request()` host function | kernel | **kernel has it, Ritrovo does not use it** | `ai-api` interface in `crates/wit/kernel.wit`, `crates/kernel/src/host/ai.rs`. | A9 |
| 31.3 | Token budgets | config | **kernel has it, Ritrovo does not use it** | `crates/kernel/src/routes/admin_ai_budget.rs`, `/admin/system/ai-budgets`. | A9 |
| 31.4 | AI permissions | config | **kernel has it, Ritrovo does not use it** | Kernel permissions, not `tap_perm` ones: `use ai`, `use ai chat` and others at `crates/kernel/src/models/role.rs:49-53`. | A9 |
| 31.5 | Content enrichment field rules on presave | config | **kernel has it, Ritrovo does not use it** | `tap_item_presave` dispatched and its `fields` merged (`crates/kernel/src/content/item_service.rs:365-390`); `trovato_ai` disabled. | A9 |
| 31.6 | AI Assist buttons in forms | kernel | **blocked on the kernel** | `POST /api/v1/ai/assist` exists (`routes/api_ai_assist.rs:54`); the buttons are injected by `tap_form_alter`, which never fires (`G-FORM-TAPS-UNREACHABLE`). | A9 |
| 31.7 | Chatbot tile with SSE and RAG | config | **kernel has it, Ritrovo does not use it** | `chat` tile type (`crates/kernel/src/services/tile.rs:120-122`), `POST /api/v1/chat` (`routes/api_chat.rs:433`). | A9 |
| 31.8 | Chatbot actions through tool calling | plugin | **kernel has it, Ritrovo does not use it** | `tap_chat_actions` is superseded; the dispatched surface is `tap_assistant_scopes`, `tap_assistant_context`, `tap_assistant_tool` (`crates/wit/kernel.wit:476-478`, `services/ai_assistant.rs:665`, `:696`). | A9 |
| 31.9 | AI admin UI and usage dashboard | config | **kernel has it, Ritrovo does not use it** | `routes/admin_ai_provider.rs`, `admin_ai_budget.rs`, `admin_ai_chat.rs`, `admin_ai_features.rs`. | A9 |
| 31.10 | MCP server | kernel | **kernel has it, Ritrovo does not use it** | `crates/mcp-server`. | A9 |
| 31.11 | VectorStore trait and pgvector | kernel | **kernel has it, Ritrovo does not use it** | `trait VectorStore`, `crates/kernel/src/services/vector_store.rs:56`. The demo's Postgres image is `postgres:16-alpine`, without pgvector. | A9 |

### Epic 4: From Demo to Data-Driven (Part 2)

| # | The brief promises | Where | Status | Evidence | Closes in |
| :- | :- | :- | :- | :- | :- |
| E4.1 | Plugin scaffold and SDK basics | plugin | **done** | All five plugins build against the public SDK and are enabled (`scripts/verify-demo.sh` plugin checks passed); `/admin/plugins` lists them. | A4 |
| E4.2 | Cron-driven conference import | plugin | **done** | 263 batches queued, drained by cron, 5,656 conferences; `/admin/config/importer` shows state and "Jobs waiting 0". Caveats are P3's. | A4 |
| E4.3 | Hierarchical topic taxonomy | config | **done** | The brief's tree exactly: 44 terms in `demo/config/tag.*.yml`, three levels, with Kotlin cross-listed under both JVM and Mobile. `verify-demo.sh` counts the terms, asserts a grandchild link exists and asserts a term with two parents exists. The confs.tech mapping is a data file, `plugins/ritrovo_importer/data/confs-tech-topics.json`; three feeds (general, opensource, testing) map to no term because the brief's tree has none, and the importer's admin screen names them rather than counting them as failures. | A4 |
| E4.4 | Advanced gathers with exposed and contextual filters | config, template | **done except the speaker relationship** | `/location/{country}` and `/location/{country}/{city}` have templates of their own and render conference cards; the raw column dump is gone, and `verify-demo.sh` asserts `search_vector` is absent and cards are present. The two tile gathers exist, with templates and aliases of their own: `/conferences/this-month` and `/cfps/closing-soon`. Both are bounded by count rather than by the brief's date window, because a gather can express "today" and nothing else (`G-NO-RELATIVE-DATE-FILTER-VALUES`). The brief's speaker relationship on the upcoming gather is not expressible either: `QueryRelationship` joins a table on two columns (`crates/kernel/src/gather/types.rs:432-450`) and the reference is a uuid array inside JSONB. | A4 |
| E4.5 | Full-text search | config | **kernel has it, Ritrovo does not use it** | Server search works (`/api/search?q=rust`, `/api/v1/search?q=rust`); the page visitors use is blank in a browser until `trovato_search` builds an index (`G-SEARCH-PAGE-BLANK-WITHOUT-INDEX`). | A9 |

### Epic 5: Look and Feel (Part 3)

| # | The brief promises | Where | Status | Evidence | Closes in |
| :- | :- | :- | :- | :- | :- |
| 34.1 | Render tree and conference templates | template | **Ritrovo must build it** | `docs/tutorial/templates/elements/item--conference.html` renders the header and links, then `{{ children }}` dumps every field again as `: value` lines (seen on "Gerrit User Summit"). Seeded Italian conferences render no description. `/cfps` cards render correctly. | A9 |
| 34.2 | File uploads with security validation | config | **blocked on the kernel** | **[corrected]** the audit recorded this as available and unused. A demo cannot use it: no config entity can create a file (`G-NO-FILE-CONFIG-ENTITY`), and a file uploaded any other way is never promoted out of temporary and is deleted six hours later, because `mark_permanent_batch` is called only from the admin content form (`G-CONFIG-IMPORT-NEVER-PROMOTES-A-FILE`). So an image reaches a Trovato site only by a human using the admin UI. `logo`, `venue_photos`, `schedule_pdf` and `headshot` are declared and their templates render them when present. | A4 |
| 34.3 | Speaker type with RecordReference | config | **done** | `demo/config/item_type.speaker.yml` is the brief's: bio, headshot, website, and no forward `field_conferences`, which duplicated a reverse reference. The conference holds `field_speakers`; a speaker page lists its conferences through `reverse_references`. Six speakers are seeded and `/speakers` renders them. `verify-demo.sh` follows a seeded conference to a speaker and back. | A4 |
| 34.4 | Slots, tiles, navigation and breadcrumbs | config, template | **blocked on the kernel** | Header, sidebar and footer tiles and breadcrumbs render; the two `gather_query` tiles render empty (`G-TILE-GATHER-QUERY-RENDERS-NOTHING`). Ritrovo's part: the 404 menu links (`G-TUTORIAL-CONFIG-SET-DEFECTS`). | A9 |
| 34.5 | Weighted full-text search page | config | **kernel has it, Ritrovo does not use it** | Six `search_field_config` rows imported; weights at `/admin/structure/types/conference/search`; `/search` blank with JavaScript (`G-SEARCH-PAGE-BLANK-WITHOUT-INDEX`). | A9 |
| 34.6 | Premium theme with design tokens | template | **Ritrovo must build it** | Kernel `theme.css` styles the site; `docs/tutorial/static/css/ritrovo.css` is served and linked by no page; no CSS for `.cfp-badge` or `.lang-badge` exists anywhere; `page--front.html` is never rendered because `/` redirects to `/conferences`. | A9 |

### Epic 6: The Editorial Engine (Part 4)

| # | The brief promises | Where | Status | Evidence | Closes in |
| :- | :- | :- | :- | :- | :- |
| 35.1 | Registration, login, sessions, and three test users | config, plugin | **done (A5)** | `editor_alice`, `publisher_bob` and `viewer_carol` exist, are active, carry the editor, publisher and viewer roles, and log in; `verify-demo.sh` asserts all three. Created by `ritrovo_access`'s `tap_install`, not by registration: the kernel counts one registration against the rate limit twice, so three per hour admits one (`G-RATE-LIMIT-COUNTED-TWICE`), and nothing in the kernel activates a user or gives it a role. | A5 |
| 35.2 | Role-based permissions with plugin Grant and Deny | config, plugin | **done (A5), with a documented substitution** | Five roles ship in `demo/config/role.*.yml` and import into any Trovato database. Grant and Deny work: an Incoming conference is 404 to a visitor and to `viewer_carol`, 200 to `editor_alice`, decided by `tap_item_access`. None of the brief's seven permission names is grantable (`G-PERM-TAP-NOT-DISPATCHED`), so the roles carry kernel permissions and the plugin reads those: `edit any content` for an editor, `delete any content` for a publisher. Coarser than the brief, named as such in the role files, the plugin and `docs/EDITORIAL.md`. | A5 |
| 35.3 | Incoming, Curated, Live workflow with enforced transitions | config, plugin | **done (A5), Ritrovo builds what the kernel lacks** | The importer lands on Incoming (5,561 on the verified run), a demo seed publishes a sample, and `/admin/content/editorial` moves a selection between stages with the workflow enforced by a transition table. Verified end to end: publisher promoted Curated to Live and an anonymous visitor then saw the conference. The kernel still moves no item between stages (`G-NO-ITEM-STAGE-TRANSITION`); this is a plugin writing `stage_id` through the raw-SQL host call, with the costs recorded there. | A5 |
| 35.4 | Revision history with revert, five scenarios | kernel | **blocked on the kernel** | Worse than first recorded: `/item/{id}/revisions` is 500 for **every** item created through the kernel, not only edited ones, because `Item::create` writes an initial revision and both queries omit two columns the struct requires. Reproduced on the demo: 200 with zero revisions, 500 with one. Revert 500s at the same line and its only button is on that page. No compare and no draft preview exist at all. Ritrovo deliberately did not build a replacement; see `docs/EDITORIAL.md` Walkthrough 4. (`G-REVISION-HISTORY-500`, `G-REVISION-NO-COMPARE`) | A5 |
| 35.5 | Admin content list with filters and bulk actions | kernel, plugin | **done, and it is not where the brief wants it** | `/admin/content` filters and offers bulk publish, unpublish and delete, administrator only. Its `publish` sets `status`, not stage, so it cannot publish anything off an internal stage, and a plugin cannot add a stage action to it: the allowlist is three literals with no seam (`G-BULK-ACTIONS-NOT-EXTENSIBLE`). Bulk Curated to Live therefore lives on Ritrovo's own screen. No progress display anywhere; nothing executes a batch (`G-BATCH-NO-EXECUTOR`). | A5 |

### Epic 7: Forms and User Input (Part 5)

| # | The brief promises | Where | Status | Evidence | Closes in |
| :- | :- | :- | :- | :- | :- |
| 36.1 | Form API pipeline and conference edit form, with `tap_form_alter` | kernel, plugin | **blocked on the kernel** | The edit form renders and saves with CSRF; `tap_form_alter` never fires (`G-FORM-TAPS-UNREACHABLE`). | A6 |
| 36.2 | WYSIWYG for rich text | config | **kernel has it, Ritrovo does not use it** | Editor.js for `Blocks` fields (`templates/admin/content-form.html:164-171`), upload gated on `trovato_block_editor`, disabled; the demo's edit form shows no editor. `TextLong` bios get no editor in the kernel. | A6 |
| 36.3 | Conditional CFP fields, add another, topic autocomplete | kernel | **blocked on the kernel** | Add-another and record autocomplete exist for administrators (`crates/kernel/src/routes/admin.rs:386-418`, `routes/api_v1.rs:73`); no conditional fields (`G-AJAX-ADMIN-ONLY-NO-CONDITIONAL-FIELDS`). | A6 |
| 36.4 | Three-step submission form landing in Incoming | plugin | **blocked on the kernel** | No multi-step flow (`G-FORM-TAPS-UNREACHABLE`); a plugin page cannot take the logo upload (`G-FILE-NO-HOST-API`); nothing lands content in Incoming (`G-NO-ITEM-STAGE-TRANSITION`, `G-DEFAULT-STAGE-IGNORED-ON-CREATE`). | A6 |
| 36.5 | `ritrovo_cfp`: badge, date validation, `cfp_closing_soon` events | plugin | **blocked on the kernel** | Badge done (P5); validation cannot refuse a save (`G-PRESAVE-CANNOT-REFUSE`); the event cannot reach another plugin's queue (`G-QUEUE-NO-CROSS-PLUGIN`). | A6 |
| 36.6 | `ritrovo_access`: stage gating and field-level access | plugin | **blocked on the kernel** | `tap_item_access` is dispatched, and grants on permissions no role can hold (`G-PERM-TAP-NOT-DISPATCHED`). The `editor_notes` half is Ritrovo work (P19). | A5 |
| 36.7 | Profile form: display name, bio, avatar, timezone, preferences | kernel | **blocked on the kernel** | `/user/profile` has username, email, timezone and password only (`G-USER-PROFILE-NOT-EXTENSIBLE`). | A6 |

### Epic 8: Community and Plugin Communication (Part 6)

| # | The brief promises | Where | Status | Evidence | Closes in |
| :- | :- | :- | :- | :- | :- |
| 37.1 | Threaded comments on conferences | config | **kernel has it, Ritrovo does not use it** | Kernel comments work (a comment and a threaded reply were posted as administrator). No role Ritrovo uses holds `post comments`: only `comment_moderator` does, and anonymous visitors see "Log in to post a comment." Granting it to `authenticated user` is config. | A7 |
| 37.2 | Comment moderation queue for editors | kernel | **blocked on the kernel** | `/admin/content/comments` exists, administrator only (`G-ADMIN-SCREENS-ARE-ADMIN-ONLY`). | A7 |
| 37.3 | Subscribe toggle and My Subscriptions page | plugin | **Ritrovo must build it** | The toggle on every conference page (shown even to anonymous visitors) does nothing; `/user/subscriptions` is 404 (`plugins/ritrovo_notify/src/lib.rs:28-57`). Buildable with `tap_api` routes over the kernel's `user_subscriptions` table; permission gating waits on `G-PERM-TAP-NOT-DISPATCHED`, the no-JS redirect on `G-PLUGIN-ROUTE-NO-HEADERS`. | A7 |
| 37.4 | `ritrovo_notify`: notifications and email digests | plugin | **blocked on the kernel** | `G-MAIL-CANNOT-REACH-A-USER`, `G-MAIL-UNAVAILABLE-IN-BACKGROUND`, `G-NO-USER-DIRECTORY`; imported changes fire no `tap_item_update` (`G-ITEM-API-BYPASSES-ITEM-SERVICE`). | A7 |
| 37.5 | Plugin-to-plugin through a shared queue | plugin | **blocked on the kernel** | `G-QUEUE-NO-CROSS-PLUGIN`. The kernel's alternative is `plugin-api` invocation. | A7 |
| 37.6 | Comment notifications to subscribers | plugin | **blocked on the kernel** | `tap_comment_insert` is dispatched (`crates/kernel/src/services/comment.rs:67`); delivery is `G-MAIL-CANNOT-REACH-A-USER`. The kernel already mails the item's author (`crates/kernel/src/routes/comment.rs:1092-1157`). | A7 |

### Epic 9: Going Global (Part 7)

| # | The brief promises | Where | Status | Evidence | Closes in |
| :- | :- | :- | :- | :- | :- |
| 38.1 | Multilingual content model | config | **blocked on the kernel** | The kernel's model is `item_translation`, and nothing can write it (`G-TRANSLATION-NO-WRITE-PATH`). The seed's parallel JSONB is not rendered: Codemotion Roma 2026 shows no description in either language. | A8 |
| 38.2 | Language routing, localized aliases, switcher, hreflang, UI strings | config, template | **blocked on the kernel** | Prefix routing works and `/it/conferenze` renders `lang="it"`; every UI string on it is English (`G-LOCALE-STRINGS-NEVER-LOADED`); no switcher in any template; pathauto writes no language (`crates/kernel/src/services/pathauto.rs:167`, `:204`). | A8 |
| 38.3 | `ritrovo_translate`: detection, queue, side-by-side translation | plugin | **blocked on the kernel** | `/admin/content/translations` is 404; the kernel's translate screens are GET only with missing templates (`G-TRANSLATION-NO-WRITE-PATH`); insert output is discarded (`G-ITEM-INSERT-OUTPUT-DISCARDED`). | A8 |
| 38.4 | About 20 Italian conferences with English translations | config | **blocked on the kernel** | 15 seeded (8 translated, 3 in progress, 4 needing translation), in a shape nothing renders; a correct seed needs a translation entity for config import (`G-TRANSLATION-NO-WRITE-PATH`). `/it/conferenze` lists the same English content as `/conferences`. | A8 |
| 38.5 | REST endpoints under `/api/v1/conferences`, topics, speakers, subscribe | plugin | **blocked on the kernel** | All 404. The read endpoints are buildable now as `tap_api` routes over the kernel's gather execution; the write endpoints would skip access checks and revisions (`G-ITEM-API-BYPASSES-ITEM-SERVICE`). | A8 |
| 38.6 | API keys and per-role rate limits | kernel, config | **kernel has it, Ritrovo does not use it** | Tokens and bearer auth exist (`crates/kernel/src/routes/api_token.rs:162-163`); limits are fixed at 100 a minute for everyone and there is no token page (`G-API-RATE-LIMITS-FIXED`). No demo key. | A8 |

### Epic 10: Production Ready (Part 8)

| # | The brief promises | Where | Status | Evidence | Closes in |
| :- | :- | :- | :- | :- | :- |
| 39.1 | Two-tier caching with tag invalidation | kernel | **kernel has it, Ritrovo does not use it** | `crates/kernel/src/cache/mod.rs:84-208`. Nothing in Ritrovo tags or invalidates, and the importer's raw SQL skips item cache invalidation. | A9 |
| 39.2 | Batch operations with progress | kernel, plugin | **partly done (A5); progress blocked** | Bulk stage moves work on Ritrovo's editorial screen, inline over the selection, the same way the kernel's own bulk publish works. There is no progress display and cannot be: nothing anywhere executes a batch, and `update_progress`, `complete` and `fail` have no callers outside their own module (`G-BATCH-NO-EXECUTOR`). A plugin also cannot add a stage action to `/admin/content` (`G-BULK-ACTIONS-NOT-EXTENSIBLE`). | A5 |
| 39.3 | S3-compatible storage | kernel | **blocked on the kernel** | `G-S3-STORAGE-REMOVED`. | A4 |
| 39.4 | Cron with distributed locking | kernel | **done** | `scripts/demo-cron.sh` posts to `/cron/{key}`; the kernel log shows "acquired cron lock, running tasks" each minute. | A4 |
| 39.5 | Prometheus metrics and health check | kernel | **done** | `/metrics` and `/health` answer 200 in the demo. | A4 |
| 39.6 | A full test suite running in CI | plugin | **done** | `.github/workflows/ci.yml` runs every plugin through the real kernel; `demo.yml` runs the one-command demo weekly; CI green on `9d5dd35`. | A4 |
| 39.7 | Configuration import and export | config | **done** | The demo is assembled by `trovato config import` (`scripts/demo-bootstrap.sh:131-153`). Ownership is the recommendation above. | A4 |

## Status by "What It Demonstrates" line

| # | The brief promises | Where | Status | Evidence | Closes in |
| :- | :- | :- | :- | :- | :- |
| D1 | Item Types and CCK: two types, JSONB fields, RecordReference, file fields | config | **blocked on the kernel** | Two types, JSONB fields and RecordReference are done (29.1, 34.3). File fields are declared and unusable (34.2). | A4 |
| D2 | Categories: three-level topics, nested queries, breadcrumbs, browsing | config, template | **blocked on the kernel** | The tree, the nested queries and the browsing are done (E4.3). The breadcrumb on a conference page is not: an item template can print a topic's uuid and nothing else, because the item route resolves record references but never a category tag (`G-ITEM-ROUTE-DOES-NOT-RESOLVE-CATEGORY-TAGS`). The gather side has breadcrumbs. | A4 |
| D3 | Stages and Revisions: workflow, draft, preview, revert, stage a new version | kernel, plugin | **half done (A5)** | The workflow half is done and demonstrable: 35.3. The revision half is not, and none of draft, preview, revert or compare works on this kernel: 35.4. | A5 |
| D4 | Gather: six definitions, exposed and contextual filters, relationships, paging | config | **done except relationships** | Eight definitions now, the brief's six plus its two tile gathers, all with templates and routes. Paging is the brief's 25. The speaker relationship is not expressible (E4.4). | A4 |
| D5 | Render Tree: conference card, detail page, speaker card, CFP badge, topic pills, profile | template | **Ritrovo must build it** | Conference cards render; the detail page dumps raw fields; no topic pills; no speaker cards to show; the badge is unstyled text. | A9 |
| D6 | Form API: edit form, multi-step submission, profile form, subscription form | kernel | **blocked on the kernel** | 36.1, 36.4, 36.7. | A6 |
| D7 | WYSIWYG for descriptions and bios, AJAX add another | config | **kernel has it, Ritrovo does not use it** | 36.2, 36.3. | A6 |
| D8 | Five plugins demonstrating the full tap lifecycle | plugin | **blocked on the kernel** | All five install and enable; of the brief's 19 intended taps, 4 work and 9 are blocked (P1 to P19). | A7 |
| D9 | Plugin-to-plugin through the notifications queue | plugin | **blocked on the kernel** | 37.5. | A7 |
| D10 | Cron: daily import, digest emails, temp file cleanup | plugin | **blocked on the kernel** | Import and cleanup run (P1; `crates/kernel/src/cron/tasks.rs:181-184`); digests are 37.4. | A7 |
| D11 | Queue: import validation queue with `tap_queue_info` and `tap_queue_worker` | plugin | **done** | 269 import jobs queued and drained on the current demo. The worker's error handling is fixed: see P3. | A4 |
| D12 | Tiles: CFPs closing soon, this month, topic cloud, recent comments, my subscriptions | config, plugin | **blocked on the kernel** | 34.4; the topic cloud is a sentence with no topics in it; recent comments and my subscriptions need tile types plugins cannot add. | A9 |
| D13 | Slots and the admin UI for placing tiles, with role and path visibility | config | **kernel has it, Ritrovo does not use it** | `/admin/structure/tiles` places tiles by region and weight; visibility exists in the model (`crates/kernel/src/models/tile.rs:211-243`) and is importable, and no demo tile uses it. | A9 |
| D14 | Search: weighted fields, stage-aware, across conferences and speakers | config | **kernel has it, Ritrovo does not use it** | E4.5, 34.5. | A9 |
| D15 | Users and Auth: registration, login, profiles with bio and avatar | kernel | **blocked on the kernel** | 35.1 for accounts; 36.7 for profiles. | A6 |
| D16 | Permissions: five roles, Grant and Deny, field visibility, stage scope | plugin, config | **blocked on the kernel** | 35.2, 36.6. | A5 |
| D17 | Files: logo, gallery, headshot, PDF, local and S3, private files | config | **kernel has it, Ritrovo does not use it** | 34.2; S3 is `G-S3-STORAGE-REMOVED`. | A4 |
| D18 | Menus: main, user, admin, footer, plugin entries, breadcrumbs | config | **Ritrovo must build it** | Menus render; "Call for Papers", "About" and "Contact" are 404; the brief's footer (API docs, data sources, privacy) does not exist; the importer's `tap_menu` entries appear in the admin menu listing. | A9 |
| D19 | REST API with keys and rate limits | plugin | **blocked on the kernel** | 38.5, 38.6. | A8 |
| D20 | Revisions: log, preview, revert to N, compare | kernel | **blocked on the kernel** | 35.4. | A5 |
| D21 | i18n: translated content, UI strings, aliases, switcher | config, plugin | **blocked on the kernel** | 38.1, 38.2. | A8 |
| D22 | Multi-step submission with state in PostgreSQL | plugin | **blocked on the kernel** | 36.4. | A6 |
| D23 | AJAX: conditional fields, add another, autocomplete, subscription toggle, filters | plugin, kernel | **blocked on the kernel** | 36.3; the toggle is 37.3; the filter form submits by full page load. | A6 |
| D24 | Caching demonstrated through Gander | kernel | **kernel has it, Ritrovo does not use it** | 39.1; Gander does not exist (`G-NO-REQUEST-PROFILER`). | A9 |
| D25 | Batch: bulk publish Curated to Live, bulk re-import, progress | plugin, kernel | **partly done (A5)** | Bulk publish Curated to Live is done, on `/admin/content/editorial`, and verified in the demo. Progress tracking is not, and nothing runs a batch: 39.2. | A5 |

## Status by intended plugin tap

| # | The brief promises | Where | Status | Evidence | Closes in |
| :- | :- | :- | :- | :- | :- |
| P1 | `ritrovo_importer` `tap_cron`: daily fetch, diff, queue | plugin | **done** | `plugins/ritrovo_importer/src/lib.rs:682`, gated to once a day by `should_import` (`:1094-1099`); the importer screen shows "Minimum interval between runs 86400 seconds". | A4 |
| P2 | `ritrovo_importer` `tap_queue_info`: declares `ritrovo_import` | plugin | **done** | `lib.rs:804`; the kernel reads its concurrency (`crates/kernel/src/cron/mod.rs:179-193`). | A4 |
| P3 | `ritrovo_importer` `tap_queue_worker`: validate, create in Incoming, log bad data | plugin | **done (A5)** | **"logged and skipped, not silently dropped" is done in A4.** The worker is a `#[plugin_tap_result]`, so a malformed batch returns `Err`, and the kernel's retry, backoff and dead-letter tier takes it: five attempts, then `status = 'dead'` with the row kept. A host-in-the-loop test drives a batch through all five attempts and asserts the row survives. The kernel's `last_error` is a constant, so the worker writes the reason into its own state and the admin screen reads it back (`G-QUEUE-DEAD-LETTER-DISCARDS-THE-PLUGINS-ERROR`). **Creates in Incoming as of A5.** It binds `stage_id` in its own INSERT, which is the only way an item reaches a non-Live stage on this kernel. The public site does not empty, because `ritrovo_access` promotes a sample and an editor promotes the rest (`G-NO-ITEM-STAGE-TRANSITION` stands: the kernel still moves nothing itself). | A5 |
| P4 | `ritrovo_importer` `tap_plugin_install`: full historical import and category seeding | plugin, config | **done** | `tap_install` (`lib.rs:232`) queued 263 batches across 12 years. The terms come from config import rather than the plugin, which has no category host call. | A4 |
| P5 | `ritrovo_cfp` `tap_item_view`: days-left badge, green, yellow, red | plugin | **done** | "CFP Urgent, 1 day left" renders on Gerrit User Summit, classed `cfp-badge--urgent` (`plugins/ritrovo_cfp/src/lib.rs:26`). The colours need CSS (34.6). | A6 |
| P6 | `ritrovo_cfp` `tap_item_insert` and `tap_item_update`: validate dates, emit `cfp_closing_soon` | plugin | **blocked on the kernel** | Not implemented. `G-PRESAVE-CANNOT-REFUSE`, `G-ITEM-INSERT-OUTPUT-DISCARDED`, `G-QUEUE-NO-CROSS-PLUGIN`; imports fire neither tap (`G-ITEM-API-BYPASSES-ITEM-SERVICE`). | A6 |
| P7 | `ritrovo_notify` `tap_menu`: `/user/{uid}/subscriptions` | plugin | **Ritrovo must build it** | Registered as a page entry with no `tap_api`, so it 404s (`plugins/ritrovo_notify/src/lib.rs:28-35`; `crates/kernel/src/routes/plugin_api.rs:111-131`). | A7 |
| P8 | `ritrovo_notify` `tap_item_view`: Subscribe toggle for signed-in users | plugin | **Ritrovo must build it** | A button with no action, shown to everyone (`lib.rs:42-57`). | A7 |
| P9 | `ritrovo_notify` `tap_item_update`: queue a notification when a subscribed conference changes | plugin | **blocked on the kernel** | Not implemented; the changes that matter come from the importer and fire no tap (`G-ITEM-API-BYPASSES-ITEM-SERVICE`). | A7 |
| P10 | `ritrovo_notify` `tap_queue_info`: declares `ritrovo_notifications` | plugin | **Ritrovo must build it** | Returns an object where the kernel reads an array, declares retry keys the kernel ignores, and has no worker or producer (`lib.rs:61`). | A7 |
| P11 | `ritrovo_notify` `tap_queue_worker`: send email or queue for digest | plugin | **blocked on the kernel** | Not implemented. `G-MAIL-CANNOT-REACH-A-USER`, `G-MAIL-UNAVAILABLE-IN-BACKGROUND`. | A7 |
| P12 | `ritrovo_notify` `tap_cron`: daily digest emails | plugin | **blocked on the kernel** | Not implemented. Same entries as P11. | A7 |
| P13 | `ritrovo_translate` `tap_item_insert`: detect language, flag for translation | plugin | **blocked on the kernel** | Returns a result the kernel discards (`plugins/ritrovo_translate/src/lib.rs:41`; `G-ITEM-INSERT-OUTPUT-DISCARDED`), and never sees imported items (`G-ITEM-API-BYPASSES-ITEM-SERVICE`). | A8 |
| P14 | `ritrovo_translate` `tap_item_view`: language badge and switcher | plugin | **Ritrovo must build it** | Guesses from the title, so Codemotion Roma 2026 is badged "English"; the Italian link `/it/conferences/{id}` is 404 where `/it/item/{id}` is 200 (`lib.rs:78-104`). Both fixable from `item.language`; knowing the page's language is `G-NO-REQUEST-LANGUAGE`. | A8 |
| P15 | `ritrovo_translate` `tap_cron`: process the translation queue | plugin | **blocked on the kernel** | Not implemented; there is nothing to write a translation into (`G-TRANSLATION-NO-WRITE-PATH`). | A8 |
| P16 | `ritrovo_translate` `tap_form_alter`: language selector on the edit form | plugin | **blocked on the kernel** | Not implemented; `G-FORM-TAPS-UNREACHABLE`. | A8 |
| P17 | `ritrovo_access` `tap_item_access`: Grant, Deny, Neutral by role and stage | plugin | **blocked on the kernel** | Dispatched and correct in its tests (`plugins/ritrovo_access/src/lib.rs:50`); grants on permissions no role can hold (`G-PERM-TAP-NOT-DISPATCHED`), over content that is all on Live. | A5 |
| P18 | `ritrovo_access` `tap_perm`: declare the editorial permissions | plugin | **blocked on the kernel** | Declared, `plugins/ritrovo_access/src/lib.rs`; still dispatched nowhere at the pinned release, confirmed by enumerating every dispatch site in the kernel (`G-PERM-TAP-NOT-DISPATCHED`). The plugin now reads its own permission first and a kernel permission second, so the day the kernel dispatches this tap the role files gain four lines and no code changes. | A5 |
| P19 | `ritrovo_access` `tap_item_view`: hide `editor_notes` from non-editors | plugin | **Ritrovo must build it** | An empty placeholder (`lib.rs:100-109`). The job is `tap_field_access`, which receives the viewer (`crates/kernel/src/content/item_service.rs:1222-1236`). Which role counts as an editor waits on `G-PERM-TAP-NOT-DISPATCHED`; `edit any content` is a kernel permission usable now. | A5 |

---

## Build order

Each prompt's rows, in the order to do them. **[kernel: `G-...`]** marks a row that
cannot close until that kernel change lands; a prompt does the Ritrovo half and
leaves the row open, saying so.

### Done, 2026-09-18

Pull request #10 fixed the three visitor-facing defects (the blank search page,
the raw field dump, the 404 menu link). The pull request after it moved the
configuration set into `demo/config` and built the brief's model on it: A4 items
1 to 8 below, except where a kernel gap stops them, and the first half of item 9.

### Before A4: re-land pull request #6

The code changes from #6 (strip `ritrovo_notify` to nothing until A7, delete the
importer's dead `conference_fields()`, correct `ritrovo_access`'s comment) onto
`main`. Its `FRICTION.md` entries are already in this pull request.

### A4: content model

1. Move the content model into `demo/config/` ("Where the content model lives"),
   import it before plugins are enabled, retire `tests/host/tutorial-config/`.
2. 29.1, D1: declare `field_topics`, `speakers`, `schedule_pdf`, multi-value venue
   photos; reconcile `description` and `editor_notes` with the brief.
3. E4.3, D2: the brief's three-level topic tree; update `SLUG_TO_TERM`.
4. 34.3: the speaker type as the brief has it, and seeded speakers.
5. E4.4, D4: the two tile gathers; templates for by-country and by-city; a
   speaker relationship; page size 25.
6. 29.3, 29.2: re-check both against the new type.
7. 34.2, D17: logo and gallery seeded for the marquee conferences.
8. D11, P1, P2, P4, E4.1, E4.2: re-verify after the model change.
9. P3, first half: make the worker a `#[plugin_tap_result]` so bad batches retry
   and dead letter, and keep landing on Live. The row closes in A5.
10. 39.4, 39.5, 39.6, 39.7: already done; keep green.
11. 39.3: nothing to do **[kernel: `G-S3-STORAGE-REMOVED`]**.

### A5: editorial

1. 35.1: the three editorial users, created by config or the bootstrap.
2. P19: `tap_field_access` in `ritrovo_access` hiding `field_editor_notes`, keyed on
   the kernel permission `edit any content`.
3. 35.5: re-verify with the new model.
4. P18, 35.2, 36.6, P17, D16 **[kernel: `G-PERM-TAP-NOT-DISPATCHED`,
   `G-PERM-GRID-SAVE-REVOKES-PLUGIN-GRANTS`]**.
5. 35.3, D3 **[kernel: `G-NO-ITEM-STAGE-TRANSITION`,
   `G-DEFAULT-STAGE-IGNORED-ON-CREATE`]**, then P3: the importer lands in Incoming.
6. 35.4, D20 **[kernel: `G-REVISION-HISTORY-500`, `G-REVISION-NO-COMPARE`]**.
7. 39.2, D25 **[kernel: `G-BATCH-NO-EXECUTOR`, `G-ADMIN-SCREENS-ARE-ADMIN-ONLY`]**.

### A6: forms

1. 36.2, D7: enable `trovato_block_editor`; confirm the description editor works.
2. P5: badge colours (with 34.6's CSS).
3. 36.1 **[kernel: `G-FORM-TAPS-UNREACHABLE`]**.
4. 36.3, D23 **[kernel: `G-AJAX-ADMIN-ONLY-NO-CONDITIONAL-FIELDS`]**.
5. 36.5, P6 **[kernel: `G-PRESAVE-CANNOT-REFUSE`, `G-QUEUE-NO-CROSS-PLUGIN`,
   `G-ITEM-API-BYPASSES-ITEM-SERVICE`]**.
6. 36.4, D22 **[kernel: `G-FORM-TAPS-UNREACHABLE`, `G-FILE-NO-HOST-API`,
   `G-NO-ITEM-STAGE-TRANSITION`]**.
7. 36.7, D15, D6 **[kernel: `G-USER-PROFILE-NOT-EXTENSIBLE`]**.

### A7: community

1. 37.1: grant `post comments` to `authenticated user` in `demo/config`.
2. P7, P8, 37.3: subscriptions as `tap_api` routes over `user_subscriptions`, with
   the toggle; permission gating **[kernel: `G-PERM-TAP-NOT-DISPATCHED`]**, no-JS
   redirect **[kernel: `G-PLUGIN-ROUTE-NO-HEADERS`]**.
3. P10: a real queue declaration and worker for on-site notifications.
4. 37.2 **[kernel: `G-ADMIN-SCREENS-ARE-ADMIN-ONLY`]**.
5. P9 **[kernel: `G-ITEM-API-BYPASSES-ITEM-SERVICE`]**.
6. 37.4, 37.6, P11, P12, D10 **[kernel: `G-MAIL-CANNOT-REACH-A-USER`,
   `G-MAIL-UNAVAILABLE-IN-BACKGROUND`, `G-NO-USER-DIRECTORY`]**; until then, the
   on-site list and a logged digest.
7. 37.5, D9 **[kernel: `G-QUEUE-NO-CROSS-PLUGIN`]**, or redesign on `plugin-api`.
8. D8: re-count the taps.

### A8: global and API

1. 38.5 read endpoints, D19: `tap_api` routes for conferences, topics, speakers and
   search over gather execution.
2. P14: badge from `item.language`; switcher links to `/it/item/{id}` and
   `/item/{id}` **[kernel: `G-NO-REQUEST-LANGUAGE`]** for the current language.
3. 38.6: a demo API key **[kernel: `G-API-RATE-LIMITS-FIXED`]** for per-role limits.
4. 38.2, D21 **[kernel: `G-LOCALE-STRINGS-NEVER-LOADED`]**; own the prefixless
   aliases in the moved config.
5. 38.1, 38.4, 38.3, P13, P15 **[kernel: `G-TRANSLATION-NO-WRITE-PATH`,
   `G-ITEM-INSERT-OUTPUT-DISCARDED`]**.
6. P16 **[kernel: `G-FORM-TAPS-UNREACHABLE`]**.
7. 38.5 write endpoints **[kernel: `G-ITEM-API-BYPASSES-ITEM-SERVICE`]**.

### A9: layout and search

1. 30.1, 30.2, E4.5, 34.5, D14: enable `trovato_search`, make the bootstrap build the
   index before it reports, and make `verify-demo.sh` check search in a browser or
   check the index exists **[kernel: `G-SEARCH-PAGE-BLANK-WITHOUT-INDEX`]** for a
   page that survives a missing index.
2. 34.1, D5: the conference template without the raw field dump; description for
   the seed.
3. D18: fix "Call for Papers"; create About; the brief's footer.
4. 34.6: link or fold `ritrovo.css`; badge styles; decide what `/` renders.
5. D13: tile visibility rules in config.
6. 39.1, D24: cache tags where Ritrovo renders **[kernel: `G-NO-REQUEST-PROFILER`]**.
7. 34.4, D12 **[kernel: `G-TILE-GATHER-QUERY-RENDERS-NOTHING`]**.
8. 30.3, 30.6, 30.7 **[kernel: `G-SEARCH-NO-ADMIN-OR-ANALYTICS`]**.
9. 30.4, 30.5, 31.1 to 31.11 (31.6 **[kernel: `G-FORM-TAPS-UNREACHABLE`]**).

### Proposed changes to the series

1. **Repair what the demo shows before building on it.** Three A9 rows are defects
   a visitor meets today, not features: the blank search page, the raw field dump
   on every conference page, and the 404 "Call for Papers" link. A4 through A8 each
   check their work in a browser against those pages. Move A9 items 1 to 3 to the
   start of A4.
2. **Put the AI rows in their own optional prompt.** Epic 3 and stories 30.4 to 30.6
   need a provider key and are kernel features to switch on, not Ritrovo features to
   build. The series has no prompt for them and A9 is already the largest; an A10
   that runs only with a key keeps A9 about layout and search.
3. **Expect A5 and A8 to close little until the kernel moves.** Of A5's rows only
   35.1, 35.5 and P19 can close; of A8's, the read half of 38.5, P14 and 38.6. Their
   blockers are the top of the kernel backlog. Counted by the rows that name them in
   "What the kernel cannot do yet": `G-PERM-TAP-NOT-DISPATCHED` with
   `G-PERM-GRID-SAVE-REVOKES-PLUGIN-GRANTS` (8), `G-NO-ITEM-STAGE-TRANSITION` (6),
   `G-FORM-TAPS-UNREACHABLE` (6), `G-TRANSLATION-NO-WRITE-PATH` (5), the two mail
   entries together (5), `G-ITEM-API-BYPASSES-ITEM-SERVICE` (5), and
   `G-REVISION-HISTORY-500` (3). Scheduling A5 and A8 after those land, and running
   the buildable halves of A6 and A7 first, wastes less.
4. **Make `ritrovo_access` part of A5 only.** It is listed under Epic 7 (forms), but
   every row it touches is editorial.
