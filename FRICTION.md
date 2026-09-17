# Ritrovo: Friction Log

Produced by running Ritrovo's five plugins on the kernel they ship against:
Trovato `v0.102.0`, `rev 20baa121810b5c656b3f80028335770069fab5e0`,
`KERNEL_API_VERSION (0, 102)`, the same revision the workspace pins for both
`trovato-sdk` and the test-only `trovato-kernel`. Ritrovo meets the kernel as a
content application: it imports Items in bulk from a third-party feed, gates them
by editorial stage, decorates their pages, and serves admin screens of its own.

Every item is severity-tagged with `file:line` evidence at that revision and
phrased as a concrete, decidable item for the kernel's own backlog. **NEW**
findings were surfaced by this repository; **RESIDUAL** ones are re-confirmed
from this consumer's side. No-friction findings are last.

The kernel is never edited from here. A gap this repository hits is written down
in this file, and the plugin works around it or does without, saying so where it
does.

The entries from `G-PERM-TAP-NOT-DISPATCHED` onward were added by the 2026-09-17
status audit (`docs/ritrovo/STATUS.md`), which set every promise in the design
brief against the running demo and the pinned kernel. Each one names the stories
it blocks, using the story numbers in `STATUS.md`. Kernel paths are relative to the
Trovato checkout at the revision above; paths under `plugins/`, `scripts/` and
`demo/` are this repository's. Three of them (`G-VIEW-TAP-INPUT-CARRIES-NO-VIEWER`,
`G-MAIL-CANNOT-REACH-A-USER`, `G-USER-API-NO-ADMIN-BYPASS`) were first written in
pull request #6, which merged into a branch that had already merged, so they never
reached `main`; they are re-verified here.

Four entries are load-bearing for the build series, because each stops a whole
prompt rather than a feature: `G-PERM-TAP-NOT-DISPATCHED` (A5, A6, A7),
`G-NO-ITEM-STAGE-TRANSITION` and `G-REVISION-HISTORY-500` (A5), and
`G-TRANSLATION-NO-WRITE-PATH` (A8).

---

## Findings

### G-ITEM-INSERT-OUTPUT-DISCARDED: **[Medium, NEW]** the kernel runs `tap_item_insert` after the insert and throws away whatever it returns, including an error

`ItemService::create` persists the Item, then dispatches `tap_item_insert` and
binds the results to `_results` (`crates/kernel/src/content/item_service.rs:392-401`).
Nothing reads them. The same is true of `tap_item_update` and `tap_item_delete`
(`item_service.rs:652`, `:715`, `:829`).

The plugin documentation describes a different tap. Both references give its
return type as `Result<(), String>` and its purpose as **pre-insert validation**
(`docs/plugin-development.md:195,217`, `docs/plugin-quick-reference.md:60,74`).
It is neither: it runs after the row exists, so it cannot prevent anything, and an
`Err` it returns is dropped with the rest, so it cannot report anything either.
The one tap that can change an Item before it is saved is `tap_item_presave`,
whose `fields` output the kernel does merge (`item_service.rs:365-386`).

**Impact for Ritrovo.** `ritrovo_translate` detects the language of every new
conference in `tap_item_insert` and returns `{"detected_language", "translation_status"}`
for "the kernel to process". The kernel does not, so no conference is ever marked
as needing translation, and the translation queue the design brief describes has no
input. The plugin is correct against the documented contract and inert against the
real one. `plugins/ritrovo_translate/tests/ritrovo_translate_host.rs::the_kernel_records_nothing_the_insert_tap_returns`
pins the current behaviour with a failure message naming this finding, so the day
the kernel changes, a test says what changed.

**Blocks.** 38.3 (`ritrovo_translate` flagging) and 36.5 (`ritrovo_cfp` validation
on insert and update). `tap_item_update` output is discarded the same way
(`item_service.rs:652-658`, and `:1502-1506` on revert).

**Recommendation.** Decide what `tap_item_insert` is for and make the contract say
it. Either (a) it is a post-insert notification: document it as one, and give it
an SDK signature that returns nothing, so no plugin author writes a return value
believing someone reads it; or (b) it can contribute to the saved Item: define the
output shape the kernel merges (as `tap_item_presave` already does for `fields`)
and apply it in the same transaction as the insert. Either way, move "pre-insert
validation" to `tap_item_presave`, the only tap that runs early enough to do it,
and give that tap a way to refuse the save.

### G-PLUGIN-INSTALL-WARNS-ON-AN-OVERLAY: **[Low, NEW]** `trovato plugin install` warns that a module is missing when it is already in place

`cmd_plugin_install` looks for fresh build output to copy in before installing. It
derives the workspace root as two directories above the discovered plugin
directory and looks for `target/wasm32-wasip1/release/<name>.wasm` there
(`crates/kernel/src/plugin/cli.rs:381-414`). That assumption holds for a plugin
inside the Trovato checkout. For a plugin found on an appended `PLUGINS_DIR`
search path, which is how every external plugin is installed, the directory two
levels up is the overlay's parent, which has no `target/`. Install then prints

```text
Warning: WASM not found at '<overlay>/target/wasm32-wasip1/release/<name>.wasm'. Build it first with:
  cargo build --target wasm32-wasip1 -p <name> --release
```

for a module that is sitting next to its manifest, and completes normally.

**Impact.** Cosmetic, but it appears on every `trovato plugin install` of an
external plugin, and it tells the operator to build something already built. The
demo avoids it only because it lets the kernel auto-install at start and then runs
`plugin enable`; this repository's host-in-the-loop suites install through
`cmd_plugin_install` and print it on every run.

**Recommendation.** Only look for build output when the plugin directory has no
`<name>.wasm`, or only when it was discovered on the kernel's own default
`./plugins` path. When a module is already present, say nothing.

### G-PERM-TAP-NOT-DISPATCHED: **[High, RESIDUAL]** a plugin's permissions exist nowhere the kernel looks, so no role can be given them

`tap_perm` is declared in the WIT and marked `(dispatch pending)`
(`crates/wit/kernel.wit:342`), listed in `KNOWN_TAPS`
(`crates/kernel/src/plugin/info_parser.rs:320`), and dispatched nowhere. The kernel
documents the consequence itself (`KNOWN-ISSUES.md:170-189`), and the permission
grid says so on the page: "a plugin's permissions do not appear here and cannot be
granted from this screen" (seen at `/admin/people/permissions` in the demo). The
grid renders `KERNEL_PERMISSIONS` only (`crates/kernel/src/routes/admin_user.rs:786`).

Config import will not grant one either. It accepts a permission only if the
kernel defines it or some role in the database already holds it, and otherwise
fails the whole set with "unknown permission"
(`crates/kernel/src/config_storage/yaml.rs:831-869`). On a fresh install nothing
holds a Ritrovo permission, so nothing can. The one remaining route is raw SQL into
`role_permissions`.

Recorded as a known issue by the kernel; re-confirmed here because it is the single
largest blocker in the brief. In the demo, `role_permissions` holds no Ritrovo
permission for any role.

**Impact for Ritrovo.** `ritrovo_access` declares seven permissions
(`plugins/ritrovo_access/src/lib.rs:12-28`), `ritrovo_importer` five, and
`ritrovo_notify` two. `tap_item_access` grants on them, so it can never grant
anything to anyone but an administrator, who bypasses the tap
(`crates/kernel/src/content/item_service.rs:1025`). Every role in the brief's
"Users, Roles & Permissions" table that differs from the kernel's defaults is
unbuildable.

**Blocks.** 35.2, 36.6, 37.2, 37.3 (`manage own subscriptions`), D16, D3, P17, P18.

**Recommendation.** Dispatch `tap_perm` at boot into a permission registry, and use
that registry in three places: the grid, config import validation, and
`current-user-has-permission`. The kernel's own note calls this additive to the
plugin contract, which it is.

### G-PERM-GRID-SAVE-REVOKES-PLUGIN-GRANTS: **[High, NEW]** saving the permission grid silently removes every plugin permission from every role

The grid handler builds each role's desired set by filtering `KERNEL_PERMISSIONS`
against the submitted checkboxes (`crates/kernel/src/routes/admin_user.rs:821-829`)
and saves it with replace semantics, where a permission not in the set is revoked
(`crates/kernel/src/models/role.rs:197-213`). A plugin permission is never in the
set, so any save revokes it.

This contradicts the workaround `KNOWN-ISSUES.md:183-185` gives for the entry
above ("grant a plugin's permissions once at `/admin/people/permissions` (or by
SQL)"): the grid cannot grant one, and it takes away one granted by SQL the next
time anyone saves the grid for any reason.

**Impact for Ritrovo.** The only way around `G-PERM-TAP-NOT-DISPATCHED` (SQL into
`role_permissions`, then export so import accepts it) is undone by an administrator
changing an unrelated checkbox.

**Blocks.** The same rows as `G-PERM-TAP-NOT-DISPATCHED`, for as long as that
workaround is the only path.

**Recommendation.** Merge rather than replace: revoke only permissions the grid
rendered. Once `tap_perm` is dispatched, render plugin permissions too, grouped by
plugin.

### G-REVISION-HISTORY-500: **[High, NEW]** the revision history page and revert fail for any item that has a revision

`ItemRevision` derives `sqlx::FromRow` with two fields beyond the eight columns the
queries select: `change_summary` and `ai_generated`
(`crates/kernel/src/models/item.rs:71-105`). Both `get_revisions` and
`get_revision` select `id, item_id, author_id, title, status, fields, created, log`
only (`models/item.rs:396-419`). Neither extra field carries `#[sqlx(default)]`, so
decoding a row fails with a missing column.

An empty result decodes, which is why the page works on an item with no revisions.
Observed in the demo: `/item/171ee407-713c-455a-a216-2eceb3e8b0a2/revisions`
answered "No revisions found." for the imported conference. After one edit through
`/item/{id}/edit`, which wrote an `item_revision` row with log "audit edit", the
same page answered **500**, and the server logged `failed to get revisions`
(`crates/kernel/src/routes/item.rs:1243`).

**Impact for Ritrovo.** The brief's five revision scenarios (`docs/ritrovo/overview.md`,
"Revision Workflow") all start from the history page, and revert loads a single
revision through `get_revision`.

**Blocks.** 35.4, D3, D20.

**Recommendation.** Select the two columns, or mark both fields `#[sqlx(default)]`.
Add an integration test that edits an item once and loads its history, since the
current tests evidently pass with zero revisions.

### G-NO-ITEM-STAGE-TRANSITION: **[High, NEW]** nothing moves one item from one stage to another, and the workflow file describing it is read by nothing

There is no route, form field, bulk action or host call that changes an item's
`stage_id`:

- the item forms carry no stage, and creation passes `stage_id: None`
  (`crates/kernel/src/routes/item.rs:957`, `routes/admin_content.rs:293`);
- bulk actions are `publish | unpublish | delete`, which set `status`
  (`routes/admin_content.rs:611-673`);
- `item-api` `save-item` has no stage on update and none on create
  (`crates/kernel/src/host/item.rs:172-185`, `:203-216`);
- `StageService::publish` moves a whole stage at once
  (`crates/kernel/src/stage/mod.rs:1-22`, `:751`, `:869`) and no route calls it.

`variable.workflow.editorial.yml` in the tutorial set describes the transitions and
the permission each needs; the kernel says "nothing in the kernel reads it"
(`crates/kernel/src/routes/admin_stage.rs:14-19`).

**Impact for Ritrovo.** The editorial pipeline (Incoming to Curated to Live) is the
centre of the brief. Content can be put on a stage only by raw SQL, and a person
cannot promote it. This is also why the importer's landing stage cannot simply be
changed: moving imports to Incoming would empty the public site with no way to
publish them.

**Blocks.** 35.3, 36.4 (submissions land in Incoming), 39.2, D3, D25, P3.

**Recommendation.** An item-level transition: a stage selector on the edit form
limited to transitions the user may make, the same as a bulk action, and a host
call. Enforce a workflow definition (the tutorial's variable, or a first-class
entity) with a permission per transition.

### G-DEFAULT-STAGE-IGNORED-ON-CREATE: **[Medium, NEW]** the stage marked default is not where new content lands

`/admin/structure/stages` tells the administrator "Exactly one stage is the default,
which is where new content lands", and stores `is_default`
(`crates/kernel/src/models/stage.rs:107-108`). Item creation binds
`input.stage_id.unwrap_or(LIVE_STAGE_ID)` (`crates/kernel/src/models/item.rs:262`),
the routes pass `None`, and a search of `content/item_service.rs`, `routes/item.rs`,
`routes/admin_content.rs` and `models/item.rs` finds no reader of `is_default`.

**Impact for Ritrovo.** "Submit a Conference" should land in Incoming. Marking
Incoming as default would not do it.

**Blocks.** 36.4, 35.3.

**Recommendation.** Resolve the default stage when a create names none, or remove
the claim and the flag.

### G-TRANSLATION-NO-WRITE-PATH: **[High, NEW]** content translations can be read, and nothing can write one

Translations live in `item_translation (item_id, language, title, fields)`, created
by the `trovato_content_translation` plugin's migration
(`plugins/trovato_content_translation/migrations/001_create_item_translation.sql:4-12`),
and `ItemService` overlays them on read (`crates/kernel/src/content/item_service.rs:467-536`).
Nothing writes them:

- the translation admin routes are GET only (`crates/kernel/src/routes/admin_translation.rs:20-27`),
  and their templates `admin/content-translate-list.html` and
  `admin/content-translate-edit.html` do not exist in the release;
- config import has no translation entity (`crates/kernel/src/config_storage/yaml.rs:51-65`);
- no host call, and no API route;
- the only `INSERT INTO item_translation` in the tree is in kernel tests.

The tutorial's Italian seed therefore stores both languages inside one field as
`{it: {...}, en: {...}}` (`docs/tutorial/config/seed-italian/`), a shape the render
path does not read. In the demo, `/it/item/031e0e55-59d2-4f4c-a2e2-81c50bce5b62`
(Codemotion Roma 2026, `translation_status: translated`) renders with no
description in either language, and `item_translation` holds 0 rows.

**Impact for Ritrovo.** The brief's i18n proof is translated content, and there is
none to show. A plugin could write the table by declaring it in `db_tables`, which
skips cache invalidation, search and revisions.

**Blocks.** 38.1, 38.3, 38.4, D21, P15.

**Recommendation.** A write path with the same guarantees as an item save: a POST
route with the two templates, a config entity so a seed can ship translations, and
a host call so a translation plugin can write one.

### G-ITEM-API-BYPASSES-ITEM-SERVICE: **[High, RESIDUAL]** a plugin's item write fires no taps, checks no access and invalidates no cache

Recorded by netgrasp-trovato as `G-SAVE-ITEM-BYPASSES-SERVICE`, re-confirmed here.
`item-api` calls the `Item` model directly, by design, "to avoid re-entrant tap
dispatch" (`crates/kernel/src/host/item.rs:1-6`). It also cannot set a stage or a
language on create (`host/item.rs:203-216`).

`ritrovo_importer` does not use `item-api` at all; it writes `INSERT INTO item` and
`UPDATE item` through raw SQL (`plugins/ritrovo_importer/src/lib.rs:1228`, `:1293`),
which bypasses the same things plus revisions. In the demo all 5,656 conferences
have `current_revision_id` null. Either way, a plugin cannot create or change
content the way a person does.

**Impact for Ritrovo.** Taps that should see imported conferences never run:
language detection in `ritrovo_translate`, CFP validation in `ritrovo_cfp`, change
notifications in `ritrovo_notify`. Write endpoints in the brief's REST API
(`POST`/`PATCH /api/v1/conferences`) would skip access checks and revisions if
served by a plugin.

**Blocks.** 38.5 (write endpoints), P9, P13, P6, 37.4.

**Recommendation.** As recorded there: route `save-item` through `ItemService` with
a re-entrancy guard, and accept stage and language.

### G-MAIL-CANNOT-REACH-A-USER: **[Medium, NEW]** a plugin has no way to send mail to one of the site's own users

The mail host has one function, `send-to-site-contacts(subject, body, attachments)`,
and the recipient is always the site's configured contact address
(`crates/wit/kernel.wit:212-225`, `crates/kernel/src/host/mail.rs:10-14`). There is
no host call that reads a user's address either: `user-api` exposes
`current-user-id` and `current-user-has-permission` only
(`crates/wit/kernel.wit:55-58`, `crates/kernel/src/host/user.rs`).

The refusal to be a relay is right. It also rules out every message a site sends to
its own members on a plugin's behalf.

**Impact for Ritrovo.** `ritrovo_notify` sends notification and digest email to
subscribers. No plugin can. Until a surface exists the deliverable is on-site
notifications and a logged digest, which is the brief's own demo fallback
("Open Questions", 7).

**Blocks.** 37.4, 37.6, D10, P11, P12.

**Recommendation.** `send-to-user(user_id, subject, body)`, where the kernel resolves
the address, refuses blocked or unverified accounts and honours a per-user opt-out,
so the plugin never sees an address.

### G-MAIL-UNAVAILABLE-IN-BACKGROUND: **[Medium, NEW]** mail fails from `tap_cron` and `tap_queue_worker`, even to the site contact

Background taps run with `RequestServices::for_background`, which sets
`email: None` (`crates/kernel/src/tap/request_state.rs:193-210`); cron and the queue
drain use it (`crates/kernel/src/cron/mod.rs:220`, `:750`, `:1181`). The mail host
returns "not configured" without an email service (`host/mail.rs:115-120`). Only
the request-time state template gets one (`crates/kernel/src/state.rs:746-748`).

**Impact for Ritrovo.** Every mail the brief sends is sent from cron or a queue
worker. Even with `G-MAIL-CANNOT-REACH-A-USER` resolved, none would go.

**Blocks.** 37.4, D10, P11, P12.

**Recommendation.** Attach the email service to background services.

### G-FORM-TAPS-UNREACHABLE: **[Medium, NEW]** `tap_form_alter`, `tap_form_validate` and `tap_form_submit` are declared, dispatched by `FormService`, and `FormService` is never called

`FormService::build` dispatches `tap_form_alter` and `process` dispatches validate
and submit (`crates/kernel/src/form/service.rs:41-161`), and no route calls either;
the kernel says so (`crates/kernel/src/routes/plugin_api.rs:14-15`). The item forms
are built by `FormBuilder` directly (`crates/kernel/src/routes/item.rs:883-886`), and
the profile form is hand written (`crates/kernel/src/routes/auth.rs:1092-1101`).
`form_state_cache` exists and its only writer is the content-type field screen
(`crates/kernel/src/routes/admin_content_type.rs:366-368`), so there is no multi-step
form flow.

**Impact for Ritrovo.** Nothing in the brief's Form API chapter that involves a
plugin can be built: the language selector `ritrovo_translate` adds, a
subscription-preference field on the profile, the multi-step submission form.

**Blocks.** 36.1, 36.4, 36.7, D6, D22, P16.

**Recommendation.** Build the item and profile forms through `FormService`, so the
three taps fire, and give multi-step forms a route that persists state in
`form_state_cache`. Or remove the taps from the contract until then.

### G-PRESAVE-CANNOT-REFUSE: **[Medium, NEW]** no tap can reject a save with a message the editor sees

`tap_item_presave` is the only tap that runs before the row exists. Its output is
merged if it has a `fields` object and ignored otherwise
(`crates/kernel/src/content/item_service.rs:365-390`), and a tap error is logged and
dropped by the dispatcher (`crates/kernel/src/tap/dispatcher.rs:98-105`). Its input
also lacks the item id, stage and author, so create and update look alike.

**Impact for Ritrovo.** `ritrovo_cfp` validates that a CFP does not close after the
conference ends, and should refuse the save with an inline error.

**Blocks.** 36.5, P6.

**Recommendation.** Let presave return errors keyed by field that abort the save and
render on the form, and pass id, stage and author.

### G-QUEUE-NO-CROSS-PLUGIN: **[Medium, NEW]** a queue belongs to the plugin that pushes into it, so two plugins cannot share one

The queue host inserts every job under the caller's own plugin name
(`crates/kernel/src/host/queue.rs:124-127`), and the drain dispatches a job only to
that plugin's `tap_queue_worker` (`crates/kernel/src/cron/mod.rs:224-226`). The
queue name is a free label inside that namespace. `tap_queue_info` is read for one
key, `concurrency`, and only from a JSON array
(`crates/kernel/src/cron/mod.rs:179-193`), so `max_retries` and
`retry_delay_seconds`, which `ritrovo_notify` declares, are ignored.

**Impact for Ritrovo.** The brief's plugin-to-plugin demonstration is `ritrovo_cfp`
writing `cfp_closing_soon` to a queue `ritrovo_notify` drains. That cannot happen:
the job would be handed back to `ritrovo_cfp`. The kernel's alternative is
`plugin-api` invocation of another plugin's declared public functions, which is a
call rather than a queue.

**Blocks.** 37.5, 36.5 (the event half), D9, P6.

**Recommendation.** Either let a plugin declare a queue others may push to, with the
worker resolved by the declaring plugin, or document `plugin-api` as the
plugin-to-plugin mechanism and drop the shared-queue promise from the tutorial.

### G-NO-REQUEST-LANGUAGE: **[Medium, NEW]** a plugin cannot tell which language the page is being rendered in

No host call returns the negotiated language, `ApiRequest` carries none
(`crates/kernel/src/routes/plugin_api.rs:336-346`), request-context keys are the
plugin's own (`crates/kernel/src/host/request_context.rs`), and variables are
namespaced to the plugin, so the site's languages are unreadable too
(`crates/kernel/src/host/variables.rs:4-5`). `tap_item_view` runs before the
translation overlay (`crates/kernel/src/routes/item.rs:332`, overlay at `:354-356`).

**Impact for Ritrovo.** The "View in English / Vedi in italiano" switcher has to
offer the other language, and cannot know which one this is. In the demo the
switcher on an Italian page offers Italian, and its link, `/it/conferences/{id}`,
is 404 (`plugins/ritrovo_translate/src/lib.rs:91-96`). Those two are Ritrovo bugs;
knowing the current language is the kernel gap.

**Blocks.** P14, 38.3.

**Recommendation.** Expose the resolved language to plugins, in `ApiRequest` and to
view taps, and run view taps after the overlay.

### G-LOCALE-STRINGS-NEVER-LOADED: **[Medium, NEW]** UI string translations for any language but the default are never loaded, and nothing imports a `.po` file

`trovato_locale` preloads the default language only
(`crates/kernel/src/state.rs:642-651`), and no other `load_language` call exists.
`LocaleService::import_translations` (`crates/kernel/src/services/locale.rs:73-84`)
has no caller: no route, no CLI command, no config entity. The tutorial ships
`docs/tutorial/config/locale/it.po`, and nothing reads it.

Observed in the demo: `/it/conferenze` renders with `lang="it"` and every string on
it in English ("Upcoming Conferences", "Apply", "Clear").

**Blocks.** 38.2, D21.

**Recommendation.** Load every enabled language's strings, and import `.po` files by
CLI and by config import.

### G-TILE-GATHER-QUERY-RENDERS-NOTHING: **[Medium, NEW]** `gather_query` and `menu` tiles render an empty placeholder that nothing fills

`render_tile` writes `<div class="tile-gather" data-query-id="...">` for a
`gather_query` tile and `<nav class="tile-menu" data-menu="...">` for a `menu` tile,
and nothing else (`crates/kernel/src/services/tile.rs:98-119`). No server code
renders the query into it, and no script in `static/` or `templates/` reads either
class. The tile type match is closed (`tile.rs:76-126`), so a plugin cannot supply
one.

Observed in the demo: the sidebar tiles "Conferences This Month" and "Open Call for
Papers" appear on every page as titled, empty boxes.

**Blocks.** 34.4, D12, and the brief's "Recent Comments" and "My Subscriptions" tiles.

**Recommendation.** Render the query server-side with its display config and a
per-tile item limit, and let plugins register tile types.

### G-SEARCH-PAGE-BLANK-WITHOUT-INDEX: **[Medium, NEW]** `/search` replaces its working server results with nothing when the Pagefind index is absent

The search template always loads `scolta.js` pointing at
`/static/pagefind/pagefind.js` (`templates/search.html:100-121`). The index exists
only when `trovato_search`, disabled by default, has asked cron to build it
(`crates/kernel/src/cron/pagefind.rs:1-8`). The server renders tsvector results, and
the script then takes over the container.

Observed in the demo in Chrome: `/search?q=rust` requests `pagefind.js`, gets 404,
and shows an empty page under the search box, while the HTML the server sent
contains "37 results for" (which is what `scripts/verify-demo.sh` checks, so the
check passes for a page no visitor can use).

**Blocks.** E4.5, 34.5, D14.

**Recommendation.** Leave the server results in place unless the index loads. That is
the progressive enhancement the search design promises.

### G-ADMIN-SCREENS-ARE-ADMIN-ONLY: **[Medium, NEW]** the content list, bulk actions and comment moderation require the administrator flag, whatever a role grants

The content list and bulk actions call `require_admin`
(`crates/kernel/src/routes/admin_content.rs:102`, `:611-618`), as does the comment
moderation list (`crates/kernel/src/routes/admin.rs:524-531`). `require_admin` checks
`is_admin` (`crates/kernel/src/routes/helpers.rs:76-90`). The kernel has a
permission-based check beside it (`helpers.rs:95-100`) that these routes do not use.
The `comment_moderator` role that `trovato_comments` creates holds
`administer comments` and cannot open the moderation queue.

**Impact for Ritrovo.** Editors and publishers are the brief's working roles, and
their screens are the content list, bulk publish and the moderation queue.

**Blocks.** 37.2, 39.2, D25.

**Recommendation.** Gate each admin screen on its own permission through
`require_permission`.

### G-USER-PROFILE-NOT-EXTENSIBLE: **[Medium, NEW]** a profile is a username, an email and a timezone, and nothing can add to it

The profile form posts `name`, `mail`, `timezone` and a password
(`crates/kernel/src/routes/auth.rs:1092-1101`, `templates/user/profile.html:37-59`).
There is no display name, bio, avatar or notification preference, no public profile
route (`/user/{username}` is not registered), and `users.data` JSONB is exposed by no
form. The form does not go through `FormService`, so a plugin cannot alter it
(`G-FORM-TAPS-UNREACHABLE`). A plugin page could hold extra fields in its own table,
but not an avatar, because a plugin route's body is UTF-8 text of at most 256 KiB
(`crates/kernel/src/routes/plugin_api.rs:309-314`) and there is no file host call.

**Blocks.** 36.7, D15.

**Recommendation.** Profile fields defined like item fields, public profile pages,
and the form built through `FormService`.

### G-BATCH-NO-EXECUTOR: **[Medium, NEW]** a batch can be created and polled, and nothing ever runs one

`/api/batch` creates a Redis progress record (`crates/kernel/src/routes/batch.rs:88-114`,
`crates/kernel/src/batch/service.rs`), and nothing outside `batch/` calls
`update_progress` or `complete`. `operation_type` is a free string with no
implementation behind any value.

**Blocks.** 39.2, D25 ("Bulk publish from Curated to Live. Bulk re-import. Progress
tracking via Redis polling").

**Recommendation.** An executor for named operations (stage publish, reindex, alias
regeneration) that reports through the existing progress record.

### G-AJAX-ADMIN-ONLY-NO-CONDITIONAL-FIELDS: **[Low, NEW]** form AJAX is for administrators, and conditional fields do not exist

`POST /system/ajax` requires `require_admin` and runs with
`RequestState::without_services`, so a `tap_form_ajax` handler has no database
(`crates/kernel/src/routes/admin.rs:386-418`). It serves "add another" for
multi-value fields. There is no conditional field mechanism in the form types.
Autocomplete for record references exists (`GET /api/v1/items/autocomplete`,
`crates/kernel/src/routes/api_v1.rs:73`).

**Blocks.** 36.3, D23 (conditional CFP fields, AJAX for non-administrators).

**Recommendation.** Permission-gate the AJAX route, give handlers services, and add a
declarative visible-when to field definitions.

### G-TUTORIAL-CONFIG-SET-DEFECTS: **[Low, NEW]** the tutorial config set that stands in for Ritrovo's model has four defects the running demo shows

The set lives at `docs/tutorial/config/` and ships in the image (`Dockerfile:72`).

1. The main menu's "Call for Papers" link is `/open-cfps`
   (`menu_link.0193a5a0-0004-7000-8000-000000000003.yml:4`); the gather's alias and
   canonical URL is `/cfps`. In the demo, `/open-cfps` is 404 and `/cfps` is 200.
   The footer's `/about` and `/contact` are 404 too: nothing in the set creates them.
2. The Italian aliases are stored with the prefix, `/it/conferenze` with
   `language: it`. The language middleware strips `/it/` before the alias lookup,
   so these rows match only a doubled prefix: in the demo `/it/it/conferenze` is 200.
   `demo/config/url_alias.*.yml` adds prefixless rows so `/it/conferenze` works.
3. `item_type.conference.yml` declares no `field_topics`, which the importer writes,
   the gathers filter on and the seed uses. Saving a conference through the edit form
   drops it: in the demo, after one form save, the item's `field_topics` key was
   gone.
4. `variable.workflow.editorial.yml` describes transitions nothing reads
   (`G-NO-ITEM-STAGE-TRANSITION`), and `locale/it.po` is loaded by nothing
   (`G-LOCALE-STRINGS-NEVER-LOADED`).

**Blocks.** D18 (1), 38.2 (2), 29.1, E4.3 and every conference edit (3).

**Recommendation.** If Ritrovo takes ownership of this model (`STATUS.md`, "Where the
content model lives"), the fixes land in Ritrovo and the kernel keeps a fixture.
Otherwise correct the three files in the kernel.

### G-QUEUE-WORKER-ERROR-IS-SUCCESS: **[Low, NEW]** a worker written with `#[plugin_tap]` that returns an error object has succeeded

A worker's result is success whenever the tap returns output, and failure only when
it traps or returns a negative length (`crates/kernel/src/cron/mod.rs:224-236`). The
SDK sets a negative length only for `#[plugin_tap_result]` returning `Err`
(`crates/plugin-sdk-macros/src/lib.rs:247-291`). A `#[plugin_tap]` worker that
returns `{"status": "error"}` has its job deleted with no retry and no dead letter.

`ritrovo_importer`'s worker is exactly that (`plugins/ritrovo_importer/src/lib.rs:826-836`),
so a malformed batch is logged and gone. That is this repository's bug to fix; the
friction is that nothing in the signature or the docs warns an author.

**Blocks.** P3 (the brief's "Bad data is logged and skipped, not silently dropped").

**Recommendation.** Make the worker signature a `Result` in the SDK so an error value
cannot be returned as success.

### G-NO-USER-DIRECTORY: **[Medium, NEW]** a plugin cannot look up a user it is not currently serving

`user-api` answers only about the current user (`crates/kernel/src/host/user.rs`).
The escape hatch is declaring the kernel's `users` table in `db_tables`, which no
policy refuses (`crates/kernel/src/plugin/db_policy.rs:147-174`) and which hands the
plugin every column, credentials included.

**Blocks.** 37.4, 37.6 (resolving a subscriber's name and preference).

**Recommendation.** A lookup by id returning public profile fields and preferences,
and remove kernel tables from what `db_tables` may name.

### G-VIEW-TAP-INPUT-CARRIES-NO-VIEWER: **[Low, NEW]** the view tap's input is the Item alone, and the documented signature that would carry the viewer does not exist

`ItemService::load_for_view` serializes the `Item` and dispatches `tap_item_view`
with it (`crates/kernel/src/content/item_service.rs:576-583`); the SDK tap takes an
`Item` and returns a `String`. The plugin documentation describes
`tap_item_view(input: ItemViewInput) -> RenderElement`
(`docs/plugin-development.md:215,292`), and there is no `ItemViewInput` in the SDK.

The viewer is reachable anyway: the tap runs with the viewer's request state, so
`current-user-id` and `current-user-has-permission` answer for the person viewing
(`crates/kernel/src/host/user.rs:14-54`), once the plugin declares `user-api`. And
hiding a field is not a view-tap job: a view tap can only append HTML, while
`tap_field_access`, whose input carries the viewer, removes fields before any view
tap runs (`item_service.rs:573`, dispatched at `:1236`).

**Impact for Ritrovo.** None to wait for. Stripping `field_editor_notes` is a
`tap_field_access` implementation in `ritrovo_access`, which is Ritrovo work. The
friction is the documentation, which sent the placeholder at
`plugins/ritrovo_access/src/lib.rs:100-109` waiting for a kernel change nobody needs.

**Blocks.** Nothing; P19 is Ritrovo work.

**Recommendation.** Document the real signature, where a plugin gets the viewer, and
that field visibility belongs to `tap_field_access`. `tap_field_access` is evaluated
only for `view` (`routes/item.rs:369`, `:1382`), so an edit form still shows a field
hidden on view; say that too, or evaluate `edit`.

### G-USER-API-NO-ADMIN-BYPASS: **[Medium, RESIDUAL]** `current-user-has-permission` is a literal membership test, so an administrator fails a plugin's permission check

Recorded by netgrasp-trovato; re-confirmed here. Every kernel route treats an
administrator as holding every permission (for example
`crates/kernel/src/routes/plugin_api.rs:277`). The host call checks the literal
string (`crates/kernel/src/host/user.rs:49`).

**Blocks.** P8, P19 once they check the viewer.

**Recommendation.** Give the host call the routes' `is_admin()` short circuit, or
expose `current-user-is-admin`.

### G-FILE-NO-HOST-API: **[Low, NEW]** a plugin cannot accept or store a file

There is no file interface in the WIT world's imports, and a plugin route's body is
UTF-8 text of at most 256 KiB (`crates/kernel/src/routes/plugin_api.rs:309-314`), so
a multipart upload to a plugin page is refused.

**Blocks.** 36.4 (logo upload in step two of a plugin-built submission form), 36.7
(avatar).

**Recommendation.** Binary request bodies for plugin routes, or a host call that
stores a temporary file and returns its id.

### G-PLUGIN-ROUTE-NO-HEADERS: **[Low, NEW]** a plugin route sees no request headers and sets none

`ApiRequest` has no headers and `ApiResponse` sets status, body, content type, theme
and title only (`crates/plugin-sdk/src/types.rs:784-808`, `:884-915`;
`crates/kernel/src/routes/plugin_api.rs:408-421`). A plugin cannot redirect after a
POST, set `Cache-Control`, or read `Accept-Language`.

**Blocks.** 37.3 (a subscribe toggle that works without JavaScript by POST then
redirect; a themed confirmation page is the workaround).

**Recommendation.** Pass request headers, and allow `Location`, `Cache-Control` and
`Set-Cookie` on responses.

### G-API-RATE-LIMITS-FIXED: **[Low, NEW]** rate limits are compiled defaults, the same for every role

`RateLimitConfig::default()` is used as is (`crates/kernel/src/middleware/rate_limit.rs:99-120`,
`crates/kernel/src/state.rs:791`): 100 API requests a minute per IP or user,
whoever they are. The brief specifies 60 for anonymous and 300 for authenticated
clients. API tokens exist (`/api/tokens`, `crates/kernel/src/routes/api_token.rs:162-163`)
with no page to manage them: `templates/user/profile.html` mentions tokens only in
the account deletion text.

**Blocks.** 38.6.

**Recommendation.** Configurable limits keyed by role, and a token section on the
profile page.

### G-S3-STORAGE-REMOVED: **[Low, NEW]** there is no S3 storage backend

"An S3-compatible backend was dropped for the 1.0 release"
(`crates/kernel/src/file/storage.rs:1-8`).

**Blocks.** 39.3, D17 (the S3 half).

**Recommendation.** None needed now; the kernel's note names the path back. The
tutorial chapter that configures S3 should say it is not available.

### G-NO-REQUEST-PROFILER: **[Low, NEW]** nothing called Gander exists, and the timing middleware that could stand in for it is not layered

Gander appears in design documents only; there is no code by that name. The
`query_profiler` middleware, which emits `Server-Timing`, is exported
(`crates/kernel/src/middleware/query_profiler.rs:1-8`,
`crates/kernel/src/middleware/mod.rs:24`) and never referenced in
`crates/kernel/src/main.rs`, where the router's layers are built.

**Blocks.** D24 (the cache walkthrough "demonstrated via Gander profiling").

**Recommendation.** Layer `track_request_timing` behind a setting, and report cache
hits and misses in it.

### G-REVISION-NO-COMPARE: **[Low, NEW]** revisions store a change summary, and nothing shows a comparison

`item_revision.change_summary` holds added, removed and changed fields
(`crates/kernel/src/models/item.rs:96-101`); no route or template renders a
comparison of two revisions.

**Blocks.** 35.4 (scenario 5), D20 ("compare revisions").

**Recommendation.** A compare view on the history page, after `G-REVISION-HISTORY-500`.

### G-SEARCH-NO-ADMIN-OR-ANALYTICS: **[Low, NEW]** search ranking is fixed in a template, and nothing records queries

The ranking settings are a JSON block in `templates/search.html:100-121`, not
configuration. There is no search settings screen and no record of queries, top
queries or expansion hit rates (a search of `crates/kernel/src` and `templates/` for
analytics or sentiment finds nothing).

**Blocks.** 30.3 (configurable ranking), 30.6 (sentiment), 30.7.

**Recommendation.** Ranking in site configuration with an admin form, and a query
log behind a privacy setting.

---

## No-friction findings (surfaces that just worked)

- **Testing an unmodified plugin against real HTTP without the network.** The
  `http` host's SSRF fence refuses loopback and private addresses by URL and by
  resolution (`crates/kernel/src/host/http.rs:360-435`), and it cannot be switched
  off, which is right. It never needed to be: `RequestServices::for_background`
  takes the `reqwest::Client` a caller chooses, so a test's client resolves the
  plugin's real public host name to a local TLS fixture server. `ritrovo_importer`
  fetches its real confs.tech URL in `tap_install` and cannot tell. See
  `tests/host/mod.rs`, `FixtureServer`.
- **The plugin queue drain as a library call.** `CronService::drain_plugin_queues`
  runs the real claim, concurrency, retry and dead-letter bookkeeping with no
  Redis connection and no cron route, so the importer's install-then-drain path
  is testable end to end in a few seconds.
- **The kernel's own install path, from an external search path.** `cmd_plugin_install`
  and `cmd_plugin_enable` check `api_version`, dependencies and migrations against
  an overlay exactly as they do for an in-tree plugin, the warning above aside.
- **Config import as a library call.** `config_storage::yaml::import_config` with
  a `DirectConfigStorage` imports a subset of the tutorial config (taxonomy,
  content type, stages) into a test database with the same validation the CLI
  applies.
