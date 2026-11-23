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

**Superseded again, 2026-09-22 (A7).** The kernel pin moved to Trovato
`v0.103.0`, `rev 496b8f113be66102830723c1e2a968cad2ec1e0d`, image
`ghcr.io/jeremyandrews/trovato:0.103.0`, and the whole demo verification passed
on it. That release closed four of this audit's kernel blockers
(`G-PERM-TAP-NOT-DISPATCHED`, `G-PERM-GRID-SAVE-REVOKES-PLUGIN-GRANTS`,
`G-USER-API-NO-ADMIN-BYPASS`, `G-TRANSLATION-NO-WRITE-PATH`) and narrowed a
fifth (`G-ADMIN-SCREENS-ARE-ADMIN-ONLY`). Rows that moved say **done (A7)** and
name the kernel change that unblocked them. Two A7 rows were expected to stay
blocked and did not: 37.2, because comment moderation became delegable, and
37.1, because a plugin's permission became grantable. One moved the other way:
P8, the Subscribe toggle on the conference page, turned out to be closed by two
gaps acting together rather than buildable.

**Superseded again, 2026-11-23 (A11).** The kernel pin moved to Trovato `main`
at `81290cc862b2c8302bc0a99d270f2571405c51ef` (workspace 0.107.0), image
`ghcr.io/jeremyandrews/trovato:nightly-81290cc`, there being no release newer
than 0.104.0. The demo came up from empty volumes on it and `verify-demo.sh`
passed in full. Every row that was blocked on the kernel was rechecked. A row
moved to done or part done only where a test or a `verify-demo.sh` check shows
it. A row whose blocker the kernel removed, and which Ritrovo has not built on,
says **buildable, not built (A11)**. Each such cell names the kernel commit and
starts its note with **A11:**.

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
| **buildable, not built (A11)** | Was blocked on the kernel. The kernel commit named in the evidence removes the blocker, by its changelog and source. Ritrovo has not built on it, so no test or demo check here has shown it working. Counted apart from both blocked and done. |

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

Recounted for A11 (2026-11-23) from the status cells below, by the first words
of each: **part done** gathers every row that says part, half or partly done (and
38.5's "read half done"). At the close of A9 the counts were 48 done, 12 part
done, 14 kernel has it, 0 must build and 33 blocked on the kernel. A11 rechecked
all 33 against the new pin; "A11: the kernel-blocked rows rechecked" below has
the row-by-row result.

A9b (2026-11-23), first part: 31.1, 31.2 and 31.3 move to done and 31.4 and
31.9 to part done, all five from "kernel has it". The other A9b rows are
unchanged until the parts that build on the first are merged.

| Status | Stories (63) | What It Demonstrates (25) | Plugin taps (19) | All (107) |
| :- | -: | -: | -: | -: |
| done | 34 | 8 | 13 | 55 |
| part done | 10 | 6 | 1 | 17 |
| kernel has it, Ritrovo does not use it | 9 | 2 | 0 | 11 |
| buildable, not built | 6 | 5 | 4 | 15 |
| Ritrovo must build it | 0 | 0 | 0 | 0 |
| blocked on the kernel | 4 | 4 | 1 | 9 |

---

## A11: the kernel-blocked rows rechecked

The 33 rows blocked on the kernel at the close of A9, set against Trovato `main`
`81290cc`. "Evidence" is a test in this repository or a `verify-demo.sh` check;
a kernel commit alone moves a row only as far as buildable, not built.

| Row | Now | Evidence or what still blocks it |
| :- | :- | :- |
| 35.4, D20 | part done | `an_editor_lists_previews_compares_and_reverts_revisions`: history with log, preview, compare, revert, as an editor. Three of the brief's five scenarios unchecked. |
| 36.2 | done | `the_conference_edit_form_is_generated_from_the_type_and_loads_the_block_editor`: the editor's form loads the block editor. |
| 38.6 | part done | `verify-demo.sh`: a token authenticates alone, and is revoked. Per-role limits not configured. |
| D16, P18 | done | Stale rows. `the_real_roles_grant_what_the_editorial_workflow_needs`, `tap_perm_declares_the_editorial_permissions`. |
| D2 | done | Stale row; A9's topic breadcrumb, asserted by `verify-demo.sh`. |
| 30.3, 30.7 | kernel has it | `verify-demo.sh`: `/admin/config/search` and `/admin/reports/search` are 200 to the administrator. |
| 29.1 | buildable, not built | `RecordReference("category_term")` with term autocomplete, `1c62b0a`. |
| 30.6 | buildable, not built | Sentiment, `bae2013`. |
| 34.2, D1 | buildable, not built | File config entity, `ecb1eca`; files promoted, `c578da4`. |
| 36.3, D23 | buildable, not built | AJAX for editors and conditional fields, `65cf830`. |
| 37.6, P11, P12 | buildable, not built | Mail to a member, `d02c721`; mail in the background, `6ce4bf2`. |
| 39.1 | buildable, not built | A plugin's raw write bumps the item cache generation, `1dc0467`; not yet measured through a plugin. |
| D12, P8 | buildable, not built | Templates name their viewer and get plugin output apart, `c8d1883`. |
| D14 | buildable, not built | `use stages`, `20bd30a`; no role has it. |
| D18 | buildable, not built | Menus as trees, `0c0f5a2`; plugin entries with a parent not rechecked. |
| P16 | buildable, not built | Every content form through `FormService`, `044c5dc`. |
| 37.4, P9 | blocked | `G-BACKGROUND-ITEM-API-HAS-NO-ITEM-SERVICE` (new): an item write from `tap_queue_worker` fails, so imports fire no tap. |
| 37.5, D9 | blocked | Shared queues designed (`29c5575`), not built. |
| 39.3 | blocked | S3 dropped for 1.0. |
| D8 | blocked | P9. |
| D10 | blocked | 37.4. |
| D24 | blocked | Gander does not exist; the cache counters count now (`verify-demo.sh`). |
| 31.6 | blocked, not rechecked | A9b, out of scope. |

The four rows that were part done with a blocked half: 36.1's save no longer
loses fields (tested) and its form taps are buildable; 38.5's writes are
buildable; 39.2's progress is still blocked (batch executor designed, not built);
D19's keys are done (38.6).

Fixed in `FRICTION.md` with evidence: `G-REVISION-HISTORY-500`,
`G-REVISION-NO-COMPARE`, `G-FORM-SAVE-DROPS-EVERY-FIELD-THE-FORM-DID-NOT-RENDER`,
`G-CARDINALITY-IS-INERT`, `G-ITEM-EDIT-FORM-LOADS-NO-ASSETS`,
`G-API-TOKENS-NEVER-AUTHENTICATE`, `G-ADHOC-GATHER-BYPASSES-FIELD-ACCESS`,
`G-OPENAPI-IS-A-STATIC-LIST`, `G-CACHE-METRICS-NEVER-COUNT`.

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
| Menus and breadcrumbs | `menu_link` admin `crates/kernel/src/routes/admin_menu.rs:892-910`; main and footer injected, `routes/helpers.rs:249-261`; breadcrumbs `routes/item.rs:714-725`, `routes/gather.rs:427`; tag breadcrumb API `routes/category.rs:76` | Main and footer menus and breadcrumbs render; every link is 200 since A9. Plugin root entries are rendered by Ritrovo's `page.html`; entries with a parent render nowhere (`G-PLUGIN-MENU-ENTRIES-WITH-A-PARENT-RENDER-NOWHERE`). |
| Tiles and slots | `crates/kernel/src/services/tile.rs:76-126`, admin `routes/tile_admin.rs:274-286`, role and path visibility `models/tile.rs:211-243` (config import only) | Eight tiles placed, with path and role visibility; the `gather_query` type still renders empty (`G-TILE-GATHER-QUERY-RENDERS-NOTHING`), so the data tiles are `custom_html` bodies rendered by `ritrovo_importer`'s cron. |
| `mail` host interface | `crates/wit/kernel.wit:212-225`, `crates/kernel/src/host/mail.rs:10-14`: site contact only | Unused. Cannot address a user (`G-MAIL-CANNOT-REACH-A-USER`). |
| Config import and export | CLI `crates/kernel/src/main.rs:94-110`; 13 entity types in dependency order, `crates/kernel/src/config_storage/yaml.rs:51-65` | Used by the demo for import. Export unused. |
| Pagefind search | `crates/kernel/src/cron/pagefind.rs:1-8`, `plugins/trovato_search` (disabled by default), `templates/search.html:100-121`, `static/js/scolta.js` | Enabled since A4; built before the demo reports ready since A9, and searchable by title only through `ritrovo_importer`'s mirror (`G-PAGEFIND-INDEX-OMITS-TITLES`). |
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
| `G-TRANSLATION-NO-WRITE-PATH` | Medium (narrowed at 0.103.0) | P15's last step |
| `G-ITEM-API-BYPASSES-ITEM-SERVICE` | High | 38.5, 37.4, P6, P9, P13 |
| `G-ITEM-INSERT-OUTPUT-DISCARDED` | Medium | 36.5 |
| `G-FORM-TAPS-UNREACHABLE` | Medium | 36.1, 36.4, 36.7, D6, D22, P16 |
| `G-MAIL-CANNOT-REACH-A-USER` | Medium | 37.4, 37.6, D10, P11, P12 |
| `G-MAIL-UNAVAILABLE-IN-BACKGROUND` | Medium | 37.4, D10, P11, P12 |
| `G-QUEUE-NO-CROSS-PLUGIN` | Medium | 37.5, 36.5, D9, P6 |
| `G-TILE-GATHER-QUERY-RENDERS-NOTHING` | Medium | nothing since A9 (the data tiles are plugin-rendered `custom_html`); it still costs staleness, language and per-viewer content |
| `G-SEARCH-PAGE-BLANK-WITHOUT-INDEX` | Medium | nothing in the demo since A9 (re-verified open at 0.103.0); every site without an index |
| `G-ADMIN-SCREENS-ARE-ADMIN-ONLY` | Medium | 37.2, 39.2, D25 |
| `G-USER-PROFILE-NOT-EXTENSIBLE` | Medium | 36.7, D15 |
| `G-BATCH-NO-EXECUTOR` | Medium | 39.2, D25 |
| `G-PRESAVE-CANNOT-REFUSE` | Medium | 36.5, P6 |
| `G-NO-REQUEST-LANGUAGE` | Medium | nothing since A8 (worked around in the template) |
| `G-LOCALE-STRINGS-NEVER-LOADED` | Medium | 38.2 (UI strings), D21 |
| `G-TRANSLATION-OVERLAY-SKIPS-THE-DEFAULT-LANGUAGE` | Medium | 38.4 as the brief words it, 38.3's English half |
| `G-RENDER-BLOCKS-PASSES-STRINGS-UNSANITIZED` | High | a stored XSS through the submission form; closed in the conference and speaker templates, open in the kernel |
| `G-TOPICS-PAGE-IGNORES-LANGUAGE` | Low | 38.2 (topics page) |
| `G-TRANSLATE-SCREEN-TEXT-FIELDS-ONLY` | Low | 38.3 (descriptions) |
| `G-ALIAS-FALLBACK-HARDCODES-EN` | Low | nothing |
| `G-GATHER-TRANSLATION-MERGE-IS-BACKWARDS` | Medium | 38.2 (translated descriptions on `/it/` listings) |
| `G-THEMED-PLUGIN-PAGE-UNDER-ADMIN-RENDERS-UNWRAPPED` | Medium | nothing (a path template supplies the variable for the translation queue) |
| `G-ITEM-TEMPLATE-RENDERED-BEFORE-ITS-TRANSLATIONS` | Low | a switcher that hides itself when there is no translation |
| `G-DEFAULT-STAGE-IGNORED-ON-CREATE` | Medium | 36.4, 35.3 |
| `G-NO-USER-DIRECTORY` | Medium | 37.4, 37.6 |
| `G-USER-API-NO-ADMIN-BYPASS` | Medium | P8, P19 once they check the viewer |
| `G-AJAX-ADMIN-ONLY-NO-CONDITIONAL-FIELDS` | Low | 36.3, D23 |
| `G-TUTORIAL-CONFIG-SET-DEFECTS` | Low | 29.1, E4.3, D18, 38.2 |
| `G-QUEUE-WORKER-ERROR-IS-SUCCESS` | Low | P3 |
| `G-FILE-NO-HOST-API` | Low | 36.4, 36.7 |
| `G-PLUGIN-ROUTE-NO-HEADERS` | Low | 37.3 |
| `G-API-RATE-LIMITS-FIXED` | Low | 38.6 (per-role limits; site-wide override since 0.103.0) |
| `G-ADHOC-GATHER-BYPASSES-FIELD-ACCESS` | High | nothing Ritrovo can build; a confidentiality hole in every field `tap_field_access` hides |
| `G-API-TOKENS-NEVER-AUTHENTICATE` | High | 38.6, D19 (keys) |
| `G-OPENAPI-IS-A-STATIC-LIST` | Low | D19 (a generated spec) |
| `G-GATHER-EXPLICIT-COLUMNS-ARE-TEXT` | Low | nothing (the API uses `item.*`) |
| `G-UNKNOWN-GATHER-IS-A-500` | Low | nothing |
| `G-S3-STORAGE-REMOVED` | Low | 39.3, D17 |
| `G-NO-REQUEST-PROFILER` | Low | D24 |
| `G-REVISION-NO-COMPARE` | Low | 35.4, D20 |
| `G-SEARCH-NO-ADMIN-OR-ANALYTICS` | Low | 30.3, 30.6, 30.7 |
| `G-VIEW-TAP-INPUT-CARRIES-NO-VIEWER` | Low | nothing (documentation) |
| `G-PAGEFIND-INDEX-OMITS-TITLES` | Medium | nothing since A9 (`ritrovo_importer` mirrors the title) |
| `G-PAGEFIND-RESULT-URL-IGNORES-ALIAS-LANGUAGE` | Low | 30.2 (a result linking to a prefixless Italian alias) |
| `G-PAGEFIND-IGNORES-FIELD-WEIGHTS` | Low | 34.5 (weights in the browser's results) |
| `G-SEARCH-CANNOT-INDEX-CATEGORY-LABELS` | Low | nothing since A9 (`field_topic_names`) |
| `G-SEARCH-TRIGGER-INDEXES-BLOCKS-JSON` | Low | nothing (ranking noise) |
| `G-MARKDOWN-FILTER-HAS-NO-TABLES` | Low | nothing (the API page's tables) |
| `G-PLUGIN-MENU-ENTRIES-WITH-A-PARENT-RENDER-NOWHERE` | Medium | D18 (`ritrovo_notify`'s user entry) |
| `G-NO-VIEWER-ID-IN-TEMPLATES` | Medium | D12 (My Subscriptions as a list), D18 |
| `G-ONLY-MAIN-AND-FOOTER-MENUS-RENDER` | Low | nothing (the user menu is in `page.html`) |
| `G-TILE-ROLES-ARE-SYNTHETIC` | Low | nothing (an editor-only tile) |
| `G-TILES-HAVE-NO-LANGUAGE` | Low | nothing (tile text is English under `/it/`) |
| `G-BREADCRUMBS-IGNORE-THE-TOPIC-TREE` | Low | nothing (drawn from `includes/topics.html`) |
| `G-PLUGIN-RESPONSE-CANNOT-REDIRECT` | Low | nothing (a one-click My Subscriptions link) |
| `G-NO-ACCESSIBILITY-CHECK` | Low | nothing (structure asserted by `verify-demo.sh`) |

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
| 29.1 | `conference` type with every field in the content model | config | **buildable, not built (A11)** | **done in A4 except `topics`.** `demo/config/item_type.conference.yml` declares 16 fields: `speakers` and `schedule_pdf` added, `venue_photo` now multi-value `venue_photos`, `editor_notes` plain text. `description` stays `Blocks`, which is the kernel's rich text and the only kind its editor edits. `topics` is deliberately NOT declared: no field kind is taxonomic (`G-NO-CATEGORY-REFERENCE-FIELD-KIND`), and declaring it as `Text` would put a text widget over a uuid array. **[corrected]** the audit read the save that dropped `field_topics` as a consequence of the field being undeclared; it is not. Either form replaces the whole field set with what it rendered (`G-FORM-SAVE-DROPS-EVERY-FIELD-THE-FORM-DID-NOT-RENDER`), so declaring it changes nothing. `cardinality: -1` on the two multi-value fields is recorded and read by nothing (`G-CARDINALITY-IS-INERT`). **A11:** The kernel now has a category term reference widget, `RecordReference("category_term")`, with term autocomplete (Trovato `1c62b0a`, B137), and a save keeps fields the form did not render (`saving_the_edit_form_keeps_every_field_including_the_ones_it_did_not_render`). Declaring `field_topics` with it is the remaining work. Not built: `the_conference_edit_form_is_generated_from_the_type_and_loads_the_block_editor` still asserts `field_topics` has no widget. | A4 |
| 29.2 | Admin form for manual conference creation, dates, checkbox, required fields | kernel | **done** | `/item/add/conference` and `/item/{id}/edit` render every declared field with date inputs, a checkbox and required markers; an edit saved. Data loss on save is 29.1's. | A4 |
| 29.3 | Upcoming Conferences gather, paged, at `/conferences` | config | **done** | `/conferences` renders 25 cards sorted by start date from today, with a pager. Page size is the brief's 25 now, here and on the topic and location gathers; `verify-demo.sh` counts the cards. | A4 |

### Epic 2: Search That Thinks

| # | The brief promises | Where | Status | Evidence | Closes in |
| :- | :- | :- | :- | :- | :- |
| 30.1 | Pagefind index rebuilt by cron on content change | config | **done (A9)** | `trovato_search` is enabled by the bootstrap; its `tap_cron` asks for a rebuild whenever Live content has changed since the last build, and the kernel rebuilds in the same cron cycle (`crates/kernel/src/cron/pagefind.rs:1-8`). `scripts/serve-demo.sh` builds the index before it reports ready and waits until the index's page count equals the Live item count; `verify-demo.sh` asserts the same count on every run. Two kernel faults made the index useless until A9 and are covered by plugin code: no title was ever in the indexed body, and items with no configured text were left out (`G-PAGEFIND-INDEX-OMITS-TITLES`). No admin or CLI re-index trigger. | A9 |
| 30.2 | Instant client-side search UI with no-JS fallback | config | **done (A9)** | In Chrome, `/search?q=Codemotion` and `/search?q=Bruno Sala` list the conference and the speaker from the Pagefind index (`scolta.js`); with JavaScript off the server's tsvector results render in the `<noscript>` block. `verify-demo.sh` runs the same query the page runs, through `pagefind.js` itself (`scripts/pagefind-query.mjs`). Kernel faults left open: the page still blanks if the index is missing, re-verified at 0.103.0 with the index moved aside (`G-SEARCH-PAGE-BLANK-WITHOUT-INDEX`; the demo always has one), and a result can link to an Italian alias without its prefix (`G-PAGEFIND-RESULT-URL-IGNORES-ALIAS-LANGUAGE`). | A9 |
| 30.3 | Configurable client-side ranking signals | kernel | **kernel has it, Ritrovo does not use it** | Scoring exists in `static/js/scolta.js`, with its values hard-coded in `templates/search.html:100-121`; there is no configuration for them (`G-SEARCH-NO-ADMIN-OR-ANALYTICS`). Re-checked at 0.103.0 (A9 part 3): unchanged; out of A9's scope by the brief, recorded only. **A11:** Search settings with ranking read from them (Trovato `0d81def`, B75.1). `verify-demo.sh` asserts the administrator gets 200 from `/admin/config/search`. Ritrovo has not configured them. | A9 |
| 30.4 | AI query expansion | config | **kernel has it, Ritrovo does not use it** | `POST /api/v1/search/expand`, `crates/kernel/src/routes/api_search.rs:25-30`; needs a Chat provider. None configured. | A9b |
| 30.5 | AI summary streamed over SSE | config | **kernel has it, Ritrovo does not use it** | `/api/v1/search/summarize`, `routes/api_search.rs:25-30`. | A9b |
| 30.6 | Conversational follow-up and sentiment analysis | kernel | **buildable, not built (A11)** | Follow-up exists (`/api/v1/search/followup`); no sentiment classification exists in `crates/kernel/src` (`G-SEARCH-NO-ADMIN-OR-ANALYTICS`). Re-checked at 0.103.0 (A9 part 3): unchanged; out of A9's scope by the brief, recorded only. **A11:** Follow-up sentiment is classified off the request path (Trovato `bae2013`, B75.3, which names this story). Not exercised from here; the demo has no AI provider. | A9 |
| 30.7 | Search configuration admin and analytics dashboard | kernel | **kernel has it, Ritrovo does not use it** | No search settings screen and no query log (`G-SEARCH-NO-ADMIN-OR-ANALYTICS`). Per-type field weights exist at `/admin/structure/types/{type}/search`. Re-checked at 0.103.0 (A9 part 3): unchanged; out of A9's scope by the brief, recorded only. **A11:** The settings screen (Trovato `0d81def`, B75.1) and search analytics with a report (`6813241`, B75.2). `verify-demo.sh` asserts the administrator gets 200 from `/admin/config/search` and `/admin/reports/search`. | A9 |

### Epic 3: AI as a Building Block

| # | The brief promises | Where | Status | Evidence | Closes in |
| :- | :- | :- | :- | :- | :- |
| 31.1 | AI provider registry, keys never in the database | config | **done (A9b)** | `demo/config/variable.ai_providers.yml`: Anthropic (`https://api.anthropic.com/v1`, chat `claude-haiku-5-5`, key from `RITROVO_ANTHROPIC_API_KEY`) and OpenAI (embeddings `text-embedding-3-small`, `RITROVO_OPENAI_API_KEY`); `variable.ai_defaults.yml` routes chat and embeddings. The registry stores the variable names only. Host tests `the_key_value_appears_in_no_page_answer_export_stored_row_or_log_line` (screen, connection test, `config export`, `site_config`, `ai_usage_log`, every log line) and `with_no_key_every_ai_surface_answers_without_a_500_or_provider_text`; `verify-demo.sh` "AI" checks `(set)`/`(not set)` and nothing key-shaped. Live (A9b, 2026-11-23): both connection tests succeeded, Anthropic 557 ms, OpenAI 1,059 ms. An empty variable counts as a key (`G-AI-REQUESTS-SENT-WITHOUT-A-KEY`), so the compose file passes the names without a default. | A9b |
| 31.2 | `ai_request()` host function | kernel | **done (A9b)** | `ritrovo_translate`'s drafting job calls it with Ritrovo's chat model and no temperature (the `0.2` it set is gone). Host tests `a_draft_goes_through_ai_request_with_ritrovo_s_model_no_temperature_and_a_priced_usage_row` and `ai_request_refuses_a_caller_without_use_ai_before_asking_any_provider`, against the kernel's scripted provider. Live (2026-11-23): one cron cycle, three `claude-haiku-5-5` calls, 429 tokens, three Italian drafts stored; the job is off again (`assist_enabled` 0). | A9b |
| 31.3 | Token budgets | config | **done (A9b)** | `variable.ai_token_budgets.yml`: daily, deny; 20,000 tokens a member and 100,000 an editor or publisher per provider, 20,000 a day for `ritrovo_translate`. `variable.ai_pricing.yml` prices both models, so every `ai_usage_log` row carries a cost. Host test `the_daily_budget_stops_a_member_at_20000_tokens_and_lets_an_editor_past_it`. Live: the dashboard shows Anthropic 429 tokens, 3 requests. Search AI is not metered (`G-SEARCH-AI-CHECKS-NO-PERMISSION-OR-BUDGET`), so it is off. | A9b |
| 31.4 | AI permissions | config | **part done (A9b)** | `use ai` and `use ai chat` for every signed-in member, `view ai usage` for editors and publishers, nothing for anonymous visitors (`role.*.yml`). Host tests `the_roles_grant_ai_to_members_usage_to_editors_and_nothing_to_visitors`, `chat_and_search_ai_refuse_a_visitor_without_use_ai_and_ask_no_provider`; `verify-demo.sh` checks the dashboard is 200 to an editor and 403 to a viewer, and anonymous chat 401. Not done: per-role rate limits (`G-AI-RATE-LIMITS-ARE-NOT-PER-ROLE`) and a permission on search AI (`G-SEARCH-AI-CHECKS-NO-PERMISSION-OR-BUDGET`; the stages are switched off instead). | A9b |
| 31.5 | Content enrichment field rules on presave | config | **kernel has it, Ritrovo does not use it** | `tap_item_presave` dispatched and its `fields` merged (`crates/kernel/src/content/item_service.rs:365-390`); `trovato_ai` disabled. | A9b |
| 31.6 | AI Assist buttons in forms | kernel | **blocked on the kernel** | `POST /api/v1/ai/assist` exists (`routes/api_ai_assist.rs:54`); the buttons are injected by `tap_form_alter`, which never fires (`G-FORM-TAPS-UNREACHABLE`). **A11:** Not rechecked: the AI rows are A9b and out of A11's scope. The kernel's changelog says the form taps now fire (`044c5dc`). | A9b |
| 31.7 | Chatbot tile with SSE and RAG | config | **kernel has it, Ritrovo does not use it** | `chat` tile type (`crates/kernel/src/services/tile.rs:120-122`), `POST /api/v1/chat` (`routes/api_chat.rs:433`). | A9b |
| 31.8 | Chatbot actions through tool calling | plugin | **kernel has it, Ritrovo does not use it** | `tap_chat_actions` is superseded; the dispatched surface is `tap_assistant_scopes`, `tap_assistant_context`, `tap_assistant_tool` (`crates/wit/kernel.wit:476-478`, `services/ai_assistant.rs:665`, `:696`). | A9b |
| 31.9 | AI admin UI and usage dashboard | config | **part done (A9b)** | `trovato_ai` enabled by `demo-bootstrap.sh`; `/admin/system/ai-providers`, `ai-budgets`, `ai-chat` and `ai-assistant` are 303 to a visitor and 200 to the administrator in `verify-demo.sh`; the usage dashboard showed live usage (screenshots in the A9b run). Not done: the kernel removed its AI features screen (`6e58a9b`) and has no screen for field rules or rate limits (`G-AI-ADMIN-HAS-NO-FEATURES-RULES-OR-RATE-LIMIT-SCREENS`). `docs/AI.md` documents each screen. | A9b |
| 31.10 | MCP server | kernel | **kernel has it, Ritrovo does not use it** | `crates/mcp-server`. | A9b |
| 31.11 | VectorStore trait and pgvector | kernel | **kernel has it, Ritrovo does not use it** | `trait VectorStore`, `crates/kernel/src/services/vector_store.rs:56`. The demo's Postgres image is `postgres:16-alpine`, without pgvector. | A9b |

### Epic 4: From Demo to Data-Driven (Part 2)

| # | The brief promises | Where | Status | Evidence | Closes in |
| :- | :- | :- | :- | :- | :- |
| E4.1 | Plugin scaffold and SDK basics | plugin | **done** | All five plugins build against the public SDK and are enabled (`scripts/verify-demo.sh` plugin checks passed); `/admin/plugins` lists them. | A4 |
| E4.2 | Cron-driven conference import | plugin | **done** | 263 batches queued, drained by cron, 5,656 conferences; `/admin/config/importer` shows state and "Jobs waiting 0". Caveats are P3's. | A4 |
| E4.3 | Hierarchical topic taxonomy | config | **done** | The brief's tree exactly: 44 terms in `demo/config/tag.*.yml`, three levels, with Kotlin cross-listed under both JVM and Mobile. `verify-demo.sh` counts the terms, asserts a grandchild link exists and asserts a term with two parents exists. The confs.tech mapping is a data file, `plugins/ritrovo_importer/data/confs-tech-topics.json`; three feeds (general, opensource, testing) map to no term because the brief's tree has none, and the importer's admin screen names them rather than counting them as failures. | A4 |
| E4.4 | Advanced gathers with exposed and contextual filters | config, template | **done except the speaker relationship** | `/location/{country}` and `/location/{country}/{city}` have templates of their own and render conference cards; the raw column dump is gone, and `verify-demo.sh` asserts `search_vector` is absent and cards are present. The two tile gathers exist, with templates and aliases of their own: `/conferences/this-month` and `/cfps/closing-soon`. Both are bounded by count rather than by the brief's date window, because a gather can express "today" and nothing else (`G-NO-RELATIVE-DATE-FILTER-VALUES`). The brief's speaker relationship on the upcoming gather is not expressible either: `QueryRelationship` joins a table on two columns (`crates/kernel/src/gather/types.rs:432-450`) and the reference is a uuid array inside JSONB. | A4 |
| E4.5 | Full-text search | config | **done (A9)** | Server search (`/search` without JavaScript, `/api/v1/search`) and the Pagefind page both find conferences by title, description, city and topic, and speakers by name and bio; asserted by `verify-demo.sh` and by `cron_writes_the_title_and_topic_labels_search_reads_at_the_brief_s_weights` (`plugins/ritrovo_importer/tests/ritrovo_importer_host.rs`). | A9 |

### Epic 5: Look and Feel (Part 3)

| # | The brief promises | Where | Status | Evidence | Closes in |
| :- | :- | :- | :- | :- | :- |
| 34.1 | Render tree and conference templates | template | **done (A9)** | `item--conference.html` renders no `children` and so no field dump (`verify-demo.sh` asserts no `field__label`), and lays out the title, dates, location, online flag, language badge, CFP badge, topic pills, description (the seeded Italian conferences' too, in either language), logo, venue gallery, links, deadline and speakers. The CFP badge is drawn by the template from `ritrovo_cfp`'s rule because the plugin's own lands in `children` (`G-RENDER-CHILDREN-MIXES-FIELD-DUMP-AND-PLUGIN-OUTPUT`); the pill labels come from a generated include because the kernel resolves no tag for a template (`G-ITEM-ROUTE-DOES-NOT-RESOLVE-CATEGORY-TAGS`). Both asserted by `verify-demo.sh`. | A9 |
| 34.2 | File uploads with security validation | config | **buildable, not built (A11)** | **[corrected]** the audit recorded this as available and unused. A demo cannot use it: no config entity can create a file (`G-NO-FILE-CONFIG-ENTITY`), and a file uploaded any other way is never promoted out of temporary and is deleted six hours later, because `mark_permanent_batch` is called only from the admin content form (`G-CONFIG-IMPORT-NEVER-PROMOTES-A-FILE`). So an image reaches a Trovato site only by a human using the admin UI. `logo`, `venue_photos`, `schedule_pdf` and `headshot` are declared and their templates render them when present. **A11:** A file config entity, so a config set can ship files (Trovato `ecb1eca`, B72.2), and files promoted on every write path (`c578da4`, B72.1). Ritrovo's conference and speaker templates now draw files from `referenced_files`; no file is shipped, so nothing renders one yet. Listings cannot draw one at all (new: `G-GATHER-ROWS-HAVE-NO-FILE-URLS`). | A4 |
| 34.3 | Speaker type with RecordReference | config | **done** | `demo/config/item_type.speaker.yml` is the brief's: bio, headshot, website, and no forward `field_conferences`, which duplicated a reverse reference. The conference holds `field_speakers`; a speaker page lists its conferences through `reverse_references`. Six speakers are seeded and `/speakers` renders them. `verify-demo.sh` follows a seeded conference to a speaker and back. | A4 |
| 34.4 | Slots, tiles, navigation and breadcrumbs | config, template | **done (A9), data tiles through plugin code** | Header, navigation, sidebar and footer slots render; every menu link is 200 (`verify-demo.sh`, navigation and the front page sections). The data tiles are `custom_html` tiles whose bodies `ritrovo_importer`'s cron renders from SQL (`plugins/ritrovo_importer/src/tiles.rs`), because the kernel's `gather_query` tile still renders nothing (`G-TILE-GATHER-QUERY-RENDERS-NOTHING`, open); `verify-demo.sh` compares each tile's rows with the same query against Postgres. Breadcrumbs follow the topic tree three levels deep on conference and topic pages, drawn from `includes/topics.html` because the kernel's do not (`G-BREADCRUMBS-IGNORE-THE-TOPIC-TREE`); asserted on `/topics/kotlin` and on a conference filed under a third-level topic. | A9 |
| 34.5 | Weighted full-text search page | config | **done (A9), weights on the server's ranking only** | The brief's weights, read back out of the tsvector by `verify-demo.sh` and the importer host test: title A, description B, city C, topics C. Topics are `field_topic_names`, written by `ritrovo_importer` from the tag tree, because a tag uuid is not a word (`G-SEARCH-CANNOT-INDEX-CATEGORY-LABELS`); the brief itself calls the field "topics (denormalized)". Country is also C, an addition the brief does not list. The browser's Pagefind results ignore the letters (`G-PAGEFIND-IGNORES-FIELD-WEIGHTS`), and a `Blocks` description indexes its JSON keys (`G-SEARCH-TRIGGER-INDEXES-BLOCKS-JSON`). | A9 |
| 34.6 | Premium theme with design tokens | template | **done (A9)** | Ritrovo's `page.html` links `docs/tutorial/static/css/ritrovo.css` after the kernel's `theme.css`: `--rt-*` tokens, dark mode by `prefers-color-scheme` (the tokens and the kernel's gray scale redefined), a phone layout, `.cfp-badge` (open, closing, urgent, contrast-paired) and `.lang-badge`, topic pills and tiles. `/` renders Ritrovo's `page--front.html` in English and under `/it/`. `verify-demo.sh` asserts the stylesheet is linked and served with those rules and that the skip link, header, main, footer and labelled navs are present; there is no kernel accessibility check to run (`G-NO-ACCESSIBILITY-CHECK`). Screenshots: light, dark, phone. | A9 |

### Epic 6: The Editorial Engine (Part 4)

| # | The brief promises | Where | Status | Evidence | Closes in |
| :- | :- | :- | :- | :- | :- |
| 35.1 | Registration, login, sessions, and three test users | config, plugin | **done (A5)** | `editor_alice`, `publisher_bob` and `viewer_carol` exist, are active, carry the editor, publisher and viewer roles, and log in; `verify-demo.sh` asserts all three. Created by `ritrovo_access`'s `tap_install`, not by registration: the kernel counts one registration against the rate limit twice, so three per hour admits one (`G-RATE-LIMIT-COUNTED-TWICE`), and nothing in the kernel activates a user or gives it a role. | A5 |
| 35.2 | Role-based permissions with plugin Grant and Deny | config, plugin | **done (A5), with a documented substitution** | Five roles ship in `demo/config/role.*.yml` and import into any Trovato database. Grant and Deny work: an Incoming conference is 404 to a visitor and to `viewer_carol`, 200 to `editor_alice`, decided by `tap_item_access`. None of the brief's seven permission names is grantable (`G-PERM-TAP-NOT-DISPATCHED`), so the roles carry kernel permissions and the plugin reads those: `edit any content` for an editor, `delete any content` for a publisher. Coarser than the brief, named as such in the role files, the plugin and `docs/EDITORIAL.md`. | A5 |
| 35.3 | Incoming, Curated, Live workflow with enforced transitions | config, plugin | **done (A5), Ritrovo builds what the kernel lacks** | The importer lands on Incoming (5,561 on the verified run), a demo seed publishes a sample, and `/admin/content/editorial` moves a selection between stages with the workflow enforced by a transition table. Verified end to end: publisher promoted Curated to Live and an anonymous visitor then saw the conference. The kernel still moves no item between stages (`G-NO-ITEM-STAGE-TRANSITION`); this is a plugin writing `stage_id` through the raw-SQL host call, with the costs recorded there. | A5 |
| 35.4 | Revision history with revert, five scenarios | kernel | **part done (A11)** | Worse than first recorded: `/item/{id}/revisions` is 500 for **every** item created through the kernel, not only edited ones, because `Item::create` writes an initial revision and both queries omit two columns the struct requires. Reproduced on the demo: 200 with zero revisions, 500 with one. Revert 500s at the same line and its only button is on that page. No compare and no draft preview exist at all. Ritrovo deliberately did not build a replacement; see `docs/EDITORIAL.md` Walkthrough 4. (`G-REVISION-HISTORY-500`, `G-REVISION-NO-COMPARE`) **A11:** `an_editor_lists_previews_compares_and_reverts_revisions` (`plugins/ritrovo_access/tests/ritrovo_access_host.rs`): an editor's save writes a revision with its log message, and the editor gets the history, a preview and a comparison, and reverts (Trovato `4c75594`, B70). On the demo the history of an edited Incoming conference is 200. `G-REVISION-HISTORY-500` and `G-REVISION-NO-COMPARE` are fixed. Not checked: the brief's draft-while-live, cross-stage and emergency-unpublish scenarios, and whether a revert keeps the history it reverts. | A5 |
| 35.5 | Admin content list with filters and bulk actions | kernel, plugin | **done, and it is not where the brief wants it** | `/admin/content` filters and offers bulk publish, unpublish and delete, administrator only. Its `publish` sets `status`, not stage, so it cannot publish anything off an internal stage, and a plugin cannot add a stage action to it: the allowlist is three literals with no seam (`G-BULK-ACTIONS-NOT-EXTENSIBLE`). Bulk Curated to Live therefore lives on Ritrovo's own screen. No progress display anywhere; nothing executes a batch (`G-BATCH-NO-EXECUTOR`). | A5 |

### Epic 7: Forms and User Input (Part 5)

| # | The brief promises | Where | Status | Evidence | Closes in |
| :- | :- | :- | :- | :- | :- |
| 36.1 | Form API pipeline and conference edit form, with `tap_form_alter` | kernel, plugin | **half done (A6), the rest blocked on the kernel** | **The edit form is done and it is configuration**: `/item/{id}/edit` renders a widget for every field `item_type.conference.yml` declares, with CSRF, and Ritrovo writes no code for it. Two host-in-the-loop tests now drive it as an editor (`plugins/ritrovo_access/tests/ritrovo_access_host.rs`) and pin what it costs: saving keeps every field the form rendered and **destroys `field_topics`, `field_speakers` and `field_venue_photos`**, because the submission handler rebuilds the field map from the POST body and the update writes it whole (`G-FORM-SAVE-DROPS-EVERY-FIELD-THE-FORM-DID-NOT-RENDER`, `G-NO-CATEGORY-REFERENCE-FIELD-KIND`, `G-CARDINALITY-IS-INERT`). `tap_form_alter` still never fires (`G-FORM-TAPS-UNREACHABLE`). Walkthrough: `docs/FORMS.md` section 1. **A11:** The save no longer destroys anything: `saving_the_edit_form_keeps_every_field_including_the_ones_it_did_not_render` now asserts that `field_topics`, `field_speakers` and `field_venue_photos` survive. The form taps: the kernel's changelog says every content form goes through `FormService` (`044c5dc`, B71.3), so `tap_form_alter` should fire. Buildable, not built. | A6 |
| 36.2 | WYSIWYG for rich text | config | **done (A11)** | The audit called this "kernel has it, Ritrovo does not use it". That is wrong for the role the brief cares about. `field_description` **is** a `Blocks` field, and `FormBuilder` **does** render the block editor's container on `/item/{id}/edit` — but that route emits a bare HTML document with no `<script>` of any kind, so nothing ever attaches and an editor sees an empty div over a hidden input. The editor works only on the administrator content form, which editors cannot reach (`G-ADMIN-SCREENS-ARE-ADMIN-ONLY`). New entry: `G-ITEM-EDIT-FORM-LOADS-NO-ASSETS`. A plugin cannot fill the gap either: no host function exposes a text format to a plugin (`G-NO-TEXT-FORMAT-HOST-API`). **A11:** The editor's `/item/{id}/edit` loads `block-editor.js` with the `Blocks` field's container (Trovato `044c5dc`, B71.3), asserted by `the_conference_edit_form_is_generated_from_the_type_and_loads_the_block_editor`. `G-ITEM-EDIT-FORM-LOADS-NO-ASSETS` is fixed. | A6 |
| 36.3 | Conditional CFP fields, add another, topic autocomplete | kernel | **buildable, not built (A11)** | Add-another and record autocomplete exist for administrators (`crates/kernel/src/routes/admin.rs:386-418`, `routes/api_v1.rs:73`); no conditional fields (`G-AJAX-ADMIN-ONLY-NO-CONDITIONAL-FIELDS`). **A11:** AJAX for every editor and conditional fields (`visible_when`) (Trovato `65cf830`, B71.4). Not configured or tested here. | A6 |
| 36.4 | Three-step submission form landing in Incoming | plugin | **done (A6), with two fields the kernel cannot carry** | `/conferences/submit` in `ritrovo_forms`: three steps, each validated before the next renders, state in a plugin-owned table, every step a plain `<form>` with the token in the body and no JavaScript anywhere. Confirming creates the conference on Incoming with the submitter as author, and the confirmation states the moderation expectation. Verified by seven host-in-the-loop tests and by `verify-demo.sh`, which drives all three steps on every demo run. The kernel's own form pipeline is still reachable from no route (`G-FORM-TAPS-UNREACHABLE`), so none of this goes through it. **Step 2 omits the logo** (`G-FILE-NO-HOST-API`) **and topics** (`G-NO-CATEGORY-REFERENCE-FIELD-KIND`), and the review step says so. The insert binds `stage_id` itself (`G-NO-ITEM-STAGE-TRANSITION`, `G-DEFAULT-STAGE-IGNORED-ON-CREATE`) and so fires no taps (`G-ITEM-API-BYPASSES-ITEM-SERVICE`). Walkthrough: `docs/FORMS.md` section 3. | A6 |
| 36.5 | `ritrovo_cfp`: badge, date validation, `cfp_closing_soon` events | plugin | **half done (A6)** | Badge done (P5). **The date rule is written and tested**: `date_complaint` in `plugins/ritrovo_cfp/src/lib.rs`, unit-tested at its boundaries, dispatched from `tap_item_presave`. It can only report: no tap can refuse a save and nothing can put a message on the form (`G-PRESAVE-CANNOT-REFUSE`, `G-FORM-TAPS-UNREACHABLE`), pinned by `the_kernel_saves_contradictory_cfp_dates_because_no_tap_can_refuse`. Enforcement lands on the submission form Ritrovo serves. The `cfp_closing_soon` event still cannot reach another plugin's queue (`G-QUEUE-NO-CROSS-PLUGIN`). | A6 |
| 36.6 | `ritrovo_access`: stage gating and field-level access | plugin | **done (A5)** | Both halves work and both are enforced by the kernel. Stage gating: `tap_item_access` decides, and a conference on Incoming is 404 to a visitor and to `viewer_carol`, 200 to `editor_alice`, asserted by `verify-demo.sh` and by a host suite driven through the real roles. Field access: `tap_field_access` denies `field_editor_notes` to non-editors, asserted by a host test that reads `Item.fields` rather than HTML. The permission names remain substitutes (`G-PERM-TAP-NOT-DISPATCHED`). | A5 |
| 36.7 | Profile form: display name, bio, avatar, timezone, preferences | kernel, plugin | **half done (A6), Ritrovo builds what the kernel lacks** | The kernel's `/user/profile` is a fixed struct with no tap anywhere along it, so nothing can be added to it (`G-USER-PROFILE-NOT-EXTENSIBLE`). Ritrovo serves its own page instead: `/user/bio`, in the new `ritrovo_forms` plugin, over a plugin-owned table, gated on a kernel permission, working with JavaScript off, verified by nine host-in-the-loop tests. **Bio is done; avatar is not** (`G-FILE-NO-HOST-API`: a plugin cannot accept a file), and the bio is plain text rather than rich text (`G-NO-TEXT-FORMAT-HOST-API`). Walkthrough: `docs/FORMS.md` section 2. | A6 |

### Epic 8: Community and Plugin Communication (Part 6)

| # | The brief promises | Where | Status | Evidence | Closes in |
| :- | :- | :- | :- | :- | :- |
| 37.1 | Threaded comments on conferences | config | **done (A7)** | `demo/config` grants `post comments` and `edit own comments` to `authenticated user`, which no role could hold until 0.103.0 dispatched `tap_perm` (`G-PERM-TAP-NOT-DISPATCHED`, fixed). A signed-in member is served the form instead of "Log in to post a comment", and a reply renders under its parent with the kernel's own `comment--depth-1` class. Host test: `a_member_posts_a_comment_on_a_conference_and_a_reply_nests_under_it`; rendered half in `verify-demo.sh`. No template was needed. | A7 |
| 37.2 | Comment moderation queue for editors | config | **done (A7)** | Was blocked on `G-ADMIN-SCREENS-ARE-ADMIN-ONLY`; **0.103.0 moved all six comment routes onto `require_permission("administer comments")`** and `/admin` itself onto `access administration pages`. `demo/config` grants both to the editor and publisher roles, so this closed as configuration with no screen built. `verify-demo.sh` asserts editor_alice opens it and viewer_carol does not. | A7 |
| 37.3 | Subscribe toggle and My Subscriptions page | plugin | **part done (A7)** | The page is built: `/user/{uid}/subscriptions` lists a member's own subscriptions and refuses another's, with subscribe and unsubscribe as `_token` form posts, gated on `manage own subscriptions` — this plugin's own permission, grantable since `G-PERM-TAP-NOT-DISPATCHED` was fixed. Rows land in the kernel's `user_subscriptions`. **The toggle is not on the conference page and cannot be:** the item template's context carries no viewer (new: `G-ITEM-TEMPLATE-HAS-NO-VIEWER`) and a `tap_item_view` toggle renders into `children`, which Ritrovo's conference template drops (`G-RENDER-CHILDREN-MIXES-FIELD-DUMP-AND-PLUGIN-OUTPUT`). The AJAX half stays out (`G-AJAX-ADMIN-ONLY-NO-CONDITIONAL-FIELDS`). | A7 |
| 37.4 | `ritrovo_notify`: notifications and email digests | plugin | **blocked on the kernel** | `G-MAIL-CANNOT-REACH-A-USER`, `G-MAIL-UNAVAILABLE-IN-BACKGROUND`, `G-NO-USER-DIRECTORY`; imported changes fire no `tap_item_update` (`G-ITEM-API-BYPASSES-ITEM-SERVICE`). **A11:** Still blocked, on a different entry. Mail to one member (`d02c721`), mail from `tap_cron` and `tap_queue_worker` (`6ce4bf2`) and the user directory (`2ac2d14`) are in the kernel. `ritrovo_importer` uses the directory: the Recent Comments tile names authors through `host::load_users`. But imported changes still fire no `tap_item_update`: an item write from the background fails (new: `G-BACKGROUND-ITEM-API-HAS-NO-ITEM-SERVICE`), so the importer stays on raw SQL. | A7 |
| 37.5 | Plugin-to-plugin through a shared queue | plugin | **blocked on the kernel** | `G-QUEUE-NO-CROSS-PLUGIN`. The kernel's alternative is `plugin-api` invocation. **A11:** Still blocked: shared queues are designed (Trovato `29c5575`, B68.1, `docs/design/plugin-item-plumbing.md` section 3) and not built; no `push-to` host function exists. | A7 |
| 37.6 | Comment notifications to subscribers | plugin | **buildable, not built (A11)** | `tap_comment_insert` is dispatched (`crates/kernel/src/services/comment.rs:67`); delivery is `G-MAIL-CANNOT-REACH-A-USER`. The kernel already mails the item's author (`crates/kernel/src/routes/comment.rs:1092-1157`). **A11:** `tap_comment_insert` is dispatched, and mail can now reach one member by id (`d02c721`, B73.4). Not built. | A7 |

### Epic 9: Going Global (Part 7)

| # | The brief promises | Where | Status | Evidence | Closes in |
| :- | :- | :- | :- | :- | :- |
| 38.1 | Multilingual content model | config | **done** | The kernel's model, used as it is: one record per conference plus an `item_translation` row per other language, written by the kernel's translate screen or by config import (`G-TRANSLATION-NO-WRITE-PATH`, narrowed at 0.103.0). `field_language` says what a conference is held in, set by the feed, then a detector. Verified 2026-11-14: `/item/37c34d73-…` is English, `/it/item/37c34d73-…` its Italian translation, title and description. The default-language half of an Italian-written record does not render (`G-TRANSLATION-OVERLAY-SKIPS-THE-DEFAULT-LANGUAGE`), which is why the seed's records are English. | A8 |
| 38.2 | Language routing, localized aliases, switcher, hreflang, UI strings | config, template | **done except UI strings and the topics page** | Prefix routing, translated aliases (`/conferences/<slug>` and `/it/conferenze/<slug>` for all 15 seeded conferences), hreflang (the kernel's) and a switcher in `item--conference.html`, followed both ways by `verify-demo.sh`; it offers the other language even where no translation exists (`G-ITEM-TEMPLATE-RENDERED-BEFORE-ITS-TRANSLATIONS`). The three doubled `/it/…` alias rows are gone. Blocked: every UI string stays English (`G-LOCALE-STRINGS-NEVER-LOADED`, re-checked at 0.103.0); `/it/argomenti` renders `lang="en"` (`G-TOPICS-PAGE-IGNORES-LANGUAGE`); pathauto writes no language (`crates/kernel/src/services/pathauto.rs:167`, `:204`), so imported conferences have no alias in either language. | A8 |
| 38.3 | `ritrovo_translate`: detection, queue, side-by-side translation | plugin | **done** | Detection in the importer (feed first) and in `tap_item_presave` for conferences people create. The queue is `/admin/content/translations` (401 / 403 / 200 to visitor / reader / editor), listing upcoming conferences with no translation in the other language, each linked to the kernel's side-by-side translate screen, which saves. Verified end to end by `verify-demo.sh` as editor_alice. Two kernel limits remain: a `Blocks` description is not on the screen (`G-TRANSLATE-SCREEN-TEXT-FIELDS-ONLY`), and an English translation of an Italian-written conference saves and does not render (`G-TRANSLATION-OVERLAY-SKIPS-THE-DEFAULT-LANGUAGE`). | A8 |
| 38.4 | About 20 Italian conferences with English translations | config | **done, the other way round** | 15 Italian conferences, every one fully translated: records in English, `field_language: it`, and an `item_translation.<uuid>.it.yml` each with title and description. `/` shows English and `/it/` Italian. Shipped English-first because an Italian record's English translation never renders (`G-TRANSLATION-OVERLAY-SKIPS-THE-DEFAULT-LANGUAGE`). The two-languages-in-one-field JSONB is gone. | A8 |
| 38.5 | REST endpoints under `/api/v1/conferences`, topics, speakers, subscribe | config, plugin | **read half done for conferences and topics; speakers not built; writes blocked** | `docs/API.md`. `/api/v1/conferences` is a URL alias onto the listing's own gather (paged; topic, country, online and language filters); `/api/v1/conferences/{id}` and `/api/v1/topics` (a tree) are `ritrovo_importer` `tap_api` routes, because a gather cannot take a path parameter and the kernel returns tags flat; search is the kernel's `/api/v1/search`. Anonymous callers get Live only, proved by `verify-demo.sh` against an upcoming Incoming conference, and a host test. Speakers were not in A8's scope. Writes, subscribe included, would skip access checks and revisions (`G-ITEM-API-BYPASSES-ITEM-SERVICE`). The kernel's ad-hoc gather route leaks editors' notes to anyone (`G-ADHOC-GATHER-BYPASSES-FIELD-ACCESS`), which no Ritrovo code can close. **A11:** Writes: plugin item writes from a request now go through `ItemService` (Trovato `db45018`, B68.2), so a `tap_api` write route would check access and write revisions. Buildable, not built. Speakers are still not built. The list shape is now `{data,total,page,per_page}`, `docs/API.md`. | A8 |
| 38.6 | API keys and per-role rate limits | kernel, config | **part done (A11)** | `/api/tokens` issues a token and no request can use it: the JWT middleware refuses every bearer before the token middleware runs (`G-API-TOKENS-NEVER-AUTHENTICATE`, observed on the demo). The `api` limit is 100 a minute for everyone, now overridable site-wide by `TROVATO_RATE_LIMIT_API` or `rate_limit.api` but not per role (`G-API-RATE-LIMITS-FIXED`, narrowed at 0.103.0). Documented in `docs/API.md`. **A11:** Keys work: `verify-demo.sh` issues editor_alice a token, gets 200 from `/api/v1/tokens` with it alone and 401 with none or a forged one, and revokes it (`G-API-TOKENS-NEVER-AUTHENTICATE` fixed, Trovato `d7a014e`). Per-role rate limits exist (`fcb3522`, B69.1) and are not configured: buildable, not built. | A8 |

### Epic 10: Production Ready (Part 8)

| # | The brief promises | Where | Status | Evidence | Closes in |
| :- | :- | :- | :- | :- | :- |
| 39.1 | Two-tier caching with tag invalidation | kernel | **buildable, not built (A11)** | The kernel's item cache is invalidated by its own writes and by nothing a plugin can call, and every Ritrovo write (the importer, the submission form, search text, tiles) is raw SQL: `G-PLUGIN-CANNOT-INVALIDATE-THE-ITEM-CACHE`. Measured, not assumed: `a_kernel_edit_shows_at_once_a_raw_write_waits_for_the_cache_and_no_view_leaks` (`plugins/ritrovo_access/tests/ritrovo_access_host.rs`) drives the item route; a form edit is on the page at once, a raw write waits for the TTL, and an editor's read of a Curated conference never reaches an anonymous request. `verify-demo.sh` shows the raw-write staleness on the demo (10 s). `docs/CACHING.md` has the hit and miss commands. **A11:** A plugin's raw write now bumps the item cache generation (`crates/kernel/src/host/db.rs:306`, Trovato `1dc0467`, B95), which would close `G-PLUGIN-CANNOT-INVALIDATE-THE-ITEM-CACHE` for Ritrovo's writes. Not yet shown from here: the host test writes with `sqlx` directly, not through a plugin, and still measures the TTL, and the demo still sets `CACHE_TTL_ITEMS=10`. | A9 |
| 39.2 | Batch operations with progress | kernel, plugin | **partly done (A5); progress blocked** | Bulk stage moves work on Ritrovo's editorial screen, inline over the selection, the same way the kernel's own bulk publish works. There is no progress display and cannot be: nothing anywhere executes a batch, and `update_progress`, `complete` and `fail` have no callers outside their own module (`G-BATCH-NO-EXECUTOR`). A plugin also cannot add a stage action to `/admin/content` (`G-BULK-ACTIONS-NOT-EXTENSIBLE`). **A11:** Still blocked for progress: the batch executor is designed (Trovato `29c5575`, B68.1, section 4) and not built. The kernel's own bulk actions work on a real selection (`dd03141`). | A5 |
| 39.3 | S3-compatible storage | kernel | **blocked on the kernel** | `G-S3-STORAGE-REMOVED`. **A11:** Still blocked: the S3 backend was dropped for 1.0 (`crates/kernel/src/file/storage.rs:5`). | A4 |
| 39.4 | Cron with distributed locking | kernel | **done** | `scripts/demo-cron.sh` posts to `/cron/{key}`; the kernel log shows "acquired cron lock, running tasks" each minute. | A4 |
| 39.5 | Prometheus metrics and health check | kernel | **done** | `/metrics` and `/health` answer 200 in the demo. | A4 |
| 39.6 | A full test suite running in CI | plugin | **done** | `.github/workflows/ci.yml` runs every plugin through the real kernel; `demo.yml` runs the one-command demo weekly; CI green on `9d5dd35`. | A4 |
| 39.7 | Configuration import and export | config | **done** | The demo is assembled by `trovato config import` (`scripts/demo-bootstrap.sh:131-153`). Ownership is the recommendation above. | A4 |

## Status by "What It Demonstrates" line

| # | The brief promises | Where | Status | Evidence | Closes in |
| :- | :- | :- | :- | :- | :- |
| D1 | Item Types and CCK: two types, JSONB fields, RecordReference, file fields | config | **buildable, not built (A11)** | Two types, JSONB fields and RecordReference are done (29.1, 34.3). File fields are declared and unusable (34.2). **A11:** 34.2. | A4 |
| D2 | Categories: three-level topics, nested queries, breadcrumbs, browsing | config, template | **done (A9)** | The tree, the nested queries and the browsing are done (E4.3). The breadcrumb on a conference page is not: an item template can print a topic's uuid and nothing else, because the item route resolves record references but never a category tag (`G-ITEM-ROUTE-DOES-NOT-RESOLVE-CATEGORY-TAGS`). The gather side has breadcrumbs. **A11:** This row was stale. A9 built the conference breadcrumb, Home › Languages › JVM › Kotlin › the conference, from the generated `includes/topics.html`, and `verify-demo.sh` asserts a three-level topic breadcrumb on a conference and on a topic page (34.4, D18). The kernel still gives the template only the uuid (`G-ITEM-ROUTE-DOES-NOT-RESOLVE-CATEGORY-TAGS`, not rechecked). | A4 |
| D3 | Stages and Revisions: workflow, draft, preview, revert, stage a new version | kernel, plugin | **half done (A5)** | The workflow half is done and demonstrable: 35.3. The revision half is not, and none of draft, preview, revert or compare works on this kernel: 35.4. | A5 |
| D4 | Gather: six definitions, exposed and contextual filters, relationships, paging | config | **done except relationships** | Eight definitions now, the brief's six plus its two tile gathers, all with templates and routes. Paging is the brief's 25. The speaker relationship is not expressible (E4.4). | A4 |
| D5 | Render Tree: conference card, detail page, speaker card, CFP badge, topic pills, profile | template | **done (A9)** | Cards (A4), the detail page and its badge and pills (34.1), speaker pages (34.3), the member bio page (36.7). Badge colours are 34.6. | A9 |
| D6 | Form API: edit form, multi-step submission, profile form, subscription form | kernel, plugin | **part done (A6)** | Three of the four are usable: the edit form (36.1), the profile form (36.7) and the multi-step submission (36.4). The subscription form is 37.3, in A7. The kernel's own Form API is reachable from no route in any of them (`G-FORM-TAPS-UNREACHABLE`); every form Ritrovo serves is a `tap_api` route with a `_token` in the body. | A6 |
| D7 | WYSIWYG for descriptions and bios, AJAX add another | config | **kernel has it, Ritrovo does not use it** | 36.2, 36.3. | A6 |
| D8 | Five plugins demonstrating the full tap lifecycle | plugin | **blocked on the kernel** | All five install and enable; of the brief's 19 intended taps, 4 work and 9 are blocked (P1 to P19). **A11:** Still blocked: the P rows below. P18 was stale and is done; P8, P11, P12 and P16 are buildable, not built; P9 is blocked on `G-BACKGROUND-ITEM-API-HAS-NO-ITEM-SERVICE`. | A7 |
| D9 | Plugin-to-plugin through the notifications queue | plugin | **blocked on the kernel** | 37.5. **A11:** Still blocked: 37.5. | A7 |
| D10 | Cron: daily import, digest emails, temp file cleanup | plugin | **blocked on the kernel** | Import and cleanup run (P1; `crates/kernel/src/cron/tasks.rs:181-184`); digests are 37.4. **A11:** Still blocked: digests are 37.4. | A7 |
| D11 | Queue: import validation queue with `tap_queue_info` and `tap_queue_worker` | plugin | **done** | 269 import jobs queued and drained on the current demo. The worker's error handling is fixed: see P3. | A4 |
| D12 | Tiles: CFPs closing soon, this month, topic cloud, recent comments, my subscriptions | config, plugin | **buildable, not built (A11)** | CFPs Closing Soon, Conferences This Month, the Topic Cloud with counts and Recent Comments render real data on the front page and the sidebar, each checked row for row against Postgres by `verify-demo.sh` (plugin code, `tiles.rs`, since `G-TILE-GATHER-QUERY-RENDERS-NOTHING` is open). My Subscriptions is a role-limited signpost to `/user/bio`, which links the member's own list: a tile cannot vary by viewer and no template knows the viewer's uid (`G-NO-VIEWER-ID-IN-TEMPLATES`). **A11:** Page templates now name their viewer (`c8d1883`, B74.1; `c596e53`, B103, `user_name`), which is what `G-NO-VIEWER-ID-IN-TEMPLATES` asked for. Not rechecked or built: My Subscriptions is still the signpost. The four data tiles are rechecked: `verify-demo.sh` compares them with Postgres on the pin. | A9 |
| D13 | Slots and the admin UI for placing tiles, with role and path visibility | config | **done (A9)** | Every tile's region, weight and visibility is in `demo/config/tile.*.yml` and editable at `/admin/structure/tiles`: path rules (`front_upcoming` on `/` only, the topic cloud on `/`, `/conferences` and `/topics*`, recent comments on `/` and `/item/*`), `paths_exclude: [/admin*]` on two, and one role rule, My Subscriptions for `authenticated user`, which `verify-demo.sh` asserts is shown to a member and not to a visitor. Only the kernel's three synthetic roles can be used (`G-TILE-ROLES-ARE-SYNTHETIC`). | A9 |
| D14 | Search: weighted fields, stage-aware, across conferences and speakers | config | **buildable, not built (A11)** | Weighted (34.5) and across both types (E4.5), in the browser and on the server. Stage-aware for visitors: the Pagefind index holds Live only (its page count equals the Live count) and `verify-demo.sh` asks both indexes, anonymously, for an Incoming and a Curated title and gets neither. `search_returns_live_to_everyone_and_another_stage_only_to_an_administrator_on_it` (`plugins/ritrovo_access/tests/ritrovo_access_host.rs`) indexes one conference per stage: anonymous and editor get Live only, and only an administrator switched to Incoming gets Incoming and Live, never Curated. An editor cannot search the stages they edit (`G-STAGE-SWITCH-IS-ADMIN-ONLY`). **A11:** A `use stages` permission and one stage resolver (Trovato `20bd30a`, B69.3). Ritrovo grants it to no role, so `search_returns_live_to_everyone_and_another_stage_only_to_an_administrator_on_it` still passes as written: an editor searches Live. | A9 |
| D15 | Users and Auth: registration, login, profiles with bio and avatar | kernel, plugin | **part done (A6)** | 35.1 for accounts. Profiles with a bio are done, on Ritrovo's own page (36.7); avatars are blocked (`G-FILE-NO-HOST-API`). | A6 |
| D16 | Permissions: five roles, Grant and Deny, field visibility, stage scope | plugin, config | **done (A5)** | 35.2, 36.6. **A11:** This row was stale: its two stories, 35.2 and 36.6, are done. `the_real_roles_grant_what_the_editorial_workflow_needs` drives the real roles on the pin, and the publisher role now also carries the kernel's `publish content`, which the kernel's workflow check requires to move content on or off Live (`demo/config/variable.workflow.editorial.yml`). | A5 |
| D17 | Files: logo, gallery, headshot, PDF, local and S3, private files | config | **kernel has it, Ritrovo does not use it** | 34.2; S3 is `G-S3-STORAGE-REMOVED`. | A4 |
| D18 | Menus: main, user, admin, footer, plugin entries, breadcrumbs | config | **buildable, not built (A11)** | Main and footer are config menus; the user (account) and admin menus are written in Ritrovo's `page.html`, because only main and footer reach a template (`G-ONLY-MAIN-AND-FOOTER-MENUS-RENDER`). `tap_menu` entries arrive as `menus`: the importer's two admin screens in the admin bar for whoever may open them, `ritrovo_forms`' Submit a Conference in the account menu, all asserted and followed to 200 by `verify-demo.sh`. `ritrovo_notify`'s Your subscriptions has a parent and a `{uid}` path, and renders in no menu (`G-PLUGIN-MENU-ENTRIES-WITH-A-PARENT-RENDER-NOWHERE`, `G-NO-VIEWER-ID-IN-TEMPLATES`). Topic breadcrumbs three levels deep, as 34.4. **A11:** Menus as trees, cached per stage and language, with menu tiles rendered (Trovato `0c0f5a2`, B74.2). Not rechecked: whether a plugin entry with a `parent` now renders. The main, footer, account and admin menus are rechecked: `verify-demo.sh` follows each. | A9 |
| D19 | REST API with keys and rate limits | config, plugin | **part done (A11)** | 38.5 (read half), 38.6. `docs/API.md` is the reference; `/api/openapi.json` cannot be (`G-OPENAPI-IS-A-STATIC-LIST`). **A11:** Keys work (38.6). Per-role rate limits are buildable, not built. `/api/v1/openapi.json` now lists Ritrovo's routes (`G-OPENAPI-IS-A-STATIC-LIST` fixed, asserted by `verify-demo.sh`). | A8 |
| D20 | Revisions: log, preview, revert to N, compare | kernel | **part done (A11)** | 35.4. **A11:** 35.4. | A5 |
| D21 | i18n: translated content, UI strings, aliases, switcher | config, plugin | **done except UI strings** | 38.1, 38.2, 38.3, 38.4. UI strings: `G-LOCALE-STRINGS-NEVER-LOADED`. | A8 |
| D22 | Multi-step submission with state in PostgreSQL | plugin | **done (A6)** | 36.4. The state is in PostgreSQL as the brief asks, in `ritrovo_form_state` rather than the kernel's `form_state_cache`: that table, its writer and its expiry task all exist and a plugin can reach none of them (`G-NO-FORM-STATE-HOST-API`). | A6 |
| D23 | AJAX: conditional fields, add another, autocomplete, subscription toggle, filters | plugin, kernel | **buildable, not built (A11)** | 36.3; the toggle is 37.3; the filter form submits by full page load. Every form Ritrovo now serves works without JavaScript, which is the half of "progressive enhancement" that can be built on this kernel; the enhancement half cannot, because form AJAX is administrator-only and routes through the `FormService` no route calls (`G-AJAX-ADMIN-ONLY-NO-CONDITIONAL-FIELDS`). **A11:** 36.3. | A6 |
| D24 | Caching demonstrated through Gander | kernel | **blocked on the kernel** | Gander does not exist and `Server-Timing` carries one total (`G-NO-REQUEST-PROFILER`, narrowed at 0.103.0, open); the cache counters never move (`G-CACHE-METRICS-NEVER-COUNT`). The walkthrough is in `docs/CACHING.md`, by behaviour, and its claims are asserted as 39.1's are. **A11:** Still blocked: Gander does not exist. Narrowed: the cache counters count (Trovato `ff0257a`, B76.1), and `verify-demo.sh` asserts `/metrics` shows item cache hits (`G-CACHE-METRICS-NEVER-COUNT` fixed). | A9 |
| D25 | Batch: bulk publish Curated to Live, bulk re-import, progress | plugin, kernel | **partly done (A5)** | Bulk publish Curated to Live is done, on `/admin/content/editorial`, and verified in the demo. Progress tracking is not, and nothing runs a batch: 39.2. | A5 |

## Status by intended plugin tap

| # | The brief promises | Where | Status | Evidence | Closes in |
| :- | :- | :- | :- | :- | :- |
| P1 | `ritrovo_importer` `tap_cron`: daily fetch, diff, queue | plugin | **done** | `plugins/ritrovo_importer/src/lib.rs:682`, gated to once a day by `should_import` (`:1094-1099`); the importer screen shows "Minimum interval between runs 86400 seconds". | A4 |
| P2 | `ritrovo_importer` `tap_queue_info`: declares `ritrovo_import` | plugin | **done** | `lib.rs:804`; the kernel reads its concurrency (`crates/kernel/src/cron/mod.rs:179-193`). | A4 |
| P3 | `ritrovo_importer` `tap_queue_worker`: validate, create in Incoming, log bad data | plugin | **done (A5)** | **"logged and skipped, not silently dropped" is done in A4.** The worker is a `#[plugin_tap_result]`, so a malformed batch returns `Err`, and the kernel's retry, backoff and dead-letter tier takes it: five attempts, then `status = 'dead'` with the row kept. A host-in-the-loop test drives a batch through all five attempts and asserts the row survives. The kernel's `last_error` is a constant, so the worker writes the reason into its own state and the admin screen reads it back (`G-QUEUE-DEAD-LETTER-DISCARDS-THE-PLUGINS-ERROR`). **Creates in Incoming as of A5.** It binds `stage_id` in its own INSERT, which is the only way an item reaches a non-Live stage on this kernel. The public site does not empty, because `ritrovo_access` promotes a sample and an editor promotes the rest (`G-NO-ITEM-STAGE-TRANSITION` stands: the kernel still moves nothing itself). | A5 |
| P4 | `ritrovo_importer` `tap_plugin_install`: full historical import and category seeding | plugin, config | **done** | `tap_install` (`lib.rs:232`) queued 263 batches across 12 years. The terms come from config import rather than the plugin, which has no category host call. | A4 |
| P5 | `ritrovo_cfp` `tap_item_view`: days-left badge, green, yellow, red | plugin | **done** | "CFP Urgent, 1 day left" renders on Gerrit User Summit, classed `cfp-badge--urgent` (`plugins/ritrovo_cfp/src/lib.rs:26`). The colours need CSS (34.6). | A6 |
| P6 | `ritrovo_cfp` `tap_item_insert` and `tap_item_update`: validate dates, emit `cfp_closing_soon` | plugin | **half done (A6)** | The date rule is implemented and dispatched, through `tap_item_presave` rather than the insert and update taps, because presave is the one that runs before the write. It reports and cannot refuse (`G-PRESAVE-CANNOT-REFUSE`), which is pinned by a test. `cfp_closing_soon` is still not emitted (`G-QUEUE-NO-CROSS-PLUGIN`), and imports fire no tap at all (`G-ITEM-API-BYPASSES-ITEM-SERVICE`). | A6 |
| P7 | `ritrovo_notify` `tap_menu`: `/user/{uid}/subscriptions` | plugin | **done (A7)** | Three `MenuRoute::api` entries, every callback served by `tap_api` in the same change, asserted by `every_registered_callback_is_served`. Only the GET is visible, under `/user`. | A7 |
| P8 | `ritrovo_notify` `tap_item_view`: Subscribe toggle for signed-in users | plugin | **buildable, not built (A11)** | Not built, and would be invisible if it were: a view tap's output is appended to `children`, which Ritrovo's conference template does not render because that string also carries the kernel's raw field dump. The template cannot render the toggle either, because its context carries no viewer. `G-RENDER-CHILDREN-MIXES-FIELD-DUMP-AND-PLUGIN-OUTPUT` + `G-ITEM-TEMPLATE-HAS-NO-VIEWER`; either fix opens it. The control is served on the plugin's own page instead. **A11:** Templates name their viewer and get plugin output on its own (Trovato `c8d1883`, B74.1), which addresses both `G-ITEM-TEMPLATE-HAS-NO-VIEWER` and `G-RENDER-CHILDREN-MIXES-FIELD-DUMP-AND-PLUGIN-OUTPUT`. Not built or rechecked. | A7 |
| P9 | `ritrovo_notify` `tap_item_update`: queue a notification when a subscribed conference changes | plugin | **blocked on the kernel** | Not implemented; the changes that matter come from the importer and fire no tap (`G-ITEM-API-BYPASSES-ITEM-SERVICE`). **A11:** Still blocked, on a new entry. Plugin item writes go through `ItemService` (Trovato `db45018`, B68.2), but only on a request. From the importer's queue worker `save_item` fails with `ERR_NO_SERVICES`, because the background services never bind an item service (`G-BACKGROUND-ITEM-API-HAS-NO-ITEM-SERVICE`, with the reproduction). So imported changes still fire no tap. | A7 |
| P10 | `ritrovo_notify` `tap_queue_info`: declares `ritrovo_notifications` | plugin | **done (A7)** | Now the JSON array the kernel reads, with `concurrency` and no key the kernel ignores. Still no worker, deliberately: the producer is blocked (P9, 37.5) and the declaration is what the eventual fix attaches to. A declared queue with no worker is silent and costs nothing (new: `G-QUEUE-WITHOUT-A-WORKER-IS-SILENT`). | A7 |
| P11 | `ritrovo_notify` `tap_queue_worker`: send email or queue for digest | plugin | **buildable, not built (A11)** | Not implemented. `G-MAIL-CANNOT-REACH-A-USER`, `G-MAIL-UNAVAILABLE-IN-BACKGROUND`. **A11:** Mail to one member (`d02c721`) and mail from `tap_queue_worker` (`6ce4bf2`). The producer, P9, is still blocked. | A7 |
| P12 | `ritrovo_notify` `tap_cron`: daily digest emails | plugin | **buildable, not built (A11)** | Not implemented. Same entries as P11. **A11:** Mail from `tap_cron` and to one member (`6ce4bf2`, `d02c721`). | A7 |
| P13 | `ritrovo_translate` `tap_item_insert`: detect language, flag for translation | plugin | **done, in presave** | `tap_item_insert` is removed: the kernel discards its output (`G-ITEM-INSERT-OUTPUT-DISCARDED`). Detection runs in `tap_item_presave`, whose `fields` the kernel merges, and in the importer, which never reaches a tap (`G-ITEM-API-BYPASSES-ITEM-SERVICE`). "Flagged" is the queue: a conference with no translation in its target language is on it. 27 of 5,681 imported titles detected, no false positive. | A8 |
| P14 | `ritrovo_translate` `tap_item_view`: language badge and switcher | plugin | **done; the page draws its own** | Badge from `field_language`, links `/item/{id}` and `/it/item/{id}`, both marked with `hreflang` because the tap cannot know the page's language (`G-NO-REQUEST-LANGUAGE`). The conference template drops plugin output (`G-RENDER-CHILDREN-MIXES-FIELD-DUMP-AND-PLUGIN-OUTPUT`), so the visible switcher is the template's, which picks the other language from `active_language`. | A8 |
| P15 | `ritrovo_translate` `tap_cron`: process the translation queue | plugin | **done, off by default** | Drafts the next three queued conferences per cycle through `ai_request` into `ritrovo_translation_draft`, shown on the queue page, when `plugin.ritrovo_translate.assist_enabled` is `1`. Tests use a mock provider. An editor saves the reviewed draft through the kernel screen, because no host call writes a translation (`G-TRANSLATION-NO-WRITE-PATH`, the narrowed residual). | A8 |
| P16 | `ritrovo_translate` `tap_form_alter`: language selector on the edit form | plugin | **buildable, not built (A11)** | Not implemented; `G-FORM-TAPS-UNREACHABLE`. **A11:** The kernel's changelog says every content form goes through `FormService`, so form taps fire (`044c5dc`, B71.3). | A8 |
| P17 | `ritrovo_access` `tap_item_access`: Grant, Deny, Neutral by role and stage | plugin | **done (A5)** | The whole decision table is one unit test, and a host suite drives the four role-and-stage combinations that matter through roles resolved from the database the way a request resolves them, so a role file that stopped granting what the tap reads fails the build. Grants now key on the brief's permission first and a kernel permission second. | A5 |
| P18 | `ritrovo_access` `tap_perm`: declare the editorial permissions | plugin | **done (A7)** | Declared, `plugins/ritrovo_access/src/lib.rs`; still dispatched nowhere at the pinned release, confirmed by enumerating every dispatch site in the kernel (`G-PERM-TAP-NOT-DISPATCHED`). The plugin now reads its own permission first and a kernel permission second, so the day the kernel dispatches this tap the role files gain four lines and no code changes. **A11:** This row was stale. `tap_perm` has been dispatched since 0.103.0, and the roles grant `ritrovo_access`'s own permissions (`demo/config/role.*.yml`). Config import validates them, and `tap_perm_declares_the_editorial_permissions` and `the_real_roles_grant_what_the_editorial_workflow_needs` pass on the pin. | A5 |
| P19 | `ritrovo_access` `tap_item_view`: hide `editor_notes` from non-editors | plugin | **done (A5), by a different tap** | Done through `tap_field_access`, which is dispatched with the viewer and removes the field before render; the `tap_item_view` stub is deleted, having fired on every item view and done nothing. A view tap could never have done this: it can only append HTML. `G-VIEW-TAP-INPUT-CARRIES-NO-VIEWER` is amended. The kernel evaluates field access for `view` only, so the edit form is guarded by `tap_item_access` instead. | A5 |

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

**Done, 2026-09-22, on Trovato 0.103.0.** Items 1 to 4 closed; 5 to 7 are still
blocked and are listed with what they wait on. Item 8 is done.

1. ~~37.1: grant `post comments` to `authenticated user` in `demo/config`.~~
   **Done**, with `edit own comments` beside it.
2. ~~P7, 37.3: subscriptions as `tap_api` routes over `user_subscriptions`.~~
   **Done**, gated on the plugin's own `manage own subscriptions`. **P8 did not
   close** and is now blocked rather than buildable: the toggle cannot go on the
   conference page at all **[kernel: `G-ITEM-TEMPLATE-HAS-NO-VIEWER` +
   `G-RENDER-CHILDREN-MIXES-FIELD-DUMP-AND-PLUGIN-OUTPUT`]**. The control is
   served on the plugin's own page, reached from the user menu, which the kernel
   filters per viewer.
3. ~~P10: a real queue declaration~~ **Done** as the array the kernel reads. The
   worker stays out: nothing can fill the queue (5 and 7 below).
4. ~~37.2~~ **Done as configuration.** `G-ADMIN-SCREENS-ARE-ADMIN-ONLY` was
   fixed for the comment routes at 0.103.0; granting `administer comments` and
   `access administration pages` to the editorial roles was the whole of it.
5. P9 **[kernel: `G-ITEM-API-BYPASSES-ITEM-SERVICE`]**. Re-derived at 0.103.0,
   still true.
6. 37.4, 37.6, P11, P12, D10 **[kernel: `G-MAIL-CANNOT-REACH-A-USER`,
   `G-MAIL-UNAVAILABLE-IN-BACKGROUND`, `G-NO-USER-DIRECTORY`]**. All three
   re-derived at 0.103.0, all three still true. Until then, the on-site list;
   the logged digest is not built either, because nothing can fill its queue.
7. 37.5, D9 **[kernel: `G-QUEUE-NO-CROSS-PLUGIN`]**, or redesign on `plugin-api`.
   Re-derived at 0.103.0, still true. The shared-table workaround is ruled out
   on purpose: the gap is a Ritrovo gate on the kernel's own backlog, so a
   convention-only table would be thrown away when the fix lands.
8. ~~D8: re-count the taps.~~ **Done**: of the brief's 19 intended taps, 7 now
   work and 8 are blocked.

### A8: global and API

1. ~~38.5 read endpoints, D19.~~ **Done** for conferences and topics, as a gather
   alias and two `tap_api` routes; search is the kernel's (`docs/API.md`).
2. P14: badge from `item.language`; switcher links to `/it/item/{id}` and
   `/item/{id}` **[kernel: `G-NO-REQUEST-LANGUAGE`]** for the current language.
3. 38.6: a demo API key **[kernel: `G-API-TOKENS-NEVER-AUTHENTICATE`,
   `G-API-RATE-LIMITS-FIXED`]**. Tokens are issued and never accepted.
4. 38.2, D21 **[kernel: `G-LOCALE-STRINGS-NEVER-LOADED`]**; own the prefixless
   aliases in the moved config.
5. 38.1, 38.4, 38.3, P13, P15 **[kernel: `G-TRANSLATION-NO-WRITE-PATH`,
   `G-ITEM-INSERT-OUTPUT-DISCARDED`]**.
6. P16 **[kernel: `G-FORM-TAPS-UNREACHABLE`]**.
7. 38.5 write endpoints **[kernel: `G-ITEM-API-BYPASSES-ITEM-SERVICE`]**.

### A9: layout and search

1. ~~30.1, 30.2, E4.5, 34.5: search.~~ **Done (A9 part 1)**: the index is built
   before the demo reports ready and holds every Live item; `verify-demo.sh` runs
   the page's own Pagefind query for a conference title and a speaker name.
   **[kernel: `G-SEARCH-PAGE-BLANK-WITHOUT-INDEX`]** still holds for a site
   without an index. D14's stage awareness is part 3.
2. ~~34.1, D5: the conference template.~~ **Done (A9 part 1)**, with the CFP badge
   and topic pills.
3. ~~D18: fix "Call for Papers"; create About; the brief's footer.~~ **Done for
   the footer (A9 part 1)**; user and admin menus, plugin root entries and topic
   breadcrumbs in part 2. **[kernel: `G-PLUGIN-MENU-ENTRIES-WITH-A-PARENT-RENDER-NOWHERE`]**
   for `ritrovo_notify`'s entry.
4. ~~34.6: link or fold `ritrovo.css`; badge styles; decide what `/` renders.~~
   **Done (A9 part 2)**: `/` and `/it/` render `page--front.html` around a
   translated welcome page; tokens, dark mode, phone layout, badges.
5. ~~D13: tile visibility rules in config.~~ **Done (A9 part 2)**, path and role.
6. 39.1, D24: see 10.
7. ~~34.4, D12~~ **Done (A9 part 2) through plugin code** (`tiles.rs`);
   **[kernel: `G-TILE-GATHER-QUERY-RENDERS-NOTHING`]** stays open, and
   **[kernel: `G-NO-VIEWER-ID-IN-TEMPLATES`]** blocks My Subscriptions as a list.
8. 30.3, 30.6, 30.7 **[kernel: `G-SEARCH-NO-ADMIN-OR-ANALYTICS`]**, re-checked in
   part 3 and unchanged; ranking configuration and analytics are out of A9's scope.
9. ~~D14's stage awareness~~ **Done for visitors (A9 part 3)**, asserted in both
   indexes; **[kernel: `G-STAGE-SWITCH-IS-ADMIN-ONLY`]** for the editor half.
10. ~~39.1, D24: the cache walkthrough~~ `docs/CACHING.md`, with a host test and a
   demo check behind every claim; **[kernel: `G-PLUGIN-CANNOT-INVALIDATE-THE-ITEM-CACHE`,
   `G-NO-REQUEST-PROFILER`]**.

### A9b: the AI rows

Moved out of A9: 30.4, 30.5 and 31.1 to 31.11 (31.6 **[kernel:
`G-FORM-TAPS-UNREACHABLE`]**). They need a provider key and no model calls or
provider configuration were made in A9, by the brief. Their status cells are
unchanged and their milestone column reads A9b.

**A9b, first part (2026-11-23):** the providers, defaults, budgets, prices and
AI permissions are configuration in `demo/config`, documented in `docs/AI.md`;
31.1 to 31.3 done, 31.4 and 31.9 part done. Search AI stays off
(`G-SEARCH-AI-CHECKS-NO-PERMISSION-OR-BUDGET`).

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
