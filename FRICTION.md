# Ritrovo: Friction Log

Produced by running Ritrovo's six plugins on the kernel they ship against. Since
A11 that is Trovato `main` at `rev 81290cc862b2c8302bc0a99d270f2571405c51ef`
(workspace version 0.107.0, `KERNEL_API_VERSION (0, 107)`, image
`ghcr.io/jeremyandrews/trovato:nightly-81290cc`), the same revision the workspace
pins for both `trovato-sdk` and the test-only `trovato-kernel`. Entries written
before A11 cite Trovato `v0.103.0`, `rev 496b8f113be66102830723c1e2a968cad2ec1e0d`,
and their `file:line` evidence is at that revision unless the entry says otherwise. Ritrovo meets the kernel as a
content application: it imports Items in bulk from a third-party feed, gates them
by editorial stage, decorates their pages, and serves admin screens of its own.

**Re-derived against 0.103.0, 2026-09-22 (A7).** Every entry this repository's
A7 work depends on was checked against the new revision rather than carried
forward. Four were fixed and are marked **FIXED at 0.103.0** in place rather
than deleted, because what a release closed is worth as much to the next reader
as what it left open: `G-PERM-TAP-NOT-DISPATCHED`,
`G-PERM-GRID-SAVE-REVOKES-PLUGIN-GRANTS`, `G-USER-API-NO-ADMIN-BYPASS` and
`G-TRANSLATION-NO-WRITE-PATH`. One was narrowed:
`G-ADMIN-SCREENS-ARE-ADMIN-ONLY`. The rest of the entries A7 touched were
re-confirmed as written. Three new entries were found by doing the work:
`G-ITEM-TEMPLATE-HAS-NO-VIEWER`, `G-PLUGIN-PERMISSION-GRID-READS-BOOT-MEMORY`
and `G-QUEUE-WITHOUT-A-WORKER-IS-SILENT`.

**A8, 2026-11-14, same revision.** The translation work re-checked every entry it leans
on against the running demo. `G-TRANSLATION-NO-WRITE-PATH` is **narrowed**, not fixed:
the screen and the config entity work, and no host call does.
`G-ITEM-INSERT-OUTPUT-DISCARDED`, `G-NO-REQUEST-LANGUAGE` and
`G-LOCALE-STRINGS-NEVER-LOADED` are re-confirmed as open, with what Ritrovo now does
around each. Eight new entries were found by doing the work:
`G-TRANSLATION-OVERLAY-SKIPS-THE-DEFAULT-LANGUAGE`, `G-ITEM-TEMPLATE-RENDERED-BEFORE-ITS-TRANSLATIONS`,
`G-TOPICS-PAGE-IGNORES-LANGUAGE`,
`G-TRANSLATE-SCREEN-TEXT-FIELDS-ONLY`, `G-ALIAS-FALLBACK-HARDCODES-EN`,
`G-THEMED-PLUGIN-PAGE-UNDER-ADMIN-RENDERS-UNWRAPPED`, `G-GATHER-TRANSLATION-MERGE-IS-BACKWARDS`
and `G-RENDER-BLOCKS-PASSES-STRINGS-UNSANITIZED`, the last a stored XSS through the
submission form that is closed in Ritrovo's templates and open in the kernel.

**A8 part 3, the API, same revision.** Five more entries, found by building
`docs/API.md` against the demo: `G-ADHOC-GATHER-BYPASSES-FIELD-ACCESS` (High: any
anonymous visitor can read the fields `tap_field_access` hides),
`G-API-TOKENS-NEVER-AUTHENTICATE` (High: no token from `/api/tokens` is ever accepted),
`G-OPENAPI-IS-A-STATIC-LIST`, `G-GATHER-EXPLICIT-COLUMNS-ARE-TEXT` and
`G-UNKNOWN-GATHER-IS-A-500`.

**A9 part 1, search and the pages a visitor meets, same revision.** Six more
entries, found by searching the demo in a browser and reading what Pagefind
returned: `G-PAGEFIND-INDEX-OMITS-TITLES` (Medium: no conference or speaker was
findable by its name), `G-PAGEFIND-RESULT-URL-IGNORES-ALIAS-LANGUAGE`,
`G-PAGEFIND-IGNORES-FIELD-WEIGHTS`, `G-SEARCH-CANNOT-INDEX-CATEGORY-LABELS`,
`G-SEARCH-TRIGGER-INDEXES-BLOCKS-JSON` and `G-MARKDOWN-FILTER-HAS-NO-TABLES`.
`G-SEARCH-PAGE-BLANK-WITHOUT-INDEX` is re-verified and still open.

**A9 part 2, layout, tiles and menus, same revision.** Seven more, found building
the front page, the tiles and the four menus: `G-PLUGIN-MENU-ENTRIES-WITH-A-PARENT-RENDER-NOWHERE`
(Medium), `G-NO-VIEWER-ID-IN-TEMPLATES` (Medium), `G-ONLY-MAIN-AND-FOOTER-MENUS-RENDER`,
`G-TILE-ROLES-ARE-SYNTHETIC`, `G-TILES-HAVE-NO-LANGUAGE`,
`G-BREADCRUMBS-IGNORE-THE-TOPIC-TREE`, `G-PLUGIN-RESPONSE-CANNOT-REDIRECT` and
`G-NO-ACCESSIBILITY-CHECK`. `G-TILE-GATHER-QUERY-RENDERS-NOTHING` stays open, with
the workaround Ritrovo now uses recorded under it.

**A9 part 3, search by stage and caching, same revision.** Two more:
`G-STAGE-SWITCH-IS-ADMIN-ONLY` (Medium: an editor's search answers from Live only)
and `G-CACHE-METRICS-NEVER-COUNT`. `G-NO-REQUEST-PROFILER` is narrowed, because
`Server-Timing` is layered at 0.103.0, and stays open.
`G-PLUGIN-CANNOT-INVALIDATE-THE-ITEM-CACHE` is re-verified with a host test and
stays open.

**A11, 2026-11-23, Trovato `main` at `81290cc`.** The pin moved from 0.103.0 to a
`main` revision, and every entry Ritrovo's code or checks lean on was re-checked
by test or by `verify-demo.sh`. Nine are **FIXED at main 81290cc**, each with the
test or demo check that shows it: `G-REVISION-HISTORY-500`,
`G-REVISION-NO-COMPARE`, `G-FORM-SAVE-DROPS-EVERY-FIELD-THE-FORM-DID-NOT-RENDER`,
`G-CARDINALITY-IS-INERT`, `G-ITEM-EDIT-FORM-LOADS-NO-ASSETS`,
`G-API-TOKENS-NEVER-AUTHENTICATE`, `G-ADHOC-GATHER-BYPASSES-FIELD-ACCESS`,
`G-OPENAPI-IS-A-STATIC-LIST` and `G-CACHE-METRICS-NEVER-COUNT`. Two are
**narrowed**: `G-FRONTEND-ITEM-FORM-STORES-EVERY-FIELD-AS-A-STRING` (Booleans are
typed; nothing else was checked) and `G-ITEM-API-BYPASSES-ITEM-SERVICE` (a request
path write goes through `ItemService`; a background one cannot be made at all, the
first entry below). Five are new: `G-BACKGROUND-ITEM-API-HAS-NO-ITEM-SERVICE`
(High), `G-SQL-GUARD-REFUSES-INDEX-EXPRESSION-CONFLICT-TARGET` (Medium),
`G-SQL-GUARD-REFUSES-WITH-UPDATE` (Low), `G-FRONT-PAGE-TILES-MATCH-THE-CONFIGURED-PATH`
(Low) and `G-GATHER-ROWS-HAVE-NO-FILE-URLS` (Low). Entries the kernel's changelog
says are addressed, and which Ritrovo has not built on, are listed in
`docs/ritrovo/STATUS.md` under "A11: the kernel-blocked rows rechecked" as
buildable and not built; they are not marked fixed here, because nothing in this
repository has exercised them.

**A9b, 2026-11-23, same revision.** The AI rows. Six new entries, found switching
the kernel's AI features on through configuration: `G-SEARCH-AI-CHECKS-NO-PERMISSION-OR-BUDGET`
(High: the AI search endpoints spend the site's tokens for any visitor, unmetered,
so Ritrovo keeps them off), `G-AI-RATE-LIMITS-ARE-NOT-PER-ROLE`,
`G-AI-REQUESTS-SENT-WITHOUT-A-KEY`, `G-AI-CHAT-ERROR-LOGS-THE-PROVIDER-BODY-UNREDACTED`,
`G-AI-ADMIN-HAS-NO-FEATURES-RULES-OR-RATE-LIMIT-SCREENS` and
`G-CONFIG-IMPORT-LEAVES-SEARCH-SETTINGS-CACHED` (Medium: an imported search setting
waits out a five-minute cache).

**A9b, second part (search, enrichment and vectors).** Four more:
`G-AI-FIELD-RULES-NEVER-LOAD` (High: `trovato_ai` queries a column `site_config`
does not have, so no field rule runs), `G-SEARCH-AI-READS-ONLY-THE-FIRST-CONTENT-BLOCK`,
`G-EMBEDDINGS-BYPASS-BUDGETS-AND-THE-USAGE-LOG` and `G-EMBEDDINGS-HAVE-NO-TEST-PROVIDER`.

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


**Re-checked at 0.103.0 (A8): still open, and Ritrovo no longer depends on it.**
`ritrovo_translate` 2.0.0 dropped its `tap_item_insert`, whose finding went nowhere, and
detects language in `tap_item_presave` instead, whose `fields` output the kernel merges
into the item before it is written (`crates/kernel/src/content/item_service.rs:353-387`).
The host suite asserts the stored `field_language`, which is the thing the old test could
only show was missing.

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

### G-PERM-TAP-NOT-DISPATCHED: **[FIXED at 0.103.0]** a plugin's permissions exist nowhere the kernel looks, so no role can be given them

**Fixed at Trovato 0.103.0, verified 2026-09-22.** The kernel dispatches
`tap_perm` at boot (`crates/kernel/src/state.rs:555-573`), parses the results
into a `PluginPermissionRegistry`, and writes them to a new `plugin_permission`
table (`crates/kernel/src/plugin/permission_registry.rs`, migration
`20260919000001_create_plugin_permission.sql`). All three consumers now read a
plugin's permissions: the grid renders them beside the kernel's with a column
naming the declaring plugin (`routes/admin_user.rs:929-948`), config-import
validation accepts them (`config_storage/yaml.rs:826-891`, which checks
`KERNEL_PERMISSIONS`, then the table, then what any role already holds), and
`current-user-has-permission` answers for them.

Verified on the running demo rather than only read: `plugin_permission` holds 38
rows, `demo/config` grants `post comments`, `edit own comments` and
`manage own subscriptions` to roles that could hold none of them a release
earlier, and `config import --dry-run` accepts the set.

**What it unblocked here.** `ritrovo_notify` stopped being a placeholder: its
`manage own subscriptions` is a real gate on three real routes rather than a
declaration into a void. Rows 37.1 and 37.3 closed. The `edit any content`
substitution the editor and publisher role files describe is no longer forced,
though it is left in place for A5 to remove rather than changed here.

The entry as originally written follows, because the recommendation it makes is
the one the kernel took.

---


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

### G-PERM-GRID-SAVE-REVOKES-PLUGIN-GRANTS: **[FIXED at 0.103.0]** saving the permission grid silently removes every plugin permission from every role

**Fixed at Trovato 0.103.0, verified 2026-09-22.** The save is now scoped
instead of replacing each role's whole set. The form states which permissions it
rendered, by submitting each name as a hidden `permname_{i}` value, and
`Role::set_permissions_within` (`crates/kernel/src/models/role.rs:242-266`)
confines removals to that set: a permission the screen never rendered keeps
whatever it had. `Role::set_permissions`, which still replaces wholesale, is
left for `config import`, where the file is the complete statement of what a
role holds.

The fix is two-layered and worth reading as one: dispatching `tap_perm` means
the grid *renders* a plugin's permissions, and scoping the save means a grant
the grid still cannot see — from a disabled plugin, from SQL, from a migration —
survives anyway.

The entry as originally written follows.

---


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

### G-REVISION-HISTORY-500: **[FIXED at main 81290cc]** the revision history page and revert fail for any item that has a revision

**Fixed at Trovato `main` 81290cc, verified 2026-11-23 (A11).** Kernel `4c75594` (B70) rebuilt the revision routes on `ItemService::load_revisions`. `an_editor_lists_previews_compares_and_reverts_revisions` (`plugins/ritrovo_access/tests/ritrovo_access_host.rs`) saves a conference through the kernel form as an editor and gets 200 from the history page, with the log message, and a working revert that puts the old value back. On the demo, `/item/{id}/revisions` was 200 for an Incoming conference after an edit. What follows is the entry as written at 0.103.0.

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

**Widened, 2026-09-18 (A5).** Three things were understated above.

*It is every item, not every edited item.* `Item::create` inserts an initial
revision of its own (`crates/kernel/src/models/item.rs:271-288`, log
`"Initial revision"`), so anything created through the kernel has a revision from
birth and its history page has never worked. The imported conferences are the only
ones that answer 200, and only because the importer writes raw SQL and creates no
revision. Reproduced on the running demo: the same conference answered 200 with
zero revisions and **500** with one.

*Revert is unreachable twice over.* `revert_to_revision` calls `get_revision` as
its first statement (`models/item.rs:430-432`), so it 500s on the same decode, and
the only affordance that reaches it is a button inside the `{% for %}` loop of
`templates/admin/revisions.html`, on the page that 500s before it renders. The
access check runs first, so an unauthorized caller still gets a correct 403.

*The root-cause fix is the SELECT list, not `#[sqlx(default)]`.* Both columns exist
(`migrations/20260330000004_add_revision_change_summary.sql:6`,
`20260330000005_add_revision_ai_generated.sql:5`). Defaulting the fields would
compile and would return NULL for data the table holds. Neither column is written
by anything, so the doc comment's "auto-generated by diffing previous revision" is
describing code that does not exist.

*Why no test caught it:* every `ItemRevision` test is a struct literal with no
database (`crates/kernel/tests/item_test.rs:713-748`), and no test requests either
route.

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

**Worked around, 2026-09-18 (A5). The finding stands.** Ritrovo now moves items
between stages, so the editorial pipeline in the brief is real in the demo. It does
it the only way available: `ritrovo_access` serves its own screen at
`/admin/content/editorial` and writes `item.stage_id` through the raw-SQL host
call, with the workflow table in the plugin
(`plugins/ritrovo_access/src/lib.rs`, `TRANSITIONS`). The importer lands
conferences on Incoming for the same reason, by binding the column in its own
INSERT.

What that costs, and why this is still a kernel gap rather than a solved problem:

- the write bypasses `ItemService` entirely, so it creates no revision, fires no
  `tap_item_update`, reindexes nothing and invalidates no cache
  (`G-ITEM-API-BYPASSES-ITEM-SERVICE`, `G-PLUGIN-CANNOT-INVALIDATE-THE-ITEM-CACHE`);
- it requires `raw_sql = true`, because the structured `db.update` call emits
  `SET col = $n` with no cast and cannot write a `uuid` column from a JSON string,
  so the plugin holds the documented escape hatch out of its own table allowlist
  in order to write one column;
- the workflow and its permissions live in a plugin, so two sites wanting the same
  pipeline write it twice;
- a conference submitted through the kernel's own form still lands on Live, because
  that path runs through `Item::create` with `stage_id: None`, which no plugin can
  reach.

Any site wanting the brief's pipeline has to rebuild this. That is the argument for
the recommendation above, not against it.

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

### G-TRANSLATION-NO-WRITE-PATH: **[NARROWED at 0.103.0; Medium]** a person and a config set can write a translation, and a plugin still cannot

**What 0.103.0 fixed, verified in the A8 demo (2026-11-14).** Three of the four
missing writers now exist and work:

- the translate screen is a real form: `GET`/`POST /admin/content/{id}/translate/{lang}`
  and a delete sibling (`crates/kernel/src/routes/admin_translation.rs:26-37`), gated on
  `translate content` and CSRF, rendering `templates/admin/content-translate-edit.html`,
  which ships in the image. In the demo editor_alice opened it from Ritrovo's queue,
  posted a title the way a browser does (every input, blanks included), got a `303`,
  and `/it/item/{id}` rendered the new title with `lang="it"`. Blank inputs are dropped
  rather than stored as empty overrides. `scripts/verify-demo.sh` does this on every run,
  then deletes the row through the delete route.
- a config entity, `item_translation.<item uuid>.<lang>.yml`, imported after `item`
  (`crates/kernel/src/config_storage/yaml.rs:65`, `direct.rs:833-878`). Ritrovo's Italian
  seed now ships fifteen of them, and the demo's `item_translation` holds 15 rows on a
  fresh start.
- `ItemService::save_translation` (`crates/kernel/src/content/item_service.rs:507-532`),
  which the screen calls.

**What is left.** No host call reaches `save_translation`: `crates/plugin-sdk/src/host.rs`
has no translation function and `crates/kernel/src/host/` registers none. So a plugin
can read translations (through its own SQL) and cannot write one. `ritrovo_translate`'s
model-assisted path therefore stores a model's draft in its own
`ritrovo_translation_draft` table, shows it on the queue page, and the editor saves it
through the kernel screen by hand. Writing `item_translation` through `raw_sql` would
work and was not done: it skips the screen's checks, and a plugin-owned write to the
kernel's table is exactly what this entry exists to avoid.

**Blocks.** P15's last step (a reviewed draft saved without an editor retyping it).
38.1, 38.3 and 38.4 are no longer blocked by this entry.

**Recommendation.** A `save-translation(item_id, language, title, fields)` host call
over `ItemService::save_translation`, with the language checked against the site's
languages as the route checks it.

### G-ITEM-API-BYPASSES-ITEM-SERVICE: **[NARROWED at main 81290cc; High]** a plugin's item write fires no taps, checks no access and invalidates no cache

**Re-checked at Trovato `main` 81290cc, 2026-11-23 (A11).** Kernel `db45018` (B68.2) sends a plugin's item writes through `ItemService` on a request. In the background, where every one of Ritrovo's item writes happens, the call fails instead (`G-BACKGROUND-ITEM-API-HAS-NO-ITEM-SERVICE`), so the importer still writes by raw SQL and fires no taps. The cache half is better: a plugin's raw write now bumps the item cache generation (`crates/kernel/src/host/db.rs:306`).

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


**Re-checked at 0.103.0 (A8): still open.** The Ritrovo half is fixed: the view tap badges
from `field_language` (then the item's `language`), and links `/item/{id}` and
`/it/item/{id}`, both of which answer 200. It emits **both** links, each marked with its
`hreflang`, because it still cannot know which one the page is. That output does not
reach the conference page anyway (`G-RENDER-CHILDREN-MIXES-FIELD-DUMP-AND-PLUGIN-OUTPUT`),
so the page's switcher is drawn by `item--conference.html`, which is given
`active_language` (`crates/kernel/src/routes/item.rs:682`) and shows whichever of the two
addresses is not the page's own. `verify-demo.sh` follows it both ways.

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


**Re-checked at 0.103.0 (A8): still open.** `crates/kernel/src/state.rs:738-746` calls
`load_language` for the default language only, inside `if enabled_set.contains("trovato_locale")`;
`LocaleService::import_translations` (`crates/kernel/src/services/locale.rs:74`) still has
no caller. The `trans` filter is registered only when the locale service exists
(`crates/kernel/src/theme/engine.rs`), and no Ritrovo template calls it, so A8's switcher
prints language codes and no words. `demo/config/locale/it.po` carries the strings A8 adds
for the day this closes.

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

**A9 workaround, 2026-11-14. Still open.** Ritrovo's tiles render, but not through
this type. Every one of the brief's data tiles (CFPs Closing Soon, Conferences
This Month, the Topic Cloud, Recent Comments, and the front page's upcoming
conferences) is a `custom_html` tile, the one type the kernel renders server-side,
whose body `ritrovo_importer`'s cron rewrites from SQL on every call
(`plugins/ritrovo_importer/src/tiles.rs`), escaping every value. Placement, weight
and visibility are still the kernel's, in `demo/config/tile.*.yml`. The costs: the
content is stale by up to one cron period, it is written in the default language
only (`G-TILES-HAVE-NO-LANGUAGE`), it can never be per viewer, and a `custom_html`
body is read from the database on every request with no cache. A `gather_query`
tile that rendered would remove all four.

### G-SEARCH-PAGE-BLANK-WITHOUT-INDEX: **[Medium, NEW]** `/search` replaces its working server results with nothing when the Pagefind index is absent

The search template always loads `scolta.js` pointing at
`/static/pagefind/pagefind.js` (`templates/search.html:100-121`). The index exists
only when `trovato_search`, disabled by default, has asked cron to build it
(`crates/kernel/src/cron/pagefind.rs:1-8`). The server renders tsvector results, and
the script then takes over the container.

Observed in the demo in Chrome: `/search?q=rust` requests `pagefind.js`, gets 404,
and shows an empty page under the search box, while the HTML the server sent
contains "37 results for".

The demo now enables `trovato_search` in `scripts/demo-bootstrap.sh`, so an index
exists and this page works there. That is a demo fix, not a kernel fix: the kernel
still throws working server results away whenever the index is missing, which is
every site that has not enabled the plugin, every site between install and the
first cron run, and any site whose index build failed. `scripts/verify-demo.sh` now
checks that `pagefind.js` answers 200 and that the index contains a known
conference, because the previous check grepped the server-rendered HTML and so
passed for a page no visitor could use.

**Re-verified at 0.103.0, 2026-11-14 (A9).** Still true. On the demo, with the
index directory moved aside so `/static/pagefind/pagefind.js` answered 404,
`/search?q=rust` in Chrome hid the server's results and sat on "Searching...":
`search-init.js` hides `#search-fallback` whenever `Scolta` is defined, before
anything is known about the index (`static/js/search-init.js:20-25`), and
`templates/search.html:125-126` always loads both scripts. The demo now builds the
index before `scripts/serve-demo.sh` reports ready and verifies it holds every
Live item, so the demo is not exposed; the kernel still is.

**Blocks.** Nothing in the demo now. Every site without a built index.

**Recommendation.** Leave the server results in place unless the index loads. That is
the progressive enhancement the search design promises.

### G-RENDER-CHILDREN-MIXES-FIELD-DUMP-AND-PLUGIN-OUTPUT: **[Medium, NEW]** a template cannot render its own fields and keep plugin render output, because the kernel hands it both in one string

The item route builds `children_html` in two passes and inserts the result under a
single name. First it walks `item.fields` and appends a generic
`<div class="field field-{name}"><strong class="field__label">{label}</strong>: {value}</div>`
for every scalar (`crates/kernel/src/routes/item.rs:536-575`). Then it appends the
output of every plugin's `tap_item_view` (`:578-581`, collected by
`content/item_service.rs:587-591`). The concatenation is inserted as `children`
(`routes/item.rs:684`), and nothing else in the context carries the plugin half.

So a template has two options and no third. Render `children`, and every scalar
field prints a second time under whatever the template already laid out, internal
fields included: on the demo's conference pages that was `source id`,
`editor notes` and `editor notes format` in a stack of `label: value` lines.
Or drop `children`, and lose every plugin's contribution to the page with it.
Ritrovo's conference template took the second option
(`docs/tutorial/templates/elements/item--conference.html`), which costs it the
`ritrovo_cfp` days-left badge, the `ritrovo_translate` language switcher and the
kernel `trovato_seo` plugin's `application/ld+json` block.

Picking the plugin half back out of the string by pattern is possible and wrong:
the field divs come first and the plugin output last, but a field value rendered
through the `filtered_html` pipeline can itself contain a closing `</div>`, so any
split is one description away from cutting the page in the wrong place.

**Blocks.** 34.1, D5; and it is why P5 and P14 no longer render on a conference
detail page.

**Partly worked around (A9).** The CFP badge is drawn again by the conference
template itself, from the same rule as `ritrovo_cfp`'s `render_badge` (end of
the closing day, whole days, the same three bands), with Tera's `now()`.
`verify-demo.sh` asserts it on a Live conference with an open call for papers. The
language switcher was already the template's (A8). The SEO plugin's JSON-LD is
still lost.

**Recommendation.** Insert the plugin outputs into the template context under their
own name as well, for example `plugin_output`, leaving `children` exactly as it is
for the templates that want the generic rendering. One line of context, no
behaviour change for anything that exists.

### G-FRONTEND-ITEM-FORM-STORES-EVERY-FIELD-AS-A-STRING: **[NARROWED at main 81290cc; Low]** the non-admin item form writes every value as a JSON string and runs none of the typed processing the admin form runs

**Re-checked at Trovato `main` 81290cc, 2026-11-23 (A11).** Kernel `5fcc2d8` (B71.2) builds a typed submission. An unchecked Boolean is stored as JSON `false` (`saving_the_edit_form_keeps_every_field_including_the_ones_it_did_not_render`). Integer, Float and Date kinds were not checked from here, so the entry stays open for them.

`ItemSubmission::from_form` inserts every posted value as
`serde_json::Value::String` with no reference to the field's declared type
(`crates/kernel/src/routes/item.rs:193-200`), and `routes/item.rs` never calls
`process_blocks_fields`, `process_compound_fields` or `validate_required_fields` —
all three are reached only from `routes/admin_content.rs` (`:248`, `:449`, `:254`,
`:455`). The admin form parses a `Blocks` field's hidden input into a real JSON
array and re-inserts it (`content/compound.rs:530-560`); `/item/{id}/edit` does not.

Observed in the demo: one edit of "Gerrit User Summit" through `/item/{id}/edit`
left `field_description`, declared `Blocks`, holding the JSON **string** `"[]"`
(`jsonb_typeof` = `string`), which `render_blocks` then printed on the page as a
literal `[]`. The same mechanism is why templates have to compare booleans against
the string `"1"`.

**Blocks.** 34.1 (the conference template now guards against it); 29.2, in that the
form a non-administrator would use saves a shape the admin form would not.

**Recommendation.** Have the two form paths share one typed submission builder, so
the field's declared type decides the stored JSON in both.

### G-ADMIN-SCREENS-ARE-ADMIN-ONLY: **[NARROWED at 0.103.0]** the content list, bulk actions and comment moderation require the administrator flag, whatever a role grants

**Narrowed, not closed, at Trovato 0.103.0, verified 2026-09-22.** Comment
moderation is delegable; the content list is not.

All six comment routes now call
`require_permission(&state, &session, "administer comments")` rather than
`require_admin` (`crates/kernel/src/routes/admin.rs:559`, `:640`, `:691`,
`:777`, `:888`, `:930`). `/admin` itself moved off `administer site` onto a new,
deliberately weak `access administration pages`, so a delegated role can reach
the dashboard that links to the screen it was given (`routes/admin.rs:107-111`).
Both are needed together: without the second, an editor reaches the moderation
queue only by typing the address.

**Closed for Ritrovo's story 37.2 as configuration.** `demo/config` grants
`administer comments` and `access administration pages` to the editor and
publisher roles, and `scripts/verify-demo.sh` asserts that editor_alice opens
`/admin/content/comments` and viewer_carol does not. No screen was built.

**What still stands.** `/admin/content`, its bulk actions and the delete route
still call `require_admin` (`routes/admin_content.rs:102`, `:611-618`), so the
brief's bulk publish is still the administrator's alone and story 39.2 keeps
this entry. And the admin layout does not filter its sidebar to what the viewer
may open, which the kernel records as its own BL-117, so a delegated editor is
shown links that answer 403. That is a worse first impression than the 403 it
replaced, and it is the kernel's to fix: filtering needs
`render_admin_template` to know the viewer, across its 80 call sites.

The entry as originally written follows.

---


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
   A8 deleted the three doubled rows from Ritrovo's set, which now stores each Italian
   alias once, prefixless. They were worse than dead: `available_translations` builds
   `/{language}{canonical_path}` (`crates/kernel/src/routes/helpers.rs:1055`), so a
   prefixed alias picked as a canonical path would have produced `/it/it/...`. The
   kernel image's own copy still has them.
3. `item_type.conference.yml` declares no `field_topics`, which the importer writes,
   the gathers filter on and the seed uses. Saving a conference through the edit form
   drops it: in the demo, after one form save, the item's `field_topics` key was
   gone.
4. `variable.workflow.editorial.yml` describes transitions nothing reads
   (`G-NO-ITEM-STAGE-TRANSITION`), and `locale/it.po` is loaded by nothing
   (`G-LOCALE-STRINGS-NEVER-LOADED`).

**A9.** Item 1 is closed in Ritrovo: the link points at `/cfps`, and `/about`,
`/contact`, `/api-docs`, `/data-sources` and `/privacy` are kernel `page` items
seeded from `demo/config/seed-content` (the API page built from `docs/API.md` at
bootstrap). `verify-demo.sh` follows every main and footer menu link as rendered.

**Blocks.** 38.2 (2), 29.1, E4.3 and every conference edit (3).

**Recommendation.** If Ritrovo takes ownership of this model (`STATUS.md`, "Where the
content model lives"), the fixes land in Ritrovo and the kernel keeps a fixture.
Otherwise correct the three files in the kernel.

### G-NO-CATEGORY-REFERENCE-FIELD-KIND: **[High, NEW]** a taxonomy reference cannot be declared as a field, so the one field the brief's model turns on is invisible to every form

`FieldType` has twelve variants and none of them is taxonomic: `Text`, `TextLong`,
`Integer`, `Float`, `Boolean`, `RecordReference(String)`, `File`, `Date`, `Email`,
`Compound`, `Blocks`, `PageBuilder` (`crates/plugin-sdk/src/types.rs:180-207`).
`RecordReference` targets an item type, not a category.

Everything else about topics works. The value is a JSONB array of tag uuids, the
gather operators `has_tag`, `has_any_tag`, `has_all_tags` and
`has_tag_or_descendants` read it with `@>` containment and a recursive CTE
(`crates/kernel/src/gather/query_builder.rs:826-894`, `:920-942`), `/topics/{slug}`
resolves a slug to a tag on the gather route (`routes/gather_routes.rs:142-149`),
and an exposed filter can offer a `CategorySelect` widget
(`gather/types.rs:125-131`). Only the *declaration* is missing, and with it every
form: neither content form has a tag widget
(`crates/kernel/src/content/form.rs:221-495`, `templates/admin/content-form.html:56-127`)
and the only autocomplete endpoint is for RecordReference items
(`routes/api_v1.rs:73`).

So Ritrovo's `conference` type deliberately does **not** declare `field_topics`.
Declaring it as `Text`, the nearest kind, would put a single-line text widget over
a JSON array, and saving that form would replace the array with a string, taking
the conference out of every topic listing.

**Blocks.** 29.1, E4.3, D1, D2; and the brief's claim that declaring the field is
what stops a save dropping it, which is not true on this kernel (see the entry
below).

**Recommendation.** A `CategoryReference(category_id)` field kind with a tag
autocomplete, storing the array shape the gather operators already read.

### G-FORM-SAVE-DROPS-EVERY-FIELD-THE-FORM-DID-NOT-RENDER: **[FIXED at main 81290cc]** saving an item through either form replaces its whole field set, so anything the form cannot show is destroyed

**Fixed at Trovato `main` 81290cc, verified 2026-11-23 (A11).** Kernel `cca05f9` (B125). `saving_the_edit_form_keeps_every_field_including_the_ones_it_did_not_render` (formerly the test that pinned the loss) saves through the kernel form as an editor and finds `field_topics`, which the form does not render, unchanged. What follows is the entry as written at 0.103.0.

`extract_content_fields` builds a **fresh** map from the POST body alone
(`crates/kernel/src/routes/admin_content.rs:23-33`): no whitelist against the
type's declared fields, no reference to the item being edited. It becomes
`input.fields`, and `Item::update` does `let fields = input.fields.unwrap_or(current.fields)`
(`crates/kernel/src/models/item.rs:326`) — a whole-value replace, under a comment
that says "Merge updates with current values". The non-admin form builds its map
the same way (`routes/item.rs:193-200`).

The consequence is not limited to undeclared keys. Any key the form did not render
is gone on save: a multi-value field (only one widget is ever rendered, see
`G-CARDINALITY-IS-INERT`), a field removed by `tap_field_access` for that viewer,
or a key written by a plugin. The audit saw `field_topics` disappear after one
save and read it as a consequence of the field being undeclared; declaring it
would not have helped.

The fix is not one line, because the two cases have to be told apart: a checkbox
posts nothing when unchecked, and that absence legitimately means false. Only the
route knows which fields it rendered, so preserving unrendered keys means seeding
the map from the existing item and then explicitly zeroing the declared fields the
form showed. That is a real change, in two form paths.

**Blocks.** 29.1, 36.1, and every editorial row that depends on an edit being safe.

**Recommendation.** Have both form paths share one typed submission builder that
takes the existing item, the content type and the rendered field list, and writes
back only what the form was actually capable of showing.

### G-CARDINALITY-IS-INERT: **[FIXED at main 81290cc]** `cardinality` round-trips through configuration and is read by nothing that renders, collects or validates

**Fixed at Trovato `main` 81290cc, verified 2026-11-23 (A11).** The edit form renders `field_venue_photos[0]`, `[1]` and `field_speakers[0]`, and a save posting `name[i]` keeps both as arrays (`the_conference_edit_form_is_generated_from_the_type_and_loads_the_block_editor` and `saving_the_edit_form_keeps_every_field_including_the_ones_it_did_not_render`). What follows is the entry as written at 0.103.0.

`FieldDefinition.cardinality` exists, defaults to 1, and `-1` means unlimited
(`crates/plugin-sdk/src/types.rs:229-244`; the only place the convention is
enforced is the admin field form, `routes/admin_content_type.rs:575-578`). It
survives config import and shows in the field-edit screen.

Nothing else reads it. `render_field` never looks at it
(`crates/kernel/src/content/form.rs:221-495`) and neither does
`templates/admin/content-form.html`, so one widget is rendered whatever the value;
no submit handler collects repeated inputs; and the only save-time field check is
`validate_required_fields` (`content/compound.rs:287-324`), which counts nothing.
Not implemented for `File` or for `RecordReference` in particular.

**Impact for Ritrovo.** `venue_photos` and `speakers` are declared `cardinality: -1`
because that is what the brief means and what the config format records. Seeded
arrays store, query and render correctly; an editor opening the form sees one box,
and saving through it collapses the array (`G-FORM-SAVE-DROPS-...`, above).

**Blocks.** 29.1, D1, 34.3.

**Recommendation.** Render `cardinality != 1` as a repeating widget with the
add-another the AJAX layer already has, collect indexed inputs into an array, and
validate the count on save.

### G-NO-FILE-CONFIG-ENTITY: **[Medium, NEW]** a configuration set cannot ship a file, so a demo cannot show an image without a human uploading one

Config import handles thirteen entity types in dependency order
(`crates/kernel/src/config_storage/yaml.rs:51-65`) and none of them is a file. An
`item.*.yml` can carry a file's uuid in `fields` — `ConfigItem.fields` is an
unconstrained `serde_json::Value` (`config_storage/mod.rs:79-80`) — but the
`file_managed` row it names has to already exist, and the only way to create one
is `POST /file/upload` (`routes/file.rs:25`), which needs a session and a CSRF
header (`routes/file.rs:60-80`).

**Blocks.** 34.2, D17, and the brief's "logo required on Live".

**Recommendation.** A `file` config entity that carries its bytes, or a CLI
`file import` that takes a path and a uuid.

### G-CONFIG-IMPORT-NEVER-PROMOTES-A-FILE: **[Medium, NEW]** a file referenced by imported configuration stays temporary and is deleted six hours later

Uploads land as `FileStatus::Temporary` and are promoted only by
`promote_file_ids`, called from the admin content routes and nowhere else
(`crates/kernel/src/routes/admin_content.rs:62-78`, the single `mark_permanent_batch`
call site at `:69`). Cron deletes temporary files older than six hours
(`cron/tasks.rs:176-184`, `TEMP_FILE_MAX_AGE_SECS`).

So even the workaround for the entry above fails: a script that logs in, uploads
the images and imports items referencing them produces a site whose images all
disappear that afternoon. Together the two entries mean an image can reach a
Trovato site only by a human using the admin form.

**Blocks.** 34.2, D17, and the seeded logo, gallery and headshot in `demo/config`.

**Recommendation.** Promote files referenced by imported items, the way the admin
routes do.

### G-ITEM-ROUTE-DOES-NOT-RESOLVE-CATEGORY-TAGS: **[Medium, NEW]** an item template can print a topic's uuid and nothing else

The item route resolves forward `RecordReference` targets into `referenced_items`
and computes `reverse_references` for every referring type
(`crates/kernel/src/routes/item.rs:583-650`), and inserts `safe_urls`,
`breadcrumbs` and the rest. It never looks at a category tag, and there is no Tera
filter or function for one either: the registered filters are `text_format`,
`format_date`, `safe_html`, `render_blocks`, `markdown`, `responsive_image`,
`field_type_label` and `trans` (`crates/kernel/src/theme/engine.rs:347-548`).

So a conference page holding `field_topics: ["<uuid>", "<uuid>"]` can render those
uuids and nothing else: no label, no link, no breadcrumb. The gather side is fine
— `/topics/{slug}` resolves the term on the route and the kernel supplies
breadcrumbs (`routes/gather.rs:432`).

**Blocks.** D5 (topic pills), D2 (the topic breadcrumb on an item page).

**Workaround in Ritrovo (A9).** The topic tree is Ritrovo's configuration, so its
labels are compiled into a template: `docs/tutorial/templates/includes/topics.html`
holds `pill`, `trail` and `label` macros with a branch per term, generated from
`demo/config/tag.*.yml` by `demo/checks/tests/topic_labels.rs`, which fails the
build when it is stale. The conference page renders topic pills through it. It
is a copy of data the kernel already has, and goes away with this entry.

**Recommendation.** Resolve an item's category references the way forward record
references are resolved, into a `referenced_tags` map of label, slug and url.

### G-NO-RELATIVE-DATE-FILTER-VALUES: **[Low, NEW]** a gather can say "today" and nothing else, so a date window cannot be expressed

`ContextualValue` is `CurrentUser`, `CurrentTime`, `CurrentDate` and `UrlArg`
(`crates/kernel/src/gather/types.rs:335-351`). There is no first-of-month,
end-of-month or today-plus-N, and a literal date in a config file is wrong the day
after it is written.

**Impact for Ritrovo.** Both of the brief's tile gathers are defined as date
windows: "Conferences This Month" between the first of this month and the first of
next, and "CFPs Closing Soon" between today and today plus fourteen days. Neither
window is expressible. Both gathers exist, bounded by count instead of by date,
and each file says so.

**Blocks.** 34.4, D12.

**Recommendation.** A relative date contextual value, for example
`current_date_plus(days)` and a month-boundary pair.

### G-QUEUE-DEAD-LETTER-DISCARDS-THE-PLUGINS-ERROR: **[Low, NEW]** a dead-lettered job records that the worker failed, never what it said

The queue's retry and dead-letter tier is complete and works: `attempts`,
`max_attempts` (5), exponential backoff, `status = 'dead'`, `dead_reason`,
`last_error` (`crates/kernel/migrations/20260403000001_plugin_queue_v2.sql`,
`crates/kernel/src/cron/mod.rs:254-300`). What it records is a constant:
`let err = "tap_queue_worker failed (trap or error result)"` (`cron/mod.rs:235`).

It cannot record more, because a tap's `Err` value never crosses the ABI — the SDK
signals failure with a negative length and drops the payload
(`crates/plugin-sdk-macros/src/lib.rs:247-291`), so `dispatch_to_plugin` returns
`None` and there is nothing to write down.

**Impact for Ritrovo.** `ritrovo_importer` writes the reason into its own
`ritrovo_state` before returning `Err`, and its admin screen reads those back, so
"logged and skipped, not silently dropped" is true. Every plugin has to invent
that for itself.

**Blocks.** P3, in the sense that the queue alone cannot answer why a batch died.

**Recommendation.** Carry the `Err` string across as the failure payload and store
it in `last_error`.

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

**Closed from Ritrovo's side, 2026-09-18 (A5). The documentation finding stands.**
`ritrovo_access` now implements `tap_field_access` and denies
`field_editor_notes` to anyone without an editor's permission, and the no-op
`tap_item_view` stub is gone: it fired on every item view and did nothing. A
host-in-the-loop test asserts the key is absent from `Item.fields` for a reader
and present for an editor, so the hiding is the kernel removing a field rather
than a template declining to print one.

The `edit` half of the recommendation is unaddressed and is the kernel's: a field
hidden on view is still rendered on the edit form. In Ritrovo nobody who lacks an
editor's permission can open that form, because `tap_item_access` denies them the
`edit` operation, so the guard is the item tap rather than the field tap. That is
a property of this plugin's rules, not of the kernel, and it would not hold for a
site whose field rule and item rule differed.

### G-USER-API-NO-ADMIN-BYPASS: **[FIXED at 0.103.0]** `current-user-has-permission` is a literal membership test, so an administrator fails a plugin's permission check

**Fixed at Trovato 0.103.0, verified 2026-09-22.** The host function now answers
the *effective* permission: `crates/kernel/src/host/user.rs:55` calls
`caller.data().request.user.can(&permission)`, and `UserContext::can` is
`self.is_admin || self.has_permission(permission)`
(`crates/kernel/src/tap/request_state.rs:162-164`). So a plugin's own check and
the kernel's route guard now agree about an administrator.

The root cause behind it was fixed in the same release: `UserContext::is_admin()`
used to be *derived* from whether the permission list contained the string
`administer site`, which made the list a place where authority could be forged.
It now carries the `users.is_admin` column, and nothing is pushed onto an
administrator's permission set.

**Read the kernel's own changelog for this release with care.** It contains two
paragraphs that contradict each other: the release summary says
`current-user-has-permission` now answers the effective permission, and a later
entry says it is "Not changed, and reported instead: ... still a literal string
membership test". The code at the tag settles it — `host/user.rs:55` is `.can()`
— and the kernel's own test
`ai_assistant_test.rs::an_administrator_alone_passes_the_plugins_own_check`
pins the new behaviour. The stale paragraph appears to have been written while
the fix landed elsewhere in the same release and never retracted. Reported as a
documentation finding rather than a behaviour one.

The entry as originally written follows.

---


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

### G-NO-FORM-STATE-HOST-API: **[Medium, NEW]** the kernel has a form-state table, a writer for it and a cron task to expire it, and a plugin can reach none of them

`crates/kernel/migrations/20260212000010_create_form_state_cache.sql` creates
`form_state_cache` with exactly the columns a multi-step form needs.
`FormService::save_state` and `::load_state` write and read it
(`crates/kernel/src/form/service.rs:417`, `:437`), and
`CronTasks::cleanup_form_state_cache` expires stale rows
(`crates/kernel/src/cron/tasks.rs:248`). Every piece of a multi-step form's state
management exists and is wired to a cron schedule.

**No route calls `FormService`** (`G-FORM-TAPS-UNREACHABLE`), and no host
interface exposes any of it, so the table is written by nothing and readable by
nobody. A plugin building a multi-step form has two options: write to the
kernel's table through raw SQL, which means squatting in a schema the kernel is
free to change and a cron task it is free to retune; or create its own table,
its own expiry and its own migration, which is one more copy of a thing the
kernel already has per plugin that needs one.

`ritrovo_forms` does the second (`plugins/ritrovo_forms/migrations/002_create_ritrovo_form_state.sql`).
The rows are the same shape as the kernel's, under a name this plugin owns.

**Blocks.** Nothing outright; it makes 36.4 and D22 more code than they should
be, and gives the site two form-state tables with two expiry policies.

**Recommendation.** A `form-state` host interface, or the smaller version: let a
plugin name a key and store a JSON blob against the current session with a TTL.
The cron task that already runs would then expire every plugin's state too.

### G-PLUGIN-CANNOT-READ-BACK-AN-INSERT: **[Medium, NEW]** a plugin cannot learn the id of a row it just wrote

`query_raw` refuses any statement that is not read-only
(`crates/kernel/src/host/db.rs:169-174`), so `INSERT … RETURNING id` is rejected
before it reaches the database. `execute_raw` runs the insert and returns a row
count. There is no third option in the SDK: `crates/plugin-sdk/src/host.rs`
exposes `query_raw` and `execute_raw` and nothing else for the database.

The kernel does implement what is needed. `crates/wit/kernel.wit` declares
`insert: func(table: string, data-json: string) -> result<string, string>` in the
`db` interface, and `do_insert` executes it with `RETURNING *` and serializes the
row (`crates/kernel/src/host/db.rs:370`). **The SDK wraps none of the four
structured calls** — `select`, `insert`, `update`, `delete` — so a plugin author
reading the SDK has no way to reach them and no indication they exist.

The consequence is small per site and awkward per plugin: any plugin writing a
row it then has to refer to must generate the key itself.
`ritrovo_forms::submit::new_uuid` does that, from the kernel's CSPRNG, which is a
normal enough pattern that it is worth saying it was not a choice.

**Blocks.** Nothing; it costs every plugin that owns a table a key generator.

**Recommendation.** Wrap the structured `db` calls in the SDK, or let
`execute_raw` return the rows of a `RETURNING` clause.

### G-BAD-CAPABILITY-NAME-MAKES-A-PLUGIN-VANISH: **[Low, NEW]** a manifest with an unknown `host_interfaces` name reports the plugin as missing rather than as wrong

`KNOWN_HOST_INTERFACES` (`crates/kernel/src/plugin/info_parser.rs:369-384`)
rejects an interface name that is not on its list, which is right. What an
operator sees is not: `trovato plugin install <name>` answers

```
plugin 'ritrovo_forms' not found in /…/overlay/plugins
```

for a plugin whose directory, module and manifest are all present and correct
except for one word. The directory scan parses each manifest and drops the ones
that fail, so a rejected manifest is indistinguishable from a missing plugin.

The manifest name matches the WIT interface exactly (`crypto-api` in both), so
the list is consistent; what it is not is discoverable from the SDK, where the
only thing an author sees is `host::crypto_random_bytes`. Writing `crypto` and
getting "not found" is the cheap mistake, and the error names neither the file
that failed to parse nor the word that was wrong. Encountered exactly that way
while building `ritrovo_forms`.

**Blocks.** Nothing. It is a diagnostic, and the friction log is for these too.

**Recommendation.** Report the manifest that failed to parse and why, at least at
`warn`, rather than silently omitting it from the scan.

### G-NO-TEXT-FORMAT-HOST-API: **[Medium, NEW]** a plugin cannot run the kernel's own HTML filter, so a plugin-served form cannot accept rich text

The kernel has text formats and uses them: `plain_text` escapes, `filtered_html` runs
the value through ammonia, `full_html` trusts it, and the permissions
`use filtered_html` and `use full_html` decide which a user may pick. All of it is
kernel-side. **No host interface exposes any of it to a plugin.** There is no
`text_format` import in the WIT world, and nothing in
`crates/plugin-sdk/src/host.rs` sanitizes, renders or filters: the imports are `db`,
`http`, `queue`, `mail`, `crypto`, `user`, `variables`, `logging`, `ai` and
`plugin-api`, and that is the whole list.

So a plugin serving its own form has two options for a rich-text field, and both are
wrong. It can accept HTML and render it, which is a stored-XSS hole, because the
kernel explicitly does not sanitize a plugin's response body
(`crates/kernel/src/routes/plugin_api.rs:60-66`). Or it can carry its own sanitizer
inside the WASM module, which means a second implementation of a security-critical
filter, shipped per plugin, versioned separately from the one the kernel applies to
the same site's content.

`ritrovo_forms` does neither: the member bio is plain text, rendered as escaped
paragraphs (`plugins/ritrovo_forms/src/web.rs`, `paragraphs`). That is a real
reduction against the brief, which asks for a bio with a WYSIWYG, and it is the only
option that is not a hole or a fork.

The same gap is why a plugin cannot offer the kernel's WYSIWYG at all: the editor is
a static asset the kernel's admin template loads, and a plugin response cannot add a
script tag to the page it is themed into.

**Blocks.** 36.7 (bio as `filtered_html`), 36.2 where a plugin serves the form.

**Recommendation.** A host call taking a string and a format name and returning the
filtered result, refusing a format the current user may not use. The filter already
exists and already knows the permission rule; only the seam is missing.

### G-ITEM-EDIT-FORM-LOADS-NO-ASSETS: **[FIXED at main 81290cc]** the non-admin item form renders the block editor's container onto a page with no scripts, so rich text is administrator-only in practice

**Fixed at Trovato `main` 81290cc, verified 2026-11-23 (A11).** Kernel `044c5dc` (B71.3) put every content form through `FormService`. The editor's `/item/{id}/edit` loads `block-editor.js`, asserted by `the_conference_edit_form_is_generated_from_the_type_and_loads_the_block_editor` (`plugins/ritrovo_access/tests/ritrovo_access_host.rs`). What follows is the entry as written at 0.103.0.

`/item/{id}/edit` builds its response as a bare HTML document in
`crates/kernel/src/routes/item.rs:1095-1115`: a `<style>` block, the form, and no
`<script>` of any kind. It is not the site template and it loads no assets.

`FormBuilder::render_field` nonetheless renders a `Blocks` field as
`<input type="hidden">` plus `<div data-block-editor>`
(`crates/kernel/src/content/form.rs:413-428`), which is exactly what the block
editor attaches to on the administrator's content form, where
`templates/admin/content-form.html` does load it. On this page nothing attaches. An
editor sees a labelled empty div, the hidden input carries the stored JSON, and
there is no way to change it.

The consequence for Ritrovo is that `field_description` — a `Blocks` field precisely
because that is the kernel's rich text — is not editable by the role the brief calls
an editor. It is editable by an administrator, on a screen `G-ADMIN-SCREENS-ARE-ADMIN-ONLY`
keeps editors off. Verified by
`the_conference_edit_form_is_generated_from_the_type_and_has_no_editor` (since renamed `..._loads_the_block_editor`, which asserts the opposite) in
`plugins/ritrovo_access/tests/ritrovo_access_host.rs`, which asserts the container is
rendered and the page carries no script.

Rendering a widget that cannot work is worse than rendering a textarea: a textarea
would at least let an editor write something.

**Blocks.** 36.2, D7 (WYSIWYG for descriptions, for any role but administrator).

**Recommendation.** Serve the non-admin item form through the site template with the
same assets the admin form gets, or have `FormBuilder` fall back to a textarea for
`Blocks` when the response will not carry the editor.

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

**A8 re-check at 0.103.0: NARROWED, and no longer "fixed".** Each bucket's count can now
be overridden for the whole site, by `TROVATO_RATE_LIMIT_<BUCKET>` in the environment or
`rate_limit.<bucket>` in site configuration, environment first
(`crates/kernel/src/middleware/rate_limit.rs:248-292`, applied at
`crates/kernel/src/state.rs:324`). The window is not configurable, and nothing is keyed by
role or by token: the `api` bucket is one number, 100 a minute by default
(`rate_limit.rs:179`), counted per client IP (`check_rate_limit`) and again per signed-in
user (`check_authenticated_rate_limit`, `:655`). A refusal is 429 with `retry-after`
and `{"error":"Rate limit exceeded","retry_after":N}` (`:603-613`). The brief's 60 for
anonymous and 300 for authenticated still cannot be expressed. Every `/api/` path but
`/api/search` falls in the `api` bucket, `/api/v1/search` included (`:491-494`).
`docs/API.md` documents the default and the override.

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

**Re-verified at 0.103.0, 2026-11-14 (A9 part 3). Narrowed, still open.** The
timing middleware is layered now, outermost (`crates/kernel/src/main.rs:410-415`),
and every response carries `Server-Timing: total;dur=<ms>`. That is the whole of
it: one number per request, with no query count, no cache hits and no breakdown,
because per-query timing "would require wrapping the `PgPool` and is not done"
(`crates/kernel/src/middleware/query_profiler.rs:47-50`). Gander still does not
exist. `docs/CACHING.md` walks through the cache by behaviour instead, because
there is no profiler to read it from, and the cache counters it would have read
never move (`G-CACHE-METRICS-NEVER-COUNT`).

### G-REVISION-NO-COMPARE: **[FIXED at main 81290cc]** revisions store a change summary, and nothing shows a comparison

**Fixed at Trovato `main` 81290cc, verified 2026-11-23 (A11).** Kernel `4c75594` (B70) added `/item/{id}/revisions/{rev}` (preview) and `/item/{id}/revisions/compare?from=&to=`. The same test, `an_editor_lists_previews_compares_and_reverts_revisions`, gets 200 from both, with the old value in the preview and both values in the comparison. What follows is the entry as written at 0.103.0.

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

### G-BULK-ACTIONS-NOT-EXTENSIBLE: **[Medium, NEW]** a plugin cannot add a bulk action to the content list, so the brief's bulk publish cannot live where an editor would look for it

The content list's bulk action set is three literals in a `matches!`
(`crates/kernel/src/routes/admin_content.rs:625`, `publish | unpublish | delete`),
re-matched in the dispatcher below it with `_ => unreachable!()` on both arms
(`:685`, `:692`). Neither the allowlist nor the `<select>` in
`templates/admin/content-list.html:48-53` is data a plugin can reach.

There is no seam of any kind. `list_content` (`admin_content.rs:97-157`) dispatches
no tap; the plugin local-task mechanism (`routes/helpers.rs:712-736`) is wired into
the item edit page only, not the list; `KNOWN_TAPS`
(`crates/kernel/src/plugin/info_parser.rs:289`) contains no tap that concerns an
admin list, a bulk operation or a content-list form; and the form taps that might
have served are dispatched by a `FormService` nothing calls
(`G-FORM-TAPS-UNREACHABLE`). A template override could retarget the form's action
at a plugin route, but it could not widen the kernel handler's allowlist.

**Impact for Ritrovo.** The brief's "bulk publish from Curated to Live" cannot be
the content list's fourth bulk action. Ritrovo serves its own editorial screen at
`/admin/content/editorial` instead (`plugins/ritrovo_access/src/lib.rs`), which
works, and which is a second content screen a reader has to be told about rather
than the one already in front of them. Two further consequences follow from it
being a plugin route: `G-PLUGIN-ROUTE-NO-HEADERS` means no post/redirect/get and
no kernel flash message, so the POST renders its own result page; and the entry
does not appear in the admin sidebar, which is hardcoded
(`templates/page--admin.html:184,190` keys on two specific plugin names).

**Blocks.** 39.2, D25, and the "bulk operations" half of 35.5.

**Recommendation.** Make the action set a registry a plugin can extend, with the
label, the permission and the handler declared together; or dispatch a tap over
the selected ids so a plugin can offer an action without owning a screen.

### G-PLUGIN-CANNOT-INVALIDATE-THE-ITEM-CACHE: **[Medium, NEW]** a plugin's write to an item is invisible for up to five minutes, and nothing it can call fixes that

`ItemService` caches Items in process, keyed by uuid, with a TTL from
`CACHE_TTL_ITEMS` defaulting to 300 seconds
(`crates/kernel/src/content/item_service.rs:328-331`,
`crates/kernel/src/config.rs:50`). Reads go through it
(`item_service.rs:416-431`), and the only invalidation is
`ItemService::invalidate` (`:1513-1515`), which no host call reaches.

The cache host interface cannot stand in for it: every key and tag a plugin writes
is namespaced under the plugin's own name (`crates/kernel/src/host/cache.rs:123-129`,
`:163`), so a plugin cannot invalidate a kernel entry even by guessing the key. And
the item cache is not tag-based at all, so there is no tag to guess: the kernel's
one tag-based invalidation is the search result cache
(`crates/kernel/src/routes/api_search.rs:123`).

Neither write path helps. `item-api`'s `save-item` goes through `Item::update` and
never calls `invalidate` (`crates/kernel/src/host/item.rs`, no such call), and a
`db` write is further still from it.

**Impact for Ritrovo.** The editorial screen changes an item's stage, which is what
decides whether a visitor may see it. After promoting a conference to Live it
appeared in every listing at once, because Gathers query the database, and its own
page kept answering 404 until the entry expired, because `check_access` read a
cached Item still carrying the Curated stage. The demo sets `CACHE_TTL_ITEMS: 10`
(`docker-compose.demo.yml`) so a promotion looks immediate to someone clicking
through. That is a demo covering a gap, not a fix: on a default install the same
promotion is invisible on the item's own page for up to five minutes.

**Blocks.** Nothing outright; it degrades 35.3, 39.2 and D25.

**Recommendation.** A host call to invalidate an item by id, gated on the same
capability as the write that made it necessary, and called by `save-item` itself.

**Re-verified at 0.103.0, 2026-11-14 (A9 part 3). Still open.** The cache is built
at `crates/kernel/src/content/item_service.rs:323-331` and invalidated only by
`update` (`:736`), `delete` (`:910`) and `invalidate` (`:1590-1592`), which no host
call reaches. The host test
`a_kernel_edit_shows_at_once_a_raw_write_waits_for_the_cache_and_no_view_leaks`
(`plugins/ritrovo_access/tests/ritrovo_access_host.rs`) drives the kernel's item
route: an edit through the form is on the page at once, and a raw SQL title change
is not, until the entry is invalidated. `verify-demo.sh` shows the same on the
demo, stale at once and fresh after the 10-second TTL. The same test shows that an
editor's read of a Curated conference never reaches the next anonymous request:
the cache holds Items, and the access check runs after it on every request.

### G-CONFIG-ITEM-CANNOT-NAME-A-STAGE: **[Medium, NEW]** configuration can ship an item, and cannot say which stage it belongs on

`ConfigItem` has eight fields and no stage (`crates/kernel/src/config_storage/mod.rs:58-89`),
and `save_item` binds the constant: `.bind(crate::models::stage::LIVE_STAGE_ID)`
(`crates/kernel/src/config_storage/direct.rs:753`). Every item a configuration set
imports therefore lands on Live, whatever the set says.

This is inconsistent within config import itself: `menu_link` and `tile` both carry
`stage_id` in their config files and both honour it (round-tripped at
`crates/kernel/tests/config_import_test.rs:467-476`). Items, the entity stages exist
for, are the exception.

**Impact for Ritrovo.** The seeded conferences in `demo/config/seed-content` cannot
demonstrate the pipeline: they are published the moment they are imported. Only the
importer's own conferences can land on Incoming, and only because it writes its own
INSERT through the raw-SQL host call. A configuration set cannot ship a demo of the
editorial workflow at all.

**Blocks.** The seeded half of 35.3; contributes to D3.

**Recommendation.** A `stage` key on `ConfigItem`, resolved by machine name like
the other stage-aware entities, defaulting to Live when absent.

### G-RATE-LIMIT-COUNTED-TWICE: **[High, NEW]** every rate limit is spent twice per request, so the documented limit is half of itself

`RateLimiter::check` increments and then compares
(`crates/kernel/src/middleware/rate_limit.rs:164`, over the atomic INCR at
`:203-215`), so calling it twice for one request costs two of the budget. Both
the middleware and the handler call it for the same categories:

- the per-IP middleware categorises and checks every request
  (`middleware/rate_limit.rs:397`, layered at `crates/kernel/src/main.rs:372`),
  and `categorize_path` matches `/user/register` and `/user/login` including
  their `/json` variants (`:252-256`);
- the handlers check again for the same category: registration at
  `crates/kernel/src/routes/auth.rs:548` (form) and `:797` (JSON), login at
  `:150` and `:371`.

So `register: (3, 3600)` (`middleware/rate_limit.rs:107`) admits **one**
registration per hour, and `login` admits half its stated rate.

**Observed, on a stack brought up from empty volumes**, three registration POSTs
in a row: the first succeeded; the second was refused by the handler, with the
`AppError` body `{"code":"rate_limited",...}`; the third was refused by the
middleware, with the different body `{"error":"Rate limit exceeded",...}`. The two
distinct shapes are the two limiters, and they are the proof. Logging in three
demo users in a row trips the login limit the same way.

**Impact for Ritrovo.** The brief creates three editorial users through the
registration form. That is impossible: one per hour. `ritrovo_access`'s
`tap_install` creates them by SQL instead, which is also the only way to activate
them or give them a role. `scripts/verify-demo.sh` retries its three logins for
the same reason, so that its editorial checks are about the editorial workflow
rather than about the rate limiter.

**Blocks.** 35.1 as written.

**Recommendation.** Check in one place. The middleware already categorises every
request, so the handler checks are redundant; if a handler needs its own bucket,
give it a category the middleware does not also count, or make `check` idempotent
per request through a request extension.

### G-ITEM-TEMPLATE-HAS-NO-VIEWER: **[Medium, NEW]** an item template cannot tell who is looking at the page, so no per-viewer control can be rendered on one

The item route builds a fresh Tera context with seven keys and renders the item
template from it (`crates/kernel/src/routes/item.rs:676-691`): `item`,
`children`, `referenced_items`, `reverse_references`, `safe_urls`,
`active_language`, `text_direction`. The viewer is loaded for the request
(`item.rs:323`) and used for access control, and is never put in that context.
There is no Tera global either: the theme engine registers filters only.

Everything that names the viewer is added **after** the item template has
rendered, by `inject_site_context` (`routes/helpers.rs:275`, `:295-298`), so
`user_authenticated` and `csrf_token` reach the page template and not
`elements/item--{type}.html`. Even there the uid is never exposed.

**Impact for Ritrovo.** It is half of why the brief's Subscribe toggle is not on
the conference page. A control rendered from the template would be shown to
anonymous visitors, who would then be refused by the endpoint — the thing A7 was
told not to do. The other half is
`G-RENDER-CHILDREN-MIXES-FIELD-DUMP-AND-PLUGIN-OUTPUT`: `tap_item_view` *does*
know the viewer, through `current-user-id`, but its output is appended into
`children`, which this repository's conference template does not render because
that string also carries the kernel's raw field dump.

**The two compose into a closed door, which neither does alone.** With a viewer
in the template context, the template could render the toggle. With plugin
output separable from the field dump, the view tap could. With neither, a
per-viewer control cannot be put on an item page at all, and `ritrovo_notify`
serves it on a page of its own instead.

**Blocks.** 37.3's toggle-on-the-conference-page half, and P8. The subscription
itself is built.

**Recommendation.** Put the viewer in the item template's context — at least
`user_authenticated` and the uid — the way `inject_site_context` already does
for the page template. It is one insert, it exposes nothing a signed-in visitor
does not already know about themselves, and it is what lets a theme render the
per-viewer affordances every CMS front end has. Fixing either this or the
`children` split would open the door; fixing this one is much the smaller change.

### G-PLUGIN-PERMISSION-GRID-READS-BOOT-MEMORY: **[Low, NEW]** enabling a plugin refreshes its permissions for `config import` and not for the screen that grants them

0.103.0 refreshes a plugin's declared permissions when it is enabled from
`/admin/plugins` (`crates/kernel/src/routes/plugin_admin.rs:254-288`). That
refresh calls `persist`, which writes `plugin_permission` — and the permission
grid does not read that table. It renders from
`state.plugin_permissions()` (`routes/admin_user.rs:941`), an immutable `Arc`
built once at boot (`state.rs:555-573`, assigned at `:1089`, no setter and no
interior mutability). So the refresh reaches `config import`, which reads the
table (`config_storage/yaml.rs:844`), and not the grid.

The kernel's own prose says otherwise in two places: `plugin_admin.rs:241-243`
("so its permissions reach the grid without waiting for a restart") and
`KNOWN-ISSUES.md:215-223`. Its own test states the split correctly
(`plugin_permissions_test.rs:216-219`, "The grid reads the in-memory registry,
which is built at boot, so a row written afterwards is not on this app's
registry"), so this is known at test level and not carried into the docs.

In practice the success branch may be unreachable anyway: `dispatch_to_plugin`
returns `None` for a plugin whose module was not compiled at boot, which is
every plugin enabled while the server runs.

**Impact for Ritrovo.** Small and real. An operator who enables `ritrovo_notify`
on a running site is told its permissions are available now, goes to the grid to
grant `manage own subscriptions`, and does not find it. `demo/config` grants it
at import, so the demo never meets this; a site administrator following the
admin UI does.

**Blocks.** Nothing. It costs one confusing round trip per plugin.

**Recommendation.** Have the grid read `plugin_permission` rather than the
boot-time registry — it is the durable copy and it is already the source config
import trusts — or say plainly in both places that a restart is needed.

### G-QUEUE-WITHOUT-A-WORKER-IS-SILENT: **[Low, NEW]** a plugin can fill a queue nothing drains, forever, with no log line and no dead-lettering

The drain skips a plugin that exports no `tap_queue_worker` with a bare
`continue` and no log (`crates/kernel/src/cron/mod.rs:1004-1012`), and the whole
drain returns early if no plugin anywhere implements the tap (`:997-999`). Rows
that plugin pushed are never claimed, so `attempts` never increments and they
never dead-letter: they accumulate in `plugin_queue` indefinitely.

Nothing validates a declared queue name either. `tap_queue_info` is read for one
key, `concurrency`, and only from a JSON array (`cron/mod.rs:174-192`); the
`name` field is read by nothing in the kernel, and jobs are routed by
`plugin_name` alone.

**Impact for Ritrovo.** None today, and it is the reason a declaration is safe:
`ritrovo_notify` declares `ritrovo_notifications` with no worker behind it,
deliberately, because the queue's producer is blocked by
`G-QUEUE-NO-CROSS-PLUGIN` and `G-ITEM-API-BYPASSES-ITEM-SERVICE` and the
declaration is what the eventual fix attaches to. The declaration costs nothing
precisely because the drain ignores it. It is recorded because the same silence
is what would hide a *real* fault: a plugin that pushes jobs and loses its
worker in a refactor looks exactly like one that is idle.

**Blocks.** Nothing.

**Recommendation.** Warn once at startup for a plugin that declares a queue
through `tap_queue_info` and exports no `tap_queue_worker`, the way
`warn_unreachable_callbacks` already does for a menu entry whose callback the
kernel will never reach. The precedent and the machinery both exist.

### G-NO-CLOCK-AND-NO-DIAGNOSIS: **[Medium, NEW]** a plugin that reads the clock fails to instantiate, and the error names neither the import nor the reason

A plugin compiled for `wasm32-wasip1` that calls `std::time::SystemTime::now()`
imports `wasi_snapshot_preview1::clock_time_get`. The per-plugin linker does not
provide it, so the module fails to instantiate, and every tap on that plugin
fails with the same opaque line:

```
ERROR trovato::tap::dispatcher: tap invocation failed
  plugin=ritrovo_access tap=tap_menu error=failed to instantiate plugin 'ritrovo_access'
```

No missing import is named, no interface, no symbol. The plugin's routes 404, its
`tap_install` silently does nothing, and its `tap_cron` fails every cycle, all
with that one message. Nothing in the manifest could have declared the capability
either: `clock_time_get` is WASI, not one of `KNOWN_HOST_INTERFACES`
(`crates/kernel/src/plugin/info_parser.rs:369`), so `host_interfaces` has no
value that would have permitted it.

**Impact for Ritrovo.** `ritrovo_access` needed a `changed` timestamp for the
stage move and took it from `SystemTime::now()`, the obvious way. Every tap on
the plugin then failed and the demo came up with no editorial screen and no
editorial users. Diagnosis took a `strings` over the two `.wasm` files to spot
that the working plugin imported no clock and the broken one did. The fix is to
take the timestamp from Postgres (`extract(epoch from now())`), which is better
anyway, but nothing pointed at it.

**Blocks.** Nothing permanently; it cost an afternoon.

**Recommendation.** Name the unresolved import in the instantiation error — the
linker knows it. Say in the plugin documentation that the guest has no clock, and
that timestamps come from the host or the database. If a clock is meant to be
available, add it to the baseline and to `KNOWN_HOST_INTERFACES`.

---

### G-TRANSLATION-OVERLAY-SKIPS-THE-DEFAULT-LANGUAGE: **[Medium, NEW]** an item written in another language can never be read in the site's default one

The item route applies a translation only when the page language is not the site
default: `if active_language != state.default_language() { apply_translation_overlay(...) }`
(`crates/kernel/src/routes/item.rs:349`). The overlay is keyed on the page, not on the
item's own `language`. So an item whose record is Italian (`language: it`) with an
English translation renders its Italian at `/item/{id}`, and the English row the
translate screen accepted and saved is never shown anywhere. `available_translations`
skips the default language for the same reason (`crates/kernel/src/routes/helpers.rs:1051`).

**Impact for Ritrovo.** The brief's Italian seed is Italian conferences with English
translations. Shipped that way, `/` would show Italian. So the seed is shipped the other
way round: each record is written in English with `field_language: it` (the conference
is held in Italian), and its Italian is the `item_translation` row, which the kernel does
overlay under `/it/`. The translation queue still offers Italian conferences an English
translation, as the brief asks, and the kernel saves it; it will render once this is
fixed. Imported Italian conferences (the importer records their `language` as `it`) are
in that state today.

**Blocks.** 38.4's literal shape (Italian originals, English translations), and the
English half of 38.3 for Italian conferences.

**Recommendation.** Overlay when the page language differs from the item's own
`language`, not from the site default, and list the item's own language in
`available_translations` at the unprefixed path only when it is the default.

### G-ITEM-TEMPLATE-RENDERED-BEFORE-ITS-TRANSLATIONS: **[Low, NEW]** an item template cannot see which languages its item exists in, because the list is built after the template has rendered

The item route renders `elements/item--{type}.html` at `crates/kernel/src/routes/item.rs:688-691`
and only afterwards computes `available_translations` and inserts it into the context
(`:754-761`), for the page template. The comment at `:750-752` says "a theme builds a
language switcher out of `available_translations`"; a theme can, in `page.html`, which
has no per-type suggestion (`page_suggestions` yields `page--item--<uuid>` and `page`,
`crates/kernel/src/theme/engine.rs:918-936`), and the item template, where a switcher
belongs, cannot.

Found in A8: a switcher written against `available_translations` in
`item--conference.html` rendered nothing on every page while the `hreflang` tags built
from the same list appeared in `<head>`.

**Impact for Ritrovo.** The switcher is built from `item.id` and `active_language` and
offers the other language whether or not a translation exists; an untranslated
conference's Italian link shows its English original under `lang="it"`.

**Blocks.** Nothing outright; a switcher that hides itself for untranslated items.

**Recommendation.** Compute `available_translations` before rendering the item template.

### G-TOPICS-PAGE-IGNORES-LANGUAGE: **[Low, NEW]** `/topics` renders `lang="en"` under `/it/`, because its handler never reads the request language

`/topics` is `browse_topics` (`crates/kernel/src/routes/category.rs:57`, `:676`), which
takes no `ResolvedLanguage` and calls `inject_site_context(..., "/topics")` (`:706`), so
`active_language` falls back to the default. Gather routes set it
(`crates/kernel/src/routes/gather.rs:418-423`).

Observed in the A8 demo: `/it/argomenti` (alias of `/topics`) and `/it/topics` both
render `<html lang="en">`, while `/it/conferenze` and `/it/relatori` (gathers) render
`lang="it"`. `verify-demo.sh` reports the value and does not assert it.

**Blocks.** 38.2's "every `/it/` page is in Italian" for the topics index.

**Recommendation.** Take `ResolvedLanguage` in `browse_topics` and put it in the context,
as the gather handlers do.

### G-TRANSLATE-SCREEN-TEXT-FIELDS-ONLY: **[Low, NEW]** the translate screen offers `Text` and `TextLong` fields only, so a `Blocks` description cannot be translated there

`templates/admin/content-translate-edit.html:60` renders an input only for
`field.field_type.Text or field.field_type == "TextLong"`. The comment above it explains
why booleans and references are left out, and a `Blocks` field is neither: it is the
longest text on most items. Ritrovo's `field_description` is `Blocks`, so the screen
offers the title, URL, city, country and editor notes and not the description.

**Impact for Ritrovo.** A translated conference's description comes only from
configuration (the seed's `item_translation` files carry it) or not at all. The queue's
model drafts include a description an editor can read and has nowhere to paste.

**Blocks.** The description half of 38.3.

**Recommendation.** Offer `Blocks` fields with the block editor, or at least as JSON in a
textarea, as the edit form does for administrators.

### G-ALIAS-FALLBACK-HARDCODES-EN: **[Low, NEW]** alias lookup falls back to `en`, not to the site's default language

`UrlAlias::find_by_alias_with_context` matches `language IN ($3, 'en')` and prefers the
requested language (`crates/kernel/src/models/url_alias.rs:103-116`). The fallback is a
literal, not the configured default. Ritrovo's default is English, so the fallback is
what makes `/it/conferences/<slug>` (the address the kernel's own switcher builds from
an English alias) resolve to the item in Italian, and it would stop doing so on a site
whose default is anything else.

**Blocks.** Nothing in Ritrovo today.

**Recommendation.** Fall back to `state.default_language()`.

### G-RENDER-BLOCKS-PASSES-STRINGS-UNSANITIZED: **[High, NEW]** `render_blocks` sanitizes a block list and returns a plain string untouched

The filter renders an array through the block renderer, which sanitizes, and returns a
string value as it is ("Plain string fallback — return as-is",
`crates/kernel/src/theme/engine.rs:526`). Templates mark its output `| safe` on the
strength of the first branch. A `Blocks` field holding a string is therefore printed raw.

**Impact for Ritrovo: a stored XSS, confirmed and closed in the templates.** The
submission form at `/conferences/submit` (`ritrovo_forms`) stores the member's
description as a string, as any raw writer must, and `item--conference.html` printed
`field_description | render_blocks | safe`. Any signed-in member could therefore put
markup on the page an editor opens to review the submission. Reproduced on the demo at
0.103.0: a description of `<b id="xss-probe">probe</b>` rendered as a live element on
`/item/{id}` for `editor_alice`. The conference template and `item--speaker.html`
(the same pattern on `field_bio`) now print a string as escaped text and send only a block
list to `render_blocks`; `verify-demo.sh` submits markup through the form and asserts it
comes back escaped. Any other template that pipes a field through `render_blocks | safe`
is exposed the same way the moment that field can hold a string, and
`G-FRONTEND-ITEM-FORM-STORES-EVERY-FIELD-AS-A-STRING` says the non-admin form stores
every value as one.

**Recommendation.** Sanitize the string branch as the array branch is sanitized (or
escape it). A filter whose output templates are told to mark safe must be safe for every
input it accepts.

### G-THEMED-PLUGIN-PAGE-UNDER-ADMIN-RENDERS-UNWRAPPED: **[Medium, NEW]** a themed plugin page at an `/admin` path cannot render into the admin template

`ApiResponse::themed` asks the kernel to wrap a plugin's body in the site theme
(`crates/kernel/src/routes/plugin_api.rs:443`, `themed_response`). For any path under
`/admin` the theme engine picks `page--admin.html`
(`crates/kernel/src/theme/engine.rs:929-931`), and that template tests
`"comments" in enabled_plugins` and `"categories" in enabled_plugins`
(`templates/page--admin.html:184,190`). Only `render_admin_template` inserts
`enabled_plugins` (`crates/kernel/src/routes/helpers.rs:358`); `themed_response` calls
`inject_site_context`, which does not. Tera fails on the undefined variable, and the
kernel logs `failed to render a plugin page into the site theme; serving it unwrapped`
and serves the bare body with no stylesheet, header or admin navigation.

**Impact for Ritrovo.** The translation queue at `/admin/content/translations` rendered as
an unstyled table. Ritrovo ships `page--admin--content--translations.html`, which extends
the kernel's template and sets `enabled_plugins` to an empty list, so the page renders in
the admin layout without the sidebar's Comments and Categories links (a template cannot
ask which plugins are enabled). `ritrovo_access`'s editorial queue avoided the problem
by building its own full HTML page. Every new plugin admin page needs one of the two.

**Recommendation.** Insert `enabled_plugins` in `inject_site_context`, or in
`themed_response` when the path is an admin path, so a plugin's themed admin page gets
the context the kernel's own admin pages get.

### G-GATHER-TRANSLATION-MERGE-IS-BACKWARDS: **[Medium, NEW]** a gather listing under `/it/` shows the translated title and the original fields

`GatherQueryBuilder::apply_translation_coalesce`
(`crates/kernel/src/gather/query_builder.rs:384-402`) overlays a translation as
`COALESCE("_tr"."fields" || "item"."fields", "item"."fields")` (`:396`). PostgreSQL's
jsonb `||` keeps the **right** operand's value for a key both sides carry
(`select '{"a":"tr"}'::jsonb || '{"a":"base"}'::jsonb` is `{"a": "base"}`, run on the
demo database), so every translated field is overwritten by the original. The doc
comment at `:381-383` says the opposite ("translated keys override originals"), and
the kernel's own test (`:2124`) asserts the SQL text, not what it returns. The title
is a plain `COALESCE` and is translated correctly.

**Impact for Ritrovo.** On `/it/conferenze` a translated seed conference shows its
Italian title (where the title differs) above its English description. The detail page,
`/it/item/{id}`, is right, because the item route overlays fields in Rust. Ritrovo cannot
work around it: the card template is given the merged row and the translated value is
not in it. `verify-demo.sh` records what the listing shows as a note.

**Recommendation.** Swap the operands, `"item"."fields" || "_tr"."fields"`, and test the
rows a translated gather returns rather than the SQL string.

### G-ADHOC-GATHER-BYPASSES-FIELD-ACCESS: **[FIXED at main 81290cc]** an anonymous POST to `/api/gather/query` returns fields `tap_field_access` hides

**Fixed at Trovato `main` 81290cc, verified 2026-11-23 (A11).** Kernel `e6a1420` requires `administer site` for an ad-hoc gather. `verify-demo.sh` now asserts the anonymous POST it used to report gets no editors' notes (it is 401). What follows is the entry as written at 0.103.0.

`POST /api/gather/query` (`crates/kernel/src/routes/gather.rs:46`, handler
`execute_adhoc_query` at `:213`) runs any query definition the caller sends. It asks for
no permission and no CSRF token; an anonymous request with no cookie is answered. The
item access pass still runs, so the stage holds, but the field access pass does not
cover a whole-column selection: for an explicit field list,
`filter_row_fields` (`crates/kernel/src/gather/access.rs:195-207`) drops an output key
only when the tap denied the item field it maps to, and a selection of `fields` maps to
the item field `fields`, which no tap is ever asked about. The whole JSONB object comes
back. `validate_definition` (`crates/kernel/src/gather/gather_service.rs:1954-1961`)
checks the base table's characters and nothing else, so a definition can name any
table: on the demo, `base_table: users` returned no rows (the access pass dropped them)
and a `total` of 5, which is the row count of the users table.

**Impact for Ritrovo.** `field_editor_notes`, which `ritrovo_access` hides from everyone
but editors (`tap_field_access`), is readable by any anonymous visitor. Reproduced on the
demo at 0.103.0: `{"definition":{"base_table":"item","item_type":"conference","fields":
[{"field_name":"id"},{"field_name":"fields"}],…}}` returned Live conferences with their
editors' notes. A configured gather that selects `fields` leaks the same way, which is
why `/api/v1/conferences` is served by a gather that selects `item.*`, whose field
filtering does work (`access.rs:189-194`). Ritrovo cannot close the ad-hoc route: a
plugin cannot remove, wrap or gate a kernel route. Until the kernel does, nothing a
`tap_field_access` hides is private on this kernel.

**Recommendation.** Gate `/api/gather/query` on an administrative permission and CSRF,
and treat a whole-`fields` selection as `item.*` in the field access pass. Restrict
`base_table` to `item` and the registered record tables.

### G-API-TOKENS-NEVER-AUTHENTICATE: **[FIXED at main 81290cc]** a token from `/api/tokens` is rejected before the middleware that would accept it runs

**Fixed at Trovato `main` 81290cc, verified 2026-11-23 (A11).** Kernel `d7a014e` (S7). `verify-demo.sh` issues editor_alice a token at `/api/v1/tokens`, gets 200 from `GET /api/v1/tokens` with it as the only credential and 401 with none or a forged one, then revokes it (204). What follows is the entry as written at 0.103.0.

`POST /api/tokens` issues a token, and `authenticate_api_token`
(`crates/kernel/src/middleware/api_token.rs:39-60`) would accept it as
`Authorization: Bearer <token>`. It never gets the chance. `authenticate_bearer_token`
(`crates/kernel/src/middleware/bearer_auth.rs:25`) is layered outside it
(`crates/kernel/src/main.rs:386-393`: the later `.layer` wraps the earlier, so the JWT
check runs first) and answers every `Bearer` header itself. With OAuth off, which is the
case whenever `JWT_SECRET` is unset, it refuses with 401 "OAuth service unavailable"
(`bearer_auth.rs:43-51`). With OAuth on, an API token is not a JWT, and it refuses with
401 "Invalid token" (`:55-66`). The token middleware's own comment
(`api_token.rs:54-58`) expects the opposite order.

**Impact for Ritrovo.** Observed on the demo, which sets no `JWT_SECRET`: `editor_alice`
created a token (201) and the same token sent as a bearer was refused with 401 on
`/api/v1/search`, the gather route and a plugin route alike. The API is therefore
anonymous or cookie-authenticated in practice, and `docs/API.md` says so. A client that
needs more than a visitor can see has no way in.

**Recommendation.** Let `authenticate_bearer_token` pass a token it cannot parse as a
JWT on to the token middleware, or swap the two layers.

### G-OPENAPI-IS-A-STATIC-LIST: **[FIXED at main 81290cc]** `/api/openapi.json` lists a fixed set of kernel routes, three of which do not exist, and no plugin route

**Fixed at Trovato `main` 81290cc, verified 2026-11-23 (A11).** Kernel `226fa5d` and `3b86b5a` build the document from the served routes. `verify-demo.sh` asserts `/api/v1/openapi.json` lists Ritrovo's `/api/v1/conferences/{id}` and `/api/v1/topics`. What follows is the entry as written at 0.103.0.

`/api/openapi.json` (`crates/kernel/src/routes/api_v1.rs:75`, `:96`) renders the
registry `register_kernel_routes` fills by hand (`crates/kernel/src/routes/route_metadata.rs:66`).
Plugin routes from `tap_menu` never enter it. Three of the paths it lists answer 404 on
the demo, because no router serves them: `/api/v1/items`, `/api/v1/items/{id}` and
`/api/v1/categories` (the routed ones are `/api/items`, `/api/item/{id}` and
`/api/categories`). Neither the gather route nor `/api/tokens` is listed.

**Impact for Ritrovo.** The spec cannot describe Ritrovo's API: `/api/v1/conferences/{id}`
and `/api/v1/topics` are absent, and `/api/v1/conferences` is an alias, which no spec
generator could see. `docs/API.md` is the reference instead.

**Recommendation.** Build the spec from the routers, plugin routes included, or from
route metadata that `tap_menu` can supply.

### G-GATHER-EXPLICIT-COLUMNS-ARE-TEXT: **[Low, NEW]** an explicit-field gather returns every JSONB value as text, and its `url` label is overwritten

A gather that names `fields.field_online` or `fields.field_topics` returns the text
extraction of each: `"false"` for a boolean, and `"[\"324db1b0-…\"]"`, a JSON array
inside a string, for a list. A column labelled `url` is replaced by the kernel's own
computed item address. Observed on the demo with a candidate
`ritrovo.api_conferences` gather.

**Impact for Ritrovo.** The API's list endpoint cannot be a tidy explicit-column gather.
It is the listing's own `item.*` gather instead, typed and correctly field-filtered, at
the cost of the kernel's internal columns in every row (`search_vector`, `tenant_id`,
`retention_days`), which `docs/API.md` tells a client to ignore.

**Recommendation.** Extract with `->` and keep JSON types; namespace the computed `url`.

### G-UNKNOWN-GATHER-IS-A-500: **[Low, NEW]** executing a gather that does not exist answers 500, and a newly imported gather does not exist until restart

`/api/query/{query_id}/execute` for an unknown id fails inside the service and is served
as 500 "An unexpected error occurred" (`crates/kernel/src/routes/gather.rs:192-200`,
logged as `query not found`). The gather registry is loaded at startup, so a gather that
`config import` has just written is unknown until the kernel restarts. Observed on the
demo.

**Impact for Ritrovo.** None for the demo, which imports before it serves. A client probing
a misspelled query id gets a server error rather than a 404.

**Recommendation.** 404 for an unknown query, and reload the registry after an import.

### G-PAGEFIND-INDEX-OMITS-TITLES: **[Medium, NEW]** the client-side index never contains an item's title, so nothing is findable by its name

`build_index_inner` writes each item as an HTML fragment whose title is an `<h1>`
OUTSIDE the `<div data-pagefind-body>` element
(`crates/kernel/src/cron/pagefind.rs:239-243`). Once a page has a
`data-pagefind-body` element, Pagefind indexes only inside it, so the `<h1>` is
captured as the result's title metadata and never as searchable text. The body
is built from the fields `search_field_config` names (`:368-405`), read out of
`item.fields`; the title is a column, so a `title` row in `search_field_config`,
which the tutorial set ships for both types, finds nothing there. And an item
none of whose configured fields has text gets an empty body and is left out of
the index altogether.

Observed on the demo in Chrome, 2026-11-14, before the workaround: `/search?q=codemotion`
answered "No results found." for the seeded "Codemotion Roma 2026"; a search for
the city, "roma", found it. A search for a speaker by name found nobody. The
index held 50 pages for 61 Live items: the eleven with no description, city or
bio were absent.

**Workaround in Ritrovo.** `ritrovo_importer`'s `tap_cron` mirrors `item.title`
into `fields.title` for conferences and speakers
(`plugins/ritrovo_importer/src/search_text.rs`), which the export does read.
`verify-demo.sh` now queries the index through `pagefind.js` itself
(`scripts/pagefind-query.mjs`) for a conference title and a speaker name, and
compares the index's page count with the Live item count.

**Blocks.** Nothing now; it is why E4.5, 34.5 and D14 needed plugin code.

**Recommendation.** Put the title inside the indexed body (or mark the `<h1>`
`data-pagefind-weight`), and stop dropping items whose configured fields are
empty.

### G-PAGEFIND-RESULT-URL-IGNORES-ALIAS-LANGUAGE: **[Low, NEW]** a search result can link to another language's alias without its prefix, which is 404

The export loads every `/item/%` alias into one map with no language
(`crates/kernel/src/cron/pagefind.rs:166-176`) and uses whichever row the
`collect` kept as the result's `url` (`:186-187`). An item with an English and
an Italian alias gets either, and an Italian alias is stored without its `/it`
prefix (the language middleware strips the prefix before lookup), so the link is
`/conferenze/<slug>`, which an English request cannot resolve.

Observed on the demo, 2026-11-14: a Pagefind search for "Codemotion Roma 2026"
returns it with the URL `/conferenze/codemotion-roma-2026`, 404;
`/it/conferenze/codemotion-roma-2026` is 200. Which alias wins is not under
Ritrovo's control. `verify-demo.sh` reports the link it gets and does not fail on
it.

**Blocks.** A clean click-through for the 15 seeded conferences that have an
Italian alias (D14, partly).

**Recommendation.** Prefer the default language's alias, or prefix the alias with
its language when it is not the default.

### G-PAGEFIND-IGNORES-FIELD-WEIGHTS: **[Low, NEW]** the A to D weights shape the server's ranking and not the browser's

The weights in `search_field_config` reach the tsvector trigger
(`crates/kernel/migrations/20260226000001_fix_search_trigger_char_cast.sql:9-29`).
The Pagefind export reads only the field names and joins the values into one
undifferentiated body, ordered by a name heuristic (`description`, `body` and
`bio` first) rather than by weight (`crates/kernel/src/cron/pagefind.rs:375-392`).
So the page a visitor uses ranks a match in a city the same as a match in a
title.

**Blocks.** "Weighted fields" on the user-facing search (34.5, D14) holds for
`/api/v1/search` and the no-JavaScript results only.

**Recommendation.** Emit each configured field in its own element with
`data-pagefind-weight` derived from its letter.

### G-SEARCH-CANNOT-INDEX-CATEGORY-LABELS: **[Low, NEW]** a taxonomy field cannot be searched by its terms' names

The trigger indexes `fields->>name` as text
(`crates/kernel/migrations/20260226000001_fix_search_trigger_char_cast.sql:20-23`),
and the Pagefind export does the same. A field holding tag uuids, which is how
`field_topics` is stored because no field kind is taxonomic
(`G-NO-CATEGORY-REFERENCE-FIELD-KIND`), indexes as uuid fragments. The brief's
"topics, weight C" cannot be configured.

**Workaround in Ritrovo.** The brief itself calls the field "topics
(denormalized)": `ritrovo_importer`'s `tap_cron` writes `field_topic_names`, the
labels of a conference's tags and of their ancestors, and
`demo/config/search_field_config.5b1e7c1a-….yml` weights it C. `verify-demo.sh`
reads the weight back out of the tsvector.

**Blocks.** Nothing now.

**Recommendation.** Resolve category references to labels when indexing.

### G-SEARCH-TRIGGER-INDEXES-BLOCKS-JSON: **[Low, NEW]** a `Blocks` field is indexed as its JSON, so every description matches "paragraph"

The trigger takes `fields->>name`, which for a `Blocks` field is the serialized
block list (`crates/kernel/migrations/20260226000001_fix_search_trigger_char_cast.sql:20-23`).
The structure's own keys and values become words: on the demo the seeded
"Droidcon Italy 2026" carries `'paragraph':B`, `'text':B`, `'data':B` and
`'type':B`, so `/search?q=paragraph` matches every conference with a
description.

**Blocks.** Nothing in the brief; it is noise in every ranking.

**Recommendation.** Index `Blocks` fields through the same text extraction the
renderer uses.

### G-MARKDOWN-FILTER-HAS-NO-TABLES: **[Low, NEW]** the `markdown` filter does not render tables

`pulldown_cmark::Parser::new` with no options
(`crates/kernel/src/theme/engine.rs:560`), so GitHub-style tables, which every
document in this repository uses, render as paragraphs of pipes.

Observed on the demo: `/api-docs`, built from `docs/API.md` at bootstrap, shows
its endpoint table as `| Endpoint | Served by | Why |` text.

**Blocks.** Nothing; the API page is readable and uglier than it should be.

**Recommendation.** `Parser::new_ext` with `Options::ENABLE_TABLES` (and
strikethrough), as GitHub renders them.

### G-PLUGIN-MENU-ENTRIES-WITH-A-PARENT-RENDER-NOWHERE: **[Medium, NEW]** a plugin menu entry with a `parent` reaches no menu a page template is given

A plugin's `tap_menu` entries reach templates in exactly one variable, `menus`,
which is `MenuRegistry::root_menus_for(viewer)` (`crates/kernel/src/routes/helpers.rs:271-272`),
and `root_menus` keeps only entries with NO parent
(`crates/kernel/src/menu/registry.rs:228-232`). An entry with a parent is used for
one thing, local-task tabs (`registry.rs:271-279`), and only when it also sets
`local_task`. The admin sidebar does not read the registry at all; it is a fixed
list in `templates/page--admin.html:177-209`. And the kernel's own `page.html`
shows `menus` only when the main menu is empty (`templates/page.html:35`), which
on any site with a main menu is never.

Observed: `ritrovo_notify`'s "Your subscriptions" (`parent("/user")`),
`ritrovo_forms`' "Your profile" (`parent("/user")`), `ritrovo_access`'s "Editorial
queue" and `ritrovo_translate`'s "Translations" (both `parent("/admin/content")`)
appear in no menu on any page. The importer's two admin screens did the same until
A9 dropped their parents.

**Ritrovo's side.** Ritrovo's `page.html` renders `menus`, the root entries, split
by path into the account menu and an admin bar for the viewers allowed to open
them, and the importer's two entries are root entries now so they reach it. The
other four keep their parents, because their tests assert them
(`the_list_sits_under_the_user_menu` among them) and those tests are right about
where the entries belong; the account menu links "Your profile" by hand.

**Blocks.** 34.4's user and admin menus being fed by `tap_menu` in full.

**Recommendation.** Give templates the whole tree (`menus` by parent), or render
children of `/user` and `/admin/*` in the kernel's user and admin menus.

### G-NO-VIEWER-ID-IN-TEMPLATES: **[Medium, NEW]** nothing a page template is given names the signed-in user, so no template can link to a page under `/user/{uid}`

`inject_site_context` gives every page `user_authenticated`, `csrf_token` and
`user_can_access_admin`, and nothing that identifies the viewer: no id, no name
(`crates/kernel/src/routes/helpers.rs:183-345`). This is the page-layout half of
`G-ITEM-TEMPLATE-HAS-NO-VIEWER`, which is the same gap in the item template's
context; together they leave no template on a page that can name its reader. A tile body is one string for
every viewer (`crates/kernel/src/services/tile.rs:44-56`).

Observed: `ritrovo_notify` serves a member's subscriptions at
`/user/{uid}/subscriptions`, with the uid in the path because the privacy rule
needs something to refuse. No menu, tile or template can link to it, because none
can write the uid, and a "My Subscriptions" tile cannot hold the list because a
tile cannot vary by viewer.

**Ritrovo's side.** The My Subscriptions tile (`roles: [authenticated user]`) is a
signpost to `/user/bio`, a plugin page, which does know the uid and links to the
list (`plugins/ritrovo_forms/src/profile.rs`, `account_links`). Two clicks
instead of one.

**Blocks.** The brief's My Subscriptions tile as a list, and one-click user-menu
entries for any per-user page.

**Recommendation.** Put the viewer's id and display name in the site context, and
let a tile type render per request.

### G-ONLY-MAIN-AND-FOOTER-MENUS-RENDER: **[Low, NEW]** a config menu other than `main` or `footer` is stored and never reaches a template

`inject_site_context` loads two config menus by name, `main` and `footer`
(`crates/kernel/src/routes/helpers.rs:249-262`). A `menu_link` row with any other
`menu_name` imports, is listed at `/admin/structure/menus`, and renders nowhere.
`templates/page.html:42-51` hard-codes the account links.

**Ritrovo's side.** The user menu is written in Ritrovo's `page.html`: Login and
Register, or Your profile, Account and Logout, plus the plugin root entries that
are not under `/admin`.

**Recommendation.** Expose every config menu to templates as `menus_by_name`.

### G-TILE-ROLES-ARE-SYNTHETIC: **[Low, NEW]** tile visibility by role sees three made-up roles, not the viewer's real ones, and every viewer is "anonymous user"

The roles a tile's `visibility.roles` is matched against are built in
`inject_site_context` from three facts: `anonymous user` always,
`authenticated user` when signed in, `administrator` for an admin
(`crates/kernel/src/routes/helpers.rs:283-289`). The viewer's actual roles are not
consulted, so a tile for `editor` or `publisher` never shows, and a tile limited to
`anonymous user` shows to everybody, signed in or not
(`crates/kernel/src/models/tile.rs:213-223`).

**Ritrovo's side.** The one role-limited tile, My Subscriptions, uses
`authenticated user`, which works.

**Recommendation.** Pass the viewer's real role names, and stop adding
`anonymous user` for a signed-in viewer.

### G-TILES-HAVE-NO-LANGUAGE: **[Low, NEW]** a tile's label and body are one string in every language

`Tile` carries no language and no translations (`crates/kernel/src/models/tile.rs:12-27`),
and `render_region` is not told the request language
(`crates/kernel/src/services/tile.rs:44-56`). So `/it/` shows "CFPs Closing Soon"
in English, and so does every tile on every Italian page.

**Recommendation.** Translate tile labels and bodies the way menu links already are
(`localize_menu_links`, `helpers.rs:253`).

### G-BREADCRUMBS-IGNORE-THE-TOPIC-TREE: **[Low, NEW]** breadcrumbs know the content type and the query label, never the category a page sits in

An item page's breadcrumbs are Home › type label › title
(`crates/kernel/src/routes/item.rs:708-719`); a gather page's are Home › query
label (`crates/kernel/src/routes/gather.rs:427-432`), so every `/topics/{slug}`
page has the same trail. Neither can follow a tag to its parents.

**Ritrovo's side.** The conference template and the topic listing draw the trail
themselves from `includes/topics.html`, which is generated from the tag config and
held to it by a test (`demo/checks/tests/topic_labels.rs`): Home › Languages › JVM
› Kotlin. Ritrovo's `page.html` drops the kernel's trail when that one is present.

**Recommendation.** Let a breadcrumb follow a category reference, and resolve tag
labels and ancestors for templates (`G-ITEM-ROUTE-DOES-NOT-RESOLVE-CATEGORY-TAGS`).

### G-PLUGIN-RESPONSE-CANNOT-REDIRECT: **[Low, NEW]** a plugin route cannot answer with a redirect

`ApiResponse` has a status, a body, a content type and two theming fields, and no
headers (`crates/plugin-sdk/src/types.rs:882-918`), so a plugin cannot set
`Location`. A plugin page at `/user/subscriptions` that sent a member on to
`/user/{uid}/subscriptions` (the uid-less address a menu could link to) cannot be
written.

**Recommendation.** Add `ApiResponse::redirect(status, location)`.

### G-NO-ACCESSIBILITY-CHECK: **[Low, NEW]** the kernel ships no accessibility check to run against a theme

The CLI has four commands, `serve`, `plugin`, `config` and `user`
(`crates/kernel/src/main.rs:72-90`), and neither the repository nor its CI runs an
accessibility audit. The brief asks for "the kernel's accessibility check" on the
theme; there is none to run.

**Ritrovo's side.** The landmarks are the kernel's (`base.html`: skip link,
`header`, `main`, `footer`) and are kept; every `nav` Ritrovo adds is labelled;
active links carry `aria-current`; the badge colours are paired for 4.5:1 or better
in both schemes and the badge says its state in words. `scripts/verify-demo.sh`
asserts the landmarks and labels on the front page.

**Recommendation.** Ship an axe or pa11y run over the default theme in CI, usable
against a site's own.

### G-STAGE-SWITCH-IS-ADMIN-ONLY: **[Medium, NEW]** search answers from Live for everyone but an administrator, so an editor cannot search the stages they edit

Server search takes its stage set from the session: Live, or the session's active
stage plus Live (`crates/kernel/src/routes/search.rs:26-46`). Only
`/admin/stage/switch` sets the active stage, and it requires an administrator
(`crates/kernel/src/routes/admin.rs:44-56`, `require_admin_json`), whatever the
role may read. The browser's search is the Pagefind index, built from Live,
published items only (`crates/kernel/src/cron/pagefind.rs:135-147`), and has no
per-viewer view at all.

Observed, in the host test
`search_returns_live_to_everyone_and_another_stage_only_to_an_administrator_on_it`:
- an editor holding the demo's `editor` role, who may open Incoming and Curated
  conferences through `ritrovo_access`, gets the Live conference only;
- their stage switch is refused with 403, and not for its CSRF token;
- an administrator switched to Incoming gets Incoming and Live, and never Curated.

**What holds.** An anonymous visitor never finds an Incoming or Curated conference,
in either index; `verify-demo.sh` checks both, by title, on the demo.

**Ritrovo's side.** An editor finds Incoming and Curated conferences on
`/admin/content/editorial`, not through search.

**Blocks.** The editor half of D14's stage-aware search.

**Recommendation.** Let a permission rather than the administrator flag choose the
search stage, and give the Pagefind export a per-stage index, or document it as
the public index.

### G-CACHE-METRICS-NEVER-COUNT: **[FIXED at main 81290cc]** `/metrics` exports cache hit and miss counters that nothing increments

**Fixed at Trovato `main` 81290cc, verified 2026-11-23 (A11).** Kernel `ff0257a` (B76.1). `verify-demo.sh` asserts that `cache_hits_total{cache="item"}` on `/metrics` is above zero after it has read conference pages. What follows is the entry as written at 0.103.0.

`cache_hits_total` and `cache_misses_total` are registered
(`crates/kernel/src/metrics/mod.rs:127-135`) and incremented nowhere in
`crates/kernel/src`. The item cache logs nothing either. On the demo both read 0
after any amount of traffic.

**Recommendation.** Count in `ItemService::load` and `CacheLayer::get`, or drop the
two series so nobody reads a zero as a cold cache.

### G-BACKGROUND-ITEM-API-HAS-NO-ITEM-SERVICE: **[High, NEW]** `item_background = true` is accepted, and every item write from `tap_cron` or `tap_queue_worker` fails with `ERR_NO_SERVICES`

Found at Trovato `main` `81290cc`, still present at `64467b3`. Blocks P9 and keeps
the importer on raw SQL (`plugins/ritrovo_importer/src/lib.rs`, `insert_conference`).

The B68 design sends a plugin's item writes through `ItemService`, and a plugin
that writes in the background declares `item_background = true` to act with
kernel authority there (`crates/kernel/src/host/item.rs:24`, `:81`, `:107`). The
write itself needs the `ItemService` that `RequestServices` holds weakly
(`decide_with`, `host/item.rs:120`). `save-item` returns `ERR_NO_SERVICES` when it
is missing (`host/item.rs:285-287`), and so do the reads that decide access
(`:216-218`).

Nothing binds it in the background. `CronService::background_services`
(`crates/kernel/src/cron/mod.rs:1841-1856`) builds the services for `tap_cron`
(`:1108`), the queue drain (`:1460`) and `tap_install` (`:1832`) from
`RequestServices::for_background(...)` and `with_plugin_runtime`, the rate limiter
and email. It never calls `RequestServices::set_item_service`
(`crates/kernel/src/tap/request_state.rs:424`), although `CronService` holds an
`ItemService` (it uses it only for the Pagefind export). The kernel's own tests do
not catch this: `crates/kernel/tests/plugin_item_api_test.rs:104` and `:824` call
`set_item_service` on the services by hand before dispatching, so they never take
the path cron takes.

**Reproduction.** A plugin with `host_interfaces = [..., "item-api"]` and
`item_background = true` whose `tap_queue_worker` calls `host::save_item` with a
new conference. Enqueue one job and run `CronService::drain_plugin_queues`, or let
the demo's cron drain it. `save_item` returns -10 (`ERR_NO_SERVICES`) and nothing
is written. In Ritrovo it was `ritrovo_importer`'s batch worker, with its insert
moved to `save_item` and the manifest carrying both keys. Every import job failed this way in the
host test, and the importer went back to SQL.

**Recommendation.** In `background_services`, call
`services.set_item_service(&self.items)` (and the gather service's equivalent)
before returning. Add a test that dispatches `tap_queue_worker` through
`drain_plugin_queues` with nothing bound by hand.

### G-SQL-GUARD-REFUSES-INDEX-EXPRESSION-CONFLICT-TARGET: **[Medium, NEW]** the raw-SQL guard cannot parse `ON CONFLICT ((expression))`, which is valid PostgreSQL and the only way to upsert against an expression index

The raw-SQL guard (`crates/kernel/src/host/sql_guard.rs`, `parse_one` at `:77`,
called by `check_execute_raw` at `:114`) parses every plugin statement with
`sqlparser`'s `PostgreSqlDialect`. That parser does not accept an index
expression as a conflict target. The statement is refused as
`Unparseable("sql parser error: Expected: identifier, found: ( at Line 1, Column
388")` before it reaches Postgres, which would run it.

**Reproduction.** `host::execute_raw` with

```sql
INSERT INTO item (id, type, title, ...) VALUES (...)
ON CONFLICT ((fields->>'field_source_id')) WHERE type = 'conference'
DO UPDATE SET ...
```

against a table with a unique index on `((fields->>'field_source_id')) WHERE type =
'conference'`. Ritrovo's conference type has exactly that index. This was the
importer's upsert through 0.103.0.

**What Ritrovo did.** A plain `INSERT`. If it fails, the importer looks the
conference up by source id and updates it, which takes two statements where one
did the job. The fallback handles a conflict however it arises, another worker's
insert of the same conference included. The cost is a failed statement and two
more round trips for every conference a re-import already has, which is most of
them.

**Recommendation.** Either accept the statement when `sqlparser` fails and
Postgres's own parser (`EXPLAIN` without `ANALYZE`, or `pg_query`) accepts it, so
the guard refuses nothing Postgres would run, or document the gap in
`docs/plugin-capabilities.md` and give plugins an upsert on `item` through the
B80.4 write seam.

### G-SQL-GUARD-REFUSES-WITH-UPDATE: **[Low, NEW]** `WITH ... UPDATE` is refused by `execute-raw` as not a row change

`check_execute_raw` (`crates/kernel/src/host/sql_guard.rs:114-118`) accepts a
top-level `INSERT`, `UPDATE` or `DELETE`. `sqlparser` parses `WITH x AS (...)
UPDATE ...` as a `Query` whose body is the update, so it falls into the `_` arm and
is refused as `NotARowChange`. `query-raw` refuses it too, because it contains a
write. So a data-modifying statement with a CTE in front cannot run at all.

**Reproduction.** `host::execute_raw("WITH RECURSIVE t AS (SELECT 1) UPDATE item SET
changed = changed WHERE false", &[])` returns the guard's refusal.

**What Ritrovo did.** `search_text.rs` moved the recursive CTE into a subquery in
the `UPDATE`'s `FROM`, which the guard accepts and which Postgres runs with the same
plan. This entry is recorded because a guard that refuses a standard form costs
every plugin author the hour it took to find out why.

**Recommendation.** Treat a `Query` whose body is an `Insert`, `Update` or `Delete`
as that statement in `check_execute_raw`, checking the CTEs for writes as
`check_query_raw` already does.

### G-FRONT-PAGE-TILES-MATCH-THE-CONFIGURED-PATH: **[Low, NEW]** at `/`, menus see `/` and tiles see the configured front page's path, so a tile placed on `/` is not on the front page

Since `9b964d2` (B122) `/` dispatches the configured `site_front_page` path in
place. The page context keeps `requested_path` (`/`) beside `path` (the configured
path) (`crates/kernel/src/routes/helpers.rs:304-306`). Menus are filtered by
`requested_path` (`:332-338`). Tiles are filtered by `path`, which
`render_region(region, LIVE_STAGE_ID, path, ...)` receives (`:429`). Page template
suggestions are built from the rewritten path as well, so `page--front.html`
applies only to the default listing and not to a front page that is an item.

**Reproduction.** Set `site_front_page` to `/item/{uuid}` and give a tile the
visibility path `/`. The tile shows at `/item/{uuid}` and not at `/`. Before
B122, `/` redirected and the tile matched the address the browser had.

**What Ritrovo did.** The two front page tiles list both `/` and the item path
(`demo/config/tile.*0003.yml`, `*0006.yml`).
`docs/tutorial/templates/page--item--7c1d4c55-2d39-4b4a-9a3e-0b1f6a2c9e06.html`
extends `page--front.html`, so the front page keeps its template.
`verify-demo.sh` asserts the welcome and the tiles at `/` and at `/it/`.

**Recommendation.** Match tiles on `requested_path`, as menus do. Add `front` to
the page suggestions whenever `is_front_page_request()` is true.

### G-GATHER-ROWS-HAVE-NO-FILE-URLS: **[Low, NEW]** an item template gets `referenced_files`, and a gather row gets a file's id and no way to link it

`c578da4` (B72.1) gives the item route `referenced_files`, keyed by field, with a
`url` and `alt` for each file. Ritrovo's `item--conference.html` and
`item--speaker.html` now draw the logo, venue photos, schedule and headshot from
it. A gather row carries the field's stored value: a file id, or a `public://`
URI in older data. It carries no `referenced_files` and no URL. No route serves a
file by its id, so a listing template cannot draw a conference's logo.

**What Ritrovo did.** `gather/includes/conf-card.html` no longer draws a logo; its
comment names this entry. Before, it drew one from the `public://` URI, which only
worked for the URI form.

**Recommendation.** Resolve File fields to `{url, alt}` on gather rows as the item
route does, or serve `/file/{id}` with the file access check `183e16e` added.


### G-SEARCH-AI-CHECKS-NO-PERMISSION-OR-BUDGET: **[High, NEW]** the AI search endpoints spend the site's provider tokens for any visitor, unmetered and unlogged

Found at Trovato `main` `81290cc` (A9b). `POST /api/v1/search/expand`, `/summarize`
and `/followup` (`crates/kernel/src/routes/api_search.rs:36-53`, handlers at `:128`,
`:259`, `:289`) are not behind a plugin gate, check no permission, check no token
budget and write no usage row: `complete` (`:501`) throws the token count away
(`:535`, `let (content, _tokens)`). The only thing in their way is the search
settings' stage switches, and all three default to on (`crates/kernel/src/search/settings.rs:127-129`).
So the moment a site configures a default chat provider and sets its key, every
search any anonymous visitor makes calls the provider twice, with no per-person
limit beyond the IP rate limit and no record on the usage dashboard. The brief asks
for `use ai` for authenticated callers, a site switch for anonymous ones, and the
31.3 budgets (`docs/ritrovo/epic-02.md`, stories 30.4 and 30.5).

A second, smaller effect: an import of the `search` variable is read through a
five-minute cache that the import does not drop (`settings.rs:11-14`), so a
running server keeps the old stages for up to five minutes.

**What Ritrovo does.** `demo/config/variable.search.yml` switches all three stages
off; each answers 404 before any provider work, which `verify-demo.sh` and
`chat_and_search_ai_refuse_a_visitor_without_use_ai_and_ask_no_provider` assert.
Search AI stays off until the kernel meters it.

**Blocks.** 30.4, 30.5; the search half of 31.4.

**Recommendation.** Check `use ai` (and a site setting for anonymous use) in the
three handlers, run the caller's budget check before `complete`, and record usage
from the token count `complete` already parses.

### G-AI-RATE-LIMITS-ARE-NOT-PER-ROLE: **[Low, NEW]** an AI rate limit is one number for every member, or one number for the whole provider

Found at `81290cc` (A9b). The brief's 31.4 composes rate limits with permissions:
"a role can have `use ai chat` with 20 req/hr". The kernel has two limits and
neither knows a role: the chat route's `rate_limit_per_hour`, one value for every
signed-in member (`crates/kernel/src/services/ai_chat.rs:54`, checked at
`crates/kernel/src/routes/api_chat.rs:151`), and each provider's
`rate_limit_rpm`, shared by every caller of that provider on every server
(`crates/kernel/src/host/ai.rs:41`). Per-role control is the daily token budget
only. Related to the residual `G-API-RATE-LIMITS-FIXED`.

**What Ritrovo does.** Sets the provider limits (30 and 60 a minute) and leaves the
chat limit at the kernel's 20 an hour; the per-role difference is in the budgets.

**Blocks.** The rate-limit half of 31.4.

**Recommendation.** Key the chat limit, and the `ai_request` limit, by role as the
budgets are.

### G-AI-REQUESTS-SENT-WITHOUT-A-KEY: **[Medium, NEW]** with the key variable unset, the kernel still sends every AI request, content and all, without credentials

Found at `81290cc` (A9b). A provider whose `api_key_env` names a variable that is
not set resolves with `api_key: None` (`crates/kernel/src/services/ai_provider.rs:750-756`),
and every send path adds the credential header only `if let Some(key)`
(`crates/kernel/src/host/ai.rs:276`, `:342`, `:404`;
`crates/kernel/src/services/ai_chat.rs:557`, `:610`;
`crates/kernel/src/routes/api_search.rs:556`, `:571`;
`crates/kernel/src/routes/api_ai_assist.rs:276`, `:291`). The request goes to the
provider anyway, the provider answers 401, and the caller gets the kernel's
generic failure. Nothing is spent, but the request is sent whole: a member's
chat message, the conference text a translation draft carries, an editor's
field for AI Assist. A site that has not configured a provider, and so has
no account or terms with it, still sends that content to it. CI and every
keyless install make outbound calls that cannot succeed, and the failure reads
as a provider outage rather than as "not configured".

An empty value counts as a key. `resolve_api_key` is `std::env::var(..).ok()`
(`ai_provider.rs:754`), so a variable set to the empty string, which is what
Compose's `${VAR:-}` produces, resolves to `Some("")`: the provider screen says
"(set)" (`mask_key_ref`, `:765-773`) and every call goes out with an empty key.
Found when `verify-demo.sh` read "(set)" off a demo started with no key.

**What Ritrovo does.** `docker-compose.demo.yml` passes each key variable through
with no value, so it is absent from the container when the starting shell has
none, and says why. Translation drafts are off by default and the search
stages are off, so a keyless demo sends only what a signed-in member types into
chat. `docs/AI.md` says so, and says to set each provider's `enabled` to false on
an install that runs without keys. The visitor-facing answers are clean
(`with_no_key_every_ai_surface_answers_without_a_500_or_provider_text`). Pinned by
`with_no_key_the_kernel_still_sends_a_member_s_message_to_the_provider`, which
fails once the kernel stops sending.

**Blocks.** Nothing.

**Recommendation.** Treat a provider whose key variable is named and unset or empty
as not configured in `resolve_provider`, so every caller takes its "no provider" path.

### G-AI-CHAT-ERROR-LOGS-THE-PROVIDER-BODY-UNREDACTED: **[Low, NEW]** a failed chat stream logs the provider's error body without the key redaction the other paths apply

Found at `81290cc` (A9b), by reading. `execute_streaming` logs the provider's
failure through `log_provider_failure` (`crates/kernel/src/services/ai_chat.rs:481-486`),
which redacts the key (`crates/kernel/src/services/ai_provider.rs:1040`, `:1055`),
and then fails with the first 200 bytes of the same body in its error
(`ai_chat.rs:487-497`). The chat route wraps that error in
`AppError::service_unavailable` (`crates/kernel/src/routes/api_chat.rs:262`), whose `reason` is logged as is (`crates/kernel/src/error.rs:403-404`).
The visitor sees only "AI is temporarily unavailable". A provider that echoed the
credential in an error body would put it in the log through this one path. Neither
of the two providers Ritrovo uses echoes the key it was sent, which is why the
host test that searches every log line for a known key value passes; that test
covers what the kernel writes of its own accord, not this path. Pinned by
`a_provider_error_that_echoes_the_key_reaches_the_log_through_the_chat_path`: a
scripted provider echoes a fake key in its 401, the member sees only the
generic answer, and the key is in the log.

**Blocks.** Nothing.

**Recommendation.** Build the error from `provider_error_excerpt(&body, api_key)`.

### G-AI-ADMIN-HAS-NO-FEATURES-RULES-OR-RATE-LIMIT-SCREENS: **[Low, NEW]** three of the five AI administration tabs the brief describes do not exist

Found at `81290cc` (A9b). The kernel's AI screens are providers
(`/admin/system/ai-providers`), budgets and usage (`/admin/system/ai-budgets`),
chat (`/admin/system/ai-chat`) and the assistant (`/admin/system/ai-assistant`),
merged at `crates/kernel/src/routes/admin.rs:1073-1079`. The features screen was
deleted as never mounted and never read (`6e58a9b`, "admin_ai_features is
deleted"), and nothing replaced it; the search AI stages are on the search
settings screen instead. There is no field rules screen (rules are a raw JSON
`site_config` value, `plugins/trovato_ai/src/lib.rs:22-24`) and no AI rate-limit
screen. The usage screen shows tokens and requests per provider and per user for
the budget period (`templates/admin/ai-budgets.html`), with no cost column, no
per-role view, no time range and no request log, although every row of
`ai_usage_log` carries a cost estimate.

**Blocks.** Part of 31.9.

**Recommendation.** A cost column and a request log on the usage screen, from the
rows already written; a field rules list; or drop those tabs from the brief.

### G-CONFIG-IMPORT-LEAVES-SEARCH-SETTINGS-CACHED: **[Medium, NEW]** importing `variable.search.yml` does not drop the cached search settings, so the old ones stay in force for up to five minutes

Found at `81290cc` (A9b), when the Demo workflow saw the search AI stages answer
503 instead of 404. `SearchSettings::load` keeps the settings in Redis under
`search_settings` for 300 seconds (`crates/kernel/src/search/settings.rs:32-35`,
`:295-320`). `SearchSettings::save`, the admin form's path, drops that entry
(`:353`). `config import` writes the `search` variable with
`save_variable` (`crates/kernel/src/config_storage/direct.rs:524-541`), which bumps
the `SiteConfig` cache generation, and that generation clears only the
in-memory front page (`crates/kernel/src/state.rs:1407`), never the Redis entry.
Anything that read the settings before the import keeps the old ones, here the
defaults with every AI stage on, until the entry expires. With a key set, that
is up to five minutes in which any visitor's search spends the site's tokens
(`G-SEARCH-AI-CHECKS-NO-PERMISSION-OR-BUDGET`).

In the demo the reader was cron. The bootstrap started temporary servers on the
published port before the import, the cron poker reached them as soon as
`/health` answered, and a cron run read the settings (`crates/kernel/src/cron/mod.rs:1184`).

**What Ritrovo does.** `scripts/demo-bootstrap.sh` runs every server before the
last on a private port, so nothing outside the container reaches the kernel
until every import is done (`demo/checks/tests/bootstrap_port.rs`). An install
that imports `variable.search.yml` into a running site still waits out the
five minutes, or saves the search settings form once.

**Blocks.** Nothing.

**Recommendation.** Have `save_variable` drop `search_settings` when the key is
`search`, or have the `SiteConfig` generation clear the Redis entries derived
from `site_config` as well as the front page.

### G-AI-FIELD-RULES-NEVER-LOAD: **[High, NEW]** `trovato_ai` reads its field rules from a column `site_config` does not have, so no field rule ever runs

Found at `81290cc` (A9b). `load_field_rules` sends
`SELECT value FROM site_config WHERE name = $1` (`plugins/trovato_ai/src/lib.rs:121-124`);
`site_config`'s column is `key`. The query fails, the error path logs at
`debug` and returns no rules (`:126-133`), and `tap_item_presave` returns the
item unchanged (`:207-210`). No save anywhere runs a rule, whatever is configured.
Pinned by `trovato_ai_reads_its_field_rules_from_a_column_site_config_does_not_have`,
which reads the query out of the kernel source and runs it against the real
schema.

Read past the query, what the plugin would do once it loads a rule falls short of
story 31.5 as well:
- `trigger` is deserialized and never read (`:34-35`), so `on_change` and
  `always` are the same.
- `suggest` skips the rule (`:234-237`) and `validate` does not exist.
- There is no stage routing: the AI output is written into the item being
  saved, so a Live item takes it at once (design decision D8 is not built).
- A `Blocks` source reaches the prompt as its JSON text (`resolve_prompt`,
  `:163-178`), which is what a conference description is.

**What Ritrovo does.** `demo/config` adds `field_summary` to the conference type,
renders it on the conference page when present, and imports the rule
(`variable.trovato_ai.field_rules.yml`: description to summary,
`fill_if_empty`), so it runs as soon as the kernel reads it. Until then only an
editor fills the summary. Imported conferences would get none in any case: they
are written by raw SQL (`G-BACKGROUND-ITEM-API-HAS-NO-ITEM-SERVICE`).

**Blocks.** 31.5.

**Recommendation.** `WHERE key = $1`, and log a failed rule query at `warn`.
Then honour `trigger`, and route an AI write to a Live item onto an internal
stage as D8 says.

### G-SEARCH-AI-READS-ONLY-THE-FIRST-CONTENT-BLOCK: **[Medium, NEW]** the search AI stages read the first content block only, so a model that thinks first answers with nothing

Found at `81290cc` (A9b). The search AI's own response parser reads
`content[0].text` and nothing else (`crates/kernel/src/routes/api_search.rs:604-617`;
the OpenAI parser reads `choices[0]`, `:589-601`). Claude Haiku 5.5 thinks by
default, so its first block is a thinking block with no `text`, and the content
comes back `""`. `complete` treats that as success (`:534-539`): expansion answers
200 `{"terms":[]}`, and the summary and follow-up 200 with an empty `summary` or
`response`, after spending the tokens. The plugin `ai_request` path and chat
join every text block and are not affected. Pinned by
`the_search_ai_stages_answer_empty_when_the_model_thinks_first`, against the
kernel's scripted provider. Live with `claude-haiku-5-5` on the demo (2026-11-23,
the stages switched on locally for three calls): expansion returned four terms,
and both summary calls answered `{"summary":""}`.

**Blocks.** 30.4 and 30.5, with `G-SEARCH-AI-CHECKS-NO-PERMISSION-OR-BUDGET`.

**Recommendation.** Join the `text` blocks as `host/ai.rs` does, and treat an
empty answer as a failure.

### G-EMBEDDINGS-BYPASS-BUDGETS-AND-THE-USAGE-LOG: **[Medium, NEW]** every embedding call is unmetered, including one per visitor query on a similarity gather

Found at `81290cc` (A9b). `AiProviderService::embed` (`crates/kernel/src/services/ai_provider.rs:796-877`)
checks no permission and no budget and writes no `ai_usage_log` row. The only
writer of that log is the plugin `ai_request` path. Its callers:
- the embed queue, on purpose (`crates/kernel/src/cron/mod.rs:637-639`);
- the `semantic_similarity` gather operator, once per request, for any visitor,
  anonymous included, with no cache (`crates/kernel/src/gather/gather_service.rs:1740-1782`).

The token budgets therefore cannot cap embedding spend, and the usage dashboard
never shows it. What is embedded is not configurable either:
- every string field of every item type (`item_embedding_text`,
  `crates/kernel/src/content/item_service.rs:83-105`), editors' notes included;
- `embed_policy` can make a type synchronous but cannot leave one out
  (`crates/kernel/src/services/embed_index.rs:72-94`);
- the backfill queues every item in every stage (`find_backfill_gaps`, `:279-302`).

The gather takes its nearest 500 candidates across every embedded item before
the gather's own filters apply (`GATHER_SEMANTIC_SEARCH_MAX`,
`crates/kernel/src/gather/access.rs:62-104`). With the demo's 5,698 items embedded,
5,635 of them Incoming, "rust conferences in europe" found two Live conferences.

**What Ritrovo does.** `ritrovo.similar_conferences` is public, as a gather is.
An embedding of a short query costs about two millionths of a dollar, so the
exposure is a rate, not a sum. `docs/AI.md` says what is embedded and sent.

**Blocks.** The embedding half of 31.3's cap. Nothing else.

**Recommendation.** Meter `embed` through the budget service under the caller's
principal, with the queue's as a background principal, and log it. Cache query
embeddings. Let `embed_policy` exclude types and fields, and the backfill take a
stage and a type.

### G-EMBEDDINGS-HAVE-NO-TEST-PROVIDER: **[Low, NEW]** no test can reach an embedding provider, so similarity ranking can only be checked live

Found at `81290cc` (A9b). `embed` calls `validate_base_url`, which refuses
loopback, private and `localhost` hosts (`crates/kernel/src/services/ai_provider.rs:395-444`,
`:815-819`). The kernel's scripted provider answers `/chat/completions` and
`/messages` only (`crates/kernel/src/services/ai_tools.rs:831-834`). The chat path
skips the URL check, which is why chat tests can script a provider. The kernel's
own `gather_semantic_test.rs` says the same in its header and seeds vectors
directly instead.

**What Ritrovo does.** The host test checks the gather's definition, pgvector and
the no-embedding answer; the ranking is checked live (`docs/AI.md`).

**Blocks.** A host test for 31.11's ranking.

**Recommendation.** Serve `/embeddings` from the scripted provider, and give tests
a way to allow its loopback address.

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
